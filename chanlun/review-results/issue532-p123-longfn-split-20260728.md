# #532 修复报告：p123_fast_replay 长函数拆分与参数簇治理

## 结论

`rust/src/bin/p123_fast_replay.rs` 两个超 50 行的生产函数（`refresh_lifecycle_cache`、
`feed_lifecycle_bar`）已拆分完毕，所有新函数 ≤50 行、≤5 参；`hist`/`dif`/`close_src`
Data Clumps 收成 `CausalSeries<'a>`，另收束两个恒同现的可变状态簇为
`LifecycleCacheState<'a>`（derived/entries/stats）与 `LifecycleDumpSink<'a>`（sink/stats）。
纯代码搬移，零逻辑改动，20k/100k 窗口下 stdout + P421 lifecycle dump 逐字节 `cmp=0`。

### 票面第三个函数（`feed_replay_bar`）的裁定

票面引用的旧行号（`:1405-1580`）来自三审基准提交 `391936a486`；该提交之后 #527/#591/#592
等并行线大改了本文件，`feed_replay_bar` 的定义体已不在 `p123_fast_replay.rs` 内——
它是 `rust/src/theta_v0/classifier/nest_lifecycle.rs:1743` 的公开函数，`p123_fast_replay.rs`
现在只在 `feed_lifecycle_bar` 内部调用它（1 处，`rust/src/bin/p123_fast_replay.rs:1937`）。
按票面"本票只覆盖 p123 长函数与参数簇"的划界，`nest_lifecycle.rs` 内的函数长度归
#454 承接，不在本票范围——此为票面 Acceptance"或给出逐函数裁定例外"条款下的裁定。

### 函数长度/参数数逐项对照

| 函数 | 拆分前 | 拆分后 |
|---|---|---|
| `refresh_lifecycle_cache` | 93 行 / 9 参 | 11 行 / 3 参（`cache`, `world: RefreshWorld`, `state: &mut LifecycleCacheState`） |
| ├─ `refresh_level_runs`（新提取） | — | 35 行 / 4 参 |
| ├─ `refresh_run_entry`（新提取） | — | 14 行 / 3 参 |
| ├─ `run_entry_is_dirty`（新提取） | — | 18 行 / 3 参 |
| ├─ `empty_run_entry`（新提取） | — | 3 行 / 3 参 |
| └─ `reevaluate_run_entry`（新提取） | — | 42 行 / 3 参 |
| `feed_lifecycle_bar` | 85 行 / 8 参 | 24 行 / 5 参（`book`, `phases`, `as_of`, `series: CausalSeries`, `dump: &mut LifecycleDumpSink`） |
| ├─ `write_feed_summary_line`（新提取） | — | 19 行 / 3 参 |
| ├─ `write_completion_signal_lines`（新提取） | — | 24 行 / 3 参 |
| ├─ `write_revision_lines`（新提取） | — | 23 行 / 2 参 |
| └─ `write_force_unavailable_lines`（新提取） | — | 23 行 / 3 参 |

原两函数头顶的 `#[allow(clippy::too_many_arguments)]` 均已移除（参数已 ≤5，豁免不再需要）。

## 边界条件

- 本票不覆盖 `nest_lifecycle.rs`/`level_view.rs` 内的长函数（#454/#497 承接域）；
  也不覆盖 `p123_fast_replay.rs` 内票面未点名的其余超长函数
  （`main` 376 行、`run_targeted_prefix_pass` 497 行、`observe_snapshot` 97 行、
  `recompute_lifecycle_window_stems` 84 行、`collect_snapshot_candidates` 81 行、
  `observe_certificates` 72 行）——这些函数不在 #429/#430 三审引用的 §5.1/§9.1 清单内，
  超出票面授权范围，需另行登记新票。
- 若后续改动使 `evaluate_run` 的签名变化（当前仍保留 10 参 + `#[allow(clippy::too_many_arguments)]`，
  与本票无关），`reevaluate_run_entry` 的调用点需同步核对。
- `CausalSeries`/`LifecycleCacheState`/`LifecycleDumpSink`/`RefreshWorld`/`RunRefreshContext`
  均为本文件私有结构体，只服务本票拆分出的函数族；未导出、未影响其它模块的公开 API。

## 影响声明

- 改动文件：`rust/src/bin/p123_fast_replay.rs`（+231/−115 行，净增 116 行——全部是拆分产生的
  函数签名/doc注释/结构体定义，无新增业务逻辑）。
- 未改动任何测试、配置、其它源文件。
- 验证：
  - `cargo build --all-targets`：绿（仅预存量、与本文件无关的 dead_code/unused_imports 警告）。
  - `cargo test --lib`（单线程 `--test-threads=1`，排除并行 flake 干扰）：`2034 passed / 1 failed
    / 136 ignored`，唯一失败为在册 `extract_signals_bit_exact_digest_guard`（#491，与本票无关，
    位于 `signal.rs`，本票未触碰该文件）。多线程默认跑一度观测到额外 2 个瞬时失败
    （`open_ledger::tests::parent_units_takes_latest_generation_snapshot` 等），复跑消失，
    判定为该测试自身的并行隔离问题，与本票模块无关，不在本票修复范围内。
  - 字节护栏对拍（`rust/release` 编译，同一 BTC 全量数据 `analysis/data_cache/btc_1m_full.json`，
    `P116_MAX_BARS=20000` 与 `100000` 两窗）：拆分前（`git show HEAD` 原始源）与拆分后二进制的
    stdout、`P421_LIFECYCLE_DUMP` 均 `cmp=0`；stderr 仅 `prefix_s=` 计时字段不同（wall-clock，
    非确定性输出，不计入护栏），其余计数字段（`triggers/reevals/reuses/pan_*` 等）逐位相等。
  - `rustfmt --check`：本票新增/改动代码块干净；文件内唯一 fmt diff 在未改动的既有测试代码
    （`:2629` 附近 `gap_len` 断言），不属本票改动范围，未触碰。

## 认识论等级（`.claude/rules/formalization-validity-domain.md`）

L2：BTC 单标的、20k/100k 两个固定前缀窗口的真实数据字节级对拍；未做跨标的/跨时段交叉验证，
不外推其它数据窗口或品种（非 L3）。函数长度/参数数核验为 L0（纯代数计数，非经验命题）。
