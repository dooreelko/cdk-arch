import layout
import pytest


def raw_with_groups(groups):
    return {"title": "T", "description": "D",
            "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                      {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
            "edges": [{"id": "e1", "from": "a", "to": "b"}], "groups": groups}


def raw_grid(groups):
    # 2x2 grid: a(0,0) b(1,0) c(0,1) d(1,1)
    return {"title": "T", "description": "D",
            "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                      {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
                      {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
                      {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}],
            "edges": [], "groups": groups}


def test_parses_valid_group_entry():
    m = layout.parse_and_validate(raw_with_groups([
        {"id": "g1", "title": "Group One", "members": ["a", "b"]}]))
    assert len(m.groups) == 1
    g = m.groups[0]
    assert g.id == "g1"
    assert g.title == "Group One"
    assert g.parent is None
    assert g.member_ids == ["a", "b"]


def test_group_parent_is_read():
    m = layout.parse_and_validate(raw_with_groups([
        {"id": "outer", "title": "O", "members": ["a"]},
        {"id": "inner", "title": "I", "members": ["b"], "parent": "outer"}]))
    inner = next(g for g in m.groups if g.id == "inner")
    assert inner.parent == "outer"


def test_no_groups_key_means_empty():
    raw = {"title": "T", "description": "D",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
           "edges": []}
    assert layout.parse_and_validate(raw).groups == []


def test_error_unknown_member():
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw_with_groups([
            {"id": "g1", "title": "G", "members": ["nope"]}]))
    assert "nope" in str(exc.value)


def test_error_unknown_parent():
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw_with_groups([
            {"id": "g1", "title": "G", "members": ["a"], "parent": "ghost"}]))
    assert "ghost" in str(exc.value)


def test_error_duplicate_group_id():
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw_with_groups([
            {"id": "g", "title": "A", "members": ["a"]},
            {"id": "g", "title": "B", "members": ["b"]}]))
    assert "duplicate group id" in str(exc.value)


def test_extent_from_direct_members():
    m = layout.parse_and_validate(raw_grid([
        {"id": "g", "title": "G", "members": ["a", "b", "c", "d"]}]))
    g = m.groups[0]
    assert (g.col0, g.col1, g.row0, g.row1) == (0, 1, 0, 1)
    assert g.depth == 0


def test_parent_extent_includes_child_and_depth_increases():
    m = layout.parse_and_validate(raw_grid([
        {"id": "outer", "title": "O", "members": ["a", "b", "c"]},
        {"id": "inner", "title": "I", "members": ["d"], "parent": "outer"}]))
    outer = next(g for g in m.groups if g.id == "outer")
    inner = next(g for g in m.groups if g.id == "inner")
    assert (outer.col0, outer.col1, outer.row0, outer.row1) == (0, 1, 0, 1)
    assert outer.depth == 0
    assert (inner.col0, inner.col1, inner.row0, inner.row1) == (1, 1, 1, 1)
    assert inner.depth == 1


def test_error_parent_cycle():
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw_grid([
            {"id": "x", "title": "X", "members": ["a"], "parent": "y"},
            {"id": "y", "title": "Y", "members": ["b"], "parent": "x"}]))
    assert "cycle" in str(exc.value)


def test_error_encloses_non_member():
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw_grid([
            {"id": "g", "title": "G", "members": ["a", "d"]}]))
    assert "encloses non-member" in str(exc.value)


def test_nesting_is_allowed():
    m = layout.parse_and_validate(raw_grid([
        {"id": "outer", "title": "O", "members": ["a", "b", "c", "d"]},
        {"id": "inner", "title": "I", "members": ["d"], "parent": "outer"}]))
    assert len(m.groups) == 2


def test_error_partial_overlap():
    raw = {"title": "T", "description": "D",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                     {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
                     {"id": "e", "label": "e", "grid_col": 2, "grid_row": 0}],
           "edges": [],
           "groups": [{"id": "g1", "title": "1", "members": ["a", "b"]},
                      {"id": "g2", "title": "2", "members": ["b", "e"]}]}
    with pytest.raises(layout.ValidationError) as exc:
        layout.parse_and_validate(raw)
    assert "overlap" in str(exc.value)


def test_vertical_ring_counts_pack_two_sides():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "mm", "label": "mm", "grid_col": 1, "grid_row": 0},
                  {"id": "z", "label": "z", "grid_col": 2, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "gl", "title": "L", "members": ["a"]},
                   {"id": "gr", "title": "R", "members": ["z"]}]})
    counts = layout.vertical_ring_counts(m.groups)
    # left bounding lane (region -1): gl's LEFT border on right side of that lane
    assert counts.get(-1, (0, 0)) == (0, 1)
    # lane region 1 (right of col0): gl's RIGHT border -> left_rings = 1
    assert counts.get(1, (0, 0))[0] == 1
    # lane region 3 (left of col2): gr's LEFT border -> right_rings = 1
    assert counts.get(3, (0, 0))[1] == 1


def test_groupless_geometry_unchanged():
    raw = {"title": "T", "description": "D",
           "nodes": [{"id": "a", "label": "alpha", "grid_col": 0, "grid_row": 0},
                     {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
           "edges": [{"id": "e1", "from": "a", "to": "b"}]}
    m = layout.parse_and_validate(raw)
    layout.geometry(m)
    a = next(n for n in m.nodes if n.id == "a")
    assert a.x == layout.GUTTER_W + 1 + layout.BOX_MARGIN_X


def test_left_lane_added_when_groups_present():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "g", "title": "G", "members": ["a"]}]})
    layout.geometry(m)
    a = next(n for n in m.nodes if n.id == "a")
    assert a.x > layout.GUTTER_W + 1 + layout.BOX_MARGIN_X


def test_group_box_surrounds_member_nodes():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "g", "title": "G", "members": ["a", "b"]}]})
    layout.geometry(m)
    g = m.groups[0]
    a = next(n for n in m.nodes if n.id == "a")
    b = next(n for n in m.nodes if n.id == "b")
    assert g.x < a.x
    assert g.x + g.w - 1 > b.x + b.w - 1
    assert g.y < a.y
    assert g.y + g.h - 1 > a.y + a.h - 1


def test_nested_group_inside_parent_box():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "outer", "title": "O", "members": ["a", "b"]},
                   {"id": "inner", "title": "I", "members": ["b"], "parent": "outer"}]})
    layout.geometry(m)
    outer = next(g for g in m.groups if g.id == "outer")
    inner = next(g for g in m.groups if g.id == "inner")
    assert outer.x < inner.x
    assert outer.y < inner.y and outer.y + outer.h > inner.y + inner.h


def test_border_cells_classify_sides():
    g = layout.Group(id="g", title="g", parent=None, member_ids=[], depth=0,
                     col0=0, col1=0, row0=0, row1=0, x=10, y=10, w=5, h=4)
    vert, horiz = layout.border_cells([g])
    assert (10, 11) in vert       # left edge, mid
    assert (14, 11) in vert       # right edge, mid
    assert (11, 10) in horiz      # top edge, mid
    assert (11, 13) in horiz      # bottom edge, mid
    assert (10, 10) in vert and (10, 10) in horiz   # corner = both


def test_lane_sizing_adds_pad_only_when_rings_present():
    pad = layout.GROUP_PAD
    # No rings -> exactly the legacy minimum (backward compat).
    assert layout.lane_width(0, 0) == layout.LANE_MIN_W
    assert layout.lane_height(0, 0) == layout.LANE_MIN_H
    # Each populated side adds its ring columns plus one PAD gap.
    assert layout.lane_width(2, 0) == 2 + pad + layout.LANE_MIN_W
    assert layout.lane_width(0, 1) == layout.LANE_MIN_W + 1 + pad
    assert layout.lane_width(2, 1) == 2 + pad + layout.LANE_MIN_W + 1 + pad
    assert layout.lane_height(1, 3) == 1 + pad + layout.LANE_MIN_H + 3 + pad


def test_edge_cannot_run_along_a_group_border():
    # a(0,0) -> c(0,1): the edge travels vertically; the group wrapping both
    # forces vertical borders in the surrounding lanes. No vertical segment of
    # the route may sit on a vertical-border cell (no run-along), and likewise
    # for horizontal segments on horizontal borders.
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1}],
        "edges": [{"id": "e1", "from": "a", "to": "c"}],
        "groups": [{"id": "g", "title": "G", "members": ["a", "c"]}]})
    layout.geometry(m)
    layout.assign_ports(m)
    layout.route_all(m)
    vborder, hborder = layout.border_cells(m.groups)
    route = m.edges[0].route
    assert route is not None
    for (x0, y0), (x1, y1) in zip(route, route[1:]):
        if x0 == x1:
            for y in range(min(y0, y1), max(y0, y1) + 1):
                assert (x0, y) not in vborder, \
                    f"vertical run along a vertical border at ({x0},{y})"
        else:
            for x in range(min(x0, x1), max(x0, x1) + 1):
                assert (x, y0) not in hborder, \
                    f"horizontal run along a horizontal border at ({x},{y0})"


def test_three_level_nesting_extents_and_depth():
    # 3x1 row a(0) b(1) e(2): outer wraps all, mid wraps b+e, inner wraps e.
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
                  {"id": "e", "label": "e", "grid_col": 2, "grid_row": 0}],
        "edges": [],
        "groups": [
            {"id": "outer", "title": "O", "members": ["a"]},
            {"id": "mid", "title": "M", "members": ["b"], "parent": "outer"},
            {"id": "inner", "title": "I", "members": ["e"], "parent": "mid"}]})
    by_id = {g.id: g for g in m.groups}
    assert (by_id["outer"].col0, by_id["outer"].col1) == (0, 2)
    assert by_id["outer"].depth == 0
    assert (by_id["mid"].col0, by_id["mid"].col1) == (1, 2)
    assert by_id["mid"].depth == 1
    assert (by_id["inner"].col0, by_id["inner"].col1) == (2, 2)
    assert by_id["inner"].depth == 2


def test_edge_crosses_group_border_perpendicular():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
        "groups": [{"id": "g", "title": "G", "members": ["a"]}],
        "hints": {"ports": [{"edge_id": "e1", "from_side": "right", "to_side": "left"}]}})
    layout.geometry(m)
    layout.assign_ports(m)
    layout.route_all(m)
    e = m.edges[0]
    assert e.route is not None
    assert all(er.code != "unroutable" for er in m.errors)


def test_left_bounding_lane_carries_no_edges():
    # A group wraps both nodes so the left bounding lane (region -1) exists. No
    # routed edge cell may land in that lane's column span -- it is reserved for
    # the outermost group frame.
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
        "groups": [{"id": "g", "title": "G", "members": ["a", "b"]}]})
    layout.geometry(m)
    layout.assign_ports(m)
    layout.route_all(m)
    lx = m.col_x[-1]
    lane = next(b for b in m.col_bands if b[0] == lx and b[2] == "lane")
    for e in m.edges:
        if e.route is None:
            continue
        for (cx, _cy) in layout._route_cell_set(e.route):
            assert cx < lane[0] or cx >= lane[1], \
                f"edge {e.id} routed into the left bounding lane at x={cx}"


def _render_to_string(m, tmp_path):
    layout.geometry(m)
    layout.assign_ports(m)
    layout.route_all(m)
    cv = layout.Canvas(m.canvas_w, m.canvas_h)
    layout.render(m, cv, str(tmp_path / "out.txt"))
    return str(cv)


def test_renders_double_line_frame_and_title(tmp_path):
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "alpha", "grid_col": 0, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "g", "title": "MyGroup", "members": ["a"]}]})
    out = _render_to_string(m, tmp_path)
    assert "╔" in out and "╗" in out and "╚" in out and "╝" in out
    assert "MyGroup" in out


def test_title_sits_inside_bottom_left(tmp_path):
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "alpha", "grid_col": 0, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "g", "title": "GG", "members": ["a"]}]})
    layout.geometry(m)
    g = m.groups[0]
    # one blank row above the bottom border (y1-2) and one blank cell in from
    # the left border (x0+2) -- a one-cell gap from each side.
    title_row = g.y + g.h - 3
    title_col = g.x + 2
    out = _render_to_string(m, tmp_path)
    lines = out.split("\n")
    cols = list(lines[title_row])
    assert cols[title_col] == "G"


def test_result_json_includes_groups():
    m = layout.parse_and_validate({
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
        "edges": [],
        "groups": [{"id": "g", "title": "G", "members": ["a"]}]})
    layout.geometry(m)
    rj = layout._result_json(m)
    groups = rj["groups"]
    assert len(groups) == 1
    assert groups[0]["id"] == "g"
    assert groups[0]["title"] == "G"
    assert isinstance(groups[0]["x"], int) and isinstance(groups[0]["w"], int)


def test_result_json_groups_empty_when_none():
    raw = {"title": "T", "description": "D",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
           "edges": []}
    m = layout.parse_and_validate(raw)
    layout.geometry(m)
    assert len(layout._result_json(m)["groups"]) == 0
