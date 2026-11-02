# #1845 合后关票门核验 46I

结论：#1845 现在**不可关**。代码面与验收 1–9 的证据已齐，缺的只有两件：合后 main `eab5dd10` 的 CI 结论，以及本地分支 `sandcastle/issue-1845` 的处置授权。两件齐了，按 CLOSING-DRAFT.md 发 resolution 即可关。

核验时间：2026-11-02T00:42:44Z（`date -u`）。只读，未删除、未关票、未改账本。

## 票面状态
- #1845 OPEN，assignee `xy7365527-lang`，labels `ready-for-agent`、`wayfinder:impl`。PR1846 没有 closingIssuesReferences，合入未自动关票。
- PR1846 已 MERGED：head `12fa0dc8bfa1bad74d233375edb3c895bfe9069e` → main `eab5dd106c42daf1ee5cbc3898a08898479e7f9b`（squash）。

## 验收逐项映射
| 验收 | 证据 | 状态 |
|---|---|---|
| 1 匹配声明正常监听 | `s_serve_explicit_source_match_listens`（rust/tests/production_session_entry.rs:790 @eab5dd10）；46B job 125000394545 逐名 ok | 满足 |
| 2 库缺声明 fail-closed、launcher failed/1 | `s_serve_explicit_source_rejects_missing_stored_declaration`:718、`production_session_refuses_s_db_without_source_declaration`:843 | 满足 |
| 3 声明不一致点名两侧 | `..._rejects_mismatch_with_both_canonical_declarations`:723、`production_session_refuses_s_db_with_mismatched_source_declaration`:848 | 满足 |
| 4 损坏声明（文件/库） | `..._rejects_corrupt_file`:728、`..._rejects_missing_file`:733、`..._rejects_corrupt_stored_declaration`:760、`production_session_refuses_s_db_with_corrupt_source_declaration`:853 | 满足 |
| 5 standalone 不带声明保持原合同 | `s_serve_standalone_without_source_keeps_missing_declaration_contract`:795、`..._does_not_check_corrupt_declaration`:800 | 满足 |
| 6 新库 init 与 serve 同一声明 | `production_session_new_s_init_and_serve_share_source_declaration`:858 | 满足 |
| 7 launcher 不再含 `immutable=1` / `check_existing_s` | `git show origin/main:rust/src/bin/production_session.rs \| grep -n "immutable=1\|check_existing_s"` @eab5dd10 → 无命中（rc=1）；合前 876fab8f 命中 :204/:208/:239/:301 | 满足 |
| 8 #1781 布局拒绝、资金隔离恢复帽、admission 不变 | 源审 45X 1–8 项＋增量 45Z PASS（/Volumes/AgentStorage/l2-review45x/REPORT.md、/Volumes/AgentStorage/l2-delta-review45z/REPORT.md） | 满足 |
| 9 精确 head job 日志新旧测试逐名 ok | /Volumes/AgentStorage/l2-ci46b/REPORT.md：run 41357398565 job 125000394545，生产 24 / TestOnly 22 passed；整 run conclusion=success | 满足 |

## 关票门
- 评审面==关票面：45X 审 57c6d7de，45Z 审 57c6d7de→12fa0dc8 增量；PR 合入的就是 12fa0dc8，无尾部未审提交。46D 核集成：main 876fab8f 与 PR 文件不相交，merge-tree 干净。
- 合后 main CI：**未完成**。CI run 41368375539（head eab5dd10）queued，所有 job 未结束、无失败；DevSkim run 41368375412 in_progress。改动含 Rust，关票声明须写 `origin/main @ eab5dd10` 及该 run 结论。
- 报告实体：本报告和 45X/45Z/46B/46D 均在 /Volumes/AgentStorage，未入仓。关票门第 6 子句要求报告入 git 的，按本仓先例由归档 PR 入 `.chanlun/review-results/`；若主控判定外盘路径足够，需在 resolution 中写明。这是主控裁定项，本报告不替代。

## 工位与分支实测
- 工位 `/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1845`：**已不存在**，`git worktree list` 无 1845 项。workers.jsonl 末条为 2026-11-01T22:28:55Z `phase=sandbox status=closed`（manager /root/l2_fix45y）。删除者未查明；不需要再清理。
- 远端分支 `origin/sandcastle/issue-1845`：**已不存在**（`git ls-remote` 空，fetch 报 couldn't find remote ref），推测合入时自动删分支。
- 本地分支 `sandcastle/issue-1845`（宿主仓 /Users/silencehan/Projects/NewChanlun-sandcastle-host）：存在，指向 `12fa0dc8`，即合入候选；squash 合入，所以不是 main 祖先，但内容已入 main。
- 恢复引用 `refs/archive/issue1845/reviewer-wip` → `6bf4797021`：存在。

## 清理授权
gates.md 里唯一 1845 相关项是 `recover1845-wip45u`（恢复两处 WIP 至 f441e9f 并受管重放），不覆盖删除分支或工位。**没有具名清理授权。**

建议：本地分支 `sandcastle/issue-1845` 先保全 `refs/archive/issue1845/12fa0dc8` 再删；`refs/archive/issue1845/reviewer-wip` 保留。若要关票时不处置分支，也可在 resolution 写“本地分支保留，内容已由 squash eab5dd10 入 main”。哪条路由主控或用户定。

## 不在本票内
本票只交付 launcher 不开 S 库、S 在写锁下自核声明。不完成任何 Destination，不关 #1279；#1823 残留 s.socket 另票。
