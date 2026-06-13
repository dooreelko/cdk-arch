import layout

def build(nodes, edges, hints=None):
    raw = {"title": "T", "description": "D", "nodes": nodes, "edges": edges}
    if hints: raw["hints"] = hints
    m = layout.parse_and_validate(raw)
    layout.geometry(m)
    layout.assign_ports(m)
    return m

NODES = [
    {"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
    {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
    {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
]

def test_forward_edge_default_sides():
    m = build(NODES, [{"id": "e1", "from": "a", "to": "b"}])
    e = m.edges[0]
    assert e.from_port.side == "right"
    assert e.to_port.side == "left"

def test_same_column_default_sides():
    m = build(NODES, [{"id": "e1", "from": "a", "to": "c"}])
    e = m.edges[0]
    assert e.from_port.side == "bottom"
    assert e.to_port.side == "top"

def test_backward_edge_default_sides():
    # Outbound still leaves b's left (data flows back leftward), but the
    # inbound right side is prohibited: a's left faces the gutter (no lane),
    # so the chooser falls to bottom -- which routes cleanly.
    m = build(NODES, [{"id": "e1", "from": "b", "to": "a"}])
    e = m.edges[0]
    assert e.from_port.side == "left"
    assert e.to_port.side != "right"
    layout.route_all(m)
    assert e.route is not None and not m.errors

def test_inbound_never_uses_right_side():
    # Every inbound port across forward/backward/vertical edges avoids 'right'.
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
             {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
             {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
             {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}]
    edges = [{"id": "e1", "from": "a", "to": "b"},
             {"id": "e2", "from": "b", "to": "a"},
             {"id": "e3", "from": "a", "to": "d"},
             {"id": "e4", "from": "d", "to": "a"}]
    m = build(nodes, edges)
    for e in m.edges:
        assert e.to_port.side != "right", (e.id, e.to_port.side)

def test_inbound_right_hint_rejected():
    import pytest
    with pytest.raises(layout.ValidationError):
        build(NODES, [{"id": "e1", "from": "a", "to": "b"}],
              hints={"ports": [{"edge_id": "e1", "to_side": "right"}]})

def test_hint_overrides_sides():
    m = build(NODES, [{"id": "e1", "from": "a", "to": "b"}],
              hints={"ports": [{"edge_id": "e1", "from_side": "top", "to_side": "bottom"}]})
    e = m.edges[0]
    assert e.from_port.side == "top"
    assert e.to_port.side == "bottom"

def test_ports_lie_on_box_border():
    m = build(NODES, [{"id": "e1", "from": "a", "to": "b"}])
    a = next(n for n in m.nodes if n.id == "a")
    p = m.edges[0].from_port
    assert p.x == a.x + a.w - 1           # right border column
    assert a.y <= p.y < a.y + a.h

def test_multiple_ports_on_side_stack_on_distinct_rows():
    nodes = NODES + [{"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}]
    edges = [{"id": "e1", "from": "a", "to": "b"},
             {"id": "e2", "from": "a", "to": "d"}]
    m = build(nodes, edges)               # both leave 'a' on the right
    ys = [e.from_port.y for e in m.edges]
    assert len(set(ys)) == 2              # distinct rows, no overlap

def test_port_overflow_reports_error_and_caps_assignments():
    """Port overflow: narrow node with 4 edges on same side exceeds capacity."""
    # Single-char label → box_w = 1 + LABEL_PAD = 5, interior = 3
    # 4 parallel edges on top side → capacity 3, overflow 1
    nodes = [
        {"id": "n0", "label": "x", "grid_col": 0, "grid_row": 0},
        {"id": "n1", "label": "x", "grid_col": 0, "grid_row": 1},
        {"id": "n2", "label": "x", "grid_col": 0, "grid_row": 2},
        {"id": "n3", "label": "x", "grid_col": 0, "grid_row": 3},
        {"id": "n4", "label": "x", "grid_col": 0, "grid_row": 4},
    ]
    # 4 edges from n4 bottom going upward (will become top side for n3 as target)
    edges = [
        {"id": "e0", "from": "n0", "to": "n1"},
        {"id": "e1", "from": "n1", "to": "n2"},
        {"id": "e2", "from": "n2", "to": "n3"},
        {"id": "e3", "from": "n4", "to": "n3"},
    ]
    m = build(nodes, edges)

    # n3 receives 3 edges on its top side: e0→n3 doesn't exist, let me recalculate
    # Actually: e2 goes from n2→n3 (top of n3), e3 goes from n4→n3 (bottom of n3)
    # That's only 1 edge on top. Need 4 parallel edges to same target node.

    # Revised: 4 sources in row 0, 1 target in row 1, all same column
    nodes = [
        {"id": "s0", "label": "x", "grid_col": 0, "grid_row": 0},
        {"id": "s1", "label": "x", "grid_col": 1, "grid_row": 0},
        {"id": "s2", "label": "x", "grid_col": 2, "grid_row": 0},
        {"id": "s3", "label": "x", "grid_col": 3, "grid_row": 0},
        {"id": "t", "label": "x", "grid_col": 1, "grid_row": 1},  # target below s1
    ]
    # All 4 edges go to 't' - but they come from different columns
    # We need same-column convergence for top/bottom overflow

    # Final fixture: sources in same column, target in same column below
    nodes = [
        {"id": "t", "label": "x", "grid_col": 0, "grid_row": 1},
        {"id": "s0", "label": "y", "grid_col": 1, "grid_row": 0},
        {"id": "s1", "label": "y", "grid_col": 1, "grid_row": 1},
        {"id": "s2", "label": "y", "grid_col": 1, "grid_row": 2},
        {"id": "s3", "label": "y", "grid_col": 1, "grid_row": 3},
    ]
    edges = [
        {"id": "e0", "from": "s0", "to": "t"},
        {"id": "e1", "from": "s1", "to": "t"},
        {"id": "e2", "from": "s2", "to": "t"},
        {"id": "e3", "from": "s3", "to": "t"},
    ]
    m = build(nodes, edges)

    # All 4 edges arrive at 't' from the right (sources in col 1, target in col 0)
    # So 't' has 4 ports on its right side, capacity = box_h - 2 = 11 - 2 = 9
    # That won't overflow. Need narrower box or more edges.

    # Use hints to force all 4 onto top side
    hints = {"ports": [
        {"edge_id": "e0", "to_side": "top"},
        {"edge_id": "e1", "to_side": "top"},
        {"edge_id": "e2", "to_side": "top"},
        {"edge_id": "e3", "to_side": "top"},
    ]}
    m = build(nodes, edges, hints)

    t_node = next(n for n in m.nodes if n.id == "t")
    cap = t_node.w - 2  # Should be 5 - 2 = 3

    # First 3 edges should have ports assigned
    assigned = [e for e in m.edges if e.to_port is not None]
    overflow = [e for e in m.edges if e.to_port is None]

    assert len(assigned) == min(4, cap)
    assert len(overflow) == max(0, 4 - cap)

    # Should have exactly 1 error
    assert len(m.errors) == 1
    err = m.errors[0]
    assert err.code == "validation"
    assert len(err.edge_ids) == len(overflow)
    assert all(e.id in err.edge_ids for e in overflow)
    assert "4 ports on top side" in err.message
    assert f"capacity {cap}" in err.message
    assert "t" in err.message

def test_port_ordering_by_target_position():
    """Ports are ordered by other endpoint's center position."""
    # Hub 'h' at grid (0,1) with forward edges to (1,2) and (1,0)
    # Both leave right side. Edge to row 0 should get smaller from_port.y
    nodes = [
        {"id": "h", "label": "h", "grid_col": 0, "grid_row": 1},
        {"id": "t0", "label": "t0", "grid_col": 1, "grid_row": 0},
        {"id": "t2", "label": "t2", "grid_col": 1, "grid_row": 2},
    ]
    edges = [
        {"id": "e_to_2", "from": "h", "to": "t2"},
        {"id": "e_to_0", "from": "h", "to": "t0"},
    ]
    m = build(nodes, edges)

    # Both edges leave h's right side
    e_to_0 = next(e for e in m.edges if e.id == "e_to_0")
    e_to_2 = next(e for e in m.edges if e.id == "e_to_2")

    assert e_to_0.from_port.side == "right"
    assert e_to_2.from_port.side == "right"

    # Target at row 0 is above target at row 2, so port should be higher (smaller y)
    assert e_to_0.from_port.y < e_to_2.from_port.y

def test_top_bottom_ports_stack_horizontally():
    """Two same-column edges from one node's bottom side get distinct x coords."""
    nodes = [
        {"id": "src", "label": "src", "grid_col": 0, "grid_row": 0},
        {"id": "t0", "label": "t0", "grid_col": 0, "grid_row": 1},
        {"id": "t1", "label": "t1", "grid_col": 0, "grid_row": 2},
    ]
    edges = [
        {"id": "e0", "from": "src", "to": "t0"},
        {"id": "e1", "from": "src", "to": "t1"},
    ]
    m = build(nodes, edges)

    # Both edges leave src's bottom side (same column, downward)
    e0 = next(e for e in m.edges if e.id == "e0")
    e1 = next(e for e in m.edges if e.id == "e1")

    assert e0.from_port.side == "bottom"
    assert e1.from_port.side == "bottom"

    # Distinct x coordinates
    assert e0.from_port.x != e1.from_port.x

def test_convergent_left_side_ports_stack_vertically():
    """Two edges converging on one node's left side get distinct y coords."""
    nodes = [
        {"id": "s0", "label": "s0", "grid_col": 0, "grid_row": 0},
        {"id": "s1", "label": "s1", "grid_col": 0, "grid_row": 1},
        {"id": "target", "label": "target", "grid_col": 1, "grid_row": 0},
    ]
    edges = [
        {"id": "e0", "from": "s0", "to": "target"},
        {"id": "e1", "from": "s1", "to": "target"},
    ]
    m = build(nodes, edges)

    # Both edges arrive at target's left side
    e0 = next(e for e in m.edges if e.id == "e0")
    e1 = next(e for e in m.edges if e.id == "e1")

    assert e0.to_port.side == "left"
    assert e1.to_port.side == "left"

    # Distinct y coordinates
    assert e0.to_port.y != e1.to_port.y

def test_self_loop_default_sides():
    """Self-loops (from==to) use deterministic top→bottom sides."""
    nodes = [
        {"id": "n", "label": "n", "grid_col": 0, "grid_row": 0},
    ]
    edges = [
        {"id": "loop", "from": "n", "to": "n"},
    ]
    m = build(nodes, edges)

    e = m.edges[0]
    assert e.from_port.side == "top"
    assert e.to_port.side == "bottom"
