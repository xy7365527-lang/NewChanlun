# graph-engineering 控制面修正报告（2026-10-20）

分支 `codex/graph-engine-fix-20261020`（基于 origin/main `28df886697`），worktree `/Volumes/AgentStorage/engine-fix-20261020`。依据：`/Volumes/AgentStorage/graph-1323-audit-20261020.md` 第 13–15 条。本报告与同目录的 dry-run 日志、对比 JSON、三轮独审原件属工作草稿，活期绑定本 PR。

## 结论

修后控制面能识别 #1340 SPEC 子树（批准态覆盖的扫描票从 23 张升到 111 张），TB 叶子只带 `wayfinder:impl` 也进扫描集；但**当前这一刻修前修后都放行 0 张**，因为子树里没有任何一张同时满足「无 assignee + 无 open blocker」。六张有负责人的票（#1394、#1488、#1493、#1419、#1393、#1527）修后全部判 claimed 或 blocked，且即使 blocker 清零也仍是 claimed（有 assignee），不会被挂 `sandcastle`。decision-claimer 在当前图上修前修后都选不出 grilling 票（全部已认领、被挡或待批），#1565 那种重复处理由单测锁住。

## 改了什么

wayfinder_engine（`scripts/wayfinder_engine.mts`）：

- 扫描集 = open 的 `ready-for-agent` ∪ `wayfinder:impl`；实施票按 `[impl]` 标题或 `wayfinder:impl` 标签认，决策票除外。
- 批准态沿 sub_issues 递归。祖先链上有 SPEC 的票，链上每张 SPEC 都已批准才算批准；没有 SPEC 的 map 直挂票及其子票继承 map 的 SPEC 批准态，并且仍须带 `ready-for-agent` 才进放行队列（独审 MED-1：避免把 #1481/#1491/#1496/#1497 这类未分诊的直挂票一并放出）。
- 不放行：任何 assignee、open blocker、`needs-approval`/`ready-for-human`/`needs-info`/`needs-triage`/`wontfix`、仍有 open 子票的汇总票（TB 汇验、#1340 本身）。挂 `sandcastle` 前重读单票复核一次。
- `isSpecTicket` 兼容「SPEC：」「SPEC:」开头（大小写敏感），原「📋 SPEC」「[spec]」照旧。
- 新增 `--explain`，逐票打印分桶与原因。

口径出处：`docs/agents/wayfinder-workflow.md`「队列交棒」行——控制面只给「已批准 spec/DAG + 未 assign + 无 open blocker」的实装票挂 `sandcastle`，不把历史 `ready-for-agent` 无差别放行；#1340 正文「`ready-for-agent` 表示已批准可拆票，不是可直接拾取」说明 rfa 不能作为叶子的唯一入口。「已有负责人不挂」保持比要求更严的形式：任何 assignee 都不放行，不看 48h 活动；释放死认领仍按审计第 9 条由人工做。该行已同步改为新扫描集口径，细则写在 `.sandcastle/docs/graph-engineering.md`「解锁」。

decision-claimer（`.sandcastle/decision-claimer.mts`）：

- live 也按本地 `summary.json` 跳过终态票（`done`/`land_t`/`shadow_p` 且无断点）；带断点的票照常续做。
- 选票时逐张读票面，跳过已关闭、已有 resolution（首行 AUTH_LINE 或「**resolution**：」）、或编排者账号首行「用户 … 批复」的票，被跳过的票不占并发槽；`processTicket` 入口同样检查（覆盖 `--tickets` 显式指定），跳过时不覆盖既有 `summary.json`。
- 批复判据：去 Markdown 前缀；排除「未/待/没/需」批复和「P 类影子模式」评论；按时间序，句首或「批复：」后以退回/驳回/打回/不批准/不同意开头的评论会重新打开；簿记评论不改变结论。
- 重跑终态票须先改名本地目录（先例 `1530.returned-20261020`），已写入文档。

## dry-run：修前 / 修后（2026-10-19T23:13–23:3xZ 实测，同一张真实图）

| | 修前 | 修后 |
|---|---|---|
| 扫描集 | 53（只 rfa） | 140（rfa ∪ wayfinder:impl） |
| 拿到「SPEC 已批准」的扫描票 | 23（仅 #1323 直挂） | 111（含 #1340 子树） |
| 本轮放行 | 0 | 0 |
| 分桶 | blocked 18 / claimed 3 / held 32 | blocked 101 / claimed 7 / held 32 |

修后 held 32 张为：非实装票（决策 `[task]`/`[debt]`/SPEC 等）、`needs-approval` 七张（#1530、#1557–#1561、#1567）、图外票、#1340（汇总）。

指定六张票：

| 票 | 修前 | 修后 | assignee | open blocker | blocker 清零后 |
|---|---|---|---|---|---|
| #1394 | 未扫描 | claimed | 有 | 0 | claimed |
| #1488 | 未扫描 | blocked | 有 | 1 | claimed |
| #1493 | 未扫描 | claimed | 有 | 0 | claimed |
| #1419 | 未扫描 | blocked | 有 | 1 | claimed |
| #1393 | 未扫描 | claimed | 有 | 0 | claimed |
| #1527 | blocked | blocked | 有 | 1 | claimed |

反事实（只把 open blocker 清零、其余不变）：修前只有 #1572 会被放行；修后 66 张会被放行（#1375–#1446 一带 TB 叶子、#1523、#1525、#1552、#1572，名单见 `dryrun-compare.json` 的 `ifUnblocked`）。这就是修正的实际效果：前置一关，叶子自动进执行面，不再需要主控手挂 `sandcastle`。

decision-claimer 选票（maps #1323、#1340）：修前 live 本轮 `[]`，修后 `[]`；本地终态跳过 #1530、#1531、#1536、#1537、#1540、#1557–#1561、#1567。#1565 已关闭。

## 验证

`npx tsx --test scripts/wayfinder_engine.test.mts scripts/wayfinder_engine.shell.test.mts .sandcastle/decision-claimer.test.ts .sandcastle/claim-loop.test.mts`：93 通过、0 失败（origin/main 基线 74 通过；原有用例全部保留，三条断言随日志措辞与夹具默认标签更新）。新增用例覆盖 #1323 形状四层子树、嵌套 SPEC 驳回、环、直挂票 rfa 要求、汇总票与截断兜底、写前复核不发写命令、批复判据正反例、断点续做 × 跳过（`reviewed_pass` 评论已发出但 gh 报错；`shadow_p_pending` 补贴标签）。dry-run 均为只读，未写 GitHub。未做类型检查（仓内无 typescript 依赖，tsx 运行通过）。

## 非作者独审

本机 claude CLI，`claude-opus-5-5` 经 CLIProxyLocal `127.0.0.1:8318`，只读工具，三轮：

- 第一轮 PASS，MED 4 / LOW 6，全部处理（原件 `review-round1.md`）。
- 第二轮 PASS，核对第一轮 10 条中 9 条已处理；新发现 N1（MED，「重写」一词会把已批复票误重开）与 N2–N4（LOW），第三个提交处理（原件 `review-round2.md`）。
- 第三轮 PASS，N1–N4 均已处理；LOW-R3a（「批复：**不同意**」「批复：否决」认不出）已在最后一个提交补上，LOW-R3b（「用户 … 退回」不含「批复」不触发重开）按安全方向保留（原件 `review-round3.md`）。

## 限制

- 引擎仍按「任何 assignee」挡放行，死认领（#1406、#1424、#1426 等审计第 9 条名单）要人工释放后才会进放行。
- 网络层的子树递归（`fetchChildren`）只经 dry-run 实测，没有单测。
- 合入后由主控快进宿主 `NewChanlun-sandcastle-host`；本 PR 未动宿主与 launchd。
