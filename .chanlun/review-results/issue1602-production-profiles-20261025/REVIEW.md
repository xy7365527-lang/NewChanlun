# PR #1681 `07e3443c51` 非作者尾部与整票复审

- 候选：`07e3443c5118bbdc3da2c3978f12576a1e5fece1`
- 前序候选：`493277a8cf230274409722d0d222905afdb15fac`
- 基线：`origin/main@3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：前序独审三项阻断、当前 10 个变更文件及直接生产调用面、配置与错误分型、生成版本戳、测试和 CI。
- 方法：只读审查当前完整 diff、ADR 0041/0044/0028、生产与测试调用路径、nc-remote 原始日志及 GitHub CI；未在本机运行 cargo。

## 代码审查报告

### 概念层质询结果

**通过。** ADR 0041 要求自由度事前从版本化记录单选，主值失败不得换备用值；P0 当前 tick_size、slippage_bps、tax_bps、entry_delay_bars 未签，正式 S 必须 fail-closed。实现与该合同一致，没有改变教义、资金、隔离或恢复语义。

前序独审发现的是配置语义与未迁移调用者的合同冲突，不是需要继承的教义矛盾。本次已按同一问题谱系修复：生产配置在开门/pending 恢复/Begin 前构造并核验，诊断配置只经明确测试入口使用。额外检索到的 `ThetaConfig::default()` 均位于测试模块或既有 TestOnly Native 路径，没有遗漏的正式生产 default 旁路。

### 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

**结论：PASS（代码与合同）。** 未发现新的 CRITICAL/HIGH 阻断。该结论不等同于 CI 全绿、macOS 覆盖完成或 main 合入许可。

## 前序阻断复核

### HIGH-1：正式 S 使用诊断夹具——已闭合

- `Driver::open_production_with_request_and_mode` 在打开库和 pending 恢复前构造 `ThetaConfig::for_production_profile("P0")`；当前未签字段会在该处直接返回 `AwaitingPolicy{venue_profile,cost_inputs}`。
- `Driver::advance` 在 `begin_advance` 前再核 `require_admissible && !theta_config.admissible()`；`advance_core` 也在 capture/open_db/Begin 前构造并核验配置。
- `prepared::Runtime` 将同一配置交给 `GenerationRuntime::with_config`，`generate_impl` 不再调用默认构造。`Generated.policy_records` 仅作运行元数据，未进入生成态内容字节。
- 正式 `run`/`dispatch`/`advance` 都选择生产构造；旧测试改用明确的 `run_fixture`/`dispatch_fixture`/`advance_core_fixture`/`handle_fixture`/`serve_fixture`。未发现生产调用误走 fixture。
- `advance_core` 的正常和幂等输出，以及 `Driver::state_summary`，都携带 `policy_records`；`Generated` 也带该版本戳。

### MEDIUM-1：缺失 profile 可从验收总判定消失——已闭合

`theta_accept` 用 `SymbolFailure::{Policy,Data}` 分离配置/政策失败与数据失败/空窗。`batch_succeeds` 要求有成功报告、全部通过且无配置失败；一个成功品种与一个缺失 profile 的品种并存仍返回 FAILURE。数据空窗保持原口径。

### MEDIUM-2：损坏生产 profile 头误报 RecordMissing——已闭合

`production_venue_from_records` 直接传播匹配记录的 `venue_header` 错误为 `InvalidPolicy`，不再吞成 `no_venue_profile()`。缺记录与损坏头保持不同错误分型。

## 验证证据

| 范围 | 结果 | 证据 |
|---|---|---|
| 新增 S/theta_accept 负控 | 各 1/1；run_records 8/8 | `/tmp/nc-remote/20261025-033153-841a94391d.log` |
| 真实生成 Oracle + 非 test 编译 | generation 5/5；s_session all-targets PASS | `/tmp/nc-remote/20261025-035704-d9a04f21fc.log` |
| 前次失败集与两种 feature 编译 | v2 23/23、S3 CLI 3/3、恢复 1/1；s_session/backtest_bin all-targets PASS | `/tmp/nc-remote/20261025-042930-07e3443c51.log` |
| 配置/验收/记录测试 | theta_accept 2/2、config 10/10、run_records 8/8 | `/tmp/nc-remote/20261025-043247-07e3443c51.log` |
| 当前 head 完整 S 回归（排除 WSL 既有 macOS-only） | **517 passed / 0 failed / 12 ignored / 29 filtered out** | `/tmp/nc-remote/20261025-043454-07e3443c51.log` |

测试名称过滤实际执行数均非零；未把 0 项当 PASS。远端均使用 windows-5090 WSL 的 nc-remote 独立工位，未在本机运行 cargo。

## CI 与平台边界

GitHub CI run [40130421406](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40130421406) 已终结为 **failure**，但只有 `rust-test-s-session-extras` 的三项 `tb03c_funding_tests` 失败：

- `each_signed_record_window_gates_alone`
- `opening_batch_negative_controls_are_typed_and_deterministic`
- `v2_domain_binds_revision_waits_in_e_and_replays`

main 对应 job [120593516825](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40005878482/job/120593516825) 的原日志有同名、同断言失败（`PolicyNotYetEffective` / `Applied` 与 `Error` / `Applied`）。这三项属 #1621 基线，不是本候选新增回归；其余作业（含四个 shard、coverage、backtest-bin、rust-check、DevSkim、test、s-observer、fixture-drift、sandcastle-ts）均成功。

**NOT_VERIFIED：** `v3::financial_session` 的 26 项 macOS 专属测试未在本候选得到执行验证。它们在本机 WSL 上按产品合同返回 `MissingDependency`，不能算通过；当前 PR 的 `macos-session` 又因 `github.event_name != workflow_dispatch` 而跳过。需通过 macOS 的 `ci-macos.yml` 显式触发或在获准 macOS 环境执行，不能以 517 项 WSL 通过替代。

## 剩余门

1. 显式取得上述 macOS 26 项的执行结果。
2. 跟踪 #1621 的主线 funding 修复；不得把当前 CI 写成全绿，也不得为了通过而删/改这些测试。
3. 最终报告入仓、main 合入人工批准及关票门复核。

本审不 merge、不关票。
