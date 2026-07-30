//! 盘背卖递归正则化三消融位回测（2026-06-11 任务）— 纯 Rust 链路。
//!
//! 调研报告：`analysis/consolidation_div_regularization_research.md`
//! 消融矩阵：V2r（基线）/ V2rR1（开腿次级别确认）/ V2rR2（兑现锚 ZD→ZG）/
//!           V2rR3（t6 次级别确认）/ V2rR12 / V2rR123（全开）。
//!
//! 守卫：V2r 基线 counters 与在册 enginefix JSON 硬校验（基线零侵入——
//! R 位默认关时逐计数等于在册 V2r）。
//!
//! 运行（长测试，默认 ignore）：
//!   cargo test --release consol_ablation_oklo -- --ignored --nocapture
//! 输出：analysis/data_cache/_consol_ablation_OKLO.json
#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::config::{variant, StopMode};
use super::runner::run_organic;
use super::trade_behavior::load_tape;

const FLOOR_LADDER: usize = 2; // LADDER_SEG（在册回测口径）

const VARIANTS: [&str; 6] = ["V2r", "V2rR1", "V2rR2", "V2rR3", "V2rR12", "V2rR123"];

/// 逐腿记录：开腿触发分解 × 闭腿原因 × 盈亏（open_log ⋈ close_log ⋈ LegTrace）。
struct LegRow {
    trigger: u8, // bit0=Sell1 bit1=盘背
    reason: u8,  // 5/6/7/8/9；0=强平（master 出场/eod，close_log 无行）
    profit: f64,
    held: i64,
}

struct TriggerStats {
    n: u64,
    wins: u64,
    gross_win: f64,
    gross_loss: f64,
    net: f64,
}

impl TriggerStats {
    fn new() -> Self {
        TriggerStats {
            n: 0,
            wins: 0,
            gross_win: 0.0,
            gross_loss: 0.0,
            net: 0.0,
        }
    }
    fn add(&mut self, p: f64) {
        self.n += 1;
        self.net += p;
        if p > 0.0 {
            self.wins += 1;
            self.gross_win += p;
        } else {
            self.gross_loss += -p;
        }
    }
    /// payoff = 平均盈利 / 平均亏损。
    fn payoff(&self) -> Option<f64> {
        let losses = self.n - self.wins;
        if self.wins == 0 || losses == 0 {
            return None;
        }
        let aw = self.gross_win / self.wins as f64;
        let al = self.gross_loss / losses as f64;
        (al > 0.0).then(|| aw / al)
    }
    fn win_rate(&self) -> Option<f64> {
        (self.n > 0).then(|| self.wins as f64 / self.n as f64 * 100.0)
    }
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map_or("null".into(), |x| format!("{x:.3}"))
}

#[test]
#[ignore]
fn consol_ablation_oklo() {
    let tape = load_tape("OKLO");
    println!("[OKLO] 磁带 {} bars 加载完成", tape.bars.len());
    let mut json = String::with_capacity(1 << 16);
    json.push_str("{\n\"symbol\": \"OKLO\",\n\"floor_ladder\": 2,\n\"variants\": {");

    for (vi, name) in VARIANTS.iter().enumerate() {
        let cfg = variant(name).unwrap_or_else(|| panic!("{name} 未注册"));
        let res = run_organic(&tape, FLOOR_LADDER, &cfg, StopMode::None, true)
            .unwrap_or_else(|e| panic!("run_organic {name}: {e}"));
        let c = &res.counters;

        // ── V2r 基线零侵入守卫（在册 enginefix 期望，trade_behavior 同源）──
        if *name == "V2r" {
            assert_eq!(c.n_rev_open, 185, "V2r 基线 n_rev_open 漂移——R 位侵入基线");
            assert_eq!(c.n_rev_open_osc, 185);
            assert_eq!(c.n_rev_close_t5, 0);
            assert_eq!(c.n_rev_close_t6, 116);
            assert_eq!(c.n_rev_close_t7, 29);
            assert_eq!(c.n_rev_zd_close, 38);
            assert_eq!(c.n_rev_mismatch_holds, 18);
            assert_eq!(c.rev_osc_pairs, 183);
            assert_eq!(c.n_rev_r1_rejects, 0);
            assert_eq!(c.n_rev_zg_close, 0);
            assert_eq!(c.n_rev_r3_holds, 0);
            println!("[V2r] 基线零侵入守卫 PASS（在册逐计数相等）");
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

        // ── 逐腿 join：open_log(trigger) ⋈ close_log(reason) ⋈ LegTrace(profit) ──
        let trig: HashMap<(u8, i64), u8> = c
            .rev_open_log
            .iter()
            .map(|o| ((o.ladder, o.bar), o.trigger))
            .collect();
        let reason: HashMap<(u8, i64), u8> = c
            .rev_close_log
            .iter()
            .map(|cl| ((cl.ladder, cl.open_bar), cl.reason))
            .collect();
        let mut legs: Vec<LegRow> = Vec::new();
        for td in res.diag.as_ref().expect("diag=true") {
            for lt in &td.diffs {
                if lt.slot.leg_kind() != "rev" {
                    continue;
                }
                let key = (lt.slot.ladder as u8, lt.sell_bar);
                let Some(&t) = trig.get(&key) else { continue }; // legacy/master rev 腿无 log
                legs.push(LegRow {
                    trigger: t,
                    reason: reason.get(&key).copied().unwrap_or(0),
                    profit: lt.profit,
                    held: lt.buy_bar - lt.sell_bar,
                });
            }
        }

        // 触发类分解（盘背腿 = bit1 置位；纯 Sell1 = bit0 only）
        let mut by_trig: HashMap<&str, TriggerStats> = HashMap::new();
        let mut by_reason: HashMap<u8, TriggerStats> = HashMap::new();
        let mut consol_by_reason: HashMap<u8, TriggerStats> = HashMap::new();
        for l in &legs {
            let tkey = match l.trigger {
                1 => "sell1",
                2 => "consol",
                3 => "both",
                _ => "other",
            };
            by_trig
                .entry(tkey)
                .or_insert_with(TriggerStats::new)
                .add(l.profit);
            by_reason
                .entry(l.reason)
                .or_insert_with(TriggerStats::new)
                .add(l.profit);
            if l.trigger & 2 != 0 {
                consol_by_reason
                    .entry(l.reason)
                    .or_insert_with(TriggerStats::new)
                    .add(l.profit);
            }
        }

        let rev_net: f64 = legs.iter().map(|l| l.profit).sum();
        let rev_wins = legs.iter().filter(|l| l.profit > 0.0).count();
        let avg_held = if legs.is_empty() {
            0.0
        } else {
            legs.iter().map(|l| l.held as f64).sum::<f64>() / legs.len() as f64
        };

        println!(
            "[{name:8}] 复利={compound:+9.2}% trades={} 胜率={:.1}% maxDD={max_dd:.1}% | \
             rev开={}(R1拒={}) 腿={} rev胜率={} rev净={rev_net:+.0} | \
             t5={} t6={}(R3抑={}) t7={} zd={} zg={}",
            res.trades.len(),
            if res.trades.is_empty() {
                0.0
            } else {
                t_wins as f64 / res.trades.len() as f64 * 100.0
            },
            c.n_rev_open,
            c.n_rev_r1_rejects,
            legs.len(),
            fmt_opt(if legs.is_empty() {
                None
            } else {
                Some(rev_wins as f64 / legs.len() as f64 * 100.0)
            }),
            c.n_rev_close_t5,
            c.n_rev_close_t6,
            c.n_rev_r3_holds,
            c.n_rev_close_t7,
            c.n_rev_zd_close,
            c.n_rev_zg_close,
        );
        for (tkey, st) in [
            ("sell1", &by_trig),
            ("consol", &by_trig),
            ("both", &by_trig),
        ]
        .iter()
        .filter_map(|(k, m)| m.get(*k).map(|s| (*k, s)))
        {
            println!(
                "    trigger={tkey:6} n={:3} 胜率={} payoff={} 净={:+.0}",
                st.n,
                fmt_opt(st.win_rate()),
                fmt_opt(st.payoff()),
                st.net
            );
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
            "\"counters\": {{\"n_rev_attempts\": {}, \"n_rev_open\": {}, \
             \"n_rev_open_osc\": {}, \"n_rev_r1_rejects\": {}, \"n_rev_depth_rejects\": {}, \
             \"n_rev_nocenter_rejects\": {}, \"n_rev_close_t5\": {}, \"n_rev_close_t6\": {}, \
             \"n_rev_close_t7\": {}, \"n_rev_zd_close\": {}, \"n_rev_zg_close\": {}, \
             \"n_rev_r3_holds\": {}, \"n_rev_mismatch_holds\": {}, \"rev_osc_pairs\": {}, \
             \"rev_osc_wins\": {}, \"rev_osc_cash\": {:.2}}}, ",
            c.n_rev_attempts,
            c.n_rev_open,
            c.n_rev_open_osc,
            c.n_rev_r1_rejects,
            c.n_rev_depth_rejects,
            c.n_rev_nocenter_rejects,
            c.n_rev_close_t5,
            c.n_rev_close_t6,
            c.n_rev_close_t7,
            c.n_rev_zd_close,
            c.n_rev_zg_close,
            c.n_rev_r3_holds,
            c.n_rev_mismatch_holds,
            c.rev_osc_pairs,
            c.rev_osc_wins,
            c.rev_osc_cash
        ));
        json.push_str(&format!(
            "\"rev_legs\": {{\"n\": {}, \"wins\": {rev_wins}, \"net\": {rev_net:.2}, \
             \"avg_held_bars\": {avg_held:.1}}}, ",
            legs.len()
        ));
        let dump_map = |m: &HashMap<u8, TriggerStats>| -> String {
            let mut keys: Vec<u8> = m.keys().copied().collect();
            keys.sort();
            let items: Vec<String> = keys
                .iter()
                .map(|k| {
                    let s = &m[k];
                    format!(
                        "\"{k}\": {{\"n\": {}, \"wins\": {}, \"net\": {:.2}, \
                         \"win_rate\": {}, \"payoff\": {}}}",
                        s.n,
                        s.wins,
                        s.net,
                        fmt_opt(s.win_rate()),
                        fmt_opt(s.payoff())
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        };
        let dump_trig = |m: &HashMap<&str, TriggerStats>| -> String {
            let mut keys: Vec<&str> = m.keys().copied().collect();
            keys.sort();
            let items: Vec<String> = keys
                .iter()
                .map(|k| {
                    let s = &m[k];
                    format!(
                        "\"{k}\": {{\"n\": {}, \"wins\": {}, \"net\": {:.2}, \
                         \"win_rate\": {}, \"payoff\": {}}}",
                        s.n,
                        s.wins,
                        s.net,
                        fmt_opt(s.win_rate()),
                        fmt_opt(s.payoff())
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        };
        json.push_str(&format!("\"by_trigger\": {}, ", dump_trig(&by_trig)));
        json.push_str(&format!("\"by_close_reason\": {}, ", dump_map(&by_reason)));
        json.push_str(&format!(
            "\"consol_by_close_reason\": {}}}",
            dump_map(&consol_by_reason)
        ));
    }
    json.push_str("\n}\n}\n");

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../analysis/data_cache/_consol_ablation_OKLO.json");
    fs::write(&out, &json).unwrap_or_else(|e| panic!("写 {out:?}: {e}"));
    println!("[OKLO] 已落盘 {out:?}");
}
