//! 次级别确认完整递归消融回测（2026-06-11 任务）— 纯 Rust 链路。
//!
//! 27课区间套："每个买卖点都要用次级别来把握"。SC 六位把次级别确认延拓到
//! 全部尚无确认的操作点（盘点见 `analysis/sublevel_confirmation_recursive.md`）：
//!   SCe 入场严格化 / SCm master 出场 / SCo main 卖开 / SCc main Buy1 闭 /
//!   SCr REV 开腿 / SC7 confirmed Buy3 回补。
//!
//! 统一谓词 `BarRows::sub_confirm` = 同 bar 事件证据 ∨ D3 方向行结构证据
//! （非 R1 的窗口记忆形式——R1 在 bi 层空定义域反选，已否证）。
//!
//! 守卫：V2r 基线 counters 与在册 enginefix JSON 硬校验（SC 位默认关 ⇒ 零侵入）。
//!
//! 运行（长测试，默认 ignore）：
//!   cargo test --release sublevel_confirm_oklo -- --ignored --nocapture
//! 输出：analysis/data_cache/_sublevel_confirm_OKLO.json
#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::config::{variant, StopMode};
use super::runner::run_organic;
use super::trade_behavior::load_tape;

const FLOOR_LADDER: usize = 2; // LADDER_SEG（在册回测口径）

const VARIANTS: [&str; 8] = [
    "V2r", "V2rSCe", "V2rSCm", "V2rSCo", "V2rSCc", "V2rSCr", "V2rSC7", "V2rSCall",
];

struct LegStats {
    n: u64,
    wins: u64,
    net: f64,
}

impl LegStats {
    fn new() -> Self {
        LegStats { n: 0, wins: 0, net: 0.0 }
    }
    fn add(&mut self, p: f64) {
        self.n += 1;
        self.net += p;
        if p > 0.0 {
            self.wins += 1;
        }
    }
}

// ════════════════════════════════════════════════════════
// SC 单测（快测试，不 ignore）——置于本文件以与消融 harness 同域
// ════════════════════════════════════════════════════════

mod sc_unit {
    use crate::buysellpoint::Side;
    use crate::divergence::DivKind;
    use crate::stroke::Direction;
    use crate::trading::center_book::CenterBook;
    use crate::trading::config::OrganicConfig;
    use crate::trading::fatigue_gate::FatigueGate;
    use crate::trading::ledger::OrganicLedger;
    use crate::trading::level_operating_unit::{BarRows, VoicePhase, VoiceUnit};
    use crate::trading::master::MasterExitSignal;
    use crate::trading::types::*;

    fn ev(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent { class, seg_idx: 0, confirmed, cs: Some(cs), zd: Some(zd), zg: Some(zg), price: 10.0 }
    }

    fn dev_sell() -> DivEvent {
        DivEvent {
            kind: DivKind::Trend,
            direction: Direction::Up,
            seg_idx: 0,
            force_a: 1.0,
            force_c: 0.5,
            price: 10.0,
        }
    }

    fn rows_with<'a>(
        evs: &'a [Vec<BspEvent>; MAX_LADDER],
        devs: &'a [Vec<DivEvent>; MAX_LADDER],
        dir_row: &'a [Option<Direction>; MAX_LADDER],
        anchors: &'a [i64; MAX_LADDER],
        buy_any: LadderMask,
        sell_any: LadderMask,
    ) -> BarRows<'a> {
        BarRows {
            evs,
            devs,
            buy_any,
            sell_any,
            dir_row: Some(dir_row),
            run_anchor: Some(anchors),
            depth: None,
            l41: None,
        }
    }

    #[test]
    fn sub_confirm_event_or_dir_evidence() {
        let evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
        let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let anchors = [0i64; MAX_LADDER];
        // 无任何证据 → 拒
        {
            let r = rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            assert!(!r.sub_confirm(2, Side::Sell));
            assert!(!r.sub_confirm(2, Side::Buy));
        }
        // 事件证据：sell_any[k−1] 置位 → Sell 确认（Buy 仍拒）
        {
            let r =
                rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(1 << 1));
            assert!(r.sub_confirm(2, Side::Sell));
            assert!(!r.sub_confirm(2, Side::Buy));
        }
        // 事件证据：次级别卖侧背驰 → Sell 确认
        devs[1] = vec![dev_sell()];
        {
            let r = rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            assert!(r.sub_confirm(2, Side::Sell));
        }
        devs[1].clear();
        // 结构证据：dir_row[k−1]=Down → Sell 确认（bi 层无事件流时的唯一通道，
        // R1 空定义域陷阱的消除点）
        dir_row[1] = Some(Direction::Down);
        {
            let r = rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            assert!(r.sub_confirm(2, Side::Sell));
            assert!(!r.sub_confirm(2, Side::Buy));
        }
        // dir 翻 Up → Buy 确认
        dir_row[1] = Some(Direction::Up);
        {
            let r = rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            assert!(r.sub_confirm(2, Side::Buy));
            assert!(!r.sub_confirm(2, Side::Sell));
        }
    }

    #[test]
    fn master_sub_confirmed_entrance() {
        // 532号类型隔离守卫：合取入口仍只接受布尔，确认假 ⇒ 无信号
        assert!(MasterExitSignal::from_sell1_row_sub_confirmed(true, true).is_some());
        assert!(MasterExitSignal::from_sell1_row_sub_confirmed(true, false).is_none());
        assert!(MasterExitSignal::from_sell1_row_sub_confirmed(false, true).is_none());
    }

    #[test]
    fn sc_rev_open_rejects_without_sub_evidence_opens_with_dir() {
        // SCr：confirmed Sell1 × 存活中枢，无次级别证据 → 拒（计数）；
        // dir_row[k−1]=Down → 开腿。
        let cfg = OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            sc_rev_open: true,
            ..OrganicConfig::default()
        };
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        let mut ledger = OrganicLedger::new(10.0, 100.0, 0.5, false);
        let gate = FatigueGate::new();
        let mut counters = Counters::default();
        let mut evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
        let devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        evs[2] = vec![ev(BspClass::Sell1, true, 1, 9.0, 9.5)];
        let mut dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let anchors = [0i64; MAX_LADDER];
        let mut v = VoiceUnit::new(2);
        {
            let rows =
                rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            v.step(&cfg, &rows, &[], 9.6, 1, &mut ledger, &book, &gate, 4, &|_| 0.5, &mut counters);
        }
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(counters.n_sc_rev_open_rejects, 1);
        assert_eq!(counters.n_rev_open, 0);
        // 次级别方向翻 Down → 确认成立，开腿
        dir_row[1] = Some(Direction::Down);
        let rows = rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
        v.step(&cfg, &rows, &[], 9.6, 2, &mut ledger, &book, &gate, 4, &|_| 0.5, &mut counters);
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(counters.n_rev_open, 1);
    }

    #[test]
    fn sc_t7_holds_then_closes_with_sub_buy_evidence() {
        // SC7：confirmed Buy3 无次级别买证据 → 持有（计数）；buy_any[k−1] → T7 闭。
        let cfg = OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            sc_t7_close: true,
            ..OrganicConfig::default()
        };
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        let mut ledger = OrganicLedger::new(10.0, 100.0, 0.5, false);
        let gate = FatigueGate::new();
        let mut counters = Counters::default();
        let mut evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
        let devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        evs[2] = vec![ev(BspClass::Sell1, true, 1, 9.0, 9.5)];
        let dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let anchors = [0i64; MAX_LADDER];
        let mut v = VoiceUnit::new(2);
        {
            let rows =
                rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            v.step(&cfg, &rows, &[], 9.6, 1, &mut ledger, &book, &gate, 4, &|_| 0.5, &mut counters);
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        // confirmed Buy3，无次级别买证据 → 持有
        evs[2] = vec![ev(BspClass::Buy3, true, 1, 9.0, 9.5)];
        {
            let rows =
                rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(0), LadderMask(0));
            v.step(&cfg, &rows, &[], 9.8, 2, &mut ledger, &book, &gate, 4, &|_| 0.5, &mut counters);
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(counters.n_sc_t7_holds, 1);
        assert_eq!(counters.n_rev_close_t7, 0);
        // buy_any[1] 置位 → T7 闭腿
        let rows =
            rows_with(&evs, &devs, &dir_row, &anchors, LadderMask(1 << 1), LadderMask(0));
        v.step(&cfg, &rows, &[], 9.8, 3, &mut ledger, &book, &gate, 4, &|_| 0.5, &mut counters);
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(counters.n_rev_close_t7, 1);
    }
}

#[test]
#[ignore]
fn sublevel_confirm_oklo() {
    let tape = load_tape("OKLO");
    println!("[OKLO] 磁带 {} bars 加载完成", tape.bars.len());
    let mut json = String::with_capacity(1 << 16);
    json.push_str("{\n\"symbol\": \"OKLO\",\n\"floor_ladder\": 2,\n\"variants\": {");

    for (vi, name) in VARIANTS.iter().enumerate() {
        let cfg = variant(name).unwrap_or_else(|| panic!("{name} 未注册"));
        let res = run_organic(&tape, FLOOR_LADDER, &cfg, StopMode::None, true)
            .unwrap_or_else(|e| panic!("run_organic {name}: {e}"));
        let c = &res.counters;

        // ── V2r 基线零侵入守卫（在册 enginefix 期望，consol harness 同源）──
        if *name == "V2r" {
            assert_eq!(c.n_rev_open, 185, "V2r 基线 n_rev_open 漂移——SC 位侵入基线");
            assert_eq!(c.n_rev_open_osc, 185);
            assert_eq!(c.n_rev_close_t6, 116);
            assert_eq!(c.n_rev_close_t7, 29);
            assert_eq!(c.n_rev_zd_close, 38);
            assert_eq!(c.rev_osc_pairs, 183);
            assert_eq!(c.n_sc_entry_expire_skips, 0);
            assert_eq!(c.n_sc_master_holds, 0);
            assert_eq!(c.n_sc_main_open_rejects, 0);
            assert_eq!(c.n_sc_main_close_holds, 0);
            assert_eq!(c.n_sc_rev_open_rejects, 0);
            assert_eq!(c.n_sc_t7_holds, 0);
            println!("[V2r] 基线零侵入守卫 PASS（在册逐计数相等 + SC 计数全零）");
        }

        // ── trade 级指标 ──
        let mut equity = 1.0f64;
        let mut peak = 1.0f64;
        let mut max_dd = 0.0f64;
        let mut t_wins = 0u64;
        for t in &res.trades {
            equity *= 1.0 + t.pnl_pct / 100.0;
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
            if t.pnl_pct > 0.0 {
                t_wins += 1;
            }
        }
        let compound = (equity - 1.0) * 100.0;

        // ── 腿级分解（main/osc/rev 三类净现金——SC 点改动的直接观测面）──
        let mut by_leg: HashMap<&str, LegStats> = HashMap::new();
        for td in res.diag.as_ref().expect("diag=true") {
            for lt in &td.diffs {
                by_leg
                    .entry(lt.slot.leg_kind())
                    .or_insert_with(LegStats::new)
                    .add(lt.profit);
            }
        }

        println!(
            "[{name:9}] 复利={compound:+9.2}% trades={} maxDD={max_dd:.1}% | \
             SCe跳={} SCm持={} SCo拒={} SCc持={} SCr拒={} SC7持={} | \
             rev开={} t6={} t7={} zd={}",
            res.trades.len(),
            c.n_sc_entry_expire_skips,
            c.n_sc_master_holds,
            c.n_sc_main_open_rejects,
            c.n_sc_main_close_holds,
            c.n_sc_rev_open_rejects,
            c.n_sc_t7_holds,
            c.n_rev_open,
            c.n_rev_close_t6,
            c.n_rev_close_t7,
            c.n_rev_zd_close,
        );
        for kind in ["main", "osc", "rev"] {
            if let Some(s) = by_leg.get(kind) {
                println!(
                    "    leg={kind:4} n={:4} 胜率={:.1}% 净={:+.0}",
                    s.n,
                    if s.n > 0 { s.wins as f64 / s.n as f64 * 100.0 } else { 0.0 },
                    s.net
                );
            }
        }

        // ── JSON ──
        if vi > 0 {
            json.push(',');
        }
        json.push_str(&format!("\n\"{name}\": {{"));
        json.push_str(&format!(
            "\"compound_pct\": {compound:.4}, \"n_trades\": {}, \"trade_wins\": {t_wins}, \
             \"max_dd_pct\": {max_dd:.4}, ",
            res.trades.len()
        ));
        json.push_str(&format!(
            "\"sc_counters\": {{\"n_sc_entry_expire_skips\": {}, \"n_sc_master_holds\": {}, \
             \"n_sc_main_open_rejects\": {}, \"n_sc_main_close_holds\": {}, \
             \"n_sc_rev_open_rejects\": {}, \"n_sc_t7_holds\": {}}}, ",
            c.n_sc_entry_expire_skips,
            c.n_sc_master_holds,
            c.n_sc_main_open_rejects,
            c.n_sc_main_close_holds,
            c.n_sc_rev_open_rejects,
            c.n_sc_t7_holds
        ));
        json.push_str(&format!(
            "\"rev_counters\": {{\"n_rev_attempts\": {}, \"n_rev_open\": {}, \
             \"n_rev_close_t5\": {}, \"n_rev_close_t6\": {}, \"n_rev_close_t7\": {}, \
             \"n_rev_zd_close\": {}, \"rev_osc_pairs\": {}, \"rev_osc_wins\": {}, \
             \"rev_osc_cash\": {:.2}}}, ",
            c.n_rev_attempts,
            c.n_rev_open,
            c.n_rev_close_t5,
            c.n_rev_close_t6,
            c.n_rev_close_t7,
            c.n_rev_zd_close,
            c.rev_osc_pairs,
            c.rev_osc_wins,
            c.rev_osc_cash
        ));
        let legs: Vec<String> = ["main", "osc", "rev"]
            .iter()
            .filter_map(|k| {
                by_leg.get(*k).map(|s| {
                    format!("\"{k}\": {{\"n\": {}, \"wins\": {}, \"net\": {:.2}}}", s.n, s.wins, s.net)
                })
            })
            .collect();
        json.push_str(&format!("\"by_leg\": {{{}}}}}", legs.join(", ")));
    }
    json.push_str("\n}\n}\n");

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../analysis/data_cache/_sublevel_confirm_OKLO.json");
    fs::write(&out, &json).unwrap_or_else(|e| panic!("写 {out:?}: {e}"));
    println!("[OKLO] 已落盘 {out:?}");
}
