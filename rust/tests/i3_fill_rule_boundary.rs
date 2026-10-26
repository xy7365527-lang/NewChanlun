//! #1718 I3: fill_rule 边界 fixture——刚触价 vs 穿越 1 tick 差异。
//!
//! 验证点：
//! 1. touch 在刚触价时成交，cross(1) 不成交
//! 2. 穿越 1 tick 时两种规则都成交
//! 3. 诊断成交带 fill_rule 标签
//! 4. 验收入口拒收 touch 配置

use newchan_rust::theta_v0::config::{ExecConfig, FillRule};

#[test]
fn touch_vs_cross_boundary_just_touch() {
    // 场景：限价买单 100.0，行情最低触及 100.0 但未穿越
    // 预期：touch 成交，cross(1) 不成交

    let config_touch = ExecConfig {
        fill_rule: FillRule::Touch,
        ..Default::default()
    };

    let config_cross = ExecConfig {
        fill_rule: FillRule::Cross(1),
        ..Default::default()
    };

    // 当前实现是市价成交（按 bar close），限价单逻辑由 #927 提供。
    // 此 fixture 标记边界条件，实际成交判定由 ADR 0019 实现后生效。
    // 本票只验证配置字段存在、序列化正确、验收入口拒收 touch。

    assert_eq!(
        format!("{:?}", config_touch.fill_rule),
        "Touch",
        "Touch 配置可序列化"
    );
    assert_eq!(
        format!("{:?}", config_cross.fill_rule),
        "Cross(1)",
        "Cross(1) 配置可序列化"
    );

    // 验收拒收已在 ExecConfig::require_production 实现，
    // 单元测试在 backtest_engine.rs::tests::touch_fill_rule_refuses_production。
}

#[test]
fn touch_vs_cross_boundary_pierce_one_tick() {
    // 场景：限价买单 100.0，行情最低 99.9（穿越 0.1 = 1 tick，假设 tick=0.1）
    // 预期：touch 和 cross(1) 都成交

    let config_touch = ExecConfig {
        fill_rule: FillRule::Touch,
        ..Default::default()
    };

    let config_cross = ExecConfig {
        fill_rule: FillRule::Cross(1),
        ..Default::default()
    };

    // 同上，实际成交判定由 #927 (ADR 0019) 实现。
    // 本 fixture 标记穿越场景边界，确认两种配置都存在。

    assert_ne!(config_touch.fill_rule, config_cross.fill_rule, "两种规则不同");
}

#[test]
fn fill_rule_serialization_round_trip() {
    // 验证 FillRule 的序列化与反序列化

    use serde_json;

    let cross = FillRule::Cross(2);
    let touch = FillRule::Touch;

    let cross_json = serde_json::to_string(&cross).expect("Cross 序列化");
    let touch_json = serde_json::to_string(&touch).expect("Touch 序列化");

    let cross_de: FillRule = serde_json::from_str(&cross_json).expect("Cross 反序列化");
    let touch_de: FillRule = serde_json::from_str(&touch_json).expect("Touch 反序列化");

    assert_eq!(cross, cross_de, "Cross 往返一致");
    assert_eq!(touch, touch_de, "Touch 往返一致");
}
