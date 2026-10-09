use clap::{CommandFactory, Parser};
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clarc", about = "Cloud architecture diagrams: service graph (JSON) in, drawio XML out")]
struct Cli {
    /// AWS theme (default)
    #[arg(long, conflicts_with = "azure")]
    aws: bool,
    /// Azure theme
    #[arg(long)]
    azure: bool,
    /// Input file; `-` reads stdin
    #[arg(long)]
    file: Option<PathBuf>,
    /// Print guidance for an LLM writing the input: an example for the chosen theme, hint directives, common glyph names
    #[arg(long)]
    agentic_help: bool,
}

fn read(file: Option<PathBuf>) -> Result<String, String> {
    match file {
        None => Err("no input: pass --file <path>, or --file - to read stdin".to_string()),
        Some(p) if p.as_os_str() == "-" => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map_err(|e| format!("stdin: {e}"))?;
            Ok(s)
        }
        Some(p) => std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display())),
    }
}

fn main() {
    let cli = Cli::parse();
    let theme = if cli.azure { clarc::catalog::Theme::Azure } else { clarc::catalog::Theme::Aws };
    if cli.agentic_help {
        println!("{}", Cli::command().render_help());
        print!("{}", clarc::example::example_text(theme));
        return;
    }
    let out = read(cli.file).and_then(|t| clarc::parse(&t)).and_then(|i| clarc::render(&i, theme));
    match out {
        Ok(r) => {
            r.warnings.iter().for_each(|w| eprintln!("warning: {w}"));
            print!("{}", r.xml);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
