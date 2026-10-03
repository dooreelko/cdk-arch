mod common;
use c43_layout::engine::flat_state;
use c43_layout::groups::{chain, owner};
use c43_layout::hier::{hier_place, title_cols_for};
use c43_layout::model::*;
use c43_layout::ports::{assign_ports, end_info, side_count};
use c43_layout::route::route_all;
use c43_layout::tracks::{by_lane, lane_segments};
use common::{case_graph, internal};
use indexmap::IndexSet;
use serde_json::json;

fn solve(mg: &Graph) -> Placement {
    flat_state(mg, &SizeHints::default(), 1.0, 10).placement
}

fn place_with(g: &Graph, tc: &dyn Fn(&Group) -> i32) -> Placement {
    hier_place(g, &solve, tc)
}

fn place(g: &Graph) -> Placement {
    place_with(g, &|_| 1)
}

fn in_box(b: &Box, c: &Cell) -> bool {
    c.col >= b.col0 && c.col <= b.col1 && c.row >= b.row0 && c.row <= b.row1
}

fn disjoint(a: &Box, b: &Box) -> bool {
    a.col1 < b.col0 || b.col1 < a.col0 || a.row1 < b.row0 || b.row1 < a.row0
}

fn within(a: &Box, b: &Box) -> bool {
    a.col0 >= b.col0 && a.col1 <= b.col1 && a.row0 >= b.row0 && a.row1 <= b.row1
}

fn boxes_hold_exactly_their_members(name: &str) {
    let g = case_graph(name);
    let p = place(&g);
    let gb = p.groups.as_ref().unwrap();
    let mut taken: IndexSet<(i32, i32)> = IndexSet::new();
    for n in &g.nodes {
        let c = p.cells[&n.id];
        assert!(c.col >= 0 && c.col < p.cols && c.row >= 0 && c.row < p.rows, "{}", n.id);
        assert!(taken.insert((c.col, c.row)), "cell clash at {}", n.id);
        for gr in &g.groups {
            assert_eq!(in_box(&gb[&gr.id], &c), chain(&g, &n.id).contains(&gr.id), "{} vs {}", n.id, gr.id);
        }
    }
    for a in &g.groups {
        for b in &g.groups {
            if a.id == b.id {
                continue;
            }
            let (x, y) = (&gb[&a.id], &gb[&b.id]);
            if chain(&g, &a.id).contains(&b.id) {
                assert!(within(x, y), "{} in {}", a.id, b.id);
            } else if !chain(&g, &b.id).contains(&a.id) {
                assert!(disjoint(x, y), "{} / {}", a.id, b.id);
            }
        }
    }
}

#[test]
fn groups_boxes_hold_exactly_their_members() {
    boxes_hold_exactly_their_members("groups");
}

#[test]
fn container_boxes_hold_exactly_their_members() {
    boxes_hold_exactly_their_members("rebob-container");
}

#[test]
fn empty_group_is_one_cell() {
    let p = place(&internal(json!({"groups":[{"id":"E"}],"nodes":[{"id":"A"}]})));
    let b = p.groups.unwrap()["E"];
    assert_eq!((b.col1 - b.col0, b.row1 - b.row0), (0, 0));
}

#[test]
fn isolated_member_stays_enclosed() {
    let p = place(&internal(json!({"groups":[{"id":"G"}],"nodes":[{"id":"A","group":"G"},{"id":"B"}]})));
    assert!(in_box(&p.groups.as_ref().unwrap()["G"], &p.cells["A"]));
}

#[test]
fn group_to_group_flow_reads_left_to_right_horizontal_neighbours_share_height() {
    let p = place(&internal(json!({
        "groups":[{"id":"A"},{"id":"B"}],
        "nodes":[{"id":"a","group":"A"},{"id":"b1","group":"B"},{"id":"b2","group":"B"}],
        "edges":[{"from":"a","to":"b1"},{"from":"a","to":"b2"}]
    })));
    let gb = p.groups.unwrap();
    let (a, b) = (gb["A"], gb["B"]);
    assert!(a.col1 < b.col0);
    assert_eq!(b.row1 - b.row0, 1, "b1, b2 stacked");
    assert_eq!((a.row0, a.row1), (b.row0, b.row1));
}

#[test]
fn two_start_groups_stack_vertically_and_share_width() {
    let p = place(&internal(json!({
        "groups":[{"id":"A"},{"id":"B"},{"id":"C"}],
        "nodes":[{"id":"a1","group":"A"},{"id":"a2","group":"A"},{"id":"b","group":"B"},{"id":"c","group":"C"}],
        "edges":[{"from":"a1","to":"a2"},{"from":"a2","to":"c"},{"from":"b","to":"c"}]
    })));
    let gb = p.groups.unwrap();
    let (a, b) = (gb["A"], gb["B"]);
    assert_eq!(a.col1 - a.col0, 1, "a1 → a2 side by side");
    assert_eq!((a.col0, a.col1), (b.col0, b.col1));
    assert!(a.row1 < b.row0 || b.row1 < a.row0);
}

#[test]
fn group_to_own_member_enters_from_the_left_long_title_widens_the_block() {
    let g = internal(json!({
        "groups":[{"id":"G","label":"a very long group title"}],
        "nodes":[{"id":"x","group":"G"},{"id":"y","group":"G"}],
        "edges":[{"from":"G","to":"x"},{"from":"x","to":"y"}]
    }));
    let p = place_with(&g, &|gr| if gr.label.len() > 10 { 4 } else { 1 });
    let b = p.groups.as_ref().unwrap()["G"];
    assert_eq!(b.col1 - b.col0 + 1, 4);
    assert!(p.cells["x"].col < p.cells["y"].col);
}

fn group(id: &str) -> Group {
    Group { id: id.into(), label: id.into(), parent: None }
}

fn sizes(titles: &[(&str, f64)]) -> SizeHints {
    SizeHints { group_title: titles.iter().map(|(id, w)| (id.to_string(), *w)).collect(), ..Default::default() }
}

#[test]
fn title_cols_for_is_conservative() {
    // title widths in units as c43 measures them at 20 px per unit: "x" and "a very long group title that needs room"
    let s = sizes(&[("x", (0.58 * 12.0 * 1.1 + 6.0) / 20.0), ("long", 12.0)]);
    let tc = title_cols_for(5.0, &s);
    assert_eq!(tc(&group("x")), 1);
    assert!(tc(&group("long")) > 1);
}

#[test]
fn title_cols_for_counts_the_border_inset() {
    // a one-column group is at least S + 3 units wide (border slots in the lanes either side), title box 1 unit narrower
    let g3 = ((0.7 + 0.58) * 12.0 * 1.1 + 6.0) / 20.0;
    assert_eq!(title_cols_for(2.0, &sizes(&[("G3", g3)]))(&group("G3")), 1);
    let api = ((0.58 * 7.0 + 0.34 * 4.0) * 12.0 * 1.1 + 6.0) / 20.0; // "api service"
    assert_eq!(title_cols_for(4.0, &sizes(&[("api", api)]))(&group("api")), 1);
}

#[test]
fn group_endpoints_get_border_ports_group_to_member_starts_inside_the_left_border() {
    let g = internal(json!({"groups":[{"id":"G"}],"nodes":[{"id":"u"},{"id":"x","group":"G"}],"edges":[{"from":"u","to":"G"},{"from":"G","to":"x"}]}));
    let p = hier_place(&g, &|mg| flat_state(mg, &SizeHints::default(), 1.0, 5).placement, &|_| 1);
    let ports = assign_ports(&g, &p);
    let entry = end_info(&ports, "G", 0, End::Dst);
    let start = end_info(&ports, "G", 1, End::Src);
    assert_eq!((entry.side, entry.count, entry.inner), (Side::Left, 2, false));
    assert_eq!((start.side, start.count, start.inner), (Side::Left, 2, true));
    assert_ne!(entry.index, start.index);
    assert_eq!(end_info(&ports, "x", 1, End::Dst).side, Side::Left);
    assert_eq!(side_count(&ports, Some(&g.group_ids)), 1);
}

fn grouped() -> (Graph, Placement, Ports, Vec<Route>) {
    let g = case_graph("groups");
    let p = hier_place(&g, &|mg| flat_state(mg, &SizeHints::default(), 1.0, 5).placement, &|_| 1);
    let ports = assign_ports(&g, &p);
    let routes = route_all(&g, &p, &ports);
    (g, p, ports, routes)
}

#[test]
fn an_edge_inside_a_group_never_leaves_the_group_region() {
    let (g, p, _, routes) = grouped();
    for r in &routes {
        let e = &g.edges[r.edge];
        let o = owner(&g, &e.from, &e.to);
        if o.is_empty() {
            continue;
        }
        let b = p.groups.as_ref().unwrap()[&o];
        for l in &r.lanes {
            if l.dir == Dir::V {
                assert!(l.i >= b.col0 && l.i <= b.col1 + 1, "edge {} lane v{}", r.edge, l.i);
            } else {
                assert!(l.i >= b.row0 && l.i <= b.row1 + 1, "edge {} lane h{}", r.edge, l.i);
            }
        }
    }
}

#[test]
fn group_to_member_starts_in_the_groups_left_lane() {
    let (g, p, _, routes) = grouped();
    let e = g.edges.iter().find(|x| x.from == "api" && x.to == "api/handler").unwrap();
    assert_eq!(routes[e.id].lanes[0], Lane { dir: Dir::V, i: p.groups.as_ref().unwrap()["api"].col0 });
}

#[test]
fn lane_segments_carry_their_band_tracks_never_mix_bands() {
    let (g, p, ports, routes) = grouped();
    let segs = lane_segments(&g, &p, &ports, &routes);
    let start = g.edges.iter().find(|e| e.from == "api" && e.to == "api/handler").unwrap();
    assert_eq!(segs.iter().find(|s| s.edge == start.id && s.k == 0).unwrap().band, "h0", "inside api's left border");
    for list in by_lane(&segs).values() {
        assert_eq!(list.iter().map(|s| format!("{}|{}", s.lane, s.band)).collect::<IndexSet<_>>().len(), 1);
    }
}
