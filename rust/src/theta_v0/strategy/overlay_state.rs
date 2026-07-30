//! ★M5 声部执行层独立账本 `OverlayState`（多空对冲.pdf p16 关卡10 / TARGET_STRATEGY_MAXFULL.md §M5）。
//!
//! ## 本文件做什么：hedge-mode 逐声部头寸簿 P^sep → N=Net(P^sep) → Order_t=N_t−N_{t−1}
//!
//! `run_theta_v0_pi` 主路径把逐声部 sized legs **立即折成净额** `p̃→p*→order`（净额账户，PDF §10.1
//! `N=Σσ_v q_v`）——多空双开/短差在净额上**抵消**（PDF §11「同标同单位数多空价格 PnL 抵消」），
//! 只作诊断（`coverage::overlay_net_delta` 的 docstring 自声明"一旦驱动真实对冲交易执行必须升级为
//! 独立 OverlayState 账本"）。本模块兑现该升级：建**持久逐声部账本**（hedge-mode，PDF §10.2
//! `position book 保存 (Q⁺,Q⁻)` 而非只存 `Q⁺−Q⁻`），逐声部保 entry_v/exit_v/parent(v)/role(v)/pnl_v。
//!
//! ## 三个订单/账本恒等（PDF p16/p20 + §11 线性性）
//!
//! - **P^sep_t = Σ_{v∈A_t} q_v σ_v e_v**（关卡10）：每活动声部 v 一条单向腿（σ_v∈{+1,−1}），整数
//!   手数 q_v（646号：sizing 连续量 `base_units×w_depth` 在此**取整为手数**，lot 对齐）。
//! - **N_t = Net(P^sep_t) = Σ_v σ_v q_v**（关卡10 / §10.1）：净敞口（有损投影，双开 (Q,Q)↦0）。
//! - **Order_t = N_t − N_{t−1} = ΔN**（关卡10 / §9 验收 `ΔN_t=Net(P^sep_{t+1})−Net(P^sep_t)`）：
//!   净额账户真实下单量。**ΔN 守恒**（验收断言）：本模块吐的 order 恒 = 当步 net 增量（构造性），
//!   runner 据此断言 executed order signed qty == ΔN 零违例。
//!
//! ## 逐声部 pnl_v 与净额价格 PnL 对账（PDF §11 线性恒等，验收）
//!
//! PDF §10.2/§11：`Q⁺ΔP − Q⁻ΔP = (Q⁺−Q⁻)ΔP` ⟹ **hedge-mode 价格 PnL ≡ 净额账户价格 PnL**。逐 bar
//! 累计每声部 MtM 价格 PnL `pnl_v += σ_v·q_v·ΔP`，则 `Σ_v pnl_v = Σ_v σ_v q_v ΔP = N·ΔP`（Fubini
//! 求和重排，**精确对账**——差异仅来自 f64 结合律重排，`|diff|<eps`）。这是 §M5 "逐声部归因
//! pnl_v" 的兑现：多空双开/短差在净额上抵消，但在分账本上**各声部单独归因**（PDF §7 诊断型
//! `C^voice=ΣG_e` 的账本层落地——但本模块是**账户型** `ΣW_v` 的净额执行，pnl_v 是净额 PnL 的
//! 逐声部**分解**，不是独立 self-financing NAV）。
//!
//! ## 隔离声明（standalone，任务约束 674/646）
//!
//! 本模块**不复用** R 账本（`strategy::ledger` R=Π−A−W）、TW 账本（`closed_loop`）、净额适配器
//! （`nautilus::account_adapter`）——OverlayState 是执行层**新账本**（第三会计范畴，674号 R/TW 不
//! 同构）。它消费 `coverage::SepLeg`（逐声部目标头寸暴露）+ 价格，产 ΔN 订单流 + 逐声部归因表。
//! units≠q_v 范畴（646号）：`SepLeg.q_units` 是 sizing 连续量，本模块取整为整数手数 q_v。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L1**（管线正确性）：ΔN 守恒 + Σpnl_v 对账是**结构恒等**（构造性 + 线性代数），零信息增量。
//! 首轮 BTC OOS 跑数产的净值/归因表是执行层**首次真实化**的物证——**不声明 alpha**（预期亏损/
//! 空转均合法，PDF §9 判定：depth>0 active 是声部生成层验收，净值 alpha 是经济有效层，须
//! `Π^overlay/IR/回撤` 另证，本模块只兑现前两层：声部生成 + 净额可见 ΔN）。成本（fee/funding/
//! borrow/liquidation）是 M6 范畴，**不入**本模块的价格 PnL 对账（PDF §11 对账是"价格 PnL"净口径）。

use std::collections::HashMap;

use super::coverage::{SepLeg, Vertical};
use super::voice::VoiceSide;
use crate::theta_v0::classifier::recursive_tower::ElementId;

/// σ_v 的符号（Long=+1 多 / Short=−1 空 / Flat=0 不入活动集，防御性）。
pub(crate) fn side_sign(s: VoiceSide) -> i64 {
    match s {
        VoiceSide::Long => 1,
        VoiceSide::Short => -1,
        VoiceSide::Flat => 0,
    }
}

/// ★逐声部账本行 `VoiceBook`（hedge-mode position book，PDF §10.2）：一个活动声部 v 的单向腿 +
/// 归因元数据。P^sep 的一条 `q_v σ_v e_v`（PDF p16）+ §M5 要求的 entry_v/exit_v/parent(v)/role(v)/pnl_v。
#[derive(Debug, Clone)]
pub struct VoiceBook {
    /// carrier v 身份（P^sep 的声部键，跨 bar 稳定）。
    pub id: ElementId,
    /// σ_v：声部方向（Long/Short；本模块 hedge-mode 每声部单向）。
    pub side: VoiceSide,
    /// q_v：当前持有整数手数（≥0；646号取整自 `SepLeg.q_units`，lot 对齐）。
    pub q: i64,
    /// role(v)：垂直角色（`ReverseOpen`=反向子声部对冲腿，PDF §8 overlay H_t 载体；原 ShortDiff，#281 更名）。
    pub role_v: Vertical,
    /// parent(v)：真 Compose 父容器身份（None=边界胚元∂根声部）。
    pub parent_id: Option<ElementId>,
    /// entry_v：入场决策 bar。
    pub entry_bar: usize,
    /// entry_v 价：入场加权均价（resize 加仓时按手数加权更新）。
    pub entry_px: f64,
    /// pnl_v：逐 bar 累计 MtM 价格 PnL（`Σ_t σ_v·q_{v,t}·ΔP_t`，与净额价格 PnL 对账的分量）。
    pub pnl_v: f64,
}

/// ★声部离场归因行 `ClosedVoice`（§M5 exit_v/pnl_v 落盘）：一个已离场声部的完整生命周期归因。
#[derive(Debug, Clone)]
pub struct ClosedVoice {
    pub id: ElementId,
    pub side: VoiceSide,
    pub role_v: Vertical,
    pub parent_id: Option<ElementId>,
    pub entry_bar: usize,
    /// exit_v：离场决策 bar。
    pub exit_bar: usize,
    pub entry_px: f64,
    pub exit_px: f64,
    /// 该声部整个生命周期累计价格 PnL（离场时冻结）。
    pub pnl_v: f64,
}

/// ★M5 声部执行层独立账本 `OverlayState`（hedge-mode P^sep 簿）。
///
/// 逐 bar [`OverlayState::step`]：① 用**离场前持有的**净敞口 N 与逐声部 q_v 累计价格 PnL（PDF §11
/// 对账分量）；② rebalance 到本 bar 目标 P^sep（`sep_legs` 暴露的逐声部目标）；③ 产 `Order_t=ΔN`。
#[derive(Debug, Clone, Default)]
pub struct OverlayState {
    /// 当前活动声部账本（P^sep 的活动腿，键=carrier id）。
    books: HashMap<ElementId, VoiceBook>,
    /// N=Net(P^sep)=Σσ_v q_v（当前净敞口，整数手数）。冗余缓存（= books 派生），守恒断言锚。
    net: i64,
    /// 上一 bar 收盘价（价格 PnL 累计的 P_{t−1}）；None=首 bar（无 ΔP 可累计）。
    last_px: Option<f64>,
    /// 账户级累计净额价格 PnL（`Σ_t N_t·ΔP_t`，与 Σpnl_v 对账的账户侧）。
    account_price_pnl: f64,
    /// 已离场声部归因表（exit_v/pnl_v，§M5 落盘）。
    closed: Vec<ClosedVoice>,
}

/// [`OverlayState::step`] 单步产出（ΔN 订单 + 守恒见证）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayStep {
    /// N_{t−1}：rebalance 前净敞口。
    pub net_before: i64,
    /// N_t：rebalance 后净敞口（= Net(P^sep_t)）。
    pub net_after: i64,
    /// Order_t = N_t − N_{t−1} = ΔN（净额账户下单量，有符号：+买/−卖）。
    pub order: i64,
}

impl OverlayState {
    pub fn new() -> Self {
        OverlayState::default()
    }

    /// 当前净敞口 N=Net(P^sep)（守恒断言 + 净额账户驱动读出）。
    pub fn net(&self) -> i64 {
        self.net
    }

    /// 活动声部账本（只读；逐声部持仓归因）。
    pub fn active_voices(&self) -> impl Iterator<Item = &VoiceBook> {
        self.books.values()
    }

    /// 已离场声部归因表（exit_v/pnl_v）。
    pub fn closed_voices(&self) -> &[ClosedVoice] {
        &self.closed
    }

    /// 账户级累计净额价格 PnL（Σ_t N_t·ΔP_t，对账账户侧）。
    pub fn account_price_pnl(&self) -> f64 {
        self.account_price_pnl
    }

    /// Σ_v pnl_v：活动声部 + 已离场声部的价格 PnL 总和（对账分账本侧，应 ≈ [`account_price_pnl`]）。
    pub fn total_voice_pnl(&self) -> f64 {
        let active: f64 = self.books.values().map(|b| b.pnl_v).sum();
        let closed: f64 = self.closed.iter().map(|c| c.pnl_v).sum();
        active + closed
    }

    /// ★逐 bar 步进（多空对冲.pdf p16 关卡10）：目标 P^sep（`sep_legs`）+ 当前价 px + bar 序号 +
    /// lot（手数对齐粒度，`RiskConfig.default_lot.max(1)`）→ 产 ΔN 订单 + 累计逐声部价格 PnL。
    ///
    /// **步序（对账正确性关键）**：
    /// 1. **价格 PnL 累计**（用**本 bar rebalance 前**持有的 N/q_v + ΔP=px−last_px）——这是 [t−1,t]
    ///    区间实际持有的头寸，先累计再 rebalance ⟹ `Σ_v pnl_v` 与 `Σ N·ΔP` 逐 bar 对齐（PDF §11）。
    /// 2. **rebalance**：books → 目标 P^sep（新声部开 entry_v、消失声部离场记 exit_v/冻结 pnl_v、
    ///    存续声部 resize q_v）。
    /// 3. **N_t = Σσ_v q_v**，`order = N_t − N_{t−1}`（ΔN，构造性守恒）。
    ///
    /// ★146/646号取整：`q_v = round(q_units/lot)·lot`（整数手数）——sizing 连续量 → 手数。q_units<lot/2
    /// 的声部取整为 0 ⟹ 不进 P^sep（未达最小手数，诚实退化，非 bug）。
    pub fn step(&mut self, sep_legs: &[SepLeg], px: f64, bar: usize, lot: i64) -> OverlayStep {
        let lot = lot.max(1);
        // ── ① 价格 PnL 累计（rebalance 前持有的 N/q_v；ΔP=px−last_px）。 ──
        if let Some(lp) = self.last_px {
            let dp = px - lp;
            if dp != 0.0 {
                self.account_price_pnl += self.net as f64 * dp;
                for b in self.books.values_mut() {
                    b.pnl_v += side_sign(b.side) as f64 * b.q as f64 * dp;
                }
            }
        }
        self.last_px = Some(px);

        let net_before = self.net;

        // ── ② rebalance books → 目标 P^sep。 ──
        // 目标声部集：q_v 取整为手数（<lot/2 归 0 ⟹ 剔除）；同 carrier 多条 SepLeg（不应发生，
        // next_active ElementId 唯一保证）取首条方向、q 累加（防御性）。
        let mut target: HashMap<ElementId, (VoiceSide, i64, Vertical, Option<ElementId>)> =
            HashMap::new();
        for leg in sep_legs {
            let q = (leg.q_units / lot as f64).round() as i64 * lot;
            if q <= 0 {
                continue; // 未达最小手数 ⟹ 不进 P^sep（诚实退化）
            }
            let entry = target
                .entry(leg.id)
                .or_insert((leg.side, 0, leg.role_v, leg.parent_id));
            entry.1 += q;
        }

        // 离场：books 中不在 target 的声部 → 记 exit_v/冻结 pnl_v，移出账本。
        let gone: Vec<ElementId> = self
            .books
            .keys()
            .filter(|id| !target.contains_key(id))
            .copied()
            .collect();
        for id in gone {
            let b = self.books.remove(&id).expect("gone 来自 books.keys");
            self.closed.push(ClosedVoice {
                id: b.id,
                side: b.side,
                role_v: b.role_v,
                parent_id: b.parent_id,
                entry_bar: b.entry_bar,
                exit_bar: bar,
                entry_px: b.entry_px,
                exit_px: px,
                pnl_v: b.pnl_v,
            });
        }

        // 开仓 / resize：target 声部映射进 books。
        for (id, (side, q, role_v, parent_id)) in target {
            match self.books.get_mut(&id) {
                Some(b) => {
                    // resize：方向不变（同 carrier σ_v 入场固定）；加仓按手数加权更新 entry_px。
                    // 减仓（q<b.q）保 entry_px 不变（同价基剩余）；pnl_v 已在 ① 累计，不在此结算。
                    if q > b.q {
                        let added = (q - b.q) as f64;
                        b.entry_px = (b.entry_px * b.q as f64 + px * added) / q as f64;
                    }
                    b.q = q;
                    b.side = side; // 恒等（防御性覆盖）
                }
                None => {
                    self.books.insert(
                        id,
                        VoiceBook {
                            id,
                            side,
                            q,
                            role_v,
                            parent_id,
                            entry_bar: bar,
                            entry_px: px,
                            pnl_v: 0.0,
                        },
                    );
                }
            }
        }

        // ── ③ N_t = Σσ_v q_v；order = ΔN。 ──
        let net_after: i64 = self.books.values().map(|b| side_sign(b.side) * b.q).sum();
        self.net = net_after;
        OverlayStep {
            net_before,
            net_after,
            order: net_after - net_before,
        }
    }

    /// 窗口终点强平（含浮盈口径）：把全部活动声部按末价 px 离场（记 exit_v/冻结 pnl_v）。
    /// 价格 PnL 已在最后一步 [`step`] 累计到 px ⟹ 此处只搬账本行（不再累计 ΔP）。net 归零。
    pub fn force_flat(&mut self, px: f64, bar: usize) {
        let ids: Vec<ElementId> = self.books.keys().copied().collect();
        for id in ids {
            let b = self.books.remove(&id).expect("ids 来自 books.keys");
            self.closed.push(ClosedVoice {
                id: b.id,
                side: b.side,
                role_v: b.role_v,
                parent_id: b.parent_id,
                entry_bar: b.entry_bar,
                exit_bar: bar,
                entry_px: b.entry_px,
                exit_px: px,
                pnl_v: b.pnl_v,
            });
        }
        self.net = 0;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ★W1 声部独立执行簿 `VoiceExecBook`（churn 修复臂；netting-vs-voice-execution-audit-20260719 §7）
//
// 与上方 `OverlayState`（每 bar rebalance 到目标 P^sep 的**只读旁路**簿）范畴不同：本簿是
// **事件驱动**的真实执行投影——
//   ① 声部独立持仓（hedge-mode (Q⁺,Q⁻)，PDF §10.2：对冲两腿各自存续、各自结算，不互相湮灭）；
//   ② fill 只在声部开/合事件产生（事件驱动；不再每 bar 净 ΔN 兜底——churn 机制根源的修复，
//      审计 §4/§5：sizing 基准 base_units=equity_nav/px 每 bar 重算致每声部目标每 bar 漂移）；
//   ③ sizing 冻结：q_v 在开仓 fill 落簿时定死（调度侧取开仓 bar 决策层 SepLeg.q_units 取整），
//      存续期不随 NAV/价重定（审计 §6.1：base_units 从「每 bar 协变量」降级为「开仓时刻快照」）。
//
// 费口径：成交费 = 成交名义 × fee_rate（与 runner `apply_fill` :3778 同口径，逐段独立累计
// `cum_fee`）；`pnl_price` 是逐 bar MtM **价格** PnL（σ_v·q_v·ΔP 累计），`pnl_net` 是平仓
// 费后已实现 PnL（apply_fill 段1 同口径：proceeds − 含费成本基）——两字段分立，禁互相冒充
// （090；审计 §7.3「禁复用 pnl_v 冒充费后」同款）。守恒不变量（runner 侧 R 分解断言锚）：
// cash + N·px − nav0 == account_price_pnl − cum_fee − 持仓成本（构造性，逐段现金流对齐）。
// ─────────────────────────────────────────────────────────────────────────────

/// 声部执行订单类型（执行投影层订单；≠ `types.rs Order` 净额决策层订单——决策层类型不改）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceOrderKind {
    /// 开仓（qty = 冻结 q_v，手数）。
    Open,
    /// 平仓（qty 字段不用；fill 时刻 flatten 簿内该声部全部持仓——执行真值口径：
    /// 延迟成交期间簿内持仓才是真值，事件时刻簿内可能尚未成交开仓单）。
    Close,
}

/// 声部执行订单（执行投影层；voice 身份在订单上留痕——净额 `Order` 三字段无身份的对照修复）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceOrder {
    /// carrier v 身份（= `ActiveLeg::id` / `SepLeg::id`）。
    pub voice: ElementId,
    /// σ_v：声部方向（Open 的消费字段；Close 不用，占位记录事件腿方向）。
    pub side: VoiceSide,
    /// Open = 冻结 q_v（手数，开仓 bar 决策层 `SepLeg.q_units` 取整）；Close = 0（flatten 语义）。
    pub qty: i64,
    pub kind: VoiceOrderKind,
    /// 延迟成交 bar（与净额 `Order.exec_index` 同语义，spec:50）。
    pub exec_index: usize,
}

/// 声部在簿持仓（hedge-mode 单向腿 + 执行真值归因）。
#[derive(Debug, Clone)]
pub struct VoicePosition {
    pub id: ElementId,
    /// σ_v（本簿 hedge-mode 每声部单向，入场固定）。
    pub side: VoiceSide,
    /// q_v：**冻结**手数（开仓 fill 落簿时定死，存续期不变——churn 修复核心）。
    pub q: i64,
    /// 开仓 fill bar（执行真值，非决策 bar）。
    pub entry_bar: usize,
    /// 含费单位成本基 = px_fill×(1+δ·fee_rate)（`apply_fill` 段2 unit_cost 同口径）。
    pub entry_px: f64,
    /// 逐 bar 累计 MtM **价格** PnL（σ_v·q_v·ΔP；PDF §11 对账分量，毛口径不含费）。
    pub pnl_price: f64,
    /// 该声部实付成交费累计（开仓段；平仓段结算时并入 `ClosedVoiceExec.fee_paid`）。
    pub fee_paid: f64,
}

/// 已结算声部执行归因行（一个 campaign 的完整生命周期：开→平/强平）。
#[derive(Debug, Clone)]
pub struct ClosedVoiceExec {
    pub id: ElementId,
    pub side: VoiceSide,
    /// 开仓 fill bar。
    pub entry_bar: usize,
    /// 平仓 fill bar（forced=窗口终点虚拟兑现 bar）。
    pub exit_bar: usize,
    /// 含费开仓成本基。
    pub entry_px: f64,
    /// 平仓成交价（forced=末可交易 bar close）。
    pub exit_px: f64,
    /// 生命周期 MtM 价格 PnL（毛，σ_v·q_v·ΔP 累计）。
    pub pnl_price: f64,
    /// 实付成交费（开+平；forced 的假设性平仓费未实付、不入本字段——见 settle_forced_virtual）。
    pub fee_paid: f64,
    /// 费后已实现 PnL（apply_fill 段1 同口径；forced=虚拟兑现口径，含假设性平仓费净额）。
    pub pnl_net: f64,
    /// 窗口终点强平（虚拟兑现，含浮盈口径）标记。
    pub forced: bool,
}

/// 声部平仓结算信息（runner 装配 trade_pnls / TradeRecord 的输入）。
#[derive(Debug, Clone, Copy)]
pub struct VoiceCloseInfo {
    /// 开仓 fill bar。
    pub entry_bar: usize,
    /// 平掉的手数。
    pub qty: i64,
    /// 方向（true=多）。
    pub long: bool,
    /// 费后已实现 PnL。
    pub pnl: f64,
}

/// 声部 fill 产出（executed_qty=0 ⟹ 未成交（拒开/无仓可平），不计 fill 事件）。
#[derive(Debug, Clone, Copy)]
pub struct VoiceFillOutcome {
    pub executed_qty: f64,
    /// 平仓分量（Some ⟹ 本 fill 结算了一个声部 campaign）。
    pub closed: Option<VoiceCloseInfo>,
}

/// ★W1 声部独立执行簿（事件驱动 + sizing 冻结；hedge-mode position book，PDF §10.2）。
#[derive(Debug, Clone)]
pub struct VoiceExecBook {
    /// 在簿声部持仓（键 = carrier id）。
    positions: HashMap<ElementId, VoicePosition>,
    /// 已结算声部归因表。
    closed: Vec<ClosedVoiceExec>,
    /// 声部账户现金（nav0 起；真实执行投影的现金真值）。
    cash: f64,
    /// 账户基线（runner nav0 同口径：initial_nav>0 ? initial_nav : 1.0）。
    nav0: f64,
    /// 上一有效价（MtM 累计用；px≤0 不刷新，与 runner prev_px 同口径）。
    last_px: Option<f64>,
    /// 账户级累计价格 PnL（Σ_t N_t·ΔP_t，N=Σσ_v q_v 派生净敞口）。
    account_price_pnl: f64,
    /// 累计成交费（Commission+Slippage+Tax，逐 fill 独立测得——R 分解守恒断言锚）。
    cum_fee: f64,
    /// fill 事件数（executed>0 才计；= runner 输出 n_orders 在声部臂下的值）。
    n_fills: usize,
    /// 开过的声部 campaign 总数（含在飞）。
    total_voices: usize,
    /// 声部毛周转（手数）G = Σ(q开+q平)（事件驱动下 = 实际成交周转 T，审计 §7.4 验收量）。
    gross_turnover_lots: i64,
}

impl VoiceExecBook {
    pub fn new(initial_nav: f64) -> Self {
        let nav0 = if initial_nav > 0.0 { initial_nav } else { 1.0 };
        VoiceExecBook {
            positions: HashMap::new(),
            closed: Vec::new(),
            cash: nav0,
            nav0,
            last_px: None,
            account_price_pnl: 0.0,
            cum_fee: 0.0,
            n_fills: 0,
            total_voices: 0,
            gross_turnover_lots: 0,
        }
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }
    pub fn nav0(&self) -> f64 {
        self.nav0
    }
    /// N_derived = Σ_v σ_v·q_v（派生只读净敞口；权益 MtM 与持仓成本计费用）。
    pub fn net_signed(&self) -> i64 {
        self.positions
            .values()
            .map(|p| side_sign(p.side) * p.q)
            .sum()
    }
    pub fn account_price_pnl(&self) -> f64 {
        self.account_price_pnl
    }
    pub fn cum_fee(&self) -> f64 {
        self.cum_fee
    }
    pub fn n_fills(&self) -> usize {
        self.n_fills
    }
    pub fn total_voices(&self) -> usize {
        self.total_voices
    }
    pub fn gross_turnover_lots(&self) -> i64 {
        self.gross_turnover_lots
    }
    /// 窗口终点仍在飞的声部数（censored）。
    pub fn n_open_end(&self) -> usize {
        self.positions.len()
    }
    pub fn positions(&self) -> impl Iterator<Item = &VoicePosition> {
        self.positions.values()
    }
    pub fn closed_voices(&self) -> &[ClosedVoiceExec] {
        &self.closed
    }
    /// Σ_v pnl_price（活动 + 已结算；PDF §11 对账分账本侧，应 ≈ account_price_pnl）。
    pub fn total_voice_price_pnl(&self) -> f64 {
        let active: f64 = self.positions.values().map(|p| p.pnl_price).sum();
        let closed: f64 = self.closed.iter().map(|c| c.pnl_price).sum();
        active + closed
    }

    /// 逐 bar MtM 价格 PnL 累计（runner 每 bar **成交前**调用——簿内持仓是前 bar 收盘
    /// 持仓，与净额臂 `cum_price_pnl += units·Δpx`（runner.rs 循环顶）同一时点口径）。
    pub fn mark_to_market(&mut self, px: f64) {
        if px > 0.0 {
            if let Some(lp) = self.last_px {
                let dp = px - lp;
                if dp != 0.0 {
                    let net = self.net_signed();
                    self.account_price_pnl += net as f64 * dp;
                    for p in self.positions.values_mut() {
                        p.pnl_price += side_sign(p.side) as f64 * p.q as f64 * dp;
                    }
                }
            }
            self.last_px = Some(px);
        }
    }

    /// 开仓 fill（q_lots 已在调度侧冻结；现金约束与 `apply_fill` 段2 同口径——开多需现金
    /// 充足否则拒开（executed=0），开空收现金无预付）。
    pub fn apply_open(
        &mut self,
        id: ElementId,
        side: VoiceSide,
        q_lots: i64,
        px: f64,
        bar: usize,
        fee_rate: f64,
    ) -> VoiceFillOutcome {
        let noop = VoiceFillOutcome {
            executed_qty: 0.0,
            closed: None,
        };
        if q_lots <= 0 || px <= 0.0 {
            return noop;
        }
        // 防御性：同 carrier 已有在簿持仓。正常路径不发生（close→reopen 由 Close 订单先平，
        // runner 调度同 bar 平单先于开单）；release 兜底先按本价结算旧 campaign 再开新。
        debug_assert!(
            !self.positions.contains_key(&id),
            "W1: 声部 {:?} 开仓时在簿已有持仓（close 事件缺失/排序违例）",
            id
        );
        let replaced = if self.positions.contains_key(&id) {
            self.flatten(id, px, bar, fee_rate, false)
                .map(|(info, _fee)| info)
        } else {
            None
        };
        let delta = side_sign(side) as f64;
        if delta == 0.0 {
            return VoiceFillOutcome {
                executed_qty: 0.0,
                closed: replaced,
            }; // Flat 不开仓
        }
        let qty = q_lots as f64;
        let cost = qty * px * (1.0 + fee_rate);
        if delta > 0.0 && self.cash < cost {
            return VoiceFillOutcome {
                executed_qty: 0.0,
                closed: replaced,
            }; // 现金不足拒开
        }
        // 开仓现金流（apply_fill 段2 同口径）：cash += −δ·qty·px·(1+δ·fee)。
        let cash_flow = -delta * qty * px * (1.0 + delta * fee_rate);
        let fee = qty * px * fee_rate;
        self.cash += cash_flow;
        self.cum_fee += fee;
        self.positions.insert(
            id,
            VoicePosition {
                id,
                side,
                q: q_lots, // ★冻结：存续期不再重定（churn 修复）
                entry_bar: bar,
                entry_px: px * (1.0 + delta * fee_rate),
                pnl_price: 0.0,
                fee_paid: fee,
            },
        );
        self.n_fills += 1;
        self.total_voices += 1;
        self.gross_turnover_lots += q_lots;
        VoiceFillOutcome {
            executed_qty: qty,
            closed: replaced,
        }
    }

    /// 平仓 fill：flatten 簿内该声部全部持仓（无持仓 ⟹ no-op executed=0——restore 祖先腿/
    /// 开仓单未成交等情形，与 typed ledger「表中无登记 ⟹ 不入 ledger」同语义）。
    pub fn apply_close(
        &mut self,
        id: ElementId,
        px: f64,
        bar: usize,
        fee_rate: f64,
    ) -> VoiceFillOutcome {
        match self.flatten(id, px, bar, fee_rate, false) {
            Some((info, _fee)) => {
                self.n_fills += 1;
                VoiceFillOutcome {
                    executed_qty: info.qty as f64,
                    closed: Some(info),
                }
            }
            None => VoiceFillOutcome {
                executed_qty: 0.0,
                closed: None,
            },
        }
    }

    /// 内部平仓（真实现金流；apply_fill 段1 同口径）。返回 (结算信息, 平仓费)。
    fn flatten(
        &mut self,
        id: ElementId,
        px: f64,
        bar: usize,
        fee_rate: f64,
        forced: bool,
    ) -> Option<(VoiceCloseInfo, f64)> {
        let pos = self.positions.remove(&id)?;
        let sigma = side_sign(pos.side) as f64;
        let qty = pos.q as f64;
        // 平仓现金流：平多（σ+1）+qty·px·(1−fee)；平空（σ−1）−qty·px·(1+fee)。
        let cash_flow = sigma * qty * px * (1.0 - sigma * fee_rate);
        let fee = qty * px * fee_rate;
        let px_exit_net = px * (1.0 - sigma * fee_rate);
        // 费后已实现 PnL = σ·(px_exit_net − 含费成本基)·qty（apply_fill 段1 同口径）。
        let pnl = sigma * (px_exit_net - pos.entry_px) * qty;
        self.cash += cash_flow;
        self.cum_fee += fee;
        self.gross_turnover_lots += pos.q;
        let fee_total = pos.fee_paid + fee;
        self.closed.push(ClosedVoiceExec {
            id,
            side: pos.side,
            entry_bar: pos.entry_bar,
            exit_bar: bar,
            entry_px: pos.entry_px,
            exit_px: px,
            pnl_price: pos.pnl_price,
            fee_paid: fee_total,
            pnl_net: pnl,
            forced,
        });
        Some((
            VoiceCloseInfo {
                entry_bar: pos.entry_bar,
                qty: pos.q,
                long: pos.side == VoiceSide::Long,
                pnl,
            },
            fee,
        ))
    }

    /// 窗口终点强平（含浮盈口径，与净额臂 `forced_pnl` 同铁律）：**虚拟兑现**——不改 cash、
    /// 不计 fill、不计费（净额臂 forced_pnl 同样不动 cash/units，final equity 按持仓 MtM 计）；
    /// 逐声部产 forced pnl 行 + 归因行入 closed（forced=true；fee_paid 只记实付开仓费，
    /// 假设性平仓费已净入 pnl_net 但未实付、不入 fee_paid——090 口径分立）。
    /// 返回确定序（entry_bar, level, ordinal 排序，bit-exact 可复现）。
    pub fn settle_forced_virtual(
        &mut self,
        last_px: f64,
        exit_bar: usize,
        fee_rate: f64,
    ) -> Vec<VoiceCloseInfo> {
        let mut rows: Vec<(usize, u32, u64, ElementId)> = self
            .positions
            .values()
            .map(|p| (p.entry_bar, p.id.level, p.id.ordinal, p.id))
            .collect();
        rows.sort_by_key(|(eb, lvl, ord, _)| (*eb, *lvl, *ord));
        let mut out = Vec::with_capacity(rows.len());
        for (_, _, _, id) in rows {
            if let Some(pos) = self.positions.remove(&id) {
                let sigma = side_sign(pos.side) as f64;
                let qty = pos.q as f64;
                let px_exit_net = last_px * (1.0 - sigma * fee_rate);
                let pnl = sigma * (px_exit_net - pos.entry_px) * qty;
                self.closed.push(ClosedVoiceExec {
                    id,
                    side: pos.side,
                    entry_bar: pos.entry_bar,
                    exit_bar,
                    entry_px: pos.entry_px,
                    exit_px: last_px,
                    pnl_price: pos.pnl_price,
                    fee_paid: pos.fee_paid,
                    pnl_net: pnl,
                    forced: true,
                });
                out.push(VoiceCloseInfo {
                    entry_bar: pos.entry_bar,
                    qty: pos.q,
                    long: pos.side == VoiceSide::Long,
                    pnl,
                });
            }
        }
        out
    }

    /// 持仓成本/罚金扣款（funding/borrow/liq；runner 镜像净额臂口径对 N_derived 计费后经此
    /// 扣声部账户现金——守恒断言要求 cash 真扣）。cost_model=None ⟹ 永不调用（bit-exact）。
    pub fn debit_cash(&mut self, amount: f64) {
        self.cash -= amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::strategy::coverage::{SepLeg, Vertical};

    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    fn leg(id: ElementId, side: VoiceSide, q_units: f64, role_v: Vertical) -> SepLeg {
        SepLeg {
            id,
            side,
            q_units,
            role_v,
            parent_id: None,
        }
    }

    /// ★ΔN 守恒（验收断言1，PDF p16 `Order_t=N_t−N_{t−1}`）：order 逐步恒 = net 增量，零违例。
    #[test]
    fn delta_n_conservation() {
        let mut ov = OverlayState::new();
        let root = eid(1, 0);
        // t0：开根多头 10 手 → N=10, order=+10。
        let s0 = ov.step(
            &[leg(root, VoiceSide::Long, 10.0, Vertical::Ambient)],
            100.0,
            0,
            1,
        );
        assert_eq!(s0.order, 10);
        assert_eq!(s0.order, s0.net_after - s0.net_before, "ΔN 守恒");
        assert_eq!(ov.net(), 10);
        // t1：加仓到 15 手 → N=15, order=+5。
        let s1 = ov.step(
            &[leg(root, VoiceSide::Long, 15.0, Vertical::Ambient)],
            101.0,
            1,
            1,
        );
        assert_eq!(s1.order, 5);
        assert_eq!(s1.order, s1.net_after - s1.net_before, "ΔN 守恒");
        // t2：全平（空目标）→ N=0, order=−15。
        let s2 = ov.step(&[], 102.0, 2, 1);
        assert_eq!(s2.order, -15);
        assert_eq!(ov.net(), 0);
        assert_eq!(ov.closed_voices().len(), 1, "根声部离场记 1 行");
    }

    /// ★双开非零 + ΔN（PDF §10.2 hedge-mode）：父多头 10 + 子空头 10（ReverseOpen）⟹ 净 N=0（双开
    /// 净额退化 C26），但 P^sep 有两条腿（active_voices=2）——净额账户 order=0 但账本记两声部。
    #[test]
    fn hedged_two_voices_net_zero_but_book_nonzero() {
        let mut ov = OverlayState::new();
        let parent = eid(2, 0);
        let child = eid(1, 0);
        let s = ov.step(
            &[
                leg(parent, VoiceSide::Long, 10.0, Vertical::FollowParent),
                leg(child, VoiceSide::Short, 10.0, Vertical::ReverseOpen),
            ],
            100.0,
            0,
            1,
        );
        assert_eq!(s.net_after, 0, "双开净额退化 N=Σσq=+10−10=0（C26/PDF §11）");
        assert_eq!(s.order, 0, "净额账户 order=0（多空抵消）");
        assert_eq!(
            ov.active_voices().count(),
            2,
            "P^sep 保留两条腿（hedge-mode，PDF §10.2）"
        );
    }

    /// ★Σpnl_v 与净额价格 PnL 对账（验收断言2，PDF §11 线性恒等 `Σσ_v q_v ΔP=N ΔP`）：
    /// 父多头 10 + 子空头 6（部分对冲，净 N=4），价格 100→110→105，逐声部 pnl_v 之和恒 = N·ΔP 累计。
    #[test]
    fn per_voice_pnl_reconciles_with_net() {
        let mut ov = OverlayState::new();
        let parent = eid(2, 0);
        let child = eid(1, 0);
        let target = [
            leg(parent, VoiceSide::Long, 10.0, Vertical::FollowParent),
            leg(child, VoiceSide::Short, 6.0, Vertical::ReverseOpen),
        ];
        ov.step(&target, 100.0, 0, 1); // 开仓，N=10−6=4
        assert_eq!(ov.net(), 4);
        ov.step(&target, 110.0, 1, 1); // ΔP=+10：account += 4·10=40；父 +10·10=100，子 −6·10=−60
        ov.step(&target, 105.0, 2, 1); // ΔP=−5：account += 4·(−5)=−20；父 −50，子 +30
                                       // 账户侧：40−20=20；分账本侧：父 100−50=50，子 −60+30=−30 ⟹ 20。
        assert!((ov.account_price_pnl() - 20.0).abs() < 1e-9, "N·ΔP 累计=20");
        assert!(
            (ov.total_voice_pnl() - ov.account_price_pnl()).abs() < 1e-9,
            "Σpnl_v == 账户净额价格 PnL（PDF §11 线性对账）"
        );
    }

    /// ★取整：q_units<lot/2 归 0 ⟹ 不进 P^sep（诚实退化，非 bug）。
    #[test]
    fn subunit_leg_rounds_to_zero() {
        let mut ov = OverlayState::new();
        let s = ov.step(
            &[leg(eid(1, 0), VoiceSide::Long, 0.4, Vertical::Ambient)],
            100.0,
            0,
            1,
        );
        assert_eq!(s.net_after, 0, "0.4 手 round→0 ⟹ 不进 P^sep");
        assert_eq!(ov.active_voices().count(), 0);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★W1 VoiceExecBook（声部独立执行簿：事件驱动 + sizing 冻结）
    // ──────────────────────────────────────────────────────────────────────

    /// ★W1 守恒（验收）：开多 10@100 → MtM 110 → 平@110，fee=3bps。
    /// 账本净变动（cash−nav0，N=0）== 价格 PnL − 成交费（构造性恒等，逐段现金流对齐）。
    #[test]
    fn voice_exec_open_close_conservation() {
        let fee = 0.0003;
        let mut vb = VoiceExecBook::new(1.0e6);
        let v = eid(1, 0);
        vb.mark_to_market(100.0); // bar0：首价，无 ΔP
        let o = vb.apply_open(v, VoiceSide::Long, 10, 100.0, 0, fee);
        assert_eq!(o.executed_qty, 10.0);
        assert_eq!(vb.net_signed(), 10);
        assert_eq!(vb.positions().next().unwrap().q, 10, "q_v 冻结为开仓手数");
        vb.mark_to_market(110.0); // ΔP=+10：account += 10·10=100；pnl_price += 100
        assert!((vb.account_price_pnl() - 100.0).abs() < 1e-9);
        let c = vb.apply_close(v, 110.0, 1, fee);
        assert_eq!(c.executed_qty, 10.0);
        let info = c.closed.expect("平仓结算");
        // 费后 PnL = (110·0.9997 − 100·1.0003)·10 = (109.967−100.03)·10 = 99.37。
        assert!((info.pnl - 99.37).abs() < 1e-9, "费后 PnL={}", info.pnl);
        assert_eq!(vb.net_signed(), 0);
        assert_eq!(
            vb.n_fills(),
            2,
            "开+平 = 2 个 fill 事件（事件驱动上界 2×1 声部）"
        );
        assert_eq!(vb.gross_turnover_lots(), 20, "G = q开+q平 = 20 手");
        // 守恒：cash−nav0 == account_price_pnl − cum_fee（N=0 终点）。
        let fee_total = 10.0 * 100.0 * fee + 10.0 * 110.0 * fee; // 0.30+0.33=0.63
        assert!((vb.cum_fee() - fee_total).abs() < 1e-9);
        let ledger_delta = vb.cash() - vb.nav0();
        let net_r = vb.account_price_pnl() - vb.cum_fee();
        assert!(
            (ledger_delta - net_r).abs() < 1e-9,
            "守恒：{} vs {}",
            ledger_delta,
            net_r
        );
        // Σpnl_price 对账（PDF §11）：唯一声部 pnl_price=100 == account。
        assert!((vb.total_voice_price_pnl() - vb.account_price_pnl()).abs() < 1e-9);
        assert_eq!(vb.closed_voices().len(), 1);
        assert!(!vb.closed_voices()[0].forced);
    }

    /// ★W1 声部独立持仓守恒（hedge-mode，PDF §10.2）：父多 10 + 子空 6 同时存续（不互相
    /// 湮灭），N_derived=Σσ_v q_v=4 与声部簿逐声部之和一致；各自独立结算。
    #[test]
    fn voice_exec_hedged_voices_independent_books() {
        let fee = 0.0003;
        let mut vb = VoiceExecBook::new(1.0e6);
        let parent = eid(2, 0);
        let child = eid(1, 0);
        vb.mark_to_market(100.0);
        vb.apply_open(parent, VoiceSide::Long, 10, 100.0, 0, fee);
        vb.apply_open(child, VoiceSide::Short, 6, 100.0, 0, fee);
        // 守恒断言：Σ_v σ_v·q_v（簿派生）== 逐声部手数有符号和。
        let sum: i64 = vb
            .positions()
            .map(|p| match p.side {
                VoiceSide::Long => p.q,
                VoiceSide::Short => -p.q,
                VoiceSide::Flat => 0,
            })
            .sum();
        assert_eq!(
            vb.net_signed(),
            4,
            "对冲两腿毛额存续，净敞口 10−6=4（不湮灭）"
        );
        assert_eq!(vb.net_signed(), sum, "Σ_v σ_v q_v 与声部簿一致");
        vb.mark_to_market(110.0); // account += 4·10=40；父+100，子−60
        assert!((vb.account_price_pnl() - 40.0).abs() < 1e-9);
        // 各自独立结算：只平空腿，多腿存续。
        vb.apply_close(child, 110.0, 1, fee);
        assert_eq!(vb.net_signed(), 10, "空腿已平，多腿独立存续");
        assert_eq!(vb.closed_voices().len(), 1);
        assert_eq!(vb.n_open_end(), 1);
        assert!(
            (vb.total_voice_price_pnl() - vb.account_price_pnl()).abs() < 1e-9,
            "Σpnl_price 对账"
        );
    }

    /// ★W1 窗口终点强平（虚拟兑现）：不改 cash、不计 fill、不计费；pnl 含假设性平仓费净额；
    /// 归因行入 closed（forced=true）。
    #[test]
    fn voice_exec_forced_settle_virtual_no_cash() {
        let fee = 0.0003;
        let mut vb = VoiceExecBook::new(1.0e6);
        let v = eid(1, 0);
        vb.mark_to_market(100.0);
        vb.apply_open(v, VoiceSide::Long, 10, 100.0, 0, fee);
        vb.mark_to_market(110.0);
        let cash_before = vb.cash();
        let fee_before = vb.cum_fee();
        let fills_before = vb.n_fills();
        let rows = vb.settle_forced_virtual(110.0, 19, fee);
        assert_eq!(rows.len(), 1);
        // 虚拟兑现 pnl = (110·0.9997 − 100·1.0003)·10 = 99.37（与真平同价同额）。
        assert!((rows[0].pnl - 99.37).abs() < 1e-9);
        assert_eq!(
            vb.cash(),
            cash_before,
            "虚拟兑现不改现金（与净额臂 forced_pnl 同口径）"
        );
        assert_eq!(vb.cum_fee(), fee_before, "假设性平仓费未实付");
        assert_eq!(vb.n_fills(), fills_before, "强平虚拟兑现不计 fill 事件");
        assert_eq!(vb.n_open_end(), 0);
        assert!(vb.closed_voices()[0].forced);
        assert!(
            (vb.closed_voices()[0].fee_paid - 10.0 * 100.0 * fee).abs() < 1e-9,
            "fee_paid 只记实付开仓费"
        );
    }

    /// ★W1 现金约束（apply_fill 段2 同口径）：开多现金不足 ⟹ 拒开（executed=0，不计 fill、
    /// 不入簿、不计费——不伪造执行事实）。
    #[test]
    fn voice_exec_open_rejected_on_insufficient_cash() {
        let mut vb = VoiceExecBook::new(100.0);
        let o = vb.apply_open(eid(1, 0), VoiceSide::Long, 10, 100.0, 0, 0.0003);
        assert_eq!(
            o.executed_qty, 0.0,
            "10 手×100×1.0003=1000.3 > 现金 100 ⟹ 拒开"
        );
        assert_eq!(vb.n_fills(), 0);
        assert_eq!(vb.net_signed(), 0);
        assert_eq!(vb.cum_fee(), 0.0);
    }

    /// ★W1 平仓无持仓 no-op（restore 祖先腿/未成交开仓单的 close 事件）：executed=0，不计 fill。
    #[test]
    fn voice_exec_close_without_position_noop() {
        let mut vb = VoiceExecBook::new(1.0e6);
        let o = vb.apply_close(eid(9, 9), 100.0, 3, 0.0003);
        assert_eq!(o.executed_qty, 0.0);
        assert!(o.closed.is_none());
        assert_eq!(vb.n_fills(), 0);
    }
}
