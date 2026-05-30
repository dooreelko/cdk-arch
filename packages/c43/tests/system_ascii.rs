// tests/system_ascii.rs
use std::path::PathBuf;
use c43::cmd::system;
use c43::ascii;

#[test]
fn test_system_ascii_render() {
    // Use the current project root for testing
    let mut repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo_path.pop(); // packages/
    repo_path.pop(); // root
    
    // Run the system extraction
    let doc = system::run(&repo_path);
    // Render ASCII diagram
    let ascii_output = ascii::render(&doc);
    println!("ASCII Output:\n{}", ascii_output);
    
    // Basic sanity checks
    assert!(ascii_output.to_lowercase().contains("system: cdk-arch"), "ASCII output missing system node");
    
    // In this repo, 'hello-world' architecture should be found
    assert!(ascii_output.contains("hello-world"), "ASCII output missing hello-world architecture");
    
    // Ensure the output is non‑empty and looks like a tree
    assert!(!ascii_output.trim().is_empty(), "ASCII output is empty");
}
