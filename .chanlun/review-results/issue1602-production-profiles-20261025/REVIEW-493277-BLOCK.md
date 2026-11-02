# #1602 / PR #1681 非作者整票独立审查

- 候选：`493277a8cf230274409722d0d222905afdb15fac`（2026-10-25 再核 PR head 相同）
- 基线：`origin/main` = `3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：20 个变更文件、#1602 专项与共同验收、ADR 0041/0044–0046、直接生产调用者、测试与 CI。
- 方法：只读；未在本机运行 cargo/rustc/cargo test。复核原实现 reviewer 的 nc-remote 记录、当前 GitHub CI 与 main 同组失败原日志。

## 结论

**FAIL / BLOCK，当前不应合入或关闭 #1602。**

记录 schema、四类字段解析、默认等值、输出版本戳与多数缺值负控已经落地，但**正式 S 生产写者仍使用 `admissible=false` 的诊断夹具配置**。这直接违反“生产路径只读当前生效场所记录、诊断夹具不参与准入”的核心合同。CI 绿不能覆盖这个调用面。

## 概念层质询结果

**发现合同矛盾。** ADR 0041 要求可调字段事前从版本化记录单选，主值失败不得换备用值；#1602 进一步要求生产、回放、准入不使用诊断夹具。候选的 `ThetaConfig::default()` 已明确变成诊断夹具，但两个正式 S 生成入口没有同步迁移，仍继续执行生产解析和生成，也不检查 `admissible()`。因此“诊断夹具只在测试/诊断使用”的声明在完整生产链上不成立。

谱系检索未发现需要继承的同构矛盾；本次是候选引入的配置语义变化与未迁移调用者之间的直接冲突。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 1 | **fail** |
| MEDIUM | 2 | **fail** |
| LOW | 0 | note |

结论：**FAIL**。

## Findings

### HIGH-1：正式 S 生产路径仍消费诊断夹具，且没有生产准入检查或记录戳

**位置：**

- `rust/src/bin/s_structure_session.rs:3067-3080`：Begin 已持久后，真实 `cmd_advance` 在 `:3080` 执行 `ThetaConfig::default()` 并继续增量解析。
- `rust/src/bin/s_session_v3/generation.rs:436-458`：正式 `generate_impl` 在 `:458` 同样执行 `ThetaConfig::default()` 并继续生成。
- `rust/src/theta_v0/config.rs:703-716`：诊断构造使用 `venue-profile-diagnostic-fixture`；该 profile 在 `run_records.rs` 与 JSON 中均为 `admissible: false`。

候选把默认配置的场所来源改成了诊断夹具，但这两个生产调用者没有改为生产 profile 构造，也没有检查 `config.admissible()`。`admissible()` 的现有调用只在报告/FFI 输出层（`theta_accept`、`theta_backtest`、`theta_replay`、`ffi`），上述 S 路径完全绕过它。`s_structure_session` 甚至在配置构造之前已持久化 Begin，所以不能靠后续失败假装没有生产副作用。

**影响：**

1. C-06/C-17 的场所 tick 与成本没有由正式 S 生产链从当前生效生产 profile 读取；诊断值仍可进入真实结构生产。
2. “诊断夹具不参与准入”只是部分 CLI 的打印声明，不是全生产链的不变式。
3. 这些 S 输出也没有本票要求的 venue/risk/data 记录 id/version 戳。

**必须返修：**迁移正式 S 配置注入与生产准入，显式分离测试夹具和生产 profile；补一条真实 S 入口负控，证明诊断配置不能用于生产，缺生产 profile 会在合同规定的阶段 fail-closed，且输出记录版本可追溯。不能只改注释或在现有 CLI 上再加一个标签。

### MEDIUM-1：`theta_accept` 可把 profile 缺失败排除在总判定之外并返回 SUCCESS

**位置：`rust/src/bin/theta_accept.rs:443-469,509-510`。**

`run_symbol` 的 `AwaitingPolicy` 被放进 `failures`，随后打印为“失败/空窗（不参与判定）”；最终退出码只检查 `ok_reports`。只要有一个其它品种成功，缺失 profile 的品种不会使验收失败。P0 将来签齐后，其它未签品种可以被跳过并得到整体 SUCCESS，这与“生产与验收回放缺值即停”的验收口径不符。

应为 `AwaitingPolicy`/配置错误建立独立失败类别并使其决定整体失败；只有真正的空窗才能按原规则处理。补多品种混合“一个成功、一个 profile 缺失”的退出码负控。

### MEDIUM-2：生产 profile 头部损坏被误报成 RecordMissing，而不是 InvalidPolicy

**位置：`rust/src/theta_v0/run_records.rs:341-351`。**

`production_venue_where` 用 `if let Ok(...) = venue_header(...)` 读取生产记录头；缺 `profile_id`、缺 `admissible` 或类型错误时被静默忽略，最终统一返回 `no_venue_profile()`。这把损坏/非法生产 profile 错分类为“未提供记录”，丢失 `InvalidPolicy` 语义，也让调用方无法区分配置损坏与未签。

应保留并传播匹配记录的解析错误，不得把 malformed header 当成不存在。补头部损坏的负控。

## 已通过的部分

- `run_records.rs` 对四类记录的字段形状、有限数、布尔、定长数组和缺值处理整体完整；缺记录、缺字段、非法值均有测试。
- `TickConfig`、`ExecConfig`、`RiskConfig`、`VoiceConfig`、`DataConfig` 的 v1 值与改前常数逐项等值；`config_reads_every_field_from_records` 覆盖新版本变异。
- 诊断 profile 明示 `admissible=false`，`theta_backtest`、`theta_replay`、`theta_accept` 和 PyO3 出口有版本戳及诊断标注；P0 未签的 tick/成本会返回 `AwaitingPolicy{venue_profile, cost_inputs}`。
- 坏 tick 闸已实际消费 `config.data.bad_tick_max_dev`，阈值变异测试能区分 0.80 与 0.25；#922 扫描与 #1013 出处已写入记录。
- `uncalibrated_addon` 仍由 `slippage_bps` 派生，没有新增第二常数源。
- 原 reviewer 的 nc-remote 记录：`theta_v0` 2349/0、`cargo check --all-targets --features backtest_bin` 通过；本审未重复运行。

## CI 与剩余门

PR CI run `40075040580`：除 `rust-test-s-session-extras` 外均成功；DevSkim `40075040582` 成功。extras job `120816473494` 的三项失败为：

- `each_signed_record_window_gates_alone`
- `opening_batch_negative_controls_are_typed_and_deterministic`
- `v2_domain_binds_revision_waits_in_e_and_replays`

main 对应 job `120593516825` 的原日志有同名同断言失败，因此这三项属既有 #1621 基线，不是本候选新增回归。但票面要求 CI 全绿，故完整 CI 仍记 **未满足**，不能写成全绿。

## 返修后复审范围

除 HIGH-1 的生产迁移外，须对新 head 重跑正式 S 负控、混合品种 `theta_accept` 退出码、malformed header 分类，以及既有 Node/Rust/Python 相关检查。最终评审必须覆盖实际生产调用面，而非只审配置模块与新增测试。
