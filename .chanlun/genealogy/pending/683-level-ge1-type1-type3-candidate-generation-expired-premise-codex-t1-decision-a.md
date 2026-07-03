---
id: "683"
number: 683
status: 生成态   # 决断已定（codex-t1 裁 A），实装未完（#123 hl13-impl in_progress）——待 GOLDEN 重冻结后回溯结算。【task #130 对齐：本条目=#130「谱系补记 mod.rs:256 过期未复核简化 / spec-execution-gap 候选」的落盘，genealogist 2026-07-03 认领对齐后 #130 completed。】
date: "2026-07-03"
type: bias-correction   # level≥1 一/三类买卖点信号系统性缺失（架构性禁闭于 L0）被识别并订正。定性=spec-execution-gap 候选（codex-decide-20260703 裁：是），下文 new_output 显式化。
source: "[新缠论] mod.rs:256-262 else{Vec::new()} 分支 + codex-decide-20260703-025537-8d3c.md（裁定：spec-execution-gap 候选=是）"
negation_source: heterogeneous   # 主裁定=codex-t1（codex-cli decide A）；git blame 溯源审计由 #119(type1-arch-audit)/#123(hl13-impl) 工位完成（homogeneous）。
negation_model: codex-cli
negation_form: waiting   # c570d2ebf6 提交注释原文自陈"是后续增量"（显式 waiting 型延迟），非静默简化。
depends_on: []
related: ["090", "231", "682", "685", "project_oddeven_mu_identity"]

# 拓扑效果标注（147号下游推论3）
# negates：commit c570d2ebf6 的假设——"units 无 direction，level≥1 一/三类判据无法 bit-exact 判定，
#   须留空等待后续 units+direction 增量"。
# 实际拓扑后果（retrospective，141号结论1）：该假设在**同一提交**内已被 project_to_units 的
#   UnitRange.direction = m.fold_direction(prev) 满足（方向字段已存在，来源是 fold 而非线段自带），
#   但 else{Vec::new()} 分支此后两次改动（98609b7821/5011c2e5b0）均未被重新评估——冻结持续到本次审计。
#   scope=downstream：受影响的不只是该分支本身，还包括下游消费者（extract_second_for_level 二类信号、
#   assemble_gamma、GOLDEN/backtest digest）在信号集变化后须重跑验证。
topo_effect: "freeze:level-ge1-type1-type3-candidate-generation:downstream"

# 矛盾（该假设的过期未复核）
contradiction:
  description: |
    commit c570d2ebf6（2026-06-27，"资本风控层+递归塔对象升级(B2/S2真接入)"）在 mod.rs:256 引入
    `else { Vec::new() }`，注释原文写明理由："上级单元无方向，第三类判据无法 bit-exact 判定 ⟹ 留空；
    需要'units 加 direction'的递归扩展，是后续增量"。但同一提交里 `project_to_units` 已经实现了
    `UnitRange.direction = m.fold_direction(prev)`（recursive_tower.rs:195 `LeveledMove::fold_direction`）
    ——注释所称的阻塞前提在写下的那一刻已经被满足，此后两次改动（98609b7821 性能重构、5011c2e5b0
    力度代理并置）都只动了 `is_l0` 分支内部，`else` 分支及其阻塞理由从未被重新评估。
    结果：level≥1（is_l0=false）的一/三类判定（`extract_signals_with_hist`）**架构性从不调用**——
    高级别 bsp 仅靠 `extract_second_for_level`（二类，次级别一类构成）产生，与缠论原文"每个级别都有
    完整三类买卖点"（第17/20课）不符。
  layer: 代码   # else{Vec::new()} 是确定性代码分支（is_l0 布尔），非数据依赖（type1-arch-audit §7 认识论等级：L0/代码事实）。
  trigger: "team-lead 质疑 #116 观测（level1-4 全第二类）的 (a) 裁定是否成立 → #119(type1-arch-audit) 分级审计 → 定位 mod.rs:256 else 分支 → git blame 溯源至 c570d2ebf6 → codex-t1(#121) 裁定。"

# 涉及的定义
definitions_involved:
  - name: "第17/20课：级别递归——每个级别有完整三类买卖点"
    version: "docs/chanlun/text/blog/INDEX.md"
    role: "被违反的原文预期。level≥1 一/三类=0 与该原文不符（非'级别越高一类越少见'的市场事实，是架构未生成）。"
  - name: "买卖点定律一 §10.2：二类=次级别一类构成"
    version: "docs/chanlun/text/blog/INDEX.md"
    role: "已忠实实装的部分（extract_second_for_level）。本记录不涉及此部分，仅涉及一/三类的级别≥1缺口。"
  - name: "LeveledMove::fold_direction（recursive_tower.rs:195）"
    version: "commit c570d2ebf6 引入"
    role: "被误判为'尚未存在'的能力——实际上与 else{Vec::new()} 同一提交产出，是本次'过期未复核'的核心证据。"

# 解决方式
resolution:
  type: 定义修正   # 建模选择裁定（A：级别-N 直接判定），非概念分离。
  description: |
    codex-t1（#121）裁 **A：级别-N 直接判定**——在级别-N 的 units（携 fold_direction）+ centers
    （detect_centers_geometric）+ 趋势裁决（classify_move==Trend(dir)）上跑类比 judge_first_cached
    的判据，A/C 段面积比较复用 sublevel_diverges 同族原语（second_for_parent 已证明可在任意级别工作）。
    拒绝 B（递归relabel）：B 的候选集合 ⊆ L0 一类候选集合，而 L0 一类在 BTC 300K 恒为 0（#119 坐实），
    故 B 在当前数据下恒空——非死路的唯一活路是 A。不选 C：无需新架构，A 的实现细化已足够。
    实装中（#123 hl13-impl，in_progress）：UnitRange/LeveledMove 不可变 view + 趋势门槛
    classify_move==Trend(dir) + A/C 段面积复用 sublevel_diverges+AreaCache + 三类同级 units+centers
    离开/回试，验证 bit_exact_synthetic + project_to_units_resume_matches_full + 全 lib + GOLDEN 诚实重算。
  decided_by: 蜂群内部（codex-t1 异质裁定，team-lead 委托审计链 #116→#119→#121→#123）

# 被否定的方案
negated:
  description: "维持 mod.rs:256 else{Vec::new()}：level≥1 一/三类候选永久留空，仅靠二类(B2/S2)覆盖高级别信号。"
  why_negated: |
    (1) 阻塞前提已过期：c570d2ebf6 同一提交内 fold_direction 已提供 level≥1 方向字段，"units 无方向"
        的留空理由从写下的那一刻起就不成立。
    (2) 与缠论原文不符：第17/20课级别递归要求每级完整三类，架构性禁闭一/三类于 L0 无原文依据
        （type1-arch-audit §3：'部分原文依据，部分待谱系确认'——本记录完成该确认，判定为无原文依据的
        实装简化，非定律推论）。
    (3) 候选生成缺口是确定性事实（架构分支，与数据窗无关，恒成立），非市场事实——不能用"level0 一类
        在 BTC 300K 罕见触发"的经验论证（那是 level0 的独立结论）去豁免 level≥1 的架构缺口。

# 新产出
new_output:
  definitions:
    - "过期未复核的文档化简化（expired-unaudited documented simplification）= **spec-execution-gap 候选**
      （codex-decide-20260703 裁定：是）：commit 内写明理由的简化，若其阻塞前提在后续提交（甚至同一提交）
      内被满足，而简化分支本身未随之复核，则该简化从'有效延迟'退化为'过期占位'——声明（commit message
      的'后续增量'）与能力（fold_direction 已提供方向）脱节 = spec-execution-gap。须定期对 else/TODO 类
      分支做前提复核，不能只信任 commit message 的时效性。"
    - "级别-N 判定的适配层设计：级别-N '线段'角色由 UnitRange/LeveledMove 承担（end_price 按 direction
      取 hi/lo）、A 段定位复用 second_for_parent 的'序列序最近同向前驱'模式、趋势门槛取该级
      classify_move 的 Trend(dir)。"
  code_changes: "rust/src/theta_v0/classifier/mod.rs:256 else{Vec::new()} → level-native 一/三类判定（#123 hl13-impl 实装中，未完成）。"
  orchestration_changes: "无（本记录为谱系补记，不改编排流程）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/classifier/mod.rs（classify_impl 主循环，else 分支替换）"
    - "rust/src/theta_v0/classifier/signal.rs（judge_first_cached 判据的级别-N 适配需求）"
    - "GOLDEN/backtest digest（新增高级别一/三类信号会改变信号集，需诚实重算，非静默接受）"
  affected_definitions:
    - "#116(type1-zero-probe) 的观测结论范围收窄：'level1-4 全第二类'不再是全部真相，是'level≥1 一/三类被架构禁闭'的表现之一。"
  downstream_implications:
    - "extract_second_for_level（二类信号）消费面积原语 sublevel_diverges 的方式，为级别-N 一类判据的复用提供了先例（second_for_parent 已证明该原语跨级可用）。"
    - "任何'else 分支 / TODO 占位'类代码，若 commit message 记录了阻塞前提，须在前提可能被后续改动满足时主动复核——不能假设 commit message 的时效性会自动被后续开发者继承（本例两次后续提交都只动了 is_l0 分支，无人复核 else 分支）。"

# 回溯结算（如果适用，待 #123 完成后补记）
retroactive_settlement:
  settled_by: "#123 hl13-impl 完成 + GOLDEN 诚实重算通过后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "无直接父记录（首次审计 else{Vec::new()} 分支的历史裁决；谱系检索无直接讨论该问题的已结算条目）。"
  children: ["685"]   # 685号推翻本记录边界条件第三条（见下方"边界条件"订正标注）。
---

# bias-correction 683：level≥1 一/三类买卖点候选生成——过期未复核的文档化简化（spec-execution-gap 候选），codex-t1 裁 A 订正

## 结论

`mod.rs:256` 的 `else { Vec::new() }` 是**有文档记录的实装简化**（commit `c570d2ebf6` message 明确写了
理由：上级单元无方向，判据无法 bit-exact 判定），**不是**未显式化的静默简化。但该理由所指向的阻塞
前提（"units 加 direction 是后续增量"）**在同一次提交里就已经被满足**——`project_to_units` 的
`UnitRange.direction = m.fold_direction(prev)`（`recursive_tower.rs:195`）与 `else` 分支同一提交产出。
此后两次提交（`98609b7821`、`5011c2e5b0`）均未触碰 `else` 分支，导致 level≥1 一/三类买卖点候选
**架构性从不生成**，持续到本次审计（`#119 type1-arch-audit` → `#121 codex-t1` 裁 A → `#123 hl13-impl`
实装中）。**codex-decide-20260703 定性为 spec-execution-gap 候选（声明「后续增量」vs 能力「方向已具备」脱节）。**

codex-t1 裁定：**A（级别-N 直接判定）**，拒绝 B（递归relabel，因其候选集合 ⊆ L0 一类候选集合而
L0 一类恒为 0 ⟹ B 恒空），不选 C（无需新架构）。

## 定义依据

- 第17/20课（级别递归，`docs/chanlun/text/blog/INDEX.md`）：每个级别应有完整三类买卖点。level≥1 仅靠
  二类覆盖，与此不符。
- 买卖点定律一 §10.2：二类=次级别一类构成——已忠实实装（`extract_second_for_level`），本记录不涉及。
- `LeveledMove::fold_direction`（`recursive_tower.rs:195`）：level≥1 方向字段的实际来源，证伪"units 无
  方向"的阻塞假设。

## 边界条件（翻转）

- 若未来形式化规格明确规定"高级别一类必须由 L0 已确认一类标签继承"（即显式选 B 而非 A）——本记录的
  订正方向翻转，codex-t1 的裁定需重新评估。
- 若 `#123 hl13-impl` 实装后 GOLDEN 重算发现级别-N 判据存在 bit-exact 偏差（如 `end_price` 取
  hi/lo 的方向映射有误），则本记录的"决策已定"状态需回退为"决策待修正"，`retroactive_settlement`
  不得填入直至偏差修复。
- ~~本记录不涉及 level0 一类=0 的市场事实结论（该结论维持 `#119` 的独立裁定，不受本记录订正影响）。~~
  **【685号订正，2026-07-03】此条已作废。** #141 漏斗真跑（`4d37f1eab9`）坐实 level0 一类=0 同样死于
  趋势门（环1，全级别 100% 候选未过），是趋势门累积链判据缺第18课走势分解 + 缺第20课中枢延伸（第20课
  中心定理一）两条确定性实现缺陷制造的伪影，**不是** `#119` 独立裁定的市场事实。详见
  `685-type1-zero-not-market-fact-trend-gate-accumulation-chain-plus-missing-center-extension.md`。

## 下游推论

- `#116(type1-zero-probe)` 的"level1-4 全第二类"观测结论范围收窄：这不是完整市场画像，是"level≥1
  一/三类被架构禁闭"的表现之一，须在 `#123` 完成后用新信号集重新观测。
- 方法论层面："else 分支 / TODO 占位"类代码若 commit message 记录了阻塞前提，该前提在后续提交
  （包括同一提交内的其他改动）满足后未被复核 = 冻结未解冻。genealogist 常设巡检可将此类模式纳入
  张力检查范围（检索 `else { Vec::new() }` / `// TODO` / "后续增量"等占位注释，核对其阻塞前提是否
  已被后续代码满足）。

## 谱系引用

- 姊妹：`682`（移植增量归属须核目标文件存在性）——同族方法论：声明/简化须核实际前提是否仍然成立，
  不能假设 commit message 的时效性会被后续开发者自动继承。
- 关联：`[[project_oddeven_mu_identity]]`（μ̂ 奇偶交替=beta 漂移伪结构）——level≥1 信号仅二类覆盖是
  「level1-4 全第二类」观测（#116）的架构成因，而奇偶 level 相位结构的可识别性讨论以「每级信号集
  完整」为隐含前提；本条目解除该架构禁闭后，#116/奇偶观测须用新信号集重测（下游推论第一条）。
- 约束：`090`（声明膨胀禁止）、`231`（形式化有效域规则——else 分支的"暂时留空"声明的有效域不能
  无限期覆盖后续提交）。
- 溯源链：Task #116（type1-zero-probe）→ Task #119（type1-arch-audit，定位 else 分支+发起 git blame
  子项）→ Task #121（codex-t1，异质裁定 A）→ Task #123（hl13-impl，实装中）。
- **task #130 对齐**：本条目即 #130「谱系补记：mod.rs:256 过期未复核的文档化简化（spec-execution-gap
  候选）」的落盘产出。#130 要求引用 #116/#119/#121 + codex-t1 裁定 A + 记完整生成史（简化引入→阻塞前提
  同 commit 失效→过期未复核→裁定 A 解除）——均已在本条目覆盖。genealogist 2026-07-03 认领对齐后 #130 completed。
- **子记录 `685`**：#141 漏斗真跑推翻本记录"边界条件"第三条（level0 一类=0 是趋势门实现缺陷伪影，
  非 `#119` 独立市场事实裁定）。本记录的 level≥1 架构禁闭（else 分支）与 685 的全级别趋势门锁死是
  **两条独立且叠加的缺陷**：即便 683 的 else 分支修复（#123 hl13-impl）让 level≥1 走级别-N 直接判定，
  该判定仍需先通过趋势门——685 未修复前，level≥1 一类同样会死于趋势门（环1），候选生成端（683）与
  过滤端（685）需分别修复才能让一类信号非 0。

## 影响声明

谱系补记，零代码改动（代码变更归属 `#123 hl13-impl`，本记录追踪其裁定依据与"过期未复核"定性）。
影响模块：`rust/src/theta_v0/classifier/mod.rs`、`signal.rs`；影响下游：GOLDEN/backtest digest
需在 `#123` 完成后诚实重算。**685号订正**：本记录"level0 市场事实"免责边界条件已作废，683/685 需
在 #142-#146 修复序（中枢延伸→走势分解→局部趋势门→A/C次级别化→全下游重跑）完成后共同结算。
