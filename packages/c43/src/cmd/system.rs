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
    "puppeteer", "ava", "tap",
];

const TEST_NAME_PATTERNS: &[&str] = &["test", "e2e", "spec", "mock", "fixture"];

#[derive(Debug, PartialEq, Clone, Copy)]
enum PackageRole {
    ArchDefiner,
    Infrastructure,
    TestPackage,
    Library,
    Frontend,
    Client,
    ClientServer,
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

    // Packages imported by others (for library detection)
    let imported_packages: HashSet<&str> = project_data
        .iter()
        .flat_map(|pd| pd.imports.iter().map(|i| i.source.as_str()))
        .collect();

    // Identify direct consumers of architecture constructs
    let mut is_consumer: HashSet<&str> = HashSet::new();
    for pd in &project_data {
        let entries = find_consumers(&pd.imports, &exported_constructs);
        if !entries.is_empty() {
            is_consumer.insert(&pd.name);
        }
    }

    // Build import graph for transitive consumer detection
    let import_sources: HashMap<&str, HashSet<&str>> = project_data
        .iter()
        .map(|pd| {
            let sources: HashSet<&str> = pd.imports.iter().map(|i| i.source.as_str()).collect();
            (pd.name.as_str(), sources)
        })
        .collect();

    // Expand to transitive consumers
    loop {
        let mut new_consumers = Vec::new();
        for pd in &project_data {
            if is_consumer.contains(pd.name.as_str()) { continue; }
            if let Some(sources) = import_sources.get(pd.name.as_str()) {
                if sources.iter().any(|s| is_consumer.contains(s)) {
                    new_consumers.push(pd.name.as_str());
                }
            }
        }
        if new_consumers.is_empty() { break; }
        for name in new_consumers { is_consumer.insert(name); }
    }

    // Pre-calculate roles for all packages
    let mut package_roles: HashMap<&str, PackageRole> = HashMap::new();
    for pd in &project_data {
        let has_binds = !pd.binds.is_empty();
        let consumer = is_consumer.contains(pd.name.as_str());
        let role = classify_package(pd, has_binds, consumer, &imported_packages);
        package_roles.insert(pd.name.as_str(), role);
    }

    // Helper to skip test packages and IDs/Files
    let is_test_pattern = |s: &str| {
        let s_lower = s.to_lowercase();
        TEST_NAME_PATTERNS.iter().any(|p| s_lower.contains(p))
    };
    let is_test_pkg = |name: &str| package_roles.get(name) == Some(&PackageRole::TestPackage);

    // Collect all valid architectural IDs (Architectures and Constructs)
    let mut arch_ids = HashSet::new();
    let mut construct_id_by_var: HashMap<&str, HashMap<String, String>> = HashMap::new(); // pkg -> var -> id
    
    for pd in &project_data {
        if is_test_pkg(&pd.name) { continue; }
        let mut vars = HashMap::new();
        for c in &pd.constructs {
            if is_test_pattern(&c.id) || is_test_pattern(&c.file) { continue; }
            if c.class_name == "Architecture" {
                arch_ids.insert(c.id.clone());
            }
            if let Some(var_name) = &c.var_name {
                vars.insert(var_name.clone(), c.id.clone());
            }
        }
        construct_id_by_var.insert(pd.name.as_str(), vars);
    }

    // Map: Construct ID -> Architecture ID
    let mut construct_to_arch: HashMap<String, String> = HashMap::new();
    for pd in &project_data {
        if is_test_pkg(&pd.name) { continue; }
        let local_vars = construct_id_by_var.get(pd.name.as_str()).unwrap();
        let consumers = find_consumers(&pd.imports, &exported_constructs);

        for c in &pd.constructs {
            if c.class_name == "Architecture" { continue; }
            if is_test_pattern(&c.id) || is_test_pattern(&c.file) { continue; }
            
            if let Some(scope_var) = &c.scope_var {
                // Resolve scope_var to an Architecture ID
                let arch_id = if let Some(id) = local_vars.get(scope_var) {
                    if arch_ids.contains(id) { Some(id.clone()) } else { None }
                } else {
                    consumers.iter()
                        .find(|cons| &cons.name == scope_var && cons.construct_type == "Architecture")
                        .map(|cons| cons.construct_id.clone())
                };

                if let Some(aid) = arch_id {
                    construct_to_arch.insert(c.id.clone(), aid);
                }
            }
        }
    }

    // Find which Architectures exist (for linking consumers to backends)
    let mut seen_architectures = HashSet::new();
    let mut arch_by_definer: HashMap<&str, Vec<String>> = HashMap::new();
    for pd in &project_data {
        if is_test_pkg(&pd.name) { continue; }
        for c in &pd.constructs {
            if is_test_pattern(&c.id) || is_test_pattern(&c.file) { continue; }
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

    let arch_defining_packages: HashSet<&str> = arch_by_definer.keys().copied().collect();

    // Classify and emit consumer packages
    for pd in &project_data {
        if is_test_pkg(&pd.name) { continue; }
        if !is_consumer.contains(pd.name.as_str()) {
            let has_non_arch = pd.constructs.iter().any(|c| c.class_name != "Architecture");
            if !has_non_arch && pd.binds.is_empty() {
                continue;
            }
        }

        let role = package_roles.get(pd.name.as_str()).unwrap();

        match role {
            PackageRole::Frontend | PackageRole::Client => {
                let type_name = if *role == PackageRole::Frontend { "Frontend" } else { "Client" };
                doc.add_node(&pd.name, &pd.name, type_name, NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: None,
                    variable: None,
                });
                doc.add_relation(&system_name, "contains", &pd.name);

                let used_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                for arch_id in used_archs {
                    if !is_test_pattern(&arch_id) {
                        doc.add_relation(&pd.name, "uses", &arch_id);
                    }
                }
            }
            PackageRole::ClientServer | PackageRole::Infrastructure => {
                // Lift uses to implemented architectures
                let implemented_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                
                // 1. Lift relations to bound constructs
                for bind in &pd.binds {
                    if !bind.has_overloads {
                        // Find the construct ID for this variable
                        let mut target_id = None;
                        
                        // Check local constructs first
                        if let Some(local_vars) = construct_id_by_var.get(pd.name.as_str()) {
                            if let Some(id) = local_vars.get(&bind.component_var) {
                                target_id = Some(id.clone());
                            }
                        }
                        
                        // Check imports
                        if target_id.is_none() {
                            let consumers = find_consumers(&pd.imports, &exported_constructs);
                            if let Some(c) = consumers.iter().find(|c| c.name == bind.component_var) {
                                target_id = Some(c.construct_id.clone());
                            }
                        }

                        if let Some(tid) = target_id {
                            // Map construct ID to parent architecture ID
                            let final_target = construct_to_arch.get(&tid).unwrap_or(&tid);
                            
                            if is_test_pattern(final_target) { continue; }
                            
                            for arch_id in &implemented_archs {
                                // Skip if the target is an architecture ID (handled by transitive arch lift)
                                // or if it's the arch itself
                                if arch_id != final_target {
                                    doc.add_relation(arch_id, "uses", final_target);
                                }
                            }
                        }
                    }
                }
            }
            PackageRole::ArchDefiner => {
                // Link Architecture to other architectures it imports
                if let Some(my_ids) = arch_by_definer.get(pd.name.as_str()) {
                    let used_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                    for my_id in my_ids {
                        for used_id in &used_archs {
                            if my_id != used_id && !is_test_pattern(used_id) {
                                doc.add_relation(my_id, "uses", used_id);
                            }
                        }
                    }
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
    let name_lower = pd.name.to_lowercase();
    let path_lower = pd.path.to_lowercase();
    
    // 1. Test package (name pattern OR path pattern OR test framework deps)
    let name_matches_test = TEST_NAME_PATTERNS.iter().any(|p| name_lower.contains(p) || path_lower.contains(p));
    let has_test_deps = pd
        .meta
        .dev_dependencies
        .iter()
        .chain(pd.meta.dependencies.iter())
        .any(|d| TEST_FRAMEWORK_DEPS.contains(&d.as_str()));
    
    if (name_matches_test && has_test_deps) || path_lower.contains("/test/") || path_lower.contains("/tests/") || path_lower.contains("/spec/") {
        return PackageRole::TestPackage;
    }

    // 2. Architecture definer
    if pd.constructs.iter().any(|c| c.class_name == "Architecture") {
        return PackageRole::ArchDefiner;
    }

    // 3. ClientServer pattern: implements a backend by binding its components
    if is_consumer && (pd.name.contains("server") || pd.name.contains("worker") || pd.name.contains("adapter") || pd.name.contains("container") || pd.name.contains("docker")) {
        return PackageRole::ClientServer;
    }

    if has_binds {
        return PackageRole::Infrastructure;
    }

    if is_consumer && imported_packages.contains(pd.name.as_str()) {
        return PackageRole::Library;
    }

    let has_web_dep = pd
        .meta
        .dependencies
        .iter()
        .any(|d| WEB_FRAMEWORK_DEPS.contains(&d.as_str()));
    if has_web_dep || pd.meta.has_index_html || pd.meta.has_web_config {
        return PackageRole::Frontend;
    }

    PackageRole::Client
}

fn rel_path(file: &str, root: &str) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .trim_start_matches('/')
        .to_string()
}
