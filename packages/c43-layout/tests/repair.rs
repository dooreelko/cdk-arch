mod common;
use c43_layout::check::{less, score};
use c43_layout::engine::build;
use c43_layout::model::*;
use c43_layout::place::place;
use c43_layout::ports::assign_ports;
use c43_layout::repair::{candidates, repair, shorten, untangle, RepairOptions};
use c43_layout::skeleton::skeleton;
use common::{case_graph, internal, placement_of};
use indexmap::IndexSet;
use serde_json::json;
use std::cell::Cell as Counter;

fn start(g: &Graph) -> State {
    let placement = place(g, &skeleton(g));
    let ports = assign_ports(g, &placement);
    State { placement, ports }
}

fn eval(g: &Graph) -> impl Fn(&State) -> LayoutResult + '_ {
    move |s| build(g, s, &SizeHints::default(), 1.0)
}

#[test]
fn candidates_without_node_moves_only_touch_ports() {
    let g = case_graph("aws");
    let s = start(&g);
    let mut n = 0;
    for c in candidates(&g, &s, false) {
        assert_eq!(c.placement, s.placement);
        n += 1;
    }
    assert!(n > 0);
}

#[test]
fn repair_stops_after_max_evals_evaluations() {
    let g = case_graph("aws");
    let evals = Counter::new(0);
    let e = eval(&g);
    repair(&g, start(&g), &|s| { evals.set(evals.get() + 1); e(s) }, 40, RepairOptions { move_nodes: true, max_evals: 5 });
    assert!(evals.get() <= 5, "evals={}", evals.get());
}

#[test]
fn less_is_lexicographic() {
    assert!(less(&[0., 1., 5.], &[0., 2., 0.]));
    assert!(!less(&[0., 2., 0.], &[0., 2., 0.]));
    assert!(!less(&[1., 0., 0.], &[0., 9., 9.]));
}

fn abc() -> Graph {
    internal(json!({"nodes":[{"id":"A"},{"id":"B"},{"id":"C"}],"edges":[{"from":"A","to":"B"},{"from":"A","to":"C"}]}))
}

#[test]
fn candidates_keep_every_node_on_a_distinct_cell() {
    let g = abc();
    let s = start(&g);
    let mut n = 0;
    for c in candidates(&g, &s, true) {
        n += 1;
        let keys: IndexSet<(i32, i32)> = c.placement.cells.values().map(|x| (x.col, x.row)).collect();
        assert_eq!(keys.len(), c.placement.cells.len());
    }
    assert!(n > 0);
}

#[test]
fn candidates_keep_data_sources_in_the_leftmost_column() {
    let g = abc();
    let s = start(&g);
    for c in candidates(&g, &s, true) {
        assert_eq!(c.placement.cells["A"].col, 0);
    }
}

#[test]
fn candidates_exchange_a_right_side_data_out_with_one_on_the_bottom() {
    let g = internal(json!({"nodes":[{"id":"A"},{"id":"B"},{"id":"C"},{"id":"D"},{"id":"E"}],
        "edges":[{"from":"A","to":"B"},{"from":"A","to":"C"},{"from":"A","to":"D"},{"from":"A","to":"E"}]}));
    let placement = placement_of(&[("A", (0, 1)), ("B", (1, 0)), ("C", (1, 1)), ("D", (1, 2)), ("E", (1, 3))]);
    let s = State { ports: assign_ports(&g, &placement), placement: placement.clone() };
    let on_bottom = |x: &State| {
        let mut v: Vec<usize> = x.ports["A"].bottom.iter().map(|r| r.edge).collect();
        v.sort();
        v
    };
    let before = on_bottom(&s);
    assert!(candidates(&g, &s, true).any(|c| c.placement == placement && on_bottom(&c) != before));
}

#[test]
fn shortening_a_clean_layout_never_changes_a_node_column() {
    let g = case_graph("aws");
    let e = eval(&g);
    let mut budget = usize::MAX;
    let clean = untangle(&g, start(&g), &e, 40, true, &mut budget);
    assert!(e(&clean).metrics.violations.is_empty());
    let shortened = shorten(&g, clean.clone(), &e, 40, true, &mut budget);
    for (id, c) in &clean.placement.cells {
        assert_eq!(shortened.placement.cells[id].col, c.col, "{id}");
    }
}

#[test]
fn score_follows_idea_priority() {
    let m = |leftward: f64, nf_bottom_share: f64, crossings: usize| Metrics {
        violations: (0..crossings).map(|_| Violation { rule: "crossing".into(), detail: String::new() }).collect(),
        crossings,
        soft: Soft { leftward, centrality: 0., area: 0., aspect: 1., nf_bottom_share, length: 0., turns: 0., group_sides: 0. },
    };
    assert!(less(&score(&m(0., 1., 5)), &score(&m(1., 1., 0))));
    assert!(less(&score(&m(0., 1., 5)), &score(&m(0., 0.5, 0))));
    assert!(less(&score(&m(0., 1., 0)), &score(&m(0., 1., 1))));
}
