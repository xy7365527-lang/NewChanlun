//! DC-E 生产适配：门后 PanDivCert → #81 OscillationBook，路由/生命周期零平行子腿账本。
//! #195 档A：OscillationBook 之下接 SplitLegLedger 数量不变量层（每 parent+lot 对一实例，
//! apply 双写 + 一致性断言，不一致即拒）；路由与 lot 生命周期仍单源在 OscillationBook。

use std::collections::{HashMap, HashSet};

use super::super::classifier::signal::PanDivCert;
use super::super::strategy::ledger::{SplitLegEvent, SplitLegLedger};
use super::super::strategy::oscillation::{
    CenterOscillationCandidate, CenterOscillationConfig, ConsolidationDivergenceEvidence,
    OscillationAction, OscillationApplyResult, OscillationBook, OscillationCenterRef,
    OscillationEvidenceRef, OscillationId, OscillationParentLeg, OscillationRouteRecordReason,
    SizeKThetaProjection,
};
use super::super::strategy::voice::VoiceSide;
use super::super::types::Side;
use super::econ_positive::GatedPanDivCert;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PanDivCertIdentity {
    level: u32,
    source_index: usize,
    side: u8,
}

impl PanDivCertIdentity {
    fn new(level: u32, cert: &PanDivCert) -> Self {
        Self {
            level,
            source_index: cert.source_index,
            side: match cert.side {
                Side::Long => 0,
                Side::Short => 1,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct PanDivProductionStats {
    pub raw_first_seen: usize,
    pub gate_rejected: usize,
    pub candidate_emitted: usize,
    pub p10_recorded: usize,
    pub priority_suppressed: usize,
    pub applied: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreparedPanDiv {
    Candidate(CenterOscillationCandidate),
    Record(OscillationRouteRecordReason),
}

/// 仅保存 cert 首见集合、计数与 #81 的唯一子腿路由账本；lot/open/close 生命周期状态单源在
/// `book`，本结构不保存第二套路由状态。#195 档A：`split_ledgers` 是 `book` 之下的数量不变量
/// 镜像层（每 parent+lot 对一实例），不持路由/生命周期语义。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PanDivProductionState {
    seen: HashSet<PanDivCertIdentity>,
    book: OscillationBook,
    /// #195 档A 共存：每 parent+lot 对（OscillationId）映射一个 SplitLegLedger 实例
    /// （ShortDiffAlreadyOpen 禁止单实例多子腿 ⟹ 每-lot 一账本）。apply 双写 + 一致性断言，
    /// 不一致即拒（panic），不允许静默漂移。本层不产订单、不改 book 语义（净额出口不动）。
    split_ledgers: HashMap<OscillationId, SplitLegLedger>,
    /// 同一已消费 cert 的部分执行续单；身份与 lot 状态仍以 `book` 为唯一真值。
    pending: Vec<CenterOscillationCandidate>,
    stats: PanDivProductionStats,
}

impl PanDivProductionState {
    pub fn sync_live_parents(&mut self, parents: Vec<OscillationParentLeg>) {
        self.book.sync_live_parents(parents);
        // #195 档A bar 级同步点（runner.rs sync_live_parents 调用处）：父表刷新后全量对账。
        self.assert_split_ledger_consistent();
    }

    /// #195 档A 全量对账：OscillationBook↔SplitLegLedger——每 lot 恰一账本、每账本恰一 lot；
    /// 短差数量/方向与 lot.remaining_units/side 恒等。任一不符即拒（panic），不允许静默漂移。
    ///
    /// 父腿侧不在本对账内：父表是逐 bar 刷新的只读快照（id 跨 campaign 复用属既有语义）。
    /// open 双写以当前父表重建（Applied Open 经 book 校验 `target == 父数量 ∧ 方向 == 父翻转`，
    /// 已保证当前快照与 lot 锚定同值）；close 双写沿用账本自身锚定的父事实（见双写函数文档）。
    fn assert_split_ledger_consistent(&self) {
        for lot in self.book.lots() {
            let id = lot.oscillation_id();
            let ledger = self.split_ledgers.get(&id).unwrap_or_else(|| {
                panic!("#195 档A 对账即拒：lot {id:?} 无对应 SplitLegLedger（漏双写）")
            });
            let legs = ledger.split_legs();
            let ledger_qty = legs.short_diff.map_or(0, |leg| leg.qty);
            assert_eq!(
                ledger_qty,
                lot.remaining_units(),
                "#195 档A 对账即拒：lot {id:?} 账本短差数量 {ledger_qty} ≠ remaining {}",
                lot.remaining_units()
            );
            if let Some(short) = legs.short_diff {
                assert_eq!(
                    short.side,
                    lot.side(),
                    "#195 档A 对账即拒：lot {id:?} 短差方向账本 {:?} ≠ book {:?}",
                    short.side,
                    lot.side()
                );
            }
        }
        for id in self.split_ledgers.keys() {
            assert!(
                self.book.lot(*id).is_some(),
                "#195 档A 对账即拒：账本 {id:?} 无对应 lot（幽灵账本）"
            );
        }
    }

    /// #195 档A 双写（open）：短差数量 = 既有账本数量 + 本次成交（增量开；上溢即拒）。
    /// 经 SplitLegLedger 自有构造/事件闸重建——Applied Open 经 book 校验（target == 父数量、
    /// 方向 == 父翻转）已保证闸必过；随后断言账本读出与 lot 真值恒等（不一致即拒）。
    fn dual_write_split_ledger_open(&mut self, id: OscillationId, units: u64) {
        let lot = self
            .book
            .lot(id)
            .expect("#195 档A 双写即拒：OpenShortDiff Applied 后 lot 必在册");
        let parent = self
            .book
            .parent(lot.parent_leg_id())
            .expect("#195 档A 双写即拒：Applied Open ⟹ 父腿在只读父表");
        let prev_qty = self
            .split_ledgers
            .get(&id)
            .and_then(|ledger| ledger.split_legs().short_diff)
            .map_or(0, |leg| leg.qty);
        let next_qty = prev_qty
            .checked_add(units)
            .expect("#195 档A 双写即拒：短差数量 u64 上溢");
        let next = SplitLegLedger::parent_only(parent.side(), parent.units())
            .and_then(|ledger| {
                ledger.apply(SplitLegEvent::OpenShortDiff {
                    side: lot.side(),
                    qty: next_qty,
                })
            })
            .unwrap_or_else(|err| {
                panic!(
                    "#195 档A 双写被账本闸拒（open id={id:?} qty={next_qty} parent=({:?},{} unit)）：{err:?}",
                    parent.side(),
                    parent.units()
                )
            });
        self.split_ledgers.insert(id, next);
        assert_eq!(
            next_qty,
            lot.remaining_units(),
            "#195 档A 一致性断言：open 双写后账本短差 {next_qty} ≠ lot.remaining {}",
            lot.remaining_units()
        );
    }

    /// #195 档A 双写（close）：短差数量 = 既有账本数量 − 本次成交（增量平；下溢即拒）。
    /// 平完经 CloseShortDiff 事件清仓（父腿逐字段不动）；部分平以**账本自身锚定的父事实**
    /// 重建为「父镜像 + 剩余短差」（事件面只有 Open/Close 两事件，无部分修改——重建值
    /// = prev−units，沿用 prev 的父腿/额度锚定，闸可证必过）。
    fn dual_write_split_ledger_close(&mut self, id: OscillationId, units: u64) {
        let lot = self
            .book
            .lot(id)
            .expect("#195 档A 双写即拒：CloseShortDiff Applied 后 lot 必在册");
        let prev = *self.split_ledgers.get(&id).unwrap_or_else(|| {
            panic!("#195 档A 双写即拒（close id={id:?}）：lot 在册但账本无对应实例（漏双写）")
        });
        let prev_legs = prev.split_legs();
        let prev_short = prev_legs.short_diff.unwrap_or_else(|| {
            panic!("#195 档A 双写即拒（close id={id:?}）：CloseShortDiff 到达时账本短差不在册")
        });
        let remaining = prev_short.qty.checked_sub(units).unwrap_or_else(|| {
            panic!(
                "#195 档A 双写下溢即拒（close id={id:?}）：账本短差 {} < 本次成交 {units}",
                prev_short.qty
            )
        });
        let next = if remaining == 0 {
            prev.apply(SplitLegEvent::CloseShortDiff).unwrap_or_else(|err| {
                panic!("#195 档A 双写被账本闸拒（close id={id:?}）：{err:?}")
            })
        } else {
            SplitLegLedger::parent_only_with_quota(
                prev_legs.parent.side,
                prev_legs.parent.qty,
                prev.short_diff_quota(),
            )
            .and_then(|ledger| {
                ledger.apply(SplitLegEvent::OpenShortDiff {
                    side: prev_short.side,
                    qty: remaining,
                })
            })
            .unwrap_or_else(|err| {
                panic!("#195 档A 双写被账本闸拒（close id={id:?} qty={remaining}）：{err:?}")
            })
        };
        self.split_ledgers.insert(id, next);
        assert_eq!(
            remaining,
            lot.remaining_units(),
            "#195 档A 一致性断言：close 双写后账本短差 {remaining} ≠ lot.remaining {}",
            lot.remaining_units()
        );
    }

    /// 首见即终局：先记 seen，再评门。门闭后相同 cert 后续出现不会重试。
    pub fn observe_raw(&mut self, level: u32, cert: &PanDivCert) -> bool {
        let first = self.seen.insert(PanDivCertIdentity::new(level, cert));
        if first {
            self.stats.raw_first_seen += 1;
        }
        first
    }

    pub fn note_gate_rejected(&mut self) {
        self.stats.gate_rejected += 1;
        self.stats.p10_recorded += 1;
    }

    pub fn prepare(&mut self, gated: GatedPanDivCert) -> PreparedPanDiv {
        let cert = gated.cert();
        let signal_side = match cert.side {
            Side::Long => VoiceSide::Long,
            Side::Short => VoiceSide::Short,
        };
        let routed = self.book.route_gated_pan_div(
            gated.level(),
            signal_side,
            OscillationCenterRef::new(cert.center.start_index, cert.center.end_index),
            ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(cert.source_index, 0)),
        );
        match routed {
            Ok(candidate) => {
                self.stats.candidate_emitted += 1;
                PreparedPanDiv::Candidate(candidate)
            }
            Err(reason) => {
                self.stats.p10_recorded += 1;
                PreparedPanDiv::Record(reason)
            }
        }
    }

    /// P1-P4、真正 BSP 或既有标准订单任一成立，PanDiv 只保留协议证据，不取得订单槽。
    pub fn select_for_bar(
        &mut self,
        candidates: &[CenterOscillationCandidate],
        higher_priority: bool,
    ) -> Option<CenterOscillationCandidate> {
        if higher_priority {
            self.stats.priority_suppressed += candidates.len();
            return None;
        }
        let mut unique = candidates.to_vec();
        unique.sort_by_key(|candidate| {
            let priority = match candidate.intent() {
                super::super::strategy::oscillation::OscillationIntent::Close => 0u8, // P7
                super::super::strategy::oscillation::OscillationIntent::Open => 1u8,  // P9
            };
            (priority, candidate.stable_key())
        });
        unique.dedup_by_key(|candidate| candidate.oscillation_id());
        if unique.len() > 1 {
            self.stats.p10_recorded += unique.len() - 1;
        }
        unique.into_iter().next()
    }

    pub fn pending_candidates(&self) -> &[CenterOscillationCandidate] {
        &self.pending
    }

    pub fn apply(
        &mut self,
        candidate: CenterOscillationCandidate,
        config: CenterOscillationConfig,
        projection: SizeKThetaProjection,
    ) -> OscillationApplyResult {
        let result = self.book.apply(candidate, config, projection);
        // #195 档A：open/close 双写 SplitLegLedger（runner.rs 消费区经本方法到达）+ 一致性
        // 断言。仅 Applied 产账本事件；Recorded/Disabled 无 book 变更 ⟹ 无账本变更。
        if let OscillationApplyResult::Applied {
            action,
            oscillation_id,
            units,
        } = result
        {
            match action {
                OscillationAction::OpenShortDiff => {
                    self.dual_write_split_ledger_open(oscillation_id, units)
                }
                OscillationAction::CloseShortDiff => {
                    self.dual_write_split_ledger_close(oscillation_id, units)
                }
                OscillationAction::Record => {
                    unreachable!("#195 档A：Applied 不携 Record（record 走 Recorded 变体）")
                }
            }
        }
        match result {
            OscillationApplyResult::Applied { oscillation_id, .. } => {
                self.stats.applied += 1;
                let incomplete =
                    self.book
                        .lot(oscillation_id)
                        .is_some_and(|lot| match candidate.intent() {
                            super::super::strategy::oscillation::OscillationIntent::Open => {
                                lot.open_residual_units() > 0 && lot.remaining_units() > 0
                            }
                            super::super::strategy::oscillation::OscillationIntent::Close => {
                                lot.remaining_units() > 0
                            }
                        });
                self.pending
                    .retain(|pending| pending.oscillation_id() != oscillation_id);
                if incomplete {
                    self.pending.push(candidate);
                }
            }
            OscillationApplyResult::Recorded { .. } => self.stats.p10_recorded += 1,
            OscillationApplyResult::Disabled => {}
        }
        result
    }

    pub fn signed_live_child_units(&self) -> i64 {
        self.book.signed_live_child_units()
    }

    #[cfg(test)]
    fn stats(&self) -> PanDivProductionStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::backtest::mu_estimator::{MuClass, PositionState};
    use crate::theta_v0::classifier::recursive_tower::ElementId;
    use crate::theta_v0::strategy::ledger::{SplitLegEvent, SplitLegLedger};
    use crate::theta_v0::strategy::oscillation::{
        BoundarySide, OscillationAction, OscillationCandidateReason, OscillationEvidence,
        OscillationId,
    };
    use crate::theta_v0::types::{BspBits, Center};

    fn center() -> Center {
        Center {
            zd: 90,
            zg: 110,
            dd: 80,
            gg: 120,
            start_index: 10,
            end_index: 20,
        }
    }

    fn cert(source_index: usize, side: Side) -> PanDivCert {
        PanDivCert {
            source_index,
            side,
            center: center(),
            seg_a: (21, 25),
            seg_c: (30, source_index),
        }
    }

    fn parent(side: VoiceSide) -> OscillationParentLeg {
        OscillationParentLeg::new(
            ElementId {
                level: 1,
                ordinal: 7,
            },
            1,
            side,
            100,
        )
        .unwrap()
    }

    fn evidence(source_index: usize) -> ConsolidationDivergenceEvidence {
        ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(source_index, 0))
    }

    #[test]
    fn pan_div_cert_identity_stable_and_consumed_once() {
        let mut state = PanDivProductionState::default();
        let c = cert(39, Side::Short);
        assert!(state.observe_raw(1, &c));
        assert!(!state.observe_raw(1, &c));
        assert_eq!(state.stats().raw_first_seen, 1);
    }

    #[test]
    fn pan_div_candidate_keeps_standard_bsp_bits_zero() {
        let p = parent(VoiceSide::Long);
        let candidate = OscillationBook::with_parents(vec![p])
            .route_gated_pan_div(
                1,
                VoiceSide::Short,
                OscillationCenterRef::new(10, 20),
                evidence(39),
            )
            .unwrap();
        assert!(matches!(
            candidate.reason(),
            OscillationCandidateReason::CenterOscillation(
                OscillationEvidence::ConsolidationDivergence(_)
            )
        ));
        assert_eq!(BspBits::default().class_index(), 0);
    }

    #[test]
    fn pan_div_class_one_like_never_enters_standard_b1_bucket() {
        let z = MuClass::from_certificate(
            1,
            -1,
            BspBits::default(),
            1,
            PositionState::Child,
        );
        assert_eq!(z.bsp_class(), 0, "PanDiv 只能走独立原因，标准 B1 桶必须为零");
    }

    #[test]
    fn pan_div_first_seen_terminal_matches_all_signal_channels() {
        let mut state = PanDivProductionState::default();
        let rejected_at_first_seen = cert(39, Side::Short);
        assert!(state.observe_raw(1, &rejected_at_first_seen));
        state.note_gate_rejected();
        assert!(!state.observe_raw(1, &rejected_at_first_seen));
        let mut later_recomposed = rejected_at_first_seen;
        later_recomposed.center.end_index += 1;
        assert!(
            !state.observe_raw(1, &later_recomposed),
            "身份必须与既有统计通道同为 (level, source_index, side)，不得因后续中枢重组重试"
        );
        assert_eq!(
            state.stats().gate_rejected,
            1,
            "后续 bar 不得重试首见已闭门证书"
        );
        assert_eq!(state.stats().p10_recorded, 1, "门闭必须落 P10 Record");
    }

    #[test]
    fn pan_div_opens_shortdiff_only_with_live_parent() {
        let mut book = OscillationBook::with_parents(vec![parent(VoiceSide::Long)]);
        let center = OscillationCenterRef::new(10, 20);
        let open = book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        assert_eq!(open.parent_leg_id(), parent(VoiceSide::Long).id());
        assert_eq!(open.oscillation_id().center(), center);
        let applied = book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert!(matches!(
            applied,
            OscillationApplyResult::Applied {
                action: OscillationAction::OpenShortDiff,
                ..
            }
        ));
        assert_eq!(book.signed_live_child_units(), -100);
    }

    #[test]
    fn pan_div_close_targets_exact_oscillation_id() {
        let p = parent(VoiceSide::Long);
        let mut book = OscillationBook::with_parents(vec![p]);
        let center = OscillationCenterRef::new(10, 20);
        let open = book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let id = open.oscillation_id();
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        let close = book
            .route_gated_pan_div(1, VoiceSide::Long, center, evidence(49))
            .unwrap();
        assert_eq!(close.oscillation_id(), id);
        assert_eq!(close.boundary_side(), BoundarySide::Below);
        let out = book.apply(
            close,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert!(
            matches!(out, OscillationApplyResult::Applied { action: OscillationAction::CloseShortDiff, oscillation_id, .. } if oscillation_id == id)
        );
        assert!(book.units_conserved());
        assert_eq!(book.signed_live_child_units(), 0);
    }

    #[test]
    fn pan_div_partial_open_recovery_closes_live_units_same_identity() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let open = state
            .book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let id = open.oscillation_id();
        state.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 40,
            },
        );
        assert_eq!(state.pending_candidates(), &[open]);
        let close = state
            .book
            .route_gated_pan_div(1, VoiceSide::Long, center, evidence(49))
            .expect("恢复父方向必须能关闭已成交 40，即使目标 100 尚有未成交残量");
        assert_eq!(close.oscillation_id(), id);
        let selected = state
            .select_for_bar(&[open, close], false)
            .expect("P7 close 必须压过同 identity 的 P9 residual");
        assert_eq!(
            selected.intent(),
            crate::theta_v0::strategy::oscillation::OscillationIntent::Close
        );
        state.apply(
            selected,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 40,
                k_theta_units: 40,
            },
        );
        assert!(state.pending_candidates().is_empty());
        assert_eq!(state.book.total_opened_units(), 40);
        assert_eq!(state.book.total_closed_units(), 40);
        assert!(state.book.units_conserved());
    }

    #[test]
    fn pan_div_round_trip_target_units_conserved() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut book = OscillationBook::with_parents(vec![p]);
        let open = book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        let close = book
            .route_gated_pan_div(1, VoiceSide::Long, center, evidence(49))
            .unwrap();
        book.apply(
            close,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert_eq!(book.total_opened_units(), book.total_closed_units());
        assert_eq!(book.live_child_units(), 0);
        assert!(book.units_conserved());
    }

    #[test]
    fn pan_div_missing_or_ambiguous_parent_is_p10_record() {
        let center = OscillationCenterRef::new(10, 20);
        let empty = OscillationBook::default();
        assert_eq!(
            empty.route_gated_pan_div(1, VoiceSide::Short, center, evidence(39)),
            Err(OscillationRouteRecordReason::MissingLiveParent)
        );
        let p1 = parent(VoiceSide::Long);
        let p2 = OscillationParentLeg::new(
            ElementId {
                level: 1,
                ordinal: 8,
            },
            1,
            VoiceSide::Long,
            100,
        )
        .unwrap();
        let ambiguous = OscillationBook::with_parents(vec![p1, p2]);
        assert_eq!(
            ambiguous.route_gated_pan_div(1, VoiceSide::Short, center, evidence(39)),
            Err(OscillationRouteRecordReason::AmbiguousLiveParent)
        );
    }

    #[test]
    fn pan_div_same_bar_deduplicates_to_one_identity() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let c = CenterOscillationCandidate::open(
            center,
            1,
            p,
            OscillationId::new(p.id(), center, 0),
            BoundarySide::Above,
            crate::theta_v0::strategy::oscillation::OscillationEvidence::ConsolidationDivergence(
                evidence(39),
            ),
        )
        .unwrap();
        let mut state = PanDivProductionState::default();
        assert_eq!(state.select_for_bar(&[c, c], false), Some(c));
    }

    #[test]
    fn pan_div_and_bsp_same_bar_emit_at_most_one_order_per_identity() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let c = CenterOscillationCandidate::open(
            center,
            1,
            p,
            OscillationId::new(p.id(), center, 0),
            BoundarySide::Above,
            crate::theta_v0::strategy::oscillation::OscillationEvidence::ConsolidationDivergence(
                evidence(39),
            ),
        )
        .unwrap();
        let mut state = PanDivProductionState::default();
        assert_eq!(state.select_for_bar(&[c], true), None);
        assert_eq!(state.stats().priority_suppressed, 1);
    }

    #[test]
    fn p1_p4_mask_pan_div_candidate() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let c = CenterOscillationCandidate::open(
            center,
            1,
            p,
            OscillationId::new(p.id(), center, 0),
            BoundarySide::Above,
            crate::theta_v0::strategy::oscillation::OscillationEvidence::ConsolidationDivergence(
                evidence(39),
            ),
        )
        .unwrap();
        let mut state = PanDivProductionState::default();
        assert_eq!(state.select_for_bar(&[c], true), None);
        assert_eq!(state.stats().applied, 0);
    }

    #[test]
    fn pan_div_unexecutable_becomes_record_with_reason() {
        let center = OscillationCenterRef::new(10, 20);
        let reason = OscillationBook::default()
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap_err();
        assert_eq!(reason, OscillationRouteRecordReason::MissingLiveParent);
    }

    #[test]
    fn pan_div_risk_projection_never_exceeds_target() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let c = OscillationBook::with_parents(vec![p])
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let out = state.apply(
            c,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 70,
                k_theta_units: 40,
            },
        );
        assert!(matches!(
            out,
            OscillationApplyResult::Applied { units: 40, .. }
        ));
        assert!(state.book.units_conserved());
        assert!(state.book.total_opened_units() <= c.target_units());
    }

    #[test]
    fn pan_div_p7_recovery_uses_composed_position_capacity() {
        use crate::theta_v0::strategy::coverage::KThetaRiskGate;

        let standard_parent_target = 100.0;
        let live_shortdiff = -40.0;
        let composed_anchor = standard_parent_target + live_shortdiff;
        assert_eq!(
            KThetaRiskGate::open().delta_capacity_units(
                100.0,
                composed_anchor,
                VoiceSide::Long,
            ),
            40,
            "父目标已在 cap 时，P7 仍必须能从组合净目标 60 回补到 100"
        );
    }

    #[test]
    fn disabled_pan_div_production_keeps_book_bit_exact() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let c = OscillationBook::with_parents(vec![p])
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let before = state.book.clone();
        assert_eq!(
            state.apply(
                c,
                CenterOscillationConfig::default(),
                SizeKThetaProjection {
                    size_theta_units: 100,
                    k_theta_units: 100
                }
            ),
            OscillationApplyResult::Disabled
        );
        assert_eq!(state.book, before);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  #195 档A：SplitLegLedger 共存接线（双写 + 一致性断言）
    // ──────────────────────────────────────────────────────────────────────

    /// 双写后账本读出必须恒等于 OscillationBook lot 真值（数量不变量层镜像）。
    fn assert_short_qty(state: &PanDivProductionState, id: OscillationId, expected: u64) {
        let legs = state
            .split_ledgers
            .get(&id)
            .expect("lot 必有对应 SplitLegLedger（双写）")
            .split_legs();
        let qty = legs.short_diff.map(|p| p.qty).unwrap_or(0);
        assert_eq!(qty, expected, "账本短差数量");
        assert_eq!(
            qty,
            state.book.lot(id).unwrap().remaining_units(),
            "账本读出必须恒等于 OscillationBook lot.remaining_units"
        );
    }

    /// #195 验收（双写一致性）：open/close apply 双写 SplitLegLedger——每 parent+lot 对一实例；
    /// 父腿镜像（方向/数量/额度），短差数量与 lot.remaining 恒等；关闭只清短差、不触碰父腿。
    #[test]
    fn pan_div_dual_write_open_close_mirrors_split_leg_ledger() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);

        let open = state
            .book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let id = open.oscillation_id();
        let out = state.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert!(matches!(
            out,
            OscillationApplyResult::Applied {
                action: OscillationAction::OpenShortDiff,
                ..
            }
        ));
        let ledger = state
            .split_ledgers
            .get(&id)
            .expect("OpenShortDiff Applied ⟹ SplitLegLedger 已双写");
        let legs = ledger.split_legs();
        assert_eq!(legs.parent.side, VoiceSide::Long);
        assert_eq!(legs.parent.qty, 100);
        let short = legs.short_diff.expect("短差腿在册");
        assert_eq!(short.side, VoiceSide::Short);
        assert_eq!(short.qty, 100);
        assert_eq!(ledger.short_diff_quota(), 100, "毛暴露额度 = 父腿数量（#135 T6 默认）");
        assert_short_qty(&state, id, 100);

        let close = state
            .book
            .route_gated_pan_div(1, VoiceSide::Long, center, evidence(49))
            .unwrap();
        let out = state.apply(
            close,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert!(matches!(
            out,
            OscillationApplyResult::Applied {
                action: OscillationAction::CloseShortDiff,
                ..
            }
        ));
        let legs = state.split_ledgers.get(&id).unwrap().split_legs();
        assert!(legs.short_diff.is_none(), "CloseShortDiff Applied ⟹ 短差腿清仓");
        assert_eq!(legs.parent.side, VoiceSide::Long);
        assert_eq!(legs.parent.qty, 100, "关闭短差不得触碰父腿（账本类型面保证）");
        assert!(state.book.units_conserved());
    }

    /// #195（增量双写）：部分成交序列（增量开 40+40、增量平 30+50）下，
    /// 账本短差数量与 lot.remaining_units 逐步恒等。
    #[test]
    fn pan_div_dual_write_partial_fills_track_remaining_units() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let open = state
            .book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let id = open.oscillation_id();
        let cfg = CenterOscillationConfig { enabled: true };

        state.apply(
            open,
            cfg,
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 40,
            },
        );
        assert_short_qty(&state, id, 40);
        state.apply(
            open,
            cfg,
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 40,
            },
        );
        assert_short_qty(&state, id, 80);

        let close = state
            .book
            .route_gated_pan_div(1, VoiceSide::Long, center, evidence(49))
            .unwrap();
        state.apply(
            close,
            cfg,
            SizeKThetaProjection {
                size_theta_units: 80,
                k_theta_units: 30,
            },
        );
        assert_short_qty(&state, id, 50);
        state.apply(
            close,
            cfg,
            SizeKThetaProjection {
                size_theta_units: 50,
                k_theta_units: 50,
            },
        );
        let legs = state.split_ledgers.get(&id).unwrap().split_legs();
        assert!(legs.short_diff.is_none(), "平完 ⟹ 短差腿清仓");
        assert_eq!(state.book.lot(id).unwrap().remaining_units(), 0);
        assert!(state.book.units_conserved());
    }

    /// #195 验收（不一致即拒）：book 被绕过双写直接推进（漏账漂移）时，
    /// bar 级同步对账必须拒（panic），不允许静默漂移。
    #[test]
    #[should_panic(expected = "#195")]
    fn pan_div_sync_rejects_book_ledger_drift() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let open = state
            .book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        // 绕过 state.apply（无双写）直接推进 book ⟹ book/账本漂移。
        let out = state.book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 100,
            },
        );
        assert!(matches!(out, OscillationApplyResult::Applied { .. }));
        state.sync_live_parents(vec![p]); // bar 级同步对账：lot 无账本 ⟹ 拒
    }

    /// #195 验收（不一致即拒）：账本读出与 book 数量不符（错账/幽灵账本）同样即拒。
    #[test]
    #[should_panic(expected = "#195")]
    fn pan_div_sync_rejects_mismatched_ledger_qty() {
        let p = parent(VoiceSide::Long);
        let center = OscillationCenterRef::new(10, 20);
        let mut state = PanDivProductionState::default();
        state.sync_live_parents(vec![p]);
        let open = state
            .book
            .route_gated_pan_div(1, VoiceSide::Short, center, evidence(39))
            .unwrap();
        let id = open.oscillation_id();
        state.apply(
            open,
            CenterOscillationConfig { enabled: true },
            SizeKThetaProjection {
                size_theta_units: 100,
                k_theta_units: 40,
            },
        );
        // 篡改账本：构造数量不符的账本塞回（模拟漂移）。
        let forged = SplitLegLedger::parent_only(VoiceSide::Long, 100)
            .unwrap()
            .apply(SplitLegEvent::OpenShortDiff {
                side: VoiceSide::Short,
                qty: 41,
            })
            .unwrap();
        state.split_ledgers.insert(id, forged);
        state.sync_live_parents(vec![p]); // 对账：账本 41 ≠ lot.remaining 40 ⟹ 拒
    }
}
