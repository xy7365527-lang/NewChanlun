---
id: "688"
number: 688
status: 生成态   # codex-decide-20260702-213839-7db4 已裁「接受性截断」（关闭条件 ii 的裁定部分已满足）；本条目=该裁定的落谱系。转 settled 待：(1) 编排者 /ritual 最终辨认（同 680「最终辨认待编排者 /ritual」）；(2) 施工级规格的文档义务核销（全库禁止「全域去根化 C_Θ 完全实现」声称——尚未逐处核对）。
date: "2026-07-03"
type: bias-correction   # 订正「theta_v0 已全域实现原文去根化平移自相似 𝒞_Θ(S_k x)=S_k𝒞_Θ(x)」的潜在声明膨胀——有效域实为内部域 lvl>=1，level0 是有限塔物理边界。
source: "[新缠论] 递归完全分类买卖点.pdf §⑤（硬公理 𝒞_Θ(S_k x)=S_k𝒞_Θ(x)）vs econ_positive.rs:811-838/1092-1134 实装 + dlpdf-a §5 定性（有限塔截断）+ codex-decide-20260702-213839-7db4.md 矛盾-B 终局裁定 + gap-master2-shard1-20260703.md §2/§3（B 类登记）"
negation_source: heterogeneous   # 终局裁定=codex-cli（codex-decide-20260702-213839-7db4，task #78）；初始定性=dlpdf-a §5 逐页读（homogeneous）+ ws-gap2s1 分片 B 类登记（homogeneous）。
negation_model: codex-cli
negation_form: separation   # 「全级别去根化自相似」统一范畴在有限塔物质化后，暴露 level0（截断边界，无次级别）与 lvl>=1（内部域）的结构异质性——两者不可完全同质比较（codex：「二者不在同一层面比较」）。非 680 式声明-实装脱节（那是 expansion）。
depends_on: ["231", "090"]
related: ["680", "625", "623", "673", "199", "656"]

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：全域平移自相似声明——「𝒞_Θ 在所有级别等变，level0 无特殊免门规则」（递归完全分类买卖点.pdf §⑤ 硬公理）。
# 实际拓扑后果（retrospective 141号结论1）：level0 被切断出平移自相似有效域，成为独立的「finite-tower boundary adapter」路径——
#   代码级 sever 的字面形态 = XzdEvidence.level 恒 >=1 不变量（econ_positive.rs:1095「lvl==0 时 base gate 免门走区间套，
#   不入本通道」）+ cand_delta_type2/3 的 `lvl == 0 ||` 免门分支（:818/:837）。level0 结构性排除于 XZD 通道，恒走
#   免门 Nest 路径；lvl>=1 保留统一 descend-anchor 判据 + XZD 通道。两条路径互不污染 ⟹ 切断，非分裂。
#   scope=downstream：level0 免门的 base-case 沿 extract_second_for_level 上溯构成高级别二类，且 codex 风险明示
#   「alpha 统计可能被 L0 base-case 主导」——切断的经济后果覆盖下游 alpha 统计路径。
topo_effect: "sever:level0-existence-gate:downstream"

# 矛盾（bias-correction 的被订正对象）
contradiction:
  description: |
    原文硬公理（递归完全分类买卖点.pdf §⑤）：全分类函数级别平移自相似 𝒞_Θ(S_k x)=S_k𝒞_Θ(x)——
    整个分类函数在级别平移下等变，**任何级别不应有特殊规则**，只应有统一的、随级别平移不变的判据。
    本仓库实装在 level0 违反此等变性：
    (1) `cand_delta_type2_completion` / `cand_delta_type3_retest`（econ_positive.rs:811-838）判据为
        `lvl == 0 || descend_type1_anchor_depth(...).is_some()`——lvl==0 时存在性**免门**（Type2/3 恒真），
        lvl>=1 时须下沉次级别 Type1 锚（第29课L396「二三类精确点要下次级别以下找第一类」）。
        判据规则本身在 lvl==0 与 lvl>=1 之间**不是平移不变的**（前者恒真，后者需 descend 锚）。
    (2) `XzdEvidence.level` 恒 >=1（econ_positive.rs:1095 字段不变量注释：「lvl==0 时 base gate 免门走区间套，
        不入本通道」）——小转大（XZD）通道**结构性排除 level0**；且 `gate_pass`（:1132-1134）自身按
        `level==1`（C2∧C3 硬门）/ `level>=2`（C2-only）分支，级别内部也非完全同质。
    合并效果：level0 是一个带特殊免门规则的特殊级别（恒 Nest + 免门 + 排除于 XZD），违反「任何级别不应有
    特殊规则」的硬公理。这是编排者/异质审「回测跑在未完全实装的全互斥/去根化定义上」在**级别域**的字面机制。
  layer: 概念   # 原文公理（全域等变） vs 实装（level0 特殊化）的口径矛盾。非纯代码 bug：实装行为 codex 裁定「维持」，矛盾在「声明有效域」层——是否可宣称全域 C_Θ 完全实现。
  trigger: "dlpdf-a 8 份 PDF 逐页全读定性（§5）→ ws-gap2s1 分片1 B 类登记（§2 矛盾-B/§3 B-s1-①）→ codex-decide-20260702-213839-7db4（task #78 A 组矛盾-A/B 终局裁定）→ team-lead 派 #161 立条。"

# 涉及的定义
definitions_involved:
  - name: "去根化级别平移自相似公理 𝒞_Θ(S_k x)=S_k𝒞_Θ(x)"
    version: "docs/formal-chain/ 递归完全分类买卖点.pdf §⑤（唯一权威起点形式化）"
    role: "被违反的原文硬公理（定义域=全级别，无特殊级别）。level0 免门是其在实装的截断边界。"
  - name: "231 形式化有效域规则（有效域 ≤ 定义域）"
    version: ".chanlun/genealogy/settled/231 + .claude/rules/formalization-validity-domain.md"
    role: "母规则。本号=231 在**级别域**的形态：平移自相似的定义域=全级别（去根化），有效域=内部域 lvl>=1 && tower[lvl-1] exists；level0 在有效域之外（有限塔物理边界）。声明全域=有效域膨胀。"
  - name: "cand_delta_type2_completion / cand_delta_type3_retest（econ_positive.rs:811-838）"
    version: "673-fix（HEAD，gap3-rework-codex9-fix 分支）"
    role: "level0 免门的代码 locus（`lvl == 0 ||`）。存在性锚=第29课L396 定律一下沉；lvl==0 无次级别⟹锚不可施⟹免门（有限塔递归底的必然，非任意特殊规则）。"
  - name: "XzdEvidence（econ_positive.rs:1092-1134）"
    version: "codex #44 终局裁定(c)（HEAD）"
    role: "小转大通道准入证书。`level` 字段恒 >=1（level0 结构性排除）+ gate_pass 按 level==1/level>=2 分支——level0 sever 出 XZD 通道的字面代码形态。"

# 解决方式
resolution:
  type: 定义修正   # codex 裁「维持免门 + 定性有效域边界 + 禁止全域声称」——修的是「声明口径」（有效域标注），非「代码行为」。
  description: |
    codex-decide-20260702-213839-7db4（矛盾-B 终局裁定）：
    **决策**：维持 `lvl==0` 免门；把它定性为有限塔实现的**有效域边界**，不声称 theta_v0 已全域实现原文
    去根化平移自相似。拒绝「去根化重构」备选（让 level0 也统一走次级别 Type1 下沉判据）——代价是 lvl==0
    无次级别时 descend 恒 None ⟹ level0 的 Type2/3 **全部门拒**，行为剧变且无原文依据要求。
    **施工级规格**：不改 Rust 行为。文档/裁定口径必须写清：平移自相似只可声明在内部域
    `lvl >= 1 && tower[lvl-1] exists`；`lvl==0` 是 finite-tower boundary adapter。**禁止**把当前 theta_v0
    宣称为原文全域去根化 C_Θ 的完全实现。
    **本条目 = 关闭条件(ii)「裁定为有限塔实装的接受性截断并落谱系」的落谱系产出**（shard §3）。
  decided_by: 蜂群内部（codex-cli 异质终局裁定，task #78；team-lead 派 #161 立条落盘）

# 被订正的方案
negated:
  description: "隐含声明：theta_v0 已全域实现原文去根化平移自相似分类函数 𝒞_Θ（所有级别用同一套随平移不变的判据，level0 无例外）。"
  why_negated: |
    (1) level0 与 lvl>=1 走**不同**的存在性判据（免门恒真 vs descend-anchor），判据规则本身非平移不变——
        与原文「同一提升算子 Prom 在前沿再作用」（所有级别同一套判据，只是作用对象随级别提升而变，判据本身
        不因级别而变）在结构上不同。
    (2) 但这**不是** L0 逻辑矛盾（dlpdf-a §5）：原文的「级别」是无限递归的抽象前沿，实装塔有限，`lvl==0`
        是塔的**物理边界**而非「特殊级别规则」——二者不在同一层面比较。level0 无次级别 ⟹ 第29课下沉锚
        物理上不可施加 ⟹ 免门是有限塔截断的**必然**边界效应，非任意特例。
    (3) 故被否定的不是免门行为本身（codex 裁「维持」），而是「全域已实现去根化自相似」的**声明**——
        有效域实为内部域 lvl>=1，声明全域=231 有效域膨胀 + 090 声明膨胀。

# 新产出
new_output:
  definitions:
    - "231 级别域形态：平移自相似公理的定义域=全级别（去根化=无根/无特殊级别），有效域=有限塔内部域
      `lvl>=1 && tower[lvl-1] exists`。level0 = finite-tower boundary adapter，在有效域之外。区别于
      680（消费侧 discard，同一 econ_positive.rs 路径的另一维度）与 625（计算侧 bug）——231 的第三个 scope=级别域截断。"
    - "有限塔截断判据：递归判据若依赖「下沉次级别」（第29课定律一下沉族），则递归底（level0，无次级别）
      必然免门——该免门不是可选特例，是有限塔对无限自相似理想截断的必然边界效应。凡此类判据，其平移
      等变性只可声明在内部域，边界级别须显式标注为 boundary adapter（不得纳入全域等变声明）。"
  code_changes: "无（codex 施工级规格：不改 Rust 行为）。约束的是文档/声明口径，非代码。"
  orchestration_changes: "文档核销义务（施工级规格）：全库凡宣称 theta_v0 实现原文 C_Θ / 去根化平移自相似处，须加有效域限定 `lvl>=1 && tower[lvl-1] exists`，level0 标注 finite-tower boundary adapter。归 quality-guard 声明膨胀巡检 + knowledge-crystallization 候选。"

# 影响范围
impact:
  affected_modules:
    - "econ_positive.rs:811-838（cand_delta_type2/3 免门分支）+ :1092-1134（XzdEvidence.level 不变量 + gate_pass 级别分支）——代码不改，语义有效域被本条目锚定。"
    - "所有宣称「theta_v0 完全实现原文去根化平移自相似 / 全域 C_Θ」的文档/报告——须加 lvl>=1 有效域限定（施工级规格）。"
    - "alpha 统计路径：codex 风险明示 level0 base-case 可能主导 alpha 统计（L0 与内部级别不可完全同质比较）——W-VERIFY/econ_positive 分级 alpha 解读须区分 level0（boundary）与 lvl>=1（interior）。"
  affected_definitions:
    - "𝒞_Θ 全分类函数（权威全本 §⑤）——实装有效域=内部域 lvl>=1，非全域。"
    - "199（t8-level0-inconclusive）：level0 alpha INCONCLUSIVE 是本号「level0 是异质边界级别」的实证侧表现之一（level0 不可与内部级别同质池化统计）。"
  downstream_implications:
    - "同轮姊妹：680（消费侧 discard，级别域之外的另一 231 locus）与本号（级别域截断）同源于 codex-decide-20260702-213839-7db4 同一裁定会话——坐实 231 在实装层多个正交 scope。"
    - "关闭条件(i)（level0 也走统一区间套判据）被 codex 否决为行为剧变（level0 Type2/3 全门拒，无原文依据）；关闭条件(ii)（接受性截断落谱系）被采纳，本条目即其落盘。转 settled 待编排者 /ritual 最终辨认 + 文档全域声称核销。"
    - "凡「递归底 / 边界级别免门」类实装（依赖下沉次级别的判据族），均须按本号判据标注有效域边界，不得纳入全域等变声明。"

# 回溯结算（如果适用，待编排者 /ritual + 文档核销后补记）
retroactive_settlement:
  settled_by: "编排者 /ritual 最终辨认（同 680 待辨认口径）+ 施工级规格文档义务核销（全库无「全域去根化 C_Θ 完全实现」未限定声称）后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "231（形式化有效域规则）——本号是其在**级别域**的形态（平移自相似定义域=全级别，有效域=内部域 lvl>=1）。"
  children: []   # 文档核销后可派生「有效域标注已全库落地」的实证子记录。
---

# bias-correction 688：level0 免门破去根化平移自相似——231 的级别域形态（有限塔截断）

## 结论

原文硬公理（递归完全分类买卖点.pdf §⑤）要求全分类函数级别平移自相似
`𝒞_Θ(S_k x)=S_k𝒞_Θ(x)`——任何级别不应有特殊规则。本仓库实装在 **level0 违反此等变性**：
`cand_delta_type2_completion`/`cand_delta_type3_retest`（`econ_positive.rs:811-838`）的
`lvl == 0 ||` 分支使 level0 存在性**免门**（Type2/3 恒真），而 lvl>=1 须下沉次级别 Type1 锚
（第29课L396）；`XzdEvidence.level` 恒 >=1（`:1095`）使小转大通道**结构性排除 level0**。level0 因此
是带特殊免门规则的特殊级别（恒 Nest + 免门 + 排除于 XZD）。

但这**不是 L0 逻辑矛盾**（dlpdf-a §5）：原文「级别」是无限递归的抽象前沿，实装塔有限，`lvl==0` 是塔的
**物理边界**——level0 无次级别 ⟹ 第29课下沉锚物理上不可施加 ⟹ 免门是有限塔截断的**必然**边界效应，
非任意特例。codex-decide-20260702-213839-7db4（矛盾-B 终局裁定）裁：**维持免门，定性为有限塔有效域
边界，禁止宣称 theta_v0 全域实现去根化 C_Θ**。被否定的不是免门行为（维持），而是「全域已实现平移
自相似」的**声明**——有效域实为内部域 `lvl>=1 && tower[lvl-1] exists`（231 有效域膨胀 + 090 声明膨胀）。

## 定义依据

- 递归完全分类买卖点.pdf §⑤：`𝒞_Θ(S_k x)=S_k𝒞_Θ(x)` 硬公理（整个分类函数级别平移等变，无特殊级别）。
- 第29课L396：「二三类精确点要下次级别以下找第一类」——lvl>=1 存在性判据的下沉锚来源；level0 无次级别故不可施加。
- 代码锚：`econ_positive.rs:818/837`（`lvl == 0 ||` 免门）、`:1095`（`XzdEvidence.level` 恒 >=1 不变量）、
  `:1132-1134`（`gate_pass` 按 level==1/level>=2 分支）。三处均本次独立读码核实。
- 231（形式化有效域）：定义域=全级别（去根化），有效域=内部域 lvl>=1。level0 在有效域之外。

## 边界条件（结论翻转）

- (a) 若采纳关闭条件(i)——让 level0 也统一走「次级别 Type1 下沉」判据——则平移等变在全域恢复，本号
  的「有效域<定义域」订正翻转。**代价**：lvl==0 无次级别 ⟹ `descend_type1_anchor_depth` 恒 None ⟹
  level0 Type2/3 **全部门拒**（行为剧变，无原文依据要求）。codex 已否决此备选。
- (b) 若后续裁定「有限塔物理边界」与「原文抽象无限前沿」应在同一层面比较（推翻 dlpdf-a §5 的「二者不在
  同一层面」定性），则本号从「有效域边界」升级为「实质违反」，须走关闭条件(i) 修复而非接受性截断。
- (c) 若代码后续把 level0 也纳入 XZD 通道（`XzdEvidence.level` 不变量放宽至含 0），则 sever 消解，本号
  的拓扑效果须重评。

## 下游推论

- 任何「theta_v0 完全实现原文 C_Θ / 去根化平移自相似」的声明须加有效域限定 `lvl>=1 && tower[lvl-1] exists`，
  level0 标注 finite-tower boundary adapter（施工级规格，quality-guard 巡检项）。
- alpha 统计：codex 风险「level0 base-case 可能主导 alpha 统计」——分级 alpha 解读须区分 level0（异质边界）
  与 lvl>=1（内部域），不可同质池化。与 199（t8-level0-inconclusive：level0 alpha INCONCLUSIVE）实证呼应。
- 方法论：凡依赖「下沉次级别」的递归判据族（第29课定律一下沉），递归底必然免门——该免门须显式标注为有效域
  边界，不得纳入全域等变声明。这是「递归底免门」的通用判据（新产出）。

## 谱系引用

- 母规则：`231`（形式化有效域）← 本号=其**级别域**形态（第三个 scope）。
- 姊妹（同 codex-decide-20260702-213839-7db4 裁定会话 / 同 econ_positive.rs 路径）：`680`（消费侧 discard，
  231 消费侧形态）——680 downstream_implications 已预告本号：「codex 矛盾-B 裁定（level0 免门=有限塔有效域
  边界）是 231 消费侧/有效域边界的姊妹实例……坐实 231 在实装层的两个 locus」。本条目=该预告的落盘。
- 231 其他 scope：`625`（计算侧 bug，L2 真实数据暴露 L1 合成掩盖）、`623`（自指工具层）——四 scope 并列。
- 相邻：`673`（StructBreak I_γ=∅ 门拒=None，矛盾-A，同一 codex 裁定会话的姊妹矛盾）、`656`（V(Z)≥V(Y)
  细分类不劣，级别域的表达力上界）、`199`（t8-level0-inconclusive，level0 异质性实证侧）。
- 约束：`090`（声明膨胀禁止——禁止宣称代码不具备的全域能力）、`231`（有效域<定义域）。
- 溯源链：dlpdf-a §5 定性（逐页读，homogeneous）→ ws-gap2s1 分片1 §2/§3 B 类登记 → codex-decide-20260702-213839-7db4
  终局裁定（task #78，heterogeneous）→ task #161 立条。

## 影响声明

纯谱系记录，零 git 代码改动（codex 施工级规格：不改 Rust 行为）。本条目锚定 `econ_positive.rs:811-838/1092-1134`
的语义有效域=内部域 lvl>=1，level0=finite-tower boundary adapter。影响声明口径：全库宣称 theta_v0 实现原文
去根化 C_Θ 处须加 lvl>=1 限定（施工级规格文档义务，待核销）。认识论如实：level0 免门实装为真且经裁定维持；
「全域已实现平移自相似」的声明为膨胀（有效域实为内部域）——本号订正的是声明口径，非代码行为。转 settled 待
编排者 /ritual 最终辨认（同 680）+ 文档全域声称核销。
