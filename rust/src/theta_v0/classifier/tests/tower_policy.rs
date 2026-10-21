//! #1603（#1586 C-09/C-10）：级别塔预算与 MACD 力度记录的输出戳、资源截断标记。

use super::super::*;
use super::fixtures::*;
use crate::theta_v0::policy_records::{
    level_config_from, parse_macd_force, parse_tower_level_budget, PolicyVersion,
};

const TOWER_V2_L0: &str =
    r#"{"id":"tower-level-budget","version":2,"fields":{"l_max":{"value":0}}}"#;
const MACD_V2: &str = r#"{"id":"macd-force","version":2,"fields":{"fast":{"value":5},"slow":{"value":35},"signal":{"value":5}}}"#;

fn config_with_tower(text: &str) -> ThetaConfig {
    let mut cfg = ThetaConfig::default();
    cfg.level = level_config_from(parse_tower_level_budget(Some(text)).unwrap(), 3);
    cfg
}

/// 等值记录：默认配置即生效记录 v1，输出写 v1 戳；该夹具到顶前自然终止。
#[test]
fn active_records_stamp_v1_and_natural_termination() {
    let cfg = ThetaConfig::try_from_active_records().expect("生效记录可解析");
    assert_eq!(cfg, ThetaConfig::default(), "fallible 入口与 Default 同值");
    let out = classify(&operation_oscillating_layer(), &cfg, &[]);
    assert_eq!(out.policy.level_budget.label(), "tower-level-budget/1");
    assert_eq!(out.policy.macd.label(), "macd-force/1");
    assert_eq!(out.termination, TowerTermination::Natural);
    assert_eq!(out.termination.code(), "NATURAL");
    assert!(!out.is_resource_truncated());
}

/// 专项验收①：在 l_max 之前仍可升级的输入，截断后输出带 INCOMPLETE_RESOURCE_BOUND，
/// 与同输入自然终止的读数可区分。变异体（读回常数 6）下本测试变红：l_max=6 时该夹具自然终止。
#[test]
fn reaching_l_max_with_upgradable_units_marks_resource_bound() {
    let layer = operation_oscillating_layer();
    let natural = classify(&layer, &ThetaConfig::default(), &[]);
    assert!(natural.classification.levels.len() >= 2, "夹具须能构到 L1");
    assert_eq!(natural.termination, TowerTermination::Natural);

    let cfg = config_with_tower(TOWER_V2_L0);
    let truncated = classify(&layer, &cfg, &[]);
    assert_eq!(truncated.classification.levels.len(), 1, "l_max=0 只构 L0");
    match truncated.termination {
        TowerTermination::ResourceBound {
            l_max,
            pending_units,
            budget,
        } => {
            assert_eq!(l_max, 0);
            assert!(pending_units >= 3, "下一级仍有 ≥ min_parts 个单元可升级");
            assert_eq!(budget.label(), "tower-level-budget/2");
        }
        other => panic!("期望资源截断，得 {other:?}"),
    }
    assert_eq!(truncated.termination.code(), "INCOMPLETE_RESOURCE_BOUND");
    assert!(truncated.is_resource_truncated());
    assert_eq!(
        truncated.policy.level_budget.label(),
        "tower-level-budget/2"
    );
}

/// 专项验收②：改 MACD 记录版本 ⟹ 输出版本号随之变化；同版本两次运行逐位相同。
#[test]
fn macd_record_version_flows_to_output_and_reruns_are_bit_identical() {
    let layer = operation_oscillating_layer();
    let base = ThetaConfig::default();
    let a = classify(&layer, &base, &[0, 1]);
    let b = classify(&layer, &base, &[0, 1]);
    assert_eq!(a, b, "同版本两次运行逐位相同");

    let mut cfg2 = ThetaConfig::default();
    cfg2.macd = parse_macd_force(Some(MACD_V2)).unwrap();
    let c = classify(&layer, &cfg2, &[0, 1]);
    let d = classify(&layer, &cfg2, &[0, 1]);
    assert_eq!(c, d, "v2 同版本两次运行逐位相同");
    assert_eq!(c.policy.macd.label(), "macd-force/2");
    assert_ne!(a.policy, c.policy, "改记录版本 ⟹ 输出戳变化");
    assert_eq!(
        (cfg2.macd.fast, cfg2.macd.slow, cfg2.macd.signal),
        (5, 35, 5)
    );
    // 计算端确实读记录参数：忽略 cfg、写死 12/26/9 的变异体在此变红。
    let closes: Vec<f64> = (0..80)
        .map(|i| 100.0 + 7.0 * ((i as f64) * 0.37).sin())
        .collect();
    let h1 = divergence::compute_macd(&closes, &base.macd).hist;
    let h2 = divergence::compute_macd(&closes, &cfg2.macd).hist;
    assert_ne!(h1, h2, "MACD v2 参数须改变力度读数");
}

/// 研究覆写不冒充签署版本。
#[test]
fn research_override_is_labelled() {
    let mut cfg = ThetaConfig::default();
    cfg.level.research_override_l_max(8);
    assert_eq!(cfg.level.budget.version, PolicyVersion::ResearchOverride);
    let out = classify(&operation_oscillating_layer(), &cfg, &[]);
    assert_eq!(
        out.policy.level_budget.label(),
        "tower-level-budget/research-override"
    );
}
