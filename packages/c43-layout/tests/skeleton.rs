mod common;
use c43_layout::skeleton::skeleton;
use common::{case_graph, internal};
use serde_json::json;

fn chain(ids: &[&str], extra: &[(&str, &str)]) -> c43_layout::model::Graph {
    let edges: Vec<_> = ids.windows(2).map(|w| json!({"from": w[0], "to": w[1]}))
        .chain(extra.iter().map(|(f, t)| json!({"from": f, "to": t}))).collect();
    internal(json!({"nodes": ids.iter().map(|id| json!({"id": id})).collect::<Vec<_>>(), "edges": edges}))
}

#[test]
fn chain_one_node_per_column() {
    assert_eq!(skeleton(&chain(&["A", "B", "C"], &[])).columns, vec![vec!["A"], vec!["B"], vec!["C"]]);
}

#[test]
fn aws_columns_follow_longest_path_and_barycenter_order() {
    assert_eq!(skeleton(&case_graph("aws")).columns, vec![
        vec!["U"], vec!["CF"], vec!["AGW"], vec!["LA", "LB", "LC"], vec!["SES", "ATH"], vec!["S3"],
    ]);
}

#[test]
fn star16_data_leaves_wrap_at_column_cap_4() {
    assert_eq!(skeleton(&case_graph("star16")).columns, vec![
        vec!["N1"], vec!["N2"], vec!["N3", "N4", "N5", "N6"], vec!["N7", "N8", "N9"],
    ]);
}

#[test]
fn cycles_terminate_true_source_stays_leftmost() {
    assert_eq!(skeleton(&chain(&["A", "B", "C"], &[("C", "B")])).columns, vec![vec!["A"], vec!["B"], vec!["C"]]);
    assert_eq!(skeleton(&chain(&["A", "B", "C"], &[("C", "A")])).columns, vec![vec!["A"], vec!["B"], vec!["C"]]);
}

#[test]
fn secondary_nodes_are_not_in_the_skeleton() {
    let g = internal(json!({"nodes":[{"id":"A"},{"id":"B","kind":"nf"},{"id":"Z"}],"edges":[{"from":"A","to":"B"}]}));
    assert!(skeleton(&g).columns.is_empty());
}
