# PR #1592 非作者独立评审（#1588 ci-time 分片）

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1592（head `1e4858d4fd`，base `main` @ `1d70db893f`）
- 评审模型：claude-fable-5
- 评审日期：2026-10-20
- 评审方式：只读。拉 PR diff 与描述，读 `.github/workflows/ci.yml`（新旧两版逐步对照）、`.github/actions/rust-job-setup/action.yml`、`.github/scripts/test-shard.py`、`test_test_shard.py`、`s-session-test-weights.tsv`、`run-cargo-with-telemetry.sh`、`test-feature-delta.sh` 改动、证据文件 `ci-time-1588.md`；本机重跑分片脚本自测；用 `gh` 拉基线 run 与末次 PR run 的 job 时长、日志，独立重算测试计数与逐名集合。未改代码、未推送、未评论 PR。

## 结论：PASS-WITH-NITS

PR 做到了票面「CI 时长有余量（拆分作业/缓存/并行，不删测试）」：最长 job 从 130m35s 降到 31–67 分钟（视 runner 机型），原 rust-check 的每一条 cargo 命令逐字保留，测试计数与逐名集合与拆前完全一致，分片覆盖由脚本当场断言 + 汇总 job 二次断言，失败路径都是红而不是绿。四条 nit 都不阻塞合入。

## 轴①：正确性 / 风险

### 没有掩盖问题
- `test-shard.py` 的失败路径逐条看过：`--list` 为空即 `fail`；cargo 非零退出码原样透传（先于「执行集==分配集」判断返回）；执行集与分配集不符即 `fail`；越界 index、缺片、各片全集不一致、分片重叠/缺漏全部 `fail`。没有任何「跳过」分支。
- `rust-test-s-session-coverage` 用 `needs` + `if: always()`：分片被取消/超时时它照样跑，并因缺 artifact 目录（`len(dirs) != count`）或缺 `executed.tsv` 而红。`if-no-files-found: error` 保证空产物也红。`fail-fast: false` 保证一片红不取消其它片，覆盖证据完整。
- `--exact` 多名过滤真实可用：末次 run 四片 `shard_result ... missing=0 extra=0`，汇总 `shard_union total=518 ok=507 ignored=11 failed=0`，与拆前单步 `507 passed; 11 ignored` 一致。

### 没有降低覆盖
- 旧 `rust-check` 共 11 条 cargo 命令（check×2、fmt、TB-02-C、default、s_session bin、s_content lib、s_content tests、economic、capture_native 增量、v3 history、backtest_bin），新版本逐字出现在 5 个 job 里，无增删改（`--skip v3::financial_session` 改为在 `--list` 阶段应用，等价）。
- 我独立重算末次 PR run 39395041855 的 8 个 Rust job 全部 `test result:` 行：**81 段 / 6839 passed / 0 failed / 311 ignored**；基线 main run 39358379553 的 rust-check：**78 段 / 6839 / 0 / 311**。+3 段即 s_session bin 由 1 段变 4 片。
- s_session bin 逐名对拍：基线该步 518 条「名+结果」与 PR 四片之并 518 条 `diff` 为空（IDENTICAL_NAME_SETS）。
- 权重表只影响 LPT 排序，`partition` 对 `sorted(set(names))` 全量分配，自测 `test_empty_weight_table_still_covers_everything` 与 `test_union_is_full_set_and_disjoint_for_any_count`（count 1..7）覆盖了「表空/过期也全覆盖」。

### 没有误改行为
- composite action 把 #1141 的三步原样搬入（记录 runner/toolchain/linker、只缓存 registry+git、记录 cache 结果），cache key 与 restore-keys 未变；`CARGO_INCREMENTAL=0`、`--jobs 1`、不缓存 target 保持。
- `mkdir -p target` 是必要的：`run-cargo-with-telemetry.sh` 的 `boundary_snapshot` 对 `du -sx -B1 target` 不带 `|| true`，失败即 `Initial resource snapshot failed` 退出；拆后测试 job 的第一个 cargo 步就是被包装的那步，此前由 `cargo check` 顺带建目录。空目录不影响构建。
- `rust-test-s-session-extras` 保留各步 `if: always()`，job 结论仍由失败步决定；TB-02-C 在 extras 与分片各跑一次，与拆前一致（计数对拍已含）。
- `test-feature-delta.sh` 只改注释。

### 发现（按严重度）

1. **nit / 低** — `.github/workflows/ci.yml:272-279`：`actions/upload-artifact@v4` 默认不允许同名 artifact 重复上传。若某片因 tb02w 偶发超时而被单独「re-run failed jobs」，第一次尝试已经（经 `if: always()`）上传了 `s-session-shard-N`，重跑再上传同名会撞冲突，把本已通过的重跑判红（假红，不是假绿）。建议加 `overwrite: true`。本次三轮 run 均一次通过，未实际触发。
2. **nit / 低** — `.github/workflows/ci.yml:245`：分片 `timeout-minutes: 90`，最慢机型（EPYC 7763）实测 66m50s–66m57s，余量约 23 分钟（约 35%）。已见机型系数达 2.2×，若再遇更慢机型或 tb02w 再涨三分之一即假红。作者已在遗留项诚实披露并建议转票（test profile 开优化或拆 tb02w/tb02c），我同意；补一句：「≤90 分钟」靠的是实测时长而不是 timeout 值，若主控更看重不出假红，timeout 可放到 100–120 而不影响票面达成。
3. **nit / 低** — `.github/scripts/test-shard.py:167`：分片被 kill（超时）时 `executed.tsv` 不会落盘，verify 读取时抛 `FileNotFoundError` traceback 退出非零。结果仍是红，但没有 `::error title=` 的结构化提示，排障时不如其它路径直观。可在 verify 里对缺文件给出 `fail("Shard executed.tsv missing", ...)`。
4. **信息 / 不阻塞** — `.github/scripts/test-shard.py:32,131-137`：结果行靠正则逐行解析。tb02w 会起 `python3` 子进程，子进程直接写继承的 fd 1 可能把 `test X ... ok` 行切断，此时判 `executed != assigned` → 红（保守方向，不会漏测而绿）。我核了基线该段 4218 行日志，`test ` 开头而不带 `... ok|FAILED|ignored` 的行为 0；三轮 PR run 四片均 missing=0。证据文件说的「被打断的行」出现在 default targets 的其它二进制，不在本分片范围。将来若出现偶发红可改用 libtest `--format json`，当前无需动。

## 轴②：是否满足 #1588 票面验收（本分片范围）

| 验收项 | 状态 |
|---|---|
| CI 时长有余量（拆分作业/缓存/并行，不删测试） | **满足**：最长 job 130m35s → 66m50s（最慢机型）/ 31m（最快机型），相对各自 timeout 的余量从 20 分钟升到 23–59 分钟；workflow 墙钟约 131m → 31–67m。手段只用拆作业 + 并行 + 按名分片，不删测试、不加 ignore、不缩范围（计数与逐名集合验证如上） |
| 逐条 现象/复现命令与输出/根因/处置 | **满足（分片证据文件）**：`.chanlun/review-results/eng-debt-20261020/ci-time/ci-time-1588.md` 六个条目齐全，复现命令可重放（我按同样命令重算了计数与时长）。汇总 `LEDGER.md` 由主控合并，不在本 PR 范围 |
| 非作者独审 | 本文 |
| squash 合入走用户闸 | 不在评审范围；提示 check 名称新增 7 个，当前 main 无分支保护（我核实 `branches/main/protection` 404、`rulesets` 为 `[]`），合入无需同步设置 |

## 我实际复核过的事实与命令

1. `gh pr list --head eng-debt-1588/ci-time` → PR #1592；`gh pr view 1592 --json body,files,commits`；`gh issue view 1588`。
2. `git diff 1d70db893f..HEAD -- .github/workflows/ci.yml` 与 `git show 1d70db893f:.github/workflows/ci.yml` 逐步对照：旧 rust-check 的 11 条 cargo 命令逐字保留在新 5 个 job 中。
3. 本机 `python3 .github/scripts/test_test_shard.py -v`（Python 3.14.6）：**Ran 10 tests … OK**。
4. `gh run view 39395041855 --json jobs`：conclusion success；shard 0 08:40:05→09:46:55 = **66m50s**，shard 1 34m11s，shard 2 20m53s，shard 3 9m42s，default 7m12s，extras 13m56s，backtest-bin 10m11s，rust-check 2m12s，coverage 0m09s。
5. 四片 job 日志 grep：shard 0 落 **EPYC 7763**，`tb02w` 完成 `finished in 3864.27s`；`shard_plan … total=518 sizes=1 149 179 189` 四片一致；`shard_result` 四片 missing=0 extra=0；coverage job `shard_union total=518 ok=507 ignored=11 failed=0`。与 PR 描述逐字一致。
6. 计数重算（awk 汇总全部 `test result:` 行）：PR run 8 个 Rust job = 81 段 / 6839 / 0 / 311；基线 run 39358379553 job 118418447937（02:20:33→04:31:08 = 130m35s）= 78 段 / 6839 / 0 / 311。
7. 逐名对拍：基线 s_session 段（日志第 14018–18235 行）提取 518 条「test 名 … 结果」，与 PR 四片提取的 518 条 `diff` 为空；该段被切断的结果行为 0。
8. `gh repo view --json isPrivate` → PUBLIC；`gh api .../branches/main/protection` → 404 Branch not protected；`gh api .../rulesets` → `[]`。
9. 读 `run-cargo-with-telemetry.sh`：确认 `boundary_snapshot` 中 `du -sx -B1 target` 失败即退出，`mkdir -p target` 必要。
10. 读 actions/upload-artifact README：同名 artifact 默认冲突失败，`overwrite` 输入可删后重传（发现 1 的依据）。
