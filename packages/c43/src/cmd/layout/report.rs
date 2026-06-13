//! Quality and diagnostics + ordered result.json serialization.
//!
//! Ports the QUALITY / DIAGNOSTICS / result.json layer of `layout.py`
//! (lines 806-983). Results are built as `serde_json::Value` with keys
//! inserted in Python's emission order; with `serde_json`'s `preserve_order`
//! feature the `json!` macro preserves the literal written key order, so
//! `serde_json::to_string_pretty` produces byte-identical output to Python's
//! `json.dump(obj, indent=2)` (2-space indent, no trailing newline).

use super::model::{Edge, Model, Port};
use super::route::is_node_region;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// A routed edge is a "wrap" once its drawn length exceeds the straight
/// port-to-port distance by this many cells.
const WRAP_EXCESS: i64 = 100;
/// Report a congested edge-pair only once they run king-adjacent for at least
/// this many cells.
const CONGEST_MIN: i64 = 6;
const KING8: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    pub dropped: i64,
    pub crossings: i64,
    pub wraps: i64,
    pub top_ports: i64,
    pub congestion: i64,
    pub length: i64,
}

/// All grid cells a polyline passes through (inclusive of vertices).
pub fn route_cell_set(route: &[[i64; 2]]) -> HashSet<(i64, i64)> {
    let mut out = HashSet::new();
    for pair in route.windows(2) {
        let [x0, y0] = pair[0];
        let [x1, y1] = pair[1];
        if x0 == x1 {
            for y in y0.min(y1)..=y0.max(y1) {
                out.insert((x0, y));
            }
        } else {
            for x in x0.min(x1)..=x0.max(x1) {
                out.insert((x, y0));
            }
        }
    }
    out
}

fn manhattan(e: &Edge) -> i64 {
    let f = e.from_port.as_ref().unwrap();
    let t = e.to_port.as_ref().unwrap();
    (f.x - t.x).abs() + (f.y - t.y).abs()
}

/// Soft quality signals the visual-approval loop optimises against.
/// Returns (quality, diagnostics). `errors` stays the hard-failure channel
/// that drives `status`; these are advisory.
pub fn quality_and_diagnostics(m: &Model) -> (Quality, Vec<Value>) {
    let mut diagnostics: Vec<Value> = Vec::new();
    let by_id: HashMap<&str, &super::model::Node> =
        m.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // crossings: distinct crossing pairs already in m.errors
    let crossings = m
        .errors
        .iter()
        .filter(|e| e.code == "crossing")
        .map(|e| {
            let mut ids = e.edge_ids.clone();
            ids.sort();
            ids
        })
        .collect::<HashSet<Vec<String>>>()
        .len() as i64;

    // dropped: unroutable + validation errors (drawn nowhere)
    let dropped = m
        .errors
        .iter()
        .filter(|e| e.code == "unroutable" || e.code == "validation")
        .count() as i64;

    // wraps
    let mut wraps = 0i64;
    for e in &m.edges {
        let route = match &e.route {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        if e.from_port.is_none() || e.to_port.is_none() {
            continue;
        }
        let direct = manhattan(e);
        let routed = route_cell_set(route).len() as i64;
        if routed - direct > WRAP_EXCESS {
            wraps += 1;
            let at = route[0];
            diagnostics.push(json!({
                "code": "wrap",
                "edge_ids": [e.id],
                "at": [at[0], at[1]],
                "message": format!(
                    "edge {} loops the canvas (drawn {} cells vs {} direct)",
                    e.id, routed, direct
                ),
                "suggestion": "route it earlier via hints.routing_order, or pick a from_side facing its target",
            }));
        }
    }

    // top ports on non-top-row nodes
    let mut top_ports = 0i64;
    for e in &m.edges {
        for (end, p) in [("from", &e.from_port), ("to", &e.to_port)] {
            match p {
                Some(p) if p.side == "top" => {}
                _ => continue,
            };
            let nid = if end == "from" { &e.from_id } else { &e.to_id };
            if by_id[nid.as_str()].grid_row > 0 {
                top_ports += 1;
            }
        }
    }

    // congestion: king-adjacent lane cells between distinct edges
    let mut owner: HashMap<(i64, i64), String> = HashMap::new();
    for e in &m.edges {
        let route = match &e.route {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        for c in route_cell_set(route) {
            if is_node_region(m, c.0, c.1) {
                continue; // port stubs bunch legitimately
            }
            owner.entry(c).or_insert_with(|| e.id.clone()); // first writer wins
        }
    }
    let mut pair_cells: HashMap<(String, String), i64> = HashMap::new();
    for ((x, y), eid) in &owner {
        for (dx, dy) in KING8 {
            if let Some(o) = owner.get(&(x + dx, y + dy)) {
                if o != eid {
                    let key = if eid <= o {
                        (eid.clone(), o.clone())
                    } else {
                        (o.clone(), eid.clone())
                    };
                    *pair_cells.entry(key).or_insert(0) += 1;
                }
            }
        }
    }
    // sorted by pair key for deterministic diagnostic order
    let mut pairs: Vec<((String, String), i64)> = pair_cells.into_iter().collect();
    pairs.sort();
    let mut congestion = 0i64;
    for ((a, b), cnt) in pairs {
        let cnt = cnt / 2; // each adjacency counted twice
        congestion += cnt;
        if cnt >= CONGEST_MIN {
            diagnostics.push(json!({
                "code": "congestion",
                "edge_ids": [a, b],
                "at": Value::Null,
                "message": format!(
                    "edges {} and {} run parallel for {} cells with no gap",
                    a, b, cnt
                ),
                "suggestion": "give one of them a different port side via hints.ports so they take separate lanes",
            }));
        }
    }

    // length: total drawn edge length
    let length: i64 = m
        .edges
        .iter()
        .filter_map(|e| e.route.as_ref())
        .filter(|r| !r.is_empty())
        .map(|r| route_cell_set(r).len() as i64)
        .sum();

    let quality = Quality {
        dropped,
        crossings,
        wraps,
        top_ports,
        congestion,
        length,
    };
    (quality, diagnostics)
}

fn quality_json(q: &Quality) -> Value {
    json!({
        "dropped": q.dropped,
        "crossings": q.crossings,
        "wraps": q.wraps,
        "top_ports": q.top_ports,
        "congestion": q.congestion,
        "length": q.length,
    })
}

fn port_json(p: &Option<Port>) -> Value {
    match p {
        None => Value::Null,
        Some(p) => json!({"side": p.side, "x": p.x, "y": p.y}),
    }
}

fn route_json(route: &Option<Vec<[i64; 2]>>) -> Value {
    match route {
        None => Value::Null,
        Some(r) => Value::Array(
            r.iter()
                .map(|p| json!([p[0], p[1]]))
                .collect(),
        ),
    }
}

fn error_json(e: &super::model::LayoutError) -> Value {
    let at = match e.at {
        None => Value::Null,
        Some(a) => json!([a[0], a[1]]),
    };
    json!({
        "code": e.code,
        "edge_ids": e.edge_ids,
        "at": at,
        "message": e.message,
        "suggestion": e.suggestion,
    })
}

pub fn result_json(m: &Model) -> Value {
    let (quality, diagnostics) = quality_and_diagnostics(m);
    let nodes: Vec<Value> = m
        .nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id,
                "label": n.label,
                "grid_col": n.grid_col,
                "grid_row": n.grid_row,
                "x": n.x,
                "y": n.y,
                "w": n.w,
                "h": n.h,
            })
        })
        .collect();
    let edges: Vec<Value> = m
        .edges
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "from": e.from_id,
                "to": e.to_id,
                "char": e.char.to_string(),
                "from_port": port_json(&e.from_port),
                "to_port": port_json(&e.to_port),
                "route": route_json(&e.route),
            })
        })
        .collect();
    let groups: Vec<Value> = m
        .groups
        .iter()
        .map(|g| {
            json!({
                "id": g.id,
                "title": g.title,
                "parent": g.parent,
                "grid": {"col0": g.col0, "col1": g.col1, "row0": g.row0, "row1": g.row1},
                "x": g.x, "y": g.y, "w": g.w, "h": g.h,
            })
        })
        .collect();
    json!({
        "status": if m.errors.is_empty() { "ok" } else { "error" },
        "errors": m.errors.iter().map(error_json).collect::<Vec<_>>(),
        "quality": quality_json(&quality),
        "diagnostics": diagnostics,
        "title": m.title,
        "description": m.description,
        "canvas": {"width": m.canvas_w, "height": m.canvas_h},
        "box": {"width": m.box_w, "height": m.box_h},
        "nodes": nodes,
        "edges": edges,
        "groups": groups,
    })
}

/// Lexicographic objective for the visual-approval loop; lower is strictly
/// better. Order: (dropped, wraps, crossings, top_ports, congestion, length)
/// — note this differs from the quality dict's key order.
pub fn score_key(q: &Quality) -> (i64, i64, i64, i64, i64, i64) {
    (
        q.dropped,
        q.wraps,
        q.crossings,
        q.top_ports,
        q.congestion,
        q.length,
    )
}

pub fn quality_of(m: &Model) -> Quality {
    quality_and_diagnostics(m).0
}

/// Error result.json with the full top-level key set, so the consumer can rely
/// on every key existing regardless of which path wrote the file.
pub fn validation_error_result(raw: Option<&Value>, message: &str, suggestion: &str) -> Value {
    let as_str = |key: &str| -> String {
        raw.and_then(|r| r.as_object())
            .and_then(|o| o.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    json!({
        "status": "error",
        "errors": [{
            "code": "validation",
            "edge_ids": Vec::<String>::new(),
            "at": Value::Null,
            "message": message,
            "suggestion": suggestion,
        }],
        "quality": {
            "dropped": 0, "crossings": 0, "wraps": 0,
            "top_ports": 0, "congestion": 0, "length": 0,
        },
        "diagnostics": Vec::<Value>::new(),
        "title": as_str("title"),
        "description": as_str("description"),
        "canvas": {"width": 0, "height": 0},
        "box": {"width": 0, "height": 0},
        "nodes": Vec::<Value>::new(),
        "edges": Vec::<Value>::new(),
        "groups": Vec::<Value>::new(),
    })
}
