// #1467/#1468：发布后L1时钟、精确投影、确认来源与分epoch恢复验证。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [binaryArg, inputArg, gapInputArg, baselineArg, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 7, 'usage: node stage17_verify.mjs BINARY VIEWS GAP_VIEWS BASELINE NEW_EXTERNAL_OUTPUT');
const binary = await fs.realpath(binaryArg), input = await fs.realpath(inputArg), gapInput = await fs.realpath(gapInputArg);
const baselineDir = await fs.realpath(baselineArg), output = path.resolve(outputArg);
const packageDir = path.dirname(fileURLToPath(import.meta.url));
const repo = await fs.realpath(path.resolve(packageDir, '../../..'));
const parent = await fs.realpath(path.dirname(output));
assert(parent !== repo && !parent.startsWith(repo + path.sep));
const hash = b => createHash('sha256').update(b).digest('hex');
const binaryHash = hash(await fs.readFile(binary));
const inputBytes = await fs.readFile(input), gapBytes = await fs.readFile(gapInput);
assert.equal(hash(inputBytes), 'a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5');
const origin = JSON.parse(await fs.readFile(path.join(packageDir, 'stage17-core-origin.json'), 'utf8'));
assert.equal(hash(await fs.readFile(path.join(packageDir, origin.file))), origin.sha256);
await fs.mkdir(output);
const receipts = [];
function lines(b) { const a = b.toString('utf8').split('\n'); assert.equal(a.pop(), ''); return a; }
async function exists(p) { try { await fs.access(p); return true; } catch (e) { if (e.code === 'ENOENT') return false; throw e; } }
async function execute(name, mode, data, cp, stop, delta = '1000000') {
  const dir = path.join(output, name); await fs.mkdir(dir);
  const rows = path.join(dir, 'rows.jsonl'), reportFile = path.join(dir, 'report.json');
  const cpArg = mode === 'run' ? path.join(dir, 'checkpoints') : cp;
  const args = [mode, data, cpArg, rows, reportFile, delta]; if (stop !== undefined) args.push(String(stop));
  const result = await new Promise((resolve, reject) => {
    const child = spawn(binary, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', timedOut = false;
    const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, 60000);
    child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
    child.stdout.on('data', x => { stdout += x; }); child.stderr.on('data', x => { stderr += x; });
    child.on('error', e => { clearTimeout(timer); reject(e); });
    child.on('close', (code, signal) => { clearTimeout(timer); resolve({ code, signal, timedOut, stdout, stderr }); });
  });
  await fs.writeFile(path.join(dir, 'stdout.log'), result.stdout, { flag: 'wx' });
  await fs.writeFile(path.join(dir, 'stderr.log'), result.stderr, { flag: 'wx' });
  const report = await exists(reportFile) ? JSON.parse(await fs.readFile(reportFile, 'utf8')) : null;
  const receipt = { name, command: [binary, ...args], exit_code: result.code, signal: result.signal,
    timed_out: result.timedOut, stderr: result.stderr.trim(), report };
  receipts.push(receipt); await fs.writeFile(path.join(dir, 'receipt.json'), JSON.stringify(receipt, null, 2), { flag: 'wx' });
  assert.equal(result.signal, null); assert.equal(result.timedOut, false);
  return { ...receipt, rows, checkpoints: cpArg };
}
async function pool(jobs) {
  const settled = [];
  for (let i = 0; i < jobs.length; i += 2) settled.push(...await Promise.allSettled(jobs.slice(i, i + 2).map(f => f())));
  for (const r of settled) if (r.status === 'rejected') throw r.reason;
  return settled.map(r => r.value);
}
function inspect(viewBytes, outputBytes, expectedReport) {
  const views = lines(viewBytes).map(x => JSON.parse(x)), texts = lines(outputBytes), out = texts.map(x => JSON.parse(x));
  assert.equal(views.length, out.length);
  const epochs = new Map(), idsW = new Set(), idsM = new Set();
  let observations = 0, weighted = 0, midpoint = 0, nonTradeQuantityConfirmations = 0;
  for (let i = 0; i < out.length; i++) {
    const v = views[i], r = out[i]; assert.equal(r.source_line, v.line); assert.equal(r.status, v.status);
    assert.equal(r.capture_time_lower_bound, v.available_at_capture);
    if (v.status !== 'ready') {
      assert.equal(r.epoch, null); assert.equal(r.new_observation, null);
      assert.deepEqual(r.weighted_confirmed, []); assert.deepEqual(r.midpoint_confirmed, []);
      assert.equal(r.weighted_active, null); assert.equal(r.midpoint_active, null);
      continue;
    }
    if (!epochs.has(r.epoch)) epochs.set(r.epoch, []);
    const ps = epochs.get(r.epoch);
    if (r.new_observation) {
      const o = r.new_observation; assert.equal(o.index, ps.length); assert.equal(o.source_line, v.line);
      assert.deepEqual(o.quote, v.quote); assert.equal(o.capture, v.available_at_capture);
      const b = BigInt(v.quote.bid_price), a = BigInt(v.quote.ask_price);
      const qb = BigInt(v.quote.bid_quantity), qa = BigInt(v.quote.ask_quantity), d = qb + qa;
      const w = (2n * (a * qb + b * qa) + d) / (2n * d), m = (a + b + 1n) / 2n;
      assert.equal(o.weighted, String(w)); assert.equal(o.midpoint, String(m)); assert(b <= w && w <= a);
      if (ps.length) assert.notDeepEqual(o.quote, ps.at(-1).quote);
      else assert.equal(o.kind, 'initial');
      ps.push(o); observations++;
      if (o.kind === 'quantity_only') {
        assert.deepEqual(r.midpoint_confirmed, []);
        if (o.native_type !== 'match') nonTradeQuantityConfirmations += r.weighted_confirmed.length;
      }
    } else assert.deepEqual(v.quote, ps.at(-1).quote);
    for (const [label, values, ids] of [['weighted', r.weighted_confirmed, idsW], ['midpoint', r.midpoint_confirmed, idsM]]) {
      for (const u of values) {
        assert.equal(u.epoch, r.epoch); const key = `${u.epoch}:${u.ordinal}`;
        assert(!ids.has(key)); ids.add(key);
        for (const p of [u.start, u.end]) {
          const o = ps[p.observation]; assert(o);
          assert.equal(p.source_line, o.source_line); assert.equal(p.capture_time_lower_bound, o.capture);
          assert.equal(p.frontier, o.frontier); assert.equal(p.value_ticks, o[label]);
        }
        assert(u.start.source_line < u.end.source_line && u.end.source_line < u.confirmed_at.source_line);
        assert(u.active_since.source_line <= u.end.source_line);
        assert.equal(u.confirmed_at.source_line, v.line);
        assert.equal(u.confirmed_at.capture_time_lower_bound, v.available_at_capture);
        const start = BigInt(u.start.value_ticks), end = BigInt(u.end.value_ticks);
        assert(u.direction === 'up' ? end - start >= 1000000n : start - end >= 1000000n);
        if (label === 'weighted') weighted++; else midpoint++;
      }
    }
    assert.equal(r.weighted_active.provisional, true); assert.equal(r.midpoint_active.provisional, true);
  }
  assert.equal(observations, expectedReport.observations);
  assert.equal(weighted, expectedReport.weighted_completed); assert.equal(midpoint, expectedReport.midpoint_completed);
  assert.equal(observations, expectedReport.reference_prefix_checks);
  assert.equal(expectedReport.eof_does_not_confirm_tail, true);
  return { out, texts, stats: { observations, weighted, midpoint, epochs: epochs.size, nonTradeQuantityConfirmations } };
}
try {
  const report = JSON.parse(await fs.readFile(path.join(baselineDir, 'report.json'), 'utf8'));
  assert.equal(report.executable_sha256, binaryHash); assert.equal(report.source_sha256, hash(inputBytes));
  assert.equal(report.delta_ticks, '1000000');
  const baselineBytes = await fs.readFile(path.join(baselineDir, 'rows.jsonl'));
  assert.equal(hash(baselineBytes), report.output_file_sha256);
  const baseline = inspect(inputBytes, baselineBytes, report);
  assert(baseline.stats.weighted > 0 && baseline.stats.midpoint > 0);
  assert(baseline.stats.nonTradeQuantityConfirmations > 0);
  const recovery = await pool([339, 340, 1000, 10000].map(n => async () => {
    const run = await execute(`resume-${n}`, 'resume', input, path.join(baselineDir, 'checkpoints', `line-${n}.json`));
    assert.equal(run.exit_code, 0); assert.equal(run.report.state_sha256, report.state_sha256);
    assert.equal(run.report.engine_sha256, report.engine_sha256);
    assert.equal(run.report.output_chain_sha256, report.output_chain_sha256);
    assert.equal(await fs.readFile(run.rows, 'utf8'), baseline.texts.slice(n).join('\n') + '\n');
    return { source_line: n, input_replay_recovery_equal: true };
  }));
  const sourceLines = lines(inputBytes);
  const prefixes = await pool([339, 340, 1000, 10000].map(n => async () => {
    const p = path.join(output, `views-prefix-${n}.jsonl`);
    await fs.writeFile(p, sourceLines.slice(0, n).join('\n') + '\n', { flag: 'wx' });
    const run = await execute(`prefix-${n}`, 'run', p, null, n);
    assert.equal(run.exit_code, 0);
    assert.equal(await fs.readFile(run.rows, 'utf8'), baseline.texts.slice(0, n).join('\n') + '\n');
    return { source_line: n, physical_prefix_equal: true };
  }));
  const gap = await execute('long-gap', 'run', gapInput);
  assert.equal(gap.exit_code, 0); const gapOutput = await fs.readFile(gap.rows);
  const gapCheck = inspect(gapBytes, gapOutput, gap.report);
  assert.equal(gap.report.epochs_started, 2); assert.equal(gap.report.ended_epochs, 1);
  assert.equal(gapCheck.out[1000].ended_scope.tail_status, 'unfinished_on_domain_exit');
  assert(gapCheck.out.slice(1000, 10004).every(r => r.epoch === null && !r.new_observation));
  assert.equal(gapCheck.out[10004].new_observation.kind, 'initial');
  assert.equal(gapCheck.out[10004].new_observation.index, 0); assert.equal(gapCheck.out[10004].epoch, 1);
  for (const r of gapCheck.out.slice(10004)) for (const u of [...r.weighted_confirmed, ...r.midpoint_confirmed]) assert(u.start.source_line >= 10005);
  const gapResume = await execute('gap-resume', 'resume', gapInput, path.join(gap.checkpoints, 'line-10000.json'));
  assert.equal(gapResume.exit_code, 0); assert.equal(gapResume.report.state_sha256, gap.report.state_sha256);
  assert.equal(gapResume.report.output_chain_sha256, gap.report.output_chain_sha256);
  assert.equal(await fs.readFile(gapResume.rows, 'utf8'), gapCheck.texts.slice(10000).join('\n') + '\n');
  const badDelta = await execute('delta-mismatch', 'resume', input, path.join(baselineDir, 'checkpoints', 'line-1000.json'), undefined, '1000001');
  assert.equal(badDelta.exit_code, 1); assert(badDelta.stderr.includes('checkpoint delta mismatch'));
  assert.equal(badDelta.report, null); assert.equal(await exists(badDelta.rows), false);
  const extreme = JSON.parse(sourceLines[339]); extreme.line = 1;
  extreme.quote = { bid_price: '9223372036854775805', ask_price: '9223372036854775807',
    bid_quantity: '9223372036854775807', ask_quantity: '9223372036854775807', scale: 100000000 };
  const extremeInput = path.join(output, 'overflow-view.jsonl');
  await fs.writeFile(extremeInput, JSON.stringify(extreme) + '\n', { flag: 'wx' });
  const overflow = await execute('overflow', 'run', extremeInput);
  assert.equal(overflow.exit_code, 1); assert(overflow.stderr.includes('projection rounding overflow'));
  assert.equal(overflow.report, null); assert.equal((await fs.stat(overflow.rows)).size, 0);
  assert.equal(hash(await fs.readFile(input)), report.source_sha256); assert.equal(hash(await fs.readFile(binary)), binaryHash);
  const summary = { all_passed: true, profile: report.profile, delta_ticks: report.delta_ticks,
    baseline_report: report, source_sha256: hash(inputBytes), executable_sha256: binaryHash,
    frozen_constructor: origin, baseline_stats: baseline.stats, recovery, prefixes,
    gap_stats: gapCheck.stats, gap_epoch_boundary: { old_epoch_ends_at: 1001, new_epoch_starts_at: 10005,
      tail_not_sealed: true, cross_gap_units: 0, recovery_equal: true },
    parameter_mismatch_rejected: true, arithmetic_overflow_rejected: true,
    runner_sha256: hash(await fs.readFile(fileURLToPath(import.meta.url))), receipts,
    scope: 'fixed observed-state clock and function-probe delta; no trade baseline A, F2 or predictive value result' };
  await fs.writeFile(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ all_passed: true, baseline: baseline.stats, recovery: 5, prefixes: 4,
    summary: path.join(output, 'summary.json') }));
} catch (error) {
  await fs.writeFile(path.join(output, 'failure.json'), JSON.stringify({ all_passed: false, error: String(error), receipts }, null, 2), { flag: 'wx' });
  throw error;
}
