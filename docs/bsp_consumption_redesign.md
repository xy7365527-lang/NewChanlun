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

**BSP→操作是分层路由**（编排者 2026-06-20，非扁平）：操作取决于 BSP 级别相对核心级别：

| BSP 级别 | type1_sell | type1_buy | 机制 |
|---------|-----------|-----------|------|
| **本级别**（== core） | 平多翻空（整仓 Z₂ 翻转） | 平空翻多（整仓 Z₂ 翻转） | reverse + 联动次级别 |
| **次级别**（core−1） | sink：reduce 父 1/3 + 同股数开空 | recover：平空 + 同股数回补父 1/3 | 齿轮配对 |
| **次次级别**（core−2…a₀） | 更小短差（1/9…，递归下去） | 更小回补 | 递归齿轮 |
| type2/type3（各级别） | 加仓/持有确认（中枢上移/破位，029:58） | 镜像 | — |

### 1.3 齿轮耦合：每级别一个 Z₂ 振荡器，相邻级别反向咬合
（[[project_t_multiscale_independent_filters]] / [[project_spiral_engine_v2_bitexact]] D∞ 群结晶）
- **每级别 = Z₂ 振荡器**（多↔空两态）。每个 T 实例维护自己的 Z₂ 状态（direction），**BSP 是驱动齿轮转动的力**（type1 翻转 Z₂）。
- **相邻级别反向咬合**：本级别翻转 → 通过 sink/recover **驱动次级别反向操作**（父级齿轮转 → 驱动子级齿轮）。
- **齿轮间耦合 = sink/recover 配对**（同股数 = 能量守恒）：
  - 父级 reduce 1/3 → 子级开空 m（父转 → 驱动子转），rec_engine.rs:444。
  - 子级平空 m → 父级 recover 1/3（子转回 → 联动父转），rec_engine.rs:495。
  - 同股数进出（M=N）保证能量守恒（TW 中性）。
- **触发器换 BSP**：齿轮转动由 BSP（type1）驱动，不由走势 completed（本次重构的唯一改动——齿轮结构本身保留）。

### 1.4 滤波器：每级别一个带通滤波器，提取自己频率的 |涨跌幅|
- **每级别 = 带通滤波器**：高级别提取低频大趋势的 |涨跌幅|，低级别提取高频回调的 |涨跌幅|。
- **总 P&L = Σ 各级别滤波器输出**。**落码须 per-level P&L 追踪**——验证每个级别独立贡献正收益。
- **多空双开**：不同级别可同时多/空（L4 持多骑大趋势 ∧ L1 持空吃回调）= 多声部赋格。
- **绩效=Σ\|涨跌幅\|** = N 级别带通滤波器并联吃掉所有尺度的绝对涨跌。

### 1.5 多重赋格的验收标准
**8 标的回测中，每个级别应独立贡献正的 |涨跌幅| 提取**（per-level P&L > 0）。若某级别持续负贡献 → 该级别的 BSP 消费有问题（诊断信号，不是 regime 借口）。这是多重赋格"做对"的判据。

## 2. 删除走势跟随残留（保留齿轮结构）

消费 BSP 后，**走势跟随的间接推导机制**删除，但**齿轮结构（sink/recover + Z₂ 翻转）保留**：

| 机制 | 处置 | 理由 |
|------|------|------|
| **extract_chain / chain[0] 投影** | **删** | 操作层直接读 `levels[k].bsps` 分层路由，不需要"链" |
| **promote**（回调短头升格核心，零真空级联） | **删** | 回补 = 本级别 type1_buy 直接翻 Z₂ + 次级别 recover 联动，不需级联升格 |
| **chain[0].direction 触发 flip/spawn** | **删** | 翻转由本级别 type1 BSP 触发，不由裸走势方向 |
| **emergent 门控 / clear_root** | **删** | −1069% 做空陷阱根源；走势方向锚/退出观望都是 workaround |
| **spawn**（核心随 r\* 升级） | **保留/简化** | 核心级别=最高活跃级别，随塔生长自然上移（齿轮加顶层），但触发判据换 BSP |
| **sink / recover**（齿轮咬合短差） | **保留** | = §1.3 齿轮间耦合，触发器从走势 completed 换为次级别 type1 BSP |
| **Z₂ 翻转**（本级别整仓翻转，旧 flip） | **保留** | = §1.3 振荡器翻转，触发器换本级别 type1 BSP |

**结论**：删的是**走势跟随的间接触发**（chain/promote/emergent/clear_root），保留的是**齿轮+滤波器结构**（Z₂ 翻转 + sink/recover 咬合 + per-level 滤波）。唯一改动 = **触发器从走势结构换成 BSP**。

## 3. 回补自动化修 ep13（齿轮立即响应 BSP）

ep13 真凶 = 回补判据用走势 completed，死等最高走势反转（1.2M bar）。**齿轮换 BSP 触发后回补立即**：下跌段的次级别 type1_buy 立即驱动次级别齿轮 recover（平空回补），本级别 type1_buy 立即翻 Z₂——843 个 type1_buy 各自驱动对应级别齿轮，无 1.2M bar 套牢。核心空头在本级别 type1_buy 出现时翻多，次级别短差早已在次级别 type1_buy 被 recover。

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
| **P1** | extract_chain 投影 `levels[k].bsps` 进 ChainView（携 kind/level/price）+ core_level；保留走势节点（sink 载体 + 级别上下文）| cargo test rec_driver（33 测试不破）+ 新测「投影 BSP 非空」|
| **P2** | rec_stream 把 BSP 灌进 view（非仅诊断），传给 driver | cargo test rec_stream 合成零 panic |
| **P3** | driver `route_bsp`：分层路由（本级别 type1→Z₂ 翻转 + 联动次级别；次级别 type1→sink/recover 齿轮；type2/3→加仓/确认）。删 chain[0].direction 触发 + emergent + clear_root + promote。**保留 sink/recover/Z₂翻转（齿轮）+ enter** | cargo test + BTC：敞口分布（应不再 82% 持空）；ep13 无 1.2M 套牢 |
| **P4** | **per-level P&L 追踪**（滤波器输出）；三阶段口径落码（待裁 §8）| BTC + 8 标的：**每级别独立贡献正 \|涨跌幅\|**（§1.5 验收）；某级别持续负=该级别 BSP 消费有问题 |
| **P5** | 三模式对照 8 标的 | 哪种模式产生正确操作循环；ep5/ep7 AND 下消失；总 P&L vs Σ\|涨跌幅\| |

## 8. 已裁 + 待裁

**已裁解消（编排者 2026-06-20）**：§8.2「结构性确认即门」vs 裁决1 BSP 消费 **不存在概念分离**——走势结构完成是信号层的事（产出 BSP），操作层只看 BSP，**BSP 本身即门**（你只在 type1_sell fire 时做空，BSP 就是门，无额外"结构确认"中间概念）。§8.2 零参数原则**仍成立**：BSP 产出由 T 算子递归结构决定（c段/中枢/背驰都是结构定义，无参数），操作由 BSP 触发（无参数），零参数全程保持。escalate 解消，可落码。

**待裁（不可自决）**：**三阶段口径冲突**——编排者「每个级别独立核算=自相似」（§1.4 滤波器 per-level P&L）vs 之前「三阶段是总体性（根 cost_basis/phase）」。两读法：(A) 每级别独立三阶段（自相似彻底，per-level 滤波器输出独立）；(B) 持仓+P&L per-level、三阶段降成本/退本金总体（根 campaign）。P4 落码须裁。

## 9. 结果包六要素

- **结论**：操作层重构为消费三类买卖点的**分层路由 + 齿轮耦合 + 滤波器**：每级别一个 Z₂ 振荡器（多↔空），BSP 是驱动齿轮的力；**本级别 type1→整仓翻转、次级别 type1→sink/recover 齿轮咬合（同股数能量守恒）、次次级别→更小短差递归**；每级别一带通滤波器提取本频率 \|涨跌幅\|，总 P&L=Σ 滤波器输出。**删走势跟随的间接触发（chain/extract_chain/promote/emergent/clear_root），保留齿轮结构（sink/recover/Z₂翻转），唯一改动=触发器从走势 completed 换成 BSP**。回补自动化（次级别 type1_buy 立即驱动齿轮 recover，修 ep13）。绩效=Σ\|涨跌幅\| = N 级别滤波器并联吃所有尺度涨跌。**验收=每级别独立贡献正 \|涨跌幅\|**（§1.5）。
- **定义依据**：知识库:363-369（6 类定义）；017:60/66/70（一买转折定律/二买第二有利/定律一）；029:52/58/396（一卖镜像/三买持有协议/归根结底都是一买）；065课 aₙ=f(aₙ₋₁)（自相似递归）；center.rs:48/divergence.rs:158（中枢≥3/背驰≥2 门槛=base case）。
- **边界条件（结论翻转）**：(a) 底分型≠一类买点，a₀=线段 base case，段太短无中枢真无买点（§6）；(b) [[project_t_multiscale_independent_filters]] 旧版「机械翻转踏空+做空失血」——本次差异（全 6 类 BSP + c段 + 对称三阶段）能否避免=L2/L3 待回测，不得假设闭合；(c) 三阶段口径（独立 vs 总体）待裁。
- **下游推论**：(1) 删 rec_driver 的 chain[0].direction 触发 + extract_chain 的链投影逻辑 + rec_engine 的 promote + emergent/clear_root；(2) **保留 sink/recover/Z₂翻转（齿轮）+ enter**，触发器换 BSP；(3) extract_chain 扩为投影 BSP，driver 新增 route_bsp 分层路由；(4) **per-level P&L 追踪（滤波器验收）**；(5) §12.4 第二不对称（三阶段多头独有→空头逐仓可强平）与 BSP 消费正交，须独立修；(6) 每步 NT 验证。
- **谱系引用**：[[project_t_multiscale_independent_filters]]（Z₂滤波器前身，commit 50f2235013，旧信号层只消费 type1 翻转）；[[project_spiral_engine_v2_bitexact]]（D∞ 群齿轮咬合）；[[project_t_operation_self_replication]]（ε对称 sink/recover 齿轮——**保留**，触发器换 BSP）；[[project_recursive_t_architecture_v2]]（§8.2 零参数已解消=BSP 即门）；§9.8（promote+emergent −1069% binding L2）。
- **影响声明**：本文 = `docs/bsp_consumption_redesign.md`（分层路由+齿轮+滤波器），**未改任何代码**。落码 = 触发器从走势 completed 换成 BSP（删 chain/promote/emergent，保留齿轮 sink/recover/Z₂），新增 route_bsp 分层路由 + per-level P&L（P1-P5 每步验证）。这是从走势跟随到缠论 BSP 操作的触发器重构。
- **认识论等级**：§0-§2 诊断/架构/冗余删除 = **L0**（原文 + 源码逐行）；§3 修 ep13/§4 三模式/§5 AND/§6 base case = **L0 设计预测，L2/L3 待回测**（§9.8 教训：不假设闭合）。
