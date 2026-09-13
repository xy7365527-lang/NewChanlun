/* #1374：经济域只读消费者；规范字节/严格解析复用现有 TB01C。 */
(function (root, factory) {
  const shared = typeof module === "object" && module.exports ? require("./tb01c-client.js") : root.TB01C;
  const api = factory(shared);
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.TB03A = api;
})(globalThis, function (shared) {
  "use strict";
  const {canonical, parse, sha256} = shared;
  const object = value => value !== null && typeof value === "object" && !Array.isArray(value);
  const need = (ok, reason) => { if (!ok) throw new Error(reason); };
  const same = (a, b) => canonical(a) === canonical(b);
  function keys(value, fields, what) {
    need(object(value) && Object.keys(value).length === fields.length && fields.every(k => Object.hasOwn(value, k)), what + " 字段不完整或多余");
  }
  function integer(value, positive = false, signed = false) {
    need(typeof value === "string" && /^(?:0|-?[1-9][0-9]*)$/.test(value), "整数不是规范文本");
    const number = BigInt(value);
    need(number <= 9223372036854775807n && number >= (signed ? -9223372036854775808n : positive ? 1n : 0n), "整数越出精确域");
    return number;
  }
  function cut(value, domain) {
    keys(value, ["domain_id", "commit_seq", "root_hash"], "域切面");
    need(value.domain_id === domain && /^[0-9a-f]{64}$/.test(value.root_hash), "切面域或根摘要不符");
    integer(value.commit_seq);
  }
  function text(value, what) {
    need(typeof value === "string" && value.length > 0, what + " 缺失");
  }
  function digest(value, what) {
    need(typeof value === "string" && /^[0-9a-f]{64}$/.test(value), what + " 摘要无效");
  }
  function reference(value) {
    keys(value, ["authority_id", "object_id", "revision", "content_hash"], "开局对象引用");
    text(value.authority_id, "权威身份"); text(value.object_id, "对象身份");
    integer(value.revision); digest(value.content_hash, "对象内容");
  }
  async function verifyVoices(book, view, discovery, domain) {
    const voices = book.voice;
    if (voices.status === "MissingDependency") {
      keys(voices, ["status", "reason", "identity"], "未绑定 Voice");
      need(voices.identity === null, "缺依赖不能携带已成立身份");
      text(voices.reason, "开局依据缺项");
      return;
    }
    keys(voices, ["status", "records"], "Voice 集合");
    need(voices.status === "Bound" && Array.isArray(voices.records) && voices.records.length > 0,
      "已绑定 Voice 集合为空或状态未知");
    const identities = new Map(), bindings = new Set();
    for (const record of voices.records) {
      keys(record, ["voice_id", "binding_id", "identity", "instrument", "operation_level", "structure_view",
        "parent_voice_id", "opening_basis", "evidence", "first_known_ns", "commit_ns", "commit_seq",
        "business_lifecycle", "responsibility_tail"], "Voice 记录");
      digest(record.voice_id, "Voice 身份"); text(record.binding_id, "绑定身份");
      need(!identities.has(record.voice_id) && !bindings.has(record.binding_id), "Voice 或绑定身份重复");
      identities.set(record.voice_id, record); bindings.add(record.binding_id);
      keys(record.identity, ["carrier_ref", "gamma_ref", "sigma", "n"], "Voice 四元身份");
      reference(record.identity.carrier_ref); reference(record.identity.gamma_ref);
      need(["Long", "Short"].includes(record.identity.sigma), "Voice 方向未知");
      integer(record.identity.n);
      need(record.instrument === book.symbol && record.operation_level === book.operation_level,
        "Voice 与所属重的标的或操作级别不同");
      const structure = record.structure_view;
      keys(structure, ["profile_id", "execution_level", "confirmation_level", "nest_depth"], "结构参照");
      text(structure.profile_id, "结构 profile");
      const execution = integer(structure.execution_level), confirmation = integer(structure.confirmation_level);
      need(confirmation === execution + integer(structure.nest_depth), "结构参照与区间套梯级数不符");
      const basis = record.opening_basis;
      keys(basis, ["fact_key", "fact_hash", "allocation_id", "e_cut", "raw_sha256", "json_pointer", "quantity", "units"], "开局量依据");
      text(basis.fact_key, "原事实身份"); digest(basis.fact_hash, "原事实");
      text(basis.allocation_id, "分配身份"); cut(basis.e_cut, "E"); digest(basis.raw_sha256, "原始字节");
      need(typeof basis.json_pointer === "string" && same(basis.units, book.units), "开局量纲或来源指针不符");
      integer(basis.quantity, true);
      keys(record.evidence, ["authority_id", "bundle_sha256", "verification_sha256"], "定义证据引用");
      text(record.evidence.authority_id, "定义权威");
      need(record.identity.carrier_ref.authority_id === record.evidence.authority_id &&
        record.identity.gamma_ref.authority_id === record.evidence.authority_id, "开局引用与定义证据权威不同");
      digest(record.evidence.bundle_sha256, "定义证据包"); digest(record.evidence.verification_sha256, "独立核验");
      need(integer(record.first_known_ns) <= integer(record.commit_ns) &&
        BigInt(record.commit_ns) <= BigInt(view.commit_ns) &&
        integer(record.commit_seq, true) <= BigInt(view.cut.commit_seq), "Voice 晚于当前可知切面");
      keys(record.business_lifecycle, ["status"], "业务生命周期");
      need(record.business_lifecycle.status === "NotRecorded", "未支持的业务生命周期记录");
      keys(record.responsibility_tail, ["status", "reason"], "责任尾账");
      need(record.responsibility_tail.status === "Unknown", "未支持的责任尾账记录");
      text(record.responsibility_tail.reason, "责任尾账缺项");
      const identity = {schema: "voice-identity/1", domain_id: domain, session_id: discovery.session_id,
        session_generation: discovery.session_generation, chong_id: book.chong_id, identity: record.identity};
      need(record.voice_id === await sha256(identity), "Voice 不可变身份摘要不符");
    }
    for (const record of voices.records) {
      if (record.parent_voice_id === null) need(record.identity.n === "0", "根 Voice 的代际须为零");
      else {
        const parent = identities.get(record.parent_voice_id);
        need(parent !== undefined && BigInt(record.identity.n) === BigInt(parent.identity.n) + 1n &&
          record.identity.sigma !== parent.identity.sigma, "Voice 父关系、代际或方向不符");
        need(BigInt(parent.commit_seq) < BigInt(record.commit_seq) &&
          BigInt(parent.commit_ns) <= BigInt(record.first_known_ns) &&
          BigInt(parent.first_known_ns) <= BigInt(record.first_known_ns), "父 Voice 未先于子 Voice 持久成立");
      }
    }
  }
  async function verifySnapshot(value, discovery) {
    keys(value, ["kind", "session_id", "session_generation", "manifest_hash", "mode", "as_known_ns", "domains", "completeness", "snapshot_token"], "经济快照");
    need(value.kind === "EconomicSnapshot" && value.session_id === discovery.session_id &&
      value.session_generation === discovery.session_generation && value.manifest_hash === discovery.manifest_hash, "经济快照会话或配置不符");
    need(value.mode === "Current" || value.mode === "AsKnown", "未知经济历史模式");
    if (value.as_known_ns !== null) integer(value.as_known_ns);
    keys(value.domains, discovery.domains, "全部经济域");
    let complete = true;
    for (const domain of discovery.domains) {
      const view = value.domains[domain];
      if (view.status === "Unavailable") {
        keys(view, ["status", "error"], "不可用域");
        need(typeof view.error === "string" && view.error.length > 0, "域不可用原因缺失");
        complete = false;
        continue;
      }
      keys(view, ["status", "cut", "commit_ns", "image", "image_hash"], "可用域");
      need(view.status === "Available", "未知域状态");
      cut(view.cut, domain);
      integer(view.commit_ns);
      if (value.as_known_ns !== null) need(BigInt(view.commit_ns) <= BigInt(value.as_known_ns), "图像晚于所声明的获知截止时刻");
      keys(view.image, ["book", "recorded", "pending", "applied", "unmatched", "history"], "完整经济图像");
      for (const key of ["recorded", "pending", "applied", "unmatched", "history"]) need(Array.isArray(view.image[key]), "经济记录族不是数组");
      need(await sha256(view.image) === view.image_hash, "完整经济图像摘要不同");
      const book = view.image.book;
      if (domain === "E") need(book === null, "E 不能伪装重账");
      else if (book !== null) {
        need(object(book) && domain === "B:" + book.chong_id, "重归属与权威域不同");
        integer(book.fixed_q, true); integer(book.operation_level); integer(book.actual_quantity, false, true);
        integer(book.opening_quantity, false, true);
        keys(book.income_book, ["Pi", "A", "W", "R"], "利润账");
        for (const amount of Object.values(book.income_book)) integer(amount, false, true);
        for (const amount of Object.values(book.known_cost_totals)) integer(amount, false, true);
        need(object(book.unknown) && object(book.voice) && Array.isArray(book.known_cost_components), "缺失账簿知识状态/原始成本史");
        await verifyVoices(book, view, discovery, domain);
      }
    }
    need(value.completeness === (complete ? "CompleteVector" : "PartialVector"), "域完整性声明与实际结果不同");
    const body = {...value}; delete body.snapshot_token;
    need(value.snapshot_token === "economic-snapshot-" + await sha256(body), "固定向量 token 与完整图像不同");
    return value;
  }
  class Client {
    constructor({fetcher = globalThis.fetch.bind(globalThis), onCommit = () => {}} = {}) {
      this.fetcher = fetcher; this.onCommit = onCommit; this.discovery = null; this.state = null; this.epoch = 0;
    }
    async fetchBounded(path, options, timeout, limit) {
      const control = new AbortController(), timer = setTimeout(() => control.abort(), timeout);
      try {
        const response = await this.fetcher(path, {...options, signal: control.signal});
        need(response.ok, "公开查询失败：HTTP " + response.status);
        const reader = response.body.getReader(), decoder = new TextDecoder("utf-8", {fatal: true});
        let size = 0, raw = "";
        try {
          while (true) {
            const {done, value} = await reader.read(); if (done) break;
            size += value.byteLength; need(size <= limit, "完整回复超过事前上限"); raw += decoder.decode(value, {stream: true});
          }
          raw += decoder.decode(); return parse(raw);
        } catch (error) { await reader.cancel(); throw error; }
      } finally { clearTimeout(timer); }
    }
    async discover() {
      if (this.discovery) return this.discovery;
      const d = await this.fetchBounded("/api/economic/ready", {}, 10000, 65536);
      need(d.schema_revision === "economic-discovery/1" && d.configured === true && d.scope === "TestOnlyRecordedEconomics", "经济查询配置未确认");
      need(typeof d.session_id === "string" && d.session_id.length > 0 && /^[0-9a-f]{64}$/.test(d.manifest_hash), "会话与配置摘要缺失");
      integer(d.session_generation, true); integer(d.producer_epoch, true);
      need(Array.isArray(d.domains) && d.domains.length > 0 && new Set(d.domains).size === d.domains.length &&
        d.domains.every(v => typeof v === "string" && (v === "E" || /^B:.+/.test(v))), "域目录无效");
      integer(d.max_reply_bytes, true); integer(d.operation_timeout_ms, true);
      need(BigInt(d.max_reply_bytes) <= 67108864n && BigInt(d.operation_timeout_ms) <= 60000n, "公开配置超过浏览器资源界限");
      this.discovery = d; return d;
    }
    async query(payload, deadline = null) {
      const d = await this.discover();
      const request = {schema_revision: "economic-session/1", session_id: d.session_id, session_generation: d.session_generation,
        source_namespace: "economic-browser", source_epoch: "1", message_id: "browser-" + globalThis.crypto.randomUUID(),
        producer_id: "economic-browser", producer_epoch: "1", payload_hash: await sha256(payload), causal_refs: [], payload};
      const timeout = deadline === null ? Number(d.operation_timeout_ms) + 1000 : Math.min(Number(d.operation_timeout_ms) + 1000, deadline - performance.now());
      need(timeout > 0, "整次追读超过事前时限，保留原切面");
      const response = await this.fetchBounded("/api/economic/" + payload.op,
        {method: "POST", headers: {"Content-Type": "application/json"}, body: canonical(request)},
        timeout, Number(d.max_reply_bytes));
      keys(response, Object.keys(request), "公共回复");
      need(response.schema_revision === request.schema_revision && response.session_id === d.session_id &&
        response.session_generation === d.session_generation && response.producer_id === "Q:" + d.session_id &&
        response.producer_epoch === d.producer_epoch && response.source_epoch === d.producer_epoch &&
        response.source_namespace === "economic-observe/replies", "公开回复身份不符");
      const ph = await sha256(response.payload);
      const cause = Object.fromEntries(["source_namespace", "source_epoch", "message_id", "payload_hash"].map(k => [k, request[k]]));
      need(response.payload_hash === ph && same(response.causal_refs, [cause]) && response.message_id === "reply-" +
        await sha256([request.source_namespace, request.source_epoch, request.message_id, ph]), "回复内容或原因果摘要不符");
      need(!response.payload.error && response.payload.ok !== false, response.payload.detail || response.payload.error || "查询失败");
      if (deadline !== null) need(performance.now() < deadline, "整次追读超过事前时限，保留原切面");
      return response.payload;
    }
    async load({cuts = null, asKnown = null, token = null} = {}) {
      const epoch = ++this.epoch;
      const result = await this.query({op: "snapshot", cuts, as_known_ns: asKnown, snapshot_token: token});
      need(result.kind !== "Gap", "固定切面已过期，请重新读取当前向量");
      await verifySnapshot(result, this.discovery);
      if (token !== null) need(result.snapshot_token === token, "返回了另一个固定 token");
      else {
        need(result.mode === (cuts !== null || asKnown !== null ? "AsKnown" : "Current") &&
          result.as_known_ns === asKnown, "历史模式或获知时刻与原请求不同");
      }
      if (cuts !== null) for (const [domain, expected] of Object.entries(cuts)) {
        const view = result.domains[domain];
        if (view.status === "Available") need(same(view.cut, expected), "回看返回其他切面");
      }
      if (epoch === this.epoch) { this.state = result; this.onCommit(result); }
      return result;
    }
    async watch() {
      need(this.state !== null && this.state.completeness === "CompleteVector", "需先取得完整向量才可续接");
      const epoch = ++this.epoch, before = this.state;
      const deadline = performance.now() + Number(this.discovery.operation_timeout_ms);
      const cuts = Object.fromEntries(Object.entries(before.domains).map(([d, v]) => [d, v.cut]));
      const changes = await this.query({op: "watch", cuts}, deadline);
      need(changes.kind === "EconomicWatch" && changes.session_id === before.session_id &&
        changes.session_generation === before.session_generation && changes.manifest_hash === before.manifest_hash, "变化身份不符");
      keys(changes.domains, this.discovery.domains, "变化域集合");
      const next = {};
      for (const [domain, result] of Object.entries(changes.domains)) {
        need(result.status === "Available", result.error || "变化域不可用，保留原切面");
        need(same(result.base_cut, cuts[domain]) && Array.isArray(result.changes), "变化起点不符");
        let previous = cuts[domain], previousNs = BigInt(before.domains[domain].commit_ns);
        for (const batch of result.changes) {
          keys(batch, ["cut", "image", "image_hash", "commit_ns"], "完整经济变化");
          cut(batch.cut, domain);
          integer(batch.commit_ns);
          need(BigInt(batch.commit_ns) >= previousNs && await sha256(batch.image) === batch.image_hash,
            "变化时间倒退或图像摘要不同");
          need(BigInt(batch.cut.commit_seq) === BigInt(previous.commit_seq) + 1n, "变化序号不连续");
          const one = {...cuts, [domain]: batch.cut};
          const fixed = await this.query({op: "snapshot", cuts: one, as_known_ns: null, snapshot_token: null}, deadline);
          await verifySnapshot(fixed, this.discovery);
          need(fixed.mode === "AsKnown" && fixed.as_known_ns === null, "逐批复核未返回指定历史模式");
          const view = fixed.domains[domain];
          need(view.status === "Available" && same(view.cut, batch.cut) && same(view.image, batch.image) &&
            view.image_hash === batch.image_hash && view.commit_ns === batch.commit_ns, "中间变化与同切面图像/时间不同");
          previous = batch.cut; previousNs = BigInt(batch.commit_ns);
        }
        need(same(previous, result.next_cut), "变化末尾与前沿不同"); next[domain] = result.next_cut;
      }
      const snapshot = await this.query({op: "snapshot", cuts: next, as_known_ns: null, snapshot_token: null}, deadline);
      await verifySnapshot(snapshot, this.discovery);
      need(snapshot.mode === "AsKnown" && snapshot.as_known_ns === null, "续接末尾历史模式不同");
      need(snapshot.completeness === "CompleteVector", "续接复核时存在不可用域");
      for (const domain of this.discovery.domains) {
        const batches = changes.domains[domain].changes;
        const expected = batches.length ? batches[batches.length - 1].image : before.domains[domain].image;
        const expectedNs = batches.length ? batches[batches.length - 1].commit_ns : before.domains[domain].commit_ns;
        need(same(snapshot.domains[domain].cut, next[domain]) && same(snapshot.domains[domain].image, expected) &&
          snapshot.domains[domain].commit_ns === expectedNs, "完整变化与同切面快照或提交时间不同");
      }
      need(performance.now() < deadline, "整次追读超过事前时限，保留原切面");
      if (epoch === this.epoch) { this.state = snapshot; this.onCommit(snapshot); }
      return snapshot;
    }
  }
  return Object.freeze({Client, verifySnapshot, canonical, parse, sha256});
});
