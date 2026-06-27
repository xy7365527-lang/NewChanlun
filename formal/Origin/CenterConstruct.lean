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
权威来源（§6.4 ZG/ZD/GG/DD 公式）
═══════════════════════════════════════════════════════════════════════════
- §6.4（知识库 + chan99 第八节）：
  · ZG = min(g₁,g₂)（前两段高点的较小者，中枢核心上沿）
  · ZD = max(d₁,d₂)（前两段低点的较大者，中枢核心下沿）
  · GG = max(gₙ)（所有构成段最高高点，外缘上界）
  · DD = min(dₙ)（所有构成段最低低点，外缘下界）
  · 不变量：DD ≤ ZD ≤ ZG ≤ GG（外缘包含核心）。
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
  - `cwo.core`：canonical Center，含 `zd ≤ zg`（核心区间良构，ZD = max(d₁,d₂)，ZG = min(g₁,g₂)）
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
  §6.4 计算：
  - ZG = min(g₁,g₂) = min(20,20) = 20
  - ZD = max(d₁,d₂) = max(10,12) = 12
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

/-- ★ZG 值机器验证（L0，由定义直接读）：core.zg = 20。 -/
theorem witnessCenter_zg : witnessCenter.core.zg = 20 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeZG
  unfold tmin segHigh ovSeg1 ovSeg2; decide

/-- ★ZD 值机器验证（L0，由定义直接读）：core.zd = 12。 -/
theorem witnessCenter_zd : witnessCenter.core.zd = 12 := by
  unfold witnessCenter CenterFull.toOuter centerFromThree computeZD
  unfold tmax segLow ovSeg1 ovSeg2; decide

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
  unfold computeZG computeZD tmin tmax segHigh segLow ovSeg1 ovSeg2
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
    § 6. 边界条件 + 下游推论 + 影响声明 + 谱系引用（结果包六要素）
    ═══════════════════════════════════════════════════════════════════════

  ★结论（task #16）：
    centersOfWithOuter : List Segment → List CenterWithOuter 真算实装。
    - 从线段序列自动识别中枢并计算 ZG=min(g₁,g₂)/ZD=max(d₁,d₂)/GG=max(gₙ)/DD=min(dₙ)
    - 每个输出元素携带机器证明的不变量 DD≤ZD≤ZG≤GG
    - 4 个数值 witness（ZG=20/ZD=12/GG=22/DD=10）机器验证公式正确性
    - 1 个端到端见证（classifyPosition 接入 witnessCenter，返回 within）
    - 1 个中心定理二见证（IsUpTrend witnessCenter witnessUpNext）
    - 认识论等级：L1（合成数值 witness，验证管线，不验证真实行情有效性）

  ★定义依据（§6.4 公式）：
    - ZG = min(g₁,g₂)：前两段高点较小者（中枢核心上沿）
    - ZD = max(d₁,d₂)：前两段低点较大者（中枢核心下沿）
    - GG = max(gₙ)：三段高点最大值（外缘上界）
    - DD = min(dₙ)：三段低点最小值（外缘下界）
    段高点/低点：segHigh = max(startPrice, endPrice)，segLow = min(startPrice, endPrice)。
    识别判据（几何必要条件）：ZD ≤ ZG（核心区间非空）。

  ★边界条件（结论翻转）：
    - 识别判据是几何必要条件（ZD ≤ ZG），非完整缠论判据（方向交替 + 第三段贯穿，见
      CenterComplete.lean CenterConfirmedComplete）。若接入完整判据（still-MISSING-B″），
      部分当前输出的"中枢"会被过滤（无方向交替的假中枢）——centersOfWithOuter 输出
      在完整判据下可能严格减少。
    - 终止性依赖"每步消费 ≥1 段"（继承自 centersOf well-founded 递归）。若识别逻辑
      引入"消费 0 段"的分支（如延伸态不前进），终止性须重证。
    - ZG/ZD 公式用前两段（§6.4 原文），若某口径用全部段动态更新 ZG/ZD，公式须改。
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
