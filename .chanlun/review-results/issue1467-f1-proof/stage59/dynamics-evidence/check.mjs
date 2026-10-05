// Stage59 author development evidence. No external dependencies, market replay,
// original Chan atom certification, or replacement of frozen input semantics.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const packet = path.resolve(here, '../..');
const root = path.resolve(packet, '../../..');
const modelFile = path.join(packet, 'stage58/admissibility-evidence/main8-model-v1.json');
const model = JSON.parse(fs.readFileSync(modelFile, 'utf8'));
const hash = p => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const abs = x => x < 0n ? -x : x;
function gcd(a, b) { while (b) [a, b] = [b, a % b]; return abs(a); }
function rat(n, d = 1n) {
  n = BigInt(n); d = BigInt(d); assert.notEqual(d, 0n);
  if (d < 0n) { n = -n; d = -d; }
  const g = gcd(n, d); return { n: n / g, d: d / g };
}
const add = (a, b) => rat(a.n * b.d + b.n * a.d, a.d * b.d);
const mul = (a, b) => rat(a.n * b.n, a.d * b.d);
const inv = a => rat(a.d, a.n);
const sub = (a, b) => add(a, rat(-b.n, b.d));
const sum = xs => xs.reduce(add, rat(0));
const str = a => `${a.n}/${a.d}`;
const sign = a => a.n < 0n ? -1 : a.n > 0n ? 1 : 0;
const eq = (a, b) => a.n === b.n && a.d === b.d;

// Derive legal order-book states from the original finite control.
const books = { bid: new Map(), ask: new Map() };
for (const [side, key] of [['bid', 'bids'], ['ask', 'asks']]) {
  for (const o of model.input.initial[key]) books[side].set(o.id, o.price);
}
function quote() {
  assert(books.bid.size && books.ask.size);
  const bid = Math.max(...books.bid.values());
  const ask = Math.min(...books.ask.values());
  assert(bid > 0 && bid < ask); return [bid, ask];
}
const states = [quote()];
for (const e of model.input.events) {
  const book = books[e.side];
  if (e.action === 'add') { assert(!book.has(e.orderId)); book.set(e.orderId, e.price); }
  else { assert.equal(e.action, 'cancel'); assert.equal(book.get(e.orderId), e.price); book.delete(e.orderId); }
  states.push(quote());
}
assert.equal(states.length, 9);
const C = model.initial_completed_P0.core;
assert.deepEqual(C, [100, 102]);
assert.equal(model.initial_completed_P0.endAt, 6);
assert.equal(model.initial_completed_P0.knownAt, 8);
assert.deepEqual(model.initial_completed_P0.events, model.input.events.slice(0, 6).map(e => e.id));
const flows = model.raw_flows.map(f => ({ id: f.id, events: f.events, range: f.range ?? f.range_at_8 }));
assert.deepEqual(flows.slice(0, 3).map(f => f.range), [[100, 102], [100, 102], [100, 102]]);
assert.deepEqual(flows[3].range, [1, 102]);
assert.deepEqual(flows.slice(4).map(f => f.range), [[1, 99], [1, 99]]);
let cursor = 0;
for (const f of flows) {
  const owned = model.input.events.slice(cursor, cursor + f.events.length).map(e => e.id);
  assert.deepEqual(f.events, owned);
  const support = states.slice(cursor, cursor + f.events.length + 1);
  assert.deepEqual(f.range, [Math.min(...support.map(q => q[0])), Math.max(...support.map(q => q[1]))]);
  cursor += f.events.length;
}
function spans(n) {
  const out = [];
  for (let lo = 0; lo < n; lo++) for (let hi = lo; hi < n; hi++) {
    const range = [Math.min(...flows.slice(lo, hi + 1).map(f => f.range[0])), Math.max(...flows.slice(lo, hi + 1).map(f => f.range[1]))];
    // Relaxed necessary crossing test: touch C AND have some support outside C.
    const touches = range[0] <= C[1] && range[1] >= C[0];
    const outside = range[0] < C[0] || range[1] > C[1];
    out.push({ lo, hi, range, crossNecessary: touches && outside });
  }
  return out;
}
const crossing = [4, 5, 6].map(n => {
  const all = spans(n);
  const orderedPairs = all.flatMap(b => all.filter(c => b.hi < c.lo).map(c => [b, c]));
  const qualifying = orderedPairs.filter(([b, c]) => b.crossNecessary && c.crossNecessary);
  assert.equal(qualifying.length, 0);
  assert(all.filter(s => s.crossNecessary).every(s => s.lo <= 3 && s.hi >= 3));
  return { n, spans: all.length, ordered_nonoverlapping_pairs: orderedPairs.length,
    relaxed_crossing_spans: all.filter(s => s.crossNecessary), qualifying_pairs: qualifying.length,
    meaning: n === 4 ? 'whole at endAt 6' : n === 5 ? 'sealed flows at knownAt 8' : 'relaxed geometry including active F5 at 8, not completed' };
});

// Exact #873 algebra on a declared atom carrier, without assigning legal atom roles.
function coefficients(displacements, b, c) {
  const a = displacements.map(() => 0n);
  for (const [i, s] of [[c[1], 1n], [c[0], -1n], [b[1], -1n], [b[0], 1n]]) {
    assert(Number.isInteger(i) && i >= 0 && i < a.length);
    a[i] += s * BigInt(displacements[i]);
  }
  return a;
}
function classify(a) {
  const p = a.some(x => x > 0n), n = a.some(x => x < 0n);
  return n && p ? 'flips' : n ? 'always_negative' : p ? 'always_positive' : 'identically_zero';
}
function value(a, durations) {
  assert.equal(a.length, durations.length);
  assert(durations.every(d => d.n > 0n && d.d > 0n));
  return sum(a.map((v, i) => mul(rat(v), inv(durations[i]))));
}
function witness(a, wanted) {
  const i = a.findIndex(v => wanted * (v < 0n ? -1 : v > 0n ? 1 : 0) > 0);
  assert(i >= 0);
  const totalAbs = a.reduce((s, v) => s + abs(v), 0n);
  const u = a.map((_, k) => rat(k === i ? totalAbs + 1n : 1n));
  const durations = u.map(inv);
  assert.equal(sign(value(a, durations)), wanted);
  // Every positive duration vector gives ONE common strictly increasing clock.
  const clock = [rat(0)];
  for (const d of durations) clock.push(add(clock.at(-1), d));
  // A fixed total duration alone leaves the sign classification unchanged.
  const fixedTotal = rat(11), factor = mul(fixedTotal, inv(sum(durations)));
  const normalized = durations.map(d => mul(factor, d));
  assert(eq(sum(normalized), fixedTotal));
  assert.equal(sign(value(a, normalized)), wanted);
  return { durations: durations.map(str), clock: clock.map(str), value: str(value(a, durations)),
    fixed_total_11_durations: normalized.map(str), fixed_total_value: str(value(a, normalized)) };
}
let controls = 0;
for (let i = -2; i <= 2; i++) for (let j = -2; j <= 2; j++)
  for (let k = -2; k <= 2; k++) for (let l = -2; l <= 2; l++) {
    const a = [i, j, k, l].map(BigInt), c = classify(a);
    if (a.some(x => x < 0n)) witness(a, -1);
    if (a.some(x => x > 0n)) witness(a, 1);
    if (c === 'identically_zero') assert.equal(sign(value(a, a.map(() => rat(1)))), 0);
    controls++;
  }
const disp = [2, -1, 3, -2, 4, -1, 5];
const coeff = coefficients(disp, [0, 2], [4, 6]);
assert.deepEqual(coeff, [2n, 0n, -3n, 0n, -4n, 0n, 5n]);
const timedControl = { displacements: disp, b: [0, 2], c: [4, 6], coefficients: coeff.map(String),
  status: 'algebraic carrier only; no legal Chan atom or structural divergence qualification',
  classification: classify(coeff), negative: witness(coeff, -1), positive: witness(coeff, 1) };
assert.deepEqual(coefficients([2, -3], [0, 0], [1, 1]), [0n, 0n]);
assert.throws(() => value([1n], [rat(0)]));
function box(a, lower, upper) {
  assert(a.length === lower.length && a.length === upper.length);
  for (let i = 0; i < a.length; i++) assert(lower[i].n > 0n && sign(sub(upper[i], lower[i])) >= 0);
  const minimum = sum(a.map((v, i) => mul(rat(v), inv(v >= 0n ? upper[i] : lower[i]))));
  const maximum = sum(a.map((v, i) => mul(rat(v), inv(v >= 0n ? lower[i] : upper[i]))));
  return { minimum: str(minimum), maximum: str(maximum), robust_strict_weak: sign(maximum) < 0 };
}
const boundedControl = box([-2n, 1n], [rat(1), rat(2)], [rat(2), rat(3)]);
assert.deepEqual(boundedControl, { minimum: '-5/3', maximum: '-1/2', robust_strict_weak: true });

// Bind author/正文/inline-annotation checks to exact local source lines.
const sourceSpecs = [
  ['docs/chanlun/text/blog/037-第37课.md', [12, 16, 18, 20, 22, 40], [16, 18, 20, 22], 40],
  ['docs/chanlun/text/blog/043-第43课.md', [12, 14, 18, 26, 28, 32, 34, 38, 42, 44, 48, 52], [18, 26, 28, 32, 34, 42, 44], 52],
  ['docs/chanlun/text/blog/044-第44课.md', [12, 20, 22, 24, 26, 28, 30, 66], [20, 22, 24, 26, 30], 66],
  ['docs/chanlun/text/blog/061-第61课.md', [12, 26, 56], [26], 56],
  ['docs/chanlun/text/blog/084-第84课.md', [12, 52, 54, 56, 60, 68], [52, 54, 56, 60], 68],
  ['.chanlun/definitions/beichi.md', [263,264,265,266,267,268,295,296,297,298,299,301,313,314,315,316,317,319,320,321,322,323,338,342,346,348,356,357,365,370,372,510,512,517,523,524,526,527,537,547,563,565,568,569,666,673,674,675,676,677,678,679,680,681], [], null]
];
const sourceEvidence = sourceSpecs.map(([rel, included, adopted, boundary]) => {
  const file = path.join(root, rel), lines = fs.readFileSync(file, 'utf8').split('\n');
  if (boundary !== null) {
    assert(lines[11].includes('作者：缠中说禅'));
    assert(lines[boundary - 1].includes('↑正文'));
    for (const n of adopted) {
      assert(n < boundary);
      assert(!/[（(](?:娇注|娇加|娇|注)[:：]/.test(lines[n - 1]));
    }
  }
  return { path: rel, sha256: hash(file), author_line: boundary ? 12 : null,
    body_boundary: boundary, adopted, lines: included.map(n => ({ n, text: lines[n - 1] })),
    note: rel.includes('043-') ? '14 is compiler note; 38 and 48 have inline notes and are captured but not adopted as whole-line author quotations.' : rel.includes('044-') ? '28 is compiler note, excluded.' : 'Only adopted blog lines are used as author assertions; doctrine is local adjudication, not original quotation.' };
});
const result = { schema: 'stage59-dynamics-development-check-v1', input: { path: path.relative(root, modelFile), sha256: hash(modelFile) },
  endAt: 6, knownAt: 8, quotes: states, crossing, coefficient_controls: controls, timedControl, boundedControl,
  tests: { zero_duration_rejected: true, singleton_coalescence: true, normalized_total_signs_checked: true },
  evidence_status: 'author exact finite self-check plus ordinary proof in report; not independent review or full original model proof' };
fs.writeFileSync(path.join(here, 'control-result.json'), JSON.stringify(result, null, 2) + '\n');
fs.writeFileSync(path.join(here, 'source-evidence.json'), JSON.stringify(sourceEvidence, null, 2) + '\n');
console.log(JSON.stringify({ status: 'PASS', crossing: crossing.map(x => ({ n: x.n, spans: x.spans, ordered_pairs: x.ordered_nonoverlapping_pairs, qualifying_pairs: x.qualifying_pairs })), coefficient_controls: controls, boundedControl, model_sha256: hash(modelFile) }, null, 2));
