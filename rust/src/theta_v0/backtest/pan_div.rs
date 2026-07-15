//! DC-E 生产适配：门后 PanDivCert → #81 OscillationBook，零平行子腿账本。

use std::collections::HashSet;

use super::super::classifier::signal::PanDivCert;
use super::super::strategy::oscillation::{
    CenterOscillationCandidate, CenterOscillationConfig, ConsolidationDivergenceEvidence,
    OscillationApplyResult, OscillationBook, OscillationCenterRef, OscillationEvidenceRef,
    OscillationParentLeg, OscillationRouteRecordReason, SizeKThetaProjection,
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

/// 仅保存 cert 首见集合、计数与 #81 的唯一子腿账本；不保存第二套 lot/open/close 状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PanDivProductionState {
    seen: HashSet<PanDivCertIdentity>,
    book: OscillationBook,
    /// 同一已消费 cert 的部分执行续单；身份与 lot 状态仍以 `book` 为唯一真值。
    pending: Vec<CenterOscillationCandidate>,
    stats: PanDivProductionStats,
}

impl PanDivProductionState {
    pub fn sync_live_parents(&mut self, parents: Vec<OscillationParentLeg>) {
        self.book.sync_live_parents(parents);
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
}
