use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analysis::scan_projects;
use crate::extract::ConstructInstance;
use crate::model::{C4Document, NodeAttributes};

pub fn run(root: &Path, container_filter: Option<&str>) -> C4Document {
    let mut doc = C4Document::new();

    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_str = root.to_str().unwrap_or("");

    let project_data = scan_projects(&root);

    // Only arch-defining packages — infra packages have no Architecture instance
    let arch_packages: Vec<_> = project_data
        .iter()
        .filter(|pd| pd.constructs.iter().any(|c| c.class_name == "Architecture"))
        .collect();

    // Flatten constructs from arch packages
    let all_constructs: Vec<(&str, &ConstructInstance)> = arch_packages
        .iter()
        .flat_map(|pd| pd.constructs.iter().map(move |c| (pd.name.as_str(), c)))
        .collect();

    // Build var_name -> construct map (first-seen wins, deduplicates cross-package)
    let mut var_to_construct: HashMap<&str, &ConstructInstance> = HashMap::new();
    for (_, c) in &all_constructs {
        if let Some(var) = c.var_name.as_deref() {
            var_to_construct.entry(var).or_insert(c);
        }
    }

    // Flatten routes from arch packages
    let all_routes: Vec<(&str, &Vec<crate::extract::RouteEntry>)> = arch_packages
        .iter()
        .flat_map(|pd| pd.routes.iter().map(|(cid, routes)| (cid.as_str(), routes)))
        .collect();

    // Collect architectures and their container children (deferred emission)
    let architectures: Vec<(&str, &ConstructInstance)> = all_constructs
        .iter()
        .filter(|(_, c)| c.class_name == "Architecture")
        .copied()
        .collect();

    let arch_data: Vec<(&str, &ConstructInstance, Vec<(&str, &ConstructInstance)>)> = architectures
        .iter()
        .filter_map(|(pkg_name, arch)| {
            let arch_var = arch.var_name.as_deref()?;
            let children: Vec<(&str, &ConstructInstance)> = all_constructs
                .iter()
                .filter(|(_, c)| {
                    c.scope_var.as_deref() == Some(arch_var)
                        && c.class_name != "Architecture"
                        && c.class_name != "Function"
                        && c.class_name != "TBDFunction"
                })
                .copied()
                .collect();
            Some((*pkg_name, *arch, children))
        })
        .collect();

    // All container ids (needed for `uses` relation resolution)
    let container_ids: HashSet<String> = arch_data
        .iter()
        .flat_map(|(_, _, children)| children.iter().map(|(_, c)| c.id.clone()))
        .collect();

    // Resolve container filter: find (container_id, container_var_name) for the given filter
    let filter_info: Option<(&str, &str)> = container_filter.and_then(|filter| {
        all_constructs
            .iter()
            .map(|(_, c)| *c)
            .find(|c| c.id == filter || c.var_name.as_deref() == Some(filter))
            .and_then(|c| c.var_name.as_deref().map(|v| (c.id.as_str(), v)))
    });

    // Handler var_names routed through the filter container
    let filter_handler_vars: Option<HashSet<&str>> = filter_info.map(|(cid, _)| {
        all_routes
            .iter()
            .filter(|(rid, _)| *rid == cid)
            .flat_map(|(_, routes)| routes.iter().map(|r| r.handler_var.as_str()))
            .collect()
    });

    // Collect Function/TBDFunction nodes with resolvable placement.
    // With a container filter, include functions that are:
    //   a. handled by the filter container (via routes), OR
    //   b. call the filter container's variable (via called_vars)
    let functions: Vec<&ConstructInstance> = all_constructs
        .iter()
        .map(|(_, c)| *c)
        .filter(|c| {
            if c.class_name != "Function" && c.class_name != "TBDFunction" {
                return false;
            }
            let has_scope = c
                .scope_var
                .as_deref()
                .map_or(false, |sv| var_to_construct.contains_key(sv));
            let is_routed = c.var_name.as_deref().map_or(false, |vn| {
                all_routes
                    .iter()
                    .any(|(_, routes)| routes.iter().any(|r| r.handler_var == vn))
            });
            if !has_scope && !is_routed {
                return false;
            }
            if let Some((_, filter_var)) = filter_info {
                let handled = filter_handler_vars
                    .as_ref()
                    .map_or(false, |vars| c.var_name.as_deref().map_or(false, |vn| vars.contains(vn)));
                let calls_container = c.called_vars.iter().any(|v| v == filter_var);
                return handled || calls_container;
            }
            true
        })
        .collect();

    let function_ids: HashSet<&str> = functions.iter().map(|c| c.id.as_str()).collect();

    // In filtered mode: only emit the filter container itself, containers that handle
    // an included function, and containers that included functions use (called_vars).
    // All other sibling containers are excluded.
    let emit_container_ids: Option<HashSet<&str>> = filter_info.map(|(filter_cid, _)| {
        let mut ids = HashSet::new();
        ids.insert(filter_cid);
        for (cid, routes) in &all_routes {
            for route in routes.iter() {
                if let Some(handler) = var_to_construct.get(route.handler_var.as_str()) {
                    if function_ids.contains(handler.id.as_str()) {
                        ids.insert(cid);
                    }
                }
            }
        }
        for ci in &functions {
            for called_var in &ci.called_vars {
                if let Some(called) = var_to_construct.get(called_var.as_str()) {
                    if container_ids.contains(&called.id) {
                        ids.insert(called.id.as_str());
                    }
                }
            }
        }
        ids
    });

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
    });

    // Emit Architecture (Backend) nodes and their container children.
    // In filtered mode, only emit containers in emit_container_ids.
    for (pkg_name, arch, children) in &arch_data {
        let rel_file = rel_path(&arch.file, root_str);
        doc.add_node(&arch.id, &arch.id, "Backend", NodeAttributes {
            project: Some(pkg_name.to_string()),
            file: Some(rel_file),
            variable: arch.var_name.clone(),
        });
        doc.add_relation(&system_uid, "contains", &arch.id);

        for (child_pkg, child) in children {
            if emit_container_ids.as_ref().map_or(true, |ids| ids.contains(child.id.as_str())) {
                let rel_file = rel_path(&child.file, root_str);
                doc.add_node(&child.id, &child.id, &child.class_name, NodeAttributes {
                    project: Some(child_pkg.to_string()),
                    file: Some(rel_file),
                    variable: child.var_name.clone(),
                });
                doc.add_relation(&arch.id, "contains", &child.id);
            }
        }
    }

    // Add Function nodes
    for ci in &functions {
        let rel_file = rel_path(&ci.file, root_str);
        doc.add_node(&ci.id, &ci.id, &ci.class_name, NodeAttributes {
            project: None,
            file: Some(rel_file),
            variable: ci.var_name.clone(),
        });
    }

    // Architecture contains Function (via scope_var)
    for ci in &functions {
        if let Some(sv) = ci.scope_var.as_deref() {
            if let Some(parent) = var_to_construct.get(sv) {
                doc.add_relation(&parent.id, "contains", &ci.id);
            }
        }
    }

    // Container handles Function (via routes)
    for (container_id, routes) in &all_routes {
        for route in routes.iter() {
            if let Some(handler) = var_to_construct.get(route.handler_var.as_str()) {
                if function_ids.contains(handler.id.as_str()) {
                    doc.add_relation(container_id, "handles", &handler.id);
                }
            }
        }
    }

    // Function uses Container (via called_vars → container variable).
    // In filtered mode, only emit if the target container is being emitted.
    for ci in &functions {
        for called_var in &ci.called_vars {
            if let Some(called) = var_to_construct.get(called_var.as_str()) {
                if container_ids.contains(&called.id) {
                    let emit = emit_container_ids
                        .as_ref()
                        .map_or(true, |ids| ids.contains(called.id.as_str()));
                    if emit {
                        doc.add_relation(&ci.id, "uses", &called.id);
                    }
                }
            }
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
