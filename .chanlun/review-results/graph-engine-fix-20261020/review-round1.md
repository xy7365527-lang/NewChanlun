**总结论：PASS**。两条硬约束都守住了：已有负责人的票和有 open blocker 的票一律不会被挂 `sandcastle`。我没找到会导致误放行的 HIGH 级问题。下面几条 MED 是建议在合并前或紧接着补上的口径缺口。

**我做了哪些验证（全程只读）**
- 四个测试文件一起跑：86 个测试全部通过，0 失败。
- 用 PR 代码对线上 tracker 做了一次只读模拟，只调用读取函数，没有写入：
  - 4 张 open map，扫描集 140 张票。
  - 分桶结果：blocked 101、claimed 7、待释放（eligible）0、不放行（held）32。**当前没有任何票会被放行。**
  - #1340 被识别为已批准 SPEC，它下面的 TB 叶子、TB 汇总票、纵片全部正确拿到"已批准"状态。
  - 已 assign 的票（#1394、#1493、#1524、#1576 等）都分进 claimed；有 open blocker 的都分进 blocked。
  - 所有 open map 里没有哪张票是因为新增的「SPEC：」写法才首次被认成 SPEC（#1340 本身标题就是「📋 SPEC：」），所以 `analyzeMap` 的起草和拆票行为目前不受影响。

---

### MED

**1. 新扫描集会放进来一批没被标成可接手、也不是从 SPEC 拆出来的实装票**
- 位置：`scripts/wayfinder_engine.mts:503`（继承规则）和 `:723`（扫描集）
- 原因：map 直挂的票继承 map 上 SPEC 的批准态，这是原有口径。但以前只扫 `ready-for-agent`，这类票实际上进不了放行队列；现在加扫 `wayfinder:impl`，它们就进来了。线上实例是 #1481、#1491、#1496、#1497、#1572：都挂在 #1323 下面，没有 `ready-for-agent`，也不是从 #1340 拆出来的。它们的 blocker 一关，下一轮就会被挂 `sandcastle`。这符合「队列交棒」行的字面意思，但比 PR 想修的范围（#1340 的 TB 叶子）大得多，而且 PR 说明里没提。
- 建议：二选一，并写进 `graph-engineering.md`：
  - 只有「祖先链上真有一张 SPEC 节点」的票，才因为带 `wayfinder:impl` 进扫描；map 直挂的票仍然要求有 `ready-for-agent`。
  - 或者在说明里明确写「map 直挂的 impl 票一律随 map 的 SPEC 放行」，并且由人点头。

**2. 人闸标签里漏了 `needs-triage`**
- 位置：`scripts/wayfinder_engine.mts:161`
- 原因：以前扫描集要求有 `ready-for-agent`，而按 triage 状态机它和 `needs-triage` 互斥，所以没还没分诊的票进不来。现在 `wayfinder:impl + needs-triage` 的票，只要在已批准 SPEC 的子树里就会被放行。线上目前没有这种组合，但 /to-tickets 新拆的票很可能就是这个状态。
- 建议：把 `needs-triage` 加进 `HOLD_LABELS`，并补一条测试。

**3. 「用户…批复」的判据太依赖格式，两个方向都可能错**
- 位置：`.sandcastle/decision-claimer.mts:603`
- **漏判（危险方向，会重复处理，就是 #1565 那种事）**：判据只看评论开头，而且要求第一个字就是「用户」。如果用户的批复写成 `**用户 2026-10-20 批复**`（加粗）、`> 用户…批复`（引用）或者 `## 用户…批复`，都认不出来，票会被重新处理。线上已经有一个现成例子：#1530 的「## 退回重写（用户 2026-10-20 批复：「同意」）」，这次只是碰巧开头是「## 退回」才没被认成批复。
- **误判（会让票卡死）**：判据也不看是谁写的、批复内容是什么。「用户尚未批复…」或者一条「用户 … 批复：退回重写」都会被当成已经裁定，票从此被跳过，只在日志里留一行。而现在的重写流程本来就要靠用户写批复来触发。
- 建议：
  - 先去掉开头的 Markdown 前缀（`**`、`#`、`>`）再匹配。
  - 只认维护者账号（orchestrator）写的评论。
  - 按时间顺序取最新一条：后面出现「退回重写」或「驳回」时，前面的批复就不算数。
  - 补对应测试。

**4. 测试没覆盖断点续做和"跳过"判断叠在一起的情况**
- 位置：`.sandcastle/decision-claimer.mts:820`
- 原因：
  - `reviewed_pass` 续做：如果 resolution 评论其实已经发出去了、只是 gh 报了错，现在会被正确跳过，这比原来更好。
  - `shadow_p_pending` 续做：影子评论首行是「**P 类影子模式，待用户批复**」，不会被误判，所以补贴标签和取消认领能照常完成。
  
  我逐行读代码确认这两条推理成立，但没有任何测试锁住它们。
- 建议：补两个测试：
  - `reviewed_pass` 的断点，加上票面已有 `AUTH_LINE` 评论 → 结果应为 skipped，不调模型、不写入。
  - `shadow_p_pending` 的断点（`pDone=[comment]`），加上票面有影子评论 → 应当补齐 label 和 unassign。

### LOW

**5. 「最近一张 SPEC 说了算」可能让嵌套 SPEC 绕过上层的驳回**
- 位置：`scripts/wayfinder_engine.mts:503`
- 原因：如果一张已批准的子 SPEC 挂在一张被驳回或还没批准的 SPEC 下面，它的子树照样放行。现有测试只覆盖了「两张 SPEC 平级、都挂在 map 下」，没覆盖「嵌套在未批准 SPEC 里面」。
- 建议：要么改成"沿祖先链所有 SPEC 都已批准才放行"，要么把现在的口径明确写进文档，并补一条嵌套测试。

**6. 影子评论正文可能被误认成 resolution**
- 位置：`.sandcastle/decision-claimer.mts:605`
- 原因：`RESOLUTION_MARK_RE` 用了多行匹配（`/m`），会扫整条评论，包括影子评论里附带的草稿全文。如果草稿里恰好有一行 `**resolution**：`，`shadow_p_pending` 续做时会被跳过：needs-approval 标签永远贴不上，票既没 assign 也不进人闸，就悬在那里没人管。
- 建议：只匹配评论首行，或者先排除以「**P 类影子模式」开头的评论。

**7. live 模式新增按本地 `summary.json` 跳过，改变了重跑方式**
- 位置：`.sandcastle/decision-claimer.mts:1071`
- 原因：主控说"退回重写"之后，如果本地还留着 `shadow_p` 或 `done` 状态的 summary，live 会一直跳过这张票。现行做法是改本地目录名（`/Volumes/AgentStorage/decisions/1530.returned-20261020` 就是先例），所以和新代码兼容；但这条做法没写进文档。
- 次要：`"skipped"` 状态从来不会写进 summary 文件，所以列在已完成状态里是无效代码。
- 建议：在 `graph-engineering.md` 写明「重跑要先改名本地目录」。

**8. 子树和扫描集有几处静默截断**
- 位置：`scripts/wayfinder_engine.mts:642`、`:723`
- 原因：
  - 子树递归最多 8 层（`MAX_TREE_DEPTH = 8`）。超过的节点不会去取子票，于是不知道它还有没有 open 子票，汇总票可能被当成叶子放行。
  - 扫描用 `--limit 500`，超出部分会被静默丢掉；当前约 85 张 `wayfinder:impl`，还有余量。
- gh 调用量（不算问题）：每张有子票的票一次调用，加每张 SPEC 一次评论调用，再加两次列表调用，可以接受。任何一次失败都会让整轮在 `runOnce` 里被跳过（不放行任何票），这个失败处理是对的。
- 建议：达到深度上限时，把带子票的节点直接按"不放行"处理；结果条数等于 limit 时打一条告警。

**9. 放行是先读再写，中间有一个很短的窗口**
- 位置：`scripts/wayfinder_engine.mts:743`
- 原因：`releaseTicket` 挂标签前不会重新读一遍 assignee 和 blocker。在扫描到挂标签之间的几秒里，如果有人刚好 assign 了这张票，它仍然会被挂上 `sandcastle`。这是原有问题，PR 没引入，但和硬约束「绝不挂」有关。
- 建议：挂标签前对单张票再读一次 assignee 和 blocker。

**10. 正本文档和测试的小遗漏**
- `docs/agents/wayfinder-workflow.md:114` 的「队列交棒」行还写着五桶之一是「全仓 `ready-for-agent`」，和新扫描集不一致。PR 只改了 `graph-engineering.md`，没改这份正本。
- 新增的大小写不敏感 SPEC 正则会把小写的 `spec: xxx` 也认成 SPEC。目前线上没有命中，可以考虑只认大写。
- 网络层的递归（`fetchChildren` 遇到 `sub_issues_summary`、visited 防环）和扫描集合并（`fetchReadyTickets`）没有测试，现有测试只覆盖纯函数。
