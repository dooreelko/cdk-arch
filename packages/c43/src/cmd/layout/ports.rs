//! Port assignment

use super::model::{Band, LayoutError, Model, Node, Port, GUTTER_W};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Outbound side. Right is fine for outbound (data flows left-to-right); the
/// inbound side is chosen separately by `inbound_side`.
fn default_from_side(src: &Node, dst: &Node) -> &'static str {
    if dst.grid_col > src.grid_col {
        "right"
    } else if dst.grid_col < src.grid_col {
        "left"
    } else if dst.grid_row > src.grid_row {
        "bottom"
    } else {
        "top" // same column upward, and self-loops
    }
}

fn sign(v: i64) -> i64 {
    (v > 0) as i64 - (v < 0) as i64
}

/// Inward approach unit vector for each inbound side.
fn inbound_vec(side: &str) -> (i64, i64) {
    match side {
        "left" => (1, 0),
        "top" => (0, 1),
        "bottom" => (0, -1),
        _ => unreachable!(),
    }
}

/// The lane cell an edge sits in just before stepping into the port.
fn inbound_pre_cell(dst: &Node, side: &str) -> (i64, i64) {
    match side {
        "left" => (dst.x - 1, dst.y + dst.h / 2),
        "top" => (dst.x + dst.w / 2, dst.y - 1),
        // bottom
        _ => (dst.x + dst.w / 2, dst.y + dst.h),
    }
}

/// Minimum bends of a Manhattan path from source point `(ax, ay)` to the
/// pre-port cell `pre`, given the forced final approach direction `d`.
fn elbows(ax: i64, ay: i64, pre: (i64, i64), d: (i64, i64)) -> i64 {
    let (bx, by) = pre;
    let (dx, dy) = (bx - ax, by - ay);
    if dx == 0 && dy == 0 {
        return 0;
    }
    if d.0 != 0 {
        // horizontal approach (left port)
        if dy == 0 {
            if dx == 0 || sign(dx) == d.0 {
                0
            } else {
                2
            }
        } else if dx == 0 {
            1
        } else if sign(dx) == d.0 {
            1
        } else {
            2
        }
    } else {
        // vertical approach (top/bottom port)
        if dx == 0 {
            if dy == 0 || sign(dy) == d.1 {
                0
            } else {
                2
            }
        } else if dy == 0 {
            1
        } else if sign(dy) == d.1 {
            1
        } else {
            2
        }
    }
}

/// `(kind, center)` of the band containing coordinate `v`, or `(None, None)`.
fn band(bands: &[Band], v: i64) -> (Option<&'static str>, Option<i64>) {
    for b in bands {
        if b.start <= v && v < b.end {
            return (Some(b.kind), b.center);
        }
    }
    (None, None)
}

/// True if stepping out of `dst`'s `side` eventually exits the node region into
/// an in-bounds lane (not the gutter, not off-canvas).
fn side_reaches_lane(m: &Model, dst: &Node, side: &str) -> bool {
    let (dx, dy) = match side {
        "left" => (-1, 0),
        "top" => (0, -1),
        _ => (0, 1), // bottom
    };
    let (mut x, mut y) = match side {
        "left" => (dst.x - 1, dst.y + dst.h / 2),
        "top" => (dst.x + dst.w / 2, dst.y - 1),
        _ => (dst.x + dst.w / 2, dst.y + dst.h),
    };
    while GUTTER_W < x && x < m.canvas_w && 0 <= y && y < m.canvas_h {
        let kind_c = band(&m.col_bands, x).0;
        let kind_r = band(&m.row_bands, y).0;
        if kind_c == Some("lane") || (kind_r == Some("lane") && kind_c.is_some()) {
            return true;
        }
        if !(band(&m.col_bands, x).0 == Some("node") && band(&m.row_bands, y).0 == Some("node")) {
            return false; // left the node region but not into a routing lane
        }
        x += dx;
        y += dy;
    }
    false
}

/// Choose the target's inbound side. Right is prohibited; among left/top/bottom
/// that reach a routing lane, rank by fewest elbows, ties broken left<bottom<top.
fn inbound_side(m: &Model, src: &Node, dst: &Node) -> &'static str {
    let sx = src.x + src.w / 2;
    let sy = src.y + src.h / 2;
    let order = |s: &str| -> i64 {
        match s {
            "left" => 0,
            "bottom" => 1,
            _ => 2, // top
        }
    };
    let mut best: Option<((i64, i64), &'static str)> = None;
    for side in ["left", "top", "bottom"] {
        if !side_reaches_lane(m, dst, side) {
            continue;
        }
        let e = elbows(sx, sy, inbound_pre_cell(dst, side), inbound_vec(side));
        let kk = (e, order(side));
        if best.is_none() || kk < best.unwrap().0 {
            best = Some((kk, side));
        }
    }
    best.map(|b| b.1).unwrap_or("bottom")
}

fn default_sides(m: &Model, src: &Node, dst: &Node) -> (&'static str, &'static str) {
    (default_from_side(src, dst), inbound_side(m, src, dst))
}

/// Map a validated hint side string to a static str.
fn side_static(s: &str) -> &'static str {
    match s {
        "left" => "left",
        "right" => "right",
        "top" => "top",
        "bottom" => "bottom",
        _ => unreachable!(),
    }
}

pub fn assign_ports(m: &mut Model) {
    // node id -> index into m.nodes
    let by_id: HashMap<String, usize> = m
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    // 1. decide a side for each (edge, endpoint). plan[edge_index] = (from, to).
    let mut plan: Vec<(&'static str, &'static str)> = Vec::with_capacity(m.edges.len());
    for e in &m.edges {
        let src = &m.nodes[by_id[&e.from_id]];
        let dst = &m.nodes[by_id[&e.to_id]];
        let (mut fs, mut ts) = default_sides(m, src, dst);
        if let Some(h) = m.hint_ports.get(&e.id) {
            if let Some(hfs) = &h.from_side {
                fs = side_static(hfs);
            }
            if let Some(hts) = &h.to_side {
                ts = side_static(hts);
            }
        }
        plan.push((fs, ts));
    }

    // 2. group ports by (node_id, side), preserving INSERTION order. Value is a
    // list of (edge_index, end) where end is "from"/"to".
    let mut groups: IndexMap<(String, String), Vec<(usize, &'static str)>> = IndexMap::new();
    for (idx, e) in m.edges.iter().enumerate() {
        let (fs, ts) = plan[idx];
        groups
            .entry((e.from_id.clone(), fs.to_string()))
            .or_default()
            .push((idx, "from"));
        groups
            .entry((e.to_id.clone(), ts.to_string()))
            .or_default()
            .push((idx, "to"));
    }

    // Collect mutations and apply after, to avoid borrow conflicts.
    let mut port_assignments: Vec<(usize, &'static str, Port)> = Vec::new();
    let mut new_errors: Vec<LayoutError> = Vec::new();

    for ((node_id, side_str), members) in &groups {
        let node = &m.nodes[by_id[node_id]];
        let side: &str = side_str.as_str();

        // order by the other endpoint's center; stable sort preserves insertion
        // order for equal keys.
        let mut members = members.clone();
        members.sort_by(|a, b| {
            let key = |item: &(usize, &'static str)| -> f64 {
                let e = &m.edges[item.0];
                let other_id = if e.from_id == node.id {
                    &e.to_id
                } else {
                    &e.from_id
                };
                let other = &m.nodes[by_id[other_id]];
                if side == "left" || side == "right" {
                    other.y as f64 + other.h as f64 / 2.0
                } else {
                    other.x as f64 + other.w as f64 / 2.0
                }
            };
            key(a).partial_cmp(&key(b)).unwrap()
        });

        let cap = if side == "left" || side == "right" {
            node.h - 2
        } else {
            node.w - 2
        };

        let n_ports = members.len() as i64;
        if n_ports > cap {
            let overflow_ids: Vec<String> = members[cap as usize..]
                .iter()
                .map(|(idx, _)| m.edges[*idx].id.clone())
                .collect();
            new_errors.push(LayoutError {
                code: "validation".to_string(),
                edge_ids: overflow_ids,
                at: None,
                message: format!(
                    "node '{}': {} ports on {} side, capacity {}",
                    node_id, n_ports, side, cap
                ),
                suggestion:
                    "move some edges to another side via hints.ports, or move neighbor nodes to other grid columns/rows"
                        .to_string(),
            });
        }

        let assigned_count = std::cmp::min(n_ports, cap);
        for (i, (idx, end)) in members[..assigned_count as usize].iter().enumerate() {
            let i = i as i64;
            let port = if side == "left" || side == "right" {
                let x = if side == "left" {
                    node.x
                } else {
                    node.x + node.w - 1
                };
                let y = node.y + 1 + (i + 1) * (node.h - 2) / (assigned_count + 1);
                Port {
                    side: side.to_string(),
                    x,
                    y,
                }
            } else {
                let y = if side == "top" {
                    node.y
                } else {
                    node.y + node.h - 1
                };
                let x = node.x + 1 + (i + 1) * (node.w - 2) / (assigned_count + 1);
                Port {
                    side: side.to_string(),
                    x,
                    y,
                }
            };
            port_assignments.push((*idx, *end, port));
        }
    }

    for (idx, end, port) in port_assignments {
        if end == "from" {
            m.edges[idx].from_port = Some(port);
        } else {
            m.edges[idx].to_port = Some(port);
        }
    }
    m.errors.extend(new_errors);
}
