//! Input document → engine graph and hints (sizes from labels, placement from node hints).
use crate::catalog::{group_look, resolve, GroupLook, Look, Theme};
use crate::input::{EdgeKind, Input, LineStyle};
use c43::drawio::text::{text_width, wrap, LABEL_PX, LINE_H, UNIT_PX};
use c43_layout::model::{Hints, InputEdge, InputGraph, InputGroup, InputNode, Kind, KindHint, PlacementHint, Rel, SizeHints};
use indexmap::IndexMap;

/// icon side in px; the node square is larger to hold the label below it
pub const ICON_PX: f64 = 60.0;
/// space above the icon and between icon and label
pub const ICON_TOP_PX: f64 = 6.0;
pub const LABEL_GAP_PX: f64 = 4.0;
const MIN_SIDE: f64 = 5.0;
const MAX_LABEL_LINES: usize = 3;
/// a note's text may be up to this many lines where it must fit a square of at most this side
const NOTE_MAX_SIDE: f64 = 7.0;
/// room the group icon takes left of the group title, in px
pub const GROUP_ICON_PX: f64 = 30.0;

pub struct Composed {
    pub graph: InputGraph,
    pub hints: Hints,
    /// the glyph of each node; a note when it has no service
    pub nodes: IndexMap<String, Look>,
    pub groups: IndexMap<String, &'static GroupLook>,
    pub warnings: Vec<String>,
    /// explicit line style per edge `from->to`
    pub line: IndexMap<(String, String), bool>,
}

impl Composed {
    /// whether an edge is drawn dashed: its explicit style, else when it is nf
    pub fn dashed(&self, from: &str, to: &str, nf: bool) -> bool {
        self.line.get(&(from.to_string(), to.to_string())).copied().unwrap_or(nf)
    }
}

fn label_of<'a>(id: &'a str, label: &'a Option<String>) -> &'a str {
    label.as_deref().unwrap_or(id)
}

/// px a node square must be so that the label fits under the icon, or None for a note (label inside)
fn side_for(label: &str, icon: bool) -> f64 {
    let fits = |side: f64| {
        let w = side * UNIT_PX - 8.0;
        let room = if icon { side * UNIT_PX - ICON_TOP_PX - ICON_PX - LABEL_GAP_PX } else { side * UNIT_PX - 8.0 };
        wrap(label, w, LABEL_PX, false).is_some_and(|ls| (!icon || ls.len() <= MAX_LABEL_LINES) && ls.len() as f64 * LABEL_PX * LINE_H <= room)
    };
    let side = (0..).map(|i| MIN_SIDE + i as f64).take_while(|s| icon || *s <= NOTE_MAX_SIDE).find(|s| fits(*s));
    // notes are bounded so that one long text does not blow up every node
    side.unwrap_or(NOTE_MAX_SIDE)
}

pub fn compose(input: &Input, theme: Theme) -> Result<Composed, String> {
    let mut warnings = vec![];
    let nodes: IndexMap<String, Look> = input
        .nodes
        .iter()
        .map(|n| Ok((n.id.clone(), n.service.as_deref().map_or(Ok(Look::Note), |s| resolve(theme, s).map_err(|e| format!("node {}: {e}", n.id)))?)))
        .collect::<Result<_, String>>()?;
    let groups: IndexMap<String, &'static GroupLook> = input
        .groups
        .iter()
        .map(|g| {
            let look = g.service.as_deref().and_then(|s| {
                let found = group_look(s);
                if found.is_none() {
                    warnings.push(format!("group {}: unknown frame kind {s}, drawn as a plain frame", g.id));
                }
                found
            });
            (g.id.clone(), look.unwrap_or_else(|| group_look("group").unwrap()))
        })
        .collect();
    let kinds: Vec<KindHint> = input
        .edges
        .iter()
        .map(|e| KindHint { from: e.from.clone(), to: e.to.clone(), kind: if matches!(e.kind, Some(EdgeKind::Nf)) { Kind::Nf } else { Kind::Data } })
        .collect();
    let placement: Vec<PlacementHint> = input
        .hints
        .iter()
        .flat_map(|(id, h)| {
            let rel = |rel: Rel, other: &Option<String>| other.clone().map(|b| PlacementHint { rel, a: id.clone(), b: Some(b), priority: false });
            [
                h.start.then(|| PlacementHint { rel: Rel::Leftmost, a: id.clone(), b: None, priority: true }),
                rel(Rel::LeftOf, &h.left_of),
                rel(Rel::RightOf, &h.right_of),
                rel(Rel::Above, &h.above),
                rel(Rel::Below, &h.below),
                rel(Rel::SameRow, &h.same_row),
                rel(Rel::SameCol, &h.same_col),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        })
        .collect();
    let sizes = SizeHints {
        node: input
            .nodes
            .iter()
            .map(|n| {
                let icon = nodes[&n.id] != Look::Note;
                (n.id.clone(), side_for(label_of(&n.id, &n.label), icon))
            })
            .collect(),
        group_title: input
            .groups
            .iter()
            .map(|g| (g.id.clone(), (text_width(label_of(&g.id, &g.label), LABEL_PX, true) + LABEL_PX / 2.0 + GROUP_ICON_PX) / UNIT_PX))
            .collect(),
        title_height: (LABEL_PX * LINE_H / UNIT_PX).ceil(),
    };
    let graph = InputGraph {
        title: input.title.clone(),
        description: input.description.clone(),
        groups: input.groups.iter().map(|g| InputGroup { id: g.id.clone(), label: g.label.clone(), group: g.group.clone() }).collect(),
        nodes: input.nodes.iter().map(|n| InputNode { id: n.id.clone(), label: n.label.clone(), group: n.group.clone() }).collect(),
        edges: input.edges.iter().map(|e| InputEdge { from: e.from.clone(), to: e.to.clone(), label: e.label.clone() }).collect(),
    };
    let line = input
        .edges
        .iter()
        .filter_map(|e| e.style.as_ref().map(|s| ((e.from.clone(), e.to.clone()), matches!(s, LineStyle::Dashed))))
        .collect();
    Ok(Composed { graph, hints: Hints { kinds, placement, sizes, gravity: true }, nodes, groups, warnings, line })
}
