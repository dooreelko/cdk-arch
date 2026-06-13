use c43::cmd::layout::model::Model;
use c43::cmd::layout::{geometry::geometry, parse::parse_and_validate, ports::assign_ports};
use serde_json::{json, Value};

fn build(nodes: Value, edges: Value, hints: Option<Value>) -> Model {
    let mut raw = json!({"title": "T", "description": "D", "nodes": nodes, "edges": edges});
    if let Some(h) = hints {
        raw["hints"] = h;
    }
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    m
}

fn nodes_abc() -> Value {
    json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
    ])
}

#[test]
fn forward_edge_default_sides() {
    let m = build(nodes_abc(), json!([{"id": "e1", "from": "a", "to": "b"}]), None);
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "right");
    assert_eq!(e.to_port.as_ref().unwrap().side, "left");
}

#[test]
fn same_column_default_sides() {
    let m = build(nodes_abc(), json!([{"id": "e1", "from": "a", "to": "c"}]), None);
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "bottom");
    assert_eq!(e.to_port.as_ref().unwrap().side, "top");
}

#[test]
fn backward_edge_default_sides() {
    // Outbound leaves b's left; inbound right is prohibited, a's left faces
    // the gutter, so it falls to a non-right side.
    let m = build(nodes_abc(), json!([{"id": "e1", "from": "b", "to": "a"}]), None);
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "left");
    assert_ne!(e.to_port.as_ref().unwrap().side, "right");
    // route_all parity covered in Task 9
}

#[test]
fn inbound_never_uses_right_side() {
    let nodes = json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
        {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1},
    ]);
    let edges = json!([
        {"id": "e1", "from": "a", "to": "b"},
        {"id": "e2", "from": "b", "to": "a"},
        {"id": "e3", "from": "a", "to": "d"},
        {"id": "e4", "from": "d", "to": "a"},
    ]);
    let m = build(nodes, edges, None);
    for e in &m.edges {
        assert_ne!(
            e.to_port.as_ref().unwrap().side,
            "right",
            "edge {} got right inbound",
            e.id
        );
    }
}

#[test]
fn inbound_right_hint_rejected() {
    let raw = json!({
        "title": "T", "description": "D",
        "nodes": nodes_abc(),
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
        "hints": {"ports": [{"edge_id": "e1", "to_side": "right"}]},
    });
    assert!(parse_and_validate(&raw).is_err());
}

#[test]
fn hint_overrides_sides() {
    let m = build(
        nodes_abc(),
        json!([{"id": "e1", "from": "a", "to": "b"}]),
        Some(json!({"ports": [{"edge_id": "e1", "from_side": "top", "to_side": "bottom"}]})),
    );
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "top");
    assert_eq!(e.to_port.as_ref().unwrap().side, "bottom");
}

#[test]
fn ports_lie_on_box_border() {
    let m = build(nodes_abc(), json!([{"id": "e1", "from": "a", "to": "b"}]), None);
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    let p = m.edges[0].from_port.as_ref().unwrap();
    assert_eq!(p.x, a.x + a.w - 1);
    assert!(a.y <= p.y && p.y < a.y + a.h);
}

#[test]
fn multiple_ports_on_side_stack_on_distinct_rows() {
    let mut nodes = nodes_abc();
    nodes.as_array_mut().unwrap().push(
        json!({"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}),
    );
    let edges = json!([
        {"id": "e1", "from": "a", "to": "b"},
        {"id": "e2", "from": "a", "to": "d"},
    ]);
    let m = build(nodes, edges, None);
    let ys: std::collections::HashSet<i64> =
        m.edges.iter().map(|e| e.from_port.as_ref().unwrap().y).collect();
    assert_eq!(ys.len(), 2);
}

#[test]
fn port_overflow_reports_error_and_caps_assignments() {
    let nodes = json!([
        {"id": "t", "label": "x", "grid_col": 0, "grid_row": 1},
        {"id": "s0", "label": "y", "grid_col": 1, "grid_row": 0},
        {"id": "s1", "label": "y", "grid_col": 1, "grid_row": 1},
        {"id": "s2", "label": "y", "grid_col": 1, "grid_row": 2},
        {"id": "s3", "label": "y", "grid_col": 1, "grid_row": 3},
    ]);
    let edges = json!([
        {"id": "e0", "from": "s0", "to": "t"},
        {"id": "e1", "from": "s1", "to": "t"},
        {"id": "e2", "from": "s2", "to": "t"},
        {"id": "e3", "from": "s3", "to": "t"},
    ]);
    let hints = json!({"ports": [
        {"edge_id": "e0", "to_side": "top"},
        {"edge_id": "e1", "to_side": "top"},
        {"edge_id": "e2", "to_side": "top"},
        {"edge_id": "e3", "to_side": "top"},
    ]});
    let m = build(nodes, edges, Some(hints));

    let t_node = m.nodes.iter().find(|n| n.id == "t").unwrap();
    let cap = t_node.w - 2;
    assert_eq!(cap, 3);

    let assigned: Vec<_> = m.edges.iter().filter(|e| e.to_port.is_some()).collect();
    let overflow: Vec<_> = m.edges.iter().filter(|e| e.to_port.is_none()).collect();

    assert_eq!(assigned.len() as i64, std::cmp::min(4, cap));
    assert_eq!(overflow.len() as i64, std::cmp::max(0, 4 - cap));

    assert_eq!(m.errors.len(), 1);
    let err = &m.errors[0];
    assert_eq!(err.code, "validation");
    assert_eq!(err.edge_ids.len(), overflow.len());
    assert!(overflow.iter().all(|e| err.edge_ids.contains(&e.id)));
    assert!(err.message.contains("4 ports on top side"));
    assert!(err.message.contains(&format!("capacity {cap}")));
    assert!(err.message.contains("t"));
}

#[test]
fn port_ordering_by_target_position() {
    let nodes = json!([
        {"id": "h", "label": "h", "grid_col": 0, "grid_row": 1},
        {"id": "t0", "label": "t0", "grid_col": 1, "grid_row": 0},
        {"id": "t2", "label": "t2", "grid_col": 1, "grid_row": 2},
    ]);
    let edges = json!([
        {"id": "e_to_2", "from": "h", "to": "t2"},
        {"id": "e_to_0", "from": "h", "to": "t0"},
    ]);
    let m = build(nodes, edges, None);
    let e_to_0 = m.edges.iter().find(|e| e.id == "e_to_0").unwrap();
    let e_to_2 = m.edges.iter().find(|e| e.id == "e_to_2").unwrap();
    assert_eq!(e_to_0.from_port.as_ref().unwrap().side, "right");
    assert_eq!(e_to_2.from_port.as_ref().unwrap().side, "right");
    assert!(e_to_0.from_port.as_ref().unwrap().y < e_to_2.from_port.as_ref().unwrap().y);
}

#[test]
fn top_bottom_ports_stack_horizontally() {
    let nodes = json!([
        {"id": "src", "label": "src", "grid_col": 0, "grid_row": 0},
        {"id": "t0", "label": "t0", "grid_col": 0, "grid_row": 1},
        {"id": "t1", "label": "t1", "grid_col": 0, "grid_row": 2},
    ]);
    let edges = json!([
        {"id": "e0", "from": "src", "to": "t0"},
        {"id": "e1", "from": "src", "to": "t1"},
    ]);
    let m = build(nodes, edges, None);
    let e0 = m.edges.iter().find(|e| e.id == "e0").unwrap();
    let e1 = m.edges.iter().find(|e| e.id == "e1").unwrap();
    assert_eq!(e0.from_port.as_ref().unwrap().side, "bottom");
    assert_eq!(e1.from_port.as_ref().unwrap().side, "bottom");
    assert_ne!(e0.from_port.as_ref().unwrap().x, e1.from_port.as_ref().unwrap().x);
}

#[test]
fn convergent_left_side_ports_stack_vertically() {
    let nodes = json!([
        {"id": "s0", "label": "s0", "grid_col": 0, "grid_row": 0},
        {"id": "s1", "label": "s1", "grid_col": 0, "grid_row": 1},
        {"id": "target", "label": "target", "grid_col": 1, "grid_row": 0},
    ]);
    let edges = json!([
        {"id": "e0", "from": "s0", "to": "target"},
        {"id": "e1", "from": "s1", "to": "target"},
    ]);
    let m = build(nodes, edges, None);
    let e0 = m.edges.iter().find(|e| e.id == "e0").unwrap();
    let e1 = m.edges.iter().find(|e| e.id == "e1").unwrap();
    assert_eq!(e0.to_port.as_ref().unwrap().side, "left");
    assert_eq!(e1.to_port.as_ref().unwrap().side, "left");
    assert_ne!(e0.to_port.as_ref().unwrap().y, e1.to_port.as_ref().unwrap().y);
}

#[test]
fn self_loop_default_sides() {
    let nodes = json!([{"id": "n", "label": "n", "grid_col": 0, "grid_row": 0}]);
    let edges = json!([{"id": "loop", "from": "n", "to": "n"}]);
    let m = build(nodes, edges, None);
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "top");
    assert_eq!(e.to_port.as_ref().unwrap().side, "bottom");
}
