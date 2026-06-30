---
id: 654
number: 654
status: 已结算   # /ritual 结算（2026-06-30，编排者授权）。BSP 量身份分离（规范树节点数 O(n) ≠ 事件流 n^1.26）三方交汇（#23 §7 L0 结构判定 + codex 异质独立确认量身份分离 + 654 已调和 codex Ω(n^1.26) vs §7 O(n) 为不同对象）→ 结算依据充分。引擎事件流去重=行动类待 task#16 fresh session。
date: "2026-06-30"
settlement_date: "2026-06-30"
type: source-tracing
depends_on: ["231", "653"]
related: ["638", "653", "222", "223", "230", "645", "mutex-derive-result", "mutex-prove-result"]
negation_source: "homogeneous（蜂群 mutex-derive #23 §7 从 M26 元素树结构判定）+ heterogeneous（codex 异质独立确认量身份分离 + ≥3 规则本身不证 O(n) 须不相交规范树前提，真异质增量）"
negation_model: "codex（OpenAI，独立证 engine O(n²) 瓶颈下界 Ω(n^1.26) + 量身份分离确认）"
negation_form: "separation"
title: "★BSP 量身份分离（首次显式化，codex 异质确认）：规范树节点数 = O(n)（L0 结构定理，M26 par 单值树+≥3 规则+不相交划分蕴含，几何级数收敛）≠ 实测 n^1.26（事件流/候选图/版本化重算计数，L2/L3 经验量）。两量混淆产生「结构否证 O(n)」假象。acceptance[4] O(n) 下界主张 Ω(n^1.26) 须精确化——只对引擎枚举/处理超线性 BSP 事件/候选成立；缠论规范 BSP 子集 O(n) 未被否证。O(n) 引擎真突破口=引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=工程问题），非缠论结构否证"

topo_effect: "separate:BSP-total-asymptotic:{canonical-tree-node-count=O(n)[L0,M26-par-single-valued-tree+disjoint-partition,geometric-convergence]|event-stream-candidate-version-count=n^1.26[L2/L3,empirical]} | precise:acceptance4-Ω(n^1.26)-lower-bound=only-for-engine-event/candidate-enumeration-NOT-canonical-BSP-subset | record:O(n)-engine-breakthrough=emit/process-each-canonical-BSP-once(event-dedup/incremental=engineering)-NOT-chanlun-structural-falsification | bound:O(n)-structural-NOT-falsified-by-n^1.26-empirical(category-difference)"

# ============================================================
# /ritual 结算区块（2026-06-30，genealogist 落实，编排者授权 /ritual）
# ============================================================
settlement:
  from_contradiction: |
    另一线 codex 证 engine O(n²) 瓶颈下界 Ω(n^1.26)，根因实测 BSP 总数 ~ n^1.26 超线性，称唯一突破口
    是「结构定理证 BSP 总数线性」。这与 #23 §7 从 M26 par 单值树推出的规范 BSP=O(n) 矛盾。
  negated_definition: |
    被否定：(1)「实测 BSP 总数 n^1.26 超线性 ⟹ 缠论 BSP 总数超线性 ⟹ acceptance[4] O(n) 结构上不可达」。
    (2)「O(n) 引擎突破口是证 BSP 节点=O(n)」（已成立无需证）。(3)「缠论 ≥3 规则本身蕴含 |E|=O(n)」。
  new_definition: |
    BSP 量身份分离：规范树节点数（O(n)，L0，M26 par 单值树+不相交划分蕴含，N_{k+1}≤N_k/3，
    |E|≤n·Σ(1/3^k)=O(n) 几何级数收敛）⊥ 事件流/候选/重评估计数（n^1.26，L2/L3 经验：中枢重构 emit 多
    BSP + 候选重叠 + 每 bar 流式重评估 O(中枢×bar)）。两量不同范畴，不可互相否证。
    |E|=O(n) 须 par 单值（树非 DAG）前提——≥3 规则本身不证 O(n)（codex 异质），须不相交父子消耗。
    acceptance[4] Ω(n^1.26) 下界只对引擎事件流计数成立；规范 BSP 子集 O(n) 未被否证。
  logical_necessity: |
    M26 元素树 T=(E,par) par 单值（每节点唯一父）⟹ 树非 DAG ⟹ 不相交划分 ⟹ N_{k+1}≤N_k/3 ⟹
    |E|≤n·Σ(1/3^k)=O(n)（几何级数收敛，L0 从定义推渐近）。BSP⊆E ⟹ 规范 BSP=O(n)。这是 L0 结构必然。
    实测 n^1.26 是引擎运行时事件流计数（不同范畴的量），L2/L3 经验量否证 L0 结构节点计数 = 231 禁止的
    跨等级互相否证。混淆两量产生「n^1.26 否证 O(n)」假象——分离后各自有效域清晰，假象消解。逻辑必然
    （par 单值蕴含树是 M26 定义的代数后果，几何级数收敛是其复杂度必然），非经验总结。
  sufficiency: |
    (1) #23 §7 L0 结构判定：从 M26 par 单值树 + ≥3 级别递归推 |E|=O(n)（几何级数收敛）。
    (2) codex 异质独立确认量身份分离 + 真异质增量（≥3 规则本身不证 O(n)，须 par 单值不相交划分；
        候选重叠破坏不相交划分时数的是重叠候选图非树）。codex 独立证 engine O(n²) 瓶颈下界 Ω(n^1.26)
        是事件流计数，与 §7 O(n) 是不同对象——654 已调和（Lead 消息确认三方交汇）。
    (3) 首次显式化（grep .chanlun/genealogy/ 确认 pending/settled 无 BSP 量身份分离条目，仅 dag.yaml）。
    (4) 231 跨等级不可否证规则的活实例 ⟹ 结算依据充分。
  tension_check: |
    vs 231（settle）：本号是其活实例（L0 节点 O(n) ⊥ L2/L3 事件流 n^1.26 不可互相否证）。印证，不破坏。
    vs 653（同轮姊妹）：同源（节点/事件 vs 状态/事件范畴分离，mutex-prove-result §7.3）。不破坏。
    vs 645（否证错对象）：方法论同构（实测量否证错对象——n^1.26 事件流非规范结构 O(n)，同 π_Θ^cov
      非 π_Θ^bsp）。互相印证，不破坏。
    vs 638（settle）/222/223/230（settle）：印证不破坏。
    vs memory project_bsp_event_stream_osb_wall：结构层根因 vs 工程层表现一致（第八墙事件流 O(S×B) =
      7.2(c) 流式重评估），不矛盾。
    无 settled 被本号结算破坏。
  downstream_action: |
    引擎事件流去重/增量维护（task#16 ElementView 大重构，见 650 需 fresh session）= 行动类，使运行时
    BSP 计数降到 O(n) 规范节点数 → acceptance[4] O(n)@16K 待重测（不预设结果，候选重叠破坏不相交划分时
    仍需核去重后是否 O(n)）。memory project_bsp_event_stream_osb_wall 建议 Lead 回填结构层根因
    （genealogist 不直接改 memory）。
  decided_by: 编排者 /ritual（授权 genealogist 落实）
---

# 654 ★BSP 量身份分离：规范树节点数 O(n) ≠ 事件流计数 n^1.26【已结算 2026-06-30】

## 结算一句话

codex 证 engine O(n²) 瓶颈下界 Ω(n^1.26)（acceptance[4] O(n)@16K 实测 exp≈2.0 FAILED），#23 §7 从 M26
par 单值结构判定：**「BSP 总数」是两个不同范畴的量被混为一谈**——规范树节点数 = **O(n)**（L0 结构
定理，M26 par 单值树+不相交划分，几何级数收敛）≠ 实测 n^1.26（事件流/候选/重评估计数，L2/L3 经验）。
两量不可互相否证。**acceptance[4] O(n) 目标结构上未被否证**——被卡的是引擎把节点计数做成了事件流计数。
真突破口 = 引擎对每规范 BSP 只 emit/process 一次（事件流去重/增量维护=**工程问题**）。

## 量身份分离表

| 量 | 渐近 | 范畴 | 来源 |
|----|------|------|------|
| 规范树节点数（BSP⊆E） | **O(n)** | L0 结构 | M26 par 单值树（非 DAG）+ ≥3 规则 + 不相交划分 ⟹ N_{k+1}≤N_k/3 ⟹ \|E\|≤n·Σ(1/3^k)=O(n) |
| 事件流/候选/重评估计数 | **n^1.26** | L2/L3 经验 | (a) 中枢重构 emit 多 BSP；(b) 候选重叠（破坏不相交划分，数重叠候选图非树）；(c) 每 bar 流式重评估 O(中枢×bar) |

混淆两量 ⟹「实测 n^1.26 否证 O(n) 结构」假象（231 禁跨等级互相否证）。

## codex 真异质增量

≥3 规则**本身不证** |E|=O(n)——只有当「constituted by」意味着**不相交父子消耗**（一元素只喂一父）时
才成立。这是 M26 元素树 **par 单值（树非 DAG）**保证的，非 ≥3 规则单独蕴含。候选重叠（一元素属多
候选）破坏不相交划分时引擎数的是重叠候选图非树。

## 与 645 / 648 方法论同构（三号同根）

否证/下界主张的有效域必须精确到实测构造的量/对象，不外推到它声称代表的结构概念：645 否证 π_Θ^cov
非 π_Θ^bsp；648 裁决 D 把可修工程错配误判为结构定理=过早接受退化；本号实测 n^1.26（事件流）否证错
对象（规范结构 O(n)），把引擎实现下界误判为结构下界=过早接受退化（161 务实否定对偶）。

---

# （以下为结算前生成态原文，谱系012：发现过程不可压扁，保留作发生史）

## 认识论等级（231号）

- 规范树节点数 |E|=O(n)（BSP⊆E ⟹ 规范 BSP=O(n)）：L0（缠论 ≥3 级别递归 + M26 par 单值树推渐近）。
- 实测 BSP 总数 n^1.26 = 事件流/候选/版本化重算计数（非规范树节点数）：L2/L3（codex engine O(n²) 瓶颈实测）。
- L0 节点 O(n) ⊥ L2/L3 事件流 n^1.26 不可互相否证：231 禁跨等级互相否证。
- O(n) 引擎真突破口 = 引擎对每规范 BSP 只 emit/process 一次（工程问题）+ 待 L2（task#16 去重后实测，不预设结果）。

## 谱系关联

- parent: 231号（有效域规则）——本号是「L0 结构节点计数 vs L2/L3 事件流计数不可互相否证」维度活实例
- related: 653（同轮姊妹，状态/事件范畴分离）、638（BSP⊆E）、645（否证错对象同构）、222/223/230、
  memory project_bsp_event_stream_osb_wall（第八墙=本号 7.2(c) 工程层表现，本号给结构层根因）
