import layout

def model_2x2():
    raw = {
        "title": "T", "description": "D",
        "nodes": [
            {"id": "a", "label": "alpha",   "grid_col": 0, "grid_row": 0},
            {"id": "b", "label": "b",       "grid_col": 1, "grid_row": 0},
            {"id": "c", "label": "charlie", "grid_col": 0, "grid_row": 1},
        ],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
    }
    return layout.parse_and_validate(raw)

def test_box_width_from_widest_label():
    m = model_2x2()
    layout.geometry(m)
    assert m.box_w == len("charlie") + layout.LABEL_PAD
    assert m.box_h == layout.BOX_H

def test_all_boxes_identical_size():
    m = model_2x2()
    layout.geometry(m)
    ws = {n.w for n in m.nodes}; hs = {n.h for n in m.nodes}
    assert ws == {m.box_w} and hs == {m.box_h}

def test_nodes_get_positive_coords_inside_canvas():
    m = model_2x2()
    layout.geometry(m)
    assert m.canvas_w > 0 and m.canvas_h > 0
    for n in m.nodes:
        assert n.x > layout.GUTTER_W
        assert n.x + n.w < m.canvas_w
        assert n.y >= layout.TITLE_H
        assert n.y + n.h < m.canvas_h

def test_column_and_row_ordering():
    m = model_2x2()
    layout.geometry(m)
    a = next(n for n in m.nodes if n.id == "a")
    b = next(n for n in m.nodes if n.id == "b")
    c = next(n for n in m.nodes if n.id == "c")
    assert a.x < b.x          # col 0 left of col 1
    assert a.y < c.y          # row 0 above row 1
    assert a.y == b.y         # same row -> same y

def test_exact_geometry_offsets():
    m = model_2x2()
    layout.geometry(m)
    # Widest label is "charlie" (7 chars)
    # With LABEL_PAD=4: box_w = 11
    assert m.box_w == 11
    # node_col_w = 11 + 2*BOX_MARGIN_X = 11 + 8 = 19
    node_col_w = 19
    # Column x offsets: start at GUTTER_W+1=9
    # col 0: 9, lane 0: 28, col 1: 44, lane 1: 63
    assert m.col_x == {0: 9, 1: 28, 2: 44, 3: 63}
    # Canvas width: 63 + LANE_MIN_W = 79
    assert m.canvas_w == 79
    # Row y offsets: start at 0. A routing lane (LANE_MIN_H=7) sits between
    # the title and node row 0 so edges approaching the top row from above
    # have a lane to gravitate into instead of hugging the box tops.
    # title: 0, top lane: 6, row 0: 13, lane 0: 28, row 1: 35, lane 1: 50
    assert m.row_y == {0: 0, 1: 13, 2: 28, 3: 35, 4: 50}
    # Canvas height: 50 + LANE_MIN_H = 57
    assert m.canvas_h == 57
    # Node "a" at grid (0,0): x = col_x[0] + BOX_MARGIN_X = 13
    #                          y = row_y[1] + BOX_MARGIN_Y = 15
    a = next(n for n in m.nodes if n.id == "a")
    assert (a.x, a.y) == (13, 15)

def test_top_lane_above_first_node_row():
    m = model_2x2()
    layout.geometry(m)
    # title band first, then a routing lane, then the first node band
    kinds = [b[2] for b in m.row_bands]
    assert kinds[0] == "title"
    assert kinds[1] == "lane"
    assert kinds[2] == "node"
    # the lane has a centre track between the title and node row 0
    title_end = m.row_bands[0][1]
    node0_start = m.row_bands[2][0]
    lane_center = m.row_bands[1][3]
    assert title_end <= lane_center < node0_start
