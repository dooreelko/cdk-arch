ID: bvthw | Severity: med | Status: doing
Title: Groups
---
the diagrams should be able to visually group elements.

the idea is to 
- add a vertical edge lane on the left (but avoid using it for edges since we prohibit inbound ports on the left)
- render rectangles surrounding a group of nodes on the edge lanes
- while edges gravitate to the middle, groups stay close to the nodes
- edges are allowed to cross group boxes' sides
- edges are prohibited from having segments running on group boxes' sides
- same padding rule applies to groups - at least one space in each direction between all edges and groups
- a group has a title, rendered inside at the bottom, one space from the left side, one space above bottom one

use the following architecure for tests. Each node with children is a group (e.g. there will be nested groups).

```
 c43 container . --ascii
  ⎿  System: rebob
     ├─ Backend: dispatcher [uses: runtime, sub-bob-manager]
     │   ├─ dispatcherruntime: runtime
     │   └─ mcpcontainer: dispatcher-mcp
     ├─ Backend: chat [uses: bus, event-storage]
     │   ├─ messagebus: bus
     │   └─ mcpcontainer: chat-mcp
     ├─ Backend: queue
     │   └─ taskqueue: task-queue
     ├─ Backend: memory [uses: graph, llm, queue]
     │   ├─ graph: graph
     │   └─ mcpcontainer: memory-mcp
     ├─ Backend: event-storage
     ├─ Backend: sub-bob
     ├─ Backend: consolidator [uses: llm, memory, queue]
     │   └─ consolidatorapi: consolidator-api
     ├─ Backend: llm
     ├─ Backend: bob-registry
     │   ├─ ApiContainer: registry [uses: runtime]
     │   └─ mcpcontainer: registry-mcp
     ├─ Backend: sub-bob-manager [uses: chat, memory]
     │   ├─ bobqueue: bob-queue
     │   ├─ ApiContainer: manager [uses: bob-queue, runtime]
     │   └─ mcpcontainer: manager-mcp
     ├─ Client: @bob/sub-bob-bootstrap [uses: memory]
     └─ Frontend: @bob/frontend [uses: bob-registry, chat, memory, sub-bob-manager]
```
 


## Delivered

Implemented in Rust (`packages/c43`, source of truth) and mirrored byte-for-byte in the Python reference (`claude-plugin/c43/skills/ascii/scripts/layout.py`); a nested-rebob golden renders identically from both (parity oracle). Delivered on the `feat/groups-bvthw` branch.

### Schema (`layout.json`)
Optional top-level `groups` array. Each entry:
- `id` (required, unique string), `title` (required, rendered inside the frame).
- `members`: leaf node ids directly in the group (child groups link via their own `parent`, not listed here).
- `parent`: id of the enclosing group, or null/absent for a top-level group.

A group's grid extent is the bounding rectangle of its members ∪ all descendant groups' extents.

### Validation (`code: "validation"`)
Rejects: unknown member id; unknown parent id; duplicate group id; parent cycle; a frame enclosing a node that is not a member or descendant-member (`encloses non-member`); two groups that partially overlap (`overlap`). Full nesting/containment and disjoint groups are allowed.

### Geometry & rendering
- A left bounding lane (region -1) is added when groups are present, hosting the outermost left frame borders. It is reserved for frames only: routing blocks it outright (inbound-left ports are prohibited, so it never needs to carry an edge), satisfying the "vertical edge lane on the left" requirement.
- Lanes widen dynamically to fit the group border "rings" passing through them; rings pack toward the nodes (deepest nesting hugs nodes) while the edge centre track stays centred, so edges gravitate to lane middles and groups stay close to nodes.
- Frames are double-line Unicode (`╔ ═ ╗ ║ ╚ ╝`), painted after scaffolding and before nodes (edges overprint at legitimate crossings). Nested frames nest visually. The title sits inside the frame with a one-cell gap from each side: one blank cell in from the left border and one blank row above the bottom border.

### Routing
Edges may cross a group border perpendicularly (no penalty) but may never run along one or turn on a border cell; ≥1-cell padding between edges and frames is preserved. The left bounding lane is never routed through.

### result.json
Adds a `groups` array: `id`, `title`, `parent`, `grid` (`col0`/`col1`/`row0`/`row1`), and resolved `x`/`y`/`w`/`h`.

### Backward compatibility
A layout with no `groups` produces byte-identical `result.txt`/`result.json` to before (verified by the untouched groupless goldens, including `rebob_render_matches_golden` and `exact_geometry_offsets`).

### Skill + UAT
- `claude-plugin/c43/skills/ascii/SKILL.md` documents the `groups` schema, rendering behaviour, validation rules, and the `result.json` field. It also guides node placement toward a balanced (roughly square, ~⌈√N⌉ columns) grid so edges have lanes to spread into rather than crowding a few wide rows.
- `claude-plugin/c43/uat/` adds a manual UAT harness (`test.sh`) with two cases (`system` flat, `container` nested groups). Each runs `claude --print --plugin-dir` against the in-dev `c43` and surfaces the generated `result.json`/`result.txt` next to a semantic `expected.txt` for human judgement.

### Known limitation
Same-depth sibling groups whose borders terminate at the same grid column/row but lie in different rows/columns are each assigned a distinct ring rank, so their frames may stair-step and the shared lane widens accordingly. All seven group rules still hold; this is a cosmetic ring-packing artifact, not a correctness issue.

### Verification
Rust suite (16 test binaries) and Python suite (124 tests) both green; Python↔Rust byte-parity reconfirmed on the regenerated rebob groups golden. `npm run build` and `npm run e2e` pass.
