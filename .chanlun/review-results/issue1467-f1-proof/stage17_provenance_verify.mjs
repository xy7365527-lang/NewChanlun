// #1467/#1468：只读现有输出，补查角色首见、确认与活动端点的来源。
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const [normalArg, gapArg, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 5, 'usage: node stage17_provenance_verify.mjs ROWS GAP_ROWS NEW_REPORT');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
function inspect(rows) {
  const epochs = new Map();
  let activeChecks = 0, completedChecks = 0, endedChecks = 0, last = null;
  const at = (e, index) => {
    const o = e.observations[index]; assert(o, 'missing observation');
    return { observation: index, source_line: o.source_line, capture_time_lower_bound: o.capture };
  };
  const point = (e, p, label, cutoff) => {
    const o = e.observations[p.observation]; assert(o, 'missing endpoint');
    assert.deepEqual(p, { ...at(e, p.observation), frontier: o.frontier, value_ticks: o[label] });
    assert(p.source_line <= cutoff, 'future endpoint');
  };
  for (const row of rows) {
    if (row.ended_scope) {
      assert(last && last.epoch === row.ended_scope.epoch);
      assert.equal(row.ended_scope.tail_status, 'unfinished_on_domain_exit');
      for (const label of ['weighted', 'midpoint']) assert.deepEqual(row.ended_scope[`${label}_active`], last[`${label}_active`]);
      endedChecks++;
    }
    if (row.epoch === null) { last = row; continue; }
    if (!epochs.has(row.epoch)) {
      assert(row.new_observation && row.new_observation.index === 0);
      epochs.set(row.epoch, { observations: [], weighted: { initial: null, prior: null, count: 0 }, midpoint: { initial: null, prior: null, count: 0 } });
    }
    const e = epochs.get(row.epoch), o = row.new_observation;
    if (o) { assert.equal(o.index, e.observations.length); e.observations.push(o); }
    for (const label of ['weighted', 'midpoint']) {
      const s = e[label];
      if (o) {
        const v = BigInt(o[label]);
        if (s.low === undefined || v < s.low) s.low = v;
        if (s.high === undefined || v > s.high) s.high = v;
        if (s.initial === null && s.high - s.low >= BigInt(row.delta_ticks)) s.initial = o.index;
      }
      for (const u of row[`${label}_confirmed`]) {
        assert(o && s.initial !== null);
        assert.equal(u.epoch, row.epoch); assert.equal(u.ordinal, s.count);
        assert.deepEqual(u.confirmed_at, at(e, o.index));
        assert.deepEqual(u.active_since, s.prior ? s.prior.confirmed_at : at(e, s.initial));
        point(e, u.start, label, row.source_line); point(e, u.end, label, row.source_line);
        assert(u.start.source_line < u.end.source_line && u.end.source_line < u.confirmed_at.source_line);
        if (s.prior) { assert.deepEqual(u.start, s.prior.end); assert.notEqual(u.direction, s.prior.direction); }
        s.prior = u; s.count++; completedChecks++;
      }
      const a = row[`${label}_active`];
      assert.equal(a.provisional, true);
      assert.deepEqual(a.active_since, s.prior ? s.prior.confirmed_at : at(e, s.initial ?? 0));
      if (s.initial === null) {
        assert.equal(a.phase, 'undecided'); point(e, a.low, label, row.source_line); point(e, a.high, label, row.source_line);
      } else {
        assert(['up', 'down'].includes(a.phase)); point(e, a.start, label, row.source_line); point(e, a.extreme, label, row.source_line);
        if (s.prior) { assert.deepEqual(a.start, s.prior.end); assert.notEqual(a.phase, s.prior.direction); }
      }
      activeChecks++;
    }
    last = row;
  }
  return { active_checks: activeChecks, completed_checks: completedChecks, ended_checks: endedChecks };
}
const normalBytes = await fs.readFile(normalArg), gapBytes = await fs.readFile(gapArg);
const parse = b => b.toString('utf8').trimEnd().split('\n').map(JSON.parse);
const normal = parse(normalBytes), gap = parse(gapBytes);
const result = { normal: inspect(normal), gap: inspect(gap), negative_controls: [] };
function rejects(name, rows, index, change) {
  const mutant = rows.slice(); mutant[index] = structuredClone(rows[index]); change(mutant[index]);
  assert.throws(() => inspect(mutant), assert.AssertionError);
  result.negative_controls.push({ name, rejected: true });
}
const doneIndex = normal.findIndex(r => r.weighted_confirmed.length > 0);
const activeIndex = normal.findIndex(r => r.weighted_active?.phase === 'up');
rejects('confirmed-observation-index', normal, doneIndex, r => { r.weighted_confirmed[0].confirmed_at.observation++; });
rejects('active-role-first-known', normal, activeIndex, r => { r.weighted_active.active_since.source_line++; });
rejects('active-endpoint-source', normal, activeIndex, r => { r.weighted_active.extreme.capture_time_lower_bound = '2099-01-01T00:00:00Z'; });
rejects('seal-domain-exit-tail', gap, gap.findIndex(r => r.ended_scope), r => { r.ended_scope.tail_status = 'completed'; });
const report = { all_passed: true, ...result, normal_sha256: digest(normalBytes), gap_sha256: digest(gapBytes),
  runner_sha256: digest(await fs.readFile(fileURLToPath(import.meta.url))),
  scope: 'finite stored-output provenance checks; author checker, not independent semantic review' };
await fs.writeFile(path.resolve(outputArg), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify(report));
