import fs from 'node:fs';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const repo = '/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const out = path.dirname(fileURLToPath(import.meta.url));
const input = `${repo}/.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-results.json`;
const bytes = fs.readFileSync(input);
const inputHash = crypto.createHash('sha256').update(bytes).digest('hex');
const data = JSON.parse(bytes);
const selected = data.examples.filter(e => /^(12_early_cut|30_parent_witness)(_mirror)?$/.test(e.name));
assert.equal(selected.length, 4);
const reports = selected.map(e => {
  const xs = e.history.xs;
  const objects = e.result.C.map((m, i) => {
    const [start, end] = m.boundaryStates;
    const support = m.turnSupport;
    const prices = xs.slice(start, end + 1);
    const lo = Math.min(...prices), hi = Math.max(...prices);
    const next = xs.findIndex((x, k) => k > end && x !== xs[end]);
    const middle = support[1];
    let occurrenceStart = middle.seed[0], occurrenceEnd = middle.seed[2];
    while (occurrenceStart > 0 && xs[occurrenceStart - 1] === middle.value) occurrenceStart--;
    while (occurrenceEnd + 1 < xs.length && xs[occurrenceEnd + 1] === middle.value) occurrenceEnd++;
    const ownEndExtreme = m.direction === 1 ? xs[end] === hi : xs[end] === lo;
    const ownStartExtreme = m.direction === 1 ? xs[start] === lo : xs[start] === hi;
    const supportIsTurn = m.direction === 1
      ? middle.value > support[0].value && middle.value > support[2].value
      : middle.value < support[0].value && middle.value < support[2].value;
    const firstNonflatContinuesOldDirection = next > end && Math.sign(xs[next] - xs[end]) === m.direction;
    assert.deepEqual([lo, hi], m.range);
    assert(ownEndExtreme);
    assert(supportIsTurn);
    assert(end < occurrenceStart);
    assert.notEqual(xs[end], middle.value);
    assert(firstNonflatContinuesOldDirection);
    assert(m.confirm >= occurrenceEnd);
    return {
      ordinal: i + 1, direction: m.direction, boundaryStates: [start, end],
      endpointValues: [xs[start], xs[end]], range: [lo, hi],
      ownEndExtreme, ownStartExtreme, confirm: m.confirm,
      certificateExtremeCore: middle.core,
      certificateExtremeValue: middle.value,
      certificateExtremePlateauObserved: [occurrenceStart, occurrenceEnd],
      certificateLocationEqualsObjectEnd: false,
      firstNonflatAfterEnd: [next, xs[next]], firstNonflatContinuesOldDirection,
    };
  });
  const joins = objects.slice(0, -1).map((left, i) => {
    const right = objects[i + 1];
    const common = left.endpointValues[1];
    assert.equal(common, right.endpointValues[0]);
    const requiredLeft = left.direction === 1 ? left.range[1] : left.range[0];
    const requiredRight = left.direction === 1 ? right.range[1] : right.range[0];
    assert.notEqual(common, requiredRight);
    return { left: i + 1, right: i + 2, common, requiredLeft, requiredRight,
      satisfiesNormalizedJoint: common === requiredLeft && common === requiredRight };
  });
  let shiftedBoundaryComparison = null;
  if (e.name.startsWith('30_parent_witness')) {
    const ends = objects.map(m => m.certificateExtremePlateauObserved[1]);
    const shiftedObjects = ends.map((end, i) => {
      const start = i === 0 ? 0 : ends[i - 1];
      const prices = xs.slice(start, end + 1);
      const range = [Math.min(...prices), Math.max(...prices)];
      const direction = objects[i].direction;
      const coreIds = e.values.map((_, k) => k).filter(k => 3 * k + 1 > start && 3 * k + 3 <= end);
      assert.equal(xs[start], direction === 1 ? range[0] : range[1]);
      assert.equal(xs[end], direction === 1 ? range[1] : range[0]);
      assert(objects[i].confirm >= end);
      return { ordinal: i + 1, boundaryStates: [start, end], events: [start + 1, end],
        coreIds, endpointValues: [xs[start], xs[end]], range,
        originalKnownConfirmation: objects[i].confirm };
    });
    const intersect = rows => [Math.max(...rows.map(m => m.range[0])), Math.min(...rows.map(m => m.range[1]))];
    const originalIntersection = intersect(objects);
    const shiftedIntersection = intersect(shiftedObjects);
    assert.deepEqual(ends, [9, 18, 27]);
    assert.deepEqual(shiftedObjects.map(m => m.coreIds), [[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
    assert.deepEqual(shiftedIntersection, [10, 30]);
    assert.notDeepEqual(originalIntersection, shiftedIntersection);
    shiftedBoundaryComparison = { scope: '有限改切点几何对照；不是全域normalizer或原义合格新F1。',
      shiftedObjects, originalIntersection, shiftedIntersection };
  }
  return { name: e.name, values: e.values, objects, joins, shiftedBoundaryComparison };
});
const result = {
  scope: '仅检验已保存四份合成RT例的标准化端点与指定三点转折证书位置；不是新F1原义判定器。',
  input, inputHash, reports,
  checks: {
    exampleCount: reports.length,
    emittedObjectCount: reports.reduce((n, e) => n + e.objects.length, 0),
    ownEndExtremePass: reports.flatMap(e => e.objects).filter(m => m.ownEndExtreme).length,
    exactCertificateLocationFail: reports.flatMap(e => e.objects).filter(m => !m.certificateLocationEqualsObjectEnd).length,
    completedJoinCount: reports.reduce((n, e) => n + e.joins.length, 0),
    normalizedCompletedJoinFail: reports.flatMap(e => e.joins).filter(j => !j.satisfiesNormalizedJoint).length,
  },
};
fs.writeFileSync(`${out}/results.json`, JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result.checks));
