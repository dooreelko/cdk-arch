use std::path::PathBuf;

use clap::{Parser, Subcommand};
use c43::{ascii, cmd};

#[derive(Parser)]
#[command(name = "c43", about = "C4 model extractor for cdk-arch TypeScript projects")]
struct Cli {
    /// Output as ASCII tree instead of JSON
    #[arg(long, global = true)]
    ascii: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract system-level C4 view (Architecture instances across workspace)
    System {
        /// Path to the repository root
        path: PathBuf,
    },
    /// Extract container-level C4 view (children of Architecture instances)
    Container {
        /// Path to the repository root
        path: PathBuf,
    },
    /// Extract component-level C4 view (Functions within architecture containers)
    Component {
        /// Path to the repository root
        path: PathBuf,
        /// Optional container name filter
        #[arg(long)]
        container: Option<String>,
    },
    /// Extract deployment view (architectureBinding.bind calls)
    Deployment {
        /// Path to architecture package
        #[arg(long)]
        arch: PathBuf,
        /// Path to infrastructure package
        #[arg(long)]
        infra: PathBuf,
    },
    /// List all detected architectures, components, and bindings per node project
    List {
        /// Path to walk
        path: PathBuf,
        /// Output as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
        /// Include node projects without any architectural components
        #[arg(long)]
        all: bool,
    },
    /// Render an ASCII C43 diagram from a layout.json (grid + edges)
    Layout {
        /// Path to layout.json
        layout: std::path::PathBuf,
        /// Iterate on engine feedback to settle ports/order automatically
        #[arg(long)]
        auto: bool,
        /// Eval budget for --auto
        #[arg(long, default_value_t = 200)]
        max_evals: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Commands::Layout { layout, auto, max_evals } = &cli.command {
        std::process::exit(cmd::layout::run(layout, *auto, *max_evals));
    }

    match cli.command {
        Commands::List { path, json, all } => {
            let output = cmd::list::run(&path, all);
            if json {
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                cmd::list::print_pretty(&output);
            }
        }
        other => {
            let doc = match other {
                Commands::System { path } => cmd::system::run(&path),
                Commands::Container { path } => cmd::container::run(&path),
                Commands::Component { path, container } => {
                    cmd::component::run(&path, container.as_deref())
                }
                Commands::Deployment { arch, infra } => cmd::deployment::run(&arch, &infra),
                Commands::List { .. } => unreachable!(),
                Commands::Layout { .. } => unreachable!(),
            };
            if cli.ascii {
                println!("{}", ascii::render(&doc));
            } else {
                println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            }
        }
    }
}
