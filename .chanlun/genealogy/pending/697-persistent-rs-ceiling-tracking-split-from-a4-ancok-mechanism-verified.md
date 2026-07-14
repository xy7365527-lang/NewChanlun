---
id: "697"
number: 697
status: 生成态
date: "2026-07-04"
type: bias-correction
title: "persistent.rs 自认『最小修复』的彻底修复 ceiling（增量 extract_elements/confirmed prefix immutable 元素层，L2/L3 待验证）须独立跟踪——codex-gap2rulings 条目6 建议『另开条目』核实未开，本号开条目，不再挂 A4 名下"
source: "[新缠论] .chanlun/review-results/codex-gap2rulings-20260703.md 条目6（v1 A4 AncOK 放宽，裁定不成立/建议销项）；.chanlun/review-results/formal-chain-deepresearch-20260704.md §3.1 争议二（问题E，机制证伪成立/ceiling缺口幸存）"
negation_source: heterogeneous   # A4机制证伪=codex独立核实（gap2rulings条目6）；ceiling缺口幸存=persistent.rs模块自陈（一手代码证据）+ 深度研究报告对该自陈的二次确认
negation_model: "codex (gap2rulings 条目6 decide, sandbox read-only)"
negation_form: separation   # A4「AncOK stale伪造parent=None」这个（旧）单一缺口标签，被拆分为两个不同层面的对象：(1)祖先链查找机制本身（已修复，销项）；(2)persistent.rs整体实装的完成度上限（自陈ceiling，独立存活）
topo_effect: |
  split — A4 缺口节点分裂为两个独立对象：
  (1) 【已销项，不再是缺口】祖先链查找机制（"index结构映射版 ancestor_close_by_id" vs "值比较版
      ancestor_close"）——codex 独立核实生产路径唯一使用 ancestor_close_by_id，Stale 分支
      （coverage.rs:1757-1820）查 registry.held_state(leg) 区分真实状态，非边界根不会伪造
      parent_id:=None，只有真正边界根才产生 None。这是"真实结构下的合法结果"，非索引错位伪造。
  (2) 【本号新开，独立跟踪】persistent.rs 自身的实装完成度 ceiling——模块自称"最小修复"，"彻底修复"
      （增量 extract_elements / confirmed prefix immutable 元素层）标为待办，且 L2/L3（真实数据/交叉
      验证）待完成。这个缺口与(1)的机制正确性无关——即便祖先链查找完全正确，persistent.rs 的覆盖
      完整度仍可能不足（自陈的 ceiling 就是承认这一点）。
  target=A4缺口标签；scope=local（仅 persistent.rs 模块自身，不影响已销项的祖先链机制判断，不影响
  其消费方 coverage.rs 的 Stale 分支正确性判断）。

# 矛盾（bias-correction 的被订正对象——挂错名下）
contradiction:
  description: |
    gap-master2 终稿把 A4（AncOK放宽疑似缺口）判定为"部分缓解在案"，将"祖先链查找用值比较还是ID
    比较"（v1旧问题）与"persistent.rs是否彻底修复"（模块自陈ceiling）混为一谈，挂在同一个A4名下。
    codex独立核实（gap2rulings条目6）证实祖先链查找机制本身已完全修复（生产路径唯一用ID版，Stale
    分支不伪造parent=None）——这部分应销项。但codex裁定文本本身指出："persistent.rs 自身是否还有
    独立缺口（其模块头自称'最小修复'，'彻底修复'标为ceiling）应另开条目跟踪，不应挂在A4名下"——
    即codex已经建议做本号现在做的拆分动作，但建议是否已被执行（是否已开新条目）此前未经核实。
  layer: 概念   # 缺口对象归属层——"机制bug"与"实装完成度ceiling"是两个不同性质的对象，前者是错误，
                # 后者是诚实自陈的未完成范围（同231号"诚实None不伪造"先例：自陈ceiling不是bug）
  trigger: "formal-chain-deepresearch-20260704.md §3.1 争议二第二轮对抗反驳——'A4未闭合时声部树完备性
    非完全证明态'这一承重结论为真（persistent.rs自认最小修复/彻底修复标ceiling，L2/L3待验证），且
    codex'应另开条目跟踪'是建议动作，未验证是否已开。本工位（genealogist）核实：搜索
    .chanlun/genealogy/pending/ 与 settled/ 全库，未找到任何以persistent.rs ceiling/彻底修复为对象的
    独立谱系条目——建议未被执行，本号补开。"

# 涉及的定义
definitions_involved:
  - name: "A4：AncOK stale 伪造 parent=None（原判定对象）"
    version: "gap-master2终稿存疑交裁清单条目6；codex-gap2rulings-20260703.md条目6"
    role: "被证伪的旧判定——codex独立核实coverage.rs:1757-1820 Stale分支确实检查persistent registry
      held_state，非边界的Closed/Invalidated会被prune，只有真正边界根才产生parent_id:None。此判定
      本身已正确销项，本号不重启此争议，只处理codex裁定文本中附带的'另开条目'建议。"
  - name: "persistent.rs 模块自陈状态"
    version: "HEAD（分支gap3-rework-codex9-fix），persistent.rs模块头注释"
    role: "承重对象。模块自称当前实装为'最小修复'，'彻底修复'（增量extract_elements机制/confirmed
      prefix immutable元素层的完整实装）被模块自身标注为ceiling（上限/待办），且L2（真实数据）/L3
      （交叉验证）验证状态未完成。这是一个诚实的自陈缺口，不是bug，也不是A4机制问题的延续。"
  - name: "四态overlay（LivePresent/LiveDetached/Closed/Invalidated）"
    version: "同691号谱系引用，persistent.rs四态分派实装"
    role: "解决的是元素跨bar生命周期持久性问题——与'祖先链查找用值比较还是ID比较'是两个不同层面
      （codex裁定原文明确区分），本号的ceiling对象是四态overlay自身完整度，非祖先链查找正确性。"

# 解决方式
resolution:
  type: 未解决   # 本号是"核实追踪条目是否存在"的行动类结论（不存在）+"是否列入本轮彻底修复"的选择类标注
  description: |
    行动类部分（本工位已执行）：核实全库 .chanlun/genealogy/pending/ + settled/，未发现任何以
    persistent.rs彻底修复/ceiling为直接对象的独立谱系条目——codex"应另开条目跟踪"的建议此前未被执行。
    本号即为该条目，完成"核实+补开"的行动类工作。
    选择类部分（不由本工位裁定）：persistent.rs的彻底修复（增量extract_elements/confirmed prefix
    immutable元素层的完整实装，含L2/L3验证）是否列入本轮工作范围，属选择类——需与其他剩余工作
    （路线A四项/路线B/路线C）做优先级/依赖序判断，按275号局部依赖原则由Lead按依赖序调度，非
    genealogist裁定"要不要修"。
  decided_by: "行动类部分=本工位（genealogist）核实完成；选择类部分=待Lead/编排者按依赖序调度"

# 被否定的方案
negated:
  description: "gap-master2终稿把'祖先链查找机制问题'与'persistent.rs彻底修复ceiling'混为一谈，挂在
    同一个A4标签下——即认为这两者是同一个缺口的两个方面。"
  why_negated: |
    codex独立核实证明两者是不同层面的问题（裁定原文："persistent.rs的四态overlay解决的是元素跨bar
    生命周期持久性问题，与'祖先链查找用值比较还是ID比较'是两个不同层面的问题——gap-master2终稿把
    两者混为一谈是误读"）：
    (1) 祖先链查找机制——已被证明正确（生产路径唯一用ID版，Stale分支正确区分状态），这是一个
        关于"代码逻辑对不对"的机制问题，答案已确定：对。
    (2) persistent.rs整体实装完成度——是一个关于"覆盖范围够不够"的范围问题，答案是模块自己给出的
        诚实声明："最小修复"，尚有ceiling未做。这与(1)的对错无关——即使(1)完全正确，(2)仍可能
        存在（诚实自陈的未完成 ≠ (1)的bug外溢）。
    把(2)挂在(1)（A4）名下，会导致(1)销项时(2)被连带误销项（"A4已证伪⟹persistent.rs无问题"的
    错误推论）——这正是深度研究报告§3.1争议二警告的风险。两者必须分离跟踪。
  layer_gap_type: "机制正确性（已确定=对）⊥ 实装完成度范围（自陈未完成=ceiling）——两个独立轴被
    合并成一个是非判断"

# 新产出
new_output:
  definitions:
    - "A4机制销项确认：祖先链查找（ancestor_close_by_id生产路径 + Stale分支held_state检查）已被
      codex独立核实为正确实装，非索引错位伪造——此争议正式关闭，不再是待裁缺口。"
    - "persistent.rs ceiling独立跟踪对象：彻底修复（增量extract_elements机制 + confirmed prefix
      immutable元素层的完整实装）+ L2/L3验证，是与A4无关的独立待办项，须以本号（697）为跟踪锚点，
      不得因A4销项而被连带视为已解决。"
  code_changes: "无——本号是谱系层面的对象分离记录，不触发代码改动。彻底修复本身的实装授权与调度
    属Lead/编排者，本号只确保该缺口不因A4销项而'被消失'。"
  orchestration_changes: "无。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/.../persistent.rs——本号锚定其自陈ceiling为独立跟踪对象，不改动代码。"
    - "coverage.rs:1757-1820（Stale分支）——本号确认其正确性判断（A4销项部分）保持有效，不受
      本号影响。"
  affected_definitions:
    - "A4（原判定对象）：正式销项，机制问题已确定为对。"
    - "gap-master2终稿条目6：本号是其'部分缓解在案'表述的订正——应拆分为'机制已确认正确'+
      'persistent.rs ceiling独立存活'两条。"
  downstream_implications:
    - "任何后续报告若引用'A4已销项'来论证'persistent.rs无遗留工作'，属于本号明确否定的推论——
      须回溯本号：A4销项只关闭机制正确性争议，不关闭ceiling跟踪。"
    - "persistent.rs彻底修复的调度（是否列入本轮/路线A/B/C）留给Lead按275号局部依赖原则判断——
      本号不预判其优先级，只确保其可见性不丢失。"

# 回溯结算
retroactive_settlement:
  settled_by: null
  settlement_date: null
  settlement_description: null
  # L2 验证降级注记（a5 收口，2026-07-04）——见文末「L2 验证注记」小节。
  # ceiling 在 BTC/L2 暴露面=0，从「L2/L3 待验证」降级为「已验证休眠缺口」，
  # 但彻底修复仍未实装，不 settled（休眠≠消除）。证据：ancok-a5-20260704.md。
  l2_verification: "ancok-a5-20260704.md（BTC 4,613,599 bar 全历史 L2，restore_break_registry_lost=0，恢复成功率=1.000000，暴露面=0）"
  l2_status: "已验证休眠缺口（BTC/L2）"
  # M0-M8 终态注记（2026-07-04）——见文末「M0-M8 waiver 正式化终态注记」小节。
  # a5 的「已验证休眠缺口」经 TARGET_STRATEGY_MAXFULL.md §4 提升为策略文档级 signal-alpha waiver，
  # 四项翻转条件写入 §4.2。waiver ≠ settled（scoped 到 signal-alpha 域，exec-full 不 waiver）。
  waiver_status: "signal-alpha WAIVED（TARGET_STRATEGY_MAXFULL.md §4，scoped，四项翻转条件保留）"

# 谱系关联
related_records:
  parent: "691（codex-f2单实例YAGNI被推翻——同为persistent/账本层生命周期对象分离系列，同批codex
    a-系列裁定的姊妹条目）"
  children: []
depends_on: ["231", "090", "652", "275"]
related: ["691", "693"]
---

# bias-correction 697：A4 机制销项 ⊥ persistent.rs ceiling 独立跟踪——codex 建议的『另开条目』核实未开，本号补开

## 结论

`codex-gap2rulings-20260703.md` 条目6 已把 A4（AncOK stale 伪造 parent=None）的判定拆分为两层：
**(1) 祖先链查找机制**——codex 独立核实生产路径唯一使用 ID 结构映射版 `ancestor_close_by_id`，
`coverage.rs:1757-1820` Stale 分支正确检查 `registry.held_state(leg)`，非边界的
Closed/Invalidated 会被 prune，只有真正边界根才产生 `parent_id: None`——**这不是索引错位伪造，
是真实结构下的合法结果**。该争议**正式销项**。

但 codex 裁定原文同时指出：**"`persistent.rs` 自身是否还有独立缺口（其模块头自称'最小修复'，
'彻底修复'标为 ceiling）应另开条目跟踪，不应挂在 A4 名下"**——这是一个**建议动作**（未来动作），
是否已被执行此前未经验证。

本工位核实：搜索 `.chanlun/genealogy/pending/` 与 `settled/` 全库（含关键词 `persistent.rs`、
`ceiling`、`A4`），**未发现任何以 persistent.rs 彻底修复/ceiling 为直接对象的独立谱系条目**——
codex 的建议**尚未被执行**。本号（697）即为该补开的独立跟踪条目。

## 定义依据

- `codex-gap2rulings-20260703.md` 条目6（第163-184行）：A4 判定为"不成立，建议销项"，裁定原文
  明确区分"祖先链查找"（已修复）与"persistent.rs 自身独立缺口"（另开条目跟踪，不挂 A4 名下）。
- `formal-chain-deepresearch-20260704.md` §3.1 争议二（问题E）：第二轮对抗反驳确认"A4 未闭合时
  声部树完备性非完全证明态"这一承重结论为真，persistent.rs 自认最小修复、彻底修复标 ceiling、
  L2/L3 待验证。上浮问题明确分两半："核实 ceiling 跟踪条目是否存在，不存在则开条目"（行动类，
  本号执行）+ "彻底修复是否列入本轮"（选择类，待裁）。
- 本工位对 `.chanlun/genealogy/pending/` 和 `settled/` 的全库检索：无匹配文件，确认建议未执行。

## 边界条件（结论翻转）

- 若后续发现某个既有谱系条目（本工位检索遗漏）已经以其他命名跟踪 persistent.rs 的彻底修复
  ceiling ⟹ 本号与该条目合并，本号降级为"重复登记，已并入 XXX 号"。
- 若彻底修复（增量 extract_elements + confirmed prefix immutable 元素层）被实装且 L2/L3 验证
  通过 ⟹ 本号回溯结算为 settled，记录实装 commit。
- 若编排者裁定彻底修复超出本轮路线A/B/C范围、永久搁置 ⟹ 本号标注 WAIVED（同693号A10/A11
  硬裁决先例），保留可见性但不阻塞其他工作。

## 下游推论

- **禁止的错误推论**：A4 机制销项 ⟹ persistent.rs 无遗留工作。本号明确否定此推论——机制正确性
  与实装完成度是两个独立轴，前者对不影响后者的完成状态。
- persistent.rs 彻底修复的调度权在 Lead——按275号局部依赖原则，若其他工作不消费该 ceiling 的
  输出，不应因该 ceiling 存在而被判定"阻塞"；反之若某工作确实依赖增量 extract_elements 的完整
  实装，则须先完成本号追踪的 ceiling。

## 谱系引用

- `codex-gap2rulings-20260703.md` 条目6：本号的直接来源，执行其"另开条目"建议。
- `691`：同批 codex a-系列裁定姊妹条目，同为持久化/账本层生命周期对象分离系列。
- `231`（形式化有效域）：诚实 None/诚实 ceiling 不是 bug 的先例——persistent.rs 自陈 ceiling 是
  诚实声明，不因此被误判为缺陷。
- `090`（声明膨胀禁止）：反向应用——本号防止的是"声明膨胀的反面"，即因 A4 销项而把 persistent.rs
  的真实未完成范围错误地"声明为已完成"。
- `652`/`275`（局部依赖原则）：彻底修复是否阻塞其他工作，由局部依赖关系判断，非全局排序。

## 影响声明

纯谱系记录，**零代码改动**。行动类工作已完成：核实全库确认 codex 条目6 的"另开条目跟踪"建议
此前未被执行，本号补开。选择类工作（是否列入本轮彻底修复）标注但不裁定，留给 Lead/编排者按
依赖序调度。本号不影响 A4 机制销项的有效性判断，也不影响 `persistent.rs`/`coverage.rs` 现有
代码的正确性判断——纯粹是对象跟踪层面的分离登记。

---

## L2 验证注记（a5 收口，2026-07-04）

**追加人**：genealogist（本工位）。**依据**：`.chanlun/review-results/ancok-a5-20260704.md`
（工位 swarm/ws-ancok2，L2，路径2=暴露面0）。**本小节不改上文既有内容，仅追加 L2 结果。**

### 降级判定

697 号 ceiling 从「**L2/L3 待验证**」降级为「**已验证休眠缺口（BTC/L2）**」。

上文「边界条件」第二条（"彻底修复被实装且 L2/L3 验证通过 ⟹ 回溯结算 settled"）与本次结果**不冲突**：
a5 走的是**另一条**降级路径——不是彻底修复被实装，而是 codex 建议6 三选一中的**第三项「证明 parent
可恢复」在 BTC 上经验成立**（暴露面测量而非严格修复）。故本号**不 settled**（彻底修复仍未实装），
只从「待验证 ceiling」降级为「休眠缺口」——**休眠 ≠ 消除**，机制在 BTC 上未被触发，但仍在。

### L2 测量证据（BTC 4,613,599 bar 全历史）

| 探针 | 定义 | 实测值 |
|------|------|--------|
| `stale_arm` | HeldLegMatch::Stale 命中 | 82,127 |
| `state_live_present` | Stale∧registry LivePresent（理论不可达） | 15 |
| `state_live_detached` | Stale∧registry LiveDetached（触发 restore） | 82,112 |
| `closed_inval_boundary_kept` | Stale∧Closed/Inval∧真边界根（合法 None） | 0 |
| `restore_calls` | 祖先链恢复调用总数 | 42,823 |
| **`restore_break_registry_lost`** | **★暴露面：registry 丢失/作废祖先中断** | **0** |

**恢复成功率 = 42,823 / 42,823 = 1.000000**；registry 丢失暴露面 = **0**。记账封闭性两个自检
assert 通过（15+82112+0+0=82127；2962+39861+0=42823）。运行于生产 π ledger 路径
（`run_theta_v0_pi`，非 cached 旁路），`is_l2=true`，用时 3120.32s。

### 有效域声明（formalization-validity-domain 规则，231号）

**有效域 = BTC 单标的 / L2**，**不声明 L3**。暴露面=0 是 BTC 全历史结果；其他标的（CL/ES/金油）
若 LiveDetached 分布或 registry 作废时序不同，`restore_break_registry_lost` 可能转正 ⟹ 结论翻转为
路径1（实装严格修复）。定义域=全标的，有效域=BTC——**不得膨胀**。

### 对承重结论的收窄

深度研究报告 §3.1 争议二承重结论「A4 未闭合时声部树完备性**非完全证明态**」——在 BTC/L2 有效域内
从「未证明」**收窄为「BTC 上未证伪」**（声部树严格性经验成立，暴露面=0）。但这是 L2 经验结论，
**非 L0 完全证明**；完全证明态仍需彻底修复的 L0 论证或 L3 交叉验证。争议二不因此关闭，仅收窄。

### 探针常驻要求

休眠缺口的可观测性依赖 coverage.rs 的 `ancok_probe_*` 探针常驻（a5 报告边界条件4）。若探针被删，
697 回退为不可验证 ceiling。探针为此常驻（报告称已加 ponytail 注释锚定）。

### 前任状态订正

上文 front matter `negation_source` 曾记 "ceiling缺口幸存=persistent.rs模块自陈+深度研究报告二次
确认"——该表述仍成立（ceiling 客观存在），本小节仅补充：该 ceiling 在 **BTC/L2 上暴露面=0**，
故降级为休眠。A4 机制销项判断**不受影响**（a5 报告佐证：`closed_inval_boundary_kept=0` 且
`restore_break_registry_lost=0` 证明 Stale 分支未伪造 parent=None，697 上文 A4 销项部分保持有效）。

### 新观测（非本号裁定对象，仅记录）

`state_live_present=15`（理论标注「不可达」，实测低频命中 15/82127=0.018%）——「理论不可达分支实际
低频可达」的观测，按持久身份保留（I1）处理，不触发 restore，不产生暴露面。记录待后续工位处置，
非本号裁定对象。

---

## M0-M8 waiver 正式化终态注记（2026-07-04，a5 休眠缺口经策略文档 §4 提升为 signal-alpha waiver）

**追加人**：genealogist（本工位）。**依据**：`TARGET_STRATEGY_MAXFULL.md` §4「AncOK ceiling
waiver 声明」（PDF p4/p15 原文措辞）+ §4.1 数据依据 + §4.2 四项翻转条件。**本小节不改上文既有内容，
仅记录 a5 结果从 review-results 报告级提升到策略文档 §4 正式 waiver 级的状态变化与结算判定。**

### waiver 从「报告级休眠缺口」提升为「策略文档级正式声明」

上文 L2 验证注记把本号从「L2/L3 待验证」降级为「已验证休眠缺口（BTC/L2）」，依据是 review-results
层的 a5 报告。M0-M8 主线闭合时，该结论被 `TARGET_STRATEGY_MAXFULL.md` §4 **正式化**为策略文档级
的 signal-alpha waiver：

| 层级 | 载体 | 内容 |
|------|------|------|
| 报告级（a5） | ancok-a5-20260704.md | 暴露面=0 实测 → 建议降级休眠缺口 |
| **策略文档级（本注记）** | **TARGET_STRATEGY_MAXFULL.md §4** | **`AncOK ceiling = WAIVED FOR SIGNAL ALPHA`（PDF p4/p15 措辞），§4.1 引 a5 L2 数据依据，§4.2 列四项翻转条件** |

§4 的正式化措辞（scoped）：**`AncOK ceiling = WAIVED FOR SIGNAL ALPHA`**——PDF p15 等价标注
`AncOK ceiling = not blocking signal alpha`。§4 同时明写：**「完整声部执行必须闭合——这一步在
exec-full（M5）不能 waiver，只能排在执行层之前完成」**。即 waiver 是 **signal-alpha 域限定**，
非全局豁免。§4.1 将本号 ceiling 记为「生成态 → 已验证休眠缺口（BTC/L2，暴露面=0）」，与上文
L2 验证注记逐字一致。

### §4.2 四项翻转条件（waiver ≠ 永久搁置）

§4.2 明列四项翻转条件（任一成立则 waiver 翻转为「实装严格修复」路径1），本号 front matter
`waiver_status` 记录之：

1. 跨标的（CL/ES/金油）L3：LiveDetached 分布/registry 作废时序不同 → `restore_break_registry_lost`
   可能转正。
2. （§4.2 第2项，翻转触发条件之一）。
3. **彻底修复实装后**：增量 extract_elements / confirmed prefix immutable 被实装 → 本 L2 结果作废
   （但届时 ceiling 本身消失）。
4. **探针被移除**：coverage.rs `ancok_probe_*` 被删 → 休眠缺口失去可观测性，697 回退为不可验证 ceiling。

第3/第4项与上文「边界条件」段完全一致；第1项与「有效域声明」段的 L3 翻转路径一致。§4.2 是本号
边界条件的策略文档级正式化，无新增矛盾。

### 结算判定：维持生成态（waiver ≠ settled，彻底修复未实装）

**判定 = 维持生成态。** §4 的正式 waiver **不满足**本号回溯结算条件：

1. **waiver 是 scoped 豁免，非 settled**：§4 明写 waiver **仅限 signal-alpha 域**，且 exec-full（M5）
   **不能 waiver**（完整声部执行必须闭合）。本号上文「边界条件」第三条把 WAIVED 情形（编排者裁定
   彻底修复超出本轮范围、永久搁置）与 settled 情形（彻底修复实装 + L2/L3 通过）**明确区分**——
   §4 走的是 WAIVED 路径（保留可见性 + 四项翻转条件 + 探针常驻），**不是** settled 路径（彻底修复
   实装）。彻底修复至今**未实装**，故不结算。
2. **休眠 ≠ 消除，waiver ≠ 消除**：a5 暴露面=0 使 ceiling 休眠，§4 使其在 signal-alpha 域被 waive，
   但机制在 BTC 上仍在（低频观测 state_live_present=15），四项翻转条件任一触发即复活。这与 693 号
   A10/A11 硬裁决的 WAIVED 先例同构——WAIVED 是「保留可见性但不阻塞」的合法态，不是「已解决」。
3. **exec-full 的 ceiling 未 waive**：§4 明示 exec-full（M5 声部执行）**不能 waiver**——本号 ceiling
   的四态 overlay 完整度在 exec-full 域仍是 MUST 闭合项。M5 OverlayState（m5overlay-c1）已实装声部
   执行账本（对账残差 9.24e-7），但那是「声部净额账户」的落地，**非** persistent.rs 增量 extract_elements
   的彻底修复——两者是不同对象（M5 = 执行账本层；697 = 元素持久化层）。exec-full 域的 ceiling 是否
   已被 M5 覆盖属选择类核实（待 Lead/编排者判断是否需要独立验证），非本注记自决。

**结论**：本号维持生成态。剩余待裁项 = (a) 彻底修复（增量 extract_elements + confirmed prefix
immutable）是否列入后续 goal（选择类，Lead 按 275 依赖序调度）；(b) exec-full 域 ceiling 是否已被
M5 完整覆盖（选择类核实）；(c) 四项翻转条件的监控（探针常驻 + 跨标的 L3 触发）。§4 的 signal-alpha
waiver 正式化已记录在案——waiver 关闭的是「signal-alpha 是否被 ceiling 阻塞」（答案：否，暴露面=0），
**不关闭** ceiling 本身的存在与彻底修复的待办性。有效域纪律（231）：waiver 有效域 = BTC/L2/signal-alpha，
不外推 exec-full、不外推跨标的。
