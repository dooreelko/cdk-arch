//! Main-level (CLI subprocess) parity tests for `c43 layout`, ported from the
//! `run_main` half of the Python `tests/test_render.py`. Each test drives the
//! built binary as a subprocess in a temp dir and inspects the result files.

use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Write `raw` to `<dir>/layout.json`, run `c43 layout layout.json` in `dir`,
/// and return (parsed result.json, result.txt, exit code).
fn run_layout(dir: &Path, raw: &Value) -> (Option<Value>, Option<String>, i32) {
    std::fs::write(dir.join("layout.json"), serde_json::to_string(raw).unwrap()).unwrap();
    run_layout_path(dir, "layout.json")
}

/// Run `c43 layout <input>` in `dir` against an already-present input file.
fn run_layout_path(dir: &Path, input: &str) -> (Option<Value>, Option<String>, i32) {
    let bin = env!("CARGO_BIN_EXE_c43");
    let st = Command::new(bin)
        .args(["layout", input])
        .current_dir(dir)
        .status()
        .unwrap();
    let rj = std::fs::read_to_string(dir.join("result.json"))
        .ok()
        .map(|s| serde_json::from_str(&s).unwrap());
    let rt = std::fs::read_to_string(dir.join("result.txt")).ok();
    (rj, rt, st.code().unwrap())
}

const FULL_KEYS: [&str; 8] = [
    "status",
    "errors",
    "title",
    "description",
    "canvas",
    "box",
    "nodes",
    "edges",
];

fn assert_validation_error_result(data: &Value) {
    assert_eq!(data["status"], "error");
    assert_eq!(data["errors"][0]["code"], "validation");
    assert!(data["errors"][0]["message"].as_str().unwrap().len() > 0);
    for k in FULL_KEYS {
        assert!(data.get(k).is_some(), "missing key {k}");
    }
}

#[test]
fn main_ok_writes_both_files() {
    let dir = tempfile::tempdir().unwrap();
    let raw = json!({
        "title": "S", "description": "d",
        "nodes": [
            {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
            {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        ],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
    });
    let (data, txt, code) = run_layout(dir.path(), &raw);
    assert_eq!(code, 0);
    let data = data.unwrap();
    assert_eq!(data["status"], "ok");
    assert_eq!(data["errors"], json!([]));
    assert!(data.get("hints").is_none());
    assert!(data.get("auto").is_none());
    let e = &data["edges"][0];
    assert_eq!(e["from"], "a");
    assert_eq!(e["to"], "b");
    assert_eq!(e["char"], "0");
    assert!(!e["route"].is_null());
    assert_eq!(e["from_port"]["side"], "right");
    assert!(data["canvas"]["width"].as_i64().unwrap() > 0);
    assert!(data["box"]["width"].as_i64().unwrap() > 0);
    let n = &data["nodes"][0];
    for k in ["id", "label", "grid_col", "grid_row", "x", "y", "w", "h"] {
        assert!(n.get(k).is_some(), "node missing key {k}");
    }
    assert!(txt.is_some());
}

#[test]
fn main_validation_error_writes_json_only() {
    let dir = tempfile::tempdir().unwrap();
    let raw = json!({
        "title": "S", "description": "d",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
        "edges": [{"id": "e1", "from": "a", "to": "ghost"}],
    });
    let (data, txt, code) = run_layout(dir.path(), &raw);
    assert_eq!(code, 1);
    let data = data.unwrap();
    assert_eq!(data["status"], "error");
    assert_eq!(data["errors"][0]["code"], "validation");
    assert!(data["errors"][0]["message"].as_str().unwrap().len() > 0);
    assert!(txt.is_none());
}

#[test]
fn main_routing_error_still_renders() {
    let dir = tempfile::tempdir().unwrap();
    // K5: 5 nodes fully connected on a 3x2 grid forces desperation crossings.
    let nodes: Vec<Value> = (0..5)
        .map(|i| {
            json!({"id": format!("n{i}"), "label": format!("n{i}"),
                   "grid_col": i % 3, "grid_row": i / 3})
        })
        .collect();
    let mut edges: Vec<Value> = Vec::new();
    for i in 0..5 {
        for j in (i + 1)..5 {
            edges.push(json!({"id": format!("e{i}{j}"),
                              "from": format!("n{i}"), "to": format!("n{j}")}));
        }
    }
    let raw = json!({
        "title": "K5", "description": "crossing test",
        "nodes": nodes, "edges": edges,
    });
    let (data, txt, code) = run_layout(dir.path(), &raw);
    assert_eq!(code, 1);
    let data = data.unwrap();
    assert_eq!(data["status"], "error");
    assert!(data["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"] == "crossing"));
    assert!(txt.is_some()); // rendered despite errors
}

#[test]
fn main_missing_arg_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_c43");
    let st = Command::new(bin)
        .args(["layout"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert_eq!(st.code().unwrap(), 2);
}

#[test]
fn main_malformed_json_writes_error_result() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("layout.json"), "{not valid json").unwrap();
    let (data, txt, code) = run_layout_path(dir.path(), "layout.json");
    assert_eq!(code, 1);
    assert_validation_error_result(&data.unwrap());
    assert!(txt.is_none());
}

#[test]
fn main_missing_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let (data, txt, code) = run_layout_path(dir.path(), "does_not_exist.json");
    assert_eq!(code, 1);
    assert_validation_error_result(&data.unwrap());
    assert!(txt.is_none());
}

#[test]
fn main_non_dict_top_level() {
    let dir = tempfile::tempdir().unwrap();
    let raw = json!([1, 2]);
    let (data, txt, code) = run_layout(dir.path(), &raw);
    assert_eq!(code, 1);
    let data = data.unwrap();
    assert_eq!(data["status"], "error");
    assert!(data["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("JSON object"));
    assert!(txt.is_none());
}

#[test]
fn main_removes_stale_outputs() {
    let dir = tempfile::tempdir().unwrap();
    // Pre-existing outputs from a previous run must never survive a new run.
    std::fs::write(dir.path().join("result.json"), r#"{"status": "ok"}"#).unwrap();
    std::fs::write(dir.path().join("result.txt"), "stale diagram\n").unwrap();
    std::fs::write(dir.path().join("layout.json"), "{broken").unwrap();
    let (data, txt, code) = run_layout_path(dir.path(), "layout.json");
    assert!(txt.is_none()); // stale result.txt removed
    assert_eq!(data.unwrap()["status"], "error"); // result.json reflects new run
    assert_eq!(code, 1);
}

#[test]
fn main_validation_error_has_full_key_set() {
    let dir = tempfile::tempdir().unwrap();
    let raw = json!({
        "title": "S", "description": "d",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
        "edges": [{"id": "e1", "from": "a", "to": "ghost"}],
    });
    let (data, _txt, _code) = run_layout(dir.path(), &raw);
    let data = data.unwrap();
    for k in FULL_KEYS {
        assert!(data.get(k).is_some(), "missing key {k}");
    }
    assert_eq!(data["canvas"], json!({"width": 0, "height": 0}));
    assert_eq!(data["box"], json!({"width": 0, "height": 0}));
    assert_eq!(data["nodes"], json!([]));
    assert_eq!(data["edges"], json!([]));
    assert_eq!(data["title"], "S");
    assert_eq!(data["description"], "d");
}
