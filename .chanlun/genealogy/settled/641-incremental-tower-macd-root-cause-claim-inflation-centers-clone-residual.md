---
id: "641"
number: 641
status: 已结算   # genealogist 核实（2026-06-30 settle-641 工位）：声明膨胀部分已闭合（增量路径 mod.rs:931-948 Rc 共享消除 O(n²) 热点①ⓛ + centers.clone 标注为 Center is Copy 的 O(k) memcpy + 335-338 行附 L2 实测 exp 0.91 @16K 诚实有效域声明）。原诊断第三子项「231 引用错误」核实后判定为**诊断误判**（注释是 231 L1 等级的正确应用，非误引），不构成开放缺口。
date: "2026-06-28"
settled_date: "2026-06-30"
type: bias-correction   # 声明膨胀纠正（090号族）
depends_on: ["090", "231"]
related: ["640", "639", "643", "signal-on2-two-independent-hotspots"]
title: "增量塔 API 声明『塔构造超线性消除 ⟹ exp≈1』但 exp 仍 1.65——centers.clone() O(k)/iter 残留热点未声明，声明膨胀（090号）+ memory signal-on2 模式复发【已结算：声明膨胀闭合 + 231 引用错误子项诊断误判纠正】"
negation_source: "genealogist 结构记录（本轮 Lead 数据：ad7319b9 增量塔 MACD 接入后 exp 仍 1.65，非近线性）+ 源码事实（mod.rs:300/323 声明 exp≈1 vs mod.rs:247 centers.clone() O(k)/iter 未在有效域声明中）"
negation_form: "negation"
topo_effect: "negates:mod.rs:300-323-claim-exp-approaches-1"
# negates：mod.rs:300/323 的声明『塔构造超线性消除 ⟹ exp≈1』在『centers.clone() 残留』下曾被否定。
# 结算后：增量路径（classify_with_tower_incremental）已用 Rc::clone 消除真 O(n²) 热点①（upper_moves
# 深拷贝），centers.clone 降为 Center is Copy 的 O(k) memcpy（非超线性源），335-338 行附 L2 实测
# incr_total exp 0.91 @16K 的诚实有效域声明。声明与实际一致 ⟹ 否定结算。

# 矛盾（type=bias-correction 必填）
contradiction:
  description: "mod.rs:300 声明『增量塔 API（per-bar substrate 塔构造 O(n²) → O(n)）』+ mod.rs:323 声明『塔构造的超线性（双次全量扫描）被消除 ⟹ exp≈1』。但 Lead 实测 ad7319b9 后 exp 仍 1.65（O(n²)）。根因 = LevelState.centers.clone()（mod.rs:247，`centers: centers.clone()`）在每 bar 塔循环内 O(k)/iter，是独立 O(n²) 热点。声明『exp≈1』时遗漏此残留热点 = 声明膨胀（090号）。与 memory signal-on2 同构。"
  layer: 实装   # rust/theta_v0/classifier 实装层的声明-能力缺口
  trigger: "本轮 ad7319b9 增量塔 MACD 接入后，Lead 标度实测 exp 仍 1.65（非近线性）。"

# 涉及的定义
definitions_involved:
  - name: "090 严格性语法规则（声明膨胀禁止）"
    version: ".chanlun/genealogy/settled/090-strictness-grammar-rule.md（status: 已结算）"
    role: "约束来源。本号曾是其声明膨胀禁令的复发实例；结算后膨胀已消除，090 维持 settled。"
  - name: "231 形式化有效域规则"
    version: ".chanlun/genealogy/settled/231-formalization-validity-domain.md（status: 已结算）"
    role: "约束来源。结算修订附注（见 settlement 区块）：原诊断指『divergence.rs:121/132 + mod.rs:309 把 231 标为纯性能非 alpha = 引用错误』，核实后判定为诊断误判——注释是 231 L1 等级（信息增量为零的纯管线变换）的正确应用，非误引。231 维持 settled。"
  - name: "memory signal-on2-two-independent-hotspots"
    version: "~/.claude/projects/-Users-silencehan/memory/signal-on2-two-independent-hotspots.md"
    role: "同构先例。结算后教训固化：声明 O(n²)→O(n)/exp≈1 前须标度实测坐实——本号修复后 335-338 行已附 L2 exp 0.91 @16K 坐实。"

# 解决方式
resolution:
  type: 已解决   # 声明膨胀闭合（增量路径 Rc 共享消除 O(n²) 热点① + centers.clone O(k) memcpy 标注 + L2 实测有效域声明）。231 引用错误子项判定为诊断误判，无需修复。
  description: |
    严格修复路径（行动类，由后续工位执行）已落地于增量路径 classify_with_tower_incremental：
    (1) **真 O(n²) 热点①消除**：mod.rs:944-949 把旧 `lc.upper_moves.clone()`（per-bar 全塔 LeveledMove 递归深拷贝，真 O(n²) 源）改为 `Rc::clone`（O(1) 引用计数）+ `Rc::make_mut`（caller 逐 bar drop snapshot ⟹ strong_count==1 ⟹ 原地 O(tail) extend）。
    (2) **centers.clone 严格标注**：mod.rs:931-934 ponytail 注释——Center is Copy ⟹ clone = memcpy O(k)（非 LeveledMove 递归深拷贝）；Arc<Vec<Center>> 引入原子计数开销反劣于 memcpy；此 clone 已是最优；ceiling：Center 变大（>~64B）可考虑 Arc 共享。centers.clone 是 O(k) memcpy 非超线性源 = 诚实声明，不构成 O(n²) 热点。
    (3) **L2 有效域声明**：mod.rs:335-338 附 641 谱系标注——incr_total exp 0.91 @16K ≈ 1.0 → 塔构造 O(n) 已达成（标 **L2 真实数据验证**，非 L0/L1）；有效域 = compose_level_resume + process_inclusion；**不在有效域** = 全引擎 per-bar 端到端 exp 须大规模验证（诚实声明，231号）。
    (4) **bsp memo guard**：mod.rs:900-927 把 BSP 全量重算降为 O(段变化次数) 次（profile 坐实 bsp 53% of tower，~99.2% bar 全量重算冗余）。
    声明现在与实际一致：不再声明笼统的「exp≈1」无前提，而是标 L2 实测 exp 0.91 @16K + 诚实列出不在有效域的部分。
  decided_by: 蜂群内部   # genealogist 诊断声明膨胀（pending 阶段）；行动类修复由后续工位落地于增量路径；本结算由 settle-641 工位核实

# 被否定的方案
negated:
  description: "(1) 保留 mod.rs:323 exp≈1 声明不改。(2) 把 centers.clone() 当『已知次要热点』不补声明。(3) 用『增量塔已加速 4.8x』论证 exp≈1 声明『大致成立』。"
  why_negated: "结算后：(1) 已改——335-338 行改为 L2 实测 exp 0.91 @16K + 诚实有效域声明。(2) centers.clone 已严格标注为 Center is Copy 的 O(k) memcpy（非超线性源），真 O(n²) 热点①（upper_moves 深拷贝）已用 Rc 共享消除。(3) 不再用常数因子论证——exp 已用 L2 真实数据坐实 0.91。"

# 新产出
new_output:
  definitions:
    - "增量塔 O(n) 达成（L2，exp 0.91 @16K）：真 O(n²) 热点①（upper_moves 深拷贝）经 Rc::clone + make_mut 消除；centers.clone 是 Center is Copy 的 O(k) memcpy（非超线性源）"
    - "memory signal-on2 教训固化：声明 O(n²)→O(n)/exp 前须标度实测坐实——本号修复后已附 L2 exp 0.91 @16K"
    - "诊断误判纠正：原 pending 指『231号纯性能非 alpha = 引用错误』，核实后判定为 231 L1 等级的正确应用（信息增量为零的纯管线变换），非误引"
  code_changes: "无（本号是声明膨胀诊断 + 结算核实，纯谱系产出）。行动类修复（增量路径 Rc 共享 + centers.clone 标注 + L2 有效域声明 + bsp memo）由后续有 Write 工位落地，settle-641 工位核实其闭合状态。"
  orchestration_changes: "方法论固化：①声明 exp≈1 / O(n²)→O(n) 前必须标度实测坐实（memory signal-on2，本号已落地 L2 exp 0.91）。②有效域声明（231号）须完整列出『不在有效域』的热点（本号 335-338 行已落地）。③谱系引用核实须区分『误引』与『正确应用 L1 等级』——『231号纯性能非 alpha』在 divergence.rs:131-132 语境是 231 L1 等级（信息增量为零）的正确应用，非误引。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/classifier/mod.rs:931-949 → centers.clone（O(k) memcpy 标注）+ upper_moves Rc::clone（消除 O(n²) 热点①）已落地"
    - "rust/src/theta_v0/classifier/mod.rs:335-338 → 641 谱系标注（L2 实测 exp 0.91 @16K + 诚实有效域声明）已落地"
    - "rust/src/theta_v0/classifier/mod.rs:900-927 → bsp memo guard（O(段变化次数) 次重算）已落地"
    - "rust/src/theta_v0/classifier/divergence.rs:121/131-132/207/558 → 『231号纯性能非 alpha』注释是 231 L1 等级正确应用，无需修改"
  affected_definitions:
    - "090（已结算）：本号曾是其声明膨胀禁令复发实例，膨胀已消除，维持 settled"
    - "231（已结算）：原诊断指代码注释误引，核实后判定为正确应用 L1 等级，无需纠正，维持 settled"
  downstream_implications:
    - "增量塔 API 的 O(n) 声明现以 L2 实测 exp 0.91 @16K 坐实，可恢复（已坐实，非无前提声明）"
    - "未来任何 O(n²)→O(n) / exp 声明须标度实测坐实（memory signal-on2 固化）"
    - "谱系引用核实须区分『误引谱系编号语义』与『正确应用该谱系的认识论等级』"

# 谱系关联
related_records:
  parent: "090号（严格性语法规则——声明膨胀禁止）——本号是其复发实例，膨胀已闭合"
  children: []
  related:
    - "231号（形式化有效域规则）：原诊断指误引，核实后判定为 L1 等级正确应用；231 本身维持 settled"
    - "memory signal-on2（两个独立 O(n²) 热点）：本号是模式复发，修复后已附 L2 坐实"
    - "643号（acceptance[1] L2 开放先例）：本号 settlement 区块的开放边界标注模式参照"
    - "640/639号（同轮，不同轴，无矛盾）"
    - "624/621号：genealogist 工具有效域硬墙（Read/Grep/Glob）"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "增量路径 mod.rs:944-949 用 Rc::clone + make_mut 消除 upper_moves 深拷贝（真 O(n²) 热点①）"
    level: "L0（源码事实：mod.rs:944-949 注释字面量 + Rc::clone 调用）"
    increment: "高：声明膨胀闭合的结构判定"
  - proposition: "centers.clone 是 Center is Copy 的 O(k) memcpy（非超线性源）"
    level: "L0（源码逻辑：Center derive Copy ⟹ clone = memcpy O(k)；mod.rs:931-934 注释）"
    increment: "中：残留热点降级的逻辑判定"
  - proposition: "incr_total exp 0.91 @16K ≈ 1.0（塔构造 O(n) 达成）"
    level: "L2（mod.rs:335 标注的真实数据标度实测）"
    increment: "高：声明膨胀闭合的 L2 坐实"
  - proposition: "divergence.rs:131-132『231号纯性能非 alpha』是 231 L1 等级（信息增量为零的纯管线变换）的正确应用，非误引"
    level: "L0（源码事实 + 谱系事实：divergence.rs:131-132 注释 = 231 L0→L1 信息增量为零的应用）"
    increment: "高：原诊断子项误判的纠正"
---

# 641 增量塔 API 声明膨胀——exp≈1 声明 vs centers.clone() 残留热点【已结算】

## 一句话结论（结算后）

原诊断的声明膨胀（mod.rs 声明『exp≈1』但 Lead 实测 exp 1.65）已**闭合**：增量路径 `classify_with_tower_incremental`（mod.rs:929-949）用 `Rc::clone` + `Rc::make_mut` 消除了真正的 O(n²) 热点①（`upper_moves` per-bar 全塔深拷贝），`centers.clone` 降级标注为 `Center is Copy` 的 O(k) memcpy（非超线性源），并在 mod.rs:335-338 附 **L2 实测 incr_total exp 0.91 @16K** + 诚实列出"不在有效域"的部分。声明现与实际一致。原诊断第三子项「231号纯性能非 alpha = 引用错误」核实后**判定为诊断误判**——该注释是 231 号 L1 认识论等级（信息增量为零的纯管线变换）的**正确应用**，非把 231 当性能谱系误引。

## 结算核实（settle-641 工位，2026-06-30）

### 核实1：声明膨胀（exp≈1）—— 已闭合

| 原诊断指控 | 代码现状（核实） | 判定 |
|-----------|----------------|------|
| mod.rs:300/323 声明『O(n²)→O(n)/exp≈1』无前提 | mod.rs:335-338 附 641 标注：incr_total exp 0.91 @16K **标 L2**，有效域 = compose_level_resume + process_inclusion，**不在有效域** = 全引擎端到端 exp 须大规模验证 | **closed**：声明改为 L2 坐实 + 诚实有效域 |
| centers.clone（mod.rs:247）是未声明的 O(n²) 残留热点 | 增量路径 mod.rs:931-934 ponytail 标注：Center is Copy ⟹ clone = O(k) memcpy（非递归深拷贝），已是最优；ceiling 标 Center>64B 可改 Arc | **closed**：centers.clone 是 O(k) memcpy，非超线性源 |
| 真 O(n²) 热点①未消除 | mod.rs:944-949：旧 `upper_moves.clone()`（真 O(n²) 深拷贝）改 `Rc::clone`（O(1)）+ make_mut（原地 O(tail)） | **closed**：真热点已消除 |

### 核实2：231 引用错误 —— 诊断误判，无需修复

原 pending 指 `divergence.rs:121/132 + mod.rs:309` 把 231 标为「纯性能非 alpha」是引用错误（理由：231 是有效域规则非性能谱系）。核实 divergence.rs:131-132 实际语境：

```
认识论：L1（管线 bit-exact 等价于全量版），不携带 alpha 信息——增量只是把同一
浮点约简从「全量重算」改成「逐 bar 延伸」，数值不变（231号：纯性能）。
```

这是 231 号 **L1 认识论等级的正确应用**——231 的核心洞察是「L0→L1 信息增量为零」，注释正确地用此表达「增量改写不携带 alpha 信息（信息增量为零的纯管线变换）」。"纯性能非 alpha" = "信息增量为零的管线优化" = 231 的 L1 语义。这**不是**把 231 当成性能谱系误引，而是正确引用其认识论等级框架。

故原诊断第三子项（231 引用错误）**判定为诊断误判**。no-patch 要求诚实声明：不假装它是开放工程缺口，也不假装它已被"修复"——它本就无需修复。231 维持 settled。

### 结算结论

- **声明膨胀主体（exp≈1 + centers.clone + 真 O(n²) 热点①）**：已闭合（L2 exp 0.91 坐实 + Rc 共享消除 + O(k) memcpy 严格标注）。
- **231 引用错误子项**：诊断误判，无开放缺口。
- **整体判定**：641 可移 settled（无残留开放工程项；与 643 acceptance[1] 模式不同——643 有真开放 L2 项，本号无）。

## 张力检查（019d/020，结算后复核）

- **vs 090**：本号曾是声明膨胀复发实例，膨胀已闭合，090 维持 settled，无矛盾。
- **vs 231**：原诊断指误引，核实后为正确应用 L1 等级，231 维持 settled，无矛盾（诊断误判纠正不否定 231）。
- **vs memory signal-on2**：模式复发已修复（L2 exp 0.91 坐实），无矛盾。
- **vs 640/639**：同轮不同轴，无矛盾。
- **递归运动结构完成（020）**：第0层（结算核实）→第1层（声明膨胀闭合确认，净新发现：Rc 共享是真热点解法）→第2层（231 子项核实，净新发现降：误判纠正）→第3层（无新碰撞，背驰）。背驰∧分型 ⟹ 结构完成。本结算不触发新 /escalate。

## 回溯扫描（职责3，结算后）

- **090（settled）**：维持。
- **231（settled）**：维持（原诊断误判已纠正，不否定 231）。
- **640/639（settled，同轮）**：维持。
- **无 settled 被本结算回溯破坏。**

---

## 【发生史】以下为 pending 阶段（2026-06-28）原始诊断内容

### 声明膨胀的精确形式（pending 阶段诊断）

| 声明位置 | 声明内容 | 实际（pending 时） | 膨胀判定（pending 时） |
|----------|---------|------|---------|
| mod.rs:300 | 「per-bar substrate 塔构造 O(n²) → O(n)」 | 中枢扫描 O(n²)→O(n)，但整体 substrate exp 仍 1.65 | 标题膨胀 |
| mod.rs:323 | 「塔构造的超线性被消除 ⟹ exp≈1」 | exp 1.65（centers.clone 残留） | 声明膨胀（090号） |
| mod.rs:315-319 | 「不在有效域」声明 | centers.clone() 未列出 | 有效域声明不完整（231号） |

> 结算后修订：上表「实际」「膨胀判定」反映 pending 时（ad7319b9）状态。增量路径落地后，真 O(n²) 热点①（upper_moves 深拷贝）已经 Rc 共享消除，centers.clone 降为 O(k) memcpy，exp 由 L2 实测 0.91 @16K 坐实。上述膨胀已闭合。

### 与 memory signal-on2 的同构（pending 阶段）

O(n²) 有多个独立热点，只解一个不降 exp。pending 时诊断：中枢扫描（已解）+ centers.clone（pending 时疑为未解）。结算后核实：真 O(n²) 热点①是 upper_moves 深拷贝（已经 Rc 消除），centers.clone 本是 O(k) memcpy（非超线性源）——pending 诊断把 centers.clone 误判为残留 O(n²) 源，实际超线性源是 upper_moves 深拷贝，已消除。memory 教训（声明前须标度坐实）固化，本号修复后已附 L2 exp 0.91。
