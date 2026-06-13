use std::process::Command;

#[test]
fn rebob_render_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let fix = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    std::fs::copy(format!("{fix}/rebob_layout.json"), dir.path().join("layout.json")).unwrap();
    let bin = env!("CARGO_BIN_EXE_c43");
    let out = Command::new(bin)
        .args(["layout", "layout.json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let got = std::fs::read_to_string(dir.path().join("result.txt"))
        .expect("result.txt missing");
    let expected = std::fs::read_to_string(format!("{fix}/expected_rebob.txt")).unwrap();
    assert_eq!(got, expected, "rebob render drifted from golden");

    let rj: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("result.json")).unwrap())
            .unwrap();
    assert_eq!(rj["status"], "error");
    assert_eq!(out.status.code(), Some(1));

    // the one known crossing: e8 x fe_to_memory
    let mut pairs: Vec<Vec<String>> = rj["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let mut v: Vec<String> = e["edge_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            v.sort();
            v
        })
        .collect();
    pairs.sort();
    assert_eq!(pairs, vec![vec!["e8".to_string(), "fe_to_memory".to_string()]]);
    assert!(rj["errors"].as_array().unwrap().iter().all(|e| e["code"] == "crossing"));

    let q = &rj["quality"];
    assert_eq!(q["dropped"], 0);
    assert_eq!(q["wraps"], 0);
    assert_eq!(q["top_ports"], 0);
    assert_eq!(q["crossings"], 1);
    assert!(q["congestion"].as_i64().unwrap() <= 10, "congestion {}", q["congestion"]);
}

#[test]
fn rebob_groups_render_matches_golden() {
    let dir = tempfile::tempdir().unwrap();
    let fix = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    std::fs::copy(format!("{fix}/rebob_groups_layout.json"), dir.path().join("layout.json"))
        .unwrap();
    let bin = env!("CARGO_BIN_EXE_c43");
    Command::new(bin)
        .args(["layout", "layout.json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let got = std::fs::read_to_string(dir.path().join("result.txt")).expect("result.txt missing");
    let expected = std::fs::read_to_string(format!("{fix}/expected_rebob_groups.txt")).unwrap();
    assert_eq!(got, expected, "rebob groups render drifted from golden");
}
