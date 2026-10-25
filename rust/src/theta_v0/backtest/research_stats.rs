//! C-19 研究回测与验收统计记录（#1605）。
//!
//! 唯一数值源是 `policy_records/research-stats.json`，经统一的 `PolicyRef` / `PolicyError`
//! 机制编进二进制。evidence_gate 的样本单位与判据不同，绝不能拿本记录替它补值。
//! 探针 NAV 也不藏在本结构里：真实调用读独立的具名 `l3-probe-profile`。

use serde_json::Value;

use super::super::policy_records::{
    invalid, parse_header, read_f64, read_str, read_u32, PolicyError, PolicyRef,
    ACTIVE_RESEARCH_STATS_JSON, RESEARCH_STATS_ID,
};

/// C-19 当前生效记录的全部统计自由度。
#[derive(Debug, Clone, PartialEq)]
pub struct ResearchStats {
    pub record: PolicyRef,
    pub z_alpha: f64,
    pub perm_alpha: f64,
    pub annualization_days: f64,
    pub significance_p_threshold: f64,
    pub cvar_tail: f64,
    pub ci_lower_quantile: f64,
    pub ci_upper_quantile: f64,
    pub sharpe_ci_z: f64,
    pub icc_low_threshold: f64,
    pub icc_high_threshold: f64,
    pub trade_block_len: usize,
    pub trade_n_resample: usize,
    pub delta_r_n_resample: usize,
    pub delta_r_block_len: usize,
    pub min_test_bars: usize,
    pub time_block_bars: usize,
    pub min_trades_for_l2: usize,
    pub econ_neff_lag: usize,
    pub econ_block_len: usize,
    pub econ_n_resample: usize,
}

fn read_usize(
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<usize, PolicyError> {
    Ok(read_u32(RESEARCH_STATS_ID, fields, name)? as usize)
}

impl ResearchStats {
    /// 值域错误全部保留记录 id 与字段名；调用方不得换值重试。
    pub fn validate(&self) -> Result<(), PolicyError> {
        for (name, value) in [
            ("z_alpha", self.z_alpha),
            ("annualization_days", self.annualization_days),
            ("sharpe_ci_z", self.sharpe_ci_z),
        ] {
            if value <= 0.0 {
                return Err(invalid(RESEARCH_STATS_ID, name, "须为有限正数"));
            }
        }
        for (name, value) in [
            ("perm_alpha", self.perm_alpha),
            ("significance_p_threshold", self.significance_p_threshold),
            ("cvar_tail", self.cvar_tail),
            ("ci_lower_quantile", self.ci_lower_quantile),
            ("ci_upper_quantile", self.ci_upper_quantile),
        ] {
            if !(0.0..1.0).contains(&value) {
                return Err(invalid(RESEARCH_STATS_ID, name, "须属于开区间 (0,1)"));
            }
        }
        if self.ci_lower_quantile >= self.ci_upper_quantile {
            return Err(invalid(
                RESEARCH_STATS_ID,
                "ci_upper_quantile",
                "须大于 ci_lower_quantile",
            ));
        }
        for (name, value) in [
            ("icc_low_threshold", self.icc_low_threshold),
            ("icc_high_threshold", self.icc_high_threshold),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(invalid(RESEARCH_STATS_ID, name, "须属于闭区间 [0,1]"));
            }
        }
        if self.icc_low_threshold >= self.icc_high_threshold {
            return Err(invalid(
                RESEARCH_STATS_ID,
                "icc_high_threshold",
                "须大于 icc_low_threshold",
            ));
        }
        for (name, value) in [
            ("trade_block_len", self.trade_block_len),
            ("trade_n_resample", self.trade_n_resample),
            ("delta_r_n_resample", self.delta_r_n_resample),
            ("delta_r_block_len", self.delta_r_block_len),
            ("time_block_bars", self.time_block_bars),
            ("econ_neff_lag", self.econ_neff_lag),
            ("econ_block_len", self.econ_block_len),
            ("econ_n_resample", self.econ_n_resample),
        ] {
            if value == 0 {
                return Err(invalid(RESEARCH_STATS_ID, name, "须为正整数"));
            }
        }
        if self.min_test_bars < 2 {
            return Err(invalid(RESEARCH_STATS_ID, "min_test_bars", "须 ≥ 2"));
        }
        if self.min_trades_for_l2 < 2 {
            return Err(invalid(RESEARCH_STATS_ID, "min_trades_for_l2", "须 ≥ 2"));
        }
        Ok(())
    }
}

/// 解析指定文本；`None` 表示记录缺失，绝不是使用旧常数的信号。
pub fn parse_research_stats(text: Option<&str>) -> Result<ResearchStats, PolicyError> {
    let (record, fields) = parse_header(RESEARCH_STATS_ID, text)?;
    // 这三项是签署元数据，也必须存在；运行计算不另建字符串事实源。
    read_str(RESEARCH_STATS_ID, &fields, "supersedes")?;
    read_str(RESEARCH_STATS_ID, &fields, "authorizer")?;
    read_str(RESEARCH_STATS_ID, &fields, "authorized_at")?;
    let stats = ResearchStats {
        record,
        z_alpha: read_f64(RESEARCH_STATS_ID, &fields, "z_alpha")?,
        perm_alpha: read_f64(RESEARCH_STATS_ID, &fields, "perm_alpha")?,
        annualization_days: read_f64(RESEARCH_STATS_ID, &fields, "annualization_days")?,
        significance_p_threshold: read_f64(
            RESEARCH_STATS_ID,
            &fields,
            "significance_p_threshold",
        )?,
        cvar_tail: read_f64(RESEARCH_STATS_ID, &fields, "cvar_tail")?,
        ci_lower_quantile: read_f64(RESEARCH_STATS_ID, &fields, "ci_lower_quantile")?,
        ci_upper_quantile: read_f64(RESEARCH_STATS_ID, &fields, "ci_upper_quantile")?,
        sharpe_ci_z: read_f64(RESEARCH_STATS_ID, &fields, "sharpe_ci_z")?,
        icc_low_threshold: read_f64(RESEARCH_STATS_ID, &fields, "icc_low_threshold")?,
        icc_high_threshold: read_f64(RESEARCH_STATS_ID, &fields, "icc_high_threshold")?,
        trade_block_len: read_usize(&fields, "trade_block_len")?,
        trade_n_resample: read_usize(&fields, "trade_n_resample")?,
        delta_r_n_resample: read_usize(&fields, "delta_r_n_resample")?,
        delta_r_block_len: read_usize(&fields, "delta_r_block_len")?,
        min_test_bars: read_usize(&fields, "min_test_bars")?,
        time_block_bars: read_usize(&fields, "time_block_bars")?,
        min_trades_for_l2: read_usize(&fields, "min_trades_for_l2")?,
        econ_neff_lag: read_usize(&fields, "econ_neff_lag")?,
        econ_block_len: read_usize(&fields, "econ_block_len")?,
        econ_n_resample: read_usize(&fields, "econ_n_resample")?,
    };
    stats.validate()?;
    Ok(stats)
}

/// 当前生效的 C-19 记录。
pub fn active_research_stats() -> Result<ResearchStats, PolicyError> {
    parse_research_stats(Some(ACTIVE_RESEARCH_STATS_JSON))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutate_record(change: impl FnOnce(&mut Value)) -> String {
        let mut root: Value = serde_json::from_str(ACTIVE_RESEARCH_STATS_JSON).unwrap();
        change(&mut root);
        root.to_string()
    }

    #[test]
    fn active_v1_is_complete_and_carries_its_identity() {
        let stats = active_research_stats().unwrap();
        assert_eq!(stats.record.label(), "#1605/research-stats/v1");
        assert_eq!(
            (
                stats.z_alpha,
                stats.perm_alpha,
                stats.annualization_days,
                stats.sharpe_ci_z,
                stats.trade_block_len,
                stats.trade_n_resample,
                stats.min_test_bars,
                stats.time_block_bars,
                stats.econ_n_resample,
            ),
            (1.645, 0.05, 252.0, 1.96, 5, 1000, 252, 43200, 2000)
        );
    }

    #[test]
    fn missing_unsigned_and_invalid_never_fall_back() {
        assert_eq!(
            parse_research_stats(None),
            Err(PolicyError::RecordMissing {
                id: RESEARCH_STATS_ID
            })
        );
        let missing = mutate_record(|root| {
            root["fields"].as_object_mut().unwrap().remove("z_alpha");
        });
        assert_eq!(
            parse_research_stats(Some(&missing)),
            Err(PolicyError::AwaitingPolicy {
                id: RESEARCH_STATS_ID,
                field: "z_alpha".into()
            })
        );
        let unsigned = mutate_record(|root| {
            root["fields"]["z_alpha"]["value"] = Value::String("AwaitingPolicy".into());
        });
        assert!(matches!(
            parse_research_stats(Some(&unsigned)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        let invalid_z = mutate_record(|root| root["fields"]["z_alpha"]["value"] = 0.into());
        assert!(matches!(
            parse_research_stats(Some(&invalid_z)),
            Err(PolicyError::Invalid { field, .. }) if field == "z_alpha"
        ));
        let reversed_ci = mutate_record(|root| {
            root["fields"]["ci_lower_quantile"]["value"] = 0.99.into();
        });
        assert!(matches!(
            parse_research_stats(Some(&reversed_ci)),
            Err(PolicyError::Invalid { .. })
        ));
    }
}
