/-
Origin/CenterConstruction.lean — centersOf 全自动构造（算 ZG/ZD/GG/DD）+ 终止性 + 唯一性（task #116）

★工位定位（#113 still-MISSING-B 缺口）：CenterStates.lean 形式化了中枢分类血肉（位置三态/
  发展三态/中心定理一二），CenterFull.lean 提供承载外缘的扩展类型；但 `centersOf : List Segment
  → List CenterFull` 的**全自动构造**——从线段序列识别中枢、计算 ZG=min(g₁,g₂,g₃)/ZD=max(d₁,d₂,d₃)
  （口径 B 全三段，637号）/GG=max(gₙ)/DD=min(dₙ)——的**终止性**与**输出唯一性**未证。本文件实装该
  构造为 Lean 全函数（well-founded 递归），机器检查终止性，并由"纯函数 ⟹ 输出唯一"导出确定性。

═══════════════════════════════════════════════════════════════════════════
构造算法（§6.4 GG/DD 计算 + 中枢识别终止性核心）
═══════════════════════════════════════════════════════════════════════════
中枢由 ≥3 个连续重叠次级别走势段构成。识别递归：
  - 取前 3 段，计算核心 ZG=min(g₁,g₂,g₃)/ZD=max(d₁,d₂,d₃)（口径 B 全三段定核心，637号）+
    外缘 GG/DD（三段聚合，与核心同三段）。
  - 若三段确有公共重叠区间（ZD ≤ ZG，中枢成立）⟹ 产出一个 CenterFull，消费 3 段递归。
  - 否则（无中枢）消费 1 段前进（滑窗）。
**终止性**：每步消费 ≥1 段 ⟹ 剩余段数严格递减 ⟹ well-founded（`List.length`）。

GG/DD/ZG/ZD 用 if 显式构造（不用 min/max——Mathlib 缺失时无 simp 引理，与 SegmentFeatureSeq 同范式）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / well-founded 递归终止 / 纯函数唯一性 / 整数 if 构造，不依赖数据）。
`lake env lean Origin/CenterConstruction.lean` 通过 = centersOf 作为全函数良定义（终止）+
输出唯一（确定性）+ ZG/ZD/GG/DD 计算良构（外缘包含核心不变量）在定义层成立，**不是**
任何"识别的中枢真对应缠论权威标注"的实证断言（L2+）。

诚实标注（gatekeeper，no-patch-mentality）：
★ TerminationAndDeterminismOnly + OuterComputationConstructive ——
  本文件证 centersOf 的**终止性 + 唯一性**（构造层骨架）+ ZG/ZD/GG/DD 的**构造性计算**
  （口径 B 全三段定核心 + 三段聚合外缘，§6.3/§6.4 公式直译，637号）。"三段是否构成中枢"的判据用
  "全三段核心区间非空（ZD ≤ ZG）"封装——这是中枢重叠的**必要**几何条件（口径 B 下已含第三段
  贯穿），良构且终止；但**不**等于完整缠论中枢识别（须次级别走势段方向交替，still-MISSING-B′）。
  把骨架冒充为完整识别 = 声明膨胀（禁止）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.CenterFull

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 从线段读价位区间 [low, high]
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 段高点（两端价的较大者，if 显式不用 max）。 -/
def segHigh (s : Segment) : Tick :=
  if s.startPrice ≥ s.endPrice then s.startPrice else s.endPrice

/-- 段低点（两端价的较小者，if 显式不用 min）。 -/
def segLow (s : Segment) : Tick :=
  if s.startPrice ≤ s.endPrice then s.startPrice else s.endPrice

/-- 段低 ≤ 段高（L0，由 if 三歧 omega）。 -/
theorem segLow_le_segHigh (s : Segment) : segLow s ≤ segHigh s := by
  unfold segLow segHigh
  simp only [Tick]
  split <;> split <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 二元/三元 min/max（if 显式，整数，避 Mathlib min/max）
    ═══════════════════════════════════════════════════════════════════════ -/

def tmin (a b : Tick) : Tick := if a ≤ b then a else b
def tmax (a b : Tick) : Tick := if a ≥ b then a else b

theorem tmin_le_left (a b : Tick) : tmin a b ≤ a := by
  unfold tmin; simp only [Tick]; split <;> omega
theorem tmin_le_right (a b : Tick) : tmin a b ≤ b := by
  unfold tmin; simp only [Tick]; split <;> omega
theorem left_le_tmax (a b : Tick) : a ≤ tmax a b := by
  unfold tmax; simp only [Tick]; split <;> omega
theorem right_le_tmax (a b : Tick) : b ≤ tmax a b := by
  unfold tmax; simp only [Tick]; split <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 从三段计算中枢核心 + 外缘（§6.3/§6.4 公式直译，口径 B 全三段，637号）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **核心上沿 ZG = min(g₁, g₂, g₃)（口径 B，637号）** —— **全三段**高点的最小者。

  ★口径 A→B 迁移（637号谱系，一级权威第17课答疑严格公式 `min(a1,b1,c1)`）：核心区间 = 前三个
  连续次级别走势的**重叠部分** = `(max(三段低), min(三段高))`。误口径 A（前两段 `min(g₁,g₂)`）
  仅在第三段贯穿核心时与 B 重合，第三段更窄时 A 高估核心上沿。canonical = B 全三段。
-/
def computeZG (s1 s2 s3 : Segment) : Tick := tmin (tmin (segHigh s1) (segHigh s2)) (segHigh s3)
/--
  **核心下沿 ZD = max(d₁, d₂, d₃)（口径 B，637号）** —— **全三段**低点的最大者。

  ★口径 B（第17课答疑 `max(a2,b2,c2)`）：误口径 A（前两段 `max(d₁,d₂)`）仅在第三段贯穿核心时
  与 B 重合。canonical = B 全三段。
-/
def computeZD (s1 s2 s3 : Segment) : Tick := tmax (tmax (segLow s1) (segLow s2)) (segLow s3)

/--
  **外缘上界 GG = max(g₁,g₂,g₃)（§6.4）** —— 三段高点的最大值。
-/
def computeGG (s1 s2 s3 : Segment) : Tick := tmax (tmax (segHigh s1) (segHigh s2)) (segHigh s3)
/--
  **外缘下界 DD = min(d₁,d₂,d₃)（§6.4）** —— 三段低点的最小值。
-/
def computeDD (s1 s2 s3 : Segment) : Tick := tmin (tmin (segLow s1) (segLow s2)) (segLow s3)

/--
  **中枢成立的几何必要条件** —— 核心非空：ZD ≤ ZG（**全三段**有共同重叠区间，口径 B）。
  这是"三段构成中枢"的几何**必要**条件（封装，非完整缠论判据，still-MISSING-B′）。
  ★口径 B（637号）：全三段核心非空 `max3(lows) ≤ min3(highs)` ⟺ 三段有共同重叠部分
  （已含"第三段贯穿前两段核心"——见 CenterComplete.lean 完整判据）。
-/
def centerHolds (s1 s2 s3 : Segment) : Bool := decide (computeZD s1 s2 s3 ≤ computeZG s1 s2 s3)

/--
  **★从三段构造 CenterFull（口径 B，仅当全三段核心非空）** —— 计算 ZG/ZD/GG/DD 并组装 CenterFull，
  携带所有不变量证明。前提 `h : ZD ≤ ZG`（全三段核心良构）由调用点 centerHolds 保证。

  外缘包含核心证明（DD ≤ ZD ≤ ZG ≤ GG）：
  - `outer_lo : DD ≤ ZD`：DD = min(d₁,d₂,d₃) ≤ d₁ ≤ max(d₁,d₂,d₃) = ZD（口径 B 同三段聚合，链）。
  - `outer_hi : ZG ≤ GG`：ZG = min(g₁,g₂,g₃) ≤ g₁ ≤ max(g₁,g₂,g₃) = GG（链）。
  ★口径 B 下核心与外缘用**同一三段聚合**：ZD=max3(lows) 同 DD 用同三段（DD≤ZD 因 min3≤max3）；
  ZG=min3(highs) 同 GG 用同三段（ZG≤GG 因 min3≤max3）。
-/
def centerFromThree (s1 s2 s3 : Segment) (h : computeZD s1 s2 s3 ≤ computeZG s1 s2 s3) : CenterFull :=
  { core :=
      { zd := computeZD s1 s2 s3
        zg := computeZG s1 s2 s3
        startIndex := s1.startIndex
        endIndex := s3.endIndex
        valid := h }
    dd := computeDD s1 s2 s3
    gg := computeGG s1 s2 s3
    outer_lo := by
      -- DD = min3(lows) ≤ d1 ≤ max3(lows) = ZD（口径 B：核心下沿与外缘下界同三段聚合，min3≤max3）。
      unfold computeDD computeZD
      have h1 : tmin (tmin (segLow s1) (segLow s2)) (segLow s3) ≤ tmin (segLow s1) (segLow s2) :=
        tmin_le_left _ _
      have h2 : tmin (segLow s1) (segLow s2) ≤ segLow s1 := tmin_le_left _ _
      have h3 : segLow s1 ≤ tmax (segLow s1) (segLow s2) := left_le_tmax _ _
      have h4 : tmax (segLow s1) (segLow s2) ≤ tmax (tmax (segLow s1) (segLow s2)) (segLow s3) :=
        left_le_tmax _ _
      simp only [Tick] at *
      omega
    outer_hi := by
      -- ZG = min3(highs) ≤ g1 ≤ max3(highs) = GG（口径 B：核心上沿与外缘上界同三段聚合，min3≤max3）。
      unfold computeZG computeGG
      have h1 : tmin (tmin (segHigh s1) (segHigh s2)) (segHigh s3) ≤ tmin (segHigh s1) (segHigh s2) :=
        tmin_le_left _ _
      have h2 : tmin (segHigh s1) (segHigh s2) ≤ segHigh s1 := tmin_le_left _ _
      have h3 : segHigh s1 ≤ tmax (segHigh s1) (segHigh s2) := left_le_tmax _ _
      have h4 : tmax (segHigh s1) (segHigh s2) ≤ tmax (tmax (segHigh s1) (segHigh s2)) (segHigh s3) :=
        left_le_tmax _ _
      simp only [Tick] at *
      omega }

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. centersOf 全自动构造（well-founded 递归，终止性机器检查）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★centersOf 全自动构造（task #116，终止性核心）** —— `List Segment → List CenterFull`。

  递归（滑窗）：
  - `s1 :: s2 :: s3 :: rest`：若 `centerHolds s1 s2`（核心非空）⟹ 产出 `centerFromThree`，
    消费 3 段对 `rest` 递归；否则消费 1 段对 `s2 :: s3 :: rest` 递归（滑窗前进）。
  - 段数 < 3：无中枢，返回 []。

  **终止性**：每步消费 ≥1 段（成立消费 3，不成立消费 1）⟹ 剩余严格递减 ⟹ well-founded。
  Lean `termination_by`/`decreasing_by` 机器检查——消解 #113 still-MISSING-B 终止性缺口。

  ★唯一性：纯全函数 ⟹ 输出唯一（`centersOf_total_unique`，§ 5）。
-/
def centersOf (segs : List Segment) : List CenterFull :=
  match segs with
  | [] => []
  | [_] => []
  | [_, _] => []
  | s1 :: s2 :: s3 :: rest =>
      if h : computeZD s1 s2 s3 ≤ computeZG s1 s2 s3 then
        centerFromThree s1 s2 s3 h :: centersOf rest
      else
        centersOf (s2 :: s3 :: rest)
  termination_by segs.length
  decreasing_by
    · -- rest 严格短于 s1::s2::s3::rest
      simp only [List.length_cons]
      omega
    · -- s2::s3::rest 严格短于 s1::s2::s3::rest
      simp only [List.length_cons]
      omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 输出唯一性（确定性）：纯全函数 ⟹ 输出唯一
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★centersOf 输出唯一性（task #116，确定性，L0）** —— 给定线段列，centersOf 输出唯一确定。
  这是 ChanlunElements.centers_total_unique 平凡桩的**非平凡见证**：centersOf 是上面 well-founded
  终止的**具体**全自动构造（带 ZG/ZD/GG/DD 计算），其确定性由"纯函数对每输入恰有一个输出"导出。
-/
theorem centersOf_total_unique :
    TotalUnique (fun segs out => centersOf segs = out) :=
  total_unique_of_fun centersOf

/-- **★全性（终止性可观测推论，L0）** —— centersOf 对任意输入返回（不发散）。 -/
theorem centersOf_total :
    Total (fun segs out => centersOf segs = out) := by
  intro segs; exact ⟨centersOf segs, rfl⟩

/-- **★单值性（确定性，L0）** —— 同一线段列不产生两个不同中枢序列。 -/
theorem centersOf_single_valued :
    SingleValued (fun segs out => centersOf segs = out) :=
  (total_and_single_of_total_unique centersOf_total_unique).2

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 计算见证（反退化：具体线段列 ⟹ 具体中枢，非平凡）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 三段重叠的三个具名段（上(10→20) 下(20→12) 上(12→22)；核心 ZG=20,ZD=12,成立。
    便于 witness 的 match 在 cons 字面上归约）。 -/
def ovSeg1 : Segment := { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 }
def ovSeg2 : Segment := { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 12 }
def ovSeg3 : Segment := { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 12, endPrice := 22 }

/-- overlappingSegs 全三段核心非空（口径 B）：ZD=max(10,12,12)=12 ≤ ZG=min(20,20,22)=20（机器算）。 -/
theorem overlapping_holds : computeZD ovSeg1 ovSeg2 ovSeg3 ≤ computeZG ovSeg1 ovSeg2 ovSeg3 := by
  unfold computeZD computeZG segLow segHigh tmax tmin ovSeg1 ovSeg2 ovSeg3
  decide

/-- ★反退化见证：[ovSeg1,ovSeg2,ovSeg3] 上 centersOf 终止并产出恰一个中枢（核心非空，三段重叠）。 -/
theorem witness_centersOf_one : (centersOf [ovSeg1, ovSeg2, ovSeg3]).length = 1 := by
  rw [centersOf.eq_def]
  dsimp only []
  rw [dif_pos overlapping_holds, centersOf.eq_def]
  rfl

/-- 三段分离的三个具名段（口径 B 核心空：ZD=max(10,20,30)=30 > ZG=min(15,25,35)=15）。 -/
def spSeg1 : Segment := { direction := Direction.up, startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 15 }
def spSeg2 : Segment := { direction := Direction.up, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 25 }
def spSeg3 : Segment := { direction := Direction.up, startIndex := 2, endIndex := 3, startPrice := 30, endPrice := 35 }

/-- separatedSegs 全三段核心空（口径 B）：¬ (ZD=max(10,20,30)=30 ≤ ZG=min(15,25,35)=15)（机器算）。 -/
theorem separated_not_holds : ¬ (computeZD spSeg1 spSeg2 spSeg3 ≤ computeZG spSeg1 spSeg2 spSeg3) := by
  unfold computeZD computeZG segLow segHigh tmax tmin spSeg1 spSeg2 spSeg3
  decide

/-- ★反退化见证：[spSeg1,spSeg2,spSeg3] 核心空，滑窗后剩 2 段无中枢 ⟹ 空中枢序列。 -/
theorem witness_centersOf_separated_empty : centersOf [spSeg1, spSeg2, spSeg3] = [] := by
  rw [centersOf.eq_def]
  dsimp only []
  rw [dif_neg separated_not_holds]
  -- 滑窗到 [spSeg2, spSeg3]（2 段）⟹ centersOf 的 [_,_] 分支返回 []
  rw [centersOf.eq_def]

/-- ★反退化见证：< 3 段 ⟹ 无中枢（边界）。 -/
theorem witness_centersOf_too_few : centersOf [] = [] := by
  rw [centersOf.eq_def]

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 边界条件 + 下游推论 + 影响声明
    ═══════════════════════════════════════════════════════════════════════

  ★已消解（task #116 相对 #113 still-MISSING-B）：
    - **GG/DD 字段**：CenterFull（CenterFull.lean）承载外缘；centersOf 产出 CenterFull，
      `computeGG/computeDD/computeZG/computeZD` 按 §6.3/§6.4 公式（口径 B 全三段核心，637号）
      **构造性计算**核心 + 外缘——消解"Center 无 GG/DD 字段 + 从线段自动计算外缘未证"。
    - **终止性**：centersOf well-founded 递归（每步消费 ≥1 段），`termination_by`/`decreasing_by`
      机器检查。**输出唯一性**：`centersOf_total_unique` + `centersOf_single_valued`。
    - **外缘包含核心不变量**：centerFromThree 的 outer_lo/outer_hi 由 tmin/tmax 链证（DD≤ZD≤ZG≤GG）。

  ★still-MISSING-B′（完整中枢识别判据，诚实开口）：
    `centerHolds`/全三段核心非空（ZD ≤ ZG，口径 B）是中枢重叠的**几何必要条件**，良构且终止；
    口径 B 下已含"第三段贯穿前两段核心"（637号 codex L0 等价：全三段核心非空 ⟺ 前两段核心非空
    ∧ 第三段贯穿）。但**不**等于完整缠论中枢识别——完整版须：(1) 三段为**次级别走势**（非任意
    线段，须方向交替，CenterComplete.lean DirAlternates），(2) 中枢延伸/新生/扩展的发展态串接
    （CenterStates.classifyDevelopment 接入相邻中枢对）。把 (1)-(2) 接入使识别 = 真缠论中枢，是
    still-MISSING-B′。本文件**不声称**识别的中枢真对应权威标注——只声称识别递归终止 + 确定 +
    核心/外缘按口径 B 公式计算。

  ★边界条件（结论翻转）：
    - 终止性依赖"每步消费 ≥1 段"。成立支消费 3，不成立支消费 1——两支都 ≥1，故终止。
      若 still-MISSING-B′ 接入完整判据后某支消费 0 段（如"重叠但不前进"），终止性翻转须重证。
    - 核心 ZG/ZD 用**全三段**（口径 B，§6.3/§6.4 ZG=min(g₁,g₂,g₃)/ZD=max(d₁,d₂,d₃)，637号）。
      若回退误口径 A（前两段 min(g₁,g₂)/max(d₁,d₂)），computeZG/ZD 须改，外缘包含核心证明须重做
      （但 A 已被一级权威第17课答疑裁错——见 637号谱系，canonical 不回退）。
    - centerHolds 用闭区间 `ZD ≤ ZG`（核心可退化为单点 ZD=ZG）。若要求严格 `ZD < ZG`
      （排除单点中枢），临界翻转。

  ★下游推论：
    - BspConstruction.lean（bspOf）第三类买卖点"回抽不破 ZG/ZD"用 centersOf 产出的 CenterFull.core
      位置三态——中枢序列构造终止 ⟹ bspOf 的中枢输入良定义。
    - centersOf 终止 + 唯一 + 产 CenterFull ⟹ 可实例化 ChanlunElements.centersOf（投影 CenterFull.core
      得 canonical Center）——消解审计判决的"centersOf 平凡桩 + Center 无外缘"。

  ★影响声明：
    - 新增 Origin.CenterConstruction 模块，import ChanlunElements + CenterStates + CenterFull（只读）。
    - 不改 canonical 类型，无反向依赖，无命名冲突。待 Lead 登记 root：`Origin.CenterConstruction`。

  ★谱系引用：消解 CenterStates.lean § 6 still-MISSING-B（centersOf 全自动构造 + GG/DD 字段缺失）；
    重锚 legacy Strict/Center.lean 外缘计算到 Origin 构造层（谱系 256/264/608 中枢三态）。
-/

end NewChanlun.Origin
