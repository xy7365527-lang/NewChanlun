/-
Origin/CenterConstruct.lean — centersOf 构造层升级：退化平凡桩 → L1 真算（task #16）

★工位定位（构造层 P1）：
  `ChanlunElements.ElementPipeline.centersOf` 的类型签名是 `List Segment → List Center`，
  其唯一性由 `centers_total_unique = total_unique_of_fun P.centersOf` 证——这是平凡桩：
  "函数即全且唯一"对中枢一无所知（ZG/ZD/GG/DD 计算、外缘包含核心、三段重叠均未建模）。

  本文件提供 `centersOfWithOuter : List Segment → List CenterWithOuter`——真算中枢：
  - 基于 CenterConstruction.centersOf（已证终止 + ZG/ZD/GG/DD 构造性计算，#116 工位）
  - 通过 CenterFull.toOuter（#116 投影）得到 List CenterWithOuter
  - 每个输出元素携带 core.valid（ZD ≤ ZG）+ outer_lo（DD ≤ ZD）+ outer_hi（ZG ≤ GG）不变量
  - 具体数值 witness（ZG/ZD/GG/DD 均机器算）证明输出非平凡

═══════════════════════════════════════════════════════════════════════════
权威来源（§6.3/§6.4 ZG/ZD/GG/DD 公式，口径 B 全三段，637号）
═══════════════════════════════════════════════════════════════════════════
- §6.3/§6.4（一级权威第17课答疑严格公式 + 知识库 + chan99 第八节）：
  · ZG = min(g₁,g₂,g₃)（**全三段**高点的最小者，中枢核心上沿，口径 B）
  · ZD = max(d₁,d₂,d₃)（**全三段**低点的最大者，中枢核心下沿，口径 B）
  · GG = max(gₙ)（所有构成段最高高点，外缘上界）
  · DD = min(dₙ)（所有构成段最低低点，外缘下界）
  · 不变量：DD ≤ ZD ≤ ZG ≤ GG（外缘包含核心；口径 B 下核心与外缘同三段聚合）。
  · ★口径 A→B 迁移（637号）：误口径 A（前两段 min(g₁,g₂)/max(d₁,d₂)）已被一级权威第17课答疑
    严格公式 `(max(a2,b2,c2), min(a1,b1,c1))` 裁错——canonical = B 全三段。本文件 centersOf 路径
    已迁 B，与 § 5.5 ref_v1 路径（本就 B 全三段）**口径统一**。
- §6.1（中枢定义）："走势中枢：某级别走势类型中，被至少三个连续次级别走势类型所重叠的部分。"
  本文件用几何必要条件（ZD ≤ ZG）作识别判据（诚实边界：still-MISSING-B′ 完整判据见
  CenterComplete.lean）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
**L1**（真编码 + 数值 witness）：
- centersOfWithOuter 是真算函数（非 total_unique_of_fun 平凡桩）：从线段列自动计算 ZG/ZD/GG/DD，
  产出带不变量证明的 CenterWithOuter 列表。
- 具体数值 witness（§ 3/4）由 decide/rfl 机器检查：给定具体段，输出的 ZG/ZD/GG/DD 值
  被机器验证与 §6.4 公式一致——这验证"管线正确性"（即编码没有 bug）。
- 信息增量诚实标注：L1 验证管线正确，不验证"在真实行情数据上识别的中枢符合缠论权威标注"
  （那是 L2+，须真实 K 线数据比对）。
- L0 vs L1 区分：L0 = 纯定义/结构不变量（不依赖数据，如 outer_lo/outer_hi 不变量证明）；
  L1 = 合成数值 witness（验证管线，输入由本文件构造）。本文件同时含 L0（不变量证明）和 L1（witness）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.CenterFull
import Origin.CenterConstruction
import Lean.Data.Json

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. centersOfWithOuter：List Segment → List CenterWithOuter（真算，非平凡桩）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **centersOfWithOuter 全自动构造（L1，task #16 构造层升级）** ——
  `List Segment → List CenterWithOuter`。

  实现：将 CenterConstruction.centersOf（产 List CenterFull，#116 真算，终止且唯一）
  通过 CenterFull.toOuter（投影，丢弃段索引保留 core/dd/gg 及不变量）
  逐元素投影得到 List CenterWithOuter。

  每个输出元素 `cwo : CenterWithOuter` 携带：
  - `cwo.core`：canonical Center，含 `zd ≤ zg`（核心区间良构，口径 B 全三段：ZD = max(d₁,d₂,d₃)，
    ZG = min(g₁,g₂,g₃)，637号）
  - `cwo.dd`：外缘下界 DD = min(d₁,d₂,d₃)
  - `cwo.gg`：外缘上界 GG = max(g₁,g₂,g₃)
  - `cwo.outer_lo : dd ≤ core.zd`（DD ≤ ZD，外缘包含核心下沿）
  - `cwo.outer_hi : core.zg ≤ gg`（ZG ≤ GG，外缘包含核心上沿）

  ★非平凡性区分：ChanlunElements.centers_total_unique = total_unique_of_fun P.centersOf
  是平凡桩——"任何函数的关系全且唯一"，对 ZG/ZD/GG/DD 计算一无所知。本函数显式按
  §6.4 公式计算每个价位，产出有结构约束的输出，是真算（不是函数包装）。
-/
def centersOfWithOuter (segs : List Segment) : List CenterWithOuter :=
  (centersOf segs).map CenterFull.toOuter

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 基本性质（L0，纯定义推论）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **centersOfWithOuter 输出唯一性（L0）** —— 纯全函数 ⟹ 输出唯一确定。
  升级平凡桩 `total_unique_of_fun P.centersOf`：本函数的唯一性同样由"纯函数"导出，
  但函数内容是真算（非恒等包装）。
-/
theorem centersOfWithOuter_total_unique :
    TotalUnique (fun segs out => centersOfWithOuter segs = out) :=
  total_unique_of_fun centersOfWithOuter

/--
  **centersOfWithOuter 全性（L0，终止性推论）** —— 对任意线段列终止并返回。
  继承自 centersOf 的 well-founded 递归终止性（CenterConstruction.lean §4）。
-/
theorem centersOfWithOuter_total :
    Total (fun segs out => centersOfWithOuter segs = out) := by
  intro segs; exact ⟨centersOfWithOuter segs, rfl⟩

/--
  **输出元素满足 CenterWithOuter 不变量（L0）** —— centersOfWithOuter 每个输出元素
  `cwo` 满足 `cwo.outer_lo : cwo.dd ≤ cwo.core.zd` 和 `cwo.outer_hi : cwo.core.zg ≤ cwo.gg`。
  这是不变量在列表级别的提升（由 CenterFull.toOuter 保不变量 + List.map 逐元素）。
-/
theorem centersOfWithOuter_invariant (segs : List Segment) :
    ∀ cwo ∈ centersOfWithOuter segs,
      cwo.dd ≤ cwo.core.zd ∧ cwo.core.zd ≤ cwo.core.zg ∧ cwo.core.zg ≤ cwo.gg := by
  intro cwo hcwo
  -- centersOfWithOuter segs = (centersOf segs).map CenterFull.toOuter
  simp only [centersOfWithOuter, List.mem_map] at hcwo
  obtain ⟨cf, _, hcf⟩ := hcwo
  rw [← hcf]
  -- CenterFull.toOuter 保 outer_lo/core.valid/outer_hi
  exact ⟨cf.outer_lo, cf.core.valid, cf.outer_hi⟩

/--
  **centersOfWithOuter 长度 = centersOf 长度（L0）** —— map 保列表长度，
  故 centersOfWithOuter 和 centersOf 识别同样多的中枢。
-/
theorem centersOfWithOuter_length (segs : List Segment) :
    (centersOfWithOuter segs).length = (centersOf segs).length := by
  simp only [centersOfWithOuter, List.length_map]

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 具体数值 witness（L1，ZG/ZD/GG/DD 机器验证，反平凡桩证据）
    ═══════════════════════════════════════════════════════════════════════ -/

/-
  witness 三段（上-下-上，§6.1 典型形态，核心 ZG/ZD 已在 CenterConstruction ovSeg* 定义）：
  - s1 = 上(10→20)：segHigh=20, segLow=10
  - s2 = 下(20→12)：segHigh=20, segLow=12
  - s3 = 上(12→22)：segHigh=22, segLow=12
  §6.3/§6.4 计算（口径 B 全三段，637号）：
  - ZG = min(g₁,g₂,g₃) = min(20,20,22) = 20（此 witness 第三段贯穿 ⟹ A/B 重合）
  - ZD = max(d₁,d₂,d₃) = max(10,12,12) = 12（此 witness A/B 重合）
  - GG = max(g₁,g₂,g₃) = max(20,20,22) = 22
  - DD = min(d₁,d₂,d₃) = min(10,12,12) = 10
  不变量：DD=10 ≤ ZD=12 ≤ ZG=20 ≤ GG=22 ✓
-/

-- 使用 CenterConstruction 已定义的段（避免重复定义）

/--
  **★L1 数值 witness：centersOfWithOuter 产恰一个中枢（上-下-上三段）** ——
  给定三段（上-下-上，核心重叠），centersOfWithOuter 产出长度为 1 的列表，
  机器验证不是空列表（非平凡）。
-/
theorem witness_centersOfWithOuter_one :
    (centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]).length = 1 := by
  unfold centersOfWithOuter
  simp only [List.length_map]
  exact witness_centersOf_one

/--
  **★L1 数值 witness：ZG 值为 20（§6.4 公式验证）** ——
  centersOfWithOuter 第一个中枢的 core.zg = 20 = min(segHigh ovSeg1, segHigh ovSeg2)
  = min(20, 20)。机器验证公式执行正确。
-/
theorem witness_zg_value :
    let centers := centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]
    centers.head? >>= (fun c => some c.core.zg) = some 20 := by
  native_decide

/--
  **★L1 数值 witness：ZD 值为 12（§6.4 公式验证）** ——
  centersOfWithOuter 第一个中枢的 core.zd = 12 = max(segLow ovSeg1, segLow ovSeg2)
  = max(10, 12)。机器验证公式执行正确。
-/
theorem witness_zd_value :
    let centers := centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]
    centers.head? >>= (fun c => some c.core.zd) = some 12 := by
  native_decide

/--
  **★L1 数值 witness：GG 值为 22（§6.4 公式验证）** ——
  centersOfWithOuter 第一个中枢的 gg = 22 = max(segHigh ovSeg1, segHigh ovSeg2, segHigh ovSeg3)
  = max(20, 20, 22)。机器验证外缘上界。
-/
theorem witness_gg_value :
    let centers := centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]
    centers.head? >>= (fun c => some c.gg) = some 22 := by
  native_decide

/--
  **★L1 数值 witness：DD 值为 10（§6.4 公式验证）** ——
  centersOfWithOuter 第一个中枢的 dd = 10 = min(segLow ovSeg1, segLow ovSeg2, segLow ovSeg3)
  = min(10, 12, 12)。机器验证外缘下界。
-/
theorem witness_dd_value :
    let centers := centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]
    centers.head? >>= (fun c => some c.dd) = some 10 := by
  native_decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 具体中枢不变量见证（L0 + L1，ZG/ZD/GG/DD 正确且满足 DD≤ZD≤ZG≤GG）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★具体构造：三段 witness 的 CenterWithOuter 实例** —— 从 ovSeg1/ovSeg2/ovSeg3
  直接构造 CenterWithOuter（通过 CenterConstruction.centerFromThree + toOuter），
  机器验证其 ZG=20/ZD=12/GG=22/DD=10 且满足不变量 DD≤ZD≤ZG≤GG。
-/
def witnessCenter : CenterWithOuter :=
  (centerFromThree ovSeg1 ovSeg2 ovSeg3 overlapping_holds).toOuter

/-- ★ZG 值机器验证（L0，由定义直接读，口径 B 全三段）：core.zg = min(20,20,22) = 20。 -/
theorem witnessCenter_zg : witnessCenter.core.zg = 20 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeZG
  unfold tmin segHigh ovSeg1 ovSeg2 ovSeg3; decide

/-- ★ZD 值机器验证（L0，由定义直接读，口径 B 全三段）：core.zd = max(10,12,12) = 12。 -/
theorem witnessCenter_zd : witnessCenter.core.zd = 12 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeZD
  unfold tmax segLow ovSeg1 ovSeg2 ovSeg3; decide

/-- ★GG 值机器验证（L0，由定义直接读）：gg = 22。 -/
theorem witnessCenter_gg : witnessCenter.gg = 22 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeGG
  unfold tmax segHigh ovSeg1 ovSeg2 ovSeg3; decide

/-- ★DD 值机器验证（L0，由定义直接读）：dd = 10。 -/
theorem witnessCenter_dd : witnessCenter.dd = 10 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeDD
  unfold tmin segLow ovSeg1 ovSeg2 ovSeg3; decide

/--
  **★具体不变量：DD=10 ≤ ZD=12 ≤ ZG=20 ≤ GG=22（L0，外缘包含核心）** ——
  witnessCenter 的四个价位满足 §6.4 链式不等式——这是"中枢不变量在具体实例上成立"
  的机器证明，是反平凡桩的核心：平凡桩不携带任何价位信息，此处每个价位均可机器读出并验证。
-/
theorem witnessCenter_chain : witnessCenter.dd ≤ witnessCenter.core.zd ∧
    witnessCenter.core.zd ≤ witnessCenter.core.zg ∧
    witnessCenter.core.zg ≤ witnessCenter.gg := by
  refine ⟨?_, ?_, ?_⟩
  · -- DD=10 ≤ ZD=12
    have h1 := witnessCenter_dd
    have h2 := witnessCenter_zd
    simp only [Tick] at *; omega
  · -- ZD=12 ≤ ZG=20
    have h1 := witnessCenter_zd
    have h2 := witnessCenter_zg
    simp only [Tick] at *; omega
  · -- ZG=20 ≤ GG=22
    have h1 := witnessCenter_zg
    have h2 := witnessCenter_gg
    simp only [Tick] at *; omega

/--
  **★ZG 分类（L0，中枢位置三态应用）** —— 价格 15 落在 witnessCenter 核心 [ZD=12, ZG=20]
  之中（IsWithin），机器验证 classifyPosition 返回 CenterPosition.within。
  这把具体 ZG/ZD 值接入 CenterStates 的位置三态分类器——端到端机器验证。
-/
theorem witnessCenter_position_within :
    classifyPosition witnessCenter.core 15 = CenterPosition.within := by
  unfold classifyPosition witnessCenter CenterFull.toOuter centerFromThree
  unfold computeZG computeZD tmin tmax segHigh segLow ovSeg1 ovSeg2 ovSeg3
  decide

/--
  **★GG/DD 中心定理二应用 witness（L0）** —— 构造后中枢外缘 DD'=28 > 前 GG=22，
  机器验证 IsUpTrend（上涨趋势/新生），§6.5 中心定理二的具体数值例。
  这把 centersOfWithOuter 的真算输出接入 CenterStates.IsUpTrend——端到端非平凡性。
-/
def witnessUpNext : CenterWithOuter :=
  { core := { zd := 30, zg := 40, startIndex := 4, endIndex := 7, valid := by decide }
    dd := 28, gg := 45, outer_lo := by decide, outer_hi := by decide }

theorem witnessCenter_upTrend :
    IsUpTrend witnessCenter witnessUpNext := by
  unfold IsUpTrend witnessUpNext witnessCenter CenterFull.toOuter centerFromThree
  unfold computeGG tmax segHigh ovSeg1 ovSeg2 ovSeg3
  decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 与 ChanlunElements 平凡桩的差异声明（L0，诚实区分）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★反平凡桩核心定理（L0）** —— centersOfWithOuter 产出 CenterWithOuter 列表，其中
  每个元素的 dd/gg/core.zd/core.zg 由 §6.4 公式计算（非占位符）。可机器读出具体值
  （witness_zg_value/witness_zd_value/witness_gg_value/witness_dd_value）。
  相比之下，ChanlunElements.centers_total_unique = total_unique_of_fun P.centersOf 只证
  "给定函数是全且唯一的关系"——对中枢的价位结构一无所知。两者的差异不是命名，而是内容。
-/
theorem centersOfWithOuter_nontrivial :
    ∃ segs : List Segment, ∃ cwo : CenterWithOuter,
      cwo ∈ centersOfWithOuter segs ∧
      cwo.dd ≤ cwo.core.zd ∧ cwo.core.zd ≤ cwo.core.zg ∧ cwo.core.zg ≤ cwo.gg ∧
      cwo.core.zd = 12 ∧ cwo.core.zg = 20 ∧ cwo.dd = 10 ∧ cwo.gg = 22 := by
  refine ⟨[ovSeg1, ovSeg2, ovSeg3], witnessCenter, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- witnessCenter ∈ centersOfWithOuter [ovSeg1, ovSeg2, ovSeg3]
    unfold centersOfWithOuter witnessCenter
    simp only [List.mem_map]
    refine ⟨centerFromThree ovSeg1 ovSeg2 ovSeg3 overlapping_holds, ?_, rfl⟩
    -- centerFromThree ... ∈ centersOf [ovSeg1, ovSeg2, ovSeg3]
    have h : centersOf [ovSeg1, ovSeg2, ovSeg3] =
             [centerFromThree ovSeg1 ovSeg2 ovSeg3 overlapping_holds] := by
      rw [centersOf.eq_def]
      dsimp only []
      rw [dif_pos overlapping_holds, centersOf.eq_def]
    rw [h]; simp
  · exact witnessCenter_chain.1
  · exact witnessCenter_chain.2.1
  · exact witnessCenter_chain.2.2
  · exact witnessCenter_zd
  · exact witnessCenter_zg
  · exact witnessCenter_dd
  · exact witnessCenter_gg

/-! ═══════════════════════════════════════════════════════════════════════
    § 5.5 reference v1 引擎语义（engine-audit 包 reference_chanlun.py 逐字直译）
    ═══════════════════════════════════════════════════════════════════════

    ★工位定位（第Ⅱ类中枢 reference 语义，codex 裁决「全部必须形式化+实装+parity」）：
      上面 §1-5 的 `centersOfWithOuter` 走 `CenterConstruction.centersOf` 路径——核心
      `ZD=max(d₁,d₂,d₃)/ZG=min(g₁,g₂,g₃)`（**全三段**，口径 B，637号迁移后与本 § ref_v1 核心
      口径统一）+ 固定三段窗口。本 § 兑现**另一条权威链**：
      `~/Downloads/newchanlun-engine-formal-audit/reference_chanlun.py`
      的 **frozen v1 rule**（README §1）——`ZD=max3(lows)/ZG=min3(highs)`（**全三段**）+ 弱接触延伸
      + gg/dd/settled 生命周期 + 结算后重叠回退 `i=max(j-2,end)`。
      ★口径收敛（637号）：迁移前 §1-5 路径核心是误口径 A（前两段），与本 § ref_v1（B 全三段）
      **核心口径分离**；迁移后两路径核心**统一为 B**，区别仅剩本 § ref_v1 额外的弱接触延伸 +
      生命周期字段（§1-5 路径用固定三段窗口，不做延伸）。`legacyV0Interval`（首尾段）仍是被裁错
      的第三种口径（与 A、B 均不同）。

    ★v0/v1 反例裁决（NewChanlunEngineAudit.lean，已 native_decide 裁 v0 错）：
      fixture `s0=[10,20],s1=[12,15],s2=[11,18]` 上 v1=[12,15]、legacy_v0=[11,18]
      （v0 用首尾段 `max(l0,l2)/min(h0,h2)`）。本 § 实装 v1 正确语义（全三段），
      `legacy_v0_not_correct_for_v1_reference` 在 Lean 重证（refV1Interval ≠ legacyV0Interval）。

    ★认识论等级（formalization-validity-domain 231号，强制）：
      全部 **L0**——纯整数 max3/min3 + fuel-bounded 滑窗（终止机器检查）+ 弱接触延伸 +
      生命周期字段，不依赖经验数据。`lake env lean Origin/CenterConstruct.lean` 通过 =
      reference v1 算法在定义层逐字忠实，**不是**任何「v1 中枢在真实行情上有效」的 L2 断言。
-/

/--
  **reference RefZhongshu（reference_chanlun.py:40-52 逐字镜像）** —— v1 中枢生命周期字段。

  - `zd/zg`：固定核心区间（**全三段** `max3(lows)/min3(highs)`，frozen v1）。
  - `start/end`：构成段在 completed 序列中的起止下标（end 随弱接触延伸增长）。
  - `count`：构成段数 `end - start + 1`。
  - `settled`：是否已结算（`j < n`——延伸后仍有后续段离开核心则结算）。
  - `breakIndex`：结算时离开核心的段下标（未结算 = -1，用 `Int`）。
  - `breakUp`：离开方向（reference `break_direction` "up"/"down" 的布尔编码，true=up）。
  - `gg/dd`：外缘上下界（初始三段 + 弱接触延伸段聚合 `max/min`）。
-/
structure RefZhongshu where
  zd : Tick
  zg : Tick
  start : Nat
  «end» : Nat
  count : Nat
  settled : Bool
  breakIndex : Int
  breakUp : Bool
  gg : Tick
  dd : Tick
deriving Repr, DecidableEq

/-- 三元 max（reference `max(a,b,c)`，全三段核心 ZD 用）。 -/
def max3 (a b c : Tick) : Tick := tmax (tmax a b) c
/-- 三元 min（reference `min(a,b,c)`，全三段核心 ZG 用）。 -/
def min3 (a b c : Tick) : Tick := tmin (tmin a b) c

/--
  **reference 弱接触延伸判据（reference_chanlun.py:78-85 逐字）** —— 延伸用弱相交：
  `component.high >= zd and component.low <= zg`（边界相切算延伸，frozen 测试口径）。
-/
def weakOverlap (compHigh compLow zd zg : Tick) : Bool :=
  decide (compHigh ≥ zd) && decide (compLow ≤ zg)

/--
  **reference 离开方向（reference_chanlun.py:88-93 逐字）** —— 结算段相对核心的离开方向。
  `low > zg → up`；`high < zd → down`；否则 `high > zg ? up : down`（防御分支）。返回 true=up。
-/
def breakDirectionUp (compHigh compLow zd zg : Tick) : Bool :=
  if compLow > zg then true
  else if compHigh < zd then false
  else decide (compHigh > zg)

/--
  **completed 段的 (high, low) 投影** —— 中枢算法只读区间端点（reference ComponentLike）。
  对齐 `CenterConstruction.segHigh/segLow`（向上段 hi=endPrice/向下段 lo=endPrice 已规约）。
-/
def segHL (s : Segment) : Tick × Tick := (segHigh s, segLow s)

/--
  **弱接触延伸内循环（reference_chanlun.py:129-134 的 `while j<n and weak_overlap`）** ——
  从下标 `j` 起，对固定核心 `[zd,zg]` 逐段做弱接触延伸，聚合 gg/dd，返回 `(end, j', gg', dd')`。
  fuel = 剩余段数上界（保证 ≥ 实际可延伸段数，终止机器检查；reference while 由 `j<n` 终止）。

  返回 `(end, jOut, gg, dd)`：`end`=最后延伸到的段下标，`jOut`=首个不延伸（离开/越界）的段下标。
-/
def extendWeak (comps : List (Tick × Tick)) (zd zg : Tick) :
    Nat → Nat → Nat → Tick → Tick → (Nat × Nat × Tick × Tick)
  | 0,        endAcc, j, gg, dd => (endAcc, j, gg, dd)  -- fuel 耗尽（不可达：fuel ≥ 剩余段数）
  | fuel + 1, endAcc, j, gg, dd =>
      match comps[j]? with
      | none => (endAcc, j, gg, dd)  -- j 越界（reference `j < n` 假）
      | some (h, l) =>
          if weakOverlap h l zd zg then
            -- 延伸：end=j，聚合 gg/dd，j+=1，fuel-1
            extendWeak comps zd zg fuel j (j + 1) (tmax gg h) (tmin dd l)
          else
            (endAcc, j, gg, dd)  -- 段离开核心（reference while 条件假，settled 候选）

/--
  **reference 主滑窗（reference_chanlun.py:117-155 的 `while i+2 < n`）** ——
  从下标 `i` 起识别一个 v1 中枢，返回 `(中枢?, 下一个 i)`。fuel 保证终止（每步 i 严格增）。

  逐字对齐 reference：
  - 前三段 `zd=max3(lows)/zg=min3(highs)`（全三段核心，frozen v1）。
  - `zg <= zd`（核心空）⟹ 无中枢，`i+=1` 滑窗（reference:122-124）。
  - 否则：gg/dd 初始三段，弱接触延伸（extendWeak），`settled = jOut < n`。
  - `settled` ⟹ 下一个 `i = max(jOut-2, end)`（重叠回退，reference:150-151）；
    未结算 ⟹ 停止（reference:152-153 break，返回 i=n 终止外循环）。
-/
def refStep (comps : List (Tick × Tick)) (n : Nat) (i : Nat) :
    (Option RefZhongshu × Nat) :=
  match comps[i]?, comps[i+1]?, comps[i+2]? with
  | some (h1, l1), some (h2, l2), some (h3, l3) =>
      let zd := max3 l1 l2 l3
      let zg := min3 h1 h2 h3
      if decide (zg ≤ zd) then
        (none, i + 1)  -- 核心空：滑窗前进 1（reference:122-124）
      else
        let gg0 := max3 h1 h2 h3
        let dd0 := min3 l1 l2 l3
        -- 弱接触延伸从 j=i+3 起（fuel = n 充分上界）
        let (endE, jOut, gg, dd) := extendWeak comps zd zg n (i + 2) (i + 3) gg0 dd0
        let settled := decide (jOut < n)
        let zs : RefZhongshu :=
          { zd := zd, zg := zg, start := i, «end» := endE, count := endE - i + 1,
            settled := settled,
            breakIndex := if settled then (jOut : Int) else (-1 : Int),
            breakUp := if settled then
                         (match comps[jOut]? with
                          | some (hb, lb) => breakDirectionUp hb lb zd zg
                          | none => true)
                       else true,
            gg := gg, dd := dd }
        if settled then
          -- 重叠回退：i = max(jOut-2, end)（reference:150-151，Nat 减法 jOut-2 已自带 ≥0 截断）
          (some zs, Nat.max (jOut - 2) endE)
        else
          (some zs, n)  -- 未结算：break（i=n 终止外循环，reference:152-153）
  | _, _, _ => (none, n)  -- 不足三段：终止（reference `i+2 < n` 假）

/--
  **reference 外循环（reference_chanlun.py:115-155 的 `while i+2 < n` 全体）** ——
  fuel-bounded 递归收集所有 v1 中枢。fuel = n（i 每步严格增 ⟹ 至多 n 步，终止机器检查）。
-/
def refLoop (comps : List (Tick × Tick)) (n : Nat) : Nat → Nat → List RefZhongshu
  | 0,        _ => []  -- fuel 耗尽（不可达：fuel = n ≥ 步数）
  | fuel + 1, i =>
      if decide (i + 2 < n) then
        let (zsOpt, iNext) := refStep comps n i
        -- iNext 保证 > i（核心空 i+1；settled max(jOut-2,end)≥i；未结算 iNext=n>i）
        -- fuel-1 配合 iNext 单调增保证终止
        match zsOpt with
        | some zs => zs :: refLoop comps n fuel iNext
        | none    => refLoop comps n fuel iNext
      else []

/--
  **★reference v1 中枢构造（reference_chanlun.py `zhongshus_from_components_ref`，逐字）** ——
  `List Segment → List RefZhongshu`。完成态段 → (high,low) 投影 → v1 滑窗。

  对齐 reference:96-155：`n = len(completed)`，`n < 3 ⟹ []`，否则 refLoop 收集。
  本函数不做 `completed_mask` 过滤（输入已是完成态段；mask 过滤在调用点，reference:158-160）。
-/
def refZhongshusFromComponents (segs : List Segment) : List RefZhongshu :=
  let comps := segs.map segHL
  let n := comps.length
  if decide (n < 3) then [] else refLoop comps n n 0

/-! ──────────────────────────────────────────────────────────────────────
    § 5.5.1 v0/v1 反例裁决（NewChanlunEngineAudit.lean Lean 重证，L0）
    ────────────────────────────────────────────────────────────────────── -/

/-- **reference v1 核心区间（全三段，reference:119-120）** —— `(max3 lows, min3 highs)`。 -/
def refV1Interval (l0 h0 l1 h1 l2 h2 : Tick) : Tick × Tick :=
  (max3 l0 l1 l2, min3 h0 h1 h2)

/-- **legacy v0 核心区间（首尾段，NewChanlunEngineAudit.lean v0Interval）** —— `(max(l0,l2), min(h0,h2))`。 -/
def legacyV0Interval (l0 h0 _l1 _h1 l2 h2 : Tick) : Tick × Tick :=
  (tmax l0 l2, tmin h0 h2)

/-- **★v1 区间值（reference fixture s0=[10,20],s1=[12,15],s2=[11,18]）= (12,15)（L0）。** -/
theorem refV1_fixture_value : refV1Interval 10 20 12 15 11 18 = (12, 15) := by native_decide

/-- **★v0 区间值（同 fixture）= (11,18)（L0）。** -/
theorem legacyV0_fixture_value : legacyV0Interval 10 20 12 15 11 18 = (11, 18) := by native_decide

/-- **★v0 ≠ v1（同 fixture，L0）** —— 同一三段两口径核心区间不同。 -/
theorem v0_ne_v1_fixture :
    legacyV0Interval 10 20 12 15 11 18 ≠ refV1Interval 10 20 12 15 11 18 := by native_decide

/--
  **★legacy v0 不是 v1 reference 的正确实现（NewChanlunEngineAudit.lean
  `legacy_v0_not_correct_for_v1_reference` Lean 重证，L0）** ——
  不存在「v0 在全部输入上等于 v1」（fixture 即反例）。这裁决：实装中枢必须用 v1 全三段。
-/
theorem legacyV0_not_correct_for_v1 :
    ¬ (∀ l0 h0 l1 h1 l2 h2 : Tick,
        legacyV0Interval l0 h0 l1 h1 l2 h2 = refV1Interval l0 h0 l1 h1 l2 h2) := by
  intro hall
  exact v0_ne_v1_fixture (hall 10 20 12 15 11 18)

/-! ──────────────────────────────────────────────────────────────────────
    § 5.5.2 reference v1 数值 witness（L0，机器导出 parity fixture 源）
    ────────────────────────────────────────────────────────────────────── -/

/-- reference fixture 三段（v1 反例 s0=[10,20],s1=[12,15],s2=[11,18]）的 Segment 编码。 -/
def refSeg0 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def refSeg1 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 15, endPrice := 12 }
def refSeg2 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 11, endPrice := 18 }

/--
  **★reference v1 三段 fixture 产恰一个中枢（L0 witness）** —— 三段核心非空（zg=15>zd=12），
  无后续段 ⟹ 弱接触延伸不触发，`jOut=3=n ⟹ settled=False`（末中枢强制 unsettled，reference:136）。
-/
theorem witness_refZhongshu_one :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).length = 1 := by native_decide

/-- **★v1 核心 zd=12（全三段 max3(10,12,11)，L0）。** -/
theorem witness_ref_zd :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).head?.map (·.zd) = some 12 := by
  native_decide

/-- **★v1 核心 zg=15（全三段 min3(20,15,18)，L0）。** -/
theorem witness_ref_zg :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).head?.map (·.zg) = some 15 := by
  native_decide

/-- **★v1 外缘 gg=20（max3 highs，L0）。** -/
theorem witness_ref_gg :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).head?.map (·.gg) = some 20 := by
  native_decide

/-- **★v1 外缘 dd=10（min3 lows，L0）。** -/
theorem witness_ref_dd :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).head?.map (·.dd) = some 10 := by
  native_decide

/-- **★末中枢强制 unsettled（reference:136 `settled = j < n`，无后续段 ⟹ jOut=n ⟹ False，L0）。** -/
theorem witness_ref_last_unsettled :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).head?.map (·.settled) = some false := by
  native_decide

/-! ──────────────────────────────────────────────────────────────────────
    § 5.5.3 弱接触延伸 + 结算 + 重叠回退 witness（多段，L0）
    ────────────────────────────────────────────────────────────────────── -/

/-- 延伸 fixture 第四段（弱接触：high=16≥zd=12 ∧ low=13≤zg=15 ⟹ 延伸吸收）。 -/
def refSeg3Ext : Segment := { direction := Direction.down, startIndex := 3, endIndex := 4, startPrice := 16, endPrice := 13 }
/-- 结算段（离开核心向上：low=20>zg=15 ⟹ 不弱接触 ⟹ 中枢结算，break_direction=up）。 -/
def refSeg4Break : Segment := { direction := Direction.up, startIndex := 4, endIndex := 5, startPrice := 25, endPrice := 22 }

/--
  **★弱接触延伸吸收第四段（reference:130-134，L0）** —— 四段序列 [refSeg0..refSeg3Ext]
  核心 [12,15]，第四段 [13,16] 弱接触 ⟹ end 延伸到下标3，count=4。末中枢 unsettled。
-/
theorem witness_ref_extends_to_four :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2, refSeg3Ext]).head?.map (·.count)
      = some 4 := by native_decide

/--
  **★结算段触发 settled + break_direction=up（reference:136-145，L0）** —— 五段序列
  [refSeg0..refSeg2, refSeg3Ext, refSeg4Break]：核心 [12,15]，第四段延伸（count→4），
  第五段 [22,25] low=22>zg=15 离开 ⟹ jOut=4<n=5 ⟹ settled=True，breakUp=true。
-/
theorem witness_ref_settled_break_up :
    (refZhongshusFromComponents [refSeg0, refSeg1, refSeg2, refSeg3Ext, refSeg4Break]).head?.map
      (fun zs => (zs.settled, zs.breakUp, zs.breakIndex)) = some (true, true, (4 : Int)) := by
  native_decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 5.5.4 机器导出 parity fixture（#eval → JSON，rust include_str! 耦合）
    ═══════════════════════════════════════════════════════════════════════ -/

open Lean (Json) in
/-- 把一个 RefZhongshu 序列化为 JSON 对象（所有字段来自真求值，机器产）。 -/
def refZhongshuJson (zs : RefZhongshu) : Json :=
  Json.mkObj [
    ("zd", Json.num zs.zd), ("zg", Json.num zs.zg),
    ("gg", Json.num zs.gg), ("dd", Json.num zs.dd),
    ("start", Json.num (zs.start : Int)), ("end", Json.num (zs.«end» : Int)),
    ("count", Json.num (zs.count : Int)),
    ("settled", Json.bool zs.settled),
    ("break_index", Json.num zs.breakIndex),
    ("break_up", Json.bool zs.breakUp)
  ]

open Lean (Json) in
/--
  **★reference v1 parity fixture（所有数值/枚举来自 refZhongshusFromComponents 真求值，机器产）** ——
  rust `theta_v0_center_parity.rs` 用 include_str! 读同一 fixture，bit-exact 断言 rust 计算 == Lean 导出。

  三个 fixture case（机器产，覆盖核心计算 + 弱接触延伸 + 结算回退）：
  - `three_unsettled`：v1 反例三段 [10,20][12,15][11,18] → 恰一中枢，末 unsettled。
  - `four_extend`：加弱接触第四段 → count=4 延伸。
  - `five_settled_break`：加结算第五段 → settled + break up。
  - `v0_v1_interval`：v0/v1 核心区间对照（裁决 v0 错）。
-/
def refV1FixtureJson : Json :=
  Json.mkObj [
    ("meta", Json.mkObj [
      ("source", Json.str "formal/Origin/CenterConstruct.lean #eval refZhongshusFromComponents (machine-exported)"),
      ("epistemic_level", Json.str "L0"),
      ("note", Json.str "reference v1 中枢语义 bit-exact Lean machine-checked; 非 L2 经验断言")
    ]),
    ("three_unsettled", Json.arr
      ((refZhongshusFromComponents [refSeg0, refSeg1, refSeg2]).map refZhongshuJson).toArray),
    ("four_extend", Json.arr
      ((refZhongshusFromComponents [refSeg0, refSeg1, refSeg2, refSeg3Ext]).map refZhongshuJson).toArray),
    ("five_settled_break", Json.arr
      ((refZhongshusFromComponents [refSeg0, refSeg1, refSeg2, refSeg3Ext, refSeg4Break]).map refZhongshuJson).toArray),
    ("v0_v1_interval", Json.mkObj [
      ("fixture", Json.str "s0=[10,20],s1=[12,15],s2=[11,18]"),
      ("v1_zd", Json.num (refV1Interval 10 20 12 15 11 18).1),
      ("v1_zg", Json.num (refV1Interval 10 20 12 15 11 18).2),
      ("v0_zd", Json.num (legacyV0Interval 10 20 12 15 11 18).1),
      ("v0_zg", Json.num (legacyV0Interval 10 20 12 15 11 18).2)
    ])
  ]

-- ★机器导出入口：#eval 输出裸 fixture JSON（单行 compress），重定向落盘为
-- rust/tests/fixtures/theta_v0_center_parity.json。fixture 内容由本 #eval 唯一确定（非手编）。
#eval IO.println refV1FixtureJson.compress

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 边界条件 + 下游推论 + 影响声明 + 谱系引用（结果包六要素）
    ═══════════════════════════════════════════════════════════════════════

  ★结论（task #16 + 637号口径 A→B 迁移）：
    centersOfWithOuter : List Segment → List CenterWithOuter 真算实装。
    - 从线段序列自动识别中枢并计算 ZG=min(g₁,g₂,g₃)/ZD=max(d₁,d₂,d₃)（口径 B 全三段）/
      GG=max(gₙ)/DD=min(dₙ)
    - 每个输出元素携带机器证明的不变量 DD≤ZD≤ZG≤GG
    - 4 个数值 witness（ZG=20/ZD=12/GG=22/DD=10，此 witness 第三段贯穿 ⟹ A/B 重合）机器验证公式正确性
    - 1 个端到端见证（classifyPosition 接入 witnessCenter，返回 within）
    - 1 个中心定理二见证（IsUpTrend witnessCenter witnessUpNext）
    - 认识论等级：L1（合成数值 witness，验证管线，不验证真实行情有效性）

  ★定义依据（§6.3/§6.4 公式，口径 B 全三段，637号）：
    - ZG = min(g₁,g₂,g₃)：全三段高点最小者（中枢核心上沿）
    - ZD = max(d₁,d₂,d₃)：全三段低点最大者（中枢核心下沿）
    - GG = max(gₙ)：三段高点最大值（外缘上界）
    - DD = min(dₙ)：三段低点最小值（外缘下界）
    段高点/低点：segHigh = max(startPrice, endPrice)，segLow = min(startPrice, endPrice)。
    识别判据（几何必要条件）：ZD ≤ ZG（全三段核心区间非空，已含第三段贯穿）。

  ★边界条件（结论翻转）：
    - 识别判据是几何必要条件（全三段 ZD ≤ ZG，口径 B 下已含第三段贯穿），非完整缠论判据
      （仍缺方向交替，见 CenterComplete.lean CenterConfirmedComplete）。若接入方向交替判据
      （still-MISSING-B″），部分当前输出的"中枢"会被过滤（无方向交替的假中枢）——
      centersOfWithOuter 输出在完整判据下可能严格减少。
    - 终止性依赖"每步消费 ≥1 段"（继承自 centersOf well-founded 递归）。若识别逻辑
      引入"消费 0 段"的分支（如延伸态不前进），终止性须重证。
    - ZG/ZD 公式用**全三段**（口径 B，637号 + 一级权威第17课答疑）。误口径 A（前两段）已被裁错，
      canonical 不回退。
    - ZG/ZD/GG/DD 基于整数价格（Tick = Int），无浮点误差；若改 Float，需重证 omega 部分。

  ★下游推论：
    - centersOfWithOuter 产出可直接接入 CenterStates.classifyPosition（位置三态）和
      CenterStates.classifyDevelopment（发展三态）——两函数接受 CenterWithOuter，与本函数输出类型匹配。
    - CenterAutoAssign.lean 或类似中枢自动赋值模块可用本函数替换 ChanlunElements.centersOf 平凡桩——
      只需把 centersOfWithOuter 的输出通过 .core 投影得 List Center（canonical 类型）。
    - 中心定理二见证（witnessCenter_upTrend）证明了 centersOfWithOuter 产出可直接进入趋势/发展判断，
      这是"构造层接入判据层"的端到端闭环（L1 层面）。

  ★谱系引用：
    - CenterStates.lean § 6 still-MISSING-B（centersOf 全自动构造 + GG/DD 字段缺失）的构造层消解：
      本文件提供 centersOfWithOuter 真算（升级 ChanlunElements.centers_total_unique 平凡桩）。
    - CenterConstruction.lean（#116 工位）提供基础：centersOf + centerFromThree + computeZG/ZD/GG/DD。
      本文件是 CenterConstruction 的消费者，不重复其证明。
    - 谱系 256/264/608（中枢三态 legacy Strict.Center → CenterStates → 本文件构造层）：本文件是
      构造层（L1），判据层（L0 完整判据）在 CenterComplete.lean，位置/发展态在 CenterStates.lean。
    - 无概念分离谱系涉及（ZG/ZD/GG/DD 公式在 §6.4 无歧义）。

  ★影响声明：
    - 新建 Origin/CenterConstruct.lean（本文件），独立工位 P1 构造层升级。
    - 禁碰（竞态）约束遵守：不改 ChanlunElements/CenterStates/CenterFull/CenterComplete/
      CenterConstruction/lakefile.toml 及任何 Origin 根聚合文件。
    - 仅新增以下名称到 Origin 命名空间：centersOfWithOuter / centersOfWithOuter_total_unique /
      centersOfWithOuter_total / centersOfWithOuter_invariant / centersOfWithOuter_length /
      witnessCenter / witnessCenter_zg/zd/gg/dd / witnessCenter_chain / witnessCenter_position_within /
      witnessUpNext / witnessCenter_upTrend / centersOfWithOuter_nontrivial。
    - 命名冲突检查：witnessCenter 与 CenterStates.lean § 5 的 sampleCenter/sampleUpNext 不同名
      （本文件前缀 witnessCenter/witnessUpNext）；无 centersOfWithOuter 已存在定义。
    - 注册由 Lead 统一处理（不修改 lakefile.toml）。
-/

end NewChanlun.Origin
