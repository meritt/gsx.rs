import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

const [entry, runs = '30'] = process.argv.slice(2);

if (!entry) {
  console.error('usage: startup.mjs <gsx.js> [runs]');
  process.exit(2);
}

const url = pathToFileURL(resolve(entry)).href;

if (process.env.GSX_STARTUP_CHILD) {
  const started = performance.now();
  const { load } = await import(url);
  const imported = performance.now();
  const gsx = await load();
  const loaded = performance.now();
  gsx.parseJSON('# a *b*');
  const parsed = performance.now();
  gsx.parseJSON('[c](https://d.example)');
  const linked = performance.now();
  console.log(
    JSON.stringify({ import: imported - started, load: loaded - imported, first: parsed - loaded, link: linked - parsed })
  );
} else {
  const samples = [];

  for (let i = 0; i < Number(runs); i++) {
    const out = execFileSync(process.execPath, [fileURLToPath(import.meta.url), entry], {
      env: { ...process.env, GSX_STARTUP_CHILD: '1' },
      encoding: 'utf8'
    });
    samples.push(JSON.parse(out));
  }

  const median = (key) => {
    const values = samples.map((s) => s[key]).sort((a, b) => a - b);
    return values[values.length >> 1];
  };
  console.log(
    JSON.stringify({
      node: process.version,
      runs: samples.length,
      wasm: readFileSync(new URL('gsx.wasm', url)).length,
      medianMs: {
        import: median('import'),
        load: median('load'),
        firstParse: median('first'),
        firstLink: median('link')
      }
    })
  );
}
