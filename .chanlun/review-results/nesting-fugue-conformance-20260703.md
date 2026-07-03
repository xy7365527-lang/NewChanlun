# 区间套下沉入场 + 多重赋格：原文规格考古 + 现状定案

> 工位 f1 | goal g-20260703T0100Z-nesting-fugue 首工位 | HEAD f8ea927b23 | 纯只读
> 编排者裁定原话（2026-07-02）：「高级别用 L0 区间套下沉定位入场 这个肯定要 并且你三类买卖点都要用上，多重赋格呢？」
> 认识论：本报告全部 **L0/L1**（原文语义提取 + 代码锚点核对，无 L2+ 实证断言）。溯源标签见每条。

---

## 0. 结论摘要（先说定案）

| 三要素 | 一句话定案 | 三态 |
|--------|-----------|------|
| ①下沉入场（区间套定位 L0 买点） | 区间套证书 `NestCertificate`+`n_delta()` 已在生产回测路径调用，rung 用**区间包含**（非端点相等，裁决①已合规），二三类经 `descend_type1_anchor_depth` 下沉次级别第一类锚 | **部分**（门在生产，但高级别状态→下沉的显式触发缺失，实测 95% 退化 depth=1） |
| ②全三类买卖点 | `bsp.rs` 三类判据（`is_first/second/third`→`buy1/2/3`,`sell1/2/3`）全实装；econ 门通道 Type1 基例 / Type2/Type3 复用下沉锚，三类均可作 L0 入场 | **已实装** |
| ③多重赋格（多声部并行） | σ_p 父方向（`parent_dir`）已在**分类层**实装（Ambient/FollowParent/ShortDiff 三态）；但**账本头寸层** overlay/对冲 still-MISSING（TW 三阶段 GAP3/576） | **部分**（分类有，持仓账本无） |

规格条数：**26 条**（下沉入场 A1–A10 共 10；多重赋格 B1–B16 共 16）。
三态计数：**已实装 9 / 部分 10 / 缺口 7**。
P0 缺口：**3 条**（P0-1 下沉触发状态机；P0-2 持仓节点=容器/position instance 非买卖点叶子；P0-3 多空对冲 overlay 账本）。

---

## 1. 原文规格逐节考古

### 1.1 下沉入场（第29课 + 区间套.pdf）

**第29课（`docs/chanlun/text/blog/029-第29课.md`）**

- **A1 下沉方向（大→小）**：line 256「已经反复强调，一定要从大级别往小级别看，**用区间套的方法**」；line 302「就是要首先看大级别的背驰段，然后再按区间套的方法看小级别来精确定位」。
- **A2 下沉锚 = 次级别第一类**：line 396「所有买点，归根结底都是第一类，要找第二、三类，其精确的，都要下次级别以下找第一类」。这是三类买卖点在下沉框架中的**角色分工的原文依据**——二三类不是独立入场判据，是「下钻到次级别第一类」的高层封装。
- **A3 下沉触发条件 = 高级别背驰段**：line 18「一个5分钟背驰段的下跌，最终通过1分钟以及1分钟以下级别的精确定位，最终可以找到背驰的精确点」；line 302「首先看大级别的背驰段」。触发信号是**高级别处于背驰段**。
- **A4 逐级下沉路径（非跳级）**：line 30「最后的背驰段…是一个1分钟以下级别的走势」；line 18「5分钟→1分钟→1分钟以下」。原文是**逐级**（5m→1m→1m 以下），不跳级。
- **A9 三类协同操作**：line 58「在第一、二类买点先买了，然后观察第三类买点是否出现，出现就继续持有，否则就可以抛出」；line 68「反弹的第一次次级别回试后买入…还有一个次级别的向上走势类型，如果出现盘整背驰就出掉」。**三类是一条操作链**，不是三个孤立信号。

**区间套.pdf（`docs/formal-chain/区间套.pdf`，15 页）**

- **A5 rung = 区间包含（非端点相等）**：p1「多级区间套的本体是 J^δ_e ⊆ J^δ_{e+1} ⊆…⊆ J^δ_ℓ，这是**区间包含链，不是端点相等链**」；p2-3 严格反例「J_k=[0,100],m=[20,80],s=50，区间套语义成立但 s=50≠80=ρ(m)，端点相等会产生 false negative」；p8 裁决①「(a) 区间包含是区间套 rung 的正确语义；端点相等没有跨级 rung 的原文依据」，「最严格应改成 start(parent)≤start(child) ∧ end(child)≤end(parent)」。
- **A6 唯一性 = 良式分解 / Sel_Θ 全序**：p5-6「若 sub_moves 是规范连续分解，区间包含定位天然唯一」；「全序 (width, end_time, start_time, idx)…Sel_Θ(C)=min_≺ C，∃!Sel_Θ(C)」。
- **A7 递归判定式**：p4「N^δ_{k↓e} = Cand^δ_k(J_{k-1},t) ∧ N^δ_{k-1↓e}，包含关系内置在 C^δ_k(J_{k-1},t)={c∈C^δ_k(t):J_{k-1}⊆I(c)}」；p9「基例 N^δ_{e↓e}=Conf^δ_e」。
- **A8 frontier bug（②）**：p8-10「未确认最后窗口视为不可变是 bug；增量 resume 只能从 sealed prefix 之后开始，未确认 frontier 必须重算」；p9「增量塔 L0 多 1 个中枢」。**裁决②：F_t = 全量重算塔为 canonical ⟹ 未确认 frontier 视为不可变就是 bug**。
- **A10 低级别终端 BSP 包装成父开仓证书**：由级别容器2.pdf p9-10 补充「区间套允许 γ=(c_{ℓ+1}; g_ℓ,g_{ℓ-1},…; δ)，则低级别终端 BSP 可以成为父 carrier 的开仓证书」——这正是「下沉入场」的证书形式：**高级别 carrier 的开仓证书 = 下沉链末端的低级别买卖点**。

### 1.2 多重赋格（子声部 / 级别容器 / 级别容器2 / 多空对冲）

**级别容器.pdf（17 页）+ 级别容器2.pdf（12 页）——持仓节点的存在论**

- **B1 持仓节点 = 容器 / position instance（非买卖点叶子）**：级别容器.pdf p1「开仓必须激活『容器声部』或『容器-入场证书位置实例』，不能只激活买卖点叶子」；p13「递归声部树中，持仓节点必须是容器 carrier 或容器-入场证书 position instance；买卖点叶子只能作为开仓证书，不能作为父声部持仓节点」；p15 修复建议「carrier_id=hostOf(g); entry_signal_id=id(g); position_node_id=hash(carrier_id,entry_signal_id,side,generation)」。
- **B2 买卖点 = transition trigger，Cell = state**：级别容器.pdf p13「Cell 是 state 节点；买卖点是 transition trigger。不能把 transition trigger 当成 state」。
- **B7 父声部 active 充要条件**：级别容器2.pdf p2-3「v∈A_t^raw ⟺ v 是存活 held position **或** v 由本 carrier 的 accepted certificate 开启」；p14「父声部 active 充要条件：held-live 或有归属于该 carrier 的 accepted opening certificate」。
- **B6 AncOK 是过滤器不是生成器**：级别容器2.pdf p3-4「AncOK(A)⊆A，永不添加节点；p(u)∉A ⟹ u 被剪…UpClose 才生成父，二者方向相反 AncOK(A)⊆A⊆UpClose(A)」；子声部.pdf p18-19「解释器必须同时接受父、子两个证书链，而不是靠子 voice 反推父」。
- **B9 深度祖先闭合定理**：级别容器2.pdf p6-7「active_depth_ge3>0 ⟹ active_depth1>0 ∧ active_depth2>0（祖先闭合）」；p7-11 实测 `active_depth_ge3:0→14166` 但 `active_depth1=0,active_depth2=0` 在同一 post-AncOK active set 下**数学不可能** ⟹ 口径不一致 / registry_alive 冒充 active position / telemetry bug。
- **B8 ShortDiff 父子对冲**：级别容器.pdf p5「u=(d,h,−σ)，par_C(d)=c，σ(u)=−σ(v)…父仓保持，子级反向双开」；p12 不变量5「σ(u)=−σ(v)，σ(v)=δ(entry(v))，不是相对裸买卖点的 parent_id」。

**子声部.pdf（27 页）——双视图架构**

- **B3 双视图 T_i ≠ K_i**：p15「T_i=结构视图树，K_i=操作 carrier forest，T_i⊆K_i⊆U_i∪G_i」；p25「T_i=ExtractTree(D_i)=↓r_i，K_i=OpCarrierForest(D_i)=T_i∪F_i^op∪G_i」。
- **B4 host^op / host^struct 分离**：p21「host^struct:B_i→T_i（结构可视化/bit-exact extract）；host^op:B_i→K_i（交易解释器）；二者不再混用」。
- **B5 Γ_i^K ≅ B_i**：p16「由 endpoint-complete，host^K(g)≠⊥ ∀g∈B_i，所以 Γ_i^K ≅ B_i，全格 BSP 都能进入操作解释器」。
- **B10 三类买卖点保留为证书参数**：p22「每个 BSP 证书保留类别集合 I_γ⊆{1,2,3}…三类买卖点不是丢掉，而是进入证书参数（priority/size 决定）」。
- **B11 全互斥解释器（7 谓词）**：p22「P1 风险强平；P2 已有反向 voice 需关闭；P3 同 carrier 反手；P4 父 voice active 且证书反父方向开 ShortDiff；P5 无 active parent 开 ambient voice；P6 同向加仓或记录；P7 不可执行记录。互斥化 ∑1[C_j]=1」。
- **子声部结构定理（关键洞察）**：p5-11「P1(extract tree only)∧P2(hostOf 严格右端点命中 T_i 内)∧D(全格 BSP 可交易)**三者不可同时成立**；当前 P1∧P2 下子声部结构性恒为 0（orphan frontier BSP 无法合法 host）；ΔSharpe=0 不是 alpha 被否证，而是子声部没进入目标头寸」；p26 最终结论「若要全格 BSP 参与多重赋格，必须把操作 host 宇宙从 T_i 扩展到 endpoint-complete K_i；否则保留 P1∧P2 时子声部结构性为零是**定理，不是工程 bug**」。

**多空对冲.pdf（16 页）——净额可见性与经济实质**

- **B12 净额 NAV 不可识别**：p1-2「净额 NAV 只能看见净头寸过程 N_t，看不见分账本腿结构 p_t；N(p)=∑σ_v q_{v,t}；ker N={p:N(p)=0} 都是分账本存在净额不可见的头寸组合」。
- **B13 §9 双开抵消定理**：p3-4「同标的、同单位数、父仓保持、反向双开 ⟹ 父子腿合计价格 PnL=0，扣成本后<0」。
- **B14 w≠1 可见 + 三层判定**：p4「w≠1 时 G_pair=σ(1−w)QΔP，净额 NAV 原理上能看到」；p13「三层：声部生成层 depth>0 是否 active；净额可见层 ‖ΔN‖_1 是否非零；经济有效层 Π^overlay/IR/回撤/尾部风险是否改善」。
- **B15 #5 经济实质 overlay**：p9「N_t^1=N_t^0+H_t，H_t=−σ_parent·h_t；Π^overlay=H_tΔP−ΔC；是否 alpha 取决 E[H_tΔP−ΔC]>0；若只机械对冲无预测性则风险中性下 E[Π^overlay]≤0（风险转换/暂时降敞口/路径风险管理）」。
- **B16 三种账户模型**：p11-12「one-way/net(N=Q⁺−Q⁻,只看净敞口)；hedge mode((Q⁺,Q⁻)腿可分开但价格 PnL 仍线性抵消)；portfolio margin(组合风险情景,保证金按风险/抵消计算,CME SPAN)」。

---

## 2. 现状对照表（规格 → 代码锚点 → 三态）

代码根：`rust/src/theta_v0/`。

### 2.1 下沉入场

| 规格 | 代码锚点 | 三态 | 说明 |
|------|---------|------|------|
| A1 大→小下沉 | `classifier/descend.rs:115 descend()`（compose 逆，取次级别走势序列） | 已实装 | RMove 递归塔沿 subs 下钻 |
| A2 下沉锚=次级别第一类 | `descend.rs:183 second_type_via_sublevel_type1`；`econ_positive.rs:591 descend_type1_anchor_depth` | 已实装 | 二类=次级别一类构造，生产门 Type2/Type3 都调此锚 |
| A3 高级别背驰段→触发下沉 | 无显式状态机 | **缺口 P0-1** | 门对每个候选无条件构造 `NestCertificate`，非由高级别背驰状态触发；力度 MACD 在 `descend.rs`（`SubLevelType1` 携真背驰 bool），但「高级别定方向→触发」联动缺失 |
| A4 逐级下沉（非跳级） | `nest.rs` NestRung 链 + `descend_level_decreases`（级别严格降） | 已实装 | well-founded 逐级 |
| A5 rung=区间包含 | `nest.rs:62 is_sub`（start≥∧end≤）；`nest.rs:123 nest_valid` 逐级 is_sub | **已实装（裁决①合规）** | is_sub 是区间包含**非端点相等**；裁决①要求已满足 |
| A6 Sel_Θ 唯一性 | `nest.rs:47 sel_order`（end_time,start_time,idx 字典序） | 已实装 | 缺 width 键（PDF 建议 (width,end,start,idx)，现为 (end,start,idx)）——部分 |
| A7 递归判定 N^δ | `nest.rs NestCertificate::n_delta`；`econ_positive.rs:559 build_multilevel_nest_cert`（324/345 生产调用） | 已实装 | Cand^δ ∧ 子⊆父 ∧ 基例 confirm_side |
| A8 frontier 未确认必重算 | `backtest/incremental.rs`（增量塔）；region resume | **缺口 P0/P1** | 裁决②「未确认 frontier 视为不可变=bug」；memory「frontier resume b_t 选太晚」印证 consumed 停太晚 |
| A9 三类协同操作链 | `econ_positive.rs:493 exit（P7）`+`closed_loop::sell` | 部分 | 一二类入场→观察三类的**状态机链**未显式建模，exit 委托 closed_loop 单点 |
| A10 低级别 BSP 包装父开仓证书 | `econ_positive.rs GateCertificate::Nest(cert)`（源信号 source_index+delta 构造） | 部分 | 证书从源信号构造，但「γ=(c_{ℓ+1};g_ℓ,…,g_e;δ) 携带整条下沉链」的父-carrier 证书形式未落地 |

### 2.2 多重赋格

| 规格 | 代码锚点 | 三态 | 说明 |
|------|---------|------|------|
| B1 持仓节点=容器/position instance | 无 position_node（开仓以信号为单位） | **缺口 P0-2** | 现状开仓以买卖点信号为单位，未建 `position_node=(carrier,γ,σ,n)`；级别容器.pdf 核心裁决未落地 |
| B2 Cell=state / BSP=trigger | `classifier/six_state.rs`（信号位 subset 64 态） | 部分 | 六状态是信号位快照，未区分 state-node vs trigger |
| B3 双视图 T_i≠K_i | `classifier/voice_eat.rs:81 verify_containment`（ContainmentReport.uncovered） | 部分 | `uncovered>0` 即 gap-B（host^struct 构成父 vs host^op 依附）已被代码化探测，但未建独立 K_i carrier forest |
| B4 host^op/host^struct 分离 | `voice_eat.rs:70` 注释明示 host^struct vs host^op | 部分 | 概念已识别（gap-B），未实装两套 host 映射 |
| B5 Γ_i^K≅B_i（全格可交易） | 无 | 缺口 | 依赖 B3/B4 的 K_i；未落地 |
| B6 AncOK 过滤器非生成器 | `voice_eat.rs` VoiceActivitySample + `eat()`；WF-Contain | 部分 | 祖先闭合约束语义在，但父声部生成路径（解释器接受父子链）未建 |
| B7 父 active 充要=held-live∨accepted cert | selector/voice `VoiceState` | 部分 | 需与 B1 position_node 联动，现状无 accepted-certificate→父 active 的显式路径 |
| B8 ShortDiff 父子对冲 σ_u=−σ_v | `backtest/selector.rs:140 ShortDiff(δ=−σ_p)`；`perm_test.rs:36 σ^H=parent_dir` | **部分（分类层已实装）** | σ_p/ShortDiff 是 MuClass 分类键（μ 估计/perm 分桶），**非账本头寸对冲** |
| B9 depth≥3⟹depth1/2 active | `econ_positive` acc_classification_level_hole；telemetry active_depth | 部分 | 定理已知；实测 depth1/2≈0 vs depth≥3>0 口径矛盾（级别容器2.pdf §7-9），待 codex 裁 telemetry 口径 |
| B10 三类保留为证书参数 I_γ | `bsp.rs` BspBits buy1/2/3+sell1/2/3 | 已实装 | 三类 bits 全保留，未压缩 |
| B11 全互斥解释器 7 谓词 | `closed_loop::sell::sell_decision_of`（P7 出场）；econ P1–P7 部分 | 部分 | 出场谓词有；P4 ShortDiff/P5 ambient 开仓谓词未成互斥解释器 |
| B12 净额 NAV 不可识别 | `backtest/econ_positive.rs` R 账本（net） | 已实装（作为限制） | econ 只用 R 账本 closed_loop（净额），正是「看不见分账本腿」的现状 |
| B13 §9 双开抵消 | 无（无分账本双开） | 缺口 | 无 overlay 腿则无双开 |
| B14 三层判定 ‖ΔN‖_1 | 无 ΔN 度量 | 缺口 | 需 overlay 头寸差 N^#5−N^base |
| B15 overlay N_t^1=N_t^0+H_t | 无（TW 三阶段 still-MISSING） | **缺口 P0-3** | `econ_positive.rs:65,508` 明示「不触碰 TW 三阶段 GAP3/576」；overlay 账本未实装 |
| B16 三账户模型 | R 账本=one-way/net | 部分 | 仅 net 模型；hedge/portfolio-margin 未建 |

---

## 3. 缺口 P0/P1 分级 + f2 实装设计草案

### P0-1 高级别方向确认 → 下沉触发的显式联动【下沉入场核心】

**现状**：`build_multilevel_nest_cert`（`econ_positive.rs:559`）对每个候选信号**无条件**构造区间套证书判 `n_delta`。σ_p（`parent_dir`）是 MuClass 的**被动分类标注**（`selector.rs:150`），不是「高级别处于背驰段/买卖点 → 触发下沉」的主动状态机。原文 A3（第29课 line 302）要求「先看大级别背驰段，再区间套下沉」——即高级别状态是下沉的**前置门**。

**f2 设计草案**：引入 `DescendTrigger`——高级别 carrier 处于背驰段（`divergence.rs segments_diverge` 已实装）或第一类买卖点状态时，标记 `trigger_descend=true`；区间套门只对被触发的高级别定位窗口下沉。这不是新增判据，是把已有的 `divergence` + `NestCertificate` 用高级别状态**门控**串起来（复用存量，非重造）。
> ponytail：不新建力度引擎——MACD 背驰已在 `divergence.rs`；只加「高级别状态→门控」的一个 bool 前置，不改区间套证书本身。

### P0-2 持仓节点 = 容器 / position instance（非买卖点叶子）【多重赋格根基】

**现状**：开仓以买卖点信号为单位，无 `position_node`。级别容器.pdf p13 + 级别容器2.pdf §14 的核心裁决「持仓节点必须是容器 carrier 或 position instance (c,γ,σ,n)，买卖点叶子只能作开仓证书」未落地。这是 B1/B6/B7/B9 一整族缺口的**根因**——没有容器持仓节点，父声部无法 held-live，子声部（ShortDiff）祖先闭合被剪，depth1/2≈0。

**f2 设计草案**：`position_node_id = hash(carrier_id=hostOf(g), entry_signal_id=id(g), side=δ(g), generation)`（级别容器.pdf p15 原文伪码）。开仓激活 `v_g=(h(g),g,δ(g))` 而非 `open g`。ShortDiff 子腿 parent 指向 `active position node on carrier_id`（非 `signal_id(g)`）。
> ponytail：先做 §14 简化 position identity（posId^simp=c）即可解 depth1/2 剪枝，完整 §13 (c,γ,σ,n) 的多实例/generation 是 P1（多空双开需要时再加）。

### P0-3 多空对冲 overlay 账本【多重赋格经济层】

**现状**：σ_p/ShortDiff 只在**分类/统计层**（selector/perm_test/μ 估计），账本头寸层 TW 三阶段 still-MISSING（`econ_positive.rs:65,508` GAP3/576），无 overlay 腿。多空对冲.pdf 的 B12–B16 全悬空。

**f2 设计草案**：先实装**诊断型**（多空对冲.pdf p8「诊断型 ∑G_e 声部覆盖 score」）——`overlay Π=H_tΔP−ΔC` + `‖ΔN‖_1` 度量，作为 depth>0 声部是否改变净头寸的**可观测量**，不改 R 账本 netting。这是三层判定（B14）的净额可见层，先量化再决定是否值得账户型 overlay。
> ponytail：不先建 hedge-mode/portfolio-margin 账户系统——先加 ‖ΔN‖_1 诊断（一个向量差的 L1 范数），确认 depth>0 声部**真的改变了净头寸**再谈账户型。多空对冲.pdf p13 明确：depth>0 但 ‖ΔN‖_1=0 时净额 NAV 原理上看不见，账户型 overlay 无意义。

### P1 缺口（依赖 P0，非阻塞）
- **A8 frontier 未确认必重算**（裁决②）：memory「frontier resume b_t 选太晚，consumed 停太晚」印证——增量塔 sealed prefix 边界 b_t 需回退到 last stable endpoint。属实现 bug（区间套.pdf p10-12），非定义冲突。
- **A6 Sel_Θ 加 width 键**：现 (end,start,idx)，PDF 建议 (width,end,start,idx)——非必需（现全序已唯一），但 width 优先更贴「最小包含父」语义。
- **B3/B4/B5 双视图 K_i carrier forest**：依赖 P0-2 position_node；`voice_eat.rs verify_containment` 的 `uncovered>0` 已探测到 gap-B 反例，是 K_i 缺失的直接证据。
- **B9 telemetry 口径**：active_depth_ge3 vs active_depth1/2 矛盾，待 codex 裁定是「registry_alive 冒充 active position」（级别容器2.pdf §7 情况C）还是 depth 计数口径不一致。

---

## 4. 数据事实分析：depth≥2≈0（真小转大 92.17%）对「逐级下沉」的意义

**事实**：memory 记录 `effective_nest_depth` 实测 95% 退化 base-case，跨级仅 4.6%，max 深度=1；`depth≥2≈0 = 真小转大（92.17%）`。区间套.pdf p7 归因为「端点相等 false negative」，但**现状 `is_sub` 已是区间包含（裁决①已合规）**——所以当前 depth 退化**不是端点相等 bug**（那个已修）。

**严格分析**（级别容器2.pdf §12 三种可能的定案）：
1. 缠论自相似递归**本身不推出** depth1/2 应接近 0（级别容器2.pdf p12「同一递归规则在每个相对层级上相同，无绝对 depth 特权」）。
2. 原文 A4「5m→1m→1m 以下逐级下沉」是**理论上限**（第29课 line 18/30 的最一般情况 a+A+b+B+c）。
3. 真实市场 depth=1 主导（92.17% 小转大）是**经验事实（L2 观测）**，不是定义违反——原文 line 408 缠师亲述「各级别小转大的存在，使各种走势都成为可能」，即多数转折在**一级下沉即锚定次级别第一类**，无需逐级深钻。小转大 = 次级别直接完成大级别走势 = 下沉一级即触底。

**定案**：depth=1 主导与原文**不矛盾**——原文的逐级下沉是完备性上限（不放过任何情况），真实数据的 depth=1 是小转大主导的经验分布。**但** 4.6% 的 depth≥2 是否被正确锚定（还是被 frontier bug A8 误剪）需 P1 修 frontier 后 L2 重测——这是 depth 退化里**唯一可能的实现 bug 残留**（区间套.pdf 裁决②未修部分）。

---

## 5. 结果包六要素

1. **结论**：三要素中②三类买卖点已实装；①下沉入场部分（区间套门在生产、rung 合规，但高级别触发 P0-1 + frontier P1 缺口）；③多重赋格部分（σ_p 分类层实装，持仓账本 P0-2/P0-3 缺口）。26 条规格：已实装 9 / 部分 10 / 缺口 7。
2. **定义依据**：第29课 line 256/302/396/58（下沉+三类协同）；区间套.pdf p4/p8 裁决①②；子声部.pdf p15/p25 双视图；级别容器.pdf p13 持仓节点；多空对冲.pdf p9/p13 overlay 三层判定。代码：`nest.rs:62 is_sub`（区间包含）、`descend.rs`、`econ_positive.rs:559`、`selector.rs:140 ShortDiff`、`bsp.rs` 三类 bits。
3. **边界条件**：结论翻转条件——(a) 若 `is_sub` 实为端点相等则裁决① P0（已核对为区间包含，不翻转）；(b) 若 telemetry active_depth 口径证实 AncOK 用 registry_alive 冒充 active position（级别容器2.pdf §7C），则 B9 从「口径问题」升为「AncOK 语义漂移 P0」；(c) 若 P1 修 frontier 后 depth≥2 显著上升，则 depth 退化归因从「小转大经验事实」翻转为「frontier bug 掩盖」。
4. **下游推论**：P0-2（position_node）是 B1/B6/B7/B9 一族的根因，先修它 depth1/2 剪枝自然解除；P0-3 先做 ‖ΔN‖_1 诊断决定是否值得账户型 overlay；多空对冲.pdf 证明 depth>0 但 ‖ΔN‖_1=0 时 overlay 净额不可见——故 f2 应先诊断后账本，不可反序。
5. **谱系引用**：606 区间套有效域；534 嵌套记账；GAP3/576 TW 三阶段账本（`econ_positive.rs:65,508` still-MISSING）；项目 memory「区间套已接入但95%退化」「frontier resume b_t 选太晚」「GAP3 L2 也不可达=架构非数据」「level 塔空洞 W-VERIFY 有效域=level0」。区间套.pdf/子声部.pdf/级别容器.pdf 三部曲同构：均「把局部边界条件误当全局结构语义」（区间套=端点相等；子声部=严格右端点命中 T_i；级别容器=叶子作持仓）。
6. **影响声明**：本报告纯只读，不改代码/git。产出 = f2 实装设计的规格基线（供 codex 审计）；识别 P0-2 position_node 为多重赋格根因、P0-1 下沉触发门、P0-3 overlay 诊断先行。未改动任何模块。

---

## 6. 认识论等级标注

- **L0**：全部规格提取（原文语义）+ 代码锚点核对（is_sub=区间包含、三类 bits、σ_p 分类键、TW still-MISSING 声明）——纯结构核对，无实证。
- **L1**：`descend` 力度分量经 MACD（`divergence.rs`）= 管线正确性。
- **L2 待做**（非本工位）：depth≥2 是否被 frontier bug 误剪需真实 BTC 重测；overlay ‖ΔN‖_1 是否非零需真实数据；子声部 alpha（ΔSharpe）需 L3 交叉验证。本报告**不**声明任何 L2+ 结论。
