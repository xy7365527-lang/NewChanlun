// #1467/#1468：导出数值/空侧/交叉报价合同；不重跑已完成的完整样本矩阵。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [binaryArg, inputArg, baselineArg, originalCheckpointsArg, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 7, 'usage: node stage16_quote_contract_verify.mjs BINARY INPUT BASELINE ORIGINAL_CHECKPOINTS NEW_EXTERNAL_OUTPUT');
const binary = fs.realpathSync(binaryArg), input = fs.realpathSync(inputArg);
const baseline = fs.realpathSync(baselineArg), originals = fs.realpathSync(originalCheckpointsArg);
const output = path.resolve(outputArg), parent = fs.realpathSync(path.dirname(output));
const repo = fs.realpathSync(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..'));
assert(parent !== repo && !parent.startsWith(repo + path.sep));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const inputBytes = fs.readFileSync(input);
const sourceHash = '8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd';
assert.equal(hash(inputBytes), sourceHash);
const baselineReport = JSON.parse(fs.readFileSync(path.join(baseline, 'report.json'), 'utf8'));
const binaryHash = hash(fs.readFileSync(binary));
assert.equal(binaryHash, baselineReport.executable_sha256);
const rowBytes = fs.readFileSync(path.join(baseline, 'rows.jsonl'));
assert.equal(hash(rowBytes), baselineReport.trace_file_sha256);
const rows = rowBytes.toString('utf8').trimEnd().split('\n').map(x => JSON.parse(x));
fs.mkdirSync(output);
const checks = [];
try {
  for (const r of rows) {
    assert.equal(r.quote !== null, r.status === 'ready');
    if (r.quote) {
      for (const k of ['bid_price', 'ask_price', 'bid_quantity', 'ask_quantity']) assert(BigInt(r.quote[k]) > 0n);
      assert(BigInt(r.quote.bid_price) < BigInt(r.quote.ask_price));
    }
  }
  function best(levels, buy) {
    const keys = Object.keys(levels); assert(keys.length > 0);
    const selected = keys.reduce((a, b) => (buy ? BigInt(a) > BigInt(b) : BigInt(a) < BigInt(b)) ? a : b);
    // Keys remain exact decimal strings. Refuse unsafe numeric quantities instead of rounding.
    assert(Number.isSafeInteger(levels[selected]) && levels[selected] > 0);
    return [selected, String(levels[selected])];
  }
  for (const name of ['after-snapshot', 'middle']) {
    const cpBytes = fs.readFileSync(path.join(originals, name + '.json'));
    const cp = JSON.parse(cpBytes); assert.equal(cp.source_sha256, sourceHash);
    const [bp, bq] = best(cp.state.bids, true), [ap, aq] = best(cp.state.asks, false);
    assert.deepEqual(rows[cp.state.lines - 1].quote, { bid_price: bp, bid_quantity: bq,
      ask_price: ap, ask_quantity: aq, scale: 100000000 });
    checks.push({ name, line: cp.state.lines, checkpoint_sha256: hash(cpBytes), quote_equal: true });
  }
  const originalLine = inputBytes.toString('utf8').split('\n')[339];
  const split = originalLine.indexOf(' '), captured = originalLine.slice(0, split);
  const snapshot = JSON.parse(originalLine.slice(split + 1));
  assert.equal(snapshot.type, 'full_snapshot');
  for (const [name, expectedStatus] of [['one-sided', 'missing_quote'], ['locked', 'crossed_or_locked_quote']]) {
    const ask = [...snapshot.asks[0]], bid = [...snapshot.bids[0]];
    bid[0] = ask[0];
    const altered = { ...snapshot, bids: name === 'one-sided' ? [] : [bid], asks: [ask] };
    const dir = path.join(output, name); fs.mkdirSync(dir);
    const alteredInput = path.join(dir, 'input.ndjson');
    fs.writeFileSync(alteredInput, captured + ' ' + JSON.stringify(altered) + '\n', { flag: 'wx' });
    const rowFile = path.join(dir, 'rows.jsonl'), reportFile = path.join(dir, 'report.json');
    const args = ['replay', alteredInput, path.join(dir, 'checkpoints'), rowFile, reportFile];
    const result = spawnSync(binary, args, { encoding: 'utf8', timeout: 45000, maxBuffer: 1024 * 1024 });
    fs.writeFileSync(path.join(dir, 'stdout.log'), result.stdout || '', { flag: 'wx' });
    fs.writeFileSync(path.join(dir, 'stderr.log'), result.stderr || '', { flag: 'wx' });
    assert.equal(result.error, undefined); assert.equal(result.signal, null); assert.equal(result.status, 2);
    const out = fs.readFileSync(rowFile, 'utf8').trimEnd().split('\n').map(x => JSON.parse(x));
    assert.equal(out.length, 1); assert.equal(out[0].status, expectedStatus); assert.equal(out[0].quote, null);
    const report = JSON.parse(fs.readFileSync(reportFile)); assert.equal(report.final_ready, false);
    checks.push({ name, command: [binary, ...args], exit_code: result.status,
      status: expectedStatus, quote_withheld: true, input_sha256: hash(fs.readFileSync(alteredInput)) });
  }
  assert.equal(hash(fs.readFileSync(input)), sourceHash);
  const summary = { all_passed: true, all_ready_rows_have_positive_quote: true,
    all_unavailable_rows_have_null_quote: true, source_sha256: sourceHash,
    executable_sha256: binaryHash, runner_sha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))),
    checks, scope: 'old checkpoint export conformance and synthetic invalid-quote controls, not an exchange oracle' };
  fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ all_passed: true, old_checkpoint_quotes: 2, quote_domain_controls: 2, summary: path.join(output, 'summary.json') }));
} catch (error) {
  fs.writeFileSync(path.join(output, 'failure.json'), JSON.stringify({ all_passed: false, error: String(error), checks }, null, 2), { flag: 'wx' });
  throw error;
}
