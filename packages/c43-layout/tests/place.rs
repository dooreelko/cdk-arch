mod common;
use c43_layout::model::{Cell, Graph, Placement};
use c43_layout::place::{compact, place};
use c43_layout::skeleton::skeleton;
use common::{case_graph, cells, cells_of, internal};
use indexmap::IndexMap;
use serde_json::json;

fn run(g: Graph) -> Placement {
    place(&g, &skeleton(&g))
}

#[test]
fn aws_placement() {
    let p = run(case_graph("aws"));
    assert_eq!((p.cols, p.rows), (6, 4));
    assert_eq!(cells_of(&p), cells(&[
        ("U", (0, 1)), ("CF", (1, 1)), ("AGW", (2, 1)), ("LA", (3, 0)), ("LB", (3, 1)), ("LC", (3, 2)),
        ("SES", (4, 0)), ("ATH", (4, 2)), ("S3", (5, 1)), ("R53", (1, 2)), ("CW", (4, 3)),
    ]));
}

#[test]
fn star16_placement() {
    let p = run(case_graph("star16"));
    assert_eq!((p.cols, p.rows), (5, 6));
    assert_eq!(cells_of(&p), cells(&[
        ("N1", (0, 1)), ("N2", (1, 1)),
        ("N3", (2, 0)), ("N4", (2, 1)), ("N5", (2, 2)), ("N6", (2, 3)),
        ("N7", (3, 0)), ("N8", (3, 1)), ("N9", (3, 2)),
        ("N10", (1, 3)), ("N11", (1, 4)), ("N12", (3, 3)), ("N13", (2, 4)), ("N14", (4, 3)), ("N15", (1, 5)), ("N16", (3, 4)),
    ]));
}

#[test]
fn top_left_cell_may_be_used() {
    let p = run(internal(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]})));
    assert_eq!(cells_of(&p), cells(&[("A", (0, 0)), ("B", (1, 0))]));
    assert_eq!(p.rows, 1);
}

#[test]
fn graph_with_only_nf_edges_places_everything_below() {
    let p = run(internal(json!({"nodes":[{"id":"A"},{"id":"B","kind":"nf"}],"edges":[{"from":"A","to":"B"}]})));
    assert_eq!(cells_of(&p), cells(&[("A", (0, 0)), ("B", (0, 1))]));
}

#[test]
fn isolated_node_goes_to_the_bottom_band() {
    let p = run(internal(json!({"nodes":[{"id":"A"},{"id":"B"},{"id":"Z"}],"edges":[{"from":"A","to":"B"}]})));
    assert_eq!(cells_of(&p), cells(&[("A", (0, 0)), ("B", (1, 0)), ("Z", (0, 1))]));
}

#[test]
fn compact_drops_empty_rows_and_cols() {
    let m: IndexMap<String, Cell> = [("A", (0, -2)), ("B", (3, -2)), ("C", (3, 5))]
        .iter().map(|(id, (col, row))| (id.to_string(), Cell { col: *col, row: *row })).collect();
    let p = compact(&m);
    assert_eq!(cells_of(&p), cells(&[("A", (0, 0)), ("B", (1, 0)), ("C", (1, 1))]));
    assert_eq!((p.cols, p.rows), (2, 2));
}

#[test]
fn secondary_node_pointing_at_its_anchor_is_never_placed_right_of_it() {
    let c = cells_of(&run(internal(json!({
        "nodes": [{"id":"A"},{"id":"B"},{"id":"C"},{"id":"M","kind":"nf"}],
        "edges": [{"from":"A","to":"B"},{"from":"A","to":"C"},{"from":"M","to":"B"}]
    }))));
    assert!(c["M"].0 <= c["B"].0, "M {:?} right of B {:?}", c["M"], c["B"]);
}

#[test]
fn secondary_node_its_anchor_points_at_is_never_placed_left_of_it() {
    let c = cells_of(&run(case_graph("star16")));
    for id in ["N10", "N11", "N12", "N13", "N14", "N15", "N16"] {
        assert!(c[id].0 >= c["N2"].0, "{id} {:?} left of N2 {:?}", c[id], c["N2"]);
    }
}
