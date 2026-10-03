#!/usr/bin/env bash
# Render generated drawio files to PNG and SVG with the drawio CLI (dev aid only).
# Usage: scripts/render.sh [file.drawio | dir ...]   (default: the generated golden cases)
# Output: out/<name>.{png,svg}; <name> = case dir for */expected.drawio, else the file's basename
set -euo pipefail
cd "$(dirname "$0")/.."
command -v drawio >/dev/null || { echo "drawio CLI not found (nix-shell provides drawio-headless)" >&2; exit 2; }
mkdir -p out

targets=("$@")
[ ${#targets[@]} -gt 0 ] || targets=(packages/c43-layout/tests/cases)
mapfile -t files < <(find "${targets[@]}" -name '*.drawio' -not -path '*/node_modules/*' -not -path './target/*' -not -path './out/*' | sort)
[ ${#files[@]} -gt 0 ] || { echo "no drawio files found" >&2; exit 1; }

for f in "${files[@]}"; do
  name=$(basename "$f" .drawio)
  [ "$name" != expected ] || name=$(basename "$(dirname "$f")")
  # very large diagrams come out blank or fail at full scale: cap the longer side at ~4000 px
  # (the margin cell spans the whole drawing)
  px=$({ grep -o '<mxCell id="margin"[^>]*><mxGeometry [^>]*' "$f" || true; } | sed -E 's/.*width="([0-9.]+)" height="([0-9.]+)".*/\1 \2/' | awk '{print ($1 > $2 ? $1 : $2)}')
  scale=$(awk -v px="${px:-0}" 'BEGIN { print (px > 4000 ? 4000 / px : 1) }')
  echo "$f -> out/$name.{png,svg} (scale $scale)"
  drawio -x -f png -s "$scale" -o "out/$name.png" "$f" --no-sandbox >/dev/null 2>&1 \
    || echo "  png render failed for $f" >&2
  # full-size vector copy: zoomable, searchable text; light theme so dark-mode viewers do not invert it
  drawio -x -f svg --svg-theme light -o "out/$name.svg" "$f" --no-sandbox >/dev/null 2>&1 \
    || echo "  svg render failed for $f" >&2
done
