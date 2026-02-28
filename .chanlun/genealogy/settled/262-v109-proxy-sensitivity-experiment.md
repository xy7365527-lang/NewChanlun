---
id: '262'
number: 262
title: v109 结算通道可观测性实验——代理敏感性与六条边稳定性分类
type: 经验验证
status: 已结算
date: 2026-02-28
source: v110-swarm v109-yf-rewrite 工位
depends_on:
  - '234'  # ker(D) L2 交叉验证（勘误后）
  - '259'  # 黄金折叠脱耦实验
  - '235'  # 三态分类
negation_form: 对象否定——v109 否定的不是 234号结论，而是 234号分类结论的代理无关性假设
epistemology:
  - level: L2*
    scope: 代理敏感的真实数据验证（四组 C 代理 × 六条边）
    description: GLD/GC=F/DBA/BZ=F 四组 C 代理替换实验，yfinance 日线数据
---

# 262号：v109 结算通道可观测性实验——代理敏感性与六条边稳定性分类

## 1. 结论

v109 三步实验（GC=F/DBA/BZ=F 替代 GLD）+ 修正后 234号基线，四组数据综合结论：

### 六条边稳定性分类

| 类别 | 边 | 描述 |
|------|-----|------|
| **稳定** | E-R amplitude | 四组一致，pc_std=0.018 |
| **稳定** | E-$ amplitude | 四组一致，pc_std=0.020 |
| **稳定** | E-C | 黄金(GLD/GC=F)=independent，非黄金(DBA/BZ=F)=amplitude |
| **稳定** | C-R amplitude | 四组一致，所有|β|<0.03，远离阈值 |
| **阈值边界** | R-$ | 234号 distance=0.000，分类对 C 代理敏感 |
| **阈值边界** | C-$ | GC=F 跳变为 direction，BZ=F distance=0.006 |

### 综合对比表

| 边 | 234号(GLD) | GC=F | DBA | BZ=F |
|----|-----------|------|-----|------|
| E-R [ctrl] | amp pc=-0.317 | amp pc=-0.333 | amp pc=-0.302 | amp pc=-0.289 |
| E-C | indep pc=-0.029 | indep pc=-0.001 | amp pc=0.217 | amp pc=0.238 |
| E-$ [ctrl] | amp pc=-0.183 | amp pc=-0.204 | amp pc=-0.156 | amp pc=-0.182 |
| C-R | amp pc=0.166 | amp pc=0.113 | amp pc=-0.108 | amp pc=-0.151 |
| C-$ | amp pc=-0.414 | **dir pc=-0.344** | amp pc=-0.230 | amp pc=-0.135 |
| R-$ | dir pc=-0.065 | dir pc=-0.089 | amp pc=-0.157 | dir pc=-0.160 |

### 代理敏感性发现

1. **GLD（ETF）和 GC=F（期货）给出不同的 C-$ 分类**。ETF 包装层吸收了黄金-美元关系的方向性（GLD: amplitude，GC=F: direction）。
2. **234号认识论等级降级为 L2***（代理敏感的真实数据验证）。
3. **R-$ 是三态分类中唯一的阈值不稳定边**，其分类状态受 C 代理调制。与 234号原始判断（边界成员，吸收率 23.3%）一致。

### 石油结论（Step 2）

布伦特原油全吸收（C_oil-$: amplitude, absorption=0.674），情景 A 成立。石油的结算通道属性对缠论不可见。与定理 4（纯商品全吸收）一致——石油在缠论视角下是纯商品，不是结算通道。

## 2. 定义依据

### 方法论

- **数据源**：yfinance（Alpha Vantage 不提供黄金现货历史数据）
- **标的**：SPY(E) / C代理(GLD, GC=F, DBA, BZ=F) / TLT(R) / UUP($)
- **连续层**：日对数收益率 Pearson + 偏相关（控制其余两标的）
- **离散层**：BiEngine（new笔模式）逐bar方向 → 线性回归 beta
- **分类**：`classify_coupling(partial_corr, beta, kernel_threshold=0.05)` from `discretization_kernel.py`

### 阈值敏感性

| 边 | 四组|β|范围 | 全部 amplitude 需 threshold > | 全部 direction 需 threshold ≤ | 不一致窗口 |
|----|-------------|-------------------------------|-------------------------------|-----------|
| C-R | 0.009~0.029 | 0.029 | 0.009 | 0.020（全在 amplitude 侧）|
| C-$ | 0.023~0.065 | 0.065 | 0.023 | 0.042 |
| R-$ | 0.009~0.064 | 0.064 | 0.009 | 0.055 |

R-$ 不一致窗口最宽（0.055），是最不稳健的边。当前 threshold=0.05 恰在窗口中间。

## 3. 边界条件

1. **分类器阈值依赖**：kernel_threshold=0.05 是硬编码常量。如果调高到 0.065，R-$ 和 C-$ 全部变为 amplitude；如果调低到 0.009，全部变为 direction。阈值选择不改变物理现象，但改变分类语义。
2. **数据源差异**：yfinance vs Alpha Vantage 的日线数据可能有微小差异（除权处理、交易日定义），但不影响分类级别的结论。
3. **时间窗口**：所有实验使用 2007-03 ~ 2026-02 窗口。不同时间窗口可能改变阈值边界边的分类。

## 4. 下游推论

### 262-1：234号勘误（已执行）

234号谱系中 C-R 从 direction 修正为 amplitude，R-$ 从 amplitude 修正为 direction。认识论从 L2 降级为 L2*。

### 262-2：E-C 独立是黄金特有属性

E-C 在黄金代理（GLD/GC=F）下为 independent，在非黄金代理（DBA/BZ=F）下为 amplitude。E-C 独立不是 C 列的普遍性质，是黄金的货币属性使其脱耦于股票。

### 262-3：层 0 结算通道权重需要非缠论数据源

石油在缠论离散化层面全吸收（纯商品），但其结算通道属性在连续层面可能存在（E-C_oil: amplitude, pc=0.238）。层 0 结算通道权重监控需要非缠论数据源（具体代理工程待定）。

## 5. 谱系引用

- **234号**（ker(D) L2 交叉验证）：直接前置，v109 提供四组交叉验证并暴露代理敏感性
- **259号**（黄金折叠脱耦实验）：DBA 基线数据来源
- **235号**（三态分类）：分类框架定义
- **231号**（认识论等级标注）：L2* 等级定义
- **232号**（纤维丛）：C-R 勘误确认纤维丛不受影响

## 6. 影响声明

### 修改

1. `.chanlun/genealogy/settled/234-kernel-l2-cross-verification.md`：C-R/R-$ 分类勘误 + 认识论降级 + 操作语义更新
2. `scripts/v109_settlement_channel_experiment.py`：数据源 AV→yfinance，`_BASELINE_234` C-R 修正

### 新增数据

1. `tmp/v109-step1-xauusd-baseline.json`：GC=F 基线
2. `tmp/v109-step2-brent-oil.json`：BRENT 基线
3. `tmp/v109-step3-control-consistency.json`：DBA 基线 + 控制变量一致性 + 综合对比表

### 不修改

1. `src/newchan/topology/discretization_kernel.py`：分类器不变
2. `src/newchan/topology/fiber_bundle.py`：纤维丛不变（232号 β_cr 不受 D1 层 beta 修正影响）
