import layout

def prep(nodes, edges, hints=None):
    raw = {"title": "T", "description": "D", "nodes": nodes, "edges": edges}
    if hints: raw["hints"] = hints
    m = layout.parse_and_validate(raw)
    layout.geometry(m)
    layout.assign_ports(m)
    layout.route_all(m)
    return m

def is_axis_aligned(route):
    return all(route[i][0] == route[i+1][0] or route[i][1] == route[i+1][1]
               for i in range(len(route)-1))

def route_cells(route):
    out = set()
    for (x0, y0), (x1, y1) in zip(route, route[1:]):
        if x0 == x1:
            for y in range(min(y0, y1), max(y0, y1) + 1): out.add((x0, y))
        else:
            for x in range(min(x0, x1), max(x0, x1) + 1): out.add((x, y0))
    return out

TWO = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
       {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}]

def test_straight_line_route():
    m = prep(TWO, [{"id": "e1", "from": "a", "to": "b"}])
    e = m.edges[0]
    assert e.route is not None and len(e.route) >= 2
    assert is_axis_aligned(e.route)
    assert not m.errors

def test_diagonal_route_bends_through_lane_center():
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
             {"id": "b", "label": "b", "grid_col": 1, "grid_row": 1}]
    m = prep(nodes, [{"id": "e1", "from": "a", "to": "b"}])
    e = m.edges[0]
    assert is_axis_aligned(e.route)
    # Edges may only enter node regions on their port stubs, so a diagonal
    # cannot cut a single elbow through the box rows; it steps into the
    # vertical lane and drops down its centre track -> a Z with two bends.
    assert len(e.route) == 4
    # the vertical leg sits on the lane centre column
    lane_center = layout._band(m.col_bands, e.route[1][0])[1]
    assert e.route[1][0] == lane_center == e.route[2][0]
    assert not m.errors

def test_parallel_edges_use_distinct_tracks():
    nodes = TWO + [{"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
                   {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}]
    edges = [{"id": "e1", "from": "a", "to": "b"},
             {"id": "e2", "from": "c", "to": "d"}]
    m = prep(nodes, edges)
    assert not (route_cells(m.edges[0].route) & route_cells(m.edges[1].route))
    assert not m.errors

def test_routes_avoid_boxes_and_gutter():
    nodes = TWO + [{"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
                   {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}]
    edges = [{"id": "e1", "from": "a", "to": "d"}]   # diagonal, must elbow around
    m = prep(nodes, edges)
    cells = route_cells(m.edges[0].route)
    for n in m.nodes:
        box = {(x, y) for x in range(n.x, n.x + n.w) for y in range(n.y, n.y + n.h)}
        assert not (cells & box)
    assert all(x > layout.GUTTER_W for (x, y) in cells)

def test_routes_respect_routing_order():
    m = prep(TWO, [{"id": "e1", "from": "a", "to": "b"}],
             hints={"routing_order": ["e1"]})
    assert m.edges[0].route is not None

def test_all_edges_routed_or_reported():
    m = prep(TWO, [{"id": "e1", "from": "a", "to": "b"}])
    for e in m.edges:
        assert e.route is not None or any(e.id in err.edge_ids for err in m.errors)

def test_parallel_segments_keep_a_gap():
    # Three edges fanning from one node's right side into a stacked column of
    # targets share the vertical lane. Their vertical legs must stay >=1 cell
    # apart -- no two parallel runs on adjacent columns -- so the lane reads
    # cleanly instead of as a solid block.
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 1},
             {"id": "t0", "label": "t0", "grid_col": 1, "grid_row": 0},
             {"id": "t1", "label": "t1", "grid_col": 1, "grid_row": 1},
             {"id": "t2", "label": "t2", "grid_col": 1, "grid_row": 2}]
    edges = [{"id": "e0", "from": "a", "to": "t0"},
             {"id": "e1", "from": "a", "to": "t1"},
             {"id": "e2", "from": "a", "to": "t2"}]
    m = prep(nodes, edges)
    assert not m.errors
    routes = [route_cells(e.route) for e in m.edges]
    # no shared cells at all
    for i in range(len(routes)):
        for j in range(i + 1, len(routes)):
            assert not (routes[i] & routes[j])
    # the two non-straight legs occupy distinct, non-adjacent lane columns
    def vertical_cols(route):
        return {x for (x0, y0), (x1, y1) in zip(route, route[1:])
                if x0 == x1 for x in [x0] if y0 != y1}
    cols = set().union(*(vertical_cols(e.route) for e in m.edges))
    ordered = sorted(cols)
    assert all(b - a >= 2 for a, b in zip(ordered, ordered[1:])), ordered

def test_no_two_edges_share_a_2x2_block_in_lanes():
    # The 2x2-cluster rule: in the lane bands, no cell of one edge may be
    # king-adjacent (incl. diagonally) to a cell of another edge. Only port
    # stubs inside node regions are exempt.
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 1},
             {"id": "t0", "label": "t0", "grid_col": 1, "grid_row": 0},
             {"id": "t1", "label": "t1", "grid_col": 1, "grid_row": 1},
             {"id": "t2", "label": "t2", "grid_col": 1, "grid_row": 2}]
    edges = [{"id": "e0", "from": "a", "to": "t0"},
             {"id": "e1", "from": "a", "to": "t1"},
             {"id": "e2", "from": "a", "to": "t2"}]
    m = prep(nodes, edges)
    assert not m.errors
    # map each lane cell to its owning edge
    owner = {}
    for e in m.edges:
        for c in route_cells(e.route):
            if layout._is_node_region(m, c[0], c[1]):
                continue   # stubs exempt
            owner.setdefault(c, e.id)
    king = [(1, 0), (-1, 0), (0, 1), (0, -1),
            (1, 1), (1, -1), (-1, 1), (-1, -1)]
    for (x, y), eid in owner.items():
        for dx, dy in king:
            other = owner.get((x + dx, y + dy))
            assert other is None or other == eid, (
                f"edges {eid} and {other} share a 2x2 block near {(x, y)}")

def _astar(start, goal, blocked, occupied, allow_cross, w, h, forbidden=None):
    """Adapter: the engine's _astar reads canvas bounds and lane bands off a
    Model. Build a minimal one whose bands span the whole test canvas as a
    single lane (centre off-canvas so centering is inert) so these unit tests
    keep exercising bounds/crossing/spacing logic with explicit coordinates.
    `forbidden` (the 2x2-spacing halo) defaults to empty for the direct tests."""
    m = layout.Model(title="", description="", nodes=[], edges=[])
    m.canvas_w, m.canvas_h = w, h
    m.col_bands = [(0, w, "lane", None)]
    m.row_bands = [(0, h, "lane", None)]
    layout._build_band_caches(m)
    return layout._astar(start, goal, blocked, occupied,
                         forbidden or set(), allow_cross, m)

def test_astar_pass1_rejects_occupied_start():
    cells, crossings = _astar((10, 5), (20, 5), set(), {(10, 5): "z"},
                              False, 30, 20)
    assert cells is None and crossings is None

def test_pass2_minimizes_crossings_over_turns():
    # an occupied horizontal track sits between start and goal on the same
    # row; running along it would save two turns but cost nine crossings.
    # crossings must dominate: the route detours and crosses nothing.
    occupied = {(x, 12): "z" for x in range(11, 20)}
    cells, crossings = _astar((10, 12), (20, 12), set(), occupied,
                              True, 30, 20)
    assert cells is not None
    assert crossings == []

def test_pass2_prefers_gapped_track_over_hugging():
    # A pass-2 (crossing) edge must keep its 2x2 gap where it can: spacing is a
    # soft cost in pass 2, not disabled. Here a vertical track of edge z sits at
    # x=15; an edge running from (12,5) to (12,25) on its own column should NOT
    # drift over to hug z at x=14 -- the `forbidden` halo of z makes x=14 cost
    # an adjacency it has no reason to pay.
    occupied = {(15, y): "z" for y in range(5, 26)}
    forbidden = set()
    for (x, y) in occupied:
        for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1),
                       (1, 1), (1, -1), (-1, 1), (-1, -1)]:
            forbidden.add((x + dx, y + dy))
    cells, _ = _astar((12, 5), (12, 25), set(), occupied, True, 30, 30,
                      forbidden=forbidden)
    assert cells is not None
    # the route stays on its own column, never stepping onto z's halo (x=14)
    assert all(c[0] <= 13 for c in cells), [c for c in cells if c[0] > 13]

def test_crossing_runs_groups_consecutive_cells():
    cells = [(10, 1), (10, 2), (10, 3), (10, 4), (10, 5), (10, 6)]
    # one owner, two separate runs (gap at (10,4))
    occupied = {(10, 2): "a", (10, 3): "a", (10, 5): "a"}
    assert layout._crossing_runs(cells, occupied) == [("a", (10, 2)), ("a", (10, 5))]
    # owner changes mid-run -> two runs, no gap needed
    occupied2 = {(10, 2): "a", (10, 3): "b"}
    assert layout._crossing_runs(cells, occupied2) == [("a", (10, 2)), ("b", (10, 3))]
    assert layout._crossing_runs(cells, {}) == []

def test_k5_forced_crossings_all_routed():
    # K5 is non-planar: on a 3x2 grid at least one crossing is unavoidable,
    # but every edge must still get a route (pass 2 never gives up on
    # reachable goals).
    import itertools
    coords = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1)]
    nodes = [{"id": f"n{i+1}", "label": f"n{i+1}", "grid_col": c, "grid_row": r}
             for i, (c, r) in enumerate(coords)]
    edges = [{"id": f"e{a}{b}", "from": f"n{a}", "to": f"n{b}"}
             for a, b in itertools.combinations(range(1, 6), 2)]
    m = prep(nodes, edges)
    crossing = [err for err in m.errors if err.code == "crossing"]
    assert crossing
    assert all(e.route is not None for e in m.edges)
    # deduped: at most one error per (pair, location)
    keys = [(frozenset(err.edge_ids), tuple(err.at)) for err in crossing]
    assert len(keys) == len(set(keys))

def test_astar_start_blocked_unroutable():
    cells, crossings = _astar((10, 5), (20, 5), {(10, 5)}, {},
                              True, 30, 20)
    assert cells is None and crossings is None

def test_astar_walled_goal_unroutable():
    goal = (20, 10)
    blocked = {(19, 10), (21, 10), (20, 9), (20, 11)}   # 4-connected wall
    cells, crossings = _astar((10, 10), goal, blocked, {},
                              True, 30, 20)
    assert cells is None and crossings is None

def test_overflow_edge_skipped_without_new_error():
    # 4 parallel same-column edges on narrow boxes overflow top/bottom capacity
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
             {"id": "b", "label": "b", "grid_col": 0, "grid_row": 1}]
    edges = [{"id": f"e{i}", "from": "a", "to": "b"} for i in range(4)]
    m = prep(nodes, edges)
    overflow_errors = [e for e in m.errors if e.code == "validation"]
    assert overflow_errors                       # overflow happened in port stage
    unported = [e for e in m.edges if e.from_port is None or e.to_port is None]
    assert unported
    for e in unported:
        assert e.route is None
    # routing added no unroutable/crossing error for unported edges
    unported_ids = {e.id for e in unported}
    for err in m.errors:
        if err.code in ("unroutable", "crossing"):
            assert not (set(err.edge_ids) & unported_ids)
