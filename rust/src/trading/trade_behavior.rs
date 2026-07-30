//! 交易行为分解数据产出（2026-06-11 编排者任务）— 纯 Rust，不走 Python→PyO3。
//!
//! 职责：从磁带二进制（`analysis/_dump_tape_rust.py` 落盘，信号事件全部来自
//! Rust 增量接口）加载 SignalTape → run_organic V2r → 输出逐腿 JSON 供分析：
//!   - rev_open_log / rev_close_log（开腿触发 kind/trigger、锚中枢、闭腿原因）
//!   - diag 中 leg=rev 的 LegTrace（配对盈亏、持仓区间）
//!   - 全部 trades（权益曲线/回撤背景）
//!
//! 守卫：磁带 header 计数 + 跑后 counters 与在册 enginefix JSON 硬校验
//! （`rev_v2_paired_backtest_enginefix.json` V2of≡V2r 条目）——任何不一致
//! fail-fast，不提供静默混表。
//!
//! 运行（长测试，默认 ignore）：
//!   cargo test --release trade_behavior_oklo -- --ignored --nocapture
//!   cargo test --release trade_behavior_brn -- --ignored --nocapture
#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use super::config::{variant, StopMode};
use super::runner::run_organic;
use super::tape::{BarSig, SignalTape};
use super::types::{BspClass, BspEvent, DivEvent, LadderMask, MAX_LADDER};
use crate::divergence::DivKind;
use crate::stroke::Direction;

const MAGIC: u64 = 0x4E43545056325200;
const FLOOR_LADDER: usize = 2; // LADDER_SEG（fugue_version_i 口径，回测在册）

struct Rd {
    b: Vec<u8>,
    p: usize,
}

impl Rd {
    fn u64(&mut self) -> u64 {
        let v = u64::from_le_bytes(self.b[self.p..self.p + 8].try_into().unwrap());
        self.p += 8;
        v
    }
    fn i64(&mut self) -> i64 {
        let v = i64::from_le_bytes(self.b[self.p..self.p + 8].try_into().unwrap());
        self.p += 8;
        v
    }
    fn f64(&mut self) -> f64 {
        let v = f64::from_le_bytes(self.b[self.p..self.p + 8].try_into().unwrap());
        self.p += 8;
        v
    }
    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.b[self.p..self.p + 2].try_into().unwrap());
        self.p += 2;
        v
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.p];
        self.p += 1;
        v
    }
}

fn bsp_class(kind: u8, side: u8) -> BspClass {
    match (kind, side) {
        (1, 0) => BspClass::Buy1,
        (2, 0) => BspClass::Buy2,
        (3, 0) => BspClass::Buy3,
        (1, 1) => BspClass::Sell1,
        (2, 1) => BspClass::Sell2,
        (3, 1) => BspClass::Sell3,
        _ => panic!("非法 bsp 编码 kind={kind} side={side}"),
    }
}

pub(super) fn load_tape(sym: &str) -> SignalTape {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../analysis/data_cache/_tape_v2r_{sym}.bin"));
    let mut r = Rd {
        b: fs::read(&path).unwrap_or_else(|e| panic!("读磁带 {path:?}: {e}")),
        p: 0,
    };
    assert_eq!(r.u64(), MAGIC, "磁带 magic 不匹配——dump 格式代次错位");
    let n = r.u64() as usize;
    let n_bsp = r.u64() as usize;
    let n_div = r.u64() as usize;
    let n_flip = r.u64() as usize;

    let closes: Vec<f64> = (0..n).map(|_| r.f64()).collect();
    let mut masks: Vec<Vec<u16>> = Vec::with_capacity(5);
    for _ in 0..5 {
        masks.push((0..n).map(|_| r.u16()).collect());
    }
    let max_ladder: Vec<u8> = (0..n).map(|_| r.u8()).collect();
    let type2: Vec<u8> = (0..n).map(|_| r.u8()).collect();

    let mut bars: Vec<BarSig> = (0..n)
        .map(|i| BarSig {
            close: closes[i],
            buy1: LadderMask(masks[0][i]),
            sell1: LadderMask(masks[1][i]),
            sell_any: LadderMask(masks[2][i]),
            buy_any: LadderMask(masks[3][i]),
            up_move_settled: LadderMask(masks[4][i]),
            max_ladder: max_ladder[i],
            type2_buy: type2[i] != 0,
            ..Default::default()
        })
        .collect();

    for _ in 0..n_bsp {
        let bar = r.i64() as usize;
        let lad = r.u8() as usize;
        let kind = r.u8();
        let side = r.u8();
        let confirmed = r.u8() != 0;
        let has_cs = r.u8() != 0;
        let seg_idx = r.i64();
        let cs_raw = r.i64();
        let zd = r.f64();
        let zg = r.f64();
        let price = r.f64();
        bars[bar]
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()))[lad]
            .push(BspEvent {
                class: bsp_class(kind, side),
                seg_idx,
                confirmed,
                cs: has_cs.then_some(cs_raw),
                zd: has_cs.then_some(zd),
                zg: has_cs.then_some(zg),
                price,
            });
    }
    for _ in 0..n_div {
        let bar = r.i64() as usize;
        let lad = r.u8() as usize;
        let kind = if r.u8() == 0 {
            DivKind::Trend
        } else {
            DivKind::Consolidation
        };
        let direction = if r.u8() == 0 {
            Direction::Up
        } else {
            Direction::Down
        };
        let seg_idx = r.i64();
        let force_a = r.f64();
        let force_c = r.f64();
        let price = r.f64();
        bars[bar]
            .div_events
            .get_or_insert_with(|| Box::new(<[Vec<DivEvent>; MAX_LADDER]>::default()))[lad]
            .push(DivEvent {
                kind,
                direction,
                seg_idx,
                force_a,
                force_c,
                price,
            });
    }
    let dir_flips: Vec<(i64, u8, Direction)> = (0..n_flip)
        .map(|_| {
            let bar = r.i64();
            let lad = r.u8();
            let d = if r.u8() == 0 {
                Direction::Up
            } else {
                Direction::Down
            };
            (bar, lad, d)
        })
        .collect();
    assert_eq!(r.p, r.b.len(), "磁带尾部有未消费字节——格式错位");
    SignalTape {
        bars,
        dir_flips: Some(dir_flips),
        run_high: None,
        trend_flips: None,
    }
}

fn opt_i64(v: Option<i64>) -> String {
    v.map_or("null".into(), |x| x.to_string())
}

fn opt_f64(v: Option<f64>) -> String {
    v.map_or("null".into(), |x| format!("{x:?}"))
}

/// counters 在册期望（enginefix JSON V2of≡V2r 条目）。
struct Expect {
    open: u64,
    osc: u64,
    t5: u64,
    t6: u64,
    t7: u64,
    zd: u64,
    mismatch: u64,
    pairs: u64,
}

fn run_and_dump(sym: &str, exp: &Expect) {
    let tape = load_tape(sym);
    println!("[{sym}] 磁带 {} bars 加载完成", tape.bars.len());
    let cfg = variant("V2r").expect("V2r 在 config.rs 注册");
    let res = run_organic(&tape, FLOOR_LADDER, &cfg, StopMode::None, true)
        .unwrap_or_else(|e| panic!("run_organic: {e}"));
    let c = &res.counters;
    // ── 在册硬校验（fail-fast，混表禁止）──
    assert_eq!(c.n_rev_open, exp.open, "n_rev_open 与在册不符");
    assert_eq!(c.n_rev_open_osc, exp.osc, "n_rev_open_osc 与在册不符");
    assert_eq!(c.n_rev_close_t5, exp.t5, "t5 与在册不符");
    assert_eq!(c.n_rev_close_t6, exp.t6, "t6 与在册不符");
    assert_eq!(c.n_rev_close_t7, exp.t7, "t7 与在册不符");
    assert_eq!(c.n_rev_zd_close, exp.zd, "zd_close 与在册不符");
    assert_eq!(
        c.n_rev_mismatch_holds, exp.mismatch,
        "mismatch_holds 与在册不符"
    );
    assert_eq!(c.rev_osc_pairs, exp.pairs, "rev_osc_pairs 与在册不符");
    assert_eq!(
        c.rev_open_log.len() as u64,
        exp.open,
        "open_log 行数 ≠ n_rev_open"
    );
    assert_eq!(
        c.rev_close_log.len() as u64,
        exp.t5 + exp.t6 + exp.t7 + exp.zd,
        "close_log 行数 ≠ 四口闭腿合计"
    );
    println!(
        "[{sym}] 在册校验 PASS：open={} t6={} t7={} zd={} pairs={}",
        c.n_rev_open, c.n_rev_close_t6, c.n_rev_close_t7, c.n_rev_zd_close, c.rev_osc_pairs
    );

    // ── JSON 落盘 ──
    let mut s = String::with_capacity(1 << 20);
    s.push_str(&format!(
        "{{\n\"symbol\": \"{sym}\",\n\"n_bars\": {},\n\"variant\": \"V2r\",\n",
        tape.bars.len()
    ));
    s.push_str("\"open_log\": [");
    for (i, o) in c.rev_open_log.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\n[{},{},{},{},{},{},{},{:?}]",
            o.ladder,
            o.bar,
            o.kind,
            o.trigger,
            opt_i64(o.anchor_cs),
            opt_f64(o.zd),
            opt_f64(o.zg),
            o.price
        ));
    }
    s.push_str("],\n\"close_log\": [");
    for (i, cl) in c.rev_close_log.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\n[{},{},{},{},{:?}]",
            cl.ladder, cl.open_bar, cl.bar, cl.reason, cl.price
        ));
    }
    s.push_str("],\n\"rev_legs\": [");
    let mut first = true;
    for td in res.diag.as_ref().expect("diag=true") {
        for lt in &td.diffs {
            if lt.slot.leg_kind() != "rev" {
                continue;
            }
            if !first {
                s.push(',');
            }
            first = false;
            s.push_str(&format!(
                "\n[{},{},{:?},{},{:?},{:?},{:?},{:?}]",
                lt.slot.ladder,
                lt.sell_bar,
                lt.sell_price,
                lt.buy_bar,
                lt.buy_price,
                lt.shares,
                lt.diff,
                lt.profit
            ));
        }
    }
    s.push_str("],\n\"trades\": [");
    for (i, t) in res.trades.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\n[{},{:?},{},{:?},{:?},\"{}\"]",
            t.entry_bar, t.entry_price, t.exit_bar, t.exit_price, t.pnl_pct, t.exit_reason
        ));
    }
    s.push_str("]\n}\n");
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../analysis/data_cache/_trade_behavior_{sym}.json"));
    fs::write(&out, &s).unwrap();
    println!(
        "[{sym}] 落盘 {out:?}：open_log={} close_log={} rev_legs(LegTrace)=见文件 trades={}",
        c.rev_open_log.len(),
        c.rev_close_log.len(),
        res.trades.len()
    );
}

#[test]
#[ignore]
fn trade_behavior_oklo() {
    run_and_dump(
        "OKLO",
        &Expect {
            open: 185,
            osc: 185,
            t5: 0,
            t6: 116,
            t7: 29,
            zd: 38,
            mismatch: 18,
            pairs: 183,
        },
    );
}

#[test]
#[ignore]
fn trade_behavior_brn() {
    run_and_dump(
        "BRN",
        &Expect {
            open: 77,
            osc: 77,
            t5: 0,
            t6: 39,
            t7: 20,
            zd: 18,
            mismatch: 21,
            pairs: 77,
        },
    );
}
