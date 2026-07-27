# oscillation_campaign 生产接线（issue #357，阻塞 #294）

## 0. 票面与口径裁定

- 票面：#357（parent #294/#278），编排者已裁（2026-07-26）：**本仓口径 = A——`Core{level}` 身份账户**（修4「每级一本双向账」的账户轴实现，直接读、零发明）。
- 本票范围：oscillation_campaign（#294 T4，已落地）接入 `fill.rs` 实时 bar 循环——生产实例化最后一公里。全程前台串行完成，未转包外部 agent。
- **关票修复（2026-07-27）**：影子评审（`/tmp/review-357-verdict.md`）判「有条件关」，提出 A/B/C 三条关票条件，均已落地（本文档 §1/§3/§4 已按修复后代码重写，数字全部来自本轮实跑 `/tmp/fix357_dump`，不沿用旧读数）：
  - **A**：本文档旧版的「344 条动作」「9/344≈2.6%」等呈现口径有误导性——已按「尝试数 vs 落地数」重写（见 §3/§4.2），并补上 L0/L1-L3 结构错配、「门关时整段不构造」措辞订正。
  - **B**：`CampaignWiringWitness::resource_exhausted_count` 单桶拆分为 `resource_exhausted_holding_negative_count`（预算耗尽）/`resource_exhausted_free_negative_count`（亏损往返）两子桶，重跑 wf8 实测两者均非零——原「全部预算耗尽」定性**部分撤销**（见 §4.2）。
  - **C**：`drive_campaign_wiring` 本仓取数改分侧口径——只读多头侧余额/成本基（`balance_side`/`cost_basis_side`，`VoiceSide::Long`），空头侧持仓不再被净额吸收误判为空仓；本级空头持仓导致的拒绝单独计入 `unsupported_short_position_count`，不与真空仓的 `no_active_campaign_count` 混计（见 §1/§4.1）。

## 1. 本仓取数与 sizing 落地（符号锚，issue #357 关票条件 C 已修复）

- `ParallelAccountLedger` 新增 `cost_basis(&self, account: AccountIdentity) -> f64`（净额聚合，#357 首版取数入口）与 `cost_basis_side(&self, account: AccountIdentity, side: VoiceSide) -> f64`（分侧聚合，本次关票修复新增，`rust/src/theta_v0/strategy/account.rs`）——后者与既有 `balance_side`（★#237 分侧账）同一过滤谓词（`key.position.side == side`），同规格现算投影、非 canonical 存储。
- `fill.rs` 的 `drive_campaign_wiring`（纯函数，可脱离完整 fill 循环直接单测，同 `step_center_oscillation` 先例）：
  - **本仓取数（关票修复后）**：逐级只读 `Core{level}` 身份的**多头侧**分量——`units = account_view.balance_side(account, VoiceSide::Long)`、`cost_basis = account_view.cost_basis_side(account, VoiceSide::Long)`（均 `as i64` 截断）构造 `CoreCostBasisSnapshot` → `CampaignBook::sync_position`。
  - **修复的问题（原判 FAIL，评审第 1 条）**：首版用 `balance()`（同身份实例有符号净额求和，account.rs:341-347），而 `identity_of` 把 `(FollowParent,Short)` 顺父级联空腿也映到 `Core{level}`——本级持有空头仓位时净额可能非正，被 `sync_position` 的 `units()>0` 判据误判为空仓，campaign 永不开局，空头侧的震荡短差整支被静默丢弃（wf8 实测：`Core{2}` 有腿 bar 中 72% 净空）。改分侧口径后，多头持仓的开局判据不再受空头腿净额污染；空头持仓本身**仍不被本接线支持**（本票范围内不做空头 campaign，属方案性收窄，见下）。
  - **方案取舍（评审二选一，本票选方案 2）**：评审给出两条路径——① `CampaignBook` 改按 (level, side) 分侧各自独立 campaign 生命周期；② 显式声明本接线只支持多头侧，空头侧单独分桶呈现「未支持」。选 ②：`CampaignBook`（#294 既有结构）按 `level: u32` 单键索引，`CenterOscillationActionRecord`（#292 既有结构）本身不携带 side 信息——方案①需要同时改这两处既有结构的键形状（level→(level,side)）+ 上游触发链补齐 side 语义，超出「#357 只接线不重新设计」的票面范围；方案②只需改 `drive_campaign_wiring` 一处取数口径 + `CampaignWiringWitness` 新增一个分桶字段，零改动既有结构键形状，与「接线」范围严格对齐。空头 campaign 支持留给后续票（若需要，见 §7）。
  - **witness 分桶同步**：`apply_action` 在 `NoActiveCampaign` 拒绝时，`drive_campaign_wiring` 现读该 level 的空头侧余额（`balance_side(account, Short) != 0.0`）——非零则计入 `unsupported_short_position_count`（未支持，真有仓）而非 `no_active_campaign_count`（预期经济场景，真空仓），两桶不混计（详见 §4.1）。
  - 报告旧版引用的「`as i64` 截断同 `TwLedgerThread::sync_basis_raw` 既有惯例」——**该引用已删除**（评审判定误导性引用，原文见下）：`sync_basis_raw` 的口径是 `(units.abs()*entry_cost.abs())`（空头取绝对额=在险市值），与本票取有符号净额、负数按空仓处理正好相反——只有「截断」这一半形似，「符号」这一半相反，不构成同一惯例。本票分侧修复后取的是多头侧分量（恒非负），与 `sync_basis_raw` 的绝对值口径同为正数但成因不同（分侧过滤 vs 取绝对值），仍不引用该先例。
- sizing=高抛量=当时持仓 1/3：复用 #294/#348 既有落地代码（`OscillationCampaign::reduce_units`），本票未改动其算法，只接入真实（分侧）取数。
- **取数时点裁定（可复检，不受本次分侧修复影响）**：`account_view` 反映的是本 bar 尚未过账前（前 bar 收盘）的 `Core{level}` 持仓——`drive_campaign_wiring` 与 `step_center_oscillation` 同一调用点执行，早于本 bar 主策略核心仓开/平仓镜像。

## 2. 动作落成事件流（对偶 + 带门）

- `CenterOscillationAction`（Reduce/Replenish）→ `CampaignBook::apply_action` → `OscillationCampaign::apply_action` → `ShortDiffAccount::record_and_apply_dual`（#293/#354 既有实现，未改动）：
  - `ShortDiff`（成本基划转）+ `Realize`（已实现盈亏）两笔对偶事件，经 `closed_loop::transition::cash_sound_gate` + OQ-9 legal 检查带门通道入 TW 账本，成功后镜像进 R=Π-A-W 账本。
  - `CampaignBook::sync_position` 生死驱动：本级多头侧从空仓→持仓开局 campaign（`notional_in=cost_basis_side(Long)`）；多头侧持仓→空仓终结 campaign（挂起随死）。
- 生产接入点：`pi_theta_fill_loop_overlay`（`config.center_oscillation.enabled` 门内，与 `step_center_oscillation` 同一 if 块）。
- **口径订正（issue #357 关票条件 A，评审第 4 条「小瑕」）**：旧版「门关时整段不构造/不驱动，零开销」表述不准——`CampaignBook::new()`/`CampaignWiringWitness::new()` 在门**外**无条件构造（`fill.rs` 生产状态初始化处），门关时是「**构造但不驱动**」：两个空 `BTreeMap` + 若干计数器，开销可忽略但确实存在构造调用，非「整段不构造」。本节措辞已按此订正。

## 3. enabled=true wf8 读数（issue #357 关票条件 A/B/C 修复后重跑，2026-07-27）

命令（沿用既有 env-gate，dump 目录换新以区分修复前后）：

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 OPSEM_DUMP_DIR=/tmp/fix357_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

真实 BTC wf8 窗（2023-08-17..2024-02-16，264960 bar）产物级见证（修复后实测）：

| 读数 | 值 |
|---|---|
| trigger_attempts | 1437 |
| dropped_center_not_alive（CenterNotAlive 丢弃率） | 113（7.9%） |
| dropped_other_trigger（FlatSignal/PriceOutsideZone） | 768 |
| action_by_level（归属分桶，**尝试数**，非落地数） | L1: reduce=198/replenish=35；L2: reduce=93/replenish=8；L3: reduce=8/replenish=2（合计 **344**） |
| suspension_by_source（挂起归宿） | broken_by_third_class_buy=3, rebase_vanished=7, reset=2, superseded=67 |
| lifecycle_opened / lifecycle_died（campaign 生死事件） | 224 / 223（窗末 1 个仍存活，`campaign_active_end=1`） |
| no_active_campaign_count（真空仓，预期经济场景） | 303 |
| unsupported_short_position_count（本级持空仓，未支持，issue #357 关票条件 C 新增分桶） | 20 |
| resource_exhausted_holding_negative_count（预算耗尽，issue #357 关票条件 B 新增子桶） | 7 |
| resource_exhausted_free_negative_count（亏损往返，issue #357 关票条件 B 新增子桶） | 2 |
| other_violation_count（真正的接线/记账错误警报，恒 0） | 0 |
| **真正落地的动作数**（344 − 303 − 20 − 7 − 2，派生） | **12（3.5%）** |
| **够到活 campaign 的尝试数**（344 − 303 − 20，派生） | **21**（其中 12 落地 + 9 因资源耗尽被拒 = **43% 拒绝率**） |

四层报告不受影响（execR=+4040483，MaxDD=0.0917，R=+4417092，LCB(R)=−1921382，INCONCLUSIVE——与 enabled=false 完全一致，campaign 是自包含旁路账本，不改主决策/主账本任何数值；见 §5）。

`lifecycle_opened=224`/`lifecycle_died=223` 与修复前（净额口径）完全相同——本窗数据无「同级多空并存」（`unsupported_short_position_count=20` 是「本级持有空头仓位而多头侧真空仓」的动作尝试计数，不改变 campaign 生死转移本身：净额口径下「本级净空」的 bar 在分侧口径下多头侧读数同为 0，两种口径在「是否空仓」这一问上恰好给出相同答案，只在触发拒绝的归因分桶上产生分歧）。故影子评审独立复算的「175/224 个 campaign 在 level 0」这一结构性发现（campaign 生死轴覆盖全部级别含 L0，动作轴只覆盖 L1-L3——L0 无次级别、`step_center_oscillation` 结构上产不出 L0 动作）在修复后**仍然成立**，不受本票分侧修复影响，本次不重新复算，原样沿用作为 §4.1 的证据。

## 4. Post-mortem（issue #357 关票条件 A/B 修复后重写，声明=能力，#294 验收项④⑤）

### 4.1 `NoActiveCampaign` 类拒绝（303 + 20 = 323，与修复前总数一致）

`CenterOscillationBook`（#292 T2）的减/补决策「无门」——独立于本级 Core 持仓状态。真实次级别买卖点信号频繁在本级空仓时也触发结构信号（结构说做、当前无仓可操作）。本次修复后按「真空仓」与「本级实际持有空头仓位（未支持）」拆两桶：

- **`no_active_campaign_count=303`**：本级多头侧真空仓——`CampaignBook::apply_action` 显式拒绝，符合 #294 module 文档「不静默创建幽灵 campaign」的原始设计意图，**预期经济场景**。
- **`unsupported_short_position_count=20`**：本级实际持有空头仓位（`balance_side(Core{level}, Short) != 0`），但本接线只支持多头侧 campaign（§1 方案 2）——这不是「预期经济场景」，是「本接线覆盖缺口」的产物级见证，如实呈现，不与前者混计（修复前旧版把这 20 条混进单一 `no_active_campaign_count=323`，评审判定「真错误被吞进预期桶」，第 5(b) 条 FAIL）。
- **结构错配（issue #357 关票条件 A 补充呈现）**：224 个 campaign 生死事件里约 175 个在 level 0（沿用 §3 末尾说明的独立复算，本票修复不影响该数字），而动作只出现在 L1/L2/L3（level 0 没有次级别，`step_center_oscillation` 结构上产不出 L0 动作）——campaign 生死轴与动作轴大部分根本不重叠，这是低落地率（§4.2）的结构性主因之一。

### 4.2 资源耗尽类拒绝（拆桶后归因，issue #357 关票条件 B 核心修复）

修复前：`resource_exhausted_count=9` 单桶丢弃了 `TransitionError` 内层错误，报告称「wf8 实测坐实：第 4 次 Reduce 打穿冻结 holding」为已判定归因（评审第 7(b) 条判「证据不足以支撑，却写成了已坐实」，本票不能带着这个未判归因关票）。

拆桶重跑后实测（§3 表）：

| 子桶 | 读数 | 含义 |
|---|---|---|
| `resource_exhausted_holding_negative_count` | **7** | `holding<0`：连续 `Reduce` 打穿冻结预算——`OscillationCampaign::reduce_units`/`replenish_units`（#294/#348 既有代码，本票未改动）按**campaign 开局时冻结的总量**算 sizing，`CampaignBook` 按**级别**（非按中枢）聚合，同级多中枢共享同一份 `holding` 预算，连续同向触发超过冻结总量可支撑的轮次即被拒。 |
| `resource_exhausted_free_negative_count` | **2** | `free<0`：**亏损往返**——`Reduce` 使 `free += units·p_sell`，`Replenish` 使 `free -= units·p_buy`，`p_buy > p_sell`（买回价高于卖出价）即 `free` 转负、被 `cash_sound_gate` 拒绝。 |

**归因结论**：9 条中 7 条（78%）确系「预算耗尽」（原归因坐实这一部分），**但 2 条（22%）是「亏损往返」——原报告「全部是预算耗尽的预期经济场景」这一定性在此撤销**。`resource_exhausted_free_negative_count` 非零意味着：**亏钱的短差往返在本账本里根本记不进去**（`record_and_apply_dual` 在 `cash_sound_gate` 拒绝时整笔回滚，§357 代码锚 `short_diff_bucket.rs::record_and_apply_dual`）——这两次触发的短差决策已经被 `CenterOscillationBook` 判定发生（挂起状态已推进），但对应的账本记账被静默回滚到调用前状态，`realized_cash`/TW 账本上完全看不到这两次亏损尝试存在过。这是**系统性上偏**：`ShortDiffBucket::realized_cash` 只可能收录被通道放行（多为盈利或轻亏）的往返，被拒绝的亏损往返不计入任何报告层读数——报告层看到的短差盈亏因此偏乐观。

**处置**：本条不在 #357 范围内修复（#357 只做接线，不改 #294/#348 的经济假设/sizing 算法），**登记为 #294 后续跟进票**——待编排者裁定是否需要（a）为 `Reduce`/`Replenish` 增加「亏损往返仍强制记账（不经 `cash_sound_gate` 拦截）」的旁路，或（b）在 `free<0` 时改为部分回补/降级处理而非整笔回滚。本次重跑读数（holding=7/free=2）已是该后续票的起点证据，无需再重新测量。

### 4.3 口径问题（issue #357 关票条件 A，尝试数 vs 落地数）

`witness.record_action` 在 `apply_action` **之前**无条件累加（`fill.rs::drive_campaign_wiring`）——`action_by_level` 的 344 是**尝试数**，不是落地数。真正落进账本的是 344 − 303（真空仓）− 20（未支持空头）− 7（预算耗尽）− 2（亏损往返）= **12 条（3.5%）**。

「够到活 campaign」的尝试数 = 344 − 303 − 20 = **21** 条（campaign 已开局、真正提交给 `OscillationCampaign::apply_action` 的调用），其中 12 条落地、9 条（7+2）因资源耗尽被拒 ⟹ **43% 拒绝率**（旧版用 9/344 呈现为 2.6%，分母口径错误，量级差一个数量级；本次已按正确分母 21 呈现）。

## 5. 双轨对照（默认关逐字节一致，本次修复后复跑亲验）

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/fix357_default_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

- `center_oscillation.enabled=false`（未设 `THETA_CENTER_OSCILLATION`）：`campaign_witness` 全零（`trigger_attempts=0`、`no_active_campaign_count=0`、`unsupported_short_position_count=0`、`resource_exhausted_holding_negative_count=0`、`resource_exhausted_free_negative_count=0` 等），四层报告数值与开启臂完全一致（execR=+4040483 等，逐位相同）。
- 双锚 `cmp`（本次修复后重新复跑，逐字节比对）：

```
cmp /tmp/fix357_default_dump/trades.jsonl        /tmp/vocab_align_dump/trades.jsonl        → IDENTICAL（948340B）
cmp /tmp/fix357_default_dump/tower_events.jsonl  /tmp/vocab_align_dump/tower_events.jsonl   → IDENTICAL（354946B）
cmp /tmp/fix357_dump/trades.jsonl                /tmp/fix357_default_dump/trades.jsonl       → IDENTICAL
cmp /tmp/fix357_dump/tower_events.jsonl          /tmp/fix357_default_dump/tower_events.jsonl  → IDENTICAL
```

四份文件两两逐字节相同 ⟹ 「campaign 是自包含旁路账本、不改主决策/主账本」在本次 B/C 修复后依然成立（修复只改 witness 分桶与内部取数口径，不改变任何落入 `trades.jsonl`/`tower_events.jsonl` 的生产决策）。

## 6. 测试与回归

- 新增/改动单测（issue #357 关票修复，2026-07-27）：
  - `account.rs::cost_basis_side_splits_by_voice_side_like_balance_side`（关票条件 C：分侧成本基取数正确性，多空并存不互相污染）。
  - `oscillation_campaign.rs::witness_splits_no_active_campaign_by_short_side_holding`（关票条件 C：`NoActiveCampaign` 按空头侧持仓拆桶）。
  - `oscillation_campaign.rs::witness_splits_resource_exhausted_by_holding_vs_free_negative`（关票条件 B：`ChannelRejected(CashUnsound)` 按 `holding`/`free` 负值拆两子桶，此前零覆盖，评审第 6 条缺口 2 已补齐）。
  - `fill.rs::drive_campaign_wiring_tallies_short_only_core_as_unsupported_not_no_active_campaign`（关票条件 C：本级仅持空头仓位时不开局、拒绝落未支持桶而非真空仓桶，评审第 6 条缺口 3 已补齐）。
  - `fill.rs::drive_campaign_wiring_opens_campaign_from_long_side_only_when_mixed_with_short`（关票条件 C：多空并存时 campaign 只按多头侧取数开局，不被空头侧净额/成本基污染）。
  - `oscillation_campaign.rs::witness_tallies_action_by_level_and_suspension_source_and_lifecycle`（既有测试，随 `record_violation` 签名变化同步更新调用点，断言逻辑不变）。
- 既有单测（#357 首版）：`account.rs::cost_basis_aggregates_per_identity_like_balance`、`oscillation_campaign.rs::witness_tallies_*`（2 条）、`fill.rs::campaign_wiring_tests` 模块（4 条）——本次修复未改动其断言内容，随 API 变化（`record_violation` 新增 `is_short_side_held` 参数）同步调整调用点。
- `cargo test --lib`：**2004 passed / 0 failed / 141 ignored**（全绿；较修复前基线 1999 passed 净增 5 条本次新增单测；`141 ignored` 与修复前一致；工作区仅含本票改动文件 + 既有脏文件 `p107_level_calib.rs` 未触碰）。
- 四把慢锁（`--ignored`，BTC 数据，本次逐一复跑验证零回归）：**4/4 绿**（`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity` / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke`），读数与既往一致（未受本票取数口径变化影响——这四把锁走的是主策略 `identity_of`/`ParallelAccountLedger` 既有路径，`balance_side`/`cost_basis_side` 是纯新增只读方法，不改既有 `balance`/`cost_basis` 语义）。
- 双锚：见 §5，逐字节一致（本次修复后重新复跑验证）。
- 慢锁不跑标注：本票未跑 `l3_*` 多窗族、`p1xx` 校准 bin 等重型窗口——留重型窗口。

## 7. 未能判定项

- sizing「冻结总量 vs 当前剩余量」、campaign 粒度「按级别 vs 按中枢」两处经济假设的对错——#357 范围内只接线不重新设计（既有 #294/#348 裁定），如需修正需编排者另立票。
- **`resource_exhausted_free_negative_count`=2（亏损往返，本次关票修复新发现）的处置方案**——是否需要让亏损短差往返强制记账、还是接受当前「被拒的往返不留痕迹」的现状——超出 #357 接线范围，已登记为 #294 后续跟进票起点（见 §4.2）。
- **空头 campaign 支持**（`unsupported_short_position_count=20`，本票选方案 2 明确搁置）——若后续需要支持空头侧短差 campaign，需改 `CampaignBook` 键形状为 `(level, side)` 并给 `CenterOscillationActionRecord` 补齐 side 语义，属架构级改动，非本票「接线」范围。
- `resource_exhausted_holding_negative_count`/`no_active_campaign_count`/`unsupported_short_position_count` 的真实发生率是否在其他窗口（非 wf8）同样稳定——本票只验收 wf8 单窗，多窗读数未跑（慢锁纪律）。

## 附：改动文件清单（issue #357 关票修复，2026-07-27）

- `rust/src/theta_v0/strategy/account.rs`（+`cost_basis_side` 分侧聚合读数 + 1 单测，关票条件 C）
- `rust/src/theta_v0/strategy/oscillation_campaign.rs`（`CampaignWiringWitness` 新增 `unsupported_short_position_count` 字段 + `resource_exhausted_count` 拆分为 `resource_exhausted_holding_negative_count`/`resource_exhausted_free_negative_count` 两字段；`record_violation` 签名新增 `is_short_side_held` 参数；+3 单测，关票条件 B/C）
- `rust/src/theta_v0/backtest/fill.rs`（`drive_campaign_wiring` 改分侧取数 + 拒绝时现读空头侧余额传给 witness；+2 单测，关票条件 C）
- `rust/src/theta_v0/backtest/wverify_run.rs`（`m8_e2e_all_systems_oos` 打印语句 + `other_violation_count` 断言注释同步新字段，关票条件 A/B/C）
- `.chanlun/review-results/oscillation-campaign-wiring-20260727.md`（本文档，全篇按关票条件 A 重写读数口径与归因结论）
