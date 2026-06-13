#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY="$HERE/../../c43-plugin/skills/c43/scripts"
BIN="$HERE/target/release/c43"
cargo build --release --manifest-path "$HERE/Cargo.toml" >/dev/null

check() {  # $1 = layout.json path, $2 = "" or "--auto"
  local fixture="$1" mode="$2"
  local pd rd; pd="$(mktemp -d)"; rd="$(mktemp -d)"
  cp "$fixture" "$pd/layout.json"; cp "$fixture" "$rd/layout.json"
  if [ "$mode" = "--auto" ]; then
    ( cd "$pd" && uv run "$PY/autolayout.py" layout.json >/dev/null 2>&1 ) || true
  else
    ( cd "$pd" && uv run "$PY/layout.py" layout.json >/dev/null 2>&1 ) || true
  fi
  ( cd "$rd" && "$BIN" layout layout.json $mode >/dev/null 2>&1 ) || true
  for f in result.txt result.json; do
    if ! diff -u "$pd/$f" "$rd/$f"; then
      echo "PARITY FAIL: $fixture [$mode] $f"; exit 1
    fi
  done
  echo "ok: $fixture [$mode]"
}

for fx in "$HERE"/tests/fixtures/*.json; do
  check "$fx" ""
  check "$fx" "--auto"
done
echo "ALL PARITY CHECKS PASSED"
