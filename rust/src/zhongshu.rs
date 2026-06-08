//! 中枢 v1（三段重叠法）— 逐位等价移植自 src/newchan/a_zhongshu_v1.py
//!
//! 两条构造路径共享滑窗算法 `scan_zhongshu`：
//! - `zhongshu_from_segments`：组件 = 线段，过滤 `confirmed AND kind=="settled"`，anchor=s0/s1
//! - `zhongshu_from_strokes`：组件 = 笔，过滤 `confirmed`，anchor=i0/i1
//!
//! ## 逐位等价要点
//! 1. 中枢构造**无浮点算术**——只有比较与三元 max/min 选择（`max(a,b,c)` = `a.max(b).max(c)`
//!    顺序约简，与 Python `max` 同序），故 zd/zg/gg/dd 等浮点字段 bit-exact 无约简风险。
//! 2. `break_seg`、续进锚 `max(break_seg-2, seg_end)` 用 i64 复刻 Python int（避免 usize 下溢）。
//! 3. 过滤逻辑（confirmed/kind/settled）在本模块内复刻，输入为未过滤的全量组件。

/// 突破方向。对应 Python break_direction: "" / "up" / "down"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakDir {
    None,
    Up,
    Down,
}

impl BreakDir {
    pub fn as_str(self) -> &'static str {
        match self {
            BreakDir::None => "",
            BreakDir::Up => "up",
            BreakDir::Down => "down",
        }
    }
}

/// 一个中枢实例。对应 Python `Zhongshu` frozen dataclass。
#[derive(Debug, Clone, Copy)]
pub struct Zhongshu {
    pub zd: f64,
    pub zg: f64,
    pub seg_start: usize,
    pub seg_end: usize,
    pub seg_count: usize,
    pub settled: bool,
    /// 突破组件索引；-1 = 未闭合（用 i64 复刻 Python 默认 -1）。
    pub break_seg: i64,
    pub break_direction: BreakDir,
    pub first_seg_s0: usize,
    pub last_seg_s1: usize,
    pub gg: f64,
    pub dd: f64,
}

/// 中枢组件的最小区间 + 时间锚（线段=s0/s1 笔索引；笔=i0/i1 merged idx）。
/// 对应 Python `_RangeComponent` + start_anchor/end_anchor 的提取结果。
#[derive(Debug, Clone, Copy)]
pub struct Component {
    pub high: f64,
    pub low: f64,
    pub anchor_start: usize,
    pub anchor_end: usize,
}

/// 从初始三段 (i, i+1, i+2) 尝试延伸中枢。移植自 `_extend_zhongshu`。
/// 返回 (seg_end_idx, j, gg, dd)。
fn extend_zhongshu(
    confirmed: &[Component],
    i: usize,
    n: usize,
    zd: f64,
    zg: f64,
) -> (usize, usize, f64, f64) {
    let mut gg = confirmed[i]
        .high
        .max(confirmed[i + 1].high)
        .max(confirmed[i + 2].high);
    let mut dd = confirmed[i]
        .low
        .min(confirmed[i + 1].low)
        .min(confirmed[i + 2].low);
    let mut seg_end_idx = i + 2;
    let mut j = i + 3;
    while j < n {
        let sj = &confirmed[j];
        if sj.high >= zd && sj.low <= zg {
            seg_end_idx = j;
            gg = gg.max(sj.high);
            dd = dd.min(sj.low);
            j += 1;
        } else {
            break;
        }
    }
    (seg_end_idx, j, gg, dd)
}

/// 判断突破方向。移植自 `_break_direction`。
fn break_direction(breaker: &Component, zg: f64, zd: f64) -> BreakDir {
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

/// 共享滑窗算法：三段重叠 → 延伸 → 突破 → 续进。移植自 `_scan_zhongshu`。
fn scan_zhongshu(confirmed: &[Component]) -> Vec<Zhongshu> {
    let n = confirmed.len();
    if n < 3 {
        return Vec::new();
    }

    let mut result: Vec<Zhongshu> = Vec::new();
    let mut i: usize = 0;

    while i + 2 < n {
        let (s1, s2, s3) = (&confirmed[i], &confirmed[i + 1], &confirmed[i + 2]);
        let zd = s1.low.max(s2.low).max(s3.low);
        let zg = s1.high.min(s2.high).min(s3.high);

        if zg <= zd {
            i += 1;
            continue;
        }

        let (seg_end_idx, j, gg, dd) = extend_zhongshu(confirmed, i, n, zd, zg);
        let settled = j < n;
        let break_seg_idx: i64 = if settled { j as i64 } else { -1 };
        let break_dir = if settled {
            break_direction(&confirmed[j], zg, zd)
        } else {
            BreakDir::None
        };

        result.push(Zhongshu {
            zd,
            zg,
            seg_start: i,
            seg_end: seg_end_idx,
            seg_count: seg_end_idx - i + 1,
            settled,
            break_seg: break_seg_idx,
            break_direction: break_dir,
            first_seg_s0: confirmed[i].anchor_start,
            last_seg_s1: confirmed[seg_end_idx].anchor_end,
            gg,
            dd,
        });

        if settled {
            // i = max(break_seg_idx - 2, seg_end_idx)（i64 复刻 Python int）
            let next = (break_seg_idx - 2).max(seg_end_idx as i64);
            i = next as usize;
        } else {
            break;
        }
    }

    result
}

/// 线段中枢：过滤 `confirmed AND kind=="settled"`，anchor=s0/s1。
/// 移植自 `zhongshu_from_segments`。
///
/// `segs`：每段 (s0, s1, high, low, confirmed, kind_is_settled)。
pub fn zhongshu_from_segments(segs: &[(usize, usize, f64, f64, bool, bool)]) -> Vec<Zhongshu> {
    let confirmed: Vec<Component> = segs
        .iter()
        .filter(|&&(_, _, _, _, confirmed, kind_settled)| confirmed && kind_settled)
        .map(|&(s0, s1, high, low, _, _)| Component {
            high,
            low,
            anchor_start: s0,
            anchor_end: s1,
        })
        .collect();
    scan_zhongshu(&confirmed)
}

/// 笔中枢：过滤 `confirmed`，anchor=i0/i1。移植自 `zhongshu_from_strokes`。
///
/// `strokes`：每笔 (i0, i1, high, low, confirmed)。
pub fn zhongshu_from_strokes(strokes: &[(usize, usize, f64, f64, bool)]) -> Vec<Zhongshu> {
    let confirmed: Vec<Component> = strokes
        .iter()
        .filter(|&&(_, _, _, _, confirmed)| confirmed)
        .map(|&(i0, i1, high, low, _)| Component {
            high,
            low,
            anchor_start: i0,
            anchor_end: i1,
        })
        .collect();
    scan_zhongshu(&confirmed)
}
