/* #1419/#1424：TB-02-E 中枢（s-center/1）与 TB-02-F 相邻中枢对（s-center-pair/1）的 typed 判据。
 * 判据与原因码逐字移植自旧非拆分客户端（c74a0efa28:s_session/browser/tb01c-client.js，分支 codex/1424-center-pair-20261014），
 * 与当时 s_tb02_contract.py 同序同原因码；按 #1394 拆分风格独立成模块，只在完整普通原件（q3-typed.plain）上执行，
 * 判据不改。字段逐项对过当前 Rust：s_session_v2/tb02e.rs、tb02f.rs 与 s_session_v3/content_structure.rs 的中枢/对构造。
 * 规则绑定（RULE_MIX）由 q3-consumer tb02Identity 按 cut 规则做；旧 validateCenterRule/validatePairRule 的规则分支不在此重复。 */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.Q3Center = api;
})(globalThis, function () {
  "use strict";
  const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
  // 只用于两侧同构普通值的相等比较（键序无关）；不产出规范字节。
  function canon(v) {
    if (Array.isArray(v)) return "[" + v.map(canon).join(",") + "]";
    if (object(v)) return "{" + Object.keys(v).sort().map(k => JSON.stringify(k) + ":" + canon(v[k])).join(",") + "}";
    return JSON.stringify(v === undefined ? null : v);
  }
  const equal = (a, b) => canon(a) === canon(b);
  const CENTER_KINDS = ["CC-016.center_seed_attempt", "CC-016.center", "CC-017.center_roles", "CC-022.unit_assignment", "CC-055.center_relation", "CC-055.center_change", "CC-056.center_version"];
  const PAIR_KINDS = ["CC-018.unit_core_position", "CC-020.center_pair", "CC-055.pair_relation", "CC-055.pair_change", "CC-056.pair_version"];
  // ── #1419 TB-02-E 中枢事实（schema s-center/1，tb1419-schema.md v1）。与 s_tb02_contract.py 同序同原因码，
  // test_tb02e_contract 对共享语料逐条对拍两侧结论与原因码。
  const CENTER_TUPLE = ["s-center/1", "center-seed/1", "strict_seed_zn_outer_c_resume", "main"];
  const TB02E_DELIVERED = ["l0_strict_seed_vector_and_core_order", "zn_outer_inner_roles_with_contributors", "c_resume_assignment_and_connectors"];
  const TB02E_DEFERRED = ["global_G009_upper_levels_1429", "ge9_upgrade_1429", "third_class_and_break_1434", "expansion_newborn_1424", "connector_upper_construction_1429"];
  const ROLE_LAYOUT = [["CORE", ["ZD", "ZG"]], ["OUTER", ["DD", "GG"]], ["INNER_EXTREMA", ["D", "G"]]];
  const ROLE_RULE = {ZD: [2, "max"], ZG: [3, "min"], DD: [2, "min"], GG: [3, "max"], D: [2, "max"], G: [3, "min"]};
  const WAIT_REASONS = ["NO_LEFT_CENTER_IN_WINDOW", "SEED_NOT_YET_EVALUABLE", "UPGRADE_PENDING_TB02H", "UNIT_NOT_COMPLETED"];
  const isCenterFamily = kind => ["CC-016.", "CC-017.", "CC-022."].some(p => kind.startsWith(p));
  function needE(ok, code, message) { if (!ok) throw new Error("TB02E_" + code + ": " + message); }
  function keysE(value, expected, name) {
    needE(object(value) && Object.keys(value).length === expected.length && expected.every(k => Object.hasOwn(value, k)), "SHAPE", name + " 字段缺失/多余");
  }
  function intE(value, name, minimum = null) {
    needE(typeof value === "string" && value.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value), "SHAPE", name + " 必须是规范 i64 文本");
    const n = BigInt(value);
    needE(n >= -9223372036854775808n && n <= 9223372036854775807n && (minimum === null || n >= BigInt(minimum)), "SHAPE", name + " 超出整数域");
    return n;
  }
  function textE(value, name) { needE(typeof value === "string" && value !== "", "SHAPE", name + " 必须是非空文本"); return value; }
  function unitAnchor(unitId) {
    needE(typeof unitId === "string" && /^segment:[1-9][0-9]*:(?:0|[1-9][0-9]*)$/.test(unitId), "IDENTITY", "单元身份须为 segment:<代>:<锚>");
    return unitId.slice("segment:".length);
  }
  function knownE(k) {
    keysE(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "known_at");
    intE(k.generation, "generation", 1); intE(k.input_frontier, "input_frontier", -1);
    for (const name of ["receipt_id", "received_at"]) if (k[name] !== null) textE(k[name], name);
    if (k.semantic_commit_ns !== null) intE(k.semantic_commit_ns, "semantic_commit_ns", 0);
  }
  function completedUnit(u, name) {
    keysE(u, ["unit_id", "unit_object_id", "direction", "lo", "hi"], name);
    unitAnchor(u.unit_id); textE(u.unit_object_id, name + ".unit_object_id");
    needE(["UP", "DOWN"].includes(u.direction), "SHAPE", name + ".direction 未声明");
    needE(intE(u.lo, name + ".lo") < intE(u.hi, name + ".hi"), "UNIT", name + " 区间须 lo<hi");
  }
  function idList(xs, name) {
    needE(Array.isArray(xs) && xs.every(x => typeof x === "string" && x !== ""), "SHAPE", name + " 须为文本数组");
    needE(new Set(xs).size === xs.length, "SHAPE", name + " 重复");
    return xs;
  }
  const witnessOk = (contributors, witness) => equal(witness, contributors.length >= 2 ? contributors : []);
  function validateRoles(roles) {
    keysE(roles, ROLE_LAYOUT.map(([r]) => r), "roles");
    for (const [role, names] of ROLE_LAYOUT) {
      needE(object(roles[role]) && Object.keys(roles[role]).length === names.length && names.every(n => Object.hasOwn(roles[role], n)), "ROLES_SHAPE", role + " 角色缺变量或混入他角色变量（D/G 不得冒充核心）");
      for (const name of names) {
        const v = roles[role][name];
        keysE(v, ["value", "contributors", "same_price_witness", "frozen", "null_reason"], name);
        needE(v.frozen === (role === "CORE"), "ROLES_SHAPE", "只有核心冻结");
        const contributors = idList(v.contributors, name + ".contributors");
        if (v.value === null) needE(role !== "CORE" && v.null_reason === "UPGRADE_PENDING_TB02H" && contributors.length === 0, "ROLE_NULL", "只有 ≥9 的动态变量可为 null 且须带原因");
        else {
          intE(v.value, name);
          needE(v.null_reason === null, "ROLE_NULL", "有值变量不得带 null 原因");
          needE(contributors.length >= 1, "CONTRIBUTORS", name + " 缺贡献成员");
        }
        needE(witnessOk(contributors, v.same_price_witness), "WITNESS", name + " 同价见证不符");
      }
    }
    const val = Object.fromEntries(ROLE_LAYOUT.flatMap(([role, names]) => names.map(n => [n, roles[role][n].value])));
    needE(BigInt(val.ZD) < BigInt(val.ZG), "ROLE_ORDER", "核心须 ZD<ZG");
    if (val.DD !== null && val.GG !== null) needE(BigInt(val.DD) <= BigInt(val.ZD) && BigInt(val.ZG) <= BigInt(val.GG), "ROLE_ORDER", "须 DD≤ZD<ZG≤GG");
    const nulls = ["DD", "GG", "D", "G"].map(n => val[n] === null);
    needE(nulls.every(x => x === nulls[0]), "ROLE_NULL", "动态四变量须同时有值或同时置 null");
  }
  function validateCenterPayload(kind, p) {
    keysE(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "中枢合同");
    needE(equal([p.schema_revision, p.semantic_version, p.policy_version, p.view_role], CENTER_TUPLE), "VERSION", "中枢规则/范围版本不符");
    intE(p.object_generation, "object_generation", 1);
    const d = p.data;
    if (kind === CENTER_KINDS[0]) {
      keysE(d, ["entity_id", "units", "vector", "ZD", "ZG", "core_order", "result", "exclusion_reason", "center_id", "known_at"], kind);
      const units = d.units;
      needE(Array.isArray(units) && units.length === 3, "SHAPE", "seed 须恰三单元");
      units.forEach(u => completedUnit(u, "seed.unit"));
      needE(new Set(units.map(u => u.unit_id)).size === 3, "SHAPE", "seed 单元重复");
      const anchor = unitAnchor(units[0].unit_id);
      needE(d.entity_id === "seed_attempt:" + anchor, "IDENTITY", "seed 尝试身份未绑定首单元");
      const v = d.vector;
      keysE(v, ["completed", "consecutive", "same_level", "alternating", "strict"], "seed.vector");
      needE(["completed", "consecutive", "same_level", "alternating"].every(x => typeof v[x] === "boolean") && (v.strict === null || typeof v.strict === "boolean"), "SHAPE", "Bool⁵ 错型");
      needE(d.exclusion_reason !== "NOT_ALTERNATING" && v.alternating, "NOT_ALTERNATING", "L0 段恒交替，NOT_ALTERNATING 一律拒");
      needE(v.completed && v.consecutive && v.same_level, "VECTOR", "已发布的 seed 尝试须三单元完成、相连、同级");
      needE((v.strict === null) === !v.completed, "VECTOR", "strict 仅在未完成时为 null");
      const lows = units.map(u => BigInt(u.lo)), highs = units.map(u => BigInt(u.hi));
      const big = (xs, pick) => xs.reduce((a, b) => (pick === "max" ? (b > a ? b : a) : (b < a ? b : a)));
      for (const [name, want, attr] of [["ZD", big(lows, "max"), "lo"], ["ZG", big(highs, "min"), "hi"]]) {
        const e = d[name];
        keysE(e, ["value", "contributors", "same_price_witness"], "seed." + name);
        needE(intE(e.value, name) === want, "CORE_VALUE", name + " 值与 seed 三单元不符");
        idList(e.contributors, name + ".contributors");
        needE(equal(e.contributors, units.filter(u => BigInt(u[attr]) === want).map(u => u.unit_id)), "CONTRIBUTORS", name + " 贡献成员缺失或不符");
        needE(witnessOk(e.contributors, e.same_price_witness), "WITNESS", name + " 同价见证不符");
      }
      const zd = BigInt(d.ZD.value), zg = BigInt(d.ZG.value);
      const order = zd < zg ? "ZD<ZG" : zd === zg ? "ZD=ZG" : "ZD>ZG";
      needE(d.core_order === order, "CORE_ORDER", "核心序三叶与 ZD/ZG 不符");
      needE(v.strict === (order === "ZD<ZG"), "VECTOR", "strict 位与核心序不符");
      needE(["FORMED", "EXCLUDED"].includes(d.result), "SHAPE", "seed 结果未声明");
      const formed = d.result === "FORMED";
      needE(formed === (v.strict === true), "RESULT", "FORMED 须五位全真");
      if (formed) {
        needE(d.exclusion_reason === null, "EXCLUSION_REASON", "成立的 seed 不得带排除原因");
        needE(d.center_id === "center:" + anchor, "IDENTITY", "成立的 seed 须指向同锚中枢");
      } else {
        needE(d.exclusion_reason === (order === "ZD=ZG" ? "TANGENT_ZD_EQ_ZG" : "INVERTED_ZD_GT_ZG"), "EXCLUSION_REASON", "相切/倒置缺原因或原因不符");
        needE(d.center_id === null, "RESULT", "不成立的 seed 不得指向中枢");
      }
      knownE(d.known_at);
    } else if (kind === CENTER_KINDS[1]) {
      keysE(d, ["entity_id", "entity_revision", "seed_attempt_id", "seed_units", "zn_direction", "members", "zn_members", "status", "settled", "break_unit", "break_up", "break_up_defined", "resume_anchor", "next_tested_unit", "roles_id", "formed_known_at", "settled_known_at"], kind);
      const members = idList(d.members, "members");
      needE(members.length >= 3, "MEMBERS", "中枢至少 seed 三单元");
      members.forEach(unitAnchor);
      const anchor = unitAnchor(members[0]);
      needE(d.entity_id === "center:" + anchor && d.seed_attempt_id === "seed_attempt:" + anchor && d.roles_id === "roles:" + d.entity_id, "IDENTITY", "中枢身份未绑定 seed 首单元");
      intE(d.entity_revision, "entity_revision", 1);
      needE(equal(d.seed_units, members.slice(0, 3)), "MEMBERS", "members 须以 seed 三单元开头");
      needE(d.break_unit === null || !members.includes(d.break_unit), "CONNECTOR_AS_MEMBER", "突破段不得是成员");
      const zn = idList(d.zn_members, "zn_members");
      needE(zn.length >= 1 && zn[0] === members[0] && zn.every(z => members.includes(z)), "ZN", "Zn 成员须含首单元且属成员");
      needE(["UP", "DOWN"].includes(d.zn_direction), "SHAPE", "zn_direction 未声明");
      needE(["EXTENDING", "SETTLED", "UPGRADE_PENDING_TB02H"].includes(d.status), "SHAPE", "中枢状态未声明");
      needE((d.status === "UPGRADE_PENDING_TB02H") === (members.length >= 9), "UPGRADE", "≥9 成员须且只须标 UPGRADE_PENDING_TB02H");
      needE([d.settled, d.break_up, d.break_up_defined].every(x => typeof x === "boolean"), "SHAPE", "结算位错型");
      const settled = d.settled;
      if (d.status !== "UPGRADE_PENDING_TB02H") needE((d.status === "SETTLED") === settled, "SETTLEMENT", "状态与 settled 不符");
      needE((d.break_unit !== null) === settled && (d.resume_anchor !== null) === settled && (d.settled_known_at !== null) === settled, "SETTLEMENT", "结算位与突破段/续扫锚/结算时点不一致");
      needE(d.break_up_defined === settled, "BREAK_UP_UNDEFINED", "未结算中枢 break_up_defined 须为 false（R4）");
      if (!settled) {
        needE(d.break_up === false, "BREAK_UP_UNDEFINED", "未结算中枢 break_up 须为 false（R4）");
        if (d.next_tested_unit !== null) { unitAnchor(d.next_tested_unit); needE(!members.includes(d.next_tested_unit), "NEXT_TESTED", "待测单元不得是成员"); }
      } else {
        unitAnchor(d.break_unit);
        needE(d.next_tested_unit === null, "NEXT_TESTED", "结算后不再有待测延伸单元");
        keysE(d.resume_anchor, ["after_unit_id", "offset"], "resume_anchor");
        needE(d.resume_anchor.after_unit_id === members[members.length - 1] && d.resume_anchor.offset === "2", "RESUME_ANCHOR", "续扫锚须为 {末成员, 2}（R8）");
        knownE(d.settled_known_at);
      }
      knownE(d.formed_known_at);
    } else if (kind === CENTER_KINDS[2]) {
      keysE(d, ["entity_id", "entity_revision", "center_id", "roles", "previous", "known_at"], kind);
      needE(typeof d.center_id === "string" && d.center_id.startsWith("center:") && d.entity_id === "roles:" + d.center_id, "IDENTITY", "角色身份未绑定中枢");
      const revision = intE(d.entity_revision, "entity_revision", 1);
      validateRoles(d.roles);
      if (d.previous !== null) {
        keysE(d.previous, ["entity_revision", "roles"], "previous");
        needE(intE(d.previous.entity_revision, "previous.entity_revision", 1) < revision, "SHAPE", "previous 修订号须小于当前");
        validateRoles(d.previous.roles);
      }
      knownE(d.known_at);
    } else if (kind === CENTER_KINDS[3]) {
      keysE(d, ["entity_id", "unit", "status", "center_id", "left_center_id", "right_center_id", "right_pending", "reason", "seed_from_self", "known_at"], kind);
      const u = d.unit, completed = !(object(u) && Object.hasOwn(u, "completed"));
      if (completed) completedUnit(u, "assignment.unit");
      else {
        keysE(u, ["unit_id", "unit_object_id", "completed"], "assignment.unit");
        unitAnchor(u.unit_id); textE(u.unit_object_id, "unit_object_id");
        needE(u.completed === false, "SHAPE", "进行中单元 completed 须为 false");
      }
      needE(d.entity_id === "assign:" + u.unit_id, "IDENTITY", "归属身份未绑定单元");
      needE(["FORMED", "FAILED", "NOT_EVALUATED"].includes(d.seed_from_self), "SHAPE", "seed_from_self 未声明");
      needE(typeof d.right_pending === "boolean", "SHAPE", "right_pending 错型");
      let ok = false;
      if (d.status === "CENTER_MEMBER") ok = d.center_id !== null && d.left_center_id === null && d.right_center_id === null && !d.right_pending && d.reason === null;
      else if (d.status === "CONNECTOR") ok = d.center_id === null && d.left_center_id !== null && (d.right_center_id !== null) !== d.right_pending && d.reason === null;
      else if (d.status === "NOT_YET_ASSIGNED") ok = WAIT_REASONS.includes(d.reason) && d.center_id === null && d.left_center_id === null && d.right_center_id === null && !d.right_pending;
      needE(ok, "ASSIGNMENT", "归属三值须恰一且字段与状态一致");
      needE(completed !== (d.reason === "UNIT_NOT_COMPLETED"), "ASSIGNMENT", "UNIT_NOT_COMPLETED 当且仅当单元进行中");
      if (!completed) needE(d.seed_from_self === "NOT_EVALUATED", "ASSIGNMENT", "进行中单元未被评估过 seed");
      knownE(d.known_at);
    } else if (kind === CENTER_KINDS[4]) {
      keysE(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], kind);
      for (const name of ["source_id", "target_id", "witness_object_id"]) textE(d[name], name);
      needE(["member_of", "connects_left", "connects_right", "attempted_from"].includes(d.relation_kind), "SHAPE", "未声明中枢关系");
      intE(d.version, "version", 1);
    } else if (kind === CENTER_KINDS[5]) {
      keysE(d, ["entity_id", "change", "before", "after", "known_at", "object_id"], kind);
      textE(d.entity_id, "entity_id");
      needE(["formed", "member_added", "roles_updated", "settled", "upgrade_pending", "withdrawn"].includes(d.change), "SHAPE", "未声明中枢变化");
      const withdrawn = d.change === "withdrawn";
      needE((d.after === null) === withdrawn && (d.object_id === null) === withdrawn && (d.before === null) === (d.change === "formed"), "SHAPE", "变化前后件与变化种类不符");
      if (d.object_id !== null) textE(d.object_id, "object_id");
      knownE(d.known_at);
    } else if (kind === CENTER_KINDS[6]) {
      keysE(d, ["input_frontier", "known_at", "scan_cursor", "waiting_reasons", "delivered_scope", "deferred_scope"], kind);
      intE(d.input_frontier, "input_frontier", -1); knownE(d.known_at);
      needE(Array.isArray(d.waiting_reasons) && d.waiting_reasons.every(r => WAIT_REASONS.includes(r)), "SHAPE", "等待原因未声明");
      needE(d.delivered_scope === "l0_strict_seed_zn_roles_c_resume_assignment" && equal(d.deferred_scope, TB02E_DEFERRED), "SCOPE", "中枢交付范围不符");
      const c = d.scan_cursor;
      keysE(c, ["state", "center_id", "seed_start_unit_id", "awaiting_unit_id", "awaiting_after_unit_id", "awaiting_offset"], "scan_cursor");
      if (c.state === "EXTENDING") needE(typeof c.center_id === "string" && c.center_id.startsWith("center:") && c.seed_start_unit_id === null, "SCAN_CURSOR", "EXTENDING 须指中枢且无 seed 起点");
      else {
        needE(c.state === "SEEKING_SEED" && c.center_id === null, "SCAN_CURSOR", "游标状态未声明");
        if (c.seed_start_unit_id !== null) unitAnchor(c.seed_start_unit_id);
      }
      if (c.awaiting_unit_id !== null) {
        unitAnchor(c.awaiting_unit_id);
        needE(c.awaiting_after_unit_id === null && c.awaiting_offset === null, "SCAN_CURSOR", "等待单元与实体锚恰一组");
      } else {
        needE(intE(c.awaiting_offset, "awaiting_offset", 1) >= 1n, "SCAN_CURSOR", "awaiting_offset 须 ≥1");
        if (c.awaiting_after_unit_id !== null) unitAnchor(c.awaiting_after_unit_id);
      }
    } else needE(false, "CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）");
    const slot = kind === CENTER_KINDS[4] ? [d.source_id, d.relation_kind, d.target_id] : kind === CENTER_KINDS[5] ? [d.known_at.generation, d.entity_id, d.change] : kind === CENTER_KINDS[6] ? "TB-02-E" : d.entity_id;
    needE(equal(p.slot, slot), "SLOT", "中枢语义槽身份不符");
  }
  // 完成段成员笔的方向极值（schema v1.3 §1，根裁定 ROOT-RULING-1419-L3；与 s_tb02_contract._member_extremes 对称）：
  // stroke_refs 中下标属于 construction.stroke_indices 的笔，取全部笔端价的 [min, max]；不取几何端点，也不取全部 stroke_refs。
  function memberExtremes(d) {
    const refs = new Map(d.stroke_refs.map(r => [r.stroke_index, r])), members = d.construction.stroke_indices;
    needE(members.length > 0 && members.every(m => refs.has(m)), "UNIT_SOURCE", "完成段成员笔不在同对象 stroke_refs 中");
    const prices = members.flatMap(m => [BigInt(refs.get(m).start_price), BigInt(refs.get(m).end_price)]);
    return [prices.reduce((a, b) => (b < a ? b : a)), prices.reduce((a, b) => (b > a ? b : a))];
  }
  function segmentUnits(records) {
    const rows = [];
    for (const r of records) {
      if (r.kind !== "CC-013.segment") continue;
      const d = r.payload.data, done = ["CASE_ONE", "CASE_TWO"].includes(d.result), start = BigInt(d.construction.start_stroke);
      if (done) {
        const [lo, hi] = memberExtremes(d);
        rows.push([start, d.entity_id, r.object_id, lo, hi, d.construction.direction, true]);
      } else rows.push([start, d.entity_id, r.object_id, null, null, null, false]);
    }
    rows.sort((x, y) => (x[0] < y[0] ? -1 : x[0] > y[0] ? 1 : 0));
    return [rows.filter(r => r[6]).map(r => r.slice(1, 6)), new Map(rows.filter(r => !r[6]).map(r => [r[1], r[2]]))];
  }
  function validateScanCursor(versions, completed, pending, pos, centers, attempts) {
    needE(versions.length === 1, "SCAN_CURSOR", "每 cut 恰一个中枢版本对象");
    const cur = versions[0].payload.data.scan_cursor, n = completed.length;
    needE(pending.size <= 1, "SCAN_CURSOR", "至多一个进行中单元");
    const ip = pending.size ? [...pending.keys()][0] : null;
    const unitAt = k => (k >= 0 && k < n ? completed[k][0] : k === n ? ip : null);
    const last = centers.length ? centers[centers.length - 1] : null;
    let wait;
    if (last !== null && !last.settled) {
      needE(cur.state === "EXTENDING" && cur.center_id === last.entity_id && cur.seed_start_unit_id === null, "SCAN_CURSOR", "末中枢开放时游标须 EXTENDING 指末中枢");
      wait = pos.get(last.members[last.members.length - 1]) + 1;
    } else {
      let s = last !== null ? pos.get(last.members[last.members.length - 1]) + 2 : 0;
      while (s < n && attempts.get("seed_attempt:" + unitAnchor(completed[s][0]))?.result === "EXCLUDED") s++;
      needE(cur.state === "SEEKING_SEED" && cur.center_id === null && cur.seed_start_unit_id === unitAt(s), "SCAN_CURSOR", "SEEKING_SEED 起点须为续扫锚经 EXCLUDED 尝试推进后的位置");
      wait = s + 2;
    }
    let want;
    if (wait === n && ip !== null) want = [ip, null, null];
    else {
      needE(wait > n - 1, "SCAN_CURSOR", "等待位置不得落在已完成单元上");
      want = [null, n ? completed[n - 1][0] : null, String(wait - (n - 1))];
    }
    needE(cur.awaiting_unit_id === want[0] && cur.awaiting_after_unit_id === want[1] && cur.awaiting_offset === want[2], "SCAN_CURSOR", "等待位置的实体锚不符");
    if (last !== null && !last.settled) needE(last.next_tested_unit === want[0], "SCAN_CURSOR", "游标与中枢 next_tested_unit 不一致");
  }
  function validateCenterLinks(records, axes = null) {
    const byKind = Object.fromEntries(CENTER_KINDS.map(k => [k, records.filter(r => r.kind === k)]));
    if (!Object.values(byKind).some(xs => xs.length)) return;
    for (const r of records) needE(!isCenterFamily(r.kind) || CENTER_KINDS.includes(r.kind), "CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）");
    const [completed, pending] = segmentUnits(records);
    const pos = new Map(completed.map((u, i) => [u[0], i])), seg = new Map(completed.map(u => [u[0], u]));
    const checkUnit = u => {
      if (Object.hasOwn(u, "completed")) { needE(pending.get(u.unit_id) === u.unit_object_id, "UNIT_SOURCE", "进行中单元未绑定同 cut 段对象"); return; }
      needE(seg.has(u.unit_id), "MEMBER_NOT_COMPLETED", "单元不是同 cut 已完成段");
      const s = seg.get(u.unit_id);
      needE(u.unit_object_id === s[1] && BigInt(u.lo) === s[2] && BigInt(u.hi) === s[3] && u.direction === s[4], "UNIT_SOURCE", "单元区间/方向/对象与同 cut CC-013 不符");
    };
    const attempts = new Map(byKind[CENTER_KINDS[0]].map(r => [r.payload.data.entity_id, r.payload.data]));
    for (const a of attempts.values()) {
      a.units.forEach(checkUnit);
      const ids = a.units.map(u => u.unit_id), p0 = pos.get(ids[0]);
      needE(ids.every((id, i) => pos.get(id) === p0 + i), "SEED_LINK", "seed 三单元须在完成单元序中相连");
    }
    const roles = new Map(byKind[CENTER_KINDS[2]].map(r => [r.payload.data.center_id, r.payload.data]));
    const centers = byKind[CENTER_KINDS[1]].map(r => r.payload.data).sort((x, y) => (pos.get(x.members[0]) ?? -1) - (pos.get(y.members[0]) ?? -1));
    const assignments = new Map();
    for (const r of byKind[CENTER_KINDS[3]]) {
      const d = r.payload.data; checkUnit(d.unit);
      needE(!assignments.has(d.unit.unit_id), "ASSIGNMENT_CONSISTENCY", "单元归属重复");
      assignments.set(d.unit.unit_id, d);
    }
    const universe = new Set([...seg.keys(), ...pending.keys()]);
    needE(assignments.size === universe.size && [...universe].every(u => assignments.has(u)), "ASSIGNMENT_CONSISTENCY", "每个单元须恰一归属");
    const memberOf = new Map();
    for (const c of centers) {
      for (const m of c.members) needE(seg.has(m), "MEMBER_NOT_COMPLETED", "成员不是同 cut 已完成段");
      const first = pos.get(c.members[0]);
      needE(c.members.every((m, i) => pos.get(m) === first + i), "MEMBERS", "成员须在完成单元序中连续");
      const seed = attempts.get(c.seed_attempt_id);
      needE(seed !== undefined && seed.result === "FORMED" && equal(seed.units.map(u => u.unit_id), c.seed_units) && seed.center_id === c.entity_id, "SEED_LINK", "中枢须由同 cut 成立的 seed 尝试支撑");
      const upgrade = c.status === "UPGRADE_PENDING_TB02H";
      for (const m of c.members) {
        const a = assignments.get(m);
        needE(!(a !== undefined && a.status === "CONNECTOR"), "CONNECTOR_AS_MEMBER", "连接段被列为中枢成员");
        const want = upgrade ? ["NOT_YET_ASSIGNED", "UPGRADE_PENDING_TB02H"] : ["CENTER_MEMBER", null];
        needE(a !== undefined && a.status === want[0] && a.reason === want[1] && (upgrade || a.center_id === c.entity_id), "ASSIGNMENT_CONSISTENCY", "成员归属与中枢成员表不一致");
        memberOf.set(m, c.entity_id);
      }
      const units = c.members.map(m => seg.get(m));
      const zn = c.members.filter((m, i) => units[i][4] === units[0][4]);
      needE(equal(c.zn_members, zn) && c.zn_direction === units[0][4], "ZN", "Zn 成员须是与首单元同向的全部成员");
      if (c.settled) {
        const nxt = first + c.members.length;
        needE(nxt < completed.length && completed[nxt][0] === c.break_unit, "BREAK_UNIT", "突破段须是末成员后的下一个完成单元");
        const seedUnits = c.members.slice(0, 3).map(m => seg.get(m));
        const zd = seedUnits.map(u => u[2]).reduce((a, b) => (b > a ? b : a)), zg = seedUnits.map(u => u[3]).reduce((a, b) => (b < a ? b : a));
        const b = completed[nxt];
        const up = b[2] > zg ? true : b[3] < zd ? false : b[3] > zg;
        needE(c.break_up === up, "BREAK_UP", "break_up 与突破段方向不符（C4）");
      }
      const rd = roles.get(c.entity_id);
      needE(rd !== undefined && rd.entity_id === c.roles_id, "ROLES_LINK", "中枢缺同 cut 角色对象");
      const allowed = {ZD: c.members.slice(0, 3), ZG: c.members.slice(0, 3), DD: zn, GG: zn, D: zn, G: zn};
      for (const [role, names] of ROLE_LAYOUT) for (const name of names) {
        const v = rd.roles[role][name];
        if (v.value === null) { needE(upgrade, "ROLE_NULL", "非 ≥9 中枢的动态变量不得为 null"); continue; }
        // #1424 C1：待升级窗口不能用真实 Zn 极值伪装成已获准的动态角色。
        if (role !== "CORE") needE(!upgrade, "ROLE_NULL", "≥9 待升级中枢的动态变量必须为 null");
        const [attr, pick] = ROLE_RULE[name];
        const values = allowed[name].map(m => seg.get(m)[attr]);
        const want = values.reduce((a, b) => (pick === "max" ? (b > a ? b : a) : (b < a ? b : a)));
        needE(BigInt(v.value) === want, "ROLE_VALUE", name + " 值不是允许成员集上的极值（D/G 不得冒充核心）");
        needE(equal(v.contributors, allowed[name].filter(m => seg.get(m)[attr] === want)), "CONTRIBUTORS", name + " 贡献成员缺失或不在允许集");
      }
    }
    for (let i = 0; i + 1 < centers.length; i++) {
      const a = centers[i], b = centers[i + 1];
      const aLast = pos.get(a.members[a.members.length - 1]), bFirst = pos.get(b.members[0]);
      needE(bFirst !== aLast + 1, "PLUS_ONE_REUSE", "下一中枢首成员是上一中枢末成员的下一个单元（+1 复用）");
      needE(bFirst > aLast + 1 && a.settled, "RESUME_ANCHOR", "下一中枢首成员早于续扫锚或上一中枢未结算");
      for (let k = aLast + 1; k < bFirst; k++) {
        const x = assignments.get(completed[k][0]);
        needE(x.status === "CONNECTOR" && x.left_center_id === a.entity_id && x.right_center_id === b.entity_id, "CONNECTOR", "两中枢之间的单元须全是 CONNECTOR(A→B)");
      }
      for (let k = aLast + 2; k < bFirst; k++) {
        const t = attempts.get("seed_attempt:" + unitAnchor(completed[k][0]));
        needE(t !== undefined && t.result === "EXCLUDED", "RESUME_GAP", "续扫锚到下一中枢之间的单元须有不成立的 seed 尝试");
      }
    }
    const upgradeMembers = new Set(centers.filter(c => c.status === "UPGRADE_PENDING_TB02H").flatMap(c => c.members));
    for (const [uid, a] of assignments) {
      if (a.status === "CENTER_MEMBER") needE(memberOf.get(uid) === a.center_id && !upgradeMembers.has(uid), "ASSIGNMENT_CONSISTENCY", "CENTER_MEMBER 指向的中枢不含该单元");
      if (a.status === "CONNECTOR") needE(!memberOf.has(uid), "CONNECTOR_AS_MEMBER", "连接段被列为中枢成员");
      if (a.reason === "UPGRADE_PENDING_TB02H") needE(upgradeMembers.has(uid), "ASSIGNMENT_CONSISTENCY", "UPGRADE_PENDING_TB02H 归属的单元不属任何 UPGRADE 中枢");
    }
    const byCenter = new Map(centers.map(c => [c.entity_id, c])), edges = new Map();
    for (const r of byKind[CENTER_KINDS[4]]) {
      const d = r.payload.data;
      if (d.relation_kind !== "member_of") continue;
      const target = byCenter.get(d.target_id);
      needE(target !== undefined && target.status !== "UPGRADE_PENDING_TB02H" && target.members.includes(d.source_id), "MEMBER_EDGE", "member_of 只能从非 UPGRADE 中枢的成员指向该中枢");
      const key = JSON.stringify([d.source_id, d.target_id]);
      edges.set(key, (edges.get(key) ?? 0) + 1);
    }
    // schema v1.2 §6 E4：非 UPGRADE 中枢的每个成员恰有一条指向该中枢的 member_of 边（缺边、重边都拒）。
    for (const c of centers) {
      if (c.status === "UPGRADE_PENDING_TB02H") continue;
      for (const m of c.members) needE((edges.get(JSON.stringify([m, c.entity_id])) ?? 0) === 1, "MEMBER_EDGE_COUNT", "非 UPGRADE 中枢的每个成员须恰有一条指向该中枢的 member_of 边（E4）");
    }
    validateScanCursor(byKind[CENTER_KINDS[6]], completed, pending, pos, centers, attempts);
    if (axes !== null && Object.hasOwn(axes, "CC-016")) {
      const ids = kinds => records.filter(r => kinds.includes(r.kind)).map(r => r.object_id).sort();
      for (const [axis, kinds] of [["CC-016", CENTER_KINDS.slice(0, 2)], ["CC-017", CENTER_KINDS.slice(2, 3)], ["CC-022", CENTER_KINDS.slice(3, 4)]])
        needE(equal([...axes[axis].evidence.object_ids].sort(), ids(kinds)), "AXIS_BINDING", axis + " 目录证据未绑定实际中枢对象");
    }
  }
  // ── #1424 TB-02-F 相邻中枢五叶关系（CC-020）与延伸测试单元位置（CC-018），schema s-center-pair/1。与 s_tb02_contract.py
  // 同序同原因码（TB02F_<CODE>），test_tb02f_contract 对共享语料逐条对拍两侧结论与原因码。根裁定 #1424 issuecomment-6160553615。
  const PAIR_TUPLE = ["s-center-pair/1", "center-pair/1", "zn_outer_five_leaf_pending_ge9", "main"];
  const TB02F_DELIVERED = ["l0_unit_core_position_five_leaves", "l0_adjacent_pair_five_leaves_zn_outer_with_full_matrix", "expansion_relation_separate_from_realization"];
  const TB02F_DEFERRED = ["upper_level_pairs_1429", "ge9_pairs_pending_1429", "upper_center_realization_1429", "trend_realization_1426", "first_retest_destruction_1434", "two_step_settlement_r5_1496"];
  const PAIR_LEAVES = ["CORE_NOT_STRICTLY_SEPARATED", "UP_NEWBIRTH", "DOWN_NEWBIRTH", "UP_EXPANSION", "DOWN_EXPANSION"];
  const UNIT_POSITIONS = ["STRICT_BELOW", "LOW_TOUCH", "INTERIOR_CONTACT", "HIGH_TOUCH", "STRICT_ABOVE"];
  const TESTED_ROLES = ["EXTENSION_MEMBER", "BREAK_UNIT", "UPGRADE_PENDING_MEMBER"];
  const PAIR_ENDPOINTS = ["DD", "ZD", "ZG", "GG"];
  const PAIR_PENDING = "PENDING_UPGRADE_TB02H";
  const PAIR_REALIZATION = {upper_center: "NOT_CONSTRUCTED_DEFERRED_1429", trend: "NOT_ASSERTED_DEFERRED_1426"};
  const PAIR_DESTRUCTION = "NOT_EVALUATED_DEFERRED_1434";
  const PAIR_CHANGES = ["formed", "leaf_revised", "pending_upgrade", "inputs_revised", "withdrawn"];
  const isPairFamily = kind => ["CC-018.", "CC-020."].some(p => kind.startsWith(p));
  function needF(ok, code, message) { if (!ok) throw new Error("TB02F_" + code + ": " + message); }
  function keysF(value, expected, name) {
    needF(object(value) && Object.keys(value).length === expected.length && expected.every(k => Object.hasOwn(value, k)), "SHAPE", name + " 字段缺失/多余");
  }
  function intF(value, name, minimum = null) {
    needF(typeof value === "string" && value.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value), "SHAPE", name + " 必须是规范 i64 文本");
    const n = BigInt(value);
    needF(n >= -9223372036854775808n && n <= 9223372036854775807n && (minimum === null || n >= BigInt(minimum)), "SHAPE", name + " 超出整数域");
    return n;
  }
  function textF(value, name) { needF(typeof value === "string" && value !== "", "SHAPE", name + " 必须是非空文本"); return value; }
  function knownF(k) {
    keysF(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "known_at");
    intF(k.generation, "generation", 1); intF(k.input_frontier, "input_frontier", -1);
    for (const name of ["receipt_id", "received_at"]) if (k[name] !== null) textF(k[name], name);
    if (k.semantic_commit_ns !== null) intF(k.semantic_commit_ns, "semantic_commit_ns", 0);
  }
  function anchorF(prefix, value) {
    needF(typeof value === "string" && new RegExp("^" + prefix + ":[1-9][0-9]*:(?:0|[1-9][0-9]*)$").test(value), "IDENTITY", "身份须为 " + prefix + ":<代>:<锚>");
    return value.slice(prefix.length + 1);
  }
  function pairLeaves(a, b) {
    const sep = b.ZG < a.ZD || b.ZD > a.ZG;
    return [["CORE_NOT_STRICTLY_SEPARATED", !sep], ["UP_NEWBIRTH", b.DD > a.GG], ["DOWN_NEWBIRTH", b.GG < a.DD],
      ["UP_EXPANSION", b.ZD > a.ZG && b.DD <= a.GG], ["DOWN_EXPANSION", b.ZG < a.ZD && b.GG >= a.DD]].filter(([, h]) => h).map(([leaf]) => leaf);
  }
  const unitPosition = (lo, hi, zd, zg) => (hi < zd ? "STRICT_BELOW" : hi === zd ? "LOW_TOUCH" : lo > zg ? "STRICT_ABOVE" : lo === zg ? "HIGH_TOUCH" : "INTERIOR_CONTACT");
  const cmpStr = (x, y) => (x < y ? "<" : x === y ? "=" : ">");
  function containment(alo, ahi, blo, bhi) {
    const aHasB = alo <= blo && bhi <= ahi, bHasA = blo <= alo && ahi <= bhi;
    return aHasB && bHasA ? "EQUAL" : aHasB ? "A_CONTAINS_B" : bHasA ? "B_CONTAINS_A" : "NONE";
  }
  function pairSide(v, name) {
    keysF(v, PAIR_ENDPOINTS, name);
    const zd = intF(v.ZD, name + ".ZD"), zg = intF(v.ZG, name + ".ZG");
    needF(zd < zg, "DOMAIN", name + " 核心须 ZD<ZG");
    needF((v.DD === null) === (v.GG === null), "SHAPE", name + " 外缘须同时有值或同时为 null");
    if (v.DD === null) return {ZD: zd, ZG: zg, DD: null, GG: null};
    const side = {ZD: zd, ZG: zg, DD: intF(v.DD, name + ".DD"), GG: intF(v.GG, name + ".GG")};
    needF(side.DD <= zd && zg <= side.GG, "DOMAIN", name + " 须 DD≤ZD<ZG≤GG");
    return side;
  }
  function validatePairPayload(kind, p) {
    keysF(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "中枢对合同");
    needF(equal([p.schema_revision, p.semantic_version, p.policy_version, p.view_role], PAIR_TUPLE), "VERSION", "中枢对规则/范围版本不符");
    intF(p.object_generation, "object_generation", 1);
    const d = p.data;
    if (kind === PAIR_KINDS[0]) {
      keysF(d, ["entity_id", "center_id", "center_revision", "unit", "core", "position", "closed_overlap", "strict_leave", "leave_side", "construction_role", "destruction", "known_at"], kind);
      const ca = anchorF("center", d.center_id), u = d.unit;
      keysF(u, ["unit_id", "unit_object_id", "direction", "lo", "hi"], "position.unit");
      const ua = anchorF("segment", u.unit_id);
      textF(u.unit_object_id, "unit_object_id");
      needF(["UP", "DOWN"].includes(u.direction), "SHAPE", "unit.direction 未声明");
      const lo = intF(u.lo, "unit.lo"), hi = intF(u.hi, "unit.hi");
      needF(lo < hi, "SHAPE", "单元区间须 lo<hi");
      needF(d.entity_id === "ucp:" + ca + ":" + ua, "IDENTITY", "位置身份未绑定中枢与单元");
      intF(d.center_revision, "center_revision", 1);
      keysF(d.core, ["ZD", "ZG"], "core");
      const zd = intF(d.core.ZD, "core.ZD"), zg = intF(d.core.ZG, "core.ZG");
      needF(zd < zg, "CORE", "冻结核心须 ZD<ZG");
      const pos = unitPosition(lo, hi, zd, zg);
      needF(UNIT_POSITIONS.includes(d.position) && d.position === pos, "POSITION", "位置五叶与单元区间、冻结核心不符");
      const closed = ["LOW_TOUCH", "INTERIOR_CONTACT", "HIGH_TOUCH"].includes(pos);
      const side = pos === "STRICT_BELOW" ? "DOWN" : pos === "STRICT_ABOVE" ? "UP" : null;
      needF(d.closed_overlap === closed && d.strict_leave === !closed && d.leave_side === side, "POSITION", "重叠/离开位与位置不符（离开侧只按区间位置判）");
      needF(TESTED_ROLES.includes(d.construction_role), "SHAPE", "构造角色未声明");
      needF((d.construction_role === "BREAK_UNIT") === !closed, "ROLE", "突破段恰为严格离开的测试单元");
      needF(d.destruction === PAIR_DESTRUCTION, "DESTRUCTION", "离开不是破坏：首回试与破坏归 #1434");
      knownF(d.known_at);
    } else if (kind === PAIR_KINDS[1]) {
      keysF(d, ["entity_id", "entity_revision", "a_center_id", "a_revision", "b_center_id", "b_revision", "connector_units", "status", "leaf", "direction", "provisional", "inputs", "matrix", "witnesses", "realization", "null_reason", "previous", "formed_known_at", "known_at"], kind);
      const aa = anchorF("center", d.a_center_id), ba = anchorF("center", d.b_center_id);
      needF(d.entity_id === "pair:" + aa + ":" + ba, "IDENTITY", "中枢对身份未绑定两中枢");
      const revision = intF(d.entity_revision, "entity_revision", 1);
      intF(d.a_revision, "a_revision", 1); intF(d.b_revision, "b_revision", 1);
      const connectors = d.connector_units;
      needF(Array.isArray(connectors) && connectors.length >= 1, "CONNECTORS", "两中枢之间至少有突破段这一连接段");
      for (const c of connectors) anchorF("segment", c);
      needF(new Set(connectors).size === connectors.length, "CONNECTORS", "两中枢之间至少有突破段这一连接段");
      needF(typeof d.provisional === "boolean", "SHAPE", "provisional 错型");
      needF(equal(d.realization, PAIR_REALIZATION), "REALIZATION", "关系成立不等于上级对象已构造/趋势已实现（不得以 level_lift 类布尔代替）");
      needF(["EVALUATED", PAIR_PENDING].includes(d.status), "SHAPE", "中枢对状态未声明");
      keysF(d.inputs, ["A", "B"], "inputs");
      const a = pairSide(d.inputs.A, "inputs.A"), b = pairSide(d.inputs.B, "inputs.B");
      if (d.status === "EVALUATED") {
        needF(a.DD !== null && b.DD !== null && d.null_reason === null, "PENDING", "可判对须两端外缘齐全且无 null 原因");
        const leaves = pairLeaves(a, b);
        needF(leaves.length === 1, "DOMAIN", "五叶不恰一（域前件被破坏）");
        needF(d.leaf === leaves[0], "LEAF", "叶与输入端点按定义重算不符");
        const wantDir = leaves[0].startsWith("UP_") ? "UP" : leaves[0].startsWith("DOWN_") ? "DOWN" : null;
        needF(d.direction === wantDir, "DIRECTION", "新生/扩展方向与叶不符");
        const cells = PAIR_ENDPOINTS.flatMap(x => PAIR_ENDPOINTS.map(y => [x, y]));
        keysF(d.matrix, cells.map(([x, y]) => `B.${x}?A.${y}`), "matrix");
        needF(cells.every(([x, y]) => d.matrix[`B.${x}?A.${y}`] === cmpStr(b[x], a[y])), "MATRIX", "端点矩阵与输入不符");
        keysF(d.witnesses, ["core_tangent", "outer_touch", "outer_containment", "core_containment"], "witnesses");
        needF(typeof d.witnesses.core_tangent === "boolean" && typeof d.witnesses.outer_touch === "boolean", "WITNESS", "相切/触及证据必须是布尔值");
        const want = {core_tangent: b.ZD === a.ZG || b.ZG === a.ZD, outer_touch: b.DD === a.GG || b.GG === a.DD,
          outer_containment: containment(a.DD, a.GG, b.DD, b.GG), core_containment: containment(a.ZD, a.ZG, b.ZD, b.ZG)};
        needF(equal(d.witnesses, want), "WITNESS", "相切/触及/包含证据与输入不符");
      } else {
        needF(d.leaf === null && d.direction === null && d.matrix === null && d.witnesses === null && d.null_reason === PAIR_PENDING, "PENDING", "待定对的叶/方向/矩阵/证据须为 null 并带原因");
        needF(a.DD === null || b.DD === null, "PENDING", "待定对须至少一端外缘为 null（≥9 窗口）");
      }
      if (revision === 1n) needF(d.previous === null, "REVISION", "首修订无前值");
      else {
        keysF(d.previous, ["entity_revision", "status", "leaf"], "previous");
        needF(intF(d.previous.entity_revision, "previous.entity_revision", 1) === revision - 1n, "REVISION", "previous 须指上一修订");
        needF(["EVALUATED", PAIR_PENDING].includes(d.previous.status) && (d.previous.leaf === null || PAIR_LEAVES.includes(d.previous.leaf)), "SHAPE", "previous 状态/叶未声明");
      }
      knownF(d.formed_known_at); knownF(d.known_at);
    } else if (kind === PAIR_KINDS[2]) {
      keysF(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], kind);
      for (const name of ["source_id", "target_id", "witness_object_id"]) textF(d[name], name);
      needF(["pair_from", "pair_to", "position_of", "position_against"].includes(d.relation_kind), "SHAPE", "未声明中枢对关系");
      intF(d.version, "version", 1);
    } else if (kind === PAIR_KINDS[3]) {
      keysF(d, ["entity_id", "change", "before", "after", "known_at", "object_id"], kind);
      textF(d.entity_id, "entity_id");
      needF(PAIR_CHANGES.includes(d.change), "SHAPE", "未声明中枢对变化");
      const withdrawn = d.change === "withdrawn";
      needF((d.after === null) === withdrawn && (d.object_id === null) === withdrawn && (d.before === null) === (d.change === "formed"), "SHAPE", "变化前后件与变化种类不符");
      if (d.object_id !== null) textF(d.object_id, "object_id");
      knownF(d.known_at);
      validatePairChange(d, p);
    } else if (kind === PAIR_KINDS[4]) {
      keysF(d, ["input_frontier", "known_at", "pair_count", "pending_pairs", "waiting_reasons", "delivered_scope", "deferred_scope"], kind);
      intF(d.input_frontier, "input_frontier", -1); knownF(d.known_at);
      const count = intF(d.pair_count, "pair_count", 0), pending = intF(d.pending_pairs, "pending_pairs", 0);
      needF(pending <= count && equal(d.waiting_reasons, pending > 0n ? [PAIR_PENDING] : []), "VERSION_COUNT", "待定对计数与等待原因不符");
      needF(d.delivered_scope === "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position" && equal(d.deferred_scope, TB02F_DEFERRED), "SCOPE", "中枢对交付范围不符");
    } else needF(false, "CANDIDATE_KIND", "未声明的中枢对 kind");
    const slot = kind === PAIR_KINDS[2] ? [d.source_id, d.relation_kind, d.target_id] : kind === PAIR_KINDS[3] ? [d.known_at.generation, d.entity_id, d.change] : kind === PAIR_KINDS[4] ? "TB-02-F" : d.entity_id;
    needF(equal(p.slot, slot), "SLOT", "中枢对语义槽身份不符");
  }
  function pairFact(d) {
    return Object.fromEntries(Object.entries(d).filter(([k]) => !["entity_revision", "previous", "formed_known_at", "known_at"].includes(k)));
  }
  function validatePairChange(d, p) {
    // #1424 H2：内在变化链与真实前 cut 的绑定分别核验。
    const before = d.before, after = d.after;
    for (const value of [before, after]) if (value !== null) {
      needF(object(value) && value.entity_id === d.entity_id, "CHANGE", "变化前后件须属于同一中枢对");
      validatePairPayload(PAIR_KINDS[1], {...p, slot: d.entity_id, data: value});
    }
    if (after !== null) needF(equal(d.known_at, after.known_at), "CHANGE", "变化获知时点须等于后件");
    if (before === null) {
      needF(after.entity_revision === "1" && equal(after.formed_known_at, after.known_at), "CHANGE", "首次形成须为修订 1 且形成时点等于获知时点");
    } else if (after === null) {
      needF(BigInt(d.known_at.generation) > BigInt(before.known_at.generation), "CHANGE", "撤回获知代须晚于前件");
    } else {
      const previous = Object.fromEntries(["entity_revision", "status", "leaf"].map(k => [k, before[k]]));
      needF(BigInt(after.entity_revision) === BigInt(before.entity_revision) + 1n && equal(after.previous, previous), "CHANGE", "修订须递增一且 previous 绑定前件");
      needF(equal(after.formed_known_at, before.formed_known_at) && BigInt(after.known_at.generation) > BigInt(before.known_at.generation), "CHANGE", "修订须保持形成时点且获知代递增");
      needF(!equal(pairFact(before), pairFact(after)), "CHANGE", "事实未变不得发修订事件");
      const change = before.status !== after.status && after.status === PAIR_PENDING ? "pending_upgrade" : before.leaf !== after.leaf ? "leaf_revised" : "inputs_revised";
      needF(d.change === change, "CHANGE", "变化种类不符合状态、叶、输入判序");
    }
  }
  function validatePairPublication(record, generation) {
    intF(generation, "publication_generation", 1);
    needF(record.payload.data.known_at.generation === generation, "PUBLICATION", "pair/change 获知代须等于实际发布代");
  }
  function validatePairTransition(previousRecords, records, eventRecords = null, generation = null) {
    const pairs = rs => {
      const result = new Map();
      for (const r of rs) if (r.kind === PAIR_KINDS[1]) {
        const eid = r.payload.data.entity_id;
        needF(!result.has(eid), "CHANGE", "相邻 cut 的中枢对身份不得重复");
        result.set(eid, r);
      }
      return result;
    };
    const before = pairs(previousRecords), after = pairs(records), events = new Map();
    for (const r of eventRecords === null ? records : eventRecords) {
      if (r.kind !== PAIR_KINDS[3]) continue;
      validatePairPayload(r.kind, r.payload);
      if (Object.hasOwn(r, "first_known_generation")) validatePairPublication(r, r.first_known_generation);
      if (generation !== null) validatePairPublication(r, generation);
      const d = r.payload.data;
      needF(!events.has(d.entity_id), "CHANGE", "同一中枢对本代只能有一个变化事件");
      events.set(d.entity_id, d);
    }
    const changed = [...new Set([...before.keys(), ...after.keys()])].filter(eid =>
      before.get(eid)?.object_id !== after.get(eid)?.object_id || !equal(before.get(eid)?.payload ?? null, after.get(eid)?.payload ?? null));
    needF(events.size === changed.length && changed.every(eid => events.has(eid)), "CHANGE", "本代变化事件须恰覆盖真实出生、修订和撤回");
    for (const eid of changed) {
      const old = before.get(eid), current = after.get(eid), d = events.get(eid);
      if (current !== undefined) {
        if (Object.hasOwn(current, "first_known_generation")) validatePairPublication(current, current.first_known_generation);
        if (generation !== null) validatePairPublication(current, generation);
      }
      needF(equal(d.before, old?.payload.data ?? null) && equal(d.after, current?.payload.data ?? null), "CHANGE", "变化前后件须绑定相邻 cut 的真实中枢对");
      needF(d.object_id === (current?.object_id ?? null), "CHANGE", "变化 object_id 须绑定实际后件对象");
    }
  }
  function validatePairHistory(records) {
    // #1424 M1：只核 births/deaths/events 影响的实体；稳定活动对不随每个历史代重扫。
    const generations = new Map(), activeByEntity = new Map(), seenObjects = new Set();
    const at = generation => {
      intF(generation, "first_known_generation", 1);
      if (!generations.has(generation)) generations.set(generation, {births: [], deaths: [], events: []});
      return generations.get(generation);
    };
    for (const r of records) {
      if (r.kind !== PAIR_KINDS[1] && r.kind !== PAIR_KINDS[3]) continue;
      validatePairPayload(r.kind, r.payload);
      validatePairPublication(r, r.first_known_generation);
      if (r.kind === PAIR_KINDS[3]) at(r.first_known_generation).events.push(r);
      else {
        needF(!seenObjects.has(r.object_id), "CHANGE", "历史中枢对对象身份不得重复");
        seenObjects.add(r.object_id);
        at(r.first_known_generation).births.push(r);
        if (r.withdrawn_generation !== null) at(r.withdrawn_generation).deaths.push(r);
      }
    }
    for (const generation of [...generations.keys()].sort((a,b) => BigInt(a) < BigInt(b) ? -1 : BigInt(a) > BigInt(b) ? 1 : 0)) {
      const group = generations.get(generation);
      const affected = new Set([...group.births, ...group.deaths, ...group.events].map(r => r.payload.data.entity_id));
      const before = [];
      for (const eid of affected) if (activeByEntity.has(eid)) before.push(activeByEntity.get(eid));
      for (const r of group.deaths) {
        const eid = r.payload.data.entity_id;
        needF(activeByEntity.get(eid)?.object_id === r.object_id, "CHANGE", "历史撤回缺少活动前件");
        activeByEntity.delete(eid);
      }
      for (const r of group.births) {
        const eid = r.payload.data.entity_id;
        needF(!activeByEntity.has(eid), "CHANGE", "相邻 cut 的中枢对身份不得重复");
        activeByEntity.set(eid, r);
      }
      const after = [];
      for (const eid of affected) if (activeByEntity.has(eid)) after.push(activeByEntity.get(eid));
      validatePairTransition(before, after, group.events, generation);
    }
  }
  function validatePairLinks(records, axes = null) {
    const byKind = Object.fromEntries(PAIR_KINDS.map(k => [k, records.filter(r => r.kind === k)]));
    for (const r of records) needF(!isPairFamily(r.kind) || PAIR_KINDS.includes(r.kind), "CANDIDATE_KIND", "未声明的中枢对 kind");
    if (!Object.values(byKind).some(v => v.length)) return;
    const [completed] = segmentUnits(records);
    const pos = new Map(completed.map((u, i) => [u[0], i])), seg = new Map(completed.map(u => [u[0], u]));
    const centers = records.filter(r => r.kind === "CC-016.center").map(r => r.payload.data).sort((x, y) => (pos.get(x.members[0]) ?? -1) - (pos.get(y.members[0]) ?? -1));
    const roles = new Map(records.filter(r => r.kind === "CC-017.center_roles").map(r => [r.payload.data.center_id, r.payload.data.roles]));
    const upgrade = new Map(centers.map(c => [c.entity_id, c.status === "UPGRADE_PENDING_TB02H"]));
    const roleValue = (cid, role, name) => { const v = roles.get(cid)[role][name].value; return v === null ? null : BigInt(v); };
    const wantTested = new Map();
    for (const c of centers) {
      needF(roles.has(c.entity_id), "CORE", "中枢缺同 cut 角色对象");
      for (const m of c.members.slice(3)) wantTested.set(JSON.stringify([c.entity_id, m]), upgrade.get(c.entity_id) ? "UPGRADE_PENDING_MEMBER" : "EXTENSION_MEMBER");
      if (c.settled) wantTested.set(JSON.stringify([c.entity_id, c.break_unit]), "BREAK_UNIT");
    }
    const byCenter = new Map(centers.map(c => [c.entity_id, c])), ucps = new Map();
    for (const r of byKind[PAIR_KINDS[0]]) {
      const d = r.payload.data, key = JSON.stringify([d.center_id, d.unit.unit_id]);
      needF(!ucps.has(key), "UCP_COUNT", "同一测试单元重复发位置");
      ucps.set(key, d);
    }
    needF(ucps.size === wantTested.size && [...ucps.keys()].every(k => wantTested.has(k)), "UCP_COUNT", "位置对象须恰覆盖每个中枢的延伸测试单元（seed 之后的成员 + 突破段）");
    for (const [key, d] of ucps) {
      const [cid, uid] = JSON.parse(key), u = d.unit, s = seg.get(uid);
      needF(s !== undefined && u.unit_object_id === s[1] && BigInt(u.lo) === s[2] && BigInt(u.hi) === s[3] && u.direction === s[4], "UNIT_SOURCE", "位置单元与同 cut CC-013 不符");
      needF(BigInt(d.core.ZD) === roleValue(cid, "CORE", "ZD") && BigInt(d.core.ZG) === roleValue(cid, "CORE", "ZG"), "CORE", "位置所对核心与中枢角色不符");
      needF(d.construction_role === wantTested.get(key), "ROLE", "构造角色与中枢成员/突破段不符");
      needF(BigInt(d.center_revision) <= BigInt(byCenter.get(cid).entity_revision), "REVISION", "位置的核心版本晚于中枢当前修订");
    }
    const pairs = new Map();
    for (const r of byKind[PAIR_KINDS[1]]) {
      const d = r.payload.data, key = JSON.stringify([d.a_center_id, d.b_center_id]);
      needF(!pairs.has(key), "PAIR_COUNT", "同一中枢对重复");
      pairs.set(key, d);
    }
    const adjacent = centers.slice(1).map((b, i) => JSON.stringify([centers[i].entity_id, b.entity_id]));
    needF([...pairs.keys()].every(k => adjacent.includes(k)), "ADJACENCY", "中枢对的两端不是扫描相邻中枢");
    needF(pairs.size === adjacent.length, "PAIR_COUNT", "中枢数 n 时须恰 n−1 个对（缺对或多对）");
    for (let i = 0; i + 1 < centers.length; i++) {
      const a = centers[i], b = centers[i + 1], d = pairs.get(JSON.stringify([a.entity_id, b.entity_id]));
      needF(d.a_revision === a.entity_revision && d.b_revision === b.entity_revision, "REVISION", "对所引中枢修订号与中枢当前修订不符");
      const between = completed.slice(pos.get(a.members[a.members.length - 1]) + 1, pos.get(b.members[0])).map(u => u[0]);
      needF(equal(d.connector_units, between), "CONNECTORS", "连接段须恰为两中枢之间的全部完成单元");
      for (const [side, c] of [["A", a], ["B", b]]) {
        const want = {ZD: roleValue(c.entity_id, "CORE", "ZD"), ZG: roleValue(c.entity_id, "CORE", "ZG"), DD: roleValue(c.entity_id, "OUTER", "DD"), GG: roleValue(c.entity_id, "OUTER", "GG")};
        const got = Object.fromEntries(Object.entries(d.inputs[side]).map(([k, v]) => [k, v === null ? null : BigInt(v)]));
        needF(PAIR_ENDPOINTS.every(k => got[k] === want[k]) && Object.keys(got).length === 4, "PAIR_INPUTS", "对的输入须等于同 cut 中枢角色（Zn 外缘，R26；不照抄塔的过渡态子中枢关系）");
      }
      // #1424 C1：须有完整 upgrade/null 双向门，再由 PAIR_INPUTS 与单对象 PENDING 门合出「≥9 ⟺ 待定」。
      needF(d.provisional === !b.settled, "PROVISIONAL", "后中枢未结算 ⟺ 关系暂定");
    }
    const sourceObjects = new Map([...byKind[PAIR_KINDS[0]], ...byKind[PAIR_KINDS[1]]].map(r => [r.payload.data.entity_id, r.object_id]));
    const wantEdges = new Set();
    for (const d of pairs.values()) for (const [k, t] of [["pair_from", d.a_center_id], ["pair_to", d.b_center_id]]) wantEdges.add(JSON.stringify([d.entity_id, k, t, d.entity_revision, sourceObjects.get(d.entity_id)]));
    for (const d of ucps.values()) {
      wantEdges.add(JSON.stringify([d.entity_id, "position_of", d.unit.unit_id, d.center_revision, sourceObjects.get(d.entity_id)]));
      wantEdges.add(JSON.stringify([d.entity_id, "position_against", d.center_id, d.center_revision, sourceObjects.get(d.entity_id)]));
    }
    const gotEdges = byKind[PAIR_KINDS[2]].map(r => { const d = r.payload.data; return JSON.stringify([d.source_id, d.relation_kind, d.target_id, d.version, d.witness_object_id]); });
    needF(new Set(gotEdges).size === gotEdges.length && gotEdges.length === wantEdges.size && gotEdges.every(e => wantEdges.has(e)), "EDGE", "中枢对/位置关系边缺失、多余或版本/见证对象不符");
    const current = new Map([...pairs.values()].map(d => [d.entity_id, d]));
    for (const r of byKind[PAIR_KINDS[3]]) {
      const d = r.payload.data;
      if (d.change === "withdrawn") needF(!current.has(d.entity_id), "CHANGE", "撤回的中枢对仍在当前态");
      else {
        needF(current.has(d.entity_id) && equal(current.get(d.entity_id), d.after), "CHANGE", "变化事件的后件须是当前中枢对");
        needF(d.object_id === sourceObjects.get(d.entity_id), "CHANGE", "变化 object_id 须绑定实际后件对象");
      }
    }
    const versions = byKind[PAIR_KINDS[4]];
    needF(versions.length === 1, "VERSION_COUNT", "每 cut 恰一个对版本对象");
    const v = versions[0].payload.data;
    needF(BigInt(v.pair_count) === BigInt(pairs.size) && BigInt(v.pending_pairs) === BigInt([...pairs.values()].filter(d => d.status === PAIR_PENDING).length), "VERSION_COUNT", "版本对象的对计数与实际不符");
    if (axes !== null && Object.hasOwn(axes, "CC-018")) {
      const ids = kind => records.filter(r => r.kind === kind).map(r => r.object_id).sort();
      for (const [axis, kind] of [["CC-018", PAIR_KINDS[0]], ["CC-020", PAIR_KINDS[1]]])
        needF(equal([...axes[axis].evidence.object_ids].sort(), ids(kind)), "AXIS_BINDING", axis + " 目录证据未绑定实际对象");
    }
  }
  // 旧 validatePairRule 的计数分支：中枢规则 cut 有中枢版本对象时须恰一个对版本对象（规则分支见文件头）。
  function validatePairVersionCount(records) {
    if (records.some(r => r.kind === CENTER_KINDS[6]))
      needF(records.filter(r => r.kind === PAIR_KINDS[4]).length === 1, "VERSION_COUNT", "中枢对规则 cut 有中枢版本对象时须恰一个对版本对象");
  }
  const AXIS_KINDS = [["CC-016", CENTER_KINDS.slice(0, 2), needE], ["CC-017", CENTER_KINDS.slice(2, 3), needE], ["CC-022", CENTER_KINDS.slice(3, 4), needE],
    ["CC-018", PAIR_KINDS.slice(0, 1), needF], ["CC-020", PAIR_KINDS.slice(1, 2), needF]];
  // 同 cut 活动集（含 CC-013.segment 普通原件）的完整中枢族链接：旧 validatePairCut 去掉规则分支后的同序组合。
  function validateCenterFamilyCut(records, axes = null) {
    for (const r of records) needE(!isCenterFamily(r.kind) || CENTER_KINDS.includes(r.kind), "CANDIDATE_KIND", "未声明的中枢 kind（不发 candidate）");
    for (const r of records) needF(!isPairFamily(r.kind) || PAIR_KINDS.includes(r.kind), "CANDIDATE_KIND", "未声明的中枢对 kind");
    validateCenterLinks(records, axes);
    validatePairVersionCount(records);
    validatePairLinks(records, axes);
    // 旧逐片链接只在有中枢族对象时才绑目录轴；22 轴 cut 无中枢对象时 CC-016…022 也须恰为空，不得挂他族对象。
    if (axes !== null && Object.hasOwn(axes, "CC-016"))
      for (const [axis, kinds, needX] of AXIS_KINDS) {
        const ids = records.filter(r => kinds.includes(r.kind)).map(r => r.object_id).sort();
        needX(equal([...axes[axis].evidence.object_ids].sort(), ids), "AXIS_BINDING", axis + " 目录证据未绑定实际中枢族对象");
      }
  }
  return Object.freeze({CENTER_KINDS: Object.freeze(CENTER_KINDS), PAIR_KINDS: Object.freeze(PAIR_KINDS), CENTER_TUPLE, PAIR_TUPLE,
    TB02E_DELIVERED, TB02E_DEFERRED, TB02F_DELIVERED, TB02F_DEFERRED,
    validateCenterPayload, validatePairPayload, validatePairPublication, validatePairHistory, validateCenterFamilyCut});
});
