use c43::cmd::layout::model::{BOX_H, GUTTER_W, LABEL_PAD, TITLE_H};
use c43::cmd::layout::{geometry::geometry, parse::parse_and_validate};
use serde_json::json;

fn model_2x2() -> c43::cmd::layout::model::Model {
    let raw = json!({"title":"T","description":"D","nodes":[
        {"id":"a","label":"alpha","grid_col":0,"grid_row":0},
        {"id":"b","label":"b","grid_col":1,"grid_row":0},
        {"id":"c","label":"charlie","grid_col":0,"grid_row":1}],
        "edges":[{"id":"e1","from":"a","to":"b"}]});
    parse_and_validate(&raw).unwrap()
}

#[test]
fn box_width_from_widest_label() {
    let mut m = model_2x2();
    geometry(&mut m);
    assert_eq!(m.box_w, "charlie".chars().count() as i64 + LABEL_PAD);
    assert_eq!(m.box_h, BOX_H);
}

#[test]
fn all_boxes_identical_size() {
    let mut m = model_2x2();
    geometry(&mut m);
    assert!(m.nodes.iter().all(|n| n.w == m.box_w));
    assert!(m.nodes.iter().all(|n| n.h == m.box_h));
}

#[test]
fn nodes_get_positive_coords_inside_canvas() {
    let mut m = model_2x2();
    geometry(&mut m);
    assert!(m.canvas_w > 0 && m.canvas_h > 0);
    for n in &m.nodes {
        assert!(n.x > GUTTER_W);
        assert!(n.x + n.w < m.canvas_w);
        assert!(n.y >= TITLE_H);
        assert!(n.y + n.h < m.canvas_h);
    }
}

#[test]
fn column_and_row_ordering() {
    let mut m = model_2x2();
    geometry(&mut m);
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    let b = m.nodes.iter().find(|n| n.id == "b").unwrap();
    let c = m.nodes.iter().find(|n| n.id == "c").unwrap();
    assert!(a.x < b.x); // col 0 left of col 1
    assert!(a.y < c.y); // row 0 above row 1
    assert_eq!(a.y, b.y); // same row -> same y
}

#[test]
fn exact_geometry_offsets() {
    let mut m = model_2x2();
    geometry(&mut m);
    assert_eq!(m.box_w, 11);
    assert_eq!(m.canvas_w, 79);
    assert_eq!(m.canvas_h, 57);
    assert_eq!(m.col_x.get(&0), Some(&9));
    assert_eq!(m.col_x.get(&1), Some(&28));
    assert_eq!(m.col_x.get(&2), Some(&44));
    assert_eq!(m.col_x.get(&3), Some(&63));
    assert_eq!(m.row_y.get(&0), Some(&0));
    assert_eq!(m.row_y.get(&1), Some(&13));
    assert_eq!(m.row_y.get(&2), Some(&28));
    assert_eq!(m.row_y.get(&3), Some(&35));
    assert_eq!(m.row_y.get(&4), Some(&50));
    let a = m.nodes.iter().find(|n| n.id == "a").unwrap();
    assert_eq!((a.x, a.y), (13, 15));
}

#[test]
fn top_lane_above_first_node_row() {
    let mut m = model_2x2();
    geometry(&mut m);
    assert_eq!(m.row_bands[0].kind, "title");
    assert_eq!(m.row_bands[1].kind, "lane");
    assert_eq!(m.row_bands[2].kind, "node");
    let title_end = m.row_bands[0].end;
    let node0_start = m.row_bands[2].start;
    let lane_center = m.row_bands[1].center.unwrap();
    assert!(title_end <= lane_center && lane_center < node0_start);
}
