import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const repo = resolve(import.meta.dirname, '../../..');
const {
  values: args,
  positionals: [glue, out]
} = parseArgs({
  allowPositionals: true,
  options: {
    budget: { type: 'string', default: '500' },
    'child-memory': { type: 'string' },
    'child-cold': { type: 'boolean' }
  }
});

function expand(source, blocks) {
  if (typeof source === 'string') {
    return source;
  }
  if (source.block !== undefined) {
    return blocks[source.block];
  }
  if (source.concat !== undefined) {
    return source.concat.map((part) => expand(part, blocks)).join('');
  }
  if (source.ramp !== undefined) {
    const { pre, step, from } = source.ramp;
    return Array.from({ length: source.times }, (_, i) => pre + step.repeat(from + i)).join('');
  }
  const unit = expand(source.repeat, blocks);
  if (source.times !== undefined) {
    return unit.repeat(source.times);
  }
  return unit.repeat(Math.max(1, Math.floor(source.until / unit.length)));
}

function loadCases() {
  const spec = JSON.parse(readFileSync(join(repo, 'testdata/bench/cases.json'), 'utf8'));
  const typical = readFileSync(join(repo, 'testdata/bench/typical.jsonl'), 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line).markdown);
  const cases = spec.cases.map(({ id, group, source }) => ({ id, group, markdown: expand(source, spec.blocks) }));
  return { typical, cases };
}

async function loadParse() {
  const { load } = await import(pathToFileURL(resolve(glue)).href);
  const module = await load();
  return (markdown) => module.parse(markdown);
}

function stats(samples) {
  const sorted = samples.toSorted((a, b) => a - b);
  const at = (q) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
  const round = (value) => Math.round(value * 1000) / 1000;
  return { median: round(at(0.5)), p95: round(at(0.95)), min: round(sorted[0]), samples: samples.length };
}

function measure(fn, budget) {
  fn();
  globalThis.gc?.();
  const samples = [];
  const started = performance.now();
  while (samples.length < 3 || (performance.now() - started < budget && samples.length < 100)) {
    const start = performance.now();
    fn();
    samples.push(performance.now() - start);
  }
  return stats(samples);
}

function outcome(parse, markdown) {
  try {
    parse(markdown);
    return 'accepted';
  } catch (error) {
    return typeof error?.code === 'string' ? error.code : (error?.name ?? 'Error');
  }
}

async function childMemory(id) {
  const { typical, cases } = loadCases();
  const parse = await loadParse();
  const markdowns = id === 'typical' ? typical : [cases.find((c) => c.id === id).markdown];
  globalThis.gc?.();
  const before = process.resourceUsage().maxRSS;
  const outcomes = markdowns.map((markdown) => outcome(parse, markdown));
  const after = process.resourceUsage().maxRSS;
  const rejected = outcomes.find((value) => value !== 'accepted');
  process.stdout.write(JSON.stringify({ outcome: rejected ?? 'accepted', maxRssKiB: after, deltaKiB: after - before }));
}

async function childCold() {
  const start = performance.now();
  const parse = await loadParse();
  const loaded = performance.now();
  parse('**Hello**, [world](https://example.com)!');
  const first = performance.now();
  process.stdout.write(JSON.stringify({ importMs: loaded - start, firstParseMs: first - loaded }));
}

function child(flags) {
  const result = spawnSync(process.execPath, ['--expose-gc', import.meta.filename, glue, ...flags], {
    encoding: 'utf8',
    maxBuffer: 1 << 20
  });
  if (result.status !== 0) {
    throw new Error(`child ${flags.join(' ')} failed: ${result.stderr}`);
  }
  return JSON.parse(result.stdout);
}

function git(...command) {
  return execFileSync('git', command, { cwd: repo, encoding: 'utf8' }).trim();
}

async function main() {
  const budget = Number(args.budget);
  const { typical, cases } = loadCases();
  const parse = await loadParse();
  const cold = Array.from({ length: 5 }, () => child(['--child-cold']));
  const results = [
    {
      id: 'typical',
      group: 'typical',
      documents: typical.length,
      units: typical.reduce((sum, markdown) => sum + markdown.length, 0),
      outcome: 'accepted',
      parse: measure(() => typical.forEach(parse), budget),
      memory: child(['--child-memory', 'typical'])
    }
  ];
  for (const c of cases) {
    process.stderr.write(`${c.id}\n`);
    results.push({
      id: c.id,
      group: c.group,
      documents: 1,
      units: c.markdown.length,
      outcome: outcome(parse, c.markdown),
      parse: measure(() => outcome(parse, c.markdown), budget),
      memory: child(['--child-memory', c.id])
    });
  }
  const wasm = join(dirname(resolve(glue)), 'gsx.wasm');
  const report = {
    reference: {
      implementation: 'gsx-wasm',
      commit: git('rev-parse', 'HEAD'),
      dirty: git('status', '--porcelain', '--untracked-files=no') !== ''
    },
    environment: {
      node: process.version,
      platform: `${os.platform()} ${os.release()} ${os.arch()}`,
      cpu: os.cpus()[0]?.model,
      date: new Date().toISOString()
    },
    budgetMs: budget,
    wasm: {
      bytes: readFileSync(wasm).length,
      gzip9Bytes: execFileSync('gzip', ['-9', '-c', wasm], { maxBuffer: 1 << 24 }).length
    },
    cold: {
      importMs: stats(cold.map(({ importMs }) => importMs)),
      firstParseMs: stats(cold.map(({ firstParseMs }) => firstParseMs))
    },
    results
  };
  writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`);
}

if (glue === undefined) {
  throw new Error('usage: node bench.mjs <gsx.js> <report.json>');
}
if (args['child-memory'] !== undefined) {
  await childMemory(args['child-memory']);
} else if (args['child-cold']) {
  await childCold();
} else {
  await main();
}
