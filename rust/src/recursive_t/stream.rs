//! **T 算子全 Rust 流式驱动器**（方案 B 精确重跑）——NautilusTrader on_bar 逐 bar 喂 OHLC。
//!
//! 与现有 `UnnStream`/`SpiralStream`/`FugueV3Stream` 的范畴差：那三个的信号层在 Python
//! （`StreamingSignalReader.process_bar` → `push_signal`）。T 是 standalone Rust，故信号层 +
//! 仓位层**全部内聚 Rust**：`push_bar(o,h,l,c)` 只传 4 个 float，零复杂跨界 marshal。
//!
//! GUARD-ROLE: t-engine-flat-branch-live-python-caller
//!
//! ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#762 C7-E3 执行票核定）
//!
//! - **名分**：**现役**（机械判据：有非测试调用者 ∧ 无 `#[deprecated]` 标记 ∧ 在唯一 git 线
//!   main 上）。名分表（`.chanlun/review-results/legacy-generation-census-20260729.md` §1.3(b)/§2）
//!   把 recursive_t/(b) T 引擎（12062 行）整体判"deprecated 待退役"，未区分内部两条互不相通的
//!   子分支。本票逐文件核查发现 (b) 实际是两个独立子簇，名分不同：**本文件（`stream.rs`，
//!   `TFugueStreamCore`）+ `t_engine.rs`（`TPositionEngine`）+ `prove_guards.rs`（共享守卫）+
//!   `ffi.rs::PyTFugueStream`** 这一支有真实非测试调用者——`ffi.rs:16`
//!   `use super::stream::TFugueStreamCore;`（生产路径，PyTFugueStream 的核心委托对象）+
//!   `trading_system/backtest_t_fugue.py:166` `nr.TFugueStream(self.config.mode)`（`git log -1`
//!   = 2026-06-21，冷但真实，文件仍在主线树）。
//! - **对照什么**：与同目录另一支（`rec_engine.rs`/`rec_stream.rs`/`rec_driver.rs` +
//!   `ffi.rs::PyRecStream`，见 `rec_engine.rs` 头部 GUARD-ROLE 块）对照：**评审 FAIL 后订正
//!   ——那一支同样现役**，非本文件此前所述的"deprecated"。原判"全仓零 python 调用方"是按
//!   Rust 内部名 `RecStream` grep 得出，而 PyO3 导出名是 `RecTStream`（`ffi.rs:291` rename），
//!   按导出名反查有 4 处真实 python 调用者（含 NT 生产策略 `rec_t_strategy.py`）。两支的
//!   真实差异是**互不调用**（本支走 `TFugueStream`/`stream.rs`，另一支走
//!   `RecTStream`/`rec_stream.rs`，各自独立可达 python），不是"一支现役一支 deprecated"。
//!   仅 `backtest.rs`/`backtest_run.rs`（`#[cfg(test)]` 模块级门控，`t_engine_run.rs` 是测试
//!   本支的独立 harness）生产零可达 ∧ 仅测试调用者，维持 deprecated 待退役——不可整
//!   12062 行文件集合笼统判定，须按"flat 支 / rec 支 / backtest 独立核"三分处理。
//! - **与现役差在哪**：π（theta_v0）完全自包含未复用本支一行（§0 全局核查复验一致），本支的
//!   "现役"仅指"仍有非测试调用者"，非"π 生产路径"。继续挂起等待批次统一处置。
//! - **禁回灌**：本次仅加标记，未删除/未移动任何代码。
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
use crate::segment::{SegKind, Segment};
use crate::stroke::{Direction as StrokeDir, Stroke};
use crate::trading::types::{Polarity, MAX_LADDER};

use super::iterate;
use super::t_engine::{TPositionEngine, TSignalView, BASE_LADDER};
use super::types::{A0Source, BSPKind, Direction as TDir, PerfectionMode, Unit};
use crate::orchestrator::RecursiveOrchestrator;

/// stroke 方向 → recursive_t 方向（构造 a₀ Unit 时桥接）。
fn stroke_to_t_dir(d: StrokeDir) -> TDir {
    match d {
        StrokeDir::Up => TDir::Up,
        StrokeDir::Down => TDir::Down,
    }
}

/// 从 a₀ 来源（线段 / 笔）构造 T 的 a₀（O(单元数)，MACD 面积经**前缀和** O(1)/单元查表）。
///
/// 与 batch `backtest::build_a0_from_segments`（每单元 `macd_area_for_range` O(单元长) + 每次重跑
/// `compute_macd_batch` O(n)）的差：流式用**增量 online MACD**（`OnlineMacdState`，O(1)/bar）+
/// pos/neg **前缀和**（`prefix_pos[i+1]−prefix_pos[i0]` = 区间面积 O(1)）⟹ 每次重跑 O(单元数) 而非
/// O(n)，消除 O(单元数×n) 瓶颈。online MACD 与 batch `compute_macd_batch` 路径不同（因果递推 vs
/// adjust=False EWM，见 macd.rs 模块文档）——流式引擎用 online（实时因果）是**更**正确选择，且
/// 流式/批量共享 `TFugueStreamCore` ⟹ 二者 bit-exact。Structural 模式不读 area，逐字不受影响。
///
/// `source`（谱系 526 / 第65课 065:182「区别仅在 a0」）：
/// - `A0Source::Segment`：过滤 `confirmed && kind==Settled`（与 batch / v3 zhongshu_from_segments
///   一致）——现状默认，逐字 bit-exact（Settled = 线段特征序列递归确认语义，第67/71课）。
/// - `A0Source::Stroke`：过滤 `confirmed`（笔无 Settled 语义，bi.md:151「最后一笔 confirmed=False，
///   直到下一笔生成后才结算」）——递归底座下移（525/526号，级别数 5-6→8-10 主因）。
///
/// `include_last_candidate`（编排者 2026-06-21 区间套提前确认）：true 时额外含**最后一个未确认
/// 单元**（c 段顶部附近、尚未确认）——让操作点提前到顶部。batch/flat 路径传 false 保 bit-exact。
/// 笔/段对称：Segment 找最后 `!confirmed` 段，Stroke 找最后 `!confirmed` 笔。
pub(crate) fn build_a0_fast(
    segs: &[Segment],
    strokes: &[Stroke],
    m2r: &[(usize, usize)],
    prefix_pos: &[f64],
    prefix_neg: &[f64],
    source: A0Source,
    include_last_candidate: bool,
) -> Vec<Unit> {
    let n = prefix_pos.len().saturating_sub(1); // raw bar 数（prefix 长 n+1）
                                                // 共享单元构造：(i0,i1,high,low,dir merged 端点) → Unit{level:0}，面积经 merged→raw 前缀和 O(1)。
                                                // 线段端点 i0/i1 与笔端点 i0/i1 同为 merged bar 坐标（笔 i0/i1 = 分型中心 df_merged iloc，
                                                // 见 stroke.rs:27），故面积/坐标换算口径逐字一致——a₀ 来源差异仅在过滤口径，不在单元构造。
    let to_unit = |i0: usize, i1: usize, high: f64, low: f64, dir: StrokeDir| -> Unit {
        let raw_i0 = m2r
            .get(i0)
            .map(|&(lo, _)| lo)
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        let raw_i1 = m2r
            .get(i1)
            .map(|&(_, hi)| hi)
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        let (lo, hi) = if raw_i0 <= raw_i1 {
            (raw_i0, raw_i1)
        } else {
            (raw_i1, raw_i0)
        };
        // 区间面积 = prefix[hi+1] − prefix[lo]（前缀和 O(1)）。
        let area_pos = prefix_pos[hi + 1] - prefix_pos[lo];
        let area_neg = prefix_neg[hi + 1] - prefix_neg[lo]; // 已存 |负 hist|，非负
        Unit {
            high,
            low,
            start_bar: i0 as i64,
            end_bar: i1 as i64,
            direction: stroke_to_t_dir(dir),
            level: 0,
            inner_zhongshu_count: 0,
            area_pos,
            area_neg,
        }
    };
    match source {
        A0Source::Segment => {
            let mut a0: Vec<Unit> = segs
                .iter()
                .filter(|s| s.confirmed && s.kind == SegKind::Settled)
                .map(|s| to_unit(s.i0, s.i1, s.high, s.low, s.direction))
                .collect();
            if include_last_candidate {
                // 最后一个**未确认**段（confirmed=false=当前形成中=顶部附近）→ 提前定位转折点。
                if let Some(cand) = segs.iter().rev().find(|s| !s.confirmed) {
                    a0.push(to_unit(
                        cand.i0,
                        cand.i1,
                        cand.high,
                        cand.low,
                        cand.direction,
                    ));
                }
            }
            a0
        }
        A0Source::Stroke => {
            // 笔完成口径 = confirmed（bi.md:151；笔无 Settled 递归确认语义，525/526号）。
            let mut a0: Vec<Unit> = strokes
                .iter()
                .filter(|s| s.confirmed)
                .map(|s| to_unit(s.i0, s.i1, s.high, s.low, s.direction))
                .collect();
            if include_last_candidate {
                if let Some(cand) = strokes.iter().rev().find(|s| !s.confirmed) {
                    a0.push(to_unit(
                        cand.i0,
                        cand.i1,
                        cand.high,
                        cand.low,
                        cand.direction,
                    ));
                }
            }
            a0
        }
    }
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

    /// a₀ 来源（线段=默认 bit-exact / 笔=递归底座下移，526号）。决定门控计数口径与 build_a0 过滤。
    a0_source: A0Source,

    // ── 段门控 / 笔门控 + diff 状态 ──
    last_stroke_n: usize,
    /// 上次 a₀ 单元数（Segment=confirmed&&settled 段数 / Stroke=confirmed 笔数）——变化 → 重跑。
    last_a0_count: usize,
    /// 已投放 BSP 身份键 (kind_disc, merged_bar, level)（append-only diff，只增不改）。
    seen_bsps: HashSet<(u8, i64, usize)>,
    /// 自下而上涌现升级开关（消融门）：`T_NO_EMERGENCE` 环境变量置位时关闭——用于 L3 A/B
    /// 隔离 emergence-upgrade 的净效应（与 `BT_DUMP_TRADES`/`BT_SYMBOLS` 同类 eval 工具）。
    emergence_enabled: bool,

    /// 流式过程中 `iterate` 涌现过的**最高级别数**（峰值 r*）。递归结构随流非单调（后续 bar
    /// 修订早期结构 ⟹ 末态塔可能浅于峰值），操作在峰值深度的级别上发生过。L2 度量 P1 用峰值
    /// r*（「带通展开到几带」），`tree_census()` 用末态塔（稳定 TV 谱）——两者并报，纯观测。
    max_levels_seen: usize,
}

impl TFugueStreamCore {
    /// `mode`：步骤c 走势完美判定（Structural/And/Or）。a₀ 来源默认 `Segment`（保 bit-exact）。
    /// orchestrator 配置与 batch `backtest_run.rs` 逐字一致（wide / min_strict_sep=5 /
    /// new_raw_gap_min=3，关 bsp/macd 下游层——segments bit-exact 且省 O(n²)）。
    pub fn new(mode: PerfectionMode) -> Self {
        Self::new_with_a0(mode, A0Source::Segment)
    }

    /// 显式指定 a₀ 来源（线段=基线 / 笔=递归底座下移，526号）。`new(mode)` 委托此构造默认 Segment。
    /// 研发-生产同源：a₀ 来源参数落在 backtest/live 共享的信号层（filter-spec 下游推论1：
    /// 两层级别增量必须分别测量，故 a₀ 来源可切换，非兼容垫片）。
    pub fn new_with_a0(mode: PerfectionMode, a0_source: A0Source) -> Self {
        TFugueStreamCore {
            orch: RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false),
            mode,
            engine: TPositionEngine::new(),
            macd: OnlineMacdState::new(12, 26, 9),
            prefix_pos: vec![0.0], // prefix[0] = 0（区间面积 = prefix[hi+1]−prefix[lo]）
            prefix_neg: vec![0.0],
            cur_bar: 0,
            finished: false,
            a0_source,
            last_stroke_n: 0,
            last_a0_count: 0,
            seen_bsps: HashSet::new(),
            emergence_enabled: std::env::var("T_NO_EMERGENCE").is_err(),
            max_levels_seen: 0,
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
        let (bsps, emergent, n_levels) = {
            let segs = self.orch.segments();
            let strokes = self.orch.strokes();
            let m2r = self.orch.merged_to_raw();
            // include_last_candidate=false（保 bit-exact）；a0_source 决定线段/笔 + 过滤口径。
            let a0 = build_a0_fast(
                segs,
                strokes,
                m2r,
                &self.prefix_pos,
                &self.prefix_neg,
                self.a0_source,
                false,
            );
            let tree = iterate(a0, self.mode);
            (tree.all_bsps(), tree.emergent_top(), tree.levels.len())
        }; // orch 借用在此释放
           // 峰值 r* 观测（非单调流式塔深，L2 度量 P1 用）。
        self.max_levels_seen = self.max_levels_seen.max(n_levels);

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
            // 额外：type1（顶/底背驰=走势终完美）单独标记，供核心走势完成清仓（546号死锁解锁，
            // 与 rec_stream 对称：同源 fresh 去重子集 ⇒ bit-exact）。
            if bsp.kind.is_buy() {
                view.buy[ladder] = true;
                if matches!(bsp.kind, BSPKind::Type1Buy) {
                    view.t1buy[ladder] = true;
                }
            } else {
                view.sell[ladder] = true;
                if matches!(bsp.kind, BSPKind::Type1Sell) {
                    view.t1sell[ladder] = true;
                }
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
        self.prefix_pos
            .push(last_pos + if hist > 0.0 { hist } else { 0.0 });
        self.prefix_neg
            .push(last_neg + if hist < 0.0 { -hist } else { 0.0 });
        let bar = self.cur_bar;

        let mut view = TSignalView::empty();

        // ── a₀ 门控：笔增长 → 数 a₀ 单元 → count 变则重跑 ──
        // 笔增长是两种来源的必要前置（线段仅在笔变时变；confirmed 笔数仅在笔变时变），
        // 故 `sc > last_stroke_n` 作廉价预门控；a0_source 决定实际触发计数口径。
        let sc = self.orch.strokes().len();
        if sc > self.last_stroke_n {
            self.last_stroke_n = sc;
            let n_a0 = match self.a0_source {
                A0Source::Segment => self
                    .orch
                    .segments()
                    .iter()
                    .filter(|s| s.confirmed && s.kind == SegKind::Settled)
                    .count(),
                A0Source::Stroke => self.orch.strokes().iter().filter(|s| s.confirmed).count(),
            };
            if n_a0 != self.last_a0_count {
                self.last_a0_count = n_a0;
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
        let last_bar = if self.cur_bar > 0 {
            Some(self.cur_bar - 1)
        } else {
            None
        };
        self.engine.finish(last_bar);
    }

    pub fn result(&self) -> &FugueResult {
        self.engine.result()
    }

    /// BSP 操作诊断快照（纯观测：逐 BSP 操作类型计数 + 核心 units 变化）。
    pub fn op_diag(&self) -> super::t_engine::BspOpDiag {
        self.engine.op_diag()
    }

    /// engine prove 守卫只读访问（验收报告/测试，与 rec_engine 对称）。
    pub fn guards(&self) -> &super::prove_guards::ProveGuards {
        self.engine.guards()
    }

    pub fn n_trades(&self) -> usize {
        self.engine.n_trades()
    }

    /// 核心走势完成清仓次数（546号死锁解锁路径触发计数，与 rec 对称）。
    pub fn n_trend_done_clears(&self) -> u64 {
        self.engine.n_trend_done_clears
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        let (navv, lu, su, voices) = self.engine.snapshot();
        (self.cur_bar, navv, lu, su, voices)
    }

    /// 流式过程中涌现过的峰值 r*（最高级别数）。见 `max_levels_seen` 字段注释。
    pub fn max_levels_seen(&self) -> usize {
        self.max_levels_seen
    }

    /// 递归塔结构普查（尺度-k 总变差 TV 谱 + r*）——L2「级别=带通滤波器」度量 P1/P2 的结构面。
    ///
    /// 在当前 orch 全历史状态上重建 a₀（与操作引擎 `rerun_and_diff` 逐字同源 `build_a0_fast`，
    /// 同一 `a0_source` 过滤口径）+ `iterate` 一次，**纯只读**汇总每级别结构（不改引擎状态）。
    /// 返回 `Vec<(level, n_input_units, tv_scale, n_completed_moves, tv_moves)>`，`r*` = 返回长度。
    /// - `n_input_units` = 级别-k 输入单元数（k=0 即 a₀ 单元数）；
    /// - `tv_scale` = Σ over 级别-k **输入单元** `(high − low)`：该尺度路径总变差（Σ|ΔP| 读法）；
    /// - `n_completed_moves` = 级别-k 终完美走势数（封装为级别-(k+1) 笔的 Move 数）；
    /// - `tv_moves` = Σ over 级别-k **封装单元** `(high − low)`：Move 端点价差读法
    ///   （结构推论 `tv_moves[k] == tv_scale[k+1]`，封装恒等式 Move(k)≡Level-(k+1) 笔，谱系 540）。
    ///
    /// 命题（233号「整流带通」）：级别对方向**带通**、对幅度**整流**——TV 是被整流的幅度谱，
    /// 不是「级别携带幅度」。该普查只读结构，不声明捕获率（捕获率 = R_k / TV_k 在操作层算）。
    pub fn tree_census(&self) -> Vec<(usize, usize, f64, usize, f64)> {
        let a0 = {
            let segs = self.orch.segments();
            let strokes = self.orch.strokes();
            let m2r = self.orch.merged_to_raw();
            build_a0_fast(
                segs,
                strokes,
                m2r,
                &self.prefix_pos,
                &self.prefix_neg,
                self.a0_source,
                false,
            )
        };
        let tree = iterate(a0.clone(), self.mode);
        let mut out = Vec::with_capacity(tree.levels.len());
        for (k, lvl) in tree.levels.iter().enumerate() {
            let input: &[Unit] = if k == 0 {
                &a0
            } else {
                &tree.levels[k - 1].next_units
            };
            let tv_scale: f64 = input.iter().map(|u| u.high - u.low).sum();
            let n_completed = lvl.trends.iter().filter(|t| t.completed).count();
            let tv_moves: f64 = lvl.next_units.iter().map(|u| u.high - u.low).sum();
            out.push((lvl.level, input.len(), tv_scale, n_completed, tv_moves));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 驱动 orchestrator + 同步累积 online MACD 前缀和（复刻 `push_bar` 的 prefix 逻辑），
    /// 返回 `(orch, prefix_pos, prefix_neg)`——供 `build_a0_fast` 测试两种 a₀ 来源。
    ///
    /// 三尺度正弦叠加（慢趋势 + 中摆动 + 快锯齿）→ 产出多级别嵌套结构（含 settled 段 + 笔），
    /// 使 a0=线段 / a0=笔 两路都非空，对比有意义。
    fn drive_orch(n: usize) -> (RecursiveOrchestrator, Vec<f64>, Vec<f64>) {
        let mut orch = RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false);
        let mut macd = OnlineMacdState::new(12, 26, 9);
        let mut prefix_pos = vec![0.0];
        let mut prefix_neg = vec![0.0];
        for i in 0..n {
            let t = i as f64;
            let slow = 0.04 * t;
            let mid = 14.0 * (t / 130.0).sin();
            let fast = 4.0 * (t / 17.0).sin();
            let c = 100.0 + slow + mid + fast;
            orch.process_bar(c - 0.1, c + 0.6, c - 0.6, c);
            let (_, _, hist) = macd.update(c);
            let lp = *prefix_pos.last().unwrap();
            let ln = *prefix_neg.last().unwrap();
            prefix_pos.push(lp + if hist > 0.0 { hist } else { 0.0 });
            prefix_neg.push(ln + if hist < 0.0 { -hist } else { 0.0 });
        }
        (orch, prefix_pos, prefix_neg)
    }

    /// **Segment a₀ 来源 bit-exact 回归**：参数化后 `A0Source::Segment` 路径逐字复刻
    /// 旧 `seg_to_unit` 闭包（confirmed && Settled 过滤 + 同面积/坐标算术）。
    #[test]
    fn a0_segment来源_bit_exact参照() {
        let (orch, pp, pn) = drive_orch(600);
        let segs = orch.segments();
        let strokes = orch.strokes();
        let m2r = orch.merged_to_raw();
        let got = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Segment, false);
        // 参照：逐字复刻旧 seg_to_unit（confirmed && Settled），独立重算面积/坐标。
        let n = pp.len() - 1;
        let reference: Vec<Unit> = segs
            .iter()
            .filter(|s| s.confirmed && s.kind == SegKind::Settled)
            .map(|s| {
                let raw_i0 = m2r
                    .get(s.i0)
                    .map(|&(lo, _)| lo)
                    .unwrap_or(0)
                    .min(n.saturating_sub(1));
                let raw_i1 = m2r
                    .get(s.i1)
                    .map(|&(_, hi)| hi)
                    .unwrap_or(0)
                    .min(n.saturating_sub(1));
                let (lo, hi) = if raw_i0 <= raw_i1 {
                    (raw_i0, raw_i1)
                } else {
                    (raw_i1, raw_i0)
                };
                Unit {
                    high: s.high,
                    low: s.low,
                    start_bar: s.i0 as i64,
                    end_bar: s.i1 as i64,
                    direction: stroke_to_t_dir(s.direction),
                    level: 0,
                    inner_zhongshu_count: 0,
                    area_pos: pp[hi + 1] - pp[lo],
                    area_neg: pn[hi + 1] - pn[lo],
                }
            })
            .collect();
        assert_eq!(
            got, reference,
            "Segment a₀ 来源逐字 bit-exact 旧 seg_to_unit"
        );
    }

    /// **Stroke a₀ 来源字段 + confirmed 过滤**：每单元 level=0 / 无内部中枢 / 字段来自对应
    /// confirmed 笔；最后未确认笔被排除（bi.md:151）。
    #[test]
    fn a0_stroke来源_字段与confirmed过滤() {
        let (orch, pp, pn) = drive_orch(600);
        let segs = orch.segments();
        let strokes = orch.strokes();
        let m2r = orch.merged_to_raw();
        let a0 = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Stroke, false);

        let n_confirmed = strokes.iter().filter(|s| s.confirmed).count();
        assert_eq!(a0.len(), n_confirmed, "Stroke a₀ 数 = confirmed 笔数");
        assert!(strokes.iter().any(|s| !s.confirmed), "应存在最后未确认笔");
        assert!(a0.len() < strokes.len(), "最后未确认笔被排除 ⟹ a₀ < 总笔数");

        let conf: Vec<&Stroke> = strokes.iter().filter(|s| s.confirmed).collect();
        for (u, s) in a0.iter().zip(conf.iter()) {
            assert_eq!(u.level, 0, "a₀ 笔 level=0");
            assert_eq!(u.inner_zhongshu_count, 0, "笔无内部中枢");
            assert_eq!(u.high, s.high);
            assert_eq!(u.low, s.low);
            assert_eq!(u.start_bar, s.i0 as i64);
            assert_eq!(u.end_bar, s.i1 as i64);
            assert_eq!(u.direction, stroke_to_t_dir(s.direction));
            assert!(u.area_pos >= 0.0 && u.area_neg >= 0.0, "面积非负");
        }
    }

    /// **底座下移**：笔比 settled 段细 ⟹ a0=笔单元数 > a0=线段（526号级别数 5-6→8-10 前提）。
    #[test]
    fn a0_stroke多于segment_底座下移() {
        let (orch, pp, pn) = drive_orch(1200);
        let segs = orch.segments();
        let strokes = orch.strokes();
        let m2r = orch.merged_to_raw();
        let a0_seg = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Segment, false);
        let a0_bi = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Stroke, false);
        assert!(
            !a0_seg.is_empty(),
            "合成序列应产出 ≥1 settled 段（否则对比无意义）"
        );
        assert!(
            a0_bi.len() > a0_seg.len(),
            "a0=笔({}) 应多于 a0=线段({})（底座下移 ⟹ 级别数增量）",
            a0_bi.len(),
            a0_seg.len()
        );
    }

    /// **递归塔深度 L1 检查**（底座下移下游推论）：a0=笔 比 a0=线段 迭代出**不浅于**的递归塔
    /// （级别数 ≥）——级别数 5-6→8-10 的合成数据侧管线验证（L1，真实增量待 L2/L3 数据）。
    #[test]
    fn a0_stroke递归塔不浅于segment() {
        let (orch, pp, pn) = drive_orch(1200);
        let segs = orch.segments();
        let strokes = orch.strokes();
        let m2r = orch.merged_to_raw();
        let a0_seg = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Segment, false);
        let a0_bi = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Stroke, false);
        let tree_seg = iterate(a0_seg, PerfectionMode::Structural);
        let tree_bi = iterate(a0_bi, PerfectionMode::Structural);
        assert!(
            tree_bi.levels.len() >= tree_seg.levels.len(),
            "a0=笔递归塔级别数({}) 应 ≥ a0=线段({})（底座下移）",
            tree_bi.levels.len(),
            tree_seg.levels.len()
        );
    }

    /// **include_last_candidate 笔/段对称**：true 时各自额外含最后未确认单元。
    #[test]
    fn a0_include_last_candidate笔段对称() {
        let (orch, pp, pn) = drive_orch(600);
        let segs = orch.segments();
        let strokes = orch.strokes();
        let m2r = orch.merged_to_raw();
        // Stroke：last candidate = 最后未确认笔 ⟹ 多 1。
        let bi_off = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Stroke, false);
        let bi_on = build_a0_fast(segs, strokes, m2r, &pp, &pn, A0Source::Stroke, true);
        assert_eq!(
            bi_on.len(),
            bi_off.len() + 1,
            "Stroke include_last_candidate 多 1 未确认笔"
        );
        assert_eq!(
            &bi_on[..bi_off.len()],
            &bi_off[..],
            "前缀逐字一致（仅尾部追加）"
        );
    }

    /// **流式 new 默认 = Segment 来源**：`new(mode)` ⟺ `new_with_a0(mode, Segment)`（bit-exact）。
    #[test]
    fn 流式new默认等价segment来源() {
        fn run(core: &mut TFugueStreamCore) {
            let mut price = 100.0;
            let mut up = true;
            for i in 0..600 {
                if i % 20 == 0 {
                    up = !up;
                }
                price += if up { 1.0 } else { -0.8 };
                let c = price;
                core.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
            }
            core.finish();
        }
        let mut a = TFugueStreamCore::new(PerfectionMode::And);
        let mut b = TFugueStreamCore::new_with_a0(PerfectionMode::And, A0Source::Segment);
        run(&mut a);
        run(&mut b);
        assert_eq!(
            a.result().final_nav,
            b.result().final_nav,
            "new 默认 Segment ⟺ new_with_a0(Segment)"
        );
        assert_eq!(a.n_trades(), b.n_trades());
    }

    /// **流式笔底座零 panic + 有限 final_nav**（守恒守卫每 bar 成立）。
    #[test]
    fn 流式笔底座零panic() {
        let mut core = TFugueStreamCore::new_with_a0(PerfectionMode::Structural, A0Source::Stroke);
        let mut price = 100.0;
        let mut up = true;
        for i in 0..600 {
            if i % 20 == 0 {
                up = !up;
            }
            price += if up { 1.0 } else { -0.8 };
            let c = price;
            core.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
        }
        core.finish();
        assert!(core.result().final_nav.is_finite(), "笔底座 final_nav 有限");
    }

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
