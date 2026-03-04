# 统一递归拓扑方案——Gemini × Codex 共识报告

**时间**: 2026-02-25
**轮次**: 2 轮（Round 1: 独立评审编排者决断 → Round 2: 交叉回应达成共识）
**结果**: **双方显式声明共识达成**

---

## 一、编排者分层采纳决断——双方评审共识

### 总体判断
**分层决断 Layer 1/2/3 在代数结构搭建的先后次序上无懈可击。**（Gemini 原话，Codex 确认）

### Layer 1（现在做）: T3 + T4 + T5 + T7 + T1(底层)

| 定理 | 内容 | 共识来源 |
|------|------|---------|
| T1 | Extended PH ≡ Zigzag（模块同构） | 暗含在底层——Gemini 提议，Codex 同意 |
| T3 | 中枢 ↔ zigzag 3-支撑条带一一对应 | 编排者原 Layer 1 |
| T4 | δ-Morse = 严格笔模式（阈值分离） | 编排者原 Layer 1 |
| T5 | Trend_k ≅ E(X_{k+1})（走势类型=上级笔） | 编排者原 Layer 1 |
| T7 | B_{k+1} ⊆ Trim(B_k)（递归条形码偏序） | **Codex 提议加入 Layer 1，Gemini Round 2 同意** |

**Layer 1 代码实现**：
- DecompositionFingerprint 加 `barcode` 字段
- gauge_equivalence_report 用 bottleneck 距离
- T7 作为递归一致性单元测试

### Layer 2（验证后决定）: T8
- 背驰 = Wasserstein-1 带容差单调下降（**非严格递减**——Codex 提议，Gemini 同意）
- 验证前提（双方合并）：
  - 仿射规范化（映射到同一纤维）= 尺度归一化
  - 5 个 failure modes：边界抖动、regime shift、跨层不可比、噪声主导、标签偏差

### Layer 3（搁置）: T6
- Leray R¹=0 条件——无 sheaf 工程基础，不可计算
- 保留为"未来规范接口"（Codex 建议，Gemini 同意）

---

## 二、数学修正共识（4项）

### 修正1: 亏格公式
- **编排者原公式**: g(Σ(W)) = #{[b_i, d_i] | d_i - b_i > τ} - 1
- **修正**: 此公式非普适。经典闭曲面 g = β₁/2
- **共识**: 用 **β₁^τ**（τ-显著的第一贝蒂数）替代 g（亏格），剥离流形包袱
- **定义**: β₁^τ(W_k) = #{I ∈ B₁(W_k) | length(I) > τ}

### 修正2: "条形码→亏格"的范畴论表述
- **编排者原说法**: "遗忘函子"
- **Gemini 修正**: "去范畴化（Decategorification）"——K₀(PersMod) 上的秩泛函
- **Codex 补充**: 等价的显式因子分解链：
  FiltTop → PersMod → K₀(PersMod) → ℤ
- **共识**: "去范畴化"是规范术语，映射链是操作实现。两者等价

### 修正3: T8 验证标准
- **编排者原说法**: "Wasserstein-1 严格递减"
- **Codex 修正**: 降格为"带容差 η 的单调下降"：W₁^{(k+1)} ≤ W₁^{(k)} - η
- **Gemini 同意**: 须在仿射规范化后执行
- **共识**: T8 = 带容差的单调下降 + 仿射规范化前提

### 修正4: 计算底层库
- **Gemini 建议**: 引入 scikit-tda/GUDHI——不是工程建议，是"数学必要性"
- **Codex 同意**: 配套版本锁定、系数域固定、回归测试
- **共识**: 底层不变量提取必须用经过同调代数验证的库

---

## 三、编排者洞察评审（逐条共识）

| 洞察 | 判定 | 共识依据 |
|------|------|---------|
| 条形码是亏格的精细化 | ✅ 核心正确，术语修正 | 去范畴化，非遗忘函子 |
| T8 最有价值 | ✅ 同意 | 结构性判据，但需验证 |
| T6 过早 | ✅ 完全同意 | 无 sheaf 工程基础 |
| δ_k = gauge choice | ✅ 完全同意 | 瓶颈稳定性定理推论 |
| 亏格 = 条形码的结算 | ✅ 数学上成立 | 去范畴化过程 |

---

## 四、Layer 1 实现清单（双方共识）

1. **底层**: T1 正确性作为公理——确保 zigzag/EPH 等价语义
2. **barcode 字段**: DecompositionFingerprint 增加 barcode: tuple[tuple[float, float], ...]
3. **β₁^τ 字段**: 替代 genus，= len([bar for bar in barcode if bar[1]-bar[0] > τ])
4. **bottleneck 距离**: gauge_equivalence_report 使用 bottleneck distance
5. **T7 递归单测**: B_{k+1} ⊆ Trim(B_k) 作为递归一致性断言
6. **库依赖**: GUDHI 或 scikit-tda（版本锁定 + 系数域固定）

---

## 五、T8 验证协议（双方合并）

### 前提条件
1. 仿射规范化（波动率/对数尺度归一化）
2. 系数域/滤过构造/trim 规则固定
3. 背驰标签集（人工标注或规则标注）

### 验证步骤
1. 提取已知背驰段（进入段 A，中枢 B，离开段 C）
2. 计算 Dgm(A) 和 Dgm(C)
3. 验证: W₁(Dgm(C)) < W₁(Dgm(A)) - η

### Failure Modes
- 边界抖动导致短条带爆发
- 波动率 regime shift
- 跨层对象不可比
- W₁ 被噪声条带主导（需先 trim）
- 标签前视偏差

---

## 谱系引用

- 195号：缠论代码拓扑化
- 统一方案 8 定理（Gemini × Codex）
- 编排者分层采纳决断
- 亏格构造否定（双方一致）
