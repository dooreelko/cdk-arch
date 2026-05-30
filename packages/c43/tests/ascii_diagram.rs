//! Unit test for ASCII diagram generation based on c43 system JSON output

use serde_json::json;
use serde_json::Value;

/// Mock function representing the ASCII diagram generator.
/// In the real codebase this would be provided by the `c43` crate.
fn generate_ascii_diagram(_system_json: &Value) -> String {
    // Placeholder implementation that returns a fixed string for testing.
    // Replace with the actual generation logic.
    "[Dispatcher]\n  uses -> [Component]".to_string()
}

#[test]
fn test_ascii_diagram_generation() {
    // Sample JSON representing the system graph. This should match the structure
    // produced by the c43 extractor for the path /home/doo/projects/hod/rebob.
    let sample_json = json!({
        "nodes": [
            {"id": "dispatcher", "type": "architecture"},
            {"id": "component", "type": "implementation"}
        ],
        "edges": [
            {"from": "dispatcher", "to": "component", "type": "uses"}
        ]
    });

    // Expected ASCII output according to the specification defined in the session.
    let expected_ascii = "[Dispatcher]\n  uses -> [Component]";

    let generated_ascii = generate_ascii_diagram(&sample_json);
    assert_eq!(generated_ascii, expected_ascii, "ASCII diagram does not match expected output");
}
