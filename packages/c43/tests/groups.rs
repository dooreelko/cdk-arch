use c43::cmd::layout::model::{Group, Model};
use c43::cmd::layout::parse::parse_and_validate;
use serde_json::json;

#[test]
fn model_starts_with_no_groups() {
    let m = Model::default();
    assert!(m.groups.is_empty());
}

#[test]
fn group_struct_has_expected_fields() {
    let g = Group {
        id: "g".into(),
        title: "G".into(),
        parent: None,
        member_ids: vec!["a".into()],
        depth: 0,
        col0: 0, col1: 0, row0: 0, row1: 0,
        x: 0, y: 0, w: 0, h: 0,
    };
    assert_eq!(g.id, "g");
    assert_eq!(g.member_ids, vec!["a".to_string()]);
}

fn raw_with_groups(groups: serde_json::Value) -> serde_json::Value {
    json!({
        "title": "T", "description": "D",
        "nodes": [
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0}
        ],
        "edges": [{"id":"e1","from":"a","to":"b"}],
        "groups": groups
    })
}

#[test]
fn parses_valid_group_entry() {
    let m = parse_and_validate(&raw_with_groups(json!([
        {"id":"g1","title":"Group One","members":["a","b"]}
    ]))).unwrap();
    assert_eq!(m.groups.len(), 1);
    let g = &m.groups[0];
    assert_eq!(g.id, "g1");
    assert_eq!(g.title, "Group One");
    assert_eq!(g.parent, None);
    assert_eq!(g.member_ids, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn group_parent_is_read() {
    let m = parse_and_validate(&raw_with_groups(json!([
        {"id":"outer","title":"O","members":["a"]},
        {"id":"inner","title":"I","members":["b"],"parent":"outer"}
    ]))).unwrap();
    let inner = m.groups.iter().find(|g| g.id == "inner").unwrap();
    assert_eq!(inner.parent.as_deref(), Some("outer"));
}

#[test]
fn no_groups_key_means_empty() {
    let raw = json!({
        "title":"T","description":"D",
        "nodes":[{"id":"a","label":"a","grid_col":0,"grid_row":0}],
        "edges":[]
    });
    assert!(parse_and_validate(&raw).unwrap().groups.is_empty());
}

#[test]
fn error_unknown_member() {
    let err = parse_and_validate(&raw_with_groups(json!([
        {"id":"g1","title":"G","members":["nope"]}
    ]))).unwrap_err();
    assert!(err.contains("nope"), "got: {err}");
}

#[test]
fn error_unknown_parent() {
    let err = parse_and_validate(&raw_with_groups(json!([
        {"id":"g1","title":"G","members":["a"],"parent":"ghost"}
    ]))).unwrap_err();
    assert!(err.contains("ghost"), "got: {err}");
}

#[test]
fn error_duplicate_group_id() {
    let err = parse_and_validate(&raw_with_groups(json!([
        {"id":"g","title":"A","members":["a"]},
        {"id":"g","title":"B","members":["b"]}
    ]))).unwrap_err();
    assert!(err.contains("duplicate group id"), "got: {err}");
}
