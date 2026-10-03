use c43_layout::check::pair_crossings;
use c43_layout::model::Pt;
use c43_layout::tracks::{assign_tracks, improve_tracks, Seg, Tracks};

fn seg(edge: usize, k: usize, lane: &str, lo: f64, hi: f64) -> Seg {
    Seg { edge, k, lane: lane.into(), band: "m".into(), lo, hi }
}

fn tracks(xs: &[((usize, usize), i32)]) -> Tracks {
    xs.iter().copied().collect()
}

#[test]
fn overlapping_segments_get_distinct_tracks_disjoint_ones_share_the_middle() {
    let t = assign_tracks(&[seg(0, 0, "v1", 0., 5.), seg(1, 0, "v1", 3., 8.), seg(2, 0, "v1", 9., 10.), seg(3, 0, "h1", 0., 10.)]);
    assert_eq!(t[&(0, 0)], 0);
    assert_eq!(t[&(1, 0)], -1);
    assert_eq!(t[&(2, 0)], 0);
    assert_eq!(t[&(3, 0)], 0);
}

#[test]
fn touching_intervals_conflict() {
    let t = assign_tracks(&[seg(0, 0, "v1", 0., 5.), seg(1, 0, "v1", 5., 8.)]);
    assert_ne!(t[&(0, 0)], t[&(1, 0)]);
}

#[test]
fn improve_tracks_swaps_tracks_to_remove_a_crossing() {
    let segs = [seg(0, 0, "v1", 0., 10.), seg(1, 0, "v1", 0., 10.)];
    // edge 0: vertical at its track, then a stub right to x = 5; edge 1: vertical at its track
    let poly_of = |e: usize, t: &Tracks| -> Vec<Pt> {
        let x = t[&(e, 0)] as f64;
        if e == 0 { vec![Pt { x, y: 0. }, Pt { x, y: 5. }, Pt { x: 5., y: 5. }] } else { vec![Pt { x, y: -5. }, Pt { x, y: 10. }] }
    };
    let bad = tracks(&[((0, 0), -1), ((1, 0), 0)]);
    assert_eq!(pair_crossings(&poly_of(0, &bad), &poly_of(1, &bad)), 1);
    let t = improve_tracks(&segs, &bad, &poly_of, &[0, 1]);
    assert_eq!(pair_crossings(&poly_of(0, &t), &poly_of(1, &t)), 0);
    assert_ne!(t[&(0, 0)], t[&(1, 0)]);
}

#[test]
fn improve_tracks_swaps_two_edges_across_all_their_shared_lanes_at_once() {
    let segs = [seg(0, 0, "h2", 13., 18.5), seg(1, 0, "h2", 13., 17.5), seg(0, 1, "v1", 25., 50.5), seg(1, 1, "v1", 25., 62.5)];
    let poly_of = |e: usize, t: &Tracks| -> Vec<Pt> {
        let y = 26. + t[&(e, 0)] as f64;
        let x = 14. + t[&(e, 1)] as f64;
        let (port, end) = if e == 0 { (18.5, 50.5) } else { (17.5, 62.5) };
        vec![Pt { x: port, y: 24. }, Pt { x: port, y }, Pt { x, y }, Pt { x, y: end }, Pt { x: 20.5, y: end }]
    };
    let bad = tracks(&[((0, 0), -1), ((0, 1), -1), ((1, 0), 0), ((1, 1), 0)]);
    assert_eq!(pair_crossings(&poly_of(0, &bad), &poly_of(1, &bad)), 2);
    let t = improve_tracks(&segs, &bad, &poly_of, &[0, 1]);
    assert_eq!(pair_crossings(&poly_of(0, &t), &poly_of(1, &t)), 0);
}
