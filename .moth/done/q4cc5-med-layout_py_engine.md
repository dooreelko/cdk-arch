Implements `docs/superpowers/specs/2026-06-12-layout-py-design.md` (authoritative
design) and the visual-quality + self-sufficiency requirements added during the
visual-approval loop. This body is the consolidated feature specification: design
detail sufficient to rebuild the same feature from scratch, plus the decisions
taken and rejected along the way. No code-level detail.

## Feature

A layout engine behind the `c43` skill that turns a coarse description of an
architecture diagram (nodes placed on an integer grid + directed edges + optional
hints) into a rendered ASCII diagram plus a fully-resolved machine-readable
result. The calling LLM owns node placement; the engine owns sizing, port
placement, edge routing, rendering, and quality reporting. A companion auto-loop
lets the skill start from a naive description and iterate to a clean layout using
only the engine's own feedback.

## Contract

- Single self-contained script, standard library only, run via the project's
  Python runner. Reads one input file; writes two outputs to the current
  directory: the ASCII diagram and a resolved JSON.
- The JSON is the input fully resolved (hints consumed, ports positioned, edge
  routes as polylines) so a downstream renderer (drawio/SVG/HTML) can reproduce
  the diagram without re-running layout.
- The diagram is written whenever routing reaches the render stage, even when
  routing reports errors, so the caller can see what went wrong. Only input
  validation failures skip rendering (JSON only).
- Stale outputs are removed before each run, so whatever exists afterward belongs
  to that run. The canvas is re-saved after every mutation (scaffolding, each box,
  each edge) so a crash or hang mid-run leaves the furthest-rendered state on disk.
- Exit codes: clean, routing-error-but-rendered, and usage/bad-input are distinct.

## Input

A document with a title, a one-line description, a list of nodes (each with an id,
a label, and integer grid column/row), and a list of directed edges (each with an
id, a from-node, and a to-node). An optional hints block may pin per-edge port
sides and/or a routing order. Hints are advisory inputs the engine and auto-loop
respect; they are never required.

## Coordinate and grid model

- A character canvas addressed by (column, row), origin top-left, shared between
  the ASCII output and the JSON.
- A left-hand gutter holds row labels; a vertical spine separates the gutter from
  the drawable area.
- Columns alternate node-columns and vertical edge-lanes; rows alternate
  node-rows and horizontal edge-lanes. All boxes are one uniform size (width from
  the widest label plus padding, fixed height); node columns/rows share that size.
- The title region spans the top. **Beneath the title sits a routing lane** so
  edges may approach the first node row from above without hugging box tops. The
  title band itself is never a routing surface.
- Lanes have a minimum size and exist to carry parallel edge tracks; "lane
  capacity exceeded" is not an error class.

## Pipeline (five stages, each fails early rather than guessing)

1. **Parse + validate.** Hard errors (status error, no diagram): duplicate node
   or edge ids; edge referencing an unknown node; two nodes in one grid cell;
   malformed hints (unknown edge id, invalid side); a `to_side` hint of "right"
   (prohibited — see ports); more edges than the per-edge character alphabet
   (`0-9a-zA-Z`, 62) allows. Each edge is assigned a distinct render character in
   definition order.

2. **Geometry.** Compute box size and the alternating band layout described above,
   including the title-region lane, and resolve each node's pixel rectangle.

3. **Port assignment.** Each edge gets a source port and a target port on box
   borders.
   - Outbound (source) side defaults by direction: forward → right, backward →
     left, same-column-down → bottom, same-column-up/self → top. Any side is
     allowed outbound.
   - **Inbound (target) side never uses "right".** It is chosen among left, top,
     and bottom — only sides that actually open onto a routing lane (a side facing
     the gutter or off-canvas is excluded) — preferring the fewest elbows, ties
     broken toward left, then bottom, then top (top last so it stays least-used).
   - A node's **top side is avoided** for inbound ports except on top-row nodes,
     which can route up into the title-region lane.
   - Hints override the chosen side per edge (except the prohibited inbound
     "right"). Multiple ports on one side stack on separate tracks, ordered by the
     other endpoint's position to reduce immediate crossings; overflowing a side's
     capacity is a validation error naming the overflowed edges.

4. **Routing.** Cell-grid shortest-path search over the canvas, two passes.
   - **Edge bodies are confined to the lanes.** An edge may enter a node region
     only on its own short port stubs (the first and last segments that connect to
     its ports); every interior segment stays in lane bands. Other node regions
     and the title band are walls.
   - **Lane-centering.** Among otherwise-equal routes an edge gravitates to the
     centre track of its lane, which keeps runs off the box edges and spreads
     parallel edges out.
   - **2×2-cluster spacing.** No two distinct edges may occupy cells that share a
     2×2 block — i.e. they keep at least one blank cell between them horizontally,
     vertically, AND diagonally. Port stubs inside node regions are exempt (ports
     legitimately bunch).
   - **Cost priority (each term an absolute tie-breaker for the next):**
     crossings, then adjacency (cells abutting another edge with no gap), then
     turns (fewest elbows), then distance from lane centre, then length.
     Turns strictly outrank centring, so a straight run is never bent merely to
     centre it.
   - **Pass 1 — no crossings, hard spacing.** Cells occupied by another edge are
     blocked; the 2×2 spacing rule is a hard block. Edge order: hinted
     routing-order first, then shortest first.
   - **Pass 2 — only edges that failed pass 1.** Crossings are permitted but
     dominate the cost (the router still minimises them); each crossing is an
     error naming both edges. Spacing is **demoted to a soft cost** here rather
     than disabled, so a crossing edge keeps its gap everywhere it can and hugs
     another edge only at the unavoidable crossing point — never running flush the
     whole way.
   - Edges unroutable even in pass 2 are omitted and reported.

5. **Render + report.** Always-on scaffolding (labeled rulers, spine/separators),
   the title block, boxes drawn with box characters and centred labels, and each
   edge body drawn entirely in its own assigned character with a port mark at the
   source and a directional arrowhead at the target. At a crossing the vertical
   edge's character is drawn over the horizontal one.

## Resolved output

Beyond the resolved nodes/edges/ports/routes and `status`/`errors` (codes:
validation, crossing, unroutable; each with a human suggestion phrased in terms of
the caller's knobs — grid moves, port hints, routing order), the result also
carries:

- A **quality scorecard** — integer counts, lower is better, in strict priority
  order: dropped (edges with no route), wraps (edges that loop the canvas),
  crossings, top-ports (top ports on non-top-row nodes), congestion (lane cells
  where two edges run with no gap), and total length. This is the objective the
  auto-loop minimises.
- **Diagnostics** — actionable entries for the soft defects (wrap, congestion)
  naming the specific edges, so an iterating caller knows which ports to perturb.

## Self-sufficient auto-loop

A companion entry point runs the engine in a loop: start from the naive input
(hints optional), route, read the engine's own diagnostics, and commit the single
port-side or routing-order change that most improves the quality scorecard;
repeat until clean or a local optimum, then write the best result found. It is:

- **Deterministic** (same input → same output; no clock/randomness; fixed
  candidate order),
- **Bounded** (a cap on attempts; performance is a known, deferred concern),
- **Best-improvement** (evaluate a full round of candidate changes and take the
  best, which avoids the trap where an early small win blocks a larger one),
- **Non-destructive to user intent** — it never changes a port side the user
  pinned; it only fills in or adjusts unspecified sides — and it reports the hints
  it settled on so the caller can pin them.

## Skill behaviour and acceptance gate

`SKILL.md` documents the engine (correct script path, output filenames, the grid
rules, hint semantics including the inbound-right prohibition, the quality/
diagnostics schema) and leads with the auto-loop as the primary path, with manual
hint-tuning as the fallback.

Acceptance (moth gate): driving the skill end-to-end, non-interactively, on the
rebob system tree below must produce a `result.txt` that is visually approved by
both the implementing agent and the user. Iterate (engine and/or hints) until
approved.

```
System: rebob
  ├─ Backend: dispatcher [uses: sub-bob-manager]
  ├─ Backend: chat [uses: event-storage]
  ├─ Backend: queue
  ├─ Backend: memory [uses: llm, queue]
  ├─ Backend: event-storage
  ├─ Backend: sub-bob
  ├─ Backend: consolidator [uses: llm, memory, queue]
  ├─ Backend: llm
  ├─ Backend: bob-registry
  ├─ Backend: sub-bob-manager [uses: chat, memory]
  ├─ Client: @bob/sub-bob-bootstrap [uses: memory]
  └─ Frontend: @bob/frontend [uses: bob-registry, chat, memory, sub-bob-manager]
```

## Decisions taken and rejected

- **Cell-grid routing, not a lane-junction graph.** The original design sketched a
  lane-graph A* where each edge claims a whole track so overlap is "impossible by
  construction." Implemented as cell-grid search instead: simpler, and it makes
  spacing/centring/confinement expressible as cell-level rules. Overlap is
  prevented by occupancy + the 2×2 rule rather than by graph structure.

- **Confine edge bodies to lanes (taken).** Early renders let edges cut through
  node-margin cells. Rejected; bodies are now lane-only except port stubs, which
  is what makes the diagram read as orthogonal lanes.

- **Add a routing lane above the first node row + avoid top ports (taken).**
  Without it, edges approaching the top row from above hugged the box tops. Top
  inbound ports are now avoided except for top-row nodes that use this lane.

- **Prohibit inbound "right" ports (taken).** Data flows left-to-right; an inbound
  port on the right reads backwards and forces awkward routing. Inbound side is
  chosen from left/top/bottom by fewest elbows; a right `to_side` hint is rejected
  at validation. (Superseded the original spec's "backward edge enters the
  target's right side" default.)

- **Strict-lexicographic cost, turns over centring (taken; weighted-sum
  rejected).** A weighted-sum cost (turn-weight × turns + centre-weight × offset)
  let a long enough off-centre straight run outweigh two turns, so edges jogged to
  centre and back. Replaced with a strict tuple where centring only breaks ties
  among equally-straight routes.

- **2×2 (king-adjacency) spacing, not perpendicular-only (taken).** A first cut
  only kept perpendicular parallel runs apart; diagonal touching still looked
  cramped. Strengthened to "no shared 2×2 block" in every direction.

- **Pass-2 spacing as a soft cost, not disabled (taken).** Disabling spacing
  entirely for a crossing edge made it run flush against unrelated edges for its
  whole length. Demoted spacing to a cost term in pass 2 so it hugs only at the
  crossing itself.

- **Loop objective ranks wraps above crossings, and dropped edges above all
  (taken).** A wrap that loops the whole canvas reads worse, and is harder to fix
  by hand, than one clean crossing; and a search must never "win" by dropping an
  edge. The scorecard order encodes this.

- **One clean crossing accepted over a forced wrap for dense graphs (taken).**
  Some dense hubs (a node with 4+ edges on one side, e.g. rebob's memory and
  frontend) have no zero-crossing layout reachable by ports alone; a single clean
  crossing beats contorting an edge into a canvas-spanning detour.

- **Self-sufficiency loop lives in the skill, as a bounded best-improvement
  hill-climb (taken; exhaustive search rejected).** Per-route evaluation is too
  slow for an exhaustive sweep, so the loop is a bounded, deterministic
  hill-climb driven by the engine's diagnostics. Further speedups are deferred
  (performance treated as a future concern).

- **Out of scope (unchanged):** a unicode edge alphabet beyond 62 edges,
  downstream renderers consuming the JSON, and automatic node placement (the
  calling LLM places nodes).
