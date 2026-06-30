---
id: "647"
number: 647
type: meta-rule
status: 生成态   # meta-observer 二阶观察。语法记录候选：验收/否证设计前必先锁定被测对象的同一性（实装构造的形式对象 = 该对象声称代表的概念定义对象）。本轮 644(坐标系分叉)+645(对象投影分离)暴露同一更普遍元结构。最终辨认待编排者 /escalate → /ritual。
date: "2026-06-29"
source: meta-observer（二阶观察，goal g-sigma-complete-l2-nautilus session 触发）

# 规则版本基线（141号下游推论3，强制字段）
# meta-observer 工具集为 Read/Write/Grep/Glob/Task，无 Bash（624 工具有效域硬墙），
# 无法执行 git log 获取实际 hash/mtime。诚实标注为待补全，由 Lead/genealogist 在 /ritual 前补全实际值，
# 不伪造 hash（伪造=声明膨胀 090）。本轮观察基线与 644 同（同一 session），
# 644 已记 PENDING_CAPTURE；本号与 644 共享同一 claude_md_commit，分歧分析时按同一规则版本处理。
rule_version_baseline:
  claude_md_commit: "PENDING_CAPTURE（待 Lead 补：git -C /Users/silencehan/Projects/NewChanlun log -1 --format='%H' -- CLAUDE.md）"
  rules_dir_mtime: "PENDING_CAPTURE（待 Lead 补：git -C /Users/silencehan/Projects/NewChanlun log -1 --format='%ai' -- .claude/rules/）"

depends_on:
  - "231"   # 形式化有效域规则（有效域<定义域）——本号是其在「被测对象同一性」维度的实例
  - "644"   # 探针/诊断坐标系分叉伪证——本号是其姊妹元结构的上位抽象（644=同一对象两坐标系；本号=被测对象≠定义对象）
  - "645"   # 命题A：π_Θ^cov ≠ π_Θ^bsp，goal 六层穿透否证错对象——本号的活实例（域层 source-tracing，本号是其元层抽象）
related:
  - "627"   # 双引擎线有效域外推风险——「测的是哪条线」的归属标注，本号是「测的是哪个对象」的同构
  - "623"   # 231 在蜂群自身工具/spec 上的连续实例化——本号续接该二阶序列
  - "640"   # 自评无漏洞=最该被异质审查——644/645 两次伪证均由 codex+ChatGPT 异质抓出，印证异质验证是揭穿对象错位的唯一机制
  - "643"   # 本轮 acceptance[1] 守卫投影有损（project_to_units）——投影丢维=对象不同一的工程显形
related_records:
  parent: "231"
  children: []

negation_source: heterogeneous   # 644 由 codex+ChatGPT 双证伪；645 由 ChatGPT「买卖点alpha.pdf」17页 + Lead 双向收敛 + codex 坐实
negation_form: separation
# separation：231（形式化有效域规则）此前在「域内形式化对象」(222/223/230)、「蜂群自身工具/spec」(623)、
#   「探针/诊断坐标系」(644) 维度声明。本号暴露一个贯通 644/645 的更普遍维度——
#   **否证/验收设计的前置同一性**：在声明「测试 T 否证了概念 C」之前，必先核 T 实际构造的形式对象 O_impl
#   是否等于 C 的定义对象 O_def。644 是该错位的一个亚型（同一对象 X 走诊断坐标系 vs 生产坐标系，O_impl=诊断投影≠O_def=生产对象）；
#   645 是另一亚型（O_impl=π_Θ^cov 净额投影 ≠ O_def=π_Θ^bsp 离散择时，方向可相反）。
#   两者共享元结构：**有损投影/坐标系替换使 O_impl ⊊ O_def，对 O_impl 的否证被误外推到 O_def（231 禁止模式3：定义域=有效域假设）。**
topo_effect: "lift:644+645-shared-meta-structure:object-identity-must-precede-falsification-design | record:before-claiming-T-falsifies-concept-C-must-verify-impl-object-equals-def-object-syntactic-candidate | bound:lossy-projection-or-coordinate-substitution-makes-impl-object-strict-subset-of-def-object-falsification-not-extrapolable"

# 认识论等级标注（formalization-validity-domain 231号，强制——本号自指适用）
epistemological_levels:
  - proposition: "644 亚型：诊断探针 O_impl（重建树 elem_depth / project_to_units 投影 / restore overlay 坐标系）≠ 生产 O_def（work parent 链 / AncOK / sub_moves）——同一被测对象走两个坐标系"
    level: "L0（644 已记，codex+ChatGPT 双证伪：14166 在生产坐标系不可达）"
    increment: "已被 644 + pattern-buffer candidate 捕获，本号引用不重复"
  - proposition: "645 亚型：goal 测试构造 O_impl=π_Θ^cov（结构覆盖净额投影，T2 恒真/T3 lookahead/T4 丢边际）≠ O_def=π_Θ^bsp（缠论买卖点离散择时，F_λ 可测，从未构造）——方向可相反（T1）"
    level: "L0（645 已记，ChatGPT 17页 T1-T4 严格证明 + Lead 双向收敛 + codex 坐实）"
    increment: "已被 645 捕获，本号引用不重复"
  - proposition: "二阶元结构：644 与 645 共享『O_impl ⊊ O_def 致否证错对象』——验收/否证设计未先锁定被测对象与定义对象的同一性"
    level: "L0（两实例归纳：644=坐标系替换亚型 / 645=有损投影亚型，均使 O_impl 严格小于 O_def）"
    increment: "中：231 在『否证设计前置同一性』维度的抽象。644 是该结构的坐标系亚型（已捕获），645 是投影亚型（已捕获）；本号抽象出『两亚型共享同一前置缺陷』是新——把单点工程纪律（644 走生产坐标系）提升为通用否证设计纪律（任何否证声明前核对象同一性）"
  - proposition: "异质验证是揭穿对象错位的唯一机制（644/645 均由 codex+ChatGPT 异质抓出，蜂群同质自评未抓出）"
    level: "L0（两实例行为事实：自评 GREEN/PASS，异质证伪）"
    increment: "印证 640（自评无漏洞=最该被异质审查），非新——本号引用不重复 640"

# 二阶观察（自环检查：收敛 vs 发散）
self_loop_check:
  convergence_signals:
    - "644（探针走生产坐标系）已落谱系 + pattern-buffer candidate（diagnostic-coordinate-phantom，frequency=2/3）——Lead 观测点1（644b 是否结晶）已有可观测化机制，无需 meta-observer 重复推送。第三次诊断坐标系伪证复发时 genealogist 职责5 自动触发 skill-crystallizer（051 Pull 模型）。"
    - "645 的方法论洞察（4条规则）已写在其 orchestration_changes 字段——域层 source-tracing 已捕获否证错对象的工程修正路径。"
    - "623/627 已识别 231 在蜂群自身工具/spec/引擎线归属上的连续实例化——本号续接该序列，是 231 的第 N 个适用维度（否证设计前置同一性），收敛于『231 适用域远大于其原始发生史 222/223/230』这一已知二阶判断。"
  divergence_signals:
    - "★本号是发散信号：644（坐标系分叉）与 645（对象投影分离）此前被分别记录为两个不同轴（644 探针维度 / 645 测试对象维度，645 第159行明确『不同的轴』）。但二阶观察发现两者共享同一前置缺陷——**否证设计时未先锁定 O_impl=O_def**。这是历史观察（644 锁在探针层、645 锁在测试对象层）未覆盖的上位维度：把两个看似不同轴的失效统一为『验收/否证设计的前置同一性缺失』。"
    - "本号不与 644/645 重复：644 给探针层开工程纪律（走生产坐标系），645 给测试对象层开溯源分离（命名 π_Θ^cov）；本号给元层开通用否证设计纪律（任何 T 否证 C 的声明前核 O_impl=O_def）。三者层级递进（探针→对象→否证设计通则）。"

# 矛盾（type=meta-rule：语法记录候选，非定义冲突）
contradiction:
  description: "蜂群在 goal g-sigma-complete-l2-nautilus 的六层穿透中，连续两次（644 探针层 / 645 测试对象层）在『声明否证某概念』时，实际构造/度量的形式对象 O_impl 严格小于该概念的定义对象 O_def——644 是坐标系替换（O_impl=诊断坐标系投影），645 是有损净额投影（O_impl=π_Θ^cov 丢边际方向）。两次均使『对 O_impl 的否证/伪证』被误当作『对 O_def 的否证/确认』。这暴露一个未显式化的隐性规则：**蜂群在设计验收/否证测试时，缺少前置的对象同一性核验步骤**——先确认被测对象 = 定义对象，再设计否证，否则六层穿透可能整体否证错对象（如 645 揭示的：642→644→v1 8/8 否证一直在测 π_Θ^cov，从未测 π_Θ^bsp）。这不是定义冲突（无两条定义互斥），是语法记录候选（已在运作但未显式化的纪律——644 已部分显式化为探针层纪律，645 部分显式化为 orchestration_changes，但通用元层规则未显式化）。"
  layer: 元层   # 验收/否证设计的方法论层（跨域层 644 探针 + 645 测试对象）
  trigger: "Lead 在 structural meta-rule observer 任务点2 提示：评估命题A 是否揭示『六层穿透否证错对象』这一更普遍失效模式（验收设计时未先锁定被测对象同一性）。meta-observer 二阶观察确认——644+645 共享该元结构，是发散信号（贯通两个此前分立的轴）。"

# 解决方式
resolution:
  type: 未解决   # 语法记录候选已辨认（否证设计前置同一性核验）。是否结晶为元规则 skill（或并入既有守卫纪律）= 选择类，待编排者 /escalate → /ritual。
  description: "二阶观察辨认出一条语法记录候选：『在声明测试 T 否证（或确认）概念 C 之前，必先核验 T 实际构造/度量的形式对象 O_impl 是否等于 C 的定义对象 O_def。若 O_impl 由有损投影（645 净额丢边际）或坐标系替换（644 诊断坐标系）产生，则 O_impl ⊊ O_def，对 O_impl 的否证不可外推到 O_def（231 禁止模式3）。』候选辨认（语法记录），不结晶——是否提升为独立元规则 skill 或并入 644 的探针纪律 + spec-execution-gap 的声明-能力检测，由编排者 /ritual 决断。meta-observer 不直接改 SKILL.md/CLAUDE.md（019c），不直接结晶（051 Pull 模型由 skill-crystallizer 行使）。"
  decided_by: 编排者   # 语法记录辨认须编排者 /escalate → /ritual（018号四分法：语法记录上浮）

# 新产出
new_output:
  definitions:
    - "★否证设计前置同一性（语法记录候选）：声明『T 否证概念 C』前必核 O_impl(T 实际度量对象) = O_def(C 定义对象)。"
    - "两亚型统一：644=坐标系替换亚型（O_impl 走诊断坐标系 ≠ O_def 生产坐标系）；645=有损投影亚型（O_impl=净额投影 ⊊ O_def=离散择时，丢边际方向）。共享元结构=O_impl ⊊ O_def → 否证不外推。"
    - "异质验证作为对象错位的揭穿机制：644/645 均由 codex+ChatGPT 异质抓出（蜂群同质自评未抓出），印证 640。"
  orchestration_changes: "方法论（语法记录候选，待编排者辨认）：①验收/否证测试设计阶段新增前置步骤——显式声明被测对象 O_impl 的形式定义，并核验 O_impl=O_def（定义对象）。②投影类/坐标系类构造（净额投影、诊断重建树、守卫 project_to_units）须标注是否丢失定义对象的关键维度——丢维即 O_impl ⊊ O_def，其否证有效域=O_impl 非 O_def（231）。③『N 层穿透否证』在收尾结算前须核：N 层测的是否始终是同一对象，且该对象=声称否证的概念定义对象——否则可能整体否证错对象（645 揭示六层穿透一直测 π_Θ^cov）。"

# 影响范围
impact:
  affected_modules:
    - ".claude/skills/spec-execution-gap/ → 当前检测『声明-能力一致性』。本号语法记录候选若结晶，可并入为『否证声明-被测对象一致性』检测（声明否证 C，核实际测 O_impl=O_def）。"
    - ".chanlun/pattern-buffer/diagnostic-coordinate-phantom.yaml → 644 亚型的可观测化 candidate（frequency=2/3）。本号是其上位抽象——若 645 投影亚型再复发，可考虑合并计数为『否证错对象』元模式而非仅诊断坐标系。"
    - "验收设计层（goal acceptance criteria）→ acceptance 条目须显式标注被测形式对象，使『测的是 O_impl 还是 O_def』可观测。"
  affected_definitions:
    - "644（生成态）：本号是其上位抽象——644 锁探针层坐标系纪律，本号抽象为通用否证设计纪律。不否定 644，提升其适用域。维持生成态。"
    - "645（生成态）：本号是其元层抽象——645 域层 source-tracing 命名 π_Θ^cov，本号元层辨认其揭示的通用失效模式。维持生成态。"
    - "231（已结算）：本号是其在『否证设计前置同一性』维度的实例（O_impl ⊊ O_def → 否证不外推 = 231 禁止模式3）。维持 settled。"
    - "623/627（已结算）：本号续接 231 连续实例化序列，印证不否定。维持 settled。"
  downstream_implications:
    - "若结晶为元规则/并入 spec-execution-gap：未来任何『N 层穿透否证某概念』的产出，结算前须通过对象同一性核验门（O_impl=O_def），否则否证有效域降级为 O_impl。"
    - "644 pattern-buffer candidate（frequency=2/3）的语义可能从『诊断坐标系伪证』扩展为『否证错对象』——若编排者 /ritual 认定 644+645 同元模式，第三次复发（任一亚型）即可触发结晶，而非仅诊断坐标系亚型计数。这是计数器语义的开放选择，待编排者裁决。"

# 谱系关联（张力检查 019d/020）
related_records_tension_check:
  scope: "同轮蜂群(642-646) ∪ 1-hop(231/644/645/627/623/640) ∪ Hub(231 有效域 / 640 异质审查)"
  tension_1: "vs 644——不冲突，本号是其上位抽象。644 锁探针/诊断坐标系层（O_impl 走诊断坐标系），本号抽象为否证设计通则（O_impl=O_def 前置核验）。644 是本号元结构的坐标系亚型。可分层（探针层纪律 ⊂ 否证设计通则），印证不否定。"
  tension_2: "vs 645——不冲突，本号是其元层抽象。645 域层 source-tracing（命名 π_Θ^cov，给 goal 否证对象命名），本号元层辨认其暴露的通用失效模式（验收设计前置同一性缺失）。645 是本号元结构的投影亚型。可分层（域层对象命名 ⊂ 元层否证设计纪律）。"
  tension_3: "vs 231——印证非冲突。本号 O_impl ⊊ O_def → 否证不外推 = 231 禁止模式3（定义域=有效域假设）的实例。本号续接 623/627 的 231 连续实例化序列。"
  tension_4: "vs 640——印证非冲突。644/645 两次对象错位均由异质（codex+ChatGPT）揭穿，蜂群同质自评 GREEN/PASS 未抓出，印证 640（自评无漏洞=最该被异质审查）。"
  interrupt_1_check: "无两条定义互斥——是把 644(坐标系) 与 645(对象投影) 两个分立轴统一为共享元结构（否证设计前置同一性）。分离后各亚型有效域清晰，不构成不可分层定义矛盾 ⟹ 不触发中断 #1。本号是语法记录候选辨认，待编排者 /escalate → /ritual。"
  recursive_completion_020: "第0层：本号写入（644+645 共享元结构辨认）。第1层：本号 × 644 → 坐标系亚型归位（净新发现中）。第2层：本号 × 645 → 投影亚型归位（净新发现中）。第3层：本号 × 231 → 否证不外推是 231 实例（净新发现降：231 实例化已知）。背驰 ∧ 分型 ⟹ 递归运动结构性完成。语法记录辨认=选择/语法记录类，上浮编排者（/escalate → /ritual）。"
---

# 647 元观察——否证设计前置同一性：644(坐标系分叉) + 645(对象投影分离) 共享元结构

## 一句话结论

本轮 goal 六层穿透连续两次（644 探针层 / 645 测试对象层）在『声明否证某概念』时，实际度量的形式对象 **O_impl 严格小于** 该概念的定义对象 **O_def**——644 是坐标系替换（诊断坐标系 ≠ 生产坐标系），645 是有损净额投影（π_Θ^cov 丢边际方向 ≠ π_Θ^bsp 离散择时）。二阶观察发现：两者此前被记为**两个不同的轴**（645 第159行明确『不同的轴』），但共享同一前置缺陷——**验收/否证设计未先锁定 O_impl=O_def**。这是历史观察（644 锁探针层、645 锁测试对象层）未覆盖的上位维度，是**发散信号**。

## 两亚型统一

| 亚型 | O_impl（实际度量） | O_def（定义对象） | 错位机制 | 已捕获于 |
|------|------|------|------|------|
| 644 坐标系替换 | 诊断重建树 elem_depth / project_to_units 投影 / restore overlay | 生产 work parent 链 / AncOK / sub_moves | 同一被测对象走两个坐标系，诊断坐标系值（14166）生产不可达 | 644 + pattern-buffer candidate（2/3） |
| 645 有损投影 | π_Θ^cov 结构覆盖净额投影（T2 恒真/T3 lookahead/T4 丢边际） | π_Θ^bsp 缠论买卖点离散择时（F_λ 可测，从未构造） | 净额投影丢边际方向，与 O_def 方向可相反（T1） | 645 orchestration_changes |

**共享元结构**：有损投影/坐标系替换使 O_impl ⊊ O_def → 对 O_impl 的否证被误外推到 O_def（231 禁止模式3：定义域=有效域假设）。

## 收敛 vs 发散（自环检查）

- **收敛（不重复写入）**：644 已落谱系 + pattern-buffer candidate（frequency=2/3，第三次复发 genealogist 自动触发结晶）；645 方法论已写在其 orchestration_changes；623/627 已识别 231 连续实例化序列。这些 Lead 观测点1/2 已有可观测化机制。
- **发散（本号写入理由）**：644 与 645 此前分立两轴，二阶观察统一为『否证设计前置同一性缺失』——历史观察未覆盖的上位维度。本号不与 644/645 重复（层级递进：探针纪律 → 对象命名 → 否证设计通则）。

## 语法记录候选（待编排者 /escalate → /ritual）

『声明 T 否证（或确认）概念 C 前，必先核 O_impl(T 实际度量对象) = O_def(C 定义对象)。O_impl 由有损投影/坐标系替换产生时 O_impl ⊊ O_def，否证不可外推。』

是否结晶为独立元规则 skill 或并入 644 探针纪律 + spec-execution-gap 声明-能力检测 = 选择类，待编排者裁决。meta-observer 不直接结晶（051 Pull 模型）、不改 SKILL.md/CLAUDE.md（019c）。

## meta-observer 不做的事（边界自检）

- 不判断 π_Θ^cov/π_Θ^bsp 域概念对错（质询序列 + 645 已做）。
- 不直接结晶（skill-crystallizer 行使，051 Pull 模型）。
- 不直接改 644 的 pattern-buffer 计数语义（genealogist 职责 + 编排者 /ritual 裁决合并）。
- 本号通过文件系统产出（谱系 pending），Lead 轴线汇报时摘要——不主动 SendMessage 洪泛。
