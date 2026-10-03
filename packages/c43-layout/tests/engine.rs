mod common;
use c43_layout::check::check;
use c43_layout::engine::{build, layout};
use c43_layout::layout::build_layout;
use c43_layout::lanes::MARGIN;
use c43_layout::model::*;
use c43_layout::place::place;
use c43_layout::ports::assign_ports;
use c43_layout::route::route_all;
use c43_layout::skeleton::skeleton;
use c43_layout::tracks::{assign_tracks, lane_segments};
use common::{case_graph, graph, internal, load_case, placement_of};
use serde_json::json;

fn run(name: &str) -> LayoutResult {
    let (g, h) = load_case(name);
    layout(&g, &h, &Default::default()).unwrap()
}

#[test]
fn improved_tracks_never_worse_than_greedy() {
    for name in ["aws", "star16"] {
        let g = case_graph(name);
        let placement = place(&g, &skeleton(&g));
        let ports = assign_ports(&g, &placement);
        let routes = route_all(&g, &placement, &ports);
        let segs = lane_segments(&g, &placement, &ports, &routes);
        let greedy = check(&build_layout(&g, &placement, &ports, &routes, &assign_tracks(&segs), &SizeHints::default(), 0.0, &segs));
        let m = build(&g, &State { placement, ports }, &SizeHints::default(), 1.0).metrics;
        assert!(m.crossings <= greedy.crossings, "{name}");
        assert_eq!(m.crossings, m.violations.iter().filter(|v| v.rule == "crossing").count());
    }
}

#[test]
fn routes_detour_around_an_already_drawn_edge_instead_of_crossing_it() {
    let g = internal(json!({"nodes":[{"id":"A"},{"id":"C"},{"id":"B","kind":"nf"}],"edges":[{"from":"A","to":"C"},{"from":"A","to":"B"}]}));
    let placement = placement_of(&[("A", (0, 0)), ("C", (1, 1)), ("B", (2, 1))]);
    let ports = assign_ports(&g, &placement);
    assert_eq!(build(&g, &State { placement, ports }, &SizeHints::default(), 1.0).metrics.crossings, 0);
}

fn inside(l: &Layout, x: f64, y: f64) -> bool {
    x >= l.frame.x && x <= l.frame.x + l.frame.w && y >= l.frame.y && y <= l.frame.y + l.frame.h
}

#[test]
fn frame_encloses_nodes_and_edges() {
    for name in ["aws", "star16", "rebob-system", "groups"] {
        let l = run(name).layout;
        for n in &l.nodes {
            assert_eq!(n.size, l.s);
            assert!(inside(&l, n.x, n.y) && inside(&l, n.x + n.size, n.y + n.size), "{name} {}", n.id);
        }
        for e in &l.edges {
            assert!(e.points.iter().all(|p| inside(&l, p.x, p.y)), "{name} edge {}", e.id);
        }
    }
}

#[test]
fn frame_is_the_grid() {
    let l = layout(&graph(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]})), &Default::default(), &Default::default()).unwrap().layout;
    let sum = |ws: &[f64]| ws.iter().sum::<f64>();
    assert_eq!(l.frame, Rect { x: 0., y: 0., w: sum(&l.lanes.v) + l.cols as f64 * l.s, h: sum(&l.lanes.h) + l.rows as f64 * l.s });
}

#[test]
fn one_node_frame_is_margin_node_margin() {
    let l = layout(&graph(json!({"nodes":[{"id":"A"}]})), &Default::default(), &Default::default()).unwrap().layout;
    assert_eq!(l.lanes, Lanes { v: vec![MARGIN, MARGIN], h: vec![MARGIN, MARGIN] });
    assert_eq!(l.frame, Rect { x: 0., y: 0., w: 2. * MARGIN + l.s, h: 2. * MARGIN + l.s });
}

#[test]
fn node_size_hint_grows_every_node() {
    let g = graph(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]}));
    let h: Hints = serde_json::from_value(json!({"sizes":{"node":{"B":4}}})).unwrap();
    let l = layout(&g, &h, &Default::default()).unwrap().layout;
    assert_eq!(l.s, 4.0);
    assert!(l.nodes.iter().all(|n| n.size == 4.0));
}

#[test]
fn graphs_without_groups_get_an_empty_group_list() {
    assert!(layout(&graph(json!({"nodes":[{"id":"A"}]})), &Default::default(), &Default::default()).unwrap().layout.groups.is_empty());
}

#[test]
fn groups_layout_rectangles_around_members_titles_inside() {
    let (_, h) = load_case("groups");
    let l = run("groups").layout;
    assert_eq!(l.groups.len(), 4);
    for gr in &l.groups {
        let t = &gr.title;
        assert!(t.x >= gr.x && t.x + t.w <= gr.x + gr.w, "{} title x", gr.id);
        assert!(t.y >= gr.y && t.y + t.h <= gr.y + gr.h, "{} title y", gr.id);
        let want = h.sizes.group_title[&gr.id];
        assert!(want - 6.0 / 20.0 <= t.w, "{} title fits", gr.id);
        assert!(t.w <= want + 1.3, "{} title only as wide as its text", gr.id);
        for n in l.nodes.iter().filter(|n| n.group.as_deref() == Some(&gr.id)) {
            assert!(n.x > gr.x && n.x + n.size < gr.x + gr.w && n.y > t.y + t.h && n.y + n.size < gr.y + gr.h, "{}", n.id);
        }
    }
    let e = l.edges.iter().find(|x| x.from == "api" && x.to == "api/handler").unwrap();
    let api = l.groups.iter().find(|x| x.id == "api").unwrap();
    assert!(e.src.inner);
    assert_eq!(e.src.x, api.x);
    assert_eq!(e.points[0].x, api.x);
}

fn rules(l: &Layout) -> Vec<String> {
    check(l).violations.into_iter().map(|v| v.rule).collect()
}

#[test]
fn groups_case_is_clean() {
    let l = run("groups").layout;
    let m = check(&l);
    assert!(m.violations.iter().all(|v| v.rule == "crossing"), "{:?}", m.violations);
    assert_eq!(m.soft.group_sides, 0.0);
}

#[test]
fn group_rule_violations_are_caught() {
    let base = run("groups").layout;

    let mut l = base.clone();
    l.nodes.iter_mut().find(|x| x.group.as_deref() == Some("store")).unwrap().x = -100.0;
    assert!(rules(&l).contains(&"group-member".into()), "node outside its group");

    let mut l = base.clone();
    let store = l.groups.iter().find(|g| g.id == "store").unwrap().clone();
    let n = l.nodes.iter_mut().find(|x| x.id == "user").unwrap();
    n.x = store.x + 1.0;
    n.y = store.y + 3.0;
    assert!(rules(&l).contains(&"group-member".into()), "foreign node inside a group");

    let mut l = base.clone();
    let tops: Vec<usize> = l.groups.iter().enumerate().filter(|(_, x)| x.depth == 0).map(|(i, _)| i).collect();
    let (ax, ay) = (l.groups[tops[0]].x, l.groups[tops[0]].y);
    l.groups[tops[1]].x = ax;
    l.groups[tops[1]].y = ay;
    assert!(rules(&l).contains(&"group-overlap".into()));

    let mut l = base.clone();
    let g = l.groups[0].clone();
    l.edges[0].points.splice(1..1, [Pt { x: g.x, y: g.y + 1.0 }, Pt { x: g.x, y: g.y + 2.0 }]);
    assert!(rules(&l).contains(&"border-overlap".into()));

    let mut l = base.clone();
    let t = l.groups[0].title.clone();
    l.edges[0].points.splice(1..1, [Pt { x: t.x + t.w / 2.0, y: t.y - 1.0 }, Pt { x: t.x + t.w / 2.0, y: t.y + t.h + 1.0 }]);
    assert!(rules(&l).contains(&"through-title".into()));

    let mut l = base;
    let tops: Vec<usize> = l.groups.iter().enumerate().filter(|(_, x)| x.depth == 0).map(|(i, _)| i).collect();
    l.groups[tops[0]].bx = Box { col0: 0, row0: 0, col1: 0, row1: 1 };
    l.groups[tops[1]].bx = Box { col0: 1, row0: 0, col1: 1, row1: 0 };
    assert_eq!(check(&l).soft.group_sides, 1.0);
}
