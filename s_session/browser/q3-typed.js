/* #1394：typed 与形态来源的同一判据；只把访问迁为完整异步 Value 遍历。 */
(function (root, factory) {
  const api = factory(typeof module === "object" && module.exports ? require("./q3-values.js") : root.Q3Values);
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.Q3Typed = api;
})(globalThis, function (V) {
  "use strict";
  const TB02_KINDS = ["CC-004.inclusion_step", "CC-005.inclusion_group", "CC-007.fractal_description", "CC-054.knowledge_state"];
  const BI_KINDS = ["CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version"];
  TB02_KINDS.push(...BI_KINDS);
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
  async function validateTyped(record) {
    record = V.of(record);
    await keys(record, ["object_id", "object_revision", "kind", "batch_id", "fact_key", "payload", "input_refs", "source_coords", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by", "lifecycle"], "typed 对象");
    for (const k of ["object_id", "kind", "batch_id", "fact_key", "first_known_cut"]) await T(await field(record, k));
    const kind = await S(await field(record, "kind"));
    need(TB02_KINDS.includes(kind) && await eqField(record, "object_revision", "1"), "typed kind/revision 不符");
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
    if (BI_KINDS.includes(kind)) await validateBiPayload(kind, p);
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
      await keys(p, ["shape_object_id", "branch", "window", "description", "reference_prices", "descriptive_labels", "subsequent_development"], "分型描述");
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
    } else {
      await keys(p, ["scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests"], "知识状态");
      need(await eqField(p, "scope", "TB-02-A") && await eqField(p, "domain", "established_direction_without_extreme_identity_competition") && await eqField(p, "known_facts", ["raw_ohlc", "inclusion_trace", "group_provenance"]), "知识域/已知事实不符");
      await I(await field(p, "input_frontier"), false, true); await texts(await field(p, "waiting_reasons")); const requests = await array(await field(p, "requests"), "逐请求集合不完整"), ids = new Set();
      const enums = {input_quality:["SUFFICIENT", "INSUFFICIENT", "INCONSISTENT"], applicability:["IN_DOMAIN", "OUTSIDE_DOMAIN", "DOMAIN_PROOF_MISSING"], computation:["COMPLETE", "INCOMPLETE"], validity:["CURRENT", "SUPERSEDED", "WITHDRAWN"]};
      for await (const r of requests.items()) {
        await keys(r, ["request_id", "subject_id", "axes", "reasons", "source_coords"], "请求状态"); const id = await T(await field(r, "request_id")); need(!ids.has(id), "请求身份重复"); ids.add(id);
        const subject = await field(r, "subject_id"); if (!(isNull(subject))) await T(subject); const axes = await field(r, "axes"); await keys(axes, Object.keys(enums), "请求四轴");
        for (const k of Object.keys(enums)) await enumValue(await field(axes, k), enums[k], "请求四轴枚举不符"); await texts(await field(r, "reasons")); await coordinates(await field(r, "source_coords"));
      }
    }
    const slot = BI_KINDS.includes(kind) ? await field(p, "slot") : kind === TB02_KINDS[0] ? await field(await field(p, "incoming"), "source_coord") : kind === TB02_KINDS[1] ? await field(p, "group_anchor") : kind === TB02_KINDS[2] ? await field(p, "window") : "TB-02-A";
    need(await canonicalMatches([kind, slot], await S(await field(record, "fact_key"))), "事实语义槽未绑定 payload");
  }
  async function validateBiPayload(kind, p) {
    p = V.of(p);
    await keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "新笔合同");
    need(await eqField(p, "schema_revision", "s-new-bi/1") && await eqField(p, "semantic_version", "new-bi-dual-coordinate/1") && await eqField(p, "policy_version", "standard_raw_gap_3") && await eqField(p, "view_role", "main"), "新笔规则/政策版本不符");
    await I(await field(p, "object_generation"), true); const d = await field(p, "data");
    async function endpoint(e) {
      await keys(e, ["kind", "merged_index", "group_anchor", "price", "extreme_roots", "raw_position", "source_coords", "sealed_at", "waiting_reasons"], "新笔端点");
      await enumValue(await field(e, "kind"), ["TOP", "BOTTOM"], "端点类型不符"); await I(await field(e, "merged_index")); await I(await field(e, "group_anchor")); await N(await field(e, "price"));
      const roots = await coordinates(await field(e, "extreme_roots")), sources = await coordinateSet(await field(e, "source_coords")); await texts(await field(e, "waiting_reasons"));
      for await (const root of roots.items()) need(sources.has(await S(root)), "实际根缺原始来源");
      for (const k of ["raw_position", "sealed_at"]) { const value = await field(e, k); if (!(isNull(value))) await I(value); }
    }
    async function conditions(c) {
      await keys(c, ["merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"], "新笔完整条件");
      await I(await field(c, "merged_gap")); const raw = await field(c, "raw_between_actual_extrema"); if (!(isNull(raw))) await I(raw); await N(await field(c, "top_price")); await N(await field(c, "bottom_price"));
      const vector = await array(await field(c, "vector"), "条件向量缺位/错型"); need(await vector.length() === 3n, "条件向量缺位/错型");
      for await (const value of vector.items()) need(["null", "bool"].includes(value.kind), "条件向量缺位/错型");
      await texts(await field(c, "failed_conditions")); await texts(await field(c, "waiting_reasons"));
    }
    async function known(k) {
      await keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "新笔获知时点");
      await I(await field(k, "generation"), true); await I(await field(k, "input_frontier")); await T(await field(k, "receipt_id")); await T(await field(k, "received_at")); const ns = await field(k, "semantic_commit_ns"); if (!(isNull(ns))) await I(ns);
    }
    if (kind === BI_KINDS[0]) await endpoint(d);
    else if ([BI_KINDS[1], BI_KINDS[2]].includes(kind)) {
      await keys(d, ["old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", "waiting_reasons"], "端点对");
      const old = await field(d, "old"), fresh = await field(d, "new"); await endpoint(old); await endpoint(fresh);
      const oldKind = await S(await field(old, "kind")), newKind = await S(await field(fresh, "kind")); need(await eqField(d, "endpoint_kind_pair", oldKind + "/" + newKind), "端点分域标签不符");
      if (kind === BI_KINDS[1]) { need(oldKind !== newKind, "异型请求实际同型"); await conditions(await field(d, "conditions")); need(await eqField(d, "comparison", null) && await eqField(d, "selection", null), "异型域混入同型选择"); }
      else {
        need(oldKind === newKind && await eqField(d, "conditions", null), "同型域混入假条件或未声明取舍");
        await enumValue(await field(d, "comparison"), ["LT", "EQ", "GT"], "同型域混入假条件或未声明取舍"); await enumValue(await field(d, "selection"), ["KEEP", "REPLACE", "UNDETERMINED"], "同型域混入假条件或未声明取舍");
      }
      await coordinates(await field(d, "retained_anchors")); await texts(await field(d, "waiting_reasons"));
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
        const synthetic = {}; for (const header of ["schema_revision", "semantic_version", "policy_version", "view_role"]) synthetic[header] = await field(p, header);
        synthetic.slot = await field(entity, "entity_id"); synthetic.object_generation = entityId.split(":")[1]; synthetic.data = entity;
        await validateBiPayload(BI_KINDS[3], synthetic);
      }
    } else {
      await keys(d, ["input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"], "新笔版本");
      await I(await field(d, "input_frontier"), false, true); await texts(await field(d, "version_causes")); await texts(await field(d, "waiting_reasons")); await known(await field(d, "known_at"));
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
  return Object.freeze({validateTyped, validateBiPayload, validateTB02ShapeRefs, TB02_KINDS, BI_KINDS});
});
