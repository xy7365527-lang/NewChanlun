//! #951 建新件：生产 π 回路的**流式单 bar 驱动** `ThetaPiStream`（流式/批量共核的核心抽法）。
//!
//! 本模块必须**无条件编译**（不随 `backtest` 模块的 `any(test, feature = "backtest_bin")` 门控）——
//! 它是 PyO3 出口 [`crate::theta_v0::ffi::PyThetaStream`] 的决策内核，而 PyO3 导出在默认
//! cdylib（Python 扩展）构建下就必须可达。
//!
//! ## 与批量 fill loop 的关系（共核在哪、不共核在哪）
//!
//! 本流式驱动逐 bar 调用的**决策内核**与 [`crate::theta_v0::backtest::fill::pi_theta_fill_loop_overlay`]
//! 完全同源——同一份 [`crate::theta_v0::strategy::coverage::pi_theta_step_traced_with_risk_seeds`]
//! （Γ→桶→活动集→p̃→LexArgmin→p_star→Schedule_Θ，产 `StepTrace.sep_legs`），同一份
//! [`crate::theta_v0::strategy::overlay_state::OverlayState::step`] 与
//! [`crate::theta_v0::strategy::level_ledger::LevelLedgerMirror::step`]（per-leg 账本只读旁路）。
//!
//! 本流式驱动**不携带** fill loop 里与 π 决策核正交的批量专有机械（中枢震荡 campaign、
//! PanDiv、nest-gate、voice-exec、typed/shadow 簿、TW 账本推进、gamma/opsem dump）——那些是
//! 回测 runner 的额外机械，不属于「目标净敞口 + per-leg 账本」这两样 D1 出口。
//!
//! ## 状态裁定（设计稿 §1.3）
//!
//! 沿用 `theta_v0` 的**无状态持仓口径**：`p_t`（调用方真实净持仓，手数）每 bar 由调用方显式
//! 传入，本流式驱动**不维护影子仓位**、**不回填成交回执**。订单由 Python 侧照
//! `rec_t_strategy.py:_rebalance` 的老办法算 `delta = target − portfolio.net_position`。
//! `push_bar` 返回 `p_star`（目标净持仓，有符号手数，lot 对齐）——与 `recursive_t` 的
//! `push_bar -> lu−su` 同范畴同量纲。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! **L0/L1**（操作语义结构，非 L2 alpha）：确定性结构变换（Γ→…→p_star→Schedule_Θ→per-leg 账本）。
//! `p_star` 是 `pi_theta_position` 的 LexArgmin 结果，不蕴含实盘盈利。

use super::classifier::streaming::OwnedIncrementalClassifier;
use super::config::ThetaConfig;
use super::strategy::coverage::{self, KThetaRiskGate, PiThetaWeights, SepLeg};
use super::strategy::interp::{self, ActiveLeg, TreeCache};
use super::strategy::level_ledger::LevelLedgerMirror;
use super::strategy::overlay_state::OverlayState;
use super::strategy::persistent::PersistentRegistry;
use super::strategy::protocol::ProtocolEventSet;
use super::types::{Bar, Order, StrictAction};
use std::collections::HashSet;

/// 生产 π 回路的流式单 bar 驱动（#951 seam：`push_bar(bar, p_t, nav) -> f64` 返回 `p_star`）。
pub struct ThetaPiStream {
    config: ThetaConfig,
    /// 自持缓冲区增量分类器（无条件编译，`append_bar` 吃 owned bar）。
    classifier: OwnedIncrementalClassifier,
    /// tree-prefix 缓存（与 fill loop 同款，`forest_epoch` O(1) 命中判据）。
    tree_cache: TreeCache,
    /// 跨 bar 持久元素注册表 Pi（anc.pdf §4-§9，restore 祖先腿）。
    registry: PersistentRegistry,
    /// 确认-bar 部署 seen-set（append-only，买卖点身份键）。
    seen_bsps: HashSet<(usize, usize, u8)>,
    /// 活动集台账（thread 跨 bar；interp::interpret 闭环递归）。
    prev_active: Vec<ActiveLeg>,
    /// 逐声部持久账本（hedge-mode P^sep 簿；per-leg 出口）。
    overlay: OverlayState,
    /// 级别账本只读镜像（按 `id.level` 分桶；per-leg 出口）。
    level_ledger: LevelLedgerMirror,
    /// 已消费 bar 数（= 当前 bar 序号，喂 `exec_index` 与 `OverlayState::step`）。
    bar_idx: usize,
    /// 上一步 p*（[`target_net_units`](Self::target_net_units) 读出）。
    p_star: f64,
    /// 上一个可交易决策 bar 的收盘价（USD）——[`finish`](Self::finish) 窗口终点强平锚。
    last_px: Option<f64>,
    /// 上一步 Schedule_Θ 订单形态（诊断，非下单指令）。
    last_order: Order,
    /// 上一步 P^sep_{t+1}（逐声部目标腿；只读快照）。
    last_sep_legs: Vec<SepLeg>,
    /// ΔN 非零步数（overlay 订单 != 0 的决策点数）。
    n_orders: usize,
}

impl ThetaPiStream {
    pub fn new(config: ThetaConfig) -> Self {
        Self {
            classifier: OwnedIncrementalClassifier::new(config.clone()),
            config,
            tree_cache: TreeCache::new(),
            registry: PersistentRegistry::new(),
            seen_bsps: HashSet::new(),
            prev_active: Vec::new(),
            overlay: OverlayState::new(),
            level_ledger: LevelLedgerMirror::new(),
            bar_idx: 0,
            p_star: 0.0,
            last_px: None,
            last_order: Order {
                action: StrictAction::Wait,
                qty: 0,
                exec_index: 0,
            },
            last_sep_legs: Vec::new(),
            n_orders: 0,
        }
    }

    /// 推一根 bar，返回目标净持仓 `p_star`（有符号手数，lot 对齐）。
    ///
    /// `p_t` = 调用方真实净持仓（手数，有符号，正多/负空/0 空仓）；`nav` = 账户净值（>0，否则
    /// 内部钳到 1.0）。不可交易 bar（`untradable`）或 `close<=0` ⟹ 不产决策，返回上一步 `p_star`
    /// （分类器仍照常 `append_bar` 以保持增量血缘锁步）。
    pub fn push_bar(&mut self, bar: Bar, p_t: f64, nav: f64) -> f64 {
        let i = self.bar_idx;
        self.bar_idx += 1;
        let (classification, tower) = self.classifier.append_bar(bar);
        let px = bar.close as f64 * self.config.tick.tick_size;
        if bar.untradable || px <= 0.0 {
            // 分类器已推进；决策层跳过（与 fill loop `if !bar.untradable && px > 0.0` 同口径）。
            return self.p_star;
        }
        let classification_step = newly_confirmed_step(&classification, &mut self.seen_bsps);
        let nav = if nav > 0.0 { nav } else { 1.0 };
        let base_units = nav / px; // U_ℓ：NAV/价 = 可建名义手数（方案A协变）
        let weights = PiThetaWeights::from_risk(&self.config.risk);
        let gate = KThetaRiskGate::open(); // 无保证金 ⟹ 𝒦_Θ=[−cap,+cap] 全开
        let forest_epoch = self.classifier.forest_epoch();
        let (step_tree, step_candidates, step_gamma) =
            interp::coverage_elements_and_gamma_with_tower_cached_gen(
                &classification_step,
                &tower,
                &mut Some(&mut self.tree_cache),
                Some(forest_epoch),
                Some(forest_epoch),
            );
        let mut step_work = coverage::ElementView::from_parts(&step_tree, step_candidates);
        if let Some((sib, id)) = self.tree_cache.tree_sibling_and_id() {
            step_work = step_work.with_base_indices(sib, id);
        }
        // χ≡1（全覆盖，不滤候选集）——生产回测 π 路径 `run_theta_v0_pi_overlay` 的 `chi=None` 同款。
        let step_gamma_trade = step_gamma.clone();
        let protocol = ProtocolEventSet::hold(0);
        let (next_active, p_star, (order, _protocol_event), step_trace) =
            coverage::pi_theta_step_traced_with_risk_seeds(
                step_work,
                &step_gamma_trade,
                &self.prev_active,
                p_t,
                i,
                base_units,
                &self.config.risk,
                weights,
                gate,
                &[], // 无 risk-close seeds（无保证金/强平门）
                &self.config.voice,
                &self.registry,
                None, // 无 TW 源 ⟹ P2/P3/P4 跳过（旧调用方 bit-exact 原路径）
                &protocol,
            );
        self.prev_active = next_active;
        let lot = self.config.risk.default_lot.max(1) as i64;
        let ostep = self.overlay.step(&step_trace.sep_legs, px, i, lot);
        let _lstep = self.level_ledger.step(&step_trace.sep_legs, px, i, lot);
        self.level_ledger.observe_lee_net(self.overlay.net());
        if ostep.order != 0 {
            self.n_orders += 1;
        }
        self.p_star = p_star;
        self.last_order = order;
        self.last_sep_legs = step_trace.sep_legs.clone();
        self.last_px = Some(px);
        p_star
    }

    /// 上一步的 p*（不推进 bar）。对位 `RecTStream.target_net_units()`。
    pub fn target_net_units(&self) -> f64 {
        self.p_star
    }

    /// 上一步 Schedule_Θ 的订单形态（诊断用，**不是**下单指令——生产下单由 Python 侧 delta 提单）。
    pub fn last_order(&self) -> Order {
        self.last_order
    }

    /// 上一步 P^sep_{t+1}（逐声部目标腿快照；只读）。
    pub fn sep_legs(&self) -> &[SepLeg] {
        &self.last_sep_legs
    }

    /// 逐声部持久账本（per-leg 出口 ② 的活动/已离场声部归因）。
    pub fn overlay(&self) -> &OverlayState {
        &self.overlay
    }

    /// 级别账本只读镜像（per-leg 出口 ② 的级别分桶 + LEE-Net 见证）。
    pub fn level_ledger(&self) -> &LevelLedgerMirror {
        &self.level_ledger
    }

    /// ΔN 非零步数。
    pub fn n_orders(&self) -> usize {
        self.n_orders
    }

    /// 已消费 bar 数（快照/锁步护栏）。
    pub fn bar_count(&self) -> usize {
        self.bar_idx
    }

    /// 快照：(bar_count, p_star, n_active_voices, n_active_levels)。
    pub fn snapshot(&self) -> (usize, f64, usize, usize) {
        (
            self.bar_idx,
            self.p_star,
            self.overlay.active_voices().count(),
            self.level_ledger.levels().count(),
        )
    }

    /// 收尾：窗口终点强平（全部活动声部按最后一个可交易决策 bar 收盘价离场，记 exit_v/冻结
    /// pnl_v）。镜像 fill loop 的 `ov.force_flat`/`ll.force_flat` 窗口终点段（fill.rs:5958-5970）。
    /// 幂等：已平仓时 `force_flat` 无活动声部 ⟹ no-op。
    pub fn finish(&mut self) {
        if let Some(px) = self.last_px {
            self.overlay.force_flat(px, self.bar_idx);
            self.level_ledger.force_flat(px, self.bar_idx);
        }
    }
}

/// 买卖点身份判别 u8（seen-set append-only diff 键；6 类 bit 打包）。
///
/// 逐字同源于 `theta_v0::backtest::signal::bsp_bits_disc`（该模块门控于 `backtest_bin`，本模块
/// 无条件编译不能引用）。★#951 登记：这是「门控源 → 无条件流」的**故意小复制**（3 行），收敛
/// 到共享无条件模块留待后续票——若未来有第二处无条件消费方，必须先把两者搬到一个共享模块。
fn bsp_bits_disc(b: &super::types::BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// per-bar 确认-bar 部署（append-only diff vs `seen`）。
///
/// 逐字同源于 `theta_v0::backtest::signal::newly_confirmed_step`（同上，门控复制）。本 bar 新确认的
/// 买卖点保留、其余级别字段清空（σ_p 父容器方向由 `tower` 经 `attach_bsp_to_tree` 查得，不读
/// classification moves/centers）。
fn newly_confirmed_step(
    classification: &super::classifier::Classification,
    seen: &mut HashSet<(usize, usize, u8)>,
) -> super::classifier::Classification {
    use super::classifier::LevelState;
    use std::rc::Rc;
    super::classifier::Classification {
        levels: classification
            .levels
            .iter()
            .enumerate()
            .map(|(lvl, ls)| LevelState {
                moves: Vec::new(),
                centers: Rc::new(Vec::new()),
                cp_ownership: Rc::new(Vec::new()),
                pan_div: Rc::new(Vec::new()),
                first_class_grades: Rc::new(Vec::new()),
                bsp: ls
                    .bsp
                    .iter()
                    .filter(|p| seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))))
                    .cloned()
                    .collect::<Vec<_>>()
                    .into(),
                level_projection: None,
            })
            .collect(),
    }
}
