use clap::{ArgAction, Command};

pub fn run(cmd: &mut Command) {
    println!("{}", cmd.get_name());
    if let Some(about) = cmd.get_about() {
        println!("  {}", about);
    }
    print_root_options(cmd);
    print_subcommands(cmd);
}

fn print_root_options(cmd: &Command) {
    let mut opts = Vec::new();
    for arg in cmd.get_arguments() {
        if arg.get_short().is_none() && arg.get_long().is_none() {
            continue;
        }
        let mut parts = Vec::new();
        if let Some(short) = arg.get_short() {
            parts.push(format!("-{}", short));
        }
        if let Some(long) = arg.get_long() {
            parts.push(format!("--{}", long));
        }
        let placeholder = match arg.get_action() {
            ArgAction::Set | ArgAction::Append => {
                let name = arg
                    .get_value_names()
                    .and_then(|v| v.first())
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| arg.get_id().as_str().to_uppercase());
                format!(" <{}>", name)
            }
            _ => String::new(),
        };
        opts.push(format!("{}{}", parts.join(", "), placeholder));
    }
    if !opts.is_empty() {
        println!("  Options:");
        for opt in opts {
            println!("    {}", opt);
        }
    }
}

fn print_subcommands(cmd: &Command) {
    println!("  Subcommands:");
    recurse_subcommands(cmd, cmd.get_name(), 1);
}

fn recurse_subcommands(cmd: &Command, prefix: &str, depth: usize) {
    let mut subs: Vec<&Command> = cmd.get_subcommands().collect();
    subs.sort_by_key(|c| c.get_name());
    if subs.is_empty() {
        return;
    }
    let indent = "  ".repeat(depth);
    for sub in subs {
        if let Some(about) = sub.get_about() {
            println!("{}# {}", indent, about.to_string().trim());
        }
        let mut opt_parts = Vec::new();
        for arg in sub.get_arguments() {
            if arg.get_short().is_none() && arg.get_long().is_none() {
                continue;
            }
            if let Some(long) = arg.get_long() {
                opt_parts.push(format!("[--{}]", long));
            } else if let Some(short) = arg.get_short() {
                opt_parts.push(format!("[--{}]", short));
            }
        }
        let cmd_line = format!(
            "{}{} {}{}",
            indent,
            prefix,
            sub.get_name(),
            if opt_parts.is_empty() {
                String::new()
            } else {
                format!(" {}", opt_parts.join(" "))
            }
        );
        println!("{}", cmd_line);
        println!();
        let new_prefix = format!("{} {}", prefix, sub.get_name());
        recurse_subcommands(sub, &new_prefix, depth + 1);
    }
}
