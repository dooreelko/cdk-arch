use c43::cmd::layout::model::{build_band_caches, Band, Model};
use c43::cmd::layout::route::{astar, band, crossing_runs, is_node_region, route_all, GUTTER_W};
use c43::cmd::layout::{geometry::geometry, parse::parse_and_validate, ports::assign_ports};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn prep(nodes: Value, edges: Value, hints: Option<Value>) -> Model {
    let mut raw = json!({"title": "T", "description": "D", "nodes": nodes, "edges": edges});
    if let Some(h) = hints {
        raw["hints"] = h;
    }
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    route_all(&mut m);
    m
}

fn is_axis_aligned(route: &[[i64; 2]]) -> bool {
    (0..route.len().saturating_sub(1))
        .all(|i| route[i][0] == route[i + 1][0] || route[i][1] == route[i + 1][1])
}

fn route_cells(route: &[[i64; 2]]) -> HashSet<(i64, i64)> {
    let mut out = HashSet::new();
    for w in route.windows(2) {
        let (x0, y0) = (w[0][0], w[0][1]);
        let (x1, y1) = (w[1][0], w[1][1]);
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

/// Adapter mirroring the Python `_astar` test helper: build a minimal Model
/// whose bands span the whole canvas as a single lane (centre off so centering
/// is inert), exercising bounds/crossing/spacing logic with explicit coords.
fn astar_adapter(
    start: (i64, i64),
    goal: (i64, i64),
    blocked: &HashSet<(i64, i64)>,
    occupied: &HashMap<(i64, i64), String>,
    allow_cross: bool,
    w: i64,
    h: i64,
    forbidden: Option<&HashSet<(i64, i64)>>,
) -> (Option<Vec<(i64, i64)>>, Option<Vec<(i64, i64)>>) {
    let mut m = Model::new(String::new(), String::new(), vec![], vec![]);
    m.canvas_w = w;
    m.canvas_h = h;
    m.col_bands = vec![Band {
        start: 0,
        end: w,
        kind: "lane",
        center: None,
    }];
    m.row_bands = vec![Band {
        start: 0,
        end: h,
        kind: "lane",
        center: None,
    }];
    build_band_caches(&mut m);
    let empty = HashSet::new();
    let forbidden = forbidden.unwrap_or(&empty);
    let no_border = HashSet::new();
    astar(
        start, goal, blocked, occupied, forbidden, allow_cross, &no_border, &no_border, &m,
    )
}

fn two_nodes() -> Value {
    json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
    ])
}

#[test]
fn straight_line_route() {
    let m = prep(two_nodes(), json!([{"id": "e1", "from": "a", "to": "b"}]), None);
    let e = &m.edges[0];
    let route = e.route.as_ref().unwrap();
    assert!(route.len() >= 2);
    assert!(is_axis_aligned(route));
    assert!(m.errors.is_empty());
}

#[test]
fn diagonal_route_bends_through_lane_center() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 1},
    ]);
    let m = prep(nodes, json!([{"id": "e1", "from": "a", "to": "b"}]), None);
    let route = m.edges[0].route.as_ref().unwrap();
    assert!(is_axis_aligned(route));
    // A diagonal cannot cut a single elbow through the box rows; it steps into
    // the vertical lane and drops down its centre -> a Z with two bends.
    assert_eq!(route.len(), 4);
    let lane_center = band(&m.col_bands, route[1][0]).1.unwrap();
    assert_eq!(route[1][0], lane_center);
    assert_eq!(route[2][0], lane_center);
    assert!(m.errors.is_empty());
}

#[test]
fn parallel_edges_use_distinct_tracks() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
        {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1},
    ]);
    let edges = json!([
        {"id": "e1", "from": "a", "to": "b"},
        {"id": "e2", "from": "c", "to": "d"},
    ]);
    let m = prep(nodes, edges, None);
    let c0 = route_cells(m.edges[0].route.as_ref().unwrap());
    let c1 = route_cells(m.edges[1].route.as_ref().unwrap());
    assert!(c0.intersection(&c1).next().is_none());
    assert!(m.errors.is_empty());
}

#[test]
fn routes_avoid_boxes_and_gutter() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
        {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1},
    ]);
    let edges = json!([{"id": "e1", "from": "a", "to": "d"}]);
    let m = prep(nodes, edges, None);
    let cells = route_cells(m.edges[0].route.as_ref().unwrap());
    for n in &m.nodes {
        let mut box_cells = HashSet::new();
        for x in n.x..n.x + n.w {
            for y in n.y..n.y + n.h {
                box_cells.insert((x, y));
            }
        }
        assert!(cells.intersection(&box_cells).next().is_none());
    }
    assert!(cells.iter().all(|(x, _)| *x > GUTTER_W));
}

#[test]
fn routes_respect_routing_order() {
    let m = prep(
        two_nodes(),
        json!([{"id": "e1", "from": "a", "to": "b"}]),
        Some(json!({"routing_order": ["e1"]})),
    );
    assert!(m.edges[0].route.is_some());
}

#[test]
fn all_edges_routed_or_reported() {
    let m = prep(two_nodes(), json!([{"id": "e1", "from": "a", "to": "b"}]), None);
    for e in &m.edges {
        assert!(
            e.route.is_some()
                || m.errors.iter().any(|err| err.edge_ids.contains(&e.id))
        );
    }
}

fn fan_nodes() -> Value {
    json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 1},
        {"id": "t0", "label": "t0", "grid_col": 1, "grid_row": 0},
        {"id": "t1", "label": "t1", "grid_col": 1, "grid_row": 1},
        {"id": "t2", "label": "t2", "grid_col": 1, "grid_row": 2},
    ])
}

fn fan_edges() -> Value {
    json!([
        {"id": "e0", "from": "a", "to": "t0"},
        {"id": "e1", "from": "a", "to": "t1"},
        {"id": "e2", "from": "a", "to": "t2"},
    ])
}

#[test]
fn parallel_segments_keep_a_gap() {
    let m = prep(fan_nodes(), fan_edges(), None);
    assert!(m.errors.is_empty());
    let routes: Vec<HashSet<(i64, i64)>> = m
        .edges
        .iter()
        .map(|e| route_cells(e.route.as_ref().unwrap()))
        .collect();
    for i in 0..routes.len() {
        for j in i + 1..routes.len() {
            assert!(routes[i].intersection(&routes[j]).next().is_none());
        }
    }
    let mut cols: HashSet<i64> = HashSet::new();
    for e in &m.edges {
        let r = e.route.as_ref().unwrap();
        for w in r.windows(2) {
            let (x0, y0) = (w[0][0], w[0][1]);
            let (x1, y1) = (w[1][0], w[1][1]);
            if x0 == x1 && y0 != y1 {
                cols.insert(x0);
            }
        }
    }
    let mut ordered: Vec<i64> = cols.into_iter().collect();
    ordered.sort();
    assert!(
        ordered.windows(2).all(|w| w[1] - w[0] >= 2),
        "{:?}",
        ordered
    );
}

#[test]
fn no_two_edges_share_a_2x2_block_in_lanes() {
    let m = prep(fan_nodes(), fan_edges(), None);
    assert!(m.errors.is_empty());
    let mut owner: HashMap<(i64, i64), String> = HashMap::new();
    for e in &m.edges {
        for c in route_cells(e.route.as_ref().unwrap()) {
            if is_node_region(&m, c.0, c.1) {
                continue;
            }
            owner.entry(c).or_insert_with(|| e.id.clone());
        }
    }
    let king = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    for (&(x, y), eid) in &owner {
        for (dx, dy) in king {
            if let Some(other) = owner.get(&(x + dx, y + dy)) {
                assert!(
                    other == eid,
                    "edges {} and {} share a 2x2 block near {:?}",
                    eid,
                    other,
                    (x, y)
                );
            }
        }
    }
}

#[test]
fn astar_pass1_rejects_occupied_start() {
    let mut occ = HashMap::new();
    occ.insert((10, 5), "z".to_string());
    let (cells, crossings) =
        astar_adapter((10, 5), (20, 5), &HashSet::new(), &occ, false, 30, 20, None);
    assert!(cells.is_none() && crossings.is_none());
}

#[test]
fn pass2_minimizes_crossings_over_turns() {
    let mut occ = HashMap::new();
    for x in 11..20 {
        occ.insert((x, 12), "z".to_string());
    }
    let (cells, crossings) =
        astar_adapter((10, 12), (20, 12), &HashSet::new(), &occ, true, 30, 20, None);
    assert!(cells.is_some());
    assert_eq!(crossings.unwrap(), Vec::<(i64, i64)>::new());
}

#[test]
fn pass2_prefers_gapped_track_over_hugging() {
    let mut occ = HashMap::new();
    for y in 5..26 {
        occ.insert((15, y), "z".to_string());
    }
    let mut forbidden = HashSet::new();
    let king = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    for &(x, y) in occ.keys() {
        for (dx, dy) in king {
            forbidden.insert((x + dx, y + dy));
        }
    }
    let (cells, _) = astar_adapter(
        (12, 5),
        (12, 25),
        &HashSet::new(),
        &occ,
        true,
        30,
        30,
        Some(&forbidden),
    );
    let cells = cells.unwrap();
    assert!(
        cells.iter().all(|c| c.0 <= 13),
        "{:?}",
        cells.iter().filter(|c| c.0 > 13).collect::<Vec<_>>()
    );
}

#[test]
fn crossing_runs_groups_consecutive_cells() {
    let cells = vec![
        (10, 1),
        (10, 2),
        (10, 3),
        (10, 4),
        (10, 5),
        (10, 6),
    ];
    let mut occ = HashMap::new();
    occ.insert((10, 2), "a".to_string());
    occ.insert((10, 3), "a".to_string());
    occ.insert((10, 5), "a".to_string());
    assert_eq!(
        crossing_runs(&cells, &occ),
        vec![("a".to_string(), (10, 2)), ("a".to_string(), (10, 5))]
    );
    let mut occ2 = HashMap::new();
    occ2.insert((10, 2), "a".to_string());
    occ2.insert((10, 3), "b".to_string());
    assert_eq!(
        crossing_runs(&cells, &occ2),
        vec![("a".to_string(), (10, 2)), ("b".to_string(), (10, 3))]
    );
    assert_eq!(crossing_runs(&cells, &HashMap::new()), vec![]);
}

#[test]
fn k5_forced_crossings_all_routed() {
    let coords = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1)];
    let nodes: Vec<Value> = coords
        .iter()
        .enumerate()
        .map(|(i, (c, r))| {
            json!({"id": format!("n{}", i + 1), "label": format!("n{}", i + 1),
                   "grid_col": c, "grid_row": r})
        })
        .collect();
    let mut edges = vec![];
    for a in 1..6 {
        for b in (a + 1)..6 {
            edges.push(json!({"id": format!("e{}{}", a, b),
                              "from": format!("n{}", a), "to": format!("n{}", b)}));
        }
    }
    let m = prep(json!(nodes), json!(edges), None);
    let crossing: Vec<_> = m.errors.iter().filter(|e| e.code == "crossing").collect();
    assert!(!crossing.is_empty());
    assert!(m.edges.iter().all(|e| e.route.is_some()));
    let mut keys = HashSet::new();
    for err in &crossing {
        let mut ids = err.edge_ids.clone();
        ids.sort();
        let at = err.at.unwrap();
        let key = (ids, at);
        assert!(keys.insert(key), "duplicate crossing error");
    }
}

#[test]
fn astar_start_blocked_unroutable() {
    let mut blocked = HashSet::new();
    blocked.insert((10, 5));
    let (cells, crossings) =
        astar_adapter((10, 5), (20, 5), &blocked, &HashMap::new(), true, 30, 20, None);
    assert!(cells.is_none() && crossings.is_none());
}

#[test]
fn astar_walled_goal_unroutable() {
    let goal = (20, 10);
    let mut blocked = HashSet::new();
    for c in [(19, 10), (21, 10), (20, 9), (20, 11)] {
        blocked.insert(c);
    }
    let (cells, crossings) =
        astar_adapter((10, 10), goal, &blocked, &HashMap::new(), true, 30, 20, None);
    assert!(cells.is_none() && crossings.is_none());
}

#[test]
fn overflow_edge_skipped_without_new_error() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 0, "grid_row": 1},
    ]);
    let edges: Vec<Value> = (0..4)
        .map(|i| json!({"id": format!("e{}", i), "from": "a", "to": "b"}))
        .collect();
    let m = prep(nodes, json!(edges), None);
    let overflow_errors: Vec<_> = m.errors.iter().filter(|e| e.code == "validation").collect();
    assert!(!overflow_errors.is_empty());
    let unported: Vec<_> = m
        .edges
        .iter()
        .filter(|e| e.from_port.is_none() || e.to_port.is_none())
        .collect();
    assert!(!unported.is_empty());
    for e in &unported {
        assert!(e.route.is_none());
    }
    let unported_ids: HashSet<&String> = unported.iter().map(|e| &e.id).collect();
    for err in &m.errors {
        if err.code == "unroutable" || err.code == "crossing" {
            assert!(err.edge_ids.iter().all(|id| !unported_ids.contains(id)));
        }
    }
}

#[test]
fn backward_edge_routes_without_errors() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
    ]);
    let m = prep(nodes, json!([{"id": "e1", "from": "b", "to": "a"}]), None);
    assert!(m.edges[0].route.is_some());
    assert!(m.errors.is_empty());
}
