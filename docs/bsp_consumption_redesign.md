# BSP 消费重构 —— 最简形式：N 个自相似级别引擎，各消费自己级别的买卖点

> 编排者北极星 2026-06-20：**买卖点没消费 → 多重赋格没实现 → 回补失败 → 一切问题的源头。**
> 信号层已验证（c段修复后 11281 个 BSP）。操作层只需消费这些 BSP，每级别自相似消费自己级别的买卖点、
> 多空双开 → 理论上达 **绩效=Σ\|涨跌幅\|**。所有中间机制（promote/spawn/flip/chain/extract_chain/
> emergent门控/clear_root/sink/recover/走势跟随）都是**不消费 BSP 而造的 workaround，消费 BSP 后全部消失**。
> 纯设计，未改代码。

## 0. 北极星与诊断

**根因链**：引擎消费 0/11281 个买卖点（rec_stream.rs:142 `all_bsps()` 算出后只灌诊断、:168 `on_view` 只吃走势链 ChainView 不传 BSP）→ 操作建立在走势方向（chain[0]）上 → 需要 promote/spawn/flip 间接"升格"推导操作 → ep13 核心做空 held 1.2M bar、843 个 type1_buy 被忽略、骑到 18109（−184%）。

**北极星**：T 算子是对的（信号层 11281 BSP）。操作层**只管消费 BSP**，每级别自相似、多空双开 → Σ\|涨跌幅\|。

## 1. 最简架构：N 个独立自相似级别引擎

### 1.1 信号层（不变，已验证）
`iterate` 产出 `levels[k].bsps`（全 6 类，11281 个，携 kind/level/price/bar，types.rs:223）。走势结构（中枢/走势/笔）是**信号层的事**——它产出 BSP，操作层不碰走势结构。

### 1.2 操作层（最简：每级别一个 T 实例直接消费本级别 BSP）
每个级别 k 一个 T 实例 `LevelEngine_k`（同一 T 算子的实例，仅级别不同 = 自相似），**独立持仓 + 独立响应**，只消费 `levels[k].bsps`：

| 本级别 BSP | LevelEngine_k 操作 |
|-----------|------|
| **type1_buy** | 持空→平空翻多；flat→做多。（下跌趋势背驰转折 = 该级别由空翻多）|
| **type1_sell** | 持多→平多翻空；flat→做空。（镜像）|
| **type2_buy** | 加多（一买后第一次回调底确认，017:60「第二有利位置」）|
| **type2_sell** | 加空（镜像）|
| **type3_buy** | 持有/加多（中枢上移确认，029:58「出现就继续持有否则抛出」；不成立=减仓）|
| **type3_sell** | 持空/加空（镜像）|

**操作跟 BSP 一一对应**：买点→做多/平空翻多，卖点→做空/平多翻空。无中间推导。

### 1.3 多重赋格 + 自相似 + 多空双开
- **多重赋格**：N 个 LevelEngine 同时运行，L0/L1/L2… 的 BSP 同时被各自级别消费 = 多声部。
- **自相似**：每个 LevelEngine 是同一 T 算子（type1/2/3 消费规则相同，仅级别不同）。
- **多空双开**：不同级别可同时多/空（L4 持多骑大趋势 ∧ L1 持空吃回调）。
- **总持仓** = Σ_k (d_k × units_k)。**短差自然涌现**：低级别高频翻转、高级别慢——不需显式 sink/recover（[[project_t_multiscale_independent_filters]] 已证「短差靠低尺度高频翻转涌现」）。
- **绩效=Σ\|涨跌幅\|**：每级别 type1 翻转吃本级别趋势、type2/3 加仓，多级别叠加 = 吃掉所有尺度的绝对涨跌。

## 2. promote/spawn/flip/chain/sink/recover —— 全部冗余，删除（回答编排者质疑）

消费 BSP 后，这些走势跟随残留**全部失去存在理由**：

| 机制 | 为何存在（走势跟随） | 消费 BSP 后为何冗余 |
|------|------|------|
| **extract_chain / ChainView / chain[0]** | 把走势树投影成一条"链"喂操作层 | 操作层每级别直接读 `levels[k].bsps`，不需要链 |
| **promote**（回调短头升格核心） | 不消费 BSP，需"升格"间接推导回补 | 低级别 type1_buy 直接翻多**本级别实例**，无核心可升格 |
| **spawn**（核心 relabel 上移） | 单核心需骑趋势上移 | 无单核心，每级别独立，无 relabel |
| **flip**（全树塌缩+反向 enter） | 单核心需整树翻转 | type1 直接翻**本级别**持仓，无全树清 |
| **sink/recover**（跨级短差） | 单核心需显式下放/归还短差 | 短差=独立级别持仓自然涌现，无显式跨级 |
| **emergent门控/clear_root** | 走势方向锚/退出观望 | 已删（−1069% 做空陷阱根源）|

**结论**：promote/spawn/flip/sink/recover/chain/extract_chain **不需要存在**——它们是"不消费 BSP"的 workaround。最简形式只有：**N 个 LevelEngine，各读本级别 BSP，type1 翻转/type2 加仓/type3 持有确认**。

## 3. 回补自动化修 ep13（无需 bottom-up 级联）

ep13 真凶 = promote 死等最高走势反转（1.2M bar）。**最简形式下回补是自动的**：下跌段的低级别 type1_buy 直接翻多**本级别 LevelEngine**（不等任何级别、不需级联确认）→ 843 个 type1_buy 立即被各自级别消费 → 各级别持仓随本级别 BSP 翻转 → **无 1.2M bar 套牢**。核心空头不再是"一个骑 1.2M bar 的巨仓"，而是"高级别 LevelEngine 的持仓"，它在高级别 type1_buy 出现时翻多；低级别 LevelEngine 早已在低级别 type1_buy 翻多吃了反弹。

## 4. 三模式 = BSP 过滤器，在最简引擎上测量（编排者补充 2）

模式是 `combine_modes`（divergence.rs:202）对 type1 的过滤：Structural（结构 5 条件，纯公理）/ AND（结构∧MACD 双确认）/ OR（任一）。最简引擎正确消费 BSP 后，三模式效果可真实测量：**哪种模式的 BSP 集合消费后产生正确操作循环**（8 标的回测）。模式只影响赚多赚少（哪些 BSP 算数），不影响能否正确操作（编排者裁决 3）。

## 5. AND 消除 ep5/ep7 假做空（编排者补充 3）

ep5/ep7 假做空源于裸走势方向触发的假反转。最简形式下**做空只由 type1_sell 触发**（不由走势方向）。AND 模式 type1_sell = 结构∧MACD 双确认——ep5 @4100 的假顶（牛市续涨到 ~19000）无真 MACD 背驰 → AND 不产 type1_sell@该级别 → 该级别不做空 → 不亏。可证伪 L2 预测（三模式对照）。

## 6. 一个不回避的边界：底分型 ≠ 一类买点（诚实上浮）

每段下跌必有几何底分型，但**一类买点是背驰点**（知识库:363），背驰需中枢（≥3 单元，center.rs:48 / divergence.rs:158）。本引擎 **a₀=confirmed&&Settled 线段**（backtest.rs:160）非笔/bar——段太短（<3 单元）无中枢 → 该级别真无 type1（ep6/9/10/11 的 0 买点）。「回调再短也有底」成立，「底就有一类买点」不严格——递归有 a₀ base case 硬底（第64课「线段只存在最低级别之下」）。**要消除此边界须把 a₀ 下沉到笔/分型层**（更深递归，独立改动，escalate）。但 029:396 保证：**只要段发育出中枢，本级别必有一类买点被本级别 LevelEngine 消费**。

## 7. 落码计划（每步 cargo + NT 验证，§9.8 binding：不假设闭合）

> 警示：[[project_t_multiscale_independent_filters]]（多尺度独立 LevelEngine）已落码过（commit 50f2235013），
> 但旧版只消费 type1 走势翻转（c段修复前）+ 旧核算，结果「机械翻转踏空 + 做空失血」L3 P1 2/8。
> 本次差异：c段修复（11281 全 6 类 BSP）+ 消费 type1/2/3 全类（非仅 type1 翻转）+ 方向对称三阶段。

| 步 | 改动 | 验证 |
|----|------|------|
| **P1** | 新 `LevelEngine`：持仓 + 消费本级别 6 类 BSP（type1 翻转/type2 加仓/type3 持有确认）。删 extract_chain/ChainView/promote/spawn/flip/sink/recover | cargo test：每 LevelEngine 单测 6 类→操作；守恒守卫 |
| **P2** | N 级别引擎并行：rec_stream 把 `levels[k].bsps` 直接派给 `LevelEngine_k`，删 on_view(ChainView) | cargo test 合成零 panic |
| **P3** | 资金/核算：根账本 free 共享 + 每级别独立 units/direction（自相似）；三阶段口径待裁（§8） | BTC：敞口分布（应不再 82% 持空）；ep13 应无 1.2M 套牢 |
| **P4** | 三模式对照 8 标的 | 哪种模式产生正确操作循环；ep5/ep7 AND 下消失；绩效 vs Σ\|涨跌幅\| |

## 8. 待裁（不可自决）

1. **三阶段口径冲突**：编排者此处「每个级别独立核算=自相似」vs 之前「三阶段是总体性（根 cost_basis/phase，非 per-instance）」。最简形式下两读法：(A) 每级别独立三阶段（自相似彻底）；(B) 持仓 per-level、三阶段总体（根 campaign）。须裁。
2. **§8.2 零操作参数 vs 裁决 1 BSP 消费**：「走势结构的结构性确认即门」（用走势 completed 代替 BSP）是架构级矛盾——本设计按裁决 1（消费 BSP）收敛，promote/spawn/flip 作为走势跟随残留删除。须 knowledge-crystallization 结晶「操作触发权威=BSP fire，非走势结构」。

## 9. 结果包六要素

- **结论**：操作层收敛到最简形式——N 个自相似 LevelEngine，各消费**自己级别**的 6 类 BSP（type1 翻转/type2 加仓/type3 持有确认），多空双开 = 多重赋格，总持仓=Σ，短差自然涌现。**promote/spawn/flip/sink/recover/chain/extract_chain/emergent/clear_root 全部是走势跟随 workaround，消费 BSP 后删除**。回补自动化（低级别 type1_buy 直接翻多本级别，修 ep13）。绩效=Σ\|涨跌幅\| 是 N 级别叠加吃所有尺度涨跌的理论上界。
- **定义依据**：知识库:363-369（6 类定义）；017:60/66/70（一买转折定律/二买第二有利/定律一）；029:52/58/396（一卖镜像/三买持有协议/归根结底都是一买）；065课 aₙ=f(aₙ₋₁)（自相似递归）；center.rs:48/divergence.rs:158（中枢≥3/背驰≥2 门槛=base case）。
- **边界条件（结论翻转）**：(a) 底分型≠一类买点，a₀=线段 base case，段太短无中枢真无买点（§6）；(b) [[project_t_multiscale_independent_filters]] 旧版「机械翻转踏空+做空失血」——本次差异（全 6 类 BSP + c段 + 对称三阶段）能否避免=L2/L3 待回测，不得假设闭合；(c) 三阶段口径（独立 vs 总体）待裁。
- **下游推论**：(1) 删除 rec_driver 的 extract_chain/ChainView/on_view + rec_engine 的 promote/spawn/flip/sink/recover；(2) 新 LevelEngine 直接消费 levels[k].bsps；(3) §12.4 第二不对称（三阶段多头独有→空头逐仓可强平）与 BSP 消费正交，须独立修；(4) 每步 NT 验证。
- **谱系引用**：[[project_t_multiscale_independent_filters]]（最简形式前身，commit 50f2235013，旧信号层）；[[project_recursive_t_architecture_v2]]（§8.2 零参数 vs 裁决1 = 架构级概念分离）；§9.8（promote+emergent −1069% binding L2）；[[project_t_operation_self_replication]]（ε对称 sink/recover——本次删除，由独立级别涌现替代）。
- **影响声明**：本文 = `docs/bsp_consumption_redesign.md`（收敛重写为最简形式），**未改任何代码**。落码 = 删 promote/spawn/flip/chain + 新 N 级别 LevelEngine 直接消费 BSP（P1-P4 每步验证）。这是从走势跟随到缠论 BSP 操作的根本重构 + 大幅简化。
- **认识论等级**：§0-§2 诊断/架构/冗余删除 = **L0**（原文 + 源码逐行）；§3 修 ep13/§4 三模式/§5 AND/§6 base case = **L0 设计预测，L2/L3 待回测**（§9.8 教训：不假设闭合）。
