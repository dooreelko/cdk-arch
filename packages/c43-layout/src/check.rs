use crate::geom::slot_offset;
use crate::model::*;
use indexmap::IndexMap;

const EPS: f64 = 1e-6;
type Seg = (Pt, Pt);

pub fn segments(pts: &[Pt]) -> Vec<Seg> {
    pts.windows(2).map(|w| (w[0], w[1])).collect()
}

fn is_h(s: &Seg) -> bool {
    (s.0.y - s.1.y).abs() < EPS
}

fn is_v(s: &Seg) -> bool {
    (s.0.x - s.1.x).abs() < EPS
}

fn span(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}

fn within(v: f64, (lo, hi): (f64, f64)) -> bool {
    v >= lo - EPS && v <= hi + EPS
}

fn meets(h: &Seg, v: &Seg) -> bool {
    within(v.0.x, span(h.0.x, h.1.x)) && within(h.0.y, span(v.0.y, v.1.y))
}

fn collinear(a: &Seg, b: &Seg) -> bool {
    if is_h(a) && is_h(b) && (a.0.y - b.0.y).abs() < EPS {
        let ((a0, a1), (b0, b1)) = (span(a.0.x, a.1.x), span(b.0.x, b.1.x));
        return a0.max(b0) <= a1.min(b1) + EPS;
    }
    if is_v(a) && is_v(b) && (a.0.x - b.0.x).abs() < EPS {
        let ((a0, a1), (b0, b1)) = (span(a.0.y, a.1.y), span(b.0.y, b.1.y));
        return a0.max(b0) <= a1.min(b1) + EPS;
    }
    false
}

fn seg_crossings(sa: &[Seg], sb: &[Seg]) -> usize {
    sa.iter()
        .map(|s| {
            sb.iter()
                .filter(|t| {
                    (is_h(s) && is_v(t) && !is_v(s) && !is_h(t) && meets(s, t))
                        || (is_v(s) && is_h(t) && !is_h(s) && !is_v(t) && meets(t, s))
                })
                .count()
        })
        .sum()
}

fn seg_overlaps(sa: &[Seg], sb: &[Seg]) -> usize {
    sa.iter().map(|s| sb.iter().filter(|t| collinear(s, t)).count()).sum()
}

pub fn pair_crossings(a: &[Pt], b: &[Pt]) -> usize {
    seg_crossings(&segments(a), &segments(b))
}

pub fn pair_overlaps(a: &[Pt], b: &[Pt]) -> usize {
    seg_overlaps(&segments(a), &segments(b))
}

/// a polyline with its segments and bounding box worked out once, for repeated pair checks
#[derive(Debug, Clone)]
pub struct Prepared {
    segs: Vec<Seg>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

pub fn prepare(pts: &[Pt]) -> Prepared {
    let xs = pts.iter().map(|p| p.x);
    let ys = pts.iter().map(|p| p.y);
    Prepared {
        segs: segments(pts),
        x0: xs.clone().fold(f64::INFINITY, f64::min),
        y0: ys.clone().fold(f64::INFINITY, f64::min),
        x1: xs.fold(f64::NEG_INFINITY, f64::max),
        y1: ys.fold(f64::NEG_INFINITY, f64::max),
    }
}

/// crossings + overlaps of two prepared polylines; apart bounding boxes cannot touch
pub fn pair_bad(a: &Prepared, b: &Prepared) -> usize {
    if a.x1 < b.x0 - EPS || b.x1 < a.x0 - EPS || a.y1 < b.y0 - EPS || b.y1 < a.y0 - EPS {
        return 0;
    }
    seg_crossings(&a.segs, &b.segs) + seg_overlaps(&a.segs, &b.segs)
}

fn through_rect(s: &Seg, r: &Rect) -> bool {
    let ((x0, x1), (y0, y1)) = (span(s.0.x, s.1.x), span(s.0.y, s.1.y));
    x1 > r.x + EPS && x0 < r.x + r.w - EPS && y1 > r.y + EPS && y0 < r.y + r.h - EPS
}

fn node_rect(n: &LayoutNode) -> Rect {
    Rect { x: n.x, y: n.y, w: n.size, h: n.size }
}

fn through_node(s: &Seg, n: &LayoutNode) -> bool {
    through_rect(s, &node_rect(n))
}

fn intersects(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w - EPS && b.x < a.x + a.w - EPS && a.y < b.y + b.h - EPS && b.y < a.y + a.h - EPS
}

fn strictly_in(a: &Rect, b: &Rect) -> bool {
    a.x > b.x + EPS && a.x + a.w < b.x + b.w - EPS && a.y > b.y + EPS && a.y + a.h < b.y + b.h - EPS
}

fn outline(r: &Rect) -> [Seg; 4] {
    let a = Pt { x: r.x, y: r.y };
    let b = Pt { x: r.x + r.w, y: r.y };
    let c = Pt { x: r.x + r.w, y: r.y + r.h };
    let d = Pt { x: r.x, y: r.y + r.h };
    [(a, b), (b, c), (c, d), (d, a)]
}

fn rule(v: &mut Vec<Violation>, rule: &str, detail: String) {
    v.push(Violation { rule: rule.into(), detail });
}

/// groups enclose exactly their members, nest or stay apart; edges never run along a border or through a title
fn group_rules(l: &Layout, v: &mut Vec<Violation>) {
    let by_id: IndexMap<&str, &LayoutGroup> = l.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    let holders = |gid: Option<&str>| {
        let mut out: Vec<String> = Vec::new();
        let mut c = gid;
        while let Some(id) = c {
            out.push(id.to_string());
            c = by_id[id].parent.as_deref();
        }
        out
    };
    for n in &l.nodes {
        let bx = node_rect(n);
        let mine = holders(n.group.as_deref());
        for g in &l.groups {
            let bad = if mine.contains(&g.id) { !strictly_in(&bx, &g.rect()) } else { intersects(&bx, &g.rect()) };
            if bad {
                rule(v, "group-member", format!("{} / {}", n.id, g.id));
            }
        }
    }
    for (i, a) in l.groups.iter().enumerate() {
        for b in &l.groups[i + 1..] {
            let ok = if holders(Some(&b.id)).contains(&a.id) {
                strictly_in(&b.rect(), &a.rect())
            } else if holders(Some(&a.id)).contains(&b.id) {
                strictly_in(&a.rect(), &b.rect())
            } else {
                !intersects(&a.rect(), &b.rect())
            };
            if !ok {
                rule(v, "group-overlap", format!("{} / {}", a.id, b.id));
            }
        }
    }
    for e in &l.edges {
        for s in segments(&e.points) {
            for g in &l.groups {
                if outline(&g.rect()).iter().any(|o| collinear(&s, o)) {
                    rule(v, "border-overlap", format!("{} on {}", e.id, g.id));
                }
                let t = &g.title;
                if through_rect(&s, &Rect { x: t.x, y: t.y, w: t.w, h: t.h }) {
                    rule(v, "through-title", format!("{} through {}", e.id, g.id));
                }
            }
        }
    }
}

/// sibling groups touching across one lane whose touching sides differ (IDEA: neighbours share height / width)
fn unequal_neighbours(l: &Layout) -> usize {
    let overlap = |a0: i32, a1: i32, b0: i32, b1: i32| a0 <= b1 && b0 <= a1;
    let mut n = 0;
    for (i, a) in l.groups.iter().enumerate() {
        for b in &l.groups[i + 1..] {
            if a.parent != b.parent {
                continue;
            }
            let (x, y) = (&a.bx, &b.bx);
            if (x.col1 + 1 == y.col0 || y.col1 + 1 == x.col0) && overlap(x.row0, x.row1, y.row0, y.row1) {
                if x.row0 != y.row0 || x.row1 != y.row1 {
                    n += 1;
                }
            } else if (x.row1 + 1 == y.row0 || y.row1 + 1 == x.row0) && overlap(x.col0, x.col1, y.col0, y.col1) && (x.col0 != y.col0 || x.col1 != y.col1) {
                n += 1;
            }
        }
    }
    n
}

fn side_name(s: Side) -> &'static str {
    match s {
        Side::Right => "right",
        Side::Bottom => "bottom",
        Side::Top => "top",
        Side::Left => "left",
    }
}

pub fn check(l: &Layout) -> Metrics {
    let mut v: Vec<Violation> = Vec::new();
    // where a cell starts: every lane up to and including the one before it, plus the cells before it
    let cell_start = |ws: &[f64], i: i32| ws[..=i as usize].iter().sum::<f64>() + i as f64 * l.s;
    let by_id: IndexMap<&str, &LayoutNode> = l.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let group_by_id: IndexMap<&str, &LayoutGroup> = l.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    // a group reads as its first column
    let col_of = |id: &str| by_id.get(id).map_or_else(|| group_by_id[id].bx.col0, |n| n.col);

    for n in &l.nodes {
        if n.size != l.s {
            rule(&mut v, "node-size", n.id.clone());
        }
        if (n.x - cell_start(&l.lanes.v, n.col)).abs() > EPS || (n.y - cell_start(&l.lanes.h, n.row)).abs() > EPS {
            rule(&mut v, "node-cell", n.id.clone());
        }
    }

    // an edge end sits on a node square or on a group's border rectangle
    let boxes: IndexMap<&str, Rect> =
        l.nodes.iter().map(|n| (n.id.as_str(), node_rect(n))).chain(l.groups.iter().map(|g| (g.id.as_str(), g.rect()))).collect();
    let mut seen: IndexMap<String, usize> = IndexMap::new();
    let mut sides: IndexMap<String, Vec<(f64, f64)>> = IndexMap::new();
    for e in &l.edges {
        for (port, id) in [(&e.src, &e.from), (&e.dst, &e.to)] {
            let b = boxes[id.as_str()];
            let key = format!("{:.6},{:.6}", port.x + 0.0, port.y + 0.0);
            match seen.get(&key) {
                Some(other) => rule(&mut v, "shared-port", format!("{} & {}", e.id, other)),
                None => {
                    seen.insert(key, e.id);
                }
            }
            let vertical = matches!(port.side, Side::Left | Side::Right);
            let (a, len) = if vertical { (port.y - b.y, b.h) } else { (port.x - b.x, b.w) };
            if a < EPS || a > len - EPS {
                rule(&mut v, "corner-port", format!("{} on {}", e.id, id));
            }
            sides.entry(format!("{}|{}", id, side_name(port.side))).or_default().push((a, len));
        }
        if e.src.side == Side::Left && !e.src.inner {
            rule(&mut v, "left-exit", e.id.to_string());
        }
        if e.dst.side == Side::Right {
            rule(&mut v, "right-entry", e.id.to_string());
        }
        for s in segments(&e.points) {
            if !is_h(&s) && !is_v(&s) {
                rule(&mut v, "orthogonal", e.id.to_string());
            }
            for n in &l.nodes {
                if through_node(&s, n) {
                    rule(&mut v, "through-node", format!("{} through {}", e.id, n.id));
                }
            }
        }
    }
    for (k, list) in sides.iter_mut() {
        list.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
        let n = list.len();
        if list.iter().enumerate().any(|(i, (a, len))| (a - slot_offset(*len, i, n)).abs() > EPS) {
            rule(&mut v, "uneven-ports", k.clone());
        }
    }

    group_rules(l, &mut v);

    let mut crossings = 0;
    for (i, a) in l.edges.iter().enumerate() {
        for b in &l.edges[i + 1..] {
            let c = pair_crossings(&a.points, &b.points);
            crossings += c;
            for _ in 0..c {
                rule(&mut v, "crossing", format!("{} x {}", a.id, b.id));
            }
            if pair_overlaps(&a.points, &b.points) > 0 {
                rule(&mut v, "overlap", format!("{} = {}", a.id, b.id));
            }
        }
    }

    let mut degree: IndexMap<&str, usize> = l.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    for e in &l.edges {
        for id in [&e.from, &e.to] {
            if let Some(d) = degree.get_mut(id.as_str()) {
                *d += 1;
            }
        }
    }
    let mut by_degree: Vec<&LayoutNode> = l.nodes.iter().collect();
    by_degree.sort_by(|a, b| degree[b.id.as_str()].cmp(&degree[a.id.as_str()]));
    let top = by_degree[0];
    let nf: Vec<&LayoutNode> = l.nodes.iter().filter(|n| n.kind == Kind::Nf).collect();
    let len = |pts: &[Pt]| segments(pts).iter().fold(0.0, |s, (a, b)| s + (a.x - b.x).abs() + (a.y - b.y).abs());
    let (cols, rows) = (l.cols as f64, l.rows as f64);
    Metrics {
        violations: v,
        crossings,
        soft: Soft {
            leftward: l.edges.iter().filter(|e| col_of(&e.to) < col_of(&e.from)).count() as f64,
            centrality: (top.col as f64 - (cols - 1.0) / 2.0).abs() + (top.row as f64 - (rows - 1.0) / 2.0).abs(),
            area: cols * rows,
            aspect: cols.max(rows) / cols.min(rows),
            nf_bottom_share: if nf.is_empty() { 1.0 } else { nf.iter().filter(|n| n.row as f64 >= (rows - 1.0) / 2.0).count() as f64 / nf.len() as f64 },
            length: l.edges.iter().fold(0.0, |s, e| s + len(&e.points)),
            turns: l.edges.iter().fold(0.0, |s, e| s + e.points.len() as f64 - 2.0),
            group_sides: unequal_neighbours(l) as f64,
        },
        ignored_hints: vec![],
    }
}

/// IDEA priority: hard rules, then left-to-right and nf-at-bottom story, then crossings, then turns/length
pub fn score(m: &Metrics) -> [f64; 6] {
    [
        m.violations.iter().filter(|v| v.rule != "crossing").count() as f64,
        m.soft.leftward,
        1.0 - m.soft.nf_bottom_share,
        m.crossings as f64,
        m.soft.turns,
        m.soft.length,
    ]
}

pub fn less(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).find(|(x, y)| x != y).is_some_and(|(x, y)| x < y)
}
