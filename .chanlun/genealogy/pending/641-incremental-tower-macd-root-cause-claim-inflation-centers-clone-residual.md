---
id: "641"
number: 641
status: 生成态   # genealogist 结构记录：声明膨胀（090号）发现 + memory signal-on2 模式复发。标度实测数据由 Lead 给出（exp 1.65），独立坐实待标度工位。最终结算待编排者 /ritual。
date: "2026-06-28"
type: bias-correction   # 声明膨胀纠正（090号族）
depends_on: ["090", "231"]
related: ["640", "639", "signal-on2-two-independent-hotspots"]
title: "增量塔 API 声明『塔构造超线性消除 ⟹ exp≈1』但 exp 仍 1.65——centers.clone() O(k)/iter 残留热点未声明，声明膨胀（090号）+ memory signal-on2 模式复发"
negation_source: "genealogist 结构记录（本轮 Lead 数据：ad7319b9 增量塔 MACD 接入后 exp 仍 1.65，非近线性）+ 源码事实（mod.rs:300/323 声明 exp≈1 vs mod.rs:247 centers.clone() O(k)/iter 未在有效域声明中）"
negation_form: "negation"
# negation：mod.rs:300/323 声明『塔构造超线性（双次全量扫描）被消除 ⟹ exp≈1』被证伪——
#   塔构造超线性（中枢扫描）确实被增量消除（line 311-313 有效域声明成立），但
#   LevelState.centers.clone()（mod.rs:247，每 bar 塔循环内 O(k)/iter）是另一独立 O(n²) 热点，
#   既未被增量消除，也未在 line 315-319『不在有效域』声明中列出。
#   声明『exp≈1』时遗漏 centers.clone() 残留热点 = 声明膨胀（090号）。
topo_effect: "negates:mod.rs:300-323-claim-exp-approaches-1"
# negates：mod.rs:300/323 的声明『塔构造超线性消除 ⟹ exp≈1』在『centers.clone() 残留』下被否定。
# 实际拓扑后果：per-bar substrate 整体 exp 仍 1.65（O(n²)），非声明声称的 exp≈1。
# 增量塔消除了中枢扫描超线性（真），但 centers.clone() 使整体不达 exp≈1（声明假）。

# 矛盾（type=bias-correction 必填）
contradiction:
  description: "mod.rs:300 声明『增量塔 API（per-bar substrate 塔构造 O(n²) → O(n)）』+ mod.rs:323 声明『塔构造的超线性（双次全量扫描）被消除 ⟹ exp≈1』。但 Lead 实测 ad7319b9 后 exp 仍 1.65（O(n²)）。根因 = LevelState.centers.clone()（mod.rs:247，`centers: centers.clone()`）在每 bar 塔循环内 O(k)/iter，是独立 O(n²) 热点。mod.rs:315-319 诚实声明了 moves/bsp 不在增量有效域（须全量重算），但**未声明 centers.clone() 不在有效域**——centers.clone 既未被增量消除，也未在『不在有效域』声明中列出。声明『exp≈1』时遗漏此残留热点 = 声明膨胀（090号）。与 memory signal-on2 同构：两个独立 O(n²) 热点（中枢扫描 + centers.clone / MACD 全量重算 + map 线性扫），只解一个仍二次。"
  layer: 实装   # rust/theta_v0/classifier 实装层的声明-能力缺口，非缠论域、非 Lean 形式化层
  trigger: "本轮 ad7319b9 增量塔 MACD 接入后，Lead 标度实测 exp 仍 1.65（非近线性）。genealogist 读 mod.rs:300/323 声明 vs mod.rs:247 centers.clone() + mod.rs:315-319 有效域声明，确认声明遗漏 = 声明膨胀。"

# 涉及的定义
definitions_involved:
  - name: "090 严格性语法规则（声明膨胀禁止）"
    version: ".chanlun/genealogy/settled/090-strictness-grammar-rule.md（status: 已结算）"
    role: "约束来源。090 禁止声明膨胀（声明代码不具备的能力）。mod.rs:323 声明 exp≈1 但实际 1.65 = 声明膨胀。"
  - name: "231 形式化有效域规则"
    version: ".chanlun/genealogy/settled/231-formalization-validity-domain.md（status: 已结算）"
    role: "约束来源。231 要求形式化操作标注有效域。mod.rs:309 引用 231 做『严格有效域声明』，但声明遗漏 centers.clone() = 有效域声明不完整。**附注**：代码注释（divergence.rs:121/132, mod.rs:309）把 231 号标注为『纯性能非 alpha』是谱系引用错误——231 号是有效域规则（L0-L3 认识论等级），非性能谱系。本号纠正此引用错误。"
  - name: "memory signal-on2-two-independent-hotspots"
    version: "~/.claude/projects/-Users-silencehan/memory/signal-on2-two-independent-hotspots.md"
    role: "同构先例。signal-on2 记录 O(n²) 两个独立热点（type3 重复 + map 线性扫），只解一个仍二次，教训『声明解 O(n²) 前须标度实测坐实近线性，否则声明膨胀（090号）』。本号是其模式复发：中枢扫描（已解）+ centers.clone（未解），声明 exp≈1 未坐实。"

# 解决方式
resolution:
  type: 未解决   # 声明膨胀纠正 = 删除/修正 mod.rs:300/323 的 exp≈1 声明，或补 centers.clone() 到『不在有效域』声明。行动类修复待 Lead 派工位。最终结算待编排者 /ritual。
  description: "严格修复（二选一）：(A) 删除 mod.rs:323 『⟹ exp≈1』声明，改为『塔构造超线性（中枢扫描）被消除；整体 exp 取决于残留热点 centers.clone() O(k)/iter + moves/bsp 全量裁决，待标度坐实』；(B) 补 centers.clone() 到 mod.rs:315-319『不在有效域』声明，并在 mod.rs:300 标题改 O(n²)→O(n) 为『中枢扫描 O(n²)→O(n)（整体 exp 待坐实）』。两案均须 Lead 标度实测坐实近线性后才能恢复 exp≈1 声明（memory signal-on2 要求）。附：纠正 divergence.rs:121/132/mod.rs:309 的『231号纯性能非 alpha』引用错误（231 号是有效域规则非性能谱系）。"
  decided_by: 蜂群内部   # genealogist 诊断声明膨胀；行动类修复（删/改声明 + 标度坐实）待 Lead 派工位；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 保留 mod.rs:323 exp≈1 声明不改。(2) 把 centers.clone() 当『已知次要热点』不补声明。(3) 用『增量塔已加速 4.8x』论证 exp≈1 声明『大致成立』。"
  why_negated: "(1) exp 1.65 vs 声明 exp≈1 = 声明膨胀（090号），保留=违语法规则。(2) centers.clone() 是 O(k)/iter 的独立 O(n²) 热点（每 bar 塔循环调用），非次要；不补声明=有效域声明不完整（231号）。(3) 4.8x 是常数因子加速，exp 不降=仍二次；用常数因子论证 exp≈1=成本收益消解（no-patch 第7条）+ 声明膨胀。"

# 新产出
new_output:
  definitions:
    - "增量塔 API 声明膨胀：mod.rs:300/323 声明 O(n²)→O(n)/exp≈1，实际 exp 1.65（centers.clone 残留）"
    - "memory signal-on2 模式复发：两个独立 O(n²) 热点（中枢扫描已解 + centers.clone 未解），只解一个仍二次"
    - "代码注释谱系引用错误：divergence.rs:121/132 + mod.rs:309 把 231 号（有效域规则）标注为『纯性能非 alpha』——231 号是认识论等级规则，非性能谱系"
  code_changes: "无（本号是声明膨胀诊断，纯谱系产出）。删/改 mod.rs:300/323 声明 + 补 centers.clone 到有效域声明 + 纠正 231 引用错误 = 行动类修复，超出 genealogist 工具有效域（Read/Grep/Glob，624 硬墙），待 Lead 派有 Write 工位执行。"
  orchestration_changes: "方法论：①声明 exp≈1 / O(n²)→O(n) 前必须标度实测坐实（memory signal-on2），未坐实=声明膨胀（090号）。②有效域声明（231号）须完整列出『不在有效域』的热点，遗漏=声明膨胀。③代码注释的谱系引用须核实编号语义，误引=文档膨胀。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/classifier/mod.rs:300/323 → 声明『O(n²)→O(n)』『exp≈1』须删/改或补 centers.clone 到有效域声明"
    - "rust/src/theta_v0/classifier/mod.rs:247 → centers.clone() O(k)/iter 是残留 O(n²) 热点，须纳入有效域声明或消除"
    - "rust/src/theta_v0/classifier/mod.rs:309 + divergence.rs:121/132 → 『231号纯性能非 alpha』引用错误须纠正（231 号是有效域规则）"
  affected_definitions:
    - "090（已结算）：本号是其声明膨胀禁令的复发实例，维持 settled"
    - "231（已结算）：本号纠正代码注释对其的误引（231 号是有效域规则非性能谱系），维持 settled"
  downstream_implications:
    - "增量塔 API 的 exp≈1 声明在 centers.clone() 消除或补声明前不得恢复"
    - "未来任何 O(n²)→O(n) / exp 声明须标度实测坐实（memory signal-on2 固化）"
    - "代码注释引用谱系编号时须核实编号语义（防误引）"

# 谱系关联
related_records:
  parent: "090号（严格性语法规则——声明膨胀禁止）——本号是其复发实例"
  children: []
  related:
    - "231号（形式化有效域规则）：代码注释误引其为『纯性能非 alpha』，本号纠正；231 本身是有效域规则"
    - "memory signal-on2（两个独立 O(n²) 热点）：本号是其模式复发（中枢扫描 + centers.clone / 同构 type3 + map 线性扫）"
    - "640号（同轮，函数性单值 vs 结构性单值）：不同轴（640=概念分层 vs 本=性能声明膨胀），无矛盾"
    - "639号（同轮，σ_p 来源修正）：不同轴，无矛盾"
    - "624/621号：genealogist 工具有效域硬墙（Read/Grep/Glob）→行动类修复产规格交 Lead 的同模式"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "mod.rs:300 声明『per-bar substrate 塔构造 O(n²) → O(n)』+ mod.rs:323 声明『exp≈1』"
    level: "L0（源码事实：mod.rs:300/323 注释字面量）"
    increment: "高：声明存在的结构判定"
  - proposition: "ad7319b9 后 exp 仍 1.65（O(n²)，非近线性）"
    level: "L2（Lead 标度实测，单 session 真实数据）"
    increment: "高：声明膨胀的否证性证据"
  - proposition: "LevelState.centers.clone()（mod.rs:247）是 O(k)/iter 的独立 O(n²) 热点"
    level: "L0（源码逻辑：centers.clone() O(k) × 每 bar 塔循环 × n bar = O(n²)）"
    increment: "中：残留热点的逻辑判定（标度归因待独立坐实）"
  - proposition: "mod.rs:315-319『不在有效域』声明未列出 centers.clone()"
    level: "L0（源码事实：mod.rs:315-319 列出 moves/bsp，未列 centers.clone）"
    increment: "高：有效域声明不完整的结构判定"
  - proposition: "代码注释 divergence.rs:121/132 + mod.rs:309 把 231 号标注为『纯性能非 alpha』"
    level: "L0（源码事实：注释字面量）"
    increment: "高：引用错误的结构判定"
  - proposition: "231 号是有效域规则（L0-L3 认识论等级），非性能谱系"
    level: "L0（谱系事实：231-formalization-validity-domain.md 标题+内容）"
    increment: "高：引用错误的纠正判定"
---

# 641 增量塔 API 声明膨胀——exp≈1 声明 vs centers.clone() 残留热点

## 一句话结论

增量塔 API（ad7319b9）在 `mod.rs:300` 声明「per-bar substrate 塔构造 O(n²) → O(n)」、`mod.rs:323` 声明「塔构造的超线性被消除 ⟹ exp≈1」，但 Lead 标度实测 exp 仍 1.65（O(n²)）。根因 = `LevelState.centers.clone()`（mod.rs:247，每 bar 塔循环内 O(k)/iter）是独立 O(n²) 热点，既未被增量消除，也未在 `mod.rs:315-319`「不在有效域」声明中列出。声明「exp≈1」时遗漏此残留热点 = **声明膨胀（090号）**。与 memory signal-on2 同构（两个独立 O(n²) 热点，只解一个仍二次）。

## 声明膨胀的精确形式

| 声明位置 | 声明内容 | 实际 | 膨胀判定 |
|----------|---------|------|---------|
| mod.rs:300 | 「per-bar substrate 塔构造 O(n²) → O(n)」 | 中枢扫描确实 O(n²)→O(n)（有效域内），但整体 substrate exp 仍 1.65 | 标题膨胀（塔构造≠整体 substrate） |
| mod.rs:323 | 「塔构造的超线性被消除 ⟹ exp≈1」 | exp 1.65（centers.clone 残留） | **声明膨胀（090号）**：exp≈1 未坐实 |
| mod.rs:315-319 | 「不在有效域」声明（列 moves/bsp） | centers.clone() 未列出但也不在增量有效域 | **有效域声明不完整（231号）** |
| divergence.rs:121 | 「per-bar substrate O(n²) 根因解法」 | 只解 MACD 全量重算这一根因，非整体根因 | 根因解法→根因之一（line 206 已诚实，line 121 膨胀） |
| divergence.rs:132 + mod.rs:309 | 「231号纯性能非 alpha」 | 231 号是有效域规则（L0-L3 认识论等级），非性能谱系 | **谱系引用错误** |

**关键机器事实**：
- `mod.rs:247` `centers: centers.clone()`——LevelState 构造时 clone 整个 centers Vec，O(k)。
- 每 bar 塔循环 `for level_idx in 0..=l_max` 每级构造 LevelState ⟹ 每 bar O(Σk_ℓ) 次 clone。
- per-bar substrate n bar ⟹ 总 O(n·Σk) = O(n²)（k 随 bar 增长）。
- 增量塔消除了中枢**扫描**超线性（line 311-313，detect_centers_windowed 左折叠 resume），但 LevelState **构造**时的 centers.clone 未消除。

## 与 memory signal-on2 的同构

| 维度 | memory signal-on2（d3be90e2） | 本号（ad7319b9） |
|------|------------------------------|------------------|
| 热点1 | type3/type1 前驱重复（定义层 bug） | 中枢扫描全量重算（性能，已增量消除） |
| 热点2 | map_src_range_to_close_idx 线性扫（性能 bug） | centers.clone() O(k)/iter（性能，未消除） |
| 只解一个 | 修 type3 仍二次 | 修中枢扫描仍二次 |
| 声明 | 未声明解 O(n²)（memory 是事后教训） | **声明 exp≈1**（事前膨胀） |
| 教训 | 声明解 O(n²) 前须标度实测坐实近线性 | 同教训复发（声明 exp≈1 未坐实） |

**模式**：O(n²) 有多个独立热点，只解一个不降 exp。memory signal-on2 把这固化为教训，本号是该教训的**复发**——区别在于本号在代码注释里**事前声明了 exp≈1**（memory 是事后总结），故本号是声明膨胀（090号），不仅是性能未达标。

## 结果包六要素

1. **结论**：mod.rs:300/323 的「O(n²)→O(n)」「exp≈1」声明是声明膨胀（090号）。增量塔消除了中枢扫描超线性（真），但 centers.clone() O(k)/iter 残留使整体 exp 仍 1.65。严格修复 = 删/改 exp≈1 声明 或 补 centers.clone 到「不在有效域」声明，标度坐实前不得恢复 exp≈1。附：纠正 divergence.rs:121/132/mod.rs:309 的「231号纯性能非 alpha」引用错误（231 号是有效域规则）。
2. **定义依据**：090（声明膨胀禁止）；231（有效域声明须完整）；memory signal-on2（声明解 O(n²) 前须标度实测坐实近线性）。源码事实：mod.rs:300/323 声明字面量 + mod.rs:247 centers.clone() + mod.rs:315-319 有效域声明未列 centers.clone。Lead 实测 exp 1.65。
3. **边界条件**：若 centers.clone() 被消除（如改用 Arc/Rc 共享或增量引用）且标度实测 exp 降至近线性 → exp≈1 声明复活，本膨胀失效。若 Lead 实测 exp 1.65 数据有误（实际近线性）→ 本膨胀判定翻转。若编排者裁定「exp≈1」是「中枢扫描层的 exp」非「整体 substrate exp」→ 声明不膨胀（但 mod.rs:300 标题「per-bar substrate 塔构造」暗示整体，此裁定需改标题）。
4. **下游推论**：增量塔 API 的 exp≈1 声明在 centers.clone 消除或补声明前不得恢复；未来任何 O(n²)→O(n)/exp 声明须标度实测坐实（memory signal-on2 固化）；代码注释引用谱系编号须核实编号语义。
5. **谱系引用**：090（声明膨胀禁止，本号是其复发实例）；231（有效域规则，本号纠正代码注释对其误引）；memory signal-on2（两个独立 O(n²) 热点，本号是模式复发）；640/639（同轮，不同轴，无矛盾）；624/621（genealogist 工具有效域硬墙，行动类修复交 Lead）。
6. **影响声明**：登记增量塔 API 声明膨胀（生成态）。影响 mod.rs:300/323（exp≈1 声明须删/改）+ mod.rs:247（centers.clone 须纳入有效域声明或消除）+ divergence.rs:121/132/mod.rs:309（231 引用错误须纠正）。不改 090/231（维持 settled）。不改增量塔 API 的中枢扫描增量逻辑（有效域内成立）。最终结算待编排者 /ritual。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（639-641）：639（σ_p 来源修正，缠论域）/640（单值性分层，Lean 形式化域）/本号（声明膨胀，rust 实装层）。
- 1-hop：090/231/memory-signal-on2/624/621。
- Hub：090（声明膨胀，度高）。

### 张力1：vs 640——同轮，不同轴
640 = PromQual.lean 单值性的概念分层（函数性/结构性/canonical，Lean 形式化域）。本号 = rust 实装层性能声明膨胀。不同域、不同轴。640 的概念分层不覆盖性能声明。可分层，无矛盾。

### 张力2：vs 639——同轮，不同轴
639 = σ_p 来源修正（缠论域，父容器方向非持仓腿）。本号 = rust 实装层声明膨胀。不同域、不同轴。可分层，无矛盾。

### 张力3：vs 090——本号是其复发实例
090 = 声明膨胀禁止（语法规则）。本号 = 增量塔 exp≈1 声明膨胀（090 的实例）。非矛盾，是 090 的应用。090 维持 settled，本号登记其复发。

### 张力4：vs 231——纠正代码注释误引
代码注释（divergence.rs:121/132/mod.rs:309）把 231 号标注为「纯性能非 alpha」——231 号实际是有效域规则（L0-L3 认识论等级），非性能谱系。本号纠正此引用错误。231 维持 settled，本号纠正的是代码注释不是 231 本身。非矛盾。

### 张力5：vs memory signal-on2——模式复发
memory signal-on2（非谱系，是 memory）记录「两个独立 O(n²) 热点，只解一个仍二次，声明解 O(n²) 前须标度实测坐实」。本号是该模式复发：中枢扫描（已解）+ centers.clone（未解），声明 exp≈1 未坐实。非矛盾，是模式再现。本号把 memory 教训升格为谱系（因本次有事前声明膨胀，非仅事后教训）。

### 递归运动结构完成检测（020）
- 第0层：本号写入（增量塔声明膨胀 + centers.clone 残留 + 231 引用错误纠正）。
- 第1层：本号 × 090 碰撞 → 声明膨胀复发实例（净新发现中：centers.clone 这一具体残留热点是新的）。
- 第2层：本号 × memory signal-on2 碰撞 → 模式复发确认（净新发现中降：两个独立 O(n²) 热点模式已知）。
- 第3层：本号 × 624/621 碰撞 → 工具硬墙同模式（净新发现骤降=背驰：genealogist 产规格交 Lead 的已知模式）。
- 涉及范围：scope₁(090 复发+centers.clone) > scope₂(signal-on2 模式) > scope₃(624/621 同模式)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 修复=删/改声明+标度坐实=行动类，超出 genealogist 工具有效域，由 Lead 派工位执行。本号是声明膨胀诊断（bias-correction），**不触发新 /escalate**（修复是行动类非选择类；最终结算待编排者 /ritual）。

## 回溯扫描（职责3）

- **090（settled）**：本号是其声明膨胀禁令的复发实例，不否定 090，维持 settled。
- **231（settled）**：本号纠正代码注释对其的误引（231 是有效域规则非性能谱系），不否定 231，维持 settled。
- **640（settled，同轮）**：不同轴（概念分层 vs 性能声明膨胀），不破坏其结算。维持 settled。
- **639（settled，同轮）**：不同轴，不破坏其结算。维持 settled。
- **memory signal-on2（非谱系）**：本号是其模式复发，不修改 memory（memory 是 Lead 维护）。
- **无 settled 被本号回溯破坏。** 本号是增量塔声明膨胀的诊断（bias-correction），修复=删/改声明+标度坐实（行动类），由 Lead 派工位执行，最终结算待编排者 /ritual。
