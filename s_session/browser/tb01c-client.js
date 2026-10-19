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
    // UTF-8字典序与合法Unicode标量序相同；先完整验证两参数，保留首错顺序。
    scalar(a); scalar(b);
    let ai = 0, bi = 0;
    while (ai < a.length && bi < b.length) {
      const aa = a.codePointAt(ai), bb = b.codePointAt(bi);
      if (aa !== bb) return aa < bb ? -1 : 1;
      ai += aa > 0xffff ? 2 : 1;
      bi += bb > 0xffff ? 2 : 1;
    }
    return ai < a.length ? 1 : bi < b.length ? -1 : 0;
  }
  function canonical(value) {
    if (value === null || typeof value === "boolean") return JSON.stringify(value);
    if (typeof value === "string") return JSON.stringify(scalar(value));
    if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
    need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
    return "{" + Object.keys(value).sort(utf8Compare).map(k => JSON.stringify(scalar(k)) + ":" + canonical(value[k])).join(",") + "}";
  }
  const equal = (a, b) => canonical(a) === canonical(b);
  // #1414：保留 canonical→parse 的严格值域与新树语义，避免中间规范字符串。
  function clone(value, depth = 0) {
    need(depth <= 128, "JSON 嵌套超过浏览器界限");
    if (value === null || typeof value === "boolean") return value;
    if (typeof value === "string") return scalar(value);
    if (Array.isArray(value)) {
      const out = [], length = value.length;
      // 原 map/join 将唯一空槽编码为 []；其余含空槽数组由 parse 拒绝。
      if (length === 1 && !(0 in value)) return out;
      for (let i = 0; i < length; i++) {
        need(i in value, "JSON 非法值；v2 整数必须使用精确文本");
        out.push(clone(value[i], depth + 1));
      }
      return out;
    }
    need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
    const out = Object.create(null);
    for (const key of Object.keys(value).sort(utf8Compare)) out[scalar(key)] = clone(value[key], depth + 1);
    return out;
  }
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
    return sha256Text(canonical(value));
  }
  async function sha256Text(source) {
    const crypto = globalThis.crypto || (typeof require === "function" ? require("node:crypto").webcrypto : null);
    need(crypto && crypto.subtle, "当前环境缺少 SHA-256，无法验证");
    const bytes = await crypto.subtle.digest("SHA-256", encoder.encode(source));
    return Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, "0")).join("");
  }
  // #1405：TB-02-A 知识域两代——旧值只在旧规则会话里 AsKnown 可读；#1343 后极值身份以根集合计。
  const KNOWLEDGE_DOMAINS = ["established_direction_without_extreme_identity_competition", "established_direction_with_extreme_root_sets"];
  const TB02_KINDS = ["CC-004.inclusion_step", "CC-005.inclusion_group", "CC-007.fractal_description", "CC-054.knowledge_state"];
  const TB02_AXES = ["CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-054", "CC-056"];
  const LEGACY_TB02_AXES = [...TB02_AXES];
  const BI_KINDS = ["CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version"];
  TB02_KINDS.push(...BI_KINDS);
  TB02_AXES.push("CC-008", "CC-009", "CC-010", "CC-055");
  const BI_AXES = [...TB02_AXES];
  const SEGMENT_KINDS = ["CC-011.segment_seed", "CC-012.feature_sequence", "CC-013.segment", "CC-055.segment_relation", "CC-055.segment_change", "CC-056.segment_version", "CC-011.seed_candidate"];
  TB02_KINDS.push(...SEGMENT_KINDS);
  TB02_AXES.push("CC-011", "CC-012", "CC-013");
  const FIRST_SEGMENT_AXES = [...TB02_AXES];
  TB02_AXES.push("CC-014");
  const INITIALIZATION_POLICY = "raw-initialization-w/1";
  const INITIALIZATION_KINDS = ["CC-004.initialization", "CC-004.undetermined_raw"];
  TB02_KINDS.push(...INITIALIZATION_KINDS);
  // #1393：只接纳本 source 已签组合；不把 #1419/#1488 新规则当成旧 schema。
  const RULE_AXES = new Map([
    ["s2-axis-quantifiers", LEGACY_TB02_AXES],
    ["s2-axis-quantifiers+new-bi/1", BI_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-first/1", FIRST_SEGMENT_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-second/1", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/1", TB02_AXES],
  ]);
  // #1393 W 单域知识域；W 规则 cut 的 TB-02-A 知识状态逐字取 W 冻结源。
  const W_KNOWLEDGE_DOMAIN = "raw_adjacent_W_initialization";
  const W_BASE_RULE = "s2-axis-quantifiers+new-bi/1+segment-second/1";
  function baseRule(rule) {
    if (rule === W_BASE_RULE + "+" + INITIALIZATION_POLICY) return W_BASE_RULE;
    need(RULE_AXES.has(rule), "未获准的完整规则组合");
    return rule;
  }
  function initializationPolicy(p) { need(p.initialization_policy_revision === INITIALIZATION_POLICY, "初始化政策版本未声明"); }
  function directionState(p) { need(["Undetermined", "Established"].includes(p.direction_state), "初始化方向状态未声明"); }
  // 由 signed-catalog.json 的全部公共静态列生成；对应漂移锁逐值重算，不是新分类语义。
  const TB02_CATALOG = Object.freeze({"catalog_revision":"s2-axis-quantifiers (SPEC-COVERAGE-INPUT.json sha256=76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c)","source_sha256":"76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c","ids":["CC-001","CC-002","CC-003","CC-004","CC-005","CC-006","CC-007","CC-008","CC-009","CC-010","CC-011","CC-012","CC-013","CC-014","CC-015","CC-016","CC-017","CC-018","CC-019","CC-020","CC-021","CC-022","CC-023","CC-024","CC-025","CC-026","CC-027","CC-028","CC-029","CC-030","CC-031","CC-032","CC-033","CC-034","CC-035","CC-036","CC-037","CC-038","CC-039","CC-040","CC-041","CC-042","CC-043","CC-044","CC-045","CC-046","CC-047","CC-048","CC-049","CC-050","CC-051","CC-052","CC-053","CC-054","CC-055","CC-056","CC-057","CC-058","CC-059","CC-060","CC-061","CC-062","LC-01","LC-02","LC-03","LC-04","LC-05","LC-06","LC-07","LC-08","LC-09","LC-10","ST-001","ST-002","ST-003","ST-004","ST-005","ST-006","ST-007","ST-008","ST-009","ST-010","ST-011","ST-012","ST-013","ST-014","ST-015","ST-016","ST-017","ST-018","ST-019","ST-020","ST-021","ST-022","ST-023","ST-024","ST-025","ST-026","ST-027","ST-028","ST-029","ST-030","ST-031","ST-032","ST-033","ST-034","ST-035","ST-036","ST-037","ST-038","ST-043","ST-044","ST-045","ST-046","ST-047","ST-048"],"public_static_sha256":"974299745db38c9b83de3ccbd1e5fc8aab93d58037fd6ef5c1667509d6da8de2"});
  function signed(value) {
    need(typeof value === "string" && value.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value), "价格须为规范整数文本");
    const n = BigInt(value); need(n >= -9223372036854775808n && n <= 9223372036854775807n, "价格超出 i64"); return n;
  }
  function texts(value) { need(Array.isArray(value), "要求文本数组"); value.forEach(text); return value; }
  function coordinates(value) {
    need(Array.isArray(value), "要求完整坐标数组");
    let last = -1n; for (const v of value) { const n = integer(v); need(n > last, "来源坐标未严格递增/重复"); last = n; } return value;
  }
  function barRef(value) {
    keys(value, ["source_coord", "open", "high", "low", "close"], "BarRef"); integer(value.source_coord);
    for (const k of ["open", "high", "low", "close"]) signed(value[k]);
  }
  function direction(value) { need(value === null || ["UP", "DOWN"].includes(value), "未知方向枚举"); }
  function directionEvidence(value) {
    if (value === null) return;
    keys(value, ["direction", "established_at", "previous_acc", "incoming", "source_coords"], "方向见证");
    need(["UP", "DOWN"].includes(value.direction), "方向见证缺方向"); integer(value.established_at);
    barRef(value.previous_acc); barRef(value.incoming); coordinates(value.source_coords);
    need(value.established_at === value.incoming.source_coord, "方向建立坐标与原 incoming 不同");
  }
  function validateTyped(record) {
    keys(record, ["object_id", "object_revision", "kind", "batch_id", "fact_key", "payload", "input_refs", "source_coords", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by", "lifecycle"], "typed 对象");
    for (const k of ["object_id", "kind", "batch_id", "fact_key", "first_known_cut"]) text(record[k]);
    need(TB02_KINDS.includes(record.kind) && record.object_revision === "1", "typed kind/revision 不符");
    const first = integer(record.first_known_generation, true); integer(record.published_generation, true);
    need(record.first_known_cut === "cut-" + record.first_known_generation && record.published_generation === record.first_known_generation, "typed 首次获知不闭合");
    if (record.withdrawn_generation === null) need(record.lifecycle === "active" && record.withdrawal_reason === null && record.superseded_by === null, "活动对象携带撤回字段");
    else {
      need(integer(record.withdrawn_generation, true) > first && record.lifecycle === "withdrawn", "撤回代际不符");
      need(["fact_removed", "superseded_by_revision"].includes(record.withdrawal_reason) && (record.withdrawal_reason === "fact_removed") === (record.superseded_by === null), "撤回原因/目标不符");
      if (record.superseded_by !== null) text(record.superseded_by);
    }
    need(Array.isArray(record.input_refs), "typed input_refs 不是数组"); coordinates(record.source_coords);
    for (const ref of record.input_refs) {
      keys(ref, ["identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord"], "typed input_ref");
      for (const value of Object.values(ref)) text(value);
      digest(ref.payload_hash); integer(ref.input_revision, true); integer(ref.revision, true); integer(ref.seq); integer(ref.source_coord);
      need(ref.input_revision === ref.revision, "输入修订引用不同");
    }
    need(equal(record.source_coords, record.input_refs.map(r => r.source_coord)), "typed 源引用/坐标不一一对应");
    const p = record.payload, kind = record.kind;
    if (SEGMENT_KINDS.includes(kind)) { validateSegmentPayload(kind, p);
    } else if (BI_KINDS.includes(kind)) { validateBiPayload(kind, p);
    } else if (kind === "CC-004.initialization") {
      keys(p, ["initialization_policy_revision", "direction_state", "first_raw_pair"], "初始化");
      initializationPolicy(p); directionState(p); directionEvidence(p.first_raw_pair);
      need((p.direction_state === "Undetermined") === (p.first_raw_pair === null), "初始化状态与首对证据不一致");
    } else if (kind === "CC-004.undetermined_raw") {
      keys(p, ["raw", "direction_state", "initialization_policy_revision", "pending_since_cut"], "未定原件");
      initializationPolicy(p); barRef(p.raw);
      need(p.direction_state === "Undetermined" && p.pending_since_cut === record.first_known_cut, "未定状态/发生cut不符");
    } else if (kind === TB02_KINDS[0]) {
      keys(p, ["action", "incoming", "acc_before", "acc_after", "direction", "direction_evidence", "comparisons", "endpoint_order", "contains", "high_sources", "low_sources", "waiting_reasons"], "包含步骤");
      need(["seed", "establish_direction", "merge", "new_group", "waiting"].includes(p.action), "步骤动作不符"); barRef(p.incoming);
      for (const k of ["acc_before", "acc_after"]) if (p[k] !== null) barRef(p[k]);
      direction(p.direction); directionEvidence(p.direction_evidence); texts(p.waiting_reasons); coordinates(p.high_sources); coordinates(p.low_sources);
      need(Array.isArray(p.comparisons) && [0, 2].includes(p.comparisons.length), "包含比较两轴不完整");
      p.comparisons.forEach((c, i) => { keys(c, ["axis", "acc", "incoming", "order"], "比较"); need(c.axis === ["high", "low"][i] && ["LT", "EQ", "GT"].includes(c.order), "比较轴/枚举不符"); signed(c.acc); signed(c.incoming); });
      need(Array.isArray(p.endpoint_order), "端点等价类不是数组");
      const labels = p.endpoint_order.flatMap(c => { need(texts(c).length > 0, "端点空类"); return c; });
      need(labels.length === 0 || (labels.length === 4 && new Set(labels).size === 4 && labels.every(x => ["acc.high", "acc.low", "incoming.high", "incoming.low"].includes(x))), "端点标签缺失/重复");
      need(p.contains === null || typeof p.contains === "boolean", "contains 不是 bool/null");
      if (p.acc_before === null) need(!labels.length && !p.comparisons.length && p.contains === null, "seed 虚构比较");
    } else if (kind === TB02_KINDS[1]) {
      const extra = Object.hasOwn(p, "initialization_policy_revision") ? ["initialization_policy_revision", "initial_direction_evidence"] : [];
      keys(p, ["group_index", "group_anchor", "members", "open", "high", "low", "close", "high_sources", "low_sources", "confirmed", "direction", "direction_evidence", "confirmation_evidence", "waiting_reasons", ...extra], "包含组");
      if (extra.length) { initializationPolicy(p); directionEvidence(p.initial_direction_evidence); need(p.initial_direction_evidence !== null, "W 正式组缺初始化证据"); }
      integer(p.group_index); integer(p.group_anchor); coordinates(p.members);
      need(p.members.length > 0 && p.members[0] === p.group_anchor, "组首未绑定首成员");
      for (const k of ["open", "high", "low", "close"]) signed(p[k]);
      for (const k of ["high_sources", "low_sources"]) need(coordinates(p[k]).length > 0 && p[k].every(c => p.members.includes(c)), "极值来源缺失/不属于成员");
      need(typeof p.confirmed === "boolean" && p.confirmed === (p.confirmation_evidence !== null), "组确认字段/见证不符"); directionEvidence(p.confirmation_evidence); direction(p.direction); directionEvidence(p.direction_evidence); texts(p.waiting_reasons);
    } else if (kind === TB02_KINDS[2]) {
      keys(p, ["shape_object_id", "branch", "window", "description", "reference_prices", "descriptive_labels", "subsequent_development"], "分型描述");
      text(p.shape_object_id); text(p.description); need(["TOP", "BOTTOM"].includes(p.branch) && coordinates(p.window).length === 3, "CC007 不是已成立顶/底");
      const names = ["left.high", "left.low", "mid.high", "mid.low", "right.high", "right.low"];
      need(Array.isArray(p.reference_prices) && p.reference_prices.length === 6, "六个参考价不完整");
      p.reference_prices.forEach((r, i) => { keys(r, ["name", "source_coord", "axis", "value"], "参考价"); need(r.name === names[i] && r.axis === names[i].split(".")[1] && r.source_coord === p.window[Math.floor(i / 2)], "参考价来源不符"); signed(r.value); });
      const labels = p.descriptive_labels; keys(labels, ["status", "reason", "labels", "source", "raw_ohlc"], "形容词未定声明");
      need(labels.status === "not_determined" && labels.reason === "no_settled_numeric_rule" && equal(labels.labels, []) && labels.source === "fenxing.md:64-74" && Array.isArray(labels.raw_ohlc), "形容词未定边界改变"); labels.raw_ohlc.forEach(barRef);
      const sub = p.subsequent_development; keys(sub, ["status", "reason", "bars"], "后续已知事实"); need(Array.isArray(sub.bars), "后续 bars 不完整");
      need(sub.bars.length ? sub.status === "observed" && sub.reason === null : sub.status === "insufficient_knowledge" && sub.reason === "no_subsequent_bar", "后续状态与原件不一致");
      for (const entry of sub.bars) {
        keys(entry, ["bar", "relations"], "后续 bar"); barRef(entry.bar); need(Array.isArray(entry.relations) && entry.relations.length === 24, "后续四价×六参考关系不完整");
        entry.relations.forEach((r, i) => { const axis = ["open", "high", "low", "close"][Math.floor(i / 6)], ref = p.reference_prices[i % 6]; keys(r, ["axis", "value", "reference", "reference_value", "order"], "后续关系"); need(r.axis === axis && r.value === entry.bar[axis] && r.reference === ref.name && r.reference_value === ref.value && ["LT", "EQ", "GT"].includes(r.order), "后续关系来源不同"); });
      }
    } else {
      const extra = Object.hasOwn(p, "initialization_policy_revision") ? ["initialization_policy_revision", "direction_state"] : [];
      keys(p, ["scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests", ...extra], "知识状态");
      if (extra.length) { initializationPolicy(p); directionState(p); }
      need(p.scope === "TB-02-A" && (extra.length ? p.domain === W_KNOWLEDGE_DOMAIN : KNOWLEDGE_DOMAINS.includes(p.domain)) && equal(p.known_facts, ["raw_ohlc", "inclusion_trace", "group_provenance"]), "知识域/已知事实不符");
      integer(p.input_frontier, false, true); texts(p.waiting_reasons); need(Array.isArray(p.requests), "逐请求集合不完整"); const ids = new Set();
      const enums = {input_quality:["SUFFICIENT", "INSUFFICIENT", "INCONSISTENT"], applicability:["IN_DOMAIN", "OUTSIDE_DOMAIN", "DOMAIN_PROOF_MISSING"], computation:["COMPLETE", "INCOMPLETE"], validity:["CURRENT", "SUPERSEDED", "WITHDRAWN"]};
      for (const r of p.requests) {
        keys(r, ["request_id", "subject_id", "axes", "reasons", "source_coords"], "请求状态"); text(r.request_id); need(!ids.has(r.request_id), "请求身份重复"); ids.add(r.request_id);
        if (r.subject_id !== null) text(r.subject_id); keys(r.axes, Object.keys(enums), "请求四轴");
        for (const k of Object.keys(enums)) need(enums[k].includes(r.axes[k]), "请求四轴枚举不符"); texts(r.reasons); coordinates(r.source_coords);
      }
    }
    const slot = kind === "CC-004.initialization" ? "W" : kind === "CC-004.undetermined_raw" ? p.raw.source_coord : [...BI_KINDS, ...SEGMENT_KINDS].includes(kind) ? p.slot : kind === TB02_KINDS[0] ? p.incoming.source_coord : kind === TB02_KINDS[1] ? p.group_anchor : kind === TB02_KINDS[2] ? p.window : "TB-02-A";
    need(record.fact_key === canonical([kind, slot]), "事实语义槽未绑定 payload");
  }
  function validateSegmentPayload(kind, p) {
    keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "段合同");
    const v2 = p.schema_revision === "s-segment/2";
    need(p.schema_revision === (v2 ? "s-segment/2" : "s-segment/1") && p.semantic_version === (v2 ? "segment-second/1" : "segment-seed-first/1") && p.policy_version === (v2 ? "seed_closed_strict_second" : "seed_closed_first_no_gap") && p.view_role === "main", "段规则/范围版本不符");
    integer(p.object_generation, true);
    const d = p.data, refs = new Map();
    let segmentStart = null;
    function known(k) {
      keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "段获知时点");
      integer(k.generation, true); integer(k.input_frontier, false, true);
      for (const name of ["receipt_id", "received_at"]) if (k[name] !== null) text(k[name]);
      if (k.semantic_commit_ns !== null) integer(k.semantic_commit_ns);
    }
    for (const r of d.stroke_refs || []) {
      keys(r, ["stroke_index", "entity_id", "start_anchor", "end_anchor", "start_price", "end_price", "start_roots", "end_roots"], "段笔引用");
      for (const name of ["stroke_index", "start_anchor", "end_anchor"]) integer(r[name]);
      signed(r.start_price); signed(r.end_price);
      need(coordinates(r.start_roots).length > 0 && coordinates(r.end_roots).length > 0, "段笔缺实际根");
      need(r.entity_id === `bi:${p.object_generation}:${r.start_anchor}` && !refs.has(r.stroke_index), "段笔身份不符/重复");
      refs.set(r.stroke_index, r);
    }
    function indices(xs) { coordinates(xs); need(xs.every(i => refs.has(i)), "段事实引用未绑定笔"); }
    function construction(v) {
      keys(v, ["start_stroke", "stroke_indices", "length", "vector", "overlap", "overlap_relation", "direction", "failed_conditions"], "段完整条件");
      integer(v.start_stroke); indices(v.stroke_indices);
      const n = v.stroke_indices.length, length = n < 3 ? `n=${n}` : n % 2 === 0 ? "even_ge3" : "odd_ge3";
      need(v.length === length, "段长度分域不符");
      need(v.stroke_indices.every((x,i) => BigInt(x) === BigInt(v.start_stroke) + BigInt(i)), "候选笔片段不连续");
      need(length === "odd_ge3" ? Array.isArray(v.vector) && v.vector.length === 4 && v.vector.every(x => typeof x === "boolean") : v.vector === null, "段四条件缺位/不适用虚填");
      if (n >= 3) { need(Array.isArray(v.overlap) && v.overlap.length === 2 && ["OVERLAP", "TOUCH", "DISJOINT"].includes(v.overlap_relation), "缺闭区间见证"); v.overlap.forEach(signed); }
      else need(v.overlap === null && v.overlap_relation === null, "不足三笔虚填公共区间");
      texts(v.failed_conditions);
      need(equal(v.vector, [true,true,true,true]) ? ["UP", "DOWN"].includes(v.direction) && !v.failed_conditions.length : v.direction === null, "段方向与适用性不符");
    }
    function element(e) {
      keys(e, ["high", "low", "stroke_idx", "stroke_indices"], "FIRST元素"); signed(e.high); signed(e.low); indices(e.stroke_indices);
      need(e.stroke_indices.includes(e.stroke_idx), "元素代表笔不属于来源");
    }
    if ([SEGMENT_KINDS[0], SEGMENT_KINDS[6]].includes(kind)) {
      keys(d, ["entity_id", "construction", "stroke_refs"], kind); construction(d.construction);
      need((kind === SEGMENT_KINDS[0]) === equal(d.construction.vector,[true,true,true,true]), "未满足完整条件的候选冒充seed");
    } else if (kind === SEGMENT_KINDS[1]) {
      keys(d, ["entity_id", "segment_id", "role", "direction", "member_direction", "facts", "stroke_refs", "scan", "second_sequence_status"], kind);
      const second = v2 && d.role === "SECOND_SEQUENCE";
      need((v2 ? ["FIRST_SEQUENCE", "SECOND_SEQUENCE"] : ["FIRST_SEQUENCE"]).includes(d.role) && ["UP", "DOWN"].includes(d.member_direction) && (second ? ["PENDING", "CONFIRMED"] : v2 ? ["NOT_APPLICABLE", "PENDING", "CONFIRMED"] : ["NOT_APPLICABLE", "WAITING_FOLLOWUP"]).includes(d.second_sequence_status), "序列角色/适用性不符"); direction(d.direction);
      keys(d.scan, ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "tail_cache_window"], "扫描范围");
      for (const [k,v] of Object.entries(d.scan)) if (k !== "resource_truncated") integer(v);
      need(typeof d.scan.resource_truncated === "boolean", "截断状态错型");
      need(refs.size > 0, "特征序列缺段笔引用");
      // 身份锚＝stroke_refs 中数值最小的笔号，即段起笔；wire 整数是十进制文本，须按数值取最小。
      segmentStart = [...refs.keys()].reduce((a, b) => BigInt(b) < BigInt(a) ? b : a);
      need(second || d.scan.from_stroke === segmentStart, "FIRST扫描起点不是段起笔");
      const f = d.facts; keys(f, ["raw_elements", "standard", "steps", "checks", ...(second ? ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "matched"] : [])], "序列事实");
      for (const k of ["raw_elements", "standard", "steps", "checks"]) need(Array.isArray(f[k]), "序列事实集合错型");
      if (second) {
        integer(f.from_stroke); integer(f.through_stroke, false, true); integer(f.input_stroke_count);
        need(typeof f.resource_truncated === "boolean" && d.scan.tail_cache_window === "0" && ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated"].every(k => f[k] === d.scan[k]) && BigInt(d.scan.from_stroke) > BigInt(segmentStart), "SECOND扫描边界不符");
      }
      [...f.raw_elements, ...f.standard].forEach(element);
      for (const s of f.steps) {
        keys(s, ["incoming", "before", "after", "action", "merge_direction", "scope"], "包含步骤"); element(s.incoming); element(s.after); if (s.before !== null) element(s.before);
        need((second ? ["append", "merge"] : ["append", "merge", "turn_boundary"]).includes(s.action) && s.scope === (second ? "SECOND_SEQUENCE" : s.action === "turn_boundary" ? "TURN_HYPOTHESIS" : "SAME_SEQUENCE"), "包含跨作用域/动作未声明");
        need(s.action === "merge" ? ["UP", "DOWN"].includes(s.merge_direction) : s.merge_direction === null, "方向性包含缺见证");
      }
      function secondCheck(c) {
        keys(c, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND分型见证");
        need(Array.isArray(c.elements) && c.elements.length === 3, "SECOND分型窗口缺元素"); c.elements.forEach(element);
        need(typeof c.top_fractal === "boolean" && typeof c.bottom_fractal === "boolean", "SECOND分型错型"); indices([c.through_stroke]);
      }
      if (second) {
        need((f.matched === null) === (d.second_sequence_status === "PENDING"), "SECOND命中与状态不符");
        if (f.matched !== null) { secondCheck(f.matched); need(f.checks.some(c => equal(c, f.matched)) && (f.matched.top_fractal || f.matched.bottom_fractal), "SECOND命中未绑定检查"); }
      }
      for (const c of f.checks) {
        if (second) { secondCheck(c); continue; }
        keys(c, ["elements", "first_fractal", "gap", "gap_closed_by_c", "provisional", "turn_hypothesis_pair"], "分型见证");
        need(Array.isArray(c.elements) && c.elements.length === 3, "分型窗口缺元素"); c.elements.forEach(element);
        need(equal(c.turn_hypothesis_pair, c.first_fractal ? c.elements.slice(0,2).map(e => e.stroke_idx) : null), "假设转折点两侧未绑定a/b元素");
        need(typeof c.first_fractal === "boolean" && typeof c.provisional === "boolean", "分型事实错型");
        for (const k of ["gap", "gap_closed_by_c"]) need(c[k] === null || typeof c[k] === "boolean", "gap见证错型");
        need((c.first_fractal || c.gap === null) && (c.gap === true || c.gap_closed_by_c === null), "不适用条件虚填布尔");
      }
    } else if (kind === SEGMENT_KINDS[2]) {
      keys(d, ["entity_id", "entity_revision", "seed_id", "sequence_id", "sequence_object_id", "construction", "result", "waiting_reasons", "termination_attempts", "geometric_end", "trigger_stroke", "formed_known_at", "terminated_known_at", "stroke_refs", ...(v2 ? ["second_sequence_id", "second_sequence_object_id", "strict", "fact_vector"] : [])], kind);
      integer(d.entity_revision, true); construction(d.construction); texts(d.waiting_reasons); known(d.formed_known_at);
      need(Array.isArray(d.termination_attempts), "终结准入见证错型");
      for (const a of d.termination_attempts) {
        keys(a, ["end_stroke", "trigger_stroke", "minimum_count_met", "construction"], "终结准入见证");
        indices([a.end_stroke]); indices([a.trigger_stroke]); need(typeof a.minimum_count_met === "boolean", "最小笔数条件错型"); construction(a.construction);
      }
      need(["NO_FIRST", "CASE_ONE", "PENDING_CASE_TWO", "FAILED_GEOMETRY", ...(v2 ? ["CASE_TWO"] : [])].includes(d.result), "冒称交付第二种终结");
      if (["CASE_ONE", "CASE_TWO"].includes(d.result)) {
        known(d.terminated_known_at); const e = d.geometric_end;
        keys(e, ["stroke_index", "group_anchor", "extreme_roots", "price"], "几何段端"); indices([e.stroke_index]); indices([d.trigger_stroke]); coordinates(e.extreme_roots); integer(e.group_anchor); signed(e.price);
      } else need(d.geometric_end === null && d.trigger_stroke === null && d.terminated_known_at === null, "发展态虚填终结");
      if (v2) {
        const strict = d.strict;
        if (strict !== null) {
          keys(strict, ["elements", "raw_gap", "gap_closed_by_c", "effective_case", "closure_operator", "closure_left", "closure_right"], "strict事实");
          need(Array.isArray(strict.elements) && strict.elements.length === 3, "strict缺FIRST窗口"); strict.elements.forEach(element);
          need(typeof strict.raw_gap === "boolean" && (strict.raw_gap ? typeof strict.gap_closed_by_c === "boolean" : strict.gap_closed_by_c === null), "strict适用性不符");
          need(["CASE_ONE", "CASE_TWO"].includes(strict.effective_case) && ["<=", ">="].includes(strict.closure_operator), "strict规则未声明"); signed(strict.closure_left); signed(strict.closure_right);
          const a = strict.elements[0], c = strict.elements[2], up = strict.closure_operator === "<=";
          need(equal([strict.closure_operator, strict.closure_left, strict.closure_right], up ? ["<=", c.low, a.high] : [">=", c.high, a.low]), "strict比较未绑定FIRST元素");
        }
        const terminated = ["CASE_ONE", "CASE_TWO"].includes(d.result);
        need(!terminated || strict !== null && strict.effective_case === d.result, "终结缺适用strict对象");
        need((d.second_sequence_id === null) === (d.second_sequence_object_id === null), "SECOND身份/对象缺位");
        if (d.second_sequence_id !== null) { need(d.second_sequence_id === "second:" + d.entity_id, "SECOND身份未绑定候选"); text(d.second_sequence_object_id); }
        need(!["CASE_TWO", "PENDING_CASE_TWO"].includes(d.result) || d.second_sequence_id !== null, "第二种缺SECOND引用");
        const b = d.fact_vector;
        keys(b, ["stroke_broken", "new_segment_formed", "segment_broken", "stroke_break_witness", "scan_from_stroke", "scan_through_stroke", "scan_complete", "termination_witness"], "独立破坏事实");
        need(["stroke_broken", "scan_complete"].every(k => typeof b[k] === "boolean") && ["new_segment_formed", "segment_broken"].every(k => b[k] === null || typeof b[k] === "boolean"), "破坏事实错型");
        integer(b.scan_from_stroke); integer(b.scan_through_stroke, false, true);
        need((b.new_segment_formed === true) === terminated && b.segment_broken === b.new_segment_formed && (b.new_segment_formed === null) === !b.scan_complete, "段破坏适用性不符");
        const certificate = b.termination_witness;
        need((certificate !== null) === (b.new_segment_formed === true), "段破坏缺同次终结证书");
        if (certificate !== null) {
          keys(certificate, ["kind", "strict", "second_match", "construction", "minimum_count_met", "through_stroke"], "终结证书");
          need(certificate.kind === d.result && equal(certificate.strict, strict) && equal(certificate.construction, d.construction), "终结证书未绑定同次strict/几何");
          construction(certificate.construction);
          need(certificate.minimum_count_met === true && equal(certificate.construction.vector, [true,true,true,true]) && certificate.through_stroke === d.trigger_stroke, "终结证书缺有效准入/实际前沿");
          indices([certificate.through_stroke]);
          const match = certificate.second_match;
          need((match !== null) === (certificate.kind === "CASE_TWO"), "终结证书SECOND适用性不符");
          if (match !== null) {
            keys(match, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND终结证书");
            need(Array.isArray(match.elements) && match.elements.length === 3 && typeof match.top_fractal === "boolean" && typeof match.bottom_fractal === "boolean" && (match.top_fractal || match.bottom_fractal), "终结证书缺任意分型见证");
            match.elements.forEach(element); indices([match.through_stroke]);
          }
        }
        const w = b.stroke_break_witness;
        need((w !== null) === b.stroke_broken, "笔破坏缺独立见证");
        if (w !== null) {
          keys(w, ["earlier_stroke", "later_stroke", "earlier_price", "later_price", "comparison"], "笔破坏见证");
          indices([w.earlier_stroke, w.later_stroke]); signed(w.earlier_price); signed(w.later_price); need(["<=", ">="].includes(w.comparison), "笔破坏比较未声明");
        }
      }
    } else if (kind === SEGMENT_KINDS[3]) {
      keys(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], kind); Object.values(d).forEach(text); need(["member_of", "derived_from"].includes(d.relation_kind), "段关系未声明");
    } else if (kind === SEGMENT_KINDS[4]) {
      keys(d, ["entity_id", "change", "before", "after", "known_at", "object_id"], kind); known(d.known_at);
      need(["formed", "updated", "terminated", "withdrawn"].includes(d.change), "段变化未声明");
      for (const k of ["before", "after"]) if (d[k] !== null) validateSegmentPayload(SEGMENT_KINDS[2], {...p, slot:d[k].entity_id, object_generation:d[k].entity_id.split(":")[1], data:d[k]});
    } else {
      keys(d, ["input_frontier", "waiting_reasons", "known_at", "delivered_scope", "deferred_scope"], kind); integer(d.input_frontier, false, true); known(d.known_at); texts(d.waiting_reasons);
      need(d.delivered_scope === (v2 ? "seed_strict_first_and_finite_second" : "seed_and_no_gap_first_kind") && equal(d.deferred_scope, v2 ? ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"] : ["second_kind", "general_equal_identity"]), "段交付范围不符");
    }
    if ([...SEGMENT_KINDS.slice(0,3), SEGMENT_KINDS[6]].includes(kind)) {
      if (kind === SEGMENT_KINDS[1]) {
        const expected = `segment:${p.object_generation}:${refs.get(segmentStart).start_anchor}`;
        need(d.segment_id === expected && d.entity_id === (d.role === "SECOND_SEQUENCE" ? "second:" : "first:") + expected && ((d.direction === d.member_direction) === (d.role === "SECOND_SEQUENCE")), "FIRST身份/反向成员不符");
      } else {
        const anchor = refs.size ? refs.get(d.construction.start_stroke).start_anchor : "empty";
        const expected = `${[SEGMENT_KINDS[0], SEGMENT_KINDS[6]].includes(kind) ? "seed" : "segment"}:${p.object_generation}:${anchor}`;
        need(d.entity_id === expected, "段身份未绑定代际/起笔");
        if (kind === SEGMENT_KINDS[2]) {
          need(d.seed_id === `seed:${p.object_generation}:${anchor}` && d.sequence_id === "first:" + expected, "段缺同源seed/FIRST身份"); text(d.sequence_object_id);
          if (d.geometric_end !== null) {
            const e = d.geometric_end, r = refs.get(e.stroke_index);
            need(e.group_anchor === r.end_anchor && e.price === r.end_price && equal(e.extreme_roots,r.end_roots), "几何段端与实际笔端不符");
          }
        }
      }
    }
    const slot = kind === SEGMENT_KINDS[3] ? [d.source_id, d.relation_kind, d.target_id] : kind === SEGMENT_KINDS[4] ? [d.known_at.generation, d.entity_id, d.change] : kind === SEGMENT_KINDS[5] ? (v2 ? "TB-02-D" : "TB-02-C") : d.entity_id;
    need(equal(p.slot, slot), "段语义槽身份不符");
  }
  // #1405（#1343）：新笔合同两代并存——s-new-bi/1 旧对象按 AsKnown 保留；s-new-bi/2 端点带极值根集
  // 首末 raw 序位，同型只有 KEEP/REPLACE 并带同价证据，policy_version 并列记录 raw 间隔门与同价策略。
  const SAME_PRICE_KEEP_EVIDENCE = "same_kind_equal_price_keep_earlier";
  const BI_VERSIONS = {
    "s-new-bi/1": ["new-bi-dual-coordinate/1", "standard_raw_gap_3"],
    "s-new-bi/2": ["new-bi-dual-coordinate/2", "standard_raw_gap_3;same-price/keep-earlier+extreme-region/1"],
  };
  // #1405：会话级 rule_revision 绑定（新笔合同, 段合同, TB-02-A 知识域）代际；旧规则会话只读（AsKnown）。
  const RULE_CONTRACTS = {
    "s2-axis-quantifiers": [null, null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1": ["s-new-bi/1", null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-first/1": ["s-new-bi/1", "s-segment/1", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-second/1": ["s-new-bi/1", "s-segment/2", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/2+segment-second/1": ["s-new-bi/2", "s-segment/2", KNOWLEDGE_DOMAINS[1]],
  };
  const RETIRED_SAME_PRICE_WAITING = ["raw_extreme_identity_tie", "actual_extreme_mapping_missing", "same_kind_endpoint_identity_tie", "earlier_endpoint_identity_unsettled"];
  function validateBiPayload(kind, p) {
    keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "新笔合同");
    const version = Object.hasOwn(BI_VERSIONS, p.schema_revision) ? BI_VERSIONS[p.schema_revision] : null;
    need(version !== null && p.semantic_version === version[0] && p.policy_version === version[1] && p.view_role === "main", "新笔规则/政策版本不符");
    const v2 = p.schema_revision === "s-new-bi/2";
    integer(p.object_generation, true);
    const d = p.data;
    function waiting(value) { texts(value); need(!v2 || !value.some(w => RETIRED_SAME_PRICE_WAITING.includes(w)), "同价域退役的等待态在 v2 对象复现"); }
    function endpoint(e) {
      keys(e, ["kind", "merged_index", "group_anchor", "price", "extreme_roots", ...(v2 ? ["raw_first", "raw_last"] : ["raw_position"]), "source_coords", "sealed_at", "waiting_reasons"], "新笔端点");
      need(["TOP", "BOTTOM"].includes(e.kind), "端点类型不符"); integer(e.merged_index); integer(e.group_anchor); signed(e.price);
      coordinates(e.extreme_roots); coordinates(e.source_coords); waiting(e.waiting_reasons);
      need(e.extreme_roots.every(c => e.source_coords.includes(c)), "实际根缺原始来源");
      if (v2) {
        const first = integer(e.raw_first), last = integer(e.raw_last);
        need(e.extreme_roots.length > 0 && last >= first && last - first >= BigInt(e.extreme_roots.length - 1), "极值根集与首末 raw 序位不闭合");
      } else if (e.raw_position !== null) integer(e.raw_position);
      if (e.sealed_at !== null) integer(e.sealed_at);
    }
    function conditions(c) {
      keys(c, ["merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"], "新笔完整条件");
      integer(c.merged_gap); if (v2 || c.raw_between_actual_extrema !== null) integer(c.raw_between_actual_extrema); signed(c.top_price); signed(c.bottom_price);
      need(Array.isArray(c.vector) && c.vector.length === 3 && c.vector.every(v => typeof v === "boolean" || (!v2 && v === null)), "条件向量缺位/错型");
      texts(c.failed_conditions); waiting(c.waiting_reasons);
    }
    function known(k) {
      keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "新笔获知时点");
      integer(k.generation, true); integer(k.input_frontier); text(k.receipt_id); text(k.received_at); if (k.semantic_commit_ns !== null) integer(k.semantic_commit_ns);
    }
    if (kind === BI_KINDS[0]) endpoint(d);
    else if ([BI_KINDS[1], BI_KINDS[2]].includes(kind)) {
      keys(d, ["old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", ...(v2 ? ["evidence"] : []), "waiting_reasons"], "端点对");
      endpoint(d.old); endpoint(d.new); need(d.endpoint_kind_pair === d.old.kind + "/" + d.new.kind, "端点分域标签不符");
      if (kind === BI_KINDS[1]) { need(d.old.kind !== d.new.kind, "异型请求实际同型"); conditions(d.conditions); need(d.comparison === null && d.selection === null && (!v2 || equal(d.evidence, [])), "异型域混入同型选择"); }
      else {
        need(d.old.kind === d.new.kind && d.conditions === null && ["LT", "EQ", "GT"].includes(d.comparison) && (v2 ? ["KEEP", "REPLACE"] : ["KEEP", "REPLACE", "UNDETERMINED"]).includes(d.selection), "同型域混入假条件或未声明取舍");
        if (v2) {
          // 一个比较器三输出（#1343 Q1）：严格更强 ⟹ REPLACE；更弱或同价 ⟹ KEEP 在先者。
          const stronger = d.comparison === (d.old.kind === "TOP" ? "GT" : "LT");
          need(d.selection === (stronger ? "REPLACE" : "KEEP"), "同型取舍与价格比较不闭合");
          need(equal(d.retained_anchors, [stronger ? d.new.group_anchor : d.old.group_anchor]), "同型保留锚不符取舍");
          need(equal(d.evidence, d.comparison === "EQ" ? [SAME_PRICE_KEEP_EVIDENCE] : []), "同价保留在先者的证据缺失或混入");
        }
      }
      coordinates(d.retained_anchors); if (v2) texts(d.evidence); waiting(d.waiting_reasons);
    } else if (kind === BI_KINDS[3]) {
      keys(d, ["identity_anchor", "start", "end", "formation", "confirmation", "entity_id", "entity_revision", "state", "formed_known_at", "confirmed_known_at", "formed_evidence", "version_causes"], "新笔实体");
      integer(d.identity_anchor); integer(d.entity_revision, true); text(d.entity_id); endpoint(d.start); endpoint(d.end); conditions(d.formation); conditions(d.formed_evidence); known(d.formed_known_at); texts(d.version_causes);
      need(d.state === (d.confirmation === null ? "FORMED_UNCONFIRMED" : "CONFIRMED"), "生命周期/确认见证不符");
      need(d.entity_id === "bi:" + p.object_generation + ":" + d.identity_anchor, "笔身份未绑定代际与形成起点");
      if (d.confirmation !== null) {
        const c = d.confirmation; keys(c, ["successor_start", "successor_end", "successor_conditions", "right_group_sealed_at", "source_coords"], "确认见证");
        integer(c.successor_start); integer(c.successor_end); integer(c.right_group_sealed_at); coordinates(c.source_coords); conditions(c.successor_conditions); known(d.confirmed_known_at);
      } else need(d.confirmed_known_at === null, "无确认见证却有确认时点");
    } else if (kind === BI_KINDS[4]) {
      keys(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], "新笔关系"); Object.values(d).forEach(text);
      need(["member_of", "derived_from", "successor_of", "leaves", "retests", "returns_to", "extends", "newborn_after", "expands_with", "caused_turn_at", "confirms", "selected_from"].includes(d.relation_kind), "未声明关系种类");
    } else if (kind === BI_KINDS[5]) {
      keys(d, ["entity_id", "change", "before", "after", "known_at", "event_at", "causes", "object_id"], "新笔变化");
      text(d.entity_id); need(["formed", "extended", "endpoint_replaced", "confirmed", "source_or_boundary_updated", "withdrawn"].includes(d.change), "未声明变化"); known(d.known_at); signed(d.event_at); texts(d.causes);
      for (const k of ["before", "after"]) if (d[k] !== null) {
        // after 与本事件同版本；before 是先前已发布对象，按其自身代际形状校验（AsKnown）。
        const schema = k === "after" ? p.schema_revision : (object(d[k].start) && Object.hasOwn(d[k].start, "raw_first") ? "s-new-bi/2" : "s-new-bi/1");
        validateBiPayload(BI_KINDS[3], {...p, schema_revision:schema, semantic_version:BI_VERSIONS[schema][0], policy_version:BI_VERSIONS[schema][1], slot:d[k].entity_id, object_generation:d[k].entity_id.split(":")[1], data:d[k]});
      }
    } else {
      keys(d, ["input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"], "新笔版本");
      integer(d.input_frontier, false, true); texts(d.version_causes); waiting(d.waiting_reasons); known(d.known_at);
    }
    const slot = kind === BI_KINDS[0] ? "fx:" + p.object_generation + ":" + d.group_anchor :
      [BI_KINDS[1], BI_KINDS[2]].includes(kind) ? [p.object_generation, d.old.group_anchor, d.new.group_anchor] :
      kind === BI_KINDS[3] ? d.entity_id :
      kind === BI_KINDS[4] ? [d.source_id, d.relation_kind, d.target_id] :
      kind === BI_KINDS[5] ? [d.known_at.generation, d.entity_id, d.change] : "TB-02-B";
    need(equal(p.slot, slot), "新笔 slot 未绑定其具名对象身份");
  }
  function validateTB02ShapeRefs(shape) {
    need(shape.kind === "CC-006.local_shape" && Array.isArray(shape.input_refs) && shape.input_refs.length === 3, "OHLC形态缺完整三组");
    const anchors = [shape.window_start, shape.window_mid, shape.window_end], all = new Set();
    let previousIndex = null;
    shape.input_refs.forEach((g, index) => {
      keys(g, ["merged_index", "raw_refs", "dependency_refs"], "OHLC形态来源组");
      const mergedIndex = integer(g.merged_index);
      need(previousIndex === null || mergedIndex === previousIndex + 1n, "形态来源组序号不连续"); previousIndex = mergedIndex;
      const sets = {};
      for (const role of ["raw_refs", "dependency_refs"]) {
        need(Array.isArray(g[role]), "形态来源角色不是数组");
        for (const r of g[role]) {
          keys(r, ["identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord"], "形态来源引用");
          Object.values(r).forEach(text); digest(r.payload_hash); integer(r.revision, true); integer(r.input_revision, true); integer(r.seq); integer(r.source_coord);
          need(r.input_revision === r.revision, "形态来源修订不符"); all.add(r.source_coord);
        }
        sets[role] = new Set(coordinates(g[role].map(r => r.source_coord)));
      }
      need(sets.raw_refs.size > 0 && [...sets.dependency_refs].every(c => !sets.raw_refs.has(c)), "形态将构造/边界依据伪作真实成员");
      need(g.raw_refs[0].source_coord === anchors[index], "形态实际成员首坐标与窗口锚不同");
    });
    need(equal(coordinates(shape.source_coords), [...all].sort((a,b) => BigInt(a)<BigInt(b)?-1:BigInt(a)>BigInt(b)?1:0)), "形态来源不是成员与构造/边界依据并集");
  }
  function validateTB02Catalog(catalog) {
    need(catalog.catalog_revision === TB02_CATALOG.catalog_revision, "未绑定受信冻结目录版本");
    need(Array.isArray(catalog.items) && catalog.items.length === TB02_CATALOG.ids.length, "完整冻结目录数量不符");
    const ids = new Set(), actual = {};
    for (const item of catalog.items) {
      keys(item, ["id", "kind", "title", "domain", "branches", "implementation_status", "proof_status", "run_status", "evidence"], "目录项");
      need(TB02_CATALOG.ids.includes(item.id) && !ids.has(item.id), "冻结目录身份缺失/重复/额外"); ids.add(item.id);
      actual[item.id] = Object.fromEntries(["kind", "title", "domain", "branches"].map(k => [k, item[k]]));
    }
    for (const [field, status, key] of [["implemented","implemented","implementation_status"], ["not_implemented","not_implemented","implementation_status"], ["run","run","run_status"], ["not_run","not_run","run_status"]])
      need(catalog.counts[field] === String(catalog.items.filter(i => i[key] === status).length), "目录状态计数不符");
    return actual;
  }
  const RAW_REF_KEYS = ["identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord"];
  const rawReference = raw => Object.fromEntries(RAW_REF_KEYS.map(k => [k, raw[k]]));
  function validateTB02PayloadSources(record, rawById) {
    const sources = new Set(record.source_coords), refs = new Map(record.input_refs.map(r => [r.source_coord, r]));
    function walk(value, key = null) {
      if (Array.isArray(value)) {
        if (["members", "high_sources", "low_sources", "source_coords", "window"].includes(key))
          need(value.every(c => sources.has(c)), "typed payload 使用未绑定的原始坐标");
        else value.forEach(v => walk(v));
      } else if (object(value)) {
        for (const [name, child] of Object.entries(value)) walk(child, name);
      } else if (["source_coord", "group_anchor", "established_at"].includes(key)) {
        need(sources.has(value), "typed payload 使用未绑定的原始坐标");
      }
    }
    walk(record.payload);
    const p = record.payload, bars = [];
    if (record.kind === "CC-004.undetermined_raw") bars.push(p.raw);
    if (record.kind === "CC-004.initialization" && p.first_raw_pair !== null)
      bars.push(p.first_raw_pair.previous_acc, p.first_raw_pair.incoming);
    if (record.kind === TB02_KINDS[0]) {
      bars.push(p.incoming);
      if (p.direction_evidence !== null) bars.push(p.direction_evidence.incoming);
    }
    if (record.kind === TB02_KINDS[1]) {
      for (const name of ["direction_evidence", "confirmation_evidence"])
        if (p[name] !== null) bars.push(p[name].incoming);
      if (Object.hasOwn(p, "initial_direction_evidence"))
        bars.push(p.initial_direction_evidence.previous_acc, p.initial_direction_evidence.incoming);
    }
    if (record.kind === TB02_KINDS[2]) bars.push(...p.descriptive_labels.raw_ohlc, ...p.subsequent_development.bars.map(b => b.bar));
    for (const bar of bars) {
      const ref = refs.get(bar.source_coord);
      need(ref, "typed payload 使用未绑定的原始坐标");
      const raw = rawById.get(canonical([ref.identity_key, ref.revision]));
      need(raw && ["source_coord", "open", "high", "low", "close"].every(k => bar[k] === raw[k]), "事实中原始 OHLC 值与真实源记录不符");
    }
  }
  function validateTB02InitializationLinks(records) {
    const initial = records.filter(r => r.kind === "CC-004.initialization");
    need(initial.length === 1, "同 cut 缺少唯一初始化对象");
    const p = initial[0].payload, initialRefs = new Map(initial[0].input_refs.map(r => [r.source_coord, r]));
    for (const record of records) {
      if (record.kind === TB02_KINDS[1]) {
        need(p.direction_state === "Established" && equal(record.payload.initial_direction_evidence, p.first_raw_pair), "组初始化证据与同 cut 初始化对象不一致");
        const refs = new Map(record.input_refs.map(r => [r.source_coord, r]));
        for (const [coord, ref] of initialRefs) need(equal(refs.get(coord), ref), "组初始化引用与同 cut 原始修订不同");
      } else if (record.kind === "CC-004.undetermined_raw") {
        need(p.direction_state === "Undetermined", "已定初始化混入未定原件");
        for (const ref of record.input_refs) need(equal(initialRefs.get(ref.source_coord), ref), "未定原件引用与同 cut 初始化不同");
      } else if (record.kind === TB02_KINDS[3]) {
        need(record.payload.direction_state === p.direction_state, "知识方向状态与同 cut 初始化不同");
      }
    }
  }
  function validateTB02CutSources(snapshot, ruleRevision = null) {
    const rule = ruleRevision === null ? null : baseRule(ruleRevision);
    const records = [...snapshot.objects, ...snapshot.withdrawn_objects], rawById = new Map(), effective = new Map(), groups = new Map();
    for (const raw of snapshot.raw_history) {
      validateTB02Raw(raw); const key = canonical([raw.identity_key, raw.revision]);
      need(!rawById.has(key), "完整原始历史身份重复"); rawById.set(key, raw);
      const previous = effective.get(raw.source_coord);
      if (!previous || BigInt(raw.revision) > BigInt(previous.revision)) effective.set(raw.source_coord, raw);
    }
    for (const record of records) {
      if (TB02_KINDS.includes(record.kind)) validateTyped(record); else validateTB02ShapeRefs(record);
      const refs = TB02_KINDS.includes(record.kind) ? record.input_refs : record.input_refs.flatMap(g => [...g.raw_refs, ...g.dependency_refs]);
      for (const ref of refs) {
        const raw = rawById.get(canonical([ref.identity_key, ref.revision]));
        need(raw && equal(ref, rawReference(raw)), "对象八键来源未绑定完整原始历史");
        if (record.lifecycle === "active") need(equal(ref, rawReference(effective.get(ref.source_coord) || {})), "活动对象使用旧原始修订");
      }
      if (TB02_KINDS.includes(record.kind)) validateTB02PayloadSources(record, rawById);
      if (record.kind === TB02_KINDS[1]) {
        const anchor = record.payload.group_anchor, versions = groups.get(anchor) || [];
        versions.push(record); groups.set(anchor, versions);
      }
    }
    const wPolicy = ruleRevision === null ? records.some(r => INITIALIZATION_KINDS.includes(r.kind) || Object.hasOwn(r.payload || {}, "initialization_policy_revision")) : rule !== ruleRevision;
    if (wPolicy && snapshot.generation !== "0") {
      // 逐发生边界投影历史活动集，撤回组不能借当前初始化对象通过校验。
      const relevant = records.filter(r => INITIALIZATION_KINDS.includes(r.kind) || [TB02_KINDS[1], TB02_KINDS[3]].includes(r.kind));
      const events = new Map(), current = integer(snapshot.generation), active = new Map();
      function event(cut) {
        if (!events.has(cut)) events.set(cut, {born: [], died: []});
        return events.get(cut);
      }
      event(1n); event(current);
      for (const record of relevant) {
        event(integer(record.first_known_generation)).born.push(record);
        if (record.withdrawn_generation !== null) event(integer(record.withdrawn_generation)).died.push(record);
      }
      for (const cut of [...events.keys()].filter(g => g <= current).sort((a, b) => a < b ? -1 : a > b ? 1 : 0)) {
        const changes = events.get(cut);
        for (const record of changes.died) active.delete(record.object_id);
        for (const record of changes.born) active.set(record.object_id, record);
        validateTB02InitializationLinks([...active.values()]);
      }
    }
    const bound = new Map();
    for (const shape of records.filter(r => r.kind === "CC-006.local_shape")) {
      const cut = integer(shape.lifecycle === "active" ? snapshot.generation : shape.first_known_generation);
      const selected = [], anchors = [shape.window_start, shape.window_mid, shape.window_end];
      shape.input_refs.forEach((ref, index) => {
        const versions = (groups.get(anchors[index]) || []).filter(g => integer(g.first_known_generation) <= cut &&
          (g.withdrawn_generation === null || integer(g.withdrawn_generation) > cut));
        need(versions.length === 1, "形态在对应cut缺少唯一完整组事实");
        const group = versions[0], payload = group.payload;
        need(ref.merged_index === payload.group_index && equal(ref.raw_refs.map(r => r.source_coord), payload.members), "形态序号/真实成员与该cut组映射不符");
        const dependencies = new Set([...(payload.direction_evidence?.source_coords || []), ...(payload.confirmation_evidence?.source_coords || [])]);
        payload.members.forEach(c => dependencies.delete(c));
        need(equal(ref.dependency_refs.map(r => r.source_coord), [...dependencies].sort((a,b) => BigInt(a)<BigInt(b)?-1:BigInt(a)>BigInt(b)?1:0)), "形态构造/边界依赖与该cut组事实不符");
        const groupRefs = new Map(group.input_refs.map(r => [r.source_coord, r]));
        for (const source of [...ref.raw_refs, ...ref.dependency_refs]) need(equal(source, groupRefs.get(source.source_coord)), "形态与组事实的原始修订不同");
        selected.push(payload);
      });
      bound.set(shape.object_id, selected);
    }
    return bound;
  }
  // 仅内部内容ID恢复旧封存的三种整数，公共JSON解析仍拒绝所有number。
  function canonicalShapeIdentity(value) {
    if (typeof value === "bigint") return value.toString();
    if (Array.isArray(value)) return "[" + value.map(canonicalShapeIdentity).join(",") + "]";
    if (object(value)) return "{" + Object.keys(value).sort(utf8Compare).map(k => JSON.stringify(k) + ":" + canonicalShapeIdentity(value[k])).join(",") + "}";
    return canonical(value);
  }
  async function validateTB02Candidate(state, fixed, check) {
    const rule = baseRule(fixed.rule_revision);
    need(await sha256(validateTB02Catalog(state.catalog)) === TB02_CATALOG.public_static_sha256, "完整冻结目录静态列摘要不符"); check();
    if (fixed.cut_generation !== "0") {
      keys(fixed.catalog_evidence.axes, RULE_AXES.get(rule), "cut规则目录轴");
    }
    const bound = validateTB02CutSources(state.snapshot, fixed.rule_revision);
    const active = new Map(state.snapshot.objects.map(r => [r.object_id, r]));
    for (const r of active.values()) {
      if (r.kind !== "CC-013.segment" || r.payload.schema_revision !== "s-segment/2") continue;
      const d = r.payload.data;
      need(d.second_sequence_id !== null || d.fact_vector.scan_complete, "未完成扫描缺SECOND资源证据");
      for (const [prefix, role] of [["sequence", "FIRST_SEQUENCE"], ["second_sequence", "SECOND_SEQUENCE"]]) {
        const oid = d[prefix + "_object_id"]; if (oid === null) continue;
        const q = active.get(oid);
        need(q && q.kind === "CC-012.feature_sequence" && q.payload.schema_revision === "s-segment/2", "段引用缺同cut序列对象");
        const v = q.payload.data;
        need(v.entity_id === d[prefix + "_id"] && v.segment_id === d.entity_id && v.role === role, "段序列引用身份/角色不符");
        need(q.source_coords.every(c => r.source_coords.includes(c)), "段缺序列实际来源");
        if (role === "FIRST_SEQUENCE" && d.strict !== null) need(v.facts.checks.some(c => equal(c.elements, d.strict.elements)), "strict未绑定FIRST检查");
        if (role === "SECOND_SEQUENCE") need(d.fact_vector.scan_complete !== (v.facts.resource_truncated && v.facts.matched === null), "段事实可知性未绑定SECOND扫描证据");
        if (role === "SECOND_SEQUENCE" && d.result === "CASE_TWO") need(v.second_sequence_status === "CONFIRMED" && equal(d.fact_vector.termination_witness.second_match, v.facts.matched), "第二种终结证书未绑定同次SECOND命中");
      }
    }
    for (const record of [...state.snapshot.objects, ...state.snapshot.withdrawn_objects]) {
      await validateTB02Identity(record, fixed, bound); check();
    }
  }
  async function validateTB02Identity(record, fixed, bound = new Map()) {
    const rule = baseRule(fixed.rule_revision);
    let digestValue;
    if (TB02_KINDS.includes(record.kind)) {
      const wPolicy = fixed.rule_revision !== rule;
      if (INITIALIZATION_KINDS.includes(record.kind)) need(wPolicy, "旧规则cut混入W对象");
      if ([TB02_KINDS[1], TB02_KINDS[3]].includes(record.kind)) need(wPolicy === Object.hasOwn(record.payload, "initialization_policy_revision"), "初始化字段与cut规则不符");
      // #1405：会话级 rule_revision（经精确白名单映射的基规则）绑定笔/段/知识域合同代际；W 规则 cut 的知识域取 W 冻结源。
      const [biSchema, segmentSchema, contractDomain] = RULE_CONTRACTS[rule], domain = wPolicy ? W_KNOWLEDGE_DOMAIN : contractDomain;
      if (SEGMENT_KINDS.includes(record.kind)) need(record.payload.schema_revision === segmentSchema, "段合同与cut规则版本不符");
      if (BI_KINDS.includes(record.kind)) need(record.payload.schema_revision === biSchema, "新笔合同与cut规则版本不符");
      if (record.kind === "CC-054.knowledge_state" && domain !== null) need(record.payload.domain === domain, "TB-02-A 知识域与cut规则版本不符");
      const identity = Object.fromEntries(["kind", "fact_key", "payload", "input_refs"].map(k => [k, record[k]]));
      identity.profile_id = fixed.profile_id; identity.rule_revision = fixed.rule_revision;
      digestValue = await sha256(identity);
    } else {
      const groupFacts = bound.get(record.object_id), identity = Object.fromEntries(["branch", "dir_ab", "dir_bc", "window_start", "window_mid", "window_end", "comparisons"].map(k => [k, record[k]]));
      identity.merged_highs = groupFacts.map(g => g.high); identity.merged_lows = groupFacts.map(g => g.low);
      identity.profile_id = fixed.profile_id; identity.rule_revision = fixed.rule_revision;
      identity.input_refs = record.input_refs.map(g => ({merged_index:integer(g.merged_index),
        ...Object.fromEntries(["raw_refs", "dependency_refs"].map(k => [k, g[k].map(r => ({...r, seq:integer(r.seq), revision:integer(r.revision, true)}))]))}));
      digestValue = await sha256Text(canonicalShapeIdentity(identity));
    }
    need(record.object_id === "obj-" + digestValue, "对象内容身份 SHA 不符");
  }
  function validateTB02Axes(axes, records = null) {
    keys(axes, Object.hasOwn(axes, "CC-014") ? TB02_AXES : Object.hasOwn(axes, "CC-011") ? FIRST_SEGMENT_AXES : Object.hasOwn(axes, "CC-008") ? BI_AXES : LEGACY_TB02_AXES, "逐 cut 类型目录");
    const ids = records === null ? null : new Set(records.map(r => r.object_id));
    for (const [cid, axis] of Object.entries(axes)) {
      keys(axis, ["impl_status", "proof_status", "run_status", "evidence"], "目录轴");
      need(["implemented", "not_implemented"].includes(axis.impl_status) && axis.proof_status === "not_proved" && ["run", "waiting", "not_run"].includes(axis.run_status), "目录轴名分不符");
      const scoped = axis.evidence.scope === "TB-02-D";
      keys(axis.evidence, [...(cid === "CC-056" ? ["scope", "object_ids", "waiting_reasons", "raw_revisions", "withdrawals", "replaces"] : ["scope", "object_ids", "waiting_reasons"]), ...(scoped ? ["delivered_subobligations", "deferred_subobligations"] : [])], "目录轴证据");
      if (scoped) {
        need(equal(axis.evidence.delivered_subobligations, ["finite_complete_prefix_strict_second", "independent_stroke_and_segment_break_facts"]) && equal(axis.evidence.deferred_subobligations, ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"]), "第二种子义务范围不符");
        need(!["CC-012", "CC-013", "CC-014"].includes(cid) || axis.impl_status === "not_implemented", "有限交付冒称全轴完成");
      }
      if (cid === "CC-056") {
        for (const name of ["raw_revisions", "withdrawals", "replaces"]) need(Array.isArray(axis.evidence[name]), "修订轴缺完整原件");
        need(axis.run_status === (["raw_revisions", "withdrawals", "replaces"].some(k => axis.evidence[k].length > 0) ? "run" : "not_run"), "修订轴状态与事实不符");
      } need(axis.evidence.scope === (Object.hasOwn(axes, "CC-014") && ["CC-011", "CC-012", "CC-013", "CC-014"].includes(cid) ? "TB-02-D" : ["CC-011", "CC-012", "CC-013"].includes(cid) ? "TB-02-C" : "TB-02-A"), "目录轴范围不符");
      texts(axis.evidence.object_ids); texts(axis.evidence.waiting_reasons);
      need(new Set(axis.evidence.object_ids).size === axis.evidence.object_ids.length && (!ids || axis.evidence.object_ids.every(id => ids.has(id))), "目录引用缺失/重复");
    }
    if (Object.hasOwn(axes, "CC-014")) {
      need(equal(axes["CC-014"].evidence.object_ids, axes["CC-013"].evidence.object_ids), "CC014未绑定CC013对象");
      if (records !== null) { const byId = new Map(records.map(r => [r.object_id, r])); need(axes["CC-014"].evidence.object_ids.every(id => byId.get(id)?.kind === "CC-013.segment"), "CC014引用非段对象"); }
    }
  }
  function validateTB02Raw(record) {
    need(record.schema_revision === "s-ohlc/1" && !Object.hasOwn(record, "price"), "OHLC schema/兼容价格泄漏");
    const prices = Object.fromEntries(["open", "high", "low", "close"].map(k => [k, signed(record[k])]));
    need(prices.low <= prices.open && prices.open <= prices.high && prices.low <= prices.close && prices.close <= prices.high, "原始 OHLC 几何不符");
    signed(record.ts); need(signed(record.volume) >= 0n, "OHLC volume 为负");
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
  function parseDiscovery(value) {
    need(object(value) && value.ok === true, "发现回执未成功");
    if (value.protocol_revision === null) {
      keys(value, ["ok", "protocol_revision", "producer_epoch"], "legacy发现");
      integer(value.producer_epoch, true);
      return clone(value);
    }
    keys(value, ["ok", "protocol_revision", "cut", "snapshot", "producer_epoch"], "v2发现");
    need(value.protocol_revision === "s-session/2", "发现协议不支持");
    keys(value.cut, ["session_id", "session_generation"], "发现会话");
    keys(value.snapshot, ["profile_id"], "发现profile");
    text(value.cut.session_id); integer(value.cut.session_generation, true); integer(value.producer_epoch, true);
    need(["testonly_tick_1_1_ohlc", "ohlc_integer_tb02a_v1"].includes(value.snapshot.profile_id), "发现profile不支持");
    return clone(value);
  }
  async function readDiscovery(fetcher) {
    // 与后续load共享绝对期限及已耗字节；不为身份发现另开可叠加预算。
    const budget = {deadline: performance.now() + DEFAULTS.timeoutMs, bytes: 0};
    const abort = new AbortController(), timer = setTimeout(() => abort.abort(), DEFAULTS.timeoutMs);
    let reader;
    try {
      const response = await fetcher("/api/discover", {signal: abort.signal, cache: "no-store"});
      need(response.ok, "发现请求不是成功 HTTP 响应");
      need(response.body && typeof response.body.getReader === "function", "缺少发现响应字节流");
      reader = response.body.getReader(); const chunks = [];
      while (true) {
        need(performance.now() < budget.deadline, "发现超过本次读取总期限");
        const {done, value} = await reader.read();
        need(performance.now() < budget.deadline, "发现超过本次读取总期限");
        if (done) break;
        budget.bytes += value.byteLength;
        need(budget.bytes <= DEFAULTS.replyBytes && budget.bytes <= DEFAULTS.totalBytes, "发现响应达到字节界限");
        chunks.push(value);
      }
      const bytes = new Uint8Array(budget.bytes); let offset = 0;
      for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
      return {discovery: parseDiscovery(parse(new TextDecoder("utf-8", {fatal: true}).decode(bytes))), budget};
    } catch (error) { if (reader) await reader.cancel().catch(() => {}); throw error; }
    finally { if (reader) reader.releaseLock(); clearTimeout(timer); }
  }
  function orderFor(profileId) { return profileId === "ohlc_integer_tb02a_v1" ? "s-record-order/2" : ORDER; }
  function cursorOf(fixed) {
    return {session_id: fixed.session_id, session_generation: fixed.session_generation, catalog_revision: fixed.catalog_revision,
      scope: clone(fixed.scope), after_generation: fixed.cut_generation, base_cut: fixed.structure_cut,
      index_frontier: fixed.index_frontier, last_seq: fixed.input_frontier, order_version: orderFor(fixed.profile_id)};
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
      case "objects": return Object.hasOwn(r, "fact_key") ? [1n, s("kind"), s("fact_key"), s("object_id"), n("object_revision")] : [0n, n("window_start"), n("window_mid"), n("window_end"), s("object_id"), n("object_revision")];
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
    constructor({fetch: fetcher, hooks, onCommit = () => {}, limits = {}}) {
      const useDefaultFetch = fetcher === undefined;
      if (useDefaultFetch) fetcher = globalThis.fetch;
      need(typeof fetcher === "function", "浏览器缺少网络读取能力");
      for (const name of ["validateState", "validateDelta", "comparePublication"]) need(typeof hooks?.[name] === "function", "缺少严格校验 " + name);
      this.fetch = useDefaultFetch ? fetcher.bind(globalThis) : fetcher;
      this.hooks = hooks; this.onCommit = onCommit; this.limits = {...DEFAULTS, ...limits};
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
      need(token.token_schema === TOKEN_SCHEMA && token.order_version === orderFor(fixed.profile_id) && token.page_size === String(this.limits.pageSize) && token.offset === String(offset), "分页 token 页序/资源/schema 不符");
      integer(token.offset); integer(token.page_size, true); digest(token.issued_capture_digest); digest(token.token_checksum);
      for (const key of ["session_id", "session_generation", "cut_generation", "scope", "history_mode", "structure_cut", "catalog_revision", "index_frontier", "profile_hash", "rule_revision"])
        need(equal(token[key], fixed[key]), "分页 token 固定身份不符：" + key);
      need(token.cut_projection_digest === projectionDigest, "分页 token 投影摘要变化");
      const unsigned = {...token}; delete unsigned.token_checksum;
      need(await sha256(unsigned) === token.token_checksum, "分页 token checksum 不符");
    }
    async collect({identity, mode, generation = null, token = null, expectedFixed = null, expectedProfile = null}, budget) {
      const expectedOrder = expectedFixed ? orderFor(expectedFixed.profile_id) : (identity.order_version || this.requestOrder || ORDER);
      const payload = token ? {op: "snapshot", snapshot_token: token} : {op: "snapshot", ...identityOf(identity), scope: SCOPE,
        history_mode: mode, as_of_generation: generation, page_size: String(this.limits.pageSize), order_version: expectedOrder};
      if (generation !== null) integer(generation);
      if (token) { validateFixed(expectedFixed); await this.validateToken(token, expectedFixed, token.cut_projection_digest, 0); }
      const rows = [], seen = new Set(), objectIds = new Set(); let base = null, baseCanonical = null, next = payload, issued = token?.issued_capture_digest;
      for (let pageNumber = 0; ; pageNumber++) {
        need(pageNumber < this.limits.maxPages, "分页数达到浏览器界限，未验证");
        const envelope = await this.request(identity, next, budget), page = envelope.payload;
        keys(page, PAGE, "Snapshot 页"); validateFixed(page.fixed_cut);
        need(sameIdentity(envelope, identity) && sameIdentity(page.fixed_cut, identity), "Snapshot 响应会话/化身不符");
        need(page.fixed_cut.history_mode === mode && (generation === null || page.fixed_cut.cut_generation === generation), "Snapshot 不属于请求的模式/cut");
        if (expectedProfile !== null) need(page.fixed_cut.profile_id === expectedProfile, "Snapshot profile与发现不符");
        if (expectedFixed) need(equal(page.fixed_cut, expectedFixed), "重建页不属于 Gap 声明的固定切面");
        need(equal(page.scope, SCOPE) && equal(page.omissions, []) && page.order_version === expectedOrder && page.order_version === orderFor(page.fixed_cut.profile_id), "Snapshot scope/omissions/order 不符");
        digest(page.cut_projection_digest); digest(page.validated_capture_digest);
        keys(page.counts, [...FAMILIES, "total"], "六族完整计数");
        for (const count of Object.values(page.counts)) integer(count);
        need(FAMILIES.reduce((n, k) => n + BigInt(page.counts[k]), 0n) === BigInt(page.counts.total) &&
          BigInt(page.counts.total) <= BigInt(this.limits.maxRows), "完整计数不符或达到浏览器行数界限");
        need(page.offset === String(rows.length) && typeof page.done === "boolean" && Array.isArray(page.rows), "分页 offset/done/rows 无效");
        const metadata = Object.fromEntries(["fixed_cut", "catalog", "counts", "scope", "omissions", "order_version", "cut_projection_digest"].map(k => [k, page[k]]));
        if (base) {
          // #1414：base是本次collect私有clone，分页内不写；仍逐页完整规范化当前metadata。
          baseCanonical ??= canonical(base);
          need(baseCanonical === canonical(metadata), "跨页固定切面/目录/计数/摘要变化");
        } else { base = clone(metadata); issued ||= page.validated_capture_digest; }
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
      if (base.fixed_cut.profile_id === "ohlc_integer_tb02a_v1") {
        await validateTB02Candidate(state, base.fixed_cut, () => this.check(budget));
      }
      return {state, projection, cursor: cursorOf(base.fixed_cut), pages: Math.max(1, Math.ceil(rows.length / this.limits.pageSize))};
    }
    #commit(candidate, budget, event) {
      this.check(budget); this.#committed = candidate; this.onCommit(candidate, event); return candidate;
    }
    async load(discovery, mode = "RecomputedWithRevision", generation = null, discoveryBudget = null) {
      const budget = this.begin(); // 无效的新请求也使旧响应失效，不能事后覆盖错误状态。
      if (discoveryBudget !== null) {
        need(object(discoveryBudget) && Number.isFinite(discoveryBudget.deadline) && Number.isSafeInteger(discoveryBudget.bytes) && discoveryBudget.bytes >= 0, "发现预算无效");
        budget.deadline = Math.min(budget.deadline, discoveryBudget.deadline); budget.bytes = discoveryBudget.bytes;
        need(budget.bytes <= this.limits.totalBytes, "发现已达到总字节界限"); this.check(budget);
      }
      discovery = parseDiscovery(discovery);
      need(discovery.protocol_revision === "s-session/2", "v2消费者拒绝legacy发现");
      const identity = identityOf(discovery.cut); text(identity.session_id); integer(identity.session_generation, true);
      this.requestOrder = orderFor(discovery.snapshot.profile_id);
      need(mode === "AsKnown" || mode === "RecomputedWithRevision", "历史模式无效");
      if (mode === "AsKnown") integer(generation);
      const candidate = await this.collect({identity, mode, generation, expectedProfile: discovery.snapshot.profile_id}, budget);
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
  return Object.freeze({TB02_KINDS: Object.freeze(TB02_KINDS), validateSegmentPayload, validateBiPayload, Client, DEFAULTS, FAMILIES, ORDER, orderFor, parseDiscovery, readDiscovery, validateTyped, validateTB02ShapeRefs, validateTB02Axes, validateTB02Raw, validateTB02Catalog, validateTB02CutSources, validateTB02Candidate, validateTB02Identity, TB02_CATALOG, canonical, parse, sha256, rowsOf, cursorOf});
});
