---
id: "643"
number: 643
status: 已结算   # 执行闭环 2026-06-30：cascade reset 实装经约束3（#56 独立工位实测 classifier/mod.rs:814/842-847）+ 约束4（#57 codex/gpt-5.5 亲核四问全 PASS）双重异质审确认非假绿，conclusive 可结算。acceptance[1] L2 真实对齐**开放项保留**（L1 合成自洽已确认，L2 不由本审翻正=有效域边界标注，非缺陷）。
date: "2026-06-29"
settled_date: "2026-06-30"
settled_by: "genealogist（结算依据 .chanlun/diagnostics/643-action-chain-status.md：约束3 #56 + 约束4 #57 双重异质审）"
type: bias-correction   # 声明膨胀纠正（acceptance[1] L1 合成 PASS 冒充 L2 真实对齐）+ 守卫完备性误判（投影相等⟹sub_moves相等 为假前提）
depends_on: ["231", "625", "640", "090"]
related: ["631", "641", "642", "001", "002"]
title: "acceptance[1] bit-exact『PASS』是 L1 合成数据假 PASS——L2 真实 CL bar1464 发散（pre-existing parser bug）；根因=古怪线段末段原地变异（67/78课定义层正确非bug）+ caller 守卫只查段数回缩不查末段值改写→复用陈旧缓存中枢；第一版 frontier_mutated 守卫比对 project_to_units 投影（有损丢 sub_moves）『碰巧过 8000bar 测试』被 codex 走真实守卫入口复现反例（115→147 vs 147→140 内点）抓出深嵌套 L2 cache subs 陈旧；修复=cascade reset（任一级 frontier 变异→该级+所有上级 reset），消除『投影有损丢 sub_moves』整类问题"
negation_source: "工位E L2 真实数据（CL bar1464 发散）+ git stash 自证 pre-existing + codex 异质审查（走真实守卫入口复现 115→147 vs 147→140 内点反例，坐实第一版守卫依赖『投影相等⟹sub_moves相等』假前提）+ 源码事实（project_to_units line 383 产 UnitRange 丢 sub_moves；sub_moves line 77-93 携坐标侧车）"
negation_form: "negation"   # negation：acceptance[1]『PASS』被 L2 真实数据否证 + 第一版守卫『完备』被 codex 反例否证

# negation：双重否证——
#   否证1（有效域膨胀）：acceptance[1] 报 PASS，但仅验 L1 合成数据（bit_exact_synthetic）；
#     工位E 跑 L2 真实 CL bar1464 发散。L1 合成 PASS ≠ L2 真实对齐（231 信息增量零，625 同构）。
#     git stash 自证是 pre-existing bug（非本轮引入）。
#   否证2（守卫完备性误判）：工位F 第一版 frontier_mutated 守卫比对 project_to_units 投影——
#     投影有损（丢 sub_moves），上级投影 bit-identical 但 sub_moves 变时漏 reset。
#     codex 走真实守卫入口复现反例（115→147 vs 147→140 内点），坐实深嵌套 L2 cache subs 陈旧。
#     8000bar PASS 只证该序列没踩反例，不证守卫完备（依赖『投影相等⟹sub_moves相等』为假前提）。

topo_effect: "negates:acceptance1-bit-exact-pass-claim; negates:frontier-mutated-guard-projection-equality-completeness"

# 矛盾（type=bias-correction 必填）
contradiction:
  description: "两层声明膨胀。层1（有效域膨胀，231/625 同构）：acceptance[1] 报 bit-exact『PASS』，但断言对象仅 L1 合成数据（bit_exact_synthetic）；工位E 跑 L2 真实 CL bar1464 揭示发散——L1 合成 PASS 掩盖 pre-existing parser bug（git stash 自证非本轮引入）。根因（定义层正确，非 bug）：parser 末段原地变异（缠论古怪线段重划，第67课线段划分标准 / 第78课古怪线段『笔破坏后未必形成线段破坏』）——这是 chanlun 定义层正确的动态行为。真 bug 在 caller 守卫：只查段数回缩（segment count 减少）触发 reset，不查末段值改写（末段几何被原地修改但段数不变）→ 复用陈旧缓存中枢。层2（守卫完备性误判，640 纪律实例）：工位F 第一版修复用 frontier_mutated 守卫比对 project_to_units 投影判断是否变异——但 project_to_units（line 383）产 UnitRange，**丢弃 sub_moves**（携坐标侧车，line 77-93）。当上级 RMove 投影 bit-identical 但其 sub_moves（深嵌套 L2 cache subs）变化时，守卫漏 reset。codex 异质审查走真实守卫入口复现反例（115→147 vs 147→140 内点：上级投影相等但内点 sub 不同），坐实第一版守卫依赖『投影相等 ⟹ sub_moves 相等』这一**假前提**。8000bar PASS 只证该测试序列没踩到反例，不证守卫完备。严格修复 = cascade reset（任一级 frontier 变异 → 该级 + 所有上级 reset），从根上消除『投影有损丢 sub_moves』整类问题（不再依赖投影比对，frontier 变即级联清缓存）。"
  layer: 实装   # rust/theta_v0/classifier parser + 缓存守卫层。根因（古怪线段末段变异）是 chanlun 定义层正确行为；bug 在 caller 守卫的有效域。非定义冲突。
  trigger: "本轮 goal g-sigma-complete-l2-nautilus：工位A 报 acceptance[1] PASS（仅 L1 合成 bit_exact_synthetic）；工位E 跑 L2 真实 CL bar1464 发散；git stash 自证 pre-existing。工位F 第一版修复『碰巧过 8000bar』，codex 异质走真实守卫入口复现 115→147 vs 147→140 内点反例，逼出 cascade reset。"

# 涉及的定义
definitions_involved:
  - name: "231 形式化有效域规则（L0/L1/L2 认识论等级）"
    version: ".claude/rules/formalization-validity-domain.md（status: 已结算，谱系 231）"
    role: "约束来源。L1 合成数据验证信息增量零（验证管线不验证假设）。acceptance[1] bit_exact_synthetic 是 L1（自造合成数据），声明『PASS』暗示 L2 真实对齐 = 有效域膨胀（声明域=真实对齐，实际域=合成自洽）。工位E 的 L2 真实 CL bar1464 是 L2 否证（缩小有效域边界，否定性结果价值）。"
  - name: "625 L2 真实数据揭示 L1 合成 GREEN 掩盖引擎 bug"
    version: ".chanlun/genealogy/settled/625（631 line 92/123 引用其同构模式）"
    role: "同构先例。625 = L2 真实数据揭示 L1 合成 GREEN 掩盖引擎 bug。本号层1 是其活实例：acceptance[1] L1 合成 PASS 掩盖 pre-existing parser 守卫 bug，L2 真实 CL bar1464 揭示。"
  - name: "640 自评无漏洞=最该被异质审查（异质审查纪律）"
    version: ".chanlun/genealogy/settled/640-function-uniqueness-vs-structural-uniqueness-prom-single-valued.md（lead-self-cleared-inflation-flag-is-rationalization memory 同源）"
    role: "约束来源。本号层2 是其活实例：工位F 第一版『碰巧过 8000bar』= 自评通过，最该被异质审查；codex 走真实守卫入口复现反例抓出『投影相等⟹sub_moves相等』假前提。运动员当裁判（自评通过的守卫）须路由异质审查。"
  - name: "缠论第67/78课 线段划分标准 + 古怪线段（笔破坏后未必形成线段破坏）"
    version: "docs/chanlun/text/blog/INDEX.md（第67课线段划分 / 第78课古怪线段 + 顶高于底硬约束）；CLAUDE.md 已知缺口补录；谱系 001/002"
    role: "根因依据（定义层正确）。parser 末段原地变异是古怪线段重划的正确动态行为——第78课『笔破坏后未必形成线段破坏』。这不是 bug，是 chanlun 定义层正确。bug 在 caller 守卫未覆盖『末段值改写而段数不变』这一合法变异路径。关联 001（退化线段）/002（来源不完备，第67/77/78课曾缺于编纂版）。"
  - name: "090 严格性语法规则（声明膨胀禁止）"
    version: ".chanlun/genealogy/settled/090（声明膨胀禁止来源）"
    role: "约束来源。acceptance[1]『PASS』声明代码不具备的能力（L2 真实对齐）= 声明膨胀（禁止模式5）。"

# 解决方式
resolution:
  type: 已结算   # cascade reset 实装经约束3（#56 独立工位实测）+ 约束4（#57 codex 异质审）双重确认非假绿，conclusive。acceptance[1] 声明已降级（L1 合成非 L2 对齐，commit 0c499e6fbb 落盘）。L2 真实 CL 对齐保留为开放有效域边界（不由本审翻正）。
  description: "严格修复（cascade reset，工位F 已实装 + codex 异质闭环确认）：任一级 frontier 变异 → 该级 + 所有上级 reset，**不再比对 project_to_units 投影**（投影有损丢 sub_moves 是假前提的根）。这从根上消除整类问题：frontier 变即级联清缓存，无需判断『投影是否相等』。被否定的第一版（比对投影）依赖『投影相等⟹sub_moves相等』假前提，codex 反例（115→147 vs 147→140 内点）证伪。声明降级（已执行，commit 0c499e6fbb）：acceptance[1] bit-exact PASS 已标注为 L1 合成自洽（bit_exact_synthetic），不声称 L2 真实对齐——L2 真实 CL 验证独立标注（feature_seq.rs:14 模板）。"
  decided_by: 蜂群内部（约束3 #56 独立工位 + 约束4 #57 codex 异质审双重确认）   # 工位F 实装 cascade reset；#56 独立工位实测真实装（classifier/mod.rs:814/842-847，非转述）；#57 codex/gpt-5.5 亲核四问全 PASS（Q1 单调向上 reset/Q2 frontier 变异三类全覆盖/Q3 真 bit-exact/Q4 L1 标注诚实）；genealogist 结算

# 被否定的方案
negated:
  description: "(1) 接受 acceptance[1] L1 合成 PASS 为『bit-exact 对齐』（暗示 L2 真实）。(2) 工位F 第一版 frontier_mutated 守卫：比对 project_to_units 投影判断是否变异，投影相等则不 reset。(3) caller 守卫只查段数回缩（segment count 减少）触发 reset。(4) 用 8000bar PASS 论证守卫完备。(5) 把 parser 末段原地变异当 bug 去消除（古怪线段重划是定义层正确）。"
  why_negated: "(1) L1 合成 PASS 信息增量零（231），工位E L2 真实 CL bar1464 发散否证『真实对齐』，git stash 自证 pre-existing。(2) project_to_units（line 383）产 UnitRange 丢 sub_moves，投影相等不蕴含 sub_moves 相等——codex 反例（115→147 vs 147→140 内点）证伪假前提。(3) 段数回缩只覆盖部分变异路径；末段原地值改写而段数不变时漏 reset→复用陈旧缓存中枢。(4) 8000bar PASS 只证该序列没踩反例，不证完备（640：自评通过最该被异质审查）。(5) 古怪线段末段重划是第67/78课定义层正确行为，删它=改定义（no-workaround）；正确修复是补全 caller 守卫（cascade reset），不动 parser 正确逻辑。"

# 新产出
new_output:
  definitions:
    - "有效域膨胀实例：acceptance[1] bit-exact『PASS』= L1 合成自洽（bit_exact_synthetic），非 L2 真实对齐。L2 真实 CL bar1464 否证（231/625 同构）。"
    - "守卫完备性假前提：『投影相等 ⟹ sub_moves 相等』为假——project_to_units 投影有损丢 sub_moves（携坐标侧车），上级投影 bit-identical 但深嵌套 sub_moves 变时第一版守卫漏 reset。"
    - "cascade reset 整类修复：任一级 frontier 变异 → 该级 + 所有上级 reset，不再比对投影，消除『投影有损丢 sub_moves』整类问题。"
    - "根因分层：parser 末段原地变异（古怪线段重划，第67/78课定义层正确，非 bug）vs caller 守卫只查段数回缩不查末段值改写（守卫有效域不完整，bug）。"
    - "640 纪律实例：工位F 第一版『碰巧过 8000bar』= 自评通过的守卫，被 codex 异质走真实守卫入口复现反例抓出。"
  code_changes: "工位F 已实装 cascade reset（classifier/mod.rs:814/842-847，#56 独立工位实测坐实真实装非转述）+ codex 异质闭环确认（#57 gpt-5.5 四问全 PASS）。acceptance[1] 声明降级（L1 合成非 L2 对齐）已落盘 commit 0c499e6fbb。"
  orchestration_changes: "方法论：①acceptance/PASS 报告必须标注 L 级——L1 合成 PASS 不得冒充 L2 真实对齐（231/625/631）。②守卫『完备』声明前必经异质审查走真实入口复现反例——8000bar/N-sample PASS 只证没踩反例，不证完备（640）。③守卫比对的『投影/摘要』必须核其是否有损——project_to_units 丢 sub_moves 是『投影相等⟹原对象相等』假前提的根；有损投影做守卫判据=漏 reset。④整类修复优先于点修复：cascade reset（frontier 变即级联清）消除整类，胜过逐反例补投影比对（no-patch：点修补陈旧缓存=补丁思维）。⑤定义层正确行为（古怪线段末段重划）不可当 bug 删——补全 caller 守卫，不动 parser 正确逻辑（no-workaround）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/classifier/mod.rs:814/842-847 → cascade reset 实装（任一级 frontier 变异→该级+所有上级 reset；#56 独立工位实测真实装；frontier 变异三类全覆盖：长度回缩/前缀改写/尾部续读）。"
    - "rust/src/theta_v0/classifier/recursive_tower.rs → project_to_units（line 383）产 UnitRange 丢 sub_moves，已确认不作守卫判据（cascade reset 不依赖投影比对替代）。"
    - "theta_v0 acceptance[1] / bit_exact_synthetic 测试 → 声明已降级为 L1 合成自洽（commit 0c499e6fbb），不冒充 L2 真实对齐；L2 真实 CL parity 独立标注（feature_seq.rs:14 模板）。"
  affected_definitions:
    - "231（已结算）：本号是 L1 合成 PASS 掩盖真实 bug 的活实例，维持 settled。"
    - "625（已结算）：本号是其同构模式（L2 揭示 L1 GREEN 掩盖 bug）在 parser 守卫维度的实例，维持 settled。"
    - "640（已结算）：本号是其异质审查纪律的活实例（自评通过守卫被 codex 反例抓出），维持 settled。"
    - "缠论第67/78课古怪线段（settled 域定义）：本号印证末段重划是定义层正确行为，不改定义。"
  downstream_implications:
    - "acceptance/PASS 报告须带 L 级标注，L1 合成不得冒充 L2 真实（231/625/631 固化）。"
    - "守卫完备性须异质审查走真实入口复现反例，N-sample PASS 不证完备（640）。"
    - "有损投影（project_to_units 丢 sub_moves）不得作守卫判据——cascade reset 整类修复替代投影比对。"
    - "cascade reset 后深嵌套 L2 cache subs 陈旧整类问题消除（实装层 conclusive）；**但 acceptance[1] L2 真实 CL bar1464 对齐仍为开放有效域边界**——L1 合成自洽已确认，L2 真实数据对齐不由本审翻正（不预设结果），须独立 L2 工位坐实。"

# 谱系关联
related_records:
  parent: "231号（有效域膨胀）+ 640号（异质审查纪律）——本号是两者的活实例"
  children: []
  related:
    - "625号：L2 真实揭示 L1 合成 GREEN 掩盖 bug——本号同构（parser 守卫维度）"
    - "631号：bit-exact 声明膨胀基线——本号是其措辞膨胀在『acceptance PASS』维度的运行时实例（631 是注释措辞，本号是测试 PASS 报告）"
    - "641号（同轮）：声明膨胀（exp≈1）——本号是声明膨胀（PASS）的姊妹实例，不同轴（性能 vs 守卫完备性）"
    - "642号（同轮）：acceptance[2] 自举缺口——本号 acceptance[1] 守卫缺口，同源（acceptance 链 L1 PASS 掩盖真实缺口），不同模块（parser 守卫 vs coverage 自举）"
    - "001号/002号：退化线段 + 来源不完备（第67/77/78课曾缺于编纂版）——本号根因（古怪线段末段重划）的定义层来源"
    - "memory lead-self-cleared-inflation-flag-is-rationalization：640 同源——自评通过须路由异质审查（codex 抓 lift_single_valued 同模式）"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "acceptance[1] bit_exact_synthetic PASS（仅 L1 合成数据自洽）"
    level: "L1（合成数据验证，信息增量零——验证管线不验证假设）"
    increment: "零：L1 合成 PASS 不验证 L2 真实对齐假设"
  - proposition: "L2 真实 CL bar1464 发散（pre-existing parser 守卫 bug，git stash 自证）"
    level: "L2（工位E 真实 CL 数据，否定性结果）"
    increment: "正：否证『L1 PASS ⟹ L2 真实对齐』，缩小有效域边界"
  - proposition: "project_to_units（line 383）产 UnitRange 丢 sub_moves；第一版守卫『投影相等⟹sub_moves相等』为假前提"
    level: "L0（源码事实：project_to_units 签名 + sub_moves line 77-93 携坐标侧车）+ codex L2 反例（115→147 vs 147→140 内点，走真实守卫入口复现）"
    increment: "高：守卫完备性假前提的否证（L0 结构 + L2 反例双坐实）"
  - proposition: "parser 末段原地变异 = 古怪线段重划（第67/78课定义层正确）"
    level: "L0（缠论定义事实：第78课『笔破坏后未必形成线段破坏』）"
    increment: "高：根因分层判定（定义正确 vs 守卫缺口）"
  - proposition: "cascade reset（任一级 frontier 变→该级+所有上级 reset）消除『投影有损丢 sub_moves』整类问题"
    level: "L1（工位F 实装 + 8000bar 合成自洽；#56 独立工位实测真实装 + #57 codex 四问 PASS 双重异质审确认非假绿）"
    increment: "中：整类修复的管线正确性 conclusive（实装层）；acceptance[1] L2 真实 CL bar1464 对齐仍开放（有效域边界，不由本审翻正）"

# 结算（settlement，2026-06-30）
settlement:
  closed_date: "2026-06-30"
  basis: "约束3（#56 独立工位实测）+ 约束4（#57 codex/gpt-5.5 异质审）双重确认 cascade reset 非假绿，conclusive。结算依据全文见 .chanlun/diagnostics/643-action-chain-status.md。"
  evidence:
    - "约束3 #56（独立工位实测，非转述）：PASS。实测纠正『cascade reset 失真』转述假阳性——cascade reset 真实装 classifier/mod.rs:814/842-847（非 recursive_tower.rs），谱系 643 未失真，acceptance[1] L1 降级真落盘 commit 0c499e6fbb。0 CRITICAL/HIGH。"
    - "约束4 #57（codex 异质审，gpt-5.5，137691 tokens）：满足。亲核四问全 PASS——Q1 该级+所有上级 reset 成立（cascade_reset 循环外单调）/Q2 frontier 变异三类全覆盖（长度回缩/前缀改写/尾部续读；反例 seg[8]147→140 经 UnitRange.hi 触发）/Q3 真 bit-exact（derive PartialEq 递归全字段 RMove::Compose.subs）/Q4 acceptance[1] L1 标注诚实（合成自洽非 L2）。codex verdict 原文：『643 cascade reset 对所给反例不是假绿；acceptance[1] 只能诚实标 L1 合成自洽，不能冒充 L2 对齐』。"
  closed_scope: "643 cascade reset 实装完备部分（双重审 conclusive）。"
  open_boundary: "acceptance[1] L2 真实 CL bar1464 对齐**保留开放**——L1 合成自洽已确认，L2 真实数据对齐不由本审翻正（231 有效域边界标注，非缺陷）。须独立 L2 工位坐实 CL bar1464 不再发散，不预设结果。"
  epistemological_level: "实装层 L1 conclusive（管线正确性双重异质审确认）；理论假设层 L2 开放（真实对齐边界保留）。"
---

# 643 acceptance[1] bit-exact 假 PASS（L1 合成有效域膨胀）+ frontier 守卫投影有损 → cascade reset

> **结算说明（2026-06-30，genealogist）**：本号原 pending 经约束3（#56 独立工位实测）+ 约束4（#57 codex/gpt-5.5 异质审）双重确认 cascade reset 实装**非假绿**，conclusive 移 settled。**acceptance[1] L2 真实 CL bar1464 对齐保留为开放有效域边界**（L1 合成自洽已确认，L2 不由本审翻正——231 标注，非缺陷）。以下正文为原 pending 全文（发生史）。

## 一句话结论

acceptance[1] 报 bit-exact「PASS」但仅验 **L1 合成数据**（bit_exact_synthetic）；工位E 跑 **L2 真实 CL bar1464 发散**（git stash 自证 pre-existing parser bug）——L1 合成 PASS 掩盖真实缺口（231/625 同构）。根因分层：parser 末段原地变异是**古怪线段重划（第67/78课定义层正确，非 bug）**；bug 在 **caller 守卫只查段数回缩不查末段值改写** → 复用陈旧缓存中枢。工位F 第一版修复（比对 `project_to_units` 投影）**「碰巧过 8000bar」被 codex 异质审查走真实守卫入口复现反例抓出**（115→147 vs 147→140 内点：上级投影相等但 sub_moves 不同）——坐实「投影相等 ⟹ sub_moves 相等」是假前提（project_to_units 丢 sub_moves）。严格修复 = **cascade reset**（任一级 frontier 变异 → 该级 + 所有上级 reset），从根上消除「投影有损丢 sub_moves」整类问题。

## 两层声明膨胀

| 层 | 声明 | 实际 | 否证 | 谱系 |
|----|------|------|------|------|
| 1 有效域膨胀 | acceptance[1] bit-exact「PASS」 | 仅 L1 合成（bit_exact_synthetic）自洽 | 工位E L2 真实 CL bar1464 发散；git stash 自证 pre-existing | 231/625/631 |
| 2 守卫完备性误判 | 第一版守卫「投影相等⟹不变异」过 8000bar 即完备 | project_to_units 丢 sub_moves，投影相等不蕴含 sub_moves 相等 | codex 反例 115→147 vs 147→140 内点（走真实守卫入口） | 640 |

## 根因分层（定义正确 vs 守卫缺口）

- **parser 末段原地变异**：缠论古怪线段重划（第67课线段划分 / 第78课「笔破坏后未必形成线段破坏」）——**定义层正确的动态行为，非 bug**。删它=改定义（no-workaround）。
- **caller 守卫缺口**：只查段数回缩（segment count 减少）触发 reset，**不查末段值改写而段数不变**的合法变异路径 → 复用陈旧缓存中枢。这是守卫**有效域不完整**（bug）。
- **正确修复**：补全 caller 守卫（cascade reset），不动 parser 正确逻辑。

## 守卫完备性的假前提（640 纪律实例）

第一版 frontier_mutated 守卫比对 `project_to_units` 投影判断是否变异。但 `project_to_units`（recursive_tower.rs:383）产 `UnitRange`，**丢弃 sub_moves**（携坐标侧车，line 77-93）。当上级 RMove 投影 bit-identical 但其深嵌套 sub_moves（L2 cache subs）变化时，守卫漏 reset。

```
codex 反例（走真实守卫入口复现）：
  上级投影：115→147   vs   147→140 内点
  project_to_units(上级) bit-identical，但内点 sub_moves 不同
  → 第一版守卫判「未变异」→ 复用陈旧 L2 cache subs
```

8000bar PASS **只证该测试序列没踩到反例**，不证守卫完备——它依赖「投影相等 ⟹ sub_moves 相等」这一**假前提**。640 铁律：自评无漏洞=最该被异质审查。工位F 自评通过的守卫，由 codex 走真实入口复现反例抓出。

## cascade reset 为何是整类修复（非点修补）

第一版「比对投影」是点修补——每发现一个反例就要补一种投影字段比对，永远追不完（no-patch 补丁思维）。cascade reset 不比对投影：**任一级 frontier 变异 → 该级 + 所有上级无条件 reset**。它不依赖「投影是否相等」的判断，因此「投影有损丢 sub_moves」这一整类问题从根上消失。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（639-643）：639/640/641/642/本号。
- 1-hop：231/625/640/090/631/001/002。
- Hub：231（有效域）、640（异质审查纪律）。

### 张力1：vs 631——运行时实例 vs 注释措辞
631 = bit-exact 注释**措辞**膨胀基线（mod.rs/signal.rs 注释暗示 Lean parity）。本号 = bit-exact **acceptance PASS 报告**膨胀（运行时报告暗示 L2 真实对齐）。同模式（231）不同载体（注释 vs PASS 报告）。可分层，无矛盾——本号是 631 模式在运行时报告维度的延伸。

### 张力2：vs 641——不同轴（声明膨胀的两种）
641 = exp≈1 性能声明膨胀。本号 = PASS/守卫完备性膨胀。同为 090 声明膨胀族，不同轴（性能 vs 正确性）。可分层，无矛盾。

### 张力3：vs 642（同轮）——同源不同模块
642 = acceptance[2] coverage 自举 host key。本号 = acceptance[1] parser 守卫。两者同源（acceptance 链 L1 PASS 掩盖真实缺口）不同模块（coverage 自举 vs parser 守卫）。可分层，无矛盾。两者共同印证 acceptance 链多处 L1 PASS 掩盖 L2 真实缺口。

### 张力4：vs 缠论第67/78课（settled 域定义）——印证非冲突
本号印证末段重划是定义层正确行为（第78课），bug 在守卫不在 parser。不改定义，无矛盾。

### 张力5：vs 640——本号是其活实例
640 = 自评无漏洞=最该被异质审查。本号守卫层2 正是其活实例（工位F 自评过 8000bar 被 codex 反例抓出）。非矛盾，是应用。

### 递归运动结构完成检测（020）
- 第0层：本号写入（acceptance[1] L1 膨胀 + 守卫投影有损假前提 + cascade reset 整类修复 + 根因分层）。
- 第1层：本号 × 231/625 碰撞 → 有效域膨胀活实例（净新发现：守卫『投影相等⟹sub_moves相等』假前提是新的具体机制）。
- 第2层：本号 × 640 碰撞 → 异质审查纪律实例（净新发现降：自评通过须异质审查已知）。
- 第3层：本号 × 631/641/642 碰撞 → 声明膨胀族同模式（净新发现骤降=背驰：声明膨胀模式已多次实例化）。
- 涉及范围：scope₁(231/625 膨胀+守卫假前提) > scope₂(640 异质审查) > scope₃(声明膨胀族)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 修复=cascade reset（工位F 已实装 + codex 闭环）+ acceptance[1] 声明降级。本号是声明膨胀 + 守卫误判诊断（bias-correction），**不触发新 /escalate**（修复是行动类非选择类）。

## 回溯扫描（职责3）

- **231（settled）**：本号是有效域膨胀活实例，不否定，维持 settled。
- **625（settled）**：本号是其同构模式（parser 守卫维度），不否定，维持 settled。
- **640（settled）**：本号是其异质审查纪律活实例，不否定，维持 settled。
- **631（settled）**：本号是其 bit-exact 膨胀模式在运行时报告维度的延伸，不否定，维持 settled。
- **641/642（同轮，生成态）**：不同轴/不同模块，不破坏，维持生成态。
- **缠论第67/78课/001/002（settled）**：本号印证末段重划定义层正确，不改定义。
- **无 settled 被本号回溯破坏。**

## 结算后回溯扫描（2026-06-30，settle 时）

- 643 移 settled 不否定任何既有 settled：cascade reset 是 bit-exact 实装修复，231/625/640/631/001/002 全部维持 settled（本号是它们的活实例/延伸，非否定）。
- 与 566a（同批结算）无张力：566a 是 dag 索引层元数据对齐，643 是 theta_v0 parser 守卫层修复，模块正交，无共享对象。
- **结论**：无张力，无概念分离信号，无需 /escalate。643 实装层 conclusive 结算 + acceptance[1] L2 对齐开放边界保留（no-patch：开放项如实标注，不假装闭合）。
