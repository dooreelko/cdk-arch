use std::collections::HashSet;
use std::path::Path;

use crate::analysis::{scan_projects, VarResolver};
use crate::cmd::system::frontend_and_client_packages;
use crate::extract::{ConstructInstance, RouteEntry};
use crate::model::{backend_uid, child_uid, C4Document, NodeAttributes};

pub fn run(root: &Path, container_filter: Option<&str>) -> C4Document {
    let mut doc = C4Document::new();

    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_str = root.to_str().unwrap_or("");

    let project_data = scan_projects(&root);

    // Only arch-defining packages — infra packages have no Architecture instance
    let arch_packages: Vec<_> = project_data
        .iter()
        .filter(|pd| pd.constructs.iter().any(|c| c.kind.is_architecture()))
        .collect();

    // Flatten constructs from arch packages
    let all_constructs: Vec<(&str, &ConstructInstance)> = arch_packages
        .iter()
        .flat_map(|pd| pd.constructs.iter().map(move |c| (pd.name.as_str(), c)))
        .collect();

    // Variable names are resolved per package (local definitions, then imports)
    let resolver = VarResolver::new(&project_data);

    // Flatten routes from arch packages: (route package, container id, routes)
    let all_routes: Vec<(&str, &str, &Vec<RouteEntry>)> = arch_packages
        .iter()
        .flat_map(|pd| {
            pd.routes
                .iter()
                .map(move |(cid, routes)| (pd.name.as_str(), cid.as_str(), routes))
        })
        .collect();

    // Uid of a construct: `backend:<id>` for Architectures, `<arch id>/<scope ids>/<id>` inside one
    let uid_of = |pkg: &str, c: &ConstructInstance| -> String { resolver.uid(pkg, c) };

    // Collect architectures and their container children (deferred emission)
    let architectures: Vec<(&str, &ConstructInstance)> = all_constructs
        .iter()
        .filter(|(_, c)| c.kind.is_architecture())
        .copied()
        .collect();

    let arch_data: Vec<(&str, &ConstructInstance, Vec<(&str, &ConstructInstance, String)>)> = architectures
        .iter()
        .map(|(pkg_name, arch)| {
            let children = all_constructs
                .iter()
                .filter(|(pkg, c)| {
                    !c.kind.is_architecture()
                        && !c.kind.is_function()
                        && c.scope_var
                            .as_deref()
                            .and_then(|sv| resolver.resolve(pkg, sv))
                            .map_or(false, |(_, parent)| std::ptr::eq(parent, *arch))
                })
                .map(|(pkg, c)| (*pkg, *c, child_uid(&arch.id, &c.id)))
                .collect();
            (*pkg_name, *arch, children)
        })
        .collect();

    // All container uids (needed for `uses` relation resolution)
    let container_uids: HashSet<&str> = arch_data
        .iter()
        .flat_map(|(_, _, children)| children.iter().map(|(_, _, uid)| uid.as_str()))
        .collect();

    // Resolve container filter: match a container by id, variable name or uid
    let filter_uid: Option<String> = container_filter.and_then(|filter| {
        arch_data
            .iter()
            .flat_map(|(_, _, children)| children.iter())
            .find(|(_, c, uid)| c.id == filter || c.var_name.as_deref() == Some(filter) || uid == filter)
            .map(|(_, _, uid)| uid.clone())
    });

    // Resolve every route handler once: (container uid, handler construct).
    // A route's container is the construct with the route's container id in the route's package.
    let routed_handlers: Vec<(String, &ConstructInstance)> = all_routes
        .iter()
        .filter_map(|&(pkg, cid, routes)| {
            // Only containers own routes; an Architecture may share the container's id
            let container_uid = arch_data
                .iter()
                .flat_map(|(_, _, children)| children.iter())
                .find(|(p, c, _)| *p == pkg && c.id == cid)
                .map(|(_, _, uid)| uid.clone())?;
            let resolver = &resolver;
            Some(routes.iter().filter_map(move |r| {
                resolver.resolve(pkg, &r.handler_var).map(|(_, h)| (container_uid.clone(), h))
            }))
        })
        .flatten()
        .collect();

    // Uids of constructs a function calls methods on (via called_vars)
    let called_uids = |pkg: &str, c: &ConstructInstance| -> Vec<String> {
        c.called_vars
            .iter()
            .filter_map(|v| resolver.resolve(pkg, v))
            .map(|(called_pkg, called)| uid_of(called_pkg, called))
            .collect()
    };
    let used_container_uids = |pkg: &str, c: &ConstructInstance| -> Vec<String> {
        called_uids(pkg, c)
            .into_iter()
            .filter(|uid| container_uids.contains(uid.as_str()))
            .collect()
    };

    // Collect Function/TBDFunction nodes with resolvable placement.
    // With a container filter, include functions that are:
    //   a. handled by the filter container (via routes), OR
    //   b. call the filter container (via called_vars)
    let functions: Vec<(&str, &ConstructInstance, String)> = all_constructs
        .iter()
        .copied()
        .filter(|(pkg, c)| {
            if !c.kind.is_function() {
                return false;
            }
            let has_scope = c
                .scope_var
                .as_deref()
                .map_or(false, |sv| resolver.resolve(pkg, sv).is_some());
            let is_routed = routed_handlers.iter().any(|(_, h)| std::ptr::eq(*h, *c));
            if !has_scope && !is_routed {
                return false;
            }
            if let Some(filter_uid) = &filter_uid {
                let handled = routed_handlers
                    .iter()
                    .any(|(cuid, h)| cuid == filter_uid && std::ptr::eq(*h, *c));
                let calls_container = used_container_uids(pkg, c).contains(filter_uid);
                return handled || calls_container;
            }
            true
        })
        .map(|(pkg, c)| (pkg, c, uid_of(pkg, c)))
        .collect();

    // In filtered mode: only emit the filter container itself, containers that handle
    // an included function, and containers that included functions use (called_vars).
    // All other sibling containers are excluded.
    let emit_container_uids: Option<HashSet<String>> = filter_uid.as_ref().map(|filter_uid| {
        let mut uids = HashSet::new();
        uids.insert(filter_uid.clone());
        for (cuid, handler) in &routed_handlers {
            if functions.iter().any(|(_, f, _)| std::ptr::eq(*f, *handler)) {
                uids.insert(cuid.clone());
            }
        }
        for (pkg, ci, _) in &functions {
            uids.extend(used_container_uids(pkg, ci));
        }
        uids
    });
    let emitted = |uid: &str| emit_container_uids.as_ref().map_or(true, |uids| uids.contains(uid));

    // Build System node
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "system".to_string());
    let system_uid = format!("system:{}", repo_name);
    doc.add_node(&system_uid, &repo_name, "System", NodeAttributes {
        project: None,
        file: None,
        variable: None,
        kind: None,
    });

    let mut emitted_uids: HashSet<String> = HashSet::new();

    // Emit Architecture (Backend) nodes and their container children.
    // In filtered mode, only emit containers in emit_container_ids.
    for (pkg_name, arch, children) in &arch_data {
        let rel_file = rel_path(&arch.file, root_str);
        let arch_uid = backend_uid(&arch.id);
        doc.add_node(&arch_uid, &arch.id, "Backend", NodeAttributes {
            project: Some(pkg_name.to_string()),
            file: Some(rel_file),
            variable: arch.var_name.clone(),
            kind: Some(arch.kind.as_str().to_string()),
        });
        doc.add_relation(&system_uid, "contains", &arch_uid);
        emitted_uids.insert(arch_uid.clone());

        for (child_pkg, child, child_uid) in children {
            if emitted(child_uid) {
                let rel_file = rel_path(&child.file, root_str);
                doc.add_node(child_uid, &child.id, &child.class_name, NodeAttributes {
                    project: Some(child_pkg.to_string()),
                    file: Some(rel_file),
                    variable: child.var_name.clone(),
                    kind: Some(child.kind.as_str().to_string()),
                });
                doc.add_relation(&arch_uid, "contains", child_uid);
                emitted_uids.insert(child_uid.clone());
            }
        }
    }

    // Add Function nodes
    for (_, ci, uid) in &functions {
        let rel_file = rel_path(&ci.file, root_str);
        doc.add_node(uid, &ci.id, &ci.class_name, NodeAttributes {
            project: None,
            file: Some(rel_file),
            variable: ci.var_name.clone(),
            kind: Some(ci.kind.as_str().to_string()),
        });
        emitted_uids.insert(uid.clone());
    }

    // Scope construct (usually the Architecture) contains Function (via scope_var)
    for (pkg, ci, uid) in &functions {
        if let Some((parent_pkg, parent)) = ci.scope_var.as_deref().and_then(|sv| resolver.resolve(pkg, sv)) {
            doc.add_relation(&uid_of(parent_pkg, parent), "contains", uid);
        }
    }

    // Container handles Function (via routes)
    for (container_uid, handler) in &routed_handlers {
        if let Some((_, _, uid)) = functions.iter().find(|(_, f, _)| std::ptr::eq(*f, *handler)) {
            doc.add_relation(container_uid, "handles", uid);
        }
    }

    // Function uses Container or another Function (via called_vars).
    // In filtered mode, containers are only targeted if emitted.
    for (pkg, ci, uid) in &functions {
        for called_uid in called_uids(pkg, ci) {
            if &called_uid != uid && emitted_uids.contains(&called_uid) {
                doc.add_relation(uid, "uses", &called_uid);
            }
        }
    }

    // Frontend/Client packages consume the system's architecture constructs, so they are
    // internal and shown. Each uses the view nodes its imports resolve to.
    // In filtered mode, only packages using an emitted node are shown.
    for (pd, type_name) in frontend_and_client_packages(&project_data) {
        let mut targets: Vec<String> = pd
            .imports
            .iter()
            .filter_map(|imp| resolver.resolve(&pd.name, &imp.local_name))
            .map(|(pkg, c)| uid_of(pkg, c))
            .filter(|uid| emitted_uids.contains(uid))
            .collect();
        targets.sort();
        targets.dedup();
        if filter_uid.is_some() && targets.is_empty() {
            continue;
        }
        doc.add_node(&pd.name, &pd.name, type_name, NodeAttributes {
            project: Some(pd.name.clone()),
            file: None,
            variable: None,
            kind: None,
        });
        doc.add_relation(&system_uid, "contains", &pd.name);
        for target in &targets {
            doc.add_relation(&pd.name, "uses", target);
        }
    }

    doc
}

fn rel_path(file: &str, root: &str) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .trim_start_matches('/')
        .to_string()
}
