create an initial engine to run in nodejs that works on two cases:

1. The architecture case from IDEA.md
2. "Extreme" case of 16 nodes where first node is the starting (so left/middle), second (main) node follows from first and all remaining nodes are linked from the second. First 5 are "data flow" nodes, last 7 are non-functional.


----- AI agent updates -------

Design doc: docs/superpowers/specs/2026-09-30-initial-engine-design.md

## Decisions

- Output: engine emits a layout JSON (grid, node cells, ports, orthogonal polylines, metrics); a separate exporter turns it into .drawio. PNG rendering only as dev aid.
- Input: nodes + edges, each with optional kind hint (data | nf), default data. Hints may come from a human or an agent. Effective edge kind: explicit edge hint, else nf if either endpoint is nf, else data (Lambda→SES data, CW→SES nf, CF→R53 nf).
- Approach: all layout is own code, no layout library. Skeleton = layering of data subgraph (columns) + in-column ordering heuristic; then grid placement, sizing, port assignment, routing, repair. The IDEA rules are the product; library layouts don't honor them and would mostly be overridden anyway.
- Stack: TypeScript on Node, zero runtime dependencies, synchronous engine.
- Graph-paper model: uniform grid of equal cells, all nodes equal squares centered in cells, all liminal lanes equal width. Node side = max ports on any side, ports at half-unit positions (3 ports → 3x3, per IDEA), never corners; fewer ports on a side spread evenly. Lane width = max parallel tracks needed.
- Ports: default exit right, entry left. Short link preferred for left→right (right→left) and top→bottom (bottom→top) relations, e.g. CF→R53. Upward links use the default (CW→SES enters SES left). Entry stays left whenever the source is to the left — an nf edge to a target below-right leaves the bottom and enters on the left; top entry only when the target is directly below or below-left (the side facing the source). Rejected: pairing a bottom exit with a top entry for every nf edge (put all nf entries on top). Top exit only exceptionally; never exit left (IDEA originally said "never right" for exits — a typo, since corrected in IDEA.md); never enter right.
- 4+ same-kind outs: the right side keeps 3 (IDEA: fewer than 4 stay on the right); the overflow peels off the end of the clockwise target order that lies farther from the node's row — targets below leave from the bottom, targets above from the top (top stays exceptional), targets level with the node stay right the longest so a right-hand neighbour keeps its straight link. "Data right, nf bottom" is a preference, not a hard rule: data overflow shares the bottom with nf outs (star16: right 3, top 1, bottom 3 data + 7 nf → node side 10). Rejected: splitting data outs half right / half top (rebob: frontend→bob-registry, its right-hand neighbour, looped over the top).
- Pipeline of pure stages: normalize → skeleton → place → ports → size → route → repair (bounded patience) → emit. Deterministic.
- Crossings: repair rearranges ports and nodes to minimize crossings, ideally to zero. Both cases expected to reach zero; if not, surfaced as a finding rather than loosening the check.
- Success: automated checker asserts zero hard-rule violations (orthogonal, equal squares in equal cells, no shared/corner ports, even port spread, no left exit / right entry, no overlapping parallel segments, zero crossings) on both cases; soft scores (L→R, centrality, compactness, top-left empty, nf low, length, turns) reported; human eyeballs rendered PNGs. perfect.drawio is a sketch of intent for the aws case, not an exact target.
- star16 clarified: 16 nodes = start → main → 14 leaves off main, 7 data + 7 nf (original text's 5/7 counts corrected). It is a stress test to find limits; awkward output (huge equal nodes from the main node's crowded bottom side — 7 nf leaves plus any data leaves repair moves below it — and wide lanes) is expected and accepted.

## Rejected

- Layout library end-to-end (placement + routing): routes ignore lane-centering, even grid, nf-to-bottom; post-snapping breaks routes.
- Layout library (elkjs/dagre) for skeleton only: was chosen at first, then dropped — own placement overrides most of its output, adds dependency and async for little gain on small graphs.
- Capping node size / relaxing equal-size rule for star16: rejected, stress test should show the rules' limits as-is.
- Integer-grid ports with side = ports+1 (perfect.drawio style 0.25/0.5/0.75): contradicts IDEA's 3→3x3.
- Allowing left exits for overflow.

## Out of scope

Groups (incl. nested) — cases don't use them; lanes model kept group-ready for a later task. Edge labels, styling, agent-skill packaging, interactive editing.



## Decisions made during implementation

- Secondary (nf-only) nodes never sit on the wrong side of their anchor: a node that points at its anchor stays at or left of it, a node its anchor points at stays at or right of it. Keeps every edge left-to-right (aws: CW lands under Athena, as in the sketch).
- Repair ranks IDEA's story rules above crossings: hard rules > no leftward edges > nf at bottom > crossings > turns > length. An earlier order (crossings first) let repair wreck the story (leftward edges, nf node to the top-right) to remove crossings — rejected.
- Wide data columns wrap: a column holds at most max(3, ceil(sqrt(node count))) nodes; the rest continue in the next column.
- Straight shots through empty cells only between directly facing ports on the same line; everything else travels in liminal lanes.
- Port reordering takes precedence over moving nodes when removing crossings. Track order inside a lane is optimised as a whole (exhaustively for small lanes) so a port reorder is judged on its best routing, not on a greedy one; only when no port move helps does repair move nodes.
- A data edge between two nodes in the same column (e.g. ATH→S3 leaving right, entering left) is acceptable story-wise; only edges pointing left count against the reading direction. Rejected: treating same-column data edges as story breaks.
- Nodes with no connection to the data flow and no anchor go below every data row; nf nodes anchored to a node may sit in the bottom data row (R53 level with LC, as in the sketch).
- Input is validated for shape (arrays, objects, string ids/labels) with clear errors; labels are plain text in drawio (never interpreted as HTML).
- Once the layout is clean, repair keeps shortening links (fewer turns, less length — IDEA "connections as short as possible"), but only with port moves and vertical node moves: a node never changes column, because columns carry the left-to-right story (e.g. Lambda C moves up under Lambda B, level with Athena; S3 stays rightmost). Rejected: unrestricted moves for length (moved S3 into Athena's column and made SES the rightmost node) and pinning only sinks.
- Port order on a side follows the far end, read clockwise around the node: the lower the far end, the lower (further clockwise) the port — right side top→bottom, bottom side right→left. On a tie the far end needing the longer detour goes outside (same row on the right: farther target higher; same column below: deeper target further left).
- Two edges running side by side through several lanes may swap tracks in all shared lanes at once; single-lane changes cannot untangle nested detours.
- Routing is crossing-aware: a crossing costs more than any number of turns (IDEA ranks "no crossings" above "fewest turns"/"shortest"). Data edges are routed first and nf edges after, so nf edges detour around the data flow rather than the reverse; then every edge is re-routed against all others for a few rounds. A route cannot reverse within a lane (no U-turn to dodge a crossing).
- While untangling, repair may move data leaves into their source's column (a leaf above or below its source is fine, even if that grows node sides) — accepted. Rejected: locking columns during untangling (star16 back to 8 crossings, aws worse), penalising node growth, keeping data leaves out of the source's column.
- Data sources (nodes with no incoming data edge — initiators) start the story on the left: repair never moves one rightwards. Found with the ad-hoc rebob graph, where repair had pushed sources (bootstrap, consolidator) into later columns.

## Implementation (abstract)

- Pure stages: validate/normalize → layer the data subgraph (cycles broken, longest path, barycenter order, column wrap) → rows centred on predecessors, secondary nodes to a bottom band near their anchor, top-left cell kept empty → port sides/slots → routing → lane tracks → geometry → checker → repair.
- Routing: fewest-crossing, then fewest-turn, then shortest path through the network of liminal lanes (vertical/horizontal lanes meeting at junctions), crossings estimated from the lanes other edges occupy; data edges first, nf after, then rip-up-and-reroute rounds; direct straight line between facing ports over empty cells.
- Parallel segments sharing a lane get distinct tracks, middle first; lane width grows (odd) to fit; track order per lane is then searched (exhaustive for small lanes, local moves otherwise) to remove crossings/overlaps.
- Checker reports hard-rule violations (orthogonal, equal squares on the grid, shared/corner/uneven ports, left exit / right entry, through-node, overlaps, crossings) and soft scores (leftward, centrality, area/aspect, top-left, nf at bottom, length, turns).
- Repair: first-improvement hill-climb over slot swaps, right↔top and right↔bottom exchange of data outs, node moves/swaps (never pushing a source rightwards), in two phases — untangle (any move, while violations remain) then shorten (port moves and same-column node moves only); bounded patience; deterministic.
- drawio export carries explicit waypoints and exit/entry constraints so drawio draws the engine's routes instead of re-routing.

## Results (initial engine)

- aws: 6×4 grid, node side 3, 0 violations, 0 crossings, all story rules met, S3 rightmost.
- star16: 5×6 grid, node side 10, 0 violations, 0 crossings, story rules met.
- rebob (ad-hoc, 12 nodes, 4 sources, one isolated node): 4×7 grid, node side 4, 1 crossing remains (not yet investigated). Kept as a sample case (its extra fields — title, description, grid hints, edge ids — are ignored by the engine) but not asserted by tests; follow-ups (its remaining crossing, title in the top-left cell) left for later tasks.

## Findings

- (resolved) star16 crossings: a crossing-blind router left nf edges cutting through the wall of data edges below the main node; fixed by crossing-aware routing plus clockwise port order and joint track swaps.
- Speed: repair rebuilds routing for every candidate move; star16 lays out in a few seconds — acceptable for the initial engine.







