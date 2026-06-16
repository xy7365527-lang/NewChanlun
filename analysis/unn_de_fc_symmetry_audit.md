# unn 触发条件对称性审计（F/C/D/E/A 五规则 × located/raw × 多空对称）

> 审计人：本 session（2026-06-16）
> 触发：用户任务 2——"用覆盖空间 58 个不变量逐个扫 `unified_necessity.rs` 找类似 bug：
> 任何 E/D 不对称、F/C 不对称、located/raw 混用"。
> 范围：`rust/src/trading/unified_necessity.rs` 的 `UnnStreamCore::step` 五规则触发条件。
> 认识论：审计本身 L0（结构对称性，从代码读出）；BTC 证据 L3（4.625M bar，2017–2026）。

---

## 0. 三个触发标准（审计的坐标系）

引擎里同时存在三种"买卖点判据"，对称性的实质 = 哪条规则用哪个标准：

| 标准 | 定义 | 严格性 | 层覆盖 |
|------|------|--------|--------|
| **located 级联链** | `chain_source(located_*)` = 区间套自上而下 `[FIRST_BSP..=S]` 统一 source（cascade_arm，由 confirm helix 武装）+ `prove_chain` | 最强（N5/N6 + helix + 时序） | `[PENDING_LO, MAX)` |
| **nf 自层 fire** | `nf_*[ladder]` = 自层 candidate 经 `helix_centripetal_confirm` 向心母线贯通 + `since<bar` | 强（helix 确认，N7 自层，不要求级联链） | `[PENDING_LO, MAX)`（segment 恒 None） |
| **raw 掩码** | `sig.buy_any/sell_any/buy1/sell1.get(k)` = 该层有买卖点信号位（含未 helix 确认的 candidate） | 弱（任意信号，无 helix） | 全层 `[0, MAX)` |

**第11环"买卖点严格形式"+ 编排者 2026-06-16 裁决"E 用向心确认 nf"** ⇒ located/nf 是合法
买卖点判据，**raw 是非严格判据**（除非作为 located 链顶的 type1 *类别判别符*——见 §2 C/F）。

---

## 1. 五规则触发条件逐行表（修复后状态）

| 规则 | 代码行 | 触发判据 | 标准 | 多空对称 |
|------|--------|---------|------|---------|
| **F 入场** | 1472/1477 | `buy_source=S` ∧ `sig.buy1.get(S)` ∧ `prove_chain` | located 链 + type1 类别判别 | 单侧（只建多——T14 删根恒多，空头经 C 翻转涌现） |
| **C 翻转(多→空)** | 1305 | `sell_source=S≥root_ladder` ∧ `sig.sell1.get(S)` ∧ `prove_chain` | located 链 + type1 | ✓ 与 C(空→多) 镜像 |
| **C 翻转(空→多)** | 1353 | `buy_source=S≥root_ladder` ∧ `sig.buy1.get(S)` ∧ `prove_chain` | located 链 + type1 | ✓ |
| **D 回补(空子)** | 1402 | `nf_buy[ladder].is_some()` | **nf**（修复后） | ✓ 与 D(多子) 镜像 |
| **D 回补(多子)** | 1401 | `nf_sell[ladder].is_some()` | **nf**（修复后） | ✓ |
| **E spawn(非根)** | 1442 | `self_level_counter_fire(dir,ladder)` = nf 自层反向 | **nf** | ✓ 多空共用同函数 |
| **E spawn(根多, confirmed_root)** | 1446 | `is_root ∧ sig.sell_any.get(ladder)` | **⚠ raw** | ✗ **无空侧对偶**（根空头 T8 leaf 跳过）|
| **A 强平** | 1200 | `capital + units·(basis−c) ≤ 0` | 会计终局（被动） | 单侧（只空头——1x 现金多头无强平，T36 earning 不对称） |

---

## 2. 已确认对称（非 bug）

- **C/F 用 `sig.*1.get(S)` 非 raw 泄漏**：`S = chain_source(located_*)`，已被 located 级联链
  （helix 武装 + `prove_chain` 硬断言 `source≥PENDING_LO` + 时序 + 统一 source）门控。
  `sig.*1` 在此**仅作 type1 vs type2/3 类别判别符**（§6/§9：C=type1，E=type2/3），不是
  "是否有信号"的 raw 入口。C-long↔C-short↔F 三者同构 ✓。
- **A 强平短侧专属**：1x 逐仓多头 `capital≡0` 不可能 `≤0`，结构上无多头强平（T36 earning
  不对称 / N8 守恒）。这是**正确的多空不对称**（定理内容），非缺陷。
- **E 根空头跳过（T8 leaf）**：根空头用 MtM-external 会计，子空头用 frozen-internal，同
  voice 不兼容两套口径 ⇒ 根空头不嵌套降成本（买点走 C 翻多）。文档化的有效域边界，非缺陷。
- **C(根)/D(子) 用不同标准（located 链 vs nf）**：C 是同级别根操作（第14环区间套 N5/N6
  须 located 链），D 是子级别子操作（N7 自层不需链）。N5/N7 二分**设计核心**，非不对称 bug。

---

## 3. ⚠ 核心发现：`confirmed_root` 的 located/raw 混用（E 内部不对称）

### 3.1 结构

E 有**两条** spawn 路径：
- **非根 + 根的 nf 路径**：`nf_trigger = self_level_counter_fire(dir,ladder)` = nf（helix）。
- **根多头额外路径**：`confirmed_root = is_root ∧ sig.sell_any.get(ladder)` = **raw**。

即根多头降成本可经 nf_sell（helix）**或** raw sell_any（任意卖点信号）触发。raw 路径
**未经 helix 确认**——这是与 D 修复同类的 located/raw 混用，只是发生在 **E 内部**（非根 nf
vs 根 raw）。`n7_sublevel_sell_spawns_cost_reduction` 测试依赖此 raw 路径（无内层 type1
卖历史 ⇒ nf_sell 不 fire ⇒ spawn 只能来自 confirmed_root）。

### 3.2 修复前后的对称性变迁（关键）

| 路径 | 修复前标准 | 修复后标准 |
|------|-----------|-----------|
| E 非根 spawn | nf | nf |
| **E 根 spawn (confirmed_root)** | **raw** | **raw（未动）** |
| **D 子回补** | **raw** | **nf（task 1 改）** |

**修复前 D(raw) ↔ confirmed_root(raw) 是对称的**（都 raw，spawn 与 recover 同频）。
**修复后** D=nf 但 confirmed_root 仍 raw ⇒ 产生新不对称：**根 spawn 频繁(raw) + 子 recover
稀少(nf)** ⇒ 子空头**只进不出** ⇒ 堆积到 `c≥2×basis` 强平。

### 3.3 L3 经验证据（BTC 4.625M bar，2017–2026）

| 指标 | 修复前 D=raw | 修复后 D=nf | Δ |
|------|------|------|---|
| strat% | **+340.9** | +136.8 | **−204.1pp** |
| MDD% | −70.5 | −76.5 | −6.0pp |
| 强平数 | **0** | **216** | **+216** |
| 子空头净亏 | **−54,322** | **−194,697** | **−140,375** |
| 多头腿数 | 43 | **1** | −42 |
| exit_reasons | recover 317 / cascade 16 | **liq 216** / recover 74 | — |

两个签名：
1. **强平爆炸 0→216**：nf-gating 让 D 回补稀少（helix 向心母线贯通罕见），子空头无法在
   回调点平仓 ⇒ 价格上行至 2×basis 强平。子空头净亏 3.6× 恶化。
2. **多头腿 43→1**：修复前 D(raw buy_any) 与 E-short-spawn(nf_buy) 触发**不同** ⇒ 空子可
   spawn 多孙（42 个多头孙节点降成本）。修复后 D 与 E-short-spawn **同触发 nf_buy** ⇒ D
   优先级 shadow 掉 E-short-spawn ⇒ 多孙路径消失（见 §4）。

### 3.4 判定

`confirmed_root` 的 raw 不是孤立可改的实现细节——它的标准（raw vs nf）是**§9"根的其他卖点
走 E"的语义裁决**（"买卖点"= raw 任意信号 还是 helix 确认？），且与编排者"E 用 nf"裁决的
**适用范围**（是否含根路径）直接冲突，并有测试编码 raw 行为。**这是 no-workaround 的真矛盾，
不可自决** ⇒ 上浮 `.chanlun/escalations/2026-06-16-confirmed-root-raw-asymmetry.md`。

---

## 4. 次级发现：D 修复 shadow E-short-spawn（多孙降成本路径）

修复后 D(空子回补)=`nf_buy[ladder]` 与 E(空子 spawn 多孙)=`self_level_counter_fire(Short,ladder)`
=`nf_buy[ladder]` **同触发**。每 bar 优先序 D 在 E 之前 ⇒ 空子在 nf_buy fire 时被 D 回补
（close），永不走到 E spawn 多孙 ⇒ **E-short-spawn 分支在修复后成为 dead path**。

- 修复前两者触发不同（raw vs nf）⇒ E-short-spawn 可达（42 多头孙节点，BTC）。
- 修复后同触发 ⇒ D 优先 ⇒ 0 多头孙节点。

这是对称化的**结构副作用**：把"空子降成本(spawn 多孙)"和"空子回补(close 返父)"绑到同一
nf_buy 信号上，二者语义冲突（同一买点：该 spawn 多孙降成本 还是 该 cover 回补？）。优先序
裁定 D 赢——概念上"买点走势完美 ⇒ 下跌段完成 ⇒ 空子该 cover"是对的，故 D 优先正确，但
E-short-spawn 沦为死代码 ⇒ 应**删除**（no-patch：死分支不保留）或并入上浮裁决。

---

## 5. 审计结论

| # | 发现 | 类别 | 处理 |
|---|------|------|------|
| 1 | D 回补 raw→nf（task 1） | located/raw 混用 | ✅ 已修（prove_d_symmetric 守卫） |
| 2 | `confirmed_root` 根 spawn 仍 raw | located/raw 混用（E 内部） | ⚠ **上浮**（§3，L3 证据） |
| 3 | E-short-spawn 被 D shadow | 对称化副作用 | 并入上浮（§4，删死分支 vs 保留待裁决） |
| 4 | segment 层 nf 恒 None | 递归基边界 | 文档化（self_level_counter_fire 注释），非缺陷 |
| 5 | A 短侧 / C/F located+type1 / 根空头 leaf | 正确的多空不对称 | 非 bug |

**总判定**：task 1 的 D 修复在**结构对称性（L0）上正确**（D 与 E-nf 路径同标准，prove 守卫），
但在 **L3（BTC 强牛 regime）上否定性**——因为它只对齐了 E 的 *一条* 路径（nf_trigger），
未对齐 *另一条*（confirmed_root raw），反而暴露了 confirmed_root 的 raw 残余（发现 2）。
完整对称化需裁决 confirmed_root 标准（上浮）。

回测是有效域读数，非验收标准（编排者裁决）：AFTER 跑通 8 prove 零 panic ⇒ 必然性 L2 仍
成立，D 修复**被接受**；L3 否定性是 regime 有效域边界（formalization-validity-domain.md：
否定性结果缩小有效域，比确认性结果更有价值）。
