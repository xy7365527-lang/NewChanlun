---
id: "633"
number: 633
status: 生成态   # source_index 坐标系 bug（slice_date_window 切片未重置局部下标致 fill_bar_index 越界吞决策，commit b6aeabbf8b 已修）的两层谱系意义落盘 + 「#2 Move 丢价格 ⊥ #5 source_index bug 疑似同源」假设被机器证据否定的跨层误判模式。三者共享同一事件（source_index bug）的不同层面。bug 本身=实现错误（已修，行动类）；谱系价值=工程缺口模式 + raw_source_index 跨 Dataset 溯源边界 + L0 模式推测被 L2 机器证据否定（formalization-validity-domain 实例）。最终辨认待编排者 /ritual。依赖 231。
date: "2026-06-27"
type: 矛盾发现
depends_on: ["231"]
related: ["632", "625", "627", "623", "090", "no-workaround", "no-patch-mentality", "formalization-validity-domain", "L2-engine-incompleteness-vs-theta-falsification"]
title: "source_index 坐标系 bug（slice_date_window 切片未重置局部下标 → fill_bar_index 越界吞决策，commit b6aeabbf8b 已修）的两层谱系意义 + 跨层误判模式：①工程缺口模式『数据切片不重置局部坐标系致下标越界』在 NewChanlun 谱系无先例（核 8 处 source_index/切片/坐标系 Grep 命中：604 切片=Lean List.drop/take、turning-node 切片=E×F 代数轨道、627 坐标系=分层诊断认识论坐标系——均非数据切片局部下标，无重复）；②raw_source_index 跨 Dataset 溯源边界（codex 提示：现修复重置 source_index 为局部一致，若未来有跨 Dataset 溯源消费者则重置破坏全集溯源，须新增 raw_source_index 字段——前瞻边界值，当前无消费者）；③跨层误判模式：Lead 曾提『#2 Move 丢价格(定义冲突,632) ⊥ #5 source_index bug(实现 bug)疑似同源』，被机器证据否定（层级不同：#2=设计契约层 canonical Move 信息不足，#5=实现疏漏切片下标越界）——这是 L0 模式推测（同源假设）被 L2 机器证据否定的 formalization-validity-domain 实例（L0→L2 信息增量为正）"
negation_source: cc
negation_form: none   # bug 本身=实现错误（已修，非对定义的否定）；本号是其谱系意义落盘 + 同源假设被否定的实例记录。同源假设否定=L2 机器证据缩小 L0 推测有效域，非对既有定义的 separation/waiting。
# none：source_index bug 是实现错误（commit b6aeabbf8b 已修，testing-override 判据：不改任何定义即可修复=实现错误）。
#   本号不否定任何既有定义——是三件谱系意义的落盘：①工程缺口模式无先例 ②raw_source_index 前瞻边界 ③同源假设被机器证据否定。
#   ③『同源假设被否定』本身携带否定性结果（L2 否证 L0 推测），但被否的是 Lead 的临时推测（非已结算定义），故 negation_form=none。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：隐含命题『#2(Move 丢价格) 与 #5(source_index bug) 同源（可由单一根因统一解释/统一修复）』
# 实际拓扑后果：同源假设被机器证据否定——两者层级正交（#2=设计契约层 ⊥ #5=实现疏漏层），
#   不可由单一根因统一解释，须分别处理（#2 走 632 escalate 待 P1 裁定 canonical 契约 / #5 已 commit 修复）。
topo_effect: "falsify:hypothesis-2-and-5-same-origin:cross-layer-orthogonal[design-contract-canonical-move-drops-price ⊥ implementation-slice-local-index-overflow] | record:source-index-slice-local-coordinate-reset-engineering-gap-no-precedent | open:raw-source-index-cross-dataset-tracing-boundary-no-current-consumer"
# falsify：『#2/#5 同源』假设被机器证据否定（层级正交，不可统一）
# record：工程缺口模式『切片不重置局部坐标系』在谱系无先例（核 8 处 Grep 命中无重复）
# open：raw_source_index 跨 Dataset 溯源边界（前瞻，当前无消费者，重置 source_index 暂不破坏任何现存溯源）

# 矛盾（type=矛盾发现 必填）
contradiction:
  description: "source_index 坐标系 bug（commit b6aeabbf8b，SG-coordsys 修复）暴露三层谱系意义，其中第③层是一个被否定的同源假设：

  ①【工程缺口模式：数据切片不重置局部坐标系致下标越界】slice_date_window 按日期窗口切片后，切片是新的局部序列（局部下标从 0 重起），但 fill_bar_index 仍按全集（切片前）下标消费——局部坐标系与全局坐标系混用 → fill_bar_index 越界 → 吞掉本应产生的决策（信号丢失）。根因=切片操作生成新坐标系但下标消费方未重置到局部坐标系。核谱系无此工程缺口先例（见 definitions_involved 核 8 处 Grep 命中）。

  ②【raw_source_index 跨 Dataset 溯源边界】修复方案=重置 source_index 为切片局部一致（局部下标 0 重起，与切片对齐）。codex 提示：此重置在『单 Dataset 内消费』语义下正确，但若未来有跨 Dataset 溯源消费者（需把切片内的局部下标映射回原始全集下标做跨数据集追踪），重置会破坏全集溯源能力——届时须新增 raw_source_index 字段保留原始全集下标，与局部 source_index 并存。这是前瞻边界值：当前无跨 Dataset 溯源消费者，故重置不破坏任何现存溯源（修复正确）；但边界一旦被未来消费者触及，须补 raw_source_index（不补=声明膨胀，声称 source_index 可跨 Dataset 溯源而实际只局部一致）。

  ③【跨层误判模式：#2/#5 同源假设被机器证据否定】Lead 曾提假设：#2（Move 丢价格端点，定义冲突=632）与 #5（source_index 切片下标越界，实现 bug）疑似同源（可能同一根因/统一修复）。机器证据（632 逐字段分析 + coordsys 修复定位）否定此假设——两者层级正交：
     - #2 是【设计契约层】矛盾：canonical ChanlunElements.Move 结构信息不足（无价格端点），movesOf 构造时丢弃 Segment 价格（632）——这是 L0 结构/契约缺陷，须 P1 owner 裁定（escalate）。
     - #5 是【实现疏漏层】bug：slice_date_window 切片后 fill_bar_index 未重置局部下标（实现疏忽，越界吞决策）——这是实现错误，不改任何定义即可修复（commit b6aeabbf8b 已修，testing-override 判据=实现错误正常修复）。
     两者唯一表面相似=『下游拿不到应有的数据/决策』，但根因层级正交（契约信息不足 ⊥ 实现下标越界），不可由单一根因统一解释、不可统一修复。同源假设=L0 模式推测（凭表面相似推测同根因），被 L2 机器证据（逐字段分析 + 代码定位）否定。

  共同结构：三者围绕同一事件（source_index bug）的不同层面。①是工程缺口模式（无先例）；②是修复方案的前瞻边界（raw_source_index）；③是该 bug 与另一缺口（632）的层级关系被误判后由机器证据校正。③本身=formalization-validity-domain『L0 模式推测被 L2 机器证据否定，L0→L2 信息增量为正』的实例（与 625『L1 合成全绿被 L2 真实数据否证』同母规则 231，不同 scope：625 是回测层 L1/L2，本号③是缺口归属推测层 L0/L2）。"
  layer: 编排 + formal/实现层   # ①②=工程实现层（source_index 切片坐标系/溯源字段）；③=编排层（缺口归属推测）+ formal 实现层（#5）与设计契约层（#2,632）的层级辨认
  trigger: "SG-coordsys 工位修复 source_index 坐标系 bug（commit b6aeabbf8b）+ codex 提示 raw_source_index 跨 Dataset 溯源边界 + Lead 提『#2/#5 疑似同源』假设被 632 逐字段分析 + coordsys 修复定位（机器证据）否定"

# 涉及的定义
definitions_involved:
  - name: "谱系无『数据切片不重置局部坐标系』工程缺口先例（核 8 处 Grep 命中）"
    version: ".chanlun/genealogy（pending + settled）全库 Grep：source_index|slice|切片|坐标系|coordsys|局部下标|fill_bar_index|raw_source_index"
    role: "①的核实依据。8 处命中逐一核实**均非**『数据切片不重置局部下标致越界』：(a) 604『切片』= Lean SegmentFromStrokeWindow 的 List.drop/take 连续覆盖笔切片（线段窗口构成，L0 几何），非数据局部下标；(b) turning-node『切片』= 561 轨道空间 E×F 乘积切片 + 纤维投影（代数轨道，非数据下标）；(c) 627『坐标系』= 分层诊断的认识论前置坐标系（引擎层 vs 定义层归属），非数据几何坐标系；(d) settled 535/329/308/062/053 主题（osc-phase/c1-c3-experiment/phase2-calibration/heterogeneity/self-referential）均无数据切片局部下标含义。⟹ 工程缺口①在谱系**无先例**，本号是首记（coordsys 工位核查 (a) 得否定答案=无重复，确认值得立）。"
  - name: "231 形式化有效域规则（L0→L2 信息增量 / 否定性结果价值）"
    version: ".chanlun/genealogy/settled/231 + .claude/rules/formalization-validity-domain.md"
    role: "③的母规则。③『#2/#5 同源假设被机器证据否定』= 231『L0 模式推测信息增量低，L2 否定性结果价值高（缩小有效域边界）』的实例：同源假设是 L0 模式推测（凭表面相似『下游拿不到数据』推测同根因），机器证据（632 逐字段分析 + coordsys 代码定位）是 L2 否证（缩小推测有效域=否定同源）。L0→L2 信息增量为正（否证缩小边界）。②『raw_source_index 须有 L2+ 验证才能声称可跨 Dataset 溯源』也受 231 约束（声称溯源能力须有等大验证，否则声明膨胀 090）。"
  - name: "632 oracle MoveJudgment 消桩=L0 结构缺口非 L2 数据缺口（生成态，#2 的载体）"
    version: ".chanlun/genealogy/pending/632（status: 生成态）"
    role: "③中 #2 的精确载体。632 揭示 #2=canonical Move 丢 Segment 价格端点（设计契约层 L0 结构缺陷，须 P1 裁定）。本号③确认 #2(632) 与 #5(source_index) **层级正交不同源**：632=设计契约层（Move 信息不足，escalate 待 P1）；#5=实现疏漏层（切片下标越界，已 commit 修复）。本号不改 632 结论，只记录二者层级关系（正交，非同源）。632 维持生成态待 /ritual。"
  - name: "625 L2 真实数据暴露 L1 合成全绿掩盖的引擎 bug（生成态，③同母规则不同 scope）"
    version: ".chanlun/genealogy/pending/625（status: 生成态）"
    role: "③的姊妹实例（同母规则 231，不同 scope）。625=231 在『回测层 L1 合成 vs L2 真实』的实例（L1 全绿被 L2 真实否证）。本号③=231 在『缺口归属推测层 L0 模式推测 vs L2 机器证据』的实例（同源假设被机器证据否证）。两者共同：表面信号（L1 全绿 / 同源相似）被更高认识论等级的证据（L2 真实 / L2 机器分析）否证。scope 不同（回测层 vs 缺口归属推测层），共同丰富 231 发生史。"
  - name: "627 双引擎线有效域外推风险（生成态，分层诊断前置坐标系）"
    version: ".chanlun/genealogy/pending/627（status: 生成态）"
    role: "③分层诊断的方法论邻接。627 立『bug 归属哪条引擎线决定缩小哪个有效域，未标注则诊断悬空』的分层诊断前置坐标系。本号③同理：缺口（#5）归属哪一层（设计契约 vs 实现疏漏）决定它如何修复（escalate 待 P1 vs 正常修复），未辨认层级则误判同源。627 是诊断坐标系（认识论），本号③是缺口归属层级辨认（同模式：先辨认层级再诊断）。"
  - name: "no-workaround / no-patch-mentality / 090（约束修复与声明性质）"
    version: ".claude/rules/no-workaround.md + no-patch-mentality.md + settled/090"
    role: "约束 ① ② 的处理性质。①source_index 修复=删错误下标消费逻辑、用切片局部坐标系正确逻辑替换（重置 source_index 与切片对齐），非加 workaround 让『大致能工作』——是诚实重写（no-patch-mentality 要求）。②raw_source_index 当前不补=诚实标注边界（无跨 Dataset 消费者时 source_index 局部一致即正确），不声称 source_index 可跨 Dataset 溯源（声称=声明膨胀 090）；边界被未来消费者触及时补 raw_source_index=届时的严格补全（非现在留 TODO 尾巴）。"

# 解决方式
resolution:
  type: 自动结算候选   # ①bug 已修（commit b6aeabbf8b，实现错误正常修复，testing-override 判据）。③同源假设否定=机器证据已定（层级正交事实）。②raw_source_index=前瞻边界（当前无消费者，无须立即行动）。本号谱系价值=工程缺口模式首记 + 231 ③实例 + raw_source_index 边界落盘，待编排者 /ritual 辨认。
  description: "source_index 坐标系 bug（slice_date_window 切片未重置局部下标 → fill_bar_index 越界吞决策）已由 SG-coordsys 修复（commit b6aeabbf8b，实现错误正常修复：重置 source_index 为切片局部一致）。本号落盘三层谱系意义：①工程缺口模式『数据切片不重置局部坐标系致下标越界』在谱系无先例（核 8 处 Grep 命中无重复），首记；②raw_source_index 跨 Dataset 溯源边界（前瞻：当前无跨 Dataset 溯源消费者→重置不破坏现存溯源→修复正确；边界被未来消费者触及时须补 raw_source_index 保全集下标，不补=声称跨 Dataset 溯源而实际只局部一致=声明膨胀）；③『#2/#5 同源』假设被机器证据否定（层级正交：#2=632 设计契约层 canonical Move 信息不足/escalate 待 P1 ⊥ #5=实现疏漏层切片越界/已修）=231『L0 模式推测被 L2 机器证据否证』的实例（与 625 同母规则不同 scope）。bug 本身属实现层（已修），③是认识论校正记录，②是前瞻边界。谱系价值待编排者 /ritual 辨认。"
  decided_by: 蜂群内部   # genealogist 核 8 处 Grep 命中确认①无先例 + 落盘②raw_source_index 边界 + 记③同源假设被机器证据否定（231 实例）；bug 修复属实现层（coordsys 已 commit）；②的未来行动（补 raw_source_index）由未来跨 Dataset 消费者出现时触发，非现在

# 被否定的方案
negated:
  description: "(1)【同源假设】把 #2(Move 丢价格,632) 与 #5(source_index 切片越界) 当同源，谋求单一根因统一解释/统一修复。(2)【①修复】对 fill_bar_index 越界加边界 clamp/try-except 吞掉越界让『大致能工作』，不重置局部坐标系。(3)【②溯源】现在就声称 source_index 可跨 Dataset 溯源（不区分局部一致 vs 全集溯源）。(4)【②过度工程】当前无跨 Dataset 消费者就先加 raw_source_index 字段（YAGNI 反向：未触边界先加字段）。"
  why_negated: "(1) 机器证据否定：632 逐字段分析证 #2=canonical Move 结构信息不足（设计契约层 L0 缺陷）；coordsys 修复定位证 #5=切片下标未重置局部坐标系（实现疏漏层）。两者层级正交（契约 ⊥ 实现），唯一相似=表面『下游拿不到数据』——凭表面相似推同源=L0 模式推测，被 L2 机器证据否证（231：L0 推测信息增量低，L2 否证价值高）。(2) 补丁思维（no-patch-mentality §1）：clamp/吞越界=在错误坐标系混用上加 workaround，不修根因（局部/全局坐标系混用）。正确=重置 source_index 为切片局部坐标系（删错误逻辑用正确逻辑替换）。(3) 声明膨胀（090）：source_index 重置后只在单 Dataset 内局部一致，声称可跨 Dataset 溯源=声明代码不具备的能力。(4) 边界前置加字段=过度工程；当前无消费者，加 raw_source_index 是无验证需求的复杂度。诚实=标注边界（当前局部一致正确，未来消费者触及时补），非现在补也非声称已具备。"

# 新产出
new_output:
  definitions:
    - "工程缺口模式①：数据切片不重置局部坐标系致下标越界（slice_date_window 切片生成新局部序列但 fill_bar_index 仍按全集下标消费 → 越界吞决策）——NewChanlun 谱系无先例（核 8 处 source_index/切片/坐标系 Grep 命中均非数据局部下标），首记"
    - "raw_source_index 跨 Dataset 溯源边界②：重置 source_index 为切片局部一致在『单 Dataset 内消费』正确（当前无跨 Dataset 消费者）；若未来有跨 Dataset 溯源消费者，重置破坏全集溯源，须补 raw_source_index 保原始全集下标——前瞻边界值，当前不补（不补不是缺陷，是诚实标注边界；补 = 届时严格补全；声称已可跨 Dataset 溯源 = 声明膨胀 090）"
    - "跨层误判模式③：『#2 Move 丢价格(632 设计契约层 L0 结构缺陷) ⊥ #5 source_index(实现疏漏层切片越界)同源』假设被机器证据否定——层级正交不可统一，唯一表面相似=『下游拿不到数据』，凭此推同源=L0 模式推测被 L2 机器证据否证"
    - "③ = 231『L0 模式推测被 L2 机器证据否定，L0→L2 信息增量为正/否定性结果价值高』的实例（与 625 同母规则 231，不同 scope：625=回测层 L1/L2，本号③=缺口归属推测层 L0/L2）"
    - "缺口归属层级辨认纪律：辨认缺口属设计契约层（escalate 待 owner 裁定）还是实现疏漏层（正常修复）须先做（627 分层诊断前置坐标系同模式），凭表面相似推同源=误判风险"
  code_changes: "无谱系侧代码改动（本号是缺口模式 + 边界 + 误判校正记录）。source_index bug 修复（commit b6aeabbf8b）由 SG-coordsys 工位执行（实现层正常修复，重置 source_index 为切片局部一致），非 genealogist 职责。raw_source_index 字段（②）当前不补（无跨 Dataset 消费者），未来触边界时由相应工位补。genealogist 未独立核实 commit b6aeabbf8b 的代码 diff（coordsys 工位转述 + Grep 核实谱系无先例；行号/commit hash 来自工位转述——见认识论诚实）。"
  orchestration_changes: "方法论：①数据切片操作（按日期/范围窗口切片）生成新局部坐标系，所有下游下标消费方须重置到局部坐标系（不可混用全集下标）——否则越界吞数据。②修复方案的溯源能力须标注有效域（局部一致 vs 全集溯源），不声称未验证的跨 Dataset 溯源（090）；前瞻边界值（如 raw_source_index）落盘但当前不实现（YAGNI），未来触边界时补。③缺口归属层级须先辨认（设计契约层 escalate / 实现疏漏层正常修复）再处理——凭表面相似（『下游拿不到数据』）推同源=L0 模式推测，须 L2 机器证据（逐字段分析/代码定位）校正（627 分层诊断前置坐标系同模式）。④Lead 提的跨缺口同源假设是 L0 推测，须机器证据验证后才能用于统一修复决策（否则误判）。"

# 影响范围
impact:
  affected_modules:
    - "source_index 切片坐标系（slice_date_window + fill_bar_index 消费链，commit b6aeabbf8b 修复，coordsys 转述未独立核实代码）→ 局部坐标系重置（bug 已修，实现层）"
    - "raw_source_index（②前瞻字段，当前不存在）→ 跨 Dataset 溯源消费者出现时须新增，保原始全集下标"
    - "632（生成态，#2 载体）→ 本号确认 #2(632) ⊥ #5(source_index) 层级正交不同源，不改 632 结论。632 维持生成态待 /ritual"
    - "231（已结算）→ ③是其在『缺口归属推测层』的实例（L0 模式推测被 L2 机器证据否定）。维持 settled，本号是确认性实例"
    - "625（生成态）→ ③的姊妹实例（同母规则 231 不同 scope）。不修改 625"
  affected_definitions:
    - "231（已结算）：③实例化确认其『L0→L2 信息增量为正/否定性结果价值』在缺口归属推测层成立。维持 settled，待编排者 /ritual 辨认是否纳入发生史。"
    - "632（生成态）：本号确认 #2(632) 与 #5(source_index) 层级正交不同源，不修改 632 的设计契约层结论。维持生成态。"
  downstream_implications:
    - "数据切片操作须重置所有下游下标消费方到局部坐标系（防越界吞数据）——工程通则①"
    - "source_index 重置后只局部一致，不可声称跨 Dataset 溯源；跨 Dataset 消费者出现时补 raw_source_index（②前瞻边界）"
    - "#2(632) 与 #5(source_index) 须分别处理：#2 escalate 待 P1 裁定 canonical 契约 / #5 已 commit 修复——不可统一修复（层级正交）"
    - "跨缺口同源假设是 L0 推测，须 L2 机器证据验证（逐字段/代码定位）才能用于统一修复决策（③校正模式）"

# 谱系关联
related_records:
  parent: "231号（形式化有效域规则）——③『#2/#5 同源假设被机器证据否定』是其『L0 模式推测被 L2 机器证据否证，L0→L2 信息增量为正』在缺口归属推测层的实例"
  children: []
  related:
    - "632号（oracle MoveJudgment L0 结构缺口）：#2 的载体；本号确认 #2(632 设计契约层) ⊥ #5(实现疏漏层)层级正交不同源"
    - "625号（L2 真实暴露 L1 合成全绿掩盖 bug）：③的姊妹实例（同母规则 231，不同 scope：625=回测层 L1/L2，本号③=缺口归属推测层 L0/L2）"
    - "627号（双引擎线分层诊断前置坐标系）：缺口归属层级辨认同模式（先辨认层级再诊断/修复）"
    - "623号（231 在自指工具/spec 层实例化）：231 跨层连续实例化族，本号③是又一 scope（缺口归属推测层）"
    - "090号（声明膨胀）：②不声称 source_index 可跨 Dataset 溯源（重置后只局部一致）"
    - "no-workaround/no-patch-mentality：①修复用正确局部坐标系替换（非 clamp/吞越界 workaround）；②前瞻边界落盘非 TODO 尾巴"
    - "MEMORY l2-engine-incompleteness-vs-theta-falsification：分层诊断铁律（#5 属实现层 bug 非 Θ 否证，正常修复）"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "工程缺口模式①『数据切片不重置局部坐标系致下标越界』在谱系无先例"
    level: "L0/L1（核 8 处 source_index/切片/坐标系 Grep 命中逐一核实均非数据局部下标——604=Lean List 切片/turning-node=E×F 代数切片/627=认识论坐标系/settled 5 文件主题无关）"
    increment: "高：确认工程缺口①首记（coordsys 工位核查 (a) 得否定答案=无重复）"
  - proposition: "source_index 重置为切片局部一致在单 Dataset 内消费正确；跨 Dataset 溯源须补 raw_source_index"
    level: "L0（坐标系机制：切片生成局部序列，局部下标与切片对齐=局部一致；跨 Dataset 需全集下标=raw_source_index）→ 当前无跨 Dataset 消费者（边界未触及，无须 L2 验证）"
    increment: "中：前瞻边界值落盘（当前局部一致正确，未来消费者触边界时补，不补不是缺陷）"
  - proposition: "③『#2(Move 丢价格,632) 与 #5(source_index bug) 同源』假设被机器证据否定"
    level: "L0 模式推测（凭表面相似『下游拿不到数据』推同源）→ 被 L2 机器证据否证（632 逐字段分析证 #2=设计契约层 + coordsys 修复定位证 #5=实现疏漏层，层级正交）"
    increment: "正（否定性结果——L2 机器证据缩小 L0 推测有效域：否定同源，确认层级正交不可统一修复）"
  - proposition: "③ = 231『L0 模式推测被 L2 机器证据否定』在缺口归属推测层的实例"
    level: "L0（231 论断 + ③对照实例 + 625 同母规则不同 scope 印证）"
    increment: "高：231 发生史延伸到缺口归属推测层（L0 推测 vs L2 机器证据）"

---

# 矛盾发现 633：source_index 坐标系 bug 的两层谱系意义 + #2/#5 同源假设被机器证据否定

## 一句话结论

`source_index` 坐标系 bug（`slice_date_window` 按日期窗口切片后**未重置局部下标**，致 `fill_bar_index` 按全集下标消费而越界，**吞掉本应产生的决策**；commit `b6aeabbf8b` 由 SG-coordsys 修复）暴露**三层谱系意义**，其中第③层是一个**被机器证据否定的同源假设**：

| # | 层 | 内容 | 类型 | 状态 |
|---|---|---|---|---|
| ① | 工程缺口模式 | 数据切片不重置局部坐标系致下标越界 | 工程缺口（谱系**无先例**，首记） | bug 已修（commit b6aeabbf8b） |
| ② | 修复方案前瞻边界 | raw_source_index 跨 Dataset 溯源边界 | 前瞻边界值（当前无消费者，不补） | 边界落盘，未触及 |
| ③ | 跨层误判模式 | 『#2 Move 丢价格 ⊥ #5 source_index』同源假设被机器证据否定 | 231 实例（L0 推测被 L2 否证） | 假设已否定，层级正交确认 |

**bug 本身 = 实现错误**（testing-override 判据：不改任何定义即可修复 → 实现错误，正常修复），已 commit。本号是其**谱系意义落盘**，非对定义的否定。

## ① 工程缺口模式：无先例核实

coordsys 工位明确请求核查 (a) 是否已有「切片不重置局部坐标」工程谱系（避免重复）。**核实结果：无先例。**

全库 Grep（`source_index|slice|切片|坐标系|coordsys|局部下标|fill_bar_index|raw_source_index`）命中 8 处，逐一核实**均非**「数据切片不重置局部下标致越界」：

| 命中 | 「切片/坐标系」实际含义 | 是否同对象 |
|---|---|---|
| 604 | Lean `SegmentFromStrokeWindow` 的 `List.drop/take` 连续覆盖笔切片（线段窗口构成，L0 几何） | 否（Lean 列表切片 ≠ 数据局部下标） |
| turning-node | 561 轨道空间 E×F 乘积切片 + 纤维投影（代数轨道） | 否（代数切片 ≠ 数据下标） |
| 627 | 分层诊断的认识论前置坐标系（引擎层 vs 定义层归属） | 否（认识论坐标系 ≠ 数据几何坐标系） |
| settled 535/329/308/062/053 | osc-phase / c1-c3-experiment / phase2-calibration / heterogeneity / self-referential（主题无关） | 否 |

⟹ 工程缺口①在谱系**无先例**，本号首记。coordsys 核查 (a) 得否定答案。

## ② raw_source_index 跨 Dataset 溯源边界

coordsys 工位转述 codex 提示：现修复**重置 source_index 为切片局部一致**，若未来有**跨 Dataset 溯源消费者**（需把切片内局部下标映射回原始全集下标），重置会破坏全集溯源 → 须新增 `raw_source_index` 保原始全集下标。

**判定（前瞻边界值）**：
- **当前正确**：无跨 Dataset 溯源消费者 → 重置 source_index 为局部一致不破坏任何现存溯源 → 修复正确。
- **当前不补 raw_source_index**：无消费者就加字段 = 过度工程（YAGNI）；不补**不是缺陷**，是诚实标注边界。
- **边界被触及时补**：未来跨 Dataset 消费者出现 → 须补 raw_source_index（届时严格补全，非现在留 TODO 尾巴）。
- **禁声明膨胀**：当前不可声称 source_index 可跨 Dataset 溯源（重置后只单 Dataset 内局部一致）——声称 = 声明代码不具备的能力（090）。

## ③ 跨层误判模式：#2/#5 同源假设被机器证据否定

Lead 曾提假设：**#2**（Move 丢价格端点，定义冲突 = 632）与 **#5**（source_index 切片下标越界，实现 bug）疑似**同源**。机器证据否定此假设——**两者层级正交**：

| | #2（632） | #5（source_index bug） |
|---|---|---|
| 层 | **设计契约层** | **实现疏漏层** |
| 根因 | canonical `Move` 结构信息不足（无价格端点），`movesOf` 丢 Segment 价格 | `slice_date_window` 切片后 `fill_bar_index` 未重置局部下标（越界） |
| 认识论等级 | L0 结构/契约缺陷 | 实现错误 |
| 处理 | escalate 待 P1 owner 裁定 canonical 契约（632） | 正常修复（commit b6aeabbf8b 已修） |
| 机器证据 | 632 逐字段分析（三 Bool 字段不可纯 L0 推导，根因 Move 无价格） | coordsys 修复定位（切片局部坐标系未重置） |

唯一**表面相似** = 「下游拿不到应有的数据/决策」。凭表面相似推同源 = **L0 模式推测**，被 **L2 机器证据**（逐字段分析 + 代码定位）否证——这正是 **231『L0 模式推测被 L2 机器证据否定，L0→L2 信息增量为正/否定性结果价值高』**的实例。

与 **625** 同母规则 231、不同 scope：
- 625：回测层 L1 合成全绿被 L2 真实数据否证。
- 本号③：缺口归属推测层 L0 同源假设被 L2 机器证据否证。

两者共同：**表面信号（L1 全绿 / 同源相似）被更高认识论等级证据（L2 真实 / L2 机器分析）否证**。

## 定义依据

- 231 / formalization-validity-domain.md：L0 模式推测信息增量低；L2 否定性结果价值高（缩小有效域边界）；声称溯源能力须有等大验证（否则声明膨胀）。
- 632：#2 = canonical Move 丢 Segment 价格（设计契约层 L0 结构缺陷）的逐字段证明。
- coordsys 工位转述：commit b6aeabbf8b 修复 source_index 切片局部坐标系 + codex raw_source_index 提示。
- 8 处 Grep 命中核实：工程缺口①无先例。

## 边界条件（结论翻转）

- 若 commit b6aeabbf8b 经核实**未真正重置局部坐标系**（仅 clamp/吞越界）→ ①仍是开放 bug（补丁未解根因），非已修。当前 coordsys 转述=重置局部一致（重写非补丁）。
- 若未来出现**跨 Dataset 溯源消费者** → ②边界被触及，须补 raw_source_index；不补则 source_index 溯源声明膨胀。当前无消费者，边界未触及。
- 若机器证据（632 逐字段 / coordsys 定位）经复核**有误** → ③同源假设的否定可能翻转；但当前两份独立机器证据（逐字段分析 + 代码定位）一致指向层级正交。
- 若 genealogist 后续独立核实 commit hash / 行号与 coordsys 转述不符 → 代码位置修正，但①②③的谱系论断不依赖具体 hash（依赖的是切片坐标系机制 / 层级正交事实 / Grep 无先例）。

## 下游推论

- 数据切片操作须重置所有下游下标消费方到局部坐标系（工程通则①，防越界吞数据）。
- source_index 重置后只局部一致，跨 Dataset 消费者出现时补 raw_source_index（②前瞻边界），当前不声称跨 Dataset 溯源。
- #2(632) 与 #5(source_index) 须**分别处理**（escalate 待 P1 / 已修），不可统一修复（层级正交）。
- 跨缺口同源假设是 L0 推测，须 L2 机器证据验证后才能用于统一修复决策（③校正模式）。

## 谱系引用

- 母规则：231（形式化有效域规则——L0 模式推测被 L2 机器证据否证）。③是其在缺口归属推测层的实例。
- #2 载体：632（oracle MoveJudgment L0 结构缺口）——本号确认 #2 ⊥ #5 层级正交不同源。
- 姊妹实例：625（L2 真实暴露 L1 合成 bug）——同母规则不同 scope。
- 分层诊断邻接：627（双引擎线分层诊断前置坐标系）——缺口归属层级辨认同模式。
- 约束：090（声明膨胀，②不声称跨 Dataset 溯源）；no-workaround/no-patch-mentality（①重写非补丁，②前瞻边界非 TODO 尾巴）。

## 影响声明

落盘 source_index 坐标系 bug（commit b6aeabbf8b 已修）的三层谱系意义：①工程缺口模式『数据切片不重置局部坐标系致下标越界』在谱系无先例（核 8 处 Grep 命中无重复），首记；②raw_source_index 跨 Dataset 溯源前瞻边界（当前无消费者不补，未来触边界补，不声称跨 Dataset 溯源）；③『#2/#5 同源』假设被机器证据否定（层级正交：632 设计契约层 ⊥ source_index 实现疏漏层）=231『L0 模式推测被 L2 机器证据否证』实例（与 625 同母规则不同 scope）。bug 本身属实现层（已修）。不破坏任何 settled 谱系（231 维持 settled，本号③是确认性实例）。不改任何代码/定义、不修改 632、不擅自补 raw_source_index、不擅自结算。

## 认识论诚实（formalization-validity-domain + 090）

- 工程缺口①无先例 = **L0/L1**（8 处 Grep 命中逐一核实，genealogist 独立完成谱系全库 Grep）。
- commit b6aeabbf8b / 代码行号 = **coordsys 工位转述**，genealogist **未独立核实代码 diff**（Glob 全库超时——LFS 大文件拖累遍历，与 622/623/624/625 同模式：genealogist 工具有效域受限）。①②③谱系论断**不依赖具体 commit hash/行号**（依赖切片坐标系机制 / 层级正交事实 / Grep 谱系无先例）。
- ③同源假设否定 = **L2 机器证据**（632 逐字段分析 + coordsys 修复定位，两份独立证据），但其中 coordsys 定位是工位转述（见上）；632 逐字段分析是 genealogist 已读核实（632 全文已读）。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（g-complete-classification-full-strategy）：632（oracle MoveJudgment，#2 载体）/629（M1 三开口）/625（L2 暴露 bug）/627（双引擎线）/628（构造层闭合）。
- 1-hop：231/632/625/627/623/090/no-workaround/no-patch-mentality。
- Hub：231（有效域规则，度高）/632（#2 载体）。

### 张力1：vs 632——确认层级正交不同源，非否定 632
632 揭示 #2=设计契约层（canonical Move 信息不足）。本号确认 #2(632) ⊥ #5(source_index 实现疏漏层)层级正交不同源——不改 632 的设计契约层结论，只记录二者层级关系。**可分层**（契约层 ⊥ 实现层），无矛盾，无中断#1。

### 张力2：vs 231——③是实例化确认非否定
231『L0 模式推测被 L2 机器证据否证』正确。本号③是其在缺口归属推测层的实例（同源假设被机器证据否定），实例化确认。**一致**，无矛盾。

### 张力3：vs 625——同母规则不同 scope，互补
625=231 在回测层（L1 合成被 L2 真实否证）。本号③=231 在缺口归属推测层（L0 同源推测被 L2 机器证据否证）。同母规则两 scope，互补丰富 231 发生史，无矛盾。

### 张力4：vs 627——分层诊断同模式
627 立分层诊断前置坐标系（bug 归属哪条引擎线）。本号③同模式：缺口归属哪一层（设计契约/实现疏漏）须先辨认再处理。**同模式深化**，无矛盾。

### 递归运动结构完成检测（020）
- 第0层：本号写入（source_index bug 三层谱系意义）。
- 第1层：本号 × 632 碰撞 → #2/#5 层级正交确认（净新发现高：同源假设被否定，层级正交=不可统一修复）。
- 第2层：本号 × 231/625 碰撞 → L0 模式推测被 L2 机器证据否证的实例化（净新发现：231 发生史延伸到缺口归属推测层）。
- 第3层：本号 × 627/623 碰撞 → 分层诊断/231 跨层同模式确认（净新发现骤降=背驰：分层辨认是已知模式）。
- 涉及范围：scope₁(632 层级正交) > scope₂(231/625 实例化) > scope₃(627/623 同模式)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** bug 修复属实现层（coordsys 已 commit）；raw_source_index 前瞻边界待未来消费者触发；③同源否定=机器证据已定。本号是谱系意义落盘 + 误判校正记录，不触发新 /escalate（无新生不可分层矛盾——三层均可分层：①工程缺口/②前瞻边界/③认识论校正）。

## 回溯扫描（职责3）

- **231（settled）**：本号③实例化确认其『L0 模式推测被 L2 机器证据否证』在缺口归属推测层成立，不否定，不破坏结算。维持 settled，待编排者 /ritual 辨认是否纳入发生史。
- **632（生成态）**：本号确认 #2(632) ⊥ #5 层级正交不同源，不修改 632 设计契约层结论，不影响其待 /ritual 状态。
- **625/627/623（生成态）**：本号③与 625 同母规则不同 scope、与 627 分层诊断同模式、与 623 同 231 跨层实例族——均不修改其内容。
- **090/no-workaround/no-patch-mentality（rules）**：本号遵守其约束（①重写非补丁、②不声明膨胀、②前瞻边界非 TODO 尾巴），不违反、不破坏。
- **无 settled 被本号回溯破坏。** 本号是 source_index bug 三层谱系意义的落盘（工程缺口首记 + raw_source_index 前瞻边界 + 同源假设被机器证据否定），bug 修复属实现层（已 commit），谱系价值待编排者 /ritual。
