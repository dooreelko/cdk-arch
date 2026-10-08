//! A layout as a drawio document.
use super::header::Header;
use c43_layout::js::num;
use c43_layout::model::{Kind, Layout, LayoutEdge, LayoutGroup, LayoutNode, Rect, Side, TextBlock};

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

pub fn vertex(id: &str, value: &str, style: &str, x: f64, y: f64, w: f64, h: f64) -> String {
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" vertex=\"1\" parent=\"1\"><mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\" /></mxCell>",
        num(x), num(y), num(w), num(h)
    )
}

/// node outline is drawn thicker than edges (default width 1)
const NODE_STROKE: u32 = 2;
/// invisible space around the frame: drawio crops exports to content, which would clip the frame line
const MARGIN_PX: f64 = 20.0;

/// an edge's style and, if its ends are moved off the node border, the (dx, dy) of the exit and of the entry
pub struct EdgeDraw {
    pub style: String,
    pub moved: Option<((f64, f64), (f64, f64))>,
    /// draw the edge as one straight line between its ends, without the engine's waypoints
    pub straight: bool,
}

/// how nodes, groups and edges are drawn; the default is plain boxes (what c43 draws)
pub struct Styler<'a> {
    /// cells of one node given its label lines and the unit size in px; the first cell is the node itself and has id `n-<id>`, edges attach to it
    pub node: Box<dyn Fn(&LayoutNode, &[String], f64) -> Vec<String> + 'a>,
    /// cells of one group, its frame first with id `g-<id>`
    pub group: Box<dyn Fn(&LayoutGroup, f64) -> Vec<String> + 'a>,
    /// style of an edge after the routing part, and px offsets of its (exit, entry) attachments from the node's border; given offsets, the engine's end points stay as waypoints
    pub edge: Box<dyn Fn(&LayoutEdge) -> EdgeDraw + 'a>,
    pub label_font: f64,
}

impl Styler<'_> {
    pub fn plain(label_font: f64) -> Styler<'static> {
        Styler {
            node: Box::new(move |n, ls, u| {
                let style = format!(
                    "html=0;whiteSpace=nowrap;aspect=fixed;fontSize={};strokeWidth={NODE_STROKE};{}",
                    num(label_font),
                    if n.kind == Kind::Nf { "dashed=1;" } else { "" }
                );
                vec![vertex(&format!("n-{}", esc(&n.id)), &lines(ls), &style, n.x * u, n.y * u, n.size * u, n.size * u)]
            }),
            group: Box::new(move |g, u| {
                // unfilled rectangle plus a one-line bold title at label size
                let title_style = format!("text;html=0;whiteSpace=nowrap;align=left;verticalAlign=top;spacing=0;fontStyle=1;fontSize={};", num(label_font));
                let t = &g.title;
                vec![
                    vertex(&format!("g-{}", esc(&g.id)), "", "rounded=0;fillColor=none;", g.x * u, g.y * u, g.w * u, g.h * u),
                    vertex(&format!("gt-{}", esc(&g.id)), &esc(&g.label), &title_style, t.x * u, t.y * u, t.w * u, t.h * u),
                ]
            }),
            edge: Box::new(|e| EdgeDraw { style: if e.kind == Kind::Nf { "dashed=1;".into() } else { String::new() }, moved: None, straight: false }),
            label_font,
        }
    }
}

/// what the renderer adds to the engine's layout: pixel scale, label lines, header band, outer frame, fonts
pub struct Drawing<'a> {
    pub layout: &'a Layout,
    pub unit_px: f64,
    pub node_lines: Vec<Vec<String>>,
    pub header: Option<Header>,
    pub frame: Rect,
    pub style: Styler<'a>,
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
    // groups (outer first, behind nodes)
    let is_group = |id: &str| l.groups.iter().any(|g| g.id == id);
    let reference = |id: &str| format!("{}-{}", if is_group(id) { 'g' } else { 'n' }, esc(id));
    cells.extend(l.groups.iter().flat_map(|g| (d.style.group)(g, u)));
    cells.extend(l.nodes.iter().zip(&d.node_lines).flat_map(|(n, ls)| (d.style.node)(n, ls, u)));
    for e in &l.edges {
        let (ex, ey) = rel(e.src.side, e.src.index, e.src.count);
        let (nx, ny) = rel(e.dst.side, e.dst.index, e.dst.count);
        let draw = (d.style.edge)(e);
        let ((xdx, xdy), (ndx, ndy)) = draw.moved.unwrap_or(((0.0, 0.0), (0.0, 0.0)));
        let style = format!(
            "edgeStyle=none;rounded=0;endArrow=classic;exitX={};exitY={};exitDx={};exitDy={};entryX={};entryY={};entryDx={};entryDy={};{}{}",
            num(ex), num(ey), num(xdx), num(xdy), num(nx), num(ny), num(ndx), num(ndy),
            // offsets are only honoured off the perimeter
            if draw.moved.is_some() { "exitPerimeter=0;entryPerimeter=0;" } else { "" },
            draw.style
        );
        let inner = if draw.straight { &e.points[..0] } else if draw.moved.is_some() { &e.points[..] } else { &e.points[1..e.points.len() - 1] };
        // a horizontal run leaving a node end that moved up or down moves with it, so it stays horizontal
        let mut pts: Vec<(f64, f64)> = inner.iter().map(|p| (p.x * u, p.y * u)).collect();
        if draw.moved.is_some() && !pts.is_empty() {
            // length of the horizontal run of equal y leading the sequence
            let run = |it: &mut dyn Iterator<Item = &(f64, f64)>| match it.next() {
                Some(first) => 1 + it.take_while(|p| p.1 == first.1).count(),
                None => 0,
            };
            let n = pts.len();
            let lead = if xdx != 0.0 && xdy != 0.0 { run(&mut pts.iter()) } else { 0 };
            let trail = if ndx != 0.0 && ndy != 0.0 { run(&mut pts.iter().rev()) } else { 0 };
            // a path that is one run is split between its two ends, otherwise the runs are disjoint
            let (lead, trail) = if lead == n || trail == n { (n / 2, n - n / 2) } else { (lead, trail) };
            pts.iter_mut().take(lead).for_each(|p| p.1 += xdy);
            pts.iter_mut().rev().take(trail).for_each(|p| p.1 += ndy);
        }
        let mids: String = pts.iter().map(|(x, y)| format!("<mxPoint x=\"{}\" y=\"{}\" />", num(*x), num(*y))).collect();
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
