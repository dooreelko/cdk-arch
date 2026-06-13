//! Geometry calculations

use super::model::{
    Band, Model, BOX_H, BOX_MARGIN_X, BOX_MARGIN_Y, GUTTER_W, LABEL_PAD, LANE_MIN_H, LANE_MIN_W,
    TITLE_H,
};

pub fn geometry(m: &mut Model) {
    use super::groups::{horizontal_ring_counts, lane_height, lane_width, vertical_ring_counts};
    use super::model::GROUP_PAD;

    m.col_x.clear();
    m.row_y.clear();
    m.col_bands.clear();
    m.row_bands.clear();

    let max_label = m
        .nodes
        .iter()
        .map(|n| n.label.chars().count() as i64)
        .max()
        .unwrap();
    m.box_w = max_label + LABEL_PAD;
    m.box_h = BOX_H;

    let max_col = m.nodes.iter().map(|n| n.grid_col).max().unwrap();
    let max_row = m.nodes.iter().map(|n| n.grid_row).max().unwrap();

    let node_col_w = m.box_w + 2 * BOX_MARGIN_X;
    let node_row_h = m.box_h + 2 * BOX_MARGIN_Y;

    // x offset of the left edge of each canvas grid column region;
    // region 0 begins just right of the spine. Each band records its
    // [start, end), kind, and centre track (lanes only) for the router.
    let vcounts = vertical_ring_counts(&m.groups);
    let hcounts = horizontal_ring_counts(&m.groups);
    let edge_center_off = |left_rings: i64| -> i64 {
        let pad = if left_rings > 0 { GROUP_PAD } else { 0 };
        left_rings + pad + LANE_MIN_W / 2
    };

    let mut x = GUTTER_W + 1;
    // Left bounding lane (region -1), only if some group's left border needs it.
    if let Some(&(l, r)) = vcounts.get(&-1) {
        let w = lane_width(l, r);
        m.col_x.insert(-1, x);
        m.col_bands.push(Band {
            start: x,
            end: x + w,
            kind: "lane",
            center: Some(x + edge_center_off(l)),
        });
        x += w;
    }
    for c in 0..=max_col {
        m.col_x.insert(2 * c, x); // node column c
        m.col_bands.push(Band {
            start: x,
            end: x + node_col_w,
            kind: "node",
            center: None,
        });
        x += node_col_w;
        let region = 2 * c + 1;
        let (l, r) = vcounts.get(&region).copied().unwrap_or((0, 0));
        let w = lane_width(l, r);
        m.col_x.insert(region, x); // vertical lane c
        m.col_bands.push(Band {
            start: x,
            end: x + w,
            kind: "lane",
            center: Some(x + edge_center_off(l)),
        });
        x += w;
    }
    m.canvas_w = x;

    // y offset of the top edge of each canvas grid row region. The title
    // region (000) carries the title text up top and a routing lane below
    // it, so edges approaching the first node row from above have a lane to
    // gravitate into instead of hugging the box tops.
    let mut y = 0;
    m.row_y.insert(0, y); // title
    m.row_bands.push(Band {
        start: 0,
        end: TITLE_H,
        kind: "title",
        center: None,
    });
    let (top_t, top_b) = hcounts.get(&-1).copied().unwrap_or((0, 0));
    let top_h = lane_height(top_t, top_b);
    m.row_bands.push(Band {
        start: TITLE_H,
        end: TITLE_H + top_h,
        kind: "lane",
        center: Some(TITLE_H + {
            let pad = if top_t > 0 { GROUP_PAD } else { 0 };
            top_t + pad + LANE_MIN_H / 2
        }),
    }); // top lane, above node row 0
    y = TITLE_H + top_h;
    for r in 0..=max_row {
        m.row_y.insert(2 * r + 1, y); // node row r
        m.row_bands.push(Band {
            start: y,
            end: y + node_row_h,
            kind: "node",
            center: None,
        });
        y += node_row_h;
        let region = 2 * r + 1; // horizontal lane region key below row r
        let (t, b) = hcounts.get(&region).copied().unwrap_or((0, 0));
        let h = lane_height(t, b);
        m.row_y.insert(2 * r + 2, y); // horizontal lane r
        m.row_bands.push(Band {
            start: y,
            end: y + h,
            kind: "lane",
            center: Some(y + {
                let pad = if t > 0 { GROUP_PAD } else { 0 };
                t + pad + LANE_MIN_H / 2
            }),
        });
        y += h;
    }
    m.canvas_h = y;

    let box_w = m.box_w;
    let box_h = m.box_h;
    for n in &mut m.nodes {
        n.w = box_w;
        n.h = box_h;
        n.x = m.col_x[&(2 * n.grid_col)] + BOX_MARGIN_X;
        n.y = m.row_y[&(2 * n.grid_row + 1)] + BOX_MARGIN_Y;
    }

    super::model::build_band_caches(m);
}
