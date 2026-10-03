// One-off generator of the c43-layout golden fixtures from the TypeScript engine it was translated from.
// Ran against c43-router@48b6428:  node packages/c43-layout/tests/gen/gen-goldens.ts <path to c43-router>
// Per case: graph.json + hints.json (engine input), layout.json (engine output), expected.drawio (c43 renderer output).
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const router = resolve(process.argv[2] ?? '');
if (!process.argv[2]) {
  console.error('usage: node gen-goldens.ts <c43-router dir>');
  process.exit(2);
}
const { layout } = await import(join(router, 'src/engine.ts'));
const { normalize } = await import(join(router, 'src/normalize.ts'));
const { toDrawio } = await import(join(router, 'src/export/drawio.ts'));
const { LABEL_PX, LINE_H, labelSide, textWidth } = await import(join(router, 'src/text.ts'));

const UNIT_PX = 20;
const out = join(import.meta.dirname, '..', 'cases');
const json = (x: unknown) => JSON.stringify(x, null, 2) + '\n';
const meta = (input: { title?: string; description?: string }) => ({
  ...(input.title !== undefined ? { title: input.title } : {}),
  ...(input.description !== undefined ? { description: input.description } : {}),
});

const cases: [string, string][] = [
  ['aws', 'aws'], ['star16', 'star16'], ['groups', 'groups'], ['rebob', 'rebob-system'], ['container', 'rebob-container'],
];

cases.forEach(([src, name]) => {
  const input = JSON.parse(readFileSync(join(router, 'cases', `${src}.json`), 'utf8'));
  const g = normalize(input);
  const graph = {
    ...meta(input),
    groups: (input.groups ?? []).map(({ id, label, group }: Record<string, string>) => ({ id, label, group })),
    nodes: input.nodes.map(({ id, label, group }: Record<string, string>) => ({ id, label, group })),
    edges: (input.edges ?? []).map(({ from, to }: Record<string, string>) => ({ from, to })),
  };
  const hints = {
    kinds: g.edges.filter((e: { kind: string }) => e.kind === 'nf').map(({ from, to, kind }: Record<string, string>) => ({ from, to, kind })),
    sizes: {
      node: Object.fromEntries(g.nodes.map((n: { id: string; label: string }) => [n.id, labelSide([n.label], UNIT_PX)])),
      groupTitle: Object.fromEntries(g.groups.map((x: { id: string; label: string }) =>
        [x.id, (textWidth(x.label, LABEL_PX, true) + LABEL_PX / 2) / UNIT_PX])),
      titleHeight: Math.ceil((LABEL_PX * LINE_H) / UNIT_PX),
    },
  };
  const t0 = performance.now();
  const res = layout(input);
  const { unitPx, fonts, header, frame, ...rest } = res.layout;
  const end = (ws: number[], n: number) => ws.reduce((s, w) => s + w, 0) + n * rest.S;
  const engine = {
    version: 1,
    layout: {
      ...rest,
      ...meta(input),
      frame: { x: 0, y: 0, w: end(rest.lanes.v, rest.cols), h: end(rest.lanes.h, rest.rows) },
      nodes: rest.nodes.map(({ lines, ...n }: { lines: string[] }) => n),
    },
    metrics: res.metrics,
  };
  const dir = join(out, name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'graph.json'), json(graph));
  writeFileSync(join(dir, 'hints.json'), json(hints));
  writeFileSync(join(dir, 'layout.json'), json(engine));
  writeFileSync(join(dir, 'expected.drawio'), toDrawio(res.layout));
  console.log(`${name}: ${(performance.now() - t0).toFixed(0)} ms, frame ${JSON.stringify(frame)} grid ${JSON.stringify(engine.layout.frame)}`);
});
