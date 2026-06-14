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
use super::types::{Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER, SUB_EXPIRY};

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
    /// 持空 [镜像推导]（双向条件轴 S1-S4，fusion_btr 白名单层专属；
    /// `analysis/bidirectional_nested_accounting.md` §5 翻转断面）。
    /// 1x 虚拟逐仓：margin = units × entry_price 在开空 bar 从 pool 锁定
    /// （= 同 bar 平多所得，M = N 同股数定理 ⇒ 翻转 bar pool 净流转为 0）。
    /// 层权益 = margin + units × (entry_price − c)；权益 ≤ 0 即逐仓强平
    /// （1x 解析强平价 = 2 × entry_price）。非白名单模式不可达（构造点
    /// 全部以 short_mask 守卫——在册路径零接触）。
    Short {
        entry_bar: i64,
        entry_price: f64,
        units: f64,
        weight: f64,
        margin: f64,
    },
    /// 纯回复门驻留（fusion_btrg 消融臂——S1-S4 判决开放轴④的载体）。
    /// 持币（平多所得已入 pool、零市场暴露），但回复词汇受与 Short 完全
    /// 同款的门（MoveUp 强制回复 / 买点 + MoveDown 停回复 + 空侧 R2 门）。
    /// 存在论：把双向臂的"空头暴露"分量摘除、保留"回复时点门"分量——
    /// btrg ≈ btr ⇒ 双向增量主体是回复门；btr ≫ btrg ⇒ 空头暴露独占
    /// 增量 = bear 段下跌直接收割。仅 short_ghost 模式可达。
    Gated { entry_bar: i64, weight: f64 },
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
    /// 持仓极性（双向会计 v2 §4.1 ④(iv)：trade 行方向字段）。在册全部
    /// 路径恒 Long；Short 行的现金流语义镜像（开空收 proceeds/平空付
    /// 买回款），NAV 重建方按此字段分派符号。
    pub polarity: Polarity,
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
    // ── 统一配置 U（fusion_u：相位递归路由 osc 层）观测面；其余模式恒零。
    //    全部 n_route_* 计数共享调用方预滤条件"本 bar sell_any≠0"
    //    （unified_osc::route docstring）──
    /// osc 开腿数（按宿主层 k）。
    pub n_osc_opens_by_ladder: [u64; MAX_LADDER],
    /// 上移开腿数（路由层 j > 宿主层 k）。
    pub n_osc_upshift_opens_by_ladder: [u64; MAX_LADDER],
    /// osc 开腿数（按路由层 j——P3"各级别路由计数 n_route(j)"）。
    pub n_osc_open_at_level: [u64; MAX_LADDER],
    /// 在外 bar 数（按宿主层；P3 槽占用率分子之一）。
    pub n_osc_out_bars_by_ladder: [u64; MAX_LADDER],
    /// 上移腿在外 bar 数（P3"上移腿独立槽占用率"）。
    pub n_osc_up_out_bars_by_ladder: [u64; MAX_LADDER],
    /// ① 相位门跳过数（按被跳过级别 j——趋势相上移的逐级读数）。
    pub n_route_phase_skips: [u64; MAX_LADDER],
    /// 对象不存在（j 层无存活中枢——非门拒，基底缺位）。
    pub n_route_no_center: [u64; MAX_LADDER],
    /// ② 振幅门拒（θ_q(j) < k×friction）。
    pub n_route_amp_rejects: [u64; MAX_LADDER],
    /// ② 参照不可定义拒（warm-up 保守拒绝，不静默放行）。
    pub n_route_amp_noref: [u64; MAX_LADDER],
    /// ③ 强震荡门拒（当前震荡脱离前上涨最后中枢区间 = 弱震荡，093:26）。
    pub n_route_weak_rejects: [u64; MAX_LADDER],
    /// ③ 参照不可定义拒（该层从未有 dir==Up 的存活中枢）。
    pub n_route_weak_noref: [u64; MAX_LADDER],
    /// H1 candidate 冻结拒（h1_freeze；锚层未决 candidate type3 离开段 ⇒
    /// 走势方向未定不开——osc_candidate_freeze 在册判据下沉，按级别 j）。
    pub n_route_h1_freezes: [u64; MAX_LADDER],
    /// Sequence38 子腿（seq38_sub；38课:36 程式）观测面：开腿数。
    pub n_seq38_opens_by_ladder: [u64; MAX_LADDER],
    /// Seq38 闭腿三岔归因：盘背买 / 不破第一段低点×次级别确认 / 新下跌背驰。
    pub n_seq38_consbuy_closes_by_ladder: [u64; MAX_LADDER],
    pub n_seq38_nobreak_closes_by_ladder: [u64; MAX_LADDER],
    pub n_seq38_newdiv_closes_by_ladder: [u64; MAX_LADDER],
    /// 路由选中数（按级别 j；选中 ≠ 开腿——触发判据另查）。
    pub n_route_selected_by_level: [u64; MAX_LADDER],
    /// 全塔拒绝数（无满足级别 ⇒ 恒仓吃趋势，053:34）。
    pub n_route_exhausted: u64,
    /// 回补归因（一 bar 一动作，优先序见 unified_osc::step_exit）。
    pub n_osc_zd_restores_by_ladder: [u64; MAX_LADDER],
    pub n_osc_death_restores_by_ladder: [u64; MAX_LADDER],
    pub n_osc_shift_restores_by_ladder: [u64; MAX_LADDER],
    pub n_osc_kbuy_restores_by_ladder: [u64; MAX_LADDER],
    pub n_osc_phase_restores_by_ladder: [u64; MAX_LADDER],
    pub n_osc_due_restores_by_ladder: [u64; MAX_LADDER],
    /// 44课铰链升级出清数（本层卖点先到，身份事后授予为减仓；前提
    /// ¬in_trend(k)——counter_sub 在册铰链可达性同构）。
    pub n_osc_escalates_by_ladder: [u64; MAX_LADDER],
    /// k 趋势相内 k 卖点对在外腿的停削抑制数（049:52——升级旁路被堵的
    /// 可观测面）。
    pub n_osc_trend_hold_sells_by_ladder: [u64; MAX_LADDER],
    /// 三卖否决 latch 置位数（049:52"不能回补"）。
    pub n_osc_sell3_vetos_by_ladder: [u64; MAX_LADDER],
    /// osc 回补义务因资金不足推迟的 bar 数。
    pub n_osc_restore_defer_bars: u64,
    /// osc 短差净现金（按宿主层 k）。
    pub osc_net_cash_by_ladder: [f64; MAX_LADDER],
    /// osc 短差净现金（按路由层 j——P4"上移腿净亏"直读）。
    pub osc_net_cash_at_level: [f64; MAX_LADDER],
    // ── P6 相位机（fusion_p/fusion_pu）观测面；其余模式恒零 ──
    /// →MOVE↑ 锁定转移数（confirmed Buy3 锚定，049:60/62）。
    pub n_phase_up_opens_by_ladder: [u64; MAX_LADDER],
    /// MOVE↑→OSC：新中枢结算关（049:54"新中枢的形成"/049:60"直到新中枢出现"）。
    pub n_phase_up_settle_closes_by_ladder: [u64; MAX_LADDER],
    /// MOVE↑→OSC：向上背驰/盘背事件关（049:54/42——EXIT 全抛强读法一期不
    /// 启用，相位翻 OSC 后卖点词汇近似承载出场）。
    pub n_phase_up_div_closes_by_ladder: [u64; MAX_LADDER],
    /// MOVE↑→OSC：dir 翻 Down 结构兜底关（b3_start 在册同构）。
    pub n_phase_up_dir_closes_by_ladder: [u64; MAX_LADDER],
    /// →MOVE↓ 锁定转移数（confirmed Sell3，049:52"不能回补"+049:40）。
    pub n_phase_dn_opens_by_ladder: [u64; MAX_LADDER],
    /// MOVE↓→OSC 关（settle/向下背驰/dir 翻 Up 合计——038:36 镜像）。
    pub n_phase_dn_closes_by_ladder: [u64; MAX_LADDER],
    /// MOVE↑ 有效驻留 bar 数（含 candidate 离开窗口——停削窗口大小，
    /// 与 kind×dir 窗口直接可比的 P6 核心机制读数）。
    pub phase_up_bars_by_ladder: [u64; MAX_LADDER],
    /// MOVE↓ 有效驻留 bar 数（含 candidate 窗口）。
    pub phase_dn_bars_by_ladder: [u64; MAX_LADDER],
    /// P7 R2 位置门拦截数（震荡相非高位卖点不削减，049:52 位置分量；
    /// fusion_tr/fusion_pr/fusion_pur，其余模式恒零）。
    pub n_r2_pos_blocks_by_ladder: [u64; MAX_LADDER],
    /// anc 豁免拦截数（hold26_anc/fusion_ta；slow_bull P7）：停削拦截中
    /// **自层 KindDir 判据为假**、纯祖先窗口（∃j>k Trend∧Up）触发的次数
    /// ——与调研 §1.4 E0 豁免桶（fusion_t 残余）直接可比，其余模式恒零。
    pub n_anc_exempt_blocks_by_ladder: [u64; MAX_LADDER],
    /// anc 豁免窗口驻留 bar 数（∃j>k Trend∧Up 成立的 bar，逐层；与持仓
    /// 无关的市场性质读数——P7 occupancy 对齐，其余模式恒零）。
    pub anc_up_bars_by_ladder: [u64; MAX_LADDER],
    // ── 双向条件轴 S1-S4（fusion_btr/fusion_btra）观测面 [镜像推导]；
    //    其余模式恒零（short_mask=0 全部路径不可达）──
    /// 卖点削减→翻空开仓数（白名单层，翻转断面 §5.2 第③步）。
    pub n_flip_shorts_by_ladder: [u64; MAX_LADDER],
    /// 买点平空→翻多数（翻转断面对偶面，exit_reason="cover_buypt"）。
    pub n_short_covers_by_ladder: [u64; MAX_LADDER],
    /// MoveUp 相强制平空数（49:52 满仓义务镜像——空头前提消失，
    /// exit_reason="cover_moveup"）。
    pub n_short_moveup_covers_by_ladder: [u64; MAX_LADDER],
    /// MoveDown 相停回补拦截数（49:52 镜像"中枢向下移动应满空仓"——
    /// P6 MoveDown 相位获得空头侧消费者，会计文档 §2.5）。
    pub n_short_trend_holds_by_ladder: [u64; MAX_LADDER],
    /// 空侧 R2 回补位置门拦截数（c > ZD 时买点不回补——049:64"在下方
    /// 如数接回"镜像 = P7 回复侧位置门获得对象，会计文档 §8.1 (d)）。
    pub n_short_r2_blocks_by_ladder: [u64; MAX_LADDER],
    /// 镜像 anc 门拒开空数（fusion_btra：∄j>k Trend∧Down ⇒ 削减照常
    /// 但不开空，层留 Flat）。
    pub n_short_anc_rejects_by_ladder: [u64; MAX_LADDER],
    /// 虚拟逐仓强平数（层权益击穿 0；强平价 = 2×entry_price 记账）。
    pub n_short_liquidations_by_ladder: [u64; MAX_LADDER],
    /// 持空 bar 数。
    pub short_held_bars_by_ladder: [u64; MAX_LADDER],
    /// 空头腿已实现净现金（Σ units×(B_s − exit)；空头 alpha 的会计读数）。
    pub short_net_cash_by_ladder: [f64; MAX_LADDER],
    /// 纯回复门消融臂（fusion_btrg）：进 Gated 驻留数（其余模式恒零）。
    pub n_gate_enters_by_ladder: [u64; MAX_LADDER],
    /// Gated → 买点回复数（门放行后 enter_or_defer）。
    pub n_gate_restores_by_ladder: [u64; MAX_LADDER],
    /// Gated → MoveUp 强制回复数（49:52 满仓义务——与 Short 同款出口）。
    pub n_gate_moveup_restores_by_ladder: [u64; MAX_LADDER],
    /// Gated 驻留 bar 数（与 short_held_bars 对照 = 消融可比性读数）。
    pub gate_held_bars_by_ladder: [u64; MAX_LADDER],
    // ── 区间套正向定位（fusion_tn/fusion_trn；027课程序定理 + 038:258
    //    "一旦进入背驰的区间套里就要陆续走"）观测面；其余模式恒零 ──
    /// 窗口武装数（candidate Type1/Type3 事件，逐层逐侧合计）。
    pub n_nest_arms_by_ladder: [u64; MAX_LADDER],
    /// 卖侧正向触发数（candidate@k × 次级别 k−1 第一个卖侧证据）。
    pub n_nest_fire_sell_by_ladder: [u64; MAX_LADDER],
    /// 买侧正向触发数（镜像）。
    pub n_nest_fire_buy_by_ladder: [u64; MAX_LADDER],
    /// 背驰段打破否定数（027课"只要没有打破背驰段"——价格越过 candidate
    /// 极值 ⇒ 窗口作废，正向定位不触发）。
    pub n_nest_breaks_by_ladder: [u64; MAX_LADDER],
    /// 正向触发领先 confirmed 的 bar 数合计（同 (class,cs) 的 confirmed
    /// 事件事后到达时配对回填；confirmed 永不到达的触发不计入）。
    pub nest_lead_bars_sum: u64,
    /// 领先样本数（nest_lead_bars_sum 的分母）。
    pub nest_lead_n: u64,
    // ── 统一递归 voice FSM（fusion_v）观测面（其余模式恒零）──
    /// Φ(k)=MoveUp 有效驻留 bar 数（freeze_up ∧ ¬R4 出口；停削窗口本体）。
    pub freeze_up_bars_by_ladder: [u64; MAX_LADDER],
    /// Φ(k)=MoveDown 有效驻留 bar 数（freeze_dn ∧ ¬R4 镜像出口；停回补窗口）。
    pub freeze_dn_bars_by_ladder: [u64; MAX_LADDER],
    /// T2W 锁存武装次数（R18：confirmed type1 动作被门拒 ⇒ 第二翻转窗口
    /// 武装；053:28 二分定理）。双侧合计。
    pub n_t2w_arms_by_ladder: [u64; MAX_LADDER],
    /// T2W 触发次数（R19：武装后 confirmed type2 同侧到达 ∧ 门放行）。
    /// 侧别归因见 trade 行 exit_reason ∈ {"t2w_sell"} / 回复计数。
    pub n_t2w_fires_by_ladder: [u64; MAX_LADDER],
    /// T2W 否定次数（R20：close 越过锁存 extreme ⇒ 清锁存；086:80 镜像）。
    pub n_t2w_negates_by_ladder: [u64; MAX_LADDER],
    // ── 双书独立逐仓 voice（fusion_vd/vdn）观测面（其余模式恒零）──
    /// 空头书独立开仓数（θ 配额逐仓——非翻转断面，与 n_flip_shorts 互斥）。
    pub n_dual_short_opens_by_ladder: [u64; MAX_LADDER],
    /// 空头书开仓门拦截数（MoveUp 满仓义务窗口 ∨ r2 位置门）。
    pub n_dual_short_open_blocks_by_ladder: [u64; MAX_LADDER],
    /// 同级别双书共存 bar 数（多头书 Long ∧ 空头书 Short——非净额合并的
    /// 核心新现象，m>N 承载位可观测面）。
    pub dual_both_held_bars_by_ladder: [u64; MAX_LADDER],
    // ── 嵌套递归赋格（nrf；`nested_fugue.rs`）观测面；其余模式恒零 ──
    /// 根 voice 入场数（按入场层——最高 θ 涌现层分布即根的爬升轨迹）。
    pub n_nrf_root_entries_by_ladder: [u64; MAX_LADDER],
    /// 子 voice spawn 数（按子层；区间套递归 = voice 诞生的直接计数）。
    pub n_nrf_spawns_by_ladder: [u64; MAX_LADDER],
    /// 027:25 否定平仓数（价格越过 spawn 时 candidate 极值，按层）。
    pub n_nrf_negate_closes_by_ladder: [u64; MAX_LADDER],
    /// 级联回收数（父腿平仓 ⇒ 子树前提消失，按被回收子层）。
    pub n_nrf_cascade_closes_by_ladder: [u64; MAX_LADDER],
    /// 35课成本门拒 spawn 数（θ_q(sub) < k×friction——经济终止，按子层）。
    pub n_nrf_cost_rejects_by_ladder: [u64; MAX_LADDER],
    /// θ 参照不可定义拒数（warm-up 保守拒绝，按子层）。
    pub n_nrf_noref_rejects_by_ladder: [u64; MAX_LADDER],
    /// floor 触底数（k−1 < floor——存在论终止（笔=a0），按父层）。
    pub n_nrf_floor_stops_by_ladder: [u64; MAX_LADDER],
    /// 尾 voice 翻转数（confirmed 走势完美 ⇒ 平旧开新；同一笔物理交易在
    /// 父层 = 短差腿闭/开——双层记账的核心机制读数，按层）。
    pub n_nrf_flips_by_ladder: [u64; MAX_LADDER],
    /// 根翻转数（v3 第23环：根走势完美 ⇒ 全链清算 + 立即按新势方向满仓
    /// 重建根——出场=翻转=新建仓；按重建层计数）。
    pub n_nrf_root_flips_by_ladder: [u64; MAX_LADDER],
    /// 递归区间套深触发数（v3 第14环：触发证据来自 k−2 或更低层——经
    /// 同侧 candidate 链下探所得；j=k−1 直接证据不计入。按窗口层 k）。
    pub n_nrf_deep_fires_by_ladder: [u64; MAX_LADDER],
    /// 链深度直方图：活跃链长 = d 的 bar 计数（stretto 并发读数，d 为索引）。
    pub nrf_depth_bars: [u64; MAX_LADDER],
    /// 物理持股 bar 数（链上多头在手单位 > 0；v4 不清仓原则下 ≈ 链活跃
    /// bar 数——根始终持有 N−m）。
    pub nrf_phys_long_bars: u64,
    /// 空头视图在手 bar 数（链上空头 voice 在手单位 > 0 的 bar）。
    pub nrf_phys_short_bars: u64,
    /// earning 增仓事件数（按增仓 voice 层；§7 cost_pool ≤ 0 后纯利润
    /// 在买点买入 Δ，N 重定基 N′ = N + Δ）。
    pub n_nrf_earning_adds_by_ladder: [u64; MAX_LADDER],
    /// earning 累计增仓单位数。
    pub nrf_earning_units: f64,
    /// 空头 earning 命中数（§7 声明对称，但挣负股数 L0 构造性不可表示
    /// ——在册结算优先；命中时现金沉淀 capital 不增仓，本计数器观测）。
    pub nrf_short_earning_hits: u64,
    /// 亏损回补缩水累计单位数（资金守恒：买不回的单位 = 亏损的物理
    /// 形式，N 重定基 N′ = N − δ——earning 重定基的镜像）。
    pub nrf_shrink_units: f64,
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

/// 统一配置 U 的 osc 层开关（相位递归路由，2026-06-12 任务；
/// `unified_osc.rs` 模块 docstring 为完整原文锚定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OscRouting {
    /// 无 osc 层（在册 fusion 家族全部形态——零接触退化面）。
    #[default]
    Off,
    /// 三门合取递归路由：①¬in_trend(j)（049:52）× ②θ_q(j) 过振幅门
    /// （035:30，H4 逐字）× ③强震荡（093:26）。strong_gate=false =
    /// U−③ 消融臂（预注册判据 P5：③是本架构唯一新词汇，必须单独消融）。
    /// h1_freeze = H1 candidate 冻结门下沉（osc_candidate_freeze 在册判据
    /// 逐字：锚层存在未决 candidate type3 离开段 ⇒ 走势方向未定 ⇒ 不开
    /// osc 腿；全量普适组合 2026-06-12 任务预注册，仅 fusion_btra*f 臂）。
    Unified { strong_gate: bool, h1_freeze: bool },
}

/// 停削时钟的层级作用域——anc 祖先趋势豁免（26:80 下沉，
/// `analysis/slow_bull_vs_bh_research.md` §4/§6 预注册）。
///
/// 026:80："如果有更大级别的单边上扬，短线的可以不必坚持小转大的原则"
/// ——豁免判据是**祖先层**的走势结构读数：∃ j > k: kind(j)==Trend ∧
/// dir(j)==Up（17课趋势定义 ≥2 同向中枢 = 中枢序列单调上移的引擎直读）。
/// 层级局部、无年度 regime 标签（GDX 否证合规）、无前视、无 per-asset 参数、
/// 零新磁带依赖（trend_flips/dir_flips 行已在）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrendScope {
    /// 在册：层 k 自身 kind==Trend ∧ dir==Up（049:52 自层窗口，fusion_t）。
    #[default]
    SelfLayer,
    /// hold26_anc（M1 主臂）：纯祖先窗口 ∃j>k Trend∧Up **替换**自层判据
    /// （26:80 下沉——慢牛标的 recL2 失血的对象域恰是"自层震荡但祖先
    /// 趋势"，调研 §1.4 E0 桶 ES −0.517 nats）。
    Ancestor,
    /// fusion_ta（M3 对照臂）：自层 ∨ 祖先（049:52 ∪ 026:80 全字面并集）。
    SelfOrAncestor,
}

/// NRF 清仓判据（C 规则）的参照系——539号开放轴的两个形态。
///
/// v4（在册基座，bit-exact）与构成性贯通（A′ 合取选层 + regime 门）是同一
/// 嵌套递归会计的两种**清仓选层 + 门控**，其余 §1-§8 会计规则逐字相同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearanceMode {
    /// v4：`sell1[top] ∧ located_sell[top]`，`top = θ 棘轮累积层`
    /// （`depth_ref.theta(k).is_some()` 的最高层）。诊断
    /// （`cl_clearance_diagnosis.md §4`）：棘轮单调攀高 + 同层同 bar 合取
    /// 把清仓频率钉死（CL=OKLO=3，与波动率脱钩）。在册 P1 3/8 最优基座。
    V4,
    /// 构成性贯通（539号 A′ + R6′，本任务实装）：**A′ 合取选层**
    /// `top* = max{k≥floor : sell1[k] ∧ located_sell[k]}`，存在即候选清仓
    /// ——层由**两半共现**决定（解耦 v4 θ 棘轮、区别 v5 的 located 单独
    /// 选层），located 仍是独立合取项（R5′ 可质询）。`regime` 选 R6′ 门形态。
    ConstitutiveThroughput { regime: RegimeGate },
}

/// R6′ regime 门形态（539号 §3.2；A′ 选出 top* 后是否清仓的判据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegimeGate {
    /// 走势类型门（任务 body 形式）：`trend_state[top*] == Trend`
    /// （17课走势完全分类）——清仓层走势是**趋势**且 type1 背驰才清仓；
    /// **盘整**中的 type1 是中枢震荡（回中枢，不清）。L3 否证（强牛子趋势
    /// 顶 = 趋势-type1，门放行 ⇒ 复现 v5 踏空，OKLO −635pp）。
    TopLevelTrend,
    /// anc 镜像门（539 R6′ 原形式，hold26_anc 的镜像）：清仓 ⟺
    /// `∃ j > top* : trend_state[j] ∧ dir_state[j] == Down`——仅当**更大
    /// 级别上下文证实向下**（趋势且向下）时清仓。强牛无更大级别向下 ⇒ 不
    /// 清仓（堵踏空）；深崩更大级别转下 ⇒ 清仓（保避险）。
    AncestorDown,
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
    /// - `osc`：统一配置 U 的 osc 层（相位递归路由，2026-06-12 任务；
    ///   `unified_osc.rs`）。要求 trend_hold（U 定义在 fusion_t 基座上）、
    ///   拒 counter_sub（同层 slice 双在外冲突未定义）、拒 trend_opts
    ///   （未预注册组合）⇒ 入口拒绝。Off = 在册行为零接触。
    /// - `phase_clock`：P6 相位机时钟（2026-06-12 任务；
    ///   `analysis/p6_phase_machine_research.md`）。停削窗口与 osc ①门从
    ///   kind×dir 行（049:52 的超集近似，research v2 §2.2 错位定理）同步
    ///   切换为 49课相位区间：MOVE↑ = candidate 离开窗口（049:68 当下读法，
    ///   价格回中枢否定 = "校正"）∪ confirmed Buy3 锁定区间
    ///   [Buy3(C), settle(C′)]（049:60 逐字）；MOVE↓ 镜像（038:36）。
    ///   kind 行零消费（错位时钟退役）。要求 trend_hold（时钟的对象是停削
    ///   窗口）、拒 counter_sub/trend_opts（未预注册；b3_start 被相位机
    ///   收编）⇒ 入口拒绝。false = 在册 kind×dir 时钟零接触。
    /// - `r2_gate`：P7 R2 位置门（535 号裁决实验；research v2 §2.3/§6 P7
    ///   预注册）。震荡相削减加位置分量——049:52"在中枢上方仓位减少"字面：
    ///   卖点削减要求 c ≥ ZG(k)（本层存活中枢上沿）。域 = 震荡相精确
    ///   （phase_clock ⇒ Φ(k)=OSC；KindDir ⇒ ¬in_trend ∧ dir≠Down——熊市
    ///   削减保留，位置门对象是中枢震荡非下行段，049:40 映射声明）；
    ///   无存活中枢 ⇒ 位置词汇无对象，卖点回退出场语义不拦截（049:54）。
    ///   回复侧位置门（c≤ZD 才回，049:64"在下方如数接回"）不在本轴——
    ///   P7 预注册为削减侧最小差分，回复侧列开放轴。要求 trend_hold、
    ///   拒 counter_sub/trend_opts（未预注册）⇒ 入口拒绝。
    /// - `trend_scope`：anc 祖先趋势豁免（26:80 下沉，slow_bull 调研 §4/§6
    ///   预注册）。停削时钟的层级作用域：SelfLayer = 在册自层窗口；
    ///   Ancestor = hold26_anc（∃j>k Trend∧Up 替换自层）；SelfOrAncestor =
    ///   fusion_ta（自层 ∨ 祖先全字面）。非 SelfLayer 要求 trend_hold
    ///   （时钟无对象）、拒 counter_sub/trend_opts/osc/phase_clock/r2_gate
    ///   （仅 M1/M3 两臂预注册）⇒ 入口拒绝。SelfLayer = 在册行为零接触。
    /// - `nest_forward`：区间套正向定位（027课精确大转折点寻找程序定理 +
    ///   038:258"不是等真跌了才问卖不卖，而是涨的时候一旦进入背驰的区间套
    ///   里，就要陆续走"）。candidate Type1/Type3 事件武装窗口（= 本级别
    ///   进入背驰段/回试段），次级别（k−1）第一个同侧证据（BSP 事件 ∨
    ///   背驰事件；k−1=bi 层无事件流时用方向翻转沿，SC 先例）即触发削减/
    ///   回复——不等本级别 confirmed。否定 = 价格越过 candidate 极值
    ///   （027课"只要没有打破背驰段"的逆否）。confirmed 基线路径不动
    ///   （nest 触发与掩码触发是 ∨ 关系）⇒ false 时在册行为零接触。
    ///   预注册臂：fusion_tn（t 基座）/fusion_trn（tr 基座）/
    ///   fusion_btran_s{digits}（btra 双向基座——全量普适组合，2026-06-12
    ///   任务预注册：nest = 同一买卖点的更早时间坐标，与翻空/平空动作
    ///   正交合取，卖侧 nf 触发沿翻转断面、买侧 nf 触发沿平空出口）。
    Fusion {
        trend_hold: bool,
        counter_sub: bool,
        decoupled: bool,
        trend_opts: TrendAxisOpts,
        osc: OscRouting,
        phase_clock: bool,
        r2_gate: bool,
        trend_scope: TrendScope,
        nest_forward: bool,
        /// 双向条件轴 S1-S4 [镜像推导]（fusion_btr_s{digits}；
        /// `analysis/bidirectional_nested_accounting.md` §8 +
        /// `slow_bull_vs_bh_research.md` §7.6）。per-层准入门：置位层的
        /// 卖点削减升格为翻空（{+Q,0} → {+Q,−Q} 极性对称延拓），买点
        /// 平空翻多（翻转断面）。0 = 在册行为零接触。位 ⊆ [2,5)：
        /// S3 尾部风险界 ⇒ 高层（≥recL3）空头禁用（GC recL4 −0.806 反例）。
        /// 仅预注册 fusion_tr 基座合取（探针2 对照臂口径）；× nest_forward
        /// 仅 anc 门形态（fusion_btran_s{digits}，全量普适组合预注册）。
        short_mask: u16,
        /// 镜像 anc 窗口门（fusion_btra）：开空 iff ∃j>k Trend∧Down
        /// （26:80 豁免下沉的空头镜像；S2 预注册条件化形式）。
        short_anc_gate: bool,
        /// 纯回复门消融臂（fusion_btrg）：白名单层卖点削减后进 Gated
        /// （持币 + 回复门）而非开空——分离空头暴露与回复时点门两机制
        /// （S1-S4 判决 §2.4/边界条件④ 预注册）。拒与 short_anc_gate
        /// 组合（消融对照臂 = btr，不混 anc 门）。
        short_ghost: bool,
        /// Sequence38 子腿声部下沉（38课:36 + 答疑:296 段间盘整背驰严格
        /// 形式；全量普适组合 2026-06-12 任务预注册，仅 fusion_btra*q 臂）。
        /// 驱动 counter_sub 同款 SubOut 载具（全抛/如数接回/44课铰链/
        /// 049:52 满仓义务 = 载具级规则共享），开闭词汇按 38课程式：
        /// 开 = 本级别∨次级别盘背卖；闭三岔 = 盘背买 ∨ 不破第一段低点×
        /// 次级别确认 ∨ 新下跌背驰。与 counter_sub 互斥（同一载具）；
        /// 翻空白名单层 slice 由双向词汇独占 ⇒ q 仅非白名单层激活。
        /// 与 LOU 在册差分（声明）：单层不递归、无 earning 相位拒、无
        /// 成本门（38课程式无振幅经济门——LOU 逐字）。
        seq38_sub: bool,
    },
    /// v4：统一递归 voice FSM（fusion_v；`unified_voice.rs`；
    /// `analysis/unified_recursive_voice_fsm_design.md` v2 §10 Phase 1 +
    /// 概念链修正 2026-06-12：删 R4（链外机制，L3 6/8 负）+ 字面翻空
    /// 替代 Gated（概念链第19环："走势终完美对涨跌都成立 ⇒ 卖点翻空
    /// 买点翻多 ⇒ 永远有方向"）。
    /// **零概念开关**——唯一参数 = a0（磁带粒度）。所有在册 flag 轴换成
    /// 走势结构自动读数：Φ 三值化（freeze_d 极性协变递归传导，275号同构）、
    /// R14 字面翻空全层（M=N 同股数翻转断面，1x 虚拟逐仓）、
    /// nest 双侧恒开（027课程序定理）、统一 osc 三门恒开（35:30/93:26/
    /// 49:68）、R18-20 T2W 第二翻转窗口（053:28/086:80）。
    /// 在册 Fusion 路径零接触（新入口，GH2 先例）。
    /// anc_freeze = 唯一消融臂（诊断工具非部署开关）：fusion_v = true；
    /// fusion_v_self freeze 仅自层（递归传导消融）。
    UnifiedVoice { anc_freeze: bool },
    /// v5：公理演绎统一 voice FSM（fusion_va；`axiom_voice.rs`；
    /// `analysis/unified_voice_axiom_derivation.md`）。FSM = 五条公理各自
    /// 自我否定后的扬弃之总和——零消融参数（编排者方法论定型：回测=证伪
    /// 检验非发现检验）。Φ 中枢账本相位（049:52 自层义务）× 26:80 豁免域
    /// settled-tail-kind（535 定理映射）双窗口、Osc 短差=配对闭环（锚 ZD
    /// 几何接回，049:64）、44课铰链升级、成本门 k_cost≡1（θ 因果均值）、
    /// 翻转落点恒零暴露。
    AxiomVoice,
    /// v6：嵌套递归赋格（nrf；`nested_fugue.rs`；2026-06-12 编排者任务
    /// "逐仓独立头寸和递归区间套是同一件事"）。区间套递归 = voice spawn
    /// 的时序机制：父层反向 candidate 武装窗口 × 次级别第一证据触发 ⇒
    /// **父仓不动**，在 k−1 开方向交替的独立逐仓头寸（35课立体性；概念链
    /// 第14/18/19/20/22环）；每层 voice 生命周期 = 该层走势完美（confirmed
    /// 反向词汇）；否定 = 破 candidate 极值（027:25 逆否，级联回收子树）；
    /// 递归终止 = floor（77-78课）∨ 35课成本门 ∨ 槽占用。零概念 flag——
    /// 唯一经验参数 = a0。清仓判据由 `clearance` 选择（v4 棘轮 vs 构成性
    /// 贯通 A′+regime 门；539号开放轴）——其余会计规则两形态逐字相同。
    NestedRecursive { clearance: ClearanceMode },
    /// 统一递归系统（从概念链 23 环直接翻译；清仓层 = E* 涌现归属，
    /// 零 flag——无 ClearanceMode 选项、无 regime 门、无白名单）。
    UnifiedRecursive,
    /// 递归嵌套多重赋格（"平多≠开空"推到极限；`recursive_nested_fugue.rs`）。
    /// 与 v4/URS 唯一构成性差异：**根永不平多**（除 EOD）——根层及以下一切卖点
    /// = 开空（降成本 spawn 子空），全部 regime 适应来自子空存活/死亡的净暴露
    /// 呼吸，无离散清仓决策。五条全局不变量每 bar 强制检查。零 flag。
    RecursiveNested,
    /// v7：双书独立逐仓 voice + 区间套链式递归（fusion_vd/vn/vdn；
    /// `dual_voice.rs`；2026-06-12 任务的分离读法——与 nrf 合一读法
    /// 同源分岔，回测裁决）。fusion_v 基座两正交轴，2×2 消融：
    /// - `dual_book`：每级别多头书（LayerState）+ 空头书独立逐仓并立，
    ///   非净额合并——卖点同 bar 驱动多头出清 ∧ 空头按 θ 配额开空
    ///   （翻转断面 M=N 让位于配额对称），同级别多空可共存
    ///   （`bidirectional_nested_accounting.md` v2 §4.1 m>N 承载位补全）；
    /// - `nest_deep`：区间套链式贯通（0027:11"反复进行下去直到最低级别"
    ///   字面）——∀ 中间层窗口同侧活动 ∧ bi 层翻转沿（a0 端最低结构
    ///   词汇）才触发，替换一层截断（k−1 任意证据）。
    /// (false,false) = fusion_v bit-exact 守卫臂（不暴露 parse，仅单测）。
    DualVoice { dual_book: bool, nest_deep: bool },
}

impl PolarityMode {
    pub fn parse(s: &str) -> Option<Self> {
        let fusion = |trend_hold: bool, counter_sub: bool, decoupled: bool| {
            Some(PolarityMode::Fusion {
                trend_hold,
                counter_sub,
                decoupled,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            })
        };
        // 统一配置 U：fusion_t 基座 + 相位递归路由 osc 层。
        let unified = |strong_gate: bool| {
            Some(PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Unified { strong_gate, h1_freeze: false },
                phase_clock: false,
                r2_gate: false,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            })
        };
        // P6 相位机：fusion_p = 相位机基座配对臂（research §6 P6，osc=Off）；
        // fusion_pu = 相位机基座 + 统一 osc 层（双侧同步——一个时钟修两侧）。
        // P7 R2 位置门（535 号裁决实验）：fusion_tr = kind 时钟×位置门
        // （P7 proper）；fusion_pr = 相位机×位置门（P6×P7 合取基座）；
        // fusion_pur = 合取 + 统一 osc 层（完整层 0，research §5.1）。
        let phase = |osc: OscRouting, r2_gate: bool| {
            Some(PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc,
                phase_clock: true,
                r2_gate,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            })
        };
        // anc 祖先趋势豁免（26:80 下沉；slow_bull 调研 §6 预注册两臂）：
        // hold26_anc = M1 主臂（纯祖先窗口替换自层）；fusion_ta = M3 对照臂
        // （自层 ∨ 祖先全字面）。其余轴全关（仅此二臂预注册）。
        let anc = |scope: TrendScope| {
            Some(PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: false,
                trend_scope: scope,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            })
        };
        match s {
            "cycle45" => Some(PolarityMode::Cycle45),
            "hold26" => Some(PolarityMode::Hold26 { sell_t1_only: false }),
            "hold26_t1" => Some(PolarityMode::Hold26 { sell_t1_only: true }),
            "fusion_va" => Some(PolarityMode::AxiomVoice),
            "nrf" => Some(PolarityMode::NestedRecursive { clearance: ClearanceMode::V4 }),
            "urs" => Some(PolarityMode::UnifiedRecursive),
            "rnf" => Some(PolarityMode::RecursiveNested),
            "nrf_ct" => Some(PolarityMode::NestedRecursive {
                clearance: ClearanceMode::ConstitutiveThroughput {
                    regime: RegimeGate::TopLevelTrend,
                },
            }),
            "nrf_ct_anc" => Some(PolarityMode::NestedRecursive {
                clearance: ClearanceMode::ConstitutiveThroughput {
                    regime: RegimeGate::AncestorDown,
                },
            }),
            "fusion_vd" => Some(PolarityMode::DualVoice { dual_book: true, nest_deep: false }),
            "fusion_vn" => Some(PolarityMode::DualVoice { dual_book: false, nest_deep: true }),
            "fusion_vdn" => Some(PolarityMode::DualVoice { dual_book: true, nest_deep: true }),
            "fusion_v" => Some(PolarityMode::UnifiedVoice { anc_freeze: true }),
            "fusion_v_self" => Some(PolarityMode::UnifiedVoice { anc_freeze: false }),
            "fusion" => fusion(true, true, false),
            "fusion_t" => fusion(true, false, false),
            "fusion_s" => fusion(false, true, false),
            "fusion_e" => fusion(true, true, true),
            "fusion_se" => fusion(false, true, true),
            "fusion_u" => unified(true),
            // U−③ 消融臂（预注册 P5：93:26 强震荡门是唯一新词汇，单独消融）
            "fusion_uw" => unified(false),
            "hold26_anc" => anc(TrendScope::Ancestor),
            "fusion_ta" => anc(TrendScope::SelfOrAncestor),
            "fusion_p" => phase(OscRouting::Off, false),
            "fusion_pu" => phase(OscRouting::Unified { strong_gate: true, h1_freeze: false }, false),
            "fusion_pr" => phase(OscRouting::Off, true),
            "fusion_pur" => phase(OscRouting::Unified { strong_gate: true, h1_freeze: false }, true),
            "fusion_tr" => Some(PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: true,
                trend_scope: TrendScope::SelfLayer,
                nest_forward: false,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }),
            // 区间套正向定位两臂（027课程序定理；fusion_tn = t 基座 + nest，
            // fusion_trn = tr 基座 + nest——在册最优 fusion_tr 的最小差分）。
            "fusion_tn" | "fusion_trn" => Some(PolarityMode::Fusion {
                trend_hold: true,
                counter_sub: false,
                decoupled: false,
                trend_opts: TrendAxisOpts::default(),
                osc: OscRouting::Off,
                phase_clock: false,
                r2_gate: s == "fusion_trn",
                trend_scope: TrendScope::SelfLayer,
                nest_forward: true,
                short_mask: 0,
                short_anc_gate: false,
                short_ghost: false,
                seq38_sub: false,
            }),
            // T 轴严格化臂：fusion_t + {g,d,b} 子集（规范序 g<d<b，不重复）。
            other => {
                // 双向条件轴 S1-S4 [镜像推导]：fusion_btr_s{digits} =
                // fusion_tr 基座 + 置位层卖点翻空/买点翻多；fusion_btrg_s =
                // 纯回复门消融臂（Gated 态，不开空）；fusion_btra{mods}_s =
                // 加镜像 anc 窗口门（开空 iff ∃j>k Trend∧Down）+ 全量普适
                // 组合模块集 mods ⊆ {n,u,f,q}（规范序 n<u<f<q，2026-06-12
                // 任务预注册消融矩阵）：n = 区间套正向定位；u = 统一 osc 层
                // （H4 振幅门②的承载臂）；f = H1 candidate 冻结（要求 u
                // ——无 osc 载体即无对象，f 无 u 为非法串）；q = Sequence38
                // 子腿声部。digits 每字符一层，升序无重复，∈ [2,4]——
                // S3 尾部风险界：≥5（recL3+）空头默认禁用（GC recL4
                // −0.806 单窗口反例）。
                if let Some(rest) = other.strip_prefix("fusion_btr") {
                    let Some((variant, digits)) = rest.split_once("_s") else {
                        return None;
                    };
                    let (anc_gate, ghost, mods) = if variant.is_empty() {
                        (false, false, "")
                    } else if variant == "g" {
                        (false, true, "")
                    } else if let Some(m) = variant.strip_prefix('a') {
                        (true, false, m)
                    } else {
                        return None;
                    };
                    let (mut nest, mut osc_u, mut h1, mut q38) =
                        (false, false, false, false);
                    let mut last_rank = 0u8;
                    for ch in mods.chars() {
                        let rank = match ch {
                            'n' => 1,
                            'u' => 2,
                            'f' => 3,
                            'q' => 4,
                            _ => return None,
                        };
                        if rank <= last_rank {
                            return None; // 乱序/重复 ⇒ 非法模式串
                        }
                        last_rank = rank;
                        match ch {
                            'n' => nest = true,
                            'u' => osc_u = true,
                            'f' => h1 = true,
                            'q' => q38 = true,
                            _ => unreachable!(),
                        }
                    }
                    if h1 && !osc_u {
                        return None; // H1 无 osc 载体即无对象 ⇒ 非法串
                    }
                    if digits.is_empty() {
                        return None; // 空白名单 = fusion_tr 冗余表示
                    }
                    let mut mask = 0u16;
                    let mut last = 0u32;
                    for ch in digits.chars() {
                        let lad = ch.to_digit(10)?;
                        if !(2..=4).contains(&lad) || lad <= last {
                            return None; // 越界/乱序/重复 ⇒ 非法模式串
                        }
                        last = lad;
                        mask |= 1 << lad;
                    }
                    return Some(PolarityMode::Fusion {
                        trend_hold: true,
                        counter_sub: false,
                        decoupled: false,
                        trend_opts: TrendAxisOpts::default(),
                        osc: if osc_u {
                            OscRouting::Unified { strong_gate: true, h1_freeze: h1 }
                        } else {
                            OscRouting::Off
                        },
                        phase_clock: false,
                        r2_gate: true,
                        trend_scope: TrendScope::SelfLayer,
                        nest_forward: nest,
                        short_mask: mask,
                        short_anc_gate: anc_gate,
                        short_ghost: ghost,
                        seq38_sub: q38,
                    });
                }
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
                    osc: OscRouting::Off,
                    phase_clock: false,
                    r2_gate: false,
                    trend_scope: TrendScope::SelfLayer,
                    nest_forward: false,
                    short_mask: 0,
                    short_anc_gate: false,
                    short_ghost: false,
                    seq38_sub: false,
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
    if mode == PolarityMode::AxiomVoice {
        return super::axiom_voice::run_axiom_voice(tape, floor_ladder);
    }
    if mode == PolarityMode::UnifiedRecursive {
        return super::unified_recursive::run_unified_recursive(tape, floor_ladder);
    }
    if mode == PolarityMode::RecursiveNested {
        return super::recursive_nested_fugue::run_recursive_nested_fugue(tape, floor_ladder);
    }
    if let PolarityMode::NestedRecursive { clearance } = mode {
        return super::nested_fugue::run_nested_fugue(tape, floor_ladder, clearance);
    }
    if let PolarityMode::DualVoice { dual_book, nest_deep } = mode {
        return super::dual_voice::run_dual_voice(tape, floor_ladder, dual_book, nest_deep);
    }
    if let PolarityMode::UnifiedVoice { anc_freeze } = mode {
        return super::unified_voice::run_unified_voice(tape, floor_ladder, anc_freeze);
    }
    if let PolarityMode::Fusion {
        trend_hold,
        counter_sub,
        decoupled,
        trend_opts,
        osc,
        phase_clock,
        r2_gate,
        trend_scope,
        nest_forward,
        short_mask,
        short_anc_gate,
        short_ghost,
        seq38_sub,
    } = mode
    {
        return super::positional_fusion::run_fusion(
            tape,
            floor_ladder,
            trend_hold,
            counter_sub,
            decoupled,
            trend_opts,
            osc,
            phase_clock,
            r2_gate,
            trend_scope,
            nest_forward,
            short_mask,
            short_anc_gate,
            short_ghost,
            seq38_sub,
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
            PolarityMode::Fusion { .. }
            | PolarityMode::UnifiedVoice { .. }
            | PolarityMode::AxiomVoice
            | PolarityMode::NestedRecursive { .. }
            | PolarityMode::UnifiedRecursive
            | PolarityMode::RecursiveNested
            | PolarityMode::DualVoice { .. } => {
                unreachable!("Fusion/UnifiedVoice/NestedRecursive/DualVoice 在入口已分派")
            }
        };
        let exit_reason = match mode {
            PolarityMode::Cycle45 | PolarityMode::Hold26 { sell_t1_only: true } => "sell1",
            PolarityMode::Hold26 { sell_t1_only: false } => "sellpt",
            PolarityMode::Fusion { .. }
            | PolarityMode::UnifiedVoice { .. }
            | PolarityMode::AxiomVoice
            | PolarityMode::NestedRecursive { .. }
            | PolarityMode::UnifiedRecursive
            | PolarityMode::RecursiveNested
            | PolarityMode::DualVoice { .. } => {
                unreachable!("Fusion/UnifiedVoice/NestedRecursive/DualVoice 在入口已分派")
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
                        polarity: Polarity::Long,
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
                (PolarityMode::Fusion { .. }, _)
                | (PolarityMode::UnifiedVoice { .. }, _)
                | (PolarityMode::AxiomVoice, _)
                | (PolarityMode::NestedRecursive { .. }, _)
                | (PolarityMode::UnifiedRecursive, _)
                | (PolarityMode::RecursiveNested, _)
                | (PolarityMode::DualVoice { .. }, _) => {
                    unreachable!("Fusion/UnifiedVoice/NestedRecursive/DualVoice 在入口已分派")
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
                (_, LayerState::Short { .. }) => {
                    unreachable!("Short 仅 fusion_btr 白名单层可达（已在入口分派）")
                }
                (_, LayerState::Gated { .. }) => {
                    unreachable!("Gated 仅 fusion_btrg 消融臂可达（已在入口分派）")
                }
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
                polarity: Polarity::Long,
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
