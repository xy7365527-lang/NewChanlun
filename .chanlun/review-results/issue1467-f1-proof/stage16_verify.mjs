// #1467/#1468：固定真实样本的前缀/恢复/扰动验证；不联网，不输出原消息或订单标识。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [binaryArg, inputArg, outputArg, baselineArg] = process.argv.slice(2);
assert(process.argv.length === 5 || process.argv.length === 6,
  'usage: node stage16_verify.mjs BINARY INPUT NEW_EXTERNAL_OUTPUT [EXISTING_BASELINE_DIR]');
const binary = await fs.realpath(binaryArg);
const input = await fs.realpath(inputArg);
const output = path.resolve(outputArg);
const packageDir = path.dirname(fileURLToPath(import.meta.url));
const repo = await fs.realpath(path.resolve(packageDir, '../../..'));
const parent = await fs.realpath(path.dirname(output));
assert(parent !== repo && !parent.startsWith(repo + path.sep), 'raw evidence must stay outside repository');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const expectedSource = '8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd';
const expectedState = 'c17603a9b18a1d885d580663ff360560ebdcfc451f09700963af334687d2b1cf';
const source = await fs.readFile(input);
assert.equal(digest(source), expectedSource);
const binaryHash = digest(await fs.readFile(binary));
const origin = JSON.parse(await fs.readFile(path.join(packageDir, 'stage16-core-origin.json'), 'utf8'));
const code = await fs.readFile(path.join(packageDir, 'coinbase_causal_view.rs'));
assert.equal(digest(code.subarray(0, origin.prefix_bytes)), origin.prefix_sha256);
assert.equal(digest(await fs.readFile(path.join(packageDir, origin.upstream_file))), origin.upstream_sha256);
await fs.mkdir(output);
const results = [];

function linesOf(bytes) {
  const lines = bytes.toString('utf8').split('\n');
  assert.equal(lines.pop(), '', 'fixture must end with a newline');
  return lines;
}
async function exists(p) {
  try { await fs.access(p); return true; }
  catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}
async function execute(name, mode, sourcePath, checkpointPath, stop) {
  const dir = path.join(output, name);
  await fs.mkdir(dir);
  const rows = path.join(dir, 'rows.jsonl');
  const reportPath = path.join(dir, 'report.json');
  const checkpoints = mode === 'replay' ? path.join(dir, 'checkpoints') : checkpointPath;
  const args = [mode, sourcePath, checkpoints, rows, reportPath];
  if (stop !== undefined) args.push(String(stop));
  const started = performance.now();
  const execution = await new Promise((resolve, reject) => {
    const child = spawn(binary, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', timeout = false;
    const timer = setTimeout(() => { timeout = true; child.kill('SIGKILL'); }, 45000);
    child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
    child.stdout.on('data', x => { stdout += x; }); child.stderr.on('data', x => { stderr += x; });
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('close', (exitCode, signal) => { clearTimeout(timer); resolve({ exitCode, signal, timeout, stdout, stderr }); });
  });
  await fs.writeFile(path.join(dir, 'stdout.log'), execution.stdout, { flag: 'wx' });
  await fs.writeFile(path.join(dir, 'stderr.log'), execution.stderr, { flag: 'wx' });
  const report = await exists(reportPath) ? JSON.parse(await fs.readFile(reportPath, 'utf8')) : null;
  const receipt = { name, command: [binary, ...args], exit_code: execution.exitCode,
    signal: execution.signal, timed_out: execution.timeout, elapsed_seconds: (performance.now() - started) / 1000,
    stderr: execution.stderr.trim(), report, rows, checkpoints };
  results.push(receipt);
  await fs.writeFile(path.join(dir, 'receipt.json'), JSON.stringify(receipt, null, 2), { flag: 'wx' });
  assert.equal(receipt.timed_out, false, name + ': timeout is not evidence');
  assert.equal(receipt.signal, null, name + ': unexpected signal');
  return receipt;
}
async function pool(jobs) {
  const outputs = [];
  for (let i = 0; i < jobs.length; i += 2) {
    const batch = await Promise.allSettled(jobs.slice(i, i + 2).map(f => f()));
    outputs.push(...batch);
  }
  for (const item of outputs) if (item.status === 'rejected') throw item.reason;
  return outputs.map(item => item.value);
}

try {
  let baselineDir;
  if (baselineArg) baselineDir = await fs.realpath(baselineArg);
  else {
    const run = await execute('baseline', 'replay', input);
    assert.equal(run.exit_code, 0);
    baselineDir = path.dirname(run.rows);
  }
  const baselineReport = JSON.parse(await fs.readFile(path.join(baselineDir, 'report.json'), 'utf8'));
  assert.equal(baselineReport.schema, 'coinbase-causal-view/1');
  assert.equal(baselineReport.source_sha256, expectedSource);
  assert.equal(baselineReport.executable_sha256, binaryHash);
  assert.equal(baselineReport.state_sha256, expectedState);
  assert.equal(baselineReport.final_ready, true);
  const baselineBytes = await fs.readFile(path.join(baselineDir, 'rows.jsonl'));
  assert.equal(digest(baselineBytes), baselineReport.trace_file_sha256);
  const baselineLines = linesOf(baselineBytes);
  const rows = baselineLines.map(x => JSON.parse(x));
  assert.equal(rows.length, 20868);
  let chain = digest(Buffer.from('coinbase-causal-view/1'));
  const prefixChains = [chain];
  for (let i = 0; i < rows.length; i++) {
    chain = digest(Buffer.concat([Buffer.from(chain), Buffer.from(baselineLines[i] + '\n')]));
    prefixChains.push(chain);
    assert.equal(rows[i].line, i + 1);
    assert.equal(rows[i].product_id, 'BTC-USD');
    assert(!baselineLines[i].includes('client_oid') && !baselineLines[i].includes('order_id'));
    if (rows[i].status !== 'ready') assert.equal(rows[i].quote, null);
    if (rows[i].quote) {
      for (const key of ['bid_price', 'ask_price', 'bid_quantity', 'ask_quantity'])
        assert.match(rows[i].quote[key], /^\d+$/);
      assert(BigInt(rows[i].quote.bid_price) < BigInt(rows[i].quote.ask_price));
    }
    if (rows[i].new_trade) assert.equal(rows[i].new_trade.known_at_line, i + 1);
  }
  assert.equal(chain, baselineReport.trace_chain_sha256);
  assert(rows.slice(0, 339).every(r => r.status === 'awaiting_snapshot' && r.quote === null));
  assert.equal(rows[339].status, 'ready');
  assert.equal(rows[339].transition, 'snapshot_batch');
  assert.equal(rows[339].applied_messages, 43);
  assert.equal(rows[339].delayed_applied_messages, 43);
  assert.equal(rows[339].frontier, '19247051589');
  assert.equal(rows.filter(r => r.new_trade !== null).length, 205);

  const checkpointNames = ['empty', 'before-snapshot', 'after-snapshot', 'middle'];
  const recoveries = await pool(checkpointNames.map(name => async () => {
    const cp = path.join(baselineDir, 'checkpoints', name + '.json');
    const meta = JSON.parse(await fs.readFile(cp, 'utf8'));
    const start = meta.state.lines;
    assert.equal(meta.trace_prefix_sha256, prefixChains[start]);
    const run = await execute('resume-' + name, 'resume', input, cp);
    assert.equal(run.exit_code, 0);
    assert.equal(run.report.state_sha256, expectedState);
    assert.equal(run.report.trace_chain_sha256, baselineReport.trace_chain_sha256);
    assert.equal(run.report.book_sha256, baselineReport.book_sha256);
    assert.equal(await fs.readFile(run.rows, 'utf8'), baselineLines.slice(start).join('\n') + '\n');
    return { name, from_line: start, suffix_equal: true, trace_chain_equal: true };
  }));

  const sourceLines = linesOf(source);
  const prefixes = await pool([339, 340, 1000, 10000].map(n => async () => {
    const prefixInput = path.join(output, 'prefix-' + n + '.ndjson');
    await fs.writeFile(prefixInput, sourceLines.slice(0, n).join('\n') + '\n', { flag: 'wx' });
    const run = await execute('prefix-' + n, 'replay', prefixInput, null, n);
    assert.equal(run.exit_code, 0);
    assert.equal(run.report.lines_consumed, n);
    assert.equal(run.report.trace_chain_sha256, prefixChains[n]);
    assert.equal(await fs.readFile(run.rows, 'utf8'), baselineLines.slice(0, n).join('\n') + '\n');
    assert.notEqual(run.report.source_sha256, expectedSource);
    return { lines: n, physical_input_prefix_equal: true, final_ready: run.report.final_ready };
  }));

  const removeIndex = 1000;
  const original = sourceLines[removeIndex];
  const space = original.indexOf(' ');
  const native = JSON.parse(original.slice(space + 1));
  assert.equal(native.sequence, 19247052250); assert.equal(native.type, 'received');
  const gapLines = sourceLines.filter((_, i) => i !== removeIndex);
  const gapInput = path.join(output, 'gap.ndjson');
  await fs.writeFile(gapInput, gapLines.join('\n') + '\n', { flag: 'wx' });
  const repairedLines = [...gapLines];
  const delayedCapture = sourceLines[1004].slice(0, sourceLines[1004].indexOf(' '));
  repairedLines.splice(1004, 0, delayedCapture + original.slice(space));
  const repairedInput = path.join(output, 'delayed.ndjson');
  await fs.writeFile(repairedInput, repairedLines.join('\n') + '\n', { flag: 'wx' });
  const [gap, delayed] = await pool([
    () => execute('gap', 'replay', gapInput),
    () => execute('delayed', 'replay', repairedInput),
  ]);
  assert.equal(gap.exit_code, 2); assert.equal(gap.report.final_ready, false);
  assert.equal(gap.report.final_frontier, '19247052249');
  assert.equal(gap.report.pending_at_end, 19867);
  const gapRows = linesOf(await fs.readFile(gap.rows)).map(x => JSON.parse(x));
  assert(gapRows.slice(1000).every(r => r.status === 'sequence_gap' && r.quote === null));
  assert.equal(delayed.exit_code, 0); assert.equal(delayed.report.final_ready, true);
  assert.equal(delayed.report.book_sha256, baselineReport.book_sha256);
  const delayedTexts = linesOf(await fs.readFile(delayed.rows));
  const delayedRows = delayedTexts.map(x => JSON.parse(x));
  assert.deepEqual(delayedTexts.slice(0, 1000), baselineLines.slice(0, 1000));
  assert(delayedRows.slice(1000, 1004).every(r => r.status === 'sequence_gap' && r.quote === null));
  const repair = delayedRows[1004];
  assert.equal(repair.status, 'ready'); assert.equal(repair.transition, 'catchup_batch');
  assert.equal(repair.native_type, 'received'); assert.equal(repair.applied_messages, 5);
  assert.equal(repair.delayed_applied_messages, 4); assert.equal(repair.frontier, rows[1004].frontier);
  assert.deepEqual(repair.quote, rows[1004].quote);

  const cpPath = path.join(baselineDir, 'checkpoints', 'after-snapshot.json');
  const cpText = await fs.readFile(cpPath, 'utf8');
  const wrongBinary = path.join(output, 'wrong-binary.json');
  const damagedState = path.join(output, 'damaged-state.json');
  const badBinaryText = cpText.replace(/"executable_sha256":"[a-f0-9]{64}"/, '"executable_sha256":"' + '0'.repeat(64) + '"');
  const badStateText = cpText.replace('"state":{"lines":340,', '"state":{"lines":341,');
  assert.notEqual(badBinaryText, cpText); assert.notEqual(badStateText, cpText);
  await fs.writeFile(wrongBinary, badBinaryText, { flag: 'wx' });
  await fs.writeFile(damagedState, badStateText, { flag: 'wx' });
  const bindingCases = await pool([
    () => execute('wrong-source', 'resume', gapInput, cpPath),
    () => execute('wrong-binary', 'resume', input, wrongBinary),
    () => execute('damaged-state', 'resume', input, damagedState),
  ]);
  const expectedErrors = ['checkpoint/source binding mismatch', 'checkpoint/executable binding mismatch', 'checkpoint state checksum mismatch'];
  for (let i = 0; i < bindingCases.length; i++) {
    assert.equal(bindingCases[i].exit_code, 1);
    assert(bindingCases[i].stderr.includes(expectedErrors[i]));
    assert.equal(bindingCases[i].report, null);
    assert.equal(await exists(bindingCases[i].rows), false);
  }
  assert.equal(digest(await fs.readFile(input)), expectedSource);
  assert.equal(digest(await fs.readFile(binary)), binaryHash);
  const summary = { schema: 'stage16-causal-view-checks/1', all_passed: true,
    baseline: { directory: baselineDir, reused: Boolean(baselineArg), report: baselineReport },
    recovery: recoveries, physical_prefixes: prefixes,
    gap: { missing_original_line: 1001, missing_sequence: '19247052250', quote_withheld_after_gap: true, pending_at_end: gap.report.pending_at_end },
    delayed: { inserted_after_original_line: 1005, repaired_capture_line: 1005,
      changed_capture_time: delayedCapture, applied: repair.applied_messages, delayed_applied: repair.delayed_applied_messages,
      old_gap_rows_unchanged: true, one_post_catchup_view: true, final_book_equal: true },
    bindings_rejected: expectedErrors, original_source_unchanged: true,
    runner_sha256: digest(await fs.readFile(fileURLToPath(import.meta.url))),
    code_sha256: digest(code), frozen_core: origin, executable_sha256: binaryHash,
    receipts: results.sort((a, b) => a.name.localeCompare(b.name)) };
  await fs.writeFile(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ all_passed: true, recovery: 4, physical_prefixes: 4,
    timing_controls: 2, binding_controls: 3, summary: path.join(output, 'summary.json') }));
} catch (error) {
  await fs.writeFile(path.join(output, 'failure.json'), JSON.stringify({ all_passed: false, error: String(error), receipts: results }, null, 2), { flag: 'wx' });
  throw error;
}
