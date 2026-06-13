use c43::cmd::layout::model::{Group, Model};

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
