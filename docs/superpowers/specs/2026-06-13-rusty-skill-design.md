# Rusty Skill — dual-use c43 diagram skill + Rust layout port

Moth: `uyzmn` ("Rusty Skill"). This is the authoritative design for migrating the
c43 diagram skill into `cdk-arch`, porting its Python layout engine to Rust as a
`c43 layout` subcommand, and making the skill dual-use (compiled binary when
present, Python fallback otherwise).

## Original task

> we're creating a claude code skill. see
> https://code.claude.com/docs/en/plugins and
> https://code.claude.com/docs/en/skills#share-skills
>
> the skill will live in the current repo and potentially will be published to the
> skill marketplace https://github.com/anthropics/claude-plugins-official
>
> the current version of the plugin is under ~/projects/c43-diag
>
> we should
> - create the plugin/skill harness here
> - copy the current version of the plugin including git history (that repo was a
>   throwaway)
> - make sure we carry over also the moths and the ./docs
>
> then add a new command `c43 layout` that is layout.py from the current
> implementation translated to rust
>
> the skill switches to dual use. if `c43` command is present it uses it,
> otherwise it falls back to use the layout.py

## Goal

Bring the throwaway `~/projects/c43-diag` skill into `cdk-arch` as a
marketplace-publishable plugin, give the existing `packages/c43` Rust CLI a fast
native layout engine, and let the skill use whichever is available without
behavioural difference.

## Scope decisions (settled during brainstorming)

- **Migration:** plain copy of the skill, moths, and `docs/`. Git history is *not*
  replayed; instead the migration commit records the source SHA range
  (`28d5563..59dad74`) for provenance.
- **Plugin location:** repo root `./c43-plugin/` (marketplace-publishable layout).
- **Port scope:** both `layout.py` (engine) and `autolayout.py` (iteration loop)
  are ported to Rust.
- **Command surface:** a single `c43 layout` subcommand; `--auto` selects the
  iteration loop (mirrors the two Python scripts via one flag).
- **Binary detection:** the skill uses plain `command -v c43` on PATH — no
  repo-specific paths — so it works for marketplace users once `c43` is installed.
- **Parity bar:** byte-identical. The Rust port must reproduce `result.txt` *and*
  `result.json` byte-for-byte against the Python on the rebob golden and the
  ported test corpus.
- **Build gate:** `cargo build --release` for `packages/c43` is wired into the
  root `npm run build`, and `cargo test` into the test path, so the CLAUDE.md
  completion gate covers the Rust code.
- **Known deferred bug preserved:** the `ready` moth
  (`render_vertical_edge_char_should_win_at_crossings`) describes a cosmetic
  deviation in `_paint_edge`. The Rust port reproduces *current* Python behaviour
  byte-for-byte, including this bug. The moth stays open.

## Components

### 1. Plugin harness — `./c43-plugin/`

```
c43-plugin/
├── .claude-plugin/plugin.json     # name, version, description
└── skills/c43/
    ├── SKILL.md                   # dual-use instructions
    └── scripts/
        ├── layout.py              # fallback engine (verbatim from c43-diag)
        ├── autolayout.py          # fallback loop (verbatim)
        └── tests/                 # python golden + unit tests (verbatim)
```

The compiled `c43` binary lives in the existing `packages/c43` crate — *not* in
the plugin dir. The plugin ships the Python fallback only; the binary is the fast
path when installed on PATH.

### 2. Migration from `c43-diag`

Plain copy into `cdk-arch`:

- skill → `c43-plugin/skills/c43/` (SKILL.md, scripts/layout.py,
  scripts/autolayout.py, scripts/tests/ — drop `__pycache__` / `.pytest_cache`).
- `docs/superpowers/specs/2026-06-12-layout-py-design.md` and
  `docs/superpowers/plans/2026-06-12-layout-py.md` → same paths under cdk-arch.
- the two moths (`done/bzumk-med-layout`, `ready/bqrzy-low-render_vertical_…`)
  recreated via the moth CLI (never written directly to `.moth/`).

Migration commit message records the source SHA range for provenance.

### 3. Rust port — `c43 layout [--auto] [--max-evals N]`

New clap subcommand on the existing CLI:

- **No `--auto`:** one deterministic pass — `parse_and_validate` → `geometry` →
  `assign_ports` → `route_all` → `render`. Removes stale `result.txt`/`result.json`
  first, writes both to cwd. Exit codes: `0` clean, `1` rendered-with-errors,
  `2` usage/bad-input — matching Python exactly.
- **`--auto`:** best-improvement hill-climb (`optimise`), then a final canonical
  render. Adds `auto: {evals, hints}` to `result.json`. `--max-evals` defaults 200.

Module structure under `packages/c43/src/cmd/layout/`, mirroring the Python stages
so the port is reviewable section-by-section against the source:

```
mod.rs        run(): orchestration, stale-cleanup, exit codes, --auto dispatch
model.rs      Node/Port/Edge/LayoutError/Model + band caches
parse.rs      parse_and_validate (every ValidationError case + message)
geometry.rs   geometry() + band caches, GUTTER_W/BOX_H/LANE_MIN_* constants
ports.rs      assign_ports + inbound-side / elbow heuristics
route.rs      A* (Dijkstra), lexicographic cost tuple, 2x2 halo, two passes
render.rs     Canvas, scaffolding, boxes, edges, incremental saves
report.rs     quality + diagnostics + result.json serialization
auto.rs       optimise() hill-climb
```

#### Fidelity risks to pin

- **Cost tuple:** the router cost is the 5-tuple `(crossings, adjacency, turns,
  centre_offset, length)`, compared strictly lexicographically. Rust tuples
  compare lexicographically natively — direct mapping.
- **JSON byte-parity:** `result.json` must serialize with the same key order and
  2-space indent as Python's `json.dump(..., indent=2)`. Use a serde struct with
  fields in Python's emission order; serde_json preserves struct field order and
  pretty-prints with 2-space indent. Verify exact whitespace (e.g. `null`,
  `[]` empty-collection formatting) against Python output.
- **A* heap tiebreak:** Python pushes an incrementing insertion `counter` into the
  heap entry so cells/dirs are never compared on cost ties. Replicate this so
  route selection is identical under ties.
- **Deferred crossing bug:** reproduce current `_paint_edge` paint order (later
  edge wins the cell) byte-for-byte. Do not fix here.

### 4. Dual-use SKILL.md

The only behavioural change to the skill text — a resolution preamble:

```sh
if command -v c43 >/dev/null 2>&1; then
  c43 layout layout.json --auto                          # fast path
else
  uv run skills/c43/scripts/autolayout.py layout.json    # fallback
fi
```

Because both paths produce identical `result.txt` / `result.json`, the rest of
SKILL.md (grid rules, hint semantics, quality/diagnostics schema, iterate-on-
feedback loop) is unchanged.

## Parity testing strategy

A `tests/` setup in the `packages/c43` crate:

1. **Golden render test** (port of `test_golden.py`): run `c43 layout` on the
   rebob `layout.json`, assert `result.txt` matches `expected_rebob.txt`
   byte-for-byte, and assert `result.json` errors are exactly
   `[["e8","fe_to_memory"]]` with the same quality scorecard
   (`dropped=0, wraps=0, top_ports=0, crossings=1, congestion<=10`). Copy
   `expected_rebob.txt` + the rebob `layout.json` into the crate's fixtures.
2. **Cross-check harness** — the real parity gate: for each fixture run *both*
   `uv run layout.py` and `c43 layout`, diff `result.txt` + `result.json`.
   Fixtures: rebob plus the unit cases from the Python `tests/`.
3. **Ported unit tests** — validation errors (exact message strings), geometry
   offsets, port capacity/overflow, routing cost ordering, `--auto` settling on
   the same hints. Python error strings become Rust assertions verbatim, since the
   SKILL contract exposes them.

The Python fallback stays fully functional and tested, so the dual-use path is
never broken.

## Build gate

- Root `package.json` `build` runs the workspaces build *and*
  `cargo build --release --manifest-path packages/c43/Cargo.toml`.
- Test path runs `cargo test` for the crate.
- CLAUDE.md completion gate (`npm run build` + `npm run e2e` both green) then
  covers the Rust layout code.

## Out of scope (YAGNI)

- Unicode edge alphabet beyond the 62-char set.
- Downstream JSON renderers (drawio/SVG/HTML).
- Automatic node placement (the calling LLM places nodes).
- Actually publishing to the marketplace repo (we make it publishable, not
  published).
- Fixing the deferred vertical-char-at-crossing bug (moth stays open).
