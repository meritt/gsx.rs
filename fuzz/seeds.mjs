import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const parse = join(import.meta.dirname, 'seeds', 'parse');
const project = join(import.meta.dirname, 'seeds', 'project');

const option = (bytes) => (bytes === undefined ? [0] : [1, ...bytes]);
const u16 = (value) => [value & 0xff, value >> 8];
const u32 = (value) => [value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, value >>> 24];
const small = (value) => [...u32(0), ...u16(value)];
const full = (value) => [...u32(0xffffffff), ...u32(value)];
const item = ({ id, format, spoiler = false, inline = false }) => [1, id, format, 0, Number(spoiler), Number(inline)];

function header(facts) {
  return [
    facts.media ?? 0,
    ...option(facts.kind === undefined ? undefined : [facts.kind]),
    facts.mediaCount ?? 0,
    ...option(facts.placement === undefined ? undefined : [facts.placement]),
    Number(facts.items !== undefined),
    ...(facts.items ?? []).flatMap(item),
    0,
    0,
    0,
    ...option(facts.maxCharacters),
    ...option(facts.reservedPerUrl),
    ...option(facts.captionLimit),
    ...option(facts.descriptionLimit),
    ...option(facts.hashtagLimit),
    0,
    Number(facts.thread ?? false),
    Number(facts.entities ?? false),
    ...option(facts.maximum === undefined ? undefined : [facts.maximum]),
  ];
}

const variants = {
  truncate: {},
  thread: { thread: true, maxCharacters: small(100) },
  'thread-tiny': { thread: true, maxCharacters: small(1), captionLimit: small(1) },
  media: {
    media: 4,
    kind: 0,
    mediaCount: 3,
    items: [{ id: 0, format: 0 }, { id: 1, format: 2, spoiler: true }, { id: 3, format: 1, inline: true }],
    thread: true,
    maxCharacters: small(200),
    captionLimit: small(120),
    entities: true,
  },
  story: {
    placement: 2,
    reservedPerUrl: small(23),
    descriptionLimit: small(150),
    hashtagLimit: small(2),
    maximum: 40,
  },
  full: { thread: true, maxCharacters: full(0xffffffff), reservedPerUrl: full(0xffffffff) },
};

mkdirSync(project, { recursive: true });
for (const file of readdirSync(parse).filter((name) => name.endsWith('.md'))) {
  const text = readFileSync(join(parse, file));
  for (const [variant, facts] of Object.entries(variants)) {
    writeFileSync(join(project, `${file.slice(0, -3)}-${variant}`), Buffer.concat([Buffer.from(header(facts)), text]));
  }
}
