---
id: "696"
number: 696
status: 已结算   # 2026-07-04 结算：编排者显式委托 codex 裁定选项 B（codex-ruling-696-20260704.md）+ a6 实装跑数落地（prereg-rev2-results-20260704.md）。选择类价值判断已由编排者通过委托 codex 行使，实装闭合，三开放项消解，无待裁残留。见文末「结算说明」。
date: "2026-07-04"
type: bias-correction
title: "μ_R(z)=E[X/d|z] estimand（结果变量按 d 缩放）与 690 号裁定的 d-入-z-桶键 是两条正交轴——690 三条理由未覆盖 estimand 分支，须新预注册（问题C，TENSION）"
source: "[新缠论] .chanlun/review-results/formal-chain-deepresearch-20260704.md §3.1 争议一（问题1.pdf 十核实点判定 #3 对抗争议）；690号谱系边界条件(a)"
negation_source: heterogeneous   # 深度研究报告（对抗验证）指出核实方与反驳方的范畴滑移，本条目是 genealogist 对该滑移的谱系化
negation_model: "codex (codex-ruling-696-20260704.md decide 选项 B，编排者显式委托裁决权)"   # 结算时更新：原为 null（仅精确化 690 覆盖范围）；编排者 2026-07-04 显式委托 codex 对本号四选项裁决，裁定 = B
negation_form: separation   # 「d 归属μ」被 690 当作单一问题处理（d 是否入 z 分类桶键），本号分离出一个 690 未触及的正交子问题（d 是否缩放结果变量 X，不进桶键）
topo_effect: |
  split — 690 号「d 层归属裁定」节点分裂为两支：
  (1) 【690 已裁，维持】d 作为 μ 分类 z 桶键维度——690 三条独立理由（权威排除/层归属/分桶灾难）成立，
      d 不入 z，属风险投影层（K_Θ/sizing），已在 risk.rs/runner.rs 生产消费。此支不受本号影响。
  (2) 【690 未裁，本号新开】d 作为 estimand 结果变量缩放因子——μ_R(z)=E[X/d|z] 不要求 d 进 z 桶键，
      只对结果 X 做 d 归一化（类比 sizing 层已做的 1/d 头寸归一化，但作用对象是「拿什么当检验的因变量」
      而非「拿什么当仓位大小」）。690 的三条理由（①权威§16排除d维②§十层归属③分桶碎片化）全部针对
      「d 进桶键」，对「d 缩放结果变量」不构成论证——690 未裁决此支。
  target=690号「d层归属裁定」；scope=local（仅 estimand 定义分支，不影响 690 已裁的桶键分支、不影响
  risk.rs/runner.rs 现有实装）。

# 矛盾（bias-correction 的被订正对象——范畴滑移）
contradiction:
  description: |
    深度研究报告问题1.pdf核实点#3判定链：核实方主张「d 未进 z 状态」GAP_REFUTED（因 sizing 层已按 1/d
    归一化 + 690 号裁 d 归风险层）；对抗反驳指出这是范畴滑移——问题C 真正的承重支不是「d 是否入分类桶键」
    （690 已裁：不入，属风险层），而是「estimand 应该是原始 μ(z,a)=E[X|z,a] 还是风险调整
    μ_R(z,a)=E[X/d|z,a]」。当前主程序（bucket_verdict/stratified_delta_perm_p）只检验前者，从未实装后者。
    690 号边界条件(a)确实预见了「风险感知 μ」这个方向，但其措辞（「把 d 离散化入 μ 桶键」）字面上只覆盖
    「d 进 z 桶键」这一种机制，未字面覆盖「d 缩放结果变量、不进 z」这一种正交机制。二者都是「让 μ 对风险
    敏感」的手段，但作用对象不同（分类维 vs 因变量尺度），前者已被690三条理由排除，后者未被论证覆盖。
  layer: 概念   # estimand 定义（检验对象是什么）的选择类问题，非代码 bug、非690裁定的对错问题
  trigger: "formal-chain-deepresearch-20260704.md 对问题1.pdf核实点#3的对抗验证（第二轮反驳）——发现核实方
    『sizing支ALIGNED』与反驳方『estimand支为真实开放项』两支论证对象不同，整体判定应为TENSION而非
    单纯的GAP_REFUTED/GAP_CONFIRMED二元。"

# 涉及的定义
definitions_involved:
  - name: "690号：d 层归属裁定"
    version: "pending/690-two-pdf-z-shape-authority-ruling-complete-strategy-s16-authoritative-d-belongs-risk-layer.md"
    role: "本号的直接父裁定。690 的三条理由（①§16权威排除同族收三排一②§十层归属非拓扑度量③分桶碎片化winner's curse）全部以「d 进 z 桶键」为论证对象，对「d 缩放结果变量」未论证——本号是690边界条件(a)在具体机制上的精确化分支。"
  - name: "690号边界条件(a)：风险感知 μ 新方向"
    version: "同上 pending/690 §边界条件(a)"
    role: "690 已预见「风险感知μ」需新预注册，但字面表述限定在『把 d 离散化入 μ 桶键』——本号指出还存在字面未覆盖的第二种机制（d 缩放结果变量，不进桶键），需要690在此方向上补充预注册范围，或另立新预注册。"
  - name: "risk.rs:147 sizing 1/d 归一化（qty∝ρ·NAV/|entry−stop|）"
    version: "HEAD，risk.rs:147"
    role: "已实装的『1/d 归一化』作用对象是仓位大小（sizing），不是检验的因变量（estimand）。核实方把这个已实装的归一化误当作estimand问题的答案——这是两个不同层面的『1/d』，一个决定下多少仓位，一个决定拿什么当μ的分子。"
  - name: "主裁决程序 bucket_verdict/stratified_delta_perm_p"
    version: "HEAD（分支 gap3-rework-codex9-fix）"
    role: "当前只检验 μ(z,a)=E[X|z,a]（原始收益），从未实装 μ_R(z,a)=E[X/d|z,a]（风险调整收益）——estimand 选择尚未被显式预注册过，两者皆非默认正确。"

# 解决方式
resolution:
  type: 已裁定   # 结算更新：原为「未解决」（选择类待编排者）；编排者显式委托 codex 裁定 = 选项 B（μ_R 作 co-primary 预注册）
  description: |
    【结算前状态（保留）】本号不裁定「应测原始μ还是μ_R」——按 no-unnecessary-escalation 四分法，这是
    **选择类**（多种合理方案，需编排者价值判断：检验原始收益还是风险调整收益，两者都是合法的alpha定义，
    选哪个改变的是策略对象的经济含义而非对错）。genealogist 职责止于：把690的裁定范围精确化为「只覆盖桶键
    分支，不覆盖estimand分支」，并标注这是新的、独立的预注册决策点，须走 /escalate。
    【结算（2026-07-04）】选择类上浮已闭合：编排者 2026-07-04 显式委托 codex 对本号四选项裁决（例外授权，
    见 codex-ruling-696-20260704.md），裁定 = **选项 B**——μ_R=E[X/d|z] 进下一轮预注册作 raw μ 的 co-primary，
    桶键仍 δ-free (level,bsp_class,parent_dir) 不加 d。a6 已按裁定实装跑数（双门 co-primary，E[Y/d] 残差框架），
    实测无任一桶双门 VALIDATED，终局 INCONCLUSIVE 不翻。详见文末「结算说明」。
  decided_by: "编排者显式委托 codex 裁定（codex-ruling-696-20260704.md，选项 B）+ a6 实装落地（prereg-rev2-results-20260704.md）——选择类价值判断由编排者通过委托 codex 行使"

# 被否定的方案（订正对象：范畴滑移的误判）
negated:
  description: "深度研究报告核实链条中隐含的推论：『690号已裁d归风险层』+『sizing层已实装1/d』⟹『问题C 已ALIGNED/可判GAP_REFUTED』。"
  why_negated: |
    这个推论把两个不同层面的『d 与 μ 的关系』合并成一个已解决的问题：
    (1) 690 只裁定了「d 是否进 z 分类桶键」——裁定为否，属风险层，已实装（risk.rs sizing）。
    (2) 但「estimand 应测原始 μ 还是风险调整 μ_R」是另一个问题——即使 d 完全不进 z 桶键，仍然可以选择
        对结果变量 X 做 d 归一化后再检验（μ_R=E[X/d|z]，z 本身不含 d 维）。这个选择690从未论证，
        690的三条理由（权威排除/层归属/分桶灾难）无一针对「结果变量缩放」这个机制。
    把(1)的裁定当作(2)已解决，是把「d 不进分类维」误推广为「d 与 μ 检验完全无关」——逻辑上不成立，
    因为 estimand 的定义域（测什么因变量）与分类桶键的定义域（按什么分组）是两个独立选择维度，
    对一个维度的裁定不自动约束另一个维度。
  layer_gap_type: "估计量选择维度 ⊥ 分类桶键维度——两个正交的『d 如何进入检验流程』的问题被合并处理"

# 新产出
new_output:
  definitions:
    - "estimand 分支识别：μ(z,a)=E[X|z,a]（原始收益，当前主程序默认检验对象）与 μ_R(z,a)=E[X/d|z,a]
      （风险调整收益，z 不变仍为δ-free (level,bsp_class,parent_dir)三元组，d 只缩放分子 X）是两个不同
      的检验对象，690号裁定不预判二者选择——690只裁『d 不进 z』，不裁『测哪个因变量』。"
    - "690号边界条件(a)覆盖范围精确化：(a) 的『把 d 离散化入 μ 桶键』字面只指分类维机制；结果变量缩放
      （estimand 选择）是另一种机制，690未论证，需独立预注册或补充(a)的范围声明。"
    - "【结算新增】estimand 选择裁定 = codex 选项 B：μ_R=E[X/d|z] 作 raw μ 的 co-primary（双门，两者
      都过门才 PASS），桶键仍 δ-free 三元组不加 d。a6 实装（E[Y/d] 残差框架，非字面 E[X/d]）跑数确认
      终局不翻。"
  code_changes: "结算前=无（选择类登记）；结算后=a6 已实装 μ_R co-primary（mu_estimator.rs ResidualTrade+d、
    perm_test.rs DeltaFreeKey、wverify_run.rs μ_R co-primary 块，见 prereg-rev2-results §3.6 影响声明），
    prereg 冻结 26ebe90b29 + 实装 2d8ad3fd71。本谱系条目本身零代码改动。"
  orchestration_changes: "结算后：codex-ruling-696 建议 Lead 触发 GOAL_AMEND 为路线 A 增补验收项——属 Lead 范围，本工位不代执行。"

# 影响范围
impact:
  affected_modules:
    - "结算前=无（纯概念登记）；结算后=a6 实装涉 mu_estimator.rs/perm_test.rs/wverify_run.rs（estimand 定义处）——μ_R co-primary 已落地，非预判。"
  affected_definitions:
    - "690号：本号结算确认与 690 桶键裁定正交——690 桶键裁定维持有效，本号 estimand 裁定（=B）独立成立，两条轴各自结算，互不覆盖。"
  downstream_implications:
    - "路线A（Π_signal-full）剩余量级(i)『d 入状态/μ_R estimand』的 estimand 子项已裁定=B 并实测（无双门 VALIDATED）——该子项闭合。桶键子项由 690 裁定（d 不入桶键），亦闭合。"
    - "前瞻约束（codex 开放项3，已落 a6 边界条件(d)）：若未来 X 改为 position-sized PnL（qty 缩放版），μ_R 的 co-primary 地位须重新评估或降级为诊断项——写入本号供未来回溯。"

# 回溯结算
retroactive_settlement:
  settled_by: "编排者显式委托 codex 裁定（codex-ruling-696-20260704.md，选项 B）+ a6 实装跑数落地（prereg-rev2-results-20260704.md，冻结 26ebe90b29 / 实装 2d8ad3fd71）"
  settlement_date: "2026-07-04"
  settlement_description: |
    选择类价值判断由编排者通过显式委托 codex 行使（既往「异质裁决≠实施授权」默认规则的显式例外，仅限
    696 四选项范围）。裁定 = 选项 B（μ_R=E[X/d|z] 进下一轮预注册作 raw μ 的 co-primary，桶键仍 δ-free
    (level,bsp_class,parent_dir) 不加 d）。a6 按裁定实装并跑数（E[Y/d] 残差框架双门），实测：raw μ 唯一
    Validated 桶在 μ_R 除 d 后退为 Inconclusive（perm_p 0.000→0.085，LCB +89.9→−0.16），无任一桶双门
    VALIDATED，终局 INCONCLUSIVE 不翻。codex 三开放项（d 退化防护 D_MIN=tick_size、口径不可比声明、
    double-count 前瞻约束）在 a6 全部消解，无待裁残留。三个边界翻转情形（并入690/永久排除/已隐含实装）
    皆未触发，结算方向唯一。

# 谱系关联
related_records:
  parent: "690（d层归属裁定——本号是690边界条件(a)覆盖范围的精确化分支，非690的否定）"
  children: []
depends_on: ["690", "231", "018"]
related: ["693", "684"]
---

# bias-correction 696：μ_R estimand 缩放分支与 690 号桶键裁定正交——690 三理由未覆盖，须新预注册【已结算=codex 选项 B】

## 结论

深度研究报告（`formal-chain-deepresearch-20260704.md` §3.1 争议一）对问题1.pdf 核实点#3 的对抗验证揭示：
「d 是否影响 μ 检验」实际上是**两个正交问题**，690号谱系只裁定了其中一个：

1. **d 是否进 z 分类桶键**（690号已裁）：不进。d 是非拓扑度量（§十明定），属风险投影层，已在
   `risk.rs`/`runner.rs` 生产消费。690 三条独立理由（权威排除/层归属/分桶碎片化）成立，此支**维持不变**。
2. **estimand 应测原始 μ(z,a)=E[X|z,a] 还是风险调整 μ_R(z,a)=E[X/d|z,a]**（690号**未裁**）：z 本身不变
   （仍是 δ-free 三元组），d 只对结果变量 X 做缩放。690 的三条理由全部以「d 进桶键」为论证对象，对
   「d 缩放结果变量」不构成论证——这是690边界条件(a)字面未覆盖、但精神上属于同一「风险感知μ」范畴
   的独立机制。**【结算】此支已由编排者委托 codex 裁定 = 选项 B（μ_R 作 co-primary），a6 实装跑数确认终局不翻。**

`sizing` 层已实装的 `1/d` 归一化（`risk.rs:147 qty∝ρ·NAV/|entry−stop|`）解决的是「下多少仓位」，不是
「拿什么当检验的因变量」——两者是不同层面的「1/d」，不能因前者已实装就推定后者已解决或不需要。

## 定义依据

- `690号` `pending/690-two-pdf-z-shape-authority-ruling-...` 三条理由：①权威§16排除d维（同族β/c/m收入z
  独排d）②§十层归属（d=非拓扑度量→风险投影层）③分桶碎片化（d连续量入桶键触发winner's curse）——
  三者均针对「d进z桶键」，未论及「d缩放结果变量X」。
- `690号边界条件(a)`：「编排者启动『风险感知μ』新方向（把d离散化入μ桶键）⟹须新预注册」——字面
  「离散化入μ桶键」限定为分类维机制，本号指出的结果变量缩放不涉及离散化/入桶键。
- `formal-chain-deepresearch-20260704.md §3` 核实点#3：sizing层1/d归一化（无争议）；estimand μ_R未实装
  且核实自认为真（无争议）；争点=μ_R是可分离的预注册选择还是问题C结论的直接内容。
- **【结算依据】`codex-ruling-696-20260704.md`**：编排者显式委托 codex 裁决权（本次例外），裁定=选项 B。
  我方独立复核（亲验 mu_estimator.rs:301-326 + l3_delta_r_alpha.rs:197）确认「X 未按仓位缩放」前提为真、
  「d ex-ante 可得」（risk.rs:74-105）成立——两处关键事实核验通过，裁定 B 不建立在误读代码之上。
- **【结算依据】`prereg-rev2-results-20260704.md`（a6）**：μ_R co-primary 双门实装+跑数，无双门 VALIDATED。

## 边界条件（结论翻转）

- 若编排者裁定690号边界条件(a)的「风险感知μ」范围本就包含结果变量缩放机制（非仅离散化入桶键）
  ⟹ 本号并入690号边界条件(a)统一预注册。**【结算核对：未触发】**——codex 裁 B = 独立 co-primary 预注册，
  非并入 690。
- 若编排者裁定当前只测原始μ、μ_R永久排除在本轮路线A范围外 ⟹ 本号降级为「历史记录」。
  **【结算核对：未触发】**——μ_R 反而被实装为 co-primary。
- 若发现主裁决程序已隐含实装μ_R（本号未核实到的代码路径）⟹ 本号消解，改标「命名/文档缺口」。
  **【结算核对：未触发】**——a6 是新实装，非既有隐含路径。
- **【结算后新增翻转条件】** 若未来 X 的定义改为 position-sized PnL（qty 缩放版）⟹ μ_R=E[X/d] 与 sizing
  层 1/d 构成 double-count，co-primary 地位须重新评估或降级为诊断项（codex 开放项3，已落 a6 边界条件(d)）。
  此翻转不否定本次结算，是本次结算已显式记录的前瞻约束。

## 下游推论

- 路线A（Π_signal-full）剩余项(i)「d 入状态/μ_R estimand」两个子项均已闭合：桶键子项（690 裁定 d 不入桶键）
  + estimand 子项（本号裁定=B 并 a6 实测无双门 VALIDATED）。
- 本号原为四分法「选择」类问题——**结算路径**：选择类的编排者处置采取了「编排者显式委托异质模型（codex）代裁」
  的形式（本次例外授权），不同于本工位自裁（被 no-unnecessary-escalation 禁止），闭合合法。

## 谱系引用

- 母节点：`690`（d层归属裁定——本号是其边界条件(a)覆盖范围的精确化分支，不否定690的桶键裁定）。
- `231`（形式化有效域——690三理由的有效域是「桶键机制」，本号是231先例的又一实例：有效域<字面定义域）。
- `018`（四分法分类——本号判定为「选择」类；结算路径=编排者委托 codex 代裁的例外授权）。
- 姊妹：`693`（A10/A11硬裁决三分冻结——同为策略对象定义须编排者裁决的选择类先例）。
- 结算依据：`codex-ruling-696-20260704.md`（编排者委托裁决=B）、`prereg-rev2-results-20260704.md`（a6 实装跑数）。
- 结算先例：`679`/`692`（codex 终局裁定 + 当前状态已正确 + 无待裁残留 ⟹ genealogist 可直接结算）。

## 影响声明

本号从生成态转已结算。**谱系条目本身零代码改动**；结算所依据的 a6 实装（μ_R co-primary）改动在
mu_estimator.rs/perm_test.rs/wverify_run.rs，由 a6 工位负责（本号不代表其真封状态）。本号精确化 690 号裁定的
覆盖范围（桶键机制 vs 结果变量缩放机制两条正交轴）这一核心发现**在结算后仍然有效**——结算的是 estimand
分支的裁定（=B），不是「690 本就覆盖 estimand」。690 桶键裁定与本号 estimand 裁定是两条独立轴，各自成立。

## 结算说明（2026-07-04）

**结算判定**：本号从生成态转已结算。四分法判据核验（team-lead 三条件 + 679/692 先例）：

1. **裁决主体 = 编排者显式委托的 codex**：`codex-ruling-696-20260704.md` 记录编排者 2026-07-04 显式委托
   （「你有问题直接问 codex」「让他给你裁定」），本次裁定为「异质裁决≠实施授权」默认规则的**显式例外**
   （仅限 696 四选项范围）。选择类的价值判断（原始 μ vs μ_R）已由编排者通过委托 codex 行使——裁定 =
   **选项 B**（μ_R=E[X/d|z] 进下一轮预注册作 raw μ 的 co-primary；桶键仍 δ-free (level,bsp_class,parent_dir)，
   d 不进桶键）。选择类上浮闭合。

2. **实装落地**：a6（`prereg-rev2-results-20260704.md`）按裁定 B 实装并跑数——μ_R co-primary 双门（残差框架
   落点 E[Y/d]，非字面 E[X/d]——X δ-baked 会污染 perm 显著性，perm_test.rs:506 铁律），prereg 冻结
   `26ebe90b29`（#135 冻结先于跑数纪律满足）+ 实装 `2d8ad3fd71`。实测：raw μ 唯一 Validated 桶
   （L0 bsp3 σ+1 force=None）在 μ_R 除 d 后退为 Inconclusive（perm_p 0.000→0.085，LCB +89.9→−0.16），
   **无任一桶双门 VALIDATED**，终局 INCONCLUSIVE 不翻（161 否定性照实）。

3. **codex 三开放项已消解**（无待裁残留）：(a) d 退化防护——D_MIN=tick_size，无一笔触发（prereg §2.3，
   无近零止损距离）；(b) 口径不可比声明——μ_R 与 raw μ 不可比已显式声明（除 d + 样本集不同
   raw_n=2256→μ_R_n=2184，剔除 d 不可得 72 笔，#135）；(c) double-count 前瞻约束——「X 改 position-sized
   ⟹ μ_R 降诊断」写入 a6 边界条件(d)。三项均落地，无 pending fork。

**与本号原边界条件的核对**：三个翻转情形皆不成立（详见上文「边界条件」结算核对标注），结算方向唯一。

**结算不改变的事实**：696 识别的「690 三理由只覆盖桶键分支、estimand 分支正交未裁」这一范畴分离**仍然有效**——
结算的是「estimand 选择已由编排者委托 codex 裁定=B 并实装」，不是「690 原本就覆盖了 estimand」。690 的桶键裁定
与 696 的 estimand 裁定是两条独立轴，各自成立。

**dag.yaml 同步请求**：本工位无 dag 大文件安全写入能力（同 692 先例），请 Lead 将 dag 中 696 的
`status: 生成态 → 已结算`、`file: pending/... → settled/...`，并将 pending/696 stub `git rm`（本工位无删除权）。
