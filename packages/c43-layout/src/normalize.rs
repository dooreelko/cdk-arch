use crate::model::*;
use indexmap::{IndexMap, IndexSet};

fn normalize_groups(input: &RawGraph) -> Result<(Vec<Group>, IndexSet<String>, IndexMap<String, String>), String> {
    let mut groups: Vec<Group> = Vec::new();
    let mut group_ids: IndexSet<String> = IndexSet::new();
    for gr in &input.groups {
        if gr.id.is_empty() {
            return Err(format!("group id must be a string: {}", gr.id));
        }
        if group_ids.contains(&gr.id) {
            return Err(format!("duplicate id: {}", gr.id));
        }
        group_ids.insert(gr.id.clone());
        groups.push(Group { id: gr.id.clone(), label: gr.label.clone().unwrap_or_else(|| gr.id.clone()), parent: gr.group.clone() });
    }
    let mut parent: IndexMap<String, String> = IndexMap::new();
    for gr in &groups {
        let Some(p) = &gr.parent else { continue };
        if !group_ids.contains(p) {
            return Err(format!("group {}: unknown group {}", gr.id, p));
        }
        parent.insert(gr.id.clone(), p.clone());
    }
    for gr in &groups {
        let mut seen: IndexSet<&str> = IndexSet::new();
        let mut c = Some(gr.id.as_str());
        while let Some(id) = c {
            if !seen.insert(id) {
                return Err(format!("group cycle: {}", gr.id));
            }
            c = parent.get(id).map(String::as_str);
        }
    }
    Ok((groups, group_ids, parent))
}

/// the graph with node kinds given (as the engine builds it for group levels)
pub fn normalize_internal(input: &RawGraph) -> Result<Graph, String> {
    if input.nodes.is_empty() {
        return Err("empty graph: at least one node required".into());
    }
    let (groups, group_ids, mut parent) = normalize_groups(input)?;
    let mut nodes: Vec<Node> = Vec::new();
    let mut kind_of: IndexMap<String, Kind> = IndexMap::new();
    for n in &input.nodes {
        if n.id.is_empty() {
            return Err(format!("node id must be a string: {}", n.id));
        }
        if group_ids.contains(&n.id) {
            return Err(format!("duplicate id: {}", n.id));
        }
        if kind_of.contains_key(&n.id) {
            return Err(format!("duplicate node id: {}", n.id));
        }
        if let Some(gr) = &n.group {
            if !group_ids.contains(gr) {
                return Err(format!("node {}: unknown group {}", n.id, gr));
            }
        }
        let kind = n.kind.unwrap_or(Kind::Data);
        kind_of.insert(n.id.clone(), kind);
        if let Some(gr) = &n.group {
            parent.insert(n.id.clone(), gr.clone());
        }
        nodes.push(Node { id: n.id.clone(), label: n.label.clone().unwrap_or_else(|| n.id.clone()), kind, parent: n.group.clone() });
    }
    let mut edges: Vec<Edge> = Vec::new();
    let mut pairs: IndexSet<String> = IndexSet::new();
    for e in &input.edges {
        for id in [&e.from, &e.to] {
            if !kind_of.contains_key(id) && !group_ids.contains(id) {
                return Err(format!("edge references unknown node: {id}"));
            }
        }
        if e.from == e.to {
            return Err(format!("self-loop not supported: {}", e.from));
        }
        let mut c = parent.get(&e.from);
        while let Some(p) = c {
            if *p == e.to {
                return Err(format!("edge {}->{}: a member cannot point at its own group", e.from, e.to));
            }
            c = parent.get(p);
        }
        let key = format!("{}->{}", e.from, e.to);
        if pairs.contains(&key) {
            return Err(format!("duplicate edge: {key}"));
        }
        pairs.insert(key);
        let nf = |id: &String| kind_of.get(id) == Some(&Kind::Nf);
        let kind = e.kind.unwrap_or(if nf(&e.from) || nf(&e.to) { Kind::Nf } else { Kind::Data });
        edges.push(Edge { id: edges.len(), from: e.from.clone(), to: e.to.clone(), kind, label: e.label.clone() });
    }
    let touched: IndexSet<&String> = edges.iter().filter(|e| e.kind == Kind::Data).flat_map(|e| [&e.from, &e.to]).collect();
    let data_nodes: IndexSet<String> = nodes.iter().map(|n| n.id.clone()).filter(|id| touched.contains(id)).collect();
    let mut degree: IndexMap<String, usize> = nodes.iter().map(|n| (n.id.clone(), 0)).collect();
    for e in &edges {
        for id in [&e.from, &e.to] {
            if let Some(d) = degree.get_mut(id) {
                *d += 1;
            }
        }
    }
    Ok(Graph {
        title: input.title.clone(),
        description: input.description.clone(),
        nodes,
        edges,
        groups,
        group_ids,
        parent,
        data_nodes,
        degree,
    })
}

/// public input: edge kinds from hints (default data); a node is nf when it has edges and none of them is data
pub fn normalize(input: &InputGraph, hints: &Hints) -> Result<Graph, String> {
    if !hints.placement.is_empty() {
        return Err("placement hints not supported yet".into());
    }
    let is_node = |id: &String| input.nodes.iter().any(|n| &n.id == id);
    let is_group = |id: &String| input.groups.iter().any(|g| &g.id == id);
    if let Some(id) = hints.sizes.node.keys().find(|id| !is_node(id)) {
        return Err(format!("hint references unknown node: {id}"));
    }
    if let Some(id) = hints.sizes.group_title.keys().find(|id| !is_group(id)) {
        return Err(format!("hint references unknown group: {id}"));
    }
    if let Some(k) = hints.kinds.iter().find(|k| !input.edges.iter().any(|e| e.from == k.from && e.to == k.to)) {
        return Err(format!("hint references unknown edge: {}->{}", k.from, k.to));
    }
    let kind_of = |e: &InputEdge| {
        hints.kinds.iter().rev().find(|k| k.from == e.from && k.to == e.to).map_or(Kind::Data, |k| k.kind)
    };
    let edges: Vec<RawEdge> = input
        .edges
        .iter()
        .map(|e| RawEdge { from: e.from.clone(), to: e.to.clone(), kind: Some(kind_of(e)), label: e.label.clone() })
        .collect();
    let node_kind = |id: &String| {
        let mine: Vec<Kind> = edges.iter().filter(|e| &e.from == id || &e.to == id).filter_map(|e| e.kind).collect();
        if !mine.is_empty() && mine.iter().all(|k| *k == Kind::Nf) { Kind::Nf } else { Kind::Data }
    };
    let raw = RawGraph {
        title: input.title.clone(),
        description: input.description.clone(),
        groups: input.groups.clone(),
        nodes: input
            .nodes
            .iter()
            .map(|n| RawNode { id: n.id.clone(), label: n.label.clone(), kind: Some(node_kind(&n.id)), group: n.group.clone() })
            .collect(),
        edges,
    };
    normalize_internal(&raw)
}
