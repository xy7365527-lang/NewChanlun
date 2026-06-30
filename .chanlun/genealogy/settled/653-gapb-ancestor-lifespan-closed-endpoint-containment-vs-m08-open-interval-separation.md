---
id: 653
number: 653
status: 已结算   # /ritual 结算（2026-06-30，编排者授权"走 /ritual 结算谱系"）。gap-B 闭端点包含不变量 Lean L0 真证（7定理零sorry）+ codex Q2 REVISE 异质独立确认 + 本轮 #24 Lean/codex/ChatGPT 四方交汇 → 结算依据充分。M08 忠实性修正标注已回填 spec（见 settlement 区块"M08 回填状态"）。
date: "2026-06-30"
settlement_date: "2026-06-30"
type: source-tracing
depends_on: ["231", "638"]
related: ["608", "638", "637", "222", "223", "230", "654", "mutex-prove-result", "mutex-derive-result"]
negation_source: "homogeneous（蜂群 mutex-prove #24 Lean 真证）+ heterogeneous（codex exec read-only 独立从零判定，Q2 REVISE-DEFINITION 真异质增量）"
negation_model: "codex（OpenAI，read-only sandbox，/tmp/gapb_codex_prompt.txt 仅收形式语义）"
negation_form: "separation"
title: "★gap-B 祖先生命期包含不变量=成立(TRUE，Lean L0 真证) + M08「开区间真包含⊂ vs 闭端点包含」概念分离（首次显式化，codex Q2 REVISE 异质确认）：∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a) 由走势分解定理二+走势必完美结构推出；正确形式是闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a（端点可重合），非 M08 的开区间真包含 I_e⊂I_{α_e}（排除端点重合，在父子共享端点处假命题）"

topo_effect: "separate:M08-interval-containment:{open-strict-⊂[unfaithful,false-at-ρ_e=ρ_a]|closed-endpoint-λ_a≤λ_e∧ρ_e≤ρ_a[faithful,gap-B-TRUE]} | record:gap-B-ancestor-lifespan-containment-invariant-TRUE-Lean-L0-7theorems-zero-sorry | bound:gap-B-validity-domain=L0-structural-not-L2-profit(覆盖≠盈利)"

# ============================================================
# /ritual 结算区块（2026-06-30，genealogist 落实，编排者授权 /ritual）
# ============================================================
settlement:
  # 从哪个矛盾中来
  from_contradiction: |
    #23 推导链复核识别 gap-B：M23 定理3「全元素覆盖」状态机（PDF §15）只证 e 自身生命期，
    未论证祖先在 e 整个生命期内保持活动。活动集递归 A_{t+1}=AncOK[(A_t∖D_t)∪B_t] 中 AncOK 可因某祖先
    提前出局而把 e 提前关闭。缺失不变量：∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a)。
  # 否定了什么旧定义
  negated_definition: |
    M08 的开区间真包含 I_e⊂I_{α_e}（严格 ⊂，排除端点重合）作为祖先-后代包含关系的忠实形式被否定。
    在 t=ρ_e=ρ_a 边界（末子与父共享结束端点，由走势必完美必然发生）处为假命题——⊂ 不允许端点重合。
  # 新定义
  new_definition: |
    闭端点包含 λ_a≤λ_e ∧ ρ_e≤ρ_a（闭区间端点不等式，允许首子 λ_e=λ_a、末子 ρ_e=ρ_a 端点重合）
    是缠论走势分解的忠实形式。gap-B 不变量在闭端点包含下成立（TRUE）。
  # 分离的逻辑必然性
  logical_necessity: |
    由走势分解定理二（任何级别走势≥3 段次级别构成⟹父=子首尾相接拼接，子 tile 父无缝无重叠）+
    走势必完美（父在最后一构成子完成时完成）⟹ 末子共享父结束端点 ρ_e=ρ_a **必然发生**。
    开区间⊂ 排除端点重合 ⟹ 在此必然发生的边界处假命题（不忠实）；闭端点包含允许端点重合 ⟹ 忠实。
    两者在端点重合处真值相反 ⟹ 不是同一对象，M08 把它们混为一谈。这是逻辑必然（走势必完美是
    一级权威定义，端点共享是其直接结构后果），非经验总结。
  # 结算依据充分性（no-workaround 自检）
  sufficiency: |
    (1) Lean L0 真证：formal/Origin/AncestorLifespan.lean 7 定理零 sorry/admit/axiom，#print axioms 仅
        propext，lake build Origin 全绿（114 jobs）。闭端点包含 ⟹ 祖先在子生命期内必活 ⟹ M23/M26
        「保持开启」步有结构根据。
    (2) codex 异质独立确认：read-only sandbox 从零判定 Q1-Q5 VERDICT TRUE 闭端点形式，Q2
        REVISE-DEFINITION（开区间⊂应改闭端点包含）是真异质增量（非同质复述）。
    (3) 本轮四方交汇（Lead 消息确认）：#24 Lean 真证 + codex + ChatGPT 四方交汇确认 gap-B 成立（情况A
        构成关系）。
    (4) 首次概念分离（grep .chanlun/genealogy/ 全目录确认 pending/settled 无 gap-B/AncestorLifespan/
        闭端点包含条目，仅 dag.yaml）+ 异质确认 + Lean 零 sorry ⟹ 结算依据充分。
  # 张力检查（结算不破坏已有 settled）
  tension_check: |
    vs 638（settle）：同根印证——638「端点共享导致开/闭区间二义」⊃ 本号祖先生命期端点共享，638 用
      右端点命中消歧，本号用闭端点包含消歧，同族解法。638 维持 settle，不破坏。
    vs 608（点-区间二义）：同构印证，不破坏。
    vs 231（settle）：gap-B 全 L0（端点代数+树归纳），M08 修正是 L0 定义域内忠实性修正非 L2 否证。
      印证 231，不破坏。
    vs 637（生成态，ZD/ZG 三口径）：不同轴（价区口径 ⊥ 包含关系开闭），可分层，不破坏。
    vs 654（同轮姊妹）：同源（节点/事件 vs 状态/事件范畴分离，mutex-prove-result §7.3），不破坏。
    无 settled 被本号结算破坏。
  # M08 回填状态（spec 文本忠实性修正）
  m08_backfill_status: |
    M08 文本回填（在 spec complete-mutex-classification-pdf-extract.md M08 条目后插入「M08-忠实性修正
    （653号结算）」标注行：保留 PDF 原文 I_e⊂I_{α_e} 作提取忠实性记录 + 标注闭端点包含为下游实装准绳）
    属 /ritual 步骤1「写入定义」=定义文件修正动作。genealogist 工具集无 Edit 且 teammate 不可 spawn
    其他 teammate（flat roster）→ M08 文本内联回填**转交 Lead 用 Edit 工位执行**（待回填）。
    回填内容已在本结算记录 negated_definition/new_definition 完整定义，spec 内联标注是其文本投影。
    reconcile 性质（M08 [精化] C18）须在回填后重核端点重合边界处理（下游定理引用 M08「区间包含」处）。
  decided_by: 编排者 /ritual（授权 genealogist 落实）
---

# 653 ★gap-B 祖先生命期闭端点包含不变量 + M08「开区间⊂ vs 闭端点包含」概念分离【已结算 2026-06-30】

## 结算一句话

#23 识别 gap-B（M23 覆盖证明的未论证前提：祖先全程存活），#24 Lean **真证成立（TRUE）**——由走势
分解定理二（子 tile 父）+ 走势必完美（父在最后子完成时完成）推出 **闭端点包含** λ_a≤λ_e∧ρ_e≤ρ_a。
同时坐实 **M08 的开区间真包含 I_e⊂I_{α_e} 不忠实**（在 t=ρ_e=ρ_a 端点共享边界处假命题）。codex Q2
REVISE-DEFINITION 异质独立确认 + Lean 零 sorry + 四方交汇 ⟹ /ritual 结算（编排者授权）。

## 概念分离表

| 形式对象 | 表述 | t=ρ_e=ρ_a 边界（末子共享父端点） | 忠实性 |
|---------|------|--------------------------------|--------|
| 开区间真包含 I_e⊂I_{α_e}（M08 原） | 严格 ⊂，排除端点重合 | **假命题**（⊂ 不允许 ρ_e=ρ_a） | 不忠实 |
| 闭端点包含 λ_a≤λ_e∧ρ_e≤ρ_a | 端点不等式，允许重合 | **真**（首子 λ_e=λ_a / 末子 ρ_e=ρ_a） | 忠实 |

走势必完美 ⟹ 父在最后一构成子完成时完成 ⟹ ρ_e=ρ_a **必然发生** ⟹ 开区间⊂ 在此处必假。两者端点
重合处真值相反 ⟹ 不是同一对象。

## 与 638 / 608 同根（端点共享二义族）

638（settle）：买卖点 source_index 恒等于宿主走势 end_index，端点是相邻走势共享交界 ⟹「区间包含」
恒落边界二义，用「右端点命中」消歧。608：点-区间二义。本号：祖先-后代生命期端点共享（首子
λ_e=λ_a、末子 ρ_e=ρ_a）⟹ 开区间⊂/闭区间≤ 二义，用「闭端点包含」消歧。**同一族结构，同族解法。**

## 结算后下游（行动/实装类，不阻塞本结算）

1. **M08 spec 文本回填**（转交 Lead Edit 工位）：保留 PDF 原文 ⊂ + 插入闭端点包含忠实性修正标注。
2. **rust MR2（AncOK 实装）**：须保证活动集更新不在 e 自己 ρ_e 之前剔除 e（gap-B 运行时形态，待实装验证）。
3. **reconcile 重核**：所有引用 M08「区间包含」的下游定理须重核端点重合边界是否正确处理。

## 严禁膨胀（231号边界）

gap-B 成立是 **L0 结构推论**（端点 Int 代数 + 树深归纳，同义反复）。M23/M26 全元素覆盖在 gap-B 真证
后结构成立（L0），但**严禁膨胀**为实盘盈利——PDF §F + L3 8/8 已否证 v1 盈利；覆盖≠盈利，认识论等级
不同不可互相否证。

---

# （以下为结算前生成态原文，谱系012：发现过程不可压扁，保留作发生史）

## 一句话结论（生成态原文）

#23 识别 gap-B，#24 Lean 真证成立（TRUE）——由走势分解定理二 + 走势必完美推出闭端点包含
λ_a≤λ_e∧ρ_e≤ρ_a。同时坐实 M08 开区间真包含不忠实（端点共享边界假命题）。这是「开区间⊂ vs
闭端点包含」概念分离的首次显式化（codex Q2 REVISE-DEFINITION 异质独立确认）。

## 认识论等级（231号）

- gap-B ∀a∈Anc(e) [λ_e,ρ_e)⊆[λ_a,ρ_a) 成立(TRUE)：L0（7定理零sorry，#print axioms 仅 propext）。
- M08 开区间⊂ 边界假命题 + 闭端点包含忠实：L0（走势必完美⟹末子 ρ_e=ρ_a 必然 + codex Q2 异质确认）。
- 覆盖定理 L0 结构 ≠ L3 经验盈利（v1 8/8 已否证），两等级不可互相否证。

## 谱系关联

- parent: 638（hostOf 右端点命中，settle）——本号是 638「端点共享二义」在祖先生命期维度的实例
- related: 608（点-区间二义同构）、637（生成态，不同轴）、231/222/223/230（有效域规则）、654（同轮姊妹）
