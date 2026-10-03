use crate::check::{pair_bad, prepare, Prepared};
use crate::geom::{end_point, lane_center, lane_key, Geom};
use crate::groups::owner;
use crate::lanes::{band_of, borders};
use crate::model::{Dir, End, Graph, Placement, Ports, Pt, Route};
use crate::ports::{end_info, side_count};
use indexmap::{IndexMap, IndexSet};

/// (edge, k) → track of the route's k-th lane
pub type Tracks = IndexMap<(usize, usize), i32>;

/**
 * the part of a route lying in one lane, as an interval along that lane (abstract W = 1 geometry);
 * `band`: which part of the lane between group borders it travels in (see lanes.rs)
 */
#[derive(Debug, Clone, PartialEq)]
pub struct Seg {
    pub edge: usize,
    pub k: usize,
    pub lane: String,
    pub band: String,
    pub lo: f64,
    pub hi: f64,
}

impl Seg {
    pub fn key(&self) -> (usize, usize) {
        (self.edge, self.k)
    }
}

pub fn lane_segments(g: &Graph, p: &Placement, ports: &Ports, routes: &[Route]) -> Vec<Seg> {
    let bs = borders(g, p);
    let geo = Geom::uniform(side_count(ports, Some(&g.group_ids)) as f64, 1.0);
    let mut segs: Vec<Seg> = Vec::new();
    for r in routes {
        let l = &r.lanes;
        if l.is_empty() {
            continue;
        }
        let e = &g.edges[r.edge];
        let si = end_info(ports, &e.from, e.id, End::Src);
        let ti = end_info(ports, &e.to, e.id, End::Dst);
        let sp = end_point(&geo, p, &e.from, &si, None);
        let tp = end_point(&geo, p, &e.to, &ti, None);
        for k in 0..l.len() {
            let along = |pt: &Pt| if l[k].dir == Dir::V { pt.y } else { pt.x };
            // a junction end covers the whole width of the crossing lane
            let span = |j: usize| {
                let c = lane_center(&geo, &l[j]);
                [c - 0.5, c + 0.5]
            };
            let mut ends: Vec<f64> = if k == 0 { vec![along(&sp)] } else { span(k - 1).to_vec() };
            ends.extend(if k == l.len() - 1 { vec![along(&tp)] } else { span(k + 1).to_vec() });
            let lo = ends.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            // straight across the lane, no track needed — between nodes only: a group's border ports move once
            // the lane plan places its borders, so a group end always gets a track to jog on
            if l.len() == 1 && hi - lo < 1e-9 && p.cells.contains_key(&e.from) && p.cells.contains_key(&e.to) {
                continue;
            }
            let lane = lane_key(&l[k]);
            let band = band_of(g, bs.get(&lane), &owner(g, &e.from, &e.to));
            segs.push(Seg { edge: r.edge, k, lane, band, lo, hi });
        }
    }
    segs
}

/// 0, -1, 1, -2, 2, … : middle of the lane first
pub fn pref_track(n: usize) -> i32 {
    let n = n as i32;
    if n == 0 { 0 } else if n % 2 == 1 { -(n + 1) / 2 } else { n / 2 }
}

pub fn clash(a: &Seg, b: &Seg) -> bool {
    a.lo <= b.hi + 1e-9 && b.lo <= a.hi + 1e-9
}

pub fn by_lane(segs: &[Seg]) -> IndexMap<String, Vec<Seg>> {
    let mut m: IndexMap<String, Vec<Seg>> = IndexMap::new();
    for s in segs {
        m.entry(format!("{}|{}", s.lane, s.band)).or_default().push(s.clone());
    }
    m
}

pub fn assign_tracks(segs: &[Seg]) -> Tracks {
    let mut tracks: Tracks = IndexMap::new();
    for list in by_lane(segs).values() {
        let mut sorted = list.clone();
        sorted.sort_by(|a, b| a.lo.partial_cmp(&b.lo).unwrap().then(a.hi.partial_cmp(&b.hi).unwrap()).then(a.edge.cmp(&b.edge)));
        let mut placed: Vec<(&Seg, i32)> = Vec::new();
        for s in &sorted {
            for n in 0.. {
                let t = pref_track(n);
                if !placed.iter().any(|(q, qt)| *qt == t && clash(q, s)) {
                    placed.push((s, t));
                    tracks.insert(s.key(), t);
                    break;
                }
            }
        }
    }
    tracks
}

/// lanes with at most this many track assignments are searched exhaustively (3 tracks × 5 segments)
const EXHAUSTIVE_LIMIT: f64 = 243.0;

/// the tracks being improved with each edge's polyline kept up to date
struct Work<'a> {
    t: Tracks,
    polys: IndexMap<usize, Prepared>,
    poly_of: &'a dyn Fn(usize, &Tracks) -> Vec<Pt>,
    edges: &'a [usize],
}

impl Work<'_> {
    fn refresh(&mut self, touched: &[usize]) {
        for e in touched {
            let p = prepare(&(self.poly_of)(*e, &self.t));
            self.polys.insert(*e, p);
        }
    }

    fn bad(&self, a: usize, b: usize) -> usize {
        pair_bad(&self.polys[&a], &self.polys[&b])
    }

    fn cost(&self, touched: &[usize]) -> usize {
        let mut c = 0;
        for e in touched {
            for f in self.edges {
                if f != e && !(touched.contains(f) && f < e) {
                    c += self.bad(*e, *f);
                }
            }
        }
        c
    }

    fn get(&self, s: &Seg) -> i32 {
        self.t[&s.key()]
    }

    fn set_all(&mut self, list: &[Seg], assign: &[i32]) {
        for (s, a) in list.iter().zip(assign) {
            self.t.insert(s.key(), *a);
        }
    }

    /// try `assign` for the lane; keep it only if the lane's edges get strictly better
    fn attempt(&mut self, list: &[Seg], touched: &[usize], assign: &[i32], best: &mut usize) -> bool {
        let before: Vec<i32> = list.iter().map(|s| self.get(s)).collect();
        self.set_all(list, assign);
        self.refresh(touched);
        let c = self.cost(touched);
        if c < *best {
            *best = c;
            return true;
        }
        self.set_all(list, &before);
        self.refresh(touched);
        false
    }
}

/**
 * per lane, pick the track assignment (within the current track range, no clashing pair on one track)
 * that minimises crossings + overlaps of the lane's edges; exhaustive when the lane is small,
 * otherwise single moves/swaps. Repeats until no lane improves.
 */
pub fn improve_tracks(segs: &[Seg], tracks: &Tracks, poly_of: &dyn Fn(usize, &Tracks) -> Vec<Pt>, edges: &[usize]) -> Tracks {
    let t = tracks.clone();
    let k = t.values().map(|x| x.abs()).fold(0, i32::max) as usize;
    let range: Vec<i32> = (0..2 * k + 1).map(pref_track).collect();
    let polys: IndexMap<usize, Prepared> = edges.iter().map(|e| (*e, prepare(&poly_of(*e, &t)))).collect();
    let mut w = Work { t, polys, poly_of, edges };

    let lanes: Vec<Vec<Seg>> = by_lane(segs).into_values().collect();
    let mut improved = true;
    let mut round = 0;
    while improved && round < 20 {
        improved = false;
        for list in &lanes {
            let touched: Vec<usize> = list.iter().map(|s| s.edge).collect::<IndexSet<_>>().into_iter().collect();
            let mut best = w.cost(&touched);
            if best == 0 {
                continue;
            }
            if (range.len() as f64).powi(list.len() as i32) <= EXHAUSTIVE_LIMIT {
                // odometer over all assignments; skip ones that put clashing segments on one track
                let mut idx = vec![0usize; list.len()];
                let mut winner: Option<Vec<i32>> = None;
                let current: Vec<i32> = list.iter().map(|s| w.get(s)).collect();
                loop {
                    let assign: Vec<i32> = idx.iter().map(|i| range[*i]).collect();
                    let valid = list.iter().enumerate().all(|(i, s)| {
                        list.iter().enumerate().all(|(j, q)| j <= i || assign[i] != assign[j] || !clash(s, q))
                    });
                    if valid && w.attempt(list, &touched, &assign, &mut best) {
                        winner = Some(assign);
                        // restore so the odometer keeps comparing against the lane's other options
                        w.set_all(list, &current);
                        w.refresh(&touched);
                    }
                    let mut d = 0;
                    while d < idx.len() {
                        idx[d] += 1;
                        if idx[d] != range.len() {
                            break;
                        }
                        idx[d] = 0;
                        d += 1;
                    }
                    if d == idx.len() {
                        break;
                    }
                }
                if let Some(win) = winner {
                    w.set_all(list, &win);
                    w.refresh(&touched);
                    improved = true;
                }
                continue;
            }
            for s in list {
                for &tt in &range {
                    let cur = w.get(s);
                    if tt == cur {
                        continue;
                    }
                    let blockers: Vec<&Seg> = list.iter().filter(|q| *q != s && w.get(q) == tt && clash(q, s)).collect();
                    if blockers.len() > 1 {
                        continue;
                    }
                    let o = blockers.first().copied();
                    if let Some(o) = o {
                        if list.iter().any(|q| q != s && q != o && w.get(q) == cur && clash(q, o)) {
                            continue;
                        }
                    }
                    let assign: Vec<i32> = list.iter().map(|q| if q == s { tt } else if Some(q) == o { cur } else { w.get(q) }).collect();
                    if w.attempt(list, &touched, &assign, &mut best) {
                        improved = true;
                    }
                }
            }
        }

        // two edges that run side by side through several lanes often untangle only when they
        // swap tracks in every shared lane at once (e.g. nested detours around the same node)
        let lane_of = by_lane(segs);
        let segs_of = |e: usize| segs.iter().filter(move |s| s.edge == e);
        for i in 0..edges.len() {
            for j in i + 1..edges.len() {
                let (e, f) = (edges[i], edges[j]);
                let pairs: Vec<(&Seg, &Seg)> =
                    segs_of(e).flat_map(|a| segs_of(f).filter(move |b| b.lane == a.lane && b.band == a.band).map(move |b| (a, b))).collect();
                if pairs.len() < 2 {
                    continue;
                }
                let touched = [e, f];
                let before = w.cost(&touched);
                if before == 0 {
                    continue;
                }
                let fits = pairs.iter().all(|(a, b)| {
                    let (ta, tb) = (w.get(a), w.get(b));
                    let others = lane_of[&format!("{}|{}", a.lane, a.band)].iter().filter(|q| q != a && q != b);
                    !others.into_iter().any(|q| (w.get(q) == tb && clash(q, a)) || (w.get(q) == ta && clash(q, b)))
                });
                if !fits {
                    continue;
                }
                let swap = |w: &mut Work| {
                    for (a, b) in &pairs {
                        let ta = w.get(a);
                        let tb = w.get(b);
                        w.t.insert(a.key(), tb);
                        w.t.insert(b.key(), ta);
                    }
                };
                swap(&mut w);
                w.refresh(&touched);
                if w.cost(&touched) < before {
                    improved = true;
                    continue;
                }
                swap(&mut w);
                w.refresh(&touched);
            }
        }
        round += 1;
    }
    w.t
}
