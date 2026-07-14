# Agent Roster — 2026-07-10

本表登记 #40 bughunt phase2 对抗核验工位（本 session）派发的所有后台 agent。

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| general-purpose | sonnet | 对抗核验 bughunt-parser-20260710.md 全部 12 项（BUG-01..12），锚定快照 bf1854d71960，只读 | 完成 |
| general-purpose | sonnet | 对抗核验 bughunt-cert-20260710.md 全部 16 项中 15 项（F-12 由本体亲验），含裁决①②复核，只读 | 完成 |
| general-purpose | sonnet | 对抗核验 bughunt-backtest-20260710.md 中 F-01/02/03/04/06/07/11（F-05/08/09/10 高危由本体亲验），只读 | 完成 |
| general-purpose (a1e841c3fc7891c69) | claude-fable-5 | #40 phase3 修复：worktree `/private/tmp/bughunt-fix-work`，分支 bughunt-fix-20260710，逐 bug 一 commit | 完成（15 commit，B 列 10 项全处理，回归绿；产出降级待 codex 复核） |
| codex exec (bg b25hwf89p) | gpt-5.6-sol | bughunt-fix-20260710 全量 diff 复核（逐 commit：B 列范围 / A 列禁修面·冻结基线 / 修复正确性），只读 | 完成：7 PASS / 8 FAIL（日志 /tmp/codex-review-bughunt-fix.log:15302 起） |
| codex exec (bg) | gpt-5.6-sol | bughunt-fix-v2 第一次派发（worktree 方案） | 失败：沙箱无法写主仓 .git（worktree 元数据越界），无仓库改动 |
| codex exec (bg bac→r2) | gpt-5.6-sol | bughunt-fix-v2：自包含 clone /tmp/bughunt-fix-v2-work（基线 bf1854d71960），cherry-pick 7 个 PASS commit + 重做 F-05/cert F-01/cert F-05/BUG-11；剔除触 A 节三项；BUG-10 待裁定 | 完成：v2 分支 bughunt-fix-v2（头 1dcdb95a09）已接回主仓；7 cherry-pick + 4 重做，全量回归 1591 passed/0 failed；报告 chanlun/review-results/bughunt-fix-v2-20260710.md |

产出汇总：`chanlun/review-results/bughunt-verify-20260710.md`

## 违规记录（20:42 补记）
1. **模型路由违规**：phase2 对抗核验（应走 `codex review`）与 phase3 修复（应走 codex-implementation / `codex exec`）均由 Claude 子代理执行，违反路由表"战术任务一律派 codex GPT-5.6 起"。
2. **登记违规**：本表未随派生实时建立/更新（phase1 codex fanout 与 phase3 修复代理均漏记）。

## 纠偏
- phase3 代理无法中断，其产出降级为"待复核材料"：停机后由 `codex review`（gpt-5.6-sol）对分支全量 diff 复核，通过后方可进入人工评审；不通过项逐条回滚。
- 后续所有实现/审查类派生一律走 codex；Claude 仅允许作 spawn 壳（名加 [5.6] 前缀）。

## 追加登记（P0 复跑派生）
| 时间 | 代理 | 模型/路由 | 任务 | 工作区 | 状态 |
|---|---|---|---|---|---|
| 21:59 | codex exec (b0vt3rt18) | gpt-5.6 xhigh, workspace-write | #41 P0 复跑：J_parent:=D_parent + I(A)⊆D_parent + λ_C 仅登记；cherry-pick 7a571d1a38；冻结文档→实现→复跑→测试→报告 p0-replay-dparent-20260710.md | /tmp/p0-replay-work2（分支 p0-replay-dparent @ 1dcdb95a09） | done(PASS, HEAD 99ce554b8a) |
| 2026-07-11 | codex exec (bitn8eymq) | gpt-5.5 xhigh, workspace-write | #42 深研A：L1→L2 0/0 代码审计+反例构造（enter_src 计算路径、近失样本剖检、层级配对） | /tmp/p0-replay-work2 | running |
| 2026-07-11 | codex exec (bfsuh6il7) | gpt-5.5 xhigh, workspace-write | #42 深研B：D_parent 左端定义谱系审查（原文24/27课 vs 冻结口径） | /tmp/p0-replay-work2 | running |
