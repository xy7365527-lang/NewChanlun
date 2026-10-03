// #1467/#1468：固定探索样本的恢复/负控制；无网络请求，原始数据只读。
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [binaryArg, inputArg, outputArg] = process.argv.slice(2);
assert(process.argv.length === 5, 'usage: node stage9_verify.mjs BINARY INPUT NEW_EXTERNAL_OUTPUT_DIR');
const binary = await fs.realpath(binaryArg);
const input = await fs.realpath(inputArg);
const output = path.resolve(outputArg);
const repo = await fs.realpath(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..'));
const parent = await fs.realpath(path.dirname(output));
assert(parent !== repo && !parent.startsWith(repo + path.sep), 'raw evidence output must be outside this repository');
assert((await fs.stat(input)).size <= 100 * 1024 * 1024, 'input exceeds byte budget');
const bytes = await fs.readFile(input);
const digest = data => createHash('sha256').update(data).digest('hex');
const expectedSource = '8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd';
assert.equal(digest(bytes), expectedSource, 'this verification is bound to the frozen one-minute sample');
await fs.mkdir(output); // 不复用已有目录，不覆盖前次证据。
const binaryHash = digest(await fs.readFile(binary));
const results = [];
async function execute(name, args) {
  const reportPath = path.join(output, name + '.json');
  const command = [binary, ...args, reportPath];
  const result = await new Promise((resolve, reject) => {
    const child = spawn(binary, command.slice(1), { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', timedOut = false;
    const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, 45000);
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', data => { stdout += data; });
    child.stderr.on('data', data => { stderr += data; });
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      resolve({ exit_code: code, signal, timed_out: timedOut, stdout, stderr });
    });
  });
  await fs.writeFile(path.join(output, name + '.stdout'), result.stdout, { flag: 'wx' });
  await fs.writeFile(path.join(output, name + '.stderr'), result.stderr, { flag: 'wx' });
  let report = null;
  try { report = JSON.parse(await fs.readFile(reportPath, 'utf8')); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  const receipt = { name, command, exit_code: result.exit_code, signal: result.signal,
    timed_out: result.timed_out, stderr: result.stderr.trim(), report };
  results.push(receipt);
  await fs.writeFile(path.join(output, name + '.receipt.json'), JSON.stringify(receipt, null, 2), { flag: 'wx' });
  assert.equal(receipt.timed_out, false, name + ': timeout is not a semantic result');
  assert.equal(receipt.signal, null, name + ': unexpected signal');
  return receipt;
}

try {
  const checkpoints = path.join(output, 'checkpoints');
  const baseline = await execute('continuous', ['audit', input, checkpoints]);
  assert.equal(baseline.exit_code, 0);
  assert.equal(baseline.report.sample_sequence_and_reconstruction_pass, true);
  const resumeNames = ['empty', 'before-snapshot', 'after-snapshot', 'middle'];
  const resumed = await Promise.all(resumeNames.map(async name => {
    const cp = path.join(checkpoints, name + '.json');
    const receipt = await execute('resume-' + name, ['resume', input, cp]);
    receipt.checkpoint_sha256 = digest(await fs.readFile(cp));
    return receipt;
  }));
  for (const receipt of resumed) {
    assert.equal(receipt.exit_code, 0, receipt.name);
    const { restored_from_line: _, ...actual } = receipt.report;
    const { restored_from_line: __, ...expected } = baseline.report;
    assert.deepEqual(actual, expected, receipt.name + ': differs from continuous execution');
  }

  const lines = bytes.toString('utf8').split('\n');
  const removedIndex = 1000;
  const removed = JSON.parse(lines[removedIndex].slice(lines[removedIndex].indexOf(' ') + 1));
  assert.equal(removed.type, 'received');
  assert.equal(removed.sequence, 19247052250);
  const missing = path.join(output, 'missing-sequence.ndjson');
  await fs.writeFile(missing, lines.filter((_, i) => i !== removedIndex).join('\n'), { flag: 'wx' });
  const duplicateIndex = 500;
  const separator = lines[duplicateIndex].indexOf(' ');
  const duplicate = JSON.parse(lines[duplicateIndex].slice(separator + 1));
  assert.equal(duplicate.sequence, 19247051750);
  assert(['buy', 'sell'].includes(duplicate.side));
  duplicate.side = duplicate.side === 'buy' ? 'sell' : 'buy';
  const conflicting = path.join(output, 'conflicting-sequence.ndjson');
  const changedLines = [...lines];
  changedLines.splice(duplicateIndex + 1, 0, lines[duplicateIndex].slice(0, separator + 1) + JSON.stringify(duplicate));
  await fs.writeFile(conflicting, changedLines.join('\n'), { flag: 'wx' });
  const negative = await Promise.all([
    execute('source-mismatch', ['resume', missing, path.join(checkpoints, 'after-snapshot.json')]),
    execute('conflicting-sequence', ['audit', conflicting, path.join(output, 'conflicting-checkpoints')]),
    execute('missing-sequence', ['audit', missing, path.join(output, 'missing-checkpoints')]),
  ]);
  for (const receipt of negative) assert.equal(receipt.exit_code, 1, receipt.name);
  assert.match(negative[0].stderr, /checkpoint\/source binding mismatch/);
  assert.equal(negative[0].report, null);
  assert.match(negative[1].stderr, /same sequence with conflicting payload/);
  assert.equal(negative[1].report, null);
  assert.match(negative[2].stderr, /sample contains unresolved sequence\/reconstruction scope/);
  assert.equal(negative[2].report.sample_sequence_and_reconstruction_pass, false);
  assert.equal(negative[2].report.sequence_gaps, 1);
  assert.equal(negative[2].report.frontier, removed.sequence - 1);
  assert(negative[2].report.pending_at_end > 0);
  assert.equal(digest(await fs.readFile(input)), expectedSource, 'original source changed');
  const summary = { schema: 'stage9-recovery-negative-controls/1', all_passed: true,
    scope: 'fixed sample only; same implementation recovery; no independent semantic oracle',
    source_sha256: expectedSource, binary_sha256: binaryHash,
    runner_sha256: digest(await fs.readFile(fileURLToPath(import.meta.url))),
    perturbations: { deleted_original_line: 1001, deleted_sequence: removed.sequence,
      duplicate_after_original_line: 501, duplicated_sequence: duplicate.sequence,
      changed_field: 'side', missing_file_sha256: digest(await fs.readFile(missing)),
      conflicting_file_sha256: digest(await fs.readFile(conflicting)) },
    results: results.sort((a, b) => a.name.localeCompare(b.name)), original_unchanged: true };
  await fs.writeFile(path.join(output, 'summary.json'), JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ all_passed: true, continuous: 1, recovery: 4,
    negative: 3, final_state_sha256: baseline.report.state_sha256,
    summary: path.join(output, 'summary.json') }));
} catch (error) {
  await fs.writeFile(path.join(output, 'failure.json'), JSON.stringify({
    all_passed: false, error: String(error), results }, null, 2), { flag: 'wx' });
  throw error;
}
