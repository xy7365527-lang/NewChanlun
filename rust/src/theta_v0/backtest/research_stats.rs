//! C-19 研究验证常数记录（#1605，AUDIT.md:180）。
//!
//! 与 evidence_gate 分开：C-19 常数用于研发回测验收（decontam、l3_delta_r_alpha、
//! l3_pi_probe 等），判据与采样单位与 evidence_gate（ADR 0022 闸门前）不同。
//! 合成一份会让无关改值触发「不同版本不可比」（ADR 0041:48）。
//!
//! decontam §4 预注册值（z_α=1.645、perm_α=0.05、功效门、年化 252.0 等）在 v1 记录
//! 里原样登记，缺失同样显式报错（#799；ADR 0041:15）。
//!
//! 探针资金（l3_pi_probe、l3_delta_r_alpha 等的 `max(首价×1000, 1.0e6)`）改读
//! funding-record 或探针 profile（票面承诺），v1 暂保留 1.0e6 下限与首价倍率逻辑。

/// C-19 研究验证常数版本化记录。
#[derive(Debug, Clone, PartialEq)]
pub struct ResearchStats {
    /// 策略记录 ID（如 `#1605/research-stats/v1`）。
    pub policy_id: String,
    /// 版本号（如 `v1`）。
    pub version: String,
    /// 被替换的版本（首版为 `none`）。
    pub supersedes: String,

    // ──── decontam §4 预注册常数 ────
    /// 单边置信下界的 z 值（decontam.rs:168，v1 = 1.645）。
    pub z_alpha: f64,
    /// 去污置换 p 值门槛（decontam.rs:169，v1 = 0.05）。
    pub perm_alpha: f64,

    // ──── 回测年化因子 ────
    /// 年化天数（metrics.rs:718，v1 = 252.0）。
    pub annualization_days: f64,

    // ──── 显著性门槛 ────
    /// 单侧显著性 p 值门槛（metrics.rs:562、l3_delta_r_alpha.rs:572 等，v1 = 0.05）。
    pub significance_p_threshold: f64,

    // ──── 分位区间 ────
    /// CVaR 尾部比例（metrics.rs:132，v1 = 0.05）。
    pub cvar_tail: f64,
    /// 置信区间下侧分位（metrics.rs:401，v1 = 0.025）。
    pub ci_lower_quantile: f64,
    /// 置信区间上侧分位（metrics.rs:401，v1 = 0.975）。
    pub ci_upper_quantile: f64,

    // ──── ICC 门 ────
    /// 同质阈值：ICC < 此值视为品种间同质（l3_delta_r_alpha.rs:2228，v1 = 0.3）。
    pub icc_low_threshold: f64,
    /// 异质阈值：ICC > 此值视为品种间异质（l3_delta_r_alpha.rs:2229，v1 = 0.7）。
    pub icc_high_threshold: f64,

    // ──── 探针资金 ────
    /// 探针 NAV 下限（l3_pi_probe.rs:38，v1 = 1.0e6）。
    pub probe_nav_floor: f64,
    /// 探针 NAV 首价倍率（v1 = 1000.0）。
    pub probe_nav_first_px_multiplier: f64,

    /// 授权人。
    pub authorizer: String,
    /// 授权日期（YYYY-MM-DD）。
    pub authorized_at: String,
}

impl ResearchStats {
    /// 验证字段值域。
    pub fn validate(&self) -> Result<(), String> {
        if self.z_alpha <= 0.0 {
            return Err(format!("z_alpha 须为正：{}", self.z_alpha));
        }
        if self.perm_alpha <= 0.0 || self.perm_alpha >= 1.0 {
            return Err(format!("perm_alpha 越界：{} ∉ (0, 1)", self.perm_alpha));
        }
        if self.annualization_days <= 0.0 {
            return Err(format!(
                "annualization_days 须为正：{}",
                self.annualization_days
            ));
        }
        if self.significance_p_threshold <= 0.0 || self.significance_p_threshold >= 1.0 {
            return Err(format!(
                "significance_p_threshold 越界：{} ∉ (0, 1)",
                self.significance_p_threshold
            ));
        }
        if self.cvar_tail <= 0.0 || self.cvar_tail >= 1.0 {
            return Err(format!("cvar_tail 越界：{} ∉ (0, 1)", self.cvar_tail));
        }
        if self.ci_lower_quantile <= 0.0
            || self.ci_lower_quantile >= 1.0
            || self.ci_upper_quantile <= 0.0
            || self.ci_upper_quantile >= 1.0
        {
            return Err("置信区间分位越界：须 ∈ (0, 1)".to_string());
        }
        if self.ci_lower_quantile >= self.ci_upper_quantile {
            return Err("ci_lower_quantile 须 < ci_upper_quantile".to_string());
        }
        if self.icc_low_threshold < 0.0
            || self.icc_low_threshold > 1.0
            || self.icc_high_threshold < 0.0
            || self.icc_high_threshold > 1.0
        {
            return Err("ICC 门槛越界：须 ∈ [0, 1]".to_string());
        }
        if self.icc_low_threshold >= self.icc_high_threshold {
            return Err("icc_low_threshold 须 < icc_high_threshold".to_string());
        }
        if self.probe_nav_floor <= 0.0 {
            return Err(format!(
                "probe_nav_floor 须为正：{}",
                self.probe_nav_floor
            ));
        }
        if self.probe_nav_first_px_multiplier <= 0.0 {
            return Err(format!(
                "probe_nav_first_px_multiplier 须为正：{}",
                self.probe_nav_first_px_multiplier
            ));
        }
        Ok(())
    }

    /// 计算探针 NAV（l3_pi_probe.rs:38、l3_delta_r_alpha.rs:536 等同源公式）。
    pub fn compute_probe_nav(&self, first_tradable_px: f64) -> f64 {
        (first_tradable_px * self.probe_nav_first_px_multiplier).max(self.probe_nav_floor)
    }
}

/// 从 ADR 文档格式的文本块解析 research_stats 记录。
///
/// 输入格式示例（v1 预注册值）：
/// ```text
/// policy_id: #1605/research-stats/v1
/// version: v1
/// supersedes: none
/// z_alpha: 1.645
/// perm_alpha: 0.05
/// annualization_days: 252.0
/// significance_p_threshold: 0.05
/// cvar_tail: 0.05
/// ci_lower_quantile: 0.025
/// ci_upper_quantile: 0.975
/// icc_low_threshold: 0.3
/// icc_high_threshold: 0.7
/// probe_nav_floor: 1000000.0
/// probe_nav_first_px_multiplier: 1000.0
/// authorizer: decontam §4（preregistered 2026-07-01）；记录化 #1605
/// authorized_at: 2026-07-01
/// ```
pub fn parse_research_stats(text: &str) -> Result<ResearchStats, String> {
    let mut fields = std::collections::HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    let policy_id = fields
        .get("policy_id")
        .ok_or("字段缺失：policy_id")?
        .clone();
    let version = fields.get("version").ok_or("字段缺失：version")?.clone();
    let supersedes = fields
        .get("supersedes")
        .ok_or("字段缺失：supersedes")?
        .clone();

    let z_alpha: f64 = fields
        .get("z_alpha")
        .ok_or("字段缺失：z_alpha")?
        .parse()
        .map_err(|_| "z_alpha 解析失败")?;
    let perm_alpha: f64 = fields
        .get("perm_alpha")
        .ok_or("字段缺失：perm_alpha")?
        .parse()
        .map_err(|_| "perm_alpha 解析失败")?;
    let annualization_days: f64 = fields
        .get("annualization_days")
        .ok_or("字段缺失：annualization_days")?
        .parse()
        .map_err(|_| "annualization_days 解析失败")?;
    let significance_p_threshold: f64 = fields
        .get("significance_p_threshold")
        .ok_or("字段缺失：significance_p_threshold")?
        .parse()
        .map_err(|_| "significance_p_threshold 解析失败")?;
    let cvar_tail: f64 = fields
        .get("cvar_tail")
        .ok_or("字段缺失：cvar_tail")?
        .parse()
        .map_err(|_| "cvar_tail 解析失败")?;
    let ci_lower_quantile: f64 = fields
        .get("ci_lower_quantile")
        .ok_or("字段缺失：ci_lower_quantile")?
        .parse()
        .map_err(|_| "ci_lower_quantile 解析失败")?;
    let ci_upper_quantile: f64 = fields
        .get("ci_upper_quantile")
        .ok_or("字段缺失：ci_upper_quantile")?
        .parse()
        .map_err(|_| "ci_upper_quantile 解析失败")?;
    let icc_low_threshold: f64 = fields
        .get("icc_low_threshold")
        .ok_or("字段缺失：icc_low_threshold")?
        .parse()
        .map_err(|_| "icc_low_threshold 解析失败")?;
    let icc_high_threshold: f64 = fields
        .get("icc_high_threshold")
        .ok_or("字段缺失：icc_high_threshold")?
        .parse()
        .map_err(|_| "icc_high_threshold 解析失败")?;
    let probe_nav_floor: f64 = fields
        .get("probe_nav_floor")
        .ok_or("字段缺失：probe_nav_floor")?
        .parse()
        .map_err(|_| "probe_nav_floor 解析失败")?;
    let probe_nav_first_px_multiplier: f64 = fields
        .get("probe_nav_first_px_multiplier")
        .ok_or("字段缺失：probe_nav_first_px_multiplier")?
        .parse()
        .map_err(|_| "probe_nav_first_px_multiplier 解析失败")?;

    let authorizer = fields
        .get("authorizer")
        .ok_or("字段缺失：authorizer")?
        .clone();
    let authorized_at = fields
        .get("authorized_at")
        .ok_or("字段缺失：authorized_at")?
        .clone();

    let record = ResearchStats {
        policy_id,
        version,
        supersedes,
        z_alpha,
        perm_alpha,
        annualization_days,
        significance_p_threshold,
        cvar_tail,
        ci_lower_quantile,
        ci_upper_quantile,
        icc_low_threshold,
        icc_high_threshold,
        probe_nav_floor,
        probe_nav_first_px_multiplier,
        authorizer,
        authorized_at,
    };

    record.validate()?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v1_preregistered() {
        let text = r#"
policy_id: #1605/research-stats/v1
version: v1
supersedes: none
z_alpha: 1.645
perm_alpha: 0.05
annualization_days: 252.0
significance_p_threshold: 0.05
cvar_tail: 0.05
ci_lower_quantile: 0.025
ci_upper_quantile: 0.975
icc_low_threshold: 0.3
icc_high_threshold: 0.7
probe_nav_floor: 1000000.0
probe_nav_first_px_multiplier: 1000.0
authorizer: decontam §4（preregistered 2026-07-01）；记录化 #1605
authorized_at: 2026-07-01
        "#;

        let rs = parse_research_stats(text).expect("解析 v1 失败");
        assert_eq!(rs.policy_id, "#1605/research-stats/v1");
        assert_eq!(rs.version, "v1");
        assert!((rs.z_alpha - 1.645).abs() < 1e-6);
        assert!((rs.perm_alpha - 0.05).abs() < 1e-6);
        assert!((rs.annualization_days - 252.0).abs() < 1e-6);
        assert!((rs.icc_low_threshold - 0.3).abs() < 1e-6);
        assert!((rs.icc_high_threshold - 0.7).abs() < 1e-6);
        assert!((rs.probe_nav_floor - 1.0e6).abs() < 1e-3);
    }

    #[test]
    fn validate_bounds() {
        let mut rs = ResearchStats {
            policy_id: "test".to_string(),
            version: "v1".to_string(),
            supersedes: "none".to_string(),
            z_alpha: 1.645,
            perm_alpha: 0.05,
            annualization_days: 252.0,
            significance_p_threshold: 0.05,
            cvar_tail: 0.05,
            ci_lower_quantile: 0.025,
            ci_upper_quantile: 0.975,
            icc_low_threshold: 0.3,
            icc_high_threshold: 0.7,
            probe_nav_floor: 1.0e6,
            probe_nav_first_px_multiplier: 1000.0,
            authorizer: "test".to_string(),
            authorized_at: "2026-07-01".to_string(),
        };

        assert!(rs.validate().is_ok());

        // z_alpha <= 0
        rs.z_alpha = -1.0;
        assert!(rs.validate().is_err());
        rs.z_alpha = 1.645;

        // perm_alpha 越界
        rs.perm_alpha = 1.5;
        assert!(rs.validate().is_err());
        rs.perm_alpha = 0.05;

        // ICC 门槛颠倒
        rs.icc_low_threshold = 0.8;
        rs.icc_high_threshold = 0.6;
        assert!(rs.validate().is_err());
    }

    #[test]
    fn compute_probe_nav_formula() {
        let rs = ResearchStats {
            policy_id: "test".to_string(),
            version: "v1".to_string(),
            supersedes: "none".to_string(),
            z_alpha: 1.645,
            perm_alpha: 0.05,
            annualization_days: 252.0,
            significance_p_threshold: 0.05,
            cvar_tail: 0.05,
            ci_lower_quantile: 0.025,
            ci_upper_quantile: 0.975,
            icc_low_threshold: 0.3,
            icc_high_threshold: 0.7,
            probe_nav_floor: 1.0e6,
            probe_nav_first_px_multiplier: 1000.0,
            authorizer: "test".to_string(),
            authorized_at: "2026-07-01".to_string(),
        };

        // 低价品种：首价 500 → 500×1000 = 500,000 < 1e6 → 1e6
        assert!((rs.compute_probe_nav(500.0) - 1.0e6).abs() < 1e-3);

        // 高价品种：首价 50000 → 50000×1000 = 5e7 > 1e6 → 5e7
        assert!((rs.compute_probe_nav(50000.0) - 5.0e7).abs() < 1e-3);
    }
}
