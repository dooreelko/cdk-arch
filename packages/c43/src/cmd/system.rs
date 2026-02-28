use std::collections::HashSet;
use std::path::Path;

use crate::analysis::{build_exported_constructs_map, find_consumers, scan_projects};
use crate::model::{C4Document, NodeAttributes};

pub fn run(root: &Path) -> C4Document {
    let mut doc = C4Document::new();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // System node = the repo
    let repo_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "system".to_string());
    let system_name = format!("system:{}", repo_name);
    doc.add_node(&system_name, &repo_name, "System", NodeAttributes {
        project: None,
        file: None,
        variable: None,
    });

    let project_data = scan_projects(&root);
    let exported_constructs = build_exported_constructs_map(&project_data);

    let mut seen_architectures = HashSet::new();

    for pd in &project_data {
        // Architecture instances become subsystem nodes
        for c in &pd.constructs {
            if c.class_name == "Architecture" && seen_architectures.insert(c.id.clone()) {
                let rel_file = c.file.strip_prefix(root.to_str().unwrap_or(""))
                    .unwrap_or(&c.file)
                    .trim_start_matches('/')
                    .to_string();
                doc.add_node(&c.id, &c.id, "Architecture", NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: Some(rel_file),
                    variable: c.var_name.clone(),
                });
                doc.add_relation(&system_name, "contains", &c.id);
            }
        }

        // Each project with bindings or components (non-Architecture) is a package node
        let has_constructs = pd.constructs.iter().any(|c| c.class_name != "Architecture");
        let has_binds = !pd.binds.is_empty();
        let consumer_entries = find_consumers(&pd.imports, &exported_constructs);
        let is_consumer = !consumer_entries.is_empty();

        if has_constructs || has_binds || is_consumer {
            doc.add_node(&pd.name, &pd.name, "Package", NodeAttributes {
                project: Some(pd.name.clone()),
                file: None,
                variable: None,
            });
            doc.add_relation(&system_name, "contains", &pd.name);

            // Package -> Architecture containment (if it defines constructs scoped to an arch)
            let mut linked_archs = HashSet::new();
            for c in &pd.constructs {
                if c.class_name == "Architecture" && linked_archs.insert(c.id.clone()) {
                    doc.add_relation(&pd.name, "defines", &c.id);
                }
            }

            // Consumer relationships: package "consumes" source package
            let mut consumed_packages = HashSet::new();
            for entry in &consumer_entries {
                if consumed_packages.insert(entry.source_package.clone()) {
                    doc.add_relation(&pd.name, "consumes", &entry.source_package);
                }
            }
        }
    }

    doc
}
