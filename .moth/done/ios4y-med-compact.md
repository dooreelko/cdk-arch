the generated cases/container is correct but is also very spacious
with keeping the layout and the rules of nodes and groups aligned and groups sharing the sizes with neighbours, we need to compact the diagram. obviously without introducing new overlaps.

----- AI agent updates -------

Design doc: docs/superpowers/specs/2026-10-03-compact-design.md
Implementation plan: docs/superpowers/plans/2026-10-03-compact.md

## Decisions

- Cause of the air in container: one global lane width set by the busiest lane (fan-in plus nested group borders and a title) applied to every lane; lanes were about twice the node side. Other sources (groups stretched to their neighbours' size, global rows/columns leaving empty cells in groups, placement) are left alone — "keep the layout".
- Compaction = per-lane width: every vertical and horizontal lane is only as wide as its own contents need (tracks, group borders, title room), odd, floor 3 for inner lanes; the four outer lanes at the frame edge keep a fixed breathing margin (more if their contents need it). Floor and margin are provisional, to be reviewed by the user on real diagrams.
- Cells stay equal squares; column/row origins are cumulative, so nodes stay globally aligned on rows and columns. Neighbour groups keep equal touching sides as they had before (they span the same lanes); per-lane widths neither fix nor worsen the existing rank caveat (see Findings).
- Applies to all diagrams, grouped and flat (user choice); flat outputs change. IDEA's "nodes are evenly spaced" becomes "cells are equal; gaps fit what runs through them".
- Only final geometry changes; placement, ports, routing, tracks and repair keep their decisions. Hard violations and crossings must not get worse. Output reports per-lane widths instead of a single lane width.

## Rejected

- Keep equal lanes and squeeze the grid instead (drop empty interior cells of stretched groups, thinner rows): small gain, changes the layout.
- Variable width along a lane (VLSI-style compaction): breaks global alignment.
- Width-aware routing now: couples routing to geometry, slower; follow-up candidate.
- Grouped-only scope (keeping flat diagrams byte-identical): user preferred one rule for all.

## Out of scope

Placement changes, shrinking stretched groups' interiors, width-aware routing, per-diagram tuning of floor/margin.

## Implementation (abstract)

- Geometry no longer assumes one pitch: it asks each lane for its start and width. Routing and track assignment keep a uniform abstract geometry (every lane 1 wide), so their decisions are untouched.
- The lane plan sizes every lane from its own bands, borders and title room (odd; floor 3 inside, margin 5 at the frame edge, both named constants) and accumulates lane starts; all final coordinates (nodes, ports, tracks, borders, group rectangles, frame, header) derive from it.
- Layout output lists the widths of all vertical and horizontal lanes in place of the single lane width; the checker's node-on-cell rule derives expected positions from them; the CLI summary prints the widest lane and frame size.
- Tests: per-lane widths/floor/margin/starts, one-node diagram (both lanes outer), checker with uneven lanes, container regression (no more crossings, area under 80 % of before, neighbour groups drawn with equal touching sides).

## Results (grid, widest lane, frame before → after, crossings unchanged)

- container: 11×10, lanes ≤ 13, 222×206 → 136×130 (~39 % of the area), 37 crossings, no other violations.
- groups: 6×3, ≤ 9, 87×53 → 55×41, 0.
- star16: 5×6, ≤ 9, 104×123 → 76×91, 0.
- rebob: 4×6, ≤ 5, 45×70 → 43×60, 1.
- aws: 6×4, ≤ 5, 51×47 → 55×51, 0 — grows: its lanes were all 3 and the outer margin 5 adds 2 per side.

## Findings

- The outer margin enlarges diagrams whose lanes were already minimal (aws); input for the user's margin/floor review.
- Correction to "neighbour groups keep equal touching sides automatically": that holds only when both neighbours' borders have the same nesting rank in the shared lane. A group containing a nested group sharing its top/bottom side draws its border one rank further out (groups case: api vs store, same rows, different drawn heights). Pre-existing, not caused by per-lane widths; candidate follow-up.
- Final review findings for the user's floor/margin review: the outer margin is consumed by borders and titles rather than added outside them (a group border can sit 1.5 units from the frame edge).
- What remains spacious in container is groups stretched to their neighbours (event-storage, consolidator, queue spanning three rows for one node) and global rows leaving empty cells inside groups — out of scope here, candidate follow-up.

## Status

Accepted by the user as done (2026-10-03). Floor 3 and outer margin 5 stay provisional — open for review in a later task.




