use std::collections::HashMap;

use crate::model::{C4Document, Node};

fn build_relations_suffix(
    node_uid: &str,
    relations: &[crate::model::Relation],
    nodes: &HashMap<&str, &Node>,
) -> String {
    let mut rel_groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for rel in relations {
        if rel.start == node_uid && rel.is != "contains" && rel.is != "handles" {
            let target_name = nodes
                .get(rel.end.as_str())
                .map(|n| n.name.as_str())
                .unwrap_or(rel.end.as_str());
            rel_groups.entry(rel.is.as_str()).or_default().push(target_name);
        }
    }

    let mut rels_str = String::new();
    if !rel_groups.is_empty() {
        let mut keys: Vec<&&str> = rel_groups.keys().collect();
        keys.sort();
        for key in keys {
            let mut targets = rel_groups.get(*key).unwrap().clone();
            targets.sort();
            rels_str.push_str(&format!(" [{}: {}]", key, targets.join(", ")));
        }
    }
    rels_str
}

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

    let mut visited = std::collections::HashSet::new();
    let mut out = String::new();
    for root in &roots {
        if !visited.insert(root.uid.as_str()) {
            continue;
        }
        let rels_str = build_relations_suffix(root.uid.as_str(), &doc.relations, &nodes);
        out.push_str(&format!("{}: {}{}\n", display_type(&root.node_type), root.name, rels_str));
        let empty = vec![];
        let children = children_map.get(root.uid.as_str()).unwrap_or(&empty);
        for (i, child_id) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            if let Some(child) = nodes.get(child_id) {
                render_node(child, &nodes, &children_map, &handles_map, &uses_map, &doc.relations, "", is_last, &mut out, &mut visited);
            }
        }
    }
    out.trim_end_matches('\n').to_string()
}

fn render_node<'a>(
    node: &'a Node,
    nodes: &HashMap<&'a str, &'a Node>,
    children_map: &HashMap<&'a str, Vec<&'a str>>,
    handles_map: &HashMap<&'a str, Vec<&'a str>>,
    uses_map: &HashMap<&'a str, Vec<&'a str>>,
    relations: &'a [crate::model::Relation],
    prefix: &str,
    is_last: bool,
    out: &mut String,
    visited: &mut std::collections::HashSet<&'a str>,
) {
    if !visited.insert(node.uid.as_str()) {
        return;
    }
    let connector = if is_last { "└─ " } else { "├─ " };
    let continuation = if is_last { "    " } else { "│   " };
    let new_prefix = format!("{}{}", prefix, continuation);

    let rels_str = build_relations_suffix(node.uid.as_str(), relations, nodes);
    out.push_str(&format!(
        "{}{}{}: {}{}\n",
        prefix,
        connector,
        display_type(&node.node_type),
        node.name,
        rels_str
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
            render_node(child, nodes, children_map, handles_map, uses_map, relations, &new_prefix, child_is_last, out, visited);
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
