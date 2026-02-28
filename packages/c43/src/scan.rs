use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_ts_files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "node_modules" && name != "dist" && name != ".git" && name != "target"
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.is_file()
                && path.extension().map_or(false, |ext| {
                    (ext == "ts" || ext == "tsx")
                        && !path.to_string_lossy().ends_with(".d.ts")
                })
        })
        .map(|e| e.into_path())
        .collect()
}

pub fn find_workspace_packages(root: &Path) -> Vec<(String, PathBuf)> {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let workspaces = match json.get("workspaces").and_then(|w| w.as_array()) {
        Some(w) => w,
        None => return Vec::new(),
    };

    let mut packages = Vec::new();
    for ws in workspaces {
        let pattern = match ws.as_str() {
            Some(p) => p,
            None => continue,
        };
        // Handle glob patterns like "packages/example/*"
        let glob_pattern = root.join(pattern);
        let glob_str = glob_pattern.to_string_lossy();
        if glob_str.contains('*') {
            // Simple glob: expand directories matching pattern
            let base = glob_str.split('*').next().unwrap_or("");
            let base_path = Path::new(base);
            if base_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(base_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("package.json").exists() {
                            let name = read_package_name(&path);
                            packages.push((name, path));
                        }
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            if path.is_dir() && path.join("package.json").exists() {
                let name = read_package_name(&path);
                packages.push((name, path));
            }
        }
    }
    packages
}

pub fn find_node_projects(root: &Path) -> Vec<(String, PathBuf)> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "node_modules" && name != "dist" && name != ".git" && name != "target"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() == "package.json")
        .map(|e| {
            let dir = e.path().parent().unwrap().to_path_buf();
            let name = read_package_name(&dir);
            (name, dir)
        })
        .collect()
}

fn read_package_name(dir: &Path) -> String {
    let pkg_path = dir.join("package.json");
    std::fs::read_to_string(&pkg_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}
