# 多空双开设计语义调研（原文 + 设计文档 + 教义）

> 任务：多空双开的**设计语义**判定——实际对冲 vs 账本旁路；目的判定——risk hedge vs structure concurrency；
> 方向规则共存分析；与嵌套赋格、多级别/同级别交易的关系。
> 工位纪律：深度调研文档，不改代码，不 git mutation。引用锚：PDF=文件名 页码；文档=文档名:行号；代码=文件:行号。
> 日期：2026-07-19。分支：kimi-nest-mainline-20260717。

## 0. 材料清单

| 材料 | 角色 |
|------|------|
| `docs/formal-chain/多空对冲.pdf`（16 页，2026-06-29，"推导完全分类"系列） | 审计线：净额不可识别定理、§9 双开抵消定理、三账户模型、#5 经济实质 |
| `docs/formal-chain/子声部.pdf`（27 页，2026-06-29） | 审计线：子声部 voice certificate 结构性为零定理、goal #5 wrong object、K_i 修复、ShortDiff σ 约束 |
| `docs/formal-chain/区间套.pdf`（15 页，2026-07-01） | 审计线：区间套 rung = 区间包含（非端点相等），depth≥2 证书可行性 |
| `docs/bsp_consumption_redesign.md`（274 行，2026-06-20/21） | 北极星线：多空双开 = 多声部赋格，sink/recover 齿轮，C1 裁决，落码史 §9.8-9.11 |
| `docs/recursive_fugue_necessity_proof.md`（475 行，2026-06-17） | 必然性线：T16/T21 → L6 并发多声部，定理 RF/RF-cyc/RF-exh |
| `docs/architecture/complete_fugue_design.md`（780 行，2026-06-05） | 早期设计：38 课同级别程式（段内单方向，无双开） |
| `docs/nested_fugue_accounting.md`（330 行，2026-06-13/14） | 会计线：同一物理交易双层记账、守恒律、§10 实盘映射 |
| `docs/memory_snapshot/feedback_recursive_nested_fugue.md`（35 行，2026-06-12） | 结晶：双开的会计双重性定义、净额翻转根因 |
| 辅证：`docs/theta-v0-grammar-audit.md`、`docs/canonical-coverage-strategy.md`、`docs/canonical-coverage-rust-impl.md`、`docs/canonical-coverage-pdf-package.md`、`docs/reading_b_t_dual_design.md`、`chanlun/review-results/p120-nested-voice-dual-ledger-design-20260718.md`、`docs/necessity_derivation.md` | FULL 规格 §8/§9 的转引与实装对照 |

---

## 1. 逐文档语义引用

### 1.1 FULL canonical 规格（§8/§9，经审计文档转引）

双开在规格层的定义是 **hedge-mode 真实双仓**：

- 声部树 `T=(V,p)`，`ℓ_v<ℓ_{p(v)}`，非根声部方向 `σ_v = σ_r·(−1)^depth`；§9 同单位数反向双开（theta-v0-grammar-audit.md:26）。
- 「开平双开 `a_v≤a_p(v)`，同单位数 `a_v=1⟹q_v=q_p(v)`」对应「PDF page-12 `Q⁺>0∧Q⁻>0`」（canonical-coverage-pdf-package.md:150）——**规格明确要求 Q⁺ 与 Q⁻ 同时为正**，即 position book 同时持多仓与空仓。
- canonical §8 严格公理含 same-unit hedge / parent preserved / one active child（canonical-coverage-strategy.md:126）。
- rust 侧：`strategy/voice.rs:98` `voice_side` 完整定义交替（canonical-coverage-rust-impl.md:109）；「同单位双开净杠杆低但总杠杆高，必须同时约束」在 rust 未实装（canonical-coverage-rust-impl.md:137）。

### 1.2 多空对冲.pdf —— 账户层审计

这是双开语义最关键的对抗性文本，它把"分账本语法"与"账户净值"严格分离：

- **净额不可识别定理（定理 1）**：「净额 NAV 只能看见净头寸过程 N_t，看不见分账本腿结构 p_t 本身」（多空对冲.pdf 页1）；「只要 #5 没有改变 N_t，净额 NAV 的 ΔSharpe 必为 0。这就是净额账户的不可识别性定理」（页3）。
- **§9 双开抵消定理的严格版**：父仓 +σQ 保持、ShortDiff 子腿 −σH，组合净头寸 `N_pair = σ(Q−H)`；同单位数 H=Q ⟹ `G_pair = 0`，有成本时 `G_pair < 0`（页3）。「父仓保持 + 同单位数反向双开 ⟹ 这对父子腿在组合净值级别不产生正价格收益」（页4）。
- **非完全对冲例外**：`H = wQ` 时 `G#5 − G_base = −σwQΔP`——「w=0.30/0.10 的子腿若真的改变净头寸，净额 NAV 理论上可以看到它」（页4）；可见性充要条件 = `∃t: N_t^1 − N_t^0 ≠ 0`（定理 2，页5）。
- **毛分账本的两种合法用法**：账户型 `Σ_v W_v`（自融资 NAV，同单位数双开价格项仍抵消，`QΔP − QΔP = 0`，页7）vs 诊断型声部毛捕获 score `C^voice = Σ G_e`（页7-8）——「它不是自融资 NAV，因为它没有同时扣掉重叠父腿上的相反浮亏。所以它是：语法覆盖 score / 声部毛捕获 score，不是：账户净值」（页7）；「voice-capture score > 0 ⇏ self-financing NAV alpha > 0」（页9）。
- **#5 的经济实质**：若对冲腿机械反向且无预测性，鞅模型下 `E[H_t ΔP_{t+1}|F_t]=0`，扣成本 `≤0`——「这时 #5 不是 alpha，而是：风险转换 / 暂时降敞口 / 路径风险管理」（页9）；其价值体现在「降低最大回撤；降低波动率；降低尾部风险；……在特定路径上实现"先锁定下跌，再保留反弹"的路径依赖收益」（页10）。
- **目标不可混用**：「若你的目标是"组合净值 alpha"，就必须看 W^#5 − W^0。若你的目标是"声部覆盖正确性"，才看 C^voice。这两个目标不能混用」（页10）。
- **三账户模型**（页11-12）：one-way/netting 只存 `N=Q⁺−Q⁻`，同标的多空被净额化「+Q−Q=0……这正是你现在遇到的核心问题」（页11-12）；hedge mode 保存 `(Q⁺,Q⁻)`，「腿的身份、保证金、执行、止损、归因可分开。它不改变：同标的同单位数多空价格 PnL 抵消」（页12）；portfolio margin 按组合风险情景计保证金（页12）。
- **最终 boxed 结论**：「#5 的 depth>0 短差声部可以在分账本语法层成立；但它是否是账户净值 alpha，唯一取决于它是否改变净头寸过程并产生正的风险调整增量 PnL」（页14）。

### 1.3 子声部.pdf —— 证书层审计

- 「BSP 全格存在 ≠ BSP 可交易」（子声部.pdf 页3）——P1（extract tree = 最高 compose chain）∧ P2（hostOf 严格右端点命中）下，orphan frontier BSP 不能生成 voice，子声部结构性为零是定理而非工程 bug（页10-11 最终定理）。
- 「当前 goal #5 的多声部 BSP 对冲层，在这个对象定义下是 wrong object」（页11 boxed）。
- **ShortDiff 方向约束**：「ShortDiff 要求：σ_u = −σ_v。这就是：父多，子空；父空，子多」（页17 §6）；全互斥解释器原始谓词 P4：「父 voice active 且证书反父方向，开 ShortDiff」（页22）。
- 修复 = 双视图：结构树 T_i 保持 P1，操作 carrier forest `K_i = T_i ∪ F^op_i ∪ G_i` endpoint-complete，host 仍是严格右端点命中但定义域扩大（页15-16, 21 P2a/P2b 拆分）；修复后「可以被交易到，不保证有 alpha。新的判定要看 ΔN_i 是否非零」（页24）。

### 1.4 区间套.pdf —— rung 定位语义

- 问题①裁决：「区间套证书定义层错误：端点相等应改为区间包含」（区间套.pdf 页1）；严格反例：s=50 ∈ m=[20,80] 区间套成立但 s≠ρ(m)=80，「端点相等会产生 false negative。你的实测 depth≥2 为 0，正是这种 false negative 的系统性表现」（页2-3）。
- 正确语义：跨级 rung 是「子区间被父区间包含」`J_child ⊆ J_parent`（页3, 页8 裁决 a），唯一性由良式分解或 Sel_Θ 全序保证（页5-6）。
- 与双开的关系：**depth≥2 的双开/三开腿能否存在，取决于区间套证书能否达到 depth≥2**——端点相等实现下该层结构性为空，双开只有 depth=1 一层。

### 1.5 bsp_consumption_redesign.md —— 北极星线

- 北极星：「操作层只需消费这些 BSP，每级别自相似消费自己级别的买卖点、多空双开 → 理论上达绩效=Σ|涨跌幅|」（bsp_consumption_redesign.md:5-6, :13）。
- **双开的定义句**：「多空双开：不同级别可同时多/空（L4 持多骑大趋势 ∧ L1 持空吃回调）= 多声部赋格」（:54）。
- **C1 裁决（2026-06-20）**：「所有级别的所有卖点都是短差（reduce + 次级别开空）……不存在整仓翻转——每级别操作永远是 reduce→次级别做空→次级别买点回补→本层恢复（齿轮咬合）」（:23）；「核心持多骑趋势永不翻空；做空全在次级别有界短差」（:177）。
- **齿轮呼吸**：sink/recover 配对同股数进出（M=N）= 能量守恒（TW 中性）（:44-48）；「BSP 是驱动齿轮的力」（:43）。
- **平多≠开空 + 净额翻转根因**：「本级别 type1_sell → sink（reduce 本层 1/3 + 次级别开空），核心不翻空……不存在"净额翻转/整仓翻空"」（:138）；「空头不赚的根因 = 净额翻转而非逐仓独立」（:139）。
- 落码史：§9.9 扁平版 sink/recover 592/592「BSP 真正被消费 ✓ 里程碑」但做空腿 −21057（55.7% 亏损率）把 +1423% 拖到 +30.5%（:213-222）；§9.10 嵌套版穿仓 −238%（做空晚平，净空头 72.6%）（:232-243）；§9.11 修复 1（递归归还子树）→ +502%、净空头 0%，修复 2（纯 BSP 类型耦合，短差腿固定做空 `mob = Polarity::Short`，删 direction gate）（:247-264）。

### 1.6 recursive_fugue_necessity_proof.md —— 必然性线

- **L6（多声部并发）**：「由 T16（所有级别同时拥有独立走势）+ T21（并发=级别同时性，操作 per-level 独立去全局互斥）：k、k−1、k−2…各级别的走势同时进行、各自独立完美 ⟹ 各级别的四步循环可同时进行」（recursive_fugue_necessity_proof.md:116-121）。
- **L8（方向交替 + 耦合）**：「相邻声部方向交替（父多子空，ε 沿级别交替）」（:134-141，前提 T18/T24/T25）。
- **定理 RF**：操作的机动层必然组织为递归嵌套多重赋格，结构存在性 L0、每声部激活 L2（:145-154）。
- **RF-cyc（关键）**：四步 1-cycle 与 H¹ 生成元扭曲配对 = −2 ≠ 0，「非平凡性的充要根因 = γ 穿过 ε=−1」；「纯多头往返（long→cash→long，不穿手性）……配对 = 0 ⟹ 平凡」——「非平凡 H¹ 生成元必须穿过 ε=−1（真做空）——纯多头环路是平凡类，不是赋格声部」（:259-270）。**真做空腿（双开的空头侧）是赋格声部在上同调意义下非平凡的充要条件。**
- **RF-NR2（激活不必然）**：「必然的是操作机会的存在性（L4，L0）；不必然的是每个声部都实际发声……激活是 regime-gated 的 H¹ 幅度」，026:80 逐字「除了日线的单边上扬走势，短线必须坚持」直接否定"吃到每一笔"为无条件必然（:187-212，教义锚 026:80 由该文档 :196-198 转引）。
- **RF-exh**：操作 = 核心仓(H⁰) ⊕ 赋格(H¹) 两层闭合，无第三层（:372-378）。

### 1.7 complete_fugue_design.md —— 早期同级别设计（无双开）

- 38 课程式逐句拆解：「向上段的运作，都是先买后卖的」（S11，complete_fugue_design.md:40）；「向下段的运作刚好相反，是先卖后买」（S13，:42）——**段内单方向，同一级别内不存在多空同时持仓**。
- 多空不对称：「纯做多版（QQQ +71.7%）显著优于双向版……设计选择：做空作为可选模式，默认仅做多」（:301-313）。
- 降成本是持有阶段内部的短差仓位循环（30% 短差仓，:237-242）——单仓位增减，无双仓位。
- 该文档（2026-06-05）通篇无"多声部同时反向持仓"概念；双开语义是 06-12 之后才结晶的（见 §1.8/1.9）。

### 1.8 nested_fugue_accounting.md —— 会计双重性

- **操作原子（卖出）的双层会计**：「同一笔物理交易」在父账本记 `shares: N → N−m`、释放资金 `cash_released = m × current_price`；同时在子账本创建 `direction: SHORT / capital: cash_released / units: m`（nested_fugue_accounting.md:26-43）。守恒守卫：`father.shares + child.units == N_base`（:46-49）。
- **递归三层**：父（k，待回补）持多 + 子（k−1）持空 m + 孙（k−2）在反弹中做多 m2（对子降空头成本）——「assert father.shares + child.units + grandchild.units == N_base」（:128-151）。**双开递归化为三开/N 开。**
- **空头腿的账本性质（§8.4 审计修正）**：「空头取冻结 capital（非逐市），未实现 (basis − c)×units 递延到回补时结算……nav 的 capital 形式是物理单真值（空头持现金，无独立 MtM 负债）」（:224-228）。
- **实盘映射（§10）**：「实盘中（Hyperliquid）：逐仓模式（isolated margin）· 每个 voice level 可以对应一个独立的逐仓 position · **或者用净额+内部记账（如果交易所不支持同标的多仓位）**」（:240-244）。
- §12 iso 森林实装：「per-voice acted（acted_bar，去全局互斥）——同 bar 多 level/多 voice 独立操作」（:309-310）= T21 的落点。

### 1.9 feedback_recursive_nested_fugue.md —— 结晶定义

- 「同一笔物理交易在递归的不同级别有不同的会计身份。父级别卖出N股=降成本……子级别开空N股=独立逐仓头寸……物理上是一笔交易，会计上是两层记账。子空头P&L ≡ 父降成本金额（同一个数字的两个会计身份）」（feedback_recursive_nested_fugue.md:11-15）。
- 「父卖出释放的现金=子开仓需要的资本。不需要额外资金，不需要配额分配机制」（:17）。
- 「区间套=voice spawn的时序机制：高级别candidate向下递归定位→到某层出现买卖点→在那层spawn voice」（:19）。
- 「平多≠开空：平多=该级别voice走势完美关闭多头；开空=子级别检测到反向走势创建新的独立空头voice。两者发生在不同级别、不同头寸，不能混淆」（:21-25）。
- 「绩效理论上限=Σ|涨跌幅|」（:28）；「为什么之前空头腿不赚钱：之前的实装是净额翻转（一个仓位从多翻空），不是逐仓独立头寸。丢掉了递归嵌套的全部信息」（:30）。

### 1.10 reading_b_t_dual_design.md —— 独立腿读法

- NAV 口径：「单一 free 池……NAV = free + Σ_k sign(dir_k)·units_k·c」（reading_b_t_dual_design.md:146）。
- 「多空双开 = **不同级别的腿同时持不同方向**（leg_4 多 + leg_3 空），这是「多重赋格」的本体，不需要 sink 从单核心借」（:163）；「读法 B 的每级别独立骑乘是 sink/recover 想模拟的东西的本体」（:161）。

### 1.11 交替规则 vs 独立根的共存证据

- 交替规则：`voice::voice_side(root_side, depth) = root_side·(−1)^depth`（voice.rs:98，p120 设计文档 :54, :143-145 逐式推导：side(child) = −side(parent) ✓）。
- 独立根：v0「recognize 产单声部独立根 depth=0（mod.rs:426-448），每个买卖点=§5 独立根 parent:=none……FULL 八的「反向次级声部对冲」「同单位数双开」**纸面有定义、运行时不产**」（theta-v0-grammar-audit.md:26, :60）。
- 焊接方案（p120，2026-07-18）：根决策保持 `root_side=cand.dir`（自由），子决策「root_side **继承树根**（非 cand.dir）⟹ voice_side 代数自动兑现 σ_child=−σ_parent」（p120-nested-voice-dual-ledger-design-20260718.md:15）；「为什么 root_side 不能取 cand.dir：若子决策 root_side=cand.dir……则 voice_side(cand.dir, d_p+1) ≠ cand.dir（d_p+1≥1 时多翻一次）——绝对方向错」（:149）。

---

## 2. 问题①判定：实际对冲 vs 账本旁路

**判定：设计语义是分层的，三种表述各自有文档依据，且审计 PDF 已给出它们的账户后果——不能用其中一种冒充另一种。**

- **规格层（FULL §8/§9）= 真实双仓（实际对冲的 position-book 语义）**。`Q⁺>0 ∧ Q⁻>0` 同存（canonical-coverage-pdf-package.md:150），父仓保持 + 同单位数反向子腿（same-unit hedge / parent preserved，canonical-coverage-strategy.md:126）。这是 hedge-mode 语义：两条真实仓位腿同时存在于仓位簿。
- **嵌套赋格会计层 = 一笔物理交易、两个账本身份（账本双重性，非两笔真实仓位）**。父卖出是真实减仓（现金释放），子空头是**冻结 capital 上的账本腿**——「空头持现金，无独立 MtM 负债」（nested_fugue_accounting.md:228）；「物理上是一笔交易，会计上是两层记账」（feedback_recursive_nested_fugue.md:14）。Σunits=N_base 是会计守恒，不是市场敞口守恒。
- **实盘 §10 允许两读**：逐仓独立 position（退回规格层真实双仓）**或净额+内部记账**（账本旁路）（nested_fugue_accounting.md:240-244）。
- **审计层的严格后果**：选"净额+内部记账"路径，则同单位数双开满足 `N_t` 不变 ⟹ 净额 NAV 原理上不可见（多空对冲.pdf 定理 1，页2-3；「这正是你现在遇到的核心问题」页12）；即使 hedge mode 真实双仓，价格 PnL 仍线性抵消（页12）。**任何路径下，同单位数双开在账户净值层都不产生正价格收益**（页4 §9 不可能定理严格版）。其可度量产出只剩两类：(a) 诊断型声部毛捕获 score（语法覆盖，非 NAV，页7-9）；(b) 净敞口时变产生的 overlay 增量（w≠1 或父 reduce 造成的 ΔN≠0，定理 2 页5）。

**一句话判定（090 口径）**：多空双开的设计语义 = **父仓保持 ∧ 反向子腿存在的分账本声部结构（语法层 L0）**；它的物质化载体可以是逐仓真实双仓（规格首选）或净额+内部记账（§10 降级路径），嵌套赋格会计进一步把它精化为"父真实减仓 + 子冻结-capital 账本空头"。声明"双开=两个方向的真实仓位必须同时存在"只在 hedge-mode/逐仓载体内成立；声明"双开在净额账户产生 NAV alpha"在任何载体内都不成立（多空对冲.pdf 页9, 页14）。

## 3. 问题②判定：risk hedge vs structure concurrency

**判定：设计文档线的语义是 structure concurrency（级别并发的结构必然性），不是 risk hedge；"对冲"只是审计线对机械反向腿在账户层的经济学判读。**

- **最深层依据（必然性线）**：双开（多声部并发反向持仓）是 T16（所有级别同时拥有独立走势）× T21（并发=级别同时性，去全局互斥）的操作层物质化（recursive_fugue_necessity_proof.md:116-121），是 H¹ 机动层的必然**可能性结构**（定理 RF，:145-154）。更强地，RF-cyc 证明不穿 ε=−1（不真做空）的环路是上同调平凡类——**做空腿是赋格声部非平凡的充要条件**（:259-270）：双开不是可加可减的风险工具，是 H¹ 结构本身的构成要素。
- **北极星线的目的**：消费 BSP → 每级别吃自己频率的 |涨跌幅| → 绩效=Σ|涨跌幅|（bsp_consumption_redesign.md:5, :13, :54-55）。这是**语法捕获**目标（每级别带通滤波器并联），不是风险对冲目标。
- **嵌套赋格线的目的**：父降成本（cost_basis 下降）= 子空头 P&L（feedback:15）——短差是降成本机制，C1 明确「核心持多骑趋势永不翻空；做空全在次级别有界短差」（bsp_consumption_redesign.md:177），语义是"减仓-回补齿轮呼吸"（:44-48），不是"对冲风险敞口"。
- **审计线的判读（risk hedge 语义的合法位置）**：若双开腿机械反向、无预测性，则经济上「#5 不是 alpha，而是：风险转换 / 暂时降敞口 / 路径风险管理」（多空对冲.pdf 页9），合法绩效读数是回撤/波动/尾部/hedge efficiency（页10）。
- **不可混用条款**：「若目标是组合净值 alpha，就看 W^#5−W^0；若目标是声部覆盖正确性，才看 C^voice。这两个目标不能混用」（多空对冲.pdf 页10）。
- **"力量对比"**：本批文档未将双开与力量对比直接挂钩（该概念属 v3/K4 另一条线，本调研照实标注为**无锚**）。**"赋格呼吸"** = sink/recover 减仓-回补循环（bsp_consumption_redesign.md §1.3 :40-49）= 四步 1-cycle 的时间展开（recursive_fugue_necessity_proof.md L5 :102-114），它是结构并发语义的时间维表现，不是独立目的。

**结论**：语义上双开 = **structure concurrency + 语法捕获（Σ|涨跌幅| 目标）+ 降成本齿轮**；risk hedge（风险转换）是同单位数机械双开在账户层的**唯一可观测经济效果**（审计线判定），而非设计文档赋予它的目的。把双开当 risk hedge 设计 = 读错对象；把它的声部捕获 score 当 NAV alpha 宣称 = 声明膨胀（090）。

## 4. 问题③：σ_root·(−1)^depth 交替 vs 独立根自由方向——如何共存

**判定：两个规则在文档中不是竞争关系，而是"树根自由、树内交替"的分层混合；depth=0 处 (−1)^depth=1，两规则在根处自然重合。**

- **交替规则的辖域 = 嵌套子声部（depth>0，同一棵树内）**：canonical §8 `σ_v=σ_r·(−1)^depth`（canonical-coverage-strategy.md:125）；ShortDiff `σ_u=−σ_v` 父多子空（子声部.pdf 页17）；T25 父多→子空→孙多（necessity_derivation.md:578）；rust `voice.rs:98` voice_side。**方向在此被代数锁定，不自由**——因为子腿的存在论身份就是"父仓的反向对冲腿"。
- **自由方向的辖域 = 根声部/独立腿（depth=0，树与树之间、级别与级别之间）**：v0 每个 BSP 产独立根 `parent:=none`、方向取 `cand.dir`（theta-v0-grammar-audit.md:26）；读法 B 每级别腿按本级别 d_top 独立切换方向（reading_b_t_dual_design.md:163）。**方向由本级别信号自由决定**——因为根的存在论身份是"本级别走势的独立骑乘者"。
- **共存形式一（规格 vs 实装的退化差）**：theta-v0-grammar-audit.md:26 把它记录为缺口——FULL 八纸面有嵌套双开定义，v0 运行时只产 depth=0 独立根，「结构预留已就位、无消费者」。
- **共存形式二（p120 焊接，2026-07-18）**：根决策 `root_side=cand.dir` 保持自由，子决策 `root_side` 继承树根，`voice_side(R, depth)=R·(−1)^depth` 自动兑现交替（p120:15, :143-149）。这是"独立根 + 交替子"的严格代数混合。
- **共存形式三（C1 退化特例）**：bsp_redesign §9.11 修复 2 把核心固定 σ_root=+1（永不翻空）、短差腿固定 Short（BSP 类型即方向，删 direction gate）（bsp_consumption_redesign.md:256-264）——交替规则在"核心恒多"下的静态特例。
- **逻辑一致性**：depth=0 时 `σ_v=σ_r·(−1)^0=σ_r`，交替规则对根不设任何约束——两个规则在同一个公式里共存，辖域按 depth 切开，无矛盾。

## 5. 问题④：与嵌套赋格、与多级别/同级别交易的关系

### 5.1 与嵌套赋格：双开是赋格的声部间方向形态

- 双开（父多 ∧ 子空）是嵌套赋格的最小非平凡实例；nested_fugue_accounting.md §5 把它递归化为父多 ∧ 子空 ∧ 孙多（:128-151），守恒沿递归链。代数身份 = H¹ 生成元沿 σ-塔的 σ-等变实现，相邻声部经 φ=0 手性缝方向交替（recursive_fugue_necessity_proof.md:306-320 定理 RF-alg）。
- **时序机制 = 区间套**：「区间套=voice spawn 的时序机制」（feedback:19）；四步循环 ②开空@k−1 由次级别 confirm 定时（recursive_fugue_necessity_proof.md L5 :102-114；bsp_redesign §9.2 递归正则化 :141-145）。区间套.pdf 的裁决（rung = 区间包含而非端点相等，页8）是 depth≥2 双开/三开腿可行性的前提——端点相等实现下「depth≥2 为 0」是 false negative（区间套.pdf 页3）。
- **证书可行性 = 子声部.pdf 的 K_i 修复**：orphan frontier BSP 在 P1∧P2 下不能成为 voice certificate（子声部.pdf 页10-11），K_i endpoint-complete 后「Γ^K ≅ B」全格可交易（页20），但「可以被交易到，不保证有 alpha」（页24）。goal #5「多声部 BSP 对冲层」在原对象定义下是 wrong object（页11 boxed）。

### 5.2 与多级别交易：双开是多级别独立操作在方向维的必然表现

- T16（所有级别同时拥有独立走势）⟹ 任意时刻常态是"某级别在涨、另级别在跌"；T21（per-level 独立，去全局互斥；落点 = iso 森林 per-voice acted，nested_fugue_accounting.md:309）⟹ 各级别独立响应自己级别的 BSP ⟹ **方向相反的多腿并发（双开）不可避免**——这正是「L4 持多骑大趋势 ∧ L1 持空吃回调 = 多声部赋格」（bsp_consumption_redesign.md:54）。
- 反向蕴涵也成立：净额单仓（一个净头寸 N_t）在数学上不可能表达多级别独立操作——净额 NAV 只识别净头寸（多空对冲.pdf 定理 1，页2）。**多级别独立交易 ⟹ 必须采用分账本/多仓位表示 ⟹ 双开是表示层的必然，不是策略选择。**「空头不赚的根因 = 净额翻转而非逐仓独立」（feedback:30；bsp_redesign:139）是同一判定的经验侧。

### 5.3 与同级别交易（38 课程式）：双开是声部间结构，不是段内结构

- 同级别分解程式段内单方向：向上段先买后卖（long-only）、向下段先卖后买（short-only）（complete_fugue_design.md:40-43，教义锚 038 课由该文档第一部分转引）；早期设计甚至默认不做空（:301-313）。
- 时间序：complete_fugue_design（2026-06-05）无多声部双开 → feedback（06-12）结晶会计双重性 → nested_fugue_accounting（06-13/14）规格化 → necessity proof（06-17）给必然性 → bsp_redesign（06-20/21）北极星化并落码。**双开语义是后结晶的，且与 38 课程式不冲突**：每个声部**内部**仍是先卖后买/先买后卖的单方向四步循环（recursive_fugue_necessity_proof.md L5，教义锚 093"先卖后买"/025:126 由该文档 :104-106 转引），双开是**声部之间**（级别与级别、父与子）的方向结构。
- 单级别内若出现"同时多+空"，在当前教义链中没有依据——双开的合法位置永远是**跨级别/跨声部**（ShortDiff σ_u=−σ_v，子声部.pdf 页17；或多级别独立腿，reading_b:163）。

---

## 6. 矛盾与待裁清单（090 诚实上浮）

1. **净额降级路径 vs 不可识别定理**：nested_fugue_accounting.md:243 允许「净额+内部记账」实盘降级，但多空对冲.pdf 定理 1（页2）证明该路径下同单位数双开在账户层不可见——此时 §1.5 验收（per-level P&L > 0，bsp_redesign:58）只能读作 voice-capture score（诊断型），不能读作 NAV。绩效=Σ|涨跌幅| 在净额账户下**只是语法覆盖 score 的同义语**（多空对冲.pdf 页9：score>0 ⇏ NAV alpha>0）。声明口径必须随载体切换。
2. **结构必然 ≠ 激活必然**：RF 定理（L0）与 RF-NR2（L2，regime-gated）分层（recursive_fugue_necessity_proof.md:187-212）；落码证据同向：做空腿 −21057 / 55.7% 亏损率（bsp_redesign:218）、026:80 单边上扬禁短差。双开的**存在**不依赖经验，双开的**发声**依赖 regime。
3. **规格-实装退化差未闭环**：FULL §8/§9 嵌套双开纸面有定义、v0 运行时不产（theta-v0-grammar-audit.md:26, P1）；p120 焊接方案（2026-07-18）是文档设计、本调研范围内未见实装证据；「同单位双开净杠杆低但总杠杆高」约束 rust 未实装（canonical-coverage-rust-impl.md:137）。
4. **C2 配额未裁**：1/3（H⁰/H¹ 上同调导出）vs 无配额（资金守恒自动，feedback:17）——bsp_redesign:178 标注「须裁」，至今未在该文档内解消。
5. **recover 有效域**：recover 率 seg 层 85-93%、rL2+ 全标的 0%（bsp_redesign:173 G4）——高层双开腿的回补（呼吸闭合）有效域 = 低级别；高级别双开结构性套牢风险是 ep13/§9.10 穿仓（−238%）的同构病（:243）。
6. **交替闭式证明缺口**：Lean 侧 `alternating`/`adjacent_opposite` 已证相邻反向，但 `σ_r(−1)^depth` 闭式未显式证（canonical-coverage-strategy.md:125，◐）。

## 7. 总结论（四问压缩）

- **①** 设计语义 = 分账本声部结构（父仓保持 ∧ 反向子腿），物质化载体二选一：逐仓真实双仓（规格首选）/ 净额+内部记账（§10 降级）；嵌套赋格会计将其精化为"一笔物理交易、两个账本身份"（父真实减仓 + 子冻结-capital 账本空头）。**既不是无条件"两个真实仓位必须同时存在"，也不是无代价的"净额单仓+声部旁路"**——后者在账户层不可识别（定理 1）。
- **②** 语义上是 **structure concurrency**（T16×T21 的级别同时性物质化 + H¹ 必然可能性结构 + Σ|涨跌幅| 语法捕获目标）；risk hedge 只是机械同单位双开在账户层的唯一可观测经济效果（风险转换），不是设计目的；两目标不可混用（多空对冲.pdf 页10）。
- **③** 两方向规则按 depth 分层共存：**depth=0 根自由（cand.dir / 本级别信号），depth>0 树内交替（σ_root·(−1)^depth 代数锁定）**；depth=0 处公式重合，无矛盾。p120 的 root_side 继承方案是两者的严格焊接；C1 核心恒多是交替规则的静态特例。
- **④** 双开 = 嵌套赋格的最小非平凡声部对（递归化为三开/N 开，守恒沿链）；= 多级别独立交易在表示层的必然（净额单仓数学上表达不了多级别独立操作）；与 38 课同级别程式不冲突——段内单方向、声部间双开，双开永在跨级别/跨声部位置。
