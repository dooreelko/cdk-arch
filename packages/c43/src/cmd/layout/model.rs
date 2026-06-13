//! Data model

use indexmap::IndexMap;
use std::collections::BTreeMap;

// Constants
pub const EDGE_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const SIDES: [&str; 4] = ["left", "right", "top", "bottom"];

// Geometry constants
pub const GUTTER_W: i64 = 8;
pub const LABEL_PAD: i64 = 4;
pub const BOX_H: i64 = 11;
pub const BOX_MARGIN_X: i64 = 4;
pub const BOX_MARGIN_Y: i64 = 2;
pub const LANE_MIN_W: i64 = 16;
pub const LANE_MIN_H: i64 = 7;
pub const TITLE_H: i64 = 6;

// Group frame glyphs (double-line box drawing) and ring padding.
pub const GROUP_TL: char = '╔';
pub const GROUP_TR: char = '╗';
pub const GROUP_BL: char = '╚';
pub const GROUP_BR: char = '╝';
pub const GROUP_H: char = '═';
pub const GROUP_V: char = '║';
/// One blank cell between adjacent group rings and between rings and the edge zone.
pub const GROUP_PAD: i64 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub grid_col: i64,
    pub grid_row: i64,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Node {
    pub fn new(id: String, label: String, grid_col: i64, grid_row: i64) -> Self {
        Self {
            id,
            label,
            grid_col,
            grid_row,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub side: String,
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub char: char,
    pub from_port: Option<Port>,
    pub to_port: Option<Port>,
    pub route: Option<Vec<[i64; 2]>>,
}

/// A visual group frame. `member_ids` are the leaf node ids directly in this
/// group; child groups link via their own `parent`. Grid extent and pixel box
/// are filled in later (parse resolves grid extent + depth; geometry the box).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub parent: Option<String>,
    pub member_ids: Vec<String>,
    pub depth: i64,
    // grid extent (inclusive grid-cell indices)
    pub col0: i64,
    pub col1: i64,
    pub row0: i64,
    pub row1: i64,
    // pixel box (inclusive corners), set in geometry
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutError {
    pub code: String,
    pub edge_ids: Vec<String>,
    pub at: Option<[i64; 2]>,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    pub start: i64,
    pub end: i64,
    pub kind: &'static str,
    pub center: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HintPort {
    pub from_side: Option<String>,
    pub to_side: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub title: String,
    pub description: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub groups: Vec<Group>,
    pub hint_ports: IndexMap<String, HintPort>,
    pub routing_order: Vec<String>,
    pub canvas_w: i64,
    pub canvas_h: i64,
    pub box_w: i64,
    pub box_h: i64,
    pub col_x: BTreeMap<i64, i64>,
    pub row_y: BTreeMap<i64, i64>,
    pub col_bands: Vec<Band>,
    pub row_bands: Vec<Band>,
    pub col_kind: Vec<Option<&'static str>>,
    pub row_kind: Vec<Option<&'static str>>,
    pub col_center: Vec<Option<i64>>,
    pub row_center: Vec<Option<i64>>,
    pub errors: Vec<LayoutError>,
}

impl Model {
    pub fn new(title: String, description: String, nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        Self {
            title,
            description,
            nodes,
            edges,
            groups: Vec::new(),
            hint_ports: IndexMap::new(),
            routing_order: Vec::new(),
            canvas_w: 0,
            canvas_h: 0,
            box_w: 0,
            box_h: 0,
            col_x: BTreeMap::new(),
            row_y: BTreeMap::new(),
            col_bands: Vec::new(),
            row_bands: Vec::new(),
            col_kind: Vec::new(),
            row_kind: Vec::new(),
            col_center: Vec::new(),
            row_center: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new(String::new(), String::new(), Vec::new(), Vec::new())
    }
}

pub fn build_band_caches(m: &mut Model) {
    fn flatten(bands: &[Band], n: i64) -> (Vec<Option<&'static str>>, Vec<Option<i64>>) {
        let n = n as usize;
        let mut kind = vec![None; n];
        let mut center = vec![None; n];
        for b in bands {
            let lo = b.start.max(0);
            let hi = b.end.min(n as i64);
            let mut v = lo;
            while v < hi {
                kind[v as usize] = Some(b.kind);
                center[v as usize] = b.center;
                v += 1;
            }
        }
        (kind, center)
    }
    let (ck, cc) = flatten(&m.col_bands, m.canvas_w);
    let (rk, rc) = flatten(&m.row_bands, m.canvas_h);
    m.col_kind = ck;
    m.col_center = cc;
    m.row_kind = rk;
    m.row_center = rc;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_is_62_chars() {
        assert_eq!(EDGE_ALPHABET.chars().count(), 62);
        assert_eq!(EDGE_ALPHABET.chars().next().unwrap(), '0');
        assert_eq!(EDGE_ALPHABET.chars().last().unwrap(), 'Z');
    }

    #[test]
    fn build_band_caches_works() {
        let mut m = Model::default();
        m.canvas_w = 10;
        m.col_bands = vec![
            Band {
                start: 0,
                end: 4,
                kind: "node",
                center: None,
            },
            Band {
                start: 4,
                end: 10,
                kind: "lane",
                center: Some(7),
            },
        ];

        build_band_caches(&mut m);

        assert_eq!(m.col_kind[0], Some("node"));
        assert_eq!(m.col_kind[5], Some("lane"));
        assert_eq!(m.col_center[5], Some(7));
        assert_eq!(m.col_center[0], None);
    }
}
