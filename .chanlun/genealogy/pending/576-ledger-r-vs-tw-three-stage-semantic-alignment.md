---
id: 576
title: "账本 R=Π-A-W vs 缠论取本金三阶段 TW=free+holding+withdrawn 不同构——语义对齐三选一"
type: 概念分离（选择类裁定 escalate）
status: 生成态
date: 2026-06-26
escalated_by: cc-gap90-totalwealth 工位（task #90）
related:
  - '534'  # 嵌套递归会计语义（独立暴露 vs 降成本短差）——同一交易两视图，本号是不同切面
  - '538'  # 嵌套赋格会计双重性（同一笔物理交易父子双视图）——确立"双视图共存"可借鉴范式
  - '231'  # 形式化有效域规则——不同构是 L0 定理（有效域=定义域）
  - '050'  # 守恒语义空洞——守恒律有效域 vs 定义域
provenance: "[新缠论:形式化] machine-checked 不同构（formal/Tlayers/Accounting/TotalWealth.lean，lake build 全绿，无 sorry/admit/axiom）+ codex gpt-5.5 异质审查确认 + 缠师第31课一级权威"
epistemological_level: "不同构裁定 = L0（两模型代数/状态结构推导，有效域=定义域）；三选一方向 = 产品级价值判断（不可由 L0 定理决定）"
---

# 576号：账本 R=Π-A-W vs 缠论取本金三阶段 TW 不同构——语义对齐三选一

## 矛盾报告

### 矛盾描述

系统中并存两个**不同构**的账本会计模型，task #90 需裁定二者的语义对齐方向：

- **模型A（财务守恒账本 R=Π-A-W）**：codex from-origin 提案的 `LedgerState{Pi,A,W,R,
  inv:R=Π-A-W}` + 我们已建的 `Strict/HybridAssembly.lean:90` `LedgerComp`（同结构）。
  R=储备、Π=累计利润、A=已分配/资本化、W=已提取。`inv: R = Π - A - W` 是**结构不变量**
  （R 由 Π/A/W 代数决定，非独立自由度）。三事件 realize/allocate/withdraw 皆 Int 可正可负，
  **操作可逆**，R=Π-A-W 全程单一恒等。

- **模型B（缠论取本金三阶段 TW=free+holding+withdrawn）**：缠师第31课「取本金」语义
  （一级权威），`rust/src/recursive_t/t_engine.rs:107-451` + `rec_engine.rs:1219-1266`
  实装，t_engine 有 prove_tw_neutral L2 守卫 + L3 回测验收。守恒量 `TW=free+holding+
  withdrawn`，三阶段状态机 `CostReduction→CapitalRecovered→EarningShares` **单向不可逆**
  （OQ-9），守恒律**随阶段切换**（CostReduction 守 Σ|units| 股数 / EarningShares 守 K
  外部投入，design §3.3「守恒律从股数守恒扬弃为外部投入资本守恒」）。

**不可弥合性（machine-checked，formal/Tlayers/Accounting/TotalWealth.lean，L0）**：二者
**不同构**——不存在「保守恒量+保操作语义」的双向忠实映射。三条结构性阻断：
1. **状态空间结构**（最强，codex 标不可反驳）：A 是可逆平移系统（Z³ 强连通）；B 含单向
   不可逆 stage 迁移（EarningShares 锁死）。B 的两态可 free/holding/withdrawn 三量全同仅
   stage 不同 ⟹ 任何 B→A 投影非单射（`not_isomorphic_stage_collapses`）。
2. **守恒律切换**：A 单一恒等 R=Π-A-W（与阶段无关永真）；B 守恒律随 stage 翻转
   （`not_isomorphic_conservation_switches`）。
3. **外生价格维度**：B 的 holding=Σunits·c 依外生市价 c（跨 bar 变动=盈亏）；A 纯 Int 格
   无外生维度（`not_isomorphic_exogenous_price`）。

**为何 CapitalPhase 不构成调和（已诊断排除一个伪解）**：`Foundation/HybridStateMachine.lean:191`
的 `CapitalPhase{phaseI/II/repair/protectedPhase/accretive}` 经核查，其 `choosePhase` 是从
4 个 Bool（returned/pendingOrReady/reserveBelow/reserveBelowAccrete）派生的**无状态纯函数
分类标签**——它每次从当前账本量重算，**无单向性、无守恒律切换载体**（可从 accretive
重算回 phaseI）。故 CapitalPhase 无法承载模型B 的 stage 单向不可逆性与守恒律切换，**不构成
两模型的调和**（确认任务卡断言）。

### 双方论证

**立场A（以 R=Π-A-W 为准，丢三阶段取本金语义）：**
- 依据：R=Π-A-W 是 codex from-origin + 我们两版**共用**的财务守恒，`HybridAssembly.lean`
  已建 LedgerComp 接入闭环装配（补 piTheta_causal 账户因果洞），与全定义策略闭环已耦合。
- 代价：**丢弃缠师第31课取本金语义**——这是缠论一级权威的核心操盘原则（"原来投入的资金
  全部收回来了"→立于不败→增股数），t_engine 已 L3 验收。丢它=丢已验证的缠论核心定义集。

**立场B（以 TW 为准，丢 R=Π-A-W）：**
- 依据：TW 三阶段是缠师第31课一级权威 + L3 实证，承载"取本金"这一缠论独有的财富守恒语义
  （stage 单向不可逆 + 守恒律切换是缠论操盘的本质结构）。
- 代价：**丢 R=Π-A-W 闭环接缝**——HybridAssembly.lean 的账户因果装配需重做；codex from-origin
  的 LedgerState 架构优势丢失。

**立场C（双层并置，非同构映射）：**
- 关键诊断（machine-checked 已结算可自决部分）：「双层映射」若指**忠实双向同构映射**，
  **已被不同构定理排除**（不存在）。剩下的「双层」只能是**两层并置 + 单向投影/嵌入**
  （如 TW→R 的部分投影，丢 stage 维度），这不是「调和」，是「承认两个会计范畴共存」。
- 范式借鉴：538号确立「同一对象两视图共存（存一次两读）」——但 538 是同一交易的双视图
  （信号层对偶），本号是两个**不同守恒结构**的并置，借鉴范式但切面不同。
- 我的 `TotalWealth.lean` 已事实上实现立场C 的并置（独立 TWState + 本地镜像 LedgerComp +
  不同构反例），但**未声称二者调和**——只机器证明了不同构 + 各自自洽。

### 涉及的定义

- 缠师第31课「资金管理的最稳固基础」（`docs/chanlun/text/blog/031-第31课.md` line 24/28/36）：
  取本金三阶段（成本变0前短差恒仓 / 出掉部分把成本降为0退本金 / 成本为0后增股数），一级权威。
- 模型A：`formal/Strict/HybridAssembly.lean:90-163`（LedgerComp + ledgerStep）。
- 模型B：`rust/src/recursive_t/t_engine.rs:107-451`（TStage + TPositionEngine 三阶段）+
  `rec_engine.rs:1254-1266`（TW + prove_tw_neutral）+ `docs/three_stages_accounting_design.md`
  （§3.3 守恒律切换 / OQ-9 相变可逆性矛盾）。
- CapitalPhase：`formal/Foundation/HybridStateMachine.lean:191-249`（派生标签，非守恒律）。
- 不同构 machine-checked 证明：`formal/Tlayers/Accounting/TotalWealth.lean`（lake build 全绿）。

### 谱系比对结果

- **534号**（嵌套递归会计语义）：处理"独立暴露 vs 降成本短差"的 regime 分离——是会计语义的
  概念分离先例，但针对的是**同一交易的两种会计读法**，非两个独立守恒结构。
- **538号**（嵌套赋格会计双重性）：确立"同一笔物理交易父级别=降成本/子级别=独立头寸，存一次
  两视图读"+ "资金守恒是递归嵌套的解锁条件"。本号可借鉴其**双视图共存范式**（立场C），但
  538 的双视图是同一对象的对偶面（同构于信号层 537），本号是**两个不同构守恒结构**的并置
  ——切面不同，不是 538 的直接重复。
- **050号**（守恒语义空洞）+ **231号**（形式化有效域）：确立"守恒律的有效域可能 < 定义域"。
  本号的不同构裁定是 L0（有效域=定义域，纯结构推导），与 050/231 一致——R=Π-A-W 是定义性
  恒等（L0 永真），TW 守恒是 L2 操作不变量（同价才成立），二者认识论等级本就不同。
- **无直接先例**处理"两个不同构账本守恒结构的产品级取舍"——本号是新切面。

### Lead 的建议方案

**建议立场C（双层并置），但需编排者确认产品边界。** 理由：
1. 不同构是 L0 定理（machine-checked），二者**不能合一为单一同构账本**——强行选 A 或 B
   都丢弃一方的已验证语义（no-workaround：丢弃已验证定义=不严格）。
2. 立场C 不是逃避——它**承认两个会计范畴各自自洽且都被需要**：R=Π-A-W=利润→分配→提取→储备
   的财务守恒视角（闭环装配/资本分配）；TW=在险资产→退出路径→财富守恒的取本金视角（缠论
   操盘本质）。完整持仓系统两者都需要、互不替代。
3. 立场C 的具体落地：TotalWealth.lean 已建（两层并置 + 不同构反例 machine-checked）。后续
   可补一个**诚实的单向投影**（TW→R 的部分映射，明确标注丢失 stage 维度 = 有效域收缩），
   而**不冒充双向同构**（formalization-validity-domain）。

**但这是产品级价值判断，Lead 不单方决定。** 若编排者认为系统只应承载单一账本范畴
（如为了简化全定义策略闭环），则需在 A/B 间取舍——那是丢弃一方已验证语义的存在论决定，
超出蜂群自决范围。

### 需要决断的问题

用缠论语言精确表述：

**缠师第31课「取本金」的三阶段守恒（恒仓降成本 → 退本金立于不败 → 成本为0后增股数，守恒律
随阶段单向切换）与财务守恒「储备=利润−分配−提取」，是两个不可合一的会计范畴（已 machine-checked
不同构）。系统应当：**

1. **只承载财务守恒 R=Π-A-W**（放弃形式化缠师取本金三阶段语义，接受系统不覆盖第31课这一
   一级权威操盘原则）？还是
2. **只承载取本金三阶段 TW**（放弃 R=Π-A-W 财务守恒，HybridAssembly 闭环账户因果接缝重做）？
   还是
3. **双层并置**（两个会计范畴各自保留，TotalWealth.lean 已建，补诚实单向投影，不冒充同构）？

**Lead 建议 3**，但请编排者确认：**系统的完整持仓管理是否需要同时承载"财务分配视角"与
"缠论取本金视角"两个会计范畴？** 若是 → 立场C；若系统定位只需其一 → 编排者在 A/B 间裁定
（这是丢弃哪一方已验证语义的存在论决定）。

### 已自决部分（不外包给编排者，no-unnecessary-escalation）

- ✅ **不同构事实**：machine-checked（TotalWealth.lean，L0，lake build 全绿）——非待裁定项。
- ✅ **CapitalPhase 不构成调和**：已核查 HybridStateMachine.lean:191（派生标签无守恒律切换）——
  排除"用 CapitalPhase 调和"的伪解。
- ✅ **「双层映射」= 忠实双向同构**：已被不同构定理排除（不存在）——立场C 只能是并置+单向投影。
- ⬆️ **仅 escalate**：系统承载哪个/哪些会计范畴的产品级价值判断（A/B/C 三选一的存在论决定）。
