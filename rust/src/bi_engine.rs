//! 笔事件引擎 — 逐 bar 驱动的增量引擎。
//! 逐位等价移植自 src/newchan/bi_engine.py 的 `BiEngine`。
//!
//! 移植边界（严格声明）：本模块复刻 **strokes 列表** 的增量计算（包含处理 →
//! 分型 → checkpoint two-pointer 笔构造），输出逐位等价于 Python 版的
//! `BiEngineSnapshot.strokes`。**事件流（events / event_id 哈希）不在本模块范围**——
//! event_id 依赖 Python `json.dumps` 的 float canonical 字符串化 + sha256，逐字节
//! 复刻脆弱，作为独立后续层处理（见 docs/architecture 移植路线）。

use crate::fractal::{classify_fractal, Fractal};
use crate::stroke::{
    build_stroke, check_gap, extend_prev_stroke, is_more_extreme, validate_direction, Stroke,
};

/// 包含处理的方向状态。对应 Python `_merge_dir_state` 的 "UP"/"DOWN"/None。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MergeDir {
    Up,
    Down,
}

/// 合并后的 K 线。对应 Python merge_buf 元素 `[o, h, l, c, raw_start, raw_end]`。
#[derive(Debug, Clone, Copy)]
struct MergeBar {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    /// 对齐 Python merge_buf[4]；raw 起点的读取统一走 merged_to_raw，故此字段
    /// 仅为结构完整性保留（Python 侧同样从不读 merge_buf[4]）。
    #[allow(dead_code)]
    raw_start: usize,
    raw_end: usize,
}

/// 检测 merge_buf 末尾三根是否形成分型模式。移植自 `a_inclusion._is_fractal_pattern`。
fn is_fractal_pattern(buf: &[MergeBar]) -> bool {
    let length = buf.len();
    if length < 3 {
        return false;
    }
    let (h_prev, l_prev) = (buf[length - 3].h, buf[length - 3].l);
    let (h_curr, l_curr) = (buf[length - 2].h, buf[length - 2].l);
    let (h_next, l_next) = (buf[length - 1].h, buf[length - 1].l);
    let is_top =
        h_curr > h_prev && h_curr > h_next && l_curr > l_prev && l_curr > l_next;
    let is_bottom =
        l_curr < l_prev && l_curr < l_next && h_curr < h_prev && h_curr < h_next;
    is_top || is_bottom
}

/// checkpoint two-pointer 推进一步。移植自 `BiEngine._advance_checkpoint_step`。
///
/// 写成自由函数以便从 `&mut self` 解构不相交字段（避免方法调用的整体借用）。
#[allow(clippy::too_many_arguments)]
fn advance_checkpoint_step(
    fxs: &[Fractal],
    cp_i: &mut usize,
    cp_j: &mut usize,
    cp_strokes: &mut Vec<Stroke>,
    highs: &[f64],
    lows: &[f64],
    use_new_bi: bool,
    min_gap: i64,
    merged_to_raw: &[(usize, usize)],
    new_raw_gap_min: i64,
) {
    let j = *cp_j;
    // Python: if j >= len(fxs) or j < 1
    if j >= fxs.len() || j < 1 {
        if j < fxs.len() {
            *cp_j = j + 1;
        }
        return;
    }

    let i = *cp_i;
    let start = fxs[i];
    let cand = fxs[j];

    if cand.kind == start.kind {
        if is_more_extreme(&cand, &start) {
            if !cp_strokes.is_empty() {
                extend_prev_stroke(cp_strokes, &cand, highs, lows);
            }
            *cp_i = j;
        }
        *cp_j = j + 1;
        return;
    }

    if !check_gap(&start, &cand, use_new_bi, min_gap, merged_to_raw, new_raw_gap_min) {
        *cp_j = j + 1;
        return;
    }

    let (direction, valid) = validate_direction(&start, &cand);
    if !valid {
        *cp_j = j + 1;
        return;
    }

    cp_strokes.push(build_stroke(&start, &cand, direction, highs, lows));
    *cp_i = j;
    *cp_j = j + 1;
}

/// 笔事件引擎。
pub struct BiEngine {
    // ── 参数 ──
    use_new_bi: bool,
    min_gap: i64,
    reset_dir_on_fractal: bool,
    new_raw_gap_min: i64,

    bar_idx: i64,

    // ── 增量 merge 状态 ──
    merge_buf: Vec<MergeBar>,
    merge_dir_state: Option<MergeDir>,
    // merged highs/lows（Python 用预分配 numpy；Rust 用 Vec，len == m_count）
    m_highs: Vec<f64>,
    m_lows: Vec<f64>,
    merged_to_raw: Vec<(usize, usize)>,

    // ── 增量 fractal 状态 ──
    confirmed_fractals: Vec<Fractal>,
    pending_fractal: Option<Fractal>,
    last_confirmed_m: usize,

    // ── 增量 stroke 状态（two-pointer 检查点）──
    fxs: Vec<Fractal>,
    cp_i: usize,
    cp_j: usize,
    cp_strokes: Vec<Stroke>,

    // ── 快照状态（原地维护，消除每变化全量拷贝稳定前缀的 O(strokes²)）──
    /// 最近一次 process_bar 后的笔快照（对应 Python BiEngineSnapshot.strokes）。
    /// 不可变冻结前缀 `[0, synced_prefix)` 逐位等于 `cp_strokes[..synced_prefix]`，
    /// 每变化只 truncate 旧尾部 + append 新冻结笔（摊还 O(1)）+ append 易变尾部（O(tail)）。
    snapshot: Vec<Stroke>,
    /// 已同步入 snapshot 的冻结前缀长度（= 上次的 base_len = cp_strokes.len()-1）。
    /// cp_strokes 前缀 `[..len-1]` 永久固定（仅 last 被 extend/push），故单调增不回退。
    synced_prefix: usize,
}

impl BiEngine {
    pub fn new(
        stroke_mode: &str,
        min_strict_sep: i64,
        reset_dir_on_fractal: bool,
        new_raw_gap_min: i64,
    ) -> Self {
        let use_new_bi = stroke_mode == "new";
        let min_gap = if stroke_mode == "wide" || stroke_mode == "new" {
            4
        } else {
            min_strict_sep
        };
        BiEngine {
            use_new_bi,
            min_gap,
            reset_dir_on_fractal,
            new_raw_gap_min,
            bar_idx: -1,
            merge_buf: Vec::new(),
            merge_dir_state: None,
            m_highs: Vec::new(),
            m_lows: Vec::new(),
            merged_to_raw: Vec::new(),
            confirmed_fractals: Vec::new(),
            pending_fractal: None,
            last_confirmed_m: 0,
            fxs: Vec::new(),
            cp_i: 0,
            cp_j: 1,
            cp_strokes: Vec::new(),
            snapshot: Vec::new(),
            synced_prefix: 0,
        }
    }

    pub fn bar_count(&self) -> i64 {
        self.bar_idx + 1
    }

    pub fn current_strokes(&self) -> &[Stroke] {
        self.snapshot.as_slice()
    }

    /// merged bar → raw bar 范围映射。MACD 背驰路径（raw 索引对齐）需要。
    /// 对应 Python `BiEngineSnapshot.merged_to_raw`。
    pub fn merged_to_raw(&self) -> &[(usize, usize)] {
        self.merged_to_raw.as_slice()
    }

    /// 重置增量状态到初始（保留参数）。对应 Python `BiEngine.reset`，用于回放 seek。
    /// 字段初值与 `new()` 一致（params 不变）。
    pub fn reset(&mut self) {
        self.bar_idx = -1;
        self.merge_buf.clear();
        self.merge_dir_state = None;
        self.m_highs.clear();
        self.m_lows.clear();
        self.merged_to_raw.clear();
        self.confirmed_fractals.clear();
        self.pending_fractal = None;
        self.last_confirmed_m = 0;
        self.fxs.clear();
        self.cp_i = 0;
        self.cp_j = 1;
        self.cp_strokes.clear();
        self.snapshot.clear();
        self.synced_prefix = 0;
    }

    // ================================================================
    // 增量包含处理 — O(1) per bar
    // ================================================================

    fn incremental_merge(&mut self, o: f64, h: f64, l: f64, c: f64) -> bool {
        let i_usize = self.bar_idx as usize;

        if self.merge_buf.is_empty() {
            self.merge_buf.push(MergeBar {
                o,
                h,
                l,
                c,
                raw_start: i_usize,
                raw_end: i_usize,
            });
            self.m_highs.push(h);
            self.m_lows.push(l);
            self.merged_to_raw.push((i_usize, i_usize));
            return true;
        }

        let last = self.merge_buf.last().unwrap();
        let last_h = last.h;
        let last_l = last.l;

        let has_inclusion =
            (last_h >= h && last_l <= l) || (h >= last_h && l <= last_l);

        if has_inclusion {
            let effective_up = match self.merge_dir_state {
                Some(MergeDir::Up) => true,
                Some(MergeDir::Down) => false,
                None => {
                    let last = self.merge_buf.last().unwrap();
                    last.c >= last.o
                }
            };
            let (new_h, new_l) = {
                let last = self.merge_buf.last_mut().unwrap();
                if effective_up {
                    last.h = last_h.max(h);
                    last.l = last_l.max(l);
                } else {
                    last.h = last_h.min(h);
                    last.l = last_l.min(l);
                }
                last.c = c;
                last.raw_end = i_usize;
                (last.h, last.l)
            };
            let mc = self.m_highs.len();
            self.m_highs[mc - 1] = new_h;
            self.m_lows[mc - 1] = new_l;
            let old_start = self.merged_to_raw[mc - 1].0;
            self.merged_to_raw[mc - 1] = (old_start, i_usize);
            return false;
        }

        let prev_dir = self.merge_dir_state;
        if h > last_h && l > last_l {
            self.merge_dir_state = Some(MergeDir::Up);
        } else if h < last_h && l < last_l {
            self.merge_dir_state = Some(MergeDir::Down);
        }
        self.merge_buf.push(MergeBar {
            o,
            h,
            l,
            c,
            raw_start: i_usize,
            raw_end: i_usize,
        });

        if self.reset_dir_on_fractal {
            if prev_dir.is_some() && self.merge_dir_state != prev_dir {
                self.merge_dir_state = None;
            } else if is_fractal_pattern(&self.merge_buf) {
                self.merge_dir_state = None;
            }
        }

        self.m_highs.push(h);
        self.m_lows.push(l);
        self.merged_to_raw.push((i_usize, i_usize));
        true
    }

    // ================================================================
    // 增量分型检测 — O(1) per bar
    // ================================================================

    fn update_fractals(&mut self) -> bool {
        let m = self.m_highs.len();
        if m < 3 {
            return false;
        }

        let mut changed = false;

        if m > self.last_confirmed_m {
            if let Some(pf) = self.pending_fractal {
                self.confirmed_fractals.push(pf);
                self.add_confirmed_to_fxs(pf);
                changed = true;
            }
            self.last_confirmed_m = m;
        }

        // **S5（T6 分型前提=包含处理先于分型判定）运行时证明**：分型判定的三根 merged bar
        // 相邻两两**非包含**（incremental_merge 已把包含关系合并为 staircase——T6"相邻两K线
        // 范围互含则方向不可辨，必须先合并"，docs/necessity_derivation.md:180）。包含残留 =
        // 合并 bug ⇒ 错误分型污染笔/线段/中枢/走势/BSP 全递归层。包含语义同 incremental_merge
        // （本文件 line 244）。violation = panic（这是 T6 的非平凡内容；fractal.rs 的顶/底互斥
        // assert 是定义一致性，此处才验包含处理）。
        let non_inclusive = |ah: f64, al: f64, bh: f64, bl: f64| {
            !((ah >= bh && al <= bl) || (bh >= ah && bl <= al))
        };
        assert!(
            non_inclusive(self.m_highs[m - 3], self.m_lows[m - 3], self.m_highs[m - 2], self.m_lows[m - 2])
                && non_inclusive(self.m_highs[m - 2], self.m_lows[m - 2], self.m_highs[m - 1], self.m_lows[m - 1]),
            "S5(T6) 违反@merged {}：分型三根 merged bar 存在包含残留（合并未完成，包含处理 bug）",
            m - 2
        );
        let new_pending = classify_fractal(
            self.m_highs[m - 3],
            self.m_highs[m - 2],
            self.m_highs[m - 1],
            self.m_lows[m - 3],
            self.m_lows[m - 2],
            self.m_lows[m - 1],
            m - 2,
        );
        if new_pending != self.pending_fractal {
            changed = true;
        }
        self.pending_fractal = new_pending;
        changed
    }

    // ================================================================
    // 增量笔构造 — checkpoint two-pointer
    // ================================================================

    fn add_confirmed_to_fxs(&mut self, fx: Fractal) {
        if self.fxs.is_empty() {
            self.fxs.push(fx);
            return;
        }

        let last = *self.fxs.last().unwrap();
        if fx.kind == last.kind {
            if is_more_extreme(&fx, &last) {
                *self.fxs.last_mut().unwrap() = fx;
            }
            return;
        }

        // 解构不相交字段，复刻 _advance_checkpoint_step。
        advance_checkpoint_step(
            &self.fxs,
            &mut self.cp_i,
            &mut self.cp_j,
            &mut self.cp_strokes,
            &self.m_highs,
            &self.m_lows,
            self.use_new_bi,
            self.min_gap,
            &self.merged_to_raw,
            self.new_raw_gap_min,
        );
        self.fxs.push(fx);
    }

    /// 从检查点恢复，处理 fxs 尾部 + pending，返回 `(base_len, tail, is_empty)`：
    /// - `base_len` = 不可变冻结前缀长度（= cp_strokes.len()-1，或 0）。
    /// - `tail` = 易变尾部笔（首元素为可能被 extend 的 cp 末笔 + 后续新建笔，末笔已置未确认）。
    /// - `is_empty` = true ⟹ 完整结果为空（语义同原 `Arc::new(Vec::new())`）。
    ///
    /// 完整笔序列 ≡ `cp_strokes[..base_len] ++ tail`（is_empty 时为空）。调用方
    /// （process_bar）据此原地维护 snapshot——只 append 新冻结前缀笔 + 重建尾部，
    /// 消除原 `cp[..base_len].to_vec()` 每变化 O(base_len) 全量拷贝的 O(strokes²)。
    /// 移植自 `_compute_strokes_from_checkpoint`（逐位等价，仅去掉全量物化）。
    fn compute_tail_from_checkpoint(&self) -> (usize, Vec<Stroke>, bool) {
        let pending = self.pending_fractal;
        let fxs = &self.fxs;
        let n_fxs = fxs.len();

        // tail_fxs 的逻辑表示（避免每 bar clone 整个 fxs；只有第二分支需替换末元素）。
        // 对应 Python 的四分支 tail_fxs 赋值。
        enum TailMode {
            /// tail_fxs = fxs（含 pending None、kind 不同、相同但不更极端三种 Python 分支）
            Fxs,
            /// tail_fxs = list(fxs); tail_fxs[-1] = pending
            FxsLastReplaced(Fractal),
            /// tail_fxs = [pending]（fxs 空且 pending 存在）
            SinglePending(Fractal),
        }

        let mode = if pending.is_none() || n_fxs == 0 {
            if pending.is_some() && n_fxs == 0 {
                TailMode::SinglePending(pending.unwrap())
            } else {
                TailMode::Fxs
            }
        } else {
            let last = fxs[n_fxs - 1];
            let p = pending.unwrap();
            if p.kind == last.kind && is_more_extreme(&p, &last) {
                TailMode::FxsLastReplaced(p)
            } else {
                TailMode::Fxs
            }
        };

        let tail_fxs_len = match mode {
            TailMode::SinglePending(_) => 1,
            _ => n_fxs,
        };

        let append_pending = pending.is_some()
            && n_fxs > 0
            && pending.unwrap().kind != fxs[n_fxs - 1].kind;

        let total_len = tail_fxs_len + if append_pending { 1 } else { 0 };

        if total_len < 2 {
            return (0, Vec::new(), true);
        }

        // get_fx(idx)：复刻 `tail_fxs[idx] if idx < len(tail_fxs) else pending`。
        let get_fx = |idx: usize| -> Fractal {
            match mode {
                TailMode::SinglePending(p) => p,
                TailMode::FxsLastReplaced(p) => {
                    if idx < n_fxs {
                        if idx == n_fxs - 1 {
                            p
                        } else {
                            fxs[idx]
                        }
                    } else {
                        pending.unwrap()
                    }
                }
                TailMode::Fxs => {
                    if idx < n_fxs {
                        fxs[idx]
                    } else {
                        pending.unwrap()
                    }
                }
            }
        };

        let mut i = self.cp_i;
        let mut j = self.cp_j;
        let cp = &self.cp_strokes;
        let cp_len = cp.len();
        let highs = &self.m_highs[..];
        let lows = &self.m_lows[..];

        // 尾部缓冲：仅含可能被修改的最后一笔 + 后续新增笔。
        let (mut tail, base_len): (Vec<Stroke>, usize) = if cp_len > 0 {
            (vec![cp[cp_len - 1]], cp_len - 1)
        } else {
            (Vec::new(), 0)
        };

        while j < total_len {
            if j < 1 {
                j = 1;
                continue;
            }

            let cand_fx = get_fx(j);
            let start_fx = get_fx(i);

            if cand_fx.kind == start_fx.kind {
                if is_more_extreme(&cand_fx, &start_fx) {
                    if !tail.is_empty() {
                        extend_prev_stroke(&mut tail, &cand_fx, highs, lows);
                    }
                    i = j;
                }
                j += 1;
                continue;
            }

            if !check_gap(
                &start_fx,
                &cand_fx,
                self.use_new_bi,
                self.min_gap,
                &self.merged_to_raw,
                self.new_raw_gap_min,
            ) {
                j += 1;
                continue;
            }

            let (direction, valid) = validate_direction(&start_fx, &cand_fx);
            if !valid {
                j += 1;
                continue;
            }

            tail.push(build_stroke(&start_fx, &cand_fx, direction, highs, lows));
            i = j;
            j += 1;
        }

        // 最后一笔置为未确认（与原全量路径一致）。
        if let Some(last) = tail.last().copied() {
            if last.confirmed {
                let n = tail.len();
                tail[n - 1] = Stroke {
                    confirmed: false,
                    ..last
                };
            }
        }

        if tail.is_empty() {
            // tail 空 ⟺ cp_len==0 ⟺ base_len==0（cp 非空时 tail 必含末笔）。
            return (0, Vec::new(), true);
        }

        // 完整序列 = cp_strokes[..base_len] ++ tail；由 process_bar 原地物化（无全量拷贝）。
        (base_len, tail, false)
    }

    // ================================================================
    // 主入口
    // ================================================================

    pub fn process_bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        self.bar_idx += 1;

        let _new_merged = self.incremental_merge(o, h, l, c);
        let fractals_changed = self.update_fractals();

        if !fractals_changed {
            // 分型未变 ⟹ 笔序列恒不变（checkpoint 输入不变）→ snapshot 原样保留（O(1)）。
            return;
        }

        let (base_len, tail, is_empty) = self.compute_tail_from_checkpoint();

        if is_empty {
            // 完整结果为空（语义同原 Arc::new(Vec::new())）：仅出现在 cp 为空的早期，
            // 此时 synced_prefix 必为 0；clear 与原全量空列表逐位等价。
            self.snapshot.clear();
            self.synced_prefix = 0;
            return;
        }

        // ── 原地物化 snapshot = cp_strokes[..base_len] ++ tail（摊还 O(tail)）──
        // 不变量：snapshot[..synced_prefix] ≡ cp_strokes[..synced_prefix]（冻结前缀）。
        // base_len 单调增（cp_strokes 仅 push/extend-last）⟹ base_len ≥ synced_prefix。
        debug_assert!(base_len >= self.synced_prefix);
        self.snapshot.truncate(self.synced_prefix); // 丢弃上次易变尾部，保留冻结前缀
        if base_len > self.synced_prefix {
            // 新近冻结的前缀笔（上次的易变尾部已固定）：append（Σ over run = O(n_strokes)）。
            self.snapshot
                .extend_from_slice(&self.cp_strokes[self.synced_prefix..base_len]);
            self.synced_prefix = base_len;
        }
        self.snapshot.extend(tail); // 易变尾部（末笔已置未确认）
    }
}
