//! A layout as a drawio document.
use super::header::Header;
use c43_layout::js::num;
use c43_layout::model::{Kind, Layout, Rect, Side, TextBlock};

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// port position as drawio's relative exit/entry constraint
fn rel(side: Side, index: usize, count: usize) -> (f64, f64) {
    let a = (2 * index + 1) as f64 / (2 * count) as f64;
    match side {
        Side::Right => (1.0, a),
        Side::Left => (0.0, a),
        Side::Top => (a, 0.0),
        Side::Bottom => (a, 1.0),
    }
}

/// engine-broken lines as one plain-text value (drawio newline entity)
fn lines(ls: &[String]) -> String {
    ls.iter().map(|l| esc(l)).collect::<Vec<_>>().join("&#xa;")
}

fn vertex(id: &str, value: &str, style: &str, x: f64, y: f64, w: f64, h: f64) -> String {
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" vertex=\"1\" parent=\"1\"><mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\" /></mxCell>",
        num(x), num(y), num(w), num(h)
    )
}

/// node outline is drawn thicker than edges (default width 1)
const NODE_STROKE: u32 = 2;
/// invisible space around the frame: drawio crops exports to content, which would clip the frame line
const MARGIN_PX: f64 = 20.0;

/// what the renderer adds to the engine's layout: pixel scale, label lines, header band, outer frame, fonts
pub struct Drawing<'a> {
    pub layout: &'a Layout,
    pub unit_px: f64,
    pub node_lines: Vec<Vec<String>>,
    pub header: Option<Header>,
    pub frame: Rect,
    pub label_font: f64,
    pub title_font: f64,
}

pub fn to_drawio(d: &Drawing) -> String {
    let (l, u) = (d.layout, d.unit_px);
    let mut cells: Vec<String> = vec!["<mxCell id=\"0\" />".into(), "<mxCell id=\"1\" parent=\"0\" />".into()];
    let (f, m) = (&d.frame, MARGIN_PX);
    // the margin also paints the white background (the drawio CLI exports SVG transparent otherwise)
    cells.push(vertex("margin", "", "strokeColor=none;fillColor=#ffffff;", f.x * u - m, f.y * u - m, f.w * u + 2.0 * m, f.h * u + 2.0 * m));
    cells.push(vertex("frame", "", "rounded=0;fillColor=none;", f.x * u, f.y * u, f.w * u, f.h * u));
    // header text is wrapped by drawio within the block width
    let text = format!("text;html=0;whiteSpace=wrap;align=left;verticalAlign=top;spacing=0;fontSize={};", num(d.title_font));
    let block = |id: &str, b: &TextBlock, style: &str| vertex(id, &esc(&b.text), style, b.x * u, b.y * u, b.w * u, b.h * u);
    if let Some(t) = d.header.as_ref().and_then(|h| h.title.as_ref()) {
        cells.push(block("title", t, &format!("{text}fontStyle=1;")));
    }
    if let Some(t) = d.header.as_ref().and_then(|h| h.description.as_ref()) {
        cells.push(block("description", t, &text));
    }
    // groups (outer first, behind nodes): unfilled rectangle plus a one-line bold title at label size
    let is_group = |id: &str| l.groups.iter().any(|g| g.id == id);
    let reference = |id: &str| format!("{}-{}", if is_group(id) { 'g' } else { 'n' }, esc(id));
    let title_style = format!("text;html=0;whiteSpace=nowrap;align=left;verticalAlign=top;spacing=0;fontStyle=1;fontSize={};", num(d.label_font));
    for g in &l.groups {
        cells.push(vertex(&format!("g-{}", esc(&g.id)), "", "rounded=0;fillColor=none;", g.x * u, g.y * u, g.w * u, g.h * u));
        let t = &g.title;
        cells.push(vertex(&format!("gt-{}", esc(&g.id)), &esc(&g.label), &title_style, t.x * u, t.y * u, t.w * u, t.h * u));
    }
    for (n, ls) in l.nodes.iter().zip(&d.node_lines) {
        let style = format!(
            "html=0;whiteSpace=nowrap;aspect=fixed;fontSize={};strokeWidth={NODE_STROKE};{}",
            num(d.label_font),
            if n.kind == Kind::Nf { "dashed=1;" } else { "" }
        );
        cells.push(vertex(&format!("n-{}", esc(&n.id)), &lines(ls), &style, n.x * u, n.y * u, n.size * u, n.size * u));
    }
    for e in &l.edges {
        let (ex, ey) = rel(e.src.side, e.src.index, e.src.count);
        let (nx, ny) = rel(e.dst.side, e.dst.index, e.dst.count);
        let style = format!(
            "edgeStyle=none;rounded=0;endArrow=classic;exitX={};exitY={};exitDx=0;exitDy=0;entryX={};entryY={};entryDx=0;entryDy=0;{}",
            num(ex), num(ey), num(nx), num(ny),
            if e.kind == Kind::Nf { "dashed=1;" } else { "" }
        );
        let mids: String = e.points[1..e.points.len() - 1].iter().map(|p| format!("<mxPoint x=\"{}\" y=\"{}\" />", num(p.x * u), num(p.y * u))).collect();
        let value = e.label.as_ref().map_or(String::new(), |t| format!(" value=\"{}\"", esc(t)));
        cells.push(format!(
            "<mxCell id=\"e-{}\"{value} style=\"{style}\" edge=\"1\" parent=\"1\" source=\"{}\" target=\"{}\"><mxGeometry relative=\"1\" as=\"geometry\">{}</mxGeometry></mxCell>",
            e.id,
            reference(&e.from),
            reference(&e.to),
            if mids.is_empty() { String::new() } else { format!("<Array as=\"points\">{mids}</Array>") }
        ));
    }
    format!(
        "<mxfile host=\"c43-router\"><diagram name=\"layout\" id=\"layout\"><mxGraphModel grid=\"1\" gridSize=\"{}\" page=\"0\" background=\"#ffffff\"><root>\n{}\n</root></mxGraphModel></diagram></mxfile>\n",
        num(u),
        cells.join("\n")
    )
}
