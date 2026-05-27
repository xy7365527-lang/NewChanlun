---
id: '520'
number: 520
title: "路径空间(t,p)Rips PH 被否决——σ是伪装的自由参数, H0退化为最大单根速度, 时间盲解在因果在线merge tree而非Rips"
type: 概念发现
status: 已结算
date: 2026-05-27
source: "编排者提出路径空间方案 → Gemini decide(选项C拒绝) → 腾讯700 L2三项证实 + 用户全局σ扩展厘清"
depends_on:
  - '378'   # 持续同调研究线开放
  - '239'   # 振幅力度∈ker(D)，H0时间盲
related:
  - '231'   # 形式化有效域规则（L0-L3，否定性结果的价值）
  - '482'   # 持续同调共享数学基础设施
epistemological_level: "L0(Gemini信息论+自由参数+退化三论证) + L2(腾讯700 P1/P2/P3逐项证实，否定性结果)"
negation_source: heterogeneous
negation_form: negation
negates: null
topo_effect: null
tensions_with: []
downstream_implications_status: "强化 archive/pending-013(PH不能扬弃MACD)；级别判断转向在线merge tree(已L2兑现)；详见 docs/persistence_theory.md §8"
---

# 520号：路径空间 (t,p) Rips PH 被双重否决

**认识论等级**: L0（Gemini decide 信息论/代数论证）+ L2（腾讯 700 单标的 P1/P2/P3 逐项证实，已产生否定性结果——比确认更有价值，231号）

## 发现过程

编排者（2026-05-27）提出"路径空间 PH"解决 PH 时间盲（239号）：每根 K 线视为 (t,p) 平面点，距离 `d=√((Δt/σ_t)²+(Δp/σ_p)²)`，对点云做 Vietoris-Rips，使 persistence 天然含时。经 orchestrator-proxy/decide 路由 Gemini（gemini-3.1-pro-preview）评审 → **选项 C 拒绝**，腾讯 700 L2 实测逐项证实。

## 结论（六要素）

1. **结论**：路径空间 (t,p) Rips PH **不能**解决时间盲，被 L0+L2 双重否决。维持 §5/§6 架构（sublevel H0 + MACD，PH 作监视层）。时间盲的"级别判断"下游问题转由**在线因果 merge tree**（a_online_persistence + a_level_detection）解决，已 L2 兑现。

2. **定义依据**：背驰（beichi.md 第24课）本质依赖全局连续 EMA 的 0 轴跨段基准；买卖点（maimai.md）= 因果离散触发。239号：H0≈振幅∈ker(D)。

3. **三个否决论证**（Gemini L0，腾讯 700 L2 证实）：
   - **P1 信息论否决**：§5 的 MACD 20× 差异来自跨笔 EMA 记忆（笔外全局状态），非笔内几何。反直觉证据：高 MACD 的"急跌"#37 反而 span 更大（13>9）。路径空间只看单笔内部点云，信息论上访问不到 → 不能复现。L2：全局 σ 下 #33=1.03 vs #37=1.04（不可分），两笔幅度近等（55.1≈54.0），差异在跨段。
   - **P2 自由参数陷阱**：σ_p/σ_t 比值控制 (t,p) 纵横比，是伪装成数据内生的自由参数。L2：换 std/mad/range，#33/#37 谁更强的定性结论翻转（top3 笔交集 0/3）。**更深形式**（用户全局 σ 扩展）：σ_p/σ_t 是「幅度↔时间」旋钮，σ_t→∞ 端=纯幅度（时间盲，与振幅 corr +0.99）、σ_t≈Δt 端=幅度被抹除——**不存在同时保幅度+保时间的 σ_t**。
   - **P3 数学退化**：单调笔路径空间 H1 恒空；H0 Rips 退化为相邻点最大距离=最大单根速度。L2：path-H0max 与 max_step_distance 中位相对误差=0.0000。一行差分即得，Rips 杀鸡用牛刀（违反 231号）。

4. **边界条件**（推翻需同时满足）：① L2/L3 证明"笔内最大瞬时速度/跳空拓扑"对真假买点有决定性贡献且与 MACD 跨笔动量统计正交；② 找到数学严格且绝对无参数（无需任何 σ 归一化）的 (t,p) 联合度量构造。

5. **下游推论**：强化 archive/pending-013（PH 不能扬弃 MACD——又一不可约维度的确认）；力度/背驰维度仍用 MACD（§6 不变）；级别判断走在线 merge tree 的因果 log-gap 横切（腾讯 700 L2：自动涌现 3 级，30min 全局 span≈300=日线级与 §4 一致）。

6. **影响声明**：新增 `src/newchan/a_path_persistence.py`（实验性证伪通道）、`scripts/tencent_path_persistence.py` + `tencent_online_levels.py`、`tests/test_path_persistence.py`；§8 记录全部。同时坐实 a_online_persistence 的批量等价性（消除其声明膨胀，`tests/test_online_persistence.py`）。未改既有 src/ 模块。

## 谱系引用

- 378号（PH 研究线）、239号（ker(D) 时间盲）、231号（有效域 L0-L3 + 否定性结果价值）。
- archive/pending-012（PH=结构监视层，强化）、archive/pending-013（PH 不能扬弃 MACD，强化）。
- 关联未结算缺口：a_level_detection.py 的 `max_levels` 软闸 spec-execution-gap（语义：总数 vs 深度，编排者 2026-05-27 答"我不知道"未裁决，软闸+xfail 诚实记录，待裁决）。
