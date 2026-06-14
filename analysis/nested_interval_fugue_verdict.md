# 区间套递归赋格（nif）判决——交易 floor 消费规则 L3 实证

> 任务（编排者 2026-06-14）："改操作语义层——BSP 的消费规则。" 三层改动严格实装：
> ① segment 级别不独立开仓（成本门递归终止）② 仓位集中高级别 + 区间套定位
> ③ 出场对齐同级别反向 BSP。实装：`rust/src/trading/nested_interval_fugue.rs`
> （mode `nif{N}`，N = min_trade_ladder 交易 floor）。
>
> 认识论等级：**L3**（8 标的 × 真实 databento 1min × 与 URS 基线同信号层口径；
> 可证伪且**部分被证伪**——见 §3）。退化锚 L0：min=FIRST_BSP_LADDER ⇒ URS bit-exact
> （Rust 单测 `nif_floor_equals_urs` 逐字段验证）。

---

## 1. 三层改动的代码形式（操作语义层，信号/会计层零改动）

引擎是 URS（`unified_recursive.rs`，缓存基线 2/8 最优）的**构成性推广**——加
`min_trade_ladder` 交易 floor。会计原语（`pop_tail`/`unwind_to`/守恒律 §1-§8）、
区间套定位（`rec_sub_evidence`/located）、E\* 涌现层判据**逐字复用**。

| 改动 | 概念依据 | 代码形式 |
|------|---------|---------|
| ① segment 不独立开仓 | 概念链第16环（成本门=递归终止）的**结构形态** | F 的 `top` 搜索 `[min_trade_ladder, MAX)`；E spawn floor 抬到 `tail.ladder ≤ min ⇒ floor_stop`；C 翻转落点 `ladder−1 ≥ min` |
| ② 仓位集中高级别 + 定位 | 第14环（区间套）+ 第15环（级别=操作量） | F 最高 θ 涌现层满仓 + `nf_buy` 向下定位；E 释放 `θ_sub/θ_total` 配额（θ_total 只跨 `[min, MAX)`） |
| ③ 出场对齐 | 第14环 + 第17环（降成本） | 根持有到 E\*（≥根入场级别）反向 BSP 才清仓/翻转；低级别反向 → E 降成本 spawn（非平根仓）；子持有到自身级别反向才回补 |

**改动1 的关键区分（严格性）**：低于 min_trade_ladder 的 BSP **仍被检测、仍武装
区间套窗口、仍参与递归定位链**（`recursive_confirmed` 下界恒 `FIRST_BSP_LADDER`）
——信号层零改动，定位词汇完整。只有**开仓动作**受 floor 约束。M2 判据
（held_bars/entries 在 < min 级别全零）在 8/8 验证此点。

**退化定理**：min = FIRST_BSP_LADDER(2) ⇒ 三处 gate 全退化为 URS 的 floor 判据
⇒ bit-exact = URS。⇒ nif 是 URS 之上的严格叠加，非平行重写。

---

## 2. 守恒律（任务硬要求：violation = panic）

`§8.1 Σ链上在手单位 = N_base` 逐 bar `assert!`（违反 = panic abort，非静默 Err）。
**8 标的全程零 panic**（~24M bar 真实数据）⇒ 守恒零违反。NAV 不变性由 `pop_tail`/
`unwind_to`（§11 审计 D1/D6 CONFORMS）构造保证。

---

## 3. L3 结果（nif3 = 交易 floor 抬到 move(L1)；nif4 = 抬到 recL2）

| 标的 | BH% | URS% | URS P1 | **nif3%** | nif3 P1 | **ΔURS** | nif4% | nif4 P1 |
|------|-----|------|--------|-----------|---------|----------|-------|---------|
| OKLO | 307.1 | 321.9 | ✓ | **921.9** | ✓ | **+600.0** | 166.1 | ✗ |
| QQQ | 174.6 | 166.2 | ✗ | 148.2 | ✗ | −18.0 | 99.9 | ✗ |
| BRN | 87.4 | 56.5 | ✗ | 59.1 | ✗ | +2.6 | 46.9 | ✗ |
| DX | 4.1 | 4.4 | ✓ | 3.3 | ✗ | −1.1 | 2.5 | ✗ |
| ES | 594.3 | 488.4 | ✗ | 472.6 | ✗ | −15.8 | 459.7 | ✗ |
| GC | 257.3 | 245.0 | ✗ | 215.8 | ✗ | −29.2 | 219.7 | ✗ |
| CL | 28.2 | 4.7 | ✗ | 6.5 | ✗ | +1.8 | 1.9 | ✗ |
| BTC | 1380.4 | 549.9 | ✗ | **1286.9** | ✗ | **+737.0** | 434.4 | ✗ |

**P1 计数**：URS 2/8（OKLO,DX）→ nif3 **1/8**（OKLO）→ nif4 0/8。
**ΔURS 正域**：nif3 ≻ URS 于 **{OKLO, BRN, CL, BTC} = 4/8**（强趋势/高波动/油链）；
≺ URS 于 {QQQ, DX, ES, GC}（平滑/震荡/无趋势）。

### 3.1 机制坐实（OKLO 逐层分解——诊断 §3/§6 的直接验证）

| | URS（segment 不消除） | **nif3（segment 消除）** |
|---|---|---|
| segment/short | 444 笔，净 **+10,302**（摩擦地板下噪声） | — 消除 |
| segment/long | 3 笔，净 −20,527 | — 消除 |
| move(L1)/long | 1 笔，**+213,666** | **3 笔，+805,829** |
| move(L1)/short | — | 19 笔，+46,066（降成本上移到 move 级） |
| recL2/long | — | 1 笔，+48,017 |
| 合计 | 448 笔，+321.9% | **23 笔，+921.9%** |

**`why_not_profitable.md` §6 诊断被实证**：444 笔 segment 散单净值≈0（+10K 噪声），
却稀释了高级别 swing 的资金/配额。消除后资金集中在 move(L1)/recL2 swing ⇒ 同一段
高级别多头从 +213K 涨到 +805K。BTC 同构（+737pp，1287% 逼近 BH 1380%）。

### 3.2 nif4 全面劣于 nif3

交易 floor 抬到 recL2 ⇒ 全标的退化为 1–3 笔近恒仓（入场点延后到 recL2、无 move 级
降成本）⇒ 普遍劣化。move(L1) 是摩擦地板的正确位置（与 §4.2"最低可交易级别 =
f(标的摩擦, 波动率)"一致——1min databento 摩擦量级下 floor 落 move(L1) 而非 recL2）。

---

## 4. 结果包六要素

1. **结论**：三层 BSP 消费改动严格实装为 `nested_interval_fugue.rs`（mode `nif{N}`）。
   守恒零违反、M1/M2 机制活性 8/8 验证。**alpha 判决：交易 floor gate 是 regime
   函数**——抬到 move(L1)（nif3）于强趋势/高波动域大幅改善（OKLO +600pp 翻 P1、
   BTC +737pp），于平滑/震荡域劣化。**P1 绝对计数 1/8 < URS 2/8**（OKLO 增益换不回
   DX 失血）；**ΔURS 正域 4/8**（机制对 §6 诊断的强趋势侧确认）。

2. **定义依据**：
   - 第16环（成本门=递归终止）：min_trade_ladder = 势在该级别不存在的**结构 floor**
     （非 θ<friction 连续阈值）——`why_not_profitable.md` §3 segment close 幅度
     0.02–0.19% < 摩擦地板 0.2% 是势消失的经验形式。
   - 第14/15环（区间套 + 级别=操作量）：F 高级别满仓 + nf 定位 = 仓位集中高级别。
   - 第17环（降成本）：低级别反向 → E spawn（非平根仓）= 出场对齐同级别。
   - 会计 §1-§8（`nested_fugue_accounting.md`）：守恒律逐字复用 URS/v4。

3. **边界条件（结论翻转条件）**：
   - 若降低摩擦假设（低成本期货 ~0.02%/腿），摩擦地板下移，floor 可降到 segment
     ⇒ nif3 优势收窄趋近 URS（§4.2 预言）。
   - 若 min_trade_ladder 设为 per-asset（OKLO/BTC=3，QQQ/ES/GC=2 即 URS），则正域
     可叠加为白名单 ⇒ 但这是 regime 路由非普适配置（§5 否证普适性）。
   - 若在更多强趋势标的上 nif3 ΔURS ≤ 0 ⇒ "高级别集中改善强趋势 alpha"被否证。

4. **下游推论**：
   - 交易 floor 不是普适 alpha 杠杆——`why_not_profitable.md` §6 #1（friction floor
     gate）**部分确认（强趋势域，机制经 OKLO/BTC 逐层坐实）+ 部分否证（震荡域）**。
     与 §6 #5（"放弃单配置普适跑赢，改 regime 路由"）共振。
   - move(L1) 是 1min databento 摩擦量级的正确交易 floor（nif4 recL2 过高）。
   - nif3 正域白名单 {OKLO, BRN, CL, BTC} 可作 regime 路由的候选（强趋势/高波动 +
     油链），但**未确立普适 P1 改善**。

5. **谱系引用**：
   - 与 `project_nrf_v4_strict_accounting`（清仓频率 regime 函数）、
     `project_constitutive_throughput_falsified`（A′ 选层踏空）、
     `project_underperform_bh_osc_bleed`（osc 腿失血）**共振**：级别/floor 选择
     是 regime 变量，非普适常量——本结果是"清仓频率/操作强度 regime 不可约"的
     又一形态（交易 floor 形态）。
   - 与 `why_not_profitable.md`（本任务上游诊断）**直接验证关系**：§3/§6 的
     "segment 散单稀释高级别 alpha" 机制被 nif3 OKLO 逐层分解坐实（+213K→+805K）。
   - **新增量（语法记录类，待编排者裁决是否结晶）**：**"交易 floor（min_trade_ladder）
     的有效域是 regime 函数——强趋势/高波动域抬高 floor 集中高级别 alpha 改善，
     震荡域劣化；move(L1) 是 1min 摩擦量级的 floor 位置"**。

6. **影响声明**：
   - 新增 `rust/src/trading/nested_interval_fugue.rs`（引擎）+ `PolarityMode::
     NestedInterval` 变体 + parse(`nif{N}`) + dispatch（`positional.rs`）+ 模块
     注册（`mod.rs`）。**未改信号层（`buysellpoint.rs`）、未改会计层
     （守恒律/`pop_tail`/`unwind_to`）、未改任何在册引擎**（URS/v4/iso 零接触）。
   - 新增 `analysis/nested_interval_fugue_backtest.py` + `data_cache/nif_*.json`。
   - 7 个 Rust 单测（含 bit-exact 退化定理、min_trade_ladder gate 分离、出场对齐、
     守恒）；全套 342 Rust 测试通过。
