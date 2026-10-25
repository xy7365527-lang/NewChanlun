//! ADR 0022 验收判据参数记录化：evidence_gate 版本化记录读取器（#1677/#1605）。
//!
//! 字段集 `{confidence, block_unit, min_events, min_blocks, min_windows, min_coverage}`，
//! 版本规则见 ADR 0022 修订节（2026-10-22）。缺失即报错，不回退 ADR 0022 正文常数（#799；
//! ADR 0041:15）。
//!
//! 本模块只提供读取器和数据结构；签署实例记录存储在 ADR 0022 文档中（v1 的 policy_id
//! 为 `#1677/evidence-gate/v1`）。消费点：#1398、#1410、#1441（ADR 0022:176-179）。

use std::fmt;

/// block_unit 枚举：日历月（UTC）或固定 bar 数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockUnit {
    /// 按 UTC 日历月分块（ADR 0022:30 默认）。
    CalendarMonthUTC,
    /// 固定 bar 数分块（1m 粒度），值为块长（ADR 0022:115 代价）。
    FixedBars1m { bars: u32 },
}

impl fmt::Display for BlockUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockUnit::CalendarMonthUTC => write!(f, "CalendarMonthUTC"),
            BlockUnit::FixedBars1m { bars } => write!(f, "FixedBars1m({})", bars),
        }
    }
}

/// evidence_gate 版本化记录（ADR 0022:99-110）。
///
/// 全部字段为定点整数或枚举（ADR 0022:99「不出现浮点」），confidence 与 min_coverage
/// 用分数表示（分子/分母）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceGate {
    /// 策略记录 ID（如 `#1677/evidence-gate/v1`）。
    pub policy_id: String,
    /// 版本号（如 `v1`）。
    pub version: String,
    /// 被替换的版本（首版为 `none`）。
    pub supersedes: String,
    /// 单边置信水平（分子/分母，如 95/100 表示 0.95）。
    pub confidence: (u32, u32),
    /// block bootstrap 的块单位。
    pub block_unit: BlockUnit,
    /// 最小事件样本数（≥ 2，ADR 0022:116）。
    pub min_events: u32,
    /// 最小月块数（≥ 2，以 block_unit 计，ADR 0022:116）。
    pub min_blocks: u32,
    /// 最小窗口数（≥ 2，ADR 0022:117）。
    pub min_windows: u32,
    /// 最小覆盖率（分子/分母，如 1/2 表示 0.5）。
    pub min_coverage: (u32, u32),
    /// 授权人。
    pub authorizer: String,
    /// 授权日期（YYYY-MM-DD）。
    pub authorized_at: String,
}

impl EvidenceGate {
    /// 获取置信水平作为 f64（如 0.95）。
    pub fn confidence_f64(&self) -> f64 {
        self.confidence.0 as f64 / self.confidence.1 as f64
    }

    /// 获取最小覆盖率作为 f64（如 0.5）。
    pub fn min_coverage_f64(&self) -> f64 {
        self.min_coverage.0 as f64 / self.min_coverage.1 as f64
    }

    /// 计算单边置信下界的 z 值（如 confidence=95/100 → z≈1.645）。
    ///
    /// 使用近似公式，只对标准置信水平给出精确值。ADR 0022:105 值域 ∈ (1/2, 1)。
    pub fn z_alpha(&self) -> f64 {
        let conf = self.confidence_f64();
        // 标准值：95/100 → 1.645；975/1000 → 1.96；99/100 → 2.326
        if (conf - 0.95).abs() < 1e-6 {
            1.645
        } else if (conf - 0.975).abs() < 1e-6 {
            1.96
        } else if (conf - 0.99).abs() < 1e-6 {
            2.326
        } else {
            // 近似：正态分布分位数的简化公式（Beasley-Springer-Moro 近似）
            let p = 1.0 - conf;
            if p >= 0.5 || p <= 0.0 {
                // validate() 已确保 conf ∈ (1/2, 1)，故 p ∈ (0, 1/2)；此分支不应到达
                unreachable!("z_alpha: p={} 越界（confidence={}），应在 validate() 拦截", p, conf);
            }
            let t = (-2.0 * p.ln()).sqrt();
            let c0 = 2.515517;
            let c1 = 0.802853;
            let c2 = 0.010328;
            let d1 = 1.432788;
            let d2 = 0.189269;
            let d3 = 0.001308;
            t - (c0 + c1 * t + c2 * t * t) / (1.0 + d1 * t + d2 * t * t + d3 * t * t * t)
        }
    }

    /// 验证字段值域（ADR 0022:103-118）。
    ///
    /// 越界即返回 Err；只检查结构约束，不检查「是否早于 run_started_at」（那是消费点职责）。
    pub fn validate(&self) -> Result<(), String> {
        // confidence ∈ (1/2, 1)
        let conf = self.confidence_f64();
        if conf <= 0.5 || conf >= 1.0 {
            return Err(format!(
                "confidence 越界：{}/{} ∉ (1/2, 1)",
                self.confidence.0, self.confidence.1
            ));
        }

        // min_events ≥ 2
        if self.min_events < 2 {
            return Err(format!("min_events 越界：{} < 2", self.min_events));
        }

        // min_blocks ≥ 2
        if self.min_blocks < 2 {
            return Err(format!("min_blocks 越界：{} < 2", self.min_blocks));
        }

        // min_windows ≥ 2
        if self.min_windows < 2 {
            return Err(format!("min_windows 越界：{} < 2", self.min_windows));
        }

        // min_coverage ∈ (0, 1]
        let cov = self.min_coverage_f64();
        if cov <= 0.0 || cov > 1.0 {
            return Err(format!(
                "min_coverage 越界：{}/{} ∉ (0, 1]",
                self.min_coverage.0, self.min_coverage.1
            ));
        }

        // block_unit = FixedBars1m 只允许 1m 粒度
        if let BlockUnit::FixedBars1m { bars } = self.block_unit {
            if bars == 0 {
                return Err("block_unit FixedBars1m 的 bars 不得为 0".to_string());
            }
            // 粒度检查由消费点根据数据集的 bar_seconds 验证（本处不持有数据集信息）
        }

        Ok(())
    }
}

/// 从字符串解析 BlockUnit（格式：`CalendarMonthUTC` 或 `FixedBars1m{N}`）。
fn parse_block_unit(s: &str) -> Result<BlockUnit, String> {
    if s == "CalendarMonthUTC" {
        Ok(BlockUnit::CalendarMonthUTC)
    } else if let Some(bars_str) = s.strip_prefix("FixedBars1m{").and_then(|x| x.strip_suffix('}'))
    {
        let bars: u32 = bars_str
            .parse()
            .map_err(|_| format!("FixedBars1m 块长解析失败：{}", bars_str))?;
        Ok(BlockUnit::FixedBars1m { bars })
    } else {
        Err(format!("无法识别的 block_unit：{}", s))
    }
}

/// 从字符串解析分数（格式：`A/B`，返回 (A, B)）。
fn parse_fraction(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err(format!("分数格式错误（需 A/B）：{}", s));
    }
    let a: u32 = parts[0]
        .parse()
        .map_err(|_| format!("分子解析失败：{}", parts[0]))?;
    let b: u32 = parts[1]
        .parse()
        .map_err(|_| format!("分母解析失败：{}", parts[1]))?;
    if b == 0 {
        return Err(format!("分母不得为 0：{}", s));
    }
    Ok((a, b))
}

/// 从 ADR 文档格式的文本块解析 evidence_gate 记录。
///
/// 输入格式示例（ADR 0022:135-147）：
/// ```text
/// policy_id: #1677/evidence-gate/v1
/// version: v1
/// supersedes: none
/// confidence: 95/100
/// block_unit: CalendarMonthUTC
/// min_events: 30
/// min_blocks: 12
/// min_windows: 3
/// min_coverage: 1/2
/// authorizer: 编排者（#853 resolution 5289198250 第 3/6/7 条）；记录化 #1677
/// authorized_at: 2026-08-14
/// ```
///
/// 字段缺失、越界或格式错误均返回 Err（#799；ADR 0041:15「缺失即报错」）。
pub fn parse_evidence_gate(text: &str) -> Result<EvidenceGate, String> {
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
    let confidence = parse_fraction(fields.get("confidence").ok_or("字段缺失：confidence")?)?;
    let block_unit = parse_block_unit(fields.get("block_unit").ok_or("字段缺失：block_unit")?)?;
    let min_events: u32 = fields
        .get("min_events")
        .ok_or("字段缺失：min_events")?
        .parse()
        .map_err(|_| "min_events 解析失败")?;
    let min_blocks: u32 = fields
        .get("min_blocks")
        .ok_or("字段缺失：min_blocks")?
        .parse()
        .map_err(|_| "min_blocks 解析失败")?;
    let min_windows: u32 = fields
        .get("min_windows")
        .ok_or("字段缺失：min_windows")?
        .parse()
        .map_err(|_| "min_windows 解析失败")?;
    let min_coverage =
        parse_fraction(fields.get("min_coverage").ok_or("字段缺失：min_coverage")?)?;
    let authorizer = fields
        .get("authorizer")
        .ok_or("字段缺失：authorizer")?
        .clone();
    let authorized_at = fields
        .get("authorized_at")
        .ok_or("字段缺失：authorized_at")?
        .clone();

    let record = EvidenceGate {
        policy_id,
        version,
        supersedes,
        confidence,
        block_unit,
        min_events,
        min_blocks,
        min_windows,
        min_coverage,
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
    fn parse_v1_from_adr_text() {
        let text = r#"
policy_id: #1677/evidence-gate/v1
version: v1
supersedes: none
confidence: 95/100
block_unit: CalendarMonthUTC
min_events: 30
min_blocks: 12
min_windows: 3
min_coverage: 1/2
authorizer: 编排者（#853 resolution 5289198250 第 3/6/7 条）；记录化 #1677
authorized_at: 2026-08-14
        "#;

        let eg = parse_evidence_gate(text).expect("解析 v1 失败");
        assert_eq!(eg.policy_id, "#1677/evidence-gate/v1");
        assert_eq!(eg.version, "v1");
        assert_eq!(eg.confidence, (95, 100));
        assert_eq!(eg.block_unit, BlockUnit::CalendarMonthUTC);
        assert_eq!(eg.min_events, 30);
        assert_eq!(eg.min_blocks, 12);
        assert_eq!(eg.min_windows, 3);
        assert_eq!(eg.min_coverage, (1, 2));
        assert!((eg.z_alpha() - 1.645).abs() < 1e-3);
        assert!((eg.min_coverage_f64() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn validate_bounds() {
        let mut eg = EvidenceGate {
            policy_id: "test".to_string(),
            version: "v1".to_string(),
            supersedes: "none".to_string(),
            confidence: (95, 100),
            block_unit: BlockUnit::CalendarMonthUTC,
            min_events: 30,
            min_blocks: 12,
            min_windows: 3,
            min_coverage: (1, 2),
            authorizer: "test".to_string(),
            authorized_at: "2026-08-14".to_string(),
        };

        assert!(eg.validate().is_ok());

        // confidence 越界
        eg.confidence = (1, 2); // = 0.5，越界
        assert!(eg.validate().is_err());
        eg.confidence = (95, 100);

        // min_events < 2
        eg.min_events = 1;
        assert!(eg.validate().is_err());
        eg.min_events = 30;

        // min_windows < 2
        eg.min_windows = 1;
        assert!(eg.validate().is_err());
    }

    #[test]
    fn parse_fixed_bars_block_unit() {
        let text = r#"
policy_id: test
version: v2
supersedes: v1
confidence: 95/100
block_unit: FixedBars1m{43200}
min_events: 30
min_blocks: 12
min_windows: 3
min_coverage: 1/2
authorizer: test
authorized_at: 2026-10-22
        "#;

        let eg = parse_evidence_gate(text).expect("解析 FixedBars 失败");
        assert_eq!(eg.block_unit, BlockUnit::FixedBars1m { bars: 43200 });
    }

    #[test]
    fn missing_field_errors() {
        let text = r#"
policy_id: test
version: v1
        "#;
        assert!(parse_evidence_gate(text).is_err());
    }
}
