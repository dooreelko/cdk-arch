use crate::model::Graph;

/// the level above every top-level node and group
pub const ROOT: &str = "";

pub fn parent_of<'a>(g: &'a Graph, id: &str) -> &'a str {
    g.parent.get(id).map_or(ROOT, String::as_str)
}

/// groups holding `id`, innermost first; a group holds itself
pub fn chain(g: &Graph, id: &str) -> Vec<String> {
    let mut out: Vec<String> = if g.group_ids.contains(id) { vec![id.to_string()] } else { vec![] };
    let mut c = g.parent.get(id);
    while let Some(p) = c {
        out.push(p.clone());
        c = g.parent.get(p);
    }
    out
}

/// `grp` is a proper ancestor of `id`
pub fn contains(g: &Graph, grp: &str, id: &str) -> bool {
    id != grp && chain(g, id).iter().any(|x| x == grp)
}

/// nesting depth: top-level = 0
pub fn depth(g: &Graph, id: &str) -> usize {
    chain(g, id).len() - usize::from(g.group_ids.contains(id))
}

/// innermost group holding both ends of an edge, else ROOT
pub fn owner(g: &Graph, from: &str, to: &str) -> String {
    let b = chain(g, to);
    chain(g, from).into_iter().find(|x| b.contains(x)).unwrap_or_else(|| ROOT.to_string())
}

/// the direct child of `level` that is or holds `id`; None when `id` is `level` or outside it
pub fn child_of(g: &Graph, level: &str, id: &str) -> Option<String> {
    if id == level {
        return None;
    }
    let mut c = Some(id);
    while let Some(x) = c {
        if parent_of(g, x) == level {
            return Some(x.to_string());
        }
        c = g.parent.get(x).map(String::as_str);
    }
    None
}

/// a group pointing at one of its own members: the edge starts inside the group's left border
pub fn is_inner(g: &Graph, from: &str, to: &str) -> bool {
    contains(g, from, to)
}
