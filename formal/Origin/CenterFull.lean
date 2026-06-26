/-
Origin/CenterFull.lean — 扩展中枢类型 CenterFull（core ZG/ZD + 外缘 GG/DD + 段索引），task #116 构造层

★工位定位（#113 still-MISSING-B 缺口 + 审计判决）：canonical `Origin.ChanlunElements.Center`
  只有 {zd, zg, startIndex, endIndex, valid}，**无 GG/DD 外缘字段**。CenterStates.lean 的中心
  定理一/二 + 发展三态用临时结构 `CenterWithOuter` 承载外缘——但 `centersOf : List Segment →
  List Center` 产不出外缘（Center 无字段）。本文件建**独立扩展层** `CenterFull`：core Center +
  GG/DD 外缘 + 索引，承载位置/中心定理所需全部字段，**不改 canonical ChanlunElements**
  （避 Center 改字段 ripple 全 Origin + 与工位B/centersOf 平凡桩 race）。

═══════════════════════════════════════════════════════════════════════════
设计依据（#113 CenterWithOuter 思路独立化 + §6.4 GG/DD 定义）
═══════════════════════════════════════════════════════════════════════════
- §6.4（知识库 + chan99 第八节）：一个中枢由其构成走势段读出六个价位——
  · ZG = min(g₁,g₂)（前两段高点的较小者，中枢上沿核心）
  · ZD = max(d₁,d₂)（前两段低点的较大者，中枢下沿核心）
  · GG = max(gₙ)（所有段高点的最大值，外缘上界）
  · DD = min(dₙ)（所有段低点的最小值，外缘下界）
  · 不变量：DD ≤ ZD ≤ ZG ≤ GG（外缘包含核心）。
- CenterStates.lean 的 `CenterWithOuter` 是同一思路的局部结构；本文件把它提升为 Origin 一等
  扩展类型，并加段索引（startIndex/endIndex）使其能作为 centersOf 的输出载体。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 结构不变量，不依赖数据）。`lake env lean Origin/CenterFull.lean` 通过 =
CenterFull 类型的良构性、外缘包含核心不变量、core 投影与 CenterStates.CenterWithOuter 的桥接
在定义层成立，**不是**任何"中枢识别在真实行情上有效"的实证断言（那是 L2+）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. CenterFull 扩展类型：core + 外缘 GG/DD + 段索引
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **扩展中枢（§6.4）** —— canonical `Center`（[zd,zg] 核心）+ 外缘 GG/DD + 构成段索引。

  - `core`：canonical `Origin.Center`，携带 `zd ≤ zg`（核心区间良构）。
  - `dd`：外缘下界 DD = min(dₙ)（所有构成段最低低点）。
  - `gg`：外缘上界 GG = max(gₙ)（所有构成段最高高点）。
  - `outer_lo : dd ≤ core.zd`、`outer_hi : core.zg ≤ gg`：外缘包含核心（§6.4 DD≤ZD≤ZG≤GG）。

  ★与 canonical `Center` 的关系：`CenterFull.core` 是 canonical `Center`，故 CenterFull 严格
  扩展 canonical 类型而不改它——任何接受 `Center` 的下游（位置三态）直接吃 `cf.core`。
-/
structure CenterFull where
  core : Center
  dd : Tick
  gg : Tick
  outer_lo : dd ≤ core.zd
  outer_hi : core.zg ≤ gg
deriving Repr

/-- 外缘良构推论（L0）：`dd ≤ gg`（外缘下界 ≤ 上界），由 outer_lo + core.valid + outer_hi 链导出。 -/
theorem CenterFull.outer_valid (cf : CenterFull) : cf.dd ≤ cf.gg := by
  have h1 := cf.outer_lo
  have h2 := cf.core.valid
  have h3 := cf.outer_hi
  simp only [Tick] at *
  omega

/-- 核心下界投影：外缘下界 DD ≤ 核心下界 ZD（L0）。 -/
theorem CenterFull.dd_le_zd (cf : CenterFull) : cf.dd ≤ cf.core.zd := cf.outer_lo

/-- 核心上界投影：核心上界 ZG ≤ 外缘上界 GG（L0）。 -/
theorem CenterFull.zg_le_gg (cf : CenterFull) : cf.core.zg ≤ cf.gg := cf.outer_hi

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 与 CenterStates.CenterWithOuter 的桥接（同构投影）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **CenterFull → CenterWithOuter 投影** —— CenterFull 丢弃段索引即得 CenterStates 的
  `CenterWithOuter`。这使本文件的扩展类型直接复用 CenterStates 已证的中心定理一/二 + 发展三态。
-/
def CenterFull.toOuter (cf : CenterFull) : CenterWithOuter :=
  { core := cf.core, dd := cf.dd, gg := cf.gg
    outer_lo := cf.outer_lo, outer_hi := cf.outer_hi }

/-- 投影保 core（L0）。 -/
theorem CenterFull.toOuter_core (cf : CenterFull) : cf.toOuter.core = cf.core := rfl
/-- 投影保 dd（L0）。 -/
theorem CenterFull.toOuter_dd (cf : CenterFull) : cf.toOuter.dd = cf.dd := rfl
/-- 投影保 gg（L0）。 -/
theorem CenterFull.toOuter_gg (cf : CenterFull) : cf.toOuter.gg = cf.gg := rfl

/--
  **★发展三态直接复用（L0）** —— CenterFull 对的发展态分类 = 其投影的发展态分类。
  这把 CenterStates.classifyDevelopment 抬升到 CenterFull 上，无需重证（投影保所有字段）。
-/
def CenterFull.developmentWith (prev next : CenterFull) : CenterDevelopment :=
  classifyDevelopment prev.toOuter next.toOuter

theorem CenterFull.development_total (prev next : CenterFull) :
    DevExtension prev.toOuter next.toOuter ∨
    DevNewBirth prev.toOuter next.toOuter ∨
    DevExpansion prev.toOuter next.toOuter :=
  _root_.NewChanlun.Origin.development_total prev.toOuter next.toOuter

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 反退化见证（具体 CenterFull 数值例子，非平凡）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 具体 CenterFull：核心 [10,20]，外缘 [5,25]，段索引 [0,3]。 -/
def sampleCenterFull : CenterFull :=
  { core := { zd := 10, zg := 20, startIndex := 0, endIndex := 3, valid := by decide }
    dd := 5, gg := 25, outer_lo := by decide, outer_hi := by decide }

/-- ★反退化见证：投影到 CenterWithOuter 后核心位置判定一致（点 15 在核心之中）。 -/
theorem witness_centerFull_within :
    classifyPosition sampleCenterFull.core 15 = CenterPosition.within := by
  unfold classifyPosition sampleCenterFull; decide

/-- ★反退化见证：外缘良构（dd=5 ≤ gg=25）。 -/
theorem witness_centerFull_outer_valid : sampleCenterFull.dd ≤ sampleCenterFull.gg := by
  unfold sampleCenterFull; decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★边界条件（结论翻转）：
    - CenterFull 用闭区间不变量 `dd ≤ zd ≤ zg ≤ gg`（外缘可与核心端点相等，退化为
      "外缘=核心"的扁中枢）。若要求外缘**严格**包含核心（dd < zd ∧ zg < gg），扁中枢被排除，
      须改不变量为严格不等式——当前裁定允许相等（与 CenterStates.CenterWithOuter 一致）。

  ★下游推论：
    - CenterConstruction.lean（centersOf）以 CenterFull 为输出元素类型——本文件提供承载外缘的
      类型，使 centersOf 能产出非平凡中枢（带 GG/DD），消解 #113 still-MISSING-B 的"Center 无外缘"缺口。
    - BspConstruction.lean 第三类买卖点"回抽不破 ZG/ZD"用 CenterFull.core 的位置三态——core 投影
      直接喂 CenterStates 的 IsAbove/IsBelow。

  ★影响声明：
    - 新增 Origin.CenterFull 模块（独立扩展层），**不改** ChanlunElements.Center / CenterStates。
    - import Origin.ChanlunElements + Origin.CenterStates（只读），无反向依赖，无命名冲突。
    - 待 Lead 登记 root：`Origin.CenterFull`。

  ★谱系引用：CenterStates.lean still-MISSING-B（GG/DD 字段缺失）的结构性消解；
    CenterWithOuter 思路（#113）提升为 Origin 一等扩展类型。无概念分离谱系涉及（纯结构扩展）。
-/

end NewChanlun.Origin
