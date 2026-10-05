import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

// 作者自检：仅一份原始端点控制。既不调用 D54，也不推造原义身份。
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../../../../..');
const output = process.argv[2];
if (!output) throw new Error('usage: node check-reflection.mjs OUTPUT_JSON');
const hash = b => createHash('sha256').update(b).digest('hex');
const check = (p, message) => { if (!p) throw new Error(message); };
const sources = JSON.parse(readFileSync(resolve(here, 'source-excerpts-v1.json'), 'utf8'));
let linesChecked = 0;
let segmentsChecked = 0;
for (const source of sources.sources) {
  const bytes = readFileSync(resolve(root, source.path));
  check(hash(bytes) === source.sha256, `source digest: ${source.path}`);
  const lines = bytes.toString('utf8').split('\n');
  for (const excerpt of source.lines) {
    check(lines[excerpt.line - 1] === excerpt.text, `${source.path}:${excerpt.line}`);
    linesChecked++;
  }
  for (const segment of source.adoptedSegments) {
    check(lines[segment.line - 1].includes(segment.text), `segment ${source.path}:${segment.line}`);
    check(!/[（(]\s*(娇注|娇|注)[:：]/u.test(segment.text), `annotation in adopted segment ${source.path}:${segment.line}`);
    segmentsChecked++;
  }
}

const x = [0, 10, 0, 10, 0, 30, 25, 20, 28];
const sealed = [];
let start = 0;
let direction = 0;
for (let event = 1; event < x.length; event++) {
  const sign = Math.sign(x[event] - x[event - 1]);
  if (sign === 0) continue;
  if (!direction) { direction = sign; continue; }
  if (sign !== direction) {
    const end = event - 1;
    const values = x.slice(start, end + 1);
    sealed.push({index: sealed.length, start, end, knownAt: event, direction,
      owned: Array.from({length: end-start}, (_, i) => start+i+1),
      range: [Math.min(...values), Math.max(...values)]});
    start = end;
    direction = sign;
  }
}
const core = [0, 10];
const intersects = range => range[0] <= core[1] && core[0] <= range[1];
check(sealed.length === 6, 'six sealed legs');
check(JSON.stringify(sealed.map(l => l.end)) === JSON.stringify([1,2,3,4,5,7]), 'leg endpoints');
check(sealed.slice(0,5).every(l => intersects(l.range)), 'prefix touches fixed core');
check(!intersects(sealed[5].range), 'R avoids fixed core');
check(sealed[5].start === 5 && sealed[5].end === 7 && sealed[5].knownAt === 8, 'R source and clock');
const hypotheticalEnd = 6;
const exactEndpointMatches = sealed.filter(l => l.end === hypotheticalEnd).map(l => l.index);
check(exactEndpointMatches.length === 0, 'interior endpoint has no full-leg match');
const result = {
  status: 'AUTHOR_SELF_CHECK_PASS',
  sourceFilesChecked: sources.sources.length, sourceLinesChecked: linesChecked,
  adoptedSegmentsChecked: segmentsChecked,
  theoremEvidence: {S58: 'source-conditioned ordinary argument', R58: 'conditional ordinary proof; raw representation not instantiated',
    G58: 'ordinary proof; one exact scalar endpoint control'},
  controlCount: 1, sourceCaseCount: 1,
  control: {states: x, events: x.length - 1, sealed, core, j: 4, r: 5,
    wholeX: {owns: [1,2,3,4,5], range: [0,30]},
    hypotheticalWhole: {end: hypotheticalEnd, owns: [1,2,3,4,5,6], qualifiedOriginalMove: null},
    exactEndpointMatches, excludedInterpretation: 'Not an original-semantics counterexample to ER57'},
  qualification: {omega56ER57: null, originalP: null, fixedF2Child: null, independentReview: null},
};
mkdirSync(dirname(resolve(output)), { recursive: true });
writeFileSync(output, JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify({status: result.status, sources: sources.sources.length,
  linesChecked, segmentsChecked, controlCount: 1, events: x.length - 1}));
