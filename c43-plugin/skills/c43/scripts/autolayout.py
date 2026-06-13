"""Self-sufficient layout loop: naive start -> iterate on engine feedback ->
settle on the best layout it can reach within an eval budget.

The skill can run this instead of hand-iterating: it reads a layout.json that
needs only nodes+edges (hints optional), repeatedly re-routes while applying
the cheapest defect-fixing port change, and writes the best result.txt /
result.json it found. It NEVER discards a user-supplied hint -- it only adds or
changes port sides the user left unspecified, and only when that strictly
improves the score. It is fully deterministic (fixed candidate order, no clock
or RNG), so reruns and tests are byte-stable.

Usage:
    uv run --native-tls autolayout.py layout.json [--max-evals N]

Exit codes match layout.py: 0 = clean (status ok), 1 = best attempt still has
errors, 2 = usage / unreadable input.
"""
import copy
import json
import sys

import layout

# Inbound 'right' is prohibited by the engine, so the loop never proposes it.
OUT_SIDES = ("left", "right", "top", "bottom")
IN_SIDES = ("left", "top", "bottom")
DEFAULT_MAX_EVALS = 200


def _hint_index(hints):
    """Map edge_id -> its ports-hint dict, for quick lookup/merge."""
    return {h["edge_id"]: h for h in hints.get("ports", [])}


def _with_port_override(hints, edge_id, end, side):
    """Return a deep-copied hints dict with `edge_id`'s `end` side set to
    `side`. Preserves every existing entry (including the user's) and only
    overwrites the one side being proposed."""
    new = copy.deepcopy(hints)
    new.setdefault("ports", [])
    idx = _hint_index(new)
    key = "from_side" if end == "from" else "to_side"
    if edge_id in idx:
        idx[edge_id][key] = side
    else:
        new["ports"].append({"edge_id": edge_id, key: side})
    return new


def _user_pinned(user_hints):
    """Set of (edge_id, end) the user explicitly fixed -- the loop leaves
    these untouched so it can never override an intentional choice."""
    pinned = set()
    for h in user_hints.get("ports", []):
        if "from_side" in h:
            pinned.add((h["edge_id"], "from"))
        if "to_side" in h:
            pinned.add((h["edge_id"], "to"))
    return pinned


def _eval(raw, hints):
    """Route `raw` under `hints`; return (score_key, quality, flagged_edges,
    model) or None if the hints are invalid (e.g. inbound-right slipped in).
    flagged_edges lists edges the engine complained about -- both hard errors
    (crossing/unroutable) and soft diagnostics (wrap/congestion) -- in a
    deterministic order, so the loop knows which ports are worth perturbing."""
    trial = dict(raw)
    trial["hints"] = hints
    try:
        m = layout.build_model(trial)
    except layout.ValidationError:
        return None
    q, diags = layout._quality_and_diagnostics(m)
    seen, flagged = set(), []
    # hard errors first (they dominate the score), then soft diagnostics
    for ids in ([e.edge_ids for e in m.errors] + [d["edge_ids"] for d in diags]):
        for eid in ids:
            if eid not in seen:
                seen.add(eid)
                flagged.append(eid)
    return layout.score_key(q), q, flagged, m


def optimise(raw, max_evals=DEFAULT_MAX_EVALS):
    """Best-improvement, defect-driven hill-climb. Starts from the user's
    hints (or the engine defaults when none). Each round generates every
    candidate single-change (a port side on a flagged edge's unpinned end, or
    promoting a wrapping edge to the front of routing_order), evaluates them
    all, and commits the single best strict improvement. Best-improvement
    rather than first-improvement avoids the trap where an early small win
    blocks a later larger one. Stops when perfect, when a round yields no
    improvement (local optimum), or when the eval budget runs out. The
    returned hints are always the best-scoring layout seen -- never a
    regression. Returns (best_hints, best_quality, evals_used)."""
    user_hints = raw.get("hints", {}) or {}
    pinned = _user_pinned(user_hints)

    best_hints = copy.deepcopy(user_hints)
    cur = _eval(raw, best_hints)
    evals = 1
    if cur is None:                       # user's own hints are invalid
        return best_hints, None, evals
    best_key, best_q, best_flagged, best_m = cur

    # A perfect score (no dropped/wraps/crossings/top-ports/congestion) is the
    # only early exit; otherwise improve until a local optimum or budget end.
    # The final term (length) is never zero, so only the first five must clear.
    def perfect(k):
        return k[0:5] == (0, 0, 0, 0, 0)

    def candidates(flagged, m):
        """Deterministic list of (hints, label) single-change neighbours of the
        current best, derived from what the engine flagged."""
        out = []
        # 1. port-side changes on flagged edges' unpinned ends
        for eid in flagged:
            for end, sides in (("to", IN_SIDES), ("from", OUT_SIDES)):
                if (eid, end) in pinned:
                    continue
                for side in sides:
                    out.append(_with_port_override(best_hints, eid, end, side))
        # 2. routing-order promotion for wrapping edges -- giving a long edge
        #    first pick of the lanes is the standard wrap fix
        wrap_ids = [e.id for e in m.errors if e.code == "unroutable"]
        wrap_ids += [d_eid for d_eid in flagged]   # flagged already wrap-first
        existing_order = best_hints.get("routing_order", [])
        for eid in wrap_ids:
            if existing_order[:1] == [eid]:
                continue
            new = copy.deepcopy(best_hints)
            new["routing_order"] = [eid] + [x for x in existing_order if x != eid]
            out.append(new)
        return out

    while evals < max_evals and not perfect(best_key):
        round_best = None
        for trial_hints in candidates(best_flagged, best_m):
            if evals >= max_evals:
                break
            res = _eval(raw, trial_hints)
            evals += 1
            if res is None:
                continue
            k, q, flagged, m = res
            if k < best_key and (round_best is None or k < round_best[0]):
                round_best = (k, q, flagged, m, trial_hints)
        if round_best is None:
            break                          # local optimum
        best_key, best_q, best_flagged, best_m, best_hints = round_best

    return best_hints, best_q, evals


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    max_evals = DEFAULT_MAX_EVALS
    for a in argv[1:]:
        if a.startswith("--max-evals="):
            max_evals = int(a.split("=", 1)[1])
    if not args:
        print("usage: autolayout.py layout.json [--max-evals=N]", file=sys.stderr)
        sys.exit(2)

    for stale in ("result.json", "result.txt"):
        try:
            import os
            os.remove(stale)
        except FileNotFoundError:
            pass

    try:
        with open(args[0], encoding="utf-8") as f:
            raw = json.load(f)
    except (OSError, json.JSONDecodeError) as exc:
        layout._write_json("result.json", layout._validation_error_result(
            None, str(exc),
            "ensure the layout.json path is correct and the file is valid JSON"))
        sys.exit(1)

    if not isinstance(raw, dict):
        layout._write_json("result.json", layout._validation_error_result(
            raw, "layout.json top level must be a JSON object",
            "ensure the layout.json path is correct and the file is valid JSON"))
        sys.exit(1)

    # Validate up front so bad input fails the same way layout.py does.
    try:
        layout.parse_and_validate(raw)
    except layout.ValidationError as exc:
        layout._write_json("result.json", layout._validation_error_result(
            raw, str(exc), "fix layout.json per the message above"))
        sys.exit(1)

    best_hints, best_q, evals = optimise(raw, max_evals)

    # Re-run the engine once more under the chosen hints and write the
    # canonical result.txt / result.json exactly as layout.py would, so the
    # two scripts produce identical artifacts for the same effective input.
    final = dict(raw)
    final["hints"] = best_hints
    m = layout.build_model(final)
    cv = layout.Canvas(m.canvas_w, m.canvas_h)
    layout.render(m, cv, "result.txt")
    result = layout._result_json(m)
    result["auto"] = {"evals": evals, "hints": best_hints}
    layout._write_json("result.json", result)

    q = result["quality"]
    print(f"autolayout: {evals} evals, "
          f"crossings={q['crossings']} wraps={q['wraps']} "
          f"top_ports={q['top_ports']} congestion={q['congestion']}",
          file=sys.stderr)
    sys.exit(1 if result["status"] == "error" else 0)


if __name__ == "__main__":
    main(sys.argv)
