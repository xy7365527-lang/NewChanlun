# oscillation_campaign 生产接线（issue #357，阻塞 #294）

## 0. 票面与口径裁定

- 票面：#357（parent #294/#278），编排者已裁（2026-07-26）：**本仓口径 = A——`Core{level}` 身份账户**（修4「每级一本双向账」的账户轴实现，直接读、零发明）。
- 本票范围：oscillation_campaign（#294 T4，已落地）接入 `fill.rs` 实时 bar 循环——生产实例化最后一公里。全程前台串行完成，未转包外部 agent。

## 1. 本仓取数与 sizing 落地（符号锚）

- `ParallelAccountLedger` 新增 `cost_basis(&self, account: AccountIdentity) -> f64`（`rust/src/theta_v0/strategy/account.rs`），与既有 `balance()` 同规格的派生聚合读数——同身份多仓位节点分实例成本基求和，非 canonical 存储。
- `fill.rs` 新增 `drive_campaign_wiring`（纯函数，可脱离完整 fill 循环直接单测，同 `step_center_oscillation` 先例）：
  - 逐级按 `Core{level}` 身份取 `units = account_view.balance(...)`、`cost_basis = account_view.cost_basis(...)`（均 `as i64` 截断，同 `TwLedgerThread::sync_basis_raw` 既有惯例）构造 `CoreCostBasisSnapshot` → `CampaignBook::sync_position`。
  - sizing=高抛量=当时持仓 1/3：复用 #294/#348 既有落地代码（`OscillationCampaign::reduce_units`），本票未改动其算法，只接入真实取数。
- **取数时点裁定（可复检）**：`account_view` 反映的是本 bar 尚未过账前（前 bar 收盘）的 `Core{level}` 持仓——`drive_campaign_wiring` 与 `step_center_oscillation` 同一调用点执行，早于本 bar 主策略核心仓开/平仓镜像。理由：次级别信号驱动的震荡短差决策与本 bar 内主策略自身仓位变化是同 bar 内正交的两条独立决策源，不宜以未定义的执行顺序静默耦合。

## 2. 动作落成事件流（对偶 + 带门）

- `CenterOscillationAction`（Reduce/Replenish）→ `CampaignBook::apply_action` → `ShortDiffAccount::record_and_apply_dual`（#293/#354 既有实现，未改动）：
  - `ShortDiff`（成本基划转）+ `Realize`（已实现盈亏）两笔对偶事件，经 `closed_loop::transition::cash_sound_gate` + OQ-9 legal 检查带门通道入 TW 账本，成功后镜像进 R=Π-A-W 账本。
  - `CampaignBook::sync_position` 生死驱动：Core{level} 从空仓→持仓开局 campaign（`notional_in=cost_basis()`）；持仓→空仓终结 campaign（挂起随死）。
- 生产接入点：`pi_theta_fill_loop_overlay`（`config.center_oscillation.enabled` 门内，与 `step_center_oscillation` 同一 if 块），门关时整段不构造/不驱动，零开销。

## 3. enabled=true wf8 读数（减补/归属/丢弃率/归宿）

命令（新增 env `THETA_CENTER_OSCILLATION=1` 覆盖 `config.center_oscillation.enabled`，仿 `THETA_DIR_PRESET`/`ENFORCE_GROSS_CAP` 既有 env-gate 先例）：

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 OPSEM_DUMP_DIR=/tmp/wire357_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

真实 BTC wf8 窗（2023-08-17..2024-02-16，264960 bar）产物级见证：

| 读数 | 值 |
|---|---|
| trigger_attempts | 1437 |
| dropped_center_not_alive（CenterNotAlive 丢弃率） | 113（7.9%） |
| dropped_other_trigger（FlatSignal/PriceOutsideZone） | 768 |
| action_by_level（归属分桶） | L1: reduce=198/replenish=35；L2: reduce=93/replenish=8；L3: reduce=8/replenish=2 |
| suspension_by_source（挂起归宿） | broken_by_third_class_buy=3, rebase_vanished=7, reset=2, superseded=67 |
| lifecycle_opened / lifecycle_died（campaign 生死事件） | 224 / 223（窗末 1 个仍存活） |
| no_active_campaign_count | 323（预期，见 §4） |
| resource_exhausted_count | 9（预期，见 §4） |
| other_violation_count | 0（真正的接线/记账错误警报，恒 0） |

四层报告不受影响（execR=+4040483，MaxDD=0.0917，R=+4417092，LCB(R)=−1921382，INCONCLUSIVE——与 enabled=false 完全一致，campaign 是自包含旁路账本，不改主决策/主账本任何数值）。

## 4. Post-mortem（声明=能力，#294 验收项④⑤）

真实数据首次暴露两类此前仅在「干净单轮round-trip」单测覆盖下未曾触达的边界，二者均由系统既有的「违规显式失败」纪律正确拦截（非静默钳制/吸收/panic），**非本票引入的接线缺陷**：

1. **`NoActiveCampaign`（323 次）**：`CenterOscillationBook`（#292 T2）的减/补决策「无门」——独立于本级 Core 持仓状态。真实次级别买卖点信号频繁在本级空仓时也触发结构信号（结构说做、当前无仓可操作），`CampaignBook::apply_action` 对空仓级别显式拒绝，符合 #294 module 文档「不静默创建幽灵 campaign」的原始设计意图。
2. **`ShortDiff(ChannelRejected)`（9 次，资源耗尽）**：`OscillationCampaign::reduce_units`/`replenish_units`（#294/#348 既有落地代码，本票未改动）按 **campaign 开局时冻结的总量** 计算 sizing（非「当前剩余可减仓量」），而 `CampaignBook` 按**级别**（非按中枢）聚合——同级多个中枢各自独立触发 `Reduce` 会共享同一份 `holding` 预算。当连续同向触发次数超过冻结总量所能支撑的轮次，`cash_sound_gate` 显式拒绝（`holding` 将变负）。这是「每仓 campaign 粒度=按级别聚合、不按中枢分」（#294 决议2 既有设计）在多中枢真实生产数据下的自然结果，**不是本票新引入的 bug**——真实数据首次触达此前只被单中枢单轮次单测覆盖的边界。
3. 处置：`CampaignWiringWitness` 三分桶（`no_active_campaign_count` / `resource_exhausted_count` / `other_violation_count`）显式区分「预期经济场景」与「真正的接线/记账逻辑错误」——后者本次实测恒 0，证明本票的取数/接线代码本身无缺陷；前两者作为产物级见证如实呈现，不断言归零。
4. 该发现建议记入 #294 后续跟进（如需把 sizing 从「冻结总量」改为「当前剩余可减仓量」，或把 campaign 粒度收窄到按中枢——均超出 #357 接线范围，留给编排者裁定）。

## 5. 双轨对照（默认关逐字节一致）

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/wire357_default_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

- `center_oscillation.enabled=false`（未设 `THETA_CENTER_OSCILLATION`）：`campaign_witness` 全零（trigger_attempts=0 等），四层报告数值与开启臂完全一致。
- `trades.jsonl`/`tower_events.jsonl` 与既有金标准 `/tmp/vocab_align_dump`（948340B / 354946B）逐字节 `cmp` 无差异——**IDENTICAL**。

## 6. 测试与回归

- 新增单测：
  - `account.rs::cost_basis_aggregates_per_identity_like_balance`（本仓取数聚合正确性）
  - `oscillation_campaign.rs::witness_tallies_*`（2 条，witness 分桶正确性）
  - `fill.rs::campaign_wiring_tests` 模块（4 条：本仓取数开局/sizing 1/3 落地+归属分桶/真实事件流生死/NoActiveCampaign 分桶）
- `cargo test --lib`：**1999 passed / 0 failed / 141 ignored**（全绿，工作区仅含本票 5 个改动文件 + 既有脏文件 `p107_level_calib.rs` 未触碰）。
- 四把锁（`--ignored`，BTC 数据）：**4/4 绿，读数与既往一致**（`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity` / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke`）。
- 双锚：见 §5，逐字节一致。
- 慢锁不跑标注：本票未跑 `l3_*` 多窗族、`p1xx` 校准 bin 等重型窗口——留重型窗口。

## 7. 未能判定项

- sizing「冻结总量 vs 当前剩余量」、campaign 粒度「按级别 vs 按中枢」两处经济假设的对错——#357 范围内只接线不重新设计（既有 #294/#348 裁定），如需修正需编排者另立票。
- `resource_exhausted_count`/`no_active_campaign_count` 的真实发生率是否在其他窗口（非 wf8）同样温和——本票只验收 wf8 单窗，多窗读数未跑（慢锁纪律）。

## 附：改动文件清单

- `rust/src/theta_v0/strategy/account.rs`（+`cost_basis` 聚合读数 + 1 单测）
- `rust/src/theta_v0/strategy/oscillation_campaign.rs`（+`CampaignWiringWitness` 观测统计 + 2 单测）
- `rust/src/theta_v0/backtest/fill.rs`（`step_center_oscillation` 接入 witness；新增 `drive_campaign_wiring` + 4 单测；`FillOutput`/三处构造点补齐 `campaign_book`/`campaign_witness` 字段）
- `rust/src/theta_v0/backtest/runner.rs`（`OverlayRunResult` 转发 `campaign_book`/`campaign_witness`，同 `tw_final` 既有先例）
- `rust/src/theta_v0/backtest/wverify_run.rs`（新增 `apply_center_oscillation_from_env`；`m8_e2e_all_systems_oos` 打印+核验 campaign witness）
