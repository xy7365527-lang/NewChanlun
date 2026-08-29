//! 第一类结构门与第一类买卖点判定（#1176 B01 判据块自 `signal.rs` 迁出，零行为）。
//!
//! 承载 [`FirstStructuralGates`] 结构门（[`first_structural_gates`] 唯一判定点）与第一类买卖点
//! 判定（[`judge_first_cached`] / [`judge_first_from_gates`]）。`extract_*` 入口族与
//! [`super::judge_segment`] 留在 [`super`]（signal.rs 门面）；消费面经 `signal` 重导出保持原路径。

use super::*;

/// 第一类结构门的分量分解（E2E-D3 结构谓词，#551 单一来源）。
///
/// 字段与 [`judge_first_cached`] 的三道结构门逐条同源，**不是**第二套判据：
/// - `side`：破最后中枢的方向（买侧向下破 = `Long`，卖侧向上破 = `Short`）。本结构存在本身即
///   蕴含 `direction` 谓词成立——未破中枢或方向锚不合（Q7-#1 裁定C）时函数返回 `None`。
/// - `extreme`：037:20 破 b = I(A) 包络极值（严格超越，等号排除）。
/// - `a_idx`/`c_idx`：I(A)/I(C) 的 closes 下标区间；两者同时 `Some` ⟺ [`Self::comparable`]，
///   即 MACD 面积坐标系上 A/C 可比较。
///
/// `None` 的语义是「**无候选身份**」（非「谓词不成立」）：破中枢不成立 ⟹ 不进候选域；A 段无法
/// 定位或 λ_C 无法定位 ⟹ `CandidateKey` 的 `seg_a`/`c_start` 无来源 ⟹ 身份不可构造。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FirstStructuralGates {
    pub side: Side,
    /// I(A) = 前中枢同向离开走势区间（source_index）。
    pub seg_a: (usize, usize),
    /// λ_C = 离开最后中枢的当前 episode 首同向段起点（source_index）。
    pub lambda_c: usize,
    pub a_idx: Option<(usize, usize)>,
    pub c_idx: Option<(usize, usize)>,
    pub extreme: bool,
}

impl FirstStructuralGates {
    /// I(A)/I(C) 均可映射到 closes 下标 ⟺ MACD 面积可算 ⟺ A/C 可比较。
    pub fn comparable(&self) -> bool {
        self.a_idx.is_some() && self.c_idx.is_some()
    }
}

/// 第一类结构门的唯一判定点（[`judge_first_cached`] 与 #550 候选事件产出同读，禁第二套判据）。
///
/// 与旧内联版逐位等价：门的判据表达式与短路条件全部原样搬入——`judge_first_cached` 仍以「三门
/// 全成立」为产点条件，只是把「不成立即 `return None`」拆成「记录谓词值 + 调用方裁剪」。唯一
/// 非语义差异：`extreme` 不成立时本函数仍计算 `a_idx`/`c_idx`（纯函数、无副作用），使候选事件
/// 侧能诚实读出 `comparable` 分量。
pub(crate) fn first_structural_gates(
    last_center: &Center,
    trend_dir: Direction,
    seg: &Segment,
    anchor_dir: Option<Direction>,
    src_to_idx: &[usize],
    a_seg: Option<((usize, usize), (Tick, Tick))>,
    c_move_start: Option<usize>,
) -> Option<FirstStructuralGates> {
    let end = seg_end(seg);
    // C 破最后中枢几何（L0）+ 方向必须 = 趋势方向（下跌趋势=向下破=底背驰；上涨=向上破=顶背驰）。
    // 因果触发点 = 破中枢段端点（Q5 等价性注记见 `judge_first_cached` 函数头）。
    // Q7-#1 裁定C：破中枢段方向锚用 anchor_dir（provenance 资格）——fallback 单元（None）不触发。
    // L0 恒 anchor_dir==Some(end.dir)（线段内在方向）⟹ 与旧 (end.dir, trend_dir) 匹配逐位一致。
    // ★p117（686 翻转条款第一支窄域授权，终端背书裁定 T2）：生产第一类调用点（judge_segment）
    // 经窄域授权改传结构方向锚（anchors_self，行程方向=τ 等式 veto）；直接调用方传 provenance
    // 锚时裁定C 行为保留（判据函数语义未动，降级在调用点——测试
    // `judge_first_cached_provenance_gate_preserved_for_direct_callers` 锁此契约）。
    let (broke, is_sell) = match (anchor_dir, trend_dir) {
        // 1 买：下跌趋势中向下破最后中枢下沿（底背驰候选）。
        (Some(Direction::Down), Direction::Down) if end.price < last_center.zd => (true, false),
        // 1 卖：上涨趋势中向上破最后中枢上沿（顶背驰候选）。
        (Some(Direction::Up), Direction::Up) if last_center.zg < end.price => (true, true),
        _ => (false, false),
    };
    if !broke {
        return None; // 未破最后中枢 ⟹ 非第一类结构候选（几何分量不足，无候选身份）。
    }
    // A 区间由调用方预算传入（缓存复用，消解热点②）。无 A 候选 ⟹ A/C 无法配对 ⟹ 身份无 seg_a。
    let ((a_start, a_end), (b_lo, b_hi)) = a_seg?;
    // λ_C（Q5）：broke 成立 ⟹ seg 自身满足「同向 ∧ start ≥ c.end_index」过滤 ⟹ 首匹配必存在。
    let Some(lambda_c) = c_move_start else {
        debug_assert!(false, "broke 成立时 λ_C 必 Some（seg 自身在过滤集内）");
        return None;
    };
    // ★037:20 破极值合取（教义必要项，p112 §6-3 裁定项，终端背书裁定 T3 核准）：c 端点必破
    // b 包络极值——下跌趋势 end.price < b_lo（c 创出新低）；上涨趋势 end.price > b_hi（c 创出
    // 新高）。b = I(A)（prev_center 的离开 episode，`locate_departure_move_a` 区间）；包络 =
    // `move_range_envelope`（与 D2 侧 R1 T2 同一原语同区间语义，单一来源）。「创出」= 严格超越
    // （等号排除，与 zd/zg 严格口径一致）。未破 b 极值的破核心段 = 037:20 否则条款域（可按盘整
    // 背驰处理），不进第一类候选域（061:28：未创新极值不构成背驰——含零 bit struct_break 候选
    // 也不产，D1 候选门收缩语义）。
    // 因果触发等价性（同 `judge_first_cached` 函数头 Q5 注记论证）：段是单向对象、终点即极值，
    // I(C) 包络首次破 b 包络 ⟺ 某同向段端点首次越 b_lo/b_hi ⟹ 在该段端点判 `end.price` 破极值
    // = I(C) 包络判破的因果触发点。本合取不改变点的因果语义，只在同一触发点加严。
    let extreme = match trend_dir {
        Direction::Down => end.price < b_lo,
        Direction::Up => end.price > b_hi,
    };
    // I(A)/I(C)（source_index 区间）→ closes 下标区间（MACD 面积坐标系，Q5 全区间口径）。
    // 越界/空 ⟹ None ⟹ `comparable` 不成立（无面积 ⟹ 无法算 C<A）。
    Some(FirstStructuralGates {
        side: if is_sell { Side::Short } else { Side::Long },
        seg_a: (a_start, a_end),
        lambda_c,
        a_idx: map_src_range_to_close_idx(src_to_idx, a_start, a_end),
        c_idx: map_src_range_to_close_idx(src_to_idx, lambda_c, seg.end_index),
        extreme,
    })
}

/// 第一类买卖点判定（契约锚 `Origin.BspClassification.IsType1 = brokeCenter ∧ isTrend ∧ IsDivergence`；
/// ★A/B/C 趋势背驰框架，第24课:22-24 + reference:34 + maimai.md:103-112）。
///
/// **第一类买卖点 = 趋势背驰点**（maimai.md:103「某级别**下跌趋势**中…向下跌破**最后一个**中枢后
/// 形成的**背驰点**」；前提 maimai.md:105「≥2个依次同向的同级别中枢」）。本函数实装 A/B/C 三段
/// 框架的趋势背驰判定（**非**退化的「任意前同向段」面积比较）：
///
/// - **C 段**（后一离开段）= 破**最后一个中枢** `last_center` 的离开段 `seg`（破中枢几何 L0）。
/// - **A 段**（前一离开段）= **倒数第二个中枢** `prev_center` 的同向离开段（[`locate_trend_seg_a`]
///   跨相邻中枢配对，第24课:24「A 之前已有一个中枢，B 是这个大趋势的另一个中枢」）。
/// - **B 段** = `prev_center` 与 `last_center` 之间的中间中枢（由趋势 ≥2 中枢门控隐含保证）。
/// - **趋势背驰** = C 段面积 < A 段面积（力度衰减，`segments_diverge` 力度原语 L1）。
///
/// - **1买**：下跌趋势（τ=Trend(Down)）中向下线段端点 `< last_center.zd`（破最后中枢下沿）∧
///   C段面积 < A段面积（底背驰）⟹ 1 买。
/// - **1卖**：上涨趋势（τ=Trend(Up)）镜像——向上端点 `> last_center.zg`，顶背驰 ⟹ 1 卖。
///
/// ★τ 门控（消解假背驰=假买卖点头号缺口）：调用方（[`extract_signals`]）已用局部趋势门（decompose，#143）
/// 确认 τ=Trend 才判第一类；`trend_dir`（趋势方向）= τ 的方向，破中枢方向必须与趋势方向一致
/// （下跌趋势=向下破=底背驰；上涨趋势=向上破=顶背驰）——盘整（Consolidation）/退化（mixed/扩张）
/// **不产第一类**（盘整背驰不产第一类，beichi #4 + maimai.md:56 已结算）。
///
/// ★A/C 跨相邻中枢配对（替换退化的「序列序任意前同向段」）：A 段不是任意前同向段，而是趋势中
/// **相邻前一中枢**（`prev_center`）的离开段。退化实现把任意相邻同向段面积变小都判背驰=产假买卖点；
/// 本实装要求 A/C 分属相邻两中枢（第24课:22「同向趋势之间一定有一个…中枢连接」）。
///
/// ★分量 L 级（formalization-validity-domain）：破中枢 `< zd`/`> zg` ∧ **037:20 破 b 包络极值**
///（`< b_lo`/`> b_hi`，b=I(A) 包络，`move_range_envelope` 单一来源）+ 趋势门控（中枢同向关系）
/// 整数几何 **L0**；C<A 面积比较 MACD **L1**。诸分量合取 = 趋势背驰 = `IsType1`。`prev_center` 无
/// 同向离开段（A 段无法定位）⟹ None（无 A/C 配对 ⟹ 无趋势背驰）。
/// ★rust 领先 Origin（裁定 T3 裁决 7，遗留项 L1）：Lean `IsType1 = brokeCenter ∧ isTrend ∧ IsDivergence`
/// 的 `brokeCenter` 只含破核心；本函数判据 = brokeCenter ∧ 037:20 ∧ IsDivergence——037:20 是否
/// 吸进 Lean `IsType1` 结构合取列 formal-chain 后续事项（本函数既有 A/B/C 真算领先的先例）。
///
/// ★#93 性能工位（热点②修复）：A 段 `(a_start, a_end)` 不再在本函数内调 `locate_trend_seg_a`——改由
/// 调用方（[`extract_signals`]）按 `last_center_idx` 缓存预算后传入（`a_seg` 入参）。
/// `locate_trend_seg_a` 与 C 段 `seg` 无关（仅依赖 `(segments, prev_center, last_center, trend_dir)`）
/// ⟹ 趋势 τ 下多段共享同一 `(prev_center, last_center)` 对 ⟹ A 段每对至多算一次，消解旧「每段重算」
/// 的 O(S²)（profile `scale_timing_breaking_path` exp≈1.99 坐实）。
///
/// ★bit-exact 不变式：传 `a_seg = locate_trend_seg_a(segments, prev_center, last_center, trend_dir)`
/// 的结果时，本函数输出与内联调 `locate_trend_seg_a` 的旧实现**逐字段相等**——broke 判定、A/C 段区间、
/// MACD 面积、`AbcDivergence` 结构、`diverges` 判定、`EndpointSituation` 与 `BspPoint` 构造全部共享
/// 同一代码路径（仅 A 段来源从「内联调」变「外传入」，值同）。
///
/// ★`prev_center`/`segments` 不入参：A 段已预算入参（含 prev_center 的语义 + segments 已扫），本函数
/// 不再需要它们（仅 `last_center` 提供 zd/zg 几何判据）。prev_center/segments 的语义在调用方缓存键
/// （last_center_idx ⟹ prev_center=centers[idx-1]）与 `locate_trend_seg_a` 预算中保留。
///
/// `hist` 是 MACD hist 序列；`src_to_idx` 是 closes 下标→source_index 映射。段无法映射到 closes
/// 区间（越界）⟹ None（无面积 ⟹ 非背驰）。
///
/// `a_seg`：`None` = `locate_departure_move_a` 返回 None（无 A 候选）；`Some(((λ_A,ρ_A),(b_lo,b_hi)))`
/// = A 区间 I(A) 及其包络（b = I(A)，037:20 破极值判据的参照极值；包络随 I(A) 同槽缓存，
/// 由调用方经 `move_range_envelope` 预算——单一来源，与 D2 侧 R1 T2 同原语同区间语义）。
/// `c_move_start`：I(C) 起点 λ_C（Q5 + codex ac4 #2）= 离开 `last_center` 的**当前 episode** 首同向
/// 段起点（[`departure_move_c_start`] 单一来源——episode 边界 = seg 之前最后一个回中枢段，「失败
/// 离开→回中枢→重新离开」不桥接）。`None` = 无同向离开段——broke 成立时 seg 自身在窗口且在最后
/// 回中枢段之后 ⟹ 必 `Some`。
///
/// ★Q5 区间语义（task #145）：I(C) = [λ_C, seg.end_index]——离开最后中枢的**整个次级别走势区间**
/// （多段 departure 含中间反向段 bar），MACD 面积/力度 proxy 全用 I(C)。**破中枢几何仍用 seg 端点**
/// （因果触发）：破中枢判据 min P(C) < ZD（Up 镜像 max P(C) > ZG）与「某同向段端点越界」等价——
/// 段是单向对象、终点即极值，多段 move 的 min/max 首次越界 ⟺ 某同向段端点越界，触发时刻即该段，
/// 故 seg 端点判破 = I(C) 极值判破的因果触发点（等价性，裁决注记）。
pub(crate) fn judge_first_cached(
    last_center: &Center,
    trend_dir: Direction,
    seg: &Segment,
    anchor_dir: Option<Direction>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    src_to_idx: &[usize],
    a_seg: Option<((usize, usize), (Tick, Tick))>,
    c_move_start: Option<usize>,
    // ★#1028 裁定 A：一类点点锚 = departure 单元终点（趋势真终点）。`Some(x)` = L≥1 调用方已按
    // `cand_predicate::departure_unit_end` 预算（末子段常是回抽反趋势段，#1035 §1.4）；`None` =
    // L0 / 直调（segment 自身即 departure 单元，点锚回退 `seg.end_index`）。只动锚，不动判定链。
    departure_end: Option<usize>,
    gauge: DivergenceGauge,
    strokes: &[Stroke],
    sorted: &[Segment],
    level: Option<u32>,
    // ★#885 S4-d：分级记录生产 sink——按 `diverged` 无条件捕获（Present 与 Missing 两域），
    // 由提取入口一路上穿进 `LevelState.first_class_grades`（否则域此前只落 thread_local 诊断
    // sidecar，不进 Classification、按坐标查不到）。只建可查性，判据（`t3_grade`/`diverged`）逐字未动。
    grade_sink: &mut Vec<FirstClassGradeRecord>,
) -> Option<BspPoint> {
    // ★#551：三道结构门由 [`first_structural_gates`] 单一来源判定（本函数与 #550 候选事件产出
    // 同读；禁第二套判据，audit §6）。本函数保持「三门全成立才产点」的收缩语义不变。
    let gates = first_structural_gates(
        last_center,
        trend_dir,
        seg,
        anchor_dir,
        src_to_idx,
        a_seg,
        c_move_start,
    )?;
    // ★SPEC #1077 3a：门判定后的第一类投影与合并扫描共享（纯函数抽取，判据零改动）——
    // 候选域经同一 gates 走 [`super::super::cand_event`] 投影，不再各自重算结构门。
    judge_first_from_gates(
        gates,
        last_center,
        trend_dir,
        seg,
        departure_end,
        hist,
        dif,
        closes_tick,
        gauge,
        strokes,
        sorted,
        level,
        grade_sink,
    )
}

/// ★SPEC #1077 3a：共享 per-segment 素材的第一类投影（[`judge_first_cached`] 门判定之后的后半段，
/// 逐字段同构，纯函数抽取）。
///
/// `gates` 已由合并扫描预算传入（一次判定，BSP 与候选两域同读）；本函数只做「已知门 ⟹ 力度
/// 确认 ⟹ 产点」的判定，判据（`extreme`/`diverged`/`t3_grade`/force/分级记录）逐字未动。
#[allow(clippy::too_many_arguments)]
pub(crate) fn judge_first_from_gates(
    gates: FirstStructuralGates,
    last_center: &Center,
    trend_dir: Direction,
    seg: &Segment,
    // ★#1028 裁定 A：一类点点锚 = departure 单元终点（趋势真终点）。`Some(x)` = L≥1 调用方已按
    // `cand_predicate::departure_unit_end` 预算；`None` = L0 / 直调（segment 自身即 departure
    // 单元，点锚回退 `seg.end_index`）。只动锚，不动判定链。
    departure_end: Option<usize>,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
    sorted: &[Segment],
    level: Option<u32>,
    // ★#885 S4-d：分级记录生产 sink——按 `diverged` 无条件捕获（Present 与 Missing 两域）。
    grade_sink: &mut Vec<FirstClassGradeRecord>,
) -> Option<BspPoint> {
    if !gates.extreme {
        super::super::diag::s2_mirror_capture::record_first_assembly(
            level.unwrap_or(0),
            last_center,
            trend_dir,
            seg,
            departure_end,
            gates,
            gauge,
            hist,
            dif,
            closes_tick,
            strokes,
            sorted,
            None,
            None,
            None,
        );
        return None; // 未破 b 包络极值 ⟹ 非趋势背驰 c（061:28：未创新极值不构成背驰）。
    }
    let (Some(c_idx), Some(a_idx)) = (gates.c_idx, gates.a_idx) else {
        super::super::diag::s2_mirror_capture::record_first_assembly(
            level.unwrap_or(0),
            last_center,
            trend_dir,
            seg,
            departure_end,
            gates,
            gauge,
            hist,
            dif,
            closes_tick,
            strokes,
            sorted,
            None,
            None,
            None,
        );
        // 区间无法映射到 closes（越界/空）⟹ 无 MACD 面积 ⟹ 无法算 C<A ⟹ 无 struct_break 候选。
        return None;
    };
    let end = seg_end(seg);
    // ★#1028 裁定 A：点锚 = departure 单元终点；无预算（L0/直调）回退 seg.end_index（旧口径）。
    // 判据链的 I(C) 区间（lambda_c..seg.end_index）与力度 proxy 仍用 seg.end_index——只动锚，不动判据。
    let point_src = departure_end.unwrap_or(end.source_index);
    let is_sell = gates.side == Side::Short;
    let (a_start, a_end) = gates.seg_a;
    let lambda_c = gates.lambda_c;
    // ★A/B/C 背驰段对（结构化，对齐 Lean `Origin.Divergence.DivergencePair { forceA, forceC, isTrend }`）：
    // I(A) + I(C)（source_index 区间，Q5 走势区间口径）+ is_trend=true（第一类只由趋势背驰产）。
    let abc = AbcDivergence {
        seg_a: (a_start, a_end),
        seg_c: (lambda_c, seg.end_index),
        is_trend: true,
    };
    // ★P2-R2（codex-decide-20260701-2121 → p2-plan §2）：C<A 从 gate 降为「buy1 判据」——**不 return
    // None**，破中枢结构候选（趋势 ∧ 破最后中枢 ∧ A/C 可配对）全部进样本（消选择偏差，下游 χ 可否证
    // MACD）。趋势背驰（L1 真算）：C 段面积严格小于 A 段面积（第24课:24）。
    // ★macd_c_lt_a 不再作独立 StructBreakFeature sidecar（P2-R2 删除死代码，codex 护栏7）——它**完全
    // 可从产出 BspPoint 派生**：`struct_break_dir=Some ∧ bits.buy1/sell1=true` ⟺ macd_c_lt_a=true
    // （背驰确认）；`struct_break_dir=Some ∧ 六 bit 全零` ⟺ macd_c_lt_a=false（未背驰）。sidecar 从
    // 未接通生产（mod.rs 全走 `.0`），删除比接通更干净且零信息损失。
    let macd_c_lt_a = abc.diverges(hist, a_idx, c_idx, trend_dir);
    // ★力度多 proxy（beta-route #115，force-proxy-survey-20260702.md）：A/C 段 close 下标区间已算出
    // （a_idx/c_idx），复用 force_features 算 5 proxy（MACD 面积/DIF 峰/振幅/速度/TV）。默认口径下
    // **不进 buy1 判据**（class_index 冻结，671 力度=feature 非 veto）——收进 BspPoint.force 单一来源，
    // 供 Candidate.force 透传（A6 #159）后由 selector z_of_candidate 读进 force_state 第 8 维。有
    // dif/closes_tick 输入时 Some（生产热路径已接线，Batch 2）；空输入（旧测试/合成入口）⟹ None
    // （诚实不造死字段）。A/C 同趋势方向。
    let force = if dif.is_empty() || closes_tick.is_empty() {
        None
    } else {
        Some(ForceProxies {
            seg_a: force_features(hist, dif, closes_tick, a_idx.0, a_idx.1, trend_dir),
            seg_c: force_features(hist, dif, closes_tick, c_idx.0, c_idx.1, trend_dir),
        })
    };
    // ★#990 I-2：教义判据 `L(C) < L(B)`（#873，经 #989 反查原语）随 ForceL 默认档接入；
    // 无 strokes / 段区间无笔 ⟹ None（不判，不降级——收敛通则禁宽松接管）。
    // seg_a/seg_c 区间是 source_index 坐标（与 strokes 同系，I-1 已核）。
    let l_c_lt_a = if strokes.is_empty() {
        None
    } else {
        match (
            crate::theta_v0::parser::segment::segment_force_l(strokes, a_start, a_end),
            crate::theta_v0::parser::segment::segment_force_l(strokes, lambda_c, seg.end_index),
        ) {
            (Some(la), Some(lc)) => Some(lc < la),
            _ => None,
        }
    };
    // ★D 判定口径（A2 #163 + #990）：`confirm_divergence_l` 单一判定点。默认 `ForceL` ⟹
    // 教义判据；`MacdArea` 等降为显式对照档（ADR-0005）。
    let diverged = divergence::confirm_divergence_l(gauge, macd_c_lt_a, force.as_ref(), l_c_lt_a);
    // ★#1229 裁定 a（2026-08-25）：T3-in-c 合取统一走 [`trend_third_class_in_c`] 全窗后扫
    // （037:18「至少一个」语义）——大闸（#607 D2）改**消费后扫结果**，不再自跑固定首对
    // （`t3_in_c_fixed_first_pair` 已退役为诊断词汇）。否则域（`Missing`）不置一类 bit，点降级
    // 为零 bit 结构候选，继续走既有候选流（P2-R2 先例，`struct_break_dir` 无条件置，见下）。
    // `THETA_T3INC_SKIP=1`（D5 counterfactual）⟹ 强制跳过本合取，恢复 #606 前旧行为
    // （T3-in-c 恒 Present，仅 `diverged` 门控一类 bit）；该臂为 counterfactual，不参生产对拍。
    let t3_grade = t3_in_c_scan_grade(
        sorted,
        last_center,
        trend_dir,
        gates.lambda_c,
        seg.end_index,
    );
    let t3_in_c_present = t3_in_c_skip_enabled() || matches!(t3_grade, T3InCScan::Present { .. });
    // ★#607 S2 D4：观测面 sidecar 挂点移入判据本体（D2 后 bits 已被 T3-in-c 二次门控，仅凭
    // `pf.bits` 观测会漏记否则域点——须在门控**之前**按 `diverged` 捕获，见 #606 S1 报告
    // §8.3 教训同源）。生产 `judge_segment`（level=Some，incremental resume 传参穿透）与
    // 3b 投影 `scan::project_cand_delta_events`（经同一单段核 `merged_judge_segment`，同携
    // level=Some）共享同一挂点——P1 的 `level_cand_delta`（level=None，#529 塔地盘不参与捕获）
    // 已随 3b 退役（ADR 0026）。仅 sidecar 已打开（env 门控，`otherwise_domain_sidecar_begin`）
    // 时捕获；投影重跑同核捕获的记录与生产帧逐值相同（upsert 幂等）。
    // ★#607 F4 登记：sidecar 记的是**真实** `t3_grade`（Present/Missing 如实反映 T3-in-c 判据），
    // 不受 `THETA_T3INC_SKIP` 影响；但 `THETA_T3INC_SKIP=1` 时下方 `below_last_center`（一类 bit）
    // 强制按 `diverged` 置（跳过 T3-in-c 二次门控，见上）。两者在 skip 臂下脱钩——bit 驱动的
    // Reset 计数（59，旧行为）与 sidecar 按真实 grade 切的 native/otherwise 桶（native=22）
    // 不再一一对应。skip 臂只用于 trades/tower 字节级反证（D5 §5.1），**不跑**②基数对拍
    // （native_count == reset_count）——两臂互斥是设计，非 bug。
    // ★#885 S4-d：分级记录**无条件**落生产 sink（按 `diverged` 捕获，Present 与 Missing 两域，
    // 与 sidecar 同口径）；`level=None`（全量 fallback）时 `level` 字段先落占位 0，由
    // `Classification` 装配方按真实级别盖章（见 [`FirstClassGradeRecord`] 文档）。sidecar 捕获
    // 维持原契约不变：仅 `level=Some`（生产 resume 路径）且 sidecar 已打开时写入——
    // `fallback_full_recompute_does_not_capture_grade_sidecar` 锁的就是这条边界。
    let grade_output = if diverged {
        let rec = FirstClassGradeRecord {
            level: level.unwrap_or(0),
            source_index: point_src,
            side: if is_sell { Side::Short } else { Side::Long },
            center_start_index: last_center.start_index,
            center_end_index: last_center.end_index,
            center_zd: last_center.zd,
            center_zg: last_center.zg,
            grade: t3_grade,
        };
        grade_sink.push(rec);
        if level.is_some() && GRADE_SIDECAR.with(|c| c.borrow().is_some()) {
            GRADE_SIDECAR.with(|cell| {
                if let Some(v) = cell.borrow_mut().as_mut() {
                    v.push(rec);
                }
            });
        }
        Some(rec)
    } else {
        None
    };
    // buy1/sell1 保严格「趋势背驰 ∧ T3-in-c」语义：仅背驰确认（D 成立）∧ T3-in-c 判 `Present`
    // 才置第一类 bit。否则（未背驰，或否则域 T3-in-c `Missing`）的破中枢候选进样本但**零
    // buy1/sell1**（Flat/否则域候选，assemble_gamma 归 𝒦 不冒充第一类，codex 语义纪律 + #607 D2）。
    let situ = EndpointSituation {
        after_first_buy: false,
        is_pullback_end: false,
        left_center: false, // 第一类是破中枢趋势背驰，非第三类的离开后回抽
        retrace_not_reenter: false,
        below_last_center: diverged && t3_in_c_present, // #607 D2：背驰 ∧ T3-in-c Present 才置一类端点语义
        // #816 B-1（#903 落实）：一类点判据 = 破中枢 ∧ 背驰——diverged 的口径即统一背驰判据
        //（beichi.md #814 正本；次级别含 Extreme；MACD 同色为辅助测量，默认 ForceL 教义档 #990）。
        is_sell_side: is_sell,
    };
    let bits = endpoint_to_bsp(&situ);
    // ★P2-R2（p2-plan §2 + codex 护栏1/2）：破中枢方向源——买侧向下破=Long，卖侧向上破=Short。
    // **无条件置**（只要几何破了最后中枢 ∧ A/C 可配对就到这里，不管 D 背驰与否）。
    // buy1/sell1 仍严格按 D（上方 situ.below_last_center）——默认口径下 D≡macd_c_lt_a 冻结语义。
    // 零 bit（未背驰）候选靠 struct_break_dir 在 candidate_dir 恢复方向进样本（消选择偏差）。
    let struct_break_dir = Some(if is_sell { Side::Short } else { Side::Long });
    // 结构候选端点：背驰确认 ⟹ buy1/sell1 止损源 pivot（破中枢段端点极值）；未背驰 ⟹ 零 bit，
    // pivot 仍按 bit 方向填（零 bit ⟹ 两侧 0）。force 旁挂进点（单一来源，不进任何 bit 判据）。
    // ★owner 载体补齐（关③ 补记② 路径 (a)）：判定中枢 last_center 构造时填载（center=Some）。
    // ★#1028 裁定 A：source_index 传 departure 单元终点（point_src），非 seg.end_index（回抽段终点）。
    let point = make_first_point(
        point_src,
        bits,
        end.price,
        last_center,
        struct_break_dir,
        force,
    );
    super::super::diag::s2_mirror_capture::record_first_assembly(
        level.unwrap_or(0),
        last_center,
        trend_dir,
        seg,
        departure_end,
        gates,
        gauge,
        hist,
        dif,
        closes_tick,
        strokes,
        sorted,
        grade_output,
        Some(t3_grade),
        Some(&point),
    );
    Some(point)
}
