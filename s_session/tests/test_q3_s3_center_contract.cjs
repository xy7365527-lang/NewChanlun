/* #1419/#1424：Q3 读 S3 结构写者（W_STRUCTURE_RULE_REVISION）的规则/22 轴/中枢族 typed 合同。
 * 夹具是按当前 Rust 构造规则（s_session_v3/content_structure.rs 中枢/对段、tb02a_facts catalog_axes）手工封装的最小 cut，
 * 非真实 Rust 产物；真实 S3 产物（fixtures/q3-real-s3-w-recover-revise.json）沿公开 Client 入口的验收见 test_q3_s3_real_cut.cjs，
 * 其第 1、2 条进 CI、第 3–5 条属重型验证窗口，运行命令与收据见该文件头注释。 */
"use strict";
const assert = require("node:assert/strict"),
  test = require("node:test");
const API = require("../browser/tb01c-client.js"),
  Q3 = API.Q3,
  typed = Q3.typed,
  C = typed.Center;
const W = "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1",
  BASE = W.slice(0, -"+raw-initialization-w/1".length);
const copy = (v) => JSON.parse(JSON.stringify(v));
const K = { generation: "1", input_frontier: "10", receipt_id: "r1", received_at: "t1", semantic_commit_ns: null };
const U = ["segment:1:0", "segment:1:3", "segment:1:6", "segment:1:9"],
  RANGE = [["UP", "10", "20"], ["DOWN", "12", "18"], ["UP", "11", "22"], ["DOWN", "1", "5"]],
  CID = "center:1:0";
const unit = (k) => ({ unit_id: U[k], unit_object_id: "obj3-seg" + k, direction: RANGE[k][0], lo: RANGE[k][1], hi: RANGE[k][2] });
const wrap = (tuple, slot, data) => ({ schema_revision: tuple[0], semantic_version: tuple[1], policy_version: tuple[2], view_role: tuple[3], object_generation: "1", slot, data });
const e = (slot, data) => wrap(C.CENTER_TUPLE, slot, data),
  f = (slot, data) => wrap(C.PAIR_TUPLE, slot, data);
const role = (value, contributors, frozen) => ({ value, contributors, same_price_witness: contributors.length >= 2 ? contributors : [], frozen, null_reason: null });
function centerCut() {
  // 四个完成段：seed U0..U2 成立（ZD=12/ZG=18），U3 严格在下 ⟹ 结算、break_up=false，U3 为右侧待定连接段。
  const segments = U.slice(0, 4).map((id, k) => ({
    kind: "CC-013.segment",
    object_id: "obj3-seg" + k,
    payload: { data: { entity_id: id, result: "CASE_ONE", construction: { start_stroke: String(k), stroke_indices: [String(k)], direction: RANGE[k][0] },
      stroke_refs: [{ stroke_index: String(k), start_price: RANGE[k][0] === "UP" ? RANGE[k][1] : RANGE[k][2], end_price: RANGE[k][0] === "UP" ? RANGE[k][2] : RANGE[k][1] }] } },
  }));
  const attempt = { entity_id: "seed_attempt:1:0", units: [0, 1, 2].map(unit), vector: { completed: true, consecutive: true, same_level: true, alternating: true, strict: true },
    ZD: { value: "12", contributors: [U[1]], same_price_witness: [] }, ZG: { value: "18", contributors: [U[1]], same_price_witness: [] },
    core_order: "ZD<ZG", result: "FORMED", exclusion_reason: null, center_id: CID, known_at: K };
  const center = { entity_id: CID, entity_revision: "1", seed_attempt_id: "seed_attempt:1:0", seed_units: U.slice(0, 3), zn_direction: "UP", members: U.slice(0, 3),
    zn_members: [U[0], U[2]], status: "SETTLED", settled: true, break_unit: U[3], break_up: false, break_up_defined: true,
    resume_anchor: { after_unit_id: U[2], offset: "2" }, next_tested_unit: null, roles_id: "roles:" + CID, formed_known_at: K, settled_known_at: K };
  const roles = { entity_id: "roles:" + CID, entity_revision: "1", center_id: CID, previous: null, known_at: K,
    roles: { CORE: { ZD: role("12", [U[1]], true), ZG: role("18", [U[1]], true) }, OUTER: { DD: role("10", [U[0]], false), GG: role("22", [U[2]], false) },
      INNER_EXTREMA: { D: role("11", [U[2]], false), G: role("20", [U[0]], false) } } };
  const assign = (k, status, extra) => ({ entity_id: "assign:" + U[k], unit: unit(k), status, center_id: null, left_center_id: null, right_center_id: null,
    right_pending: false, reason: null, seed_from_self: k === 0 ? "FORMED" : "NOT_EVALUATED", known_at: K, ...extra });
  const ucp = { entity_id: "ucp:1:0:1:9", center_id: CID, unit: unit(3), core: { ZD: "12", ZG: "18" }, position: "STRICT_BELOW", closed_overlap: false,
    strict_leave: true, leave_side: "DOWN", construction_role: "BREAK_UNIT", destruction: "NOT_EVALUATED_DEFERRED_1434", center_revision: "1", known_at: K };
  const rel = (kind, s, r, t, v, w) => ({ kind, object_id: "obj3-rel-" + [s, r, t].join("|"), payload: (kind.includes("pair") ? f : e)([s, r, t], { source_id: s, relation_kind: r, target_id: t, version: v, witness_object_id: w }) });
  const records = [
    ...segments,
    { kind: C.CENTER_KINDS[0], object_id: "obj3-attempt", payload: e(attempt.entity_id, attempt) },
    { kind: C.CENTER_KINDS[1], object_id: "obj3-center", payload: e(CID, center) },
    { kind: C.CENTER_KINDS[2], object_id: "obj3-roles", payload: e(roles.entity_id, roles) },
    ...[0, 1, 2].map((k) => ({ kind: C.CENTER_KINDS[3], object_id: "obj3-assign" + k, payload: e("assign:" + U[k], assign(k, "CENTER_MEMBER", { center_id: CID })) })),
    { kind: C.CENTER_KINDS[3], object_id: "obj3-assign3", payload: e("assign:" + U[3], assign(3, "CONNECTOR", { left_center_id: CID, right_pending: true })) },
    rel(C.CENTER_KINDS[4], "seed_attempt:1:0", "attempted_from", U[0], "1", "obj3-attempt"),
    ...[0, 1, 2].map((k) => rel(C.CENTER_KINDS[4], U[k], "member_of", CID, "1", "obj3-center")),
    rel(C.CENTER_KINDS[4], U[3], "connects_left", CID, "1", "obj3-assign3"),
    { kind: C.CENTER_KINDS[5], object_id: "obj3-cchange", payload: e(["1", CID, "formed"], { entity_id: CID, change: "formed", before: null, after: center, known_at: K, object_id: "obj3-center" }) },
    { kind: C.CENTER_KINDS[6], object_id: "obj3-cver", payload: e("TB-02-E", { input_frontier: "10", known_at: K, waiting_reasons: [],
      scan_cursor: { state: "SEEKING_SEED", center_id: null, seed_start_unit_id: null, awaiting_unit_id: null, awaiting_after_unit_id: U[3], awaiting_offset: "3" },
      delivered_scope: "l0_strict_seed_zn_roles_c_resume_assignment", deferred_scope: C.TB02E_DEFERRED }) },
    { kind: C.PAIR_KINDS[0], object_id: "obj3-ucp", payload: f(ucp.entity_id, ucp) },
    rel(C.PAIR_KINDS[2], ucp.entity_id, "position_of", U[3], "1", "obj3-ucp"),
    rel(C.PAIR_KINDS[2], ucp.entity_id, "position_against", CID, "1", "obj3-ucp"),
    { kind: C.PAIR_KINDS[4], object_id: "obj3-pver", payload: f("TB-02-F", { input_frontier: "10", known_at: K, pair_count: "0", pending_pairs: "0", waiting_reasons: [],
      delivered_scope: "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position", deferred_scope: C.TB02F_DEFERRED }) },
  ];
  for (const r of records) Object.assign(r, { first_known_generation: "1", withdrawn_generation: null });
  return records;
}
// 两中枢：A=U0..U2（结算于 U3），B=U4..U6（EXTENDING）；U3 为 A→B 连接段；对 A→B 叶 DOWN_NEWBIRTH（B.GG=5<A.DD=10）。
function twoCenterCut() {
  U.push("segment:1:12", "segment:1:15", "segment:1:18");
  RANGE.push(["UP", "0", "4"], ["DOWN", "1", "3"], ["UP", "0", "5"]);
  try {
    const records = centerCut().filter((r) => ![C.CENTER_KINDS[6], C.PAIR_KINDS[4]].includes(r.kind));
    const BID = "center:1:12", A = { ZD: "12", ZG: "18", DD: "10", GG: "22" }, B = { ZD: "1", ZG: "3", DD: "0", GG: "5" };
    const connector = records.find((r) => r.object_id === "obj3-assign3").payload.data;
    Object.assign(connector, { right_center_id: BID, right_pending: false });
    const segs = [4, 5, 6].map((k) => ({ kind: "CC-013.segment", object_id: "obj3-seg" + k,
      payload: { data: { entity_id: U[k], result: "CASE_ONE", construction: { start_stroke: String(k), stroke_indices: [String(k)], direction: RANGE[k][0] },
        stroke_refs: [{ stroke_index: String(k), start_price: RANGE[k][0] === "UP" ? RANGE[k][1] : RANGE[k][2], end_price: RANGE[k][0] === "UP" ? RANGE[k][2] : RANGE[k][1] }] } } }));
    const attempt = { entity_id: "seed_attempt:1:12", units: [4, 5, 6].map(unit), vector: { completed: true, consecutive: true, same_level: true, alternating: true, strict: true },
      ZD: { value: "1", contributors: [U[5]], same_price_witness: [] }, ZG: { value: "3", contributors: [U[5]], same_price_witness: [] },
      core_order: "ZD<ZG", result: "FORMED", exclusion_reason: null, center_id: BID, known_at: K };
    const center = { entity_id: BID, entity_revision: "1", seed_attempt_id: attempt.entity_id, seed_units: U.slice(4, 7), zn_direction: "UP", members: U.slice(4, 7),
      zn_members: [U[4], U[6]], status: "EXTENDING", settled: false, break_unit: null, break_up: false, break_up_defined: false,
      resume_anchor: null, next_tested_unit: null, roles_id: "roles:" + BID, formed_known_at: K, settled_known_at: null };
    const roles = { entity_id: "roles:" + BID, entity_revision: "1", center_id: BID, previous: null, known_at: K,
      roles: { CORE: { ZD: role("1", [U[5]], true), ZG: role("3", [U[5]], true) }, OUTER: { DD: role("0", [U[4], U[6]], false), GG: role("5", [U[6]], false) },
        INNER_EXTREMA: { D: role("0", [U[4], U[6]], false), G: role("4", [U[4]], false) } } };
    const ends = ["DD", "ZD", "ZG", "GG"], cmp = (x, y) => (BigInt(x) < BigInt(y) ? "<" : x === y ? "=" : ">"), matrix = {};
    for (const x of ends) for (const y of ends) matrix[`B.${x}?A.${y}`] = cmp(B[x], A[y]);
    const pair = { entity_id: "pair:1:0:1:12", entity_revision: "1", a_center_id: CID, a_revision: "1", b_center_id: BID, b_revision: "1", connector_units: [U[3]],
      status: "EVALUATED", leaf: "DOWN_NEWBIRTH", direction: "DOWN", provisional: true, inputs: { A, B }, matrix,
      witnesses: { core_tangent: false, outer_touch: false, outer_containment: "NONE", core_containment: "NONE" },
      realization: { upper_center: "NOT_CONSTRUCTED_DEFERRED_1429", trend: "NOT_ASSERTED_DEFERRED_1426" }, null_reason: null, previous: null, formed_known_at: K, known_at: K };
    const rel = (kind, s, r, tt, v, w) => ({ kind, object_id: "obj3-rel-" + [s, r, tt].join("|"), payload: (kind.includes("pair") ? f : e)([s, r, tt], { source_id: s, relation_kind: r, target_id: tt, version: v, witness_object_id: w }) });
    const assign = (k) => ({ kind: C.CENTER_KINDS[3], object_id: "obj3-assign" + k, payload: e("assign:" + U[k], { entity_id: "assign:" + U[k], unit: unit(k), status: "CENTER_MEMBER",
      center_id: BID, left_center_id: null, right_center_id: null, right_pending: false, reason: null, seed_from_self: k === 4 ? "FORMED" : "NOT_EVALUATED", known_at: K }) });
    const more = [
      ...segs,
      { kind: C.CENTER_KINDS[0], object_id: "obj3-attemptB", payload: e(attempt.entity_id, attempt) },
      { kind: C.CENTER_KINDS[1], object_id: "obj3-centerB", payload: e(BID, center) },
      { kind: C.CENTER_KINDS[2], object_id: "obj3-rolesB", payload: e(roles.entity_id, roles) },
      ...[4, 5, 6].map(assign),
      rel(C.CENTER_KINDS[4], attempt.entity_id, "attempted_from", U[4], "1", "obj3-attemptB"),
      ...[4, 5, 6].map((k) => rel(C.CENTER_KINDS[4], U[k], "member_of", BID, "1", "obj3-centerB")),
      rel(C.CENTER_KINDS[4], U[3], "connects_right", BID, "1", "obj3-assign3"),
      { kind: C.CENTER_KINDS[5], object_id: "obj3-cchangeB", payload: e(["1", BID, "formed"], { entity_id: BID, change: "formed", before: null, after: center, known_at: K, object_id: "obj3-centerB" }) },
      { kind: C.CENTER_KINDS[6], object_id: "obj3-cver", payload: e("TB-02-E", { input_frontier: "10", known_at: K, waiting_reasons: [],
        scan_cursor: { state: "EXTENDING", center_id: BID, seed_start_unit_id: null, awaiting_unit_id: null, awaiting_after_unit_id: U[6], awaiting_offset: "1" },
        delivered_scope: "l0_strict_seed_zn_roles_c_resume_assignment", deferred_scope: C.TB02E_DEFERRED }) },
      { kind: C.PAIR_KINDS[1], object_id: "obj3-pair", payload: f(pair.entity_id, pair) },
      rel(C.PAIR_KINDS[2], pair.entity_id, "pair_from", CID, "1", "obj3-pair"),
      rel(C.PAIR_KINDS[2], pair.entity_id, "pair_to", BID, "1", "obj3-pair"),
      { kind: C.PAIR_KINDS[3], object_id: "obj3-pchange", payload: f(["1", pair.entity_id, "formed"], { entity_id: pair.entity_id, change: "formed", before: null, after: pair, known_at: K, object_id: "obj3-pair" }) },
      { kind: C.PAIR_KINDS[4], object_id: "obj3-pver", payload: f("TB-02-F", { input_frontier: "10", known_at: K, pair_count: "1", pending_pairs: "0", waiting_reasons: [],
        delivered_scope: "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position", deferred_scope: C.TB02F_DEFERRED }) },
    ];
    for (const r of more) Object.assign(r, { first_known_generation: "1", withdrawn_generation: null });
    return [...records, ...more];
  } finally {
    U.length = 4;
    RANGE.length = 4;
  }
}
const AXES22 = ["CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008", "CC-009", "CC-010", "CC-011", "CC-012", "CC-013", "CC-014",
  "CC-016", "CC-017", "CC-018", "CC-020", "CC-022", "CC-054", "CC-055", "CC-056"];
function axes22(records) {
  const ids = (kinds) => records.filter((r) => kinds.includes(r.kind)).map((r) => r.object_id);
  const out = {};
  for (const cid of AXES22) {
    const seg = ["CC-011", "CC-012", "CC-013", "CC-014"].includes(cid),
      center = ["CC-016", "CC-017", "CC-022"].includes(cid),
      pair = ["CC-018", "CC-020"].includes(cid);
    const evidence = { scope: seg ? "TB-02-D" : center ? "TB-02-E" : pair ? "TB-02-F" : "TB-02-A", object_ids: [], waiting_reasons: [] };
    if (cid === "CC-013" || cid === "CC-014") evidence.object_ids = ids(["CC-013.segment"]);
    if (cid === "CC-016") evidence.object_ids = ids(C.CENTER_KINDS.slice(0, 2));
    if (cid === "CC-017") evidence.object_ids = ids([C.CENTER_KINDS[2]]);
    if (cid === "CC-022") evidence.object_ids = ids([C.CENTER_KINDS[3]]);
    if (cid === "CC-018") evidence.object_ids = ids([C.PAIR_KINDS[0]]);
    if (cid === "CC-020") evidence.object_ids = ids([C.PAIR_KINDS[1]]);
    if (seg) Object.assign(evidence, { delivered_subobligations: ["finite_complete_prefix_strict_second", "independent_stroke_and_segment_break_facts"],
      deferred_subobligations: ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"] });
    if (center) Object.assign(evidence, { delivered_subobligations: C.TB02E_DELIVERED, deferred_subobligations: C.TB02E_DEFERRED });
    if (pair) Object.assign(evidence, { delivered_subobligations: C.TB02F_DELIVERED, deferred_subobligations: C.TB02F_DEFERRED });
    if (cid === "CC-056") Object.assign(evidence, { raw_revisions: [], withdrawals: [], replaces: [] });
    const notImpl = ["CC-012", "CC-013", "CC-014", "CC-016", "CC-017", "CC-018", "CC-020", "CC-022"].includes(cid);
    out[cid] = { impl_status: notImpl ? "not_implemented" : "implemented", proof_status: "not_proved",
      run_status: cid === "CC-056" ? "not_run" : evidence.object_ids.length ? "run" : "not_run", evidence };
  }
  return out;
}

test("S3 规则串：W 后继显式映射 CF22 基规则，未授权组合一律拒", () => {
  assert.equal(typed.W_STRUCTURE_RULE, W);
  assert.equal(typed.baseRule(W), BASE);
  assert.deepEqual(typed.RULE_CONTRACTS[BASE], ["s-new-bi/2", "s-segment/5", "established_direction_with_extreme_root_sets"]);
  assert(typed.CENTER_RULES.has(BASE));
  const bad = [
    BASE, // 剥去 W 后缀：库里无此 cut 规则
    W + "+raw-initialization-w/1", // 重复后缀
    BASE + "+raw-initialization-w/2", // W 版本变体
    "s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1+raw-initialization-w/1", // 旧 CF22 未经段组件
    "s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1",
    W.replace("center-seed/1+center-pair/1", "center-pair/1+center-seed/1"), // 错序
    W.replace("+segment-turn/1", ""), // 缺组件
    W.replace("+center-pair/1", ""),
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+raw-initialization-w/1",
  ];
  for (const rule of bad) assert.throws(() => typed.baseRule(rule), /未获准的完整规则组合/, rule);
});

test("22 轴目录：CF22/W 规则选 22 轴，scope 与子义务按 E/F 片，中枢轴绑定实际对象", async () => {
  const records = centerCut(),
    axes = axes22(records);
  await Q3.axesValidate(axes);
  const reject = async (mutate, pattern) => {
    const a = copy(axes);
    mutate(a);
    await assert.rejects(() => Q3.axesValidate(a), pattern);
  };
  await reject((a) => delete a["CC-020"], /逐cut目录轴 完整字段不符/);
  await reject((a) => (a["CC-016"].evidence.scope = "TB-02-A"), /完整字段不符|轴范围错误/);
  await reject((a) => (a["CC-018"].evidence.scope = "TB-02-E"), /中枢子义务范围不符|轴范围错误/);
  await reject((a) => (a["CC-017"].evidence.deferred_subobligations = C.TB02F_DEFERRED), /中枢子义务范围不符/);
  await reject((a) => (a["CC-020"].impl_status = "implemented"), /有限交付冒称全轴完成/);
  await reject((a) => (a["CC-013"].evidence.scope = "TB-02-W"), /轴范围错误|完整字段不符/);
  // 22 轴之外再多一条段/中枢轴都不是合法轴表。
  await reject((a) => (a["CC-015"] = copy(a["CC-016"])), /逐cut目录轴 完整字段不符/);
  // 对象绑定：轴证据须恰为实际中枢族对象（含无中枢族对象时为空）。
  const bound = copy(axes);
  bound["CC-016"].evidence.object_ids.pop();
  assert.throws(() => C.validateCenterFamilyCut(records, bound), /TB02E_AXIS_BINDING/);
  const segOnly = records.filter((r) => r.kind === "CC-013.segment"),
    empty = axes22(segOnly);
  C.validateCenterFamilyCut(segOnly, empty);
  empty["CC-022"].evidence.object_ids = ["obj3-seg0"];
  assert.throws(() => C.validateCenterFamilyCut(segOnly, empty), /TB02E_AXIS_BINDING/);
});

test("中枢族 typed：每种对象按 s-center/1、s-center-pair/1 合同校验，同 cut 链接闭合", () => {
  const records = centerCut();
  for (const r of records)
    if (C.CENTER_KINDS.includes(r.kind)) C.validateCenterPayload(r.kind, r.payload);
    else if (C.PAIR_KINDS.includes(r.kind)) C.validatePairPayload(r.kind, r.payload);
  C.validateCenterFamilyCut(records, axes22(records));
  C.validatePairHistory(records);
  const reject = (mutate, pattern) => {
    const rs = copy(records);
    mutate(rs);
    assert.throws(() => {
      for (const r of rs)
        if (C.CENTER_KINDS.includes(r.kind)) C.validateCenterPayload(r.kind, r.payload);
        else if (C.PAIR_KINDS.includes(r.kind)) C.validatePairPayload(r.kind, r.payload);
      C.validateCenterFamilyCut(rs, axes22(rs));
    }, pattern);
  };
  const find = (rs, kind, k = 0) => rs.filter((r) => r.kind === kind)[k].payload;
  reject((rs) => (find(rs, C.CENTER_KINDS[1]).schema_revision = "s-center/2"), /TB02E_VERSION/);
  reject((rs) => (find(rs, C.PAIR_KINDS[0]).policy_version = "x"), /TB02F_VERSION/);
  reject((rs) => (find(rs, C.CENTER_KINDS[2]).data.roles.OUTER.DD.value = "11"), /TB02E_ROLE_VALUE|TB02E_CONTRIBUTORS/);
  reject((rs) => (find(rs, C.CENTER_KINDS[0]).data.ZD.value = "11"), /TB02E_CORE_VALUE/);
  reject((rs) => (find(rs, C.PAIR_KINDS[0]).data.position = "LOW_TOUCH"), /TB02F_POSITION/);
  reject((rs) => (find(rs, C.PAIR_KINDS[0]).data.construction_role = "EXTENSION_MEMBER"), /TB02F_ROLE/);
  reject((rs) => rs.splice(rs.findIndex((r) => r.payload.data?.relation_kind === "member_of"), 1), /TB02E_MEMBER_EDGE_COUNT/);
  reject((rs) => rs.splice(rs.findIndex((r) => r.payload.data?.relation_kind === "position_of"), 1), /TB02F_EDGE/);
  reject((rs) => rs.splice(rs.findIndex((r) => r.kind === C.PAIR_KINDS[4]), 1), /TB02F_VERSION_COUNT/);
  reject((rs) => (find(rs, C.CENTER_KINDS[6]).data.scan_cursor.awaiting_offset = "2"), /TB02E_SCAN_CURSOR/);
  reject((rs) => (find(rs, C.CENTER_KINDS[3], 3).data.status = "CENTER_MEMBER"), /TB02E_ASSIGNMENT/);
  reject((rs) => (find(rs, C.CENTER_KINDS[1]).slot = "center:1:3"), /TB02E_SLOT/);
  // 单元区间须是同 cut CC-013 成员笔极值。
  reject((rs) => (rs[0].payload.data.stroke_refs[0].end_price = "21"), /TB02E_UNIT_SOURCE/);
  reject((rs) => rs.push({ kind: "CC-016.center_shadow", object_id: "x", payload: {} }), /TB02E_CANDIDATE_KIND/);
});

test("公开入口：中枢族记录经 typed 路由与规则绑定，旧规则 cut 混入中枢族/W 对象拒", async () => {
  const records = centerCut(),
    attempt = records.find((r) => r.kind === C.CENTER_KINDS[0]);
  const ref = { identity_key: "k0", payload_hash: "0".repeat(64), receipt_id: "r1", event_id: "e1", input_revision: "1", revision: "1", seq: "0", source_coord: "0" };
  const record = { object_id: "obj3-x", object_revision: "1", kind: attempt.kind, batch_id: "b1", fact_key: API.canonical([attempt.kind, attempt.payload.slot]),
    payload: attempt.payload, input_refs: [ref], source_coords: ["0"], first_known_generation: "1", first_known_cut: "cut-1", published_generation: "1",
    withdrawn_generation: null, withdrawal_reason: null, superseded_by: null, lifecycle: "active" };
  await typed.validateTyped(Q3.V.of(record));
  const bad = copy(record);
  bad.payload.data.core_order = "ZD=ZG";
  await assert.rejects(() => typed.validateTyped(Q3.V.of(bad)), /TB02E_CORE_ORDER/);
  const wrongSlot = copy(record);
  wrongSlot.fact_key = API.canonical([attempt.kind, "seed_attempt:1:3"]);
  await assert.rejects(() => typed.validateTyped(Q3.V.of(wrongSlot)), /事实语义槽未绑定 payload/);
  // 规则绑定先于内容身份：W 规则下进入身份比对（身份不符），旧规则下按 RULE_MIX 拒。
  const fixed = (rule) => ({ rule_revision: rule, profile_id: "ohlc_integer_tb02a_v1", protocol: "s-session/3" });
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(record), fixed(W)), /对象内容身份不符/);
  // S3 结构规则只属 s-session/3：v2 协议 cut 携 W 串按未获准组合拒（v2 写者已退役）。
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(record), { ...fixed(W), protocol: "s-session/2" }), /未获准的完整规则组合/);
  const old = "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2";
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(record), fixed(old)), /TB02E_RULE_MIX/);
  const pair = records.find((r) => r.kind === C.PAIR_KINDS[0]);
  const pairRecord = { ...copy(record), kind: pair.kind, payload: pair.payload, fact_key: API.canonical([pair.kind, pair.payload.slot]) };
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(pairRecord), fixed(old)), /TB02F_RULE_MIX/);
  // W 对象只准出现在 W 后继串 cut。
  const wRecord = { ...copy(record), kind: "CC-004.initialization", payload: { initialization_policy_revision: "raw-initialization-w/1", direction_state: "Undetermined", first_raw_pair: null },
    fact_key: API.canonical(["CC-004.initialization", "W"]) };
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(wRecord), fixed(old)), /旧规则cut混入W对象/);
  await assert.rejects(() => Q3.validateTB02Identity(Q3.V.of(wRecord), fixed(W)), /对象内容身份不符/);
});

test("相邻中枢对：五叶/方向/矩阵/证据/连接段/修订/边/版本计数闭合，出生事件绑定发布代", () => {
  const records = twoCenterCut();
  const run = (rs) => {
    for (const r of rs)
      if (C.CENTER_KINDS.includes(r.kind)) C.validateCenterPayload(r.kind, r.payload);
      else if (C.PAIR_KINDS.includes(r.kind)) { C.validatePairPayload(r.kind, r.payload); if ([C.PAIR_KINDS[1], C.PAIR_KINDS[3]].includes(r.kind)) C.validatePairPublication(r, r.first_known_generation); }
    C.validateCenterFamilyCut(rs, axes22(rs));
    C.validatePairHistory(rs);
  };
  run(records);
  const pairOf = (rs) => rs.find((r) => r.kind === C.PAIR_KINDS[1]).payload.data;
  const reject = (mutate, pattern) => { const rs = copy(records); mutate(rs); assert.throws(() => run(rs), pattern); };
  reject((rs) => (pairOf(rs).leaf = "DOWN_EXPANSION"), /TB02F_LEAF/);
  reject((rs) => (pairOf(rs).direction = "UP"), /TB02F_DIRECTION/);
  reject((rs) => (pairOf(rs).matrix["B.GG?A.DD"] = ">"), /TB02F_MATRIX/);
  reject((rs) => (pairOf(rs).provisional = false), /TB02F_PROVISIONAL|TB02F_CHANGE/);
  reject((rs) => (pairOf(rs).realization.trend = "ASSERTED"), /TB02F_REALIZATION/);
  reject((rs) => (pairOf(rs).inputs.B.GG = "6"), /TB02F_/);
  reject((rs) => (pairOf(rs).known_at = { ...K, generation: "2" }), /TB02F_PUBLICATION|TB02F_CHANGE/);
  reject((rs) => rs.splice(rs.findIndex((r) => r.kind === C.PAIR_KINDS[3]), 1), /TB02F_CHANGE/);
  reject((rs) => (rs.find((r) => r.kind === C.PAIR_KINDS[4]).payload.data.pair_count = "0"), /TB02F_VERSION_COUNT/);
  reject((rs) => (rs.find((r) => r.object_id === "obj3-assign3").payload.data.right_center_id = null), /TB02E_/);
  // 撤回：后一代中枢对撤回须有 withdrawn 事件，且前件为真实活动对象。
  const withdrawn = copy(records).filter((r) => [C.PAIR_KINDS[1], C.PAIR_KINDS[3]].includes(r.kind));
  withdrawn.find((r) => r.kind === C.PAIR_KINDS[1]).withdrawn_generation = "2";
  assert.throws(() => C.validatePairHistory(withdrawn), /TB02F_CHANGE/);
  const pairData = pairOf(records), K2 = { ...K, generation: "2" };
  withdrawn.push({ kind: C.PAIR_KINDS[3], object_id: "obj3-pchange2", first_known_generation: "2", withdrawn_generation: null,
    payload: f(["2", pairData.entity_id, "withdrawn"], { entity_id: pairData.entity_id, change: "withdrawn", before: pairData, after: null, known_at: K2, object_id: null }) });
  C.validatePairHistory(withdrawn);
});

test("通用整数遍历：中枢/中枢对 payload.slot 是逻辑身份（取 q3-center.js 的 schema），换成非逻辑槽 schema 即按整数拒", async () => {
  const records = twoCenterCut(),
    center = records.filter((r) => C.CENTER_KINDS.includes(r.kind)),
    pair = records.filter((r) => C.PAIR_KINDS.includes(r.kind));
  assert(center.length && pair.length);
  // 正控：中枢族与中枢对各 kind 的 slot（"TB-02-E"/"TB-02-F"、实体 id、三元组）经整数遍历放行。
  for (const r of [...center, ...pair]) await Q3.integerWalk(Q3.V.of({ kind: r.kind, payload: r.payload }));
  // 负控：同一 payload 只改 schema_revision（不在逻辑槽表），slot 回到整数口径即拒。
  for (const r of [center.find((x) => x.kind === C.CENTER_KINDS[6]), pair.find((x) => x.kind === C.PAIR_KINDS[4])]) {
    const bad = copy(r.payload);
    bad.schema_revision = "s-unknown/1";
    await assert.rejects(() => Q3.integerWalk(Q3.V.of({ kind: r.kind, payload: bad })), /精确整数必须是规范文本/);
  }
  // 见证等非逻辑槽对象的整数 slot 仍按整数核对。
  await assert.rejects(() => Q3.integerWalk(Q3.V.of({ witness_id: "w", slot: "TB-02-E" })), /精确整数必须是规范文本/);
});
