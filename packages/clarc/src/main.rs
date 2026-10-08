use clap::Parser;
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
    /// Input file; stdin when absent
    #[arg(long)]
    file: Option<PathBuf>,
}

fn read(file: Option<PathBuf>) -> Result<String, String> {
    match file {
        Some(p) => std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display())),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map_err(|e| format!("stdin: {e}"))?;
            Ok(s)
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let theme = if cli.azure { clarc::catalog::Theme::Azure } else { clarc::catalog::Theme::Aws };
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
