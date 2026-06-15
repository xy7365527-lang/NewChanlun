# 嵌套递归赋格严格会计体系

> 从买卖点操作的原子语义出发，设计覆盖所有情况的会计规则。
> 每条规则必须有守恒律守卫。违反守恒=bug。

## 1. 基本量

- **N_base**：根voice的运行态在手单位聚合。建仓时 N_base = 建仓股数。**"恒定"读作"不主动加仓"**
  （26课"一开始就买够不加仓"的语义），**非"数量不可变"**：N_base 双向重定基——earning 阶段
  N_base += Δ（挣股数），亏损回补阶段 N_base −= shortfall（该 shortfall 单位永久缩水，但 N_base
  总量可经后续 earning 回升——双向，非单调）。详见 §11.2（DIVERGES）。
  - **符号约定**：下文 §2-§8 中裸符号 **N** 为 N_base 的简写（指代当前运行态基准值）；**N′** 表示
    重定基后的新 N_base 值；**M** 为单次操作股数，26课同股数进出语义下 M = N_base。守恒律与全局
    不变量（§2/§5/§8.1）显式写 N_base；局部操作流（如 `N → N-m`）用简写 N。
- **voice**：递归赋格的一个声部。每个voice有独立的级别、方向、生命周期。
- **根voice**：最高涌现级别的voice。建仓=根voice诞生。只有根voice走势完美时才清仓。

## 2. 操作原子——卖出

### 触发条件
某级别k出现confirmed卖点（或区间套递归定位的卖点），且该级别voice持多。

### 物理交易
卖出m股（m由35课成本门决定——振幅足以覆盖摩擦的最大可操作量）。

### 双层会计（同一笔物理交易）

**父voice（级别k）账本更新**：
```
shares: N → N-m
释放资金: cash_released = m × current_price
状态: 待回补
cost_basis: 不变（降成本在回补时计算）
```

**子voice（级别k-1）账本创建**：
```
direction: SHORT
capital: cash_released = m × current_price
units: m
entry_price: current_price
status: 活跃
```

### 守恒律守卫
```
assert father.shares + child.units == N_base  // 股数守恒（记账两侧一致，见 §8.1）
assert total_NAV不变  // 一笔交易不改变总价值
```

## 3. 操作原子——买回（回补）

### 触发条件
级别k-1出现confirmed买点（或区间套定位），子voice持空。

### 物理交易
买入m股。

### 双层会计（同一笔物理交易）

**父voice（级别k）账本更新**：
```
shares: N-m → N
降成本金额: profit = (sell_price - buy_price) × m
new_cost_basis: old_cost_basis - profit / N  // 均价降低
状态: 持有（非待回补）
```

**子voice（级别k-1）账本结算**：
```
P&L: (entry_price - exit_price) × m  // 空头盈亏
// 注意: child.P&L ≡ father.降成本金额 仅在 {盈利 ∧ shortfall=0 ∧ cost_pool足} 时成立。
//       一般情形 child.P&L 分裂为四去向（cost_reduce + earning_excess − shortfall_loss + free沉淀）。详见 §11.1。
status: 关闭（或翻转为LONG——如果k-1级别新走势开始）
```

### 守恒律守卫
```
assert father.shares == N_base  // 回到满仓（N_base=当前运行态基准，非建仓常数，见 §1/§11.2）
// 下式为「条件恒等」——仅 {盈利 ∧ 满额回补 ∧ cost_pool足} 时成立：
assert child.P&L == father.cost_reduction  // 双视图一致（条件，见 §11.1）
// 无条件成立的是「记账两侧一致」（防单边记账bug）；一般情形按四去向分裂核验。
assert total_NAV变化 == child.P&L  // NAV只变化了子的盈亏
```
> **双重性的时间结构**（§11.1）："双视图一致"仅在 voice **关闭时刻**成立；存活期父子各持
> 不完整视图（父 cost_pool 未降、子 capital 冻结）。

## 4. 子voice翻转（非回补，而是子级别走势完美→新走势）

### 触发条件
子voice（级别k-1）的走势完美。新走势方向与旧方向相反。

### 物理交易
与回补相同——买入m股（平空）。但同时开新方向=父voice回补完成+子voice翻转。

### 双层会计
```
// 父voice: 降成本完成（同回补）
father.shares: N-m → N
father.cost_basis降低

// 子voice: 旧方向关闭 + 新方向可能开启
child_old: 关闭，P&L结算
child_new: 如果k-1级别新走势需要操作
           → 等到k-1的新走势出现卖/买点 → 新的spawn
```

注意：子voice翻转不等于"立即开新方向"——要等新走势的买卖点。但在a0层面买卖点首尾相连，所以这个等待很短。

### voice升级（走势涌现）——会计结构重组，N_base不变

当更高级别走势涌现（递归层级增加，如 recL2→recL3），根voice的**身份级别上移**：原根voice
成为新根voice的子结构。这是**会计结构的重组**（voice森林新增一层、归属层重新锚定），**不是
单位数量的变化**：

```
voice拓扑: root(k) → root(k+1) ⊃ {child(k) = 原root}
单位守恒:  N_base 不变（重组只改 voice 的级别标签与父子关系，不增减在手单位）
```

升级与 §1 的 N_base 重定基**正交**：N_base 只随 earning（+Δ）/ 亏损回补（−δ）改变（物理交易
驱动），voice升级不触发物理交易 ⇒ N_base 守恒。Σ(所有活跃voice.units) 在重组前后逐位相等。
代码层面对应归属层（涌现级别）的动态锚定（参 §12 改动3"root 可多 child"森林结构）。

> **来源标注**：本段为 A5 新增的结构性声明，依据 §12 森林结构与 A5 条目，**非 §11 审计直接覆盖项**
> （§11 的 D1-D6 未对 voice 升级作判决）。性质为对会计结构在级别涌现下行为的补充说明。

## 5. 递归嵌套——子voice做降成本

子voice（级别k-1，持空）在运行期间，如果k-2级别出现买点（反弹）：

### 物理交易
子voice"卖出"m2的空头（=买回m2股）→释放给孙voice。

### 三层会计
```
// 父voice(k): 仍待回补（不受影响）
// 子voice(k-1): 
//   持空 m → m-m2（释放m2给孙）
//   待回补：等孙voice完成后回到m
// 孙voice(k-2):
//   direction: LONG（在反弹中做多=对子voice降空头成本）
//   capital: m2 × current_price
//   units: m2
```

### 守恒律
```
assert father.shares + child.units + grandchild.units == N_base
// 总股数在递归链上守恒（记账两侧一致，见 §8.1）
```

## 6. 不清仓原则

- 根voice在建仓后持有 N_base 股，**不主动清零**（N_base 可因亏损回补缩水，但不主动归零——
  "不清零"约束的是主动归零行为，非 N_base 数量不变）——除非最高涌现级别走势完美（十年1-2次）
- 根voice在段级/L1/L2卖点时只做降成本（释放部分给子voice），不清仓
- "清仓"=N_base→0（主动全平），只在以下条件同时满足时发生：
  1. 最高涌现级别（recL3/recL4）的confirmed背驰
  2. 背驰已被区间套递归确认
  3. 全链子voice级联清算后
- 清仓后=等新的最高级别买点→重新建仓（这是真正的"空仓gap"，但极少发生）

## 7. earning阶段

当降成本使cost_basis ≤ 0时：
```
cost_basis ≤ 0 → 仓位免费 → 后续降成本产生的现金=纯利润
```
纯利润可以用来增加 N_base（挣股数）——同股数进出原则下 M = N_base（新增的股数也参与后续的同股数进出），这增加了递归的资本基础。

### earning增仓的精确会计

**条件**：cost_basis ≤ 0（成本已降到零或负）

**增仓量**：Δ = earning纯利润P / 当前价格

**增仓时机**：必须在买点（有买卖点依据）。earning不改变"操作只在买卖点"原则。

**增仓后**：
```
N' = N + Δ
根voice.shares: N → N'
后续所有"同股数进出"以N'为基准
守恒律更新: Σ(所有活跃voice.units) = N'
```

**增仓的资金来源**：降成本产生的纯现金（cost≤0后的盈余），不是外部注入。

**增仓的递归效应**：N'更大→每次降成本释放的m可以更大→子voice资本更充裕→递归深层嵌套更有力。这是正反馈循环——成本越低，操作能力越强。

**多空对称（分层）**：earning 的对称性**分两层，不可笼统说"完全对称"**（§11.3 GAP）：

- **金额守恒 / 极性翻转层——对称成立**：earning 产生的纯现金、cost_basis 的极性翻转
  （多头 cost 下穿 0 / 空头 cost 上穿开空均价）在守恒律层完全镜像。
- **资本化构造层——不对称**：
  - 多头父 earning：`parent.units += dq`（增多头股数，**可构造**）。
  - 空头父 earning：**L0 构造性不可表示**——"挣股数"需要持负数量股（空头均价 ≤0 后继续盈利），
    线性载体无法表示。代码仅记 `nrf_short_earning_hits += 1`（计数），**不增 units**。
  - 解除条件：换**凸性载体**（期权，参 `project_put_option_short_earning`）。

> 即：方向是 voice 的属性，earning 的**守恒/极性机制**与方向无关（对称）；但 earning 的
> **资本化（增仓）构造**依赖载体凸性，线性载体下空头侧撞 L0 墙（不对称）。详见 §11.3。

## 8. 全局不变量（每个bar检查）

```
1. Σ(所有活跃voice的units) = N_base（记账两侧一致）
2. 同一笔物理交易只在ledger中出现一次（物理层），视图层按需导出
3. 子voice.P&L ≡ 父voice.cost_reduction（条件恒等：仅 盈利∧满额回补∧cost_pool足）
4. total_NAV = initial_capital + Σ(已关闭voice的P&L) + 未实现P&L（多头逐市；空头递延，见下）
5. 零强平（027:25否定线先于保证金线）
```

**§8.1 守卫语义澄清（A6 / §11.2）**：不变量 1 的守卫验的是**"记账两侧一致"**（每笔物理交易
在父子两个账本的单位增减相抵，防单边记账 bug），**不是"N=建仓常数恒仓"**。N_base 是运行态
在手单位聚合（earning +Δ / 亏损 −δ 双向重定基，见 §1/§11.2），守卫随 N_base 同步更新；
违反 1 = 单边记账 bug，**不**= "仓位不再等于建仓股数"。

**§8.3 恒等式有效域（A3 / §11.1）**：不变量 3 是**条件恒等**，仅
{盈利 ∧ shortfall=0（满额回补）∧ cost_pool 足} 时成立。一般情形 child.P&L 分裂为四去向
（cost_pool_reduce + earning_excess − shortfall_loss + free沉淀）。无条件成立的只有"记账两侧一致"。

**§8.4 NAV 未实现 P&L 的方向不对称（A8 / §11.4）**：不变量 4 的"未实现 P&L"**仅多头逐市**
（mark-to-market）。**空头取冻结 capital（非逐市）**，未实现 (basis − c)×units **递延到回补时
结算**。逐 bar 误差**双向**：空头浮盈→equity 低估；空头浮亏→equity 高估；界 =
`(−Σu×basis, Σu×basis]`，受 1x 逐仓强平守卫单边上封。终态 final_nav 正确（全回补后双通道归零）——
GAP 仅是逐 bar 口径，**nav 的 capital 形式是物理单真值**（空头持现金，无独立 MtM 负债）。

## 9. 与当前代码的差异

当前NRF v2的C规则是"根完美→全链清算"——每次根卖点就清仓。这摧毁了递归嵌套。

修改：C规则只在最高涌现级别走势完美时触发。其他卖点全部走E规则（spawn子voice降成本）。

## 10. 物理执行映射

回测中：一笔物理交易=一个price×units。双层记账在内存中。

实盘中（Hyperliquid）：
- 逐仓模式（isolated margin）
- 每个voice level可以对应一个独立的逐仓position
- 或者用净额+内部记账（如果交易所不支持同标的多仓位）
- Hyperliquid支持同标的多个逐仓position吗？需要确认

## 11. 审计修正（2026-06-13，有效域标注）

> 来源：`analysis/accounting_duality_audit.md`（会计双重性审计，L0，6 维度对抗核验）。
> 性质：**代码（`nested_fugue.rs`）正确，本规格 §1-§8 的四条无条件声明是 26课理想化口径
> 未覆盖逆境分支 = 声明膨胀（090号）。修规格向代码看齐，不修代码。** 原 §1-§10 保留
> （谱系），下表标注有效域。审计判决：D1 存一次/D6 递归守恒 = CONFORMS（双重性核心正确）；
> D2/D4/D5 = GAP；D3 = DIVERGES。

### 11.1 §8.3 恒等式 child.P&L ≡ father.cost_reduction —— GAP

无条件恒等**仅在 {盈利 ∧ shortfall=0（满额回补）∧ cost_pool 足}成立**。一般情形 child.P&L
分裂为**四去向**（唯一物理量 leftover = capital − u_back×c）：

```
child.P&L ≡ cost_pool_reduce（降成本）+ earning_excess（N增,池竭后）− shortfall_loss（亏损,N减）+ free沉淀
```

- 盈利+池竭：超额走 earning（N 重定基），不进 cost_reduction。
- 亏损（c>basis）：leftover=0 ⇒ cost_reduction=0；亏损物化为 **shortfall 单位永久缩水**
  （`n_base −= shortfall`），**不是负 cost_reduction**（cost_pool 单调不增）。

**双重性的时间结构**：§8.3"双视图一致"仅在 voice **关闭时刻**成立；存活期父子各持**不完整**
视图（父 cost_pool 未降、子 capital 冻结）。

### 11.2 §1 N 恒定 —— DIVERGES（N_base 双向重定基）

§1"N 建仓后恒定，只 earning 增"与代码 `n_base −= shortfall`（亏损回补 N **减**，line 220）
冲突。**N_base ≡ 运行态在手单位聚合**（双向重定基：earning N+Δ / 亏损 N−δ，代码 doc line 12-13
"镜像"），**非建仓常数**。§1"恒定"应读作"不主动加仓"（26课语义），非"数量不可变"。
§8.1 守恒守卫验的是"记账两侧一致"（防单边记账 bug），**不**验"N=建仓常数恒仓"。

### 11.3 §7 earning 多空对称 —— GAP（构造不对称）

§7"多空完全对称"= **金额守恒/极性翻转对称 ∧ 资本化构造不对称**：
- 多头父 earning：`parent.units += dq`（增股，可构造）。
- 空头父 earning：**L0 构造性不可表示**（"挣负股数"——空头均价 ≤0 后继续盈利需持负数量股）。
  代码仅 `nrf_short_earning_hits += 1`（计数），不增 units（line 247-259）。
- 解除条件：换**凸性载体**（期权，`project_put_option_short_earning`）。

### 11.4 §8.4 NAV 未实现 P&L —— GAP（方向不对称递延）

§8.4"每 bar 含未实现 P&L"仅多头逐市；空头取**冻结 capital**（非逐市），未实现 (basis−c)×units
**递延到回补**。误差**双向**：空头浮盈→equity 低估；空头浮亏→equity 高估。界 = `(−Σu×basis,
Σu×basis]`，受 1x 逐仓强平守卫单边上封，回补时双通道（leftover 盈/shrink 亏）归零。终态
final_nav 正确（全回补后）；GAP 是逐 bar 口径。**nav 的 capital 形式是物理单真值**（空头持
现金无独立 MtM 负债）——修复方向是改本 §8.4 措辞，非改 nav()。

### 11.5 对分阶段推进（Phase 2-5）的下游约束

- **Phase 3（逐仓嵌套/双层记账）**：D1+D6 CONFORMS ⇒ 地基稳固，可建。
- **Phase 5（earning）**：D4 ⇒ 空头 earning 线性载体撞 L0 墙。按代码现实（仅多头父增仓）或换期权载体。
- **Phase 2（多空对称）**："一套逻辑多空镜像"成立于守恒/极性层（D1/D6），**不**成立于 earning/N
  重定基层（D3/D4）。须分层：守恒层对称 ∧ earning 层构造不对称。

## 12. Phase 3 实装记录（2026-06-14，逐仓声部森林）

> §11.5 预告"Phase 3 逐仓嵌套 D1+D6 CONFORMS ⇒ 地基稳固，可建"。已建：
> `rust/src/trading/isolated_fugue.rs`（mode = "iso"）。

**实装范围**：把 v4(`nested_fugue.rs`)/URS(`unified_recursive.rs`) 的 voice **栈**
（每级别至多一个 voice、链尾操作、全局 `acted` 互斥）升级为 voice **森林**：

1. **per-voice acted**（`acted_bar`，去全局互斥）——同 bar 多 level/多 voice 独立操作。
2. **Type2 接入区间套窗口**（`e.class.side()` 归侧，删 `Sell2|Buy2 => continue`）——
   中枢回测确认词汇（概念链第12环）经 located/nf 触发 spawn。
3. **VoiceLedger 森林**（root 可多 child，`child_voice_ids: Vec`）——会计原语
   （`close_voice` 短/多分支）逐字复用 `pop_tail`（§11 审计 CONFORMS），只从链尾
   访问改为 id 寻址 + 树**后序**遍历。孤儿不可能定理：父仅经后序 `close_voice`
   关闭 ⇒ 子必先关 ⇒ 无孤儿 ⇒ 无需 reparent。
4. **逐 level 独立 BSP 消费**（改动1 推论）。

清仓（§6）= URS E* 涌现归属层 sell_any ∧ 递归确认 ⇒ cascade 全树回现金。

**验证（认识论等级标注）**：
- L0/L1：335 单测全过（含 12 个 iso 单测：森林多 child、同 bar 多 voice 回补、
  三层守恒、Type2 武装、earning 重定基）。会计正确性独立 code-review = PASS
  （守恒 §8.1 / 孤儿不可能 / NAV 连续 / per-voice 互斥全 VERIFIED）。
- **L3**（8 标的真实数据 ~25M bars）：**零守恒违反**（§8.1 守卫全程未 Err）⇒ 森林
  会计在真实数据上鲁棒。**P1（≥BH）= 1/8**（仅 OKLO）；**iso ≻ urs 6/8**
  （BTC +549pp，清仓回现金 > URS 翻转 churn）；**iso ≺ v4 5/8**（v4 仍最强基座）。
  报告：`analysis/isolated_fugue_forest_verdict.md`。
- **否定性发现**：用户前提"没吃到信号 → 森林吃更多 → 更赚"**未被证实**——iso 仅
  3/8 比 urs 多吃信号（R_capture），且多吃的 ES/GC 既不跑赢 BH 也不跑赢 v4。
  **信号捕获与 alpha 正交**——"12 信号丢失机制"被修 ≠ alpha（栈纪律是特征非 bug；
  参 §11 + `project_shared_position_fugue` L2 否证）。
