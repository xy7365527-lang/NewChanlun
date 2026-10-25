//! ADR 0022 evidence_gate 记录读取器（#1677；#1605 只统一到 main 的记录机制）。
//!
//! 六字段与 C-19 研究统计保持两份独立记录。这里不实现 #1398/#1410/#1441 的下游闸门，
//! 但读取结果携带 `PolicyRef`，缺失、未签和非法值均为 `PolicyError`，没有备用值。

use std::fmt;

use super::super::policy_records::{
    invalid, parse_header, read_str, read_u32, PolicyError, PolicyRef, ACTIVE_EVIDENCE_GATE_JSON,
    EVIDENCE_GATE_ID,
};

/// block_unit 枚举：日历月（UTC）或固定 1m bar 数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockUnit {
    CalendarMonthUTC,
    FixedBars1m { bars: u32 },
}

impl fmt::Display for BlockUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockUnit::CalendarMonthUTC => write!(f, "CalendarMonthUTC"),
            BlockUnit::FixedBars1m { bars } => write!(f, "FixedBars1m({bars})"),
        }
    }
}

/// ADR 0022 六字段记录；confidence/min_coverage 始终以定点分数保存。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceGate {
    pub record: PolicyRef,
    pub confidence: (u32, u32),
    pub block_unit: BlockUnit,
    pub min_events: u32,
    pub min_blocks: u32,
    pub min_windows: u32,
    pub min_coverage: (u32, u32),
}

impl EvidenceGate {
    pub fn confidence_f64(&self) -> f64 {
        self.confidence.0 as f64 / self.confidence.1 as f64
    }

    pub fn min_coverage_f64(&self) -> f64 {
        self.min_coverage.0 as f64 / self.min_coverage.1 as f64
    }

    /// 单边正态分位。保留原实施的标准置信值与 Beasley-Springer-Moro 近似；
    /// 越界仍在 validate 拒绝，不用静默 0.0 冒充读数。
    pub fn z_alpha(&self) -> f64 {
        let conf = self.confidence_f64();
        if (conf - 0.95).abs() < 1e-6 {
            1.645
        } else if (conf - 0.975).abs() < 1e-6 {
            1.96
        } else if (conf - 0.99).abs() < 1e-6 {
            2.326
        } else {
            let p = 1.0 - conf;
            if p >= 0.5 || p <= 0.0 {
                unreachable!(
                    "z_alpha: p={} 越界（confidence={}），应在 validate() 拦截",
                    p, conf
                );
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

    pub fn validate(&self) -> Result<(), PolicyError> {
        for (name, (_, denominator)) in [
            ("confidence", self.confidence),
            ("min_coverage", self.min_coverage),
        ] {
            if denominator == 0 {
                return Err(invalid(EVIDENCE_GATE_ID, name, "分母不得为 0"));
            }
        }
        let conf = self.confidence_f64();
        if conf <= 0.5 || conf >= 1.0 {
            return Err(invalid(EVIDENCE_GATE_ID, "confidence", "须属于 (1/2,1)"));
        }
        for (name, value) in [
            ("min_events", self.min_events),
            ("min_blocks", self.min_blocks),
            ("min_windows", self.min_windows),
        ] {
            if value < 2 {
                return Err(invalid(EVIDENCE_GATE_ID, name, "须 ≥ 2"));
            }
        }
        let coverage = self.min_coverage_f64();
        if coverage <= 0.0 || coverage > 1.0 {
            return Err(invalid(EVIDENCE_GATE_ID, "min_coverage", "须属于 (0,1]"));
        }
        if matches!(self.block_unit, BlockUnit::FixedBars1m { bars: 0 }) {
            return Err(invalid(
                EVIDENCE_GATE_ID,
                "block_unit",
                "FixedBars1m 块长须 ≥ 1",
            ));
        }
        Ok(())
    }
}

fn parse_block_unit(id: &'static str, raw: &str) -> Result<BlockUnit, PolicyError> {
    if raw == "CalendarMonthUTC" {
        return Ok(BlockUnit::CalendarMonthUTC);
    }
    if let Some(bars) = raw
        .strip_prefix("FixedBars1m{")
        .and_then(|x| x.strip_suffix('}'))
    {
        let bars = bars
            .parse::<u32>()
            .map_err(|_| invalid(id, "block_unit", format!("非法固定块长 `{bars}`")))?;
        return Ok(BlockUnit::FixedBars1m { bars });
    }
    Err(invalid(
        id,
        "block_unit",
        format!("须为 CalendarMonthUTC 或 FixedBars1m{{N}}，得 `{raw}`"),
    ))
}

fn parse_fraction(id: &'static str, name: &str, raw: &str) -> Result<(u32, u32), PolicyError> {
    let (a, b) = raw
        .split_once('/')
        .ok_or_else(|| invalid(id, name, "定点分数须写成 A/B"))?;
    if b.contains('/') {
        return Err(invalid(id, name, "定点分数须且只须含一个 /"));
    }
    let numerator = a
        .parse::<u32>()
        .map_err(|_| invalid(id, name, format!("非法分子 `{a}`")))?;
    let denominator = b
        .parse::<u32>()
        .map_err(|_| invalid(id, name, format!("非法分母 `{b}`")))?;
    if denominator == 0 {
        return Err(invalid(id, name, "分母不得为 0"));
    }
    Ok((numerator, denominator))
}

/// 从统一 JSON 机制读取 evidence_gate；`None` 是 RecordMissing，不是默认 v1。
pub fn parse_evidence_gate(text: Option<&str>) -> Result<EvidenceGate, PolicyError> {
    let (record, fields) = parse_header(EVIDENCE_GATE_ID, text)?;
    read_str(EVIDENCE_GATE_ID, &fields, "supersedes")?;
    read_str(EVIDENCE_GATE_ID, &fields, "authorizer")?;
    read_str(EVIDENCE_GATE_ID, &fields, "authorized_at")?;
    let confidence_raw = read_str(EVIDENCE_GATE_ID, &fields, "confidence")?;
    let coverage_raw = read_str(EVIDENCE_GATE_ID, &fields, "min_coverage")?;
    let block_raw = read_str(EVIDENCE_GATE_ID, &fields, "block_unit")?;
    let gate = EvidenceGate {
        record,
        confidence: parse_fraction(EVIDENCE_GATE_ID, "confidence", &confidence_raw)?,
        block_unit: parse_block_unit(EVIDENCE_GATE_ID, &block_raw)?,
        min_events: read_u32(EVIDENCE_GATE_ID, &fields, "min_events")?,
        min_blocks: read_u32(EVIDENCE_GATE_ID, &fields, "min_blocks")?,
        min_windows: read_u32(EVIDENCE_GATE_ID, &fields, "min_windows")?,
        min_coverage: parse_fraction(EVIDENCE_GATE_ID, "min_coverage", &coverage_raw)?,
    };
    gate.validate()?;
    Ok(gate)
}

pub fn active_evidence_gate() -> Result<EvidenceGate, PolicyError> {
    parse_evidence_gate(Some(ACTIVE_EVIDENCE_GATE_JSON))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn mutate_record(change: impl FnOnce(&mut Value)) -> String {
        let mut root: Value = serde_json::from_str(ACTIVE_EVIDENCE_GATE_JSON).unwrap();
        change(&mut root);
        root.to_string()
    }

    #[test]
    fn active_v1_preserves_adr_values_and_identity() {
        let gate = active_evidence_gate().unwrap();
        assert_eq!(gate.record.label(), "#1677/evidence-gate/v1");
        assert_eq!(gate.confidence, (95, 100));
        assert_eq!(gate.block_unit, BlockUnit::CalendarMonthUTC);
        assert_eq!(
            (gate.min_events, gate.min_blocks, gate.min_windows),
            (30, 12, 3)
        );
        assert_eq!(gate.min_coverage, (1, 2));
        assert!((gate.z_alpha() - 1.645).abs() < 1e-3);
    }

    #[test]
    fn fixed_bars_and_fail_closed_errors() {
        let fixed = mutate_record(|root| {
            root["fields"]["block_unit"]["value"] = "FixedBars1m{43200}".into();
        });
        assert_eq!(
            parse_evidence_gate(Some(&fixed)).unwrap().block_unit,
            BlockUnit::FixedBars1m { bars: 43200 }
        );
        assert_eq!(
            parse_evidence_gate(None),
            Err(PolicyError::RecordMissing {
                id: EVIDENCE_GATE_ID
            })
        );
        let missing = mutate_record(|root| {
            root["fields"].as_object_mut().unwrap().remove("confidence");
        });
        assert!(matches!(
            parse_evidence_gate(Some(&missing)),
            Err(PolicyError::AwaitingPolicy { field, .. }) if field == "confidence"
        ));
        let zero_denominator = mutate_record(|root| {
            root["fields"]["confidence"]["value"] = "95/0".into();
        });
        assert!(matches!(
            parse_evidence_gate(Some(&zero_denominator)),
            Err(PolicyError::Invalid { field, .. }) if field == "confidence"
        ));
        let too_few = mutate_record(|root| root["fields"]["min_events"]["value"] = 1.into());
        assert!(matches!(
            parse_evidence_gate(Some(&too_few)),
            Err(PolicyError::Invalid { field, .. }) if field == "min_events"
        ));
    }
}
