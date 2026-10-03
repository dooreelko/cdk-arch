mod common;
use c43_layout::model::{Box as GBox, Dir, Lane, Route};
use c43_layout::ports::assign_ports;
use c43_layout::route::route_all;
use common::{internal, placement_of};
use serde_json::json;

fn routes(input: serde_json::Value, cells: &[(&str, (i32, i32))]) -> Vec<Route> {
    let g = internal(input);
    let p = placement_of(cells);
    route_all(&g, &p, &assign_ports(&g, &p))
}

fn ab(kind: Option<&str>) -> serde_json::Value {
    json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B","kind":kind}]})
}

fn v(i: i32) -> Lane {
    Lane { dir: Dir::V, i }
}

#[test]
fn an_edge_goes_around_a_group_holding_neither_end() {
    let g = internal(json!({"groups":[{"id":"F"}],"nodes":[{"id":"A"},{"id":"f","group":"F"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]}));
    let mut p = placement_of(&[("A", (0, 1)), ("f", (1, 1)), ("B", (2, 1))]);
    p.rows = 3;
    p.groups = Some([("F".to_string(), GBox { col0: 1, row0: 0, col1: 1, row1: 2 })].into_iter().collect());
    let lanes = &route_all(&g, &p, &assign_ports(&g, &p))[0].lanes;
    assert!(!lanes.iter().any(|l| l.dir == Dir::H && (l.i == 1 || l.i == 2)), "{lanes:?}");
}

#[test]
fn adjacent_facing_ports_direct() {
    assert!(routes(ab(None), &[("A", (0, 0)), ("B", (1, 0))])[0].lanes.is_empty());
}

#[test]
fn straight_across_an_empty_cell_direct() {
    assert!(routes(ab(None), &[("A", (0, 0)), ("B", (2, 0))])[0].lanes.is_empty());
}

#[test]
fn vertical_straight_across_an_empty_cell_direct() {
    assert!(routes(ab(Some("nf")), &[("A", (0, 0)), ("B", (0, 2))])[0].lanes.is_empty());
}

#[test]
fn blocked_straight_goes_around_through_lanes() {
    let r = &routes(json!({"nodes":[{"id":"A"},{"id":"X"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]}),
        &[("A", (0, 1)), ("X", (1, 1)), ("B", (2, 1))])[0].lanes;
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], v(1));
    assert_eq!(r[1].dir, Dir::H);
    assert_eq!(r[2], v(2));
}

#[test]
fn diagonal_neighbour_uses_a_single_lane() {
    assert_eq!(routes(ab(None), &[("A", (0, 1)), ("B", (1, 0))])[0].lanes, vec![v(1)]);
}

#[test]
fn upward_in_the_same_column_goes_right_across_into_the_left_side() {
    let r = &routes(ab(None), &[("A", (0, 2)), ("B", (0, 0))])[0].lanes;
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], v(1));
    assert_eq!(r[1].dir, Dir::H);
    assert_eq!(r[2], v(0));
}
