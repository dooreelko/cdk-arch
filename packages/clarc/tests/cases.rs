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
    CASES.iter().for_each(|(name, theme, start)| {
        let c = compose(&input(name), *theme).unwrap();
        let r = clarc::lay_out(&c).unwrap();
        let col = |id: &str| r.layout.nodes.iter().find(|n| n.id == id).unwrap().col;
        assert!(r.layout.nodes.iter().all(|n| col(start) <= n.col), "{name}: {start} is not left-most");
        assert!(r.metrics.ignored_hints.is_empty(), "{name}: {:?}", r.metrics.ignored_hints);
        assert!(c.warnings.is_empty(), "{name}: {:?}", c.warnings);
    });
}

#[test]
fn azure_uses_azure_icons_and_aws_uses_aws_icons() {
    let aws = clarc::render(&input("phase2"), Theme::Aws).unwrap().xml;
    let azure = clarc::render(&input("azure-phase2"), Theme::Azure).unwrap().xml;
    assert!(aws.contains("mxgraph.aws4.resourceIcon") && !aws.contains("azure2"));
    assert!(azure.contains("img/lib/azure2/") && !azure.contains("aws4"));
}

#[test]
fn glyph_names_ignore_case_dash_and_underscore() {
    use clarc::catalog::{resolve, Look};
    let r53 = Look::AwsRes("route_53", "#8C4FFF");
    ["route_53", "route53", "Route-53", "ROUTE_53"].iter().for_each(|n| assert_eq!(resolve(Theme::Aws, n).unwrap(), r53, "{n}"));
    assert!(matches!(resolve(Theme::Aws, "role").unwrap(), Look::AwsShape("role", _)), "standalone shapes keep their kind");
    assert_eq!(resolve(Theme::Azure, "compute/Function_Apps").unwrap(), Look::AzureImg("compute/Function_Apps"));
    assert_eq!(resolve(Theme::Azure, "compute/function-apps").unwrap(), Look::AzureImg("compute/Function_Apps"));
    assert_eq!(resolve(Theme::Azure, "Browser").unwrap(), Look::AzureImg("general/Browser"), "a bare azure name that only one category has");
    assert_eq!(resolve(Theme::Aws, "note").unwrap(), Look::Note);
}

#[test]
fn unknown_and_ambiguous_glyph_names_are_errors_with_help() {
    use clarc::catalog::resolve;
    let typo = resolve(Theme::Aws, "lamda").unwrap_err();
    assert!(typo.contains("unknown aws glyph \"lamda\"") && typo.contains("lambda") && typo.contains("github.com"), "{typo}");
    assert!(resolve(Theme::Aws, "cloudwatch").unwrap_err().contains("cloudwatch_2"), "the old stencil name is not a glyph name");
    let amb = resolve(Theme::Azure, "App_Services").unwrap_err();
    assert!(amb.contains("ambiguous") && amb.contains("compute/App_Services") && amb.contains("app_services/App_Services"), "{amb}");
    assert!(resolve(Theme::Azure, "nope/Nothing").unwrap_err().contains("unknown azure glyph"));
    // an aws name is not an azure glyph and vice versa
    assert!(resolve(Theme::Azure, "lambda").is_err() && resolve(Theme::Aws, "compute/Function_Apps").is_err());
}

#[test]
fn edges_are_data_unless_marked_nf_so_one_service_has_both() {
    let i = clarc::parse(
        r#"{"nodes":[{"id":"f","service":"fargate"},{"id":"b","service":"s3"},{"id":"w","service":"cloudwatch_2"}],
            "edges":[{"from":"f","to":"b"},{"from":"f","to":"w","kind":"nf"},{"from":"w","to":"b"}]}"#,
    )
    .unwrap();
    let c = compose(&i, Theme::Aws).unwrap();
    use c43_layout::model::Kind::*;
    let kinds: Vec<_> = c.hints.kinds.iter().map(|k| (k.from.as_str(), k.to.as_str(), k.kind)).collect();
    assert_eq!(kinds, vec![("f", "b", Data), ("f", "w", Nf), ("w", "b", Data)], "a monitoring service does not make its edges nf by itself");
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
fn unknown_service_fails_the_render_naming_the_node() {
    let i = clarc::parse(r#"{"nodes":[{"id":"a","service":"quantum-db"}]}"#).unwrap();
    let e = clarc::render(&i, Theme::Aws).unwrap_err();
    assert!(e.contains("node a: unknown aws glyph \"quantum-db\""), "{e}");
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
    let lay = clarc::lay_out(&compose(&i, Theme::Aws).unwrap()).unwrap();
    let pos = |id: &str| lay.layout.nodes.iter().find(|n| n.id == id).map(|n| (n.col, n.row)).unwrap();
    assert!(pos("a").1 == pos("b").1 && pos("b").1 == pos("c").1 && pos("a").0 < pos("b").0 && pos("b").0 < pos("c").0, "premise: b between a and c on one row");
    assert!(edge_between(&r.xml, "a", "c").contains("<mxPoint"), "must route around b");
}

const THEMES: [Theme; 2] = [Theme::Aws, Theme::Azure];

#[test]
fn example_json_is_a_small_valid_input_that_renders_cleanly_in_each_theme() {
    THEMES.iter().for_each(|t| {
        let i = clarc::parse(&clarc::example::example_json(*t)).unwrap();
        assert!(i.title.is_some() && i.description.is_some());
        assert!((2..=4).contains(&i.nodes.len()) && (2..=3).contains(&i.edges.len()) && i.hints.len() == 2);
        assert!(!i.groups.is_empty() && i.nodes.iter().any(|n| n.group.is_some()), "example shows a group with a member");
        let r = clarc::render(&i, *t).unwrap();
        assert!(r.warnings.is_empty(), "{t}: {:?}", r.warnings);
    });
}

#[test]
fn example_text_lists_every_hint_directive_the_input_accepts() {
    // serde names all valid keys when it meets an unknown one
    let err = clarc::parse(r#"{"nodes":[],"hints":{"a":{"bogus":1}}}"#).unwrap_err();
    let listed = clarc::example::HINT_DIRECTIVES.iter().map(|(n, _)| *n).collect::<Vec<_>>();
    assert!(listed.iter().all(|n| err.contains(&format!("`{n}`"))), "listed but not accepted: {err}");
    assert!(err.matches('`').count() / 2 - 1 == listed.len(), "accepted but not listed: {err}");
    THEMES.iter().for_each(|t| {
        let text = clarc::example::example_text(*t);
        assert!(text.starts_with(&clarc::example::example_json(*t)) && listed.iter().all(|n| text.contains(&format!("  {n}: "))));
    });
}

#[test]
fn example_text_names_real_common_glyphs_a_link_to_all_and_the_group_frames() {
    use clarc::catalog;
    THEMES.iter().for_each(|t| {
        let text = clarc::example::example_text(*t);
        assert!(catalog::common(*t).len() >= 10);
        catalog::common(*t).iter().for_each(|(n, p)| {
            assert!(catalog::resolve(*t, n).is_ok() && !p.is_empty(), "{t}: {n}");
            assert!(text.contains(&format!("  {n}: {p}")), "{t}: {n} not listed");
        });
        THEMES.iter().for_each(|l| assert!(text.contains(catalog::glyph_list_url(*l)), "{t}: no link to the {l} list"));
        assert!(text.lines().count() < 80, "the full glyph list must not be dumped");
    });
    catalog::GROUPS.iter().for_each(|g| assert!(clarc::example::GROUP_PURPOSES.iter().any(|(k, p)| *k == g.key && !p.is_empty()), "no purpose for group {}", g.key));
    assert_eq!(clarc::example::GROUP_PURPOSES.len(), catalog::GROUPS.len());
    // azure help speaks azure names and aws help speaks aws names
    assert!(clarc::example::example_text(Theme::Azure).contains("compute/Function_Apps") && !clarc::example::example_text(Theme::Aws).contains("compute/Function_Apps"));
}

fn run(args: &[&str], stdin: &str) -> std::process::Output {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_clarc"))
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn file_dash_reads_stdin_and_no_file_is_an_error() {
    let text = std::fs::read_to_string(format!("{}/input.json", dir("phase2"))).unwrap();
    let ok = run(&["--file", "-"], &text);
    assert!(ok.status.success() && String::from_utf8_lossy(&ok.stdout).starts_with("<mxfile"));
    let none = run(&[], &text);
    assert!(!none.status.success() && none.stdout.is_empty());
    assert!(String::from_utf8_lossy(&none.stderr).contains("--file"), "{}", String::from_utf8_lossy(&none.stderr));
}

#[test]
fn agentic_help_prints_the_example_for_the_theme_and_needs_no_input() {
    let out = run(&["--agentic-help"], "");
    assert!(out.status.success());
    let help = run(&["--help"], "");
    let (help, got) = (String::from_utf8_lossy(&help.stdout).to_string(), String::from_utf8_lossy(&out.stdout).to_string());
    assert!(help.contains("--agentic-help") && got.starts_with(help.trim_end()), "the output starts with the contents of --help");
    assert!(got.ends_with(&clarc::example::example_text(Theme::Aws)));
    let az = run(&["--azure", "--agentic-help"], "");
    assert!(String::from_utf8_lossy(&az.stdout).ends_with(&clarc::example::example_text(Theme::Azure)));
}
