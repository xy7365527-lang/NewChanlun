//! ★issue #841 探针：两张按级别分钱的权重表是不是同一根轴（map #787，喂 #839）。
//!
//! **只读、零生产行为改动**：整模块 `#[cfg(test)]`（见 `backtest/mod.rs` 声明），不进 release
//! 产物；三支探针全部消费生产函数（`extract_carrier_forest` / `leg_target` /
//! `run_theta_v0_pi_overlay` / `LevelLedgerMirror`），不另起平行管线。
//!
//! ## 三支探针
//!
//! - **A 结构口径**（`issue841_axis_structure`）：逐 bar 用生产增量分类器建塔 → 生产
//!   `coverage::extract_carrier_forest`（= `interp.rs:838-842` 声明的生产元素宇宙 K_i）→
//!   对每个元素算 `(root_level, depth) → 绝对级别 e.level`。`depth` 用与
//!   `coverage/leg.rs:215-232 element_depth` **逐字同构**的 parent 链游走复刻，并逐元素与生产
//!   `coverage::leg_target(..).units` 对拍（default `VoiceConfig` 下 `w_dir≡1`、`w_grade≡1` ⟹
//!   `units == base_units × depth_weight(depth)`，见 `coverage/leg.rs:159`）——对拍不过即 panic。
//! - **B 资金口径**（`issue841_axis_funded`）：跑生产 `run_theta_v0_pi_overlay`，从
//!   `LevelLedgerMirror`（只读旁路，消费与 `OverlayState` 同一份 `sep_legs`）取出**真正拿到钱的**
//!   声部（`q>0` 才进 `build_level_targets`），用 `parent_id` 链算 depth，交叉表
//!   `(root_level, depth) × 绝对级别`。
//! - **C 反事实**（`issue841_level_cap_counterfactual`）：`enforce_level_cap` off/on 两臂同窗对比。
//!
//! 跑法（`#[ignore]`，需 `analysis/data_cache/btc_1m_full.json`）：
//! ```text
//! cd rust && cargo test --release --lib -- --ignored --nocapture issue841_
//! ```
//! 窗口 bar 数由 env `ISSUE841_BARS` 控制（默认 20000，与 #755 同口径）；探针 A 的 bar 抽样步长
//! 由 `ISSUE841_STRIDE` 控制（默认 1 = 不抽样）。

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::super::classifier::recursive_tower::ElementId;
use super::super::config::{ThetaConfig, VoiceConfig};
use super::super::strategy::coverage::{extract_carrier_forest, leg_target, CoverageElement};
use super::super::strategy::voice::depth_weight;
use super::data;
use super::runner::run_theta_v0_pi_overlay;

/// 窗口 bar 数（env `ISSUE841_BARS`，默认 20000 = #755 同口径）。
fn win_bars() -> usize {
    std::env::var("ISSUE841_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000)
}

fn load_window() -> data::Dataset {
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg)
        .expect("BTC 数据（analysis/data_cache/btc_1m_full.json）");
    let n = ds.bars.len().min(win_bars());
    eprintln!(
        "[841] BTC 全集 bars={}，本次窗口 bars={n}（截断照实声明）",
        ds.bars.len()
    );
    data::Dataset {
        symbol: ds.symbol.clone(),
        bars: ds.bars[..n].to_vec(),
        dates: ds.dates[..n].to_vec(),
        bar_seconds: ds.bar_seconds,
    }
}

/// `coverage/leg.rs:215-232 element_depth` 的逐字同构复刻（沿 `parent` 链上溯，根=0）。
/// 生产函数是 `pub(super)`（模块外不可见）⟹ 本复刻由下方 `leg_target` 对拍承保等价。
fn walk_depth(elements: &[CoverageElement], mut idx: usize) -> u32 {
    let fuel = elements.len();
    let mut depth = 0u32;
    while let Some(p) = elements[idx].parent {
        depth += 1;
        assert!(
            depth as usize <= fuel,
            "parent 图含环（同 leg.rs:221 硬门）"
        );
        idx = p;
    }
    depth
}

/// 链顶元素的级别（root_level）。
fn walk_root_level(elements: &[CoverageElement], mut idx: usize) -> u32 {
    let fuel = elements.len();
    let mut steps = 0usize;
    while let Some(p) = elements[idx].parent {
        steps += 1;
        assert!(steps <= fuel, "parent 图含环");
        idx = p;
    }
    elements[idx].level
}

// ════════════════════════════════════════════════════════════════════════════
//  探针 A：结构口径 —— (root_level, depth) → 绝对级别 的实际映射
// ════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "issue #841 探针A：cargo test --release --lib -- --ignored --nocapture issue841_axis_structure"]
fn issue841_axis_structure() {
    let cfg = ThetaConfig::default();
    let ds = load_window();
    let stride: usize = std::env::var("ISSUE841_STRIDE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let mut incr = super::incremental::IncrementalClassifier::new(&ds.bars, &cfg);
    // (root_level, depth) → 该路径命中的绝对级别多重集
    let mut path_to_levels: BTreeMap<(u32, u32), BTreeMap<u32, u64>> = BTreeMap::new();
    // 绝对级别 → 供给它的 (root_level, depth) 路径集合（含元素计数）
    let mut level_to_paths: BTreeMap<u32, BTreeMap<(u32, u32), u64>> = BTreeMap::new();
    // 同上，但只统计**拿到钱的**深度（depth_weight>0 ⟹ default 表下 depth ∈ {0,1,2}）
    let mut level_to_paths_funded: BTreeMap<u32, BTreeMap<(u32, u32), u64>> = BTreeMap::new();
    let mut n_elem: u64 = 0;
    let mut n_mismatch: u64 = 0; // level != root_level - depth 的元素数
    let mut n_bars_sampled: u64 = 0;
    let mut legtarget_checked: u64 = 0;
    // ★同时性（严格「同一 bar」口径）：本 bar 内是否存在某绝对级别被 ≥2 个不同 depth 供给。
    let mut n_bars_multi_depth: u64 = 0;
    let mut bars_multi_levels: BTreeMap<u32, u64> = BTreeMap::new();

    for i in (0..ds.bars.len()).step_by(stride.max(1)) {
        let (_cls, tower) = incr.classify_at(i);
        let forest = extract_carrier_forest(&tower);
        if forest.is_empty() {
            continue;
        }
        n_bars_sampled += 1;
        let mut bar_level_depths: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for idx in 0..forest.len() {
            let d = walk_depth(&forest, idx);
            let rl = walk_root_level(&forest, idx);
            let lv = forest[idx].level;
            n_elem += 1;
            if rl as i64 - d as i64 != lv as i64 {
                n_mismatch += 1;
            }
            *path_to_levels
                .entry((rl, d))
                .or_default()
                .entry(lv)
                .or_insert(0) += 1;
            *level_to_paths
                .entry(lv)
                .or_default()
                .entry((rl, d))
                .or_insert(0) += 1;
            if depth_weight(d, &cfg.voice) > 0.0 {
                *level_to_paths_funded
                    .entry(lv)
                    .or_default()
                    .entry((rl, d))
                    .or_insert(0) += 1;
                bar_level_depths.entry(lv).or_default().insert(d);
            }
        }
        let multi: Vec<u32> = bar_level_depths
            .iter()
            .filter(|(_, ds)| ds.len() > 1)
            .map(|(l, _)| *l)
            .collect();
        if !multi.is_empty() {
            n_bars_multi_depth += 1;
            for l in multi {
                *bars_multi_levels.entry(l).or_insert(0) += 1;
            }
        }
        // ★与生产 sizing 对拍（每 bar 抽查前 64 个元素，控成本）：default VoiceConfig 下
        //   leg_target.units == base_units × depth_weight(depth)（leg.rs:159，w_dir≡1、w_grade≡1）。
        let vc = VoiceConfig::default();
        for idx in 0..forest.len().min(64) {
            let d = walk_depth(&forest, idx);
            let got = leg_target(&forest, idx, 1000.0, &vc).units;
            let want = 1000.0 * depth_weight(d, &vc);
            assert!(
                (got - want).abs() < 1e-9,
                "探针 depth 与生产 leg_target 不对拍：bar={i} idx={idx} depth={d} got={got} want={want}"
            );
            legtarget_checked += 1;
        }
    }

    // ── 报告 ──
    let mut s = String::new();
    s.push_str(&format!(
        "# #841 探针A 结构口径（BTC 前 {} bar，stride={stride}）\n\n\
         采样 bar 数（森林非空）={n_bars_sampled}；元素观测数（分母）={n_elem}；\
         leg_target 对拍次数={legtarget_checked}\n\n\
         **level ≠ root_level − depth 的元素数 = {n_mismatch}**（占 {:.4}%）\n\n",
        ds.bars.len(),
        if n_elem > 0 {
            n_mismatch as f64 * 100.0 / n_elem as f64
        } else {
            f64::NAN
        }
    ));

    s.push_str(
        "## (root_level, depth) → 绝对级别（每路径命中的级别分布）\n\n\
                | root_level | depth | w_depth | 绝对级别→元素数 |\n|---|---|---|---|\n",
    );
    for (&(rl, d), lvls) in &path_to_levels {
        let lv_str: Vec<String> = lvls.iter().map(|(l, c)| format!("L{l}:{c}")).collect();
        s.push_str(&format!(
            "| {rl} | {d} | {:.2} | {} |\n",
            depth_weight(d, &cfg.voice),
            lv_str.join(" ")
        ));
    }

    s.push_str("\n## 绝对级别 ← 哪些 (root_level, depth) 路径供给（★Q2 主表）\n\n\
                | 绝对级别 | 全部路径数 | 路径明细(rl,d)=元素数 | **拿到钱的路径数**(w>0) | 拿到钱的 depth 集合 |\n\
                |---|---|---|---|---|\n");
    for (lv, paths) in &level_to_paths {
        let all: Vec<String> = paths
            .iter()
            .map(|((rl, d), c)| format!("({rl},{d})={c}"))
            .collect();
        let funded = level_to_paths_funded.get(lv);
        let n_funded = funded.map_or(0, |m| m.len());
        let depths: BTreeSet<u32> = funded
            .map(|m| m.keys().map(|(_, d)| *d).collect())
            .unwrap_or_default();
        s.push_str(&format!(
            "| L{lv} | {} | {} | **{n_funded}** | {:?} |\n",
            paths.len(),
            all.join(" "),
            depths
        ));
    }

    let multi_funded: Vec<u32> = level_to_paths_funded
        .iter()
        .filter(|(_, m)| m.keys().map(|(_, d)| *d).collect::<BTreeSet<u32>>().len() > 1)
        .map(|(l, _)| *l)
        .collect();
    s.push_str(&format!(
        "\n**被多个不同 depth（⟹ 多个不同 w_depth）供给的绝对级别（全窗聚合）**：{multi_funded:?}\n\n\
         **严格同 bar 口径**：{n_bars_multi_depth}/{n_bars_sampled} 个采样 bar 上，\
         至少有一个绝对级别在**同一 bar 内**被 ≥2 个不同 depth 同时供给（{:.2}%）；\
         逐级别的同 bar 命中 bar 数 = {bars_multi_levels:?}\n",
        if n_bars_sampled > 0 {
            n_bars_multi_depth as f64 * 100.0 / n_bars_sampled as f64
        } else {
            f64::NAN
        }
    ));

    std::fs::write("/tmp/issue841_probeA_structure.md", &s).ok();
    eprintln!("{s}");
    eprintln!("[841] 探针A 落盘 /tmp/issue841_probeA_structure.md");
}

// ════════════════════════════════════════════════════════════════════════════
//  探针 B：资金口径 —— 真正拿到钱的声部按 (root_level, depth) × 绝对级别 交叉
// ════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "issue #841 探针B：cargo test --release --lib -- --ignored --nocapture issue841_axis_funded"]
fn issue841_axis_funded() {
    let cfg = ThetaConfig::default();
    let ds = load_window();
    let years = ds.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let nav = ds
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * cfg.tick.tick_size)
        .unwrap_or(1.0)
        * 1000.0;

    let r = run_theta_v0_pi_overlay(&ds, &cfg, years, nav);
    let ll = &r.level_ledger;

    // 全部曾拿到钱的声部（closed + 窗末在飞）：id → parent_id。
    let last_bar = ds.bars.len().saturating_sub(1);
    let mut parent_of: HashMap<ElementId, Option<ElementId>> = HashMap::new();
    // (id, parent_id, entry_bar, exit_bar)
    let mut voices: Vec<(ElementId, Option<ElementId>, usize, usize)> = Vec::new();
    for lvl in 0..=12u32 {
        for cv in ll.closed_voices(lvl) {
            parent_of.insert(cv.id, cv.parent_id);
            voices.push((cv.id, cv.parent_id, cv.entry_bar, cv.exit_bar));
        }
    }
    for lvl in ll.levels() {
        for vb in ll.active_voices(lvl) {
            parent_of.insert(vb.id, vb.parent_id);
            voices.push((vb.id, vb.parent_id, vb.entry_bar, last_bar));
        }
    }

    let mut n_resolved: u64 = 0;
    let mut n_unresolved: u64 = 0; // parent_id=Some 但父从未拿到钱 ⟹ 链断，depth 不可判
    let mut n_mismatch: u64 = 0;
    let mut level_to_paths: BTreeMap<u32, BTreeMap<(u32, u32), u64>> = BTreeMap::new();
    let mut depth_hist: BTreeMap<u32, u64> = BTreeMap::new();

    // 解出 depth 的声部（含持仓区间），供「同时持有」重叠检查。
    let mut resolved: Vec<(u32, u32, u32, usize, usize)> = Vec::new(); // (abs_level, root_level, depth, entry, exit)
    for (id, pid, entry, exit) in &voices {
        // 走链：只要父在 parent_of 里就能继续；否则链断。
        let mut cur = *id;
        let mut curp = *pid;
        let mut d = 0u32;
        let mut broken = false;
        let mut steps = 0usize;
        while let Some(p) = curp {
            match parent_of.get(&p) {
                Some(&pp) => {
                    d += 1;
                    cur = p;
                    curp = pp;
                }
                None => {
                    broken = true;
                    break;
                }
            }
            steps += 1;
            assert!(steps <= voices.len() + 1, "parent_id 链含环");
        }
        if broken {
            n_unresolved += 1;
            continue;
        }
        n_resolved += 1;
        let rl = cur.level;
        *depth_hist.entry(d).or_insert(0) += 1;
        if rl as i64 - d as i64 != id.level as i64 {
            n_mismatch += 1;
        }
        *level_to_paths
            .entry(id.level)
            .or_default()
            .entry((rl, d))
            .or_insert(0) += 1;
        resolved.push((id.level, rl, d, *entry, *exit));
    }

    // ★同时持有口径：同一绝对级别、不同 depth 的两条声部持仓区间是否重叠。
    let mut overlap_pairs: BTreeMap<u32, u64> = BTreeMap::new();
    let mut overlap_depth_pairs: BTreeMap<(u32, u32, u32), u64> = BTreeMap::new();
    for a in 0..resolved.len() {
        for b in (a + 1)..resolved.len() {
            let (la, _, da, ea, xa) = resolved[a];
            let (lb, _, db, eb, xb) = resolved[b];
            if la != lb || da == db {
                continue;
            }
            if ea.max(eb) <= xa.min(xb) {
                *overlap_pairs.entry(la).or_insert(0) += 1;
                let (lo, hi) = if da < db { (da, db) } else { (db, da) };
                *overlap_depth_pairs.entry((la, lo, hi)).or_insert(0) += 1;
            }
        }
    }

    let mut s = String::new();
    s.push_str(&format!(
        "# #841 探针B 资金口径（BTC 前 {} bar，生产 run_theta_v0_pi_overlay）\n\n\
         n_orders={} 声部总数（分母）={}（closed={} active={}）\n\
         链可解 depth 的声部={n_resolved}；**链断（父从未拿到钱）不可判={n_unresolved}**\n\
         level ≠ root_level − depth 的声部数={n_mismatch}\n\
         depth 直方图={depth_hist:?}\n\n",
        ds.bars.len(),
        r.net_result.n_orders,
        voices.len(),
        ll.n_closed(),
        ll.n_active(),
    ));
    s.push_str(
        "| 绝对级别 | 供给路径 (root_level,depth)=声部数 | 不同 depth 数 |\n|---|---|---|\n",
    );
    for (lv, paths) in &level_to_paths {
        let all: Vec<String> = paths
            .iter()
            .map(|((rl, d), c)| format!("({rl},{d})={c}"))
            .collect();
        let nd = paths
            .keys()
            .map(|(_, d)| *d)
            .collect::<BTreeSet<u32>>()
            .len();
        s.push_str(&format!("| L{lv} | {} | {nd} |\n", all.join(" ")));
    }
    s.push_str(&format!(
        "\n**同时持有口径**（同一绝对级别、不同 depth、持仓区间重叠的声部对数）：{overlap_pairs:?}\n\
         逐 (级别, depth_a, depth_b) 明细：{overlap_depth_pairs:?}\n"
    ));
    std::fs::write("/tmp/issue841_probeB_funded.md", &s).ok();
    eprintln!("{s}");
    eprintln!("[841] 探针B 落盘 /tmp/issue841_probeB_funded.md");
}

// ════════════════════════════════════════════════════════════════════════════
//  探针 C：w_ℓ 反事实（enforce_level_cap off/on）
// ════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "issue #841 探针C：cargo test --release --lib -- --ignored --nocapture issue841_level_cap_counterfactual"]
fn issue841_level_cap_counterfactual() {
    let ds = load_window();
    let plain = ThetaConfig::default();
    let years = ds.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let nav = ds
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * plain.tick.tick_size)
        .unwrap_or(1.0)
        * 1000.0;

    let run = |cfg: &ThetaConfig| {
        let r = run_theta_v0_pi_overlay(&ds, cfg, years, nav);
        let eq = r
            .net_result
            .equity_curve
            .last()
            .copied()
            .unwrap_or(f64::NAN);
        let pnl: f64 = r.net_result.trade_pnls.iter().sum();
        (
            r.net_result.n_orders,
            r.net_result.trade_pnls.clone(),
            pnl,
            eq,
        )
    };

    let cfg_off = ThetaConfig::default();
    assert!(!cfg_off.risk.enforce_level_cap, "default 必须 off");
    let (n_off, pnls_off, sum_off, eq_off) = run(&cfg_off);

    let mut s = String::new();
    s.push_str(&format!(
        "# #841 探针C w_ℓ 反事实（BTC 前 {} bar）\n\n\
         | 臂 | level_weights | Σw | n_orders | trade 笔数 | 已实现 PnL 和 | 权益终值 | 与 off 逐位相同？ |\n\
         |---|---|---|---|---|---|---|---|\n\
         | off（default） | []（空表） | 0 | {n_off} | {} | {sum_off:.6} | {eq_off:.6} | — |\n",
        ds.bars.len(),
        pnls_off.len()
    ));

    // 三套 on 配置：#755 偏紧 0.01×6、#310/m8 既有 0.05×6、宽松 1/6×6（Σ=1）。
    let sixth = 1.0f64 / 6.0;
    let arms: Vec<(&str, Vec<f64>)> = vec![
        ("#755 紧", vec![0.01; 6]),
        ("#310/m8", vec![0.05; 6]),
        ("Σ=1 宽", vec![sixth; 6]),
    ];
    let mut all_identical = true;
    for (tag, w) in arms {
        let mut cfg_on = ThetaConfig::default();
        cfg_on.risk.enforce_level_cap = true;
        cfg_on.risk.level_weights = w.clone();
        assert!(
            super::super::strategy::level_risk::level_weights_sum_le_one(&cfg_on.risk),
            "Σw_ℓ≤1"
        );
        let (n_on, pnls_on, sum_on, eq_on) = run(&cfg_on);
        let bitwise = pnls_on.len() == pnls_off.len()
            && pnls_on
                .iter()
                .zip(pnls_off.iter())
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && n_on == n_off
            && eq_on.to_bits() == eq_off.to_bits();
        if !bitwise {
            all_identical = false;
        }
        s.push_str(&format!(
            "| on {tag} | {:?} | {:.3} | {n_on} | {} | {sum_on:.6} | {eq_on:.6} | **{}** |\n",
            w,
            w.iter().sum::<f64>(),
            pnls_on.len(),
            if bitwise {
                "是（逐位相同）"
            } else {
                "否"
            }
        ));
    }
    s.push_str(&format!(
        "\n**三套 on 配置是否全部与 off 逐位相同：{}**\n\
         （全相同 ⟹ `enforce_level_cap` 开关在当前生产路径上是死的，须明写为发现，不得当「无影响」）\n",
        if all_identical { "是" } else { "否" }
    ));
    std::fs::write("/tmp/issue841_probeC_levelcap.md", &s).ok();
    eprintln!("{s}");
    eprintln!("[841] 探针C 落盘 /tmp/issue841_probeC_levelcap.md");
}
