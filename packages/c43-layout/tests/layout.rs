mod common;
use c43_layout::layout::{build_layout, width_for};
use c43_layout::model::SizeHints;
use c43_layout::place::place;
use c43_layout::ports::assign_ports;
use c43_layout::route::route_all;
use c43_layout::skeleton::skeleton;
use c43_layout::tracks::{assign_tracks, lane_segments, Tracks};
use common::case_graph;

#[test]
fn width_for_odd_at_least_3_covers_the_widest_track() {
    assert_eq!(width_for(&Tracks::new()), 3.0);
    assert_eq!(width_for(&[((0, 0), -1), ((1, 0), 1)].into_iter().collect()), 3.0);
    assert_eq!(width_for(&[((0, 0), 2)].into_iter().collect()), 5.0);
}

#[test]
fn aws_geometry_ports_straight_shots_grid_alignment() {
    let g = case_graph("aws");
    let p = place(&g, &skeleton(&g));
    let ports = assign_ports(&g, &p);
    let routes = route_all(&g, &p, &ports);
    let segs = lane_segments(&g, &p, &ports, &routes);
    let l = build_layout(&g, &p, &ports, &routes, &assign_tracks(&segs), &SizeHints::default(), 0.0, &segs);
    assert_eq!(l.s, 3.0);
    assert!(l.lanes.v.iter().chain(&l.lanes.h).all(|w| *w >= 3.0 && w % 2.0 == 1.0));
    for ws in [&l.lanes.v, &l.lanes.h] {
        assert!(ws[0] >= 5.0 && *ws.last().unwrap() >= 5.0, "outer lanes keep the margin");
    }
    let at = |ws: &[f64], i: i32| ws[..=i as usize].iter().sum::<f64>() + i as f64 * l.s;
    for n in &l.nodes {
        assert_eq!(n.size, l.s);
        assert_eq!(n.x, at(&l.lanes.v, n.col));
        assert_eq!(n.y, at(&l.lanes.h, n.row));
    }
    for e in &l.edges {
        assert_eq!((e.points[0].x, e.points[0].y), (e.src.x, e.src.y));
        let last = e.points.last().unwrap();
        assert_eq!((last.x, last.y), (e.dst.x, e.dst.y));
    }
    let straight = ["U>CF", "CF>AGW", "CF>R53", "AGW>LB", "LA>SES", "LB>S3", "LC>ATH"];
    for e in &l.edges {
        if straight.contains(&format!("{}>{}", e.from, e.to).as_str()) {
            assert_eq!(e.points.len(), 2, "{}>{}", e.from, e.to);
        }
    }
    let ath_s3 = l.edges.iter().find(|e| e.from == "ATH" && e.to == "S3").unwrap();
    assert_eq!(ath_s3.points.len(), 4);
}
