# Groups — visual grouping in the c43 ASCII layout engine

**Moth issue:** `bvthw` (med, doing) — "Groups"
**Date:** 2026-06-13

## Original task (from moth)

> the diagrams should be able to visually group elements.
>
> the idea is to
> - add a vertical edge lane on the left (but avoid using it for edges since we prohibit inbound ports on the left)
> - render rectangles surrounding a group of nodes on the edge lanes
> - while edges gravitate to the middle, groups stay close to the nodes
> - edges are allowed to cross group boxes' sides
> - edges are prohibited from having segments running on group boxes' sides
> - same padding rule applies to groups - at least one space in each direction between all edges and groups
> - a group has a title, rendered inside at the bottom, one space from the left side, one space above bottom one
>
> use the following architecture for tests. Each node with children is a group (e.g. there will be nested groups).
>
> (the rebob System > Backend > container tree — see moth issue for the full listing)

## Goal

Let the c43 layout engine draw nested rectangular **group frames** around sets of
nodes. Group frames live in the edge lanes, hug the node side of each lane while
edges keep the lane centre, support arbitrary nesting depth, and carry a title.

Implement in Rust first (the `packages/c43` engine), then translate to the
Python reference (`claude-plugin/c43/skills/ascii/scripts/layout.py`) to preserve
the byte-identical parity oracle. Update the ascii SKILL.md to document the new
`groups` schema.

## Non-goals

- Generating `groups` from the C4 container tree (`container.rs` / `ascii.rs`).
  That is a separate downstream task. This task only extends the `layout.json`
  schema and the layout engine. The C4 tree (`c43 container --ascii`) keeps using
  its existing separate tree renderer in `ascii.rs`, untouched.
- Any change to edge routing semantics other than the group-border interaction
  described below.

## Architecture overview

The layout pipeline is `parse → geometry → ports → route → render`. Groups thread
through every stage:

- **parse**: read + validate the new `groups` array into `Group` structs.
- **geometry**: compute group grid extents (members ∪ descendant groups), size
  lanes dynamically to fit the group border "rings" that pass through them, add
  the new bounding lanes (left, right, bottom), then assign each group its pixel
  box and each border its ring track.
- **route**: classify group-border cells so edges may cross them perpendicularly
  but never run along them, and keep ≥1 space padding.
- **render**: paint group frames (after scaffolding, before nodes) in double-line
  Unicode, with titles.

## Section 1 — Schema & data model

Extend `layout.json` with an optional top-level `groups` array:

```json
"groups": [
  { "id": "rebob",      "title": "rebob",      "members": ["dispatcher","chat"], "parent": null },
  { "id": "dispatcher", "title": "dispatcher", "members": ["runtime","disp_mcp"], "parent": "rebob" }
]
```

- `id` — unique group id (string).
- `title` — text rendered inside the frame.
- `members` — **leaf node ids** that belong directly to this group. Child groups
  are linked via their own `parent`, not listed here.
- `parent` — id of the enclosing group, or `null`/absent for a top-level group.

A group's **grid extent** is the bounding rectangle (in grid cells) of its direct
members' cells **unioned with the extents of all descendant groups**. So a parent
group with no direct members still encloses everything its children enclose.

### Data model (`model.rs`)

New struct, and `Model` gains `groups: Vec<Group>`:

```rust
pub struct Group {
    pub id: String,
    pub title: String,
    pub parent: Option<String>,
    pub member_ids: Vec<String>,
    pub depth: i64,            // nesting depth, 0 = top-level (set in geometry)
    // grid extent (inclusive grid-cell indices), set in geometry:
    pub col0: i64, pub col1: i64,
    pub row0: i64, pub row1: i64,
    // pixel box, set in geometry:
    pub x: i64, pub y: i64, pub w: i64, pub h: i64,
}
```

### Parse validation (errors, `code: "validation"`)

- unknown member id; unknown parent id; duplicate group id;
- cycle in the parent chain;
- a group's computed extent encloses a node that is neither a member nor a member
  of a descendant group → error (`group X encloses non-member Y`);
- two groups **partially** overlap → error (`groups X and Y overlap`). Full
  containment (true nesting) is allowed; partial overlap is not.

Validation is consistent with the existing validation-heavy parse stage. Each new
check has a unit test.

## Section 2 — Geometry (dynamic lanes + bounding lanes)

Today geometry lays out alternating node columns and fixed-width edge lanes
(`LANE_MIN_W` / `LANE_MIN_H`) starting just right of the gutter spine at
`GUTTER_W + 1`, plus a title row and a top lane. There is no lane left of column 0,
nor right of the last column, nor below the last row.

Changes:

1. **Bounding lanes.** Add a left lane (region index `-1`, just right of the
   gutter spine, before node column 0), a right lane (after the last node column),
   and a bottom lane (after the last node row). The top lane already exists. These
   host the outermost group frames. The **left lane is never used for edges**
   (inbound-left ports are prohibited), satisfying "add a vertical edge lane on
   the left."

2. **Group extents & depth.** Resolve each group's `col0/col1/row0/row1` from
   members ∪ descendants (bottom-up over the parent tree). Compute `depth`.

3. **Dynamic lane width/height.** For each vertical lane sitting between node
   column `c` and `c+1`, collect the group borders that must pass through it:
   - on the lane's **left** sub-region: the **right borders** of groups whose
     extent ends at column `c`;
   - on the lane's **right** sub-region: the **left borders** of groups whose
     extent starts at column `c+1`.

   Borders are ordered by nesting so the **innermost group hugs the node** and
   outer groups stack toward the lane centre. Width is:

   ```
   left_rings + PAD + EDGE_TRACKS + PAD + right_rings   (floored at LANE_MIN_W)
   ```

   where `PAD = 1` (the ≥1-space padding rule) and `EDGE_TRACKS` is the existing
   edge-routing centre allowance. The **edge centre track stays centred** so edges
   gravitate to the middle while group rings hug the nodes. Horizontal lanes use
   the same logic with `LANE_MIN_H`. The left/right/bottom bounding lanes are sized
   the same way (they only ever carry the outermost rings).

   This means **geometry must know group extents before sizing lanes** — extents
   (step 2) are computed before band/`col_x`/`row_y` construction.

4. **Pixel boxes & ring tracks.** With lanes sized, assign each group border to its
   ring track (deepest nesting innermost) and fill in each group's `x,y,w,h`.

5. **Band caches.** Lane band `center` values (used by the router for the
   centre-offset cost term) must remain the **edge centre track**, not the lane
   geometric centre, so asymmetric ring padding does not pull edges off centre.

Each piece (extent resolution, lane-width math, ring assignment) gets a unit test.

## Section 3 — Routing interaction

Group-border cells get a classification distinct from `occupied` (edge-claimed)
cells. In the A* step (`route.rs::astar` / Python equivalent):

- **Crossing a border is allowed, perpendicular only, no penalty.** An edge may
  step onto a border cell only as a straight pass-through (the move into and out
  of the cell keep the same direction — no turn on a border cell). Unlike
  edge–edge crossings, this carries **no cost penalty**.
- **Running along a border is blocked.** A border cell cannot be entered when the
  move direction is parallel to that border's segment orientation (vertical border
  ⇒ no vertical move onto it; horizontal border ⇒ no horizontal move onto it).
- **≥1-space padding.** Cells immediately adjacent to a border on the lane-interior
  side are added to the existing `forbidden` set, reusing the king-move padding
  mechanism, so edges keep at least one space from group frames.

Because rings pack to the node side and edges hold the lane centre, edges seldom
touch borders except when legitimately crossing into/out of a group.

Tests: an edge crossing a border perpendicularly succeeds with no penalty; an edge
cannot route along a border; padding keeps edges off the border by ≥1 cell.

## Section 4 — Rendering

- **Paint order:** scaffolding → **group frames** → node boxes → edges. Edges
  paint last, so at a legitimate crossing the edge character overprints the border
  (consistent with existing edge overprint behaviour).
- **Glyphs:** double-line Unicode box-drawing `╔ ═ ╗ ║ ╚ ╝`, one glyph per cell.
  The Canvas already stores `char`s, so multibyte glyphs occupy one cell.
- **Title:** rendered **inside** the frame at the bottom — one space right of the
  left border, on the row one above the bottom border.
- **Nesting:** nested frames nest visually because their rings were placed
  inner→outer in geometry.

## Section 5 — Testing & delivery

- **Golden fixture:** a nested `groups` `layout.json` derived from the issue's
  rebob architecture (System > Backends > containers), plus its `expected_*.txt`.
  Add to the Rust golden suite (`tests/golden.rs`) and the Python golden suite
  (`scripts/tests/test_golden.py`).
- **Unit tests:** parse validation (overlap, enclosing non-member, parent cycle,
  unknown ids); group extent from members ∪ descendants; dynamic lane width/height
  math; ring assignment order; routing (perpendicular cross ok, run-along blocked,
  padding) — in both Rust and Python suites.
- **Parity:** implement in Rust first, then translate to `layout.py` so the
  byte-identical Python↔Rust parity oracle still holds. The parity cross-check
  must pass on a groups fixture.
- **result.json:** include resolved group boxes (`id`, `title`, grid extent, x/y/w/h)
  in the report so downstream consumers (and the auto loop) can see them.

## Section 6 — SKILL.md

`claude-plugin/c43/skills/ascii/SKILL.md` (the ascii skill) drives how the
layout is invoked. Update it to:

- document the optional `groups` array in the `layout.json` schema example;
- explain that groups draw nested double-line frames in the edge lanes, hug nodes
  while edges keep lane centres, and carry titles;
- note the validation rules (contiguous/nesting only — no partial overlap, no
  enclosing strangers);
- mention the new group fields in `result.json`.

## Completion gate (per CLAUDE.md)

- `npm run build` from repo root succeeds (includes `cargo build --release`).
- `npm run e2e` from `packages/example/local-docker` succeeds.
- Rust + Python test suites pass, including the new groups golden and the parity
  cross-check.
- Append the delivered specification-relevant parts to the moth issue.
