use crate::model::{Box as GBox, Cell, Dir, Lane, Placement, Pt, Rect, Side};

/// S = node side; each lane's start and width along its axis, all in grid units
pub struct Geom<'a> {
    pub s: f64,
    start: std::boxed::Box<dyn Fn(&Lane) -> f64 + 'a>,
    width: std::boxed::Box<dyn Fn(&Lane) -> f64 + 'a>,
}

impl<'a> Geom<'a> {
    pub fn new(s: f64, start: std::boxed::Box<dyn Fn(&Lane) -> f64 + 'a>, width: std::boxed::Box<dyn Fn(&Lane) -> f64 + 'a>) -> Self {
        Geom { s, start, width }
    }

    /// every lane W wide (the abstract routing geometry uses W = 1)
    pub fn uniform(s: f64, w: f64) -> Geom<'static> {
        Geom::new(s, std::boxed::Box::new(move |l| l.i as f64 * (s + w)), std::boxed::Box::new(move |_| w))
    }

    pub fn start(&self, l: &Lane) -> f64 {
        (self.start)(l)
    }

    pub fn width(&self, l: &Lane) -> f64 {
        (self.width)(l)
    }
}

/// a cell starts right after the lane left of / above it
pub fn node_origin(g: &Geom, c: &Cell) -> Pt {
    let v = Lane { dir: Dir::V, i: c.col };
    let h = Lane { dir: Dir::H, i: c.row };
    Pt { x: g.start(&v) + g.width(&v), y: g.start(&h) + g.width(&h) }
}

/// slot `index` of `count` on a side of length S: evenly spread, never on a corner
pub fn slot_offset(s: f64, index: usize, count: usize) -> f64 {
    (s * (2 * index + 1) as f64) / (2 * count) as f64
}

pub fn port_point(g: &Geom, c: &Cell, side: Side, index: usize, count: usize) -> Pt {
    let o = node_origin(g, c);
    let a = slot_offset(g.s, index, count);
    match side {
        Side::Right => Pt { x: o.x + g.s, y: o.y + a },
        Side::Left => Pt { x: o.x, y: o.y + a },
        Side::Top => Pt { x: o.x + a, y: o.y },
        Side::Bottom => Pt { x: o.x + a, y: o.y + g.s },
    }
}

pub fn cell_box(c: &Cell) -> GBox {
    GBox { col0: c.col, row0: c.row, col1: c.col, row1: c.row }
}

/// a node's cell as a box, or a group's block
pub fn box_of(p: &Placement, id: &str) -> GBox {
    match p.cells.get(id) {
        Some(c) => cell_box(c),
        None => p.groups.as_ref().unwrap()[id],
    }
}

/// the liminal lane a port on `side` of a box opens into
pub fn attach_lane_box(b: &GBox, side: Side) -> Lane {
    match side {
        Side::Right => Lane { dir: Dir::V, i: b.col1 + 1 },
        Side::Left => Lane { dir: Dir::V, i: b.col0 },
        Side::Top => Lane { dir: Dir::H, i: b.row0 },
        Side::Bottom => Lane { dir: Dir::H, i: b.row1 + 1 },
    }
}

pub fn attach_lane(c: &Cell, side: Side) -> Lane {
    attach_lane_box(&cell_box(c), side)
}

/// abstract group rectangle: borders on the middles of the lanes around the block
pub fn box_rect(g: &Geom, b: &GBox) -> Rect {
    let x = lane_center(g, &Lane { dir: Dir::V, i: b.col0 });
    let y = lane_center(g, &Lane { dir: Dir::H, i: b.row0 });
    Rect {
        x,
        y,
        w: lane_center(g, &Lane { dir: Dir::V, i: b.col1 + 1 }) - x,
        h: lane_center(g, &Lane { dir: Dir::H, i: b.row1 + 1 }) - y,
    }
}

/// what an edge end needs to know about its port
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndInfo {
    pub side: Side,
    pub index: usize,
    pub count: usize,
    pub inner: bool,
}

/// a port's point: on a node's side, or on a group's border (`rect_of`: the group's drawn rectangle, default abstract)
pub fn end_point(g: &Geom, p: &Placement, id: &str, info: &EndInfo, rect_of: Option<&dyn Fn(&str) -> Rect>) -> Pt {
    if let Some(c) = p.cells.get(id) {
        return port_point(g, c, info.side, info.index, info.count);
    }
    let r = match rect_of {
        Some(f) => f(id),
        None => box_rect(g, &p.groups.as_ref().unwrap()[id]),
    };
    rect_port_point(&r, info.side, info.index, info.count)
}

/// slot on a rectangle side, spread like node ports
pub fn rect_port_point(r: &Rect, side: Side, index: usize, count: usize) -> Pt {
    let a = slot_offset(if matches!(side, Side::Left | Side::Right) { r.h } else { r.w }, index, count);
    match side {
        Side::Right => Pt { x: r.x + r.w, y: r.y + a },
        Side::Left => Pt { x: r.x, y: r.y + a },
        Side::Top => Pt { x: r.x + a, y: r.y },
        Side::Bottom => Pt { x: r.x + a, y: r.y + r.h },
    }
}

/// lane middle line: x for vertical lanes, y for horizontal ones
pub fn lane_center(g: &Geom, l: &Lane) -> f64 {
    g.start(l) + g.width(l) / 2.0
}

pub fn track_pos(g: &Geom, l: &Lane, k: f64) -> f64 {
    lane_center(g, l) + k
}

pub fn lane_key(l: &Lane) -> String {
    format!("{}{}", if l.dir == Dir::V { 'v' } else { 'h' }, l.i)
}

pub fn cell_key(c: &Cell) -> String {
    format!("{},{}", c.col, c.row)
}
