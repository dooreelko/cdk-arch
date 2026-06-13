use c43::cmd::layout::geometry::geometry;
use c43::cmd::layout::groups::border_cells;
use c43::cmd::layout::groups::{horizontal_ring_counts, vertical_ring_counts};
use c43::cmd::layout::model::{Group, Model};
use c43::cmd::layout::parse::parse_and_validate;
use c43::cmd::layout::ports::assign_ports;
use c43::cmd::layout::route::route_all;
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

fn raw_grid(groups: serde_json::Value) -> serde_json::Value {
    // 2x2 grid: a(0,0) b(1,0) c(0,1) d(1,1)
    json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0},
            {"id":"c","label":"c","grid_col":0,"grid_row":1},
            {"id":"d","label":"d","grid_col":1,"grid_row":1}
        ],
        "edges":[],
        "groups": groups
    })
}

#[test]
fn extent_from_direct_members() {
    let m = parse_and_validate(&raw_grid(json!([
        {"id":"g","title":"G","members":["a","b","c","d"]}
    ]))).unwrap();
    let g = &m.groups[0];
    assert_eq!((g.col0, g.col1, g.row0, g.row1), (0, 1, 0, 1));
    assert_eq!(g.depth, 0);
}

#[test]
fn parent_extent_includes_child_and_depth_increases() {
    let m = parse_and_validate(&raw_grid(json!([
        {"id":"outer","title":"O","members":["a","b","c"]},
        {"id":"inner","title":"I","members":["d"],"parent":"outer"}
    ]))).unwrap();
    let outer = m.groups.iter().find(|g| g.id == "outer").unwrap();
    let inner = m.groups.iter().find(|g| g.id == "inner").unwrap();
    // outer encloses a(0,0) plus inner's extent at d(1,1)
    assert_eq!((outer.col0, outer.col1, outer.row0, outer.row1), (0, 1, 0, 1));
    assert_eq!(outer.depth, 0);
    assert_eq!((inner.col0, inner.col1, inner.row0, inner.row1), (1, 1, 1, 1));
    assert_eq!(inner.depth, 1);
}

#[test]
fn error_parent_cycle() {
    let err = parse_and_validate(&raw_grid(json!([
        {"id":"x","title":"X","members":["a"],"parent":"y"},
        {"id":"y","title":"Y","members":["b"],"parent":"x"}
    ]))).unwrap_err();
    assert!(err.contains("cycle"), "got: {err}");
}

#[test]
fn error_group_with_no_members_and_no_children() {
    // a group with empty members and no children has no extent -> error
    let err = parse_and_validate(&raw_grid(json!([
        {"id":"empty","title":"E","members":[]}
    ]))).unwrap_err();
    assert!(err.contains("no members"), "got: {err}");
}

#[test]
fn three_level_nesting_extents_and_depth() {
    // gp contains p contains c; only c has a direct member (d at (1,1)),
    // gp also directly holds a at (0,0). Extents must fold up correctly.
    let m = parse_and_validate(&raw_grid(json!([
        {"id":"gp","title":"GP","members":["a","b","c"]},
        {"id":"p","title":"P","members":[],"parent":"gp"},
        {"id":"c","title":"C","members":["d"],"parent":"p"}
    ]))).unwrap();
    let gp = m.groups.iter().find(|g| g.id == "gp").unwrap();
    let p = m.groups.iter().find(|g| g.id == "p").unwrap();
    let c = m.groups.iter().find(|g| g.id == "c").unwrap();
    assert_eq!(c.depth, 2);
    assert_eq!(p.depth, 1);
    assert_eq!(gp.depth, 0);
    // c is just d(1,1)
    assert_eq!((c.col0, c.col1, c.row0, c.row1), (1, 1, 1, 1));
    // p has no direct members, inherits c's extent
    assert_eq!((p.col0, p.col1, p.row0, p.row1), (1, 1, 1, 1));
    // gp = a(0,0) ∪ p's extent (1,1) => (0,1,0,1)
    assert_eq!((gp.col0, gp.col1, gp.row0, gp.row1), (0, 1, 0, 1));
}

#[test]
fn error_encloses_non_member() {
    // group spans a(0,0)..d(1,1) but only claims a and d; b and c are strangers.
    let err = parse_and_validate(&raw_grid(json!([
        {"id":"g","title":"G","members":["a","d"]}
    ]))).unwrap_err();
    // b at (1,0) and c at (0,1) are inside the rectangle but not members
    assert!(err.contains("encloses non-member"), "got: {err}");
}

#[test]
fn nesting_is_allowed() {
    // inner fully inside outer -> OK, no error
    let m = parse_and_validate(&raw_grid(json!([
        {"id":"outer","title":"O","members":["a","b","c","d"]},
        {"id":"inner","title":"I","members":["d"],"parent":"outer"}
    ]))).unwrap();
    assert_eq!(m.groups.len(), 2);
}

#[test]
fn error_partial_overlap() {
    // 3x1 row: a(0,0) b(1,0) e(2,0); g1 spans a..b, g2 spans b..e -> overlap at b
    let raw = json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0},
            {"id":"e","label":"e","grid_col":2,"grid_row":0}
        ],
        "edges":[],
        "groups":[
            {"id":"g1","title":"1","members":["a","b"]},
            {"id":"g2","title":"2","members":["b","e"]}
        ]
    });
    let err = parse_and_validate(&raw).unwrap_err();
    assert!(err.contains("overlap"), "got: {err}");
}

#[test]
fn vertical_ring_counts_pack_two_sides() {
    let m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"mm","label":"mm","grid_col":1,"grid_row":0},
            {"id":"z","label":"z","grid_col":2,"grid_row":0}
        ],
        "edges":[],
        "groups":[
            {"id":"gl","title":"L","members":["a"]},
            {"id":"gr","title":"R","members":["z"]}
        ]
    })).unwrap();
    // map: lane region index -> (left_rings, right_rings)
    let counts = vertical_ring_counts(&m.groups);
    // left bounding lane (region -1): gl's LEFT border sits on the right side of that lane
    assert_eq!(counts.get(&-1).copied().unwrap_or((0,0)), (0, 1));
    // lane region 1 (right of col0): gl's RIGHT border -> left_rings = 1
    assert_eq!(counts.get(&1).copied().unwrap_or((0,0)).0, 1);
    // lane region 3 (right of col1 / left of col2): gr's LEFT border -> right_rings = 1
    assert_eq!(counts.get(&3).copied().unwrap_or((0,0)).1, 1);
}

#[test]
fn horizontal_ring_counts_smoke() {
    let m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":0,"grid_row":1}
        ],
        "edges":[],
        "groups":[{"id":"g","title":"G","members":["a"]}]
    })).unwrap();
    let counts = horizontal_ring_counts(&m.groups);
    // group g spans row 0 only: top border in top lane (region -1) bottom side (.1),
    // bottom border in lane below row 0 (region 1) bottom side (.0).
    assert_eq!(counts.get(&-1).copied().unwrap_or((0,0)).1, 1);
    assert_eq!(counts.get(&1).copied().unwrap_or((0,0)).0, 1);
}

#[test]
fn groupless_geometry_unchanged() {
    use c43::cmd::layout::model::{GUTTER_W, BOX_MARGIN_X};
    let raw = json!({"title":"T","description":"D",
        "nodes":[{"id":"a","label":"alpha","grid_col":0,"grid_row":0},
                 {"id":"b","label":"b","grid_col":1,"grid_row":0}],
        "edges":[{"id":"e1","from":"a","to":"b"}]});
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    assert_eq!(a.x, GUTTER_W + 1 + BOX_MARGIN_X);
}

#[test]
fn left_lane_added_when_groups_present() {
    use c43::cmd::layout::model::{GUTTER_W, BOX_MARGIN_X};
    let mut m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[{"id":"a","label":"a","grid_col":0,"grid_row":0}],
        "edges":[],
        "groups":[{"id":"g","title":"G","members":["a"]}]
    })).unwrap();
    geometry(&mut m);
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    // a left bounding lane now sits between spine and node col 0, pushing the box right
    assert!(a.x > GUTTER_W + 1 + BOX_MARGIN_X,
        "expected left lane to push col 0 right, got x={}", a.x);
}

#[test]
fn group_box_surrounds_member_nodes() {
    let mut m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0}
        ],
        "edges":[],
        "groups":[{"id":"g","title":"G","members":["a","b"]}]
    })).unwrap();
    geometry(&mut m);
    let g = &m.groups[0];
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    let b = m.nodes.iter().find(|n| n.id == "b").unwrap();
    assert!(g.x < a.x, "left border left of node a (g.x={}, a.x={})", g.x, a.x);
    assert!(g.x + g.w - 1 > b.x + b.w - 1, "right border right of node b");
    assert!(g.y < a.y, "top border above nodes");
    assert!(g.y + g.h - 1 > a.y + a.h - 1, "bottom border below nodes");
}

#[test]
fn nested_group_inside_parent_box() {
    let mut m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0}
        ],
        "edges":[],
        "groups":[
            {"id":"outer","title":"O","members":["a","b"]},
            {"id":"inner","title":"I","members":["b"],"parent":"outer"}
        ]
    })).unwrap();
    geometry(&mut m);
    let outer = m.groups.iter().find(|g| g.id == "outer").unwrap();
    let inner = m.groups.iter().find(|g| g.id == "inner").unwrap();
    assert!(outer.x < inner.x, "outer left border left of inner left border (outer.x={}, inner.x={})", outer.x, inner.x);
    assert!(outer.y < inner.y && outer.y + outer.h > inner.y + inner.h,
        "outer encloses inner vertically");
}

#[test]
fn border_cells_classify_sides() {
    use c43::cmd::layout::model::Group;
    // group box 5 wide, 4 tall at (10,10): verticals x=10 & x=14; horizontals y=10 & y=13.
    let g = Group { id:"g".into(), title:"g".into(), parent:None, member_ids:vec![],
        depth:0, col0:0,col1:0,row0:0,row1:0, x:10,y:10,w:5,h:4 };
    let (vert, horiz) = border_cells(std::slice::from_ref(&g));
    assert!(vert.contains(&(10, 11)));   // left edge, mid
    assert!(vert.contains(&(14, 11)));   // right edge, mid
    assert!(horiz.contains(&(11, 10)));  // top edge, mid
    assert!(horiz.contains(&(11, 13)));  // bottom edge, mid
    assert!(vert.contains(&(10, 10)) && horiz.contains(&(10, 10))); // corner = both
}

#[test]
fn edge_crosses_group_border_perpendicular() {
    let mut m = parse_and_validate(&json!({
        "title":"T","description":"D",
        "nodes":[
            {"id":"a","label":"a","grid_col":0,"grid_row":0},
            {"id":"b","label":"b","grid_col":1,"grid_row":0}
        ],
        "edges":[{"id":"e1","from":"a","to":"b"}],
        "groups":[{"id":"g","title":"G","members":["a"]}],
        "hints":{"ports":[{"edge_id":"e1","from_side":"right","to_side":"left"}]}
    })).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    route_all(&mut m);
    let e = &m.edges[0];
    assert!(e.route.is_some(), "edge crossing a group border must route");
    assert!(m.errors.iter().all(|er| er.code != "unroutable"));
}
