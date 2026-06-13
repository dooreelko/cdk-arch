//! Group frames: build + validate from raw JSON, resolve grid extents,
//! lay out lane rings, and expose border cells for routing/rendering.

#![allow(dead_code)] // functions are wired in over the next tasks

use super::model::{Group, Node};
use serde_json::Value;
use std::collections::HashMap;
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

/// Resolve grid extents (members ∪ descendant extents) and depth for every
/// group, in place. Returns an error on a parent cycle.
pub fn resolve_extents(groups: &mut [Group], nodes: &[Node]) -> Result<(), String> {
    let node_cell: HashMap<&str, (i64, i64)> = nodes
        .iter()
        .map(|n| (n.id.as_str(), (n.grid_col, n.grid_row)))
        .collect();

    let id_index: HashMap<String, usize> =
        groups.iter().enumerate().map(|(i, g)| (g.id.clone(), i)).collect();

    // children adjacency
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, g) in groups.iter().enumerate() {
        if let Some(p) = &g.parent {
            let pi = id_index[p];
            children.entry(pi).or_default().push(i);
        }
    }

    // depth: walk parent chain; detect cycle with a bounded step count.
    let n = groups.len();
    for i in 0..n {
        let mut depth = 0i64;
        let mut cur = groups[i].parent.clone();
        let mut steps = 0;
        while let Some(p) = cur {
            steps += 1;
            if steps > n as i64 {
                return Err(format!("cycle detected in group parent chain at {}", groups[i].id));
            }
            depth += 1;
            cur = groups[id_index[&p]].parent.clone();
        }
        groups[i].depth = depth;
    }

    // extents: post-order over the parent tree (deepest first). Sorting indices
    // by descending depth guarantees children are computed before parents.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| -groups[i].depth);

    // seed every group from its direct members
    let mut ext: Vec<Option<(i64, i64, i64, i64)>> = vec![None; n];
    for i in 0..n {
        for mid in &groups[i].member_ids {
            let (c, r) = node_cell[mid.as_str()];
            ext[i] = Some(match ext[i] {
                None => (c, c, r, r),
                Some((c0, c1, r0, r1)) => (c0.min(c), c1.max(c), r0.min(r), r1.max(r)),
            });
        }
    }
    // fold child extents up into parents (deepest first)
    for &i in &order {
        if let Some(kids) = children.get(&i) {
            for &k in kids {
                if let Some((kc0, kc1, kr0, kr1)) = ext[k] {
                    ext[i] = Some(match ext[i] {
                        None => (kc0, kc1, kr0, kr1),
                        Some((c0, c1, r0, r1)) =>
                            (c0.min(kc0), c1.max(kc1), r0.min(kr0), r1.max(kr1)),
                    });
                }
            }
        }
    }

    for i in 0..n {
        let (c0, c1, r0, r1) = ext[i].ok_or_else(|| {
            format!("group {} has no members and no child groups", groups[i].id)
        })?;
        groups[i].col0 = c0;
        groups[i].col1 = c1;
        groups[i].row0 = r0;
        groups[i].row1 = r1;
    }

    Ok(())
}
