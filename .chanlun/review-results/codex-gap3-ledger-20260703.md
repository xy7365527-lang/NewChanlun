# codex 全权裁定：GAP3 桥「已实现利润/可正可负 MTM 账本重装」（R3 留白项重新提交裁决）

- 工位: ws-codexgap3（Task #139）| 分支: gap3-rework-codex9-fix | 日期: 2026-07-03
- 裁决路径: 延续 R3 授权链——编排者明令「有要裁定的直接问 codex，他全权裁定」，本轮不回蜂群内部
  二次讨论合法性。
- 调用方式: 本机 `codex exec --skip-git-repo-check --sandbox read-only`（codex-cli 0.142.5），与 R3
  同款、同版本，保持裁决路径一致性。
- 认识论等级: L1（文本/代码对照裁决，非数据验证；产出=账本重装的架构合法性判定，不产生 alpha 证据）。

## 1. 争议起点

`.chanlun/review-results/codex-r3-ruling-20260702.md`（R3 终局裁决）判定 `hwm_gain` 高水位棘轮违反
PDF p8③「禁止语义回补」，移除其承重后，裁决原文明确列出两条允许的改造方向并要求「重新提交裁决」：

> (b) 若坚持价格桥路径，改为「已实现利润」入账（而非未实现浮盈峰值）；
> (c) 或做可正可负的当前 MTM 账本，把「权益」和「可退现金」概念分离。

本任务（#139，来源 #124 G5 上浮，`g5-impl-20260703.md` 边界条件 (a)）= 构建证据包并重新提交裁决。

## 2. 证据包摘要（提交 codex 前打包，完整证据包见 `/tmp/codex-gap3-ledger-ctx.md` 内容，逐段摘录如下）

### 2.1 PDF p8①②③ 原文（与 R3 同一 PDF，唯一权威源）

> ① 断裂的句法条件——断裂必须以否定句出现；不能以"完成/形成/结束"类谓词出现；必须指向某个已
> 声明的不变量。断裂 ≠ 新状态；断裂 = 旧条件不再可维持。
> ② 不变量的存在依赖——不变量不是"结构"，而是**生成过程能否持续的最低条件**。
> ③ Ledger 的强制介入点——Ledger 不是记录结果，而是**在条件被否定时，强制冻结所有解释权**。
> **不允许用"级别、调整、延续"进行语义回补**。

### 2.2 现行实装全景（G5 已把 TW 桥真接生产交易流，f9333e21b2/972d5cfefa）

- `TwState`/`TwEvent`/`tw_step`（`rust/src/theta_v0/strategy/ledger.rs`）：现七构造子全部严格保
  `tw()=free+holding+withdrawn` 守恒（`tw_step_preserves_tw` 测试逐事件验证）——**没有任何构造子
  能让 `tw()` 净增/净减**，这是「利润无入口」的代数根因。
- `stage_progression`（`closed_loop/transition.rs:282-326`）：`holding≥notional_in ∧ free≥
  recover_target` ⟹ 退本金；`EnterReady` 五合取 ⟹ EnterEarning。
- 生产接线（`backtest/runner.rs:669-729`）：TW 只经**成本基方向差分**驱动 `ShortDiff`，刻意绕开
  出场价 `px`（即绕开盈亏本身）。
- **关键发现**：`apply_fill`（`runner.rs:1697-1765`）在每次平仓 fill 时已经计算**含费用调整的
  逐笔已实现盈亏** `pnl = pos_sign*(px_exit_net-entry_cost)*close_qty`，驱动 `cash`/`trade_pnls`/
  `equity_curve`——**这套账本已经跑通、已经精确，只是从未喂给 `tw.free`**。
- `tw_ledger_producer_in_place_and_conserved`（`runner.rs:2056-2075`）用**逐 bar 微涨的真实价格
  序列**（`px100_bar`）完成一笔有正 P&L 的平仓，当前断言"TW 守恒 + stage 不推进"——该测试的正 P&L
  在账本里凭空消失，是重装后**具名确定要重写**的测试。
- `earning_shares_structurally_unreachable_from_campaign_tw_conserved`（`runner.rs:2389-2472`）：
  不可达定理的前提正是"没有任何事件让 TW 净增"，一旦新增此类构造子，该定理需限定为 L0 同价版本。
- closed_loop 结构层（`transition.rs:164-206` `schedule_adapter`）：减仓分支 `LedgerEvent::Realize`
  命名具有误导性——它用 `avg_cost` 而非 `price` 计算 `cost_flow`，只释放成本基不釐真实盈亏，是与
  生产层同构的平行缺口。

### 2.3 三个候选设计（提交裁决）

- **候选 A**：`TwEvent` 新增第 8 构造子 `Realize(i64)`（可正可负），复用 `apply_fill` 已有的 `pnl`
  计算作为唯一资金源；`hwm_gain`/`Revalue` 维持诊断-only 不变。
- **候选 B**：可正可负 MTM 账本，权益/可退现金分离（codex 原裁决备选(c)）——证据包自带质询：若该
  权益标量驱动不可逆 `TStage.rank`，会与 `hwm_gain` 同构地复现 p8③ 违规（门是单向的，信号对称与否
  不改变这一点）；若不驱动 stage，则退化为候选 A + 一个不承重的诊断字段。
- **候选 C**：维持现状不裁，`hwm_gain` 保持诊断-only，P2/P3/P4 保持结构不可达。

## 3. codex 裁定原文（完整，未删减；已核验其引用的代码行号/结构描述准确）

> **终局裁定**
>
> 选 **A'：已实现利润入 TW 账本**。A' 是候选 A 加两个硬边界：只认实际平仓 fill 结算的费后 PnL；
> `Realize` 进入 stage 判据前必须先完成同一成交的成本基 `ShortDiff` 更新。候选 B 若驱动不可逆
> `TStage`，仍违反 PDF p8③；若只诊断，则退化为无承重字段。候选 C 不选，因为生产代码已有合法资金
> 源：`apply_fill` 的费后逐笔已实现 PnL。
>
> **推导链**
>
> 1. PDF p8①要求"断裂"必须是否定句：旧账本条件下，GAP3 的否定句是"`TW` 守恒时没有非回补资金源
>    支撑退本金"。R3 已据此否定 `hwm_gain` 棘轮。
> 2. PDF p8②说不变量是生成过程最低条件，不是结构外壳；所以不能靠"曾经浮盈"补一个当前不可结算
>    的退本金条件。
> 3. PDF p8③禁止在条件被否定后用延续/调整/级别回补解释权。`hwm_gain` 违法点正是未实现峰值回撤
>    后仍写入 `free`。
> 4. 已实现 PnL 不同：平仓 fill 一旦发生，`pnl = pos_sign * (px_exit_net - entry_cost) * close_qty`
>    是结算事实，后续价格不能否定这笔结算。见 `runner.rs:1713`。
> 5. 因此 `Realize(d_pi)` 可以成为新的非回补资金源；`d_pi` 必须可正可负。只入正数会重造"利润
>    棘轮"。
> 6. 允许驱动 `stage_progression` 的字段：`free` 中的成本基回流与已实现 PnL、`holding`、
>    `withdrawn`、`notional_in`、`open_legacy_legs`、`risk_mode`。不得驱动 stage 的字段：
>    `hwm_gain`、当前 MTM equity、`forced_pnl`、任何未平仓路径依赖浮盈。
> 7. `TwEvent::is_legal_from`：`Realize(_)` 对 OQ-9 **恒合法**，同 `ShortDiff` 一类，因为它不改
>    stage、不开闭 legacy 腿。真正约束在 producer/source-validity 和 cash-sound gate。
> 8. `Realize` 后不回退 stage。若本金已实际 `withdrawn >= notional_in`，之后亏损不会否定"本金已
>    退"这个历史事实；它只降低后续 `free`/权益，影响风险与后续动作。
> 9. 结算时点：只能锚定实际平仓 fill 的 close 分支。partial close 按平掉数量结算；flip 只对先平
>    的反向段结算；多 fill 可同 bar 聚合。
> 10. 不允许用逐 bar MTM 累计再"持仓归零释放"，除非它严格等同于实际 close fill 当刻的费后 PnL，
>     且没有峰值/路径依赖/未平仓释放语义。
> 11. 当前 `forced_pnl` 是报告用假设强平，不改 `units/cash`，不能喂 `Realize`。只有未来若强平成为
>     真实订单并改变持仓/现金，才可按真实 fill 结算。
>
> **下游清单核定**
>
> §5 方向大体对，但不完整。关键修正：P2 不是唯一可能改变订单流的 TW 下游。`coverage.rs` 中
> P3/P4 成立时会消耗当步裁决并屏蔽 P5..P10，虽然 `RecoverCapital/EnterEarning` 本身不直接产订单，
> 仍可能改变同 bar 普通开/平仓订单。见 `coverage.rs:2510`。
>
> 可执行改动清单：
>
> 1. `ledger.rs`：新增 `TwEvent::Realize(i64)`；`tw_step` 做 `free += d_pi`；新增不变量"非
>    Realize 保 TW，Realize 使 TW 漂移等于 Σd_pi"。更新 `tw_step_preserves_tw`。
> 2. `is_legal_from`：把 `Realize(_)` 归入恒合法；另加 producer/cash-sound 测试，防负 `free`
>    静默落盘。
> 3. `runner.rs`：让 `apply_fill/apply_order` 返回本次 fill 的 realized PnL；在现有 ②' 成本基
>    `ShortDiff` 之后、构造 `TwStepCtx` 之前，应用聚合 `Realize`。
> 4. `transition.rs`：closed_loop 不能继续把减仓 `LedgerEvent::Realize` 当成本基释放；要区分成本基
>    `ShortDiff` 与利润 `Realize`，必要时 `OrderOut` 携 TW 事件序列。
> 5. `coverage.rs`/`mutex.rs`：保留 P2/P3/P4 优先级，但测试要覆盖 P3/P4 masking 对订单流的影响。
> 6. 重写 `tw_ledger_producer_in_place_and_conserved` 为"TW 增量 = 已实现 PnL 量化和"。
> 7. `earning_shares_structurally_unreachable_from_campaign_tw_conserved` 改名/改断言为 L0 同价
>    无盈亏定理；L2 realized-PnL 路径不再被该定理覆盖。
> 8. #135/q4 重跑清单需标注两类 bit-exact 风险：P2 CloseOverlay 直接产订单；P3/P4 priority
>    masking 间接改订单。若后续把 `TStage/ηBucket` 加进 z 并被 χ 消费，μ key 变化也会改订单流。
> 9. `LedgerComp` 仍不在本次范围；但文档要避免把 `LedgerEvent::Realize` 和 `TwEvent::Realize`
>    混称。

## 4. 本工位质询核验（对 codex 裁定的简化质询，非擅自推翻）

1. **§4 修正的真实性核验**：codex 指出「P3/P4 触发时经 `coverage.rs:2513` 分支，`buckets=
   {close:∅, open:∅, record:gamma}`，屏蔽当步普通候选的开平仓桶」——本工位直接读取
   `coverage.rs:2470-2538` 确认代码原文与 codex 描述完全一致（非幻觉）：P3/P4 分支确实会让
   `schedule_order` 基于「本 bar 无普通开平仓、候选全推迟记录」的桶集合计算订单，这与「P3/P4 不
   触发」时的普通路径产出的订单不同。**codex 对我方证据包 §5 的修正成立**，本工位据此接受该修正
   并入下游影响清单（见 §6.6 影响声明）。
2. **裁定内部一致性核验**：候选 A' 与 R3 C' 的推导链完全同构（p8①②③逐条对应），且明确划出
   「可驱动 stage」vs「只能诊断」两组字段清单（推导链第6条），边界精确、无模糊地带——满足
   no-patch-mentality 的严格性要求。
3. **未被其他机制覆盖的检查**：`apply_fill` 的已实现盈亏计算虽已存在，但从未有任何现存测试/文档
   声称它已接入 TW 账本（`g5-impl-20260703.md` §3.4 边界条件 (a) 反而明确诚实声明"无非回补资金源"）
   ——本裁决不是重复劳动，是填补一个此前明确留白的缺口。
4. **严重性判定核验**：裁定要求重写的两个测试（`tw_ledger_producer_in_place_and_conserved`、
   `earning_shares_structurally_unreachable_from_campaign_tw_conserved`）确系当前唯二直接断言
   "TW 守恒/stage 不推进"的生产级见证测试，严重性判定（"具名确定受影响"）准确，非夸大。

**质询结论：codex 裁定成立，未发现误判或需要打回的分歧点。**

## 5. 终局结论（本工位裁定摘要）

- **裁定 = A'（已实现利润入 TW 账本，两条硬边界）**：新增 `TwEvent::Realize(i64)`（可正可负），
  唯一合法资金源 = 实际平仓 fill 的费后已实现 PnL（复用 `apply_fill` 已有计算），`Realize` 对
  OQ-9 恒合法、不驱动 legacy 腿计数，`stage_progression` 只消费「成本基回流+已实现 PnL 后的
  `free`/`holding`/`withdrawn`/`notional_in`/`open_legacy_legs`/`risk_mode`」，不得消费
  `hwm_gain`/当前 MTM 权益/`forced_pnl`/任何未平仓路径依赖浮盈。
- 候选 B（对称 MTM 账本）**未被选中**：若驱动不可逆 stage 门则与 `hwm_gain` 同构违反 p8③；若不
  驱动则无实质效力，已被候选 A' 的诊断字段覆盖。
- 候选 C（维持现状）**未被选中**：生产代码已有合法资金源（`apply_fill` 的已实现 PnL），维持现状
  = 放弃已存在的、无需新造概念的修复路径。

## 6. 结果包六要素

1. **结论**：codex 终局裁定 A'——GAP3 桥的「已实现利润入账」重装方案获批，`TwEvent::Realize(i64)`
   （可正可负，复用生产已有的 `apply_fill` 逐笔费后 PnL）为唯一允许驱动 `stage_progression`
   （P3/P4）与 P2 判据的新增非回补资金源；`hwm_gain`/`Revalue`/当前 MTM 权益/`forced_pnl` 继续
   排除在 stage 驱动链之外（纯诊断或纯报告）。
2. **定义依据**：PDF p8①②③（断裂句法条件/不变量存在依赖/Ledger 强制介入点）+ R3 C' 原裁决（同一
   推导框架的延续）+ `apply_fill`（`runner.rs:1713/1726`）已实现盈亏计算的代码事实——已实现 PnL
   是「平仓这一结算事实」的函数，不因后续价格变动被否定，故不落入 p8③「旧条件不再可维持后用
   延续/调整/级别回补解释权」的禁止范围（与 `hwm_gain` 的未实现峰值有本质区别：后者的"条件"
   （浮盈峰值）会被价格回撤直接否定，前者的"条件"（已发生的平仓）永不被否定）。
3. **边界条件（结论翻转条件）**：(a) 若 `Realize` 的资金源改为逐 bar MTM 累计（非锚定实际 close
   fill）⟹ 裁定不再适用（推导链第10条明文禁止）；(b) 若 `Realize` 被允许驱动 stage 判据但同时
   仍保留 `hwm_gain`/MTM 权益等未实现信号参与判据 ⟹ 违反推导链第6条边界，需重新裁决；(c) 若
   `forced_pnl`（窗口终点报告用假设强平）未来真的变成会改变 `units`/`cash` 的真实订单 ⟹ 那时才
   可按真实 fill 重新评估是否纳入 `Realize` 资金源（当前明确排除）。
4. **下游推论**：#124/G5 的 P2/P3/P4 结构不可达声明需要更新为"结构可达，待 A' 实装"；
   `g5-impl-20260703.md` §3.4/§4 的"可达性限定语"（当前合法账本语义下 stage 恒 CostReduction）
   在 A' 实装后失效，需要下游实施工位重写；#131（q3 阶段二证明文档）与 #135（q4 全量回测 prereg
   冻结）应等待 A' 实装落地后再冻结口径（G3 的 `TStage`/`ηBucket` z 维度届时才非死维度）。
   codex 明确指出的额外风险（P3/P4 priority masking 间接改变同 bar 普通订单，非仅 P2 直接产订单）
   已经本工位核验为真（`coverage.rs:2470-2538`），必须并入 #135 重跑清单的 bit-exact 风险标注，
   不能仅标注 P2 一条路径。
5. **谱系引用**：`.chanlun/review-results/codex-r3-ruling-20260702.md`（R3 终局裁决，本次裁决的
   直接前提与推导框架来源）；`.chanlun/review-results/g5-impl-20260703.md`（本任务的直接上浮源，
   §3.4/§4 边界条件 (a) 是本次裁决要解除的留白）；231号（形式化有效域——L0/L1/L2/L3 认识论等级，
   本裁决属 L1 代码/文本对照裁决，实装后的重跑归 L2）；090号（严格性语法——推导链第6条精确划分
   驱动/诊断字段边界，无模糊地带）；576号/674号（R vs TW 双账本不同构，`LedgerComp` 明确不在本次
   重装范围）。
6. **影响声明**：本文件为裁决归档，零代码改动、零 git 操作。产出=终局裁决记录 + 可执行改动清单
   （9 项，见 §3 codex 原文），需后续实施工位分别执行：
   - `rust/src/theta_v0/strategy/ledger.rs`（新增 `TwEvent::Realize`/`tw_step` 分支/新守恒不变量/
     更新 `tw_step_preserves_tw`）
   - `rust/src/theta_v0/backtest/runner.rs`（`apply_fill`/`apply_order` 返回值扩展/②' 后接
     `Realize` 派发/重写 `tw_ledger_producer_in_place_and_conserved`）
   - `rust/src/theta_v0/closed_loop/transition.rs`（`schedule_adapter` 减仓分支区分成本基
     `ShortDiff` 与利润 `Realize`/`OrderOut` 携带 TW 事件序列/重写
     `earning_shares_structurally_unreachable_from_campaign_tw_conserved` 为 L0 限定版本）
   - `rust/src/theta_v0/strategy/coverage.rs`/`mutex.rs`（新增 P3/P4 masking 对订单流影响的测试
     覆盖，不改判据优先级本身）
   - #135 q4 回测重跑清单（新增 P2 直接改订单 + P3/P4 masking 间接改订单两类 bit-exact 风险标注）
