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

/// which side of a lane, seen from its cells: before it (low) or after it (high)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Low,
    High,
}

/// slots a group title takes inside its top border: one text line plus a gap
pub fn title_slots(sizes: &crate::model::SizeHints) -> f64 {
    sizes.title_height.ceil() + 1.0
}

/// narrowest inner lane; lanes at the frame edge keep at least MARGIN of breathing room — both provisional (moth ios4y)
pub const LANE_MIN: f64 = 3.0;
pub const MARGIN: f64 = 5.0;

#[derive(Debug, Clone, Default)]
struct SideInfo {
    start: Vec<f64>,
    border: Vec<f64>,
    total: f64,
}

/// `k(lane, band)`: largest |track| used in a band (tracks are centred on 0)
pub type Reach<'a> = std::boxed::Box<dyn Fn(&str, &str) -> i32 + 'a>;

pub struct LanePlan<'a> {
    k: Reach<'a>,
    info: IndexMap<String, (SideInfo, SideInfo)>,
    widths: IndexMap<String, f64>,
    starts: IndexMap<String, f64>,
}

fn lane_index(l: &str) -> i32 {
    l[1..].parse().unwrap()
}

impl LanePlan<'_> {
    fn slots(&self, lane: &str, band: &str) -> f64 {
        2.0 * (self.k)(lane, band).max(0) as f64 + 1.0
    }

    /// a lane's own width: odd, fits its bands, borders and title room
    pub fn width(&self, lane: &str) -> f64 {
        self.widths[lane]
    }

    /// absolute coordinate where a lane begins (lanes and S-wide cells alternate)
    pub fn start(&self, lane: &str) -> f64 {
        self.starts[lane]
    }

    /// absolute coordinate of track t of a band
    pub fn pos(&self, lane: &str, band: &str, t: f64) -> f64 {
        let (low, high) = &self.info[lane];
        let w = self.width(lane);
        let base = self.start(lane);
        if band == "m" {
            return base + low.total + ((w - low.total - high.total) / 2.0).floor() + 0.5 + t;
        }
        let b: usize = band[1..].parse().unwrap();
        let is_low = band.starts_with('l');
        let centre = (if is_low { low } else { high }).start[b] + (self.slots(lane, band) / 2.0).floor();
        if is_low { base + centre + 0.5 + t } else { base + w - (centre + 0.5) + t }
    }

    /// absolute coordinate of the k-th border on a side
    pub fn border(&self, lane: &str, s: Half, k: i32) -> f64 {
        let (low, high) = &self.info[lane];
        let base = self.start(lane);
        match s {
            Half::Low => base + low.border[k as usize - 1] + 0.5,
            Half::High => base + self.width(lane) - (high.border[k as usize - 1] + 0.5),
        }
    }
}

pub fn lane_plan<'a>(s: f64, bs: &IndexMap<String, Borders>, k: Reach<'a>, title_slots: f64, lane_keys: &[String]) -> LanePlan<'a> {
    let mut plan = LanePlan { k, info: IndexMap::new(), widths: IndexMap::new(), starts: IndexMap::new() };
    let side = |plan: &LanePlan, lane: &str, half: Half| -> SideInfo {
        let ranks = bs.get(lane).map(|b| if half == Half::Low { &b.low } else { &b.high });
        let n = ranks.map_or(0, |r| r.values().copied().fold(0, i32::max));
        let mut out = SideInfo::default();
        let mut at = 0.0;
        for b in 0..n {
            out.start.push(at);
            let tag = if half == Half::Low { 'l' } else { 'h' };
            at += plan.slots(lane, &format!("{tag}{b}")) + if half == Half::High && lane.starts_with('h') { title_slots } else { 0.0 };
            out.border.push(at);
            at += 1.0;
        }
        out.total = at;
        out
    };
    let info: IndexMap<String, (SideInfo, SideInfo)> =
        lane_keys.iter().map(|l| (l.clone(), (side(&plan, l, Half::Low), side(&plan, l, Half::High)))).collect();
    let last = |d: char| lane_keys.iter().filter(|l| l.starts_with(d)).map(|l| lane_index(l)).fold(-1, i32::max);
    let outer = |l: &str| lane_index(l) == 0 || lane_index(l) == last(l.chars().next().unwrap());
    let widths: IndexMap<String, f64> = lane_keys
        .iter()
        .map(|l| {
            let (low, high) = &info[l];
            let w = (if outer(l) { MARGIN } else { LANE_MIN }).max(low.total + high.total + plan.slots(l, "m"));
            (l.clone(), if w % 2.0 != 0.0 { w } else { w + 1.0 })
        })
        .collect();
    let mut starts: IndexMap<String, f64> = IndexMap::new();
    for d in ['v', 'h'] {
        let mut at = 0.0;
        for i in 0..=last(d) {
            let key = format!("{d}{i}");
            starts.insert(key.clone(), at);
            at += widths.get(&key).copied().unwrap_or(LANE_MIN) + s;
        }
    }
    plan.info = info;
    plan.widths = widths;
    plan.starts = starts;
    plan
}
