import layout
import pytest

def test_module_exposes_data_model():
    n = layout.Node(id="a", label="A", grid_col=0, grid_row=0)
    assert n.x == 0 and n.w == 0
    assert len(layout.EDGE_ALPHABET) == 62
    assert layout.SIDES == ("left", "right", "top", "bottom")

def base():
    return {
        "title": "T", "description": "D",
        "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                  {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
        "edges": [{"id": "e1", "from": "a", "to": "b"}],
    }

def test_parse_ok_assigns_chars_in_order():
    raw = base()
    raw["edges"].append({"id": "e2", "from": "b", "to": "a"})
    m = layout.parse_and_validate(raw)
    assert [e.char for e in m.edges] == ["0", "1"]
    assert m.title == "T" and m.description == "D"

def test_duplicate_node_id():
    raw = base()
    raw["nodes"].append({"id": "a", "label": "x", "grid_col": 2, "grid_row": 0})
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_duplicate_edge_id():
    raw = base()
    raw["edges"].append({"id": "e1", "from": "b", "to": "a"})
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_edge_unknown_node():
    raw = base()
    raw["edges"][0]["to"] = "zzz"
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_two_nodes_same_cell():
    raw = base()
    raw["nodes"][1]["grid_col"] = 0  # same as node a
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_bad_hint_edge_id():
    raw = base()
    raw["hints"] = {"ports": [{"edge_id": "nope", "from_side": "right", "to_side": "left"}]}
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_bad_hint_side():
    raw = base()
    raw["hints"] = {"ports": [{"edge_id": "e1", "from_side": "sideways", "to_side": "left"}]}
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_too_many_edges():
    raw = base()
    raw["nodes"] = [{"id": f"n{i}", "label": "x", "grid_col": i, "grid_row": 0} for i in range(63)]
    raw["edges"] = [{"id": f"e{i}", "from": "n0", "to": f"n{i}"} for i in range(1, 64)]  # 63 edges
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_bad_routing_order_edge_id():
    raw = base()
    raw["hints"] = {"routing_order": ["nope"]}
    with pytest.raises(layout.ValidationError):
        layout.parse_and_validate(raw)

def test_missing_nodes_key():
    raw = {"title": "T", "description": "D", "edges": []}
    with pytest.raises(layout.ValidationError, match=r"missing required key.*nodes"):
        layout.parse_and_validate(raw)

def test_missing_edges_key():
    raw = {"title": "T", "description": "D", "nodes": []}
    with pytest.raises(layout.ValidationError, match=r"missing required key.*edges"):
        layout.parse_and_validate(raw)

def test_empty_edges_allowed():
    raw = {"title": "T", "description": "D",
           "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
           "edges": []}
    m = layout.parse_and_validate(raw)
    assert m.edges == []

def test_empty_nodes_rejected():
    raw = {"title": "T", "description": "D", "nodes": [], "edges": []}
    with pytest.raises(layout.ValidationError, match=r"nodes must not be empty"):
        layout.parse_and_validate(raw)

def test_missing_node_id():
    raw = base()
    raw["nodes"].append({"label": "x", "grid_col": 2, "grid_row": 0})
    with pytest.raises(layout.ValidationError, match=r"node at index 2.*missing.*id"):
        layout.parse_and_validate(raw)

def test_missing_node_label():
    raw = base()
    raw["nodes"].append({"id": "c", "grid_col": 2, "grid_row": 0})
    with pytest.raises(layout.ValidationError, match=r"node at index 2.*id='c'.*missing.*label"):
        layout.parse_and_validate(raw)

def test_missing_node_grid_col():
    raw = base()
    raw["nodes"].append({"id": "c", "label": "C", "grid_row": 0})
    with pytest.raises(layout.ValidationError, match=r"node at index 2.*id='c'.*missing.*grid_col"):
        layout.parse_and_validate(raw)

def test_missing_node_grid_row():
    raw = base()
    raw["nodes"].append({"id": "c", "label": "C", "grid_col": 2})
    with pytest.raises(layout.ValidationError, match=r"node at index 2.*id='c'.*missing.*grid_row"):
        layout.parse_and_validate(raw)

def test_missing_edge_id():
    raw = base()
    raw["edges"].append({"from": "a", "to": "b"})
    with pytest.raises(layout.ValidationError, match=r"edge at index 1.*missing.*id"):
        layout.parse_and_validate(raw)

def test_missing_edge_from():
    raw = base()
    raw["edges"].append({"id": "e2", "to": "b"})
    with pytest.raises(layout.ValidationError, match=r"edge at index 1.*id='e2'.*missing.*from"):
        layout.parse_and_validate(raw)

def test_missing_edge_to():
    raw = base()
    raw["edges"].append({"id": "e2", "from": "a"})
    with pytest.raises(layout.ValidationError, match=r"edge at index 1.*id='e2'.*missing.*to"):
        layout.parse_and_validate(raw)

def test_missing_hint_port_edge_id():
    raw = base()
    raw["hints"] = {"ports": [{"from_side": "right"}]}
    with pytest.raises(layout.ValidationError, match=r"hint.*index 0.*missing.*edge_id"):
        layout.parse_and_validate(raw)

def test_grid_col_string_rejected():
    raw = base()
    raw["nodes"][0]["grid_col"] = "0"
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_col.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_grid_row_string_rejected():
    raw = base()
    raw["nodes"][0]["grid_row"] = "0"
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_row.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_grid_col_negative_rejected():
    raw = base()
    raw["nodes"][0]["grid_col"] = -1
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_col.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_grid_row_negative_rejected():
    raw = base()
    raw["nodes"][0]["grid_row"] = -1
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_row.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_grid_col_bool_rejected():
    raw = base()
    raw["nodes"][0]["grid_col"] = True
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_col.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_grid_row_bool_rejected():
    raw = base()
    raw["nodes"][0]["grid_row"] = False
    with pytest.raises(layout.ValidationError, match=r"node.*id='a'.*grid_row.*must be.*int.*>= 0"):
        layout.parse_and_validate(raw)

def test_duplicate_hint_ports():
    raw = base()
    raw["hints"] = {"ports": [
        {"edge_id": "e1", "from_side": "right"},
        {"edge_id": "e1", "to_side": "left"}
    ]}
    with pytest.raises(layout.ValidationError, match=r"duplicate.*hint.*edge_id.*e1"):
        layout.parse_and_validate(raw)

def test_duplicate_routing_order_ids():
    raw = base()
    raw["edges"].append({"id": "e2", "from": "b", "to": "a"})
    raw["hints"] = {"routing_order": ["e1", "e2", "e1"]}
    with pytest.raises(layout.ValidationError, match=r"duplicate.*routing_order.*e1"):
        layout.parse_and_validate(raw)

def test_missing_title_key():
    raw = {"description": "D", "nodes": [], "edges": []}
    with pytest.raises(layout.ValidationError, match=r"missing required key.*title"):
        layout.parse_and_validate(raw)

def test_missing_description_key():
    raw = {"title": "T", "nodes": [], "edges": []}
    with pytest.raises(layout.ValidationError, match=r"missing required key.*description"):
        layout.parse_and_validate(raw)

def test_empty_title_allowed():
    raw = {"title": "", "description": "D",
           "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
           "edges": []}
    m = layout.parse_and_validate(raw)
    assert m.title == ""

def test_empty_description_allowed():
    raw = {"title": "T", "description": "",
           "nodes": [{"id": "a", "label": "A", "grid_col": 0, "grid_row": 0}],
           "edges": []}
    m = layout.parse_and_validate(raw)
    assert m.description == ""

def test_hint_side_error_names_edge_and_valid_sides():
    raw = base()
    raw["hints"] = {"ports": [{"edge_id": "e1", "from_side": "sideways"}]}
    with pytest.raises(layout.ValidationError, match=r"hint for edge e1.*invalid side 'sideways'.*must be one of.*left.*right.*top.*bottom"):
        layout.parse_and_validate(raw)

def test_exactly_62_edges_passes():
    raw = {
        "title": "T", "description": "D",
        "nodes": [{"id": f"n{i}", "label": "x", "grid_col": i % 10, "grid_row": i // 10}
                  for i in range(63)],
        "edges": [{"id": f"e{i}", "from": "n0", "to": f"n{i+1}"} for i in range(62)]
    }
    m = layout.parse_and_validate(raw)
    assert len(m.edges) == 62
    assert m.edges[-1].char == layout.EDGE_ALPHABET[61]

def test_unknown_hint_key_rejected():
    raw = base()
    raw["hints"] = {"port": [{"edge_id": "e1"}]}  # typo: "port" instead of "ports"
    with pytest.raises(layout.ValidationError, match=r"unknown key in hints.*port.*allowed.*ports.*routing_order"):
        layout.parse_and_validate(raw)
