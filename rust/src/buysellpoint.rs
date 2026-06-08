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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
fn find_next_seg_by_direction(segs: &[SegView], start: i64, direction: Direction) -> Option<i64> {
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
fn find_assoc_trend_move(moves: &[MoveView], center_idx: usize, n_zhongshus: usize) -> Option<usize> {
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
fn detect_type1(
    segs: &[SegView],
    zss: &[ZsView],
    moves: &[MoveView],
    divergences: &[Divergence],
    level_id: i64,
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
        if seg_idx >= 0 && (seg_idx as usize) < segs.len() {
            let seg = &segs[seg_idx as usize];
            price = if side == Side::Buy { seg.low } else { seg.high };
            bar_idx = seg.i1 as i64;
        }

        // candidate-fix: confirmed = 面积比 force_c/force_a ≤ 阈值（力竭确认）。
        let confirmed = div.force_a > 0.0 && div.force_c / div.force_a <= TYPE1_CONFIRM_RATIO;

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
) -> BuySellPoint {
    let assoc = lookup.find(seg_idx);
    let price = if side == Side::Buy { seg.low } else { seg.high };
    let confirmed = if side == Side::Buy {
        price >= t1.price // 回调不创新低
    } else {
        price <= t1.price // 反弹不创新高
    };
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
) -> BuySellPoint {
    let assoc = lookup.find(seg_idx);
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
                ));
            }
            crate::zhongshu::BreakDir::Down if pullback_seg.high < zs.zd => {
                result.push(make_type3_point(
                    zs, pullback, pullback_seg, Side::Sell, lookup, moves, level_id, confirmed,
                ));
            }
            _ => {}
        }
    }
    result
}

/// 2B+3B 重合检测。移植自 `_detect_overlap`。
fn detect_overlap(type2: &mut [BuySellPoint], type3: &mut [BuySellPoint]) {
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
) -> Vec<BuySellPoint> {
    let lookup = MoveLookup::build(moves, segs.len());

    let type1 = detect_type1(segs, zss, moves, divergences, level_id);
    let mut type2 = detect_type2(&type1, segs, &lookup, moves, level_id);
    let mut type3 = detect_type3(zss, zs_break, segs, &lookup, moves, level_id);
    detect_overlap(&mut type2, &mut type3);

    // sorted(type1 + type2 + type3, key=seg_idx)，稳定排序复刻 Python `sorted`。
    let mut all: Vec<BuySellPoint> = Vec::with_capacity(type1.len() + type2.len() + type3.len());
    all.extend(type1);
    all.extend(type2);
    all.extend(type3);
    all.sort_by_key(|bp| bp.seg_idx);
    all
}
