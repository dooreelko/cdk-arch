//! Parse and validate

use super::model::{Edge, HintPort, Model, Node, EDGE_ALPHABET, SIDES};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashSet;

/// Format a JSON value the way Python's `{!r}` would.
fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::String(s) => format!("'{}'", s),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                n.to_string()
            } else if let Some(f) = n.as_f64() {
                // Python float repr; serde_json already renders floats compactly.
                let s = f.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    s
                } else {
                    format!("{}.0", s)
                }
            } else {
                n.to_string()
            }
        }
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

/// Format a JSON value used as an id the way Python's `str()` would for the
/// f-strings that interpolate it bare (no `!r`). These are always strings in
/// practice, so emit the raw string; fall back to a JSON rendering otherwise.
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

/// True iff the value is a JSON integer that is not a bool. Mirrors Python's
/// `isinstance(x, int) and not isinstance(x, bool)`. In serde a bool is never
/// `Value::Number`, so the only thing to exclude is non-integer numbers.
fn as_grid_int(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

pub fn parse_and_validate(raw: &Value) -> Result<Model, String> {
    let get = |k: &str| raw.get(k);

    if get("title").is_none() {
        return Err("missing required key: title".to_string());
    }
    if get("description").is_none() {
        return Err("missing required key: description".to_string());
    }
    if get("nodes").is_none() {
        return Err("missing required key: nodes".to_string());
    }
    if get("edges").is_none() {
        return Err("missing required key: edges".to_string());
    }

    let nodes_raw = raw["nodes"].as_array().cloned().unwrap_or_default();
    let edges_raw = raw["edges"].as_array().cloned().unwrap_or_default();

    if nodes_raw.is_empty() {
        return Err("nodes must not be empty".to_string());
    }

    let mut seen_node_ids: HashSet<String> = HashSet::new();
    let mut cells: std::collections::HashMap<(i64, i64), String> = std::collections::HashMap::new();
    let mut nodes: Vec<Node> = Vec::new();

    for (i, nr) in nodes_raw.iter().enumerate() {
        let id_v = nr.get("id");
        if id_v.is_none() {
            return Err(format!("node at index {i} missing required field: id"));
        }
        let nid_v = id_v.unwrap();
        let nid_repr = py_repr(nid_v);

        if nr.get("label").is_none() {
            return Err(format!(
                "node at index {i} (id={nid_repr}) missing required field: label"
            ));
        }
        if nr.get("grid_col").is_none() {
            return Err(format!(
                "node at index {i} (id={nid_repr}) missing required field: grid_col"
            ));
        }
        if nr.get("grid_row").is_none() {
            return Err(format!(
                "node at index {i} (id={nid_repr}) missing required field: grid_row"
            ));
        }

        let grid_col_v = &nr["grid_col"];
        let grid_row_v = &nr["grid_row"];

        let grid_col = match as_grid_int(grid_col_v) {
            Some(n) if n >= 0 => n,
            _ => {
                return Err(format!(
                    "node at index {i} (id={nid_repr}): grid_col must be an int >= 0, got {}",
                    py_repr(grid_col_v)
                ))
            }
        };
        let grid_row = match as_grid_int(grid_row_v) {
            Some(n) if n >= 0 => n,
            _ => {
                return Err(format!(
                    "node at index {i} (id={nid_repr}): grid_row must be an int >= 0, got {}",
                    py_repr(grid_row_v)
                ))
            }
        };

        let nid = py_str(nid_v);
        if seen_node_ids.contains(&nid) {
            return Err(format!("duplicate node id: {nid}"));
        }
        seen_node_ids.insert(nid.clone());

        let cell = (grid_col, grid_row);
        if let Some(first) = cells.get(&cell) {
            return Err(format!(
                "two nodes in grid cell ({}, {}): {} and {}",
                grid_col, grid_row, first, nid
            ));
        }
        cells.insert(cell, nid.clone());

        let label = py_str(&nr["label"]);
        nodes.push(Node::new(nid, label, grid_col, grid_row));
    }

    if edges_raw.len() > EDGE_ALPHABET.chars().count() {
        return Err(format!(
            "{} edges exceeds the {}-char alphabet; unicode edge alphabet is the future expansion path",
            edges_raw.len(),
            EDGE_ALPHABET.chars().count()
        ));
    }

    let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let alphabet: Vec<char> = EDGE_ALPHABET.chars().collect();
    let mut seen_edge_ids: HashSet<String> = HashSet::new();
    let mut edges: Vec<Edge> = Vec::new();

    for (i, er) in edges_raw.iter().enumerate() {
        let id_v = er.get("id");
        if id_v.is_none() {
            return Err(format!("edge at index {i} missing required field: id"));
        }
        let eid_v = id_v.unwrap();
        let eid_repr = py_repr(eid_v);

        if er.get("from").is_none() {
            return Err(format!(
                "edge at index {i} (id={eid_repr}) missing required field: from"
            ));
        }
        if er.get("to").is_none() {
            return Err(format!(
                "edge at index {i} (id={eid_repr}) missing required field: to"
            ));
        }

        let eid = py_str(eid_v);
        if seen_edge_ids.contains(&eid) {
            return Err(format!("duplicate edge id: {eid}"));
        }
        seen_edge_ids.insert(eid.clone());

        for end in ["from", "to"] {
            let ref_id = py_str(&er[end]);
            if !node_ids.contains(&ref_id) {
                return Err(format!("edge {eid} references unknown node id: {ref_id}"));
            }
        }

        let from_id = py_str(&er["from"]);
        let to_id = py_str(&er["to"]);
        edges.push(Edge {
            id: eid,
            from_id,
            to_id,
            char: alphabet[i],
            from_port: None,
            to_port: None,
            route: None,
        });
    }

    // hints: missing/null -> empty
    let empty = Value::Object(serde_json::Map::new());
    let hints = match raw.get("hints") {
        Some(Value::Null) | None => &empty,
        Some(h) => h,
    };
    let hints_obj = hints.as_object().cloned().unwrap_or_default();

    for key in hints_obj.keys() {
        if key != "ports" && key != "routing_order" {
            return Err(format!(
                "unknown key in hints: {}, allowed keys are: ports, routing_order",
                py_repr(&Value::String(key.clone()))
            ));
        }
    }

    let mut hint_ports: IndexMap<String, HintPort> = IndexMap::new();
    let mut seen_hint_edge_ids: HashSet<String> = HashSet::new();

    let ports = hints_obj
        .get("ports")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    for (i, hp) in ports.iter().enumerate() {
        let edge_id_v = hp.get("edge_id");
        if edge_id_v.is_none() {
            return Err(format!(
                "hint port at index {i} missing required field: edge_id"
            ));
        }
        let hint_edge_id = py_str(edge_id_v.unwrap());

        if seen_hint_edge_ids.contains(&hint_edge_id) {
            return Err(format!("duplicate hint port for edge_id: {hint_edge_id}"));
        }
        seen_hint_edge_ids.insert(hint_edge_id.clone());

        if !seen_edge_ids.contains(&hint_edge_id) {
            return Err(format!("hint references unknown edge_id: {hint_edge_id}"));
        }

        for key in ["from_side", "to_side"] {
            if let Some(side) = hp.get(key) {
                let side_str = side.as_str();
                let valid = side_str.map(|s| SIDES.contains(&s)).unwrap_or(false);
                if !valid {
                    return Err(format!(
                        "hint for edge {hint_edge_id}: invalid side {}, must be one of ('left', 'right', 'top', 'bottom')",
                        py_repr(side)
                    ));
                }
            }
        }

        if hp.get("to_side").and_then(|v| v.as_str()) == Some("right") {
            return Err(format!(
                "hint for edge {hint_edge_id}: to_side 'right' is prohibited; inbound ports use left, top, or bottom"
            ));
        }

        let from_side = hp
            .get("from_side")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let to_side = hp
            .get("to_side")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        hint_ports.insert(hint_edge_id, HintPort { from_side, to_side });
    }

    let routing_order_raw = hints_obj
        .get("routing_order")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut routing_order: Vec<String> = Vec::new();
    let mut seen_routing_ids: HashSet<String> = HashSet::new();
    for eid_v in &routing_order_raw {
        let eid = py_str(eid_v);
        if seen_routing_ids.contains(&eid) {
            return Err(format!("duplicate edge_id in routing_order: {eid}"));
        }
        seen_routing_ids.insert(eid.clone());
        if !seen_edge_ids.contains(&eid) {
            return Err(format!("routing_order references unknown edge_id: {eid}"));
        }
        routing_order.push(eid);
    }

    let mut groups = super::groups::build_groups(raw, &nodes)?;
    super::groups::resolve_extents(&mut groups, &nodes)?;
    super::groups::validate_extents(&groups, &nodes)?;

    let title = py_str(&raw["title"]);
    let description = py_str(&raw["description"]);
    let mut model = Model::new(title, description, nodes, edges);
    model.hint_ports = hint_ports;
    model.routing_order = routing_order;
    model.groups = groups;
    Ok(model)
}
