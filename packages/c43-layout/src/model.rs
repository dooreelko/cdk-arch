use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Data,
    Nf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Right,
    Bottom,
    Top,
    Left,
}

pub const SIDES: [Side; 4] = [Side::Right, Side::Bottom, Side::Top, Side::Left];

// ---- public input ----

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputGroup {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputEdge {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// a compound graph: containment is a tree (groups), edges cross it and may end on groups
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputGraph {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub groups: Vec<InputGroup>,
    pub nodes: Vec<InputNode>,
    #[serde(default)]
    pub edges: Vec<InputEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KindHint {
    pub from: String,
    pub to: String,
    pub kind: Kind,
}

fn one() -> f64 {
    1.0
}

/// sizes the engine cannot know without measuring text, in grid units
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeHints {
    /// smallest node side per node (all nodes take the largest)
    #[serde(default)]
    pub node: IndexMap<String, f64>,
    /// width of a group's one-line title
    #[serde(default)]
    pub group_title: IndexMap<String, f64>,
    /// height of a group title line
    #[serde(default = "one")]
    pub title_height: f64,
}

impl Default for SizeHints {
    fn default() -> Self {
        SizeHints { node: IndexMap::new(), group_title: IndexMap::new(), title_height: 1.0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Hints {
    /// edge kinds; edges without a hint are data
    #[serde(default)]
    pub kinds: Vec<KindHint>,
    /// relative placement suggestions (not supported yet)
    #[serde(default)]
    pub placement: Vec<serde_json::Value>,
    #[serde(default)]
    pub sizes: SizeHints,
}

// ---- internal input (node kinds given), the router's InputGraph ----

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawNode {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawGraph {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub groups: Vec<InputGroup>,
    pub nodes: Vec<RawNode>,
    #[serde(default)]
    pub edges: Vec<RawEdge>,
}

// ---- normalized graph ----

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub parent: Option<String>,
}

/// `id` equals the edge's index in `Graph.edges`
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: usize,
    pub from: String,
    pub to: String,
    pub kind: Kind,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub title: Option<String>,
    pub description: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub groups: Vec<Group>,
    pub group_ids: IndexSet<String>,
    /// member (node or group) → its parent group; top-level ids are absent
    pub parent: IndexMap<String, String>,
    /// nodes touching at least one data edge; all others are secondary
    pub data_nodes: IndexSet<String>,
    pub degree: IndexMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub col: i32,
    pub row: i32,
}

/// inclusive cell span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Box {
    pub col0: i32,
    pub row0: i32,
    pub col1: i32,
    pub row1: i32,
}

/// `groups`: each group's block of cells (absent for graphs without groups)
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub cols: i32,
    pub rows: i32,
    pub cells: IndexMap<String, Cell>,
    pub groups: Option<IndexMap<String, Box>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    Src,
    Dst,
}

/// `inner`: the start of a group → own member edge, inside the group's left border
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortRef {
    pub edge: usize,
    pub end: End,
    pub inner: bool,
}

/// per side, ordered: top→bottom on left/right, left→right on top/bottom
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SidePorts {
    pub right: Vec<PortRef>,
    pub bottom: Vec<PortRef>,
    pub top: Vec<PortRef>,
    pub left: Vec<PortRef>,
}

impl SidePorts {
    pub fn get(&self, s: Side) -> &Vec<PortRef> {
        match s {
            Side::Right => &self.right,
            Side::Bottom => &self.bottom,
            Side::Top => &self.top,
            Side::Left => &self.left,
        }
    }
    pub fn get_mut(&mut self, s: Side) -> &mut Vec<PortRef> {
        match s {
            Side::Right => &mut self.right,
            Side::Bottom => &mut self.bottom,
            Side::Top => &mut self.top,
            Side::Left => &mut self.left,
        }
    }
}

pub type Ports = IndexMap<String, SidePorts>;

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub placement: Placement,
    pub ports: Ports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    V,
    H,
}

/// vertical lane `i` runs left of column i; horizontal lane `i` runs above row i
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lane {
    pub dir: Dir,
    pub i: i32,
}

/// lanes travelled in order; [] = direct straight shot between facing ports
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub edge: usize,
    pub lanes: Vec<Lane>,
}

// ---- output ----

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortOut {
    pub side: Side,
    pub index: usize,
    pub count: usize,
    #[serde(skip_serializing_if = "is_false")]
    pub inner: bool,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub col: i32,
    pub row: i32,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutEdge {
    pub id: usize,
    pub from: String,
    pub to: String,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub src: PortOut,
    pub dst: PortOut,
    pub points: Vec<Pt>,
}

/// a text box: the renderer draws `text` inside it
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextBlock {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// a group's border rectangle (in the lanes around its block) with its one-line title just inside the top border
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutGroup {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub depth: usize,
    #[serde(rename = "box")]
    pub bx: Box,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub title: TextBlock,
}

impl LayoutGroup {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.w, h: self.h }
    }
}

/// lane widths, outer lanes included: v[i] runs left of column i, h[i] above row i
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lanes {
    pub v: Vec<f64>,
    pub h: Vec<f64>,
}

/// all coordinates in grid units
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Layout {
    #[serde(rename = "S")]
    pub s: f64,
    pub cols: i32,
    pub rows: i32,
    pub lanes: Lanes,
    /// outer groups first
    pub groups: Vec<LayoutGroup>,
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    /// encloses the grid, including the outer lanes
    pub frame: Rect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Violation {
    pub rule: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Soft {
    pub leftward: f64,
    pub centrality: f64,
    pub area: f64,
    pub aspect: f64,
    pub nf_bottom_share: f64,
    pub length: f64,
    pub turns: f64,
    /// sibling groups touching across a lane with unequal touching sides
    pub group_sides: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    pub violations: Vec<Violation>,
    pub crossings: usize,
    pub soft: Soft,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutResult {
    pub version: u32,
    pub layout: Layout,
    pub metrics: Metrics,
}
