> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# #1370 CI 覆盖窄检查

结论：**固定 f635 的现有 CI 不会编译或执行 `s_structure_session` 目标。** 建议的专用 `cargo test` 步骤是最小正确补法；但固定旧版二进制中 **0 个 `#[test]`**，所以还必须等作者新增测试落到最终载体，再核实际非零执行结果。仅命令存在不算回归锁运行。

基线：`f6354d3e7fb7fc863ca569e8786bd62807e85c0e`。所有 Cargo、CI、wrapper 和旧测试载体都用 `git show <完整SHA>:<path>` 读取，未读取作者正在修改的 Q 工作文件。`f635` 与 tree/blob 短号有歧义，已解析唯一 commit 后使用完整 SHA。逐文件 SHA 见[机器报告](../payload/inputs/TB01-A-CI-COVERAGE-CHECK.json)。

## 已有命令的实际覆盖

| 固定 CI 命令 | 对 s_session 的结果 |
|---|---|
| `cargo check --all-targets`（ci.yml:211） | default feature 为空，`required-features=["s_session"]` 的 bin 被跳过；check 本身也不执行断言 |
| `cargo check --all-targets --features backtest_bin`（222） | backtest_bin 只启用 nautilus/sha2，未启用 s_session；仍跳过 |
| `cargo test --all-targets --jobs 1`（249） | 会执行默认 lib/目标测试，但未启用 s_session，跳过该 bin |
| `cargo test --lib --features backtest_bin --jobs 1`（256） | 只选 lib，且未启用 s_session；不执行 bin 测试 |
| `cargo fmt --all -- --check` | 只检查格式，不证明该 feature 编译或测试执行 |

Cargo.toml:48 的 `default=[]`、63 的 `s_session=["dep:sha2","dep:rusqlite"]` 和 77–79 的具名 bin / required-features 共同决定上述结果。**`--all-targets` 不等于 `--all-features`。** `.github` 固定内容未发现其他启用 s_session 的命令。

旧测试并非全部漏跑：`rust/src/theta_v0/classifier/local_shape.rs:223–346` 有四个普通、未 ignore 的 CC-006 测试（four_branch_oracle、equal、inclusion、sliding_window_count）；`classifier/mod.rs:69` 无 feature 门公开该模块，默认 `cargo test --all-targets` 已包含它们。它们证明分类器局部行为，不测试 S 的身份重放、SQLite 发布、wire 整数或新修复。

## 最小补法

在现有 `rust-check` job 中，放在 default tests 后、backtest_bin tests 前，沿用 `working-directory: rust` 和 telemetry：

```yaml
      - name: cargo test — s_session feature (#1370)
        run: ../.github/scripts/run-cargo-with-telemetry.sh cargo test --locked --features s_session --bin s_structure_session --jobs 1
```

这条命令启用需要的依赖，选择具名二进制测试 harness，**默认实际执行其中未 ignore 的测试**。不需要再加一条只编译的 s_session check 来替代它，也不需要把高成本 `--all-features --all-targets` 加进现有矩阵。

本地只验证测试时，可在 `rust/` 用同一 Cargo argv：

```bash
cargo test --locked --features s_session --bin s_structure_session --jobs 1
```

Linux CI wrapper 依赖 GNU 工具，不把在 macOS 直接执行该 wrapper 当成必要步骤。

## 明确环境前件

- **现有 Rust/Cargo/linker 和 C 编译器。** `rusqlite` 使用 `bundled`；固定 lock 已有 rusqlite **0.40.2**、libsqlite3-sys **0.38.2**、cc **1.2.65**，不要求另装系统 SQLite 或 sqlite3 CLI。现有 job 已记录 `cc`，无需因本 feature 新增数据库服务。
- **沿用现有 PyO3 的 Python/链接前件。** s_session 新增的 feature 依赖只含 sha2/rusqlite，不启用 Nautilus 或 extension-module，不引入新的 Python 包前件。`rust-check` 与另一个 `test` job 隔离，不继承后者 setup-python；此处仍依赖 runner 原有 Python，YAML 只读核对不能声称远端环境已经验证。
- **lock 与最终源码一致。** 固定 lock 已包含当前 feature 依赖，`--locked` 合理；作者若新加测试依赖，应同时交付一致的 lock，不能让 CI 临时改锁。
- **telemetry 既有前件。** wrapper 要求 Linux 的 `free/df/du/ps/awk`、存在的 `target/`，及至少 **2 GiB** 可用磁盘；前面的 cargo check 已建立 target。它保留 cargo 的失败退出码，不能加 `continue-on-error`。不扩大整套 bin 测试可减少已有磁盘风险。
- **新增测试载体仍待接回。** 若测试放在 `rust/tests/<name>.rs`，`--bin` 不会运行该 integration target，须补具名 `--test <name>`；若新增测试实际调用 Python/Node/外部程序，再按最终文件声明相应前件。当前旧载体不足以提前宣告这些工具必要或已具备。

`--jobs 1` 只限制 Cargo 构建并行度，不等于 Rust 测试线程为 1。只有最终测试确有共享进程状态且未隔离时，才需要处理测试并发；不预先添加全局串行参数掩盖测试设计问题。

## 后续验收必须保留的证据

作者最终测试接回后，核对具名测试位于被选目标、无 ignore/额外 feature 门，并运行上述**实际执行**命令，保留测试名、非零 count 和退出码。`-- --list` 只枚举，`--no-run` 只编译，二者都不能替代执行。固定旧版该 bin 没有测试，即使新命令能 exit=0，也可能只是 `running 0 tests`。

本轮只读检查和 TOML 解析已完成，未运行 Cargo、CI workflow、端口或数据库；未修改任何仓文件或 GitHub，只新增本报告及 JSON。未审作者最终新增测试，未取远端 job 日志，**不声称当前 CI 已通过该新增回归**。Ubuntu job 也不替代 macOS fullfsync、launcher 或真实最高缝验收。

现有 workflow 的 push/PR 基线保持 main；本建议不改变批准的隔离分支基线、不写 main、不触发发布。局部 CI 覆盖补齐不关闭 #1370、TB-01 或 #1323。
