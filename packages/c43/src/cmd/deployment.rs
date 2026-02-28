use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analysis::scan_directory;
use crate::extract::ConstructInstance;
use crate::model::{C4Document, NodeAttributes};

pub fn run(arch_path: &Path, infra_path: &Path) -> C4Document {
    let mut doc = C4Document::new();

    let arch_path = arch_path
        .canonicalize()
        .unwrap_or_else(|_| arch_path.to_path_buf());
    let infra_path = infra_path
        .canonicalize()
        .unwrap_or_else(|_| infra_path.to_path_buf());

    let arch_data = scan_directory(&arch_path);
    let infra_data = scan_directory(&infra_path);

    let arch_str = arch_path.to_str().unwrap_or("");

    // Build var_name -> construct for arch
    let arch_var_map: HashMap<&str, &ConstructInstance> = arch_data
        .constructs
        .iter()
        .filter_map(|c| c.var_name.as_deref().map(|v| (v, c)))
        .collect();

    let mut seen_endpoints = HashSet::new();
    let mut seen_components = HashSet::new();

    for bind in &infra_data.binds {
        let component = arch_var_map.get(bind.component_var.as_str());

        let component_id = component
            .map(|c| c.id.clone())
            .unwrap_or_else(|| bind.component_var.clone());

        let component_type = component
            .map(|c| c.class_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        if seen_components.insert(component_id.clone()) {
            let attrs = match component {
                Some(c) => {
                    let rel_file = c.file.strip_prefix(arch_str)
                        .unwrap_or(&c.file)
                        .trim_start_matches('/')
                        .to_string();
                    NodeAttributes {
                        project: None,
                        file: Some(rel_file),
                        variable: c.var_name.clone(),
                    }
                }
                None => NodeAttributes {
                    project: None,
                    file: None,
                    variable: Some(bind.component_var.clone()),
                },
            };
            doc.add_node(&component_id, &component_id, &component_type, attrs);
        }

        if let Some(base_url) = &bind.base_url {
            let endpoint_id = base_url.clone();
            if seen_endpoints.insert(endpoint_id.clone()) {
                doc.add_node(&endpoint_id, base_url, "Endpoint", NodeAttributes {
                    project: None,
                    file: Some(bind.file.clone()),
                    variable: None,
                });
            }
            doc.add_relation(&component_id, "deployed on", &endpoint_id);
        }

        for key in &bind.overload_keys {
            doc.add_relation(&component_id, "binds", key);
        }
    }

    doc
}
