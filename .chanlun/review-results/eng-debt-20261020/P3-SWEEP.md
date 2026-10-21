# p3sweep — #1588 第 5 项：各票登记的 P3/观察项中「当前就错」的部分

- 基线：`origin/main` = `bf871af9ce`（#1589 合入后）；所有代码复核均用 `git show origin/main:<path>` / `git grep … origin/main`，未在本机跑 cargo，也未调用 nc-remote（静态证据 + CI 日志已足够）。
- CI 证据：main `1d70db893f` 的 rust-check run [39380862852](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39380862852)（job 118489420938，success），日志本地副本 `/tmp/p3sweep/rustcheck-1d70.log`。
- 范围：2026-09-20 以后有活动的 issue 144 张、PR 51 个（含已关；原文落 `/tmp/p3sweep/raw/`），重点 #1323 子票、#1394、#1405、#1419/#1420、#1488、#1493、#1495、#1527、#1563、#1568/#1570、#1576、#1580–#1597 及其相关 PR（#1473/#1477/#1480/#1482/#1485/#1574/#1577/#1589）与 `.chanlun/review-results/` 下的评审报告。
- 已排除（其它分片处理）：DevSkim（PR #1593）、CI 时长（#1592）、基线失败测试 / financial_session / m3（#1594）、解析内存（#1596/#1597）、nc-remote 共享 target 污染。
- 类别：**(a)** 当前就错，main 上可复现；**(a?)** 疑似当前就错，证据不全；**(b)** 设计/性能观察；**(c)** 已修；**(d)** 已有专票。

## (a) 当前就错

| # | 来源票与评论链接 | 现象 | 类别 | 证据（origin/main bf871af9ce） | 建议处置 |
|---|---|---|---|---|---|
| A1 | PR #1480 Codex P1 [r4260955949](https://github.com/xy7365527-lang/NewChanlun/pull/1480#discussion_r4260955949)（未回复、未修） | Sandcastle codex-child 的 `git()` 把完整 `process.env` 传给 preflight git 子进程，`CLIPROXY_LOCAL_KEY` 等 provider key 随之泄给 git 及 git 调起的 hook、credential helper 和 fsmonitor。PR 正文声称「密钥只进 Codex 子进程环境」，与实现不符 | (a) 安全/文档与代码不符 | `.sandcastle/codex-child.ts:124-131`：`env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" }`；调用点 :569、:581（preflight）。对照 `workerEnvironment`（:207）走 `PROVIDER_KEY_ENV` 白名单，说明设计意图是隔离 | **可当场小修**：`git()` 改用剔除 provider key 的 env（或复用最小白名单）。补一条断言「git 子进程 env 不含 PROVIDER_KEY_ENV」 |
| A2 | PR #1485 Codex P2 [r4264297314](https://github.com/xy7365527-lang/NewChanlun/pull/1485#discussion_r4264297314)；PR #1592 未处理 | ci.yml 的诊断/上传步骤用 `if: always()`：run 被取消（手动取消或新 push 覆盖）时这些步骤仍会执行，取消要等它们跑完。工作流还没有 `concurrency:` 组，同一分支的旧 run 不会被新 push 自动取消，叠加 `timeout-minutes: 150` 后会长时间占 runner | (a) CI 配置错（低危） | `.github/workflows/ci.yml` 中 `if: always()` 位于 :270、278、282、291、298、304、308（rust-check）和 :355、359（s-observer）；全文件 `grep -n concurrency` 无结果；:140 `timeout-minutes: 150` | **转现有票 #1592**（CI 时长分片），或当场小修：`always()` → `!cancelled()`，加 `concurrency: {group: ${{ github.workflow }}-${{ github.ref }}, cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}}` |
| A3 | PR #1482 Codex P2 [r4262772366](https://github.com/xy7365527-lang/NewChanlun/pull/1482#discussion_r4262772366)（同类 [r4262772347](https://github.com/xy7365527-lang/NewChanlun/pull/1482#discussion_r4262772347)） | 教义正本 `bi.md` 的小节标题写「机读 AFFECTED-CODE.json」，但仓库中没有这个文件，只有 `.chanlun/review-results/issue1323-grilling-20261013/1343-AFFECTED-CODE.md`。按正本指针找机读清单的流程会扑空 | (a) 文档悬空指针 | `.chanlun/definitions/bi.md:338`：`#### #1343 受影响代码清单（基线 main 3da0b2c8；机读 AFFECTED-CODE.json）`；`git ls-tree -r origin/main` 全树无 `AFFECTED-CODE.json` | **可当场小修**：改为指向 `1343-AFFECTED-CODE.md` 的实际路径（或补 JSON）。注意 `bi.md` 是教义文件，改动按教义文件流程走 |
| A4 | `.chanlun/review-results/issue1394-ac123-closeout-20261019/REVIEW-MERGE-1405-01.md` L-4 | `content_producer.rs` 中的 doc comment 仍写「旧生成路径……就不给分型选根」，但该函数在 #1405 后只作覆盖断言，已不参与生成，注释误导读者 | (a) 文档与代码不符（低危） | `rust/src/bin/s_session_v2/content_producer.rs:3748` 注释；唯一用途在 :4305-4309（注释「#1405：多根极值仍覆盖，但不再生成待决形态请求」） | **可当场小修**：改写注释为「覆盖断言用」 |
| A5 | #1419 [issuecomment-6247532193](https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6247532193)（「should be tracked separately if warning-free acceptance is required」，未另开）；历史专票 #913 已于 2026-08-16 关闭，告警已回涨 | main 编译告警大量回涨：lib 81 条、lib test 84 条、s_structure_session test 159 条，另有 p*/strict_nest_check 等 bin。按类型计：933 条 never-used 函数、271 条 unused import、267 条 never-used 方法、151 条 unused variable、132 条 never-read 字段、73 条 deprecated、14 条 unexpected cfg、14 条 unused must_use、14 条 unreachable，以及 1 条 mixed-script confusable identifier。其中下列几类不是噪声，属于真信号（明细见下方 A5 附表） | (a) 编译告警 | CI run 39380862852 日志（main 1d70db893f）。重点位置：`fetch_update` 已 deprecated（改名 `try_update`），见 `session_transport.rs:621`、`s_content/capture_allocation/scratch.rs:125/187/201/216`、`bin/s_session_v3/audit_owner/native.rs:527/537/578`；unexpected cfg `issue1394_capture_callback_refusal`（`s_session_v2/content_store.rs:3055`）和 `issue1394_closed_owner_refusal`（`s_session_v3/audit_store.rs:989`），这两个 cfg 未在 `check-cfg` 声明，对应的拒绝路径代码在任何构建下都不会编译；测试中 7 处 `progress_handler(...)` 返回的 `Result` 被丢弃（`s_session_v3/driver.rs:2415/2745`、`prepared.rs:1710`、`request_audit.rs:1118`、`s_allocation.rs:404`、`cross_request_source_cache.rs:1608/1614`）；unreachable else 3 处（`s_session_v3/object_index.rs:1887`、`audit_owner/q.rs:398`、`q_content.rs:222`） | **新开票**（承接已关的 #913 口径：先量再动，dead_code 中的探针不盲删）。其中 deprecated、unexpected cfg 和 unreachable pattern（A6）三类可单独先修。是否在 CI 上加 `-D warnings` 由新票裁定 |
| A6 | 由 A5 的 `unreachable pattern` 告警追出（无票登记，本分片新发现） | 第一类漏斗探针 `type1_funnel_dx` 的 broke 判据把「上涨趋势向上破 zg」那一臂误写成 `(Some(Down), Down)`，与上一臂完全重复，因此不可达。结果是漏斗从不计入 sell1 一侧。函数尾部的 parity 断言 `assert_eq!(prod_t1, f.s_diverge, "漏斗环7（背驰）须=生产 buy1/sell1 数")` 在生产路径有 sell1 时会失败 | (a) 代码错（诊断探针；生产判据未受影响） | `rust/src/theta_v0/classifier/signal.rs:1586-1590`：<br>`(Some(Direction::Down), Direction::Down) => end.price < c.zd,`<br>`(Some(Direction::Down), Direction::Down) => c.zg < end.price,`<br>生产同源判据 `signal/gates.rs:67` 为 `(Some(Direction::Up), Direction::Up) if last_center.zg < end.price`。回归由 `1592bf05ca`（#988，2026-08-15）引入，`git show 1592bf05ca` 中可见 `-(Some(Direction::Up), Direction::Up)` → `+(Some(Direction::Down), Direction::Down)`。消费方是 `#[ignore]` 的真实数据普查 `tests/pipeline_geometry.rs:649 type1_funnel_census_btc`，常规 CI 不跑，所以一直没暴露。本分片未实跑该 ignored 测试 | **可当场小修**：第二臂恢复为 `(Some(Direction::Up), Direction::Up)`，修后跑一次 `type1_funnel_census_btc`（nc-remote）确认 parity |
| A7 | #1568 正文 | S v3 `init` 在磁盘满时留下 4096 字节残留库，重试需人工清理 | (a)（已有专票） | `rust/src/bin/s_session_v3/atomic_init.rs` 在 main 上不存在，修复未合入；#1568 OPEN | **转现有票 #1568**（修复分支待合） |
| A8 | #1570 正文（#1568 后续） | S v1/v2 的 `init`/`reset` 仍直接在目标路径建库，失败留半成品 | (a)（已有专票） | #1570 OPEN；main 上 v1/v2 init 路径未改 | **转现有票 #1570** |

### A5 附表：真信号类告警（去重后位置）

| 类型 | 位置 | 说明 |
|---|---|---|
| deprecated `AtomicUsize::fetch_update` → `try_update` | `src/session_transport.rs:621`；`src/s_content/capture_allocation/scratch.rs:125,187,201,216`；`src/bin/s_session_v3/audit_owner/native.rs:527,537,578` | 工具链升级后变成 deprecated，下一次升级可能硬错 |
| unexpected cfg | `src/bin/s_session_v2/content_store.rs:3055`（`issue1394_capture_callback_refusal`）；`src/bin/s_session_v3/audit_store.rs:989`（`issue1394_closed_owner_refusal`） | 未声明的 cfg，被它门控的代码永远不编译；需确认是否为有意的故障注入开关，是则在 `Cargo.toml [lints.rust] unexpected_cfgs check-cfg` 声明，否则删除 |
| unreachable pattern | `src/theta_v0/classifier/signal.rs:1588` | 即 A6 |
| unreachable `else` | `src/bin/s_session_v3/object_index.rs:1887`；`audit_owner/q.rs:398`；`q_content.rs:222` | `let … else` 的 else 不可达，属于冗余写法，低危 |
| unused `Result` must_use | `src/bin/s_session_v3/driver.rs:2415,2745`；`prepared.rs:1710`；`request_audit.rs:1118`；`s_allocation.rs:404`；`cross_request_source_cache.rs:1608,1614` | 均为 `progress_handler(...)` 的返回值（rusqlite 新版本返回 `Result`），故障注入挂钩失败时会被静默吞掉，相关测试可能假绿 |

## (a?) 疑似当前就错（证据不全，需补查）

| # | 来源票与评论链接 | 现象 | 类别 | 证据 | 建议处置 |
|---|---|---|---|---|---|
| Q1 | #1405 [issuecomment-6270543288](https://github.com/xy7365527-lang/NewChanlun/issues/1405#issuecomment-6270543288)（「launcher 的 stop-s 返回『s 尚未退出，不自动升级信号』……上一次运行 stop-q 报了同样的错……没有进一步查」）；PR #1574 正文第 9 行同一现象 | 在 Linux/WSL 上，`stop-s`/`stop-q` 在 10 s 观察窗内判不到进程退出而抛 `TimeoutError`，runtime RESULT 因此记为 FAIL；3 s 后复查无残留进程、端口未监听。至少两次运行复现，会导致验收 RESULT 误判 FAIL | (a?) 脚本误报 | `s_session/s_service_control.py:652-676` `stop()`：以 `startup_timeout_ms`（10 s）为观察窗，超时 `raise TimeoutError(service + " 尚未退出，不自动升级信号")`。存活判定 `process_identity`（:151）用 `ps`，Z 态在 :168 处理。根因未查：可能是窗口过短、僵尸或回收时序，或 ps 判定的边界问题 | **新开票**（诊断票：在 Linux 上复现，记录 stop 期间 `ps` 状态序列，再决定是放宽窗口还是修存活判定） |
| Q2 | PR #1589 Codex P1 [r4322141762](https://github.com/xy7365527-lang/NewChanlun/pull/1589#discussion_r4322141762)、P1 [r4322141774](https://github.com/xy7365527-lang/NewChanlun/pull/1589#discussion_r4322141774)、P2 [r4322141784](https://github.com/xy7365527-lang/NewChanlun/pull/1589#discussion_r4322141784)（PR 已合，评论未回复） | ADR 0040 已合入 main。(1) §四规定「Q 变小或消失不减仓」，但没有要求降额版本生效前校验 `Σ既有责任 ≤ 新 cap`，与 ADR 0029 冲突。(2) 多重 profile 中 `q_ref` 指向只含总投入的 funding-record，按 ADR 0034 读 `q_ref` 的消费者会读到 5,000,000，而不是该重的固定 Q。(3) L1 tier policy 示例缺少 ADR 0034:14 要求随版本冻结的 `tier_quantity`（只出现在 :299 的派生结果里） | (a?) 正本文档间自相矛盾；尚无代码消费（`git grep q_ref/tier_quantity` 在 rust、s_session 下无命中），暂不构成运行错误 | `docs/adr/0040-funding-signature-and-chong-derivation.md:59`（第 5 条与「变小或消失不减仓」）、:195-205（`q_ref: #1557/funding-authorization/v1@P0-acceptance-5m`，tiers 无 `tier_quantity`）、:299 | **新开票或回 #1557/#1558 补裁**（两票已关）：属教义/ADR 修订，须在任何消费 `q_ref` 的实现开工前裁掉 |

## (b) 设计/性能观察（非错误）

| # | 来源 | 现象 | 证据/说明 | 建议处置 |
|---|---|---|---|---|
| B1 | #1394 [issuecomment-6090179596](https://github.com/xy7365527-lang/NewChanlun/issues/1394#issuecomment-6090179596) | object_index 加载 10.4 s | 性能观察；冷启动路径 | 保持观察；如需治理可并入 #1488 降阶线 |
| B2 | REVIEW-1576-opus.md INFO-4 | 历史规则的空闲库是否应在 serve 时拒绝 | 待裁定的语义问题，不是错误 | 留 #1576 裁定 |
| B3 | REVIEW-1576-opus.md INFO-3 | 空显示值的呈现方式 | 观察 | 留 #1576 |
| B4 | REVIEW-MERGE-MAIN-28df-01.md INFO-1 | `d5b9d01814` 中的两条预算检查未进 main，属契约缺口 | 设计层：当前没有测试或行为依赖它们 | 转 #1420，或在 #1394 收尾登记中新开小票 |
| B5 | REVIEW-MERGE-MAIN-28df-01.md L3 | 检查顺序不一致 | 风格/一致性 | 不处置或顺手 |
| B6 | 测试覆盖类 LOW：28df L1/L2；REVIEW-MERGE-1405-01 L-1/L-2/L-3/LOW-5；REVIEW-1576-tail INFO-1 | 测试缺口 | 无已知错误行为 | 由各自后续票补测 |
| B7 | #1488 [issuecomment-6244683195](https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6244683195) | 冷根风险与 P3 登记 | 都在 #1488 分支上，计划在整票验收前处理 | 留 #1488 |
| B8 | graph-engine-fix-20261020 review-round3 LOW-R3b | — | 评审方认可是有意为之 | 无 |

## (c) 已修

| # | 来源 | 现象 | 修复 |
|---|---|---|---|
| C1 | PR #1473 Codex P2（tb02c compare-existing） | `save_comparison` 遇到既有收据会出错 | main `s_session/tests/tb02c_harness.py` 已处理既有收据 |
| C2 | PR #1473 Codex P2（`second_kind_out_of_scope` 等待原因） | 等待原因残留 | main `rust/src/theta_v0/parser/tb02c_tests.rs:112` 已断言不再出现 |
| C3 | PR #1477 | SECOND 扫描起点 | 已在 PR #1477 内修复并合入 |
| C4 | #1563 | `install-decision-claimer.sh` 路径依赖 cwd | PR #1564（`2517b12625`，已在 main）改用 `SCRIPT_DIR` |
| C5 | #1405 补评 WARN-3 → #1576 | 旧规则库遗留未完成的 Begin 时，serve→recover 走成死路 | PR #1577（`262f8c863e`；`c6bac5ca6f` 已在 main），修掉了 REVIEW-1576 的 WARN-1 和 INFO-1。#1576 仍 OPEN，只剩 INFO-3/INFO-4 两项裁定（见 B2/B3） |
| C6 | decision-archaeology-gate-20261020 review-round3 NEW3-LOW-1 | 续做（resume）分支未恢复 `factGapNote` | `b1643f4e4f`（随 PR #1580 合入）；main `.sandcastle/decision-claimer.mts:1132` 的 `Object.assign` 已含 `factGapNote: p.factGapNote` |
| C7 | #1520「main CI 红：3 个 theta_v0::backtest 测试失败」 | — | 最新 main rust-check run 39380862852、39358379553 均 success。该票实质上已被取代（基线失败测试归 #1594 分片），建议关票或并入 #1594 |
| C8 | calibration/seam 测试相关观察项 | — | 已随 #1394 收尾合入修复 |

## (d) 已有专票跟踪

| # | 来源 | 现象 | 专票 |
|---|---|---|---|
| D1 | #1527 [issuecomment-6267311108](https://github.com/xy7365527-lang/NewChanlun/issues/1527#issuecomment-6267311108) | fixed_radix 失败 | 修复 `5cb7253d` 未进 main，走 #1488 线 |
| D2 | #1493 [issuecomment-6287964291](https://github.com/xy7365527-lang/NewChanlun/issues/1493#issuecomment-6287964291) | 读超时 | #1497 |
| D3 | #1493 裁定四 | X-37 下游读取 `segment_parse_invalid` 阻断标记 | #1581 / #1584 |
| D4 | #1493 | 解析内存 | 排除（#1596/#1597） |
| D5 | #1394 | 2h10m 运行时长 | 排除（#1592） |
| D6 | #1394 | 逐提交编译例外 | 在 `PERCOMMIT-CHECK.md` 登记，属于范围外 |
| D7 | #1568 / #1570 | 见 A7/A8 | #1568、#1570 |

## 结论摘要

当前就错 8 项：
- 可当场小修 4 项：A1 key 泄露、A3 悬空指针、A4 过时注释、A6 漏斗探针判据错臂。
- 转现有票 3 项：A2 → #1592；A7/A8 → #1568/#1570。
- 新开票 1 项：A5 编译告警回涨（承接已关的 #913）。

疑似当前就错 2 项：Q1 stop-s/stop-q 超时误判 FAIL（建议开诊断票）；Q2 ADR 0040 的 Codex P1/P2 三点（回 #1557/#1558 补裁，或新开票）。
