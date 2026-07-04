# 形式化链深度研究报告（终合成）

- 日期：2026-07-04
- 工位：swarm/wf-synthesize（终合成）
- 输入：①5 主题演进综合 ②问题1.pdf 十核实点判定（含对抗验证）③40 份 PDF revisions 字段汇总
- 认识论等级：全文 L0/L1（PDF 亲读 + 源码/谱系静态核对的二次综合），零经验数据；任何 alpha/可达性 L2/L3 结论均为转引并标注
- 排序诚实声明：40 份 PDF 的 mtime 全部相同（批量导入，唯问题1.pdf 晚 3 分钟），多数修订仅注明「本对话前一版」而非跨文档编号/日期。**材料不足以给出严格全序**。本报告 §1 的阶段序依内部引用链（2026-07-01/07-02/07-03 日期锚点）+ 内容逻辑递进重建，非时间戳排序。

---

## §1 全链一页总图：推导演进主线（约 6-25 → 7-04）

整条形式化链是一个**逐层收窄有效域**的自我否定过程。被反复修订的两个膨胀结论：(A)「完整缠论无 alpha」、(B)「一类买卖点=0 是市场现实」。终点收敛为 **Π_tested ≠ Π_max-full、终局是 INCONCLUSIVE 而非无 alpha**。

**阶段1 · 分账本覆盖起点**（推导完全分类.pdf / 推导完全分类(1).pdf）
「吃到每一笔」强/弱两命题：强=事后端点无延迟全覆盖（判不可达），弱=每个已确认元素有唯一对应操作（可证）。两次内部收窄：「唯一同向声部」→「唯一规范吃笔声部 ν(b)=最深同向活动声部」；「事后端点吃满」→「吃到操作语义中已可判定的笔元素」。引入分账本头寸语义 P^sep。此阶段无 Role 分类、无互斥要求。

**阶段2 · 完全互斥引入**（推导完全互斥分类.pdf）
新增两个证明目标：分类互斥穷尽 + 策略对合法状态唯一给出动作。Role 四类 {RootDir,SameDir,SubFollow,ShortDiff}；补入「短差是相对父声部的操作角色，不是绝对方向」。

**阶段3 · 买卖点降为语法判定**（买卖点.pdf）
纲领裁定「买卖点不是最终分类，只是语法判定」。放弃 B·S=0 互斥，6 维信号 2^6=64 态完全分类；Role 四类细化为五类（SameDir 拆 SameFollow/SameReverse）；固定总序 ≺_Θ + 确定性解释器消解重合。

**阶段4 · 去根化三轴重构**（递归完全分类买卖点.pdf）
最大结构重写：根头寸/根级别不作特殊语法，仅是有限观察窗口前沿。L=Z + 边界胚元 ∂_{ℓ+1}。推翻 Root/Same/Sub 与五类 Role，改三轴 R(g)=(H,V,δ)，H∈{First,SameFollow,SameReverse}、V∈{Ambient,FollowParent,ShortDiff}。新增自相似证明 C_Θ(S_k x)=S_k C_Θ(x)。短差去根化：ShortDiff⟺σ_{p(g)}≠0∧δ_g=−σ_{p(g)}。

**阶段5 · 资本层不相容**（绝对资本.pdf）
三选二定理：尺度等变 + 固定绝对资本上限 + 全定义域策略等变不可兼得。裁定原 spec 是真矛盾；若保去根化全域自相似**必须选方案 A**（协变资本单位 + 无量纲化）。项目实际采用 A/B/C 未裁定（开放）。

**阶段6 · 合并集大成**（推导完全分类全pdf.pdf）★分类结构 living authority
合并阶段4+5，并订正元素对象：完成元素 e∈𝒯_ℓ^fin（完成走势类型单元），非 e∈C_ℓ；「三个次级别走势」=三个已完成走势类型单元满足 Overlap/Qual 条件。

**阶段7 · Role 相对化**（最高级别走势类型.pdf）★三轴之上的最新收窄
推翻 R(g) 作为元素绝对标签/单元函数，改为相对操作容器/区间套证书的关系 R_t(g;c)/R_t(γ)。δ(g) 是稳定语法属性；高级别涌现只新增包含关系不改写子方向；不可回溯只在订单层不在方向层。

**阶段8 · 结构覆盖≠alpha 的认识论降级**（alpha.pdf → 严格alpha.pdf → 缠论全互斥定义策略1/2.pdf → 过拟合.pdf → 经济正条件.pdf → alpha检验.pdf → 多级别检验.pdf → 奇偶相位.pdf → alpha分离.pdf → 级别和sigma.pdf）★认识论 living authority：买卖点alpha2.pdf + 级别和sigma.pdf
关键切割：Eat(e)/uncovered=0/G_e>0 是 L0 结构覆盖 soundness/端点定向恒真，不是盈利定理。确立 L0/L1/L2/L3 四层验收；alpha 判据=条件边际收益 μ(z,a)>0；鞅不可能定理；LCB 门 χ=1[LCB(μ)>θ]；过拟合归位（确定性分类器 C_Θ 不过拟合，过拟合在 μ̂ 估计层，Winner's curse/Le Cam 硬墙）；L3 全窗否证重判为功效不足 INCONCLUSIVE；beta 剥离（X_i 不可作检验对象，单标 BTC 残差 Y_i 数学恒等退化）；正确状态收窄为 (ℓ,δ,σ_higher)/(ℓ,q=δσ^H)，三重判据 μ̂>0∧p_perm<0.05∧LCB_OOS>0。全互斥分类被存在论重定位为「alpha 命题的形式化实验台」（可定义/可分解/可检验，非先验盈利保证）。

**阶段9 · 递归结构与身份线**（anc.pdf / 级别容器.pdf / 级别容器2.pdf / 子声部.pdf / 递归证明.pdf / on2.pdf / 多空对冲.pdf / 买卖点alpha.pdf / 买卖点2.pdf）
持仓载体=容器/位置实例，BSP 叶子只作证书（推翻引擎 open_node_id=signal_id）；AncOK 是过滤器非生成器；上级买卖点⟸次级终端证书+Lift 四条件（方向反转才严格）；orphan frontier 结构性为零定理（host 宇宙须扩到 endpoint-complete K_i）；depth>0 腿被身份层杀死非交易逻辑（四态分类，Stale⟺pid∉P_j）；π^cov≠π^bsp（结构覆盖净额投影 vs 事件状态机择时，b1 只否证前者）。

**阶段10 · GAP/技术簇**（gap.pdf / 三个gap.pdf / k的条件.pdf / 目前的缺口.pdf / 关于背驰.pdf / 区间套.pdf ≡ 回测的问题.pdf；引用 2026-07-01 文档，07-02 入仓）
「旧无 alpha 只否证 Π_simp」元裁决首次成型；区间套 rung 端点相等→区间包含 J_child⊆J_parent（bit-exact，解释 depth≥2 恒 0）；confirmed prefix immutable / mutable frontier 必重算；撤回所有非 bit-exact 增量塔的「高级别无 alpha」结论；背驰=递归力度签名 𝔉_ℓ，MACD 仅投影坐标，支配序三值；GAP3 η_⋆(x)=L^wc+κQ、κ 为风险政策参数价格不可导出；576 账本裁 C 双层并置。

**阶段11 · 整合**（完整的策略.pdf，07-02 22:54）★完整形态 living authority
𝔊_Θ^full 十三元组闭环 + 九处简化实装缺口自检表；作废 τ^reverse 收益函数改 typed exit；禁 in-sample μ̂>0/p<0.05 作判据改 LCB_OOS>0；四账本分离。

**阶段12 · 终局降级**（一类买卖点.pdf + 问题1.pdf，07-04）★顶层元裁决 living authority：问题1.pdf
推翻「一类=0 是市场事实」——两个定义级实现错误（trend_class 全历史作用链吸收锁死 + 中枢延伸缺失）；终局数学表述 Π_tested≠Π_max-full；主裁决桶须 δ-free；停用「终局/完整缠论已回测」措辞，改「五步结构修复后的 alpha 初检」；要求策略对象冻结表（已落地 TARGET_STRATEGY.md @ ac3b156eec）。

---

## §2 当前有效口径总表

### 2.1 核心对象权威表

| 对象 | 现行权威（文档+页码） | 要点 |
|------|---------------------|------|
| C_Θ 分类结构 | 推导完全分类全pdf.pdf p1-13 | 去根化 L=Z + ∂_{ℓ+1} + 三轴 R(g)=(H,V,δ) + 自相似等变 + 12 假设下 ∀x∃!O_{t+1} |
| Role 语义 | 最高级别走势类型.pdf p1-2,p11 | R 须实装为关系 R_t(g;c)/R_t(γ)，禁单元函数绝对标签；δ(g) 稳定 |
| 元素对象 | 推导完全分类全pdf.pdf p15-17 | e∈𝒯_ℓ^fin=(I,P,τ∈{U,D,P},Z,∂)；三个次级别走势=三个完成单元+Overlap/Qual |
| z 状态向量 | 完整的策略.pdf p4/§16 | 20 维含 Jchain 不含 d；β/c/m 收入（ForceState/CostBucket/MarginState），d 归风险层。**材料不足**：§16 字面页码本轮提取未隔离，判定依赖 p4 定义 + MEMORY 持久锚点交叉确认，需字面须回读原 PDF |
| Π 策略对象 | 完整的策略.pdf p1,p10 + 问题1.pdf p7,p9-15 + TARGET_STRATEGY.md | 𝔊_Θ^full 十三元组；三对象 Π_signal/Π_exec/Π_treasury + MUST/WAIVED/OUT_OF_SCOPE 冻结表；本轮 Π_target=Π_signal-full |
| alpha 判据 | alpha.pdf p1-14 + 经济正条件.pdf p10-17 + 严格alpha.pdf p5-6,p23-25 + 级别和sigma.pdf p9-12 + 过拟合.pdf p10-12 | μ(z,a)>0 最一般经济正条件；L0-L3 四层验收；鞅不可能定理；三重判据 μ̂>0∧p_perm<0.05∧LCB_OOS>0；功效不足=INCONCLUSIVE 非否证 |
| 主裁决桶键 | 问题1.pdf p11,p14 | δ-free Z_decision=(level,bsp_class,parent_dir)；含 δ 4 元组仅报告桶。实装现状：主程序仍含 δ（已确认工程 gap，见 §3 问题F） |
| 区间套/frontier | 区间套.pdf ≡ 回测的问题.pdf p3-12（整合版优先）+ anc.pdf p7,p24-25 | rung=区间包含 J_child⊆J_parent；confirmed prefix immutable / mutable frontier 必重算（中枢塔层与持久元素层同一原则）。MEMORY：已落地 bit-exact 差异 0、parity 固化 |
| 背驰力度 | 关于背驰.pdf p8,p11-12 + 三个gap.pdf p13 | 递归力度签名 𝔉_ℓ，MACD 仅投影坐标；支配序 ≺_𝒜 三值 {Yes/No/Incomparable}；Cand≠Conf 宽结构候选 |
| 资本层 | 推导完全分类全pdf.pdf p19-25 / 绝对资本.pdf p9 | 三选二不相容定理；欲保全域自相似必选方案 A（协变资本单位+无量纲化）；**项目实际采用哪案未裁定（开放）** |
| GAP3/资金层 | k的条件.pdf p3,p9-11 + 目前的缺口.pdf p7-10,p15-18 | η_⋆(x_t)=L^wc_{t+1}+κQ_t 状态依赖 barrier；κ 为风险政策参数价格不可导出；三层验收；576 账本 C 双层并置 |
| 持仓载体 | 级别容器.pdf p11-15 + 级别容器2.pdf p1,p3-4,p11-12 | 持仓节点=carrier/位置实例，BSP 叶子只作证书；AncOK 过滤器非生成器；父声部须归属该 carrier 的 accepted certificate |
| 身份持久性 | anc.pdf p6-8,p17-19 | 四态 {LivePresent,LiveDetached,Closed,Invalidated}；Stale⟺pid∉P_j 非 ∉E_j；held 腿挂 op_parent |
| 递归提升 | 递归证明.pdf p2-4,p9,p11 | 上级买卖点⟸次级一买/卖+Lift(Host∧Nest∧Context∧Fresh)；级别平移等变 |
| 子声部 host 宇宙 | 子声部.pdf p15-20,p24-25 | 全格 BSP 参与须扩到 endpoint-complete K_i（Γ_i^K≅B_i）；否则结构性为零是定理非 bug |
| 收益函数/出场 | 完整的策略.pdf p6,p8 | typed exit X^full（τ^typed≠τ^reverse）；可交易判据 LCB_OOS>0，禁 in-sample μ̂/p<0.05 |
| 终局措辞 | 问题1.pdf p1,p7,p14-15 | Π_tested≠Π_max-full；「五步结构修复后 alpha 初检」；三态 VALIDATED/FALSIFIED/INCONCLUSIVE |

### 2.2 已作废口径清单（防误引，合并去重）

1. 「唯一同向声部」→ 唯一规范吃笔声部 ν(b)。
2. 「事后端点吃满每一笔」可证 → 收窄为「已可判定笔元素」。
3. Role 四类/五类、Root/Same/Sub 三分类 → 三轴 (H,V,δ)。
4. 根头寸/根级别特殊语法 → 去根化前沿 ℓ_max(t)+∂_{ℓ+1}。
5. 「短差=根多头里的次级别空头」→ 去根化 ShortDiff⟺δ_g=−σ_{p(g)}。
6. R(g) 绝对标签/单元函数 → 关系 R_t(g;c)。
7. e_i∈C_ℓ → e_i∈𝒯_ℓ^fin。
8. 买卖点互斥 B·S=0 → 64 态完全分类；cls_Θ 压扁 I_γ → 保留全集。
9. 「结构完备/覆盖完备/Eat(e)⇒盈利」→ L0 soundness，非 L3。
10. 「端点价差扣成本为正=唯一经济正条件」→ 最原子路径级条件之一；「顶高于底⇒因果正收益」被反例否定。
11. 实测归因 b2 → b1；「b1 否证买卖点择时」→ 只否证 π^cov。
12. 「G_e>0 是择时 alpha」→ 端点定向语法恒真。
13. 买卖点=结构覆盖持续腿 → 事件状态机转移 π^bsp。
14. 「一类买卖点=0 是市场事实/几何环颈」→ 两个定义级实现错误。
15. 「完整缠论无 alpha/已回测否证」→ 仅对 Π_simp 有效；「L3 全窗否证」→ 功效不足 INCONCLUSIVE。
16. 基于非 bit-exact 增量塔的「高级别无 alpha」→ 撤回/标注 frontier-bug 污染。
17. 区间套 rung 端点相等 s=ρ(m) → J_child⊆J_parent；point-contains 也不够严格。
18. 增量塔未确认窗口纳入 confirmed prefix → frontier bug。
19. Stale⇒parent:=None 伪造根 → 诚实 prune + 四态分派；Stale=∉E_j → ∉P_j。
20. open_node_id=signal_id(g)（叶子作持仓）→ carrier+position node。
21. AncOK 生成语义（子自动开父）/registry_alive 代 active → 过滤器语义；命题 Q 强形式被推翻。
22. 「任意次级一买/卖⇒上级买卖点」→ 方向反转+Lift。
23. goal #5「全格 BSP 子声部 alpha」→ wrong object（orphan frontier 被 P1∧P2 排除）。
24. 奇偶交替=独立 alpha / 「只做顺上级」/ beta-alpha 命名旧版 / 「负类反做」/ 单标 beta 残差 Y_i → 全部降格或否证。
25. z=(B/S) 裸态、z=(ℓ,δ) 分桶、多空/级别池化 → 全维化+拆分。
26. X_i 方向签名 PnL 作检验对象 → beta 剥离残差；X^old=τ^reverse 出场 → typed exit。
27. MACD 一票否决 canonical → 选择偏差；单级 DivCand 称「区间套已接入」→ 只能叫 DivCandGate。
28. INV-2「cost_basis≤0⇒EarningShares」→ 反向蕴含；η_⋆ 硬编码常数 → 状态依赖 barrier。
29. 含 δ 报告桶做主裁决 → δ-free 三元组（δ 入桶键与 i_class 共线自毁置换）。
30. 原 spec 尺度等变+绝对资本共存 → 三选二不相容，资本层必改。
31. 「终局」一词 → 「五步结构修复后 alpha 初检」。

---

## §3 问题1.pdf 十裁定点判定表

判定来源：材料2（十核实点 + 对抗验证）。全部 L0/L1 静态核对。

| # | 核实点 | verdict | 证据摘要 | 对抗验证 |
|---|--------|---------|---------|---------|
| 1 | A11 声部执行层未接真实开平仓 | GAP_CONFIRMED | overlay_net_delta/separate.rs 全仓无生产调用者，真实下单路径（StrictAction→OrderIntent）与 P^sep/ΔN 正交；674/642/646 裁定均有效；性质=YAGNI/有效域裁剪非缺陷。措辞修正：「被 674+646 阻断」应为「有效域裁剪+非同构约束需独立 OverlayState 账本」 | 无反驳，成立 |
| 2 | A10 waiver ⟹ 回测对象=R_no-margin | ALIGNED | margin=None 默认 M1/M2/M3 不可达；PnL 成本仅 commission+slippage+tax，Funding/Borrow/LiquidationLoss 永不入账；与 693 号「A10=OUT_OF_SCOPE、cash-settled no-margin netting」逐字对应。小瑕：M2/M3 机制层已实装（#135 零 binding），但被检验对象维度 ChatGPT 正确 | 无反驳，成立 |
| 3 | d 未进 z 状态 | GAP_REFUTED（**对抗争议→建议按 TENSION 采信**） | sizing 层已按 1/d 归一化（risk.rs:147 qty∝ρ·NAV/|entry−stop|）；§16 权威+690 号裁 d 归风险层 | **反驳成立**：核实存在范畴滑移——问题C 承重支是 estimand μ_R(z)=E[X/d\|z]（d 不进桶键，690 三理由不适用），核实自认「当前测原始 μ 非 μ_R」为真且未实装（690 边界条件(a) 活预注册选择）。降级标注：sizing 支 ALIGNED 属实，estimand 支为真实开放项，整体应为 TENSION，**上浮编排者**（详见 §3.1） |
| 4 | ForceState 分层键未真进主裁决桶 | GAP_CONFIRMED | ForceStateA5 已入 z 第 8 维（selector.rs:264）；但主裁决 stratified_delta_perm_p 桶键无 force_state，ThetaScoreBin/β_bin 未接 ResidualTrade（a2 登记诚实缺口，只差 ws-etab 一行装配+δ-共线检查）；fullz 分支含 force_state 但非默认主裁决 | 无反驳，成立 |
| 5 | A4 AncOK stale 伪造 parent=None 仍待裁 | GAP_REFUTED（**对抗部分翻转→机制证伪、实质 ceiling 幸存**） | 代码四态分派不伪造（None 仅真边界根）；codex 条目6 已判 A4 销项 | 两轮对抗：第一轮反驳失败（file:line 全部属实）；第二轮反驳成立——「A4 未闭合时声部树完备性非完全证明态」这一承重结论为真（persistent.rs 自认最小修复/彻底修复标 ceiling、L2/L3 待验证），且 codex「应另开条目跟踪」是未来动作未验证已开。降级标注：机制断言（伪造）与「A4 待裁」确被证伪，但 **ceiling 缺口幸存（情形c），须验证跟踪条目是否已开**（详见 §3.1） |
| 6 | δ-free 主裁决应由主程序直接输出 | GAP_CONFIRMED | 主程序 wverify_full/bucket_verdict 仍以含 δ 4 元组算 verdict；#186 只交付离线 dump+精确重算（明文「不进判定路径」），恰是问题F 承认的「离线合并」半；严格半（在线 δ-free 主裁决键）未闭合。与终局 INCONCLUSIVE 正交，不否证任何 alpha 结论 | 无反驳，成立 |
| 7 | 高级别定方向+低级别执行统计 | GAP_CONFIRMED（三分定性） | 交易执行层已有等价物（route_bsp top-down 方向传导）；alpha 统计层为全新缺口（三套桶键 level 全是独立分层维，无跨级 pooling）；σ^H 半个零件已在（塔真值填第 9 维）但被 codex-q1 G2 裁定不进桶键。落地成本低于从零，需新 prereg（i_class×δ 共线教训在案） | 无反驳，成立 |
| 8 | 13 条概念裁决会改变 C_Θ/Γ_t | GAP_REFUTED（**对抗争议→建议部分翻转**） | 分层清单：689/690/691/688=出处/权威订正不阻断；678=真概念但代码已落（#44/#179 gate_pass）；680=前提 stale（Z 分桶已落）；615/681=Lean/命名层；644=实装 bug；692=伪缺口；576=唯一真扩 Γ_t 但未进生产且已正确挂 pending | **反驳成立**：核实自认 678「确实改了 Γ_t 准入门」、576「唯一真正会扩 Γ_t」——问题H 字面命题「是否会改变 C_Θ/Γ_t」对此两条为肯定；678 边界条件③（CHECK_FAIL 可召回）自证概念仍开放；680 消解为 L1-only 未穷尽消费点。降级标注：「谱系待 /ritual≠分类未闭合」的层区分批评成立，但对 678/576 的字面回答是「会」，整体 GAP_REFUTED 过硬（详见 §3.1） |
| 9 | 三路线 A/B/C + §5 十件事对照 | ALIGNED | 框架已落地：TARGET_STRATEGY.md 冻结表存在（ac3b156eec）+ 有效域定理文档；两条任务前提被推翻（TARGET_STRATEGY 已存在；GAP3 Stage III L1 已可达——利润桥 runner.rs:2439 assert 通过，推翻 MEMORY「L2 也不可达=架构」旧归因）。剩余为路线内实装收口（见 §4） | 无反驳，成立 |
| 10 | Π_tested≠Π_max-full 数学严格性 + 「终局」措辞 | GAP_CONFIRMED（窄口径） | M* 反例的 ⇏ 论证严格成立（模态非蕴含只需逻辑可能）；报告三态纪律/口径限定已 ALIGNED；真缺口=「终局」标签（final-alpha 标题/§0/正文三处）+ 缺显式「执行层/资金层未闭合」作用域免责句。修法为纯文档改名+加一句，不触及 alpha 数字 | 无反驳，成立 |

**汇总**：GAP_CONFIRMED×5（#1/#4/#6/#7/#10）、ALIGNED×2（#2/#9）、GAP_REFUTED×3（#3/#5/#8），其中三条 GAP_REFUTED 全部遭遇对抗反驳且反驳内容实质成立或部分成立。

### §3.1 对抗争议项详述（上浮编排者）

材料内在歧义诚实声明：材料2 的 `survives` 布尔标志与所附 refutation 文本语义存在冲突（#3 survives=true 但反驳主张改判 TENSION；#5 survives=false 但两轮反驳一败一成；#8 survives=true 但反驳主张 GAP_REFUTED 不成立）。该标志语义在本材料内 **UNRESOLVABLE**，以下按 refutation 文本实质内容采信较弱结论，最终定性属选择类，须编排者裁定。

**争议一（#3，d/μ_R estimand）——建议改判 TENSION**
- 事实两清：sizing 层 1/d 归一化已实装（无争议）；风险调整 estimand μ_R(z)=E[X/d|z] 未实装且核实自认为真（无争议）。
- 争点：μ_R 是「可分离的预注册选择/边界条件」（核实方）还是「问题C 结论『风险归一化未完全实现』的直接内容」（反驳方）。690 号边界条件(a) 本身把「风险感知 μ」标为待编排者预注册的新决策=四分法「选择」类。
- 上浮问题：是否开 μ_R 新预注册工位（测原始 μ 还是风险调整 μ_R 作为 estimand）。

**争议二（#5，A4/persistent ceiling）——机制证伪成立，剩余缺口幸存**
- 已定：Stale 分支不伪造 parent=None（四态分派，None 仅真边界根）；A4 已被 codex 销项。
- 幸存：persistent.rs 自认「最小修复」，彻底修复（增量 extract_elements/confirmed prefix immutable 元素层）标为 ceiling 且 L2/L3 待验证；codex「应另开条目跟踪」是建议动作，**是否已开条目未经验证**。
- 上浮问题（行动类可自决部分已明确）：核实 ceiling 跟踪条目是否存在，不存在则开条目——此半可直接执行；「彻底修复是否列入本轮」属选择类待裁。

**争议三（#8，13 条概念裁决）——层区分批评与字面肯定并存**
- 已定：「谱系待 /ritual ≠ 分类函数未闭合」的存在论层区分成立；689/690/691/688/615/681/644/692 确不阻断。
- 幸存：678 的 C3 门改动是真概念层且可被 CHECK_FAIL 召回（概念仍开放）；576 转折节点 E⊗F 若被 /ritual 采纳将真扩 Γ_t（E0 未实装）；680 的消解仅 L1 静态、未穷尽消费点。
- 上浮问题：576/E0 是否列入生产 Γ_t（选择类）；678 的 /ritual 形式结算（行动类）。

---

## §4 三条路线落地对照表（不做价值裁决）

来源：材料2 第 9 项对照（L0/L1 静态）。依赖序原则：路线内无数据依赖项并行；A 先于 B；C 与 A/B 无数据依赖可并行（有效域定理1 保证互斥）。

### 路线A · Π_signal-full（本轮 Π_target，TARGET_STRATEGY.md:61）

| 构件 | 现状 | 剩余量级 |
|------|------|---------|
| P1 区间套 | 已实装（nest.rs/descend.rs，bit-exact parity） | 深度触达 L2 验证 |
| P2 ForceState 入主桶 | selector.rs:264 已真进 z 第 8 维 | 已收口；残 SubMovePower 5-proxy 精度（非阻塞）+ ThetaScoreBin/β_bin 分层键装配（ws-etab 一行+δ-共线检查，小） |
| P4 六类买卖点 | 已接，type1 全级别>0 已修 | — |
| P7 typed exit | 已接（ExitType 入桶） | — |
| (i) d 入状态/μ_R estimand | MuClass 无 d 字段，μ_R 无实现；sizing 层 1/d 已有 | 中（且含 §3.1 争议一的选择类前置：测 μ 还是 μ_R） |
| (iii) δ-free 在线主裁决 | ResidualTrade 载体已建；主程序仍含 δ 桶+离线合并（#186 只闭离线半） | 中 |
| (iv) AncOK/身份彻底修复 | persistent overlay 缓解态，三选一（增量 extract/持久 carrier/证明可恢复）未定 | 中-大，含概念裁定 |
| 高级别定方向+低级别统计 | 执行层已有（route_bsp）；统计估计量层全新，σ^H 已算但不进桶键 | 中，需新 prereg（纵向池化；与跨标的横向池化正交） |

(i)(iii)(iv) 相互独立可并行；全部完成后方能跑 signal-full 首轮 OOS（PDF §5 第 10 条）。**量级=本轮可闭合，最小剩余路径。**

### 路线B · Π_exec-full（A 完成后的下一 goal）

| 构件 | 现状 | 剩余量级 |
|------|------|---------|
| A11 声部执行层接真实开平仓 | 仅诊断层（overlay_net_delta 只读、separate.rs 无调用者）；需独立 OverlayState 账本（674 非同构约束） | 大（架构级新增），未启动 |
| A10 保证金/强平/资金费/ADL | M2/M3 机制已装但默认 None+零 binding；Funding/Borrow/LiquidationLoss 不入 PnL | 大，未启动；依赖序在 A11 后（PDF §7 排 #2） |

### 路线C · Π_treasury-full / GAP3（可与 A/B 并行独立验）

| 构件 | 现状 | 剩余量级 |
|------|------|---------|
| Stage II/III 可达性 | L1 已可达（利润桥 runner.rs:2439 assert；推翻「架构不可达」旧归因）；L0 同价恒 count=0（合法情形） | 中：L2 真实数据可达性证人（真实 BTC 变价触发频率） |
| 终态判据 Q_T>Q_0∧W_T≥I_0∧η_T≥L^wc+κQ_T | κ 待外部风险政策四选一 | 选择类待裁（不计工作量） |

### §5 十件事映射（PDF 问题1 p13-15）

1 冻结表=已完成；2/3 A11/A10 硬裁决=693 pending 待 /ritual 追认（工作口径已冻结）；4 d 入状态=未做（争议一前置）；5 ForceState 进桶=已收口；6 AncOK 收口=未收口；7 δ-free 逐笔直接输出=收口中；8 高低配统计=统计层未实装；9 三态化=已入冻结表；10 停「终局」词=**未完成**（final-alpha 三处「终局」+缺免责句，见 §3 第 10 项，纯文档修复）。

---

## §5 结果包（六要素）

**结论**
形式化链 40 份 PDF 构成一条自我否定收窄链，当前有效口径收敛为：①分类结构=去根化三轴 R(g)=(H,V,δ)+Role 相对化+元素 e∈𝒯_ℓ^fin（推导完全分类全pdf + 最高级别走势类型）；②z 状态=完整的策略.pdf §16 二十维（含 Jchain 不含 d）；③alpha 判据=μ(z,a)>0 + L0-L3 四层验收 + 三重判据 μ̂>0∧p_perm<0.05∧LCB_OOS>0，功效不足=INCONCLUSIVE 非否证；④主裁决桶=δ-free (level,bsp_class,parent_dir)；⑤终局元裁决=Π_tested≠Π_max-full，当前只是「五步结构修复后的 alpha 初检」（问题1.pdf）。问题1.pdf 十核实点判定：GAP_CONFIRMED×5（A11 诊断层边界、ForceState 分层键、δ-free 在线主裁决、高低配统计估计量、「终局」措辞膨胀）、ALIGNED×2（A10=R_no-margin、三路线框架）、GAP_REFUTED×3——但三条 REFUTED 全部遭对抗反驳且反驳实质成立/部分成立，按较弱结论采信：#3(d/μ_R) 应为 TENSION、#5(A4) 机制证伪但 persistent ceiling 缺口幸存、#8(13条) 对 678/576 的字面回答是「会改变 Γ_t」。三争议项须上浮编排者（μ_R estimand 预注册选择、ceiling 跟踪条目核实、576/E0 是否进生产 Γ_t）。三路线剩余量级：A=本轮可闭合（d/δ-free 在线/AncOK/高低配四项，前两中一中-大一中）、B=大（A11+A10 架构级，A 后启动）、C=中（L2 真实利润 witness，可并行）。

**定义依据**
①living authority 判定依「后文档显式修订前文档」的引用链（材料1 五主题 living_authority 字段逐条给出文档+页码）；②十裁定点判定依材料2 的源码 file:line 静态核对（coverage.rs/wverify_run.rs/mu_estimator.rs/perm_test.rs/risk.rs/runner.rs 等）+ 谱系 settled/pending 原文（674/642/646/690/693/codex-gap2rulings 等）+ PDF 亲读页码；③alpha 口径依 formalization-validity-domain 规则的 L0-L3 分级（本报告全部 L0/L1，L2/L3 数据为转引）。

**边界条件（结论翻转）**
①若 wverify_full/bucket_verdict 裁决键改为 δ-free 三元组 → 问题F 从 GAP_CONFIRMED 转 ALIGNED；②若 ResidualTrade 完成 β_bin 装配+δ-共线门且默认桶键升含 ForceState → 问题D 缺口关闭；③若编排者 /ritual 翻转 693（A10→MUST 对 signal-full）→ 路线A 定义扩张本轮不可闭合，§4 对照表须重画；④若 #148/#149 后 Stale 四态分派被改使非边界根产生 parent=None → #5 机制证伪翻转；⑤若 576/E0 被采纳进生产 → Γ_t 候选集从「已挂 pending」升为「真阻断完备性」；⑥若 L2 复跑发现 econ_positive 存在绕过 z 的消费路径 → 680 前提部分复活，#8 判定弱化；⑦本报告排序依内部引用链非时间戳，若后续发现跨文档显式日期证据与本序冲突，§1 阶段序须修订。

**下游推论**
①三条对抗争议项（μ_R estimand 选择、A4 ceiling 跟踪条目、576/E0 采纳）是 /escalate 候选，其中「核实 ceiling 条目是否已开、未开则开」为行动类可直接执行；②「终局」措辞修复（final-alpha 改名+加免责句）为定理/行动类纯文档修复，不触 alpha 数字；③type1 修复后所有 type1=0 下游结论重跑、非 bit-exact 增量塔「高级别无 alpha」撤回标注，均为既裁未尽事项；④路线A 四项收口是 signal-full 首轮 OOS 的前置；⑤§2.2 三十一条作废口径清单应作为未来报告的防误引对照表。

**谱系引用**
直接相关：674（R/TW 不同构 C 双层）、642（host key/ΔN 机制层）、646（units≠q_v 范畴）、690（两 PDF z 形态权威+d 层归属）、693（A11/A10 硬裁决三分冻结，pending 待 /ritual）、688/689/691/692/678/680/681/615/644/576（settle-sweep 维持 20 中的概念型条目，分层判定见 §3 #8）、663/667（LCB≤0 不外推 μ≤0）、231（声明膨胀/诚实缺口登记）、090/161（严格性/务实否定——「终局」措辞膨胀的裁定依据）。MEMORY 交叉锚点：两 PDF z 形态冲突、GAP3 L2 不可达旧归因（本报告依 runner.rs:2439 利润桥订正为 L1 已可达）、q4 π^full 终判、goal type1 CLOSED。不确定谱系：材料2 未给 547/638（orphan frontier 路线风险标签）的谱系文件路径，仅转引子声部.pdf p7，明确说明未核原文。

**影响声明**
本报告为纯综合产出，未改任何代码、未跑任何测试、未产生新数据；落盘 1 个文件（.chanlun/review-results/formal-chain-deepresearch-20260704.md）。它改动的是认知层：①确立 §2 权威表为形式化链的当前唯一引用口径；②把材料2 三条 GAP_REFUTED 降级为争议态（对原核实结论的削弱，须编排者最终裁定）；③订正两条任务前提（TARGET_STRATEGY.md 已存在；GAP3 Stage III L1 已可达）。不影响任何已 settled 谱系的效力；对 693 pending 无新增裁定，仅汇总其待 /ritual 状态。
