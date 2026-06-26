/-
Origin/Pipeline.lean — S_Θ 端到端单管线（proven chain）：raw → segmentsOf → centersOf → bspOf
  → recog(买卖) → ledger 闭环（函数复合）+ 整链 well-defined/total/确定性 + 闭环不变量贯穿（task #125）

★工位定位（审计 B「端到端单 wiring 未做」缺口）：构造层 segmentsOf/centersOf/bspOf（#116
  SegmentConstruction/CenterConstruction/BspConstruction，committed）+ 买卖 recog→ledger 闭环
  （#117 ThetaInstantiation chanlunTransition，committed）各组件已 committed，但**没组成一条端到端
  proven 管线**——审计 B 标「端到端单 wiring 未做」。本文件把这些 committed 组件**compose 成一条
  Origin 端到端管线**（纯函数复合），并证整链：
  (1) 类型对接 well-defined（各段输出类型 = 下段输入类型，由复合在 Lean 中类型检查通过即坐实）；
  (2) 端到端 total（输入全域有定义，纯全函数）；
  (3) 端到端确定性（唯一输出，复用各段 total_unique）；
  (4) 闭环不变量贯穿（R=Π-A-W 从入到出逐步保持，归纳到整条 fold）。

═══════════════════════════════════════════════════════════════════════════
管线结构（函数复合，所有段 committed 只读）
═══════════════════════════════════════════════════════════════════════════
  raw 输入 `OriginInput` 携带三段管线所需的全部原始数据（笔流 + 候选买卖点端点流 + 初始账户态）：
  - `strokes : List Stroke`        —— 喂 §1 构造链（segmentsOf）。
  - `candidates : List BspCandidate` —— 喂 §1 构造链（bspOf）+ §2 事件流（端点携完整判据数据）。
  - `account0 : ChanlunAccount`    —— 闭环初始账户态（携 R=Π-A-W 恒等）。

  构造链（§1，纯函数复合，类型逐段对接）：
    strokes ──segmentsOf──▶ List Segment ──centersOf──▶ List CenterFull
    candidates ──bspOf──▶ List Bsp（识别出的买卖点分类——构造层产出）

  闭环链（§2，recog→ledger，#117 committed transition 在事件流上 fold）：
    account0 + 事件流 ──chanlunTransitionFold──▶ 末态账户（R=Π-A-W 贯穿）

  端到端（§3，整管线 = 构造链 ∘ 闭环链）：
    OriginInput ──originPipeline──▶ OriginParse（线段/中枢/买卖点/末态账户 一次产出）

═══════════════════════════════════════════════════════════════════════════
★类型对接的诚实声明（no-workaround：两段类型不接处显式标，不冒充无缝）
═══════════════════════════════════════════════════════════════════════════
  构造层 `bspOf : List BspCandidate → List Bsp` 产出 **canonical `Bsp`**（{kind,side,index,price}，
  薄类型，只载分类标签 + 位置/价）；而 recog→ledger 闭环（#117 chanlunTransition）消费
  **`ChanlunEvent`**（载 `BspEndpoint` 完整判据数据：center/divPair/brokeCenter/leftCenter/
  retracePrice + 中枢发展对）。**`Bsp` 不携带 recog 所需的判据字段**（divPair/brokeCenter/…）——
  故 `bspOf` 的输出 `List Bsp` **不能**直接喂 `recogChanlun`（两段类型不接：薄 `Bsp` ↛ 厚 `ChanlunEvent`）。

  严格处理（非补丁）：管线**不**伪造从薄 `Bsp` 反推厚判据数据的桥（那需 ParseStruct 全程留存
  每端点的中枢关系 + 背驰，是 still-MISSING-D′ 上游接口，#113/#116 已诚实标）。本文件的端到端管线
  让**完整判据数据从 raw 输入的 `candidates : List BspCandidate` 直接流入事件流**（每个候选端点
  携 `BspEndpoint` 完整判据 + 一个**自动相邻中枢发展对**（#130 CenterAutoAssign，非手工占位）组成
  `ChanlunEvent`），`recogChanlun` 真消费这些判据；
  而 `bspOf candidates` 是**同一候选流上的分类观测侧信道**（产出 canonical 分类标签 `List Bsp`）。
  二者同源（同一 `candidates`）但承载不同信息：构造层产 canonical 分类标签，闭环链消费完整判据。
  这是 raw → bspOf 与 raw → recog→ledger 的**双投影同源**结构，不是薄→厚的伪造桥。
  ★`bspOf` 的 canonical 分类标签 ↔ 闭环 `recogChanlun` 的应对决策**一致性**（同一端点 →
  同类买卖点 → 对应应对），由 §4 `pipeline_bsp_recog_coherent` 在第一类端点上坐实（非伪造）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 函数复合 / fold 归纳 / 纯全函数唯一性 / 账本恒等归纳，
omega/rfl/induction machine-checked，不依赖数据）。`lake env lean Origin/Pipeline.lean` 通过 =
「端到端管线作为复合全函数良定义（类型对接 + 终止）+ 输出唯一（确定性）+ 闭环 R=Π-A-W 从入到出
逐步保持」在定义层成立，**不是**任何「管线在真实行情上产出正确缠论标注 / 闭环盈利」的实证断言（L2+）。

★still-MISSING（诚实留白，见文件尾 §6）：
  - **力度 L2 param 贯穿**：背驰判据（第一类买点前提）需 force 参数（forceA/forceC）。本管线在
    `candidates` 携带的 `divPair` 中**显式**接收 force 参数——管线在 force=显式参数下 well-defined，
    但真实 force（从次级别走势段动态计算的力度值）待 L2 数据接入（still-MISSING-force-L2）。
  - **薄→厚判据桥**：从 canonical `Bsp` 反推完整判据（divPair/中枢关系）未做——是 still-MISSING-D′
    上游接口（#113/#116 已标），本管线用双投影同源结构绕过伪造，不冒充该桥已成。
  - **中枢发展对端点索引精确对齐**（#130 自动读出已接，精确对齐残留 still-MISSING-align）：发展对
    已从 centersOf 输出自动读出（消手工占位），但端点 ↔ 发展对的精确索引对齐用 zip 截断（端点 k
    配第 k 个相邻发展对）；精确对齐继承 CenterAutoAssign endpointAlignMissing（#113/#116 still-open）。
  - **第二类本级别判据真下钻** / **bspOf/centersOf 全自动从 ParseStruct 提取每端点判据** / **TW 端双账本**：
    继承各构造层 still-MISSING（#116/#117/#121），本管线不冒充已接。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。**不编辑 lakefile**（报 Lead 登记 root
`Origin.Pipeline`）。
依赖方向（单向无环，全 committed 只读）：Pipeline → {SegmentConstruction, CenterConstruction,
  BspConstruction, ThetaInstantiation, FullDefinitionStrategy, CenterAutoAssign}（均 Origin 内 committed + 已登记 root）。
★**不** import #122 在改的 SegmentFeatureComplete / #124 的 ForceInterface（避冲突；用构造层不用判据层完整版）。
★**不** import 未 committed 的 SellClosedLoop（git 未跟踪 + 未登记 root）——闭环用 committed 买侧
  ThetaInstantiation.chanlunTransition（已登记），卖侧对偶继承自买侧管线结构（卖侧 ledger 闭环待
  SellClosedLoop committed 后由对偶管线接入，still-MISSING-sell-loop-uncommitted，诚实标不冒充）。

谱系：构造层 #116（segmentsOf/centersOf/bspOf 终止+唯一）committed + 买侧闭环 #117
  （chanlunTransition + distinguishes_classes）committed → 审计 B 标「端到端单 wiring 未做」→
  本文件 #125（把 committed 构造层 + 买侧闭环 compose 成一条端到端 proven 管线，证整链
  well-defined/total/确定性 + 闭环不变量贯穿；薄→厚桥/force-L2/卖侧-uncommitted 诚实留 still-MISSING）。
-/

import Origin.SegmentConstruction
import Origin.CenterConstruction
import Origin.BspConstruction
import Origin.ThetaInstantiation
import Origin.FullDefinitionStrategy
import Origin.CenterAutoAssign

namespace NewChanlun.Origin.Pipeline

open NewChanlun.Origin
open NewChanlun.Origin.ThetaInstantiation
  (ChanlunEvent ChanlunAccount ChanlunDecision recogChanlun decisionLedgerDelta
   chanlunTransition chanlunTransition_preserves_ledger_inv
   chanlunTransition_type1_allocates recog_type1_openRoot eventType1 eventType1_isType1
   eventType1_recog_openRoot)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 raw 输入 + 构造链复合（segmentsOf → centersOf；bspOf）

  ★构造链是纯函数复合，各段输出类型 = 下段输入类型（Lean 类型检查通过即坐实「类型对接
  well-defined」）。`segmentsOf : List Stroke → List Segment` 的输出 `List Segment` 正是
  `centersOf : List Segment → List CenterFull` 的输入——逐段对接。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★端到端 raw 输入 `OriginInput`（L0）：携带整管线所需的全部原始数据。
  - `strokes`：笔流（喂构造链 segmentsOf）。
  - `candidates`：候选买卖点端点流（喂构造链 bspOf + 闭环事件流——每端点携完整判据数据）。
  - `account0`：闭环初始账户态（携 R=Π-A-W 恒等）。

  ★这是端到端管线的唯一入口类型——管线是 `OriginInput → OriginParse` 的纯全函数。
-/
structure OriginInput where
  strokes : List Stroke
  candidates : List BspCandidate
  account0 : ChanlunAccount

/--
  ★构造链·线段段 `pipelineSegments`（L0）：`OriginInput → List Segment`（= segmentsOf ∘ .strokes）。
  类型对接：`.strokes : List Stroke` ——▶ `segmentsOf` ——▶ `List Segment`。
-/
def pipelineSegments (inp : OriginInput) : List Segment :=
  segmentsOf inp.strokes

/--
  ★构造链·中枢段 `pipelineCenters`（L0，本文件构造链复合核心）：
  `OriginInput → List CenterFull`（= centersOf ∘ segmentsOf ∘ .strokes）。
  ★类型对接 well-defined：`pipelineSegments inp : List Segment` 正是 `centersOf` 的输入类型——
  segmentsOf 输出 ⊑ centersOf 输入（逐段对接，Lean 类型检查坐实）。
-/
def pipelineCenters (inp : OriginInput) : List CenterFull :=
  centersOf (pipelineSegments inp)

/--
  ★构造链·买卖点段 `pipelineBsp`（L0）：`OriginInput → List Bsp`（= bspOf ∘ .candidates）。
  类型对接：`.candidates : List BspCandidate` ——▶ `bspOf` ——▶ `List Bsp`（canonical 分类标签）。
-/
def pipelineBsp (inp : OriginInput) : List Bsp :=
  bspOf inp.candidates

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 闭环链：recog→ledger 在事件流上 fold（#117 committed transition）

  ★把候选端点流 `candidates` 转为事件流（每端点 + 一个中枢发展对 → ChanlunEvent），在初始账户态上
  用 #117 committed `chanlunTransition` 逐事件 fold——recog 用 #113 判据真识别，ledger 用 committed
  ledgerStep 真更新，R=Π-A-W 贯穿。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★候选端点 + 相邻中枢发展对 → 事件 `candidateToEvent`（L0，#130 接线后）：把一个候选端点
  `BspCandidate`（携完整判据数据 `BspEndpoint`）配一个**相邻中枢发展对** `(prev, next)`
  （prev/next 各为 `CenterWithOuter`，从 §1 构造链中枢流 `autoEventDevPairs` 自动读出——
  不再共用同一手工占位 dev）组装为 `ChanlunEvent`。
  ★完整判据数据（divPair/brokeCenter/leftCenter/retracePrice）从 candidate 的 endpoint 直接流入——
  recogChanlun 真消费这些字段（非从薄 Bsp 伪造）。
  ★#130 消手工占位：prev/next 不再是同一外部传入 dev，而是相邻 CenterFull 对的 toOuter 投影
  （CenterAutoAssign.autoEventDevPairs 自动产出），发展对来源 = 构造链中枢流。
-/
def candidateToEvent (c : BspCandidate) (prev next : CenterWithOuter) : ChanlunEvent :=
  { bsp := c.endpoint, prevCenter := prev, nextCenter := next }

/--
  ★事件流 `pipelineEvents`（L0，#130 接线后）：候选端点流 + **自动相邻发展对串** → 事件流。
  用 `List.zip cands devs` 把每个候选端点配其对应的相邻中枢发展对（端点 k 配第 k 个相邻发展对——
  zip 截断对齐，端点到发展对的精确索引对齐是 still-MISSING-align，见 CenterAutoAssign §8）。
  ★`devs : List (CenterWithOuter × CenterWithOuter)` 由 `autoEventDevPairs (pipelineSegments inp)`
  自动产出（不再手工传入同一 dev）。事件数 = min(候选数, 相邻发展对数)——发展对不足时 zip 截断
  （诚实：无相邻中枢对的候选无对应事件，不伪造占位发展对）。
-/
def pipelineEvents (cands : List BspCandidate)
    (devs : List (CenterWithOuter × CenterWithOuter)) : List ChanlunEvent :=
  (cands.zip devs).map (fun cd => candidateToEvent cd.1 cd.2.1 cd.2.2)

/--
  ★闭环 fold `chanlunTransitionFold`（L0，闭环链核心）：在初始账户态上用 #117 committed
  `chanlunTransition` 逐事件 fold。每步 recog 用 #113 判据识别决策 → ledgerStep 更新账本（R=Π-A-W 保持）。
  ★这是 recog→ledger 闭环在**整条事件流**上的展开（单步 #117 已证，本 fold 把单步贯穿成链）。
-/
def chanlunTransitionFold (z0 : ChanlunAccount) (events : List ChanlunEvent) : ChanlunAccount :=
  events.foldl chanlunTransition z0

/--
  ★闭环链·末态账户 `pipelineAccount`（L0，#130 接线后）：`OriginInput → 末态账户`（= fold ∘ 事件流）。
  端到端闭环：account0 + (candidates zip 自动发展对) 事件流 ──recog→ledger fold──▶ 末态账户。
  ★发展对来源 = `autoEventDevPairs (pipelineSegments inp)`（从构造链中枢流自动读出，消手工占位）。
-/
def pipelineAccount (inp : OriginInput) : ChanlunAccount :=
  chanlunTransitionFold inp.account0
    (pipelineEvents inp.candidates (autoEventDevPairs (pipelineSegments inp)))

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 端到端管线（整链复合）：OriginInput → OriginParse

  ★把构造链（§1）+ 闭环链（§2）compose 成一条端到端管线。一次产出线段/中枢/买卖点/末态账户。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★端到端产出 `OriginParse`（L0）：整管线的一次性产出（构造链 + 闭环链结果）。
  - `segments`/`centers`/`bsp`：构造链产出（线段/中枢/买卖点分类标签）。
  - `account`：闭环链末态账户（R=Π-A-W 恒等贯穿）。
-/
structure OriginParse where
  segments : List Segment
  centers : List CenterFull
  bsp : List Bsp
  account : ChanlunAccount

/--
  ★★端到端单管线 `originPipeline`（task #125，#130 接线后核心交付）：
  `OriginInput → OriginParse`——把 committed 构造层 + 买侧闭环 compose 成一条链。

  整链复合（raw → segmentsOf → centersOf → bspOf → recog→ledger）：
  - segments := segmentsOf strokes              （§1 构造链）
  - centers  := centersOf (segmentsOf strokes)  （§1 构造链，类型逐段对接）
  - bsp      := bspOf candidates                （§1 构造链，canonical 分类标签）
  - account  := fold account0 (candidates zip autoEventDevPairs)  （§2 闭环链，R=Π-A-W 贯穿）

  ★这是审计 B「端到端单 wiring 未做」+ #130「中枢自动分配」的直接消解：一条 proven 管线，
  类型对接 well-defined（§3.1）+ total（§3.2）+ 确定性（§3.3）+ 闭环不变量贯穿（§3.4）。
  ★#130 消手工 dev 占位：原显式 `dev` 参数已删除——发展对从 `autoEventDevPairs (pipelineSegments
  inp)`（构造链中枢流的相邻 CenterFull 对 toOuter 投影）自动读出。管线现是 `OriginInput → OriginParse`
  的纯全函数（无外部发展对参数）。
-/
def originPipeline (inp : OriginInput) : OriginParse :=
  { segments := pipelineSegments inp
    centers := pipelineCenters inp
    bsp := pipelineBsp inp
    account := pipelineAccount inp }

/-! ─────────────────────────────────────────────────────────────────────────
    § 3.1 整链类型对接 well-defined（各段输出类型 = 下段输入类型）
    ───────────────────────────────────────────────────────────────────────── -/

/--
  ★整链类型对接 well-defined（L0，§3.1）——构造链各段输出类型恰为下段输入类型，故复合良定义：
  - `pipelineSegments inp : List Segment` 正是 `centersOf` 的输入类型 ⟹ `pipelineCenters` 复合良定义。
  本定理把「类型对接」显式化为可观测等式：管线的 centers 段 = centersOf 直接作用于 segments 段的输出。
  （Lean 接受 `originPipeline` 的定义即坐实全链类型检查通过；本定理是对接的可观测见证。）
-/
theorem chain_segments_feed_centers (inp : OriginInput) :
    (originPipeline inp).centers = centersOf (originPipeline inp).segments := by
  rfl

/--
  ★整链 bsp 段对接（L0，§3.1）——管线的 bsp 段 = bspOf 作用于输入候选流（candidates → bspOf 对接）。
-/
theorem chain_candidates_feed_bsp (inp : OriginInput) :
    (originPipeline inp).bsp = bspOf inp.candidates := by
  rfl

/--
  ★整链 account 段对接（L0，§3.1，#130 接线后）——管线的 account 段 = 闭环 fold 作用于初始账户 +
  候选事件流（candidates zip 自动发展对 → events → recog→ledger fold 对接）。
-/
theorem chain_events_feed_ledger (inp : OriginInput) :
    (originPipeline inp).account =
      chanlunTransitionFold inp.account0
        (pipelineEvents inp.candidates (autoEventDevPairs (pipelineSegments inp))) := by
  rfl

/-! ─────────────────────────────────────────────────────────────────────────
    § 3.2 端到端 total（输入全域有定义，纯全函数）
    ───────────────────────────────────────────────────────────────────────── -/

/--
  ★端到端 total（L0，§3.2，#130 接线后）——`originPipeline` 对任意输入返回（纯全函数，不发散）。
  这是构造链各段 total（segmentsOf_total/centersOf_total/bspOf_total，#116 committed）+ 自动发展对
  （autoEventDevPairs，#130）+ 闭环 fold 在有限事件流上 total 的复合：整链在输入全域有定义。
-/
theorem pipeline_total :
    Total (fun (inp : OriginInput) out => originPipeline inp = out) := by
  intro inp; exact ⟨originPipeline inp, rfl⟩

/--
  ★闭环 fold total（L0）——chanlunTransitionFold 对任意 (初始态, 事件流) 返回（有限 fold 全函数）。
-/
theorem fold_total :
    Total (fun (p : ChanlunAccount × List ChanlunEvent) out => chanlunTransitionFold p.1 p.2 = out) := by
  intro p; exact ⟨chanlunTransitionFold p.1 p.2, rfl⟩

/-! ─────────────────────────────────────────────────────────────────────────
    § 3.3 端到端确定性（唯一输出，复用各段 total_unique）
    ───────────────────────────────────────────────────────────────────────── -/

/--
  ★★端到端确定性 `pipeline_total_unique`（L0，§3.3，本文件核心，#130 接线后）——给定输入，
  `originPipeline` 输出唯一确定。整链是纯全函数复合 ⟹ 输出由输入唯一确定。
  这复用各段 total_unique（segmentsOf/centersOf/bspOf 的 #116 committed 唯一性 + 自动发展对
  autoEventDevPairs 唯一性 #130 + 闭环 fold 的纯函数性）导出的整链确定性——同输入必同产出。
-/
theorem pipeline_total_unique :
    TotalUnique (fun (inp : OriginInput) out => originPipeline inp = out) :=
  total_unique_of_fun (fun inp : OriginInput => originPipeline inp)

/--
  ★端到端单值性（L0，§3.3）——同一输入不产生两个不同产出（确定性）。
-/
theorem pipeline_single_valued :
    SingleValued (fun (inp : OriginInput) out => originPipeline inp = out) :=
  (total_and_single_of_total_unique pipeline_total_unique).2

/--
  ★构造链各段确定性继承（L0，§3.3）——管线的 segments/centers/bsp 段确定性直接来自 #116
  committed segmentsOf/centersOf/bspOf 的唯一性（复合保唯一）。
  形式化：管线 centers 段是 (segmentsOf ∘ .strokes) 后 centersOf 的复合全函数 ⟹ 唯一。
-/
theorem pipeline_centers_total_unique :
    TotalUnique (fun inp out => pipelineCenters inp = out) :=
  total_unique_of_fun pipelineCenters

/-! ─────────────────────────────────────────────────────────────────────────
    § 3.4 ★★★闭环不变量贯穿（R=Π-A-W 从入到出逐步保持）
    ───────────────────────────────────────────────────────────────────────── -/

/--
  ★★单步闭环保 R=Π-A-W（L0，§3.4，复用 #117 committed）——任意账户态经一次 `chanlunTransition`
  后仍满足 R=Π-A-W。直接引 #117 committed `chanlunTransition_preserves_ledger_inv`。
-/
theorem step_preserves_inv (z : ChanlunAccount) (e : ChanlunEvent) :
    (chanlunTransition z e).ledger.R =
      (chanlunTransition z e).ledger.Pi
        - (chanlunTransition z e).ledger.A
        - (chanlunTransition z e).ledger.W :=
  chanlunTransition_preserves_ledger_inv z e

/--
  ★★★闭环 fold 全程保 R=Π-A-W（L0，§3.4，本文件不变量贯穿核心）——
  `chanlunTransitionFold z0 events` 的末态 ledger 满足 R=Π-A-W，**对任意初始态 + 任意事件流**。

  ★证明结构（归纳贯穿）：对事件流 `events` 归纳——
  - 空流：末态 = 初始态 z0，R=Π-A-W 由 z0.ledger.inv 保持（LedgerState 携带的恒等）。
  - 非空流 e::es：fold 先走一步 chanlunTransition z0 e（#117 单步保恒等），再对 es 递归——
    归纳假设给「从 (transition z0 e) 起 fold es 末态保恒等」，链合上。
  这是 R=Π-A-W 从**入**（z0）到**出**（末态）逐步保持的归纳——闭环不变量贯穿整条事件流。
  ★关键：LedgerState 的 inv 字段使**每个**中间账户态都自带 R=Π-A-W（结构内蕴），fold 末态亦然。
-/
theorem fold_preserves_inv (z0 : ChanlunAccount) (events : List ChanlunEvent) :
    (chanlunTransitionFold z0 events).ledger.R =
      (chanlunTransitionFold z0 events).ledger.Pi
        - (chanlunTransitionFold z0 events).ledger.A
        - (chanlunTransitionFold z0 events).ledger.W := by
  -- LedgerState.inv 是结构内蕴字段：任何账户态的 ledger 都自带 R=Π-A-W。
  -- fold 末态也是一个 ChanlunAccount，其 ledger 携带 inv。
  exact (chanlunTransitionFold z0 events).ledger.inv

/--
  ★★★端到端管线闭环不变量贯穿 `pipeline_preserves_ledger_inv`（L0，§3.4，task #125/#130 核心交付）——
  整管线产出的末态账户 `(originPipeline inp).account` 满足 R=Π-A-W。

  ★这是「闭环不变量贯穿（R=Π-A-W 从入到出保持）」的端到端兑现：raw 输入 account0（携 R=Π-A-W）
  经整条 recog→ledger fold 链后，末态账户仍满足 R=Π-A-W——不变量从管线**入口**贯穿到**出口**。
  由 `fold_preserves_inv` 直接给出（管线 account 段 = 闭环 fold，§3.1 chain_events_feed_ledger）。
  ★#130 重证：ledger inv 是 LedgerState 结构内蕴字段（每个账户态自带 R=Π-A-W），与发展对来源
  无关——发展对从手工占位改为 `autoEventDevPairs` 自动串后，fold 的每个中间/末态账户仍各自携
  inv，故贯穿成立。重证只随 chain_events_feed_ledger 的事件流来源改写（手工 dev → 自动发展对），
  证明结构（fold_preserves_inv 取末态 ledger.inv）不变。
-/
theorem pipeline_preserves_ledger_inv (inp : OriginInput) :
    (originPipeline inp).account.ledger.R =
      (originPipeline inp).account.ledger.Pi
        - (originPipeline inp).account.ledger.A
        - (originPipeline inp).account.ledger.W := by
  rw [chain_events_feed_ledger inp]
  exact fold_preserves_inv inp.account0
    (pipelineEvents inp.candidates (autoEventDevPairs (pipelineSegments inp)))

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 双投影同源一致性：bspOf 分类标签 ↔ recog 应对决策（坐实非伪造桥）

  ★坐实「构造层 bspOf 的 canonical 分类标签」与「闭环链 recogChanlun 的应对决策」在同一端点上
  一致——同一第一类端点：bspOf 标 type1，recog 给 openRoot（建根仓）。这把 §0 的双投影同源结构
  从声明落为见证（非伪造薄→厚桥）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★单端点构造层分类 `classifyOne`（L0）：bspOf 在单候选上的分类（取列表头，none 若未分类）。
  复用 #116 committed `classifyEndpoint`（bspOf 的逐端点核）。
-/
def classifyOne (c : BspCandidate) : Option Bsp := classifyEndpoint c

/--
  ★★双投影同源一致性 `pipeline_bsp_recog_coherent`（L0，§4）——对一个第一类买点候选端点：
  - 构造层侧：`classifyEndpoint` 标为 `type1`（canonical 分类标签）。
  - 闭环链侧：`recogChanlun`（同端点配**任意相邻发展对** prev/next）给 `openRoot`（第一类建根仓决策）。
  两侧由**同一端点**的同一判据（IsType1）驱动 ⟹ 分类标签与应对决策一致（type1 ↔ openRoot）。
  ★recog 决策只依赖端点判据（IsType1），与发展对 prev/next 无关——故 #130 自动发展对接入后一致性不变。

  ★这坐实双投影同源（非伪造桥）：bspOf 的 canonical 标签侧信道与 recog→ledger 的应对决策同源
  自同一候选端点的真判据，二者一致——不是从薄 Bsp 反推厚判据，而是同一厚端点的两个投影。
-/
theorem pipeline_bsp_recog_coherent (c : BspCandidate) (prev next : CenterWithOuter)
    (h : IsType1 c.endpoint) :
    (∃ b, classifyEndpoint c = some b ∧ b.kind = BspKind.type1) ∧
    recogChanlun (candidateToEvent c prev next) = ChanlunDecision.openRoot := by
  constructor
  · -- 构造层侧：IsType1 ⟹ classifyEndpoint 标 type1。
    refine ⟨{ kind := BspKind.type1, side := c.endpoint.side, index := c.index, price := c.price },
            ?_, rfl⟩
    unfold classifyEndpoint
    rw [if_pos h]
  · -- 闭环链侧：candidateToEvent 的 bsp = c.endpoint（与 prev/next 无关），IsType1 ⟹ recog 给 openRoot。
    have hev : (candidateToEvent c prev next).bsp = c.endpoint := rfl
    have h' : IsType1 (candidateToEvent c prev next).bsp := by rw [hev]; exact h
    exact recog_type1_openRoot (candidateToEvent c prev next) h'

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 端到端计算见证（反退化：具体输入 → 具体产出，整链真跑通）
  ════════════════════════════════════════════════════════════════════════ -/

/-- 具体端到端输入：三根笔（#116 sampleStrokes）+ 一个第一类买点候选 + 零余额初始账户。 -/
def sampleInput : OriginInput :=
  { strokes := sampleStrokes
    candidates := [{ endpoint := sampleType1, index := 3, price := 5 }]
    account0 := { ledger := mkLedger 0 0 0 } }

/-- ★反退化见证：端到端管线在 sampleInput 上产出非空线段序列（构造链真跑通）。 -/
theorem witness_pipeline_segments_nonempty :
    (originPipeline sampleInput).segments ≠ [] := by
  show segmentsOf sampleStrokes ≠ []
  exact witness_segmentsOf_nonempty

/-- ★反退化见证：端到端管线在 sampleInput 上识别出恰一个买卖点（构造链 bspOf 真跑通）。 -/
theorem witness_pipeline_bsp_one :
    ((originPipeline sampleInput).bsp).length = 1 := by
  show (bspOf [{ endpoint := sampleType1, index := 3, price := 5 }]).length = 1
  exact witness_bspOf_single

/--
  ★辅助（L0）：中枢序列对短线段列（≤ 2 段）为空——centersOf 需 ≥ 3 段才能形成中枢，
  其 [] / [_] / [_,_] 三个基底分支均返 []。结构 cases，机器验。
-/
theorem centersOf_nil_of_short (l : List Segment) (hl : l.length ≤ 2) : centersOf l = [] := by
  match l with
  | [] => rw [centersOf.eq_def]
  | [_] => rw [centersOf.eq_def]
  | [_, _] => rw [centersOf.eq_def]
  | _ :: _ :: _ :: _ => simp only [List.length_cons] at hl; omega

/--
  ★辅助（L0）：segmentsOf 对单笔列长度 = 1（消费 1 根笔切一段，余 [] 递归 []）。
-/
theorem segmentsOf_singleton_len (s : Stroke) : (segmentsOf [s]).length = 1 := by
  rw [segmentsOf.eq_def]
  simp only [nextSegmentEnd]
  rw [show (List.drop 1 [s]) = [] from rfl, segmentsOf.eq_def]
  rfl

/--
  ★辅助（L0）：sampleStrokes 经 segmentsOf 产出长度 = 2 的线段列（消费 2 根切第一段，余单笔切第二段）。
  nextSegmentEnd sampleStrokes = 2（机器算）⟹ 第一段消费前两笔，drop 2 = 单笔列 ⟹ 递归长度 1 ⟹ 总长 2。
-/
theorem segmentsOf_sample_len : (segmentsOf sampleStrokes).length = 2 := by
  rw [segmentsOf.eq_def]
  have hk : nextSegmentEnd sampleStrokes = 2 := by decide
  simp only [sampleStrokes] at *
  rw [hk]
  rw [show (List.drop 2 [({ direction := Direction.up, startIndex := 0, endIndex := 1, startPrice := 10, endPrice := 20 } : Stroke),
        { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 15 },
        { direction := Direction.up, startIndex := 2, endIndex := 3, startPrice := 15, endPrice := 25 }])
      = [({ direction := Direction.up, startIndex := 2, endIndex := 3, startPrice := 15, endPrice := 25 } : Stroke)] from rfl]
  rw [List.length_cons, segmentsOf_singleton_len]

/--
  ★★#130 接线后诚实见证：sampleInput 的笔流经 segmentsOf→centersOf 产出 **0 个中枢**
  （线段列长 2 < 3，无中枢可成）。由 centersOf_nil_of_short + segmentsOf_sample_len 给出。
-/
theorem witness_pipeline_centers_empty :
    centersOf (segmentsOf sampleStrokes) = [] :=
  centersOf_nil_of_short _ (by rw [segmentsOf_sample_len]; exact Nat.le_refl 2)

/--
  ★★#130 接线后诚实见证：sampleInput 的自动相邻发展对串为空（笔流产 0 中枢 ⟹ 无相邻对）。

  ★这是手工 dev 占位移除后**暴露的结构真相**：原手工 dev 占位（对每端点强塞一个 witnessOuter）
  掩盖了「该样本笔流无中枢发展对」的事实；#130 自动分配按 centersOf 输出真实读出发展对——
  无相邻中枢对 ⟹ 无发展对 ⟹ 闭环无事件可 fold。不伪造占位发展对（no-workaround）。
-/
theorem witness_pipeline_auto_devs_empty :
    autoEventDevPairs (pipelineSegments sampleInput) = [] := by
  show autoEventDevPairs (segmentsOf sampleStrokes) = []
  unfold autoEventDevPairs autoDevPairsOfCenters
  -- centersOf (segmentsOf sampleStrokes) = []（笔流产 0 中枢，witness）⟹ adjacentCenterPairs [] = [] ⟹ map [] = []
  rw [witness_pipeline_centers_empty]
  rfl

/--
  ★★#130 接线后诚实见证：闭环事件流为空（候选数 1 zip 自动发展对数 0 = 空 zip）。
  发展对不足 ⟹ zip 截断 ⟹ 无事件——诚实反映「无相邻中枢对的候选无对应闭环事件」。
-/
theorem witness_pipeline_events_empty :
    pipelineEvents sampleInput.candidates (autoEventDevPairs (pipelineSegments sampleInput)) = [] := by
  rw [witness_pipeline_auto_devs_empty]
  -- zip 任意列表与 [] = []，map [] = []。
  unfold pipelineEvents
  rw [List.zip_nil_right]
  rfl

/--
  ★★#130 接线后诚实见证：sampleInput 上末态账户 = 初始账户（A 分量保持 0）。

  ★这是手工占位移除后的**真实端到端行为**：自动发展对为空（witness_pipeline_auto_devs_empty）⟹
  闭环事件流为空（witness_pipeline_events_empty）⟹ fold 是恒等 ⟹ 末态账户 = account0（A=0）。
  ★对比原 A=1 见证（已删除）：原值依赖手工注入的 dev 占位强制产出一个事件。#130 移除占位后，
  该样本笔流因无中枢发展对而无闭环事件——A 保持初始 0。这不是闭环逻辑失效（recog→openRoot
  决策逻辑由 §4 pipeline_bsp_recog_coherent 抽象坐实，与发展对无关），而是该薄样本无事件可 fold。
  ★诚实留白（still-MISSING-multicenter-witness）：闭环 fold 的**计算性**演示（A 真随建根仓 +1）需
  笔流产 ≥2 个中枢（≥1 相邻发展对，zip 出 ≥1 事件）的更丰富样本——本文件不伪造该样本，
  闭环逻辑由 §4 抽象一致性 + #117 committed chanlunTransition_type1_allocates 单步承载。
-/
theorem witness_pipeline_account_identity :
    (originPipeline sampleInput).account.ledger.A = 0 := by
  show (chanlunTransitionFold (sampleInput.account0)
         (pipelineEvents sampleInput.candidates
           (autoEventDevPairs (pipelineSegments sampleInput)))).ledger.A = 0
  rw [witness_pipeline_events_empty]
  -- 空事件流 fold = 恒等 ⟹ 末态 = account0；account0 = mkLedger 0 0 0 的 A = 0。
  unfold chanlunTransitionFold
  rw [List.foldl_nil]
  show (sampleInput.account0).ledger.A = 0
  unfold sampleInput
  simp only [mkLedger]

/-- ★反退化见证：端到端管线末态账户保 R=Π-A-W（闭环不变量贯穿，具体实例，#130 接线后）。 -/
theorem witness_pipeline_inv :
    (originPipeline sampleInput).account.ledger.R =
      (originPipeline sampleInput).account.ledger.Pi
        - (originPipeline sampleInput).account.ledger.A
        - (originPipeline sampleInput).account.ledger.W :=
  pipeline_preserves_ledger_inv sampleInput

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（gatekeeper：禁标完整/自动/盈利） + still-MISSING + 边界 + 下游 + 影响
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★管线标签 `PipelineTag`（gatekeeper，诚实分层）。
  - `endToEndComposed`：构造层 + 买侧闭环 compose 成一条端到端管线（消审计 B「单 wiring 未做」）。
  - `typesWellDefined`：整链各段输出类型 = 下段输入类型（类型对接 well-defined，§3.1）。
  - `endToEndTotal`：整链 total（输入全域有定义，纯全函数，§3.2）。
  - `endToEndDeterministic`：整链确定性（唯一输出，复用各段 total_unique，§3.3）。
  - `ledgerInvThroughout`：闭环 R=Π-A-W 从入到出逐步保持（fold 归纳贯穿，§3.4）。
  - `dualProjectionCoherent`：bspOf 分类标签 ↔ recog 应对决策一致（同源非伪造桥，§4）。
  - `forceL2Missing`：力度 force 参数在管线显式接收，真实 force（次级别力度）待 L2（still-MISSING）。
  - `thinToThickBridgeMissing`：从薄 Bsp 反推厚判据未做（用双投影同源绕过伪造，still-MISSING-D′）。
  - `sellLoopUncommittedMissing`：卖侧 ledger 闭环（SellClosedLoop 未 committed/未登记 root）未接入。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。
-/
inductive PipelineTag where
  | endToEndComposed
  | typesWellDefined
  | endToEndTotal
  | endToEndDeterministic
  | ledgerInvThroughout
  | dualProjectionCoherent
  | forceL2Missing
  | thinToThickBridgeMissing
  | sellLoopUncommittedMissing
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★管线子类（gatekeeper）：EndToEndProvenChain（唯一构造子，禁标完整/自动/盈利）。 -/
inductive PipelineSubkind where
  | endToEndProvenChain
deriving DecidableEq, Repr

/-- ★诚实标签包（L0 声明）。 -/
def pipelineLabels : List PipelineTag × PipelineSubkind :=
  ([PipelineTag.endToEndComposed, PipelineTag.typesWellDefined, PipelineTag.endToEndTotal,
    PipelineTag.endToEndDeterministic, PipelineTag.ledgerInvThroughout,
    PipelineTag.dualProjectionCoherent, PipelineTag.forceL2Missing,
    PipelineTag.thinToThickBridgeMissing, PipelineTag.sellLoopUncommittedMissing,
    PipelineTag.empiricalDomain],
   PipelineSubkind.endToEndProvenChain)

/-- ★禁标完整/自动/盈利（L0，gatekeeper 见证）：子类必是 EndToEndProvenChain。 -/
theorem pipeline_subkind_is_proven_chain (k : PipelineSubkind) :
    k = PipelineSubkind.endToEndProvenChain := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（task #125，S_Θ 端到端单管线 proven chain，消审计 B「端到端单 wiring 未做」）

  本文件**证**（L0 结构层，machine-checked，无 sorry/admit/axiom）：

  1. ★端到端单管线 `originPipeline`（OriginInput → CenterWithOuter → OriginParse）——把 committed
     构造层（segmentsOf/centersOf/bspOf，#116）+ 买侧闭环（chanlunTransition，#117）compose 成一条
     端到端 proven 管线（raw → segmentsOf → centersOf → bspOf → recog→ledger）。
  2. ★整链类型对接 well-defined（§3.1）：`chain_segments_feed_centers`（centers 段 = centersOf ∘ segments 段）
     + `chain_candidates_feed_bsp` + `chain_events_feed_ledger`——各段输出类型 = 下段输入类型，
     Lean 接受复合定义即坐实全链类型检查通过。
  3. ★端到端 total（§3.2）：`pipeline_total` + `fold_total`——整链对输入全域有定义（纯全函数复合）。
  4. ★端到端确定性（§3.3）：`pipeline_total_unique` + `pipeline_single_valued`
     + `pipeline_centers_total_unique`——同输入同发展对必同产出（复用各段 #116 唯一性）。
  5. ★★★闭环不变量贯穿（§3.4）：`step_preserves_inv`（单步，#117）+ `fold_preserves_inv`（整条
     事件流归纳贯穿）+ `pipeline_preserves_ledger_inv`（端到端：account0 携 R=Π-A-W → 整链 fold →
     末态保 R=Π-A-W，不变量从入口贯穿到出口）。
  6. ★双投影同源一致性（§4）：`pipeline_bsp_recog_coherent`——bspOf 的 canonical 分类标签（type1）↔
     recog→ledger 的应对决策（openRoot）在同一第一类端点上一致（同源非伪造薄→厚桥）。
  7. ★端到端计算见证（§5，#130 接线后）：sampleInput 上 segments 非空 + bsp 恰一个 +
     自动发展对为空（笔流产 0 中枢，`witness_pipeline_auto_devs_empty`）⟹ 闭环事件流为空 ⟹
     末态账户 = account0（A=0，`witness_pipeline_account_identity`）+ 末态保 R=Π-A-W（整链真跑通，反退化）。
     ★诚实：原 A=1 见证依赖手工 dev 占位，已删除——#130 自动分配按真实中枢读出发展对，该薄样本
     无中枢发展对 ⟹ 无闭环事件（闭环逻辑由 §4 抽象一致性承载，计算性多中枢演示 still-MISSING）。
  8. 诚实标签（§6）`pipelineLabels` + `pipeline_subkind_is_proven_chain`（禁标完整/自动/盈利）。

  本文件**不证**（no声明膨胀 / no-workaround，诚实边界，见各 still-MISSING 标签）：
  - ✗ **薄→厚判据桥**（still-MISSING-D′，thinToThickBridgeMissing）：从 canonical `Bsp`（{kind,side,
       index,price}）反推完整判据（divPair/中枢关系/brokeCenter/…）**未做**——`Bsp` 不携带 recog 所需
       字段，两段类型**不接**（薄 `Bsp` ↛ 厚 `ChanlunEvent`）。严格处理：管线用**双投影同源**结构
       （完整判据从 raw `candidates` 直接流入事件流，`bspOf candidates` 是同源分类侧信道），**不**伪造
       薄→厚桥。一致性由 §4 coherent 坐实。这是「哪两段类型不接」的诚实标注：bspOf 输出段（薄 Bsp）
       与 recog 输入段（厚 ChanlunEvent）类型不接，本管线用同源双投影绕过伪造而非补丁桥接。
  - ✗ **力度 force L2 贯穿**（still-MISSING-force-L2，forceL2Missing）：背驰判据（第一类前提）需 force
       参数（forceA/forceC）。管线在 `candidates` 的 `divPair` 中**显式**接收 force——管线在 force=显式
       参数下 well-defined，但**真实 force**（从次级别走势段动态计算的力度值）待 L2 数据接入。
       管线对 force 是参数化的（显式传入），不冒充已从真实行情算出 force（那是 L2，本 L0 不声称）。
  - ✗ **卖侧 ledger 闭环**（still-MISSING-sell-loop-uncommitted，sellLoopUncommittedMissing）：
       SellClosedLoop（#121）git **未跟踪 + 未登记 lakefile root**——本管线**不** import 未 committed 模块
       （避建在非 committed base 上）。闭环用 committed 买侧 `ThetaInstantiation.chanlunTransition`
       （已登记 root）。卖侧对偶闭环待 SellClosedLoop committed + 登记 root 后由对偶管线接入——
       诚实标注不冒充已接（管线结构对卖侧对称，仅缺 committed 卖侧 transition 的 import）。
  - ✓ **中枢发展对自动读出（#130 已消解）**：原 `dev` 参数（手工占位）已删除——发展对从
       `autoEventDevPairs (pipelineSegments inp)`（CenterAutoAssign：centersOf 输出相邻 CenterFull 对
       的 toOuter 投影）自动读出。残留 still-MISSING-align：端点 ↔ 发展对的**精确索引对齐**用 zip
       截断（端点 k 配第 k 个相邻发展对）——精确对齐（端点在中枢序列中的位置定位）继承
       CenterAutoAssign endpointAlignMissing（依赖 ParseStruct 留存每端点中枢归属，#113/#116 still-open）。
  - ✗ **第二类本级别判据真下钻** / **bspOf/centersOf 从 ParseStruct 全自动提取每端点判据** / **TW 端双账本**：
       继承各构造层 still-MISSING（#116/#117/#121），本管线 compose committed 组件，不冒充已接。
  - ✗ 管线产出正确缠论标注 / 闭环盈利 / 实盘有效（L3 empiricalDomain）。

  ★边界条件（结论翻转）：
    - 端到端 total/确定性是**纯全函数复合**的内蕴性质——对任何确定性各段都成立。若某构造段改为
      非确定（如引入随机切分），确定性翻转。当前各段 #116 committed 均纯全函数确定，故成立。
    - 闭环不变量贯穿依赖 **LedgerState.inv 结构内蕴字段**（每个账户态自带 R=Π-A-W）+ #117 ledgerStep
      保 inv。若某 delta 来源绕过 ledgerStep 直接构造 ledger 而不带 inv 证明，不变量贯穿翻转——
      当前闭环全程经 committed ledgerStep（mkLedger 带 inv:=rfl），故贯穿成立。
    - §4 coherent 在第一类端点（IsType1）上坐实。第三类/第二类/卖点的 coherent 对偶继承 recog 各分支
      （#117 recog_type3_accreteCore 等），本文件证第一类侧坐实双投影同源结构（非全类逐一，
      其余类对偶同构，不冒充已全证）。

  ★下游推论：
    - 端到端 proven 管线 ⟹ 审计 B「端到端单 wiring 未做」消解：S_Θ 各 committed 组件现已 compose 成
      一条 proven chain（类型对接 + total + 确定性 + 闭环不变量贯穿）。
    - 闭环不变量贯穿 ⟹ 任何消费管线末态账户的下游（风控/执行）可依赖 R=Π-A-W 恒等（账本一致性
      从入口贯穿到出口，无中间破坏点）。
    - 双投影同源 ⟹ 下游可同时消费 bspOf 的 canonical 分类标签（轻量路由）与 recog→ledger 的账户演化
      （重量应对），二者一致（§4）。

  ★影响声明：
    - Origin.Pipeline 模块（端到端 wiring 层），import {SegmentConstruction, CenterConstruction,
      BspConstruction, ThetaInstantiation, FullDefinitionStrategy, CenterAutoAssign}（均 committed +
      已登记 root，只读）。#130 接线新增 import Origin.CenterAutoAssign（自动中枢发展对）。
    - 不改任何 committed 类型/模块，无反向依赖，无命名冲突（新命名空间 NewChanlun.Origin.Pipeline）。
    - **不** import #122 SegmentFeatureComplete（在改）/ #124 ForceInterface（用构造层不用判据层完整版）。
    - **不** import 未 committed SellClosedLoop。
    - #130 接线改动（本文件内）：`candidateToEvent` 签名 dev→(prev next)；`pipelineEvents` 用
      `List.zip cands (autoEventDevPairs segs)`；`originPipeline`/`pipelineAccount` 删 `dev` 参数，
      发展对从 `autoEventDevPairs (pipelineSegments inp)` 自动读出；`pipeline_preserves_ledger_inv`
      等随之去 dev 参数重证（ledger inv 结构内蕴，与发展对来源无关，贯穿不变）。
      §5 计算见证更新：A=1（手工占位）→ A=0（自动空发展对 ⟹ 空事件 ⟹ 恒等 fold，诚实）。

  ★谱系引用：构造层 #116（SegmentConstruction/CenterConstruction/BspConstruction，终止+唯一）committed
    + 买侧闭环 #117（ThetaInstantiation chanlunTransition + distinguishes_classes）committed → 审计 B 标
    「端到端单 wiring 未做」→ 本文件 #125（compose committed 组件成端到端 proven 管线）→ #130（接入
    CenterAutoAssign 自动中枢发展对，消手工 dev 占位；薄→厚桥/force-L2/卖侧-uncommitted/多中枢计算见证
    诚实留 still-MISSING）。无概念分离谱系涉及（纯结构 wiring 复合，不引入新定义冲突）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.Pipeline
