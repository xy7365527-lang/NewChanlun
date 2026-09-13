#!/usr/bin/env node
// #1372：真实 HTTP 证据采集；生产 Client 决定提交，raw 命令只校验公共头和响应身份。
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const net = require('node:net');
const crypto = require('node:crypto');
const {performance} = require('node:perf_hooks');
const API = require('../browser/tb01c-client.js');
const {makeHooks} = require('./test_tb01c_browser.cjs');

const LIMITS = Object.freeze({commandMs: 30000, responseBytes: 8 * 1024 * 1024,
  totalBytes: 64 * 1024 * 1024, inputBytes: 1024 * 1024, headerBytes: 16384,
  clients: 4, queue: 8, requests: 4096});
const SOURCE_PATHS = [__filename, path.resolve(__dirname, '../browser/tb01c-client.js'),
  path.resolve(__dirname, '../browser/index.html'), path.join(__dirname, 'test_tb01c_browser.cjs')];
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
const utc = () => new Date().toISOString();
const need = (condition, message) => { if (!condition) throw new Error(message); };
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const same = (a, b) => API.canonical(a) === API.canonical(b);
class RecordingError extends Error {
  constructor(error) { super('证据介质写入失败：' + String(error.message || error), {cause: error}); }
}
function record(action) {
  try { return action(); } catch (error) { throw new RecordingError(error); }
}
const identity = value => {
  need(object(value) && typeof value.session_id === 'string' && value.session_id.length > 0,
    '缺少会话身份');
  decimal(value.session_generation, true);
  return {session_id: value.session_id, session_generation: value.session_generation};
};
function decimal(value, positive = false) {
  need(typeof value === 'string' && /^(0|[1-9][0-9]{0,18})$/.test(value), 'wire 整数必须为规范文本');
  const n = BigInt(value);
  need(n >= (positive ? 1n : 0n) && n <= 9223372036854775807n, 'wire 整数超出 i64');
  return n;
}
function safeName(value) {
  need(typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/.test(value),
    'id/client 只允许 1–64 位 ASCII 字母、数字、下划线和连字符，首位须为字母或数字');
  return value;
}
function parseBase(source) {
  const url = new URL(source);
  need(url.protocol === 'http:' && ['127.0.0.1', '[::1]'].includes(url.hostname)
    && !url.username && !url.password && url.pathname === '/' && !url.search && !url.hash,
  'base-url 必须是无凭证、路径或查询的显式 loopback HTTP origin');
  return url;
}
function sourceHashes() {
  return Object.fromEntries(SOURCE_PATHS.map(file => [file, hash(fs.readFileSync(file))]));
}
function writeJSON(file, value) {
  record(() => fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n', {flag: 'wx'}));
}
function artifact(file, bytes) {
  record(() => fs.writeFileSync(file, bytes, {flag: 'wx'}));
  return {path: file, bytes: bytes.length, sha256: hash(bytes)};
}
function checkCommand(command) {
  need(object(command), '命令必须是 JSON 对象');
  safeName(command.id); safeName(command.client);
  const fields = {load: ['mode', 'generation', 'slowPageDelayMs'], watch: [],
    rawWatch: ['cursor', 'client_id', 'max_batches'], snapshotToken: ['token']}[command.op];
  need(fields, 'op 必须为 load/watch/rawWatch/snapshotToken');
  need(Object.keys(command).every(key => ['id', 'client', 'op', ...fields].includes(key)), '命令有未知字段');
  if (command.op === 'load') {
    need(command.mode === undefined || ['AsKnown', 'RecomputedWithRevision'].includes(command.mode), '历史模式无效');
    if (command.generation !== undefined && command.generation !== null) decimal(command.generation);
    if (command.mode === 'AsKnown') decimal(command.generation);
    if (command.slowPageDelayMs !== undefined) need(Number.isSafeInteger(command.slowPageDelayMs)
      && command.slowPageDelayMs >= 0 && command.slowPageDelayMs < LIMITS.commandMs, 'slowPageDelayMs 越界');
  } else if (command.op === 'rawWatch') {
    identity(command.cursor); decimal(command.max_batches, true);
    need(typeof command.client_id === 'string' && command.client_id.length > 0
      && Buffer.byteLength(command.client_id) <= 1024, 'client_id 必须为有界具名文本');
  } else if (command.op === 'snapshotToken') identity(command.token);
  return command;
}
function parseHTTP(raw) {
  const end = raw.indexOf('\r\n\r\n');
  need(end >= 0 && end + 4 <= LIMITS.headerBytes, 'HTTP 头截断或超过界限');
  const lines = raw.subarray(0, end).toString('latin1').split('\r\n');
  const status = /^HTTP\/1\.[01] ([1-5][0-9]{2}) [^\r\n]*$/.exec(lines.shift());
  need(status, 'HTTP 状态行无效');
  const headers = Object.create(null);
  for (const line of lines) {
    const match = /^([!#$%&'*+.^_`|~0-9A-Za-z-]+):[ \t]*([^\x00-\x08\x0A-\x1F\x7F]*)$/.exec(line);
    need(match, 'HTTP 头字段无效');
    const key = match[1].toLowerCase();
    need(!Object.hasOwn(headers, key), 'HTTP 头字段重复');
    headers[key] = match[2].trim();
  }
  need(!Object.hasOwn(headers, 'transfer-encoding'), '采集器要求 Q 固定 Content-Length，不接受 Transfer-Encoding');
  need(!headers['content-encoding'] || headers['content-encoding'] === 'identity', '不接受压缩响应');
  need(typeof headers['content-length'] === 'string' && /^[0-9]+$/.test(headers['content-length']), '缺少明确响应长度');
  const body = raw.subarray(end + 4);
  need(BigInt(headers['content-length']) === BigInt(body.length), 'HTTP 正文截断或含额外帧');
  return {status: Number(status[1]), headers, body};
}

class Capture {
  constructor(base, output) {
    this.base = parseBase(base);
    this.output = path.resolve(output);
    fs.mkdirSync(this.output); // 已存在即拒绝；每个 helper 使用新的独立目录。
    fs.mkdirSync(path.join(this.output, 'commands'));
    fs.mkdirSync(path.join(this.output, 'rejections'));
    this.sources = sourceHashes(); this.hooks = makeHooks();
    need(same(this.sources, sourceHashes()), '初始化期间生产源码变化');
    this.clients = new Map(); this.committedFiles = new Map();
    this.queue = []; this.pumping = null; this.active = null;
    this.rejection = 0n;
    writeJSON(path.join(this.output, 'SESSION.json'), {schema: 'tb01c-http-capture/1', started_at: utc(),
      base_url: this.base.origin, node: process.version, pid: process.pid, limits: LIMITS,
      production_limits: API.DEFAULTS, sources: this.sources,
      timing_scope: '30s 从命令出队开始，包含 discovery/页间延迟/正式读取；队列等待另记',
      byte_scope: '8MiB 每份原始 HTTP 响应（含头）；64MiB 每命令保存的请求与响应字节合计',
      claim: '只返回采集/严格提交状态，不自动裁定目标业务行为通过；raw 不安装完整状态'});
  }
  check(context) {
    need(!context.abort.signal.aborted, '命令已取消');
    need(performance.now() < context.deadline, '命令超过 30s 总期限，未验证');
  }
  async delay(context, milliseconds, signal) {
    this.check(context);
    need(milliseconds < context.deadline - performance.now(), '页间延迟将超出命令总期限');
    await new Promise((resolve, reject) => {
      const abort = () => { cleanup(); reject(new Error('页间延迟已取消')); };
      const cleanup = () => { clearTimeout(timer); context.abort.signal.removeEventListener('abort', abort);
        signal?.removeEventListener('abort', abort); };
      const timer = setTimeout(() => { cleanup(); resolve(); }, milliseconds);
      context.abort.signal.addEventListener('abort', abort, {once: true});
      signal?.addEventListener('abort', abort, {once: true});
      if (signal?.aborted || context.abort.signal.aborted) abort();
    });
    this.check(context);
  }
  async fetch(context, input, options = {}) {
    this.check(context);
    const url = new URL(input, this.base);
    need(url.origin === this.base.origin && !url.search && !url.hash && !url.username && !url.password,
      '请求必须留在声明的同一 loopback origin');
    const method = options.method || 'GET';
    need((url.pathname === '/api/state' && method === 'GET') ||
      (['/api/v2/snapshot', '/api/v2/watch'].includes(url.pathname) && method === 'POST'), '请求路由或方法未授权');
    need(++context.requests <= LIMITS.requests, '命令请求数量达到上限');
    const body = Buffer.from(options.body || '', 'utf8');
    need(body.length <= LIMITS.inputBytes, '请求正文超过 1MiB');
    const request = Buffer.concat([Buffer.from(`${method} ${url.pathname} HTTP/1.0\r\nHost: ${this.base.host}\r\n`
      + 'Connection: close\r\nAccept: application/json\r\nAccept-Encoding: identity\r\n'
      + (method === 'POST' ? `Content-Type: application/json\r\nContent-Length: ${body.length}\r\n` : '') + '\r\n'), body]);
    need(context.bytes + request.length <= LIMITS.totalBytes, '命令总字节达到上限');
    const prefix = path.join(context.directory, 'http', String(context.requests).padStart(5, '0'));
    const meta = {url: url.href, method, started_at: utc(), started_monotonic_ms: performance.now(),
      request: artifact(prefix + '-request.http', request), request_body: artifact(prefix + '-request.body', body),
      request_write_completed: false, response_eof: false, http_complete: false, response_observed_bytes: 0,
      response_captured_bytes: 0, source: 'direct-loopback-net-socket; no proxy or redirect'};
    context.bytes += request.length;
    const metaFile = prefix + '-metadata.json';
    writeJSON(metaFile, {...meta, observation: 'pending'});
    const responseFile = prefix + '-response.http';
    const responseFD = record(() => fs.openSync(responseFile, 'wx'));
    const chunks = []; let header = Buffer.alloc(0), headerDone = false;
    try {
      if (url.pathname === '/api/v2/snapshot') {
        if (context.snapshotRequests++ > 0 && context.slowPageDelayMs) {
          meta.page_delay_ms = context.slowPageDelayMs;
          await this.delay(context, context.slowPageDelayMs, options.signal);
        }
      }
      this.check(context);
      await new Promise((resolve, reject) => {
        let settled = false;
        const socket = new net.Socket();
        const finish = error => {
          if (settled) return;
          settled = true; clearTimeout(timer);
          options.signal?.removeEventListener('abort', abort);
          context.abort.signal.removeEventListener('abort', abort);
          socket.destroy(); error ? reject(error) : resolve();
        };
        const abort = () => finish(new Error('命令或生产 Client 的读取期限已取消请求'));
        const timer = setTimeout(() => finish(new Error('HTTP 读取达到命令总期限')), Math.max(1, context.deadline - performance.now()));
        options.signal?.addEventListener('abort', abort, {once: true});
        context.abort.signal.addEventListener('abort', abort, {once: true});
        socket.on('error', finish);
        socket.on('close', () => { if (!settled) finish(new Error('HTTP 连接关闭，未见响应 EOF')); });
        socket.on('connect', () => {
          if (settled) return;
          meta.connected_monotonic_ms = performance.now();
          socket.write(request, error => {
            if (error) return finish(error);
            meta.request_write_completed = true; meta.request_written_monotonic_ms = performance.now();
          });
        });
        socket.on('data', chunk => {
          if (settled) return;
          try {
            this.check(context);
            meta.first_response_monotonic_ms ??= performance.now();
            meta.response_observed_bytes += chunk.length;
            const room = Math.min(LIMITS.responseBytes - meta.response_captured_bytes, LIMITS.totalBytes - context.bytes);
            const captured = chunk.subarray(0, Math.max(0, room));
            if (captured.length) {
              record(() => fs.writeSync(responseFD, captured)); chunks.push(captured);
              context.bytes += captured.length; meta.response_captured_bytes += captured.length;
            }
            need(captured.length === chunk.length, '响应或命令达到字节上限；只保存有界前缀，未验证');
            if (!headerDone) {
              header = Buffer.concat([header, captured]);
              const end = header.indexOf('\r\n\r\n');
              need(end >= 0 ? end + 4 <= LIMITS.headerBytes : header.length <= LIMITS.headerBytes, 'HTTP 头超过 16KiB');
              if (end >= 0) { headerDone = true; header = Buffer.alloc(0); }
            }
          } catch (error) { finish(error); }
        });
        socket.on('end', () => { meta.response_eof = true; meta.response_eof_monotonic_ms = performance.now(); finish(); });
        if (options.signal?.aborted || context.abort.signal.aborted) abort();
        else socket.connect({host: this.base.hostname.replace(/^\[|\]$/g, ''), port: Number(this.base.port || '80')});
      });
      this.check(context);
      const parsed = parseHTTP(Buffer.concat(chunks));
      meta.status = parsed.status; meta.headers = parsed.headers; meta.http_complete = true;
      meta.response_body = artifact(prefix + '-response.body', parsed.body);
      return new Response(parsed.body.length ? parsed.body : null, {status: parsed.status, headers: parsed.headers});
    } catch (error) {
      meta.error = String(error.message || error); throw error;
    } finally {
      record(() => fs.closeSync(responseFD));
      meta.finished_at = utc(); meta.finished_monotonic_ms = performance.now();
      meta.response = {path: responseFile, bytes: meta.response_captured_bytes, sha256: hash(Buffer.concat(chunks))};
      record(() => fs.writeFileSync(metaFile, JSON.stringify({...meta, observation: 'finished'}, null, 2) + '\n'));
      context.http.push(metaFile);
    }
  }
  client(name) {
    if (!this.clients.has(name)) {
      need(this.clients.size < LIMITS.clients, '具名 client 数量达到 4 个上限');
      this.clients.set(name, new API.Client({hooks: this.hooks,
        fetch: (input, options) => this.fetch(this.active, input, options),
        onCommit: (_candidate, event) => { this.active.commitEvent = event; },
        limits: {timeoutMs: LIMITS.commandMs, replyBytes: LIMITS.responseBytes, totalBytes: LIMITS.totalBytes}}));
    }
    return this.clients.get(name);
  }
  async raw(client, command, context) {
    const requestIdentity = identity(command.op === 'rawWatch' ? command.cursor : command.token);
    const payload = command.op === 'rawWatch' ? {op: 'watch', cursor: command.cursor,
      max_batches: command.max_batches, client_id: command.client_id} : {op: 'snapshot', snapshot_token: command.token};
    const budget = client.begin();
    const reply = await client.request(requestIdentity, payload, budget);
    if (command.op === 'rawWatch') {
      const page = reply.payload;
      need(same(identity(reply), identity(page.observed_head)), 'raw Watch 公共头与 observed_head 身份不符');
      if (page.gap === null) need(same(identity(reply), requestIdentity), 'raw Watch 无 Gap 却换会话身份');
      else {
        need(object(page.gap) && same(page.gap.requested_identity, command.cursor)
          && same(page.gap.rebuild, page.observed_head), 'raw Gap 未绑定原 cursor 或重建身份');
        const changed = !same(identity(reply), requestIdentity);
        need(changed ? page.gap.reason === 'session_identity_changed' :
          ['delivery_retention_gap', 'client_backlog_overflow'].includes(page.gap.reason), 'raw Gap 的跨身份原因不符');
      }
    } else {
      const page = reply.payload;
      need(same(identity(reply), requestIdentity) && same(identity(page.fixed_cut), requestIdentity), 'raw Snapshot 与 token 身份不符');
      const offset = decimal(command.token.offset);
      need(offset <= BigInt(Number.MAX_SAFE_INTEGER), 'token offset 超过客户端精确索引界限');
      await client.validateToken(command.token, page.fixed_cut, page.cut_projection_digest, Number(offset));
      need(page.offset === command.token.offset, 'raw Snapshot 返回错 offset');
    }
    client.check(budget); this.check(context);
    return reply;
  }
  async execute(item) {
    const {command, directory, enqueuedAt, enqueuedMono} = item;
    const start = performance.now();
    const context = {directory, deadline: start + LIMITS.commandMs, abort: new AbortController(),
      requests: 0, bytes: 0, http: [], snapshotRequests: 0, slowPageDelayMs: command.slowPageDelayMs || 0};
    this.active = context;
    const result = {id: command.id, client: command.client, op: command.op, enqueued_at: enqueuedAt,
      started_at: utc(), queue_ms: start - enqueuedMono, result: 'not_verified', http: context.http,
      semantic_acceptance: 'not_evaluated', full_state_installed: false};
    let client, before, fatalError;
    try {
      need(same(this.sources, sourceHashes()), '生产源码与该 helper 固定版本不符，停止采集');
      client = this.client(command.client); before = client.committed;
      result.before = before ? {cursor: before.cursor, projection_sha256: await API.sha256(before.projection),
        candidate: this.committedFiles.get(command.client) || null} : null;
      let discovery;
      if (command.op === 'load') {
        const response = await this.fetch(context, '/api/state');
        need(response.ok, '发现请求不是成功 HTTP 响应');
        discovery = API.parse(new TextDecoder('utf-8', {fatal: true}).decode(await response.arrayBuffer()));
        need(discovery.ok === true, '发现回执未成功'); identity(discovery.cut);
        // GET 仅发现 v2 身份；完整 cut 始终由生产 Client 固定分页取得。
      }
      this.check(context);
      const remainingMs = Math.floor(context.deadline - performance.now());
      need(remainingMs > 0, '已用尽命令总期限');
      client.limits.timeoutMs = remainingMs;
      if (command.op === 'load' || command.op === 'watch') {
        const candidate = command.op === 'load' ? await client.load(discovery, command.mode || 'RecomputedWithRevision',
          command.generation ?? null) : await client.watch();
        need(client.committed === candidate, '生产 Client 没有安装返回候选');
        this.check(context); need(context.requests > 0, '没有真实 HTTP 响应，不能形成采集结论');
        result.candidate = artifact(path.join(directory, 'committed-candidate.json'), Buffer.from(JSON.stringify(candidate, null, 2) + '\n'));
        this.committedFiles.set(command.client, result.candidate);
        result.result = 'captured_validated_commit'; result.full_state_installed = true;
        result.commit_event = context.commitEvent; result.cursor = candidate.cursor;
      } else {
        const reply = await this.raw(client, command, context);
        result.envelope = artifact(path.join(directory, 'validated-envelope.json'), Buffer.from(API.canonical(reply) + '\n'));
        result.result = 'captured_envelope_validated';
        result.validation_scope = 'production Client.request 完整公共头/因果/hash + 请求响应身份；raw 业务内容留待逐字段核对';
        need(client.committed === before, 'raw 请求意外改变已提交状态');
      }
    } catch (error) {
      result.error = String(error.stack || error); client?.cancel();
      result.committed_changed = !!client && client.committed !== before;
      if (client?.committed) result.retained_cursor = client.committed.cursor;
      if (error instanceof RecordingError) {
        fatalError = error; this.fatal = error;
        result.result = 'fatal_recording_failure';
        result.full_state_installed = result.committed_changed;
        result.committed_but_not_archived = result.committed_changed;
        // 私有提交已发生时不伪称回退；终止本helper，排队命令不再执行。
        result.queued_commands_not_executed = this.queue.map(item => item.command.id);
        this.queue.length = 0;
      }
    } finally {
      context.abort.abort(); this.active = null;
      if (client) client.limits.timeoutMs = LIMITS.commandMs;
      result.finished_at = utc(); result.elapsed_ms = performance.now() - start;
      result.recorded_wire_bytes = context.bytes; result.requests = context.requests;
      result.sources = this.sources; result.sources_unchanged = same(this.sources, sourceHashes());
    }
    const file = path.join(directory, 'RESULT.json'); writeJSON(file, result);
    await this.reply({id: command.id, result: result.result, path: file});
    if (fatalError) throw fatalError;
  }
  reply(value) {
    return new Promise((resolve, reject) => process.stdout.write(JSON.stringify(value) + '\n', error => error ? reject(error) : resolve()));
  }
  async rejectFrame(raw, error, complete = true) {
    const directory = path.join(this.output, 'rejections', 'rejection-' + (++this.rejection).toString());
    fs.mkdirSync(directory);
    const input = artifact(path.join(directory, 'input.jsonl'), raw);
    const file = path.join(directory, 'RESULT.json');
    writeJSON(file, {result: 'rejected_command', error: String(error.message || error), input, input_complete: complete, at: utc()});
    await this.reply({id: null, result: 'rejected_command', path: file});
  }
  async enqueue(raw) {
    if (this.fatal) throw this.fatal;
    let command, directory;
    try {
      command = checkCommand(JSON.parse(new TextDecoder('utf-8', {fatal: true}).decode(raw)));
      need(this.queue.length < LIMITS.queue, '待执行命令队列达到 8 条上限');
      directory = path.join(this.output, 'commands', command.id);
      try { fs.mkdirSync(directory); } catch (error) {
        if (error.code === 'EEXIST') throw new Error('命令 id 已存在，不能重复执行');
        throw new RecordingError(error);
      }
      record(() => fs.mkdirSync(path.join(directory, 'http')));
      artifact(path.join(directory, 'input.jsonl'), raw);
    } catch (error) {
      if (error instanceof RecordingError) { this.fatal = error; throw error; }
      await this.rejectFrame(raw, error); return;
    }
    this.queue.push({command, directory, enqueuedAt: utc(), enqueuedMono: performance.now()});
    if (!this.pumping) {
      this.pumping = (async () => {
        try { while (this.queue.length) await this.execute(this.queue.shift()); }
        finally { this.pumping = null; }
      })();
      // 记录/输出介质错误不能继续形成成功证据；取消本 helper，保留已落原件。
      this.pumping.catch(error => { process.stderr.write(String(error.stack || error) + '\n');
        this.fatal = error; this.queue.length = 0;
        process.exitCode = 1; this.active?.abort.abort(); process.stdin.destroy(); });
    }
  }
  async run(input = process.stdin) {
    let parts = [], length = 0, oversized = false;
    for await (const chunk of input) {
      let offset = 0;
      while (offset < chunk.length) {
        const end = chunk.indexOf(10, offset), final = end >= 0;
        const piece = chunk.subarray(offset, final ? end + 1 : chunk.length);
        const keep = Math.min(piece.length, Math.max(0, LIMITS.inputBytes - length));
        if (keep) parts.push(piece.subarray(0, keep));
        length += keep; if (keep < piece.length) oversized = true;
        offset += piece.length;
        if (final) {
          const raw = Buffer.concat(parts, length);
          if (oversized) await this.rejectFrame(raw, new Error('输入单帧超过 1MiB，仅保存前缀'), false);
          else await this.enqueue(raw);
          parts = []; length = 0; oversized = false;
        }
      }
    }
    if (length || oversized) await this.rejectFrame(Buffer.concat(parts, length), new Error('stdin EOF 前缺少 JSONL 换行，未执行'), false);
    if (this.pumping) await this.pumping;
    if (this.fatal) throw this.fatal;
  }
}

async function main() {
  const args = process.argv.slice(2);
  need(args.length === 4 && args[0] === '--base-url' && args[2] === '--output-dir',
    'usage: node tb01c_http_capture.cjs --base-url http://127.0.0.1:PORT --output-dir NEW_DIRECTORY');
  const capture = new Capture(args[1], args[3]);
  await capture.run();
}
module.exports = {Capture, LIMITS, parseBase, checkCommand, parseHTTP};
if (require.main === module) main().catch(error => { process.stderr.write(String(error.stack || error) + '\n'); process.exitCode = 1; });
