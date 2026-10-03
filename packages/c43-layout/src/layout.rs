use crate::geom::{end_point, lane_key, node_origin, track_pos, Geom};
use crate::groups::depth;
use crate::lanes::{borders, lane_plan, title_slots, Half, LanePlan, Reach};
use crate::model::*;
use crate::ports::{end_info, side_count};
use crate::tracks::{Seg, Tracks};
use indexmap::IndexMap;
use std::rc::Rc;

/// lane width: odd (tracks on half units), at least 3, wide enough for every track
pub fn width_for(tracks: &Tracks) -> f64 {
    3.0f64.max(2.0 * tracks.values().map(|t| t.abs()).fold(0, i32::max) as f64 + 1.0)
}

fn simplify(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for q in pts {
        let n = out.len();
        if let Some(a) = out.last() {
            if (a.x - q.x).abs() < 1e-9 && (a.y - q.y).abs() < 1e-9 {
                continue;
            }
        }
        if n >= 2 {
            let (b, a) = (out[n - 2], out[n - 1]);
            if ((b.x - a.x).abs() < 1e-9 && (a.x - q.x).abs() < 1e-9) || ((b.y - a.y).abs() < 1e-9 && (a.y - q.y).abs() < 1e-9) {
                out.pop();
            }
        }
        out.push(*q);
    }
    out
}

/// port → stub to first track → corners at track intersections → stub → port; `pos`: coordinate of track t of the k-th lane
pub fn polyline(geo: &Geom, r: &Route, tracks: &Tracks, sp: Pt, tp: Pt, pos: Option<&dyn Fn(usize, f64) -> f64>) -> Vec<Pt> {
    let l = &r.lanes;
    if l.is_empty() || (l.len() == 1 && !tracks.contains_key(&(r.edge, 0))) {
        return vec![sp, tp];
    }
    let default = |k: usize, t: f64| track_pos(geo, &l[k], t);
    let pos: &dyn Fn(usize, f64) -> f64 = pos.unwrap_or(&default);
    let c = |k: usize| pos(k, tracks[&(r.edge, k)] as f64);
    let mut pts = vec![sp, if l[0].dir == Dir::V { Pt { x: c(0), y: sp.y } } else { Pt { x: sp.x, y: c(0) } }];
    for k in 1..l.len() {
        pts.push(if l[k - 1].dir == Dir::V { Pt { x: c(k - 1), y: c(k) } } else { Pt { x: c(k), y: c(k - 1) } });
    }
    let last = l.len() - 1;
    pts.push(if l[last].dir == Dir::V { Pt { x: c(last), y: tp.y } } else { Pt { x: tp.x, y: c(last) } });
    pts.push(tp);
    simplify(&pts)
}

pub fn lane_keys(p: &Placement) -> Vec<String> {
    (0..=p.cols).map(|i| format!("v{i}")).chain((0..=p.rows).map(|i| format!("h{i}"))).collect()
}

/// what turns cells, routes and tracks into coordinates: geometry, lane plan, group rectangles, segment bands
pub struct Frame<'a> {
    pub geo: Geom<'a>,
    pub plan: Rc<LanePlan<'a>>,
    pub rects: IndexMap<String, Rect>,
    pub band: IndexMap<(usize, usize), String>,
}

/// final geometry: each lane as wide as the lane plan made it
fn plan_geom<'a>(s: f64, plan: &Rc<LanePlan<'a>>) -> Geom<'a> {
    let (a, b) = (plan.clone(), plan.clone());
    Geom::new(s, std::boxed::Box::new(move |l| a.start(&lane_key(l))), std::boxed::Box::new(move |l| b.width(&lane_key(l))))
}

pub fn frame_for<'a>(g: &Graph, p: &Placement, s: f64, plan: LanePlan<'a>, segs: &[Seg]) -> Frame<'a> {
    let bs = borders(g, p);
    let mut rects: IndexMap<String, Rect> = IndexMap::new();
    for (id, b) in p.groups.iter().flatten() {
        let k = |lane: &str, half: Half| {
            let side = &bs[lane];
            (if half == Half::Low { &side.low } else { &side.high })[id]
        };
        let (l, r, t, d) = (format!("v{}", b.col0), format!("v{}", b.col1 + 1), format!("h{}", b.row0), format!("h{}", b.row1 + 1));
        let x = plan.border(&l, Half::High, k(&l, Half::High));
        let y = plan.border(&t, Half::High, k(&t, Half::High));
        let w = plan.border(&r, Half::Low, k(&r, Half::Low)) - x;
        let h = plan.border(&d, Half::Low, k(&d, Half::Low)) - y;
        rects.insert(id.clone(), Rect { x, y, w, h });
    }
    let plan = Rc::new(plan);
    Frame { geo: plan_geom(s, &plan), plan, rects, band: segs.iter().map(|s| (s.key(), s.band.clone())).collect() }
}

pub fn edge_polyline(g: &Graph, p: &Placement, ports: &Ports, f: &Frame, r: &Route, tracks: &Tracks) -> Vec<Pt> {
    let e = &g.edges[r.edge];
    let si = end_info(ports, &e.from, e.id, End::Src);
    let ti = end_info(ports, &e.to, e.id, End::Dst);
    let rect_of = |id: &str| f.rects[id];
    let sp = end_point(&f.geo, p, &e.from, &si, Some(&rect_of));
    let tp = end_point(&f.geo, p, &e.to, &ti, Some(&rect_of));
    let pos = |k: usize, t: f64| f.plan.pos(&lane_key(&r.lanes[k]), f.band.get(&(r.edge, k)).map_or("m", String::as_str), t);
    polyline(&f.geo, r, tracks, sp, tp, Some(&pos))
}

/// largest |track| per lane band
fn band_reach<'a>(segs: &[Seg], tracks: &Tracks) -> Reach<'a> {
    let mut k: IndexMap<String, i32> = IndexMap::new();
    for s in segs {
        if let Some(t) = tracks.get(&s.key()) {
            let e = k.entry(format!("{}|{}", s.lane, s.band)).or_insert(0);
            *e = (*e).max(t.abs());
        }
    }
    std::boxed::Box::new(move |lane, band| k.get(&format!("{lane}|{band}")).copied().unwrap_or(0))
}

/**
 * node side: at least `min_s` (labels), at least the busiest side's port count;
 * lane widths: each lane fits its own bands, group borders and titles (see lanes.rs)
 */
#[allow(clippy::too_many_arguments)]
pub fn build_layout(g: &Graph, p: &Placement, ports: &Ports, routes: &[Route], tracks: &Tracks, sizes: &SizeHints, min_s: f64, segs: &[Seg]) -> Layout {
    let s = min_s.max(side_count(ports, Some(&g.group_ids)) as f64);
    let slots = title_slots(sizes);
    let plan = lane_plan(s, &borders(g, p), band_reach(segs, tracks), slots, &lane_keys(p));
    let f = frame_for(g, p, s, plan, segs);
    let end = |lane: &str| f.plan.start(lane) + f.plan.width(lane);
    let (grid_w, grid_h) = (end(&format!("v{}", p.cols)), end(&format!("h{}", p.rows)));
    let title_h = slots - 0.5;
    let mut groups: Vec<(usize, usize, &Group)> = g.groups.iter().enumerate().map(|(i, gr)| (depth(g, &gr.id), i, gr)).collect();
    groups.sort_by_key(|(d, i, _)| (*d, *i));
    Layout {
        s,
        cols: p.cols,
        rows: p.rows,
        lanes: Lanes {
            v: (0..=p.cols).map(|i| f.plan.width(&format!("v{i}"))).collect(),
            h: (0..=p.rows).map(|i| f.plan.width(&format!("h{i}"))).collect(),
        },
        frame: Rect { x: 0.0, y: 0.0, w: grid_w, h: grid_h },
        title: g.title.clone(),
        description: g.description.clone(),
        groups: groups
            .into_iter()
            .map(|(d, _, gr)| {
                let r = f.rects[&gr.id];
                // the title sits just inside the top border, one line, in the room the lane plan keeps for it;
                // only as wide as its text, so edges can still enter the group from above beside it
                let tw = (r.w - 1.0).min(sizes.group_title.get(&gr.id).copied().unwrap_or(0.0).ceil());
                LayoutGroup {
                    id: gr.id.clone(),
                    label: gr.label.clone(),
                    parent: gr.parent.clone(),
                    depth: d,
                    bx: p.groups.as_ref().unwrap()[&gr.id],
                    x: r.x,
                    y: r.y,
                    w: r.w,
                    h: r.h,
                    title: TextBlock { text: gr.label.clone(), x: r.x + 0.5, y: r.y + 0.25, w: tw, h: title_h },
                }
            })
            .collect(),
        nodes: g
            .nodes
            .iter()
            .map(|n| {
                let c = p.cells[&n.id];
                let o = node_origin(&f.geo, &c);
                LayoutNode { id: n.id.clone(), label: n.label.clone(), kind: n.kind, col: c.col, row: c.row, x: o.x, y: o.y, size: s, group: n.parent.clone() }
            })
            .collect(),
        edges: g
            .edges
            .iter()
            .map(|e| {
                let si = end_info(ports, &e.from, e.id, End::Src);
                let ti = end_info(ports, &e.to, e.id, End::Dst);
                let points = edge_polyline(g, p, ports, &f, &routes[e.id], tracks);
                let (a, b) = (points[0], *points.last().unwrap());
                LayoutEdge {
                    id: e.id,
                    from: e.from.clone(),
                    to: e.to.clone(),
                    kind: e.kind,
                    label: e.label.clone(),
                    src: PortOut { side: si.side, index: si.index, count: si.count, inner: si.inner, x: a.x, y: a.y },
                    dst: PortOut { side: ti.side, index: ti.index, count: ti.count, inner: ti.inner, x: b.x, y: b.y },
                    points,
                }
            })
            .collect(),
    }
}
