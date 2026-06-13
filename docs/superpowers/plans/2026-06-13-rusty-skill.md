# Rusty Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the c43 diagram skill into `cdk-arch` as a marketplace-publishable plugin, port its Python layout engine to Rust as `c43 layout [--auto]`, and make the skill use the compiled binary when present and the Python script otherwise.

**Architecture:** The plugin lives at repo root `./c43-plugin/` shipping the Python fallback (`layout.py` + `autolayout.py`) verbatim. The existing `packages/c43` Rust crate gains a `layout` subcommand whose modules mirror the Python pipeline stages 1:1, reproducing `result.txt` and `result.json` byte-for-byte. The migrated `layout.py` is the authoritative line-by-line spec for the port; an always-on golden test plus a Python⇄Rust cross-check script are the parity gate.

**Tech Stack:** Rust (clap, serde, serde_json — all already in `Cargo.toml`), Python 3 via `uv` (fallback runtime + cross-check oracle), npm workspaces (build gate).

**Authoritative source after Task 1:** `c43-plugin/skills/c43/scripts/layout.py` and `autolayout.py`. Every port task below cites the function and line range in that file. When this plan and the source disagree, **the source wins** — the goal is byte-identical reproduction of current behaviour, *including* the known deferred bug (vertical-char-at-crossing, moth `bqrzy`), which must NOT be fixed here.

---

## File Structure

**Migrated (Task 1–3), repo root:**
```
c43-plugin/
├── .claude-plugin/plugin.json           # plugin manifest
└── skills/c43/
    ├── SKILL.md                         # dual-use (edited in Task 16)
    └── scripts/
        ├── layout.py                    # verbatim copy — authoritative spec
        ├── autolayout.py                # verbatim copy
        └── tests/                       # python tests, verbatim (no __pycache__)
docs/superpowers/specs/2026-06-12-layout-py-design.md   # migrated
docs/superpowers/plans/2026-06-12-layout-py.md          # migrated
.moth/done/<id>-med-layout.md            # recreated via moth CLI
.moth/ready/<id>-low-render_vertical_edge_char...md     # recreated via moth CLI
```

**Rust port (Task 4–14), under `packages/c43/`:**
```
src/cmd/layout/
├── mod.rs        # run(), --auto dispatch, stale cleanup, exit codes, main pipeline
├── model.rs      # Node/Port/Edge/LayoutError/Model, band caches, constants
├── parse.rs      # parse_and_validate
├── geometry.rs   # geometry() + band caches
├── ports.rs      # assign_ports + inbound-side/elbow heuristics
├── route.rs      # A* (Dijkstra), lexicographic cost, 2x2 halo, two passes
├── render.rs     # Canvas, scaffolding, boxes, edges, incremental saves
├── report.rs     # quality, diagnostics, result.json serialization
└── auto.rs       # optimise() hill-climb (--auto)
src/cmd/mod.rs    # add `pub mod layout;`
src/main.rs       # add Layout subcommand + dispatch
tests/
├── fixtures/
│   ├── rebob_layout.json       # copy of c43-diag/layout.json
│   └── expected_rebob.txt      # copy of scripts/tests/expected_rebob.txt
├── golden.rs                   # always-on byte-identical rebob test
├── validate.rs                 # ported parse/validation tests
├── geometry.rs                 # ported geometry tests
├── ports.rs                    # ported port tests
├── routing.rs                  # ported routing tests
├── render.rs                   # ported render + main tests
└── autolayout.rs               # ported auto-loop tests
parity-check.sh                 # Python⇄Rust cross-check oracle (dev gate)
```

---

## Task 1: Migrate skill scripts and docs (plain copy)

**Files:**
- Create: `c43-plugin/skills/c43/SKILL.md`, `c43-plugin/skills/c43/scripts/layout.py`, `c43-plugin/skills/c43/scripts/autolayout.py`
- Create: `c43-plugin/skills/c43/scripts/tests/*` (test_*.py, conftest.py, expected_rebob.txt)
- Create: `docs/superpowers/specs/2026-06-12-layout-py-design.md`, `docs/superpowers/plans/2026-06-12-layout-py.md`

- [ ] **Step 1: Copy the skill tree, excluding Python caches**

```bash
mkdir -p c43-plugin/skills/c43/scripts/tests
cp ~/projects/c43-diag/.claude/skills/c43/SKILL.md c43-plugin/skills/c43/SKILL.md
cp ~/projects/c43-diag/.claude/skills/c43/scripts/layout.py c43-plugin/skills/c43/scripts/layout.py
cp ~/projects/c43-diag/.claude/skills/c43/scripts/autolayout.py c43-plugin/skills/c43/scripts/autolayout.py
cp ~/projects/c43-diag/.claude/skills/c43/scripts/tests/test_*.py c43-plugin/skills/c43/scripts/tests/
cp ~/projects/c43-diag/.claude/skills/c43/scripts/tests/conftest.py c43-plugin/skills/c43/scripts/tests/
cp ~/projects/c43-diag/.claude/skills/c43/scripts/tests/expected_rebob.txt c43-plugin/skills/c43/scripts/tests/
```

- [ ] **Step 2: Copy the migrated docs**

```bash
mkdir -p docs/superpowers/plans
cp ~/projects/c43-diag/docs/superpowers/specs/2026-06-12-layout-py-design.md docs/superpowers/specs/
cp ~/projects/c43-diag/docs/superpowers/plans/2026-06-12-layout-py.md docs/superpowers/plans/
```

- [ ] **Step 3: Verify no caches were copied**

Run: `find c43-plugin -name '__pycache__' -o -name '.pytest_cache' | head`
Expected: no output (empty).

- [ ] **Step 4: Verify the Python fallback runs from its new home**

Run:
```bash
cd c43-plugin/skills/c43/scripts && uv run python -m pytest tests/ -q 2>&1 | tail -5; cd -
```
Expected: tests collect and pass (golden test path in `test_golden.py` resolves the repo via `os.pardir * 5`; if it fails *only* on the golden path skip it for now — Task 14 re-pins the golden in Rust). All non-golden tests must pass.

- [ ] **Step 5: Capture source provenance and commit**

```bash
SRC_RANGE=$(git -C ~/projects/c43-diag log --oneline | tail -1 | cut -d' ' -f1)..$(git -C ~/projects/c43-diag rev-parse --short HEAD)
git add c43-plugin docs/superpowers
git commit -m "[uyzmn] Migrate c43 skill + docs from c43-diag

Plain copy of the c43 diagram skill (SKILL.md, layout.py, autolayout.py,
tests) and its design/plan docs from the throwaway ~/projects/c43-diag repo.
Source commits: ${SRC_RANGE} (c43-diag).
Caches (__pycache__, .pytest_cache) excluded."
```

---

## Task 2: Migrate the two moths via the moth CLI

**Files:**
- Create (via CLI, never write `.moth/` directly): one `done` moth, one `ready` moth.

- [ ] **Step 1: Recreate the completed layout moth, body from the source**

```bash
cat ~/projects/c43-diag/.moth/done/bzumk-med-layout.md | moth new "layout.py engine" -s med --no-edit --stdin
```
Capture the new id printed by moth (call it `$DONE_ID`).

- [ ] **Step 2: Move it to done**

```bash
moth mv $DONE_ID done
```

- [ ] **Step 3: Recreate the open render-bug moth**

```bash
cat ~/projects/c43-diag/.moth/ready/bqrzy-low-render_vertical_edge_char_should_win_at_crossings.md \
  | moth new "render vertical edge char should win at crossings" -s low --no-edit --stdin
```
This stays in `ready` (default). It documents the deferred bug the Rust port must preserve.

- [ ] **Step 4: Verify**

Run: `moth ls -a`
Expected: the current `uyzmn` Rusty Skill moth in `doing`, the new layout moth in `done`, the render-bug moth in `ready`.

- [ ] **Step 5: Commit**

```bash
git add .moth
git commit -m "[uyzmn] Migrate c43-diag moths (layout done, render-bug open)"
```

---

## Task 3: Plugin manifest

**Files:**
- Create: `c43-plugin/.claude-plugin/plugin.json`

- [ ] **Step 1: Write the manifest**

```json
{
  "name": "c43-diagram",
  "version": "0.1.0",
  "description": "Generate C43 architecture diagrams as ASCII from a grid + edges description.",
  "author": { "name": "Oleksandr Fedorenko" }
}
```

Path: `c43-plugin/.claude-plugin/plugin.json`

- [ ] **Step 2: Validate JSON**

Run: `cat c43-plugin/.claude-plugin/plugin.json | uv run python -c "import sys,json; json.load(sys.stdin); print('ok')"`
Expected: `ok`

- [ ] **Step 3: Commit**

```bash
git add c43-plugin/.claude-plugin/plugin.json
git commit -m "[uyzmn] Add c43 plugin manifest"
```

---

## Task 4: Rust scaffold — `layout` subcommand + build wiring

**Files:**
- Modify: `packages/c43/src/cmd/mod.rs`
- Create: `packages/c43/src/cmd/layout/mod.rs` (stub)
- Modify: `packages/c43/src/main.rs:23-94` (add subcommand + dispatch)
- Modify: `package.json` (root) — wire cargo into build/test

- [ ] **Step 1: Register the module**

In `packages/c43/src/cmd/mod.rs`, add a line:
```rust
pub mod layout;
```

- [ ] **Step 2: Create the layout module stub with its submodules declared**

Create `packages/c43/src/cmd/layout/mod.rs`:
```rust
mod model;
mod parse;
mod geometry;
mod ports;
mod route;
mod render;
mod report;
mod auto;

use std::path::Path;

/// Run the layout engine. `auto` selects the iteration loop; `max_evals`
/// bounds it. Writes result.txt/result.json to the current directory.
/// Returns the process exit code (0 clean, 1 rendered-with-errors, 2 usage).
pub fn run(input: &Path, auto: bool, max_evals: usize) -> i32 {
    let _ = (input, auto, max_evals);
    todo!("filled in by later tasks")
}
```

Create empty placeholder files so the crate compiles, each with a `//! stage` doc comment:
`model.rs`, `parse.rs`, `geometry.rs`, `ports.rs`, `route.rs`, `render.rs`, `report.rs`, `auto.rs`.

- [ ] **Step 3: Add the subcommand to clap**

In `packages/c43/src/main.rs`, add to `enum Commands` (after `List`):
```rust
    /// Render an ASCII C43 diagram from a layout.json (grid + edges)
    Layout {
        /// Path to layout.json
        layout: PathBuf,
        /// Iterate on engine feedback to settle ports/order automatically
        #[arg(long)]
        auto: bool,
        /// Eval budget for --auto
        #[arg(long, default_value_t = 200)]
        max_evals: usize,
    },
```

In `fn main()`, handle it as its own arm that exits with the engine's code (it does its own I/O; the `--ascii` flag does not apply):
```rust
    if let Commands::Layout { layout, auto, max_evals } = &cli.command {
        std::process::exit(cmd::layout::run(layout, *auto, *max_evals));
    }
```
Place this immediately after `let cli = Cli::parse();`. Leave the existing `match cli.command` below unchanged (the `Layout` arm will be unreachable there; add `Commands::Layout { .. } => unreachable!(),` to both match blocks that need exhaustiveness).

- [ ] **Step 4: Verify it builds and the subcommand is wired**

Run: `cargo build --manifest-path packages/c43/Cargo.toml 2>&1 | tail -3 && packages/c43/target/debug/c43 layout --help`
Expected: build succeeds; help shows `--auto` and `--max-evals`. (Calling it on a real file panics with `todo!` — expected until later tasks.)

- [ ] **Step 5: Wire cargo into the npm build gate**

In root `package.json`, change the `build` and `test` scripts:
```json
    "build": "npm run build --workspaces && cargo build --release --manifest-path packages/c43/Cargo.toml",
    "test": "cd packages/cdk-arch && npm run test && cargo test --manifest-path ../c43/Cargo.toml"
```

- [ ] **Step 6: Verify the gate runs cargo**

Run: `npm run build 2>&1 | tail -5`
Expected: workspaces build, then `cargo build --release` compiles c43 (the release `c43` binary appears at `packages/c43/target/release/c43`).

- [ ] **Step 7: Commit**

```bash
git add packages/c43/src package.json
git commit -m "[uyzmn] Scaffold c43 layout subcommand + wire cargo into npm build"
```

---

## Task 5: Port the data model (`model.rs`)

Spec: `layout.py:1-86` (constants, `Node`, `Port`, `Edge`, `LayoutError`, `Model`) and `layout.py:272-284` (`_build_band_caches`).

**Files:**
- Modify: `packages/c43/src/cmd/layout/model.rs`

- [ ] **Step 1: Translate constants and structs**

Write `model.rs` with:
- `EDGE_ALPHABET`: a `&str` = `"0123..9abc..zABC..Z"` (digits+lowercase+uppercase, 62 chars). Build with a `const` or a function; assert `len()==62` in a unit test.
- `pub const SIDES: [&str; 4] = ["left", "right", "top", "bottom"];`
- Geometry constants as `pub const ... : i64`: `GUTTER_W=8, LABEL_PAD=4, BOX_H=11, BOX_MARGIN_X=4, BOX_MARGIN_Y=2, LANE_MIN_W=16, LANE_MIN_H=7, TITLE_H=6`.
- `struct Node { id, label: String, grid_col, grid_row: i64, x,y,w,h: i64 }` (x/y/w/h default 0).
- `struct Port { side: String, x: i64, y: i64 }`.
- `struct Edge { id, from_id, to_id: String, char: char, from_port: Option<Port>, to_port: Option<Port>, route: Option<Vec<[i64;2]>> }`.
- `struct LayoutError { code: String, edge_ids: Vec<String>, at: Option<[i64;2]>, message: String, suggestion: String }`.
- `struct Band { start: i64, end: i64, kind: &'static str, center: Option<i64> }` (replaces the Python 4-tuple).
- `struct Model { ... }` with all Python fields: `title, description: String; nodes: Vec<Node>; edges: Vec<Edge>; hint_ports: BTreeMap<String, HintPort>; routing_order: Vec<String>; canvas_w, canvas_h, box_w, box_h: i64; col_x, row_y: BTreeMap<i64,i64>; col_bands, row_bands: Vec<Band>; col_kind, row_kind: Vec<Option<&'static str>>; col_center, row_center: Vec<Option<i64>>; errors: Vec<LayoutError> }`.
- `struct HintPort { from_side: Option<String>, to_side: Option<String> }`.

> Use `i64` for all coordinates (Python ints). Use `BTreeMap` where Python relies on dict iteration that the code later sorts anyway, but **preserve insertion order** where Python iterates a dict in insertion order and does NOT sort (see Task 8 `groups`); use `indexmap::IndexMap` for those — add `indexmap = "2"` to `Cargo.toml` if needed, OR carry an explicit `Vec` of keys. Note in code which maps are order-sensitive.

- [ ] **Step 2: Port `_build_band_caches`**

```rust
// flatten(bands, n) -> (kind, center) per coordinate
pub fn build_band_caches(m: &mut Model) {
    let flatten = |bands: &[Band], n: i64| {
        let n = n as usize;
        let mut kind = vec![None; n];
        let mut center = vec![None; n];
        for b in bands {
            for v in b.start.max(0)..b.end.min(n as i64) {
                kind[v as usize] = Some(b.kind);
                center[v as usize] = b.center;
            }
        }
        (kind, center)
    };
    let (ck, cc) = flatten(&m.col_bands, m.canvas_w);
    let (rk, rc) = flatten(&m.row_bands, m.canvas_h);
    m.col_kind = ck; m.col_center = cc;
    m.row_kind = rk; m.row_center = rc;
}
```

- [ ] **Step 3: Unit test the alphabet and a band cache**

In `model.rs` `#[cfg(test)]`:
```rust
#[test]
fn alphabet_is_62_chars() {
    assert_eq!(EDGE_ALPHABET.chars().count(), 62);
    assert_eq!(EDGE_ALPHABET.chars().next().unwrap(), '0');
    assert_eq!(EDGE_ALPHABET.chars().last().unwrap(), 'Z');
}
```

- [ ] **Step 4: Run**

Run: `cargo test --manifest-path packages/c43/Cargo.toml model:: 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port layout data model to Rust"
```

---

## Task 6: Port parse + validate (`parse.rs`)

Spec: `layout.py:25 (ValidationError), 89-212 (parse_and_validate)`. Ported tests: `scripts/tests/test_validate.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/parse.rs`
- Create: `packages/c43/tests/validate.rs`

- [ ] **Step 1: Write failing ported validation tests**

Create `packages/c43/tests/validate.rs`. Port every case in `test_validate.py`. The parser input is parsed JSON; expose a test entry point `parse_and_validate(value: &serde_json::Value) -> Result<Model, String>` (the `Err(String)` is the ValidationError message). Example cases (port ALL of them — message substrings must match Python exactly):
```rust
use serde_json::json;
// helper mirrors test_validate.base()
fn base() -> serde_json::Value {
    json!({"title":"T","description":"D",
      "nodes":[{"id":"a","label":"a","grid_col":0,"grid_row":0},
               {"id":"b","label":"b","grid_col":1,"grid_row":0}],
      "edges":[{"id":"e1","from":"a","to":"b"}]})
}

#[test]
fn parse_ok_assigns_chars_in_order() {
    let mut raw = base();
    raw["edges"].as_array_mut().unwrap().push(json!({"id":"e2","from":"b","to":"a"}));
    let m = c43::cmd::layout::parse::parse_and_validate(&raw).unwrap();
    let chars: Vec<char> = m.edges.iter().map(|e| e.char).collect();
    assert_eq!(chars, vec!['0','1']);
}

#[test]
fn duplicate_node_id_rejected() {
    let mut raw = base();
    raw["nodes"].as_array_mut().unwrap().push(json!({"id":"a","label":"x","grid_col":2,"grid_row":0}));
    let err = c43::cmd::layout::parse::parse_and_validate(&raw).unwrap_err();
    assert!(err.contains("duplicate node id"), "{err}");
}

#[test]
fn grid_col_string_rejected() {
    let mut raw = base();
    raw["nodes"][0]["grid_col"] = json!("0");
    let err = c43::cmd::layout::parse::parse_and_validate(&raw).unwrap_err();
    assert!(err.contains("grid_col") && err.contains("must be") && err.contains(">= 0"), "{err}");
}

#[test]
fn inbound_right_hint_rejected() {
    let mut raw = base();
    raw["hints"] = json!({"ports":[{"edge_id":"e1","to_side":"right"}]});
    assert!(c43::cmd::layout::parse::parse_and_validate(&raw).is_err());
}

#[test]
fn unknown_hint_key_rejected() {
    let mut raw = base();
    raw["hints"] = json!({"port":[{"edge_id":"e1"}]});
    let err = c43::cmd::layout::parse::parse_and_validate(&raw).unwrap_err();
    assert!(err.contains("unknown key in hints") && err.contains("ports") && err.contains("routing_order"), "{err}");
}
```
> Port the remaining ~30 cases from `test_validate.py` the same way: missing keys (`title`/`description`/`nodes`/`edges`), empty nodes rejected, empty edges/title/description allowed, duplicate edge id, unknown node ref, two nodes same cell, missing node/edge fields with index+id in the message, bool grid values rejected (JSON `true`/`false` — note Python rejects bool because `isinstance(x,bool)`; in serde a JSON bool is `Value::Bool`, not a number, so the int check already rejects it — assert the same message), too-many-edges (63), exactly-62 passes, duplicate hint ports, duplicate routing_order, bad hint side names the edge and lists all four sides.

To make the crate testable as a library, ensure `packages/c43/src/main.rs` exposes modules via a `lib.rs` OR mark the crate to expose `cmd`. Simplest: add `packages/c43/src/lib.rs` re-exporting `pub mod cmd;` (+ the other mods it needs: `model, parse,` etc. are under `cmd::layout`, so `pub mod cmd;` plus making the layout submodules `pub` suffices). Update `main.rs` to `use c43::...` or keep its own `mod` declarations — pick one: **add `lib.rs`, make `cmd` and `cmd::layout` and its submodules `pub`, and have `main.rs` use the library crate.** Adjust `Cargo.toml` with a `[lib]`/`[[bin]]` split if needed (default layout: `src/lib.rs` + `src/main.rs` both present works automatically; `main.rs` then does `use c43::cmd;`).

- [ ] **Step 2: Run tests to confirm they fail to compile/panic**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test validate 2>&1 | tail -5`
Expected: compile error or `todo!` panic (function not implemented).

- [ ] **Step 3: Implement `parse_and_validate`**

Translate `layout.py:89-212` into `parse.rs`. Signature:
```rust
pub fn parse_and_validate(raw: &serde_json::Value) -> Result<super::model::Model, String>
```
Follow the Python check order EXACTLY (the error reported for a malformed doc depends on order). Key fidelity points:
- Required keys checked in order: title, description, nodes, edges.
- grid_col/grid_row must be JSON integers ≥ 0; reject strings, floats, booleans, negatives, with message `node at index {i} (id={nid!r}): grid_col must be an int >= 0, got {v!r}`. Reproduce Python's `repr` formatting for the value (`'0'` for a string, `True` for bool, `-1` for int). For a faithful `{!r}`: strings→`'...'`, bools→`True`/`False`, ints→bare. Write a small `py_repr(&Value) -> String` helper.
- First node missing-id message has no id; later messages include `(id={nid!r})`.
- Edge char assigned from `EDGE_ALPHABET` by definition index.
- Hints: unknown top-level hint key rejected; `ports[].edge_id` required, must be known, no dups; `from_side`/`to_side` validated against SIDES; `to_side=="right"` rejected; `routing_order` ids known, no dups.
- Return a `Model` with `hint_ports` and `routing_order` populated (geometry/ports/etc. left at defaults).

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test validate 2>&1 | tail -10`
Expected: all ported validation tests PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port parse_and_validate with full validation test parity"
```

---

## Task 7: Port geometry (`geometry.rs`)

Spec: `layout.py:215-269 (geometry)`. Ported tests: `scripts/tests/test_geometry.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/geometry.rs`
- Create: `packages/c43/tests/geometry.rs`

- [ ] **Step 1: Write failing geometry tests**

Create `packages/c43/tests/geometry.rs` porting `test_geometry.py`, including the exact-offsets test:
```rust
use serde_json::json;
fn model_2x2() -> c43::cmd::layout::model::Model {
    let raw = json!({"title":"T","description":"D","nodes":[
        {"id":"a","label":"alpha","grid_col":0,"grid_row":0},
        {"id":"b","label":"b","grid_col":1,"grid_row":0},
        {"id":"c","label":"charlie","grid_col":0,"grid_row":1}],
        "edges":[{"id":"e1","from":"a","to":"b"}]});
    c43::cmd::layout::parse::parse_and_validate(&raw).unwrap()
}

#[test]
fn exact_geometry_offsets() {
    let mut m = model_2x2();
    c43::cmd::layout::geometry::geometry(&mut m);
    assert_eq!(m.box_w, 11);
    assert_eq!(m.canvas_w, 79);
    assert_eq!(m.canvas_h, 57);
    assert_eq!(m.col_x.get(&0), Some(&9));
    assert_eq!(m.col_x.get(&1), Some(&28));
    assert_eq!(m.col_x.get(&2), Some(&44));
    assert_eq!(m.col_x.get(&3), Some(&63));
    assert_eq!(m.row_y.get(&0), Some(&0));
    assert_eq!(m.row_y.get(&1), Some(&13));
    assert_eq!(m.row_y.get(&2), Some(&28));
    assert_eq!(m.row_y.get(&3), Some(&35));
    assert_eq!(m.row_y.get(&4), Some(&50));
    let a = m.nodes.iter().find(|n| n.id=="a").unwrap();
    assert_eq!((a.x, a.y), (13, 15));
}
```
Also port: box width from widest label, all boxes identical size, nodes inside canvas, column/row ordering, top-lane-above-first-row (`row_bands[0].kind=="title"`, `[1]=="lane"`, `[2]=="node"`, and `title_end <= lane_center < node0_start`).

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test geometry 2>&1 | tail -5`
Expected: FAIL/panic.

- [ ] **Step 3: Implement `geometry`**

Translate `layout.py:215-269` verbatim into `pub fn geometry(m: &mut Model)`. Build `col_bands`/`row_bands` as `Vec<Band>`, fill `col_x`/`row_y`, set node x/y/w/h, then call `model::build_band_caches(m)`. Integer division uses Rust `/` on `i64` (matches Python `//` for non-negative operands here).

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test geometry 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port geometry stage with exact-offset parity"
```

---

## Task 8: Port port assignment (`ports.rs`)

Spec: `layout.py:287-453` (`_default_from_side`, `_sign`, `_INBOUND`, `_inbound_pre_cell`, `_elbows`, `_side_reaches_lane`, `_inbound_side`, `_default_sides`, `_other_endpoint`, `assign_ports`) and `_band` at `456-463`. Ported tests: `scripts/tests/test_ports.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/ports.rs`
- Create: `packages/c43/tests/ports.rs`

- [ ] **Step 1: Write failing port tests**

Create `packages/c43/tests/ports.rs` porting `test_ports.py`. Provide a `build(nodes, edges, hints)` helper that runs parse→geometry→assign_ports. Port: forward default sides (right/left), same-column (bottom/top), backward (from=left, to≠right, routes clean), inbound-never-right across a 2×2, inbound-right hint rejected, hint overrides, ports on box border, stacking on distinct rows/cols, the port-overflow case (final fixture: `t` at (0,1), `s0..s3` in col 1 rows 0-3, all four `to_side:top` via hints → capacity `w-2=3`, 1 overflow, exactly one `validation` error whose message contains `"4 ports on top side"`, `"capacity 3"`, and `"t"`), ordering by target position, self-loop top→bottom.
```rust
#[test]
fn forward_edge_default_sides() {
    let m = build(NODES, &[json!({"id":"e1","from":"a","to":"b"})], None);
    let e = &m.edges[0];
    assert_eq!(e.from_port.as_ref().unwrap().side, "right");
    assert_eq!(e.to_port.as_ref().unwrap().side, "left");
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test ports 2>&1 | tail -5`
Expected: FAIL/panic.

- [ ] **Step 3: Implement the port stage**

Translate `layout.py:287-453`. Critical fidelity points:
- `assign_ports` groups by `(node_id, side)` and **iterates groups in insertion order** (Python dict preserves it), but within a group sorts members by the other endpoint's center. Use an order-preserving map (IndexMap or a `Vec<(key, Vec<member>)>`) for `groups` so error/port ordering matches.
- The member sort key uses float center (`other.y + other.h/2.0`); keep it as `f64` for the comparison only. Sorting must be **stable** (Python `list.sort` is stable) — use `sort_by` with a stable comparator; on equal keys preserve prior order. Use `slice::sort_by` (stable).
- Capacity: left/right → `node.h-2`; top/bottom → `node.w-2`. Overflow appends a `validation` LayoutError naming the overflowed edge ids (members beyond capacity), message `node {id!r}: {n} ports on {side} side, capacity {cap}`.
- Port coordinate formulas exactly as Python (`node.y + 1 + (i+1)*(node.h-2)//(assigned+1)` etc.), integer division.
- `_inbound_side` tie-break order `left<bottom<top`; `_side_reaches_lane` walks out from the side until it hits a lane or leaves the node region; fallback `bottom`.

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test ports 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port port assignment with ordering + overflow parity"
```

---

## Task 9: Port routing (`route.rs`) — the A* engine

Spec: `layout.py:465-698` (`_is_node_region`, `_build_blocked`, `_port_exit`, `_port_stub`, `_KING`, `_astar`, `_to_polyline`, `_manhattan`, `_crossing_runs`, `route_all`). Ported tests: `scripts/tests/test_routing.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/route.rs`
- Create: `packages/c43/tests/routing.rs`

- [ ] **Step 1: Write failing routing tests**

Create `packages/c43/tests/routing.rs` porting `test_routing.py`. This includes direct `_astar` adapter tests, so expose `pub fn astar(...)` and `pub fn crossing_runs(...)` from `route.rs`, plus a test helper that builds a single-lane Model like the Python `_astar` adapter (whole canvas one lane, center None). Port: straight line, diagonal Z (4 vertices, vertical leg on lane center), parallel distinct tracks, avoid boxes/gutter, respect routing_order, all-routed-or-reported, parallel gap ≥2 columns, no shared 2×2 block, pass1 rejects occupied start, pass2 minimizes crossings over turns (`crossings==[]`), pass2 prefers gapped track (route stays `c[0]<=13`), `crossing_runs` grouping (three asserts), K5 all routed with deduped crossings, start blocked unroutable, walled goal unroutable, overflow edge skipped without new error.
```rust
#[test]
fn astar_pass1_rejects_occupied_start() {
    let mut occ = std::collections::HashMap::new();
    occ.insert((10,5), "z".to_string());
    let (cells, crossings) = astar_adapter((10,5),(20,5), &Default::default(), &occ, false, 30, 20, &Default::default());
    assert!(cells.is_none() && crossings.is_none());
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test routing 2>&1 | tail -5`
Expected: FAIL/panic.

- [ ] **Step 3: Implement A* with the lexicographic cost and insertion-counter tiebreak**

Translate `layout.py:522-611`. Key Rust-specific details:
- Cost is `type Cost = (i64,i64,i64,i64,i64)` = `(crossings, adjacency, turns, centre_offset, length)`. Rust tuples derive `Ord` lexicographically — exactly Python's tuple comparison.
- `BinaryHeap` is a max-heap; push `std::cmp::Reverse((cost, counter, cell, dir))` so the smallest cost pops first. `counter` is a monotonically increasing `u64` ensuring cells/dirs are never compared on ties (mirrors `itertools.count()`).
- Direction is `Option<(i64,i64)>`. State key `(cell, dir)` for `best`/`came` maps (HashMap).
- 4-connected neighbours in the order `[(1,0),(-1,0),(0,1),(0,-1)]` (Python iterates this exact order; with the counter tiebreak the order only matters for determinism, but keep it identical).
- `in_bounds`: `GUTTER_W < x < w && 0 <= y < h`.
- Adjacency term: `np != goal && forbidden.contains(np) && !(col_kind[nx]=="node" && row_kind[ny]=="node")`. In pass 1 (`!allow_cross`) adjacency is a hard `continue`; in pass 2 it's the second cost term.
- Crossing term: `np in occupied`; hard block in pass 1.
- Centre offset: horizontal step → `row_center[ny]`; vertical step → `col_center[nx]`; `offset = abs(coord - center)` when center is Some.
- Reconstruct path by walking `came` from `(goal, dir)`, reverse; crossings = path cells present in `occupied`.

Also port `_build_blocked` (node-region cells + title band), `_port_exit`, `_port_stub`, `_to_polyline`, `_manhattan`, `_crossing_runs`, and `route_all` (two passes: pass 1 no crossings shortest-first within routing_order, pass 2 desperation with crossing/unroutable errors). `claim` updates `occupied` (setdefault — never overwrite) and adds the `_KING` halo to `forbidden`.

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test routing 2>&1 | tail -15`
Expected: PASS. If the diagonal-Z or pass2 tests fail, re-check the neighbour order and the cost-term composition against `layout.py:592-605`.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port A* routing engine with lexicographic cost parity"
```

---

## Task 10: Port rendering (`render.rs`)

Spec: `layout.py:701-803` (`Canvas`, `ARROWS`, `_paint_text`, `_paint_scaffolding`, `_draw_box`, `_paint_edge`, `render`). Ported tests: `scripts/tests/test_render.py` (the non-`main` half).

**Files:**
- Modify: `packages/c43/src/cmd/layout/render.rs`
- Create: `packages/c43/tests/render.rs`

- [ ] **Step 1: Write failing render tests**

Create `packages/c43/tests/render.rs` porting the canvas/render tests from `test_render.py` (the `main`-level tests come in Task 12): canvas paint + str + out-of-bounds noop, boxes+labels present, title+scaffolding chars (`│ ─ ┼`, `nodes/edges/title`), edge char + arrowhead `►` + source `*`, edge body uses only its char on horizontal runs, incremental save count ≥4 with each save differing. For the incremental-save test, expose `render` returning the sequence of canvas snapshots (e.g. accept a callback or return `Vec<String>` of saves) so the Rust test can assert ≥4 distinct snapshots; keep the on-disk `save` behaviour too.
```rust
#[test]
fn canvas_paint_out_of_bounds_is_noop() {
    let mut cv = Canvas::new(3,3);
    cv.paint(-1,0,'X'); cv.paint(0,-1,'X'); cv.paint(3,0,'X'); cv.paint(0,3,'X');
    assert!(!cv.to_string().contains('X'));
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test render 2>&1 | tail -5`
Expected: FAIL/panic.

- [ ] **Step 3: Implement rendering**

Translate `layout.py:701-803`. Fidelity points:
- `Canvas` holds `Vec<Vec<char>>`; `to_string` joins rows with `\n`, **right-strips each row** (`rstrip`), and appends a trailing `\n` (Python: `"\n".join(...) + "\n"`).
- `save` writes to `path.tmp` then atomic rename (use `std::fs::rename`).
- `ARROWS`: left→`►`, right→`◄`, top→`▼`, bottom→`▲`.
- `_paint_edge`: paint each polyline segment in the edge char, then source `*` and target arrow. **Preserve the current paint order (later edge overwrites at shared cells)** — this is the deferred bug in moth `bqrzy`; do NOT fix it.
- `render`: scaffolding → save; each box → save; each routed edge → save. Edges with `route==None` skipped.
- Multibyte chars (`│ ─ ┼ ► ◄ ▼ ▲`): store as `char` in the grid; `to_string` produces correct UTF-8. (Python indexes by code point; Rust `char` matches.)

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test render 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port rendering (preserving deferred crossing paint-order bug)"
```

---

## Task 11: Port quality, diagnostics, and result.json (`report.rs`)

Spec: `layout.py:806-936` (`WRAP_EXCESS`, `CONGEST_MIN`, `_KING8`, `_route_cell_set`, `_quality_and_diagnostics`, `_port_json`, `_result_json`), `948-958` (`score_key`, `quality_of`), `966-983` (`_validation_error_result`).

**Files:**
- Modify: `packages/c43/src/cmd/layout/report.rs`

- [ ] **Step 1: Implement quality + diagnostics**

Translate `_quality_and_diagnostics` and `_route_cell_set`. `quality` is an ordered set of int counts: `dropped, crossings, wraps, top_ports, congestion, length`. `score_key` returns the tuple `(dropped, wraps, crossings, top_ports, congestion, length)` (note the order differs from the quality dict — match Python exactly). Congestion counts king-adjacent lane cells between distinct edges, each pair halved; diagnostics emitted for `wrap` (drawn-vs-direct > 100) and `congestion` (≥6 cells).

- [ ] **Step 2: Implement byte-identical result.json serialization**

`_result_json` must serialize with the **same key order** as Python and 2-space indent. Build it as a `serde_json::Value` constructed in Python's emission order (serde_json `Map` preserves insertion order when the `preserve_order` feature is on — **add `serde_json = { version = "1", features = ["preserve_order"] }` to `Cargo.toml`**), then `serde_json::to_string_pretty(&value)`.

Top-level key order (from `layout.py:917-936`): `status, errors, quality, diagnostics, title, description, canvas, box, nodes, edges`. (`--auto` later appends `auto`.) `errors[]` order: `code, edge_ids, at, message, suggestion`. `quality` order: `dropped, crossings, wraps, top_ports, congestion, length`. node order: `id, label, grid_col, grid_row, x, y, w, h`. edge order: `id, from, to, char, from_port, to_port, route`. port: `side, x, y`.

> Verify whitespace matches `json.dump(indent=2)`: serde pretty fully expands nested arrays (so a route `[[x,y],...]` becomes multi-line), same as Python. No trailing newline from `to_string_pretty` — Python `json.dump` also writes none. The cross-check in Task 15 is the byte gate; do not hand-verify beyond a spot check here.

- [ ] **Step 3: Implement `_validation_error_result`**

Full key set with zeros/empties, `title`/`description` echoed from raw when it is a dict, else `""`.

- [ ] **Step 4: Unit test score_key ordering**

```rust
#[test]
fn score_key_orders_dropped_then_wraps_then_crossings() {
    // (dropped, wraps, crossings, top_ports, congestion, length)
    let q = Quality { dropped: 1, crossings: 5, wraps: 0, top_ports: 0, congestion: 0, length: 9 };
    assert_eq!(score_key(&q), (1, 0, 5, 0, 0, 9));
}
```
Run: `cargo test --manifest-path packages/c43/Cargo.toml report:: 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43 packages/c43/Cargo.toml
git commit -m "[uyzmn] Port quality/diagnostics + ordered result.json serialization"
```

---

## Task 12: Wire the single-pass pipeline (`mod.rs`) + main-level tests

Spec: `layout.py:938-946 (build_model)`, `985-1029 (main)`. Ported tests: the `main`-level half of `test_render.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/mod.rs`
- Create: `packages/c43/tests/render_main.rs`

- [ ] **Step 1: Write failing main-level tests**

Create `packages/c43/tests/render_main.rs`. Drive the built `c43` binary as a subprocess in a temp dir (mirrors `run_main`), asserting on `result.json`/`result.txt`/exit code:
```rust
use std::process::Command;
fn run_layout(dir: &std::path::Path, raw: &serde_json::Value) -> (Option<serde_json::Value>, Option<String>, i32) {
    std::fs::write(dir.join("layout.json"), raw.to_string()).unwrap();
    let bin = env!("CARGO_BIN_EXE_c43");
    let st = Command::new(bin).args(["layout","layout.json"]).current_dir(dir).status().unwrap();
    let rj = std::fs::read_to_string(dir.join("result.json")).ok().map(|s| serde_json::from_str(&s).unwrap());
    let rt = std::fs::read_to_string(dir.join("result.txt")).ok();
    (rj, rt, st.code().unwrap())
}
```
Port: ok writes both files (status ok, no `hints`/`auto` key, edge char `0`, from_port right, canvas/box > 0, exit 0), validation error writes json only (exit 1, no result.txt), routing error (K5) still renders (exit 1, a `crossing` error, result.txt present), missing arg exits 2 (clap handles missing positional → exit 2), malformed json → error result json only, missing input file → error result, non-dict top-level → error mentioning "JSON object", removes stale outputs, validation error has full key set with zeroed canvas/box and echoed title/description.

- [ ] **Step 2: Run, confirm failure**

Run: `cargo build --manifest-path packages/c43/Cargo.toml && cargo test --manifest-path packages/c43/Cargo.toml --test render_main 2>&1 | tail -5`
Expected: FAIL (run() is still `todo!`).

- [ ] **Step 3: Implement `run` (single-pass path)**

In `mod.rs`, implement the non-`--auto` branch translating `main` (`layout.py:985-1029`):
1. Remove stale `result.json`/`result.txt` (ignore not-found).
2. Read+parse input JSON; on read/parse error write `_validation_error_result(None, ...)` and return 1.
3. If top-level not an object, write error result and return 1.
4. `parse_and_validate`; on Err write `_validation_error_result(raw, msg, "fix layout.json per the message above")` and return 1.
5. geometry → assign_ports → route_all → build Canvas → render("result.txt") → write result.json.
6. Return 1 if status error else 0.

Add a `build_model(raw) -> Result<Model,String>` helper (parse+geometry+ports+route) for reuse by `auto`.

Exit code 2 (usage): clap already returns 2 for a missing positional arg, matching Python's `sys.exit(2)`. No extra handling needed; assert it in the test by invoking `c43 layout` with no path.

- [ ] **Step 4: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test render_main 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Wire single-pass layout pipeline + main-level test parity"
```

---

## Task 13: Port the auto-loop (`auto.rs`) + `--auto`

Spec: `autolayout.py` (whole file). Ported tests: `scripts/tests/test_autolayout.py`.

**Files:**
- Modify: `packages/c43/src/cmd/layout/auto.rs`, `packages/c43/src/cmd/layout/mod.rs`
- Create: `packages/c43/tests/autolayout.rs`

- [ ] **Step 1: Write failing auto-loop tests**

Create `packages/c43/tests/autolayout.rs` porting `test_autolayout.py`: never returns worse than start, deterministic (two runs identical hints/quality/evals), respects user-pinned ports (e1 from_side top stays top), never proposes inbound-right (and chosen hints build), improves-or-matches a crossing graph (dropped 0, score ≤ naive). Expose `pub fn optimise(raw: &Value, max_evals: usize) -> (Value /*hints*/, Option<Quality>, usize /*evals*/)`.
```rust
#[test]
fn loop_is_deterministic() {
    let r = raw(None);
    let (h1,q1,n1) = optimise(&r, 60);
    let (h2,q2,n2) = optimise(&r, 60);
    assert_eq!(h1, h2); assert_eq!(q1, q2); assert_eq!(n1, n2);
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test autolayout 2>&1 | tail -5`
Expected: FAIL/panic.

- [ ] **Step 3: Implement `optimise`**

Translate `autolayout.py:30-152`. Fidelity points:
- `OUT_SIDES = [left,right,top,bottom]`, `IN_SIDES = [left,top,bottom]`, `DEFAULT_MAX_EVALS = 200`.
- `_eval(raw, hints)` builds the model under merged hints; returns None if the hints fail validation (inbound-right). Computes `score_key`, quality, and a **deterministically ordered** `flagged` edge list: hard errors first (in `m.errors` order), then diagnostics, dedup preserving first-seen.
- `candidates` generates, in this exact order: for each flagged edge, for `("to", IN_SIDES)` then `("from", OUT_SIDES)`, skipping pinned `(eid,end)`, each side → a `_with_port_override`; then routing_order promotions for unroutable + flagged ids. Determinism depends on this order — match it.
- Best-improvement: evaluate a full round, commit the single strictly-best improvement (`k < best_key`); stop on perfect (`first 5 terms == 0`), local optimum, or eval budget.
- `_with_port_override` deep-copies hints and sets one side; `_user_pinned` collects `(edge_id,end)` the user fixed.
- Hints are `serde_json::Value` objects throughout so equality comparisons in tests match Python dict equality (with `preserve_order`, key order is insertion order — ensure overrides mutate in the same pattern Python does: existing entry updated in place, else appended).

- [ ] **Step 4: Implement the `--auto` branch in `run`**

Translate `autolayout.py:155-214`: stale cleanup, read/parse (same error results as single-pass), validate up front, `optimise`, then re-run `build_model` under the chosen hints, render, write result.json with an appended `auto: {evals, hints}` key (preserve_order keeps it last), print the `autolayout: N evals, crossings=... wraps=... top_ports=... congestion=...` line to stderr, return 1 if error else 0.

- [ ] **Step 5: Run until green**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test autolayout 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Port self-sufficient auto-loop + --auto flag"
```

---

## Task 14: Byte-identical golden test (rebob)

Spec: `scripts/tests/test_golden.py`.

**Files:**
- Create: `packages/c43/tests/fixtures/rebob_layout.json`, `packages/c43/tests/fixtures/expected_rebob.txt`
- Create: `packages/c43/tests/golden.rs`

- [ ] **Step 1: Copy fixtures into the crate**

```bash
mkdir -p packages/c43/tests/fixtures
cp ~/projects/c43-diag/layout.json packages/c43/tests/fixtures/rebob_layout.json
cp ~/projects/c43-diag/.claude/skills/c43/scripts/tests/expected_rebob.txt packages/c43/tests/fixtures/expected_rebob.txt
```

- [ ] **Step 2: Write the golden test**

Create `packages/c43/tests/golden.rs`:
```rust
use std::process::Command;

#[test]
fn rebob_render_matches_golden() {
    let dir = tempdir();
    let fix = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    std::fs::copy(format!("{fix}/rebob_layout.json"), dir.join("layout.json")).unwrap();
    let bin = env!("CARGO_BIN_EXE_c43");
    let out = Command::new(bin).args(["layout","layout.json"]).current_dir(&dir).output().unwrap();
    let got = std::fs::read_to_string(dir.join("result.txt")).expect("result.txt missing");
    let expected = std::fs::read_to_string(format!("{fix}/expected_rebob.txt")).unwrap();
    assert_eq!(got, expected, "rebob render drifted from golden");

    let rj: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("result.json")).unwrap()).unwrap();
    assert_eq!(rj["status"], "error");
    assert_eq!(out.status.code(), Some(1));
    let pairs: Vec<Vec<String>> = rj["errors"].as_array().unwrap().iter()
        .map(|e| { let mut v: Vec<String> = e["edge_ids"].as_array().unwrap().iter()
            .map(|s| s.as_str().unwrap().to_string()).collect(); v.sort(); v }).collect();
    assert_eq!(pairs, vec![vec!["e8".to_string(), "fe_to_memory".to_string()]]);
    assert!(rj["errors"].as_array().unwrap().iter().all(|e| e["code"]=="crossing"));
    let q = &rj["quality"];
    assert_eq!(q["dropped"], 0); assert_eq!(q["wraps"], 0); assert_eq!(q["top_ports"], 0);
    assert_eq!(q["crossings"], 1);
    assert!(q["congestion"].as_i64().unwrap() <= 10);
}
```
> `tempdir()`: add `tempfile = "3"` to `[dev-dependencies]` in `Cargo.toml` and use `tempfile::tempdir()`.

- [ ] **Step 3: Run the golden test**

Run: `cargo test --manifest-path packages/c43/Cargo.toml --test golden 2>&1 | tail -15`
Expected: PASS. If `result.txt` differs, the diff localizes the porting bug — fix the responsible stage (do not edit `expected_rebob.txt`; it is the Python-approved golden).

- [ ] **Step 4: Commit**

```bash
git add packages/c43
git commit -m "[uyzmn] Byte-identical rebob golden test for c43 layout"
```

---

## Task 15: Python⇄Rust cross-check oracle

**Files:**
- Create: `packages/c43/parity-check.sh`

- [ ] **Step 1: Write the cross-check script**

Create `packages/c43/parity-check.sh` (executable). For each fixture JSON, run both engines in separate temp dirs and diff both outputs, for single-pass and `--auto`:
```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY="$HERE/../../c43-plugin/skills/c43/scripts"
BIN="$HERE/target/release/c43"
cargo build --release --manifest-path "$HERE/Cargo.toml" >/dev/null

check() {  # $1 = layout.json path, $2 = "" or "--auto"
  local fixture="$1" mode="$2"
  local pd rd; pd="$(mktemp -d)"; rd="$(mktemp -d)"
  cp "$fixture" "$pd/layout.json"; cp "$fixture" "$rd/layout.json"
  if [ "$mode" = "--auto" ]; then
    ( cd "$pd" && uv run "$PY/autolayout.py" layout.json >/dev/null 2>&1 ) || true
  else
    ( cd "$pd" && uv run "$PY/layout.py" layout.json >/dev/null 2>&1 ) || true
  fi
  ( cd "$rd" && "$BIN" layout layout.json $mode >/dev/null 2>&1 ) || true
  for f in result.txt result.json; do
    if ! diff -u "$pd/$f" "$rd/$f"; then
      echo "PARITY FAIL: $fixture [$mode] $f"; exit 1
    fi
  done
  echo "ok: $fixture [$mode]"
}

for fx in "$HERE"/tests/fixtures/*.json; do
  check "$fx" ""
  check "$fx" "--auto"
done
echo "ALL PARITY CHECKS PASSED"
```

- [ ] **Step 2: Add more cross-check fixtures**

Copy a couple of the unit fixtures (a clean 2×2 and the K5 crossing graph) into `tests/fixtures/` as standalone JSONs so the cross-check exercises ok/error/auto paths, not just rebob.

- [ ] **Step 3: Run the cross-check**

Run: `chmod +x packages/c43/parity-check.sh && packages/c43/parity-check.sh 2>&1 | tail -20`
Expected: `ALL PARITY CHECKS PASSED`. Any `PARITY FAIL` diff pinpoints a porting discrepancy — fix the stage and re-run.

- [ ] **Step 4: Commit**

```bash
git add packages/c43/parity-check.sh packages/c43/tests/fixtures
git commit -m "[uyzmn] Python<->Rust parity cross-check oracle"
```

---

## Task 16: Make SKILL.md dual-use

**Files:**
- Modify: `c43-plugin/skills/c43/SKILL.md`

- [ ] **Step 1: Add the engine-resolution preamble**

In `SKILL.md`, replace the algorithm's "Run the auto loop" command (`uv run .claude/skills/c43/scripts/autolayout.py layout.json`) and the single-pass note with a dual-use resolution block. Insert near the top of the `#### Algorithm` section:

```markdown
**Running the engine.** Prefer the compiled `c43` binary; fall back to Python:

    if command -v c43 >/dev/null 2>&1; then
      c43 layout layout.json --auto         # auto loop (drop --auto for one pass)
    else
      uv run skills/c43/scripts/autolayout.py layout.json   # fallback loop
      # one pass: uv run skills/c43/scripts/layout.py layout.json
    fi

Both paths write identical `result.txt` / `result.json`, so everything below
applies unchanged regardless of which ran.
```

Update the two later inline command references (steps 2 and 4 of the algorithm) to point at this block rather than the old `.claude/skills/...` paths. Keep all grid rules, hint semantics, and the quality/diagnostics schema unchanged.

- [ ] **Step 2: Verify both paths still produce a diagram**

Run (fallback path, binary not on PATH):
```bash
cd /tmp && cp ~/projects/cdk-arch/packages/c43/tests/fixtures/rebob_layout.json layout.json
uv run ~/projects/cdk-arch/c43-plugin/skills/c43/scripts/autolayout.py layout.json >/dev/null 2>&1; head -3 result.txt
```
Run (binary path):
```bash
cd /tmp && ~/projects/cdk-arch/packages/c43/target/release/c43 layout layout.json --auto >/dev/null 2>&1; head -3 result.txt
```
Expected: both produce a `result.txt` with the rebob scaffolding.

- [ ] **Step 3: Commit**

```bash
git add c43-plugin/skills/c43/SKILL.md
git commit -m "[uyzmn] Make c43 SKILL.md dual-use (binary preferred, python fallback)"
```

---

## Task 17: Full gate + close the moth

**Files:**
- Modify: the `uyzmn` moth body (append specification-relevant parts, per CLAUDE.md)

- [ ] **Step 1: Run the complete project gate**

Run:
```bash
npm run build 2>&1 | tail -5
npm run e2e 2>&1 | tail -5
cargo test --manifest-path packages/c43/Cargo.toml 2>&1 | tail -10
packages/c43/parity-check.sh 2>&1 | tail -3
```
Expected: build succeeds (incl. cargo release), e2e passes, all cargo tests pass, parity passes. **Per CLAUDE.md the task is not complete until `npm run build` and `npm run e2e` both succeed.**

- [ ] **Step 2: Append the delivered specification to the moth**

Append (do not replace) a spec summary to the `uyzmn` moth via the CLI, keeping the original task description:
```bash
{ moth show uyzmn | sed -n '/^---/,$p'; cat <<'EOF'

## Delivered

- Plugin at repo-root `c43-plugin/` (manifest + skills/c43 with SKILL.md and
  python fallback layout.py/autolayout.py + tests). Migrated from c43-diag by
  plain copy; provenance recorded in the migration commit.
- Migrated docs (spec + plan) and the two moths (layout=done, render-bug=ready).
- `c43 layout [--auto] [--max-evals N]` in packages/c43 — a byte-identical Rust
  port of layout.py + autolayout.py. Modules mirror the python stages.
- Parity gates: always-on rebob golden test + Python<->Rust cross-check script.
  The deferred vertical-char-at-crossing bug (moth bqrzy) is preserved, not fixed.
- Dual-use SKILL: uses `c43` if on PATH, else the python fallback.
- cargo build/test wired into root `npm run build` / `npm run test`.
EOF
} | moth update uyzmn
```

- [ ] **Step 3: Final commit and mark done**

```bash
git add .moth
git commit -m "[uyzmn] Append delivered spec to moth"
moth done uyzmn
```

---

## Notes for the implementer

- **The migrated `layout.py`/`autolayout.py` are the spec.** Where this plan summarizes behaviour, the source is authoritative. Read the cited line range before porting each stage.
- **Determinism is a hard requirement.** The A* insertion counter, the stable port-member sort, the candidate-generation order, and `preserve_order` JSON all exist to keep output byte-stable. Do not "simplify" them away.
- **Do not fix the crossing paint-order bug** (moth `bqrzy`) — byte parity requires reproducing it.
- **Library exposure:** Tasks 6+ test internal functions, so `packages/c43` needs a `lib.rs` exposing `cmd::layout` (and its submodules) publicly, with `main.rs` using the lib crate. Set this up in Task 6 Step 1 if not already.
- If the golden or cross-check fails, **bisect by stage**: the diff line/cell maps to geometry (positions), ports (port coords/sides), routing (edge paths), or render (characters).
```
