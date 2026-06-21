---
name: orbit-enumeration-duality
description: D∞轨道枚举+H¹同调穷尽+物理重诠释：H⁰=核心仓(2/3)，H¹=机动仓(1/3短差)，Δr=-1=四步减仓-回补路径，单边上扬=H¹幅度→0，τ⊥减仓(范畴正交)
metadata:
  type: project
  originSessionId: cowork-2026-06-16
---

## 2026-06-16 轨道枚举+扬弃调研

### 核心发现

**穷尽不完整的原因**：58条推论只穷尽了H⁰（σ-不变量），没穷尽H¹（变化模式的不变量=扬弃）。

**H¹(D∞, ℝ₋) = ℝ（一维）**：唯一生成元 = φ(σ)=1 = "开仓r=k，闭合r=k-1"。角向Z/23贡献零，H¹纯在径向。

**cd依赖系数环**：
- cd_Q(D∞) = 1 → 实值观测H⁰+H¹闭合（两层辩证法）
- cd_Z(D∞) = ∞ → 整数股数H²=(Z/2)²不消失，量子化阻止综合

### 7条否定推论 NR-1 to NR-7
- NR-1：同级别闭合非必然（Δr=0是σ=e伪影）
- NR-2：move级降成本时间无界（Δt∝λ^{k-1}）
- NR-5：顶层σ越界
- NR-7：整数股数独立手性障碍（H²第二生成元，T51未捕获）

### A1定理（扬弃推论）
跨级别闭合率Δr≠0是导出不变量（1-cocycle，非上边界）= NR-1的正面对偶 = 否定之否定

### 实装方向
P-close：出场从同级别BSP改为向心跨级别φ=0（第64课+T49）

### 文档
- docs/orbit_enumeration_completeness.md（681行，轨道枚举+H¹）
- analysis/spiral_solution_to_underperformance.md（五方向+F裁决）

**Why:** 这解释了为什么58条推论穷尽后引擎仍然7/8跑不赢BH——H⁰不够，需要H¹
**How to apply:** 下一步实装P-close（跨级别闭合），验证是否消除E spawn全亏问题

Related: [[session4-strict-necessity]], [[necessity-accumulation]], [[recursive-nested-fugue]]
