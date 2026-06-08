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
///
/// `PartialEq` 仅供 differential test 逐字段比对（f64 精确相等，中枢字段恒有限无 NaN）。
#[derive(Debug, Clone, Copy, PartialEq)]
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
    scan_zhongshu_range(confirmed, 0)
}

/// 从指定起点 `start_i` 开始的滑窗扫描（绝对索引语义不变）。
///
/// `scan_zhongshu(c) == scan_zhongshu_range(c, 0)`（逐位等价，由 test 守卫）。
/// 增量复用前提：`confirmed[..start_i]` 已产出的 settled 中枢永久固定，
/// 从 `start_i` 重扫只复现易变尾部。`start_i` 必须 = 最后一个 settled 中枢的续进锚
/// `max(break_seg-2, seg_end)`（即原循环 settled 后的 `i` 取值），否则破坏 bit-exact。
fn scan_zhongshu_range(confirmed: &[Component], start_i: usize) -> Vec<Zhongshu> {
    let n = confirmed.len();
    if n < 3 {
        return Vec::new();
    }

    let mut result: Vec<Zhongshu> = Vec::new();
    let mut i: usize = start_i;

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

/// settled 中枢的续进锚 —— 复刻 scan 循环 settled 分支 `i = max(break_seg-2, seg_end)`。
///
/// 仅对 `settled==true` 的中枢有意义（break_seg ≥ 0）。返回下一轮扫描的起点。
fn resume_index_after(zs: &Zhongshu) -> usize {
    let v = (zs.break_seg - 2).max(zs.seg_end as i64);
    v.max(0) as usize
}

/// 增量笔中枢扫描器 —— 消除 `current_bi_zhongshu_buysellpoints` 的逐笔 O(S) 全量重扫。
///
/// ## 不变量（bit-exact 契约，由 differential test 守卫）
/// 任意时刻 `current() == scan_zhongshu(&confirmed_so_far)`（全量重扫的逐位结果）。
///
/// ## 增量原理（摊还 O(S) total）
/// `scan_zhongshu` 左→右单调处理。一个 settled 中枢的 extent 在其 breaker 出现后
/// **永久固定**（移植要点 E：字段由 seg_start..=break_seg 内 confirmed 笔唯一决定，
/// 与后续新增笔无关）。唯一易变的是**末尾未结算中枢** + 最后一个 settled 之后的未成形窗口。
/// 故缓存全部 settled 中枢（stable 前缀）+ 续进锚 `scan_i`，每次 append 只从 `scan_i`
/// 重扫易变尾部。每个组件一生只进入易变窗口一次 → Σ 重扫 = O(S)。
///
/// ## 前提
/// 输入是 **confirmed 笔**，append-only（已 confirmed 笔不可变，最后一笔 unconfirmed
/// 不传入——调用方只在笔 confirm 后 push）。详见 bi_engine.rs append-only 语义。
#[derive(Debug, Clone, Default)]
pub struct IncrementalBiZhongshu {
    confirmed: Vec<Component>,
    /// 当前完整中枢列表 = stable 前缀（全 settled）+ 易变尾部。
    zhongshus: Vec<Zhongshu>,
    /// zhongshus[..stable_count] 永久固定（全 settled），不再重算。
    stable_count: usize,
    /// 续进锚：最后一个 settled 中枢的 `max(break_seg-2, seg_end)`；从此重扫尾部。
    scan_i: usize,
}

impl IncrementalBiZhongshu {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一根 confirmed 笔并增量更新中枢列表。
    pub fn push_confirmed(&mut self, high: f64, low: f64, anchor_start: usize, anchor_end: usize) {
        self.confirmed.push(Component {
            high,
            low,
            anchor_start,
            anchor_end,
        });
        self.rescan_tail();
    }

    /// 丢弃易变尾部，从续进锚重扫，更新 stable 边界。
    fn rescan_tail(&mut self) {
        self.zhongshus.truncate(self.stable_count);
        let tail = scan_zhongshu_range(&self.confirmed, self.scan_i);
        // 尾部形如 [settled*, 可选一个 unsettled]（scan 遇 unsettled 即 break）。
        // 所有 settled 永久固定 → 计入 stable；续进锚推进到最后一个 settled 之后。
        let mut settled_in_tail = 0usize;
        let mut next_scan_i = self.scan_i;
        for z in &tail {
            if z.settled {
                settled_in_tail += 1;
                next_scan_i = resume_index_after(z);
            }
        }
        self.zhongshus.extend_from_slice(&tail);
        self.stable_count += settled_in_tail;
        self.scan_i = next_scan_i;
    }

    /// 当前完整中枢列表（逐位等价于 `scan_zhongshu(&confirmed)`）。
    pub fn current(&self) -> &[Zhongshu] {
        &self.zhongshus
    }
}

#[cfg(test)]
mod incremental_tests {
    use super::*;

    fn comp(high: f64, low: f64, idx: usize) -> Component {
        Component {
            high,
            low,
            anchor_start: idx,
            anchor_end: idx,
        }
    }

    /// 逐前缀断言：增量结果在每一步都 == 全量 scan_zhongshu。
    fn assert_incremental_matches_full(comps: &[Component]) {
        let mut inc = IncrementalBiZhongshu::new();
        for (k, c) in comps.iter().enumerate() {
            inc.push_confirmed(c.high, c.low, c.anchor_start, c.anchor_end);
            let full = scan_zhongshu(&comps[..=k]);
            assert_eq!(
                inc.current(),
                full.as_slice(),
                "增量与全量在前缀长度 {} 处发散",
                k + 1
            );
        }
    }

    #[test]
    fn matches_full_on_simple_overlap_then_break() {
        // 三段重叠成中枢，第四段向上突破 → settled，后续再形成第二中枢。
        let comps = vec![
            comp(110.0, 90.0, 0),
            comp(105.0, 95.0, 1),
            comp(108.0, 92.0, 2),
            comp(130.0, 120.0, 3), // 突破：low>zg → settled up
            comp(135.0, 122.0, 4),
            comp(133.0, 121.0, 5),
            comp(138.0, 123.0, 6),
            comp(160.0, 150.0, 7), // 第二次突破
        ];
        assert_incremental_matches_full(&comps);
    }

    #[test]
    fn matches_full_no_overlap_sequence() {
        // 无重叠（持续单调）→ 无中枢，i+=1 滑过。
        let comps: Vec<Component> = (0..12)
            .map(|k| comp(100.0 + k as f64 * 10.0 + 5.0, 100.0 + k as f64 * 10.0, k))
            .collect();
        assert_incremental_matches_full(&comps);
    }

    #[test]
    fn matches_full_long_unsettled_tail() {
        // 形成中枢后长期盘整不突破 → 末尾 unsettled 中枢随每根笔扩展（易变尾部压力测试）。
        let mut comps = vec![
            comp(110.0, 90.0, 0),
            comp(105.0, 95.0, 1),
            comp(108.0, 92.0, 2),
        ];
        for k in 3..20 {
            // 持续落在 [92,105] 重叠区内 → 不突破，中枢不断 extend。
            let phase = if k % 2 == 0 { 104.0 } else { 93.0 };
            comps.push(comp(phase + 1.0, phase - 1.0, k));
        }
        assert_incremental_matches_full(&comps);
    }

    #[test]
    fn matches_full_deterministic_lcg_stress() {
        // 确定性 LCG 生成多样 high/low，覆盖 overlap/break/extend/续进 各路径。
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64 // [0,1)
        };
        let mut comps: Vec<Component> = Vec::new();
        let mut center = 100.0_f64;
        for k in 0..400 {
            center += (next() - 0.5) * 8.0; // 随机游走中心
            let half = 1.0 + next() * 6.0; // 半宽
            comps.push(comp(center + half, center - half, k));
        }
        assert_incremental_matches_full(&comps);
    }

    #[test]
    fn empty_and_under_three() {
        let inc = IncrementalBiZhongshu::new();
        assert_eq!(inc.current(), &[] as &[Zhongshu]);
        let comps = vec![comp(110.0, 90.0, 0), comp(105.0, 95.0, 1)];
        assert_incremental_matches_full(&comps); // <3 → 恒空
    }
}
