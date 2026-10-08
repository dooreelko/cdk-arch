//! The reference architectures: phase1 and phase2 of ghosted on AWS, plus an Azure replica of phase2.
//! `UPDATE_GOLDEN=1 cargo test -p clarc` rewrites the expected drawio files.
use clarc::catalog::Theme;
use clarc::compose::compose;

const CASES: [(&str, Theme, &str); 3] =
    [("phase1", Theme::Aws, "viewer"), ("phase2", Theme::Aws, "viewer"), ("azure-phase2", Theme::Azure, "viewer")];

fn dir(name: &str) -> String {
    format!("{}/tests/cases/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn input(name: &str) -> clarc::input::Input {
    clarc::parse(&std::fs::read_to_string(format!("{}/input.json", dir(name))).unwrap()).unwrap()
}

#[test]
fn golden_drawio() {
    CASES.iter().for_each(|(name, theme, _)| {
        let r = clarc::render(&input(name), *theme).unwrap();
        let path = format!("{}/expected.drawio", dir(name));
        if std::env::var("UPDATE_GOLDEN").is_ok() {
            std::fs::write(&path, &r.xml).unwrap();
        }
        assert_eq!(r.xml, std::fs::read_to_string(&path).unwrap(), "{name}: rendered drawio differs from {path}");
        assert!(r.xml.starts_with("<mxfile") && r.xml.trim_end().ends_with("</mxfile>"), "{name}");
    });
}

#[test]
fn start_node_is_leftmost_and_hints_hold() {
    CASES.iter().for_each(|(name, _, start)| {
        let c = compose(&input(name));
        let r = clarc::lay_out(&c).unwrap();
        let col = |id: &str| r.layout.nodes.iter().find(|n| n.id == id).unwrap().col;
        assert!(r.layout.nodes.iter().all(|n| col(start) <= n.col), "{name}: {start} is not left-most");
        assert!(r.metrics.ignored_hints.is_empty(), "{name}: {:?}", r.metrics.ignored_hints);
        assert!(c.warnings.is_empty(), "{name}: {:?}", c.warnings);
    });
}

#[test]
fn azure_uses_azure_icons_and_aws_uses_aws_icons() {
    let i = input("phase2");
    let aws = clarc::render(&i, Theme::Aws).unwrap().xml;
    let azure = clarc::render(&i, Theme::Azure).unwrap().xml;
    assert!(aws.contains("mxgraph.aws4.resourceIcon") && !aws.contains("azure2"));
    assert!(azure.contains("img/lib/azure2/") && !azure.contains("aws4"));
}

#[test]
fn services_are_provider_neutral_with_native_aliases() {
    use clarc::catalog::service;
    assert_eq!(service("cloudfront").unwrap().key, "cdn");
    assert_eq!(service("front-door").unwrap().key, "cdn");
    assert!(service("nope").is_none());
}

#[test]
fn nf_services_default_their_edges_to_nf_and_notes_are_isolated() {
    let c = compose(&input("phase2"));
    let kind = |from: &str, to: &str| c.hints.kinds.iter().find(|k| k.from == from && k.to == to).unwrap().kind;
    use c43_layout::model::Kind::*;
    assert_eq!(kind("r53", "cf"), Nf);
    assert_eq!(kind("viewer", "cf"), Data);
    assert_eq!(kind("role", "data"), Data, "explicit kind wins");
}

#[test]
fn bad_input_is_an_error() {
    assert!(clarc::parse("{").is_err());
    assert!(clarc::parse(r#"{"nodes":[{"id":"a","bogus":1}]}"#).is_err());
    let i = clarc::parse(r#"{"nodes":[{"id":"a"}],"hints":{"a":{"leftOf":"zz"}}}"#).unwrap();
    assert!(clarc::render(&i, Theme::Aws).unwrap_err().contains("unknown id: zz"));
    let two = clarc::parse(r#"{"nodes":[{"id":"a"},{"id":"b"}],"hints":{"a":{"start":true},"b":{"start":true}}}"#).unwrap();
    assert!(clarc::render(&two, Theme::Aws).unwrap_err().contains("more than one priority"));
}

#[test]
fn unknown_service_is_a_warning_and_a_note() {
    let i = clarc::parse(r#"{"nodes":[{"id":"a","service":"quantum-db"}]}"#).unwrap();
    let r = clarc::render(&i, Theme::Aws).unwrap();
    assert!(r.warnings.iter().any(|w| w.contains("unknown service quantum-db")));
}

/// first number following `key` inside `s`
fn num_after(s: &str, key: &str) -> f64 {
    let rest = &s[s.find(key).unwrap_or_else(|| panic!("{key} not in {s}")) + key.len()..];
    rest.chars().take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-')).collect::<String>().parse().unwrap()
}

fn cell<'a>(xml: &'a str, id: &str) -> &'a str {
    let at = xml.find(&format!("<mxCell id=\"{id}\"")).unwrap();
    &xml[at..at + xml[at..].find("</mxCell>").unwrap()]
}

#[test]
fn edge_ends_sit_on_the_icon_and_a_straight_edge_stays_straight() {
    let i = clarc::parse(r#"{"nodes":[{"id":"a","service":"s3"},{"id":"b","service":"s3"}],"edges":[{"from":"a","to":"b"}]}"#).unwrap();
    let xml = clarc::render(&i, Theme::Aws).unwrap().xml;
    let (n, e) = (cell(&xml, "n-a"), cell(&xml, "e-0"));
    let (y, h) = (num_after(n, "y=\""), num_after(n, "height=\""));
    let end = y + num_after(e, "exitY=") * h + num_after(e, "exitDy=");
    let icon = clarc::compose::ICON_TOP_PX;
    assert!(end >= y + icon && end <= y + icon + clarc::compose::ICON_PX, "port {end} is not on the icon of a node at {y}");
    // every waypoint is on the end's y: the run is moved once with its end, not once per end
    let ys: Vec<f64> = e.match_indices("<mxPoint").map(|(at, _)| num_after(&e[at..], "y=\"")).collect();
    assert!(!ys.is_empty() && ys.iter().all(|v| *v == end), "waypoints {ys:?} vs end {end}");
}

/// the y of an edge end in px: the node's top plus the relative and absolute parts of the attachment
fn end_y(xml: &str, node: &str, edge: &str, exit: bool) -> f64 {
    let (n, e) = (cell(xml, &format!("n-{node}")), cell(xml, edge));
    let (rel, d) = if exit { ("exitY=", "exitDy=") } else { ("entryY=", "entryDy=") };
    num_after(n, "y=\"") + num_after(e, rel) * num_after(n, "height=\"") + num_after(e, d)
}

#[test]
fn single_port_facing_a_busy_port_on_the_same_row_is_level_with_it() {
    // phase1: viewer has one port, cloudfront's left side has two; the HTTPS edge must not bend
    let xml = clarc::render(&input("phase1"), Theme::Aws).unwrap().xml;
    let (out, into) = (end_y(&xml, "viewer", "e-0", true), end_y(&xml, "cf", "e-0", false));
    assert_eq!(out, into, "viewer exits at {out}, cloudfront is entered at {into}");
    let e = cell(&xml, "e-0");
    let ys: Vec<f64> = e.match_indices("<mxPoint").map(|(at, _)| num_after(&e[at..], "y=\"")).collect();
    assert!(ys.iter().all(|y| *y == out), "waypoints {ys:?} bend away from {out}");
}

#[test]
fn single_port_receiving_from_a_busy_port_on_the_same_row_is_level_with_it() {
    // a has two ports on its right (neither in the middle), b one on its left: a→b must not bend
    let i = clarc::parse(
        r#"{"nodes":[{"id":"a","service":"s3"},{"id":"b","service":"s3"},{"id":"c","service":"s3"}],
            "edges":[{"from":"a","to":"b"},{"from":"a","to":"c"}],
            "hints":{"b":{"sameRow":"a"},"c":{"below":"a"}}}"#,
    )
    .unwrap();
    let r = clarc::render(&i, Theme::Aws).unwrap();
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let xml = r.xml;
    let e = cell(&xml, "e-0");
    assert!(e.contains("source=\"n-a\"") && e.contains("target=\"n-b\""), "{e}");
    let (out, into) = (end_y(&xml, "a", "e-0", true), end_y(&xml, "b", "e-0", false));
    assert_eq!(out, into, "a exits at {out}, b is entered at {into}");
    let ys: Vec<f64> = e.match_indices("<mxPoint").map(|(at, _)| num_after(&e[at..], "y=\"")).collect();
    assert!(ys.iter().all(|y| *y == out), "waypoints {ys:?} bend away from {out}");
}

/// the edge cell between two nodes
fn edge_between<'a>(xml: &'a str, from: &str, to: &str) -> &'a str {
    let at = xml.find(&format!("source=\"n-{from}\" target=\"n-{to}\"")).unwrap_or_else(|| panic!("no edge {from}->{to}"));
    let start = xml[..at].rfind("<mxCell id=\"e-").unwrap();
    &xml[start..start + xml[start..].find("</mxCell>").unwrap()]
}

#[test]
fn level_pair_with_nothing_between_is_a_straight_line() {
    // phase1: cloudfront → vpc origin are on one row and their ends are level: no waypoints, no detour
    let xml = clarc::render(&input("phase1"), Theme::Aws).unwrap().xml;
    let e = edge_between(&xml, "cf", "origin");
    assert!(!e.contains("<mxPoint"), "bends: {e}");
    let id = &e[e.find("id=\"").unwrap() + 4..][..e[e.find("id=\"").unwrap() + 4..].find('"').unwrap()];
    assert_eq!(end_y(&xml, "cf", id, true), end_y(&xml, "origin", id, false));
}

#[test]
fn level_pair_with_a_node_between_keeps_the_engine_route() {
    // a → c is a single port facing c's two on one row, but b sits between them: a straight line would cross b
    let i = clarc::parse(
        r#"{"nodes":[{"id":"a","service":"s3"},{"id":"e","service":"s3"},{"id":"b","service":"s3"},{"id":"c","service":"s3"}],
            "edges":[{"from":"e","to":"b"},{"from":"a","to":"c"},{"from":"b","to":"c"}],
            "hints":{"b":{"sameRow":"a"},"c":{"sameRow":"a"}}}"#,
    )
    .unwrap();
    let r = clarc::render(&i, Theme::Aws).unwrap();
    let lay = clarc::lay_out(&compose(&i)).unwrap();
    let pos = |id: &str| lay.layout.nodes.iter().find(|n| n.id == id).map(|n| (n.col, n.row)).unwrap();
    assert!(pos("a").1 == pos("b").1 && pos("b").1 == pos("c").1 && pos("a").0 < pos("b").0 && pos("b").0 < pos("c").0, "premise: b between a and c on one row");
    assert!(edge_between(&r.xml, "a", "c").contains("<mxPoint"), "must route around b");
}
