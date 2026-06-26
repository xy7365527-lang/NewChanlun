---
id: "608"
number: 608
status: 生成态   # Phase 2 claim9（中枢位置三态完全分类）形式化结晶。依赖 603（范式根·实数三歧推论，生成态）+ 007/008（中心定理一/二，已结算）+ maimai 买卖点位置（已结算 v1.0）+ Phase1 CenterTrichotomy（复用 Center 类型 + 第20课两中枢关系三分）。最终编号 + 结算待 /ritual 在统一编号空间裁定（同 597-607 族）。
date: "2026-06-25"
type: domain
# ★provenance（genealogist 2026-06-25）：Phase 2 claim9（中枢位置三态）真完全分类形式化结晶 + Option A 交叉验证已 wire 进 build。严格产生自：
#   - formal/Phase2/Claim9_CenterPosition.lean（machine-checked，lake build green 16 jobs，无 sorry/admit/axiom，L0；import Formal.CenterTrichotomy，复用 Phase1 Center 类型；ns Formal.CenterPosition，#36）
#   - 第49课（2007-03-22）中枢三态原文（zhongshu.md:84-88："对任何级别的走势中枢，无非有三种情况：当下在该中枢之中/之下/之上"）
#   - maimai.md:133/145/173/307（第三类买卖点边界 ZG/ZD + 1B 中枢下方/3B 中枢上方 + 判定范围 [ZD,ZG] 已结算）
#   - 603 §三（实数三歧 trichotomy 推论，内涵式完全性）
#   非机械转写：六要素 / 认识论等级 / 张力检查 / 回溯扫描齐全。跨 claim 全集张力扫描（唯一合法汇合点）确认。
title: "中枢位置三态完全分类（第49课：价格点 p 相对单个中枢核心区间 [ZD,ZG] 的位置 = below(p<ZD) / within(ZD≤p≤ZG) / above(p>ZG)）= 实数线相对闭区间的三歧 trichotomy 推论（穷尽 ∧ 两两互斥 ∧ 恰好其一，L0）+ 与 maimai 第三类买卖点边界一致（不破 ZG/ZD 算中枢外，临界点归闭核心区间）+ 与第20课两中枢关系三分（claim CenterTrichotomy）是不同的完全分类（点vs区间 ≠ 区间vs区间）"
negation_source: heterogeneous
negation_model: "Option A 交叉验证（两套独立形式化统一进 build）+ codex 真 session 异质审计塑造 Phase1 CenterTrichotomy 三态判据（019eff21 R1#4/R2#4-5：须显式编码中心定理二原公式，非 ¬up∧¬down）。claim9 在此已审定的 Center 类型上加位置三态——复用 core_valid 不变量做 below/above 互斥证明（omega 机器可检验）"
negation_form: refinement
# refinement：claim9 把"价格点相对中枢的位置"从口语三态（之中/之下/之上）精炼为"实数点 p 相对闭核心区间 [ZD,ZG] 的真划分（partition）"——闭区间端点约定（p=ZD/p=ZG 归 within）由第49课"小于 ZD/大于 ZG"严格不等式 + maimai 第三类买卖点"不破才算外"互补钉死。非概念分离——同一第49课位置三态的忠实形式化。

# 拓扑效果标注（147号下游推论3）
# negates：把"位置三态"与"两中枢关系三态（延伸/新生/扩展）"混为同一完全分类（点相对区间 ≠ 区间相对区间）；把临界点 p=ZD/ZG 归类模糊（开闭区间未定）
topo_effect: "refine:position-trichotomy-ambiguous-boundary:point-relative-closed-core-interval-partition"
# refine：把中枢位置三态从「口语三态 + 边界开闭未定 + 与两中枢关系三分混淆」精炼为「实数点相对闭核心区间 [ZD,ZG] 真划分（端点归 within）+ 与第20课两中枢关系三分严格区分」；
#   scope=单个中枢的位置分类层（below/within/above），区别于 CenterTrichotomy 的两中枢关系层

# 涉及的定义
definitions_involved:
  - name: "第49课中枢三态（zhongshu.md:84-88，已结算 v1.0 含第49课）"
    version: ".chanlun/definitions/zhongshu.md（第49课 2007-03-22：'对任何级别的走势中枢，无非有三种情况：当下在该中枢之中(ZG-ZD)/之下(小于ZD)/之上(大于ZG)'）"
    role: "★位置三态完全分类的 L0 核心——RelativePosition 三构造子{below,within,above}穷尽（position_trichotomy）；第49课'无非有三种情况'= 实数线相对闭核心区间 [ZD,ZG] 的三歧 trichotomy。边界：第49课用'小于 ZD/大于 ZG'（严格不等式）刻画之下/之上 ⟹ 之中是闭区间，端点 p=ZD/ZG 归 within"
  - name: "中心定理一/二（007/008，第20课两中枢关系三分）"
    version: ".chanlun/definitions/zhongshu.md（中心定理一延伸 + 中心定理二新生/扩张）+ Formal/CenterTrichotomy.lean"
    role: "★区分对象（非同一完全分类）——claim9 位置三态分类'一个点相对一个区间'，CenterTrichotomy 分类'两个区间的关系'（延伸/上涨延续/下跌延续/扩展）。二者是不同的完全分类，claim9 注释显式区分（Claim9:11-12），无混淆。claim9 复用 CenterTrichotomy.Center 类型（core_valid: zd<zg 不变量），但位置三态是独立分类"
  - name: "买卖点位置（maimai.md:133/145/173/307，已结算 v1.0）"
    version: ".chanlun/definitions/maimai.md（第三类买点回试不破 ZG / 第三类卖点回抽不破 ZD / 1B 中枢下方·3B 中枢上方 / 判定范围 [ZD,ZG] 非 [DD,GG]）"
    role: "★边界约定一致性 + 位置语义引用——claim9 闭核心区间 [ZD,ZG]（端点归 within）与 maimai:133'第三类买点回试低点不跌破 ZG'（破 ZG 才算入/跌入中枢，不破算中枢上方）严格互补；claim9 用 c.zd/c.zg 核心区间，与 maimai:307 已结算'判定范围=[ZD,ZG] 非 [DD,GG]'一致（未误用外缘 DD/GG）。maimai:173 互斥表'1B 中枢下方·3B 中枢上方'对应 below/above 位置语义"
  - name: "603号 §三（完全分类·递归范式，实数三歧推论）"
    version: ".chanlun/genealogy/pending/603-complete-classification-recursive-vs-axis-paradigm（status: 生成态，§三）"
    role: "★范式根——位置三态完全性是 603 内涵式完全性（构造子穷尽，非轴枚举）对'单中枢位置'的应用：RelativePosition 三构造子穷尽（对构造子结构归纳）+ 实数线相对闭区间三歧 trichotomy（position_predicates_total by omega）。非外延式特征空间枚举"
  - name: "Phase1 CenterTrichotomy.Center（复用类型）"
    version: "Formal/CenterTrichotomy.lean（Center: dd/zd/zg/gg + core_valid: zd<zg + outer_lo/outer_hi）"
    role: "★类型复用对象——claim9 import Formal.CenterTrichotomy + open (Center)，直接复用 Phase1 的 Center 类型（含 core_valid: zd<zg 不变量）。below_above_disjoint / position_exactly_one 的互斥证明依赖 c.core_valid（zd<zg ⟹ p<zd 与 zg<p 不可同真）。这是 claim9（ns Formal.CenterPosition）与 Phase1 脊柱的真 import 层连接（非命题层脱钩），无符号碰撞"

# 解决方式
resolution:
  type: domain
  description: "中枢位置三态完全分类形式化为实数点 p 相对单个中枢闭核心区间 [ZD,ZG] 的真划分：RelativePosition 三构造子{below(p<ZD),within(ZD≤p≤ZG),above(p>ZG)}穷尽（position_trichotomy 对构造子结构归纳 + position_predicates_total 实数三歧 by omega，L0）+ 两两互斥（below_within_disjoint / below_above_disjoint 依赖 core_valid: zd<zg / within_above_disjoint）+ 恰好其一（position_exactly_one：穷尽 ∧ 互斥的合取 = 真 partition）+ classify 全函数忠实于三态谓词（classify_eq_below/above/within）。复用 Phase1 CenterTrichotomy.Center 类型（import 层连接）。边界约定（端点归 within 闭区间）由第49课严格不等式刻画 + maimai 第三类买卖点'不破才算外'互补钉死。"
  decided_by: 蜂群内部   # Option A 交叉验证 wire 进 build（lake build green 16 jobs）+ Phase1 CenterTrichotomy 已 codex 审定（019eff21 R1#4/R2#4-5）；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "把中枢位置三态与第20课两中枢关系三分（延伸/新生/扩展）混为同一完全分类；或把边界开闭未定（p=ZD/ZG 归属模糊）；或用外缘波动区间 [DD,GG] 作为位置判定基准。"
  why_negated: "范畴/边界混淆（formalization-validity-domain + maimai 已结算口径）：(1)位置三态分类'一个点相对一个区间'（第49课），两中枢关系三分类'两个区间的关系'（中心定理二）——不同的完全分类对象，混淆导致语义错位（claim9:11-12 显式区分）；(2)边界开闭未定使 p=ZD/ZG 归属模糊——第49课用'小于 ZD/大于 ZG'严格不等式刻画之下/之上 ⟹ 之中必是闭区间 [ZD,ZG]（端点归 within），与 maimai:133 第三类买卖点'不破 ZG/ZD 才算中枢外'严格互补（破才算入）；(3)用 [DD,GG] 外缘做位置基准违反 maimai:307 已结算'第三类买卖点判定范围=[ZD,ZG] 非 [DD,GG]'。三者均在不改任何定义下以正确口径形式化（实现错误，testing-override，正常构造，不上浮）——lake build green 确认。"

# 新产出
new_output:
  definitions:
    - "中枢位置三态完全分类（第49课，递归范式·实数三歧推论）= RelativePosition 三构造子{below(p<ZD),within(ZD≤p≤ZG),above(p>ZG)}穷尽（position_trichotomy + position_predicates_total by omega，L0）"
    - "位置三态真 partition：穷尽（position_predicates_total）∧ 两两互斥（below_within/below_above 依赖 core_valid: zd<zg / within_above）∧ 恰好其一（position_exactly_one）——第49课'无非有三种情况'是实数线相对闭核心区间的真划分"
    - "闭核心区间边界约定：之中 [ZD,ZG] 闭区间（端点 p=ZD/ZG 归 within），之下/之上严格不等式（p<ZD / p>ZG）。与 maimai:133 第三类买卖点'不破 ZG/ZD 算中枢外'严格互补（破才算入，临界点归核心闭区间）"
    - "位置三态 ≠ 两中枢关系三态：claim9（点相对区间）与 CenterTrichotomy（区间相对区间，延伸/新生/扩展）是两个不同的完全分类，类型复用（共享 Center）但分类独立"
    - "classify 全函数忠实判定（classify_eq_below/above/within）：p<ZD→below / ZD≤p≤ZG→within / p>ZG→above，与三态谓词一一对应（omega 机器可检验）"
  code_changes: "不改动代码（L0 理论交付，Lean 已 machine-checked）。formal/Phase2/Claim9_CenterPosition.lean machine-checked：RelativePosition（三构造子）+ IsBelow/IsWithin/IsAbove（三谓词）+ classify + classify_eq_below/above/within（忠实判定）+ position_trichotomy（构造子穷尽）+ position_predicates_total（实数三歧 by omega）+ below_within/below_above/within_above_disjoint（两两互斥）+ position_exactly_one（真 partition）。Option A 交叉验证已 wire 进 build（lib Phase2Claims，srcDir Phase2/，lake build green 16 jobs）。下游：操作触发点的中枢位置判定（claim6 ResolvedOpSignal 解析）= 引擎层职责（不在本号）。"
  orchestration_changes: "无。纯谱系记录（018 行动类）。"

# 影响范围
impact:
  affected_modules:
    - "formal/Phase2/Claim9_CenterPosition.lean（machine-checked，ns Formal.CenterPosition，#36，Option A 交叉验证进 build）；下游操作触发点中枢位置判定（claim6 ResolvedOpSignal）= Phase2+ 引擎层职责，不在本号"
  affected_definitions:
    - "603号 §三：本号是 603 内涵式完全性（实数三歧 trichotomy 推论）对单中枢位置的应用——RelativePosition 三构造子穷尽。一致深化，非新分离"
    - "中心定理一/二（007/008 settled）+ CenterTrichotomy：本号位置三态与两中枢关系三分是不同的完全分类（点vs区间 ≠ 区间vs区间），复用 Center 类型但分类独立。维持 007/008 settled（不破坏），与 CenterTrichotomy 无符号碰撞（import 层连接）"
    - "maimai 买卖点位置（settled v1.0）：本号闭核心区间 [ZD,ZG] 边界约定与 maimai:133/307 一致（不破 ZG/ZD 算中枢外 + 判定范围 [ZD,ZG] 非 [DD,GG]）；below/above 对应 maimai:173 '1B 中枢下方·3B 中枢上方'位置语义。维持 maimai settled（一致引用，不破坏）"
    - "Phase1 CenterTrichotomy.Center：本号复用其类型（import + open），互斥证明依赖 core_valid: zd<zg 不变量。与 Phase1 一致（真 import 层连接，非脱钩），无冲突"
  downstream_implications:
    - "中枢位置判定须用核心区间 [ZD,ZG]（非外缘 [DD,GG]）——下游引擎第三类买卖点/操作触发的位置判定与 maimai:307 已结算口径一致"
    - "位置三态边界临界点（p=ZD/ZG）归 within 闭核心区间——下游'不破才算外'判据与本号闭区间约定严格互补，须保持一致"
    - "位置三态（单中枢）与两中枢关系三态（延伸/新生/扩展）是不同分类——下游不应混用（一个判点的位置，一个判两中枢演化）"
    - "★claim9 位置三态 ↔ claim6 操作触发 / claim7 第一类 BSP 的类型桥接（位置→第三类买卖点判据→操作触发）是 Phase2+ 引擎层职责（脱钩点诚实分层，见 §张力检查）——claim9 不 import BSPLabels/DivergenceNesting，位置三态独立于操作/背驰类型"

# 谱系关联
related_records:
  parent: "603号 §三（完全分类·递归范式，实数三歧推论）——本号是 603 内涵式完全性对单中枢位置的应用"
  children: []
  related:
    - "中心定理一/二（007/008 settled）：本号位置三态与两中枢关系三分是不同完全分类，复用 Center 类型但分类独立（不破坏 007/008）"
    - "maimai 买卖点位置（settled v1.0）：本号闭核心区间边界与 maimai:133/307 一致（不破 ZG/ZD 算中枢外 + 判定范围 [ZD,ZG]）；below/above 对应 1B/3B 位置"
    - "604号（claim5 元素阶梯）：同 Phase2 批次；claim5 自包含 Center 与本号复用的 Phase1 Center 字段同构（语义一致，见 §张力检查·张力1）"
    - "605号（claim6 操作语义）：同 Phase2 批次，位置三态 ↔ 操作触发脱钩点见 §张力检查"
    - "606号（claim7 背驰区间套）：同 Phase2 批次，位置三态 ↔ 第一类 BSP 脱钩点见 §张力检查"
    - "609号（claim10 线段v1）：同 Phase2 批次，二者均用 Int 价格 + 实数三歧/二歧 trichotomy（一致范式）"
    - "598号（真完全分类元判据）：位置三态满足 598'无遗漏'要素=实数三歧 trichotomy 推论（递归范式实现，非 Burnside 轨道）"
    - "231号（formalization-validity-domain）：Lean build green = 逻辑正确（L0），不是实证有效域，不得膨胀（claim9 注释显式标注）"
    - "090号（声明膨胀）：边界开闭未定 / 位置与两中枢关系混淆 = 模糊地带；本号闭区间约定 + 显式区分 = 严格"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "位置三态完全分类（below/within/above 三构造子穷尽）"
    level: "L0（第49课已结算定义 + RelativePosition 结构归纳，Lean machine-checked，Option A 交叉验证进 build）"
    increment: "高：位置完备性继承第49课'无非有三种情况'，非断言"
  - proposition: "位置三态真 partition（穷尽 ∧ 两两互斥 ∧ 恰好其一）"
    level: "L0（实数线相对闭区间三歧 trichotomy，position_predicates_total/disjoint/exactly_one by omega，依赖 core_valid: zd<zg）"
    increment: "高：第49课位置三态是实数线相对核心区间的真划分（omega 机器可检验）"
  - proposition: "闭核心区间边界约定（端点 p=ZD/ZG 归 within）"
    level: "L0（第49课严格不等式'小于 ZD/大于 ZG' + maimai:133 第三类买卖点'不破才算外'互补）"
    increment: "高：关死边界开闭模糊地带，与 maimai 已结算口径一致"
  - proposition: "位置三态 ≠ 两中枢关系三态（点相对区间 ≠ 区间相对区间）"
    level: "L0（分类对象区分，claim9:11-12 显式标注）"
    increment: "高：关死'位置三态=延伸/新生/扩展'范畴混淆"
  - proposition: "claim9 位置三态与 claim6/claim7 操作/背驰类型桥接"
    level: "未做（Phase2+ 引擎层职责）"
    increment: "否定性：claim9 不 import BSPLabels/DivergenceNesting，位置独立于操作/背驰，是有效域诚实分层"
---

# 608号（生成态）：中枢位置三态完全分类 = 点相对单个中枢核心区间的真划分

## 一句话结论

**中枢位置三态（第49课）在递归数据类型范式下形式化为实数点 p 相对单个中枢闭核心区间 [ZD,ZG] 的真划分——RelativePosition 三构造子{below(p<ZD),within(ZD≤p≤ZG),above(p>ZG)}穷尽（对构造子结构归纳 + 实数三歧 trichotomy by omega，L0），且穷尽 ∧ 两两互斥 ∧ 恰好其一（position_exactly_one 真 partition），classify 全函数忠实于三态谓词。** 这是 603 §三内涵式完全性对"单中枢位置"的应用（实数三歧 trichotomy 推论，非轴枚举）。关键区分：位置三态分类"一个点相对一个区间"，**不同于** 第20课两中枢关系三分（延伸/新生/扩展，CenterTrichotomy 分类"两个区间的关系"）。边界约定（端点 p=ZD/ZG 归 within 闭区间）由第49课严格不等式 + maimai 第三类买卖点"不破 ZG/ZD 才算中枢外"严格互补钉死。Option A 交叉验证已 wire 进 build（lake build green 16 jobs，无 sorry/admit/axiom，#36）。

## 位置三态形式化（machine-checked，formal/Phase2/Claim9_CenterPosition.lean）

| 部件 | 内容 | 关键定理 | 有效域 |
|---|------|---------|--------|
| 位置三态 | RelativePosition{below,within,above}穷尽 | `position_trichotomy` | 完全分类（L0） |
| 三谓词穷尽 | p<ZD ∨ ZD≤p≤ZG ∨ p>ZG | `position_predicates_total`（omega） | 实数三歧 trichotomy（L0） |
| 两两互斥 | below/within、below/above（依赖 zd<zg）、within/above | `*_disjoint` | 真 partition（L0） |
| 恰好其一 | 穷尽 ∧ 互斥合取 | `position_exactly_one` | 真划分（L0） |
| 忠实判定 | p<ZD→below / ZD≤p≤ZG→within / p>ZG→above | `classify_eq_below/above/within` | 全函数忠实（L0） |

## 类型复用 + 边界约定（Phase1 Center + maimai 一致）

claim9 `import Formal.CenterTrichotomy` + `open (Center)`——**直接复用 Phase1 脊柱的 Center 类型**（dd/zd/zg/gg + core_valid: zd<zg）。`below_above_disjoint` / `position_exactly_one` 的互斥证明依赖 `c.core_valid`（zd<zg ⟹ p<zd 与 zg<p 不可同真）。这是 claim9（ns `Formal.CenterPosition`）与 Phase1 的**真 import 层连接**（非命题层脱钩，无符号碰撞）。

边界约定（claim9:14-27）：第49课用"小于 ZD/大于 ZG"严格不等式刻画之下/之上 ⟹ 之中是闭区间 [ZD,ZG]，端点 p=ZD/ZG 归 within。与 maimai:133 第三类买点"回试不跌破 ZG → 中枢上方"（破 ZG 才算入）**严格互补**——临界点归核心闭区间，"不破才算外"。claim9 用 `c.zd/c.zg` 核心区间，与 maimai:307 已结算"第三类买卖点判定范围=[ZD,ZG] 非 [DD,GG]"**一致**（未误用外缘）。

## ★张力检查（019d/020号）— 跨 claim 全集扫描（唯一合法汇合点）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群：604（claim5）/605（claim6）/606（claim7）/607（claim8）/609（claim10）+ Phase1 五脊柱模块
- 1-hop 邻接：603 §三（范式根）/007/008（中心定理 settled）/maimai（settled）/Phase1 CenterTrichotomy/598/231/090
- Hub 节点：603（范式根）/Phase1 CenterTrichotomy（Center 类型复用）

### 张力1：vs claim5 自包含 Center（604）— 语义一致，命名空间隔离
claim5（ns `Chanlun.Phase2.ConstitutiveLadder`）第6级自包含 Center（字段 dd/zd/zg/gg + core_valid: zd<zg + outer_lo/outer_hi，Claim5:258-265），与 claim9 复用的 Phase1 `Formal.CenterTrichotomy.Center`（字段完全相同，CenterTrichotomy:32-39）**字段同构 + 不变量同构（都是 [ZD,ZG] 核心区间，zd<zg 成立条件，DD/GG 外缘）**。**判定**：二者命名空间隔离（无 import 互联），但**语义一致**——都是中枢的 [ZD,ZG] 核心区间 + [DD,GG] 外缘，core_valid 不变量相同。无定理同时断言两 Center 相等又不等，无逻辑矛盾。这是构成性阶梯（claim5 自包含）vs 中枢位置层（claim9 复用 Phase1）的诚实分层。

### 张力2：vs CenterTrichotomy 两中枢关系三分 — 不同完全分类，非冲突
CenterTrichotomy 分类"前后两中枢的关系"（延伸/上涨延续/下跌延续/扩展，区间vs区间）；claim9 分类"一个价格点相对一个中枢核心区间的位置"（below/within/above，点vs区间）。**判定**：两个**不同的完全分类**（分类对象不同），claim9:11-12 显式区分。复用同一 Center 类型不导致分类混淆——一个判两区间演化关系，一个判点位置。**无逻辑矛盾，一致（共享类型，分类独立）。**

### 张力3：vs maimai 买卖点位置（settled）— 边界约定一致，不破坏 settled
maimai:133 第三类买点"回试不跌破 ZG"+ maimai:307 已结算"判定范围=[ZD,ZG] 非 [DD,GG]"+ maimai:173 "1B 中枢下方·3B 中枢上方"。claim9 闭核心区间 [ZD,ZG]（端点归 within，破才算入）与 maimai"不破 ZG/ZD 算中枢外"**严格互补**；claim9 用核心区间 c.zd/c.zg（非外缘），与 maimai:307 **一致**；below/above 对应 1B/3B 位置语义。**与 maimai settled 一致（边界 + 范围 + 位置语义引用），不破坏。**

### ★脱钩点（vs 605 claim6 / 606 claim7）— 位置三态 ↔ 操作/背驰类型未桥接，诚实分层非矛盾
- claim9 位置三态经本地 `RelativePosition`（below/within/above）表达；claim6（605）操作触发经 `OperationTrigger`（BSPLabels.BSPLabelSet）/ ResolvedOpSignal 表达；claim7（606）第一类 BSP 经 `Type1BSPWithNesting` 表达。
- **判定**：claim9 不 `import Formal.BSPLabels` / `Formal.DivergenceNesting`，位置三态独立于操作/背驰类型，类型层未连接。这不是逻辑矛盾——位置三态在其有效域内自洽（单中枢的点位置分类，不需要 BSP/操作语义），claim9 注释诚实标注 L0 不得膨胀。
- **下游推论**：位置三态（below/within/above）→ 第三类买卖点判据（回试不破 ZG → 中枢上方 = above）→ 操作触发点的统一类型流是 **Phase2+ 引擎层职责**。诚实分层（formalization-validity-domain），非膨胀。
- **∴ 无不可分层矛盾，无中断#1。** 记入下游推论供 Lead/编排者纳入轴线汇报。

### 命名空间张力（已知，必须记录 — integrator 诚实标注）
- claim9 用 `Formal.CenterPosition`（与 Phase1/solo 一致），**不属命名空间张力**（claim9 是 Formal.* 系）。命名空间不一致的张力仅限 Claim5（`Chanlun.Phase2.ConstitutiveLadder`）/ Claim10（`Chanlun.Phase2.SegmentV1`）vs 其余 `Formal.*`——见 §跨 claim 命名空间张力（汇报项）。
- claim9 是 `Formal.*` 系且真 import Phase1 CenterTrichotomy（最强连接），无命名空间张力。

### 递归运动结构完成检测（020号）
- 第0层：本号写入（中枢位置三态完全分类）
- 第1层：本号 × 603 §三碰撞 → 实数三歧 trichotomy 推论应用（净新发现：位置三态是点相对闭区间真 partition）
- 第2层：本号 × 007/008/maimai 碰撞 → 边界约定 + 位置语义引用（净新发现：闭区间端点约定 + 与两中枢关系三分区分，但这是已结算定义的引用，净新发现量骤降=**背驰**）
- 涉及范围：scope₁(603) > scope₂(007/008/maimai/598 引用)=**顶分型**
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 无结构内不可分层矛盾，无新 /escalate。

## 回溯扫描（职责3）

**本号写入是否回溯结算/破坏既有记录：**
- **603 §三**：本号是 603 实数三歧 trichotomy 推论的应用（深化非结算）。603 仍生成态。
- **007/008（中心定理 settled）**：本号位置三态与两中枢关系三分是不同分类，维持 settled，**不破坏**。
- **maimai（settled v1.0）**：本号边界 + 范围 + 位置语义与 maimai 一致引用，维持 settled，**不破坏**。
- **Phase1 CenterTrichotomy**：本号复用其 Center 类型（真 import），与 Phase1 一致，无冲突。
- **无 settled 被本号回溯破坏。** 604/605/606/607/609（同 Phase2 批次）+ 603 整族仍生成态，待编排者 /ritual 统一结算。

## 结果包六要素

1. **结论**：中枢位置三态（第49课）形式化为实数点相对单中枢闭核心区间 [ZD,ZG] 的真划分——RelativePosition 三构造子{below,within,above}穷尽 + 真 partition（穷尽 ∧ 两两互斥 ∧ 恰好其一，L0）+ classify 全函数忠实。复用 Phase1 Center 类型（真 import）。Lean machine-checked（lake build green 16 jobs，无 sorry/admit/axiom，Option A 交叉验证进 build）。
2. **定义依据**：第49课中枢三态（zhongshu.md:84-88 已结算）+ 中心定理一/二（007/008）+ maimai 买卖点位置（maimai:133/307/173 settled）+ 603 §三实数三歧 trichotomy 推论 + Phase1 CenterTrichotomy.Center 类型。
3. **边界条件（结论翻转）**：①若存在点 p 既不 below 也不 within 也不 above（或同时满足两个）⟹ 位置三态非真 partition（实数三歧 trichotomy + core_valid: zd<zg 否定此）；②若端点 p=ZD/ZG 应归 below/above（开区间）而非 within ⟹ 边界约定错（第49课严格不等式 + maimai 不破才算外否定此）；③若位置判定应用外缘 [DD,GG] 而非核心 [ZD,ZG] ⟹ 与 maimai:307 已结算冲突（本号用核心区间，一致）；④若位置三态与两中枢关系三分是同一分类 ⟹ 范畴混淆（claim9:11-12 区分否定此）。
4. **下游推论**：中枢位置判定须用核心区间 [ZD,ZG]（非外缘）；临界点 p=ZD/ZG 归 within（与"不破才算外"互补）；位置三态 ≠ 两中枢关系三态（不混用）；claim9↔claim6/claim7 类型桥接（位置→第三类买卖点→操作触发）是引擎层职责（脱钩点诚实分层）。
5. **谱系引用**：本号是 603 §三实数三歧 trichotomy 推论对单中枢位置的应用（parent:603 §三）；与 007/008（中心定理 settled）位置三态 vs 两中枢关系三分区分；与 maimai（settled）边界 + 范围 + 位置语义一致引用。**这是 domain 层形式化结晶（非概念分离）——同一第49课位置三态的忠实形式化。** related:604/605/606/607/609/598/231/090。
6. **影响声明**：不改动代码或定义（L0 理论交付，Lean machine-checked）；新增本谱系记录（pending 生成态）；应用 603 §三 + 复用 Phase1 Center 类型（真 import）；引用 007/008/maimai settled（不破坏）；记录 claim9↔claim6/claim7 类型脱钩点（下游推论）；最终结算待编排者 /ritual。

## ★立号与结算建议（给 Lead/编排者）
- **立号**：生成态草稿号 608（Phase2 claim9，main pending 续号——pending 最大 607，608 未占用，无编号碰撞）。
- **结算路径**：**生成态**（本工位不自行 settle）。这是 domain 层形式化结晶（中枢位置三态，Option A 交叉验证进 build）——建议编排者走 **/ritual** 与 603-607/609 整族统一结算。**关键裁定项**：①位置三态完全分类的递归范式确认（实数三歧 trichotomy 推论，构造子穷尽）；②位置三态 ≠ 两中枢关系三态的范畴区分确认；③claim9↔claim6/claim7 类型桥接（位置→第三类买卖点→操作触发）排期。
