use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analysis::{build_exported_constructs_map, find_consumers, scan_projects, ProjectData, VarResolver};
use crate::extract::ConstructInstance;
use crate::model::{backend_uid, C4Document, NodeAttributes};

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
        kind: None,
    });

    let project_data = scan_projects(&root);
    let resolver = VarResolver::new(&project_data);
    let packages = classify_packages(&project_data);
    let test_packages: HashSet<&str> = packages
        .iter()
        .filter(|(_, role)| *role == PackageRole::TestPackage)
        .map(|(pd, _)| pd.name.as_str())
        .collect();
    let is_test_pkg = |name: &str| test_packages.contains(name);
    let is_test_construct = |c: &ConstructInstance| is_test_pattern(&c.id) || is_test_pattern(&c.file);

    // Find which Architectures exist (for linking consumers to backends)
    let mut seen_architectures = HashSet::new();
    let mut arch_by_definer: HashMap<&str, Vec<String>> = HashMap::new();
    for pd in &project_data {
        if is_test_pkg(&pd.name) { continue; }
        for c in &pd.constructs {
            if is_test_construct(c) { continue; }
            if c.kind.is_architecture() && seen_architectures.insert(c.id.clone()) {
                let rel_file = rel_path(&c.file, root_str);
                let uid = backend_uid(&c.id);
                doc.add_node(&uid, &c.id, "Backend", NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: Some(rel_file),
                    variable: c.var_name.clone(),
                    kind: Some(c.kind.as_str().to_string()),
                });
                doc.add_relation(&system_name, "contains", &uid);
                arch_by_definer.entry(&pd.name).or_default().push(uid);
            }
        }
    }

    let arch_defining_packages: HashSet<&str> = arch_by_definer.keys().copied().collect();

    // Emit consumer packages by role
    for (pd, role) in &packages {
        match role {
            PackageRole::Frontend | PackageRole::Client => {
                let type_name = if *role == PackageRole::Frontend { "Frontend" } else { "Client" };
                doc.add_node(&pd.name, &pd.name, type_name, NodeAttributes {
                    project: Some(pd.name.clone()),
                    file: None,
                    variable: None,
                    kind: None,
                });
                doc.add_relation(&system_name, "contains", &pd.name);

                let used_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                for arch_uid in used_archs {
                    if !is_test_pattern(&arch_uid) {
                        doc.add_relation(&pd.name, "uses", &arch_uid);
                    }
                }
            }
            PackageRole::ClientServer | PackageRole::Infrastructure => {
                // Lift uses to implemented architectures
                let implemented_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);

                // Lift relations to the Architectures owning the bound constructs
                for bind in pd.binds.iter().filter(|b| !b.has_overloads) {
                    let Some((cpkg, bound)) = resolver.resolve(&pd.name, &bind.component_var) else { continue };
                    if is_test_pkg(cpkg) || is_test_construct(bound) { continue; }
                    let Some(arch) = resolver.architecture_of(cpkg, bound) else { continue };
                    if is_test_pattern(&arch.id) { continue; }
                    let target = backend_uid(&arch.id);
                    for arch_uid in &implemented_archs {
                        if arch_uid != &target {
                            doc.add_relation(arch_uid, "uses", &target);
                        }
                    }
                }
            }
            PackageRole::ArchDefiner => {
                // Link Architecture to other architectures it imports
                if let Some(my_uids) = arch_by_definer.get(pd.name.as_str()) {
                    let used_archs = find_used_architectures(pd, &project_data, &arch_defining_packages, &arch_by_definer);
                    for my_uid in my_uids {
                        for used_uid in &used_archs {
                            if my_uid != used_uid && !is_test_pattern(used_uid) {
                                doc.add_relation(my_uid, "uses", used_uid);
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

/// Packages that consume architecture constructs (directly or transitively) and are
/// classified as Frontend or Client. They are internal to the system: they use its Architectures.
/// Returns (package, node type).
pub fn frontend_and_client_packages(project_data: &[ProjectData]) -> Vec<(&ProjectData, &'static str)> {
    classify_packages(project_data)
        .into_iter()
        .filter_map(|(pd, role)| match role {
            PackageRole::Frontend => Some((pd, "Frontend")),
            PackageRole::Client => Some((pd, "Client")),
            _ => None,
        })
        .collect()
}

/// Role of every package relevant to the system view: consumers of architecture constructs
/// (directly or transitively) and packages with constructs or bindings of their own.
fn classify_packages(project_data: &[ProjectData]) -> Vec<(&ProjectData, PackageRole)> {
    let exported_constructs = build_exported_constructs_map(project_data);

    // Packages imported by others (for library detection)
    let imported_packages: HashSet<&str> = project_data
        .iter()
        .flat_map(|pd| pd.imports.iter().map(|i| i.source.as_str()))
        .collect();

    // Identify direct consumers of architecture constructs
    let mut is_consumer: HashSet<&str> = HashSet::new();
    for pd in project_data {
        if !find_consumers(&pd.imports, &exported_constructs).is_empty() {
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
        for pd in project_data {
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

    project_data
        .iter()
        .map(|pd| {
            let consumer = is_consumer.contains(pd.name.as_str());
            (pd, consumer, classify_package(pd, !pd.binds.is_empty(), consumer, &imported_packages))
        })
        .filter(|(pd, consumer, role)| {
            *role == PackageRole::TestPackage
                || *consumer
                || pd.constructs.iter().any(|c| !c.kind.is_architecture())
                || !pd.binds.is_empty()
        })
        .map(|(pd, _, role)| (pd, role))
        .collect()
}

/// Test ids, files and package names are excluded from the system view.
fn is_test_pattern(s: &str) -> bool {
    let s_lower = s.to_lowercase();
    TEST_NAME_PATTERNS.iter().any(|p| s_lower.contains(p))
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
    if pd.constructs.iter().any(|c| c.kind.is_architecture()) {
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
