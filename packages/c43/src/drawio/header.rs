//! The title/description band above the grid.
use super::text::{wrap, LINE_H, PAD, TITLE_PX};
use c43_layout::model::{Rect, TextBlock};

/// title/description band above the grid (ends at y = 0)
pub struct Header {
    pub rect: Rect,
    pub title: Option<TextBlock>,
    pub description: Option<TextBlock>,
}

/**
 * title then description, stacked in a band `grid_w` wide (widened only if one word cannot fit);
 * heights come from an estimated line count, the renderer does the actual wrapping; band rounded up to whole units
 */
pub fn header(title: Option<&str>, description: Option<&str>, grid_w: f64, unit_px: f64) -> Option<Header> {
    if title.is_none() && description.is_none() {
        return None;
    }
    let mut w = grid_w;
    loop {
        let inner = w * unit_px - 2.0 * PAD;
        let lines = |t: Option<&str>, bold: bool| t.map_or(Some(vec![]), |t| wrap(t, inner, TITLE_PX, bold));
        let (Some(tl), Some(dl)) = (lines(title, true), lines(description, false)) else {
            w += 1.0;
            continue;
        };
        let lh = (TITLE_PX * LINE_H) / unit_px;
        let pad = PAD / unit_px;
        let h = ((2.0 * PAD + (tl.len() + dl.len()) as f64 * TITLE_PX * LINE_H) / unit_px).ceil();
        let (x, bw, ty) = (pad, w - 2.0 * pad, -h + pad);
        let dy = ty + tl.len() as f64 * lh;
        let block = |text: &str, y: f64, n: usize| TextBlock { text: text.to_string(), x, y, w: bw, h: n as f64 * lh };
        return Some(Header {
            rect: Rect { x: 0.0, y: -h, w, h },
            title: title.map(|t| block(t, ty, tl.len())),
            description: description.map(|d| block(d, dy, dl.len())),
        });
    }
}
