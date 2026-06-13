import contextlib, json, os, sys, string, heapq, itertools
from dataclasses import dataclass, field

EDGE_ALPHABET = string.digits + string.ascii_lowercase + string.ascii_uppercase  # 62 chars
SIDES = ("left", "right", "top", "bottom")

# Geometry constants (tunable during the visual-approval loop)
GUTTER_W   = 8     # label gutter width; the spine '│' sits at column index GUTTER_W
LABEL_PAD  = 4     # total horizontal padding added to widest label -> box width
BOX_H      = 11    # fixed box height in characters
BOX_MARGIN_X = 4   # chars between a node column's left edge and its box
BOX_MARGIN_Y = 2   # chars between a node row's top edge and its box
LANE_MIN_W = 16    # minimum width (chars) of a vertical edge lane
LANE_MIN_H = 7     # minimum height (chars) of a horizontal edge lane
TITLE_H    = 6     # height of the title region (row 000)

# Routing cost is the strictly-lexicographic tuple
# (crossings, turns, centre_offset, length): each term is an absolute
# tie-breaker for the next. Crossings dominate (they raise errors); among
# equal-crossing routes the straightest (fewest turns) wins; only among
# equally-straight routes does an edge gravitate to its lane centre; length
# breaks the last tie. So a long off-centre straight run is never traded for
# extra turns -- it stays straight.

class ValidationError(Exception):
    """Stage-1 hard error: no result.txt is produced, only result.json."""

@dataclass
class Node:
    id: str
    label: str
    grid_col: int
    grid_row: int
    x: int = 0; y: int = 0; w: int = 0; h: int = 0   # filled by geometry

@dataclass
class Port:
    side: str
    x: int
    y: int

@dataclass
class Edge:
    id: str
    from_id: str
    to_id: str
    char: str
    from_port: Port = None
    to_port: Port = None
    route: list = None          # list of [x, y] polyline vertices

@dataclass
class LayoutError:
    code: str                   # "validation" | "crossing" | "unroutable"
    edge_ids: list
    at: list                    # [x, y] or None
    message: str
    suggestion: str

@dataclass
class Model:
    title: str
    description: str
    nodes: list                 # list[Node]
    edges: list                 # list[Edge]
    hint_ports: dict = field(default_factory=dict)   # edge_id -> {"from_side","to_side"}
    routing_order: list = field(default_factory=list)
    canvas_w: int = 0
    canvas_h: int = 0
    box_w: int = 0
    box_h: int = 0
    col_x: dict = field(default_factory=dict)
    row_y: dict = field(default_factory=dict)
    # band intervals: (start, end, kind, center) where kind is
    # "node"/"lane" for columns and "title"/"node"/"lane" for rows.
    col_bands: list = field(default_factory=list)
    row_bands: list = field(default_factory=list)
    # Per-coordinate caches built once by geometry() so the hot routing loop
    # does O(1) lookups instead of scanning the band lists: col_kind[x] /
    # row_kind[y] give the band kind, col_center[x] / row_center[y] the lane
    # centre track (None outside a lane).
    col_kind: list = field(default_factory=list)
    row_kind: list = field(default_factory=list)
    col_center: list = field(default_factory=list)
    row_center: list = field(default_factory=list)
    errors: list = field(default_factory=list)        # list[LayoutError]


def parse_and_validate(raw):
    if "title" not in raw:
        raise ValidationError("missing required key: title")
    if "description" not in raw:
        raise ValidationError("missing required key: description")

    if "nodes" not in raw:
        raise ValidationError("missing required key: nodes")
    if "edges" not in raw:
        raise ValidationError("missing required key: edges")

    nodes_raw = raw["nodes"]
    edges_raw = raw["edges"]

    if not nodes_raw:
        raise ValidationError("nodes must not be empty")

    seen_node_ids, cells = set(), {}
    nodes = []
    for i, nr in enumerate(nodes_raw):
        if "id" not in nr:
            raise ValidationError(f"node at index {i} missing required field: id")
        nid = nr["id"]
        if "label" not in nr:
            raise ValidationError(f"node at index {i} (id={nid!r}) missing required field: label")
        if "grid_col" not in nr:
            raise ValidationError(f"node at index {i} (id={nid!r}) missing required field: grid_col")
        if "grid_row" not in nr:
            raise ValidationError(f"node at index {i} (id={nid!r}) missing required field: grid_row")

        grid_col = nr["grid_col"]
        grid_row = nr["grid_row"]
        if not isinstance(grid_col, int) or isinstance(grid_col, bool) or grid_col < 0:
            raise ValidationError(f"node at index {i} (id={nid!r}): grid_col must be an int >= 0, got {grid_col!r}")
        if not isinstance(grid_row, int) or isinstance(grid_row, bool) or grid_row < 0:
            raise ValidationError(f"node at index {i} (id={nid!r}): grid_row must be an int >= 0, got {grid_row!r}")

        if nid in seen_node_ids:
            raise ValidationError(f"duplicate node id: {nid}")
        seen_node_ids.add(nid)
        cell = (grid_col, grid_row)
        if cell in cells:
            raise ValidationError(
                f"two nodes in grid cell {cell}: {cells[cell]} and {nid}")
        cells[cell] = nid
        nodes.append(Node(id=nid, label=nr["label"],
                          grid_col=grid_col, grid_row=grid_row))

    if len(edges_raw) > len(EDGE_ALPHABET):
        raise ValidationError(
            f"{len(edges_raw)} edges exceeds the {len(EDGE_ALPHABET)}-char "
            f"alphabet; unicode edge alphabet is the future expansion path")

    node_ids = {n.id for n in nodes}
    seen_edge_ids = set()
    edges = []
    for i, er in enumerate(edges_raw):
        if "id" not in er:
            raise ValidationError(f"edge at index {i} missing required field: id")
        eid = er["id"]
        if "from" not in er:
            raise ValidationError(f"edge at index {i} (id={eid!r}) missing required field: from")
        if "to" not in er:
            raise ValidationError(f"edge at index {i} (id={eid!r}) missing required field: to")

        if eid in seen_edge_ids:
            raise ValidationError(f"duplicate edge id: {eid}")
        seen_edge_ids.add(eid)
        for end in ("from", "to"):
            if er[end] not in node_ids:
                raise ValidationError(
                    f"edge {eid} references unknown node id: {er[end]}")
        edges.append(Edge(id=eid, from_id=er["from"], to_id=er["to"],
                          char=EDGE_ALPHABET[i]))

    hints = raw.get("hints") or {}

    allowed_hint_keys = {"ports", "routing_order"}
    for key in hints:
        if key not in allowed_hint_keys:
            raise ValidationError(
                f"unknown key in hints: {key!r}, allowed keys are: {', '.join(sorted(allowed_hint_keys))}")

    hint_ports = {}
    seen_hint_edge_ids = set()
    for i, hp in enumerate(hints.get("ports", [])):
        if "edge_id" not in hp:
            raise ValidationError(f"hint port at index {i} missing required field: edge_id")

        hint_edge_id = hp["edge_id"]

        if hint_edge_id in seen_hint_edge_ids:
            raise ValidationError(f"duplicate hint port for edge_id: {hint_edge_id}")
        seen_hint_edge_ids.add(hint_edge_id)

        if hint_edge_id not in seen_edge_ids:
            raise ValidationError(f"hint references unknown edge_id: {hint_edge_id}")

        for key in ("from_side", "to_side"):
            if key in hp and hp[key] not in SIDES:
                raise ValidationError(
                    f"hint for edge {hint_edge_id}: invalid side {hp[key]!r}, "
                    f"must be one of {SIDES}")
        # The right side is reserved for outbound ports (data flows left to
        # right); an inbound port may never use it.
        if hp.get("to_side") == "right":
            raise ValidationError(
                f"hint for edge {hint_edge_id}: to_side 'right' is prohibited; "
                f"inbound ports use left, top, or bottom")
        hint_ports[hint_edge_id] = hp

    routing_order = hints.get("routing_order", [])
    seen_routing_ids = set()
    for eid in routing_order:
        if eid in seen_routing_ids:
            raise ValidationError(f"duplicate edge_id in routing_order: {eid}")
        seen_routing_ids.add(eid)

        if eid not in seen_edge_ids:
            raise ValidationError(f"routing_order references unknown edge_id: {eid}")

    return Model(title=raw["title"], description=raw["description"],
                 nodes=nodes, edges=edges,
                 hint_ports=hint_ports, routing_order=routing_order)


def geometry(m):
    m.col_x.clear()
    m.row_y.clear()
    m.col_bands.clear()
    m.row_bands.clear()

    max_label = max(len(n.label) for n in m.nodes)
    m.box_w = max_label + LABEL_PAD
    m.box_h = BOX_H

    max_col = max(n.grid_col for n in m.nodes)
    max_row = max(n.grid_row for n in m.nodes)

    node_col_w = m.box_w + 2 * BOX_MARGIN_X
    node_row_h = m.box_h + 2 * BOX_MARGIN_Y

    # x offset of the left edge of each canvas grid column region;
    # region 0 begins just right of the spine. Each band records its
    # [start, end), kind, and centre track (lanes only) for the router.
    x = GUTTER_W + 1
    for c in range(max_col + 1):
        m.col_x[2 * c] = x          # node column c
        m.col_bands.append((x, x + node_col_w, "node", None))
        x += node_col_w
        m.col_x[2 * c + 1] = x      # vertical lane c
        m.col_bands.append((x, x + LANE_MIN_W, "lane", x + LANE_MIN_W // 2))
        x += LANE_MIN_W
    m.canvas_w = x

    # y offset of the top edge of each canvas grid row region. The title
    # region (000) carries the title text up top and a routing lane below
    # it, so edges approaching the first node row from above have a lane to
    # gravitate into instead of hugging the box tops.
    y = 0
    m.row_y[0] = y                  # title
    m.row_bands.append((0, TITLE_H, "title", None))
    m.row_bands.append((TITLE_H, TITLE_H + LANE_MIN_H, "lane",
                        TITLE_H + LANE_MIN_H // 2))   # top lane, above node row 0
    y = TITLE_H + LANE_MIN_H
    for r in range(max_row + 1):
        m.row_y[2 * r + 1] = y      # node row r
        m.row_bands.append((y, y + node_row_h, "node", None))
        y += node_row_h
        m.row_y[2 * r + 2] = y      # horizontal lane r
        m.row_bands.append((y, y + LANE_MIN_H, "lane", y + LANE_MIN_H // 2))
        y += LANE_MIN_H
    m.canvas_h = y

    for n in m.nodes:
        n.w = m.box_w
        n.h = m.box_h
        n.x = m.col_x[2 * n.grid_col] + BOX_MARGIN_X
        n.y = m.row_y[2 * n.grid_row + 1] + BOX_MARGIN_Y

    _build_band_caches(m)


def _build_band_caches(m):
    """Flatten the band interval lists into per-coordinate arrays so the
    routing loop avoids a linear band scan on every cell visit."""
    def flatten(bands, n):
        kind = [None] * n
        center = [None] * n
        for s, e, k, c in bands:
            for v in range(max(0, s), min(n, e)):
                kind[v] = k
                center[v] = c
        return kind, center
    m.col_kind, m.col_center = flatten(m.col_bands, m.canvas_w)
    m.row_kind, m.row_center = flatten(m.row_bands, m.canvas_h)


def _default_from_side(src, dst):
    """Outbound side. Right is fine for outbound (data flows left-to-right);
    the inbound side is chosen separately by _inbound_side."""
    if dst.grid_col > src.grid_col:
        return "right"
    if dst.grid_col < src.grid_col:
        return "left"
    if dst.grid_row > src.grid_row:
        return "bottom"
    return "top"   # same column upward, and self-loops

def _sign(v):
    return (v > 0) - (v < 0)

# Inward approach unit vector for each inbound side, and the pre-port cell
# offset from the node box for estimating elbows.
_INBOUND = {
    "left":   (+1, 0),
    "top":    (0, +1),
    "bottom": (0, -1),
}

def _inbound_pre_cell(dst, side):
    """The lane cell an edge sits in just before stepping into the port."""
    if side == "left":
        return (dst.x - 1, dst.y + dst.h // 2)
    if side == "top":
        return (dst.x + dst.w // 2, dst.y - 1)
    # bottom
    return (dst.x + dst.w // 2, dst.y + dst.h)

def _elbows(ax, ay, pre, d):
    """Minimum bends of a Manhattan path from source point (ax, ay) to the
    pre-port cell `pre`, given the forced final approach direction `d` into
    the port. A source on the wrong side of the approach axis costs an extra
    turn (it must loop around), which is exactly how we steer inbound ports
    toward the source."""
    bx, by = pre
    dx, dy = bx - ax, by - ay
    if dx == 0 and dy == 0:
        return 0
    if d[0] != 0:                       # horizontal approach (left port)
        if dy == 0:
            return 0 if (dx == 0 or _sign(dx) == d[0]) else 2
        if dx == 0:
            return 1
        return 1 if _sign(dx) == d[0] else 2
    else:                               # vertical approach (top/bottom port)
        if dx == 0:
            return 0 if (dy == 0 or _sign(dy) == d[1]) else 2
        if dy == 0:
            return 1
        return 1 if _sign(dy) == d[1] else 2

def _side_reaches_lane(m, dst, side):
    """True if stepping out of `dst`'s `side` eventually exits the node region
    into an in-bounds lane (not the gutter, not off-canvas). A column-0 node's
    left side faces the gutter, so left is unroutable there; a top-row node's
    top reaches the title-region lane, etc."""
    dx, dy = {"left": (-1, 0), "top": (0, -1), "bottom": (0, 1)}[side]
    if side == "left":
        x, y = dst.x - 1, dst.y + dst.h // 2
    elif side == "top":
        x, y = dst.x + dst.w // 2, dst.y - 1
    else:
        x, y = dst.x + dst.w // 2, dst.y + dst.h
    while GUTTER_W < x < m.canvas_w and 0 <= y < m.canvas_h:
        kind_c = _band(m.col_bands, x)[0]
        kind_r = _band(m.row_bands, y)[0]
        if kind_c == "lane" or (kind_r == "lane" and kind_c is not None):
            return True
        if not (_band(m.col_bands, x)[0] == "node"
                and _band(m.row_bands, y)[0] == "node"):
            return False    # left the node region but not into a routing lane
        x += dx
        y += dy
    return False

def _inbound_side(m, src, dst):
    """Choose the target's inbound side. The right side is prohibited; among
    left/top/bottom that actually reach a routing lane, rank by fewest elbows,
    ties broken left < bottom < top (top last so it stays least-used)."""
    sx, sy = src.x + src.w // 2, src.y + src.h // 2
    order = {"left": 0, "bottom": 1, "top": 2}
    best = None
    for side in ("left", "top", "bottom"):
        if not _side_reaches_lane(m, dst, side):
            continue
        e = _elbows(sx, sy, _inbound_pre_cell(dst, side), _INBOUND[side])
        kk = (e, order[side])
        if best is None or kk < best[0]:
            best = (kk, side)
    # Fallback: if nothing reaches a lane (degenerate geometry) prefer bottom,
    # which always borders a horizontal lane.
    return best[1] if best else "bottom"

def _default_sides(m, src, dst):
    return _default_from_side(src, dst), _inbound_side(m, src, dst)

def _other_endpoint(by_id, node, edge):
    other_id = edge.to_id if edge.from_id == node.id else edge.from_id
    return by_id[other_id]

def assign_ports(m):
    by_id = {n.id: n for n in m.nodes}

    # 1. decide a side for each (edge, endpoint)
    plan = {}   # (edge_id, "from"/"to") -> side
    for e in m.edges:
        src, dst = by_id[e.from_id], by_id[e.to_id]
        fs, ts = _default_sides(m, src, dst)
        h = m.hint_ports.get(e.id)
        if h:
            fs = h.get("from_side", fs)
            ts = h.get("to_side", ts)
        plan[(e.id, "from")] = fs
        plan[(e.id, "to")] = ts

    # 2. group ports by (node, side), order them, stack on distinct cells
    groups = {}   # (node_id, side) -> list[(edge, end)]
    for e in m.edges:
        groups.setdefault((e.from_id, plan[(e.id, "from")]), []).append((e, "from"))
        groups.setdefault((e.to_id, plan[(e.id, "to")]), []).append((e, "to"))

    for (node_id, side), members in groups.items():
        node = by_id[node_id]
        # order by the other endpoint's center to reduce immediate crossings
        def keyfn(item):
            e, _end = item
            other = _other_endpoint(by_id, node, e)
            return (other.y + other.h / 2) if side in ("left", "right") else (other.x + other.w / 2)
        members.sort(key=keyfn)

        # Check capacity
        if side in ("left", "right"):
            cap = node.h - 2
        else:
            cap = node.w - 2

        n_ports = len(members)
        if n_ports > cap:
            # Report overflow error
            overflow_ids = [e.id for e, _end in members[cap:]]
            m.errors.append(LayoutError(
                code="validation",
                edge_ids=overflow_ids,
                at=None,
                message=f"node {node_id!r}: {n_ports} ports on {side} side, capacity {cap}",
                suggestion="move some edges to another side via hints.ports, or move neighbor nodes to other grid columns/rows"
            ))

        # Assign ports only up to capacity
        assigned_count = min(n_ports, cap)
        for i, (e, end) in enumerate(members[:assigned_count]):
            if side in ("left", "right"):
                x = node.x if side == "left" else node.x + node.w - 1
                # spread across interior rows, leaving the corners alone
                y = node.y + 1 + (i + 1) * (node.h - 2) // (assigned_count + 1)
                port = Port(side=side, x=x, y=y)
            else:
                y = node.y if side == "top" else node.y + node.h - 1
                x = node.x + 1 + (i + 1) * (node.w - 2) // (assigned_count + 1)
                port = Port(side=side, x=x, y=y)
            if end == "from":
                e.from_port = port
            else:
                e.to_port = port


def _band(bands, v):
    """(kind, center) of the band containing coordinate v, or (None, None).
    Linear scan -- used outside the hot loop (the router reads the cached
    per-coordinate arrays built by _build_band_caches instead)."""
    for s, e, kind, center in bands:
        if s <= v < e:
            return kind, center
    return None, None

def _is_node_region(m, x, y):
    """A node cell -- the box and its margins. Edges may only enter here on
    their port stubs (first/last segment); the body of every route stays in
    the lane bands."""
    return (0 <= x < m.canvas_w and 0 <= y < m.canvas_h
            and m.col_kind[x] == "node" and m.row_kind[y] == "node")

def _build_blocked(m):
    """Cells never routable for the body of a route: every node-region cell
    (box border + interior + the margins around it) and the whole title band
    (it carries the diagram title/description, not routing). Per-edge port
    stubs are carved back out in route_all so an edge can reach its ports.
    The gutter/spine is excluded via the bounds check in _astar."""
    blocked = set()
    for s_x, e_x, ckind, _ in m.col_bands:
        if ckind != "node":
            continue
        for s_y, e_y, rkind, _ in m.row_bands:
            if rkind != "node":
                continue
            for x in range(s_x, e_x):
                for y in range(s_y, e_y):
                    blocked.add((x, y))
    # title band: full canvas width, never a routing surface
    for s_y, e_y, rkind, _ in m.row_bands:
        if rkind == "title":
            for x in range(GUTTER_W + 1, m.canvas_w):
                for y in range(s_y, e_y):
                    blocked.add((x, y))
    return blocked

def _port_exit(port):
    if port.side == "left":   return (port.x - 1, port.y)
    if port.side == "right":  return (port.x + 1, port.y)
    if port.side == "top":    return (port.x, port.y - 1)
    return (port.x, port.y + 1)

def _port_stub(m, port):
    """The straight corridor from a port out to the first lane band: the
    margin cells the route must traverse to leave (or enter) the node cell.
    These node-region cells are carved out of `blocked` for this edge only."""
    dx, dy = {"left": (-1, 0), "right": (1, 0),
              "top": (0, -1), "bottom": (0, 1)}[port.side]
    x, y = _port_exit(port)
    cells = []
    while (GUTTER_W < x < m.canvas_w and 0 <= y < m.canvas_h
           and _is_node_region(m, x, y)):
        cells.append((x, y))
        x += dx
        y += dy
    return cells

_INF_COST = (1 << 62,) * 5

_KING = ((1, 0), (-1, 0), (0, 1), (0, -1),
         (1, 1), (1, -1), (-1, 1), (-1, -1))

def _astar(start, goal, blocked, occupied, forbidden, allow_cross, m):
    """Uniform-cost (Dijkstra) search on the 4-connected grid, no heuristic.
    Strictly-lexicographic cost (crossings, adjacency, turns, centre_offset,
    length): each term is an absolute tie-breaker for the next. Crossings
    dominate (they raise errors); then adjacency (cells abutting another
    edge's track with no gap); then straightness; then lane-centring; length
    breaks the final tie. Each term is additive and non-negative per step, so
    Dijkstra stays correct under lexicographic comparison.

    occupied: dict cell -> edge_id (for crossing detection). `forbidden` is the
    king-neighbour halo of every already-claimed edge (maintained in
    route_all), used for the 2x2 no-shared-block rule. In pass 1 a halo cell is
    refused outright. In pass 2 (a crossing edge) the rule is demoted to the
    `adjacency` cost term instead of a hard block: the edge still prefers
    gapped tracks and only hugs another edge where geometry forces it -- right
    at the unavoidable crossing -- rather than running flush the whole way.
    Returns (cells, crossing_cells) or (None, None)."""
    w, h = m.canvas_w, m.canvas_h
    col_kind, row_kind = m.col_kind, m.row_kind   # cached band lookups (hot path)
    col_center, row_center = m.col_center, m.row_center
    def in_bounds(c):
        return GUTTER_W < c[0] < w and 0 <= c[1] < h

    # A port exit landing in a blocked or out-of-bounds cell makes the
    # edge unroutable -- report, never crash or route through a box.
    if not in_bounds(start) or not in_bounds(goal) or start in blocked or goal in blocked:
        return None, None
    # In pass 1 a start cell already claimed by another edge would silently
    # overlap that route; refuse so the edge retries in pass 2, where the
    # shared start is reported as a crossing.
    if not allow_cross and start in occupied:
        return None, None

    counter = itertools.count()              # heap tiebreak: never compare cells/dirs
    pq = [((0, 0, 0, 0, 0), next(counter), start, None)]   # cost, tiebreak, cell, dir
    best = {(start, None): (0, 0, 0, 0, 0)}
    came = {}
    while pq:
        cost, _, pos, dirn = heapq.heappop(pq)
        if cost > best.get((pos, dirn), _INF_COST):
            continue
        if pos == goal:
            cells = [pos]
            st = (pos, dirn)
            while st in came:
                st = came[st]
                cells.append(st[0])
            cells.reverse()
            crossings = [c for c in cells if c in occupied]
            return cells, crossings
        for d in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            np_ = (pos[0] + d[0], pos[1] + d[1])
            if not in_bounds(np_):
                continue
            if np_ in blocked and np_ != goal:
                continue
            crossed = np_ in occupied
            if crossed and not allow_cross:
                continue
            nx, ny = np_
            # 2x2-cluster spacing via the king-neighbour halo `forbidden`. A
            # lane cell in the halo abuts another edge with no gap. In a node
            # region edges legitimately bunch (port stubs), and the goal must
            # stay reachable, so those are exempt.
            adj = (np_ != goal and np_ in forbidden
                   and not (col_kind[nx] == "node" and row_kind[ny] == "node"))
            # Pass 1: a hard block (no shared 2x2 ever). Pass 2 (crossing edge):
            # demoted to a cost so the edge hugs only where it must.
            if adj and not allow_cross:
                continue
            crs, adjc, turns, off, length = cost
            turn = 1 if dirn is not None and d != dirn else 0
            if d[1] == 0:       # horizontal segment -> centre within its row lane
                center = row_center[ny]
            else:               # vertical segment -> centre within its col lane
                center = col_center[nx]
            offset = 0
            if center is not None:
                offset = abs((ny if d[1] == 0 else nx) - center)
            ncost = (crs + (1 if crossed else 0),
                     adjc + (1 if adj else 0),
                     turns + turn,
                     off + offset,
                     length + 1)
            nstate = (np_, d)
            if ncost < best.get(nstate, _INF_COST):
                best[nstate] = ncost
                came[nstate] = (pos, dirn)
                heapq.heappush(pq, (ncost, next(counter), np_, d))
    return None, None

def _to_polyline(cells):
    poly = [list(cells[0])]
    for i in range(1, len(cells) - 1):
        a, b, c = cells[i - 1], cells[i], cells[i + 1]
        if not ((a[0] == b[0] == c[0]) or (a[1] == b[1] == c[1])):
            poly.append(list(b))
    poly.append(list(cells[-1]))
    return poly

def _manhattan(e):
    return abs(e.from_port.x - e.to_port.x) + abs(e.from_port.y - e.to_port.y)

def _crossing_runs(cells, occupied):
    """Group consecutive path cells claimed by the same owner.
    Returns [(owner_edge_id, first_cell_of_run), ...] -- one entry per
    contiguous run, so a long shared track yields one error, not one per cell."""
    runs = []
    prev_owner = None
    for c in cells:
        owner = occupied.get(c)
        if owner is not None and owner != prev_owner:
            runs.append((owner, c))
        prev_owner = owner
    return runs

def route_all(m):
    base_blocked = _build_blocked(m)
    occupied = {}            # cell -> edge_id of claiming edge
    # King-neighbours of every occupied cell. A pass-1 candidate in a lane is
    # refused if it lands here -- precomputing the halo turns the 8-neighbour
    # scan in _astar into a single set membership test (the routing hot path).
    forbidden = set()

    # Edges whose port assignment overflowed (Task 4) carry a None port and
    # an already-recorded error; skip them silently here.
    routable = [e for e in m.edges if e.from_port is not None and e.to_port is not None]
    order_index = {eid: i for i, eid in enumerate(m.routing_order)}
    ordered = sorted(routable,
                     key=lambda e: (order_index.get(e.id, len(order_index)), _manhattan(e)))

    def attempt(e, allow_cross):
        # Carve this edge's own port stubs out of the node-region block so it
        # can reach its ports, while every other node region stays walled off
        # -- the route body is thereby confined to the lane bands.
        stubs = set(_port_stub(m, e.from_port)) | set(_port_stub(m, e.to_port))
        blocked = base_blocked - stubs
        return _astar(_port_exit(e.from_port), _port_exit(e.to_port),
                      blocked, occupied, forbidden, allow_cross, m)

    def claim(e, cells):
        for c in cells:
            occupied.setdefault(c, e.id)     # never steal another edge's claim
            cx, cy = c
            for kx, ky in _KING:
                forbidden.add((cx + kx, cy + ky))

    failed = []
    for e in ordered:                        # pass 1: no crossings allowed
        cells, _ = attempt(e, allow_cross=False)
        if cells is None:
            failed.append(e)
            continue
        e.route = _to_polyline(cells)
        claim(e, cells)

    for e in failed:                         # pass 2: desperation, crossings ok
        cells, _ = attempt(e, allow_cross=True)
        if cells is None:
            # Defensive arm: hard to reach end-to-end with current geometry
            # (lanes always leave a corridor), but a port exit landing in a
            # blocked/out-of-bounds cell or a fully walled goal ends up here.
            m.errors.append(LayoutError(
                code="unroutable", edge_ids=[e.id], at=None,
                message=f"edge {e.id} could not be routed even with crossings",
                suggestion="move its endpoints to adjacent grid cells "
                           "(grid_col/grid_row), pick other sides via hints.ports, "
                           "or free a lane via hints.routing_order"))
            continue
        for owner, c in _crossing_runs(cells, occupied):
            m.errors.append(LayoutError(
                code="crossing", edge_ids=[e.id, owner], at=list(c),
                message=f"edges {e.id} and {owner} cross at {list(c)}",
                suggestion="reorder with hints.routing_order, adjust port sides "
                           "via hints.ports, or move a node to open a parallel track"))
        e.route = _to_polyline(cells)
        claim(e, cells)


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.grid = [[" "] * w for _ in range(h)]

    def paint(self, x, y, ch):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.grid[y][x] = ch

    def char_at(self, x, y):
        if not (0 <= x < self.w and 0 <= y < self.h):
            raise IndexError(
                f"char_at({x}, {y}) out of bounds for {self.w}x{self.h} canvas")
        return self.grid[y][x]

    def __str__(self):
        return "\n".join("".join(row).rstrip() for row in self.grid) + "\n"

    def save(self, path):
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(str(self))
        os.replace(tmp, path)


# Arrowhead keyed by the TARGET port's side: an edge entering a left-side
# port moves rightward, so it ends in '►'; and so on for the other sides.
ARROWS = {"left": "►", "right": "◄", "top": "▼", "bottom": "▲"}


def _paint_text(cv, x, y, s):
    for i, ch in enumerate(s):
        cv.paint(x + i, y, ch)


def _paint_scaffolding(m, cv):
    assert TITLE_H >= 5, "TITLE_H must fit column headers (rows 1-2) + title block (rows 3-4)"
    # gutter spine
    for y in range(cv.h):
        cv.paint(GUTTER_W, y, "│")

    # column headers (rows 1-2): region index + kind
    for ridx, rx in sorted(m.col_x.items(), key=lambda kv: kv[1]):
        kind = "nodes" if ridx % 2 == 0 else "edges"
        _paint_text(cv, rx + 1, 1, f"{ridx:03d}")
        _paint_text(cv, rx + 1, 2, kind)

    # row labels in the gutter + horizontal separators at region tops
    for ridx, ry in sorted(m.row_y.items(), key=lambda kv: kv[1]):
        if ry > 0:
            for x in range(1, cv.w):
                cv.paint(x, ry, "─")
            cv.paint(GUTTER_W, ry, "┼")
        kind = "title" if ridx == 0 else ("nodes" if ridx % 2 == 1 else "edges")
        _paint_text(cv, 2, ry + 1, f"{ridx:03d}")
        _paint_text(cv, 2, ry + 2, kind)

    # title block sits at rows 3-4 of the title region, clear of the
    # column headers painted at rows 1-2
    _paint_text(cv, GUTTER_W + 2, m.row_y[0] + 3, m.title)
    _paint_text(cv, GUTTER_W + 2, m.row_y[0] + 4, m.description)


def _draw_box(cv, n):
    x0, y0 = n.x, n.y
    x1, y1 = n.x + n.w - 1, n.y + n.h - 1
    for x in range(x0, x1 + 1):
        cv.paint(x, y0, "-")
        cv.paint(x, y1, "-")
    for y in range(y0, y1 + 1):
        cv.paint(x0, y, "|")
        cv.paint(x1, y, "|")
    for cx, cy in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
        cv.paint(cx, cy, "+")
    lx = x0 + (n.w - len(n.label)) // 2
    _paint_text(cv, lx, y0 + n.h // 2, n.label)


def _paint_edge(cv, e):
    for (x0, y0), (x1, y1) in zip(e.route, e.route[1:]):
        if y0 == y1:
            for x in range(min(x0, x1), max(x0, x1) + 1):
                cv.paint(x, y0, e.char)
        else:
            for y in range(min(y0, y1), max(y0, y1) + 1):
                cv.paint(x0, y, e.char)
    cv.paint(e.from_port.x, e.from_port.y, "*")
    cv.paint(e.to_port.x, e.to_port.y, ARROWS[e.to_port.side])


def render(m, cv, path="result.txt"):
    """Paint scaffolding, boxes, then edges -- saving after every mutation
    so a crash mid-run leaves the last good state on disk."""
    _paint_scaffolding(m, cv)
    cv.save(path)
    for n in m.nodes:
        _draw_box(cv, n)
        cv.save(path)
    for e in m.edges:
        if e.route is None:
            continue
        _paint_edge(cv, e)
        cv.save(path)


# A routed edge is a "wrap" once its drawn length exceeds the straight
# port-to-port distance by this many cells -- enough to mean it looped the
# canvas, not merely took a long L-bend.
WRAP_EXCESS = 100
# Report a congested edge-pair only once they run king-adjacent for at least
# this many cells (a sustained parallel run, not an incidental touch).
CONGEST_MIN = 6
_KING8 = ((1, 0), (-1, 0), (0, 1), (0, -1),
          (1, 1), (1, -1), (-1, 1), (-1, -1))

def _route_cell_set(route):
    """All grid cells a polyline passes through (inclusive of vertices)."""
    out = set()
    for (x0, y0), (x1, y1) in zip(route, route[1:]):
        if x0 == x1:
            for y in range(min(y0, y1), max(y0, y1) + 1):
                out.add((x0, y))
        else:
            for x in range(min(x0, x1), max(x0, x1) + 1):
                out.add((x, y0))
    return out

def _quality_and_diagnostics(m):
    """Soft quality signals the visual-approval loop optimises against.
    Returns (quality_dict, diagnostics_list). `errors` (validation / crossing
    / unroutable) stays the hard-failure channel that drives `status`; these
    are advisory and never flip status on their own.

    quality (all ints, lower is better):
      crossings   distinct crossing pairs already in m.errors
      wraps       edges that loop the canvas (drawn length >> direct distance)
      top_ports   top-side ports on nodes below the top row (those have no
                  title-region lane to use, so a top port there hugs a box)
      congestion  total lane cells where two edges run king-adjacent
      length      total drawn edge length
    diagnostics: [{code, edge_ids, at, message, suggestion}] for wrap and
    congestion, so an iterating agent can act on the specific edges."""
    diagnostics = []
    by_id = {n.id: n for n in m.nodes}

    crossings = len({tuple(sorted(e.edge_ids))
                     for e in m.errors if e.code == "crossing"})

    # unroutable + port-overflow edges -- these have NO route drawn, the worst
    # outcome; an iterating loop must not be allowed to "win" by dropping edges.
    dropped = sum(1 for e in m.errors if e.code in ("unroutable", "validation"))

    # wraps
    wraps = 0
    for e in m.edges:
        if not e.route or e.from_port is None or e.to_port is None:
            continue
        direct = _manhattan(e)
        routed = len(_route_cell_set(e.route))
        if routed - direct > WRAP_EXCESS:
            wraps += 1
            diagnostics.append({
                "code": "wrap", "edge_ids": [e.id], "at": list(e.route[0]),
                "message": f"edge {e.id} loops the canvas "
                           f"(drawn {routed} cells vs {direct} direct)",
                "suggestion": "route it earlier via hints.routing_order, or "
                              "pick a from_side facing its target"})

    # top ports on non-top-row nodes
    top_ports = 0
    for e in m.edges:
        for end, p in (("from", e.from_port), ("to", e.to_port)):
            if p is None or p.side != "top":
                continue
            nid = e.from_id if end == "from" else e.to_id
            if by_id[nid].grid_row > 0:
                top_ports += 1

    # congestion: king-adjacent lane cells between distinct edges
    owner = {}
    for e in m.edges:
        if not e.route:
            continue
        for c in _route_cell_set(e.route):
            if _is_node_region(m, c[0], c[1]):
                continue                      # port stubs bunch legitimately
            owner.setdefault(c, e.id)
    pair_cells = {}
    for (x, y), eid in owner.items():
        for dx, dy in _KING8:
            o = owner.get((x + dx, y + dy))
            if o is not None and o != eid:
                key = tuple(sorted((eid, o)))
                pair_cells[key] = pair_cells.get(key, 0) + 1
    congestion = 0
    for (a, b), cnt in sorted(pair_cells.items()):
        cnt //= 2                              # each adjacency counted twice
        congestion += cnt
        if cnt >= CONGEST_MIN:
            diagnostics.append({
                "code": "congestion", "edge_ids": [a, b], "at": None,
                "message": f"edges {a} and {b} run parallel for {cnt} cells "
                           f"with no gap",
                "suggestion": "give one of them a different port side via "
                              "hints.ports so they take separate lanes"})

    length = sum(len(_route_cell_set(e.route)) for e in m.edges if e.route)

    quality = {"dropped": dropped, "crossings": crossings, "wraps": wraps,
               "top_ports": top_ports, "congestion": congestion,
               "length": length}
    return quality, diagnostics

def _port_json(p):
    return None if p is None else {"side": p.side, "x": p.x, "y": p.y}

def _result_json(m):
    quality, diagnostics = _quality_and_diagnostics(m)
    return {
        "status": "error" if m.errors else "ok",
        "errors": [{"code": e.code, "edge_ids": e.edge_ids, "at": e.at,
                    "message": e.message, "suggestion": e.suggestion}
                   for e in m.errors],
        "quality": quality,
        "diagnostics": diagnostics,
        "title": m.title, "description": m.description,
        "canvas": {"width": m.canvas_w, "height": m.canvas_h},
        "box": {"width": m.box_w, "height": m.box_h},
        "nodes": [{"id": n.id, "label": n.label,
                   "grid_col": n.grid_col, "grid_row": n.grid_row,
                   "x": n.x, "y": n.y, "w": n.w, "h": n.h} for n in m.nodes],
        "edges": [{"id": e.id, "from": e.from_id, "to": e.to_id, "char": e.char,
                   "from_port": _port_json(e.from_port),
                   "to_port": _port_json(e.to_port),
                   "route": e.route} for e in m.edges],
    }

def build_model(raw):
    """Run the full pipeline (parse -> geometry -> ports -> route) on a raw
    layout dict and return the Model. Raises ValidationError on bad input.
    Does no file I/O -- the loop in autolayout.py drives this in-process."""
    m = parse_and_validate(raw)
    geometry(m)
    assign_ports(m)
    route_all(m)
    return m

def score_key(quality):
    """Lexicographic objective for the visual-approval loop; lower is strictly
    better. `dropped` (unroutable / overflowed edges, drawn nowhere) dominates
    -- the loop must never "win" by losing an edge. Then wraps (an edge looping
    the canvas reads worse, and is harder to hand-fix, than a clean crossing),
    then crossings, then top ports, then congestion, then total length."""
    return (quality["dropped"], quality["wraps"], quality["crossings"],
            quality["top_ports"], quality["congestion"], quality["length"])

def quality_of(m):
    return _quality_and_diagnostics(m)[0]

def _write_json(path, obj):
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, indent=2)
    os.replace(tmp, path)

def _validation_error_result(raw, message, suggestion):
    """Error result.json with the full top-level key set, so the consumer
    can rely on every key existing regardless of which path wrote the file."""
    is_dict = isinstance(raw, dict)
    return {
        "status": "error",
        "errors": [{"code": "validation", "edge_ids": [], "at": None,
                    "message": message, "suggestion": suggestion}],
        "quality": {"dropped": 0, "crossings": 0, "wraps": 0, "top_ports": 0,
                    "congestion": 0, "length": 0},
        "diagnostics": [],
        "title": raw.get("title", "") if is_dict else "",
        "description": raw.get("description", "") if is_dict else "",
        "canvas": {"width": 0, "height": 0},
        "box": {"width": 0, "height": 0},
        "nodes": [],
        "edges": [],
    }

def main(argv):
    if len(argv) < 2:
        print("usage: layout.py layout.json", file=sys.stderr)
        sys.exit(2)

    # Stale outputs from a previous run must never be mistaken for this
    # run's results, even if we crash before writing anything new.
    for stale in ("result.json", "result.txt"):
        with contextlib.suppress(FileNotFoundError):
            os.remove(stale)

    try:
        with open(argv[1], encoding="utf-8") as f:
            raw = json.load(f)
    except (OSError, json.JSONDecodeError) as exc:
        _write_json("result.json", _validation_error_result(
            None, str(exc),
            "ensure the layout.json path is correct and the file is valid JSON"))
        sys.exit(1)

    if not isinstance(raw, dict):
        _write_json("result.json", _validation_error_result(
            raw, "layout.json top level must be a JSON object",
            "ensure the layout.json path is correct and the file is valid JSON"))
        sys.exit(1)

    try:
        m = parse_and_validate(raw)
    except ValidationError as exc:
        _write_json("result.json", _validation_error_result(
            raw, str(exc), "fix layout.json per the message above"))
        sys.exit(1)

    geometry(m)
    assign_ports(m)
    route_all(m)
    cv = Canvas(m.canvas_w, m.canvas_h)
    render(m, cv, "result.txt")
    result = _result_json(m)
    _write_json("result.json", result)
    if result["status"] == "error":
        sys.exit(1)

if __name__ == "__main__":
    main(sys.argv)
