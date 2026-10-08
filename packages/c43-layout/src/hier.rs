use crate::groups::{child_of, contains, owner, parent_of, ROOT};
use crate::model::{Box as GBox, Cell, Graph, Group, Kind, Placement, PlacementHint, Rel, RawEdge, RawGraph, RawNode, SizeHints};
use crate::normalize::normalize_internal;
use indexmap::{IndexMap, IndexSet};

/// virtual ends standing for "outside this group": inputs come from the left, outputs go to the right
const IN: &str = "\u{0}in";
const OUT: &str = "\u{0}out";

/// a level laid out in its own cell coordinates
struct Block {
    cols: i32,
    rows: i32,
    cells: IndexMap<String, Cell>,
    boxes: IndexMap<String, GBox>,
}

pub type Solve<'a> = dyn Fn(&Graph) -> Placement + 'a;
pub type TitleCols<'a> = dyn Fn(&Group) -> i32 + 'a;

/**
 * smallest column count whose group rectangle surely holds the bold title: c nodes of at least min_s,
 * c − 1 lanes of at least 3 between them, and a border at least 1.5 units out on each side,
 * minus the unit the title box keeps clear (title boxes round up to whole units, hence + 1)
 */
pub fn title_cols_for(min_s: f64, sizes: &SizeHints) -> impl Fn(&Group) -> i32 + '_ {
    move |gr| {
        let need = sizes.group_title.get(&gr.id).copied().unwrap_or(0.0) + 1.0;
        let mut c = 1;
        while c as f64 * min_s + 3.0 * (c - 1) as f64 + 2.0 < need {
            c += 1;
        }
        c
    }
}

fn inside(g: &Graph, level: &str, id: &str) -> bool {
    level == ROOT || contains(g, level, id)
}

/// hints seen from `level`: each end becomes the child holding it; hints inside one child are that child's business
fn lifted_hints(g: &Graph, level: &str, kids: &[String]) -> Vec<PlacementHint> {
    let lift = |id: &String| child_of(g, level, id).filter(|c| kids.contains(c));
    g.placement
        .iter()
        .filter_map(|h| {
            let a = lift(&h.a)?;
            let b = match &h.b {
                Some(b) => Some(lift(b)?),
                None => None,
            };
            (b.as_ref() != Some(&a)).then(|| PlacementHint { a, b, ..h.clone() })
        })
        .collect()
}

/// the level's children as a small graph: child groups are unit super-nodes, edges lifted, outside = IN/OUT
fn macro_graph(g: &Graph, level: &str, kids: &[String]) -> RawGraph {
    let mut kinds: IndexMap<(String, String), Kind> = IndexMap::new();
    for e in &g.edges {
        let (a, b) = if owner(g, &e.from, &e.to) == level {
            (Some(child_of(g, level, &e.from).unwrap_or_else(|| IN.into())), child_of(g, level, &e.to))
        } else if inside(g, level, &e.from) && !inside(g, level, &e.to) {
            (child_of(g, level, &e.from), Some(OUT.into()))
        } else if inside(g, level, &e.to) && !inside(g, level, &e.from) {
            (Some(IN.into()), child_of(g, level, &e.to))
        } else {
            (None, None)
        };
        let (Some(a), Some(b)) = (a, b) else { continue };
        if a == b {
            continue;
        }
        let k = (a, b);
        let kind = if kinds.get(&k) == Some(&Kind::Data) || e.kind == Kind::Data { Kind::Data } else { Kind::Nf };
        kinds.insert(k, kind);
    }
    let edges: Vec<RawEdge> = kinds.into_iter().map(|((from, to), kind)| RawEdge { from, to, kind: Some(kind), label: None }).collect();
    let used: IndexSet<&String> = edges.iter().flat_map(|e| [&e.from, &e.to]).collect();
    let kind_of: IndexMap<&String, Kind> = g.nodes.iter().map(|n| (&n.id, n.kind)).collect();
    let ids: Vec<String> = used
        .contains(&IN.to_string())
        .then(|| IN.to_string())
        .into_iter()
        .chain(kids.iter().cloned())
        .chain(used.contains(&OUT.to_string()).then(|| OUT.to_string()))
        .collect();
    let nodes = ids
        .into_iter()
        .map(|id| {
            let kind = kind_of.get(&id).copied().unwrap_or(Kind::Data);
            RawNode { id, label: None, kind: Some(kind), group: None }
        })
        .collect();
    RawGraph { title: None, description: None, groups: vec![], nodes, edges, placement: lifted_hints(g, level, kids), gravity: g.gravity }
}

/// lay out the level's children by the diagram rules, then expand macro cells into cell spans
fn expand(g: &Graph, level: &str, kids: &[String], solve: &Solve, title_cols: &TitleCols) -> Block {
    let sub: IndexMap<&String, Block> = kids.iter().filter(|id| g.group_ids.contains(*id)).map(|id| (id, block_of(g, id, solve, title_cols))).collect();
    let mg = normalize_internal(&macro_graph(g, level, kids)).expect("macro graph of a valid graph is valid");
    let mac = solve(&mg);
    let cell_of = |id: &str| mac.cells[id];
    let occupied: IndexMap<(i32, i32), &String> = kids.iter().map(|id| ((cell_of(id).col, cell_of(id).row), id)).collect();
    let at = |c: i32, r: i32| occupied.get(&(c, r)).copied();
    let base = |id: &str| sub.get(&id.to_string()).map_or((1, 1), |b| (b.cols, b.rows));
    let sorted = |f: fn(&Cell) -> i32| {
        let mut v: Vec<i32> = kids.iter().map(|id| f(&cell_of(id))).collect::<IndexSet<_>>().into_iter().collect();
        v.sort();
        v
    };
    let cols = sorted(|c| c.col);
    let rows = sorted(|c| c.row);
    let col_w: IndexMap<i32, i32> =
        cols.iter().map(|c| (*c, kids.iter().filter(|id| cell_of(id).col == *c).map(|id| base(id).0).max().unwrap())).collect();
    let row_h: IndexMap<i32, i32> =
        rows.iter().map(|r| (*r, kids.iter().filter(|id| cell_of(id).row == *r).map(|id| base(id).1).max().unwrap())).collect();
    // neighbour groups share the touching side: a stack takes the column width, a row the row height
    let is_g = |id: Option<&String>| id.is_some_and(|id| g.group_ids.contains(id));
    let size = |id: &String| {
        let (Cell { col, row }, b) = (cell_of(id), base(id));
        if !is_g(Some(id)) {
            return b;
        }
        let stack = is_g(at(col, row - 1)) || is_g(at(col, row + 1));
        let line = is_g(at(col - 1, row)) || is_g(at(col + 1, row));
        (if stack { col_w[&col] } else { b.0 }, if line { row_h[&row] } else { b.1 })
    };
    let mut x0: IndexMap<i32, i32> = IndexMap::new();
    let mut y0: IndexMap<i32, i32> = IndexMap::new();
    cols.iter().fold(0, |x, c| {
        x0.insert(*c, x);
        x + col_w[c]
    });
    rows.iter().fold(0, |y, r| {
        y0.insert(*r, y);
        y + row_h[r]
    });
    let mut block = Block { cols: col_w.values().sum(), rows: row_h.values().sum(), cells: IndexMap::new(), boxes: IndexMap::new() };
    for id in kids {
        let Cell { col, row } = cell_of(id);
        let sz = size(id);
        let c0 = x0[&col] + (col_w[&col] - sz.0) / 2;
        let r0 = y0[&row] + (row_h[&row] - sz.1) / 2;
        let Some(s) = sub.get(id) else {
            block.cells.insert(id.clone(), Cell { col: c0, row: r0 });
            continue;
        };
        block.boxes.insert(id.clone(), GBox { col0: c0, row0: r0, col1: c0 + sz.0 - 1, row1: r0 + sz.1 - 1 });
        // the interior is centred in a stretched block
        let (dc, dr) = (c0 + (sz.0 - s.cols) / 2, r0 + (sz.1 - s.rows) / 2);
        for (n, c) in &s.cells {
            block.cells.insert(n.clone(), Cell { col: c.col + dc, row: c.row + dr });
        }
        for (h, b) in &s.boxes {
            block.boxes.insert(h.clone(), GBox { col0: b.col0 + dc, row0: b.row0 + dr, col1: b.col1 + dc, row1: b.row1 + dr });
        }
    }
    // a plain node with a same-row hint to a member of a sibling block lines up with that member, within its row span;
    // plain nodes lined up with it follow
    let span = |id: &String| {
        let r = cell_of(id).row;
        (y0[&r], y0[&r] + row_h[&r] - 1)
    };
    let peers = |id: &String| -> Vec<String> {
        g.placement
            .iter()
            .filter(|h| h.rel == Rel::SameRow)
            .filter_map(|h| match &h.b {
                Some(b) if h.a == *id => Some(b.clone()),
                Some(b) if b == id => Some(h.a.clone()),
                _ => None,
            })
            .collect()
    };
    for id in kids.iter().filter(|id| !sub.contains_key(*id)) {
        let anchor = peers(id).into_iter().find(|p| sub.values().any(|s| s.cells.contains_key(p)));
        let Some(row) = anchor.and_then(|p| block.cells.get(&p)).map(|c| c.row) else { continue };
        let mut work = vec![(id.clone(), row)];
        let mut done: IndexSet<String> = IndexSet::new();
        while let Some((n, row)) = work.pop() {
            if !done.insert(n.clone()) {
                continue;
            }
            let (lo, hi) = span(&n);
            block.cells.get_mut(&n).unwrap().row = row.clamp(lo, hi);
            let row = block.cells[&n].row;
            work.extend(peers(&n).into_iter().filter(|p| kids.contains(p) && !sub.contains_key(p)).map(|p| (p, row)));
        }
    }
    block
}

fn block_of(g: &Graph, level: &str, solve: &Solve, title_cols: &TitleCols) -> Block {
    let kids: Vec<String> =
        g.nodes.iter().map(|n| &n.id).chain(g.groups.iter().map(|x| &x.id)).filter(|id| parent_of(g, id) == level).cloned().collect();
    // an empty group is one cell
    let block = if kids.is_empty() {
        Block { cols: 1, rows: 1, cells: IndexMap::new(), boxes: IndexMap::new() }
    } else {
        expand(g, level, &kids, solve, title_cols)
    };
    if level == ROOT {
        return block;
    }
    // a title wider than the contents widens the block (as a long label grows a node)
    let need = title_cols(g.groups.iter().find(|x| x.id == level).unwrap());
    if block.cols >= need {
        return block;
    }
    let d = (need - block.cols) / 2;
    Block {
        cols: need,
        rows: block.rows,
        cells: block.cells.into_iter().map(|(n, c)| (n, Cell { col: c.col + d, row: c.row })).collect(),
        boxes: block.boxes.into_iter().map(|(h, b)| (h, GBox { col0: b.col0 + d, col1: b.col1 + d, ..b })).collect(),
    }
}

/// bottom-up: every group's children laid out by the diagram rules, then expanded into one global grid
pub fn hier_place(g: &Graph, solve: &Solve, title_cols: &TitleCols) -> Placement {
    let mut b = block_of(g, ROOT, solve, title_cols);
    if g.gravity {
        gravitate(g, &mut b);
    }
    Placement { cols: b.cols, rows: b.rows, cells: b.cells, groups: Some(b.boxes) }
}

/// gravity: a top-level node with only nf edges moves to the nearest free cell outside every group frame,
/// at or right of and at or below the member of a group that points at it (nf runs downward); hinted nodes stay as hinted
fn gravitate(g: &Graph, b: &mut Block) {
    let nodes: Vec<String> = g
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .filter(|id| !g.data_nodes.contains(id) && parent_of(g, id) == ROOT && !g.placement.iter().any(|h| h.a == *id))
        .collect();
    for id in nodes {
        let Some(from) = g.edges.iter().find(|e| e.to == id && g.parent.contains_key(&e.from) && b.cells.contains_key(&e.from)).map(|e| b.cells[&e.from]) else { continue };
        let dist = |c: &Cell| (c.col - from.col) + (c.row - from.row);
        let free = |c: &Cell| {
            !b.cells.values().any(|o| o == c) && !b.boxes.values().any(|x| x.col0 <= c.col && c.col <= x.col1 && x.row0 <= c.row && c.row <= x.row1)
        };
        let best = (from.col..b.cols).flat_map(|col| (from.row..b.rows).map(move |row| Cell { col, row })).filter(|c| free(c)).min_by_key(|c| (dist(c), c.row));
        let here = b.cells[&id];
        if let Some(c) = best.filter(|c| dist(c) < (here.col - from.col).abs() + (here.row - from.row).abs()) {
            b.cells.insert(id, c);
        }
    }
    drop_empty_lines(b);
}

/// renumber so that no row or column is empty (a row or column is used by a node or a group frame)
fn drop_empty_lines(b: &mut Block) {
    let used = |nodes: Vec<i32>, frames: Vec<(i32, i32)>| -> Vec<i32> {
        let mut v: Vec<i32> = nodes.into_iter().chain(frames.into_iter().flat_map(|(lo, hi)| lo..=hi)).collect::<IndexSet<_>>().into_iter().collect();
        v.sort();
        v
    };
    let cols = used(b.cells.values().map(|c| c.col).collect(), b.boxes.values().map(|x| (x.col0, x.col1)).collect());
    let rows = used(b.cells.values().map(|c| c.row).collect(), b.boxes.values().map(|x| (x.row0, x.row1)).collect());
    let at = |v: &[i32], x: i32| v.iter().position(|y| *y == x).unwrap() as i32;
    b.cells.values_mut().for_each(|c| *c = Cell { col: at(&cols, c.col), row: at(&rows, c.row) });
    b.boxes.values_mut().for_each(|x| *x = GBox { col0: at(&cols, x.col0), col1: at(&cols, x.col1), row0: at(&rows, x.row0), row1: at(&rows, x.row1) });
    b.cols = cols.len() as i32;
    b.rows = rows.len() as i32;
}
