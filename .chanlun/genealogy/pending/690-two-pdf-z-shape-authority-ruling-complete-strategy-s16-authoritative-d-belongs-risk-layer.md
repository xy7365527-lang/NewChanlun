---
id: "690"
number: 690
status: 生成态   # 权威裁定落谱系；转 settled 待编排者 /ritual 最终辨认（同 689/688 口径）。本条目锚定 A1 两残余（Jchain/d）both 消解的出处链；实装（risk 层 d、selector 恒等式护栏）已在，本条目订正的是「z 形态权威归属」与「d 层归属」，非代码行为。
date: "2026-07-03"
type: source-tracing   # 出处订正：A1「Jchain+d 缺维残余」（gap-master2-final 继承 shard4）从「权威 z 缺两维」订正为「both 消解——Jchain 代数派生已覆盖、d 属风险层已实装」。两 PDF z 形态差异裁定：完整的策略.pdf §16 为唯一权威。
source: "[新缠论] docs/formal-chain/INDEX.md 编排者裁定 2026-07-02（『本目录=形式化唯一权威』）+ 完整的策略.pdf §3（p2-3 N^δ 逐级相邻）/§6（p4 canonical z 含 Jchain 不含 d）/§16（p10 最完整形态 20 维排除 d）；缠论的全互斥定义策略.pdf §九（p22 示意 Z_t 带「…」）/§十（p22-23 d=非拓扑度量归风险层）；实装 mu_estimator.rs:136-138（Jchain 代数派生论证）/selector.rs:250-251（恒等式护栏机器可检）/risk.rs:74-105+194-198（structural_stop+|entry-stop|）/runner.rs:517-553（gate 消费）。核验=ws-a1dims 逐页 Read 原始 PDF（.chanlun/review-results/a1-zdims-20260704.md，Task #162）"
negation_source: heterogeneous   # 出处裁定=INDEX.md 编排者裁定（完整的策略.pdf 为唯一权威）；原文核验=ws-a1dims 逐页 Read 两 PDF + 独立读码（homogeneous）。
negation_model: null   # 无外部模型裁量——本条目的裁定依据是编排者既有 INDEX.md 权威链裁定的直接应用（Lead 裁定(a)：定理类）。
negation_form: separation   # 两 PDF 的 Z_t 形态是两个不同的能指（权威 §16 20 维含 Jchain 不含 d vs 示意 §九带「…」列 d），被 shard4 当作同一个 z 混淆；分离后：权威 z（μ 分类状态，§16）vs 示意 z（含 d 的非拓扑度量，§十亲口归风险层）。非定义冲突——§十自己把 d/β/c/m 归「非拓扑度量」，层归属由原文明定。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：(1) gap-master2-final A1 行的「Jchain+d 缺维残余」判定；(2) shard4 把示意性 §九 z（含 d）当作权威 z 的错源。
topo_effect: |
  (1) sever — A1「d 是缺失的 μ 分类 z 维」误归属边切断。d 的层归属从「μ 分类状态 z」re-anchor 到
      「风险投影层 K_Θ/sizing」——依据缠论的全互斥定义策略.pdf §十亲口把 d 与 β/c/m 并列归「非拓扑度量」。
      d 已在风险层生产（risk.rs:74-105 structural_stop + :194-198 risk_dist_usd=|entry-stop|·tick）+ 消费
      （runner.rs:517-553 gate）——层归属正确，非缺口。target=A1 行 d 归属；scope=local。
  (2) split — Q「z 形态」在两 PDF 间分裂：完整的策略.pdf §16（唯一权威）保留 20 维 canonical z 分支（含
      Jchain、排除 d）；缠论的全互斥定义策略.pdf §九示意 Z_t 分支携带违反记录（带「…」非穷举 + 混入 d 等
      非拓扑度量）——不装配进 canonical z。target=z 形态权威归属；scope=local（仅 shard4/gap-master2-final
      A1 行的出处订正，零下游代码改动）。

# 矛盾（source-tracing 的被订正对象——出处/层归属张力）
contradiction:
  description: |
    gap-master2-final A1 行判定「canonical z 缺两维残余：Jchain + d（结构止损距离），须加维实装」。
    经 ws-a1dims 逐页 Read 两份 PDF 与代码交叉核对（Task #162），此判定 both 消解：
    (a) **Jchain**：权威 z（完整的策略.pdf §6/§16）确含 Jchain，但 mu_estimator.rs:136-138 已论证
        Jchain ≅ (origin_level, level) 代数派生、无独立自由度，且 §3 N^δ 定义强制逐级相邻（区间套
        J^δ_e⊆…⊆J^δ_ℓ，不可跳级）证实该论证——链签名由 (ℓ,e) 完整携带。恒等式护栏
        origin_level = level + nest_depth 已在 selector.rs:250-251 机器可检（debug_assert + 单测 :547-566）。
        Jchain 结构上不可表达跳级 ⟹ 已覆盖，非缺口，无加维空间。
    (b) **d（失效/止损距离）**：权威最完整形态 z（完整的策略.pdf §16，20 维）显式列 β(ForceState)/
        c(CostBucket)/m(MarginState)，**独缺 d**——同族四度量收三排一。d 只出现在**另一份** PDF
        （缠论的全互斥定义策略.pdf §九）的**示意性**（带「…」，非穷举）Z_t 中，且该 PDF §十亲口把
        d 与 β/c/m 并列归「非拓扑度量」、属风险投影层。d 已在风险层生产+消费（risk.rs structural_stop +
        |entry-stop|，runner.rs gate）。不是 μ 分类 z 维缺口，已在正确的层实装。
  layer: 概念   # 两 PDF z 形态的出处归属 + d 的层归属。非代码 bug：Jchain 护栏已在、d 已在风险层实装，零 z 装配代码改动。
  trigger: "shard4（gap-master2-shard4-20260703.md）读缠论的全互斥定义策略.pdf 示意 z（含 d）未与权威 §16 交叉核对 ⟹ gap-master2-final A1 行继承错源 ⟹ #162 A1-zdims-residual 逐页 Read 两 PDF 核验消解 ⟹ team-lead 派 #177 立条。"

# 涉及的定义
definitions_involved:
  - name: "完整的策略.pdf §16 canonical z（20 维最完整形态）"
    version: "docs/formal-chain/完整的策略.pdf §16（p10），INDEX.md 裁定唯一权威（形式化最终形态）"
    role: "z 形态的**唯一权威**。20 维=（ℓ,e,δ,I_γ,Ndepth,CandType,ForceState,Jchain,σ_higher,σ_p,role,posState,shortDiff,H,TStage,ηBucket,RiskMode,CostBucket,MarginState,ExitType）——**含 Jchain、排除 d**。同族 β(ForceState)/c(CostBucket)/m(MarginState) 收入 z，独排 d，是权威作者刻意选择（非渲染遗漏）。"
  - name: "缠论的全互斥定义策略.pdf §九/§十 示意 Z_t"
    version: "docs/formal-chain/缠论的全互斥定义策略.pdf §九（p22）/§十（p22-23），二级（非唯一权威；与权威 §16 z 形态不同）"
    role: "被 shard4 误当作权威 z 的示意性 Z_t。§九 Z_t=(ℓ,δ,I_γ,r,σ_p,ω,κ,β,d,c,m,…) **带「…」非穷举**；§十把纯拓扑态 {ℓ,δ,I_γ,r,σ_p,κ} 与非拓扑度量 {β_t=背驰强度, d_t=失效/止损距离, c_t=成本, m_t=保证金} **明确二分**——d 归非拓扑度量（风险投影层），非 μ 分类状态。"
  - name: "INDEX.md 唯一权威裁定"
    version: "docs/formal-chain/INDEX.md（编排者裁定 2026-07-02）"
    role: "母裁定。『本目录=最严格起点下的形式化推导链，为项目形式化唯一权威』。两 PDF 均在本目录内，但 z 形态冲突时以最完整最严格形态（完整的策略.pdf §16「最完整、最严格」自述）为准——本条目据此裁 §16 优先于 §九示意 z。"
  - name: "Jchain 代数派生论证（mu_estimator.rs:136-138）+ 恒等式护栏（selector.rs:250-251）"
    version: "HEAD（gap3-rework-codex9-fix 分支），#138 G3 首次登记"
    role: "Jchain 无独立自由度的实装论证 + 机器可检护栏。origin_level = level + nest_depth 由 §3 N^δ 逐级相邻（nest.rs:140-173/200-218 递归每级 -1）在结构上强制。已覆盖，非缺口。"
  - name: "d 风险层实装（risk.rs + runner.rs）"
    version: "HEAD，risk.rs:74-105（structural_stop：1/2 买=pivot_low、3 买=ZG、卖镜像）+ :194-198（risk_dist_usd=|entry-stop|·tick_size）+ runner.rs:517-553（gate 消费）"
    role: "d 在**风险投影层**（K_Θ/sizing）已生产+消费的实装。层归属正确——非 μ 分类 z 维缺口。"

# 解决方式
resolution:
  type: 定义修正   # 出处/层归属订正 + 权威链调和（非代码行为变更——Jchain 护栏已在、d 已在风险层，零 z 装配改动）。
  description: |
    Lead 裁定(a)（定理类：既有权威链规则直接应用，无需外部裁量）：
    **z 形态权威裁定**：两 PDF 的 Z_t 是两个不同能指。完整的策略.pdf §16（20 维，含 Jchain 不含 d）是
    **唯一权威**——依据 INDEX.md 既有裁定（完整的策略.pdf 为形式化唯一权威）+ §16 自述「最完整、最严格」。
    缠论的全互斥定义策略.pdf §九 Z_t 带「…」是**示意性**（非穷举），不装配进 canonical z；其列出的 d 由
    同 PDF §十亲口归「非拓扑度量」→ 风险投影层。
    **A1 两残余 both 消解**：
    (1) Jchain=已覆盖（代数派生 mu_estimator.rs:136-138 + §3 N^δ 逐级相邻证实 + 恒等式护栏
        selector.rs:251 机器可检）——无加维空间；
    (2) d=不在权威 canonical z（§16 排除）、属风险投影层（§十明定）、已在 risk.rs 生产 + runner.rs 消费
        ——层归属正确，非缺口。
    **零 z 装配代码改动**——强行加维 = 声明膨胀（090号）/ 自造（编排者令「勿自造」）。
    d 不入 μ 桶的三条独立理由：①权威排除（§16 同族收三排一，刻意选择）；②层归属（§十明定 d=非拓扑度量
    → K_Θ/sizing）；③分桶灾难（d=|entry−stop| 含当前价格尺度=连续量，入桶键 ⟹ K 爆炸 = winner's curse，
    UClass 降维本为抗此；d 的止损规则已由 (I_γ, level pivot) 决定，无独立分类信息）。
  decided_by: 蜂群内部（Lead 裁定(a)，定理类=既有 INDEX.md 权威链裁定直接应用；核验=ws-a1dims Task #162 逐页 Read；team-lead 派 #177 立条落盘）

# 被订正的方案
negated:
  description: "gap-master2-final A1 行（继承 shard4）：canonical z 缺 Jchain + d 两维残余，须加维实装。"
  why_negated: |
    (1) **shard4 错源**：shard4 读的是缠论的全互斥定义策略.pdf §九示意 Z_t（其列了 d），未与
        完整的策略.pdf §16 权威最完整形态交叉核对。§16 才是唯一权威（INDEX.md 裁定），其 20 维含 Jchain、
        排除 d，且把 d 的同族 β/c/m 收入 z、把 d 归风险层。gap-master2-final A1 继承了这个错源。
    (2) **Jchain 非缺口**：Jchain 虽在权威 z 内，但 ≅ (origin_level, level) 代数派生（mu_estimator.rs:136-138），
        由 §3 N^δ 逐级相邻（nest.rs 递归每级 -1，不可跳级）在结构上强制无独立自由度，恒等式护栏
        selector.rs:251 机器可检。加维 = 把已由 (ℓ,e) 完整携带的信息重复编码 = 声明膨胀。
    (3) **d 非 μ 分类 z 维**：d 是非拓扑度量（§十明定），属风险投影层，已在 risk.rs/runner.rs 生产消费。
        把风险层的度量硬塞进 μ 分类桶键 ⟹ ①违 §16 权威排除；②违 §十层归属；③按连续价格尺度碎片化分桶
        触发 winner's curse。三条独立理由均指向：d 不入 μ 分类 z。
    故 A1「须加维实装」是出处错误驱动的伪缺口——这是「原始/最权威 PDF 覆盖次级/示意表述」的 source-tracing
    （同 002 号：编纂版/示意表述遗漏或差异须回溯最权威原文）。

# 新产出
new_output:
  definitions:
    - "**两 PDF z 形态权威裁定**：完整的策略.pdf §16（20 维 canonical z，含 Jchain 不含 d）为**唯一权威**
      （INDEX.md 裁定 + §16 自述『最完整最严格』）。缠论的全互斥定义策略.pdf §九 Z_t 带「…」是示意性
      （非穷举），不装配 canonical z。两者 z 形态差异不是矛盾——示意 z 服从最完整形态。"
    - "**d 层归属裁定**：d（失效/止损距离）= 非拓扑度量（缠论的全互斥定义策略.pdf §十明定，与 β/c/m 并列），
      属**风险投影层**（K_Θ/sizing），**非 μ 分类状态 z 维**。d 已在 risk.rs:74-105（structural_stop）+
      :194-198（risk_dist_usd）+ runner.rs:517-553（gate 消费）生产消费——层归属正确，非缺口。"
    - "**Jchain 已覆盖裁定**：Jchain ≅ (origin_level, level) 代数派生，由 §3 N^δ 逐级相邻结构强制无独立
      自由度，恒等式护栏 selector.rs:250-251 机器可检。已覆盖，非缺口，无加维空间——加维=重复编码=声明膨胀。"

# 影响范围
impact:
  affected_modules:
    - "gap-master2-final A1 行 + gap-master2-shard4-20260703.md——错源须订正（读示意性 §九 z 未与权威 §16
      交叉核对）。本条目为**订正载体**，不改写原报告（谱系落痕订正，非篡改历史）。"
    - "rust/src/theta_v0/classifier/mu_estimator.rs:136-138（Jchain 代数派生论证）+ selector.rs:250-251
      （恒等式护栏机器可检）——本条目锚定其覆盖 Jchain『缺口』的正确性，零改动。"
    - "rust/src/theta_v0/backtest/risk.rs:74-105/194-198 + runner.rs:517-553（d 风险层生产+消费）——本条目
      锚定 d 的层归属正确性（风险投影层，非 μ 分类 z），零改动。"
  affected_definitions:
    - "canonical z（完整的策略.pdf §16）：20 维定型，含 Jchain、排除 d。本条目锚定 z 形态权威归属，不新增维。"
    - "Jchain：代数派生（≅(origin_level,level)），无独立自由度，护栏机器可检。"
    - "d（失效/止损距离）：非拓扑度量，风险投影层，非 μ 分类 z 维。"
  downstream_implications:
    - "002 又一 locus：原始/最权威 PDF（完整的策略.pdf §16）覆盖次级/示意表述（缠论的全互斥定义策略.pdf
      §九带「…」）的 source-tracing 实例——凡引用 §九示意 z（含 d）主张『z 缺 d 维』者，须按本号订正为
      §16 权威形态（d 归风险层）。"
    - "#162→#165(A7 exitmu) 解锁链：#162 因本发现翻转任务前提（A1『须加维』伪缺口消解），无 z 装配实装可做；
      #165 出场侧 μ(z,exit) 构造依赖 canonical z 形态稳定（§16 20 维定型）——本条目确认 z 形态不再新增维
      ⟹ #165 的 z 快照口径以 §16 20 维为准。"
    - "凡后续报告以缠论的全互斥定义策略.pdf §九 z（含 d/示意「…」）作为 canonical z 依据者，均属错源，须回溯
      完整的策略.pdf §16 权威最完整形态。d 的止损语义由 (I_γ, level pivot) 决定，在风险层，不入分类桶键。"

# 回溯结算（待编排者 /ritual + 错源核销后补记）
retroactive_settlement:
  settled_by: "编排者 /ritual 最终辨认（同 689/688 待辨认口径）+ gap-master2-final A1 行 / shard4 错源核销后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "002（来源不完备/权威链）——本条目=最权威原始 PDF（§16）覆盖次级/示意表述（§九带「…」）的 source-tracing 实例；090（声明膨胀禁止）——A1『须加维』= 在错前提上加维=声明膨胀，本号否定之。"
  children: []   # gap-master2-final A1 / shard4 错源全库核销后可派生「A1 两残余 both 消解已落地」的实证子记录。
depends_on: ["002", "090", "231"]
related: ["689", "685", "138", "696"]
---

# source-tracing 690：两 PDF z 形态差异权威裁定——完整的策略 §16 为准，d 归风险层（A1 两残余 both 消解）

## 结论

`gap-master2-final` A1 行（继承 `shard4`）判定「canonical z 缺 Jchain + d 两维残余，须加维实装」。经
ws-a1dims 逐页 Read 两份 PDF 与代码交叉核对（Task #162），此判定**both 消解——无 z 装配实装可做**：

1. **z 形态权威裁定**：两 PDF 的 Z_t 是**两个不同能指**。`完整的策略.pdf` §16（20 维，含 Jchain 不含 d）
   是**唯一权威**（依据 INDEX.md 编排者裁定「本目录=形式化唯一权威」+ §16 自述「最完整、最严格」）。
   `缠论的全互斥定义策略.pdf` §九 Z_t 带「…」是**示意性**（非穷举），不装配进 canonical z。

2. **Jchain=已覆盖，非缺口**：Jchain 虽在权威 z 内，但 `mu_estimator.rs:136-138` 已论证其 ≅ (origin_level,
   level) 代数派生、无独立自由度，且 §3 N^δ 定义强制逐级相邻（区间套不可跳级）**证实**该论证。恒等式护栏
   `origin_level = level + nest_depth` 已在 `selector.rs:250-251` 机器可检。无加维空间。

3. **d=属风险层，非 μ 分类 z 维**：权威 §16（20 维）显式列 β(ForceState)/c(CostBucket)/m(MarginState)，
   **独缺 d**。d 只出现在**另一份** PDF（`缠论的全互斥定义策略.pdf` §九）的示意 Z_t 中，且该 PDF §十亲口把
   d 与 β/c/m 并列归**非拓扑度量**、属风险投影层。d 已在 `risk.rs:74-105`（structural_stop）+ `:194-198`
   （`risk_dist_usd=|entry-stop|·tick_size`）+ `runner.rs:517-553`（gate 消费）生产消费——**层归属正确，非缺口**。

**零 z 装配代码改动**——强行加维 = 声明膨胀（090号）/ 自造（编排者令「勿自造」）。

## 定义依据

- **完整的策略.pdf §16**（p10，唯一权威最完整形态）：z_v 20 维清单，含 Jchain、排除 d。同族 β/c/m 收入 z
  独排 d，是权威作者刻意选择。§6（p4）canonical z 同清单。§3（p2-3）N^δ 区间套 `J^δ_e⊆…⊆J^δ_ℓ` 逐级相邻，
  「区间包含非端点相等……不能作跨级 rung」——Jchain 级别结构由递归强制不可跳级。
- **缠论的全互斥定义策略.pdf §九**（p22，二级）：`Z_t=(ℓ,δ,I_γ,r,σ_p,ω,κ,β,d,c,m,…)` **带「…」示意性非穷举**。
  **§十**（p22-23）：纯拓扑态 `{ℓ,δ,I_γ,r,σ_p,κ}` **vs** 非拓扑度量 `{β_t=背驰强度, d_t=失效/止损距离,
  c_t=成本, m_t=保证金}` 二分——d 归非拓扑度量（风险投影层）。
- **INDEX.md 编排者裁定 2026-07-02**：「本目录=最严格起点下的形式化推导链，为项目形式化唯一权威」——
  两 PDF z 形态冲突时以 §16 最完整最严格形态为准。
- **实装锚**：`mu_estimator.rs:136-138`（Jchain 代数派生）、`selector.rs:250-251`（恒等式护栏 + 单测 :547-566）、
  `nest.rs:140-173/200-218`（N^δ 递归每级 -1）、`risk.rs:74-105/194-198`（d structural_stop + risk_dist_usd）、
  `runner.rs:517-553`（gate 消费 d）。ws-a1dims Task #162 逐页 Read + 独立读码核实
  （`.chanlun/review-results/a1-zdims-20260704.md`）。
- **002**（来源不完备/权威链）：最权威原始 PDF 覆盖次级/示意表述的先例。**090**（声明膨胀禁止）：在错前提
  上加维 = 声明膨胀。

## 边界条件（结论翻转）

- **(a) 编排者启动「风险感知 μ」新方向**（把 d 离散化入 μ 桶键）⟹ 须**新预注册 + 抗碎片化设计**（避免连续
  价格尺度触发 K 爆炸/winner's curse），且属**新工位**范围。**此翻转不改变本条目的层归属判定**——d 当前仍是
  风险投影层度量；「风险感知 μ」是把风险层度量**升格**为分类维的**新决策**，须显式预注册，不是补回「缺失的
  z 维」。本条目在该新方向下降级为「历史层归属记录」，不预判编排者裁决。
- **(b) 认定 `缠论的全互斥定义策略.pdf` 示意 z（含 d）覆盖 `完整的策略.pdf` §16** ⟹ 与 INDEX.md 唯一权威
  裁定冲突，须走 change request 重裁权威归属。当前裁定：§16 优先（最完整最严格形态 + INDEX.md 裁定）。
- **(c) Jchain 结论翻转**当 `nest.rs` 出现跳级 rung（当前区间套递归结构不可表达跳级——§3「非端点相等不能作
  跨级 rung」在结构上禁止）。若未来放开逐级相邻约束，Jchain 恢复独立自由度，须重估其是否为独立 z 维。

## 下游推论

- **002 裁决 locus**：最权威原始 PDF（§16）覆盖次级/示意表述（§九带「…」）——凡引用 §九示意 z（含 d）主张
  「z 缺 d 维」者，须按本号订正为 §16 权威形态（d 归风险层）。示意性表述（带省略号）不可作为穷举 canonical z
  的依据。
- **#162→#165(A7) 解锁链**：#162 因 A1「须加维」伪缺口消解、无 z 装配可做而翻转任务前提；#165 出场侧 μ(z,exit)
  构造依赖 canonical z 形态稳定——本条目确认 z 形态定型为 §16 20 维、不再新增维 ⟹ #165 的 exit-time z 快照
  口径以 §16 20 维为准（含 ExitType 维，不含 d）。
- **d 分桶灾难警戒**：d=|entry−structural_stop| 含当前价格尺度（连续量）。任何把 d 入 μ 桶键的实装（除非走
  边界条件(a) 的新预注册 + 抗碎片化设计）都会按连续风险距离碎片化分桶 ⟹ K 爆炸 = §29-30 winner's curse
  （`mu_estimator.rs:218-234` UClass 降维本就为抗此）。d 的止损**规则**已由 (I_γ, level pivot) 决定，无独立
  分类信息——d 入分类桶是零信息增量的碎片化。

## 谱系引用

- 母规则：`002`（来源不完备/权威链——最权威原始 PDF 覆盖次级/示意表述的 source-tracing 先例）、`090`
  （声明膨胀禁止——在错前提上加维 = 声明膨胀）、`231`（形式化有效域——示意 z 定义域 ⊇ 权威 z 有效域，
  d 的分类维「定义域」代数可加但「有效域」在 μ 分类下为空）。
- 姊妹：`689`（「9段升级」出处订正——第33课非第20课；同为 source-tracing 出处订正 + 231 有效域收窄）。
  689 订正**升级机制**出处，本号订正 **z 形态权威归属 + d 层归属**——同一 gap-master2 报告族的两条错源订正。
- 相邻：`685`（#141 漏斗真因——中枢延伸/分解出处）、`138`（G3 Jchain 代数派生首次登记——本号锚定其覆盖
  Jchain『缺口』的正确性）。
- 溯源链：shard4 读示意性 §九 z（未与权威 §16 交叉核对，错源）→ gap-master2-final A1 行继承 → #162
  A1-zdims-residual 逐页 Read 两 PDF + 独立读码核验消解（ws-a1dims，homogeneous）→ team-lead 派 #177 立条。

## 影响声明

纯谱系记录，**零 git 代码改动**。本条目锚定：(1) 两 PDF z 形态差异裁定——`完整的策略.pdf` §16（20 维，含
Jchain 不含 d）为唯一权威，`缠论的全互斥定义策略.pdf` §九带「…」是示意性；(2) d（失效/止损距离）=非拓扑度量
（§十明定），属风险投影层，已在 `risk.rs`/`runner.rs` 生产消费——层归属正确，非 μ 分类 z 维缺口；(3) Jchain
≅(origin_level,level) 代数派生，护栏 `selector.rs:251` 机器可检——已覆盖，非缺口。A1「Jchain+d 缺维残余」
（gap-master2-final 继承 shard4 错源）**both 消解，无 z 装配实装可做**——加维=声明膨胀/自造，被 090号/编排者
令禁止。本条目为订正载体，不改写原报告（谱系落痕订正，非篡改历史）。边界条件(a)（编排者启动风险感知 μ）成立时
须新预注册+抗碎片化设计（新工位），**不翻转本条目的 d 层归属判定**——已显式记录，不预判编排者裁决。

## settle-sweep 增量扫描交叉引用注记（2026-07-04，问题C）

`formal-chain-deepresearch-20260704.md` §3.1 争议一（问题1.pdf 十核实点判定 #3）对抗验证发现：本条目
（690）的三条理由（①权威排除②层归属③分桶碎片化）全部针对「d 是否进 z 分类桶键」这一支，对「estimand
应测原始 μ(z,a)=E[X|z,a] 还是风险调整 μ_R(z,a)=E[X/d|z,a]」（d 缩放结果变量、不进桶键）这一正交支
**未构成论证**——本条目的裁定范围**不如字面看起来那样完整**。genealogist 已就此开立独立谱系条目
`696`（`settled/696-mu-r-estimand-outcome-scaling-orthogonal-to-690-bucket-key-exclusion.md`），
将本条目的桶键裁定（维持有效）与 estimand 选择（690 未裁，696 号新开）显式分离。

**本条目状态不变**——d 不进 z 桶键的裁定继续有效，本注记只是把「本条目的有效范围止于桶键机制」这一
边界显式化，防止后续报告误把 690 的裁定当作 estimand 问题的答案（该误用已在 formal-chain-deepresearch
§3 核实点#3 中实际发生过一次）。

## μ_R 归零下游印证注记（2026-07-04，a6 co-primary 实测）

696 号（estimand 分支）已由编排者委托 codex 裁定为选项 B（μ_R=E[X/d|z] 作 co-primary），a6
（`prereg-rev2-results-20260704.md`）按裁定实装并跑数。**a6 的 μ_R 实测结果是本条目「d 归风险层、
不进 μ 分类桶键」裁定的下游印证**：

- a6 μ_R δ-free 主裁决 25 桶 = **V=0/F=1/I=24**（无 Validated）。raw μ 的唯一 Validated 桶
  （L0 bsp3 σ+1 force=None，已知 beta 漂移伪结构）在 μ_R 除 d 后**退为 Inconclusive**
  （perm_p 0.000→0.085，LCB +89.9→−0.16）——风险归一化后 beta 漂移伪结构的「超额」被止损距离摊平。
- **对本条目的印证方向**：a6 用 co-primary 方式让 d **只缩放结果变量 X**（μ_R=E[X/d]），桶键**仍是
  δ-free (level,bsp_class,parent_dir) 三元组、d 不进桶键**——这正是本条目裁定的「d 不入 μ 分类桶键」
  在 estimand 层的忠实实装。d 以「结果变量缩放因子」进入检验（696 的 estimand 分支），而非以「分类维」
  进入桶键（本条目已排除的分支）。两条轴各自成立、互不覆盖：本条目管桶键（d 不入），696 管 estimand
  （d 缩放 X，裁定=B）。a6 实测确认这一分离在代码层可实现且不违反本条目的桶键裁定。
- **本条目状态不变**：d 层归属裁定（非拓扑度量→风险投影层，不进 μ 分类桶键）继续有效。a6 的 μ_R
  co-primary 不是「把 d 放进桶键」（那会触发本条目边界条件(a) 的翻转）——它是 696 裁定的独立
  estimand 轴，桶键始终无 d。故本条目边界条件(a)（编排者启动「d 离散化入 μ 桶键」）**未被触发**——
  a6 走的是「d 缩放结果变量」路径，桶键无 d，本条目裁定不受影响，反被印证（d 在风险层的度量语义
  即「除以 d = 单位风险收益」，与 sizing 层 1/d 同源，均在风险投影层而非分类层）。
