mod common;
use c43_layout::engine::layout;
use c43_layout::model::*;
use common::load_case;

fn run(name: &str) -> LayoutResult {
    let (g, h) = load_case(name);
    layout(&g, &h, &Default::default()).unwrap()
}

fn at<'a>(l: &'a Layout, id: &str) -> &'a LayoutNode {
    l.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn flat_cases_zero_violations_story_rules_deterministic() {
    for name in ["aws", "star16"] {
        let r = run(name);
        assert!(r.metrics.violations.is_empty(), "{name}: {:?}", r.metrics.violations);
        assert_eq!(r.metrics.soft.leftward, 0.0);
        assert_eq!(r.metrics.soft.nf_bottom_share, 1.0);
        assert_eq!(r, run(name), "{name} deterministic");
    }
}

#[test]
fn groups_zero_violations_story_rules() {
    let m = run("groups").metrics;
    assert!(m.violations.is_empty(), "{:?}", m.violations);
    assert_eq!(m.soft.leftward, 0.0);
}

#[test]
fn container_stress_group_rules_hold() {
    let t0 = std::time::Instant::now();
    let r = run("rebob-container");
    let l = &r.layout;
    assert!(r.metrics.violations.iter().all(|v| v.rule == "crossing"), "{:?}", r.metrics.violations.iter().filter(|v| v.rule != "crossing").collect::<Vec<_>>());
    assert!(r.metrics.crossings <= 37, "crossings {}", r.metrics.crossings);
    assert!(l.frame.w * l.frame.h < 0.8 * 222.0 * 206.0, "frame {} × {}", l.frame.w, l.frame.h);
    for a in &l.groups {
        for b in &l.groups {
            if a.parent != b.parent || a.id == b.id {
                continue;
            }
            if a.bx.row0 == b.bx.row0 && a.bx.row1 == b.bx.row1 {
                assert_eq!((a.y, a.h), (b.y, b.h), "{} / {}", a.id, b.id);
            }
            if a.bx.col0 == b.bx.col0 && a.bx.col1 == b.bx.col1 {
                assert_eq!((a.x, a.w), (b.x, b.w), "{} / {}", a.id, b.id);
            }
        }
    }
    assert!(t0.elapsed().as_secs() < 120, "took {:?}", t0.elapsed());
}

#[test]
fn aws_reads_like_the_sketch() {
    let l = run("aws").layout;
    assert_eq!(at(&l, "U").col, 0);
    assert_eq!(at(&l, "S3").col, l.cols - 1);
    assert_eq!(at(&l, "R53").col, at(&l, "CF").col);
    assert!(at(&l, "R53").row > at(&l, "CF").row);
    assert!(at(&l, "SES").row < at(&l, "S3").row);
    assert_eq!(at(&l, "CW").row, l.rows - 1);
    // ports alone need 3 (AGW); labels such as "CloudWatch" need more at the default unit
    assert_eq!(l.s, 5.0);
    assert_eq!(at(&l, "LC").col, at(&l, "LB").col);
    assert_eq!(at(&l, "LC").row, at(&l, "LB").row + 1);
    assert_eq!(at(&l, "LC").row, at(&l, "ATH").row);
}

#[test]
fn star16_stress_shape() {
    let l = run("star16").layout;
    assert_eq!(at(&l, "N1").col, 0);
    assert_eq!(at(&l, "N2").col, 1);
    assert_eq!(at(&l, "N1").row, at(&l, "N2").row);
    assert!(l.s >= 7.0, "S={}", l.s);
}
