//! Cloud look for the shared drawio exporter: icons with labels below, themed group frames, labelled edges.
use crate::catalog::{GroupLook, Look, Theme};
use crate::compose::{Composed, GROUP_ICON_PX, ICON_PX, ICON_TOP_PX, LABEL_GAP_PX};
use c43::drawio::export::{esc, vertex, EdgeDraw, Styler};
use c43_layout::js::num;
use c43_layout::model::{Kind, PortOut, Side};
use indexmap::IndexMap;

const FONT: f64 = 12.0;
const EDGE_FONT: f64 = 11.0;
const TEXT_COLOR: &str = "#232F3E";
const NOTE_BORDER: &str = "#7D8998";

/// icon shape style, label left to the separate text cell
fn icon_style(theme: Theme, look: Look) -> String {
    let base = "html=1;outlineConnect=0;gradientColor=none;sketch=0;verticalLabelPosition=bottom;verticalAlign=top;align=center;aspect=fixed;";
    match (theme, look) {
        (_, Look::AwsRes(res, fill)) => format!("{base}shape=mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.{res};fillColor={fill};strokeColor=#ffffff;"),
        (_, Look::AwsShape(shape, color)) => format!("{base}shape=mxgraph.aws4.{shape};fillColor={color};strokeColor=none;"),
        (_, Look::AzureImg(path)) => format!("image;html=1;aspect=fixed;points=[];image=img/lib/azure2/{path}.svg;"),
        (_, Look::Note) => String::new(),
    }
}

fn text_cell(id: &str, value: &str, x: f64, y: f64, w: f64, h: f64, extra: &str) -> String {
    vertex(id, value, &format!("text;html=0;whiteSpace=nowrap;fontSize={};fontColor={TEXT_COLOR};{extra}", num(FONT)), x, y, w, h)
}

fn group_frame(theme: Theme, look: &GroupLook) -> (String, &'static str) {
    match theme {
        Theme::Aws => {
            let icon = look.aws_icon.map_or(String::new(), |i| format!("shape=mxgraph.aws4.group;grIcon={i};"));
            (format!("rounded=0;fillColor=none;strokeColor={};{icon}", look.aws_color), look.aws_color)
        }
        Theme::Azure => (format!("rounded=0;fillColor=none;strokeColor={};dashed=1;", look.azure_color), look.azure_color),
    }
}

/// fraction along a node side where the engine put a port
fn slot(p: &PortOut) -> f64 {
    (2 * p.index + 1) as f64 / (2 * p.count) as f64
}

/// connection offsets that pull an edge end in from the node square to the icon it belongs to;
/// left and right ports sit at `on_icon` of the icon's height, not of the square's (which includes the label)
fn pull(side: Side, own: f64, on_icon: f64, side_px: f64) -> (f64, f64) {
    let gap = (side_px - ICON_PX) / 2.0;
    let dy = ICON_TOP_PX + on_icon * ICON_PX - own * side_px;
    match side {
        Side::Right => (-gap, dy),
        Side::Left => (gap, dy),
        Side::Top => (0.0, ICON_TOP_PX),
        Side::Bottom => (0.0, 0.0),
    }
}

/// `side_px`: the node square the engine laid out
pub fn styler<'a>(theme: Theme, c: &'a Composed, side_px: f64, cells: IndexMap<String, (i32, i32)>) -> Styler<'a> {
    Styler {
        node: Box::new(move |n, ls, u| {
            let look = c.nodes.get(&n.id).copied().unwrap_or(Look::Note);
            let id = format!("n-{}", esc(&n.id));
            let (x, y, w) = (n.x * u, n.y * u, n.size * u);
            let label = ls.iter().map(|l| esc(l)).collect::<Vec<_>>().join("&#xa;");
            if look == Look::Note {
                // the note is the node: bordered, label inside
                let style = format!("rounded=0;html=0;whiteSpace=wrap;align=left;verticalAlign=top;spacing=6;fontSize={};fontColor={TEXT_COLOR};strokeColor={NOTE_BORDER};fillColor=#ffffff;", num(FONT));
                return vec![vertex(&id, &label, &style, x, y, w, w)];
            }
            // invisible square the edges attach to; the icon and the label sit inside it
            let icon_x = x + (w - ICON_PX) / 2.0;
            let label_y = y + ICON_TOP_PX + ICON_PX + LABEL_GAP_PX;
            vec![
                vertex(&id, "", "fillColor=none;strokeColor=none;", x, y, w, w),
                vertex(&format!("i-{}", esc(&n.id)), "", &icon_style(theme, look), icon_x, y + ICON_TOP_PX, ICON_PX, ICON_PX),
                text_cell(&format!("l-{}", esc(&n.id)), &label, x, label_y, w, y + w - label_y, "align=center;verticalAlign=top;spacing=0;"),
            ]
        }),
        group: Box::new(move |g, u| {
            let look = c.groups.get(&g.id).copied().unwrap_or(&crate::catalog::GROUPS[3]);
            let (style, color) = group_frame(theme, look);
            let t = &g.title;
            let indent = if theme == Theme::Aws && look.aws_icon.is_some() { GROUP_ICON_PX } else { 0.0 };
            vec![
                vertex(&format!("g-{}", esc(&g.id)), "", &style, g.x * u, g.y * u, g.w * u, g.h * u),
                text_cell(&format!("gt-{}", esc(&g.id)), &esc(&g.label), t.x * u + indent, t.y * u, t.w * u - indent, t.h * u, &format!("align=left;verticalAlign=top;spacing=0;fontStyle=1;fontColor={color};")),
            ]
        }),
        edge: Box::new(move |e| {
            let dashed = if c.dashed(e.from.as_str(), e.to.as_str(), e.kind == Kind::Nf) { "dashed=1;" } else { "" };
            let icon = |id: &str| c.nodes.get(id).is_some_and(|l| *l != Look::Note);
            // one port facing a busier one on the same row takes the busier port's height, so the edge runs straight
            let same_row = |a: &str, b: &str| cells.get(a).zip(cells.get(b)).is_some_and(|(x, y)| x.1 == y.1);
            let level = icon(&e.from) && icon(&e.to) && same_row(&e.from, &e.to)
                && e.src.side == Side::Right && e.dst.side == Side::Left && (e.src.count == 1) != (e.dst.count == 1);
            // nothing on the row between the two, so a straight line crosses no node
            let clear = level && {
                let (a, b) = (cells[&e.from].0, cells[&e.to].0);
                !cells.iter().any(|(id, c)| *id != e.from && *id != e.to && c.1 == cells[&e.from].1 && c.0 > a.min(b) && c.0 < a.max(b))
            };
            let on_icon = |p: &PortOut, other: &PortOut| if level && p.count == 1 { slot(other) } else { slot(p) };
            let end = |id: &str, p: &PortOut, other: &PortOut| if icon(id) { pull(p.side, slot(p), on_icon(p, other), side_px) } else { (0.0, 0.0) };
            EdgeDraw {
                style: format!("strokeWidth=2;fontSize={};labelBackgroundColor=#ffffff;{dashed}", num(EDGE_FONT)),
                moved: Some((end(&e.from, &e.src, &e.dst), end(&e.to, &e.dst, &e.src))),
                straight: clear,
            }
        }),
        label_font: FONT,
    }
}
