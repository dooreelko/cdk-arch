import esbuild from 'esbuild';
import { fileURLToPath } from 'url';
import path from 'path';

const servers = [
  { entry: 'src/entrypoints/api-server.ts', out: 'api-server.js' },
  { entry: 'src/entrypoints/jsonstore-server.ts', out: 'jsonstore-server.js' }
];

const __dirname = path.dirname(fileURLToPath(import.meta.url));

async function bundle() {
  const outdir = path.resolve(__dirname, '../dist/docker');

  for (const server of servers) {
    await esbuild.build({
      entryPoints: [path.resolve(__dirname, '..', server.entry)],
      bundle: true,
      outfile: path.join(outdir, server.out),
      format: 'esm',
      platform: 'node',
      target: 'node20',
      minify: false,
      sourcemap: false,
      mainFields: ['module', 'main'],
      external: ['pg-native']
    });
    console.log(`Bundled ${server.entry} -> dist/docker/${server.out}`);
  }
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
