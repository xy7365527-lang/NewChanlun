/* #1372：Q v2 严格只读消费者。暂存完整分页/批次，全部通过才安装 cut 与 cursor。 */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.TB01C = api;
})(globalThis, function () {
  "use strict";
  const FAMILIES = ["objects", "withdrawn_objects", "witnesses", "relations", "observations", "raw_history"];
  const ORDER = "s-record-order/1", TOKEN_SCHEMA = "s-snapshot-token/1";
  const SCOPE = {structure: "CompleteCut", economic: "not_started"};
  const HEADER = ["schema_revision", "session_id", "session_generation", "source_namespace", "source_epoch", "message_id", "producer_id", "producer_epoch", "payload_hash", "causal_refs", "payload"];
  const FIXED = ["session_id", "session_generation", "structure_cut", "cut_generation", "catalog_revision", "index_frontier", "profile_id", "profile_hash", "input_frontier", "seq_range", "catalog_run_status", "catalog_evidence", "scope", "rule_revision", "history_mode", "as_of_generation"];
  const TOKEN = ["token_schema", "session_id", "session_generation", "cut_generation", "scope", "history_mode", "structure_cut", "catalog_revision", "index_frontier", "profile_hash", "rule_revision", "order_version", "page_size", "offset", "cut_projection_digest", "issued_capture_digest", "token_checksum"];
  const CURSOR = ["session_id", "session_generation", "catalog_revision", "scope", "after_generation", "base_cut", "index_frontier", "last_seq", "order_version"];
  const PAGE = ["fixed_cut", "catalog", "rows", "counts", "scope", "omissions", "order_version", "offset", "done", "next_token", "cut_projection_digest", "validated_capture_digest", "ok"];
  const WATCH = ["deltas", "next_cursor", "observed_head", "delivery_frontier", "has_more", "gap", "validated_capture_digest", "ok"];
  const DEFAULTS = Object.freeze({pageSize: 31, watchBatches: 4, pollMs: 500, timeoutMs: 30000,
    replyBytes: 8 * 1024 * 1024, totalBytes: 64 * 1024 * 1024, maxRows: 100000, maxPages: 4096});
  const encoder = new TextEncoder();
  const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
  function need(condition, message) { if (!condition) throw new Error(message); }
  function keys(value, expected, name) {
    need(object(value) && Object.keys(value).length === expected.length && expected.every(k => Object.hasOwn(value, k)), name + " 完整字段不符");
  }
  function scalar(value) {
    need(typeof value === "string", "要求 UTF-8 文本");
    for (let i = 0; i < value.length; i++) {
      const code = value.charCodeAt(i);
      if (code >= 0xd800 && code <= 0xdbff) {
        const next = value.charCodeAt(++i);
        need(next >= 0xdc00 && next <= 0xdfff, "孤立 Unicode 高代理项");
      } else need(code < 0xdc00 || code > 0xdfff, "孤立 Unicode 低代理项");
    }
    return value;
  }
  function text(value) { scalar(value); need(value.length > 0, "身份文本为空"); return value; }
  function integer(value, positive = false, frontier = false) {
    need(typeof value === "string" && value.length <= 19 && /^(?:0|[1-9][0-9]*|-1)$/.test(value), "精确整数必须是规范文本");
    const n = BigInt(value);
    need(n >= (frontier ? -1n : positive ? 1n : 0n) && n <= 9223372036854775807n, "精确整数超出 i64 域");
    return n;
  }
  function digest(value) { need(typeof value === "string" && /^[a-f0-9]{64}$/.test(value), "SHA-256 文本不规范"); }
  function utf8Compare(a, b) {
    const aa = encoder.encode(scalar(a)), bb = encoder.encode(scalar(b));
    for (let i = 0; i < Math.min(aa.length, bb.length); i++) if (aa[i] !== bb[i]) return aa[i] < bb[i] ? -1 : 1;
    return Math.sign(aa.length - bb.length);
  }
  function canonical(value) {
    if (value === null || typeof value === "boolean") return JSON.stringify(value);
    if (typeof value === "string") return JSON.stringify(scalar(value));
    if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
    need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
    return "{" + Object.keys(value).sort(utf8Compare).map(k => JSON.stringify(scalar(k)) + ":" + canonical(value[k])).join(",") + "}";
  }
  const equal = (a, b) => canonical(a) === canonical(b);
  const clone = value => parse(canonical(value));
  // JSON.parse 会丢失重复键和整数精度；只有单个字符串 token 借用其转义解码。
  function parse(source) {
    let p = 0;
    function whitespace() { while (/[ \t\r\n]/.test(source[p] || "x")) p++; }
    function string() {
      const start = p++;
      while (p < source.length) {
        const ch = source[p++];
        if (ch === "\\") { p++; continue; }
        if (ch === '"') return scalar(JSON.parse(source.slice(start, p)));
      }
      throw new Error("JSON 字符串截断");
    }
    function value(depth) {
      need(depth <= 128, "JSON 嵌套超过浏览器界限"); whitespace();
      const ch = source[p];
      if (ch === '"') return string();
      if (ch === "{") {
        p++; const out = Object.create(null); whitespace();
        if (source[p] === "}") { p++; return out; }
        while (true) {
          whitespace(); need(source[p] === '"', "JSON 对象键无效");
          const k = string(); need(!Object.hasOwn(out, k), "重复 JSON 键：" + k);
          whitespace(); need(source[p++] === ":", "JSON 缺少冒号"); out[k] = value(depth + 1);
          whitespace(); const end = source[p++]; if (end === "}") return out;
          need(end === ",", "JSON 对象截断或分隔错误");
        }
      }
      if (ch === "[") {
        p++; const out = []; whitespace(); if (source[p] === "]") { p++; return out; }
        while (true) { out.push(value(depth + 1)); whitespace(); const end = source[p++];
          if (end === "]") return out; need(end === ",", "JSON 数组截断或分隔错误"); }
      }
      for (const [word, result] of [["true", true], ["false", false], ["null", null]]) {
        if (source.slice(p, p + word.length) === word) { p += word.length; return result; }
      }
      throw new Error("JSON 非法值；v2 整数必须使用精确文本");
    }
    const result = value(0); whitespace(); need(p === source.length, "JSON 尾部额外内容"); return result;
  }
  async function sha256(value) {
    const crypto = globalThis.crypto || (typeof require === "function" ? require("node:crypto").webcrypto : null);
    need(crypto && crypto.subtle, "当前环境缺少 SHA-256，无法验证");
    const bytes = await crypto.subtle.digest("SHA-256", encoder.encode(canonical(value)));
    return Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, "0")).join("");
  }
  function validateFixed(fixed) {
    keys(fixed, FIXED, "fixed_cut"); text(fixed.session_id); integer(fixed.session_generation, true);
    const gen = integer(fixed.cut_generation); integer(fixed.input_frontier, false, true);
    text(fixed.catalog_revision); text(fixed.rule_revision); scalar(fixed.index_frontier);
    scalar(fixed.profile_id); scalar(fixed.profile_hash);
    need(fixed.structure_cut === "cut-" + fixed.cut_generation && equal(fixed.scope, SCOPE), "固定 cut/scope 不符");
    keys(fixed.seq_range, ["from", "to"], "seq_range"); integer(fixed.seq_range.from); integer(fixed.seq_range.to, false, true);
    need(fixed.seq_range.to === fixed.input_frontier, "固定 cut 输入前沿不符");
    need(fixed.history_mode === "AsKnown" ? fixed.as_of_generation === fixed.cut_generation :
      fixed.history_mode === "RecomputedWithRevision" && fixed.as_of_generation === null, "历史模式与指定 cut 不符");
    if (gen > 0n) need(object(fixed.catalog_evidence) && fixed.catalog_evidence.rule_revision === fixed.rule_revision, "固定 cut 规则修订与目录证据不符");
  }
  function cursorOf(fixed) {
    return {session_id: fixed.session_id, session_generation: fixed.session_generation, catalog_revision: fixed.catalog_revision,
      scope: clone(fixed.scope), after_generation: fixed.cut_generation, base_cut: fixed.structure_cut,
      index_frontier: fixed.index_frontier, last_seq: fixed.input_frontier, order_version: ORDER};
  }
  function identityOf(fixed) { return {session_id: fixed.session_id, session_generation: fixed.session_generation}; }
  function sameIdentity(a, b) { return a.session_id === b.session_id && a.session_generation === b.session_generation; }
  function rowKey(family, record) {
    const fields = family === "objects" || family === "withdrawn_objects" ? ["object_id"] :
      family === "relations" ? ["subject", "relation_type", "object"] :
      family === "raw_history" ? ["identity_key", "revision"] : [family === "witnesses" ? "witness_id" : "observation_id"];
    return fields.map(k => text(record[k]));
  }
  function orderTuple(family, r) {
    const n = k => integer(r[k]), s = k => text(r[k]);
    switch (family) {
      case "objects": return [n("window_start"), n("window_mid"), n("window_end"), s("object_id"), n("object_revision")];
      case "withdrawn_objects": return [n("withdrawn_generation"), s("object_id"), n("object_revision")];
      case "witnesses": return [s("object_id"), n("slot"), s("witness_id")];
      case "relations": return [s("subject"), s("relation_type"), s("object")];
      case "observations": return [s("kind"), r.window_start === null ? -1n : n("window_start"), s("observation_id")];
      case "raw_history": return [n("seq"), s("identity_key"), n("revision")];
      default: throw new Error("未知记录族");
    }
  }
  function compareRows(a, b) {
    const family = FAMILIES.indexOf(a.family) - FAMILIES.indexOf(b.family); if (family) return family;
    const aa = orderTuple(a.family, a.record), bb = orderTuple(b.family, b.record);
    for (let i = 0; i < aa.length; i++) {
      const cmp = typeof aa[i] === "bigint" ? (aa[i] < bb[i] ? -1 : aa[i] > bb[i] ? 1 : 0) : utf8Compare(aa[i], bb[i]);
      if (cmp) return cmp;
    }
    return 0;
  }
  function rowsOf(snapshot) {
    return FAMILIES.flatMap(family => snapshot[family].map(record => ({family, key: rowKey(family, record), record})).sort(compareRows));
  }
  function stateOf(projection) {
    const fixed = projection.fixed_cut, snapshot = {...clone(fixed), generation: fixed.cut_generation};
    delete snapshot.cut_generation;
    for (const family of FAMILIES) snapshot[family] = projection.rows.filter(row => row.family === family).map(row => clone(row.record));
    return {ok: true, catalog: clone(projection.catalog), snapshot,
      cut: {session_id: fixed.session_id, session_generation: fixed.session_generation, generation: fixed.cut_generation,
        structure_cut: fixed.structure_cut, catalog_revision: fixed.catalog_revision, index_frontier: fixed.index_frontier}};
  }
  class Client {
    #committed = null;
    get committed() { return this.#committed; }
    constructor({fetch: fetcher = globalThis.fetch, hooks, onCommit = () => {}, limits = {}}) {
      need(typeof fetcher === "function", "浏览器缺少网络读取能力");
      for (const name of ["validateState", "validateDelta", "comparePublication"]) need(typeof hooks?.[name] === "function", "缺少严格校验 " + name);
      this.fetch = fetcher; this.hooks = hooks; this.onCommit = onCommit; this.limits = {...DEFAULTS, ...limits};
      for (const value of Object.values(this.limits)) need(Number.isSafeInteger(value) && value > 0, "浏览器资源界限无效");
      this.instance = globalThis.crypto?.randomUUID ? globalThis.crypto.randomUUID() :
        (typeof require === "function" ? require("node:crypto").randomUUID() : null);
      need(this.instance, "无法建立独立查询身份"); this.serial = 0n; this.operation = 0;
    }
    begin() { return {operation: ++this.operation, deadline: performance.now() + this.limits.timeoutMs, bytes: 0}; }
    check(budget) {
      need(budget.operation === this.operation, "已被较新读取取代，旧响应不提交");
      need(performance.now() < budget.deadline, "本次读取超过声明总期限，未验证");
    }
    cancel() { this.operation++; }
    async request(identity, payload, budget) {
      this.check(budget); text(identity.session_id); integer(identity.session_generation, true);
      const request = {schema_revision: "s-session/2", ...identityOf(identity), source_namespace: "s-observe/browser",
        source_epoch: this.instance, message_id: "query-" + (++this.serial).toString(), producer_id: "browser:" + this.instance,
        producer_epoch: "1", payload_hash: await sha256(payload), causal_refs: [], payload};
      this.check(budget);
      const abort = new AbortController(), timer = setTimeout(() => abort.abort(), Math.max(1, budget.deadline - performance.now()));
      try {
        const response = await this.fetch("/api/v2/" + payload.op, {method: "POST", headers: {"Content-Type": "application/json"},
          body: canonical(request), signal: abort.signal, cache: "no-store"});
        need(response.body && typeof response.body.getReader === "function", "缺少有限响应字节流");
        const reader = response.body.getReader(), chunks = []; let length = 0;
        try {
          while (true) {
            this.check(budget); const {done, value} = await reader.read(); this.check(budget);
            if (done) break; length += value.byteLength; budget.bytes += value.byteLength;
            need(length <= this.limits.replyBytes && budget.bytes <= this.limits.totalBytes, "读取达到浏览器字节界限，未验证"); chunks.push(value);
          }
        } catch (error) { await reader.cancel().catch(() => {}); throw error; }
        finally { reader.releaseLock(); }
        const bytes = new Uint8Array(length); let offset = 0;
        for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
        const reply = parse(new TextDecoder("utf-8", {fatal: true}).decode(bytes));
        keys(reply, HEADER, "响应公共头");
        need(reply.schema_revision === "s-session/2", "响应协议版本错误");
        for (const k of HEADER.filter(k => !["causal_refs", "payload"].includes(k))) text(reply[k]);
        integer(reply.session_generation, true); integer(reply.producer_epoch, true); digest(reply.payload_hash);
        need(reply.source_namespace === "s-observe/replies" && reply.source_epoch === reply.producer_epoch &&
          reply.producer_id === "Q:" + reply.session_id && object(reply.payload), "响应生产者/来源身份错误");
        const causal = Object.fromEntries(["source_namespace", "source_epoch", "message_id", "payload_hash"].map(k => [k, request[k]]));
        need(equal(reply.causal_refs, [causal]), "响应因果身份不属于本次请求");
        need(await sha256(reply.payload) === reply.payload_hash, "响应 payload SHA-256 不符");
        need(reply.message_id === "reply-" + await sha256([request.source_namespace, request.source_epoch, request.message_id, reply.payload_hash]), "响应消息身份不符");
        this.check(budget);
        need(response.ok && reply.payload.ok === true, "查询未完成：" + (reply.payload.error || response.status));
        return reply;
      } finally { clearTimeout(timer); }
    }
    async validateToken(token, fixed, projectionDigest, offset) {
      keys(token, TOKEN, "分页 token");
      need(token.token_schema === TOKEN_SCHEMA && token.order_version === ORDER && token.page_size === String(this.limits.pageSize) && token.offset === String(offset), "分页 token 页序/资源/schema 不符");
      integer(token.offset); integer(token.page_size, true); digest(token.issued_capture_digest); digest(token.token_checksum);
      for (const key of ["session_id", "session_generation", "cut_generation", "scope", "history_mode", "structure_cut", "catalog_revision", "index_frontier", "profile_hash", "rule_revision"])
        need(equal(token[key], fixed[key]), "分页 token 固定身份不符：" + key);
      need(token.cut_projection_digest === projectionDigest, "分页 token 投影摘要变化");
      const unsigned = {...token}; delete unsigned.token_checksum;
      need(await sha256(unsigned) === token.token_checksum, "分页 token checksum 不符");
    }
    async collect({identity, mode, generation = null, token = null, expectedFixed = null}, budget) {
      const payload = token ? {op: "snapshot", snapshot_token: token} : {op: "snapshot", ...identityOf(identity), scope: SCOPE,
        history_mode: mode, as_of_generation: generation, page_size: String(this.limits.pageSize), order_version: ORDER};
      if (generation !== null) integer(generation);
      if (token) { validateFixed(expectedFixed); await this.validateToken(token, expectedFixed, token.cut_projection_digest, 0); }
      const rows = [], seen = new Set(), objectIds = new Set(); let base = null, next = payload, issued = token?.issued_capture_digest;
      for (let pageNumber = 0; ; pageNumber++) {
        need(pageNumber < this.limits.maxPages, "分页数达到浏览器界限，未验证");
        const envelope = await this.request(identity, next, budget), page = envelope.payload;
        keys(page, PAGE, "Snapshot 页"); validateFixed(page.fixed_cut);
        need(sameIdentity(envelope, identity) && sameIdentity(page.fixed_cut, identity), "Snapshot 响应会话/化身不符");
        need(page.fixed_cut.history_mode === mode && (generation === null || page.fixed_cut.cut_generation === generation), "Snapshot 不属于请求的模式/cut");
        if (expectedFixed) need(equal(page.fixed_cut, expectedFixed), "重建页不属于 Gap 声明的固定切面");
        need(equal(page.scope, SCOPE) && equal(page.omissions, []) && page.order_version === ORDER, "Snapshot scope/omissions/order 不符");
        digest(page.cut_projection_digest); digest(page.validated_capture_digest);
        keys(page.counts, [...FAMILIES, "total"], "六族完整计数");
        for (const count of Object.values(page.counts)) integer(count);
        need(FAMILIES.reduce((n, k) => n + BigInt(page.counts[k]), 0n) === BigInt(page.counts.total) &&
          BigInt(page.counts.total) <= BigInt(this.limits.maxRows), "完整计数不符或达到浏览器行数界限");
        need(page.offset === String(rows.length) && typeof page.done === "boolean" && Array.isArray(page.rows), "分页 offset/done/rows 无效");
        const metadata = Object.fromEntries(["fixed_cut", "catalog", "counts", "scope", "omissions", "order_version", "cut_projection_digest"].map(k => [k, page[k]]));
        if (base) need(equal(base, metadata), "跨页固定切面/目录/计数/摘要变化"); else { base = clone(metadata); issued ||= page.validated_capture_digest; }
        const rest = BigInt(page.counts.total) - BigInt(rows.length);
        const expectedLength = rest < BigInt(this.limits.pageSize) ? Number(rest) : this.limits.pageSize;
        need(rest >= 0n && page.rows.length === expectedLength && page.done === (rest === BigInt(page.rows.length)), "分页截断/提前结束/空转");
        for (const row of page.rows) {
          keys(row, ["family", "key", "record"], "记录行"); need(FAMILIES.includes(row.family) && object(row.record), "未知记录族/记录形状");
          need(equal(row.key, rowKey(row.family, row.record)), "记录公开主键不符"); orderTuple(row.family, row.record);
          const id = canonical([row.family, row.key]); need(!seen.has(id), "分页重复公开身份"); seen.add(id);
          if (["objects", "withdrawn_objects"].includes(row.family)) { need(!objectIds.has(row.key[0]), "活动/撤回身份冲突"); objectIds.add(row.key[0]); }
          need(rows.length === 0 || compareRows(rows[rows.length - 1], row) < 0, "分页记录未按声明全序严格前进"); rows.push(clone(row));
        }
        if (page.done) { need(page.next_token === null, "末页仍返回 token"); break; }
        await this.validateToken(page.next_token, base.fixed_cut, base.cut_projection_digest, rows.length);
        need(page.next_token.issued_capture_digest === issued, "续页 token 签发来源改变");
        next = {op: "snapshot", snapshot_token: clone(page.next_token)};
      }
      for (const family of FAMILIES) need(String(rows.filter(r => r.family === family).length) === base.counts[family], "六族完整计数不符：" + family);
      const projection = {...base, rows}; delete projection.cut_projection_digest;
      need(await sha256(projection) === base.cut_projection_digest, "完整分页投影 SHA-256 不符");
      const state = stateOf(projection); this.hooks.validateState(state, "s-session/2"); this.check(budget);
      return {state, projection, cursor: cursorOf(base.fixed_cut), pages: Math.max(1, Math.ceil(rows.length / this.limits.pageSize))};
    }
    #commit(candidate, budget, event) {
      this.check(budget); this.#committed = candidate; this.onCommit(candidate, event); return candidate;
    }
    async load(discovery, mode = "RecomputedWithRevision", generation = null) {
      const budget = this.begin(); // 无效的新请求也使旧响应失效，不能事后覆盖错误状态。
      need(object(discovery) && object(discovery.cut), "缺少 v2 发现身份");
      const identity = identityOf(discovery.cut); text(identity.session_id); integer(identity.session_generation, true);
      need(mode === "AsKnown" || mode === "RecomputedWithRevision", "历史模式无效");
      if (mode === "AsKnown") integer(generation);
      const candidate = await this.collect({identity, mode, generation}, budget);
      return this.#commit(candidate, budget, {kind: "snapshot"});
    }
    applyBatch(snapshot, delta) {
      keys(delta, ["generation", "session_id", "catalog_revision", "base_cut", "next_cut", "seq_range", "index_frontier", "input_frontier", "catalog_run_status", "catalog_evidence", "delta"], "完整原子批次");
      keys(delta.delta, ["generation", "session_id", "catalog_revision", "base_cut", "next_cut", "seq_range", "index_frontier", "input_frontier", "catalog_run_status", "catalog_evidence", "upserts", "withdrawals", "replaces", "witnesses", "relations", "observations", "raw_history_added"], "完整原子批次载荷");
      this.hooks.validateDelta(delta);
      const result = clone(snapshot), dd = delta.delta;
      const maps = Object.fromEntries(FAMILIES.map(family => [family, new Map(result[family].map(r => [canonical(rowKey(family, r)), r]))]));
      for (const [field, family] of [["upserts", "objects"], ["withdrawals", "objects"], ["witnesses", "witnesses"], ["relations", "relations"], ["observations", "observations"], ["raw_history_added", "raw_history"]]) {
        const unique = new Set(); for (const record of dd[field]) { const key = canonical(rowKey(family, record)); need(!unique.has(key), "批次含重复身份：" + field); unique.add(key); }
      }
      for (const withdrawal of dd.withdrawals) {
        const key = canonical([withdrawal.object_id]), previous = maps.objects.get(key);
        need(previous, "撤回引用不属于已提交活动对象");
        maps.withdrawn_objects.set(key, {...previous, lifecycle: "withdrawn", withdrawal_reason: withdrawal.reason,
          superseded_by: withdrawal.superseded_by, withdrawn_generation: delta.generation}); maps.objects.delete(key);
      }
      for (const record of dd.upserts) { const key = canonical(rowKey("objects", record)); maps.objects.set(key, {...clone(record), lifecycle: "active"}); maps.withdrawn_objects.delete(key); }
      for (const [field, family] of [["witnesses", "witnesses"], ["relations", "relations"], ["observations", "observations"], ["raw_history_added", "raw_history"]])
        for (const record of dd[field]) maps[family].set(canonical(rowKey(family, record)), clone(record));
      for (const family of FAMILIES) result[family] = [...maps[family].values()];
      return result;
    }
    async watch() {
      const budget = this.begin();
      need(this.committed, "尚未安装完整分页，不能 Watch");
      const before = this.committed;
      const reply = await this.request(before.cursor, {op: "watch", cursor: before.cursor,
        max_batches: String(this.limits.watchBatches), client_id: this.instance}, budget);
      const page = reply.payload; keys(page, WATCH, "Watch"); validateFixed(page.observed_head); digest(page.validated_capture_digest);
      need(page.observed_head.history_mode === "RecomputedWithRevision", "Watch observed_head 不是当前重算切面");
      need(sameIdentity(reply, page.observed_head), "Watch 头与观测切面身份不符");
      keys(page.delivery_frontier, ["policy_revision", "retain_generations", "first_available_generation", "head_generation"], "投递前沿");
      text(page.delivery_frontier.policy_revision); integer(page.delivery_frontier.retain_generations, true);
      integer(page.delivery_frontier.first_available_generation, true); integer(page.delivery_frontier.head_generation);
      need(page.delivery_frontier.head_generation === page.observed_head.cut_generation &&
        BigInt(page.delivery_frontier.first_available_generation) <= BigInt(page.delivery_frontier.head_generation) + 1n, "投递前沿与观测 cut 不符");
      need(Array.isArray(page.deltas) && page.deltas.length <= this.limits.watchBatches && typeof page.has_more === "boolean", "Watch 批次数/has_more 无效");
      keys(page.next_cursor, CURSOR, "完整 Watch cursor");
      if (page.gap !== null) {
        const gap = page.gap; keys(gap, ["requested_identity", "reason", "missing_range", "rebuild", "snapshot_token"], "Gap");
        need(equal(gap.requested_identity, before.cursor) && equal(page.next_cursor, before.cursor) && page.deltas.length === 0 && !page.has_more, "Gap 与请求身份/批次不符");
        need(equal(gap.rebuild, page.observed_head), "Gap 重建切面与当前观测身份不符");
        const changed = !sameIdentity(before.cursor, gap.rebuild);
        if (gap.reason === "session_identity_changed") need(changed && gap.missing_range === null, "会话 Gap 的身份/缺口范围不符");
        else {
          need(!changed && ["delivery_retention_gap", "client_backlog_overflow"].includes(gap.reason), "未知或错身份 Gap");
          keys(gap.missing_range, ["from_generation", "to_generation"], "Gap 缺口范围");
          const to = gap.reason === "delivery_retention_gap" ? BigInt(page.delivery_frontier.first_available_generation) - 1n : BigInt(gap.rebuild.cut_generation);
          need(gap.missing_range.from_generation === (BigInt(before.cursor.after_generation) + 1n).toString() &&
            gap.missing_range.to_generation === to.toString() && BigInt(gap.missing_range.from_generation) <= to, "Gap 缺口区间不符");
        }
        const candidate = await this.collect({identity: identityOf(gap.rebuild), mode: gap.rebuild.history_mode,
          generation: gap.rebuild.cut_generation, token: gap.snapshot_token, expectedFixed: gap.rebuild}, budget);
        return this.#commit(candidate, budget, {kind: "gap", reason: gap.reason});
      }
      need(sameIdentity(reply, before.cursor), "无 Gap 不得切换会话/化身");
      let candidate = before, applied = clone(before.state.snapshot);
      for (const delta of page.deltas) {
        this.hooks.validateDelta(delta);
        need(delta.generation === (BigInt(candidate.cursor.after_generation) + 1n).toString() && delta.base_cut === candidate.cursor.base_cut &&
          delta.session_id === before.cursor.session_id && delta.catalog_revision === before.cursor.catalog_revision &&
          delta.seq_range.from === (BigInt(candidate.cursor.last_seq) + 1n).toString(), "Delta 代际/身份/cut/输入前沿断链");
        applied = this.applyBatch(applied, delta);
        candidate = await this.collect({identity: before.cursor, mode: "RecomputedWithRevision", generation: delta.generation}, budget);
        this.hooks.comparePublication(delta, candidate.state.snapshot);
        need(equal(rowsOf(applied), candidate.projection.rows), "Delta 应用与同 cut 分页六族全字段/顺序不一致");
      }
      if (!page.deltas.length) {
        candidate = await this.collect({identity: before.cursor, mode: before.projection.fixed_cut.history_mode,
          generation: before.cursor.after_generation}, budget);
        need(equal(candidate.projection, before.projection), "空增量下已提交完整切面改变");
      }
      need(equal(page.next_cursor, candidate.cursor), "Watch 完整 next_cursor 不匹配应用末批");
      const head = BigInt(page.observed_head.cut_generation), end = BigInt(candidate.cursor.after_generation);
      need(head >= end && page.has_more === (head > end) && (page.deltas.length > 0 || head === end), "Watch observed_head/has_more 或批次截断");
      if (head === end) {
        const fixed = {...candidate.projection.fixed_cut, history_mode: "RecomputedWithRevision", as_of_generation: null};
        need(equal(page.observed_head, fixed), "Watch observed_head 与同代完整发布身份不符");
      }
      return this.#commit(candidate, budget, {kind: "watch", batches: page.deltas.length, hasMore: page.has_more});
    }
  }
  return Object.freeze({Client, DEFAULTS, FAMILIES, ORDER, canonical, parse, sha256, rowsOf, cursorOf});
});
