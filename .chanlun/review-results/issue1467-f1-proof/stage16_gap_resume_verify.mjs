// #1467/#1468：针对未补齐序号缺口中的持久化恢复，不重复Stage16已通过矩阵。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [binaryArg, inputArg, baselineArg, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 6, 'usage: node stage16_gap_resume_verify.mjs BINARY INPUT BASELINE_DIR NEW_EXTERNAL_OUTPUT');
const binary = fs.realpathSync(binaryArg), input = fs.realpathSync(inputArg);
const baselineDir = fs.realpathSync(baselineArg), output = path.resolve(outputArg);
const repo = fs.realpathSync(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..'));
const parent = fs.realpathSync(path.dirname(output));
assert(parent !== repo && !parent.startsWith(repo + path.sep));
const hash = data => createHash('sha256').update(data).digest('hex');
const source = fs.readFileSync(input);
const expectedSource = '8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd';
assert.equal(hash(source), expectedSource);
const executableHash = hash(fs.readFileSync(binary));
const baseline = JSON.parse(fs.readFileSync(path.join(baselineDir, 'report.json'), 'utf8'));
assert.equal(baseline.executable_sha256, executableHash);
assert.equal(baseline.source_sha256, expectedSource);
assert.equal(baseline.state_sha256, 'c17603a9b18a1d885d580663ff360560ebdcfc451f09700963af334687d2b1cf');
fs.mkdirSync(output);
const receipts = [];
function lines(bytes) {
  const out = bytes.toString('utf8').split('\n'); assert.equal(out.pop(), ''); return out;
}
function run(name, mode, data, checkpoint) {
  const dir = path.join(output, name); fs.mkdirSync(dir);
  const rows = path.join(dir, 'rows.jsonl'), reportFile = path.join(dir, 'report.json');
  const cp = mode === 'replay' ? path.join(dir, 'checkpoints') : checkpoint;
  const args = [mode, data, cp, rows, reportFile];
  const result = spawnSync(binary, args, { encoding: 'utf8', timeout: 45000, maxBuffer: 1024 * 1024 });
  fs.writeFileSync(path.join(dir, 'stdout.log'), result.stdout || '', { flag: 'wx' });
  fs.writeFileSync(path.join(dir, 'stderr.log'), result.stderr || '', { flag: 'wx' });
  const report = fs.existsSync(reportFile) ? JSON.parse(fs.readFileSync(reportFile, 'utf8')) : null;
  const receipt = { name, command: [binary, ...args], exit_code: result.status,
    signal: result.signal, error: result.error ? String(result.error) : null, report };
  receipts.push(receipt);
  fs.writeFileSync(path.join(dir, 'receipt.json'), JSON.stringify(receipt, null, 2), { flag: 'wx' });
  assert.equal(receipt.error, null); assert.equal(receipt.signal, null); assert.equal(receipt.exit_code, 0);
  return { receipt, rows, checkpoints: cp };
}
try {
  const raw = lines(source), removed = raw[1000], space = removed.indexOf(' ');
  const message = JSON.parse(removed.slice(space + 1));
  assert.equal(message.sequence, 19247052250); assert.equal(message.type, 'received');
  const captured = raw[10004].slice(0, raw[10004].indexOf(' '));
  const delayed = raw.filter((_, i) => i !== 1000);
  delayed.splice(10004, 0, captured + removed.slice(space));
  const delayedFile = path.join(output, 'delayed-across-checkpoint.ndjson');
  fs.writeFileSync(delayedFile, delayed.join('\n') + '\n', { flag: 'wx' });
  const continuous = run('continuous', 'replay', delayedFile);
  const allLines = lines(fs.readFileSync(continuous.rows)), all = allLines.map(x => JSON.parse(x));
  const baseBytes = fs.readFileSync(path.join(baselineDir, 'rows.jsonl'));
  assert.equal(hash(baseBytes), baseline.trace_file_sha256);
  const baseLines = lines(baseBytes);
  assert.deepEqual(allLines.slice(0, 1000), baseLines.slice(0, 1000));
  assert(all.slice(1000, 10004).every(r => r.status === 'sequence_gap' && r.quote === null));
  const repaired = all[10004];
  assert.equal(repaired.status, 'ready'); assert.equal(repaired.transition, 'catchup_batch');
  assert.equal(repaired.applied_messages, 9005); assert.equal(repaired.delayed_applied_messages, 9004);
  assert.deepEqual(repaired.quote, JSON.parse(baseLines[10004]).quote);
  assert.equal(continuous.receipt.report.book_sha256, baseline.book_sha256);
  const cpFile = path.join(continuous.checkpoints, 'middle.json');
  const cp = JSON.parse(fs.readFileSync(cpFile, 'utf8')); // Only metadata read; never reserialize numeric state.
  assert.equal(cp.state.lines, 10000);
  assert.equal(Object.keys(cp.state.pending).length, 9000);
  let chain = hash(Buffer.from('coinbase-causal-view/1'));
  for (const row of allLines.slice(0, 10000)) chain = hash(Buffer.concat([Buffer.from(chain), Buffer.from(row + '\n')]));
  assert.equal(chain, cp.trace_prefix_sha256);
  const resumed = run('resumed', 'resume', delayedFile, cpFile);
  assert.equal(fs.readFileSync(resumed.rows, 'utf8'), allLines.slice(10000).join('\n') + '\n');
  assert.equal(resumed.receipt.report.state_sha256, continuous.receipt.report.state_sha256);
  assert.equal(resumed.receipt.report.trace_chain_sha256, continuous.receipt.report.trace_chain_sha256);
  assert.equal(resumed.receipt.report.book_sha256, baseline.book_sha256);
  assert.equal(all.filter(r => r.new_trade !== null).length, 205);
  assert.equal(hash(fs.readFileSync(input)), expectedSource);
  const summary = { all_passed: true, scope: 'synthetic delayed-delivery control over the fixed sample',
    source_sha256: expectedSource, delayed_source_sha256: hash(fs.readFileSync(delayedFile)),
    executable_sha256: executableHash, runner_sha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))),
    missing_sequence: '19247052250', original_line: 1001, inserted_after_original_line: 10005,
    repair_capture_line: 10005, repair_capture_time: captured, checkpoint_line: 10000,
    checkpoint_pending: 9000, delayed_applied_at_repair: 9004, total_applied_at_repair: 9005,
    original_prefix_unchanged: true, gap_quotes_withheld: true, restored_suffix_equal: true,
    final_trace_chain_equal: true, final_book_equal_original: true, original_unchanged: true, receipts };
  fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ all_passed: true, checkpoint_pending: 9000, restored_suffix_equal: true, summary: path.join(output, 'summary.json') }));
} catch (error) {
  fs.writeFileSync(path.join(output, 'failure.json'), JSON.stringify({ all_passed: false, error: String(error), receipts }, null, 2), { flag: 'wx' });
  throw error;
}
