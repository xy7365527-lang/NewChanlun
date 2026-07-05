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
fn side_sign(s: VoiceSide) -> i64 {
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
    /// role(v)：垂直角色（`ShortDiff`=反向子声部对冲腿，PDF §8 overlay H_t 载体）。
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
            let entry = target.entry(leg.id).or_insert((leg.side, 0, leg.role_v, leg.parent_id));
            entry.1 += q;
        }

        // 离场：books 中不在 target 的声部 → 记 exit_v/冻结 pnl_v，移出账本。
        let gone: Vec<ElementId> =
            self.books.keys().filter(|id| !target.contains_key(id)).copied().collect();
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
                    self.books.insert(id, VoiceBook {
                        id,
                        side,
                        q,
                        role_v,
                        parent_id,
                        entry_bar: bar,
                        entry_px: px,
                        pnl_v: 0.0,
                    });
                }
            }
        }

        // ── ③ N_t = Σσ_v q_v；order = ΔN。 ──
        let net_after: i64 = self.books.values().map(|b| side_sign(b.side) * b.q).sum();
        self.net = net_after;
        OverlayStep { net_before, net_after, order: net_after - net_before }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::strategy::coverage::{SepLeg, Vertical};

    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    fn leg(id: ElementId, side: VoiceSide, q_units: f64, role_v: Vertical) -> SepLeg {
        SepLeg { id, side, q_units, role_v, parent_id: None }
    }

    /// ★ΔN 守恒（验收断言1，PDF p16 `Order_t=N_t−N_{t−1}`）：order 逐步恒 = net 增量，零违例。
    #[test]
    fn delta_n_conservation() {
        let mut ov = OverlayState::new();
        let root = eid(1, 0);
        // t0：开根多头 10 手 → N=10, order=+10。
        let s0 = ov.step(&[leg(root, VoiceSide::Long, 10.0, Vertical::Ambient)], 100.0, 0, 1);
        assert_eq!(s0.order, 10);
        assert_eq!(s0.order, s0.net_after - s0.net_before, "ΔN 守恒");
        assert_eq!(ov.net(), 10);
        // t1：加仓到 15 手 → N=15, order=+5。
        let s1 = ov.step(&[leg(root, VoiceSide::Long, 15.0, Vertical::Ambient)], 101.0, 1, 1);
        assert_eq!(s1.order, 5);
        assert_eq!(s1.order, s1.net_after - s1.net_before, "ΔN 守恒");
        // t2：全平（空目标）→ N=0, order=−15。
        let s2 = ov.step(&[], 102.0, 2, 1);
        assert_eq!(s2.order, -15);
        assert_eq!(ov.net(), 0);
        assert_eq!(ov.closed_voices().len(), 1, "根声部离场记 1 行");
    }

    /// ★双开非零 + ΔN（PDF §10.2 hedge-mode）：父多头 10 + 子空头 10（ShortDiff）⟹ 净 N=0（双开
    /// 净额退化 C26），但 P^sep 有两条腿（active_voices=2）——净额账户 order=0 但账本记两声部。
    #[test]
    fn hedged_two_voices_net_zero_but_book_nonzero() {
        let mut ov = OverlayState::new();
        let parent = eid(2, 0);
        let child = eid(1, 0);
        let s = ov.step(
            &[
                leg(parent, VoiceSide::Long, 10.0, Vertical::FollowParent),
                leg(child, VoiceSide::Short, 10.0, Vertical::ShortDiff),
            ],
            100.0,
            0,
            1,
        );
        assert_eq!(s.net_after, 0, "双开净额退化 N=Σσq=+10−10=0（C26/PDF §11）");
        assert_eq!(s.order, 0, "净额账户 order=0（多空抵消）");
        assert_eq!(ov.active_voices().count(), 2, "P^sep 保留两条腿（hedge-mode，PDF §10.2）");
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
            leg(child, VoiceSide::Short, 6.0, Vertical::ShortDiff),
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
        let s = ov.step(&[leg(eid(1, 0), VoiceSide::Long, 0.4, Vertical::Ambient)], 100.0, 0, 1);
        assert_eq!(s.net_after, 0, "0.4 手 round→0 ⟹ 不进 P^sep");
        assert_eq!(ov.active_voices().count(), 0);
    }
}
