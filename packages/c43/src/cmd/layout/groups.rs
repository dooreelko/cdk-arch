//! Group frames: build + validate from raw JSON, resolve grid extents,
//! lay out lane rings, and expose border cells for routing/rendering.

#![allow(dead_code)] // functions are wired in over the next tasks

use super::model::{Group, Node};
use serde_json::Value;
use std::collections::HashSet;

/// `str()`-style rendering for ids (always strings in practice).
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

/// Build groups from the raw `groups` array (missing/null -> empty). Validates
/// structure only (ids, parents, members); extents/depth are resolved later.
pub fn build_groups(raw: &Value, nodes: &[Node]) -> Result<Vec<Group>, String> {
    let arr = match raw.get("groups") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(a)) => a.clone(),
        Some(other) => {
            return Err(format!("groups must be an array, got {}", other));
        }
    };

    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut seen_group_ids: HashSet<String> = HashSet::new();
    let mut groups: Vec<Group> = Vec::new();

    // First pass: read entries, validate ids/members. Parent existence checked
    // in a second pass once all group ids are known.
    for (i, gr) in arr.iter().enumerate() {
        let id_v = gr
            .get("id")
            .ok_or_else(|| format!("group at index {i} missing required field: id"))?;
        let gid = py_str(id_v);
        if !seen_group_ids.insert(gid.clone()) {
            return Err(format!("duplicate group id: {gid}"));
        }

        let title = gr
            .get("title")
            .map(py_str)
            .ok_or_else(|| format!("group {gid} missing required field: title"))?;

        let members_v = gr
            .get("members")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut member_ids = Vec::new();
        for mv in &members_v {
            let mid = py_str(mv);
            if !node_ids.contains(mid.as_str()) {
                return Err(format!("group {gid} references unknown member node id: {mid}"));
            }
            member_ids.push(mid);
        }

        let parent = match gr.get("parent") {
            None | Some(Value::Null) => None,
            Some(p) => Some(py_str(p)),
        };

        groups.push(Group {
            id: gid,
            title,
            parent,
            member_ids,
            depth: 0,
            col0: 0, col1: 0, row0: 0, row1: 0,
            x: 0, y: 0, w: 0, h: 0,
        });
    }

    // Second pass: every parent must name an existing group.
    for g in &groups {
        if let Some(p) = &g.parent {
            if !seen_group_ids.contains(p) {
                return Err(format!("group {} references unknown parent group id: {p}", g.id));
            }
        }
    }

    Ok(groups)
}
