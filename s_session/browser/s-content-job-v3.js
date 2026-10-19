/* #1394 C §5.6.1：纯累计账；实际 I/O、实际分配测量及业务验证属于调用方。 */
(function (root, factory) {
  "use strict";
  const common = typeof module === "object" && module.exports;
  const api = factory(common ? require("./s-content-v3.js") : root.SContentV3);
  if (common) module.exports = api;
  else root.SContentJobV3 = api;
})(typeof globalThis === "object" ? globalThis : this, function (content) {
  "use strict";
  const MAX = (1n << 63n) - 1n;
  const LIMIT_FIELDS = Object.freeze(["max_total_wire_bytes", "max_total_node_bytes",
    "max_total_nodes", "max_total_records", "max_total_requests", "max_elapsed_ms",
    "max_staged_bytes", "max_store_bytes"]);
  function fail(code, detail) {
    const error = new Error(detail);
    error.code = code;
    throw error;
  }
  function exact(value, fields, code) {
    if (!value || typeof value !== "object" || Array.isArray(value) ||
      Reflect.ownKeys(value).length !== fields.length || fields.some(k => !Object.hasOwn(value, k)))
      fail(code, "字段集合必须 exact；没有缺省值");
  }
  function captureFields(value, fields, code) {
    exact(value, fields, code);
    // 不在验证后重读调用方属性；getter 产生的值也只捕获一次。
    return Object.freeze(Object.fromEntries(fields.map(k => [k, value[k]])));
  }
  function parseLimits(value) {
    value = captureFields(value, LIMIT_FIELDS, "job_limit_field");
    const copy = {};
    for (const key of LIMIT_FIELDS) {
      const text = value[key];
      if (typeof text !== "string") fail("job_limit_field", key + " 必须是十进制文本");
      if (!/^[1-9][0-9]*$/.test(text) || BigInt(text) > MAX)
        fail("job_limit_domain", key + " 不在规范 1..2^63-1 域");
      copy[key] = text;
    }
    return Object.freeze(copy);
  }
  function validateConfig(value) {
    const controls = ["pageSize", "watchBatches", "pollMs", "timeoutMs", "replyBytes"];
    value = captureFields(value, ["schema_revision", ...controls, "job_limits"], "job_config_field");
    if (value.schema_revision !== "s-query-client-resources/3")
      fail("job_config_schema", "浏览器配置必须为 s-query-client-resources/3");
    const copy = {schema_revision: value.schema_revision};
    for (const key of controls) {
      const n = value[key];
      if (!Number.isSafeInteger(n) || n <= 0) fail("job_config_domain", key + " 必须是显式安全正整数");
      copy[key] = n;
    }
    copy.job_limits = parseLimits(value.job_limits);
    return Object.freeze(copy);
  }
  function amount(value, code = "job_amount") {
    if (typeof value === "bigint" && value >= 0n) return value;
    if (typeof value === "string" && /^(?:0|[1-9][0-9]*)$/.test(value)) return BigInt(value);
    fail(code, "实际量必须是非负 BigInt 或规范非负十进制文本");
  }
  const space = baseline => ({baseline, committed: 0n, pending: 0n, peak: baseline});
  class Job {
    #limits; #caps; #clock; #readingClock = false; #last = null; #started = null; #stopped = null;
    #counts = {wire_bytes: 0n, node_bytes: 0n, nodes: 0n, records: 0n, requests: 0n};
    #staged; #store; #accepted = new Map(); #open = new Map();
    constructor(limits, existing, clock) {
      this.#limits = parseLimits(limits);
      this.#caps = Object.fromEntries(LIMIT_FIELDS.map(k => [k, BigInt(this.#limits[k])]));
      existing = captureFields(existing, ["store_bytes", "staged_bytes"], "job_existing");
      const store = amount(existing.store_bytes, "job_limit_domain");
      const staged = amount(existing.staged_bytes, "job_limit_domain");
      if (staged > store) fail("job_limit_domain", "既存暂存必须是总存储的子集");
      if (staged > this.#caps.max_staged_bytes || store > this.#caps.max_store_bytes)
        fail("job_limit_exceeded", "既存空间已超上限，即使纯读取也拒绝");
      if (typeof clock !== "function") fail("job_clock", "必须显式注入单调毫秒时钟");
      this.#clock = clock;
      this.#staged = space(staged); this.#store = space(store);
    }
    #stop(code, detail) {
      if (this.#stopped === null) this.#stopped = Object.freeze({code, detail});
      fail(this.#stopped.code, this.#stopped.detail);
    }
    #assertLive() {
      this.#assertOutsideClock();
      if (this.#stopped) fail(this.#stopped.code, this.#stopped.detail);
    }
    #assertOutsideClock() {
      if (this.#readingClock) this.#stop("job_clock", "时钟回调不得重入 Job 操作");
    }
    #now() {
      this.#assertOutsideClock();
      let now;
      this.#readingClock = true;
      try {
        let raw = this.#clock();
        if (typeof raw === "number" && Number.isSafeInteger(raw) && raw >= 0) raw = BigInt(raw);
        now = amount(raw, "job_clock");
      } catch { this.#stop("job_clock", "时钟回调失败或未返回规范非负毫秒值"); }
      finally { this.#readingClock = false; }
      if (this.#last !== null && now < this.#last) this.#stop("job_clock", "时钟不得倒退");
      this.#last = now;
      return now;
    }
    #checkTime(now) {
      if (this.#started !== null && now - this.#started >= this.#caps.max_elapsed_ms)
        this.#stop("job_deadline", "首次读取的绝对期限已过");
    }
    #gate() {
      this.#assertLive();
      const now = this.#now();
      // 外部回调可能 cancel 或吞掉重入拒绝；返回后必须再核永久停止状态。
      this.#assertLive();
      this.#checkTime(now);
    }
    beginFirstRead() {
      this.#gate();
      if (this.#started === null) this.#started = this.#last;
    }
    firstReadDeadline() {
      this.beginFirstRead();
      return (this.#started + this.#caps.max_elapsed_ms).toString();
    }
    checkDeadline() { this.#gate(); }
    preIoCheck() { this.#gate(); }
    // 本口只收已经发生的实际量：先永久入账，再判 cap/期限。
    // 写前预检被拒、没有访问或发起请求时，adapter 不得调用 actual charge。
    #charge(field, cap, actual) {
      this.#assertLive();
      actual = amount(actual);
      // 已发生且合法的 actual 不可逆；时钟失败也不能丢掉尾量。
      this.#counts[field] += actual;
      const now = this.#now();
      this.#assertLive();
      if (this.#started === null) this.#started = now;
      if (this.#counts[field] > this.#caps[cap]) this.#stop("job_limit_exceeded", cap + " 实际累计已超上限");
      this.#checkTime(now);
    }
    chargeWireBytes(actual) { this.#charge("wire_bytes", "max_total_wire_bytes", actual); }
    chargeRecords(actual) { this.#charge("records", "max_total_records", actual); }
    chargeRequests(actual) { this.#charge("requests", "max_total_requests", actual); }
    async acceptNode(node) {
      this.beginFirstRead();
      // 输入均为字符串，await 前复制；不保存调用方可变对象或借用其字节缓冲区。
      const raw = captureFields(node, ["root", "tag", "body_hex"], "node_digest");
      this.#assertLive();
      if ([raw.root, raw.tag, raw.body_hex].some(v => typeof v !== "string"))
        fail("node_digest", "节点原件字段必须为字符串");
      if (!content?.GraphView) fail("job_content_core", "必须加载固定 s-content-v3 核心");
      let graph;
      try { graph = await content.GraphView.create([raw]); }
      catch (error) {
        this.#gate();
        fail("node_digest", "原件 F 或局部 header 不合法：" + error.message);
      }
      // hash 的异步边界后复查取消/绝对期限；晚到结果不得重新开启 job。
      this.#gate();
      const old = this.#accepted.get(raw.root);
      if (old) {
        if (old.tag !== raw.tag || old.body_hex !== raw.body_hex)
          fail("node_digest", "相同根不同字节");
        return "already_accepted";
      }
      const bytes = this.#counts.node_bytes + BigInt(graph.metrics.framedBytes);
      const nodes = this.#counts.nodes + 1n;
      // “接纳”的写前准入：候选 F 核过不等于本 job 已接纳；真实 wire 另走 actual charge。
      if (bytes > this.#caps.max_total_node_bytes || nodes > this.#caps.max_total_nodes)
        this.#stop("job_limit_exceeded", "候选节点超出节点接纳上限");
      this.#counts.node_bytes = bytes; this.#counts.nodes = nodes;
      this.#accepted.set(raw.root, raw);
      return "first_accept";
    }
    cancel() {
      if (this.#stopped === null) this.#stopped = Object.freeze({code: "job_cancelled", detail: "调用方取消 job"});
    }
    #refreshPeaks() {
      const staged = this.#stagedExposure(), store = this.#storeExposure();
      if (staged > this.#staged.peak) this.#staged.peak = staged;
      if (store > this.#store.peak) this.#store.peak = store;
    }
    #reserve(kind, requested) {
      this.#gate(); requested = amount(requested);
      // 两本 cap 先一起核完，再动 pending/open/peak，拒绝时不残留部分预留。
      if ((kind === "staged" && this.#stagedExposure() + requested > this.#caps.max_staged_bytes) ||
        this.#storeExposure() + requested > this.#caps.max_store_bytes)
        this.#stop("job_limit_exceeded", "空间预留超出暂存或总存储上限");
      const grant = Object.freeze({kind, requested: requested.toString()});
      const entry = Object.freeze({kind, requested});
      // 对象 identity 与本 job 的私有表绑定，spread/继承/跨 job 都不是句柄。
      this.#open.set(grant, entry);
      (kind === "staged" ? this.#staged : this.#store).pending += requested;
      this.#refreshPeaks();
      return grant;
    }
    reserveStaged(requested) { return this.#reserve("staged", requested); }
    reserveStore(requested) { return this.#reserve("store", requested); }
    #entry(grant) {
      this.#assertOutsideClock();
      const entry = this.#open.get(grant);
      if (!entry) fail("job_reservation", "伪造、跨 job 或已关闭的预留句柄");
      return entry;
    }
    reconcile(grant, actual) {
      const entry = this.#entry(grant); actual = amount(actual);
      const ledger = entry.kind === "staged" ? this.#staged : this.#store;
      this.#open.delete(grant); ledger.pending -= entry.requested; ledger.committed += actual;
      this.#refreshPeaks();
      // stopped 仍能清理旧预留；实际超预留必须照实进账，首次停止原因不会被替换。
      if (actual > entry.requested) this.#stop("job_reconcile", "实际分配超过预留，已照实入账");
    }
    release(grant) {
      const entry = this.#entry(grant);
      this.#open.delete(grant);
      (entry.kind === "staged" ? this.#staged : this.#store).pending -= entry.requested;
    }
    reclaimStaged(actualDeleted) {
      this.#assertOutsideClock();
      actualDeleted = amount(actualDeleted);
      if (actualDeleted > this.#staged.committed) fail("job_reclaim", "只能回收本 job 已对账的暂存");
      this.#staged.committed -= actualDeleted;
    }
    finishCheck() {
      this.#gate();
      if (this.#open.size) fail("job_reservation", "安装前必须完成全部预留对账或释放");
    }
    #stagedExposure() { const s = this.#staged; return s.baseline + s.committed + s.pending; }
    #storeExposure() {
      const s = this.#store;
      return s.baseline + s.committed + s.pending + this.#staged.committed + this.#staged.pending;
    }
    #spaceSnapshot(s, exposure) {
      return Object.freeze({baseline: s.baseline.toString(), committed: s.committed.toString(),
        pending: s.pending.toString(), exposure: exposure.toString(), peak: s.peak.toString()});
    }
    snapshot() {
      const now = this.#now();
      // 取快照不能把过期的 job 描述为仍可使用；旧停止原因永久保留。
      if (!this.#stopped && this.#started !== null && now - this.#started >= this.#caps.max_elapsed_ms)
        this.#stopped = Object.freeze({code: "job_deadline", detail: "首次读取的绝对期限已过"});
      return Object.freeze({limits: this.#limits, started: this.#started !== null,
        elapsed_ms: this.#started === null ? null : (now - this.#started).toString(),
        ...Object.fromEntries(Object.entries(this.#counts).map(([k, v]) => [k, v.toString()])),
        staged: this.#spaceSnapshot(this.#staged, this.#stagedExposure()),
        store: this.#spaceSnapshot(this.#store, this.#storeExposure()),
        open_reservations: this.#open.size.toString(), stopped: this.#stopped?.code ?? null});
    }
  }
  return Object.freeze({LIMIT_FIELDS, parseLimits, validateConfig, Job});
});
