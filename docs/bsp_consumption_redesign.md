# BSP 消费重构 —— 引擎从"走势结构跟随"重构为"消费三类买卖点"（缠论操作）

> 编排者裁决 2026-06-20。诊断铁证：递归引擎消费 **0** 个买卖点（11281 个全 6 类 BSP 产出后全丢弃），
> 操作建立在走势结构（chain[0] 几何方向 + 回调 completed）上 = **不是缠论操作**。本文设计把操作层重构为
> 消费三类买卖点。12-agent 工作流（原文溯源 + 三视角设计 + 三对抗验证）+ 编排者三补充 综合。**纯设计，未改代码。**

## 0. 诊断铁证与命题

### 0.1 引擎消费 0 个买卖点（架构级，非局部 bug）
- `rec_stream.rs:141-168` 逐字坐实：`tree.all_bsps()`（:142）算出全 6 类 BSP，只灌诊断计数器（:148-160），操作调用 `driver.on_view(&view,…)`（:168）**只吃 ChainView，完全不传 BSP**。
- `extract_chain`（rec_driver.rs:205-244）只投影 `levels[k].trends/units` 几何结构，**对 `levels[k].bsps` 一字节不碰**。`ChainNode = {node:TrendNode, completed}` 无 BSP 字段。
- BSP 产出层完全就绪（`TLevelOutput.bsps` types.rs:223 / `all_bsps()` :270）。**断裂点唯一在投影层 extract_chain + driver 消费判据**（建立在 TrendNode 几何方向上，rec_engine.rs:124「方向由所骑走势方向定，非 BSP 推断」）。
- **BSP 产出清单**：type1 buy2216/sell2927 · type2 buy378/sell408 · type3 buy2837/sell2515 = **11281 个全丢弃**。引擎实际操作 ~93 个（enter24/flip24/sink13/recover10/spawn19/promote3），全由走势结构驱动。

### 0.2 ep13 真凶
核心做空@6367 held **1.2M bar 穿越整牛市**，期间 **843 个 type1_buy（最低 4442 < 做空价 6367）全被忽略**，回补判据用走势 completed（rec_driver.rs:131）死等最高走势反转（1.2M bar）→ 骑到 18109 = **−184%**（−175% 全量的主因）。

### 0.3 编排者裁决（不接受 workaround）
1. 缠论所有操作基于三类买卖点 → **引擎必须消费三类买卖点**（§8.2「走势结构代替 BSP」是架构级矛盾，须 escalate 结晶）。
2. **多重赋格**：每级别 T 实例消费**自己级别**的 BSP，多声部同时产出+消费 + 父子协调，不是中央走势跟随器。
3. **三模式（Structural/AND/OR）的答案在 BSP 消费角度**：模式=BSP 过滤器；正确消费后才能真实测量。
4. ep5/ep7 假做空应被 AND 模式消除：裸走势方向触发假反转，AND 双确认 type1_sell 的假回调通不过 MACD。

## 1. 6 类 BSP → 操作映射（缠论操作语义，对抗已修正）

判据 = 三元组 **(BSP.kind, BSP.level vs core_level, BSP 方向 vs 核心持仓方向)**，替换现状单一判据 chain[0].direction（裸几何方向 rec_driver.rs:87）。

| BSP | 核心级别（level==core_level） | 次级别（level < core_level） | 原文 |
|-----|------|------|------|
| **一买 Type1Buy** | 核心持空→**reverse 整仓翻多**（promote/flip，方向门=**背驰确认非裸走势**，修 ep5/ep13）；空仓→enter 建多 | 反核心向（持空遇）→**bottom-up 回补候选**（§3）；同核心向→无 | 017:60/70 转折定律 |
| **一卖 Type1Sell** | 核心持多→reverse 翻空（镜像一买） | 持多遇（反核心向）→**sink 减1/3 下放短差**（rec_engine:444，降成本来源） | 029:52 镜像 |
| **二买 Type2Buy** | 同核心向→**加仓入场**（一买后第一次回调底=「第二有利位置」017:60，**新建/加仓暴露，非 recover**） | 次级别加仓确认 | 017:60/66 定律一 |
| **二卖 Type2Sell** | 镜像二买（空头核心加仓） | 镜像 | — |
| **三买 Type3Buy** | **hold-confirm**：中枢上移成立=保持/加多（029:58「出现就继续持有否则抛出」）；不成立（回抽破 ZG）=反向减仓 | 次级别中枢突破确认 | 029:58；operator.rs:55 已检测 |
| **三卖 Type3Sell** | 镜像三买（持空确认/不成立则减仓） | 镜像 | — |

**关键修正（对抗）**：
- **二买 ≠ recover**：二买是次级别第一类买点的**独立入场点**（次优建仓），与「recover=归还 sunk 配额」（由回调完成/底背驰触发）**概念分开**（no-patch：声明与实际一致）。
- **sink（减仓）来源**：不由六类中任一类独占，而由「**与核心持仓方向相反的次级别买卖点**」触发（持多遇次级别卖点 sink / 持空遇次级别买点→回补候选）。
- **recover（归还短差）触发**：回调走势完成（底背驰/顶背驰）——与二买独立。

## 2. 多重赋格：每级别 T 实例消费自己级别的 BSP（编排者补充 1）

route_bsp 按 **level** 路由 = 天然多声部：`view.bsps` 中每个 BSP 按 `level` 派给该级别的 T 实例操作——
- L0/L1/L2… 的 BSP **同时**在产出+消费（多声部赋格），各 T 实例消费**本级别** BSP；
- 核心级别 BSP → 核心反转/加仓；次级别 BSP → sink/recover/回补候选；
- 父子协调 = 区间套（高级别由低级别逐级确认，§3）。

**这修正当前 bug 的根**：现状只看 chain[0]（一个中央信号）= 单声部走势跟随；重构为每级别独立消费 = 真多重赋格（与 §1「每级别一 T 实例」一致）。

## 3. bottom-up 回补修 ep13（核心持空时低级别 type1_buy 逐级 promote）

### 3.1 阶梯（无数值参数，对抗已修正判据）
核心持空时，连续低级别 type1_buy（view.bsps 中 kind==Type1Buy，反核心向）按**区间套逐级向上确认** → promote 多头核心：
- **触发判据 = 第33课:48「反弹是否重回中枢」**（非 price 单调，对抗修正）：低级别 type1_buy 后的反弹若**重回（突破）上方最近中枢的 ZD/ZG**（operator.rs type3 的「回抽不破 ZG/ZD」检测已实装）= 可能转折，逐级向上传递；若**不重回中枢** = 纯回调 → recover 吃短差降成本，核心空头不动。
- **级联完整到核心级别**（连续级别都重回中枢）→ promote 翻多。缺一级=链断=不 promote。**阈值无参数靠级别连续性自身**（§8.2 零操作参数）。
- 843 个 type1_buy 大部分被消费为**有界次级别短差降成本**（方向对称 account_core_reduce 已支持 rec_engine.rs:361）；只有级联完整的才 promote 翻多 → 核心空头在 4442 附近就回补而非骑到 18109。

### 3.2 promote 缺口（须扩展，§11.6-1）
当前 promote（rec_engine.rs:597）硬前置 = root 须有「现成回调子 T」。回补侧反弹长子 T 罕见（§11.4 无 return address）→ 退化 clear_root。**须补**：promote 能「用频繁低级别 type1_buy **construct** 反弹子 T 再升格」，非仅升格现成子 T。
**且废弃 emergent_top 锚**（跨年滞后致 −1069% 做空陷阱，§9.8 binding L2）。

## 4. 三模式 = BSP 过滤器，在完整引擎上测量（编排者补充 2）

三模式是 `judge_divergence` 的 `combine_modes`（divergence.rs:202）对 type1 的过滤：
- **Structural**：只结构 5 条件（37课，纯缠论公理）→ BSP 多。
- **AND**：结构 ∧ MACD 双确认 → 少而可靠。
- **OR**：结构 ∨ MACD → 多但假信号多。

「哪种理论上严格」的答案**在 BSP 消费角度**：模式决定哪些 type1 被产出（过滤强度）；一旦操作层正确消费 BSP，三模式的效果 = **哪种模式的 BSP 消费后产生正确操作循环**（在完整 BSP 消费引擎上 8 标的回测，非简化 backtest 比收益）。模式只影响**赚多赚少**（哪些 BSP 算数），不影响「能否正确操作」（操作循环结构正确性，编排者裁决 3）。

## 5. AND 消除 ep5/ep7 假做空（编排者补充 3）

ep5/ep7 做空 = chain[0] 裸走势方向翻 down 触发的**假反转**（牛市途中回调，价格续涨）。重构后核心反转由 **type1_sell（背驰确认）** 触发而非裸走势方向：
- **AND 模式**要求 type1_sell = 结构 ∧ MACD 双确认。ep5 @4100 的假顶（牛市续涨到 ~19000）**无真 MACD 背驰动量** → AND 模式不产 type1_sell@核心 → **不做空** → 不亏。
- 这是可证伪 L2 预测：BSP 消费引擎 AND 模式下 ep5/ep7 的假做空消失。（Structural 模式可能仍产假 type1_sell，故三模式对照可测量。）

## 6. 结构太短 → 低级别检测：底分型 ≠ 一类买点（诚实上浮，非 workaround）

编排者命题「回调再短也有底，底就有买点只是级别低」**半真半伪**（不回避）：
- **「回调再短也有底」成立**：每段下跌必有几何底分型。
- **「底就有买点」不严格**：一类买点是**背驰点**（知识库:363），背驰需中枢；中枢需 ≥3 单元（center.rs:48 `n<3 返回空`；divergence.rs:158 趋势背驰 n_centers≥2）。段内无中枢 → 底只是**分型（候选转折点）**，不是任何级别的一类买点。
- **递归有 base case 硬底**（第33课:182「最小级别中枢用三根 K 线」；第17课「分不是无限的」）。本引擎 **a₀=confirmed&&Settled 线段**（backtest.rs:160/stream.rs:59）非笔/bar——一根下跌线段内部 1-2 单元则是 a₀ 单元，引擎**无法在其内部检测买点**（笔/段封装在 T₀ 构造器内对 T 不可见）。
- **ep6/9/10/11（0 买点）归因**：真没有引擎可表示级别的买点（段太短无中枢），非「高级别没有低级别有但没投影」。

**结论**：低级别买点的**有效域 = 该段发育出 ≥1 中枢**。这不是 regime workaround，是 a₀ base case 的结构边界（第64课「线段只存在最低级别之下」）。**若要消除此边界 → 须把 a₀ 下沉到笔/分型层**（更深递归，独立改动，escalate）。但 029:396「所有买点归根结底都是第一类买点，要找第二三类都要下次级别找第一类」保证：**只要段发育出中枢，回补必能递归到低级别一买**，不死等最高走势反转。

## 7. 消费接口（最小改动面，对抗已修正：route_bsp 单一触发权威）

### 数据结构（rec_driver.rs）
```rust
pub struct ChainBsp { pub kind: BSPKind, pub level: usize, pub price: f64, pub bar: i64 }
pub struct ChainView {
    pub nodes: Vec<ChainNode>,   // 不变：走势结构=级别上下文 + sink 载体 TrendNode
    pub bsps:  Vec<ChainBsp>,    // 新增：消费对象（全来自 types::BSP 已有字段）
    pub core_level: usize,       // 新增：= levels.len()-1，路由判 level vs core_level
}
```

### 触发权威单一化（对抗修正）
- `extract_chain` 保留 trend 链投影（级别上下文 + sink 的 callback_node 载体）+ **新增遍历 `levels[k].bsps` 注入 view.bsps**。
- **`reconcile_chain` 的 sink/recover 操作触发逻辑删除**——降级为**纯级别上下文/链拓扑投影器**（只维护 ChainNode 级别身份 + 供 route_bsp 取 callback_node TrendNode，**不调用任何 rec_engine 算子**）。
- **`route_bsp(view,c,bar)` 是唯一 τ 操作触发权威**：遍历 view.bsps 按 (kind, level vs core_level, 方向) 分派（§1 映射）。操作不再由「走势 completed」触发，只由 BSP 触发（裁决 1）。
- 复用现有算子 enter/sink/recover/promote/flip（无新算子，promote 须扩展 §3.2）。

## 8. 落码计划（每步 NT/cargo 验证，§9.8 binding：不得假设闭合）

> **前置约束**：promote+emergent 落码后 BTC −1069%（做空陷阱）。每步必 cargo test（守恒守卫逐操作 panic）+ BTC 回测对照敞口分布，不重复 NRF v5 错误。

| 步 | 改动 | 验证 |
|----|------|------|
| **P1** | extract_chain 携 BSP（ChainView+ChainBsp+core_level，投影 levels[k].bsps） | cargo test rec_driver（33 测试不破）+ 新测「投影 BSP 非空」 |
| **P2** | rec_stream 接线（new_bsps 灌 view 非仅诊断） | cargo test rec_stream（合成零 panic） |
| **P3** | route_bsp 6 类路由；核心反转判据 chain[0].direction→core_level type1 BSP fire；reconcile_chain 降级为上下文 | cargo test + BTC 对照 chain[0] vs 核心方向分布（应不再 82% 持空） |
| **P4** | bottom-up 回补（低级别 type1_buy 重回中枢级联 promote）+ promote construct 扩展 | BTC：ep13 核心空头应在 4442 附近回补；敞口分布；NAV |
| **P5** | 三模式对照（Structural/AND/OR）8 标的回测 | 测量哪种模式 BSP 消费产生正确操作循环；ep5/ep7 AND 下消失 |

## 9. 结果包六要素

- **结论**：引擎消费 0/11281 个买卖点（走势结构驱动），重构为消费三类买卖点：6 类→操作三元判据（一买/卖=reverse 背驰确认门 / 二买/卖=加仓入场 / 三买/卖=hold-confirm；sink=反核心向次级别；recover=回调完成）；多重赋格（route_bsp 按 level 多声部）；bottom-up 回补（低级别 type1_buy 重回中枢级联 promote 修 ep13）；三模式=BSP 过滤器（完整引擎测量）；AND 消除 ep5/ep7 假做空；接口=ChainView 携 BSP + route_bsp 单一触发权威。
- **定义依据**：知识库:363-369（三类定义）；017:60/66/70（一买转折定律/二买第二有利位置/定律一）；029:52/58/396（一卖镜像/三买持有协议/归根结底都是一买）；033:48（重回中枢=转折/回调判据）；017:22（任何走势类型终要完成）；center.rs:48/divergence.rs:158（中枢≥3/背驰≥2 门槛）。
- **边界条件（结论翻转）**：(a) 低级别买点有效域=该段发育出 ≥1 中枢；a₀=线段 base case，段太短无中枢则真无买点（要消除须下沉 a₀ 到笔层，escalate）；(b) bottom-up 级联若过密致核心 churn 恶化收益，加级别门只影响赚多赚少不影响正确操作；(c) 能否实修 ep13/收益=L2 待回测，不得假设闭合（§9.8）。
- **下游推论**：(1) reconcile_chain 的 sink/recover 触发删除，降级上下文；(2) promote 扩展 construct 反弹子 T；(3) 废弃 emergent_top + 走势 completed 触发；(4) §12.4 第二不对称（三阶段多头独有→空头永远逐仓可强平，短核心等 recover 期被强平致 no-op）与 BSP 消费**正交，须独立修**（空头三阶段对称已部分落码 commit 14b840a0）。
- **谱系引用**：[[project_recursive_t_architecture_v2]]（三失效点 + §8.2 零操作参数 vs 裁决1 BSP 消费=**架构级概念分离，须 escalate/结晶**）；§9.8（promote+emergent −1069% binding L2）；§11.4/§12（做空/回补不对称 base case）；[[project_unn_btc_spawn_throwback]]（E spawn 强牛过度同构）；docs/bsp_engine_sublevel_fix.md（递归层 BSP 暴露先例）。
- **影响声明**：本文 = `docs/bsp_consumption_redesign.md`，12-agent 工作流 + 三对抗 + 编排者三补充综合，**未改任何代码**。落码含义：rec_driver ChainView 携 BSP + route_bsp 单一触发 + bottom-up 回补 + promote construct 扩展（P1-P5，每步 NT 验证）。这是引擎从走势跟随到缠论 BSP 操作的根本重构。
- **影响声明（认识论等级）**：§0 诊断/§1 映射/§6 base case 边界/§7 接口 = **L0**（原文 + 源码逐行）；§3 修 ep13/§4 三模式效果/§5 AND 消除 ep5 = **L0 设计预测，L2/L3 待回测**（不得声称闭合，§9.8 教训）。

## 10. 须 escalate（不可自决）

**架构级概念分离**：§8.2「走势结构的结构性确认即门，零操作参数」（故意用走势 completed 代替 BSP）vs 编排者裁决 1「操作基于三类买卖点」——是**架构级矛盾，非局部 bug**。「走势方向 vs BSP 消费」是否已有谱系条目不确定。**建议 knowledge-crystallization 或 /escalate 裁定结晶**：缠论操作的触发权威是「走势结构完成」还是「BSP fire」？（本文按裁决 1 = BSP fire 设计，但与 §8.2 零参数原则的关系须正式结晶。）
