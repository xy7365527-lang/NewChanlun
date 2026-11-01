# #1598 定向返修草稿

基线 main：13ed4d45c8ff85175819ca987690328ab2a247a4。沿用主控评论 6395646381；待非作者独审及 P 类批准。受影响代码附件：affected-code.md。

## 问题

票面 A-01/A-09/A-10/B-05 四条审计项（#1586 自由度审计）均将本仓量档、成本与历史模拟正本中的单值字段或「被否」候选标记为 `[新缠论:选择]`，但正本并未把它们落为可选值字段，违反 ADR 0041 自由度通则裁定三（可调字段＋默认值）与裁定四（另一种合法取值保留为可选值，不写成「被否」）。本票改写四份正本（ADR 0017 量档分子、ADR 0034 量档政策、ADR 0035 成本门、ADR 0033 历史模拟），将真正的自由度落为具名版本化记录字段，保留合法候选并写明代价，排除真错候选并写明排除依据。

## 权威依据

- docs/chanlun/text/blog/035-第35课.md:30【正文，界:52】「级别越小，平均的买卖点间波幅也越小，因此，那些太小的级别，不足以让交易成本、交易误差等相对买卖点间波幅足够小，这样的操作，从长期的角度看，是没有意义的。」
- docs/chanlun/text/blog/040-第40课.md:20【正文，界:24】「每一重都对应着一定的资金与筹码」
- docs/chanlun/text/blog/049-第49课.md:62【正文，界:78】「其后，中枢已经完成就无所谓中枢震荡了」
- ADR 0017 docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:147 明写 δ=0.5、文献带 0.4–0.7 作敏感性区间
- ADR 0017:157/:176 明写对数形式作为在案反面证据，登记不推翻，未算齐次性
- ADR 0034 docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:29 明写触发量按 units_g 或 cash_g 两口径、开重时冻结，市值不触发
- ADR 0034:39 被否方案列探针容量自动准入（否决理由：x/Y/σ 口径未裁）
- ADR 0035 docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md:12 明写 pairing=ROUNDTRIP
- ADR 0035:36 明写 ROUNDTRIP 溯源标签 `[新缠论:推论]`（#907）
- ADR 0035:27 被否方案列 AMP_EXEC 配对
- ADR 0033 docs/adr/0033-history-simulation-bar-batch-model.md:12 明写 model_id `history-sim/bar-batch-known-frontier/1`，v1 确定性，seed: none
- ADR 0033:23 明写不从 bar volume 推可用量
- ADR 0040 docs/adr/0040-funding-signature-and-chong-derivation.md:135 已签容量域 {1m, δ=0.5, x=10%, Y=0.9, σ=GK 中位日, 单腿}
- ADR 0040:309 被否方案重申探针容量自动准入被否
- ADR 0041 docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md:17-24 裁定三：落法为版本化记录字段＋默认值（签署门字段默认 AwaitingPolicy）＋出处＋改签即新版本
- ADR 0041:27-30 裁定四：另一种合法取值保留为可选值、写明代价与适用条件，不删除不写成「被否」；真正错误（违反原文/违反已裁推论/被实测否证/属 #799 兜底形态）照旧排除
- CONTEXT.md:37 _Avoid_ policy 词条：用账户权益或市值当 Q 或容量
- #907 resolution 评论5194145006 明写 ROUNDTRIP 三步推理链与否证口（原件 evidence/907-resolution-5194145006.txt）
- #1353 resolution 评论 6149639079（gh api repos/xy7365527-lang/NewChanlun/issues/1353/comments，作者 xy7365527-lang）第二问选 A：部分成交只来自具证容量合同，不从 bar volume 推可用量
- #1677（已关闭）落 ADR 0022 修订节为 evidence_gate 记录化，v1 默认值为现行值，v2 起须签署

## 候选

只向用户呈现 B 类 Q2/Q6/Q7；Q1/Q3/Q4/Q5/Q8 的已裁默认沿用。以下 A 类的历史问法是来源说明，不是再次提问。

**Q1 [A]** A-01(a) δ 的可选值集合怎么写——值域是文献带区间 [0.4, 0.7]（敏感性研究区间，不新增界外拒收门）还是离散集，默认 0.5；以及改 δ 的代价是否明写「量档 w 与容量 C(ℓ) 两表同时作废须重签」

**出处**：ADR 0017:147 δ=0.5、文献带 0.4–0.7；ADR 0034 裁定一 δ 已是 tiers.Measured 与 capacities.domain 的记录字段；ADR 0040:135 P0 已签 δ=0.5；ADR 0041 裁定三第 2/3 款

**推荐值**：字段说明写 δ 可选值 = 文献带 [0.4, 0.7]（整数对记录；文献带不自动构成合法值硬边界）；默认 0.5（出处 ADR 0017:147 三支持文献＋ADR 0040 已签）；代价栏写 同一波幅比的示例：固定 w(0.5)=0.32，则 w(0.4)=0.2406786、w(0.7)=0.4431346（分别−24.7879%、+38.4796%）与「改 δ ⟹ tiers+capacities 两表同时重签新版本」；运行态默认按 B-10 走 AwaitingPolicy（δ 属容量域签署字段）——**按本点分类处置：A 沿用，B 确认或否决**

**Q2 [B]** A-01(b) `impact_form ∈ {power(δ), log}` 挂在哪个记录、`log` 如何标「待标定」——只挂 tiers[].source.Measured 还是同时进 capacities[].domain；`log` 选中时记录落什么状态

**出处**：ADR 0017:157/:176 明写对数形式登记为在案反面证据、不推翻；对数不齐次未算；ADR 0034 裁定一「来源域与 profile 不一致即 Unmeasured」；AUDIT A-01 + review-round2 第 10 项

**推荐值**：`impact_form` 同时进 `tiers[].source.Measured{…}` 与 `capacities[].domain{…}`，默认 `power(δ)`（出处 ADR 0017 裁定六，三支持文献）；`log` 列为可选值并标「待标定」：选中时 tiers 记 Unmeasured、C 记 Unknown（⟹ AwaitingPolicy，不伪造），字段说明写代价「约化未算，原文推测不约（0017:176 未算），且 Y 在聚合 Bar 通道标不了（0017:151，#917）」；是否另开 research 票查逐笔数据源可得性不在本票，只留指针——**按本点分类处置：A 沿用，B 确认或否决**

**Q3 [A]** A-09(a) `capacities[].source` 列 `ProbeAuto` 为可选值——其前置「x/Y/σ 口径签署」在 ADR 0040 P0 已满足，但 ADR 0040 被否方案同日又重申「探针容量自动准入」被否；ProbeAuto 是作可选值保留，还是按「签署门字段」维持排除

**出处**：ADR 0034 裁定三（新探针读数是版本触发）与裁定四（容量须用户签署，否决理由「x/Y/σ 口径未裁」）；ADR 0040:135（口径已签）与:309（同日重申被否）；ADR 0041 裁定三第 2 款（容量是签署门字段）；ADR 0017 裁定七（有效域 L1–L2）

**推荐值**：保留为可选值：`source ∈ {UserSigned(可引探针), ProbeAuto{probe_run_ref, domain_ref→已签域}}`，默认 UserSigned；前置两条：①该 profile 的 {粒度, δ, x, Y, σ 口径} 已由用户签署（P0 已满足，ADR 0040:135）；②探针输出满足 ADR 0022 裁定七窗口纪律并带不确定性带（#1444 待补，当前不满足）；适用域只限 ADR 0017 裁定七干净区间 L1–L2，其余级别仍 Unknown；代价写「容量随探针读数漂移，每次新读数即新版本、在途重不重测」。ADR 0040:309 该项按本票口径改写为「可选值，前置未满足」，排除类别不再是「被否」——**按本点分类处置：A 沿用，B 确认或否决**

**Q4 [A]** A-09(b) `overflow_trigger ∈ {units, capital, mark_value}` 默认 units——{units, capital} 在 ADR 0034:29 已是「开重时冻结口径」的二选一；`mark_value` 是作可选值保留，还是因 CONTEXT.md「量档 policy」词条 _Avoid_「用账户权益或市值当 Q 或容量」而维持排除

**出处**：ADR 0034 裁定六:29（units/capital 二口径已存在、开重时冻结；市值不触发）；CONTEXT.md:37 量档 policy 词条 _Avoid_

**沿用要求与实施前置**：`overflow_trigger ∈ {units, capital, mark_value}`，默认 `units`。开重时冻结选择的口径及版本，改口径须新版本，不因某次未触发而切换。`units` 以 `Q + units_g` 的数量观测量与已签数量阈值比较；`capital` 以 `q_ref + cash_g` 的资金观测量与同币种阈值比较；冻结的是基值/口径，不是增长量。`mark_value` 保留为可选值，以当前数量和该 bar 获知价格计算市值观测量；价格来源、时间及阈值换算必须在具名版本中明确，比较两侧须同量纲，不能把市值直接与数量 C(ℓ) 比较。现有源未给这套换算，#1446 实施前必须明确并验收；不在本稿虚拟填值。其代价是对价格波动敏感，可能频繁触发，不改变 Q 或容量本身的定义。CONTEXT.md 的禁令不能用来删除该可选值。

**Q6 [B]** A-10(b) 第二层判据加 `rule ∈ {order_relation, share_threshold(θ₂)}`——默认 order_relation；`share_threshold` 的 θ₂ 是否按签署门字段处理（默认 AwaitingPolicy、读数前预登记）

**出处**：ADR 0035 裁定四:21 与被否:27（占比阈值无错误证据，只「不填占比」）；ADR 0035:36 其余 `[新缠论:选择]`；#1354 Q4；ADR 0041 第三条第 2 款 B-10；#1677（evidence_gate 预登记先例）

**推荐值**：`second_layer.rule ∈ {order_relation, share_threshold{theta2 整数对}}`，默认 `order_relation`（出处 ADR 0035 裁定四，无需签数）；`share_threshold` 列为可选值，θ₂ 为签署门字段（默认 AwaitingPolicy{theta2}，推荐值进签署单），代价写「多签一个数＋须在读数前预登记版本（挂 #1677 evidence_gate 规则）」；统计方法及分层按裁定六完整推荐一并待P确认；两值共同前置 S4-a/S4-b 修完——**按本点分类处置：A 沿用，B 确认或否决**

**Q7 [B]** B-05(a) 随机微事件模型改为「可选值（待标定）＋代价」挂在 ADR 0033 模型 id 下——集合怎么写、`seed` 与 ADR 0038 双跑逐位确定性验收如何相容

**出处**：ADR 0033 裁定一:12（model_id/seed/input_domain 字段、「为将来版本留接口」）与溯源:53；#1353 resolution（否决理由=缺标定，与 ADR 0018:82 k-tick 同理）；ADR 0038:29 双跑五元组含 seed

**推荐值**：ADR 0033 字段说明：`model_id` 可选值集合 = {`history-sim/bar-batch-known-frontier/1`（默认，确定性，seed none；出处 ADR 0033 裁定一），`history-sim/stochastic-microevent/<rev>`（待标定：概率与延迟需逐笔/盘口数据，#917 查实本仓无；须 seed≠none，验收改同 seed 复现；代价「bar 内路径是造出来的，报告须单列」）}；`seed` 字段：v1 确定性取 none，随机模型版本取整数；ADR 0038 双跑固定 seed 作五元组之一，同 seed 逐位复现——**按本点分类处置：A 沿用，B 确认或否决**

**Q8 [A]** B-05(b) 「从 bar volume 推可用量」改为「可选值（待标定）＋代价」挂在 `capacity_source` 下——与 ADR 0016 容量门、#1350 投影清单的关系

**出处**：#1353 resolution 第二问选 A：部分成交只来自具证容量合同，不从 bar volume 推可用量；ADR 0033:44 `capacity_source` 进版本合同；ADR 0016 裁定三容量门

**推荐值**：ADR 0033 `capacity_source` 字段说明：可选值 = {`scenario_contract`（默认，场景合同声明或输入原件；出处 #1353 第二问），`bar_volume_derived`（待标定：缺推导公式，报告须单列并标 L0 假设；前置 ADR 0016 容量门已准入）}；无容量证据时在已准入规模内「穿越即全量成交」，不伪造容量值；批内按 #1350 投影清单冻结比例分配——**按本点分类处置：A 沿用，B 确认或否决**

## 裁定

正本落点：docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md

正本落点：docs/adr/0034-quantity-tier-policy-growth-equity-shift.md

正本落点：docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md

正本落点：docs/adr/0033-history-simulation-bar-batch-model.md

### 一、A-01(a) 量档分子 δ 敏感性区间与代价（Q1 沿用）

ADR 0017 字段说明补：δ 的文献敏感性区间 [0.4, 0.7]（整数对记录）（敏感性研究区间，不新增界外拒收门），默认 0.5（出处：Almgren 等 2005 美股 0.600±0.038、东京交易所 2025 全市场普查约 10⁸ metaorder 强力支持 0.5，ADR 0040 P0 已签 δ=0.5）。敏感性代价：δ 从 0.5 改至 0.4 或 0.7 时，以固定波幅比 r=√0.32 的示例计算：w(δ)=r^(1/δ)=0.32^(0.5/δ)，w(0.4)=0.2406786、w(0.7)=0.4431346，相对 w(0.5)=0.32 分别变化−24.7879%、+38.4796%（仅敏感性示例，不是生产量档；计算原件 evidence/delta-calculation.json）；改 δ 即新版本，量档 tiers 与容量 capacities 两表同时作废须重签（δ 既是冲击形态参数也是容量域坐标）。签署门字段，运行时默认 AwaitingPolicy{delta}（用户未签不跑、不回退备用值）。

### 二、A-01(b) 冲击形式：幂律与对数（Q2 确认）

ADR 0017 字段说明补 `impact_form`，同时进 `tiers[].source.Measured{impact_form, delta, …}` 与 `capacities[].domain{impact_form, delta, …}`。可选值集合 = {`power(δ)`（默认，出处：ADR 0017 裁定六，三支持文献 Tóth 等2011、Sato & Kanazawa 2025、Donier & Bonart 2015），`log`（待标定）}。选 `log` 时：tiers 记 `Unmeasured{reason: "impact_form=log 的约化与系数标定尚未完成"}`、capacities 记 `Unknown{reason: "log 形式未标定"}`，运行时 AwaitingPolicy 不伪造默认值。代价（ADR 0017:176 未算）：对数形式能否约掉 Y/σ_日/V_日/x 尚未计算；原 ADR 推测不能约掉，需标定冲击系数 Y，而 Y 需带买卖方向的逐笔成交去重建 metaorder（ADR 0017:151，#917 查实本仓三条数据通道全是聚合 Bar，做不到）；且 Y 在聚合 Bar 通道标不了。前置：#917 查实逐笔数据源可得性（当前不可得）。来源：ADR 0017:157 在案反面证据 Zarinelli 2015 arXiv:1412.2152「logarithmic functional form describes our data better than the power-law (square-root)」，登记不推翻。

### 三、A-09(a) 探针容量自动准入（Q3 沿用）

ADR 0034 字段说明补 `capacities[].source` 可选值：{`UserSigned{probe_run_ref | direct}`（默认，出处：ADR 0034 裁定四容量须用户签署、ADR 0040:135 P0 已签 L0–L2 容量），`ProbeAuto{probe_run_ref, domain_ref}`（待标定，前置未满足）}。`ProbeAuto` 前置条件两条：①该 profile 的 {粒度, δ, x, Y, σ 口径} 已由用户签署（ADR 0040:135 P0 已满足）；②探针输出满足 ADR 0022 裁定七窗口纪律并带不确定性带（#1444 待补，当前不满足 ⟹ 选中时容量记 Unknown ⟹ AwaitingPolicy）。适用域只限 ADR 0017 裁定七干净区间 L1–L2（`capacities[].level ∈ {L1, L2}`），其余级别仍 Unknown（不回落旧数）。代价：容量随探针读数漂移，每次新读数即新版本（ADR 0034 裁定三「新探针读数是版本触发」），在途重继续使用开重时冻结的已签版本，新版本不追改在途记录。ADR 0040:309 被否方案该项改写为「可选值，前置未满足、不自动生效」，排除类别不再是「被否」。

### 四、A-09(b) 溢出触发量口径（Q4 沿用）

`overflow_trigger ∈ {units, capital, mark_value}`，默认 `units`。开重时冻结选择的口径及版本，改口径须新版本，不因某次未触发而切换。`units` 以 `Q + units_g` 的数量观测量与已签数量阈值比较；`capital` 以 `q_ref + cash_g` 的资金观测量与同币种阈值比较；冻结的是基值/口径，不是增长量。`mark_value` 保留为可选值，以当前数量和该 bar 获知价格计算市值观测量；价格来源、时间及阈值换算必须在具名版本中明确，比较两侧须同量纲，不能把市值直接与数量 C(ℓ) 比较。现有源未给这套换算，#1446 实施前必须明确并验收；不在本稿虚拟填值。其代价是对价格波动敏感，可能频繁触发，不改变 Q 或容量本身的定义。CONTEXT.md 的禁令不能用来删除该可选值。

### 五、A-10(a) 成本配对单值 ROUNDTRIP（Q5 A 已裁，沿用）

ADR 0035 `amplitude.pairing` 保持单值 ROUNDTRIP，不恢复 AMP_EXEC。依据 [#907 resolution 评论5194145006](https://github.com/xy7365527-lang/NewChanlun/issues/907#issuecomment-5194145006) 的三步：成本每趟往返支付；与其可比的波幅是同一趟往返能吃到的幅度；相邻同向点不对应完整交易。标签 `[新缠论:推论]`，排除类别为「违已裁推论」，不以拟合好坏作选择理由。该评论的否证口逐字为：“若能论证「成本也可以按单边逐点计而不按往返计」，则 AMP_EXEC 回到候选。”逐字原件见 evidence/907-resolution-5194145006.txt。若将来满足否证口，仍按现行裁定变更流程处理；后一句是流程要求，不冒称 #907 原话。

### 六、A-10(b) 第二层判据形式（Q6 确认）

ADR 0035 字段说明补 `second_layer.rule`。可选值 = {`order_relation`（默认，出处：ADR 0035 裁定四:21，序关系判据，无需签数），`share_threshold{theta2}`（待标定，前置 S4-a/S4-b 修完）}。两值语义差异：(1) `order_relation` = 「主因已转为到底」⟺ `share_lb_95(reached_bottom) > share_ub_95(divergence_not_judged)` ∧ 证据门（ADR 0035:21），判据形式完全由读数区间序关系决定、不需用户签数值阈值；(2) `share_threshold{theta2}` = 「逐层、逐窗的已到底占比单边置信下界 ≥ θ₂」∧ 下述具名证据门，θ₂ 为签署门字段（默认 AwaitingPolicy{theta2}，推荐值进签署单、用户批复后生效），须在读数前预登记版本；引用已授权的#1677/evidence-gate/v1不要求重签，改六字段才另签v2。两值共同前置：S4-a/S4-b 修完（到底分类器与延伸实例证据收集）。代价：`share_threshold` 多签一个数（θ₂ 整数对）、多一次签署版本预登记流程。ADR 0035:27 被否方案「第二层用占比阈值」改为「可选值 share_threshold，前置未满足、θ₂ 待签署」，排除类别不再是「被否」；被否理由「不填占比」不是证伪、属「不推荐、代价高」，符合 ADR 0041 裁定四保留合法候选。

Q6 的既裁部分：引用 `#1677/evidence-gate/v1`（origin/main 的 ADR0022 修订§1–4、§6，原件 evidence/0022-main.txt），v1 六值为 confidence=95/100、block_unit=CalendarMonthUTC、min_events=30、min_blocks=12、min_windows=3、min_coverage=1/2。方法仍为时间块 bootstrap；下界用分布的 (1−confidence) 分位；固定规则为单边下界而非点估计、互不重叠窗口、每窗各自过线、不以合计替代。窗口表及版本早于 run_started_at 冻结，覆盖按预登记数据集首末 bar 时间区间计。v1已获授权，不能误写其缺用户重签；改六值须另签v2。

Q6 的新增推荐（供本次 P 确认，不是已批准参数）：`share_threshold` 的比较量为每个预登记分层、每窗 `reached_bottom` 占比的单边置信下界，非点估计。分层固定为 `(profile_id, symbol, granularity, base_ref, evaluated_level)`；不跨级/跨标的/跨profile合并。每层每窗的分母为该层该窗全部合格且已分类的下钻判定事件（由 classification_set 具名版本确定，每事件唯一计数），分子为其中 reached_bottom 事件。缺失/无法归类事件单列，存在此类事件时该层窗为 InsufficientEvidence，不静默剔除后过门。

推荐具体算法：按所引 block_unit 将该窗事件分块，保留块内全部事件及依赖；从该窗 m 个非空块有放回抽 m 块，每次取合并事件的分子总数/分母总数，不平均各块比例。建议 method_id=`descent-share/block-bootstrap/1`、resamples=10000、seed=0；PRNG建议固定PCG-XSH-RR 64/32（stream=1），块索引用拒绝取模避免偏差。以上算法标识/次数/随机种子和实现版本均在读数前记录；它们只属于Q6方法推荐，不加入或改写evidence_gate六字段。每次重采样按抽样块计重复，得B个精确有理占比；升序排列，取第 ceil((1−confidence)×B) 项（1-based）为下界，不作插值。固定种子仅保证可复现，不替代区间有效性；在#1441实现时以独立定义oracle和同seed双跑检验方法。

每层每窗分别须 events≥min_events、非空块数≥min_blocks；每层所报窗口数≥min_windows，窗口互不重叠、合计时长覆盖≥min_coverage数据集区间。有效窗各自比较 `share_lb(reached_bottom) ≥ θ₂`，所有窗全过才给该层通过；所有被宣称通过的层逐层满足，不能以合计或其他层补偿。门不足为InsufficientEvidence，不是Fail；完整门成立但任一窗下界<θ₂为Fail。θ₂保持AwaitingPolicy，未签不运行share_threshold。默认 `order_relation` 及其已裁比较式不改；两规则共同前置S4-a/S4-b。用户只确认这份完整方法推荐及政策形式，不在这里代签θ₂。

### 七、B-05(a) 随机微事件模型与种子（Q7 确认）

ADR 0033 字段说明补 `model_id` 与 `seed`。`model_id` 可选值 = {`history-sim/bar-batch-known-frontier/1`（默认，确定性模型，seed: none；出处：ADR 0033 裁定一，批内各重订单各自独立对 OHLC 判定、互不排序、`intra_batch_order: Unknown`），`history-sim/stochastic-microevent/<rev>`（待标定，前置未满足）}。`stochastic-microevent` 标「待标定」理由：bar 内微事件概率分布与延迟需逐笔成交数据或盘口深度（#917 查实本仓三条数据通道全是聚合 Bar，无逐笔/盘口，与 ADR 0018:82 否决「穿透 k 个 tick 缓冲」同理：「k 无数据可标定」），选中时容量记 Unknown ⟹ AwaitingPolicy 不伪造默认值。代价：bar 内路径是造出来的（实盘 bar 内顺序不可知，ADR 0018 裁定三），报告须单列随机模拟计数并标 L1 假设（#1418 报告分层规则），不得作为真实路径证据。`seed` 字段：v1 确定性模型取 `none`，随机模型版本取整数（非负 i64）。ADR 0038:29 双跑五元组固定 seed：同 (input bar 序列, policy 版本, model_id/revision, seed, launcher scheduler) 五元组逐位复现（#1411 双跑验收规则消费本字段）。前置：#917 数据接入查实逐笔/盘口数据源可得性（当前不可得）。

### 八、B-05(b) 从 bar volume 推可用量（Q8 沿用）

ADR 0033 字段说明补 `capacity_source`。可选值 = {`scenario_contract`（默认，场景合同声明或输入原件；出处：#1353 resolution 第二问选 A「部分成交只来自具证容量合同，不从 bar volume 推可用量」），`bar_volume_derived`（待标定，前置未满足）}。`bar_volume_derived` 标「待标定」理由：缺从 bar.volume 推导可成交量的公式（需标定市场微观结构参数，当前无），选中时容量记 Unknown ⟹ AwaitingPolicy。前置：ADR 0016 裁定三容量门已准入（当前 L0 已准入）。代价：报告须单列 `capacity_source=bar_volume_derived` 的成交计数并标 L0 假设（#1418 报告分层规则），不得作为容量证据（ADR 0033:20 根补充「L0 单列要求」）。无容量证据时在已准入规模内继承「穿越即全量成交」（ADR 0033:20），显式标为 L0 假设、不伪造容量数值。批内同向可成量 q 按 ADR 0032 裁定一/二（#1350 投影清单）的冻结比例分到各重，剩余量按 P7 进 E。

## 验收清单

**共同验收**（本批 #1586 正本改写票同款）：

- [ ] 每条：原写「被否」或写死单值处，改写为「可选值集合＋各值来源与代价＋适用条件」，并给默认值与推导依据（原文锚/ADR/在案读数）（ADR 0041 裁定三、四）
- [ ] 同条中属真错的部分（违原文、违已裁推论、实测否证、#799 兜底形态）保留排除，并写明排除类别
- [ ] 引缠论原文一律按 AGENTS.md 四步核（行号、逐字、界前后与行首注、行内注目视避开），只认 docs/chanlun/text/blog/
- [ ] 附受影响代码清单（文件:行号，Rust/Lean/Python 分列）
- [ ] 改既有裁定须走该 ADR/定义所属的裁定口径（ADR 0041 代价第 5 条:51：既有裁定不追溯自动改写）；推论层文档不回改

**A-01**：

- [ ] ADR 0017 字段说明补 δ 的文献敏感性区间 [0.4, 0.7]（整数对记录）、默认 0.5 与敏感性代价（固定示例 w(0.5)=0.32，按 w(δ)=0.32^(0.5/δ) 得变化−24.7879%至+38.4796%，不作生产参数、改 δ 即 tiers+capacities 两表重签）
- [ ] ADR 0017 字段说明补 `impact_form ∈ {power(δ), log}`，默认 power(δ)；log 标待标定、代价写明约化未算，原文推测不约（:176 未算）、Y 在聚合 Bar 标不了（:151）

**A-09**：

- [ ] ADR 0034 字段说明补 `capacities[].source ∈ {UserSigned, ProbeAuto}`，默认 UserSigned；ProbeAuto 前置两条（口径已签＋探针窗口纪律 #1444）、适用域 L1–L2、代价写容量漂移每次新版本
- [ ] ADR 0034 字段说明补 `overflow_trigger ∈ {units, capital, mark_value}`，默认 units；三值语义差异（口径和版本冻结、观测值更新）、版本规则（改口径即新版本）、mark_value 的价格敏感代价、同量纲阈值前置

**A-10**：

- [ ] ADR 0035 字段说明补 `pairing` 保持单值 ROUNDTRIP、排除类别「违已裁推论」、#907 否证口原样写入
- [ ] ADR 0035 字段说明补 `second_layer.rule ∈ {order_relation, share_threshold{theta2}}`，默认 order_relation；share_threshold 的 θ₂ 为签署门字段（默认 AwaitingPolicy{theta2}）、引用#1677/evidence-gate/v1；Q6完整方法明确下界/分层/逐窗/覆盖/样本门，新增方法参数标待P确认

**B-05**：

- [ ] ADR 0033 字段说明补 `model_id ∈ {history-sim/bar-batch-known-frontier/1, history-sim/stochastic-microevent/<rev>}`，默认 v1 确定性；随机模型标待标定、代价写 bar 内路径是造出来的＋报告须单列（#1418）
- [ ] ADR 0033 字段说明补 `seed`：v1 确定性取 none，随机模型版本取整数；ADR 0038 双跑固定 seed 作五元组之一（#1411）
- [ ] ADR 0033 字段说明补 `capacity_source ∈ {scenario_contract, bar_volume_derived}`，默认 scenario_contract；bar_volume_derived 标待标定、代价写报告须单列 L0 假设（#1418）
- [ ] ADR 0033:20 根补充「L0 单列要求」不改（已在 #1353 resolution 评论 6149639079 落地）

**受影响代码清单**（当前 main 实际检索）：

- [ ] Rust 代码清单：逐文件逐行列出 A-01（δ/impact_form）、A-09（capacities.source/overflow_trigger）、A-10（pairing/second_layer.rule）、B-05（model_id/seed/capacity_source）的消费点，明确哪些是参数读取、哪些是常数待改读记录、哪些是测试 golden 值
- [ ] Lean/Python 代码清单：同上，没有实现的项目写零命中及完整检索方法（rg 正则表达式）

**拟新增 blocking 边**（裁定落地后核下游接单并回填本票）：

- [ ] #1598→#1444（探针窗口纪律，A-09(a) ProbeAuto 前置）
- [ ] #1598→#1418（报告分层规则，B-05(a)/(b) 随机模型与 L0 容量须单列）
- [ ] #1598→#1411（双跑验收规则，B-05(a) 同 seed 逐位复现）

## 受影响票与 blocking 边变化

代码清单已实际附于 affected-code.md，不是待补项。#1445 不属于 B-05 直接前置，不接该边。

**拟新增 blocking 边 3 条**：#1598 为 blocker，约束下游启用/实施；下游不阻塞本票政策讨论。只在裁定落地及核边后写 tracker。

1. #1598→#1444（探针窗口纪律与不确定性带，A-09(a) `ProbeAuto` 前置条件②）
2. #1598→#1418（报告分层规则，B-05(a) 随机微事件模型与 B-05(b) `bar_volume_derived` 须单列计数并标 L0/L1 假设）
3. #1598→#1411（双跑验收规则，B-05(a) 同 seed 五元组逐位复现消费本票 `seed` 字段）

**受影响票 2 条**（本票引用其裁定结果或记录形状）：

1. #1677（已关闭，ADR 0022 修订节 `evidence_gate` 记录化，A-10(b) 引用已授权v1六字段及固定窗纪律；改六字段才签v2，θ₂另按成本政策签署）
2. #917（数据接入，A-01(b) 对数形式与 B-05(a) 随机模型的「待标定」前置：逐笔/盘口数据源可得性，当前不可得）

**下游实施票**：以附件 affected-code.md 第1、5节为清单：#1444量档/容量来源，#1446溢出触发，#1441成本证据，#1381/#1382历史模型/适配器，#1399修订，#1411双跑，#1418报告；#1406既有资金合同，TB10旧入口终态。关票前核各票实际承接条目，不能仅凭此清单代替下游接单。

## 授权引用

本票为 P 类裁决（分级器结论：票面 A-01/A-09/A-10/B-05 均改写量档记录 ADR 0017 δ 影响形态、ADR 0034 capacities.source/overflow_trigger、成本记录 ADR 0035 pairing、历史模拟 ADR 0033 model_id/capacity_source，属 graph-engineering.md「1. 分级」P 类明列的「资金与费用参数（本金、量档、cap、分配）」范畴）。

待用户批复（影子模式）。用户批复后，草稿所列 Q2/Q6/Q7 三个 B 类点的「确认或否决」结论将写入四份正本（ADR 0017/0034/0035/0033）字段说明；各字段的默认值、可选值集合、代价、适用条件、排除依据按批复结果落地。签署门字段（δ、θ₂）默认值 = `AwaitingPolicy`（ADR 0041 裁定三 B-10），用户未签署时该字段所阻的动作不跑、不回退备用值。待标定字段（log、ProbeAuto、stochastic-microevent、bar_volume_derived）选中时记录状态为 Unmeasured/Unknown ⟹ AwaitingPolicy，不伪造默认值。

## 引用自证

| 引用 | 核实命令 | 结果摘要 |
|---|---|---|
| docs/chanlun/text/blog/035-第35课.md:30 | `rg -n "级别越小.*平均的买卖点间波幅也越小.*那些太小的级别.*不足以让交易成本" docs/chanlun/text/blog/035-第35课.md` | 命中:30【正文】逐字匹配，界:52 前无「读者」/「网友」，行首无「评论」/「留言」 |
| docs/chanlun/text/blog/040-第40课.md:20 | `rg -n "每一重都对应着一定的资金与筹码" docs/chanlun/text/blog/040-第40课.md` | 命中:20【正文】逐字匹配，界:24 前无「读者」，行首无「评论」 |
| docs/chanlun/text/blog/049-第49课.md:62 | `rg -n "其后.*中枢已经完成.*无所谓.*震荡" docs/chanlun/text/blog/049-第49课.md` | 命中:62【正文】逐字匹配「其后，中枢已经完成就无所谓中枢震荡了」，界:78 前无「读者」，行首无「评论」 |
| ADR 0017:147 | `rg -n "δ = 0.5.*文献带.*0.4.*0.7" docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md` | 命中:147「δ = 0.5（文献带 **0.4–0.7** 作敏感性区间；主流 0.5」 |
| ADR 0017:157/:176 | `rg -n "对数.*不齐次\|在案反面证据.*对数" docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md` | 命中:157「★ 在案反面证据：源头论文认为**对数**拟合优于幂律」，:163「对数不齐次，大概率不约」，:176「若改用对数形式，w(d) 还约不约得掉 Y/σ_日/V_日/x？（推测约不掉——齐次性是约掉的机制，对数没有；未算）」 |
| ADR 0034:29 | `rg -n "触发量.*Q.*units_g.*开重时冻结口径" docs/adr/0034-quantity-tier-policy-growth-equity-shift.md` | 命中:29「触发量 = Q + units_g（或资金口径 q_ref + cash_g，开重时冻结口径）对已签 C(ℓ)；市值不触发」 |
| ADR 0034:39 | `rg -n "探针容量自动准入" docs/adr/0034-quantity-tier-policy-growth-equity-shift.md` | 命中:39 被否方案「探针容量自动准入（x 未裁）」 |
| ADR 0035:12 | `rg -n "pairing=ROUNDTRIP" docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md` | 命中:12「amplitude{pairing=ROUNDTRIP, per_level[…], probe_run_ref}」 |
| ADR 0035:36 | `rg -n "ROUNDTRIP.*新缠论:推论" docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md` | 命中:36「ROUNDTRIP `[新缠论:推论]`（#907）」 |
| ADR 0035:27 | `rg -n "AMP_EXEC.*配对" docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md` | 命中:27 被否方案「AMP_EXEC 配对」 |
| ADR 0033:12 | `rg -n "model_id.*history-sim.*seed.*none" docs/adr/0033-history-simulation-bar-batch-model.md` | 命中:12「模型 id `history-sim/bar-batch-known-frontier/1`…v1 确定性，`seed: none`」 |
| ADR 0033:23 | `rg -n "不从.*bar volume.*推.*可用量" docs/adr/0033-history-simulation-bar-batch-model.md` | 命中:23「不从 bar volume 推可用量；不设模型容量函数」 |
| ADR 0040:135 | Read docs/adr/0040-funding-signature-and-chong-derivation.md:133-152 | 命中:135「容量域：{1m, δ=0.5, x=10%, Y=0.9, σ=GK 中位日, 单腿}，来源 UserSigned(引 #917 评论 5199839236 / #915 报告 :272)」 |
| ADR 0040:309 | Read docs/adr/0040-funding-signature-and-chong-derivation.md:305-310 | 命中:309 被否方案「探针容量自动准入（ADR 0034:39）」 |
| ADR 0041:17-24 | Read docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md:15-26 | 命中:17-24 裁定三「落法」四款：版本化记录字段、默认值（签署门字段默认 AwaitingPolicy）、出处、改签即新版本 |
| ADR 0041:27-30 | Read docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md:27-31 | 命中:27-30 裁定四「另一种合法取值作为可选值保留，写明代价与适用条件，不删除不写成『被否』；真正错误照旧排除」 |
| CONTEXT.md:37 | Read CONTEXT.md:35-38 | 命中:37「_Avoid_: 把 depth_weights 当量档；用底档 0.32 代填高档；用账户权益或市值当 Q 或容量」 |
| #907 评论5194145006 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/5194145006 --jq .body` | 命中：往返成本→同趟可得波幅→相邻同向点不构成交易；否证口逐字见上及原件，不含“须重开” |
| #1353 评论 6149639079 | `gh api repos/xy7365527-lang/NewChanlun/issues/1353/comments --jq '.[] \| select(.id==6149639079) \| {author: .user.login, body: .body}'` | 命中：作者 xy7365527-lang，第二问选 A「部分成交只来自具证容量合同，不从 bar volume 推可用量」，ADR 0033:20 根补充「L0 单列要求」 |
| #1677 | `gh issue view 1677 --repo xy7365527-lang/NewChanlun --json state,title` | 命中：state=closed, title="[grilling] A-02(#1586)：验收统计口径记录化（evidence_gate 读数前签版本）"，ADR 0022 修订节 evidence_gate 记录化 |

## 考古对照

| 点 | 归类 | 出处 | 草稿处置 |
|---|---|---|---|
| Q1 | A 已裁 | ADR 0017:147、ADR 0040:135、ADR 0041 裁定三 | 沿用考古推荐值：δ 的文献敏感性区间 [0.4, 0.7]（整数对记录）、默认 0.5、代价写敏感性数字与两表重签，写入裁定一 |
| Q2 | B 待决 | ADR 0017:157/:176、ADR 0034 裁定一 | 确认考古推荐值：impact_form ∈ {power(δ), log}、默认 power(δ)、log 标待标定并写代价，写入裁定二 |
| Q3 | A 已裁 | ADR 0034 裁定三/四、ADR 0040:135/:309、ADR 0041 裁定三 | 沿用考古推荐值：ProbeAuto 保留为可选值、前置两条、适用域 L1–L2、代价写容量漂移，写入裁定三 |
| Q4 | A 已裁 | ADR 0034:29、CONTEXT.md:37 | 沿用考古推荐值：overflow_trigger ∈ {units, capital, mark_value}、默认 units、三值语义差异、mark_value 代价与同量纲阈值前置，写入裁定四 |
| Q5 | A 已裁 | ADR 0035:36、#907、ADR 0041 第一条 | 沿用考古结论：pairing 保持单值 ROUNDTRIP、补排除依据与否证口，写入裁定五 |
| Q6 | B 待决 | ADR 0035:21/:27、ADR 0041 裁定三 B-10、#1677 | 确认考古推荐值：second_layer.rule ∈ {order_relation, share_threshold{theta2}}、默认 order_relation、θ₂ 为签署门字段，写入裁定六 |
| Q7 | B 待决 | ADR 0033:12/:53、#1353、ADR 0038:29 | 确认考古推荐值：model_id 与 seed 字段、随机模型标待标定、seed 挂双跑五元组，写入裁定七 |
| Q8 | A 已裁 | #1353 resolution、ADR 0033:44、ADR 0016 裁定三 | 沿用考古推荐值：capacity_source ∈ {scenario_contract, bar_volume_derived}、默认 scenario_contract、bar_volume_derived 标待标定，写入裁定八 |
