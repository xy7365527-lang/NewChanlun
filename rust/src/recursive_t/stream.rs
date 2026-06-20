//! **T 算子全 Rust 流式驱动器**（方案 B 精确重跑）——NautilusTrader on_bar 逐 bar 喂 OHLC。
//!
//! 与现有 `UnnStream`/`SpiralStream`/`FugueV3Stream` 的范畴差：那三个的信号层在 Python
//! （`StreamingSignalReader.process_bar` → `push_signal`）。T 是 standalone Rust，故信号层 +
//! 仓位层**全部内聚 Rust**：`push_bar(o,h,l,c)` 只传 4 个 float，零复杂跨界 marshal。
//!
//! ## 真流式（方案 B 精确重跑，非方案 C 查表）
//! 逐 bar：
//! 1. `orch.process_bar(o,h,l,c)`（复用纯 Rust `RecursiveOrchestrator` 产笔/段）；
//! 2. **段门控**：笔增长（`stroke_count`）→ 数 confirmed&&settled 段，count 变 → 新 a₀；
//! 3. **重跑 T**：`build_a0_from_segments` + `iterate(a0, mode)` 产全塔（精确，非近似）；
//! 4. **diff 新增 BSP**：`tree.all_bsps()` vs `seen`（append-only 前缀冻结 ⟹ 只增不改）；
//! 5. 本 bar 新增 BSP **在当前 bar 投放**（确认时点，成交 close[i]）→ 喂 `TPositionEngine`。
//!
//! ## 比 batch `backtest.rs` 更严格（消除残留 look-ahead）
//! `backtest.rs` batch 在全历史段上一次跑 T，BSP 成交 @ 端点 `raw_end`（端点早于段确认时点 ⟹
//! 残留 look-ahead，模块头已标注）。本流式版 BSP 直到**段确认那根 bar** 才被算出/投放，成交
//! 用确认时点 close ⟹ 因果（无价格 / 时间 look-ahead，除不可消除的确认滞后本身就是因果的）。
//! 故流式结果**不**与 batch backtest.rs bit-exact——它们是不同口径回测器（流式 + 完整仓位）。
//!
//! ## 段门控触发频率（性能）
//! 重跑频率 = confirmed&&settled 段数（远低于 bar 数）。每次重跑 O(段数)（build_a0 含 O(n_bars)
//! MACD batch + iterate O(段数)），总 O(段数 × n_bars)。段门控把重跑从 bar 级降到段级。

use std::collections::HashSet;

use crate::fugue_v3::layer::FugueResult;
use crate::macd::OnlineMacdState;
use crate::segment::{Segment, SegKind};
use crate::stroke::Direction as StrokeDir;
use crate::trading::types::{Polarity, MAX_LADDER};

use super::iterate;
use super::t_engine::{TPositionEngine, TSignalView, BASE_LADDER};
use super::types::{BSPKind, Direction as TDir, PerfectionMode, Unit};
use crate::orchestrator::RecursiveOrchestrator;

/// stroke 方向 → recursive_t 方向（构造 a₀ Unit 时桥接）。
fn stroke_to_t_dir(d: StrokeDir) -> TDir {
    match d {
        StrokeDir::Up => TDir::Up,
        StrokeDir::Down => TDir::Down,
    }
}

/// 从线段构造 T 的 a₀（O(段数)，MACD 面积经**前缀和** O(1)/段查表）。
///
/// 与 batch `backtest::build_a0_from_segments`（每段 `macd_area_for_range` O(段长) + 每次重跑
/// `compute_macd_batch` O(n)）的差：流式用**增量 online MACD**（`OnlineMacdState`，O(1)/bar）+
/// pos/neg **前缀和**（`prefix_pos[i+1]−prefix_pos[i0]` = 区间面积 O(1)）⟹ 每次重跑 O(段数) 而非
/// O(n)，消除 O(段数×n) 瓶颈。online MACD 与 batch `compute_macd_batch` 路径不同（因果递推 vs
/// adjust=False EWM，见 macd.rs 模块文档）——流式引擎用 online（实时因果）是**更**正确选择，且
/// 流式/批量共享 `TFugueStreamCore` ⟹ 二者 bit-exact。Structural 模式不读 area，逐字不受影响。
///
/// 过滤口径 `confirmed && kind==Settled`（与 batch / v3 zhongshu_from_segments 一致）。
fn build_a0_fast(segs: &[Segment], m2r: &[(usize, usize)], prefix_pos: &[f64], prefix_neg: &[f64]) -> Vec<Unit> {
    let n = prefix_pos.len().saturating_sub(1); // raw bar 数（prefix 长 n+1）
    segs.iter()
        .filter(|s| s.confirmed && s.kind == SegKind::Settled)
        .map(|s| {
            // 段端点 merged → raw 区间 [raw_i0, raw_i1]（含端点），越界 clamp。
            let raw_i0 = m2r.get(s.i0).map(|&(lo, _)| lo).unwrap_or(0).min(n.saturating_sub(1));
            let raw_i1 = m2r.get(s.i1).map(|&(_, hi)| hi).unwrap_or(0).min(n.saturating_sub(1));
            let (lo, hi) = if raw_i0 <= raw_i1 { (raw_i0, raw_i1) } else { (raw_i1, raw_i0) };
            // 区间面积 = prefix[hi+1] − prefix[lo]（前缀和 O(1)）。
            let area_pos = prefix_pos[hi + 1] - prefix_pos[lo];
            let area_neg = prefix_neg[hi + 1] - prefix_neg[lo]; // 已存 |负 hist|，非负
            Unit {
                high: s.high,
                low: s.low,
                start_bar: s.i0 as i64,
                end_bar: s.i1 as i64,
                direction: stroke_to_t_dir(s.direction),
                level: 0,
                inner_zhongshu_count: 0,
                area_pos,
                area_neg,
            }
        })
        .collect()
}

/// BSPKind → 去重判别 u8（seen-set 身份键）。
fn bsp_disc(k: BSPKind) -> u8 {
    match k {
        BSPKind::Type1Buy => 0,
        BSPKind::Type1Sell => 1,
        BSPKind::Type2Buy => 2,
        BSPKind::Type2Sell => 3,
        BSPKind::Type3Buy => 4,
        BSPKind::Type3Sell => 5,
    }
}

/// T 流式引擎核心（orchestrator + T 重跑 + 仓位引擎，全 Rust 内聚）。
pub struct TFugueStreamCore {
    orch: RecursiveOrchestrator,
    mode: PerfectionMode,
    engine: TPositionEngine,
    /// 增量 online MACD（O(1)/bar，避免每次重跑 O(n) batch 重算）。
    macd: OnlineMacdState,
    /// MACD hist 正柱（红）累积**前缀和**（`prefix_pos[i]` = Σ max(hist[0..i],0)）。
    prefix_pos: Vec<f64>,
    /// MACD hist 负柱（绿）|累积| **前缀和**（`prefix_neg[i]` = Σ max(−hist[0..i],0)，非负）。
    prefix_neg: Vec<f64>,
    cur_bar: i64,
    finished: bool,

    // ── 段门控 + diff 状态 ──
    last_stroke_n: usize,
    /// 上次 a₀（confirmed&&settled）段数——变化 → 重跑。
    last_cs_segs: usize,
    /// 已投放 BSP 身份键 (kind_disc, merged_bar, level)（append-only diff，只增不改）。
    seen_bsps: HashSet<(u8, i64, usize)>,
    /// 自下而上涌现升级开关（消融门）：`T_NO_EMERGENCE` 环境变量置位时关闭——用于 L3 A/B
    /// 隔离 emergence-upgrade 的净效应（与 `BT_DUMP_TRADES`/`BT_SYMBOLS` 同类 eval 工具）。
    emergence_enabled: bool,
}

impl TFugueStreamCore {
    /// `mode`：步骤c 走势完美判定（Structural/And/Or）。orchestrator 配置与 batch
    /// `backtest_run.rs` 逐字一致（wide / min_strict_sep=5 / new_raw_gap_min=3，关 bsp/macd
    /// 下游层——segments bit-exact 且省 O(n²)）。
    pub fn new(mode: PerfectionMode) -> Self {
        TFugueStreamCore {
            orch: RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false),
            mode,
            engine: TPositionEngine::new(),
            macd: OnlineMacdState::new(12, 26, 9),
            prefix_pos: vec![0.0], // prefix[0] = 0（区间面积 = prefix[hi+1]−prefix[lo]）
            prefix_neg: vec![0.0],
            cur_bar: 0,
            finished: false,
            last_stroke_n: 0,
            last_cs_segs: 0,
            seen_bsps: HashSet::new(),
            emergence_enabled: std::env::var("T_NO_EMERGENCE").is_err(),
        }
    }

    /// 重跑 T，diff 出本 bar 新增 BSP → 写入 view（buy/sell by ladder）。
    ///
    /// **零 O(n) 克隆**：orch 的 `segments()`/`merged_to_raw()` 在块内**借用**（不 `.to_vec()`），
    /// 块内算出 tree + BSP 列表（owned），块结束释放 orch 借用。
    /// 新设计只需 buy/sell——删 root 后操作层无方向门/成本门/ascend（纯 BSP 驱动），故不再算
    /// 方向/锚/θ/ceiling/root（旧 root-based 设计的方向态残留）。
    fn rerun_and_diff(&mut self, view: &mut TSignalView) {
        // ── 块内借 orch（segs+m2r）+ prefix，算 tree + bsp 列表（owned，块后释放）──
        let (bsps, emergent) = {
            let segs = self.orch.segments();
            let m2r = self.orch.merged_to_raw();
            let a0 = build_a0_fast(segs, m2r, &self.prefix_pos, &self.prefix_neg);
            let tree = iterate(a0, self.mode);
            (tree.all_bsps(), tree.emergent_top())
        }; // orch 借用在此释放

        // ── 自下而上涌现上界 → (ladder, 操作极性)：向上走势=做多归属 / 向下走势=做空归属。
        //    写入 view.emergent_top，step 在 BSP 路由前据此把核心仓 relabel 升级归属（不等高级别 BSP）。
        //    消融门 `emergence_enabled`（T_NO_EMERGENCE）关闭时不写 ⟹ A/B 基线（退化为纯 BSP 驱动）。──
        if let (true, Some((t_level, dir))) = (self.emergence_enabled, emergent) {
            let ladder = t_level + BASE_LADDER;
            if ladder < MAX_LADDER {
                let pol = match dir {
                    TDir::Up => Polarity::Long,
                    TDir::Down => Polarity::Short,
                };
                view.emergent_top = Some((ladder, pol));
            }
        }

        // ── diff 新增 BSP（本 bar 新可见的买卖点；key 不依赖 m2r，块外安全）──
        for bsp in bsps {
            let ladder = bsp.level + BASE_LADDER;
            if ladder >= MAX_LADDER {
                continue;
            }
            let key = (bsp_disc(bsp.kind), bsp.bar, bsp.level);
            if !self.seen_bsps.insert(key) {
                continue; // 已投放，跳过
            }
            // 新设计不分 type1/2/3——任意买点→该级别买点，任意卖点→该级别卖点。
            if bsp.kind.is_buy() {
                view.buy[ladder] = true;
            } else {
                view.sell[ladder] = true;
            }
        }
    }

    /// 逐 bar 推送。返回本 bar **新增** trade 数（ffi 据此切片 result.trades）。
    pub fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64) -> usize {
        self.orch.process_bar(o, h, l, c);
        // 增量 MACD（O(1)）+ pos/neg 前缀和扩展（区间面积 O(1) 查表的基础）。
        let (_, _, hist) = self.macd.update(c);
        let last_pos = *self.prefix_pos.last().unwrap();
        let last_neg = *self.prefix_neg.last().unwrap();
        self.prefix_pos.push(last_pos + if hist > 0.0 { hist } else { 0.0 });
        self.prefix_neg.push(last_neg + if hist < 0.0 { -hist } else { 0.0 });
        let bar = self.cur_bar;

        let mut view = TSignalView::empty();

        // ── 段门控：笔增长 → 数 confirmed&&settled 段 → count 变则重跑 ──
        let sc = self.orch.strokes().len();
        if sc > self.last_stroke_n {
            self.last_stroke_n = sc;
            let n_cs = self
                .orch
                .segments()
                .iter()
                .filter(|s| s.confirmed && s.kind == SegKind::Settled)
                .count();
            if n_cs != self.last_cs_segs {
                self.last_cs_segs = n_cs;
                self.rerun_and_diff(&mut view);
            }
        }

        let before = self.engine.n_trades();
        self.engine.step(&view, bar, c);
        self.cur_bar += 1;
        self.engine.n_trades() - before
    }

    /// 收尾（eod 清仓 + final_nav）。幂等。
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let last_bar = if self.cur_bar > 0 { Some(self.cur_bar - 1) } else { None };
        self.engine.finish(last_bar);
    }

    pub fn result(&self) -> &FugueResult {
        self.engine.result()
    }

    /// BSP 操作诊断快照（纯观测：逐 BSP 操作类型计数 + 核心 units 变化）。
    pub fn op_diag(&self) -> super::t_engine::BspOpDiag {
        self.engine.op_diag()
    }

    pub fn n_trades(&self) -> usize {
        self.engine.n_trades()
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        let (navv, lu, su, voices) = self.engine.snapshot();
        (self.cur_bar, navv, lu, su, voices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 上涨后回落的合成序列：流式喂应零 panic（守恒守卫每 bar）+ 产出结果。
    #[test]
    fn 流式合成序列零panic() {
        let mut core = TFugueStreamCore::new(PerfectionMode::Structural);
        // 锯齿上行 → 顶 → 下行，制造笔/段/中枢。
        let mut price = 100.0;
        let mut up = true;
        for i in 0..400 {
            if i % 20 == 0 {
                up = !up;
            }
            price += if up { 1.0 } else { -0.8 };
            let c = price;
            let o = c - 0.1;
            let h = c + 0.5;
            let l = c - 0.5;
            core.push_bar(o, h, l, c);
        }
        core.finish();
        // 能产出结果（零 panic ⟹ 守恒/NAV 中性/互斥全程成立）。
        let _ = core.result().final_nav;
        assert!(core.result().final_nav.is_finite(), "final_nav 有限");
    }

    #[test]
    fn 流式零bar不panic() {
        let mut core = TFugueStreamCore::new(PerfectionMode::And);
        core.finish();
        assert_eq!(core.n_trades(), 0);
    }
}
