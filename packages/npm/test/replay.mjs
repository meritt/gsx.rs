import { readFileSync, writeFileSync } from 'node:fs';
import { load } from '@meritt/gsx-wasm';

const [out, ...files] = process.argv.slice(2);

if (!out || files.length === 0) {
  console.error('usage: replay.mjs <out.jsonl> <corpus.jsonl>...');
  process.exit(2);
}

const gsx = await load();
const lines = [];
const names = gsx.targets().map(({ name }) => name);

if (names.length === 0 || !Object.isFrozen(gsx.targets()[0])) {
  throw new Error(`targets() lists ${names}`);
}

for (const file of files) {
  const cases = readFileSync(file, 'utf8').split('\n').slice(1).filter(Boolean).map(JSON.parse);

  for (const { markdown, media, target, facts } of cases) {
    const json = target ? gsx.projectJSON(markdown, target, facts, media) : gsx.parseJSON(markdown, media);
    const wire = JSON.parse(json);

    try {
      if (target) {
        gsx.project(markdown, target, facts, media);
        if (!wire.projection) throw new Error(`project returned a projection for ${json}`);
      } else {
        gsx.parse(markdown, media);
        if (!wire.document) throw new Error(`parse returned a document for ${json}`);
      }
    } catch (error) {
      if (!(error instanceof gsx.ParseError) || error.code !== wire.error?.code) throw error;
    }

    lines.push(json);

    if (!target) {
      const serialized = gsx.serializeJSON(markdown, media);
      const wire = JSON.parse(serialized);

      try {
        if (gsx.serialize(markdown, media) !== wire.serialized) throw new Error(`serialize differs for ${serialized}`);
      } catch (error) {
        if (!(error instanceof gsx.ParseError) || error.code !== wire.error?.code) throw error;
      }

      lines.push(serialized);
    }
  }
}

writeFileSync(out, `${lines.join('\n')}\n`);
