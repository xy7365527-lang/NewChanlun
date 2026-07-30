//! GUARD-ROLE: toplevel-loose-files——名分：现役（详见 `lib.rs` 头部 GUARD-ROLE 块，#764 C7-E5 核定；零删除/零移入 legacy/）。
//!
//! level.rs — 泛化中枢/走势（级别递归核）逐位等价移植自 src/newchan/a_zhongshu_level.py
//!
//! 递归层的两个计算核：
//! - `zhongshu_from_components`：从 MoveProtocol 组件序列构造 `LevelZhongshu`
//!   （组件 = 下级 settled Move，high=zg_max||high，low=zd_min||low，
//!   component_idx = 在 settled 列表中的位置）。
//! - `moves_from_level_zhongshus`：从 `LevelZhongshu` 贪心分组构造 `Move`。
//!
//! ## 与 level-1（zhongshu.rs / moves.rs）的差异
//! 1. 输出中枢类型为 `LevelZhongshu`（comp_start/comp_end 同时充当位置与锚，
//!    因 component_idx == 在 settled 列表中的位置）。
//! 2. `moves_from_level_zhongshus` 的 seg_end 语义**与 level-1 对齐**（修复A，
//!    engine_bsp_gap_diagnosis §1.3/§4）：非末组 seg_end = 下一组首中枢 comp_start - 1，
//!    末组 = num_components - 1。曾直接取 last_zs.comp_end，使趋势背驰 C 段
//!    （离开最后中枢后的组件）恒空（c_start = comp_end+1 > c_end = comp_end）
//!    → 递归层 type1/type2 结构性不存在（OKLO 447K 16/16 实证）。
//! 3. Move.first_seg_s0 = first_zs.comp_start，last_seg_s1 = last_zs.comp_end
//!    （level-1 用 first_zs.first_seg_s0 / last_zs.last_seg_s1；锚点不随 seg_end 扩展）。
//!
//! ## 逐位等价要点
//! 与 zhongshu.rs / moves.rs 同：中枢/走势构造**无浮点算术**，只有比较 + 三元
//! max/min 选择（顺序约简与 Python `max`/`min` 同序），故 zd/zg/gg/dd/high/low/
//! zg_max/zd_min bit-exact。break_comp、续进锚 `max(j-2, end_offset)` 用 i64 复刻
//! Python int（避免 usize 下溢）。

use crate::moves::{Move, MoveKind};
use crate::stroke::Direction;
use crate::zhongshu::BreakDir;

/// 泛化中枢：可由任意级别的组件构造。对应 Python `LevelZhongshu` frozen dataclass。
#[derive(Debug, Clone, Copy)]
pub struct LevelZhongshu {
    pub zd: f64,
    pub zg: f64,
    pub comp_start: usize,
    pub comp_end: usize,
    pub comp_count: usize,
    pub settled: bool,
    /// 突破组件 component_idx；-1 = 未闭合（用 i64 复刻 Python 默认 -1）。
    pub break_comp: i64,
    pub break_direction: BreakDir,
    pub gg: f64,
    pub dd: f64,
    pub level_id: i64,
}

/// 中枢组件视图 — `zhongshu_from_components` 实际访问的字段子集。
///
/// 对应 `MoveAsComponent`：high = zg_max||high，low = zd_min||low，
/// component_idx = 在 settled 列表中的位置。调用方负责仅传 completed（settled）组件。
#[derive(Debug, Clone, Copy)]
pub struct CompView {
    pub high: f64,
    pub low: f64,
    pub component_idx: usize,
}

/// 尝试延伸中枢，返回 (end_offset, gg, dd)。移植自 `_try_extend_zhongshu`（start=i+2）。
fn try_extend(comps: &[CompView], start: usize, n: usize, zd: f64, zg: f64) -> (usize, f64, f64) {
    // gg/dd 初值覆盖 start-2, start-1, start（即初始三组件 i, i+1, i+2）。
    let mut gg = comps[start - 2]
        .high
        .max(comps[start - 1].high)
        .max(comps[start].high);
    let mut dd = comps[start - 2]
        .low
        .min(comps[start - 1].low)
        .min(comps[start].low);
    let mut end_offset = start;

    let mut j = start + 1;
    while j < n {
        let cj = &comps[j];
        if cj.high >= zd && cj.low <= zg {
            end_offset = j;
            gg = gg.max(cj.high);
            dd = dd.min(cj.low);
            j += 1;
        } else {
            break;
        }
    }
    (end_offset, gg, dd)
}

/// 判断突破方向。移植自 `_determine_break_direction` 的方向部分。
fn break_direction(breaker: &CompView, zg: f64, zd: f64) -> BreakDir {
    if breaker.low > zg {
        return BreakDir::Up;
    }
    if breaker.high < zd {
        return BreakDir::Down;
    }
    if breaker.high > zg {
        BreakDir::Up
    } else {
        BreakDir::Down
    }
}

/// 从 MoveProtocol 组件序列计算中枢。移植自 `zhongshu_from_components`。
///
/// `comps`：已过滤的 completed 组件（调用方保证）。`out_level_id`：输出中枢的
/// level_id（= completed[0].level_id + 1 = 本递归级别 ID）。
pub fn zhongshu_from_components(comps: &[CompView], out_level_id: i64) -> Vec<LevelZhongshu> {
    let n = comps.len();
    if n < 3 {
        return Vec::new();
    }

    let mut result: Vec<LevelZhongshu> = Vec::new();
    let mut i: usize = 0;

    while i + 2 < n {
        let (c1, c2, c3) = (&comps[i], &comps[i + 1], &comps[i + 2]);
        let zd = c1.low.max(c2.low).max(c3.low);
        let zg = c1.high.min(c2.high).min(c3.high);

        if zg <= zd {
            i += 1;
            continue;
        }

        let (end_offset, gg, dd) = try_extend(comps, i + 2, n, zd, zg);
        let j = end_offset + 1;
        let settled = j < n;
        let break_comp_idx: i64 = if settled {
            comps[j].component_idx as i64
        } else {
            -1
        };
        let break_dir = if settled {
            break_direction(&comps[j], zg, zd)
        } else {
            BreakDir::None
        };

        result.push(LevelZhongshu {
            zd,
            zg,
            comp_start: comps[i].component_idx,
            comp_end: comps[end_offset].component_idx,
            comp_count: end_offset - i + 1,
            settled,
            break_comp: break_comp_idx,
            break_direction: break_dir,
            gg,
            dd,
            level_id: out_level_id,
        });

        if settled {
            // i = max(j - 2, end_offset)（i64 复刻 Python int）
            let next = ((j as i64) - 2).max(end_offset as i64);
            i = next as usize;
        } else {
            break;
        }
    }

    result
}

/// 分组方向状态。对应 Python current_dir: "" / "up" / "down"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupDir {
    None,
    Up,
    Down,
}

/// 后枢 ZD 严格高于 前枢 ZG → 上涨延续。移植自 `_is_ascending`。
#[inline]
fn is_ascending(c1: &LevelZhongshu, c2: &LevelZhongshu) -> bool {
    c2.zd > c1.zg
}

/// 后枢 ZG 严格低于 前枢 ZD → 下跌延续。移植自 `_is_descending`。
#[inline]
fn is_descending(c1: &LevelZhongshu, c2: &LevelZhongshu) -> bool {
    c2.zg < c1.zd
}

/// 贪心分组：同向中枢归入同一 group。移植自 `_greedy_group_zhongshus`。
fn greedy_group(settled_zs: &[LevelZhongshu]) -> Vec<(Vec<usize>, GroupDir)> {
    let mut groups: Vec<(Vec<usize>, GroupDir)> = Vec::new();
    let mut current_offsets: Vec<usize> = vec![0];
    let mut current_dir = GroupDir::None;

    for k in 1..settled_zs.len() {
        let prev_zs = &settled_zs[k - 1];
        let curr_zs = &settled_zs[k];

        if is_ascending(prev_zs, curr_zs)
            && (current_dir == GroupDir::None || current_dir == GroupDir::Up)
        {
            current_offsets.push(k);
            current_dir = GroupDir::Up;
        } else if is_descending(prev_zs, curr_zs)
            && (current_dir == GroupDir::None || current_dir == GroupDir::Down)
        {
            current_offsets.push(k);
            current_dir = GroupDir::Down;
        } else {
            groups.push((current_offsets, current_dir));
            current_offsets = vec![k];
            current_dir = GroupDir::None;
        }
    }

    groups.push((current_offsets, current_dir));
    groups
}

/// 将一个 group 转换为 Move。移植自 `_group_to_move`。
///
/// seg_end 扩展语义与 level-1 `moves::group_to_move` 逐字一致（修复A）：
/// 非末组 = next_seg_start - 1；末组 = num_components - 1（>0 时）。
fn group_to_move(
    offsets: &[usize],
    direction: GroupDir,
    settled_zs: &[LevelZhongshu],
    settled_indices: &[usize],
    next_seg_start: Option<i64>,
    num_components: Option<usize>,
) -> Move {
    let first_zs = &settled_zs[offsets[0]];
    let last_zs = &settled_zs[offsets[offsets.len() - 1]];
    let zs_count = offsets.len();
    let zs_start = settled_indices[offsets[0]];
    let zs_end = settled_indices[offsets[offsets.len() - 1]];

    let mut base_seg_end: i64 = last_zs.comp_end as i64;
    if let Some(nss) = next_seg_start {
        base_seg_end = nss - 1;
    } else if let Some(nc) = num_components {
        if nc > 0 {
            base_seg_end = nc as i64 - 1;
        }
    }

    let (kind, move_dir) = if zs_count >= 2 {
        let d = match direction {
            GroupDir::Up => Direction::Up,
            GroupDir::Down => Direction::Down,
            GroupDir::None => Direction::Up, // 不可达（trend 必有方向）
        };
        (MoveKind::Trend, d)
    } else {
        let d = match first_zs.break_direction {
            BreakDir::Up => Direction::Up,
            BreakDir::Down => Direction::Down,
            BreakDir::None => Direction::Up, // 不可达（settled 中枢有突破方向）
        };
        (MoveKind::Consolidation, d)
    };

    // group_centers = [settled_zs[o] for o in offsets]，max/min 顺序约简
    let mut high = settled_zs[offsets[0]].gg;
    let mut low = settled_zs[offsets[0]].dd;
    let mut zg_max = settled_zs[offsets[0]].zg;
    let mut zd_min = settled_zs[offsets[0]].zd;
    for &o in &offsets[1..] {
        let z = &settled_zs[o];
        high = high.max(z.gg);
        low = low.min(z.dd);
        zg_max = zg_max.max(z.zg);
        zd_min = zd_min.min(z.zd);
    }

    Move {
        kind,
        direction: move_dir,
        seg_start: first_zs.comp_start as i64,
        seg_end: base_seg_end,
        zs_start,
        zs_end,
        zs_count,
        settled: true,
        high,
        low,
        first_seg_s0: first_zs.comp_start,
        last_seg_s1: last_zs.comp_end,
        zg_max,
        zd_min,
        persistence: 0.0,
    }
}

/// 从 LevelZhongshu 列表构造 Move（贪心分组，只处理 settled 中枢）。
/// 移植自 `moves_from_level_zhongshus`。末组 settled 强制置 False。
///
/// `num_components`：组件总数（completed 组件序列长度）——末组 seg_end 扩展用，
/// 与 level-1 `moves_from_zhongshus` 的 num_segments 同义（修复A）。
pub fn moves_from_level_zhongshus(
    zhongshus: &[LevelZhongshu],
    num_components: Option<usize>,
) -> Vec<Move> {
    let mut settled_indices: Vec<usize> = Vec::new();
    let mut settled_zs: Vec<LevelZhongshu> = Vec::new();
    for (idx, zs) in zhongshus.iter().enumerate() {
        if zs.settled {
            settled_indices.push(idx);
            settled_zs.push(*zs);
        }
    }
    if settled_zs.is_empty() {
        return Vec::new();
    }

    let groups = greedy_group(&settled_zs);
    let n_groups = groups.len();
    let mut result: Vec<Move> = Vec::with_capacity(n_groups);
    for (g_idx, (offsets, direction)) in groups.iter().enumerate() {
        let next_seg_start: Option<i64> = if g_idx < n_groups - 1 {
            let next_first_offset = groups[g_idx + 1].0[0];
            Some(settled_zs[next_first_offset].comp_start as i64)
        } else {
            None
        };
        result.push(group_to_move(
            offsets,
            *direction,
            &settled_zs,
            &settled_indices,
            next_seg_start,
            num_components,
        ));
    }

    if let Some(last) = result.last_mut() {
        last.settled = false;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(high: f64, low: f64, idx: usize) -> CompView {
        CompView {
            high,
            low,
            component_idx: idx,
        }
    }

    #[test]
    fn fewer_than_three_components_no_zhongshu() {
        let comps = [comp(10.0, 5.0, 0), comp(11.0, 6.0, 1)];
        assert!(zhongshu_from_components(&comps, 2).is_empty());
    }

    #[test]
    fn three_overlapping_components_unsettled_zhongshu() {
        // 三组件区间重叠 [5,10] ∩ [6,11] ∩ [4,9] → zd=max(5,6,4)=6, zg=min(10,11,9)=9。
        // 无第四组件突破 → 未闭合。
        let comps = [comp(10.0, 5.0, 0), comp(11.0, 6.0, 1), comp(9.0, 4.0, 2)];
        let zs = zhongshu_from_components(&comps, 2);
        assert_eq!(zs.len(), 1);
        let z = &zs[0];
        assert_eq!(z.zd, 6.0);
        assert_eq!(z.zg, 9.0);
        assert_eq!(z.comp_start, 0);
        assert_eq!(z.comp_end, 2);
        assert_eq!(z.comp_count, 3);
        assert!(!z.settled);
        assert_eq!(z.break_comp, -1);
        assert_eq!(z.break_direction, BreakDir::None);
        assert_eq!(z.gg, 11.0); // max(10,11,9)
        assert_eq!(z.dd, 4.0); // min(5,6,4)
        assert_eq!(z.level_id, 2);
    }

    #[test]
    fn fourth_component_breaks_up_settles() {
        // 前三组件中枢 zd=6 zg=9；第四组件 low=10 > zg=9 → 向上突破闭合。
        let comps = [
            comp(10.0, 5.0, 0),
            comp(11.0, 6.0, 1),
            comp(9.0, 4.0, 2),
            comp(15.0, 10.0, 3),
        ];
        let zs = zhongshu_from_components(&comps, 2);
        assert_eq!(zs.len(), 1);
        let z = &zs[0];
        assert!(z.settled);
        assert_eq!(z.break_comp, 3);
        assert_eq!(z.break_direction, BreakDir::Up);
        assert_eq!(z.comp_end, 2); // 突破组件不并入中枢
    }

    #[test]
    fn moves_empty_when_no_settled_zhongshu() {
        // 单个未闭合中枢 → settled 列表空 → 无 move。
        let comps = [comp(10.0, 5.0, 0), comp(11.0, 6.0, 1), comp(9.0, 4.0, 2)];
        let zs = zhongshu_from_components(&comps, 2);
        assert!(moves_from_level_zhongshus(&zs, Some(comps.len())).is_empty());
    }

    #[test]
    fn single_settled_zhongshu_yields_consolidation_move_unsettled() {
        // 一个 settled 中枢（向上突破）→ 一个盘整 move；末 move 强制 unsettled。
        let comps = [
            comp(10.0, 5.0, 0),
            comp(11.0, 6.0, 1),
            comp(9.0, 4.0, 2),
            comp(15.0, 10.0, 3),
        ];
        let zs = zhongshu_from_components(&comps, 2);
        let mvs = moves_from_level_zhongshus(&zs, Some(comps.len()));
        assert_eq!(mvs.len(), 1);
        let m = &mvs[0];
        assert_eq!(m.kind, MoveKind::Consolidation);
        assert_eq!(m.direction, Direction::Up); // = 中枢突破方向
        assert!(!m.settled); // 末 move 置 false
        assert_eq!(m.seg_start, 0); // comp_start
        assert_eq!(m.seg_end, 3); // 修复A：末组 = num_components - 1（含离开中枢的突破组件）
        assert_eq!(m.last_seg_s1, 2); // 锚点不扩展：仍 = comp_end
        assert_eq!(m.zg_max, 9.0);
        assert_eq!(m.zd_min, 6.0);
    }

    #[test]
    fn fix_a_non_last_move_extends_to_next_group_start() {
        // 两个反向 settled 中枢 → 两个 move；非末 move 的 seg_end 扩展到
        // 下一组首中枢 comp_start - 1（level-1 语义对齐，修复A）。
        let comps = [
            // 中枢1：[6,9]，被 comp3 向上突破
            comp(10.0, 5.0, 0),
            comp(11.0, 6.0, 1),
            comp(9.0, 4.0, 2),
            // 中枢2：[16,19]（递升），被 comp6 向下突破
            comp(20.0, 15.0, 3),
            comp(21.0, 16.0, 4),
            comp(19.0, 14.0, 5),
            comp(3.0, 1.0, 6),
        ];
        let zs = zhongshu_from_components(&comps, 2);
        assert_eq!(zs.len(), 2);
        assert!(zs[0].settled && zs[1].settled);
        let mvs = moves_from_level_zhongshus(&zs, Some(comps.len()));
        // 中枢2 ZD=16 > 中枢1 ZG=9 → 上涨延续 → 单个 trend move（2 中枢）
        assert_eq!(mvs.len(), 1);
        assert_eq!(mvs[0].kind, MoveKind::Trend);
        // 末组：seg_end = num_components - 1 = 6（含离开中枢2的突破组件）
        assert_eq!(mvs[0].seg_end, 6);
        assert_eq!(mvs[0].last_seg_s1, 5); // 锚点 = last_zs.comp_end
    }
}
