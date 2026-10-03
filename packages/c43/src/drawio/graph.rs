//! C4 document → layout graph and hints.
use super::text::{label_side, text_width, LABEL_PX, LINE_H, UNIT_PX};
use crate::model::{C4Document, NodeAttributes};
use c43_layout::model::{Hints, InputEdge, InputGraph, InputGroup, InputNode, SizeHints};
use indexmap::{IndexMap, IndexSet};

pub struct Built {
    pub graph: InputGraph,
    pub hints: Hints,
    /// relations that could not be drawn as given, and what was done about them
    pub warnings: Vec<String>,
}

/// a C4 document from its JSON form (`{nodes:[{uid,name,type,attributes}], relations:[{start,is,end}]}`)
pub fn doc_from_json(v: &serde_json::Value) -> C4Document {
    let s = |x: &serde_json::Value, k: &str| x[k].as_str().unwrap_or_default().to_string();
    let mut doc = C4Document::new();
    for n in v["nodes"].as_array().into_iter().flatten() {
        let a = &n["attributes"];
        let opt = |k: &str| a[k].as_str().map(String::from);
        let attributes = NodeAttributes { project: opt("project"), file: opt("file"), variable: opt("variable"), kind: opt("kind") };
        doc.add_node(&s(n, "uid"), &s(n, "name"), &s(n, "type"), attributes);
    }
    for r in v["relations"].as_array().into_iter().flatten() {
        doc.add_relation(&s(r, "start"), &s(r, "is"), &s(r, "end"));
    }
    doc
}

/// the system node becomes the title, `contains` becomes group membership (whatever contains something is a group),
/// any other relation an edge labelled with its kind (labels shown only when there is more than one kind)
pub fn build(doc: &C4Document) -> Built {
    let mut warnings: Vec<String> = Vec::new();
    let systems: IndexSet<&str> = doc.nodes.iter().filter(|n| n.node_type == "system").map(|n| n.uid.as_str()).collect();
    let title = doc.nodes.iter().find(|n| n.node_type == "system").map(|n| n.name.clone());
    let known: IndexSet<&str> = doc.nodes.iter().map(|n| n.uid.as_str()).collect();
    let contains: Vec<_> = doc.relations.iter().filter(|r| r.is == "contains" && !systems.contains(r.start.as_str())).collect();
    let group_ids: IndexSet<&str> = contains.iter().map(|r| r.start.as_str()).collect();
    let mut parent: IndexMap<&str, &str> = IndexMap::new();
    for r in &contains {
        if !known.contains(r.start.as_str()) || !known.contains(r.end.as_str()) {
            let missing = if known.contains(r.start.as_str()) { &r.end } else { &r.start };
            warnings.push(format!("dropped {} contains {}: unknown node {missing}", r.start, r.end));
            continue;
        }
        let mut up = Some(r.start.as_str());
        let cycle = std::iter::from_fn(|| {
            let cur = up?;
            up = parent.get(cur).copied();
            Some(cur)
        })
        .any(|x| x == r.end);
        if cycle {
            warnings.push(format!("dropped {} contains {}: containment cycle", r.start, r.end));
            continue;
        }
        match parent.get(r.end.as_str()) {
            Some(p) => warnings.push(format!("{} contained by both {p} and {}, keeping {p}", r.end, r.start)),
            None => {
                parent.insert(&r.end, &r.start);
            }
        }
    }
    let group_of = |uid: &str| parent.get(uid).map(|p| p.to_string());
    let groups: Vec<InputGroup> = doc
        .nodes
        .iter()
        .filter(|n| group_ids.contains(n.uid.as_str()))
        .map(|n| InputGroup { id: n.uid.clone(), label: Some(n.name.clone()), group: group_of(&n.uid) })
        .collect();
    let nodes: Vec<InputNode> = doc
        .nodes
        .iter()
        .filter(|n| !systems.contains(n.uid.as_str()) && !group_ids.contains(n.uid.as_str()))
        .map(|n| InputNode { id: n.uid.clone(), label: Some(n.name.clone()), group: group_of(&n.uid) })
        .collect();
    let ancestor = |id: &str, of: &str| {
        let mut c = parent.get(id);
        while let Some(p) = c {
            if *p == of {
                return true;
            }
            c = parent.get(p);
        }
        false
    };
    let mut kinds: IndexMap<(String, String), Vec<String>> = IndexMap::new();
    for r in doc.relations.iter().filter(|r| r.is != "contains") {
        if systems.contains(r.start.as_str()) || systems.contains(r.end.as_str()) {
            continue;
        }
        let why = if let Some(missing) = [&r.start, &r.end].into_iter().find(|id| !known.contains(id.as_str())) {
            Some(format!("unknown node {missing}"))
        } else if r.start == r.end {
            Some("self-loop".to_string())
        } else if ancestor(&r.start, &r.end) {
            Some("a member cannot point at its own group".to_string())
        } else {
            None
        };
        if let Some(why) = why {
            warnings.push(format!("dropped {} {} {}: {why}", r.start, r.is, r.end));
            continue;
        }
        let is = kinds.entry((r.start.clone(), r.end.clone())).or_default();
        if !is.contains(&r.is) {
            is.push(r.is.clone());
        }
    }
    let labelled = kinds.values().flatten().collect::<IndexSet<_>>().len() > 1;
    let edges: Vec<InputEdge> = kinds
        .into_iter()
        .map(|((from, to), is)| InputEdge { from, to, label: labelled.then(|| is.join(", ")) })
        .collect();
    let sizes = SizeHints {
        node: nodes.iter().map(|n| (n.id.clone(), label_side(&[n.label.as_deref().unwrap_or(&n.id)], UNIT_PX))).collect(),
        group_title: groups
            .iter()
            .map(|g| (g.id.clone(), (text_width(g.label.as_deref().unwrap_or(&g.id), LABEL_PX, true) + LABEL_PX / 2.0) / UNIT_PX))
            .collect(),
        title_height: (LABEL_PX * LINE_H / UNIT_PX).ceil(),
    };
    Built {
        graph: InputGraph { title, description: None, groups, nodes, edges },
        hints: Hints { kinds: vec![], placement: vec![], sizes },
        warnings,
    }
}
