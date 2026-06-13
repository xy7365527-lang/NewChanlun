//! 买卖点 v1 — 逐位等价移植自 src/newchan/a_buysellpoint_v1.py
//!
//! type1（趋势背驰）/ type2（回调反弹）/ type3（中枢突破回试）三类买卖点检测 +
//! 2B+3B 重合标记。消费 segments + zhongshus + moves + divergences。
//!
//! ## 性能要点（架构报告指出的 O(N²) 瓶颈）
//! Python `_find_move_for_seg` 对每个 BSP 线性扫描 moves（O(BSP × moves)）。本模块改为
//! **区间表 O(1) 查询**：`seg_to_move[seg_idx] = move_idx`，构建 O(Σ move 跨度)，查询 O(1)。
//! Move 区间 [seg_start, seg_end] 不重叠，但为复刻 Python「列表序首个匹配」语义，填表时
//! **仅在槽位为空时写入**（首个 move 胜出）。
//!
//! ## 逐位等价要点
//! 1. confirmed 唯一浮点算术是 `force_c / force_a ≤ TYPE1_CONFIRM_RATIO`（单次除法，bit-exact）。
//! 2. price = seg.low/seg.high（直接取值，无算术）。
//! 3. 最终 `sorted(by seg_idx)` 用稳定排序（Rust `sort_by_key` 稳定），复刻 Python `sorted`。
//! 4. seg_idx / move_seg_start 用 i64 复刻 Python int（divergence.seg_c_end 可超出段数组）。

use crate::divergence::{DivDir, DivKind, Divergence, MoveView, SegView, ZsView};
use crate::stroke::Direction;

/// Type1 confirmed 阈值（背驰面积比力竭确认）。复刻 Python `TYPE1_CONFIRM_RATIO`。
const TYPE1_CONFIRM_RATIO: f64 = 0.9;

/// 买卖点类型。对应 Python Literal["type1", "type2", "type3"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BspKind {
    Type1,
    Type2,
    Type3,
}

impl BspKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BspKind::Type1 => "type1",
            BspKind::Type2 => "type2",
            BspKind::Type3 => "type3",
        }
    }
}

/// 买卖方向。对应 Python Literal["buy", "sell"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Buy => "buy",
            Side::Sell => "sell",
        }
    }
}

/// 2B+3B 重合标记。对应 Python overlaps_with: Literal["type2","type3"] | None。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    None,
    Type2,
    Type3,
}

impl Overlap {
    pub fn as_str(self) -> Option<&'static str> {
        match self {
            Overlap::None => None,
            Overlap::Type2 => Some("type2"),
            Overlap::Type3 => Some("type3"),
        }
    }
}

/// 买卖点实例。对应 Python `BuySellPoint` frozen dataclass。
#[derive(Debug, Clone, Copy)]
pub struct BuySellPoint {
    pub kind: BspKind,
    pub side: Side,
    pub level_id: i64,
    pub seg_idx: i64,
    pub move_seg_start: i64,
    pub divergence_key: Option<(usize, usize, usize)>,
    pub center_zd: f64,
    pub center_zg: f64,
    pub center_seg_start: Option<usize>,
    pub price: f64,
    pub bar_idx: i64,
    pub confirmed: bool,
    pub settled: bool,
    pub overlaps_with: Overlap,
}

/// 区间表 O(1) move 查询。替代 Python `_find_move_for_seg` 的线性扫描。
///
/// 仅暴露 settled 字段（BSP 唯一从 assoc_move 读取的是 .settled）。
struct MoveLookup {
    /// seg_idx → Some(move 在原列表中的索引)；空槽 = 无覆盖。
    seg_to_move: Vec<Option<usize>>,
}

impl MoveLookup {
    /// 构建区间表。覆盖 [0, max(段数, 各 move.seg_end+1))。
    fn build(moves: &[MoveView], n_segments: usize) -> Self {
        let mut cap = n_segments;
        for mv in moves {
            if mv.seg_end >= 0 {
                cap = cap.max(mv.seg_end as usize + 1);
            }
        }
        let mut seg_to_move: Vec<Option<usize>> = vec![None; cap];
        for (mi, mv) in moves.iter().enumerate() {
            if mv.seg_start < 0 {
                continue;
            }
            let lo = mv.seg_start as usize;
            let hi = if mv.seg_end >= 0 { mv.seg_end as usize } else { continue };
            for slot in seg_to_move.iter_mut().take(hi.min(cap - 1) + 1).skip(lo) {
                // 首个 move 胜出（复刻 Python 列表序首个匹配）
                if slot.is_none() {
                    *slot = Some(mi);
                }
            }
        }
        MoveLookup { seg_to_move }
    }

    /// 查询覆盖 seg_idx 的 move 索引。对应 `_find_move_for_seg`。
    fn find(&self, seg_idx: i64) -> Option<usize> {
        if seg_idx < 0 {
            return None;
        }
        let s = seg_idx as usize;
        if s < self.seg_to_move.len() {
            self.seg_to_move[s]
        } else {
            None
        }
    }
}

/// 从 start 开始找第一个指定方向的段索引。移植自 `_find_next_seg_by_direction`。
pub(crate) fn find_next_seg_by_direction(segs: &[SegView], start: i64, direction: Direction) -> Option<i64> {
    if start < 0 {
        return None;
    }
    let n = segs.len();
    for k in (start as usize)..n {
        if segs[k].direction == direction {
            return Some(k as i64);
        }
    }
    None
}

/// 找包含背驰中枢的趋势 Move 索引。移植自 `_find_assoc_trend_move`。
pub(crate) fn find_assoc_trend_move(moves: &[MoveView], center_idx: usize, n_zhongshus: usize) -> Option<usize> {
    if center_idx >= n_zhongshus {
        return None;
    }
    moves.iter().position(|m| {
        m.zs_start <= center_idx
            && center_idx <= m.zs_end
            && m.kind == crate::moves::MoveKind::Trend
    })
}

/// 第一类买卖点检测。移植自 `_detect_type1`。
/// `require_settled`：confirmed 合取背驰 C 段（次级别走势）已 settle——压制 pending
/// 生长期伪背驰（§3）。默认 false（在册口径，逐位等价 Python）。
fn detect_type1(
    segs: &[SegView],
    zss: &[ZsView],
    moves: &[MoveView],
    divergences: &[Divergence],
    level_id: i64,
    require_settled: bool,
) -> Vec<BuySellPoint> {
    let mut result: Vec<BuySellPoint> = Vec::new();
    for div in divergences {
        if div.kind != DivKind::Trend {
            continue;
        }
        let assoc_mi = match find_assoc_trend_move(moves, div.center_idx, zss.len()) {
            Some(mi) => mi,
            None => continue,
        };
        let assoc = &moves[assoc_mi];
        let zs = &zss[div.center_idx];
        let side = if div.direction == DivDir::Bottom {
            Side::Buy
        } else {
            Side::Sell
        };
        let seg_idx = div.seg_c_end;

        let mut price = 0.0;
        let mut bar_idx: i64 = 0;
        let mut seg_settled = false;
        if seg_idx >= 0 && (seg_idx as usize) < segs.len() {
            let seg = &segs[seg_idx as usize];
            price = if side == Side::Buy { seg.low } else { seg.high };
            bar_idx = seg.i1 as i64;
            seg_settled = seg.settled;
        }

        // candidate-fix: confirmed = 面积比 force_c/force_a ≤ 阈值（力竭确认）。
        // require_settled: 合取 C 段（次级别走势）已 settle（走势完成，非生长中）。
        let confirmed = div.force_a > 0.0
            && div.force_c / div.force_a <= TYPE1_CONFIRM_RATIO
            && (!require_settled || seg_settled);

        result.push(BuySellPoint {
            kind: BspKind::Type1,
            side,
            level_id,
            seg_idx,
            move_seg_start: assoc.seg_start,
            divergence_key: Some((
                div.center_idx,
                div.seg_c_start.max(0) as usize,
                div.seg_c_end.max(0) as usize,
            )),
            center_zd: zs.zd,
            center_zg: zs.zg,
            center_seg_start: Some(zs.seg_start),
            price,
            bar_idx,
            confirmed,
            settled: assoc.settled,
            overlaps_with: Overlap::None,
        });
    }
    result
}

/// 构造 Type2 买卖点。移植自 `_make_type2_point`。
#[allow(clippy::too_many_arguments)]
fn make_type2_point(
    t1: &BuySellPoint,
    seg_idx: i64,
    seg: &SegView,
    side: Side,
    lookup: &MoveLookup,
    moves: &[MoveView],
    level_id: i64,
    require_settled: bool,
) -> BuySellPoint {
    let assoc = lookup.find(seg_idx);
    let price = if side == Side::Buy { seg.low } else { seg.high };
    // require_settled: 合取回试段（次级别回试走势）已 settle（回试走势完成，非生长中）。
    let geom = if side == Side::Buy {
        price >= t1.price // 回调不创新低
    } else {
        price <= t1.price // 反弹不创新高
    };
    let confirmed = geom && (!require_settled || seg.settled);
    BuySellPoint {
        kind: BspKind::Type2,
        side,
        level_id,
        seg_idx,
        move_seg_start: t1.move_seg_start,
        divergence_key: t1.divergence_key,
        center_zd: t1.center_zd,
        center_zg: t1.center_zg,
        center_seg_start: t1.center_seg_start,
        price,
        bar_idx: seg.i1 as i64,
        confirmed,
        settled: assoc.map(|mi| moves[mi].settled).unwrap_or(false),
        overlaps_with: Overlap::None,
    }
}

/// 第二类买卖点检测。移植自 `_detect_type2`。
fn detect_type2(
    type1: &[BuySellPoint],
    segs: &[SegView],
    lookup: &MoveLookup,
    moves: &[MoveView],
    level_id: i64,
    require_settled: bool,
) -> Vec<BuySellPoint> {
    let mut result: Vec<BuySellPoint> = Vec::new();
    for t1 in type1 {
        match t1.side {
            Side::Buy => {
                let rebound = match find_next_seg_by_direction(segs, t1.seg_idx + 1, Direction::Up) {
                    Some(k) => k,
                    None => continue,
                };
                let callback =
                    match find_next_seg_by_direction(segs, rebound + 1, Direction::Down) {
                        Some(k) => k,
                        None => continue,
                    };
                result.push(make_type2_point(
                    t1,
                    callback,
                    &segs[callback as usize],
                    Side::Buy,
                    lookup,
                    moves,
                    level_id,
                    require_settled,
                ));
            }
            Side::Sell => {
                let pullback =
                    match find_next_seg_by_direction(segs, t1.seg_idx + 1, Direction::Down) {
                        Some(k) => k,
                        None => continue,
                    };
                let rebound_s =
                    match find_next_seg_by_direction(segs, pullback + 1, Direction::Up) {
                        Some(k) => k,
                        None => continue,
                    };
                result.push(make_type2_point(
                    t1,
                    rebound_s,
                    &segs[rebound_s as usize],
                    Side::Sell,
                    lookup,
                    moves,
                    level_id,
                    require_settled,
                ));
            }
        }
    }
    result
}

/// 构造 Type3 买卖点。移植自 `_make_type3_point`。
#[allow(clippy::too_many_arguments)]
fn make_type3_point(
    zs: &ZsView,
    seg_idx: i64,
    seg: &SegView,
    side: Side,
    lookup: &MoveLookup,
    moves: &[MoveView],
    level_id: i64,
    confirmed: bool,
    require_settled: bool,
) -> BuySellPoint {
    let assoc = lookup.find(seg_idx);
    // require_settled: 合取回抽段（次级别回抽走势）已 settle（回抽走势完成，非生长中）。
    let confirmed = confirmed && (!require_settled || seg.settled);
    BuySellPoint {
        kind: BspKind::Type3,
        side,
        level_id,
        seg_idx,
        move_seg_start: zs.seg_start as i64,
        divergence_key: None,
        center_zd: zs.zd,
        center_zg: zs.zg,
        center_seg_start: Some(zs.seg_start),
        price: if side == Side::Buy { seg.low } else { seg.high },
        bar_idx: seg.i1 as i64,
        confirmed,
        settled: assoc.map(|mi| moves[mi].settled).unwrap_or(false),
        overlaps_with: Overlap::None,
    }
}

/// 第三类买卖点检测。移植自 `_detect_type3`。
///
/// 入参 `zs_break`：每个中枢的 (settled, break_direction, break_seg)（type1/背驰用的
/// ZsView 不含 break 字段，type3 单独取）。
fn detect_type3(
    zss: &[ZsView],
    zs_break: &[(bool, crate::zhongshu::BreakDir, i64)],
    segs: &[SegView],
    lookup: &MoveLookup,
    moves: &[MoveView],
    level_id: i64,
    require_settled: bool,
) -> Vec<BuySellPoint> {
    let mut result: Vec<BuySellPoint> = Vec::new();
    for (zi, zs) in zss.iter().enumerate() {
        let (settled, break_dir, break_seg) = zs_break[zi];
        if !settled || break_dir == crate::zhongshu::BreakDir::None {
            continue;
        }
        if break_seg < 0 || break_seg >= segs.len() as i64 {
            continue;
        }
        let opposite = match break_dir {
            crate::zhongshu::BreakDir::Up => Direction::Down,
            crate::zhongshu::BreakDir::Down => Direction::Up,
            crate::zhongshu::BreakDir::None => continue,
        };
        let break_direction = match break_dir {
            crate::zhongshu::BreakDir::Up => Direction::Up,
            crate::zhongshu::BreakDir::Down => Direction::Down,
            crate::zhongshu::BreakDir::None => continue,
        };
        let pullback = match find_next_seg_by_direction(segs, break_seg + 1, opposite) {
            Some(k) => k,
            None => continue,
        };
        let pullback_seg = &segs[pullback as usize];

        // candidate-fix: confirmed = 回试段之后出现 break_direction 方向的延续段。
        let continuation = find_next_seg_by_direction(segs, pullback + 1, break_direction);
        let confirmed = continuation.is_some();

        match break_dir {
            crate::zhongshu::BreakDir::Up if pullback_seg.low > zs.zg => {
                result.push(make_type3_point(
                    zs, pullback, pullback_seg, Side::Buy, lookup, moves, level_id, confirmed,
                    require_settled,
                ));
            }
            crate::zhongshu::BreakDir::Down if pullback_seg.high < zs.zd => {
                result.push(make_type3_point(
                    zs, pullback, pullback_seg, Side::Sell, lookup, moves, level_id, confirmed,
                    require_settled,
                ));
            }
            _ => {}
        }
    }
    result
}

/// 2B+3B 重合检测。移植自 `_detect_overlap`。
pub(crate) fn detect_overlap(type2: &mut [BuySellPoint], type3: &mut [BuySellPoint]) {
    use std::collections::HashMap;
    // (seg_idx, side, level_id) → type3 index
    let mut t3_map: HashMap<(i64, Side, i64), usize> = HashMap::new();
    for (i, t3) in type3.iter().enumerate() {
        t3_map.insert((t3.seg_idx, t3.side, t3.level_id), i);
    }
    for t2 in type2.iter_mut() {
        let key = (t2.seg_idx, t2.side, t2.level_id);
        if let Some(&j) = t3_map.get(&key) {
            t2.overlaps_with = Overlap::Type3;
            type3[j].overlaps_with = Overlap::Type2;
        }
    }
}

/// 从某一递归层级的走势结构中识别所有买卖点。移植自 `buysellpoints_from_level`。
///
/// `zs_break`：与 `zss` 等长的 (settled, break_direction, break_seg) 列表（type3 用）。
pub fn buysellpoints_from_level(
    segs: &[SegView],
    zss: &[ZsView],
    zs_break: &[(bool, crate::zhongshu::BreakDir, i64)],
    moves: &[MoveView],
    divergences: &[Divergence],
    level_id: i64,
    require_settled: bool,
) -> Vec<BuySellPoint> {
    let lookup = MoveLookup::build(moves, segs.len());

    let type1 = detect_type1(segs, zss, moves, divergences, level_id, require_settled);
    let mut type2 = detect_type2(&type1, segs, &lookup, moves, level_id, require_settled);
    let mut type3 = detect_type3(zss, zs_break, segs, &lookup, moves, level_id, require_settled);
    detect_overlap(&mut type2, &mut type3);

    // sorted(type1 + type2 + type3, key=seg_idx)，稳定排序复刻 Python `sorted`。
    let mut all: Vec<BuySellPoint> = Vec::with_capacity(type1.len() + type2.len() + type3.len());
    all.extend(type1);
    all.extend(type2);
    all.extend(type3);
    all.sort_by_key(|bp| bp.seg_idx);
    all
}

// ════════════════════════════════════════════════════════════
// 单项构造自由函数（pub(crate)）——供 IncrementalBsp（笔级）+ IncrementalSegBsp（线段级）复用。
// lookup_find: seg_idx → assoc move 索引（线段级用窗口 lookup，笔级用增量 lookup）。
// ════════════════════════════════════════════════════════════

/// 单 div 构造 type1（复刻 `detect_type1` 循环体；trend div 由调用方过滤）。
pub(crate) fn build_type1_bsp(
    div: &Divergence,
    segs: &[SegView],
    zss: &[ZsView],
    moves: &[MoveView],
    level_id: i64,
    require_settled: bool,
) -> Option<BuySellPoint> {
    let assoc_mi = find_assoc_trend_move(moves, div.center_idx, zss.len())?;
    let assoc = &moves[assoc_mi];
    let zs = &zss[div.center_idx];
    let side = if div.direction == DivDir::Bottom { Side::Buy } else { Side::Sell };
    let seg_idx = div.seg_c_end;
    let mut price = 0.0;
    let mut bar_idx: i64 = 0;
    let mut seg_settled = false;
    if seg_idx >= 0 && (seg_idx as usize) < segs.len() {
        let seg = &segs[seg_idx as usize];
        price = if side == Side::Buy { seg.low } else { seg.high };
        bar_idx = seg.i1 as i64;
        seg_settled = seg.settled;
    }
    let confirmed = div.force_a > 0.0
        && div.force_c / div.force_a <= TYPE1_CONFIRM_RATIO
        && (!require_settled || seg_settled);
    Some(BuySellPoint {
        kind: BspKind::Type1,
        side,
        level_id,
        seg_idx,
        move_seg_start: assoc.seg_start,
        divergence_key: Some((
            div.center_idx,
            div.seg_c_start.max(0) as usize,
            div.seg_c_end.max(0) as usize,
        )),
        center_zd: zs.zd,
        center_zg: zs.zg,
        center_seg_start: Some(zs.seg_start),
        price,
        bar_idx,
        confirmed,
        settled: assoc.settled,
        overlaps_with: Overlap::None,
    })
}

/// 单 type1 构造 type2（复刻 `detect_type2` 循环体 + `make_type2_point`）。
pub(crate) fn build_type2_bsp(
    t1: &BuySellPoint,
    segs: &[SegView],
    moves: &[MoveView],
    level_id: i64,
    lookup_find: impl Fn(i64) -> Option<usize>,
    require_settled: bool,
) -> Option<BuySellPoint> {
    let (callback, side) = match t1.side {
        Side::Buy => {
            let rebound = find_next_seg_by_direction(segs, t1.seg_idx + 1, Direction::Up)?;
            let cb = find_next_seg_by_direction(segs, rebound + 1, Direction::Down)?;
            (cb, Side::Buy)
        }
        Side::Sell => {
            let pullback = find_next_seg_by_direction(segs, t1.seg_idx + 1, Direction::Down)?;
            let rb = find_next_seg_by_direction(segs, pullback + 1, Direction::Up)?;
            (rb, Side::Sell)
        }
    };
    let seg = &segs[callback as usize];
    let price = if side == Side::Buy { seg.low } else { seg.high };
    let geom = if side == Side::Buy { price >= t1.price } else { price <= t1.price };
    let confirmed = geom && (!require_settled || seg.settled);
    let assoc = lookup_find(callback);
    Some(BuySellPoint {
        kind: BspKind::Type2,
        side,
        level_id,
        seg_idx: callback,
        move_seg_start: t1.move_seg_start,
        divergence_key: t1.divergence_key,
        center_zd: t1.center_zd,
        center_zg: t1.center_zg,
        center_seg_start: t1.center_seg_start,
        price,
        bar_idx: seg.i1 as i64,
        confirmed,
        settled: assoc.map(|mi| moves[mi].settled).unwrap_or(false),
        overlaps_with: Overlap::None,
    })
}

/// 单中枢构造 type3（复刻 `detect_type3` 循环体 + `make_type3_point`）。
pub(crate) fn build_type3_bsp(
    zs: &ZsView,
    break_dir: crate::zhongshu::BreakDir,
    break_seg: i64,
    segs: &[SegView],
    moves: &[MoveView],
    level_id: i64,
    lookup_find: impl Fn(i64) -> Option<usize>,
    require_settled: bool,
) -> Option<BuySellPoint> {
    if break_seg < 0 || break_seg >= segs.len() as i64 {
        return None;
    }
    let (opposite, break_direction) = match break_dir {
        crate::zhongshu::BreakDir::Up => (Direction::Down, Direction::Up),
        crate::zhongshu::BreakDir::Down => (Direction::Up, Direction::Down),
        crate::zhongshu::BreakDir::None => return None,
    };
    let pullback = find_next_seg_by_direction(segs, break_seg + 1, opposite)?;
    let pullback_seg = &segs[pullback as usize];
    let continuation = find_next_seg_by_direction(segs, pullback + 1, break_direction);
    // require_settled: 合取回抽段（次级别回抽走势）已 settle。
    let confirmed = continuation.is_some() && (!require_settled || pullback_seg.settled);
    let side = match break_dir {
        crate::zhongshu::BreakDir::Up if pullback_seg.low > zs.zg => Side::Buy,
        crate::zhongshu::BreakDir::Down if pullback_seg.high < zs.zd => Side::Sell,
        _ => return None,
    };
    let assoc = lookup_find(pullback);
    Some(BuySellPoint {
        kind: BspKind::Type3,
        side,
        level_id,
        seg_idx: pullback,
        move_seg_start: zs.seg_start as i64,
        divergence_key: None,
        center_zd: zs.zd,
        center_zg: zs.zg,
        center_seg_start: Some(zs.seg_start),
        price: if side == Side::Buy { pullback_seg.low } else { pullback_seg.high },
        bar_idx: pullback_seg.i1 as i64,
        confirmed,
        settled: assoc.map(|mi| moves[mi].settled).unwrap_or(false),
        overlaps_with: Overlap::None,
    })
}

// ════════════════════════════════════════════════════════════
// 增量买卖点引擎 — 消除 `buysellpoints_from_level` 逐笔全量 O(S) 的 O(S²) 累积
// ════════════════════════════════════════════════════════════

/// 增量买卖点构造器 —— bit-exact 等价于逐前缀 `buysellpoints_from_level`，摊还 O(window)/笔。
///
/// ## 不变量（bit-exact 契约，differential test 守卫）
/// `current() == buysellpoints_from_level(segs, zss, zs_break, moves, divs, level_id)`。
///
/// ## 增量原理
/// 全量 detect 每次 O(S)（MoveLookup.build O(段数) + type3 遍历全中枢 O(Z)）。本引擎：
/// 1. **稳定前缀缓存**：`seg_idx < stable_anchor` 的 bsp 永久固定（其全部依赖段 ∈ confirmed
///    前缀且 `find_next` 延续段已确定）。`stable_anchor = n_segs - SAFE_WINDOW` 单调前进。
/// 2. **尾部重算**：每次只对 `seg_idx >= recompute_from` 的 bsp 重算（源 div/中枢/type1 的段
///    单调递增 → 用 frontier 取后缀，O(window)）。`recompute_from = stable_anchor - SRC_MARGIN`。
/// 3. **增量 seg→move 映射**：closed move 槽永久固定，pending 尾部每次重置重填 O(window)。
///
/// SAFE_WINDOW / SRC_MARGIN 为 `find_next` 跨度 + move 跨度的结构上界（非数据相关 magic）；
/// differential test 在真实数据逐前缀确认稳定前缀 bit-exact。
#[derive(Debug, Clone)]
pub struct IncrementalBsp {
    level_id: i64,
    /// 次级别走势 settle 合取门（编排者 2026-06-13）。bi 级笔全 confirmed ⟹ 恒真无效。
    require_settled: bool,
    /// seg_idx < stable_anchor 的 bsp（永久固定，按 seg_idx 升序）。
    stable_bsps: Vec<BuySellPoint>,
    /// 已 finalize 的段边界（单调前进）。
    stable_anchor: i64,
    /// 增量 seg→move 映射：closed move 槽永久，[perm_upto, n) 为 pending 瞬态区。
    seg_to_move: Vec<Option<usize>>,
    /// 永久填充到的段数（= 最后一个 closed move 的 seg_end + 1）。
    perm_upto: usize,
    /// 已永久填槽的 closed move 数。
    filled_moves: usize,
    /// div frontier：已跳过的 seg_c_end < src_from 的 div 前缀（单调推进，消除 O(D) 遍历）。
    div_scan_from: usize,
    /// 中枢 frontier：已跳过的 settled 且 break_seg < src_from 的中枢前缀（单调推进，消除 O(Z)）。
    zs_scan_from: usize,
    /// stable_bsps frontier：seg_idx < src_from 的前缀（type2 重算只需 seg_idx>=src_from 的源 type1）。
    stable_scan_from: usize,
    /// 当前全量 bsp（stable_bsps + 尾部重算，缓存供 current() 返回）。
    full_bsps: Vec<BuySellPoint>,
}

/// 尾部重算的安全窗口（输出侧）：seg_idx < n - SAFE_WINDOW 视为永久固定。
/// find_next（type2 跨≤2方向段、type3 跨≤2段）+ 单个 move 段跨度的保守上界。
const SAFE_WINDOW: i64 = 256;
/// 源侧回看裕度：源段 >= n - SAFE_WINDOW - SRC_MARGIN 的 type1/中枢/div 参与重算。
const SRC_MARGIN: i64 = 256;

impl IncrementalBsp {
    pub fn new(level_id: i64, require_settled: bool) -> Self {
        IncrementalBsp {
            level_id,
            require_settled,
            stable_bsps: Vec::new(),
            stable_anchor: 0,
            seg_to_move: Vec::new(),
            perm_upto: 0,
            filled_moves: 0,
            div_scan_from: 0,
            zs_scan_from: 0,
            stable_scan_from: 0,
            full_bsps: Vec::new(),
        }
    }

    /// 更新 seg→move 映射（增量）：新 closed move 永久填槽，pending 瞬态区重置重填。
    fn update_lookup(&mut self, moves: &[MoveView], n_segs: usize) {
        // closed move 数 = moves.len()-1（最后一个是 pending）；无 move 时 0。
        let n_closed = moves.len().saturating_sub(1);
        // 扩展槽数组到 n_segs。
        if self.seg_to_move.len() < n_segs {
            self.seg_to_move.resize(n_segs, None);
        }
        // 1. 永久填新 closed move 的槽（首个 move 胜出 ⟹ 只填空槽）。
        for mi in self.filled_moves..n_closed {
            let mv = &moves[mi];
            if mv.seg_start < 0 {
                continue;
            }
            let lo = mv.seg_start as usize;
            let hi = if mv.seg_end >= 0 {
                (mv.seg_end as usize).min(n_segs.saturating_sub(1))
            } else {
                continue;
            };
            for slot in self.seg_to_move.iter_mut().take(hi + 1).skip(lo) {
                if slot.is_none() {
                    *slot = Some(mi);
                }
            }
            // closed move 段连续分区 → perm_upto 推进到该 move seg_end+1。
            self.perm_upto = self.perm_upto.max(hi + 1);
        }
        self.filled_moves = n_closed;
        // 2. 瞬态区 [perm_upto, n_segs) 重置为 None。
        for slot in self.seg_to_move.iter_mut().take(n_segs).skip(self.perm_upto) {
            *slot = None;
        }
        // 3. pending move（最后一个）填瞬态区空槽（mi = n_closed，封闭后索引不变 → 稳定）。
        if !moves.is_empty() {
            let pend = &moves[moves.len() - 1];
            if pend.seg_start >= 0 {
                let lo = (pend.seg_start as usize).max(self.perm_upto);
                let hi = if pend.seg_end >= 0 {
                    (pend.seg_end as usize).min(n_segs.saturating_sub(1))
                } else {
                    n_segs.saturating_sub(1)
                };
                if lo <= hi {
                    for slot in self.seg_to_move.iter_mut().take(hi + 1).skip(lo) {
                        if slot.is_none() {
                            *slot = Some(n_closed);
                        }
                    }
                }
            }
        }
    }

    fn lookup_find(&self, seg_idx: i64) -> Option<usize> {
        if seg_idx < 0 {
            return None;
        }
        let s = seg_idx as usize;
        self.seg_to_move.get(s).copied().flatten()
    }

    /// 增量更新买卖点。bit-exact 等价于 `buysellpoints_from_level(...)`。
    ///
    /// `divs_stable_len`：divs 的永久前缀长度（`IncrementalDivergences::stable_len`，
    /// = closed move 的 div 数；末尾 pending div 易变）。frontier 只在永久前缀内推进。
    pub fn update(
        &mut self,
        segs: &[SegView],
        zss: &[ZsView],
        zs_break: &[(bool, crate::zhongshu::BreakDir, i64)],
        moves: &[MoveView],
        divs: &[Divergence],
        divs_stable_len: usize,
    ) {
        let n_segs = segs.len();
        self.update_lookup(moves, n_segs);

        // B2 anchor 门控：pending move 的趋势背驰 C 段极值可远落于段数组末端之前
        // （seg_c_end < n - SAFE_WINDOW），其 type1/type2 源自易变 div，不可 finalize。
        // pending div 全部 seg 引用 ≥ pending.seg_start（c_start ≥ move 首中枢 seg_start）
        // ⟹ anchor 以 pending.seg_start 为上限即保证 stable 区无易变源。
        // pending.seg_start 随 move 封闭单调前进 ⟹ anchor 单调性保持。
        // moves 为空 → floor=0（未来首 move 的 seg_start ≥ 0，anchor 不得先行越过）。
        let pending_floor = moves.last().map(|m| m.seg_start).unwrap_or(0);
        let new_anchor = (n_segs as i64 - SAFE_WINDOW)
            .min(pending_floor)
            .max(self.stable_anchor);
        let src_from = (new_anchor - SRC_MARGIN).max(0);

        // ── 尾部 type1：seg_c_end >= stable_anchor（**未 finalize** 的 type1；已 finalize 的
        //    [src_from, stable_anchor) type1 由 stable_t1 提供——二者按 stable_anchor 严格
        //    不相交，避免同一 type1 双重供源 → type2 重复。原边界为 src_from（与 stable_t1
        //    重叠）：B2 前 closed move 的 type1 结构性不存在（C 段恒空）故不可达；B2 解锁
        //    持久 type1 后即触发，与 IncrementalSegBsp 的 bar 350200 修复对齐。──
        // B2 后 seg_c_end 跨 move 不再全局单调（趋势 C 段可越入下一中枢覆盖区，与后继
        // 盘整 div 局部逆序）⟹ frontier 仅在永久前缀内推进，其后全部重扫
        // （内层 continue 过滤已 finalize 项）。
        while self.div_scan_from < divs_stable_len.min(divs.len())
            && divs[self.div_scan_from].seg_c_end < self.stable_anchor
        {
            self.div_scan_from += 1;
        }
        let mut tail_type1: Vec<BuySellPoint> = Vec::new();
        for div in &divs[self.div_scan_from..] {
            if div.kind != DivKind::Trend || div.seg_c_end < self.stable_anchor {
                continue;
            }
            if let Some(bp) = self.build_type1(div, segs, zss, moves) {
                tail_type1.push(bp);
            }
        }

        // ── 尾部 type2：源 type1.seg_idx >= src_from（含稳定缓存中尾部 type1）──
        // stable_bsps 升序 → frontier 跳过 seg_idx < src_from 前缀，只取后缀的 type1 作源。
        while self.stable_scan_from < self.stable_bsps.len()
            && self.stable_bsps[self.stable_scan_from].seg_idx < src_from
        {
            self.stable_scan_from += 1;
        }
        let stable_t1: Vec<BuySellPoint> = self.stable_bsps[self.stable_scan_from..]
            .iter()
            .filter(|bp| bp.kind == BspKind::Type1)
            .copied()
            .collect();
        let mut tail_type2: Vec<BuySellPoint> = Vec::new();
        for t1 in stable_t1.iter().chain(tail_type1.iter()) {
            if let Some(bp) = self.build_type2(t1, segs, moves) {
                tail_type2.push(bp);
            }
        }

        // ── 尾部 type3：break_seg >= src_from（frontier 跳过 settled 且 break_seg<src_from 的前缀）──
        // settled 中枢 break_seg 单调递增；遇 unsettled（break_seg=-1，仅末尾）即停，不越过。
        while self.zs_scan_from < zss.len() {
            let (settled, _, break_seg) = zs_break[self.zs_scan_from];
            if settled && break_seg < src_from {
                self.zs_scan_from += 1;
            } else {
                break;
            }
        }
        let mut tail_type3: Vec<BuySellPoint> = Vec::new();
        for zi in self.zs_scan_from..zss.len() {
            let (settled, break_dir, break_seg) = zs_break[zi];
            if !settled || break_dir == crate::zhongshu::BreakDir::None || break_seg < src_from {
                continue;
            }
            if let Some(bp) = self.build_type3(&zss[zi], break_dir, break_seg, segs, moves) {
                tail_type3.push(bp);
            }
        }

        // overlap（2B+3B）：尾部内 + 稳定 type2/type3 在尾部窗口的重合。
        detect_overlap(&mut tail_type2, &mut tail_type3);

        // 合并尾部，排序。
        let mut tail: Vec<BuySellPoint> =
            Vec::with_capacity(tail_type1.len() + tail_type2.len() + tail_type3.len());
        tail.extend(tail_type1);
        tail.extend(tail_type2);
        tail.extend(tail_type3);
        tail.sort_by_key(|bp| bp.seg_idx);

        // finalize：tail 含 [src_from..] 全部重算结果。其中：
        //   [src_from, stable_anchor) —— 已在 stable_bsps（重复，丢弃，由 lo 裁掉）
        //   [stable_anchor, new_anchor) —— 新 finalize，转入 stable_bsps
        //   [new_anchor, ∞) —— 仍易变，留 tail
        // SRC_MARGIN ≥ new_anchor-stable_anchor 保证 [stable_anchor, new_anchor) 全被 tail 覆盖。
        let lo = tail.partition_point(|bp| bp.seg_idx < self.stable_anchor);
        let hi = tail.partition_point(|bp| bp.seg_idx < new_anchor);
        for bp in &tail[lo..hi] {
            self.stable_bsps.push(*bp);
        }
        self.stable_anchor = new_anchor;

        // full = stable + tail[hi..]
        self.full_bsps.clear();
        self.full_bsps.extend_from_slice(&self.stable_bsps);
        self.full_bsps.extend_from_slice(&tail[hi..]);
    }

    /// 单 div 构造 type1（委托 build_type1_bsp）。
    fn build_type1(
        &self,
        div: &Divergence,
        segs: &[SegView],
        zss: &[ZsView],
        moves: &[MoveView],
    ) -> Option<BuySellPoint> {
        build_type1_bsp(div, segs, zss, moves, self.level_id, self.require_settled)
    }

    /// 单 type1 构造 type2（委托 build_type2_bsp，用增量 lookup）。
    fn build_type2(
        &self,
        t1: &BuySellPoint,
        segs: &[SegView],
        moves: &[MoveView],
    ) -> Option<BuySellPoint> {
        build_type2_bsp(t1, segs, moves, self.level_id, |s| self.lookup_find(s), self.require_settled)
    }

    /// 单中枢构造 type3（委托 build_type3_bsp，用增量 lookup）。
    fn build_type3(
        &self,
        zs: &ZsView,
        break_dir: crate::zhongshu::BreakDir,
        break_seg: i64,
        segs: &[SegView],
        moves: &[MoveView],
    ) -> Option<BuySellPoint> {
        build_type3_bsp(zs, break_dir, break_seg, segs, moves, self.level_id, |s| {
            self.lookup_find(s)
        }, self.require_settled)
    }

    /// 当前全量买卖点（逐位等价于 `buysellpoints_from_level`）。
    pub fn current(&self) -> &[BuySellPoint] {
        &self.full_bsps
    }

    /// 已 finalize 的段边界（seg_idx < stable_anchor 的 bsp 永久固定 + confirmed 终态确定）。
    pub fn stable_anchor(&self) -> i64 {
        self.stable_anchor
    }
}

// ════════════════════════════════════════════════════════════
// require_settled 合取门测试（编排者 2026-06-13：次级别走势完成判定）
// ════════════════════════════════════════════════════════════
#[cfg(test)]
mod require_settled_tests {
    use super::*;
    use crate::divergence::{DivDir, DivKind, Divergence};
    use crate::stroke::Direction;
    use crate::zhongshu::BreakDir;

    fn seg(direction: Direction, high: f64, low: f64, settled: bool) -> SegView {
        SegView { direction, high, low, i0: 0, i1: 0, settled }
    }

    fn trend_move() -> MoveView {
        MoveView {
            kind: crate::moves::MoveKind::Trend,
            direction: Direction::Down,
            seg_start: 0,
            seg_end: 3,
            zs_start: 0,
            zs_end: 0,
            zs_count: 2,
            settled: true,
        }
    }

    fn zs() -> ZsView {
        ZsView { zd: 100.0, zg: 110.0, seg_start: 0, seg_end: 2, settled: true }
    }

    /// type1：背驰 C 段（seg_c_end=3）未 settle → require_settled 翻 confirmed 为 false。
    #[test]
    fn type1_settled_gate_flips_confirmed() {
        let zss = vec![zs()];
        let moves = vec![trend_move()];
        // 4 段：0..2 + C 段 seg_c_end=3（底背驰买点，anchor=segs[3]）。
        let mut segs = vec![
            seg(Direction::Down, 120.0, 110.0, true),
            seg(Direction::Up, 118.0, 108.0, true),
            seg(Direction::Down, 112.0, 100.0, true),
            seg(Direction::Down, 105.0, 90.0, false), // C 段：生长中（未 settle）
        ];
        // 力竭背驰：force_c/force_a ≤ 0.9 → 基线 confirmed=true。
        let div = Divergence {
            kind: DivKind::Trend,
            direction: DivDir::Bottom,
            level_id: 1,
            seg_a_start: 0,
            seg_a_end: 0,
            seg_c_start: 3,
            seg_c_end: 3,
            center_idx: 0,
            force_a: 10.0,
            force_c: 5.0,
            confirmed: true,
            dif_peak_a: 0.0,
            dif_peak_c: 0.0,
            hist_peak_a: 0.0,
            hist_peak_c: 0.0,
        };
        let base = build_type1_bsp(&div, &segs, &zss, &moves, 1, false).unwrap();
        assert!(base.confirmed, "基线（require_settled=false）应 confirmed=true");
        let gated = build_type1_bsp(&div, &segs, &zss, &moves, 1, true).unwrap();
        assert!(!gated.confirmed, "C 段未 settle ⟹ require_settled 应 confirmed=false");
        // C 段 settle 后：门放行。
        segs[3].settled = true;
        let gated2 = build_type1_bsp(&div, &segs, &zss, &moves, 1, true).unwrap();
        assert!(gated2.confirmed, "C 段 settle 后 require_settled 应放行 confirmed=true");
    }

    /// type3：回抽段未 settle → require_settled 翻 confirmed 为 false。
    #[test]
    fn type3_settled_gate_flips_confirmed() {
        let z = zs();
        // 0..2 中枢；3 向上突破；4 向下回抽(low>zg=110 → 三买)；5 向上延续(confirmed 基线)。
        let mut segs = vec![
            seg(Direction::Down, 120.0, 100.0, true),
            seg(Direction::Up, 118.0, 105.0, true),
            seg(Direction::Down, 112.0, 102.0, true),
            seg(Direction::Up, 130.0, 115.0, true), // break up, break_seg=3
            seg(Direction::Down, 125.0, 112.0, false), // 回抽 low=112>zg=110，生长中
            seg(Direction::Up, 135.0, 120.0, true),    // 延续 → confirmed 基线 true
        ];
        let lk = |_s: i64| None;
        let base =
            build_type3_bsp(&z, BreakDir::Up, 3, &segs, &[], 1, lk, false).unwrap();
        assert_eq!(base.side, Side::Buy);
        assert!(base.confirmed, "基线应 confirmed=true（有延续段）");
        let gated =
            build_type3_bsp(&z, BreakDir::Up, 3, &segs, &[], 1, lk, true).unwrap();
        assert!(!gated.confirmed, "回抽段未 settle ⟹ require_settled 应 confirmed=false");
        segs[4].settled = true;
        let gated2 =
            build_type3_bsp(&z, BreakDir::Up, 3, &segs, &[], 1, lk, true).unwrap();
        assert!(gated2.confirmed, "回抽段 settle 后 require_settled 应放行");
    }
}
