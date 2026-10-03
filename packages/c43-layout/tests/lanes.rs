mod common;
use c43_layout::lanes::{band_of, borders, lane_plan, Half, LANE_MIN, MARGIN};
use c43_layout::model::{Box as GBox, Cell, Graph, Placement};
use common::internal;
use indexmap::IndexMap;
use serde_json::json;

fn keys(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

fn setup() -> (Graph, Placement) {
    let g = internal(json!({
        "groups": [{"id":"G"},{"id":"H","group":"G"}],
        "nodes": [{"id":"a","group":"H"},{"id":"b","group":"G"},{"id":"c"}]
    }));
    // G = cols 0..1, H = col 0, c at col 2
    let p = Placement {
        cols: 3,
        rows: 1,
        cells: [("a", 0), ("b", 1), ("c", 2)].iter().map(|(id, col)| (id.to_string(), Cell { col: *col, row: 0 })).collect(),
        groups: Some([("G", GBox { col0: 0, row0: 0, col1: 1, row1: 0 }), ("H", GBox { col0: 0, row0: 0, col1: 0, row1: 0 })]
            .iter().map(|(id, b)| (id.to_string(), *b)).collect()),
    };
    (g, p)
}

fn ranks(m: &IndexMap<String, i32>) -> Vec<(&str, i32)> {
    m.iter().map(|(id, k)| (id.as_str(), *k)).collect()
}

#[test]
fn border_ranks_innermost_nearest_the_cells() {
    let (g, p) = setup();
    let bs = borders(&g, &p);
    assert_eq!(ranks(&bs["v0"].high), [("H", 1), ("G", 2)]);
    assert_eq!(ranks(&bs["v1"].low), [("H", 1)]);
    assert_eq!(ranks(&bs["v2"].low), [("G", 1)]);
    assert!(!bs.contains_key("v3"));
}

#[test]
fn border_rank_follows_nesting_depth() {
    let g2 = internal(json!({
        "groups": [{"id":"P"},{"id":"C1","group":"P"},{"id":"C2","group":"P"}],
        "nodes": [{"id":"a","group":"C1"},{"id":"b","group":"C2"}]
    }));
    let p2 = Placement {
        cols: 2,
        rows: 1,
        cells: [("a", 0), ("b", 1)].iter().map(|(id, col)| (id.to_string(), Cell { col: *col, row: 0 })).collect(),
        groups: Some([
            ("P", GBox { col0: 0, row0: 0, col1: 1, row1: 0 }),
            ("C1", GBox { col0: 0, row0: 0, col1: 0, row1: 0 }),
            ("C2", GBox { col0: 1, row0: 0, col1: 1, row1: 0 }),
        ].iter().map(|(id, b)| (id.to_string(), *b)).collect()),
    };
    let top = &borders(&g2, &p2)["h0"].high;
    assert_eq!(top["C1"], 1);
    assert_eq!(top["C2"], 1);
    assert_eq!(top["P"], 2);
}

#[test]
fn bands_inside_a_border_between_borders_outside() {
    let (g, p) = setup();
    let bs = borders(&g, &p);
    assert_eq!(band_of(&g, bs.get("v0"), "H"), "h0");
    assert_eq!(band_of(&g, bs.get("v0"), "G"), "h1");
    assert_eq!(band_of(&g, bs.get("v0"), ""), "m");
    assert_eq!(band_of(&g, bs.get("v2"), "G"), "l0");
    assert_eq!(band_of(&g, None, "G"), "m");
}

#[test]
fn no_borders_each_lane_its_own_width_outer_lanes_keep_the_margin() {
    let k = |l: &str, _: &str| if l == "v2" { 2 } else { 1 };
    let plan = lane_plan(5.0, &IndexMap::new(), Box::new(k), 2.0, &keys(&["v0", "v1", "v2", "v3"]));
    assert_eq!(plan.width("v0"), MARGIN, "outer, contents need only 3");
    assert_eq!(plan.width("v1"), 3.0, "inner, one band of reach 1");
    assert_eq!(plan.width("v2"), 5.0, "inner, reach 2 needs 5");
    assert_eq!(plan.width("v3"), MARGIN);
    assert_eq!(["v0", "v1", "v2", "v3"].map(|l| plan.start(l)), [0.0, 10.0, 18.0, 28.0]);
    assert_eq!(plan.pos("v1", "m", 1.0), 10.0 + 1.5 + 1.0, "centred in its own lane");
    assert_eq!(plan.pos("v2", "m", -2.0), 18.0 + 2.5 - 2.0);
}

#[test]
fn no_lane_is_even_or_below_the_floor() {
    let plan = lane_plan(5.0, &IndexMap::new(), Box::new(|_: &str, _: &str| 0), 2.0, &keys(&["v0", "v1", "v2", "h0", "h1", "h2"]));
    for l in ["v1", "h1"] {
        assert_eq!(plan.width(l), LANE_MIN);
    }
    for l in ["v0", "v2", "h0", "h2"] {
        assert_eq!(plan.width(l), MARGIN);
    }
}

#[test]
fn borders_and_bands_do_not_collide_and_fit_their_lane() {
    let (g, p) = setup();
    let bs = borders(&g, &p);
    let plan = lane_plan(5.0, &bs, Box::new(|_: &str, _: &str| 0), 2.0, &keys(&["v0", "v1", "v2", "v3", "h0", "h1"]));
    let xs = [plan.pos("v0", "h0", 0.0), plan.border("v0", Half::High, 1), plan.pos("v0", "h1", 0.0), plan.border("v0", Half::High, 2), plan.pos("v0", "m", 0.0)];
    let mut sorted = xs;
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(sorted, xs, "from the cells (high end) outwards");
    assert!(xs.windows(2).all(|w| w[0] != w[1]));
    assert!(xs.iter().all(|x| *x > 0.0 && *x < plan.width("v0")));
    assert_eq!(plan.width("v0") % 2.0, 1.0);
}

#[test]
fn top_lane_reserves_title_room_inside_each_top_border() {
    let (g, p) = setup();
    let bs = borders(&g, &p);
    let plan = lane_plan(5.0, &bs, Box::new(|_: &str, _: &str| 0), 2.0, &keys(&["v0", "v1", "v2", "v3", "h0", "h1"]));
    assert!(plan.border("h0", Half::High, 1) - plan.pos("h0", "h0", 0.0) <= -2.0, "title room between H border and its tracks");
    assert!(plan.width("h0") >= 9.0);
}
