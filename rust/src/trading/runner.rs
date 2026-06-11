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
//! 当前磁带（organic_signals.py）已产出 D3 方向行（dir_flips 稀疏翻转，
//! tape.rs）⇒ sub_anchor/tranche 变体可运行；仅 run_high 行仍未产出 ⇒
//! rev_gate 变体 fail-fast（M2 信号层工位，引擎零改动边界）。

use super::allocator::SizeAllocator;
use super::center_book::CenterBook;
use super::config::{
    EntryMode, ExitMode, OrganicConfig, RevCycle, Sizing, StopMode, ThetaMode, VoiceMode,
};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::fatigue_gate::FatigueGate;
use super::ledger::{LegTrace, OrganicLedger};
use super::level_operating_unit::{BarRows, VoicePhase, VoiceUnit};
use super::master::{MasterExitSignal, MasterState};
use super::tape::SignalTape;
use super::trend_exhaustion::TrendExhaustion;
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
    /// 锚中枢相对振幅调研日志（每中枢一行 (ladder, seg_start, (ZG−ZD)/c)，
    /// 延伸取最终观测值；仅 diag=true 落盘——θ 自适应任务的分布调研面）。
    pub center_amp_log: Vec<(u8, i64, f64)>,
    /// master 递归建仓成交日志 (bar, ladder, frac, price)——含初始入场档
    /// 与每次追加档（entry_mode=Full 恒空表）。按 trade 的 [entry_bar,
    /// exit_bar] 区间切分可重建逐仓入场过程（均价/档数/到满仓 bar 数）。
    pub rec_fills: Vec<(i64, u8, f64, f64)>,
}

const FLAT: u8 = 0;
const ARMED: u8 = 1;
const LONG: u8 = 2;

struct Run {
    floor_ladder: usize,
    stop_on: bool,
    entry_mode: EntryMode,
    // ── 递归入场状态（entry_mode=Recursive；Full 模式为满仓哨兵）──
    /// 已消费的最高确认级别（追加只认 > 本值的 buy1——级别确认升级）。
    fill_ladder: usize,
    /// 已部署资金比例（唯一真相源 = pos.undeployed_cash，本值为其导出量）。
    filled_frac: f64,
    /// 下一档追加 quota（exp2 翻倍）。
    next_quota: f64,
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
        self.pos = Some(match self.entry_mode {
            EntryMode::Full => OrganicLedger::new(
                price,
                INITIAL_CAPITAL / price,
                level_frac,
                self.with_diag,
            ),
            EntryMode::Recursive { base_frac } => {
                let deployed = INITIAL_CAPITAL * base_frac;
                self.res.rec_fills.push((bar, el as u8, base_frac, price));
                OrganicLedger::with_reserve(
                    price,
                    deployed,
                    INITIAL_CAPITAL - deployed,
                    level_frac,
                    self.with_diag,
                )
            }
        });
        self.fill_ladder = el;
        self.filled_frac = match self.entry_mode {
            EntryMode::Full => 1.0,
            EntryMode::Recursive { base_frac } => base_frac,
        };
        self.next_quota = match self.entry_mode {
            EntryMode::Full => 0.0,
            EntryMode::Recursive { base_frac } => base_frac * 2.0,
        };
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
        // undeployed_cash：Full 模式恒 0.0（+0.0 位等价，O0≡P5 零接触）；
        // Recursive 未满仓出场时未部署现金按原值计入（资金守恒）。
        let total_value =
            pos.total_shares * price + pos.cumulative_recovered + pos.undeployed_cash;
        if pos.undeployed_cash > 0.0 {
            self.counters.n_rec_entry_partial_exits += 1;
        }
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
    if (cfg.r1_sub_sell_open || cfg.r2_anchor_zg || cfg.r3_t6_sub_confirm) && !cfg.rev_paired {
        return Err(
            "R1/R2/R3（盘背递归正则化）仅定义于 rev_paired 路径——legacy 腿无
             kind/锚概念，组合不可表示"
                .to_string(),
        );
    }
    if (cfg.sell2_open || cfg.buy2_close) && !cfg.rev_paired {
        return Err(
            "T2o/T2c（type2 开闭腿轴）仅定义于 rev_paired 路径——配对语义
             （震荡型同锚/逃逸型任意）在 legacy 腿上不可表示"
                .to_string(),
        );
    }
    if cfg.r1_sub_sell_open && !tape.has_dir_rows() {
        return Err(
            "r1_sub_sell_open（C段窗口判定）要求磁带 dir_flips 行（D3）——
             次级别 Up run 锚是窗口起点的数据基础（不提供同 bar 共现降级）"
                .to_string(),
        );
    }
    let sc_any = cfg.sc_entry_strict
        || cfg.sc_master_exit
        || cfg.sc_main_open
        || cfg.sc_main_close
        || cfg.sc_rev_open
        || cfg.sc_t7_close;
    if sc_any && !tape.has_dir_rows() {
        return Err(
            "SC 位（次级别确认完整递归）要求磁带 dir_flips 行（D3）——结构证据是
             bi 层确认的唯一数据基础（纯事件证据 = R1 空定义域反选，消融判决 §3.1）"
                .to_string(),
        );
    }
    if cfg.sc_rev_open && !cfg.rev_paired {
        return Err(
            "sc_rev_open 仅定义于 rev_paired 路径（与 R 位同先例——legacy 腿
             开腿语义不在本任务作用域）"
                .to_string(),
        );
    }
    if cfg.rev_sub_depth > 0 && !cfg.rev_paired {
        return Err(
            "rev_sub_depth（递归子 LOU）仅定义于 rev_paired 路径——legacy 腿无
             kind/锚概念，子腿配对闭腿不可表示（R/SC 位同先例）"
                .to_string(),
        );
    }
    if cfg.rev_sub_depth + 1 > MAX_REV_DEPTH {
        return Err(format!(
            "rev_sub_depth={} 超出路径键容量（MAX_REV_DEPTH={MAX_REV_DEPTH}：\
             路径 = 1 父段 + depth 子段）",
            cfg.rev_sub_depth
        ));
    }
    if cfg.rev_sub_depth > 0
        && cfg.sub_mode == super::config::SubMode::Fractal
        && !tape.has_dir_rows()
    {
        return Err(
            "sub_mode=Fractal（笔级分型子腿）要求磁带 dir_flips 行（D3）——
             顶/底分型确认信号 = 方向行翻转，无行即无信号源"
                .to_string(),
        );
    }
    if cfg.rev_l41_gate && !tape.has_dir_rows() {
        return Err(
            "rev_l41_gate（41课门 REV 主腿形态）要求磁带 dir_flips 行（D3）——
             父级别 Up/Down 段切分 = 方向行翻转，无行时追踪器恒无段对观测
             = 死门（不静默放行）"
                .to_string(),
        );
    }
    if (cfg.sub_cost_gate || cfg.sub_l41_gate)
        && !(cfg.rev_sub_depth > 0 && cfg.sub_mode == super::config::SubMode::Fractal)
    {
        return Err(
            "sub_cost_gate/sub_l41_gate（P1 双门）仅定义于 Fractal 子腿路径
             （rev_sub_depth > 0 ∧ sub_mode=Fractal）——Zhongshu 模式的逐锚
             2×sub_friction_rt 经济门已是其成本门形态，组合不可表示"
                .to_string(),
        );
    }
    if cfg.sub_cost_gate && !(cfg.sub_cost_k > 0.0 && cfg.sub_cost_k.is_finite()) {
        return Err(format!(
            "sub_cost_gate 要求 sub_cost_k 为正有限数（成本门下限 = k×friction）；\
             sub_cost_k={}",
            cfg.sub_cost_k
        ));
    }
    if cfg.rev_cycle == RevCycle::Cycle38 {
        if !cfg.rev_mode {
            return Err(
                "rev_cycle=Cycle38 要求 rev_mode=true——循环短差是 REV 族行为，\
                 rev_mode=false 下循环不可表示"
                    .to_string(),
            );
        }
        if !tape.has_trend_rows() {
            return Err(
                "rev_cycle=Cycle38 要求磁带 trend_flips 行（趋势态）——循环存续
                 条件 = 宿主尾 move kind==Trend，无行即无存续判据（不提供
                 方向行代理降级：方向 ≠ 趋势，17课趋势定义是 ≥2 同向中枢）"
                    .to_string(),
            );
        }
        if !tape.has_dir_rows() {
            return Err(
                "rev_cycle=Cycle38 要求磁带 dir_flips 行（D3）——宿主趋势态的
                 方向分量（kind==Trend ∧ dir==Up）依赖方向行"
                    .to_string(),
            );
        }
        if cfg.rev_sub_depth > 0 {
            return Err(
                "rev_cycle=Cycle38 × rev_sub_depth 组合未定义：循环腿无固定
                 REV 窗口（逐次开闭），子 LOU 的域语义未设计——显式拒绝"
                    .to_string(),
            );
        }
        if cfg.tranche {
            return Err(
                "rev_cycle=Cycle38 × tranche 组合未定义：循环腿是单 tranche
                 逐次开闭，递归建仓语义未设计——显式拒绝"
                    .to_string(),
            );
        }
        if !(cfg.theta_cost_k > 0.0
            && cfg.theta_cost_k.is_finite()
            && cfg.friction_rt > 0.0
            && cfg.friction_rt.is_finite())
        {
            return Err(format!(
                "rev_cycle=Cycle38 成本门要求 theta_cost_k/friction_rt 为正有限数；\
                 theta_cost_k={} friction_rt={}",
                cfg.theta_cost_k, cfg.friction_rt
            ));
        }
        if cfg.rev_cycle_close == super::config::RevCycleClose::Paired
            && (cfg.r2_anchor_zg
                || cfg.r3_t6_sub_confirm
                || cfg.sc_t7_close
                || cfg.buy2_close)
        {
            return Err(
                "rev_cycle_close=Paired × {r2_anchor_zg/r3_t6_sub_confirm/\
                 sc_t7_close/buy2_close} 组合未定义：Paired 闭腿集是
                 step_down_paired 在这四位全关时的精确退化形式（T7>T6>同锚
                 Buy1>ZD），且其证据记忆（sub_buy_bar）在 Cycle38 模式不推进
                 ——开位即声明膨胀，显式拒绝"
                    .to_string(),
            );
        }
    }

    if let EntryMode::Recursive { base_frac } = cfg.entry_mode {
        if !(base_frac > 0.0 && base_frac < 1.0 && base_frac.is_finite()) {
            return Err(format!(
                "entry_mode=Recursive 要求 0 < base_frac < 1（base_frac=1 是 Full
                 的冗余表示——声明=能力，显式拒绝）；base_frac={base_frac}"
            ));
        }
    }

    // ── master 出场状态驱动 guard（2026-06-11 任务，exit_mode 轴）──
    if cfg.exit_mode == ExitMode::HoldTrend && !tape.has_dir_rows() {
        return Err(
            "exit_mode=HoldTrend（趋势态出场）要求磁带 dir_flips 行（D3）——
             父级别 Up 段切分 = 方向行翻转，无行时追踪器恒无段对观测 ⇒
             up_unexhausted 恒 false ⇒ 静默退化为 Signal 行为（声明=能力，
             不提供静默降级）"
                .to_string(),
        );
    }
    if matches!(cfg.exit_mode, ExitMode::Emergent { .. }) && !tape.has_dir_rows() {
        return Err(
            "exit_mode=Emergent（出场级别 = 持仓走势的涌现级别归属）要求磁带
             dir_flips 行（D3）——归属判据 = 父级别方向行连续 Up 的逐层递归，
             无行即无涌现读数（不提供 max_ladder 全局塔高代理降级：塔高是
             外部参数非走势自身涌现，第一版 Climb 设计缺陷先例）"
                .to_string(),
        );
    }
    // ── master 入场级别下限 guard（2026-06-11 任务，entry_min_ladder 轴）──
    if cfg.entry_min_ladder != 0
        && !(FIRST_BSP_LADDER..MAX_LADDER).contains(&cfg.entry_min_ladder)
    {
        return Err(format!(
            "entry_min_ladder 必须为 0（无约束）或落在 [{FIRST_BSP_LADDER}, \
             {MAX_LADDER})（BSP 承载层区间）；entry_min_ladder={}",
            cfg.entry_min_ladder
        ));
    }
    if cfg.exit_mode != ExitMode::Signal && cfg.earning_reaction {
        return Err(
            "exit_mode≠Signal × earning_reaction 组合未定义：earning 升级出场
             /REV 腿降格的出场语义建立在 entry 级 sell1 事件驱动之上，状态
             驱动形态未设计——显式拒绝（Cycle38×tranche 同先例）"
                .to_string(),
        );
    }

    // ── 账本 voice guard（2026-06-11 并发赋格最小实验）──
    // Ledger 是裸账本消费（无相位/无锚/无门/无配对）：FSM 与 main 腿的全部
    // 机制位在账本路径不被消费，保留非关闭值即声明膨胀（090号）——显式拒绝。
    if cfg.voice_mode == VoiceMode::Ledger {
        let fsm_bits_off = !cfg.rev_mode
            && !cfg.tranche
            && cfg.rev_sub_depth == 0
            && cfg.rev_cycle == RevCycle::Single
            && !cfg.earning_reaction
            && !cfg.sc_main_open
            && !cfg.sc_main_close
            && !cfg.sc_t7_close
            && cfg.open_kinds.is_empty()
            && !cfg.hard_type3
            && !cfg.pre_type3
            && !cfg.center_gate
            && !cfg.same_center_close
            && !cfg.osc_mode;
        if !fsm_bits_off {
            return Err(
                "voice_mode=Ledger 是裸账本消费（无相位/无锚/无门）——FSM/main 腿
                 机制位（rev_mode/tranche/rev_sub_depth/rev_cycle/earning_reaction/
                 SC 位/open_kinds/hard_type3/pre_type3/center_gate/
                 same_center_close/osc_mode）必须全关：账本路径不消费这些位，
                 保留即声明膨胀"
                    .to_string(),
            );
        }
    } else if cfg.ledger_sell_t1_only {
        return Err(
            "ledger_sell_t1_only 仅定义于 voice_mode=Ledger 路径（FSM 路径
             不消费该位——声明=能力，显式拒绝）"
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
        entry_mode: cfg.entry_mode,
        fill_ladder: usize::MAX,
        filled_frac: 1.0,
        next_quota: 0.0,
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
    // 趋势态滚动状态（稀疏翻转行 → 逐 bar 视图；dir_state 同构）。
    let tflips: &[(i64, u8, bool)] = tape.trend_flips.as_deref().unwrap_or(&[]);
    let mut tflip_ptr = 0usize;
    let mut trend_state: [bool; MAX_LADDER] = [false; MAX_LADDER];
    let has_trend = tape.has_trend_rows();
    // 中枢三态事件缓冲（每 bar 复用，仅 rev 消费路径填充）。
    let collect_center_events = cfg.rev_gate || cfg.tranche;
    let mut center_evs: [Vec<CenterEvent>; MAX_LADDER] = Default::default();
    // 锚中枢振幅因果参照（θ 自适应深度门 + 振幅分布调研日志）。局部变量而非
    // Run 字段——BarRows 持其只读借用横跨 LONG 块，与 close_position(&mut run)
    // 的字段借用不相容。
    let depth_window = match cfg.theta_mode {
        ThetaMode::AdaptiveQuantile { window, .. } => window,
        ThetaMode::Fixed => DEPTH_REF_WINDOW,
    };
    let mut depth_ref = DepthRef::new(depth_window);
    // 父级别走势衰竭追踪器（41课门；市场性质，与 depth_ref 同置局部变量
    // ——BarRows 持只读借用横跨 LONG 块）。sub_l41_gate（Fractal 子腿，Down
    // 侧）或 rev_l41_gate（REV 主腿，Up 侧）任一变体实例化。
    let mut trend_exh = (cfg.sub_l41_gate
        || cfg.rev_l41_gate
        || cfg.exit_mode == ExitMode::HoldTrend
        || cfg.exit_mode == (ExitMode::Emergent { hold_trend: true }))
        .then(TrendExhaustion::new);

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
        while tflip_ptr < tflips.len() && tflips[tflip_ptr].0 == i as i64 {
            let (_, lad, is_trend) = tflips[tflip_ptr];
            trend_state[lad as usize] = is_trend;
            tflip_ptr += 1;
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] =
            sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 走势衰竭追踪（P1 41课门；市场性质，全态每 bar 驱动）──
        if let Some(te) = trend_exh.as_mut() {
            te.observe(&dir_state, devrows, c);
        }

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
            // 振幅参照观测（市场性质，与持仓状态无关；边界变化才记录）。
            depth_ref.observe(&run.book, c);
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
            // 取最高 buy1 层（Python 升序扫描覆盖 hi）。entry_min_ladder 只抬高
            // 布防下界（对称升级轴）——ARMED 升级/区间套次级别确认零改动。
            let arm_floor = FIRST_BSP_LADDER.max(cfg.entry_min_ladder);
            let mut hi: i64 = -1;
            for k in arm_floor..max_l {
                if sig.buy1.get(k) {
                    hi = k as i64;
                }
            }
            if hi >= arm_floor as i64 {
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
                if cfg.sc_entry_strict {
                    // SCe：严格区间套——超时不强制入场，继续等待次级别确认
                    // 或 sell1 撤防（27课：无次级别把握的买点不操作）。
                    if (i as i64 - run.arm_bar) == SUB_EXPIRY + 1 {
                        run.counters.n_sc_entry_expire_skips += 1;
                    }
                } else {
                    do_enter = true;
                }
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
            // rows 构造前移到 master 段之前（SC 位的 sub_confirm 数据视图；
            // 纯引用结构，零拷贝——voice 循环共用同一实例）。
            let rows = BarRows {
                evs: evrows,
                devs: devrows,
                buy_any: sig.buy_any,
                sell_any: sig.sell_any,
                dir_row: has_d3.then_some(&dir_state),
                run_anchor: has_d3.then_some(&anchor_state),
                depth: Some(&depth_ref),
                l41: trend_exh.as_ref(),
                trend_row: has_trend.then_some(&trend_state),
            };

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
                let sc_m = !cfg.sc_master_exit
                    || rows.sub_confirm(exit_lad, crate::buysellpoint::Side::Sell);
                if exit_lad > entry_ladder && sig.sell1.get(exit_lad) && !sc_m {
                    run.counters.n_sc_master_holds += 1;
                }
                if exit_lad > entry_ladder
                    && MasterExitSignal::from_sell1_row_sub_confirmed(
                        sig.sell1.get(exit_lad),
                        sc_m,
                    )
                    .is_some()
                {
                    run.counters.n_exit_upgraded += 1;
                    let reason = format!("exit_{}_earning_upgrade", ladder_name(exit_lad));
                    run.close_position(i as i64, c, &reason);
                }
            } else {
                // master RIDE：出场判定（C1——唯一输入是 policy 层 sell1 行；
                // SCm：sell1 ∧ 次级别卖确认——27课区间套的出场时机细化）。
                // exit_mode 选择判据级别：Signal/HoldTrend = entry 级（在册）；
                // HighestOnly = 当前最高涌现层（31课"历史性大顶"的级别相对化）；
                // Emergent = 持仓走势的级别归属——从 entry 级向上，父级别
                // 当前段为持仓期间生长出的 Up 段（dir==Up ∧ 段锚 ≥
                // entry_bar）的连续最高层。两个条件缺一不可：
                //   dir==Up 单独成立 = "市场环境向上"（牛市全塔 Up，链
                //   直通塔顶 = HighestOnly 退化——本判据第一版缺陷，OKLO
                //   em_bars=292K/447K 暴露）；
                //   anchor ≥ entry_bar = 该 Up 段在持仓期间被父级别同级别
                //   分解确认 = 它是由持仓走势自身生长出来的（中枢扩展/
                //   新中枢形成 → 走势级别上升的方向行投影）。
                // 无状态每 bar 重读，高层翻转归属即时回落——保留出场能力。
                // 与入场区间套对称：入场看 buy1 涌现级别，出场看走势涌现
                // 级别。不用 max_ladder 全局塔高——塔高是外部参数非走势
                // 自身涌现（第一版 Climb 设计缺陷）。
                let mx_lad = match cfg.exit_mode {
                    ExitMode::Signal | ExitMode::HoldTrend => entry_ladder,
                    ExitMode::HighestOnly => {
                        (sig.max_ladder as usize).min(MAX_LADDER - 1)
                    }
                    ExitMode::Emergent { .. } => {
                        let mut lad = entry_ladder;
                        while lad + 1 < max_l
                            && dir_state[lad + 1]
                                == Some(crate::stroke::Direction::Up)
                            && anchor_state[lad + 1] >= run.entry_bar
                        {
                            lad += 1;
                        }
                        if lad > entry_ladder {
                            run.counters.n_exit_emergent_bars += 1;
                        }
                        lad
                    }
                };
                let sc_m = !cfg.sc_master_exit
                    || rows.sub_confirm(mx_lad, crate::buysellpoint::Side::Sell);
                if sig.sell1.get(mx_lad) && !sc_m {
                    run.counters.n_sc_master_holds += 1;
                }
                let sig_trigger = MasterExitSignal::from_sell1_row_sub_confirmed(
                    sig.sell1.get(mx_lad),
                    sc_m,
                )
                .is_some();
                // HoldTrend（49课利润最大化 + 41课）：本级别 sell1 只是必要
                // 条件，还需父级别（entry_ladder+1）上行趋势衰竭——正面延续
                // 证据（相邻同向段创新高 ∧ 无盘整背驰）成立时拦截出场持仓，
                // sell1 的短差机会由 voice 承载（在册逻辑零接触）。
                // 衰竭确认的父级别：HoldTrend = entry_ladder+1（静态在册）；
                // Emergent{ht} = mx_lad+1（随归属动态上移——出场判据归属到
                // 哪一级，衰竭就在哪一级的直接父级别上确认）。
                let ht_parent = match cfg.exit_mode {
                    ExitMode::HoldTrend => Some(entry_ladder + 1),
                    ExitMode::Emergent { hold_trend: true } => Some(mx_lad + 1),
                    _ => None,
                };
                let trigger = match ht_parent {
                    None => sig_trigger,
                    Some(parent) => {
                        let unexh = rows
                            .l41
                            .expect("guard: HoldTrend/Emergent{ht} ⇒ TrendExhaustion 实例化")
                            .up_unexhausted(parent);
                        if sig_trigger && unexh {
                            run.counters.n_exit_trend_holds += 1;
                        }
                        sig_trigger && !unexh
                    }
                };
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
                        // reason 按模式区分（Signal 逐字在册——O0≡P5 守卫面）
                        let reason = match cfg.exit_mode {
                            ExitMode::Signal => {
                                format!("exit_{}_type1sell", ladder_name(entry_ladder))
                            }
                            ExitMode::HoldTrend => format!(
                                "exit_{}_type1sell_trendexh",
                                ladder_name(entry_ladder)
                            ),
                            ExitMode::HighestOnly => format!(
                                "exit_{}_type1sell_highest",
                                ladder_name(mx_lad)
                            ),
                            ExitMode::Emergent { hold_trend: false } => format!(
                                "exit_{}_type1sell_emergent",
                                ladder_name(mx_lad)
                            ),
                            ExitMode::Emergent { hold_trend: true } => format!(
                                "exit_{}_type1sell_emergentht",
                                ladder_name(mx_lad)
                            ),
                        };
                        run.close_position(i as i64, c, &reason);
                    }
                } else if cfg.earning_reaction
                    && run.pos.as_ref().is_some_and(|p| p.phase.is_earning())
                {
                    let exit_lad = (entry_ladder + 1).min(sig.max_ladder as usize);
                    let sc_u = !cfg.sc_master_exit
                        || rows.sub_confirm(exit_lad, crate::buysellpoint::Side::Sell);
                    if exit_lad > entry_ladder && sig.sell1.get(exit_lad) && !sc_u {
                        run.counters.n_sc_master_holds += 1;
                    }
                    if exit_lad > entry_ladder
                        && MasterExitSignal::from_sell1_row_sub_confirmed(
                            sig.sell1.get(exit_lad),
                            sc_u,
                        )
                        .is_some()
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

            // ── master 递归建仓追加（entry_mode=Recursive；2026-06-11 任务）──
            // 更高级别 confirmed buy1 = 转折的级别确认升级 → 追加下一档 quota
            // （exp2 翻倍）；buy1 落在当前最高涌现层 = "最高级别确认" → 补满
            // 剩余全部。出场（entry_ladder 的 sell1）与 voice 声部零改动。
            if matches!(run.entry_mode, EntryMode::Recursive { .. }) && run.filled_frac < 1.0 {
                let mut k_hi: Option<usize> = None;
                for k in (run.fill_ladder + 1)..max_l {
                    if sig.buy1.get(k) {
                        k_hi = Some(k);
                    }
                }
                if let Some(k) = k_hi {
                    let pos = run.pos.as_mut().expect("LONG ⇒ pos 存在");
                    let cash = if k == sig.max_ladder as usize {
                        pos.undeployed_cash // 最高级别确认 → 补满
                    } else {
                        (INITIAL_CAPITAL * run.next_quota).min(pos.undeployed_cash)
                    };
                    if pos.add_entry_tranche(cash, c) {
                        run.fill_ladder = k;
                        run.next_quota *= 2.0;
                        run.counters.n_rec_entry_fills += 1;
                        run.res.rec_fills.push((i as i64, k as u8, cash / INITIAL_CAPITAL, c));
                        if pos.undeployed_cash <= 0.0 {
                            run.filled_frac = 1.0;
                            run.counters.n_rec_entry_full += 1;
                        } else {
                            run.filled_frac = 1.0 - pos.undeployed_cash / INITIAL_CAPITAL;
                        }
                    } else {
                        run.counters.n_rec_entry_earning_rejects += 1;
                    }
                }
            }

            // ── voice 声部（main 腿 = P4 离开段腿；osc/rev = VoiceUnit）──
            // frac 快照（11×f64 复制规避闭包对 run.alloc 的跨字段借用）
            let level_frac = run.pos.as_ref().expect("LONG ⇒ pos 存在").level_frac;
            let structure = cfg.sizing == Sizing::Structure;
            let alloc_frac: [f64; MAX_LADDER] = core::array::from_fn(|k| run.alloc.frac(k));
            let frac_of =
                move |k: usize| -> f64 { if structure { alloc_frac[k] } else { level_frac } };
            let pos = run.pos.as_mut().expect("LONG ⇒ pos 存在");

            // ── 账本 voice（voice_mode=Ledger，2026-06-11 并发赋格最小实验）──
            // voice@k 的唯一状态 = 腿槽占用（资源状态）；每个 confirmed BSP
            // 直接饱和执行：Sell@k 且槽空 → 卖出 frac_k（清 slice），Buy@k
            // 且槽开 → 买回（填 slice）。无相位过滤、无锚比较、无门谓词。
            // 同 bar 多事件按磁带序（ladder 升序、层内事件序）确定性消费。
            // master 出场时 close_position 的强制清腿在册逻辑自动覆盖账本槽。
            if cfg.voice_mode == VoiceMode::Ledger {
                for &ladder in &run.active_levels {
                    let key = SlotKey::main(ladder);
                    for e in &evrows[ladder] {
                        if !e.confirmed {
                            continue;
                        }
                        match e.class.side() {
                            crate::buysellpoint::Side::Sell => {
                                if cfg.ledger_sell_t1_only && e.class != BspClass::Sell1 {
                                    continue;
                                }
                                if pos.open_slot(key).is_none() {
                                    if pos.open_diff(
                                        key,
                                        frac_of(ladder),
                                        c,
                                        i as i64,
                                        LegAnchor::SegmentScale,
                                    ) {
                                        run.counters.n_ledger_opens += 1;
                                    }
                                } else {
                                    run.counters.n_ledger_sell_noops += 1;
                                }
                            }
                            crate::buysellpoint::Side::Buy => {
                                if pos.open_slot(key).is_some() {
                                    pos.close_diff(key, c, i as i64);
                                    run.counters.n_ledger_closes += 1;
                                } else {
                                    run.counters.n_ledger_buy_noops += 1;
                                }
                            }
                        }
                    }
                }
                if sig.type2_buy {
                    run.res.n_addon += 1;
                }
                continue;
            }

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
                        let normal_raw = evs.iter().any(|e| {
                            e.confirmed
                                && matches!(e.class, BspClass::Buy1)
                                && (!cfg.same_center_close || e.cs == anchor_cs)
                        });
                        // SCc：同锚 Buy1 闭腿 ∧ 次级别买确认（27课区间套的
                        // 买回时机细化；pre/hard type3 通道不受 SCc 影响）
                        let normal = normal_raw
                            && (!cfg.sc_main_close
                                || rows.sub_confirm(ladder, crate::buysellpoint::Side::Buy));
                        if normal_raw && !normal {
                            run.counters.n_sc_main_close_holds += 1;
                        }
                        let pre = cfg.pre_type3
                            && evs
                                .iter()
                                .any(|e| !e.confirmed && matches!(e.class, BspClass::Buy3));
                        // SC7：main hard（confirmed Buy3）∧ 次级别买确认
                        let hard_raw = cfg.hard_type3
                            && evs
                                .iter()
                                .any(|e| e.confirmed && matches!(e.class, BspClass::Buy3));
                        let hard = hard_raw
                            && (!cfg.sc_t7_close
                                || rows.sub_confirm(ladder, crate::buysellpoint::Side::Buy));
                        if hard_raw && !hard {
                            run.counters.n_sc_t7_holds += 1;
                        }
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
                        // SCo：降成本腿卖开 ∧ 次级别卖确认（27课区间套的
                        // 卖出时机细化——次级别向上走势结束证据）
                        if cfg.sc_main_open
                            && !rows.sub_confirm(ladder, crate::buysellpoint::Side::Sell)
                        {
                            run.counters.n_sc_main_open_rejects += 1;
                            continue;
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
    if diag {
        res.center_amp_log = depth_ref.take_log();
    }
    res.leg_contribution =
        run.fsm_recovered.into_iter().map(|(k, (cash, cnt))| (k, cash, cnt)).collect();
    Ok(res)
}
