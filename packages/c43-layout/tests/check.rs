use c43_layout::check::{check, pair_crossings, pair_overlaps};
use c43_layout::model::*;

const S: f64 = 2.0;
const W: f64 = 3.0;
const P: f64 = 5.0;

fn pt(x: f64, y: f64) -> Pt {
    Pt { x, y }
}

fn node(id: &str, col: i32, row: i32, kind: Kind) -> LayoutNode {
    LayoutNode { id: id.into(), label: id.into(), kind, col, row, x: col as f64 * P + W, y: row as f64 * P + W, size: S, group: None }
}

fn data(id: &str, col: i32, row: i32) -> LayoutNode {
    node(id, col, row, Kind::Data)
}

fn port(n: &LayoutNode, side: Side, a: f64) -> PortOut {
    let (x, y) = match side {
        Side::Right => (n.x + S, n.y + a),
        Side::Left => (n.x, n.y + a),
        Side::Top => (n.x + a, n.y),
        Side::Bottom => (n.x + a, n.y + S),
    };
    PortOut { side, index: 0, count: 1, inner: false, x, y }
}

fn edge_with(id: usize, a: &LayoutNode, b: &LayoutNode, bs: Side, mid: &[Pt], src: PortOut) -> LayoutEdge {
    let dst = port(b, bs, S / 2.0);
    let points = [vec![pt(src.x, src.y)], mid.to_vec(), vec![pt(dst.x, dst.y)]].concat();
    LayoutEdge { id, from: a.id.clone(), to: b.id.clone(), kind: Kind::Data, label: None, src, dst, points }
}

fn edge(id: usize, a: &LayoutNode, as_: Side, b: &LayoutNode, bs: Side, mid: &[Pt]) -> LayoutEdge {
    edge_with(id, a, b, bs, mid, port(a, as_, S / 2.0))
}

fn layout(nodes: Vec<LayoutNode>, edges: Vec<LayoutEdge>, cols: i32, rows: i32) -> Layout {
    Layout {
        s: S,
        cols,
        rows,
        lanes: Lanes { v: vec![W; cols as usize + 1], h: vec![W; rows as usize + 1] },
        groups: vec![],
        nodes,
        edges,
        frame: Rect { x: 0.0, y: 0.0, w: cols as f64 * P + W, h: rows as f64 * P + W },
        title: None,
        description: None,
    }
}

fn rules(l: &Layout) -> Vec<String> {
    check(l).violations.into_iter().map(|v| v.rule).collect()
}

#[test]
fn node_cell_follows_uneven_lanes() {
    let lay = |x: f64, y: f64| Layout {
        lanes: Lanes { v: vec![5.0, 3.0, 7.0, 5.0], h: vec![5.0, 9.0, 5.0] },
        nodes: vec![LayoutNode { x, y, ..data("a", 2, 1) }],
        ..layout(vec![], vec![], 3, 2)
    };
    // col 2: lanes v0..v2 (5 + 3 + 7) and two cells; row 1: lanes h0..h1 (5 + 9) and one cell
    assert!(!rules(&lay(15.0 + 2.0 * S, 14.0 + S)).contains(&"node-cell".to_string()));
    assert!(rules(&lay(15.0 + 2.0 * S + 1.0, 14.0 + S)).contains(&"node-cell".to_string()));
}

#[test]
fn segment_helpers() {
    assert_eq!(pair_crossings(&[pt(0., 0.), pt(4., 0.)], &[pt(2., -1.), pt(2., 1.)]), 1);
    assert_eq!(pair_crossings(&[pt(0., 0.), pt(4., 0.)], &[pt(0., 1.), pt(4., 1.)]), 0);
    assert_eq!(pair_overlaps(&[pt(0., 0.), pt(0., 5.)], &[pt(0., 3.), pt(0., 8.)]), 1);
    assert_eq!(pair_overlaps(&[pt(0., 0.), pt(0., 5.)], &[pt(1., 0.), pt(1., 5.)]), 0);
}

#[test]
fn clean_layout_has_no_violations() {
    let (a, b) = (data("A", 0, 1), data("B", 1, 1));
    let m = check(&layout(vec![a.clone(), b.clone()], vec![edge(0, &a, Side::Right, &b, Side::Left, &[])], 3, 3));
    assert!(m.violations.is_empty());
    assert_eq!(m.crossings, 0);
    assert_eq!(m.soft.leftward, 0.0);
}

#[test]
fn crossing_is_detected() {
    let (a, b, c, d) = (data("A", 0, 1), data("B", 2, 1), data("C", 1, 0), data("D", 1, 2));
    let es = vec![edge(0, &a, Side::Right, &b, Side::Left, &[]), edge(1, &c, Side::Bottom, &d, Side::Top, &[])];
    let m = check(&layout(vec![a, b, c, d], es, 3, 3));
    assert_eq!(m.crossings, 1);
    assert_eq!(m.violations.iter().map(|v| v.rule.as_str()).collect::<Vec<_>>(), ["crossing"]);
}

#[test]
fn through_node_left_exit_right_entry_corner_port() {
    let (a, x, b) = (data("A", 0, 1), data("X", 1, 1), data("B", 2, 1));
    let e = edge(0, &a, Side::Right, &b, Side::Left, &[]);
    assert!(rules(&layout(vec![a, x, b], vec![e], 3, 3)).contains(&"through-node".into()));
    let (c, d) = (data("C", 0, 1), data("D", 1, 1));
    let e = edge(0, &c, Side::Left, &d, Side::Right, &[pt(1., 9.), pt(1., 7.), pt(11., 7.), pt(11., 9.)]);
    let r = rules(&layout(vec![c.clone(), d.clone()], vec![e], 3, 3));
    assert!(r.contains(&"left-exit".into()));
    assert!(r.contains(&"right-entry".into()));
    let e = edge_with(0, &c, &d, Side::Left, &[], port(&c, Side::Right, 0.0));
    assert!(rules(&layout(vec![c, d], vec![e], 3, 3)).contains(&"corner-port".into()));
}

#[test]
fn soft_scores_leftward_edges_and_nf_placement() {
    let (a, b) = (data("A", 1, 1), node("B", 0, 2, Kind::Nf));
    let e = edge(0, &a, Side::Bottom, &b, Side::Top, &[pt(9., 11.5), pt(4., 11.5)]);
    let m = check(&layout(vec![a, b], vec![e], 2, 3));
    assert_eq!(m.soft.leftward, 1.0);
    assert_eq!(m.soft.nf_bottom_share, 1.0);
    assert_eq!(m.soft.turns, 2.0);
}
