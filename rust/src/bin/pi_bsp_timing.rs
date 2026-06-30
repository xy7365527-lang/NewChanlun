//! # `pi_bsp_timing` —— 忠实实装 π_Θ^bsp 买卖点离散择时多声部状态机（goal #5）
//!
//! ## 与 `pure_bsp_timing`（工位 Q 简化版）的区别 —— no-patch 核心
//! 工位 Q 的 `pure_bsp_timing` 把全部买卖点坍缩成**单仓 ambient toggle**（q∈{+1,0,−1}），
//! 既无声部 position instance、无祖先闭合、也无 N_t^bsp=Σσ_v q_v。那是 §19 明确禁止的
//! 「每元素覆盖」对偶的另一种坍缩（坍缩成单声部）。本 bin 忠实实装 PDF 全 19 节的
//! **多声部持仓状态机**。
//!
//! ## 逐节 PDF 对应（忠实度第一验收 —— 报告里逐条标注 §几）
//! - §1-2  三类买卖点 6 维向量 b_ℓ → 方向证书 γ=(c,ℓ,δ,I_γ,t)。本 bin 从生产 classifier
//!         `Classification.levels[ℓ].bsp[*].bits` 直接读 b_ℓ（忠实，零重判）。carrier c = **产出该
//!         买卖点的走势容器** hostOf(g) 的稳定 `ElementId`（PDF c 是「操作容器/走势 carrier」；
//!         644 坐标纪律：用 [`extract_elements`] 真嵌套塔 + [`attach_bsp_to_tree`] 638 hostOf 判准取，
//!         非伪 (level, source_index) 标识——后者无真 Compose 父子，子声部恒 0）。
//! - §3    声部 position instance v=(c_v, η_v, σ_v, n_v)。[`Voice`] 结构忠实。
//! - §4    祖先闭合 A_{t+1}=AncOK(A^raw)；父 active 子才 active（[`anc_ok`]）。
//! - §5    入场证书 δ(γ)=σ_v、出场证书 δ(γ)=−σ_v（[`step`] 内 entry/exit 判定）。
//! - §6    短差子声部 σ_u=−σ_p（[`Voice::child_dir`]，§16 多空双开）。
//! - §7    I_v^in/I_v^out 类别集合 + Q_Θ 仓位 size。本实装 Q_Θ=固定单位 1（诚实简化，见下）。
//! - §9    先平后开 A^raw=(A_t\D_t)∪O_t（[`step`] 顺序）。
//! - §10-11 全互斥 6 谓词 P_{v,1..6} → C_{v,j}（[`classify_action`]，恰一类成立的 §10 证明）。
//! - §12   目标头寸 p̃^bsp=Σ q_v e_v^σ；★净额 N_t^bsp=Σ σ_v q_v（[`net_target`]）。
//! - §13   风险投影 K_Θ + LexArgmin。本实装 K_Θ=恒等（无杠杆/保证金约束），LexArgmin=p̃ 本身
//!         （诚实简化，见 ceiling）。
//! - §14   全定义（9 假设 ∀x ∃!O）：本状态机每 bar 对每声部恰产一个 C_{v,j}（§10 互斥保证）。
//! - §18   r^bsp=N^bsp·ΔP−C，alpha 判定（NAV 口径复用 metrics，可比 π^cov）。
//! - §19   π^cov≤0 不能推出 π^bsp≤0；本 bin 单独实装 π^bsp 回答 goal#5。
//!
//! ## 诚实简化（ceiling 标注，formalization-validity-domain）
//! - **Q_Θ=1 单位**（§7/§13）：本实装每声部 q_v=1 固定单位，不做 vol/equity sizing。
//!   ceiling：sizing 需接 risk.rs K_Θ；本轮先测「声部状态机的净额方向」是否携带 alpha——
//!   方向是 N_t^bsp 符号结构的核心，sizing 是二阶标度。升级路径：Q_Θ 读 §7 I_v^in 类别 + equity。
//! - **K_Θ=恒等、LexArgmin=p̃**（§13）：无保证金/杠杆约束 ⟹ p* = p̃^bsp 直接成交。
//!   ceiling：真实约束需 risk.rs；纯择时 edge 测试不需要杠杆建模（杠杆只放大，不改方向 alpha）。
//! - **carrier = hostOf(g) ElementId**（§2 c，644 结构父子）：PDF c 是抽象「操作容器」，本实装用
//!   产出买卖点的真嵌套塔走势容器 hostOf(g) 的稳定 `ElementId`（跨 bar 全量/增量同 ID）。父子关系
//!   p(v) = hostOf 的**真 Compose 父容器** `parent_id`（§9.1 par_C(κ(u))=κ(v)）上的 active 声部——
//!   **非**级别差伪造、**非**同 bar 反向证书匹配（旧版子声部恒 0 的根因 #3）。短差子声部 σ_u=−σ_p
//!   是父持仓**期内**的对冲腿（§6/§16），出场证书严格按 carrier 配对（反向证书在子 carrier 开子声部，
//!   不平父声部）。
//!
//! ## 坐标系纪律（644）+ NAV 口径（可比 π^cov）
//! 走生产 `IncrementalClassifier::classify_at(i)`（因果塔，≤i，无 look-ahead）+ 与 runner
//! `newly_confirmed_step` 同的 seen-set append-only diff。NAV 用 net-position 逐 bar MtM
//! （N_t^bsp × ΔP，§18），归一化 + `metrics::compute`/`significance` —— 与工位 L 测 π^cov
//! **同口径** ⟹ Sharpe 可比。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin pi_bsp_timing -- <SYMBOL> [START END]
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::backtest::metrics::{self, TradeRecord};
use newchan_rust::theta_v0::backtest::mu_estimator::{
    marginal_return, MuClass, MuEstimator, MuObservation, PositionState,
};
use newchan_rust::theta_v0::backtest::selector::chi_t;
use newchan_rust::theta_v0::classifier::recursive_tower::ElementId;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::classifier::Classification;
use newchan_rust::theta_v0::strategy::coverage::{
    attach_bsp_carrier_indexed, attach_bsp_parent_carrier_indexed, build_tree_endpoint_index,
    extract_carrier_forest,
};
use newchan_rust::theta_v0::types::{BspBits, Side};
use std::collections::{HashMap, HashSet};

/// ★Lift.Context^δ_j（递归证明.pdf §2.3；codex 裁决口径 A，2026-06-30）：判次级别买卖点 g 是否
/// **证成上级 carrier c 第 j 类**——即上级级别 ℓ_c 在 c 右端点 ρ_c 处**已独立分类出**与 g 同向 δ 的
/// 买卖点。
///
/// ## 为何是「读上级已确认 bsp」而非「现场重算 EndpointSituation」（codex no-patch 裁决）
/// EndpointSituation 的 left_center / retrace_not_reenter / 一买背驰（A/C MACD 面积）字段是**上级
/// 级别的段对结构**（leave 段 / retest 段 / A 段 / C 段）算出的——次级别 g 只是上级某走势的右端点
/// **单点**，不携带上级段对。若用 g 单点 + 上级 centers 现场构造 EndpointSituation，这些字段只能瞎填
/// false（伪 Context，违反 no-patch.md）。上级级别 ℓ_c 自己的 `judge_first/third/extract_second`
/// **已用上级段对算过** ⟹ `Classification.levels[ℓ_c].bsp` 就是「相对 c 重算」的忠实结果。Context
/// = ∃ 上级 bsp 在 ρ_c 处含同向 δ 证书（codex 审点 #1=A，#4=坐标对齐 ρ==source_index）。
///
/// ## δ 取向（codex 审点 #2=σ_u/证书方向）
/// `side` 是子声部要 Lift 的**证书方向**（g 证成的买卖点方向）——非被对冲的父方向 σ_p。买 δ=+1
/// 用 conf_plus，卖 δ=−1 用 conf_minus（`confirm_side`）。
///
/// 返回 true ⟹ g 可提升（U(g) 的 Context 分量满足）；false ⟹ Context 缺失（子声部不 Lift）。
fn context_lifts(
    classification: &Classification,
    parent_level: u32,
    parent_rho: usize,
    side: Side,
) -> bool {
    classification
        .levels
        .get(parent_level as usize)
        .is_some_and(|ls| {
            ls.bsp
                .iter()
                .any(|p| p.source_index == parent_rho && p.bits.confirm_side(side))
        })
}

/// 声部 position instance（PDF §3：v=(c_v, η_v, σ_v, n_v)）。
///
/// - `carrier`：c_v —— 真嵌套塔里**产出该买卖点的走势容器** hostOf(g) 的稳定 `ElementId`
///   （644 坐标纪律：跨 bar 稳定，全量/增量同 ID）。同 carrier 多次开仓由 `generation` 区分（§3 n_v）。
///   ★644 修复：旧版用 (level, source_index) 伪 carrier + 同 bar 反向证书伪父子（子声部恒 0）；
///   现用 [`extract_elements`] 真 Compose 嵌套树 + [`attach_bsp_to_tree`] 638 hostOf 判准建结构父子。
/// - `dir`：σ_v ∈ {+1,−1} 持仓方向（§3）。多头 +1 / 空头 −1。
/// - `parent`：p(v) ∈ V∪{⊥}（§3 父子声部关系）。`None`=根声部（⊥，host 父容器=边界胚元 ∂）；
///   `Some(idx)`=父声部在 `voices` 的索引（host 的真 Compose 父容器 `parent_id` 上的 active 声部）。
/// - `qty`：q_v 目标单位数（§7/§12）。本实装固定 1（诚实简化）。
/// - `generation`：n_v（§3，区分同 carrier 多次开仓）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Voice {
    carrier: ElementId,
    dir: i8,
    parent: Option<usize>,
    qty: f64,
    generation: u32,
    entry_bar: usize,
    /// 开仓证书的 I_γ 类别（§1-2 b_ℓ ∈ {0,1}^6）——μ(z) 分类的 i_class 分量（不压扁，§P4 §5）。
    /// 子声部用父 carrier 反向证书的 bits（开子声部那条证书），根声部用自身证书 bits。
    bits: BspBits,
    /// 开仓证书所在级别 ℓ（μ(z) 的 level 分量；carrier.level，§12 γ=(c,ℓ,δ,I_γ,t)）。
    level: u32,
}

impl Voice {
    /// §6/§16 短差子声部方向 σ_u=−σ_p（父 active 时反向买卖点证书开子声部）。
    fn child_dir(parent_dir: i8) -> i8 {
        -parent_dir
    }
}

/// 一根买卖点向量 b_ℓ 的方向证书摘要（§1-2 γ）：本 bar 是否出现买侧/卖侧确认。
/// I_γ⊆{1,2,3} 保留（任意类成立即证书成立，§2 I_γ≠∅）—— 这里只需方向 δ（买/卖）。
struct CertHit {
    buy: bool,  // δ=+1 证书（I_γ={i:B_iℓ=1}≠∅）
    sell: bool, // δ=−1 证书（I_γ={i:S_iℓ=1}≠∅）
}

fn cert_of(b: &BspBits) -> CertHit {
    CertHit { buy: b.conf_plus(), sell: b.conf_minus() }
}

/// §4 祖先闭合 AncOK(A)={v∈A : Anc(v)⊆A}。父声部不在 active 集 ⟹ 子声部被剪。
///
/// 输入 `active`=A^raw 索引集 + `voices`（含 parent 链）。返回闭合后的 active 集（父不 active ⟹ 子移除）。
/// 迭代到不动点（§9：先平后开后 AncOK；移除父可能级联移除子）。
fn anc_ok(mut active: HashSet<usize>, voices: &[Voice]) -> HashSet<usize> {
    loop {
        let before = active.len();
        let snapshot = active.clone();
        active.retain(|&v| match voices[v].parent {
            None => true,                  // 根声部（⊥）无祖先，恒满足
            Some(p) => snapshot.contains(&p), // 父 active 才保留（§4 a_{v,t}≤a_{p(v),t}）
        });
        if active.len() == before {
            break; // 不动点
        }
    }
    active
}

/// §10-11 全互斥动作分类 C_{v,j}（对 active 声部 v）。返回是否本 bar 应**平仓**该声部。
///
/// 6 谓词（§10）：P1=风险关闭, P2=父声部关闭, P3=I_v^out≠∅（反向证书=出场）, P4=未 active∧I_v^in≠∅,
/// P5=active∧I_v^in≠∅, P6=无有效信号。互斥化 C_{v,1}=P1, C_{v,j}=P_j∧⋀_{k<j}¬P_k（§10 close 优先）。
/// 对 active 声部，触发平仓的类 = P1/P2/P3（前三优先级）。本函数判这三者（§11 「close 优先于 open」）。
fn should_close(exit_cert: bool, parent_active: bool, risk_close: bool) -> bool {
    // C_{v,1}=P1（风险关闭）→ 平。C_{v,2}=P2∧¬P1（父关闭）→ 平。C_{v,3}=P3∧¬P1∧¬P2（出场证书）→ 平。
    // 互斥优先级：任一前三谓词成立即平（§11 close 优先）。父关闭（P2）在 anc_ok 阶段级联处理，
    // 故此处 parent_active 恒传 true，实际触发 = 风险关闭(P1) 或 出场证书(P3)。
    risk_close || !parent_active || exit_cert
}

/// §12 净额 N_t^bsp = Σ_{v∈A} σ_v q_v（声部状态机持仓净额，**不是** 覆盖 Σ_e s_e ε_e）。
fn net_target(active: &HashSet<usize>, voices: &[Voice]) -> f64 {
    active.iter().map(|&v| voices[v].dir as f64 * voices[v].qty).sum()
}

/// 喂 μ 的父向：根声部父向=0；子声部父向=父 voice.dir（§16 σ_p）。
fn parent_dir_of(voices: &[Voice], v: usize) -> i8 {
    voices[v].parent.map_or(0, |p| voices[p].dir)
}

/// open_carrier：确保 carrier 上有 active voice（已 live ⟹ 复用首个；否则沿父链先开父再开本级）。
/// 返回该 carrier 的 active voice 索引。父链上溯到 ∂（parent_id=None）或无同 bar 证书的祖先为止。
#[allow(clippy::too_many_arguments)]
fn open_carrier(
    voices: &mut Vec<Voice>,
    active: &mut HashSet<usize>,
    gen_counter: &mut HashMap<ElementId, u32>,
    active_voice_by_carrier: &HashMap<ElementId, usize>,
    certs_by_carrier: &HashMap<ElementId, (Option<ElementId>, i8, BspBits)>,
    opened_this_bar: &mut HashMap<ElementId, usize>,
    carrier: ElementId,
    parent_id: Option<ElementId>,
    cert_dir: i8,
    cert_bits: BspBits,
    entry_bar: usize,
) -> usize {
    // 已 live（上一 bar 留存）⟹ 复用（§9 父已 live 分支）。
    if let Some(&idx) = active_voice_by_carrier.get(&carrier) {
        return idx;
    }
    // 本 bar 已开过该 carrier ⟹ 复用（防同 bar 重复开，父链多子共享父）。
    if let Some(&idx) = opened_this_bar.get(&carrier) {
        return idx;
    }
    // 父声部：parent_id 有真父 ∧（父已 live 或父本 bar 有证书）⟹ 递归先开父（§9 生成父 voice）。
    // ★Lift.Nest（递归证明.pdf §2.2，codex YES#1）：J_ℓ(d)⊆J_{ℓ+1}(c)（子区间⊆父区间）。
    // K_i 真嵌套（push_element_tree 子 sub_moves 挂父，compose 父区间取首尾子）下**恒满足**
    // ⟹ debug_assert 守卫足够（生产域无反例；只防手工乱序/伪造 LeveledMove）。本 bin 在 K_i 上
    // 建 carrier，子 carrier 与父 carrier 的区间⊆关系由 extract_carrier_forest 真嵌套保证。
    let parent = parent_id.and_then(|pid| {
        if active_voice_by_carrier.contains_key(&pid)
            || opened_this_bar.contains_key(&pid)
            || certs_by_carrier.contains_key(&pid)
        {
            let (ppid, pdir, pbits) =
                certs_by_carrier.get(&pid).copied().unwrap_or((None, cert_dir, cert_bits));
            Some(open_carrier(
                voices, active, gen_counter, active_voice_by_carrier,
                certs_by_carrier, opened_this_bar, pid, ppid, pdir, pbits, entry_bar,
            ))
        } else {
            None // 父无同 bar 证书且未 live ⟹ 不生成父（codex 严格条件）⟹ 本级作根声部。
        }
    });
    // 子声部方向 = −父方向（§6/§16 σ_u=−σ_p）；根声部方向 = 证书方向（§5）。
    let final_dir = match parent {
        Some(pidx) => Voice::child_dir(voices[pidx].dir),
        None => cert_dir,
    };
    let g = gen_counter.entry(carrier).or_insert(0);
    *g += 1;
    let idx = voices.len();
    voices.push(Voice {
        carrier,
        dir: final_dir,
        parent,
        qty: 1.0, // §7/§13 Q_Θ=1 单位（诚实简化）
        generation: *g,
        entry_bar,
        bits: cert_bits, // 开仓证书 I_γ（μ(z) i_class，§P4 §5）
        level: carrier.level, // ℓ（μ(z) level 分量）
    });
    active.insert(idx);
    opened_this_bar.insert(carrier, idx);
    idx
}

/// 单遍状态机的产出（净额序列 + 估出的 μ 表 + 诊断 + 全部声部）。
struct PassResult {
    net_per_bar: Vec<f64>,
    mu_est: MuEstimator,
    voices: Vec<Voice>,
    diag_certs: usize,
    diag_host_miss: usize,
    diag_has_parent: usize,
    diag_parent_active_found: usize,
    diag_lift_pass: usize,
    /// χ 门否决的开仓证书数（μ(z)≤θ ∨ ¬RiskOK ∨ ¬ConflictOK）。Pass 1（χ≡1）恒 0。
    chi_rejected: usize,
}

/// 跑一遍 π_Θ^bsp 多声部状态机（§1-19）。
///
/// `chi_gate`：
/// - `None` ⟹ **χ≡1 全覆盖**（Pass 1）：每条 Lift 通过的证书都开仓（现有 pi_bsp_timing 语义），
///   同时累加 μ 表供 Pass 2 用。
/// - `Some((est, theta))` ⟹ **χ=1[μ>θ]**（Pass 2，§13）：开仓前用 Pass 1 的 μ 表 `est` 查 z 的
///   μ(z)，仅当 `chi_t(μ, θ, RiskOK, ConflictOK, empty=pass)` 为真才开。
///   - RiskOK：本 bin K_Θ=恒等（无杠杆/保证金约束，文件头诚实简化）⟹ RiskOK≡true。
///   - ConflictOK：carrier 配对 + anc_ok 结构性保证同 carrier 唯一声部（§9/§4）⟹ ConflictOK≡true。
///     ★诚实（no-patch）：mutex.rs 8 谓词互斥化是覆盖路径解释器的 C_j 裁决；本 bin 用 carrier
///     状态机已结构性保证唯一，**不**双重接入 mutex（那会与 carrier 配对产生两套互斥逻辑）。
///   - empty=pass=true：Pass 1 见过的 z 才有 μ；Pass 2 中**未见过的 z**默认放行（全覆盖兜底），
///     使 Pass 2 ⊆ Pass 1 的开仓集——过滤只可能移除（μ≤θ 的已见类），不会新增（保证 ΔN 可比）。
fn run_state_machine(
    bars: &[newchan_rust::theta_v0::types::Bar],
    prices: &[f64],
    config: &ThetaConfig,
    fee_rate: f64,
    chi_gate: Option<(&MuEstimator, f64)>,
) -> PassResult {
    let n = bars.len();
    let mut classifier = IncrementalClassifier::new(bars, config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    let mut seen_lifted: HashSet<(ElementId, i8)> = HashSet::new();
    let mut voices: Vec<Voice> = Vec::new();
    let mut active: HashSet<usize> = HashSet::new();
    let mut gen_counter: HashMap<ElementId, u32> = HashMap::new();
    let mut net_per_bar: Vec<f64> = vec![0.0; n];
    let mut mu_est = MuEstimator::new();
    let mut diag_certs = 0usize;
    let mut diag_host_miss = 0usize;
    let mut diag_has_parent = 0usize;
    let mut diag_parent_active_found = 0usize;
    let mut diag_lift_pass = 0usize;
    let mut chi_rejected = 0usize;

    for i in 0..n {
        let (classification, tower) = classifier.classify_at(i);
        let tree = extract_carrier_forest(&tower);
        let tree_idx = build_tree_endpoint_index(&tree);

        let mut new_certs: Vec<(
            ElementId,
            Option<ElementId>,
            Option<u32>,
            Option<usize>,
            i8,
            BspBits,
        )> = Vec::new();
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in &ls.bsp {
                if seen.insert((lvl, p.source_index, p.bits.class_index())) {
                    let c = cert_of(&p.bits);
                    if c.buy || c.sell {
                        diag_certs += 1;
                    }
                    let (host_parent_idx, _attached_dir, carrier_id) =
                        attach_bsp_carrier_indexed(&tree_idx, &tree, lvl as u32, p.source_index);
                    if (c.buy || c.sell) && carrier_id.is_none() {
                        diag_host_miss += 1;
                    }
                    let carrier = carrier_id
                        .unwrap_or(ElementId { level: lvl as u32, ordinal: p.source_index as u64 });
                    let parent_id = host_parent_idx.map(|pi| tree[pi].id);
                    if (c.buy || c.sell) && parent_id.is_some() {
                        diag_has_parent += 1;
                    }
                    let parent_carrier = attach_bsp_parent_carrier_indexed(
                        &tree_idx, &tree, lvl as u32, p.source_index,
                    );
                    let parent_level = parent_carrier.map(|(_, lc, _)| lc);
                    let parent_rho = parent_carrier.map(|(_, _, rho_c)| rho_c);
                    if c.buy {
                        new_certs.push((carrier, parent_id, parent_level, parent_rho, 1, p.bits));
                    }
                    if c.sell {
                        new_certs.push((carrier, parent_id, parent_level, parent_rho, -1, p.bits));
                    }
                }
            }
        }

        // ── §9 先平后开 A^raw=(A_t\D_t)∪O_t ──（出场证书严格按 carrier 配对，§5/§16）
        let mut closing: HashSet<usize> = HashSet::new();
        for &v in &active {
            let voice = &voices[v];
            let exit_dir = -voice.dir;
            let exit_cert = new_certs
                .iter()
                .any(|&(c, _, _, _, d, _)| c == voice.carrier && d == exit_dir);
            if should_close(exit_cert, true, false) {
                closing.insert(v);
            }
        }
        // D_t 应用：A_t \ D_t。真实平仓 ⟹ 喂 μ 估计器（τ_γ=i 出场证书命中，因果可测）。
        for &v in &closing {
            let voice = voices[v];
            let entry_px = prices[voice.entry_bar.min(n - 1)];
            let exit_px = prices[i]; // τ_γ=i：出场证书命中的当前 bar close（F_i-可测，非后视）
            let x_gamma = marginal_return(entry_px, exit_px, voice.qty, fee_rate, voice.dir);
            let z = MuClass::from_certificate(
                voice.level,
                voice.dir,
                voice.bits,
                parent_dir_of(&voices, v),
                if voice.parent.is_some() { PositionState::Child } else { PositionState::Root },
            );
            mu_est.observe(MuObservation { class: z, x_gamma });
            active.remove(&v);
        }

        // O_t：开仓声部集（§5 入场证书 δ(γ)=σ_v）。
        let active_voice_by_carrier: HashMap<ElementId, usize> =
            active.iter().map(|&v| (voices[v].carrier, v)).collect();
        let certs_by_carrier: HashMap<ElementId, (Option<ElementId>, i8, BspBits)> =
            new_certs.iter().map(|&(c, p, _lc, _rho, d, b)| (c, (p, d, b))).collect();

        for &(_carrier, parent_id, _lc, _rho, _cert_dir, _bits) in &new_certs {
            if let Some(pid) = parent_id {
                if active_voice_by_carrier.contains_key(&pid) {
                    diag_parent_active_found += 1;
                }
            }
        }
        let mut opened_this_bar: HashMap<ElementId, usize> = HashMap::new();
        for &(carrier, parent_id, parent_level, parent_rho, cert_dir, cert_bits) in &new_certs {
            // ── Lift 谓词链门（递归证明.pdf §2/§4）──
            if parent_id.is_some() {
                let lifts = match (parent_level, parent_rho) {
                    (Some(lc), Some(rc)) => {
                        let side = if cert_dir > 0 { Side::Long } else { Side::Short };
                        context_lifts(&classification, lc, rc, side)
                    }
                    _ => false,
                };
                if !lifts {
                    continue;
                }
                if !seen_lifted.insert((carrier, cert_dir)) {
                    continue;
                }
                diag_lift_pass += 1;
            }

            // ── χ_t(γ) 阈值过滤门（alpha2 §13，task #41）──
            // Pass 2（chi_gate=Some）：开仓前查 μ(z)，χ_t=1[μ>θ ∧ RiskOK ∧ ConflictOK] 才开。
            // z 的预构造：根声部父向=0/Root（cert_dir=最终方向）；子声部父向=父 carrier 的 active
            // voice.dir、方向=−父向、Child（§6/§16）。**与平仓喂 μ 时的 z 同口径**（同 MuClass 分量），
            // 保证 Pass 2 查的 μ 与 Pass 1 估的 μ 在同一 z 桶（否则查不到 ⟹ empty=pass 兜底）。
            if let Some((est, theta)) = chi_gate {
                // 预判该证书将开成的 z（不真开，只查 μ）。复用 open_carrier 的父链解析逻辑判方向。
                let parent_voice_dir = parent_id.and_then(|pid| {
                    active_voice_by_carrier.get(&pid).map(|&pi| voices[pi].dir)
                });
                let (z_dir, z_parent_dir, z_pos) = match parent_voice_dir {
                    Some(pdir) => (Voice::child_dir(pdir), pdir, PositionState::Child),
                    None => (cert_dir, 0, PositionState::Root),
                };
                let z = MuClass::from_certificate(
                    carrier.level, z_dir, cert_bits, z_parent_dir, z_pos,
                );
                // RiskOK≡true（K_Θ 恒等）、ConflictOK≡true（carrier+anc_ok 结构唯一）——见 fn 文档。
                // empty=pass=true：未见过的 z 放行（Pass 2 ⊆ Pass 1，过滤只移除已见的 μ≤θ 类）。
                if !chi_t(est.mu(&z), theta, true, true, true) {
                    chi_rejected += 1;
                    continue; // χ=0 ⟹ 不开（Γ_t^trade 排除该证书，§13 line 2248）。
                }
            }

            open_carrier(
                &mut voices, &mut active, &mut gen_counter, &active_voice_by_carrier,
                &certs_by_carrier, &mut opened_this_bar, carrier, parent_id, cert_dir, cert_bits, i,
            );
        }

        // ── §9 祖先闭合 A_{t+1}=AncOK(A^raw) ──（父不 active ⟹ 子声部被剪，§4）
        active = anc_ok(std::mem::take(&mut active), &voices);
        net_per_bar[i] = net_target(&active, &voices);
    }

    PassResult {
        net_per_bar,
        mu_est,
        voices,
        diag_certs,
        diag_host_miss,
        diag_has_parent,
        diag_parent_active_found,
        diag_lift_pass,
        chi_rejected,
    }
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        eprintln!(
            "用法: {} <SYMBOL> [START_DATE END_DATE]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO",
            args.first().map(String::as_str).unwrap_or("pi_bsp_timing")
        );
        return std::process::ExitCode::from(2);
    }
    let config = ThetaConfig::default();
    let full = match load_by_symbol(&args[1], &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let dataset = if args.len() == 4 {
        full.slice_date_window(&args[2], &args[3])
    } else {
        full
    };
    if dataset.bars.is_empty() {
        eprintln!("空数据集——无法回测（inconclusive）");
        return std::process::ExitCode::FAILURE;
    }

    let bars = &dataset.bars;
    let n = bars.len();
    let prices: Vec<f64> = bars.iter().map(|b| b.close as f64 * config.tick.tick_size).collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let years = (n as f64 / (365.25 * 24.0 * 60.0)).max(1e-9);

    // ── π_Θ^bsp 多声部状态机 ──
    // voices：所有曾创建的 position instance（append-only，parent 索引稳定）。
    // active：当前 A_t（§4 活动声部索引集）。
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // ★Fresh（递归证明.pdf §2，codex NO#3）：独立 Lift 记忆——防同一 (carrier,方向) 重复 Lift。
    // **不**复用 `seen`（seen 在证书发现去重，早于 Lift；会吞「先 U(g)=∅ 父未 live、后父成熟」的合法
    // 迟到 Lift）。seen_lifted 只在真正 Lift 成功（子声部开成）时记 ⟹ 迟到 Lift 仍可激活。
    let mut seen_lifted: HashSet<(ElementId, i8)> = HashSet::new();
    let mut voices: Vec<Voice> = Vec::new();
    let mut active: HashSet<usize> = HashSet::new();
    // 每 carrier 的代数计数（§3 n_v：同 carrier 多次开仓）。carrier=hostOf ElementId（644）。
    let mut gen_counter: HashMap<ElementId, u32> = HashMap::new();
    // 净额 N_t^bsp 逐 bar 序列（§12/§18 头寸）。
    let mut net_per_bar: Vec<f64> = vec![0.0; n];

    // ── μ(z,a) 类别条件边际收益估计器（alpha2 §12，task #39 mu-estimator）──
    // 每个声部 v 真实平仓（closing 触发 = 出场证书 δ=−σ_v 命中 τ_γ，**非**窗口强平）时构造一笔
    // MuObservation：z=(level,δ,I_γ,父向,短差,仓位态)，X_γ=δ(P_τ−P_t)−C 用真实 entry/exit close 价。
    // ★因果性（alpha2 §5 / project_bsp_direction_valid_friction_floor）：τ_γ=i 是出场证书命中的当前
    // bar（F_i-可测），P 用 prices[entry_bar]/prices[i]（confirm close，可成交价）——**非**端点后视。
    let mut mu_est = MuEstimator::new();
    // 单笔强平计数（窗口末端仍 active 的声部用末 bar 强平喂 μ，标注 forced——退出非证书触发，
    // 与真实 τ_γ 区分；codex 审退出时刻定义时此项需如实报，不混入「证书退出」μ）。
    let mut mu_forced_obs = 0usize;
    // 喂 μ 的辅助闭包载体：根声部父向=0；子声部父向=父 voice.dir（§16 σ_p）。
    let parent_dir_of = |voices: &[Voice], v: usize| -> i8 {
        voices[v].parent.map_or(0, |p| voices[p].dir)
    };

    // ── 子声部可达性诊断（647 第四根因坐实，L2 否定性结果）──
    // 647 诊断 π^bsp 子声部=0 根因为 bin 内三处逻辑（P3 出场优先 + 跨 carrier 方向匹配 +
    // 单次快照），并提修复路径（按 carrier 匹配 + host-miss 不丢弃）。本实装已落该修复，但子
    // 声部仍=0——揭示 647 未追到的第四根因：`extract_elements` 单最高级降序树只覆盖部分 L0 走势，
    // bsp 点大多落树外（orphan frontier 段）⟹ host-miss ⟹ ∂ 根声部。这些统计量化联合条件
    //（host-hit ∧ parent-container-voice-active）的真实有效域（231：L2 否定性结果有信息增量）。
    let mut diag_certs = 0usize; // 方向证书总数
    let mut diag_host_miss = 0usize; // hostOf 未命中（落 extract_elements 树外，∂ 根）
    let mut diag_has_parent = 0usize; // host 命中且有真 Compose 父容器
    let mut diag_parent_active_found = 0usize; // 父容器上有 active 声部（子声部可达的必要条件）
    let mut diag_lift_pass = 0usize; // ★严格可提升数（Context∧Fresh 双门通过的子声部 Lift，≤裸 host-hit）

    for i in 0..n {
        let (classification, tower) = classifier.classify_at(i);

        // ── 方案D（视图分离，裁决648）：host^op 用 **K_i 操作 carrier forest**（非 T_i）──
        // 旧版 `extract_elements`=T_i=↓r_i（只展开最高非空级别根，覆盖 ~36% L0）⟹ bsp 大量落 orphan
        // frontier ⟹ host-miss ⟹ ∂ 根声部 ⟹ 子声部恒 0（PDF §8/§11，line 188-190 旧诊断坐实）。
        // `extract_carrier_forest`=K_i=U_i=全量 tower 元素（endpoint-complete：∀bsp ∃c ρ(c)=s(g)）⟹
        // host^op 不再 miss ⟹ 子声部 carrier/parent_id 真命中（PDF §3/§19，host 严格右端点命中不变）。
        // 每个 `LeveledMove` 是元素 e；其 `sub_moves` 是真 Compose 子（push_element_tree 真父子）。
        // hostOf 的 `parent_id`（真 Compose 父容器 ElementId）= 子声部的父声部 carrier（§9.1）。
        let tree = extract_carrier_forest(&tower);
        let tree_idx = build_tree_endpoint_index(&tree);

        // 本 bar 新确认的方向证书（§1-2 γ）。每条证书携：
        // (carrier=hostOf ElementId, parent_id=hostOf 真 Compose 父容器, σ_p=父方向)。
        // append-only seen-set diff（644：与 runner newly_confirmed_step 同语义，无 look-ahead）。
        // (carrier, parent_id, dir)：dir 是证书方向（买 +1 / 卖 −1），由 §5/§16 在 open 时按是否有父决定 final_dir。
        // 元组：(carrier, parent_id, parent_level ℓ_c, parent_rho ρ_c, dir)。(ℓ_c,ρ_c) = 上级
        // carrier c 坐标（Lift.Context 投影去 levels[ℓ_c].bsp 找 g 证成上级第 j 类的已确认买卖点）。
        // 元组末位 `BspBits` = 该证书的 I_γ 类别（μ(z) i_class 分量，§P4 §5 不压扁）。
        let mut new_certs: Vec<(
            ElementId,
            Option<ElementId>,
            Option<u32>,
            Option<usize>,
            i8,
            BspBits,
        )> = Vec::new();
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in &ls.bsp {
                if seen.insert((lvl, p.source_index, p.bits.class_index())) {
                    let c = cert_of(&p.bits);
                    if c.buy || c.sell {
                        diag_certs += 1;
                    }
                    // 638 hostOf 附着（生产同口径 interp.rs:498）：carrier 容器 hostOf(g) id（§13/§14
                    // 工位 H：carrier 容器本身作 position node，非叶子 ordinal）。host-miss ⟹ carrier_id=None
                    // ⟹ 叶子 ordinal id（边界胚元 ∂ = 根声部，**不**丢弃——旧版 continue 丢 97.6% 证书的根因）。
                    let (host_parent_idx, _attached_dir, carrier_id) =
                        attach_bsp_carrier_indexed(&tree_idx, &tree, lvl as u32, p.source_index);
                    if (c.buy || c.sell) && carrier_id.is_none() {
                        diag_host_miss += 1;
                    }
                    // carrier：hostOf 容器 id（找到）/ 叶子 ordinal id（host-miss = ∂ 根，§14 退化）。
                    // 同 carrier 同 bar 多 bsp 共享 id（§14 简化，与生产 interp.rs:516 同口径）。
                    let carrier = carrier_id
                        .unwrap_or(ElementId { level: lvl as u32, ordinal: p.source_index as u64 });
                    // 父声部 carrier = hostOf 的真 Compose 父容器 ElementId（§9.1 par_C(κ(u))=κ(v)）。
                    let parent_id = host_parent_idx.map(|pi| tree[pi].id);
                    if (c.buy || c.sell) && parent_id.is_some() {
                        diag_has_parent += 1;
                    }
                    // 上级 carrier c 的 (level_c, rho_c)——Lift.Context 投影载体（codex 口径 A）。
                    let parent_carrier = attach_bsp_parent_carrier_indexed(
                        &tree_idx, &tree, lvl as u32, p.source_index,
                    );
                    let parent_level = parent_carrier.map(|(_, lc, _)| lc);
                    let parent_rho = parent_carrier.map(|(_, _, rho_c)| rho_c);
                    if c.buy {
                        new_certs.push((carrier, parent_id, parent_level, parent_rho, 1, p.bits));
                    }
                    if c.sell {
                        new_certs.push((carrier, parent_id, parent_level, parent_rho, -1, p.bits));
                    }
                }
            }
        }

        // ── §9 先平后开 A^raw=(A_t\D_t)∪O_t ──
        // D_t：出场声部集（§5 出场证书 δ(γ)=−σ_v）。出场证书 = **同 carrier** 上的反向证书
        // （声部级 §5 δ=−σ_v，落在产出该声部的走势容器上）。
        //
        // 644 修复（根因 #3，task close-优先误平）：旧版把**任意**反向证书当出场（global any_sell），
        // 导致父根声部被同 bar 出现的反向证书平掉 → §16 短差子声部永无机会附着（子声部恒 0）。
        // 短差子声部（§16）是父持仓**期内**的对冲腿——反向证书应在**子 carrier**开子声部，**不**平父声部。
        // 故出场证书严格按 carrier 配对：声部 v 的出场 = v 自身 carrier 上的反向证书（其他 carrier 反向
        // 证书是开新声部/子声部的入场触发，与 v 出场无关）。
        let mut closing: HashSet<usize> = HashSet::new();
        for &v in &active {
            let voice = &voices[v];
            // 出场方向证书：dir>0（多头）出场=卖证书（−1）；dir<0（空头）出场=买证书（+1）。
            let exit_dir = -voice.dir;
            let exit_cert = new_certs
                .iter()
                .any(|&(c, _, _, _, d, _)| c == voice.carrier && d == exit_dir);
            // §10-11 close 优先：风险关闭(本实装 risk_close=false, K_Θ 恒等)/父关闭/出场证书。
            // 父关闭在 anc_ok 阶段级联处理，此处判出场证书（P3，同 carrier 反向）。
            if should_close(exit_cert, true, false) {
                closing.insert(v);
            }
        }
        // D_t 应用：A_t \ D_t。真实平仓 ⟹ 喂 μ 估计器（τ_γ=i 出场证书命中，因果可测）。
        for &v in &closing {
            let voice = voices[v];
            let entry_px = prices[voice.entry_bar.min(n - 1)];
            let exit_px = prices[i]; // τ_γ=i：出场证书命中的当前 bar close（F_i-可测，非后视）
            let x_gamma = marginal_return(entry_px, exit_px, voice.qty, fee_rate, voice.dir);
            let z = MuClass::from_certificate(
                voice.level,
                voice.dir,
                voice.bits,
                parent_dir_of(&voices, v),
                if voice.parent.is_some() { PositionState::Child } else { PositionState::Root },
            );
            mu_est.observe(MuObservation { class: z, x_gamma });
            active.remove(&v);
        }

        // O_t：开仓声部集（§5 入场证书 δ(γ)=σ_v）。
        // 根声部（§5，host 父容器=∂，parent_id=None）：买侧证书 → 开多头根声部（σ_v=+1）；
        //   卖侧证书 → 开空头根声部（σ_v=−1）。
        // 短差子声部（§6/§16，host 有真 Compose 父容器 parent_id）：父声部 = parent_id carrier 上的
        //   active 声部；子声部 σ_u=−σ_p（结构性，与同 bar 是否有反向证书无关——644 修复核心）。
        //
        // 644 结构父子：父子关系由 hostOf 的真 Compose 父容器 `parent_id`（§9.1 par_C）决定，
        // 不再由「同 bar 同时出现的反向证书」伪造（旧版子声部恒 0 的根因）。父声部 = parent_id 容器
        // 上当前 active 的声部（§4 父 active 子才 active；anc_ok 阶段对位）。

        // 当前每 carrier 上的 active 声部索引（用于按 parent_id 反查父声部）。同 carrier 多声部取首个
        // （generation 区分；anc_ok 用 parent 索引链剪枝，首个 active 即代表该容器的持仓节点）。
        let active_voice_by_carrier: HashMap<ElementId, usize> =
            active.iter().map(|&v| (voices[v].carrier, v)).collect();

        // ★方案D §9（裁决648 / codex YES#3）：解释器**生成父 voice**（接受父子证书链），不靠子反推父。
        // 本 bar 证书按 carrier 建表（carrier→(parent_id, cert_dir)）——open 子声部时，若其父 carrier
        // **本 bar 也有证书**（父子证书链 γ_0..γ_d 同 bar 存在）或父已 live，则**同时开父 voice**再开子。
        // codex 严格条件：父 voice 进 O_i **仅因**父证书同 bar 或父已 live（**非** AncOK 生成父——AncOK
        // 仍是过滤器只剪孤儿子，§6/§7）。沿真 Compose parent 链上溯，对每个有同 bar 证书的祖先开 voice。
        let certs_by_carrier: HashMap<ElementId, (Option<ElementId>, i8, BspBits)> =
            new_certs.iter().map(|&(c, p, _lc, _rho, d, b)| (c, (p, d, b))).collect();

        // open_carrier：确保 carrier 上有 active voice（已 live ⟹ 复用首个；否则沿父链先开父再开本级）。
        // 返回该 carrier 的 active voice 索引。父链上溯到 ∂（parent_id=None）或无同 bar 证书的祖先为止。
        fn open_carrier(
            voices: &mut Vec<Voice>,
            active: &mut HashSet<usize>,
            gen_counter: &mut HashMap<ElementId, u32>,
            active_voice_by_carrier: &HashMap<ElementId, usize>,
            certs_by_carrier: &HashMap<ElementId, (Option<ElementId>, i8, BspBits)>,
            opened_this_bar: &mut HashMap<ElementId, usize>,
            carrier: ElementId,
            parent_id: Option<ElementId>,
            cert_dir: i8,
            cert_bits: BspBits,
            entry_bar: usize,
        ) -> usize {
            // 已 live（上一 bar 留存）⟹ 复用（§9 父已 live 分支）。
            if let Some(&idx) = active_voice_by_carrier.get(&carrier) {
                return idx;
            }
            // 本 bar 已开过该 carrier ⟹ 复用（防同 bar 重复开，父链多子共享父）。
            if let Some(&idx) = opened_this_bar.get(&carrier) {
                return idx;
            }
            // 父声部：parent_id 有真父 ∧（父已 live 或父本 bar 有证书）⟹ 递归先开父（§9 生成父 voice）。
            // ★Lift.Nest（递归证明.pdf §2.2，codex YES#1）：J_ℓ(d)⊆J_{ℓ+1}(c)（子区间⊆父区间）。
            // K_i 真嵌套（push_element_tree 子 sub_moves 挂父，compose 父区间取首尾子）下**恒满足**
            // ⟹ debug_assert 守卫足够（生产域无反例；只防手工乱序/伪造 LeveledMove）。本 bin 在 K_i 上
            // 建 carrier，子 carrier 与父 carrier 的区间⊆关系由 extract_carrier_forest 真嵌套保证。
            let parent = parent_id.and_then(|pid| {
                if active_voice_by_carrier.contains_key(&pid)
                    || opened_this_bar.contains_key(&pid)
                    || certs_by_carrier.contains_key(&pid)
                {
                    let (ppid, pdir, pbits) =
                        certs_by_carrier.get(&pid).copied().unwrap_or((None, cert_dir, cert_bits));
                    Some(open_carrier(
                        voices, active, gen_counter, active_voice_by_carrier,
                        certs_by_carrier, opened_this_bar, pid, ppid, pdir, pbits, entry_bar,
                    ))
                } else {
                    None // 父无同 bar 证书且未 live ⟹ 不生成父（codex 严格条件）⟹ 本级作根声部。
                }
            });
            // 子声部方向 = −父方向（§6/§16 σ_u=−σ_p）；根声部方向 = 证书方向（§5）。
            let final_dir = match parent {
                Some(pidx) => Voice::child_dir(voices[pidx].dir),
                None => cert_dir,
            };
            let g = gen_counter.entry(carrier).or_insert(0);
            *g += 1;
            let idx = voices.len();
            voices.push(Voice {
                carrier,
                dir: final_dir,
                parent,
                qty: 1.0, // §7/§13 Q_Θ=1 单位（诚实简化）
                generation: *g,
                entry_bar,
                bits: cert_bits, // 开仓证书 I_γ（μ(z) i_class，§P4 §5）
                level: carrier.level, // ℓ（μ(z) level 分量）
            });
            active.insert(idx);
            opened_this_bar.insert(carrier, idx);
            idx
        }

        for &(_carrier, parent_id, _lc, _rho, _cert_dir, _bits) in &new_certs {
            if let Some(pid) = parent_id {
                if active_voice_by_carrier.contains_key(&pid) {
                    diag_parent_active_found += 1;
                }
            }
        }
        let mut opened_this_bar: HashMap<ElementId, usize> = HashMap::new();
        for &(carrier, parent_id, parent_level, parent_rho, cert_dir, cert_bits) in &new_certs {
            // ── Lift 谓词链门（递归证明.pdf §2/§4：Lift=Host∧Nest∧Context∧Fresh，全 1 ⟹ U(g)≠∅）──
            // Host：carrier 命中 hostOf（new_certs 已是命中结果）。Nest：K_i 真嵌套自动满足（debug_assert）。
            // 有真父的证书 = 候选子声部 g（要 Lift 到上级 c）。无真父 = 根声部证书，无需 Lift 门，直接开根。
            if parent_id.is_some() {
                // ★Context^δ_j（codex 口径 A）：g 必须证成上级 carrier c 第 j 类（上级 ℓ_c 在 ρ_c 处
                // 已独立分类出同向 δ 买卖点）。投影坐标 (ℓ_c=parent_level, ρ_c=parent_rho) 在证书收集
                // 时由 attach_bsp_parent_carrier_indexed 取真 Compose 父 carrier 的 level/rho。
                let lifts = match (parent_level, parent_rho) {
                    (Some(lc), Some(rc)) => {
                        // δ：子声部要 Lift 的证书方向 g（codex 审点 #2=σ_u/证书方向，非父 σ_p）。
                        let side = if cert_dir > 0 { Side::Long } else { Side::Short };
                        context_lifts(&classification, lc, rc, side)
                    }
                    _ => false, // ℓ_c/ρ_c 缺失（host 父坐标丢失）⟹ Context 不可判 ⟹ 不可提升。
                };
                if !lifts {
                    continue; // Context 失败 ⟹ g 不可提升 ⟹ 不开子、不开父、不降级根（codex 审点 #3）。
                }
                // ★Fresh（codex NO#3）：独立 seen_lifted 防同一 Lift（carrier+方向）重复触发——
                // **不**复用 seen（seen 早于 Lift 在证书发现去重，会吞「先 U(g)=∅ 后成熟」的合法迟到 Lift）。
                // seen_lifted 只在真正 Lift 成功时记，故迟到 Lift（父成熟后）仍可激活。
                if !seen_lifted.insert((carrier, cert_dir)) {
                    continue; // 该 (carrier,方向) 已 Lift 过 ⟹ Fresh 失败 ⟹ 跳过（不重复开子声部）。
                }
                diag_lift_pass += 1;
            }
            open_carrier(
                &mut voices, &mut active, &mut gen_counter, &active_voice_by_carrier,
                &certs_by_carrier, &mut opened_this_bar, carrier, parent_id, cert_dir, cert_bits, i,
            );
        }

        // ── §9 祖先闭合 A_{t+1}=AncOK(A^raw) ──（父不 active ⟹ 子声部被剪，§4）
        active = anc_ok(std::mem::take(&mut active), &voices);

        // §12 净额 N_t^bsp = Σ σ_v q_v（声部状态机持仓净额）。
        net_per_bar[i] = net_target(&active, &voices);
    }

    // ── 窗口末端仍 active 的声部强平喂 μ（退出非证书触发，标注 forced，与真实 τ_γ 区分）──
    // ★诚实（formalization-validity-domain）：forced 退出不是 alpha2 §12 的 τ_γ=出场证书时刻，
    // 是窗口边界人为强平。混入会污染「证书退出」μ 的因果语义——故单独计数 mu_forced_obs 如实报，
    // 但仍喂入 μ 桶（否则末端未平声部的已实现浮盈被丢弃，低估持仓期收益）。codex 审退出定义时需裁。
    let last_bar = n - 1;
    for v in 0..voices.len() {
        if active.contains(&v) {
            let voice = voices[v];
            if last_bar > voice.entry_bar {
                let entry_px = prices[voice.entry_bar.min(last_bar)];
                let exit_px = prices[last_bar];
                let x_gamma = marginal_return(entry_px, exit_px, voice.qty, fee_rate, voice.dir);
                let z = MuClass::from_certificate(
                    voice.level,
                    voice.dir,
                    voice.bits,
                    parent_dir_of(&voices, v),
                    if voice.parent.is_some() { PositionState::Child } else { PositionState::Root },
                );
                mu_est.observe(MuObservation { class: z, x_gamma });
                mu_forced_obs += 1;
            }
        }
    }

    // ── §18 r^bsp = N^bsp·ΔP − C：逐 bar net-position MtM 权益曲线（同 π^cov NAV 口径）──
    // N_t^bsp 是连续净额（非 ±1 toggle）；equity 归一化初始=1。成本在净额变动 bar 按 |ΔN| 名义额扣。
    let nav_base: f64 = {
        // 名义基准 = 净额峰值 × 均价量级（与 significance nav_base 同口径：名义额，方向无关）。
        let peak = net_per_bar.iter().fold(0.0f64, |a, &x| a.max(x.abs())).max(1.0);
        let avg_px = prices.iter().sum::<f64>() / n.max(1) as f64;
        (peak * avg_px * (1.0 + fee_rate)).max(1e-9)
    };
    let mut equity_curve: Vec<f64> = Vec::with_capacity(n);
    let mut daily_returns: Vec<f64> = Vec::with_capacity(n);
    let mut cum_abs = 0.0;
    for i in 0..n {
        if i > 0 {
            let dp = prices[i] - prices[i - 1];
            cum_abs += net_per_bar[i - 1] * dp; // N_{t-1}·ΔP（§18）
            // 净额变动名义额扣双边费（§18 −C）。
            let dn = (net_per_bar[i] - net_per_bar[i - 1]).abs();
            cum_abs -= dn * prices[i] * fee_rate;
        }
        let eq = 1.0 + cum_abs / nav_base;
        let ret = if i == 0 {
            0.0
        } else {
            let prev = *equity_curve.last().unwrap();
            if prev.abs() > 1e-12 { eq / prev - 1.0 } else { 0.0 }
        };
        equity_curve.push(eq);
        daily_returns.push(ret);
    }

    // ── 逐笔 TradeRecord（§4 声部生命周期：每声部 [entry_bar, 平仓 bar) 配对）──
    // 声部状态机的「交易」= 每个声部的开→平。重建：扫 voices，按 dir 产多/空笔。
    // 平仓 bar：声部从 active 移除的 bar（本实装窗口终点统一强平未平声部，与工位 L forced_close 一致）。
    // 简化：声部平仓点用净额过零段近似不可靠 ⟹ 用声部 entry_bar + 窗口终点强平（保守，含浮盈）。
    // ★诚实：随机对照需逐笔 entry/exit；声部级精确平仓点需在状态机内记录（本实装记 entry，
    //   平仓在 closing 时未单独存 exit_bar）。为给 significance 提供 trades，按声部 entry→末 bar 强平。
    let mut trades: Vec<TradeRecord> = Vec::new();
    for v in &voices {
        if n - 1 > v.entry_bar {
            trades.push(TradeRecord {
                entry_bar: v.entry_bar,
                exit_bar: n - 1,
                hold_bars: n - 1 - v.entry_bar,
                qty: v.qty,
                long: v.dir > 0,
                forced_close: true,
            });
        }
    }
    let trade_pnls: Vec<f64> = trades
        .iter()
        .map(|t| {
            let e = prices[t.entry_bar.min(n - 1)];
            let x = prices[t.exit_bar.min(n - 1)];
            let raw = if t.long {
                t.qty * (x * (1.0 - fee_rate) - e * (1.0 + fee_rate))
            } else {
                t.qty * (e * (1.0 - fee_rate) - x * (1.0 + fee_rate))
            };
            raw / nav_base
        })
        .collect();

    let bh_return =
        if n >= 2 && prices[0].abs() > 1e-12 { prices[n - 1] / prices[0] - 1.0 } else { 0.0 };
    let m = metrics::compute(&equity_curve, &daily_returns, &trade_pnls, years, bh_return);
    let sig = metrics::significance(&trade_pnls, &daily_returns, &trades, &prices, fee_rate, m.strat_return);

    let n_voices = voices.len();
    let n_long = voices.iter().filter(|v| v.dir > 0).count();
    let n_children = voices.iter().filter(|v| v.parent.is_some()).count();
    let max_net = net_per_bar.iter().fold(0.0f64, |a, &x| a.max(x.abs()));

    println!("=== π_Θ^bsp 买卖点多声部离散择时回测（忠实 PDF §1-19）===");
    println!("品种            : {}", dataset.symbol);
    println!("bar 数          : {n}");
    println!("年化基数(years) : {years:.4}");
    println!("--- 声部状态机（§3 position instance / §12 N^bsp=Σσ_v q_v）---");
    println!("声部总数        : {n_voices}（多 {n_long} / 空 {}）", n_voices - n_long);
    println!("短差子声部数    : {n_children}（§6/§16 σ_u=−σ_p）");
    println!("净额峰值 |N^bsp|: {max_net:.2}");
    println!("--- 子声部可达性诊断（647 第四根因坐实，L2 否定性结果）---");
    let host_hit = diag_certs.saturating_sub(diag_host_miss);
    let hit_pct = if diag_certs > 0 { 100.0 * host_hit as f64 / diag_certs as f64 } else { 0.0 };
    println!("方向证书总数    : {diag_certs}");
    println!("hostOf 命中     : {host_hit}（{hit_pct:.1}%；其余落 extract_elements 树外=∂ 根）");
    println!("命中且有父容器  : {diag_has_parent}（host 有真 Compose 父）");
    println!("父容器有 active : {diag_parent_active_found}（子声部可达必要条件——0 ⟹ 子声部结构性不可达）");
    println!("★严格可提升 Lift: {diag_lift_pass}（Context^δ_j∧Fresh 双门通过——收紧后真子声部；≤裸 host-hit）");
    println!("--- 指标（§18 r^bsp=N^bsp·ΔP−C，同工位 L NAV 口径，可比 π^cov）---");
    println!("★Sharpe         : {:.4}", m.sharpe);
    println!("strat_return    : {:.4}", m.strat_return);
    println!("buy&hold_return : {:.4}", m.bh_return);
    println!("Sortino         : {:.4}", m.sortino);
    println!("MaxDrawdown     : {:.4}", m.max_drawdown);
    println!("win_rate        : {:.4}", m.win_rate);
    println!("profit_factor   : {:.4}", m.profit_factor);
    println!("--- 同口径随机对照（§4 协议，seed 冻结）---");
    println!("theta_same_caliber : {:.6}", sig.theta_return_same_caliber);
    println!("shift  mean/p   : {:.6} / {:.4}", sig.shift_mean_return, sig.shift_pvalue);
    println!("indep  mean/p   : {:.6} / {:.4}", sig.indep_mean_return, sig.indep_pvalue);
    println!("beats_random    : {}", sig.theta_beats_random);
    println!("controls_degen  : {}", sig.controls_degenerate);
    // ── μ(z,a) 类别条件边际收益表（alpha2 §12，task #39，L2 真实数据可否证）──
    // 按 z=(ℓ,δ,I_γ,父向,短差,仓位态) 分桶；μ(z)=E[X_γ|z]=ΣX_γ/|S_z| 绝对收益（与 metrics 同单位）。
    // ★μ(z)>0 ⟹ 该类买卖点在此退出规则/成本/样本下有正边际期望（alpha 候选）；μ(z)≤0 ⟹ 无正期望
    // （§12 line 2186）——否定性结果（formalization-validity-domain：μ≤0 比确认更有信息增量）。
    let mut mu_rows: Vec<(MuClass, f64, u64)> = mu_est
        .iter_mu()
        .map(|(z, mu)| (z, mu, mu_est.count(&z)))
        .collect();
    // 按 μ 降序（正期望在前），便于读 alpha 候选。
    mu_rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let n_pos = mu_rows.iter().filter(|r| r.1 > 0.0).count();
    let n_nonpos = mu_rows.len() - n_pos;
    println!("--- μ(z,a) 类别条件边际收益表（alpha2 §12，task #39 mu-estimator）---");
    println!(
        "z 类总数        : {}（μ>0: {n_pos} alpha候选 / μ≤0: {n_nonpos} 无正期望）",
        mu_rows.len()
    );
    println!("强平观测(forced): {mu_forced_obs}（退出非证书τ_γ，窗口边界强平——与证书退出μ混桶，codex待裁）");
    if mu_rows.is_empty() {
        println!("（无平仓观测——无声部生命周期闭合，μ 表空，inconclusive）");
    } else {
        println!(
            "{:>3} {:>3} {:>4} {:>5} {:>6} {:>6} {:>6} {:>14}",
            "ℓ", "δ", "Iγ", "父向", "短差", "仓位", "|S_z|", "μ(z)绝对"
        );
        for (z, mu, cnt) in &mu_rows {
            let alpha = if *mu > 0.0 { "+" } else { "≤0" };
            println!(
                "{:>3} {:>3} {:>4} {:>5} {:>6} {:>6} {:>6} {:>14.4} {alpha}",
                z.level,
                z.delta,
                z.i_class,
                z.parent_dir,
                if z.short_swing { "短差" } else { "顺势" },
                match z.position {
                    PositionState::Root => "根",
                    PositionState::Child => "子",
                },
                cnt,
                mu,
            );
        }
    }
    println!("--- 认识论 L2（真实数据，可否证；§18 alpha 判定）---");
    if voices.is_empty() {
        println!("等级: L1（无买卖点确认 ⟹ 无声部，inconclusive）");
    } else if n_children == 0 {
        // 647 第四根因：647 修复（按 carrier 匹配 + host-miss 不丢弃）已实装，子声部仍=0。
        // ⟹ N^bsp 退化为纯根声部方向序列，§6/§16 多空双开/短差对冲层**结构性不可达**。
        println!(
            "等级: L2（否定性结果）——647 修复已落但子声部=0：N^bsp 退化纯根声部。\n      \
             根因（647 第四项）：extract_elements 单最高级降序树仅覆盖部分 L0 走势，\n      \
             bsp 点 {hit_pct:.1}% 命中 hostOf，父容器有 active=0 ⟹ §16 多空双开不可达。\n      \
             Sharpe={:.4} 仅反映纯根声部退化形态，**不**反映多声部对冲（不得用作对冲无 alpha 的 L2 结论）。",
            m.sharpe
        );
    } else {
        println!("等级: L2——π^bsp Sharpe={:.4} 子声部={n_children} 多声部对冲真激活（§19 goal#5）", m.sharpe);
    }
    std::process::ExitCode::SUCCESS
}
