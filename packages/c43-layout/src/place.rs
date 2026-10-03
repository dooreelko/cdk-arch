use crate::geom::cell_key;
use crate::js;
use crate::model::{Cell, Graph, Kind, Placement};
use crate::skeleton::Skeleton;
use indexmap::{IndexMap, IndexSet};

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub fn place(g: &Graph, sk: &Skeleton) -> Placement {
    let mut cells: IndexMap<String, Cell> = IndexMap::new();
    let mut preds: IndexMap<String, Vec<String>> = IndexMap::new();
    for e in g.edges.iter().filter(|e| e.kind == Kind::Data) {
        preds.entry(e.to.clone()).or_default().push(e.from.clone());
    }

    // rows: each node wants the mean row of its placed data predecessors;
    // a column keeps its order, packs rows, then shifts so the spread is centred on the wishes
    for (c, col) in sk.columns.iter().enumerate() {
        let mut order: Vec<(String, usize, f64)> = col
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let rs: Vec<f64> = preds.get(id).map_or(vec![], |ps| ps.iter().filter_map(|q| cells.get(q)).map(|c| c.row as f64).collect());
                (id.clone(), i, if rs.is_empty() { 0.0 } else { mean(&rs) })
            })
            .collect();
        order.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap().then(a.1.cmp(&b.1)));
        let mut rows: Vec<f64> = Vec::new();
        for (i, o) in order.iter().enumerate() {
            rows.push(if i == 0 { js::round(o.2) } else { js::round(o.2).max(rows[i - 1] + 1.0) });
        }
        let shift = js::round(mean(&order.iter().enumerate().map(|(i, o)| o.2 - rows[i]).collect::<Vec<_>>()));
        for (i, o) in order.iter().enumerate() {
            cells.insert(o.0.clone(), Cell { col: c as i32, row: (rows[i] + shift) as i32 });
        }
    }

    place_secondary(g, &mut cells);
    compact(&cells)
}

/// placed neighbour to sit near; `left_of` when the node points at it, otherwise it sits at or right of it (no leftward edges)
fn anchor_of(g: &Graph, cells: &IndexMap<String, Cell>, id: &str) -> Option<(Cell, bool)> {
    if let Some(into) = g.edges.iter().find(|e| e.to == id && cells.contains_key(&e.from)) {
        return Some((cells[&into.from], false));
    }
    g.edges.iter().find(|e| e.from == id && cells.contains_key(&e.to)).map(|out| (cells[&out.to], true))
}

/// nodes without data edges go to the bottom band, as close to their anchor column as possible
fn place_secondary(g: &Graph, cells: &mut IndexMap<String, Cell>) {
    let mut pending: Vec<String> = g.nodes.iter().map(|n| n.id.clone()).filter(|id| !cells.contains_key(id)).collect();
    let mut taken: IndexSet<String> = cells.values().map(cell_key).collect();
    let bottom_row = cells.values().map(|c| c.row).max().unwrap_or(0);
    while !pending.is_empty() {
        let idx = pending.iter().position(|id| anchor_of(g, cells, id).is_some()).unwrap_or(0);
        let id = pending.remove(idx);
        let (anchor, left_of) = anchor_of(g, cells, &id).unwrap_or((Cell { col: 0, row: bottom_row }, false));
        let (min_col, max_col) = if left_of { (0, anchor.col) } else { (anchor.col, i32::MAX) };
        let cell = nearest_free(&taken, anchor.col, bottom_row.max(anchor.row + 1), min_col, max_col);
        cells.insert(id, cell);
        taken.insert(cell_key(&cell));
    }
}

/// cost = 2·(rows below min_row) + |column offset|; ties: smaller offset, then right, then higher; stays within [min_col, max_col]
fn nearest_free(taken: &IndexSet<String>, col: i32, min_row: i32, min_col: i32, max_col: i32) -> Cell {
    for cost in 0.. {
        for dr in (0..=cost / 2).rev() {
            let off = cost - 2 * dr;
            let dcs: &[i32] = if off == 0 { &[0] } else { &[off, -off] };
            for dc in dcs {
                let c = Cell { col: col + dc, row: min_row + dr };
                if c.col >= min_col.max(0) && c.col <= max_col && !taken.contains(&cell_key(&c)) {
                    return c;
                }
            }
        }
    }
    unreachable!()
}

/// renumber to dense 0-based rows/cols (title/meta live in the header band, so no cell is reserved)
pub fn compact(cells: &IndexMap<String, Cell>) -> Placement {
    let used = |f: fn(&Cell) -> i32| {
        let mut v: Vec<i32> = cells.values().map(f).collect::<IndexSet<_>>().into_iter().collect();
        v.sort();
        v
    };
    let cols_used = used(|c| c.col);
    let rows_used = used(|c| c.row);
    let index = |v: &[i32], x: i32| v.iter().position(|y| *y == x).unwrap() as i32;
    let out: IndexMap<String, Cell> = cells
        .iter()
        .map(|(id, c)| (id.clone(), Cell { col: index(&cols_used, c.col), row: index(&rows_used, c.row) }))
        .collect();
    Placement { cols: cols_used.len() as i32, rows: rows_used.len() as i32, cells: out, groups: None }
}
