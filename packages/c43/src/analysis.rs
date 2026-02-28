use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::extract::{extract_from_file, BindCall, ConstructInstance, ImportInfo, ReExport, RouteEntry};
use crate::scan::{find_node_projects, find_ts_files};

/// All extracted data for a single node project
pub struct ProjectData {
    pub name: String,
    pub path: String,
    pub constructs: Vec<ConstructInstance>,
    pub routes: Vec<(String, Vec<RouteEntry>)>,
    pub binds: Vec<BindCall>,
    pub imports: Vec<ImportInfo>,
    pub exported_names: Vec<String>,
    pub reexports: Vec<ReExport>,
}

/// A resolved consumer relationship: this project imports construct X from package Y
#[derive(Debug, Clone)]
pub struct ConsumerEntry {
    pub source_package: String,
    pub name: String,
    pub construct_type: String,
}

/// Map: package_name -> { exported_name -> construct_type }
pub type ExportedConstructsMap = HashMap<String, HashMap<String, String>>;

/// Scan all node projects under root, skipping workspace roots.
/// Returns project data for each leaf package.
pub fn scan_projects(root: &Path) -> Vec<ProjectData> {
    let projects = find_node_projects(root);
    let mut result = Vec::new();

    for (name, pkg_path) in &projects {
        if is_workspace_root(pkg_path) {
            continue;
        }

        let mut pd = scan_directory(pkg_path);

        let rel_path = pkg_path
            .strip_prefix(root)
            .unwrap_or(pkg_path)
            .to_string_lossy()
            .to_string();

        pd.name = name.clone();
        pd.path = if rel_path.is_empty() {
            ".".to_string()
        } else {
            rel_path
        };

        result.push(pd);
    }

    result
}

/// Extract all data from TypeScript files in a single directory.
pub fn scan_directory(dir: &Path) -> ProjectData {
    let ts_files = find_ts_files(dir);

    let mut constructs = Vec::new();
    let mut routes = Vec::new();
    let mut binds = Vec::new();
    let mut imports = Vec::new();
    let mut exported_names = Vec::new();
    let mut reexports = Vec::new();

    for file in &ts_files {
        let extracts = extract_from_file(file);
        constructs.extend(extracts.constructs);
        routes.extend(extracts.routes);
        binds.extend(extracts.binds);
        imports.extend(extracts.imports);
        exported_names.extend(extracts.exported_names);
        reexports.extend(extracts.reexports);
    }

    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    ProjectData {
        name,
        path: dir.to_string_lossy().to_string(),
        constructs,
        routes,
        binds,
        imports,
        exported_names,
        reexports,
    }
}

/// Build a map of package_name -> { var_name -> construct_type }
/// resolving one level of re-exports (barrel files).
pub fn build_exported_constructs_map(projects: &[ProjectData]) -> ExportedConstructsMap {
    // Collect directly exported constructs per project
    let mut direct_exports: HashMap<&str, HashMap<&str, &str>> = HashMap::new();

    for pd in projects {
        let exported_set: HashSet<&str> = pd.exported_names.iter().map(|s| s.as_str()).collect();
        let mut exports = HashMap::new();
        for c in &pd.constructs {
            if let Some(var_name) = &c.var_name {
                if exported_set.contains(var_name.as_str()) {
                    exports.insert(var_name.as_str(), c.class_name.as_str());
                }
            }
        }
        if !exports.is_empty() {
            direct_exports.insert(&pd.name, exports);
        }
    }

    // Resolve re-exports
    let mut result: ExportedConstructsMap = HashMap::new();

    for pd in projects {
        let mut pkg_exports: HashMap<String, String> = HashMap::new();

        if let Some(direct) = direct_exports.get(pd.name.as_str()) {
            for (name, typ) in direct {
                pkg_exports.insert(name.to_string(), typ.to_string());
            }
        }

        for reexport in &pd.reexports {
            let source_pkg = &reexport.source;
            if let Some(source_exports) = direct_exports.get(source_pkg.as_str()) {
                if reexport.local_name == "*" {
                    for (name, typ) in source_exports.iter() {
                        pkg_exports.insert(name.to_string(), typ.to_string());
                    }
                } else if let Some(typ) = source_exports.get(reexport.local_name.as_str()) {
                    pkg_exports.insert(reexport.local_name.clone(), typ.to_string());
                }
            }
        }

        if !pkg_exports.is_empty() {
            result.insert(pd.name.clone(), pkg_exports);
        }
    }

    result
}

/// Find consumer relationships for a project's imports against the exported constructs map.
/// Returns deduplicated entries.
pub fn find_consumers(
    imports: &[ImportInfo],
    exported_constructs: &ExportedConstructsMap,
) -> Vec<ConsumerEntry> {
    let mut by_source: HashMap<&str, Vec<&str>> = HashMap::new();
    for imp in imports {
        by_source
            .entry(imp.source.as_str())
            .or_default()
            .push(imp.local_name.as_str());
    }

    let mut consumers = Vec::new();
    for (source, imported_names) in &by_source {
        if let Some(pkg_constructs) = exported_constructs.get(*source) {
            let mut seen = HashSet::new();
            for name in imported_names {
                if let Some(typ) = pkg_constructs.get(*name) {
                    if seen.insert(*name) {
                        consumers.push(ConsumerEntry {
                            source_package: source.to_string(),
                            name: name.to_string(),
                            construct_type: typ.clone(),
                        });
                    }
                }
            }
        }
    }

    consumers.sort_by(|a, b| a.source_package.cmp(&b.source_package));
    consumers
}

fn is_workspace_root(dir: &Path) -> bool {
    let pkg_path = dir.join("package.json");
    std::fs::read_to_string(pkg_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|v| v.get("workspaces").cloned())
        .is_some()
}
