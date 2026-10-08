mod common;
use c43_layout::model::{Edge, Kind, Node, RawGraph};
use c43_layout::normalize::{normalize, normalize_internal};
use common::{graph, hints, load_case};
use serde_json::json;

fn raw(v: serde_json::Value) -> RawGraph {
    serde_json::from_value(v).unwrap()
}

fn sorted(xs: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut v: Vec<String> = xs.into_iter().collect();
    v.sort();
    v
}

#[test]
fn defaults_label_is_id_kind_is_data() {
    let g = normalize_internal(&raw(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]}))).unwrap();
    let node = |id: &str| Node { id: id.into(), label: id.into(), kind: Kind::Data, parent: None };
    assert_eq!(g.nodes, vec![node("A"), node("B")]);
    assert_eq!(g.edges, vec![Edge { id: 0, from: "A".into(), to: "B".into(), kind: Kind::Data, label: None }]);
}

#[test]
fn edge_kind_explicit_over_nf_endpoint_over_data() {
    let g = normalize_internal(&raw(json!({
        "nodes": [{"id":"LA"},{"id":"SES"},{"id":"CW","kind":"nf"},{"id":"CF"},{"id":"R53","kind":"nf"},{"id":"X"}],
        "edges": [
            {"from":"LA","to":"SES"},
            {"from":"CW","to":"SES"},
            {"from":"CF","to":"R53"},
            {"from":"CF","to":"X","kind":"nf"},
            {"from":"X","to":"R53","kind":"data"}
        ]
    }))).unwrap();
    use Kind::*;
    assert_eq!(g.edges.iter().map(|e| e.kind).collect::<Vec<_>>(), vec![Data, Nf, Nf, Nf, Data]);
}

#[test]
fn data_nodes_touch_a_data_edge_degree_counts_all_edges() {
    let (g, h) = load_case("aws");
    let g = normalize(&g, &h).unwrap();
    assert_eq!(sorted(g.data_nodes.iter().cloned()), ["AGW", "ATH", "CF", "LA", "LB", "LC", "S3", "SES", "U"]);
    assert_eq!(g.degree["AGW"], 4);
    assert_eq!(g.degree["S3"], 3);
    assert_eq!(g.degree["R53"], 1);
}

#[test]
fn star16_case_shape() {
    let (g, h) = load_case("star16");
    let g = normalize(&g, &h).unwrap();
    assert_eq!(g.nodes.len(), 16);
    assert_eq!(g.edges.len(), 15);
    assert_eq!(g.edges.iter().filter(|e| e.kind == Kind::Nf).count(), 7);
}

#[test]
fn rejects_invalid_input_with_a_clear_message() {
    let err = |v| normalize_internal(&raw(v)).unwrap_err();
    assert!(err(json!({"nodes":[],"edges":[]})).contains("empty graph"));
    assert!(err(json!({"nodes":[{"id":"A"},{"id":"A"}]})).contains("duplicate node id: A"));
    assert!(err(json!({"nodes":[{"id":"A"}],"edges":[{"from":"A","to":"B"}]})).contains("unknown node: B"));
    assert!(err(json!({"nodes":[{"id":"A"}],"edges":[{"from":"A","to":"A"}]})).contains("self-loop not supported: A"));
    assert!(err(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"},{"from":"A","to":"B"}]}))
        .contains("duplicate edge: A->B"));
    assert!(err(json!({"nodes":[{"id":""}]})).contains("node id must be a string"));
}

#[test]
fn optional_title_and_description_carried_through() {
    let (g, h) = load_case("rebob-system");
    let g = normalize(&g, &h).unwrap();
    assert_eq!(g.title.as_deref(), Some("rebob"));
    assert_eq!(g.description.as_deref(), Some("System architecture for the rebob platform"));
    let bare = normalize(&graph(json!({"nodes":[{"id":"A"}]})), &Default::default()).unwrap();
    assert_eq!(bare.title, None);
    assert_eq!(bare.description, None);
}

#[test]
fn groups_parent_map_group_ids_labels_default_to_id() {
    let g = normalize_internal(&raw(json!({
        "groups": [{"id":"G"},{"id":"H","label":"inner","group":"G"}],
        "nodes": [{"id":"A","group":"H"},{"id":"B"}],
        "edges": [{"from":"B","to":"G"},{"from":"G","to":"A"}]
    }))).unwrap();
    assert_eq!(g.groups.iter().map(|x| (x.id.as_str(), x.label.as_str(), x.parent.as_deref())).collect::<Vec<_>>(),
        vec![("G", "G", None), ("H", "inner", Some("G"))]);
    assert_eq!(g.group_ids.iter().cloned().collect::<Vec<_>>(), ["G", "H"]);
    assert_eq!(g.parent["A"], "H");
    assert_eq!(g.parent["H"], "G");
    assert!(!g.parent.contains_key("B"));
    assert_eq!(g.edges[0].kind, Kind::Data);
    assert_eq!(sorted(g.data_nodes.iter().cloned()), ["A", "B"]);
    assert!(!g.degree.contains_key("G"));
}

#[test]
fn group_edge_kind_follows_the_node_endpoint() {
    let g = normalize_internal(&raw(json!({"groups":[{"id":"G"}],"nodes":[{"id":"A","kind":"nf"}],"edges":[{"from":"A","to":"G"}]}))).unwrap();
    assert_eq!(g.edges[0].kind, Kind::Nf);
}

#[test]
fn group_cases_normalize() {
    let (s, h) = load_case("groups");
    assert_eq!(normalize(&s, &h).unwrap().groups.len(), 4);
    let (c, h) = load_case("rebob-container");
    let c = normalize(&c, &h).unwrap();
    assert_eq!(c.title.as_deref(), Some("rebob"));
    assert_eq!(c.groups.len(), 11);
    assert_eq!(c.edges.len(), 30);
    assert_eq!(c.nodes.len(), 23);
    assert_eq!(c.nodes.iter().filter(|n| n.parent.is_none()).count(), 2, "clients sit outside every group");
    assert_eq!(c.edges.iter().filter(|e| c.group_ids.contains(&e.to)).count(), 14, "clients use whole services (groups)");
    assert!(c.edges.iter().all(|e| e.kind == Kind::Data));
    assert_eq!(c.parent["chat/bus"], "backend:chat");
}

#[test]
fn group_validation_errors() {
    let err = |v| normalize_internal(&raw(v)).unwrap_err();
    let n = json!([{"id":"A"}]);
    assert!(err(json!({"groups":[{"id":"A"}],"nodes":n})).contains("duplicate id: A"));
    assert!(err(json!({"groups":[{"id":"G","group":"X"}],"nodes":n})).contains("group G: unknown group X"));
    assert!(err(json!({"nodes":[{"id":"A","group":"X"}]})).contains("node A: unknown group X"));
    assert!(err(json!({"groups":[{"id":"G","group":"A"}],"nodes":n})).contains("group G: unknown group A"));
    assert!(err(json!({"groups":[{"id":"G","group":"H"},{"id":"H","group":"G"}],"nodes":n})).contains("group cycle: G"));
    assert!(err(json!({"groups":[{"id":"G"}],"nodes":[{"id":"A","group":"G"}],"edges":[{"from":"A","to":"G"}]}))
        .contains("edge A->G: a member cannot point at its own group"));
    assert!(err(json!({"groups":[{"id":"G"},{"id":"H","group":"G"}],"nodes":n,"edges":[{"from":"H","to":"G"}]}))
        .contains("edge H->G: a member cannot point at its own group"));
}

#[test]
fn unknown_hint_id_is_an_error() {
    let g = graph(json!({"groups":[{"id":"G"}],"nodes":[{"id":"a"},{"id":"b"}],"edges":[{"from":"a","to":"b"}]}));
    let e = |h| normalize(&g, &hints(h)).unwrap_err();
    assert_eq!(e(json!({"kinds":[{"from":"a","to":"x","kind":"nf"}]})), "hint references unknown edge: a->x");
    assert_eq!(e(json!({"sizes":{"node":{"zz":3}}})), "hint references unknown node: zz");
    assert_eq!(e(json!({"sizes":{"node":{"G":3}}})), "hint references unknown node: G");
    assert_eq!(e(json!({"sizes":{"groupTitle":{"a":3}}})), "hint references unknown group: a");
}

#[test]
fn node_kind_is_derived_from_edges() {
    let g = graph(json!({"nodes":[{"id":"a"},{"id":"b"},{"id":"c"},{"id":"d"}],"edges":[{"from":"a","to":"b"},{"from":"a","to":"c"}]}));
    let h = hints(json!({"kinds":[{"from":"a","to":"c","kind":"nf"}]}));
    let n = normalize(&g, &h).unwrap();
    use Kind::*;
    assert_eq!(n.nodes.iter().map(|n| n.kind).collect::<Vec<_>>(), vec![Data, Data, Nf, Data], "isolated d stays data");
    assert_eq!(n.edges.iter().map(|e| e.kind).collect::<Vec<_>>(), vec![Data, Nf]);
}

#[test]
fn placement_hint_errors() {
    let g = graph(json!({"nodes":[{"id":"a"},{"id":"b"}]}));
    let err = |h: serde_json::Value| normalize(&g, &hints(json!({"placement":[h]}))).unwrap_err();
    assert_eq!(err(json!({"rel":"left-of","a":"a","b":"zz"})), "hint references unknown id: zz");
    assert!(err(json!({"rel":"left-of","a":"a"})).contains("second, different id"));
    assert!(err(json!({"rel":"leftmost","a":"a","b":"b"})).contains("second, different id"));
    let two = hints(json!({"placement":[{"rel":"leftmost","a":"a","priority":true},{"rel":"left-of","a":"b","b":"a","priority":true}]}));
    assert_eq!(normalize(&g, &two).unwrap_err(), "more than one priority placement hint");
}

#[test]
fn edge_label_is_carried() {
    let g = graph(json!({"nodes":[{"id":"a"},{"id":"b"}],"edges":[{"from":"a","to":"b","label":"uses"}]}));
    assert_eq!(normalize(&g, &Default::default()).unwrap().edges[0].label.as_deref(), Some("uses"));
}

#[test]
fn js_round_matches_javascript() {
    assert_eq!([-1.5, -0.5, 0.5, 1.5, 2.5, -2.4].map(c43_layout::js::round), [-1.0, 0.0, 1.0, 2.0, 3.0, -2.0]);
}

#[test]
fn js_num_prints_like_javascript() {
    use c43_layout::js::num;
    assert_eq!([num(3.0), num(-0.0), num(0.5), num(-12.25), num(0.1 + 0.2)], ["3", "0", "0.5", "-12.25", "0.30000000000000004"]);
}
