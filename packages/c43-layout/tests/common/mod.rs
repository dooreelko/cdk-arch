#![allow(dead_code)]
use c43_layout::model::{Cell, Hints, InputGraph, Placement};
use indexmap::IndexMap;

pub fn case_dir(name: &str) -> String {
    format!("{}/tests/cases/{name}", env!("CARGO_MANIFEST_DIR"))
}

pub fn load_case(name: &str) -> (InputGraph, Hints) {
    let read = |f: &str| std::fs::read_to_string(format!("{}/{f}", case_dir(name))).unwrap();
    (serde_json::from_str(&read("graph.json")).unwrap(), serde_json::from_str(&read("hints.json")).unwrap())
}

pub fn graph(v: serde_json::Value) -> InputGraph {
    serde_json::from_value(v).unwrap()
}

pub fn hints(v: serde_json::Value) -> Hints {
    serde_json::from_value(v).unwrap()
}

pub fn cells_of(p: &Placement) -> IndexMap<String, (i32, i32)> {
    p.cells.iter().map(|(id, c)| (id.clone(), (c.col, c.row))).collect()
}

pub fn placement_of(cells: &[(&str, (i32, i32))]) -> Placement {
    let map: IndexMap<String, Cell> = cells.iter().map(|(id, (col, row))| (id.to_string(), Cell { col: *col, row: *row })).collect();
    Placement {
        cols: map.values().map(|c| c.col).max().unwrap() + 1,
        rows: map.values().map(|c| c.row).max().unwrap() + 1,
        cells: map,
        groups: None,
    }
}
