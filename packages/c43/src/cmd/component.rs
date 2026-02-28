use std::path::Path;

use crate::extract::{extract_from_file, ConstructInstance, RouteEntry};
use crate::model::{C4Document, NodeAttributes};
use crate::scan::find_ts_files;

pub fn run(pkg_path: &Path, container_filter: Option<&str>) -> C4Document {
    let mut doc = C4Document::new();
    let pkg_path = pkg_path
        .canonicalize()
        .unwrap_or_else(|_| pkg_path.to_path_buf());

    let ts_files = find_ts_files(&pkg_path);

    let mut all_constructs: Vec<ConstructInstance> = Vec::new();
    let mut all_routes: Vec<(String, Vec<RouteEntry>)> = Vec::new();

    for file in &ts_files {
        let extracts = extract_from_file(file);
        all_constructs.extend(extracts.constructs);
        all_routes.extend(extracts.routes);
    }

    // Build var_name -> construct map
    let var_to_construct: std::collections::HashMap<&str, &ConstructInstance> = all_constructs
        .iter()
        .filter_map(|c| c.var_name.as_deref().map(|v| (v, c)))
        .collect();

    // Filter by container name if specified
    let constructs_to_show: Vec<&ConstructInstance> = if let Some(filter) = container_filter {
        all_constructs
            .iter()
            .filter(|c| {
                c.id == filter
                    || c.scope_var.as_deref() == Some(filter)
                    || c.var_name.as_deref() == Some(filter)
            })
            .collect()
    } else {
        all_constructs.iter().collect()
    };

    let pkg_str = pkg_path.to_str().unwrap_or("");

    for ci in &constructs_to_show {
        let rel_file = ci.file.strip_prefix(pkg_str)
            .unwrap_or(&ci.file)
            .trim_start_matches('/')
            .to_string();
        doc.add_node(&ci.id, &ci.id, &ci.class_name, NodeAttributes {
            project: None,
            file: Some(rel_file),
            variable: ci.var_name.clone(),
        });
    }

    // Add relations
    // 1. Scope containment
    for ci in &constructs_to_show {
        if let Some(scope) = &ci.scope_var {
            if let Some(parent) = var_to_construct.get(scope.as_str()) {
                // Only add if parent is also in our view
                if constructs_to_show.iter().any(|c| c.id == parent.id) {
                    doc.add_relation(&parent.id, "contains", &ci.id);
                }
            }
        }
    }

    // 2. Routes
    for (container_id, routes) in &all_routes {
        if !constructs_to_show.iter().any(|c| &c.id == container_id) {
            continue;
        }
        for route in routes {
            if let Some(handler) = var_to_construct.get(route.handler_var.as_str()) {
                if constructs_to_show.iter().any(|c| c.id == handler.id) {
                    doc.add_relation(container_id, "handles", &handler.id);
                }
            }
        }
    }

    // 3. Function dependencies — check if any function's scope is a construct (uses relationship)
    for ci in &constructs_to_show {
        if ci.class_name == "Function" || ci.class_name == "TBDFunction" {
            if let Some(scope) = &ci.scope_var {
                if let Some(parent) = var_to_construct.get(scope.as_str()) {
                    // If the parent is not an Architecture, this function "uses" it
                    if parent.class_name != "Architecture"
                        && constructs_to_show.iter().any(|c| c.id == parent.id)
                    {
                        doc.add_relation(&ci.id, "uses", &parent.id);
                    }
                }
            }
        }
    }

    doc
}
