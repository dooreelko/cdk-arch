import json, os, pytest, layout

def render_to(tmp_path, nodes, edges):
    raw = {"title": "Sys", "description": "desc", "nodes": nodes, "edges": edges}
    m = layout.parse_and_validate(raw)
    layout.geometry(m); layout.assign_ports(m); layout.route_all(m)
    cv = layout.Canvas(m.canvas_w, m.canvas_h)
    out = os.path.join(tmp_path, "result.txt")
    layout.render(m, cv, out)
    with open(out) as f:
        return m, f.read()

TWO = [{"id": "a", "label": "aaa", "grid_col": 0, "grid_row": 0},
       {"id": "b", "label": "bbb", "grid_col": 1, "grid_row": 0}]

def test_canvas_paint_and_str():
    cv = layout.Canvas(5, 2)
    cv.paint(1, 0, "X")
    assert cv.char_at(1, 0) == "X"
    assert "X" in str(cv)

def test_canvas_paint_out_of_bounds_is_noop():
    cv = layout.Canvas(3, 3)
    cv.paint(-1, 0, "X"); cv.paint(0, -1, "X"); cv.paint(3, 0, "X"); cv.paint(0, 3, "X")
    assert "X" not in str(cv)

def test_render_draws_boxes_and_labels(tmp_path):
    m, txt = render_to(str(tmp_path), TWO, [{"id": "e1", "from": "a", "to": "b"}])
    assert "+" in txt and "|" in txt
    assert "aaa" in txt and "bbb" in txt

def test_render_includes_title_and_scaffolding(tmp_path):
    m, txt = render_to(str(tmp_path), TWO, [{"id": "e1", "from": "a", "to": "b"}])
    assert "Sys" in txt and "desc" in txt
    assert "│" in txt and "─" in txt and "┼" in txt
    assert "nodes" in txt and "edges" in txt and "title" in txt

def test_render_paints_edge_char_and_arrowhead(tmp_path):
    m, txt = render_to(str(tmp_path), TWO, [{"id": "e1", "from": "a", "to": "b"}])
    assert m.edges[0].char in txt          # the edge body ('0')
    assert "►" in txt                       # forward edge enters target's left side
    assert "*" in txt                       # source port

def test_edge_body_uses_only_edge_char(tmp_path):
    # the straight horizontal route between a and b must be painted with '0' only
    m, txt = render_to(str(tmp_path), TWO, [{"id": "e1", "from": "a", "to": "b"}])
    e = m.edges[0]
    lines = txt.split("\n")
    for (x0, y0), (x1, y1) in zip(e.route, e.route[1:]):
        if y0 == y1:
            for x in range(min(x0, x1), max(x0, x1) + 1):
                assert lines[y0][x] == e.char

def test_incremental_save_after_each_mutation(tmp_path, monkeypatch):
    saves = []
    orig = layout.Canvas.save
    def spy(self, path):
        saves.append(str(self))
        return orig(self, path)
    monkeypatch.setattr(layout.Canvas, "save", spy)
    render_to(str(tmp_path), TWO, [{"id": "e1", "from": "a", "to": "b"}])
    # scaffolding + 2 boxes + 1 edge => at least 4 saves
    assert len(saves) >= 4
    # progressive: each save's content differs from the previous
    assert all(saves[i] != saves[i+1] for i in range(len(saves)-1))

def test_unrouted_edge_skipped(tmp_path):
    # overflow fixture: narrow boxes, 4 same-column edges -> some have no ports/route
    nodes = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
             {"id": "b", "label": "b", "grid_col": 0, "grid_row": 1}]
    edges = [{"id": f"e{i}", "from": "a", "to": "b"} for i in range(4)]
    m, txt = render_to(str(tmp_path), nodes, edges)   # must not crash
    unrouted = [e for e in m.edges if e.route is None]
    assert unrouted
    # no-crash is the contract: render must skip unrouted edges silently;
    # the scaffolding must still have been painted
    assert "title" in txt

def run_main(tmp_path, raw=None, input_path=None):
    """Run layout.main in tmp_path; returns (result_json, result_txt, exit_code).
    exit_code is 0 when main returned normally, else the SystemExit code."""
    if input_path is None:
        input_path = os.path.join(tmp_path, "layout.json")
        with open(input_path, "w") as f:
            json.dump(raw, f)
    cwd = os.getcwd(); os.chdir(tmp_path)
    code = 0
    try:
        try:
            layout.main(["layout.py", input_path])
        except SystemExit as e:
            code = e.code
    finally:
        os.chdir(cwd)
    rj = os.path.join(tmp_path, "result.json")
    rt = os.path.join(tmp_path, "result.txt")
    data = None
    if os.path.exists(rj):
        with open(rj) as f:
            data = json.load(f)
    txt = None
    if os.path.exists(rt):
        with open(rt) as f:
            txt = f.read()
    return data, txt, code

def test_main_ok_writes_both_files(tmp_path):
    raw = {"title": "S", "description": "d",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
                     {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0}],
           "edges": [{"id": "e1", "from": "a", "to": "b"}]}
    data, txt, code = run_main(str(tmp_path), raw)
    assert code == 0
    assert data["status"] == "ok" and data["errors"] == []
    assert "hints" not in data
    e = data["edges"][0]
    assert e["from"] == "a" and e["to"] == "b" and e["char"] == "0"
    assert e["route"] is not None
    assert e["from_port"]["side"] == "right"
    assert data["canvas"]["width"] > 0 and data["box"]["width"] > 0
    n = data["nodes"][0]
    assert {"id", "label", "grid_col", "grid_row", "x", "y", "w", "h"} <= set(n)
    assert txt is not None

def test_main_validation_error_writes_json_only(tmp_path):
    raw = {"title": "S", "description": "d",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
           "edges": [{"id": "e1", "from": "a", "to": "ghost"}]}
    data, txt, code = run_main(str(tmp_path), raw)
    assert code == 1
    assert data["status"] == "error"
    assert data["errors"][0]["code"] == "validation"
    assert data["errors"][0]["message"]
    assert txt is None

def test_main_routing_error_still_renders(tmp_path):
    # K5: 5 nodes fully connected on a 3x2 grid forces desperation crossings
    nodes = [{"id": f"n{i}", "label": f"n{i}",
              "grid_col": i % 3, "grid_row": i // 3} for i in range(5)]
    edges = [{"id": f"e{i}{j}", "from": f"n{i}", "to": f"n{j}"}
             for i in range(5) for j in range(i + 1, 5)]
    raw = {"title": "K5", "description": "crossing test",
           "nodes": nodes, "edges": edges}
    data, txt, code = run_main(str(tmp_path), raw)
    assert code == 1
    assert data["status"] == "error"
    assert any(err["code"] == "crossing" for err in data["errors"])
    assert txt is not None          # rendered despite errors

def test_main_missing_arg_exits_2():
    with pytest.raises(SystemExit) as ei:
        layout.main(["layout.py"])
    assert ei.value.code == 2

FULL_KEYS = {"status", "errors", "title", "description",
             "canvas", "box", "nodes", "edges"}

def assert_validation_error_result(data):
    assert data["status"] == "error"
    assert data["errors"][0]["code"] == "validation"
    assert data["errors"][0]["message"]
    assert FULL_KEYS <= set(data)

def test_main_malformed_json_writes_error_result(tmp_path):
    p = os.path.join(str(tmp_path), "layout.json")
    with open(p, "w") as f:
        f.write("{not valid json")
    data, txt, code = run_main(str(tmp_path), input_path=p)
    assert code == 1
    assert_validation_error_result(data)
    assert txt is None

def test_main_missing_input_file_writes_error_result(tmp_path):
    p = os.path.join(str(tmp_path), "does_not_exist.json")
    data, txt, code = run_main(str(tmp_path), input_path=p)
    assert code == 1
    assert_validation_error_result(data)
    assert txt is None

def test_main_non_dict_top_level_writes_error_result(tmp_path):
    data, txt, code = run_main(str(tmp_path), raw=[1, 2])
    assert code == 1
    assert_validation_error_result(data)
    assert "JSON object" in data["errors"][0]["message"]
    assert txt is None

def test_main_removes_stale_outputs(tmp_path):
    # pre-existing outputs from a previous run must never survive a new run
    with open(os.path.join(str(tmp_path), "result.json"), "w") as f:
        json.dump({"status": "ok"}, f)
    with open(os.path.join(str(tmp_path), "result.txt"), "w") as f:
        f.write("stale diagram\n")
    p = os.path.join(str(tmp_path), "layout.json")
    with open(p, "w") as f:
        f.write("{broken")
    data, txt, code = run_main(str(tmp_path), input_path=p)
    assert txt is None                       # stale result.txt removed
    assert data["status"] == "error"         # result.json reflects new run
    assert code == 1

def test_main_validation_error_has_full_key_set(tmp_path):
    raw = {"title": "S", "description": "d",
           "nodes": [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0}],
           "edges": [{"id": "e1", "from": "a", "to": "ghost"}]}
    data, _txt, _code = run_main(str(tmp_path), raw)
    assert FULL_KEYS <= set(data)
    assert data["canvas"] == {"width": 0, "height": 0}
    assert data["box"] == {"width": 0, "height": 0}
    assert data["nodes"] == [] and data["edges"] == []
    assert data["title"] == "S" and data["description"] == "d"
