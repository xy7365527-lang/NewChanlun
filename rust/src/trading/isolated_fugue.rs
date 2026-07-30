//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! isolated_fugue — 逐仓独立声部森林（mode = "iso"）。
//!
//! 设计源头：`docs/nested_fugue_accounting.md`（§1-§8 严格会计 + §11 审计有效域）
//! + `docs/concept_movement_chain.md`（23 环）。任务（2026-06-14 编排者）：
//! **把已逐仓的 voice "栈"升级为逐仓的 voice "森林"——四改动一次性严格实装。**
//!
//! ## 与 v4（`nested_fugue.rs`）/ URS（`unified_recursive.rs`）的构成性差异
//!
//! v4/URS 的 `chain: Vec<Voice>` 是**栈**：每级别至多一个 voice，操作只在
//! 链尾，且 `acted: bool` 全局互斥（A→F 第一个 action 阻断其余）。后果：
//!   1. root@k spawn 子后，第二个 sell@k 无法消费（root 不在尾）⇒ 信号丢失。
//!   2. 同 bar 多级别信号只吃第一个 ⇒ 信号丢失。
//!   3. Type2 在区间套窗口被 `continue` 跳过 ⇒ 中枢回测确认词汇丢失。
//!
//! 本模块用**森林**消解（§11 审计："Phase 3 逐仓嵌套 D1+D6 CONFORMS ⇒ 地基
//! 稳固，可建"——双层记账原语已正确，本模块是其森林化扩展，非重写会计）：
//!
//! - **改动1（per-voice acted）**：`acted_bar` 逐 voice 标记（替代全局 `bool`）。
//!   每个 voice 每 bar 至多一个操作（避免自冲突 = 单 voice 不双动）；不同 voice
//!   （含同级别多声部）互不阻断。这是"level k 的 BSP 只看 level k 的 voice"的
//!   严格形式——per-voice ⊇ per-level（同级别多 voice 各自响应同级别信号，
//!   如两个 short@3 在 buy@3 走势完美时各自回补，per-level 会丢第二个）。
//! - **改动2（Type2 接入）**：区间套窗口对 Sell2/Buy2 与 Sell1/3/Buy1/3 同等
//!   武装（删 `continue`）。Type2 = 中枢回测确认（概念链第12环：三类买卖点 =
//!   中枢生命周期三阶段），经 located/nf 触发 spawn(E)；sell_any/buy_any 本含
//!   type2 ⇒ 回补(D)/清仓(C) 已消费，本改动补齐 spawn 侧。
//! - **改动3（净额→逐仓的森林形式）**：`VoiceLedger` 森林——每 voice 独立 ledger
//!   （units/basis/capital/cost_pool/realized_pnl + parent/children 树链）。物理
//!   一笔交易，会计分层记录。守恒律 §8.1（Σ 活跃 voice 在手 = N_base）+ §8.3
//!   （child.realized_pnl ≡ parent.cost_reduction，同一数字传导）每 bar 守卫，
//!   违反即 Err（fail-fast）。注：v4/URS 已是逐仓 voice（§11 审计 D1/D6
//!   CONFORMS），本模块的增量是**多声部森林**（root 可有多个 child，
//!   `child_voice_ids: Vec`），非"从净额改逐仓"。
//! - **改动4（BSP 不被其他 level 阻断）**：D/E 逐 voice 处理——voice@k 的回补/
//!   spawn 只读 k 级别信号 + 自身 ledger 状态，不查其他 level 仓位。改动1 的
//!   直接推论（去全局互斥 = 去跨 level 阻断）。
//!
//! ## 森林不变量（孤儿不可能定理）
//!
//! voice 仅经 `close_voice`（**后序**：先关活跃子树再结算自身）关闭 ⇒ 父关闭
//! 时子必已关闭 ⇒ **永不产生孤儿**（无需 reparent）。两种传播方向一致：
//!   · 隔离（向上不传播）：子走势完美 ⇒ 子独立回补，返还 units 给父，父不受
//!     扰动继续运作（"子voice平仓不影响父voice"，编排者裁决）。
//!   · cascade（向下传播）：父走势完美/否定/清仓 ⇒ 级联关闭整个子树（k 级别
//!     完成蕴含 k−1 子结构前提消失，17课级别完全分类）。
//! root（parent==None）仅经清仓(C)/EOD 关闭，二者皆 cascade 全树 ⇒ 关闭前森林
//! 清空 ⇒ 单根不变量（同时至多一个 root）。
//!
//! ## 每 bar 处理（去全局互斥；逐 voice/逐 level 独立）
//!
//! A. 强平兜底（逐活跃空头 voice：capital + u×(basis−c) ≤ 0 ⇒ 1x 逐仓解析强平）
//! B. 否定扫描（逐活跃 voice：破 027:25 negate_line ⇒ 关该 voice + 子树）
//! C. 清仓（根 E* 涌现层 sell_any ∧ 递归确认 ⇒ cascade 全树回现金，§6 十年 1-2 次）
//! D. 回补（逐活跃非根 voice：自层走势完美 confirmed 反向词汇 ⇒ 隔离平仓返父）
//! E. spawn 降成本（逐活跃 voice：nf 定位反向点 ∨ 根 confirmed 卖 ⇒ 释放 θ 配额
//!    给子 voice；终止 = floor(77-78课笔) ∨ 35课成本门）
//! F. 根入场（森林空 ⇒ 最高 θ 涌现层买证据满仓开多）
//!
//! 空头会计 [镜像推导]（38:36 镜像止于判断-动作序列）；earning 多空不对称
//! （§7/§11.3：空头挣负股数 L0 构造性不可表示，仅计数观测）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::nested_fugue::{rec_sub_evidence, Win};
use super::positional::{theta_weights, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// voice 生命周期状态（任务 §3 `VoiceStatus`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VoiceStatus {
    /// 持有自身仓位、参与操作。
    Active,
    /// husk：units 已全释放给子（units≈0）但未关闭——等子回补回满。仍是
    /// 合法父 + 仍计入森林（units 0），但不作为 D/E 的动作主体（无 units）。
    PendingRecovery,
    /// 已结算并返还父/现金（id 保留以维持 parent/children 引用稳定）。
    Closed,
}

/// 森林中一个 voice（一个级别的独立逐仓 ledger，任务 §3 `VoiceLedger`）。
///
/// 字段映射任务规格：`level=ladder`、`direction=dir`、`entry_price=basis`、
/// `parent_voice_id=parent`、`child_voice_ids=children`。`cost_pool`（§7
/// earning 判据）、`negate_line`（§B 027:25）、`entry_bar`（E* 涌现层 + trade
/// 行）、`acted_bar`（改动1 per-voice 互斥）是 §1-§8 会计的必要载体。
#[derive(Debug, Clone)]
pub(super) struct VoiceLedger {
    pub(super) ladder: usize,
    pub(super) dir: Polarity,
    /// 在手单位（多 = 持股；空 = 未回补敞口）。Σ 活跃 voice 在手 = N_base。
    pub(super) units: f64,
    /// 相位开仓均价（视图 P&L 锚；earning 增仓时加权更新）。
    pub(super) basis: f64,
    /// 成本池（§7 earning 判据：回补利润递减，≤0 后纯利润）。
    pub(super) cost_pool: f64,
    /// 空头 voice 在手现金（= 父层卖出所得 = 回补弹药；多头恒 0）。
    pub(super) capital: f64,
    pub(super) entry_bar: i64,
    /// 出生相否定线（spawn 时 candidate 极值，027:25）；confirmed 出生无。
    pub(super) negate_line: Option<f64>,
    pub(super) status: VoiceStatus,
    pub(super) parent: Option<usize>,
    /// 历史全部子 id（含已 Closed——id 稳定性）；活跃性按子自身 status 查。
    pub(super) children: Vec<usize>,
    /// 累计已实现 P&L（关闭时结算；§8.3 双视图一致的子侧读数）。
    pub(super) realized_pnl: f64,
    /// 本 voice 最近一次操作的 bar（改动1 per-voice 互斥；-1 = 从未）。
    pub(super) acted_bar: i64,
}

impl VoiceLedger {
    #[inline]
    fn is_active(&self) -> bool {
        self.status != VoiceStatus::Closed
    }
    /// 可作为 D/E 动作主体（活跃 ∧ 有在手单位 ∧ 本 bar 未动作）。
    /// `pub(super)`：`unified_necessity` 复用 per-voice 互斥判据（N2）。
    #[inline]
    pub(super) fn can_act(&self, bar: i64) -> bool {
        self.is_active() && self.units > 0.0 && self.acted_bar != bar
    }
    /// units 变动后刷新 husk 状态（不改 Closed）。`pub(super)`：`unified_necessity`
    /// 复用 husk 状态机（spawn/翻转后父 units→0 转 PendingRecovery）。
    #[inline]
    pub(super) fn refresh_status(&mut self) {
        if self.status == VoiceStatus::Closed {
            return;
        }
        self.status = if self.units > 1e-12 {
            VoiceStatus::Active
        } else {
            VoiceStatus::PendingRecovery
        };
    }
}

/// 物理 NAV（单一真值）：自由现金 + 活跃多头在手×价 + 活跃空头净值。
///
/// 可见性 `pub(super)`：`unified_necessity` 复用森林 NAV（同一物理单真值口径）。
///
/// 空头净值口径分两类（§8.4 审计有效域边界 + T14 根翻空扩展）：
/// - **子空头**（parent=Some）= 内部负债（父吸收，§5 递归）⇒ 冻结 capital。
///   有效域 = 父吸收 liability 的瞬时降成本子空：回补/否定时 δ 缩水由父 units
///   吸收，units 欠款在父子间内部抵消，故不独立 MtM（§8.4 "空头持冻结 capital
///   无独立 MtM 负债" 的精确有效域）。iso 全部空头属此类（根恒多）⇒ iso bit-exact。
/// - **根空头**（parent=None，T14 根翻空后）= 外部市场负债 ⇒ MtM = capital−units×c。
///   根无父吸收 liability，frozen 会使 cash 平仓返还全部 capital = 下跌利润丢失
///   （短头对）/亏损不计（短头错）= "只赚不赔"提款机 bug。MtM 使根空头的翻转/
///   否定/清仓/EOD 全部同价 c 守恒且真实兑现 P&L。
pub(super) fn nav(voices: &[VoiceLedger], free: f64, c: f64) -> f64 {
    let mut v = free;
    for x in voices.iter().filter(|x| x.is_active()) {
        match x.dir {
            Polarity::Long => v += x.units * c,
            Polarity::Short => {
                v += if x.parent.is_none() {
                    x.capital - x.units * c
                } else {
                    x.capital
                };
            }
        }
    }
    v
}

/// 根 voice 的涌现归属级别 E\*（会计 §1"根 = 最高涌现级别"；URS
/// `root_emergent_ladder` 同构——逐字复用读法，森林不改变 E\* 语义）。
fn root_emergent_ladder(
    root_ladder: usize,
    root_entry_bar: i64,
    dir_state: &[Option<Direction>; MAX_LADDER],
    anchor_state: &[i64; MAX_LADDER],
    max_l: usize,
) -> usize {
    let mut lad = root_ladder;
    while lad + 1 < max_l
        && dir_state[lad + 1] == Some(Direction::Up)
        && anchor_state[lad + 1] >= root_entry_bar
    {
        lad += 1;
    }
    lad
}

/// 递归确认（URS `recursive_confirmed` 同构）：a0(bi) 方向翻向目标侧 ∧
/// [FIRST_BSP_LADDER, estar] 每层 located 完整（弱确认过滤）。
fn recursive_confirmed(
    estar: usize,
    want: Direction,
    located: &[Option<f64>; MAX_LADDER],
    dir_state: &[Option<Direction>; MAX_LADDER],
) -> bool {
    if dir_state[FIRST_BSP_LADDER - 1] != Some(want) {
        return false;
    }
    (FIRST_BSP_LADDER..=estar).all(|k| located[k].is_some())
}

/// 结算一个 voice 的当前相位（trade 行 + 计数 + realized_pnl）。返回视图 P&L。
///
/// 可见性 `pub(super)`：`unified_necessity` 在根 in-place 翻转时复用——翻转结算前一
/// 相（长腿/空腿）的 trade 行，但**不**改 free/units/capital（settle 纯记账观测）⇒
/// 翻转的 NAV 守恒仍由翻转处的 free/capital 现金流保证（settle 对 N8 中性）。
pub(super) fn settle(
    voices: &mut [VoiceLedger],
    id: usize,
    exit_bar: i64,
    exit_price: f64,
    reason: &'static str,
    res: &mut PositionalResult,
) -> f64 {
    let v = &voices[id];
    let pnl = match v.dir {
        Polarity::Long => v.units * (exit_price - v.basis),
        Polarity::Short => v.units * (v.basis - exit_price),
    };
    if v.dir == Polarity::Short {
        res.short_net_cash_by_ladder[v.ladder] += pnl;
    }
    res.trades.push(LayerTrade {
        ladder: v.ladder as u8,
        entry_bar: v.entry_bar,
        entry_price: v.basis,
        exit_bar,
        exit_price,
        shares: v.units,
        weight_at_entry: 1.0,
        deferred_bars: 0,
        partial: false,
        exit_reason: reason,
        polarity: v.dir,
    });
    res.n_exits_by_ladder[v.ladder] += 1;
    voices[id].realized_pnl += pnl;
    pnl
}

/// 关闭一个 voice（后序：先 cascade 关活跃子树，再结算自身并返还父/现金）。
///
/// 会计原语逐字复用 `nested_fugue::pop_tail`（§2/§3/§5/§7），仅链尾访问改为
/// id 寻址 + 树后序遍历。`px` = 自身 trade 行价；子树 cascade 用市价 `c`、
/// reason="cascade"。`at_point` = 自身关闭是否在 confirmed 买卖点（§7 earning
/// 时机；cascade/negate/liq 子为 false）。
///
/// 可见性 `pub(super)`：`unified_necessity` 复用森林会计原语（N8 双层会计——
/// `close_voice` 是 `pop_tail` 的森林形式，§11 审计 D1/D6 CONFORMS）。
#[allow(clippy::too_many_arguments)]
pub(super) fn close_voice(
    id: usize,
    bar: i64,
    px: f64,
    c: f64,
    reason: &'static str,
    at_point: bool,
    voices: &mut Vec<VoiceLedger>,
    free: &mut f64,
    n_base: &mut f64,
    res: &mut PositionalResult,
) {
    debug_assert!(voices[id].is_active(), "close_voice 前提：voice 活跃");
    // ① 后序：先级联关闭活跃子树（每子返还到本 voice）。
    let kids = voices[id].children.clone();
    for k in kids {
        if voices[k].is_active() {
            let lad = voices[k].ladder;
            close_voice(k, bar, c, c, "cascade", false, voices, free, n_base, res);
            res.n_nrf_cascade_closes_by_ladder[lad] += 1;
        }
    }
    // ② 结算自身相位。
    settle(voices, id, bar, px, reason, res);
    // ③ 返还父/现金（pop_tail 会计原语，id 寻址）。
    let dir = voices[id].dir;
    let units = voices[id].units;
    let capital = voices[id].capital;
    match voices[id].parent {
        None => {
            // 根弹出：在手变现回 free，N_base 归零（清仓/否定/EOD；T14 删根恒多）。
            match dir {
                // 多头根：卖出在手 units×c（多头 capital≡0 ⇒ 与旧式 units×c+capital
                // 逐字等价；iso 永不创建空头根 ⇒ 本改动对 iso bit-exact）。
                Polarity::Long => *free += units * c + capital,
                // 空头根（T14 根翻空后；外部市场负债 MtM）：买回 units×c 平空，余
                // capital−units×c 回 free（= 空头真实兑现 P&L；与 nav() 根空头 MtM
                // 口径一致 ⇒ 同价 c 下 step 内 NAV 守恒，非"只赚不赔"冻结）。
                Polarity::Short => *free += capital - units * c,
            }
            *n_base = 0.0;
        }
        Some(p) => match dir {
            // 空子返还多父（回补/否定/强平）：capital 弹药买回 u′=min(u,K/c)，
            // 缩水 δ 传播 + N 重定基；剩余现金 = 降成本金额 ≡ 子 P&L。
            Polarity::Short => {
                let u_back = units.min(capital / c);
                let leftover = capital - u_back * c;
                let shortfall = units - u_back;
                // **A4（T38 N_base 双向重定基——亏损侧 N−δ）运行时证明**：u_back=min(units,
                // capital/c)≤units ⇒ shortfall=units−u_back≥0（单位永久缩水 δ≥0，非负——T15
                // 破极值否定亏损物化为 N−δ，非负 cost_reduction）。双向重定基：父 units +u_back
                // 与 n_base −shortfall 同源一笔（防单边记账）。violation = panic。
                assert!(
                    shortfall >= -1e-9 * units.max(1.0),
                    "A4(T38) 违反@bar {bar}：亏损重定基 shortfall={shortfall} < 0（u_back 应 ≤ units，N−δ 的 δ≥0）"
                );
                voices[p].units += u_back;
                *n_base -= shortfall;
                res.nrf_shrink_units += shortfall;
                let pool = voices[p].cost_pool.max(0.0);
                let reduce = leftover.min(pool);
                voices[p].cost_pool -= reduce;
                let excess = leftover - reduce;
                if excess > 0.0 {
                    if at_point && voices[p].dir == Polarity::Long {
                        // §7 earning：纯利润在买点买入 Δ，N 重定基；basis 加权。
                        let dq = excess / c;
                        // **A4（T38 N_base 双向重定基——earning 侧 N+Δ）运行时证明**：cost_pool≤0
                        // 后纯利润 excess>0（上方 `if excess>0` 守卫）∧ c>0 ⇒ dq>0（增正股数 +Δ，
                        // T36 多头 earning 可构造）。双向重定基：父 units +dq 与 n_base +dq 同源
                        // 一笔（同一 dq，防单边记账）。violation = panic。
                        assert!(
                            dq > 0.0 && dq.is_finite(),
                            "A4(T38) 违反@bar {bar}：earning 重定基 dq={dq} 非正/非有限（cost≤0 后纯利润买入 Δ 应 >0）"
                        );
                        let pu = voices[p].units;
                        voices[p].basis = (voices[p].basis * pu + excess) / (pu + dq);
                        voices[p].units += dq;
                        *n_base += dq;
                        res.n_nrf_earning_adds_by_ladder[voices[p].ladder] += 1;
                        res.nrf_earning_units += dq;
                        *free += reduce;
                    } else {
                        if voices[p].dir == Polarity::Short {
                            res.nrf_short_earning_hits += 1;
                        }
                        *free += leftover;
                    }
                } else {
                    *free += leftover;
                }
            }
            // 多子（孙）返还空父（§5 递归）：卖出所得回流父 capital，利润递减
            // 父成本池（空头降成本 = 均价抬高）。空头 earning L0 不可构造。
            Polarity::Long => {
                let proceeds = units * c;
                voices[p].capital += proceeds;
                voices[p].units += units;
                let profit = units * (c - voices[id].basis);
                let reduce = profit.max(0.0).min(voices[p].cost_pool.max(0.0));
                voices[p].cost_pool -= reduce;
                // 计数条件读减扣**后**的 cost_pool（与 nested_fugue::pop_tail
                // line 256 逐字一致：profit > cost_pool_after.max(0) + reduce）。
                if profit > voices[p].cost_pool.max(0.0) + reduce && at_point {
                    res.nrf_short_earning_hits += 1;
                }
            }
        },
    }
    voices[id].status = VoiceStatus::Closed;
    voices[id].units = 0.0;
    if let Some(p) = voices[id].parent {
        voices[p].refresh_status();
    }
}

/// spawn 一个子 voice（§2/§5 降成本释放；返回是否成功开仓）。父在手 −m，
/// 子诞生（多父→空子收 capital；空父→多孙 capital 买入）。成本门/floor 终止。
#[allow(clippy::too_many_arguments)]
fn try_spawn(
    parent_id: usize,
    bar: i64,
    c: f64,
    nest_fired: Option<f64>,
    floor_ladder: usize,
    depth_ref: &DepthRef,
    voices: &mut Vec<VoiceLedger>,
    res: &mut PositionalResult,
) -> bool {
    let p_ladder = voices[parent_id].ladder;
    let p_dir = voices[parent_id].dir;
    let p_units = voices[parent_id].units;
    let p_capital = voices[parent_id].capital;
    if p_ladder == floor_ladder {
        res.n_nrf_floor_stops_by_ladder[p_ladder] += 1;
        return false;
    }
    let sub = p_ladder - 1;
    match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
        None => {
            res.n_nrf_noref_rejects_by_ladder[sub] += 1;
            false
        }
        Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
            res.n_nrf_cost_rejects_by_ladder[sub] += 1;
            false
        }
        Some(_) => {
            // m = 在手 × θ_sub/θ_total（53课配额留白的 hold26 在册形态）；
            // 空头父释放受 capital 可买量约束（资金守恒）。
            let (thetas, theta_total) = theta_weights(depth_ref, floor_ladder);
            let w = thetas[sub].map(|t| t / theta_total);
            let m_quota = w.map_or(0.0, |w| p_units * w);
            let m = match p_dir {
                Polarity::Long => m_quota,
                Polarity::Short => m_quota.min(p_capital / c),
            };
            if !(m > 0.0 && m.is_finite()) {
                return false;
            }
            let child_dir = match p_dir {
                Polarity::Long => Polarity::Short,
                Polarity::Short => Polarity::Long,
            };
            let child = VoiceLedger {
                ladder: sub,
                dir: child_dir,
                units: m,
                basis: c,
                cost_pool: m * c,
                // 多父卖 m：所得 = 子空 capital；空父买回 m：子多载体无现金。
                capital: if child_dir == Polarity::Short {
                    m * c
                } else {
                    0.0
                },
                entry_bar: bar,
                negate_line: nest_fired,
                status: VoiceStatus::Active,
                parent: Some(parent_id),
                children: Vec::new(),
                realized_pnl: 0.0,
                acted_bar: bar,
            };
            let child_id = voices.len();
            voices[parent_id].units -= m;
            if child_dir == Polarity::Long {
                voices[parent_id].capital -= m * c;
            }
            voices[parent_id].children.push(child_id);
            voices[parent_id].refresh_status();
            voices.push(child);
            res.n_nrf_spawns_by_ladder[sub] += 1;
            res.n_entries_by_ladder[sub] += 1;
            true
        }
    }
}

/// 主入口（`PolarityMode::Isolated` 经 `run_positional` 分派至此）。零 flag。
pub(crate) fn run_isolated_fugue(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "isolated_fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})（BSP \
             承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("isolated_fugue 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "isolated_fugue 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理）；E* 涌现层读 \
             dir_state + anchor_state，缺 dir_flips 行即判据残缺"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut voices: Vec<VoiceLedger> = Vec::new();
    let mut free = INITIAL_CAPITAL;
    let mut n_base = 0.0f64;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut located_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    let mut anchor_state: [i64; MAX_LADDER] = [-1; MAX_LADDER];

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        let bar = i as i64;

        // 方向/段锚滚动状态（E* 涌现层 ①②；当 bar 翻转沿 flip_edge）。
        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        while flip_ptr < flips.len() && flips[flip_ptr].0 == bar {
            let (_, lad, dir) = flips[flip_ptr];
            flip_edge[lad as usize] = Some(dir);
            dir_state[lad as usize] = Some(dir);
            anchor_state[lad as usize] = bar;
            flip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 区间套窗口维护（第14环）：① 打破否定 → ② candidate 武装（改动2：
        //    Type2 同等武装）/ confirmed 清窗 → ③ 递归证据触发 ──
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if nest_sell[k].is_some_and(|w| c > w.extreme) {
                nest_sell[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
            }
            if nest_buy[k].is_some_and(|w| c < w.extreme) {
                nest_buy[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
            }
            if sig.bsp_events.is_some() {
                for e in &evrows[k] {
                    // 改动2：Type2（中枢回测确认）与 Type1/Type3 同等武装区间套
                    // 窗口——按 side 归侧（Sell* → 卖窗，Buy* → 买窗）。
                    let sellside = match e.class.side() {
                        Side::Sell => true,
                        Side::Buy => false,
                    };
                    let win = if sellside {
                        &mut nest_sell[k]
                    } else {
                        &mut nest_buy[k]
                    };
                    if e.confirmed {
                        *win = None; // confirmed 同侧让位（本 bar 走 confirmed 路径）
                    } else {
                        let ext = win.map_or(e.price, |w| {
                            if sellside {
                                w.extreme.max(e.price)
                            } else {
                                w.extreme.min(e.price)
                            }
                        });
                        *win = Some(Win { extreme: ext });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                }
            }
            // 第14环：递归下探至 a0。证据层 < k−1 ⇒ 深触发。
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, &flip_edge) {
                    nf_sell[k] = Some(w.extreme);
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, &flip_edge) {
                    nf_buy[k] = Some(w.extreme);
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }

        // 区间套定位记忆（§6.2"背驰已被区间套递归确认"）。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if let Some(ext) = nf_sell[k] {
                located_sell[k] = Some(ext);
            }
            if located_sell[k].is_some_and(|ext| c > ext) {
                located_sell[k] = None;
            }
        }

        // 最高 θ 涌现层（F 入场层）+ E* 爬升上界。
        let top = (floor_ladder..MAX_LADDER).rev().find(|&k| {
            depth_ref
                .theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS)
                .is_some()
        });
        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        // §8.3 守恒守卫的入口快照：本 bar 全部操作（A-F）在固定价 c 下进行，
        // 每条操作（spawn/close/recover/negate/liq/clearance/entry）逐项保 NAV
        // 不变（物理一笔交易不改总价值）——故 ops 后 NAV 必 == ops 前（§8.3
        // "child.P&L ≡ parent.cost_reduction" 的 §11 审计严格形式：朴素恒等式
        // 条件成立，NAV 不变性是无条件守恒律）。
        let nav_in = nav(&voices, free, c);

        // ── A. 强平兜底（逐活跃空头 voice；per-voice 互斥——强平某 voice 不
        //    阻断其他 voice 的回补/spawn）──
        let snap: Vec<usize> = (0..voices.len()).collect();
        for id in &snap {
            let id = *id;
            if !voices[id].is_active() || voices[id].dir != Polarity::Short {
                continue;
            }
            let v = &voices[id];
            if v.capital + v.units * (v.basis - c) <= 0.0 {
                let lad = v.ladder;
                let b = 2.0 * v.basis;
                close_voice(
                    id,
                    bar,
                    b,
                    c,
                    "liq",
                    false,
                    &mut voices,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                res.n_short_liquidations_by_ladder[lad] += 1;
            }
        }

        // ── B. 否定扫描（逐活跃 voice：破 027:25 极值线 ⇒ 关该 voice + 子树）──
        let snap: Vec<usize> = (0..voices.len()).collect();
        for id in &snap {
            let id = *id;
            if !voices[id].is_active() {
                continue;
            }
            let v = &voices[id];
            let broke = v.negate_line.is_some_and(|line| match v.dir {
                Polarity::Short => c > line,
                Polarity::Long => c < line,
            });
            if broke {
                let lad = v.ladder;
                close_voice(
                    id,
                    bar,
                    c,
                    c,
                    "negate",
                    false,
                    &mut voices,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
            }
        }

        // ── C. 清仓（§6）：根 E* 涌现层 sell_any ∧ 递归确认 ⇒ cascade 全树
        //    回现金（十年 1-2 次）。根 = parent==None 的活跃 voice（单根不变量）──
        let mut cleared = false;
        if let Some(root_id) = voices
            .iter()
            .position(|v| v.is_active() && v.parent.is_none() && v.units > 0.0)
        {
            let root = &voices[root_id];
            let estar = root_emergent_ladder(
                root.ladder,
                root.entry_bar,
                &dir_state,
                &anchor_state,
                max_l,
            );
            if sig.sell_any.get(estar)
                && recursive_confirmed(estar, Direction::Down, &located_sell, &dir_state)
            {
                close_voice(
                    root_id,
                    bar,
                    c,
                    c,
                    "sellpt",
                    false,
                    &mut voices,
                    &mut free,
                    &mut n_base,
                    &mut res,
                );
                located_sell = [None; MAX_LADDER];
                cleared = true;
            }
        }

        // ── D. 回补（逐活跃非根 voice：自层 confirmed 反向词汇 = 走势完美 ⇒
        //    隔离平仓返父——改动4：只读自层信号 + 自身状态，不查其他 level）──
        if !cleared {
            let snap: Vec<usize> = (0..voices.len()).collect();
            for id in &snap {
                let id = *id;
                if !voices[id].can_act(bar) || voices[id].parent.is_none() {
                    continue;
                }
                let v = &voices[id];
                let perfected = match v.dir {
                    Polarity::Short => sig.buy_any.get(v.ladder),
                    Polarity::Long => sig.sell_any.get(v.ladder),
                };
                if perfected {
                    voices[id].acted_bar = bar;
                    close_voice(
                        id,
                        bar,
                        c,
                        c,
                        "recover",
                        true,
                        &mut voices,
                        &mut free,
                        &mut n_base,
                        &mut res,
                    );
                }
            }
        }

        // ── E. spawn 降成本（逐活跃 voice：nf 定位反向点 ∨ 根 confirmed 卖 ⇒
        //    释放 θ 配额给子 voice。改动1：每 voice 独立 spawn——root 可同 bar/
        //    跨 bar 多次 spawn 出多个 child（森林）；改动4：只读自层）──
        if !cleared {
            let snap: Vec<usize> = (0..voices.len()).collect();
            for id in &snap {
                let id = *id;
                if !voices[id].can_act(bar) {
                    continue;
                }
                let dir = voices[id].dir;
                let ladder = voices[id].ladder;
                let is_root = voices[id].parent.is_none();
                let (nest_fired, confirmed_sell_root) = match dir {
                    // 根尾的 confirmed 卖（C 未消费的一切卖点，§9"其他卖点走E"）；
                    // 非根多 voice 仅 nf 定位触发（grandchild 递归同律）。
                    Polarity::Long => (nf_sell[ladder], is_root && sig.sell_any.get(ladder)),
                    Polarity::Short => (nf_buy[ladder], false),
                };
                if nest_fired.is_some() || confirmed_sell_root {
                    let ok = try_spawn(
                        id,
                        bar,
                        c,
                        nest_fired,
                        floor_ladder,
                        &depth_ref,
                        &mut voices,
                        &mut res,
                    );
                    if ok {
                        voices[id].acted_bar = bar;
                    }
                }
            }
        }

        // ── F. 根入场（森林空 ∧ 未清仓本 bar）：最高 θ 涌现层买证据 ⇒ 满仓开多 ──
        let any_active = voices.iter().any(|v| v.is_active());
        if !cleared && !any_active {
            if let Some(top) = top {
                let nf = nf_buy[top];
                if sig.buy_any.get(top) || nf.is_some() {
                    let units = free / c;
                    if units > 0.0 && units.is_finite() {
                        let line = if sig.buy_any.get(top) { None } else { nf };
                        voices.push(VoiceLedger {
                            ladder: top,
                            dir: Polarity::Long,
                            units,
                            basis: c,
                            cost_pool: free,
                            capital: 0.0,
                            entry_bar: bar,
                            negate_line: line,
                            status: VoiceStatus::Active,
                            parent: None,
                            children: Vec::new(),
                            realized_pnl: 0.0,
                            acted_bar: bar,
                        });
                        n_base = units;
                        free = 0.0;
                        res.n_nrf_root_entries_by_ladder[top] += 1;
                        res.n_entries_by_ladder[top] += 1;
                    }
                }
            }
        }

        // ── 守恒律逐 bar 强制（编排者裁决：violation = panic，非 Err——守恒
        //    违反是会计 bug 而非输入问题，必须立即 abort，不静默吞错）──
        // §8.1 股数守恒：Σ 活跃 voice 在手 = N_base。
        let sum_units: f64 = voices
            .iter()
            .filter(|v| v.is_active())
            .map(|v| v.units)
            .sum();
        assert!(
            (sum_units - n_base).abs() <= 1e-6 * n_base.max(1.0),
            "守恒律 §8.1 违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}"
        );
        // §8.3 价值守恒（NAV 不变性 = child.P&L ≡ parent.cost_reduction 的无条件
        // 严格形式）：ops 后 NAV == ops 前（同价 c 下每笔物理交易保值）。
        let nav_out = nav(&voices, free, c);
        assert!(
            (nav_out - nav_in).abs() <= 1e-4 * nav_in.abs().max(1.0),
            "守恒律 §8.3 违反@bar {bar}：NAV {nav_in} → {nav_out}（物理交易改变了总价值）"
        );

        // 观测：森林规模直方图 + 物理暴露 + 各层视图持有 bar 计数。
        let active_count = voices.iter().filter(|v| v.is_active()).count();
        res.nrf_depth_bars[active_count.min(MAX_LADDER - 1)] += 1;
        let mut long_units = 0.0;
        let mut short_units = 0.0;
        for v in voices.iter().filter(|v| v.is_active()) {
            match v.dir {
                Polarity::Long => long_units += v.units,
                Polarity::Short => short_units += v.units,
            }
            res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }
        if long_units > 0.0 {
            res.nrf_phys_long_bars += 1;
        }
        if short_units > 0.0 {
            res.nrf_phys_short_bars += 1;
        }

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, nav(&voices, free, c)));
        }
    }

    // eod：cascade 关闭根（单根不变量 ⇒ 关根即清全森林）。
    if let Some(root_id) = voices
        .iter()
        .position(|v| v.is_active() && v.parent.is_none())
    {
        let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
        let last_bar = (n as i64) - 1;
        close_voice(
            root_id,
            last_bar,
            c_last,
            c_last,
            "eod",
            false,
            &mut voices,
            &mut free,
            &mut n_base,
            &mut res,
        );
    }
    res.final_nav = free;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::{BspClass, LadderMask};

    const ISO: PolarityMode = PolarityMode::Isolated;

    fn bar(close: f64) -> BarSig {
        BarSig {
            close,
            max_ladder: 5,
            ..Default::default()
        }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() {
            (Some(50.0), Some(60.0))
        } else {
            (None, None)
        };
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs,
            zd,
            zg,
            price,
        }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
        b
    }

    fn buypt(mut b: BarSig, lad: usize) -> BarSig {
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sellpt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    fn sell1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热（层 3/4 各 SUB_COST_MIN_OBS 个锚；同 nested_fugue）。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)),
            );
            b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            rows[4][0].zd = Some(50.0);
            rows[4][0].zg = Some(53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    /// 层 2 预热补充（θ₂=0.5%；含层 2 时 θ_total=4.5%——同 nested_fugue）。
    fn warmup2(bars: &mut Vec<BarSig>) {
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(
                bar(100.0),
                2,
                ev_full(BspClass::Sell1, false, 0.0, Some(500 + j)),
            );
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            bars.push(b);
        }
    }

    fn run(bars: Vec<BarSig>, dir_flips: Vec<(i64, u8, Direction)>) -> PositionalResult {
        let t = SignalTape {
            bars,
            dir_flips: Some(dir_flips),
            ..Default::default()
        };
        run_positional(&t, 2, ISO).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("iso"), Some(ISO));
        // 缺背驰磁带 ⇒ Err。
        let t = SignalTape {
            bars: vec![with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            )],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, ISO).unwrap_err().contains("背驰磁带"));
        // 缺 dir_flips ⇒ Err。
        let t2 = SignalTape {
            bars: vec![with_empty_div(with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            ))],
            dir_flips: None,
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, ISO)
            .unwrap_err()
            .contains("dir_flips"));
    }

    #[test]
    fn root_enters_full_position_at_top() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(bar(101.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "根开在最高 θ 涌现层");
        let t = &r.trades[0];
        assert_eq!((t.ladder, t.polarity), (4, Polarity::Long));
        assert!((t.shares - INITIAL_CAPITAL / 100.0).abs() < 1e-9, "满仓");
        assert!((r.final_nav - 101_000.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_releases_theta_quota_not_all() {
        // §2 卖出原子：nf 定位卖@4 ⇒ 释放 m=N×θ₃/θ_total=250 给子空@3，根
        // 保留 750（不清仓原则）。Σ 在手 = N 守恒（引擎内 §8.1 守卫）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        ));
        bars.push(bar(104.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "子空@3 诞生");
        let short = r
            .trades
            .iter()
            .find(|t| t.polarity == Polarity::Short)
            .unwrap();
        assert!((short.shares - 250.0).abs() < 1e-9, "θ 配额 m = 1000×1/4");
        // NAV：750×104 + 26_000(子 capital) = 104_000。
        assert!(
            (r.final_nav - 104_000.0).abs() < 1e-6,
            "final={}",
            r.final_nav
        );
    }

    #[test]
    fn recovery_refills_parent_isolated() {
        // §3 回补原子：confirmed 买@3 ⇒ 平子（隔离）+ 父回满 + 降成本现金沉淀。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        ));
        bars.push(buypt(bar(96.0), 3)); // 走势完美 ⇒ 回补@96
        bars.push(bar(96.0));
        let r = run(bars, vec![]);
        let rec = r
            .trades
            .iter()
            .find(|t| t.exit_reason == "recover")
            .unwrap();
        assert_eq!((rec.ladder, rec.polarity), (3, Polarity::Short));
        let pnl = rec.shares * (rec.entry_price - rec.exit_price);
        assert!((pnl - 2000.0).abs() < 1e-6, "子 P&L = 250×8 = 2000");
        // eod NAV = 2000 + 1000×96 = 98_000。
        assert!(
            (r.final_nav - 98_000.0).abs() < 1e-6,
            "final={}",
            r.final_nav
        );
    }

    #[test]
    fn negation_kills_child_with_shrink_rebase() {
        // 027:25 否定：破极值 ⇒ 子死，capital 追价买回缩水 + N 重定基。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // 线 110
        bars.push(bar(111.0)); // 破 110 ⇒ 否定
        bars.push(bar(111.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_negate_closes_by_ladder[3], 1);
        let expect_back = 26_000.0 / 111.0;
        assert!((r.nrf_shrink_units - (250.0 - expect_back)).abs() < 1e-9);
        assert!((r.final_nav - (750.0 * 111.0 + 26_000.0)).abs() < 1e-6);
    }

    #[test]
    fn type2_arms_nest_window_and_spawns() {
        // 改动2：Type2 卖点（中枢回测确认）武装区间套卖窗 ⇒ 次级别证据触发 ⇒
        // spawn 子空@3。Type1 专属位（sell1）不置——验证 Type2 独立通路。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        // Type2 卖@4（candidate，sell1 不置）武装卖窗，极值 110。
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell2, false, 110.0, None),
        ));
        // 次级别证据@3 ⇒ nf_sell[4] ⇒ spawn 子空@3。
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        ));
        bars.push(bar(104.0));
        let r = run(bars, vec![]);
        assert_eq!(
            r.n_nrf_spawns_by_ladder[3], 1,
            "Type2 武装窗口 ⇒ spawn 子空@3"
        );
        assert!(r.n_nest_arms_by_ladder[4] >= 1, "Type2 计入武装");
    }

    #[test]
    fn root_spawns_multiple_children_forest() {
        // 改动1+3（森林核心）：root@4 在两个不同 bar 各响应一个卖点 ⇒ 长出
        // **两个** child@3——栈模型不可能（spawn 子后 root 不在尾）。验证森林。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 根 1000@100
                                         // 第一次 confirmed 卖@4（根 sell_any ⇒ E 降成本）⇒ child A@3。
        bars.push(sellpt(bar(105.0), 4));
        // 第二次 confirmed 卖@4 ⇒ child B@3（根仍在，森林再 spawn）。
        bars.push(sellpt(bar(106.0), 4));
        bars.push(bar(106.0));
        let r = run(bars, vec![]);
        assert_eq!(
            r.n_nrf_spawns_by_ladder[3], 2,
            "森林：root@4 长出两个 child@3（栈模型只吃第一个）"
        );
        // 两个 short trade 行（eod cascade 平）。
        let shorts = r
            .trades
            .iter()
            .filter(|t| t.polarity == Polarity::Short)
            .count();
        assert_eq!(shorts, 2, "两个独立 child@3 各一 trade 行");
    }

    #[test]
    fn two_children_recover_same_bar() {
        // 改动1（per-voice ⊋ per-level）：同级别两个 child@3 在同一 buy@3 走势
        // 完美时**各自**回补——per-level 互斥只会吃一个，per-voice 两个全吃。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(sellpt(bar(105.0), 4)); // child A@3
        bars.push(sellpt(bar(106.0), 4)); // child B@3
        bars.push(buypt(bar(96.0), 3)); // 走势完美@3 ⇒ A、B 同 bar 各自回补
        bars.push(bar(96.0));
        let r = run(bars, vec![]);
        let recs = r
            .trades
            .iter()
            .filter(|t| t.exit_reason == "recover")
            .count();
        assert_eq!(
            recs, 2,
            "两个 child@3 同 bar 各自回补（per-voice 非 per-level）"
        );
    }

    #[test]
    fn grandchild_long_from_short_child_capital() {
        // §5 递归嵌套：子空@3 在手，nf 定位买@3 ⇒ 释放 m2 给孙多@2，三层守恒。
        let mut bars = warmup34();
        warmup2(&mut bars); // θ₂=0.5% ⇒ 孙@2 过成本门
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // 子空@3
        bars.push(with_ev(
            bar(98.0),
            3,
            ev_full(BspClass::Buy1, false, 97.0, None),
        )); // 买窗@3
        bars.push(with_ev(
            bar(99.0),
            2,
            ev_full(BspClass::Buy1, true, 0.0, None),
        )); // 证据@2 ⇒ 孙多@2
        bars.push(bar(99.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_spawns_by_ladder[2], 1, "孙多@2 诞生（三层森林）");
        assert!(r.nrf_depth_bars[3] > 0, "三 voice 并存");
        let gc = r.trades.iter().find(|t| t.ladder == 2).unwrap();
        assert_eq!(gc.polarity, Polarity::Long);
    }

    #[test]
    fn clearance_at_emergent_perfection() {
        // §6 清仓：根 E* 涌现层完整看跌递归链 ∧ sell_any@E* ⇒ cascade 全树
        // 回现金。构造 located[4] + bi 向下 + sell_any@4（E*=4，无更高 Up 段）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // nf@4 ⇒ located[4]
            // bi 向下翻转（递归基）；located[2,3] 也需——构造完整链。
        let mut b = with_ev(bar(103.0), 2, ev_full(BspClass::Sell1, false, 104.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 105.0, None));
        bars.push(b);
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(sell1pt(bar(102.0), 4)); // sell_any@4 ∧ 完整链 ⇒ 清仓
        bars.push(bar(102.0));
        let r = run(bars, vec![(evidence_bar, 1, Direction::Down)]);
        let cleared = r.trades.iter().any(|t| t.exit_reason == "sellpt");
        assert!(cleared, "完整递归链 + sell_any@E* ⇒ 清仓回现金");
        assert!(r.nrf_depth_bars[0] > 0, "清仓后回 Idle（真正的空仓 gap）");
    }

    #[test]
    fn fund_conservation_through_recursion() {
        // 资金守恒：递归三层流转不增不减——NAV 重建 = 现金流闭合。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // 1000@100
        bars.push(with_ev(
            bar(105.0),
            4,
            ev_full(BspClass::Sell1, false, 110.0, None),
        ));
        bars.push(with_ev(
            bar(104.0),
            3,
            ev_full(BspClass::Sell1, true, 0.0, None),
        )); // 子空 250@104
        bars.push(buypt(bar(95.0), 3)); // 子走势完美 ⇒ 回补@95
        bars.push(bar(95.0));
        let r = run(bars, vec![]);
        // 根回满 1000@95 + 子利润 250×(104−95)=2250。
        let expect = 1000.0 * 95.0 + 250.0 * 9.0;
        assert!(
            (r.final_nav - expect).abs() < 1e-6,
            "final={} expect={expect}",
            r.final_nav
        );
    }

    #[test]
    fn earning_adds_units_at_buy_point_after_pool_zero() {
        // §7 earning：成本池磨穿后回补纯利润在买点买入 Δ，N 重定基。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // pool = 100_000
        for _ in 0..51 {
            let mut b = with_ev(bar(104.0), 4, ev_full(BspClass::Sell1, false, 110.0, None));
            b = with_ev(b, 3, ev_full(BspClass::Sell1, true, 0.0, None));
            bars.push(b);
            bars.push(buypt(bar(96.0), 3));
        }
        bars.push(bar(96.0));
        let r = run(bars, vec![]);
        assert!(
            r.nrf_earning_units > 0.0,
            "池磨穿后 earning 增仓：units={}",
            r.nrf_earning_units
        );
        assert!(
            r.n_nrf_earning_adds_by_ladder[4] > 0,
            "增仓发生在根层（买点时机）"
        );
        assert!(r.final_nav > 100_000.0, "短差利润沉淀 NAV 上升");
    }
}
