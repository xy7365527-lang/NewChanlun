---
id: '522'
number: 522
title: "persistence泛函比积分泛函(MACD面积)更抗参数变化——521号架构内MACD动量的over-hist泛函选择(L2正向, 翻转率0%vs60%)"
type: 概念发现
status: 已结算
date: 2026-05-27
source: "Stop-Guard 模块3任务(参数鲁棒性) → 腾讯700日线5对+30min3对 L2实测 → 521号架构内正向增量"
depends_on:
  - '521'   # 纯拓扑动量不存在——动量必须MACD(本发现在此架构内)
  - '378'   # 持续同调研究线
related:
  - '231'   # 形式化有效域规则(L0-L3, 归一化自由度=520σ教训)
  - '520'   # σ自由参数陷阱(bottleneck归一化的caveat来源)
  - '239'   # H0≈振幅∈ker(D)
epistemological_level: "L2(腾讯700日线5背驰对 + 30min 3对=弱L3跨周期, 可否证, 正向结果)"
negation_source: empirical
negation_form: affirmation
negates: null
topo_effect: null
tensions_with: []
downstream_implications_status: "建议: MACD背驰判据可用persistence泛函(over hist)替代积分泛函(面积)以提升参数鲁棒性; 不改521号(动量仍是MACD几何); 详见 docs/persistence_theory.md §14.7"
---

# 522号：persistence 泛函比积分泛函（MACD 面积）更抗参数变化

**认识论等级**: L2（腾讯 700 日线 5 个背驰对 + 30min 3 个对 = 弱 L3 跨周期，可否证，
正向结果——但单标的，L3 多标的裁决仍缺）。

## 发现过程

Stop-Guard 注入模块3（参数鲁棒性）任务：不同 MACD 参数（8-21-5 / 12-26-9 / 20-40-9）
分别算 MACD → 分别做 PH，看 persistence diagram 的 bottleneck 距离 vs 原始 MACD 面积
的参数敏感性。腾讯 700 日线 + 30min L2 实测。

**方法论自检（520号 σ 教训）**：bottleneck（hist 量纲）与面积（积分量纲）不同量纲，
归一化方式本身是伪自由参数。故主判据取**无量纲**的背驰判定量——背驰**比值**
（force_c/force_a 与 area_c/area_a，均无量纲）的变异系数 CV + 背驰 bool 判定翻转率。

## 结论（六要素）

1. **结论**：在 521号架构内（动量 = MACD 几何，PH 仅提取其 hist 的拓扑结构），
   **persistence 泛函（over MACD hist）的背驰判定比积分泛函（面积）更抗 MACD 参数变化**。
   腾讯 700 日线（5 个相邻下跌背驰对，参数集 3 组）：
   - **背驰 bool 判定翻转率：拓扑泛函 0% vs 面积泛函 60%**（面积判据在 5 对中 3 对
     随参数翻转"是否背驰"的结论；persistence 判据 0 翻转）。
   - 背驰比值 CV 中位：拓扑 0.123 vs 面积 0.535（30min 同向：拓扑 0.016~0.034 << 面积）。

2. **定义依据**：背驰（第24课）= C 段力度 < A 段。"力度"可用任一 over-hist 泛函度量：
   积分泛函 = Σ|hist|（缠论传统面积）；拓扑泛函 = sublevel-H0 total_persistence
   （hist 摆动 prominence 累加）。两者都 over **同一** MACD hist（几何动量）。
   persistence 捕捉 hist 极值结构（prominence），对参数引起的 hist 整体缩放/平移
   不敏感；积分对全局缩放敏感 → 拓扑泛函参数更稳。

3. **边界条件（推翻条件）**：① L3 多标的大样本上 persistence 泛函翻转率不再显著低于
   面积；② 找到一组参数使 persistence 判定也频繁翻转而面积稳定。当前单标的 L2 未见。
   caveat：bottleneck 补充判据因归一化自由度（按 total_persistence 归一）仅作参照，
   不作承重结论（520号 σ 教训）。

4. **下游推论**：**MACD 背驰判据可用 persistence 泛函（over hist）替代积分泛函（面积）
   以提升参数鲁棒性**——这是 521号架构内（动量仍是 MACD 几何）的工程改进建议，
   **不**升级 PH 到动量层（521号定理不变）。a_geometric_momentum.momentum_divergence
   提供此泛函（metric="total_persistence"）。

5. **谱系引用**：521号（纯拓扑动量不存在——本发现严格在此架构内，两泛函都 over MACD，
   不冲突）、520号（σ 自由参数陷阱 → bottleneck 归一化 caveat）、231号（有效域 L0-L3 +
   否定性结果价值）、239号（ker(D)）、378号（PH 研究线）。

6. **影响声明**：新增 `src/newchan/a_geometric_momentum.py`（原"a_momentum_topology"按
   521号重命名为"几何动量 over 拓扑特征"，10 测试）、`scripts/tencent_param_robustness.py`
   （L2 实测，7 测试）。docs/persistence_theory.md §14.7 记录。**不改** 521号定理
   （动量必须 MACD）；本发现是 521号架构内 MACD 动量的 over-hist 泛函选择的正向增量。

## 与 521号的关系（关键澄清，避免误读）

521号：纯拓扑因果动量不变量**不存在**（动量需时间度量，拓扑丢弃时间）。
522号：**不**声称 PH 产生动量。522 比较的是"同一 MACD 几何动量的两个 over-hist 泛函
（拓扑 persistence vs 积分面积）的参数稳定性"——两者都依赖 MACD 提供的动量，差异仅在
读取 MACD hist 的方式（极值结构 vs 积分）。故 522 完全在 521 架构内，是正向增量而非
对 521 的削弱。

## 谱系引用

- 521号（纯拓扑动量不存在，架构约束）、520号（σ 陷阱）、231号（有效域 + 否定性结果）、
  239号（ker(D)）、378号（PH 研究线）。
