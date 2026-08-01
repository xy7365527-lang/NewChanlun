//! ★issue #837 探针：跨级别反向动作的真实频次（map #787，喂 #834）。
//!
//! **只读、零生产行为改动**：整模块 `#[cfg(test)]`（见 `backtest/mod.rs` 声明），不进 release
//! 产物；数据全部来自生产跑批 [`run_theta_v0_pi_overlay`] 的产出（`OverlayRunResult.level_ledger`
//! = LEE M1 只读旁路镜像，#644）与生产增量分类器 [`IncrementalClassifier`]，不另起平行管线。
//!
//! 三种「相反动作」定义（票面 D1/D2/D3，逐种给数）：
//!
//! - **D1 同 bar 反向信号**：同一 bar 上级别 i 产出买入类 BSP、级别 j 产出卖出类 BSP（i≠j）。
//!   分母 = 有新确认 BSP 的 bar 数（`newly_confirmed_step` 的 append-only diff 非空的 bar）。
//! - **D2 同 bar 反向账户动作**：同一 bar 上级别 i 的级别账本发生净买、级别 j 净卖。事件源 =
//!   `LevelLedgerMirror` 逐声部生命周期（开仓/平仓）——**已知欠计**：resize（同声部加/减仓）
//!   不落 `ClosedVoice`，本口径只覆盖开/平两类事件，见报告 090 段。
//! - **D3 持仓期方向对冲**：级别 i 的多头声部与级别 j 的空头声部持仓区间重叠（`N=Q−H` 抵消
//!   构型）。区间来自 `ClosedVoice{entry_bar,exit_bar}` + 窗末仍在飞的 `VoiceBook`（censored）。
//!
//! **H/Q 口径诚实声明（090）**：`ClosedVoice` 不落盘 `q_v`（手数），故本探针的 H/Q **是声部
//! 计数口径**（同 bar 存活的空头声部数 / 多头声部数），不是手数口径。手数口径需在
//! `LevelLedgerMirror` 里加逐 bar 采集器 = 生产改动，超出本票「零生产行为改动」约束 ⟹ 未测。
//!
//! 跑法（`#[ignore]`，需 `analysis/data_cache/btc_1m_full.json`）：
//! ```text
//! cd rust && cargo test --release --lib -- --ignored --nocapture issue837_cross_level_opposing_actions
//! ```
//! 窗口由 env `ISSUE837_WINDOWS`（逗号分隔 tag）过滤；默认跑 p3fold。

use super::super::config::ThetaConfig;
use super::super::strategy::coverage::Vertical;
use super::super::strategy::voice::VoiceSide;
use super::data;
use super::runner::run_theta_v0_pi_overlay;

/// 一条声部的持仓区间（来自生产 `LevelLedgerMirror` 终态：已离场 + 窗末在飞）。
#[derive(Debug, Clone, Copy)]
struct VoiceSpan {
    level: u32,
    side: VoiceSide,
    role_v: Vertical,
    entry_bar: usize,
    /// 已离场 = `exit_bar`；窗末在飞 = 窗口末 bar（`censored=true`）。
    exit_bar: usize,
    censored: bool,
    pnl_v: f64,
}

fn side_sign(s: VoiceSide) -> i32 {
    match s {
        VoiceSide::Long => 1,
        VoiceSide::Short => -1,
        VoiceSide::Flat => 0,
    }
}

/// 从生产终态镜像抽出全部声部区间（**只读**：`closed_voices` / `active_voices` 均是 pub 只读 API）。
fn collect_spans(
    ll: &super::super::strategy::level_ledger::LevelLedgerMirror,
    last_bar: usize,
) -> Vec<VoiceSpan> {
    let mut out = Vec::new();
    let levels: Vec<u32> = ll.levels().collect();
    // 已离场声部按级别取（`closed` 的键集可能比 `books` 的键集大 ⟹ 级别集合取并）。
    for lvl in 0..=8u32 {
        for cv in ll.closed_voices(lvl) {
            out.push(VoiceSpan {
                level: cv.id.level,
                side: cv.side,
                role_v: cv.role_v,
                entry_bar: cv.entry_bar,
                exit_bar: cv.exit_bar,
                censored: false,
                pnl_v: cv.pnl_v,
            });
        }
    }
    for lvl in levels {
        for vb in ll.active_voices(lvl) {
            out.push(VoiceSpan {
                level: vb.id.level,
                side: vb.side,
                role_v: vb.role_v,
                entry_bar: vb.entry_bar,
                exit_bar: last_bar,
                censored: true,
                pnl_v: vb.pnl_v,
            });
        }
    }
    out
}

/// D1 读数。
#[derive(Debug, Default, Clone, Copy)]
struct D1Stats {
    n_bars: usize,
    /// 有新确认 BSP 的 bar 数（分母）。
    n_bars_with_bsp: usize,
    /// 新确认 BSP 覆盖 ≥2 个级别的 bar 数。
    n_bars_multilevel: usize,
    /// 同 bar 存在「级别 i 买 ∧ 级别 j 卖，i≠j」的 bar 数（分子）。
    n_bars_cross_level_opposing: usize,
    /// 同 bar 同一级别内即有买有卖的 bar 数（对照：同级冲突，非跨级）。
    n_bars_same_level_opposing: usize,
    /// 新确认 BSP 总数。
    n_bsp_total: usize,
    /// 生产 Γ 候选总数（`assemble_gamma_with_tower`，与 fill loop 同一入参 `classification_step`+tower）。
    n_gamma: usize,
    /// 其中 V=ReverseOpen 的候选数（= 反父级方向的对冲腿候选，跨级反向动作的**候选面**）。
    n_gamma_reverse_open: usize,
    /// 其中 V=FollowParent / Ambient 的候选数（对照）。
    n_gamma_follow_parent: usize,
    n_gamma_ambient: usize,
}

/// D2 读数。
#[derive(Debug, Default, Clone, Copy)]
struct D2Stats {
    /// 有账户动作（开/平）的 bar 数（分母）。
    n_bars_with_action: usize,
    /// 动作覆盖 ≥2 个级别的 bar 数。
    n_bars_multilevel: usize,
    /// 同 bar 级别 i 净买、级别 j 净卖（i≠j）的 bar 数（分子）。
    n_bars_cross_level_opposing: usize,
    /// 账户动作事件总数（开+平）。
    n_actions: usize,
}

/// D3 读数。
#[derive(Debug, Default, Clone)]
struct D3Stats {
    /// 声部总数（分母之一）。
    n_voices: usize,
    /// 跨级反向重叠的声部对数（分子）。
    n_pairs: usize,
    /// 至少参与过一次跨级反向重叠的声部数。
    n_voices_involved: usize,
    /// 至少一次跨级反向重叠在场的 bar 数（去重，分母 = 窗口 bar 数）。
    n_bars_hedged: usize,
    /// 重叠时长（bar）分布：各对的重叠长度。
    overlap_lens: Vec<usize>,
    /// 参与声部的 `pnl_v` 合计（净额收益合计，价格 PnL 口径，不含费）。
    involved_pnl: f64,
    /// 逐 bar H/Q（**声部计数口径**）：仅统计有跨级反向重叠的 bar。
    hq_ratios: Vec<f64>,
    /// 按级别对拆分：(lo, hi) → 对数。
    by_level_pair: std::collections::BTreeMap<(u32, u32), usize>,
    /// 最小可复现实例（重叠最长的前 N 条）。
    examples: Vec<(VoiceSpan, VoiceSpan, usize)>,
}

fn pct(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 * 100.0 / den as f64
    }
}

fn quantiles(mut v: Vec<f64>) -> (f64, f64, f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| -> f64 {
        let i = ((v.len() - 1) as f64 * p).round() as usize;
        v[i]
    };
    (q(0.0), q(0.25), q(0.5), q(0.75), q(1.0))
}

/// D3 计算：跨级反向持仓重叠。
fn compute_d3(spans: &[VoiceSpan], n_bars: usize) -> D3Stats {
    let mut st = D3Stats {
        n_voices: spans.len(),
        ..Default::default()
    };
    let mut involved = vec![false; spans.len()];
    // 差分数组（O(V+bars)，替代 O(pairs×len) 逐 bar 涂色）：hedged_diff 记重叠区间覆盖数。
    let nb = n_bars.max(1);
    let mut hedged_diff = vec![0i64; nb + 1];
    for a in 0..spans.len() {
        for b in (a + 1)..spans.len() {
            let (x, y) = (spans[a], spans[b]);
            if x.level == y.level {
                continue;
            }
            if side_sign(x.side) == 0 || side_sign(y.side) == 0 {
                continue;
            }
            if side_sign(x.side) == side_sign(y.side) {
                continue;
            }
            let lo = x.entry_bar.max(y.entry_bar);
            let hi = x.exit_bar.min(y.exit_bar);
            if hi < lo {
                continue;
            }
            let len = hi - lo + 1;
            st.n_pairs += 1;
            st.overlap_lens.push(len);
            involved[a] = true;
            involved[b] = true;
            let key = (x.level.min(y.level), x.level.max(y.level));
            *st.by_level_pair.entry(key).or_insert(0) += 1;
            let hi_c = hi.min(nb - 1);
            hedged_diff[lo] += 1;
            hedged_diff[hi_c + 1] -= 1;
            st.examples.push((x, y, len));
            if st.examples.len() >= 4096 {
                st.examples.sort_by_key(|(_, _, l)| std::cmp::Reverse(*l));
                st.examples.truncate(32);
            }
        }
    }
    // 前缀和还原覆盖标记。
    let mut hedged_bars = vec![false; nb];
    let mut acc = 0i64;
    for (t, item) in hedged_bars.iter_mut().enumerate() {
        acc += hedged_diff[t];
        *item = acc > 0;
    }
    st.n_bars_hedged = hedged_bars.iter().filter(|b| **b).count();
    st.n_voices_involved = involved.iter().filter(|b| **b).count();
    st.involved_pnl = spans
        .iter()
        .zip(involved.iter())
        .filter(|(_, inv)| **inv)
        .map(|(s, _)| s.pnl_v)
        .sum();
    // 逐 bar H/Q（声部计数口径）：差分数组求逐 bar 存活多/空声部数，只在有跨级反向重叠的 bar 统计。
    let mut dq = vec![0i64; nb + 1];
    let mut dh = vec![0i64; nb + 1];
    for s in spans {
        let lo = s.entry_bar.min(nb - 1);
        let hi = s.exit_bar.min(nb - 1);
        match s.side {
            VoiceSide::Long => {
                dq[lo] += 1;
                dq[hi + 1] -= 1;
            }
            VoiceSide::Short => {
                dh[lo] += 1;
                dh[hi + 1] -= 1;
            }
            VoiceSide::Flat => {}
        }
    }
    let (mut q, mut h) = (0i64, 0i64);
    for t in 0..nb {
        q += dq[t];
        h += dh[t];
        if hedged_bars[t] && q > 0 {
            st.hq_ratios.push(h as f64 / q as f64);
        }
    }
    st.examples.sort_by_key(|(_, _, l)| std::cmp::Reverse(*l));
    st.examples.truncate(8);
    st
}

/// D2 计算：同 bar 跨级反向账户动作（开/平事件口径）。
fn compute_d2(spans: &[VoiceSpan]) -> D2Stats {
    use std::collections::BTreeMap;
    // (bar, level) → 净动作符号累加（开 Long=+1 买、平 Long=−1 卖、开 Short=−1 卖、平 Short=+1 买）。
    let mut ev: BTreeMap<usize, BTreeMap<u32, i32>> = BTreeMap::new();
    let mut n_actions = 0usize;
    for s in spans {
        let sg = side_sign(s.side);
        if sg == 0 {
            continue;
        }
        *ev.entry(s.entry_bar).or_default().entry(s.level).or_insert(0) += sg;
        n_actions += 1;
        if !s.censored {
            *ev.entry(s.exit_bar).or_default().entry(s.level).or_insert(0) -= sg;
            n_actions += 1;
        }
    }
    let mut st = D2Stats {
        n_actions,
        ..Default::default()
    };
    for (_bar, per_level) in ev.iter() {
        let nz: Vec<(u32, i32)> = per_level
            .iter()
            .filter(|(_, v)| **v != 0)
            .map(|(l, v)| (*l, *v))
            .collect();
        if nz.is_empty() {
            continue;
        }
        st.n_bars_with_action += 1;
        if nz.len() >= 2 {
            st.n_bars_multilevel += 1;
        }
        let has_pos = nz.iter().any(|(_, v)| *v > 0);
        let has_neg = nz.iter().any(|(_, v)| *v < 0);
        if has_pos && has_neg {
            st.n_bars_cross_level_opposing += 1;
        }
    }
    st
}

/// D1 计算：重驱生产增量分类器，逐 bar 取 `newly_confirmed_step` 的新确认 BSP。
///
/// **对拍（issue821 做法）**：末 bar 的增量分类结果与全量 `classify(parse_layer(bars))` 逐级别
/// 中枢序列 `assert_eq!`——探针与生产 `classify()` 对拍通过后再取数。
fn compute_d1(bars: &[super::super::types::Bar], cfg: &ThetaConfig) -> D1Stats {
    use super::signal::newly_confirmed_step;
    let mut incr = super::incremental::IncrementalClassifier::new(bars, cfg);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    let mut st = D1Stats {
        n_bars: bars.len(),
        ..Default::default()
    };
    let mut last_cls = None;
    for i in 0..bars.len() {
        let (cls, tower) = incr.classify_at(i);
        let step = newly_confirmed_step(&cls, &mut seen);
        // ★候选面（生产同源）：与 fill loop 同一 `(classification_step, tower)` 入参调生产
        // `assemble_gamma_with_tower`（其 doc 声明与 `coverage_elements_and_gamma_with_tower`
        // bit-exact；fill.rs:4851 走 cached_gen 版，同一函数族同一建树）。
        for c in super::super::strategy::interp::assemble_gamma_with_tower(&step, &tower) {
            st.n_gamma += 1;
            match c.role.v {
                Vertical::ReverseOpen => st.n_gamma_reverse_open += 1,
                Vertical::FollowParent => st.n_gamma_follow_parent += 1,
                Vertical::Ambient => st.n_gamma_ambient += 1,
            }
        }
        let mut buy_levels: Vec<u32> = Vec::new();
        let mut sell_levels: Vec<u32> = Vec::new();
        let mut n_here = 0usize;
        for (lvl, ls) in step.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                n_here += 1;
                let b = &p.bits;
                if b.buy1 || b.buy2 || b.buy3 {
                    buy_levels.push(lvl as u32);
                }
                if b.sell1 || b.sell2 || b.sell3 {
                    sell_levels.push(lvl as u32);
                }
            }
        }
        st.n_bsp_total += n_here;
        if n_here > 0 {
            st.n_bars_with_bsp += 1;
            let mut lv: Vec<u32> = buy_levels.iter().chain(sell_levels.iter()).copied().collect();
            lv.sort_unstable();
            lv.dedup();
            if lv.len() >= 2 {
                st.n_bars_multilevel += 1;
            }
            let cross = buy_levels
                .iter()
                .any(|bl| sell_levels.iter().any(|sl| bl != sl));
            if cross {
                st.n_bars_cross_level_opposing += 1;
            }
            let same = buy_levels
                .iter()
                .any(|bl| sell_levels.iter().any(|sl| bl == sl));
            if same {
                st.n_bars_same_level_opposing += 1;
            }
        }
        if i + 1 == bars.len() {
            last_cls = Some(cls);
        }
    }
    // ── 对拍：末帧增量 == 全量生产 classify（逐级别中枢序列）。 ──
    if let Some(cls) = last_cls {
        let layer = super::super::parser::parse_layer(bars, cfg);
        let full = super::super::classifier::classify(&layer, cfg);
        assert_eq!(
            cls.levels.len(),
            full.levels.len(),
            "#837 对拍：增量末帧级别数 == 全量 classify 级别数"
        );
        for (lvl, (a, b)) in cls.levels.iter().zip(full.levels.iter()).enumerate() {
            assert_eq!(
                *a.centers, *b.centers,
                "#837 对拍：L{lvl} 中枢序列（增量末帧 vs 全量生产 classify）"
            );
        }
        eprintln!(
            "[837][对拍] 末帧增量分类 == 全量 classify（{} 级别中枢序列逐字段一致）",
            cls.levels.len()
        );
    }
    st
}

/// 一个窗口一条臂的跑批读数。
struct ArmOut {
    n_bars: usize,
    n_orders: usize,
    net_r: f64,
    max_dd: f64,
    n_voices: usize,
    spans: Vec<VoiceSpan>,
}

fn run_arm(ds: &data::Dataset, cfg: &ThetaConfig, nav: f64) -> ArmOut {
    let years = ds.bars.len() as f64 / (365.25 * 24.0 * 60.0);
    let r = run_theta_v0_pi_overlay(ds, cfg, years, nav);
    let last = ds.bars.len().saturating_sub(1);
    let spans = collect_spans(&r.level_ledger, last);
    ArmOut {
        n_bars: ds.bars.len(),
        n_orders: r.net_result.n_orders,
        net_r: r.net_result.r_decomp.as_ref().map_or(f64::NAN, |d| d.net_r),
        max_dd: r.net_result.metrics.max_drawdown,
        n_voices: r.level_ledger.n_active() + r.level_ledger.n_closed(),
        spans,
    }
}

/// ★#837 主探针（`#[ignore]`）。
#[test]
#[ignore = "issue #837 探针：cargo test --release --lib -- --ignored --nocapture issue837_cross_level_opposing_actions"]
fn issue837_cross_level_opposing_actions() {
    let plain = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &plain).expect("BTC 数据（analysis/data_cache/btc_1m_full.json）");
    eprintln!("[837] BTC 全集 bars={}", ds.bars.len());

    let mut wins: Vec<(String, String, String)> = match std::env::var("ISSUE837_BASE_WINDOW") {
        Ok(spec) => {
            let p: Vec<&str> = spec.split(':').collect();
            assert_eq!(p.len(), 3, "ISSUE837_BASE_WINDOW 格式 tag:start:end");
            vec![(p[0].into(), p[1].into(), p[2].into())]
        }
        Err(_) => vec![("p3fold".into(), "2023-01-01".into(), "2023-06-30".into())],
    };
    if let Ok(extra) = std::env::var("ISSUE837_EXTRA_WINDOWS") {
        // 形如 "tag:2024-01-01:2024-06-30,tag2:...".
        for spec in extra.split(',').filter(|s| !s.is_empty()) {
            let p: Vec<&str> = spec.split(':').collect();
            assert_eq!(p.len(), 3, "ISSUE837_EXTRA_WINDOWS 格式 tag:start:end");
            wins.push((p[0].into(), p[1].into(), p[2].into()));
        }
    }

    let nav_of = |d: &data::Dataset| {
        d.bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * plain.tick.tick_size)
            .unwrap_or(1.0)
            * 1000.0
    };

    let mut report = String::new();
    for (tag, lo, hi) in &wins {
        let win = ds.slice_date_window(lo, hi);
        assert!(!win.bars.is_empty(), "窗口 {tag} 为空");
        let nav = nav_of(&win);
        eprintln!("[837] 窗 {tag} {lo}..{hi} bars={}", win.bars.len());

        // ── 臂A：生产默认（disable_reverse_open=false）。 ──
        super::super::strategy::coverage::ancok_probe_reset();
        let cfg_a = ThetaConfig::default();
        let a = run_arm(&win, &cfg_a, nav);
        let ancok_a = super::super::strategy::coverage::ancok_probe_snapshot();

        // ── 臂B：反事实 disable_reverse_open=true（剔掉全部反父级对冲腿）。 ──
        super::super::strategy::coverage::ancok_probe_reset();
        let mut cfg_b = ThetaConfig::default();
        cfg_b.voice.disable_reverse_open = true;
        let b = run_arm(&win, &cfg_b, nav);
        let ancok_b = super::super::strategy::coverage::ancok_probe_snapshot();

        let d2 = compute_d2(&a.spans);
        let d3 = compute_d3(&a.spans, a.n_bars);
        let d1 = compute_d1(&win.bars, &cfg_a);

        let n_ro = a
            .spans
            .iter()
            .filter(|s| s.role_v == Vertical::ReverseOpen)
            .count();
        let n_fp = a
            .spans
            .iter()
            .filter(|s| s.role_v == Vertical::FollowParent)
            .count();
        let n_amb = a
            .spans
            .iter()
            .filter(|s| s.role_v == Vertical::Ambient)
            .count();
        let mut by_level: std::collections::BTreeMap<u32, usize> = Default::default();
        for s in &a.spans {
            *by_level.entry(s.level).or_insert(0) += 1;
        }

        let (l0, l25, l50, l75, l100) = quantiles(d3.overlap_lens.iter().map(|v| *v as f64).collect());
        let (h0, h25, h50, h75, h100) = quantiles(d3.hq_ratios.clone());
        let near_full = d3.hq_ratios.iter().filter(|r| (**r - 1.0).abs() <= 0.25).count();

        report.push_str(&format!(
            "\n## 窗 {tag}（{lo}..{hi}，bars={}）\n\n\
             ### 臂A 生产默认 vs 臂B disable_reverse_open\n\
             | 臂 | n_orders | net_r | max_dd | 声部数 |\n|---|---|---|---|---|\n\
             | A 默认 | {} | {:.2} | {:.4} | {} |\n\
             | B 剔ReverseOpen | {} | {:.2} | {:.4} | {} |\n\
             | Δ(A−B) | {} | {:.2} | {:.4} | {} |\n\n\
             AncOK 探针（臂A/臂B）：placeholder_pruned_by_ancok={}/{} \
             restore_parent_unresolved={}/{} restore_calls={}/{} \
             restore_break_registry_lost={}/{} closed_inval_pruned={}/{}\n\n\
             ### 声部结构（臂A）\n\
             总声部={} ReverseOpen={} FollowParent={} Ambient={} 按级别={:?}\n\n\
             ### D1 同 bar 反向信号\n\
             分母(有新确认BSP的bar)={} / 窗口bar={}；新确认BSP总数={}\n\
             多级别同bar={} ({:.3}%)；**跨级反向={}** ({:.3}%)；同级反向={} ({:.3}%)\n\
             生产 Γ 候选面：总={} ReverseOpen={} ({:.2}%) FollowParent={} Ambient={}；\
             实际开出的 ReverseOpen 声部={}（候选→声部转化率 {:.2}%）\n\n\
             ### D2 同 bar 反向账户动作（开/平事件口径，resize 欠计）\n\
             分母(有动作的bar)={}；动作事件数={}\n\
             多级别同bar={} ({:.3}%)；**跨级反向={}** ({:.3}%)\n\n\
             ### D3 持仓期方向对冲\n\
             声部数={} 参与对冲的声部={} ({:.2}%)；**跨级反向重叠对数={}**\n\
             有对冲在场的bar={} / {} ({:.3}%)\n\
             重叠时长(bar) min/p25/med/p75/max = {:.0}/{:.0}/{:.0}/{:.0}/{:.0}\n\
             H/Q(声部计数口径) min/p25/med/p75/max = {:.3}/{:.3}/{:.3}/{:.3}/{:.3}；\
             |H/Q−1|≤0.25 的 bar 占比 = {}/{} ({:.2}%)\n\
             参与声部 Σpnl_v = {:.2}\n\
             按级别对：{:?}\n\n\
             最长重叠实例（前 {}）：\n",
            a.n_bars,
            a.n_orders, a.net_r, a.max_dd, a.n_voices,
            b.n_orders, b.net_r, b.max_dd, b.n_voices,
            a.n_orders as i64 - b.n_orders as i64, a.net_r - b.net_r, a.max_dd - b.max_dd,
            a.n_voices as i64 - b.n_voices as i64,
            ancok_a.placeholder_pruned_by_ancok, ancok_b.placeholder_pruned_by_ancok,
            ancok_a.restore_parent_unresolved, ancok_b.restore_parent_unresolved,
            ancok_a.restore_calls, ancok_b.restore_calls,
            ancok_a.restore_break_registry_lost, ancok_b.restore_break_registry_lost,
            ancok_a.closed_inval_pruned, ancok_b.closed_inval_pruned,
            a.spans.len(), n_ro, n_fp, n_amb, by_level,
            d1.n_bars_with_bsp, d1.n_bars, d1.n_bsp_total,
            d1.n_bars_multilevel, pct(d1.n_bars_multilevel, d1.n_bars_with_bsp),
            d1.n_bars_cross_level_opposing, pct(d1.n_bars_cross_level_opposing, d1.n_bars_with_bsp),
            d1.n_bars_same_level_opposing, pct(d1.n_bars_same_level_opposing, d1.n_bars_with_bsp),
            d1.n_gamma, d1.n_gamma_reverse_open, pct(d1.n_gamma_reverse_open, d1.n_gamma.max(1)),
            d1.n_gamma_follow_parent, d1.n_gamma_ambient,
            n_ro, pct(n_ro, d1.n_gamma_reverse_open.max(1)),
            d2.n_bars_with_action, d2.n_actions,
            d2.n_bars_multilevel, pct(d2.n_bars_multilevel, d2.n_bars_with_action),
            d2.n_bars_cross_level_opposing, pct(d2.n_bars_cross_level_opposing, d2.n_bars_with_action),
            d3.n_voices, d3.n_voices_involved, pct(d3.n_voices_involved, d3.n_voices.max(1)),
            d3.n_pairs,
            d3.n_bars_hedged, a.n_bars, pct(d3.n_bars_hedged, a.n_bars),
            l0, l25, l50, l75, l100,
            h0, h25, h50, h75, h100,
            near_full, d3.hq_ratios.len(), pct(near_full, d3.hq_ratios.len().max(1)),
            d3.involved_pnl,
            d3.by_level_pair,
            d3.examples.len(),
        ));
        for (x, y, len) in &d3.examples {
            report.push_str(&format!(
                "- L{}/{:?}({:?}) bars[{}..{}]{} × L{}/{:?}({:?}) bars[{}..{}]{} → 重叠 {} bar，pnl_v {:.2}/{:.2}\n",
                x.level, x.side, x.role_v, x.entry_bar, x.exit_bar,
                if x.censored { "(在飞)" } else { "" },
                y.level, y.side, y.role_v, y.entry_bar, y.exit_bar,
                if y.censored { "(在飞)" } else { "" },
                len, x.pnl_v, y.pnl_v
            ));
        }
        eprintln!("{report}");
    }
    let path = std::env::var("ISSUE837_OUT").unwrap_or_else(|_| "/tmp/issue837_probe.md".into());
    std::fs::write(&path, &report).ok();
    eprintln!("[837] 读数落盘 {path}");
}
