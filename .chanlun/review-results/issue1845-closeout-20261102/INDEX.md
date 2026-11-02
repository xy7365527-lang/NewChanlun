# #1845 关票证据索引

本目录把 #1845 的审查、行为验收、集成验收和关票门核验报告实体入仓，满足 `docs/agents/delivery-discipline.md` 关票门第 6 子句。各报告按原样复制自外盘工作区，保留写成时的状态与结论；合入后的最终状态以本索引为准。

## 交付对象

- PR [#1846](https://github.com/xy7365527-lang/NewChanlun/pull/1846)，候选 head `12fa0dc8bfa1bad74d233375edb3c895bfe9069e`。
- 2026-11-02T00:40:02Z squash 合入 `origin/main @ eab5dd106c42daf1ee5cbc3898a08898479e7f9b`；合前 main 为 `876fab8fbf2ea5f20611f83690b6d01dcd614e1a`。
- 合入未触发自动关票，#1845 仍 OPEN。

## 报告

| 文件 | 内容 | 原路径 |
|---|---|---|
| `review45x-source.md` | 非作者源审，审 `57c6d7de`，验收 1–8 PASS（Opus 审 gpt-6.1-sol 作者） | `/Volumes/AgentStorage/l2-review45x/REPORT.md` |
| `review45z-delta.md` | 增量审，`57c6d7de`→`12fa0dc8`，PASS | `/Volumes/AgentStorage/l2-delta-review45z/REPORT.md` |
| `ci46b-behavior.md` | 验收 9：精确 head job 新旧测试逐名 ok | `/Volumes/AgentStorage/l2-ci46b/REPORT.md` |
| `ci46b-job125000394545-tests.md` | 上项所依据的 job 日志摘录：45 行单行 `ok`，另有 1 项被资源监控输出拆成两行，收录原日志第 736–743 行原文配对；合计生产 24 / TestOnly 22 passed，无失败 | `/Volumes/AgentStorage/l2-ci46b/production-session.log` |
| `accept46d-integration.md` | 独立集成验收：PR 与 main `876fab8f` 文件不相交，merge-tree 干净 | `/Volumes/AgentStorage/l2-accept46d/REPORT.md` |
| `landing1846.md` | 主控合入回执 | goal1323 store `landing1846-20261102.md` |
| `closegate46i.md` | 合后关票门核验：验收 1–9 逐项映射、工位分支实测 | `/Volumes/AgentStorage/l2-closegate46i/REPORT.md` |

`ci46b-behavior.md` 与 `accept46d-integration.md` 写于候选 CI 未完时，其中“未完成”的 job 后来全部成功：run [41357398565](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41357398565) conclusion=success，DevSkim run 41357398552 success。

## 评审面与关票面

45X 审至 `57c6d7de`，45Z 审 `57c6d7de`→`12fa0dc8` 增量；合入候选即 `12fa0dc8`，无尾部未审提交。

## 分支与工位

- 工位 `.sandcastle/worktrees/sandcastle-issue-1845` 与远端分支 `sandcastle/issue-1845` 在 46I 核验时均已不存在；删除原因未查明。
- 宿主仓本地分支 `sandcastle/issue-1845` 指向 `12fa0dc8`，目前保留；内容已经由 squash `eab5dd10` 进入 main。删除未获授权。
- 关票门第 5 子句要求关票时处置本票关联的 worktree/branch（票关即残骸，残骸即删）。这一本地分支尚未处置，**第 5 子句未结清**，在它处置之前不能宣称关票门通过。
- 恢复引用 `refs/archive/issue1845/reviewer-wip` → `6bf47970` 保留。

## 对 `closegate46i.md` 的订正

`closegate46i.md` 按原字节保留，其中两处建议已被取代：

- 它提出“关票时不处置分支，也可在 resolution 写本地分支保留”。该做法不满足关票门第 5 子句，不能作为关票依据；分支处置仍是未结清项。
- 它提出“若主控判定外盘路径足够，在 resolution 中写明”。主控已选择把报告实体入仓，即本目录；外盘路径不再作为替代。

## 关票前仍需

- 本地分支 `sandcastle/issue-1845` 按第 5 子句处置（需删除授权）。

- 合后 main CI：2026-11-02T00:45:42Z 查询时 run [41368375539](https://github.com/xy7365527-lang/NewChanlun/actions/runs/41368375539) queued，DevSkim run 41368375412 in_progress，均针对 `eab5dd10`。关票声明须写入其最终结论。
- 本归档 PR 本身需非作者审查与 CI 后合入，关票声明引用合入后的路径。

本票只交付 launcher 不再打开 S 库、S 在写锁下自核声明这一段，不完成任何 Destination，不关闭 #1279。
