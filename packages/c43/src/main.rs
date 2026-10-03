use std::path::PathBuf;
use clap::{CommandFactory, Parser, Subcommand};
use c43::{ascii, cmd, drawio};

#[derive(Parser)]
#[command(name = "c43", about = "C4 model extractor for cdk-arch TypeScript projects")]
struct Cli {
    /// Output as ASCII tree instead of JSON
    #[arg(long, global = true)]
    ascii: bool,
    /// Output a laid-out drawio diagram instead of JSON (system and container); warnings go to stderr
    #[arg(long, global = true, conflicts_with = "ascii")]
    drawio: bool,
    /// Print a recursive overview of all commands and options (for LLM agents)
    #[arg(long)]
    agent_help: bool,
    #[command(subcommand)]
    command: Option<Commands>,
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
}

fn main() {
    let mut cli = Cli::parse();

    if cli.agent_help {
        let mut cmd = Cli::command();
        cmd::agent_help::run(&mut cmd);
        return;
    }

    let command = match cli.command.take() {
        Some(c) => c,
        None => {
            eprintln!("error: a subcommand is required (try --agent-help or --help)");
            std::process::exit(2);
        }
    };

    match command {
        Commands::List { path, json, all } => {
            if cli.drawio {
                eprintln!("error: --drawio applies to system and container");
                std::process::exit(2);
            }
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
                Commands::List { .. } => unreachable!(),
            };
            if cli.drawio {
                match drawio::render(&doc) {
                    Ok(r) => {
                        r.warnings.iter().for_each(|w| eprintln!("warning: {w}"));
                        print!("{}", r.xml);
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            } else if cli.ascii {
                println!("{}", ascii::render(&doc));
            } else {
                println!("{}", serde_json::to_string_pretty(&doc).unwrap());
            }
        }
    }
}
