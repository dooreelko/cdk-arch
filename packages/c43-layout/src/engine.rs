use crate::check::check;
use crate::hier::{hier_place, title_cols_for};
use crate::lanes::{borders, lane_plan, title_slots};
use crate::layout::{build_layout, edge_polyline, frame_for, lane_keys};
use crate::model::{Graph, Hints, InputGraph, LayoutResult, SizeHints, State};
use crate::normalize::normalize;
use crate::place::place;
use crate::ports::{assign_ports, side_count};
use crate::repair::{repair, RepairOptions};
use crate::route::route_all;
use crate::skeleton::skeleton;
use crate::tracks::{assign_tracks, improve_tracks, lane_segments};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub patience: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { patience: 40 }
    }
}

/**
 * evaluations the global (port-only) repair of a grouped layout may spend; each is a full build
 * (container: ~1.7 s per evaluation, and 60 evaluations only took crossings from 102 to 97)
 */
const GROUP_EVALS: usize = 20;

/// smallest node side the size hints ask for (at least 1)
pub fn min_side(g: &Graph, sizes: &SizeHints) -> f64 {
    g.nodes.iter().filter_map(|n| sizes.node.get(&n.id)).fold(1.0, |a, b| a.max(*b))
}

/// state → routes → tracks → geometry → metrics; `min_s` = node side the labels need
pub fn build(g: &Graph, s: &State, sizes: &SizeHints, min_s: f64) -> LayoutResult {
    let p = &s.placement;
    let routes = route_all(g, p, &s.ports);
    let segs = lane_segments(g, p, &s.ports, &routes);
    let greedy = assign_tracks(&segs);
    // while tracks are being improved every band may use the greedy range
    let side = min_s.max(side_count(&s.ports, Some(&g.group_ids)) as f64);
    let k = greedy.values().map(|t| t.abs()).fold(0, i32::max);
    let f = frame_for(g, p, side, lane_plan(side, &borders(g, p), Box::new(move |_, _| k), title_slots(sizes), &lane_keys(p)), &segs);
    let edges: Vec<usize> = g.edges.iter().map(|e| e.id).collect();
    let tracks = improve_tracks(&segs, &greedy, &|edge, t| edge_polyline(g, p, &s.ports, &f, &routes[edge], t), &edges);
    let layout = build_layout(g, p, &s.ports, &routes, &tracks, sizes, min_s, &segs);
    let metrics = check(&layout);
    LayoutResult { version: 1, layout, metrics }
}

/// flat path: skeleton → place → repair
pub fn flat_state(g: &Graph, sizes: &SizeHints, min_s: f64, patience: usize) -> State {
    let placement = place(g, &skeleton(g));
    let ports = assign_ports(g, &placement);
    repair(g, State { placement, ports }, &|s| build(g, s, sizes, min_s), patience, RepairOptions::default())
}

pub fn layout(input: &InputGraph, hints: &Hints, opts: &Options) -> Result<LayoutResult, String> {
    let g = normalize(input, hints)?;
    let sizes = &hints.sizes;
    let min_s = min_side(&g, sizes);
    if g.groups.is_empty() {
        return Ok(build(&g, &flat_state(&g, sizes, min_s, opts.patience), sizes, min_s));
    }
    // groups: every level placed by the flat pipeline, then a global port-only repair on a bounded budget
    let placement = hier_place(&g, &|mg| flat_state(mg, sizes, 1.0, opts.patience).placement, &title_cols_for(min_s, sizes));
    let ports = assign_ports(&g, &placement);
    let best = repair(&g, State { placement, ports }, &|s| build(&g, s, sizes, min_s), opts.patience,
        RepairOptions { move_nodes: false, max_evals: GROUP_EVALS });
    Ok(build(&g, &best, sizes, min_s))
}
