use crate::geom::{attach_lane_box, box_of, cell_key, end_point, lane_center, lane_key, port_point, EndInfo, Geom};
use crate::groups::{chain, owner};
use crate::model::{Box as GBox, Cell, Dir, End, Graph, Kind, Lane, Placement, Ports, Pt, Route, Side};
use crate::ports::{end_info, side_count};
use crate::tracks::lane_segments;
use indexmap::{IndexMap, IndexSet};

const TURN: f64 = 10_000.0;
/// a crossing costs more than any number of turns: IDEA ranks "no crossings" above "fewest turns"
const CROSS: f64 = 1_000_000.0;
const RIP_UP_ROUNDS: usize = 3;
const EPS: f64 = 1e-9;

fn along(pt: &Pt, lane: &Lane) -> f64 {
    if lane.dir == Dir::V { pt.y } else { pt.x }
}

/// what other edges already occupy: lane intervals, and points where straight shots cut across a lane
#[derive(Default)]
struct Obstacles {
    segs: IndexMap<String, Vec<(f64, f64)>>,
    crossers: IndexMap<String, Vec<f64>>,
}

/// route every edge on its own, then re-route each one seeing all the others, until nothing changes
pub fn route_all(g: &Graph, p: &Placement, ports: &Ports) -> Vec<Route> {
    let geo = Geom::uniform(side_count(ports, Some(&g.group_ids)) as f64, 1.0);
    let taken: IndexSet<String> = p.cells.values().map(cell_key).collect();
    let empty = IndexMap::new();
    let boxes = p.groups.as_ref().unwrap_or(&empty);
    let one = |edge: usize, obs: &Obstacles| -> Route {
        let e = &g.edges[edge];
        let si = end_info(ports, &e.from, e.id, End::Src);
        let ti = end_info(ports, &e.to, e.id, End::Dst);
        // stay inside the group holding both ends; avoid the inside of groups holding neither
        let own = owner(g, &e.from, &e.to);
        let ends: IndexSet<String> = chain(g, &e.from).into_iter().chain(chain(g, &e.to)).collect();
        let foreign: Vec<GBox> = boxes.iter().filter(|(id, _)| !ends.contains(*id)).map(|(_, b)| *b).collect();
        let region = if own.is_empty() { None } else { boxes.get(&own).copied() };
        Route { edge, lanes: route_edge(p, &taken, &geo, &e.from, &si, &e.to, &ti, obs, region, &foreign) }
    };
    // seed: data edges first (the story), nf edges after, each seeing what is already drawn
    let mut routes: Vec<Route> = g.edges.iter().map(|e| Route { edge: e.id, lanes: vec![] }).collect();
    let mut drawn: Vec<Route> = Vec::new();
    let mut order: Vec<_> = g.edges.iter().collect();
    order.sort_by_key(|e| (e.kind == Kind::Nf, e.id));
    for e in order {
        let obs = if drawn.is_empty() { Obstacles::default() } else { obstacles(g, p, ports, &geo, &drawn) };
        routes[e.id] = one(e.id, &obs);
        drawn.push(routes[e.id].clone());
    }
    for _ in 0..RIP_UP_ROUNDS {
        let mut changed = false;
        for e in &g.edges {
            let others: Vec<Route> = routes.iter().filter(|r| r.edge != e.id).cloned().collect();
            let next = one(e.id, &obstacles(g, p, ports, &geo, &others));
            if next.lanes != routes[e.id].lanes {
                changed = true;
            }
            routes[e.id] = next;
        }
        if !changed {
            break;
        }
    }
    routes
}

fn obstacles(g: &Graph, p: &Placement, ports: &Ports, geo: &Geom, others: &[Route]) -> Obstacles {
    let mut segs: IndexMap<String, Vec<(f64, f64)>> = IndexMap::new();
    for s in lane_segments(g, p, ports, others) {
        segs.entry(s.lane).or_default().push((s.lo, s.hi));
    }
    let mut crossers: IndexMap<String, Vec<f64>> = IndexMap::new();
    let mut cross = |lane: Lane, at: f64| crossers.entry(lane_key(&lane)).or_default().push(at);
    for r in others {
        if !r.lanes.is_empty() {
            continue;
        }
        let e = &g.edges[r.edge];
        // straight shots only join two nodes
        let (Some(sc), Some(tc)) = (p.cells.get(&e.from), p.cells.get(&e.to)) else { continue };
        let si = end_info(ports, &e.from, e.id, End::Src);
        let sp = port_point(geo, sc, si.side, si.index, si.count);
        if sc.row == tc.row {
            (sc.col + 1..=tc.col).for_each(|i| cross(Lane { dir: Dir::V, i }, sp.y));
        } else {
            (sc.row + 1..=tc.row).for_each(|j| cross(Lane { dir: Dir::H, i: j }, sp.x));
        }
    }
    Obstacles { segs, crossers }
}

/// facing ports on one line with only empty cells between them
fn is_direct(taken: &IndexSet<String>, sp: &Pt, tp: &Pt, sc: &Cell, si: &EndInfo, tc: &Cell, ti: &EndInfo) -> bool {
    if si.side == Side::Right && ti.side == Side::Left && sc.row == tc.row && tc.col > sc.col && (sp.y - tp.y).abs() < EPS {
        return !(sc.col + 1..tc.col).any(|c| taken.contains(&cell_key(&Cell { col: c, row: sc.row })));
    }
    if si.side == Side::Bottom && ti.side == Side::Top && sc.col == tc.col && tc.row > sc.row && (sp.x - tp.x).abs() < EPS {
        return !(sc.row + 1..tc.row).any(|r| taken.contains(&cell_key(&Cell { col: sc.col, row: r })));
    }
    false
}

struct Step {
    to: usize,
    len: f64,
    lane: String,
    dir: i8,
}

/// `region`: the owner group's block (junctions outside it are unusable); `foreign`: blocks whose inside costs a crossing per step
#[allow(clippy::too_many_arguments)]
fn route_edge(
    p: &Placement, taken: &IndexSet<String>, geo: &Geom, from: &str, si: &EndInfo, to: &str, ti: &EndInfo, obs: &Obstacles,
    region: Option<GBox>, foreign: &[GBox],
) -> Vec<Lane> {
    let sp = end_point(geo, p, from, si, None);
    let tp = end_point(geo, p, to, ti, None);
    if let (Some(sc), Some(tc)) = (p.cells.get(from), p.cells.get(to)) {
        if is_direct(taken, &sp, &tp, sc, si, tc, ti) {
            return vec![];
        }
    }

    let s_lane = attach_lane_box(&box_of(p, from), si.side);
    let t_lane = attach_lane_box(&box_of(p, to), ti.side);
    let rows1 = (p.rows + 1) as usize;
    let jid = |i: i32, j: i32| i as usize * rows1 + j as usize;
    let s = (p.cols + 1) as usize * rows1;
    let t = s + 1;
    let usable = |v: usize| -> bool {
        let Some(region) = region else { return true };
        if v >= s {
            return true;
        }
        let (i, j) = ((v / rows1) as i32, (v % rows1) as i32);
        i >= region.col0 && i <= region.col1 + 1 && j >= region.row0 && j <= region.row1 + 1
    };
    let v_c = |i: i32| lane_center(geo, &Lane { dir: Dir::V, i });
    let h_c = |j: i32| lane_center(geo, &Lane { dir: Dir::H, i: j });
    // steps strictly inside a group holding neither end
    let intrusions = |lane: &Lane, mid: f64| -> usize {
        foreign
            .iter()
            .filter(|f| {
                if lane.dir == Dir::V {
                    f.col0 < lane.i && lane.i <= f.col1 && mid > h_c(f.row0) && mid < h_c(f.row1 + 1)
                } else {
                    f.row0 < lane.i && lane.i <= f.row1 && mid > v_c(f.col0) && mid < v_c(f.col1 + 1)
                }
            })
            .count()
    };

    // every lane with the vertices on it, keyed by position along the lane
    let mut lanes: IndexMap<String, (Lane, Vec<(usize, f64)>)> = IndexMap::new();
    for i in 0..=p.cols {
        let lane = Lane { dir: Dir::V, i };
        lanes.insert(lane_key(&lane), (lane, (0..=p.rows).map(|j| (jid(i, j), h_c(j))).collect()));
    }
    for j in 0..=p.rows {
        let lane = Lane { dir: Dir::H, i: j };
        lanes.insert(lane_key(&lane), (lane, (0..=p.cols).map(|i| (jid(i, j), v_c(i))).collect()));
    }
    lanes.get_mut(&lane_key(&s_lane)).unwrap().1.push((s, along(&sp, &s_lane)));
    lanes.get_mut(&lane_key(&t_lane)).unwrap().1.push((t, along(&tp, &t_lane)));

    // a lane step costs its length plus a crossing for every straight shot cutting across it on the way
    let mut adj: IndexMap<usize, Vec<Step>> = IndexMap::new();
    for (key, (lane, pts)) in lanes.iter_mut() {
        pts.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap().then(x.0.cmp(&y.0)));
        let cuts = obs.crossers.get(key).map_or(&[][..], Vec::as_slice);
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if !usable(a.0) || !usable(b.0) {
                continue;
            }
            let n_cuts = cuts.iter().filter(|c| **c > a.1 + EPS && **c < b.1 - EPS).count();
            let len = b.1 - a.1 + CROSS * (n_cuts + intrusions(lane, (a.1 + b.1) / 2.0)) as f64;
            adj.entry(a.0).or_default().push(Step { to: b.0, len, lane: key.clone(), dir: 1 });
            adj.entry(b.0).or_default().push(Step { to: a.0, len, lane: key.clone(), dir: -1 });
        }
    }

    // going straight through a junction crosses every other edge whose segment spans the crossing lane there
    let junction_crossings = |v: usize, lane: &str| -> usize {
        if v >= s {
            return 0;
        }
        let (i, j) = ((v / rows1) as i32, (v % rows1) as i32);
        let (perp, pos) = if lane.starts_with('v') { (format!("h{j}"), v_c(i)) } else { (format!("v{i}"), h_c(j)) };
        obs.segs.get(&perp).map_or(0, |ss| ss.iter().filter(|(lo, hi)| *lo < pos - 0.5 - EPS && *hi > pos + 0.5 + EPS).count())
    };

    // Dijkstra over (vertex, lane of arrival, direction along it); no U-turns within a lane;
    // ties resolved by insertion order
    type Key = (usize, String, i8);
    let start: Key = (s, lane_key(&s_lane), 0);
    let goal_lane = lane_key(&t_lane);
    let mut dist: IndexMap<Key, f64> = IndexMap::new();
    dist.insert(start.clone(), 0.0);
    let mut prev: IndexMap<Key, Key> = IndexMap::new();
    let mut open: IndexSet<Key> = IndexSet::new();
    open.insert(start);
    let mut done: IndexSet<Key> = IndexSet::new();
    while !open.is_empty() {
        let mut best: Option<&Key> = None;
        for k in &open {
            if best.is_none_or(|b| dist[k] < dist[b]) {
                best = Some(k);
            }
        }
        let best = best.unwrap().clone();
        open.shift_remove(&best);
        done.insert(best.clone());
        if best.0 == t && best.1 == goal_lane {
            let mut keys: Vec<String> = Vec::new();
            let mut k = Some(&best);
            while let Some(x) = k {
                keys.push(x.1.clone());
                k = prev.get(x);
            }
            keys.reverse();
            keys.dedup();
            return keys.iter().map(|k| lanes[k].0).collect();
        }
        let (v, lk, d) = (best.0, best.1.clone(), best.2);
        let here = dist[&best];
        for nb in adj.get(&v).map_or(&[][..], Vec::as_slice) {
            if nb.lane == lk && nb.dir == -d {
                continue;
            }
            let nk: Key = (nb.to, nb.lane.clone(), nb.dir);
            if done.contains(&nk) {
                continue;
            }
            let cost = here + nb.len + if nb.lane == lk { CROSS * junction_crossings(v, &lk) as f64 } else { TURN };
            if dist.get(&nk).is_none_or(|old| cost < *old) {
                dist.insert(nk.clone(), cost);
                prev.insert(nk.clone(), best.clone());
                open.insert(nk);
            }
        }
    }
    panic!("no route found");
}
