//! DC-E 触发链生产适配：门后 PanDivCert → [`PanDivTrigger`]（#282 保留的触发链）。
//!
//! #282（#280 裁定，ADR 0001 修正案一 修1 废止 S6 开空腿形态）两刀已落：
//! - #195 档A SplitLegLedger 数量不变量镜像层（每 parent+lot 一实例、apply 双写 +
//!   一致性断言）删除；
//! - #81 OscillationBook 账面形态（路由进 candidate、lots 账本、apply、P10 Record、
//!   优先级/select/pending 续单、净额投影出口）删除。
//!
//! **保留（#274 狭义短差实装原料）**：cert 首见集合（同证书终局不重试）+ Nest/XZD
//! 生产门消费（econ_positive 单源）+ 触发事件产出（盘背 PanDiv + 本级中枢上下沿）。
//! 触发经 fill.rs 门内段上协议轨（ProtocolEventSet），本模块不持任何账本/订单语义。

use std::collections::HashSet;

use super::super::classifier::signal::PanDivCert;
use super::super::strategy::oscillation::{
    ConsolidationDivergenceEvidence, OscillationCenterRef, OscillationEvidenceRef, PanDivTrigger,
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
    pub trigger_emitted: usize,
}

/// 触发链生产状态：cert 首见集合 + 门/触发计数。不持路由状态、不持账本（账面形态
/// 已随 #282 删除）；触发事件是纯值，产出即上交（fill.rs 门内段 → 协议轨）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PanDivProductionState {
    seen: HashSet<PanDivCertIdentity>,
    stats: PanDivProductionStats,
}

impl PanDivProductionState {
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
    }

    /// 门后证书 → 触发事件（盘背 PanDiv + 本级中枢上下沿映射，oscillation.rs 单源构造点）。
    /// `PanDivCert.side ∈ {Long, Short}`（types::Side 无 Flat）⟹ 触发构造不可失败。
    pub fn prepare(&mut self, gated: GatedPanDivCert) -> PanDivTrigger {
        let cert = gated.cert();
        let signal_side = match cert.side {
            Side::Long => VoiceSide::Long,
            Side::Short => VoiceSide::Short,
        };
        self.stats.trigger_emitted += 1;
        PanDivTrigger::from_gated_pan_div(
            gated.level(),
            signal_side,
            OscillationCenterRef::new(cert.center.start_index, cert.center.end_index),
            ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(cert.source_index, 0)),
        )
        .expect("PanDivCert side ∈ {Long,Short} ⟹ 触发构造不可失败（Flat 才拒绝）")
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
    use crate::theta_v0::strategy::oscillation::BoundarySide;
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

    #[test]
    fn pan_div_cert_identity_stable_and_consumed_once() {
        let mut state = PanDivProductionState::default();
        let c = cert(39, Side::Short);
        assert!(state.observe_raw(1, &c));
        assert!(!state.observe_raw(1, &c));
        assert_eq!(state.stats().raw_first_seen, 1);
    }

    /// 触发链产出物核验：级别/信号方向/中枢身份/边界侧/盘背证据五要素齐备且同口径
    /// （Short 信号 = 上沿），不携账面语义（无父数量/父翻转/开平仓意向——类型面保证）。
    #[test]
    fn pan_div_trigger_carries_only_trigger_facts() {
        let mut state = PanDivProductionState::default();
        let c = cert(39, Side::Short);
        assert!(state.observe_raw(1, &c));
        let gated = GatedPanDivCert::gated_for_test(1, c);
        let trigger = state.prepare(gated);
        assert_eq!(trigger.level(), 1);
        assert_eq!(trigger.signal_side(), VoiceSide::Short);
        assert_eq!(trigger.boundary_side(), BoundarySide::Above);
        assert_eq!(
            trigger.center(),
            OscillationCenterRef::new(c.center.start_index, c.center.end_index)
        );
        assert_eq!(trigger.evidence().reference().source_index(), 39);
        assert_eq!(state.stats().trigger_emitted, 1);
    }

    #[test]
    fn pan_div_trigger_keeps_standard_bsp_bits_zero() {
        // PanDiv 只能走独立原因（CenterOscillation），标准 B1/B2/B3 bits 必须为零。
        assert_eq!(BspBits::default().class_index(), 0);
    }

    #[test]
    fn pan_div_class_one_like_never_enters_standard_b1_bucket() {
        let z = MuClass::from_certificate(1, -1, BspBits::default(), 1, PositionState::Child);
        assert_eq!(
            z.bsp_class(),
            0,
            "PanDiv 只能走独立原因，标准 B1 桶必须为零"
        );
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
    }

    /// KΘ 组合净额容量判据（风险门单源数学）：父目标已在 cap 时，从组合净目标 60
    /// 回补到 100 仍有 40 容量。#282 注：原消费点（门内 P9 经济减仓/P7 回补的净额叠加
    /// 出口）随账面形态删除；本断言保留为门数学回归锁（`delta_capacity_units` 未删）。
    #[test]
    fn delta_capacity_units_composed_position_capacity() {
        use crate::theta_v0::strategy::coverage::KThetaRiskGate;

        let standard_parent_target = 100.0;
        let live_reverse_open = -40.0;
        let composed_anchor = standard_parent_target + live_reverse_open;
        assert_eq!(
            KThetaRiskGate::open().delta_capacity_units(100.0, composed_anchor, VoiceSide::Long,),
            40,
            "父目标已在 cap 时，仍必须能从组合净目标 60 回补到 100"
        );
    }
}
