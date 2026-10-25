//! L3 探针具名资金 profile（#1605）。
//!
//! 只有 `max(首可交易价 × 1000, 1e6)` 这一族探针使用本记录。`replay_dump` 等
//! `首价 × 10000` 是另一回测口径，不在没有新裁定的情况下混改。

use serde_json::Value;

use super::super::config::ThetaConfig;
use super::super::policy_records::{
    invalid, parse_header, read_f64, read_str, PolicyError, PolicyRef,
    ACTIVE_L3_PROBE_PROFILE_JSON, ACTIVE_NO_FLOOR_PROBE_PROFILE_JSON, L3_PROBE_PROFILE_ID,
    NO_FLOOR_PROBE_PROFILE_ID,
};
use super::data::Dataset;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeProfile {
    pub record: PolicyRef,
    pub nav_floor: f64,
    pub first_px_multiplier: f64,
    pub missing_first_px_fallback: f64,
}

impl ProbeProfile {
    pub fn validate(&self) -> Result<(), PolicyError> {
        for (name, value) in [
            ("nav_floor", self.nav_floor),
            ("first_px_multiplier", self.first_px_multiplier),
            ("missing_first_px_fallback", self.missing_first_px_fallback),
        ] {
            if !value.is_finite() || value < 0.0 || (name != "nav_floor" && value == 0.0) {
                return Err(invalid(
                    self.record.id,
                    name,
                    "须为有限正数；nav_floor 允许 0",
                ));
            }
        }
        Ok(())
    }

    /// 探针真实 NAV 公式；不是未接线的展示函数。
    pub fn nav_for(&self, win: &Dataset, config: &ThetaConfig) -> f64 {
        self.nav_for_bars(&win.bars, config)
    }

    /// 训练 typed ledger 只有 bar 切片，与 Dataset 窗口共用同一记录公式。
    pub fn nav_for_bars(&self, bars: &[crate::theta_v0::types::Bar], config: &ThetaConfig) -> f64 {
        let first_px = bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(self.missing_first_px_fallback);
        (first_px * self.first_px_multiplier).max(self.nav_floor)
    }
}

fn parse_probe_profile(id: &'static str, text: Option<&str>) -> Result<ProbeProfile, PolicyError> {
    let (record, fields) = parse_header(id, text)?;
    read_str(id, &fields, "supersedes")?;
    read_str(id, &fields, "authorizer")?;
    read_str(id, &fields, "authorized_at")?;
    let profile = ProbeProfile {
        record,
        nav_floor: read_f64(id, &fields, "nav_floor")?,
        first_px_multiplier: read_f64(id, &fields, "first_px_multiplier")?,
        missing_first_px_fallback: read_f64(id, &fields, "missing_first_px_fallback")?,
    };
    profile.validate()?;
    Ok(profile)
}

pub fn parse_l3_probe_profile(text: Option<&str>) -> Result<ProbeProfile, PolicyError> {
    parse_probe_profile(L3_PROBE_PROFILE_ID, text)
}

pub fn parse_no_floor_probe_profile(text: Option<&str>) -> Result<ProbeProfile, PolicyError> {
    parse_probe_profile(NO_FLOOR_PROBE_PROFILE_ID, text)
}

pub fn active_l3_probe_profile() -> Result<ProbeProfile, PolicyError> {
    parse_l3_probe_profile(Some(ACTIVE_L3_PROBE_PROFILE_JSON))
}

pub fn active_no_floor_probe_profile() -> Result<ProbeProfile, PolicyError> {
    parse_no_floor_probe_profile(Some(ACTIVE_NO_FLOOR_PROBE_PROFILE_JSON))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutate_record(change: impl FnOnce(&mut Value)) -> String {
        let mut root: Value = serde_json::from_str(ACTIVE_L3_PROBE_PROFILE_JSON).unwrap();
        change(&mut root);
        root.to_string()
    }

    #[test]
    fn active_profile_has_current_formula_and_identity() {
        let profile = active_l3_probe_profile().unwrap();
        assert_eq!(profile.record.label(), "#1605/l3-probe-profile/v1");
        assert_eq!(
            (
                profile.nav_floor,
                profile.first_px_multiplier,
                profile.missing_first_px_fallback
            ),
            (1.0e6, 1000.0, 1.0)
        );
    }

    /// 两个真实 NAV 消费域保持旧公式：L3 有下限，同族无下限回放不得被统一成 1e6。
    #[test]
    fn named_profiles_reach_real_nav_and_preserve_no_floor_behavior() {
        let config = ThetaConfig::default();
        let bar = |i, close, untradable| crate::theta_v0::types::Bar {
            source_index: i,
            timestamp: i as i64,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
            untradable,
        };
        let bars = vec![bar(0, 9_999, true), bar(1, 12_345, false)];
        let first_px = 12_345.0 * config.tick.tick_size;
        let l3 = active_l3_probe_profile().unwrap();
        let no_floor = active_no_floor_probe_profile().unwrap();
        assert_eq!(
            l3.nav_for_bars(&bars, &config),
            (first_px * 1000.0).max(1.0e6)
        );
        assert_eq!(no_floor.nav_for_bars(&bars, &config), first_px * 1000.0);
        assert_eq!(no_floor.record.label(), "#1605/no-floor-probe-profile/v1");

        let mut changed = no_floor;
        changed.first_px_multiplier = 500.0;
        assert_ne!(
            changed.nav_for_bars(&bars, &config),
            no_floor.nav_for_bars(&bars, &config),
            "用无下限 profile 避免 L3 的 1e6 下限遮住倍率变异"
        );
        let missing = active_no_floor_probe_profile().unwrap();
        assert_eq!(missing.nav_for_bars(&[], &config), 1000.0);
    }

    #[test]
    fn missing_and_invalid_profile_are_typed_errors() {
        assert_eq!(
            parse_l3_probe_profile(None),
            Err(PolicyError::RecordMissing {
                id: L3_PROBE_PROFILE_ID
            })
        );
        let missing = mutate_record(|root| {
            root["fields"].as_object_mut().unwrap().remove("nav_floor");
        });
        assert!(matches!(
            parse_l3_probe_profile(Some(&missing)),
            Err(PolicyError::AwaitingPolicy { field, .. }) if field == "nav_floor"
        ));
        let invalid =
            mutate_record(|root| root["fields"]["first_px_multiplier"]["value"] = 0.into());
        assert!(matches!(
            parse_l3_probe_profile(Some(&invalid)),
            Err(PolicyError::Invalid { field, .. }) if field == "first_px_multiplier"
        ));
        assert!(matches!(
            parse_no_floor_probe_profile(None),
            Err(PolicyError::RecordMissing { id }) if id == NO_FLOOR_PROBE_PROFILE_ID
        ));
        let mut root: Value = serde_json::from_str(ACTIVE_NO_FLOOR_PROBE_PROFILE_JSON).unwrap();
        root["fields"]["nav_floor"]["value"] = (-1).into();
        assert!(matches!(
            parse_no_floor_probe_profile(Some(&root.to_string())),
            Err(PolicyError::Invalid { field, .. }) if field == "nav_floor"
        ));
    }
}
