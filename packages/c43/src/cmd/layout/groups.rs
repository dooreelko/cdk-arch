//! Group frames: build + validate from raw JSON, resolve grid extents,
//! lay out lane rings, and expose border cells for routing/rendering.

#![allow(dead_code)] // functions are wired in over the next tasks

use super::model::{Group, Node};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

/// `str()`-style rendering for ids (always strings in practice).
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

/// Build groups from the raw `groups` array (missing/null -> empty). Validates
/// structure only (ids, parents, members); extents/depth are resolved later.
pub fn build_groups(raw: &Value, nodes: &[Node]) -> Result<Vec<Group>, String> {
    let arr = match raw.get("groups") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(a)) => a.clone(),
        Some(other) => {
            return Err(format!("groups must be an array, got {}", other));
        }
    };

    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut seen_group_ids: HashSet<String> = HashSet::new();
    let mut groups: Vec<Group> = Vec::new();

    // First pass: read entries, validate ids/members. Parent existence checked
    // in a second pass once all group ids are known.
    for (i, gr) in arr.iter().enumerate() {
        let id_v = gr
            .get("id")
            .ok_or_else(|| format!("group at index {i} missing required field: id"))?;
        let gid = py_str(id_v);
        if !seen_group_ids.insert(gid.clone()) {
            return Err(format!("duplicate group id: {gid}"));
        }

        let title = gr
            .get("title")
            .map(py_str)
            .ok_or_else(|| format!("group {gid} missing required field: title"))?;

        let members_v = gr
            .get("members")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut member_ids = Vec::new();
        for mv in &members_v {
            let mid = py_str(mv);
            if !node_ids.contains(mid.as_str()) {
                return Err(format!("group {gid} references unknown member node id: {mid}"));
            }
            member_ids.push(mid);
        }

        let parent = match gr.get("parent") {
            None | Some(Value::Null) => None,
            Some(p) => Some(py_str(p)),
        };

        groups.push(Group {
            id: gid,
            title,
            parent,
            member_ids,
            depth: 0,
            col0: 0, col1: 0, row0: 0, row1: 0,
            x: 0, y: 0, w: 0, h: 0,
        });
    }

    // Second pass: every parent must name an existing group.
    for g in &groups {
        if let Some(p) = &g.parent {
            if !seen_group_ids.contains(p) {
                return Err(format!("group {} references unknown parent group id: {p}", g.id));
            }
        }
    }

    Ok(groups)
}

/// Resolve grid extents (members ∪ descendant extents) and depth for every
/// group, in place. Returns an error on a parent cycle.
pub fn resolve_extents(groups: &mut [Group], nodes: &[Node]) -> Result<(), String> {
    let node_cell: HashMap<&str, (i64, i64)> = nodes
        .iter()
        .map(|n| (n.id.as_str(), (n.grid_col, n.grid_row)))
        .collect();

    let id_index: HashMap<String, usize> =
        groups.iter().enumerate().map(|(i, g)| (g.id.clone(), i)).collect();

    // children adjacency
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, g) in groups.iter().enumerate() {
        if let Some(p) = &g.parent {
            let pi = id_index[p];
            children.entry(pi).or_default().push(i);
        }
    }

    // depth: walk parent chain; detect cycle with a bounded step count.
    let n = groups.len();
    for i in 0..n {
        let mut depth = 0i64;
        let mut cur = groups[i].parent.clone();
        let mut steps = 0;
        while let Some(p) = cur {
            steps += 1;
            if steps > n as i64 {
                return Err(format!("cycle detected in group parent chain at {}", groups[i].id));
            }
            depth += 1;
            cur = groups[id_index[&p]].parent.clone();
        }
        groups[i].depth = depth;
    }

    // extents: post-order over the parent tree (deepest first). Sorting indices
    // by descending depth guarantees children are computed before parents.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| -groups[i].depth);

    // seed every group from its direct members
    let mut ext: Vec<Option<(i64, i64, i64, i64)>> = vec![None; n];
    for i in 0..n {
        for mid in &groups[i].member_ids {
            let (c, r) = node_cell[mid.as_str()];
            ext[i] = Some(match ext[i] {
                None => (c, c, r, r),
                Some((c0, c1, r0, r1)) => (c0.min(c), c1.max(c), r0.min(r), r1.max(r)),
            });
        }
    }
    // fold child extents up into parents (deepest first)
    for &i in &order {
        if let Some(kids) = children.get(&i) {
            for &k in kids {
                if let Some((kc0, kc1, kr0, kr1)) = ext[k] {
                    ext[i] = Some(match ext[i] {
                        None => (kc0, kc1, kr0, kr1),
                        Some((c0, c1, r0, r1)) =>
                            (c0.min(kc0), c1.max(kc1), r0.min(kr0), r1.max(kr1)),
                    });
                }
            }
        }
    }

    for i in 0..n {
        let (c0, c1, r0, r1) = ext[i].ok_or_else(|| {
            format!("group {} has no members and no child groups", groups[i].id)
        })?;
        groups[i].col0 = c0;
        groups[i].col1 = c1;
        groups[i].row0 = r0;
        groups[i].row1 = r1;
    }

    Ok(())
}

/// Validate that each group's rectangle encloses only its own members or
/// descendants' members, and that no two groups partially overlap.
pub fn validate_extents(groups: &[Group], nodes: &[Node]) -> Result<(), String> {
    let id_index: HashMap<&str, usize> =
        groups.iter().enumerate().map(|(i, g)| (g.id.as_str(), i)).collect();

    // owned[i] = members of group i ∪ members of all its descendants.
    let n = groups.len();
    let mut owned: Vec<HashSet<&str>> = vec![HashSet::new(); n];
    for i in 0..n {
        for (j, _g) in groups.iter().enumerate() {
            // j is owned-by-i if i is on j's ancestor chain (including j == i)
            let mut cur = Some(j);
            let mut is_desc = false;
            let mut steps = 0;
            while let Some(c) = cur {
                if c == i { is_desc = true; break; }
                cur = groups[c].parent.as_deref().map(|p| id_index[p]);
                steps += 1;
                if steps > n { break; }
            }
            if is_desc {
                for mid in &groups[j].member_ids {
                    owned[i].insert(mid.as_str());
                }
            }
        }
    }

    // enclosing non-members
    for (i, g) in groups.iter().enumerate() {
        for nd in nodes {
            let inside = g.col0 <= nd.grid_col && nd.grid_col <= g.col1
                && g.row0 <= nd.grid_row && nd.grid_row <= g.row1;
            if inside && !owned[i].contains(nd.id.as_str()) {
                return Err(format!(
                    "group {} encloses non-member node {}", g.id, nd.id
                ));
            }
        }
    }

    // partial overlap: rectangles either disjoint, or one fully contains the other
    let contains = |a: &Group, b: &Group| -> bool {
        a.col0 <= b.col0 && b.col1 <= a.col1 && a.row0 <= b.row0 && b.row1 <= a.row1
    };
    let disjoint = |a: &Group, b: &Group| -> bool {
        a.col1 < b.col0 || b.col1 < a.col0 || a.row1 < b.row0 || b.row1 < a.row0
    };
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (&groups[i], &groups[j]);
            if disjoint(a, b) || contains(a, b) || contains(b, a) {
                continue;
            }
            return Err(format!("groups {} and {} overlap", a.id, b.id));
        }
    }

    Ok(())
}

/// For each vertical lane region index, how many group borders pack on its
/// left side (right-borders of groups ending at the node column to the left)
/// and its right side (left-borders of groups starting at the node column to
/// the right). Region -1 is the left bounding lane.
/// Returns (vertical_border_cells, horizontal_border_cells) for all group
/// frames. Vertical = left/right sides (block vertical travel; cross with
/// horizontal moves). Horizontal = top/bottom sides. Corners appear in both.
pub fn border_cells(groups: &[Group]) -> (HashSet<(i64, i64)>, HashSet<(i64, i64)>) {
    let mut vert = HashSet::new();
    let mut horiz = HashSet::new();
    for g in groups {
        let (x0, y0) = (g.x, g.y);
        let (x1, y1) = (g.x + g.w - 1, g.y + g.h - 1);
        for y in y0..=y1 {
            vert.insert((x0, y));
            vert.insert((x1, y));
        }
        for x in x0..=x1 {
            horiz.insert((x, y0));
            horiz.insert((x, y1));
        }
    }
    (vert, horiz)
}

pub fn vertical_ring_counts(groups: &[Group]) -> BTreeMap<i64, (i64, i64)> {
    let mut counts: BTreeMap<i64, (i64, i64)> = BTreeMap::new();
    for g in groups {
        // left border sits in the lane left of col0: region 2*col0 - 1
        let left_lane = 2 * g.col0 - 1;
        counts.entry(left_lane).or_insert((0, 0)).1 += 1; // right side of that lane
        // right border sits in the lane right of col1: region 2*col1 + 1
        let right_lane = 2 * g.col1 + 1;
        counts.entry(right_lane).or_insert((0, 0)).0 += 1; // left side of that lane
    }
    counts
}

/// Row analogue. Region -1 is the top lane (before node row 0). top side ->
/// `.1`, bottom side -> `.0`.
pub fn horizontal_ring_counts(groups: &[Group]) -> BTreeMap<i64, (i64, i64)> {
    let mut counts: BTreeMap<i64, (i64, i64)> = BTreeMap::new();
    for g in groups {
        let top_lane = 2 * g.row0 - 1;
        counts.entry(top_lane).or_insert((0, 0)).1 += 1;
        let bottom_lane = 2 * g.row1 + 1;
        counts.entry(bottom_lane).or_insert((0, 0)).0 += 1;
    }
    counts
}

/// Depth-rank of group `id` among all groups matching `predicate`, deepest = 0.
/// Used to pack nested borders: rank 0 hugs the nodes, higher ranks step toward
/// the lane centre.
fn rank_by_depth(groups: &[Group], id: &str, predicate: impl Fn(&Group) -> bool) -> i64 {
    let mut owners: Vec<&Group> = groups.iter().filter(|g| predicate(g)).collect();
    owners.sort_by_key(|g| -g.depth);
    owners.iter().position(|g| g.id == id).unwrap() as i64
}

/// Assign each group its pixel rectangle. Requires `col_x`/`row_y`/bands built
/// by geometry. Borders pack toward nodes by nesting depth (deepest innermost):
/// on a lane's side nearer the enclosed nodes the deepest group sits closest to
/// the nodes, shallower groups step outward toward the lane centre.
pub fn assign_boxes(
    groups: &mut [Group],
    col_x: &BTreeMap<i64, i64>,
    row_y: &BTreeMap<i64, i64>,
    col_bands: &[super::model::Band],
    row_bands: &[super::model::Band],
) {
    // Column lane span: col_x region keys align with band starts (incl. -1).
    let col_lane_span = |region: i64| -> Option<(i64, i64)> {
        let s = *col_x.get(&region)?;
        let b = col_bands.iter().find(|b| b.start == s && b.kind == "lane")?;
        Some((b.start, b.end))
    };
    // Row lanes are located by band adjacency to the node row, because row_y's
    // lane keys (2r+2) and the top lane (no key) don't match the column
    // convention. The lane ABOVE node row r is the "lane" band whose `end`
    // touches the node row start; the lane BELOW row r is the "lane" band whose
    // `start` touches the node row end.
    let row_node_start = |r: i64| -> Option<i64> { row_y.get(&(2 * r + 1)).copied() };
    let lane_above_row = |r: i64| -> Option<(i64, i64)> {
        let ns = row_node_start(r)?;
        let b = row_bands.iter().find(|b| b.end == ns && b.kind == "lane")?;
        Some((b.start, b.end))
    };
    let lane_below_row = |r: i64| -> Option<(i64, i64)> {
        let ns = row_node_start(r)?;
        // node band starts at ns; find it to learn its end, then the lane there.
        let node = row_bands.iter().find(|b| b.start == ns && b.kind == "node")?;
        let b = row_bands
            .iter()
            .find(|b| b.start == node.end && b.kind == "lane")?;
        Some((b.start, b.end))
    };

    let n = groups.len();

    // Vertical borders (left/right x).
    for i in 0..n {
        let (c0, c1) = (groups[i].col0, groups[i].col1);
        // LEFT border in lane region 2*c0 - 1, packed on its RIGHT side:
        // deepest hugs the right nodes => nearest lane END (e-1).
        let lreg = 2 * c0 - 1;
        groups[i].x = if let Some((_s, e)) = col_lane_span(lreg) {
            let rank = rank_by_depth(groups, &groups[i].id, |g| 2 * g.col0 - 1 == lreg);
            e - 1 - rank
        } else {
            unreachable!("geometry always emits a lane adjacent to each node column")
        };
        // RIGHT border in lane region 2*c1 + 1, packed on its LEFT side:
        // deepest hugs the left nodes => nearest lane START (s).
        let rreg = 2 * c1 + 1;
        let right_x = if let Some((s, _e)) = col_lane_span(rreg) {
            let rank = rank_by_depth(groups, &groups[i].id, |g| 2 * g.col1 + 1 == rreg);
            s + rank
        } else {
            unreachable!("geometry always emits a lane adjacent to each node column")
        };
        groups[i].w = right_x - groups[i].x + 1;
    }

    // Horizontal borders (top/bottom y).
    for i in 0..n {
        let (r0, r1) = (groups[i].row0, groups[i].row1);
        // TOP border in the lane ABOVE node row r0, packed on its BOTTOM side:
        // deepest hugs the row below => nearest lane END.
        groups[i].y = if let Some((_s, e)) = lane_above_row(r0) {
            let rank = rank_by_depth(groups, &groups[i].id, |g| g.row0 == r0);
            e - 1 - rank
        } else {
            unreachable!("geometry always emits a lane adjacent to each node row")
        };
        // BOTTOM border in the lane BELOW node row r1, packed on its TOP side:
        // deepest hugs the row above => nearest lane START.
        let bot_y = if let Some((s, _e)) = lane_below_row(r1) {
            let rank = rank_by_depth(groups, &groups[i].id, |g| g.row1 == r1);
            s + rank
        } else {
            unreachable!("geometry always emits a lane adjacent to each node row")
        };
        groups[i].h = bot_y - groups[i].y + 1;
    }
}

/// Width of a vertical lane given its (left_rings, right_rings). With no rings
/// this is exactly LANE_MIN_W (backward compatible). Each populated side adds
/// its ring columns plus one PAD gap separating rings from the edge zone.
pub fn lane_width(left_rings: i64, right_rings: i64) -> i64 {
    use super::model::{GROUP_PAD, LANE_MIN_W};
    let left = if left_rings > 0 { left_rings + GROUP_PAD } else { 0 };
    let right = if right_rings > 0 { right_rings + GROUP_PAD } else { 0 };
    left + LANE_MIN_W + right
}

/// Height of a horizontal lane (same rule with LANE_MIN_H).
pub fn lane_height(top_rings: i64, bottom_rings: i64) -> i64 {
    use super::model::{GROUP_PAD, LANE_MIN_H};
    let top = if top_rings > 0 { top_rings + GROUP_PAD } else { 0 };
    let bottom = if bottom_rings > 0 { bottom_rings + GROUP_PAD } else { 0 };
    top + LANE_MIN_H + bottom
}
