---
id: 653
number: 653
status: 生成态   # genealogist 结构记录：gap-B 祖先生命期包含不变量真证（Lean L0，7定理零sorry）+ M08「开区间真包含⊂ vs 闭端点包含」概念分离首次显式化。codex 异质独立确认 Q2 REVISE。最终结算待编排者 /ritual（spec M08 忠实性修正 = 定义层修正）。
date: "2026-06-30"
type: source-tracing   # 溯源分离：把 M08 当作单一「区间包含」断言（开区间真包含 I_e⊂I_{α_e}）的对象，分离为「开区间真包含（不忠实，t=ρ_e=ρ_a 边界处假命题）」与「闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（忠实形式）」。codex Q2 REVISE-DEFINITION 异质独立确认。
depends_on: ["231", "638"]
related: ["608", "638", "637", "222", "223", "230", "mutex-prove-result", "mutex-derive-result"]
negation_source: "homogeneous（蜂群 mutex-prove #24 Lean 真证）+ heterogeneous（codex exec read-only 独立从零判定，Q2 REVISE-DEFINITION 真异质增量）"
negation_model: "codex（OpenAI，read-only sandbox，/tmp/gapb_codex_prompt.txt 仅收形式语义）"
negation_form: "separation"   # 把 M08 的「区间包含」分离为两个形式对象：开区间真包含⊂（不忠实）与闭端点包含（忠实）。两者在 t=ρ_e=ρ_a 边界处真值相反——开区间⊂ 排除端点重合（假），闭端点包含允许端点重合（真）。
title: "★gap-B 祖先生命期包含不变量=成立(TRUE，Lean L0 真证) + M08「开区间真包含⊂ vs 闭端点包含」概念分离（首次显式化，codex Q2 REVISE 异质确认）：∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a) 由走势分解定理二+走势必完美结构推出；正确形式是闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（端点可重合），非 M08 的开区间真包含 I_e⊂I_{α_e}（排除端点重合，在父子共享端点处假命题）"

# separation：本号把一个被混为一谈的对象（M08「区间包含」）分离为两个形式对象——
#   开区间真包含 I_e ⊂ I_{α_e}（M08 原表述）：严格包含，排除端点重合。
#     在 t=ρ_e=ρ_a 边界（最后一子与父共享结束端点）处为假命题——因走势必完美，
#     父在最后一构成子完成时完成 ⟹ ρ_e=ρ_a 必然发生 ⟹ 开区间⊂ 在此处不成立。
#   闭端点包含 λ_a≤λ_e ∧ ρ_e≤ρ_a（忠实形式）：允许端点重合（首子 λ_e=λ_a、末子 ρ_e=ρ_a）。
#     这才是缠论走势分解的真实结构（子首尾相接 tile 父，端点共享）。
#   两者在端点重合处真值相反 ⟹ 不是同一对象。gap-B 在闭端点包含下成立（TRUE），在开区间⊂ 下假。

topo_effect: "separate:M08-interval-containment:{open-strict-⊂[unfaithful,false-at-ρ_e=ρ_a]|closed-endpoint-λ_a≤λ_e∧ρ_e≤ρ_a[faithful,gap-B-TRUE]} | record:gap-B-ancestor-lifespan-containment-invariant-TRUE-Lean-L0-7theorems-zero-sorry | bound:gap-B-validity-domain=L0-structural-not-L2-profit(覆盖≠盈利)"

# 矛盾/分离（type=source-tracing）
contradiction:
  description: |
    #23 推导链复核识别 gap-B：M23 定理3「全元素覆盖」的状态机（PDF §15）只证 e 自身生命期
    （t=λ_e 进 B_t，t∈(λ_e,ρ_e) e∉D_t，t=ρ_e e∈D_t），**未论证祖先在 e 整个生命期内保持活动**。
    活动集递归 A_{t+1}=AncOK[(A_t∖D_t)∪B_t] 中，AncOK 可因某祖先提前出局而把 e 提前关闭（即使
    e 自己 ρ_e 未到）。「保持开启」步 e∈A_t⟹e∈A_{t+1} 隐含假定 AncOK 不剔除 e，但 PDF 从未论证
    祖先存活。缺失不变量：∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a)（祖先生命期包含后代生命期）。

    #24 Lean 真证判定 gap-B **成立（TRUE）**（非反例，不 escalate）：由走势分解定理二（任何级别
    走势≥3 段次级别构成⟹父=子首尾相接拼接，子 tile 父无缝无重叠）+ 走势必完美（父在最后一构成子
    完成时完成）⟹ 直接父子满足闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（首子共享父左端 λ_e=λ_a，末子共享父
    结束 ρ_e=ρ_a，端点可重合）⟹ 子不能延续超父（ρ_e≤ρ_a）⟹ 同时死亡 t=ρ_e=ρ_a 合法（e 因自己
    ρ_e 离开非被祖先提前剔除）。

    **核心分离（codex Q2 REVISE-DEFINITION 异质独立确认）**：M08 把「区间包含」表述为**开区间真包含**
    I_e⊂I_{α_e}（严格 ⊂，排除端点重合）**不忠实**——在 t=ρ_e=ρ_a 边界（末子与父共享结束端点，由
    走势必完美必然发生）处为**假命题**。正确形式是**闭端点包含** λ_a≤λ_e∧ρ_e≤ρ_a（端点可重合）。
    两者在端点重合处真值相反 ⟹ 是两个不同的形式对象，M08 把它们混为一谈。

    这不是定义冲突（无两条互斥定义争夺同一域）——是把 M08 的单一「区间包含」断言分离为两个形式对象
    （开区间⊂ 不忠实 / 闭端点包含 忠实），分离后各自真值域清晰可分层 ⟹ **不触发中断 #1**。
    gap-B 成立判定是 L0 结构推论（端点 Int 代数 + 树深归纳），不膨胀为实盘盈利（覆盖≠盈利，231）。
  layer: 概念   # spec M08 定义层的忠实性修正（开区间⊂→闭端点包含）。不是实装冲突——是 spec 表述与缠论结构（走势必完美⟹端点共享）的不一致。修正 = 用闭端点包含替换开区间⊂（无 workaround）。
  trigger: "#23 §3-B 首次显式化 gap-B（祖先生命期包含=覆盖证明的未论证前提）→ #24 Lean 真证（Origin/AncestorLifespan.lean 7定理零 sorry，#print axioms 仅 propext）+ codex exec read-only 从零独立判定 Q1-Q5 VERDICT TRUE 闭端点形式（Q2 REVISE：闭非开 = 真异质增量）。"

# 概念分离
separation:
  before: "M08「区间包含」——把祖先-后代生命期/价区的包含关系当作单一断言：开区间真包含 I_e⊂I_{α_e}。"
  after:
    - name: "开区间真包含 I_e ⊂ I_{α_e}（M08 原表述，不忠实）"
      definition: "严格包含 ⊂，排除端点重合（λ_a<λ_e ∧ ρ_e<ρ_a 隐含或 I_e 真子集）。"
      source: "spec M08（complete-mutex-classification-pdf-extract.md）原表述。在 t=ρ_e=ρ_a 边界假命题。"
    - name: "闭端点包含 λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a（忠实形式）"
      definition: "闭区间端点不等式，允许端点重合（首子 λ_e=λ_a、末子 ρ_e=ρ_a）。"
      source: "缠论知识库 §8 走势分解定理二（子 tile 父无缝）+ 走势必完美（父在最后子完成时完成⟹端点共享）。Lean Origin/AncestorLifespan.lean LifespanContains 谓词 + DirectParentContains 公理真证。codex Q2 REVISE 异质独立确认。"
  pending_verification: "(a) spec M08 文本是否回填闭端点包含修正（reconcile 性质须重证）——这是对 spec 的忠实性修正，待 /ritual。(b) rust MR2（AncOK 实装）是否维护「活动集更新不在 e 自己 ρ_e 之前剔除 e」——gap-B 的运行时形态，待实装验证。"

# 涉及的定义
definitions_involved:
  - name: "gap-B 祖先生命期包含不变量 ∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a)"
    version: "formal/Origin/AncestorLifespan.lean（#24 本轮新建，7 定理零 sorry/admit/axiom，#print axioms 仅 propext）"
    role: "分离的真证载体。LifespanContains（闭包含谓词）/lifespanContains_trans（传递）/DirectParentContains（直接父子闭包含公理=走势分解二+必完美结构编码）/ancestor_lifespan_contains（主引理，沿祖先树深归纳）/ancestor_alive_in_child_life（顶点，覆盖证明保持步根据）/child_not_outlive_ancestor（ρ_e≤ρ_a，反例不存在坐实 VERDICT TRUE）。"
  - name: "M08 区间包含（spec 原表述：开区间真包含 I_e⊂I_{α_e}）"
    version: ".chanlun/specs/2026-06-28-complete-mutex-classification-pdf-extract.md M08"
    role: "被分离的对象（不忠实表述）。codex Q2 REVISE-DEFINITION：开区间⊂ 应改闭端点包含。在 t=ρ_e=ρ_a 边界假命题。"
  - name: "走势分解定理二 + 走势必完美（缠论知识库 §8，一级权威）"
    version: "缠论知识库.md §8 + docs/chanlun/text/blog/INDEX.md"
    role: "闭端点包含的定义依据。父≥3 段次级别构成⟹子 tile 父（首尾相接无缝）；父在最后一子完成时完成⟹末子 ρ_e=ρ_a、首子 λ_e=λ_a（端点共享）。"
  - name: "231 形式化有效域规则（L0/L1/L2/L3）"
    version: ".claude/rules/formalization-validity-domain.md（settled，谱系 231）"
    role: "约束来源。gap-B 全 L0（端点 Int 代数+树深归纳，同义反复），lake build 绿 ≠ 实盘盈利。M08 开区间⊂ 在边界处假命题 = L0 定义域内的忠实性修正（非 L2 否证）。"
  - name: "638 hostOf 附着判准（右端点命中，settled）"
    version: ".chanlun/genealogy/.../638（settle）"
    role: "同根关联。638 指出买卖点 source_index 恒等于宿主走势 end_index，端点是相邻走势共享交界 ⟹「区间包含 [start,end]∋source_index」在每个 bsp 处落边界 → 恒二义（同 608 点-区间结构）。本号 gap-B 是同一「端点共享导致开区间/闭区间二义」结构在祖先生命期维度的实例。638 用「右端点命中」消歧，本号用「闭端点包含」消歧——同一族解法。"

# 解决方式
resolution:
  type: 概念分离   # M08 开区间⊂ → 闭端点包含的忠实性修正（定义层），由 #24 Lean 真证 + codex Q2 异质确认坐实。最终 spec 文本回填 + /ritual 结算属编排者。
  description: |
    概念分离已完成（L0 真证）：M08「区间包含」分离为开区间真包含⊂（不忠实，边界假命题）与
    闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（忠实，gap-B TRUE）。Lean Origin/AncestorLifespan.lean 7 定理
    零 sorry 真证闭端点包含 ⟹ 祖先在子生命期内必活 ⟹ M23/M26 的「保持开启」步有结构根据
    （不再依赖被藏起来的字段假设 activeAt）。codex 从零独立判定 Q1-Q5 VERDICT TRUE 闭端点形式，
    Q2 REVISE 是真异质增量。gap-B 成立 **非反例，不 escalate**（M23 Eat 定义无需修正）。
    严格下一步（行动/定义类，待编排者 /ritual）：(a) spec M08 文本回填「开区间⊂→闭端点包含」
    忠实性修正（reconcile 性质重证）；(b) rust MR2 维护活动集不提前剔除（gap-B 运行时形态）。
  decided_by: 蜂群内部   # #24 Lean L0 真证（mutex-prove 工位）+ codex 异质独立确认；genealogist 结构记录概念分离；spec 文本回填 + 最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) M23 定理3「全元素覆盖」的状态机已完整证明 ∀e Eat(e)（无未 discharge 前提）。(2) M08 的开区间真包含 I_e⊂I_{α_e} 是祖先-后代包含关系的忠实形式。(3) 祖先生命期包含是可从 M08 直接得到的平凡推论（无需新不变量）。"
  why_negated: "(1) #23 §3-B + codex 逐字独立重现：M23 状态机只证 e 自身生命期，未论证祖先全程存活；A_{t+1}=AncOK[…] 可因祖先提前出局剔除 e。缺「祖先生命期包含」不变量 → 「保持开启」步无根据，Lean 会卡 sorry。(2) 走势必完美 ⟹ 末子 ρ_e=ρ_a（端点共享）必然发生，开区间⊂ 在此处假命题（排除端点重合）——不忠实。codex Q2 REVISE-DEFINITION 独立确认。(3) M08 只给直接父子的开区间⊂，未递归传递到全祖先、未保证端点闭包含——gap-B 须新建闭端点包含不变量 + 沿祖先树深归纳的传递引理（ancestor_lifespan_contains）+ 直接父子闭包含公理（DirectParentContains，走势分解二+必完美的结构编码），非平凡推论。"

# 新产出
new_output:
  definitions:
    - "★概念分离：M08「区间包含」= 开区间真包含⊂（不忠实，t=ρ_e=ρ_a 边界假）⊥ 闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（忠实，gap-B TRUE）。两者端点重合处真值相反。"
    - "gap-B 祖先生命期包含不变量 ∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a) = 成立（TRUE，L0 真证），由走势分解定理二+走势必完美结构推出。"
    - "闭端点包含 ⟹ 祖先在子生命期内必活（ancestor_alive_in_child_life）⟹ M23/M26 覆盖证明「保持开启」步有结构根据（不依赖被藏的字段假设 activeAt）。"
    - "child_not_outlive_ancestor（ρ_e≤ρ_a）：子不延续超父，反例不存在坐实 VERDICT TRUE（父在最后一子完成时完成）。"
  code_changes: "本轮已落：(a) formal/Origin/AncestorLifespan.lean（新建，7定理零sorry）；(b) formal/lakefile.toml Origin lib roots 加 Origin.AncestorLifespan；(c) .chanlun/diagnostics/mutex-prove-result.md + mutex-derive-result.md（报告）。lake build Origin 全绿（114 jobs）。待 /ritual：spec M08 文本回填闭端点包含。待实装：rust MR2 AncOK 不提前剔除。"
  orchestration_changes: |
    方法论：①「区间/价区包含」类形式化在「端点共享」结构（缠论走势 tile：相邻走势共享交界端点）下，
    开区间真包含⊂ 与闭端点包含 ≤ 在端点重合处真值相反——须显式区分（M08 混为一谈是不忠实）。
    这是 638 hostOf（右端点命中消歧）/608（点-区间二义）同族结构。
    ②「覆盖证明」类定理须核「被覆盖元素的祖先/容器在其整个生命期内存活」不变量——PDF 状态机常只证
    元素自身生命期（藏祖先存活为字段假设），Lean 形式化会在此卡 sorry，暴露未论证前提（gap-B）。

# 影响范围
impact:
  affected_modules:
    - "formal/Origin/AncestorLifespan.lean（新建，gap-B 真证）→ MW7（AncestorClosure AncOK）/MW8（MutexRecursive）的 ∀e Eat(e) 提供「保持开启」步结构根据。"
    - "spec M08（complete-mutex-classification-pdf-extract.md）→ 开区间真包含⊂ 应回填为闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（忠实性修正，待 /ritual；reconcile 性质须重证）。"
    - "rust MR2（AncOK 实装）→ 须保证活动集更新不在 e 自己 ρ_e 之前剔除 e（gap-B 运行时形态，待实装验证）。"
  affected_definitions:
    - "M23 定理3 / M26 自相似递归：gap-B 真证后「保持开启」步不再依赖被藏的字段假设 activeAt，∀e Eat(e) 有结构根据。M23 Eat 定义无需修正（gap-B 非反例）。"
    - "M08（spec）：开区间真包含⊂ 不忠实 → 闭端点包含（待 /ritual 回填）。"
    - "638（settle）：本号 gap-B 是 638「端点共享导致开/闭区间二义」结构在祖先生命期维度的同根实例；638 用右端点命中消歧，本号用闭端点包含消歧。印证不否定，维持 settle。"
    - "608（点-区间二义）：本号端点重合二义与 608 同构。关联印证。"
    - "637（中枢核心区间三口径，生成态）：637 是 ZD/ZG 价区口径分离（min/max 选段），本号是生命期/包含关系的开/闭分离——不同轴（价区口径 ⊥ 包含关系开闭）。无矛盾，可分层。"
    - "231（settle）：gap-B 全 L0（端点代数+树归纳），不膨胀盈利；M08 修正是 L0 定义域内忠实性修正非 L2 否证。印证 231，维持 settle。"
  downstream_implications:
    - "M23/M26 全元素覆盖（X^cover_Θ「吃到每个元素」）在 gap-B 真证后结构成立（L0）——但**严禁膨胀**为实盘盈利（PDF §F + L3 8/8 已否证 v1 盈利；覆盖≠盈利，认识论等级不同不可互相否证）。"
    - "rust 实装层（MR2 AncOK）须把 gap-B 不变量作为运行时约束维护（活动集更新不提前剔除生命期内元素）——否则实装与 L0 结构脱节。"
    - "spec M08 回填闭端点包含后，所有引用 M08「区间包含」的下游定理（reconcile 性质）须重核端点重合边界是否被正确处理。"

# 谱系关联
related_records:
  parent: "638（hostOf 右端点命中，settle）——本号 gap-B 是 638「端点共享导致开/闭区间二义」结构在祖先生命期维度的同根实例"
  children: []
  related:
    - "608（点-区间二义）：本号端点重合二义同构"
    - "638（settle，hostOf 右端点命中消歧）：同族解法（端点共享消歧）"
    - "637（生成态，ZD/ZG 三口径）：不同轴（价区口径 ⊥ 包含关系开闭），无矛盾"
    - "231（settle，有效域规则）：gap-B 全 L0 不膨胀盈利；M08 修正是 L0 忠实性修正"
    - "222/223/230（settle，有效域≠定义域三例）：gap-B 在 L0 定义域内成立（闭包含可证），不外推 L2/L3 盈利"
    - "654（本轮，BSP 量身份分离）：mutex-prove-result §7.3 明示 gap-B「状态/事件范畴分离」与 BSP「节点/事件范畴分离」同源——同轮姊妹发现"

# 认识论等级标注（231号强制）
epistemological_levels:
  - proposition: "gap-B ∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a) 成立（TRUE）"
    level: "L0（Origin/AncestorLifespan.lean 7定理零sorry，#print axioms 仅 propext；端点 Int 代数+祖先树深归纳）"
    increment: "零（对 gap-B 本身——从走势分解二+必完美的同义反复）；高（对「M23/M26 覆盖证明保持步有结构根据」的判定）"
  - proposition: "M08 开区间真包含 I_e⊂I_{α_e} 在 t=ρ_e=ρ_a 边界处假命题；正确形式是闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a"
    level: "L0（走势必完美⟹末子 ρ_e=ρ_a 端点共享必然发生；codex Q2 REVISE-DEFINITION 异质独立确认）"
    increment: "高：M08 表述不忠实的判定 + 忠实形式的确定（开/闭分离）"
  - proposition: "覆盖定理 M23/M26 成立只是 L0 全元素覆盖结构性质，不蕴含实盘盈利"
    level: "L0 结构（覆盖）vs L3 经验（v1 8/8 已否证盈利）——两个认识论等级，不可互相否证"
    increment: "高：覆盖≠盈利的等级边界（严禁把 L0 覆盖膨胀为 L2/L3 盈利，231）"
---

# 653 ★gap-B 祖先生命期闭端点包含不变量 + M08「开区间⊂ vs 闭端点包含」概念分离

## 一句话结论

#23 识别 gap-B（M23 覆盖证明的未论证前提：祖先全程存活），#24 Lean **真证成立（TRUE）**——由走势
分解定理二（子 tile 父）+ 走势必完美（父在最后子完成时完成）推出 **闭端点包含** λ_a≤λ_e∧ρ_e≤ρ_a。
同时坐实 **M08 的开区间真包含 I_e⊂I_{α_e} 不忠实**（在 t=ρ_e=ρ_a 端点共享边界处假命题），正确形式是
闭端点包含——这是「开区间⊂ vs 闭端点包含」概念分离的**首次显式化**（codex Q2 REVISE-DEFINITION 异质独立确认）。

## 概念分离表

| 形式对象 | 表述 | t=ρ_e=ρ_a 边界（末子共享父端点） | 忠实性 |
|---------|------|--------------------------------|--------|
| 开区间真包含 I_e⊂I_{α_e}（M08 原） | 严格 ⊂，排除端点重合 | **假命题**（⊂ 不允许 ρ_e=ρ_a） | 不忠实 |
| 闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a | 端点不等式，允许重合 | **真**（首子 λ_e=λ_a / 末子 ρ_e=ρ_a） | 忠实 |

走势必完美 ⟹ 父在最后一构成子完成时完成 ⟹ ρ_e=ρ_a **必然发生** ⟹ 开区间⊂ 在此处必假。两者端点
重合处真值相反 ⟹ 不是同一对象，M08 把它们混为一谈。

## 为何首次记录（核 .chanlun/genealogy/ 确认）

grep `.chanlun/genealogy/` 全目录：「gap-B / AncestorLifespan / 闭端点 / 闭区间包含 / endpoint containment」
仅命中 dag.yaml，pending/settled 无条目 ⟹ **首次显式化**（EVIDENCE mutex-prove-result §4.5/§5 + mutex-derive-result
§5 均明确请求 genealogist 核查首次性，本号确认并立号）。

## 为何 source-tracing 而非矛盾发现 / 不触发中断 #1

无两条互斥定义争夺同一域——是把 M08 单一「区间包含」断言**分离**为两个真值域清晰、可分层的形式对象
（开区间⊂ 不忠实 / 闭端点包含 忠实）。分离后各自有效域清晰（端点重合处真值分别确定），不构成
不可分层矛盾 ⟹ **不触发中断 #1**。gap-B 成立非反例（no-workaround 已自检：闭包含修正是忠实性修正非补丁）。

## 与 638 / 608 同根（端点共享二义族）

638（settle）：买卖点 source_index 恒等于宿主走势 end_index，端点是相邻走势共享交界 ⟹「区间包含
[start,end]∋source_index」恒落边界 → 二义。638 用「右端点命中」消歧。
608：点-区间二义。
本号：祖先-后代生命期端点共享（首子 λ_e=λ_a、末子 ρ_e=ρ_a）⟹ 开区间⊂/闭区间≤ 二义。本号用「闭端点包含」消歧。
**同一族结构**（端点共享导致开/闭二义），同族解法（显式端点处理消歧）。

## 张力检查（019d/020）

### 检查范围
同轮（mutex-derive #23 / mutex-prove #24 + 654 BSP 量身份分离）∪ 1-hop（231/638/608/637）∪ Hub（231 有效域、638 hostOf）。

### 张力1：vs 638（settle）——同根印证，不违
638 端点共享二义（右端点命中消歧）⊃ 本号祖先生命期端点共享（闭端点包含消歧）= 同族结构。本号是 638 结构在祖先生命期维度的实例。无矛盾，638 维持 settle。

### 张力2：vs 637（生成态，ZD/ZG 三口径）——不同轴
637 是中枢核心**价区**口径分离（min/max 选哪些段）；本号是生命期**包含关系**的开/闭分离。两轴正交（价区 ⊥ 包含开闭），可分层。无矛盾，637 维持生成态。

### 张力3：vs 231（settle）——印证
gap-B 全 L0（端点代数+树归纳，同义反复信息增量零），不膨胀盈利；M08 修正是 L0 定义域内忠实性修正（非 L2 否证）。本号是 231 在「L0 结构忠实性修正」维度的实例。印证，231 维持 settle。

### 张力4：vs 654（同轮，BSP 量身份分离）——同源姊妹
mutex-prove-result §7.3 明示：gap-B 的「状态/事件范畴分离」与 BSP 的「节点/事件范畴分离」同源（都是把两个不同范畴的量混为一谈）。本号与 654 是同轮同源姊妹发现，不同对象（生命期包含 vs BSP 计数）。无矛盾。

### 概念分离信号检测（中断 #1）
M08 开区间⊂ vs 闭端点包含分离后真值域清晰可分层 ⟹ **不触发中断 #1**。genealogist 记录分离，不发 SendMessage（已由 Lead 消息触发本次更新）。

### 递归运动结构完成检测（020）
- 第0层：本号写入（gap-B 真证 + M08 开/闭分离 + 同根 638/608）。
- 第1层：本号 × 638 → 端点共享二义族（净新发现高：祖先生命期维度的新实例 + 闭端点消歧）。
- 第2层：本号 × 231 → L0 忠实性修正实例（净新发现中：L0 同义反复已知，开/闭真值相反是新）。
- 第3层：本号 × 637/608 → 关联（净新发现降：不同轴/同构，收尾）。
- scope₁(gap-B真证+开/闭分离) > scope₂(231 L0实例) > scope₃(关联)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** spec M08 回填 + 最终结算待编排者 /ritual；rust MR2 运行时约束=行动类待实装。

## 回溯扫描（职责3）
- **638/608/231/222/223/230（settle）**：本号印证不破坏，维持 settle。
- **637（生成态）**：不同轴，不破坏，维持生成态。
- **无 settled 被本号回溯破坏。** 本号是概念分离（source-tracing）+ gap-B L0 真证的结构记录，spec M08 回填 + 结算待编排者 /ritual。
