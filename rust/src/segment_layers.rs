//! segment_layers.rs — 线段级中枢 / 背驰的**增量器**（消除 orchestrator 残留 O(n_seg²)）。
//!
//! ## 背景（残留 O(结构²) 第二波）
//! `bi_engine` + `SegCheckpoint` 已把笔/线段层增量化。但 orchestrator 的线段级
//! 中枢/走势/买卖点层每次结构变化仍**全量重算**：`zhongshu_from_segments(全部线段)`
//! 每次 O(n_seg) 过滤 + 扫描，× O(n_seg) 次结构变化 = O(n_seg²)（buysellpoint 层同构）。
//!
//! ## append-only 障碍与解法
//! 笔级 `IncrementalBiZhongshuBsp` 的四层增量器假设组件 **append-only**。线段级**违反**
//! 此假设：`kind=="settled"` 的段在 checkpoint 稳定（`trigger_k+1+MARGIN ≤ n_strokes`）
//! **之前**仍可被修订（这正是「40 vs 42 段」回归的根因——第二特征序列前向扫描窗口）。
//!
//! 解法——**稳定边界感知**：`SegCheckpoint.stable_count` 给出永久固定段前缀 `[0, stable_count)`。
//! 仅此前缀的 settled 段是 append-only 永久组件；尾部段 `[stable_count, n_seg)` 有界
//! （≤ MARGIN 笔窗口内的段数，≤ ~17）必须每次重算。
//!
//! 1. `IncrementalSegZhongshu`：组件 = settled 段。永久组件（< stable 边界）的中枢缓存，
//!    易变尾部组件每次 `scan_zhongshu_range` 续扫（摊还 O(易变尾)）。中枢「永久固定」判据
//!    从笔级的「settled」收紧为「settled ∧ break_seg < comp_stable_len」（破点组件在永久前缀内）。
//! 2. `IncrementalSegDivergences`：背驰 per-move（回溯仅限 move 自身段范围，有界）。
//!    `seg_end < n_seg - WINDOW` 的 move 背驰永久固定（缓存），尾部窗口每次重算。
//!
//! 买卖点层直接复用 `buysellpoint::IncrementalBsp`（其本身已用 SAFE_WINDOW=256 段窗口重算，
//! 对易变段尾鲁棒，无 append-only 假设）。view 镜像（SegView/ZsView/MoveView）由 orchestrator
//! 增量维护（截断到稳定边界 + 重推有界尾部）。
//!
//! ## 有效域声明（formalization-validity-domain.md）
//! 本模块的背驰路径为**纯结构性**（价格振幅 fallback，`detect_one` 内 `macd=None`）——
//! 对应 `enable_macd_divergence=False`（E/I 两版均如此）。MACD 三维度背驰依赖原始 bar，
//! 非 per-move 有界回溯，**不在本增量器有效域内**；orchestrator 在 `enable_macd` 时回退全量。
//!
//! ## bit-exact 契约
//! - `IncrementalSegZhongshu.zhongshus() == zhongshu_from_segments(全部线段)`
//! - `IncrementalSegDivergences.current() == divergences_from_moves_v1(.., None)`
//! 由本模块 `incremental_tests`（LCG 真实线段序列 + 复刻 checkpoint 稳定判据）逐前缀守卫；
//! 端到端真实数据（OKLO 447K 逐 bar）由 Python 差分脚本守卫。

use crate::buysellpoint::{
    build_type1_bsp, build_type2_bsp, build_type3_bsp, detect_overlap, BspKind, BuySellPoint,
};
use crate::divergence::{detect_one, DivKind, Divergence, MoveView, SegView, ZsView};
use crate::segment::{SegKind, Segment};
use crate::zhongshu::{resume_index_after, scan_zhongshu_range, BreakDir, Component, Zhongshu};

// ════════════════════════════════════════════════════════════
// 线段级增量中枢（稳定边界感知 — 处理易变段尾）
// ════════════════════════════════════════════════════════════

/// 线段级增量中枢构造器 —— bit-exact 等价于逐前缀 `zhongshu_from_segments(全部线段)`，
/// 摊还 O(易变段尾)/结构变化（替代每次 O(n_seg) 过滤 + 扫描）。
///
/// ## 不变量（bit-exact 契约，differential test 守卫）
/// `zhongshus() == zhongshu_from_segments(segments)`（全量过滤 settled + scan_zhongshu）。
///
/// ## 增量原理（与 `IncrementalBiZhongshu` 同构，但稳定判据收紧）
/// settled 段组件 `[0, comp_stable_len)` 来自永久固定段前缀 `[0, stable_count)`——append-only。
/// 一个中枢「永久固定」⟺ `settled ∧ break_seg < comp_stable_len`（破点组件在永久前缀内 ⟹
/// 其全部组件 [seg_start..=break_seg] 均在永久前缀 ⟹ 字段不再变）。缓存这些 + 续进锚 `scan_i`，
/// 每次只从 `scan_i` 重扫 `comp_all`（= 永久组件 ++ 易变尾组件）。
///
/// 与笔级的差异：笔级 push 的组件**全部**永久（append-only），故每个 settled 中枢即永久；
/// 线段级组件尾部易变，故须用 `comp_stable_len` 守门——只把破点在永久区的中枢转永久。
#[derive(Debug, Clone, Default)]
pub struct IncrementalSegZhongshu {
    /// 全部 settled 段组件 = [永久组件 `[0,comp_stable_len)` | 易变尾组件]（每次截断+重建尾）。
    comp_all: Vec<Component>,
    /// 永久组件前缀长度（来自永久固定段前缀的 settled 过滤，单调增）。
    comp_stable_len: usize,
    /// 已分类入永久组件的段数（= 上次 stable_count，O(1) 续扫起点，单调增）。
    seg_scanned: usize,
    /// 永久固定中枢前缀（break_seg < comp_stable_len，按扫描序，永不重算）。
    zs_stable: Vec<Zhongshu>,
    /// 续进锚（组件空间）：最后一个永久中枢之后；从此重扫尾部。
    scan_i: usize,
    /// 当前全量中枢 = zs_stable ++ 易变尾（缓存供 zhongshus() 返回）。
    full_zs: Vec<Zhongshu>,
}

impl IncrementalSegZhongshu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 增量更新中枢。
    ///
    /// `segments`：当前全量线段列表（含易变尾；orchestrator 的 `prev_segments`）。
    /// `stable_count`：永久固定段前缀长度（`SegCheckpoint.stable_count`，单调增）——
    ///   `segments[0, stable_count)` 跨调用逐位不变（resume≡full 不变量保证）。
    pub fn update(&mut self, segments: &[Segment], stable_count: usize) {
        // 防御：stable_count 单调（checkpoint 不变量）；若回退则全量重建（不应发生）。
        if stable_count < self.seg_scanned {
            self.comp_all.clear();
            self.comp_stable_len = 0;
            self.seg_scanned = 0;
            self.zs_stable.clear();
            self.scan_i = 0;
        }

        // 1. 丢弃上次易变尾组件，保留永久组件前缀。
        self.comp_all.truncate(self.comp_stable_len);

        // 2. 新晋永久组件：扫 segments[seg_scanned..stable_count] 的 settled 段（O(增量)）。
        let sc = stable_count.min(segments.len());
        while self.seg_scanned < sc {
            let s = &segments[self.seg_scanned];
            if s.confirmed && s.kind == SegKind::Settled {
                self.comp_all.push(seg_component(s));
            }
            self.seg_scanned += 1;
        }
        self.comp_stable_len = self.comp_all.len();

        // 3. 易变尾组件：扫 segments[stable_count..] 的 settled 段（O(易变尾)，每次重建）。
        for s in &segments[sc..] {
            if s.confirmed && s.kind == SegKind::Settled {
                self.comp_all.push(seg_component(s));
            }
        }

        // 4. 从续进锚重扫尾部（O(comp_all.len() - scan_i) = O(易变尾 + 未成形窗口)）。
        let tail = scan_zhongshu_range(&self.comp_all, self.scan_i);

        // 5. 推进永久边界：尾部前缀中 settled ∧ break_seg < comp_stable_len 的中枢转永久。
        //    （break 组件在永久前缀 ⟹ 该中枢字段由永久组件唯一决定 ⟹ 后续不变。）
        let mut n_stab = 0usize;
        for z in &tail {
            if z.settled && z.break_seg >= 0 && (z.break_seg as usize) < self.comp_stable_len {
                self.zs_stable.push(*z);
                self.scan_i = resume_index_after(z);
                n_stab += 1;
            } else {
                break;
            }
        }

        // 6. full = zs_stable ++ tail[n_stab..]（避免重复计入已转永久的前缀）。
        self.full_zs.clear();
        self.full_zs.extend_from_slice(&self.zs_stable);
        self.full_zs.extend_from_slice(&tail[n_stab..]);
    }

    /// 当前全量中枢（逐位等价于 `zhongshu_from_segments(segments)`）。
    pub fn zhongshus(&self) -> &[Zhongshu] {
        &self.full_zs
    }

    /// 永久固定中枢前缀长度（`full_zs[..stable_count]` 全 settled 不再变）——
    /// 上层 view 镜像（zs_view/zs_break）的增量截断边界来源。单调非减。
    pub fn stable_count(&self) -> usize {
        self.zs_stable.len()
    }
}

/// settled 段 → 中枢组件（anchor=s0/s1，复刻 `zhongshu_from_segments` 的映射）。
#[inline]
fn seg_component(s: &Segment) -> Component {
    Component {
        high: s.high,
        low: s.low,
        anchor_start: s.s0,
        anchor_end: s.s1,
    }
}

// ════════════════════════════════════════════════════════════
// 线段级增量背驰（per-move 窗口重算 — 处理易变 move 尾）
// ════════════════════════════════════════════════════════════

/// 背驰窗口（段空间）：`seg_end < n_seg - DIV_WINDOW` 的 move 背驰永久固定。
/// 易变段尾（≤ ~17）≪ 256，保守覆盖；与 `IncrementalBsp::SAFE_WINDOW` 对齐。
const DIV_WINDOW: i64 = 256;

/// 线段级增量背驰构造器 —— bit-exact 等价于逐前缀 `divergences_from_moves_v1(.., None)`，
/// 摊还 O(窗口内 move)/结构变化（替代每次 O(n_seg) 全 move 段遍历）。
///
/// ## 不变量（bit-exact 契约，differential test 守卫）
/// `current() == divergences_from_moves_v1(segs, zss, moves, level_id, None)`。
///
/// ## 增量原理
/// 背驰 per-move（每 move 0 或 1 个 div），回溯仅限 move 自身段范围 `[a_start, seg_end]`。
/// `seg_end < n_seg - DIV_WINDOW` 的 move：其段范围全在固定区 ⟹ div 永久固定（缓存）。
/// pending move（末个，`seg_end = num_seg-1`）恒 ≥ anchor ⟹ 每次重算。窗口内 move 稀疏
/// （平均 move ~17 段 → 256 段窗口 ~15 move），每次重算 O(窗口) 而非 O(全 move)。
///
/// **append-only 安全性**：committed move（`seg_end < anchor`）依赖的 settled 中枢段范围
/// 全 < anchor = n_seg - 256，而易变段尾 < 17 ≪ 256 ⟹ 跨调用逐位不变。`detect_one` 不读
/// `mv.settled`（committed 时可能仍标 pending），故 settled 翻转不影响已缓存 div。
#[derive(Debug, Clone, Default)]
pub struct IncrementalSegDivergences {
    /// committed move 的 div（seg_end < anchor，永久固定，按 move 序）。
    stable_divs: Vec<Divergence>,
    /// 已 committed 的 move 数（前缀，单调增）。
    processed_moves: usize,
    /// 当前全量 div = stable_divs ++ 尾部窗口重算（缓存供 current() 返回）。
    full_divs: Vec<Divergence>,
}

impl IncrementalSegDivergences {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 增量更新背驰列表。
    ///
    /// `moves`：当前全量 MoveView（最后一个为 pending）。`segs`/`zss`：当前全量视图。
    /// `n_seg`：线段总数（窗口锚点 = n_seg - DIV_WINDOW）。
    pub fn update(
        &mut self,
        moves: &[MoveView],
        segs: &[SegView],
        zss: &[ZsView],
        level_id: i64,
        n_seg: usize,
    ) {
        let anchor = n_seg as i64 - DIV_WINDOW;

        // 1. commit 新 move（依赖全固定 → div 永久）。按序推进。
        //    B2 后趋势 C 段窗口可越入下一 settled 中枢覆盖区 ⟹ commit 判据从
        //    `mv.seg_end < anchor` 收紧为 `detect_dep_end < anchor`（最大段依赖
        //    含 B2 窗口上限；pending move 窗口无界 = i64::MAX 永不 commit）。
        while self.processed_moves < moves.len() {
            let mv = &moves[self.processed_moves];
            let dep = crate::divergence::detect_dep_end(zss, mv);
            if mv.seg_end >= 0 && dep < anchor {
                if let Some(d) = detect_one(segs, zss, mv, level_id) {
                    self.stable_divs.push(d);
                }
                self.processed_moves += 1;
            } else {
                break;
            }
        }

        // 2. 尾部窗口重算（moves[processed_moves..]）：每次 O(窗口内 move)。
        self.full_divs.clear();
        self.full_divs.extend_from_slice(&self.stable_divs);
        for mv in &moves[self.processed_moves..] {
            if let Some(d) = detect_one(segs, zss, mv, level_id) {
                self.full_divs.push(d);
            }
        }
    }

    /// 当前全量背驰（逐位等价于 `divergences_from_moves_v1(.., None)`）。
    pub fn current(&self) -> &[Divergence] {
        &self.full_divs
    }

    /// 永久前缀长度（`current()[..stable_len]` 跨调用逐位不变）——
    /// IncrementalSegBsp 的 div frontier 推进边界来源。单调非减。
    pub fn stable_len(&self) -> usize {
        self.stable_divs.len()
    }

    /// commit frontier：已 commit 的 move 前缀长度（单调非减）。
    /// `moves[processed_moves]` 即第一个未 commit（易变）move——
    /// IncrementalSegBsp 的 volatile_floor 来源。
    pub fn processed_moves(&self) -> usize {
        self.processed_moves
    }
}

// ════════════════════════════════════════════════════════════
// 线段级增量买卖点（处理易变 div/move/zhongshu 尾 — 无状态 binary-search 窗口）
// ════════════════════════════════════════════════════════════

/// 买卖点稳定窗口（段空间）：seg_idx < n_seg - SAFE_W 的 bsp 永久固定。
const BSP_SAFE_W: i64 = 256;
/// 源侧回看裕度：源段 >= n_seg - SAFE_W - SRC_MARGIN 的 div/type1/中枢参与尾部重算。
const BSP_SRC_MARGIN: i64 = 256;

/// 线段级增量买卖点构造器 —— bit-exact 等价于逐前缀 `buysellpoints_from_level(.., None div)`，
/// 摊还 O(窗口)/结构变化（替代每次 O(n_seg) MoveLookup.build + 全 div/中枢遍历）。
///
/// ## 为何不能直接复用 `IncrementalBsp`（笔级全链增量器）
/// `IncrementalBsp` 用**单调 frontier**（div_scan_from/zs_scan_from/stable_scan_from，索引进
/// volatile 数组）+ **move-based lookup 永久缓存**（filled_moves/perm_upto），二者均假设输入
/// **append-only**。线段级 div/move/中枢**尾部易变**（settled 段在 checkpoint 稳定前可修订 →
/// 中枢/走势/背驰尾部重算 → 数组索引漂移）违反此假设（已由 OKLO 447K 差分在 bar 350200 暴露：
/// type2/type3 错配 + 计数差 1）。
///
/// ## 解法：无状态 binary-search 窗口 + 每次重建窗口 lookup
/// 1. **稳定前缀缓存**：`seg_idx < stable_anchor = n_seg - SAFE_W` 的 bsp 永久固定（其全部依赖段
///    ∈ 固定区，易变段尾 < 17 ≪ 256）。`stable_anchor` 单调前进。
/// 2. **尾部重算**：每次对 `src >= src_from = stable_anchor - SRC_MARGIN` 的 div/type1/中枢重算。
///    源已按 seg 排序（div.seg_c_end / bsp.seg_idx / 中枢 break_seg 单调）→ `partition_point`
///    O(log n) 定位窗口起点，**不缓存索引**（每次重搜，对易变尾鲁棒）。
/// 3. **窗口 lookup**：seg→move 映射只为 `[src_from, n_seg)` 重建（O(窗口)），不做永久缓存。
///
/// 每次 O(log n + 窗口)，全程 O(n_seg)。bit-exact 由 differential test + 真实数据端到端守卫。
#[derive(Debug, Clone)]
pub struct IncrementalSegBsp {
    level_id: i64,
    /// 次级别走势（线段）settle 合取门（编排者 2026-06-13）：require_settled 时
    /// BSP confirmed 合取 anchor 段 `Segment.confirmed`（走势已完成，第65课），压制
    /// pending 生长期伪背驰（§3，B2 未覆盖的 pending 侧）。默认 false（在册口径）。
    require_settled: bool,
    /// seg_idx < stable_anchor 的 bsp（永久固定，按 seg_idx 升序）。
    stable_bsps: Vec<BuySellPoint>,
    stable_anchor: i64,
    /// div frontier：已确认永久 finalize 的 div 前缀（仅在 divs 永久前缀内推进）。
    /// B2 后 div.seg_c_end 跨 move 不再全局单调（趋势 C 段可越入下一中枢覆盖区，
    /// 与后继盘整 div 形成局部逆序）⟹ 不能 partition_point 二分，改单调 frontier。
    div_done: usize,
    /// 当前全量 bsp（stable + 尾部，缓存供 current() 返回）。
    full_bsps: Vec<BuySellPoint>,
}

impl IncrementalSegBsp {
    pub fn new(level_id: i64, require_settled: bool) -> Self {
        IncrementalSegBsp {
            level_id,
            require_settled,
            stable_bsps: Vec::new(),
            stable_anchor: 0,
            div_done: 0,
            full_bsps: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.stable_bsps.clear();
        self.stable_anchor = 0;
        self.div_done = 0;
        self.full_bsps.clear();
    }

    /// 增量更新买卖点。`divs`：纯结构性背驰（inc_seg_div 输出，macd=None）。
    /// `divs_stable_len`：divs 的永久前缀长度（`IncrementalSegDivergences::stable_len`）——
    /// frontier 只在永久前缀内推进（易变尾 div 索引可漂移，不可缓存其位置）。
    /// `volatile_floor`：第一个未 commit move 的 seg_start（`IncrementalSegDivergences::
    /// processed_moves` 处 move；moves 空 → 0）——B2 后易变 div 的 C 段极值可远落于
    /// `n - SAFE_W` 之前，anchor 以此为上限保证 stable 区无易变源（单调：commit
    /// frontier 前进 ⟹ floor 前进）。
    pub fn update(
        &mut self,
        segs: &[SegView],
        zss: &[ZsView],
        zs_break: &[(bool, BreakDir, i64)],
        moves: &[MoveView],
        divs: &[Divergence],
        divs_stable_len: usize,
        volatile_floor: i64,
    ) {
        let n_seg = segs.len();
        let new_anchor = (n_seg as i64 - BSP_SAFE_W)
            .min(volatile_floor)
            .max(self.stable_anchor);
        let src_from = (new_anchor - BSP_SRC_MARGIN).max(0);

        // ── 窗口 lookup：seg→move 映射，仅覆盖 [src_from, n_seg)（每次重建，O(窗口)）──
        let win_lo = src_from.max(0) as usize;
        let win_len = n_seg.saturating_sub(win_lo);
        let mut win_lookup: Vec<Option<usize>> = vec![None; win_len];
        // moves 按 seg_end 升序 → partition_point 定位首个 seg_end >= src_from 的 move。
        let mstart = moves.partition_point(|m| m.seg_end < src_from);
        for (off, mv) in moves[mstart..].iter().enumerate() {
            let mi = mstart + off;
            if mv.seg_start < 0 || mv.seg_end < 0 {
                continue;
            }
            let lo = (mv.seg_start.max(src_from)) as usize;
            let hi = (mv.seg_end as usize).min(n_seg.saturating_sub(1));
            for slot in win_lookup
                .iter_mut()
                .take(hi + 1 - win_lo)
                .skip(lo - win_lo)
            {
                if slot.is_none() {
                    *slot = Some(mi);
                }
            }
        }
        let lookup_find = |seg_idx: i64| -> Option<usize> {
            if seg_idx < src_from || seg_idx < 0 {
                return None;
            }
            let idx = seg_idx as usize - win_lo;
            win_lookup.get(idx).copied().flatten()
        };

        // ── 尾部 type1：seg_c_end >= stable_anchor（**未 finalize** 的 type1；已 finalize 的
        //    [src_from, stable_anchor) type1 由 stable_t1 提供——二者按 stable_anchor 严格不相交，
        //    避免同一 type1 在 stable_t1+tail_type1 双重计入 → type2 重复（bar 350200 发散根因））。
        //    frontier：只越过「永久前缀内且 seg_c_end < stable_anchor」的 div（B2 后
        //    seg_c_end 非全局单调，不可二分；内层 continue 过滤 frontier 之后的已 finalize 项）。
        while self.div_done < divs_stable_len.min(divs.len())
            && divs[self.div_done].seg_c_end < self.stable_anchor
        {
            self.div_done += 1;
        }
        let mut tail_type1: Vec<BuySellPoint> = Vec::new();
        for div in &divs[self.div_done..] {
            if div.kind != DivKind::Trend || div.seg_c_end < self.stable_anchor {
                continue;
            }
            if let Some(bp) =
                build_type1_bsp(div, segs, zss, moves, self.level_id, self.require_settled)
            {
                tail_type1.push(bp);
            }
        }

        // ── 尾部 type2：源 type1.seg_idx >= src_from——
        //    stable_t1（已 finalize，seg_idx ∈ [src_from, stable_anchor)）+ tail_type1（seg_idx ≥
        //    stable_anchor），两段不相交并覆盖全部 seg_idx ≥ src_from 的 type1 源。
        let sstart = self.stable_bsps.partition_point(|bp| bp.seg_idx < src_from);
        let stable_t1: Vec<BuySellPoint> = self.stable_bsps[sstart..]
            .iter()
            .filter(|bp| bp.kind == BspKind::Type1)
            .copied()
            .collect();
        let mut tail_type2: Vec<BuySellPoint> = Vec::new();
        for t1 in stable_t1.iter().chain(tail_type1.iter()) {
            if let Some(bp) = build_type2_bsp(
                t1,
                segs,
                moves,
                self.level_id,
                &lookup_find,
                self.require_settled,
            ) {
                tail_type2.push(bp);
            }
        }

        // ── 尾部 type3：settled ∧ break_seg >= src_from（partition_point，break_seg 升序）──
        // zs_break[i] = (settled, break_dir, break_seg)；unsettled（break_seg=-1）仅末尾。
        let zstart = zs_break.partition_point(|(settled, _, bs)| *settled && *bs < src_from);
        let mut tail_type3: Vec<BuySellPoint> = Vec::new();
        for zi in zstart..zss.len() {
            let (settled, break_dir, break_seg) = zs_break[zi];
            if !settled || break_dir == BreakDir::None || break_seg < src_from {
                continue;
            }
            if let Some(bp) = build_type3_bsp(
                &zss[zi],
                break_dir,
                break_seg,
                segs,
                moves,
                self.level_id,
                &lookup_find,
                self.require_settled,
            ) {
                tail_type3.push(bp);
            }
        }

        // overlap（2B+3B）：尾部内重合（稳定区 type2/type3 已在 finalize 时检测过）。
        detect_overlap(&mut tail_type2, &mut tail_type3);

        // 合并尾部，按 seg_idx 稳定排序（复刻 buysellpoints_from_level 末尾 sort）。
        let mut tail: Vec<BuySellPoint> =
            Vec::with_capacity(tail_type1.len() + tail_type2.len() + tail_type3.len());
        tail.extend(tail_type1);
        tail.extend(tail_type2);
        tail.extend(tail_type3);
        tail.sort_by_key(|bp| bp.seg_idx);

        // finalize：[stable_anchor, new_anchor) 转永久；[new_anchor, ∞) 留尾部。
        // SRC_MARGIN ≥ new_anchor - stable_anchor 保证 [stable_anchor, new_anchor) 全被 tail 覆盖。
        let lo = tail.partition_point(|bp| bp.seg_idx < self.stable_anchor);
        let hi = tail.partition_point(|bp| bp.seg_idx < new_anchor);
        for bp in &tail[lo..hi] {
            self.stable_bsps.push(*bp);
        }
        self.stable_anchor = new_anchor;

        self.full_bsps.clear();
        self.full_bsps.extend_from_slice(&self.stable_bsps);
        self.full_bsps.extend_from_slice(&tail[hi..]);
    }

    /// 当前全量买卖点（逐位等价于 `buysellpoints_from_level(.., None div)`）。
    pub fn current(&self) -> &[BuySellPoint] {
        &self.full_bsps
    }
}

// ════════════════════════════════════════════════════════════
// 差分测试：逐前缀增量 == 全量（覆盖易变段尾维度）
// ════════════════════════════════════════════════════════════
#[cfg(test)]
mod incremental_tests {
    use super::*;
    use crate::divergence::divergences_from_moves_v1;
    use crate::moves::moves_from_zhongshus;
    use crate::segment::{segments_from_strokes_v1, MAX_SECOND_SEQ_SCAN};
    use crate::stroke::{Direction, Stroke};
    use crate::zhongshu::zhongshu_from_segments;

    /// 从极值价构造交替方向的笔（bi 不变量：方向逐笔交替）。
    fn mk_strokes(prices: &[f64]) -> Vec<Stroke> {
        let mut out = Vec::new();
        for k in 0..prices.len().saturating_sub(1) {
            let p0 = prices[k];
            let p1 = prices[k + 1];
            let direction = if p1 > p0 {
                Direction::Up
            } else {
                Direction::Down
            };
            out.push(Stroke {
                i0: k,
                i1: k + 1,
                direction,
                high: p0.max(p1),
                low: p0.min(p1),
                p0,
                p1,
                confirmed: true,
            });
        }
        out
    }

    /// 复刻 `SegCheckpoint._update_checkpoint` 的稳定判据，独立计算 stable_count。
    /// 一段稳定 ⟺ confirmed ∧ break_evidence ∧ trigger_k+1+MARGIN ≤ n_strokes（gap_type 无关）。
    fn compute_stable_count(segments: &[Segment], n_strokes: usize) -> usize {
        let mut sc = 0usize;
        for seg in segments {
            if !seg.confirmed {
                break;
            }
            let trigger_k = match seg.break_evidence {
                Some(be) => be.trigger_stroke_k,
                None => break,
            };
            if trigger_k + 1 + MAX_SECOND_SEQ_SCAN > n_strokes {
                break;
            }
            sc += 1;
        }
        sc
    }

    /// 线段级 SegView 镜像（复刻 orchestrator）。
    fn seg_views(segs: &[Segment]) -> Vec<SegView> {
        segs.iter()
            .map(|s| SegView {
                direction: s.direction,
                high: s.high,
                low: s.low,
                i0: s.i0,
                i1: s.i1,
                settled: s.confirmed,
            })
            .collect()
    }

    fn zs_views(zss: &[Zhongshu]) -> Vec<ZsView> {
        zss.iter()
            .map(|z| ZsView {
                zd: z.zd,
                zg: z.zg,
                seg_start: z.seg_start,
                seg_end: z.seg_end,
                settled: z.settled,
            })
            .collect()
    }

    fn move_views(mvs: &[crate::moves::Move]) -> Vec<MoveView> {
        mvs.iter()
            .map(|m| MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            })
            .collect()
    }

    fn assert_zs_eq(inc: &[Zhongshu], full: &[Zhongshu], prefix: usize) {
        assert_eq!(
            inc.len(),
            full.len(),
            "前缀{} 中枢数 inc={} full={}",
            prefix,
            inc.len(),
            full.len()
        );
        for (k, (a, b)) in inc.iter().zip(full.iter()).enumerate() {
            assert_eq!(
                a, b,
                "前缀{} 中枢[{}] 发散\n  inc ={:?}\n  full={:?}",
                prefix, k, a, b
            );
        }
    }

    fn div_key(d: &Divergence) -> (i64, i64, i64, i64, usize, u64, u64, bool) {
        (
            d.seg_a_start,
            d.seg_a_end,
            d.seg_c_start,
            d.seg_c_end,
            d.center_idx,
            d.force_a.to_bits(),
            d.force_c.to_bits(),
            d.confirmed,
        )
    }

    fn assert_div_eq(inc: &[Divergence], full: &[Divergence], prefix: usize) {
        assert_eq!(
            inc.len(),
            full.len(),
            "前缀{} 背驰数 inc={} full={}",
            prefix,
            inc.len(),
            full.len()
        );
        for (k, (a, b)) in inc.iter().zip(full.iter()).enumerate() {
            assert_eq!(a.kind, b.kind, "前缀{} div[{}] kind", prefix, k);
            assert_eq!(a.direction, b.direction, "前缀{} div[{}] dir", prefix, k);
            assert_eq!(div_key(a), div_key(b), "前缀{} div[{}] 字段", prefix, k);
        }
    }

    /// 逐前缀驱动：模拟 orchestrator 逐 bar 喂入增长的笔列表，比对增量 vs 全量。
    fn assert_inc_matches_full(strokes: &[Stroke]) {
        let mut inc_zs = IncrementalSegZhongshu::new();
        let mut inc_div = IncrementalSegDivergences::new();

        for m in 3..=strokes.len() {
            let s = &strokes[..m];
            let segments = segments_from_strokes_v1(s, 3, true);
            let stable_count = compute_stable_count(&segments, s.len());

            // ── 中枢层 ──
            inc_zs.update(&segments, stable_count);
            let full_zs = zhongshu_from_segments(
                &segments
                    .iter()
                    .map(|x| {
                        (
                            x.s0,
                            x.s1,
                            x.high,
                            x.low,
                            x.confirmed,
                            x.kind == SegKind::Settled,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            assert_zs_eq(inc_zs.zhongshus(), &full_zs, m);

            // ── 背驰层（用增量中枢输出驱动 moves，再比对增量背驰 vs 全量）──
            let full_moves = moves_from_zhongshus(inc_zs.zhongshus(), Some(segments.len()));
            let segv = seg_views(&segments);
            let zsv = zs_views(inc_zs.zhongshus());
            let mvv = move_views(&full_moves);
            inc_div.update(&mvv, &segv, &zsv, 1, segments.len());
            let full_div = divergences_from_moves_v1(&segv, &zsv, &mvv, 1, None);
            assert_div_eq(inc_div.current(), &full_div, m);
        }
    }

    #[test]
    fn matches_full_equal_zigzag() {
        let prices: Vec<f64> = (0..50)
            .map(|k| if k % 2 == 0 { 100.0 } else { 110.0 })
            .collect();
        assert_inc_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn matches_full_trending_breaks() {
        let mut prices = Vec::new();
        let mut base = 100.0;
        for k in 0..80 {
            if k % 2 == 0 {
                prices.push(base);
            } else {
                prices.push(base + 12.0);
                base += 4.0;
            }
        }
        assert_inc_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn matches_full_lcg_stress() {
        // 确定性 LCG zigzag：随机游走中心 + 随机半幅，覆盖 overlap/break/extend/缺口/
        // 第二特征序列各路径。500 笔触发多次稳定前缀推进 + 易变尾翻转。
        let mut state: u64 = 0xD1B54A32D192ED03;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64
        };
        let mut prices = Vec::new();
        let mut center = 100.0_f64;
        for k in 0..500 {
            center += (next() - 0.5) * 6.0;
            let half = 1.0 + next() * 7.0;
            if k % 2 == 0 {
                prices.push(center - half);
            } else {
                prices.push(center + half);
            }
        }
        assert_inc_matches_full(&mk_strokes(&prices));
    }

    #[test]
    fn matches_full_lcg_seeds() {
        for seed in [1u64, 42, 0xFFFF_FFFF, 0x9E37_79B9_7F4A_7C15] {
            let mut state = seed;
            let mut next = || {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 33) as f64 / (1u64 << 31) as f64
            };
            let mut prices = Vec::new();
            let mut center = 100.0_f64;
            for k in 0..300 {
                center += (next() - 0.5) * 8.0;
                let half = 1.0 + next() * 6.0;
                if k % 2 == 0 {
                    prices.push(center - half);
                } else {
                    prices.push(center + half);
                }
            }
            assert_inc_matches_full(&mk_strokes(&prices));
        }
    }
}

// ════════════════════════════════════════════════════════════
// 性能 profile（IncrementalSegZhongshu 标度 — scan_i 停滞 regime 的 O(n²) 定位）
// ════════════════════════════════════════════════════════════
//
// 递增笔规模，每次 segments_from_strokes_v1 构造线段 + IncrementalSegZhongshu::update，
// 测累计耗时，拟合标度。trend regime（无 settled 中枢）触发 scan_i 停滞 → 期望 O(n²)（优化前）。
//
// L1 度量（231号）：CPU 耗时是确定性工程度量，零信息增量。
// 跑法：cargo test --lib segment_layers::perf_profile -- --ignored --nocapture
#[cfg(test)]
mod perf_profile {
    use super::*;
    use crate::segment::segments_from_strokes_v1;
    use crate::stroke::{Direction, Stroke};
    use std::time::Instant;

    const SIZES: [usize; 5] = [500, 1000, 2000, 4000, 8000];

    fn mk_strokes(prices: &[f64]) -> Vec<Stroke> {
        let mut out = Vec::new();
        for k in 0..prices.len().saturating_sub(1) {
            let p0 = prices[k];
            let p1 = prices[k + 1];
            let direction = if p1 > p0 {
                Direction::Up
            } else {
                Direction::Down
            };
            out.push(Stroke {
                i0: k,
                i1: k + 1,
                direction,
                high: p0.max(p1),
                low: p0.min(p1),
                p0,
                p1,
                confirmed: true,
            });
        }
        out
    }

    /// trend：单调上行 prices → 笔单调 → 线段单调 → 无三段重叠 → 无 settled 中枢。
    fn gen_trend(n: usize) -> Vec<Stroke> {
        let prices: Vec<f64> = (0..n).map(|k| 100.0 + k as f64).collect();
        mk_strokes(&prices)
    }

    /// 驱动：逐前缀 m 调 update，测累计耗时（隔离 update，不含 segments 构造）。
    /// `full_stable`：true=stable_count=segs.len()（全永久）；false=真实 stable_count
    /// （复刻 SegCheckpoint 稳定判据，有易变尾，接近生产配置）。
    fn drive_update(strokes: &[Stroke], full_stable: bool) -> (std::time::Duration, usize) {
        // 预构造所有前缀的 segments + stable_count（排除构造耗时）。
        let all: Vec<(Vec<Segment>, usize)> = (3..=strokes.len())
            .step_by(2)
            .map(|m| {
                let s = &strokes[..m];
                let segs = segments_from_strokes_v1(s, 3, true);
                let sc = if full_stable {
                    segs.len()
                } else {
                    compute_stable_count(&segs, s.len())
                };
                (segs, sc)
            })
            .collect();
        let t = Instant::now();
        let mut inc = IncrementalSegZhongshu::new();
        let mut last_len = 0usize;
        for (segs, sc) in &all {
            inc.update(segs, *sc);
            last_len = inc.zhongshus().len();
        }
        (t.elapsed(), std::hint::black_box(last_len))
    }

    /// 复刻 SegCheckpoint._update_checkpoint 稳定判据（同 incremental_tests）。
    fn compute_stable_count(segments: &[Segment], n_strokes: usize) -> usize {
        use crate::segment::MAX_SECOND_SEQ_SCAN;
        let mut sc = 0usize;
        for seg in segments {
            if !seg.confirmed {
                break;
            }
            let trigger_k = match seg.break_evidence {
                Some(be) => be.trigger_stroke_k,
                None => break,
            };
            if trigger_k + 1 + MAX_SECOND_SEQ_SCAN > n_strokes {
                break;
            }
            sc += 1;
        }
        sc
    }

    #[test]
    #[ignore = "性能 profile：cargo test --lib segment_layers::perf_profile -- --ignored --nocapture"]
    fn profile_incremental_seg_zhongshu_scaling() {
        eprintln!("\n===== segment_layers IncrementalSegZhongshu 标度 profile =====");
        for (name, gen) in [("trend(无中枢)", gen_trend as fn(usize) -> Vec<Stroke>)] {
            for (cfg, full) in [("全stable", true), ("真实stable(有易变尾)", false)] {
                eprintln!("\n── regime={name} / {cfg} ──");
                let mut rows: Vec<(usize, f64)> = Vec::new();
                for &n in &SIZES {
                    let strokes = gen(n);
                    let mut best = f64::INFINITY;
                    let mut zs = 0usize;
                    for _ in 0..3 {
                        let (d, l) = drive_update(&strokes, full);
                        best = best.min(d.as_secs_f64());
                        zs = l;
                    }
                    rows.push((n, best));
                    eprintln!("  n={n:>6}  total={best:>10.6}s  zs={zs}");
                }
                for w in rows.windows(2) {
                    let exp = (w[1].1 / w[0].1).ln() / (w[1].0 as f64 / w[0].0 as f64).ln();
                    eprintln!("    inc {}→{}: exp≈{exp:.2}", w[0].0, w[1].0);
                }
                let (n0, n1) = (rows[0].0 as f64, rows[rows.len() - 1].0 as f64);
                let (t0, t1) = (rows[0].1, rows[rows.len() - 1].1);
                let overall = (t1 / t0).ln() / (n1 / n0).ln();
                eprintln!("    overall: exp≈{overall:.2}  (1=线性 2=平方)");
            }
        }
    }
}
