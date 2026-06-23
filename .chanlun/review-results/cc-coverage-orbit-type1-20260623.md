# type1 买卖点轨道完全枚举 + 确定操作映射（mandate · orbit-class type1）

> 工位：swarm/cc-coverage-spec/orbit-type1（069号递归拓扑异步自指 + 275号局部依赖）
> 日期：2026-06-23 ｜ 分支：prop4-nest-readingB-20260623 ｜ 父工位：cc-coverage-spec（任务 #28）
> 范围：**type1 买/卖单类本级别操作内容枚举 + 确定操作映射，不实装**（spec 交付，实装归任务 #29）
> 认识论等级：原文导出 = **L0 缠论语义/原文**；群论穷尽链接 = **L0 群论**；操作正确性净收益 = **L3 未决**（task #22/#29 判决场，诚实标注，§五边界条件）

---

## ★★ 主判决（type1 orbit-class 的 CC 划分片段）

**561 已证 Ω 必覆盖完全分类所有轨道（覆盖性 = L0 强必然），但只证「Ω 必全」未枚举每轨道 CONTENT。本工位枚举 type1 这一 orbit-class 的完全轨道 + 每轨道确定操作。** type1 = 本级别走势完成（背驰确认）的转折点（第17课「下跌走势转化的关节点」+ 第21课「下跌确立后中枢下方」）。

**type1 的 CC 片段定位（编排者二次重定，N9 ⊊ CC，561 §九.5）**：N9（极性翻转只在 φ=0 缝）= type1 走势完成**单轨道**的操作规则。本工位 = N9 的完整轨道展开 + 「fake-trigger flip 禁止」（背驰段 ≠ type1 confirmed）的精确边界——这正是 539 **过覆盖**根源（NEST 用背驰段武装翻转误触发，560 读法乙 7/8 穿仓）。

**type1-flip 的精确触发门控（539 过覆盖的正确操作，核心交付）**：

> **type1 confirmed flip ⟺ 几何门 G ∧ (mode 力度判据) ∧ 核心级（无活跃祖先）∧ 反向**。
> 缺 G（c 未创新高/新低）= **背驰段**（trend_diverging_segment，去 new_extreme），= **走势尚未完成** = **不操作**（no-op / hold 核心多）。

四态精确分界（divergence.rs 已实装的判据，本工位定位其操作语义）：

| 走势状态 | judge_divergence | trend_candidate | trend_diverging_segment | type1 操作 |
|---|---|---|---|---|
| c 段进行中、未近顶 | None | false | false | **no-op**（持核心多/空，d 未翻） |
| c 段近顶、双确认未创新高 | None | **true** | **true** | **no-op**（背驰段，539 误翻空现场，禁 flip）|
| c 段背驰段力度衰减、仍未创新高 | None | (按 macd) | **true** | **no-op**（背驰段 ⊋ 完成，禁 flip）|
| **c 创新高/新低 + 力度衰减 + (mode)** | **Some(BSP)** | true | true | **type1 confirmed → flip（核心级）/ sink（子级）** |

**判据**：`judge_divergence` 的 `new_extreme` 门（几何门 G·条件4，divergence.rs:202-208）= type1 confirmed 与背驰段的**唯一分界**。背驰段（trend_diverging_segment，divergence.rs:273）= judge_divergence **去掉** new_extreme ⟹ 背驰段 ⊋ 走势完成。**flip 只接 judge_divergence 的 Some，绝不接 trend_diverging_segment/trend_candidate**（后两者武装 flip = 539 过覆盖）。

---

## 一、type1 的原文定义依据（逐字引用，一级权威）

### 1.1 第17课（走势终完美 → type1 首次提出）

> 「对于下跌的走势来说，一旦完成，只能转化为上涨与盘整，因此，一旦能把握下跌走势转化的关节点买入，就在市场中占据了一个最有利的位置，而这个买点，就是前面反复强调的『第一类买点』」（第17课 L60，docs/chanlun/text/blog/017-第17课.md:60）

> 「缠中说禅趋势转折定律：任何级别的上涨转折都是由某级别的第一类卖点构成的；任何的下跌转折都是由某级别的第一类买点构成的。」（第17课 L70）

> 「注意，这某级别不一定是次级别……还有这种情况，就是不同级别同时出现第一类买卖点，也就是出现不同级别的同步共振，所以这里只说是某级别。」（第17课 L72）

**导出**：①type1买=下跌走势完成转折点（向上转折）；②type1卖=上涨走势完成转折点（向下转折）；③转折定律：每段走势的终结 = 某级别 type1（趋势转折定律，561 §九.1 升跌完备性的 type1 投影）；④共振：多级别同时 type1 = 信号最强（本工位定位为多级别核心级同时 flip，跨级别归 orbit-levelT 工位）。

### 1.2 第19课（背驰=力度衰减，type1 触发判据的力度维）

> 「背驰是两相邻同向趋势间，后者比前者的走势力度减弱所造成的」（第19课 L834，019-第19课.md:834）

> 「下跌才出货，都是有毛病的行为……出货永远都是在上涨中出的，一旦出现背驰性的上涨，就要出货」（第19课 L146-148）

> 「背驰只可能出现一次，怎么可能一次又一次。你认为的一次又一次的，根本就不是本ID所说的背驰，注意，背驰是两个同级别趋势之间对比产生的。」（第19课 L150）

**导出**：①type1 卖在上涨**背驰**确认后操作（不等下跌）——对应 `judge_divergence` 在 c 创新高时 fire（不等回落）；②背驰 = 同级别两同向趋势力度对比（≥2 同向中枢，divergence.rs `n_centers < 2 → None`）；③「背驰只一次」= type1 confirmed 不可逆（maimai.md #3 C3，confirmed 不可变回 false）。

### 1.3 第21课（type1 的完备性定位 + 与中枢/走势类型的约束）

> 「只有在下跌确立后的中枢下方才可能出现买点。这就是第一类买点。」（第21课 L42，021-第21课.md:42）

> 「一个上涨趋势确定后，不可能再有第一类与第二类买点，只可能有第三类买点。」（第21课 L40）

> 「第一类买点与第二类买点是前后出现的，不可能产生重合，而第一类与第三类买点，一个在中枢之下、一个在中枢之上，也不可能产生重合。」（第21课 L48）

> 「缠中说禅升跌完备性定理：市场中的任何向上与下跌，都必然从三类缠中说禅买卖点中的某一类开始以及结束。换言之，市场走势完全由这样的线段构成，线段的端点是某级别三类缠中说禅买卖点中的某一类。」（第21课 L66）

**导出（type1 的有效域约束，关键）**：①type1 买**仅**在「下跌确立（≥2 依次向下中枢）后的中枢下方」——**盘整下方不产生 type1**（maimai.md #1 已结算严格口径）；②上涨趋势确立后**无 type1 买/卖**，只有 type3 ⟹ **强牛中无真 type1 卖 = 不做空**（561 §九.4 编排者论证：强牛 s 保持多）；③type1 与 type3 中枢位置互斥（type1 中枢下、type3 中枢上）——这是 type1 与 type3 轨道边界，跨类归 orbit-type3 工位。

---

## 二、type1 轨道完全枚举 + 确定操作（核心 spec table）

**轨道维度**（type1 这一 orbit-class 的完全分类，561 §九.1 完全划分的 type1 切片）：
- **极性**：type1买（下跌走势完成）/ type1卖（上涨走势完成）；
- **走势类型**：趋势背驰（≥2 同向中枢，第37课 5 条件）/ 盘整背驰（单中枢，第19/24课）；
- **级别身份**：核心级（无活跃祖先 = route_bsp `None` 分支）/ 子级（有活跃祖先 = `Some(p)` 分支）；
- **完成度**：confirmed（judge_divergence Some，几何门 G 过）/ 背驰段（trend_diverging_segment，G 未过 = 未完成）。

引擎操作原语（mandate）：`enter` / `clear_all+enter=flip` / `sink` / `recover` / `drain` / `ascend` / `no-op`。

| # | 状态描述（轨道）| 缠论定义依据[逐字引用] | 确定操作[引擎原语] | 极性/仓位影响 | 539 过/欠覆盖 | 等级 |
|---|---|---|---|---|---|---|
| **T1-1** | type1卖 confirmed · 趋势背驰 · **核心级** · 反向（当前持多）| 第21课L66「向上从买点结束」+ 第37课 5 条件 + 第24课力度·judge_divergence Some(Type1Sell) | **flip**（clear_all + enter Short @ j）| 核心多→空，整仓极性翻转（φ=0 缝，N9 合法 τ）| **正确覆盖**（route_bsp:1031-1038 `dir!=cdir` 现路径，但缺 d_core 反转校验）| L0 原文+群论 |
| **T1-2** | type1买 confirmed · 趋势背驰 · **核心级** · 反向（当前持空）| 第21课L42「下跌确立后中枢下方」+ 第17课L60·judge_divergence Some(Type1Buy) | **flip**（clear_all + enter Long @ j）| 核心空→多，整仓翻转（φ=0 缝）| 正确覆盖（同 T1-1）| L0 原文+群论 |
| **T1-3** | **背驰段**（趋势/盘整，c 未创新高/新低）· 核心级 · 反向 | 第27课区间套前提「大级别**背驰段**」⊊「走势完成」（beichi.md L123 背驰段定义）·trend_diverging_segment true, judge_divergence **None** | **no-op**（持核心多/空不动，等 G 创新高确认）| **极性不翻**——d_core 未翻转（走势未完成），N9 强制锁多/空 | **539 过覆盖正确解**：禁 flip！NEST 用背驰段武装 flip = 560 读法乙 7/8 穿仓现场 | L0 原文（缠论语义中：背驰段⊊完成）|
| **T1-4** | type1 confirmed · **核心级** · **同向**（BSP 极性==核心极性）| 第21课L40「上涨趋势确立后只有 type3」——同向 type1 不存在于核心级（走势未完成才同向）| **no-op** | 极性不变 | 欠覆盖边界：同向 type1 = 概念不可能（走势完成必反向），落 no-op 安全 | L0 群论（划分排除）|
| **T1-5** | type1卖 confirmed · 趋势/盘整背驰 · **子级**（有活跃祖先 P，父持多）| 第27课区间套「次级别背驰段逐级收缩」+ 买卖点定律一（type2=次级别 type1）·judge_divergence Some(Type1Sell), nearest_active_parent Some(p), pdir=Long | **sink(p,j)**（父减 1/3 + 子级开 flip_pol(Long)=Short 短差腿吃回调）| 父核心多减仓 1/3，子级建 Short 吃次级别下跌（N9：s_sub=Short 协变于 d_sub=Down 回调，**非违反**）| **正确覆盖**（route_bsp:994-1003 reduce 分支，子级永不独立翻转）| L0 原文+群论 |
| **T1-6** | type1买 confirmed · 趋势/盘整背驰 · **子级**（父持空）| 第27课区间套对称 + 买卖点定律一·judge_divergence Some(Type1Buy), pdir=Short | **sink(p,j)**（父减 1/3 + 子级开 flip_pol(Short)=Long 短差腿吃反弹）| 父核心空减仓 1/3，子级建 Long 吃次级别上涨（N9 协变于 d_sub=Up）| 正确覆盖（route_bsp:994-1003）| L0 原文+群论 |
| **T1-7** | type1买 confirmed · 子级 · 父持多（**同父向减仓信号**，子级持 Short 短差）| 第27课「次级别走势完成升回」+ recover 语义（次级别走势完成⇒短差平清升回父向）·is_reduce=false, j 持 mob=Short | **recover(p,j)**（子短差腿全平 m=u_sub 升回父 d_P=Long）| 子级 Short 短差平清，资本升回父核心多（次级别回调走完，吃完回调）| 正确覆盖（route_bsp:1006-1010）| L0 原文 |
| **T1-8** | type1卖 confirmed · 子级 · 父持空（同父向减仓，子级持 Long 短差）| 第27课对称 + recover·is_reduce=false, j 持 mob=Long | **recover(p,j)**（子 Long 短差升回父 Short）| 子级 Long 短差平清，升回父核心空 | 正确覆盖 | L0 原文 |
| **T1-9** | type1买 confirmed · 子级 · 父持多 · j 持**同父向遗留仓**（Long，非短差 mob）| route_bsp drain 分支（子级同父向遗留仓 + 反父向 BSP）·is_reduce 路径，j.dir==Long==父向 | **drain(j)**（减暴露 1/3，不翻转）| 子级同向遗留仓减 1/3（σ-不变配额）| 正确覆盖（route_bsp:1001 drain）| L0 群论（σ-不变）|
| **T1-10** | type1卖 confirmed · 子级 · 父持空 · j 持同父向遗留仓（Short）| 对称 T1-9·drain | **drain(j)** | 子级同向遗留仓减 1/3 | 正确覆盖 | L0 群论 |
| **T1-11** | type1买 confirmed · 子级 · 父持空 · j 持同父向（Short）· **同父向 BSP 但 j 无短差** | route_bsp same-dir no-pyramid（父空+卖点对称为父向，但此处买点=反父向已归 T1-6/T1-9；本格=父空买点反父向无短差→sink）| 见 T1-6（已覆盖）| —— | 欠覆盖防御：route_bsp:1011-1013 `buy_noop` 计数（浪费的买点）| L0 群论 |
| **T1-12** | type1 confirmed · 核心级 · **首次建仓**（highest_active None）| 第17课L60「下跌走势转化关节点买入」（建仓起点）·route_bsp None, highest_active None | **enter(j, dir, node)**（全 free 建核心仓）| 空仓→核心仓（dir=dir_to_polarity(走势方向)，N9 初始锚定）| 正确覆盖（route_bsp:1023 enter）| L0 原文 |

### 2.1 关键轨道的精确判据（代码锚点）

- **T1-1/T1-2（核心 flip）的当前缺陷**：route_bsp:1031 门控**仅** `dir != cdir`（BSP 方向 ≠ 核心方向），**没有校验核心级别自身走势 d_core 是否真翻转**（561 §3.1）。严格 type1 confirmed 要求：核心级走势在该 bar 处 φ=0（judge_divergence 在**核心所在级别**产 type1）。当前 NEST 旁路（nest_try_flip 用背驰段武装）= T1-3 误判为 T1-1 = 539 过覆盖。**正确操作 = flip 只接 `is_seam_bar[core]`（核心级 judge_divergence Some），背驰段（trend_diverging_segment）不武装 flip。**
- **T1-3（背驰段 no-op）= 539 过覆盖的吃跌正确解**：`trend_diverging_segment(t, mode)`（divergence.rs:273）= `judge_divergence` **去 new_extreme**（几何门 G·创新高分量）⟹ 背驰段 ⊋ 走势完成。背驰段成立但未创新高 = **走势尚未完成** ⟹ d_core 未翻 ⟹ N9 强制持核心多 ⟹ **no-op**。强牛中走势反复背驰段但不创最终新高（beichi.md L226「背了又背」筑顶阶段），若用背驰段 flip → 反复误翻空穿仓（560 7/8）。
- **T1-5/T1-6（子级 sink）的 N9 协变正确性**：sink 开 `flip_pol(pdir)` 看似 s_sub ⊥ d_parent，但 N9 是逐级 s_k=d_k——次级别回调时 d_sub=Down（回调=次级别下跌走势），s_sub=Short 协变于 d_sub（561 §3.3）。**子级永不独立翻转**（route_bsp Some(p) 分支注释）= 区间套约束 = 正确覆盖。
- **盘整背驰的 type1 归属**（边界，见 §四 escalate 检查）：maimai.md #4 已结算「盘整背驰不产生三类买卖点」，但 divergence.rs `judge_consolidation_divergence` 仍产 BSPKind::Type1Buy/Sell。**本工位不硬编码特例**——见 §四矛盾分析（已判定为**实现错误非定义冲突**，不 escalate）。

---

## 三、type1 与 539 两失效形式的对应（重诊断 type1 切片）

| 539 失效形式 | type1 轨道 | 机制 | 正确操作 |
|---|---|---|---|
| **过覆盖①**（背驰段武装翻转）| T1-3 误判为 T1-1/T1-2 | NEST nest_try_flip 用 trend_diverging_segment（≠ type1 confirmed）武装 flip ⟹ 强牛背驰段反复翻空 | T1-3 → **no-op**（只 judge_divergence Some 才 flip）|
| **过覆盖②**（经验趋势触发）| 非 type1 状态误触发 flip | route_bsp 核心 flip 门仅 `dir!=cdir`，无 d_core 走势完成校验 | flip 加 `is_seam_bar[core]` 门（核心级 judge_divergence Some）|
| **欠覆盖**（真买卖点轨道无操作）| type1 子级轨道落父向默认 | route_bsp 只消费 is_buy，type2/3 + 中枢三态丢弃落默认分支（死扣）| **type2/3 + 中枢三态归 orbit-type2/type3 工位**；type1 子级已覆盖（T1-5..T1-11）|

**type1 orbit-class 的 539 净贡献**：过覆盖①（T1-3 背驰段误翻空）是 type1 这一类对 539 的**主贡献**（560 读法乙 7/8 穿仓）。修复 = T1-3 严格 no-op + flip 只接 judge_divergence Some（几何门 G 创新高确认）。欠覆盖的 type2/3 部分**不归本工位**（边界：本工位管 type1 单类）。

---

## 四、type1 定义矛盾检查（no-workaround：遇矛盾停下 escalate）

### 4.1 检查项：盘整背驰是否产 type1？（潜在定义冲突）

**矛盾候选**：
- maimai.md #4 已结算：「盘整背驰不产生三类买卖点中的任何一类，属于中枢震荡操作……a_buysellpoint_v1.py 只接收 kind="trend" 的 Divergence」。
- 但 divergence.rs `judge_consolidation_divergence`（divergence.rs:380-437）**仍产** `BSPKind::Type1Buy/Type1Sell`，且 T 引擎据此 route_bsp。

**矛盾分析**（no-workaround 流程：精确描述 + 依据 + 双向后果）：
- 若接受 maimai.md #4（盘整背驰 ∉ type1）：则 rec_engine 的 type1 轨道**仅含趋势背驰**（T1-1..T1-12 的「盘整背驰」分支删除），盘整背驰的离开段操作归「中枢震荡」（type3 体系或 drain）。
- 若接受 divergence.rs 现状（盘整背驰产 type1）：则 type1 轨道含盘整背驰，与 maimai.md #4 冲突。

**判定 = 实现错误 vs 定义冲突**（testing-override.md 判据）：
- 关键区分：**T 引擎的 type1 = 第65课递归不变量的「走势完成」涌现量（types.rs:172-183 注释「type1 作为迭代不变量，type2/3 是它在递归塔里的投影」），不是 a_buysellpoint_v1.py 的「三类买卖点完备性」语义**。两者是**不同所指的能指碰撞**：
  - maimai.md #4 的 type1 = 第21课买卖点完备性体系（趋势背驰才算 type1）；
  - divergence.rs 的 Type1Buy/Sell = 第65课递归「走势完成」标记（盘整走势完成也产「走势完成点」，因为盘整也终完美——第17课走势分解定理一：盘整也是完成的走势类型）。
- **盘整也终完美（第17课基本原理一「任何级别任何走势类型终要完成」）⟹ 盘整走势完成点是一个真实的转折/延续节点**。divergence.rs 把它标 Type1Buy 是**递归层「走势完成」语义**，不是第21课「下跌确立后中枢下方」的严格 type1。
- **修复不需改变任何定义的含义/边界/适用范围**——只需在 spec 层**澄清能指**：rec_engine 的 `Type1Buy/Sell`（盘整背驰分支）= 「盘整走势完成节点」，操作映射上**仍走 T1-x 轨道**（盘整走势完成同样触发核心 flip 或子级 sink），与第21课严格 type1 的差异**不影响操作映射**（两者都是「该级别走势完成 → 极性按 N9 协变切换」）。

**结论：实现错误（能指未澄清）≠ 定义冲突 ⟹ 不 escalate**。判据：能在不改任何定义前提下，通过 spec 层澄清「rec_engine type1 = 递归走势完成节点（含盘整终完美），操作语义 = 走势完成→N9 协变切换」消解。盘整背驰与趋势背驰在**操作映射上同构**（第33课「盘整背驰与背驰本质上一样」beichi.md L79），故 T1-1..T1-12 的趋势/盘整背驰分支共享操作。

### 4.2 检查项：背驰段与走势完成的边界是否可判定？（mandate 点名的 escalate 候选）

**mandate 明确要求检查**：「背驰段与走势完成的边界不可判定」是否构成矛盾。

**分析**：边界**完全可判定**——几何门 G·条件4（new_extreme：c 创新高/新低）是判定边界（divergence.rs:202-208）。背驰段（trend_diverging_segment）= judge_divergence 去 new_extreme，二者差**唯一** = `new_extreme` 布尔门。**可判定，无矛盾 ⟹ 不 escalate**。

> ⚠ 诚实标注（有效域边界，561 §2.3/§3.4 codex 精化继承）：「边界可判定」是 **L0 结构**（new_extreme 是确定布尔判据）；但「判定的**及时性**」是 **L0 不可约的确认滞后**（b1）——c 创新高在 a0 兑现时回调可能已结束（NR-4 向心读过去：背驰确认是回溯性非预测性）。**边界可判定 ≠ 及时**。这不是定义矛盾（边界清晰），是确认机制的结构税（561 (b1) L0 不可约），归 prop4-bidir L3 判决，**不 escalate**。

### 4.3 检查结论

**无 type1 定义矛盾需 escalate**。两个候选（盘整背驰 type1 归属 / 背驰段边界可判定）均判定为**实现层能指澄清 / 结构税**，非定义冲突。按 testing-override.md 判据：不需改变任何定义的含义、边界、适用范围 ⟹ 实现层处理，不上浮。

---

## 五、结果包六要素

1. **结论**：type1 orbit-class 完全枚举 12 条轨道（T1-1..T1-12），每条映射确定引擎原语（flip/sink/recover/drain/enter/no-op）。**核心交付 = type1-flip 精确触发门控**：flip ⟺ judge_divergence Some（几何门 G·c 创新高 ∧ 力度衰减 ∧ mode）∧ 核心级 ∧ 反向；背驰段（trend_diverging_segment，G 未过）= 走势未完成 = **no-op**（T1-3）。这是 539 过覆盖（NEST 背驰段武装 flip → 强牛误翻空 560 7/8 穿仓）的正确操作。

2. **定义依据**：
   - 第17课 L60/L70/L72（type1买卖定义 + 趋势转折定律 + 多级别共振）、L22-58（走势终完美 + 走势分解定理）；
   - 第19课 L834/L146-150（背驰=同级别两同向趋势力度衰减 + 背驰只一次 + 上涨背驰才出货）；
   - 第21课 L40-48/L66（type1仅下跌确立后中枢下方 + 上涨确立后无 type1 + 升跌完备性）；
   - 第37课 5 条件（divergence.rs:18-31 judge_trend_divergence 全实装）；第24/33课（盘整背驰本质同构）；
   - 输入数据满足定义：divergence.rs `judge_divergence` 的 `new_extreme` 门（几何门 G）= type1 confirmed 与背驰段的唯一分界；rec_stream `LevelView.t1buy/t1sell`（rec_stream.rs:84-85）= type1 子集信号；route_bsp（rec_engine.rs:983）`None`/`Some(p)` 分支 = 核心级/子级判据。

3. **边界条件（结论翻转条件）**：
   - 若 flip 接 trend_diverging_segment（背驰段）而非 judge_divergence Some（创新高）→ T1-3 误判为 T1-1/T1-2 → 539 过覆盖复现（560 7/8 穿仓）。**翻转条件 = 触发器从 judge_divergence 换成 trend_diverging_segment**。
   - 若核心 flip 门保持仅 `dir!=cdir`（无 `is_seam_bar[core]` = 无核心级 judge_divergence Some 校验）→ 经验趋势/旁路 NEST 仍可武装 flip（过覆盖②）。**翻转条件 = flip 门是否加核心级走势完成校验**。
   - 若 T1-3 背驰段在强牛下其净收益**为正**（吃到回调下跌）→ 背驰段 flip 反而有利，T1-3 no-op 判定需重审。**当前 560/553 证据指向背驰段 flip 净亏（确认滞后逻辑性，b1 L0 不可约），但 task #22 严格逐级区间套未试，L3 未决**。
   - 若盘整背驰的「走势完成」语义被重定为 ∉ 操作触发（接受 maimai.md #4 严格 type1 仅趋势背驰）→ T1-x 的盘整背驰分支删除，盘整离开段归 type3/drain。**翻转条件 = rec_engine type1 能指是否锚定第21课严格语义而非第65课递归走势完成**（§4.1 已判定为不翻转：操作映射同构）。

4. **下游推论**：
   - **type1-flip 门控修复 ⟹ 539 过覆盖①消失**（背驰段不武装 flip，强牛底仓锁多，吃大涨）——与 552 anchor（核心仓不僵死）、561 N9（协变强制）一致，**无 if-else 硬编码 regime**。
   - type1 子级轨道（T1-5..T1-11，sink/recover/drain）**已正确覆盖**（route_bsp Some(p) 分支协变 by construction），539 欠覆盖**不在 type1 子级**——欠覆盖的 type2/3 + 中枢三态归 orbit-type2/type3 工位（边界）。
   - type1 confirmed 触发的核心 flip = N9 的 type1 单轨道实例（561 §九.5 N9 ⊊ CC）——本工位为 N9 提供完整的 type1 轨道展开 + 背驰段排除子规则。
   - **实装映射（task #29）**：route_bsp 二元 → type1 轨道映射，需 LevelView 暴露「核心级 judge_divergence Some」（is_seam_bar[core]）而非仅 is_buy；flip 接此门，背驰段（d_top 背驰段链）只用于 sink 触发（子级），不武装核心 flip。

5. **谱系引用**（曾发生概念分离的领域）：
   - **561**（极性-级别协变 N9 ⊊ CC，本工位是 N9 的 type1 轨道完整展开）——本工位 depends_on 561，提供 type1-flip 精确门控（561 §九.5 N9 = type1 单轨道规则的完整化）。
   - **560**（读法乙 top-down 嵌套 a0 定位，7/8 穿仓）——T1-3 背驰段误翻空 = 560 穿仓的 type1 切片根因。
   - **553**（cascade flip 证伪 + 对称双吃不可达 L3 未决）——核心 flip 门缺 d_core 校验 = 553 cascade 现场；本工位定位为过覆盖②。
   - **555/554**（区间套递归精度 / A 路 d_top 嵌套精度证伪空域）——背驰段链（trend_diverging_segment）用于子级 sink 触发的精度依赖，T1-3 的 no-op 判定不依赖该精度（背驰段≠完成是 L0 布尔门）。
   - **539**（清仓判据 regime 门控开放轴）——本工位重诊断 539 过覆盖①的 type1 根因（背驰段武装 flip）。
   - **能指碰撞（已澄清，§4.1）**：rec_engine `Type1Buy/Sell`（递归走势完成节点，含盘整终完美）vs maimai.md 第21课严格 type1（仅趋势背驰）——本工位 spec 层澄清操作映射同构，不构成定义分离。
   - 不确定是否有更早的「type1 操作映射」专属谱系：检索 settled/ 未见独立「type1 orbit operation」节点，本文是首次将 type1 的完全轨道 + 引擎原语映射 + 背驰段/完成边界结晶为 spec table。

6. **影响声明**：
   - **不改动代码或定义**（spec 交付，不实装，mandate 指示；实装归 task #29）。
   - **新增**：本文件（review-results）。
   - **影响模块（若 task #29 实装）**：`route_bsp`（rec_engine.rs:983 二元 → type1 轨道映射 + 核心 flip 加 is_seam_bar[core] 门）、`LevelView`（rec_engine.rs:277 暴露核心级 judge_divergence Some 而非仅 is_buy）、`nest_try_flip`（禁背驰段武装核心 flip）、`rec_stream`（d_top 背驰段链只用于子级 sink 触发器）。
   - **重诊断 539**：type1 切片 = 过覆盖①（背驰段武装 flip，主贡献，560 7/8）；T1-3 严格 no-op = 修复。
   - **为 N9（561）提供**：type1 单轨道（561 §九.5）的完整展开（T1-1..T1-12）+ 「fake-trigger flip 禁止」子规则的精确判据（judge_divergence Some vs trend_diverging_segment）。

---

## 六、认识论等级标注（formalization-validity-domain 强制）

| 命题 | 等级 | 信息增量 | 同义反复 vs 增量 |
|---|---|---|---|
| type1 = 走势完成转折点（第17/21课）| L0 原文 | 正 | 增量：原文逐字 |
| 12 条 type1 轨道完全枚举（CC 的 type1 切片）| L0 群论+原文 | 正（覆盖性）| 增量：Burnside type1 投影 + 第21课语义 |
| type1-flip ⟺ judge_divergence Some（几何门 G 创新高）| L0 缠论语义/代码 | **高** | 增量：背驰段/完成边界的操作语义首次定位（new_extreme 门）|
| 背驰段（trend_diverging_segment）= no-op（T1-3）| L0 原文（背驰段⊊完成，第27课）| **高** | 增量：539 过覆盖①的正确解 |
| 每轨道操作内容（flip/sink/recover/...）| 缠论语义（中，原文导出）| 正 | 第21课延续/转折 + 第27课区间套 + 第33课盘整同构 |
| 盘整背驰 type1 归属 = 实现层能指澄清（§4.1）| L0（能指分离判定）| 中 | 增量：rec_engine type1（递归完成）vs 第21课 type1 澄清 |
| 背驰段 flip 净收益（T1-3 no-op 是否吃跌优）| **L3 未决** | —— | 不声称——task #22/#29 判决场，N9 必要非充分 |
| 完全 type1 覆盖 ⟹ 539 过覆盖① net 消除 | **L3 未决** | —— | 诚实标注（formalization-validity-domain 模式 3 禁有效域=定义域）|

> **核心诚实声明**：本工位的覆盖性枚举（12 轨道）= L0 强必然（CC 的 type1 切片，561 已证 Ω 必全）；每轨道操作内容 = 缠论语义导出（L0 原文，非纯群论）；**最关键的增量 = type1-flip 精确门控（judge_divergence Some vs 背驰段）这一操作语义定位**——把 539 过覆盖①的正确解（背驰段 no-op）从散落的 560/553 经验失败结晶为 type1 轨道的 L0 操作规则。但「背驰段 no-op 是否在强牛真吃跌（净收益）」= **L3 未决**（确认滞后 b1 L0 不可约使 c 创新高兑现晚于回调起点，是否净亏到算 regime 税交 prop4-bidir/task #22）。**不声称 L0 净收益（声明膨胀禁止）。**
