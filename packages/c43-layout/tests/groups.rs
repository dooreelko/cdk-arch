mod common;
use c43_layout::groups::{chain, child_of, contains, depth, is_inner, owner, ROOT};
use c43_layout::model::{Graph, RawGraph};
use c43_layout::normalize::normalize_internal;
use serde_json::json;

fn g() -> Graph {
    let raw: RawGraph = serde_json::from_value(json!({
        "groups": [{"id":"G"},{"id":"H","group":"G"},{"id":"K"}],
        "nodes": [{"id":"a","group":"H"},{"id":"b","group":"G"},{"id":"c","group":"K"},{"id":"d"}]
    })).unwrap();
    normalize_internal(&raw).unwrap()
}

#[test]
fn chain_contains_depth() {
    let g = g();
    assert_eq!(chain(&g, "a"), ["H", "G"]);
    assert_eq!(chain(&g, "H"), ["H", "G"]);
    assert!(chain(&g, "d").is_empty());
    assert!(contains(&g, "G", "a"));
    assert!(!contains(&g, "H", "H"));
    assert_eq!(depth(&g, "a"), 2);
    assert_eq!(depth(&g, "G"), 0);
}

#[test]
fn owner_is_innermost_group_holding_both_ends() {
    let g = g();
    assert_eq!(owner(&g, "a", "b"), "G");
    assert_eq!(owner(&g, "G", "a"), "G");
    assert_eq!(owner(&g, "a", "c"), ROOT);
    assert_eq!(owner(&g, "d", "G"), ROOT);
}

#[test]
fn child_of_and_is_inner() {
    let g = g();
    assert_eq!(child_of(&g, "G", "a").as_deref(), Some("H"));
    assert_eq!(child_of(&g, "G", "b").as_deref(), Some("b"));
    assert_eq!(child_of(&g, "G", "G"), None);
    assert_eq!(child_of(&g, "G", "c"), None);
    assert_eq!(child_of(&g, ROOT, "a").as_deref(), Some("G"));
    assert_eq!(child_of(&g, ROOT, "d").as_deref(), Some("d"));
    assert!(is_inner(&g, "G", "a"));
    assert!(!is_inner(&g, "d", "G"));
}
