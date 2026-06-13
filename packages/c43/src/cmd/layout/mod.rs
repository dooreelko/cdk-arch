pub mod model;
pub mod parse;
pub mod geometry;
pub mod ports;
pub mod route;
pub mod render;
pub mod report;
pub mod auto;

use std::path::Path;

/// Run the layout engine. `auto` selects the iteration loop; `max_evals`
/// bounds it. Writes result.txt/result.json to the current directory.
/// Returns the process exit code (0 clean, 1 rendered-with-errors, 2 usage).
pub fn run(input: &Path, auto: bool, max_evals: usize) -> i32 {
    let _ = (input, auto, max_evals);
    todo!("filled in by later tasks")
}
