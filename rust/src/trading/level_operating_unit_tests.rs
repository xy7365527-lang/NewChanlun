use super::*;

fn cfg_rev() -> OrganicConfig {
    OrganicConfig {
        rev_mode: true,
        ..OrganicConfig::default()
    }
}

fn ev(class: BspClass, confirmed: bool) -> BspEvent {
    BspEvent {
        class,
        seg_idx: 0,
        confirmed,
        cs: None,
        zd: None,
        zg: None,
        price: 10.0,
    }
}

fn ev_anchored(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
    BspEvent {
        class,
        seg_idx: 0,
        confirmed,
        cs: Some(cs),
        zd: Some(zd),
        zg: Some(zg),
        price: 10.0,
    }
}

fn empty_rows<'a>(
    evs: &'a [Vec<BspEvent>; MAX_LADDER],
    devs: &'a [Vec<DivEvent>; MAX_LADDER],
) -> BarRows<'a> {
    BarRows {
        evs,
        devs,
        buy_any: LadderMask(0),
        sell_any: LadderMask(0),
        dir_row: None,
        run_anchor: None,
        depth: None,
        l41: None,
        trend_row: None,
    }
}

struct Fixture {
    evs: [Vec<BspEvent>; MAX_LADDER],
    devs: [Vec<DivEvent>; MAX_LADDER],
    ledger: OrganicLedger,
    book: CenterBook,
    gate: FatigueGate,
    counters: Counters,
}

impl Fixture {
    fn new() -> Self {
        Fixture {
            evs: Default::default(),
            devs: Default::default(),
            ledger: OrganicLedger::new(10.0, 100.0, 0.5, false),
            book: CenterBook::new(),
            gate: FatigueGate::new(),
            counters: Counters::default(),
        }
    }
}

#[test]
fn t1_opens_rev_without_touching_osc() {
    // C2 验证：osc 开放时 REV 开腿，osc 槽不被截断
    let mut fx = Fixture::new();
    // 先放一个存活中枢并开 osc 腿
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    assert!(fx.ledger.open_diff(
        SlotKey::osc(2),
        0.5,
        10.0,
        0,
        LegAnchor::Center {
            cs: Some(1),
            boundary: Some(9.0),
            zg: Some(9.5),
            kind: AnchorKind::Osc
        },
    ));
    // 段终结触发（confirmed Sell1）
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_rev(),
        &rows,
        &[],
        9.6,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_open, 1);
    // osc 槽仍开放（v1 的"先 CLOSE_OSC"在 v2 没有代码位）
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_some());
}

/// 出口1（osc_shift_close）：中枢向上移动（新中枢 ZD > 锚中枢 ZG）⇒
/// 旧中枢 osc 空腿立即回补（49课"中枢向上移动时就应该满仓"）。
/// 僵尸场景再现：价格不回 ZD（无触线）、锚中枢未死（无 type3 确认）——
/// 在册行为是腿悬挂至 master 强平；本出口在结构层闭腿。
#[test]
fn osc_shift_close_kills_zombie_leg() {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        osc_shift_close: true,
        ..OrganicConfig::default()
    };
    // 锚中枢 cs=1 [9.0, 9.5]，osc 空腿挂锚（zg 快照 = 9.5）
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    assert!(fx.ledger.open_diff(
        SlotKey::osc(2),
        0.5,
        10.0,
        0,
        LegAnchor::Center {
            cs: Some(1),
            boundary: Some(9.0),
            zg: Some(9.5),
            kind: AnchorKind::Osc
        },
    ));
    // 新中枢 cs=5 [10.0, 11.0] 形成——新 ZD 10.0 > 锚 ZG 9.5 = 中枢向上移动
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 5, 10.0, 11.0)],
        true,
        None,
    );
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    // c=10.5：不触旧 ZD（9.0）、锚中枢未死——僵尸条件成立
    v.step(
        &cfg,
        &rows,
        &[],
        10.5,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert!(
        fx.ledger.open_slot(SlotKey::osc(2)).is_none(),
        "中枢上移必须回补"
    );
    assert_eq!(fx.counters.n_osc_shift_close, 1);
    assert_eq!(fx.counters.n_osc_zd_close, 0);
}

/// 出口1负控（判据边界）：新中枢形成但 ZD ≤ 锚 ZG（横向/重叠移动）⇒
/// 不是"中枢向上移动"，出口不触发——OKLO/BRN 正常腿不受影响的微观基础。
#[test]
fn osc_shift_close_ignores_overlapping_center() {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        osc_shift_close: true,
        ..OrganicConfig::default()
    };
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    assert!(fx.ledger.open_diff(
        SlotKey::osc(2),
        0.5,
        10.0,
        0,
        LegAnchor::Center {
            cs: Some(1),
            boundary: Some(9.0),
            zg: Some(9.5),
            kind: AnchorKind::Osc
        },
    ));
    // 新中枢 [9.2, 9.8]：ZD 9.2 ≤ 锚 ZG 9.5 ⇒ 与旧中枢重叠，非上移
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 5, 9.2, 9.8)],
        true,
        None,
    );
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.6,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert!(
        fx.ledger.open_slot(SlotKey::osc(2)).is_some(),
        "重叠中枢不触发出口"
    );
    assert_eq!(fx.counters.n_osc_shift_close, 0);
}

/// 出口1负控（O0≡P5 零接触面）：开关关时同一上移场景不闭腿——
/// 在册行为逐位保持。
#[test]
fn osc_shift_close_off_preserves_p5_behavior() {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig::default(); // osc_shift_close=false
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    assert!(fx.ledger.open_diff(
        SlotKey::osc(2),
        0.5,
        10.0,
        0,
        LegAnchor::Center {
            cs: Some(1),
            boundary: Some(9.0),
            zg: Some(9.5),
            kind: AnchorKind::Osc
        },
    ));
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 5, 10.0, 11.0)],
        true,
        None,
    );
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        10.5,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert!(
        fx.ledger.open_slot(SlotKey::osc(2)).is_some(),
        "开关关 = 在册行为"
    );
    assert_eq!(fx.counters.n_osc_shift_close, 0);
}

/// 38课循环：进入（宿主趋势态）→ 卖点开 → 次级别买点闭 → 同窗口再开 →
/// 趋势态翻落强闭退出。循环内可多次开闭是与单次 REV 腿的判别性差异。
#[test]
fn cycle38_loop_open_close_reopen_and_forced_exit() {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_mode: true,
        rev_cycle: RevCycle::Cycle38,
        ..OrganicConfig::default()
    };
    // 成本门参照：12 个 1% 振幅中枢（θ_q=1% ≥ 2×10bps 下界）
    let mut dr = super::super::depth_ref::DepthRef::new(50);
    for i in 0..12 {
        fx.book.ingest(
            2,
            &[ev_anchored(BspClass::Sell1, true, 100 + i as i64, 9.0, 9.1)],
            true,
            None,
        );
        dr.observe(&fx.book, 10.0);
    }
    let mut trow = [false; MAX_LADDER];
    let mut dir = [None; MAX_LADDER];
    let mut v = VoiceUnit::new(2);
    macro_rules! step {
        ($buy_mask:expr, $c:expr, $bar:expr) => {{
            let rows = BarRows {
                evs: &fx.evs,
                devs: &fx.devs,
                buy_any: LadderMask($buy_mask),
                sell_any: LadderMask(0),
                dir_row: Some(&dir),
                run_anchor: None,
                depth: Some(&dr),
                l41: None,
                trend_row: Some(&trow),
            };
            v.step(
                &cfg,
                &rows,
                &[],
                $c,
                $bar,
                &mut fx.ledger,
                &fx.book,
                &fx.gate,
                4,
                &|_| 0.5,
                &mut fx.counters,
            );
        }};
    }
    let key = SlotKey::rev(2, 2);
    // bar0：宿主非趋势态 → 不进入循环（卖点存在也不开）
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    step!(0, 10.0, 0);
    assert_eq!(fx.counters.n_c38_enter, 0);
    assert_eq!(fx.counters.n_c38_open, 0);
    // bar1：宿主趋势态成立（kind=Trend ∧ dir=Up）→ 进入循环 + 卖点开腿
    trow[3] = true;
    dir[3] = Some(Direction::Up);
    step!(0, 10.0, 1);
    assert_eq!(fx.counters.n_c38_enter, 1);
    assert_eq!(fx.counters.n_c38_open, 1);
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert!(fx.ledger.open_slot(key).is_some());
    // bar2：次级别（ladder1）买点 → 买回（盈利短差），仍在循环内
    fx.evs[2].clear();
    step!(1 << 1, 9.5, 2);
    assert_eq!(fx.counters.n_c38_close, 1);
    assert_eq!(fx.counters.c38_pairs, 1);
    assert_eq!(fx.counters.c38_wins, 1);
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert!(fx.ledger.open_slot(key).is_none());
    assert_eq!(fx.counters.n_c38_exit, 0);
    // bar3：同循环窗口内再次卖点 → 重开（单次 REV 腿做不到的判别点）
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    step!(0, 10.2, 3);
    assert_eq!(fx.counters.n_c38_open, 2);
    // bar4：宿主趋势态翻落 → 未决腿强闭（亏损短差）+ 退出循环
    trow[3] = false;
    fx.evs[2].clear();
    step!(0, 10.5, 4);
    assert_eq!(fx.counters.n_c38_forced_close, 1);
    assert_eq!(fx.counters.n_c38_exit, 1);
    assert_eq!(fx.counters.c38_pairs, 2);
    assert_eq!(fx.counters.c38_wins, 1);
    assert!(fx.ledger.open_slot(key).is_none());
    // bar5：循环已退出 → 卖点不再开腿
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    step!(0, 10.0, 5);
    assert_eq!(fx.counters.n_c38_open, 2);
    // 成本门全程未拒（参照充足且振幅过下界）
    assert_eq!(fx.counters.n_c38_cost_rejects, 0);
    assert_eq!(fx.counters.n_c38_cost_noref_rejects, 0);
}

/// 成本门：warm-up 参照不足 → 保守拒绝（不静默放行）；振幅过薄 → 级别关闭。
#[test]
fn cycle38_cost_gate_rejects() {
    for (rel_amp, n_centers, want_cost, want_noref) in [(0.01, 5, 0u64, 1u64), (0.0005, 12, 1, 0)] {
        let mut fx = Fixture::new();
        let cfg = OrganicConfig {
            rev_mode: true,
            rev_cycle: RevCycle::Cycle38,
            ..OrganicConfig::default()
        };
        let mut dr = super::super::depth_ref::DepthRef::new(50);
        for i in 0..n_centers {
            fx.book.ingest(
                2,
                &[ev_anchored(
                    BspClass::Sell1,
                    true,
                    100 + i as i64,
                    9.0,
                    9.0 + rel_amp * 10.0,
                )],
                true,
                None,
            );
            dr.observe(&fx.book, 10.0);
        }
        let mut trow = [false; MAX_LADDER];
        trow[3] = true;
        let mut dir = [None; MAX_LADDER];
        dir[3] = Some(Direction::Up);
        let mut v = VoiceUnit::new(2);
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(&dir),
            run_anchor: None,
            depth: Some(&dr),
            l41: None,
            trend_row: Some(&trow),
        };
        v.step(
            &cfg,
            &rows,
            &[],
            10.0,
            0,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
        assert_eq!(fx.counters.n_c38_open, 0);
        assert_eq!(fx.counters.n_c38_cost_rejects, want_cost);
        assert_eq!(fx.counters.n_c38_cost_noref_rejects, want_noref);
        // 进入循环本身不被成本门阻止（门在开腿端）
        assert_eq!(fx.counters.n_c38_enter, 1);
    }
}

/// 买回判据消融公共夹具：12 个 1% 振幅中枢（成本门参照充足 + 存活锚
/// cs=111/zd=9.0/zg=9.1）+ 宿主趋势态成立 + Sell1 开腿。返回开腿完毕、
/// 处于 DownLeg 的 (fixture, voice, depth_ref, trow, dir)。
fn c38_close_fixture() -> (Fixture, VoiceUnit, super::super::depth_ref::DepthRef) {
    let mut fx = Fixture::new();
    let mut dr = super::super::depth_ref::DepthRef::new(50);
    for i in 0..12 {
        fx.book.ingest(
            2,
            &[ev_anchored(BspClass::Sell1, true, 100 + i as i64, 9.0, 9.1)],
            true,
            None,
        );
        dr.observe(&fx.book, 10.0);
    }
    let v = VoiceUnit::new(2);
    (fx, v, dr)
}

/// 消融轴步进宏的函数形式（trow/dir 固定为宿主趋势态成立）。
#[allow(clippy::too_many_arguments)]
fn c38_step(
    fx: &mut Fixture,
    v: &mut VoiceUnit,
    dr: &super::super::depth_ref::DepthRef,
    cfg: &OrganicConfig,
    buy_mask: u16,
    c: f64,
    bar: i64,
) {
    let mut trow = [false; MAX_LADDER];
    trow[3] = true;
    let mut dir = [None; MAX_LADDER];
    dir[3] = Some(Direction::Up);
    let rows = BarRows {
        evs: &fx.evs,
        devs: &fx.devs,
        buy_any: LadderMask(buy_mask),
        sell_any: LadderMask(0),
        dir_row: Some(&dir),
        run_anchor: None,
        depth: Some(dr),
        l41: None,
        trend_row: Some(&trow),
    };
    v.step(
        cfg,
        &rows,
        &[],
        c,
        bar,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
}

fn cfg_c38(close: RevCycleClose) -> OrganicConfig {
    OrganicConfig {
        rev_mode: true,
        rev_cycle: RevCycle::Cycle38,
        rev_cycle_close: close,
        ..OrganicConfig::default()
    }
}

/// C38buy1：次级别任意买点**不再**闭腿（与 SubAny 的判别性差异）；
/// 异锚 confirmed Buy1 不闭；同锚 confirmed Buy1 闭（reason 5）。
#[test]
fn cycle38_close_buy1_requires_same_anchor() {
    let cfg = cfg_c38(RevCycleClose::Buy1);
    let (mut fx, mut v, dr) = c38_close_fixture();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
    assert_eq!(fx.counters.n_c38_open, 1);
    // 开腿携带锚快照（同锚比较基准）
    let rev = v.rev.as_ref().expect("DownLeg ⇒ rev 存在");
    assert_eq!(rev.anchor_cs, Some(111));
    assert_eq!(rev.zd, Some(9.0));
    // 次级别任意买点 → 不闭（SubAny 的死因在此被切除）
    fx.evs[2].clear();
    c38_step(&mut fx, &mut v, &dr, &cfg, 1 << 1, 9.5, 1);
    assert_eq!(fx.counters.n_c38_close, 0);
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // 异锚 confirmed Buy1 → 不闭（同锚要求）
    fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 50, 9.0, 9.1)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.5, 2);
    assert_eq!(fx.counters.n_c38_close, 0);
    // 同锚 confirmed Buy1 → 闭（盈利短差）
    fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 111, 9.0, 9.1)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.4, 3);
    assert_eq!(fx.counters.n_c38_close, 1);
    assert_eq!(fx.counters.n_c38_close_buy1, 1);
    assert_eq!(fx.counters.c38_wins, 1);
    assert_eq!(v.phase, VoicePhase::UpLeg);
}

/// C38zd：ZD 触线闭（reason 8）；次级别买点/未触线不闭。
#[test]
fn cycle38_close_zd_touch() {
    let cfg = cfg_c38(RevCycleClose::Zd);
    let (mut fx, mut v, dr) = c38_close_fixture();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
    assert_eq!(fx.counters.n_c38_open, 1);
    fx.evs[2].clear();
    // 未触 ZD（9.0）+ 次级别买点 → 不闭
    c38_step(&mut fx, &mut v, &dr, &cfg, 1 << 1, 9.5, 1);
    assert_eq!(fx.counters.n_c38_close, 0);
    // 触线 c ≤ ZD → 闭
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.0, 2);
    assert_eq!(fx.counters.n_c38_close, 1);
    assert_eq!(fx.counters.n_c38_close_zd, 1);
    assert_eq!(fx.counters.c38_wins, 1);
}

/// C38pair：四元素闭腿集（T7 confirmed Buy3 / T6 candidate Buy3 /
/// 同锚 Buy1 / ZD 触线）各自闭腿且 reason 归因正确；纯次级别买点不闭。
#[test]
fn cycle38_close_paired_four_elements() {
    type Probe = (Vec<BspEvent>, u16, f64, fn(&Counters) -> u64, bool);
    let probes: Vec<Probe> = vec![
        (
            vec![ev(BspClass::Buy3, true)],
            0,
            9.5,
            |c| c.n_c38_close_t7,
            true,
        ),
        (
            vec![ev(BspClass::Buy3, false)],
            0,
            9.5,
            |c| c.n_c38_close_t6,
            true,
        ),
        (
            vec![ev_anchored(BspClass::Buy1, true, 111, 9.0, 9.1)],
            0,
            9.5,
            |c| c.n_c38_close_buy1,
            true,
        ),
        (vec![], 0, 9.0, |c| c.n_c38_close_zd, true),
        // 次级别任意买点单独存在 → 不闭（区间套双向性的代码落点）
        (vec![], 1 << 1, 9.5, |c| c.n_c38_close, false),
    ];
    for (close_evs, mask, price, counter_of, expect_close) in probes {
        let cfg = cfg_c38(RevCycleClose::Paired);
        let (mut fx, mut v, dr) = c38_close_fixture();
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
        assert_eq!(fx.counters.n_c38_open, 1);
        fx.evs[2] = close_evs;
        c38_step(&mut fx, &mut v, &dr, &cfg, mask, price, 1);
        if expect_close {
            assert_eq!(fx.counters.n_c38_close, 1);
            assert_eq!(counter_of(&fx.counters), 1);
        } else {
            assert_eq!(counter_of(&fx.counters), 0);
            assert_eq!(v.phase, VoicePhase::DownLeg);
        }
    }
}

/// 递归因果臂（BspAny）：本级别任意 confirmed 买点（任意锚 Buy1/Buy2/
/// Buy3）闭腿且归因正确；candidate Buy3 与次级别买点不闭；闭腿集外的
/// confirmed 买点在其他臂被反事实计数（holds）。
#[test]
fn cycle38_close_bspany_recursive_causality() {
    type Probe = (Vec<BspEvent>, fn(&Counters) -> u64, bool);
    let probes: Vec<Probe> = vec![
        // 一卖→回落→三买涌现（回补位本体）
        (vec![ev(BspClass::Buy3, true)], |c| c.n_c38_close_t7, true),
        // 一卖→回落→二买涌现（底部确认）——任意锚
        (
            vec![ev_anchored(BspClass::Buy2, true, 999, 8.0, 8.5)],
            |c| c.n_c38_close_buy2,
            true,
        ),
        // 任意锚 Buy1（同锚要求取消——reason 11 与同锚 5 区分）
        (
            vec![ev_anchored(BspClass::Buy1, true, 50, 8.0, 8.5)],
            |c| c.n_c38_close_buy1any,
            true,
        ),
        // candidate Buy3 不入集（t6 在册负槽）
        (vec![ev(BspClass::Buy3, false)], |c| c.n_c38_close, false),
    ];
    for (close_evs, counter_of, expect_close) in probes {
        let cfg = cfg_c38(RevCycleClose::BspAny);
        let (mut fx, mut v, dr) = c38_close_fixture();
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
        assert_eq!(fx.counters.n_c38_open, 1);
        fx.evs[2] = close_evs;
        c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.5, 1);
        if expect_close {
            assert_eq!(fx.counters.n_c38_close, 1);
            assert_eq!(counter_of(&fx.counters), 1);
            // 逐腿日志携带 (reason, profit)
            assert_eq!(fx.counters.c38_close_profits.len(), 1);
            assert!(fx.counters.c38_close_profits[0].1 > 0.0);
        } else {
            assert_eq!(counter_of(&fx.counters), 0);
            assert_eq!(v.phase, VoicePhase::DownLeg);
        }
    }
    // 反事实 holds：Zd 臂持腿期 confirmed Buy2 出现但不在闭腿集 → 计数
    let cfg = cfg_c38(RevCycleClose::Zd);
    let (mut fx, mut v, dr) = c38_close_fixture();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
    fx.evs[2] = vec![ev_anchored(BspClass::Buy2, true, 999, 8.0, 8.5)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.5, 1);
    assert_eq!(fx.counters.n_c38_close, 0);
    assert_eq!(fx.counters.n_c38_buy2_holds, 1);
    assert_eq!(fx.counters.n_c38_buy1_holds, 0);
}

/// BspAnyZd：买点缺席时 ZD 触线兜底闭腿。
#[test]
fn cycle38_close_bspanyzd_geometric_floor() {
    let cfg = cfg_c38(RevCycleClose::BspAnyZd);
    let (mut fx, mut v, dr) = c38_close_fixture();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
    assert_eq!(fx.counters.n_c38_open, 1);
    fx.evs[2].clear();
    // 未触线无买点 → 持腿
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.5, 1);
    assert_eq!(fx.counters.n_c38_close, 0);
    // 触 ZD（9.0）→ 兜底闭腿
    c38_step(&mut fx, &mut v, &dr, &cfg, 0, 9.0, 2);
    assert_eq!(fx.counters.n_c38_close, 1);
    assert_eq!(fx.counters.n_c38_close_zd, 1);
}

/// 锚不可定义（存活中枢被 confirmed Sell3 终结）⇒ 非 SubAny 变体开腿
/// 保守拒绝并计数；SubAny 同条件照常开（无锚依赖，在册零接触）。
#[test]
fn cycle38_nocenter_rejects_anchor_dependent_variants() {
    for (close, want_open, want_reject) in [
        (RevCycleClose::Buy1, 0u64, 1u64),
        (RevCycleClose::Zd, 0, 1),
        (RevCycleClose::Paired, 0, 1),
        (RevCycleClose::BspAnyZd, 0, 1),
        (RevCycleClose::Buy1AnyZd, 0, 1),
        // 无锚依赖臂：任意锚买点不消费锚快照 ⇒ 不捕获不拒绝
        (RevCycleClose::SubAny, 1, 0),
        (RevCycleClose::BspAny, 1, 0),
    ] {
        let cfg = cfg_c38(close);
        let (mut fx, mut v, dr) = c38_close_fixture();
        // 终结存活中枢（cs=111）→ alive(2) = None；DepthRef 参照保留
        fx.book.ingest(
            2,
            &[ev_anchored(BspClass::Sell3, true, 111, 9.0, 9.1)],
            true,
            None,
        );
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        c38_step(&mut fx, &mut v, &dr, &cfg, 0, 10.0, 0);
        assert_eq!(fx.counters.n_c38_open, want_open, "{close:?}");
        assert_eq!(fx.counters.n_c38_nocenter_rejects, want_reject, "{close:?}");
        assert_eq!(fx.counters.n_c38_cost_rejects, 0);
    }
}

#[test]
fn fractal_sub_cycle_short_diff() {
    // 38课向下段程式笔级直读：Up→Down 翻转（顶分型确认）开 Short 子腿，
    // Down→Up 翻转（底分型确认）买回；首个观测只建基准不触发。
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_sub_depth: 1,
        sub_mode: SubMode::Fractal,
        rev_paired: true,
        ..cfg_rev()
    };
    let mut sub = SubLou::new(1, RevPath::single(2).child(1));
    let anchor_row = [0i64; MAX_LADDER];
    let mut dir = [None; MAX_LADDER];
    let step = |sub: &mut SubLou,
                fx: &mut Fixture,
                dir: &[Option<Direction>; MAX_LADDER],
                anchor_row: &[i64; MAX_LADDER],
                c: f64,
                bar: i64| {
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(dir),
            run_anchor: Some(anchor_row),
            depth: None,
            l41: None,
            trend_row: None,
        };
        let (book, counters) = (&fx.book, &mut fx.counters);
        sub.step(&cfg, &rows, book, c, bar, &mut fx.ledger, 2, 50.0, counters);
    };
    // bar0：Down 基准——不开（陈旧方向不行动）
    dir[1] = Some(Direction::Down);
    step(&mut sub, &mut fx, &dir, &anchor_row, 10.0, 0);
    assert_eq!(fx.counters.n_sub_open, 0);
    // bar1：翻 Up（底分型）——无持腿，无动作
    dir[1] = Some(Direction::Up);
    step(&mut sub, &mut fx, &dir, &anchor_row, 10.5, 1);
    assert_eq!(fx.counters.n_sub_open, 0);
    // bar2：翻 Down（顶分型确认）→ 开 Short @11
    dir[1] = Some(Direction::Down);
    step(&mut sub, &mut fx, &dir, &anchor_row, 11.0, 2);
    assert_eq!(fx.counters.n_sub_open, 1);
    let key = SlotKey::rev_path(2, RevPath::single(2).child(1));
    assert!(fx.ledger.open_slot(key).is_some());
    // bar3：翻 Up（底分型确认）→ 买回 @9，盈利短差
    dir[1] = Some(Direction::Up);
    step(&mut sub, &mut fx, &dir, &anchor_row, 9.0, 3);
    assert_eq!(fx.counters.n_sub_close, 1);
    assert_eq!(fx.counters.sub_pairs, 1);
    assert_eq!(fx.counters.sub_wins, 1);
    assert!(fx.ledger.open_slot(key).is_none());
    assert!(fx.counters.sub_cash > 0.0);
    // 全程零中枢依赖：无中枢拒/振幅拒计数恒 0
    assert_eq!(fx.counters.n_sub_nocenter_rejects, 0);
    assert_eq!(fx.counters.n_sub_amp_rejects, 0);
    // P1 双门默认关：成本拒/41课拒计数恒 0（默认行为零接触）
    assert_eq!(fx.counters.n_sub_cost_rejects, 0);
    assert_eq!(fx.counters.n_sub_cost_noref_rejects, 0);
    assert_eq!(fx.counters.n_sub_l41_rejects, 0);
}

/// Sequence38 测试驱动：ladder 2 子腿（home=3），可注入 dir 行。
fn seq38_step(
    sub: &mut SubLou,
    fx: &mut Fixture,
    dir: &[Option<Direction>; MAX_LADDER],
    cfg: &OrganicConfig,
    c: f64,
    bar: i64,
) {
    let anchor_row = [0i64; MAX_LADDER];
    let rows = BarRows {
        evs: &fx.evs,
        devs: &fx.devs,
        buy_any: LadderMask(0),
        sell_any: LadderMask(0),
        dir_row: Some(dir),
        run_anchor: Some(&anchor_row),
        depth: None,
        l41: None,
        trend_row: None,
    };
    let (book, counters) = (&fx.book, &mut fx.counters);
    sub.step(cfg, &rows, book, c, bar, &mut fx.ledger, 3, 50.0, counters);
}

fn cfg_seq38() -> OrganicConfig {
    OrganicConfig {
        rev_sub_depth: 1,
        sub_mode: SubMode::Sequence38,
        rev_paired: true,
        ..cfg_rev()
    }
}

#[test]
fn sequence38_opens_on_cons_sell_closes_on_cons_buy() {
    // 38课:36：第一段盘整背驰结束点先卖；段间盘整背驰买点买回。
    let mut fx = Fixture::new();
    let cfg = cfg_seq38();
    let mut sub = SubLou::new(2, RevPath::single(3).child(2));
    let dir = [None; MAX_LADDER];
    // bar0：无事件——只累计 run_low（第一段起点低点参照）
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.0, 0);
    assert_eq!(fx.counters.n_sub_open, 0);
    // bar1：本级别盘整背驰卖点 → 开 Short @10
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 10.0, 1);
    assert_eq!(fx.counters.n_sub_open, 1);
    let key = SlotKey::rev_path(3, RevPath::single(3).child(2));
    assert!(fx.ledger.open_slot(key).is_some());
    // bar2：段间盘整背驰买点 → 买回 @9.5，盈利短差
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Down)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.5, 2);
    assert_eq!(fx.counters.n_sub_close, 1);
    assert_eq!(fx.counters.n_sub_seq_consbuy_close, 1);
    assert_eq!(fx.counters.sub_pairs, 1);
    assert_eq!(fx.counters.sub_wins, 1);
    assert!(fx.ledger.open_slot(key).is_none());
    assert!(fx.counters.sub_cash > 0.0);
    // 全程零中枢依赖：无中枢拒/振幅拒恒 0（Zhongshu 域缺口在此模式无定义）
    assert_eq!(fx.counters.n_sub_nocenter_rejects, 0);
    assert_eq!(fx.counters.n_sub_amp_rejects, 0);
}

#[test]
fn sequence38_opens_on_sublevel_cons_sell() {
    // 任务规格：开腿信号 = 本级别**或次级别**的盘整背驰卖点。
    let mut fx = Fixture::new();
    let cfg = cfg_seq38();
    let mut sub = SubLou::new(2, RevPath::single(3).child(2));
    let dir = [None; MAX_LADDER];
    fx.devs[1] = vec![dev(DivKind::Consolidation, Direction::Up)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 10.0, 0);
    assert_eq!(fx.counters.n_sub_open, 1);
}

#[test]
fn sequence38_nobreak_close_needs_sub_confirm_and_unbroken_low() {
    // 38课:36 分岔1 + 答疑:296："不跌破"靠次级别内部结构确认，非几何触线。
    let mut fx = Fixture::new();
    let cfg = cfg_seq38();
    let mut sub = SubLou::new(2, RevPath::single(3).child(2));
    let mut dir = [None; MAX_LADDER];
    // bar0：建立第一段低点参照 9.0
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.0, 0);
    // bar1：盘背卖开 @10（seg1_low 冻结 = 9.0）
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 10.0, 1);
    assert_eq!(fx.counters.n_sub_open, 1);
    fx.devs[2].clear();
    // bar2：低点未破（9.5 > 9.0）但无次级别确认 → 持腿（结构确认缺席）
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.5, 2);
    assert_eq!(fx.counters.n_sub_close, 0);
    // bar3：次级别（ladder 1）方向行翻 Up = 第二段完成的结构证据 → 买回
    dir[1] = Some(Direction::Up);
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.5, 3);
    assert_eq!(fx.counters.n_sub_close, 1);
    assert_eq!(fx.counters.n_sub_seq_nobreak_close, 1);
    assert_eq!(fx.counters.sub_wins, 1);
}

#[test]
fn sequence38_broken_low_waits_for_new_divergence() {
    // 38课:36 分岔2/观望：跌破第一段低点后，次级别确认不再是买回理由
    // ——观望直到段间盘背买或新的下跌背驰（confirmed Buy1）。
    let mut fx = Fixture::new();
    let cfg = cfg_seq38();
    let mut sub = SubLou::new(2, RevPath::single(3).child(2));
    let mut dir = [None; MAX_LADDER];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.0, 0);
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 10.0, 1);
    fx.devs[2].clear();
    // bar2：跌破第一段低点（8.5 < 9.0）
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 8.5, 2);
    assert_eq!(fx.counters.n_sub_close, 0);
    // bar3：次级别翻 Up 但低点已破 → 不跌破分岔失效，继续观望
    dir[1] = Some(Direction::Up);
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 8.8, 3);
    assert_eq!(fx.counters.n_sub_close, 0);
    // bar4：新的下跌背驰（confirmed Buy1）→ 观望出口买回
    fx.evs[2] = vec![ev(BspClass::Buy1, true)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 8.6, 4);
    assert_eq!(fx.counters.n_sub_close, 1);
    assert_eq!(fx.counters.n_sub_seq_newdiv_close, 1);
    assert_eq!(fx.counters.n_sub_seq_nobreak_close, 0);
    // 买回后中间循环复位：再来一轮盘背卖可再开
    fx.evs[2].clear();
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    seq38_step(&mut sub, &mut fx, &dir, &cfg, 9.2, 5);
    assert_eq!(fx.counters.n_sub_open, 2);
}

/// P1 成本门测试夹具：ladder 2 子腿 + n 个已观测中枢（相对振幅 rel_amp）。
/// 返回 (counters, 是否开腿)。
fn fractal_cost_step(rel_amp: f64, n_centers: usize) -> (Counters, bool) {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_sub_depth: 1,
        sub_mode: SubMode::Fractal,
        rev_paired: true,
        sub_cost_gate: true,
        ..cfg_rev()
    };
    // DepthRef 参照集：n 个不同 seg_start 的中枢，振幅 rel_amp（c=10）
    let mut dr = super::super::depth_ref::DepthRef::new(50);
    for i in 0..n_centers {
        fx.book.ingest(
            2,
            &[ev_anchored(
                BspClass::Sell1,
                true,
                100 + i as i64,
                9.0,
                9.0 + rel_amp * 10.0,
            )],
            true,
            None,
        );
        dr.observe(&fx.book, 10.0);
    }
    let mut sub = SubLou::new(2, RevPath::single(3).child(2));
    let anchor_row = [0i64; MAX_LADDER];
    let mut dir = [None; MAX_LADDER];
    let mut counters = Counters::default();
    let mut step = |dir: &[Option<Direction>; MAX_LADDER], c: f64, bar: i64| {
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(dir),
            run_anchor: Some(&anchor_row),
            depth: Some(&dr),
            l41: None,
            trend_row: None,
        };
        sub.step(
            &cfg,
            &rows,
            &fx.book,
            c,
            bar,
            &mut fx.ledger,
            3,
            50.0,
            &mut counters,
        );
    };
    // Up 基准 → Down 翻转（顶分型确认 = 开腿尝试）
    dir[2] = Some(Direction::Up);
    step(&dir, 10.0, 0);
    dir[2] = Some(Direction::Down);
    step(&dir, 10.0, 1);
    drop(step);
    let opened = counters.n_sub_open == 1;
    (counters, opened)
}

#[test]
fn cost_gate_closes_thin_level_and_passes_thick() {
    // 35课：典型振幅 0.05% < 2×10bps=0.2% ⇒ 级别关闭
    let (c, opened) = fractal_cost_step(0.0005, 12);
    assert!(!opened);
    assert_eq!(c.n_sub_cost_rejects, 1);
    assert_eq!(c.n_sub_cost_noref_rejects, 0);
    // 典型振幅 1% ≥ 0.2% ⇒ 正常开腿
    let (c, opened) = fractal_cost_step(0.01, 12);
    assert!(opened);
    assert_eq!(c.n_sub_cost_rejects, 0);
}

#[test]
fn cost_gate_rejects_when_reference_undefined() {
    // warm-up：样本 5 < SUB_COST_MIN_OBS=10 ⇒ 参照不可定义，保守拒绝
    let (c, opened) = fractal_cost_step(0.01, 5);
    assert!(!opened);
    assert_eq!(c.n_sub_cost_noref_rejects, 1);
    assert_eq!(c.n_sub_cost_rejects, 0);
}

#[test]
fn l41_gate_rejects_while_parent_trend_unexhausted() {
    // 41课：父级别（ladder 2）相邻 Down 段创新低且无盘整背驰 ⇒ 子腿拒开；
    // 盘整背驰出现（衰竭证据）⇒ 门开，子腿正常开。
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_sub_depth: 1,
        sub_mode: SubMode::Fractal,
        rev_paired: true,
        sub_l41_gate: true,
        ..cfg_rev()
    };
    let mut te = super::super::trend_exhaustion::TrendExhaustion::new();
    let devs_empty: [Vec<DivEvent>; MAX_LADDER] = Default::default();
    // 父级别 ladder 2 走出两个创新低 Down 段（趋势未完）
    let mut pdir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    pdir[2] = Some(Direction::Down);
    te.observe(&pdir, &devs_empty, 9.0);
    pdir[2] = Some(Direction::Up);
    te.observe(&pdir, &devs_empty, 9.5);
    pdir[2] = Some(Direction::Down);
    te.observe(&pdir, &devs_empty, 8.0);
    assert!(te.down_unexhausted(2));

    let mut sub = SubLou::new(1, RevPath::single(2).child(1));
    let anchor_row = [0i64; MAX_LADDER];
    let mut dir = [None; MAX_LADDER];
    {
        let mut step = |dir: &[Option<Direction>; MAX_LADDER], c: f64, bar: i64| {
            let rows = BarRows {
                evs: &fx.evs,
                devs: &fx.devs,
                buy_any: LadderMask(0),
                sell_any: LadderMask(0),
                dir_row: Some(dir),
                run_anchor: Some(&anchor_row),
                depth: None,
                l41: Some(&te),
                trend_row: None,
            };
            sub.step(
                &cfg,
                &rows,
                &fx.book,
                c,
                bar,
                &mut fx.ledger,
                2,
                50.0,
                &mut fx.counters,
            );
        };
        // 子级别 Up 基准 → Down 翻转：被 41课门拒
        dir[1] = Some(Direction::Up);
        step(&dir, 10.0, 0);
        dir[1] = Some(Direction::Down);
        step(&dir, 10.0, 1);
    }
    assert_eq!(fx.counters.n_sub_open, 0);
    assert_eq!(fx.counters.n_sub_l41_rejects, 1);
    // 父级别盘整背驰出现（衰竭证据）→ 门开
    let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
    devs[2] = vec![DivEvent {
        kind: DivKind::Consolidation,
        direction: Direction::Down,
        seg_idx: 0,
        force_a: 0.0,
        force_c: 0.0,
        price: 0.0,
    }];
    te.observe(&pdir, &devs, 7.9);
    assert!(!te.down_unexhausted(2));
    {
        let mut step = |dir: &[Option<Direction>; MAX_LADDER], c: f64, bar: i64| {
            let rows = BarRows {
                evs: &fx.evs,
                devs: &fx.devs,
                buy_any: LadderMask(0),
                sell_any: LadderMask(0),
                dir_row: Some(dir),
                run_anchor: Some(&anchor_row),
                depth: None,
                l41: Some(&te),
                trend_row: None,
            };
            sub.step(
                &cfg,
                &rows,
                &fx.book,
                c,
                bar,
                &mut fx.ledger,
                2,
                50.0,
                &mut fx.counters,
            );
        };
        // 重新走一次翻转（Up 再 Down）→ 正常开腿
        dir[1] = Some(Direction::Up);
        step(&dir, 10.5, 2);
        dir[1] = Some(Direction::Down);
        step(&dir, 10.2, 3);
    }
    assert_eq!(fx.counters.n_sub_open, 1);
    assert_eq!(fx.counters.n_sub_l41_rejects, 1);
}

#[test]
fn t7_closes_all_tranches_and_returns_upleg() {
    let mut fx = Fixture::new();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_rev(),
            &rows,
            &[],
            10.0,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // confirmed Buy3 → T7
    fx.evs[2] = vec![ev(BspClass::Buy3, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_rev(),
        &rows,
        &[],
        9.0,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert!(v.rev.is_none());
    assert_eq!(fx.counters.n_rev_close_t7, 1);
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_none());
}

#[test]
fn t5_nesting_closes_prefix_only() {
    // 双 tranche（level 2 + 3），level 2 买点 → 只回补 level 2，高层存续
    let mut fx = Fixture::new();
    let mut v = VoiceUnit::new(2);
    v.phase = VoicePhase::DownLeg;
    let mut leg = RevLeg::new(2, 0, None);
    leg.tranches.push(RevTranche {
        level: 3,
        last_dir: None,
    });
    v.rev = Some(leg);
    assert!(fx
        .ledger
        .open_diff(SlotKey::rev(2, 2), 0.3, 10.0, 0, LegAnchor::SegmentScale));
    assert!(fx
        .ledger
        .open_diff(SlotKey::rev(2, 3), 0.3, 10.0, 0, LegAnchor::SegmentScale));
    // level 2 confirmed Buy1
    fx.evs[2] = vec![ev(BspClass::Buy1, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_rev(),
        &rows,
        &[],
        9.0,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.3,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::DownLeg); // 高层 tranche 存续
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_none());
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_some());
    assert_eq!(fx.counters.n_rev_tranche_closes, 1);
    // level 3 confirmed Buy1 同时出现 level 2 买点 → max 读数全回补
    fx.evs[2] = vec![ev(BspClass::Buy1, true)];
    fx.evs[3] = vec![ev(BspClass::Buy1, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_rev(),
        &rows,
        &[],
        8.5,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.3,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_none());
}

#[test]
fn g1_direction_anchor_rejects_when_sub_not_down() {
    let mut fx = Fixture::new();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let cfg = OrganicConfig {
        rev_mode: true,
        sub_anchor: SubAnchor::Direction,
        ..OrganicConfig::default()
    };
    let mut v = VoiceUnit::new(2);
    let dir_row: [Option<Direction>; MAX_LADDER] = [Some(Direction::Up); MAX_LADDER];
    let anchors = [0i64; MAX_LADDER];
    let rows = BarRows {
        evs: &fx.evs,
        devs: &fx.devs,
        buy_any: LadderMask(0),
        sell_any: LadderMask(0),
        dir_row: Some(&dir_row),
        run_anchor: Some(&anchors),
        depth: None,
        l41: None,
        trend_row: None,
    };
    v.step(
        &cfg,
        &rows,
        &[],
        10.0,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_sub_anchor_rejects, 1);
    assert_eq!(fx.counters.n_rev_open, 0);
}

#[test]
fn t4b_adds_tranche_on_center_formed_in_rev_run() {
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_mode: true,
        tranche: true,
        ..OrganicConfig::default()
    };
    let mut v = VoiceUnit::new(2);
    v.phase = VoicePhase::DownLeg;
    v.rev = Some(RevLeg::new(2, 0, Some(Direction::Down)));
    assert!(fx
        .ledger
        .open_diff(SlotKey::rev(2, 2), 0.3, 10.0, 0, LegAnchor::SegmentScale));
    let mut dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    dir_row[2] = Some(Direction::Down);
    let anchors = [0i64; MAX_LADDER];
    let rows = BarRows {
        evs: &fx.evs,
        devs: &fx.devs,
        buy_any: LadderMask(0),
        sell_any: LadderMask(0),
        dir_row: Some(&dir_row),
        run_anchor: Some(&anchors),
        depth: None,
        l41: None,
        trend_row: None,
    };
    let formed = CenterEvent::Formed {
        seg_start: 9,
        zd: 8.0,
        zg: 9.0,
    };
    v.step(
        &cfg,
        &rows,
        &[formed],
        9.0,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.2,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_rev_tranche_adds, 1);
    assert_eq!(v.rev.as_ref().unwrap().max_level(), 3);
    assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_some());
    // 第二个 Formed（未见 Terminated{Down}）不再加码（(a) 一次性）
    v.step(
        &cfg,
        &rows,
        &[formed],
        9.0,
        4,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.2,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_rev_tranche_adds, 1);
}

fn cfg_paired(theta: f64) -> OrganicConfig {
    OrganicConfig {
        rev_mode: true,
        rev_paired: true,
        theta_depth: theta,
        ..OrganicConfig::default()
    }
}

#[test]
fn paired_osc_open_anchors_alive_center_and_closes_at_zd() {
    // 震荡型：confirmed Sell1 × 存活中枢 [9.0, 9.5] → 开腿锚 ZD=9.0；
    // 触线 c≤9.0 → 闭腿（n_rev_zd_close），按类型记账。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_open_osc, 1);
    let leg = v.rev.as_ref().unwrap();
    assert_eq!(leg.open_kind, Some(RevOpenKind::Oscillation));
    assert_eq!(leg.anchor_cs, Some(1));
    assert_eq!(leg.zd, Some(9.0));
    // 无事件 bar：c=9.0 触线 → 闭腿
    fx.evs[2].clear();
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.0),
        &rows,
        &[],
        9.0,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_zd_close, 1);
    assert_eq!(fx.counters.rev_osc_pairs, 1);
    assert_eq!(fx.counters.rev_osc_wins, 1); // 卖9.6买9.0
    assert!(fx.counters.rev_osc_cash > 0.0);
}

#[test]
fn seq_nobreak_closes_on_held_low_with_sub_confirm() {
    // Seq38n（rev_seq_nobreak）：bar1 低点 9.0（第一段起点）→ bar2 9.6
    // Sell1 开腿（seg1_low 冻结 9.0）→ bar3 9.3 回拉不破 ∧ 无次级别
    // 确认 → 持有；bar4 9.3 次级别买证据（buy_any[k−1]）共现 → reason
    // 11 闭腿。中枢 [8.5, 9.5]：c=9.3 不触 ZD，分支隔离。
    let mut fx = Fixture::new();
    let cfg = OrganicConfig {
        rev_seq_nobreak: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        // bar1：第一段起点低点（UpLeg 空转，仅推进 rev_run_low）
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.0,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 8.5, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(v.rev.as_ref().unwrap().seg1_low, Some(9.0));
    fx.evs[2].clear();
    {
        // bar3：不破第一段低点但次级别确认缺失 → 持有
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.3,
            3,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_seq_nobreak_close, 0);
    // bar4：次级别买侧事件证据（sub_confirm 的 buy_any 分量）共现 → 闭腿
    let rows = BarRows {
        buy_any: LadderMask(1 << 1),
        ..empty_rows(&fx.evs, &fx.devs)
    };
    v.step(
        &cfg,
        &rows,
        &[],
        9.3,
        4,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_seq_nobreak_close, 1);
    assert_eq!(fx.counters.rev_close_log.last().unwrap().reason, 11);
    // 中间循环复位：rev_run_low 从闭腿 close 重新累计
    assert_eq!(v.rev_run_low, 9.3);
}

#[test]
fn seq_nobreak_holds_when_seg1_low_broken_or_gate_off() {
    // 破第一段低点（low_since_open ≤ seg1_low）→ 即使次级别确认共现也
    // 不闭（分岔1 前提不成立——破位后的出口是盘背买/新下跌背驰，主腿
    // 形式即 T5/ZD 在册路径）；同场景 rev_seq_nobreak=false → 同样不闭
    // （O0 零接触：默认位下 seg1_low 纯状态跟踪无消费者）。
    for gate_on in [true, false] {
        let mut fx = Fixture::new();
        let cfg = OrganicConfig {
            rev_seq_nobreak: gate_on,
            ..cfg_paired(0.0)
        };
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg,
                &rows,
                &[],
                9.0,
                1,
                &mut fx.ledger,
                &fx.book,
                &fx.gate,
                4,
                &|_| 0.5,
                &mut fx.counters,
            );
        }
        fx.book.ingest(
            2,
            &[ev_anchored(BspClass::Sell1, true, 1, 8.5, 9.5)],
            true,
            None,
        );
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg,
                &rows,
                &[],
                9.6,
                2,
                &mut fx.ledger,
                &fx.book,
                &fx.gate,
                4,
                &|_| 0.5,
                &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        fx.evs[2].clear();
        {
            // bar3：c=8.8 破第一段低点 9.0（不触 ZD=8.5）
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg,
                &rows,
                &[],
                8.8,
                3,
                &mut fx.ledger,
                &fx.book,
                &fx.gate,
                4,
                &|_| 0.5,
                &mut fx.counters,
            );
        }
        // bar4：次级别买证据共现但低点已破 → 持有
        let rows = BarRows {
            buy_any: LadderMask(1 << 1),
            ..empty_rows(&fx.evs, &fx.devs)
        };
        v.step(
            &cfg,
            &rows,
            &[],
            9.2,
            4,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::DownLeg, "gate_on={gate_on}");
        assert_eq!(fx.counters.n_rev_seq_nobreak_close, 0, "gate_on={gate_on}");
    }
}

#[test]
fn paired_rejects_mismatched_buys_accepts_same_anchor_buy1() {
    // 闭腿配对：Buy2 / 盘背买 / 异锚 Buy1 不闭（mismatch_holds 计数），
    // 同锚 confirmed Buy1 闭（T5）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // Buy2（kind 不匹配）+ 异锚 Buy1（cs=99 ≠ 锚 1）→ 不闭
    fx.evs[2] = vec![
        ev(BspClass::Buy2, true),
        ev_anchored(BspClass::Buy1, true, 99, 8.0, 8.5),
    ];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            9.3,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_mismatch_holds, 1);
    // 同锚 confirmed Buy1（cs=1）→ T5 闭腿
    fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 1, 9.0, 9.5)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.0),
        &rows,
        &[],
        9.2,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_close_t5, 1);
    assert_eq!(fx.counters.rev_osc_pairs, 1);
}

#[test]
fn t2o_sell2_opens_oscillation_leg_when_enabled() {
    // T2o：confirmed Sell2 × 存活中枢 → 震荡型开腿（trigger 掩码 bit3）；
    // 开关关时同一事件不开腿（默认行为零接触）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell2, true)];
    let mut v = VoiceUnit::new(2);
    {
        // 开关关：Sell2 不是触发源
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_open, 0);
    // 开关开：同一事件开震荡型腿
    let cfg = OrganicConfig {
        sell2_open: true,
        ..cfg_paired(0.0)
    };
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.6,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_open_osc, 1);
    assert_eq!(fx.counters.n_rev_sell2_open, 1);
    let log = fx.counters.rev_open_log.last().unwrap();
    assert_eq!(log.trigger, 8); // bit3 独占（无 Sell1/盘背共现）
    assert_eq!(
        v.rev.as_ref().unwrap().open_kind,
        Some(RevOpenKind::Oscillation)
    );
}

#[test]
fn t2c_buy2_same_anchor_closes_when_enabled_rejects_foreign_anchor() {
    // T2c：异锚 Buy2 不闭（mismatch hold），同锚 confirmed Buy2 闭
    // （reason=10，n_rev_buy2_close）。
    let cfg = OrganicConfig {
        buy2_close: true,
        ..cfg_paired(0.0)
    };
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // 异锚 Buy2（cs=99 ≠ 锚 1）→ 不闭
    fx.evs[2] = vec![ev_anchored(BspClass::Buy2, true, 99, 8.0, 8.5)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.3,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_mismatch_holds, 1);
    // 同锚 confirmed Buy2（cs=1）→ T2c 闭腿
    fx.evs[2] = vec![ev_anchored(BspClass::Buy2, true, 1, 9.0, 9.5)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.2,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_buy2_close, 1);
    assert_eq!(fx.counters.n_rev_close_t5, 0); // 与 Buy1 通道独立计数
    assert_eq!(fx.counters.rev_osc_pairs, 1);
    assert_eq!(fx.counters.rev_close_log.last().unwrap().reason, 10);
}

#[test]
fn paired_escape_opens_on_sell3_no_zd_line_closes_on_any_buy1() {
    // 逃逸型：confirmed Sell3（中枢死亡）→ 开腿无 ZD 线；
    // c 低于死中枢 ZD 不触发闭腿；任意锚 confirmed Buy1 闭（趋势配对）。
    let mut fx = Fixture::new();
    // 中枢 [9.0,9.5] 先存活，confirmed Sell3 将其向下终结
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    assert!(fx.book.alive(2).is_none());
    fx.evs[2] = vec![ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            8.8,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_open_esc, 1);
    let leg = v.rev.as_ref().unwrap();
    assert_eq!(leg.open_kind, Some(RevOpenKind::Escape));
    assert_eq!(leg.zd, None);
    // 无事件 + 价格更低 → 无 ZD 线，不闭
    fx.evs[2].clear();
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            8.0,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // 异锚 confirmed Buy1（新中枢 cs=7）→ 趋势配对闭腿
    fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 7, 7.5, 7.9)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.0),
        &rows,
        &[],
        7.8,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_close_t5, 1);
    assert_eq!(fx.counters.rev_esc_pairs, 1);
    assert_eq!(fx.counters.rev_esc_wins, 1); // 卖8.8买7.8
}

#[test]
fn paired_depth_gate_rejects_shallow_center() {
    // 深度门：中枢 [9.0, 9.005] 振幅/价 ≈0.05% < θ=1% → 拒，计数。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.005)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.01),
        &rows,
        &[],
        9.1,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_depth_rejects, 1);
    assert_eq!(fx.counters.n_rev_open, 0);
}

#[test]
fn paired_osc_requires_alive_center() {
    // 震荡型无存活中枢（孤证 confirmed Sell1）→ nocenter 拒。
    let mut fx = Fixture::new();
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.0),
        &rows,
        &[],
        9.6,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_nocenter_rejects, 1);
    assert_eq!(fx.counters.n_rev_open, 0);
}

#[test]
fn paired_escape_off_does_not_open_on_sell3() {
    // V2o 消融：rev_escape_open=false 时 confirmed Sell3 不开腿，
    // 也不降级为震荡型（中枢已死 ⇒ alive 检查拒）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)];
    let cfg = OrganicConfig {
        rev_escape_open: false,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        8.8,
        1,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_open, 0);
    assert_eq!(fx.counters.n_rev_attempts, 0); // 信号谓词同步排除 Sell3
}

#[test]
fn paired_buy3_escape_close_still_works() {
    // Buy3 回补位闭腿（恢复暴露）在配对模式保留（T7）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    fx.evs[2] = vec![ev(BspClass::Buy3, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_paired(0.0),
        &rows,
        &[],
        9.8,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_close_t7, 1);
    assert_eq!(fx.counters.rev_osc_pairs, 1);
    assert_eq!(fx.counters.rev_osc_wins, 0); // 卖9.6买9.8 亏损回补
}

fn dev(kind: DivKind, direction: Direction) -> DivEvent {
    DivEvent {
        kind,
        direction,
        seg_idx: 0,
        force_a: 1.0,
        force_c: 0.5,
        price: 10.0,
    }
}

#[test]
fn r1_gates_consol_open_on_sub_sell_in_c_window() {
    // R1：盘背触发无次级别 Sell 证据 → 拒；同 bar 次级别卖侧背驰 → 开。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        r1_sub_sell_open: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    let mut dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    dir_row[1] = Some(Direction::Up); // 次级别 Up run（C 段窗口）
    let anchors = [0i64; MAX_LADDER];
    // bar 1：盘背触发，无次级别证据 → R1 拒
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    {
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(&dir_row),
            run_anchor: Some(&anchors),
            depth: None,
            l41: None,
            trend_row: None,
        };
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_r1_rejects, 1);
    assert_eq!(fx.counters.n_rev_open, 0);
    // bar 2：次级别卖侧背驰（窗口内，bar 2 ≥ 锚 0）+ 盘背触发 → 开腿
    fx.devs[1] = vec![dev(DivKind::Trend, Direction::Up)];
    let rows = BarRows {
        evs: &fx.evs,
        devs: &fx.devs,
        buy_any: LadderMask(0),
        sell_any: LadderMask(0),
        dir_row: Some(&dir_row),
        run_anchor: Some(&anchors),
        depth: None,
        l41: None,
        trend_row: None,
    };
    v.step(
        &cfg,
        &rows,
        &[],
        9.6,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_open_osc, 1);
    assert_eq!(fx.counters.rev_open_log.last().unwrap().trigger, 2); // bit1 盘背
}

#[test]
fn r2_realizes_at_zg_extends_to_zd_when_sub_pullback_growing() {
    // R2：盘背触发腿触 ZG，次级别回拉未完成 ∧ 延伸档门过 → 持有；
    // 触 ZD → reason 8。（R2 作用域 = trigger==2，Sell1 腿不消费。）
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    let cfg = OrganicConfig {
        r2_anchor_zg: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            10.0,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    let leg = v.rev.as_ref().unwrap();
    assert_eq!(leg.zg, Some(9.5));
    assert!(leg.ext_allowed);
    assert_eq!(leg.open_trigger, 2);
    // bar 2：c=9.4 ≤ ZG，无次级别买证据 → 延伸持有
    fx.devs[2].clear();
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.4,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_zg_close, 0);
    // bar 3：c=9.0 触 ZD → 延伸目标兑现（reason 8）
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.0,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_zd_close, 1);
    assert_eq!(fx.counters.rev_close_log.last().unwrap().reason, 8);
}

#[test]
fn r2_realizes_at_zg_when_sub_pullback_done() {
    // R2：盘背触发腿触 ZG ∧ 次级别买侧证据已出现（腿生命期内）→ reason 9。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    let cfg = OrganicConfig {
        r2_anchor_zg: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            10.0,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    // bar 2：次级别（ladder 1）买点出现（buy_any）——回拉完成证据
    fx.devs[2].clear();
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.buy_any = LadderMask(1 << 1);
        v.step(
            &cfg,
            &rows,
            &[],
            9.7,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg); // 未触 ZG，不兑现
                                              // bar 3：c=9.45 ≤ ZG ∧ 回拉完成 → reason 9
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.45,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_zg_close, 1);
    assert_eq!(fx.counters.rev_close_log.last().unwrap().reason, 9);
    assert_eq!(fx.counters.rev_osc_pairs, 1);
    assert_eq!(fx.counters.rev_osc_wins, 1); // 卖10.0买9.45
}

#[test]
fn r2_does_not_touch_sell1_triggered_legs() {
    // 任务硬约束：type1 卖（Sell1 触发）REV 腿逻辑不可被改动——
    // R2 开启时 Sell1 腿兑现锚仍是 ZD（c ≤ ZG 不兑现）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    let cfg = OrganicConfig {
        r2_anchor_zg: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.rev.as_ref().unwrap().open_trigger, 1);
    // c=9.3 ≤ ZG：Sell1 腿不在 ZG 兑现
    fx.evs[2].clear();
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.3,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_zg_close, 0);
    // c=9.0 触 ZD → 原锚兑现
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.0,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_zd_close, 1);
}

#[test]
fn r3_suppresses_t6_until_sub_pullback_evidence() {
    // R3：盘背触发腿 candidate Buy3 无次级别证据 → t6 抑制；
    // 证据齐 ∧ 低点>ZG → t6 触发。（作用域 = trigger==2。）
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    let cfg = OrganicConfig {
        r3_t6_sub_confirm: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // bar 2：candidate Buy3，无次级别买证据 → 抑制
    fx.devs[2].clear();
    fx.evs[2] = vec![ev(BspClass::Buy3, false)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.7,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_r3_holds, 1);
    assert_eq!(fx.counters.n_rev_close_t6, 0);
    // bar 3：candidate Buy3 + 次级别买证据，低点 9.6 > ZG 9.5 → t6 触发
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.buy_any = LadderMask(1 << 1);
        v.step(
            &cfg,
            &rows,
            &[],
            9.8,
            3,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert_eq!(fx.counters.n_rev_close_t6, 1);
}

#[test]
fn r3_holds_t6_when_pullback_low_reentered_center() {
    // R3：盘背触发腿回拉低点 ≤ ZG（已重回中枢）→ 即使次级别证据在，t6 仍抑制。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.devs[2] = vec![dev(DivKind::Consolidation, Direction::Up)];
    let cfg = OrganicConfig {
        r3_t6_sub_confirm: true,
        ..cfg_paired(0.0)
    };
    let mut v = VoiceUnit::new(2);
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    // bar 2：c=9.3 ≤ ZG（低点重回中枢；无 R2 ⇒ 不在此兑现，ZD 未触）
    fx.devs[2].clear();
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg,
            &rows,
            &[],
            9.3,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    // bar 3：candidate Buy3 + 次级别证据，但 low=9.3 ≤ 9.5 → 抑制
    fx.evs[2] = vec![ev(BspClass::Buy3, false)];
    let mut rows = empty_rows(&fx.evs, &fx.devs);
    rows.buy_any = LadderMask(1 << 1);
    v.step(
        &cfg,
        &rows,
        &[],
        9.8,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert_eq!(fx.counters.n_rev_r3_holds, 1);
    assert_eq!(fx.counters.n_rev_close_t6, 0);
}

fn cfg_sub1() -> OrganicConfig {
    // V2of 语义（震荡型独占 + rev_paired）+ 深度1 递归子 LOU
    OrganicConfig {
        rev_sub_depth: 1,
        rev_escape_open: false,
        ..cfg_paired(0.0)
    }
}

/// 父 REV 开在 ladder 3 → 子反弹腿（Long）开闭于 ladder 2 的存活中枢域。
fn open_parent_and_sub(fx: &mut Fixture, v: &mut VoiceUnit) {
    // bar1：ladder 3 confirmed Sell1 × 存活中枢 [9.0,9.5] → 父 REV 开 @9.6
    fx.book.ingest(
        3,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[3] = vec![ev(BspClass::Sell1, true)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_sub1(),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            5,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert!(fx.ledger.open_slot(SlotKey::rev(3, 3)).is_some());
    // bar2：ladder 2 存活中枢 [9.1,9.4] + confirmed Buy1 → 子腿先买 @9.2
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 11, 9.1, 9.4)],
        true,
        None,
    );
    fx.evs[3].clear();
    fx.evs[2] = vec![ev(BspClass::Buy1, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_sub1(),
        &rows,
        &[],
        9.2,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_sub_open, 1);
    let sub_key = SlotKey::rev_path(3, RevPath::single(3).child(2));
    let leg = fx.ledger.open_slot(sub_key).expect("子腿槽开放");
    assert_eq!(leg.cycle.shares, 50.0); // 预算基 = 父腿敞口（100×0.5）
                                        // 深度预算：depth1 配置下不再生成 depth2 子节点
    assert!(v
        .rev
        .as_ref()
        .unwrap()
        .sub
        .as_ref()
        .unwrap()
        .child
        .is_none());
}

#[test]
fn sub_rebound_leg_opens_and_closes_on_same_anchor_sell1() {
    let mut fx = Fixture::new();
    let mut v = VoiceUnit::new(3);
    open_parent_and_sub(&mut fx, &mut v);
    // bar3：ladder 2 同锚 confirmed Sell1（cs=11）→ 子腿卖出 @9.35（反弹顶）
    fx.evs[2] = vec![ev_anchored(BspClass::Sell1, true, 11, 9.1, 9.4)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_sub1(),
        &rows,
        &[],
        9.35,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_sub_close, 1);
    assert_eq!(fx.counters.sub_pairs, 1);
    assert_eq!(fx.counters.sub_wins, 1); // 买 9.2 卖 9.35
    assert!(fx.counters.sub_cash > 0.0);
    let sub_key = SlotKey::rev_path(3, RevPath::single(3).child(2));
    assert!(fx.ledger.open_slot(sub_key).is_none());
    // 父腿不受影响（级别隔离：子腿闭合不是父腿闭合证据）
    assert_eq!(v.phase, VoicePhase::DownLeg);
    assert!(fx.ledger.open_slot(SlotKey::rev(3, 3)).is_some());
    // 注意：ladder 2 的 Sell1 会被父层（ladder 3 谓词）忽略——开腿循环可重启
}

#[test]
fn parent_close_cascades_open_sub_leg() {
    let mut fx = Fixture::new();
    let mut v = VoiceUnit::new(3);
    open_parent_and_sub(&mut fx, &mut v);
    // bar3：c=9.0 触父锚 ZD → 父腿闭合，子腿级联强闭（同价）
    fx.evs[2].clear();
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_sub1(),
        &rows,
        &[],
        9.0,
        3,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(v.phase, VoicePhase::UpLeg);
    assert!(v.rev.is_none());
    assert_eq!(fx.counters.n_sub_forced_close, 1);
    assert_eq!(fx.counters.sub_pairs, 1);
    assert_eq!(fx.counters.sub_wins, 0); // 买 9.2 强闭卖 9.0 = 亏损
    assert!(fx.counters.sub_cash < 0.0);
    assert!(fx.ledger.open_slot(SlotKey::rev(3, 3)).is_none());
    assert!(fx
        .ledger
        .open_slot(SlotKey::rev_path(3, RevPath::single(3).child(2)))
        .is_none());
}

#[test]
fn sub_amp_gate_rejects_thin_center() {
    // 经济终止条件：子中枢振幅 (9.205−9.195)/9.2 ≈ 0.011% < 2×0.1% → 拒
    let mut fx = Fixture::new();
    let mut v = VoiceUnit::new(3);
    fx.book.ingest(
        3,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[3] = vec![ev(BspClass::Sell1, true)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_sub1(),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            5,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 11, 9.195, 9.205)],
        true,
        None,
    );
    fx.evs[3].clear();
    fx.evs[2] = vec![ev(BspClass::Buy1, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_sub1(),
        &rows,
        &[],
        9.2,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_sub_amp_rejects, 1);
    assert_eq!(fx.counters.n_sub_open, 0);
}

#[test]
fn sub_not_instantiated_below_first_bsp_ladder() {
    // 存在论终止：voice k=2 的子级别 1 是 bi 级（无中枢概念）→ 节点不实例化
    let mut fx = Fixture::new();
    let mut v = VoiceUnit::new(2);
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.evs[2] = vec![ev(BspClass::Sell1, true)];
    {
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_sub1(),
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            5,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(v.phase, VoicePhase::DownLeg);
    fx.evs[2].clear();
    fx.evs[1] = vec![ev(BspClass::Buy1, true)];
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg_sub1(),
        &rows,
        &[],
        9.2,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        5,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert!(v.rev.as_ref().unwrap().sub.is_none());
    assert_eq!(fx.counters.n_sub_open, 0);
}

#[test]
fn osc_subloop_p5_open_close() {
    // P5 域腿逐字：c≥ZG ∧ sub_sell 开；c≤锚ZD 关
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig::default(); // O0：rev 关
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1); // sub_sell = sell_any[k-1]
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
    // 触 ZD 回补
    let rows = empty_rows(&fx.evs, &fx.devs);
    v.step(
        &cfg,
        &rows,
        &[],
        9.0,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_osc_zd_close, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
}

/// H4 滚动振幅准入（osc_amp_gate）：参照不可定义保守拒 → 振幅不足拒
/// → 振幅达标放行；闭腿路径零接触（只挡开腿）。
#[test]
fn osc_amp_gate_noref_thin_then_pass_close_untouched() {
    let cfg = OrganicConfig {
        osc_amp_gate: true,
        ..OrganicConfig::default()
    };
    let open_ev = ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5);

    // 阶段1：DepthRef 空参照（warm-up）⇒ noref 保守拒
    let mut fx = Fixture::new();
    fx.book.ingest(2, &[open_ev.clone()], true, None);
    let dr = DepthRef::new(50);
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.depth = Some(&dr);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 0);
    assert_eq!(fx.counters.n_osc_amp_noref_rejects, 1, "warm-up 期保守拒绝");
    assert_eq!(fx.counters.osc_amp_reject_log, vec![(2, 1, 0)]);

    // 阶段2：10 个 0.01% 振幅中枢喂参照 ⇒ θ_q < 2×10bps ⇒ 振幅不足拒
    let feed = |amps: &[(i64, f64, f64)]| {
        let mut dr = DepthRef::new(50);
        let mut feed_book = CenterBook::new();
        for &(cs, zd, zg) in amps {
            feed_book.ingest(
                2,
                &[ev_anchored(BspClass::Sell1, true, cs, zd, zg)],
                true,
                None,
            );
            dr.observe(&feed_book, 100.0);
        }
        dr
    };
    let thin: Vec<(i64, f64, f64)> = (0..10).map(|j| (10 + j, 50.0, 50.01)).collect();
    let dr_thin = feed(&thin);
    let mut fx2 = Fixture::new();
    fx2.book.ingest(2, &[open_ev.clone()], true, None);
    let mut v2 = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx2.evs, &fx2.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.depth = Some(&dr_thin);
        v2.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx2.ledger,
            &fx2.book,
            &fx2.gate,
            4,
            &|_| 0.5,
            &mut fx2.counters,
        );
    }
    assert_eq!(fx2.counters.n_osc_open, 0);
    assert_eq!(fx2.counters.n_osc_amp_rejects, 1, "θ_q=0.01% < 0.2% 不准入");
    assert_eq!(fx2.counters.osc_amp_reject_log, vec![(2, 1, 1)]);

    // 阶段3：10 个 1% 振幅中枢 ⇒ θ_q=1% ≥ 0.2% ⇒ 准入开腿；
    // 闭腿（ZD 触线）不过门——只挡开腿
    let wide: Vec<(i64, f64, f64)> = (0..10).map(|j| (10 + j, 50.0, 51.0)).collect();
    let dr_wide = feed(&wide);
    let mut fx3 = Fixture::new();
    fx3.book.ingest(2, &[open_ev], true, None);
    let mut v3 = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx3.evs, &fx3.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.depth = Some(&dr_wide);
        v3.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx3.ledger,
            &fx3.book,
            &fx3.gate,
            4,
            &|_| 0.5,
            &mut fx3.counters,
        );
    }
    assert_eq!(fx3.counters.n_osc_open, 1, "振幅达标 → 准入");
    {
        let mut rows = empty_rows(&fx3.evs, &fx3.devs);
        // 闭腿 bar 不喂 depth——闭腿路径不读门（读了会 panic capability expect）
        rows.depth = None;
        v3.step(
            &cfg,
            &rows,
            &[],
            9.0,
            2,
            &mut fx3.ledger,
            &fx3.book,
            &fx3.gate,
            4,
            &|_| 0.5,
            &mut fx3.counters,
        );
    }
    assert_eq!(fx3.counters.n_osc_zd_close, 1, "闭腿零接触");
    assert!(fx3.ledger.open_slot(SlotKey::osc(2)).is_none());
}

#[test]
fn osc_l41_gate_rejects_while_parent_up_trend_unexhausted() {
    // 41课域腿门：父级别（k+1=3）相邻 Up 段创新高且无盘整背驰 ⇒ osc 拒开；
    // 盘整背驰出现（衰竭证据）⇒ 门开，osc 正常开腿。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_l41_gate: true,
        ..OrganicConfig::default()
    };
    let mut te = super::super::trend_exhaustion::TrendExhaustion::new();
    let devs_empty: [Vec<DivEvent>; MAX_LADDER] = Default::default();
    // 父级别 ladder 3 走出两个创新高 Up 段（上涨趋势未完）
    let mut pdir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    pdir[3] = Some(Direction::Up);
    te.observe(&pdir, &devs_empty, 10.0);
    pdir[3] = Some(Direction::Down);
    te.observe(&pdir, &devs_empty, 9.5);
    pdir[3] = Some(Direction::Up);
    te.observe(&pdir, &devs_empty, 11.0);
    assert!(te.up_unexhausted(3));

    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1); // sub_sell = sell_any[k-1]
        rows.l41 = Some(&te);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 0);
    assert_eq!(fx.counters.n_osc_l41_rejects, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
    // 父级别盘整背驰（Consolidation×Up）→ 衰竭证据成立 → 门开
    let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
    devs[3] = vec![DivEvent {
        kind: DivKind::Consolidation,
        direction: Direction::Up,
        seg_idx: 0,
        force_a: 0.0,
        force_c: 0.0,
        price: 0.0,
    }];
    te.observe(&pdir, &devs, 11.1);
    assert!(!te.up_unexhausted(3));
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.l41 = Some(&te);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    assert_eq!(fx.counters.n_osc_l41_rejects, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
}

#[test]
fn osc_domain_consolidation_only_blocks_trend_move_opens() {
    // osc 操作域严格化：锚中枢所在走势（trend_row[k]）kind==Trend ⇒
    // 操作对象不存在不开；kind==Consolidation ⇒ 38课域内正常开。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::ConsolidationOnly,
        ..OrganicConfig::default()
    };
    let mut trow = [false; MAX_LADDER];
    trow[2] = true; // 本层尾 move 是趋势走势
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1); // sub_sell = sell_any[k-1]
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 0);
    assert_eq!(fx.counters.n_osc_domain_rejects, 1);
    assert_eq!(fx.counters.osc_domain_reject_log, vec![(2, 1)]);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
    // 尾 move 翻盘整（趋势终结）→ 域内，正常开腿
    trow[2] = false;
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            2,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    assert_eq!(fx.counters.n_osc_domain_rejects, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
}

#[test]
fn osc_domain_does_not_constrain_close_path() {
    // 域只定义开腿对象——已开腿在走势翻趋势后照常按 ZD 触线回补
    // （闭腿是兑现路径，不是操作对象选择）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::ConsolidationOnly,
        ..OrganicConfig::default()
    };
    let mut trow = [false; MAX_LADDER]; // 盘整域内开腿
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1);
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    // 开腿后走势升级为趋势 → 触 ZD 仍回补
    trow[2] = true;
    let mut rows = empty_rows(&fx.evs, &fx.devs);
    rows.trend_row = Some(&trow);
    v.step(
        &cfg,
        &rows,
        &[],
        9.0,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_osc_zd_close, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
}

#[test]
fn trend_upshift_reroutes_open_to_parent_center() {
    // H3 级别上移：趋势态下 osc 重路由到 k+1 层中枢——触发判据
    // （c≥ZG(k+1)∧sub_sell(k)）、锚（k+1 中枢快照、kind=OscUp）全部
    // 按 k+1 级别；k 层中枢即使满足触发条件也不开（趋势态 k 层永不开）。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.book.ingest(
        3,
        &[ev_anchored(BspClass::Sell1, true, 7, 19.0, 19.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::TrendUpshift,
        ..OrganicConfig::default()
    };
    let mut trow = [false; MAX_LADDER];
    trow[2] = true; // 本层趋势态 ⇒ 重路由
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        // 上移后次级别 = k：sub_sell 读 sell_any[2]（同时设 [1] 证明
        // k 层判据不被消费——k 层触发要的是 sell_any[1]）
        rows.sell_any = LadderMask((1 << 2) | (1 << 1));
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            19.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    assert_eq!(fx.counters.n_osc_upshift_open, 1);
    assert_eq!(fx.counters.osc_upshift_open_log, vec![(2, 1)]);
    assert_eq!(fx.counters.n_osc_domain_rejects, 0); // 重路由非删除
    let leg = fx
        .ledger
        .open_slot(SlotKey::osc(2))
        .expect("上移腿占 k 层槽");
    assert!(matches!(
        leg.anchor,
        LegAnchor::Center {
            cs: Some(7),
            boundary: Some(b),
            zg: Some(g),
            kind: AnchorKind::OscUp,
        } if b == 19.0 && g == 19.5
    ));
}

#[test]
fn trend_upshift_no_parent_center_stays_flat() {
    // 趋势态 k+1 无中枢 ⇒ 自然不开（机制预测：负域浅回调在 k+1 无
    // 信号）；k 层中枢满足触发条件也不开——重路由不是 fallback。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::TrendUpshift,
        ..OrganicConfig::default()
    };
    let mut trow = [false; MAX_LADDER];
    trow[2] = true;
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask((1 << 2) | (1 << 1)); // k 层触发条件齐备
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 0);
    assert_eq!(fx.counters.n_osc_upshift_open, 0);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
}

#[test]
fn trend_upshift_consolidation_keeps_home_level() {
    // 盘整态维持 k 层原路径（ConsolidationOnly 放行分支同语义）——
    // 锚 kind=Osc、boundary=k 层 ZD，upshift 计数零。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::TrendUpshift,
        ..OrganicConfig::default()
    };
    let trow = [false; MAX_LADDER]; // 盘整态
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 1); // k 层次级别卖点
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            9.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_open, 1);
    assert_eq!(fx.counters.n_osc_upshift_open, 0);
    let leg = fx.ledger.open_slot(SlotKey::osc(2)).expect("k 层腿");
    assert!(matches!(
        leg.anchor,
        LegAnchor::Center { kind: AnchorKind::Osc, boundary: Some(b), .. } if b == 9.0
    ));
}

#[test]
fn trend_upshift_leg_closes_on_parent_boundary() {
    // 上移腿的出口跟随锚层：ZD 触线判据用 k+1 中枢边界（19.0），
    // k 层中枢边界（9.0）不被消费。
    let mut fx = Fixture::new();
    fx.book.ingest(
        2,
        &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)],
        true,
        None,
    );
    fx.book.ingest(
        3,
        &[ev_anchored(BspClass::Sell1, true, 7, 19.0, 19.5)],
        true,
        None,
    );
    let cfg = OrganicConfig {
        osc_domain: OscDomain::TrendUpshift,
        ..OrganicConfig::default()
    };
    let mut trow = [false; MAX_LADDER];
    trow[2] = true;
    let mut v = VoiceUnit::new(2);
    {
        let mut rows = empty_rows(&fx.evs, &fx.devs);
        rows.sell_any = LadderMask(1 << 2);
        rows.trend_row = Some(&trow);
        v.step(
            &cfg,
            &rows,
            &[],
            19.6,
            1,
            &mut fx.ledger,
            &fx.book,
            &fx.gate,
            4,
            &|_| 0.5,
            &mut fx.counters,
        );
    }
    assert_eq!(fx.counters.n_osc_upshift_open, 1);
    // c=19.0 触 k+1 层 ZD ⇒ 回补（k 层 ZD=9.0 远在下方——若闭腿误用
    // k 层边界本步不会触发）
    let mut rows = empty_rows(&fx.evs, &fx.devs);
    rows.trend_row = Some(&trow);
    v.step(
        &cfg,
        &rows,
        &[],
        19.0,
        2,
        &mut fx.ledger,
        &fx.book,
        &fx.gate,
        4,
        &|_| 0.5,
        &mut fx.counters,
    );
    assert_eq!(fx.counters.n_osc_zd_close, 1);
    assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
}
