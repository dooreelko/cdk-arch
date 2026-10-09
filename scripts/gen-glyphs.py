#!/usr/bin/env python3
"""Regenerates packages/clarc/src/glyphs/{aws.tsv,azure.txt} from a drawio source tree (dev aid).
Usage: scripts/gen-glyphs.py <drawio>/src/main/webapp
aws.tsv:   name <TAB> r|s <TAB> fill colour   (r: resource icon, s: standalone shape), from Sidebar-AWS4.js
azure.txt: <category>/<Name>                  (img/lib/azure2/<category>/<Name>.svg)"""
import os, re, sys

root = sys.argv[1]
out = os.path.join(os.path.dirname(__file__), "..", "packages", "clarc", "src", "glyphs")
src = open(os.path.join(root, "js/diagramly/sidebar/Sidebar-AWS4.js"), encoding="utf-8").read()

skip = {"Arrows", "Groups", "Illustrations"}
entry = re.compile(r"createVertexTemplateEntry\((n\d*) \+ ('[^']*'(?: \+ gn \+ '[^']*')?)")
found = {}
parts = re.split(r"Sidebar\.prototype\.addAWS4(\w+)Palette = function", src)
for pal, body in zip(parts[1::2], parts[2::2]):
    if pal in skip:
        continue
    colors = {m.group(1): (re.search(r"fillColor=(#[0-9A-Fa-f]{6})", m.group(2)) or [None, "#232F3E"])[1]
              for m in re.finditer(r"var (n\d*) = ([^;]*(?:;[^;\n]*)*?)\+ mxConstants", body)}
    for m in entry.finditer(body):
        lit = m.group(2).replace("' + gn + '", "mxgraph.aws4").strip("'")
        r = re.match(r"resourceIcon;resIcon=mxgraph\.aws4\.(\w+);", lit)
        if r:
            name, kind = r.group(1), "r"
        else:
            name, kind = lit.split(";")[0], "s"
            if not re.fullmatch(r"\w+", name):
                continue
        color = colors.get(m.group(1), "#232F3E")
        # a resource icon wins over a standalone shape of the same name
        if name not in found or (kind == "r" and found[name][0] == "s"):
            found[name] = (kind, color)
with open(os.path.join(out, "aws.tsv"), "w") as f:
    f.write("".join(f"{n}\t{k}\t{c}\n" for n, (k, c) in sorted(found.items())))

az = os.path.join(root, "img/lib/azure2")
names = sorted(
    f"{d}/{f[:-4]}" for d in os.listdir(az) if d != "menu" and os.path.isdir(os.path.join(az, d))
    for f in os.listdir(os.path.join(az, d)) if f.endswith(".svg"))
open(os.path.join(out, "azure.txt"), "w").write("\n".join(names) + "\n")
print(len(found), "aws,", len(names), "azure")
