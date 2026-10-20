# REVIEW-TAIL-01 — #1394 关票门第 7 子句尾部补评（快速轴）

- 审查者：独立审查者（非作者），只读；本机未跑 cargo
- 仓：`/Users/silencehan/Projects/NewChanlun`
- 范围：`7e914f1e55`、`77295a9047`、`9b00359052`、`6badaa6049`（前序已由 REVIEW-MERGE-1405-01 覆盖到 `720c0c5337`，REVIEW-MERGE-MAIN-28df-01 覆盖 `2a38b56fb0`）
- 拓扑：`720c0c5337 → 77295a9047 → 7e914f1e55 → 2a38b56fb0(merge 28df886697) → 9b00359052(merge bc401fc5b3) → 6badaa6049(main，第一父 bc401fc5b3)`

## 1. `7e914f1e55` ci：rust-check `timeout-minutes` 60→150 — 通过

- diff 只动 `.github/workflows/ci.yml`：`rust-check` 作业的 `timeout-minutes: 60 → 150`，外加 3 行注释。`test`、`macos-session`、`s-observer`、`fixture-drift` 等其他作业没有改动（`git show 7e914f1e55` 全文 diff 只有一个 hunk，+4/−1）。
- 理由核实（`gh run view`）：
  - 38516525815（`b46ba29699`）：rust-check 05:25:12→07:14:40，约 1h49m。
  - 39221888630（`b333d252c5`）：08:18:58→10:06:06，约 1h47m。
  - 这两次的结论都是 failure，但失败点在 `default targets` 和 `backtest_bin` 两步，后续步骤照常跑完，所以耗时能代表全程。两次的提交都不含 `0c397d20de`（60 分钟上限），因此没有被截断。
  - 39309747120（`720c0c5337`，已含上限）：19:17:39 开始，在 `s_session 正式结构会话（#1370）` 这一步中途被取消（job 于 20:22:39 结束）。其后 6 个 #1394 测试步骤都没有执行。与提交说明一致。
  - 补充证据：本分支合并后的提交 `9b00359052`，其 PR CI run 39348287055 中 rust-check **success**，00:32:53→02:19:28，约 1h46m35s。正常全程在 150 分钟以内，余量约 43 分钟。
- 防挂死上限仍在：150 是有限值，`0c397d20de`（#1406）防遥测采样进程挂死的目的保留，只是阈值放宽。注释写明了来由。
- 附注（不阻断）：余量约 40%。日后 rust-check 再加步骤，需要重新评估这个阈值。

## 2. `77295a9047` docs：只改注释 — 通过

- 只动了 2 个文件：`rust/src/bin/s_session_v3/pending_source_tests.rs`（+2/−1）和 `rust/src/bin/s_session_v3/source_audit.rs`（+3/−1）。增删行全部是 `//` 注释，补记 `813fc75fa5`、`199caedfd0`、`dccb69793c` 三点实测与冻结值逐值相同。冻结常量、断言和代码均无变化。

## 3. `9b00359052` merge main `bc401fc5b3` — 通过

- 父：`2a38b56fb0`（本分支）与 `bc401fc5b3`（main）。`2a38b56fb0` 的 main 侧父是 `28df886697`。
- 等价性：将 `git diff 9b00359052^1 9b00359052` 与 main 自身变化 `git diff 28df886697 bc401fc5b3` 按 `-U0` 只比较增删行，结果**完全相同**（17 文件，+4000/−45）。全量 diff 的差别只在 `s_session_v2/mod.rs` 的 blob 索引、hunk 行号偏移（分支侧该文件更长）和一行上下文，没有任何作者手改。提交信息中的 “no conflicts” 属实。
- 引入的 main 提交：#1577（#1576：`a899d33a87`、`d3e7fd9e0e`、`c6bac5ca6f`、`f3e2a7d666`）与 #1578（graph-engine：`0a7cd70a81`…`c0baec7757`）。其中只有 #1576 触及 Rust（`s_session_v2/mod.rs` +113），其余为 `.sandcastle/`、`scripts/wayfinder_engine*`、文档和收据。
- #1576 与本分支 open_db 只读探测（v2/v3 双当前规则）的语义相容性：
  - #1576 只在 `serve` 的 `advance_state != idle` 分支里加了一个判断：`rule_revision != RULE_REVISION` 时，把拒绝文案改为 `VersionMismatch`，并改指 CLI 只读查看或新建代际。闭门不放松。
  - `serve` 在此之前要求 `active()`，即 `protocol_revision == "s-session/2"` 且 5 张控制表齐全，所以 v3 库（`s-session/3`）在到达 #1576 分支前就被拒绝。双当前规则里的 `V3_RULE_REVISION` 不会被 v2 serve 误判为“历史规则”。
  - #1576 文案断言“recover 同样拒绝”，与分支上的 `recover_core`（`s_structure_session.rs:5199-5201`，`rule_revision != RULE_REVISION` 时返回 VersionMismatch）一致。
  - v2 的规则修订历来带 `+segment-second/*` 后缀（见 `tb02a_facts.rs:928-933` 的选表，以及 `RULE_REVISION` 的 `-S` 历史检索：v2 从未取过裸 `new-bi/2`）。因此，“v2 库落在 `V3_RULE_REVISION`、被探测放行为写连接、却被 #1576 判为历史”的交叉情形，现实中不存在。
  - #1576 新测 `v2_serve_stale_begin_on_historical_rule_points_to_walkable_paths` 把规则写成 `new-bi/1`。在分支的 `open_historical_readonly` 下，它走只读连接；测试里关于字节不变的断言，与该函数注释中“只读连接最多留空 `-wal`”的口径一致（测试把空文件与缺失同视）。
  - 实跑：PR CI 39348287055 在 `9b00359052` 上 rust-check success，覆盖 v2 测试。
- 附注（不阻断）：同一 PR 的 DevSkim 结论是 failure。但 main 上 #1577、#1578 的 push 以及 10-19 的 schedule DevSkim 也都是 failure，属于 main 既有状态，不是本分支引入的。

## 4. `6badaa6049`（PR #1573 进 main）— 通过

- 第一父 `bc401fc5b3`，第二父 `9b00359052`，说明合并时 main 没有新提交。
- `git diff 9b00359052 6badaa6049` 为空。两者树哈希同为 `26afd456b5050a6ee82dd8418c72314582c98c5d`，树完全相等。
- `6badaa6049` 的 `ci.yml:140` 为 `timeout-minutes: 150`，第 1 项改动已落到 main。
- 附注：撰写本报告时，main push CI 39358379553 仍为 in_progress。树与 39348287055 测过的完全相同，结论不受影响。

## 结论

尾部四项都与报告一致：CI 超时调整理由成立，挂死上限仍在，没有误改其他作业；docs 提交只改注释；合并 main 无手改，#1576 与双当前规则探测语义相容；进 main 的合并树与 `9b00359052` 相同。与 REVIEW-MERGE-1405-01、REVIEW-MERGE-MAIN-28df-01 合起来，评审面覆盖到关票面 `6badaa6049`。

VERDICT: PASS
