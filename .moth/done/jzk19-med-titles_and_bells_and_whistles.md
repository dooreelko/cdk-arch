additions

- a diagram should have surrounding group/frame
- top right should contain diagram's title and description. title bold, both twice the size of the node's label
- node's size should be enough to hold label with spacing around it (in rebob dispatcher good, @bob/frontend bad)
- node square's line should be thicker than connections/edges


----- AI agent updates -------

## Decisions

- Title corner: top LEFT, not top right. The task text's "top right" is a typo — IDEA.md reserves the top-left for title/meta.
- Room for the title: always a header band above the grid, inside the frame, holding title then description. Rejected: fitting the title into empty top-left cells with a band only as fallback (layout-dependent, harder to predict); confining it to empty cells (would truncate/cramp long titles).
- The header band is now THE top-left title/meta place, so the placement rule "keep the top-left grid cell empty" (from ehfb0) is dropped entirely — also for diagrams without a title. Reason: with the band, the reserved cell only produced a wasted empty top row under the title (rebob). The "top-left empty" soft score goes with it. Rejected: keeping the rule always; dropping it only when a header exists.
- Title/description source: optional top-level `title` and `description` in the input graph (rebob.json already has them). Validated as strings. Neither given → no header band; the frame is still drawn.
- Title bold, title and description font twice the node label font. Header text is wrapped by the renderer (drawio text component), not by the engine: the engine passes the raw text and a box of the frame width, sized from an estimated line count (the frame widens only if one word cannot fit). Rejected: engine-inserted line breaks in title/description — the estimate is conservative, so lines ended well short of the frame (ragged, wasted width).
- Frame: one rectangle around the whole diagram (header band + grid incl. its outer lanes), unfilled, drawn behind everything. Edges in outer lanes stay inside it.
- Padding outside the frame: exports keep clear space around the frame. Found: drawio crops exports (PNG etc.) to content, which put the frame line on the image edge, half clipped and effectively invisible. Handled in the drawio export itself (invisible margin around the frame) so every drawio export — CLI or app — keeps it, not only the dev render script.
- Node size also fits the label: node side = max(port-driven side, smallest square that holds the wrapped label with padding on all sides). Still one size for all nodes (IDEA equal squares), so one long label grows every node.
- Label wrapping: labels may break at spaces and after `-` `/` `_` `.`; a single unbreakable piece must fit on one line (node grows). The engine decides the line breaks and emits them, so exporters do not re-wrap (drawio would otherwise wrap differently than the size was computed for).
- Text width is estimated from character classes and font size (no font metrics dependency); conservative so labels fit in drawio's default font.
- Node outline thicker than edges (edges keep default width).
- Consequence accepted: aws node side grows from 3 to 5 at the default unit size because labels such as "CloudWatch" need more room than 3 port slots give.

## Implementation (abstract)

- A small text module estimates text width, breaks text into lines at allowed break points, and finds the smallest square side (in grid units) holding every label with padding.
- The engine computes that label side once and uses the larger of it and the port-driven side for all nodes; each laid-out node carries its label lines.
- The layout additionally carries the frame rectangle, the optional header band above the grid with title/description as raw text in positioned boxes, and font sizes; grid coordinates are unchanged (the band sits above them).
- drawio export: invisible margin, frame, title/description text cells (drawio wraps them), then nodes (thick outline, engine line breaks, no drawio wrapping), then edges.

## Results

- aws: 6×4, node side 5, 0 violations, 0 crossings. The aws case now carries a title and a long description as the header stress sample (6 wrapped lines at full frame width).
- star16: 5×6, node side 10 (port-driven), 0 violations, 0 crossings.
- rebob: 4×6 (was 4×7 with the reserved top row), node side 5, 1 crossing (pre-existing, unchanged); title + description rendered, labels fit.

## Findings

- Visible gap between header band and first row equals the outer lane width, which is the global (equal) lane width — wide in rebob (5) because of its busiest lane. Left as is (IDEA: all lanes equal width).


