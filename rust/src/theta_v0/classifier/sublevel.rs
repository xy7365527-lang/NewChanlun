//! 递归组装层第二类（B2/S2）提取与次级别背驰力度（`descend` 取回的次级别走势序列内
//! 识别第二类走势结构，§10.2 买卖点定律一），含 area-memo 命中的段面积原语。

use super::tower_cache::AreaCache;
use super::*;

/// 递归组装层第二类提取（对一级的每个上级走势 `RMove::Compose` 产 B2/S2）。
///
/// ★#53 接入点（still-MISSING-塔解除）：对本级每个上级走势 `LeveledMove`（`RMove::Compose`），
/// 双侧（Long/Short）调 `signal::extract_second_signals`——从 `descend parent` 取回的次级别走势
/// 序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高，§10.2 买卖点定律一）。产出的 B2/S2
/// 端点零改动接入生产路径。
///
/// 三个闭包参数的真实接入（非占位）：
/// - `c1`（次级别中枢）：`RMove::Compose.centers` 的首个中枢（窗口三段区间重叠真派生，B 口径核心
///   区间）——`find_second_type_structure` 用它判次级别第一类离开是否破中枢。
/// - `divergence_of`（MACD 背驰）：次级别走势的 source_index 区间 → `hist` 面积，相对**前一同向次
///   级别走势**面积严格变小（reference:34 背驰，`divergence.rs` 真算 L1）。
/// - `index_of`（坐标）：从坐标侧车 `subs`（携 source_index 的 `LeveledMove`）按结构身份查回原始
///   K 序（`index_of_in`）——B2/S2 的 `source_index` 真坐标（still-MISSING-坐标解除）。
///
/// ★诚实 still-MISSING（背驰力度引擎配对，no-声明膨胀）：`divergence_of` 对次级别走势的「前一同向
/// 走势」配对用**序列序最近同向前驱**（与 signal.rs `extract_first_for_center` 同口径）——次级别
/// 走势的 close 区间由 source_index 坐标定位到 `hist`。L1 管线正确性（MACD 面积比较确定），**不**是
/// 「背驰预测在真实行情有效」（L2/L3，不在本层）。
pub(super) fn extract_second_for_level(
    upper_moves: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
) -> Vec<BspPoint> {
    let mut points = Vec::new();
    // 单次全量调用：无跨 bar 复用需求，本地缓存仅消同一调用内的重复 (start,end)（若有），
    // `stable_len=hist.len()` 视全 hist 为稳定（一次性快照，调用期间不会被改写）。
    let area_cache = RefCell::new(AreaCache::new());
    let stable_len = hist.len();
    for parent in upper_moves {
        second_for_parent(parent, hist, close_src, &area_cache, stable_len, &mut points);
    }
    points
}

/// 单个上级走势 `parent` 的第二类 B2/S2 端点（[`extract_second_for_level`] 的 per-parent 主体）。
///
/// ★纯函数于 `parent`：只依赖 `parent`（`c1`=Compose 首中枢、subs=坐标侧车）+ 全局 hist/close_src，
/// **不依赖其它 parent**。这是 07b frontier 门控（[`extract_second_resume`]）的正确性基础——confirmed
/// 前缀 parent 的 B2 跨 bar 不变（源区间落稳定前缀，hist 前缀 append-only 稳定）⟹ 可缓存。
pub(super) fn second_for_parent(
    parent: &LeveledMove,
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
    out: &mut Vec<BspPoint>,
) {
    // 次级别中枢（RMove::Compose.centers 首个，窗口真派生 B 口径核心区间）。
    let c1 = match &parent.rmove {
        descend::RMove::Compose { centers, .. } => match centers.first() {
            Some(c) => *c,
            None => return, // 无中枢载荷 ⟹ 跳过（compose_level 必带中枢，防御性）。
        },
        // L0 线段（递归底）不会出现在 upper_moves（compose_level 只产 Compose），防御性跳过。
        descend::RMove::Segment { .. } => return,
    };
    // 坐标侧车：构成 parent 的次级别 LeveledMove 序列（与 descend parent 同序同长）。
    let subs = descend_leveled(parent);
    // 双侧识别第二类结构（B2=Long / S2=Short），各产至多一个端点。
    for side in [Side::Long, Side::Short] {
        let pts = signal::extract_second_signals(
            &parent.rmove,
            side,
            &c1,
            // 背驰：次级别走势 source_index 区间 → hist 面积，相对前一同向次级别走势严格变小。
            |m| sublevel_diverges(m, &subs[..], hist, close_src, area_cache, stable_len),
            // 坐标：从侧车按结构身份查回次级别走势的原始 K 序（end_index）。
            |m| index_of_in(&subs[..], m),
        );
        out.extend(pts);
    }
}

/// 07b frontier 门控（A 泳道 resume 家族，与 A3/07c 同族）：增量热路径的 [`extract_second_for_level`]
/// 每 memo-miss 全塔重扫 O(U)、跨 N bar 累积 O(U²)（profile 坐实 CL 1M 修前 3990ms、修后 1026ms，74%↓）。
/// 每个 parent 的 B2 只依赖该 parent（[`second_for_parent`] 纯函数于 parent）——confirmed 前缀 parent
/// （`upper_moves[..prefix_count]`，源区间落稳定前缀 + hist 前缀 append-only 稳定）的 B2 跨 bar 不变，
/// 可缓存。故**跳过 confirmed 前缀 parent 的重复背驰扫描**（`sublevel_diverges` 的 O(range) MACD 面积
/// 累加），只对 frontier tail `[prefix_count..]` 每 bar 重算，前缀 B2 一生一算。
///
/// ★area-memo 已接入（`d516aa42ea`，[`cached_segment_area`]）：MACD 面积经 `(start,end)→f64` 冻结
/// 缓存，`sublevel_diverges` 内 `07b_area` 实测 ~46ns/call（10.2M call@BTC-1M）= cache-hit 主导，
/// 面积累加已非瓶颈。门控（本函数）+ area-memo 两处均已落地。
///
/// ★残余 O(n²) 根因订正（on2-sweep 直测坐实，BTC-1M）：**不是** frontier parent 的 area 累加，而是
/// **cascade 触发的前缀重扫**。07b miss 的 frontier tail 分裂两支——纯 append miss（`07b_miss_append_tail`
/// sum=28570，avg=2.87，门控生效尾极短）vs cascade miss（`07b_miss_cascade_tail` sum=2015978=98.6%，
/// avg=97.5，max=2095）。cascade_reset 定义性清空 `cached_second`（前缀 B2 缓存失效，见 line ~1192），
/// 整个前缀被重扫 ⟹ cascade 频率 × 前缀长度 = O(n²)。cascade 频率由 `recursive_tower` 域的 frontier
/// 重排决定（H5/H9 已 NO-SHIP：cascade 定义性清前缀，不可在本文件域降阶），非 `second_for_parent`/
/// `sublevel_diverges` 的可优化项。per-call 常数因子（`position` 递归 eq ~14%、`rmove_direction`
/// 递归 `.hi()` ~15%）均 subs.len()≤8 有界，改写只削常数不改指数（收益极低，不 ship）。
///
/// ★bit-exact 铁律：返回值逐字段 == [`extract_second_for_level`]（parent 序拼接：前缀 B2 + tail B2
/// = 全 parent 序，与全量重扫同序同集）。debug/test 护栏逐 bar 对拍全量重算锁定。
///
/// 前提（`prefix_count` = confirmed 前缀数，由主循环 pop 后 `lc.upper_moves.len()` 给出）：
/// `upper_moves[..prefix_count]` 跨 bar immutable（anc.pdf §16），故其 B2 一旦算出即永久稳定。
pub(super) fn extract_second_resume(
    cached: &mut Vec<BspPoint>,
    cached_count: &mut usize,
    upper_moves: &[LeveledMove],
    prefix_count: usize,
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
) -> Vec<BspPoint> {
    // 单调性守卫（§16：confirmed 前缀单调非降 ⟹ 正常永不触发；cascade 已在别处 clear 缓存）。若违反
    // ⟹ 缓存越过 confirmed 边界（曾判 confirmed 的 parent 又变 frontier 可变）⟹ 保守全量重算重置缓存。
    if *cached_count > prefix_count {
        cached.clear();
        *cached_count = 0;
    }
    // 推进：新晋 confirmed 的 parent [cached_count..prefix_count] 的 B2 一次性算入缓存（一生一算）。
    for parent in &upper_moves[*cached_count..prefix_count] {
        second_for_parent(parent, hist, close_src, area_cache, stable_len, cached);
    }
    *cached_count = prefix_count;
    // 结果 = confirmed 前缀 B2（缓存 clone）+ frontier tail B2（每 bar 重算，tail 小）。
    let mut out = cached.clone();
    for parent in &upper_moves[prefix_count..] {
        second_for_parent(parent, hist, close_src, area_cache, stable_len, &mut out);
    }
    debug_assert!(
        out == extract_second_for_level(upper_moves, hist, close_src),
        "07b frontier 门控破裂：门控输出 != 全量重扫（前缀 immutable/hist 前缀稳定不变式被违反）"
    );
    out
}

/// 次级别走势的 MACD 背驰判定（reference:34，`divergence.rs` 真算 L1）。
///
/// 给定次级别走势 `m`（descend 取回的 `RMove`）+ 坐标侧车 `subs`：定位 `m` 在 `subs` 中的位置，
/// 取其 source_index 区间 → `hist` 面积，相对**序列序最近同向次级别前驱走势**面积严格变小 ⟹ 背驰。
/// 同向 = 走势的 direction 相同（`RMove::Segment.direction`；`Compose` 走势取外缘趋势方向占位）。
///
/// ★诚实 still-MISSING（背驰力度引擎）：无前同向走势（`m` 是序列首个该向走势）⟹ 无背驰对照
/// ⟹ false（与 signal.rs `extract_first_for_center` 同口径——第一类是趋势末段必有前同向段）。
/// 无法定位 source_index 区间到 `hist`（坐标越界）⟹ false（不冒充背驰）。
pub(super) fn sublevel_diverges(
    m: &descend::RMove,
    subs: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
) -> bool {
    // 定位 m 在 subs 中的位置（结构身份匹配）。
    let Some(idx) = subs.iter().position(|x| &x.rmove == m) else {
        return false; // m 不在 subs（防御性）⟹ 无坐标 ⟹ 非背驰。
    };
    let curr = &subs[idx];
    let curr_dir = rmove_direction(&curr.rmove);
    // 序列序最近同向前驱走势（reference:34「末段相对前同向段」的确定配对）。
    let Some(prev) = subs[..idx].iter().rev().find(|x| rmove_direction(&x.rmove) == curr_dir) else {
        return false; // 无前同向走势 ⟹ 无背驰对照 ⟹ 非第一类（趋势末段必有前同向段）。
    };
    // 两走势 source_index 区间 → hist 面积比较（curr < prev ⟹ 背驰，divergence.rs 真算）。
    let (Some(curr_seg), Some(prev_seg)) = (
        map_src_to_close_idx(close_src, curr.start_index, curr.end_index),
        map_src_to_close_idx(close_src, prev.start_index, prev.end_index),
    ) else {
        return false; // 区间越界/空 ⟹ 无面积 ⟹ 不冒充背驰。
    };
    // B3 #4 area-memo：面积经 (start,end)→f64 冻结缓存（[`cached_segment_area`]），逐字段
    // == `divergence::segments_diverge`（同一 `segment_macd_area` 结果，仅省重复求和）。
    let prev_area = cached_segment_area(area_cache, hist, stable_len, prev_seg.0, prev_seg.1);
    let curr_area = cached_segment_area(area_cache, hist, stable_len, curr_seg.0, curr_seg.1);
    divergence::is_divergence(prev_area, curr_area)
}

/// B3 #4 area-memo：`(start,end)`→`segment_macd_area` 冻结缓存查询/写入（[`AreaCache`] 文档）。
///
/// 只有 `end < stable_len`（区间落在本 bar 已确认的 hist 前缀内）才读写缓存——`end >= stable_len`
/// 触及仍可能被下一 bar 改写的 unstable tail（`compute_macd_hist_incremental` 每 bar 末元素语义），
/// 缓存这类值会在下 bar 值变化后返回过期错值，故每次现算、绝不写入缓存（bit-exact 铁律）。
///
/// bit-exact：返回值逐位 == `divergence::segment_macd_area(hist, start, end)`——本函数只做**结果**
/// 记忆化（首次算出后原样存取），不做前缀和差分（B3 报告明确禁止：浮点求和顺序改变可能破 bit-exact）。
pub(super) fn cached_segment_area(
    cache: &RefCell<AreaCache>,
    hist: &[f64],
    stable_len: usize,
    start: usize,
    end: usize,
) -> f64 {
    if end < stable_len {
        if let Some(&area) = cache.borrow().get(&(start, end)) {
            return area;
        }
        let area = divergence::segment_macd_area(hist, start, end);
        cache.borrow_mut().insert((start, end), area);
        area
    } else {
        divergence::segment_macd_area(hist, start, end)
    }
}

/// 走势方向（`RMove::Segment` 直接取 direction；`Compose` 取外缘趋势方向占位——首子升=Up）。
///
/// ★诚实有效域：`Compose` 走势的方向是**外缘占位**（subs 区间聚合趋势），用于背驰「同向段」配对的
/// 序列序判定。不冒充 §6.1 意义的线段方向交替（中枢检测用几何路径，不读方向，见 center.rs）。
pub(super) fn rmove_direction(m: &descend::RMove) -> Direction {
    match m {
        descend::RMove::Segment { direction, .. } => *direction,
        // Compose 走势：外缘下沿 vs 上沿——hi 偏离 lo 多者为趋势向（占位，背驰同向配对用）。
        // subs 首尾区间趋势：末子 hi >= 首子 hi ⟹ Up（外缘上移），否则 Down。
        descend::RMove::Compose { subs, .. } => {
            match (subs.first(), subs.last()) {
                (Some(f), Some(l)) if l.hi() >= f.hi() => Direction::Up,
                (Some(_), Some(_)) => Direction::Down,
                _ => Direction::Up, // 空 subs ⟹ 缺省 Up（防御性）。
            }
        }
    }
}
