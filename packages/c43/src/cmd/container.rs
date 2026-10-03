use std::path::Path;

use crate::analysis::{scan_projects, VarResolver};
use crate::extract::{ConstructInstance, RouteEntry};
use crate::model::{backend_uid, child_uid, C4Document, NodeAttributes};

pub fn run(root: &Path) -> C4Document {
    // Start with the full system-level document (Backend, Frontend, Client nodes + relations)
    let mut doc = super::system::run(root);

    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_str = root.to_str().unwrap_or("");

    let project_data = scan_projects(&root);

    // Flatten constructs and routes across all projects, tracking package origin
    let mut all_constructs: Vec<(&str, &ConstructInstance)> = Vec::new();
    let mut all_routes: Vec<(&str, &(String, Vec<RouteEntry>))> = Vec::new();

    for pd in &project_data {
        for c in &pd.constructs {
            all_constructs.push((&pd.name, c));
        }
        all_routes.extend(pd.routes.iter().map(|r| (pd.name.as_str(), r)));
    }

    // Variable names are resolved per package (local definitions, then imports)
    let resolver = VarResolver::new(&project_data);

    // Shown containers: direct non-function children of each Architecture
    let containers: Vec<(&str, &ConstructInstance, &ConstructInstance, String)> = all_constructs
        .iter()
        .filter(|(_, c)| !c.kind.is_architecture() && !c.kind.is_function())
        .filter_map(|&(pkg, c)| {
            let (_, parent) = resolver.resolve(pkg, c.scope_var.as_deref()?)?;
            parent
                .kind
                .is_architecture()
                .then(|| (pkg, c, parent, child_uid(&parent.id, &c.id)))
        })
        .collect();

    for (pkg, c, arch, uid) in &containers {
        doc.add_node(uid, &c.id, &c.class_name, NodeAttributes {
            project: Some(pkg.to_string()),
            file: Some(rel_path(&c.file, root_str)),
            variable: c.var_name.clone(),
            kind: Some(c.kind.as_str().to_string()),
        });
        doc.add_relation(&backend_uid(&arch.id), "contains", uid);
    }

    // Node a construct is drawn as in this view: itself if it is a shown container, else the
    // nearest shown container or Architecture enclosing it (via scope links).
    let shown_as = |pkg: &str, c: &ConstructInstance| -> Option<String> {
        let (mut pkg, mut c) = (pkg.to_string(), c);
        loop {
            if c.kind.is_architecture() {
                return Some(backend_uid(&c.id));
            }
            if let Some((_, _, _, uid)) = containers.iter().find(|(_, x, _, _)| std::ptr::eq(*x, c)) {
                return Some(uid.clone());
            }
            let (p, parent) = resolver.resolve(&pkg, c.scope_var.as_deref()?)?;
            (pkg, c) = (p.to_string(), parent);
        }
    };

    // Routes: (shown node of the routing container, handler package, handler).
    // A route belongs to the non-function construct with the route's container id declared in
    // the route's package (never an Architecture, which may share the id).
    let routes: Vec<(String, &str, &ConstructInstance)> = all_routes
        .iter()
        .filter_map(|(pkg, (cid, entries))| {
            let (_, container) = all_constructs.iter().find(|(p, c)| {
                p == pkg && &c.id == cid && !c.kind.is_architecture() && !c.kind.is_function()
            })?;
            let host = shown_as(pkg, container)?;
            let resolver = &resolver;
            Some(entries.iter().filter_map(move |r| {
                resolver
                    .resolve(pkg, &r.handler_var)
                    .map(|(hpkg, h)| (host.clone(), hpkg, h))
            }))
        })
        .flatten()
        .collect();

    // Shown nodes a construct acts through: for a function, the containers routing it, or
    // (when unrouted) the node enclosing it; for anything else, the node it is drawn as.
    let acting_as = |pkg: &str, c: &ConstructInstance| -> Vec<String> {
        if c.kind.is_function() {
            let mut hosts: Vec<String> = routes
                .iter()
                .filter(|(_, _, h)| std::ptr::eq(*h, c))
                .map(|(host, _, _)| host.clone())
                .collect();
            if hosts.is_empty() {
                hosts.extend(c.scope_var.as_deref().and_then(|sv| resolver.resolve(pkg, sv)).and_then(|(p, parent)| shown_as(p, parent)));
            }
            hosts
        } else {
            shown_as(pkg, c).into_iter().collect()
        }
    };

    // Container routes to a non-function handler
    for (host, hpkg, handler) in &routes {
        if !handler.kind.is_function() {
            if let Some(target) = shown_as(hpkg, handler) {
                if &target != host {
                    doc.add_relation(host, "routes to", &target);
                }
            }
        }
    }

    // Function calls collapsed to container level, across Architectures:
    // every node a function acts through uses every node the called construct acts through.
    for &(pkg, f) in all_constructs.iter().filter(|(_, c)| c.kind.is_function()) {
        let sources = acting_as(pkg, f);
        if sources.is_empty() {
            continue;
        }
        for called_var in &f.called_vars {
            let Some((cpkg, called)) = resolver.resolve(pkg, called_var) else { continue };
            for target in acting_as(cpkg, called) {
                for source in &sources {
                    if source != &target {
                        doc.add_relation(source, "uses", &target);
                    }
                }
            }
        }
    }

    doc
}

fn rel_path<'a>(file: &'a str, root: &str) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .trim_start_matches('/')
        .to_string()
}
