//! #1302 对拍锁：图/链解栈机制链可达域 bit-exact 对拍（#1278 批三 B7c 裁定）。
//!
//! ## 锁的对象
//!
//! 本仓有两条解栈会计原语，覆盖同一套「附庸的附庸不是我的附庸」回流语义：
//! - **链机制**：`nested_fugue::{pop_tail, unwind_to}`（`Vec<Voice>` 栈；nrf/urs/pcf/nif/rnf
//!   五个链引擎共用的线性解栈原语）；
//! - **图机制**：`isolated_fugue::close_voice`（`Vec<VoiceLedger>` 森林；iso 引擎，unn 复用）。
//!
//! 两者在**链可达域**上应对同一输入产生 bit-exact 相同的解栈后状态。链可达域 =
//! 森林退化为一条线性路径（root → child → grandchild → …，每 voice 至多一个活跃子），
//! 此时 `unwind_to(g)`（从链位置 g 解栈到尾）与 `close_voice(g)`（后序级联关闭 g 及
//! 其活跃子树）是同一会计的两种表示。
//!
//! ## 判定面（issue #1302）
//!
//! 解栈后状态逐位比对三面：
//! 1. **voice 存活集**：链解栈后 = `specs[0..g]`；森林解栈后 = `status != Closed` 的
//!    voice 集，须恰为连续前缀 `0..g`（数量与身份逐位）；
//! 2. **暴露**：存活 voice 的 units/basis/cost_pool/capital + `n_base` 逐位（`to_bits`）；
//! 3. **锁存清空**：森林被解栈 voice（`g..n`）`status == Closed` 且 `units == 0`。
//!
//! 另逐位比对 free（现金）与解栈会计观测面（cascade 计数 / shrink / earning / exits /
//! short_net_cash / trades 行）——两者同为解栈机制的输出，任一漂移都算差异清单。
//!
//! ## 输入向量：链引擎实际解栈触发序列
//!
//! - **negate**（否定扫描）：`unwind_to(g)`，g = 根→尾第一个「背驰段被打破」的 voice
//!   （可落在 0..n 任意位置）；
//! - **sellpt**（清仓 C）：`unwind_to(0)` 全链解栈；
//! - **eod**：`unwind_to(0)` 全链解栈；
//! - **liq**（强平兜底）：尾空头单级 `pop_tail`，trade 行价 = 2×basis；
//! - **recover**（回补 D）：非根尾单级 `pop_tail`，`at_point = true`（earning 增仓时机）。
//!
//! 图机制对应侧均为 `close_voice(g, …, at_point)`：negate/sellpt/eod/liq 传 `false`，
//! recover 传 `true`。
//!
//! ## 结论语义
//!
//! 全绿 = 图 vs 链在链可达域上 bit-exact ⟹ 表示差（#1278 B7c 裁定「bit-exact 则图 vs
//! 链降级为表示差」）。任一断言红 = 差异清单（assert 消息即差异明细），按 #1302 验收
//! 口径列票（收敛设计票 #1293 同批输入）。
#![cfg(test)]

use super::isolated_fugue::{close_voice, VoiceLedger, VoiceStatus};
use super::nested_fugue::{pop_tail, unwind_to, Voice};
use super::positional::{LayerTrade, PositionalResult};
use super::types::{Polarity, MAX_LADDER};

/// 场景规格：一个 voice 的链/森林公有字段（链可达域下两表示逐字段同源）。
struct VSpec {
    ladder: usize,
    dir: Polarity,
    units: f64,
    basis: f64,
    cost_pool: f64,
    capital: f64,
    entry_bar: i64,
    negate_line: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn v(
    ladder: usize,
    dir: Polarity,
    units: f64,
    basis: f64,
    cost_pool: f64,
    capital: f64,
    entry_bar: i64,
    negate_line: Option<f64>,
) -> VSpec {
    VSpec {
        ladder,
        dir,
        units,
        basis,
        cost_pool,
        capital,
        entry_bar,
        negate_line,
    }
}

/// 由 specs 建链表示（root 在 0，尾在 n−1；父 = 前一索引，隐式）。
fn build_chain(specs: &[VSpec]) -> Vec<Voice> {
    specs
        .iter()
        .map(|s| Voice {
            ladder: s.ladder,
            dir: s.dir,
            units: s.units,
            basis: s.basis,
            cost_pool: s.cost_pool,
            capital: s.capital,
            entry_bar: s.entry_bar,
            negate_line: s.negate_line,
        })
        .collect()
}

/// 由 specs 建森林表示（线性路径：parent = Some(i−1)，children = [i+1]）。
fn build_forest(specs: &[VSpec]) -> Vec<VoiceLedger> {
    let n = specs.len();
    specs
        .iter()
        .enumerate()
        .map(|(i, s)| VoiceLedger {
            ladder: s.ladder,
            dir: s.dir,
            units: s.units,
            basis: s.basis,
            cost_pool: s.cost_pool,
            capital: s.capital,
            entry_bar: s.entry_bar,
            negate_line: s.negate_line,
            status: VoiceStatus::Active,
            parent: if i == 0 { None } else { Some(i - 1) },
            children: if i + 1 < n { vec![i + 1] } else { Vec::new() },
            realized_pnl: 0.0,
            acted_bar: -1,
        })
        .collect()
}

/// 链机制入口：多级解栈（negate/sellpt/eod 走 `unwind_to`，at_point 恒 false）或
/// 单级弹尾（liq/recover 走 `pop_tail`，at_point 按场景）。
enum ChainOp {
    Unwind { g: usize },
    PopTail,
}

impl ChainOp {
    fn g(&self, n: usize) -> usize {
        match self {
            ChainOp::Unwind { g } => *g,
            ChainOp::PopTail => n - 1,
        }
    }
}

fn assert_f64_eq(a: f64, b: f64, ctx: &str, what: &str) {
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "{ctx}: {what} 漂移（chain={a} vs forest={b}）"
    );
}

fn assert_f64_arr_eq(a: &[f64; MAX_LADDER], b: &[f64; MAX_LADDER], ctx: &str, what: &str) {
    for (k, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "{ctx}: {what}[{k}] 漂移（chain={x} vs forest={y}）"
        );
    }
}

fn assert_voice_eq(cv: &Voice, fv: &VoiceLedger, ctx: &str) {
    assert_eq!(cv.ladder, fv.ladder, "{ctx}: ladder 漂移");
    assert_eq!(cv.dir, fv.dir, "{ctx}: dir 漂移");
    assert_f64_eq(cv.units, fv.units, ctx, "units");
    assert_f64_eq(cv.basis, fv.basis, ctx, "basis");
    assert_f64_eq(cv.cost_pool, fv.cost_pool, ctx, "cost_pool");
    assert_f64_eq(cv.capital, fv.capital, ctx, "capital");
    assert_eq!(cv.entry_bar, fv.entry_bar, "{ctx}: entry_bar 漂移");
    match (cv.negate_line, fv.negate_line) {
        (None, None) => {}
        (Some(a), Some(b)) => assert_f64_eq(a, b, ctx, "negate_line"),
        (a, b) => panic!("{ctx}: negate_line 形态漂移（chain={a:?} vs forest={b:?}）"),
    }
}

fn assert_trades_eq(a: &[LayerTrade], b: &[LayerTrade], ctx: &str) {
    assert_eq!(a.len(), b.len(), "{ctx}: trades 行数漂移");
    for (i, (ta, tb)) in a.iter().zip(b.iter()).enumerate() {
        let c = format!("{ctx} trade[{i}]");
        assert_eq!(ta.ladder, tb.ladder, "{c}: ladder");
        assert_eq!(ta.entry_bar, tb.entry_bar, "{c}: entry_bar");
        assert_f64_eq(ta.entry_price, tb.entry_price, &c, "entry_price");
        assert_eq!(ta.exit_bar, tb.exit_bar, "{c}: exit_bar");
        assert_f64_eq(ta.exit_price, tb.exit_price, &c, "exit_price");
        assert_f64_eq(ta.shares, tb.shares, &c, "shares");
        assert_f64_eq(
            ta.weight_at_entry,
            tb.weight_at_entry,
            &c,
            "weight_at_entry",
        );
        assert_eq!(ta.deferred_bars, tb.deferred_bars, "{c}: deferred_bars");
        assert_eq!(ta.partial, tb.partial, "{c}: partial");
        assert_eq!(ta.exit_reason, tb.exit_reason, "{c}: exit_reason");
        assert_eq!(ta.polarity, tb.polarity, "{c}: polarity");
    }
}

/// 核心对拍：同一 specs、同一解栈入口、同一价格/理由/at_point，跑链与图两机制，
/// 逐位比对解栈后状态（存活集/暴露/锁存清空/free/n_base/会计观测面/trades）。
fn run_compare(
    specs: &[VSpec],
    op: ChainOp,
    bar: i64,
    px: f64,
    c: f64,
    reason: &'static str,
    at_point: bool,
) {
    let n = specs.len();
    let g = op.g(n);
    assert!(g < n, "解栈位置 g={g} 越界（链长 {n}）");

    let chain = build_chain(specs);
    let forest = build_forest(specs);
    let free0 = 0.0;
    let n_base0: f64 = specs.iter().map(|s| s.units).sum();

    let mut chain = chain;
    let mut free_c = free0;
    let mut n_base_c = n_base0;
    let mut res_c = PositionalResult::default();
    match op {
        ChainOp::Unwind { g: ug } => unwind_to(
            ug,
            bar,
            px,
            c,
            reason,
            &mut chain,
            &mut free_c,
            &mut n_base_c,
            &mut res_c,
        ),
        ChainOp::PopTail => pop_tail(
            bar,
            px,
            c,
            reason,
            at_point,
            &mut chain,
            &mut free_c,
            &mut n_base_c,
            &mut res_c,
        ),
    }

    let mut forest = forest;
    let mut free_f = free0;
    let mut n_base_f = n_base0;
    let mut res_f = PositionalResult::default();
    close_voice(
        g,
        bar,
        px,
        c,
        reason,
        at_point,
        &mut forest,
        &mut free_f,
        &mut n_base_f,
        &mut res_f,
    );

    let ctx = format!("g={g} reason={reason} c={c} px={px} at_point={at_point}");

    // ① voice 存活集。
    assert_eq!(chain.len(), g, "{ctx}: 链解栈后存活数应 = g");
    let survivors: Vec<usize> = forest
        .iter()
        .enumerate()
        .filter(|(_, x)| x.status != VoiceStatus::Closed)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(survivors.len(), g, "{ctx}: 森林存活集大小漂移");
    for (k, &id) in survivors.iter().enumerate() {
        assert_eq!(id, k, "{ctx}: 森林存活集非连续前缀（位 {k} = {id}）");
    }

    // ② 暴露：存活 voice 逐位 + n_base。
    for k in 0..g {
        assert_voice_eq(&chain[k], &forest[k], &format!("{ctx} survivor[{k}]"));
    }
    assert_f64_eq(n_base_c, n_base_f, &ctx, "n_base");
    assert_f64_eq(free_c, free_f, &ctx, "free");

    // ③ 锁存清空：森林被解栈 voice g..n 必须 Closed 且 units == 0。
    for (id, fv) in forest.iter().enumerate().skip(g) {
        assert_eq!(fv.status, VoiceStatus::Closed, "{ctx}: forest[{id}] 未关闭");
        assert_eq!(
            fv.units.to_bits(),
            0.0f64.to_bits(),
            "{ctx}: forest[{id}] units 未清空（={}）",
            fv.units
        );
    }

    // ④ 解栈会计观测面 bit-exact。
    assert_eq!(
        res_c.n_nrf_cascade_closes_by_ladder, res_f.n_nrf_cascade_closes_by_ladder,
        "{ctx}: cascade 计数漂移"
    );
    assert_f64_eq(
        res_c.nrf_shrink_units,
        res_f.nrf_shrink_units,
        &ctx,
        "nrf_shrink_units",
    );
    assert_f64_eq(
        res_c.nrf_earning_units,
        res_f.nrf_earning_units,
        &ctx,
        "nrf_earning_units",
    );
    assert_eq!(
        res_c.n_nrf_earning_adds_by_ladder, res_f.n_nrf_earning_adds_by_ladder,
        "{ctx}: earning_adds 漂移"
    );
    assert_eq!(
        res_c.nrf_short_earning_hits, res_f.nrf_short_earning_hits,
        "{ctx}: short_earning_hits 漂移"
    );
    assert_eq!(
        res_c.n_exits_by_ladder, res_f.n_exits_by_ladder,
        "{ctx}: exits 漂移"
    );
    assert_f64_arr_eq(
        &res_c.short_net_cash_by_ladder,
        &res_f.short_net_cash_by_ladder,
        &ctx,
        "short_net_cash",
    );
    assert_trades_eq(&res_c.trades, &res_f.trades, &ctx);
}

/// 三声部链（root 多@4 → 子空@3 → 孙多@2）：层序、dir 交替；子/孙成本池与弹药按
/// spawn 惯例，root 成本池刻意取小（1_000，非 units×basis）以覆盖 excess>0 的
/// 现金沉淀分支。覆盖「孙多回流空父 → 子空回流多根」的双段级联。
fn specs_three() -> Vec<VSpec> {
    vec![
        v(4, Polarity::Long, 800.0, 100.0, 1_000.0, 0.0, 0, None),
        v(
            3,
            Polarity::Short,
            160.0,
            105.0,
            16_800.0,
            16_800.0,
            5,
            Some(110.0),
        ),
        v(
            2,
            Polarity::Long,
            40.0,
            110.0,
            4_400.0,
            0.0,
            10,
            Some(115.0),
        ),
    ]
}

/// 四声部链（root 多@5 → 子空@4 → 孙多@3 → 曾孙空@2）：三段级联 + 交替回流。
fn specs_four() -> Vec<VSpec> {
    vec![
        v(5, Polarity::Long, 640.0, 100.0, 100_000.0, 0.0, 0, None),
        v(
            4,
            Polarity::Short,
            128.0,
            105.0,
            13_440.0,
            13_440.0,
            5,
            Some(110.0),
        ),
        v(
            3,
            Polarity::Long,
            32.0,
            110.0,
            3_520.0,
            0.0,
            10,
            Some(115.0),
        ),
        v(
            2,
            Polarity::Short,
            8.0,
            115.0,
            920.0,
            920.0,
            15,
            Some(120.0),
        ),
    ]
}

/// negate（否定扫描）· 中间位解栈：g=1，孙多级联回流空父后子空回流多根。
/// 三个 c 覆盖：盈亏平衡（shrink=0, excess=0）/ 亏损（shrink>0）/ 盈利（excess>0,
/// at_point=false ⟹ 现金沉淀，不增仓）。
#[test]
fn negate_middle_destack_bit_exact() {
    let specs = specs_three();
    for c in [105.0, 130.0, 98.0] {
        run_compare(&specs, ChainOp::Unwind { g: 1 }, 20, c, c, "negate", false);
    }
}

/// negate（否定扫描）· 尾位解栈：g=2（孙多单级回流空父）。c 覆盖孙多亏损/持平/盈利。
#[test]
fn negate_tail_destack_bit_exact() {
    let specs = specs_three();
    for c in [95.0, 110.0, 120.0] {
        run_compare(&specs, ChainOp::Unwind { g: 2 }, 20, c, c, "negate", false);
    }
}

/// sellpt（清仓 C）· 全链解栈：g=0，根→尾三段级联（孙多→子空→根多）。c 覆盖
/// 缩水（shrink）与盈利（excess 现金沉淀）分支。
#[test]
fn clearance_full_destack_bit_exact() {
    let specs = specs_three();
    for c in [90.0, 105.0, 130.0] {
        run_compare(&specs, ChainOp::Unwind { g: 0 }, 20, c, c, "sellpt", false);
    }
}

/// eod · 全链解栈：g=0，理由 eod（末 bar 平仓）。三段级联。
#[test]
fn eod_full_destack_bit_exact() {
    let specs = specs_three();
    for c in [100.0, 112.0] {
        run_compare(&specs, ChainOp::Unwind { g: 0 }, 30, c, c, "eod", false);
    }
}

/// liq（强平兜底）· 尾空头单级解栈：trade 行价 px = 2×basis。c 覆盖缩水与持平。
#[test]
fn liq_tail_destack_bit_exact() {
    let specs = vec![
        v(4, Polarity::Long, 800.0, 100.0, 100_000.0, 0.0, 0, None),
        v(
            3,
            Polarity::Short,
            160.0,
            105.0,
            16_800.0,
            16_800.0,
            5,
            Some(110.0),
        ),
    ];
    for c in [130.0, 105.0] {
        run_compare(&specs, ChainOp::PopTail, 20, 2.0 * 105.0, c, "liq", false);
    }
}

/// recover（回补 D）· 尾空头单级解栈 at_point=true：盈利 excess > 父 cost_pool ⟹
/// earning 增仓分支（Δ = excess/c，basis 加权，n_base 重定基）。另覆盖 excess 落入
/// 成本池（无增仓）与持平（excess=0）两臂。
#[test]
fn recover_tail_at_point_destack_bit_exact() {
    // 父 cost_pool=1000，子空 capital=20000、units=200：c=100 ⟹ leftover=20000−200×100=0
    // ⟹ 持平。c=95 ⟹ leftover=1000 ⟹ excess=0（全落入父成本池）。c=90 ⟹ leftover=2000
    // ⟹ excess=1000 ⟹ earning 增仓 Δ=1000/90。
    let specs = vec![
        v(4, Polarity::Long, 800.0, 100.0, 1_000.0, 0.0, 0, None),
        v(
            3,
            Polarity::Short,
            200.0,
            100.0,
            20_000.0,
            20_000.0,
            5,
            Some(110.0),
        ),
    ];
    for c in [100.0, 95.0, 90.0] {
        run_compare(&specs, ChainOp::PopTail, 20, c, c, "recover", true);
    }
}

/// recover（回补 D）· 尾孙多头单级解栈 at_point=true：孙多回流空父（proceeds/利润
/// 递减父成本池），profit 超父成本池且 at_point ⟹ short_earning_hits 计数分支。
#[test]
fn recover_long_tail_at_point_destack_bit_exact() {
    // 子空父 cost_pool=1000；孙多 units=40 basis=110。c=120 ⟹ profit=400 ≤ 1000 无
    // short_earning_hits；c=140 ⟹ profit=1200 > 1000（父 cost_pool）⟹ short_earning_hits。
    let specs = vec![
        v(4, Polarity::Long, 800.0, 100.0, 100_000.0, 0.0, 0, None),
        v(
            3,
            Polarity::Short,
            160.0,
            105.0,
            1_000.0,
            16_800.0,
            5,
            Some(110.0),
        ),
        v(
            2,
            Polarity::Long,
            40.0,
            110.0,
            4_400.0,
            0.0,
            10,
            Some(115.0),
        ),
    ];
    for c in [120.0, 140.0] {
        run_compare(&specs, ChainOp::PopTail, 20, c, c, "recover", true);
    }
}

/// 四声部深链 · 中间位解栈：g=1 三段级联（曾孙空→孙多→子空→根多），交替回流全开。
#[test]
fn deep_cascade_middle_destack_bit_exact() {
    let specs = specs_four();
    for c in [95.0, 115.0, 135.0] {
        run_compare(&specs, ChainOp::Unwind { g: 1 }, 20, c, c, "negate", false);
    }
}

/// 四声部深链 · 全链解栈：g=0 四段级联（sellpt）。
#[test]
fn deep_cascade_full_destack_bit_exact() {
    let specs = specs_four();
    for c in [100.0, 125.0] {
        run_compare(&specs, ChainOp::Unwind { g: 0 }, 20, c, c, "sellpt", false);
    }
}

/// 单根链 · 根解栈：g=0（sellpt/eod 根分支）。根恒多头。
#[test]
fn root_only_destack_bit_exact() {
    let specs = vec![v(
        4,
        Polarity::Long,
        1_000.0,
        100.0,
        100_000.0,
        0.0,
        0,
        None,
    )];
    run_compare(
        &specs,
        ChainOp::Unwind { g: 0 },
        20,
        105.0,
        105.0,
        "sellpt",
        false,
    );
    run_compare(
        &specs,
        ChainOp::Unwind { g: 0 },
        30,
        105.0,
        105.0,
        "eod",
        false,
    );
}
