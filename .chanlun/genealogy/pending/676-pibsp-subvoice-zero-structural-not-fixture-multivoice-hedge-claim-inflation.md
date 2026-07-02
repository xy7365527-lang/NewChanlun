---
id: "647"
number: 647
status: 生成态   # 同质代理质询（异质源 OpenAI GPT-5.5 429 insufficient_quota 不可用，降级）。结算待异质源恢复后补真异质验证 + 编排者 /ritual。
date: "2026-06-29"
type: bias-correction   # π^bsp「多声部对冲」声明膨胀诊断 + acceptance[1] parity 缺口坐实（631 #4 仍开放）
depends_on: ["231", "640", "090", "643", "645"]
related: ["631", "642", "644", "646"]
title: "π^bsp 子声部恒=0 是实装逻辑根因（出场优先 P3 + 跨carrier错配），非 fixture——commit 自陈『致命缺陷』诚实但『多声部对冲层未真测』⟹ 命题A『可分离』在实测上是 L0 代数可分非 L2 实测可分；acceptance[1] bit_exact 断言 incr==leg（L1 同引擎自洽）非 rust==Lean，coverage.rs ElementId 无 Lean parity assert（631 #4 缺口仍开放）"
negation_source: "同质代理质询（Claude，异质源 OpenAI GPT-5.5 429 不可用降级）+ 源码事实 L0：pi_bsp_timing.rs line 201-215（P3 出场优先平仓）/ line 227-243（active_root_by_carrier find 只比方向不比 carrier）/ line 261-266（buy/sell 分两循环但快照只取一次）；incremental.rs:186（assert incr_cls==leg_cls 同引擎两路径）；coverage.rs ElementId 全为结构 assert 无 Lean parity"
negation_form: "unclassified"   # 同质代理质询，非异质否定四形态（waiting/expansion/separation）

# 关键标注：本号是同质代理质询，不是异质否定。
# 异质源（OpenAI GPT-5.5 主 / gpt-5.5-pro 降级）429 insufficient_quota 不可用。
# 推理链由 Claude 代理执行（定义回溯+反例构造+源码事实坐实），价值低于真异质质询
# （无法发现同质模型家族共享盲区）。异质源恢复后须补真异质验证。

contradiction:
  description: "两个声明一致性问题。问题1（π^bsp 多声部对冲膨胀）：commit b29d3d4b19 自陈『π^bsp 忠实实装』+ 命题A（645）『π^cov ≠ π^bsp 可分离』，同时自陈『★致命缺陷(640披露): 短差子声部=0, 多空双开从未触发→退化纯根声部, 多声部对冲层未真测』。源码事实坐实子声部=0 是**实装逻辑根因非 fixture/输入问题**：(a) §11 close 优先（line 201-210/119 should_close）——active 声部遇反向证书先经 P3 出场被 closing 移除（line 213-215），开仓循环（line 261-266）在 D_t 平仓之后，故同 bar 反向证书先平掉同向根声部，active 集里不再有可作父的反向根声部；(b) open_new 内 active_root_by_carrier.find（line 240-243）只比对方向 `*pdir == -dir` **不比对 carrier**，且 buy_certs/sell_certs 分两个循环处理（line 261-266）而 active_root_by_carrier 在循环外只快照一次（line 227）——子声部父子关系建立条件实际上永不满足（先存在的反向 active 根声部已被 P3 平掉）。⟹ N_t^bsp=Σσ_v q_v 退化为纯根声部方向序列。命题A『可分离』在实测上仅是 **L0 代数可分**（定义 Σσ_v q_v ≠ Σs_e ε_e 不同公式）非 **L2 实测可分**（子声部=0 时 π^bsp 净额是否与 π^cov 投影产出不同序列**未验**）。声明『测到多声部对冲』= 声明膨胀（声明域=多声部对冲实测，实际域=纯根声部退化）——但 commit 已诚实标注『未真测』『致命缺陷』，故膨胀在『忠实实装』措辞 + 命题A『可分离』的实测等级未标注，非赤裸虚报。问题2（acceptance[1] parity 缺口，631 #4）：commit 自陈 acceptance[1] bit-exact ✅闭环。源码事实：bit_exact_per_bar/bit_exact_synthetic 断言对象 incremental.rs:186 `assert_eq!(incr_cls, leg_cls)` = 增量分类器 vs 全量重算（**同一 rust 引擎两路径自洽，L1**），**非 rust==Lean 跨实现 parity（L2）**。coverage.rs 的 ElementId 全部为 host/父/方向结构 assert（line 1868-1904 等），**无任何针对 Lean 输出的 parity 比对**。『bit-exact/镜像 Lean』措辞下无真实 Lean parity 测试——631 #4 gap 仍开放（643 已结算 acceptance[1] 是 L1 合成，本号坐实 Lean parity 维度同样缺）。"
  layer: 实装   # pi_bsp_timing.rs 状态机 + incremental.rs/coverage.rs 测试断言层。非定义冲突（命题A/π^bsp 定义本身无矛盾，问题在实测等级标注 + 实装逻辑使子声部不可达）。
  trigger: "Lead 派 gemini-challenger 工位质询 commit b29d3d4b19 的两声明：(1) π^bsp『多声部对冲』vs『子声部=0 退化』；(2) acceptance[1] bit-exact 是否掩盖 parity 缺口。异质源 429 不可用，同质代理质询坐实。"

definitions_involved:
  - name: "231 形式化有效域规则（L0/L1/L2）"
    version: ".claude/rules/formalization-validity-domain.md（settled）"
    role: "约束来源。命题A『可分离』须标 L 级：L0 代数可分（公式不同）信息增量零；L2 实测可分（子声部>0 产不同序列）才有否证力。子声部=0 时 L2 实测可分未验。acceptance[1] incr==leg 是 L1（管线自洽），rust==Lean parity 是 L2，后者无测试。"
  - name: "640 自评无漏洞=最该被异质审查"
    version: ".chanlun/genealogy/settled/640"
    role: "约束来源 + 应用。commit 自己引用 640 披露『致命缺陷』——这是 640 的正确应用（运动员自报缺陷）。本号是其延伸：『忠实实装』+ 命题A『可分离』的实测等级仍须异质审查（本应由异质源做，降级为同质代理）。"
  - name: "643 acceptance[1] L1 合成假 PASS（同轮）"
    version: ".chanlun/genealogy/pending/643"
    role: "姊妹号。643 = acceptance[1] L1 合成 vs L2 真实 CL 维度。本号 = acceptance[1] L1 同引擎自洽 vs L2 rust==Lean parity 维度。同源（acceptance[1] 多处 L1 掩盖 L2 缺口）不同对拍轴（真实数据 vs Lean 实现）。"
  - name: "645 命题A（π^cov ≠ π^bsp）"
    version: "本轮谱系 645（commit message 引用）"
    role: "被质询对象。命题A 代数成立（公式不同，L0），但『可分离』的实测有效域在子声部=0 时退化——本号不否定命题A 的 L0 代数内容，质询其『可分离』声明的 L 级标注（L0 vs L2）。"
  - name: "631 bit-exact 措辞膨胀基线（#4 parity 缺口）"
    version: ".chanlun/genealogy/settled/631"
    role: "约束来源。本号坐实 631 #4（parity 测试缺口）在 coverage.rs/incremental.rs 仍开放——bit_exact 断言是 incr==leg（同引擎）非 rust==Lean。"
  - name: "090 严格性语法规则（声明膨胀禁止）"
    version: ".chanlun/genealogy/settled/090"
    role: "约束来源。『测到多声部对冲』『忠实实装』在子声部=0 时是声明膨胀（禁止模式5）；但 commit 已标注『未真测』减轻——膨胀残留在『忠实』措辞 + 命题A 实测等级未标。"

resolution:
  type: 待结算   # 同质代理质询坐实两问题；异质源恢复后补真异质验证；编排者裁决声明降级范围。
  description: "诊断（同质代理质询）：(1) π^bsp 子声部=0 是实装逻辑根因（P3 出场优先 + open_new 跨 carrier 方向匹配 + buy/sell 分循环单次快照）使子声部父子关系不可达，非 fixture/输入问题。修复路径（行动类，待 Lead 派工位）：open_new 内 find 须按 carrier 匹配 + 子声部触发须在 P3 出场**之前**评估反向证书（或区分『出场反向证书』与『开子声部反向证书』语义）。修复前『多声部对冲』『命题A 可分离』须标注为 L0 代数可分（实测 L2 未验，子声部=0 退化纯根声部）。(2) acceptance[1] bit_exact 断言降级标注：incr==leg 是 L1 同引擎自洽，非 rust==Lean parity（L2）；『镜像 Lean/bit-exact』措辞须加 L 级标注或补真 Lean parity 测试（631 #4）。本号是同质代理质询，价值低于真异质——异质源（OpenAI GPT-5.5）恢复后须补真异质质询坐实/否定。"
  decided_by: 待裁决   # 同质代理质询坐实源码事实；声明降级范围 + 子声部修复授权待编排者/Lead；真异质验证待异质源恢复

negated:
  description: "(1) π^bsp『忠实实装多声部对冲』在当前实装下成立（子声部=0 退化纯根声部，多声部对冲层从未执行）。(2) 命题A『π^cov 与 π^bsp 可分离』是 L2 实测可分（子声部=0 时两者实测序列是否不同未验，仅 L0 公式不同）。(3) acceptance[1] bit_exact『闭环』暗示 rust==Lean parity（实际 incr==leg 同引擎 L1）。(4) 子声部=0 是 fixture/输入数据问题（实为实装逻辑使父子关系不可达）。"
  why_negated: "(1) 源码 L0：P3 出场优先（line 119/208）在开仓（line 261）之前平掉反向 active 根声部，子声部 find（line 240）无可匹配父。commit 自陈『子声部=0/未真测』印证。(2) 231：L0 公式不同≠L2 实测可分；子声部=0 时 π^bsp=纯根声部净额，与 π^cov 投影实测对拍未做。(3) incremental.rs:186 断言 incr_cls==leg_cls（同 rust 引擎增量 vs 全量）；coverage.rs ElementId 无 Lean parity assert——631 #4。(4) active_root_by_carrier 在 P3 平仓后快照（line 227 在 line 213-215 之后），逻辑使可作父的反向根声部恒被先平；非数据问题。"

new_output:
  definitions:
    - "π^bsp 子声部恒=0 根因 = 实装逻辑（P3 出场优先 + open_new 跨carrier方向匹配 + buy/sell分循环单次快照），非 fixture——多声部对冲层从未执行，N^bsp 退化纯根声部方向序列。"
    - "命题A『可分离』L 级澄清：L0 代数可分（Σσ_v q_v ≠ Σs_e ε_e 公式不同）成立；L2 实测可分（子声部=0 时两者产不同序列）未验。声明须标 L 级。"
    - "acceptance[1] bit_exact 断言对象澄清：incr_cls==leg_cls（同 rust 引擎增量 vs 全量，L1 自洽），非 rust==Lean（L2 parity）。631 #4 parity 缺口在 coverage.rs/incremental.rs 仍开放。"
  code_changes: "无代码改动（质询诊断）。修复路径（行动类待 Lead）：(a) pi_bsp_timing.rs open_new find 按 carrier 匹配 + 子声部反向证书在 P3 出场前评估；(b) acceptance[1] bit_exact 加 L 级标注或补 Lean parity 测试。"
  orchestration_changes: "方法论：①『忠实实装 X』须验 X 的核心机制实际执行（子声部=0 时多声部对冲未执行≠忠实实装多声部对冲）。②『可分离/不同』声明须标 L0 代数 vs L2 实测。③bit_exact 断言须标对象（同引擎自洽 L1 vs 跨实现 parity L2）——『镜像 Lean』措辞不证 Lean parity 测试存在。④异质源不可用时同质代理质询须标注降级，不冒充异质否定（negation_source≠heterogeneous）。"

impact:
  affected_modules:
    - "rust/src/bin/pi_bsp_timing.rs → open_new（line 233-259）子声部父匹配逻辑 + P3 出场/开仓顺序（line 201-266）；子声部恒=0 使 §6/§16 多空双开未执行。"
    - "rust/src/theta_v0/backtest/incremental.rs:186 → bit_exact 断言 incr==leg（L1 同引擎），须标注非 Lean parity。"
    - "rust/src/theta_v0/strategy/coverage.rs → ElementId 结构 assert 无 Lean parity（631 #4）。"
  affected_definitions:
    - "231（settled）：本号是 L 级标注缺失活实例（命题A 可分离 + bit_exact 对象），维持 settled。"
    - "640（settled）：commit 自报『致命缺陷』是其正确应用；本号延伸（措辞膨胀须异质审查），维持 settled。"
    - "645（命题A）：L0 代数内容不否定，质询其『可分离』L 级标注。"
    - "631（settled）：本号坐实 #4 parity 缺口仍开放，维持 settled。"
  downstream_implications:
    - "π^bsp 在子声部修复前，CL/BTC Sharpe≤0 结果只反映纯根声部退化形态，不反映多声部对冲——不得用作『多声部对冲无 alpha』的 L2 结论。"
    - "命题A『可分离』须补 L2 实测（修复子声部后对拍 π^cov vs π^bsp 序列）或降级为 L0 代数可分。"
    - "acceptance[1] bit_exact 闭环须补 Lean parity 测试（631 #4）或标注为 L1 同引擎自洽。"
    - "本号是同质代理质询——异质源（OpenAI GPT-5.5）恢复后须补真异质质询坐实/否定（同质代理无法发现模型家族共享盲区）。"

related_records:
  parent: "231（L 级标注）+ 643（acceptance[1] L1 掩盖 L2，姊妹号）"
  children: []
  related:
    - "643号（同轮）：acceptance[1] L1 合成 vs L2 真实 CL——本号 acceptance[1] L1 同引擎 vs L2 Lean parity，同源不同对拍轴"
    - "645号：命题A（π^cov≠π^bsp）——本号质询其『可分离』L 级"
    - "631号：bit-exact 措辞膨胀 #4 parity 缺口——本号坐实仍开放"
    - "640号：自评须异质审查——commit 自报致命缺陷是正确应用，本号延伸措辞膨胀维度"
    - "642号/644号（同轮）：acceptance[2] 缺口——本号 acceptance[1]/π^bsp，同源 acceptance 链 L1 掩盖 L2"

epistemological_levels:
  - proposition: "π^bsp 子声部恒=0（多声部对冲层未执行，退化纯根声部）"
    level: "L0（源码事实：P3 出场优先 line 119/208 + open_new find line 240 + 单次快照 line 227）"
    increment: "高：坐实是实装逻辑根因非 fixture，多空双开父子关系不可达"
  - proposition: "命题A『π^cov 与 π^bsp 可分离』"
    level: "L0 代数可分（公式 Σσ_v q_v ≠ Σs_e ε_e 不同）已证；L2 实测可分（子声部=0 时序列是否不同）未验"
    increment: "中：澄清 L 级——L0 成立不蕴含 L2，声明须标"
  - proposition: "acceptance[1] bit_exact 断言对象 = incr_cls==leg_cls"
    level: "L0（源码事实 incremental.rs:186）+ L1（同 rust 引擎增量 vs 全量自洽）"
    increment: "高：坐实非 rust==Lean parity（L2），631 #4 缺口仍开放"
  - proposition: "本号为同质代理质询（异质源 429 不可用）"
    level: "标注：非异质否定，价值低于真异质，待异质源恢复补验"
    increment: "零信息增量声明（诚实标注降级，不冒充）"
---

# 647 π^bsp 子声部恒=0 是实装根因 + acceptance[1] bit_exact 非 Lean parity（同质代理质询，异质源降级）

## 一句话结论

commit b29d3d4b19 两声明经同质代理质询坐实：(1) **π^bsp 子声部恒=0 是实装逻辑根因**（§11 close 优先在开仓前平掉反向 active 根声部 + open_new find 跨 carrier 只比方向 + buy/sell 分循环单次快照），使 §6/§16 多空双开父子关系**不可达**——多声部对冲层从未执行，N^bsp 退化纯根声部方向序列。commit 自陈『致命缺陷/未真测』**诚实**，膨胀残留在『忠实实装』措辞 + 命题A『可分离』的 L 级未标（L0 代数可分 ≠ L2 实测可分）。(2) **acceptance[1] bit_exact 断言对象是 `incr_cls==leg_cls`（同 rust 引擎增量 vs 全量，L1 自洽），非 rust==Lean parity（L2）**；coverage.rs ElementId 全为结构 assert，无 Lean parity——631 #4 缺口仍开放。

## 重要标注：本号是同质代理质询，非异质否定

异质源（OpenAI GPT-5.5 主 / gpt-5.5-pro 降级）429 insufficient_quota 不可用。推理链由 Claude 代理执行（定义回溯 + 源码事实坐实），**价值低于真异质质询**——无法发现同质模型家族共享盲区。异质源恢复后须补真异质验证。`negation_source` 标注为同质代理，`negation_form: unclassified`（非异质四形态）。

## 子声部恒=0 的实装逻辑根因（L0 源码事实）

```
每 bar 顺序（pi_bsp_timing.rs line 196-272）：
  1. P3 出场（line 201-210）：active 声部遇反向证书 → should_close（line 119 exit_cert）→ closing
  2. D_t 应用（line 213-215）：active.remove(closing) ← 反向根声部在此被平掉
  3. active_root_by_carrier 快照（line 227）← 此时反向根声部已不在 active
  4. open_new（line 261-266）：find(*pdir == -dir)（line 240）← 无可匹配父 → parent=None → 根声部
```

三重阻断使子声部父子关系不可达：
- **顺序**：P3 出场（close 优先 §11）在开仓前平掉反向 active 根声部
- **匹配**：find（line 240）只比方向 `*pdir == -dir`，不比 carrier（跨 carrier 错配，即使有父也错）
- **快照时机**：active_root_by_carrier 在 D_t 平仓后快照（line 227 在 line 213 之后）

⟹ `n_children`（line 345 `parent.is_some()` 计数）恒=0，N^bsp=Σσ_v q_v 退化为纯根声部方向净额。

## acceptance[1] bit_exact 断言对象（L0/L1）

```
incremental.rs:186  assert_eq!(incr_cls, leg_cls, "synthetic bar {i}: classification bit-exact 破裂")
                    ↑ 增量分类器 vs 全量重算 —— 同一 rust 引擎两路径（L1 自洽，cached==nocache）
                    ✗ 不是 rust==Lean 跨实现 parity（L2）
coverage.rs         assert_eq!(parent, ...) / assert_eq!(sigma, ...) —— host/父/方向结构断言
                    ✗ 无任何针对 Lean 输出的 parity 比对（631 #4 gap 仍开放）
```

『bit-exact/镜像 Lean』措辞下**无真实 Lean parity 测试**。643 已结算 acceptance[1] 是 L1 合成（真实数据维度）；本号坐实 Lean parity 维度同样缺。

## 张力检查（019d/020）

- vs 643（同轮）：643=acceptance[1] L1 合成 vs L2 真实 CL；本号=acceptance[1] L1 同引擎 vs L2 Lean parity。同源不同对拍轴，可分层，无矛盾。
- vs 645（命题A）：不否定 L0 代数内容（公式不同），质询『可分离』L 级。无矛盾。
- vs 631（#4 parity 缺口）：本号坐实仍开放，应用非否定。
- vs 640：commit 自报致命缺陷是其正确应用；本号延伸（措辞膨胀维度）。无矛盾。
- **无 settled 被本号回溯破坏。** 本号是声明等级标注诊断（bias-correction），修复是行动类（待 Lead），声明降级范围 + 真异质验证待编排者/异质源恢复。
