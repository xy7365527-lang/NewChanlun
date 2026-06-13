# 嵌套递归赋格严格会计体系

> 从买卖点操作的原子语义出发，设计覆盖所有情况的会计规则。
> 每条规则必须有守恒律守卫。违反守恒=bug。

## 1. 基本量

- **N**：建仓股数。建仓后恒定（26课"一开始就买够不加仓"）。只有earning阶段可增加。
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
assert father.shares + child.units == N  // 股数守恒
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
// 注意: child.P&L ≡ father.降成本金额（同一数字）
status: 关闭（或翻转为LONG——如果k-1级别新走势开始）
```

### 守恒律守卫
```
assert father.shares == N  // 回到满仓
assert child.P&L == father.cost_reduction  // 双视图一致
assert total_NAV变化 == child.P&L  // NAV只变化了子的盈亏
```

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
assert father.shares + child.units + grandchild.units == N
// 总股数在递归链上守恒
```

## 6. 不清仓原则

- 根voice在建仓后持有N股，**永不清零**——除非最高涌现级别走势完美（十年1-2次）
- 根voice在段级/L1/L2卖点时只做降成本（释放部分给子voice），不清仓
- "清仓"=N→0，只在以下条件同时满足时发生：
  1. 最高涌现级别（recL3/recL4）的confirmed背驰
  2. 背驰已被区间套递归确认
  3. 全链子voice级联清算后
- 清仓后=等新的最高级别买点→重新建仓（这是真正的"空仓gap"，但极少发生）

## 7. earning阶段

当降成本使cost_basis ≤ 0时：
```
cost_basis ≤ 0 → 仓位免费 → 后续降成本产生的现金=纯利润
```
纯利润可以用来增加N（挣股数）——但M=N（新增的股数也参与后续的同股数进出），这增加了递归的资本基础。

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

**多空对称**：earning对多空都适用。
- 多头voice降成本到cost≤0 → earning → 增加多头股数
- 空头voice降成本到cost≤0 → earning → 增加空头股数
空头的"降成本"=在下跌中做反弹短差→空头均价抬高→cost≡开空均价越来越有利→cost≤0（空头免费）→纯利润→增加空头仓位。
会计完全对称——方向是voice的属性，earning机制与方向无关。

## 8. 全局不变量（每个bar检查）

```
1. Σ(所有活跃voice的units) = N（根voice恒仓）
2. 同一笔物理交易只在ledger中出现一次（物理层），视图层按需导出
3. 子voice.P&L ≡ 父voice.cost_reduction（对任何已完成的短差周期）
4. total_NAV = initial_capital + Σ(已关闭voice的P&L) + 未实现P&L
5. 零强平（027:25否定线先于保证金线）
```

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
