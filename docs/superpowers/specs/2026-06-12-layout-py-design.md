# layout.py — C43 ASCII layout engine

Design for the layout script behind the `c43` skill. The script turns a
`layout.json` (nodes on a coarse grid, edges, optional hints) into a rendered
ASCII diagram plus a fully resolved JSON suitable for downstream renderers
(drawio, SVG, HTML).

## Contract

- Location: `.claude/skills/c43/scripts/layout.py`, single file, stdlib only.
- Invocation: `uv run .claude/skills/c43/scripts/layout.py layout.json`
- Outputs, written to the current directory:
  - `result.txt` — the ASCII diagram with grid scaffolding.
  - `result.json` — expanded `layout.json`: hints consumed, ports positioned,
    edge routes resolved, plus status/errors.
- `result.txt` is always written when routing reaches the render stage, even
  on routing errors, so the caller can see what went wrong. Only input
  validation failures skip rendering (then only `result.json` is written).
- Incremental saves: the script re-writes `result.txt` after every canvas
  mutation, no matter how small — scaffolding painted, each box drawn, each
  edge routed and painted. If the script crashes or hangs mid-run, the last
  written `result.txt` shows exactly how far rendering got.

`SKILL.md` must be updated to match: script path `c43/scripts/layout.py`
(currently says `c43-drawio`), output filenames `result.txt`/`result.json`,
and the result schema described below.

## Input: layout.json

As specified in SKILL.md:

```json
{
  "title": "System Name",
  "description": "One line description",
  "nodes": [{"id": "svc", "label": "svc", "grid_col": 0, "grid_row": 0}],
  "edges": [{"id": "e1", "from": "svc", "to": "db"}],
  "hints": {
    "ports": [{"edge_id": "e1", "from_side": "right", "to_side": "left"}],
    "routing_order": ["e1"]
  }
}
```

`grid_col`/`grid_row` are node-cell indices (0, 1, 2, …). `hints` is
optional.

## Pipeline

Five stages; each stage fails early rather than falling back to defaults.

### 1. Parse + validate

Hard errors (`status: "error"`, no `result.txt`):

- duplicate node or edge ids
- edge referencing an unknown node id
- two nodes in the same grid cell
- malformed hints (unknown `edge_id`, invalid side name)
- more than 62 edges (per-edge character alphabet exhausted; error message
  notes unicode expansion as the future path)

Each edge is assigned a render character at parse time, in definition order,
from the alphabet `0-9a-zA-Z`.

### 2. Geometry

- Uniform box size per diagram: width = widest label + padding, fixed height.
  All boxes identical; column width follows box width.
- Canvas alternates node columns and vertical edge lanes, node rows and
  horizontal edge lanes. Title row spans the top.
- Lanes size dynamically: a lane grows to fit however many parallel tracks
  routing needs (with a minimum matching the SKILL example's proportions).
  "Lane capacity exceeded" is therefore not an error class; the only routing
  failure modes are crossing and unroutability.
- All coordinates are character-canvas coordinates `(col, row)` — shared
  between `result.txt` and `result.json`.

### 3. Port assignment

Defaults:

- forward edge (target in a later column): out the source's right side, into
  the target's left side
- same-column edge: bottom of upper node → top of lower node
- backward edge (target in an earlier column): out the source's left side,
  into the target's right side, routed through the regular edge lanes

`hints.ports` overrides sides per edge. Multiple ports on one box side stack
on separate character rows, ordered by target position to reduce immediate
crossings. Ports render as `*`.

### 4. Routing — lane-graph A*, two passes

Routing graph: vertices are lane junctions (where horizontal and vertical
lanes meet), edges are lane segments. Each routed diagram-edge claims a
distinct track (one character row/column) inside every lane it traverses, so
parallel overlap is impossible by construction.

A* cost is lexicographic **(turns, length)**: minimize elbows first, total
path length only as tie-break. This yields straight L-shaped routes and keeps
letter-rendered edges legible.

- **Pass 1 — no crossings.** Cells occupied by perpendicular segments are
  blocked. Edge order: `hints.routing_order` first, remaining edges
  shortest-Manhattan-first.
- **Pass 2 — desperation.** Only edges that failed pass 1, re-routed with
  crossings permitted at high A* cost (router still minimizes them). Every
  crossing produced is an **error** entry in `result.json`; status becomes
  `"error"` but the diagram is still rendered.
- Edges unroutable even in pass 2 are omitted from the drawing and reported
  as errors.

After routing, each lane orders its parallel segments into tracks arranged to
minimize wiggle.

### 5. Render + report

- Scaffolding always on: labeled rulers (`000 nodes` / `001 edges` headers,
  row index cells) and `│ ─ ┼` separators, exactly as in SKILL.md's example.
- Title cell: title + description in row `000`.
- Boxes drawn with `+ - |`; node label centered.
- Edge bodies — horizontal runs, vertical runs, and elbows — drawn entirely
  with the edge's assigned character. No `+`/`-`/`|` for edges.
- Ports: `*`. Arrowheads: `► ◄ ▲ ▼`.
- At a (desperation-pass) crossing, the vertical edge's character is drawn
  over the horizontal one; the error entry names both edge ids.

## Output: result.json

`layout.json` fully resolved — no `hints` — sufficient to re-render the
diagram in any target format without re-running layout:

```json
{
  "status": "ok",
  "errors": [],
  "title": "rebob",
  "description": "System architecture for the rebob platform",
  "canvas": {"width": 110, "height": 64},
  "box": {"width": 24, "height": 10},
  "nodes": [
    {"id": "frontend", "label": "@bob/frontend",
     "grid_col": 0, "grid_row": 0,
     "x": 10, "y": 8, "w": 24, "h": 10}
  ],
  "edges": [
    {"id": "fe_to_chat", "from": "frontend", "to": "chat", "char": "b",
     "from_port": {"side": "right", "x": 33, "y": 12},
     "to_port":   {"side": "left",  "x": 65, "y": 12},
     "route": [[34, 12], [48, 12], [48, 30], [64, 30]]}
  ]
}
```

- `route` is the polyline of vertices: exit point, every bend, entry point.
  All segments are axis-aligned, so intermediate cells are derivable. This is
  the waypoint list a drawio edge wants.
- Error entries: `{"code", "edge_ids", "at", "message", "suggestion"}`.
  Suggestions are phrased in terms of the caller's knobs: `grid_col`/
  `grid_row` moves, `hints.ports`, `hints.routing_order`. Codes:
  `validation`, `crossing`, `unroutable`.

## Testing

Pytest under `.claude/skills/c43/scripts/tests/`, run with
`uv run --with pytest pytest`:

- unit: validation errors, geometry math, port defaults and hint overrides,
  edge-character assignment
- router: small synthetic graphs — straight line, one elbow, parallel edges
  sharing a lane (distinct tracks), a forced-crossing case (pass 2 + error),
  an unroutable case
- golden file: render the rebob `layout.json` from the repo root and compare
  `result.txt` against a checked-in expected output

## Out of scope (future)

- Unicode edge alphabet beyond 62 edges
- Renderers consuming `result.json` (drawio/SVG/HTML)
- Automatic grid placement (the calling LLM owns node placement)
