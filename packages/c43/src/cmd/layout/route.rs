//! Routing: A*/Dijkstra grid router.
//!
//! Port of `layout.py` lines 465-698. Strictly lexicographic cost
//! `(crossings, adjacency, turns, centre_offset, length)` with an insertion
//! counter tiebreak so cells/dirs are never compared on cost ties — this is
//! what guarantees byte-identical output against the Python reference.

use super::model::{Band, LayoutError, Model, Port};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub use super::model::GUTTER_W;

/// Lexicographic cost: (crossings, adjacency, turns, centre_offset, length).
type Cost = (i64, i64, i64, i64, i64);

const INF_COST: Cost = (1 << 62, 1 << 62, 1 << 62, 1 << 62, 1 << 62);

const KING: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// `(kind, center)` of the band containing coordinate `v`, or `(None, None)`.
/// Exposed for tests (mirrors Python `_band`).
pub fn band(bands: &[Band], v: i64) -> (Option<&'static str>, Option<i64>) {
    for b in bands {
        if b.start <= v && v < b.end {
            return (Some(b.kind), b.center);
        }
    }
    (None, None)
}

/// A node cell -- the box and its margins. Edges may only enter here on their
/// port stubs; the body of every route stays in the lane bands.
pub fn is_node_region(m: &Model, x: i64, y: i64) -> bool {
    0 <= x
        && x < m.canvas_w
        && 0 <= y
        && y < m.canvas_h
        && m.col_kind[x as usize] == Some("node")
        && m.row_kind[y as usize] == Some("node")
}

/// Cells never routable for the body of a route: every node-region cell plus
/// the whole title band. Per-edge port stubs are carved out in `route_all`.
fn build_blocked(m: &Model) -> HashSet<(i64, i64)> {
    let mut blocked = HashSet::new();
    for cb in &m.col_bands {
        if cb.kind != "node" {
            continue;
        }
        for rb in &m.row_bands {
            if rb.kind != "node" {
                continue;
            }
            for x in cb.start..cb.end {
                for y in rb.start..rb.end {
                    blocked.insert((x, y));
                }
            }
        }
    }
    // title band: full canvas width, never a routing surface
    for rb in &m.row_bands {
        if rb.kind == "title" {
            for x in (GUTTER_W + 1)..m.canvas_w {
                for y in rb.start..rb.end {
                    blocked.insert((x, y));
                }
            }
        }
    }
    blocked
}

fn port_exit(port: &Port) -> (i64, i64) {
    match port.side.as_str() {
        "left" => (port.x - 1, port.y),
        "right" => (port.x + 1, port.y),
        "top" => (port.x, port.y - 1),
        _ => (port.x, port.y + 1),
    }
}

/// The straight corridor from a port out to the first lane band.
fn port_stub(m: &Model, port: &Port) -> Vec<(i64, i64)> {
    let (dx, dy) = match port.side.as_str() {
        "left" => (-1, 0),
        "right" => (1, 0),
        "top" => (0, -1),
        _ => (0, 1),
    };
    let (mut x, mut y) = port_exit(port);
    let mut cells = Vec::new();
    while GUTTER_W < x && x < m.canvas_w && 0 <= y && y < m.canvas_h && is_node_region(m, x, y) {
        cells.push((x, y));
        x += dx;
        y += dy;
    }
    cells
}

/// Uniform-cost (Dijkstra) search on the 4-connected grid, no heuristic.
/// Returns `(cells, crossing_cells)` or `(None, None)`.
#[allow(clippy::too_many_arguments)]
pub fn astar(
    start: (i64, i64),
    goal: (i64, i64),
    blocked: &HashSet<(i64, i64)>,
    occupied: &HashMap<(i64, i64), String>,
    forbidden: &HashSet<(i64, i64)>,
    allow_cross: bool,
    m: &Model,
) -> (Option<Vec<(i64, i64)>>, Option<Vec<(i64, i64)>>) {
    let (w, h) = (m.canvas_w, m.canvas_h);
    let in_bounds = |c: (i64, i64)| GUTTER_W < c.0 && c.0 < w && 0 <= c.1 && c.1 < h;

    if !in_bounds(start)
        || !in_bounds(goal)
        || blocked.contains(&start)
        || blocked.contains(&goal)
    {
        return (None, None);
    }
    if !allow_cross && occupied.contains_key(&start) {
        return (None, None);
    }

    type Dir = Option<(i64, i64)>;
    type State = ((i64, i64), Dir);

    let mut counter: u64 = 0;
    // Reverse so the smallest (cost, counter) pops first. counter is unique per
    // push so cell/dir fields are never actually compared.
    let mut pq: BinaryHeap<Reverse<(Cost, u64, (i64, i64), Dir)>> = BinaryHeap::new();
    pq.push(Reverse(((0, 0, 0, 0, 0), counter, start, None)));
    counter += 1;

    let mut best: HashMap<State, Cost> = HashMap::new();
    best.insert((start, None), (0, 0, 0, 0, 0));
    let mut came: HashMap<State, State> = HashMap::new();

    while let Some(Reverse((cost, _, pos, dirn))) = pq.pop() {
        if cost > *best.get(&(pos, dirn)).unwrap_or(&INF_COST) {
            continue;
        }
        if pos == goal {
            let mut cells = vec![pos];
            let mut st: State = (pos, dirn);
            while let Some(&prev) = came.get(&st) {
                st = prev;
                cells.push(st.0);
            }
            cells.reverse();
            let crossings: Vec<(i64, i64)> = cells
                .iter()
                .filter(|c| occupied.contains_key(*c))
                .copied()
                .collect();
            return (Some(cells), Some(crossings));
        }
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let np = (pos.0 + d.0, pos.1 + d.1);
            if !in_bounds(np) {
                continue;
            }
            if blocked.contains(&np) && np != goal {
                continue;
            }
            let crossed = occupied.contains_key(&np);
            if crossed && !allow_cross {
                continue;
            }
            let (nx, ny) = np;
            let adj = np != goal
                && forbidden.contains(&np)
                && !(m.col_kind[nx as usize] == Some("node")
                    && m.row_kind[ny as usize] == Some("node"));
            if adj && !allow_cross {
                continue;
            }
            let (crs, adjc, turns, off, length) = cost;
            let turn = if dirn.is_some() && Some(d) != dirn {
                1
            } else {
                0
            };
            let center = if d.1 == 0 {
                m.row_center[ny as usize]
            } else {
                m.col_center[nx as usize]
            };
            let offset = match center {
                Some(c) => ((if d.1 == 0 { ny } else { nx }) - c).abs(),
                None => 0,
            };
            let ncost: Cost = (
                crs + if crossed { 1 } else { 0 },
                adjc + if adj { 1 } else { 0 },
                turns + turn,
                off + offset,
                length + 1,
            );
            let nstate: State = (np, Some(d));
            if ncost < *best.get(&nstate).unwrap_or(&INF_COST) {
                best.insert(nstate, ncost);
                came.insert(nstate, (pos, dirn));
                pq.push(Reverse((ncost, counter, np, Some(d))));
                counter += 1;
            }
        }
    }
    (None, None)
}

/// Keep endpoints; drop collinear interior points.
fn to_polyline(cells: &[(i64, i64)]) -> Vec<[i64; 2]> {
    let mut poly = vec![[cells[0].0, cells[0].1]];
    for i in 1..cells.len().saturating_sub(1) {
        let a = cells[i - 1];
        let b = cells[i];
        let c = cells[i + 1];
        if !((a.0 == b.0 && b.0 == c.0) || (a.1 == b.1 && b.1 == c.1)) {
            poly.push([b.0, b.1]);
        }
    }
    let last = cells[cells.len() - 1];
    poly.push([last.0, last.1]);
    poly
}

fn manhattan(e: &super::model::Edge) -> i64 {
    let f = e.from_port.as_ref().unwrap();
    let t = e.to_port.as_ref().unwrap();
    (f.x - t.x).abs() + (f.y - t.y).abs()
}

/// Group consecutive path cells claimed by the same owner.
pub fn crossing_runs(
    cells: &[(i64, i64)],
    occupied: &HashMap<(i64, i64), String>,
) -> Vec<(String, (i64, i64))> {
    let mut runs = Vec::new();
    let mut prev_owner: Option<&String> = None;
    for c in cells {
        let owner = occupied.get(c);
        if let Some(o) = owner {
            if Some(o) != prev_owner {
                runs.push((o.clone(), *c));
            }
        }
        prev_owner = owner;
    }
    runs
}

pub fn route_all(m: &mut Model) {
    let base_blocked = build_blocked(m);
    let mut occupied: HashMap<(i64, i64), String> = HashMap::new();
    let mut forbidden: HashSet<(i64, i64)> = HashSet::new();

    // routable edge indices (both ports present)
    let order_index: HashMap<&String, usize> = m
        .routing_order
        .iter()
        .enumerate()
        .map(|(i, eid)| (eid, i))
        .collect();
    let order_len = order_index.len();

    let mut ordered: Vec<usize> = (0..m.edges.len())
        .filter(|&i| m.edges[i].from_port.is_some() && m.edges[i].to_port.is_some())
        .collect();
    // STABLE sort by (order_index or len, manhattan)
    ordered.sort_by_key(|&i| {
        let e = &m.edges[i];
        let oi = order_index.get(&e.id).copied().unwrap_or(order_len);
        (oi, manhattan(e))
    });

    let attempt = |m: &Model,
                   occupied: &HashMap<(i64, i64), String>,
                   forbidden: &HashSet<(i64, i64)>,
                   e: &super::model::Edge,
                   allow_cross: bool| {
        let mut stubs: HashSet<(i64, i64)> = HashSet::new();
        stubs.extend(port_stub(m, e.from_port.as_ref().unwrap()));
        stubs.extend(port_stub(m, e.to_port.as_ref().unwrap()));
        let blocked: HashSet<(i64, i64)> = base_blocked.difference(&stubs).copied().collect();
        astar(
            port_exit(e.from_port.as_ref().unwrap()),
            port_exit(e.to_port.as_ref().unwrap()),
            &blocked,
            occupied,
            forbidden,
            allow_cross,
            m,
        )
    };

    fn claim(
        occupied: &mut HashMap<(i64, i64), String>,
        forbidden: &mut HashSet<(i64, i64)>,
        id: &str,
        cells: &[(i64, i64)],
    ) {
        for &c in cells {
            occupied.entry(c).or_insert_with(|| id.to_string());
            for (kx, ky) in KING {
                forbidden.insert((c.0 + kx, c.1 + ky));
            }
        }
    }

    // Pass 1: no crossings allowed.
    let mut failed: Vec<usize> = Vec::new();
    for &i in &ordered {
        let (cells, _) = attempt(m, &occupied, &forbidden, &m.edges[i], false);
        match cells {
            None => failed.push(i),
            Some(cells) => {
                let poly = to_polyline(&cells);
                let id = m.edges[i].id.clone();
                m.edges[i].route = Some(poly);
                claim(&mut occupied, &mut forbidden, &id, &cells);
            }
        }
    }

    // Pass 2: failed only, crossings ok.
    for &i in &failed {
        let (cells, _) = attempt(m, &occupied, &forbidden, &m.edges[i], true);
        let id = m.edges[i].id.clone();
        match cells {
            None => {
                m.errors.push(LayoutError {
                    code: "unroutable".to_string(),
                    edge_ids: vec![id.clone()],
                    at: None,
                    message: format!("edge {id} could not be routed even with crossings"),
                    suggestion: "move its endpoints to adjacent grid cells \
                                 (grid_col/grid_row), pick other sides via hints.ports, \
                                 or free a lane via hints.routing_order"
                        .to_string(),
                });
            }
            Some(cells) => {
                for (owner, c) in crossing_runs(&cells, &occupied) {
                    m.errors.push(LayoutError {
                        code: "crossing".to_string(),
                        edge_ids: vec![id.clone(), owner.clone()],
                        at: Some([c.0, c.1]),
                        message: format!("edges {id} and {owner} cross at [{}, {}]", c.0, c.1),
                        suggestion: "reorder with hints.routing_order, adjust port sides \
                                     via hints.ports, or move a node to open a parallel track"
                            .to_string(),
                    });
                }
                let poly = to_polyline(&cells);
                m.edges[i].route = Some(poly);
                claim(&mut occupied, &mut forbidden, &id, &cells);
            }
        }
    }
}
