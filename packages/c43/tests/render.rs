use c43::cmd::layout::model::Model;
use c43::cmd::layout::render::{render, render_with_observer, Canvas};
use c43::cmd::layout::{geometry::geometry, parse::parse_and_validate, ports::assign_ports, route::route_all};
use serde_json::{json, Value};
use std::fs;

/// parse -> geometry -> assign_ports -> route_all -> render to a temp path,
/// reading the file back. Returns (Model, rendered text).
fn render_to(nodes: Value, edges: Value) -> (Model, String) {
    let raw = json!({"title": "Sys", "description": "desc", "nodes": nodes, "edges": edges});
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    route_all(&mut m);
    let mut cv = Canvas::new(m.canvas_w, m.canvas_h);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("result.txt");
    render(&m, &mut cv, &out).unwrap();
    let txt = fs::read_to_string(&out).unwrap();
    (m, txt)
}

fn two() -> Value {
    json!([
        {"id": "a", "label": "aaa", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "bbb", "grid_col": 1, "grid_row": 0},
    ])
}

#[test]
fn canvas_paint_and_str() {
    let mut cv = Canvas::new(5, 2);
    cv.paint(1, 0, 'X');
    assert_eq!(cv.char_at(1, 0), 'X');
    assert!(cv.to_string().contains('X'));
}

#[test]
fn canvas_paint_out_of_bounds_is_noop() {
    let mut cv = Canvas::new(3, 3);
    cv.paint(-1, 0, 'X');
    cv.paint(0, -1, 'X');
    cv.paint(3, 0, 'X');
    cv.paint(0, 3, 'X');
    assert!(!cv.to_string().contains('X'));
}

#[test]
fn render_draws_boxes_and_labels() {
    let (_m, txt) = render_to(two(), json!([{"id": "e1", "from": "a", "to": "b"}]));
    assert!(txt.contains('+') && txt.contains('|'));
    assert!(txt.contains("aaa") && txt.contains("bbb"));
}

#[test]
fn render_includes_title_and_scaffolding() {
    let (_m, txt) = render_to(two(), json!([{"id": "e1", "from": "a", "to": "b"}]));
    assert!(txt.contains("Sys") && txt.contains("desc"));
    assert!(txt.contains('│') && txt.contains('─') && txt.contains('┼'));
    assert!(txt.contains("nodes") && txt.contains("edges") && txt.contains("title"));
}

#[test]
fn render_paints_edge_char_and_arrowhead() {
    let (m, txt) = render_to(two(), json!([{"id": "e1", "from": "a", "to": "b"}]));
    assert!(txt.contains(m.edges[0].char)); // edge body ('0')
    assert!(txt.contains('►')); // forward edge enters target's left side
    assert!(txt.contains('*')); // source port
}

#[test]
fn edge_body_uses_only_edge_char() {
    let (m, txt) = render_to(two(), json!([{"id": "e1", "from": "a", "to": "b"}]));
    let e = &m.edges[0];
    let lines: Vec<&str> = txt.split('\n').collect();
    let route = e.route.as_ref().unwrap();
    for w in route.windows(2) {
        let (x0, y0) = (w[0][0], w[0][1]);
        let (x1, y1) = (w[1][0], w[1][1]);
        if y0 == y1 {
            for x in x0.min(x1)..=x0.max(x1) {
                let ch = lines[y0 as usize].chars().nth(x as usize).unwrap();
                assert_eq!(ch, e.char);
            }
        }
    }
}

#[test]
fn incremental_save_after_each_mutation() {
    let raw = json!({"title": "Sys", "description": "desc", "nodes": two(),
                     "edges": [{"id": "e1", "from": "a", "to": "b"}]});
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    route_all(&mut m);
    let mut cv = Canvas::new(m.canvas_w, m.canvas_h);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("result.txt");
    let mut saves: Vec<String> = Vec::new();
    render_with_observer(&m, &mut cv, &out, &mut |c| saves.push(c.to_string())).unwrap();
    // scaffolding + 2 boxes + 1 edge => at least 4 saves
    assert!(saves.len() >= 4, "{} saves", saves.len());
    // progressive: each save's content differs from the previous
    assert!((0..saves.len() - 1).all(|i| saves[i] != saves[i + 1]));
}

#[test]
fn unrouted_edge_skipped() {
    // overflow fixture: narrow boxes, 4 same-column edges -> some have no route
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 0, "grid_row": 1},
    ]);
    let edges: Vec<Value> = (0..4)
        .map(|i| json!({"id": format!("e{}", i), "from": "a", "to": "b"}))
        .collect();
    let (m, txt) = render_to(nodes, json!(edges)); // must not crash
    let unrouted: Vec<_> = m.edges.iter().filter(|e| e.route.is_none()).collect();
    assert!(!unrouted.is_empty());
    assert!(txt.contains("title"));
}
