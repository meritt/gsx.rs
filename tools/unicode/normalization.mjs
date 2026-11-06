import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const VERSION = '17.0.0';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const crate = join(root, 'crates', 'gsx-core');
const source = process.argv[2];

const test = source
  ? await readFile(join(source, 'NormalizationTest.txt'), 'utf8')
  : await (await fetch(`https://www.unicode.org/Public/${VERSION}/ucd/NormalizationTest.txt`)).text();
if (!test.includes(`NormalizationTest-${VERSION}.txt`)) {
  throw new Error(`NormalizationTest.txt is not version ${VERSION}`);
}

const cases = test
  .split('\n')
  .map((line) => line.replace(/#.*/, '').trim())
  .filter((line) => line && !line.startsWith('@'))
  .map((line) => line.split(';').slice(0, 3))
  .filter(([source, composed, decomposed]) => source !== composed || source !== decomposed)
  .map((columns) => columns.join(';'));
await mkdir(join(crate, 'tests', 'project'), { recursive: true });
await writeFile(join(crate, 'tests', 'project', 'NormalizationTest.txt'), `${VERSION}\n${cases.join('\n')}\n`);
