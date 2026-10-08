//! placement hints: how skeleton, place and repair take them, and how a finished layout is checked against them
use crate::model::{Box as GBox, Cell, Graph, PlacementHint, Placement, Rel};
use indexmap::IndexMap;

/// hints with the priority one last, so that it overwrites whatever the others arranged
pub fn priority_last(g: &Graph) -> Vec<&PlacementHint> {
    let mut v: Vec<&PlacementHint> = g.placement.iter().collect();
    v.sort_by_key(|h| h.priority);
    v
}

/// hints with the priority one first: it gets its way before the others can occupy the ground
pub fn priority_first(g: &Graph) -> Vec<&PlacementHint> {
    let mut v: Vec<&PlacementHint> = g.placement.iter().collect();
    v.sort_by_key(|h| !h.priority);
    v
}

/// cell span of a node or a group
fn span(p: &Placement, id: &str) -> Option<GBox> {
    p.cells.get(id).map(|c| GBox { col0: c.col, row0: c.row, col1: c.col, row1: c.row }).or_else(|| p.groups.as_ref().and_then(|g| g.get(id)).copied())
}

fn overlap(a: (i32, i32), b: (i32, i32)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}

/// None = an end is not in this placement
pub fn holds(h: &PlacementHint, p: &Placement) -> Option<bool> {
    let a = span(p, &h.a)?;
    let Some(b) = h.b.as_ref() else {
        return Some(a.col0 == p.cells.values().map(|c| c.col).chain(p.groups.iter().flat_map(|g| g.values().map(|b| b.col0))).min()?);
    };
    let b = span(p, b)?;
    Some(match h.rel {
        Rel::LeftOf => a.col1 < b.col0,
        Rel::RightOf => a.col0 > b.col1,
        Rel::Above => a.row1 < b.row0,
        Rel::Below => a.row0 > b.row1,
        Rel::SameRow => overlap((a.row0, a.row1), (b.row0, b.row1)),
        Rel::SameCol => overlap((a.col0, a.col1), (b.col0, b.col1)),
        Rel::Leftmost => unreachable!(),
    })
}

pub fn violated(hints: &[PlacementHint], p: &Placement) -> usize {
    hints.iter().filter(|h| holds(h, p) == Some(false)).count()
}

/// one line per hint the layout does not satisfy
pub fn ignored(hints: &[PlacementHint], p: &Placement) -> Vec<String> {
    hints
        .iter()
        .filter(|h| holds(h, p) != Some(true))
        .map(|h| match &h.b {
            Some(b) => format!("{} {:?} {}", h.a, h.rel, b),
            None => format!("{} {:?}", h.a, h.rel),
        })
        .collect()
}

/// the node and everything already lined up with it by applied same-row hints
fn row_group(linked: &[(String, String)], id: &str) -> Vec<String> {
    let mut group = vec![id.to_string()];
    while let Some(more) = linked.iter().find_map(|(x, y)| match (group.contains(x), group.contains(y)) {
        (true, false) => Some(y.clone()),
        (false, true) => Some(x.clone()),
        _ => None,
    }) {
        group.push(more);
    }
    group
}

/// move the group `delta` rows; whoever else sits on a moved node's new cell goes to the nearest free odd row of its column
fn shift_group(cells: &mut IndexMap<String, Cell>, group: &[String], delta: i32) {
    group.iter().for_each(|id| cells[id].row += delta);
    for id in group {
        let c = cells[id];
        let at = |cells: &IndexMap<String, Cell>, row: i32, not: &str| cells.iter().find(|(o, v)| v.col == c.col && v.row == row && *o != not).map(|(o, _)| o.clone());
        if let Some(o) = at(cells, c.row, id).filter(|o| !group.contains(o)) {
            let row = (1..).flat_map(|k| [c.row - 2 * k + 1, c.row + 2 * k - 1]).find(|r| at(cells, *r, &o).is_none()).unwrap();
            cells[&o].row = row;
        }
    }
}

/// rows doubled so that a node can be slotted between two others, see `apply_rows`
pub fn apply_rows(g: &Graph, cells: &mut IndexMap<String, Cell>) {
    cells.values_mut().for_each(|c| c.row *= 2);
    let mut linked: Vec<(String, String)> = Vec::new();
    // same-row hints define the rows, so they go first; moving a node onto a row later would undo above/below
    let mut ordered = priority_last(g);
    ordered.sort_by_key(|h| (h.priority, h.rel != Rel::SameRow));
    for h in ordered {
        let (Some(b_id), true) = (&h.b, cells.contains_key(&h.a)) else { continue };
        let (Some(a), Some(b)) = (cells.get(&h.a).copied(), cells.get(b_id).copied()) else { continue };
        let occupant = |cells: &IndexMap<String, Cell>, col: i32, row: i32, not: &str| cells.iter().find(|(id, c)| c.col == col && c.row == row && *id != not).map(|(id, _)| id.clone());
        match h.rel {
            Rel::SameRow => {
                // the keyed node `a` moves onto b's row together with everything already lined up with it
                let group = row_group(&linked, &h.a);
                linked.push((h.a.clone(), b_id.clone()));
                if a.row == b.row || group.contains(b_id) {
                    continue;
                }
                if group.len() == 1 {
                    if let Some(o) = occupant(cells, a.col, b.row, &h.a) {
                        cells[&o].row = a.row;
                    }
                    cells[&h.a].row = b.row;
                } else {
                    shift_group(cells, &group, b.row - a.row);
                }
            }
            Rel::Above | Rel::Below => {
                // the subject `a` moves, never the node it is placed against
                let above = h.rel == Rel::Above;
                let (u, d) = if above { (a, b) } else { (b, a) };
                if u.row < d.row {
                    continue;
                }
                // nearest free odd row beyond `b`, in `a`'s column
                let row = (1..).map(|k| if above { b.row - 2 * k + 1 } else { b.row + 2 * k - 1 }).find(|r| occupant(cells, a.col, *r, &h.a).is_none()).unwrap();
                shift_group(cells, &row_group(&linked, &h.a), row - a.row);
            }
            _ => {}
        }
    }
}
