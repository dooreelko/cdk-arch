use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NodeAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
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
}

impl C4Document {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relations: Vec::new(),
        }
    }

    pub fn add_node(&mut self, uid: &str, name: &str, node_type: &str, attributes: NodeAttributes) {
        self.nodes.push(Node {
            uid: uid.to_string(),
            name: name.to_string(),
            node_type: node_type.to_lowercase(),
            attributes,
        });
    }

    pub fn add_relation(&mut self, start: &str, is: &str, end: &str) {
        self.relations.push(Relation {
            start: start.to_string(),
            is: is.to_string(),
            end: end.to_string(),
        });
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
