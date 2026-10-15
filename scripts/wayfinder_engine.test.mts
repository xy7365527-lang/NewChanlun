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
  withSpecApproval,
  summarizeQueue,
  planReleases,
  renderStatus,
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

test("isSpecApproved：否定/延后/技术通过不得误触发闸一", () => {
  const hostile = [
    "not approved yet, need another grilling round",
    "I won't ship it yet, needs more review",
    "don't ship it",
    "waiting for approval",
    "测试通过了，spec 我再看",
    "CI 通过，逻辑还要改",
    "等我看完再批准",
    "稍后批准",
    "> @xy7365527-lang 请批【闸一】：批准后引擎自动拆实装票（blocking 边 + 逐条挂裁定票 + sandcastle 标签）。\n\n先看一眼。",
  ];
  for (const body of hostile) {
    assert.equal(
      isSpecApproved(spec(1, "OPEN", [{ author: ORCH, body, createdAt: "2026-08-18T12:00:00Z" }]), ORCH),
      false,
      `不应批准：${JSON.stringify(body)}`,
    );
  }
});

test("isSpecApproved：真批准与「技术通过 + 真批准」仍放行", () => {
  assert.equal(isSpecApproved(spec(1, "OPEN", [approvalComment()]), ORCH), true);
  assert.equal(
    isSpecApproved(
      spec(1, "OPEN", [{ author: ORCH, body: "approve", createdAt: "2026-08-18T12:00:00Z" }]),
      ORCH,
    ),
    true,
  );
  assert.equal(
    isSpecApproved(
      spec(1, "OPEN", [{ author: ORCH, body: "ship it", createdAt: "2026-08-18T12:00:00Z" }]),
      ORCH,
    ),
    true,
  );
  assert.equal(
    isSpecApproved(
      spec(1, "OPEN", [{ author: ORCH, body: "测试通过了。批准。", createdAt: "2026-08-18T12:00:00Z" }]),
      ORCH,
    ),
    true,
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
    labels: [],
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
  assert.equal(text.includes("全仓 ready-for-agent（open）：2 张"), true);
  assert.equal(text.includes("已获 sandcastle 拾取权（可拾取）：0"), true);
  assert.equal(text.includes("待释放（合资格实装票，本轮挂 sandcastle）：1 → #1"), true);
  assert.equal(text.includes("不合资格（决策/spec/未批准 spec 等，不放行）：1"), true);
});
