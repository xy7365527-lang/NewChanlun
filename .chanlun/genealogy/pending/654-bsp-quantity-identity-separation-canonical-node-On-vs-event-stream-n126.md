---
id: 654
number: 654
status: 生成态   # genealogist 结构记录：BSP 量身份分离（规范树节点数 O(n) ≠ 事件流/候选/重评估计数 n^1.26）首次显式化 + acceptance[4] O(n) 下界主张 Ω(n^1.26) 精确化（只对事件流计数成立，规范 BSP=O(n) 未被否证）。codex 异质独立确认量身份分离，无新增 gap。最终结算待编排者 /ritual。
date: "2026-06-30"
type: source-tracing   # 溯源分离：把「BSP 总数」当作单一渐近量的对象，分离为「规范树节点数（O(n)，M26 par 单值树蕴含）」与「事件流/候选/重评估计数（实测 n^1.26）」——两个不同范畴的量，被混淆产生「结构否证 O(n)」假象。
depends_on: ["231", "653"]
related: ["638", "653", "222", "223", "230", "mutex-derive-result", "mutex-prove-result"]
# memory 关联（Lead 维护）：project_bsp_event_stream_osb_wall（第八墙=事件流全量 marshal+扫描，本号结构层根因）
negation_source: "homogeneous（蜂群 mutex-derive #23 §7 从 M26 元素树结构判定）+ heterogeneous（codex 异质独立确认量身份分离 + ≥3 规则本身不证 O(n) 须不相交规范树前提，真异质增量）"
negation_model: "codex（OpenAI，独立证 engine O(n²) 瓶颈下界 Ω(n^1.26) + 量身份分离确认）"
negation_form: "separation"   # 把「BSP 总数渐近」分离为两个不同范畴的量：规范树节点数（O(n）)与事件流/候选/重评估计数（n^1.26）。两者不可互相否证——前者 L0 结构，后者 L2/L3 经验（事件流计数）。
title: "★BSP 量身份分离（首次显式化，codex 异质确认）：规范树节点数 = O(n)（L0 结构定理，M26 par 单值树+≥3 规则+不相交划分蕴含，几何级数收敛）≠ 实测 n^1.26（事件流/候选图/版本化重算计数，L2/L3 经验量）。两量混淆产生「结构否证 O(n)」假象。acceptance[4] O(n) 下界主张 Ω(n^1.26) 须精确化——只对「引擎枚举/处理超线性 BSP 事件/候选」成立；缠论规范 BSP 子集 O(n) 未被否证。O(n) 引擎真突破口=引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=工程问题），非缠论结构否证"

# separation：本号把一个被混为一谈的量（「BSP 总数渐近」）分离为两个不同范畴的量——
#   规范树节点数（structural node count）：在 M26 元素树 T=(E,par) par 单值（每节点唯一父）+
#     ≥3 规则（级别 k+1 由 ≥3 个级别 k 构成）+ 不相交划分（一个级别-k 元素只喂一个父）下，
#     N_{k+1}≤N_k/3，级别数 O(log₃ n)，|E|≤n·Σ(1/3^k)=O(n)（几何级数收敛）。BSP⊆E ⟹ 规范 BSP=O(n)。
#   事件流/候选/重评估计数（event/candidate/version count）：实测 n^1.26，来自 (a) 中枢可重构/延伸，
#     一中枢随时间 emit 多个 BSP（事件计数）；(b) 候选中枢重叠/共享次级别（一元素属多个候选，破坏不相交
#     划分，引擎数重叠候选图非树）；(c) 每 bar 流式重评估 BSP，计 O(中枢数×bar 数) 事件非 O(中枢数) 节点。
#   两量不同范畴（L0 节点 vs L2/L3 事件流），不可互相否证。混淆产生「实测 n^1.26 否证 O(n) 结构」假象。

topo_effect: "separate:BSP-total-asymptotic:{canonical-tree-node-count=O(n)[L0,M26-par-single-valued-tree+disjoint-partition,geometric-convergence]|event-stream-candidate-version-count=n^1.26[L2/L3,empirical]} | precise:acceptance4-Ω(n^1.26)-lower-bound=only-for-engine-event/candidate-enumeration-NOT-canonical-BSP-subset | record:O(n)-engine-breakthrough=emit/process-each-canonical-BSP-once(event-dedup/incremental=engineering)-NOT-chanlun-structural-falsification | bound:O(n)-structural-NOT-falsified-by-n^1.26-empirical(category-difference)"

# 矛盾/分离（type=source-tracing）
contradiction:
  description: |
    另一线 codex 证 engine O(n²) 瓶颈下界 Ω(n^1.26)，根因实测 BSP 总数 ~ n^1.26 超线性，称唯一突破口
    是「结构定理证 BSP 总数线性」。#23 §7 从 M26 元素树 T=(E,par) + M05 级别关系做 L0 结构判定：

    **7.1 |E|=O(n)（成立，须不相交规范树前提）**：缠论级别递归——级别 k+1 由 ≥3 个级别 k 次级别走势
    构成（中枢=≥3 连续次级别有重叠）。**若**每个级别-k 元素恰属 ≤1 父（不相交规范树），则 N_{k+1}≤N_k/3，
    级别数 O(log₃ n)，|E|≤n·Σ(1/3^k)=O(n)（几何级数收敛）。**关键前提（codex 异质确认）**：≥3 规则
    **本身不证** |E|=O(n)——只有当「constituted by」意味着**不相交父子消耗**时才成立。这正是 M26 元素树
    T=(E,par) 的 **par 单值（每节点唯一父）蕴含 |E|=O(n)**（树非 DAG）。

    **7.2 量的身份混淆（核心发现）**：BSP⊆E（买卖点是元素子集，每中枢 O(1) 个 type1/2/3）。若 |E|=O(n)
    则**规范 BSP 节点数=O(n)**。这与实测 n^1.26 矛盾。矛盾点（codex 确认）：**两个不同的量**——
    - 规范树节点数：M26 不相交树模型下 = **O(n)**。
    - 实测 n^1.26：是**另一个量**——事件流/候选图/版本化重算计数，不是规范树节点数。
      来自 (a) 中枢重构/延伸 emit 多 BSP（事件 vs 节点）；(b) 候选中枢重叠/共享次级别（破坏不相交划分，
      数重叠候选图非树）；(c) 每 bar 流式重评估 BSP，计 O(中枢数×bar 数) 事件非 O(中枢数) 节点。

    **7.3 判决**：缠论规范 BSP 子集渐近=O(n)（L0 真结构定理，M26 par 单值树蕴含，已成立）。下界主张
    Ω(n^1.26) **须精确化**：只对「引擎枚举/处理超线性 BSP 事件/候选」成立；若主张「缠论规范 BSP 子集超线性」
    则是**伪命题（量混淆）**。acceptance[4] 的 O(n) 目标**结构上未被否证**（规范 BSP=O(n)），被卡的是
    引擎把节点计数做成了事件流计数。O(n) 引擎真突破口=引擎对每规范 BSP 只 emit/process 一次
    （事件流去重/增量维护=**工程问题**），不是缠论结构否证。

    这不是定义冲突——是把「BSP 总数渐近」单一量分离为两个不同范畴的量（L0 规范节点 / L2/L3 事件流），
    分离后各自有效域清晰可分层 ⟹ **不触发中断 #1**。
  layer: 实装   # 引擎事件流计数层 vs M26 规范树节点计数层。非缠论定义冲突（M26 树结构不变），是引擎实装把节点计数做成事件流计数（O(中枢×bar) 重评估）。修复=事件流去重/增量维护（工程），不改缠论结构。
  trigger: "codex 证 engine O(n²) 瓶颈下界 Ω(n^1.26)（acceptance[4] O(n)@16K 实测 exp≈2.0 FAILED，见 650）→ #23 §7 从 M26 par 单值树做 L0 结构判定 |E|=O(n) → codex 异质独立确认量身份分离（规范节点 O(n) ≠ 事件流 n^1.26）+ ≥3 规则须不相交划分前提（真异质增量）。"

# 概念分离
separation:
  before: "「BSP 总数渐近」——把买卖点总数随 bar 数 n 的增长当作单一渐近量。实测 n^1.26 被当作「缠论 BSP 总数超线性」⟹ 误推「O(n) 结构被否证」。"
  after:
    - name: "规范树节点数（structural node count）= O(n)"
      definition: "M26 元素树 T=(E,par) 中 BSP⊆E 的节点计数。par 单值（每节点唯一父，树非 DAG）+ ≥3 规则 + 不相交划分 ⟹ N_{k+1}≤N_k/3，|E|≤n·Σ(1/3^k)=O(n)（几何级数收敛）。"
      source: "缠论知识库 §8（级别递归 ≥3 构成）+ M26 元素树 par 单值。codex 异质确认 ≥3 规则须不相交划分前提。L0 结构定理。"
    - name: "事件流/候选图/版本化重算计数（event/candidate/version count）= 实测 n^1.26"
      definition: "引擎运行时产生的 BSP 事件总数。来自中枢重构/延伸 emit 多 BSP（事件计数）+ 候选中枢重叠共享次级别（重叠候选图非树）+ 每 bar 流式重评估（O(中枢数×bar 数) 事件）。"
      source: "codex engine O(n²) 瓶颈实测 n^1.26（acceptance[4]）。memory project_bsp_event_stream_osb_wall 第八墙（事件流全量 marshal+扫描）。L2/L3 经验量。"
  pending_verification: "(a) 引擎能否对每规范 BSP 只 emit/process 一次（事件流去重/增量维护）使运行时计数 = O(n) 规范节点数 → acceptance[4] O(n)@16K 实测验证（待 task#16 ElementView 大重构，需 fresh session，见 650）。(b) 候选中枢重叠破坏不相交划分时，引擎数的是重叠候选图——去重后是否仍 O(n)。"

# 涉及的定义
definitions_involved:
  - name: "M26 元素树 T=(E,par) par 单值（每节点唯一父）"
    version: ".chanlun/specs/2026-06-28-complete-mutex-classification-pdf-extract.md M26 + formal Origin/MutexRecursive.lean"
    role: "规范节点 O(n) 的结构依据。par 单值蕴含树（非 DAG）⟹ 不相交划分 ⟹ N_{k+1}≤N_k/3 ⟹ |E|=O(n)。codex 异质确认：≥3 规则本身不证 O(n)，须 par 单值的不相交父子消耗。"
  - name: "acceptance[4] O(n)@16K 引擎复杂度目标"
    version: "goal g-sigma-complete-l2-nautilus acceptance[4]（见 650：实测 exp≈2.0 FAILED）"
    role: "被精确化的下界主张。codex Ω(n^1.26) 下界只对事件流/候选计数成立；规范 BSP 子集 O(n) 未被否证。O(n) 目标结构上未被否证，被卡的是引擎节点计数做成事件流计数。"
  - name: "231 形式化有效域规则（L0/L1/L2/L3）"
    version: ".claude/rules/formalization-validity-domain.md（settled，谱系 231）"
    role: "约束来源。规范节点 O(n) 全 L0（从缠论定义+树结构推渐近，不依赖数据）；实测 n^1.26 是 L2/L3 经验量（事件流计数）。两者不可互相否证——正是它们被混淆才产生「L2/L3 数据否证 L0 结构」假象（231 禁止：跨等级互相否证）。"
  - name: "653 gap-B「状态/事件范畴分离」"
    version: ".chanlun/genealogy/pending/653（本轮同源姊妹）"
    role: "同源关联。mutex-prove-result §7.3 明示：本号 BSP「节点/事件范畴分离」与 653 gap-B「状态/事件范畴分离」同源——都是把两个不同范畴的量/对象混为一谈。同轮姊妹发现。"

# 解决方式
resolution:
  type: 概念分离   # BSP 量身份分离（规范节点 O(n) / 事件流 n^1.26）+ acceptance[4] 下界主张精确化。由 #23 §7 L0 结构判定 + codex 异质确认坐实。引擎事件流去重=行动类，最终结算待编排者 /ritual。
  description: |
    概念分离已完成（L0 结构判定）：「BSP 总数渐近」分离为规范树节点数（O(n)，M26 par 单值树蕴含，
    几何级数收敛）与事件流/候选/重评估计数（实测 n^1.26，L2/L3 经验量）。两量不同范畴，不可互相否证。
    codex 异质独立确认量身份分离 + ≥3 规则须不相交划分前提（真异质增量），无新增 gap。
    判决：缠论规范 BSP 子集 O(n) 是真结构定理（已成立，L0）；acceptance[4] 下界 Ω(n^1.26) 只对
    「引擎枚举/处理超线性事件/候选」成立，**没否证** O(n) 规范结构。
    严格下一步（行动类，待 Lead/fresh session）：引擎对每规范 BSP 只 emit/process 一次
    （事件流去重/增量维护 = task#16 ElementView 大重构，见 650 需 fresh session）→ 使运行时计数=O(n)
    规范节点数。这是工程问题（增量维护），不是缠论结构否证。
  decided_by: 蜂群内部   # #23 §7 L0 结构判定（mutex-derive 工位）+ codex 异质独立确认；genealogist 结构记录量身份分离；引擎事件流去重=行动类待 Lead/fresh session；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 实测 BSP 总数 n^1.26 超线性 ⟹ 缠论 BSP 总数超线性 ⟹ acceptance[4] 的 O(n) 引擎目标结构上不可达（被否证）。(2) O(n) 引擎的突破口是「结构定理证 BSP 总数线性」（即证 BSP 节点=O(n)）。(3) 缠论 ≥3 规则（级别 k+1 由 ≥3 个级别 k 构成）本身蕴含 |E|=O(n)。"
  why_negated: "(1) 量身份混淆：实测 n^1.26 是事件流/候选/重评估计数（中枢重构 emit 多 BSP + 候选重叠 + 每 bar 重评估），不是规范树节点数。规范 BSP 节点数 = O(n)（M26 par 单值树蕴含）。L2/L3 事件流计数不否证 L0 结构节点计数（231 禁跨等级否证）。(2) BSP 节点=O(n) **已成立**（无需再证）；真突破口是引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=工程问题），不是结构定理。(3) codex 异质确认：≥3 规则本身**不证** O(n)——只有当「constituted by」意味着不相交父子消耗（一元素只喂一父）时才成立；这是 M26 par 单值（树非 DAG）保证的，非 ≥3 规则单独蕴含。候选中枢重叠（一元素属多候选）破坏不相交划分时数的是重叠候选图非树。"

# 新产出
new_output:
  definitions:
    - "★BSP 量身份分离：规范树节点数（O(n)，L0，M26 par 单值树+不相交划分蕴含）⊥ 事件流/候选/重评估计数（n^1.26，L2/L3 经验）。两量不同范畴，不可互相否证。"
    - "|E|=O(n) 须 par 单值（树非 DAG）前提：N_{k+1}≤N_k/3 ⟹ |E|≤n·Σ(1/3^k)=O(n)（几何级数收敛）。≥3 规则本身不证 O(n)（codex 异质），须不相交父子消耗（par 单值蕴含）。"
    - "acceptance[4] Ω(n^1.26) 下界精确化：只对「引擎枚举/处理超线性 BSP 事件/候选」成立；缠论规范 BSP 子集 O(n) 未被否证。"
    - "O(n) 引擎真突破口=引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=工程问题），非缠论结构否证。"
  code_changes: "无（本号是量身份分离的结构记录，纯谱系产出）。引擎事件流去重/增量维护（task#16 ElementView 大重构，见 650 需 fresh session）= 行动类。本轮已落：.chanlun/diagnostics/mutex-derive-result.md §7（结构判定报告）。"
  orchestration_changes: |
    方法论：①「实测渐近否证结构复杂度目标」类主张前必先核：实测量（事件流/候选/版本计数）是否等于
    结构量（规范树节点数）——否则否证的是错对象（事件流超线性不否证规范结构 O(n)）。这与 645
    （goal 六层穿透否证错对象 π_Θ^cov 非 π_Θ^bsp）同构——**否证有效域 = 实测构造的量，非该量声称代表的结构概念**。
    ②O(n) 复杂度「下界」主张须区分「结构下界」（规范节点数）与「引擎实现下界」（事件流计数）——
    后者可经去重/增量维护降到前者，不是结构不可达。把工程实现下界误判为结构下界 = 过早接受退化（161 务实否定对偶，同 648 裁决 D 方法论）。

# 影响范围
impact:
  affected_modules:
    - "引擎 BSP 事件流计数层（memory project_bsp_event_stream_osb_wall 第八墙）→ 本号是其结构层根因（事件流超线性 = 节点计数做成事件流计数）。修复=事件流去重/增量维护（task#16，见 650）。"
    - "goal g-sigma-complete-l2-nautilus acceptance[4] O(n)@16K → 下界 Ω(n^1.26) 精确化为「引擎事件流下界」，结构 O(n) 未被否证；O(n) 目标待引擎去重后重测（650：task#16 fresh session）。"
    - "formal Origin/MutexRecursive.lean（M26 元素树）→ par 单值蕴含 |E|=O(n) 的结构根据（候选实证待 Lean 立 |E|=O(n) 引理，本号未实装该引理，仅结构判定）。"
  affected_definitions:
    - "M26 元素树（par 单值树）：本号坐实其 par 单值蕴含 |E|=O(n)（规范节点）。不改 M26（结构不变），是给其复杂度后果命名。"
    - "acceptance[4]（650 记 FAILED）：本号精确化——FAILED 的是引擎事件流计数（exp≈2.0/n^1.26），非规范结构 O(n)。650 维持生成态（其 reader/writer 契约矛盾正交）。"
    - "653（本轮姊妹，生成态）：本号 BSP「节点/事件范畴分离」与 653 gap-B「状态/事件范畴分离」同源。同轮印证，不否定，653 维持生成态。"
    - "231（settle）：本号是其活实例——L0 节点 O(n) 与 L2/L3 事件流 n^1.26 不可互相否证（跨等级否证 = 231 禁止模式）。印证，维持 settle。"
    - "缠论 ≥3 规则 / 级别递归定义（一级权威）：本号不改——指出 ≥3 规则本身不证 O(n)（须 par 单值不相交划分），是对其复杂度蕴含的精确化（codex 异质增量）。"
  downstream_implications:
    - "★acceptance[4] O(n) 目标结构上未被否证：实测 n^1.26 否证的是引擎事件流计数（可经去重降到 O(n)），不是缠论规范 BSP 结构。下界主张须标注是「结构下界」还是「引擎实现下界」（后者可优化）。"
    - "引擎事件流去重/增量维护（task#16 ElementView，见 650 fresh session）后，运行时 BSP 计数应降到 O(n) 规范节点数 → acceptance[4] O(n)@16K 待重测（不预设结果，候选中枢重叠破坏不相交划分时仍需核去重后是否 O(n)）。"
    - "memory project_bsp_event_stream_osb_wall（第八墙）：本号给其结构层根因——第八墙「事件流全量 marshal+扫描」正是 7.2(c) 流式重评估（O(中枢×bar) 事件非 O(中枢) 节点）。结构层 BSP=O(n) ⊥ 工程层事件流超线性（节点/事件两范畴）。"

# 谱系关联
related_records:
  parent: "231号（有效域规则）——本号是其在「L0 结构节点计数 vs L2/L3 事件流计数不可互相否证」维度的活实例"
  children: []
  related:
    - "653（本轮姊妹，gap-B 状态/事件范畴分离）：mutex-prove-result §7.3 明示同源（节点/事件 vs 状态/事件范畴分离）"
    - "638（settle，hostOf）：BSP⊆E 元素子集 + source_index=end_index 端点共享，与 638 同涉 BSP 结构"
    - "231（settle，有效域规则）：L0 节点 O(n) ⊥ L2/L3 事件流 n^1.26，跨等级不可互相否证"
    - "222/223/230（settle，有效域≠定义域三例）：本号是「实测量否证结构量=错对象」的同族（与 645 否证错对象同构）"
    - "memory project_bsp_event_stream_osb_wall：第八墙事件流 O(S×B) 墙 = 本号 7.2(c) 流式重评估的工程层表现；本号给其结构层根因（节点/事件范畴分离）"

# 认识论等级标注（231号强制）
epistemological_levels:
  - proposition: "规范树节点数 |E|=O(n)（BSP⊆E ⟹ 规范 BSP=O(n)）"
    level: "L0（从缠论 ≥3 级别递归 + M26 par 单值树 + 不相交划分推渐近，几何级数收敛，不依赖数据）"
    increment: "高（对「规范 BSP=O(n) 已成立，acceptance[4] 结构未被否证」的判定）；codex ≥3 须 par 单值不相交划分是真异质增量"
  - proposition: "实测 BSP 总数 n^1.26 = 事件流/候选/版本化重算计数（非规范树节点数）"
    level: "L2/L3（codex engine O(n²) 瓶颈实测，事件流计数；acceptance[4] exp≈2.0 FAILED）"
    increment: "高：实测量的范畴识别（事件流非节点）——揭示「n^1.26 否证 O(n)」是量混淆假象"
  - proposition: "L0 节点计数 O(n) 与 L2/L3 事件流计数 n^1.26 不可互相否证（不同范畴）"
    level: "L0 结构 vs L2/L3 经验——两个认识论等级（231 禁跨等级互相否证）"
    increment: "高：量身份分离的核心判定（acceptance[4] O(n) 目标结构上未被否证，被卡的是引擎节点计数做成事件流计数）"
  - proposition: "O(n) 引擎真突破口 = 引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护）"
    level: "L0（结构判定：节点 O(n) 已成立，差距在引擎事件多重性）+ 待 L2（task#16 引擎去重后实测，不预设结果）"
    increment: "高：突破口从「证结构 O(n)」（已成立无需证）修正为「引擎事件去重」（工程问题）"
---

# 654 ★BSP 量身份分离：规范树节点数 O(n) ≠ 事件流计数 n^1.26

## 一句话结论

codex 证 engine O(n²) 瓶颈下界 Ω(n^1.26)（acceptance[4] O(n)@16K 实测 exp≈2.0 FAILED），#23 §7 从
M26 元素树 par 单值结构判定：**「BSP 总数」是两个不同范畴的量被混为一谈**——规范树节点数 = **O(n)**
（L0 结构定理，M26 par 单值树+不相交划分蕴含，几何级数收敛）≠ 实测 n^1.26（事件流/候选/重评估计数，
L2/L3 经验量）。两量不可互相否证。**acceptance[4] 的 O(n) 目标结构上未被否证**——被卡的是引擎把节点
计数做成了事件流计数。真突破口 = 引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=**工程问题**），
不是缠论结构否证。codex 异质独立确认量身份分离 + ≥3 规则须不相交划分前提（真异质增量）。

## 量身份分离表

| 量 | 渐近 | 范畴 | 来源 |
|----|------|------|------|
| 规范树节点数（BSP⊆E） | **O(n)** | L0 结构 | M26 par 单值树（非 DAG）+ ≥3 规则 + 不相交划分 ⟹ N_{k+1}≤N_k/3 ⟹ \|E\|≤n·Σ(1/3^k)=O(n) |
| 事件流/候选/重评估计数 | **n^1.26** | L2/L3 经验 | (a) 中枢重构 emit 多 BSP；(b) 候选重叠（一元素属多候选，破坏不相交划分，数重叠候选图非树）；(c) 每 bar 流式重评估 O(中枢×bar) |

混淆两量 ⟹「实测 n^1.26 否证 O(n) 结构」假象（231 禁跨等级互相否证）。

## codex 真异质增量

≥3 规则（级别 k+1 由 ≥3 个级别 k 构成）**本身不证** |E|=O(n)——只有当「constituted by」意味着
**不相交父子消耗**（一元素只喂一父）时才成立。这是 M26 元素树 **par 单值（每节点唯一父，树非 DAG）**
保证的，非 ≥3 规则单独蕴含。候选中枢重叠（一元素属多候选）破坏不相交划分时，引擎数的是重叠候选图非树。

## 为何首次记录

grep `.chanlun/genealogy/`「n^1.26 / 1.26 / BSP 量身份 / 节点 vs 事件流」仅命中 dag.yaml（dag 拓扑映射），
pending/settled 无 BSP 量身份分离条目 ⟹ **首次显式化**。memory project_bsp_event_stream_osb_wall（第八墙）
是其**工程层表现**（事件流 O(S×B) 墙），本号给其**结构层根因**（节点/事件范畴分离）——两者一致不矛盾。

## 与 645 / 648 方法论同构

- 645：goal 六层穿透否证错对象（π_Θ^cov 覆盖投影 ≠ π_Θ^bsp 买卖点择时）——否证有效域=实测构造的量，非该量声称代表的概念。
- 648 裁决 D：把可修的工程错配误判为结构定理 = 过早接受退化（161 务实否定对偶）。
- 本号：实测 n^1.26（事件流）否证错对象（规范结构 O(n)）；把引擎实现下界误判为结构下界 = 过早接受退化。

**三号同根**：否证/下界主张的有效域必须精确到实测构造的量/对象，不外推到它声称代表的结构概念。

## 为何 source-tracing 而非矛盾发现 / 不触发中断 #1

无两条互斥定义——是把「BSP 总数渐近」单一量分离为两个不同范畴的量（L0 规范节点 / L2/L3 事件流），
分离后各自有效域清晰可分层 ⟹ **不触发中断 #1**。

## 张力检查（019d/020）

### 检查范围
同轮（mutex-derive #23 §7 / 653 gap-B）∪ 1-hop（231/638/653）∪ Hub（231 有效域、645 否证错对象同构）。

### 张力1：vs 653（同轮姊妹）——同源印证
mutex-prove-result §7.3 明示本号「节点/事件范畴分离」与 653 gap-B「状态/事件范畴分离」同源。不同对象（BSP 计数 vs 生命期包含），同源结构（两范畴量/对象混淆）。无矛盾。

### 张力2：vs 231（settle）——活实例
L0 节点 O(n) 与 L2/L3 事件流 n^1.26 不可互相否证（跨等级否证=231 禁止）。本号是 231 在「计数量范畴分离」维度的活实例。印证，231 维持 settle。

### 张力3：vs 645（否证错对象）——方法论同构
645 否证 π_Θ^cov 非 π_Θ^bsp；本号实测 n^1.26（事件流）非规范结构 O(n)。同构（否证有效域=实测量非声称概念）。无矛盾，互相印证。

### 张力4：vs memory project_bsp_event_stream_osb_wall——结构层 vs 工程层一致
第八墙（事件流 O(S×B)）= 本号 7.2(c) 流式重评估工程表现；本号给结构层根因。结构 BSP=O(n) ⊥ 工程事件流超线性（节点/事件两范畴），一致不矛盾。

### 概念分离信号检测（中断 #1）
量身份分离后各自范畴清晰可分层 ⟹ **不触发中断 #1**。genealogist 记录分离，不发 SendMessage（已由 Lead 消息触发）。

### 递归运动结构完成检测（020）
- 第0层：本号写入（BSP 量身份分离 + acceptance[4] 下界精确化 + 真突破口）。
- 第1层：本号 × codex Ω(n^1.26) → 量身份分离（净新发现高：实测 n^1.26=事件流非节点，O(n) 结构未被否证）。
- 第2层：本号 × 231/645 → 跨等级不可否证 + 否证错对象同构（净新发现中：231 已知，BSP 维度实例是新）。
- 第3层：本号 × project_bsp_event_stream_osb_wall/653 → 结构层根因 + 同源姊妹（净新发现降：第八墙已知，收尾）。
- scope₁(量身份分离+O(n)未被否证) > scope₂(231/645同构) > scope₃(关联)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 引擎事件流去重=行动类（task#16，见 650 fresh session）；最终结算待编排者 /ritual。

## 回溯扫描（职责3）
- **231/638/222/223/230（settle）**：本号印证不破坏，维持 settle。
- **653（本轮姊妹，生成态）/650（生成态，acceptance[4] FAILED 记录）**：本号印证/精确化不否定，维持生成态。
- **memory project_bsp_event_stream_osb_wall（Lead 维护）**：本号给结构层根因，建议 Lead 回填「第八墙事件流超线性是工程层；结构层规范 BSP=O(n)；O(n) 目标结构未被否证，真突破口=引擎事件去重」。genealogist 不直接改 memory。
- **无 settled 被本号回溯破坏。** 本号是量身份分离（source-tracing）+ L0 结构判定的结构记录，引擎去重=行动类，最终结算待编排者 /ritual。
