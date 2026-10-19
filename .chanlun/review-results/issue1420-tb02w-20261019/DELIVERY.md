# #1420（TB-02-W）交付报告：第二种候选三态终点、K1 续扫与跨 cut 检查点

> 接续负责人 `/root/owner_1420`（父线程 `/root`）。分支 `codex/1420-scan-boundary-onmain-20261019`，基点 main `5baeb030a6`（含 #1405 PR #1553 与 #1563 PR #1564；实测时基点为 `4aea4f72ef`，其后无冲突 rebase，Rust 与 s_session 源码不受影响）。原实施会话的 11 个提交在 2026-10-14 已过独立评审（W1/W2 两项见证已固化为夹具），本轮把它们原样 cherry-pick 到当前 main，合成会话规则，并补齐验收条件 3、4 的正式会话实测。收据在 `receipts/`，大件原件在 Windows 主机 `windows-5090:~/nc-remote/tb02w/20261019-193207/`。

## 结论

线段第二种终结候选按 #1344 裁定（`.chanlun/definitions/xianduan.md` §5.1）只有三个语义终点：第二特征序列先成分型为 CONFIRMED，原方向先严格越过候选极值为 KILLED_BY_NEW_EXTREME（`078-第78课.md:52`，失效后不复活、原段不终结），已知前缀内二者都未发生为 OPEN_AT_PREFIX_END（不是否定）。续扫用 K1 每候选折叠游标，扫描覆盖与语义状态正交，预算是具名版本 `second-seq-scan-budget/1` 的调度参数，生产值 0（不截断）。S 段合同升 `s-segment/3`，会话规则与 #1405 合成为 `s2-axis-quantifiers+new-bi/2+segment-second/2`，旧规则会话 AsKnown 只读。

正式会话入口实测（Windows 主机 WSL，release 二进制，launcher + socket 逐根接纳 + HTTP 公共查询 + 真 Chromium 读既有页面）在失效与确认两个设计、上下两个方向上，每个手算判定点都对上；同一输入新进程重放 40/40 个 cut 全字段哈希相同；中间消息 SIGKILL 后 recover 续跑的对拍见验收矩阵第 4 行。

## 提交

| 提交 | 内容 |
|---|---|
| `8badf07694`…`f06388c8be`（11 个） | 原实施，cherry-pick 自 `codex/1420-scan-boundary-20261014`（`8e940314ad`…`7a5a8c6159`）。手工合并一处在 `1570f74209`：Lean 引文行保留 main 已订正的 071 正本行，加 078:52 指路行 |
| `c37dd813b2` | 原 `903369bb4d`，冲突在会话规则表：#1405 的 `new-bi/2` 与本票的 `segment-second/2` 合成 `…+new-bi/2+segment-second/2`，绑 `s-new-bi/2`、`s-segment/3`、知识域 V2；Rust/Python/JS/页面四处同步 |
| `3b55b45f6e` | `test_tb02w_contract` 的 py/node 两套按合成规则改常量，接入 s-observer CI |
| `b64424c392`…`8a6aea67b8` | 正式会话验收脚本 `s_session/tests/tb02w_runtime.py` 与 `tb02w_browser.cjs` 及其修订（见「验收脚本的几处取舍」） |

## 验收矩阵

| # | 票面验收 | 结果 | 证据 |
|---|---|---|---|
| 1 | 旧 50 外才出现有效证据、耗尽/续扫/恢复分支；获准预算并绑定版本 | 通过。第 51 项证据不被当否定、资源截断候选不被跳过；窗口 0/50/200 不改已判语义；预算耗尽在下一 cut 先追加再续扫，若干轮后与预算 0 逐位相同；游标与 `SECOND_CANDIDATE_RULE_REVISION=second-candidate-terminal/1`、`SECOND_SEQ_SCAN_BUDGET_POLICY`、`second-fold-cursor/1` 绑定 | `tb02w_ast_048_01_*`、`tb02w_ast_048_02_*`、`tb02w_k1_budget_grant_*`、`tb02w_store_rounds_match_batch_and_grant_exhausted_cursors`、S 会话 `tb02w_killed_case_two_scan_budget_resume_and_recover`；WSL `cargo test --release --lib theta_v0::parser` 147/0/9，CI s_session 步通过 |
| 2 | 分批/恢复与无中断扫描逐位对拍，不丢第二种终结与独立特征序列事实 | 通过。批处理/增量/检查点恢复三路逐位相同（预算 0 与 5、两向）；99 笔长缺口链 `from_checkpoint` 每步等于全量（W1：无边界的 `from_full` 在 q=99 发散）；试探中耗尽阻塞封段（W2） | `tb02w_three_way_batch_incremental_restore_bit_exact`、`tb02w_checkpoint_restore_long_gappy_bit_exact`、`tb02w_k1_resume_and_restore_equal_uninterrupted_fold`、`tb02w_k1_wire_restore_verifies_then_resumes`、`tb02w_w1_*`、`tb02w_w2_*` |
| 3 | 正式会话入口非真空输入、公共查询/变化、浏览器核对 | 通过（详见下节）。四个用例 A 臂各 40 个 cut 全过 Python 合同；浏览器在 open/decided/final 三个 cut 读到 `OPEN_AT_PREFIX_END`/`PENDING_CASE_TWO`、`KILLED_BY_NEW_EXTREME`/`killed_by`/`KILLED_CASE_TWO`/`second_candidate_killed_by_new_extreme`（或 `CONFIRMED`/`CASE_TWO`），0 page error | `receipts/EVAL-*.json`、`receipts/browser-*/` |
| 4 | 独立 oracle；同输入新进程重放与持久边界恢复对拍 | 手算判定点 PASS；新进程重放 40/40 cut 全字段与语义两层哈希相同；SIGKILL+recover：语义层除故障代外 40/40 相同；故障代（第 21 代）只有两条版本对象的 `known_at.semantic_commit_ns` 不同（首次尝试 `…000200000000` 与恢复后重试 `…001200000000`，由时钟计划决定），去掉该字段后 40/40 相同 | `receipts/TB02W-SESSION-CHECK-*.json` |

## 正式会话实测

二进制：分支 HEAD 上 `cargo build --release --features s_session --bin s_structure_session`（Rust 源码自组合提交起未变）。输入：#1414 固定生成器从端点生成的整数 OHLC，失效设计 `[0,20,10,40,25,35,22,45,30,38]`、确认设计 `[0,20,10,40,25,35,22,30,23,33]`，各 40 根、每根一代；DOWN 为 10000 镜像。

| 用例 | 臂 | cut | 手算判定点 | `segment:1:1` 状态序列（首次出现的 cut） | 合同逐 cut | 浏览器 |
|---|---|---|---|---|---|---|
| killed_up | A 不中断 | 40 | n=6@27、n=7@31、n=9@39 全 PASS | 15 NO_FIRST → 27 PENDING_CASE_TWO / OPEN_AT_PREFIX_END → 31 KILLED_CASE_TWO / KILLED_BY_NEW_EXTREME（至 40 不变） | passed | passed（open 27、decided 31、final 40） |
| killed_up | B 新进程重放 | 40 | 同 A | 同 A | passed | — |
| killed_up | C 第 21 根 SIGKILL→recover | 40 | 同 A | 同 A | passed | — |
| killed_down | A | 40 | 同上 | 同上 | passed | passed |
| confirmed_up | A / B / C | 40 | n=6@27、n=9@39 PASS | 15 NO_FIRST → 27 PENDING_CASE_TWO / OPEN_AT_PREFIX_END → 39 CASE_TWO / CONFIRMED，终点组锚 13、笔 2、价 6000 | passed | A passed |
| confirmed_down | A | 40 | 同上（价 4000） | 同上 | passed | passed |

失效 cut 上的 `killed_by` 为 `{stroke_idx: 6, price: 6500, candidate_extreme: 6000, comparison: ">"}`（DOWN 为 3500 / 4000 / `<`），段带 `second_candidate_killed_by_new_extreme`；扫描 `coverage=COMPLETE`、预算 `granted=null`、策略 `second-seq-scan-budget/1`，游标 `second-fold-cursor/1`、`rule_revision=second-candidate-terminal/1`。三臂之间：B 与 A 逐 cut 全字段哈希 40/40 相同（killed_up、confirmed_up 两例）；C 见验收矩阵第 4 行。全部 Tb02aRun 结果为 `CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER`，旧 cut 重访与首访完整候选相同，进程与端口按 launcher 收据回收。原件（每臂 `semantic-core.jsonl` 约 1.3 GB、库、HAR、截图）留在 `windows-5090:~/nc-remote/tb02w/20261019-193207/`，打包摘要 `G:\AgentStorage\nc-remote\tb1420-receipts.tgz`。

## 验收脚本的几处取舍

- **期望值来源**：`tb02w_runtime.py` 的 `EXPECTED` 按笔逐步手算（注释写明每个判定点的依据），输入由 #1414 的固定生成器 `fixtures/tb02d/generate-input.py::raw_rows` 只从端点生成，不导入生产代码。cut 约定同 tb02d：笔 i 在已接纳 4i+7 根时首次已知。
- **长度上限 40 根**：正式 ingest 一条消息只许一事件（`InvalidDomain：ingest必须一事件`），每根一代；Q 冷校验在本机实测第 43 代越过 `verify_deadline_ms=5000`（`QueryBudgetExceeded`，即 #1497 登记的超线性）。所以失效用例收到 9 笔：失效后旧「只认分型」的复活诱饵（要到 s10）和 A 由第一种情况终结（要到 s9 补出 FIRST 顶分型）两件事不在本脚本，由 Rust 锁 `tb02w_killed_candidate_never_revives_incremental_and_from_full` 与 S 会话测试 `tb02w_killed_case_two_scan_budget_resume_and_recover` 承担。Q 资源一项未改。
- **生产预算 0**：正式会话入口的扫描预算是生产值 0（`tb02c::ROUND_BUDGET`），跨 cut 的预算耗尽/追加分支只在 TestOnly 预算下可达，由验收 1 的 Rust/S 会话测试承担。正式入口这里实测的「跨 cut」是候选在 open cut 未决、后续 cut 定终点。
- **宿主环境**：WSL 非 tty 下 procps 的 `ps` 向 stderr 报 `screen size is bogus`，`s_service_control.process_identity` 会把它当成状态不明；运行脚本 `export COLUMNS=4096 LINES=50` 规避，不改产品代码。浏览器为 Playwright chromium-headless-shell 156.0.8078.4；观察等待放宽到 60 s（只是读页等待，不碰 S/Q 资源）。

## tb02d 夹具审计（#1344 第三问回填）

tb02d 七个设计（`second_open`、`second_nondual`、`second_inclusion`、`second_late_close`、`strict_c_closed`、`first_extension_pending`、`no_gap_control`）在 s3（P=40）之后的端点最大值分别为 35/39/38/35/35/35/35，全不严格越过 P，处于 `078:52` 中性域；期待值无需按新规则重算。`test_tb02d_*` 五模块在本分支通过。

## CI

rebase 前的 dispatch 运行 [39236762674](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39236762674)（组合提交，基点 `4aea4f72ef`）：test / fixture-drift / s-observer 通过，rust-check 内 cargo check（默认、backtest_bin）、fmt、TB-02-C、s_session 正式结构会话通过；`default targets` 与 `backtest_bin` 两步 lib 测试第 2 次尝试为 2827/1 与 2833/1，唯一失败是 main 既有红 `calibration_sweep_finds_non_collapsed_segments`（#1563）。第 1 次尝试 backtest_bin 另有 `incremental_tower_scaling_dominates_full_synthetic` 计时比 0.743 > 0.7，重跑通过，判计时抖动。main `4aea4f72ef` 的 CI 39226675505 为 m3 + calibration 两红；**本分支上 `m3_partition_synthetic_holds` 转绿**（#1563 的第一项）。`default targets` 在 lib 处先红，其后的集成目标与当时的 main 一样未执行；#1564 已给该步加 `--no-fail-fast` 并修了 calibration，rebase 后以 PR 上的 CI 为准（见票面评论）。

## 不在本 PR

- 恢复 v4（旧规则 store 只读恢复）：同 #1405 的主控裁定，作独立门，仍 BLOCK_NOT_MERGEABLE。
- `codex/1420-preintegrate-20261015` 的 `bd75a633d4`（空会话先拒未知规则）改的是 #1394 的 `q3-consumer.js`，main 上无此文件，留给 #1394 rebase 时带上。
- 族I 50 窗与余量判据的收编/退役归 TB-10（#1344 第五问）。
