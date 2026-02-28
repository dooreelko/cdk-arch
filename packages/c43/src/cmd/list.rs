use std::path::Path;

use serde::Serialize;

use crate::analysis::{
    build_exported_constructs_map, find_consumers, scan_projects,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub architectures: Vec<ArchSummary>,
    pub components: Vec<ComponentSummary>,
    pub bindings: Vec<BindingSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub consumers: Vec<ConsumerRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchSummary {
    pub id: String,
    pub var_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub component_type: String,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BindingSummary {
    pub component: String,
    pub base_url: Option<String>,
    pub overloads: Vec<String>,
    pub file: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsumerRef {
    pub source_package: String,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListOutput {
    pub projects: Vec<ProjectSummary>,
}

pub fn run(root: &Path, all: bool) -> ListOutput {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let project_data = scan_projects(&root);
    let exported_constructs = build_exported_constructs_map(&project_data);

    let summaries: Vec<ProjectSummary> = project_data
        .iter()
        .filter_map(|pd| {
            let architectures: Vec<ArchSummary> = pd
                .constructs
                .iter()
                .filter(|c| c.class_name == "Architecture")
                .map(|c| ArchSummary {
                    id: c.id.clone(),
                    var_name: c.var_name.clone(),
                })
                .collect();

            let components: Vec<ComponentSummary> = pd
                .constructs
                .iter()
                .filter(|c| c.class_name != "Architecture")
                .map(|c| ComponentSummary {
                    id: c.id.clone(),
                    component_type: c.class_name.clone(),
                    scope: c.scope_var.clone(),
                })
                .collect();

            let bindings: Vec<BindingSummary> = pd
                .binds
                .iter()
                .map(|b| {
                    let rel_file = b
                        .file
                        .strip_prefix(root.to_str().unwrap_or(""))
                        .unwrap_or(&b.file)
                        .trim_start_matches('/')
                        .to_string();
                    BindingSummary {
                        component: b.component_var.clone(),
                        base_url: b.base_url.clone(),
                        overloads: b.overload_keys.clone(),
                        file: rel_file,
                    }
                })
                .collect();

            let consumer_entries = find_consumers(&pd.imports, &exported_constructs);

            // Group consumer entries by source package
            let mut consumer_map = std::collections::HashMap::<&str, Vec<String>>::new();
            for entry in &consumer_entries {
                consumer_map
                    .entry(&entry.source_package)
                    .or_default()
                    .push(format!("{} [{}]", entry.name, entry.construct_type));
            }
            let mut consumers: Vec<ConsumerRef> = consumer_map
                .into_iter()
                .map(|(pkg, imports)| ConsumerRef {
                    source_package: pkg.to_string(),
                    imports,
                })
                .collect();
            consumers.sort_by(|a, b| a.source_package.cmp(&b.source_package));

            let has_content = !architectures.is_empty()
                || !components.is_empty()
                || !bindings.is_empty()
                || !consumers.is_empty();

            if !all && !has_content {
                return None;
            }

            Some(ProjectSummary {
                name: pd.name.clone(),
                path: pd.path.clone(),
                architectures,
                components,
                bindings,
                consumers,
            })
        })
        .collect();

    ListOutput { projects: summaries }
}

pub fn print_pretty(output: &ListOutput) {
    for project in &output.projects {
        println!("{} ({})", project.name, project.path);

        if !project.architectures.is_empty() {
            println!("  Architectures:");
            for arch in &project.architectures {
                match &arch.var_name {
                    Some(var) => println!("    - {} (var: {})", arch.id, var),
                    None => println!("    - {}", arch.id),
                }
            }
        }

        if !project.components.is_empty() {
            println!("  Components:");
            for comp in &project.components {
                match &comp.scope {
                    Some(scope) => {
                        println!(
                            "    - {} [{}] (scope: {})",
                            comp.id, comp.component_type, scope
                        )
                    }
                    None => println!("    - {} [{}]", comp.id, comp.component_type),
                }
            }
        }

        if !project.bindings.is_empty() {
            println!("  Bindings:");
            for bind in &project.bindings {
                let url = bind.base_url.as_deref().unwrap_or("(no baseUrl)");
                print!("    - {} -> {}", bind.component, url);
                if !bind.overloads.is_empty() {
                    print!(" [overloads: {}]", bind.overloads.join(", "));
                }
                println!("  ({})", bind.file);
            }
        }

        if !project.consumers.is_empty() {
            println!("  Consumes:");
            for consumer in &project.consumers {
                println!("    from {}:", consumer.source_package);
                for imp in &consumer.imports {
                    println!("      - {}", imp);
                }
            }
        }

        println!();
    }
}
