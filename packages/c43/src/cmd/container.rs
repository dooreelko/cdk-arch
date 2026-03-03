use std::collections::HashMap;
use std::path::Path;

use crate::analysis::scan_projects;
use crate::extract::ConstructInstance;
use crate::model::{C4Document, NodeAttributes};

pub fn run(root: &Path) -> C4Document {
    // Start with the full system-level document (Backend, Frontend, Client nodes + relations)
    let mut doc = super::system::run(root);

    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_str = root.to_str().unwrap_or("");

    let project_data = scan_projects(&root);

    // Flatten constructs and routes across all projects, tracking package origin
    let mut all_constructs: Vec<(&str, &ConstructInstance)> = Vec::new();
    let mut all_routes = Vec::new();

    for pd in &project_data {
        for c in &pd.constructs {
            all_constructs.push((&pd.name, c));
        }
        all_routes.extend(&pd.routes);
    }

    // Build var_name -> (pkg_name, construct) map
    let var_to_construct: HashMap<&str, (&str, &ConstructInstance)> = all_constructs
        .iter()
        .filter_map(|&(pkg, c)| c.var_name.as_deref().map(|v| (v, (pkg, c))))
        .collect();

    // Find Architecture instances
    let architectures: Vec<(&str, &ConstructInstance)> = all_constructs
        .iter()
        .filter(|(_, c)| c.class_name == "Architecture")
        .copied()
        .collect();

    for (_, arch) in &architectures {
        let arch_var = match &arch.var_name {
            Some(v) => v.as_str(),
            None => continue,
        };

        // Find direct container children: constructs scoped to this architecture,
        // excluding individual Function instances (which belong in the component diagram)
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

        for (child_pkg, child) in &children {
            let rel_file = rel_path(&child.file, root_str);
            doc.add_node(&child.id, &child.id, &child.class_name, NodeAttributes {
                project: Some(child_pkg.to_string()),
                file: Some(rel_file),
                variable: child.var_name.clone(),
            });
            doc.add_relation(&arch.id, "contains", &child.id);

            // Link routes to container-level handlers only (not Function instances).
            // For Function handlers, derive container-to-container "uses" relations
            // from the variables the handler calls methods on.
            for (container_id, routes) in &all_routes {
                if container_id == &child.id {
                    for route in routes {
                        if let Some((_, handler)) = var_to_construct.get(route.handler_var.as_str()) {
                            if handler.class_name != "Function" && handler.class_name != "TBDFunction" {
                                doc.add_relation(&child.id, "routes to", &handler.id);
                            } else {
                                for called_var in &handler.called_vars {
                                    if let Some((_, called)) = var_to_construct.get(called_var.as_str()) {
                                        if called.id != child.id
                                            && children.iter().any(|(_, c)| c.id == called.id)
                                        {
                                            doc.add_relation(&child.id, "uses", &called.id);
                                        }
                                    }
                                }
                            }
                        }
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
