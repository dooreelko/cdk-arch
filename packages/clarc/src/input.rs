//! The clarc input document: a compound graph of cloud services plus placement hints per node.
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: Option<String>,
    /// frame kind: network, region, cloud, group (default)
    pub service: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub label: Option<String>,
    pub service: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Data,
    Nf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineStyle {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    /// overrides the kind derived from the services
    pub kind: Option<EdgeKind>,
    /// overrides the line derived from the kind (nf → dashed)
    pub style: Option<LineStyle>,
}

/// placement suggestions about one node; the values are other ids
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeHints {
    /// the left-most node; the one hint that wins over the rest
    #[serde(default)]
    pub start: bool,
    pub left_of: Option<String>,
    pub right_of: Option<String>,
    pub above: Option<String>,
    pub below: Option<String>,
    pub same_row: Option<String>,
    pub same_col: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    /// per node id
    #[serde(default)]
    pub hints: IndexMap<String, NodeHints>,
}
