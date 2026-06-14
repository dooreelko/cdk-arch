#!/usr/bin/env bash
#
# Manual UAT for the c43 ascii skill.
#
# For each case directory (one with an input.txt) this script:
#   1. creates a fresh /tmp/<case-name> working dir and cd's there
#   2. runs `claude --print` non-interactively with:
#        - PATH pointing at the in-development c43 binary (so the skill's
#          preferred `c43` binary path is exercised, not a stale global one)
#        - --plugin-dir pointing at this c43 plugin
#        - a prompt asking it to use the c43:ascii skill to render the diagram
#          described by the case's input.txt
#   3. reports the files it generated and prints the case's expected.txt so a
#      human can judge the result (this is a manual acceptance test — the LLM
#      authors node placement, so output is not byte-deterministic).
#
# Usage:
#   ./test.sh                 # run every case
#   ./test.sh bvthw-container # run one case by directory name
#
# Env overrides:
#   C43_BIN    path to the c43 binary to test  (default: the release build in
#              this repo: packages/c43/target/release/c43)
#   CLAUDE_BIN claude executable               (default: claude on PATH)
#   KEEP=1     keep an existing /tmp/<case> dir instead of wiping it

set -uo pipefail

# --- locate things relative to this script -----------------------------------
UAT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_DIR="$(cd "$UAT_DIR/.." && pwd -P)"          # claude-plugin/c43
# repo root = up from claude-plugin/c43/uat -> claude-plugin/c43 -> claude-plugin -> repo
REPO_ROOT="$(cd "$PLUGIN_DIR/../.." && pwd -P)"

C43_BIN="${C43_BIN:-$REPO_ROOT/packages/c43/target/release/c43}"
CLAUDE_BIN="${CLAUDE_BIN:-claude}"

# --- preflight ----------------------------------------------------------------
fail_pre() { echo "UAT preflight error: $*" >&2; exit 2; }

[ -x "$C43_BIN" ] || fail_pre "c43 binary not found/executable at: $C43_BIN
  build it first:  cargo build --release --manifest-path packages/c43/Cargo.toml
  or set C43_BIN=/path/to/c43"
command -v "$CLAUDE_BIN" >/dev/null 2>&1 || fail_pre "claude CLI not found (set CLAUDE_BIN=...)"
[ -f "$PLUGIN_DIR/.claude-plugin/plugin.json" ] || fail_pre "plugin.json not found under $PLUGIN_DIR"

# make the in-dev c43 the first thing on PATH for the spawned claude
BIN_SHIM="$(mktemp -d)"
ln -sf "$C43_BIN" "$BIN_SHIM/c43"
trap 'rm -rf "$BIN_SHIM"' EXIT

# --- choose cases -------------------------------------------------------------
if [ "$#" -gt 0 ]; then
  CASES=("$@")
else
  CASES=()
  for d in "$UAT_DIR"/*/; do
    [ -f "${d}input.txt" ] && CASES+=("$(basename "$d")")
  done
fi
[ "${#CASES[@]}" -gt 0 ] || fail_pre "no cases found (a case is a subdir with input.txt)"

echo "c43 binary : $C43_BIN"
echo "plugin dir : $PLUGIN_DIR"
echo "claude     : $(command -v "$CLAUDE_BIN")"
echo "cases      : ${CASES[*]}"
echo

# --- run each case ------------------------------------------------------------
overall=0
for case in "${CASES[@]}"; do
  CASE_DIR="$UAT_DIR/$case"
  INPUT="$CASE_DIR/input.txt"
  EXPECTED="$CASE_DIR/expected.txt"
  if [ ! -f "$INPUT" ]; then
    echo "SKIP $case: no input.txt" >&2
    overall=1
    continue
  fi

  WORK="/tmp/$case"
  if [ "${KEEP:-}" != "1" ]; then rm -rf "$WORK"; fi
  mkdir -p "$WORK"
  cp "$INPUT" "$WORK/input.txt"

  echo "============================================================"
  echo "CASE: $case"
  echo "work: $WORK"
  echo "============================================================"

  read -r -d '' PROMPT <<EOF
Use the c43:ascii skill to generate a c43 ASCII architecture diagram.

The architecture to render is in the file ./input.txt in the current directory
(a c43 container/system tree). Read it, build the layout.json the skill needs
(place nodes on the grid; for the container view, every node that has children
must become a group, producing nested groups), then run the c43 layout engine so
it writes result.txt and result.json into the current directory.

Render in this directory. Do not ask me questions; make reasonable choices and
produce the diagram. When finished, briefly confirm the files you wrote.
EOF

  (
    cd "$WORK" || exit 11
    PATH="$BIN_SHIM:$PATH" "$CLAUDE_BIN" --print \
      --plugin-dir "$PLUGIN_DIR" \
      --permission-mode acceptEdits \
      "$PROMPT"
  )
  rc=$?
  echo
  echo "--- claude exit code: $rc"

  echo "--- generated files in $WORK:"
  ls -la "$WORK" | sed 's/^/    /'

  if [ -f "$WORK/result.json" ]; then
    echo "--- result.json summary:"
    python3 - "$WORK/result.json" <<'PY' 2>/dev/null | sed 's/^/    /' || echo "    (could not parse result.json)"
import json, sys
d = json.load(open(sys.argv[1]))
print("status   :", d.get("status"))
errs = d.get("errors", [])
from collections import Counter
print("errors   :", dict(Counter(e.get("code") for e in errs)) or "none")
print("nodes    :", len(d.get("nodes", [])))
print("edges    :", len(d.get("edges", [])))
groups = d.get("groups", [])
print("groups   :", len(groups))
for g in groups:
    print("           - %-16s parent=%-8s grid=(%s,%s,%s,%s)" % (
        g.get("id"), g.get("parent"),
        g.get("grid", {}).get("col0"), g.get("grid", {}).get("col1"),
        g.get("grid", {}).get("row0"), g.get("grid", {}).get("row1")))
q = d.get("quality", {})
print("quality  :", q)
PY
  else
    echo "--- NO result.json produced (skill did not complete a render)"
    overall=1
  fi

  if [ -f "$WORK/result.txt" ]; then
    echo "--- result.txt (rendered diagram):"
    sed 's/^/    /' "$WORK/result.txt"
  fi

  echo
  echo "--- EXPECTED (judge the above against this):"
  if [ -f "$EXPECTED" ]; then
    sed 's/^/    /' "$EXPECTED"
  else
    echo "    (no expected.txt for this case)"
  fi
  echo
done

echo "============================================================"
echo "Manual UAT complete. Review each case's result.* against its expected.txt."
echo "Working dirs left under /tmp/<case> for inspection."
exit $overall
