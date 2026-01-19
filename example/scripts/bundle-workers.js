const esbuild = require('esbuild');
const path = require('path');

const workers = [
  { entry: 'src/cloudflare/entrypoints/api-worker.ts', out: 'api-worker.js' },
  { entry: 'src/cloudflare/entrypoints/jsonstore-worker.ts', out: 'jsonstore-worker.js' }
];

async function bundle() {
  const outdir = path.resolve(__dirname, '../dist/cloudflare');

  for (const worker of workers) {
    await esbuild.build({
      entryPoints: [path.resolve(__dirname, '..', worker.entry)],
      bundle: true,
      outfile: path.join(outdir, worker.out),
      format: 'esm',
      platform: 'browser',
      target: 'es2022',
      minify: false,
      sourcemap: false,
      mainFields: ['module', 'main'],
      conditions: ['worker', 'browser', 'import', 'default'],
    });
    console.log(`Bundled ${worker.entry} -> dist/cloudflare/${worker.out}`);
  }
}

bundle().catch((err) => {
  console.error('Bundle failed:', err);
  process.exit(1);
});
