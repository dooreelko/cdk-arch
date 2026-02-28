use std::path::Path;

use crate::extract::{extract_from_file, BindCall, ConstructInstance};
use crate::model::{C4Document, NodeAttributes};
use crate::scan::find_ts_files;

pub fn run(arch_path: &Path, infra_path: &Path) -> C4Document {
    let mut doc = C4Document::new();

    let arch_path = arch_path
        .canonicalize()
        .unwrap_or_else(|_| arch_path.to_path_buf());
    let infra_path = infra_path
        .canonicalize()
        .unwrap_or_else(|_| infra_path.to_path_buf());

    // Extract constructs from architecture
    let arch_files = find_ts_files(&arch_path);
    let mut arch_constructs: Vec<ConstructInstance> = Vec::new();
    for file in &arch_files {
        let extracts = extract_from_file(file);
        arch_constructs.extend(extracts.constructs);
    }

    // Build var_name -> construct for arch
    let arch_var_map: std::collections::HashMap<String, &ConstructInstance> = arch_constructs
        .iter()
        .filter_map(|c| c.var_name.as_ref().map(|v| (v.clone(), c)))
        .collect();

    // Extract binds from infra
    let infra_files = find_ts_files(&infra_path);
    let mut all_binds: Vec<BindCall> = Vec::new();
    let mut infra_imports: Vec<(String, String)> = Vec::new();

    for file in &infra_files {
        let extracts = extract_from_file(file);
        all_binds.extend(extracts.binds);
        for import in &extracts.imports {
            infra_imports.push((import.local_name.clone(), import.source.clone()));
        }
    }

    let arch_str = arch_path.to_str().unwrap_or("");

    // For each bind call, resolve the component and create deployment nodes
    let mut seen_endpoints = std::collections::HashSet::new();
    let mut seen_components = std::collections::HashSet::new();

    for bind in &all_binds {
        // Try to resolve the component var to an architecture construct
        let component = arch_var_map.get(&bind.component_var);

        let component_id = component
            .map(|c| c.id.clone())
            .unwrap_or_else(|| bind.component_var.clone());

        let component_type = component
            .map(|c| c.class_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Add the component node (deduplicated)
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

        // Create a deployment endpoint node from baseUrl
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

        // Add overload relations
        for key in &bind.overload_keys {
            doc.add_relation(&component_id, "binds", key);
        }
    }

    doc
}
