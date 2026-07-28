//! #571 typed ledger 在飞表：以 `(carrier, generation)` 标识 campaign 实例。
//!
//! 结构层关闭事件只携 carrier；本模块按 generation 升序 drain 该 carrier 的全部 live
//! campaign，各自结算为一条 [`TypedTrade`]。新开不会把旧 generation 假结算为 supersede。

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

    pub(super) fn parent_units(&self, carrier: ElementId) -> Option<u64> {
        let mut found = false;
        let mut total = 0.0;
        for open in self
            .live_for_carrier(carrier)
            .filter(|open| open.entry_v != Vertical::ShortDiff)
        {
            assert!(
                open.units.is_finite() && open.units >= 0.0,
                "#571 PanDiv 父快照非法 units：carrier={carrier:?}, units={}",
                open.units
            );
            found = true;
            total += open.units;
        }
        if !found {
            return None;
        }
        assert!(
            total.is_finite() && total.round() <= u64::MAX as f64,
            "#571 PanDiv 父快照 units 溢出：carrier={carrier:?}, total={total}"
        );
        Some(total.round() as u64)
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

    fn open_at(
        table: &mut OpenTable<LedgerOpen>,
        id: ElementId,
        side: VoiceSide,
        entry_bar: usize,
        vertical: Vertical,
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
            units: 1.0,
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
}
