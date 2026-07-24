# 级别执行与买卖点生命周期既有实现盘点（LEE / 级别账本 / 双开 / 出场侧）

- 日期：2026-07-20（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD=`640609071d`）
- 性质：**只读调研文档工位**——不改 `rust/src` 任何一行，无 git mutation，主仓只读。每个判定附代码行号/commit/文档锚；照实否定为合格产出。
- 任务来源：本 wave「级别执行与买卖点生命周期既有实现盘点」四问（①voice.rs 根=L* 与实装深度；②dual_ledger.rs 完成度与未接线点；③LEE 雏形 grep；④BSP 形成→消费→失效路径）。
- 邻接文档：`multi-level-native-execution-design-20260719.md`（LEE 候选设计本体，下称 **LEE 设计文档**）、`p120-nested-voice-dual-ledger-design-20260718.md`（关⑤施工图，下称 **p120**）、`dual-open-e2e-fix-design-20260719.md`（载体三选一裁定，下称 **e2e-fix**）、`nest-exit-implementation-card-20260719.md` / `nest-exit-gate-impl-20260719.md`（出场证书化）。

---

## ① voice.rs：根=L* 机制 + voice_side + depth_weights 实装深度

### 1.1 原语层：齐全且带测试

`rust/src/theta_v0/strategy/voice.rs`（443 行）实装了声部树的全部纯函数原语：

| 原语 | 位置 | 语义 |
|---|---|---|
| `VoiceSide{Long,Short,Flat}` / `flip` | voice.rs:27 / :39 | σ∈{+1,−1,0}；Flat 不参与翻转（:43） |
| `dir_of_depth` | voice.rs:64 | σ=(−1)^depth，根恒 Long（Origin.VoiceTree `dirOfDepth` bit-exact 镜像） |
| `voice_side(root_side, depth)` | voice.rs:98 | 绝对方向 = root_side·(−1)^depth；**plan_orders 消费本函数**（:76-77），覆盖 StrategyFamily §5 多独立根（含 Short 根，:79-91） |
| `VoiceState` / `ActState` / `act_state` / `target_pos` | voice.rs:125 / :135 / :153 / :171 | 4 互斥动作态 {close,open,hold,wait}，Σ𝟙=1（Origin.VoiceTree `voice_action_exhaustive_exclusive`） |
| `depth_weight` / `within_max_depth` | voice.rs:188 / :200 | w=[0.60,0.30,0.10] 按深度索引；越界返 0.0（未用部分留现金不重分配，:182-184） |
| `RootCandidates` / `root_sel` / `root_sel_mirror` | voice.rs:218 / :265 / :280 | RootSel_Θ 四组合；(1,1) 双触发由镜像反对称**唯一消歧为 Flat**（:246-256，非人为裁决），测试 :396/:407 锁全 4 组合等变 |

资金帽深度权重**已真实接线**（非死代码）：`config.rs:137`/`:156`（默认 [0.60,0.30,0.10]）→ `depth_weight` 被 sizing（`risk.rs:121` w_depth、`mod.rs:371`）、`coverage.rs:1524`/`:1548`（`leg_target` w = depth_weight × dir_weight × w_grade）、`runner.rs:2025`（w_depth_at_entry）/`:2839`、`l3_fullwindow.rs:657`/`:685` 消费。

### 1.2 「v0 只产 depth=0 单声部」（voice.rs:93-95）核实结果：**对 `recognize` 本体仍属实，系统级已过时**

- voice.rs:93-95 自述：「v0 recognize 产**单声部决策**（depth 0，独立根）……嵌套声部树（depth>0 子对冲）……v0 未触发（无嵌套）」。
- **对 `recognize` 本体：属实。** `build_decision` 硬编码 `depth: 0`（`strategy/mod.rs:560`），recognize 恒产独立根。
- **系统级：已被关⑤实装超越。** `recognize_nested`（`strategy/mod.rs:663`，随收口 commit `640609071d` 于 2026-07-19 入库）经**角色门四合取**（ShortDiff 角色 ∧ 活父存在 ∧ 方向对偶 ∧ 深度余量 ∧ 附着一致，mod.rs:641-648/:710-727 + `live_parent_for` mod.rs:747）产 **depth>0 子声部**，`root_side` 继承树根（mod.rs:714-723，σ_child=−σ_parent 代数兑现 :650-654），形态 = 单脊柱赋格树（depth 索引单槽，多孩子分叉列 v1 边界外，mod.rs:629）。接线：runner 双账路径（`runner.rs:3778` per-bar 喂入）+ nautilus 生产路径（`nautilus/strategy.rs:246` `recognize_current`）。A/B 组测试在 mod.rs:1619-1823。
- **连带后果**：LEE 设计文档 §C.4「未就位①：v0 recognize 只产 depth=0 单声部决策，嵌套声部树未触发（voice.rs:93-95）」的结论写于 2026-07-19、与 `640609071d` 同日——**该条已被部分超越**（嵌套声部产出已实装）；但 §C.4 的另两条（②决策出口仍是每 bar 统一 p̃ 净目标、③声部账本只读旁路）经本次核实**仍成立**（见 §③/§④）。voice.rs:93-95 注释本身对 recognize 无错，但作为系统能力声明需在下一波加注 recognize_nested 的存在（登记为文档陈旧项，本工位不改代码）。

### 1.3 根=L* 的实装形态

voice.rs:5 声明「根 = 当前最高有效决策级别 L*」。实装兑现：**每个 BSP 候选经 interpret 环5 分桶后成为 depth=0 独立根**（`VoiceDecision.level = cand.level`，mod.rs:572），级别身份随决策携带；「L*」并未实装为一个全局选级机制，而是退化为「各 BSP 各自带 level 的独立根」——与 LEE 设计文档 §C.4「根=L* 目前退化为每个 BSP 一个独立根」的判断一致。

---

## ② dual_ledger.rs（p120 C1 引擎）：完成度与未接线点

### 2.1 完成度：**引擎本体完成且已接线到回测双臂**（不止任务卡片所列 :40-49/:130/:233）

`rust/src/theta_v0/backtest/dual_ledger.rs`（526 行）：

- **账本**：`DualLedger{cash,q_long,q_short,cost_long,cost_short}`（:42）+ `net_units`/`gross_units`/`equity`（:57/:62/:67，净投影估值 `equity=cash+Net·px`，M30 有效域声明 :24-25）。
- **成交**：`apply_fill_dual`（:130）四腿规则表（:122-127）——分腿不先净额（开空**不触多腿**，:161-169，M13 父仓保持）、realized 只出平仓腿（:76 A' 结算源）、close 超腿 clamp+reject 不开反向（:173-193）、PnL/现金/成本基公式与净额 `apply_fill` 逐字同构（:120）。
- **嵌入恒等**：`compatible_leg_orders`（:233）把净语义订单流翻译为等效腿序列，不变量 `q_long−q_short==units` 逐笔保持（:231-232）。
- **测试**：D1-D8（:304-525），其中 **D8 核心回归锁**（:440）：脚本化净语义订单流经兼容腿标记重放 vs 现行 `apply_order`，cash/equity/realized/fee/trade_pnls/成本基**逐字节相等**（:486-515）。

**接线链**（全部在位）：
`plan_orders_dual`（`strategy/mod.rs:888`）/ `plan_orders_dual_traced`（mod.rs:905，订单↔decision 精确配对）→ `runner.rs plan_and_fill_mtm_dual`（runner.rs:3682；apply_fill_dual 调用点 :3741/:3815/:3917/:3951；`FillOutputDual` 携终态 DualLedger + leg_log，:3611-3626）→ `run_theta_v0_dual`（runner.rs:372）→ wverify 三臂（`wverify_run.rs:1436`/`:1569`/`:1609`，臂①双仓 vs 臂②净额 vs 臂③）→ E 组集成测试（runner.rs:8089 起，:8207/:8265/:8291 嵌套双账、:8318-8343 与 `plan_and_fill_mtm` 链路级对拍）。e2e-fix:8 亦核实「p120 C1 已实装」。

### 2.2 未接线点（逐条照实）

1. **生产路径（nautilus）不持有 DualLedger**：`account_adapter.rs:46` `to_account_state` 把 `net_position` 整体归 `voice_qty[0]`——NETTING 单净仓 vs S_Θ 多根分账的口径差已诚实标注（account_adapter.rs:41-45：「多根对冲时需在适配层拆分账，TODO」）。venue hedge-mode（q⁺/q⁻ 双腿共存）前提列部署裁定（`nautilus/strategy.rs:236-237`，p120 §7 L7）。
2. **TW（总财富三阶段）未接线**：双账路径 `tw_final=None`（runner.rs:3988 诚实 None；wverify_run.rs:1725-1726 注记「E2 G5 同数锁待 TW 接线后启用，p120 §6 E2」）。
3. **入口 F-01 deprecated**：`run_theta_v0_dual`/`run_theta_v0` 均标记 F-01 deprecated（结构确认前视，禁 L2/L3 论据；wverify_run.rs:1728-1733）——臂①一切数值按诊断口径读；生产因果接线在 `ThetaCore::recognize_current`（per-bar 窗口，nautilus/strategy.rs:242-247）。
4. **成本模型缺口**：做空保证金/借券成本未建模（dual_ledger.rs:22，沿用 runner.rs:3119-3121 声明）；双腿共存时 |net| 基 accrual 是近似口径，per-leg 精化列遗留 L6（dual_ledger.rs:23，涉成本数字变更须独立裁定）。
5. **载体 C2/C3 分支未实装**：e2e-fix:21 裁定——§3 方案是 C1 分支（已实装即本文件），§4 是 C2 分支设计，**C3 只有要点素描（§7-0，未展开为方案）**。
6. **LEE 合并未启动**：(ℓ, voice) 二维键控簿是 LEE M1-M4 的事（e2e-fix:19/:185-186），见 §③——当前 DualLedger 是**单级平坦双腿簿**，无级别维度。

---

## ③ LEE 级别事件驱动雏形：grep 判定 = **全缺**（无实装雏形）

### 3.1 grep 结果（2026-07-20，HEAD=`640609071d`）

- 模式 `Ledger_|level_ledger|event_clock|clock_|formation_level` 在 `rust/src/theta_v0/strategy` 与 `rust/src/theta_v0/backtest`：**0 匹配**。
- 同模式扩至全 `rust/src/`：**0 匹配**（`formation_level` 全仓不存在）。
- 近名排查：`BspKey` 仅见于 `rust/src/orchestrator.rs:299`——是编排器去重枚举（`Big{n_seg,s0,s1,n_zs,n_mv}`/`Small`，:304-310），**不是** roadmap:64 的 `BspKey=(formation_level,point_class,side,source_index)`，与 LEE 路由键无关。`rust/src/bin/cp_capability_smoke.rs:504` 的 `level_events` 是图表解析 capability smoke 的局部统计变量（P52/P53 召回按 level 分桶），与 LEE 无关。
- 补充：`Consume_at|consume_at|ManagedBsp` 在全 `rust/src/` **0 匹配**——E2E-O `Consume_at`（LEE 支柱3 的生产侧上游）仍是文档/原型层（`doc-divergence-endtoend-prototype-20260718.md:75-78`），无 rust 实装。

### 3.2 四支柱完成度判定（对照 LEE 设计文档 §C.1）

| LEE 支柱 | 判定 | 依据 |
|---|---|---|
| 支柱1 每级独立仓位账本 Ledger_ℓ | **全缺** | 无 Ledger_ℓ/级别键控账本；现行主路径仍是净额 p̃（`coverage.rs:2762-2763` 诚实声明「v0 净额」）；DualLedger 是单级双腿簿（无 level 字段，dual_ledger.rs:42-48） |
| 支柱2 每级独立事件时钟 clock_ℓ | **全缺** | 无事件钟对象；现行是统一 bar tick + 全级别混合 diff（`runner.rs:809-835` `newly_confirmed_step` 对本 bar 所有级别新确认 BSP 一次 diff，LEE 设计文档 §B.1 根因一） |
| 支柱3 每级独立 BSP 消费 Consume_ℓ | **全缺** | `Consume_at` 无 rust 实装（grep 0 匹配）；消费出口仍是统一 `pi_theta_step` → p̃ → 订单（`coverage.rs:2794-2801`） |
| 支柱4 每级独立 sizing | **同构机制已就位，级别重锚未做** | `depth_weights=[0.60,0.30,0.10]` 按 depth 加权已接线（§1.1）；LEE 需把它从 depth 重锚到 level（LEE 设计文档 §C.1 支柱4 自承「重锚」，非新增机制） |

### 3.3 可复用候选源（非 LEE 实装，LEE M1 起可直接取用的既有件）

- `overlay_state.rs` `VoiceBook`（hedge-mode 逐声部簿，LEE 设计文档 §C.1 点名账本行复用，:59-60）——当前是只读旁路（`overlay_state.rs:27`「pnl_v 不是独立 self-financing NAV」）。
- `nest.rs:540-542` `events_by_level`（下标=塔级别，LEE 设计文档 §C.1 支柱2 点名的事件流源）。
- `runner.rs:809-835` 的全级别混合 diff——按 ℓ 拆分即 clock_ℓ 的雏形输入（LEE 设计文档 §C.2 伪码变化部分①）。
- E2E 五钟（`observed/first_provable/structure_end/confirmed/invalidated`，roadmap:75 E2E-N5）——事件时点的因果定义在文档层就绪，无实装。

---

## ④ 买卖点生命周期：形成 → 消费 → 失效

### 4.1 形成（classifier 侧，单源）

- 判据：`bsp.rs endpoint_to_bsp`（:77）——`EndpointSituation`(:34) 六字段布尔合取 → 非互斥 `BspBits`（2B/3B 可共存，bsp.rs:13-17；`theorem_615` 测试 :276 锁 ConstructorExhaustive⊊complete）。
- 条目：`BspPoint`（bsp.rs:113）携 `source_index`（:115，L0 原始 K 序 pivot 定位）、`pivot_low/pivot_high`（:119/:121，1/2 类结构止损源）、`center`（:130，3 类止损取 zg/zd）——**结构止损价的 single source**（bsp.rs:91-100，路 B 裁定：strategy 不从 bars 重算 pivot）。
- 产出位置：`classification.levels[ℓ].bsp`（recognize_nested 消费点 mod.rs:674-678）。

### 4.2 消费（strategy/backtest 侧）

链路：`interp::assemble_gamma_with_tower` / `coverage_elements_and_gamma_with_tower`（候选 Γ 组装，真父子塔派生角色）→ `interpret` / `interpret_with_close_triggers`（环5 三桶 ℬ/𝒟/𝒲，interp.rs:1151/:1165；**规则2：反向候选关闭同级别活动腿**，interp.rs:1198）→ `recognize`（depth=0 独立根）/ `recognize_nested`（mod.rs:663，角色门产 depth>0）→ `VoiceDecision` → `plan_orders`（mod.rs:252）/ `plan_orders_dual`（mod.rs:888）→ runner 模拟撮合 / nautilus `plan_for_bar` 意图（nautilus/strategy.rs:125）。

### 4.3 失效（出场侧）——**持仓侧机制存在且成体系；「BSP Invalidated」结构事件不存在**

现行「入场后 BSP 被反向破坏时持仓怎么办」的实际答案：**持仓不随 BSP 标签自动失效**——持有（ActState::Hold，voice.rs:141）直到以下任一触发才平仓（§9 closePred 四析取 X = ¬ParentValid ∨ χ^{σ_p} ∨ Stop ∨ RiskClose，`exit.rs:16-18`，Lean 锚 `Origin.SubVoiceOpenClose.closePred`）：

| 机制 | 位置 | 语义 |
|---|---|---|
| **结构止损 = 「结构失效价」** | `risk.rs:67-68`（语义裁定）+ `structural_stop` `risk.rs:74` | 1/2 买=pivot_low、3 买=ZG（卖镜像）；多类重合取**最宽**失效线（risk.rs:64-70，Θ_risk 设计选择，spec:46 未定义项）→ `stop_hit` 触及 → Close（exit.rs:206）。**这就是「BSP 结构被反向破坏」的运行时出口**——BSP 的 pivot/ZG/ZD 被击穿即结构失效，以止损形式平仓 |
| **反向 BSP 关闭 χ^{σ_p}** | exit.rs:213-220（`reverse_signal`）+ interp.rs:1198（规则2 close 桶，π 生产路径）+ mod.rs:699-708（recognize_nested 常规反向关闭） | 持多遇同级别卖侧信号 ⟹ 先平（新反向 BSP 消费 = 旧持仓失效确认） |
| **父失效级联（嵌套）** | `parent_invalid_at` exit.rs:273 + `cascade_exit_decisions` exit.rs:291 | M16 AncOK「父关则子关」，最深优先；depth=0 根恒 false（exit.rs:274-275） |
| **F1 ShortDiff 段终点锚** | exit.rs:228-232（`short_diff_anchor_hit`） | depth>0 对冲腿用「父级回调触及中枢对向沿 ZG/ZD」替代通用反向项（W3 F1，快照 `stop_in.center` 为锚） |
| **RiskClose（退化）** | exit.rs:237-244 | v0 只可计算 Insolvent（equity≤0）；maint_margin/buffer/liq_flag 未建模，诚实退化（exit.rs:107-110） |
| **证书化出场（可选门）** | `exit_decision_for_nested_cert` exit.rs:158 | env `THETA_NEST_CERT_GATE=1` 下 depth=0 反向项从裸 BspBits 升格为 typed nest 证书（`reverse_nest_cert_base` :181）；**默认关闭** = 裸 bits 逐字节不变（exit.rs:42-47）；v1 E2E-N3 真链接口预留 |
| **π 生产路径 active set 失效登记** | `persistent.rs:106`（`invalidated`）/`:165`（`invalidate`） | successor invalidation：只有显式 close/risk close/invalidation 才让 persistent element 退出 live（§9 rule 5，persistent.rs:14/:164）；经 `coverage_step_prebuilt(..., registry)` 接线进生产 π 管线（coverage.rs:2412） |

**不存在的部分（照实否定）**：

1. **无「BSP Invalidated」结构事件**：LEE 支柱2 所需的「BSP Confirmed/Invalidated」事件流（LEE 设计文档 §C.1）没有实装——BSP 一旦进入 `classification.levels[ℓ].bsp` 即无失效标记位（`BspPoint` 无 invalidated 字段，bsp.rs:113-165）；分类层 grep 到的 `invalidate*` 全部是**增量缓存失效**（`recursive_tower.rs:1437` `invalidate_cp_lifecycle_dirty_dependencies`、`classifier/mod.rs:145` 等，工程缓存一致性语义），不是 BSP 生命周期语义。
2. **无级别归因的失效传播**：跨级影响（父级失效要求子级强平）目前只有**声部树内**的 depth 级联（exit.rs:291），没有 LEE「级别封闭 + 显式跨级消息」机制（LEE 设计文档 §C.2 不变量三）。

---

## ⑤ 四域既有实现盘点表 + 完成度判定

| 域 | 既有实现 | 完成度判定 |
|---|---|---|
| **声部树 / 根=L\*** | 原语齐全（voice.rs 全函数+测试）；depth_weights 已接线 sizing；`recognize` 仍 depth=0 单声部；`recognize_nested`（关⑤，`640609071d`）已产 depth>0 子声部并接线 runner 双账 + nautilus | **语义层就位 + 嵌套产出已实装（单脊柱）**；多孩子分叉树、voice.rs:93-95 注释系统级陈旧为遗留 |
| **双开 / 双账本（p120 C1）** | DualLedger + apply_fill_dual + compatible_leg_orders + D1-D8 逐字节锁；plan_orders_dual → plan_and_fill_mtm_dual → run_theta_v0_dual → wverify 三臂全接线 | **引擎完成、回测双臂接线完成**；未接线：nautilus 持仓真相源（NETTING 口径差 TODO）、TW、F-01 前视处置、做空成本模型（L6）、C2/C3 载体分支 |
| **LEE 级别账本 / 事件钟** | grep 判定：**无任何实装雏形**（Ledger_ℓ/clock_ℓ/formation_level/Consume_at 全 0 匹配）；可复用源：VoiceBook、events_by_level、runner diff、depth_weight 同构、E2E 五钟（文档层） | **全缺**（四支柱仅支柱4 有同构机制）；LEE 设计文档（20260719）+ e2e-fix 合并次序（声部独立执行先行、LEE M1-M4 在其上加 level 路由）是既定路径 |
| **BSP 生命周期（形成→消费→失效）** | 形成/消费链完整且单源（bsp.rs → interp 三桶 → recognize(_nested) → plan_orders(_dual)）；失效 = 持仓侧五机制（结构止损=结构失效价、反向 BSP 关闭、父失效级联、F1 锚、RiskClose）+ persistent registry successor invalidation + 可选证书门 | **失效机制：持仓侧有（成体系），结构事件侧无**——「BSP Invalidated」作为可消费事件不存在；「入场后 BSP 被反向破坏」的现行兑现 = 结构止损价触及或反向 BSP 信号触发平仓，中间态 Hold |

## ⑥ 可复用项清单（LEE/级别执行后续工位直接取用）

1. `overlay_state.rs` `VoiceBook`/`SepLeg`（账本行；加 level 字段即 Ledger_ℓ 行——LEE 设计文档 §C.1 已点名）。
2. `nest.rs:540-542` `events_by_level` + `runner.rs:809-835` diff（clock_ℓ 事件流源，按 ℓ 拆分）。
3. `voice::depth_weight` 机制 + `coverage.rs:1524/:1548` `leg_target` 权重管线（支柱4 sizing 同构，level 重锚点）。
4. `dual_ledger.rs` D8 嵌入恒等协议（compatible_leg_orders 重放逐字节对拍）——LEE M1/M2「加性细化 bit-exact」的验收协议模板（e2e-fix:150 已指明三者同一协议）。
5. `exit.rs` 四析取 + `HeldVoice` 台账 + cascade（Ledger_ℓ 内声部出场语义的单源，生产/回测双路径已共享，exit.rs:3-14）。
6. `persistent.rs` `PersistentRegistry`（跨 bar active set 持久化 + 显式 invalidation 先例，已接线 coverage 生产 π 路径）。
7. `recognize_nested` 角色门四合取 + `live_parent_for` 附着一致（mod.rs:710-775）——(ℓ, voice) 二维键控下「声部归属级别」的既有判据件。
8. E2E 五钟因果定义（roadmap:75，文档层）——clock_ℓ 合法 tick 的无前视锚。

## ⑦ 锚点索引

- **commit**：`640609071d`（收口，关⑤ recognize_nested/双账接线/本盘点基线 HEAD）。
- **代码**：voice.rs:27-280（声部原语）；mod.rs:560（depth:0 硬编码）/:663/:710-727/:888/:905（recognize_nested/plan_orders_dual）；dual_ledger.rs:42/:130/:233/:440（引擎+D8）；runner.rs:3682/:3741/:3778/:3988（双账管线+TW None）；exit.rs:16-18/:114/:133/:158/:206/:273/:291（出场全机制）；risk.rs:67-68/:74（结构失效价）；persistent.rs:106/:165（successor invalidation）；account_adapter.rs:41-46（NETTING 口径差）；interp.rs:1151/:1165/:1198（三桶+规则2）；coverage.rs:2412/:2762-2763（registry 接线+净额声明）；bsp.rs:77/:113-130（BSP 形成+止损 single source）。
- **文档**：multi-level-native-execution-design-20260719.md §B（扁平化两根因）/§C（LEE 四支柱+不变量）/§C.4（根=L* 判定，①条已被关⑤部分超越）/§D（M0-M4）；p120-nested-voice-dual-ledger-design-20260718.md §4（方案 B）/§4.4（嵌入恒等）/§7（L3/L6/L7 遗留）；dual-open-e2e-fix-design-20260719.md :8/:19/:21/:145/:149-153/:185-186（C1 已实装核实+LEE 合并次序+C3 未展开）；nest-exit-implementation-card-20260719.md §2.4（证书化出场）；doc-divergence-endtoend-prototype-20260718.md :75-78（Consume_at 原型签名，无 rust 实装）。
- ** Lean 契约**：Origin.VoiceTree（dirOfDepth/flipDir/actState/targetPos/voice_action_exhaustive_exclusive）、Origin.SubVoiceOpenClose.closePred（exit.rs:16-18）、StrategyFamily.lean:570（long_short_both_open_allowed，voice.rs:79-84）。
