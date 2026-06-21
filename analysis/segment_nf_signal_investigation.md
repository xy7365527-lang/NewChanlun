# segment 级别（ladder=2）nf 信号缺失调研

> 任务：调研为什么 segment 级别（ladder=2）不产生 nf 信号，以及如何让它产生。
> 范围：**只调研，不改信号代码**。
> 日期：2026-06-17
> 引擎：`rust/src/trading/unified_necessity.rs`（unn 统一必然性引擎）
> 信号层：`analysis/organic_signals.py` / `analysis/fugue_version_i.py`

---

## 0. 结论摘要（TL;DR）

1. **`nf_sell[2]/nf_buy[2]` 恒为 `None`，是 L0 编译期确定的**——nf 填充循环
   `unified_necessity.rs:1161` 下界是 `PENDING_LO = 3`，**结构性跳过 k=2**。不需要跑数据，
   这是循环边界常量决定的。

2. **这不是 bug，是三层叠加的设计必然**：
   - **机械层（L0）**：`PENDING_LO = FIRST_BSP_LADDER + 1 = 3`，nf 循环、nest 武装、
     helix confirm 全部从 PENDING_LO 起步。
   - **概念层（L0）**：segment 被定义为「非势源」——第14环「高级别 BSP 由低级别定位」中，
     segment 是被向心回溯的**内圈结构基底证据**，不是发起势的外圈。
   - **经验层（L2，已验证）**：`project_pcf_pending_locate_collapse_fix` 谱系——
     segment 抢先武装 ⇒ located 链顶 source 坍缩 **85–91%**。

3. **更深的概念障碍**：即使机械下放循环边界，segment 的 `helix_centripetal_confirm`
   会**退化为恒真**（`for j in (2..2)` 空循环 → `return true`）——因为 segment 的次级别是
   笔（bi）/K线（bar），这两层无中枢、无 type1 BSP，向心回溯没有真实内圈 type1 可贯通。
   退化的 nf ≈ raw candidate，正是 pcf 坍缩的根因。**让 segment 产生「合法的 φ=0 走势完美 nf」
   在缠论上不可达**（笔不是走势，107号「笔不裁决」）。

4. **命名陷阱（务必先澄清）**：代码里 `LADDER_SEG = 2`，但它的**语义是「笔中枢」
   （bi_zhongshu）不是「线段 segment」**。见 §1。

---

## 1. 级别命名澄清（关键前置）

`analysis/fugue_version_i.py:144-151`：

```python
LADDER_BAR, LADDER_BI, LADDER_SEG, LADDER_MOVE = 0, 1, 2, 3
FIRST_BSP_LADDER = LADDER_SEG  # ladder≥2 = 中枢承载层（segment=笔中枢，走势+=线段/递归中枢）
```

| ladder | 变量名 | **真实语义** | BSP 来源 | 中枢? |
|--------|--------|-------------|---------|------|
| 0 | `LADDER_BAR` | K 线 | PH proxy（close 树） | ❌ 无中枢（单 K 不是走势） |
| 1 | `LADDER_BI` | 笔（bi） | PH proxy（stroke 端点树） | ❌ 无中枢（单笔不是走势） |
| **2** | **`LADDER_SEG`** | **笔中枢（bi_zhongshu，`BI_ZHONGSHU_LEVEL_ID=1`）** | 真实 BSP（525号下放） | ✅ type1/2/3 |
| 3 | `LADDER_MOVE` | 走势(L1) | 真实 BSP（trend snapshot） | ✅ type1/2/3 |
| L+2 | 递归层 | 递归 L(≥2) | 真实 BSP（适配器） | ✅ type1/2/3 |

**`LADDER_SEG=2` 的事件实际来自 `orch.take_bi_zhongshu_bsp_events(BI_ZHONGSHU_LEVEL_ID)`**
（`organic_signals.py:321`），即「笔中枢」层，不是「线段」层。

历史（`fugue_version_i.py:65`）：
> ★本次：FIRST_BSP_LADDER 从 3 下放到 2——525号笔中枢（三笔重叠→笔中枢→笔级别走势→type1/2/3）
> 给 segment 级一个真实中枢承载层，故 entry_level 可下探到 segment。

所以 ladder2 是「曾经 FIRST_BSP_LADDER=3，525号下放到 2」后新增的最低中枢承载层。
`PENDING_LO` 是 `FIRST_BSP_LADDER + 1`，所以 `FIRST_BSP_LADDER` 从 3→2 时，`PENDING_LO`
从 4→3——**始终保持「pending 势源比最低中枢承载层高一级」的不变量**。

> 下文沿用代码用词「segment」指 ladder2（=笔中枢），与缠论「线段」无关，避免歧义时标注「笔中枢」。

---

## 2. nf 信号是什么 / 怎么产生（数据流）

### 2.1 nf 的定义

`unified_necessity.rs:39`：
> **逐层 nf fire（N7）**：`nf_sell[k]/nf_buy[k]` = confirm@k 的本层极值。

nf = 某层 candidate 经 **`helix_centripetal_confirm`（T49 向心回溯）** 确认为「走势完美（φ=0）」
后兑现的极值价。它是「第11环买卖点的严格形式」——**不是** raw candidate（`sig.*_any`），
必须经区间套向心确认。

### 2.2 nf 的产生循环（`unified_necessity.rs:1156-1232`）

```rust
let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
let mut nf_buy:  [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
...
for k in PENDING_LO..MAX_LADDER {          // ← line 1161：下界 PENDING_LO=3，跳过 k=2
    // ① 破极值否定（nest_sell/nest_buy[k]）
    // ② candidate 武装 / confirmed 清窗（nest_*[k]）
    // ③ helix 向心 confirm：
    if let Some(w) = self.nest_sell[k] {
        let helix = helix_centripetal_confirm(&self.type1_hist, k, Side::Sell, w.since_bar);
        let confirmed = w.since_bar < bar && helix;
        if confirmed {
            confirm_sell[k] = Some((w.extreme, w.since_bar)); // → located 级联（N5，供 C/F）
            nf_sell[k] = Some(w.extreme);                     // → 自层 fire（N7，供 E/D）
            ...
        }
    }
    // buy 侧对称
}
```

**`nf_*[k]` 仅在 `k ∈ [PENDING_LO, MAX_LADDER) = [3, 11)` 内可能被写入。
k=2（segment）从不进入这个循环 ⇒ `nf_sell[2]/nf_buy[2]` 永远是初值 `None`。**

### 2.3 nf 的两个消费者（同一 fire 两路）

`unified_necessity.rs:32-45`（header §32）："同一 confirm@k fire 分两路"：

| 路 | 数组 | 消费者 | 必然性 | 级别要求 |
|----|------|--------|--------|---------|
| 级联 located 链 | `confirm_*[k]` → `cascade_arm` → `located_*` | **C**（清仓/翻转）/ **F**（入场） | N5/N6 | source ≥ PENDING_LO（硬断言） |
| 自层 nf fire | `nf_*[k]` | **E**（降成本 spawn）/ **D**（回补） | N7 | trigger 层 == voice 层 |

- **E 段**（`:1500-1503`）：`nf_sell[ladder].is_some()`（多头）/ `nf_buy[ladder].is_some()`（空头）
- **D 段**（`:1457-1460`）：`nf_buy[v.ladder]`（空头回补）/ `nf_sell[v.ladder]`（多头回补）

因此 **ladder2 的 voice 无法触发 E/D**——其自层 nf 恒 None。注释 `:588-592` 已显式声明这一点：
> `nf_*[FIRST_BSP_LADDER]` 恒 None：segment 非势源，无角向圈/无向心 confirm，T56。故 segment 层
> voice 自层 counter fire 恒 false——E 不 spawn（sub=bi θ=None 本就终止）、D 不独立回补
> （只经父 cascade close 或 A 强平离场）。**这是递归基的自然终止边界，非缺陷。**

### 2.4 关键非对称：segment 是「被回溯的内圈」而非「发起的外圈」

`unified_necessity.rs:1239-1251`（type1 历史记录）：

```rust
for (j, hist_j) in self.type1_hist.iter_mut().enumerate()
        .take(MAX_LADDER).skip(FIRST_BSP_LADDER) {   // ← skip(2)：从 segment 起记录
    for e in &evs[j] {
        if e.class.kind() == BspKind::Type1 { hist_j[si].push(bar); }
    }
}
```

**segment（ladder2）的 type1 BSP 被记录到 `type1_hist[2]`**，供 k≥3 层的 `helix_centripetal_confirm`
向心回溯使用（`:299` `for j in (FIRST_BSP_LADDER..k).rev()`，当 k=3 时 j=2）。

> segment 已经作为「内圈结构基底证据」深度参与系统——move(ladder3) 的 nf 确认，
> 其向心回溯的最内圈恰是 segment 的 type1。
> **segment 是被回溯者（内圈），不是发起者（外圈）。** 这是设计的核心对称性。

---

## 3. 为什么 segment 不产生 nf（四层原因，逐层加深）

### 3.1 机械层（L0）：循环边界常量

`unified_necessity.rs:203`：
```rust
const PENDING_LO: usize = FIRST_BSP_LADDER + 1;   // = 3
```
nf 填充循环（`:1161`）、located 级联（`:1258`）、结合律重 fold（`:553`）全部以 `PENDING_LO` 为下界。
nest 武装数组 `nest_sell/nest_buy` 的 k=2 槽位永远不被触碰。**编译期即确定 `nf_*[2]≡None`。**

### 3.2 概念层（L0）：segment 非「势源」（N5/N6 第14环）

`unified_necessity.rs:49-51`：
> **pending 势源**（N5/N6）：仅 `k ≥ PENDING_LO = FIRST_BSP_LADDER+1 = move(L1)` 注册 pending
> （segment 非势源……segment 仅作 `helix_centripetal_confirm` 向心回溯的**结构基底证据**）。

缠论依据（`缠论知识库.md` §7.3）：级别 = 递归层级，**自上而下**确立标准中枢压制多义性。
势从高级别走势发起，由低级别走势 confirm（区间套逐级收缩定位）。segment（笔中枢）是
「次级别走势」的承载基底，它**定位**更高级别的势，自己不是势的来源。

硬断言守卫（多处 panic guard）：
- `cascade_arm:244`：`debug_assert!(source >= PENDING_LO)`
- `prove_chain:449-452`：`assert!(s >= PENDING_LO, "N5 违反……segment 非势源")`
- `prove_n5_cascade:528`：`assert!(e.source_ladder >= PENDING_LO ……segment 武装为势源)`

→ 任何把 segment 作为 located 链 source 的尝试都会**直接 panic**。

### 3.3 经验层（L2，已验证）：pcf 坍缩谱系

记忆/谱系 `project_pcf_pending_locate_collapse_fix`（已结算）：
> 坍缩根因 = segment 瞬时 nf[2] 抢先武装 source=2；重构（先势后定位）：pending 只在 k≥move(L1)=3
> 注册……坍缩 91-96%→0% 结构性（1min 三标的 + 1s 双粒度 CL1MO seg%=0.0）。

即：**历史上 segment 确实能产生 nf 并武装 source**，结果是 located 链顶（操作级别）被 segment
永久占据 **85–96%**，体现不出「势的级别」——操作全坍缩到最低层。这是 L2 真实数据的否定性结果，
不是推测。当前的 `PENDING_LO=3` 正是这次坍缩修复的产物。

### 3.4 概念层（L0，最深）：segment 的向心 confirm 退化为恒真

假设强行让 segment 进入 confirm 循环。`helix_centripetal_confirm`（`:287-309`）：

```rust
fn helix_centripetal_confirm(type1_hist, k, side, since) -> bool {
    let mut upper = since;
    for j in (FIRST_BSP_LADDER..k).rev() {   // k=2 时 → (2..2) 空循环
        match latest_le(&type1_hist[j][si], upper) {
            Some(b) => upper = b,
            None => return false,
        }
    }
    true   // ← k=2：空循环直接到这里 = 恒真（只要 since<bar）
}
```

**当 k = FIRST_BSP_LADDER = 2，内圈范围 `(2..2)` 为空，函数无条件 `return true`。**

含义：segment 的「走势完美」**没有任何内圈 type1 母线需要贯通**——因为它的次级别是笔（ladder1）
和 K 线（ladder0），这两层**无中枢、无 type1**（`fugue_version_i.py:68`「单笔不是走势 → 无 type1/2/3」）。

于是 segment 的 nf 退化为「任意 candidate + since<bar 后续走势」≈ **raw candidate**。
这正是 §3.3 pcf 坍缩的机制根因，也违反引擎对 nf 的核心声明（`:586-588`）：
> `nf_*` = `helix_centripetal_confirm` 向心确认（第11环买卖点严格形式 = φ=0 走势完美），
> **非 raw** `sig.*_any`（任意 candidate……≠ φ=0）。

**缠论闭环**（`缠论知识库.md` §10/§6.1）：三类买卖点都是「与该级别最近的走势中枢的关系」，
而中枢由「至少三个连续**次级别走势类型**重叠」构成。segment=笔中枢的次级别是笔，
**笔不是走势**（107号「笔不裁决」）⇒ segment 没有合法的「次级别走势 type1」可供区间套向心确认。
**「让 segment 产生合法的 φ=0 走势完美 nf」在缠论定义上不可达。**

---

## 4. 如何让 segment 产生 nf（方案 / 代价 / 障碍）

> ⚠ 三个方案都涉及改信号代码，**本任务范围外**。此处仅列出方案空间、各自的障碍与认识论等级，
> 供后续决断（这是一个「选择/语法记录」类决断，需 `/escalate`，见 §6）。

### 方案 A：机械下放（循环边界 `PENDING_LO → FIRST_BSP_LADDER`）

把 `:1161` 循环下界、`:1258` 级联下界、`:553` 重 fold 下界改到 `FIRST_BSP_LADDER`。

| 维度 | 结果 |
|------|------|
| nf_*[2] | 可填充，但 helix 退化恒真（§3.4）⇒ ≈ raw |
| located 级联 | confirm_*[2] 进 cascade ⇒ `prove_chain`/`prove_n5_cascade` 的 `s≥PENDING_LO` 硬断言 **panic** |
| 经验 | pcf 坍缩 85–96% 复现（L2 已否证） |
| 认识论 | L2 否定性结果已存在 ⇒ **不应采纳** |

**否决**：直接撞 panic guard + 已被 L2 否证。

### 方案 B：双轨分离（nf 供 E/D 下放，located 供 C/F 不下放）

- nest 武装 + helix confirm 循环下界 → `FIRST_BSP_LADDER`（segment 产生 `nf_*[2]`）
- **但** `cascade_arm`（located，`:1258`）保持 `PENDING_LO..` 下界（`confirm_*[2]` **不进** located）
- located 链顶仍 ≥3（不坍缩），`prove_n5_cascade` 的 segment 断言保留

| 障碍 | 说明 |
|------|------|
| helix 退化 | `nf_*[2]` 仍是退化恒真 = raw candidate（§3.4）⇒ 违反「nf = φ=0 走势完美，非 raw」声明（`no-patch-mentality.md` 声明膨胀） |
| E 无子可生 | segment voice 的 E spawn 目标是 sub=bi（ladder1），`theta(bi)=None`（depth_ref 只观测 `[FIRST_BSP, MAX)`）⇒ `try_spawn_cost_gated` `noref_reject` ⇒ E 必失败。**即使 nf_*[2] 填充，segment 的 E 也无目标层** |
| D 可回补但触发器是 raw | segment voice 的 D（close 返父）可触发，但触发条件 `nf_*[2]` 是退化 raw ⇒ 强牛 regime 下过早回补（pcf/throwback 风险） |
| `prove_self_level_symmetric` | 该 prove（`:616`）守 D/E 内联 gate == `self_level_counter_fire` 规范。规范读 `nf_*[ladder]`——若 nf_*[2] 填充则规范在 segment 也非 None，prove 不 panic（一致下放）。但这等于把「退化 raw」固化为「合法走势完美」，是**声明膨胀的形式化掩盖** |

**结论**：方案 B 可实装且不撞 located 硬断言，但产出的 `nf_*[2]` 是**退化信号**（helix 恒真 = raw），
E 无子可生、D 触发器不可靠。**收益存疑，且需先验证 L2/L3 才能采纳**（当前是 L0 推断为负）。

### 方案 C：概念正确的下放（先给 segment 真实内圈）

要让 `nf_*[2]` 是合法 φ=0，需 segment 有真实「次级别走势 type1」内圈可向心贯通。
segment 的次级别是笔——需把 bi（ladder1）也升级为中枢承载层（`FIRST_BSP_LADDER → 1`）。

| 障碍 | 说明 |
|------|------|
| 缠论否决 | 「单笔不是走势」（`知识库 §7.3`/§5.1），107号「笔不裁决」禁笔作走势组件 ⇒ bi 无中枢无 type1 |
| 递归基 | bi/bar 是递归终端（PH proxy），不是中枢承载层。下放到 1 = 把 PH proxy 伪装成 type1 BSP = 假信号 |

**否决**：违反缠论递归基定义。segment 的向心内圈天然缺失，**这是递归基的自然边界**（`:592`），不是缺陷。

### 方案对比小结

| 方案 | 撞 panic? | 缠论合法? | nf 质量 | 认识论 | 裁决 |
|------|----------|----------|---------|--------|------|
| A 机械下放 | ✅ 撞 | ❌ | raw | L2 已否证 | 否决 |
| B 双轨分离 | ❌ 不撞 | ⚠ 退化 | 退化 raw | L0 推断负，需 L2/L3 | 存疑，需 escalate |
| C 真实内圈 | ❌ | ❌ | — | L0 缠论否决 | 否决 |

---

## 5. 当前架构下 segment 层的「正确」角色

segment（ladder2 笔中枢）在 unn 引擎里**已经有明确且充分的角色**，不需要 nf：

1. **入场层（F）**：`entry_level` 可下探到 segment（525号下放的目的）——根 voice 可以**开在** segment 层
   （located 链顶 source 最低到 ladder3=move，但 cascade 武装 `[FIRST_BSP..=source]` 覆盖 segment，
   `:248` `skip(FIRST_BSP_LADDER)`）。
2. **内圈确认证据（type1_hist[2]）**：segment 的 type1 是 move(ladder3) 向心 confirm 的最内圈母线（§2.4）。
3. **递归终止边界**：segment voice 的 E（sub=bi θ=None）自然终止，无需 floor 参数（N4，`:53-54`）。

segment **不发起 nf** 与它**承载入场 + 作内圈证据**并不矛盾——这正是「区间套自上而下」的体现：
低级别**定位/确认**高级别的势，高级别**发起**势。segment 在「定位/确认」侧满载，在「发起」侧留空，
是设计的对称性，不是功能缺失。

---

## 6. 结果包（六要素，完整版——涉及概念定义）

### 6.1 结论
`nf_sell[2]/nf_buy[2]` 恒为 `None` 是 **L0 编译期确定**的（`unified_necessity.rs:1161` 循环下界
`PENDING_LO=3` 结构性跳过 k=2）。这不是 bug，是「segment 非势源」（N5/N6 第14环）的实现，
有 L2 经验支撑（pcf 坍缩 85–96%）。让 segment 产生**合法**的 φ=0 走势完美 nf 在缠论上不可达
（segment 次级别是笔，笔不是走势，无内圈 type1 可向心贯通，helix 退化恒真 = raw）。

### 6.2 定义依据
- 引擎：`unified_necessity.rs:203`（`PENDING_LO`）、`:1161`（nf 循环）、`:287-309`（helix 退化）、
  `:49-54`（势源 vs 降成本目标级别语义）、`:588-592`（segment nf 恒 None 声明）。
- 缠论：`知识库 §6.1`（中枢=三次级别走势重叠）、`§7.3`（级别=递归层级，自上而下确立）、
  `§10`（三类买卖点=与中枢关系）、`§8`（走势分解定理：至少三段次级别走势）。
- 命名：`fugue_version_i.py:144-151`（ladder2=笔中枢，非线段）。

### 6.3 边界条件（结论何时翻转）
- 若 `helix_centripetal_confirm` 对 k=FIRST_BSP_LADDER 的退化（空循环恒真）被裁决为**合法的
  走势完美**（而非 raw）——则方案 B 的 `nf_*[2]` 即为合法 nf，结论「不可达」翻转为「可达但退化」。
  这是一个待裁决的**语法记录类决断**（segment 的「无内圈区间套」是否算确认）。
- 若 525号被进一步推翻、bi 升级为中枢承载层——则 segment 获得真实内圈，结论翻转（但违反 107号）。

### 6.4 下游推论
- 任何依赖「ladder2 自层降成本」的策略（segment voice 的 E/D）在当前架构下**不可能触发**——
  这解释了 unn 回测中 segment 层 voice 只能经父 cascade close 或 A 强平离场。
- `n_nest_fire_sell_by_ladder[2] = n_nest_fire_buy_by_ladder[2] = 0`（L0 必然，无需跑数据验证）。
- 若未来要让 segment 参与降成本，正确路径不是「给 segment 加 nf」，而是「让 segment voice 经
  **父层（move/ladder3）的 located 级联**被动 close」——这已是现状。

### 6.5 谱系引用
- `project_pcf_pending_locate_collapse_fix`（已结算）：segment 抢先武装 ⇒ source 坍缩 85–96%→0%
  的结构性修复，是 `PENDING_LO=3` 的直接来源。**本调研确认该谱系仍然支配当前设计。**
- `project_pcf_1s_a0_source_collapse`（已被上条重构消除）：坍缩现象的首次观测。
- 525号谱系（`fugue_version_i.py:65,70-71`）：笔中枢下放 FIRST_BSP_LADDER 3→2。
- 107号「笔不裁决」：禁笔作走势组件——segment 内圈缺失的缠论根据。
- N5/N7 张力（`unified_necessity.rs:32-45`）：「同一 confirm 两路消费」——nf 的双消费者结构。

### 6.6 影响声明
- **本调研产出**：新增报告 `analysis/segment_nf_signal_investigation.md`。**未改动任何信号代码**
  （`unified_necessity.rs` / `signal*` / `organic_signals.py` 零修改）。
- **关联的仓库改动**（独立任务）：已 `git revert 3e49d20`（fugue_v3 recover 级别锚定修正），
  新 commit `cbdad96702`，恢复 fugue_v3 到 5cbb00e 行为——该 revert 不触碰 trading/unn 引擎，
  与本调研基线正交。
- **开放轴（需 escalate，非本任务决断）**：方案 B「segment 退化 nf 是否合法」是一个语法记录类
  决断（segment 的空区间套确认是否算走势完美）。当前 L0 推断其收益为负（E 无子、D 触发器是 raw），
  但未经 L2/L3 验证。

---

## 附录：认识论等级标注（formalization-validity-domain 规则）

| 论断 | 等级 | 依据 |
|------|------|------|
| `nf_*[2]≡None` | **L0** | 循环边界常量 `PENDING_LO=3`，编译期确定 |
| segment helix 退化恒真 | **L0** | `(2..2)` 空循环 return true，代码结构 |
| segment 无合法内圈 type1 | **L0** | 缠论定义（笔不是走势）+ ladder0/1 无中枢 |
| pcf 坍缩 85–96% | **L2** | 真实数据（1min 三标的 + 1s 双粒度 CL），谱系已结算 |
| 方案 B 收益为负 | **L0 推断** | E 无子（θ(bi)=None）+ D 触发 raw；**未经 L2/L3 验证** |
| 方案 B 是否值得实装 | **未验证** | 需 L2/L3 真实数据假设检验 |
