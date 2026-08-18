//! #648 T1 主题件：pipeline_geometry（自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。

use super::super::super::parser::ParseLayer;
use super::super::super::types::{Direction, MoveKind};
use super::super::*;
use super::fixtures::*;

#[test]
fn p52_frontier_diagnostics_are_level_scoped_and_resettable() {
    cp_replay_diagnostics::enable();
    cp_replay_diagnostics::record_dirty_invalidation(2, 3, 5);
    cp_replay_diagnostics::record_tail_reinherit(2);
    cp_replay_diagnostics::record_tail_reinherit(1);

    let counters = cp_replay_diagnostics::snapshot();
    assert_eq!(counters[1].tail_reinherits, 1);
    assert_eq!(counters[2].pending_fallbacks, 3);
    assert_eq!(counters[2].tail_reinherits, 1);
    assert_eq!(counters[2].certificate_clear_recomputes, 5);

    cp_replay_diagnostics::disable();
    assert!(cp_replay_diagnostics::snapshot().is_empty());
}

/// ★端到端 B2 真产出（#53 验证门，L1 管线正确性）：升级后的递归塔（`RMove::Compose` 携 subs）
/// 让 `extract_second_signals` **真接入生产路径**——classify 在真实结构输入上产出 B2 买点。
///
/// 旧塔（`UnitRange` 无 subs）在**任何**输入上产 0 个 B2（descend 得空，结构上不可产）；新塔
/// 在此输入上产 1 个 B2，坐实升级解除了 still-MISSING-塔。
///
/// 路径（codex 异质裁决确认）：B2 在 **L1→L2 几何路径**产出——3 个同向 L1 走势经几何窗口
/// （不强制方向交替）compose 成 L2 走势，其 descend 取回的 3 个 L1 走势内识别第二类结构：
/// L1[1]（i1=1）破 L2 中枢 + MACD 背驰（相对前同向 L1[0]）= 第一类离开；L1[2]（i2=2）回拉不
/// 创新低 = 第二类回拉走势。B2 端点 = L1[2] 的回拉结束点（坐标由 source_index 侧车真映射）。
///
/// ★诚实边界（still-MISSING-窗口，codex 裁决坐实）：B2/S2 **不在 L0→L1 三段交替窗口产**——
/// 三段方向交替窗口里可背驰的同向段只在位置 2（末段，无后继回拉），位置 0 无前同向对照，
/// 位置 1 是唯一异向（无前同向）。这是固定三段封装的结构上界，非接入缺陷（接入逻辑双侧完整）。
#[test]
fn end_to_end_second_buy_via_l1_l2_geometric() {
    let cfg = ThetaConfig::default();
    // 9 段 L0：三组（每组 → 一个 L1 走势）。★中枢延伸语义下的诚实重算（PDF §5，task #142）：
    // 组间首段必须与前组**冻结核心 [ZD,ZG]** 不相交（Step3 non-extension），否则整串被 Step2
    // 吸收为 1 个延伸中枢 ⟹ 塔不生长（旧全触及 fixture 的坍缩后果）。推导：
    // - 组A up-down-up：核心 K_A=[max(110,120,120),min(150,150,148)]=[120,148]，外缘 O_A=[110,150]。
    // - 组B down-up-down：首段 [80,115] hi=115 < ZD_A=120 ⟹ non-extension（组间分离）；
    //   核心 K_B=[max(80,80,85),min(115,125,114)]=[85,114]，外缘 O_B=[80,125]。
    // - 组C up-down-up：首段 [115,148] lo=115 > ZG_B=114 ⟹ non-extension；
    //   核心 K_C=[max(115,112,112),min(148,148,147)]=[115,147]，外缘 O_C=[112,148]。
    // L2 核心（几何路径，三 L1 外缘交）= [max(110,80,112), min(150,125,148)] = [112,125] 非空。
    // B2 结构：L1[1].lo=80 < ZD2=112 深破 L2 核心下沿（第一类离开候选，Side::Long）；
    // L1[2] 回拉不创新低（lo=112 >= L1[1].lo=80）；L1[0]/L1[1] 外缘占位方向同 Down
    // （末子 hi < 首子 hi：148<150 / 114<115）⟹ 背驰可配对（closes 前大后小）。
    let segments = vec![
        // 组A（L1[0]）：up-down-up，核心 [120,148]，外缘 [110,150]
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        // 组B（L1[1]）：down-up-down，首段 hi=115<ZD_A=120 non-ext，lo=80 深破 L2 核心下沿 112
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        // 组C（L1[2]）：up-down-up，首段 lo=115>ZG_B=114 non-ext，回拉不创新低（lo=112 >= 80）
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147),
    ];
    // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
    // 前段大幅波动（面积大），后段小幅（面积小）。
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    } // L1[0] 大幅
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    } // L1[1] 小幅（背驰）
    for i in 0..16 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    } // L1[2] 更小
    let layer = ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;

    // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
    assert!(
        out.levels.len() >= 2,
        "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级"
    );
    let l1 = &out.levels[1];
    assert_eq!(
        l1.centers.len(),
        1,
        "3 个 L1 走势 → 1 个 L2 中枢（几何路径）"
    );
    let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
    assert_eq!(
        second_buys.len(),
        1,
        "★升级后塔真产 B2（旧 UnitRange 塔产 0）"
    );
    let b2 = second_buys[0];
    // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
    assert_eq!(
        b2.source_index, 36,
        "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）"
    );
    // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）——止损仍 pivot 非 center.zg/zd。
    assert!(
        b2.pivot_low != 0,
        "B2 携结构止损价 pivot_low（回拉低点 single source）"
    );
    // ★#218 面 A（spec owner-attribution-fix-20260724 ID-1，机械改写归因：载体形态变化）：
    // 二类点归属载体从判定中枢 c1（次级别中枢，确认层对象）改载该走势一类点身份锚——
    // 第一类离开走势 m1（L1[1]，背驰次级别走势）的终点坐标（区间套：该走势终点极值点 =
    // 一类点）；止损仍 pivot（上条已锁，止损语义不变）。
    assert_eq!(
        b2.center,
        Some(signal::OwnerRef::Type1Anchor(24)),
        "二类点归属载体 = 该走势一类点锚（m1=L1[1] 终点坐标 24）"
    );
    // 互斥语义：B2 端点不置 1/3 类 bit。
    assert!(!b2.bits.buy1 && !b2.bits.buy3, "第二类端点不置 1/3 类 bit");
}

/// ★still-MISSING-窗口边界（codex 异质裁决坐实，编码为可执行断言，formalization-validity-domain）：
/// L0→L1 的三段方向交替窗口**结构上不产 B2/S2**——三段交替里可背驰的同向段只在位置 2（末段，
/// 无后继回拉），位置 0 无前同向对照，位置 1 是唯一异向（无前同向）。故单个 L1 走势的 3 段 L0
/// subs 内识别不出「第一类离开（破中枢∧背驰）+ 后继回拉」。
///
/// 此断言锁定边界：L0 级别（索引 0）的 bsp **不含 B2/S2**（B2/S2 由 L1→L2 几何路径产，见
/// `end_to_end_second_buy_via_l1_l2_geometric`）。这是固定三段封装的结构上界，非补丁——放松
/// 背驰约束或窗口大小来强产 L0 层 B2 = 声明膨胀（no-patch 禁止）。
#[test]
fn l0_level_emits_no_second_class_window_bound() {
    let cfg = ThetaConfig::default();
    // 简单 up-down-up 三段（一个 L0 中枢，一个 L1 走势）——L1 走势 3 段 subs 内无法产 B2/S2。
    let segments = vec![
        seg(Direction::Up, 0, 4, 100, 200),
        seg(Direction::Down, 4, 8, 200, 100),
        seg(Direction::Up, 8, 12, 100, 200),
    ];
    let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
    let layer = ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
    for p in out.levels[0].bsp.iter() {
        assert!(
            !p.bits.buy2,
            "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）"
        );
        assert!(
            !p.bits.sell2,
            "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）"
        );
    }
}

/// ★codex-decide-20260703 裁定 A 最大实现风险点（end_price 忠实性，单测强制）：级别-N 输入单元
/// → Segment 的端点价按 fold_direction 取 hi/lo，且与 `segment_to_unit` 互逆（L0 段 round-trip
/// bit-exact）。此测试失败 ⟹ 级别-N「线段」端点价错位 ⟹ A/C 破中枢几何 + judge_third 判据全错。
#[test]
fn unit_to_segment_endpoint_faithful_and_roundtrips() {
    use super::super::center::UnitRange;
    // 向上单元：起点=lo、终点=hi（seg_end 取 end_price=hi=高点）。
    let up = UnitRange {
        start_index: 4,
        end_index: 8,
        direction: Direction::Up,
        lo: 90,
        hi: 150,
    };
    let s_up = unit_to_segment(&up);
    assert_eq!(
        (s_up.start_price, s_up.end_price),
        (90, 150),
        "向上单元 end_price=hi（终点=高点）"
    );
    assert_eq!(s_up.direction, Direction::Up);
    assert_eq!(
        (s_up.start_index, s_up.end_index),
        (4, 8),
        "source_index 坐标保留（A/C 面积映射用）"
    );
    // 向下单元：起点=hi、终点=lo（终点=低点）。
    let down = UnitRange {
        start_index: 8,
        end_index: 12,
        direction: Direction::Down,
        lo: 90,
        hi: 150,
    };
    let s_down = unit_to_segment(&down);
    assert_eq!(
        (s_down.start_price, s_down.end_price),
        (150, 90),
        "向下单元 end_price=lo（终点=低点）"
    );
    // round-trip：L0 段 → segment_to_unit → unit_to_segment == 原段（互逆 bit-exact）。
    for orig in [
        seg(Direction::Up, 0, 4, 100, 200),
        seg(Direction::Down, 4, 8, 200, 50),
    ] {
        let back = unit_to_segment(&segment_to_unit(&orig));
        assert_eq!(
            (
                back.direction,
                back.start_index,
                back.end_index,
                back.start_price,
                back.end_price
            ),
            (
                orig.direction,
                orig.start_index,
                orig.end_index,
                orig.start_price,
                orig.end_price
            ),
            "segment_to_unit ∘ unit_to_segment = id（round-trip bit-exact）"
        );
    }
}

/// ★裁定 A gap-fill 非 no-op（结构合法性）：`extract_first_third_for_level` 在级别-N 下跌趋势
/// units（承担线段角色）+ ≥2 依次向下中枢（Trend(Down)）+ C 段破最后中枢 + C<A 背驰上产 1 买。
/// 复用 signal.rs `first_buy_extracted_with_trend_divergence` 的 A/B/C 几何，但以 UnitRange 表达
/// ——证明级别-N 一/三类判定真接线（旧 `else { Vec::new() }` 恒产 0，此测试产 1 = 缺口已填）。
#[test]
fn level_ge1_extract_first_third_produces_type1_via_units() {
    use super::super::center::UnitRange;
    // 两依次向下中枢（c1.gg=210 < c0.dd=290 ⟹ DownContinuation ⟹ trend_class=Trend(Down)）。
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
    // Down 单元：lo=终点价、hi=起点价（unit_to_segment 还原 start=hi/end=lo）。
    let units = vec![
        UnitRange {
            start_index: 3,
            end_index: 5,
            direction: Direction::Down,
            lo: 250,
            hi: 350,
        }, // A 段（C0 离开）
        UnitRange {
            start_index: 5,
            end_index: 7,
            direction: Direction::Up,
            lo: 250,
            hi: 280,
        }, // B 段连接
        UnitRange {
            start_index: 9,
            end_index: 11,
            direction: Direction::Down,
            lo: 80,
            hi: 150,
        }, // C 段破 C1（<100）
        UnitRange {
            start_index: 11,
            end_index: 13,
            direction: Direction::Up,
            lo: 80,
            hi: 90,
        }, // #607 D2：T3-in-c 固定首对 retest（仍 < zd=100）
    ];
    // A 段 bar[3,5] 急跌（hist 面积大）、C 段 bar[9,11] 缓动（面积小=背驰）——同 signal.rs fixture。
    let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
    let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
    let close_src: Vec<usize> = (0..prices.len()).collect();
    let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
    // 本测试只验结构六 bit（force 旁挂不改），传空 dif/closes_tick ⟹ force=None（不影响 buy1 判据）。
    // Q7-#1 裁定C：显式全锚（本测试验证的是 Trend ownership 单元的 gap-fill 路径）。
    let anchors = [
        Some(Direction::Down),
        Some(Direction::Up),
        Some(Direction::Down),
        Some(Direction::Up),
    ];
    let (bsp, _pan) = extract_first_third_for_level(
        &[c0, c1],
        &units,
        &anchors,
        &hist,
        &[],
        &[],
        &close_src,
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    let buy1: Vec<_> = bsp.iter().filter(|p| p.bits.buy1).collect();
    assert_eq!(
        buy1.len(),
        1,
        "级别-N 下跌趋势 C 段破最后中枢 ∧ C<A 背驰 ⟹ 一个 1 买（缺口已填，非 no-op）"
    );
    assert_eq!(
        buy1[0].source_index, 11,
        "1 买端点 = C 段（破最后中枢单元）终止 source_index"
    );
    assert_eq!(
        buy1[0].pivot_low, 80,
        "1 买止损源 = pivot_low（C 段破中枢端点极值）"
    );
    // ★owner 载体补齐（关③ 补记② 路径 (a)）：一类点构造时填入判定中枢 last_center=c1
    //（被破的最后中枢）——名实一致根据同 signal.rs `first_buy_extracted_with_trend_divergence`
    //（本测试复用其 A/B/C 几何的 UnitRange 表达）；center 是 owner 载体，止损仍 pivot
    //（pivot_low=80 上条已锁，center 不进 1/2 类止损判据）。
    // （#218 面 A 载体形态机械适配：一/三类载 OwnerRef::Center，语义不动。）
    assert_eq!(
        buy1[0].center,
        Some(signal::OwnerRef::Center(c1)),
        "一类点 center = 判定中枢（owner 载体）；止损仍 pivot 非 center"
    );
}

/// ★裁定 A 三类（高级别「中枢外缘区间」边界语义，codex 风险点单独 snapshot）：级别-N 离开中枢
/// + 回试不重入 ⟹ 3 买。`judge_third` 在级别-N units（外缘区间端点 hi/lo）vs 几何中枢 [zd,zg]
/// 上判定——离开单元终点 > c.zg ∧ 回试单元终点 > c.zg（严格不触闭区间）。
#[test]
fn level_ge1_extract_first_third_produces_type3_via_units() {
    use super::super::center::UnitRange;
    // 单中枢 [100,200]（盘整 τ ⟹ 无一类）——三类是纯几何位置判据，不依赖趋势门控。
    let c = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 0,
        end_index: 12,
    };
    let units = vec![
        // 离开单元：向上，终点=hi=250 > zg=200（离开中枢上方）。
        UnitRange {
            start_index: 12,
            end_index: 16,
            direction: Direction::Up,
            lo: 150,
            hi: 250,
        },
        // 回试单元：向下，终点=lo=210 > zg=200（不重入闭区间中枢）⟹ 3 买。
        UnitRange {
            start_index: 16,
            end_index: 20,
            direction: Direction::Down,
            lo: 210,
            hi: 250,
        },
    ];
    // 三类无 MACD 依赖（纯整数几何），hist 空亦可——传空 hist/dif/closes_tick（第一类自然不产，force=None）。
    // Q7-#1 裁定C：显式全锚（leave 单元有 Trend ownership 资格的三类路径）。
    let anchors = [Some(Direction::Up), Some(Direction::Down)];
    let (bsp, _pan) = extract_first_third_for_level(
        &[c],
        &units,
        &anchors,
        &[],
        &[],
        &[],
        &(0..24).collect::<Vec<_>>(),
        divergence::DivergenceGauge::MacdArea,
        &[],
        &mut Vec::new(),
    );
    let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
    assert_eq!(
        buy3.len(),
        1,
        "级别-N 离开中枢 + 回试不重入 ⟹ 一个 3 买（外缘区间端点判据）"
    );
    assert_eq!(
        buy3[0].source_index, 20,
        "3 买端点 = 回试单元终止 source_index"
    );
    assert_eq!(buy3[0].pivot_low, 210, "3 买止损源 = pivot_low（回试低点）");
    assert_eq!(
        buy3[0].center.and_then(|o| match o {
            signal::OwnerRef::Center(c) => Some(c.zg),
            _ => None,
        }),
        Some(200),
        "3 买 center=Some（止损=zg；#218 面 A 载体形态：Center 变体读出）"
    );
}

/// ★L2 信号普查诊断（裁定 A gap-fill 真实数据核验，`--ignored` 手动跑，依赖 analysis/data_cache）：
/// 逐级别统计 BTC 全量分类的 buy1/sell1/buy2/sell2/buy3/sell3 计数 + 抽样 level≥1 一类端点。
/// 修前 level≥1 一/三类恒 0（audit #119 `else{Vec::new()}`），故 level≥1 的 type1/type3 计数 =
/// 本次实装引入的净增信号。运行：`cargo test --lib -- --ignored --nocapture level_signal_census_btc`。
#[test]
#[ignore = "L2 真实数据普查：cargo test --lib -- --ignored --nocapture level_signal_census_btc"]
fn level_signal_census_btc() {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::parser::parse_layer;
    let cfg = ThetaConfig::default();
    let full =
        load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    // 可选日期窗（CENSUS_WINDOW="2020-10-01,2021-04-01"）——检验 type1 的水平线依赖性：
    // 全历史中枢链全局非单调 ⟹ trend_class=Degenerate ⟹ type1=0；单向牛/熊窗内某级链可单调 ⟹ type1>0。
    let ds = match std::env::var(crate::theta_v0::env_registry::CENSUS_WINDOW) {
        Ok(w) => {
            let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
            eprintln!("[census] window={s}..{e}");
            full.slice_date_window(s, e)
        }
        Err(_) => full,
    };
    eprintln!("[census] BTC bars={}", ds.bars.len());
    let layer = parse_layer(&ds.bars, &cfg);
    eprintln!(
        "[census] L0 segments={} merged_bars={}",
        layer.segments.len(),
        layer.merged_bars.len()
    );
    let out = classify(&layer, &cfg, &[]).classification;
    eprintln!("[census] levels={}", out.levels.len());
    for (li, lv) in out.levels.iter().enumerate() {
        let trend = lv
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Trend)
            .count();
        let (mut b1, mut s1, mut b2, mut s2, mut b3, mut s3) = (0, 0, 0, 0, 0, 0);
        // ★#884/#816 B-2② 实测对照：「跌破一买」类二类点计数——旧 `no_new_low` 硬闸下恒 0
        // （结构上不产），拆闸后经 `retrace_breaks_type1=Some(true)` 重合标注可查（语义归 #817）。
        let (mut b2_broke, mut s2_broke) = (0usize, 0usize);
        for p in lv.bsp.iter() {
            b1 += p.bits.buy1 as usize;
            s1 += p.bits.sell1 as usize;
            b2 += p.bits.buy2 as usize;
            s2 += p.bits.sell2 as usize;
            b3 += p.bits.buy3 as usize;
            s3 += p.bits.sell3 as usize;
            match p.retrace_breaks_type1 {
                Some(true) if p.bits.buy2 => b2_broke += 1,
                Some(true) if p.bits.sell2 => s2_broke += 1,
                _ => {}
            }
        }
        eprintln!(
                "[census] L{li}: centers={} moves={}(trend={}) bsp={} pan_div={} | buy1={b1} sell1={s1} buy2={b2} sell2={s2} buy3={b3} sell3={s3} | b2_broke={b2_broke} s2_broke={s2_broke}",
                lv.centers.len(), lv.moves.len(), trend, lv.bsp.len(), lv.pan_div.len()
            );
        // 抽样：level≥1 的前 3 个一类端点（若有）+ 前 3 个三类端点（人工核结构合法性——
        // source_index + center[zd,zg] + pivot（回试端点极值）；三类不重入判据由 judge_third 保证）。
        if li >= 1 {
            let t1: Vec<_> = lv
                .bsp
                .iter()
                .filter(|p| p.bits.buy1 || p.bits.sell1)
                .take(3)
                .collect();
            for (k, p) in t1.iter().enumerate() {
                eprintln!(
                        "[census]   L{li} type1#{k}: src_idx={} buy1={} sell1={} break_dir={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy1, p.bits.sell1, p.struct_break_dir, p.pivot_low, p.pivot_high
                    );
            }
            let t3: Vec<_> = lv
                .bsp
                .iter()
                .filter(|p| p.bits.buy3 || p.bits.sell3)
                .take(3)
                .collect();
            for (k, p) in t3.iter().enumerate() {
                eprintln!(
                        "[census]   L{li} type3#{k}: src_idx={} buy3={} sell3={} center_zd={:?} center_zg={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy3, p.bits.sell3,
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zd), _ => None }),
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
                        p.pivot_low, p.pivot_high
                    );
            }
        }
    }
}

/// ★#141 一类判据链漏斗普查（外审问题包证据，`--ignored` 手动跑，依赖 analysis/data_cache）：
/// 逐级别逐环真实计数——候选评估→趋势门→≥2中枢→broke→A/C配对→背驰。级别循环与 `classify_impl`
/// 同一批私有函数（classify_level/compose_level/project_to_units），每级 centers 与生产 `classify`
/// 输出 assert 对拍（675号：探针走生产路径）。另产每级中枢链关系直方图 + 前缀 τ 时间线（因果
/// 重放中趋势门何时永久锁死 Degenerate）+ 反事实局部同向 run 计数（若按走势分解的局部趋势数）。
/// 运行：`cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
/// 窗口对照：`CENSUS_WINDOW="2020-10-01,2021-04-01" cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
#[test]
#[ignore = "L2 真实数据漏斗普查：cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc"]
fn type1_funnel_census_btc() {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::parser::parse_layer;
    use super::super::center::{classify_relation, CenterRelation};
    let cfg = ThetaConfig::default();
    let full =
        load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    let ds = match std::env::var(crate::theta_v0::env_registry::CENSUS_WINDOW) {
        Ok(w) => {
            let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
            eprintln!("[funnel] window={s}..{e}");
            full.slice_date_window(s, e)
        }
        Err(_) => full,
    };
    eprintln!("[funnel] BTC bars={}", ds.bars.len());
    let layer = parse_layer(&ds.bars, &cfg);
    eprintln!(
        "[funnel] L0 segments={} merged_bars={}",
        layer.segments.len(),
        layer.merged_bars.len()
    );

    // 生产对拍源（675号守卫：级别循环不分叉）。
    let out = classify(&layer, &cfg, &[]).classification;

    // classify_impl 同源输入（同一批私有函数，非重写）。
    let min_parts = cfg.level.min_parts_per_level as usize;
    let l_max = cfg.level.l_max as usize;
    let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
    assert!(!units.is_empty(), "空 L0 无漏斗对象");
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let series = divergence::compute_macd(&closes, &cfg.macd);
    let closes_tick: Vec<Tick> = layer.merged_bars.iter().map(|b| b.close).collect();
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                LeveledMove::from_unit(
                    u,
                    ElementId {
                        level: 0,
                        ordinal: i as u64,
                    },
                )
            })
            .collect(),
    );

    for level_idx in 0..=l_max {
        if units.len() < min_parts {
            break;
        }
        let is_l0 = level_idx == 0;
        let (centers, _outcome) = classify_level(&units, is_l0);
        assert_eq!(
            centers, *out.levels[level_idx].centers,
            "L{level_idx} 中枢对拍（探针级别循环须与生产 classify 逐字段一致）"
        );

        // ★task #142 量化验收：延伸段数分布（窗口段数 = seed 3 + 延伸段；同一 build 直调
        // detect_centers_windowed_resume 取窗口，中枢序列与生产 classify_level 对拍）。
        {
            let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                center::center_from_segments
            } else {
                center::center_from_window
            };
            let windowed = recursive_tower::detect_centers_windowed_resume(&units, build, 0).0;
            assert_eq!(
                windowed.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                centers,
                "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
            );
            let mut hist = [0usize; 4]; // 桶：=3（无延伸）/4-5/6-9/≥10 段
            let mut max_w = 0usize;
            for (_, (s, e)) in &windowed {
                let w = e - s + 1;
                max_w = max_w.max(w);
                hist[if w <= 3 {
                    0
                } else if w <= 5 {
                    1
                } else if w <= 9 {
                    2
                } else {
                    3
                }] += 1;
            }
            eprintln!(
                "[funnel] L{level_idx}: 窗口段数分布 =3段:{} 4-5:{} 6-9:{} ≥10:{} max={}",
                hist[0], hist[1], hist[2], hist[3], max_w
            );
        }

        // 中枢链相邻关系直方图 + 前缀 τ 时间线 + 反事实局部同向 run。
        let rels: Vec<CenterRelation> = centers
            .windows(2)
            .map(|w| classify_relation(&w[0], &w[1]))
            .collect();
        let n_up = rels
            .iter()
            .filter(|r| **r == CenterRelation::UpContinuation)
            .count();
        let n_down = rels
            .iter()
            .filter(|r| **r == CenterRelation::DownContinuation)
            .count();
        let n_exp = rels
            .iter()
            .filter(|r| **r == CenterRelation::LevelExpansion)
            .count();
        let n_ext = rels
            .iter()
            .filter(|r| **r == CenterRelation::CoreOverlap)
            .count();
        // 前缀 τ：τ(前k中枢)=Trend ⟺ k≥2 ∧ rels[0..k-1] 全等且为趋势延续关系（#898 四态：
        // 扩展与延伸均断链）。锁死点=首个异关系下标。
        let trend_open = !rels.is_empty() && rels[0].is_continuation();
        let lock_at = if rels.is_empty() {
            None
        } else if !trend_open {
            Some(0) // 首关系即 Expansion ⟹ 第3个中枢确认时 τ 已锁死 Degenerate
        } else {
            rels.iter().position(|r| *r != rels[0])
        };
        // 反事实（若走势分解为局部走势类型）：同向关系（Up/Down）的极大 run，每个 run 长 L = 局部
        // 趋势含 L+1 个中枢。计 run 数与最长 run。
        let (mut runs_ge1, mut longest_run, mut cur_run) = (0usize, 0usize, 0usize);
        for (k, r) in rels.iter().enumerate() {
            let same_dir = r.is_continuation();
            let cont = same_dir && (k == 0 || rels[k - 1] == *r);
            if same_dir {
                cur_run = if cont { cur_run + 1 } else { 1 };
                if cur_run == 1 {
                    runs_ge1 += 1;
                }
                longest_run = longest_run.max(cur_run);
            } else {
                cur_run = 0;
            }
        }
        let lock_desc = match lock_at {
            None if trend_open => format!("全链同向（不锁死）"),
            None => format!("链长<2 无关系"),
            Some(i) => {
                let c_end = centers[i + 1].end_index;
                let date = ds
                    .dates
                    .get(c_end)
                    .map(|d| d.get(..10).unwrap_or("?"))
                    .unwrap_or("?");
                format!("中枢#{}（end_src={} {date}）", i + 1, c_end)
            }
        };

        let (segs, funnel_anchors): (Vec<Segment>, Option<Vec<Option<Direction>>>) = if is_l0 {
            (layer.segments.to_vec(), None)
        } else {
            // Q7-#1 裁定C + 675号：漏斗探针锚与生产 units_anchors 同源（producer blocks 派生）。
            let pb = &out.levels[level_idx - 1].moves;
            (
                units.iter().map(unit_to_segment).collect(),
                Some(
                    (0..units.len())
                        .map(|i| decompose::center_own_dir_at(pb, i))
                        .collect(),
                ),
            )
        };
        let f = signal::type1_funnel_dx(
            &centers,
            &segs,
            funnel_anchors.as_deref(),
            &series.hist,
            &series.dif,
            &closes_tick,
            &close_src,
        );
        eprintln!(
                "[funnel] L{level_idx}: centers={} segs={} rel(up/down/exp/ext)={}/{}/{}/{} blocks(trend/consol)={}/{} 最长趋势块={}中枢 | 旧AllTrend锁死点={} 局部同向run≥2中枢数={} 最长run={}(={}中枢)",
                f.n_centers, f.n_segments, n_up, n_down, n_exp, n_ext, f.n_trend_blocks, f.n_consol_blocks,
                f.longest_trend_run, lock_desc, runs_ge1, longest_run, longest_run + 1
            );
        eprintln!(
                "[funnel] L{level_idx}: 环0候选(有最近中枢)={} → 环1有前驱中枢={} → 环2过局部趋势门={} → 环3破最后中枢={} → 环4 A/C配对={} → 环4b 037:20破b极值={} → 环5坐标映射={} → 环6背驰C<A={}",
                f.s_with_center, f.s_pos_ge1, f.s_gate_open, f.s_broke, f.s_a_paired, f.s_extreme, f.s_mapped, f.s_diverge
            );
        // ★task #144 验收证据：2021 顶区 sell1 在全历史因果重放中出现（先例窗反差闭合的正验证，
        // 生产 classify 输出直读——非探针另算）。窗口 = #141 外审切窗 2020-10-01..2021-04-15。
        {
            let top_sell1: Vec<&str> = out.levels[level_idx]
                .bsp
                .iter()
                .filter(|p| p.bits.sell1)
                .filter_map(|p| {
                    ds.dates
                        .get(p.source_index)
                        .map(|d| d.get(..10).unwrap_or("?"))
                })
                .filter(|d| ("2020-10-01".."2021-04-15").contains(d))
                .collect();
            let n_sell1 = out.levels[level_idx]
                .bsp
                .iter()
                .filter(|p| p.bits.sell1)
                .count();
            let n_buy1 = out.levels[level_idx]
                .bsp
                .iter()
                .filter(|p| p.bits.buy1)
                .count();
            eprintln!(
                    "[funnel] L{level_idx}: 全历史 buy1={} sell1={} | 2021顶区(2020-10-01..2021-04-15) sell1×{}: {:?}",
                    n_buy1, n_sell1, top_sell1.len(), top_sell1
                );
        }

        let (_cw, upper_moves, _) =
            compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        units = project_to_units(&upper_moves, &out.levels[level_idx].moves); // Q7：生产同源块
        moves_tower = Rc::new(upper_moves);
        if units.is_empty() {
            break;
        }
    }
}

/// #821 探针（wayfinder map #787 子票，乙类 task——只测量，不改生产行为/生产代码）：
/// `recursive_tower.rs:766-780` 的 ≥9 段升级重切分支，子中枢核心「继承母中枢」
/// （生产现状，口径 I）vs「按自身三段重算 `ZD=max(3 lo)`/`ZG=min(3 hi)` 且严格 `ZD<ZG`」
/// （候选口径 R，`ZD>=ZG` 不产出该子中枢）——两种做法在真实行情上差多少。
///
/// 与 `type1_funnel_census_btc` 同一套私有函数（`classify_level`/`compose_level`/
/// `project_to_units`/`detect_centers_windowed_resume`）驱动级别循环，逐级别与生产
/// `classify` 输出 assert 对拍（675号：探针走生产路径，非另起一套）。触发窗口的定位不靠
/// 重新扫描——直接读 `detect_centers_windowed_resume` 返回的 `WinMeta.emitted`（升级窗口
/// 该字段 = k > 1，`out`/`metas` 逐位对齐），子窗边界 `(s,e)` 按生产同一公式
/// `s=win_start+t*3, e=(t+1==k ? win_exit-1 : s+2)` 重建，再从**同一份生产 `units`**切片
/// 读三段真实 lo/hi——不重写 `detect_centers_windowed_resume` 本身的扫描逻辑。
///
/// 数据：`analysis/data_cache/btc_1m_full.json`（`load_by_symbol("BTC", ..)`，1 分钟 K，
/// 全历史）——bar 数以运行时 `[821] BTC bars=` 行为准（未做窗口切片，全量）。
///
/// 下游翻转（点4）仅做**同级本地**翻译：把触发级别的中枢序列替换为口径 R 产出（丢弃项跳过、
/// 保留项区间替换），直接喂 `decompose::decompose` 读 Trend/Consolidation 块计数差——
/// **不做跨级级联**（口径 R 改变本级中枢数会经 `project_to_units` 改变下一级输入单元，
/// 逐级复算等价于另起一条平行管线，超出本票探针预算，如实标注不做，非"无差异"）。
/// BSP（买卖点）计数同理不做——`extract_signals_with_hist`/`extract_first_third_for_level`/
/// `extract_second_for_level` 参数面广且依赖 MACD 背驰真算，接不通，如实标注。
///
/// 运行：`cargo test --release --lib -- --ignored --nocapture issue821_upgrade_recut_probe`
#[test]
#[ignore = "issue #821 探针：cargo test --release --lib -- --ignored --nocapture issue821_upgrade_recut_probe"]
fn issue821_upgrade_recut_probe() {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::parser::parse_layer;

    let cfg = ThetaConfig::default();
    let full =
        load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    eprintln!("[821] BTC bars={}", full.bars.len());
    let layer = parse_layer(&full.bars, &cfg);
    eprintln!(
        "[821] L0 segments={} merged_bars={}",
        layer.segments.len(),
        layer.merged_bars.len()
    );

    // 生产对拍源（675号守卫：级别循环不分叉）。
    let out = classify(&layer, &cfg, &[]).classification;

    let min_parts = cfg.level.min_parts_per_level as usize;
    let l_max = cfg.level.l_max as usize;
    let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                LeveledMove::from_unit(
                    u,
                    ElementId {
                        level: 0,
                        ordinal: i as u64,
                    },
                )
            })
            .collect(),
    );

    let (mut total_centers_i, mut total_centers_r) = (0usize, 0usize);
    let (mut total_triggers, mut total_windows) = (0usize, 0usize);
    let mut total_discards = 0usize;
    let mut discard_examples: Vec<String> = Vec::new();
    let mut width_diffs: Vec<i64> = Vec::new(); // R宽 - 母核宽（ticks，可负=更窄）
    let mut outside_mother: Vec<String> = Vec::new();
    let mut trend_delta_examples: Vec<String> = Vec::new();

    for level_idx in 0..=l_max {
        if units.len() < min_parts {
            break;
        }
        let is_l0 = level_idx == 0;
        let (centers, _blocks) = classify_level(&units, is_l0);
        assert_eq!(
            centers, *out.levels[level_idx].centers,
            "L{level_idx} 中枢对拍（探针须与生产 classify 逐字段一致）"
        );

        let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
            center::center_from_segments
        } else {
            center::center_from_window
        };
        let (out_win, metas, _cursor) =
            recursive_tower::detect_centers_windowed_resume(&units, build, 0);
        assert_eq!(
            out_win.len(),
            metas.len(),
            "L{level_idx} out/metas 1:1 对齐"
        );
        assert_eq!(
            out_win.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
            centers,
            "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
        );

        let level_centers_i = out_win.len();
        total_centers_i += level_centers_i;

        let mut centers_r_level: Vec<Center> = Vec::new();
        let (mut level_triggers, mut level_centers_r, mut level_discards) =
            (0usize, 0usize, 0usize);
        let mut level_windows = 0usize; // 扫描出的成立窗口数（触发窗口计1，非触发窗口每条 meta 计1）
        let mut idx = 0usize;
        while idx < metas.len() {
            let m = metas[idx];
            level_windows += 1;
            if m.emitted > 1 {
                level_triggers += 1;
                total_triggers += 1;
                let k = m.emitted;
                let i = m.win_start;
                let j = m.win_exit;
                for t in 0..k {
                    let s = i + t * 3;
                    let e = if t + 1 == k { j - 1 } else { s + 2 };
                    let sub_units = &units[s..=e];
                    let zd_r = sub_units.iter().map(|u| u.lo).max().expect("子窗非空");
                    let zg_r = sub_units.iter().map(|u| u.hi).min().expect("子窗非空");
                    let mother = out_win[idx + t].0; // 口径I：zd/zg=继承母核心
                    if zd_r < zg_r {
                        level_centers_r += 1;
                        total_centers_r += 1;
                        let width_r = zg_r - zd_r;
                        let width_mother = mother.zg - mother.zd;
                        width_diffs.push(width_r - width_mother);
                        if zd_r < mother.zd || zg_r > mother.zg {
                            outside_mother.push(format!(
                                "L{level_idx} win=(i={i},j={j}) 子#{t} segs[{s}..={e}] \
                                     R核心=[{zd_r},{zg_r}] 母核心=[{},{}]",
                                mother.zd, mother.zg
                            ));
                        }
                        centers_r_level.push(Center {
                            zd: zd_r,
                            zg: zg_r,
                            dd: mother.dd,
                            gg: mother.gg,
                            start_index: mother.start_index,
                            end_index: mother.end_index,
                        });
                    } else {
                        level_discards += 1;
                        total_discards += 1;
                        if discard_examples.len() < 5 {
                            let detail: Vec<String> = sub_units
                                .iter()
                                .enumerate()
                                .map(|(si, u)| {
                                    format!("段{}(unit_idx={}) lo={} hi={}", si, s + si, u.lo, u.hi)
                                })
                                .collect();
                            discard_examples.push(format!(
                                "L{level_idx} win=(i={i},j={j}) 子#{t} segs[{s}..={e}]: {} \
                                     => ZD=max(lo)={zd_r} ZG=min(hi)={zg_r}（ZD>=ZG，丢弃）",
                                detail.join("; ")
                            ));
                        }
                    }
                }
                idx += k;
            } else {
                level_centers_r += 1;
                total_centers_r += 1;
                centers_r_level.push(out_win[idx].0);
                idx += 1;
            }
        }
        total_windows += level_windows;
        eprintln!(
            "[821] L{level_idx}: windows={level_windows} centers_I={level_centers_i} \
                 centers_R={level_centers_r} triggers={level_triggers} discards={level_discards}"
        );

        // 点4（部分，同级本地翻译，不跨级级联——见函数头注释）：口径R替换后 decompose 块计数差。
        if level_triggers > 0 {
            let blocks_i = decompose::decompose(&centers);
            let blocks_r = decompose::decompose(&centers_r_level);
            let trend_i = blocks_i
                .iter()
                .filter(|b| b.kind == MoveKind::Trend)
                .count();
            let trend_r = blocks_r
                .iter()
                .filter(|b| b.kind == MoveKind::Trend)
                .count();
            trend_delta_examples.push(format!(
                "L{level_idx}: blocks_I={} (trend={trend_i}) blocks_R={} (trend={trend_r})",
                blocks_i.len(),
                blocks_r.len()
            ));
        }

        let (_cw, upper_moves, _) =
            compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        units = project_to_units(&upper_moves, &out.levels[level_idx].moves);
        moves_tower = Rc::new(upper_moves);
        if units.is_empty() {
            break;
        }
    }

    eprintln!(
        "[821] TOTAL centers_I={total_centers_i} centers_R={total_centers_r} \
             triggers={total_triggers} discards={total_discards} (windows_seen={total_windows})"
    );
    eprintln!("[821] discard 示例（ZD>=ZG，最多5条）:");
    for ex in &discard_examples {
        eprintln!("  {ex}");
    }
    eprintln!(
        "[821] outside-mother 次数（R核心落在母核心之外）={}",
        outside_mother.len()
    );
    for ex in outside_mother.iter().take(10) {
        eprintln!("  {ex}");
    }
    if !width_diffs.is_empty() {
        let sum: i64 = width_diffs.iter().sum();
        let avg = sum as f64 / width_diffs.len() as f64;
        let min_d = *width_diffs.iter().min().unwrap();
        let max_d = *width_diffs.iter().max().unwrap();
        eprintln!(
            "[821] 区间宽度差（R宽-母核宽，ticks）: avg={avg:.3} min={min_d} max={max_d} n={}",
            width_diffs.len()
        );
    } else {
        eprintln!("[821] 区间宽度差：无保留的 R 子中枢样本（0 触发或全部丢弃）");
    }
    eprintln!("[821] 点4（trend block 计数，同级本地，非级联）:");
    for ex in &trend_delta_examples {
        eprintln!("  {ex}");
    }
}

/// #826 探针（wayfinder map #787 子票，乙类 task——只测量，不改生产行为/生产代码）：
/// 本仓的塔（`compose_level` 逐级上升）产出的走势分解，与缠师第 38 课「同级别分解」
/// （`docs/chanlun/text/blog/038-第38课.md:18`：把所有走势按一固定级别的走势类型分解成
/// 一段段连接）是不是同一个东西。
///
/// 测四件（票面）：
/// 1. **切点对照**：塔每级的上级走势序列在 L0 原始 K 序上的切点，是否构成对整条行情的
///    **无缝无重叠覆盖**（同级别分解的定义性要求：「分解成一段段走势类型的**连接**」，
///    连接 ⟹ 前一段终点即后一段起点，无遗漏）。量：覆盖率、缺口条数、缺口跨的 bar 数。
/// 2. **唯一性**：同输入同输出（纯函数）+ 前缀稳定性（把 units 截断到前 n 个再分解，
///    结果是否是全量分解的前缀）——第 38 课要求「分解的唯一性」。
/// 3. **第 38 课细则**（`038-第38课.md:24`+`:26`）：同级别分解在**操作级别上不定义中枢
///    延伸、允许盘整+盘整**，而**该级别以下允许延伸**。塔在**每一级**都无条件跑
///    `detect_centers_windowed_resume` 的 Step2 延伸吸收 ⟹ 量「每级有多少窗口发生了延伸
///    （窗口段数>3）」，若每级都>0，则塔不存在一个「关掉延伸」的操作级别。
/// 4. **最小反例**：打印首个缺口（塔丢弃的连接段）的坐标 + 该缺口两侧上级走势。
///
/// 数据：`analysis/data_cache/btc_1m_full.json`（`load_by_symbol("BTC", ..)`，1 分钟 K，
/// 全历史）——bar 数以运行时 `[826] BTC bars=` 行为准。级别循环与生产 `classify` 逐级
/// assert 对拍（675号：探针走生产路径）。
///
/// 运行：`cargo test --release --lib -- --ignored --nocapture issue826_same_level_decomp_probe`
#[test]
#[ignore = "issue #826 探针：cargo test --release --lib -- --ignored --nocapture issue826_same_level_decomp_probe"]
fn issue826_same_level_decomp_probe() {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::parser::parse_layer;

    let cfg = ThetaConfig::default();
    let full =
        load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    eprintln!("[826] BTC bars={}", full.bars.len());
    let layer = parse_layer(&full.bars, &cfg);
    eprintln!(
        "[826] L0 segments={} merged_bars={}",
        layer.segments.len(),
        layer.merged_bars.len()
    );
    let l0_span_lo = layer.segments.first().map(|s| s.start_index).unwrap_or(0);
    let l0_span_hi = layer.segments.last().map(|s| s.end_index).unwrap_or(0);
    eprintln!("[826] L0 线段覆盖的原始 K 序区间 = [{l0_span_lo}, {l0_span_hi}]");

    let out = classify(&layer, &cfg, &[]).classification;

    let min_parts = cfg.level.min_parts_per_level as usize;
    let l_max = cfg.level.l_max as usize;
    let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                LeveledMove::from_unit(
                    u,
                    ElementId {
                        level: 0,
                        ordinal: i as u64,
                    },
                )
            })
            .collect(),
    );

    let mut first_gap_report: Option<String> = None;

    for level_idx in 0..=l_max {
        if units.len() < min_parts {
            eprintln!(
                "[826] L{level_idx}: units={} < min_parts ⟹ 塔自然终止",
                units.len()
            );
            break;
        }
        let is_l0 = level_idx == 0;
        let (centers, _blocks) = classify_level(&units, is_l0);
        assert_eq!(
            centers, *out.levels[level_idx].centers,
            "L{level_idx} 中枢对拍（探针须与生产 classify 逐字段一致）"
        );

        let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
            center::center_from_segments
        } else {
            center::center_from_window
        };
        let (out_win, _metas, _cursor) =
            recursive_tower::detect_centers_windowed_resume(&units, build, 0);
        assert_eq!(
            out_win.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
            centers,
            "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
        );

        let (_cw, upper_moves, _) =
            compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);

        // ── 点 1：单元层覆盖（本级 units 有多少被上级走势吃掉，多少被丢弃）
        let n_units = units.len();
        let mut consumed_units = 0usize;
        for (_, (a, b)) in &out_win {
            consumed_units += b - a + 1;
        }
        // 缺口 = 相邻窗口之间未被任何窗口覆盖的 unit 段（含头尾）
        let mut gap_runs: Vec<(usize, usize)> = Vec::new(); // unit 索引闭区间
        let mut cursor_u = 0usize;
        for (_, (a, b)) in &out_win {
            if *a > cursor_u {
                gap_runs.push((cursor_u, a - 1));
            }
            cursor_u = b + 1;
        }
        if cursor_u < n_units {
            gap_runs.push((cursor_u, n_units - 1));
        }
        let gap_units: usize = gap_runs.iter().map(|(a, b)| b - a + 1).sum();
        // ── 基线校准：本级 units 序列自身在 bar 坐标上是否首尾相接（若本身就不相接，
        // 则塔的 bar 层断点不能全部归因于「丢弃连接段」）。
        let unit_adj_ok = units
            .windows(2)
            .filter(|w| w[0].end_index == w[1].start_index)
            .count();
        let unit_adj_gapbars: i64 = units
            .windows(2)
            .map(|w| (w[1].start_index as i64 - w[0].end_index as i64).max(0))
            .sum();
        eprintln!(
                "[826] L{level_idx} 基线: units 相邻首尾相接={unit_adj_ok}/{} units 自身缺口总 bar={unit_adj_gapbars}",
                units.len().saturating_sub(1)
            );
        // 点4 最小反例：L0 头 3 条缺口的逐单元明细（塔丢弃的连接段）
        if is_l0 {
            for (gi, (a, b)) in gap_runs.iter().take(3).enumerate() {
                let detail: Vec<String> = (*a..=*b)
                    .map(|k| {
                        format!(
                            "unit#{k}[src {}..{} lo={} hi={}]",
                            units[k].start_index, units[k].end_index, units[k].lo, units[k].hi
                        )
                    })
                    .collect();
                eprintln!(
                    "[826] 点4 L0 缺口#{gi}: 丢弃 units[{a}..={b}]（{} 段） {}",
                    b - a + 1,
                    detail.join(" ")
                );
            }
        }

        // ── 点 1b：bar 层覆盖（上级走势在 L0 原始 K 序上的切点是否首尾相接）
        let mut covered_bars: i64 = 0;
        let mut joint_ok = 0usize;
        let mut joint_gap = 0usize;
        let mut joint_overlap = 0usize;
        let mut gap_bars: i64 = 0;
        for m in &upper_moves {
            covered_bars += m.end_index as i64 - m.start_index as i64;
        }
        for w in upper_moves.windows(2) {
            let (prev, next) = (&w[0], &w[1]);
            match next.start_index.cmp(&prev.end_index) {
                std::cmp::Ordering::Equal => joint_ok += 1,
                std::cmp::Ordering::Greater => {
                    joint_gap += 1;
                    gap_bars += next.start_index as i64 - prev.end_index as i64;
                    if first_gap_report.is_none() {
                        first_gap_report = Some(format!(
                                "L{level_idx}→L{}: 上级走势#{}[src {}..{}] 与 #{}[src {}..{}] 之间空出 {} 根 K（塔丢弃的连接段）",
                                level_idx + 1,
                                upper_moves.iter().position(|x| std::ptr::eq(x, prev)).unwrap_or(0),
                                prev.start_index, prev.end_index,
                                upper_moves.iter().position(|x| std::ptr::eq(x, next)).unwrap_or(0),
                                next.start_index, next.end_index,
                                next.start_index - prev.end_index
                            ));
                    }
                }
                std::cmp::Ordering::Less => joint_overlap += 1,
            }
        }
        let span_lo = upper_moves.first().map(|m| m.start_index).unwrap_or(0);
        let span_hi = upper_moves.last().map(|m| m.end_index).unwrap_or(0);
        let head_bars = span_lo as i64 - l0_span_lo as i64;
        let tail_bars = l0_span_hi as i64 - span_hi as i64;

        // ── 点 3：延伸窗口占比（窗口段数 > 3 ⟹ 该窗口发生了中枢延伸吸收）
        let ext_windows = out_win.iter().filter(|(_, (a, b))| b - a + 1 > 3).count();
        let max_win_len = out_win
            .iter()
            .map(|(_, (a, b))| b - a + 1)
            .max()
            .unwrap_or(0);

        // ── 点 1c：上级走势的跨度分布（同级别分解要求同级别 ⟹ 尺度可比）
        let mut spans: Vec<i64> = upper_moves
            .iter()
            .map(|m| m.end_index as i64 - m.start_index as i64)
            .collect();
        spans.sort_unstable();
        let (s_min, s_med, s_max) = if spans.is_empty() {
            (0, 0, 0)
        } else {
            (spans[0], spans[spans.len() / 2], spans[spans.len() - 1])
        };

        eprintln!(
                "[826] L{level_idx}: units={n_units} upper_moves={} | 单元覆盖: 被吃={consumed_units} 丢弃={gap_units}（{:.2}%）缺口段数={} \
                 | bar 切点: 首尾相接={joint_ok} 断开={joint_gap} 重叠={joint_overlap} 断开总 bar={gap_bars} 头={head_bars} 尾={tail_bars} \
                 | 上级走势跨度(bar) min={s_min} med={s_med} max={s_max} | 延伸窗口={ext_windows}/{} 最长窗口段数={max_win_len}",
                upper_moves.len(),
                100.0 * gap_units as f64 / n_units.max(1) as f64,
                gap_runs.len(),
                out_win.len(),
            );
        let _ = covered_bars;

        // ── 点 1d：与本仓**已有的另一套同级分解**对照——`decompose::decompose(centers)`
        // 产出的 MoveBlock 链（走势类型 Trend/Consolidation 的连接，中枢下标空间上
        // `b[j+1].start == b[j].end` 严格连续）。这是形状上唯一与「同级别分解」对得上的
        // 对象。塔**不走它**（塔走的是一中枢一上级走势的 upper_moves）。逐点比两者切点。
        let blocks = &out.levels[level_idx].moves;
        let blk_cuts: Vec<(usize, usize)> = blocks
            .iter()
            .map(|b| {
                (
                    centers[b.start_center].start_index,
                    centers[b.end_center].end_index,
                )
            })
            .collect();
        let mut blk_joint_ok = 0usize;
        let mut blk_joint_gap = 0usize;
        let mut blk_joint_overlap = 0usize;
        let mut blk_gap_bars: i64 = 0;
        for w in blk_cuts.windows(2) {
            match w[1].0.cmp(&w[0].1) {
                std::cmp::Ordering::Equal => blk_joint_ok += 1,
                std::cmp::Ordering::Greater => {
                    blk_joint_gap += 1;
                    blk_gap_bars += w[1].0 as i64 - w[0].1 as i64;
                }
                std::cmp::Ordering::Less => blk_joint_overlap += 1,
            }
        }
        // 切点集合逐点比：上级走势起点集合 vs 块起点集合
        let up_starts: std::collections::BTreeSet<usize> =
            upper_moves.iter().map(|m| m.start_index).collect();
        let blk_starts: std::collections::BTreeSet<usize> = blk_cuts.iter().map(|c| c.0).collect();
        let shared = up_starts.intersection(&blk_starts).count();
        eprintln!(
                "[826] L{level_idx} 两套口径切点对照: 走势类型块={} (切点接合: 相接={blk_joint_ok} 断开={blk_joint_gap} 重叠={blk_joint_overlap} 断开总bar={blk_gap_bars}) \
                 vs 塔上级走势={} | 起点切点交集={shared}（占块起点 {:.2}%，占塔起点 {:.2}%）",
                blocks.len(),
                upper_moves.len(),
                100.0 * shared as f64 / blk_starts.len().max(1) as f64,
                100.0 * shared as f64 / up_starts.len().max(1) as f64,
            );

        // ── 点 1e ★核心对照：口径 S = 第 38 课同级别分解规则的直译
        // （`038-第38课.md:20`「在这种同级别的分解中，是**不需要中枢延伸或扩展的概念**的，
        //   对30分钟来说，只要5分钟级别的**三段**上下上或下上下类型有价格区间的重合就构成
        //   中枢。如果这5分钟次级别延伸出6段，那么就当成**两个**30分钟盘整类型的连接」）：
        // 同一个 seed 判据（`build`，与生产逐字段同源），但窗口**恒为 3 段**、成立即 i += 3，
        // 不做任何延伸吸收。口径 T = 生产塔（seed + 延伸吸收 + ≥9 段重切）。
        let mut s_windows: Vec<(usize, usize)> = Vec::new();
        {
            let mut i = 0usize;
            while i + 2 < units.len() {
                if build(&units[i], &units[i + 1], &units[i + 2]).is_some() {
                    s_windows.push((i, i + 2));
                    i += 3;
                } else {
                    i += 1;
                }
            }
        }
        let t_windows: Vec<(usize, usize)> = out_win.iter().map(|(_, w)| *w).collect();
        let s_starts: std::collections::BTreeSet<usize> = s_windows.iter().map(|w| w.0).collect();
        let t_starts: std::collections::BTreeSet<usize> = t_windows.iter().map(|w| w.0).collect();
        let st_shared = s_starts.intersection(&t_starts).count();
        let identical_windows = s_windows
            .iter()
            .filter(|w| t_windows.binary_search(w).is_ok())
            .count();
        let s_consumed: usize = s_windows.len() * 3;
        eprintln!(
                "[826] L{level_idx} ★口径对照 S(38课·禁延伸·恒3段) vs T(生产塔·延伸吸收): \
                 中枢数 S={} T={} (T/S={:.3}) | 窗口起点交集={st_shared}（占S {:.2}%，占T {:.2}%）\
                 | 完全相同的窗口(起止都同)={identical_windows} | 单元覆盖 S={s_consumed}/{n_units} T={consumed_units}/{n_units}",
                s_windows.len(),
                t_windows.len(),
                t_windows.len() as f64 / s_windows.len().max(1) as f64,
                100.0 * st_shared as f64 / s_starts.len().max(1) as f64,
                100.0 * st_shared as f64 / t_starts.len().max(1) as f64,
            );
        if is_l0 {
            // 最小反例：首个「T 延伸吸收 ≥6 段、S 拆成 ≥2 个中枢」的窗口
            if let Some((a, b)) = t_windows.iter().find(|(a, b)| b - a + 1 >= 6) {
                let s_inside: Vec<_> = s_windows
                    .iter()
                    .filter(|w| w.0 >= *a && w.1 <= *b)
                    .collect();
                eprintln!(
                        "[826] 点4 最小反例（L0 首个 ≥6 段延伸窗口）: T 把 units[{a}..={b}]（{} 段，src {}..{}）\
                         吃成 **1 个**中枢/1 段上级走势；S（38课）在同一区间产 **{} 个**中枢 {:?} \
                         ⟹ 上级走势数差 {}，38课口径下这里是「盘整+盘整」的连接，塔口径下是单个延伸中枢",
                        b - a + 1,
                        units[*a].start_index,
                        units[*b].end_index,
                        s_inside.len(),
                        s_inside,
                        s_inside.len() as i64 - 1,
                    );
            }
        }

        // ── 点 2：唯一性 / 前缀稳定性（同一份 units 截断到 90% 再分解，比对前缀）
        if n_units >= 20 {
            let cut = n_units * 9 / 10;
            let (pre_win, _, _) =
                recursive_tower::detect_centers_windowed_resume(&units[..cut], build, 0);
            // 全量里完全落在 [0,cut) 内的窗口
            let full_inside: Vec<_> = out_win
                .iter()
                .filter(|(_, (_, b))| *b < cut)
                .cloned()
                .collect();
            let common = full_inside.len().min(pre_win.len());
            let mismatch = (0..common)
                .filter(|&k| full_inside[k] != pre_win[k])
                .count();
            for k in 0..common {
                if full_inside[k] != pre_win[k] {
                    eprintln!(
                            "[826]   L{level_idx} 前缀不一致 #{k}: 全量 win={:?} center=[{},{}] vs 截断 win={:?} center=[{},{}]（cut={cut}）",
                            full_inside[k].1, full_inside[k].0.zd, full_inside[k].0.zg,
                            pre_win[k].1, pre_win[k].0.zd, pre_win[k].0.zg,
                        );
                    break;
                }
            }
            eprintln!(
                    "[826] L{level_idx} 前缀稳定性: 截断到 units[..{cut}]，截断产出={} 全量内含={} 共同前缀比对不一致={}",
                    pre_win.len(),
                    full_inside.len(),
                    mismatch
                );
            // 确定性：同输入跑两遍
            let (again, _, _) = recursive_tower::detect_centers_windowed_resume(&units, build, 0);
            assert_eq!(again, out_win, "L{level_idx} 同输入两次分解必须逐位相同");
        }

        units = project_to_units(&upper_moves, &out.levels[level_idx].moves);
        moves_tower = Rc::new(upper_moves);
        if units.is_empty() {
            break;
        }
    }

    eprintln!("[826] 点4 首个 bar 层缺口（最小反例锚）：");
    match &first_gap_report {
        Some(r) => eprintln!("  {r}"),
        None => eprintln!("  未发现 bar 层缺口"),
    }
}

#[test]
fn classify_empty_layer_yields_empty() {
    let cfg = ThetaConfig::default();
    let out = classify(&ParseLayer::default(), &cfg, &[]).classification;
    assert_eq!(out, Classification::default());
}

#[test]
fn fewer_than_min_parts_natural_termination() {
    // L0 线段数 < min_parts_per_level(3) ⟹ 无 L0 走势，levels 空（自然终止）。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 0, 10),
            seg(Direction::Down, 4, 8, 10, 5),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    assert!(out.levels.is_empty(), "2 段 < min_parts 3 ⟹ 自然终止");
}

#[test]
fn three_overlapping_segments_form_center() {
    // 完整判据（Origin.CenterComplete，口径 B 637号）：方向交替 上-下-上 + 全三段核心非空。
    // 段区间 [0,10]up,[3,12]down,[5,15]up：核心取**全三段** zd=max(0,3,5)=5, zg=min(10,12,15)=10。
    // 第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）⟹ 真中枢成立。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 0, 10),
            seg(Direction::Down, 4, 8, 12, 3),
            seg(Direction::Up, 8, 12, 5, 15),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    assert!(!out.levels.is_empty());
    let l0 = &out.levels[0];
    assert_eq!(l0.centers.len(), 1, "三段方向交替+贯穿 ⟹ 一个真中枢");
    // 核心取全三段（口径 B，637号 computeZD/computeZG s1 s2 s3）——第三段收窄核心下沿至 5。
    assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
    // 一个中枢 ⟹ 分解 = 单盘整块（PDF §6 情形1）。
    assert_eq!(l0.moves.len(), 1);
    assert_eq!(
        (
            l0.moves[0].kind,
            l0.moves[0].start_center,
            l0.moves[0].end_center
        ),
        (MoveKind::Consolidation, 0, 0)
    );
}

#[test]
fn same_direction_three_segments_rejected_by_complete() {
    // G4 完整判据反退化：三段同向（全 up）+ 前两段核心非空，但无方向交替 ⟹ L0 不识别中枢
    // （旧几何窗口会误判，完整判据正确拒绝，对齐 Origin.CenterComplete.sameDir_not_centerConfirmed）。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 10, 20),
            seg(Direction::Up, 4, 8, 18, 25),
            seg(Direction::Up, 8, 12, 22, 30),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    // 无方向交替 ⟹ L0 无中枢（完整判据拒绝单边三段）。
    assert_eq!(out.levels.len(), 1);
    assert!(
        out.levels[0].centers.is_empty(),
        "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）"
    );
}

#[test]
fn lmax_bound_respected() {
    // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
    let cfg = ThetaConfig::default();
    let mut segments = Vec::new();
    // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
    for i in 0..27 {
        let dir = if i % 2 == 0 {
            Direction::Up
        } else {
            Direction::Down
        };
        segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
    }
    let layer = ParseLayer {
        segments: Rc::new(segments),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
    assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
}

#[test]
fn no_center_terminates_recursion() {
    // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 0, 4),
            seg(Direction::Down, 4, 8, 14, 10),
            seg(Direction::Up, 8, 12, 20, 24),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    // L0 无中枢 ⟹ 该级 moves 空（裁决退化），递归在该级后终止（无上级单元）。
    assert_eq!(out.levels.len(), 1);
    assert!(out.levels[0].centers.is_empty());
    assert!(out.levels[0].moves.is_empty());
}

/// ★端到端 fixture（Lead 验证门）：≥3 重叠线段 → 非空中枢 + 至少一个买卖点。
/// 解阻塞点 A：classify 在真实结构输入上返回非空 Classification（n_orders>0 的前提）。
#[test]
fn end_to_end_third_buy_signal() {
    let cfg = ThetaConfig::default();
    // 段0-2：三段在 [100,200] 重叠 ⟹ seed 中枢，核心 [ZD,ZG]=[100,200] 冻结，end_index=12。
    // 段3：向上离开——延伸语义下（PDF §5 Step3，task #142）离开段必须与冻结核心不相交：
    //   lo=205 > ZG=200 ⟹ non-extension（旧 fixture lo=150 ≤ 200 会被 Step2 吸收进中枢 ⟹ 无离开段）。
    // 段4：向下回试低点 210 > zg=200（严格不触闭区间）⟹ 3 买 @ source_index=20。
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 100),
            seg(Direction::Up, 8, 12, 100, 200),
            seg(Direction::Up, 12, 16, 205, 250), // 离开中枢上方（lo=205>ZG ⟹ non-extension）
            seg(Direction::Down, 16, 20, 250, 210), // 回试低点 > zg → 3 买
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;

    // 1) 非空 Classification + L0 有中枢。
    assert!(!out.levels.is_empty(), "解阻塞 A：classify 返回非空");
    let l0 = &out.levels[0];
    assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
    assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (100, 200));

    // 2) 至少一个买卖点（第三类买点），且携带结构止损价 single source。
    assert!(!l0.bsp.is_empty(), "解阻塞 A：L0 至少一个买卖点");
    let third_buys: Vec<_> = l0.bsp.iter().filter(|p| p.bits.buy3).collect();
    assert_eq!(third_buys.len(), 1, "一个第三类买点");
    let p = third_buys[0];
    assert_eq!(p.source_index, 20, "买卖点定位回试端点");
    assert_eq!(p.pivot_low, 210, "结构止损价 pivot_low single source");
    assert_eq!(
        p.center.and_then(|o| match o {
            signal::OwnerRef::Center(c) => Some(c.zg),
            _ => None,
        }),
        Some(200),
        "3 买止损 = ZG single source（#218 面 A 载体形态：Center 变体读出）"
    );
}

/// 端到端边界：有中枢但无离开/回试 ⟹ 中枢非空、bsp 空（无买卖点是诚实产出，非错误）。
#[test]
fn end_to_end_center_without_signal() {
    let cfg = ThetaConfig::default();
    // 三段重叠成中枢，但无后续离开线段 ⟹ 无第三类买卖点。
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 100),
            seg(Direction::Up, 8, 12, 100, 200),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg, &[]).classification;
    assert_eq!(out.levels[0].centers.len(), 1);
    assert!(
        out.levels[0].bsp.is_empty(),
        "无离开/回试 ⟹ 无买卖点（诚实空）"
    );
}

/// ★#885 S4-d（**验收测试锁**）：否则域（`T3InCGrade::Missing`）分级记录进
/// `Classification`、按 (level, source_index) 坐标可查——此前记录体只在 env 门控的
/// thread_local 诊断 sidecar `GRADE_SIDECAR`，生产 `Classification` 按坐标查不到，
/// 任何涉及类一类点的命中率因此只是下界。本票只建载体/可查性，判据未动
/// （否则域点仍零一类 bit，#607 D2 语义不变）。
///
/// fixture（全管线 classify 真跑）：两依次向下中枢（C0[300,400] → C1[180,210]，外缘
/// C1.gg=280 < C0.dd=290 ⟹ Trend(Down)）+ C 段 s6 破 C1 核心（端点 80 < zd=180）；固定首对
/// = (s6 Down, s7 Down) 同向 ⟹ `Missing(SameDirection)`（#606 D1 五桶之一，不后扫）。
/// closes：A 段 [12,20] 急跌（hist 面积大）→ 回拉 → C 段 [24,28] 缓跌（面积小 ⟹ C<A 背驰）。
#[test]
fn otherwise_domain_records_queryable_by_coordinate_in_classification() {
    // #990：默认档已切 ForceL；本测试验收 #885 可查性（diverged 两域记录），
    // 非教义判据本身 ⟹ 显式 MacdArea 对照档（fixture 无 strokes，语义锁定旧面积口径）。
    let mut cfg = ThetaConfig::default();
    cfg.divergence_gauge = divergence::DivergenceGauge::MacdArea;
    let segments = vec![
        seg(Direction::Up, 0, 4, 300, 400),
        seg(Direction::Down, 4, 8, 400, 290),
        seg(Direction::Up, 8, 12, 290, 410), // → C0 [300,400]（[0,12]，dd=290/gg=410）
        seg(Direction::Down, 12, 16, 280, 180), // [180,280] 不触 C0 核心 ⟹ non-extension
        seg(Direction::Up, 16, 20, 180, 210),
        seg(Direction::Down, 20, 24, 210, 150), // → C1 [180,210]（[12,24]，dd=150/gg=280）
        seg(Direction::Down, 24, 28, 170, 80),  // s6 C 段：破 C1 核心（80 < zd=180）
        seg(Direction::Down, 28, 32, 80, 70),   // s7 与 s6 同向 ⟹ 固定首对 Missing(SameDirection)
    ];
    let closes: Vec<i64> = vec![
        350, 350, 350, 350, 350, 350, 350, 350, 350, 350, 350,
        350, // 0..12 预热（EMA 收敛）
        340, 320, 290, 260, 230, 200, 170, 150, // 12..20 A 段急跌（hist 面积大）
        160, 180, 200, 210, // 20..24 回拉（EMA 收敛）
        205, 200, 195, 190, // 24..28 C 段缓跌（hist 面积小 ⟹ C<A）
        188, 186, 184, 182, 180, // 28..33 缓跌延续
    ];
    let layer = ParseLayer {
        segments: Rc::new(segments.clone()),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let cls = classify(&layer, &cfg, &[]).classification;

    // 0) 结构前提：L0 两中枢 + Trend(Down)（fixture 不自证则后述断言全空转）。
    let l0 = &cls.levels[0];
    assert_eq!(l0.centers.len(), 2, "fixture 前提：L0 两中枢");
    assert_eq!(
        (l0.centers[1].zd, l0.centers[1].zg),
        (180, 210),
        "fixture 前提：C1 核心 [180,210]"
    );

    // 1) 否则域点仍零一类 bit（#607 D2 语义不变——本票只建可查性，判据未动）。
    let p28 = l0
        .bsp
        .iter()
        .find(|p| p.source_index == 28)
        .expect("否则域候选点仍在 bsp 样本（零 bit 结构候选）");
    assert!(
        !p28.bits.buy1 && !p28.bits.sell1,
        "否则域点零一类 bit（D2 语义不变）"
    );

    // 2) **验收锁**：按 (level=0, source_index=28) 坐标查到否则域记录，字段逐位锁定。
    let rec = cls
        .otherwise_domain_at(0, 28)
        .expect("否则域记录按坐标可查（#885 验收）");
    assert_eq!(rec.level, 0, "level 由装配方按真实级别盖章");
    assert_eq!(rec.side, Side::Long);
    assert_eq!(
        rec.grade,
        signal::T3InCGrade::Missing(signal::T3InCGradeReason::SameDirection),
        "固定首对 (s6 Down, s7 Down) 同向 ⟹ Missing(SameDirection)"
    );
    assert_eq!(
        (
            rec.center_start_index,
            rec.center_end_index,
            rec.center_zd,
            rec.center_zg
        ),
        (12, 24, 180, 210),
        "中枢身份 = 判定中枢 C1"
    );

    // 3) LevelState 侧查询同 record；否则域迭代器覆盖全部 Missing 记录（含 s7 若 diverged）。
    assert_eq!(
        l0.first_class_grade_at(28),
        Some(rec),
        "LevelState::first_class_grade_at 与 Classification::otherwise_domain_at 同源"
    );
    let otherwise: Vec<_> = l0.otherwise_domain_records().collect();
    // 前后对照的可测面数字（钉死 fixture 产出，防静默漂移）：本 fixture L0 两个 diverged
    // 一类候选（s6/s7，C 段 episode 未回中枢 ⟹ 同一固定首对同向桶）均落否则域
    // Missing(SameDirection)，无 Present 记录 ⟹ 全部记录恰 2 条且全是否则域。
    // 改前同一 fixture 在 Classification 上的可查否则域数恒 0（无字段，只落 env 门控
    // sidecar）；改后 = 2（下界口径不变，可测面从 0 扩到 2）。
    assert_eq!(
        l0.first_class_grades.len(),
        2,
        "fixture 钉死：L0 恰 2 条分级记录（s6/s7 两候选）"
    );
    assert_eq!(
        otherwise.len(),
        2,
        "fixture 钉死：2 条记录全是否则域 Missing(SameDirection)（无 Present）"
    );
    assert!(otherwise
        .iter()
        .all(|r| r.grade == signal::T3InCGrade::Missing(signal::T3InCGradeReason::SameDirection)));
    assert_eq!(
        otherwise.iter().map(|r| r.source_index).collect::<Vec<_>>(),
        vec![28, 32],
        "记录按 source_index 升序（提取出口 canonical 排序）"
    );

    // 4) 负坐标/越界 level/非 Missing 坐标 ⟹ None。
    assert!(
        cls.otherwise_domain_at(0, 24).is_none(),
        "无该坐标的记录 ⟹ None"
    );
    assert!(
        cls.otherwise_domain_at(9, 28).is_none(),
        "越界 level ⟹ None"
    );

    // 5) 增量路径同可查（bit-exact 含新字段）：逐段前缀重放，增量 == 全量。
    let mut cache = TowerCache::new();
    for n in 1..=segments.len() {
        let prefix = ParseLayer {
            segments: Rc::new(segments[..n].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let __co1 = classify_incremental(&prefix, &cfg, &mut cache, &[]);
        let inc_cls = __co1.classification;
        let _inc_tower = __co1.tower;
        let __co2 = classify(&prefix, &cfg, &[]);
        let full_cls = __co2.classification;
        let _full_tower = __co2.tower;
        assert_eq!(
            inc_cls, full_cls,
            "n={n}: 增量 Classification == 全量（含 first_class_grades，#885 新字段 bit-exact）"
        );
    }
    let __co3 = classify_incremental(&layer, &cfg, &mut TowerCache::new(), &[]);
    let inc_cls_final = __co3.classification;
    let _t = __co3.tower;
    let inc_rec = inc_cls_final
        .otherwise_domain_at(0, 28)
        .expect("增量路径同样按坐标可查");
    assert_eq!(inc_rec, rec, "增量/全量同一否则域记录");
}

/// ★#897 验收核心读数（#[ignore]，真实 BTC 数据，默认尾 300K bar 窗——截断窗=显式有效域）：
/// ① `Compose.centers` 长度分布（趋势 run 载荷后 1/2/3+ 桶，分级）——M-1 落地前全为 1；
/// ② `rmove_dir` 判定差：新判据（M-2 趋势 run 序列）vs 旧端点兼容缝逐 move 对拍，
/// 报告 一致 / 改判 / 新判出方向（旧 None 不可比）三类计数——#870 三臂重测的前置读数。
#[test]
#[ignore]
fn issue897_centers_run_distribution_btc() {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::classifier::cand_predicate::rmove_dir;
    use super::super::super::classifier::descend::RMove;
    use super::super::super::parser::parse_layer;
    let cfg = ThetaConfig::default();
    let full =
        load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    let n_full = full.bars.len();
    let max_bars = 300_000usize; // 截断窗（显式有效域边界，非全窗结论）
    let ds = if n_full > max_bars {
        full.slice_bar_range(n_full - max_bars, n_full)
    } else {
        full
    };
    let layer = parse_layer(&ds.bars, &cfg);
    let __co4 = classify(&layer, &cfg, &[]);
    let cls = __co4.classification;
    let tower = __co4.tower;

    let mut len_hist: std::collections::BTreeMap<(usize, usize), u64> = Default::default();
    let mut agree = 0u64;
    let mut changed = 0u64; // 旧 Some ↔ 新 Some 但方向不同 / 新旧 Some-None 互换
    let mut new_multi = 0u64; // centers ≥ 2 的 Compose 数（M-2 真正生效面）
    let mut multi_dir_some = 0u64; // 其中新判据给出方向的
    for (lvl, moves) in tower.iter().enumerate().skip(1) {
        for m in moves.iter() {
            if let RMove::Compose { centers, subs, .. } = &m.rmove {
                *len_hist.entry((lvl, centers.len().min(4))).or_insert(0) += 1;
                // 旧端点兼容缝（改前行为）：首末子走势 hi 端点比较。
                let old_dir = if subs.len() >= 2 {
                    let f = subs.first().unwrap();
                    let l = subs.last().unwrap();
                    Some(if l.hi() >= f.hi() {
                        Direction::Up
                    } else {
                        Direction::Down
                    })
                } else {
                    None
                };
                let new_dir = rmove_dir(&m.rmove);
                if centers.len() >= 2 {
                    new_multi += 1;
                    if new_dir.is_some() {
                        multi_dir_some += 1;
                    }
                }
                match (old_dir, new_dir) {
                    (a, b) if a == b => agree += 1,
                    _ => changed += 1,
                }
            }
        }
    }
    eprintln!(
        "★#897 读数（BTC 尾 {} bar / 全量 {}）：",
        ds.bars.len(),
        n_full
    );
    eprintln!("① Compose.centers 长度分布（(级别, 长度帽4)=计数）：");
    for ((lvl, len), cnt) in &len_hist {
        eprintln!(
            "   L{} len={}{}: {}",
            lvl,
            len,
            if *len == 4 { "+" } else { "" },
            cnt
        );
    }
    eprintln!(
        "② rmove_dir 对拍：一致 {agree} / 改判 {changed}；多中枢 Compose {new_multi}（其中新判据出方向 {multi_dir_some}）"
    );
    let _ = cls;
}
