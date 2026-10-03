//! A lane is cut into 1-unit slots. Group borders run in the lanes around their block; edge tracks sit in
//! bands between them. From the cells before the lane ("low" side): band l0, border 1, band l1, border 2, …;
//! mirrored from the cells after it ("high" side): h0, border 1, h1, …; the middle band `m` (edges outside
//! every border) is centred in what is left. A top border keeps room for its group's title just inside it.
//! Each lane is only as wide as its own bands, borders and title room need.
use crate::groups::chain;
use crate::model::Graph;
use crate::model::Placement;
use indexmap::{IndexMap, IndexSet};

/// per side, group id → rank k (1 = nearest the cells, enclosing groups further out)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Borders {
    pub low: IndexMap<String, i32>,
    pub high: IndexMap<String, i32>,
}

pub fn borders(g: &Graph, p: &Placement) -> IndexMap<String, Borders> {
    let mut at: IndexMap<String, (Vec<String>, Vec<String>)> = IndexMap::new();
    let mut add = |lane: String, high: bool, id: &str| {
        let e = at.entry(lane).or_default();
        if high { e.1.push(id.to_string()) } else { e.0.push(id.to_string()) }
    };
    for (id, b) in p.groups.iter().flatten() {
        add(format!("v{}", b.col0), true, id);
        add(format!("v{}", b.col1 + 1), false, id);
        add(format!("h{}", b.row0), true, id);
        add(format!("h{}", b.row1 + 1), false, id);
    }
    // k = 1 + the deepest nesting of same-side groups inside this one (siblings share a rank)
    let rank = |ids: &[String]| -> IndexMap<String, i32> {
        fn of(g: &Graph, ids: &[String], id: &str, k: &mut IndexMap<String, i32>) -> i32 {
            if let Some(x) = k.get(id) {
                return *x;
            }
            let inside: Vec<&String> = ids.iter().filter(|o| *o != id && chain(g, o)[1..].iter().any(|c| c == id)).collect();
            let v = 1 + inside.into_iter().map(|o| of(g, ids, o, k)).fold(0, i32::max);
            k.insert(id.to_string(), v);
            v
        }
        let mut k: IndexMap<String, i32> = IndexMap::new();
        let mut ranked: Vec<(String, usize, i32)> = ids.iter().enumerate().map(|(i, id)| (id.clone(), i, of(g, ids, id, &mut k))).collect();
        ranked.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
        ranked.into_iter().map(|(id, _, k)| (id, k)).collect()
    };
    at.into_iter().map(|(lane, (low, high))| (lane, Borders { low: rank(&low), high: rank(&high) })).collect()
}

/// band of a segment owned by `own`: inside the innermost border holding the owner, else the middle
pub fn band_of(g: &Graph, b: Option<&Borders>, own: &str) -> String {
    let Some(b) = b else { return "m".into() };
    if own.is_empty() {
        return "m".into();
    }
    let holds: IndexSet<String> = chain(g, own).into_iter().collect();
    for (side, tag) in [(&b.low, 'l'), (&b.high, 'h')] {
        let ks: Vec<i32> = side.iter().filter(|(id, _)| holds.contains(*id)).map(|(_, k)| *k).collect();
        if let Some(k) = ks.iter().min() {
            return format!("{tag}{}", k - 1);
        }
    }
    "m".into()
}
