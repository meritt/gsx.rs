import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const VERSION = '17.0.0';
const FILES = {
  break: `ucd/auxiliary/GraphemeBreakProperty.txt`,
  breakTest: `ucd/auxiliary/GraphemeBreakTest.txt`,
  core: `ucd/DerivedCoreProperties.txt`,
  emoji: `ucd/emoji/emoji-data.txt`,
  sequences: `emoji/emoji-sequences.txt`,
  zwj: `emoji/emoji-zwj-sequences.txt`
};

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const crate = join(root, 'crates', 'gsx-core');
const source = process.argv[2];

async function load(path) {
  const text = source
    ? await readFile(join(source, path.split('/').at(-1)), 'utf8')
    : await (await fetch(`https://www.unicode.org/Public/${VERSION}/${path}`)).text();
  const name = path.split('/').at(-1).replace('.txt', '');
  if (!text.includes(`# ${name}-${VERSION}.txt`) && !text.includes(`# Version: ${VERSION.slice(0, -2)}\n`)) {
    throw new Error(`${path} is not version ${VERSION}`);
  }
  return text;
}

function records(text) {
  return text
    .split('\n')
    .map((line) => line.replace(/#.*/, '').trim())
    .filter(Boolean)
    .map((line) => line.split(';').map((field) => field.trim()));
}

function range(field) {
  const [from, to = from] = field.split('..').map((value) => Number.parseInt(value, 16));
  return [from, to];
}

const GCB = [
  'Other',
  'CR',
  'LF',
  'Control',
  'Extend',
  'ZWJ',
  'Regional_Indicator',
  'Prepend',
  'SpacingMark',
  'L',
  'V',
  'T',
  'LV',
  'LVT'
];
const INCB = ['None', 'Linker', 'Consonant', 'Extend'];

const [breaks, breakTest, core, emoji, sequences, zwj] = await Promise.all(
  Object.values(FILES).map(load)
);

const property = new Uint8Array(0x110000);
for (const [field, value] of records(breaks)) {
  const index = GCB.indexOf(value);
  if (index < 0) {
    throw new Error(`unknown Grapheme_Cluster_Break ${value}`);
  }
  const [from, to] = range(field);
  property.fill(index, from, to + 1);
}

const sets = { Extended_Pictographic: [], Emoji_Presentation: [], Emoji_Modifier: [] };
for (const [field, value] of records(emoji)) {
  if (value in sets) {
    sets[value].push(range(field));
  }
}
for (const [from, to] of sets.Extended_Pictographic) {
  for (let point = from; point <= to; point += 1) {
    property[point] |= 1 << 4;
  }
}
for (const [field, name, value] of records(core)) {
  if (name === 'InCB') {
    const [from, to] = range(field);
    for (let point = from; point <= to; point += 1) {
      property[point] |= INCB.indexOf(value) << 5;
    }
  }
}

const ignorable = records(core)
  .filter(([, name]) => name === 'Default_Ignorable_Code_Point')
  .map(([field]) => range(field));
for (let point = 0; point < 0x110000; point += 1) {
  const listed = ignorable.some(([from, to]) => from <= point && point <= to);
  const surrogate = point >= 0xd800 && point <= 0xdfff;
  if (!surrogate && listed !== /\p{Default_Ignorable_Code_Point}/u.test(String.fromCodePoint(point))) {
    throw new Error(`Node disagrees on Default_Ignorable_Code_Point U+${point.toString(16).toUpperCase()}`);
  }
}

const map = [];
for (let point = 0; point < property.length; point += 1) {
  if (point === 0 || property[point] !== property[point - 1]) {
    map.push(point * 256 + property[point]);
  }
}

function inversion(ranges) {
  const points = new Set();
  for (const [from, to] of ranges) {
    for (let point = from; point <= to; point += 1) {
      points.add(point);
    }
  }
  const list = [];
  for (let point = 0; point <= 0x110000; point += 1) {
    if (points.has(point) !== points.has(point - 1)) {
      list.push(point);
    }
  }
  return list;
}

const singles = [];
const multiple = [];
for (const [field, type] of [...records(sequences), ...records(zwj)]) {
  if (field.includes('..')) {
    const [from, to] = range(field);
    if (type !== 'Basic_Emoji') {
      throw new Error(`range in ${type}`);
    }
    singles.push([from, to]);
    continue;
  }
  const points = field.split(/\s+/).map((value) => Number.parseInt(value, 16));
  if (points.length === 1) {
    singles.push([points[0], points[0]]);
  } else {
    multiple.push(String.fromCodePoint(...points));
  }
}

const SECOND = (point) =>
  point === 0xfe0f ||
  point === 0x200d ||
  (point >= 0x1f3fb && point <= 0x1f3ff) ||
  (point >= 0xe0020 && point <= 0xe007f) ||
  (point >= 0x1f1e6 && point <= 0x1f1ff);
const encoder = new TextEncoder();
const sorted = [...new Set(multiple)].toSorted((left, right) =>
  Buffer.compare(Buffer.from(encoder.encode(left)), Buffer.from(encoder.encode(right)))
);
let longest = 0;
for (const sequence of sorted) {
  const points = [...sequence].map((value) => value.codePointAt(0));
  if (!SECOND(points[1])) {
    throw new Error(`sequence ${points.map((p) => p.toString(16))} has an unexpected second code point`);
  }
  longest = Math.max(longest, points.length);
}
const ends = [];
let blob = '';
for (const sequence of sorted) {
  blob += sequence;
  ends.push(encoder.encode(blob).length);
}
if (ends.at(-1) > 0xffff) {
  throw new Error('sequence blob exceeds u16 offsets');
}

const rgi = new RegExp(String.raw`^\p{RGI_Emoji}$`, 'v');
for (const sequence of sorted) {
  if (!rgi.test(sequence)) {
    throw new Error(`Node disagrees on ${sequence}`);
  }
}

function escape(text) {
  return [...text].map((value) => `\\u{${value.codePointAt(0).toString(16).toUpperCase()}}`).join('');
}

function array(name, type, values, hex = true) {
  const items = values.map((value) => (hex ? `0x${value.toString(16).toUpperCase()}` : String(value)));
  return `pub static ${name}: [${type}; ${values.length}] = [${items.join(', ')}];\n`;
}

const rust =
  `pub const UNICODE_VERSION: &str = "${VERSION}";\n\n` +
  array('BREAK', 'u32', map) +
  '\n' +
  array('EMOJI_PRESENTATION', 'u32', inversion(sets.Emoji_Presentation)) +
  '\n' +
  array('DEFAULT_IGNORABLE', 'u32', inversion(ignorable)) +
  '\n' +
  array('RGI_SINGLE', 'u32', inversion(singles)) +
  '\n' +
  `pub const RGI_LONGEST: usize = ${longest};\n\n` +
  `pub static RGI_SEQUENCES: &str = "${escape(blob)}";\n\n` +
  array('RGI_ENDS', 'u16', ends, false);

const tables = join(crate, 'src', 'measure', 'tables.rs');
await writeFile(tables, rust);
execFileSync('rustfmt', ['--edition', '2024', '--config-path', join(root, 'rustfmt.toml'), tables]);

const test = breakTest
  .split('\n')
  .map((line) => line.replace(/#.*/, '').trim())
  .filter(Boolean);
await mkdir(join(crate, 'tests', 'measure'), { recursive: true });
await writeFile(
  join(crate, 'tests', 'measure', 'GraphemeBreakTest.txt'),
  `${VERSION}\n${test.join('\n')}\n`
);

console.log(
  `BREAK ${map.length}, EMOJI_PRESENTATION ${inversion(sets.Emoji_Presentation).length}, ` +
    `RGI_SINGLE ${inversion(singles).length}, RGI_SEQUENCES ${sorted.length} (${ends.at(-1)} bytes, longest ${longest}), ` +
    `GraphemeBreakTest ${test.length}`
);
