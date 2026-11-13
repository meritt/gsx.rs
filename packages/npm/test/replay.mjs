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

if (gsx.isMediaId('a\ud800') || gsx.isMediaId('\udc00a')) {
  throw new Error('isMediaId accepts a lone surrogate');
}

try {
  gsx.isMediaId(1);
  throw new Error('isMediaId accepts a number');
} catch (error) {
  if (!(error instanceof TypeError)) throw error;
}

if (gsx.isMediaId('a'.repeat(1e7))) {
  throw new Error('isMediaId accepts an ID over the byte limit');
}

for (const text of ['a'.repeat(262145), 'a\ud800']) {
  try {
    gsx.projectJSON(text, 'no-such-target');
    throw new Error(`projectJSON accepts an unknown target for ${text.length} units`);
  } catch (error) {
    if (!error.message.startsWith('unknown target')) throw error;
  }
}

try {
  gsx.projectJSON('', 'telegram-html-rich', { items: [{ id: 'x"/><img src="media:evil', format: 'image' }] });
  throw new Error('projectJSON accepts an item ID outside the media ID rule');
} catch (error) {
  if (error.message !== 'item 0 id is not a media ID') throw error;
}

const patterns = ['', 'u', 'v'].map((flags) => new RegExp(gsx.MEDIA_ID_PATTERN, flags));

function mediaRule({ mediaRule, mediaId }) {
  if (mediaRule) {
    return JSON.stringify({
      mediaIdMaxBytes: gsx.MEDIA_ID_MAX_BYTES,
      mediaGroupMin: gsx.MEDIA_GROUP_MIN,
      mediaGroupMax: gsx.MEDIA_GROUP_MAX,
      mediaIdPattern: gsx.MEDIA_ID_PATTERN,
    });
  }

  const valid = gsx.isMediaId(mediaId);

  if (patterns.some((pattern) => pattern.test(mediaId) !== valid)) {
    throw new Error(`MEDIA_ID_PATTERN and isMediaId differ for ${JSON.stringify(mediaId)}`);
  }

  return JSON.stringify(valid);
}

for (const file of files) {
  const cases = readFileSync(file, 'utf8').split('\n').slice(1).filter(Boolean).map(JSON.parse);

  for (const record of cases) {
    if ('mediaRule' in record || 'mediaId' in record) {
      lines.push(mediaRule(record));
      continue;
    }

    const { markdown, media, target, facts } = record;
    let json;

    try {
      json = target ? gsx.projectJSON(markdown, target, facts, media) : gsx.parseJSON(markdown, media);
    } catch (error) {
      if (!target) throw error;
      lines.push(JSON.stringify({ refused: error.message }));
      continue;
    }

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
