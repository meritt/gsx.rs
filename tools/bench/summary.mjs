import { readFileSync, writeFileSync } from 'node:fs';

const [input, output] = process.argv.slice(2);
if (input === undefined || output === undefined) {
  throw new Error('usage: node summary.mjs <report.json> <report.md>');
}
const report = JSON.parse(readFileSync(input, 'utf8'));
const ms = (value) => (value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2));
const mib = (kib) => (kib / 1024).toFixed(1);
const row = (cells) => `| ${cells.join(' | ')} |`;

const native = report.reference.implementation === 'gsx-native';
const { reference, environment } = report;
const lines = [
  `${native ? 'gsx' : 'gsx Wasm'} ${reference.commit.slice(0, 7)}${reference.dirty ? ' (dirty)' : ''}, ${native ? `native, ${environment.rustc}` : `Node.js ${environment.node}`}, ${environment.cpu}, ${environment.date.slice(0, 10)}`,
  ''
];
if (report.wasm !== undefined) {
  lines.push(`gsx.wasm: ${report.wasm.bytes} bytes, ${report.wasm.gzip9Bytes} in gzip -9.`, '');
}
if (report.cold !== undefined) {
  lines.push(
    `Cold start: import ${ms(report.cold.importMs.median)} ms, first parse ${ms(report.cold.firstParseMs.median)} ms.`,
    ''
  );
}
lines.push(
  row(['Case', 'UTF-16 units', 'Outcome', 'Parse median, ms', 'p95, ms', native ? 'Peak heap +MiB' : 'Peak RSS +MiB']),
  row(['---', '---:', '---', '---:', '---:', '---:'])
);
for (const result of report.results) {
  const perDocument = result.documents > 1 ? ` (${ms(result.parse.median / result.documents)} per document)` : '';
  lines.push(
    row([
      result.documents > 1 ? `${result.id} ×${result.documents}` : result.id,
      result.units,
      result.outcome,
      `${ms(result.parse.median)}${perDocument}`,
      ms(result.parse.p95),
      mib(result.memory.deltaKiB)
    ])
  );
}

const projected = (report.targets ?? []).length === 0 ? [] : report.results.filter((result) => result.projections);
if (projected.length > 0) {
  lines.push(
    '',
    'Projection median, ms; * fitted to the limit.',
    '',
    row(['Target', ...projected.map((result) => result.id)]),
    row(['---', ...projected.map(() => '---:')])
  );
  for (const target of report.targets) {
    lines.push(
      row([
        target,
        ...projected.map((result) => {
          const value = result.projections[target];
          return `${ms(value.median)}${value.fitted ? '*' : ''}`;
        })
      ])
    );
  }
}
writeFileSync(output, `${lines.join('\n')}\n`);
