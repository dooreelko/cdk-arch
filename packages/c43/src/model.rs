use std::collections::HashSet;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NodeAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    /// cdk-arch base kind of a construct node (`architecture`, `apicontainer`, `function`,
    /// `construct`); the node `type` is the concrete class name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

impl Node {
    pub fn is_function(&self) -> bool {
        match self.attributes.kind.as_deref() {
            Some(kind) => kind == "function",
            None => self.node_type == "function" || self.node_type == "tbdfunction",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub uid: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub attributes: NodeAttributes,
}

#[derive(Debug, Clone, Serialize)]
pub struct Relation {
    pub start: String,
    pub is: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct C4Document {
    pub nodes: Vec<Node>,
    pub relations: Vec<Relation>,
    #[serde(skip)]
    seen_nodes: HashSet<String>,
    #[serde(skip)]
    seen_relations: HashSet<(String, String, String)>,
}

/// Uid of a Backend node (an `Architecture` construct). Prefixed by type, like `system:<repo>`,
/// so an Architecture never shares a uid with one of its own containers of the same id.
pub fn backend_uid(arch_id: &str) -> String {
    format!("backend:{}", arch_id)
}

/// Uid of a construct owned by an Architecture (container, function): `<arch id>/<id>`.
/// Construct ids are only unique within their Architecture.
pub fn child_uid(arch_id: &str, id: &str) -> String {
    format!("{}/{}", arch_id, id)
}

impl C4Document {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relations: Vec::new(),
            seen_nodes: HashSet::new(),
            seen_relations: HashSet::new(),
        }
    }

    pub fn add_node(&mut self, uid: &str, name: &str, node_type: &str, attributes: NodeAttributes) {
        if self.seen_nodes.insert(uid.to_string()) {
            self.nodes.push(Node {
                uid: uid.to_string(),
                name: name.to_string(),
                node_type: node_type.to_lowercase(),
                attributes,
            });
        }
    }

    pub fn add_relation(&mut self, start: &str, is: &str, end: &str) {
        let key = (start.to_string(), is.to_string(), end.to_string());
        if self.seen_relations.insert(key) {
            self.relations.push(Relation {
                start: start.to_string(),
                is: is.to_string(),
                end: end.to_string(),
            });
        }
    }


}
