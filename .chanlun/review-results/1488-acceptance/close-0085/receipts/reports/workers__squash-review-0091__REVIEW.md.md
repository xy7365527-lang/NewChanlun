# 收据 workers/squash-review-0091/REVIEW.md

- 原件：`workers/squash-review-0091/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`89be2b8b52f52bb7bfaec711b3cdb7e2c212891d23789877ac32b9b7669e0c06`
- 字节数：16546；行数：200

## 关键行（行号: 原文）

```
1: # #1488 0091：squash 面与 PR 非作者独审
10: ## 结论：**FAIL**（1 个 P1，2 个 P2，5 个 P3）
12: 主要问题是 P1：P58 全量 fmt 后没有同步 bsp 白名单锚，squash 树会在 main ci.yml 的 `rust-test-default` 里红，`rust-test-backtest-bin` 按同样的道理也会红。这是确定性失败，不是环境原因。本票自己的验证没有覆盖 `cargo test --all-targets`，所以没有发现它。
22: 第 1、4、5、6 项需要 close-0085/REPORT.md 和 PR。写报告时这两样都还没有（PR 未开），所以只做了预审，没有给最终结论。P1 修好后 squash 会重建，届时要对新的 squash 和 PR 复审这四项，同时复跑 `cargo test --all-targets`。
26: ## P1
28: ### P1-1：fmt 把 content_producer.rs 的 raw_bar_at 绑定行从 :3109 推到 :3114，bsp 白名单没有跟着改，`no_stray_bsp_binding_rewrites_outside_registry` 失败
41:   - squash：FAILED。
56: ## P2
58: ### P2-1：gate_warn.py 插入的 28 行无条件 allow，reason “各构建组合均无使用方（可能只在其他 testonly feature 下用）”不属实或只是推测，与 main #1632 体例不一致
83: ### P2-2：native 档（`s_session,s_capture_native_testonly`）的 `RUSTFLAGS=-D warnings cargo check --all-targets` 失败；main 在这一档原本没有告警，合并后的 squash 有 59 条
95:   - 判 P2 而不是 P1：负责人的要求明确没有满足，但没有造成门禁红或行为错误。
98:   - 或者由负责人明确裁定 native 档不在“四档”内。
100: ## P3
102: 1. `fetch_update` 还剩两处：`rust/src/bin/s_session_v3/request_audit.rs:478` 和 `rust/src/s_content.rs:8009`。main #1632 已经全部改为 `try_update`；本票取 ours 时把旧写法带了回来（与 P2-2 部分重叠）。
115: ## 第 2 项：pre-squash 提交 8a1924993b（父 327aa94c39）——**通过**
117: - 改动范围：
129: ## 第 3 项：P58 全量 fmt——**排版与 nobid 两项通过**（但 fmt 引出了 P1-1）
138:   - 336b 轮作者另有 log 20261022-033549-336b53db8d，VERDICT 为 ALL_PANIC_LOCATION。
139: - 漏项：fmt 改变了行号，依赖源码行号的“锚式”测试（bsp 白名单）没有同步，见 P1-1。我对其他锚式登记没有做穷举扫描，建议作者在修 P1 时一并全量跑 `--all-targets`，靠测试兜住。
141: ## 第四次合并面 5417eea488（父 dbfb13d90b + main 21b145250e）——**通过（附 P2-1、P2-2、P3-1 至 P3-3）**
150: - 告警门控意图：CI 三档（default、backtest_bin、s_session）的 `-D warnings` 在 336b 上全部 rc=0；json 档也是 rc=0；native 档不通过（P2-2）。
151: - 114 行 cfg_attr/allow：带 reason 的体例与 main 一致，但 28 行无条件 allow 的 reason 不属实或只是推测（P2-1）。
154: ## 第 7 项：独立复跑 @336b53db8d——**除 P1-1 与 P2-2 外通过**
164: | -D warnings native | 同上 | **rc=101**（P2-2） | 20261022-025240 |
169: | rust-test-default | `cargo test --all-targets --no-fail-fast` | **rc=101**：lib 2893/1/153（P1-1），其余 target 全部 ok | 20261022-031741 |
172: | extras #1394 编译负例 | check-compile-negatives-1394.sh | PASS | 20261022-041906 |
181: ## 第 1、4、5、6 项：预审（REPORT/PR 尚未产出，**未下结论**）
189:   - d2b6e25a5d 的表述（CLOSE-DECLARATION.md:73：生产可达，成功路径等价，失败路径改为返回 Err）已经按 0090 P2-1 更正，但行号过期（P3-5）。
190:   - G3 占位采用【合入后填】，没有发现预写的结论。
191:   - 声明里“rust-test-default 预期为绿”与实测不符（P1-1）。
194: - 第 6 项：uo-ci 39654630073 @336b53db8d success（uo s_session 组和 native 组都是 success，S4 PASS_RELEASE）。PR 的 checks 还没有。按 P1-1，PR 上的 `rust-test-default` 预计会红。
196: ## 需要作者处理后复审的事项
198: 1. 修 P1-1，重建 squash，补跑 `cargo test --all-targets` 与 `--lib --features backtest_bin`。
199: 2. 处理 P2-1 和 P2-2：修复，或者由负责人裁定，并在声明中照实写明。
```
