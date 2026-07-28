use super::*;

/// ★A1（prereg-rev2-20260704）：force_state 第 8 维的 dump 编码（离线 round-trip 无损）。
/// None=0 / Dominated=1 / Dominates=2 / Tie=3 / Incomparable=4——`load_deltafree_dump` 逆映射。
pub(super) fn force_state_code(fs: Option<ForceStateA5>) -> u8 {
    match fs {
        None => 0,
        Some(ForceStateA5::Dominated) => 1,
        Some(ForceStateA5::Dominates) => 2,
        Some(ForceStateA5::Tie) => 3,
        Some(ForceStateA5::Incomparable) => 4,
    }
}

/// force_state 报告标签（None/Dom-/Dom+/Tie/Inc，δ-free 主裁决桶表可读列）。
pub(super) fn force_state_label(fs: Option<ForceStateA5>) -> &'static str {
    match fs {
        None => "None",
        Some(ForceStateA5::Dominated) => "Dom-",
        Some(ForceStateA5::Dominates) => "Dom+",
        Some(ForceStateA5::Tie) => "Tie",
        Some(ForceStateA5::Incomparable) => "Inc",
    }
}

/// ★A1（prereg-rev2 §1.2）ForceState⊥δ 检验：对每个出现的 force_state 态统计 δ∈{+1,−1} 计数。
///
/// 判据（i_class×δ 共线自毁铁律 memory `iclass_delta_collinearity_perm_degeneracy`）：
/// - **PASS**：每态 δ 两向都非空（min(n+,n−)≥1）⟹ 置换在该态内有交换自由度，非共线 ⟹ ForceState 合法进
///   主裁决聚合基。
/// - **FAIL**：任一态 δ 完全单向（n+=0 或 n−=0）⟹ 进基即毁其内 δ 置换 ⟹ A1 判 FALSIFIED，退出主裁决基
///   （仅报告桶维），停下上浮（fail 条件 3）。
/// 返回 markdown 报告串（含 PASS/FAIL 结论 + 逐态计数）——**不 panic**：FAIL 是诚实产出（照实入结果包，
/// 由 Lead/上浮决策 A1 处置），非管线错误（161 否定性照实）。
pub(super) fn forcestate_delta_orthogonality(records: &[ResidualTrade]) -> String {
    // 逐 force_state 态计 (n_δ+1, n_δ−1)。只检 Some(态)——None 是「无力度源」大桶，δ 两向天然混合
    // （不细分即不改置换），非 A1 关心的方向性维共线风险。
    let mut counts: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
    for r in records {
        if let Some(fs) = r.class.force_state {
            let e = counts.entry(force_state_code(Some(fs))).or_insert((0, 0));
            match r.class.delta {
                1 => e.0 += 1,
                -1 => e.1 += 1,
                _ => {}
            }
        }
    }
    let mut fail_states: Vec<String> = Vec::new();
    let mut rows = String::from("| force_state | n(δ+1) | n(δ−1) | 交换自由度 |\n|---|---|---|---|\n");
    for (&code, &(np, nm)) in &counts {
        let lbl = force_state_label(force_state_decode(code));
        let ok = np >= 1 && nm >= 1;
        rows.push_str(&format!("| {lbl} | {np} | {nm} | {} |\n", if ok { "有(非共线)" } else { "无(单向共线)" }));
        if !ok {
            fail_states.push(format!("{lbl}(n+={np},n−={nm})"));
        }
    }
    let verdict = if counts.is_empty() {
        "PASS(空——无 Some(force_state) 记录，force_state 全 None ⟹ A1 细分退化为单 None 桶，不改置换)"
            .to_string()
    } else if fail_states.is_empty() {
        "PASS(每态 δ 两向非空，ForceState⊥δ 有交换自由度 ⟹ 合法进主裁决聚合基)".to_string()
    } else {
        format!("FAIL(单向共线态: {}) ⟹ A1 判 FALSIFIED，ForceState 退出主裁决基，停下上浮(fail 条件 3)", fail_states.join(", "))
    };
    format!("**ForceState⊥δ 检验结论**：{verdict}\n\n{rows}")
}

/// force_state 编码逆映射（[`force_state_code`]），离线 dump 复现器还原第 8 维。
pub(super) fn force_state_decode(code: u8) -> Option<ForceStateA5> {
    match code {
        1 => Some(ForceStateA5::Dominated),
        2 => Some(ForceStateA5::Dominates),
        3 => Some(ForceStateA5::Tie),
        4 => Some(ForceStateA5::Incomparable),
        _ => None,
    }
}

/// ExitType 诊断切片的 dump 编码（codex-ruling-exittype-20260704 裁定甲：逐笔存档合法）。
/// **不进桶键/门控/裁决基**——纯诊断列（[`exit_type_decode`] 逆映射，[`exit_type_label`] 报告标签）。
pub(super) fn exit_type_code(et: ExitType) -> u8 {
    match et {
        ExitType::CloseRoot => 0,
        ExitType::ReduceCore => 1,
        ExitType::CloseShortDiff => 2,
        ExitType::RiskExit => 3,
        ExitType::Hold => 4,
    }
}

/// ExitType 编码逆映射（[`exit_type_code`]），离线 dump 复现器还原诊断列。
pub(super) fn exit_type_decode(code: u8) -> ExitType {
    match code {
        0 => ExitType::CloseRoot,
        1 => ExitType::ReduceCore,
        2 => ExitType::CloseShortDiff,
        3 => ExitType::RiskExit,
        _ => ExitType::Hold,
    }
}

/// ExitType 报告标签（W-VERIFY 5 变体占比拆解节可读列）。
pub(super) fn exit_type_label(et: ExitType) -> &'static str {
    match et {
        ExitType::CloseRoot => "CloseRoot(P5)",
        ExitType::ReduceCore => "ReduceCore(P6)",
        ExitType::CloseShortDiff => "CloseShortDiff(P7)",
        ExitType::RiskExit => "RiskExit(P1)",
        ExitType::Hold => "Hold(P0)",
    }
}

/// 连续价的逐 bar 差分样本标准差（σ̂ 计算核心，money path）。`pxs` = 时间序可交易价序列。
pub(super) fn stdev_consecutive_diffs(pxs: &[f64]) -> f64 {
    let diffs: Vec<f64> = pxs.windows(2).map(|w| w[1] - w[0]).collect();
    let n = diffs.len();
    if n < 2 {
        return 0.0;
    }
    let mean = diffs.iter().sum::<f64>() / n as f64;
    let var = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    var.sqrt()
}

/// 品种归一化尺度 σ̂_symbol（prereg-l3norm-20260703 §1，跨品种残差通约）：pre-OOS（date<OOS_START）
/// 全体可交易 bar 逐 bar close-to-close $ 涨跌的样本 std。**因果无前视**：池化残差 entry 全 ≥OOS_START，
/// σ̂ 由严格早于 OOS 的数据估 ⟹ F_entry-可测。返回 (σ̂, pre-OOS 可交易 bar 数)。
pub(super) fn sigma_pre_oos(ds: &data::Dataset, cfg: &ThetaConfig) -> (f64, usize) {
    let pre = ds.slice_date_window("1900-01-01", "2022-12-31"); // 严格早于 OOS_START=2023-01-01（闭区间）
    let pxs: Vec<f64> = pre
        .bars
        .iter()
        .filter(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * cfg.tick.tick_size)
        .collect();
    (stdev_consecutive_diffs(&pxs), pxs.len())
}

/// Z_decision 逐笔序列落盘（问题F 一等输出：`wverify_full` 主路径无条件外化）。
///
/// 每笔外化 δ-free 主裁决键 Z_decision=(level,bsp_class,parent_dir,force_state) + 残差成分（TSV：
/// level bsp δ σ^H force_state h_bucket time_block resid_base_bits cost_bits d_bits，A1/A6 增
/// force_state+d 两列）。默认落 `/tmp/wv_full_zdecision.tsv`
/// （与 /tmp/wv_full_rows.md 等主路径产物同级，无 env 门控）；`DELTAFREE_DUMP=<path>` 覆盖路径（离线
/// 复算/自检指定用）。resid_base/cost 用 `f64::to_bits` 十六进制（round-trip 精确）——离线
/// [`deltafree_exact_recompute`] 逐字节还原内存值，n_eff（Geyer IPS）/ δ-free perm_p 与在线口径 bit-exact
/// 可比。**本 dump 是输出产物，不进判定路径**——在线 δ-free 主裁决直接在内存 `records` 上算
/// （见 [`deltafree_verdict`]），不消费本文件。保 records 顺序（walk-forward 时间序）⟹ effective_n
/// 的成交时间序前提成立（decontam 口径）。
pub(super) fn dump_deltafree_pertrade(records: &[ResidualTrade]) {
    let path = std::env::var("DELTAFREE_DUMP")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/tmp/wv_full_zdecision.tsv".into());
    // A1/A6（prereg-rev2-20260704）：dump 增 force_state（δ-free 主裁决基第 8 维）+ d_bits（μ_R 分母）
    // 两列——离线复现器 [`deltafree_exact_recompute`] 逐字节还原 ⟹ 与在线主裁决/μ_R bit-exact。
    // 路线.pdf p13 逐笔存档：末列 exit_type 诊断切片（裁定甲存档合法，不进裁决桶键）。
    let mut out = String::from("level\tbsp\tdelta\tsigma_h\tforce_state\th_bucket\ttime_block\tresid_base_bits\tcost_bits\td_bits\texit_type\n");
    for r in records {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{:016x}\t{}\n",
            r.class.level, r.class.bsp_class(), r.class.delta, r.class.parent_dir,
            force_state_code(r.class.force_state),
            r.h_bucket, r.time_block, r.resid_base.to_bits(), r.cost.to_bits(), r.d.to_bits(),
            exit_type_code(r.exit_type)
        ));
    }
    std::fs::write(&path, &out).unwrap_or_else(|e| panic!("Z_decision dump 落盘失败 {path}：{e}"));
    eprintln!("[zdecision-dump] {} 笔 → {path}", records.len());
}

// ── δ-free 主裁决键形状冻结（编译期守卫，a3）──────────────────────────────────
// Z_decision 键 = (level, bsp_class, parent_dir, force_state) 四元组，**不含 δ 分量**
// （prereg-rev2 §1 + ★A1 force_state 第 8 维）。恒等函数指针赋值 ⟹ 若未来有人把 δ 加回
// [`perm_test::DeltaFreeKey`]（形状/分量类型改变），本行编译失败——比运行期测试更早拦截。
// 语义级守卫（仅 δ 不同的记录池化同桶）见 `tests::deltafree_verdict_key_is_delta_free_four_tuple`。
const _DELTAFREE_KEY_SHAPE_FROZEN: fn(
    perm_test::DeltaFreeKey,
) -> (u32, u8, i8, Option<ForceStateA5>) = |k| k;

/// δ-free 聚合基 (level, bsp_class, parent_dir) 精确三态裁决——**主判据**（prereg §3.2；问题F 收口）。
///
/// 输入任意残差序列（在线 `wverify_full` 内存 `records`，或离线 dump 还原），按 Z_decision 键池化两 δ
/// 方向，精确 Welford std（n−1 分母，非由 LCB 反推）+ 精确 n_eff（[`decontam::effective_n`] Geyer IPS）
/// + δ-free perm_p（[`perm_test::stratified_delta_perm_p_deltafree`]，B22 同批置换只读出侧池化）→
/// [`decontam::classify_bucket`] 三态。返回 (22 桶 markdown 表, 全局裁决, (V,F,I) 计数, LCB>0 桶摘要)。
///
/// **单一口径**：在线主路径与离线 [`deltafree_exact_recompute`] 共用本纯函数（两个 caller 一个逻辑，
/// 非「在线包一层调离线」垫片）⟹ 同数据同 records 序 ⟹ 逐字节同结果。records 迭代序须为 walk-forward
/// 时间序（effective_n 成交时间序前提）；BTreeMap 输出确定序。
pub(super) fn deltafree_verdict(
    records: &[ResidualTrade],
) -> (String, decontam::AcceptanceVerdict, (usize, usize, usize), String) {
    // δ-free 主裁决基 (level,bsp_class,parent_dir,force_state)：池化两 δ 方向，保时间序 Y 序列
    // （records 已按 walk-forward 序）。★A1（prereg-rev2-20260704）：force_state 第 8 维进主裁决基
    // （dfonline-a2 §4 两注入点之一，与 perm_test.rs base_map 键同步——否则 records Some(态) vs 键缺维
    // 全表 miss，fullz G2 前例）。
    let mut series: BTreeMap<perm_test::DeltaFreeKey, Vec<f64>> = BTreeMap::new();
    for r in records {
        series
            .entry((r.class.level, r.class.bsp_class(), r.class.parent_dir, r.class.force_state))
            .or_default()
            .push(r.y());
    }
    let pp = perm_test::stratified_delta_perm_p_deltafree(records, perm_test::N_PERM, perm_test::PERM_SEED);
    let (za, pa) = (1.645_f64, 0.05_f64);

    let mut rows = String::from(
        "| L | bsp | σ^H | force | N | n_eff | mean(Y) | std | lcb | ucb | cv | perm_p | state |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    let mut states = Vec::new();
    let mut lcb_pos: Vec<String> = Vec::new();
    for ((lv, bc, pd, fs), ys) in &series {
        let n = ys.len();
        let mean = ys.iter().sum::<f64>() / n as f64;
        // 精确 Welford std（样本 std，n−1 分母）——非由 LCB 反推的正态近似 std。
        let std = if n < 2 {
            f64::NAN
        } else {
            (ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
        };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let n_eff = decontam::effective_n(ys); // 精确 Geyer IPS
        let perm_p = *pp.get(&(*lv, *bc, *pd, *fs)).unwrap_or(&1.0);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        let fs_lbl = force_state_label(*fs);
        if lcb > 0.0 {
            lcb_pos.push(format!("L{lv} bsp{bc} σ{pd:+} f={fs_lbl} (mean{mean:+.2} lcb{lcb:+.2} n_eff{n_eff:.1} perm_p{perm_p:.3} {st:?})"));
        }
        rows.push_str(&format!(
            "| L{lv} | {bc} | σ{pd:+} | {fs_lbl} | {n} | {n_eff:.2} | {mean:+.4} | {std:.2} | {lcb:+.4} | {ucb:+.4} | {cv:.3} | {perm_p:.3} | {st:?} |\n"
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    let lcb_summary = if lcb_pos.is_empty() { "无".into() } else { lcb_pos.join("; ") };
    (rows, v, (nv, nf, ni), lcb_summary)
}

/// ExitType 诊断切片：按 5 变体拆解 n/占比/mean(Y)（codex-ruling-exittype-20260704 裁定甲）。
///
/// **纯描述性**——无三态判定、无 LCB 门、不进 accept/reject。防止诊断切片伪装成裁决层：本函数
/// 只回报每变体的计数/占比/收益均值（异质性阅读），不做 Validated/Falsified 分类。exit_type 在入场
/// 决策点 t 不可观测（post-treatment），故只作事后诊断，绝不进 μ 桶键/χ_t 门控/δ-free 主裁决基。
pub(super) fn exit_type_breakdown(records: &[ResidualTrade]) -> String {
    let n_total = records.len();
    // 5 变体固定序（CloseRoot/ReduceCore/CloseShortDiff/RiskExit/Hold）——即使某变体 0 笔也列出（穷尽）。
    let variants = [
        ExitType::CloseRoot,
        ExitType::ReduceCore,
        ExitType::CloseShortDiff,
        ExitType::RiskExit,
        ExitType::Hold,
    ];
    let mut rows = String::from(
        "| exit_type | n | 占比 | mean(Y) |\n|---|---|---|---|\n",
    );
    for et in variants {
        let ys: Vec<f64> = records.iter().filter(|r| r.exit_type == et).map(|r| r.y()).collect();
        let n = ys.len();
        let frac = if n_total == 0 { 0.0 } else { n as f64 / n_total as f64 };
        let mean = if n == 0 { f64::NAN } else { ys.iter().sum::<f64>() / n as f64 };
        rows.push_str(&format!(
            "| {} | {} | {:.4} | {:+.6} |\n",
            exit_type_label(et), n, frac, mean
        ));
    }
    format!(
        "# ExitType 诊断切片（裁定甲，纯描述性——无三态/无 LCB 门/不进裁决基）\n\n\
         - 总笔数 n={n_total}（5 变体穷尽拆解，占比和 = 1.0）\n\
         - **地位**：事后诊断切片。exit_type 不进 μ 桶键/χ_t 门控/δ-free 主裁决基（exit-μ-BUCKETING-FROZEN #180）。\n\n{rows}"
    )
}

pub(super) fn verdict_by<K: Ord + Copy + std::fmt::Debug + std::hash::Hash>(
    records: &[ResidualTrade],
    key_of: impl Fn(&MuClass) -> K,
    perm: &HashMap<K, f64>,
    level_of: impl Fn(&K) -> Option<u32>,
) -> (String, String, (usize, usize, usize)) {
    let mut agg: BTreeMap<K, (u64, f64, f64)> = BTreeMap::new();
    let mut series: BTreeMap<K, Vec<f64>> = BTreeMap::new();
    for r in records {
        let key = key_of(&r.class);
        let y = r.y();
        let e = agg.entry(key).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        let dl = y - e.1;
        e.1 += dl / e.0 as f64;
        e.2 += dl * (y - e.1);
        series.entry(key).or_default().push(y);
    }
    let (za, pa) = (1.645_f64, 0.05_f64);
    let mut states = Vec::new();
    let mut rows = String::from("| key | n | n_eff | mean(Y) | lcb | ucb | cv | perm_p | state | frontier(≥2) |\n");
    for (key, &(n, mean, m2)) in &agg {
        let std = if n < 2 { f64::NAN } else { (m2 / (n - 1) as f64).sqrt() };
        let se = std / (n as f64).sqrt();
        let (lcb, ucb) = (mean - za * se, mean + za * se);
        let cv = if mean == 0.0 { f64::INFINITY } else { std / mean.abs() };
        let perm_p = *perm.get(key).unwrap_or(&1.0);
        let ys = series.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let n_eff = decontam::effective_n(ys);
        let st = decontam::classify_bucket(mean, lcb, ucb, perm_p, n_eff, cv, za, pa);
        states.push(st);
        let frontier = match level_of(key) {
            Some(l) if l >= 2 => "frontier已修(c546b5633c bit-exact)，污染标注解除",
            _ => "—",
        };
        rows.push_str(&format!(
            "| {key:?} | {n} | {n_eff:.2} | {mean:.6} | {lcb:.6} | {ucb:.6} | {cv:.3} | {perm_p:.3} | {st:?} | {frontier} |\n"
        ));
    }
    let v = decontam::global_verdict(&states);
    let nv = states.iter().filter(|s| matches!(s, decontam::AlphaState::Validated)).count();
    let nf = states.iter().filter(|s| matches!(s, decontam::AlphaState::Falsified)).count();
    let ni = states.iter().filter(|s| matches!(s, decontam::AlphaState::Inconclusive)).count();
    (rows, format!("{v:?}"), (nv, nf, ni))
}

/// 从 δ-free dump（[`dump_deltafree_pertrade`] 落盘）逐行重建 `ResidualTrade`（Task #186 离线重算入口）。
/// resid_base/cost/d 由 `f64::from_bits`（十六进制 round-trip）逐字节还原内存值 ⟹ 与在线 records bit-exact。
/// A1/A6：force_state（第 8 维，code 编码）+ d（μ_R 分母）随 dump 还原——force_state 进 δ-free 主裁决基。
/// position 不入任何 δ-free/报告桶分层键 ⟹ 重建恒用 `PositionState::Root`（占位，不影响统计）。
/// bsp_class→BspBits 由 (bsp,δ) 唯一确定；force_state 经 struct-update 覆盖（from_certificate 恒 None）。
pub(super) fn load_deltafree_dump(path: &str) -> Vec<ResidualTrade> {
    use super::super::mu_estimator::PositionState;
    use crate::theta_v0::types::BspBits;
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("δ-free dump 读取失败 {path}：{e}"));
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        // 跳表头
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 11, "δ-free dump 行须 11 列（A1/A6 增 force_state+d_bits，裁定甲增 exit_type），得 {}：{line}", f.len());
        let level: u32 = f[0].parse().unwrap();
        let bsp: u8 = f[1].parse().unwrap();
        let delta: i8 = f[2].parse().unwrap();
        let sigma_h: i8 = f[3].parse().unwrap();
        let force_state = force_state_decode(f[4].parse().unwrap());
        let h_bucket: u8 = f[5].parse().unwrap();
        let time_block: u32 = f[6].parse().unwrap();
        let resid_base = f64::from_bits(u64::from_str_radix(f[7], 16).unwrap());
        let cost = f64::from_bits(u64::from_str_radix(f[8], 16).unwrap());
        let d = f64::from_bits(u64::from_str_radix(f[9], 16).unwrap());
        let exit_type = exit_type_decode(f[10].parse().unwrap()); // 诊断切片还原（不进裁决键）
        let bits = match (bsp, delta > 0) {
            (1, true) => BspBits { buy1: true, ..Default::default() },
            (1, false) => BspBits { sell1: true, ..Default::default() },
            (2, true) => BspBits { buy2: true, ..Default::default() },
            (2, false) => BspBits { sell2: true, ..Default::default() },
            (_, true) => BspBits { buy3: true, ..Default::default() },
            (_, false) => BspBits { sell3: true, ..Default::default() },
        };
        let class = MuClass {
            force_state, // A1：δ-free 主裁决基第 8 维还原
            ..MuClass::from_certificate(level, delta, bits, sigma_h, PositionState::Root)
        };
        out.push(ResidualTrade { class, resid_base, cost, h_bucket, time_block, d, exit_type });
    }
    out
}


/// ★#389 T3：LEE 稀疏性六字段的**单一来源**（m8 跑批的 stdout 诊断行与产物表共用一份取值，
/// 两处手抄同一组字段会静默漂移）。顺序 = 产物表列序：
/// `n_decisions` / `n_cap_narrowed` / `off_clock 订单` / `其中风控·帽可解释` /
/// `off_clock 级别Δq` / `未解释`。
pub(super) fn lee_row_cells(s: &super::super::super::strategy::level_order::LevelOrderStats) -> [u64; 6] {
    [
        s.n_decisions,
        s.n_cap_narrowed,
        s.n_orders_off_structural_clock,
        s.n_orders_off_structural_clock_risk_explained,
        s.n_levels_off_clock_delta,
        s.n_levels_off_clock_delta_unexplained,
    ]
}

/// ★#374 有效域收窄 / #388 T2 / #423 的**报告层标注**：单标量成本口径**无良定义**的档下，
/// 随机对照系读数一律标此串。
///
/// **适用范围 = 标量无良定义档，不是「标定档」全体**（★#423 第二阶段订正）。判定本体 =
/// [`treasury::scalar_cost_rate_opt`](super::super::treasury::scalar_cost_rate_opt)（→
/// `venue_fee::VenueFeeSchedule::constant_effective_rate` 的结构检查），两档落本标注：
/// - **按股档（per-share）**：逐笔费率随 (qty, px, side) 非线性变（最低佣金托底 / 1% 名义额上限 /
///   仅卖出监管费）⟹ 不存在常数等效费率 ⟹ 依赖它的 LCB_OOS(R)（block bootstrap 的成本口径）
///   与三态判据**确无**合法取值；
/// - **按金额档但 maker≠taker**：角色维不消失 ⟹ 常数不由 datum 内容唯一确定，选一侧充数就是把
///   不对称藏进一个常数。
///
/// **不落本标注的档**：未标定档（`fee_schedule = None`，常率）与**按金额对称档**
/// （per-notional 且 maker/taker 逐位对称，如 Binance 现货 VIP0 = 10bp/side）——后者自 ★#423
/// 第一阶段起标量**可得**，其层4 读数照常产出（此前实现按 `fee_schedule.is_some()` 一刀切判
/// `None`、连带撤下该档读数，那道保守收窄已拆除）。
///
/// 喂近似值（实际有效费率等）在无良定义档已由 #374 明文否决——那只是把不对称藏进一个更贵的
/// 常数（090 声明膨胀）。
pub(super) const SCALAR_UNDEFINED_UNAVAILABLE: &str = "不可用(单标量成本口径无良定义 #374/#423)";

/// m8 四层报告的**层4 两格**（LCB_OOS(R) / 三态）渲染。
///
/// - `lcb = Some(x)`（标量可得档：未标定档，或按金额对称标定档）⟹ 与降级改造前**逐字符相同**的
///   原口径（M8:168 三分支）；
/// - `lcb = None`（标量无良定义档，`RunResult::fee_rate = None`）⟹ 两格均
///   [`SCALAR_UNDEFINED_UNAVAILABLE`]。三态**不用 R 的正负顶替**：判据是 `LCB>0`，缺 LCB 即无
///   判据（诚实缺席，不降格冒充）。
///
/// 本函数的入参与层4 声明段（[`layer4_scalar_caliber_notice`]）/ 结算行
/// （[`layer4_verdict_line`]）**同一个真值**——三者都由 `scalar_cost_rate_opt` 的可得性驱动，
/// 不存在第二个判据（★#423 第二阶段：此前声明段按 `fee_schedule.is_some()` 判、数值按
/// `fee_rate` 判，按金额对称档下二者分歧 ⟹ 同一报告内声明「不可用」而表格给数，090 声明膨胀）。
pub(super) fn layer4_cells(lcb: Option<f64>, r_total: f64) -> (String, String) {
    match lcb {
        Some(lcb_r) => {
            let verdict = if lcb_r > 0.0 {
                "CONFIRMED"
            } else if r_total > 0.0 {
                "INCONCLUSIVE"
            } else {
                "无(R≤0)"
            };
            (format!("{lcb_r:+.0}"), verdict.to_string())
        }
        None => (
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
        ),
    }
}

/// m8 四层报告的**层4 随机对照三格**（★#423 收尾轮 F）——`Θ>随机` / `p_shift` / `p_indep`。
///
/// ## 为什么加这三格（票体交付缺口）
///
/// `metrics::significance` 在标量可得档被**完整调用**，`theta_beats_random` / `shift_pvalue` /
/// `indep_pvalue` 三值都已算出，但此前 m8 只取 `boot_ci95_lo` 一个字段 ⟹ 票体 What-to-build
/// 明写要出的「随机对照」读数**无落盘出口、采不到**。本函数是那个出口，**不新增任何计算**
/// （同一次 `significance` 调用的既有字段）⟹ 不改 RNG 消耗、不改 dump 字节。
///
/// ## 渲染口径
///
/// - `None`（标量无良定义档：按股 / 按金额非对称）⟹ 三格均 [`SCALAR_UNDEFINED_UNAVAILABLE`]。
///   该档本就不调 `significance`（无输入费率）⟹ 三值**未定义**，照实标不可用，
///   **不填 0、不留空**（090：空格会被读成"算了但为 0"）。与层4 两格同一个真值、同一个标注串。
/// - `Some(sig)` ⟹ `theta_beats_random` 渲染「是/否」；两个 p 值 `{:.4}`。
///
/// **退化标注**：`sig.controls_degenerate` 为真时（schedule-shift 无非零合法平移，或
/// independent 全笔 `hold ≥ len` ⟹ 无真随机样本）p_upper 无统计含义（`metrics` 该字段文档）。
/// 此时三格各附 `(对照退化)` / `(退化)`——p=1.0 是真算出来的值，但不得被读作"未能否证"。
/// 该标志是**行级**的（`metrics::Significance` 只给一个合并标志，不分 shift/indep）⟹ 两个 p
/// 格同标；不在本处推断"是哪一个退化"（那要改 `metrics` 的返回形态，另票）。
pub(super) fn layer4_random_control_cells(sig: Option<&super::super::metrics::Significance>) -> (String, String, String) {
    match sig {
        None => (
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
            SCALAR_UNDEFINED_UNAVAILABLE.to_string(),
        ),
        Some(s) => {
            let deg = if s.controls_degenerate { "(退化)" } else { "" };
            (
                format!(
                    "{}{}",
                    if s.theta_beats_random { "是" } else { "否" },
                    if s.controls_degenerate { "(对照退化)" } else { "" }
                ),
                format!("{:.4}{deg}", s.shift_pvalue),
                format!("{:.4}{deg}", s.indep_pvalue),
            )
        }
    }
}

/// m8 报告头的**层4 标量成本口径声明段**（★#423 第二阶段）——`None` = 不出声明段。
///
/// 三分叉，判据**只有一个**：[`treasury::scalar_cost_rate_opt`](super::super::treasury::scalar_cost_rate_opt)
/// 在本 `exec` 上的可得性（与层4 两格 [`layer4_cells`] 的数值来源同一函数）。
///
/// | 档 | 产出 |
/// |---|---|
/// | 未标定（`fee_schedule = None`） | `None`——不出声明段（臂R 产物逐字节不变） |
/// | 标定 + 标量可得（按金额对称档） | 「可得」正文：层4 读数**照常产出**，并登记本声明不覆盖什么 |
/// | 标定 + 标量无良定义（按股档 / 按金额非对称档） | 「不可用」正文，读数标 [`SCALAR_UNDEFINED_UNAVAILABLE`] |
pub(super) fn layer4_scalar_caliber_notice(exec: &ExecConfig) -> Option<String> {
    exec.fee_schedule.as_ref()?; // 未标定档（臂R）不出声明段——产物逐字节不变。
    Some(match super::super::treasury::scalar_cost_rate_opt(exec) {
        Some(rate) => format!(
            "> **标定档单标量成本口径可得声明（★#423）**：本跑批经 `M8_FEE_DATUM` 注入 venue 费率 \
             datum，且该档为**按金额档（per-notional）且 maker/taker 逐位对称**、撮合角色取自编译期\
             常量 `venue_fee::PRODUCTION_LIQUIDITY_ROLE` ⟹ 存在与 (qty, px, side) 无关的常数等效\
             费率（判定本体 = `venue_fee::VenueFeeSchedule::constant_effective_rate`）。单标量成本\
             费率（`RunResult::fee_rate`，经 `treasury::scalar_cost_rate_opt`）= 档 bps/1e4 + \
             `slippage_bps`/1e4 + `tax_bps`/1e4 = **{:.6}**（标定档下 `tax_bps` 由 \
             `treasury::fee_quoter` 的 assert 强制为 0）。\n\
             > 故层4 的 `LCB_OOS(R)` 与三态判据在本臂**照常产出**——与未标定臂同一函数、同一口径，\
             反事实臂（随机对照在不同 px 上重执行）用同一个常数，不引入口径不对称。\n\
             >\n\
             > **★#423 收尾轮 F 新增输出**：层4 表新增三列 `Θ>随机` / `p_shift(平移)` / \
             `p_indep(独立)`，取自**同一次** `metrics::significance` 调用的既有字段\
             （`theta_beats_random` / `shift_pvalue` / `indep_pvalue`）——此前只取 `boot_ci95_lo`，\
             这三值算了却无落盘出口。**旧产物没有这三列**（本轮新增列，**不是**读数漂移）；\
             三值不新增任何计算 ⟹ `trades.jsonl` / `tower_events.jsonl` 逐字节不变（回归门 + `cmp` 实证）。\
             p 值口径 = `p_upper=(1+count(rand≥theta))/(N+1)`，N=1000，seed 冻结；\
             `Θ>随机 ⟺ 两对照 p_upper 均 ≤0.05 且对照未退化`；对照退化时格内附 `(退化)`\
             （p=1.0 是真算出的值，但**无统计含义**，不得读作「未能否证」）。\n\
             >\n\
             > **本声明不覆盖**（照实登记，不膨胀）：\n\
             > - `slippage_bps` 仍**未标定**（venue datum 只标佣金/监管/清算科目，报告 §3.3）⟹ 本臂\
             标量是「L2 标定佣金 + L1 未标定滑点」的合成；成交费率科目的口径标签见上方抬头；\n\
             > - `l3_delta_r_alpha` 鞅守卫的成本剥离**不在本跑批路径内**（其唯一调用方是同文件的 \
             `#[ignore]` 合成鞅守卫测试）；\n\
             > - 层1 signal 的 INCONCLUSIVE 与本档无关（转引，不重算）。\n\n",
            rate
        ),
        None => format!(
            "> **单标量成本口径无良定义声明（#374 / #385 / ★#423）**：本跑批经 `M8_FEE_DATUM` 注入 \
             venue 费率 datum，且该档**不存在**与 (qty, px, side) 无关的常数等效费率 ⟹ 单标量成本\
             费率（`RunResult::fee_rate`）**无良定义**（`treasury::scalar_cost_rate_opt` 判 `None`，\
             判定本体 = `venue_fee::VenueFeeSchedule::constant_effective_rate`）。两种档落此支：\n\
             > - **按股档（per-share）**：逐笔费率随 (qty, px, side) 非线性变（最低佣金托底 / 1% 名义\
             额上限 / 仅卖出监管费）；\n\
             > - **按金额档但 maker≠taker**：角色维不消失，常数不由 datum 内容唯一确定。\n\
             >\n\
             > 故下列读数标 `{SCALAR_UNDEFINED_UNAVAILABLE}`，**不以近似费率顶替**——喂实际有效费率\
             （Σfee/Σnotional 等）只是把口径不对称藏进一个更贵的常数（#374 明文否决，090 声明膨胀）：\n\
             > - `LCB_OOS(R)`（block bootstrap 的成本口径依赖单标量费率）；\n\
             > - **三态判据**（判据是 `LCB>0`，缺 LCB 即无判据——不用 R 的正负降格顶替）；\n\
             > - `metrics::significance` 派生的随机对照系——层4 表的 `Θ>随机` / `p_shift(平移)` / \
             `p_indep(独立)` 三列（★#423 收尾轮 F 新增列；本档下 `significance` **一律不调**，\
             无输入费率 ⟹ 三值未定义，故三格同标不可用，**不填 0 也不留空**）；\n\
             > - `l3_delta_r_alpha` 鞅守卫的成本剥离（不在本跑批路径内；本档要接须改逐笔实付累计）。\n\
             >\n\
             > **仍然有效**（与单标量费率无关，逐笔实付经 `treasury::fee_quoter` 解析）：n_orders / \
             ΣN_tΔP_t / Comm+Slip / Funding / Borrow / LiqLoss / net_r(execR) / MaxDD / 声部数 / \
             终Stage / Q_T / W_T / η 列 / R(含浮盈) / `NEST_GATE_STATS`。\n\
             >\n\
             > **对上文抬头的更正**：抬头「signal 层无 alpha ⟹ 端到端负/INCONCLUSIVE 照实」一句\
             描述的是**标量可得档**的三态判读。本臂层4 **无结论**（判据缺输入），该句对本臂不适用——\
             不得把「不可用」读作「负」或「INCONCLUSIVE」。\n\n"
        ),
    })
}

/// m8「判据结算」段的**层4 结算行**（★#423 第二阶段）——与 [`layer4_cells`] /
/// [`layer4_scalar_caliber_notice`] 同一个真值（`scalar_cost_rate_opt` 的可得性）。
///
/// - `Some`（标量可得：未标定档，或按金额对称标定档）⟹ 常规判据行（见 LCB 列，未过 ⟹
///   INCONCLUSIVE）；
/// - `None`（标量无良定义档）⟹ **无结论**行。措辞§5.6 的 INCONCLUSIVE 是「有 LCB 且 ≤0」的态，
///   拿它套「LCB 不存在」是静默越域。
pub(super) fn layer4_verdict_line(scalar_rate: Option<f64>) -> &'static str {
    match scalar_rate {
        None => {
            "- **层4 完整策略** `LCB_OOS(R)>0`：**本臂无结论**——本档单标量成本口径无良定义 ⟹ \
             LCB_OOS(R) 不可用（#374 / ★#423，见上方声明），判据无输入。**既不宣称 confirmed \
             alpha，也不判 INCONCLUSIVE**（后者是「有 LCB 且 ≤0」的态，套用于「LCB 不存在」是越域）。\
             该层要在本档下有结论，须先给随机对照接 (qty, px, side) 逐笔费率缝（另票）。\n"
        }
        Some(_) => {
            "- **层4 完整策略** `LCB_OOS(R)>0`：见 LCB_OOS(R) 列——**未过 ⟹ INCONCLUSIVE**，\
             不宣称 confirmed alpha（措辞§5.6：INCONCLUSIVE≠无 alpha；§5.3：不外推 max-full）。\n"
        }
    }
}

/// ★#490 MED-3：报告 header 的执行域必须与 `OverlayRunResult.voice_exec` 同口径。
pub(super) fn m8_execution_projection_label(voice_exec_is_some: bool) -> &'static str {
    if voice_exec_is_some {
        "声部执行投影 + 净额影子账本"
    } else {
        "净额执行 + overlay 旁路/账本"
    }
}

/// ★#492（#481 二轮复审 MED）：审计表的 trades/fill 必须各自显式标注计数域。
pub(super) fn m8_fee_audit_headings(voice_exec_is_some: bool) -> (&'static str, &'static str) {
    if voice_exec_is_some {
        ("声部投影trades", "净额影子fill")
    } else {
        ("净额trades", "净额fill")
    }
}

/// ★#490 MED-2 / #492：逐窗净额 FeeAudit 行的单一渲染入口；
/// `unclassified_venue` 是未标定/per-notional 档的真实 venue 科目，必须与已分类科目并列可见。
pub(super) fn format_m8_fee_audit_row(
    tag: &str,
    projected_trades: usize,
    fee: super::super::treasury::FeeAudit,
) -> String {
    let effective_venue_rate = if fee.notional > 0.0 {
        fee.venue_total() / fee.notional
    } else {
        0.0
    };
    format!(
        "| {tag} | {projected_trades} | {} | {} | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} | \
         {:.6} | {:.6} | {:.6} | {}/{} | {}/{} | {}/{} | {:.6e} |\n",
        fee.n_fills,
        fee.notional,
        fee.commission,
        fee.passthru,
        fee.clearing_cat,
        fee.sec,
        fee.taf,
        fee.unclassified_venue,
        fee.slippage,
        fee.total_fee,
        fee.min_commission_hits,
        fee.n_fills,
        fee.notional_cap_hits,
        fee.n_fills,
        fee.taf_cap_hits,
        fee.n_fills,
        effective_venue_rate,
    )
}

pub(super) fn m8_layer23_settlement(
    symbol: &str,
    rows: &[(String, f64, super::super::super::strategy::ledger::TStage)],
) -> (String, String) {
    let execution = rows
        .iter()
        .map(|(tag, net_r, _)| format!("`{tag}` net_r={net_r:+.0}"))
        .collect::<Vec<_>>()
        .join("；");
    let treasury = rows
        .iter()
        .map(|(tag, _, stage)| {
            let stage = match stage {
                super::super::super::strategy::ledger::TStage::CostReduction => "I(降成本)",
                super::super::super::strategy::ledger::TStage::CapitalRecovered => "II(已回本)",
                super::super::super::strategy::ledger::TStage::EarningShares => "III(赚份额)",
            };
            format!("`{tag}`={stage}")
        })
        .collect::<Vec<_>>()
        .join("；");
    (
        format!(
            "- **层2 execution** `E[R(Π_exec)]>0`：{execution}。\
             {symbol} 本窗读数仅作 execution/treasury 路径与费用算术审计，禁作 alpha 论据或策略择优输入。"
        ),
        format!(
            "- **层3 treasury** `Reach(StageIII)>0`：{treasury}；\
             终态按本次真实窗 TW 账本逐窗登记，不由 signal 层结论静态推断。"
        ),
    )
}
