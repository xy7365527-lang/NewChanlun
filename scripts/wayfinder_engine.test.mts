/**
 * wayfinder_engine 纯函数走查器测试（#1084 验收第一项）。
 * 合成 tracker 状态夹具（图清 / 批准 / 全关三态 + 负例）→ 输出动作逐项断言。
 * 跑法：npx tsx --test scripts/wayfinder_engine.test.mts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  analyzeMap,
  walkGraph,
  walkGraphs,
  isSpecApproved,
  unresolvedFog,
  parseSplitPlan,
  isDecisionTicket,
  isSpecTicket,
  isImplTicket,
  isPureDecisionMap,
  classifyTicket,
  openBlockerCount,
  buildSpecApprovalIndex,
  buildOpenChildIndex,
  buildSpecTreeFacts,
  recheckBeforeRelease,
  releaseTicket,
  withSpecApproval,
  explainTicket,
  isImplCandidate,
  summarizeQueue,
  planReleases,
  renderStatus,
  resolveSpecDrafter,
  specDrafterCommand,
  specDrafterEnv,
  DEFAULT_SPEC_MODEL,
  DEFAULT_SPEC_PROVIDER,
  SUPPORTED_SPEC_MODELS,
} from "./wayfinder_engine.mts";
import type {
  ChildIssue,
  IssueComment,
  MapState,
  WalkOptions,
  ReadyTicket,
} from "./wayfinder_engine.mts";

const ORCH = "xy7365527-lang";
const OPTS: WalkOptions = { orchestrator: ORCH };

// ── 夹具构造 ──────────────────────────────────────────────────────────────────

function child(partial: Partial<ChildIssue> & { number: number; title: string }): ChildIssue {
  return {
    body: "",
    state: "OPEN",
    labels: [],
    assignees: [],
    blockedBy: 0,
    comments: [],
    ...partial,
  };
}

function decision(n: number, state: "OPEN" | "CLOSED" = "CLOSED"): ChildIssue {
  return child({ number: n, title: `[grilling] 决策 ${n}`, labels: ["wayfinder:grilling"], state });
}

function spec(n: number, state: "OPEN" | "CLOSED", comments: IssueComment[]): ChildIssue {
  return child({
    number: n,
    title: `📋 SPEC：实施总单（map #42）`,
    state,
    comments,
  });
}

function impl(n: number, state: "OPEN" | "CLOSED" = "OPEN"): ChildIssue {
  return child({ number: n, title: `[impl] S${n} 实装 ${n}`, state });
}

function map(partial: Partial<MapState> & { number: number }): MapState {
  return {
    title: `🗺️ 测试图 ${partial.number}`,
    body: "",
    state: "OPEN",
    labels: ["wayfinder:map"],
    children: [],
    ...partial,
  };
}

function approvalComment(body = "批准。照此执行。"): IssueComment {
  return { author: ORCH, body, createdAt: "2026-08-18T12:00:00Z" };
}

function rejectComment(body = "不批准，打回重写。"): IssueComment {
  return { author: ORCH, body, createdAt: "2026-08-18T12:01:00Z" };
}

// ── 三态验收夹具 ──────────────────────────────────────────────────────────────

test("图清态：决策票全关、无 spec/实装 → draft_spec", () => {
  const m = map({ number: 42, children: [decision(101), decision(102)] });
  const a = analyzeMap(m, OPTS);
  assert.equal(a.phase.includes("图清"), true);
  assert.deepEqual(a.actions, [{ kind: "draft_spec", map: 42 }]);
  assert.deepEqual(walkGraph(m, OPTS), [{ kind: "draft_spec", map: 42 }]);
});

test("批准态：spec 存在且被编排者批 → split_impl_tickets", () => {
  const m = map({
    number: 42,
    children: [decision(101), spec(500, "OPEN", [approvalComment()])],
  });
  const a = analyzeMap(m, OPTS);
  assert.equal(a.phase.includes("已批准"), true);
  assert.deepEqual(a.actions, [{ kind: "split_impl_tickets", map: 42, spec: 500 }]);
});

test("全关态：实装票全关 + 无残雾 → close_graph", () => {
  const m = map({
    number: 42,
    body: "## Not yet specified\n\n（无）\n",
    children: [decision(101), spec(500, "CLOSED", []), impl(201, "CLOSED"), impl(202, "CLOSED")],
  });
  const a = analyzeMap(m, OPTS);
  assert.equal(a.phase.includes("图关判据满足"), true);
  assert.deepEqual(a.actions, [{ kind: "close_graph", map: 42 }]);
});

// ── 中间态与负例 ──────────────────────────────────────────────────────────────

test("走图中：任一决策票 open → 无动作", () => {
  const m = map({ number: 42, children: [decision(101), decision(102, "OPEN")] });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("spec 待批准（无编排者批）→ 无动作", () => {
  const m = map({ number: 42, children: [decision(101), spec(500, "OPEN", [])] });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("spec 被驳回（编排者批「不批准」）→ 无动作", () => {
  const m = map({ number: 42, children: [decision(101), spec(500, "OPEN", [rejectComment()])] });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("驳回后再批准（时间序推进）→ split", () => {
  const m = map({
    number: 42,
    children: [
      decision(101),
      spec(500, "OPEN", [
        rejectComment(),
        { author: ORCH, body: "批准，按此拆票。", createdAt: "2026-08-18T12:02:00Z" },
      ]),
    ],
  });
  assert.deepEqual(walkGraph(m, OPTS), [{ kind: "split_impl_tickets", map: 42, spec: 500 }]);
});

test("实装中（有 open 实装票）→ 无动作", () => {
  const m = map({
    number: 42,
    children: [decision(101), spec(500, "CLOSED", []), impl(201, "CLOSED"), impl(202, "OPEN")],
  });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("批准后拆到一半（计划内缺票）→ 补拆（幂等再触发）", () => {
  const planBody = [
    "## 实施票拆分（机器可读）",
    "<!-- wayfinder-split -->",
    "```json",
    '{"tickets":[{"id":"S1","title":"a"},{"id":"S2","title":"b"}]}',
    "```",
  ].join("\n");
  const s = spec(500, "OPEN", [approvalComment()]);
  s.body = planBody;
  const m = map({
    number: 42,
    children: [decision(101), s, child({ number: 201, title: "[impl] S1 a", state: "OPEN" })],
  });
  assert.deepEqual(walkGraph(m, OPTS), [{ kind: "split_impl_tickets", map: 42, spec: 500 }]);
});

test("批准且拆齐（计划全覆盖）→ 不再拆，落入实装中", () => {
  const planBody = [
    "<!-- wayfinder-split -->",
    "```json",
    '{"tickets":[{"id":"S1","title":"a"}]}',
    "```",
  ].join("\n");
  const s = spec(500, "OPEN", [approvalComment()]);
  s.body = planBody;
  const m = map({
    number: 42,
    children: [decision(101), s, child({ number: 201, title: "[impl] S1 a", state: "OPEN" })],
  });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("实装全关但残雾无去向 → 不关图（无动作）", () => {
  const m = map({
    number: 42,
    body: "## Not yet specified\n\n- 某片雾还没有去处\n",
    children: [decision(101), spec(500, "CLOSED", []), impl(201, "CLOSED")],
  });
  const a = analyzeMap(m, OPTS);
  assert.deepEqual(a.actions, []);
  assert.equal(a.phase.includes("残雾"), true);
});

test("纯决策图（声明不带实装）→ 无动作（v1 不碰）", () => {
  const m = map({
    number: 42,
    body: "## Notes\n本图为纯决策图，不带实装——理由：只裁决策。",
    children: [decision(101)],
  });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("已关图 → 无动作", () => {
  const m = map({ number: 42, state: "CLOSED", children: [decision(101)] });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("无决策子票的新图 → 无动作（不是图清）", () => {
  const m = map({ number: 42, children: [] });
  assert.deepEqual(walkGraph(m, OPTS), []);
});

test("walkGraphs 对多图汇聚动作", () => {
  const m1 = map({ number: 1, children: [decision(11)] });
  const m2 = map({ number: 2, children: [decision(21), decision(22, "OPEN")] });
  assert.deepEqual(walkGraphs([m1, m2], OPTS), [{ kind: "draft_spec", map: 1 }]);
});

// ── 分类与解析纯函数 ──────────────────────────────────────────────────────────

test("角色分类", () => {
  assert.equal(isDecisionTicket(decision(1)), true);
  assert.equal(isSpecTicket(spec(1, "OPEN", [])), true);
  assert.equal(isImplTicket(impl(1)), true);
  assert.equal(isSpecTicket(child({ number: 1, title: "[impl] S1 x" })), false);
  assert.equal(isPureDecisionMap(map({ number: 1, body: "本图为纯决策图，不带实装——理由：x" })), true);
  assert.equal(isPureDecisionMap(map({ number: 1, body: "## Notes\n本图带实装（默认）" })), false);
});

test("isSpecApproved：只认编排者、驳回优先、时间序推进", () => {
  const s = spec(1, "OPEN", [
    { author: "bot-helper", body: "approve", createdAt: "2026-08-18T10:00:00Z" },
    approvalComment(),
  ]);
  assert.equal(isSpecApproved(s, ORCH), true);
  assert.equal(isSpecApproved(spec(1, "OPEN", [rejectComment()]), ORCH), false);
  assert.equal(isSpecApproved(spec(1, "OPEN", [approvalComment(), rejectComment()]), ORCH), false);
  // 「不通过」不得被「通过」子串误判为批准
  assert.equal(
    isSpecApproved(spec(1, "OPEN", [{ author: ORCH, body: "不通过。", createdAt: "2026-08-18T12:00:00Z" }]), ORCH),
    false,
  );
});

test("unresolvedFog：有去向的雾不算残雾", () => {
  assert.deepEqual(unresolvedFog(""), []);
  assert.deepEqual(unresolvedFog("## Not yet specified\n\n- 归 #1057\n- 明写搁置，无人跟进\n- 移 Out of scope\n"), []);
  assert.deepEqual(unresolvedFog("## Not yet specified\n\n- 还没想好\n"), ["- 还没想好"]);
  // 雾段之后的其它 ## 段不算残雾
  assert.deepEqual(unresolvedFog("## Not yet specified\n\n- 归 #1\n\n## Out of scope\n\n- 这行不算雾\n"), []);
});

test("parseSplitPlan：解析 wayfinder-split 标记的 json 围栏", () => {
  const body = [
    "## 实施票拆分（机器可读）",
    "",
    "<!-- wayfinder-split -->",
    "```json",
    '{"tickets":[{"id":"S1","title":"3a 生产单扫描","summary":"两路合一","depends_on":[],"rulings":[1057]},',
    '{"id":"S2","title":"Lean 镜像","depends_on":["S1"],"rulings":[1058]}]}',
    "```",
  ].join("\n");
  const plan = parseSplitPlan(body);
  assert.ok(plan);
  assert.equal(plan.length, 2);
  assert.equal(plan[0]!.id, "S1");
  assert.equal(plan[0]!.rulings[0], 1057);
  assert.deepEqual(plan[1]!.dependsOn, ["S1"]);
});

test("parseSplitPlan：无标记时兜底扫全文；无票时返回 null", () => {
  const body = "```json\n{\"tickets\":[{\"id\":\"S1\",\"title\":\"x\"}]}\n```";
  assert.equal(parseSplitPlan(body)!.length, 1);
  assert.equal(parseSplitPlan("没有 json"), null);
  assert.equal(parseSplitPlan("```json\n{\"other\":1}\n```"), null);
});

// ── #1084 追加：队列分类与 sandcastle 释放规划 ─────────────────────────────────

function ready(partial: Partial<ReadyTicket> & { number: number }): ReadyTicket {
  return {
    title: `#${partial.number} 测试票`,
    // 旧扫描集只有 ready-for-agent，夹具默认带上；新增的 wayfinder:impl 路径由各测试显式给标签。
    labels: ["ready-for-agent"],
    assignees: [],
    blockedBy: 0,
    specApproved: null,
    ...partial,
  };
}

test("classifyTicket：五桶判据逐项", () => {
  assert.equal(classifyTicket(ready({ number: 1, blockedBy: 1 })), "blocked");
  assert.equal(classifyTicket(ready({ number: 2, assignees: ["me"] })), "claimed");
  assert.equal(
    classifyTicket(ready({ number: 3, assignees: ["me"], labels: ["sandcastle"] })),
    "running",
  );
  assert.equal(classifyTicket(ready({ number: 4, labels: ["sandcastle"] })), "released");
  assert.equal(
    classifyTicket(ready({ number: 5, title: "[impl] S1 x", specApproved: true })),
    "eligible",
  );
  // 合资格实装票但已 assign / 已 blocked → 不判 eligible
  assert.equal(
    classifyTicket(ready({ number: 6, title: "[impl] S1 x", specApproved: true, assignees: ["me"] })),
    "claimed",
  );
  assert.equal(
    classifyTicket(ready({ number: 7, title: "[impl] S1 x", specApproved: true, blockedBy: 1 })),
    "blocked",
  );
  // 不合资格：非实装票 / 实装票但 spec 未批准 / 不在任何 open map
  assert.equal(classifyTicket(ready({ number: 8, title: "[task] 探针" })), "held");
  assert.equal(
    classifyTicket(ready({ number: 9, title: "[impl] S1 x", specApproved: false })),
    "held",
  );
  assert.equal(
    classifyTicket(ready({ number: 10, title: "[impl] S1 x", specApproved: null })),
    "held",
  );
});

test("openBlockerCount：只计 OPEN blocker，totalCount 含已关不算", () => {
  assert.equal(openBlockerCount(undefined), 0);
  assert.equal(openBlockerCount({ totalCount: 0 }), 0);
  assert.equal(
    openBlockerCount({ nodes: [{ state: "CLOSED" }], totalCount: 1 }),
    0,
  );
  assert.equal(
    openBlockerCount({ nodes: [{ state: "OPEN" }, { state: "CLOSED" }], totalCount: 2 }),
    1,
  );
});

test("buildSpecApprovalIndex + withSpecApproval：父图 spec 批准态并入", () => {
  const approvedMap = map({
    number: 42,
    children: [decision(101), spec(500, "OPEN", [approvalComment()]), impl(201)],
  });
  const pendingMap = map({
    number: 43,
    children: [decision(102), spec(501, "OPEN", []), impl(202)],
  });
  const idx = buildSpecApprovalIndex([approvedMap, pendingMap], ORCH);
  assert.equal(idx.get(201), true);
  assert.equal(idx.get(202), false);

  const tickets = withSpecApproval(
    [ready({ number: 201, title: "[impl] S1 x" }), ready({ number: 999, title: "[impl] S9 x" })],
    idx,
  );
  assert.equal(tickets[0]!.specApproved, true);
  assert.equal(tickets[1]!.specApproved, null);
});

test("planReleases：只放行合资格实装票，不无差别放行历史 ready-for-agent", () => {
  const tickets = [
    ready({ number: 1, title: "[impl] S1 a", specApproved: true }),        // eligible
    ready({ number: 2, title: "[impl] S2 b", specApproved: true }),        // eligible
    ready({ number: 3, title: "[impl] S3 c", specApproved: true, blockedBy: 1 }), // blocked
    ready({ number: 4, title: "[impl] S4 d", specApproved: true, assignees: ["me"] }), // claimed
    ready({ number: 5, title: "[impl] S5 e", specApproved: false }),       // 未批准 spec
    ready({ number: 6, title: "[impl] S6 f", specApproved: null }),        // 不在 open map
    ready({ number: 7, title: "📋 SPEC：x", specApproved: true }),          // 非实装票
    ready({ number: 8, title: "[task] 探针", specApproved: true }),        // 非实装票
    ready({ number: 9, title: "[impl] S9 g", specApproved: true, labels: ["sandcastle"] }), // 已释放
  ];
  assert.deepEqual(planReleases(tickets), [1, 2]);
  const summary = summarizeQueue(tickets);
  assert.equal(summary.buckets.eligible, 2);
  assert.equal(summary.buckets.blocked, 1);
  assert.equal(summary.buckets.claimed, 1);
  assert.equal(summary.buckets.released, 1);
  assert.equal(summary.buckets.held, 4);
  assert.deepEqual(summary.eligible, [1, 2]);
});

test("renderStatus：五桶分明，不把 sandcastle 空误写成 ready-for-agent 空", () => {
  const tickets = [
    ready({ number: 1, title: "[impl] S1 a", specApproved: true }),
    ready({ number: 2, title: "📋 SPEC：x", specApproved: true }),
  ];
  const summary = summarizeQueue(tickets);
  const text = renderStatus(
    { dryRun: true, stamp: "2026-08-21 00:00:00" },
    [],
    tickets,
    summary,
  );
  assert.equal(text.includes("扫描集 open（ready-for-agent ∪ wayfinder:impl）：2 张"), true);
  assert.equal(text.includes("已获 sandcastle 拾取权（可拾取）：0"), true);
  assert.equal(text.includes("待释放（合资格实装票，本轮挂 sandcastle）：1 → #1"), true);
  assert.equal(text.includes("不合资格（决策/spec/未批准 spec/人闸标签等，不放行）：1"), true);
});

// ── 图审计 2026-10-20 第 14/15 条：SPEC 子树递归、wayfinder:impl 扫描、SPEC 标题兼容 ──

test("isSpecTicket：兼容「📋 SPEC：」「SPEC：」「SPEC:」与 [spec]，不误认正文提及", () => {
  for (const t of ["📋 SPEC：完整生产程序（#1323，R2）", "SPEC：完整生产程序（#1323）", "SPEC: x", "  SPEC：x", "[spec] x", "📋 SPEC x"]) {
    assert.equal(isSpecTicket({ title: t }), true, t);
  }
  for (const t of ["[impl] TB-02 落地 SPEC：x", "SPECIAL 项", "关于 SPEC：x 的讨论", "[grilling] SPEC 批准口径", "spec: 小写不算"]) {
    assert.equal(isSpecTicket({ title: t }), false, t);
  }
});

test("isImplCandidate：[impl] 标题或 wayfinder:impl 标签；决策票即使带 impl 标签也不算", () => {
  assert.equal(isImplCandidate({ title: "[impl] x", labels: [] }), true);
  assert.equal(isImplCandidate({ title: "TB-03-B 叶子", labels: ["wayfinder:impl"] }), true);
  assert.equal(isImplCandidate({ title: "[task] x", labels: [] }), false);
  assert.equal(isImplCandidate({ title: "[impl] x", labels: ["wayfinder:grilling"] }), false);
});

/** #1323 形状：map → SPEC #1340 → TB 汇总 → 叶 → 纵片；另有 map 直挂实装票与其子票。 */
function tree1323(specComments: IssueComment[]): MapState {
  const slice = child({ number: 1524, title: "[impl] 纵片 B", labels: ["wayfinder:impl"] });
  const leaf = child({ number: 1394, title: "[impl] TB-03-B 叶", labels: ["wayfinder:impl"], children: [slice] });
  const leafClosed = child({ number: 1390, title: "[impl] TB-03-A 叶", state: "CLOSED", labels: ["wayfinder:impl"] });
  const tb = child({ number: 1362, title: "TB-04 汇验", labels: ["wayfinder:impl"], children: [leaf, leafClosed] });
  const spec1340 = child({ number: 1340, title: "📋 SPEC：完整生产程序（#1323，R2）", comments: specComments, children: [tb] });
  const nested = child({ number: 1527, title: "[task] 恢复路径", labels: ["wayfinder:impl"] });
  const direct = child({ number: 1405, title: "[impl] 直挂", children: [nested] });
  return map({ number: 1323, children: [decision(1530, "OPEN"), spec1340, direct] });
}

test("buildSpecApprovalIndex：已批准 SPEC 的批准态递归到 TB 汇总、叶、纵片与直挂票的子票", () => {
  const idx = buildSpecApprovalIndex([tree1323([approvalComment()])], ORCH);
  for (const n of [1362, 1394, 1390, 1524, 1405, 1527]) assert.equal(idx.get(n), true, `#${n}`);
  const pending = buildSpecApprovalIndex([tree1323([])], ORCH);
  for (const n of [1362, 1394, 1524, 1527]) assert.equal(pending.get(n), false, `#${n}`);
  // 驳回后不放行（时间序推进）。
  const rejected = buildSpecApprovalIndex([tree1323([approvalComment(), rejectComment()])], ORCH);
  assert.equal(rejected.get(1394), false);
});

test("buildSpecApprovalIndex：子树里的 SPEC 用自己的批准态（最近 SPEC 说了算）", () => {
  const leaf = child({ number: 901, title: "[impl] 子 spec 下的叶", labels: ["wayfinder:impl"] });
  const subSpec = child({ number: 900, title: "SPEC：子 spec", children: [leaf] });
  const m = map({ number: 42, children: [spec(500, "OPEN", [approvalComment()]), subSpec] });
  assert.equal(buildSpecApprovalIndex([m], ORCH).get(901), false);
  subSpec.comments = [approvalComment()];
  const m2 = map({ number: 43, children: [spec(501, "OPEN", []), subSpec] });
  assert.equal(buildSpecApprovalIndex([m2], ORCH).get(901), true);
});

test("buildSpecApprovalIndex：环与重复挂载不死循环", () => {
  const a = child({ number: 1, title: "[impl] a" });
  const b = child({ number: 2, title: "[impl] b", children: [a] });
  a.children = [b];
  const idx = buildSpecApprovalIndex([map({ number: 42, children: [spec(500, "OPEN", [approvalComment()]), a] })], ORCH);
  assert.equal(idx.get(1), true);
  assert.equal(idx.get(2), true);
});

test("buildSpecTreeFacts：嵌套 SPEC 不能绕过上层未批准 / 驳回；underSpec 只标 SPEC 子树", () => {
  const leaf = child({ number: 911, title: "[impl] 叶", labels: ["wayfinder:impl"] });
  const inner = child({ number: 910, title: "SPEC：内层", comments: [approvalComment()], children: [leaf] });
  const outer = child({ number: 909, title: "📋 SPEC：外层", comments: [rejectComment()], children: [inner] });
  const direct = child({ number: 912, title: "[impl] 直挂" });
  const f = buildSpecTreeFacts([map({ number: 42, children: [outer, direct] })], ORCH);
  assert.equal(f.approved.get(911), false);
  outer.comments = [approvalComment()];
  const g = buildSpecTreeFacts([map({ number: 42, children: [outer, direct] })], ORCH);
  assert.equal(g.approved.get(911), true);
  assert.equal(g.underSpec.has(911), true);
  assert.equal(g.underSpec.has(912), false);
  assert.equal(g.approved.get(912), true); // map 直挂继承 map 的 SPEC（外层即 map 直挂 SPEC）
});

test("buildOpenChildIndex：递归截断时用 sub_issues_summary 判汇总票", () => {
  const truncated = child({ number: 920, title: "[impl] 深层汇总", labels: ["wayfinder:impl"], openSubIssues: 2 });
  const idx = buildOpenChildIndex([map({ number: 42, children: [truncated] })]);
  assert.equal(idx.get(920), 2);
});

test("recheckBeforeRelease：写前重读到负责人 / blocker / 人闸 / 已关即不挂", () => {
  assert.equal(recheckBeforeRelease({ state: "OPEN", labels: [], assignees: [], blockedBy: { nodes: [] } }), null);
  assert.match(recheckBeforeRelease({ state: "OPEN", assignees: [{ login: "xy7365527-lang" }] })!, /已有负责人/);
  assert.match(recheckBeforeRelease({ state: "OPEN", blockedBy: { nodes: [{ state: "OPEN" }] } })!, /blocker/);
  assert.match(recheckBeforeRelease({ state: "OPEN", labels: [{ name: "needs-triage" }] })!, /人闸/);
  assert.match(recheckBeforeRelease({ state: "CLOSED" })!, /已关闭/);
  assert.equal(recheckBeforeRelease({ state: "OPEN", blockedBy: { nodes: [{ state: "CLOSED" }] } }), null);
});

test("放行：TB 叶子只带 wayfinder:impl 也能放行；已有负责人、open blocker、汇总票、人闸标签一律不放行", () => {
  const maps = [tree1323([approvalComment()])];
  const scan: ReadyTicket[] = [
    ready({ number: 1524, title: "纵片 B", labels: ["wayfinder:impl"] }),                       // 可放行
    ready({ number: 1394, title: "[impl] TB-03-B", labels: ["wayfinder:impl"], assignees: ["xy7365527-lang"] }), // 负责人在做
    ready({ number: 1527, title: "[task] 恢复路径", labels: ["wayfinder:impl"], blockedBy: 1 }),   // open blocker
    ready({ number: 1362, title: "TB-04 汇验", labels: ["wayfinder:impl"] }),                     // 汇总票，子票未全关
    ready({ number: 1405, title: "[impl] 直挂", labels: ["needs-approval"] }),                     // 人闸
    ready({ number: 1530, title: "[grilling] FU-08", labels: ["wayfinder:grilling", "ready-for-agent"] }), // 决策票
    ready({ number: 7777, title: "[impl] 图外", labels: ["wayfinder:impl"] }),                    // 不在 open map 子树
    ready({ number: 1390, title: "[impl] TB-03-A", labels: ["wayfinder:impl", "needs-triage"] }), // 未分诊
  ];
  const facts = buildSpecTreeFacts(maps, ORCH);
  const view = withSpecApproval(scan, facts.approved, buildOpenChildIndex(maps), facts.underSpec);
  assert.deepEqual(planReleases(view), [1524]);
  const why = Object.fromEntries(view.map((t) => [t.number, explainTicket(t)]));
  assert.match(why[1394]!, /^claimed/);
  assert.match(why[1527]!, /^blocked/);
  assert.match(why[1362]!, /汇总票，open 子票 1/);
  assert.match(why[1405]!, /人闸标签 needs-approval/);
  assert.match(why[1530]!, /非实装票/);
  assert.match(why[7777]!, /不在任何 open map 子树内/);
  assert.match(why[1390]!, /人闸标签 needs-triage/);
});

test("map 直挂的 wayfinder:impl 票：未挂 ready-for-agent 不放行，挂了才放行（#1084 原口径）", () => {
  const kid = child({ number: 931, title: "[impl] 直挂票的子票", labels: ["wayfinder:impl"] });
  const direct = child({ number: 930, title: "[impl] 直挂", labels: ["wayfinder:impl"] });
  const parent = child({ number: 932, title: "[impl] 直挂汇总", children: [kid] });
  const maps = [map({ number: 42, children: [spec(500, "OPEN", [approvalComment()]), direct, parent] })];
  const facts = buildSpecTreeFacts(maps, ORCH);
  const view = withSpecApproval(
    [
      ready({ number: 930, title: "[impl] 直挂", labels: ["wayfinder:impl"] }),
      ready({ number: 930, title: "[impl] 直挂", labels: ["wayfinder:impl", "ready-for-agent"] }),
      ready({ number: 931, title: "[impl] 直挂票的子票", labels: ["wayfinder:impl"] }),
    ],
    facts.approved,
    buildOpenChildIndex(maps),
    facts.underSpec,
  );
  assert.equal(classifyTicket(view[0]!), "held");
  assert.match(explainTicket(view[0]!), /祖先链上无 SPEC/);
  assert.equal(classifyTicket(view[1]!), "eligible");
  assert.equal(classifyTicket(view[2]!), "held");
});

test("已有负责人：不论 assignee 最近是否活跃，引擎都不放行（释放认领归人工）", () => {
  const t = ready({ number: 1488, title: "[impl] S 写者", labels: ["wayfinder:impl", "ready-for-agent"], assignees: ["xy7365527-lang"], specApproved: true });
  assert.equal(classifyTicket(t), "claimed");
  assert.deepEqual(planReleases([t]), []);
});

test("releaseTicket：写前复核读到负责人 / blocker 不发 --add-label；干净才发", () => {
  const run = (view: object) => {
    const writes: string[] = [];
    const why = releaseTicket(1524, { read: () => JSON.stringify(view), write: (a) => (writes.push(a), "") });
    return { why, writes };
  };
  const busy = run({ state: "OPEN", labels: [], assignees: [{ login: "xy7365527-lang" }], blockedBy: { nodes: [] } });
  assert.match(busy.why!, /已有负责人/);
  assert.deepEqual(busy.writes, []);
  const blocked = run({ state: "OPEN", labels: [], assignees: [], blockedBy: { nodes: [{ state: "OPEN" }] } });
  assert.deepEqual(blocked.writes, []);
  const ok = run({ state: "OPEN", labels: [{ name: "wayfinder:impl" }], assignees: [], blockedBy: { nodes: [] } });
  assert.equal(ok.why, null);
  assert.deepEqual(ok.writes, ["issue edit 1524 --add-label sandcastle"]);
});

// ── #1617：spec 起草子代理与主控同型（经 CLIProxyLocal） ─────────────────

test("起草子代理默认：claude-cliproxy / gpt-6-astra", () => {
  assert.equal(DEFAULT_SPEC_PROVIDER, "claude-cliproxy");
  assert.equal(DEFAULT_SPEC_MODEL, "gpt-6-astra");
  assert.deepEqual(resolveSpecDrafter({}), { specProvider: "claude-cliproxy", specModel: "gpt-6-astra" });
  assert.deepEqual([...SUPPORTED_SPEC_MODELS], ["gpt-6-astra"]);
});

test("起草子代理显式配置主控同型仍可用", () => {
  assert.deepEqual(resolveSpecDrafter({ WAYFINDER_SPEC_MODEL: "gpt-6-astra" }), {
    specProvider: "claude-cliproxy",
    specModel: "gpt-6-astra",
  });
});

test("起草子代理配非主控模型 → fail-loud，不回退默认", () => {
  for (const m of ["deepseek-v4-pro", "kimi-k3.1-pro", "claude-opus-5-5", "claude-fable-5-1", "mf/claude-opus-5-5", "claude-haiku-4-5", ""]) {
    assert.throws(() => resolveSpecDrafter({ WAYFINDER_SPEC_MODEL: m }), /不等于当前主控模型/, `model=${m}`);
  }
});

test("起草子代理配非 claude-cliproxy provider → fail-loud（旧 deepseek / prime-agent 路由不复活）", () => {
  for (const p of ["deepseek", "cliproxy-local", "anthropic"]) {
    assert.throws(
      () => resolveSpecDrafter({ WAYFINDER_SPEC_PROVIDER: p, WAYFINDER_SPEC_MODEL: "gpt-6-astra" }),
      /不受支持/,
      `provider=${p}`,
    );
  }
});

test("起草子代理调用：Claude Code CLI 无头 -p、禁用全部工具、不落会话，模型显式传入", () => {
  const { file, args } = specDrafterCommand("gpt-6-astra");
  assert.equal(file, "claude");
  assert.deepEqual(args, ["-p", "--bare", "--model", "gpt-6-astra", "--output-format", "text", "--tools", "", "--no-session-persistence"]);
  assert.ok(!args.includes("prime-agent") && !args.some((a) => /deepseek/.test(a)));
});

test("起草子代理 env：剔除继承的 ANTHROPIC_API_KEY（防绕开 CLIProxyLocal），路由变量覆盖宿主值，不改入参", () => {
  const base = {
    PATH: "/fixture/bin",
    ANTHROPIC_API_KEY: "fixture-api-key",
    ANTHROPIC_BASE_URL: "https://fixture-direct.invalid",
  };
  const proxy = { ANTHROPIC_BASE_URL: "http://127.0.0.1:8318", ANTHROPIC_AUTH_TOKEN: "fixture-proxy-token" };
  const env = specDrafterEnv(base, proxy);
  assert.ok(!("ANTHROPIC_API_KEY" in env));
  assert.equal(env.ANTHROPIC_BASE_URL, "http://127.0.0.1:8318");
  assert.equal(env.ANTHROPIC_AUTH_TOKEN, "fixture-proxy-token");
  assert.equal(env.PATH, "/fixture/bin");
  assert.equal(base.ANTHROPIC_API_KEY, "fixture-api-key");
});
