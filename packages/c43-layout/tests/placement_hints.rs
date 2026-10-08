mod common;
use c43_layout::layout;
use c43_layout::model::LayoutResult;
use c43_layout::place::place;
use c43_layout::skeleton::skeleton;
use common::{graph, hints};
use serde_json::json;

fn col_row(r: &LayoutResult, id: &str) -> (i32, i32) {
    let n = r.layout.nodes.iter().find(|n| n.id == id).unwrap();
    (n.col, n.row)
}

fn run(g: serde_json::Value, h: serde_json::Value) -> LayoutResult {
    layout(&graph(g), &hints(json!({"placement": h})), &Default::default()).unwrap()
}

fn chain() -> serde_json::Value {
    json!({"nodes":[{"id":"a"},{"id":"b"},{"id":"c"},{"id":"d"}],"edges":[{"from":"a","to":"b"},{"from":"b","to":"c"},{"from":"d","to":"c"}]})
}

#[test]
fn without_hints_layout_has_no_ignored_hints() {
    let r = run(chain(), json!([]));
    assert!(r.metrics.ignored_hints.is_empty());
    assert_eq!(col_row(&r, "d").0, 0);
}

#[test]
fn leftmost_pulls_a_mid_node_to_the_first_column() {
    // c has incoming edges but is declared the start: nothing is left of it
    let r = run(chain(), json!([{"rel":"leftmost","a":"c","priority":true}]));
    let min = r.layout.nodes.iter().map(|n| n.col).min().unwrap();
    assert_eq!(col_row(&r, "c").0, min);
}

#[test]
fn left_of_moves_unrelated_node_right_of_the_other() {
    let g = json!({"nodes":[{"id":"a"},{"id":"b"},{"id":"x"},{"id":"y"}],"edges":[{"from":"a","to":"b"},{"from":"x","to":"y"}]});
    let r = run(g.clone(), json!([{"rel":"right-of","a":"x","b":"b"}]));
    assert!(col_row(&r, "x").0 > col_row(&r, "b").0);
    assert!(r.metrics.ignored_hints.is_empty(), "{:?}", r.metrics.ignored_hints);
}

#[test]
fn same_row_and_above() {
    let g = json!({"nodes":[{"id":"a"},{"id":"b"},{"id":"c"},{"id":"d"}],"edges":[{"from":"a","to":"b"},{"from":"c","to":"d"}]});
    let r = run(g.clone(), json!([{"rel":"same-row","a":"a","b":"d"}]));
    assert_eq!(col_row(&r, "a").1, col_row(&r, "d").1);
    let r = run(g, json!([{"rel":"above","a":"d","b":"a"}]));
    assert!(col_row(&r, "d").1 < col_row(&r, "a").1);
}

#[test]
fn impossible_hint_is_reported_not_fatal() {
    // a->b is an edge: b cannot also be left of a
    let g = json!({"nodes":[{"id":"a"},{"id":"b"}],"edges":[{"from":"a","to":"b"}]});
    let r = run(g, json!([{"rel":"left-of","a":"b","b":"a"}]));
    assert_eq!(r.metrics.ignored_hints.len(), 1);
}

#[test]
fn hints_cross_group_boundaries() {
    let g = json!({"groups":[{"id":"G"}],"nodes":[{"id":"s","group":"G"},{"id":"db"}],"edges":[{"from":"s","to":"db"}]});
    let r = run(g, json!([{"rel":"right-of","a":"db","b":"s"}]));
    assert!(col_row(&r, "db").0 > col_row(&r, "s").0);
}

#[test]
fn above_works_for_nf_node_outside_a_group() {
    let g = json!({"groups":[{"id":"G"}],"nodes":[{"id":"o","group":"G"},{"id":"e","group":"G"},{"id":"ssm"},{"id":"v"}],
        "edges":[{"from":"v","to":"o"},{"from":"o","to":"e"},{"from":"e","to":"ssm"}]});
    let h = json!({"placement":[{"rel":"above","a":"ssm","b":"e"}],"kinds":[{"from":"e","to":"ssm","kind":"nf"}]});
    let r = layout(&graph(g), &hints(h), &Default::default()).unwrap();
    assert!(col_row(&r, "ssm").1 < col_row(&r, "e").1, "{:?} {:?}", col_row(&r, "ssm"), col_row(&r, "e"));
}

#[test]
fn node_with_an_incoming_nf_edge_follows_its_predecessor_not_the_sources() {
    // r is fed only by an nf edge from a, so it is no source: it sits in a's column and what it feeds goes right
    let g = common::internal(json!({
        "nodes": [{"id":"s"},{"id":"a"},{"id":"x"},{"id":"r"},{"id":"t"}],
        "edges": [
            {"from":"s","to":"a"},{"from":"a","to":"x"},
            {"from":"a","to":"r","kind":"nf"},{"from":"r","to":"t","kind":"data"}
        ]
    }));
    let sk = skeleton(&g);
    let col = |id: &str| sk.columns.iter().position(|c| c.iter().any(|n| n == id)).unwrap();
    assert_eq!(col("r"), col("a"));
    assert!(col("t") > col("r"));
    let p = place(&g, &sk);
    assert_eq!(p.cells["r"].col, p.cells["a"].col);
    assert!(p.cells["r"].row > p.cells["a"].row, "nf runs downward");
}

#[test]
fn a_node_without_any_incoming_edge_is_still_a_source() {
    let g = json!({"nodes":[{"id":"s"},{"id":"a"},{"id":"t"}],"edges":[{"from":"s","to":"a"},{"from":"t","to":"a"}]});
    let r = run(g, json!([]));
    assert_eq!(col_row(&r, "t").0, 0);
    assert_eq!(col_row(&r, "s").0, 0);
}

#[test]
fn same_row_lines_a_plain_node_up_with_a_member_of_a_sibling_group() {
    let g = json!({
        "groups": [{"id":"G"}],
        "nodes": [{"id":"o"},{"id":"p","group":"G"},{"id":"q","group":"G"},{"id":"r","group":"G"}],
        "edges": [{"from":"o","to":"p"},{"from":"p","to":"q"},{"from":"p","to":"r"}]
    });
    for peer in ["q", "r"] {
        let res = run(g.clone(), json!([{"rel":"same-row","a":"o","b":peer}]));
        assert_eq!(col_row(&res, "o").1, col_row(&res, peer).1, "o beside {peer}");
        assert!(res.metrics.ignored_hints.is_empty(), "{:?}", res.metrics.ignored_hints);
    }
}

#[test]
fn same_row_wins_over_a_below_hint_listed_before_it() {
    // moving cf onto v's row must not undo "r below cf"
    let g = json!({
        "nodes": [{"id":"s"},{"id":"cf"},{"id":"site"},{"id":"r","kind":"nf"},{"id":"v"},{"id":"w"}],
        "edges": [{"from":"s","to":"cf"},{"from":"cf","to":"site"},{"from":"cf","to":"v"},{"from":"w","to":"v"},{"from":"r","to":"cf"}]
    });
    let res = run(g, json!([
        {"rel":"below","a":"r","b":"cf"},
        {"rel":"above","a":"site","b":"cf"},
        {"rel":"same-row","a":"v","b":"cf"}
    ]));
    assert!(res.metrics.ignored_hints.is_empty(), "{:?}", res.metrics.ignored_hints);
    assert!(col_row(&res, "r").1 > col_row(&res, "cf").1);
    assert_eq!(col_row(&res, "v").1, col_row(&res, "cf").1);
}

fn rowed() -> serde_json::Value {
    json!({
        "nodes": [{"id":"s"},{"id":"cf"},{"id":"site"},{"id":"r","kind":"nf"},{"id":"v"},{"id":"w"},{"id":"y"}],
        "edges": [{"from":"s","to":"cf"},{"from":"cf","to":"site"},{"from":"cf","to":"v"},{"from":"w","to":"v"},{"from":"r","to":"cf"},{"from":"v","to":"y"}]
    })
}

#[test]
fn single_data_target_of_the_start_node_shares_its_row() {
    // the story reads left to right: s's only data edge is level with s, even with other row hints around
    let res = run(rowed(), json!([
        {"rel":"leftmost","a":"s","priority":true},
        {"rel":"above","a":"site","b":"cf"},
        {"rel":"below","a":"r","b":"cf"},
        {"rel":"same-row","a":"y","b":"cf"}
    ]));
    assert_eq!(col_row(&res, "s").1, col_row(&res, "cf").1);
    assert_eq!(col_row(&res, "y").1, col_row(&res, "cf").1);
    assert!(res.metrics.ignored_hints.is_empty(), "{:?}", res.metrics.ignored_hints);
}

#[test]
fn start_node_with_two_data_targets_gets_no_row_rule() {
    let g = json!({"nodes":[{"id":"s"},{"id":"a"},{"id":"b"},{"id":"c"}],"edges":[{"from":"s","to":"a"},{"from":"s","to":"b"},{"from":"a","to":"c"}]});
    let res = run(g, json!([{"rel":"leftmost","a":"s","priority":true}]));
    assert!(res.metrics.ignored_hints.is_empty());
    let rows: Vec<i32> = ["a", "b"].iter().map(|id| col_row(&res, id).1).collect();
    assert_ne!(rows[0], rows[1]);
}

#[test]
fn start_node_follows_its_target_onto_a_group_members_row() {
    // s → cf, and cf lines up with p inside G: the whole story s, cf, p stays on one row
    let g = json!({
        "groups": [{"id":"G"}],
        "nodes": [{"id":"s"},{"id":"cf"},{"id":"p","group":"G"},{"id":"q","group":"G"},{"id":"r","group":"G"},{"id":"x"}],
        "edges": [{"from":"s","to":"cf"},{"from":"cf","to":"p"},{"from":"p","to":"q"},{"from":"p","to":"r"},{"from":"cf","to":"x"}]
    });
    for peer in ["p", "q", "r"] {
        let res = run(g.clone(), json!([
            {"rel":"leftmost","a":"s","priority":true},
            {"rel":"same-row","a":"cf","b":peer}
        ]));
        let row = |id: &str| col_row(&res, id).1;
        assert_eq!((row("s"), row("cf")), (row(peer), row(peer)), "story on one row through {peer}");
        assert!(res.metrics.ignored_hints.is_empty(), "{:?}", res.metrics.ignored_hints);
    }
}

#[test]
fn same_row_moves_the_keyed_node_onto_the_other_ones_row() {
    // data is keyed: it joins site's row; site (held above cf) stays where it is
    let g = json!({
        "nodes": [{"id":"s"},{"id":"cf"},{"id":"site"},{"id":"ls"},{"id":"role","kind":"nf"},{"id":"data"}],
        "edges": [{"from":"s","to":"cf"},{"from":"cf","to":"site"},{"from":"cf","to":"data"},{"from":"cf","to":"ls"},
                  {"from":"ls","to":"role","kind":"nf"},{"from":"role","to":"data","kind":"data"}]
    });
    let hints = json!([
        {"rel":"leftmost","a":"s","priority":true},
        {"rel":"above","a":"site","b":"cf"},
        {"rel":"right-of","a":"data","b":"site"},
        {"rel":"same-row","a":"data","b":"site"}
    ]);
    let res = run(g.clone(), hints);
    assert!(res.metrics.ignored_hints.is_empty(), "{:?}", res.metrics.ignored_hints);
    assert_eq!(col_row(&res, "data").1, col_row(&res, "site").1);
    assert!(col_row(&res, "data").0 > col_row(&res, "site").0);
    // and with the hint reversed the other node moves
    let rev = run(g, json!([{"rel":"leftmost","a":"s","priority":true},{"rel":"same-row","a":"site","b":"data"}]));
    assert_eq!(col_row(&rev, "data").1, col_row(&rev, "site").1);
}

fn gravity_graph() -> serde_json::Value {
    // n has only an nf edge from q, a member of block G; in the grid G is tall, so n would sit under it
    json!({
        "groups": [{"id":"G"}],
        "nodes": [{"id":"s"},{"id":"p","group":"G"},{"id":"q","group":"G"},{"id":"r1","group":"G"},{"id":"r2","group":"G"},{"id":"t"},{"id":"n"}],
        "edges": [{"from":"s","to":"p"},{"from":"p","to":"q"},{"from":"p","to":"r1"},{"from":"p","to":"r2"},{"from":"q","to":"t"},{"from":"q","to":"n"}]
    })
}

fn dist(r: &LayoutResult, x: &str, y: &str) -> i32 {
    let (a, b) = (col_row(r, x), col_row(r, y));
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

#[test]
fn gravity_puts_an_nf_only_node_next_to_its_source_member_of_a_group() {
    let nf = json!([{"from":"q","to":"n","kind":"nf"}]);
    let off = layout(&graph(gravity_graph()), &hints(json!({"kinds": nf})), &Default::default()).unwrap();
    let on = layout(&graph(gravity_graph()), &hints(json!({"kinds": nf, "gravity": true})), &Default::default()).unwrap();
    assert!(dist(&on, "n", "q") < dist(&off, "n", "q"), "gravity brings it closer: {} vs {}", dist(&on, "n", "q"), dist(&off, "n", "q"));
    let (q, n) = (col_row(&on, "q"), col_row(&on, "n"));
    assert!(n.0 >= q.0 && n.1 >= q.1, "nf never runs upward or leftward: {n:?} vs {q:?}");
    // outside the group frame, and no free cell at or right of / below q outside every frame is closer
    let in_group = |c: i32, r: i32| on.layout.groups.iter().any(|g| g.bx.col0 <= c && c <= g.bx.col1 && g.bx.row0 <= r && r <= g.bx.row1);
    assert!(!in_group(n.0, n.1), "n must not sit inside a group frame");
    let taken: Vec<(i32, i32)> = on.layout.nodes.iter().map(|x| (x.col, x.row)).collect();
    let closer = (q.0..on.layout.cols).flat_map(|c| (q.1..on.layout.rows).map(move |r| (c, r)))
        .filter(|(c, r)| (c - q.0) + (r - q.1) < dist(&on, "n", "q") && !taken.contains(&(*c, *r)) && !in_group(*c, *r))
        .count();
    assert_eq!(closer, 0, "a closer free cell exists for n at {n:?} (q at {q:?})");
    assert!(on.metrics.ignored_hints.is_empty());
}
