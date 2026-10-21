# PR #1611 非作者独立评审（#1588 p3fix 分片：A1 / A3 / A6）

- PR：https://github.com/xy7365527-lang/NewChanlun/pull/1611（head `4de612c241`，base `main` @ `a0522447ca`）
- 评审模型：claude-fable-5
- 评审日期：2026-10-20
- 评审方式：只读。未改代码、未推送、未评论 PR、未合入。本机不跑 cargo，Rust 行为经 `nc-remote`（WSL，`NC_TARGET=eng1588`）复核。

## 结论：PASS-WITH-NITS

三项修复都正确、范围克制、不掩盖问题、不降覆盖、不误改生产行为。作者证据报告中可复核的主张我都复核了并与之一致（见「实际复核过的事实」）。无阻断项；下列发现均为 nit / 后续项，由主控裁定是否在本 PR 内处理或转票。

## 两轴结论

### ① 正确性 / 风险

| 项 | 判断 | 说明 |
|---|---|---|
| A1 `gitEnvironment()` 白名单 | 正确 | 预检 5 次 git 调用全是本地读（`rev-parse --show-toplevel`、`ls-files -v`、`symbolic-ref`、`rev-parse HEAD`、`status`），且都以 `-C root` 定位；白名单 `PATH/HOME/USER/LOGNAME/TMPDIR/LANG/LC_*/XDG_CONFIG_HOME` 足够，`HOME` 透传意味着用户全局 gitconfig（含 fsmonitor 等）仍生效。**不透传宿主 `GIT_*` 的取舍我同意**：`GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` 透传反而会把预检引到 `-C` 之外的仓库，而 `GIT_SSH_COMMAND`/`GIT_CONFIG_*` 对本地只读预检无意义。仓内 `.sandcastle/` 没有任何依赖 `GIT_*` 的接线（只有 `prebake-kernel.sh` 自己设 author/committer）。macOS 专用入口（`launchctl`），不需要 `SYSTEMROOT` 一类 Windows 变量。 |
| A1 两条新测试 | 有效 | 我把新测试文件跑在**修前**的 `codex-child.ts`（`a0522447ca` 版本，拷到 /tmp）上：集成测试按预期失败，断言信息正是「git 环境含 CLIPROXY_LOCAL_KEY」；修后 71/71 通过。`finally` 对 `PATH`、`CLIPROXY_LOCAL_KEY`、`GH_TOKEN`、`OPENAI_API_KEY` 的恢复逻辑按读码是完整的（`saved` 在改动前采样，`undefined` 走 delete）。 |
| A3 bi.md 指针 | 正确、纯指针 | diff 仅 1 行（1+/1-），定义文字未动；目标文件存在且其头部自述即为 #1343 受影响代码清单、名分为工作草稿，新措辞「工作草稿副本」与 AGENTS.md 的五类名分一致。 |
| A6 `signal.rs:1588` | 正确 | `git show 1592bf05ca` 确认该提交把 `(Some(Up), Up)` 改成了 `(Some(Down), Down)`，本 PR 是逐字恢复；与生产 `signal/gates.rs:67` 的 `(Some(Direction::Up), Direction::Up) if last_center.zg < end.price` 同判据。函数在 `#[cfg(test)]` 下（`signal.rs:1515`），生产路径不受影响。 |
| 是否掩盖问题 / 降覆盖 | 否 | 没有删测试、加 `#[ignore]`、缩窗口或排除规则。普查全历史 FAIL 被照实写明并指向 main 既有前置失败（#1392/#1393）。 |
| CI | rust-check / test / s-observer / fixture-drift 绿；DevSkim 红为 main 既有 | 我比对了本 PR 与 base `a0522447ca` 的 DevSkim 失败清单：`codex-child.ts` / `codex-child.test.ts` 命中都是 13 条且规则相同，只是行号因新增函数整体 +11；本 PR 新增/改动的行没有新命中。DevSkim 由 #1593 分片处理。 |

### ② 是否满足 #1588 票面验收

- 票面第 5 项「P3/观察项中当前就错的部分」：本分片任务书（`prompts/p3fix.md`）限定 A1/A3/A6 + 开 A5/Q1/Q2 三票，PR 全部覆盖。三张票 #1608/#1609/#1610 已开（标题、label 未逐一核对正文，不在本评审范围）。
- 「逐条 现象、复现命令与输出、根因、处置」：`p3fix/REPORT.md` 对三项都有，格式满足；总账 `LEDGER.md` 由主控汇总，不在本 PR。
- 「每个修复非作者独审」：本文件即独审记录。
- 「不删测试」：满足。
- 三个独立提交各含 #1588，可按 squash 规则合入。
- 注意：`p3sweep.md` 列的 4 项「可当场小修」里 **A4**（`content_producer.rs:3748` 过时注释）不在本分片任务书内，PR 未处理属正常，但主控需确认 A4 另有去处。

## 逐条发现

1. **[低 / nit] `.sandcastle/delivery-bundle.ts:126` 仍是 `env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" }`。**
   理由：与 A1 同一类问题——若 delivery-bundle 在管理者进程里跑而该进程环境含 `CLIPROXY_LOCAL_KEY`，这里的 git 子进程同样拿到密钥。它不在 p3sweep A1 的登记范围（只点名 codex-child.ts），不算本 PR 的缺口，但修法现成（复用 `gitEnvironment()` 或同等白名单）。
   建议：本 PR 顺手补一个提交，或开一张小票挂 #1588。不阻断。

2. **[低] A6 只有静态依据，无行为测试钉住上涨臂。**（作者「请评审重点看」第 3 点）
   理由：普查在所有可跑窗口里「环2过局部趋势门」恒为 0，环 3 的 broke 判据从未被求值，修前修后输出逐行相同——作者已如实写明。当前唯一消费方 `pipeline_geometry.rs:821` 是 `#[ignore]` 的真实数据普查，常规 CI 不跑；若将来再被误改，仍只靠 `unreachable pattern` 告警兜底（而 A5 票正是告警已多到 84 条、没人看）。
   建议：补一个小型合成夹具单测（一个上涨趋势、`c.zg < end.price` 的段），断言 `f.s_broke == 1`，并同时覆盖 `(Some(Up), Down)` 落入 `_ => false`。可在本 PR 内做也可随 A5 票一起。不阻断，因为修复本身是对 #988 误改的逐字回退且与生产判据静态一致。

3. **[低 / 证据完整性] REPORT.md 中「`cargo test --release --lib theta_v0::classifier` → 919 passed」在本机 `/tmp/nc-remote/` 没有对应日志。**
   理由：A6 其它四次 nc-remote 运行（修前/修后 cargo check、两次普查）都有日志（`20261020-184033/184213/184549/184656/184813`），唯独这一条没有。我自己重跑复现了同样结果（见下），所以主张为真，但报告应能回溯。
   建议：REPORT.md 补该次 run id，或注明日志未保留。nit。

4. **[nit] 集成测试是文件最后一条，`finally` 若恢复不全不会被后续测试暴露。**
   理由：读码确认恢复正确，仅指出该测试自身不验证恢复。
   建议：可在 `finally` 后不做任何事；若想钉住，可在测试末尾 `assert.equal(process.env.PATH, saved.PATH)`。可忽略。

5. **[信息] 关于 `GIT_*` 取舍的裁定意见。**
   同意不透传。补充两点给后来者：(a) `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM` 不透传意味着宿主用这两者隔离配置时，预检会回落到 `$HOME/.gitconfig` + 系统配置——对只读预检无害；(b) `GIT_EXEC_PATH` 不透传在非标准安装的 git 上理论上可能影响子命令定位，但 git 默认从自身二进制位置推导，实际风险可忽略。白名单不缺平台必需变量。

## 实际复核过的事实与命令

| # | 主张 | 复核方式 | 结果 |
|---|---|---|---|
| 1 | A1 修后两测试文件 71/71 通过、tsc exit 0 | 本机临时软链 `/tmp/ge1547deps/node_modules`（与 package.json 同套依赖），`node --import tsx --test .sandcastle/codex-child.test.ts .sandcastle/delivery-bundle.test.ts`；`./node_modules/.bin/tsc -p tsconfig.sandcastle.json`；完毕后删软链，`git status` 清洁 | `tests 71 / pass 71 / fail 0`；tsc exit=0 |
| 2 | A1 新集成测试在修前会失败（测试有效性） | 把 `.sandcastle/` 拷到 `/tmp/p3fix-review-prefix/`，用 `git show a0522447ca:.sandcastle/codex-child.ts` 覆盖，跑 `--test-name-pattern="1588 A1"` | 纯函数测试 `gitEnvironment is not a function`（预期，修前无此函数）；集成测试 `AssertionError: git 环境含 CLIPROXY_LOCAL_KEY` |
| 3 | A6 修后 `theta_v0::classifier` 919 passed、无 unreachable pattern | `NC_REPO=<工位> NC_TARGET=eng1588 nc-remote eng-debt-1588/p3fix rust bash -c 'cargo test --release --lib theta_v0::classifier 2>&1 \| grep -aE "unreachable\|signal.rs:1588\|test result\|^error\|warning: .*generated"'`（head `4de612c241`，日志 `/tmp/nc-remote/20261020-213958-4de612c241.log`） | `(lib test) generated 52 warnings`；`test result: ok. 919 passed; 0 failed; 10 ignored`；无 unreachable 行；EXIT=0 |
| 4 | A6 修前/修后 cargo check 告警 53→52、unreachable 1→0 | 查作者日志 `/tmp/nc-remote/20261020-184213-6958eefcb8.log`（修前）与 `20261020-184033-1b27e492b0.log`（修后） | 修前第 137 行 `src/theta_v0/classifier/signal.rs:1588:13: warning: unreachable pattern`，`(lib test)` 53；修后 `unreachable_pattern_total=0`，`(lib test)` 52 |
| 5 | A6 普查修前修后无法区分、全历史 FAIL | 查 `20261020-184549-1b27e492b0.log`（2020-10..2021-04 窗口）、`20261020-184813-1b27e492b0.log`（四窗口） | 各级「环2过局部趋势门=0 → 环3破最后中枢=0」，`buy1=0 sell1=0`；2017-09 起窗口 L0 segments=633 ok；全历史/2023 起窗口 L0=0 FAIL（与报告一致） |
| 6 | A6 回归来源是 `1592bf05ca`（#988） | `git show 1592bf05ca -- rust/src/theta_v0/classifier/signal.rs` | 可见 `-(Some(Direction::Up), Direction::Up) => c.zg < end.price` / `+(Some(Direction::Down), Direction::Down) => c.zg < end.price` |
| 7 | A3 `AFFECTED-CODE.json` 从未存在 | `git log --all --diff-filter=A -- '*AFFECTED-CODE.json'`；`git ls-files \| grep AFFECTED-CODE` | 前者无输出；后者只命中 `.md`（1342–1358、1478、1489、1490）；目标文件 `1343-AFFECTED-CODE.md` 存在 |
| 8 | PR 的 DevSkim 红是 main 既有 | `gh pr checks 1611`；`gh run view 39411448476 --log-failed` 与 base 对应 run `39406786934` 比对 codex-child 命中 | 两边 codex-child 命中均 13 条、规则相同、行号 +11 偏移；本 PR 改动行无新命中；`rust-check` 2h21m 绿 |
| 9 | PR 范围与任务书一致、无其它 git 子进程漏改 | 读 `prompts/p3fix.md`、`p3sweep.md`；`grep -rno "GIT_[A-Z_]*" .sandcastle/`；`grep -n "execFileSync(\"git\"" .sandcastle/*.ts` | 任务书限定 A1/A3/A6；codex-child.ts 仅此一处 git 调用；`delivery-bundle.ts:124-127` 仍为旧模式（发现 1） |
