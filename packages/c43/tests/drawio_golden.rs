use c43::drawio::graph::doc_from_json;
use c43::drawio::{render, render_layout};
use c43_layout::{layout, Hints, InputGraph};
use serde_json::json;

fn case(name: &str, f: &str) -> String {
    std::fs::read_to_string(format!("{}/../c43-layout/tests/cases/{name}/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn draw(g: &InputGraph, h: &Hints) -> String {
    let r = layout(g, h, &Default::default()).unwrap();
    render_layout(&r.layout, g.title.as_deref(), g.description.as_deref())
}

fn golden(name: &str) {
    let g: InputGraph = serde_json::from_str(&case(name, "graph.json")).unwrap();
    let h: Hints = serde_json::from_str(&case(name, "hints.json")).unwrap();
    let got = draw(&g, &h);
    let want = case(name, "expected.drawio");
    if got != want {
        let line = got.lines().zip(want.lines()).position(|(a, b)| a != b);
        panic!("{name}: drawio differs from the TS export at line {line:?}:\n{:?}\n{:?}",
            line.map(|i| got.lines().nth(i)), line.map(|i| want.lines().nth(i)));
    }
}

#[test]
fn aws() {
    golden("aws")
}

#[test]
fn star16() {
    golden("star16")
}

#[test]
fn groups() {
    golden("groups")
}

#[test]
fn rebob_system() {
    golden("rebob-system")
}

#[test]
fn rebob_container() {
    golden("rebob-container")
}

fn graph(v: serde_json::Value) -> InputGraph {
    serde_json::from_value(v).unwrap()
}

fn cell_line<'a>(xml: &'a str, id: &str) -> &'a str {
    xml.lines().find(|l| l.contains(&format!("<mxCell id=\"{id}\""))).unwrap()
}

#[test]
fn labels_are_xml_escaped_and_unicode_is_kept() {
    let g = graph(json!({"title": "T & <x>", "nodes": [{"id":"A","label":"R&D <core> \"x\" — Ünï"},{"id":"B"}], "edges": [{"from":"A","to":"B"}]}));
    let xml = draw(&g, &Hints::default());
    let value = cell_line(&xml, "n-A").split("value=\"").nth(1).unwrap().split('"').next().unwrap();
    assert_eq!(value.replace("&#xa;", " "), "R&amp;D &lt;core&gt; &quot;x&quot; — Ünï");
    assert!(!xml.contains("<core>"));
    assert!(cell_line(&xml, "title").contains("value=\"T &amp; &lt;x&gt;\""));
}

#[test]
fn long_title_keeps_the_frame_width_and_gets_room_for_several_lines() {
    let words = (0..30).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
    let base = json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]});
    let with = |t: &str| { let mut v = base.clone(); v["title"] = json!(t); draw(&graph(v), &Hints::default()) };
    let attr = |line: &str, k: &str| -> f64 { line.split(&format!(" {k}=\"")).nth(1).unwrap().split('"').next().unwrap().parse().unwrap() };
    let (short, long) = (with("x"), with(&words));
    assert_eq!(attr(cell_line(&short, "frame"), "width"), attr(cell_line(&long, "frame"), "width"));
    assert!(attr(cell_line(&long, "title"), "height") > attr(cell_line(&short, "title"), "height"));
}

#[test]
fn no_header_cells_without_title_or_description() {
    let xml = draw(&graph(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B"}]})), &Hints::default());
    assert!(!xml.contains("id=\"title\"") && !xml.contains("id=\"description\""));
    assert!(xml.contains("<mxCell id=\"frame\""));
}

#[test]
fn edge_labels_become_edge_values() {
    let g = graph(json!({"nodes":[{"id":"A"},{"id":"B"}],"edges":[{"from":"A","to":"B","label":"uses, reads"}]}));
    assert!(cell_line(&draw(&g, &Hints::default()), "e-0").contains("value=\"uses, reads\""));
}

#[test]
fn empty_document_renders_frame_and_title() {
    let doc = doc_from_json(&json!({"nodes":[{"uid":"system:s","name":"local","type":"system","attributes":{}}],"relations":[]}));
    let r = render(&doc).unwrap();
    assert!(r.xml.starts_with("<mxfile"));
    assert!(cell_line(&r.xml, "title").contains("value=\"local\""));
    assert!(r.xml.contains("<mxCell id=\"frame\""));
    assert_eq!(r.warnings, ["nothing to lay out"]);
}

#[test]
fn render_reports_violations_as_warnings() {
    let doc = doc_from_json(&serde_json::from_str(&std::fs::read_to_string(
        format!("{}/tests/fixtures/rebob-container.c43.json", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap());
    let r = render(&doc).unwrap();
    assert!(r.xml.contains("id=\"n-backend:sub-bob\""));
    assert!(r.warnings.iter().any(|w| w.starts_with("crossing: ")), "{:?}", r.warnings);
}
