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

/// scan 循环退出时的可恢复断点（消除 scan_i 停滞 regime 的 O(n²)）。
///
/// scan 是**单调单遍**循环（i 只 +=1 或 settled 后跳跃前进，从不回退），故退出点
/// 可缓存，push 新组件后从断点续扫而非从 start_i 重扫。两个相位：
/// - `Exhausted { i }`：越界退出（while `i+2 < n` 失败），i = 下一待判三元组起点。
///   消除 trend regime（无中枢、scan_i 停滞）每次 push 从 scan_i 全程重扫 [scan_i,k] 的 O(n²)。
/// - `Unsettled { i, seg_end, gg, dd }`：push 了 unsettled 中枢后 break，缓存 extend 累积
///   状态。下次 push 只需对**新增组件**续 extend（O(1)），而非从 i+3 全程重扫重叠段
///   （unsettled regime——单中枢无限延伸——的 O(n²) 根因）。
#[derive(Debug, Clone, Copy)]
pub(crate) enum ScanResume {
    Exhausted { i: usize },
    Unsettled { i: usize, seg_end: usize, gg: f64, dd: f64 },
}

/// 从指定起点 `start_i` 开始的滑窗扫描（绝对索引语义不变）。
///
/// `scan_zhongshu(c) == scan_zhongshu_range(c, 0)`（逐位等价，由 test 守卫）。
/// 增量复用前提：`confirmed[..start_i]` 已产出的 settled 中枢永久固定，
/// 从 `start_i` 重扫只复现易变尾部。`start_i` 必须 = 最后一个 settled 中枢的续进锚
/// `max(break_seg-2, seg_end)`（即原循环 settled 后的 `i` 取值），否则破坏 bit-exact。
pub(crate) fn scan_zhongshu_range(confirmed: &[Component], start_i: usize) -> Vec<Zhongshu> {
    let mut out = Vec::new();
    scan_zhongshu_core(confirmed, start_i, &mut out);
    out
}

/// 可恢复版 scan：从 `start_i` 跑 scan 循环，结果追加到 `out`，返回退出断点。
///
/// 与 `scan_zhongshu_range(confirmed, start_i)` 逐位等价（out 追加内容相同），
/// 但暴露退出 `ScanResume` 供增量器续扫。`out` 不清空（调用方可预填不可变前缀）。
fn scan_zhongshu_core(confirmed: &[Component], start_i: usize, out: &mut Vec<Zhongshu>) -> ScanResume {
    scan_zhongshu_from(confirmed, start_i, None, out)
}

/// 从起点 `start_i` + 可选 resume 状态跑 scan 循环。
///
/// `resume = Some(Unsettled { i, seg_end, gg, dd })`：跳过 i 处成中枢判定（上轮已做），
/// 直接从 `seg_end + 1` 续 extend（用累积 gg/dd），消除 unsettled regime 每次 push 从
/// i+3 全程重扫重叠段的 O(n²)。`resume = None` 等价于 `scan_zhongshu_core`。
pub(crate) fn scan_zhongshu_from(
    confirmed: &[Component],
    start_i: usize,
    resume: Option<ScanResume>,
    out: &mut Vec<Zhongshu>,
) -> ScanResume {
    let n = confirmed.len();

    // unsettled resume：从缓存的 extend 断点续扫（可能产出 settled 或仍 unsettled）。
    if let Some(ScanResume::Unsettled { i, seg_end: prev_seg_end, gg: prev_gg, dd: prev_dd }) = resume {
        if i + 2 < n {
            let (s1, s2, s3) = (&confirmed[i], &confirmed[i + 1], &confirmed[i + 2]);
            let zd = s1.low.max(s2.low).max(s3.low);
            let zg = s1.high.min(s2.high).min(s3.high);
            let mut seg_end_idx = prev_seg_end;
            let mut j = prev_seg_end + 1;
            let mut gg = prev_gg;
            let mut dd = prev_dd;
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
            let settled = j < n;
            let break_seg_idx: i64 = if settled { j as i64 } else { -1 };
            let break_dir = if settled {
                break_direction(&confirmed[j], zg, zd)
            } else {
                BreakDir::None
            };
            out.push(Zhongshu {
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
                let next = (break_seg_idx - 2).max(seg_end_idx as i64);
                return scan_main_loop(confirmed, next as usize, out);
            } else {
                return ScanResume::Unsettled { i, seg_end: seg_end_idx, gg, dd };
            }
        }
        return ScanResume::Exhausted { i };
    }

    scan_main_loop(confirmed, start_i, out)
}

/// scan 主循环（无 resume）：从 start_i 跑滑窗扫描。
fn scan_main_loop(confirmed: &[Component], start_i: usize, out: &mut Vec<Zhongshu>) -> ScanResume {
    let n = confirmed.len();
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

        out.push(Zhongshu {
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
            // unsettled 中枢：break，缓存 extend 断点（下次 push 续 extend 而非重扫）。
            return ScanResume::Unsettled { i, seg_end: seg_end_idx, gg, dd };
        }
    }

    // 越界退出：i 是下一待判三元组起点（i+2 >= n）。
    ScanResume::Exhausted { i }
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
pub(crate) fn resume_index_after(zs: &Zhongshu) -> usize {
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
/// ## scan_i 停滞 regime 的 resume 优化（消除残留 O(n²)）
/// 当数据**永不成 settled 中枢**（trend 单调 / unsettled 长期盘整）时 `scan_i` 停滞，
/// 原实现每次 push 从 `scan_i` 全程重扫 [scan_i, k] → Σ = O(n²)。优化：缓存 scan 循环
/// 退出断点 `ScanResume.i`（单调单遍 scan 的续行点），push 后从断点续扫而非从 scan_i 重扫。
/// 断点之前产出的 Zhongshu（全 settled，extent 永久固定）跨 push 不变 → 复用为不可变前缀。
/// 每次续扫仅重判末尾三元组（涉及新组件）→ 摊还 O(1)/push → 全程 O(S)。
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
    /// scan 循环退出断点缓存（组件空间）。scan_i 停滞时从断点续扫而非从 scan_i 重扫。
    /// 失效条件：scan_i 推进 / confirmed 非单调 append（回退）。None = 需从 scan_i 全扫。
    resume: Option<(usize, ScanResume)>,
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

        // resume 优化：若 scan_i 未变且 confirmed 单调增长，从上次 scan 退出断点续扫，
        // 而非从 scan_i 全程重扫。消除 scan_i 停滞 regime（trend/unsettled）的 O(n²)。
        //
        // bit-exact 论证：resume 只在 `scan_i_advanced == false`（本轮无 settled 产出）
        // 时缓存。断点 `ScanResume.i` 是 scan 循环退出点；scan_i..i 之间恒无 Zhongshu
        // （若有 settled，scan_i 必推进 → resume 失效）。故从 i 续扫的结果 == 从 scan_i
        // 全扫的结果（scan 单调单遍，i 之前无产出，i 之后判定不依赖已被跳过的前缀）。
        let (start_i, cached_resume): (usize, Option<ScanResume>) = match self.resume {
            Some((csi, ScanResume::Exhausted { i })) if csi == self.scan_i => (i, None),
            Some((csi, ur @ ScanResume::Unsettled { .. })) if csi == self.scan_i => {
                (self.scan_i, Some(ur))
            }
            _ => (self.scan_i, None),
        };
        let mut tail = Vec::new();
        let new_resume = if let Some(ur) = cached_resume {
            scan_zhongshu_from(&self.confirmed, start_i, Some(ur), &mut tail)
        } else {
            scan_zhongshu_core(&self.confirmed, start_i, &mut tail)
        };

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
        let scan_i_advanced = next_scan_i != self.scan_i;
        self.scan_i = next_scan_i;

        // 更新 resume 缓存：scan_i 推进则失效（下次从新 scan_i 全扫）；否则记当前断点。
        if scan_i_advanced {
            self.resume = None;
        } else {
            self.resume = Some((self.scan_i, new_resume));
        }
    }

    /// 当前完整中枢列表（逐位等价于 `scan_zhongshu(&confirmed)`）。
    pub fn current(&self) -> &[Zhongshu] {
        &self.zhongshus
    }

    /// 永久固定的 settled 中枢前缀长度（`zhongshus[..stable_count]` 全 settled 不再变）。
    /// 上层增量（走势/买卖点）的 append-only 稳定边界来源。
    pub fn stable_count(&self) -> usize {
        self.stable_count
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

// ════════════════════════════════════════════════════════════
// 性能 profile（O(n²) 热点定位 + 优化前后标度对比）—— 性能工位 task #93
// ════════════════════════════════════════════════════════════
//
// 隔离基准：纯 zhongshu.rs（Component + scan_zhongshu_range + IncrementalBiZhongshu），
// 不依赖 segment.rs/divergence.rs。逐 push 驱动 IncrementalBiZhongshu（模拟逐笔增量），
// 递增规模实测总耗时，拟合 log-log 标度指数 exp（total_time ~ n^exp）。
//
// 三种数据 regime（定位 scan_i 停滞导致的尾部重扫 O(n²)）：
//   - trend     ：单调上行，无三段重叠 → 永不成中枢 → scan_i 停滞 0 → 每 push 重扫 [0,k]。
//   - unsettled ：持续盘整不突破 → 单个未结算中枢无限延伸 → scan_i 停滞 → 每 push 重扫。
//   - settling  ：规律突破结算 → scan_i 随结算中枢前进 → 期望近线性。
//
// L1 度量（formalization-validity-domain 231号）：CPU 耗时是确定性工程度量，零信息增量。
// 跑法：cargo test --lib zhongshu::perf_profile -- --ignored --nocapture
#[cfg(test)]
mod perf_profile {
    use super::*;
    use std::time::Instant;

    const SIZES: [usize; 5] = [1000, 2000, 4000, 8000, 16000];

    /// 单调上行：comp k = [101+2k, 100+2k]（low 始终高于前一 high）→ 无重叠 → 无中枢。
    fn gen_trend(n: usize) -> Vec<Component> {
        (0..n)
            .map(|k| Component {
                high: 101.0 + 2.0 * k as f64,
                low: 100.0 + 2.0 * k as f64,
                anchor_start: k,
                anchor_end: k,
            })
            .collect()
    }

    /// 持续盘整：所有组件落在 [≈95.5,104.5] 重叠带内 → 单个中枢无限 extend，永不突破。
    fn gen_unsettled(n: usize) -> Vec<Component> {
        (0..n)
            .map(|k| {
                let j = (k % 2) as f64 * 0.5;
                Component {
                    high: 104.0 + j,
                    low: 96.0 - j,
                    anchor_start: k,
                    anchor_end: k,
                }
            })
            .collect()
    }

    /// 规律结算：周期 4 —— 3 个紧重叠组件成中枢 + 1 个向上突破组件结算，中心逐块抬升。
    fn gen_settling(n: usize) -> Vec<Component> {
        let mut out = Vec::with_capacity(n);
        let mut center = 100.0_f64;
        for k in 0..n {
            let c = match k % 4 {
                0 => Component { high: center + 6.0, low: center - 6.0, anchor_start: k, anchor_end: k },
                1 => Component { high: center + 4.0, low: center - 4.0, anchor_start: k, anchor_end: k },
                2 => Component { high: center + 5.0, low: center - 5.0, anchor_start: k, anchor_end: k },
                _ => {
                    let c = Component { high: center + 30.0, low: center + 20.0, anchor_start: k, anchor_end: k };
                    center += 25.0;
                    c
                }
            };
            out.push(c);
        }
        out
    }

    fn drive_incremental(comps: &[Component]) -> (std::time::Duration, usize) {
        let t = Instant::now();
        let mut inc = IncrementalBiZhongshu::new();
        for c in comps {
            inc.push_confirmed(c.high, c.low, c.anchor_start, c.anchor_end);
        }
        let len = inc.current().len();
        (t.elapsed(), std::hint::black_box(len))
    }

    fn fit_and_print(name: &str, gen: impl Fn(usize) -> Vec<Component>) {
        eprintln!("\n── regime={name} ──（IncrementalBiZhongshu 逐 push 累计总耗时；单次 scan_zhongshu 作 O(n) 对照）");
        let mut rows: Vec<(usize, f64, f64, usize)> = Vec::new();
        for &n in &SIZES {
            let comps = gen(n);
            let mut best_inc = f64::INFINITY;
            let mut zs_len = 0usize;
            for _ in 0..3 {
                let (d, l) = drive_incremental(&comps);
                best_inc = best_inc.min(d.as_secs_f64());
                zs_len = l;
            }
            // 单次全量 scan 作线性对照。
            let mut best_full = f64::INFINITY;
            for _ in 0..3 {
                let t = Instant::now();
                let r = scan_zhongshu(&comps);
                let _ = std::hint::black_box(r.len());
                best_full = best_full.min(t.elapsed().as_secs_f64());
            }
            rows.push((n, best_inc, best_full, zs_len));
            eprintln!("  n={n:>6}  inc_total={best_inc:>10.6}s  full_once={best_full:>10.6}s  zs={zs_len}");
        }
        let expfit = |sel: &dyn Fn(&(usize, f64, f64, usize)) -> f64| {
            let (n0, n1) = (rows[0].0 as f64, rows[rows.len() - 1].0 as f64);
            let (t0, t1) = (sel(&rows[0]), sel(&rows[rows.len() - 1]));
            (t1 / t0).ln() / (n1 / n0).ln()
        };
        for w in rows.windows(2) {
            let exp = (w[1].1 / w[0].1).ln() / (w[1].0 as f64 / w[0].0 as f64).ln();
            eprintln!("    inc {}→{}: exp≈{exp:.2}", w[0].0, w[1].0);
        }
        eprintln!(
            "    overall: inc_exp≈{:.2}  full_exp≈{:.2}  (1=线性 2=平方)",
            expfit(&|r| r.1),
            expfit(&|r| r.2)
        );
    }

    #[test]
    #[ignore = "性能 profile：cargo test --lib zhongshu::perf_profile -- --ignored --nocapture"]
    fn profile_incremental_bi_zhongshu_scaling() {
        eprintln!("\n===== zhongshu.rs IncrementalBiZhongshu 标度 profile（debug，L1 度量）=====");
        fit_and_print("trend(无中枢)", gen_trend);
        fit_and_print("unsettled(单中枢无限延伸)", gen_unsettled);
        fit_and_print("settling(规律结算)", gen_settling);
    }
}
