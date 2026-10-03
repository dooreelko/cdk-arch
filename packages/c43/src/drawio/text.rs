//! Text size estimates for the drawio rendering (no font metrics dependency).

/// node label font size in px; title and description use twice this
pub const LABEL_PX: f64 = 12.0;
pub const TITLE_PX: f64 = 2.0 * LABEL_PX;
/// line height as a multiple of font size
pub const LINE_H: f64 = 1.25;
/// clear space between text and the box around it, each side, in px
pub const PAD: f64 = LABEL_PX;
/// px per layout grid unit
pub const UNIT_PX: f64 = 20.0;

const NARROW: &str = "iljtfr.,:;'\"|!()[]/ ";
const WIDE: &str = "mwMW@%&";

/// estimated width in px: char classes × font size, a little conservative for drawio's Helvetica
pub fn text_width(s: &str, font_px: f64, bold: bool) -> f64 {
    let em = s.chars().fold(0.0, |em, ch| {
        em + if NARROW.contains(ch) {
            0.34
        } else if WIDE.contains(ch) {
            0.9
        } else if ch.is_ascii_uppercase() {
            0.7
        } else {
            0.58
        }
    });
    em * font_px * if bold { 1.1 } else { 1.0 }
}

fn is_sep(c: char) -> bool {
    matches!(c, '-' | '/' | '_' | '.')
}

/// pieces a label may break between: at spaces (dropped) and after - / _ . (kept); `space`: a space precedes it
fn pieces(s: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for word in s.split_whitespace() {
        let mut parts: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_seps = false;
        for c in word.chars() {
            if in_seps && !is_sep(c) {
                parts.push(std::mem::take(&mut cur));
                in_seps = false;
            }
            in_seps |= is_sep(c);
            cur.push(c);
        }
        parts.push(cur);
        let first = out.is_empty();
        out.extend(parts.into_iter().enumerate().map(|(i, text)| (text, i == 0 && !first)));
    }
    out
}

/// greedy line breaking to `max_px`; None when a single piece is wider than that
pub fn wrap(s: &str, max_px: f64, font_px: f64, bold: bool) -> Option<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for (text, space) in pieces(s) {
        if text_width(&text, font_px, bold) > max_px {
            return None;
        }
        let joined = format!("{cur}{}{text}", if !cur.is_empty() && space { " " } else { "" });
        if cur.is_empty() || text_width(&joined, font_px, bold) <= max_px {
            cur = joined;
        } else {
            lines.push(std::mem::replace(&mut cur, text));
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    Some(lines)
}

/// label lines inside a square of `side_px`, or None if the label does not fit
pub fn label_lines(label: &str, side_px: f64) -> Option<Vec<String>> {
    let inner = side_px - 2.0 * PAD;
    wrap(label, inner, LABEL_PX, false).filter(|lines| lines.len() as f64 * LABEL_PX * LINE_H <= inner)
}

/// smallest node side in grid units that holds every label
pub fn label_side(labels: &[&str], unit_px: f64) -> f64 {
    labels.iter().fold(1.0, |s, l| {
        let mut s = s;
        while label_lines(l, s * unit_px).is_none() {
            s += 1.0;
        }
        s
    })
}
