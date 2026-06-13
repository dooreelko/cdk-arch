//! Render ASCII
//!
//! Byte-for-byte port of the RENDERING stage of `layout.py` (lines 701-803).

use super::model::{Edge, Model, GUTTER_W, TITLE_H};
use std::fmt;
use std::path::Path;

/// A character grid. Stores `char`s (not bytes) so each multibyte glyph
/// (`│ ─ ┼ ► ◄ ▼ ▲`) occupies exactly one cell, matching Python's
/// per-code-point indexing.
pub struct Canvas {
    pub w: i64,
    pub h: i64,
    grid: Vec<Vec<char>>,
}

impl Canvas {
    pub fn new(w: i64, h: i64) -> Self {
        let grid = (0..h.max(0)).map(|_| vec![' '; w.max(0) as usize]).collect();
        Self { w, h, grid }
    }

    /// No-op if out of bounds (mirrors Python's bounds-checked paint).
    pub fn paint(&mut self, x: i64, y: i64, ch: char) {
        if 0 <= x && x < self.w && 0 <= y && y < self.h {
            self.grid[y as usize][x as usize] = ch;
        }
    }

    /// Panics on out-of-bounds access (Python raises IndexError).
    pub fn char_at(&self, x: i64, y: i64) -> char {
        if !(0 <= x && x < self.w && 0 <= y && y < self.h) {
            panic!(
                "char_at({}, {}) out of bounds for {}x{} canvas",
                x, y, self.w, self.h
            );
        }
        self.grid[y as usize][x as usize]
    }

    /// Write the Display string to `<path>.tmp`, then atomically rename it
    /// over `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let tmp = {
            let mut s = path.as_os_str().to_os_string();
            s.push(".tmp");
            std::path::PathBuf::from(s)
        };
        std::fs::write(&tmp, self.to_string())?;
        std::fs::rename(&tmp, path)
    }
}

impl fmt::Display for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // "\n".join("".join(row).rstrip() for row in grid) + "\n"
        // Each row -> string -> right-strip trailing spaces (the only filler).
        let rows: Vec<String> = self
            .grid
            .iter()
            .map(|row| {
                let s: String = row.iter().collect();
                s.trim_end_matches(char::is_whitespace).to_string()
            })
            .collect();
        write!(f, "{}\n", rows.join("\n"))
    }
}

// Arrowhead keyed by the TARGET port's side: an edge entering a left-side
// port moves rightward, so it ends in '►'; and so on for the other sides.
fn arrow(side: &str) -> char {
    match side {
        "left" => '►',
        "right" => '◄',
        "top" => '▼',
        "bottom" => '▲',
        _ => '?',
    }
}

fn paint_text(cv: &mut Canvas, x: i64, y: i64, s: &str) {
    for (i, ch) in s.chars().enumerate() {
        cv.paint(x + i as i64, y, ch);
    }
}

fn paint_scaffolding(m: &Model, cv: &mut Canvas) {
    debug_assert!(TITLE_H >= 5, "TITLE_H must fit headers + title block");

    // gutter spine
    for y in 0..cv.h {
        cv.paint(GUTTER_W, y, '│');
    }

    // column headers (rows 1-2): region index + kind, sorted by x value
    let mut cols: Vec<(i64, i64)> = m.col_x.iter().map(|(&k, &v)| (k, v)).collect();
    cols.sort_by_key(|&(_, v)| v);
    for (ridx, rx) in cols {
        let kind = if ridx % 2 == 0 { "nodes" } else { "edges" };
        paint_text(cv, rx + 1, 1, &format!("{:03}", ridx));
        paint_text(cv, rx + 1, 2, kind);
    }

    // row labels in the gutter + horizontal separators at region tops
    let mut rows: Vec<(i64, i64)> = m.row_y.iter().map(|(&k, &v)| (k, v)).collect();
    rows.sort_by_key(|&(_, v)| v);
    for (ridx, ry) in rows {
        if ry > 0 {
            for x in 1..cv.w {
                cv.paint(x, ry, '─');
            }
            cv.paint(GUTTER_W, ry, '┼');
        }
        let kind = if ridx == 0 {
            "title"
        } else if ridx % 2 == 1 {
            "nodes"
        } else {
            "edges"
        };
        paint_text(cv, 2, ry + 1, &format!("{:03}", ridx));
        paint_text(cv, 2, ry + 2, kind);
    }

    // title block at rows 3-4 of the title region, clear of headers
    let title_y = m.row_y[&0];
    paint_text(cv, GUTTER_W + 2, title_y + 3, &m.title);
    paint_text(cv, GUTTER_W + 2, title_y + 4, &m.description);
}

fn draw_box(cv: &mut Canvas, n: &super::model::Node) {
    let (x0, y0) = (n.x, n.y);
    let (x1, y1) = (n.x + n.w - 1, n.y + n.h - 1);
    for x in x0..=x1 {
        cv.paint(x, y0, '-');
        cv.paint(x, y1, '-');
    }
    for y in y0..=y1 {
        cv.paint(x0, y, '|');
        cv.paint(x1, y, '|');
    }
    for (cx, cy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        cv.paint(cx, cy, '+');
    }
    let label_len = n.label.chars().count() as i64;
    let lx = x0 + (n.w - label_len) / 2;
    paint_text(cv, lx, y0 + n.h / 2, &n.label);
}

fn draw_group(cv: &mut Canvas, g: &super::model::Group) {
    use super::model::{GROUP_BL, GROUP_BR, GROUP_H, GROUP_TL, GROUP_TR, GROUP_V};
    let (x0, y0) = (g.x, g.y);
    let (x1, y1) = (g.x + g.w - 1, g.y + g.h - 1);
    for x in x0..=x1 {
        cv.paint(x, y0, GROUP_H);
        cv.paint(x, y1, GROUP_H);
    }
    for y in y0..=y1 {
        cv.paint(x0, y, GROUP_V);
        cv.paint(x1, y, GROUP_V);
    }
    cv.paint(x0, y0, GROUP_TL);
    cv.paint(x1, y0, GROUP_TR);
    cv.paint(x0, y1, GROUP_BL);
    cv.paint(x1, y1, GROUP_BR);
    // title: inside, one space from left border, one row above bottom border
    paint_text(cv, x0 + 1, y1 - 1, &g.title);
}

fn paint_edge(cv: &mut Canvas, e: &Edge) {
    let route = e.route.as_ref().expect("paint_edge requires a route");
    for w in route.windows(2) {
        let (x0, y0) = (w[0][0], w[0][1]);
        let (x1, y1) = (w[1][0], w[1][1]);
        if y0 == y1 {
            for x in x0.min(x1)..=x0.max(x1) {
                cv.paint(x, y0, e.char);
            }
        } else {
            for y in y0.min(y1)..=y0.max(y1) {
                cv.paint(x0, y, e.char);
            }
        }
    }
    let fp = e.from_port.as_ref().expect("routed edge has from_port");
    let tp = e.to_port.as_ref().expect("routed edge has to_port");
    cv.paint(fp.x, fp.y, '*');
    cv.paint(tp.x, tp.y, arrow(&tp.side));
}

/// Paint scaffolding, boxes, then edges -- saving after every mutation
/// so a crash mid-run leaves the last good state on disk.
pub fn render(m: &Model, cv: &mut Canvas, path: &Path) -> std::io::Result<()> {
    render_with_observer(m, cv, path, &mut |_| {})
}

/// Like [`render`], but invokes `on_save(&Canvas)` after each save (for tests).
pub fn render_with_observer(
    m: &Model,
    cv: &mut Canvas,
    path: &Path,
    on_save: &mut dyn FnMut(&Canvas),
) -> std::io::Result<()> {
    paint_scaffolding(m, cv);
    cv.save(path)?;
    on_save(cv);
    let mut groups: Vec<&super::model::Group> = m.groups.iter().collect();
    groups.sort_by_key(|g| g.depth); // outer (depth 0) first
    for g in groups {
        draw_group(cv, g);
        cv.save(path)?;
        on_save(cv);
    }
    for n in &m.nodes {
        draw_box(cv, n);
        cv.save(path)?;
        on_save(cv);
    }
    for e in &m.edges {
        if e.route.is_none() {
            continue;
        }
        paint_edge(cv, e);
        cv.save(path)?;
        on_save(cv);
    }
    Ok(())
}
