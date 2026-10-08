use crate::model::{Graph, Kind, Rel};
use indexmap::{IndexMap, IndexSet};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    pub columns: Vec<Vec<String>>,
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn reaches(succ: &IndexMap<String, Vec<String>>, from: &str, to: &str) -> bool {
    let mut seen: IndexSet<&str> = IndexSet::new();
    let mut stack = vec![from];
    while let Some(v) = stack.pop() {
        if v == to {
            return true;
        }
        if seen.insert(v) {
            stack.extend(succ[v].iter().map(String::as_str));
        }
    }
    false
}

pub fn skeleton(g: &Graph) -> Skeleton {
    let ids: Vec<String> = g.nodes.iter().map(|n| n.id.clone()).filter(|id| g.data_nodes.contains(id)).collect();
    let mut succ: IndexMap<String, Vec<String>> = ids.iter().map(|id| (id.clone(), vec![])).collect();
    let mut has_in: IndexSet<String> = IndexSet::new();
    for e in g.edges.iter().filter(|e| e.kind == Kind::Data) {
        succ.get_mut(&e.from).unwrap().push(e.to.clone());
        has_in.insert(e.to.clone());
    }

    // placement hints: the leftmost node takes no incoming layering edge; left/right hints are extra layering edges unless they close a cycle
    for h in crate::hints::priority_first(g) {
        match (h.rel, &h.b) {
            (Rel::Leftmost, _) if succ.contains_key(&h.a) => {
                succ.values_mut().for_each(|ws| ws.retain(|w| *w != h.a));
                has_in.shift_remove(&h.a);
            }
            (Rel::LeftOf | Rel::RightOf, Some(b)) if succ.contains_key(&h.a) && succ.contains_key(b) => {
                let (from, to) = if h.rel == Rel::LeftOf { (&h.a, b) } else { (b, &h.a) };
                if !reaches(&succ, to, from) && !succ[from].contains(to) {
                    succ.get_mut(from).unwrap().push(to.clone());
                    has_in.insert(to.clone());
                }
            }
            _ => {}
        }
    }

    // break cycles: DFS from true sources first, back edges are ignored for layering
    let mut back: IndexSet<(String, String)> = IndexSet::new();
    let mut state: IndexMap<String, u8> = IndexMap::new(); // 1 = on stack, 2 = done
    fn dfs(v: &str, succ: &IndexMap<String, Vec<String>>, state: &mut IndexMap<String, u8>, back: &mut IndexSet<(String, String)>) {
        state.insert(v.to_string(), 1);
        for w in &succ[v] {
            match state.get(w).copied().unwrap_or(0) {
                1 => {
                    back.insert((v.to_string(), w.clone()));
                }
                0 => dfs(w, succ, state, back),
                _ => {}
            }
        }
        state.insert(v.to_string(), 2);
    }
    for id in ids.iter().filter(|id| !has_in.contains(*id)).chain(ids.iter()) {
        if state.get(id).copied().unwrap_or(0) == 0 {
            dfs(id, &succ, &mut state, &mut back);
        }
    }

    let fwd = |v: &str| -> Vec<String> {
        succ[v].iter().filter(|w| !back.contains(&(v.to_string(), (*w).clone()))).cloned().collect()
    };
    let mut preds: IndexMap<String, Vec<String>> = ids.iter().map(|id| (id.clone(), vec![])).collect();
    for v in &ids {
        for w in fwd(v) {
            preds.get_mut(&w).unwrap().push(v.clone());
        }
    }

    // longest-path layering (Kahn, input order)
    let mut indeg: IndexMap<String, usize> = ids.iter().map(|id| (id.clone(), preds[id].len())).collect();
    let mut layer: IndexMap<String, i32> = ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut queue: VecDeque<String> = ids.iter().filter(|id| indeg[*id] == 0).cloned().collect();
    while let Some(v) = queue.pop_front() {
        for w in fwd(&v) {
            let lv = layer[&v];
            let lw = layer.get_mut(&w).unwrap();
            *lw = (*lw).max(lv + 1);
            let d = indeg.get_mut(&w).unwrap();
            *d -= 1;
            if *d == 0 {
                queue.push_back(w);
            }
        }
    }

    // a node with no data predecessor but an incoming nf edge is not a source: nf runs downward, so it joins its
    // nf predecessor's column and whatever it feeds moves on to keep data edges pointing right
    let lifted: Vec<&String> = ids.iter().filter(|id| preds[*id].is_empty()).collect();
    for _ in 0..ids.len() {
        let mut changed = false;
        for id in &lifted {
            let into = g.edges.iter().filter(|e| e.kind == Kind::Nf && e.to == **id).filter_map(|e| layer.get(&e.from)).max().copied();
            if let Some(l) = into.filter(|l| *l > layer[*id]) {
                layer.insert((*id).clone(), l);
                let mut stack = vec![(*id).clone()];
                while let Some(v) = stack.pop() {
                    for w in fwd(&v) {
                        if layer[&w] <= layer[&v] {
                            layer.insert(w.clone(), layer[&v] + 1);
                            stack.push(w);
                        }
                    }
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // same-col hints: pull the second node into the first one's layer when its edges allow
    for h in crate::hints::priority_last(g).into_iter().filter(|h| h.rel == Rel::SameCol) {
        let Some(b) = &h.b else { continue };
        let (Some(la), true) = (layer.get(&h.a).copied(), layer.contains_key(b)) else { continue };
        let fits = preds[b].iter().all(|p| layer[p] < la) && fwd(b).iter().all(|w| layer[w] > la);
        if fits {
            layer.insert(b.clone(), la);
        }
    }

    // barycenter order per layer; layers taller than `cap` wrap into extra columns
    let cap = 3usize.max((g.nodes.len() as f64).sqrt().ceil() as usize);
    let mut pos: IndexMap<String, f64> = IndexMap::new();
    let mut columns: Vec<Vec<String>> = Vec::new();
    let max_layer = layer.values().copied().fold(-1, i32::max);
    for l in 0..=max_layer {
        let mut keyed: Vec<(String, usize, f64)> = ids
            .iter()
            .filter(|id| layer[*id] == l)
            .enumerate()
            .map(|(i, id)| {
                let ps: Vec<f64> = preds[id].iter().filter_map(|p| pos.get(p).copied()).collect();
                (id.clone(), i, if ps.is_empty() { 0.0 } else { mean(&ps) })
            })
            .collect();
        keyed.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap().then(a.1.cmp(&b.1)));
        for chunk in keyed.chunks(cap) {
            let chunk: Vec<String> = chunk.iter().map(|x| x.0.clone()).collect();
            chunk.iter().enumerate().for_each(|(i, id)| {
                pos.insert(id.clone(), i as f64 - (chunk.len() as f64 - 1.0) / 2.0);
            });
            columns.push(chunk);
        }
    }
    Skeleton { columns }
}
