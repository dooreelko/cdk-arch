import commonjsPlugin from '@chialab/esbuild-plugin-commonjs';
import esbuild from 'esbuild';
import { fileURLToPath } from 'url';
import path from 'path';

const workers = [
  { entry: 'src/entrypoints/api-worker.ts', out: 'api-worker.js' },
  { entry: 'src/entrypoints/jsonstore-worker.ts', out: 'jsonstore-worker.js' }
];

const __dirname = path.dirname(fileURLToPath(import.meta.url));

async function bundle() {
  const outdir = path.resolve(__dirname, '../dist/cloudflare');

  for (const worker of workers) {
    await esbuild.build({
      entryPoints: [path.resolve(__dirname, '..', worker.entry)],
      bundle: true,
      outfile: path.join(outdir, worker.out),
      format: 'esm',
      platform: 'browser',
      target: 'esnext',
      minify: false,
      sourcemap: false,
      mainFields: ['module', 'main'],
      conditions: ['worker', 'browser', 'import', 'default'],
      external: ['crypto'],
      plugins: [
        // {
        //   name: "rewrite-node-to-internal",
        //   setup(build) {
        //     build.onResolve({ filter: /^crypto$/ }, async (args) => {
        //       const module = args.path.substring("node:".length);
        //       return { path: `node:crypto`, external: true };
        //     });
        //   },
        // },
        commonjsPlugin()
      ]
    });
    console.log(`Bundled ${worker.entry} -> dist/cloudflare/${worker.out}`);
  }
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
