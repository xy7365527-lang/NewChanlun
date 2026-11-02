# #1601 / PR #1680 最终非作者独立复审

- 审查候选：`107336f3d4ae0e4b2b9b52d3916d62311685829c`
- 审查基线：`origin/main = 13ed4d45c8ff85175819ca987690328ab2a247a4`
- rebase 前旧 head：`ffcbfca4d4128e0eedbc3e1b4343743f1958db3e`
- PR：<https://github.com/xy7365527-lang/NewChanlun/pull/1680>
- 身份：独立非作者复审，不是实施者。
- 写报告前已再次查询 PR：`headRefOid` 仍等于上述候选，`baseRefOid` 仍等于上述基线，PR 为 OPEN；本地 HEAD 也等于候选，工作区干净。因此本报告对应该精确候选。

## 裁决

| 严重级别 | 数量 |
|---|---:|
| CRITICAL | 0 |
| HIGH | 0 |
| MEDIUM | 0 |
| LOW | 0 |

- **代码 / 合同：PASS。** 完整候选差异中未发现新的阻断或普通缺陷；旧 head 的 HIGH-1（三个 NT 回放 venue 固定使用 1,000,000）已闭合。
- **CI：PENDING，不能记为全绿。** 截止本报告状态快照，CI run `40143245113` 仍有 3 个 job 在运行，详见下文。
- **综合合入门：暂 BLOCK，仅因 CI 尚未终结，不是代码 / 合同发现。** CI 全部终结且无新增失败后，代码 / 合同侧没有本审留下的阻断；合入与关票仍由人的批准门决定。

## 范围与方法

- 阅读根 `AGENTS.md`、issue #1601、PR #1680、旧 head 尾部独审、前次非作者独审、NT 修复说明，以及 ADR 0040 / 0041 / 0043；为核生产名分另读 ADR 0039 / 0048 对应条目。
- 审查完整 `base..candidate` 差异：**54 个文件，2335 行新增、483 行删除**，包括 funding-record、回放账本、NT 策略与 venue、起始操作级别、旧线 policy、Python FFI、测试和调用方。
- `git diff --check base..candidate` 通过。
- 本审只读源码、票据和 CI；未改代码、票据或分支，未派孙代理，未在本机运行 cargo / rustc / cargo test。测试结论来自当前精确候选的 CI 与已注明的既有独审证据，不冒充本机复跑。

## rebase 等价性

执行：

```bash
git range-diff \
  3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f..ffcbfca4d4128e0eedbc3e1b4343743f1958db3e \
  13ed4d45c8ff85175819ca987690328ab2a247a4..107336f3d4ae0e4b2b9b52d3916d62311685829c
```

结果三对提交全部为 `=`：

```text
1:  8200ea514f = 1:  cc897fd286  回放/NT/操作级别改读签署记录
2:  5ac78ab2f2 = 2:  99743cbae9  排版复审
3:  ffcbfca4d4 = 3:  107336f3d4  NT 回放 venue 与引擎共用资金记录
```

旧 head 到新 head 的树差只含新 base 经 #1688 带入的 8 个文件；本 PR 三个补丁没有改写。因此没有需要另外解释的冲突适配或补丁级行为变化。

## 决定性证据

### 1. 回放可读取证据记录，生产下单不能越过 scope 门

- `rust/src/funding_record/funding-record.json` 与 ADR 0040 一致：P0 为 100,000 USDC，P1-hl-testnet 为 500 USDC，P0-acceptance-5m 为 5,000,000 USDC 且 `scope=evidence_only`。
- `rust/src/funding_record.rs:278-328` 把两类入口分开：回放可经 `active` 读取生效记录；生产必须经 `admit_production` / `production`，缺记录产生 `AwaitingPolicy{funding_record}`，`evidence_only` 明确报错，不能构造生产策略。
- `rust/src/theta_v0/nautilus/theta_strategy.rs`、`theta_pi_strategy.rs`、`backtest_engine.rs` 都由同一 funding-record 取得资金并走生产准入；venue starting balance 与策略 nav 不再各有一套资金值。
- `rust/src/theta_v0/ffi.rs:70-105` 的 `PyThetaStream` 使用 `FundingRecord::production`；`checked_nav` 只接受正有限数。nav≤0、NaN、Infinity 都直接报错，当前 bar 不推进，不再换成 1e6。
- `trading_system/strategy/theta_rec_strategy.py` 已删除 `initial_nav=1e6` 备用值；账户不可用时返回的非法 nav 会由上述 Rust 门拒绝，而不是被替换成默认资金。

这符合 ADR 0040「验收专用回放只出证据、不进生产、不下单」以及 ADR 0041「主值失败不得启用备用值」。

### 2. 回放总投入、版本戳与 NT venue 已同源

- `rust/src/funding_record.rs:414-480` 的 `ReplayRecords` 一次绑定 funding、derivation-input、arm-sub-expiry 三份记录；`label()` 同时输出三份 id / version。`py_replay_records` 未指定 profile 就报错，不选择默认 profile。
- `rust/src/trading/runner.rs:1518-1554` 与 `rust/src/trading/positional.rs:1603-1640` 的专项测试都实际以 P0 和 P0-acceptance-5m 各跑一次回放，断言记录版本、Q_total 驱动的 50 倍资金缩放，以及与旧常数等值夹具的对拍。当前候选的 `rust-test-default` 已成功。
- `trading_system/backtest_fugue_v3.py:243-255`、`backtest_t_fugue.py:206-218`、`backtest_unn_stream.py:184-196` 都在 `BacktestEngine` 装配前读取 `nr.read_funding_record(FUNDING_PROFILE)`，venue 的 `starting_balances` 使用同一返回值的 `q_total`；策略流也接收同一 `FUNDING_PROFILE`。`backtest_unn.py:230-242` 与 `backtest/theta_backtest_py.py:117-131` 同样没有第二资金来源。
- `tests/test_nt_funding_records.py:148-203` 共 18 项：三个入口 × 两份 profile 的正例、三个缺 profile 负控、三个入口 × RecordMissing / FieldMissing / Invalid 九项负控。缺值或非法值均在 engine 装配前停止。当前候选的 Python `test` job 已成功。

证据边界：这 18 项直接执行生产 `build_engine`，但 mock NT、数据和 Rust 协作者，证明的是 venue 装配合同，不是完整 NT 历史行情回放；本审不把它扩写成后者。票面条款指定的两份回放由上述 Rust 专项测试覆盖。

### 3. 起始操作级别的行为差异被显式保留，没有暗回旧默认

- `rust/src/funding_record/chong-derivation-input.json` 三个 profile 都签为 `l_start=L0`。
- `rust/src/funding_record.rs:331-380` 明确换算 `ladder = FIRST_BSP_LADDER + ℓ`，所以生效 L0 对应 ladder 2；旧 `DEFAULT_OP_LADDER=3` 对应 L1。两者不是等值行为。
- `rust/src/funding_record.rs:457-470` 的等值对拍夹具仅在 `#[cfg(test)]` 下使用 Q=100,000、L1/ladder 3、sub_expiry=60，并仍走同一解析器。
- `rust/src/trading/positional.rs:1643-1657` 锁定生效 L0 在 fusion 下界处显式报错，不允许静默退回 ladder 3；等值夹具仍可越过该 ladder 门。

因此这里既没有把签署 L0 偷换成旧 L1，也没有把必要的行为变化伪装成「全部逐位相同」。

### 4. AwaitingPolicy、非法值与旧线 policy 边界成立

- funding 与 derivation 的缺记录、缺字段、未签和非法值均经 typed error 传播；回放与生产调用点没有 `unwrap_or` 旧资金或旧级别。
- `rust/src/theta_v0/policy_records/arm-sub-expiry.json` 是 #1601 C-22 明文允许的「旧线 policy 字段」形态：值为 60，有版本、出处、选项与代价；source 明说 v1 只迁移现行值、未经新签。它没有被伪装成新的用户签署值。
- `rust/src/trading/types.rs` 的 `INITIAL_CAPITAL`、`DEFAULT_OP_LADDER`、`SUB_EXPIRY` 生产常量已删除；等值数字只剩 `#[cfg(test)]` 夹具。精确残留搜索的其它命中是注释、禁止词扫描或记录出处，不是运行值。
- ADR 0043 的 `voice-open-funding` 未被塞入 funding-record，仍属另一份 policy 记录；本 PR 未改变 funding-record 八字段形状。

### 5. RecTStream 的 evidence_only 读取不构成生产越权

补查发现 `rust/src/recursive_t/ffi.rs` 的 `RecTStream` 使用 `py_funding_record`，所以可读取 evidence_only 记录；但 ADR 0048:267-278 明确把 `PyRecStream` / `RecTStream` 列为**冻结影子**，不持写权、不产交易意图，替代入口是 `production_session`。ADR 0039:216-223 同样规定生产进程不加载该 cdylib。

当前仓内也没有 `rec_t_strategy.py` 或 RecTStream 的生产调用者；命中的是冻结研究脚本及 `theta_rec_strategy.py` 的历史说明。因此不能仅凭 `RecStream::new_production` 这个内部命名，把该冻结对照入口误判为本票的生产下单面。本项不列为 finding。

## CI 状态（与代码 / 合同分开）

当前精确候选的 CI run：<https://github.com/xy7365527-lang/NewChanlun/actions/runs/40143245113>。

已成功：

- `test`
- `rust-check`
- `rust-test-default`
- `rust-test-backtest-bin`
- `rust-test-s-session` shard 2、shard 3
- `s-observer`
- `fixture-drift`
- `sandcastle-ts`
- 独立 workflow `DevSkim`（run `40143245052`）

仍在运行：

- `rust-test-s-session` shard 0
- `rust-test-s-session` shard 1
- `rust-test-s-session-extras`

`macos-session` 为 SKIPPED，不计作成功，也不把它伪报成失败。当前快照没有已终结的失败，但运行中的 job 不能预先判绿。

## 剩余门与交付边界

1. 等上述 3 个 CI job 终结并读取真实结论；若出现失败，再按当前 head 的原日志归责，不沿用旧 base 的失败结论。
2. 本报告按任务要求写在外部指定路径，未修改仓库或票据。issue 共同验收要求的 `.chanlun/review-results/` 归档与票内链接仍需后续交付步骤完成。
3. 不自动合 main、不自动关票；本审没有执行这些动作。
