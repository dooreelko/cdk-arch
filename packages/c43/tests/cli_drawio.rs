use std::process::Command;

fn c43(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_c43")).args(args).output().unwrap()
}

const EXAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../example");

#[test]
fn container_drawio_on_example() {
    let out = c43(&["--drawio", "container", EXAMPLE]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let xml = String::from_utf8(out.stdout).unwrap();
    assert!(xml.starts_with("<mxfile"));
    assert!(xml.contains("id=\"n-hello-world/api\""));
    assert!(xml.contains("id=\"g-backend:hello-world\""));
}

#[test]
fn system_drawio_on_example() {
    let out = c43(&["system", "--drawio", EXAMPLE]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let xml = String::from_utf8(out.stdout).unwrap();
    assert!(xml.contains("id=\"n-backend:hello-world\""));
}

#[test]
fn empty_system_still_draws_and_warns() {
    let out = c43(&["--drawio", "container", concat!(env!("CARGO_MANIFEST_DIR"), "/../example/local-docker")]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("<mxfile"));
    assert!(String::from_utf8(out.stderr).unwrap().contains("warning: nothing to lay out"));
}

#[test]
fn removed_commands_are_gone() {
    for c in ["layout", "component", "deployment"] {
        assert!(!c43(&[c, "."]).status.success(), "{c}");
    }
}

#[test]
fn drawio_and_ascii_conflict() {
    assert_eq!(c43(&["--drawio", "--ascii", "container", EXAMPLE]).status.code(), Some(2));
}

#[test]
fn drawio_does_not_apply_to_list() {
    let out = c43(&["--drawio", "list", EXAMPLE]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("--drawio applies to system and container"));
}

#[test]
fn agent_help_mentions_drawio_not_layout() {
    let help = String::from_utf8(c43(&["--agent-help"]).stdout).unwrap();
    assert!(help.contains("--drawio"));
    assert!(!help.contains("c43 layout") && !help.contains("c43 component") && !help.contains("c43 deployment"));
}
