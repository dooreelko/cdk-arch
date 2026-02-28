use std::collections::HashMap;
use std::path::Path;

use crate::analysis::scan_directory;
use crate::extract::ConstructInstance;
use crate::model::{C4Document, NodeAttributes};

pub fn run(pkg_path: &Path, container_filter: Option<&str>) -> C4Document {
    let mut doc = C4Document::new();
    let pkg_path = pkg_path
        .canonicalize()
        .unwrap_or_else(|_| pkg_path.to_path_buf());
    let pkg_str = pkg_path.to_str().unwrap_or("");

    let pd = scan_directory(&pkg_path);

    // Build var_name -> construct map
    let var_to_construct: HashMap<&str, &ConstructInstance> = pd
        .constructs
        .iter()
        .filter_map(|c| c.var_name.as_deref().map(|v| (v, c)))
        .collect();

    // Filter by container name if specified
    let constructs_to_show: Vec<&ConstructInstance> = if let Some(filter) = container_filter {
        pd.constructs
            .iter()
            .filter(|c| {
                c.id == filter
                    || c.scope_var.as_deref() == Some(filter)
                    || c.var_name.as_deref() == Some(filter)
            })
            .collect()
    } else {
        pd.constructs.iter().collect()
    };

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

    // 1. Scope containment
    for ci in &constructs_to_show {
        if let Some(scope) = &ci.scope_var {
            if let Some(parent) = var_to_construct.get(scope.as_str()) {
                if constructs_to_show.iter().any(|c| c.id == parent.id) {
                    doc.add_relation(&parent.id, "contains", &ci.id);
                }
            }
        }
    }

    // 2. Routes
    for (container_id, routes) in &pd.routes {
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

    // 3. Function dependencies
    for ci in &constructs_to_show {
        if ci.class_name == "Function" || ci.class_name == "TBDFunction" {
            if let Some(scope) = &ci.scope_var {
                if let Some(parent) = var_to_construct.get(scope.as_str()) {
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
