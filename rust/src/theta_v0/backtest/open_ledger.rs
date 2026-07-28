//! #571 typed ledger 在飞表：以 `(carrier, generation)` 标识 campaign 实例。
//!
//! 结构层关闭事件只携 carrier；本模块按 generation 升序 drain 该 carrier 的全部 live
//! campaign，各自结算为一条 [`TypedTrade`]。新开不会把旧 generation 假结算为 supersede。
//!
//! # drain 的有效域声明（#596 MED-1）
//!
//! **已验证**：drain 使 typed ledger 与**决策层** `StepTrace` 生命周期事件一一对应——每条
//! `opened` 恰对一条 `TypedTrade`（wf8 BTC 双向 multiset join 795/795、失配 0/0，键
//! `(entry_bar, level, source_index, dir)`；#571 报告 §9.3）。这是 L2（真实数据）等级的证据，
//! 有效域 = 「typed ledger ↔ 决策层腿事件」这一条对应关系。
//!
//! **未验证、且上述 join 结构上无法证伪**：typed ledger 与**执行层**腿数的对应。自 #571 起
//! 同一 carrier 可并存 N 个 live generation，各自落一条 typed 行；而净额执行路径（`apply_order`
//! 单一净持仓）与 `SepLeg`（按 carrier 唯一，`strategy/coverage/step.rs` #512 守卫）对该 carrier
//! 只有**一条**记录 ⟹ **typed trade 数 ≥ 执行层腿数**，同 carrier 多实例时严格大于。795/795 join
//! 的两侧都在决策层（`step_trace.opened` ↔ typed ledger），换任何执行层读数都不进该 join ⟹ 它
//! 无论通过与否都不能证伪这一不等式。
//!
//! **下游约束**：凡把 typed ledger 行数/行级 `units` 当作执行层腿数或腿级持仓消费的读数，
//! 都落在本模块已验证的有效域**之外**，需自带执行层证据。`LedgerOpen::units` 本身是 carrier 级
//! 目标快照而非实例级增量——其单份取用见 [`OpenTable::parent_units`]（#596 HIGH-1）。

use std::collections::{BTreeSet, HashMap, HashSet};

use super::super::classifier::recursive_tower::ElementId;
use super::super::strategy::coverage::Vertical;
use super::super::strategy::interp::{reverse_exit_type, ExitType};
use super::ledger::{LedgerOpen, TwLedgerThread, TypedTrade};
use super::opsem_dump::OpsemDump;
use super::selector::ZExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OpenKey {
    carrier: ElementId,
    generation: u32,
}

pub(super) struct OpenTable<V> {
    live: HashMap<OpenKey, V>,
    generations_by_carrier: HashMap<ElementId, BTreeSet<u32>>,
    generation_hiwater: HashMap<ElementId, u32>,
}

impl<V> OpenTable<V> {
    pub(super) fn new() -> Self {
        Self {
            live: HashMap::new(),
            generations_by_carrier: HashMap::new(),
            generation_hiwater: HashMap::new(),
        }
    }

    pub(super) fn open<F>(&mut self, carrier: ElementId, build: F) -> u32
    where
        F: FnOnce(u32) -> V,
    {
        let generation = self.next_generation(carrier);
        let key = OpenKey {
            carrier,
            generation,
        };
        let old = self.live.insert(key, build(generation));
        assert!(
            old.is_none(),
            "#571 open-table 重复主键：carrier={carrier:?}, generation={generation}"
        );
        let inserted = self
            .generations_by_carrier
            .entry(carrier)
            .or_default()
            .insert(generation);
        assert!(
            inserted,
            "#571 carrier 索引重复 generation：carrier={carrier:?}, generation={generation}"
        );
        generation
    }

    fn next_generation(&mut self, carrier: ElementId) -> u32 {
        let generation = match self.generation_hiwater.get(&carrier) {
            Some(&hi) => hi
                .checked_add(1)
                .unwrap_or_else(|| panic!("#571 generation 溢出：carrier={carrier:?}, hi={hi}")),
            None => 0,
        };
        self.generation_hiwater.insert(carrier, generation);
        generation
    }

    pub(super) fn live_for_carrier(&self, carrier: ElementId) -> impl Iterator<Item = &V> {
        self.generations_by_carrier
            .get(&carrier)
            .into_iter()
            .flatten()
            .map(move |&generation| self.expect_live(carrier, generation))
    }

    fn expect_live(&self, carrier: ElementId, generation: u32) -> &V {
        self.live
            .get(&OpenKey {
                carrier,
                generation,
            })
            .unwrap_or_else(|| {
                panic!("#571 open-table 索引悬空：carrier={carrier:?}, generation={generation}")
            })
    }

    pub(super) fn drain_carrier(&mut self, carrier: ElementId) -> Vec<(u32, V)> {
        let Some(generations) = self.generations_by_carrier.remove(&carrier) else {
            return Vec::new();
        };
        generations
            .into_iter()
            .map(|generation| {
                let key = OpenKey {
                    carrier,
                    generation,
                };
                let value = self.live.remove(&key).unwrap_or_else(|| {
                    panic!("#571 drain 索引悬空：carrier={carrier:?}, generation={generation}")
                });
                (generation, value)
            })
            .collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.live.len()
    }
}

/// 在飞表的只读实例视图（风控门 [`super::admission::k_theta_risk_gate`] 唯一消费方）。
///
/// #571 之前一个 carrier 至多一条 `LedgerOpen`，风控 `get(carrier)` 即得全部；改键后同
/// carrier 可并存多 generation ⟹ 止损必须**逐实例**判（#570 探针 §3「风控止损」行：
/// 按实例 side/stop 聚合 long/short stop）。本 trait 是该枚举面的最小接口。
pub(super) trait LiveOpenView {
    fn for_each_live(&self, carrier: ElementId, visit: &mut dyn FnMut(&LedgerOpen));
}

impl LiveOpenView for OpenTable<LedgerOpen> {
    fn for_each_live(&self, carrier: ElementId, visit: &mut dyn FnMut(&LedgerOpen)) {
        self.live_for_carrier(carrier).for_each(visit);
    }
}

pub(super) struct SettlementContext<'a> {
    exit_bar: usize,
    exit_px: f64,
    exit_ext: &'a ZExt,
    typed_ledger: &'a mut Vec<TypedTrade>,
    opsem: &'a mut Option<OpsemDump>,
    tw_thread: &'a mut TwLedgerThread,
}

pub(super) struct ExitSnapshot<'a> {
    bar: usize,
    px: f64,
    ext: &'a ZExt,
}

impl<'a> ExitSnapshot<'a> {
    pub(super) fn new(bar: usize, px: f64, ext: &'a ZExt) -> Self {
        Self { bar, px, ext }
    }
}

impl<'a> SettlementContext<'a> {
    pub(super) fn new(
        exit: ExitSnapshot<'a>,
        typed_ledger: &'a mut Vec<TypedTrade>,
        opsem: &'a mut Option<OpsemDump>,
        tw_thread: &'a mut TwLedgerThread,
    ) -> Self {
        Self {
            exit_bar: exit.bar,
            exit_px: exit.px,
            exit_ext: exit.ext,
            typed_ledger,
            opsem,
            tw_thread,
        }
    }
}

#[derive(Clone, Copy)]
enum CloseReason {
    Reverse(u8),
    Silent,
    Risk,
    Overlay,
}

#[derive(Clone, Copy)]
struct SettlementSpec {
    exit_type: ExitType,
    via_structural_prune: bool,
    trigger_bsp_class: Option<u8>,
    close_tw: bool,
}

impl OpenTable<LedgerOpen> {
    pub(super) fn settle_reverse(
        &mut self,
        carrier: ElementId,
        trigger_bsp_class: u8,
        ctx: &mut SettlementContext<'_>,
    ) {
        self.settle_carrier(carrier, CloseReason::Reverse(trigger_bsp_class), ctx);
    }

    pub(super) fn settle_silent(&mut self, carrier: ElementId, ctx: &mut SettlementContext<'_>) {
        self.settle_carrier(carrier, CloseReason::Silent, ctx);
    }

    pub(super) fn settle_risk(&mut self, carrier: ElementId, ctx: &mut SettlementContext<'_>) {
        self.settle_carrier(carrier, CloseReason::Risk, ctx);
    }

    pub(super) fn settle_overlay(&mut self, carrier: ElementId, ctx: &mut SettlementContext<'_>) {
        self.settle_carrier(carrier, CloseReason::Overlay, ctx);
    }

    fn settle_carrier(
        &mut self,
        carrier: ElementId,
        reason: CloseReason,
        ctx: &mut SettlementContext<'_>,
    ) {
        for (generation, open) in self.drain_carrier(carrier) {
            assert_eq!(
                open.position_node_id.generation, generation,
                "#571 键/值 generation 漂移：carrier={carrier:?}"
            );
            let spec = settlement_spec(reason, &open);
            settle_one(carrier, open, spec, ctx);
        }
    }

    pub(super) fn settle_all_hold(&mut self, ctx: &mut SettlementContext<'_>) {
        let mut censored: Vec<(OpenKey, LedgerOpen)> = self.live.drain().collect();
        self.generations_by_carrier.clear();
        censored.sort_by_key(|(key, open)| {
            (
                open.entry_bar,
                key.carrier.level,
                key.carrier.ordinal,
                key.generation,
            )
        });
        for (key, open) in censored {
            assert_eq!(
                open.position_node_id.generation, key.generation,
                "#571 Hold 键/值 generation 漂移：carrier={:?}",
                key.carrier
            );
            let spec = SettlementSpec {
                exit_type: ExitType::Hold,
                via_structural_prune: false,
                trigger_bsp_class: None,
                close_tw: false,
            };
            settle_one(key.carrier, open, spec, ctx);
        }
    }

    pub(super) fn shortdiff_carriers(&self) -> HashSet<ElementId> {
        self.live
            .iter()
            .filter(|(_, open)| open.entry_v == Vertical::ShortDiff)
            .map(|(key, _)| key.carrier)
            .collect()
    }

    /// PanDiv 父腿快照：该 carrier 的**单份** sizing 目标（`Option::None` = 该 carrier 当前不出父腿）。
    ///
    /// #596 HIGH-1：`LedgerOpen::units` 是开腿当步冻结的 `SepLeg::q_units`，而 `SepLeg` **按 carrier
    /// 唯一**（`strategy/coverage/step.rs` #512 守卫对 `next_active` 重复 `ElementId` fail-loud）⟹ 同
    /// carrier 的 N 个 live generation 持有的是同一份 carrier 级目标的 N 份拷贝，**不是** N 份可叠加
    /// 的腿量。求和 = 同一份 units 计 N 次 ⟹ `OscillationParentLeg::target_units` 翻 N 倍 ⟹ 震荡子仓
    /// 下单量翻 N 倍。故取单份。
    ///
    /// **取法与 #571 前单槽世界逐点等价**（#600 MED-1）：先取 `generation` 最大的在飞实例（= 单槽
    /// 世界里 supersede 覆盖后 `get(carrier)` 唯一能拿到的那条），**再**判其 `entry_v`——最新实例为
    /// `ShortDiff` ⟹ `None`。顺序不可倒：先滤 `ShortDiff` 再取最大者**不**等价——单槽世界里被
    /// ShortDiff 覆盖掉的旧 Ambient 快照已不存在，反序却会让它复活并以父腿身份给震荡子仓定量。
    ///
    /// 「最大 generation」由 `max_by_key(position_node_id.generation)` 自证，不依赖
    /// [`Self::live_for_carrier`] 的 `BTreeSet<u32>` 升序迭代这一隐式契约。非法 units 断言逐实例
    /// 施加（fail-loud 覆盖全部在飞实例，不只被选中的那份）。
    pub(super) fn parent_units(&self, carrier: ElementId) -> Option<u64> {
        for open in self.live_for_carrier(carrier) {
            assert!(
                open.units.is_finite() && open.units >= 0.0,
                "#571 PanDiv 父快照非法 units：carrier={carrier:?}, units={}",
                open.units
            );
        }
        // tie 不可达：同 carrier 内 generation 唯一，由插入期的 `assert!(inserted, ..)`（:73-76）
        // 保证——`max_by_key` 遇 tie 时取迭代序末位的隐式规则在此不生效，因为唯一键使 tie 集恒为单元素。
        let latest = self
            .live_for_carrier(carrier)
            .max_by_key(|open| open.position_node_id.generation)?;
        if latest.entry_v == Vertical::ShortDiff {
            return None;
        }
        let units = latest.units;
        assert!(
            units.round() < u64::MAX as f64,
            "#571 PanDiv 父快照 units 溢出：carrier={carrier:?}, units={units}"
        );
        Some(units.round() as u64)
    }
}

fn settlement_spec(reason: CloseReason, open: &LedgerOpen) -> SettlementSpec {
    match reason {
        CloseReason::Reverse(trigger) => SettlementSpec {
            exit_type: reverse_exit_type(open.entry_v, trigger),
            via_structural_prune: false,
            trigger_bsp_class: Some(trigger),
            close_tw: true,
        },
        CloseReason::Silent => SettlementSpec {
            exit_type: if open.entry_v == Vertical::Ambient {
                ExitType::CloseRoot
            } else {
                ExitType::CloseShortDiff
            },
            via_structural_prune: true,
            trigger_bsp_class: None,
            close_tw: true,
        },
        CloseReason::Risk => SettlementSpec {
            exit_type: ExitType::RiskExit,
            via_structural_prune: false,
            trigger_bsp_class: None,
            close_tw: true,
        },
        CloseReason::Overlay => SettlementSpec {
            exit_type: ExitType::CloseShortDiff,
            via_structural_prune: false,
            trigger_bsp_class: None,
            close_tw: true,
        },
    }
}

fn settle_one(
    carrier: ElementId,
    open: LedgerOpen,
    spec: SettlementSpec,
    ctx: &mut SettlementContext<'_>,
) {
    if spec.close_tw && open.entry_v == Vertical::ShortDiff {
        ctx.tw_thread.close_share_leg();
    }
    let pushed = TypedTrade {
        entry_z: open.entry_z,
        voice_id: carrier,
        entry_bar: open.entry_bar,
        exit_bar: ctx.exit_bar,
        exit_type: spec.exit_type,
        entry_px: open.entry_px,
        exit_px: ctx.exit_px,
        via_structural_prune: spec.via_structural_prune,
        position_node_id: open.position_node_id,
        entry_stop_dist: open.entry_stop_dist,
        exit_z: super::selector::exit_z_of(open.entry_z, ctx.exit_ext),
        units: open.units,
    };
    write_trade_fail_loud(&pushed, &open, spec.trigger_bsp_class, ctx);
    ctx.typed_ledger.push(pushed);
}

fn write_trade_fail_loud(
    trade: &TypedTrade,
    open: &LedgerOpen,
    trigger_bsp_class: Option<u8>,
    ctx: &mut SettlementContext<'_>,
) {
    let Some(dump) = ctx.opsem.as_mut() else {
        return;
    };
    dump.mark_exit(ctx.exit_bar);
    dump.write_trade(trade, open, trigger_bsp_class)
        .unwrap_or_else(|error| {
            panic!(
                "#571 OPSEM trade 写入失败：carrier={:?}, generation={}, exit_bar={}, error={error}",
                trade.voice_id, trade.position_node_id.generation, trade.exit_bar
            )
        });
}

#[cfg(test)]
mod tests {
    use super::super::super::strategy::interp::PositionNodeId;
    use super::super::super::strategy::ledger::RiskPolicy;
    use super::super::super::strategy::voice::VoiceSide;
    use super::super::super::types::BspBits;
    use super::super::ledger::OpsemEntrySnapshot;
    use super::super::mu_estimator::{MuClass, PositionState};
    use super::*;

    fn carrier(ordinal: u64) -> ElementId {
        ElementId { level: 1, ordinal }
    }

    fn test_z(side: VoiceSide) -> MuClass {
        let (delta, bits) = match side {
            VoiceSide::Long => (
                1,
                BspBits {
                    buy1: true,
                    ..Default::default()
                },
            ),
            VoiceSide::Short => (
                -1,
                BspBits {
                    sell1: true,
                    ..Default::default()
                },
            ),
            VoiceSide::Flat => panic!("#571 测试不构造 Flat campaign"),
        };
        MuClass::from_certificate(1, delta, bits, 0, PositionState::Root)
    }

    /// 不关心 `units` 的开腿（固定 1.0）；口径唯一来源是 [`open_units_at`]（#600 Std MED-3）。
    fn open_at(
        table: &mut OpenTable<LedgerOpen>,
        id: ElementId,
        side: VoiceSide,
        entry_bar: usize,
        vertical: Vertical,
    ) {
        open_units_at(table, id, side, entry_bar, vertical, 1.0);
    }

    /// #596 HIGH-1 守卫用：显式给 `units` 的开腿（`open_at` 固定 1.0，测不出 N 倍双计）。
    fn open_units_at(
        table: &mut OpenTable<LedgerOpen>,
        id: ElementId,
        side: VoiceSide,
        entry_bar: usize,
        vertical: Vertical,
        units: f64,
    ) {
        table.open(id, |generation| LedgerOpen {
            entry_bar,
            entry_px: 100.0 + entry_bar as f64,
            entry_z: test_z(side),
            entry_stop_dist: None,
            entry_stop: None,
            entry_v: vertical,
            position_node_id: PositionNodeId {
                carrier: id,
                entry_certificate: None,
                side,
                generation,
            },
            opsem: OpsemEntrySnapshot::default(),
            units,
        });
    }

    fn settle_ctx<'a>(
        trades: &'a mut Vec<TypedTrade>,
        opsem: &'a mut Option<OpsemDump>,
        tw: &'a mut TwLedgerThread,
    ) -> SettlementContext<'a> {
        SettlementContext::new(ExitSnapshot::new(99, 123.0, &ZExt::NONE), trades, opsem, tw)
    }

    /// #571 C1：同 carrier 同 bar 双向开，两 generation 各落一条真实 TypedTrade。
    #[test]
    fn same_bar_opposite_sides_both_survive_until_close() {
        let mut table = OpenTable::new();
        open_at(
            &mut table,
            carrier(7),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
        );
        open_at(
            &mut table,
            carrier(7),
            VoiceSide::Short,
            10,
            Vertical::Ambient,
        );

        let mut trades = Vec::new();
        let mut opsem = None;
        let mut tw = TwLedgerThread::new(1_000.0, RiskPolicy::baseline());
        table.settle_reverse(
            carrier(7),
            1,
            &mut settle_ctx(&mut trades, &mut opsem, &mut tw),
        );

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].position_node_id.side, VoiceSide::Long);
        assert_eq!(trades[1].position_node_id.side, VoiceSide::Short);
    }

    /// #571：跨 bar 同向重开不能被方向键或 carrier 键覆盖。
    #[test]
    fn cross_bar_same_side_reopen_keeps_both_generations() {
        let mut table = OpenTable::new();
        open_at(
            &mut table,
            carrier(8),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
        );
        open_at(
            &mut table,
            carrier(8),
            VoiceSide::Long,
            20,
            Vertical::Ambient,
        );

        let mut trades = Vec::new();
        let mut opsem = None;
        let mut tw = TwLedgerThread::new(1_000.0, RiskPolicy::baseline());
        table.settle_silent(
            carrier(8),
            &mut settle_ctx(&mut trades, &mut opsem, &mut tw),
        );

        assert_eq!(
            trades.iter().map(|t| t.entry_bar).collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert_eq!(
            trades
                .iter()
                .map(|t| t.position_node_id.generation)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    /// #571：跨 bar 异向重开也保留两个 generation。
    #[test]
    fn cross_bar_opposite_side_reopen_keeps_both_generations() {
        let mut table = OpenTable::new();
        open_at(
            &mut table,
            carrier(9),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
        );
        open_at(
            &mut table,
            carrier(9),
            VoiceSide::Short,
            20,
            Vertical::Ambient,
        );

        let mut trades = Vec::new();
        let mut opsem = None;
        let mut tw = TwLedgerThread::new(1_000.0, RiskPolicy::baseline());
        table.settle_overlay(
            carrier(9),
            &mut settle_ctx(&mut trades, &mut opsem, &mut tw),
        );

        assert_eq!(trades.len(), 2);
        assert!(trades
            .iter()
            .all(|t| t.exit_type == ExitType::CloseShortDiff));
    }

    /// #571：RiskExit 是 carrier drain，全部 live generation 各落 RiskExit，TW 计数对称清零。
    #[test]
    fn risk_exit_drains_every_live_generation() {
        let mut table = OpenTable::new();
        let mut tw = TwLedgerThread::new(1_000.0, RiskPolicy::baseline());
        for entry_bar in [10, 20, 30] {
            open_at(
                &mut table,
                carrier(10),
                VoiceSide::Short,
                entry_bar,
                Vertical::ShortDiff,
            );
            tw.open_share_leg();
        }
        assert_eq!(tw.tw.open_legacy_legs, 3);

        let mut trades = Vec::new();
        let mut opsem = None;
        table.settle_risk(
            carrier(10),
            &mut settle_ctx(&mut trades, &mut opsem, &mut tw),
        );

        assert_eq!(trades.len(), 3);
        assert!(trades.iter().all(|t| t.exit_type == ExitType::RiskExit));
        assert_eq!(tw.tw.open_legacy_legs, 0);
        assert_eq!(table.len(), 0);
    }

    /// #571：窗末多 generation 全部 Hold，排序含 generation，结果可复现。
    #[test]
    fn window_end_holds_every_generation_in_deterministic_order() {
        let mut table = OpenTable::new();
        open_at(
            &mut table,
            carrier(12),
            VoiceSide::Long,
            20,
            Vertical::Ambient,
        );
        open_at(
            &mut table,
            carrier(11),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
        );
        open_at(
            &mut table,
            carrier(11),
            VoiceSide::Short,
            10,
            Vertical::Ambient,
        );

        let mut trades = Vec::new();
        let mut opsem = None;
        let mut tw = TwLedgerThread::new(1_000.0, RiskPolicy::baseline());
        table.settle_all_hold(&mut settle_ctx(&mut trades, &mut opsem, &mut tw));

        let keys = trades
            .iter()
            .map(|t| {
                (
                    t.entry_bar,
                    t.voice_id.ordinal,
                    t.position_node_id.generation,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, vec![(10, 11, 0), (10, 11, 1), (20, 12, 0)]);
        assert!(trades.iter().all(|t| t.exit_type == ExitType::Hold));
        assert_eq!(table.len(), 0);
    }

    /// #596 HIGH-1 双计守卫：同 carrier 同 bar 双向开（#570 §2 bar3047 Arm0 实证形态），
    /// 两实例的 `units` 都是**同一份** `SepLeg::q_units` 拷贝（sep_legs 按 carrier 唯一，
    /// `coverage/step.rs` #512 守卫）⟹ 父腿快照必须是单份 40，不是 2×40。
    #[test]
    fn parent_units_is_single_carrier_snapshot_not_sum() {
        let mut table = OpenTable::new();
        open_units_at(
            &mut table,
            carrier(13),
            VoiceSide::Long,
            3047,
            Vertical::Ambient,
            40.0,
        );
        open_units_at(
            &mut table,
            carrier(13),
            VoiceSide::Short,
            3047,
            Vertical::Ambient,
            40.0,
        );

        assert_eq!(table.parent_units(carrier(13)), Some(40));
    }

    /// #596 HIGH-1 / #600 Std MED-2：跨 bar 重开时单份取 **generation 最大者**。数值方向刻意
    /// 反向（gen0 的 units=55 > gen1 的 units=40），使三条可能的读法彼此可分：取 max(generation)
    /// 得 40（本判据），取 max(units) 得 55，取"迭代序末位"得 40 但依赖 `BTreeSet` 升序这一隐式
    /// 契约——实现侧 `max_by_key(generation)` 使第三条不再被依赖。
    #[test]
    fn parent_units_takes_latest_generation_snapshot() {
        let mut table = OpenTable::new();
        open_units_at(
            &mut table,
            carrier(14),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
            55.0,
        );
        open_units_at(
            &mut table,
            carrier(14),
            VoiceSide::Long,
            20,
            Vertical::Ambient,
            40.0,
        );

        assert_eq!(table.parent_units(carrier(14)), Some(40));
    }

    /// #600 Spec MED-1：父腿快照跟随**最新实例**的 vertical，与 #571 前单槽世界逐点等价——
    /// supersede 覆盖下 `get(carrier)` 拿到的就是最后一次开腿，其 `entry_v==ShortDiff` ⟹ None。
    /// 反序（先滤 ShortDiff、后取最大）不等价：单槽世界里被 ShortDiff 覆盖掉的旧 Ambient 快照
    /// 已不存在，反序会让它复活冒充父腿。
    #[test]
    fn parent_units_follows_latest_instance_vertical() {
        // 最新 = ShortDiff ⟹ None（旧 Ambient 快照不复活）
        let mut latest_short = OpenTable::new();
        open_units_at(
            &mut latest_short,
            carrier(15),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
            40.0,
        );
        open_units_at(
            &mut latest_short,
            carrier(15),
            VoiceSide::Short,
            20,
            Vertical::ShortDiff,
            9.0,
        );
        assert_eq!(latest_short.parent_units(carrier(15)), None);

        // 最新 = 非 ShortDiff ⟹ 取其 units（更早的 ShortDiff 实例不影响）
        let mut latest_ambient = OpenTable::new();
        open_units_at(
            &mut latest_ambient,
            carrier(17),
            VoiceSide::Short,
            10,
            Vertical::ShortDiff,
            9.0,
        );
        open_units_at(
            &mut latest_ambient,
            carrier(17),
            VoiceSide::Long,
            20,
            Vertical::Ambient,
            40.0,
        );
        assert_eq!(latest_ambient.parent_units(carrier(17)), Some(40));

        // 全为 ShortDiff ⟹ 无父腿
        let mut only_short = OpenTable::new();
        open_units_at(
            &mut only_short,
            carrier(16),
            VoiceSide::Short,
            10,
            Vertical::ShortDiff,
            9.0,
        );
        assert_eq!(only_short.parent_units(carrier(16)), None);
    }

    /// #600 LOW-2：非有限 units 的 fail-loud 覆盖（`SepLeg::q_units` 出 NaN = 上游 sizing 已坏，
    /// 静默 `as u64`（NaN→0）会把坏值伪装成"零单位不冒充父腿"）。
    #[test]
    #[should_panic(expected = "#571 PanDiv 父快照非法 units")]
    fn parent_units_fails_loud_on_non_finite_units() {
        let mut table = OpenTable::new();
        open_units_at(
            &mut table,
            carrier(18),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
            f64::NAN,
        );
        let _ = table.parent_units(carrier(18));
    }

    /// #600 LOW-2：超 `u64` 值域的 fail-loud 覆盖（`as u64` 会饱和到 `u64::MAX` 而非报错）。
    #[test]
    #[should_panic(expected = "#571 PanDiv 父快照 units 溢出")]
    fn parent_units_fails_loud_on_u64_overflow() {
        let mut table = OpenTable::new();
        open_units_at(
            &mut table,
            carrier(19),
            VoiceSide::Long,
            10,
            Vertical::Ambient,
            1.9e19,
        );
        let _ = table.parent_units(carrier(19));
    }
}
