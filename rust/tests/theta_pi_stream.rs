//! #951 ThetaPiStream tracer-bullet + D1 出口行为不变式回归。
//!
//! seam：`ThetaPiStream::push_bar(bar, p_t, nav) -> f64`（返回目标净敞口 p_star，与
//! `recursive_t` 的 `push_bar -> lu−su` 同范畴）。本测试只打这条 seam——
//! 不 mock 内部协作者，不测内部实现；期望值用字面量/工作例，不源自实现同源推导。
//!
//! 断言面（#799 裁定十一：行为不变式 → 测试锁）：
//! 1. `push_bar` 返回有限 f64，且同输入同状态序列 ⟹ 逐位相同（确定性）。
//! 2. `last_order()` 与 `schedule_order(p_star, p_t)` 一致（qty = |round(p_star − p_t)|）。
//! 3. per-leg 账本对账：`reconcile_residual = |account_price_pnl − total_voice_pnl| < 1e-6`；
//!    `level_ledger` 的 LEE-Net 恒等残差恒 0。

use newchan_rust::theta_v0::classifier::consume_at::{ManagedBspPolicy, PolicyRule};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::stream::ThetaPiStream;
use newchan_rust::theta_v0::types::{Bar, StrictAction};

/// 合成锯齿行情（确定性；摆动高低点足够形成分型/笔，不依赖是否形成买卖点）。
fn synthetic_bars() -> Vec<Bar> {
    let mut bars = Vec::with_capacity(80);
    let mut close: i64 = 100_000_000;
    let mut idx = 0usize;
    for swing in 0..16usize {
        let dir: i64 = if swing % 2 == 0 { 1 } else { -1 };
        for _step in 0..5usize {
            let open = close;
            close += dir * 500;
            let high = open.max(close) + 100;
            let low = open.min(close) - 100;
            bars.push(Bar {
                source_index: idx,
                timestamp: idx as i64,
                open,
                high,
                low,
                close,
                volume: 1000.0,
                untradable: false,
            });
            idx += 1;
        }
    }
    bars
}

/// 两条独立流喂同一序列 ⟹ p_star 逐位相同（确定性）。
#[test]
fn push_bar_is_deterministic_and_finite() {
    let mut a = ThetaPiStream::new(ThetaConfig::default());
    let mut b = ThetaPiStream::new(ThetaConfig::default());
    let bars = synthetic_bars();
    let mut p_t = 0.0f64;
    for (i, bar) in bars.iter().enumerate() {
        let pa = a.push_bar(*bar, p_t, 1.0e6);
        let pb = b.push_bar(*bar, p_t, 1.0e6);
        assert!(pa.is_finite(), "bar {i}: p_star 非有限：{pa}");
        assert_eq!(
            pa.to_bits(),
            pb.to_bits(),
            "bar {i}: 两条流同输入 p_star 不一致：{pa} vs {pb}"
        );
        p_t = pa;
    }
}

/// last_order 与 schedule_order(p_star, p_t) 一致；per-leg 账本对账恒等成立。
#[test]
fn last_order_consistent_and_ledger_reconciles() {
    let mut s = ThetaPiStream::new(ThetaConfig::default());
    let bars = synthetic_bars();
    let mut p_t = 0.0f64;
    let mut last_input_p_t = p_t;
    for bar in &bars {
        let p_star = s.push_bar(*bar, p_t, 1.0e6);
        last_input_p_t = p_t;
        p_t = p_star;
    }
    let order = s.last_order();
    // qty 恒 = |round(p_star − p_t)|（Schedule_Θ 全函数契约的构造性检查）。
    assert_eq!(
        order.qty,
        (s.target_net_units() - last_input_p_t).abs().round() as i64,
        "last_order.qty 与 |round(p_star − p_t)| 不一致"
    );
    // action ∈ 七构造子（StrictAction 枚举闭包）。
    assert!(matches!(
        order.action,
        StrictAction::Buy
            | StrictAction::Sell
            | StrictAction::Add
            | StrictAction::Reduce
            | StrictAction::Hold
            | StrictAction::Close
            | StrictAction::Wait
    ));
    // D1 出口 ② per-leg 账本对账（PDF §11 线性恒等）。
    let residual = (s.overlay().account_price_pnl() - s.overlay().total_voice_pnl()).abs();
    assert!(residual < 1e-6, "reconcile_residual={residual} 超 1e-6");
    // LEE-Net 恒等残差恒 0（整数手数求和，非 eps 容差）。
    assert_eq!(s.level_ledger().lee_net_witness().max_abs_residual, 0);
}

/// N7 消费接线（#1097）：`push_bar` 驱动的 E2E-O 装配（准入信号授权形成面）+ `consume_managed`
/// 经 `consume_router` 分桶（执行持有面）是**只读旁路**——不回流决策核，且消费产出满足分桶
/// 不变量与幂等零增量。
///
/// 本测试只锁接线契约（不变量 + 幂等），不锁具体数量——合成锯齿行情可能不形成 Closed 链，
/// 空账本的分桶不变量真空成立、二次 consume 零增量恒成立。
#[test]
fn consume_wiring_read_only_and_routes_by_level() {
    let mut s = ThetaPiStream::new(ThetaConfig::default());
    let bars = synthetic_bars();
    let mut p_t = 0.0f64;
    for (i, bar) in bars.iter().enumerate() {
        let p_star = s.push_bar(*bar, p_t, 1.0e6);
        assert!(p_star.is_finite(), "bar {i}: p_star 非有限：{p_star}");
        p_t = p_star;
    }

    let policy = ManagedBspPolicy {
        rules: vec![PolicyRule {
            rule_id: 1,
            policy_id: 1,
            slot: 0,
        }],
    };
    let (creations, _links, ledger) = s
        .consume_managed(&policy)
        .expect("合法 policy + 单调 prior ⟹ consume 应成功");
    // 分桶不变量：每个受管 BSP 恰好落在其 formation_level 桶。
    assert!(ledger.level_invariant_holds(), "分桶不变量成立");
    // 每个 creation 恰好落一桶（无丢失、无重复）。
    assert_eq!(creations.len(), ledger.total(), "creation 数 == 账本总数");
    // 幂等：跨链 prior 已累积，二次 consume 零增量（同 e2eo 模块单测口径）。
    let (second_creations, second_links, _) =
        s.consume_managed(&policy).expect("二次 consume 应成功");
    assert!(
        second_creations.is_empty() && second_links.is_empty(),
        "二次 consume 零增量（幂等）"
    );
}
