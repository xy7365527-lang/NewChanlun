//! GUARD-ROLE: toplevel-loose-files——名分：现役（详见 `lib.rs` 头部 GUARD-ROLE 块，#764 C7-E5 核定；零删除/零移入 legacy/）。
//!
//! 背驰 / 盘整背驰 v1 — 逐位等价移植自 src/newchan/a_divergence_v1.py
//!
//! 检测每个 Move 的趋势背驰（C 段力度 < A 段力度）或盘整背驰（同向离开力度衰竭）。
//!
//! ## 两条力度路径（formalization-validity-domain.md）
//! 1. **价格振幅 fallback**（`_compute_force` else 分支）：force = (high-low) * duration。
//!    三维度 OR 退化为 `force_c < force_a`（dif/hist 峰值恒 0 → T6/T7 恒 False）。
//!    对应 Python `divergences_from_moves_v1(df_macd=None)`。
//! 2. **MACD 三维度**（第七层接通）：force = abs(area_pos|area_neg)；T6=DIF 峰值；
//!    T7=HIST 峰值；T4=B 段黄白线穿越 0 轴。对应 Python
//!    `divergences_from_moves_v1(df_macd=df, merged_to_raw=m2r)`。
//!
//! 是否走 MACD 由 `MacdCtx` 是否为 `Some` 决定（复刻 Python `df_macd is not None
//! and merged_to_raw is not None`）。golden 测试两条路径都在 L1/L2 数据上 bit-exact。
//!
//! ## 逐位等价要点
//! 1. fallback force 唯一的浮点算术是 `(high-low) * duration`（单次乘法，IEEE-754 bit-exact）。
//! 2. high/low 由 max/min 顺序约简（与 Python `max`/`min` 同序），bit-exact。
//! 3. MACD force 走 `macd::macd_area_for_range`（pairwise 累加 + round6，bit-exact）。
//! 4. 盘整背驰按方向迭代顺序 up→down（复刻 Python dict 插入序），首个成立即返回。
//! 5. seg 索引算术用 i64 复刻 Python int（a_end = zs_last.seg_start-1 可为负，退化分支处理）。

/// 背驰类型。对应 Python Literal["trend", "consolidation"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DivKind {
    Trend,
    Consolidation,
}

impl DivKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DivKind::Trend => "trend",
            DivKind::Consolidation => "consolidation",
        }
    }
}

/// 背驰方向。对应 Python Literal["top", "bottom"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DivDir {
    Top,
    Bottom,
}

impl DivDir {
    pub fn as_str(self) -> &'static str {
        match self {
            DivDir::Top => "top",
            DivDir::Bottom => "bottom",
        }
    }
}

/// 背驰检测所需的最小线段视图（direction/high/low/i0/i1）。
#[derive(Debug, Clone, Copy)]
pub struct SegView {
    pub direction: crate::stroke::Direction,
    pub high: f64,
    pub low: f64,
    pub i0: usize,
    pub i1: usize,
    /// 该段所代表的「次级别走势」是否已完成（线段已被后续线段破坏，`Segment.confirmed`，
    /// 第65课线段分解定理「线段破坏的充要条件就是被另一线段破坏」）。
    /// `require_settled` 模式下作为 BSP `confirmed` 的合取前提（编排者 2026-06-13：a₀=线段
    /// 让 segment=次级别走势在形式上成立，但 segment 的生成≠走势完成——用未 settle 的
    /// 生长中线段判 BSP = 伪信号，正是 §3 生长期 C 段伪背驰的源头，B2 未覆盖的 pending 侧）。
    /// 非 require_settled 路径不读此字段（背驰检测只用 direction/high/low/i0/i1）。
    pub settled: bool,
}

/// 背驰检测所需的最小中枢视图。
#[derive(Debug, Clone, Copy)]
pub struct ZsView {
    pub zd: f64,
    pub zg: f64,
    pub seg_start: usize,
    pub seg_end: usize,
    pub settled: bool,
}

/// 走势视图。背驰用 kind/direction/seg_end/zs_*；BSP 额外用 seg_start/settled。
#[derive(Debug, Clone, Copy)]
pub struct MoveView {
    pub kind: crate::moves::MoveKind,
    pub direction: crate::stroke::Direction,
    pub seg_start: i64,
    pub seg_end: i64,
    pub zs_start: usize,
    pub zs_end: usize,
    pub zs_count: usize,
    pub settled: bool,
}

/// 背驰结果。对应 Python `Divergence` frozen dataclass。
/// dif_peak/hist_peak 恒 0.0（fallback 无 MACD 维度）。
#[derive(Debug, Clone, Copy)]
pub struct Divergence {
    pub kind: DivKind,
    pub direction: DivDir,
    pub level_id: i64,
    pub seg_a_start: i64,
    pub seg_a_end: i64,
    pub seg_c_start: i64,
    pub seg_c_end: i64,
    pub center_idx: usize,
    pub force_a: f64,
    pub force_c: f64,
    pub confirmed: bool,
    pub dif_peak_a: f64,
    pub dif_peak_c: f64,
    pub hist_peak_a: f64,
    pub hist_peak_c: f64,
}

/// MACD 上下文。携带 df_macd 的 macd/hist 两列 + merged→raw 映射。
/// 对应 Python `df_macd`（仅 'macd'/'hist' 列参与）+ `merged_to_raw`。
///
/// `Some(MacdCtx)` ⇔ Python `df_macd is not None and merged_to_raw is not None`。
pub struct MacdCtx<'a> {
    /// df_macd['macd'] 列（DIF / 黄白线）。
    pub macd: &'a [f64],
    /// df_macd['hist'] 列（柱状图）。
    pub hist: &'a [f64],
    /// merged bar → raw bar 范围映射 [(raw_lo, raw_hi); n_merged]。
    pub merged_to_raw: &'a [(usize, usize)],
}

impl MacdCtx<'_> {
    /// merged 索引 → raw_i0（复刻 `merged_to_raw[i0][0] if i0 < len else 0`）。
    fn raw_i0(&self, merged_i0: usize) -> i64 {
        if merged_i0 < self.merged_to_raw.len() {
            self.merged_to_raw[merged_i0].0 as i64
        } else {
            0
        }
    }

    /// merged 索引 → raw_i1（复刻 `merged_to_raw[i1][1] if i1 < len else 0`）。
    fn raw_i1(&self, merged_i1: usize) -> i64 {
        if merged_i1 < self.merged_to_raw.len() {
            self.merged_to_raw[merged_i1].1 as i64
        } else {
            0
        }
    }
}

/// 获取 segments[start..=end] 覆盖的 merged bar 索引范围 (i0, i1)。
/// 移植自 `_seg_merged_range`：范围非法返回 (0, 0)。
fn seg_merged_range(segs: &[SegView], seg_start: i64, seg_end: i64) -> (usize, usize) {
    let n = segs.len() as i64;
    if seg_start > seg_end || seg_start < 0 || seg_end >= n {
        return (0, 0);
    }
    (segs[seg_start as usize].i0, segs[seg_end as usize].i1)
}

/// 计算一段 segments 的力度。移植自 `_compute_force`（MACD 分支 + fallback 分支）。
///
/// 范围非法返回 0.0。有 MACD 时用 MACD 面积（up→abs(area_pos)，down→abs(area_neg)），
/// 否则用价格振幅 × 持续 bar 数。
fn compute_force(
    segs: &[SegView],
    seg_start: i64,
    seg_end: i64,
    direction: crate::stroke::Direction,
    macd: Option<&MacdCtx>,
) -> f64 {
    let n = segs.len() as i64;
    if seg_start > seg_end || seg_start < 0 || seg_end >= n {
        return 0.0;
    }
    let (i0, i1) = seg_merged_range(segs, seg_start, seg_end);

    if let Some(ctx) = macd {
        let raw_i0 = ctx.raw_i0(i0);
        let raw_i1 = ctx.raw_i1(i1);
        let area = crate::macd::macd_area_for_range(ctx.hist, raw_i0, raw_i1);
        return match direction {
            crate::stroke::Direction::Up => area.area_pos.abs(),
            crate::stroke::Direction::Down => area.area_neg.abs(),
        };
    }

    // Fallback：价格振幅 × 持续 bar 数。
    let (us, ue) = (seg_start as usize, seg_end as usize);
    let mut high = segs[us].high;
    let mut low = segs[us].low;
    for k in (us + 1)..=ue {
        high = high.max(segs[k].high);
        low = low.min(segs[k].low);
    }
    let duration = (i1 as i64 - i0 as i64).max(1);
    (high - low) * duration as f64
}

/// 计算一段 segments 的 (dif_peak, hist_peak)。移植自 `_compute_macd_peaks`。
/// 无 MACD 返回 (0.0, 0.0)。
fn compute_macd_peaks(
    segs: &[SegView],
    seg_start: i64,
    seg_end: i64,
    direction: crate::stroke::Direction,
    macd: Option<&MacdCtx>,
) -> (f64, f64) {
    let ctx = match macd {
        None => return (0.0, 0.0),
        Some(c) => c,
    };
    let (i0, i1) = seg_merged_range(segs, seg_start, seg_end);
    let raw_i0 = ctx.raw_i0(i0);
    let raw_i1 = ctx.raw_i1(i1);
    let up = matches!(direction, crate::stroke::Direction::Up);
    (
        crate::macd::dif_peak_for_range(ctx.macd, raw_i0, raw_i1, up),
        crate::macd::histogram_peak_for_range(ctx.hist, raw_i0, raw_i1, up),
    )
}

/// 收集 [zs_start, zs_end] 范围内的 settled 中枢索引。移植自 `_collect_settled_zs_indices`。
fn collect_settled_zs_indices(zss: &[ZsView], zs_start: usize, zs_end: usize) -> Vec<usize> {
    let upper = (zs_end + 1).min(zss.len());
    (zs_start..upper).filter(|&i| zss[i].settled).collect()
}

/// B2（C 段越界极值定义）：`after` 之后第一个 settled 中枢的 seg_end。
/// None ⟺ 无后继 settled 中枢（pending move），调用方代入 n-1。
///
/// 贪心分组对 settled 中枢做连续分划 ⟹ 此中枢恰为下一 move 的首中枢——
/// C 段搜索窗口允许越入其覆盖区（缠师第24课：背驰段终于走势转折点；
/// 相邻中枢首尾相接时转折极值落在下一中枢覆盖区内，旧定义 c_start > c_end
/// 使 C 段在 settle 瞬间归零——见 analysis/engine_bsp_gap_diagnosis.md §2.2 机制二）。
pub(crate) fn next_settled_zs_seg_end(zss: &[ZsView], after: usize) -> Option<i64> {
    zss[after + 1..]
        .iter()
        .find(|z| z.settled)
        .map(|z| z.seg_end as i64)
}

/// B2：窗口 [lo, hi] 内的趋势极值段（down→最低 low，up→最高 high）。
/// 平值取**首个**（复刻 Python `min`/`max` 返回首个最优元素）。
fn trend_extreme_seg(
    segs: &[SegView],
    lo: usize,
    hi: usize,
    direction: crate::stroke::Direction,
) -> i64 {
    let mut best = lo;
    match direction {
        crate::stroke::Direction::Up => {
            let mut bv = segs[lo].high;
            for (k, seg) in segs.iter().enumerate().take(hi + 1).skip(lo + 1) {
                if seg.high > bv {
                    bv = seg.high;
                    best = k;
                }
            }
        }
        crate::stroke::Direction::Down => {
            let mut bv = segs[lo].low;
            for (k, seg) in segs.iter().enumerate().take(hi + 1).skip(lo + 1) {
                if seg.low < bv {
                    bv = seg.low;
                    best = k;
                }
            }
        }
    }
    best as i64
}

/// 增量器 commit 判据：单 move 背驰检测的最大段依赖索引。
///
/// trend（zs_count≥2 且范围内 ≥2 settled 中枢）→ max(mv.seg_end, B2 窗口上限)；
/// 窗口无界（无后继 settled 中枢 ⟹ pending move，上限随 n 增长）→ `i64::MAX`
/// （不可 commit，留在易变尾每次重算）。其余（盘整/前提不满足）→ mv.seg_end。
pub(crate) fn detect_dep_end(zss: &[ZsView], mv: &MoveView) -> i64 {
    if mv.kind != crate::moves::MoveKind::Trend || mv.zs_count < 2 {
        return mv.seg_end;
    }
    let indices = collect_settled_zs_indices(zss, mv.zs_start, mv.zs_end);
    if indices.len() < 2 {
        return mv.seg_end;
    }
    match next_settled_zs_seg_end(zss, indices[indices.len() - 1]) {
        Some(e) => mv.seg_end.max(e),
        None => i64::MAX,
    }
}

/// 趋势背驰 A 段 seg 范围（前枢结束+1 → 后枢开始-1，紧邻时退化）。移植自 `_trend_a_segment_range`。
fn trend_a_segment_range(zs_prev: &ZsView, zs_last: &ZsView) -> (i64, i64) {
    let mut a_start = zs_prev.seg_end as i64 + 1;
    let mut a_end = zs_last.seg_start as i64 - 1;
    if a_start > a_end {
        a_start = zs_prev.seg_end as i64;
        a_end = zs_prev.seg_end as i64;
    }
    (a_start, a_end)
}

/// 三维度 OR 判定。移植自 `_check_three_dim_divergence`。
/// T2: force_c < force_a；T6: dif_a>0 且 dif_c<dif_a；T7: hist_a>0 且 hist_c<hist_a。任一即背驰。
/// 无 MACD 时 dif/hist 恒 0 → T6/T7 恒 False → 退化为 T2。
#[inline]
fn check_three_dim(
    force_a: f64,
    force_c: f64,
    dif_a: f64,
    dif_c: f64,
    hist_a: f64,
    hist_c: f64,
) -> bool {
    let t2 = force_c < force_a;
    let t6 = dif_a > 0.0 && dif_c < dif_a;
    let t7 = hist_a > 0.0 && hist_c < hist_a;
    t2 || t6 || t7
}

/// 计算 A/C 力度 + MACD 峰值 + 三维度判定，构造 Divergence。移植自 `_compare_and_build`。
#[allow(clippy::too_many_arguments)]
fn compare_and_build(
    segs: &[SegView],
    direction: crate::stroke::Direction,
    a_start: i64,
    a_end: i64,
    c_start: i64,
    c_end: i64,
    center_idx: usize,
    kind: DivKind,
    level_id: i64,
    confirmed: bool,
    macd: Option<&MacdCtx>,
) -> Option<Divergence> {
    let force_a = compute_force(segs, a_start, a_end, direction, macd);
    let force_c = compute_force(segs, c_start, c_end, direction, macd);
    if force_a <= 0.0 {
        return None;
    }
    let (dif_a, hist_a) = compute_macd_peaks(segs, a_start, a_end, direction, macd);
    let (dif_c, hist_c) = compute_macd_peaks(segs, c_start, c_end, direction, macd);

    if !check_three_dim(force_a, force_c, dif_a, dif_c, hist_a, hist_c) {
        return None;
    }
    let div_dir = match direction {
        crate::stroke::Direction::Up => DivDir::Top,
        crate::stroke::Direction::Down => DivDir::Bottom,
    };
    Some(Divergence {
        kind,
        direction: div_dir,
        level_id,
        seg_a_start: a_start,
        seg_a_end: a_end,
        seg_c_start: c_start,
        seg_c_end: c_end,
        center_idx,
        force_a,
        force_c,
        confirmed,
        dif_peak_a: dif_a,
        dif_peak_c: dif_c,
        hist_peak_a: hist_a,
        hist_peak_c: hist_c,
    })
}

/// T4 前提检查：B 段（最后中枢）黄白线穿越 0 轴。移植自 `_trend_t4_check` + `_b_segment_crosses_zero`。
/// 无 MACD 时直接通过（返回 true）。
fn trend_t4_check(segs: &[SegView], zs_last: &ZsView, macd: Option<&MacdCtx>) -> bool {
    let ctx = match macd {
        None => return true,
        Some(c) => c,
    };
    // B 段的 merged bar 范围（复刻 Python：seg_start/seg_end 越界守卫返回 0）。
    let i0 = if zs_last.seg_start < segs.len() {
        segs[zs_last.seg_start].i0
    } else {
        0
    };
    let i1 = if zs_last.seg_end < segs.len() {
        segs[zs_last.seg_end].i1
    } else {
        0
    };
    let raw_i0 = ctx.raw_i0(i0);
    let raw_i1 = ctx.raw_i1(i1);
    crate::macd::b_segment_crosses_zero(ctx.macd, raw_i0, raw_i1)
}

/// 检测单个趋势 Move 的背驰。移植自 `_detect_trend_divergence`（T4 无 MACD 直接通过）。
fn detect_trend_divergence(
    segs: &[SegView],
    zss: &[ZsView],
    mv: &MoveView,
    level_id: i64,
    macd: Option<&MacdCtx>,
) -> Option<Divergence> {
    if mv.kind != crate::moves::MoveKind::Trend || mv.zs_count < 2 {
        return None;
    }
    let indices = collect_settled_zs_indices(zss, mv.zs_start, mv.zs_end);
    if indices.len() < 2 {
        return None;
    }
    let zs_prev = &zss[indices[indices.len() - 2]];
    let zs_last = &zss[indices[indices.len() - 1]];

    let (a_start, a_end) = trend_a_segment_range(zs_prev, zs_last);
    let c_start = zs_last.seg_end as i64 + 1;
    let n = segs.len() as i64;
    // B2（C 段越界极值定义）：C 段不被 mv.seg_end 截断——
    // 搜索窗口 = [c_start, 下一 settled 中枢 seg_end]（无 → n-1），
    // C 段终点 = 窗口内趋势极值段（走势转折点，第24课）。
    let last_idx = indices[indices.len() - 1];
    let search_end = next_settled_zs_seg_end(zss, last_idx)
        .unwrap_or(n - 1)
        .min(n - 1);
    if c_start > search_end || a_start >= n {
        return None;
    }
    let c_end = trend_extreme_seg(segs, c_start as usize, search_end as usize, mv.direction);
    // T4 前提（B 段黄白线穿越 0 轴）：有 MACD 时检查，无 MACD 时直接通过。
    if !trend_t4_check(segs, zs_last, macd) {
        return None;
    }
    compare_and_build(
        segs,
        mv.direction,
        a_start,
        a_end,
        c_start,
        c_end,
        indices[indices.len() - 1],
        DivKind::Trend,
        level_id,
        true,
        macd,
    )
}

/// 收集离开中枢的段索引，按方向分组 (up, down)。移植自 `_collect_exit_segments`。
fn collect_exit_segments(
    segs: &[SegView],
    zs: &ZsView,
    move_seg_end: i64,
) -> (Vec<usize>, Vec<usize>) {
    let mut up: Vec<usize> = Vec::new();
    let mut down: Vec<usize> = Vec::new();
    let n = segs.len();
    // 中枢内超出 [ZD, ZG] 的段
    let inner_upper = (zs.seg_end + 1).min(n);
    for k in zs.seg_start..inner_upper {
        let seg = &segs[k];
        if seg.high > zs.zg || seg.low < zs.zd {
            match seg.direction {
                crate::stroke::Direction::Up => up.push(k),
                crate::stroke::Direction::Down => down.push(k),
            }
        }
    }
    // 中枢之后到 Move 终点
    let outer_start = zs.seg_end + 1;
    let outer_upper = ((move_seg_end + 1).max(0) as usize).min(n);
    for k in outer_start..outer_upper {
        match segs[k].direction {
            crate::stroke::Direction::Up => up.push(k),
            crate::stroke::Direction::Down => down.push(k),
        }
    }
    (up, down)
}

/// 检测单个盘整 Move 的盘整背驰。移植自 `_detect_consolidation_divergence`。
///
/// 方向迭代顺序 up→down（复刻 Python dict 插入序），首个成立即返回。
fn detect_consolidation_divergence(
    segs: &[SegView],
    zss: &[ZsView],
    mv: &MoveView,
    level_id: i64,
    macd: Option<&MacdCtx>,
) -> Option<Divergence> {
    if mv.kind != crate::moves::MoveKind::Consolidation || mv.zs_count < 1 {
        return None;
    }
    if mv.zs_start >= zss.len() {
        return None;
    }
    let zs = &zss[mv.zs_start];
    let (up_exits, down_exits) = collect_exit_segments(segs, zs, mv.seg_end);

    for (direction, exits) in [
        (crate::stroke::Direction::Up, &up_exits),
        (crate::stroke::Direction::Down, &down_exits),
    ] {
        if exits.len() < 2 {
            continue;
        }
        let a_idx = exits[exits.len() - 2] as i64;
        let c_idx = exits[exits.len() - 1] as i64;
        let result = compare_and_build(
            segs,
            direction,
            a_idx,
            a_idx,
            c_idx,
            c_idx,
            mv.zs_start,
            DivKind::Consolidation,
            level_id,
            true,
            macd,
        );
        if result.is_some() {
            return result;
        }
    }
    None
}

/// 检测所有 Move 的背驰。移植自 `divergences_from_moves_v1`。
///
/// `macd = None` ⇔ Python `df_macd=None`（价格振幅 fallback）。
/// `macd = Some(ctx)` ⇔ Python `df_macd=df, merged_to_raw=m2r`（MACD 三维度）。
/// 每个 Move 最多报告一个背驰（趋势优先，其次盘整）。
pub fn divergences_from_moves_v1(
    segs: &[SegView],
    zss: &[ZsView],
    moves: &[MoveView],
    level_id: i64,
    macd: Option<&MacdCtx>,
) -> Vec<Divergence> {
    let mut result: Vec<Divergence> = Vec::new();
    for mv in moves {
        if let Some(div) = detect_trend_divergence(segs, zss, mv, level_id, macd) {
            result.push(div);
            continue;
        }
        if let Some(div) = detect_consolidation_divergence(segs, zss, mv, level_id, macd) {
            result.push(div);
        }
    }
    result
}

// ════════════════════════════════════════════════════════════
// 增量背驰引擎 — 消除 `divergences_from_moves_v1` 逐笔全量段遍历 O(S) 的 O(S²) 累积
// ════════════════════════════════════════════════════════════

/// 单 Move 背驰检测（趋势优先，其次盘整）。复刻 `divergences_from_moves_v1` 的 per-move 体。
/// `macd=None`（bi-zhongshu 路径价格振幅 fallback）。
pub(crate) fn detect_one(
    segs: &[SegView],
    zss: &[ZsView],
    mv: &MoveView,
    level_id: i64,
) -> Option<Divergence> {
    detect_trend_divergence(segs, zss, mv, level_id, None)
        .or_else(|| detect_consolidation_divergence(segs, zss, mv, level_id, None))
}

/// 增量背驰构造器 —— bit-exact 等价于逐前缀 `divergences_from_moves_v1(.., None)`，摊还 O(1)/笔。
///
/// ## 不变量（bit-exact 契约，differential test 守卫）
/// `current() == divergences_from_moves_v1(segs, zss, moves, level_id, None)`。
///
/// ## 增量原理
/// 背驰为 per-move 计算（每 move 0 或 1 个 div）。closed move（IncrementalMoves 已封闭）的
/// zs 范围 ∈ settled 前缀、段范围固定 ⟹ div 永久固定。唯一易变的是最后一个 pending move 的
/// div。故缓存 closed move 的 div + 每次只重算 pending move 的 div。closed move 的 div 在其
/// 封闭瞬间 detect 一次（依赖全在固定前缀，与后续 segs/zss 增长无关）。
///
/// **B2 下 closed move 仍永久**：B2 的趋势 C 段窗口上限 = 下一 settled 中枢 seg_end。
/// move 封闭 ⟺ 后继 settled 中枢存在（greedy 因它封组）；笔级中枢 append-only、
/// settled 中枢 extent 永久固定、confirmed 笔 append-only ⟹ 窗口与窗口内段在封闭
/// 瞬间即逐位固定。pending move 无后继 settled 中枢（窗口上限 = n-1 随笔增长）→ 每次重算。
#[derive(Debug, Clone, Default)]
pub struct IncrementalDivergences {
    /// closed move 产生的 div（仅成立的，按 move 顺序；永久固定）。
    closed_divs: Vec<Divergence>,
    /// 已 detect 的 closed move 数。
    processed_closed: usize,
    /// 上次并入 full_divs 的 closed_divs 长度（增量拼接锚）。
    prev_closed_div_len: usize,
    /// 全量 divs = closed_divs + pending move 的 div（增量维护）。
    full_divs: Vec<Divergence>,
}

impl IncrementalDivergences {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新背驰列表。
    ///
    /// `closed_moves`：永久封闭的 move 前缀（IncrementalMoves.closed_moves，append-only）。
    /// `pending`：当前最后一个未封闭 move（None = 无 settled 中枢）。
    /// `segs`/`zss`：当前全量段/中枢视图。
    pub fn update(
        &mut self,
        closed_moves: &[MoveView],
        pending: Option<&MoveView>,
        segs: &[SegView],
        zss: &[ZsView],
        level_id: i64,
    ) {
        // 1. 新封闭 move 的 div detect（一次性，永久固定）。
        while self.processed_closed < closed_moves.len() {
            let mv = &closed_moves[self.processed_closed];
            if let Some(d) = detect_one(segs, zss, mv, level_id) {
                self.closed_divs.push(d);
            }
            self.processed_closed += 1;
        }
        // 2. 增量拼接 full_divs = closed_divs + pending div。
        self.full_divs.truncate(self.prev_closed_div_len);
        self.full_divs
            .extend_from_slice(&self.closed_divs[self.prev_closed_div_len..]);
        self.prev_closed_div_len = self.closed_divs.len();
        if let Some(p) = pending {
            if let Some(d) = detect_one(segs, zss, p, level_id) {
                self.full_divs.push(d);
            }
        }
    }

    /// 当前全量背驰（逐位等价于 `divergences_from_moves_v1(.., None)`）。
    pub fn current(&self) -> &[Divergence] {
        &self.full_divs
    }

    /// 永久前缀长度（closed move 的 div 数；`current()[..stable_len]` 跨调用逐位不变）。
    /// IncrementalBsp 的 div frontier 推进边界来源。单调非减。
    pub fn stable_len(&self) -> usize {
        self.closed_divs.len()
    }
}
