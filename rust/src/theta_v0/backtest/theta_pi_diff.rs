//! #1306 对拍 harness v1：回放（stream）vs 批量（fill loop）三面 diff + 预期差机械清单。
//!
//! ## 背景（#1304 第一批② 最大风险对策）
//!
//! stream 与批量 fill loop「共核不共外围」：决策核 [`pi_theta_step_traced_with_risk_seeds`] 两侧
//! 同一份，但外围（风控门/risk seeds/TW 谓词/候选过滤/账本机械）只在批量侧存在。逐位对拍若不
//! 先钉死预期差清单，会把预期内差异误报成回归、或把真回归漏进预期差里（scan §4）。
//!
//! ## 三面
//!
//! 1. **信号面**（runner.rs:351 闭包同源）：`(classification, tower, tower_gen, forest_epoch)`。
//!    两侧分类器同走 `append_incr_layer` + `classify_incremental` ⟹ **bit-exact 铁锁，零预期差**。
//! 2. **状态面**（sep_legs + PersistentRegistry）：`pi_theta_step_traced_with_risk_seeds` 产出。
//!    预期差 = 批量侧外围触发（风控门偏离 open / risk seeds / TW 谓词 / 候选过滤）+ 级联。
//! 3. **账本面**（TwLedger + TypedTrade + LevelLedgerMirror）：TwLedger/TypedTrade 批量侧独有
//!    （stream 侧恒缺 ⟹ 预期差）；LevelLedgerMirror 两侧同构，逐 bar 步进读数 + 终态对拍。
//!
//! ## 预期差机械清单（机器可判）
//!
//! | 规则 | 判据（本 bar） | 面 |
//! |---|---|---|
//! | TwLedgerBatchOnly | stream 侧无 TW 账本，批量侧 `tw_final.is_some()` | 账本 |
//! | TypedTradeBatchOnly | stream 侧无 typed 簿，批量侧 `typed_ledger` 非空 | 账本 |
//! | RiskGate | `gate != KThetaRiskGate::open()` | 状态 |
//! | RiskSeeds | `n_risk_seeds > 0` | 状态 |
//! | TwPredicate | `tw_event || overlay_closes > 0` | 状态 |
//! | CandidateFilter | `n_gamma_raw != n_gamma_trade`（χ/nest/entry_stop_recheck） | 状态 |
//! | Cascade | 状态面已在更早 bar 分歧 | 状态 |
//!
//! 不落在上表的差异 = **预期外 = 回归告警**（含 stream 侧 PersistentRegistry 不合并这类 seam
//! 缺口——它们会以「预期外」显性浮出，正是本 harness 要交的物证）。
//!
//! ## v1 范围
//!
//! CI 用全可交易合成锯齿 bar（无 untradable bar）。untradable bar 的分类口径（stream 逐 bar
//! append vs 批量只对可交易 bar `classify_at`）是已知 seam 差异，v1 不展开——喂全可交易数据
//! 即不触达。

use std::fmt;
use std::rc::Rc;

use super::super::config::ThetaConfig;
use super::super::strategy::coverage::{KThetaRiskGate, SepLeg};
use super::super::strategy::ledger::TwState;
use super::super::strategy::level_ledger::{LevelLedgerMirror, LevelLedgerStep};
use super::super::strategy::overlay_state::OverlayState;
use super::super::strategy::persistent::PersistentRegistry;
// 以下四类仅 `#[cfg(test)]` 测试构造用（backtest_bin 非测试构建下保持零新增 warning）。
#[cfg(test)]
use super::super::classifier::recursive_tower::ElementId;
#[cfg(test)]
use super::super::classifier::Classification;
#[cfg(test)]
use super::super::strategy::coverage::Vertical;
#[cfg(test)]
use super::super::strategy::voice::VoiceSide;
use super::super::stream::signal_capture::{self, SignalObs};
use super::super::stream::ThetaPiStream;
use super::super::types::Bar;
use super::diff_capture::{self, BatchBarObs, BatchCapture};
use super::incremental::IncrementalClassifier;
use super::ledger::TypedTrade;
use super::runner::pi_theta_fill_loop_overlay;

// ──────────────────────────────────────────────────────────────────────────────
//  观测载体
// ──────────────────────────────────────────────────────────────────────────────

/// 批量侧整窗捕获 + 终态账本。
///
/// #1308 公开：`theta_accept` bin 把它当不透明句柄传给 [`run_stream`]/[`diff`]，字段不暴露。
pub struct BatchSide {
    cap: BatchCapture,
    signals: Vec<SignalObs>,
    level_ledger: LevelLedgerMirror,
    tw_final: Option<TwState>,
    typed_ledger: Vec<TypedTrade>,
}

/// 流式侧单 bar 观测。
#[derive(Debug, Clone)]
struct StreamBarObs {
    sep_legs: Vec<SepLeg>,
    registry_len: usize,
    level_step: LevelLedgerStep,
}

/// 流式侧整窗捕获 + 终态。
///
/// #1308 公开：`theta_accept` bin 把它当不透明句柄传给 [`diff`]，字段不暴露。
pub struct StreamSide {
    bars: Vec<StreamBarObs>,
    signals: Vec<SignalObs>,
    registry: PersistentRegistry,
    level_ledger: LevelLedgerMirror,
}

// ──────────────────────────────────────────────────────────────────────────────
//  预期差规则与报告
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    Signal,
    State,
    Ledger,
}

impl fmt::Display for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Surface::Signal => write!(f, "信号"),
            Surface::State => write!(f, "状态"),
            Surface::Ledger => write!(f, "账本"),
        }
    }
}

/// 预期差机械清单（机器可判；判据见模块头表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedRule {
    TwLedgerBatchOnly,
    TypedTradeBatchOnly,
    RiskGate,
    RiskSeeds,
    TwPredicate,
    CandidateFilter,
    Cascade,
}

impl ExpectedRule {
    fn name(self) -> &'static str {
        match self {
            ExpectedRule::TwLedgerBatchOnly => "TwLedgerBatchOnly",
            ExpectedRule::TypedTradeBatchOnly => "TypedTradeBatchOnly",
            ExpectedRule::RiskGate => "RiskGate",
            ExpectedRule::RiskSeeds => "RiskSeeds",
            ExpectedRule::TwPredicate => "TwPredicate",
            ExpectedRule::CandidateFilter => "CandidateFilter",
            ExpectedRule::Cascade => "Cascade",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Cause {
    Expected(ExpectedRule),
    Unexpected(String),
}

#[derive(Debug, Clone)]
struct Finding {
    bar: usize,
    surface: Surface,
    cause: Cause,
    detail: String,
}

/// 对拍报告：预期内差异计数 + 预期外差异（回归告警）。
///
/// #1308 公开：`theta_accept` bin 用 [`Display`](fmt::Display) 打报告正文，用访问器取
/// PASS/FAIL 判据（`n_unexpected()==0 ⟺ 无回归`）。
#[derive(Debug, Default)]
pub struct DiffReport {
    n_bars: usize,
    findings: Vec<Finding>,
}

impl DiffReport {
    fn push(&mut self, bar: usize, surface: Surface, cause: Cause, detail: impl Into<String>) {
        self.findings.push(Finding {
            bar,
            surface,
            cause,
            detail: detail.into(),
        });
    }

    /// 回放过的可交易决策 bar 数（对拍窗口）。
    pub fn n_bars(&self) -> usize {
        self.n_bars
    }

    /// 预期内差异计数（预期差机械清单命中的差异）。
    pub fn n_expected(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| matches!(f.cause, Cause::Expected(_)))
            .count()
    }

    /// 预期外差异计数（预期差清单未覆盖的差异 = 回归告警）。
    pub fn n_unexpected(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| matches!(f.cause, Cause::Unexpected(_)))
            .count()
    }

    /// 信号面差异数（信号面是共核铁锁：应恒 0）。
    pub fn n_signal_diffs(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.surface == Surface::Signal)
            .count()
    }
}

impl DiffReport {
    /// 预期内差异按规则计数（机器可判清单的产出侧读数）。
    fn expected_by_rule(&self) -> Vec<(ExpectedRule, usize)> {
        let mut counts: Vec<(ExpectedRule, usize)> = Vec::new();
        for f in &self.findings {
            if let Cause::Expected(r) = f.cause {
                match counts.iter_mut().find(|(rule, _)| *rule == r) {
                    Some((_, c)) => *c += 1,
                    None => counts.push((r, 1)),
                }
            }
        }
        counts
    }
}

impl fmt::Display for DiffReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== #1306 对拍报告（回放 vs 批量三面 diff）===")?;
        writeln!(f, "bar 数      : {}", self.n_bars)?;
        writeln!(f, "信号面分歧  : {}", self.n_signal_diffs())?;
        writeln!(f, "预期内差异  : {}", self.n_expected())?;
        writeln!(
            f,
            "预期外差异  : {}  {}",
            self.n_unexpected(),
            if self.n_unexpected() == 0 {
                "(无回归)"
            } else {
                "(回归告警)"
            }
        )?;
        let by_rule = self.expected_by_rule();
        if !by_rule.is_empty() {
            writeln!(f, "预期内差异按规则:")?;
            for (rule, c) in &by_rule {
                writeln!(f, "  {:<24} {}", rule.name(), c)?;
            }
        }
        let unexpected: Vec<&Finding> = self
            .findings
            .iter()
            .filter(|f| matches!(f.cause, Cause::Unexpected(_)))
            .collect();
        if !unexpected.is_empty() {
            writeln!(f, "预期外差异明细（回归告警）:")?;
            for finding in &unexpected {
                if let Cause::Unexpected(d) = &finding.cause {
                    writeln!(
                        f,
                        "  bar {:>5} {:<4} {}  {}",
                        finding.bar, finding.surface, d, finding.detail
                    )?;
                }
            }
        }
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────────────────────
//  跑双路径
// ──────────────────────────────────────────────────────────────────────────────

/// 跑批量侧（overlay 臂，χ≡1，voice_exec=None）：`classify_at` 闭包逐 bar 外化信号面，
/// [`diff_capture`] sink 逐 bar 外化状态/账本面。
///
/// `initial_nav` 由调用方显式传入（#1308：验收报告器用与回放 driver 同口径的
/// `max(first_px×10_000, 1e6)`，保证对拍与回放在同一账户尺度上）。
pub fn run_batch(bars: &[Bar], config: &ThetaConfig, initial_nav: f64) -> BatchSide {
    let mut classifier_incr = IncrementalClassifier::new(bars, config);
    let mut signals: Vec<SignalObs> = Vec::new();
    let mut overlay = OverlayState::new();
    let mut level_ledger = LevelLedgerMirror::new();

    diff_capture::start();
    let fill = pi_theta_fill_loop_overlay(
        // ★runner.rs:351 闭包同源：classify_at + last_l0（strokes 供 nest 门 ForceL）+
        // tower_confirmed_lens / tower_generation / forest_epoch。
        |i| {
            let out = classifier_incr.classify_at(i);
            let l0 = classifier_incr
                .last_l0()
                .expect("classify_at 已推进 ParseLayer");
            let cls = out.classification;
            let tower = out.tower;
            let strokes = Rc::clone(&l0.strokes);
            let confirmed_lens = classifier_incr.tower_confirmed_lens(tower.len());
            let gen = classifier_incr.tower_generation();
            let fe = classifier_incr.forest_epoch();
            signals.push(SignalObs {
                classification: cls.clone(),
                tower: tower.clone(),
                tower_gen: gen,
                forest_epoch: fe,
            });
            (cls, tower, confirmed_lens, gen, fe, strokes)
        },
        bars,
        initial_nav,
        config,
        None, // χ≡1（overlay 臂同款配置）
        Some(&mut overlay),
        Some(&mut level_ledger),
        None, // voice_exec=None ⟹ 净额臂 + overlay 只读旁路
    );
    let cap = diff_capture::take().expect("start() 后必有捕获");
    BatchSide {
        cap,
        signals,
        level_ledger,
        tw_final: fill.tw_final,
        typed_ledger: fill.typed_ledger,
    }
}

/// 跑流式侧：逐 bar 喂批量侧同源的 `p_t`/`equity_nav`，采集三面。
pub fn run_stream(bars: &[Bar], config: &ThetaConfig, batch: &BatchSide) -> StreamSide {
    let mut stream = ThetaPiStream::new(config.clone());
    signal_capture::start();
    let mut out = Vec::with_capacity(batch.cap.bars.len());
    for obs in &batch.cap.bars {
        let bar = bars[obs.i];
        let _p_star = stream.push_bar(bar, obs.p_t, obs.equity_nav);
        out.push(StreamBarObs {
            sep_legs: stream.sep_legs().to_vec(),
            registry_len: stream.registry().len(),
            level_step: stream.last_level_step(),
        });
    }
    stream.finish();
    let signals = signal_capture::take().expect("start() 后必有信号面捕获");
    StreamSide {
        bars: out,
        signals,
        registry: stream.registry().clone(),
        level_ledger: stream.level_ledger().clone(),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
//  diff 引擎
// ──────────────────────────────────────────────────────────────────────────────

/// 批量侧本 bar 的外围触发器（机械判据，见模块头表）。
fn batch_triggers(obs: &BatchBarObs) -> Vec<ExpectedRule> {
    let mut t = Vec::new();
    if obs.gate != KThetaRiskGate::open() {
        t.push(ExpectedRule::RiskGate);
    }
    if obs.n_risk_seeds > 0 {
        t.push(ExpectedRule::RiskSeeds);
    }
    if obs.tw_event || obs.overlay_closes > 0 {
        t.push(ExpectedRule::TwPredicate);
    }
    if obs.n_gamma_raw != obs.n_gamma_trade {
        t.push(ExpectedRule::CandidateFilter);
    }
    t
}

/// 三面对拍。
pub fn diff(batch: &BatchSide, stream: &StreamSide) -> DiffReport {
    let n = batch.cap.bars.len().min(stream.bars.len());
    let mut report = DiffReport {
        n_bars: n,
        findings: Vec::new(),
    };

    // 信号面：共核铁锁（零预期差）。
    for k in 0..n {
        let b = &batch.signals[k];
        let s = &stream.signals[k];
        if b != s {
            report.push(
                batch.cap.bars[k].i,
                Surface::Signal,
                Cause::Unexpected("信号面分歧：两侧分类器输出不一致".into()),
                "classification/tower/gen/epoch 任一不等",
            );
        }
    }

    // 状态面（sep_legs + PersistentRegistry）：统一逐 bar，共享分歧历史——把「外围触发器 →
    // 分歧 → 级联」的因果归因单遍完成，避免把 registry 驱动的 sep_legs 分歧误报成「无触发
    // 预期外」。分歧历史一旦置位，后续 bar 的状态差即级联（因果上由首个分歧 bar 驱动）。
    let mut state_diverged = false;
    let mut registry_reported = false;
    for k in 0..n {
        let obs = &batch.cap.bars[k];
        let s = &stream.bars[k];
        let triggers = batch_triggers(obs);

        if obs.sep_legs != s.sep_legs {
            let cause = if let Some(rule) = triggers.first().copied() {
                Cause::Expected(rule)
            } else if state_diverged {
                Cause::Expected(ExpectedRule::Cascade)
            } else {
                Cause::Unexpected(format!(
                    "状态面分歧且无外围触发：batch {} 腿 vs stream {} 腿",
                    obs.sep_legs.len(),
                    s.sep_legs.len()
                ))
            };
            report.push(
                obs.i,
                Surface::State,
                cause,
                format!(
                    "sep_legs 分歧（batch {} 腿 vs stream {} 腿；gate={:?} seeds={} tw={}/{} γ={}→{}）",
                    obs.sep_legs.len(),
                    s.sep_legs.len(),
                    obs.gate,
                    obs.n_risk_seeds,
                    obs.tw_event,
                    obs.overlay_closes,
                    obs.n_gamma_raw,
                    obs.n_gamma_trade
                ),
            );
            state_diverged = true;
        }

        if obs.registry_len != s.registry_len {
            let cause = if !triggers.is_empty() {
                Cause::Expected(triggers[0])
            } else if state_diverged {
                Cause::Expected(ExpectedRule::Cascade)
            } else {
                Cause::Unexpected(
                    "registry 分歧：stream 侧持久注册表不合并（restore 祖先腿无源）".into(),
                )
            };
            report.push(
                obs.i,
                Surface::State,
                cause,
                format!(
                    "registry_len batch {} vs stream {}",
                    obs.registry_len, s.registry_len
                ),
            );
            registry_reported = true;
            state_diverged = true;
        }
    }

    // 状态面（PersistentRegistry 终态全量）：仅当逐 bar 长度一路相等却仍内容不等时兜底
    // （stream 侧恒空 ⟹ 长度已充分；本兜底为未来 stream 合并 registry 后的内容级对拍保留）。
    if !registry_reported {
        if let Some(breg) = &batch.cap.final_registry {
            if breg != &stream.registry {
                report.push(
                    n.saturating_sub(1),
                    Surface::State,
                    Cause::Unexpected("registry 终态分歧（逐 bar 长度相等但内容不等）".into()),
                    format!(
                        "registry 终态 batch {} 元素 vs stream {} 元素",
                        breg.len(),
                        stream.registry.len()
                    ),
                );
            }
        }
    }

    // 账本面（LevelLedgerMirror 逐 bar 步进读数）：step 是 sep_legs 的纯函数 ⟹ sep_legs
    // 一致而 step 分歧 = 账本漂移（预期外）；sep_legs 分歧 ⟹ 级联（预期内）。
    for k in 0..n {
        let obs = &batch.cap.bars[k];
        let s = &stream.bars[k];
        let (Some(bl), sl) = (obs.level_step, s.level_step) else {
            continue;
        };
        if bl != sl {
            let cause = if obs.sep_legs != s.sep_legs {
                Cause::Expected(ExpectedRule::Cascade)
            } else {
                Cause::Unexpected("level_ledger 步进分歧且 sep_legs 未分歧".into())
            };
            report.push(
                obs.i,
                Surface::Ledger,
                cause,
                format!("level_step batch {:?} vs stream {:?}", bl, sl),
            );
        }
    }

    // 账本面（LevelLedgerMirror 终态）。
    if batch.level_ledger != stream.level_ledger {
        let cause = if state_diverged {
            Cause::Expected(ExpectedRule::Cascade)
        } else {
            Cause::Unexpected("level_ledger 终态分歧且状态面未分歧".into())
        };
        report.push(
            n.saturating_sub(1),
            Surface::Ledger,
            cause,
            "LevelLedgerMirror 终态不等",
        );
    }

    // 账本面（TwLedger / TypedTrade：批量侧独有，stream 侧恒缺 ⟹ 预期差）。
    if batch.tw_final.is_some() {
        report.push(
            n.saturating_sub(1),
            Surface::Ledger,
            Cause::Expected(ExpectedRule::TwLedgerBatchOnly),
            "TW 账本仅批量侧存在（stream 无 TW 源）",
        );
    }
    if !batch.typed_ledger.is_empty() {
        report.push(
            n.saturating_sub(1),
            Surface::Ledger,
            Cause::Expected(ExpectedRule::TypedTradeBatchOnly),
            format!("typed 簿仅批量侧存在，{} 笔", batch.typed_ledger.len()),
        );
    }

    report
}

// ──────────────────────────────────────────────────────────────────────────────
//  合成数据 + 测试
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
/// 确定性合成锯齿行情（全可交易，无 untradable bar）。
///
/// 沿用 `runner_tests.rs` 的 4 升/4 降步进形态（open=high=low=close，step 递变）——该形态
/// 已被 `run_theta_v0_pi_zigzag_prefix_path_runs_clean` 坐实「锯齿产笔/段、前缀塔逐 bar
/// 演化」⟹ **信号面**（classification/tower）跑在真结构上（塔非平凡演化）。实测此形态的
/// 决策层恒空（不产买卖点信号 ⟹ sep_legs/registry/level_ledger 全程空）——状态/账本面的
/// 真结构对拍由 `replay_vs_batch_structured_report`（`#[ignore]`）承载，CI 只锁信号面铁锁。
fn synthetic_bars() -> Vec<Bar> {
    (0..240)
        .map(|i| {
            let up = ((i / 4) % 2) == 0;
            let base = 10_000_000_000i64;
            let step = 250_000_000i64 * ((i % 4) as i64);
            let close_tick = if up {
                base + step
            } else {
                base + 1_000_000_000 - step
            };
            Bar {
                source_index: i,
                timestamp: i as i64,
                open: close_tick,
                high: close_tick,
                low: close_tick,
                close: close_tick,
                volume: 1.0,
                untradable: false,
            }
        })
        .collect()
}

#[cfg(test)]
fn run_pair(bars: &[Bar], config: &ThetaConfig) -> (BatchSide, StreamSide) {
    let batch = run_batch(bars, config, 1.0e6);
    let stream = run_stream(bars, config, &batch);
    (batch, stream)
}

/// 信号面 bit-exact 铁锁 + 报告产出（预期内/预期外计数）。
#[test]
fn replay_vs_batch_signal_surface_bit_exact_and_report() {
    let bars = synthetic_bars();
    let config = ThetaConfig::default();
    let (batch, stream) = run_pair(&bars, &config);
    let report = diff(&batch, &stream);

    // 信号面是共核铁锁：逐 bar bit-exact（分类器同源，零预期差规则）。
    assert_eq!(
        report.n_signal_diffs(),
        0,
        "信号面必须 bit-exact（共核铁锁）\n{report}"
    );
    // 报告打印（预期内差异计数 + 预期外差异=回归告警）。
    println!("{report}");
}

/// 预期差机械清单逐项机器可判：`batch_triggers` 对外围触发器的判据逐项验证。
#[test]
fn expected_diff_rules_are_machine_encoded() {
    let base = BatchBarObs {
        i: 0,
        p_t: 0.0,
        equity_nav: 1.0e6,
        sep_legs: Vec::new(),
        gate: KThetaRiskGate::open(),
        n_risk_seeds: 0,
        tw_event: false,
        overlay_closes: 0,
        n_gamma_raw: 3,
        n_gamma_trade: 3,
        level_step: None,
        registry_len: 0,
    };

    let mut obs = base.clone();
    obs.gate = KThetaRiskGate {
        stop_long: true,
        ..KThetaRiskGate::open()
    };
    assert_eq!(batch_triggers(&obs), vec![ExpectedRule::RiskGate]);

    let mut obs = base.clone();
    obs.n_risk_seeds = 1;
    assert_eq!(batch_triggers(&obs), vec![ExpectedRule::RiskSeeds]);

    let mut obs = base.clone();
    obs.tw_event = true;
    assert_eq!(batch_triggers(&obs), vec![ExpectedRule::TwPredicate]);

    let mut obs = base.clone();
    obs.overlay_closes = 2;
    assert_eq!(batch_triggers(&obs), vec![ExpectedRule::TwPredicate]);

    let mut obs = base.clone();
    obs.n_gamma_trade = 1;
    assert_eq!(batch_triggers(&obs), vec![ExpectedRule::CandidateFilter]);

    assert!(batch_triggers(&base).is_empty(), "无触发 ⟹ 空清单");

    // TwLedgerBatchOnly / TypedTradeBatchOnly 是账本面规则，判据在 diff() 的终态段（stream 侧
    // 恒缺）；此处只锁规则命名完备（报告分桶键）。
    for r in [
        ExpectedRule::TwLedgerBatchOnly,
        ExpectedRule::TypedTradeBatchOnly,
        ExpectedRule::RiskGate,
        ExpectedRule::RiskSeeds,
        ExpectedRule::TwPredicate,
        ExpectedRule::CandidateFilter,
        ExpectedRule::Cascade,
    ] {
        assert!(!r.name().is_empty());
    }
}

/// 判别力负对照：手工构造单 bar 最小场景（无外围触发、无历史分歧），把 stream 侧 sep_legs
/// 篡改为与 batch 不一致，diff 引擎必须报「状态面分歧且无外围触发」的预期外（非漏报）。
///
/// 不依赖合成数据是否产腿——浅锯齿数据 sep_legs 恒空（不产买卖点信号），`run_pair` 场景下
/// 篡改分支根本不触达；直接锁 diff 引擎判别逻辑才是有判别力的负对照。
#[test]
fn diff_engine_flags_unexpected_state_divergence() {
    let leg = SepLeg {
        id: ElementId {
            level: 0,
            ordinal: 0,
        },
        side: VoiceSide::Long,
        q_units: 2.0,
        role_v: Vertical::Ambient,
        parent_id: None,
    };
    let signal = SignalObs {
        classification: Classification::default(),
        tower: Vec::new(),
        tower_gen: 0,
        forest_epoch: 0,
    };
    let step = LevelLedgerStep {
        total_net: 0,
        n_active_levels: 0,
    };

    let batch = BatchSide {
        cap: BatchCapture {
            bars: vec![BatchBarObs {
                i: 0,
                p_t: 0.0,
                equity_nav: 1.0e6,
                sep_legs: vec![leg],
                gate: KThetaRiskGate::open(),
                n_risk_seeds: 0,
                tw_event: false,
                overlay_closes: 0,
                n_gamma_raw: 1,
                n_gamma_trade: 1,
                level_step: None,
                registry_len: 0,
            }],
            final_registry: None,
        },
        signals: vec![signal.clone()],
        level_ledger: LevelLedgerMirror::default(),
        tw_final: None,
        typed_ledger: Vec::new(),
    };
    // stream 侧同 bar sep_legs 篡改为空（batch 1 腿 vs stream 0 腿），其余三面恒等。
    let stream = StreamSide {
        bars: vec![StreamBarObs {
            sep_legs: Vec::new(),
            registry_len: 0,
            level_step: step,
        }],
        signals: vec![signal],
        registry: PersistentRegistry::default(),
        level_ledger: LevelLedgerMirror::default(),
    };

    let report = diff(&batch, &stream);
    assert_eq!(
        report.n_unexpected(),
        1,
        "篡改 sep_legs 后必须报预期外（回归告警），否则 harness 漏报\n{report}"
    );
    assert!(
        report.findings.iter().any(|f| {
            matches!(&f.cause, Cause::Unexpected(d) if d.contains("状态面分歧且无外围触发"))
        }),
        "预期外必须归因到「状态面分歧且无外围触发」\n{report}"
    );
}

/// 结构化随机游走报告（`#[ignore]`：CI 只跑浅锯齿信号锁；本测试供手工/12 个月回放对拍
/// 报告场景——真结构上三面都有非平凡读数，预期外差异显性浮出）。
#[test]
#[ignore]
fn replay_vs_batch_structured_report() {
    // 确定性 LCG 随机游走：足够多的 bar 形成分型/笔/段/中枢。
    let mut state: u64 = 0x1306_5eed_0000_0001u64;
    let mut close: i64 = 10_000_000_000i64;
    let mut bars = Vec::with_capacity(4000);
    for i in 0..4000usize {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let drift = ((state >> 33) % 4001) as i64 - 2000; // [-2000, +2000] tick
        let open = close;
        close = (close + drift).max(1);
        let high = open.max(close) + 300;
        let low = open.min(close) - 300;
        bars.push(Bar {
            source_index: i,
            timestamp: i as i64,
            open,
            high,
            low,
            close,
            volume: 1000.0,
            untradable: false,
        });
    }
    let config = ThetaConfig::default();
    let (batch, stream) = run_pair(&bars, &config);
    let report = diff(&batch, &stream);
    println!("{report}");
    // 信号面仍是铁锁（真结构上分类器同源 bit-exact）。
    assert_eq!(report.n_signal_diffs(), 0, "信号面在真结构上仍须 bit-exact");
}
