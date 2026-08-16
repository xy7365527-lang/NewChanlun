//! #648 T1 主题件：third_class_anchor（自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。

use super::super::super::types::Direction;
use super::super::*;
use super::fixtures::*;

/// ★ADR 补充十三 / #486 / Spec #485：L≥1 一/三类的方向锚均取单元结构方向。
/// provenance endpoint fallback（anchor=None）仍是序列成员，不再阻断几何合法的一/三类。
/// 一类 p117 直调契约由 signal.rs 的
/// `judge_first_cached_provenance_gate_preserved_for_direct_callers` 独立锁定；三类的方向匹配、
/// 回试方向、严格 `>ZG/<ZD` 与 OwnerRef 契约均不变。
#[test]
fn q7_ruling_c_first_and_third_class_structural_direction_authorized() {
    use super::super::center::UnitRange;
    // fixture 同 level_ge1_extract_first_third_fills_type1_gap（两下行中枢 + A/B/C 三单元）。
    let c0 = Center {
        zd: 300,
        zg: 400,
        dd: 290,
        gg: 410,
        start_index: 0,
        end_index: 2,
    };
    let c1 = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 8,
    };
    let units = vec![
        UnitRange {
            start_index: 3,
            end_index: 5,
            direction: Direction::Down,
            lo: 250,
            hi: 350,
        },
        UnitRange {
            start_index: 5,
            end_index: 7,
            direction: Direction::Up,
            lo: 250,
            hi: 280,
        },
        UnitRange {
            start_index: 9,
            end_index: 11,
            direction: Direction::Down,
            lo: 80,
            hi: 150,
        },
        UnitRange {
            start_index: 11,
            end_index: 13,
            direction: Direction::Up,
            lo: 80,
            hi: 90,
        }, // #607 D2：T3-in-c 固定首对 retest（仍 < zd=100）
    ];
    let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
    let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
    let close_src: Vec<usize> = (0..prices.len()).collect();
    let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
    // 一类只锁 L≥1 调用层的结构锚授权：helper 会重建结构锚，故不在这里伪造 provenance
    // 变体。p117 的 provenance 直调契约由 signal.rs 上述独立测试锁定。
    let (first_bsp, _) = extract_first_third_for_level(
        &[c0, c1],
        &units,
        &[None, None, None, None],
        &hist,
        &[],
        &[],
        &close_src,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    assert_eq!(
        first_bsp.iter().filter(|p| p.bits.buy1).count(),
        1,
        "#486：L≥1 一类按结构方向锚产一买"
    );
    // 三类：ADR 补充十三 / #486 / Spec #485 授权 L≥1 leave 使用结构方向锚；
    // provenance fallback=None 不再否决三买，retest 仍是几何角色、不另设锚门。
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 12,
    };
    let u3 = vec![
        UnitRange {
            start_index: 12,
            end_index: 16,
            direction: Direction::Up,
            lo: 150,
            hi: 250,
        },
        UnitRange {
            start_index: 16,
            end_index: 20,
            direction: Direction::Down,
            lo: 210,
            hi: 250,
        },
    ];
    let src24: Vec<usize> = (0..24).collect();
    let (bsp, _) = extract_first_third_for_level(
        &[c],
        &u3,
        &[None, Some(Direction::Down)],
        &[],
        &[],
        &[],
        &src24,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
    assert_eq!(
        buy3.len(),
        1,
        "#486：provenance fallback=None 时，L≥1 仍按结构方向产三买"
    );
    assert_eq!(
        buy3[0].center,
        Some(signal::OwnerRef::Center(c)),
        "三类 OwnerRef 仍精确指向所破中枢"
    );
    // provenance 是否有值不再改变同一结构几何的三类输出。
    let (bsp2, _) = extract_first_third_for_level(
        &[c],
        &u3,
        &[Some(Direction::Up), Some(Direction::Down)],
        &[],
        &[],
        &[],
        &src24,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    assert_eq!(bsp2, bsp);
}

#[test]
fn issue486_level_ge1_third_rejects_structural_direction_geometry_mismatch() {
    use super::super::center::UnitRange;
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 12,
    };
    let units = vec![
        // 价格位于上方，但结构方向是 Down；旧 provenance=Up 不得越权把它认作向上离开。
        UnitRange {
            start_index: 12,
            end_index: 16,
            direction: Direction::Down,
            lo: 250,
            hi: 260,
        },
        UnitRange {
            start_index: 16,
            end_index: 20,
            direction: Direction::Down,
            lo: 210,
            hi: 250,
        },
    ];
    let src24: Vec<usize> = (0..24).collect();
    let (bsp, _) = extract_first_third_for_level(
        &[c],
        &units,
        &[Some(Direction::Up), Some(Direction::Down)],
        &[],
        &[],
        &[],
        &src24,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    assert!(
        bsp.iter().all(|p| !p.bits.buy3 && !p.bits.sell3),
        "结构方向与三买几何方向不符时必须拒绝，provenance 不得覆盖结构事实"
    );
}

#[test]
fn issue486_level_ge1_third_rejects_retest_equal_center_edge() {
    use super::super::center::UnitRange;
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 12,
    };
    let src24: Vec<usize> = (0..24).collect();
    let buy_equal = vec![
        UnitRange {
            start_index: 12,
            end_index: 16,
            direction: Direction::Up,
            lo: 150,
            hi: 250,
        },
        UnitRange {
            start_index: 16,
            end_index: 20,
            direction: Direction::Down,
            lo: 200,
            hi: 250,
        },
    ];
    let sell_equal = vec![
        UnitRange {
            start_index: 12,
            end_index: 16,
            direction: Direction::Down,
            lo: 50,
            hi: 150,
        },
        UnitRange {
            start_index: 16,
            end_index: 20,
            direction: Direction::Up,
            lo: 50,
            hi: 100,
        },
    ];
    for units in [&buy_equal, &sell_equal] {
        let (bsp, _) = extract_first_third_for_level(
            &[c],
            units,
            &[None, None],
            &[],
            &[],
            &[],
            &src24,
            divergence::DivergenceGauge::MacdArea,
            &[],
            &mut Vec::new(),
        );
        assert!(
            bsp.iter().all(|p| !p.bits.buy3 && !p.bits.sell3),
            "retest==ZG/ZD 仍触及闭区间中枢，必须严格拒绝"
        );
    }
}

#[test]
fn issue486_l0_third_output_fields_unchanged() {
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 12,
    };
    let segs = vec![
        Segment {
            direction: Direction::Up,
            start_index: 12,
            end_index: 16,
            start_price: 150,
            end_price: 250,
        },
        Segment {
            direction: Direction::Down,
            start_index: 16,
            end_index: 20,
            start_price: 250,
            end_price: 210,
        },
    ];
    let src24: Vec<usize> = (0..24).collect();
    let (bsp, pan) = signal::extract_signals_with_hist(
        &[c],
        &segs,
        &[],
        &[],
        &[],
        &src24,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    assert!(pan.is_empty());
    assert_eq!(bsp.len(), 1);
    let p = &bsp[0];
    assert_eq!(p.source_index, 20);
    assert!(!p.bits.buy1);
    assert!(!p.bits.buy2);
    assert!(p.bits.buy3);
    assert!(!p.bits.sell1);
    assert!(!p.bits.sell2);
    assert!(!p.bits.sell3);
    assert_eq!(p.pivot_low, 210);
    assert_eq!(p.pivot_high, 0);
    assert_eq!(p.center, Some(signal::OwnerRef::Center(c)));
    assert_eq!(p.struct_break_dir, None);
    assert!(p.force.is_none());
}

/// ★#905 / #816 B-3 锁（020:62【正文】「必须是第一次」= 判据级必要条件）：同一中枢的
/// **第二次**回试不置三类 bit；首对价格失败（重回中枢）后，远期合法相邻对**不得回填**。
/// 语义锚 = Lean `BspClassification.IsType3Sell.firstRetrace`。
#[test]
fn third_class_first_retrace_only_lock() {
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 4,
    };
    let seg = |dir, si, ei, sp, ep| crate::theta_v0::types::Segment {
        direction: dir,
        start_index: si,
        end_index: ei,
        start_price: sp,
        end_price: ep,
    };
    // 场景 A：首对成功（卖3 于首回试）+ 第二次回试同样合法 ⟹ 只产一枚。
    let segs_a = vec![
        seg(Direction::Down, 4, 6, 150, 80), // leave 破 ZD=100
        seg(Direction::Up, 6, 8, 80, 95),    // 第一次回试：95 < 100 不入 ⟹ sell3
        seg(Direction::Down, 8, 10, 95, 82), // 再次离开
        seg(Direction::Up, 10, 12, 82, 98),  // 第二次回试：98 < 100 仍不入 ⟹ **不得**置 sell3
    ];
    let pts_a = signal::extract_signals(
        &[c],
        &segs_a,
        &[],
        &[],
        &crate::theta_v0::config::MacdConfig::default(),
    );
    let sell3_a: Vec<_> = pts_a.iter().filter(|p| p.bits.sell3).collect();
    assert_eq!(
        sell3_a.len(),
        1,
        "B-3：仅第一次回试置三类（第二回试不回填）；实得 {pts_a:?}"
    );

    // 场景 B：首对价格失败（回试 105 ≥ ZD 重回）⟹ 首对不产；其后合法相邻对同样不得回填。
    let segs_b = vec![
        seg(Direction::Down, 4, 6, 150, 80),
        seg(Direction::Up, 6, 8, 80, 105), // 第一次回试重回中枢 ⟹ 非三类
        seg(Direction::Down, 8, 10, 105, 82),
        seg(Direction::Up, 10, 12, 82, 98), // 远期合法对 ⟹ 仍不得置 sell3
    ];
    let pts_b = signal::extract_signals(
        &[c],
        &segs_b,
        &[],
        &[],
        &crate::theta_v0::config::MacdConfig::default(),
    );
    assert!(
        !pts_b.iter().any(|p| p.bits.sell3),
        "B-3：首对失败后远期对不回填（「必须是第一次」）；实得 {pts_b:?}"
    );
}
