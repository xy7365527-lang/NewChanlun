# #1588 工程债清零 · tests 分片：三组基线失败测试的复现、根因与处置

- 票：#1588（工程债清零）· 分片：tests · 分支 `eng-debt-1588/tests`（基于 `origin/main` `1d70db893f`）
- 名分：绑定开放票 #1588 的工作草稿（宪法 §5），活期随票。
- 执行面：本机 macOS 不跑 cargo；Rust 一律用 `~/.local/bin/nc-remote <ref> rust <命令>` 在 windows-5090 的 WSL Ubuntu 上跑（下称「WSL」），日志在 `/tmp/nc-remote/<run-id>.log`（下列 run-id 可直接对日志）。CI 证据用 `gh run view --log` / `--json jobs` 取。
- 结论速览：**三组都不在 main 上构成缺陷，本分片不改任何代码**。A 组 16 条在 main 上不存在或为绿，全部红在 #1488 未合入的分支线上，转回 #1488；B 组 22 条是平台限定的预期行为（判环境问题），macOS CI 同批全绿；C 组确定性稳定（CI 连续 9 次 + WSL 10 个独立进程同读数），但**它的绿是偶然的，已不再锁住线段停产**，这一点转 #1493。

---

## A. #1488 宽对拍里「基线即失败」的 16 条

### A-0 名单来源

`/Volumes/AgentStorage/eng-debt-20261020/1488-baseline-failures.md`（#1488 负责人写，2026-10-20 14:43 出现）：宽对拍命令 `cargo test --features s_session,s_capture_native_testonly --bin s_structure_session -- v3::audit_owner v3::request_audit v3::cross_request_source_cache v3::fixed_radix v3::driver`，冻结源 `fff166540e` 235P/16F/5I。

### A-1 关键事实：名单里的「基线」不是 main

| 核查 | 命令 | 结果 |
|---|---|---|
| #1488 的「基线」提交 | `git log -1 959d860065`；`git merge-base --is-ancestor 959d860065 origin/main` | `959d860065 WIP(#1488): checkpoint incomplete budget ownership refactor`，**不在 main**；它是 #1488 自己的 WIP，从 main 的 `b46ba29699` 分出后已走 28 个提交（#1488 票面 comment 第 987–1003 行：「基线 959d860 本身两组 features 都编译不过（17 处）」，之后 `a36c252589`、`25333c0f2a` 两轮机械适配才可编译） |
| 16 条在 main 上是否存在 | `grep -rln "fn <名>\b" rust/src`（在 `1d70db893f`） | **11 条在 main 上不存在**（只在 #1488 分支上），5 条存在 |
| 11 条由谁引入 | `git log -S "fn <名>" b46ba29699..origin/codex/1488-uo-ci-20261019 -- rust/src` | 全部由 #1488 自己的提交引入：`3f2b4fd924`（3 条）、`9d03c773ae`（2 条）、`ea41d4dfd1`（6 条，提交标题自注「test compatibility v2 candidate (not accepted runtime)」） |
| 同一宽对拍在 main 上 | `nc-remote eng-debt-1588/tests rust cargo test --locked --features s_session,s_capture_native_testonly --bin s_structure_session --no-fail-fast -- v3::audit_owner v3::request_audit v3::cross_request_source_cache v3::fixed_radix v3::driver`（run `20261020-144647-1d70db893f`） | `test result: ok. 103 passed; 0 failed; 5 ignored`；main 上存在的 5 条均 `... ok` |
| 同一宽对拍在 #1488 当前头 | 同命令，ref=`origin/codex/1488-uo-ci-20261019` @ `c2bdf9b662`（run `20261020-145629-c2bdf9b662`） | `236 passed; 16 failed; 5 ignored`；失败集与名单 `diff` **逐名相同（16 条）** |
| main 存在的 5 条从何时开始红 | ref=`25333c0f2a`（#1488 线上第一个可编译提交），`--exact` 5 条（run `20261020-145230-25333c0f2a`） | 4 条已红，panic 文案与当前头相同；`source_keeper_driver_append_repost_restart…` 在此处仍 `ok`，在 #1488 线之后的提交才转红 |
| main CI 同一批 | run [39358379553](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39358379553)（`6badaa6049`）「s_capture_native_testonly 特性新增的测试」步 | `feature_delta target=bin … delta=174`，`167 passed; 0 failed; 7 ignored`；r11/r13/r14/driver/request_audit r11 五条均 `... ok` |

`6badaa6049`..`1d70db893f` 之间 `rust/src/bin/s_session_v3/` 无差异（`git diff --stat` 为空），所以 CI 的 main 读数与 WSL 的 main 读数对的是同一份源码。

### A-2 逐条台账

列说明：「main」＝ `1d70db893f` 上的状态（WSL run `20261020-144647-1d70db893f` + CI 39358379553）；「#1488 头」＝ `c2bdf9b662` 上的 panic 点（WSL run `20261020-145629-c2bdf9b662`）；处置一律**转 #1488**（理由见 A-3）。

| # | 测试 | main | #1488 头 panic 点与文案 | 引入/转红 |
|---|---|---|---|---|
| 1 | `v3::audit_owner::construction_tests::owner_set_real_publish_commit_failure_revokes_everything` | 不存在 | `audit_owner.rs:5366:14` unwrap Err「prototype_audit_allocation_incomplete：Native文本复制需要当前WorkScope」 | #1488 `3f2b4fd924` 引入 |
| 2 | `…construction_tests::owner_set_real_publish_stage_effects_commit_promotes` | 不存在 | 同上 `:5366:14` | `3f2b4fd924` |
| 3 | `…construction_tests::owner_set_real_publish_unscoped_same_value_update_rejects` | 不存在 | 同上 `:5366:14` | `3f2b4fd924` |
| 4 | `v3::audit_owner::effects::construction_coverage_tests::owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge` | 不存在 | `audit_owner/effects.rs:3024:9`「原codec retain仍在同一个父账中」 | `9d03c773ae` |
| 5 | `…construction_coverage_tests::owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain` | 不存在 | `effects.rs:3053:9`「原codec retain仍由原父账持有」 | `9d03c773ae` |
| 6 | `v3::audit_owner::effects::e_owner_move_tests::e_apply_native_insert_is_ordered_and_releases_only_manual_outer_charge` | 不存在 | `effects.rs:3161:9` `left == right` | `ea41d4dfd1` |
| 7 | `…e_owner_move_tests::e_apply_native_replace_and_delete_keep_remaining_rows_owned` | 不存在 | `effects.rs:3227:13` `left == right` | `ea41d4dfd1` |
| 8 | `…e_owner_move_tests::e_stage_into_row_keeps_charge_after_header_drop` | 不存在 | `effects.rs:3094:9` `left == right` | `ea41d4dfd1` |
| 9 | `v3::audit_owner::q::actual_nonempty_native_q_accept_preserves_full_raw_without_manual_double_charge` | 不存在 | `audit_owner/q.rs:1033:5` `left == right` | `ea41d4dfd1` |
| 10 | `v3::cross_request_source_cache::tests::r11_source_binding_new_and_seal_use_shared_scope` | **ok** | `cross_request_source_cache.rs:1074:17`「新trial完整原件绑定须消费父域许可」 | 在 `25333c0f2a` 已红（同文案，`:793:17`） |
| 11 | `v3::cross_request_source_cache::tests::r13_source_trial_cold_hit_miss_record_and_seal_use_shared_scope` | **ok** | `:1383:13`「保守费用保留在父账」left 128 / right 135732（fff166540e 读数） | 在 `25333c0f2a` 已红（`:1102:13`） |
| 12 | `v3::cross_request_source_cache::tests::r14_source_pool_normal_growth_and_sparse_cap_match_none` | **ok** | `:1205:18` unwrap Err `Resource(Ledger)` | 在 `25333c0f2a` 已红（`:924:18`） |
| 13 | `v3::driver::tests::source_keeper_driver_append_repost_restart_preserves_results_and_request_reuse` | **ok** | `driver.rs:3109:9`「追加请求须复用已核旧cut」 | `25333c0f2a` 上仍 ok，#1488 线之后转红 |
| 14 | `v3::request_audit::r_owner_move_tests::r_optional_bad_sql_precedes_scope_and_rejected_read_refunds` | 不存在 | `request_audit.rs:2555:39` unwrap Err `BudgetExceeded` | `ea41d4dfd1` |
| 15 | `v3::request_audit::r_owner_move_tests::r_sql_row_pop_and_blob_move_keep_exact_parent_charge` | 不存在 | `request_audit.rs:2530:9` `left == right` | `ea41d4dfd1` |
| 16 | `v3::request_audit::tests::r11_request_native_shared_scope_cold_warm_append_full_output_equal` | **ok** | `request_audit.rs:1326:21`「legacy retain is still explicitly present」 | 在 `25333c0f2a` 已红（`:1023:21`） |

### A-3 处置：16 条全部转 #1488，本分片不修

- **根因层级**：16 条的红都来自 #1488 自己的「预算/所有权统一」重构线（WIP `959d860065` 起），不是 main 的缺陷。main 上 5 条同名测试是绿的，另 11 条在 main 上根本不存在。名单把这条线的起点叫「基线」，相对 #1488 的增量来说没错；但相对 main 来说，这 16 条是 **#1488 分支内的未完成项**。
- **为什么不在本分片修**：① 修改面全在 #1488 未合入的分支上（`audit_owner*`、`cross_request_source_cache.rs`、`request_audit.rs`、`driver.rs` 的所有权/父账重构）。本分片若在该分支上改码，就会覆盖在途工作，违反「不得回退或覆盖他人改动」。② #1488 票面已把这件事列为自己的剩余项（comment 第 1174 行：「宽对拍里 20 条基线既有失败的归属」）。③ 测试和实现谁错要按 #1488 那条线的已批合同判（父账、WorkScope、retain 的语义是 #1488 的设计面，不归 `.chanlun/definitions/` 缠论教义管）。这个判断只能由 #1488 的负责人做。
- **查重**：`gh issue list --state all --search` 搜 `r11_source_binding_new_and_seal_use_shared_scope`、`r14_source_pool_normal_growth`、`owner_set_real_publish`、`source_keeper_driver_append_repost_restart`、`r11_request_native_shared_scope`，结果均为空。搜 panic 文案「Native文本复制需要当前WorkScope」命中 #1488、#1323（均 open；#1323 是「完整生产程序接线总单」图，不是这 16 条的独立票）。归属已在 #1488 票面在案，**不另开票**。
- **与主控追加指示的关系**（评审 F1）：名单文件末尾的主控指示要求从 #1488 合并面另起 `eng-debt-1588/tests-1488` 修这 16 条。本分片未照做，理由：指示点名的基分支 `codex/1488-budget-full-chain-fix-v2-20261018` 远端不存在；#1488 头在运行期间仍在移动；16 条的「测试对还是实现对」属 #1488 预算/所有权合同的裁定面，走指示中的「需裁定则转票」出口；A 组无代码改动，「A 与 B/C 分开 PR」的目的（依赖 #1488 先合）不成立。主控已接受该偏离。
- **交接已落地**（评审 F2，主控 2026-10-20）：#1488 负责人已核实并接手（切片 0058 逐条 bisect 首红、分类为生产回归/语义改变/夹具问题，合入 main 前清零）。
- **给 #1488 的一句话**：这 16 条在合入 main 之前必须清零，或者逐条裁「测试过时」后改写。不能以「基线既有」为由带进 main：对 main 而言它们是新增红，其中 5 条是把 main 上的绿锁打红。

---

## B. WSL 上 `v3::financial_session` 22 条失败

### B-1 复现

```
nc-remote eng-debt-1588/tests rust cargo test --locked --features s_session --bin s_structure_session v3::financial_session
# run 20261020-144202-1d70db893f
test result: FAILED. 4 passed; 22 failed; 3 ignored; 0 measured; 518 filtered out; finished in 32.60s
```

失败点汇总（`grep panicked` 后去重计数）：

```
  21 called `Result::unwrap()` on an `Err` value: "MissingDependency：会话保留量读数当前只在macOS实现"
   1 startup_resource_caps_and_custody_refuse_to_start @ financial_session_tests.rs:633:5  →  "MissingDependency：会话保留量读数当前只在macOS实现"
```

其中 21 条的 unwrap 落在 `financial_session_tests.rs:40:32`（`start()` 内 `start_with(f, resources()).unwrap()`）或各自的 `start_with(...)` 调用点（`:677:35`、`:721:35`、`:806:35`、`:1567:39`）。第 22 条在 `:633`，期望 `Incomplete/CaptureBytes`，实际拿到的错误同样是 MissingDependency。**22 条的失败原因相同，都是这一个错误。**

通过的 4 条（`cli_grammars_are_exact`、`connections_before_ready_are_refused`、`error_prefixes_are_not_nested`、`resource_and_custody_documents_are_exact`）不调 `Session::start`。

### B-2 根因：平台限定的诚实拒绝

`rust/src/bin/s_session_v3/financial_session.rs`：`:496` `#[cfg(target_os = "macos")]` 实现会话保留量读数（heap_probe：`malloc_zone_statistics` 差 − `sqlite3_memory_used` 差），`:503–505` `#[cfg(not(target_os = "macos"))]` 直接 `Err(missing("会话保留量读数当前只在macOS实现"))`。`:515/:528`、`:781/:789` 两处（peer uid / getpeereid 一族）同构。依据是 #1394 根裁定 A（`ci-macos.yml:3-7` 注释）：同一判定不得在两个平台给出两种读数（#804），所以 Linux 不提供替代读数，诚实报缺依赖。

### B-3 macOS CI 同批全绿

- ci-macos run [39358379664](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39358379664)（main @ `6badaa6049`，该工作流在 main 上最近一次 run，`conclusion=success`）：步「cargo test — v3::financial_session（macOS 保留量读数与 peer uid，#1394 根裁定 A）」→ `test result: ok. 26 passed; 0 failed; 3 ignored; 0 measured; 518 filtered out`。WSL 上失败的 22 条在这里逐条 `... ok`，日志已逐名核对。
- Linux CI（ci.yml）在「s_session 正式结构会话」步以 `--skip v3::financial_session` 跳过整批（ci.yml:265-271 注释写明原因）：run 39358379553 该步 `running 518 tests … 507 passed; 0 failed; 11 ignored; 29 filtered out`。29 = 26 + 3，正好是 financial_session 全集。
- `6badaa6049`..`1d70db893f` 的 `rust/src/bin/s_session_v3/` 无差异，macOS 读数适用于本基点。

### B-4 处置：判环境问题，不改码

考虑过、未采纳的两条路：

1. `#[cfg_attr(not(target_os="macos"), ignore)]`：任务禁止（等价于加 ignore）。
2. `#[cfg(target_os = "macos")]` 编译门控 22 条：macOS CI 全集不受影响，所以覆盖不降。但收益只是「Linux 本地全量跑不出 22 条红」，代价有三：①（评审 F3 订正：此条不成立——测试模块经 `financial_session.rs` 的 `#[path] mod tests` 挂载，一层 `#[cfg(target_os = "macos")] mod` 即可门控，无 dead_code 问题，故删去此论据）；② 改动后测试在 Linux 上**不出现**，而现状是**诚实地红并写明原因**。前者更容易让人误以为 Linux 上测过；③ Linux CI 已经用 `--skip` 加注释显式记账，加门后这条 `--skip` 变成冗余，ci.yml 与源码要双改（ci-time 分片也在动 ci.yml，有冲突面）。

所以最诚实的处置是：**判平台限定的预期行为**，证据为 B-1（22 条失败点都是 MissingDependency）+ B-2（源码 cfg 分支）+ B-3（macOS 同批 26/0/3 绿，Linux CI 显式 skip）。在 Linux/WSL 上做全量验证时，照 ci.yml 加 `-- --skip v3::financial_session`，并以 macOS 作业为该批的唯一判据。

可选的后续（不在本分片做，留主控定）：4 条不依赖 `Session::start` 的平台无关用例目前在 Linux CI 上也被整批 skip 掉，覆盖仍在（macOS 跑），不构成缺口。如果想让 Linux 也执行它们，可以把 ci.yml 的 skip 收窄成具名列表，但要和 ci-time 分片协调。

---

## C. `m3_partition_synthetic_holds` 稳定性

### C-1 CI：main 上连续 9 次「cargo test — default targets」绿

先 `gh run view <id> --log | grep m3_partition_synthetic_holds`，再用 `--json jobs` 的步起止时间把命中行对到步。

| run | head | default targets 步 | m3 结果 | 取证方式 |
|---|---|---|---|---|
| [39321352559](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39321352559) | `b34b0278ee` | failure | **FAILED**（`l3_delta_r_alpha.rs:2541:9` ledger 空） | 日志 grep |
| [39321369264](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39321369264) | `b63a3fd586` | success | ok（20:45:12，步窗 20:43:27–20:46:37） | 日志 grep |
| [39330677971](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39330677971) | `59088aef41` | success | ok（21:56:30，步窗 21:54:13–21:58:53） | 日志 grep |
| [39332095664](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39332095664) | `28df886697` | success | ok（22:07:32，步窗 22:05:30–22:09:30） | 日志 grep |
| [39347718692](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39347718692) | `262f8c863e` | success | ok（00:32:46，步窗 00:30:13–00:35:04） | 日志 grep |
| [39347739246](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39347739246) | `bc401fc5b3` | success | ok（00:33:21，步窗 00:30:46–00:35:47） | 日志 grep |
| [39358379553](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39358379553) | `6badaa6049` | success | ok（02:27:07，步窗 02:24:17–02:33:06） | 日志 grep |
| [39379637876](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39379637876) | `d2f20aaca8` | success | 步 success ⟹ 含 m3 在内无失败（该步 `--no-fail-fast`） | `--json jobs`（整 run 未完，日志未开放） |
| [39380854169](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39380854169) | `c185f77ae0` | success | 同上 | `--json jobs` |
| [39380862852](https://github.com/xy7365527-lang/NewChanlun/actions/runs/39380862852) | `1d70db893f` | success | 同上 | `--json jobs` |

另外，前 6 个已完成的 run 的「backtest_bin feature」步也各有一次 m3 `... ok`。

**转绿点订正**：任务前提说「#1493 称在 `e79bdb2a52` 上转绿」。`e79bdb2a52` 只在 #1493 分支（PR #1582）上，**不在 main**。main 的转绿点是 `b63a3fd586`（#1569 合入，含 `279b7c8708` feat(#1420) 078:52 失效语义）：`git merge-base --is-ancestor 279b7c8708` 对 `b34b0278ee` 为否、对 `b63a3fd586` 为是。

### C-2 WSL：10 个独立进程同读数

```
nc-remote eng-debt-1588/tests rust bash -c 'cargo test --locked --lib --no-run; for i in $(seq 1 10); do cargo test --locked --lib m3_partition_synthetic_holds -- --nocapture --exact theta_v0::backtest::l3_delta_r_alpha::tests::m3_partition_synthetic_holds | grep …; done'
# run 20261020-144314-1d70db893f，EXIT=0
iter 1..10 每次：
[m3-partition-synth] |ledger|=1 kept=1 same_bar=0 nonpos=0 |records|=1
test theta_v0::backtest::l3_delta_r_alpha::tests::m3_partition_synthetic_holds ... ok
```

每次都是新进程，HashMap 的 RandomState 每次重新取种，10 次读数逐位相同。

### C-3 随机性排查

- 输入：`synthetic_martingale(3000, 20260704, 100)`，SplitMix64 定种（`l3_delta_r_alpha.rs:1052`），不读时间、不起线程。
- 路径上的哈希容器：`mu_estimator.rs:469/695` 的 `HashMap` 桶只进 `_est`，测试不用；`l3_delta_r_alpha.rs:1818` 的 `HashSet` 只做成员去重，不按迭代序产出。测试的断言只看 ledger/records 的计数与分区守恒，与这些容器的迭代序无关。
- 结论：**无随机性，无需修成确定性**。

### C-4 但这份绿是偶然的：它已经锁不住线段停产

#1563 关票时 PR #1564 写明 m3 是生产行为问题、不改测试、归 #1487/#1493，换 seed 或加长输入让 m3 变绿会掩盖缺陷（转述，评审 F4 订正引号）。#1420 独审（`.chanlun/review-results/issue1420-tb02w-20261019/review-20261019/REVIEW.md` §4）查明转绿始于 `279b7c8708`，属于段划分改变后的副产物：该种子 n=3000 上段数 24→14，自第 160 笔起卡在 `FAILED_GEOMETRY/top_gt_bottom` 吸收态。ledger 从 0 变 1 只是塔结构变化的连带结果。

本分片用同一探针（`probe1420.sh` 的 `probe1420_segments`，打在本地临时提交上，**未推送**：main 探针 `645d66de23`、#1582 探针 `05c3cadd3d`）重测：

| 源 | n=3000 段数 / 末段终点 / ledger | n=20000 段数 / ledger | m3 |
|---|---|---|---|
| main `1d70db893f`（run `20261020-145621-645d66de23`） | **14** / 1708（merged 1998） / 1 | 79 / 6 | ok |
| #1582 头 `8f86b657e6`（run `20261020-150044-05c3cadd3d`；m3 单跑 run `20261020-144723-8f86b657e6`） | **32** / 2958 / 1 | 192 / 15 | ok |

所以：

1. 在 main 上，停产在这个种子上仍然存在（14 段，末段止于 1708/1998），但 m3 是绿的。**m3 当前不是停产回归锁**。它唯一的非空门 `!ledger.is_empty()` 在 ledger=1 时刚好过线。
2. #1493 落地后（#1582 头），段数回到连续产出（32 段，末段贴右缘），m3 仍绿，ledger 仍为 1。所以 m3 在「停产」和「不停产」两种状态下读数相同，**区分不了两者**。
3. 处置：**C 的稳定性问题判为不存在**，不改测试，也不能为了「锁」而在 main 上加一条会红的断言（停产修复在 #1582 在途，现在加只会多一条已知红，并和 #1582 的段层锁重叠）。**「停产回归锁失效」转 #1493**：建议 #1582 合入前补一条针对本种子的段层连续性锁，例如 n=3000 段数下限或末段终点贴右缘（#1582 的「零计数锁」族是天然载体）。#1563 关票时依赖的「m3 作为停产锁」前提已不成立，需要在 #1493 票面明记。

---

## D. 顺带发现

- **#1520 是陈旧重复票**：`[task] main CI 红：3 个 theta_v0::backtest 测试失败`（open，needs-triage，0 评论），列的 m3 / calibration_sweep / seam_btc_tick 三条在上表 6 个已完成的 main run 中全部 `... ok`（calibration 由 #1564 重标定、m3 见 C）。它的内容被 #1563（已关）完全覆盖。建议主控以 #1563 + 本报告 C-1 为证据把 #1520 作为重复关闭（本分片不做对外关票动作）。

## E. 评审重点（请非作者独审着重看）

1. A-1 的判断：「名单里的基线 ≠ main」。核对 `959d860065` 不在 main，并在 WSL main 上复跑宽对拍（103P/0F）。
2. B-4 不采纳 cfg 门控的理由是否站得住，「判环境问题」是否比加门更诚实。
3. C-4 的「m3 已不是停产锁」及转 #1493 的建议，是否需要升级为 #1493 合入门的硬条件。

## F. 复现命令汇总

```bash
# A：main 与 #1488 头的宽对拍
nc-remote eng-debt-1588/tests rust cargo test --locked --features s_session,s_capture_native_testonly --bin s_structure_session --no-fail-fast -- v3::audit_owner v3::request_audit v3::cross_request_source_cache v3::fixed_radix v3::driver
nc-remote origin/codex/1488-uo-ci-20261019 rust <同上>
# B
nc-remote eng-debt-1588/tests rust cargo test --locked --features s_session --bin s_structure_session v3::financial_session
# C
nc-remote eng-debt-1588/tests rust cargo test --locked --lib m3_partition_synthetic_holds -- --nocapture   # ×10，见 C-2
cd /tmp && gh run view <run-id> --repo xy7365527-lang/NewChanlun --log | grep m3_partition_synthetic_holds
```

## G. 共享 target 污染窗口后的复跑（主控，2026-10-20）

2026-10-20 12:00–15:38 WSL nc-remote 各运行共用 `CARGO_TARGET_DIR`，并发会互相覆盖测试二进制（#1488 通报；nc-remote 已改私有 `NC_TARGET` + flock，备份 `~/.local/bin/nc-remote.bak-20261020`）。本报告 A/B/C 的远端读数全在该窗口内，故用新 nc-remote（`NC_TARGET=eng1588`）在 `1d70db893f` 上串行复跑：

| 组 | 复跑日志 | 读数 | 与原读数 |
|---|---|---|---|
| B | `20261020-160510-1d70db893f` | 4 passed; 22 failed; 3 ignored | 一致 |
| C | `20261020-160635-1d70db893f` | 6 次 ok，每次 `\|ledger\|=1 kept=1 same_bar=0 nonpos=0 \|records\|=1` | 一致 |
| A-main | `20261020-160727-1d70db893f` | 103 passed; 0 failed; 5 ignored | 一致 |

#1488 头一侧的 236/16 读数未复跑（属 #1488 自证范围，其切片 0058 正在重做）。C 组另有 main CI 9 次「default targets」步绿为不受该窗口影响的独立证据。#1493 已据 C-4 在 PR #1582 加段层停产锁 `m3_synthetic_segment_layer_does_not_stall`（提交 `9f3b5281eb`，段数≥30、末段终点距末笔≤150 根）。
