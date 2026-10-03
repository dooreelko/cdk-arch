use c43::drawio::graph::{build, doc_from_json};
use c43_layout::{Hints, InputGraph};
use serde_json::{json, Value};

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn rebob() -> c43::drawio::graph::Built {
    build(&doc_from_json(&serde_json::from_str(&read("tests/fixtures/rebob-container.c43.json")).unwrap()))
}

#[test]
fn rebob_container_dump_builds_the_router_case() {
    let b = rebob();
    let want: InputGraph = serde_json::from_str(&read("../c43-layout/tests/cases/rebob-container/graph.json")).unwrap();
    assert_eq!(serde_json::to_value(&b.graph).unwrap(), serde_json::to_value(&want).unwrap());
    assert!(b.warnings.is_empty(), "{:?}", b.warnings);
}

#[test]
fn hints_match_ts_text_sizes() {
    let b = rebob();
    let want: Hints = serde_json::from_str(&read("../c43-layout/tests/cases/rebob-container/hints.json")).unwrap();
    assert_eq!(b.hints.sizes, want.sizes);
    assert!(b.hints.kinds.is_empty());
}

fn node(uid: &str, ty: &str) -> Value {
    json!({"uid": uid, "name": uid, "type": ty, "attributes": {}})
}

fn rel(s: &str, is: &str, e: &str) -> Value {
    json!({"start": s, "is": is, "end": e})
}

#[test]
fn rejected_relations_become_warnings() {
    let doc = doc_from_json(&json!({
        "nodes": [node("system:s", "system"), node("g", "backend"), node("h", "backend"), node("a", "x"), node("b", "x")],
        "relations": [
            rel("system:s", "contains", "g"), rel("g", "contains", "a"), rel("h", "contains", "a"),
            rel("a", "uses", "a"), rel("a", "uses", "g"), rel("a", "uses", "ghost"), rel("a", "uses", "b")
        ]
    }));
    let b = build(&doc);
    assert_eq!(b.graph.title.as_deref(), Some("system:s"));
    let a = b.graph.nodes.iter().find(|n| n.id == "a").unwrap();
    assert_eq!(a.group.as_deref(), Some("g"));
    assert_eq!(b.graph.edges.iter().map(|e| (e.from.as_str(), e.to.as_str())).collect::<Vec<_>>(), [("a", "b")]);
    assert_eq!(b.warnings, [
        "a contained by both g and h, keeping g",
        "dropped a uses a: self-loop",
        "dropped a uses g: a member cannot point at its own group",
        "dropped a uses ghost: unknown node ghost",
    ]);
    assert!(c43_layout::layout(&b.graph, &b.hints, &Default::default()).is_ok());
}

#[test]
fn labels_only_with_several_kinds() {
    let nodes = json!([node("a", "x"), node("b", "x"), node("c", "x")]);
    let b = build(&doc_from_json(&json!({"nodes": nodes, "relations": [rel("a", "uses", "b"), rel("a", "reads", "b"), rel("b", "uses", "c")]})));
    assert_eq!(b.graph.edges.iter().map(|e| e.label.as_deref()).collect::<Vec<_>>(), [Some("uses, reads"), Some("uses")]);
    let b = build(&doc_from_json(&json!({"nodes": nodes, "relations": [rel("a", "uses", "b"), rel("b", "uses", "c")]})));
    assert!(b.graph.edges.iter().all(|e| e.label.is_none()));
}
