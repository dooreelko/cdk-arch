use c43::cmd::layout::model::{EDGE_ALPHABET, SIDES};
use c43::cmd::layout::parse::parse_and_validate;
use serde_json::json;

fn base() -> serde_json::Value {
    json!({
        "title": "T",
        "description": "D",
        "nodes": [
            {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
            {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}
        ],
        "edges": [{"id": "e1", "from": "a", "to": "b"}]
    })
}

#[test]
fn module_exposes_data_model() {
    assert_eq!(EDGE_ALPHABET.chars().count(), 62);
    assert_eq!(SIDES, ["left", "right", "top", "bottom"]);
}

#[test]
fn parse_ok_assigns_chars_in_order() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "e2", "from": "b", "to": "a"}));
    let m = parse_and_validate(&raw).unwrap();
    assert_eq!(
        m.edges.iter().map(|e| e.char).collect::<Vec<_>>(),
        vec!['0', '1']
    );
    assert_eq!(m.title, "T");
    assert_eq!(m.description, "D");
}

#[test]
fn duplicate_node_id() {
    let mut raw = base();
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "a", "label": "x", "grid_col": 2, "grid_row": 0}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "duplicate node id: a");
}

#[test]
fn duplicate_edge_id() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "e1", "from": "b", "to": "a"}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "duplicate edge id: e1");
}

#[test]
fn edge_unknown_node() {
    let mut raw = base();
    raw["edges"][0]["to"] = json!("zzz");
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "edge e1 references unknown node id: zzz");
}

#[test]
fn two_nodes_same_cell() {
    let mut raw = base();
    raw["nodes"][1]["grid_col"] = json!(0);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "two nodes in grid cell (0, 0): a and b");
}

#[test]
fn bad_hint_edge_id() {
    let mut raw = base();
    raw["hints"] = json!({"ports": [{"edge_id": "nope", "from_side": "right", "to_side": "left"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "hint references unknown edge_id: nope");
}

#[test]
fn bad_hint_side() {
    let mut raw = base();
    raw["hints"] =
        json!({"ports": [{"edge_id": "e1", "from_side": "sideways", "to_side": "left"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert!(
        err.contains("invalid side 'sideways'") && err.contains("must be one of"),
        "{err}"
    );
}

#[test]
fn too_many_edges() {
    let mut raw = base();
    let nodes: Vec<_> = (0..63)
        .map(|i| json!({"id": format!("n{i}"), "label": "x", "grid_col": i, "grid_row": 0}))
        .collect();
    let edges: Vec<_> = (1..64)
        .map(|i| json!({"id": format!("e{i}"), "from": "n0", "to": format!("n{i}")}))
        .collect();
    raw["nodes"] = json!(nodes);
    raw["edges"] = json!(edges);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "63 edges exceeds the 62-char alphabet; unicode edge alphabet is the future expansion path"
    );
}

#[test]
fn bad_routing_order_edge_id() {
    let mut raw = base();
    raw["hints"] = json!({"routing_order": ["nope"]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "routing_order references unknown edge_id: nope");
}

#[test]
fn missing_nodes_key() {
    let raw = json!({"title": "T", "description": "D", "edges": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "missing required key: nodes");
}

#[test]
fn missing_edges_key() {
    let raw = json!({"title": "T", "description": "D", "nodes": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "missing required key: edges");
}

#[test]
fn empty_edges_allowed() {
    let raw = json!({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
        "edges": []
    });
    let m = parse_and_validate(&raw).unwrap();
    assert!(m.edges.is_empty());
}

#[test]
fn empty_nodes_rejected() {
    let raw = json!({"title": "T", "description": "D", "nodes": [], "edges": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "nodes must not be empty");
}

#[test]
fn missing_node_id() {
    let mut raw = base();
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"label": "x", "grid_col": 2, "grid_row": 0}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "node at index 2 missing required field: id");
}

#[test]
fn missing_node_label() {
    let mut raw = base();
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "c", "grid_col": 2, "grid_row": 0}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 2 (id='c') missing required field: label"
    );
}

#[test]
fn missing_node_grid_col() {
    let mut raw = base();
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "c", "label": "C", "grid_row": 0}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 2 (id='c') missing required field: grid_col"
    );
}

#[test]
fn missing_node_grid_row() {
    let mut raw = base();
    raw["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "c", "label": "C", "grid_col": 2}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 2 (id='c') missing required field: grid_row"
    );
}

#[test]
fn missing_edge_id() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"from": "a", "to": "b"}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "edge at index 1 missing required field: id");
}

#[test]
fn missing_edge_from() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "e2", "to": "b"}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "edge at index 1 (id='e2') missing required field: from"
    );
}

#[test]
fn missing_edge_to() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "e2", "from": "a"}));
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "edge at index 1 (id='e2') missing required field: to");
}

#[test]
fn missing_hint_port_edge_id() {
    let mut raw = base();
    raw["hints"] = json!({"ports": [{"from_side": "right"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "hint port at index 0 missing required field: edge_id");
}

#[test]
fn grid_col_string_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_col"] = json!("0");
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_col must be an int >= 0, got '0'"
    );
}

#[test]
fn grid_row_string_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_row"] = json!("0");
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_row must be an int >= 0, got '0'"
    );
}

#[test]
fn grid_col_negative_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_col"] = json!(-1);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_col must be an int >= 0, got -1"
    );
}

#[test]
fn grid_row_negative_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_row"] = json!(-1);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_row must be an int >= 0, got -1"
    );
}

#[test]
fn grid_col_bool_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_col"] = json!(true);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_col must be an int >= 0, got True"
    );
}

#[test]
fn grid_row_bool_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_row"] = json!(false);
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "node at index 0 (id='a'): grid_row must be an int >= 0, got False"
    );
}

#[test]
fn duplicate_hint_ports() {
    let mut raw = base();
    raw["hints"] = json!({"ports": [
        {"edge_id": "e1", "from_side": "right"},
        {"edge_id": "e1", "to_side": "left"}
    ]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "duplicate hint port for edge_id: e1");
}

#[test]
fn duplicate_routing_order_ids() {
    let mut raw = base();
    raw["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "e2", "from": "b", "to": "a"}));
    raw["hints"] = json!({"routing_order": ["e1", "e2", "e1"]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "duplicate edge_id in routing_order: e1");
}

#[test]
fn missing_title_key() {
    let raw = json!({"description": "D", "nodes": [], "edges": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "missing required key: title");
}

#[test]
fn missing_description_key() {
    let raw = json!({"title": "T", "nodes": [], "edges": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(err, "missing required key: description");
}

#[test]
fn empty_title_allowed() {
    let raw = json!({
        "title": "", "description": "D",
        "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
        "edges": []
    });
    let m = parse_and_validate(&raw).unwrap();
    assert_eq!(m.title, "");
}

#[test]
fn empty_description_allowed() {
    let raw = json!({
        "title": "T", "description": "",
        "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
        "edges": []
    });
    let m = parse_and_validate(&raw).unwrap();
    assert_eq!(m.description, "");
}

#[test]
fn hint_side_error_names_edge_and_valid_sides() {
    let mut raw = base();
    raw["hints"] = json!({"ports": [{"edge_id": "e1", "from_side": "sideways"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "hint for edge e1: invalid side 'sideways', must be one of ('left', 'right', 'top', 'bottom')"
    );
}

#[test]
fn exactly_62_edges_passes() {
    let nodes: Vec<_> = (0..63)
        .map(|i| {
            json!({"id": format!("n{i}"), "label": "x", "grid_col": i % 10, "grid_row": i / 10})
        })
        .collect();
    let edges: Vec<_> = (0..62)
        .map(|i| json!({"id": format!("e{i}"), "from": "n0", "to": format!("n{}", i + 1)}))
        .collect();
    let raw = json!({"title": "T", "description": "D", "nodes": nodes, "edges": edges});
    let m = parse_and_validate(&raw).unwrap();
    assert_eq!(m.edges.len(), 62);
    assert_eq!(
        m.edges.last().unwrap().char,
        EDGE_ALPHABET.chars().nth(61).unwrap()
    );
}

#[test]
fn unknown_hint_key_rejected() {
    let mut raw = base();
    raw["hints"] = json!({"port": [{"edge_id": "e1"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "unknown key in hints: 'port', allowed keys are: ports, routing_order"
    );
}

#[test]
fn unknown_hint_key_reports_first_inserted() {
    // Two unknown keys; Python reports the first INSERTED ("zzz"), not the first sorted ("aaa").
    let mut raw = base();
    raw["hints"] = json!({"zzz": [], "aaa": []});
    let err = parse_and_validate(&raw).unwrap_err();
    assert!(err.contains("unknown key in hints: 'zzz'"), "{err}");
}

#[test]
fn to_side_right_prohibited() {
    let mut raw = base();
    raw["hints"] = json!({"ports": [{"edge_id": "e1", "to_side": "right"}]});
    let err = parse_and_validate(&raw).unwrap_err();
    assert_eq!(
        err,
        "hint for edge e1: to_side 'right' is prohibited; inbound ports use left, top, or bottom"
    );
}
