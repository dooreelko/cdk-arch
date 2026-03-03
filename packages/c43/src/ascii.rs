use std::collections::HashMap;

use crate::model::{C4Document, Node};

pub fn render(doc: &C4Document) -> String {
    let nodes: HashMap<&str, &Node> =
        doc.nodes.iter().map(|n| (n.uid.as_str(), n)).collect();

    // Build children map for "contains" relations, excluding function/tbdfunction nodes
    let mut children_map: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut contained: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for rel in &doc.relations {
        if rel.is == "contains" {
            let child_type = nodes
                .get(rel.end.as_str())
                .map(|n| n.node_type.as_str())
                .unwrap_or("");
            if child_type != "function" && child_type != "tbdfunction" {
                children_map
                    .entry(rel.start.as_str())
                    .or_default()
                    .push(rel.end.as_str());
                contained.insert(rel.end.as_str());
            }
        }
    }

    // Build handles map: container_id → Vec<function_id>
    let mut handles_map: HashMap<&str, Vec<&str>> = HashMap::new();
    for rel in &doc.relations {
        if rel.is == "handles" {
            handles_map
                .entry(rel.start.as_str())
                .or_default()
                .push(rel.end.as_str());
        }
    }

    // Build uses map: function_id → Vec<container_id>
    let mut uses_map: HashMap<&str, Vec<&str>> = HashMap::new();
    for rel in &doc.relations {
        if rel.is == "uses" {
            uses_map
                .entry(rel.start.as_str())
                .or_default()
                .push(rel.end.as_str());
        }
    }

    // Root nodes: not contained by anything and not functions
    let roots: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| {
            !contained.contains(n.uid.as_str())
                && n.node_type != "function"
                && n.node_type != "tbdfunction"
        })
        .collect();

    let mut out = String::new();
    for root in &roots {
        out.push_str(&format!("{}: {}\n", display_type(&root.node_type), root.name));
        let empty = vec![];
        let children = children_map.get(root.uid.as_str()).unwrap_or(&empty);
        for (i, child_id) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            if let Some(child) = nodes.get(child_id) {
                render_node(child, &nodes, &children_map, &handles_map, &uses_map, "", is_last, &mut out);
            }
        }
    }
    out.trim_end_matches('\n').to_string()
}

fn render_node(
    node: &Node,
    nodes: &HashMap<&str, &Node>,
    children_map: &HashMap<&str, Vec<&str>>,
    handles_map: &HashMap<&str, Vec<&str>>,
    uses_map: &HashMap<&str, Vec<&str>>,
    prefix: &str,
    is_last: bool,
    out: &mut String,
) {
    let connector = if is_last { "└─ " } else { "├─ " };
    let continuation = if is_last { "    " } else { "│   " };
    let new_prefix = format!("{}{}", prefix, continuation);

    out.push_str(&format!(
        "{}{}{}: {}\n",
        prefix,
        connector,
        display_type(&node.node_type),
        node.name
    ));

    // Print handles sub-items, aligning the uses column
    if let Some(handled) = handles_map.get(node.uid.as_str()) {
        let max_fn_len = handled.iter().map(|id| id.len()).max().unwrap_or(0);
        for fn_id in handled {
            let uses_str = match uses_map.get(fn_id) {
                Some(used) if !used.is_empty() => format!("  uses ──▶ {}", used.join(", ")),
                _ => String::new(),
            };
            out.push_str(&format!(
                "{}handles ──▶ {:width$}{}\n",
                new_prefix,
                fn_id,
                uses_str,
                width = max_fn_len,
            ));
        }
    }

    // Recurse into container children
    let empty = vec![];
    let children = children_map.get(node.uid.as_str()).unwrap_or(&empty);
    for (i, child_id) in children.iter().enumerate() {
        let child_is_last = i == children.len() - 1;
        if let Some(child) = nodes.get(child_id) {
            render_node(child, nodes, children_map, handles_map, uses_map, &new_prefix, child_is_last, out);
        }
    }
}

fn display_type(t: &str) -> &str {
    match t {
        "system" => "System",
        "backend" => "Backend",
        "frontend" => "Frontend",
        "client" => "Client",
        "apicontainer" => "ApiContainer",
        "wscontainer" => "WsContainer",
        "datastore" => "DataStore",
        "function" => "Function",
        "tbdfunction" => "TBDFunction",
        other => other,
    }
}
