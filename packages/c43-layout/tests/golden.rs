mod common;
use c43_layout::engine::layout;
use common::{case_dir, load_case};
use serde_json::Value;

/// numbers compared as numbers: the TS output prints 3 where serde prints 3.0
fn norm(v: Value) -> Value {
    match v {
        Value::Number(n) => serde_json::json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.into_iter().map(norm).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, norm(v))).collect()),
        x => x,
    }
}

/// first paths where the two values differ
fn diff(a: &Value, b: &Value, path: String, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => diff(p, q, format!("{path}.{k}"), out),
                    (p, q) => out.push(format!("{path}.{k}: {p:?} vs {q:?}")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            x.iter().zip(y).enumerate().for_each(|(i, (p, q))| diff(p, q, format!("{path}[{i}]"), out))
        }
        (p, q) if p != q => out.push(format!("{path}: {p} vs {q}")),
        _ => {}
    }
}

fn golden(name: &str) {
    let (g, h) = load_case(name);
    let got = serde_json::to_value(layout(&g, &h, &Default::default()).unwrap()).unwrap();
    let want: Value = serde_json::from_str(&std::fs::read_to_string(format!("{}/layout.json", case_dir(name))).unwrap()).unwrap();
    let (got, want) = (norm(got), norm(want));
    if got != want {
        let out = format!("{}/target-got-{name}.json", std::env::temp_dir().display());
        std::fs::write(&out, serde_json::to_string_pretty(&got).unwrap()).unwrap();
        let mut d = Vec::new();
        diff(&got, &want, String::new(), &mut d);
        panic!("{name}: layout differs from the TS engine output (got written to {out}):\n{}", d[..d.len().min(20)].join("\n"));
    }
}

#[test]
fn aws() {
    golden("aws")
}

#[test]
fn star16() {
    golden("star16")
}

#[test]
fn groups() {
    golden("groups")
}

#[test]
fn rebob_system() {
    golden("rebob-system")
}

#[test]
fn rebob_container() {
    let t = std::time::Instant::now();
    golden("rebob-container");
    if !cfg!(debug_assertions) {
        assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    }
}
