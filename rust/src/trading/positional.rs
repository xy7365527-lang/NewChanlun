//! positional — 多级别仓位分层（positional fugue，26/44课直接形式化）。
//!
//! 设计：`analysis/positional_fugue_design.md`（D1-D9 决断表 + 预注册判据 P1-P3）。
//!
//! 与 runner.rs（单体 master）的范畴差：不是"一个 FSM 拿全仓"，是
//! "每个 BSP 承载层一个独立 45课二元 FSM，各拿涌现配额"。
//!   - 入场：buy1@k 布防 + 次级别区间套确认（与在册 master 逐字同构，per-layer 化）；
//!   - 出场：sell1@k 只清**本层**股数（44课："不可能按30分钟操作，一见1分钟
//!     顶背驰就全部扔掉"）；
//!   - 配额：w_k = θ_k / Σ_j θ_j，θ_k = DepthRef 因果滚动 P50 中枢相对振幅
//!     （26课"级别的意义基本只和买卖量有关" + 38课"历史上某级别平均震荡幅度"
//!     ——配额从结构涌现，零预设百分比；常数复用在册 SUB_COST_Q/
//!     SUB_COST_MIN_OBS/DEPTH_REF_WINDOW，零新参数）；
//!   - 层间：无级联清仓、无消息传递（27课区间套定理：大级别转折结构上必然
//!     伴随各级别自身卖点——低级别层在自己的事件流里自然获知）。
//!
//! 会计不变量：NAV = pool + Σ_k shares_k×c；bar 内出场/入场都是现金↔股数的
//! 等价转换（close 单一成交价），NAV 在 bar 内不因交易而变 ⇒ 阶段 2 用
//! 阶段 1 后的 NAV 快照定目标金额是严格的。
//!
//! run_organic 路径零接触（O0≡P5 守卫面不受影响）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::tape::SignalTape;
use super::types::{FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER, SUB_EXPIRY};

/// NAV 采样间隔（1min bar 口径 ≈ 1 日；分年 nats 分解的数据基础）。
pub const EQUITY_SAMPLE_BARS: i64 = 1440;
/// 防尘埃最小成交比例（D9：实际可成交 < 1%×目标金额 ⇒ 推迟而非开尘埃腿）。
pub const MIN_FILL_FRAC: f64 = 0.01;

/// 层 FSM 状态（45课持股/持币二元循环的 per-layer 形态 + 资金推迟态）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LayerState {
    /// 持币：等待本级别 buy1。
    Flat,
    /// 布防：buy1@k 已现，等待次级别区间套确认（在册 master ARMED 同构）。
    Armed { arm_bar: i64 },
    /// 确认已成立但资金不可用（pool 不足）——推迟入场（D7/D9）。
    /// sell1@k 取消（该买点起始的走势类型已被宣告结束）。
    Pending { confirm_bar: i64 },
    /// 持股：本层股数 + 入场快照。
    Long {
        entry_bar: i64,
        entry_price: f64,
        shares: f64,
        weight: f64,
        deferred_bars: i64,
        partial: bool,
    },
}

/// 层级 trade 记录（每层每个持股周期一条；股数守恒：进出同股数）。
#[derive(Debug, Clone)]
pub struct LayerTrade {
    pub ladder: u8,
    pub entry_bar: i64,
    pub entry_price: f64,
    pub exit_bar: i64,
    pub exit_price: f64,
    pub shares: f64,
    /// 入场时刻的涌现配额 w_k（归因/诊断）。
    pub weight_at_entry: f64,
    /// 确认 bar → 实际入场 bar 的推迟（0 = 当 bar 成交）。
    pub deferred_bars: i64,
    /// 实际成交 < 目标金额（pool 部分充足）。
    pub partial: bool,
    /// "sell1" | "eod"。
    pub exit_reason: &'static str,
}

#[derive(Debug, Default)]
pub struct PositionalResult {
    pub trades: Vec<LayerTrade>,
    /// (bar, nav) 采样（含末 bar）。
    pub equity: Vec<(i64, f64)>,
    pub final_nav: f64,
    pub n_entries_by_ladder: [u64; MAX_LADDER],
    pub n_exits_by_ladder: [u64; MAX_LADDER],
    pub held_bars_by_ladder: [u64; MAX_LADDER],
    /// ARMED 期 sell1 撤防数。
    pub n_disarms_by_ladder: [u64; MAX_LADDER],
    /// PENDING 期 sell1 取消数（资金始终未释放、买点过期）。
    pub n_pending_cancels_by_ladder: [u64; MAX_LADDER],
    /// 确认成立但 θ_k 无参照（warm-up/级别未统计涌现）的入场跳过数。
    pub n_noref_skips_by_ladder: [u64; MAX_LADDER],
    /// 部分成交入场数。
    pub n_partial_by_ladder: [u64; MAX_LADDER],
    /// 推迟发生数（确认 bar 资金不足进入 Pending）。
    pub n_deferred_by_ladder: [u64; MAX_LADDER],
    // ── Fusion（B+C 合体）观测面；legacy 模式恒零（positional_fusion.rs）──
    /// 趋势相停削数（49课:52 满仓——本层卖点在趋势相不削减）。
    pub n_trend_holds_by_ladder: [u64; MAX_LADDER],
    /// 049:54 趋势顶背驰全抛数（div_exit：趋势相内 sell1@k ⇒ 出清本层）。
    pub n_trend_div_exits_by_ladder: [u64; MAX_LADDER],
    /// 41课衰竭门拒绝停削数（父层向下未衰竭 ⇒ 趋势相不成立，卖点照常削减）。
    pub n_gate41_blocks_by_ladder: [u64; MAX_LADDER],
    /// 层内 C 短差开（53课次级别卖证据全抛本层 slice）。
    pub n_sub_opens_by_ladder: [u64; MAX_LADDER],
    /// 短差回补（k−1 镜像买证据，"如数接回"——含推迟后补完）。
    pub n_sub_restores_by_ladder: [u64; MAX_LADDER],
    /// 本层 k 级买点通道的回补（26课"任何的买点都是买点"）。
    pub n_sub_kbuy_restores_by_ladder: [u64; MAX_LADDER],
    /// 44课铰链恶化升级（本层卖点先到 ⇒ 短差卖出升级为减仓出清）。
    pub n_sub_escalates_by_ladder: [u64; MAX_LADDER],
    /// 趋势相开始强制回补（49课:52 满仓义务）。
    pub n_sub_phase_closes_by_ladder: [u64; MAX_LADDER],
    /// 35课成本门拒开（θ_q(k−1) < k×friction）。
    pub n_sub_cost_rejects_by_ladder: [u64; MAX_LADDER],
    /// 成本门参照不可定义拒开（warm-up，保守拒绝不静默放行）。
    pub n_sub_noref_rejects_by_ladder: [u64; MAX_LADDER],
    /// 回补义务因资金不足推迟的 bar 数（D7 推迟同构；decoupled 下判据 =
    /// escrow + pool 不足，专款兜底后仅追价 deficit 仍可推迟）。
    pub n_sub_restore_defer_bars: u64,
    /// 解耦回补中专款不足、由共享池补差的次数（追价 regime 观测——
    /// 回补成本 > 卖出所得 ⇔ 买回价 > 卖出价）。耦合模式恒零。
    pub n_sub_pool_topup_by_ladder: [u64; MAX_LADDER],
    /// 层内短差净现金（Σ 卖出所得 − 接回成本；降成本的会计读数）。
    pub sub_net_cash_by_ladder: [f64; MAX_LADDER],
}

/// θ 配额表：对 [floor, MAX_LADDER) 各层取 DepthRef P50；Σ 只跨有定义的层
/// （级别的统计涌现性，D2）。ladder 升序累加（确定性求和序）。
pub(crate) fn theta_weights(
    depth_ref: &DepthRef,
    floor_ladder: usize,
) -> ([Option<f64>; MAX_LADDER], f64) {
    let mut thetas: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
    let mut total = 0.0f64;
    for (k, slot) in thetas.iter_mut().enumerate().take(MAX_LADDER).skip(floor_ladder) {
        if let Some(t) = depth_ref.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
            if t > 0.0 && t.is_finite() {
                *slot = Some(t);
                total += t;
            }
        }
    }
    (thetas, total)
}

fn nav_of(layers: &[LayerState; MAX_LADDER], cash: f64, c: f64, floor: usize) -> f64 {
    let mut v = cash;
    for layer in layers.iter().take(MAX_LADDER).skip(floor) {
        if let LayerState::Long { shares, .. } = layer {
            v += shares * c;
        }
    }
    v
}

/// T 轴（趋势相停削）严格化选项——趋势态停削机制研究任务（2026-06-12，
/// hold26 L3"牛市 α 6/6 全负"开放轴 1 + fusion 判决 §5.4 F5 修复）。
/// 三选项独立可消融（VLg 负交互先例 ⇒ 组合臂预注册）；全 false = 在册
/// fusion_t 逐字行为（在册判决零漂移由构造保证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrendAxisOpts {
    /// g — 41课:22 衰竭门："如果一个小级别的买卖点和大级别的走势方向相反，
    /// 而该大级别走势没有任何衰竭，这时候参与小级别买卖点，就意味着要冒着
    /// 大级别走势延续的风险"。父层（k+1）dir==Down 且本方向 run 内无向下
    /// 背驰事件（未衰竭，41课:28"没有进入背驰段，就不能操作"）⇒ 层 k
    /// 趋势相不成立（bear rally 停削漏出的修复——F5 否证 −0.13~−0.23 nats）。
    pub gate41: bool,
    /// d — 49课:54 背驰出场："如果这个中枢完成的向上移动出现背驰，就要把
    /// 所有筹码抛出，因为这个级别的走势类型完成"。趋势相内 sell1@k
    /// （type1 = 趋势顶背驰词汇）⇒ 全抛本层（exit_reason="trend_div"），
    /// 其余卖点仍停削（049:60"中途不参与短差"）。
    pub div_exit: bool,
    /// b — 49课:60 三买起点："在中枢第三类买点后持股直到新中枢出现继续
    /// 中枢震荡操作，中途不参与短差"。confirmed Buy3@k ⇒ 开三买窗口
    /// （覆盖 kind 尚为盘整、第二中枢未结算的首次离开段——17课 kind 判据
    /// 的结构性滞后区）；窗口关闭 = 新中枢事件（事件 cs > 锚 cs）∨ 向上
    /// 背驰/盘背@k（049:42/46）∨ dir 翻 Down。
    pub b3_start: bool,
}

impl TrendAxisOpts {
    pub fn any(self) -> bool {
        self.gate41 || self.div_exit || self.b3_start
    }
}

/// 仓位极性模式——v1 否证（BTC L2：P1 ✗ −3.32 nats / P3 ✗ +152%）后从
/// 26课:34 字面回读出的范畴对立：
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarityMode {
    /// v1：每层独立 45课持股/持币循环，**从持币开始**——buy1@k 布防 +
    /// 区间套确认入场，sell1@k 清层。否证形态保留（生成史 + 对照基线）：
    /// θ 配额是预留制，高层 FLAT 期配额闲置 = 结构性现金拖累。
    Cycle45,
    /// v2：26课恒仓——"本ID的仓位是一直不变的……根据不同级别的卖点把仓位
    /// 减少，买点的时候又回复原来的数量，但绝对不加仓"（026:34 逐字）。
    /// 振荡围绕**满仓**：slice 默认持有；confirmed 卖点@k 削减本层 slice，
    /// confirmed 买点@k 回复（任何买点都是买点——26课）。每 slice 周期内
    /// 股数守恒（卖出 = 周期买入的全部股数）。confirmed 已含次级别完成
    /// 判定（四案收敛）⇒ 直接消费，无 ARMED 相位。
    Hold26 {
        /// 卖点词汇：true = 仅 type1（VLs1 先例）；false = 任意卖点（26课字面）。
        sell_t1_only: bool,
    },
    /// v3：B+C 合体（hold26 × CounterSeg 合流，2026-06-12 任务；
    /// `positional_fusion.rs`）。基座 = Hold26 任意卖点词汇；两轴独立可消融
    /// （VLg 负交互先例 ⇒ 交互项预注册）：
    /// - `trend_hold`：49课:52 二相——"中枢向上移动时，就应该满仓，这才是
    ///   最正确的仓位"。层 k 趋势相（尾 move kind==Trend ∧ dir==Up——17课
    ///   趋势定义 ≥2 同向中枢的引擎直读）⇒ 本层卖点停削；震荡相 ⇒ 在册削减。
    /// - `counter_sub`：层内 C 短差——53课:34"参与其中的买卖，用的都是低级别
    ///   的买卖点" + 49课:64"在中枢上方全部抛出筹码，在下方如数接回"。
    ///   震荡相中 k−1 级卖证据全抛本层 slice，k−1 买证据如数接回；
    ///   44课:44 铰链：卖出不预声明身份——本层卖点先到 = 升级为减仓（出清），
    ///   买回证据先到 = 短差（回补）。
    /// - `decoupled`：C 轴资金解耦（earmark）——短差卖出所得不入共享池，
    ///   锁定为该层回补专款（049:64"如数接回"义务语义的资金面物理化：
    ///   义务资金不可被其它层新入场挪用）。53课"该级别能容纳的资金量……
    ///   以后再说"留白区的一种资金语义，与耦合臂（在册判决基线）同为
    ///   合法读法，优劣由回测裁决（fusion 负交互机械根因 = pool 耦合，
    ///   `hold26_counterseg_fusion_results.md` §3.3/§5.2）。
    ///   decoupled ∧ ¬counter_sub 无对象（专款只属于短差义务）⇒ 入口拒绝。
    /// - `trend_opts`：T 轴严格化三选项（41课衰竭门 / 49课:54 背驰出场 /
    ///   49课:60 三买起点），见 `TrendAxisOpts`。要求 trend_hold（T 轴的
    ///   修饰子，无 T 轴即无对象）⇒ 入口拒绝。
    Fusion {
        trend_hold: bool,
        counter_sub: bool,
        decoupled: bool,
        trend_opts: TrendAxisOpts,
    },
}

impl PolarityMode {
    pub fn parse(s: &str) -> Option<Self> {
        let fusion = |trend_hold: bool, counter_sub: bool, decoupled: bool| {
            Some(PolarityMode::Fusion {
                trend_hold,
                counter_sub,
                decoupled,
                trend_opts: TrendAxisOpts::default(),
            })
        };
        match s {
            "cycle45" => Some(PolarityMode::Cycle45),
            "hold26" => Some(PolarityMode::Hold26 { sell_t1_only: false }),
            "hold26_t1" => Some(PolarityMode::Hold26 { sell_t1_only: true }),
            "fusion" => fusion(true, true, false),
            "fusion_t" => fusion(true, false, false),
            "fusion_s" => fusion(false, true, false),
            "fusion_e" => fusion(true, true, true),
            "fusion_se" => fusion(false, true, true),
            // T 轴严格化臂：fusion_t + {g,d,b} 子集（规范序 g<d<b，不重复）。
            other => {
                let rest = other.strip_prefix("fusion_t")?;
                if rest.is_empty() {
                    unreachable!("fusion_t 已由上方臂覆盖")
                }
                let mut opts = TrendAxisOpts::default();
                let mut last_rank = 0u8;
                for ch in rest.chars() {
                    let rank = match ch {
                        'g' => 1,
                        'd' => 2,
                        'b' => 3,
                        _ => return None,
                    };
                    if rank <= last_rank {
                        return None; // 乱序/重复 ⇒ 非法模式串
                    }
                    last_rank = rank;
                    match ch {
                        'g' => opts.gate41 = true,
                        'd' => opts.div_exit = true,
                        'b' => opts.b3_start = true,
                        _ => unreachable!(),
                    }
                }
                Some(PolarityMode::Fusion {
                    trend_hold: true,
                    counter_sub: false,
                    decoupled: false,
                    trend_opts: opts,
                })
            }
        }
    }
}

/// 主入口。floor_ladder ∈ [FIRST_BSP_LADDER, MAX_LADDER)。
pub fn run_positional(
    tape: &SignalTape,
    floor_ladder: usize,
    mode: PolarityMode,
) -> Result<PositionalResult, String> {
    if let PolarityMode::Fusion { trend_hold, counter_sub, decoupled, trend_opts } = mode {
        return super::positional_fusion::run_fusion(
            tape,
            floor_ladder,
            trend_hold,
            counter_sub,
            decoupled,
            trend_opts,
        );
    }
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "positional fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （BSP 承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("positional fugue 要求事件磁带（bsp_events 全空）".to_string());
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut layers: [LayerState; MAX_LADDER] = [LayerState::Flat; MAX_LADDER];
    let mut pool = INITIAL_CAPITAL;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;

        // ── 市场性质：中枢账本 + 振幅参照（与持仓状态无关，事件 bar 驱动）──
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }

        // 卖点谓词（模式词汇）：Cycle45/Hold26_t1 = type1 卖；Hold26 = 任意
        // confirmed 卖点（26课"根据不同级别的卖点把仓位减少"）。
        let sell_hit = |k: usize| match mode {
            PolarityMode::Cycle45 | PolarityMode::Hold26 { sell_t1_only: true } => {
                sig.sell1.get(k)
            }
            PolarityMode::Hold26 { sell_t1_only: false } => sig.sell_any.get(k),
            PolarityMode::Fusion { .. } => {
                unreachable!("Fusion 在入口已分派到 run_fusion")
            }
        };
        let exit_reason = match mode {
            PolarityMode::Cycle45 | PolarityMode::Hold26 { sell_t1_only: true } => "sell1",
            PolarityMode::Hold26 { sell_t1_only: false } => "sellpt",
            PolarityMode::Fusion { .. } => {
                unreachable!("Fusion 在入口已分派到 run_fusion")
            }
        };

        // ── 阶段 1：出场（全层先于入场——卖点释放的资金当 bar 可供买点）──
        for k in floor_ladder..MAX_LADDER {
            if let LayerState::Long {
                entry_bar,
                entry_price,
                shares,
                weight,
                deferred_bars,
                partial,
            } = layers[k]
            {
                res.held_bars_by_ladder[k] += 1;
                if sell_hit(k) {
                    pool += shares * c;
                    res.trades.push(LayerTrade {
                        ladder: k as u8,
                        entry_bar,
                        entry_price,
                        exit_bar: i as i64,
                        exit_price: c,
                        shares,
                        weight_at_entry: weight,
                        deferred_bars,
                        partial,
                        exit_reason,
                    });
                    res.n_exits_by_ladder[k] += 1;
                    layers[k] = LayerState::Flat;
                }
            }
        }

        // ── 阶段 2：布防/确认/入场。NAV 快照在出场后取一次（bar 内交易是
        //    现金↔股数等价转换，NAV 不变 ⇒ 快照严格）。ladder 降序——大级别
        //    优先拿配额（26课大级别大资金；同 bar 竞争 pool 的确定性序，D7）──
        let bar_nav = nav_of(&layers, pool, c, floor_ladder);
        let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
        for k in (floor_ladder..MAX_LADDER).rev() {
            match (mode, layers[k]) {
                // ── Cycle45（v1，否证形态保留）──
                (PolarityMode::Cycle45, LayerState::Flat) => {
                    if sig.buy1.get(k) {
                        layers[k] = LayerState::Armed { arm_bar: i as i64 };
                    }
                }
                (PolarityMode::Cycle45, LayerState::Armed { arm_bar }) => {
                    // 区间套确认：次级别任意买点（[0, k) 任一层）；超时 fallback
                    // 入场（在册 master ARMED 逐字同构，per-layer 化）。
                    let sub_mask = (1u16 << k) - 1;
                    let confirmed = sig.buy_any.0 & sub_mask != 0
                        || (i as i64 - arm_bar) > SUB_EXPIRY;
                    if confirmed {
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    } else if sig.sell1.get(k) {
                        res.n_disarms_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    }
                }
                // ── Hold26（v2，26课恒仓）：confirmed 买点@k 直接回复 slice
                //    （四案收敛：confirmed 已含次级别完成判定，再确认=纯延迟；
                //    26课"任何的买点都是买点"）──
                (PolarityMode::Hold26 { .. }, LayerState::Flat) => {
                    if sig.buy_any.get(k) {
                        layers[k] = enter_or_defer(
                            k, i as i64, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                (PolarityMode::Hold26 { .. }, LayerState::Armed { .. }) => {
                    unreachable!("Hold26 无 ARMED 相位——confirmed 事件直接消费")
                }
                (PolarityMode::Fusion { .. }, _) => {
                    unreachable!("Fusion 在入口已分派到 run_fusion")
                }
                // Pending（两模式共用）：卖点@k 取消（该买点起始的走势已被
                // 宣告结束）；否则重试入场。
                (_, LayerState::Pending { confirm_bar }) => {
                    if sell_hit(k) {
                        res.n_pending_cancels_by_ladder[k] += 1;
                        layers[k] = LayerState::Flat;
                    } else {
                        layers[k] = enter_or_defer(
                            k, confirm_bar, i as i64, c, bar_nav, &thetas, theta_total,
                            &mut pool, &mut res,
                        );
                    }
                }
                (_, LayerState::Long { .. }) => {}
            }
        }

        // ── NAV 采样（入场后口径——bar 内 NAV 不变，与快照等值）──
        if i as i64 % EQUITY_SAMPLE_BARS == 0 || i == n - 1 {
            res.equity.push((i as i64, bar_nav));
        }
    }

    // ── eod：全层强平（与在册 eod_close 同语义）──
    let last_close = tape.bars[n - 1].close;
    for k in floor_ladder..MAX_LADDER {
        if let LayerState::Long {
            entry_bar,
            entry_price,
            shares,
            weight,
            deferred_bars,
            partial,
        } = layers[k]
        {
            pool += shares * last_close;
            res.trades.push(LayerTrade {
                ladder: k as u8,
                entry_bar,
                entry_price,
                exit_bar: n as i64 - 1,
                exit_price: last_close,
                shares,
                weight_at_entry: weight,
                deferred_bars,
                partial,
                exit_reason: "eod",
            });
            res.n_exits_by_ladder[k] += 1;
            layers[k] = LayerState::Flat;
        }
    }
    res.final_nav = pool;
    Ok(res)
}

/// 入场尝试（ARMED 确认 bar 或 PENDING 重试 bar），返回新层状态。
/// θ_k 无参照 ⇒ 跳过（配额未定义，回 FLAT 并计数——不静默给默认值，D2）；
/// pool 可成交 < MIN_FILL_FRAC×目标 ⇒ Pending 推迟（D7/D9）；否则成交
/// min(w_k×NAV, pool)，部分成交计数。
#[allow(clippy::too_many_arguments)]
pub(crate) fn enter_or_defer(
    k: usize,
    confirm_bar: i64,
    bar: i64,
    c: f64,
    bar_nav: f64,
    thetas: &[Option<f64>; MAX_LADDER],
    theta_total: f64,
    pool: &mut f64,
    res: &mut PositionalResult,
) -> LayerState {
    let Some(theta_k) = thetas[k] else {
        res.n_noref_skips_by_ladder[k] += 1;
        return LayerState::Flat;
    };
    debug_assert!(theta_total > 0.0, "θ_k 有定义 ⇒ total > 0");
    let w = theta_k / theta_total;
    let want = w * bar_nav;
    let cash = want.min(*pool);
    if cash < want * MIN_FILL_FRAC || !(c > 0.0) {
        if bar == confirm_bar {
            res.n_deferred_by_ladder[k] += 1;
        }
        return LayerState::Pending { confirm_bar };
    }
    *pool -= cash;
    let partial = cash < want * (1.0 - 1e-12);
    if partial {
        res.n_partial_by_ladder[k] += 1;
    }
    res.n_entries_by_ladder[k] += 1;
    LayerState::Long {
        entry_bar: bar,
        entry_price: c,
        shares: cash / c,
        weight: w,
        deferred_bars: bar - confirm_bar,
        partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::{BspClass, BspEvent, LadderMask};

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    /// 带中枢锚的 candidate 事件——喂 CenterBook/DepthRef 参照集（不触发交易）。
    fn anchor_ev(cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: false,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    fn with_anchor(mut b: BarSig, lad: usize, cs: i64, zd: f64, zg: f64) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(anchor_ev(cs, zd, zg));
        b
    }

    fn buy1(mut b: BarSig, lad: usize) -> BarSig {
        b.buy1 = LadderMask(b.buy1.0 | (1 << lad));
        b
    }

    fn sub_confirm(mut b: BarSig, lad: usize) -> BarSig {
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sell1(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b
    }

    /// 头部喂 ladder 2/4 各 SUB_COST_MIN_OBS 个参照中枢（θ₂=1%、θ₄=3%，
    /// c=100 口径）——之后 w₂=0.25、w₄=0.75。
    fn warmup_bars() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let b = with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0);
            bars.push(with_anchor(b, 4, 100 + j, 50.0, 53.0));
        }
        bars
    }

    #[test]
    fn weights_emerge_from_amplitude_share() {
        let mut bars = warmup_bars();
        // buy1@2 → 次级别确认（bi=1）→ 入场 w=0.25
        bars.push(buy1(bar(100.0), 2));
        bars.push(sub_confirm(bar(100.0), 1));
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 1);
        let tr = &r.trades[0];
        assert!((tr.weight_at_entry - 0.25).abs() < 1e-12, "w₂ = 1%/(1%+3%) = 0.25");
        assert!((tr.shares - 0.25 * INITIAL_CAPITAL / 100.0).abs() < 1e-9);
        assert_eq!(tr.exit_reason, "eod");
    }

    #[test]
    fn layers_exit_independently_lesson44() {
        // 双层持股，sell1@2 只清层 2，层 4 持有到 eod——44课级别错配禁令。
        let mut bars = warmup_bars();
        bars.push(buy1(buy1(bar(100.0), 2), 4));
        bars.push(sub_confirm(bar(100.0), 1)); // 同时确认两层（bi 是双方次级别）
        bars.push(sell1(bar(110.0), 2));
        bars.push(bar(120.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 1);
        assert_eq!(r.n_entries_by_ladder[4], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t2[0].exit_reason, "sell1");
        assert_eq!(t2[0].exit_price, 110.0);
        assert_eq!(t4[0].exit_reason, "eod", "段级 sell1 无权清 recL2 配额");
        assert_eq!(t4[0].exit_price, 120.0);
        // 股数守恒：层 4 进出同股数
        assert!((t4[0].shares - 0.75 * INITIAL_CAPITAL / 100.0).abs() < 1e-9);
    }

    #[test]
    fn noref_skip_when_theta_undefined() {
        // 无参照集（零 warm-up）：buy1@2 确认后配额未定义 → 跳过并计数。
        let bars = vec![
            with_anchor(bar(100.0), 2, 1, 50.0, 51.0), // 1 个参照 < min_obs
            buy1(bar(100.0), 2),
            sub_confirm(bar(100.0), 1),
            bar(100.0),
        ];
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 0);
        assert_eq!(r.n_noref_skips_by_ladder[2], 1);
        assert_eq!(r.final_nav, INITIAL_CAPITAL);
    }

    #[test]
    fn pool_starvation_defers_then_fills_after_exit() {
        // 层 2 先满配额入场（仅层 2 有 θ ⇒ w₂=1.0 全仓），层 4 warm-up 后
        // buy1@4 确认 → pool 空 → Pending；层 2 sell1 释放资金 → 层 4 入场。
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0));
        }
        bars.push(buy1(bar(100.0), 2));
        bars.push(sub_confirm(bar(100.0), 1)); // 层 2 全仓入场
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 4, 100 + j, 50.0, 53.0));
        }
        bars.push(buy1(bar(100.0), 4));
        bars.push(sub_confirm(bar(100.0), 1)); // 层 4 确认但 pool=0 → Pending
        bars.push(sell1(bar(100.0), 2)); // 层 2 出场释放 → 层 4 同 bar 入场
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 1);
        assert_eq!(r.n_deferred_by_ladder[4], 1, "确认 bar 资金不足 → 推迟");
        assert_eq!(r.n_entries_by_ladder[4], 1, "层 2 出场释放后入场");
        let t4 = r.trades.iter().find(|t| t.ladder == 4).unwrap();
        assert!(t4.deferred_bars > 0);
        // 层 4 配额 w=0.75 但 pool 只有层 2 回笼的全部 → min(0.75×NAV, NAV) = 0.75×NAV
        assert!((t4.weight_at_entry - 0.75).abs() < 1e-12);
    }

    #[test]
    fn pending_cancelled_by_sell1() {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0));
        }
        bars.push(buy1(bar(100.0), 2));
        bars.push(sub_confirm(bar(100.0), 1)); // 层 2 全仓
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 4, 100 + j, 50.0, 53.0));
        }
        bars.push(buy1(bar(100.0), 4));
        bars.push(sub_confirm(bar(100.0), 1)); // 层 4 Pending（pool 空）
        bars.push(sell1(bar(100.0), 4)); // 层 4 的卖点先到 → 取消
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_pending_cancels_by_ladder[4], 1);
        assert_eq!(r.n_entries_by_ladder[4], 0);
    }

    #[test]
    fn armed_disarms_on_sell1() {
        let mut bars = warmup_bars();
        bars.push(buy1(bar(100.0), 2));
        bars.push(sell1(bar(100.0), 2)); // 确认未到，sell1 撤防
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_disarms_by_ladder[2], 1);
        assert_eq!(r.n_entries_by_ladder[2], 0);
    }

    #[test]
    fn expiry_fallback_entry() {
        // 次级别确认不来 → SUB_EXPIRY 超时入场（在册 master 同构）。
        let mut bars = warmup_bars();
        bars.push(buy1(bar(100.0), 2));
        for _ in 0..(SUB_EXPIRY + 2) {
            bars.push(bar(100.0));
        }
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 1);
    }

    #[test]
    fn nav_conservation_round_trip() {
        // 入场→出场价格不变 ⇒ NAV 守恒（零摩擦口径的会计自检）。
        let mut bars = warmup_bars();
        bars.push(buy1(bar(100.0), 2));
        bars.push(sub_confirm(bar(100.0), 1));
        bars.push(sell1(bar(100.0), 2));
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r = run_positional(&t, 2, PolarityMode::Cycle45).unwrap();
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9);
    }

    fn sellpt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    #[test]
    fn hold26_slices_oscillate_around_full() {
        // 26课恒仓：买点直接回复 slice（无 ARMED），卖点只削减本层。
        let mut bars = warmup_bars();
        bars.push(sub_confirm(sub_confirm(bar(100.0), 2), 4)); // 买点@2 @4
        bars.push(sellpt(bar(110.0), 2)); // 卖点@2 → 削层2，层4 持有
        bars.push(sub_confirm(bar(105.0), 2)); // 买点@2 → 回复
        bars.push(bar(120.0));
        let t = SignalTape { bars, ..Default::default() };
        let r =
            run_positional(&t, 2, PolarityMode::Hold26 { sell_t1_only: false }).unwrap();
        assert_eq!(r.n_entries_by_ladder[2], 2, "削减后买点回复");
        assert_eq!(r.n_entries_by_ladder[4], 1);
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2[0].exit_reason, "sellpt");
        assert_eq!(t2[0].exit_price, 110.0);
        assert_eq!(t2[1].exit_reason, "eod");
        let t4: Vec<_> = r.trades.iter().filter(|t| t.ladder == 4).collect();
        assert_eq!(t4.len(), 1, "层4 不被层2 卖点触及");
        assert_eq!(t4[0].exit_reason, "eod");
        // 周期股数守恒：层2 第一周期进出同股数（@100 买 @110 卖全清）
        assert!((t2[0].shares - 0.25 * INITIAL_CAPITAL / 100.0).abs() < 1e-9);
    }

    #[test]
    fn hold26_t1_vocabulary_ignores_non_t1_sells() {
        let mut bars = warmup_bars();
        bars.push(sub_confirm(bar(100.0), 2));
        bars.push(sellpt(bar(110.0), 2)); // 任意卖点置位但非 type1
        bars.push(bar(120.0));
        let t = SignalTape { bars, ..Default::default() };
        let r =
            run_positional(&t, 2, PolarityMode::Hold26 { sell_t1_only: true }).unwrap();
        let t2: Vec<_> = r.trades.iter().filter(|t| t.ladder == 2).collect();
        assert_eq!(t2.len(), 1);
        assert_eq!(t2[0].exit_reason, "eod", "hold26_t1 只认 type1 卖点");
    }

    #[test]
    fn hold26_pending_when_pool_dry_cancelled_by_sellpt() {
        // 层2 全仓占满（仅层2 有 θ）→ 层4 warm-up 后买点 Pending →
        // 层4 卖点取消（买点起始走势已结束）。
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 2, 10 + j, 50.0, 51.0));
        }
        bars.push(sub_confirm(bar(100.0), 2)); // 层2 w=1.0 全仓
        for j in 0..SUB_COST_MIN_OBS as i64 {
            bars.push(with_anchor(bar(100.0), 4, 100 + j, 50.0, 53.0));
        }
        bars.push(sub_confirm(bar(100.0), 4)); // 层4 买点，pool 空 → Pending
        bars.push(sellpt(bar(100.0), 4)); // 层4 卖点 → 取消
        bars.push(bar(100.0));
        let t = SignalTape { bars, ..Default::default() };
        let r =
            run_positional(&t, 2, PolarityMode::Hold26 { sell_t1_only: false }).unwrap();
        assert_eq!(r.n_deferred_by_ladder[4], 1);
        assert_eq!(r.n_pending_cancels_by_ladder[4], 1);
        assert_eq!(r.n_entries_by_ladder[4], 0);
    }

    #[test]
    fn floor_guard_rejects_bar_level() {
        let t = SignalTape { bars: vec![bar(100.0)], ..Default::default() };
        assert!(run_positional(&t, 0, PolarityMode::Cycle45).is_err());
        assert!(run_positional(&t, 2, PolarityMode::Cycle45).is_err(), "事件磁带全空也拒绝");
    }
}
