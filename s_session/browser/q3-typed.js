/* #1394：typed 与形态来源的同一判据；只把访问迁为完整异步 Value 遍历。 */
(function (root, factory) {
  const api = factory(typeof module === "object" && module.exports ? require("./q3-values.js") : root.Q3Values);
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.Q3Typed = api;
})(globalThis, function (V) {
  "use strict";
  // #1405：旧合同 AsKnown 保留，代际由会话规则绑定，不能混用。
  const KNOWLEDGE_DOMAINS = ["established_direction_without_extreme_identity_competition", "established_direction_with_extreme_root_sets"];
  const BI_VERSIONS = {
    "s-new-bi/1": ["new-bi-dual-coordinate/1", "standard_raw_gap_3"],
    "s-new-bi/2": ["new-bi-dual-coordinate/2", "standard_raw_gap_3;same-price/keep-earlier+extreme-region/1"],
  };
  const RULE_CONTRACTS = {
    "s2-axis-quantifiers": [null, null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1": ["s-new-bi/1", null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-first/1": ["s-new-bi/1", "s-segment/1", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-second/1": ["s-new-bi/1", "s-segment/2", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/2": ["s-new-bi/2", null, KNOWLEDGE_DOMAINS[1]],
    "s2-axis-quantifiers+new-bi/2+segment-second/1": ["s-new-bi/2", "s-segment/2", KNOWLEDGE_DOMAINS[1]],
    "s2-axis-quantifiers+new-bi/2+segment-second/2": ["s-new-bi/2", "s-segment/3", KNOWLEDGE_DOMAINS[1]],
    // #1493（xianduan.md §7.1 第 7 条）：§7.1 写的 s-segment/3 已被 #1420 占用，本代段合同用 s-segment/4。
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1": ["s-new-bi/2", "s-segment/4", KNOWLEDGE_DOMAINS[1]], "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2": ["s-new-bi/2", "s-segment/5", KNOWLEDGE_DOMAINS[1]], // #1604：预算 token 必填值 v1→v2 ⟹ 规则加 token、段合同升 /5（形状同 /4）
  };
  // #1491 ADR 0037 §五/§六：rule_revision 的布局组件与教义组件正交。轴表/合同只看去掉布局组件后的
  // 教义串；串内无 `bounded-layout` 组件即 layout/1（定义，不是兜底）；未知或重复布局组件一律拒收。
  const LAYOUT_COMPONENT = "bounded-layout/", LAYOUTS = Object.freeze({LAYOUT1: "layout/1", BOUNDED1: "bounded-layout/1"});
  function splitRule(rule) {
    need(typeof rule === "string", "cut规则修订未声明");
    let layout = null; const rest = [];
    for (const component of rule.split("+")) {
      if (!component.startsWith(LAYOUT_COMPONENT)) { rest.push(component); continue; }
      need(component === LAYOUTS.BOUNDED1, "cut规则修订未声明：未知布局组件 " + component);
      need(layout === null, "cut规则修订未声明：多个布局组件");
      layout = LAYOUTS.BOUNDED1;
    }
    return {doctrine: rest.join("+"), layout: layout || LAYOUTS.LAYOUT1};
  }
  const ruleContract = rule => { const {doctrine} = splitRule(rule); return Object.hasOwn(RULE_CONTRACTS, doctrine) ? RULE_CONTRACTS[doctrine] : null; };
  const DEVELOPMENT_BATCH_KIND = "CC-007.development_batch";
  const SAME_PRICE_KEEP_EVIDENCE = "same_kind_equal_price_keep_earlier";
  const RETIRED_SAME_PRICE_WAITING = ["raw_extreme_identity_tie", "actual_extreme_mapping_missing", "same_kind_endpoint_identity_tie", "earlier_endpoint_identity_unsettled"];
  const TB02_KINDS = ["CC-004.inclusion_step", "CC-005.inclusion_group", "CC-007.fractal_description", "CC-054.knowledge_state"];
  const BI_KINDS = ["CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version"];
  TB02_KINDS.push(...BI_KINDS);
  const SEGMENT_KINDS = ["CC-011.segment_seed", "CC-012.feature_sequence", "CC-013.segment", "CC-055.segment_relation", "CC-055.segment_change", "CC-056.segment_version", "CC-011.seed_candidate"];
  TB02_KINDS.push(...SEGMENT_KINDS);
  TB02_KINDS.push(DEVELOPMENT_BATCH_KIND);
  const REF_KEYS = ["identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord"];
  const {need, keys, T, S, I, N, equal} = V;
  const field = (v, k) => V.field(v, k);
  const isNull = v => V.of(v).kind === "null";
  const eqField = async (v, k, expected) => equal(await field(v, k), expected);
  async function enumValue(v, values, message) {
    need((V.of(v).kind) === "string", message);
    const value = await S(v); need(values.includes(value), message); return value;
  }
  async function array(v, message) { v = V.of(v); need((v.kind) === "array", message); return v; }
  async function texts(v) {
    v = await array(v, "要求文本数组"); for await (const item of v.items()) await T(item); return v;
  }
  async function coordinates(v) {
    v = await array(v, "要求完整坐标数组"); let last = -1n;
    for await (const item of v.items()) { const n = await I(item); need(n > last, "来源坐标未严格递增/重复"); last = n; }
    return v;
  }
  async function coordinateSet(v) {
    const result = new Set(); for await (const item of (await coordinates(v)).items()) result.add(await S(item)); return result;
  }
  async function digest(v) { const value = await S(v); need(/^[a-f0-9]{64}$/.test(value), "SHA-256 文本不规范"); }
  async function barRef(v) {
    await keys(v, ["source_coord", "open", "high", "low", "close"], "BarRef"); await I(await field(v, "source_coord"));
    for (const k of ["open", "high", "low", "close"]) await N(await field(v, k));
  }
  async function direction(v) { if (!(isNull(v))) await enumValue(v, ["UP", "DOWN"], "未知方向枚举"); }
  async function directionEvidence(v) {
    if (isNull(v)) return;
    await keys(v, ["direction", "established_at", "previous_acc", "incoming", "source_coords"], "方向见证");
    await enumValue(await field(v, "direction"), ["UP", "DOWN"], "方向见证缺方向"); await I(await field(v, "established_at"));
    await barRef(await field(v, "previous_acc")); const incoming = await field(v, "incoming"); await barRef(incoming);
    await coordinates(await field(v, "source_coords"));
    need(await equal(await field(v, "established_at"), await field(incoming, "source_coord")), "方向建立坐标与原 incoming 不同");
  }
  async function reference(v, name, revisionMessage) {
    await keys(v, REF_KEYS, name); for await (const entry of V.of(v).entries()) await T(entry.value);
    await digest(await field(v, "payload_hash")); await I(await field(v, "input_revision"), true); await I(await field(v, "revision"), true);
    await I(await field(v, "seq")); await I(await field(v, "source_coord"));
    need(await equal(await field(v, "input_revision"), await field(v, "revision")), revisionMessage);
  }
  // 比较规范文本槽时逐片消费，避免把无限长窗口或 payload 物化。
  async function canonicalMatches(v, expected) {
    let offset = 0;
    for await (const chunk of V.canonicalChunks(V.of(v))) {
      if (expected.slice(offset, offset + chunk.length) !== chunk) return false;
      offset += chunk.length;
    }
    return offset === expected.length;
  }
  // `layout` 来自本 cut 的 rule_revision（splitRule）；未给出 ⟹ 调用方读的是 layout/1 库。
  async function validateTyped(record, layout = LAYOUTS.LAYOUT1) {
    need(Object.values(LAYOUTS).includes(layout), "未知内容布局");
    const bounded = layout === LAYOUTS.BOUNDED1;
    record = V.of(record);
    await keys(record, ["object_id", "object_revision", "kind", "batch_id", "fact_key", "payload", "input_refs", "source_coords", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by", "lifecycle"], "typed 对象");
    for (const k of ["object_id", "kind", "batch_id", "fact_key", "first_known_cut"]) await T(await field(record, k));
    const kind = await S(await field(record, "kind"));
    need(TB02_KINDS.includes(kind) && await eqField(record, "object_revision", "1"), "typed kind/revision 不符");
    need(kind !== DEVELOPMENT_BATCH_KIND || bounded, "layout/1 库不含单元批记录");
    const first = await I(await field(record, "first_known_generation"), true); await I(await field(record, "published_generation"), true);
    need(await eqField(record, "first_known_cut", "cut-" + await S(await field(record, "first_known_generation"))) && await equal(await field(record, "published_generation"), await field(record, "first_known_generation")), "typed 首次获知不闭合");
    const withdrawn = await field(record, "withdrawn_generation"), reason = await field(record, "withdrawal_reason"), superseded = await field(record, "superseded_by");
    if (isNull(withdrawn)) need(await eqField(record, "lifecycle", "active") && isNull(reason) && isNull(superseded), "活动对象携带撤回字段");
    else {
      need(await I(withdrawn, true) > first && await eqField(record, "lifecycle", "withdrawn"), "撤回代际不符");
      const why = await enumValue(reason, ["fact_removed", "superseded_by_revision"], "撤回原因/目标不符");
      need((why === "fact_removed") === isNull(superseded), "撤回原因/目标不符"); if (!(isNull(superseded))) await T(superseded);
    }
    const refs = await array(await field(record, "input_refs"), "typed input_refs 不是数组"), coords = await coordinates(await field(record, "source_coords"));
    const coordIterator = coords.items()[Symbol.asyncIterator]();
    for await (const ref of refs.items()) {
      await reference(ref, "typed input_ref", "输入修订引用不同"); const next = await coordIterator.next();
      need(!next.done && await equal(next.value, await field(ref, "source_coord")), "typed 源引用/坐标不一一对应");
    }
    need((await coordIterator.next()).done, "typed 源引用/坐标不一一对应");
    const p = await field(record, "payload");
    if (SEGMENT_KINDS.includes(kind)) validateSegmentPayload(kind, await plain(p));
    else if (BI_KINDS.includes(kind)) await validateBiPayload(kind, p);
    else if (kind === TB02_KINDS[0]) {
      await keys(p, ["action", "incoming", "acc_before", "acc_after", "direction", "direction_evidence", "comparisons", "endpoint_order", "contains", "high_sources", "low_sources", "waiting_reasons"], "包含步骤");
      await enumValue(await field(p, "action"), ["seed", "establish_direction", "merge", "new_group", "waiting"], "步骤动作不符"); await barRef(await field(p, "incoming"));
      for (const k of ["acc_before", "acc_after"]) { const value = await field(p, k); if (!(isNull(value))) await barRef(value); }
      await direction(await field(p, "direction")); await directionEvidence(await field(p, "direction_evidence")); await texts(await field(p, "waiting_reasons"));
      await coordinates(await field(p, "high_sources")); await coordinates(await field(p, "low_sources"));
      const comparisons = await array(await field(p, "comparisons"), "包含比较两轴不完整");
      need([0n, 2n].includes(await comparisons.length()), "包含比较两轴不完整"); let index = 0;
      for await (const c of comparisons.items()) {
        await keys(c, ["axis", "acc", "incoming", "order"], "比较");
        need(await eqField(c, "axis", ["high", "low"][index++]), "比较轴/枚举不符"); await enumValue(await field(c, "order"), ["LT", "EQ", "GT"], "比较轴/枚举不符");
        await N(await field(c, "acc")); await N(await field(c, "incoming"));
      }
      const endpoints = await array(await field(p, "endpoint_order"), "端点等价类不是数组"), labels = new Set(); let count = 0n, valid = true;
      for await (const cls of endpoints.items()) {
        await texts(cls); need(await cls.length() > 0n, "端点空类");
        for await (const label of cls.items()) { const value = await S(label); count++; labels.add(value); valid = valid && ["acc.high", "acc.low", "incoming.high", "incoming.low"].includes(value); }
      }
      need(count === 0n || (count === 4n && labels.size === 4 && valid), "端点标签缺失/重复");
      const contains = await field(p, "contains"); need(["null", "bool"].includes(contains.kind), "contains 不是 bool/null");
      if (isNull(await field(p, "acc_before"))) need(count === 0n && await comparisons.length() === 0n && isNull(contains), "seed 虚构比较");
    } else if (kind === TB02_KINDS[1]) {
      await keys(p, ["group_index", "group_anchor", "members", "open", "high", "low", "close", "high_sources", "low_sources", "confirmed", "direction", "direction_evidence", "confirmation_evidence", "waiting_reasons"], "包含组");
      await I(await field(p, "group_index")); await I(await field(p, "group_anchor"));
      const members = await coordinates(await field(p, "members")), memberSet = await coordinateSet(members), firstMember = await members.items()[Symbol.asyncIterator]().next();
      need(!firstMember.done && await equal(firstMember.value, await field(p, "group_anchor")), "组首未绑定首成员");
      for (const k of ["open", "high", "low", "close"]) await N(await field(p, k));
      for (const k of ["high_sources", "low_sources"]) {
        const sources = await coordinates(await field(p, k)); let present = true;
        for await (const source of sources.items()) present = present && memberSet.has(await S(source));
        need(await sources.length() > 0n && present, "极值来源缺失/不属于成员");
      }
      const confirmed = await field(p, "confirmed"), evidence = await field(p, "confirmation_evidence");
      need(confirmed.kind === "bool" && await equal(confirmed, !(isNull(evidence))), "组确认字段/见证不符");
      await directionEvidence(evidence); await direction(await field(p, "direction")); await directionEvidence(await field(p, "direction_evidence")); await texts(await field(p, "waiting_reasons"));
    } else if (kind === TB02_KINDS[2]) {
      // bounded-layout/1：分型对象不含后续发展（进单元批记录）；layout/1 原件照旧逐项核。
      await keys(p, ["shape_object_id", "branch", "window", "description", "reference_prices", "descriptive_labels", ...(bounded ? [] : ["subsequent_development"])], "分型描述");
      await T(await field(p, "shape_object_id")); await T(await field(p, "description"));
      await enumValue(await field(p, "branch"), ["TOP", "BOTTOM"], "CC007 不是已成立顶/底");
      const window = await coordinates(await field(p, "window")); need(await window.length() === 3n, "CC007 不是已成立顶/底"); const windows = await V.list(window);
      const names = ["left.high", "left.low", "mid.high", "mid.low", "right.high", "right.low"], pricesValue = await array(await field(p, "reference_prices"), "六个参考价不完整");
      need(await pricesValue.length() === 6n, "六个参考价不完整"); const prices = []; for await (const price of pricesValue.items()) prices.push(price);
      for (let i = 0; i < 6; i++) {
        const r = prices[i]; await keys(r, ["name", "source_coord", "axis", "value"], "参考价");
        need(await eqField(r, "name", names[i]) && await eqField(r, "axis", names[i].split(".")[1]) && await equal(await field(r, "source_coord"), windows[Math.floor(i / 2)]), "参考价来源不符"); await N(await field(r, "value"));
      }
      const labels = await field(p, "descriptive_labels"); await keys(labels, ["status", "reason", "labels", "source", "raw_ohlc"], "形容词未定声明"); const bars = await field(labels, "raw_ohlc");
      need(await eqField(labels, "status", "not_determined") && await eqField(labels, "reason", "no_settled_numeric_rule") && await eqField(labels, "labels", []) && await eqField(labels, "source", "fenxing.md:64-74") && bars.kind === "array", "形容词未定边界改变");
      for await (const bar of bars.items()) await barRef(bar);
      if (bounded) { /* 后续单元由 CC-007.development_batch 承载，跨对象绑定见 bindDevelopmentBatches。 */ } else {
      const sub = await field(p, "subsequent_development"); await keys(sub, ["status", "reason", "bars"], "后续已知事实"); const subsequent = await array(await field(sub, "bars"), "后续 bars 不完整");
      need(await subsequent.length() > 0n ? await eqField(sub, "status", "observed") && await eqField(sub, "reason", null) : await eqField(sub, "status", "insufficient_knowledge") && await eqField(sub, "reason", "no_subsequent_bar"), "后续状态与原件不一致");
      for await (const entry of subsequent.items()) {
        await keys(entry, ["bar", "relations"], "后续 bar"); const bar = await field(entry, "bar"); await barRef(bar);
        const relations = await array(await field(entry, "relations"), "后续四价×六参考关系不完整"); need(await relations.length() === 24n, "后续四价×六参考关系不完整"); let i = 0;
        for await (const r of relations.items()) {
          const axis = ["open", "high", "low", "close"][Math.floor(i / 6)], ref = prices[i++ % 6];
          await keys(r, ["axis", "value", "reference", "reference_value", "order"], "后续关系");
          need(await eqField(r, "axis", axis) && await equal(await field(r, "value"), await field(bar, axis)) && await equal(await field(r, "reference"), await field(ref, "name")) && await equal(await field(r, "reference_value"), await field(ref, "value")), "后续关系来源不同");
          await enumValue(await field(r, "order"), ["LT", "EQ", "GT"], "后续关系来源不同");
        }
      }
      }
    } else if (kind === DEVELOPMENT_BATCH_KIND) {
      await keys(p, ["bar", "entries"], "单元批记录"); const bar = await field(p, "bar"); await barRef(bar);
      const entries = await array(await field(p, "entries"), "单元批记录 entries 不完整"); need(await entries.length() > 0n, "空单元批记录"); const ids = new Set();
      for await (const entry of entries.items()) {
        await keys(entry, ["fractal_object_id", "unit", "development"], "单元批记录 entry"); const id = await T(await field(entry, "fractal_object_id")); need(!ids.has(id), "单元批记录分型重复"); ids.add(id);
        const unit = await field(entry, "unit"); await keys(unit, ["bar", "relations"], "后续 bar"); need(await equal(await field(unit, "bar"), bar), "单元 bar 与批记录 bar 不同");
        const relations = await array(await field(unit, "relations"), "后续四价×六参考关系不完整"); need(await relations.length() === 24n, "后续四价×六参考关系不完整");
        const development = await field(entry, "development"); await keys(development, ["root", "count"], "单元序列"); const units = await array(await field(development, "root"), "单元序列不完整");
        const count = await I(await field(development, "count"), true); need(count === await units.length(), "单元序列计数不符");
        need(await equal(await units.at(count - 1n), unit), "单元序列末项不是本单元");
      }
    } else {
      await keys(p, ["scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests"], "知识状态");
      need(await eqField(p, "scope", "TB-02-A") && KNOWLEDGE_DOMAINS.includes(await S(await field(p, "domain"))) && await eqField(p, "known_facts", ["raw_ohlc", "inclusion_trace", "group_provenance"]), "知识域/已知事实不符");
      await I(await field(p, "input_frontier"), false, true); await texts(await field(p, "waiting_reasons")); const requests = await array(await field(p, "requests"), "逐请求集合不完整"), ids = new Set();
      const enums = {input_quality:["SUFFICIENT", "INSUFFICIENT", "INCONSISTENT"], applicability:["IN_DOMAIN", "OUTSIDE_DOMAIN", "DOMAIN_PROOF_MISSING"], computation:["COMPLETE", "INCOMPLETE"], validity:["CURRENT", "SUPERSEDED", "WITHDRAWN"]};
      for await (const r of requests.items()) {
        await keys(r, ["request_id", "subject_id", "axes", "reasons", "source_coords"], "请求状态"); const id = await T(await field(r, "request_id")); need(!ids.has(id), "请求身份重复"); ids.add(id);
        const subject = await field(r, "subject_id"); if (!(isNull(subject))) await T(subject); const axes = await field(r, "axes"); await keys(axes, Object.keys(enums), "请求四轴");
        for (const k of Object.keys(enums)) await enumValue(await field(axes, k), enums[k], "请求四轴枚举不符"); await texts(await field(r, "reasons")); await coordinates(await field(r, "source_coords"));
      }
    }
    const slot = [...BI_KINDS, ...SEGMENT_KINDS].includes(kind) ? await field(p, "slot") : kind === TB02_KINDS[0] ? await field(await field(p, "incoming"), "source_coord") : kind === TB02_KINDS[1] ? await field(p, "group_anchor") : kind === TB02_KINDS[2] ? await field(p, "window") : kind === DEVELOPMENT_BATCH_KIND ? await field(await field(p, "bar"), "source_coord") : "TB-02-A";
    need(await canonicalMatches([kind, slot], await S(await field(record, "fact_key"))), "事实语义槽未绑定 payload");
  }
  // #1404/#1414/#1420：s-segment/1、/2、/3 段合同沿用已审同步判据（单一实现，含 K1 游标）。
  // 调用面两处：导出给既有观察器/合同测试的同步入口；validateTyped 在该 payload 的完整原件上调用（见 plain）。
  // #1493：段候选态/窗成员与同 cut FIRST 序列的跨对象绑定（q3-consumer bindSegmentSequences 调用）。
  let segmentTurnLinks = null;
  const PARSE_INVALID_REASON = "segment_parse_invalid";
  // #1493 裁定 4c（X-36）：CC-013 段合同 result → 签名目录 CC-013 semantic_branches 叶（与 s_tb02_contract.RESULT_SEMANTIC_BRANCH 对称）。
  const RESULT_SEMANTIC_BRANCH = Object.freeze({NO_FIRST: "NO_FIRST", CASE_ONE: "CASE_ONE", CASE_TWO: "CASE_TWO", PENDING_CASE_TWO: "PENDING_CASE_TWO",
    FAILED_GEOMETRY: "FAILED_GEOMETRY", PENDING_CASE_ONE: "NO_FIRST", KILLED_CASE_TWO: "PENDING_CASE_TWO"});
  const validateSegmentPayload = (function () {
    const need = V.need;
    const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
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
    function signed(value) {
      need(typeof value === "string" && value.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value), "价格须为规范整数文本");
      const n = BigInt(value); need(n >= -9223372036854775808n && n <= 9223372036854775807n, "价格超出 i64"); return n;
    }
    function texts(value) { need(Array.isArray(value), "要求文本数组"); value.forEach(text); return value; }
    function coordinates(value) {
      need(Array.isArray(value), "要求完整坐标数组");
      let last = -1n; for (const v of value) { const n = integer(v); need(n > last, "来源坐标未严格递增/重复"); last = n; } return value;
    }
    function direction(value) { need(value === null || ["UP", "DOWN"].includes(value), "未知方向枚举"); }
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
    const SECOND_FOLD_CURSOR = ["schema", "rule_revision", "budget_policy", "budget_granted", "candidate", "input_stroke_count", "from_stroke", "through_stroke", "consumed_len", "consumed_digest", "dir_state", "standard", "breach_watermark", "semantic_state", "killed_by", "matched"];
    // #1420 K1 游标线形自身：形状/枚举/整数/摘要、候选起点（b+1）与封存见证非空性；同对象相等绑定由调用方另做（与 s_tb02_contract._fold_cursor 对称）。
    const budgetPolicy = v5 => v5 ? "second-seq-scan-budget/2" : "second-seq-scan-budget/1"; function foldCursor(c, element, kill, check, v5 = false) { // #1604：budgetPolicy = s-segment/5 写本次所读预算记录（签署 v2），s-segment/3、/4 为历史 token v1（与 s_tb02_contract._budget_policy 对称）
      keys(c, SECOND_FOLD_CURSOR, "K1游标");
      need(c.schema === "second-fold-cursor/1" && c.rule_revision === "second-candidate-terminal/1" && c.budget_policy === budgetPolicy(v5), "K1游标版本不符");
      const candidate = c.candidate; keys(candidate, ["seg_dir", "b_stroke", "candidate_extreme"], "K1候选身份");
      need(["UP", "DOWN"].includes(candidate.seg_dir), "K1候选方向未声明"); signed(candidate.candidate_extreme);
      const start = integer(c.from_stroke);
      need(integer(candidate.b_stroke) + 1n === start, "K1游标起点不是b笔之后");
      for (const name of ["input_stroke_count", "through_stroke", "consumed_len"]) integer(c[name]);
      if (c.budget_granted !== null) integer(c.budget_granted, true);
      need(typeof c.consumed_digest === "string" && /^[0-9a-f]{16}$/.test(c.consumed_digest), "K1游标指纹不规范");
      direction(c.dir_state); need(Array.isArray(c.standard), "K1游标标准元素错型"); c.standard.forEach(e => element(e));
      if (c.breach_watermark !== null) signed(c.breach_watermark);
      const sealed = c.semantic_state;
      // 游标只封存已决（CONFIRMED/KILLED）或未决（null）；OPEN 与覆盖只在出具结果时按输入末端判定。
      need([null, "CONFIRMED", "KILLED_BY_NEW_EXTREME"].includes(sealed), "K1游标封存了未决终态");
      need((c.killed_by !== null) === (sealed === "KILLED_BY_NEW_EXTREME") && (c.matched !== null) === (sealed === "CONFIRMED"), "K1游标见证与封存终态不符");
      if (c.killed_by !== null) kill(c.killed_by);
      if (c.matched !== null) check(c.matched);
      return start;
    }
    // CC-056 续扫检查点没有 stroke_refs：元素只验形状，笔号只验为非负整数串。#1493 v4：元素带窗成员键，SECOND/复现元素恒空。
    function checkpointElement(e, v4 = false) {
      keys(e, ["high", "low", "stroke_idx", "stroke_indices", ...(v4 ? ["window_members"] : [])], "FIRST元素"); signed(e.high); signed(e.low);
      need(Array.isArray(e.stroke_indices), "要求完整坐标数组"); e.stroke_indices.forEach(i => integer(i));
      need(e.stroke_indices.includes(e.stroke_idx), "元素代表笔不属于来源");
      if (v4) need(Array.isArray(e.window_members) && e.window_members.length === 0, "SECOND/复现元素不得带窗成员");
    }
    function checkpointKill(k) {
      keys(k, ["stroke_idx", "price", "candidate_extreme", "comparison"], "失效见证"); integer(k.stroke_idx); signed(k.price); signed(k.candidate_extreme);
      need([">", "<"].includes(k.comparison), "失效见证比较未声明");
    }
    function checkpointCheck(c, v4 = false) {
      keys(c, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND分型见证");
      need(Array.isArray(c.elements) && c.elements.length === 3, "SECOND分型窗口缺元素"); c.elements.forEach(e => checkpointElement(e, v4));
      need(typeof c.top_fractal === "boolean" && typeof c.bottom_fractal === "boolean", "SECOND分型错型"); integer(c.through_stroke);
    }
    // #1420 r2 CC-056.scan_cursors：本 cut 续扫检查点只含调度相关游标（累计预算大于每轮或本轮耗尽）；无同对象 facts 可比。
    function scanCheckpoints(cursors, roundStrokes, v4 = false, v5 = false) {
      need(Array.isArray(cursors), "续扫检查点不是数组");
      need(roundStrokes !== 0n || cursors.length === 0, "预算0却登记续扫检查点");
      const seen = new Set();
      for (const c of cursors) {
        foldCursor(c, e => checkpointElement(e, v4), checkpointKill, k => checkpointCheck(k, v4), v5);
        need(c.budget_granted !== null && BigInt(c.consumed_len) <= BigInt(c.budget_granted), "续扫检查点预算不符");
        const identity = canonical([c.candidate.seg_dir, c.candidate.b_stroke, c.candidate.candidate_extreme]);
        need(!seen.has(identity), "续扫检查点候选身份重复"); seen.add(identity);
      }
    }
    // #1493 s-segment/4（xianduan.md §7.1）：第一种情况候选三态、转折点窗与段端门 fail-closed（与 s_tb02_contract 对称）。
    const TURN_CHECK_STATES = ["CONFIRMED", "KILLED_BY_NEW_EXTREME", "NOT_CASE_ONE"], TURN_OPEN = "OPEN_AT_PREFIX_END";
    const FIRST_TURN_ACTIONS = ["boundary_pair", "window_member", "tie_pair"];
    const PARSE_INVALID_REASON = "segment_parse_invalid", PENDING_CASE_ONE_REASON = "first_kind_third_stroke_within_first", MIN_LENGTH_REASON = "minimum_segment_length_not_met";
    function turnCandidate(d, indices) {
      const state = d.turn_hypothesis_state, members = d.window_members;
      need([null, "CONFIRMED", "KILLED_BY_NEW_EXTREME", TURN_OPEN].includes(state), "段候选态未声明");
      need(Array.isArray(members), "窗成员错型"); indices(members);
      need(!(state === null && members.length) && (d.result === "PENDING_CASE_ONE") === d.waiting_reasons.includes(PENDING_CASE_ONE_REASON), "候选态/窗成员与等待原因不配套");
      need(d.result !== "PENDING_CASE_ONE" || state === TURN_OPEN && members.length > 0, "PENDING_CASE_ONE缺开窗候选或窗成员");
      need(!(d.result === "CASE_TWO" && state !== null) && !(d.result === "CASE_ONE" && ![null, "CONFIRMED"].includes(state)), "终结与第一种情况候选态不符");
      const last = d.termination_attempts.length ? d.termination_attempts[d.termination_attempts.length - 1] : null;
      need(d.result !== "FAILED_GEOMETRY" || last !== null && last.minimum_count_met === true && !equal(last.construction.vector, [true,true,true,true]), "最少笔数不足或构造完整却称几何失败");
      need(["CASE_ONE", "CASE_TWO"].includes(d.result) || d.waiting_reasons.includes(MIN_LENGTH_REASON) === (last !== null && last.minimum_count_met === false), "最少笔数等待原因未绑定末次终结尝试");
      if (d.parse_invalid_from !== null) integer(d.parse_invalid_from);
      const x = d.parse_invalid;
      if (x !== null) {
        keys(x, ["seg_start", "end_stroke", "construction_vector", "failed_conditions", "elements"], "段端门失败复现");
        need(x.seg_start === d.parse_invalid_from && integer(x.end_stroke) >= BigInt(d.parse_invalid_from), "段端门失败复现未绑定候选起点");
        need(x.construction_vector === null || Array.isArray(x.construction_vector) && x.construction_vector.length === 4 && x.construction_vector.every(v => typeof v === "boolean"), "段端门失败复现四条件错型");
        need(!equal(x.construction_vector, [true,true,true,true]), "段端门失败复现构造完整");
        texts(x.failed_conditions); need(Array.isArray(x.elements) && x.elements.length === 3, "段端门失败复现缺分型三元素"); x.elements.forEach(e => checkpointElement(e, true));
      }
    }
    // 段候选的候选态与窗成员须取自同 cut FIRST 序列的开窗候选或检查；不重判先破。
    function turnLinks(d, f) {
      const state = d.turn_hypothesis_state, members = d.window_members;
      if (d.result === "PENDING_CASE_ONE") { const o = f.open_turn; need(o !== null && state === TURN_OPEN && members.length > 0 && equal(o.elements[1].window_members, members), "PENDING_CASE_ONE未绑定FIRST开窗候选"); }
      if (state === null) return;
      const sources = [...(f.open_turn !== null ? [[f.open_turn.turn_hypothesis_state, f.open_turn.elements[1]]] : []), ...f.checks.map(c => [c.turn_hypothesis_state, c.elements[1]])];
      need(sources.some(([s, b]) => s === state && equal(b.window_members, members)), "段候选态/窗成员未绑定FIRST检查");
    }
    // #1420 s-segment/3：SECOND 语义终点、扫描覆盖、预算与 K1 游标的类型和同对象绑定；不重放折叠（与 s_tb02_contract._second_terminal 对称）。
    function secondTerminal(d, element, indices, secondCheck, v5 = false) {
      const scan = d.scan, f = d.facts, semantic = scan.semantic_state;
      need([null, "CONFIRMED", "KILLED_BY_NEW_EXTREME", "OPEN_AT_PREFIX_END"].includes(semantic) && ["COMPLETE", "INCOMPLETE_RESOURCE_BOUND"].includes(scan.coverage), "SECOND终态/覆盖未声明");
      const incomplete = scan.coverage === "INCOMPLETE_RESOURCE_BOUND";
      need(incomplete === scan.resource_truncated && (semantic === null) === incomplete, "SECOND覆盖与语义/截断不一致");
      need(d.second_sequence_status === (semantic === "CONFIRMED" ? "CONFIRMED" : semantic === "KILLED_BY_NEW_EXTREME" ? "KILLED" : "PENDING"), "SECOND状态未绑定语义终点");
      need(f.semantic_state === semantic && f.coverage === scan.coverage, "SECOND事实终态与扫描不一致");
      keys(scan.budget, ["policy", "granted"], "扫描预算");
      need(scan.budget.policy === budgetPolicy(v5), "扫描预算政策版本不符");
      if (scan.budget.granted !== null) integer(scan.budget.granted, true);
      const extreme = signed(f.candidate_extreme);
      if (f.breach_watermark !== null) signed(f.breach_watermark);
      const comparison = d.direction === "UP" ? ">" : d.direction === "DOWN" ? "<" : null;
      function kill(k) {
        keys(k, ["stroke_idx", "price", "candidate_extreme", "comparison"], "失效见证"); indices([k.stroke_idx]);
        const price = signed(k.price);
        need(comparison !== null && k.comparison === comparison && k.candidate_extreme === f.candidate_extreme && (comparison === ">" ? price > extreme : price < extreme), "失效见证未严格越过候选极值");
      }
      need((f.killed_by !== null) === (semantic === "KILLED_BY_NEW_EXTREME"), "失效见证与语义终点不符");
      if (f.killed_by !== null) { kill(f.killed_by); need(f.breach_watermark === f.killed_by.price, "越界水位未绑定失效见证"); }
      const c = scan.cursor, start = foldCursor(c, element, kill, secondCheck, v5);
      need(c.budget_granted === scan.budget.granted, "K1游标预算未绑定扫描预算");
      need(c.candidate.seg_dir === d.direction && start === BigInt(scan.from_stroke) && c.candidate.candidate_extreme === f.candidate_extreme, "K1候选身份未绑定SECOND扫描");
      need(c.input_stroke_count === scan.input_stroke_count && BigInt(c.consumed_len) <= (BigInt(c.input_stroke_count) > start ? BigInt(c.input_stroke_count) - start : 0n), "K1游标消费范围越界");
      need(c.semantic_state === null || c.semantic_state === semantic, "K1游标封存终态与扫描不同");
      need(c.killed_by === null || equal(c.killed_by, f.killed_by), "K1游标失效见证与事实不同");
      need(c.matched === null || equal(c.matched, f.matched), "K1游标命中与事实不同");
    }
    segmentTurnLinks = turnLinks;
    return function validateSegmentPayload(kind, p) {
      keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "段合同");
      // #1420：v2＝「含第二种」（s-segment/2 与 /3 共用）；v3 只另管版本、SECOND 三态终点/K1 游标、KILLED_CASE_TWO、CC-056 范围与槽。
      // #1493：v4（s-segment/4）⊃ v3，只另管版本、FIRST 动作集/作用域与 tie_with、元素窗成员、候选态与 a/b 配套、open_turn、PENDING_CASE_ONE、CC-013 段端门 fail-closed。
      const v5 = p.schema_revision === "s-segment/5", v4 = v5 || p.schema_revision === "s-segment/4", v3 = v4 || p.schema_revision === "s-segment/3", v2 = v3 || p.schema_revision === "s-segment/2";
      need(p.schema_revision === (v5 ? "s-segment/5" : v4 ? "s-segment/4" : v3 ? "s-segment/3" : v2 ? "s-segment/2" : "s-segment/1") && p.semantic_version === (v3 ? "segment-second/2" : v2 ? "segment-second/1" : "segment-seed-first/1") && p.policy_version === (v2 ? "seed_closed_strict_second" : "seed_closed_first_no_gap") && p.view_role === "main", "段规则/范围版本不符");
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
      function element(e, second = false) {
        keys(e, ["high", "low", "stroke_idx", "stroke_indices", ...(v4 ? ["window_members"] : [])], "FIRST元素"); signed(e.high); signed(e.low); indices(e.stroke_indices);
        need(e.stroke_indices.includes(e.stroke_idx), "元素代表笔不属于来源");
        if (v4) {
          // §7.1 第 3 条：窗成员是 b 之后、未并入 b 的同序列元素；SECOND 序列无窗。
          indices(e.window_members); need(!second || e.window_members.length === 0, "SECOND元素不得带窗成员");
          need(e.window_members.every(m => BigInt(m) > BigInt(e.stroke_idx) && !e.stroke_indices.includes(m)), "窗成员不在b之后或混入合并来源");
        }
      }
      if ([SEGMENT_KINDS[0], SEGMENT_KINDS[6]].includes(kind)) {
        keys(d, ["entity_id", "construction", "stroke_refs"], kind); construction(d.construction);
        need((kind === SEGMENT_KINDS[0]) === equal(d.construction.vector,[true,true,true,true]), "未满足完整条件的候选冒充seed");
      } else if (kind === SEGMENT_KINDS[1]) {
        keys(d, ["entity_id", "segment_id", "role", "direction", "member_direction", "facts", "stroke_refs", "scan", "second_sequence_status"], kind);
        const second = v2 && d.role === "SECOND_SEQUENCE";
        need((v2 ? ["FIRST_SEQUENCE", "SECOND_SEQUENCE"] : ["FIRST_SEQUENCE"]).includes(d.role) && ["UP", "DOWN"].includes(d.member_direction) && (second ? ["PENDING", "CONFIRMED", ...(v3 ? ["KILLED"] : [])] : v2 ? ["NOT_APPLICABLE", "PENDING", "CONFIRMED", ...(v3 ? ["KILLED"] : [])] : ["NOT_APPLICABLE", "WAITING_FOLLOWUP"]).includes(d.second_sequence_status), "序列角色/适用性不符"); direction(d.direction);
        keys(d.scan, ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "tail_cache_window", ...(second && v3 ? ["semantic_state", "coverage", "budget", "cursor"] : [])], "扫描范围");
        for (const [k,v] of Object.entries(d.scan)) if (["from_stroke", "through_stroke", "input_stroke_count", "tail_cache_window"].includes(k)) integer(v);
        need(typeof d.scan.resource_truncated === "boolean", "截断状态错型");
        need(refs.size > 0, "特征序列缺段笔引用");
        // 身份锚＝stroke_refs 中数值最小的笔号，即段起笔；wire 整数是十进制文本，须按数值取最小。
        segmentStart = [...refs.keys()].reduce((a, b) => BigInt(b) < BigInt(a) ? b : a);
        need(second || d.scan.from_stroke === segmentStart, "FIRST扫描起点不是段起笔");
        const f = d.facts; keys(f, ["raw_elements", "standard", "steps", "checks", ...(v4 && !second ? ["open_turn"] : []), ...(second ? ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "matched"] : []), ...(second && v3 ? ["candidate_extreme", "breach_watermark", "semantic_state", "killed_by", "coverage"] : [])], "序列事实");
        for (const k of ["raw_elements", "standard", "steps", "checks"]) need(Array.isArray(f[k]), "序列事实集合错型");
        if (second) {
          integer(f.from_stroke); integer(f.through_stroke, false, true); integer(f.input_stroke_count);
          need(typeof f.resource_truncated === "boolean" && d.scan.tail_cache_window === "0" && ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated"].every(k => f[k] === d.scan[k]) && BigInt(d.scan.from_stroke) > BigInt(segmentStart), "SECOND扫描边界不符");
        }
        [...f.raw_elements, ...f.standard].forEach(e => element(e, second));
        need(!v4 || second || f.raw_elements.every(e => e.window_members.length === 0), "原始特征元素不得带窗成员");
        // #1493 v4：FIRST 动作集加 boundary_pair／window_member／tie_pair（作用域 TURN_HYPOTHESIS），旧 turn_boundary 退出。
        const firstActions = ["append", "merge", ...(v4 ? FIRST_TURN_ACTIONS : ["turn_boundary"])];
        for (const s of f.steps) {
          keys(s, ["incoming", "before", "after", "action", "merge_direction", "scope", ...(v4 ? ["tie_with"] : [])], "包含步骤"); element(s.incoming, second); element(s.after, second); if (s.before !== null) element(s.before, second);
          const turn = FIRST_TURN_ACTIONS.includes(s.action) || s.action === "turn_boundary";
          need((second ? ["append", "merge"] : firstActions).includes(s.action) && s.scope === (second ? "SECOND_SEQUENCE" : turn ? "TURN_HYPOTHESIS" : "SAME_SEQUENCE"), "包含跨作用域/动作未声明");
          need(s.action === "merge" ? ["UP", "DOWN"].includes(s.merge_direction) : s.merge_direction === null, "方向性包含缺见证");
          if (v4) {
            need((s.tie_with !== null) === (s.action === "tie_pair"), "tie_with只属并列对");
            need(!FIRST_TURN_ACTIONS.includes(s.action) || s.before !== null, "转折点动作缺末标准元素");
            if (s.action === "tie_pair") { integer(s.tie_with); need(s.tie_with === s.before.stroke_idx, "tie_with未绑定末标准元素"); }
            if (s.action === "window_member") need(["high", "low", "stroke_idx", "stroke_indices"].every(k => equal(s.after[k], s.before[k])) && equal(s.after.window_members, [...s.before.window_members, s.incoming.stroke_idx]), "窗成员步改写了b或未记入成员");
          }
        }
        function secondCheck(c) {
          keys(c, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND分型见证");
          need(Array.isArray(c.elements) && c.elements.length === 3, "SECOND分型窗口缺元素"); c.elements.forEach(e => element(e, true));
          need(typeof c.top_fractal === "boolean" && typeof c.bottom_fractal === "boolean", "SECOND分型错型"); indices([c.through_stroke]);
        }
        if (second) {
          // v3：未决（PENDING）与失效（KILLED）都无命中；v2 只有 PENDING 无命中。
          need(v3 ? (f.matched !== null) === (d.second_sequence_status === "CONFIRMED") : (f.matched === null) === (d.second_sequence_status === "PENDING"), "SECOND命中与状态不符");
          if (f.matched !== null) { secondCheck(f.matched); need(f.checks.some(c => equal(c, f.matched)) && (f.matched.top_fractal || f.matched.bottom_fractal), "SECOND命中未绑定检查"); }
        }
        for (const c of f.checks) {
          if (second) { secondCheck(c); continue; }
          keys(c, ["elements", "first_fractal", "gap", "gap_closed_by_c", "provisional", "turn_hypothesis_pair", ...(v4 ? ["turn_hypothesis_state"] : [])], "分型见证");
          need(Array.isArray(c.elements) && c.elements.length === 3, "分型窗口缺元素"); c.elements.forEach(e => element(e));
          if (v4) {
            // §7.1 第 1/4 条：a/b 在「按适用判据成立的分型」或第一种情况候选态已出时给出；CONFIRMED 即分型成立。
            const state = c.turn_hypothesis_state;
            need(TURN_CHECK_STATES.includes(state), "第一种情况候选态未声明");
            need(!(state === "CONFIRMED" && c.first_fractal !== true) && !(state === "KILLED_BY_NEW_EXTREME" && c.first_fractal !== false) && c.provisional === false, "候选态与分型事实不配套");
          }
          need(equal(c.turn_hypothesis_pair, c.first_fractal || (v4 && c.turn_hypothesis_state !== "NOT_CASE_ONE") ? c.elements.slice(0,2).map(e => e.stroke_idx) : null), "假设转折点两侧未绑定a/b元素");
          need(typeof c.first_fractal === "boolean" && typeof c.provisional === "boolean", "分型事实错型");
          for (const k of ["gap", "gap_closed_by_c"]) need(c[k] === null || typeof c[k] === "boolean", "gap见证错型");
          need((c.first_fractal || c.gap === null) && (c.gap === true || c.gap_closed_by_c === null), "不适用条件虚填布尔");
        }
        if (v4 && !second && f.open_turn !== null) {
          const o = f.open_turn; keys(o, ["elements", "turn_hypothesis_pair", "turn_hypothesis_state", "killed_by"], "前缀末端开窗候选");
          need(Array.isArray(o.elements) && o.elements.length === 2, "开窗候选缺a/b元素"); o.elements.forEach(e => element(e));
          need([TURN_OPEN, "KILLED_BY_NEW_EXTREME"].includes(o.turn_hypothesis_state) && equal(o.turn_hypothesis_pair, o.elements.map(e => e.stroke_idx)) && equal(o.elements, f.standard.slice(-2)), "开窗候选未绑定末两标准元素");
          const k = o.killed_by; need((k === null) === (o.turn_hypothesis_state === TURN_OPEN), "开窗候选作废见证与状态不符");
          if (k !== null) {
            keys(k, ["stroke_idx", "price", "comparison"], "开窗作废见证"); indices([k.stroke_idx]);
            const b = o.elements[1], up = d.direction === "UP"; signed(k.price); const price = BigInt(k.price);
            need(k.comparison === (up ? ">" : "<") && (up ? price > BigInt(b.high) : price < BigInt(b.low)) && BigInt(k.stroke_idx) > BigInt(b.stroke_idx), "开窗作废见证未严格越过P");
          }
        }
        if (second && v3) secondTerminal(d, e => element(e, true), indices, secondCheck, v5);
      } else if (kind === SEGMENT_KINDS[2]) {
        keys(d, ["entity_id", "entity_revision", "seed_id", "sequence_id", "sequence_object_id", "construction", "result", "waiting_reasons", "termination_attempts", "geometric_end", "trigger_stroke", "formed_known_at", "terminated_known_at", "stroke_refs", ...(v2 ? ["second_sequence_id", "second_sequence_object_id", "strict", "fact_vector"] : []), ...(v4 ? ["turn_hypothesis_state", "window_members", "parse_invalid_from", "parse_invalid"] : [])], kind);
        integer(d.entity_revision, true); construction(d.construction); texts(d.waiting_reasons); known(d.formed_known_at);
        need(Array.isArray(d.termination_attempts), "终结准入见证错型");
        for (const a of d.termination_attempts) {
          keys(a, ["end_stroke", "trigger_stroke", "minimum_count_met", "construction"], "终结准入见证");
          indices([a.end_stroke]); indices([a.trigger_stroke]); need(typeof a.minimum_count_met === "boolean", "最小笔数条件错型"); construction(a.construction);
        }
        need(["NO_FIRST", "CASE_ONE", "PENDING_CASE_TWO", "FAILED_GEOMETRY", ...(v2 ? ["CASE_TWO"] : []), ...(v3 ? ["KILLED_CASE_TWO"] : []), ...(v4 ? ["PENDING_CASE_ONE"] : [])].includes(d.result), "冒称交付第二种终结");
        if (v4) { need(Object.hasOwn(RESULT_SEMANTIC_BRANCH, d.result), "段结果未归签名目录叶"); turnCandidate(d, indices); }
        if (["CASE_ONE", "CASE_TWO"].includes(d.result)) {
          known(d.terminated_known_at); const e = d.geometric_end;
          keys(e, ["stroke_index", "group_anchor", "extreme_roots", "price"], "几何段端"); indices([e.stroke_index]); indices([d.trigger_stroke]); coordinates(e.extreme_roots); integer(e.group_anchor); signed(e.price);
        } else need(d.geometric_end === null && d.trigger_stroke === null && d.terminated_known_at === null, "发展态虚填终结");
        if (v2) {
          const strict = d.strict;
          if (strict !== null) {
            keys(strict, ["elements", "raw_gap", "gap_closed_by_c", "effective_case", "closure_operator", "closure_left", "closure_right"], "strict事实");
            need(Array.isArray(strict.elements) && strict.elements.length === 3, "strict缺FIRST窗口"); strict.elements.forEach(e => element(e));
            need(typeof strict.raw_gap === "boolean" && (strict.raw_gap ? typeof strict.gap_closed_by_c === "boolean" : strict.gap_closed_by_c === null), "strict适用性不符");
            need(["CASE_ONE", "CASE_TWO"].includes(strict.effective_case) && ["<=", ">="].includes(strict.closure_operator), "strict规则未声明"); signed(strict.closure_left); signed(strict.closure_right);
            const a = strict.elements[0], c = strict.elements[2], up = strict.closure_operator === "<=";
            need(equal([strict.closure_operator, strict.closure_left, strict.closure_right], up ? ["<=", c.low, a.high] : [">=", c.high, a.low]), "strict比较未绑定FIRST元素");
          }
          const terminated = ["CASE_ONE", "CASE_TWO"].includes(d.result);
          need(!terminated || strict !== null && strict.effective_case === d.result, "终结缺适用strict对象");
          need((d.second_sequence_id === null) === (d.second_sequence_object_id === null), "SECOND身份/对象缺位");
          if (d.second_sequence_id !== null) { need(d.second_sequence_id === "second:" + d.entity_id, "SECOND身份未绑定候选"); text(d.second_sequence_object_id); }
          need(!["CASE_TWO", "PENDING_CASE_TWO", "KILLED_CASE_TWO"].includes(d.result) || d.second_sequence_id !== null, "第二种缺SECOND引用");
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
              match.elements.forEach(e => element(e, true)); indices([match.through_stroke]);
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
        keys(d, ["input_frontier", "waiting_reasons", "known_at", "delivered_scope", "deferred_scope", ...(v3 ? ["scan_budget", "scan_cursors"] : [])], kind); integer(d.input_frontier, false, true); known(d.known_at); texts(d.waiting_reasons);
        need(d.delivered_scope === (v3 ? "seed_strict_first_and_second_terminal" : v2 ? "seed_strict_first_and_finite_second" : "seed_and_no_gap_first_kind") && equal(d.deferred_scope, v3 ? ["general_equal_identity", "global_G006_bridge"] : v2 ? ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"] : ["second_kind", "general_equal_identity"]), "段交付范围不符");
        if (v3) { keys(d.scan_budget, ["policy", "round_strokes"], "段扫描预算"); need(d.scan_budget.policy === budgetPolicy(v5), "扫描预算政策版本不符"); scanCheckpoints(d.scan_cursors, integer(d.scan_budget.round_strokes), v4, v5); }
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
      const slot = kind === SEGMENT_KINDS[3] ? [d.source_id, d.relation_kind, d.target_id] : kind === SEGMENT_KINDS[4] ? [d.known_at.generation, d.entity_id, d.change] : kind === SEGMENT_KINDS[5] ? (v3 ? "TB-02-W" : v2 ? "TB-02-D" : "TB-02-C") : d.entity_id;
      need(equal(p.slot, slot), "段语义槽身份不符");
    };
  })();
  // 段合同按上面的原判据在完整 payload 原件上执行：普通值即原件；图值经同一 Reader（含其期限/累计账检查）逐字段取回。
  // 生产 v2 wire 不含 JSON number（parse 已拒绝），图值的 number 同样拒绝，与原判据 canonical 的拒绝一致。
  async function plain(v) {
    v = V.of(v);
    if (!v.reader) return v.plain;
    if (v.kind === "array") {
      const out = [];
      for await (const item of v.items()) out.push(await plain(item));
      return out;
    }
    if (v.kind === "object") {
      const out = {};
      for await (const entry of v.entries())
        Object.defineProperty(out, entry.key, {value: await plain(entry.value), enumerable: true, writable: true, configurable: true});
      return out;
    }
    need(v.kind !== "number", "v2 JSON 数值必须在源头投影为精确文本");
    return v.scalar();
  }
  async function validateBiPayload(kind, p) {
    p = V.of(p);
    await keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "新笔合同");
    const schema = await S(await field(p, "schema_revision"));
    const version = Object.hasOwn(BI_VERSIONS, schema) ? BI_VERSIONS[schema] : null;
    need(version !== null && await eqField(p, "semantic_version", version[0]) && await eqField(p, "policy_version", version[1]) && await eqField(p, "view_role", "main"), "新笔规则/政策版本不符");
    const v2 = schema === "s-new-bi/2";
    async function waiting(value) {
      const reasons = await texts(value);
      if (v2) for await (const reason of reasons.items())
        need(!RETIRED_SAME_PRICE_WAITING.includes(await S(reason)), "同价域退役的等待态在 v2 对象复现");
    }
    await I(await field(p, "object_generation"), true); const d = await field(p, "data");
    async function endpoint(e) {
      await keys(e, ["kind", "merged_index", "group_anchor", "price", "extreme_roots", ...(v2 ? ["raw_first", "raw_last"] : ["raw_position"]), "source_coords", "sealed_at", "waiting_reasons"], "新笔端点");
      await enumValue(await field(e, "kind"), ["TOP", "BOTTOM"], "端点类型不符"); await I(await field(e, "merged_index")); await I(await field(e, "group_anchor")); await N(await field(e, "price"));
      const roots = await coordinates(await field(e, "extreme_roots")), sources = await coordinateSet(await field(e, "source_coords")); await waiting(await field(e, "waiting_reasons"));
      for await (const root of roots.items()) need(sources.has(await S(root)), "实际根缺原始来源");
      if (v2) {
        const first = await I(await field(e, "raw_first")), last = await I(await field(e, "raw_last")), count = await roots.length();
        need(count > 0n && last >= first && last - first >= count - 1n, "极值根集与首末 raw 序位不闭合");
      } else { const position = await field(e, "raw_position"); if (!isNull(position)) await I(position); }
      const sealed = await field(e, "sealed_at"); if (!isNull(sealed)) await I(sealed);
    }
    async function conditions(c) {
      await keys(c, ["merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"], "新笔完整条件");
      await I(await field(c, "merged_gap")); const raw = await field(c, "raw_between_actual_extrema"); if (v2 || !isNull(raw)) await I(raw); await N(await field(c, "top_price")); await N(await field(c, "bottom_price"));
      const vector = await array(await field(c, "vector"), "条件向量缺位/错型"); need(await vector.length() === 3n, "条件向量缺位/错型");
      for await (const value of vector.items()) need(value.kind === "bool" || (!v2 && value.kind === "null"), "条件向量缺位/错型");
      await texts(await field(c, "failed_conditions")); await waiting(await field(c, "waiting_reasons"));
    }
    async function known(k) {
      await keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "新笔获知时点");
      await I(await field(k, "generation"), true); await I(await field(k, "input_frontier")); await T(await field(k, "receipt_id")); await T(await field(k, "received_at")); const ns = await field(k, "semantic_commit_ns"); if (!(isNull(ns))) await I(ns);
    }
    if (kind === BI_KINDS[0]) await endpoint(d);
    else if ([BI_KINDS[1], BI_KINDS[2]].includes(kind)) {
      await keys(d, ["old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", ...(v2 ? ["evidence"] : []), "waiting_reasons"], "端点对");
      const old = await field(d, "old"), fresh = await field(d, "new"); await endpoint(old); await endpoint(fresh);
      const oldKind = await S(await field(old, "kind")), newKind = await S(await field(fresh, "kind")); need(await eqField(d, "endpoint_kind_pair", oldKind + "/" + newKind), "端点分域标签不符");
      if (kind === BI_KINDS[1]) { need(oldKind !== newKind, "异型请求实际同型"); await conditions(await field(d, "conditions")); need(await eqField(d, "comparison", null) && await eqField(d, "selection", null) && (!v2 || await eqField(d, "evidence", [])), "异型域混入同型选择"); }
      else {
        need(oldKind === newKind && await eqField(d, "conditions", null), "同型域混入假条件或未声明取舍");
        const comparison = await enumValue(await field(d, "comparison"), ["LT", "EQ", "GT"], "同型域混入假条件或未声明取舍");
        await enumValue(await field(d, "selection"), v2 ? ["KEEP", "REPLACE"] : ["KEEP", "REPLACE", "UNDETERMINED"], "同型域混入假条件或未声明取舍");
        if (v2) {
          const stronger = comparison === (oldKind === "TOP" ? "GT" : "LT");
          need(await eqField(d, "selection", stronger ? "REPLACE" : "KEEP"), "同型取舍与价格比较不闭合");
          need(await eqField(d, "retained_anchors", [await field(stronger ? fresh : old, "group_anchor")]), "同型保留锚不符取舍");
          need(await eqField(d, "evidence", comparison === "EQ" ? [SAME_PRICE_KEEP_EVIDENCE] : []), "同价保留在先者的证据缺失或混入");
        }
      }
      await coordinates(await field(d, "retained_anchors")); if (v2) await texts(await field(d, "evidence")); await waiting(await field(d, "waiting_reasons"));
    } else if (kind === BI_KINDS[3]) {
      await keys(d, ["identity_anchor", "start", "end", "formation", "confirmation", "entity_id", "entity_revision", "state", "formed_known_at", "confirmed_known_at", "formed_evidence", "version_causes"], "新笔实体");
      await I(await field(d, "identity_anchor")); await I(await field(d, "entity_revision"), true); await T(await field(d, "entity_id")); await endpoint(await field(d, "start")); await endpoint(await field(d, "end"));
      await conditions(await field(d, "formation")); await conditions(await field(d, "formed_evidence")); await known(await field(d, "formed_known_at")); await texts(await field(d, "version_causes"));
      const confirmation = await field(d, "confirmation"), absent = isNull(confirmation);
      need(await eqField(d, "state", absent ? "FORMED_UNCONFIRMED" : "CONFIRMED"), "生命周期/确认见证不符");
      need(await eqField(d, "entity_id", "bi:" + await S(await field(p, "object_generation")) + ":" + await S(await field(d, "identity_anchor"))), "笔身份未绑定代际与形成起点");
      if (!absent) {
        await keys(confirmation, ["successor_start", "successor_end", "successor_conditions", "right_group_sealed_at", "source_coords"], "确认见证");
        await I(await field(confirmation, "successor_start")); await I(await field(confirmation, "successor_end")); await I(await field(confirmation, "right_group_sealed_at")); await coordinates(await field(confirmation, "source_coords"));
        await conditions(await field(confirmation, "successor_conditions")); await known(await field(d, "confirmed_known_at"));
      } else need(await eqField(d, "confirmed_known_at", null), "无确认见证却有确认时点");
    } else if (kind === BI_KINDS[4]) {
      await keys(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], "新笔关系"); for await (const entry of d.entries()) await T(entry.value);
      await enumValue(await field(d, "relation_kind"), ["member_of", "derived_from", "successor_of", "leaves", "retests", "returns_to", "extends", "newborn_after", "expands_with", "caused_turn_at", "confirms", "selected_from"], "未声明关系种类");
    } else if (kind === BI_KINDS[5]) {
      await keys(d, ["entity_id", "change", "before", "after", "known_at", "event_at", "causes", "object_id"], "新笔变化");
      await T(await field(d, "entity_id")); await enumValue(await field(d, "change"), ["formed", "extended", "endpoint_replaced", "confirmed", "source_or_boundary_updated", "withdrawn"], "未声明变化"); await known(await field(d, "known_at")); await N(await field(d, "event_at")); await texts(await field(d, "causes"));
      for (const k of ["before", "after"]) {
        const entity = await field(d, k); if (isNull(entity)) continue;
        const entityId = await S(await field(entity, "entity_id"));
        // before 按其原件形状保留历史代际；after 必须与本事件同代。
        const entitySchema = k === "after" ? schema : await (await field(entity, "start")).has("raw_first") ? "s-new-bi/2" : "s-new-bi/1";
        const synthetic = {schema_revision:entitySchema, semantic_version:BI_VERSIONS[entitySchema][0], policy_version:BI_VERSIONS[entitySchema][1], view_role:await field(p, "view_role")};
        synthetic.slot = await field(entity, "entity_id"); synthetic.object_generation = entityId.split(":")[1]; synthetic.data = entity;
        await validateBiPayload(BI_KINDS[3], synthetic);
      }
    } else {
      await keys(d, ["input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"], "新笔版本");
      await I(await field(d, "input_frontier"), false, true); await texts(await field(d, "version_causes")); await waiting(await field(d, "waiting_reasons")); await known(await field(d, "known_at"));
    }
    const generation = await field(p, "object_generation");
    const slot = kind === BI_KINDS[0] ? "fx:" + await S(generation) + ":" + await S(await field(d, "group_anchor")) :
      [BI_KINDS[1], BI_KINDS[2]].includes(kind) ? [generation, await field(await field(d, "old"), "group_anchor"), await field(await field(d, "new"), "group_anchor")] :
      kind === BI_KINDS[3] ? await field(d, "entity_id") :
      kind === BI_KINDS[4] ? [await field(d, "source_id"), await field(d, "relation_kind"), await field(d, "target_id")] :
      kind === BI_KINDS[5] ? [await field(await field(d, "known_at"), "generation"), await field(d, "entity_id"), await field(d, "change")] : "TB-02-B";
    need(await equal(await field(p, "slot"), slot), "新笔 slot 未绑定其具名对象身份");
  }
  async function validateTB02ShapeRefs(shape) {
    shape = V.of(shape);
    const refs = await field(shape, "input_refs"); need(await eqField(shape, "kind", "CC-006.local_shape") && refs.kind === "array" && await refs.length() === 3n, "OHLC形态缺完整三组");
    const anchors = [await field(shape, "window_start"), await field(shape, "window_mid"), await field(shape, "window_end")], all = new Set(); let previousIndex = null, index = 0;
    for await (const g of refs.items()) {
      await keys(g, ["merged_index", "raw_refs", "dependency_refs"], "OHLC形态来源组"); const mergedIndex = await I(await field(g, "merged_index"));
      need(previousIndex === null || mergedIndex === previousIndex + 1n, "形态来源组序号不连续"); previousIndex = mergedIndex;
      const sets = {}; let firstRaw = null;
      for (const role of ["raw_refs", "dependency_refs"]) {
        const groupRefs = await array(await field(g, role), "形态来源角色不是数组"), set = new Set(); let last = -1n;
        for await (const r of groupRefs.items()) {
          await reference(r, "形态来源引用", "形态来源修订不符"); const coordValue = await field(r, "source_coord"), coord = await S(coordValue), n = await I(coordValue);
          need(n > last, "来源坐标未严格递增/重复"); last = n; all.add(coord); set.add(coord);
          if (role === "raw_refs" && firstRaw === null) firstRaw = coordValue;
        }
        sets[role] = set;
      }
      let disjoint = true; for (const coord of sets.dependency_refs) disjoint = disjoint && !sets.raw_refs.has(coord);
      need(sets.raw_refs.size > 0 && disjoint, "形态将构造/边界依据伪作真实成员");
      need(await equal(firstRaw, anchors[index++]), "形态实际成员首坐标与窗口锚不同");
    }
    const coords = await coordinates(await field(shape, "source_coords"));
    // source_coords 已严格递增，等长加每项属于完整并集等价于原排序后全数组比较。
    need(await coords.length() === BigInt(all.size), "形态来源不是成员与构造/边界依据并集");
    for await (const coord of coords.items()) need(all.has(await S(coord)), "形态来源不是成员与构造/边界依据并集");
  }
  // #1491：bounded-layout/1 的 CC-007 三跳只读——分型对象 → 批记录 entry → 单元序列（§5.6 分片由
  // Value 读取器按需取）。只核单元确实是该分型的（参考价、bar 四价逐项相同），不重算 order（#804）。
  async function bindDevelopmentBatches(records, layout) {
    const fractals = new Map(), batches = [];
    let anyBatch = false;
    for (const record of records) {
      const kind = await S(await field(record, "kind"));
      anyBatch = anyBatch || kind === DEVELOPMENT_BATCH_KIND;
      if (!(await eqField(record, "lifecycle", "active"))) continue;
      if (kind === TB02_KINDS[2]) {
        const window = await V.list(await field(await field(record, "payload"), "window"));
        fractals.set(await S(await field(record, "object_id")), {record, start: BigInt(await S(window[0])), end: BigInt(await S(window[2]))});
      } else if (kind === DEVELOPMENT_BATCH_KIND) {
        batches.push({record, coord: BigInt(await S(await field(await field(await field(record, "payload"), "bar"), "source_coord")))});
      }
    }
    need(layout === LAYOUTS.BOUNDED1 || !anyBatch, "layout/1 库不含单元批记录");
    batches.sort((a, b) => (a.coord < b.coord ? -1 : a.coord > b.coord ? 1 : 0));
    const last = new Map();
    for (const {record: batch, coord} of batches) {
      const p = await field(batch, "payload"), bar = await field(p, "bar"); let previous = null;
      for await (const entry of (await field(p, "entries")).items()) {
        const id = await S(await field(entry, "fractal_object_id")), fractal = fractals.get(id);
        need(fractal, "单元批记录指向缺失分型");
        // entries 现按分型窗口序：纯追加下与 first_known_generation 序相同，中段插入时分叉（#1491 PR #1780 M-2 待裁定）。
        need(previous === null || fractal.start > previous, "单元批记录 entries 不是分型窗口序"); previous = fractal.start;
        need(coord > fractal.end, "单元不在分型右组之后");
        const prices = []; for await (const price of (await field(await field(fractal.record, "payload"), "reference_prices")).items()) prices.push(price);
        let i = 0;
        for await (const r of (await field(await field(entry, "unit"), "relations")).items()) {
          const axis = ["open", "high", "low", "close"][Math.floor(i / 6)], ref = prices[i++ % 6];
          await keys(r, ["axis", "value", "reference", "reference_value", "order"], "后续关系");
          need(await eqField(r, "axis", axis) && await equal(await field(r, "value"), await field(bar, axis)) && await equal(await field(r, "reference"), await field(ref, "name")) && await equal(await field(r, "reference_value"), await field(ref, "value")), "后续关系来源不同");
          await enumValue(await field(r, "order"), ["LT", "EQ", "GT"], "后续关系来源不同");
        }
        const count = await I(await field(await field(entry, "development"), "count"), true);
        need(count === (last.get(id) || 0n) + 1n, "分型单元序列不连续"); last.set(id, count);
      }
    }
    return last;
  }
  return Object.freeze({RULE_CONTRACTS, splitRule, ruleContract, LAYOUTS, DEVELOPMENT_BATCH_KIND, bindDevelopmentBatches, validateTyped, validateBiPayload, validateSegmentPayload, turnLinks: (d, f) => segmentTurnLinks(d, f), PARSE_INVALID_REASON, RESULT_SEMANTIC_BRANCH, validateTB02ShapeRefs, plain,
    TB02_KINDS: Object.freeze(TB02_KINDS), BI_KINDS, SEGMENT_KINDS: Object.freeze(SEGMENT_KINDS)});
});
