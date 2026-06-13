use c43::cmd::layout::model::Model;
use c43::cmd::layout::report::{
    quality_and_diagnostics, quality_of, result_json, route_cell_set, score_key,
    validation_error_result, Quality,
};
use c43::cmd::layout::{
    geometry::geometry, parse::parse_and_validate, ports::assign_ports, route::route_all,
};
use serde_json::{json, Value};

/// Assert that each needle appears in `haystack` and in strictly increasing
/// byte position order.
fn assert_substring_order(haystack: &str, needles: &[&str]) {
    let mut last = 0usize;
    for n in needles {
        let pos = haystack[last..]
            .find(n)
            .unwrap_or_else(|| panic!("missing key {n:?} after position {last} in:\n{haystack}"));
        last += pos + n.len();
    }
}

fn build_ab() -> Model {
    let raw = json!({
        "title": "Sys", "description": "desc",
        "nodes": [
            {"id": "a", "label": "aaa", "grid_col": 0, "grid_row": 0},
            {"id": "b", "label": "bbb", "grid_col": 1, "grid_row": 0},
        ],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
    });
    let mut m = parse_and_validate(&raw).unwrap();
    geometry(&mut m);
    assign_ports(&mut m);
    route_all(&mut m);
    m
}

#[test]
fn score_key_order() {
    let q = Quality {
        dropped: 1,
        crossings: 5,
        wraps: 0,
        top_ports: 0,
        congestion: 0,
        length: 9,
    };
    // (dropped, wraps, crossings, top_ports, congestion, length)
    assert_eq!(score_key(&q), (1, 0, 5, 0, 0, 9));
}

#[test]
fn route_cell_set_counts_l_route() {
    // L-route: (0,0) -> (0,3) vertical (4 cells), then (0,3) -> (2,3)
    // horizontal (3 cells), sharing the corner (0,3).
    let route = [[0, 0], [0, 3], [2, 3]];
    let cells = route_cell_set(&route);
    assert_eq!(cells.len(), 6);
    assert!(cells.contains(&(0, 0)));
    assert!(cells.contains(&(0, 3)));
    assert!(cells.contains(&(2, 3)));
    assert!(cells.contains(&(1, 3)));
}

#[test]
fn clean_ab_is_ok_zero_quality_length_positive() {
    let m = build_ab();
    let (q, diags) = quality_and_diagnostics(&m);
    assert_eq!(q.dropped, 0);
    assert_eq!(q.crossings, 0);
    assert_eq!(q.wraps, 0);
    assert_eq!(q.top_ports, 0);
    assert_eq!(q.congestion, 0);
    assert!(q.length > 0, "length should be positive, got {}", q.length);
    assert!(diags.is_empty());

    // quality_of agrees with quality_and_diagnostics
    assert_eq!(quality_of(&m), q);

    let v = result_json(&m);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn result_json_key_order() {
    let m = build_ab();
    let v = result_json(&m);
    let s = serde_json::to_string_pretty(&v).unwrap();

    // top-level emission order
    assert_substring_order(
        &s,
        &[
            "\"status\"",
            "\"errors\"",
            "\"quality\"",
            "\"diagnostics\"",
            "\"title\"",
            "\"description\"",
            "\"canvas\"",
            "\"box\"",
            "\"nodes\"",
            "\"edges\"",
        ],
    );

    // quality dict order (note: differs from score_key order)
    assert_substring_order(
        &s,
        &[
            "\"quality\"",
            "\"dropped\"",
            "\"crossings\"",
            "\"wraps\"",
            "\"top_ports\"",
            "\"congestion\"",
            "\"length\"",
        ],
    );

    // node order
    assert_substring_order(
        &s,
        &[
            "\"nodes\"",
            "\"id\"",
            "\"label\"",
            "\"grid_col\"",
            "\"grid_row\"",
            "\"x\"",
            "\"y\"",
            "\"w\"",
            "\"h\"",
        ],
    );

    // edge order
    assert_substring_order(
        &s,
        &[
            "\"edges\"",
            "\"id\"",
            "\"from\"",
            "\"to\"",
            "\"char\"",
            "\"from_port\"",
            "\"to_port\"",
            "\"route\"",
        ],
    );

    // port order (side, x, y) — appears inside from_port
    assert_substring_order(&s, &["\"from_port\"", "\"side\"", "\"x\"", "\"y\""]);

    // char serialized as a 1-char string
    assert!(
        s.contains("\"char\": \"0\""),
        "char should serialize as 1-char string, in:\n{s}"
    );

    // no trailing newline (json.dump indent=2 writes none)
    assert!(!s.ends_with('\n'));
}

#[test]
fn validation_error_result_shape_and_order() {
    let raw = json!({"title": "S", "description": "d"});
    let v = validation_error_result(Some(&raw), "bad thing", "fix it");

    assert_eq!(v["status"], "error");
    let errors = v["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["code"], "validation");
    assert_eq!(errors[0]["edge_ids"].as_array().unwrap().len(), 0);
    assert!(errors[0]["at"].is_null());
    assert_eq!(errors[0]["message"], "bad thing");
    assert_eq!(errors[0]["suggestion"], "fix it");

    assert_eq!(v["quality"]["dropped"], 0);
    assert_eq!(v["quality"]["crossings"], 0);
    assert_eq!(v["quality"]["wraps"], 0);
    assert_eq!(v["quality"]["top_ports"], 0);
    assert_eq!(v["quality"]["congestion"], 0);
    assert_eq!(v["quality"]["length"], 0);

    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 0);
    assert_eq!(v["title"], "S");
    assert_eq!(v["description"], "d");
    assert_eq!(v["canvas"]["width"], 0);
    assert_eq!(v["canvas"]["height"], 0);
    assert_eq!(v["box"]["width"], 0);
    assert_eq!(v["box"]["height"], 0);
    assert_eq!(v["nodes"].as_array().unwrap().len(), 0);
    assert_eq!(v["edges"].as_array().unwrap().len(), 0);

    let s = serde_json::to_string_pretty(&v).unwrap();
    assert_substring_order(
        &s,
        &[
            "\"status\"",
            "\"errors\"",
            "\"code\"",
            "\"edge_ids\"",
            "\"at\"",
            "\"message\"",
            "\"suggestion\"",
            "\"quality\"",
            "\"diagnostics\"",
            "\"title\"",
            "\"description\"",
            "\"canvas\"",
            "\"box\"",
            "\"nodes\"",
            "\"edges\"",
        ],
    );
}

#[test]
fn validation_error_result_non_dict_raw_empty_strings() {
    let v = validation_error_result(None, "m", "s");
    assert_eq!(v["title"], "");
    assert_eq!(v["description"], "");
    // also accepts a non-dict Value
    let raw: Value = json!("not a dict");
    let v2 = validation_error_result(Some(&raw), "m", "s");
    assert_eq!(v2["title"], "");
    assert_eq!(v2["description"], "");
}
