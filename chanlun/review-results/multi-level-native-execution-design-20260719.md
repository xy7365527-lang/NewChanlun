# 多级别同级别交易端到端设计研究：1 分钟递归塔 vs 传统多时间窗口

- 日期：2026-07-19（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）
- 性质：**深度调研文档工位**——纯设计研究，不改 `rust/src` 任何一行；不引入概率/统计推断作决策基础；不用回测验证策略；不假设 EMH。本文一切「设计」均为候选方案的形式化陈述，不是实装承诺。
- 用户核心洞察（本研究的出发点）：传统缠论多级别交易用不同固定时间窗口（1min/5min/30min 图各做各的同级别交易），而本项目只有 1 分钟 K 线向上递归——**没有固定时间窗口，只有级别**。本研究检验该洞察的机制含义，并给出「级别事件驱动执行」的候选设计。
- 关联文档：`chanlun/plans/mainline-merged-roadmap-20260717.md`（横切⑪ E2E 定义，行 60-79）、`chanlun/review-results/doc-divergence-endtoend-prototype-20260718.md`（E2E 原型 §1/§6）、`chanlun/review-results/nest-tower-native-gap-audit-20260717.md`（§2.G7 `events_by_level`）。

## 0. 结论摘要

1. 用户洞察成立且可形式化：传统多时间窗口给了每级**独立时钟**（5min 图一根 bar=5 分钟，天然低频）与**独立图表边界**（各级别只看自己的结构）；我们的递归塔给了每级**独立结构**（L3 中枢/L3 BSP）但**没有独立时钟**——执行层仍在每 1 分钟 bar 上统一 tick（`rust/src/theta_v0/backtest/runner.rs:432-448` per-bar 驱动 + 活动集台账）。
2. 扁平化的形式化根因有二，均在执行层而不在分类层：①**统一 bar tick** 把 L3 事件切成 1min 碎片（全级别新确认 BSP 在同一 bar diff 中混合消费，`runner.rs:809-835`）；②**净额**把各级别头寸在订单层压成一个有符号标量（`coverage.rs:2340`、`overlay_state.rs:15-18`），级别身份在 `Order_t=ΔN` 处丢失。
3. `voice.rs:5` 的根=L*+max_depth 设计**在语义上已级别索引**（声部树按级别分层、资金帽按深度加权），但 v0 recognize 只产 depth=0 单声部（`voice.rs:93-95` 诚实声明），且决策出口仍是每 bar 的净目标 p̃（`coverage.rs:2780-2802`）——**语义支持级别事件驱动的一半（级别索引就位），实装缺另一半（级别事件时钟 + 级别独立账本）**。
4. 候选设计「级别事件驱动执行」（LEE）：每级独立仓位账本 + 独立事件时钟 + 独立 BSP 消费 + 独立 sizing；物理订单 = Σ_ℓ Δq_ℓ，净额恒等保持（Σ_ℓ Δq_ℓ ≡ ΔN），净额从**决策机制**降级为**账户层可行性约束**。与 E2E-O `Consume_at` 的关系：LEE 是 `Consume_at` 输出按 `formation_level` 的执行层投影（`doc-divergence-endtoend-prototype-20260718.md:75-78`）。

---

## A. 传统多时间窗口 vs 1min 递归塔：机制对照

### A.1 教义侧：原文的「级别」本来就是递归的，不是时间窗口的

- 039 课注记直接点明：「38课针对后期递归级别的5分同级别分解，39课针对后期递归级别的1分同级别分解」（039:14）。即原文后期的「1 分」「5 分」是**从 1 分钟 K 线递归上来的走势级别**，与我们塔的 L2/L3 同构，不是行情软件的时间窗口切换。
- 同级别分解的机械操作程式定义在**段事件**上而非时间 tick 上：「Ai 与 Ai+2 之间就可以不断地比较力度，用盘整背驰的方法决定买卖点……只理会一点，就是 Ai 与 Ai+2 之间是否盘整背驰，只要盘整背驰，就在 i+2 为偶数时卖出，为奇数时买入」（039:30）。操作触发器 = 段完成/盘背确认，是**结构事件**，不是「每根 K 线检查一次」。
- 韵律/节奏是级别的属性：「最大的就是向上段先买后卖与向下段先卖后买的韵律」（039:20）；「节奏是第一的，你跳舞，节奏全乱，会有好心情、好心态吗？」（039:147）。
- 按级别操作的原文形态：「你是按30分钟级别操作的，明明顶背驰了，你不卖……这样下来，你很快就不用玩股票了」（032:26）；「每次，5分钟的向上离开中枢后，一旦背驰，就要出来，然后如果一个5分钟级别的回拉不回到中枢里，就意味着有第三类买点，那就要回补，等待c段的向上」（032:30）。**持仓生命周期锚定在该级别的结构事件序列上**。
- 级别间不可约：「背驰的级别和上涨的级别没有什么必然的对应关系，要有对应关系，就必须满足区间套关系」（032:227）；存在「向上30分钟级别的a+A+b+B+c，如果c是一个1分钟级别的背驰，最终引发下跌拉回B里」的跨级形态（044:16）。（roadmap:64 已据此冻结：谱系可记 skip edge，不得伪造中间级别证书。）

### A.2 工程侧：两种实现的机制对照表

| 维度 | 传统多时间窗口（1min/5min/30min 三图） | 本项目 1min 递归塔 |
|---|---|---|
| 级别来源 | 时间聚合：5min bar = 5 根 1min bar 重采样 | 结构递归：L_{k+1} 由 L_k 走势类型三段以上重叠构成（中枢递归，039:26 的 a+A 结合运算同义） |
| 每级时钟 | **独立**：5min 图一根 bar=5 分钟，决策频率天然被窗口压低至 1/5、1/30 | **共享**：塔每根 1min bar 前缀重分类一次（`runner.rs:437-438`），所有级别事件在同一 1min 钟上到达 |
| 每级图表边界 | **独立**：5min 图只看 5min 结构，1min 噪声在聚合中消失 | **共享输入**：各级别结构都从同一条 L0 序列递归；噪声不消失，被结构定义吸收 |
| 每级仓位账本 | **独立**（交易员惯例）：5min 头寸、30min 头寸分账管理，各有各的开平仓节奏 | **净额**：p̃=Σσ_v·q_v 单一有符号净目标（`coverage.rs:2340` 恒等 `net_target_units(&legs)==p_tilde`） |
| 同级别交易形态 | 每张图上各做各的同级别分解交易（039 课程式逐图独立运行） | 塔内 `classification.levels[ℓ].bsp` 提供各级别 BSP（`runner.rs:481`），但消费后被折成统一 p̃ |

### A.3 本质差异的凝练

时间窗口做了两件我们塔里没有对应物的事：

1. **独立时钟（降频器）**：5min 窗口把该级的决策点稀疏化到每 5 分钟一次，30min 到每 30 分钟一次。窗口越宽，该级别的操作越「钝」——这正是「按 30 分钟级别操作」的机械保证（032:26）。递归塔里 L3 结构虽然低频**演化**，但它的**消费**每 1 分钟发生一次。
2. **独立边界（隔离器）**：每张图是一个封闭观察域，1min 图的波动进不了 30min 图的决策。递归塔里所有级别共享同一 L0 输入与同一执行出口，级别间的隔离只靠数据结构（`levels[ℓ]`）维持，到执行层即消失。

反过来，递归塔给了时间窗口没有的东西：**级别的结构严格性**（级别由中枢递归定义而非任意重采样）与**跨级谱系**（区间套逐边见证，E2E-L，roadmap:64）。本设计的目标是把前两件（独立时钟、独立边界）在塔内补回，同时保留后两件。

---

## B. 扁平化的形式化根因

「交易扁平化」= 各级别交易在执行层失去级别身份，退化成一个在 1min 钟上调整的单一净头寸。根因二条，互为放大器。

### B.1 根因一：统一 bar tick 把 L3 事件切成 1min 碎片

执行主环是 per-bar 的：`pi_theta_fill_loop`（`runner.rs:1020`）每 bar ①延迟成交 ②取 p_t=units（净 lot）③前缀因果重分类 ④`pi_theta_step` 产订单（`runner.rs:953` 的步骤注释）。BSP 部署按「确认-bar diff」：`newly_confirmed_step`（`runner.rs:809-835`）对本 bar **所有级别**的新确认 BSP 一次性 diff（`seen.insert((lvl, source_index, bits))`，runner.rs:828），全部塞进同一个 `Classification` 投影喂给当步决策。

后果：一个 L3 BSP 的确认与一个 L1 BSP 的确认在同一 bar tick 进入同一决策调用；L3 持仓目标在其寿命内的**每一根 1min bar** 都被重新参与 p̃ 计算（`coverage.rs:2796-2797` 每 bar 重跑环5+环6）。L3 事件没有「事件时点建仓—持有—事件时点平仓」的完整生命周期，而是被摊成 1min 网格上的目标头寸序列——**L3 交易被切成 1min 碎片**。这正是 039:30 机械程式（Ai+2 盘背才动作）与现行实装（每 bar 重估）之间的结构差。

### B.2 根因二：净额把各级别头寸混合成单一标量

环7 决策出口是有符号净持仓：`coverage.rs:2762-2763` 诚实声明「净额降维：p 是有符号净持仓（分账本多空腿 q^± 在 net_target_units 已降维……v0 净额）」。恒等式 `net_target_units(&legs)==p_tilde`（`coverage.rs:2340`）保证 Σσ_v·q_units == Net(P^sep)。Overlay 层如实记录后果：「多空双开/短差在净额上**抵消**」（`overlay_state.rs:5-6`），订单为 `Order_t = N_t − N_{t−1} = ΔN`（`overlay_state.rs:16-18`），runner 逐 bar 断言 ΔN 守恒（`runner.rs:1560-1566`）。

后果：即使声部层（P^sep）保留了逐声部腿，订单层只看得见净增量。L3 多头与 L1 短差空头在 ΔN 中互相抵消，**交易所侧持仓、成本基准、强平暴露全部是级别混合的**；`overlay_state.rs:22-27` 的对账恒等（Σpnl_v = N·ΔP，Fubini 重排）只给**事后归因**，pnl_v「不是独立 self-financing NAV」（`overlay_state.rs:27`）。级别身份在 `ΔN` 处不可恢复地丢失（净额是有损投影，`overlay_state.rs:15`「有损投影，双开 (Q,Q)↦0」）。

### B.3 二因叠加 = 扁平化

B.1 让所有级别共享最高频时钟，B.2 让所有级别共享同一头寸容器。叠加后：任何级别的交易都表现为「1min 钟上的净头寸抖动」，多级别同级别交易（039 课在各递归级别上各自独立运行的程式）在执行层塌缩为单级别净额交易。分类层（塔、`levels×bsp` 同构，`coverage.rs:2376-2378`）给出的级别信息在执行层被两处有损投影（bar tick 混合 + net 混合）丢弃——**扁平化是执行层投影的信息损失，不是分类层的级别缺失**。

---

## C. 候选设计：级别事件驱动执行（LEE, Level-Event-driven Execution）

设计目标：L3 的仓位只在 L3 结构事件（L3 BSP 形成/失效、L3 中枢生灭）时变动，L1 同理；各级别独立仓位账本 + 独立事件时钟 + 独立 BSP 消费 + 独立 sizing；净额降级为账户层约束。

### C.1 四根支柱

**支柱 1：每级独立仓位账本 Ledger_ℓ。**
每级 ℓ 一个账本，键 = 在该级开仓的声部（开仓事件 = `formation_level=ℓ` 的 BSP）。`BspKey=(formation_level,point_class,side,source_index)`（roadmap:64）天然给出按级别路由的键——BSP 在谱系层已有全局点身份，LEE 只是按 `formation_level` 分桶持有。账本行可复用 `overlay_state.rs` 的 `VoiceBook`（hedge-mode 逐声部簿，`overlay_state.rs:59-60`）加 `level` 字段；`trading/level_operating_unit.rs:48-55` 的 `RevTranche{level,…}` 与「每级别至多一个开放 tranche」（`level_operating_unit.rs:9-10`）已是 trading/ 线的同形先例。

**支柱 2：每级独立事件时钟 clock_ℓ。**
clock_ℓ 的事件流 = 塔级别 ℓ 的结构事件 diff：BSP Confirmed/Invalidated、中枢形成/破坏、段完成，全部从 per-bar 前缀因果重分类的 `classification.levels[ℓ]`（`runner.rs:815-833` 同型投影）与 `events_by_level[ℓ]`（nest-tower-native-gap-audit §2.G7，nest.rs:540-542，下标=塔级别）派生。无事件 bar 对 Ledger_ℓ 是**纯 hold**：目标不变、零订单、零成本。这把 B.1 的统一 tick 拆开：每级只在自己的事件时点「醒来」。E2E 原型的五钟（`observed/first_provable/structure_end/confirmed/invalidated`，roadmap:75 E2E-N5 行）提供事件时点的因果定义——`confirmed_at` 即 clock_ℓ 的合法 tick，无前视。

**支柱 3：每级独立 BSP 消费 Consume_ℓ。**
Consume_ℓ = E2E-O `Consume_at` 按级别的执行层投影。`Consume_at` 的现行冻结语义是「授权 BSP 在同一生产事务形成并写入谱系反向边」（roadmap:62；原型签名 `Consume_at(as_of, prior_bsp_by_key, prior_links_by_key, closed_lineage, ManagedBspPolicy) → (ManagedBspCreation[], BspLink[])`，doc-divergence-endtoend-prototype-20260718.md:75-78），且只接收 `Closed` 谱系（:85）。LEE 在其后加一步确定性路由：`ManagedBspCreation` 按其 `BspKey.formation_level` 投入对应 Ledger_ℓ；`ManagedBspPolicy` 可按级别族配置（如「L3 账本只消费 formation_level=3 且 side=Long 的 BSP」）。**LEE 不改变 `Consume_at` 的生产语义，只改变其输出的持有方式**——这是对 E2E 架构的纯下游扩展，不触碰八道 E2E-S* 缝合线（roadmap:79）。

**支柱 4：每级独立 sizing。**
资金帽按级别加权：w_ℓ 为级别 ℓ 的资本权重，Σw_ℓ ≤ 1，未用部分保留现金不重分配——`voice.rs:12` 的 `depth_weights=[0.60,0.30,0.10]` 已是同构机制（按 depth 加权），LEE 把它从 depth（声部树深度）重锚到 level（塔级别）。单级 sizing 内部沿用既有 `base_units×w→q` 管线（lot 对齐，`overlay_state.rs:14`）。sizing 参数属 Θ_risk（结构承载非缠论可导，`coverage.rs:2430-2434` 同纪律），LEE 不新增任何缠论声明。

### C.2 伪码（候选，非实装）

```text
// 不变部分：因果前缀分类（639 纪律，per-bar 前缀塔，无前视）
for bar i in bars:
    (class_i, tower_i) = classify_prefix(bars[0..=i])        // 与现行 runner.rs:563-565 同

    // 变化部分①：每级独立事件钟（替代全级别混合 diff，runner.rs:809-835）
    for ℓ in 0..=top_level:
        events_ℓ = diff_level_events(class_i.levels[ℓ], seen_ℓ)   // BSP 生灭/中枢生灭/段完成
        if events_ℓ.is_empty():
            continue                    // clock_ℓ 本 bar 无 tick：Ledger_ℓ 纯 hold
        // 变化部分②：级别独立消费 + 独立 sizing（替代统一 p̃ 净目标）
        target_ℓ = ledger_ℓ.on_events(events_ℓ, policy_ℓ, w_ℓ, base_units)
        Δq_ℓ     = target_ℓ − ledger_ℓ.q_current

    // 变化部分③：物理订单 = 各级增量之和；净额退为账户层可行性约束
    order_raw  = Σ_ℓ Δq_ℓ                                     // 线性叠加
    order      = K_Θ_gate(order_raw, account, margin, γ̄)      // 沿用 coverage.rs:2476-2489 协变 cap
    schedule(order, exec_index)                               // 沿用 Schedule_Θ 单出口
```

不变量（设计即须承诺的可证性质）：

- **LEE-Net 恒等**：Σ_ℓ Δq_ℓ ≡ ΔN'，其中 N' = Σ_ℓ q_ℓ 为级别账本净额；与现行 ΔN 守恒（`runner.rs:1560-1566`）同型，逐级分解是 Net 的**加性细化**（线性代数，同 `overlay_state.rs:22` Fubini 对账一类，认识论 L1）。
- **稀疏性**：bar i 无 ℓ 级事件 ⟹ Δq_ℓ(i)=0——订单时点集合 = 各级事件时点集合之并，严格稀疏于 bar 全集。
- **级别封闭**：Ledger_ℓ 的持仓只由 formation_level=ℓ 的事件改变；跨级影响（如父级失效要求子级强平，ancestor_close 类）必须以**显式跨级消息**落账，不得经净额隐式传导。

### C.3 与 E2E-O Consume_at 的关系（任务④联动）

- `Consume_at` 解决的是「BSP 什么时候、凭什么谱系被授权形成」（生产侧，roadmap:62）；LEE 解决的是「授权形成的 BSP 由谁持有、按什么时钟动作」（执行侧）。二者在 `ManagedBspCreation`/`BspLink` 输出处对接。
- `event_level` 的既有定义（「运行该背驰谓词的塔桶/中枢级别」，roadmap:64；原型 :30）给事件本身标了级；`BspKey.formation_level` 给 BSP 标了级。LEE 的消费路由只读这两个既有字段，不新增口径——满足 roadmap:60「不得重命名后偷换口径」。
- 跨级形态（1 分钟背驰触发 30 分钟走势，044:16）在 LEE 下的表达：L1 账本消费该 1 分钟 BSP（formation_level=1），L3 账本**不**因此动作，除非其 policy 显式订阅带 skip edge 的谱系投影（ProjectionKey，roadmap:64）。跨级授权走谱系显式边，不走净额隐式抵消——这正是 E2E-L「不得伪造中间级别证书」纪律在执行层的同构。

### C.4 对任务④的直接回答：voice.rs 根=L* 是否已支持级别事件驱动？

一半支持，一半缺失：

- **已就位（语义层）**：根=当前最高有效决策级别 L*、最多 max_depth 层 L*,L*-1,L*-2（`voice.rs:5`）——声部树按级别分层；σ 交替按深度（`voice.rs:6-8`）；资金帽按深度加权（`voice.rs:12`）；多独立根在 StrategyFamily §5 有效域内已证允许（`voice.rs:79-89`）。级别索引与分级资金权重的**概念槽位**都在。
- **未就位（实装层）**：①v0 recognize 只产 depth=0 单声部决策，嵌套声部树未触发（`voice.rs:93-95` 诚实声明）——「根=L*」目前退化为「每个 BSP 一个独立根」；②决策出口仍是每 bar 统一 `pi_theta_step` 产净目标 p̃→p*→O（`coverage.rs:2794-2801`），没有「只在 ℓ 级事件时变」的门控；③声部账本（OverlayState）只读旁路净额主路径，pnl_v 非独立 NAV（`overlay_state.rs:27`）。

结论：voice.rs 的 L* 设计在语义上兼容 LEE（甚至可视为 LEE 的声部层前驱），但当前实装仍是 bar tick + 净额。LEE 不是对 voice.rs 的推翻，而是把它声明的级别语义兑现到执行层。

---

## D. 迁移路径：从净额执行到级别事件驱动

原则：每步保持既有不变量可验（ΔN 守恒、bit-exact 断言、E2E-S* 门不改写）；先加性细化（订单流不变），后事件门控（订单时点变，需独立验收）。照实否定任何一步均为合格结果（161 号）。

| 步 | 内容 | 不变量/验收 | 风险点 |
|---|---|---|---|
| M0 | 现状：统一 bar tick + 净额主路径 + OverlayState 只读旁路（`runner.rs:1036-1041`） | ΔN 守恒（runner.rs:1560-1566）、Σpnl_v=N·ΔP 对账（overlay_state.rs:22-27） | — |
| M1 | **级别账本旁路**：OverlayState 的 SepLeg 按 formation_level 分组，建只读 Ledger_ℓ 镜像；不改净额主路径，bit-exact | 各级账本净额之和恒等 ΔN（加性细化断言）；既有测试零变化 | SepLeg 目前是否携带可靠的 formation_level 待核（G7 候选 gamma_index 与 levels 对齐已由 coverage.rs:2376-2378 同构保证，工程上可行） |
| M2 | **订单归因改造**：物理订单改由 Σ_ℓ Δq_ℓ 生成（各级目标仍每 bar 重估），证明 Σ_ℓ Δq_ℓ == ΔN，订单流与 M0 逐 bar 相等 | bit-exact：成交序列不变，仅归因维度增加 | 浮点结合律重排误差（overlay_state.rs:22 已声明同型 eps 容差） |
| M3 | **事件门控**：Ledger_ℓ 只在 clock_ℓ 事件时点重估目标；无事件 bar 目标=前值 | 订单时点集合 ⊆ 事件时点并集（稀疏性）；需新验收：与 E2E 五钟（roadmap:75）的时点一致性实证；**本步起订单流与 M0 分叉，必须独立评审**，不得借 M2 的 bit-exact 蒙混 | L3 持仓穿越 L1 噪声期的浮盈回撤行为改变；风控门（force_flat/止损，`runner.rs:837-848` k_theta_risk_gate）须保持每 bar 生效——**事件门控只门控结构交易，不门控风控** |
| M4 | **级别 sizing/risk**：w_ℓ 资金权、级别级风险帽上线；voice.rs depth_weights 与 LEE w_ℓ 的对偶关系统一 | Σw_ℓ ≤ 1；𝒦_Θ 协变 cap（coverage.rs:2476-2489）按级别分解后仍满足 | 参数膨胀风险：w_ℓ 全属 Θ_risk，禁冒充缠论可导 |

**L0–L5 映射**（塔级别 ↔ 结构对象 ↔ 传统窗口的大致对应；对应是结构的不是时间的，**不承诺 bar 数等效**）：

| 塔级别 | 结构对象 | 地板/谱系口径 | 传统对照（粗略） |
|---|---|---|---|
| L0 | 笔/笔间类背驰 | QuasiFloor（roadmap:63；065:94「和背驰是两回事情」） | 1min 图笔以下 |
| L1 | 线段/形式背驰首层 | FormalFloor（roadmap:63） | 1min 级别走势 |
| L2 | L1 三段以上重叠构成的走势类型（1 分中枢） | events_by_level[2] | ≈ 传统「1 分钟级别」 |
| L3 | L2 递归（5 分中枢） | events_by_level[3] | ≈ 传统「5 分钟级别」 |
| L4 | L3 递归（30 分中枢） | events_by_level[4] | ≈ 传统「30 分钟级别」 |
| L5 | L4 递归（日线中枢） | events_by_level[5] | ≈ 传统「日线级别」 |

关键差异（再次强调）：传统窗口的级别 ↔ bar 数是**固定比例**（5min=5×1min），塔级别 ↔ bar 数是**结构依赖**的——一段行情的 L3 中枢可能跨 30 根 1min bar，另一段跨 300 根。LEE 的 clock_ℓ 因此是**变频率事件钟**，比固定窗口的等频钟更贴近 039:30 的段事件程式；但也意味着 sizing/风控不能假设任何级别的固定持有期——这是 LEE 相对传统多窗口必须诚实声明的有效域差异。

---

## E. 与嵌套赋格的关系：级内赋格 vs 跨级赋格

- **现状赋格是级内的**：σ_v=(-1)^depth 的交替定义在同一信号根的声部树深度上（`voice.rs:6-8`、`voice.rs:48-58`），即一个级别内部的多空交替；教义源是 039:20 的两段韵律（向上段先买后卖/向下段先卖后买）与 038/039 课的段间机械操作。
- **RNF 已摸到跨级赋格的形，但执行仍扁平**：`recursive_nested_fugue.rs:14-18` 的「根永不平多 + 子空存活/死亡 ⇒ 净多头暴露 = N − Σ子空 units，随走势结构呼吸」本质上是**跨级别头寸叠加的呼吸**；`level_operating_unit.rs` 的 RevTranche 按 level append-only（:9-10）已是级别键控的仓位槽。但它们的执行仍是每 bar 磁带驱动 + 净额（`level_operating_unit.rs:11-12`「确认级别是磁带的每 bar 纯函数」）——跨级赋格的概念在，级别事件时钟不在。
- **LEE 下的分工**：级内赋格 = Ledger_ℓ 内部的声部交替（该级 BSP 序列驱动的开/减/平/反向）；跨级赋格 = 各级账本头寸的叠加呼吸，净暴露 = Σ_ℓ q_ℓ，其形态由各级事件序列的交织**涌现**，而非由统一 p̃ 预先混合。RNF 的「净暴露呼吸」（recursive_nested_fugue.rs:17-18）在 LEE 下获得严格执行层对应物：呼吸的每一次起伏都可归因到某一级的某一事件。
- **跨级消费的教义边界**：区间套递归的是「父背驰段内部的背驰段」（027:38、027:46，roadmap:62 已冻结），LEE 中父级账本对子级事件的订阅必须经 Closed 谱系（Consume_at 只收 Closed，原型 :85）的显式投影，不得为缺失中间级别伪造证书（roadmap:64）——跨级赋格是**谱系显式边**上的协作，不是净额账户里的隐式抵消。

---

## F. 边界与纪律声明

1. 本文是设计研究，非实装承诺；伪码（§C.2）未编译、未运行，不声称任何正确性证据。M1–M4 每一步均需独立评审与验收，尤其 M3 起订单流分叉，禁止以 M2 的 bit-exact 结果外推。
2. 本文不声明 alpha、不做任何回测比较（v3 硬禁令：历史数据只验证代码正确性与不变量）；LEE 的优劣判据是**结构保真度**（级别身份是否贯穿到持仓），不是净值。
3. 「扁平化是交易结果欠佳的原因」超出本文可证范围——本文只证明：扁平化 = 执行层两处有损投影（bar tick 混合 + 净额混合）造成的级别信息丢失（§B），这是构造性事实；至于该信息保留后交易表现如何，属 L3 经验问题，需另案且不得以回测定优劣（只能验不变量与结构一致性）。
4. 引用核验：039:14/20/26/30/147、032:26/28/30/227、044:16 均经本博文本行号核对；代码锚均按 worktree 当前 HEAD 行号（runner.rs / coverage.rs / voice.rs / overlay_state.rs / level_operating_unit.rs / recursive_nested_fugue.rs）。行号随后续提交漂移时，以锚点周围的语义注释（如「净额降维」「ΔN 守恒」「根 = 当前最高有效决策级别」）为准重定位。
5. 未决问题（照实列出）：①SepLeg/ActiveLeg 当前是否完整携带 `formation_level`（M1 的前置核实项）；②clock_ℓ 事件集的最小完备定义（BSP 生灭之外是否含中枢生灭/段完成，需与 classifier 的 LevelState 输出逐字段对齐）；③风控门与事件门控的优先级形式化（M3 风险点）；④L* 的「当前最高有效决策级别」在 LEE 下是否仍需要单一根，还是让位给多级平行账本（voice.rs:5 的语义重审属另案）。
