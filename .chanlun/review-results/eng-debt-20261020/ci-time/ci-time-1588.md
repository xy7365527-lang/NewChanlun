# #1588 工程债分片 ci-time：rust-check 拆并行 job + s_session bin 测试分片

- 票：#1588（工程债清零），分片名 ci-time；分支 `eng-debt-1588/ci-time`（基于 origin/main `1d70db893f`）；PR #1592
- 目标：最长 CI job ≤ 90 分钟。手段限于拆作业、缓存、并行、测试线程；不删测试、不降覆盖、不加 ignore。
- 结论：**已达成**。最长 job 从 2h10m35s 降到 31m00s，整个 CI workflow 墙钟约 31 分钟。测试计数前后逐名一致。

## 条目 1：s_session bin 测试一步 89 分钟

**现象**：main run 39358379553 的 `rust-check` job 用时 2h10m35s（02:20:33→04:31:08），`timeout-minutes: 150` 只剩约 20 分钟余量。其中 `cargo test — s_session 正式结构会话（#1370）` 一步用了 89.1 分钟。

**复现命令与输出**（`gh run view 39358379553 --log --job 118418447937`）：

```
02:34:12 Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 05s
02:34:12 running 518 tests
04:02:07 test result: ok. 507 passed; 0 failed; 11 ignored; 0 measured; 29 filtered out; finished in 5275.31s
```

按分钟统计完成数：505 条测试在 02:34–03:14:58 之间全部完成。之后只剩两条：

```
03:42:15 test tests::tb02c_persistent_first_kind_and_normal_recovery ... ok
04:02:07 test tests::tb02w_killed_case_two_scan_budget_resume_and_recover ... ok
```

两条的开始时间按「has been running for over 60 seconds」减 60 秒推算，都在约 02:52:5x：tb02w 约 4150s，tb02c 约 2964s。

**根因**：
1. **慢在执行，不在编译。** 编译 1m05s，执行 5275s。
2. **测试线程没有被压成 1。** `--jobs 1` 只限制编译并行度。runner 显示 `CPU(s): 4`，job 里没设 `RUST_TEST_THREADS`，所以 libtest 默认开 4 个测试线程。日志里同一秒内有大量测试同时完成，tb02c/tb02w 的运行时段也互相重叠，都能印证这一点。
3. **两条长测试是 CPU 密集型。** 源码在 `rust/src/bin/s_structure_session.rs:5540` 和 `:5779`。每接纳一根 K 线就 `cmd_advance` 一次，S 每代全量重算并核整条代链；tb02w 每代还起 `python3` 做合同校验。源码注释 `:5693` 自己写着「debug 下逐根推进过慢」。
4. **4 vCPU 争用把它们拖慢约 2.4 倍。** 拆分后两条测试各自独占 runner：tb02w 1755s、tb02c 约 1229s（PR run 39382771454）。拆前分别约 4150s、2964s。再加上 libtest 按名字顺序起跑，`tests::tb02*` 排在中段，开跑约 19 分钟后才启动，于是成了尾巴。

**处置 = 修**：拆 job 并按名分片，见条目 2。

## 条目 2：拆 job 与分片方案

**改动**：
- `.github/workflows/ci.yml`：`rust-check` 拆成下列 job，原有步骤与命令逐条保留。
  - `rust-check`：wrapper 自测、分片脚本自测、check 两档、fmt
  - `rust-test-default`
  - `rust-test-s-session (shard 0..3)`
  - `rust-test-s-session-coverage`
  - `rust-test-s-session-extras`：TB-02-C、s_content lib、s_content tests、economic、capture_native 增量、v3 history，保留各自的 `if: always()`
  - `rust-test-backtest-bin`
- `.github/scripts/test-shard.py`（新增）：
  - 每片现场用 `cargo test ... -- --list --skip v3::financial_session` 动态生成全集，不维护静态清单；
  - 按 `.github/scripts/s-session-test-weights.tsv` 做 LPT 分片：权重降序，放进当前总权重最小的片，平局取编号小者，表外测试取中位数权重；
  - 当场断言「两两不交、并集 == 全集」，执行后断言「执行集 == 分配集」；
  - 产物 all.txt、assigned.txt、executed.tsv 上传为 artifact。
- `rust-test-s-session-coverage`（`needs` 分片 job，`if: always()`）：下载 4 片产物，再核「各片全集一致、片数齐全、并集 == 全集、执行集 == 分配集」，并汇总 ok/ignored/failed。
- `.github/scripts/test_test_shard.py`（新增，在 `rust-check` 里跑）：用桩 cargo 做 10 条自测，覆盖以下情形：
  - 任意片数下并集 == 全集、两两不交；
  - 计划与 list 顺序无关；
  - 空权重表仍全覆盖；
  - 漏跑一条即红；
  - cargo 失败码透传；
  - 缺片、越界 index 即红。
- `.github/actions/rust-job-setup`（新增 composite action）：各 Rust job 共用的前置，包括 #1141 记录 runner/工具链/资源、只缓存 registry + git、记录缓存结果，并预建 `rust/target`。预建的原因：遥测包装器的边界快照做 `du target`，失败即判红；拆 job 后首个 cargo 步就可能是被包装的那一步。
- 未改的部分：
  - #1141 遥测包装 `run-cargo-with-telemetry.sh` 原样包住每个 cargo test 步，分片步包住 `test-shard.py`；
  - `CARGO_INCREMENTAL=0`、`--jobs 1`、不缓存 target 都保持不变；
  - 测试线程保持 libtest 默认（=4）。

**为什么不加 target 缓存（Swatinem/rust-cache 等）**：拆后各 job 的编译只占 1–2 分钟（例：分片 2 `Finished ... in 1m 10s`），缓存收益很小。而且缓存 target 会重新引入 #1141 的磁盘风险，以及缓存体积和失效问题。现在瓶颈是执行，不是编译。

**为什么分 4 片**：按暂定权重模拟，2/3/4 片时的各片（条数, 权重和, 最大单条）如下：

- 3 片：`(161, 4524, 4150)`，tb02w 要和 160 条测试同片争用；
- 4 片：`(1, 4150, 4150) (149, 3141, 2964) (179, 3141, 1288) (189, 3141, 1022)`，tb02w 独占一片。

条目 1 已证实争用会把长测试拖慢约 2.4 倍，所以取 4 片。多一片的代价是多一份约 1–2 分钟的编译。

**处置 = 修**，验证见条目 3。

## 条目 3：前后对比（同测试面）

基线 run 39358379553（headSha `6badaa6049`）与本分支基点 `1d70db893f` 之间，`rust/` 与 `.github/` 零差异（`git diff --stat 6badaa6049 1d70db893f -- rust .github` 输出为空），测试面相同。

### 时长（job 墙钟，GitHub API started_at→completed_at）

| 拆前 run 39358379553 | 时长 | 拆后 PR run 39382771454 | 时长 |
|---|---|---|---|
| rust-check（全部 Rust 步串行） | **130m35s** | rust-check（check×2 + fmt + 两个脚本自测） | 1m42s |
| ↳ default targets 步 | 8.8m | rust-test-default | 10m37s |
| ↳ s_session bin 步 | 89.1m | rust-test-s-session (shard 0)，仅 tb02w | **31m00s**（最长） |
|  |  | rust-test-s-session (shard 1)，含 tb02c | 22m17s |
|  |  | rust-test-s-session (shard 2) | 10m47s |
|  |  | rust-test-s-session (shard 3) | 15m48s |
|  |  | rust-test-s-session-coverage | 0m08s |
| ↳ TB-02-C + s_content×2 + economic + capture_native 增量 + v3 history | 1.5+1.4+1.1+0.3+15.4+1.3 = 21.0m | rust-test-s-session-extras | 21m39s |
| ↳ backtest_bin --lib 步 | 9.3m | rust-test-backtest-bin | 6m20s |

- CI workflow 墙钟：拆前约 130.6 分钟（由 rust-check 决定），拆后约 31.2 分钟（06:50:45→07:21:58）。
- runner 总分钟数：拆前约 131 分钟，拆后约 120 分钟（9 个 job 相加）。各 job 自带 checkout 和编译开销，但分片后两条长测试不再争用，执行时间反而下降。

### 测试计数对拍（日志全部 `test result:` 行汇总）

| | 二进制段数 | passed | failed | ignored |
|---|---|---|---|---|
| 拆前 rust-check | 78 | 6839 | 0 | 311 |
| 拆后全部 rust-test-* | 81 | 6839 | 0 | 311 |

二进制段数 78→81：s_session bin 由 1 段变成 4 片各 1 段（+3），其余不变。

拆前按步：

- TB-02-C：1/0/0
- default：3037/0/148（66 段）
- s_session bin：507/0/11
- s_content lib：83/0/0
- s_content tests：53/0/0
- economic：77/0/7
- capture_native 增量：193/0/7
- v3 history：35/0/1
- backtest_bin：2853/0/137

拆后：

- s_session 4 片：1 + 149 + 179 + 178 = 507 passed，ignored 0 + 0 + 0 + 11 = 11
- 其余各 job 与拆前对应步逐项相同

**逐名对拍**：从两边日志提取「test 名 + 结果」的多重集，再用 diff 比对。

```
# s_session bin：拆前该步 vs 拆后 4 片之并
base-names.txt 518 / pr-names.txt 518 → diff 无输出（IDENTICAL_NAME_SETS）
# 全部 Rust 测试：拆前 rust-check 全 job vs 拆后全部 rust-test-* job
base-all.txt 7052 / pr-all.txt 7052 → diff 无输出（IDENTICAL_MULTISET）
```

两边都有少量结果行被测试自身输出打断，所以逐名行数 7052 小于 `test result:` 合计 7150，两边打断的情况一致。

**分片门输出**（PR run 39382771454）：

```
shard_plan index=0 count=4 total=518 sizes=1 149 179 189
shard_result index=0 assigned=1 executed=1 missing=0 extra=0 cargo_status=0
shard_result index=1 assigned=149 executed=149 missing=0 extra=0 cargo_status=0
shard_result index=2 assigned=179 executed=179 missing=0 extra=0 cargo_status=0
shard_result index=3 assigned=189 executed=189 missing=0 extra=0 cargo_status=0
shard_union total=518 ok=507 ignored=11 failed=0
```

### 磁盘（#1141 遥测，`df` 可用量最小值）

- 拆前 rust-check：68.7 GiB
- 拆后各 job：76.1 GiB（rust-test-default）到 83.5 GiB

拆后每个 job 只编自己那档 feature，峰值下降。

## 条目 4：check 名称变化（分支保护）

- `gh api repos/xy7365527-lang/NewChanlun/branches/main/protection` → `404 Branch not protected`；`gh api .../rulesets` → `[]`。
- 结论：当前**没有 required check**，合入本 PR 不需要同步改分支保护。
- 名称变化：
  - `rust-check` 名字保留，但只剩检查类步骤；
  - 新增 `rust-test-default`、`rust-test-s-session (shard 0)`～`(shard 3)`、`rust-test-s-session-coverage`、`rust-test-s-session-extras`、`rust-test-backtest-bin`。
- 以后设分支保护时，上述新名字都应列为 required；`rust-test-s-session-coverage` 是防漏测的汇总门，必须列入。

## 条目 5：远端（WSL）测时旁证

- 命令：`nc-remote eng-debt-1588/ci-time rust bash -c '... RUSTC_BOOTSTRAP=1 cargo test --locked --features s_session --bin s_structure_session -- --skip v3::financial_session --test-threads 4 -Z unstable-options --report-time'`
- 日志：`/tmp/nc-remote/20261020-144404-1d70db893f.log`（本机临时，未入仓）。
- 读数：505 条带 report-time 完成，合计 4113s，最长依次为：
  - evidence_graph_downgrade_is_strict 795s
  - parity_lock_804_on_frozen_tb02bcd_raw_ledgers 465s
  - parser_fractal_sources_reuse_full_raw_once_with_exact_parity 287s
- tb02c、tb02w 在 WSL 上跑了 37 分钟以上仍未结束。CI 拆后的实测读数已经足以定案，为不占用共享机，由本人手动终止（`EXIT=101`，原因是 SIGTERM，不是测试失败）。
- 用途：只作旁证（同样是这两条最慢、其余单条均不超过 795s），不作权重来源。

## 遗留与转票建议

1. **最长 job 的下限是 tb02w 单条测试本身**（拆后 1755s）。在本分片允许的手段内已无可再压的空间。如要继续压，有两条路，都超出本分片手段限定，建议转票由主控裁定：
   - 测试 profile 开优化（如仅 CI 设 `CARGO_PROFILE_TEST_OPT_LEVEL=1`，debug-assertions/overflow-checks 不变），未测；
   - 把 tb02w/tb02c 拆成多条测试（改测试代码，要由测试作者评估等价性）。
2. **权重表是粗估**，读数含争用。均衡度靠 PR run 验证，覆盖与权重无关。将来某片明显变长时，用该片日志重估后整表替换（刷新口径写在表头）。
