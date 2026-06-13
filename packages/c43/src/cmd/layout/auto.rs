//! Self-sufficient layout loop: naive start -> iterate on engine feedback ->
//! settle on the best layout reachable within an eval budget.
//!
//! Ports `autolayout.py`'s `optimise` (the deterministic best-improvement
//! hill-climb over port hints). Hints are modelled as `serde_json::Value`
//! objects/arrays so equality and key-order match Python dicts exactly
//! (serde_json's `preserve_order` is on). The loop NEVER discards a
//! user-supplied hint; it only adds or changes port sides the user left
//! unspecified, and only when that strictly improves the score.

use super::build_model;
use super::report::{quality_and_diagnostics, score_key, Quality};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Inbound 'right' is prohibited by the engine, so the loop never proposes it.
const OUT_SIDES: [&str; 4] = ["left", "right", "top", "bottom"];
const IN_SIDES: [&str; 3] = ["left", "top", "bottom"];
pub const DEFAULT_MAX_EVALS: usize = 200;

/// The hints `Value` for an edge end: `"from"` -> `"from_side"`, else `"to_side"`.
fn side_key(end: &str) -> &'static str {
    if end == "from" {
        "from_side"
    } else {
        "to_side"
    }
}

/// `hints["ports"]` as a slice, or empty.
fn ports_of(hints: &Value) -> &[Value] {
    hints
        .get("ports")
        .and_then(|p| p.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Return a DEEP-COPIED hints `Value` with `edge_id`'s `end` side set to
/// `side`. Preserves every existing entry (including the user's) and only
/// overwrites the one side being proposed. Mirrors `_with_port_override`.
fn with_port_override(hints: &Value, edge_id: &str, end: &str, side: &str) -> Value {
    let mut new = hints.clone();
    // ensure "ports" is an array (setdefault)
    if !new.get("ports").map(|p| p.is_array()).unwrap_or(false) {
        new["ports"] = json!([]);
    }
    let key = side_key(end);
    let arr = new["ports"].as_array_mut().unwrap();
    // find the matching edge_id entry
    let found = arr
        .iter_mut()
        .find(|h| h.get("edge_id").map(|v| v == &json!(edge_id)).unwrap_or(false));
    match found {
        Some(h) => {
            h[key] = json!(side);
        }
        None => {
            arr.push(json!({ "edge_id": edge_id, key: side }));
        }
    }
    new
}

/// Set of (edge_id, end) the user explicitly fixed. Mirrors `_user_pinned`.
fn user_pinned(user_hints: &Value) -> HashSet<(String, String)> {
    let mut pinned = HashSet::new();
    for h in ports_of(user_hints) {
        let eid = match h.get("edge_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if h.get("from_side").is_some() {
            pinned.insert((eid.clone(), "from".to_string()));
        }
        if h.get("to_side").is_some() {
            pinned.insert((eid, "to".to_string()));
        }
    }
    pinned
}

/// Result of a single evaluation: the routed model plus its derived signals.
struct Eval {
    key: (i64, i64, i64, i64, i64, i64),
    quality: Quality,
    flagged: Vec<String>,
    /// edge ids the engine reported as `unroutable` (model's `m.errors`),
    /// used by `candidates` for routing-order promotion.
    unroutable: Vec<String>,
}

/// Route `raw` under `hints`; return Some(Eval) or None if the hints are
/// invalid (e.g. inbound-right slipped in). `flagged` lists edges the engine
/// complained about (hard errors first, then soft diagnostics) deduped in a
/// deterministic first-seen order. Mirrors `_eval`.
fn eval(raw: &Value, hints: &Value) -> Option<Eval> {
    let mut trial = raw.clone();
    trial["hints"] = hints.clone();
    let m = match build_model(&trial) {
        Ok(m) => m,
        Err(_) => return None, // ValidationError
    };
    let (q, diags) = quality_and_diagnostics(&m);
    // unroutable edge ids, for routing-order promotion in `candidates`.
    // NOTE: Python's autolayout reads `e.id` here, but `LayoutError` has no
    // `id` field (only `edge_ids`) -- so on an unroutable error the Python
    // would raise AttributeError. The tested graphs never produce one, so this
    // never diverges; we use `edge_ids` as the faithful, non-crashing reading.
    let unroutable: Vec<String> = m
        .errors
        .iter()
        .filter(|e| e.code == "unroutable")
        .flat_map(|e| e.edge_ids.iter().cloned())
        .collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut flagged: Vec<String> = Vec::new();
    // hard errors first (they dominate the score), then soft diagnostics
    let id_lists = m
        .errors
        .iter()
        .map(|e| e.edge_ids.clone())
        .chain(diags.iter().map(|d| {
            d["edge_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        }));
    for ids in id_lists {
        for eid in ids {
            if seen.insert(eid.clone()) {
                flagged.push(eid);
            }
        }
    }
    Some(Eval {
        key: score_key(&q),
        quality: q,
        flagged,
        unroutable,
    })
}

/// Best-improvement, defect-driven hill-climb. Returns
/// `(best_hints, best_quality, evals_used)`. Mirrors `optimise`.
pub fn optimise(raw: &Value, max_evals: usize) -> (Value, Option<Quality>, usize) {
    let user_hints = match raw.get("hints") {
        Some(h) if h.is_object() => h.clone(),
        _ => json!({}),
    };
    let pinned = user_pinned(&user_hints);

    let mut best_hints = user_hints.clone();
    let mut evals: usize = 1;
    let cur = match eval(raw, &best_hints) {
        Some(c) => c,
        None => return (best_hints, None, evals), // user's own hints are invalid
    };
    let mut best_key = cur.key;
    let mut best_q = cur.quality;
    let mut best_flagged = cur.flagged;
    let mut best_unroutable = cur.unroutable;

    // perfect: first five score terms all zero (length term never matters here).
    let perfect = |k: &(i64, i64, i64, i64, i64, i64)| {
        k.0 == 0 && k.1 == 0 && k.2 == 0 && k.3 == 0 && k.4 == 0
    };

    while evals < max_evals && !perfect(&best_key) {
        let cands = candidates(&best_hints, &best_flagged, &best_unroutable, &pinned);
        let mut round_best: Option<(
            (i64, i64, i64, i64, i64, i64),
            Quality,
            Vec<String>,
            Vec<String>,
            Value,
        )> = None;
        for trial_hints in cands {
            if evals >= max_evals {
                break;
            }
            let res = eval(raw, &trial_hints);
            evals += 1;
            let res = match res {
                Some(r) => r,
                None => continue,
            };
            // strict best-improvement: first candidate to reach a given best
            // key within the round wins (`< round_best.key`, strictly).
            if res.key < best_key
                && round_best
                    .as_ref()
                    .map(|rb| res.key < rb.0)
                    .unwrap_or(true)
            {
                round_best = Some((
                    res.key,
                    res.quality,
                    res.flagged,
                    res.unroutable,
                    trial_hints,
                ));
            }
        }
        match round_best {
            None => break, // local optimum
            Some((k, q, flagged, unroutable, hints)) => {
                best_key = k;
                best_q = q;
                best_flagged = flagged;
                best_unroutable = unroutable;
                best_hints = hints;
            }
        }
    }

    (best_hints, Some(best_q), evals)
}

/// Deterministic list of single-change neighbours of the current best, derived
/// from what the engine flagged. Order MUST match Python exactly. Mirrors the
/// inner `candidates` closure.
fn candidates(
    best_hints: &Value,
    flagged: &[String],
    unroutable: &[String],
    pinned: &HashSet<(String, String)>,
) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    // 1. port-side changes on flagged edges' unpinned ends
    for eid in flagged {
        for (end, sides) in [("to", &IN_SIDES[..]), ("from", &OUT_SIDES[..])] {
            if pinned.contains(&(eid.clone(), end.to_string())) {
                continue;
            }
            for side in sides {
                out.push(with_port_override(best_hints, eid, end, side));
            }
        }
    }
    // 2. routing-order promotion for wrapping edges. wrap_ids = unroutable edge
    //    ids (from best_m.errors), then flagged (already wrap-first).
    let mut wrap_ids: Vec<String> = unroutable.to_vec();
    wrap_ids.extend(flagged.iter().cloned());

    let existing_order: Vec<String> = best_hints
        .get("routing_order")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    for eid in wrap_ids {
        if existing_order.first() == Some(&eid) {
            continue;
        }
        let mut new = best_hints.clone();
        let mut new_order: Vec<Value> = vec![json!(eid)];
        for x in &existing_order {
            if x != &eid {
                new_order.push(json!(x));
            }
        }
        new["routing_order"] = Value::Array(new_order);
        out.push(new);
    }
    out
}
