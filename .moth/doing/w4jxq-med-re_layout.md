there's a new layout engine at ../c43-router we

- completely throw away the cell based layout approach
- carry over moth tasks from it
- faithfully translate the layout ts code to rust following prinicples in moths and the code. this is not a reimplementation, but a translation while keeping the location hints. the scope is subject to discussion
- add a drawio command that similar to ascii "renders" a drawio document

----- AI agent updates -------

## Decisions

- The layout engine becomes a separate Rust library crate `packages/c43-layout`. It only computes layout, it does not render anything. c43 depends on it.
- Root Cargo workspace next to the npm root (`Cargo.toml`, `Cargo.lock` and `target/` at the repo root). npm `workspaces` is an explicit list, so no conflict. Root `build`/`test` scripts call cargo at the root. Any reference to `packages/c43/target` gets fixed.
- Input = a compound graph plus a hints doc:
  - graph: optional title, groups (id, label, parent), nodes (id, label, parent), edges (from, to, optional label). Containment is a tree. Edges cross it and may end on groups.
  - hints, in separate parts:
    - a) relative placement suggestions: `left-of right-of above below same-row same-col`. They may cross group boundaries, since nodes are aligned globally anyway (e.g. a server in a cluster group next to a db outside it). Hints are suggestions. At most one may carry a priority flag, which wins over the others (e.g. the start node). More than one priority → error.
    - b) kind: `a->b is nf`, default data. The lib has no other domain knowledge beyond "data flows left→right, nf flows up→down". Node kind is derived inside the lib: a node is nf ⇔ it has edges and none of them is data (isolated nodes are data, as in the router).
    - c) sizes in abstract cell units (no text in the engine): min node size per node (engine uses the global max, keeping equal squares), min group title width, header height.
  - Unknown id in hints → error.
- Output: `{version, layout, metrics}` (violations, crossings, soft scores, ignored placement hints).
- Translation, not reimplementation, of c43-router `src/` (normalize, skeleton, place, ports, lanes, tracks, route, repair, hier, groups, geom, layout, engine, check). No TS line refs in the Rust code. c43-router gets deleted after the port.
  - Deliberate deviations: `text.ts` moves out of the engine into c43's drawio renderer, which measures labels and passes unit sizes. Node kind is derived as above.
  - Determinism: no HashMap iteration in decisions; keep TS insertion orders and stable sorts.
- Order: port first (golden tests, no placement hints), then placement hints (moth placement-hints).
- Golden tests: router cases `aws star16 groups rebob container` (renamed `rebob.json` → `rebob-system.json`, `container.json` → `rebob-container.json`; rebob ones are renders of ~/projects/hod/rebob, OK to commit, its source stays private) are converted to graph + hints. Node kinds become per-edge kind hints with the effective kinds. Text-derived sizes are recorded from one TS run and stored as unit hints, alongside the TS expected output. The comparison normalizes numbers (`3` vs `3.0`). Any diff caused by the node-kind derivation gets reported and decided, not hidden. `aws` is kept for a future dedicated aws/azure renderer.
- c43 side:
  - `system` and `container` build graph + hints from the C4 doc. System node → title. `contains` start → group. `uses` (and any other non-contains `is`) → data edge. External hints are not supported here; the lib will later serve a separate aws/azure renderer fed by a skill or agent.
  - Edge labels = `is`. They are shown only when more than one non-contains `is` kind occurs. The same pair with different `is` becomes one edge with joined labels. No label space is reserved in the engine; the renderer places labels.
  - New `--drawio` option on `system` and `container`, next to `--ascii`. Drawio XML goes to stdout, warnings/violations go to stderr. Exit code 0 even with violations; non-zero only if generation is impossible (bad input, error).
  - `--ascii` stays as is and does not use the new engine.
  - Removed: the `layout` command (with `--auto`, `--max-evals`, `--out-txt/--out-json`), the cell engine `src/cmd/layout/`, `tests/fixtures/*_layout.json`, and the `component` and `deployment` commands. agent-help and README get updated.
- e2e (local-docker): `c43 container . --drawio` produces valid non-empty drawio.
- Moths: router's done moths (ehfb0 ios4y jzk19 z16j2) are copied as-is into `.moth/done`. pxxyh is deleted (cell renderer gone). New moths for the plugin skill rewrite and for placement hints.
