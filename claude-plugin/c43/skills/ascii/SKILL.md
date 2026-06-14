---
description: Always use when user asks to create, generate, or build a c4 or c43 architecture diagram. 
---

# C43 ascii diagram skill

Generate C43 architecture diagrams.
Provided "$ARGUMENTS" is either 
- kind of diagram the user wants to generate (system, component, etc). In this case run `c43 <kind> --ascii .`
- the ready output from the c43. In this case use input as is for the diagram
- or nothing. In this case run `c43 system --ascii .`

This will give you the architecture, you can proceed with rendering.


## How to create a diagram

Describe the diagram as `layout.json` (nodes on a grid + edges), then run the
layout engine. The engine produces `result.txt` (the ASCII diagram) and
`result.json` (machine-readable outcome) in the current directory.

### The grid
`result.txt` lays node cells on a grid with ruler scaffolding. Each edge is
drawn entirely in its own character (`0-9a-zA-Z`, assigned in definition
order), with `*` at source ports and `► ◄ ▲ ▼` arrowheads at targets. Edge
bodies stay inside the edge lanes (entering a node cell only on the short
port stub), gravitate to the centre of their lane, and never share a 2x2
block with another edge — so parallel runs always keep a blank cell between
them horizontally, vertically, and diagonally:

```
        │
  000   │ 000               001             002               003
  title │ nodes             edges           nodes             edges
        │ Title
        │ Description
        │
        │
        │
        │
        │
        │
        │
        │
 ───────┼────────────────────────────────────────────────────────────────────
  001   │
  nodes │    +--------+                        +--------+
        │    |        |                        |        |
        │    |        |                        |        |
        │    |        |                        |        |
        │    |        *0000000000000           |        |
        │    |  api   |            000000000000►   db   |
        │    |        |                        |        |
        │    |        *1111111111111           |        |
        │    |        |            1           |        |
        │    |        |            1           |        |
        │    +--------+            1           +--------+
        │                          1
        │                          1
 ───────┼──────────────────────────1─────────────────────────────────────────
  002   │                          1
  edges │                          1
        │                          1
        │                          1
        │                          1
        │                          1
 ───────┼──────────────────────────1─────────────────────────────────────────
  003   │                          1
  nodes │                          1           +--------+
        │                          1           |        |
        │                          1           |        |
        │                          1           |        |
        │                          1           |        |
        │                          111111111111► worker |
        │                                      |        |
        │                                      |        |
        │                                      |        |
        │                                      |        |
        │                                      +--------+
        │
        │
 ───────┼────────────────────────────────────────────────────────────────────
  004   │
  edges │
        │
        │
        │
        │
```

#### Rules in order of importance

1 The diagram/data flows left to right, top to bottom
1 Row `000 title` carries the title/description and a routing lane beneath it,
  so edges may approach the first node row from above without hugging boxes
1 Odd rows and even columns are reserved for nodes
1 Even rows and odd columns are reserved for edge lanes
1 Edges route only in the lanes; a node's top side is avoided except for
  row-0 nodes that route up into the title-region lane
1 Inbound (target) ports never use the right side; they pick left, top, or
  bottom (whichever needs fewest elbows). Outbound (source) ports may use any side
1 User-facing nodes on the left
1 Data sources / external systems on the right
1 Secondary/auxilary services (monitoring, DLQ) go below
1 Nodes with more in or out connections gravitate to the center
1 Linked nodes should be as close to each other as possible
1 Keep the grid balanced: spread nodes across BOTH columns and rows rather than
  cramming everything into a few rows. The grid can grow arbitrarily large in
  either direction (there is no column/row limit), so prefer adding a row over
  overloading one. A roughly square node grid (columns ≈ rows) renders close to
  a 16:9 landscape, because each grid step is wider than it is tall. A wide, flat
  grid (many columns, few rows) forces many long edges to share the same few
  horizontal lanes, which crowds them — distribute nodes so edges have lanes to
  spread into. As a rule of thumb, for N nodes aim for about ⌈√N⌉ columns.

#### Algorithm

**Running the engine.** Prefer the compiled `c43` binary; fall back to Python:

    if command -v c43 >/dev/null 2>&1; then
      c43 layout layout.json --auto         # auto loop (drop --auto for one pass)
    else
      uv run skills/c43/scripts/autolayout.py layout.json   # fallback loop
      # one pass: uv run skills/c43/scripts/layout.py layout.json
    fi

Both paths write identical `result.txt` / `result.json`, so everything below
applies unchanged regardless of which ran.

1. Produce `layout.json` with node grid positions:
   ```json
   {
     "title": "System Name",
     "description": "One line description",
     "nodes": [{"id": "svc", "label": "svc", "grid_col": 0, "grid_row": 0}],
     "edges": [{"id": "e1", "from": "svc", "to": "db"}],
     "groups": [
       {"id": "svc_group", "title": "Service Group", "members": ["svc", "db"]},
       {"id": "inner", "title": "Inner", "members": ["db"], "parent": "svc_group"}
     ],
     "hints": {
       "ports": [{"edge_id": "e1", "from_side": "right", "to_side": "left"}],
       "routing_order": ["e1"]
     }
   }
   ```
   - `grid_col` / `grid_row` are node-cell indices (0, 1, 2 …)
   - `hints` is optional — omit it and let the auto loop (step 2) find the
     ports. Supply a hint only to pin a choice you want kept.
   - `hints.ports` entries: `edge_id` (required) plus optional `from_side` /
     `to_side`, each one of `left`, `right`, `top`, `bottom`; one entry per
     edge. `to_side: "right"` is rejected (inbound ports never use the right
     side)
   - `hints.routing_order`: edge ids to route first, in order; unlisted edges
     follow, shortest first
   - `groups` represent nodes with both parent and child nodes. 
     Each group draws a nested double-line frame
     (`╔═╗║╚╝`) around its members in the edge lanes. Fields:
     - `id` (required), `title` (required), `members` (leaf node ids directly
       in the group), `parent` (optional id of the enclosing group).
     - A group's extent is the bounding rectangle of its members plus all
       descendant groups. Frames hug the nodes while edges keep lane centres;
       edges may cross a frame side but never run along one, and keep ≥1 cell
       of clearance from frames. The title sits inside, bottom-left.
     - Validation rejects: unknown member/parent ids, duplicate group ids,
       parent cycles, a frame enclosing a non-member node, and partially
       overlapping frames (full nesting is allowed).
2. Run the auto loop (see **Running the engine** above for the resolution
   block — `c43 layout layout.json --auto`, or the `autolayout.py` fallback):
   - This starts from your `layout.json` (hints optional), then iterates:
     re-routes, reads the engine's own defect feedback, and keeps the single
     port/order change that most improves the layout, until it is clean or
     reaches a local optimum. It writes the best `result.txt` / `result.json`
     it found. It never changes a port side you pinned in `hints` — it only
     fills in or adjusts the ones you left unspecified.
   - It is deterministic (same input → same output) and bounded
     (`--max-evals=N`, default 200). Exit codes match the engine: 0 = clean,
     1 = best attempt still has errors, 2 = usage.
   - For a single deterministic pass with no iteration (hand-tuning, or when
     you have already pinned every hint), drop `--auto` (binary) or run the
     `layout.py` fallback instead — see **Running the engine** above.
3. Read `result.json`. Every key is always present regardless of outcome:
   - `status`: `"ok"` | `"error"`
   - `errors`: list of `{code, edge_ids, at, message, suggestion}`
     - `code`: `validation` (bad input / port overflow), `crossing` (two edges
       share cells; `at` is `[x, y]`), `unroutable` (edge omitted from drawing)
     - `suggestion` says which hint or grid change is most likely to fix it
   - `quality`: integer scorecard, lower is better — `dropped` (edges with no
     route), `wraps` (edges looping the canvas), `crossings`, `top_ports`
     (top ports on non-top-row nodes), `congestion` (lane cells where two
     edges run with no gap), `length`. This is exactly what the auto loop
     minimises, in that priority order.
   - `diagnostics`: list of `{code, edge_ids, at, message, suggestion}` for the
     soft defects `wrap` and `congestion` — the actionable detail behind the
     `quality` counts.
   - `auto` (autolayout only): `{evals, hints}` — how many attempts it made and
     the hints it settled on. Copy `hints` into `layout.json` to pin the result.
   - `title`, `description`, `canvas` (`width`/`height`), `box` (`width`/`height`)
   - `nodes`: id, label, grid_col/grid_row and resolved x/y/w/h
   - `edges`: id, from, to, char, from_port/to_port (`side`,`x`,`y`), route polyline
   - `groups`: id, title, parent, grid (col0/col1/row0/row1), and resolved x/y/w/h
4. If the result still has defects the loop could not remove (check `quality`
   and `diagnostics`), the levers, in order of leverage, are:
   1. `hints.ports` — move one edge of a `crossing`/`congestion` pair off the
      contested side: spread a hub node's incoming edges across
      `top`/`bottom`/`left` instead of stacking them, send fan-out edges from
      `top`/`bottom` rather than all from `right`.
   2. `hints.routing_order` — list a `wrap`/`unroutable` long edge first so it
      claims a clean lane.
   3. Grid moves — only for a structural conflict (two hubs adjacent, or a node
      far from everything it links to).
   - Pin the lever in `layout.json` and re-run step 2; the loop optimises the
     rest around it. Dense graphs (a hub with 4+ edges on one side) may have no
     zero-`crossing` layout — one or two clean crossings beats forcing a wrap.
5. If `status == "ok"` (or the residual `quality` is acceptable for a dense
   graph): `result.txt` is the final diagram. Show it to the user.

