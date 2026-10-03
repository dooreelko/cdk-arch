use c43_layout::geom::*;
use c43_layout::model::{Box as GBox, Cell, Dir, Lane, Pt, Rect, Side};

fn lane(dir: Dir, i: i32) -> Lane {
    Lane { dir, i }
}

#[test]
fn box_lanes_and_border_ports() {
    let b = GBox { col0: 1, row0: 0, col1: 2, row1: 1 };
    assert_eq!(attach_lane_box(&b, Side::Right), lane(Dir::V, 3));
    assert_eq!(attach_lane_box(&b, Side::Left), lane(Dir::V, 1));
    assert_eq!(attach_lane_box(&b, Side::Top), lane(Dir::H, 0));
    assert_eq!(attach_lane_box(&b, Side::Bottom), lane(Dir::H, 2));
    let g = Geom::uniform(3.0, 1.0);
    assert_eq!(box_rect(&g, &b), Rect { x: 4.5, y: 0.5, w: 8.0, h: 8.0 });
    let r = Rect { x: 0.0, y: 0.0, w: 8.0, h: 4.0 };
    assert_eq!(rect_port_point(&r, Side::Left, 0, 2), Pt { x: 0.0, y: 1.0 });
    assert_eq!(rect_port_point(&r, Side::Bottom, 1, 2), Pt { x: 6.0, y: 4.0 });
}

#[test]
fn grid_geometry() {
    let g = Geom::uniform(3.0, 3.0);
    assert_eq!(g.start(&lane(Dir::V, 2)), 12.0);
    assert_eq!(g.width(&lane(Dir::H, 7)), 3.0);
    assert_eq!(node_origin(&g, &Cell { col: 1, row: 2 }), Pt { x: 9.0, y: 15.0 });
    assert_eq!(slot_offset(3.0, 0, 1), 1.5);
    assert_eq!(slot_offset(3.0, 0, 2), 0.75);
    assert_eq!(slot_offset(3.0, 2, 3), 2.5);
    assert_eq!(port_point(&g, &Cell { col: 0, row: 0 }, Side::Right, 0, 2), Pt { x: 6.0, y: 3.75 });
    assert_eq!(port_point(&g, &Cell { col: 0, row: 0 }, Side::Bottom, 0, 1), Pt { x: 4.5, y: 6.0 });
    let c = Cell { col: 2, row: 1 };
    assert_eq!(attach_lane(&c, Side::Right), lane(Dir::V, 3));
    assert_eq!(attach_lane(&c, Side::Left), lane(Dir::V, 2));
    assert_eq!(attach_lane(&c, Side::Top), lane(Dir::H, 1));
    assert_eq!(attach_lane(&c, Side::Bottom), lane(Dir::H, 2));
    assert_eq!(lane_center(&g, &lane(Dir::V, 2)), 13.5);
    assert_eq!(track_pos(&g, &lane(Dir::V, 2), -1.0), 12.5);
    assert_eq!(lane_key(&lane(Dir::H, 4)), "h4");
    assert_eq!(cell_key(&Cell { col: 2, row: 5 }), "2,5");
}

#[test]
fn per_lane_geometry_follows_each_lanes_own_width() {
    let v = [5.0, 3.0, 7.0];
    let h = [5.0, 9.0];
    let ws = move |l: &Lane| if l.dir == Dir::V { v.to_vec() } else { h.to_vec() };
    let at = move |w: Vec<f64>, i: i32| w[..i as usize].iter().map(|x| x + 3.0).sum::<f64>();
    let g = Geom::new(3.0, Box::new(move |l| at(ws(l), l.i)), Box::new(move |l| ws(l)[l.i as usize]));
    assert_eq!(node_origin(&g, &Cell { col: 1, row: 1 }), Pt { x: 5.0 + 3.0 + 3.0, y: 5.0 + 3.0 + 9.0 });
    assert_eq!(lane_center(&g, &lane(Dir::V, 2)), 5.0 + 3.0 + 3.0 + 3.0 + 3.5);
    assert_eq!(box_rect(&g, &GBox { col0: 0, row0: 0, col1: 1, row1: 0 }), Rect { x: 2.5, y: 2.5, w: 15.0, h: 10.0 });
}
