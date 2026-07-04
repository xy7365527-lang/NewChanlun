//! a3 acc-highlow-power：高级别方向条件 × 低级别执行样本的条件化估计量 μ̂(z_L0 | D_hi)
//! （区间套的统计版本，谱系 694 号扬弃承接；prereg 冻结于
//! `.chanlun/review-results/highlow-a3-20260704.md` §1-§2，先于跑数落盘）。
//!
//! ## 估计量
//!
//! `μ̂(z_L0 | D_hi=d) = mean{ Y_i : level_i=0, D_hi(t_i)=d }`，Y_i 为 wverify 残差口径
//! （[`ResidualTrade::y`]），样本链与 `wverify_run::wverify_full` 完全同源：BTC wf_anchored
//! 12 窗 walk-forward OOS，逐窗 test 段 [`build_mu_from_bars`]（生产 π fill loop typed ledger）。
//!
//! ## 条件维（prereg §1.2 冻结，δ-free ⟹ 657 共线自毁免疫）
//!
//! - **D1 σ^H_tower** = `MuClass.sigma_higher`（塔上级方向态，入场时塔快照，F_t 可测——
//!   `selector::sigma_higher_at` 零前视，666/667 号）。**不进桶键**（codex-q1 G2 / 694 号注记）：
//!   作切片分层，置换在切片内进行（分层键 (ℓ,h,tb,σ_p) 恒定不变）。
//! - **D2 nest_anchor** = `MuClass.nest_depth` Some/None（区间套 N^δ 锚存在性）。π fill loop
//!   候选不过 Nest 门 ⟹ 预声明可能全 None（维退化照实报，不硬造）。
//!
//! ## 统计口径（与 wverify_full bit-same 函数链）
//!
//! Welford + [`perm_test::stratified_delta_perm_p`]（切片子集，N_PERM=200 seed=20260701）+
//! [`decontam::effective_n`] + [`decontam::classify_bucket`]（z_α=1.645, perm_α=0.05）+
//! [`perm_test::drop_top_k_mean`]（删尾 3）。beta 漂移探针（prereg §2.5）：Validated 格查同
//! (bsp,σ_p) 反 δ 格 mean 同号 ⟹ BETA-DRIFT-SUSPECT（memory oddeven / final-alpha 教训）。
//!
//! `#[ignore]`: `cargo test --release --lib theta_v0::backtest::highlow_mu::highlow_full -- --ignored --nocapture`

use super::super::config::ThetaConfig;
use super::l3_delta_r_alpha::build_mu_from_bars;
use super::mu_estimator::ResidualTrade;
use super::prereg_windows::{OOS_START, PREREG_WINDOWS};
use super::{data, decontam, perm_test};
use std::collections::BTreeMap;

/// walk-forward 窗间 time block 偏移步长——与 `wverify_run::WF_TIME_STRIDE` 同值锚定
/// （该 mod 私有不可 import，a2 工位写争用禁改；值漂移会破坏与 wverify_full 的分层可比性）。
const WF_TIME_STRIDE: u32 = 10_000;
/// 删尾赢家数（alpha检验.pdf §6，与 wverify_full TRIM_K 同值锚定）。
const TRIM_K: usize = 3;

/// BTC wf_anchored walk-forward OOS 残差聚合（与 `wverify_run::walk_forward_oos_residuals`
/// 同口径自建：逐窗 test 段独立 build_mu_from_bars，time_block_base=win.i·stride，BTC
/// symbol_index=0 无品种偏移 ⟹ 与 wverify_full 的 records bit-exact 同源）。
fn wf_oos_records(ds: &data::Dataset, cfg: &ThetaConfig) -> Vec<ResidualTrade> {
    let sw = PREREG_WINDOWS
        .iter()
        .find(|w| w.symbol == "BTC")
        .expect("BTC 在 PREREG_WINDOWS（预注册缺口，非静默兜底）");
    let mut agg: Vec<ResidualTrade> = Vec::new();
    for win in sw.wf_anchored {
        if win.test_start < OOS_START {
            continue; // IS 期窗不算样本外证据
        }
        let test_ds = ds.slice_date_window(win.test_start, win.test_end);
        if test_ds.bars.is_empty() {
            continue;
        }
        let (_est, records) = build_mu_from_bars(&test_ds.bars, cfg, win.i * WF_TIME_STRIDE);
        eprintln!(
            "[highlow-wf] win{} {}→{} bars={} residuals={}",
            win.i, win.test_start, win.test_end, test_ds.bars.len(), records.len()
        );
        agg.extend(records.iter().copied());
    }
    agg
}

/// 单格统计（prereg §2.2 报告列）。
struct CellStat {
    n: u64,
    n_eff: f64,
    mean: f64,
    lcb: f64,
    ucb: f64,
    cv: f64,
    perm_p: f64,
    trim_mean: f64,
    flipped: bool,
    state: decontam::AlphaState,
}

/// 切片（或全体）records → 逐执行桶 (ℓ,bsp,δ,σ_p) 统计（与 wverify_full 同口径：
/// Welford + 切片内分层 δ 置换 + n_eff + 删尾 + 三态）。切片先于置换固定（D 维 δ-free
/// ⟹ 层内 shuffle δ 不改切片归属，prereg §1.2 共线免疫论证）。
fn cell_stats(records: &[ResidualTrade]) -> BTreeMap<(u32, u8, i8, i8), CellStat> {
    let mut agg: BTreeMap<(u32, u8, i8, i8), (u64, f64, f64)> = BTreeMap::new();
    let mut series: BTreeMap<(u32, u8, i8, i8), Vec<f64>> = BTreeMap::new();
    for r in records {
        let key = (r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir);
        let y = r.y();
        let e = agg.entry(key).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = y - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (y - e.1);
        series.entry(key).or_default().push(y);
    }
    let pp = perm_test::stratified_delta_perm_p(records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut out = BTreeMap::new();
    for (&key, &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *pp.get(&key).unwrap_or(&1.0);
        let ys = series.get(&key).map(Vec::as_slice).unwrap_or(&[]);
        let n_eff = decontam::effective_n(ys);
        let (trim_mean, flipped) = perm_test::drop_top_k_mean(ys, TRIM_K);
        let state = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        out.insert(key, CellStat { n, n_eff, mean, lcb, ucb, cv, perm_p, trim_mean, flipped, state });
    }
    out
}

/// beta 漂移探针（prereg §2.5，memory oddeven）：`key` 为 Validated 时查同 (ℓ,bsp,σ_p)
/// 反 δ 格——两 δ mean 同号 ⟹ 残差是方向无关漂移非结构 alpha ⟹ SUSPECT。
fn beta_drift_suspect(stats: &BTreeMap<(u32, u8, i8, i8), CellStat>, key: (u32, u8, i8, i8)) -> bool {
    let (lv, bc, d, pd) = key;
    let Some(cell) = stats.get(&key) else { return false };
    match stats.get(&(lv, bc, -d, pd)) {
        Some(opp) => cell.mean.signum() == opp.mean.signum(),
        None => false, // 反 δ 格无样本——探针不可算，不标 suspect（照实由表读者判）
    }
}

/// 逐格 markdown 行 + 与基线对照（prereg §2.3）+ Validated 探针标注。
/// 返回 (rows, 非 suspect 的 Validated 格键列表——prereg §2.6 停机条款输入)。
fn render_slice(
    label: &str,
    stats: &BTreeMap<(u32, u8, i8, i8), CellStat>,
    base: &BTreeMap<(u32, u8, i8, i8), CellStat>,
) -> (String, Vec<(u32, u8, i8, i8)>) {
    let mut rows = format!(
        "\n### {label}\n\n| L | bsp | δ | σ_p | n | n_eff | mean(Y) | lcb | ucb | cv | perm_p | 删尾mean(−{TRIM_K}) | 翻转 | state | 基线state | 判定变化 | beta探针 |\n|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|\n"
    );
    let mut alerts = Vec::new();
    for (&key, c) in stats {
        let (lv, bc, d, pd) = key;
        let base_state = base
            .get(&key)
            .map(|b| format!("{:?}", b.state))
            .unwrap_or_else(|| "—".into());
        let changed = if base_state == format!("{:?}", c.state) { "同" } else { "**变**" };
        let probe = if matches!(c.state, decontam::AlphaState::Validated) {
            if beta_drift_suspect(stats, key) {
                "BETA-DRIFT-SUSPECT"
            } else {
                alerts.push(key);
                "**ALERT非suspect**"
            }
        } else {
            "—"
        };
        rows.push_str(&format!(
            "| L{lv} | {bc} | {d:+} | {pd:+} | {} | {:.2} | {:.6} | {:.6} | {:.6} | {:.3} | {:.3} | {:.6} | {} | {:?} | {base_state} | {changed} | {probe} |\n",
            c.n, c.n_eff, c.mean, c.lcb, c.ucb, c.cv, c.perm_p, c.trim_mean, c.flipped, c.state
        ));
    }
    (rows, alerts)
}

/// a3 主跑批（prereg 冻结后执行）。BTC 全历史 walk-forward OOS，L0 执行样本，
/// D1 σ^H / D2 nest_anchor 两维独立切片 + 无条件基线对照。产物 /tmp/highlow_a3.md。
#[test]
#[ignore]
fn highlow_full() {
    let cfg = ThetaConfig::default();
    let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载（btc_1m_full.json）");
    let records = wf_oos_records(&ds, &cfg);
    assert!(!records.is_empty(), "walk-forward OOS 聚合未产出残差——窗口/数据不匹配");

    // L0 执行样本（prereg §1.1）；高级别自身样本弃用于估计（694：稀疏无功效，只作条件源）。
    let l0: Vec<ResidualTrade> = records.iter().filter(|r| r.class.level == 0).copied().collect();
    let n_hi = records.len() - l0.len();
    assert!(!l0.is_empty(), "L0 执行样本空——样本链断裂");

    let base = cell_stats(&l0);
    let (base_rows, base_alerts) = render_slice("无条件基线 μ̂(z_L0)", &base, &base);

    let mut report = format!(
        "# a3 highlow μ̂(z_L0 | D_hi)（BTC wf OOS，prereg highlow-a3-20260704 冻结口径）\n\n\
         residuals 全级别={} L0={} 高级别(弃估计,只作条件源)={}\n\n## 基线\n{base_rows}",
        records.len(), l0.len(), n_hi
    );
    let mut all_alerts: Vec<String> = base_alerts.iter().map(|k| format!("base:{k:?}")).collect();

    // ── D1 σ^H_tower 切片（None 单列报数不判定，prereg §1.2）──
    let n_sig_none = l0.iter().filter(|r| r.class.sigma_higher.is_none()).count();
    report.push_str(&format!("\n## D1 σ^H_tower（sigma_higher=None 计数={n_sig_none}，不判定）\n"));
    for v in [1i8, -1, 0] {
        let slice: Vec<ResidualTrade> =
            l0.iter().filter(|r| r.class.sigma_higher == Some(v)).copied().collect();
        if slice.is_empty() {
            report.push_str(&format!("\n### σ^H={v:+}\n（空切片——照实报，无样本不判定）\n"));
            continue;
        }
        let stats = cell_stats(&slice);
        let (rows, alerts) = render_slice(&format!("σ^H={v:+}（n={}）", slice.len()), &stats, &base);
        report.push_str(&rows);
        all_alerts.extend(alerts.iter().map(|k| format!("D1(σ^H={v:+}):{k:?}")));
    }

    // ── D2 nest_anchor 切片（退化预声明：π fill loop 不过 Nest 门 ⟹ 可能全 None）──
    let anchored: Vec<ResidualTrade> =
        l0.iter().filter(|r| r.class.nest_depth.is_some()).copied().collect();
    let unanchored: Vec<ResidualTrade> =
        l0.iter().filter(|r| r.class.nest_depth.is_none()).copied().collect();
    report.push_str(&format!(
        "\n## D2 nest_anchor（anchored={} unanchored={}）\n",
        anchored.len(), unanchored.len()
    ));
    if anchored.is_empty() {
        report.push_str(
            "\n**维退化坐实**（prereg §1.2 预声明）：π fill loop 样本链 nest_depth 全 None——\
             D2 在本样本链无生产者，不判定。区间套锚条件化需 econ 门路径样本源（新 prereg）。\n",
        );
    } else {
        for (lbl, slice) in [("anchored", &anchored), ("unanchored", &unanchored)] {
            let stats = cell_stats(slice);
            let (rows, alerts) =
                render_slice(&format!("nest={lbl}（n={}）", slice.len()), &stats, &base);
            report.push_str(&rows);
            all_alerts.extend(alerts.iter().map(|k| format!("D2({lbl}):{k:?}")));
        }
    }

    // ── 停机条款（prereg §2.6）：非 suspect Validated 条件格 ⟹ 上浮信号 ──
    report.push_str(&format!(
        "\n## 停机条款检查\n\n非 BETA-DRIFT-SUSPECT 的 Validated 格：{}\n",
        if all_alerts.is_empty() { "无".into() } else { all_alerts.join("；") }
    ));
    if !all_alerts.is_empty() {
        eprintln!("HIGHLOW_ALERT（prereg §2.6 停机条款触发，须上浮 main）：{all_alerts:?}");
    }

    std::fs::write("/tmp/highlow_a3.md", &report).expect("报告落盘 /tmp/highlow_a3.md");
    eprintln!(
        "HIGHLOW_FULL residuals={} l0={} hi={} sigma_none={n_sig_none} nest_anchored={} alerts={}",
        records.len(), l0.len(), n_hi, anchored.len(), all_alerts.len()
    );
}

#[cfg(test)]
mod self_check {
    use super::super::mu_estimator::{MuClass, PositionState, ResidualTrade};
    use super::*;

    /// 合成 L0 记录（L1 管线自检专用——只验切片/探针逻辑，零 alpha 信息增量，231）。
    fn rec(delta: i8, sigma_h: i8, resid: f64, tb: u32) -> ResidualTrade {
        ResidualTrade {
            class: MuClass {
                level: 0,
                delta,
                i_class: if delta == 1 { 0b000_001 } else { 0b001_000 }, // buy1 / sell1
                parent_dir: 0,
                short_swing: false,
                position: PositionState::Root,
                horizontal: None,
                force_state: None,
                sigma_higher: Some(sigma_h),
                cand_channel: None,
                nest_depth: None,
                origin_level: None,
                risk_mode: None,
                t_stage: None,
                eta_bucket: None,
            },
            resid_base: resid,
            cost: 0.0,
            h_bucket: 0,
            time_block: tb,
            d: 1.0, // A6：合成自检 d=1（μ_R 分母占位，切片逻辑不依赖 d）
            exit_type: crate::theta_v0::strategy::interp::ExitType::Hold, // 诊断切片占位（不进桶键）
        }
    }

    /// 切片归属 δ-free + 管线跑通 + beta 探针方向性（prereg §1.2/§2.5 的可运行检查）。
    #[test]
    fn slice_and_probe_logic() {
        let mut rs: Vec<ResidualTrade> = Vec::new();
        // σ^H=+1 切片：δ=+1 残差正（y=+resid），δ=−1 残差也"赚"（y=−resid，resid<0）→ 两 δ mean 同不同号可控。
        for i in 0..30 {
            rs.push(rec(1, 1, 1.0 + (i % 3) as f64 * 0.1, i / 10));
            rs.push(rec(-1, 1, 1.0 + (i % 3) as f64 * 0.1, i / 10)); // y=−resid<0 ⟹ 两 δ 异号
        }
        for i in 0..10 {
            rs.push(rec(1, -1, -0.5, i / 5)); // σ^H=−1 切片
        }
        let plus: Vec<ResidualTrade> =
            rs.iter().filter(|r| r.class.sigma_higher == Some(1)).copied().collect();
        let minus: Vec<ResidualTrade> =
            rs.iter().filter(|r| r.class.sigma_higher == Some(-1)).copied().collect();
        assert_eq!(plus.len(), 60);
        assert_eq!(minus.len(), 10);

        let stats = cell_stats(&plus);
        let kbuy = (0u32, 1u8, 1i8, 0i8);
        let ksell = (0u32, 1u8, -1i8, 0i8);
        assert_eq!(stats.get(&kbuy).map(|c| c.n), Some(30));
        assert_eq!(stats.get(&ksell).map(|c| c.n), Some(30));
        // 同一正 resid_base：买 y>0、卖 y<0 ⟹ 异号 ⟹ 非 beta 漂移。
        assert!(!beta_drift_suspect(&stats, kbuy), "两 δ mean 异号不应标 suspect");

        // 构造同号情形：卖侧 resid_base 取负 ⟹ y=−resid>0 与买侧同正 ⟹ suspect。
        let drifted: Vec<ResidualTrade> = plus
            .iter()
            .map(|r| {
                let mut r2 = *r;
                if r2.class.delta == -1 {
                    r2.resid_base = -r2.resid_base;
                }
                r2
            })
            .collect();
        let dstats = cell_stats(&drifted);
        assert!(beta_drift_suspect(&dstats, kbuy), "两 δ mean 同号必须标 suspect");
    }
}
