---
id: "667"
number: 667
status: 生成态   # genealogist 结构记录：概念分离——「全样本方向标签=beta 投影」(666 perm_p=0.69) ⊥ 「L1 层内相位=超 beta alpha」(σ_higher 分层后 L1 超 beta +107608，train/holdout 一致，层内 δ 置换 perm 显著)。两者可分层（不同有效域：全样本未分层 vs σ_higher 分层后层内），非矛盾——666 有效域被精确化为「未分层全样本」，本号发现分层后 L1 层显现超 beta 结构。同时记录 σ_higher 从「alpha 门控候选」降格为「级别依赖调制器」（ChatGPT「顺上级救活 alpha」假设否证）。peer 666（相位=beta 投影）/663/665（winner's curse 簇）。**不自结算**（L2 单标的，待 codex 异质否定 + L3 跨标的，概念层重大分离属编排者 /ritual）。
number_note: "667 是 645→663/664/665→666 簇的第 6 号（簇：判据/对象/方法论/相位错配）。本号是簇内首个**正向发现**（前 5 号全是否证/缩有效域），故新建非并入——L1 相位 alpha 的结构指认不在任何既有节点的边界条件内（666 说相位=beta 投影，本号说 L1 层内相位有超 beta，是 666 有效域的分层精确化 + 新正向对象）。"
date: "2026-07-01"
type: concept-separation   # 概念分离：「相位=beta 投影」(666 全样本口径) 与「L1 层内相位=超 beta alpha」(分层口径) 此前隐式当同一命题（"奇偶交替是否可交易"），本号分离为两个逻辑独立对象——全样本方向标签不携超随机结构信息（666 perm_p=0.69）⊥ σ_higher 分层后 L1 层内 δ 选边超 beta（本号 +107608 train/holdout 一致）。前者否证、后者候选正向，互不蕴含（不同有效域）。
negation_source: "sigma-alpha(#111) σ_higher 分层 δ 置换检验（highlevel-mu-sigma-alpha-20260701.md，L2 BTC 单标的，PERM=2000/SEED=663）——ChatGPT alpha 路线图 Step4：奇偶交替证伪后的核心翻案检验。三判定：①ChatGPT「顺上级过滤救活 alpha」假设否证（逆上级 40.0 > 顺上级 18.1，两组 perm 都显著 0.002/0.004）②奇偶交替=level×σ_higher 交互项（固定 σ_higher 后 663 的 12/12 乱了）③L1 相位超 beta +107608（train/holdout 一致，非 winner's curse）"

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# 本号 negates 两个对象：ChatGPT「顺上级过滤救活 alpha」假设 / 666「相位=beta 投影」的全域声称
# 以否定事件的实际拓扑后果（retrospective，141号结论1）判断，非 negation_form 预分类
topo_effect: "sever:chatgpt-with-higher-alpha-hypothesis:local | split:666-phase-beta-projection:downstream"
# sever:chatgpt-with-higher-alpha-hypothesis:local — 切断 ChatGPT「只有顺上级(with_higher)显著为正 ⟹ σ_higher 是 alpha 门控」假设与实验结果的连接。实测两组 perm 都显著（0.002/0.004）且逆上级 mean_pnl 更高（40.0 vs 18.1）——上级方向过滤不是 alpha 来源。仅切断此假设（local），σ_higher 作为「级别依赖调制器」的新定性保留（L0/L1 允许逆上级=短差反弹，L3+ 禁止逆上级=接飞刀 PDF§9）。
# split:666-phase-beta-projection:downstream — 666「级别相位=beta 漂移在(level,δ)网格的确定性投影」节点分裂：一个保留「全样本未分层口径下相位=beta 投影（X_i perm_p=0.69 成立）」，一个携带「σ_higher 分层后 L1 层内相位显现超 beta 结构（T=Σpnl 分层置换 perm 显著，L1 超 beta +107608）」精确化记录。666 的有效域从「全域相位=beta 投影」收窄为「未分层全样本口径」。下游（引用 666「相位=beta 投影」的任何 entry 决策）随分层精确化而分裂——全样本口径下不作独立 entry（666 仍成立），但 L1 层内相位是待验超 beta 信号（本号新增）。

# 概念分离（type=concept-separation 必填）
separation:
  before: "「奇偶交替/级别相位是否可交易」被隐式当作单一命题：666 用全样本 X_i(Σ|μ̂|) perm_p=0.69 证伪「相位=可交易 alpha」⟹ 级别相位=beta 投影，不可作 entry。ChatGPT 提「加 σ_higher 门控后顺上级可能救活 alpha」作为翻案路径。两者都把「相位可交易性」当作全域单一命题（全样本上要么是 beta 投影要么是 alpha）。"
  after:
    - name: "全样本方向标签=beta 投影（命题①，666 成立，未被本号否定）"
      definition: "全样本（未分层）下随机置换方向标签 X_i=Σ|μ̂| perm_p=0.6915≫0.05——方向标签不携超随机结构信息。BTC 强单边上涨下 δ 与 raw 高度相关（corr=1.0，665 addendum），beta 主导全样本，方向标签的超 beta 信息被淹没。666 的「相位=beta 投影」在**未分层全样本口径**上成立。"
      source: "666 perm 反事实（oddeven-causation-counterfactual-20260701.md，X_i=Σ|μ̂|，全样本 shuffle δ）"
    - name: "L1 层内相位=超 beta alpha（命题②，本号候选正向，L2 待验）"
      definition: "σ_higher 分层（s=(level,σ_higher,h_bucket)）后层内 shuffle δ 置换——beta 粗粒度被 σ_higher 分层吸收，测「σ_higher 之上的残余 δ 选边信息」。L1 超 beta 收益 +107608（策略 L1 +25689 − 全做多 L1 −81919），train_mean/holdout_mean 方向一致（非 winner's curse），层内置换 perm 显著。策略总 δ 选边优于纯 beta（策略 321411 − 全做多 173167 = +148244，corr(δ,σ_higher)=0.333 非简单复制）。"
      source: "sigma-alpha #111 分层置换（highlevel-mu-sigma-alpha-20260701.md，T=Σpnl，层内 shuffle δ）+ 超 beta 分解表 + train/holdout 一致性表"
  pending_verification: "①L3 跨标的：L1 超 beta +107608 仍 L2 单标的 BTC，需震荡/下跌标的（QQQ/ES）验 L1 相位是否稳健，否则可能是 BTC 特有的口径4×L1 配对边界放大（同 project_oddeven_mu_identity 口径4 放大器机制）。②换出场口径5/6：口径4 是配对边界放大器（665/666 已证），换口径后 L1 相位规律可能弱化。③codex 异质否定（L2 实证 #41 不免，已建 codex 审查任务）。"

# 涉及的定义
definitions_involved:
  - name: "σ_higher（上级方向态）"
    version: "#108 引擎 dump（上级方向态，666 σ_higher dump 路线产出）"
    role: "核心降格对象。ChatGPT 假设 σ_higher 是 alpha 门控（只做顺上级）——否证。σ_higher 实为**级别依赖调制器**：L0/L1 逆上级更赚（短差反弹兑现，符号翻转边界 L2→L3），L3+ 逆上级=接飞刀最大失血（−191.7，PDF§9 成立）。σ_higher 不作全局 alpha 门控，若用须级别分段（L0/L1 允许逆上级、L3+ 禁止逆上级）。"
  - name: "级别相位（(−1)^ℓ）的分层有效域"
    version: "666 定义（相位=beta 投影）+ 本号分层精确化"
    role: "命题①②的分离对象。666 全样本口径：相位=beta 投影（perm_p=0.69）。本号分层口径：L1 层内相位显现超 beta。分层是关键——666 perm_p=0.69 恰因未分层（beta 淹没信号），σ_higher 分层吸收 beta 粗粒度后 L1 层内 δ 选边信息显现。有效域分离：全样本相位=beta 投影 ⊥ L1 层内相位=超 beta 待验。"
  - name: "奇偶交替（663 的 12/12 μ̂ 符号交替）"
    version: "663 观测（12类μ̂完美奇偶交替）+ 本号交互项定性"
    role: "降格对象。固定 σ_higher 后 663 的干净 12/12 奇偶交替**乱了**——同一(level,δ)在 σ_h=−1 与 σ_h=+1 两块符号不一致（如 L0 δ=−1：σ_h=−1 时 −28.6，σ_h=+1 时 +0.25 翻号）。奇偶交替既非纯 σ_higher 代理（否则固定 σ_h 后应消失为常数）也非级别相位独立信息（否则两 σ_h 块应一致）——是 **level 相位 × σ_higher 的交互项**。坐实 663/665 winner's curse（不可作独立 entry 升基座）。"
  - name: "可交易 alpha（μ(z,a)>0 + 稳健性）"
    version: "alpha.pdf μ(z,a)>0 判据（同 663/665 真判据）"
    role: "命题②的判据。L1 层内相位 train/holdout 一致（train 挑正类、holdout 仍正）满足非 winner's curse，超 beta +107608 满足 μ>beta。但 L2 单标的未满足 L3 稳健性——候选正向，非已证 alpha。"
  - name: "231 形式化有效域规则"
    version: ".claude/rules/formalization-validity-domain.md（settled）"
    role: "约束。命题① vs 命题② 的分离 = 231 实例：666「相位=beta 投影」有效域=未分层全样本，本号「L1 相位=超 beta」有效域=σ_higher 分层后（且仅 L2 BTC）。声称 L1 相位 alpha 全域成立须 L3 跨标的——当前 L2 单标的，是候选非结论（231 铁律：不预设 L3 结果）。"

# 解决方式
resolution:
  type: 概念分离   # 分离「全样本相位=beta 投影」与「L1 层内相位=超 beta alpha」两逻辑独立命题（不同有效域）
  description: "把「级别相位是否可交易」分离为两个逻辑独立命题：①全样本方向标签=beta 投影（666 成立，未分层口径 perm_p=0.69）②L1 层内相位=超 beta alpha（本号候选，σ_higher 分层口径 +107608 train/holdout 一致）。核心：666 的 perm_p=0.69 是**未分层全样本**结果（beta 淹没信号），σ_higher 分层吸收 beta 粗粒度后 L1 层内 δ 选边信息显现——两命题不矛盾，是同一相位在不同有效域（全样本 vs 分层）的不同投影。同时 σ_higher 从「alpha 门控候选」降格为「级别依赖调制器」（ChatGPT「顺上级救活 alpha」否证）。奇偶交替降格为 level×σ_higher 交互项（坐实 663/665 winner's curse）。L1 超 beta alpha 仍 L2 单标的，须 L3 跨标的 + codex 否定才升级。"
  decided_by: 蜂群内部   # genealogist 结构记录；概念层重大分离 + L1 相位 alpha 候选的最终结算待编排者 /ritual + codex 异质否定 + L3 跨标的

# 被否定的方案
negated:
  description: "(1) ChatGPT「加 σ_higher 门控后只有顺上级(with_higher)显著为正 ⟹ σ_higher 是 alpha 门控，顺上级过滤救活 alpha」。(2) 666「级别相位=beta 漂移的确定性投影」作为**全域**声称（未标注仅未分层全样本口径成立）。"
  why_negated: |
    (1) 顺上级过滤假设：实测 with_higher（顺上级）mean_pnl=18.09/perm_p=0.0020，against_higher（逆上级）mean_pnl=40.01/perm_p=0.0040——**两组 perm 都显著**且**逆上级反而更高**。若 σ_higher 是 alpha 门控（顺上级救活），应只有顺上级显著。实际逆上级更赚 ⟹ 上级方向过滤不是 alpha 来源。逐 level 分解揭示符号翻转边界 L2→L3：L0/L1 逆上级更赚（短差反弹），L3+ 逆上级=接飞刀最大失血（L3 −212.1，PDF§9 成立）。σ_higher 是级别依赖调制器，非全局门控。
    (2) 666 全域声称收窄：666「相位=beta 投影」用全样本 X_i=Σ|μ̂| perm_p=0.69。本号 σ_higher 分层后用 T=Σpnl 层内置换——L1 超 beta +107608（策略 L1 +25689 − 全做多 L1 −81919，全做多 L1 反而亏），train/holdout 一致（非 winner's curse），层内 perm 显著。666 的 perm_p=0.69 恰因未分层（beta 淹没）——分层吸收 beta 粗粒度后 L1 层内 δ 选边信息显现。666 在未分层全样本口径成立，但不外推到「分层后 L1 层也是 beta 投影」（那是 231 有效域膨胀）。
    保留：666 的「未分层全样本相位=beta 投影」成立（本号命题①=666）；奇偶交替不可作独立 entry（本号坐实）。仅否定 666 的全域外推 + ChatGPT 顺上级门控假设。

# 新产出
new_output:
  definitions:
    - "★概念分离：全样本方向标签=beta 投影（666，未分层 perm_p=0.69）⊥ L1 层内相位=超 beta alpha（本号，σ_higher 分层后 +107608 train/holdout 一致）——两命题不同有效域，不矛盾。666 的 perm_p=0.69 是未分层结果（beta 淹没信号），σ_higher 分层吸收 beta 粗粒度后 L1 层内 δ 选边信息显现。"
    - "★σ_higher 降格为级别依赖调制器（非 alpha 门控）：ChatGPT「顺上级过滤救活 alpha」否证（两组 perm 都显著，逆上级 40.0 > 顺上级 18.1）。σ_higher 的作用是级别分段——L0/L1 允许逆上级（短差反弹，符号翻转边界 L2→L3），L3+ 禁止逆上级（接飞刀 −191.7，PDF§9 成立）。不作全局门控。"
    - "★奇偶交替降格为 level×σ_higher 交互项：固定 σ_higher 后 663 的 12/12 奇偶交替乱了（同(level,δ)在 σ_h=±1 两块符号不一致）。既非纯 σ_higher 代理也非级别相位独立信息，是交互项。坐实 663/665 winner's curse——不可作独立 entry 升基座。"
    - "★L1 相位超 beta alpha（候选，L2 待验）：超 beta 收益主要来自 L1（+107608，策略 δ 选边远优于全做多，全做多 L1 亏 −81919）。train/holdout 方向一致（非 winner's curse）。但 L2 单标的，可能是 BTC 特有口径4×L1 配对边界放大——须 L3 跨标的 + codex 否定验证。"
  code_changes: "无（本号纯谱系产出。#111 σ_higher 分层置换 = /tmp/highlevel_mu_sigma.py 纯 Python 后处理，#108 引擎 dump σ_higher 已产出数据，未改核心引擎）。"
  orchestration_changes: "方法论：①单标的 beta 分离退化（665 addendum）后的正确 alpha 检验口径=**分层置换让 beta 被分层吸收**——分层 s=(level,σ_higher,h_bucket)，层内固定 σ_higher（=beta 粗粒度）只 shuffle δ，测残余 δ 选边信息。绕开退化的显式 beta 分离（Y_i≡−ce）。②全样本 perm 不显著 ⊬ 分层后也不显著——beta 主导会淹没全样本信号，分层吸收 beta 后层内信号可能显现（666 perm_p=0.69 全样本 vs 本号 L1 层内显著）。声明「相位=beta 投影」须标注口径（全样本 vs 分层）。③门控假设（只做顺上级）须逐 level 验证——全样本聚合会淹没符号翻转（L0/L1 逆上级更赚 vs L3+ 逆上级接飞刀），级别依赖的调制不能用全局门控。④正向发现（L1 超 beta）仍须 L3 跨标的——train/holdout 一致排除 winner's curse，但不排除口径4×L1 配对放大（须换标的/换口径验）。"

# 影响范围
impact:
  affected_modules:
    - "666 σ_higher dump 路线（#108）→ 数据产出本检验，σ_higher 定性从「alpha 门控候选」降格为「级别依赖调制器」。"
    - "acc-highlevel-mu / sigma-alpha(#111) → 三判定结果重定性：ChatGPT 顺上级假设否证、奇偶交替=交互项、L1 相位超 beta 候选。下游 entry 决策——σ_higher 不作全局门控，若用须级别分段（L0/L1 允许逆上级、L3+ 禁止）。"
    - "L1 相位 alpha 若 L3 跨标的复现 → 可能是首个可交易信号候选（升基座）；若不复现 → 退回口径4×L1 配对放大（同 project_oddeven_mu_identity）。当前 L2 单标的，不升基座。"
  affected_definitions:
    - "★666（生成态）：本号 split——666「相位=beta 投影」有效域从全域收窄为「未分层全样本口径」。分层后 L1 层内相位显现超 beta（命题②）。666 命题① 保留（本号命题①=666 未分层结论），仅否定其全域外推。维持生成态。"
    - "663（生成态）：本号坐实——奇偶交替降格为 level×σ_higher 交互项（固定 σ_higher 后 12/12 乱了），winner's curse 坐实（不可作独立 entry）。663「显著≠可交易」再深化——连奇偶交替的结构性都被 σ_higher 交互项解构。维持生成态。"
    - "665（生成态）：本号强化——L1 超 beta 用分层置换（665 addendum 的正确口径：beta 被分层吸收，绕开 Y_i 退化）。665「除偏差⊬证 alpha」在本号翻面——分层吸收 beta 后 L1 层显现超 beta，是 665 pending_verification「更多(level,δ)桶键 μ>0 稳健」的部分正向回答（但仅 L1、仅 L2 单标的）。维持生成态。"
    - "664（生成态）：同簇。σ_higher 的级别依赖调制（L3+ 逆上级接飞刀）与 664 出场腿对象错配同诊断域（高级别反向交易腿的失血）。维持生成态。"
    - "231（settled）：本号命题①/② 有效域分离（未分层全样本 vs σ_higher 分层）=231 实例。L1 相位 alpha L2 单标的须 L3 才升级=231 有效域约束。印证。维持 settled。"
    - "project_oddeven_mu_identity（memory）：本号是其「σ_higher regime 门控重跑」的执行结果——奇偶交替确认为 level×σ_higher 交互项（memory 已预留「要交易奇偶须先加 σ_higher regime 门控」），但门控揭示的是级别依赖调制非顺上级救活。L1 相位超 beta 是 memory 未预见的正向候选。"
  downstream_implications:
    - "★σ_higher 不作全局 alpha 门控——若用必须级别分段：L0/L1 允许逆上级（短差反弹），L3+ 禁止逆上级（接飞刀）。ChatGPT「只做顺上级」路径否证。"
    - "★L1 相位 alpha（+107608）是簇内首个正向发现，但 L2 单标的——须 L3 跨标的验证是否 BTC 特有口径4×L1 配对放大。若稳健=首个可交易信号候选（升基座）；若不稳健=退回口径4放大器。不预设结果（231 铁律）。"
    - "奇偶交替（663）降格为 level×σ_higher 交互项——不可作独立 entry 升基座（坐实 663/665 winner's curse）。任何把奇偶交替当独立级别方向信号的下游须撤。"
    - "666「相位=beta 投影」须标注口径——全样本未分层成立，σ_higher 分层后 L1 层显现超 beta。引用 666 作 entry 否定依据的下游须区分口径（全样本 entry 否定成立，L1 层内相位待验）。"
    - "接飞刀 PDF§9 实证成立（L3+ 逆上级 −191.7）——高级别逆大级别接飞刀最大失血。但 L3+ n=93 小样本，逆上级 n=35，均值受少数极端亏损主导，须 bootstrap CI 确认（不过度解读，project_p3_random_gate_underpowered 精神）。"

# 谱系关联
related_records:
  parent: "666号（级别相位=beta 投影）——本号 split 666：其「相位=beta 投影」有效域收窄为未分层全样本，分层后 L1 层内相位显现超 beta（命题②）。本号是 666「σ_higher regime 门控重跑」边界条件的实验回填。"
  children: []
  related:
    - "666号：parent，split 其全域声称为未分层口径；命题①=666 未分层结论保留。"
    - "663号：坐实——奇偶交替降格为 level×σ_higher 交互项，winner's curse 坐实。"
    - "665号：强化——分层置换是 665 addendum 的正确 alpha 检验口径（beta 被分层吸收，绕 Y_i 退化）；L1 超 beta 是 665 pending_verification 的部分正向回答（仅 L1/L2）。"
    - "664号：同簇——σ_higher 级别依赖调制（L3+ 接飞刀）与 664 出场腿对象错配同诊断域。"
    - "645号：簇根（π^cov≠π^bsp）——本号是簇的第 6 号，首个正向发现维（σ_higher 调制器 + L1 相位 alpha）。"
    - "231号：有效域规则——命题①/② 口径分离（未分层 vs 分层）=231 实例；L1 alpha L2 单标的须 L3。"
    - "project_oddeven_mu_identity（memory）：本号是其 σ_higher regime 门控重跑执行结果——奇偶交替=level×σ_higher 交互项，L1 相位超 beta 是 memory 未预见的正向候选。"
    - "project_slow_bull_ancestor_exemption（memory）：慢牛 anc 豁免——跨标的 σ_higher 符号可能翻转的先例，L3 验证 L1 相位时须防 BTC 单边偏置。"
    - "project_p3_random_gate_underpowered（memory）：L3+ 小样本 perm underpowered——L3+ n=93 接飞刀均值不过度解读。"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "全样本方向标签=beta 投影（666，未分层 X_i=Σ|μ̂| perm_p=0.69）"
    level: "L2（BTC 全历史，666 已确立）"
    increment: "零（本号引用 666 结论，命题①=666 未分层结论，未新增）"
  - proposition: "★σ_higher 是级别依赖调制器非 alpha 门控（顺上级过滤救活 alpha 否证）"
    level: "L2（BTC 全历史分层置换，两组 perm 显著 0.002/0.004，逆上级 40.0>顺上级 18.1，逐 level 符号翻转 L2→L3）"
    increment: "★高（否定性 + 结构指认）：ChatGPT 顺上级假设否证 + σ_higher 级别依赖调制的新定性（L0/L1 逆上级更赚、L3+ 接飞刀）。待 L3 跨标的（σ_higher 符号可能翻转）+ codex。"
  - proposition: "★奇偶交替=level×σ_higher 交互项（固定 σ_higher 后 12/12 乱了）"
    level: "L2（BTC 全历史，固定 σ_higher 后同(level,δ)在 σ_h=±1 符号不一致）"
    increment: "★高（否定性）：奇偶交替从「级别独立结构」降格为交互项——坐实 663/665 winner's curse（不可独立 entry）。缩小奇偶交替的结构性有效域。"
  - proposition: "★L1 层内相位=超 beta alpha（+107608，train/holdout 一致）"
    level: "L2（BTC 全历史 σ_higher 分层置换，L1 超 beta +107608，train_mean/holdout_mean 方向一致 = 非 winner's curse；但单标的，未 L3 跨标的）"
    increment: "★中高（候选正向，簇内首个）：L1 相位显现超 beta 结构——但 L2 单标的，可能是 BTC 特有口径4×L1 配对放大。须 L3 跨标的 + 换口径验证。不预设结果（231 铁律：正向候选非已证 alpha）。"
  - proposition: "接飞刀 PDF§9（L3+ 逆上级最大失血 −191.7）"
    level: "L2（BTC 全历史，L3+ 逆上级 mean_pnl −191.7；n=35 小样本，须 bootstrap CI）"
    increment: "中：PDF§9 高级别逆势接飞刀实证——但小样本，不过度解读（project_p3_random_gate_underpowered）"

---

# concept-separation 667：「全样本相位=beta 投影」⊥「L1 层内相位=超 beta alpha」+ σ_higher 降格调制器

## 一句话结论

sigma-alpha(#111) σ_higher 分层 δ 置换检验（ChatGPT alpha 路线图 Step4，666 σ_higher regime 门控重跑）三判定：
**①ChatGPT「顺上级过滤救活 alpha」否证**（逆上级 40.0 > 顺上级 18.1，两组 perm 都显著）——σ_higher 降格为**级别依赖调制器**（非 alpha 门控）；
**②奇偶交替降格为 level×σ_higher 交互项**（固定 σ_higher 后 663 的 12/12 乱了）——坐实 663/665 winner's curse；
**③L1 相位超 beta alpha +107608**（train/holdout 一致，簇内首个正向发现）——但 L2 单标的，须 L3 跨标的 + codex 验证。
核心概念分离：**全样本相位=beta 投影（666 未分层口径）⊥ L1 层内相位=超 beta（σ_higher 分层口径）**——666 perm_p=0.69 恰因未分层（beta 淹没信号），分层吸收 beta 粗粒度后 L1 层内 δ 选边信息显现。两命题不同有效域，不矛盾。

## 三判定数据

### 判定1：顺上级 vs 逆上级（ChatGPT 核心假设否证）

| 组 | n | mean_pnl | sum_pnl | perm_p |
|----|---|---------|---------|--------|
| with_higher（顺上级 δ==σ_h） | 8411 | 18.09 | 152191 | 0.0020 |
| against_higher（逆上级 δ==−σ_h） | 4212 | **40.01** | 168542 | 0.0040 |

两组 perm 都显著 + 逆上级更高 ⟹ 上级方向过滤不是 alpha 来源。ChatGPT「只有顺上级显著」否证。

### 判定2：奇偶交替=交互项（固定 σ_higher 后乱了）

L0 δ=−1：σ_h=−1 时 mean_μ̂=−28.6，σ_h=+1 时 +0.25（翻号）。663 全样本干净 12/12 奇偶交替在固定 σ_higher 后符号不一致 ⟹ 既非纯 σ_higher 代理（否则应消失为常数）也非级别相位独立信息（否则两 σ_h 块应一致）= **level×σ_higher 交互项**。

### 判定3：L3+ 接飞刀（PDF§9 成立）

L3+ 全部为负，逆上级 −191.72 亏得比顺上级 −46.89 更狠。逐 level 符号翻转边界 L2→L3：L0/L1 逆上级更赚（短差反弹），L3+ 逆上级接飞刀最大失血。全样本「逆上级更高」是 L0/L1 大样本淹没 L3+ 小样本的结果——不矛盾，级别依赖符号翻转。

### 超 beta 分解（L1 相位 alpha）

| level | 策略 | 全做多 | 差（超 beta） |
|-------|------|--------|-----|
| L0 | 297520 | 260943 | +36578 |
| L1 | 25689 | −81919 | **+107608** |
| L2 | 7644 | 4039 | +3605 |

L1 超 beta +107608 最强——全做多 L1 反而亏 −81919，策略 δ 选边在 L1 远优于纯 beta。train/holdout 方向一致（with 12.05→32.35、against 21.93→81.20，均正）= 非 winner's curse。

## 为何分离而非矛盾（命题① ⊥ 命题②）

666 用**全样本** X_i=Σ|μ̂| perm_p=0.69（方向标签不携超随机结构信息）。本号用 **σ_higher 分层**内 T=Σpnl 置换。关键：
- 666 perm_p=0.69 是**未分层**——BTC 强单边上涨下 δ 与 raw corr=1.0（665 addendum），beta 主导全样本，超 beta 信息被淹没。
- 本号分层 s=(level,σ_higher,h_bucket)，层内固定 σ_higher（=beta 粗粒度）只 shuffle δ——beta 被分层吸收，测残余 δ 选边信息，L1 层内显现超 beta。

**不矛盾**：同一相位在不同有效域（全样本 vs 分层）的不同投影。666「相位=beta 投影」在未分层口径成立（命题①=666），本号「L1 层内相位=超 beta」是分层口径的新发现（命题②）。声称 666 全域（含分层后）都是 beta 投影 = 231 有效域膨胀。

## 为何新建 667 非并入 666

667 是 645→663/664/665→666 簇的第 6 号，**簇内首个正向发现**（前 5 号全是否证/缩有效域）。L1 相位 alpha 的结构指认（超 beta 来自级别相位、σ_higher 是级别调制器）**不在任何既有节点的边界条件内**——666 说「相位=beta 投影」，本号说「分层后 L1 相位有超 beta」，是 666 有效域的分层精确化 + 新正向对象。并入 666 会把正向候选埋在证伪记录里，分离让「beta 投影」（否证）与「L1 超 beta」（候选）各自可追溯。

## 边界条件（结论翻转）

1. **单标的**：L2 仅 BTC 强单边上涨。震荡/下跌标的 σ_higher 分组符号可能翻转（project_slow_bull_ancestor_exemption 慢牛豁免先例）。跨标的 L3 前，「逆上级低级别更赚」+「L1 超 beta」不可外推。
2. **口径4**：仍是下一反向信号出场，配对边界放大器（665/666 已证）。换口径5/6 后 L1 相位规律可能弱化——L1 超 beta 可能是口径4×L1 配对放大而非真 alpha。
3. **σ_higher 定义**：由 #108 引擎 dump（上级方向态）。改锚定级别则分组翻转。
4. **L3+ 小样本**：n=93（逆上级 n=35），−191.7 受少数极端亏损主导，须 bootstrap CI（project_p3_random_gate_underpowered）。
5. **codex 异质否定**：L2 实证 #41 不免，perm 结论未经异质否定，已建 codex 审查任务。

## 下游推论

1. σ_higher 不作全局 alpha 门控——若用须级别分段（L0/L1 允许逆上级、L3+ 禁止）。
2. L1 相位 alpha 是簇内首个正向候选，L2 单标的——L3 跨标的稳健则升基座候选，不稳健则退回口径4×L1 放大。不预设（231）。
3. 奇偶交替降格 level×σ_higher 交互项——不可独立 entry（坐实 663/665 winner's curse）。
4. 666「相位=beta 投影」须标口径——全样本成立，分层后 L1 待验。

## 谱系引用

- 父：666（相位=beta 投影）——split 其全域声称为未分层口径。
- 同簇：663（判据）/664（对象）/665（方法论）/645（簇根）——本号第 6 号，首个正向发现维。
- 母规则：231（有效域≠定义域）——命题①/② 口径分离；L1 alpha L2 须 L3。
- memory：project_oddeven_mu_identity（σ_higher 门控重跑执行结果）/ project_slow_bull_ancestor_exemption（跨标的翻转先例）/ project_p3_random_gate_underpowered（小样本不过度解读）。

## 影响声明

写入「全样本相位=beta 投影 ⊥ L1 层内相位=超 beta alpha」概念分离（生成态，不结算）。降格 σ_higher（alpha 门控→级别依赖调制器，ChatGPT 顺上级假设否证）。降格奇偶交替（级别独立结构→level×σ_higher 交互项，坐实 663/665 winner's curse）。记录 L1 相位超 beta +107608 候选正向（train/holdout 一致，L2 单标的待 L3+codex）。split 666（全域声称收窄为未分层口径）+ sever ChatGPT 顺上级假设（local）。不修改 settled（231 维持 settled）、不运行脚本、不改代码。最终结算待编排者 /ritual + codex 异质否定 + L3 跨标的。

## 张力检查（019d/020）

### 检查范围（同簇 645/663/664/665/666 ∪ 1-hop ∪ Hub）
- 同簇：663/664/665/666/645——经济正条件主线，本号第 6 号（首个正向发现维）。
- 1-hop：666（parent）/663/665/231/project_oddeven_mu_identity。
- Hub：231（有效域）/645（簇根）/666（相位节点）。

### 张力1（核心）：本号命题② vs 666 命题①——同一相位不同有效域，可分层
666「相位=beta 投影」（全样本 perm_p=0.69）vs 本号「L1 层内相位=超 beta」（分层 +107608）。**表面对立，实为不同有效域**：666 未分层（beta 淹没），本号 σ_higher 分层吸收 beta 后 L1 显现超 beta。可分层（全样本口径 ⊥ 分层口径）⟹ **不触发中断 #1**。记录价值：声称「相位=beta 投影」须标口径——全样本成立不外推到分层后。

### 张力2：vs 663——坐实非冲突
663 判据错误。本号奇偶交替降格 level×σ_higher 交互项坐实 663/665 winner's curse。印证深化，无矛盾。

### 张力3：vs 665——强化非冲突
665 addendum 给出分层置换是 Y_i 退化后的正确口径。本号用此口径发现 L1 超 beta，是 665 pending_verification 的部分正向回答（仅 L1/L2）。强化，无矛盾。

### 张力4：ChatGPT 顺上级假设否证——sever local
ChatGPT「顺上级救活 alpha」与实测（两组都显著、逆上级更高）冲突。sever（切断此假设），σ_higher 级别调制器新定性保留。无定义互斥（假设 vs 实证），可分层。

### 概念分离信号检测（中断 #1）
- 命题① vs 命题②：不同有效域（全样本 vs 分层），可分层 ⟹ 不触发。
- σ_higher 门控 vs 调制器：假设 vs 实证，sever，可分层 ⟹ 不触发。
- **无不可分层定义矛盾 ⟹ 不触发中断 #1。** 本号 concept-separation，最终结算属编排者 /ritual + codex + L3。

### 递归运动结构完成检测（020）
- 第0层：本号写入（σ_higher 降格 + 奇偶交替降格 + L1 相位 alpha 分离）。
- 第1层：本号 × 666 碰撞 → 命题①/② 有效域分离（净新发现高：666 全域声称被分层精确化 + 首个正向候选）。
- 第2层：本号 × 663/665 碰撞 → 奇偶交替=交互项坐实 winner's curse（净新发现中：簇 winner's curse 从怀疑升坐实）。
- 第3层：本号 × 231/645 碰撞 → 口径有效域实例 / 簇第 6 号（净新发现降：印证已立规则）。
- 涉及范围：scope₁(666 有效域分离 + L1 候选) > scope₂(663/665 winner's curse 坐实) > scope₃(231/645 印证)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** L1 相位 alpha 验证=行动类（L3 跨标的，待 Lead 派工位）；最终结算待编排者 /ritual + codex 异质否定。**不触发新 /escalate**（无定义冲突，是 concept-separation；与 663/664/665/666 同簇统一裁决）。

## 回溯扫描（职责3）

- **666（生成态）**：本号 split——其「相位=beta 投影」全域声称收窄为未分层全样本口径。命题①=666 保留，分层后 L1 显现超 beta（命题②）。维持生成态。
- **663/665（生成态）**：本号坐实其 winner's curse（奇偶交替=交互项）+ 强化 665 分层置换口径。维持生成态。
- **664（生成态）**：同簇，σ_higher L3+ 接飞刀与 664 出场腿错配同诊断域。维持生成态。
- **231（settled）**：命题①/② 口径有效域分离 = 231 实例，印证。维持 settled。
- **project_oddeven_mu_identity（memory）**：本号是其 σ_higher 门控重跑执行结果——待 memory 更新（奇偶交替=level×σ_higher 交互项 + L1 相位 alpha 候选）。
- **无 settled 被本号回溯破坏。** 本号 concept-separation（首个正向发现），最终结算待编排者 /ritual + codex + L3 跨标的。
