use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analysis::{build_exported_constructs_map, find_consumers, scan_projects, ProjectData};
use crate::model::{C4Document, NodeAttributes};

const WEB_FRAMEWORK_DEPS: &[&str] = &[
    "react", "vue", "svelte", "angular", "@angular/core",
    "next", "nuxt", "solid-js", "preact", "lit",
];

const TEST_FRAMEWORK_DEPS: &[&str] = &[
    "jest", "mocha", "cucumber", "@cucumber/cucumber",
    "playwright", "@playwright/test", "cypress", "vitest",
    "puppeteer",
];

const TEST_NAME_PATTERNS: &[&str] = &["test", "e2e", "spec"];

#[derive(Debug, PartialEq)]
enum PackageRole {
    ArchDefiner,
    Infrastructure,
    TestPackage,
    Library,
    Frontend,
    Client,
}

pub fn run(root: &Path) -> C4Document {
    let mut doc = C4Document::new();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_str = root.to_str().unwrap_or("");

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

    // Build import graph: for each package, which packages does it import from?
    let import_sources: HashMap<&str, HashSet<&str>> = project_data
        .iter()
        .map(|pd| {
            let sources: HashSet<&str> = pd.imports.iter().map(|i| i.source.as_str()).collect();
            (pd.name.as_str(), sources)
        })
        .collect();

    // Identify direct consumers of architecture constructs
    let mut is_consumer: HashSet<&str> = HashSet::new();
    for pd in &project_data {
        let entries = find_consumers(&pd.imports, &exported_constructs);
        if !entries.is_empty() {
            is_consumer.insert(&pd.name);
        }
    }

    // Expand to transitive consumers: if A imports from B and B is a consumer, A is too
    loop {
        let mut new_consumers = Vec::new();
        for pd in &project_data {
            if is_consumer.contains(pd.name.as_str()) {
                continue;
            }
            if let Some(sources) = import_sources.get(pd.name.as_str()) {
                if sources.iter().any(|s| is_consumer.contains(s)) {
                    new_consumers.push(pd.name.as_str());
                }
            }
        }
        if new_consumers.is_empty() {
            break;
        }
        for name in new_consumers {
            is_consumer.insert(name);
        }
    }

    // Collect which packages are imported by others (for library detection)
    let imported_packages: HashSet<&str> = project_data
        .iter()
        .flat_map(|pd| pd.imports.iter().map(|i| i.source.as_str()))
        .collect();

    // Find which Architectures exist (for linking consumers to backends)
    let mut seen_architectures = HashSet::new();
    let mut arch_by_definer: HashMap<&str, Vec<String>> = HashMap::new();
    for pd in &project_data {
        for c in &pd.constructs {
            if c.class_name == "Architecture" && seen_architectures.insert(c.id.clone()) {
                let rel_file = rel_path(&c.file, root_str);
                doc.add_node(&c.id, &c.id, "Backend", NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: Some(rel_file),
                    variable: c.var_name.clone(),
                });
                doc.add_relation(&system_name, "contains", &c.id);
                arch_by_definer
                    .entry(&pd.name)
                    .or_default()
                    .push(c.id.clone());
            }
        }
    }

    // Build: for each consumer, find which Architecture(s) it ultimately uses
    // by tracing the import chain to an arch-defining package
    let arch_defining_packages: HashSet<&str> = arch_by_definer.keys().copied().collect();

    // Classify and emit consumer packages
    for pd in &project_data {
        if !is_consumer.contains(pd.name.as_str()) {
            // Not a consumer at all — check if it has constructs or binds
            let has_non_arch = pd.constructs.iter().any(|c| c.class_name != "Architecture");
            if !has_non_arch && pd.binds.is_empty() {
                continue;
            }
        }

        let has_binds = !pd.binds.is_empty();
        let consumer = is_consumer.contains(pd.name.as_str());
        let role = classify_package(pd, has_binds, consumer, &imported_packages);

        match role {
            PackageRole::Frontend | PackageRole::Client => {
                let type_name = if role == PackageRole::Frontend {
                    "Frontend"
                } else {
                    "Client"
                };
                doc.add_node(&pd.name, &pd.name, type_name, NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: None,
                    variable: None,
                });
                doc.add_relation(&system_name, "contains", &pd.name);

                // Link consumer to architectures via import chain
                let arch_ids =
                    find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                for arch_id in arch_ids {
                    doc.add_relation(&pd.name, "uses", &arch_id);
                }
            }
            _ => {}
        }
    }

    doc
}

/// Trace the import chain from a consumer package to find which Architecture(s) it uses.
fn find_used_architectures<'a>(
    pd: &ProjectData,
    all_projects: &'a [ProjectData],
    arch_packages: &HashSet<&str>,
    arch_by_definer: &HashMap<&str, Vec<String>>,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue: Vec<&str> = pd.imports.iter().map(|i| i.source.as_str()).collect();

    while let Some(source) = queue.pop() {
        if !visited.insert(source) {
            continue;
        }
        if let Some(arch_ids) = arch_by_definer.get(source) {
            result.extend(arch_ids.iter().cloned());
        } else {
            // Follow transitive imports
            for other_pd in all_projects {
                if other_pd.name == source {
                    for imp in &other_pd.imports {
                        if arch_packages.contains(imp.source.as_str())
                            || !visited.contains(imp.source.as_str())
                        {
                            queue.push(&imp.source);
                        }
                    }
                    break;
                }
            }
        }
    }

    result.sort();
    result.dedup();
    result
}

fn classify_package(
    pd: &ProjectData,
    has_binds: bool,
    is_consumer: bool,
    imported_packages: &HashSet<&str>,
) -> PackageRole {
    // 1. Architecture definer
    if pd.constructs.iter().any(|c| c.class_name == "Architecture") {
        return PackageRole::ArchDefiner;
    }

    // 2. Infrastructure (has bindings)
    if has_binds {
        return PackageRole::Infrastructure;
    }

    // 3. Test package (name pattern AND test framework deps)
    let name_lower = pd.name.to_lowercase();
    let name_matches_test = TEST_NAME_PATTERNS.iter().any(|p| name_lower.contains(p));
    let has_test_deps = pd
        .meta
        .dev_dependencies
        .iter()
        .chain(pd.meta.dependencies.iter())
        .any(|d| TEST_FRAMEWORK_DEPS.contains(&d.as_str()));
    if name_matches_test && has_test_deps {
        return PackageRole::TestPackage;
    }

    // 4. Library (consumed by other workspace packages)
    if is_consumer && imported_packages.contains(pd.name.as_str()) {
        return PackageRole::Library;
    }

    // 5. Frontend (web framework deps or web file heuristics)
    let has_web_dep = pd
        .meta
        .dependencies
        .iter()
        .any(|d| WEB_FRAMEWORK_DEPS.contains(&d.as_str()));
    if has_web_dep || pd.meta.has_index_html || pd.meta.has_web_config {
        return PackageRole::Frontend;
    }

    // 6. Client (default for remaining consumers)
    PackageRole::Client
}

fn rel_path(file: &str, root: &str) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .trim_start_matches('/')
        .to_string()
}
