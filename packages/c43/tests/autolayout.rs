//! Ports `tests/test_autolayout.py`: the self-sufficient layout loop.
//! A small 2x2 graph with crossings under naive routing exercises the loop.

use c43::cmd::layout::auto::optimise;
use c43::cmd::layout::build_model;
use c43::cmd::layout::report::{quality_of, score_key};
use serde_json::{json, Value};

fn nodes() -> Value {
    json!([
        {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
        {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
        {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
        {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}
    ])
}

fn edges() -> Value {
    json!([
        {"id": "e1", "from": "a", "to": "d"},
        {"id": "e2", "from": "c", "to": "b"},
        {"id": "e3", "from": "a", "to": "b"},
        {"id": "e4", "from": "c", "to": "d"}
    ])
}

fn raw(hints: Option<Value>) -> Value {
    let mut r = json!({
        "title": "T", "description": "D",
        "nodes": nodes(), "edges": edges(),
    });
    if let Some(h) = hints {
        r["hints"] = h;
    }
    r
}

/// trial = raw_in with hints; build_model; score_key(quality_of)
fn score(raw_in: &Value, hints: &Value) -> (i64, i64, i64, i64, i64, i64) {
    let mut trial = raw_in.clone();
    trial["hints"] = hints.clone();
    let m = build_model(&trial).expect("build_model should succeed");
    score_key(&quality_of(&m))
}

#[test]
fn loop_never_returns_worse_than_start() {
    let r = raw(None);
    let start = score(&r, &json!({}));
    let (best_hints, _, _) = optimise(&r, 60);
    assert!(score(&r, &best_hints) <= start);
}

#[test]
fn loop_is_deterministic() {
    let r = raw(None);
    let (h1, q1, n1) = optimise(&r, 60);
    let (h2, q2, n2) = optimise(&r, 60);
    assert_eq!(h1, h2);
    assert_eq!(q1, q2);
    assert_eq!(n1, n2);
}

#[test]
fn loop_respects_user_pinned_ports() {
    // Pin e1's from_side to top; the loop must never change that side.
    let r = raw(Some(json!({"ports": [{"edge_id": "e1", "from_side": "top"}]})));
    let (best_hints, _, _) = optimise(&r, 60);
    let ports = best_hints["ports"].as_array().expect("ports array");
    let e1 = ports
        .iter()
        .find(|h| h["edge_id"] == json!("e1"))
        .expect("e1 hint present");
    assert_eq!(e1["from_side"], json!("top"));
}

#[test]
fn loop_never_proposes_inbound_right() {
    let r = raw(None);
    let (best_hints, _, _) = optimise(&r, 60);
    if let Some(ports) = best_hints.get("ports").and_then(|p| p.as_array()) {
        for h in ports {
            assert_ne!(h.get("to_side"), Some(&json!("right")));
        }
    }
    // and the chosen hints actually build
    let mut trial = r.clone();
    trial["hints"] = best_hints;
    assert!(build_model(&trial).is_ok());
}

#[test]
fn loop_improves_or_matches_a_crossing_graph() {
    let r = raw(None);
    let naive_q = quality_of(&build_model(&r).expect("naive build"));
    let (_, best_q, _) = optimise(&r, 80);
    let best_q = best_q.expect("best quality present");
    assert_eq!(best_q.dropped, 0);
    assert!(score_key(&best_q) <= score_key(&naive_q));
}
