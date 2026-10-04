import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

// Only this new, explicitly synthetic 40-trade control is evaluated.
// No old parser, enumeration, market replay, or semantic Completed oracle.
const out = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(out, '../../../../..');
const levels = [0, 10, 7, 2, 4, 9, 6, 1, 3, 8, 6, 3, 4];
const prices = [];
for (let i = 0; i < levels.length; i++) {
  if (i === 6) prices.push(8);
  prices.push(levels[i], levels[i], levels[i]);
}
assert.equal(prices.length, 40);
const events = prices.map((coordinate, i) => ({ seq: i + 1, price: 100 + coordinate, coordinate, quantity: 1 }));
const runs = [];
for (let lo = 0; lo < prices.length;) {
  let hi = lo + 1;
  while (hi < prices.length && prices[hi] === prices[lo]) hi++;
  runs.push({ price: prices[lo], first: lo + 1, last: hi, count: hi - lo });
  lo = hi;
}
const nuclei = runs.filter(r => r.count === 3).map((r, i) => ({ id: `K${i}`, ...r }));
const nonNucleus = runs.filter(r => r.count < 3);
assert.deepEqual(nuclei.map(k => k.price), levels);
assert.deepEqual(nonNucleus, [{ price: 8, first: 19, last: 19, count: 1 }]);
const states = [0, ...prices];
const hull = (start, end) => [Math.min(...states.slice(start, end + 1)), Math.max(...states.slice(start, end + 1))];
const intersect = ranges => [Math.max(...ranges.map(r => r[0])), Math.min(...ranges.map(r => r[1]))];

function table(cuts) {
  return Array.from({ length: 6 }, (_, i) => {
    const a = 2 * i, b = a + 1, witness = b + 1;
    const direction = Math.sign(levels[b] - levels[a]);
    assert.equal(Math.sign(levels[witness] - levels[b]), -direction);
    assert.ok(cuts[i] < nuclei[a].first && nuclei[b].last <= cuts[i + 1]);
    return {
      id: `T${i + 1}`, start: cuts[i], end: cuts[i + 1],
      proposedType: direction === 1 ? 'U' : 'D',
      nuclei: [nuclei[a].id, nuclei[b].id],
      reversalNucleus: nuclei[witness].id,
      reversalKnownAt: nuclei[witness].first + 2,
      rawFullRange: hull(cuts[i], cuts[i + 1]),
    };
  });
}
const right = table([0, 6, 12, 18, 25, 31, 37]);
const comparison = table([0, 6, 12, 19, 25, 31, 37]);
const signature = ts => ts.map(({ proposedType, nuclei, reversalNucleus, reversalKnownAt }) => ({ proposedType, nuclei, reversalNucleus, reversalKnownAt }));
assert.deepEqual(signature(right), signature(comparison));
const groups = ts => [0, 3].map(i => ({
  factors: ts.slice(i, i + 3).map(t => t.id),
  start: ts[i].start, end: ts[i + 2].end,
  rawFullRange: hull(ts[i].start, ts[i + 2].end),
  threeRangeIntersection: intersect(ts.slice(i, i + 3).map(t => t.rawFullRange)),
}));
const rGroups = groups(right), cGroups = groups(comparison);
assert.deepEqual(rGroups.map(g => g.threeRangeIntersection), [[2, 9], [3, 8]]);
assert.deepEqual(cGroups.map(g => g.threeRangeIntersection), [[2, 9], [3, 8]]);
assert.deepEqual(rGroups.map(g => g.rawFullRange), [[0, 10], [1, 9]]);
assert.deepEqual(cGroups.map(g => g.rawFullRange), [[0, 10], [1, 8]]);

const packet = {
  identity: 'B55-TR40-v1',
  status: 'synthetic-trade-model; no market sample; no semantic completion certificate',
  nucleusIdentityQualification: 'nuclei means the selected candidate directory of three-trade formation witnesses; distinct original instances and lifecycle are not certified',
  coordinateConvention: 'coordinate = transaction price - 100; all printed ranges use coordinate',
  events, initialPrice: 100, initialStateCoordinate: 0, nuclei, nonNucleus,
  selectedDelta: 'deltaR: end at last owned nucleus; transition event 19 belongs to successor',
  selectedFactors: right, selectedGroups: rGroups,
  comparisonPurpose: 'test information content of the nucleus/reversal certificate only; not a second legal decomposition',
  comparisonFactors: comparison, comparisonGroups: cGroups,
  numericalFindings: {
    nucleusAndReversalSignaturesEqual: true,
    firstGroupEnd: [18, 19],
    secondGroupRange: [[1, 9], [1, 8]],
  },
};
fs.writeFileSync(path.join(out, 'control-v1.json'), JSON.stringify(packet, null, 2) + '\n');

const lineSets = {
  '017-第17课.md': [12, 36, 40, 44, 46, 52, 60, 98],
  '018-第18课.md': [12, 24, 26, 28, 38, 40, 44, 64, 68],
  '020-第20课.md': [12, 34, 48, 52, 54, 58, 94],
  '032-第32课.md': [163, 165],
  '035-第35课.md': [12, 14, 16, 18, 22, 24, 38],
  '037-第37课.md': [164, 166, 170, 172, 174],
  '038-第38课.md': [12, 18, 22, 24, 26, 44, 290, 292, 296, 298, 300],
  '039-第39课.md': [12, 26, 28, 30, 48],
  '061-第61课.md': [12, 44, 56],
  '084-第84课.md': [12, 52, 54, 56, 60, 68],
};
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const excerpts = Object.entries(lineSets).map(([name, nums]) => {
  const rel = `docs/chanlun/text/blog/${name}`;
  const bytes = fs.readFileSync(path.join(root, rel));
  const lines = bytes.toString().split('\n');
  return { path: rel, sha256: sha(bytes), lines: nums.map(line => ({ line, text: lines[line - 1] })) };
});
fs.writeFileSync(path.join(out, 'source-excerpts-v1.json'), JSON.stringify(excerpts, null, 2) + '\n');
console.log(JSON.stringify({ control: packet.identity, trades: events.length, nuclei: nuclei.length, nonNucleusEvents: [19], numericalChecks: 'pass', semanticClaim: 'none' }));
