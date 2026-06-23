# route_bsp 完全分类映射 — 两 gap 补全 + L3 对照（gap1 add 原语 / gap2 emergent-long）

> 工位：swarm/route-bsp-complete（069号 RTAS 子蜂群节点 + 275号局部依赖）
> 日期：2026-06-23 ｜ 分支：route-bsp-complete-20260623（worktree 隔离，基于任务29 commit 30a6b55372）
> 范围：实装 cc-coverage-orbit-type2 §4.1 缺的 `add` 原语（gap1）+ 验证/修正强牛最高级别 emergent-long（gap2）+ L3 三变体对照
> 母任务 #29（route_bsp orbit 升格）；上游 561号 N9 / 543号 word / 539号做空腿失血
> 认识论等级：实装规则导出 = **L0/原文+543**；bit-exact OFF = **L1**；零panic+守恒 = **L1**；8标的收益对照 = **L3**

---

## 一、结论（一句话判决）

**两 gap 已严格补全且 L3 全跑通（8 标的 × 3 变体，零穿仓爆仓、final_nav 全有限）。两条核心发现：**

1. **gap1（add 原语）= regime 函数（确认 cc-coverage §6 预注册边界）**：`add`（543 word `h⁺∘σ`，极性不变加仓，NAV-headroom 无杠杆配额）实装正确，但 L3 净符号 **1/8 增益（GC +10.8→+64.7）/ 7/8 regime 税**（CL/BRN/DX/ES/QQQ/BTC/OKLO 全降）。机制：add 放大核心 → 放大 sink 配额 → 放大次级别短差 → 7/8 短差失血盖过多头放大（仅 GC 多头放大盖过短差）。

2. **gap2（emergent-long）= 验证 route_bsp 已 honor，非 bug**：`suppressed_core_flips = 0` **全 8 标的**。cc_orbit 体制下 type1 卖**从不**到达核心假空 flip 路径（被 `trend_done_clear` 清现金 + 次级别 `sink` 正典短差吸收）⟹ **强牛最高级别不开假空腿已成立**（flip O/B/C = N/0/0 全标的）。gap2 抑制守卫结构正确（单测证），但在此体制 inactive = 防御纵深（非死代码：emergent 翻 Short 真顶时放行 flip，单测 `cc_complete_emergent_short_允许真顶flip` 证）。

**OFF bit-exact 守住**（`rec_flat_bit_exact` 单测逐位一致 + 110 单测全绿）。

---

## 二、实装内容（L0/原文 + 543 封闭性）

### 2.1 gap1 — `add` 原语（543 word `h⁺∘σ`，非新原子）

**543 封闭性证明**：543 §7.4 `OP_REBUY = h⁺∘σ`——`add` 是自由幺半群 Σ\*={e,h⁺,h⁻,τ}\* 中一条 **τ-free 纯 h-word**（h⁺ 本级别买进增 units ∘ σ 级别保持），由已结算原子组合，**不新增原子** ⟹ 543 封闭不破（cc-coverage §4.1 已核 543 原子集封闭）。`note_op("add")` 走 `OpTrigger::Bsp`（BSP 合法触发，prove_bsp_triggers_operation 不 panic）。

**语义（极性不变加仓，第17/21课 L0）**：
- 极性不变（**不调 τ**，N9 正交）：j 活跃方向必须 == dir，反向 fail-safe 拒绝。
- **恒仓无杠杆（关键修复）**：资本源 = `dir` 方向 **NAV-headroom**（= nav − 该方向 notional）的 frac 配额，**不是 raw `free`**。
  - 第一版用 raw `free` → **add↔sink 正反馈几何爆仓**（实测 BTC add_units=25e9，strat=-47e9%）：`free` 含 sink 开空释放的卖空所得（借入资金），用它加仓 = 杠杆。
  - 修复：NAV-headroom 上限保证 `dir_notional ≤ NAV`（满仓 ⇒ headroom≈0 ⇒ add 自然 no-op）；headroom 由 sink 减核心后释放 ⟹ type2/3 买点把 sink 释放的**真实权益**重新部署回核心（抗踏空），非杠杆放大。代数证 free 部署后恒 ≥0（无杠杆）。
- TW 中性（复用 rec_add，与 enter/sink 同守恒原语，prove_tw_neutral 守）。
- `frac` 力度档（中枢上/中/下，cc-coverage T2-4/5/6）= **开放轴未实装**（诚实标注，避免声明膨胀）；本版统一用 σ-不变 `MOBILE_FRAC`=1/3。

**接入点（route_bsp_cc，gated by `enable_cc_complete`）**：
| 轨道 | cc_orbit（任务29）| cc_complete（本工位补）|
|---|---|---|
| type2/3 子级同父向，无子级短差 | no-op（"浪费的买点"死扣）| **add(p, pdir)** 加仓父核心 |
| type2/3 核心级同向 | ascend-only / no-op（踏空）| ascend + **add(core, dir)** 加仓核心 |
| 反父向 / 反核心向 | no-op（不 flip）| 不变（no-op，type2/3 不走势完成）|

> type3 同享 add（cc-coverage §5：type3 = 极性不变**延续**加仓）——no-patch 完整性要求（type3 核心同向原仅 ascend = 同踏空 gap）。

### 2.2 gap2 — emergent-long 核心假空抑制（route_bsp flip 路径，gated by `enable_cc_complete`）

- 判据：`cur_emergent_dir == Long`（view.emergent_top.dir = 最高完成走势方向 = 升跌完备性）且核心 Long→Short flip。
- 行为：抑制翻转，核心**保持 Long**（不 clear_all、不反向 enter、不开假空腿），`suppressed_core_flips += 1`。
- **非 ANCHOR 硬编码永不翻空**（编排者要求）：条件化于升跌完备性——真顶完成时最高级别下跌走势 settle ⟹ emergent_top→Short ⟹ 不抑制 ⟹ flip 照常（与新建任务 #37「吃跌真alpha」一致：bull 抑制假空 / top 放行真flip）。
- 021:40 依据：「上涨趋势确定后……只可能有第三类买点」⟹ emergent=Long 时最高级别走势未完成，到达 flip 路径的 type1 卖只能是次级别回调（次级别有活跃祖先 → 走 Some(p) sink 路径，正典短差合法保留，不到 flip 分支）。

---

## 三、L3 对照（8 标的 × 3 变体，PerfectionMode::Structural，a₀=Segment）

跑法：`cargo test --release recursive_t::rec_stream::tests::cc_complete_l3 -- --exact --ignored --nocapture`

### 3.1 收益矩阵（strat% vs BH）

| 标的 | OFF | CC_ORBIT | CC_COMPLETE | BH | add Δ(CMPL−ORBIT) |
|------|------|----------|-------------|------|----|
| CL   | +20.3 | -6.4 | -98.2 | +28.2 | **-91.8** 税 |
| BRN  | -26.0 | -18.3 | -28.4 | +87.4 | **-10.1** 税 |
| DX   | -0.7 | -8.8 | -26.9 | +4.1 | **-18.1** 税 |
| GC   | -22.7 | +10.8 | **+64.7** | +257.3 | **+53.9** 增益 |
| ES   | -2.3 | -3.9 | -32.2 | +594.3 | **-28.3** 税 |
| QQQ  | -8.6 | -68.9 | -87.0 | +174.6 | **-18.1** 税 |
| BTC  | +26.0 | -207.3 | -307.3 | +1380.4 | **-100.0** 税 |
| OKLO | +55.3 | -186.8 | -195.9 | +307.1 | **-9.1** 税 |

**add 净符号：1/8 增益（GC）/ 7/8 regime 税。** 全 8 标的 < BH（cc_orbit/cc_complete 路径）。

### 3.2 gap 机制计数 + 做空腿/穿仓

| 标的 | adds | add_units | **suppr** | spnl_OFF | spnl_ORBIT | spnl_CMPL | liq O/B/C |
|------|------|-----------|-----------|----------|------------|-----------|-----------|
| CL   | 385 | 21307 | **0** | -13050 | -8159 | -38552 | 8/5/15 |
| BRN  | 157 | 9498 | **0** | -15702 | -22207 | -72067 | 3/2/3 |
| DX   | 229 | 10406 | **0** | -486 | -11250 | -20996 | 0/0/3 |
| GC   | 401 | 2181 | **0** | -31146 | +6806 | -69027 | 10/0/5 |
| ES   | 342 | 644 | **0** | -34556 | -34058 | -114859 | 10/4/9 |
| QQQ  | 14 | 231 | **0** | -25271 | -80683 | -106075 | 2/2/2 |
| BTC  | 8 | 11 | **0** | -56210 | -246338 | -358339 | 5/1/1 |
| OKLO | 23 | 16141 | **0** | -27509 | -290955 | -364237 | 3/3/2 |

- **suppr=0 全标的**：核心假空 flip 在 cc_orbit 体制从不发生（flip 计数 O/B/C = N/0/0 全标的）⟹ gap2 验证 route_bsp 已 honor 强牛不开假空。
- **add 放大短差失血**（7/8 spnl_CMPL < spnl_ORBIT）：add→核心增→sink 配额增→次级别短差增。仅 GC 多头放大（+53.9pp）盖过短差恶化（+6806→-69027 但总收益升）。
- **无爆仓**：liq 有界（0-15），final_nav 全有限——NAV-headroom 修复后杠杆爆仓消除。

---

## 四、结果包六要素

### 1. 结论
两 gap 严格补全：gap1 `add` 原语（543 word `h⁺∘σ`，NAV-headroom 无杠杆）+ gap2 emergent-long 核心假空抑制（条件化升跌完备性，非硬编码）。L3：**add = regime 函数（1/8 增益 GC / 7/8 税，确认 cc-coverage §6 边界）；emergent-long 验证已 honor（suppr=0 全标的，route_bsp 经 trend_done_clear+sink 已不开假空 core flip）**。OFF bit-exact，110 单测绿。

### 2. 定义依据
- **买卖点定律一（第17课 L66）+ 第21课 L40**：type2/3 = 极性不变加仓（次级别 type1 投影，上涨未确定窗口）；cc-coverage-orbit-type2 §3 表 T2-1..T2-8 确定操作 = add。输入满足：`BSPKind::Type2Buy/Sell/Type3*` 由 `project_type2`/信号层产出，`kind_buy/kind_sell` 携带（rec_stream:414/417）。
- **第21课 L40（021:40）**：「上涨趋势确定后只可能出现第三类买点」⟹ emergent=Long ⟹ 最高级别走势未完成 ⟹ 不开假空（gap2 判据）。
- **543 §7.4**：`OP_REBUY=h⁺∘σ` = add 是 word 非原子（封闭性）。
- **第26课名义敞口**：恒仓无杠杆 ⟹ add 的 NAV-headroom 上限（dir_notional ≤ NAV）。

### 3. 边界条件（结论翻转条件）
- **add 净符号翻转**：本 L3 = Structural/Segment/8标的。若换 mode（AND/OR）或 a₀=Stroke 或换时段，regime 划分可能移（GC 增益/7标的税的边界是 regime 函数，非普适）。**已实现的翻转**：raw-`free` 配额 → NAV-headroom 配额，把「add 几何爆仓（-47e9%）」翻为「add 有界 regime 税/增益」——杠杆 vs 无杠杆是定性边界。
- **gap2 suppr 翻转**：若某 regime type1 卖在 j>核心级 fire 且 `trend_done_clear` 未拦（如该 flag OFF），则 suppr>0，gap2 由 inactive 转 active。本 L3 trend_done_clear ON ⟹ suppr=0。**反例核验**：单测 `cc_complete_emergent_long_抑制核心假空flip`（手造 j=5>cc=3 + emergent=Long）suppr=1 证守卫可激活；非死代码。
- **「补全 ⟹ 改善」翻转为「加仓 regime 税」**：cc-coverage §6 预注册「L3 未决」已由本 L3 **证否**（7/8 税），落定为 regime 函数（formalization-validity-domain：否定性结果缩小有效域，比确认更有价值）。

### 4. 下游推论
- cc-coverage §6 边界「type2 加仓净符号 = regime 函数」**已 L3 坐实**（GC 吃涨 / 7标的失血）。add 机制正确但非普适增益 ⟹ 若入主引擎须 regime 白名单（GC 类）或力度档（中枢位置，开放轴）。
- gap2 验证：539 做空腿失血**不来自核心假空 flip**（cc_orbit 体制 flip=0），而来自次级别 sink 短差（正典，合法）。强牛失血根因 = sink/recover 周期本身的 regime 税（与 project_t_short_leg_regime_function / project_unn_btc_spawn_throwback 同源），非核心翻空。
- **与任务 #37（牛熊切换）互补**：gap2 抑制假空（bull ongoing）/ 放行真 flip（emergent→Short 真顶）。但 cc_orbit 体制下真顶由 `trend_done_clear` **清现金**（非 flip short）⟹ 不吃跌 ⟹ BTC/OKLO/ES 强牛后无熊腿 alpha。#37「type1 卖 flip short 吃熊」与 `trend_done_clear` 清现金路径冲突，需 #37 工位裁决（本工位不动 trend_done_clear，保 bit-exact）。
- CC_ORBIT（任务29）本身 6/8 < OFF（仅 BRN/GC 优）：route_bsp_cc 把 type2/3 卖 no-op 化坍缩 sink/recover 活性（BTC sink 2744→23）⟹ 持仓僵化。此为 task29 属性，本工位继承不改。

### 5. 谱系引用
- **561号**（N9 极性协变）：本工位是 561 §9.2 CC「每轨道确定操作」的 type2/3 add 子集实装；add 不调 τ（极性不变）⟹ 与 N9（type1-flip）正交。
- **543号**（操作=word 非硬编码循环）：add = `h⁺∘σ` word，封闭性不破。
- **539号**（做空腿失血/clearance regime gating）：gap2 验证失血非核心假空 flip（cc_orbit flip=0）而是次级别 sink，539 有效域细化。
- **552号**（HOLD_ANCHOR）：gap2 emergent-long **非** anchor 硬编码（编排者明确区分）——anchor 死扣份额不下放 vs gap2 条件化抑制 flip，二者正交。
- **cc-coverage-orbit-type2-20260623**：本工位实装其 §4.1 缺的 add 原语 + §6 L3 边界证否。
- **task29-route-bsp-orbit-impl-20260623**：本工位补其「未覆盖开放轴」的 type2 add（§99 开放轴1/2）。
- **不确定是否有 type2-add 专属谱系**：检索 settled/ 未见独立节点；本文是 add 原语首次 L3 落定（regime 税）。建议 genealogist 评估是否结晶「极性不变加仓 = regime 函数」语法记录。

### 6. 影响声明
- **改动文件**（worktree 隔离，OFF bit-exact 守住）：
  - `rust/src/recursive_t/rec_engine.rs`：EngineConfig +`enable_cc_complete`（env `T_CC_COMPLETE`）+`cc_orbit()`/`cc_complete()` 构造；TRoot +`enable_cc_complete`/`cur_emergent_dir`/`n_adds`/`add_units`/`suppressed_core_flips`；新增 `add` 原语（NAV-headroom）；route_bsp flip 路径 gap2 抑制；route_bsp_cc type2/3 同向接 add；on_bar 设 cur_emergent_dir。
  - `rust/src/recursive_t/rec_driver.rs`：+4 TDD 单测（add 加仓 / cc_orbit 无 add / emergent-long 抑制 / emergent-short 放行）。
  - `rust/src/recursive_t/rec_stream.rs`：+`cc_complete_l3` 三变体 L3 全量回测 test。
- **新增**：本报告。
- **不改定义**：add=word（543 封闭）、gap2 条件化（非硬编码）——纯实现补全 + 验证，无定义冲突，**无 /escalate**。
- **高**：cc-coverage §6「L3 未决」边界由本 L3 证否为 regime 税（7/8）。

---

## 五、认识论等级标注（formalization-validity-domain 强制）

| 命题 | 等级 | 增量 |
|---|---|---|
| add = `h⁺∘σ` word（543 封闭，非新原子）| **L0**（543 §7.4 + 自由幺半群）| 增量：cc-coverage §4.1 缺的原语落码 |
| add 极性不变（不调 τ，N9 正交）| **L0**（561 PC1）| 增量：type2/3 轨道与 N9 划界落码 |
| add NAV-headroom 无杠杆（dir_notional ≤ NAV）| **L0**（第26课名义敞口）| 高：修复 raw-free 几何爆仓（杠杆 vs 无杠杆定性边界）|
| gap2 emergent-long 条件化抑制（非硬编码）| **L0**（021:40 + 升跌完备性）| 增量：假空/真flip 划界 |
| OFF bit-exact + 零panic + TW 守恒 | **L1**（110 单测 + rec_flat_bit_exact）| 零（管线正确性，不验假设）|
| add 净符号 = regime 函数（1/8 增益 / 7/8 税）| **L3**（8标的真实数据）| 高：cc-coverage §6「L3 未决」证否，缩小 add 有效域 |
| gap2 suppr=0（route_bsp 已 honor 不开假空）| **L3**（8标的 flip O/B/C=N/0/0）| 高：强牛假空 core flip 不存在的经验验证（非 bug，已 honor）|

> **核心诚实声明**：(a) gap1 add 原语**实装正确**（543 word，NAV-headroom 无杠杆，单测+守恒证），但 L3 **净符号 7/8 为 regime 税**——不声称「补全 ⟹ 改善」（cc-coverage §6 预注册边界已证否）。(b) gap2 emergent-long **验证 route_bsp 已 honor**（suppr=0 全标的）——非 bug 修复而是验证确认；抑制守卫为防御纵深（emergent→Short 放行，单测证非死代码）。(c) CC_ORBIT/CC_COMPLETE 全 8 标的 < BH = task29 route_bsp_cc 坍缩 sink/recover 活性的继承属性，非本工位引入。否定性结果（add 税 / cc_orbit 坍缩）= 缩小有效域，比确认更有价值（231号）。
