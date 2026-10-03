fix cases/container.json to match our schema and render with groups.
a group follows the same rules as the diagram, preferring (but not requiring) data input on the left and data output on the right.

group spatial location is decided like nodes. a data flow from group to group suggests left-to-right, but way less strict (two start nodes in different groups will rightly end up in a vertical stack)

a group also has a title in the top left, single line, bold, same font size as node label.

neighbor groups at the same level are aligned and prefer to have same size on the neighboring side. e.g. a vertical stack of groups have same width, a horizontal one - same height. if a group has neighbours in both direction, horizontal sizing takes precedence (storytelling)

----- AI agent updates -------

Design doc: docs/superpowers/specs/2026-10-01-groups-design.md
Implementation plan: docs/superpowers/plans/2026-10-01-groups.md

## Decisions

- Schema: optional flat top-level `groups` list (id, optional label, optional parent `group`); nodes get optional `group`. Nesting at any depth. Nodes and groups share one id space.
- Edges may end on a node or on a group — both allowed, they are different relations (e.g. a client "uses" a whole service; a group may point at one of its own members). Groups have no kind; edge kind = edge hint, else node endpoint kind, else data.
- Empty groups are valid and drawn as one cell with the title only.
- Graphs without groups lay out exactly as before.
- container.json is rewritten into the engine schema: the system node (`system:rebob`) becomes the diagram title (the frame already surrounds everything), `contains` becomes group membership, `uses` and `handles` become data edges (`uses` is a hint of a data relation; `handles` = the service's API node passes a request to its handler), names become labels, source types/attributes dropped.
- Source data issues were fixed upstream by the user (not by the engine): shared `runtime` and self-containment ("dispatcher contains dispatcher") replaced by distinct per-service nodes; `handles` starting at the service group was a bug and now starts at the service's API node. A node belongs to at most one group. Isolated nodes (mcp containers, unused API/legacy nodes) are laid out as they are.
- The container dump went through several user revisions; all use the same conversion. Function-level versions (handlers and functions inside each service) had only node→node edges; adding the client apps (top-level, outside every group) with function-level cross-service uses grew it to 110 edges and ~4 min of layout. The current case is the service-level view: 5 client apps (top-level nodes) use whole services (edges ending on groups), services contain only their containers (api, runtime, mcp, bus, graph, queue), no functions.
- The conversion is a dev script (user request), so new dumps need no hand work: dump in, case file out. Groups = everything that contains something plus every backend; containment by the system node means top level; relations touching the system node are dropped; an unknown relation kind is an error rather than silently dropped (new kinds need a decision on how to draw them). The raw dump is kept next to the case it was converted into (`container.json.c43`), so the case can be regenerated.
- Approach: hierarchical blocks. Each group's children are laid out bottom-up by the diagram rules (child groups act as super-nodes of their block size); edges are lifted to the level of the innermost group containing both ends; edges leaving a group are seen from inside as input from the left / output to the right (preference, not requirement); a group→own-descendant edge is input from the left. Blocks then expand into one global uniform grid (IDEA: globally aligned equal squares, equal lanes); ports, routing, tracks and checking stay global; global repair is bounded so cost follows the largest group, not total node count.
- Neighbour sizing by stretching a block's cell span; the interior is centred in the stretched block. Vertical stack → same width, horizontal row → same height, horizontal wins when both apply.
- Group borders live in the lanes around the block, alongside edge tracks; nested borders take separate positions; edges may cross borders but never run along one; an edge stays inside the innermost group containing both ends and crosses only borders it must. Lane width stays global and grows to fit tracks + borders + titles.
- Group title: inside top-left, in the lane above the group's first row, single line, bold, label font size; a title wider than the block widens the block.
- Group ports follow node port rules (exit right/bottom, exceptionally top, never left; entry left/top, never right; evenly spread; never corners). A group→own-descendant edge starts on the inner side of the left border and flows right (request enters from the left).
- Checker gains group rules (rect holds exactly its descendants, rects nest or are disjoint, no edge along a border, group port rules) and a soft score for unequal neighbour sides.
- Cases: a small synthetic nested case asserted at zero hard violations; container is a stress test (like star16) — huge equal node side (11 inbound on memory/graph, long function labels) and wide lanes are expected and accepted; it must finish in reasonable time (~1 min target).

## Rejected

- Flat layout with group-cohesion penalties: no rectangle guarantee, group-to-group L→R hint hard to express, repair cost infeasible at ~80 nodes.
- Boxes drawn around a flat layout: groups overlap, no storytelling.
- Retargeting group-endpoint edges to a representative inner node.
- `system:rebob` as an outermost group (redundant with the frame).
- One-level-only nesting.

## Out of scope

Edge labels, per-type node styling, collapsing groups, using source `type`/`attributes`.

## Decisions made during implementation

- Groups at a level that have no data flow between them at that level are "secondary" like unconnected nodes (ehfb0) and line up in a row at the bottom; only groups with start nodes feeding something stack vertically. (Found by a test that had two unrelated groups.)
- The edge from a group to its own member starts on the inner side of the group's left border; a member pointing at its own group (or a group at its parent) is rejected as input — it has no sensible drawing.
- An edge with a group end always gets a track in its lane, even when its ports looked aligned while routing: group borders only get their final place once lane widths are known, so a "straight across" shortcut would come out diagonal.
- A group's title box is only as wide as its text, not the whole group: edges entering a group from above may pass beside the title but never through it (checked as a hard rule).
- Graphs without groups keep exactly the old layout (verified byte-identical on aws, rebob, star16).
- Global repair of a grouped layout only moves ports (never nodes — that would break the blocks) and has a small evaluation budget: each evaluation is a full layout, and on the container case extra budget bought almost nothing (102 → 97 crossings for 60 evaluations, ~90 s). Rejected: the larger budget (200) — minutes per diagram.
- Speed: comparing edge pairs while improving tracks now skips pairs whose bounding boxes are apart and reuses each polyline's segments; same results, ~5× faster (also benefits graphs without groups).
- Isolated nodes (likely data bugs in the container dump: mcp containers, unused API/legacy nodes) are deliberately laid out as they are, so they stand out.

## Implementation (abstract)

- Input: optional flat group list with parents; node `group`; one id space; validation of ids, parents, cycles and self-pointing edges. Tree helpers: groups holding an id, owner (innermost group holding both ends of an edge), child of a level containing an id.
- Hierarchical placement: per group, bottom-up, its direct children form a small graph (child groups as single super-nodes, edges lifted to the owner level, edges leaving the group replaced by a virtual input on the left / output on the right) laid out by the existing flat pipeline; the resulting macro grid expands into real cell spans (column width = widest item, row height = tallest), neighbouring groups stretch to share the touching side, interiors are centred; a long title widens its block; empty group = one cell. The root block is the global grid.
- Ports: group ends get ports on the group's border with the node rules; the inner start of a group→member edge sits on the left border.
- Routing: an edge may only use lanes inside the block of its owner group; stepping through the inside of a group holding neither end costs as much as a crossing.
- Lanes: each lane is cut into slots; group borders take slots at the lane's sides (innermost nearest the cells), edge tracks sit in bands between borders chosen by the edge's owner, top borders keep room for the title; the global lane width fits the busiest lane. Without groups this reduces to the old centred lane.
- Output: group rectangles (border positions from the lane plan), titles, node group membership, inner-start flag on ports. Checker: members enclosed, foreign nodes outside, groups nested or apart, no edge along a border or through a title, group port rules, soft count of unequal neighbour sides. drawio: group rectangles and bold titles behind nodes; group endpoints attach to the group cell.
- Dev render script scales very large diagrams down so drawio can export them as PNG, and also exports every case as a full-size SVG (zoomable, searchable text; light theme). The drawio export paints a white background behind everything, since drawio's SVG export is otherwise transparent (user request: PNG of container not useful, SVG with white background wanted).

## Results

- groups (synthetic, nested, group endpoints, empty group): 6×3, node side 4, lane 9, 0 violations, 0 crossings, ~0.05 s.
- container (final, service level with cross-service uses: 11 groups, 26 nodes, 29 edges — 13 client → service group, some service group → node, member → member across services): 11×10, node side 6, lane 13, 37 crossings, no other violations, ~30–60 s. The test's time limit was raised from 60 s to 120 s (the ~1 min target holds alone; the parallel test suite slows it).
- Previous service-level version (19 edges): 7×12, node side 6, lane 11, 10 crossings (all among client → service edges), no other violations, ~23 s. Clients read on the left, services to their right, services nobody uses (queue, consolidator, llm, llm-legacy, empty sub-bob) gather at the bottom. Viewed as a full-size SVG with white background.
- Earlier function-level container: 13×18, node side 11 (memory/graph has 11 inbound), lane 21, 100 crossings, ~45 s; with clients added (110 edges): 19×15, 380 crossings, ~4 min — too slow; not pursued since the case moved to the service level.
- aws / rebob / star16: unchanged.

## Findings

- container's crossings concentrate where the consolidator's ten functions fan out to graph, llm and task-queue in three other groups; port-only global repair barely helps. Candidates for later: group-aware node moves in global repair, crossing-aware ordering of groups at the root level.
- Equal lane width (IDEA) makes grouped diagrams very airy: the busiest lane (fan-in at memory/graph plus borders and titles) sets 21 units for every lane.
- Neighbour stretching makes small groups big when they share a row or column with a big one (llm-legacy, queue, sub-bob in container).
- Full-scale PNG export of container fails or comes out blank in drawio (size limit); the .drawio itself is fine.
- Final review (fresh reviewer, fuzzing 200 small grouped graphs): no crashes, validation solid. Fixed after review: nested borders sharing a lane side are ranked by nesting depth (siblings share a rank) — counting them inflated the global lane width; a group widens for its title only when the title cannot fit even with the border inset a group always has — short titles were doubling small diagrams.
- Open (follow-up candidates): small grouped diagrams show avoidable crossings noticeably more often than flat ones (~13–23% vs ~3% of random small cases): grouped layouts cannot move nodes in global repair, and the router does not count a crossing where another edge turns at the same junction. Also: the virtual input/output of a group are not pinned left/right, so a group's output member can land left of its input member; a rare title crossing port moves cannot fix.




