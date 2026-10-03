use crate::check::{less, score};
use crate::model::{Cell, End, Graph, Kind, LayoutResult, PortRef, Side, State, SIDES};
use crate::place::compact;
use crate::ports::assign_ports;
use indexmap::{IndexMap, IndexSet};

/// neighbouring states: adjacent slot swaps, right↔top / right↔bottom data-out exchange, node moves/swaps (if `move_nodes`)
pub fn candidates<'a>(g: &'a Graph, s: &'a State, move_nodes: bool) -> std::boxed::Box<dyn Iterator<Item = State> + 'a> {
    let swaps = g.nodes.iter().map(|n| &n.id).chain(g.groups.iter().map(|x| &x.id)).flat_map(move |id| {
        SIDES.into_iter().flat_map(move |side| {
            let len = s.ports[id].get(side).len();
            (0..len.saturating_sub(1)).map(move |i| {
                let mut ports = s.ports.clone();
                ports.get_mut(id).unwrap().get_mut(side).swap(i, i + 1);
                State { placement: s.placement.clone(), ports }
            })
        })
    });
    let data_outs = move |list: &[PortRef]| -> Vec<PortRef> {
        list.iter().filter(|r| r.end == End::Src && g.edges[r.edge].kind == Kind::Data).copied().collect()
    };
    let exchanges = g.nodes.iter().flat_map(move |n| {
        let sides = &s.ports[&n.id];
        [Side::Top, Side::Bottom].into_iter().flat_map(move |other| {
            data_outs(&sides.right).into_iter().flat_map(move |r| {
                data_outs(sides.get(other)).into_iter().map(move |t| {
                    let mut ports = s.ports.clone();
                    let x = ports.get_mut(&n.id).unwrap();
                    x.right = x.right.iter().map(|q| if *q == r { t } else { *q }).collect();
                    let o = x.get_mut(other);
                    *o = o.iter().map(|q| if *q == t { r } else { *q }).collect();
                    State { placement: s.placement.clone(), ports }
                })
            })
        })
    });
    if !move_nodes {
        return std::boxed::Box::new(swaps.chain(exchanges));
    }
    // data sources (initiators) start the story on the left: a move may never push one rightwards
    let fed: IndexSet<&String> = g.edges.iter().filter(|e| e.kind == Kind::Data).map(|e| &e.to).collect();
    let sources: Vec<String> = g.nodes.iter().map(|n| n.id.clone()).filter(|id| g.data_nodes.contains(id) && !fed.contains(id)).collect();
    let moves = g.nodes.iter().flat_map(move |n| {
        let c = s.placement.cells[&n.id];
        let sources = sources.clone();
        [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().filter_map(move |(dc, dr)| {
            let target = Cell { col: c.col + dc, row: c.row + dr };
            let mut cells: IndexMap<String, Cell> = s.placement.cells.clone();
            for (id, v) in cells.iter_mut() {
                if *id != n.id && *v == target {
                    *v = c;
                }
            }
            cells.insert(n.id.clone(), target);
            let placement = compact(&cells);
            if sources.iter().any(|id| placement.cells[id].col > s.placement.cells[id].col) {
                return None;
            }
            let ports = assign_ports(g, &placement);
            Some(State { placement, ports })
        })
    });
    std::boxed::Box::new(swaps.chain(exchanges).chain(moves))
}

#[derive(Debug, Clone, Copy)]
pub struct RepairOptions {
    /// false: port moves only (grouped layouts keep their blocks)
    pub move_nodes: bool,
    /// evaluations allowed over both phases; the best state so far is kept when it runs out
    pub max_evals: usize,
}

impl Default for RepairOptions {
    fn default() -> Self {
        RepairOptions { move_nodes: true, max_evals: usize::MAX }
    }
}

pub type Evaluate<'a> = dyn Fn(&State) -> LayoutResult + 'a;

/// first-improvement hill-climb over `allowed` candidates, while `go` holds for the current result
#[allow(clippy::too_many_arguments)]
fn climb(
    g: &Graph, start: State, evaluate: &Evaluate, patience: usize, allowed: &dyn Fn(&State, &State) -> bool,
    go: &dyn Fn(&LayoutResult) -> bool, move_nodes: bool, budget: &mut usize,
) -> State {
    if *budget == 0 {
        return start;
    }
    let mut cur = start;
    *budget -= 1;
    let mut res = evaluate(&cur);
    let mut sc = score(&res.metrics);
    let mut round = 0;
    while round < patience && go(&res) {
        let mut next: Option<(State, LayoutResult, [f64; 6])> = None;
        let mut exhausted = false;
        for cand in candidates(g, &cur, move_nodes) {
            if !allowed(&cur, &cand) {
                continue;
            }
            if *budget == 0 {
                exhausted = true;
                break;
            }
            *budget -= 1;
            let r = evaluate(&cand);
            let c = score(&r.metrics);
            if less(&c, &sc) {
                next = Some((cand, r, c));
                break;
            }
        }
        if exhausted {
            return cur;
        }
        let Some((cand, r, c)) = next else { break };
        cur = cand;
        res = r;
        sc = c;
        round += 1;
    }
    cur
}

fn same_columns(a: &State, b: &State) -> bool {
    a.placement.cells.iter().all(|(id, c)| b.placement.cells[id].col == c.col)
}

/// phase 1: any port/node move, only while hard-rule violations (incl. crossings) remain
pub fn untangle(g: &Graph, start: State, evaluate: &Evaluate, patience: usize, move_nodes: bool, budget: &mut usize) -> State {
    climb(g, start, evaluate, patience, &|_, _| true, &|r| !r.metrics.violations.is_empty(), move_nodes, budget)
}

/// phase 2: shorten links (turns, length) with port moves and vertical node moves only — columns carry the story
pub fn shorten(g: &Graph, start: State, evaluate: &Evaluate, patience: usize, move_nodes: bool, budget: &mut usize) -> State {
    climb(g, start, evaluate, patience, &same_columns, &|_| true, move_nodes, budget)
}

pub fn repair(g: &Graph, start: State, evaluate: &Evaluate, patience: usize, opts: RepairOptions) -> State {
    let mut budget = opts.max_evals;
    let s = untangle(g, start, evaluate, patience, opts.move_nodes, &mut budget);
    shorten(g, s, evaluate, patience, opts.move_nodes, &mut budget)
}
