use super::super::pipeline::{classify_level, extract_first_third_for_level, segment_to_unit};
use super::super::*;
use super::super::super::parser::ParseLayer;
use super::super::super::types::{Direction, MoveKind};
use super::{bars_from_closes, seg};

/// 两个 `--ignored` 真实数据普查（[`level_signal_census_btc`] / [`type1_funnel_census_btc`]）
/// 共享的 BTC 数据面：加载 → 可选切窗 → 解析。
///
/// 可选日期窗（`CENSUS_WINDOW="2020-10-01,2021-04-01"`）——检验 type1 的水平线依赖性：
/// 全历史中枢链全局非单调 ⟹ `trend_class=Degenerate` ⟹ type1=0；单向牛/熊窗内某级链可单调
/// ⟹ type1>0。
fn census_btc_dataset(
    tag: &str,
    cfg: &ThetaConfig,
) -> (super::super::super::backtest::data::Dataset, ParseLayer) {
    use super::super::super::backtest::data::load_by_symbol;
    use super::super::super::parser::parse_layer;
    let full =
        load_by_symbol("BTC", cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
    let ds = match std::env::var(crate::theta_v0::env_registry::CENSUS_WINDOW) {
        Ok(w) => {
            let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
            eprintln!("[{tag}] window={s}..{e}");
            full.slice_date_window(s, e)
        }
        Err(_) => full,
    };
    eprintln!("[{tag}] BTC bars={}", ds.bars.len());
    let layer = parse_layer(&ds.bars, cfg);
    eprintln!(
        "[{tag}] L0 segments={} merged_bars={}",
        layer.segments.len(),
        layer.merged_bars.len()
    );
    (ds, layer)
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
    let layer = l1_l2_geometric_fixture();
    let out = classify(&layer, &cfg);

    // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
    assert!(out.levels.len() >= 2, "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级");
    let l1 = &out.levels[1];
    assert_eq!(l1.centers.len(), 1, "3 个 L1 走势 → 1 个 L2 中枢（几何路径）");
    let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
    assert_eq!(second_buys.len(), 1, "★升级后塔真产 B2（旧 UnitRange 塔产 0）");
    let b2 = second_buys[0];
    // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
    assert_eq!(b2.source_index, 36, "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）");
    // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）——止损仍 pivot 非 center.zg/zd。
    assert!(b2.pivot_low != 0, "B2 携结构止损价 pivot_low（回拉低点 single source）");
    // ★#218 面 A（spec owner-attribution-fix-20260724 ID-1，机械改写归因：载体形态变化）：
    // 二类点归属载体从判定中枢 c1（次级别中枢，确认层对象）改载该走势一类点身份锚——
    // 第一类离开走势 m1（L1[1]，背驰次级别走势）的终点坐标（区间套：该走势终点极值点 =
    // 一类点）；止损仍 pivot（上条已锁，止损语义不变）。
    assert_eq!(b2.center, Some(signal::OwnerRef::Type1Anchor(24)), "二类点归属载体 = 该走势一类点锚（m1=L1[1] 终点坐标 24）");
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
        seg(Direction::Up,   0,  4, 100, 200),
        seg(Direction::Down, 4,  8, 200, 100),
        seg(Direction::Up,   8, 12, 100, 200),
    ];
    let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
    let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
    let out = classify(&layer, &cfg);
    // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
    for p in out.levels[0].bsp.iter() {
        assert!(!p.bits.buy2, "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）");
        assert!(!p.bits.sell2, "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）");
    }
}

/// ★codex-decide-20260703 裁定 A 最大实现风险点（end_price 忠实性，单测强制）：级别-N 输入单元
/// → Segment 的端点价按 fold_direction 取 hi/lo，且与 `segment_to_unit` 互逆（L0 段 round-trip
/// bit-exact）。此测试失败 ⟹ 级别-N「线段」端点价错位 ⟹ A/C 破中枢几何 + judge_third 判据全错。
#[test]
fn unit_to_segment_endpoint_faithful_and_roundtrips() {
    use super::super::center::UnitRange;
    // 向上单元：起点=lo、终点=hi（seg_end 取 end_price=hi=高点）。
    let up = UnitRange { start_index: 4, end_index: 8, direction: Direction::Up, lo: 90, hi: 150 };
    let s_up = unit_to_segment(&up);
    assert_eq!((s_up.start_price, s_up.end_price), (90, 150), "向上单元 end_price=hi（终点=高点）");
    assert_eq!(s_up.direction, Direction::Up);
    assert_eq!((s_up.start_index, s_up.end_index), (4, 8), "source_index 坐标保留（A/C 面积映射用）");
    // 向下单元：起点=hi、终点=lo（终点=低点）。
    let down = UnitRange { start_index: 8, end_index: 12, direction: Direction::Down, lo: 90, hi: 150 };
    let s_down = unit_to_segment(&down);
    assert_eq!((s_down.start_price, s_down.end_price), (150, 90), "向下单元 end_price=lo（终点=低点）");
    // round-trip：L0 段 → segment_to_unit → unit_to_segment == 原段（互逆 bit-exact）。
    for orig in [seg(Direction::Up, 0, 4, 100, 200), seg(Direction::Down, 4, 8, 200, 50)] {
        let back = unit_to_segment(&segment_to_unit(&orig));
        assert_eq!(
            (back.direction, back.start_index, back.end_index, back.start_price, back.end_price),
            (orig.direction, orig.start_index, orig.end_index, orig.start_price, orig.end_price),
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
    let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
    let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
    // Down 单元：lo=终点价、hi=起点价（unit_to_segment 还原 start=hi/end=lo）。
    let units = vec![
        UnitRange { start_index: 3, end_index: 5, direction: Direction::Down, lo: 250, hi: 350 }, // A 段（C0 离开）
        UnitRange { start_index: 5, end_index: 7, direction: Direction::Up, lo: 250, hi: 280 },   // B 段连接
        UnitRange { start_index: 9, end_index: 11, direction: Direction::Down, lo: 80, hi: 150 }, // C 段破 C1（<100）
    ];
    // A 段 bar[3,5] 急跌（hist 面积大）、C 段 bar[9,11] 缓动（面积小=背驰）——同 signal.rs fixture。
    let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
    let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
    let close_src: Vec<usize> = (0..prices.len()).collect();
    let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
    // 本测试只验结构六 bit（force 旁挂不改），传空 dif/closes_tick ⟹ force=None（不影响 buy1 判据）。
    // Q7-#1 裁定C：显式全锚（本测试验证的是 Trend ownership 单元的 gap-fill 路径）。
    let anchors = [Some(Direction::Down), Some(Direction::Up), Some(Direction::Down)];
    let (bsp, _pan) = extract_first_third_for_level(&[c0, c1], &units, &anchors, &hist, &[], &[], &close_src, divergence::DivergenceGauge::default());
    let buy1: Vec<_> = bsp.iter().filter(|p| p.bits.buy1).collect();
    assert_eq!(buy1.len(), 1, "级别-N 下跌趋势 C 段破最后中枢 ∧ C<A 背驰 ⟹ 一个 1 买（缺口已填，非 no-op）");
    assert_eq!(buy1[0].source_index, 11, "1 买端点 = C 段（破最后中枢单元）终止 source_index");
    assert_eq!(buy1[0].pivot_low, 80, "1 买止损源 = pivot_low（C 段破中枢端点极值）");
    // ★owner 载体补齐（关③ 补记② 路径 (a)）：一类点构造时填入判定中枢 last_center=c1
    //（被破的最后中枢）——名实一致根据同 signal.rs `first_buy_extracted_with_trend_divergence`
    //（本测试复用其 A/B/C 几何的 UnitRange 表达）；center 是 owner 载体，止损仍 pivot
    //（pivot_low=80 上条已锁，center 不进 1/2 类止损判据）。
    // （#218 面 A 载体形态机械适配：一/三类载 OwnerRef::Center，语义不动。）
    assert_eq!(buy1[0].center, Some(signal::OwnerRef::Center(c1)), "一类点 center = 判定中枢（owner 载体）；止损仍 pivot 非 center");
}

/// ★裁定 A 三类（高级别「中枢外缘区间」边界语义，codex 风险点单独 snapshot）：级别-N 离开中枢
/// + 回试不重入 ⟹ 3 买。`judge_third` 在级别-N units（外缘区间端点 hi/lo）vs 几何中枢 [zd,zg]
/// 上判定——离开单元终点 > c.zg ∧ 回试单元终点 > c.zg（严格不触闭区间）。
#[test]
fn level_ge1_extract_first_third_produces_type3_via_units() {
    use super::super::center::UnitRange;
    // 单中枢 [100,200]（盘整 τ ⟹ 无一类）——三类是纯几何位置判据，不依赖趋势门控。
    let c = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 };
    let units = vec![
        // 离开单元：向上，终点=hi=250 > zg=200（离开中枢上方）。
        UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 150, hi: 250 },
        // 回试单元：向下，终点=lo=210 > zg=200（不重入闭区间中枢）⟹ 3 买。
        UnitRange { start_index: 16, end_index: 20, direction: Direction::Down, lo: 210, hi: 250 },
    ];
    // 三类无 MACD 依赖（纯整数几何），hist 空亦可——传空 hist/dif/closes_tick（第一类自然不产，force=None）。
    // Q7-#1 裁定C：显式全锚（leave 单元有 Trend ownership 资格的三类路径）。
    let anchors = [Some(Direction::Up), Some(Direction::Down)];
    let (bsp, _pan) = extract_first_third_for_level(&[c], &units, &anchors, &[], &[], &[], &(0..24).collect::<Vec<_>>(), divergence::DivergenceGauge::default());
    let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
    assert_eq!(buy3.len(), 1, "级别-N 离开中枢 + 回试不重入 ⟹ 一个 3 买（外缘区间端点判据）");
    assert_eq!(buy3[0].source_index, 20, "3 买端点 = 回试单元终止 source_index");
    assert_eq!(buy3[0].pivot_low, 210, "3 买止损源 = pivot_low（回试低点）");
    assert_eq!(
        buy3[0].center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
        Some(200),
        "3 买 center=Some（止损=zg；#218 面 A 载体形态：Center 变体读出）"
    );
}

/// ★Q7-#1 裁定C + p117 窄域授权（686 翻转条款第一支，终端背书裁定 T2 核准）：Consolidation
/// ownership 的 endpoint fallback 单元（anchor=None）在**第一类路径**经窄域授权降级——方向锚
/// 取单元结构方向（`anchors_self`，行程方向=τ 等式 veto，「趋势中」定义域由 τ 门承担），
/// fallback 不再是第一类击杀理由；**三类路径**裁定C 整体保留（leave 段 fallback 仍不得作
/// 三类方向锚）。三向验证（同 fixture 对照）：全锚 ⟹ 1 买产（对照组，保留）；A 段 fallback
/// ⟹ 结构同向筛选可配 ⟹ 产 1 买（0→1 授权翻转）；C 段 fallback ⟹ 结构方向 Down=τ ⟹ broke
/// 触发 ⟹ 产 1 买（0→1 授权翻转）。三类：leave 段 fallback ⟹ 不产 3 买（保留）。成员身份
/// 不变（centers/分解不受锚门影响）。
#[test]
fn q7_ruling_c_first_class_structural_direction_third_class_provenance_kept() {
    use super::super::center::UnitRange;
    // fixture 同 level_ge1_extract_first_third_fills_type1_gap（两下行中枢 + A/B/C 三单元）。
    let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
    let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
    let units = vec![
        UnitRange { start_index: 3, end_index: 5, direction: Direction::Down, lo: 250, hi: 350 },
        UnitRange { start_index: 5, end_index: 7, direction: Direction::Up, lo: 250, hi: 280 },
        UnitRange { start_index: 9, end_index: 11, direction: Direction::Down, lo: 80, hi: 150 },
    ];
    let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
    let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
    let close_src: Vec<usize> = (0..prices.len()).collect();
    let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
    let n_buy1 = |anchors: &[Option<Direction>]| {
        let (bsp, _) = extract_first_third_for_level(&[c0, c1], &units, anchors, &hist, &[], &[], &close_src, divergence::DivergenceGauge::default());
        bsp.iter().filter(|p| p.bits.buy1).count()
    };
    // 对照组：全锚 ⟹ 1 买产（gap-fill 路径活；保留断言）。
    assert_eq!(n_buy1(&[Some(Direction::Down), Some(Direction::Up), Some(Direction::Down)]), 1);
    // A 段单元 fallback：p117 后第一类 A 窗筛选用单元结构方向（行程方向 Down=τ）⟹ A 候选
    // 可配 ⟹ 产 1 买（S4 救回型；授权翻转 0→1。provenance 锚消费方仅余三类/诊断仪器）。
    assert_eq!(n_buy1(&[None, Some(Direction::Up), Some(Direction::Down)]), 1,
        "p117 窄域授权：第一类 A 段方向锚 = 单元结构方向（fallback 单元行程方向=τ 时可配）");
    // C 段（破中枢段）单元 fallback：p117 后 broke 锚门用单元结构方向（Down=τ）⟹ 触发 ⟹
    // 产 1 买（S2 救回型；授权翻转 0→1。行程方向 ≠τ 的单元仍拒——signal.rs 反向 veto 锁）。
    assert_eq!(n_buy1(&[Some(Direction::Down), Some(Direction::Up), None]), 1,
        "p117 窄域授权：第一类破中枢段方向锚 = 单元结构方向（fallback 单元行程方向=τ 时触发）");
    // 三类：leave 段 fallback ⟹ 不产 3 买（686 对三类的保护整体保留；retest 是几何角色不设锚门）。
    let c = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 };
    let u3 = vec![
        UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 150, hi: 250 },
        UnitRange { start_index: 16, end_index: 20, direction: Direction::Down, lo: 210, hi: 250 },
    ];
    let src24: Vec<usize> = (0..24).collect();
    let (bsp, _) = extract_first_third_for_level(&[c], &u3, &[None, Some(Direction::Down)], &[], &[], &[], &src24, divergence::DivergenceGauge::default());
    assert_eq!(bsp.iter().filter(|p| p.bits.buy3).count(), 0, "fallback 单元不得作三类离开段方向锚（686 裁定C 三类保留）");
    // 成员身份不变：同 fixture 全锚下产出恢复（锚门不改变序列成员/中枢几何）。
    let (bsp2, _) = extract_first_third_for_level(&[c], &u3, &[Some(Direction::Up), Some(Direction::Down)], &[], &[], &[], &src24, divergence::DivergenceGauge::default());
    assert_eq!(bsp2.iter().filter(|p| p.bits.buy3).count(), 1);
}

/// ★L2 信号普查诊断（裁定 A gap-fill 真实数据核验，`--ignored` 手动跑，依赖 analysis/data_cache）：
/// 逐级别统计 BTC 全量分类的 buy1/sell1/buy2/sell2/buy3/sell3 计数 + 抽样 level≥1 一类端点。
/// 修前 level≥1 一/三类恒 0（audit #119 `else{Vec::new()}`），故 level≥1 的 type1/type3 计数 =
/// 本次实装引入的净增信号。运行：`cargo test --lib -- --ignored --nocapture level_signal_census_btc`。
#[test]
#[ignore = "L2 真实数据普查：cargo test --lib -- --ignored --nocapture level_signal_census_btc"]
fn level_signal_census_btc() {
    let cfg = ThetaConfig::default();
    let (_ds, layer) = census_btc_dataset("census", &cfg);
    let out = classify(&layer, &cfg);
    eprintln!("[census] levels={}", out.levels.len());
    for (li, lv) in out.levels.iter().enumerate() {
        let trend = lv.moves.iter().filter(|m| m.kind == MoveKind::Trend).count();
        let (mut b1, mut s1, mut b2, mut s2, mut b3, mut s3) = (0, 0, 0, 0, 0, 0);
        for p in lv.bsp.iter() {
            b1 += p.bits.buy1 as usize; s1 += p.bits.sell1 as usize;
            b2 += p.bits.buy2 as usize; s2 += p.bits.sell2 as usize;
            b3 += p.bits.buy3 as usize; s3 += p.bits.sell3 as usize;
        }
        eprintln!(
            "[census] L{li}: centers={} moves={}(trend={}) bsp={} pan_div={} | buy1={b1} sell1={s1} buy2={b2} sell2={s2} buy3={b3} sell3={s3}",
            lv.centers.len(), lv.moves.len(), trend, lv.bsp.len(), lv.pan_div.len()
        );
        if li >= 1 {
            report_level_ge1_endpoint_samples(li, lv);
        }
    }
}

/// 抽样：level≥1 的前 3 个一类端点（若有）+ 前 3 个三类端点（人工核结构合法性——
/// `source_index` + center`[zd,zg]` + pivot（回试端点极值）；三类不重入判据由 `judge_third` 保证）。
fn report_level_ge1_endpoint_samples(li: usize, lv: &LevelState) {
    let t1: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy1 || p.bits.sell1).take(3).collect();
    for (k, p) in t1.iter().enumerate() {
        eprintln!(
            "[census]   L{li} type1#{k}: src_idx={} buy1={} sell1={} break_dir={:?} pivot_low={} pivot_high={}",
            p.source_index, p.bits.buy1, p.bits.sell1, p.struct_break_dir, p.pivot_low, p.pivot_high
        );
    }
    let t3: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy3 || p.bits.sell3).take(3).collect();
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
    let cfg = ThetaConfig::default();
    let (ds, layer) = census_btc_dataset("funnel", &cfg);
    // 生产对拍源（675号守卫：级别循环不分叉）。
    let out = classify(&layer, &cfg);

    // classify_impl 同源输入（同一批私有函数，非重写）。
    let min_parts = cfg.level.min_parts_per_level as usize;
    let l_max = cfg.level.l_max as usize;
    let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
    assert!(!units.is_empty(), "空 L0 无漏斗对象");
    let (series, closes_tick, close_src) = funnel_probe_series(&layer, &cfg);
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
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

        report_window_span_histogram(level_idx, &units, is_l0, &centers);
        let chain = center_chain_stats(&centers, &ds);

        let (segs, funnel_anchors) = funnel_level_geometry(&layer, &out, &units, level_idx, is_l0);
        let f = signal::type1_funnel_dx(&centers, &segs, funnel_anchors.as_deref(), &series.hist, &series.dif, &closes_tick, &close_src);
        report_funnel_line(level_idx, &f, &chain);
        report_top_region_sell1(level_idx, &out.levels[level_idx], &ds);

        let (_cw, upper_moves, _) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        units = project_to_units(&upper_moves, &out.levels[level_idx].moves); // Q7：生产同源块
        moves_tower = Rc::new(upper_moves);
        if units.is_empty() {
            break;
        }
    }
}

/// ★task #142 量化验收：延伸段数分布（窗口段数 = seed 3 + 延伸段；同一 build 直调
/// `detect_centers_windowed_resume` 取窗口，中枢序列与生产 `classify_level` 对拍）。
fn report_window_span_histogram(
    level_idx: usize,
    units: &[UnitRange],
    is_l0: bool,
    centers: &[Center],
) {
    let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
        center::center_from_segments
    } else {
        center::center_from_window
    };
    let windowed = recursive_tower::detect_centers_windowed_resume(units, build, 0).0;
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
        hist[if w <= 3 { 0 } else if w <= 5 { 1 } else if w <= 9 { 2 } else { 3 }] += 1;
    }
    eprintln!(
        "[funnel] L{level_idx}: 窗口段数分布 =3段:{} 4-5:{} 6-9:{} ≥10:{} max={}",
        hist[0], hist[1], hist[2], hist[3], max_w
    );
}

/// [`center_chain_stats`] 的产出：中枢链相邻关系直方图 + 前缀 τ 锁死点 + 反事实局部同向 run。
struct CenterChainStats {
    n_up: usize,
    n_down: usize,
    n_exp: usize,
    runs_ge1: usize,
    longest_run: usize,
    lock_desc: String,
}

/// 中枢链相邻关系直方图 + 前缀 τ 时间线 + 反事实局部同向 run。
///
/// 前缀 τ：`τ(前k中枢)=Trend ⟺ k≥2 ∧ rels[0..k-1] 全等且非 Expansion`。锁死点 = 首个异关系下标。
/// 反事实（若走势分解为局部走势类型）：同向关系（Up/Down）的极大 run，每个 run 长 L = 局部趋势
/// 含 L+1 个中枢。计 run 数与最长 run。
fn center_chain_stats(
    centers: &[Center],
    ds: &super::super::super::backtest::data::Dataset,
) -> CenterChainStats {
    use super::super::center::{classify_relation, CenterRelation};
    let rels: Vec<CenterRelation> =
        centers.windows(2).map(|w| classify_relation(&w[0], &w[1])).collect();
    let n_up = rels.iter().filter(|r| **r == CenterRelation::UpContinuation).count();
    let n_down = rels.iter().filter(|r| **r == CenterRelation::DownContinuation).count();
    let n_exp = rels.iter().filter(|r| **r == CenterRelation::LevelExpansion).count();
    let trend_open = !rels.is_empty() && rels[0] != CenterRelation::LevelExpansion;
    let lock_at = if rels.is_empty() {
        None
    } else if !trend_open {
        Some(0) // 首关系即 Expansion ⟹ 第3个中枢确认时 τ 已锁死 Degenerate
    } else {
        rels.iter().position(|r| *r != rels[0])
    };
    let (mut runs_ge1, mut longest_run, mut cur_run) = (0usize, 0usize, 0usize);
    for (k, r) in rels.iter().enumerate() {
        let same_dir = *r != CenterRelation::LevelExpansion;
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
        None if trend_open => "全链同向（不锁死）".to_string(),
        None => "链长<2 无关系".to_string(),
        Some(i) => {
            let c_end = centers[i + 1].end_index;
            let date = ds.dates.get(c_end).map(|d| d.get(..10).unwrap_or("?")).unwrap_or("?");
            format!("中枢#{}（end_src={} {date}）", i + 1, c_end)
        }
    };
    CenterChainStats { n_up, n_down, n_exp, runs_ge1, longest_run, lock_desc }
}

/// ★task #144 验收证据：2021 顶区 sell1 在全历史因果重放中出现（先例窗反差闭合的正验证，
/// 生产 `classify` 输出直读——非探针另算）。窗口 = #141 外审切窗 2020-10-01..2021-04-15。
fn report_top_region_sell1(
    level_idx: usize,
    lv: &LevelState,
    ds: &super::super::super::backtest::data::Dataset,
) {
    let top_sell1: Vec<&str> = lv
        .bsp
        .iter()
        .filter(|p| p.bits.sell1)
        .filter_map(|p| ds.dates.get(p.source_index).map(|d| d.get(..10).unwrap_or("?")))
        .filter(|d| ("2020-10-01".."2021-04-15").contains(d))
        .collect();
    let n_sell1 = lv.bsp.iter().filter(|p| p.bits.sell1).count();
    let n_buy1 = lv.bsp.iter().filter(|p| p.bits.buy1).count();
    eprintln!(
        "[funnel] L{level_idx}: 全历史 buy1={} sell1={} | 2021顶区(2020-10-01..2021-04-15) sell1×{}: {:?}",
        n_buy1, n_sell1, top_sell1.len(), top_sell1
    );
}

/// 漏斗探针的 MACD/坐标序列面（与 `classify_impl` 的 `full_macd_series` 同口径：同一
/// `compute_macd` 单趟 + 同一 `merged_bars` 投影）。
fn funnel_probe_series(
    layer: &ParseLayer,
    cfg: &ThetaConfig,
) -> (divergence::MacdSeries, Vec<Tick>, Vec<usize>) {
    let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
    let series = divergence::compute_macd(&closes, &cfg.macd);
    let closes_tick: Vec<Tick> = layer.merged_bars.iter().map(|b| b.close).collect();
    (series, closes_tick, close_src)
}

/// 逐级漏斗两行报告：结构概览（中枢/段/关系/块/锁死点/反事实 run）+ 六环逐环计数。
fn report_funnel_line(level_idx: usize, f: &signal::Type1Funnel, chain: &CenterChainStats) {
    eprintln!(
        "[funnel] L{level_idx}: centers={} segs={} rel(up/down/exp)={}/{}/{} blocks(trend/consol)={}/{} 最长趋势块={}中枢 | 旧AllTrend锁死点={} 局部同向run≥2中枢数={} 最长run={}(={}中枢)",
        f.n_centers, f.n_segments, chain.n_up, chain.n_down, chain.n_exp,
        f.n_trend_blocks, f.n_consol_blocks, f.longest_trend_run, chain.lock_desc,
        chain.runs_ge1, chain.longest_run, chain.longest_run + 1
    );
    eprintln!(
        "[funnel] L{level_idx}: 环0候选(有最近中枢)={} → 环1有前驱中枢={} → 环2过局部趋势门={} → 环3破最后中枢={} → 环4 A/C配对={} → 环4b 037:20破b极值={} → 环5坐标映射={} → 环6背驰C<A={}",
        f.s_with_center, f.s_pos_ge1, f.s_gate_open, f.s_broke, f.s_a_paired, f.s_extreme, f.s_mapped, f.s_diverge
    );
}

/// 漏斗探针的逐级几何输入：L0 用 parser 线段账本；L≥1 用 units→Segment 投影 + 方向锚。
///
/// Q7-#1 裁定C + 675号：探针锚与生产 `units_anchors` 同源（producer blocks 派生），不另起一套。
fn funnel_level_geometry(
    layer: &ParseLayer,
    out: &Classification,
    units: &[UnitRange],
    level_idx: usize,
    is_l0: bool,
) -> (Vec<Segment>, Option<Vec<Option<Direction>>>) {
    if is_l0 {
        return (layer.segments.to_vec(), None);
    }
    let pb = &out.levels[level_idx - 1].moves;
    (
        units.iter().map(unit_to_segment).collect(),
        Some((0..units.len()).map(|i| decompose::center_own_dir_at(pb, i)).collect()),
    )
}

/// [`end_to_end_second_buy_via_l1_l2_geometric`] 的 9 段 L0 夹具（三组，每组 → 一个 L1 走势）。
///
/// ★中枢延伸语义下的诚实重算（PDF §5，task #142）：组间首段必须与前组**冻结核心 [ZD,ZG]**
/// 不相交（Step3 non-extension），否则整串被 Step2 吸收为 1 个延伸中枢 ⟹ 塔不生长
/// （旧全触及 fixture 的坍缩后果）。推导：
/// - 组A up-down-up：核心 `K_A=[max(110,120,120),min(150,150,148)]=[120,148]`，外缘 `O_A=[110,150]`。
/// - 组B down-up-down：首段 `[80,115]` hi=115 < ZD_A=120 ⟹ non-extension（组间分离）；
///   核心 `K_B=[max(80,80,85),min(115,125,114)]=[85,114]`，外缘 `O_B=[80,125]`。
/// - 组C up-down-up：首段 `[115,148]` lo=115 > ZG_B=114 ⟹ non-extension；
///   核心 `K_C=[max(115,112,112),min(148,148,147)]=[115,147]`，外缘 `O_C=[112,148]`。
///
/// L2 核心（几何路径，三 L1 外缘交）= `[max(110,80,112), min(150,125,148)] = [112,125]` 非空。
/// B2 结构：`L1[1].lo=80 < ZD2=112` 深破 L2 核心下沿（第一类离开候选，`Side::Long`）；
/// `L1[2]` 回拉不创新低（`lo=112 >= L1[1].lo=80`）；`L1[0]`/`L1[1]` 外缘占位方向同 Down
/// （末子 hi < 首子 hi：148<150 / 114<115）⟹ 背驰可配对（closes 前大后小）。
///
/// closes 让 `L1[1]` 区间（`source_index [12,24]`）MACD 面积 < `L1[0]` 区间（`[0,12]`）= 背驰
/// （真算）：前段大幅波动（面积大），后段小幅（面积小）。
fn l1_l2_geometric_fixture() -> ParseLayer {
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
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        // 组B（L1[1]）：down-up-down，首段 hi=115<ZD_A=120 non-ext，lo=80 深破 L2 核心下沿 112
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        // 组C（L1[2]）：up-down-up，首段 lo=115>ZG_B=114 non-ext，回拉不创新低（lo=112 >= 80）
        seg(Direction::Up,  24, 28, 115, 148),
        seg(Direction::Down,28, 32, 148, 112),
        seg(Direction::Up,  32, 36, 112, 147),
    ];
    // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
    // 前段大幅波动（面积大），后段小幅（面积小）。
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); } // L1[0] 大幅
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 5 } else { -5 }); }   // L1[1] 小幅（背驰）
    for i in 0..16 { closes.push(100 + if i % 2 == 0 { 3 } else { -3 }); }   // L1[2] 更小
    let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
    layer
}
