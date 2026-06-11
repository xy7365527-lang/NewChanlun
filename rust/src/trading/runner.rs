//! run_organic — 有机赋格回测主循环。
//!
//! 控制流逐字移植 Python `organic_fugue.run_organic`（V0≡P5 逐位等价的承重面）：
//! 中枢账本 ingest → fatigue observe → FLAT/ARMED/LONG 分派 → (LONG) allocator →
//! master 循环 → voice main 腿 → VoiceUnit.step → eod_close。
//!
//! v2 与 Python v1 的语义差异（仅 REV 路径，O0/V0 不触及）：
//!   - master 出场 = `MasterExitSignal`（C1 类型隔离；Python 的 master_seg_end
//!     轴在 v2 不存在——三触发喂 master 不可表示）；
//!   - voice REV = tranche/G1/区间套（level_operating_unit.rs，C2-C5）；
//!   - 41课门 = 点态（fatigue_gate.rs，C7）。
//!
//! ## capability guards（声明=能力，fail-fast）
//! - 事件磁带非全空；rev_mode ⇒ div 磁带非全空（Python 逐字）；
//! - rev_gate ⇒ run_high 行（C7 清空路径(1) 的数据依赖）；
//! - sub_anchor≠Off ⇒ dir_row 行（G1）；
//! - tranche ⇒ dir_row + run_anchor 行（T4b/T5b）。
//! 当前磁带（organic_signals.py）无 D3 行 ⇒ 可运行配置 = V0/V3′ 与
//! sub_anchor=Off 的 rev 变体；D3 行是 M2 信号层工位（引擎零改动边界）。

use super::allocator::SizeAllocator;
use super::center_book::CenterBook;
use super::config::{OrganicConfig, Sizing, StopMode};
use super::fatigue_gate::FatigueGate;
use super::ledger::{LegTrace, OrganicLedger};
use super::level_operating_unit::{BarRows, VoicePhase, VoiceUnit};
use super::master::{MasterExitSignal, MasterState};
use super::tape::SignalTape;
use super::types::*;

/// 完成交易记录（Python `CompletedTrade` 逐字段）。
#[derive(Debug, Clone)]
pub struct TradeRec {
    pub entry_bar: i64,
    pub entry_price: f64,
    pub exit_bar: i64,
    pub exit_price: f64,
    /// py_round(·, 4)。
    pub pnl_pct: f64,
    pub exit_reason: String,
    pub n_short_diffs: u32,
    /// py_round(·, 6)。
    pub cost_basis_at_exit: f64,
}

/// diag 模式逐 trade 快照（Python diag dict 的核心字段；派生字段
/// entry_ladder_name / cost_basis_entry / total_shares_entry / n_diffs 由
/// 消费方按定义重建）。
#[derive(Debug, Clone)]
pub struct TradeDiag {
    pub entry_bar: i64,
    pub entry_price: f64,
    pub exit_bar: i64,
    pub exit_price: f64,
    pub exit_reason: String,
    pub entry_ladder: usize,
    pub pnl_pct: f64,
    /// 未 round（Python diag["cost_basis_exit"] = pos.cost_basis 原值）。
    pub cost_basis_exit: f64,
    pub total_shares_exit: f64,
    pub reached_earning: bool,
    pub diffs: Vec<LegTrace>,
}

#[derive(Debug, Default)]
pub struct RunResult {
    pub trades: Vec<TradeRec>,
    pub counters: Counters,
    pub rev_attempts_by_ladder: [u64; MAX_LADDER],
    pub rev_opens_by_ladder: [u64; MAX_LADDER],
    pub ladder_attribution: [u64; MAX_LADDER],
    pub ladder_held_bars: [u64; MAX_LADDER],
    /// py_key → (recovered_cash, n_short_diffs)，py_key 升序。
    pub leg_contribution: Vec<(i64, f64, u64)>,
    pub n_addon: u64,
    pub n_core_stops: u64,
    pub diag: Option<Vec<TradeDiag>>,
}

const FLAT: u8 = 0;
const ARMED: u8 = 1;
const LONG: u8 = 2;

struct Run {
    floor_ladder: usize,
    stop_on: bool,
    // ── 仓位状态（Python 平行变量组的逐字镜像）──
    state: u8,
    entry_bar: i64,
    entry_price: f64,
    entry_ladder: usize,
    arm_bar: i64,
    arm_ladder: usize,
    pos: Option<OrganicLedger>,
    active_levels: Vec<usize>,
    voices: Vec<VoiceUnit>,
    master_state: MasterState,
    // ── 市场性质（跨 trade 持续）──
    book: CenterBook,
    gate: FatigueGate,
    alloc: SizeAllocator,
    // ── 计数与归因 ──
    counters: Counters,
    res: RunResult,
    fsm_recovered: std::collections::BTreeMap<i64, (f64, u64)>,
    with_diag: bool,
}

impl Run {
    fn open_position(&mut self, bar: i64, price: f64, el: usize) {
        self.entry_bar = bar;
        self.entry_price = price;
        self.entry_ladder = el;
        self.active_levels = (self.floor_ladder..el).collect();
        let n_sub = self.active_levels.len();
        let level_frac = if n_sub > 0 { 1.0 / n_sub as f64 } else { 0.0 };
        self.pos = Some(OrganicLedger::new(
            price,
            INITIAL_CAPITAL / price,
            level_frac,
            self.with_diag,
        ));
        self.voices = self.active_levels.iter().map(|&k| VoiceUnit::new(k)).collect();
        self.master_state = MasterState::Ride;
        self.state = LONG;
    }

    fn reset_flat(&mut self) {
        self.state = FLAT;
        self.entry_price = 0.0;
        self.pos = None;
        self.active_levels = Vec::new();
        self.voices = Vec::new();
        self.master_state = MasterState::Ride;
    }

    /// Python `_close` 逐字（含 total_shares≤0 早退、插入序清腿、round 语义）。
    fn close_position(&mut self, bar: i64, price: f64, reason: &str) {
        let Some(mut pos) = self.pos.take() else {
            self.reset_flat();
            return;
        };
        if self.entry_price <= 0.0 || pos.total_shares <= 0.0 {
            self.reset_flat();
            return;
        }
        for key in pos.open_keys() {
            pos.close_diff(key, price, bar);
        }
        let total_value = pos.total_shares * price + pos.cumulative_recovered;
        let pnl_pct = (total_value - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100.0;
        let held = bar - self.entry_bar;
        self.res.trades.push(TradeRec {
            entry_bar: self.entry_bar,
            entry_price: self.entry_price,
            exit_bar: bar,
            exit_price: price,
            pnl_pct: py_round(pnl_pct, 4),
            exit_reason: reason.to_string(),
            n_short_diffs: pos.completed.len() as u32,
            cost_basis_at_exit: py_round(pos.phase.cost_basis(), 6),
        });
        if pos.phase.is_earning() {
            self.counters.n_earning_reached += 1;
        }
        self.counters.n_open_rejects_zero += pos.n_open_rejects_zero as u64;
        self.counters.n_t5_shareconserving_after_earning +=
            pos.n_t5_shareconserving_after_earning as u64;
        if self.with_diag {
            let diffs = pos.trace.take().unwrap_or_default();
            self.res.diag.get_or_insert_with(Vec::new).push(TradeDiag {
                entry_bar: self.entry_bar,
                entry_price: self.entry_price,
                exit_bar: bar,
                exit_price: price,
                exit_reason: reason.to_string(),
                entry_ladder: self.entry_ladder,
                pnl_pct: py_round(pnl_pct, 4),
                cost_basis_exit: pos.phase.cost_basis(),
                total_shares_exit: pos.total_shares,
                reached_earning: pos.phase.is_earning(),
                diffs,
            });
        }
        for (key, cyc) in &pos.completed {
            let e = self.fsm_recovered.entry(key.py_key()).or_insert((0.0, 0));
            e.0 += cyc.profit();
            e.1 += 1;
        }
        self.res.ladder_attribution[self.entry_ladder] += 1;
        self.res.ladder_held_bars[self.entry_ladder] += held.max(0) as u64;
        self.reset_flat();
        self.entry_ladder = usize::MAX; // Python entry_ladder=-1（哨兵，不再被读）
    }
}

/// 主入口。`diag=true` 时返回逐 trade 快照 + 腿 trace。
pub fn run_organic(
    tape: &SignalTape,
    floor_ladder: usize,
    cfg: &OrganicConfig,
    stop_mode: StopMode,
    diag: bool,
) -> Result<RunResult, String> {
    // ── capability guards（Python 逐字 + v2 D3）──
    if floor_ladder < FIRST_BSP_LADDER {
        return Err(format!(
            "有机赋格要求 floor_ladder ≥ {FIRST_BSP_LADDER}（中枢承载层）；\
             bar/bi 级无 kind/中枢概念。floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("有机赋格要求事件磁带（bsp_events 全空）".to_string());
    }
    if cfg.rev_mode && !tape.has_div_events() {
        return Err("rev_mode 要求背驰磁带（div_events 全空）".to_string());
    }
    if cfg.rev_gate && !tape.has_run_high() {
        return Err(
            "rev_gate（41课门 v2 点态）要求磁带 run_high 行（D3）——当前磁带未产出，\
             清空路径(1) 创新高判定无数据基础（不提供 close 代理降级）"
                .to_string(),
        );
    }
    if cfg.sub_anchor != super::config::SubAnchor::Off && !tape.has_dir_rows() {
        return Err("sub_anchor（G1 锚定）要求磁带 dir_flips 行（D3）".to_string());
    }
    if cfg.tranche && !tape.has_dir_rows() {
        return Err("tranche（T4b/T5b 递归建仓）要求磁带 dir_flips 行（D3）".to_string());
    }
    if cfg.rev_paired && cfg.tranche {
        return Err(
            "rev_paired × tranche 组合未定义：配对闭腿（同锚 Buy1/ZD 触线）是腿级
             谓词，per-tranche 锚语义未设计——显式拒绝，不提供静默降级"
                .to_string(),
        );
    }

    // MarketMode 穷举（F1 期货实装时新增变体，编译器强制此处表态——v1R §2.2）。
    match cfg.market_mode {
        super::config::MarketMode::Stock => {}
    }

    let n = tape.bars.len();
    let mut run = Run {
        floor_ladder,
        stop_on: stop_mode.is_on(),
        state: FLAT,
        entry_bar: -1,
        entry_price: 0.0,
        entry_ladder: usize::MAX,
        arm_bar: -1,
        arm_ladder: LADDER_MOVE,
        pos: None,
        active_levels: Vec::new(),
        voices: Vec::new(),
        master_state: MasterState::Ride,
        book: CenterBook::new(),
        gate: FatigueGate::new(),
        alloc: SizeAllocator::new(),
        counters: Counters::default(),
        res: RunResult { diag: if diag { Some(Vec::new()) } else { None }, ..Default::default() },
        fsm_recovered: std::collections::BTreeMap::new(),
        with_diag: diag,
    };

    // 空行共享单例（Python NO_LADDER_EVENTS / NO_LADDER_DIVS 的零分配对应）。
    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
    // D3 滚动状态：稀疏翻转行 → 逐 bar 方向/锚视图（与密集行逐位等价，tape.rs）。
    let flips: &[(i64, u8, crate::stroke::Direction)] =
        tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<crate::stroke::Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let mut anchor_state: [i64; MAX_LADDER] = [-1; MAX_LADDER];
    let has_d3 = tape.has_dir_rows();
    // 中枢三态事件缓冲（每 bar 复用，仅 rev 消费路径填充）。
    let collect_center_events = cfg.rev_gate || cfg.tranche;
    let mut center_evs: [Vec<CenterEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;

        // ── D3 滚动状态推进（翻转行 bar 升序；同 bar 信号当 bar 可见）──
        while flip_ptr < flips.len() && flips[flip_ptr].0 == i as i64 {
            let (_, lad, dir) = flips[flip_ptr];
            dir_state[lad as usize] = Some(dir);
            anchor_state[lad as usize] = i as i64;
            flip_ptr += 1;
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] =
            sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 中枢生命周期账本（每 bar，市场性质；Python 逐字含 sentinel 守卫）──
        if collect_center_events {
            for row in center_evs.iter_mut() {
                row.clear();
            }
        }
        if sig.bsp_events.is_some() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                let out = if collect_center_events { Some(&mut center_evs[lad]) } else { None };
                run.book.ingest(lad, &evrows[lad], cfg.hard_type3, out);
            }
        }

        // ── FatigueGate（v2 点态：rev_gate 时每 bar 每承载层驱动——清空路径(1)
        //    创新高判定是逐 bar 价格事件，不能沿用 v1 的"仅事件 bar"调用门控）──
        if cfg.rev_gate {
            let rh = tape.run_high.as_deref().expect("guard 已验证 run_high 行存在");
            let run_high = &rh[i * MAX_LADDER..(i + 1) * MAX_LADDER];
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                run.gate.observe(
                    lad,
                    &evrows[lad],
                    &devrows[lad],
                    &center_evs[lad],
                    sig.up_move_settled.get(lad),
                    c,
                    run_high[lad],
                );
            }
            let mask = run.gate.fatigued_mask();
            for lad in 0..MAX_LADDER {
                if (mask >> lad) & 1 == 1 {
                    run.counters.fatigue_open_bars_by_ladder[lad] += 1;
                }
            }
        }

        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        if run.state == FLAT {
            // 取最高 buy1 层（Python 升序扫描覆盖 hi）
            let mut hi: i64 = -1;
            for k in FIRST_BSP_LADDER..max_l {
                if sig.buy1.get(k) {
                    hi = k as i64;
                }
            }
            if hi >= FIRST_BSP_LADDER as i64 {
                run.state = ARMED;
                run.arm_bar = i as i64;
                run.arm_ladder = hi as usize;
            }
        } else if run.state == ARMED {
            for k in FIRST_BSP_LADDER..max_l {
                if sig.buy1.get(k) && k > run.arm_ladder {
                    run.arm_ladder = k;
                }
            }
            // 区间套入场：次级别任意买点确认（[bar, arm_ladder) 任一层）
            let sub_mask = (1u16 << run.arm_ladder) - 1;
            let mut do_enter = sig.buy_any.0 & sub_mask != 0;
            if !do_enter && (i as i64 - run.arm_bar) > SUB_EXPIRY {
                do_enter = true;
            }
            if do_enter {
                run.open_position(i as i64, c, run.arm_ladder);
            } else if sig.sell1.get(run.arm_ladder) {
                run.state = FLAT;
            }
        } else {
            // ── LONG ──
            if cfg.sizing == Sizing::Structure {
                run.alloc.maybe_recompute(run.book.version, &run.active_levels, &run.book, c);
            }
            let entry_ladder = run.entry_ladder;
            let ev_entry = &evrows[entry_ladder];
            let dev_entry = &devrows[entry_ladder];

            // ── master 循环（45课持股持币；C1：出场只认 MasterExitSignal）──
            if run.stop_on && c < run.entry_price * (1.0 - STOP_FRAC) {
                run.res.n_core_stops += 1;
                run.close_position(i as i64, c, "stop_core_2pct");
            } else if run.master_state == MasterState::Rev {
                // earning 反作用下的 master REV 腿（entry 级先卖后买，43课；K5）
                let mrkey = SlotKey::rev(entry_ladder, entry_ladder);
                let pre = cfg.pre_type3
                    && ev_entry
                        .iter()
                        .any(|e| !e.confirmed && matches!(e.class, BspClass::Buy3));
                let hard = ev_entry
                    .iter()
                    .any(|e| e.confirmed && matches!(e.class, BspClass::Buy3));
                let sub_buy = entry_ladder >= 1 && sig.buy_any.get(entry_ladder - 1);
                let t5 = VoiceUnit::buy_trigger_at(cfg, ev_entry, dev_entry, sub_buy);
                if hard || pre || t5 {
                    if let Some(pos) = run.pos.as_mut() {
                        pos.close_diff(mrkey, c, i as i64);
                    }
                    run.counters.n_master_rev_close += 1;
                    run.master_state = MasterState::Ride;
                }
                // 升级出场（31课"历史性大顶"的级别相对化）在 REV 态同样有效
                let exit_lad = (entry_ladder + 1).min(sig.max_ladder as usize);
                if exit_lad > entry_ladder
                    && MasterExitSignal::from_sell1_row(sig.sell1.get(exit_lad)).is_some()
                {
                    run.counters.n_exit_upgraded += 1;
                    let reason = format!("exit_{}_earning_upgrade", ladder_name(exit_lad));
                    run.close_position(i as i64, c, &reason);
                }
            } else {
                // master RIDE：出场判定（C1——唯一输入是 policy 层 sell1 行）
                let trigger =
                    MasterExitSignal::from_sell1_row(sig.sell1.get(entry_ladder)).is_some();
                if trigger {
                    let earning =
                        run.pos.as_ref().is_some_and(|p| p.phase.is_earning());
                    if cfg.earning_reaction && earning {
                        // §5.6：清仓降格为 entry 级 REV 腿（金额守恒挣股数）
                        let mrkey = SlotKey::rev(entry_ladder, entry_ladder);
                        let opened = run
                            .pos
                            .as_mut()
                            .expect("LONG ⇒ pos 存在")
                            .open_diff(mrkey, 1.0, c, i as i64, LegAnchor::SegmentScale);
                        if opened {
                            run.counters.n_master_rev_open += 1;
                            run.master_state = MasterState::Rev;
                        } else {
                            let reason =
                                format!("exit_{}_type1sell", ladder_name(entry_ladder));
                            run.close_position(i as i64, c, &reason);
                        }
                    } else {
                        let reason = format!("exit_{}_type1sell", ladder_name(entry_ladder));
                        run.close_position(i as i64, c, &reason);
                    }
                } else if cfg.earning_reaction
                    && run.pos.as_ref().is_some_and(|p| p.phase.is_earning())
                {
                    let exit_lad = (entry_ladder + 1).min(sig.max_ladder as usize);
                    if exit_lad > entry_ladder
                        && MasterExitSignal::from_sell1_row(sig.sell1.get(exit_lad)).is_some()
                    {
                        run.counters.n_exit_upgraded += 1;
                        let reason = format!("exit_{}_earning_upgrade", ladder_name(exit_lad));
                        run.close_position(i as i64, c, &reason);
                    }
                }
            }

            if run.state != LONG {
                continue; // master 已清仓
            }

            // ── voice 声部（main 腿 = P4 离开段腿；osc/rev = VoiceUnit）──
            let rows = BarRows {
                evs: evrows,
                devs: devrows,
                buy_any: sig.buy_any,
                sell_any: sig.sell_any,
                dir_row: has_d3.then_some(&dir_state),
                run_anchor: has_d3.then_some(&anchor_state),
            };
            // frac 快照（11×f64 复制规避闭包对 run.alloc 的跨字段借用）
            let level_frac = run.pos.as_ref().expect("LONG ⇒ pos 存在").level_frac;
            let structure = cfg.sizing == Sizing::Structure;
            let alloc_frac: [f64; MAX_LADDER] = core::array::from_fn(|k| run.alloc.frac(k));
            let frac_of =
                move |k: usize| -> f64 { if structure { alloc_frac[k] } else { level_frac } };
            let pos = run.pos.as_mut().expect("LONG ⇒ pos 存在");
            for vi in 0..run.voices.len() {
                let ladder = run.voices[vi].ladder;
                let evs = &evrows[ladder];
                let devs = &devrows[ladder];
                // main 腿闭腿（P5 逐字：normal > pre > hard）
                let mkey = SlotKey::main(ladder);
                // anchor 是 Copy——复制快照规避借用冲突
                if let Some(anchor) = pos.open_slot(mkey).map(|l| l.anchor) {
                    if !evs.is_empty() {
                        let anchor_cs = match anchor {
                            LegAnchor::Center { cs, .. } => cs,
                            LegAnchor::SegmentScale => None,
                        };
                        let normal = evs.iter().any(|e| {
                            e.confirmed
                                && matches!(e.class, BspClass::Buy1)
                                && (!cfg.same_center_close || e.cs == anchor_cs)
                        });
                        let pre = cfg.pre_type3
                            && evs
                                .iter()
                                .any(|e| !e.confirmed && matches!(e.class, BspClass::Buy3));
                        let hard = cfg.hard_type3
                            && evs
                                .iter()
                                .any(|e| e.confirmed && matches!(e.class, BspClass::Buy3));
                        if normal {
                            pos.close_diff(mkey, c, i as i64);
                            run.counters.n_close_normal += 1;
                        } else if pre {
                            pos.close_diff(mkey, c, i as i64);
                            run.counters.n_close_pre_type3 += 1;
                        } else if hard {
                            pos.close_diff(mkey, c, i as i64);
                            run.counters.n_close_hard_type3 += 1;
                        }
                    }
                }
                // main 腿开腿（P5 逐字：首个合格事件，开后 break）
                if !cfg.open_kinds.is_empty()
                    && !evs.is_empty()
                    && pos.open_slot(mkey).is_none()
                    && !run.book.is_frozen(ladder)
                {
                    for e in evs {
                        if !(e.confirmed
                            && e.class.side() == crate::buysellpoint::Side::Sell
                            && cfg.open_kinds.contains(&e.class.kind()))
                        {
                            continue;
                        }
                        if cfg.center_gate {
                            let gate_pass = match (e.cs, e.zg) {
                                (Some(cs), Some(zg)) => {
                                    !run.book.is_dead(ladder, cs)
                                        && c > zg
                                        && (c - zg) / c >= cfg.theta_amp
                                }
                                _ => false, // cs is None（type1 无锚）
                            };
                            if !gate_pass {
                                run.counters.n_open_gate_rejects += 1;
                                continue;
                            }
                        }
                        pos.open_diff(
                            mkey,
                            frac_of(ladder),
                            c,
                            i as i64,
                            LegAnchor::Center {
                                cs: e.cs,
                                boundary: e.zg,
                                kind: AnchorKind::Bsp(e.class.kind()),
                            },
                        );
                        break;
                    }
                }
                // VoiceUnit（osc 域腿 + rev 段尺度腿）
                if cfg.rev_mode
                    && run.voices[vi].phase == VoicePhase::UpLeg
                    && (!evs.is_empty() || !devs.is_empty())
                    && VoiceUnit::rev_open_signal(cfg, evs, devs)
                {
                    run.res.rev_attempts_by_ladder[ladder] += 1;
                }
                let n_rev_before = run.counters.n_rev_open;
                run.voices[vi].step(
                    cfg,
                    &rows,
                    &center_evs[ladder],
                    c,
                    i as i64,
                    pos,
                    &run.book,
                    &run.gate,
                    entry_ladder,
                    &frac_of,
                    &mut run.counters,
                );
                if run.counters.n_rev_open > n_rev_before {
                    run.res.rev_opens_by_ladder[ladder] += 1;
                }
            }
            if sig.type2_buy {
                run.res.n_addon += 1;
            }
        }
    }

    if run.state == LONG && run.pos.as_ref().is_some_and(|p| p.total_shares > 0.0) {
        let last_close = tape.bars[n - 1].close;
        run.close_position(n as i64 - 1, last_close, "eod_close");
    }

    let mut res = run.res;
    res.counters = run.counters;
    res.leg_contribution =
        run.fsm_recovered.into_iter().map(|(k, (cash, cnt))| (k, cash, cnt)).collect();
    Ok(res)
}
