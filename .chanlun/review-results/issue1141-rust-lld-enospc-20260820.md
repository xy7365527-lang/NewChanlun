# #1141 rust-lld signal 7 / ENOSPC 诊断与 CI 处置

> 名分：工作草稿，绑定 [#1141](https://github.com/xy7365527-lang/NewChanlun/issues/1141)。
> 日期：2026-08-20。
> 基线：`88d75076419670b210f16453855ea805dfbf6566`。
> 实现边界：CI 行为只改 `.github/workflows/ci.yml`，本稿仅同步证据边界；不改 Rust 业务代码、测试、linker 或 runner/toolchain 选择。

## 结论

三轮失败的决定性证据不是 LLVM 的报错模板，而是第三轮在同一 `cargo test --all-targets` 末尾明确给出 `No space left on device (os error 28)`。三轮的受害 bin 一直漂移，且 Cargo 在并行链接多个目标。由此裁定：

- 直接根因是 `rust-check` job 磁盘耗尽；
- `rust-lld` signal 7 是低磁盘条件下的受害表现，现有证据不支持把它定性为 LLVM regression；
- 三轮都恢复同一份 1,933,299,350 B 的旧 Cargo cache archive；该 archive 是 registry、git 与 target 的合包，不能把总大小归到 `rust/target`，也没有证据证明其中内容损坏；
- 固定某个 Rust binary 的解释不成立，因为失败目标跨轮漂移。

最终处置只降低磁盘占用和并行峰值：`rust-check` 不再缓存 `rust/target`，设 `CARGO_INCREMENTAL=0`，两档原有测试各加 `--jobs 1`。移除 target 后的实际节省量须由候选 workflow 的 cache 后与测试期 telemetry 实测，不能从旧合包大小推出。仍使用 `ubuntu-latest`、镜像自带的 stable Rust 和原 linker。

## 三轮一手证据

| 轮次 | SHA / job | 恢复的 rust-check cache | 失败位置与受害目标 | 磁盘证据 |
|---|---|---|---|---|
| 1 | [run 32313048647 attempt 1 / job 96259762175](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32313048647/job/96259762175)，`31197f7e60212a3e0e0891ebe9a56363e0bd09bb` | exact key `Linux-cargo-42da…582`；registry/git/target 合包 `1,933,299,350 B` | `cargo test --all-targets`；`p119_l5_decompose` 与 `p76_case2_replay` 链接时 signal 7 | 当轮未记录 `df`；不能单靠本轮声称 ENOSPC |
| 2 | [run 32313048647 attempt 2 / job 96261345517](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32313048647/job/96261345517)，同 SHA、failed-jobs rerun | 同一 exact key、同一 registry/git/target 合包 `1,933,299,350 B` | 同一命令；受害目标漂为 `p123_fast_replay` 与 `p122_replay_profile` | GitHub runner 低盘诊断在失败后报只余 `88 MB` |
| 3 | [run 32315548218 / job 96266878066](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32315548218/job/96266878066)，`88d75076419670b210f16453855ea805dfbf6566` | 仍是同一 exact key、同一 registry/git/target 合包 `1,933,299,350 B` | 同一命令；`p104_trend_div_funnel` 链接 signal 7，另一并发目标 `p117_h4_attribution` 随即失败 | signal 7 后约 18 ms 明文：`No space left on device (os error 28)` |

第三轮日志的相邻顺序如下。它把前两轮的“低盘强嫌疑”闭合成直接证据：

```text
2026-08-20T00:04:59.4403887Z error: linking with `cc` failed: exit status: 1
2026-08-20T00:04:59.4475929Z collect2: fatal error: ld terminated with signal 7 [Bus error], core dumped
2026-08-20T00:04:59.4654399Z No space left on device (os error 28)
```

三轮都先通过两档 `cargo check` 与 `cargo fmt --check`，再在 default-target tests 的链接扇出阶段失败。轮 1 的两个 signal 7 相隔约 2.44 秒，轮 2 的两个相隔约 354 ms；日志还有 `build failed, waiting for other jobs to finish...`。这与共享磁盘在并发链接阶段越过容量边界相符。

证据边界也要写清：失败 workflow 没有运行 `rustc -V`。`p122_replay_profile` 的 stack 确认实际崩溃程序是 stable sysroot 内的 bundled `rust-lld`，SONAME 含 `libLLVM.so.22.1-rust-1.97.1-stable`；这不等于可以把当时或未来的 `stable` alias 断言为固定 `1.97.1`。候选 workflow 只记录实际版本，不做版本相等断言。

## 变更与约束核对

### 降低峰值

1. `actions/cache@v4` 在 `rust-check` 中只保留 `~/.cargo/registry` 与 `~/.cargo/git`。旧 key namespace 停用，`rust/target` 完全移除。
2. job 级 `CARGO_INCREMENTAL=0`，不再保留本轮无用、但会占盘的 incremental 产物。
3. 原两档覆盖保持不变：
   - `cargo test --all-targets --jobs 1`
   - `cargo test --lib --features backtest_bin --jobs 1`
4. 不换 linker，不 skip/删除测试，不改业务 Rust，不把 `ubuntu-latest` 伪装成 `ubuntu-24.04` pin，也不固定 stable Rust 版本。

### Fail-loud 与取证

每档测试前都执行同一门槛：当前文件系统可用空间必须不少于 `2,147,483,648 B`（2 GiB），否则先打印 `free -b`、`df -B1 -T .`、`du -sx -B1 target`，再用 GitHub `::error` 退出。2 GiB 是安全门槛，不是假装测得的精确链接需求：它约为历史 14 GB runner 容量的 14%，并远高于已失败的 88 MB；旧 `1,933,299,350 B` 是 registry/git/target 合包，不能拿它当 target 节省量。真正峰值与移除 target cache 后的实际节省继续由 telemetry 记录。

以下边界都保留资源快照：checkout/工具链初态、cache 后、两档 check 前后、fmt 前后、两档 test 前后。两档 test 期间每 10 秒后台采样一次 `free`、`df` 和 cargo/rustc/rust-lld 进程数；退出 trap 无论绿红都停止 sampler、打印最终 `du`。cargo 失败时原样返回 cargo 状态；cargo 成功时 sampler 意外退出/失败或最终 snapshot 失败都会发出 `::error` 并令步骤失败，脚本主动 SIGTERM sampler 的预期 143 状态不误报。

workflow 新增 `workflow_dispatch`。它只提供合入后的可控复验入口，不改变 push 和 pull_request 原有触发。

## 验收与退场条件

本提交不能在未合入、未 push 的本地 worktree 冒充远端验证。合入后需用同一 SHA、同一 workflow 连续手动触发两次，并逐轮记录：

- `rust-check` 结论；
- runner image、实际 rustc/cargo/linker 版本；
- 两个 preflight 的 `available_bytes`；
- sampler 观测到的最低余盘；
- `rust-check` job 总耗时与两档 cargo test 各自耗时；
- 两档 cargo test 是否完整执行，是否出现 ENOSPC、signal 7 或 LLVM crash。

两次都通过才满足 #1141 的稳定性验收。任一轮触发 2 GiB 门槛，按预期 fail-loud，不能手工 rerun 当修复；应带着 `df/free/du` 证据另开容量处置。
在这两轮数据落盘前不预设任何分钟级上限或其他 timeout；记录实际 job 与两档 test 耗时后再裁。

退场分两层：

1. `workflow_dispatch` 与高频 sampler 属诊断设施。合入后连续 10 次 `rust-check` 通过、覆盖至少 7 天，且两档测试期间最低余盘均不低于 4 GiB、无 ENOSPC/signal 7，方可在绑定 #1141 的后续提交中移除。边界 `df/free/du` 和 2 GiB fail-loud 门保留。
2. `--jobs 1` 与 `CARGO_INCREMENTAL=0` 属当前稳定处置。若要放宽，必须另票做同 workflow 对照，并连续两次证明默认并行下最低余盘仍不低于 4 GiB。`rust/target` cache 不自动恢复；重新引入也要独立容量证据，不能只因若干轮变绿就复活旧 key。

截至本草稿落盘时，合入后的两轮 `workflow_dispatch` 尚未执行。
