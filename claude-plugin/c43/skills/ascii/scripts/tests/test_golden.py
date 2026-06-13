"""Golden render of the real rebob diagram (repo-root layout.json).

Pins the end-to-end output -- geometry constants, port assignment, routing
and rendering all feed into this byte-for-byte comparison.  The rebob graph
is dense (frontend fans out 4 edges, memory takes 6 in-edges); the tuned
hints in layout.json get it down to a single known crossing
(bootstrap_to_memory x e6), which this test also pins so a routing change
that silently adds crossings fails loudly.

If a deliberate engine or layout.json change shifts the render: re-run,
visually re-approve the new result.txt, and refresh expected_rebob.txt.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(os.path.dirname(HERE), "layout.py")
# Canonical rebob inputs live with the Rust engine fixtures (the parity oracle
# source of truth); both implementations render them byte-identically.
RUST_FIXTURES = os.path.abspath(
    os.path.join(HERE, *[os.pardir] * 6, "packages", "c43", "tests", "fixtures"))
LAYOUT = os.path.join(RUST_FIXTURES, "rebob_layout.json")
EXPECTED = os.path.join(HERE, "expected_rebob.txt")
GROUPS_LAYOUT = os.path.join(RUST_FIXTURES, "rebob_groups_layout.json")
GROUPS_EXPECTED = os.path.join(HERE, "expected_rebob_groups.txt")


def _run(tmp_path, layout=LAYOUT):
    return subprocess.run([sys.executable, SCRIPT, layout], cwd=tmp_path,
                          capture_output=True, text=True)


def test_rebob_render_matches_golden(tmp_path):
    proc = _run(tmp_path)
    out = os.path.join(tmp_path, "result.txt")
    assert os.path.exists(out), proc.stderr
    with open(out, encoding="utf-8") as f:
        got = f.read()
    with open(EXPECTED, encoding="utf-8") as f:
        expected = f.read()
    assert got == expected, (
        "rebob render drifted from golden; inspect result.txt, re-approve "
        "visually, then update expected_rebob.txt")


def test_rebob_errors_are_exactly_the_known_crossing(tmp_path):
    proc = _run(tmp_path)
    with open(os.path.join(tmp_path, "result.json"), encoding="utf-8") as f:
        result = json.load(f)
    assert result["status"] == "error"
    assert proc.returncode == 1
    pairs = [sorted(e["edge_ids"]) for e in result["errors"]]
    assert pairs == [["e8", "fe_to_memory"]], pairs
    assert all(e["code"] == "crossing" for e in result["errors"])
    # quality scorecard reflects the known single crossing and no worse defects
    q = result["quality"]
    assert q["dropped"] == 0 and q["wraps"] == 0 and q["top_ports"] == 0
    assert q["crossings"] == 1
    # the crossing edge hugs another only at the crossing itself, not the whole
    # way (pass-2 spacing is a soft cost, not disabled): congestion stays tiny
    assert q["congestion"] <= 10, q["congestion"]


def test_rebob_groups_render_matches_golden(tmp_path):
    """Byte-parity oracle: the Python reference renders the nested-group rebob
    fixture identically to the Rust engine's committed golden."""
    proc = _run(tmp_path, GROUPS_LAYOUT)
    out = os.path.join(tmp_path, "result.txt")
    assert os.path.exists(out), proc.stderr
    with open(out, encoding="utf-8") as f:
        got = f.read()
    with open(GROUPS_EXPECTED, encoding="utf-8") as f:
        expected = f.read()
    assert got == expected, (
        "rebob groups render drifted from golden; inspect result.txt, "
        "re-approve visually, then update expected_rebob_groups.txt")
