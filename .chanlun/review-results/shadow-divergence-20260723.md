# #196 阶段 A：shadow 双链比对分歧报告（零行为变更）

日期：2026-07-23。票据：GitHub issue #196（parent spec #193；map #174）。
依据：`chanlun/plans/spec-accounting-layer-alignment-20260723.md` ID-2 WP-3 阶段 A + Testing Decisions；
`.chanlun/review-results/p1p8-wiring-cost-memo-20260723.md` §1/§3b/§3c/§5。

## 1. 机制（seam = coverage.rs:5068 同构断言思路的全量化）

生产链 `run_theta_v0_pi` 主循环每 bar 在组合层裁决点（`pi_theta_step_traced` 调用，runner.rs）
**之后**并行跑 channel 适配层：channel 每 slot 出一枚裁决，与该 slot 的生产事实
（StepTrace 各桶 + next_active）逐声部比对——仅记录分歧，**不改裁决与订单流**。

- seam 兑现：coverage.rs:5068（5118–5166 双链交叉断言）是仓内既有唯一双链同构见证，
  单点断言；本 shadow = 同一断言思路在**生产每 bar 每声部**的全量化（复用不新建，
  spec Testing Decisions 字面口径）。
- 隔离边界选择：runner 侧 sidecar 簿（`ShadowVoiceBook`）——**未选** StepTrace 增字段方案，
  因为 shadow 状态需跨 bar 持久（P7 检测器/步计数），StepTrace 是每 bar 快照；
  runner 侧方案下 `pi_theta_step_traced` 签名/本体一字不动，其 10 枚自测与
  bit-exact 锁结构性不翻。StepTrace 零改动。
- channel 只出裁决不建腿：声部腿槽每 bar 由生产活动集镜像覆写；`advance` 不进生产路径
  （检测器推进复用其 Hold 转移单源 = 新增 `channel::shadow_observe` 一行委托）。

## 2. 输入适配层（新建适配代码，非新建解释器）

- **活动集 → 每 slot 一声部**：slot = `(level, σ)`（interp 规则3/4 的 slot 唯一性语义；
  VoiceState 单腿 slot 恰好覆盖——同级异向两腿 = 两个独立 slot 各自裁决，备忘 §1d 的
  「同级双向」场景由此自然承载）。持仓边界（腿 id 切换/空仓↔持仓）⟹ P7 检测器与
  步计数重置（「本级持仓期间」字面边界）；slot 消失（无腿无候选）⟹ 从簿清除。
- **candidates 按声部分发 clone**：全量 clone（channel 谓词自带级别过滤：`find_reverse`/
  `find_open`/`observe_sub_cycle` 均按 level 匹配），不新建切分逻辑。
- **parent_projections 生产构造**：runner 侧用现成单源
  `interp::parent_certificate_projection`（interp.rs:180–226）对 χ 后候选集
  （`step_gamma_trade`，与生产喂 interpret 同一集）逐候选构造；适配层按
  `parent_id == 腿.id` 逐声部过滤分发（`ParentCertificateProjection` 新增只读 getter
  `parent_id()`，token 私有字段/构造/核验路径不动）。
- **parent_kappa 恒 `ParentKappa::Unknown`**：P7 记录语境降级（用户裁定 2026-07-23
  接受为 v1 状态，计入 fog；缺口 G7 在册）。

## 3. 降级声明（090：声明=能力）

1. `VoiceState.short_diff` 槽恒 `None`：生产短差由 TW/PanDiv 链独立承担（channel 8 槽
   无 TW 通道，备忘 §2）——P4 恒不触发；P5（OpenShortDiff）/P7（Record）裁决无生产
   对应语义，入 `ChannelOnly` 类（预期内分歧，非缺陷）。
2. entry_v=ShortDiff 的腿作为独立 `(level,σ)` slot 参与比对；channel 对其反向证书
   不置 P2/P3（`reverse_exit_type → CloseShortDiff` 防御分支），裁 Hold vs 生产
   关闭 → 将记为 `ChannelHoldProductionExit`（已知缺口，channel 8 槽无 TW/短差生产语义）。
3. shadow 状态是「生产轨迹的观测者」：同 id 腿延续 ⟹ 检测器跨 bar 连续；channel 裁
   出场而生产未关的分歧 bar 之后，声部继续跟随生产腿（不跟 shadow 裁决走）——
   零行为变更下唯一自洽语义。
4. 分歧落盘 env 门控（`THETA_V0_SHADOW_DIVERGENCE_PATH`，测试经线程局部 override，
   opsem 2026-07-13 竞态同款规避）；默认 None ⟹ 零 IO，bit-exact 原路径。

## 4. 分歧分类法（裁决 × 生产事实交叉表，全定义 match）

| kind | 语义 | 性质 |
|---|---|---|
| Match | 裁决与事实一致（含 P1 对空仓 slot 无操作同效） | 只计数 |
| ExitTypedMismatch | 同是出场但 typed 不一致 | 正常数据不应出现（单源 reverse_exit_type） |
| ChannelExitProductionHold | channel 裁出场，生产腿延续 | TW 屏蔽/fold 结构差，预期 |
| ChannelHoldProductionExit | channel 裁 Hold，生产腿离场 | ShortDiff/TW 已知缺口，预期 |
| ChannelOpenProductionIdle | channel 裁开仓，生产未开 | AncOK 剪 / 候选被 fold 规则2 消费为关闭触发，预期 |
| ChannelHoldProductionOpen | channel 裁 Hold，生产开新腿 | ShortDiff 角色候选开仓（channel P6 排除、interp 规则3 不排除），预期 |
| ChannelOnly | OpenShortDiff/Record/AddPosition | 生产无对应语义，票面明知 |
| ProductionSilentDrop | 生产 §13 AncOK/Stale 静默剪 | channel 无结构剪枝语义，预期缺口 |

## 5. 运行见证

### 5a. 生产路径真实执行（runner 级见证测试）

`theta_v0::backtest::runner::tests::shadow_dual_chain_witness_in_pi_loop`（新增，本票）：
buy1@3 确认 → 开 Long 持仓多 bar 的 fill loop 内，shadow 每 bar 真实跑 channel 裁决并
经线程局部 override 落盘；断言报告含汇总头、voice_steps>0（非空转）、单腿单候选
无反向场景全 Match（divergences=0）。**绿**。

### 5b. 真实分歧场景（typed_ledger_reverse_close_root 场景，进程 env 落盘实测）

场景：buy1@3 开 Long（bar7）→ sell@12 一类反向（bar14）生产 CloseRoot 关腿。
实测命令：

```text
THETA_V0_SHADOW_DIVERGENCE_PATH=... cargo test --lib typed_ledger_reverse_close_root
```

落盘原文（2026-07-23 实测）：

```text
# shadow 双链比对分歧报告（#196 阶段 A，零行为变更）
voice_steps=9 matches=8 divergences=1
  exit_typed_mismatch=0
  channel_exit_production_hold=0
  channel_hold_production_exit=0
  channel_open_production_idle=1
  channel_hold_production_open=0
  channel_only=0
  production_silent_drop=0
## 逐条分歧（bar, slot, leg, channel, decision, production, kind）
bar=14 slot=(L0,Short) leg=None channel=Cj(6) decision=Open production=Idle kind=ChannelOpenProductionIdle
```

读法：
- **同构关闭的正面见证**：bar14 持仓 slot (L0,Long) 的 channel 裁决
  `Exit(CloseRoot)` 与生产 closed 携 typed `CloseRoot` 一致（计入 matches=8）——
  coverage.rs:5068 断言思路在生产数据上的全量化兑现。
- **唯一分歧是预期结构差**：bar14 空仓 slot (L0,Short) 裁 Open——散装 fold 规则2 已把
  sell@12 消费为关闭触发（不入 open 桶），channel 声部独立互斥语义下该 slot 独立裁
  Open。此非缺陷：备忘 §3「多候选场景两链裁决结构不同」的实例，正是阶段 A 要全量化
  记录的素材。

### 5c. 订单轨 bit-exact 不变（硬门槛，四把锁逐项）

| 锁 | 结果 |
|---|---|
| `center_oscillation_default_inactive_order_track_bitexact`（订单轨默认非活跃） | 绿 |
| `m6_cost_model_none_bit_exact_and_conserves`（成本模型 none） | 绿 |
| `opsem_dump_env_gated_bit_exact`（opsem dump env 门控） | 绿 |
| `pan_div_dc_e_default_inactive_order_track_bitexact` | 绿 |

基线行为对照（同输入两臂）：上述锁本身即两臂对照（enabled vs disabled / set vs unset
逐位一致），且全量 `cargo test --lib` = 1786 passed / 0 failed，与 main 基线
（1779 passed / 0 failed，同工作区既有未提交改动下实测）之差恰为本票新增 7 枚测试
（shadow.rs 6 + runner 见证 1），**零既有测试翻动**。

## 6. 验收对照

- [x] shadow 在 run_theta_v0_pi 生产路径运行：§5a（见证测试）+ §5b（真实场景落盘）。
- [x] 订单轨 bit-exact 不变：§5c 四把锁全绿 + 全量对照零翻动。
- [x] coverage.rs:5068 同构断言思路全量化为分歧记录；分歧报告落盘：本文件 + §5b 实测。
- [x] channel 自测 22 枚继续通过（票面写 23 枚——实际计数 22，以 `cargo test --lib
  theta_v0::strategy::channel::` 运行为准，22 passed / 0 failed）；interp 自测 33 枚
  不翻（备忘 §3b 口径 10 枚包含在内；`interpret`/`interpret_with_close_triggers`
  等函数零删除零签名改动，仅 `ParentCertificateProjection` 新增只读 getter）。

## 7. 遗留 / 未验证项（如实标注）

1. 真实分歧场景目前覆盖两类（全 Match 场景 + 1 条 ChannelOpenProductionIdle）；
   ChannelOnly（P5/P7 真实触发）、ChannelHoldProductionOpen（ShortDiff 角色开仓）、
   ProductionSilentDrop 等类在生产数据上的实例待阶段 B/C 或更长基线数据丰富——
   其判定逻辑由 `classify` 全分支对照表单测 + 语义分析背书，**未在生产数据实测**。
2. #195 短差场景（pan_div enabled 臂）的 shadow 记录未单独采证（该测试两臂顺序
   enabled→control，进程 env 落盘被后跑臂覆盖；线程局部 override 留给后续按需采样）。
3. 性能：每 bar 双跑 + 每 slot candidates 全量 clone（memo §4 已批准此成本量级）；
   未做基准量化（calib 内层高频调用场景的成本实测留待阶段 B 前评估）。
4. parent_kappa=Unknown 下 P7 记录桶的 κ 端点恒 Unknown（fog 在册 G7）——
   中枢语境提取器不在本票范围。
