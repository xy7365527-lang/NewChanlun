import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

// 作者有限证据检查。这里的原始腿没有原义次级身份，结果不签发语义资格。
const here = path.dirname(fileURLToPath(import.meta.url));
const proof = path.resolve(here, '../..');
const diagnosticPath = path.join(proof, 'stage52/diagnostic-evidence/diagnostic-input.json');
const mainPath = path.join(proof, 'stage52/candidate-evidence/results-v1/same_up_counterexample.json');
const sha = p => createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const diagnostic = JSON.parse(fs.readFileSync(diagnosticPath, 'utf8'));
const main = JSON.parse(fs.readFileSync(mainPath, 'utf8')).source;

function replay(input, count) {
  const orders = new Map(Object.entries(input.initial).map(([side, o]) => [o.id, { ...o, side }]));
  function bestQ() {
    const bids = [...orders.values()].filter(o => o.side === 'bid' && o.quantity > 0);
    const price = Math.max(...bids.map(o => o.price));
    assert.equal(price, 100);
    return bids.filter(o => o.price === price).reduce((s, o) => s + o.quantity, 0);
  }
  const q = [bestQ()];
  for (const e of input.events.slice(0, count)) {
    assert.equal(e.seq, q.length);
    if (e.kind === 'add') {
      assert.ok(!orders.has(e.id));
      assert.ok(e.quantity > 0);
      orders.set(e.id, { ...e });
    } else {
      assert.equal(e.kind, 'reduce');
      const o = orders.get(e.id);
      assert.ok(o && e.quantity > 0 && o.quantity >= e.quantity);
      o.quantity -= e.quantity;
    }
    q.push(bestQ());
  }
  return q;
}

function legs(q) {
  const out = [];
  let direction = 0, start = 0;
  for (let i = 1; i < q.length; i++) {
    const next = Math.sign(q[i] - q[i - 1]);
    if (!next) continue;
    if (!direction) direction = next;
    else if (next !== direction) {
      const end = i - 1;
      const values = q.slice(start, end + 1);
      out.push({ ordinal: out.length + 1, start, end, knownAt: i,
        direction, range: [Math.min(...values), Math.max(...values)],
        ownedEvents: Array.from({ length: end - start }, (_, k) => start + k + 1) });
      start = end;
      direction = next;
    }
  }
  return out;
}

const core = xs => [Math.max(...xs.map(x => x.range[0])), Math.min(...xs.map(x => x.range[1]))];
function crossing(q, l, c) {
  for (let i = l.start + 1; i <= l.end; i++) {
    if (q[i - 1] <= c[1] && q[i] > c[1]) return i;
  }
  return null;
}

function scope(q, xs, li, ri) {
  const c = core(xs.slice(0, 3)), l = xs[li], r = xs[ri];
  const refs = [...xs.slice(0, 3), l, r];
  const distinctOrdinals = [...new Set(refs.map(x => x.ordinal))];
  return {
    core: c, seedOrdinals: [1, 2, 3], departureOrdinal: l.ordinal, returnOrdinal: r.ordinal,
    sourceOrder: { seedEndsAt: xs[2].end, departureStartsAt: l.start,
      departureEndsAt: l.end, returnStartsAt: r.start, returnEndsAt: r.end,
      postSeedDistinctAdjacentObjects: l.ordinal > 3 && r.ordinal === l.ordinal + 1 && l.start >= xs[2].end },
    distinctObjectCount: distinctOrdinals.length, distinctOrdinals,
    numerical: { upwardCrossingObservedAt: crossing(q, l, c),
      departureUp: l.direction === 1, returnDown: r.direction === -1,
      returnStrictlyAbove: r.range[0] > c[1] },
    timing: { conditionalFormationEndpoint: xs[2].end,
      rawThreeMembersKnownAt: Math.max(...xs.slice(0, 3).map(x => x.knownAt)),
      rawPairKnownAt: Math.max(l.knownAt, r.knownAt) },
    semantic: { originalCenterIdentity: null, originalLowerMoveIdentity: null,
      originalFormationAt: null, originalKnownAt: null, semanticEligibleAt: null },
  };
}

const qD = replay(diagnostic, 6);
const qM = replay(main, 7);
assert.deepEqual(qD, [100, 110, 104, 112, 111, 113, 112]);
assert.deepEqual(qM, [1040, 1040, 1000, 1030, 1010, 1140, 1100, 1130]);
const d = legs(qD), m = legs(qM);
assert.equal(d.length, 5); assert.equal(m.length, 5);
const overlap = scope(qD, d, 2, 3), postSeed = scope(qD, d, 3, 4), standard = scope(qM, m, 3, 4);
assert.equal(overlap.numerical.upwardCrossingObservedAt, 3);
assert.equal(overlap.numerical.returnStrictlyAbove, true);
assert.equal(overlap.distinctObjectCount, 4);
assert.equal(overlap.sourceOrder.postSeedDistinctAdjacentObjects, false);
assert.equal(overlap.timing.rawPairKnownAt, 5);
assert.equal(overlap.numerical.upwardCrossingObservedAt, overlap.timing.conditionalFormationEndpoint);
assert.equal(postSeed.distinctObjectCount, 5);
assert.equal(postSeed.sourceOrder.postSeedDistinctAdjacentObjects, true);
assert.equal(postSeed.numerical.departureUp, false);
assert.equal(postSeed.numerical.upwardCrossingObservedAt, null);
assert.equal(standard.sourceOrder.postSeedDistinctAdjacentObjects, true);
assert.equal(standard.numerical.upwardCrossingObservedAt, 5);
assert.equal(standard.timing.rawPairKnownAt, 7);
assert.ok(standard.sourceOrder.departureStartsAt < standard.timing.rawThreeMembersKnownAt);
assert.deepEqual(core(d.slice(1, 4)), [111, 110]);
assert.deepEqual(core(d.slice(2, 5)), [111, 112]);

const out = {
  status: 'author-check-pass', scope: '两条既有合成流的13事件引用、15含初态状态；不是新样本或独评；不复跑Stage52解析器。',
  inputs: [{ path: diagnosticPath, sha256: sha(diagnosticPath), eventReferences: 6 },
    { path: mainPath, sha256: sha(mainPath), eventReferences: 7 }],
  diagnostic: { q: qD, completedRawLegs: d, seedTailPair: overlap, postSeedPair: postSeed,
    rightShiftedWindows: [{ seedOrdinals: [2, 3, 4], intersectionBounds: [111, 110], nonempty: false },
      { seedOrdinals: [3, 4, 5], intersectionBounds: [111, 112], nonempty: true, completedPostSeedLRAvailable: false }],
    precedingCompleteObject: null },
  stage52MainPrefix: { q: qM, completedRawLegs: m, postSeedPair: standard },
  lesson61TextualIndexExpansion: {
    rejected: { seed: [[70,71],[71,72],[72,73]], proposedL: [72,73], proposedR: [73,74], distinctObjects: 4 },
    acceptedByAuthor: { seed: [[69,70],[70,71],[71,72]], L: [72,73], R: [73,74], distinctObjects: 5 },
    pricesReconstructedFromImage: false,
  },
};
fs.writeFileSync(path.join(here, 'source-scope-checks.json'), JSON.stringify(out, null, 2) + '\n');
console.log(JSON.stringify({ status: out.status, eventReferences: 13, states: 15,
  diagnosticPairDistinctObjects: 4, mainPairDistinctObjects: 5, semanticFieldsRemainNull: true }));
