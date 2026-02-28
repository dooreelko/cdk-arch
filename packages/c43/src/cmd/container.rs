use std::path::Path;

use crate::extract::{extract_from_file, ConstructInstance, RouteEntry};
use crate::model::{C4Document, NodeAttributes};
use crate::scan::{find_ts_files, find_workspace_packages};

pub fn run(root: &Path) -> C4Document {
    let mut doc = C4Document::new();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let packages = find_workspace_packages(&root);

    let mut all_constructs: Vec<(String, ConstructInstance)> = Vec::new();
    let mut all_routes: Vec<(String, Vec<RouteEntry>)> = Vec::new();

    for (pkg_name, pkg_path) in &packages {
        let ts_files = find_ts_files(pkg_path);
        for file in &ts_files {
            let extracts = extract_from_file(file);
            for c in extracts.constructs {
                all_constructs.push((pkg_name.clone(), c));
            }
            all_routes.extend(extracts.routes);
        }
    }

    // Find Architecture instances — these are top-level
    let architectures: Vec<&(String, ConstructInstance)> = all_constructs
        .iter()
        .filter(|(_, c)| c.class_name == "Architecture")
        .collect();

    // Build var_name -> (pkg, construct) map
    let var_to_construct: std::collections::HashMap<&str, &(String, ConstructInstance)> = all_constructs
        .iter()
        .filter_map(|entry| entry.1.var_name.as_deref().map(|v| (v, entry)))
        .collect();

    let root_str = root.to_str().unwrap_or("");

    for (pkg_name, arch) in &architectures {
        let rel_file = arch.file.strip_prefix(root_str)
            .unwrap_or(&arch.file)
            .trim_start_matches('/')
            .to_string();
        doc.add_node(&arch.id, &arch.id, "Architecture", NodeAttributes {
            project: Some(pkg_name.clone()),
            file: Some(rel_file),
            variable: arch.var_name.clone(),
        });

        let arch_var = match &arch.var_name {
            Some(v) => v.as_str(),
            None => continue,
        };

        // Find direct children: constructs whose scope_var matches the architecture's var_name
        let children: Vec<&(String, ConstructInstance)> = all_constructs
            .iter()
            .filter(|(_, c)| c.scope_var.as_deref() == Some(arch_var) && c.class_name != "Architecture")
            .collect();

        for (child_pkg, child) in &children {
            let rel_file = child.file.strip_prefix(root_str)
                .unwrap_or(&child.file)
                .trim_start_matches('/')
                .to_string();
            doc.add_node(&child.id, &child.id, &child.class_name, NodeAttributes {
                project: Some(child_pkg.clone()),
                file: Some(rel_file),
                variable: child.var_name.clone(),
            });
            doc.add_relation(&arch.id, "contains", &child.id);

            // If this is an ApiContainer, find its routes and link handlers
            for (container_id, routes) in &all_routes {
                if container_id == &child.id {
                    for route in routes {
                        if let Some((_, handler_construct)) = var_to_construct.get(route.handler_var.as_str()).map(|e| (&e.0, &e.1)) {
                            doc.add_relation(&child.id, "routes to", &handler_construct.id);
                        }
                    }
                }
            }
        }
    }

    doc
}
