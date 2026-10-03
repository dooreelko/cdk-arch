use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::extract::{
    extract_from_file, BindCall, ClassInfo, ConstructInstance, ConstructKind, ImportInfo, ReExport, RouteEntry,
};
use crate::model::{backend_uid, child_uid};
use crate::scan::{find_node_projects, find_ts_files};

/// Metadata from package.json relevant for classification
#[derive(Default)]
pub struct PackageMeta {
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    pub has_index_html: bool,
    pub has_web_config: bool,
}

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
    pub classes: Vec<ClassInfo>,
    pub meta: PackageMeta,
}

/// A resolved consumer relationship: this project imports construct X from package Y
#[derive(Debug, Clone)]
pub struct ConsumerEntry {
    pub source_package: String,
    pub name: String,
    pub construct_type: String,
    pub construct_id: String,
}

/// Map: package_name -> { exported_name -> (construct_type, construct_id) }
pub type ExportedConstructsMap = HashMap<String, HashMap<String, (String, String)>>;

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

    resolve_construct_kinds(&mut result);
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
    let mut classes = Vec::new();

    for file in &ts_files {
        let extracts = extract_from_file(file);
        constructs.extend(extracts.constructs);
        routes.extend(extracts.routes);
        binds.extend(extracts.binds);
        imports.extend(extracts.imports);
        exported_names.extend(extracts.exported_names);
        reexports.extend(extracts.reexports);
        classes.extend(extracts.classes);
    }

    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let meta = read_package_meta(dir);

    ProjectData {
        name,
        path: dir.to_string_lossy().to_string(),
        constructs,
        routes,
        binds,
        imports,
        exported_names,
        reexports,
        classes,
        meta,
    }
}

fn read_package_meta(dir: &Path) -> PackageMeta {
    let pkg_path = dir.join("package.json");
    let json = std::fs::read_to_string(&pkg_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());

    let json = match json {
        Some(v) => v,
        None => return PackageMeta::default(),
    };

    let dep_names = |key: &str| -> Vec<String> {
        json.get(key)
            .and_then(|d| d.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default()
    };

    let dependencies = dep_names("dependencies");
    let dev_dependencies = dep_names("devDependencies");

    // Check for web files: index.html in common locations
    let has_index_html = ["index.html", "public/index.html", "src/index.html"]
        .iter()
        .any(|p| dir.join(p).exists());

    // Check for web build configs
    let has_web_config = ["vite.config", "webpack.config", "next.config"]
        .iter()
        .any(|prefix| {
            ["ts", "js", "mjs", "cjs"]
                .iter()
                .any(|ext| dir.join(format!("{}.{}", prefix, ext)).exists())
        });

    PackageMeta {
        dependencies,
        dev_dependencies,
        has_index_html,
        has_web_config,
    }
}

/// Build a map of package_name -> { var_name -> (construct_type, construct_id) }
/// resolving one level of re-exports (barrel files).
pub fn build_exported_constructs_map(projects: &[ProjectData]) -> ExportedConstructsMap {
    // Collect directly exported constructs per project
    let mut direct_exports: HashMap<&str, HashMap<&str, (&str, &str)>> = HashMap::new();

    for pd in projects {
        let exported_set: HashSet<&str> = pd.exported_names.iter().map(|s| s.as_str()).collect();
        let mut exports = HashMap::new();
        for c in &pd.constructs {
            if let Some(var_name) = &c.var_name {
                if exported_set.contains(var_name.as_str()) {
                    exports.insert(var_name.as_str(), (c.class_name.as_str(), c.id.as_str()));
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
        let mut pkg_exports: HashMap<String, (String, String)> = HashMap::new();

        if let Some(direct) = direct_exports.get(pd.name.as_str()) {
            for (name, (typ, id)) in direct {
                pkg_exports.insert(name.to_string(), (typ.to_string(), id.to_string()));
            }
        }

        for reexport in &pd.reexports {
            let source_pkg = &reexport.source;
            if let Some(source_exports) = direct_exports.get(source_pkg.as_str()) {
                if reexport.local_name == "*" {
                    for (name, (typ, id)) in source_exports.iter() {
                        pkg_exports.insert(name.to_string(), (typ.to_string(), id.to_string()));
                    }
                } else if let Some((typ, id)) = source_exports.get(reexport.local_name.as_str()) {
                    pkg_exports.insert(reexport.local_name.clone(), (typ.to_string(), id.to_string()));
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
                if let Some((typ, id)) = pkg_constructs.get(*name) {
                    if seen.insert(*name) {
                        consumers.push(ConsumerEntry {
                            source_package: source.to_string(),
                            name: name.to_string(),
                            construct_type: typ.clone(),
                            construct_id: id.clone(),
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

/// Resolves a variable name, as seen from a given package, to the construct it refers to.
/// Lookup order: constructs defined in the package itself, then named imports
/// (following the source package's re-exports). Variable names are never resolved
/// globally, so same-named variables in unrelated packages do not collide.
pub struct VarResolver<'a> {
    projects: HashMap<&'a str, &'a ProjectData>,
}

const MAX_RESOLVE_DEPTH: usize = 8;

impl<'a> VarResolver<'a> {
    pub fn new(projects: &'a [ProjectData]) -> Self {
        Self {
            projects: projects.iter().map(|pd| (pd.name.as_str(), pd)).collect(),
        }
    }

    /// Returns (defining package name, construct) for `var` as seen from package `pkg`.
    pub fn resolve(&self, pkg: &str, var: &str) -> Option<(&'a str, &'a ConstructInstance)> {
        self.resolve_depth(pkg, var, 0)
    }

    /// Enclosing Architecture of `c` and the ids of the constructs between them (outermost first).
    fn scope_path(&self, pkg: &str, c: &ConstructInstance) -> Option<(&'a ConstructInstance, Vec<&'a str>)> {
        let mut path = Vec::new();
        let (mut pkg, mut parent) = self.resolve(pkg, c.scope_var.as_deref()?)?;
        for _ in 0..MAX_RESOLVE_DEPTH {
            if parent.kind.is_architecture() {
                path.reverse();
                return Some((parent, path));
            }
            path.push(parent.id.as_str());
            (pkg, parent) = self.resolve(pkg, parent.scope_var.as_deref()?)?;
        }
        None
    }

    /// The Architecture `c` belongs to: itself if it is one, else its enclosing Architecture.
    pub fn architecture_of(&self, pkg: &str, c: &'a ConstructInstance) -> Option<&'a ConstructInstance> {
        if c.kind.is_architecture() {
            return Some(c);
        }
        self.scope_path(pkg, c).map(|(arch, _)| arch)
    }

    /// Node uid of construct `c` (defined in package `pkg`):
    /// `backend:<id>` for an Architecture, `<arch id>/<scope ids...>/<id>` for anything
    /// inside an Architecture, the bare id otherwise.
    pub fn uid(&self, pkg: &str, c: &ConstructInstance) -> String {
        if c.kind.is_architecture() {
            return backend_uid(&c.id);
        }
        match self.scope_path(pkg, c) {
            Some((arch, path)) => {
                let mut uid = arch.id.clone();
                for id in path {
                    uid = child_uid(&uid, id);
                }
                child_uid(&uid, &c.id)
            }
            None => c.id.clone(),
        }
    }

    fn resolve_depth(&self, pkg: &str, var: &str, depth: usize) -> Option<(&'a str, &'a ConstructInstance)> {
        if depth > MAX_RESOLVE_DEPTH {
            return None;
        }
        let pd = self.projects.get(pkg)?;

        if let Some(c) = pd.constructs.iter().find(|c| c.var_name.as_deref() == Some(var)) {
            return Some((pd.name.as_str(), c));
        }

        if let Some(imp) = pd.imports.iter().find(|i| i.local_name == var) {
            let name = imp.imported_name.as_deref().unwrap_or(var);
            if name != "*" && name != "default" {
                if let Some(found) = self.resolve_depth(&imp.source, name, depth + 1) {
                    return Some(found);
                }
            }
        }

        if let Some(found) = pd
            .reexports
            .iter()
            .filter(|r| r.local_name == var || r.local_name == "*")
            .find_map(|r| self.resolve_depth(&r.source, var, depth + 1))
        {
            return Some(found);
        }

        // `inst.field`: resolve the instance, then its class-template construct in that field
        let (head, field) = var.split_once('.')?;
        let (ipkg, inst) = self.resolve_depth(pkg, head, depth + 1)?;
        let qualified = format!("{}.{}", inst.var_name.as_deref()?, field);
        self.resolve_depth(ipkg, &qualified, depth + 1)
    }
}

/// Packages that export the cdk-arch base classes. Classes reached here are not followed
/// further: their name is the base kind.
const CDK_ARCH_PACKAGES: &[&str] = &["@arinoto/cdk-arch"];

const MAX_CLASS_DEPTH: usize = 16;

fn base_kind(name: &str) -> ConstructKind {
    match name {
        "Architecture" => ConstructKind::Architecture,
        "ApiContainer" => ConstructKind::ApiContainer,
        "Function" | "TBDFunction" => ConstructKind::Function,
        _ => ConstructKind::Construct,
    }
}

/// Where a class name, as seen from a package, is declared.
enum ClassRef<'a> {
    /// Declared in a scanned package
    Declared(&'a str, &'a ClassInfo),
    /// A cdk-arch base class (by exported name)
    CdkArch(String),
    /// Imported from a package that is not scanned: cannot be inspected
    External,
    /// Not declared nor imported (a global such as `Error` or `Map`)
    Unknown,
}

struct ClassResolver<'a> {
    projects: HashMap<&'a str, &'a ProjectData>,
}

impl<'a> ClassResolver<'a> {
    fn find(&self, pkg: &str, name: &str, depth: usize) -> ClassRef<'a> {
        if CDK_ARCH_PACKAGES.contains(&pkg) {
            return ClassRef::CdkArch(name.to_string());
        }
        if depth > MAX_CLASS_DEPTH {
            return ClassRef::Unknown;
        }
        let Some(pd) = self.projects.get(pkg) else {
            return ClassRef::External;
        };
        if let Some(ci) = pd.classes.iter().find(|c| c.name == name) {
            return ClassRef::Declared(pd.name.as_str(), ci);
        }
        if let Some(imp) = pd.imports.iter().find(|i| i.local_name == name) {
            let imported = imp.imported_name.as_deref().unwrap_or(name);
            // Relative imports stay inside the package, where the class was not found
            if imp.source.starts_with('.') {
                return ClassRef::Unknown;
            }
            return self.find(&imp.source, imported, depth + 1);
        }
        for r in pd.reexports.iter().filter(|r| r.local_name == name || r.local_name == "*") {
            match self.find(&r.source, name, depth + 1) {
                ClassRef::Unknown => continue,
                found => return found,
            }
        }
        ClassRef::Unknown
    }

    /// Classes from `name` up its `extends` chain (scanned ones only), and the resulting kind.
    fn chain(&self, pkg: &str, name: &str) -> (Vec<(&'a str, &'a ClassInfo)>, ConstructKind) {
        let mut chain = Vec::new();
        let (mut pkg, mut name) = (pkg.to_string(), name.to_string());
        for _ in 0..MAX_CLASS_DEPTH {
            match self.find(&pkg, &name, 0) {
                ClassRef::Declared(p, ci) => {
                    chain.push((p, ci));
                    match &ci.extends {
                        Some(ext) => (pkg, name) = (p.to_string(), ext.clone()),
                        None => return (chain, ConstructKind::NotConstruct),
                    }
                }
                ClassRef::CdkArch(n) => return (chain, base_kind(&n)),
                // Unknowable: keep it as a generic construct
                ClassRef::External => return (chain, ConstructKind::Construct),
                ClassRef::Unknown => return (chain, ConstructKind::NotConstruct),
            }
        }
        (chain, ConstructKind::NotConstruct)
    }
}

/// Resolve every construct's kind through its class hierarchy, drop instances of classes that
/// do not derive from a cdk-arch Construct, and instantiate class templates: constructs and
/// routes a class creates in its constructor become constructs/routes of each instance
/// (including instances of subclasses). A template construct stored in `this.<field>` of an
/// instance held in variable `v` gets variable name `v.<field>` and scope `v`.
pub fn resolve_construct_kinds(projects: &mut [ProjectData]) {
    let mut resolved: Vec<(Vec<ConstructInstance>, Vec<(String, Vec<RouteEntry>)>)> = Vec::new();
    {
        let resolver = ClassResolver {
            projects: projects.iter().map(|pd| (pd.name.as_str(), pd)).collect(),
        };
        for pd in projects.iter() {
            let mut constructs = Vec::new();
            let mut routes = Vec::new();
            // (construct, package its class name is resolved in)
            let mut work: Vec<(ConstructInstance, &str)> =
                pd.constructs.iter().rev().map(|c| (c.clone(), pd.name.as_str())).collect();
            let mut budget = 10_000;
            while let Some((mut c, class_pkg)) = work.pop() {
                budget -= 1;
                if budget == 0 {
                    break;
                }
                let (chain, kind) = resolver.chain(class_pkg, &c.class_name);
                if kind == ConstructKind::NotConstruct {
                    continue;
                }
                c.kind = kind;
                if let Some(var) = c.var_name.clone() {
                    for (cls_pkg, cls) in &chain {
                        for (field, tc) in cls.constructs.iter().rev() {
                            let mut t = tc.clone();
                            t.scope_var = Some(var.clone());
                            t.var_name = Some(format!("{}.{}", var, field));
                            work.push((t, cls_pkg));
                        }
                        if !cls.routes.is_empty() {
                            let entries = cls
                                .routes
                                .iter()
                                .map(|r| RouteEntry {
                                    handler_var: match r.handler_var.strip_prefix("this.") {
                                        Some(field) => format!("{}.{}", var, field),
                                        None => r.handler_var.clone(),
                                    },
                                    ..r.clone()
                                })
                                .collect();
                            routes.push((c.id.clone(), entries));
                        }
                    }
                }
                constructs.push(c);
            }
            resolved.push((constructs, routes));
        }
    }
    for (pd, (constructs, routes)) in projects.iter_mut().zip(resolved) {
        pd.constructs = constructs;
        pd.routes.extend(routes);
    }
}
