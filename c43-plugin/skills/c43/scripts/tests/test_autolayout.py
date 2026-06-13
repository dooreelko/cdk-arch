"""The self-sufficient layout loop: naive in, best-effort clean layout out."""
import copy

import autolayout
import layout

# A small 2x2 graph with a couple of crossings under naive routing -- enough
# to exercise the loop without rebob's ~1s/eval cost.
NODES = [{"id": "a", "label": "a", "grid_col": 0, "grid_row": 0},
         {"id": "b", "label": "b", "grid_col": 1, "grid_row": 0},
         {"id": "c", "label": "c", "grid_col": 0, "grid_row": 1},
         {"id": "d", "label": "d", "grid_col": 1, "grid_row": 1}]
EDGES = [{"id": "e1", "from": "a", "to": "d"},
         {"id": "e2", "from": "c", "to": "b"},
         {"id": "e3", "from": "a", "to": "b"},
         {"id": "e4", "from": "c", "to": "d"}]

def raw(hints=None):
    r = {"title": "T", "description": "D",
         "nodes": copy.deepcopy(NODES), "edges": copy.deepcopy(EDGES)}
    if hints is not None:
        r["hints"] = hints
    return r

def score(raw_in, hints):
    trial = dict(raw_in); trial["hints"] = hints
    return layout.score_key(layout.quality_of(layout.build_model(trial)))


def test_loop_never_returns_worse_than_start():
    r = raw()
    start = score(r, {})
    best_hints, _, _ = autolayout.optimise(r, max_evals=60)
    assert score(r, best_hints) <= start


def test_loop_is_deterministic():
    r = raw()
    h1, q1, n1 = autolayout.optimise(r, max_evals=60)
    h2, q2, n2 = autolayout.optimise(r, max_evals=60)
    assert h1 == h2 and q1 == q2 and n1 == n2


def test_loop_respects_user_pinned_ports():
    # Pin e1's from_side to top; the loop must never change that side.
    r = raw({"ports": [{"edge_id": "e1", "from_side": "top"}]})
    best_hints, _, _ = autolayout.optimise(r, max_evals=60)
    e1 = next(h for h in best_hints["ports"] if h["edge_id"] == "e1")
    assert e1["from_side"] == "top"


def test_loop_never_proposes_inbound_right():
    # Whatever hints the loop settles on must validate (inbound-right is
    # rejected at parse time, so a proposal of it would raise).
    r = raw()
    best_hints, _, _ = autolayout.optimise(r, max_evals=60)
    for h in best_hints.get("ports", []):
        assert h.get("to_side") != "right"
    # and the chosen hints actually build
    trial = dict(r); trial["hints"] = best_hints
    layout.build_model(trial)


def test_loop_improves_or_matches_a_crossing_graph():
    # The naive layout of this graph has crossings; the loop should not make
    # the crossing count worse, and should reach a routable result.
    r = raw()
    naive_q = layout.quality_of(layout.build_model(r))
    best_hints, best_q, _ = autolayout.optimise(r, max_evals=80)
    assert best_q["dropped"] == 0
    assert layout.score_key(best_q) <= layout.score_key(naive_q)
