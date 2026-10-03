// #1467：有界运行及原始收据；不打印行情或私密检查点。
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
export const PROFILE = 'rd-q-native-available/1';
export const sha = b => createHash('sha256').update(b).digest('hex');
export async function runCase({ binary, input, dir, checkpoint, lines = [], stop }) {
  const repo = await fs.realpath(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..'));
  const parent = await fs.realpath(path.dirname(dir));
  if (parent === repo || parent.startsWith(repo + path.sep)) throw Error('raw evidence must remain outside repository');
  await fs.mkdir(dir, { mode: 0o700 });
  const rows = path.join(dir, 'rows.jsonl'), reportFile = path.join(dir, 'report.json');
  const args = [checkpoint ? 'resume' : 'replay', input, checkpoint ?? path.join(dir, 'checkpoints'),
    rows, reportFile, lines.length ? lines.join(',') : '-'];
  if (stop !== undefined) args.push(String(stop));
  args.push(PROFILE);
  const start = Date.now();
  const run = await new Promise((resolve, reject) => {
    const p = spawn(binary, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', timedOut = false;
    const timer = setTimeout(() => { timedOut = true; p.kill('SIGKILL'); }, 60000);
    p.stdout.setEncoding('utf8'); p.stderr.setEncoding('utf8');
    p.stdout.on('data', s => { stdout += s; }); p.stderr.on('data', s => { stderr += s; });
    p.on('error', e => { clearTimeout(timer); reject(e); });
    p.on('close', (exitCode, signal) => { clearTimeout(timer); resolve({ stdout, stderr, exitCode, signal, timedOut }); });
  });
  await fs.writeFile(path.join(dir, 'stdout.log'), run.stdout, { flag: 'wx', mode: 0o600 });
  await fs.writeFile(path.join(dir, 'stderr.log'), run.stderr, { flag: 'wx', mode: 0o600 });
  let report = null;
  try { report = JSON.parse(await fs.readFile(reportFile, 'utf8')); } catch (e) { if (e.code !== 'ENOENT') throw e; }
  const receipt = { command: [binary, ...args], exit_code: run.exitCode, signal: run.signal,
    timed_out: run.timedOut, elapsed_ms: Date.now() - start, stderr: run.stderr.trim(),
    executable_sha256: sha(await fs.readFile(binary)), input_sha256: sha(await fs.readFile(input)),
    report, rows, dir };
  await fs.writeFile(path.join(dir, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  return receipt;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [binary, input, dir, cps = '-'] = process.argv.slice(2);
  if (!binary || !input || !dir) throw Error('usage: node stage26_run.mjs BINARY INPUT NEW_DIR [CP_LINES]');
  const r = await runCase({ binary, input, dir, lines: cps === '-' ? [] : cps.split(',').map(Number) });
  console.log(JSON.stringify({ exit_code: r.exit_code, timed_out: r.timed_out, elapsed_ms: r.elapsed_ms,
    model_events: r.report?.model_events, completed: r.report?.completed_blocks, barriers: r.report?.barriers, receipt: path.join(dir, 'receipt.json') }));
  if (r.exit_code !== 0 || r.timed_out) process.exitCode = 1;
}
