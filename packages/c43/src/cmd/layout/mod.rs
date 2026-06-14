pub mod model;
pub mod groups;
pub mod parse;
pub mod geometry;
pub mod ports;
pub mod route;
pub mod render;
pub mod report;
pub mod auto;

use model::Model;
use render::Canvas;
use serde_json::Value;
use std::path::Path;

/// Atomically write `value` as 2-space pretty JSON to `path` (no trailing
/// newline), mirroring Python's `_write_json`: write `<path>.tmp`, then rename.
fn write_json(path: &Path, value: &Value) -> std::io::Result<()> {
    let tmp = {
        let mut s = path.as_os_str().to_os_string();
        s.push(".tmp");
        std::path::PathBuf::from(s)
    };
    std::fs::write(&tmp, serde_json::to_string_pretty(value).unwrap())?;
    std::fs::rename(&tmp, path)
}

/// Run the full pipeline (parse -> geometry -> ports -> route) on a raw layout
/// value and return the routed Model. Does no file I/O. Mirrors Python's
/// `build_model` (layout.py:938-946); Task 13's auto-loop reuses this.
pub fn build_model(raw: &Value) -> Result<Model, String> {
    let mut m = parse::parse_and_validate(raw)?;
    geometry::geometry(&mut m);
    ports::assign_ports(&mut m);
    route::route_all(&mut m);
    Ok(m)
}

/// Run the layout engine. `auto` selects the iteration loop; `max_evals`
/// bounds it. Writes output to `json_out` and `txt_out` (defaulting to
/// result.json / result.txt when called from the CLI without overrides).
/// Returns the process exit code (0 clean, 1 rendered-with-errors, 2 usage).
pub fn run(input: &Path, auto: bool, max_evals: usize, txt_out: &Path, json_out: &Path) -> i32 {
    // Stale outputs from a previous run must never be mistaken for this run's
    // results, even if we crash before writing anything new.
    for stale in [json_out, txt_out] {
        let _ = std::fs::remove_file(stale); // ignore NotFound (and any) errors
    }

    let json_path = json_out;
    let txt_path = txt_out;

    // Read + parse the input. On either failure, emit a validation error
    // result (no raw echo) and exit 1.
    let raw: Value = match std::fs::read_to_string(input) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                let r = report::validation_error_result(
                    None,
                    &e.to_string(),
                    "ensure the layout.json path is correct and the file is valid JSON",
                );
                let _ = write_json(json_path, &r);
                return 1;
            }
        },
        Err(e) => {
            let r = report::validation_error_result(
                None,
                &e.to_string(),
                "ensure the layout.json path is correct and the file is valid JSON",
            );
            let _ = write_json(json_path, &r);
            return 1;
        }
    };

    if !raw.is_object() {
        let r = report::validation_error_result(
            Some(&raw),
            "layout.json top level must be a JSON object",
            "ensure the layout.json path is correct and the file is valid JSON",
        );
        let _ = write_json(json_path, &r);
        return 1;
    }

    if auto {
        return run_auto(&raw, max_evals, json_path, txt_path);
    }


    let mut m = match parse::parse_and_validate(&raw) {
        Ok(m) => m,
        Err(msg) => {
            let r = report::validation_error_result(
                Some(&raw),
                &msg,
                "fix layout.json per the message above",
            );
            let _ = write_json(json_path, &r);
            return 1;
        }
    };

    geometry::geometry(&mut m);
    ports::assign_ports(&mut m);
    route::route_all(&mut m);
    let mut cv = Canvas::new(m.canvas_w, m.canvas_h);
    let _ = render::render(&m, &mut cv, txt_path);
    let result = report::result_json(&m);
    let _ = write_json(json_path, &result);
    if result["status"] == "error" {
        1
    } else {
        0
    }
}

/// The `--auto` branch: validate up front (same as single-pass), run the
/// best-improvement hill-climb, then render + write the canonical artifacts
/// under the chosen hints, appending an `"auto"` block as the LAST key.
/// Mirrors `autolayout.py`'s `main` (lines 187-214).
fn run_auto(raw: &Value, max_evals: usize, json_path: &Path, txt_path: &Path) -> i32 {
    // Validate up front so bad input fails the same way single-pass does.
    if let Err(msg) = parse::parse_and_validate(raw) {
        let r = report::validation_error_result(
            Some(raw),
            &msg,
            "fix layout.json per the message above",
        );
        let _ = write_json(json_path, &r);
        return 1;
    }

    let (best_hints, _best_q, evals) = auto::optimise(raw, max_evals);

    // Re-run the engine once more under the chosen hints and write the
    // canonical result.txt / result.json. It validated above, so build_model
    // should succeed; degrade gracefully if it somehow does not.
    let mut final_raw = raw.clone();
    final_raw["hints"] = best_hints.clone();
    let m = match build_model(&final_raw) {
        Ok(m) => m,
        Err(msg) => {
            let r = report::validation_error_result(
                Some(raw),
                &msg,
                "fix layout.json per the message above",
            );
            let _ = write_json(json_path, &r);
            return 1;
        }
    };
    let mut cv = Canvas::new(m.canvas_w, m.canvas_h);
    let _ = render::render(&m, &mut cv, txt_path);

    let mut result = report::result_json(&m);
    // Append the auto block as the LAST key (preserve_order keeps it last).
    result["auto"] = serde_json::json!({ "evals": evals, "hints": best_hints });
    let _ = write_json(json_path, &result);

    let q = &result["quality"];
    eprintln!(
        "autolayout: {} evals, crossings={} wraps={} top_ports={} congestion={}",
        evals, q["crossings"], q["wraps"], q["top_ports"], q["congestion"]
    );
    if result["status"] == "error" {
        1
    } else {
        0
    }
}
