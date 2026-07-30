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
//! cargo run --release --features backtest_bin --bin pi_bsp_timing -- <SYMBOL> [START END [THETA [Z_ALPHA]]]
//! ```
//! #412 修复：`START END` 窗口切片只取决于是否传了这两个参数，与是否额外带 THETA/Z_ALPHA
//! **无关**——旧版用 `args.len()==4` 判断是否切片，THETA 一出现 `args.len()` 变 5，窗口切片
//! 被无声跳过、退化为跑全量数据集（OKLO 全量 34 万 bar，233s vs 窗口化 0.3s，#412）。见 `parse_cli_args`。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::backtest::metrics::{self, TradeRecord};
use newchan_rust::theta_v0::backtest::mu_estimator::{
    chi_dimension_three_return, MuClass, MuEstimator, MuObservation, PositionState,
};
use newchan_rust::theta_v0::backtest::selector::chi_open_gate_lcb;
use newchan_rust::theta_v0::classifier::recursive_tower::ElementId;
use newchan_rust::theta_v0::classifier::Classification;
use newchan_rust::theta_v0::config::ThetaConfig;
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
    CertHit {
        buy: b.conf_plus(),
        sell: b.conf_minus(),
    }
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
            None => true,                     // 根声部（⊥）无祖先，恒满足
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
    active
        .iter()
        .map(|&v| voices[v].dir as f64 * voices[v].qty)
        .sum()
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
            let (ppid, pdir, pbits) = certs_by_carrier
                .get(&pid)
                .copied()
                .unwrap_or((None, cert_dir, cert_bits));
            Some(open_carrier(
                voices,
                active,
                gen_counter,
                active_voice_by_carrier,
                certs_by_carrier,
                opened_this_bar,
                pid,
                ppid,
                pdir,
                pbits,
                entry_bar,
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
        bits: cert_bits,      // 开仓证书 I_γ（μ(z) i_class，§P4 §5）
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
    /// #563 M7：entry_px≤0 被跳过喂 μ 的笔数（真实标的池预期恒 0，见 mu_estimator.rs 的
    /// `chi_dimension_three_return` 前置条件文档）。
    diag_nonpositive_px: usize,
}

/// 跑一遍 π_Θ^bsp 多声部状态机（§1-19）。
///
/// `chi_gate`：
/// - `None` ⟹ **χ≡1 全覆盖**（Pass 1）：每条 Lift 通过的证书都开仓（现有 pi_bsp_timing 语义），
///   同时累加 μ 表供 Pass 2 用。
/// - `Some((est, theta, z_alpha))` ⟹ **χ=1[LCB(μ)>θ]**（Pass 2，§13 + p25 §12 LCB 升级）：开仓前
///   用 Pass 1 的 μ/std 表 `est` 查 z 的 **LCB(μ)=mean−z_alpha·std/√n**（非裸 μ，防高维 z 过拟合），
///   仅当 [`chi_open_gate_lcb`]`(est, z, θ, z_alpha, RiskOK, ConflictOK, empty=pass)` 为真才开（封装内自算
///   LCB(μ)=mean−z_alpha·std/√n，非外部传入）。`z_alpha=0` ⟹ LCB=mean ⟹ 退化
///   回裸 μ 门（bit-exact 现有验收）。n<2 单样本 ⟹ mu_lcb=None ⟹ 走 empty=pass 分支（与未见 z 合流）。
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
    chi_gate: Option<(&MuEstimator, f64, f64)>,
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
    let mut diag_nonpositive_px = 0usize;

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
            for p in ls.bsp.iter() {
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
                    let carrier = carrier_id.unwrap_or(ElementId {
                        level: lvl as u32,
                        ordinal: p.source_index as u64,
                    });
                    let parent_id = host_parent_idx.map(|pi| tree[pi].id);
                    if (c.buy || c.sell) && parent_id.is_some() {
                        diag_has_parent += 1;
                    }
                    let parent_carrier = attach_bsp_parent_carrier_indexed(
                        &tree_idx,
                        &tree,
                        lvl as u32,
                        p.source_index,
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
                                     // #563 M7：entry_px≤0 ⟹ chi_dimension_three_return 的量纲③分母 entry_notional≤0，
                                     // release 下 debug_assert 失守会让 Inf/NaN 进 Welford 永久污染整桶——显式守卫跳过该笔，
                                     // 与 l3_delta_r_alpha.rs 的 LedgerDisposition::NonPositivePx 同一防线（本 bin 无 ledger 层，
                                     // 守卫落在 μ 喂入点）。不跳过平仓本身（voice 仍须从 active 移除，只是不喂 μ）。
            if entry_px > 0.0 {
                let x_gamma =
                    chi_dimension_three_return(entry_px, exit_px, voice.qty, fee_rate, voice.dir);
                let z = MuClass::from_certificate(
                    voice.level,
                    voice.dir,
                    voice.bits,
                    parent_dir_of(&voices, v),
                    if voice.parent.is_some() {
                        PositionState::Child
                    } else {
                        PositionState::Root
                    },
                );
                mu_est.observe(MuObservation { class: z, x_gamma });
            } else {
                diag_nonpositive_px += 1;
            }
            active.remove(&v);
        }

        // O_t：开仓声部集（§5 入场证书 δ(γ)=σ_v）。
        let active_voice_by_carrier: HashMap<ElementId, usize> =
            active.iter().map(|&v| (voices[v].carrier, v)).collect();
        let certs_by_carrier: HashMap<ElementId, (Option<ElementId>, i8, BspBits)> = new_certs
            .iter()
            .map(|&(c, p, _lc, _rho, d, b)| (c, (p, d, b)))
            .collect();

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
                        let side = if cert_dir > 0 {
                            Side::Long
                        } else {
                            Side::Short
                        };
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
            if let Some((est, theta, z_alpha)) = chi_gate {
                // 预判该证书将开成的 z（不真开，只查 μ）。复用 open_carrier 的父链解析逻辑判方向。
                let parent_voice_dir = parent_id
                    .and_then(|pid| active_voice_by_carrier.get(&pid).map(|&pi| voices[pi].dir));
                let (z_dir, z_parent_dir, z_pos) = match parent_voice_dir {
                    Some(pdir) => (Voice::child_dir(pdir), pdir, PositionState::Child),
                    None => (cert_dir, 0, PositionState::Root),
                };
                let z =
                    MuClass::from_certificate(carrier.level, z_dir, cert_bits, z_parent_dir, z_pos);
                // RiskOK≡true（K_Θ 恒等）、ConflictOK≡true（carrier+anc_ok 结构唯一）——见 fn 文档。
                // empty=pass=true：未见 z 或 n<2 单样本（mu_lcb=None）放行（Pass 2 ⊆ Pass 1，过滤只
                // 移除已见 n≥2 的 LCB≤θ 类）。准入用 LCB(μ) 而非裸 μ（p25 §12 防高维 z 过拟合）。
                // #394 接线：改调 selector::chi_open_gate_lcb 封装（原地 chi_t(est.mu_lcb(..)) 内联
                // 组合与该封装体内表达式逐项一致，见函数体 `chi_t(est.mu_lcb(z, z_alpha), theta,
                // risk_ok, conflict_ok, treat_empty_as_pass)`——risk_ok=true/conflict_ok=true/
                // treat_empty_as_pass=true 代入后与本调用点原表达式按变量逐项相同，bit-exact）。
                if !chi_open_gate_lcb(est, &z, theta, z_alpha, true, true, true) {
                    chi_rejected += 1;
                    continue; // χ=0 ⟹ 不开（Γ_t^trade 排除该证书，§13 line 2248）。
                }
            }

            open_carrier(
                &mut voices,
                &mut active,
                &mut gen_counter,
                &active_voice_by_carrier,
                &certs_by_carrier,
                &mut opened_this_bar,
                carrier,
                parent_id,
                cert_dir,
                cert_bits,
                i,
            );
        }

        // ── §9 祖先闭合 A_{t+1}=AncOK(A^raw) ──（父不 active ⟹ 子声部被剪，§4）
        active = anc_ok(std::mem::take(&mut active), &voices);
        net_per_bar[i] = net_target(&active, &voices);
    }

    // ── 窗口末端仍 active 的声部强平喂 μ（forced，与真实 τ_γ 区分）──
    // ★诚实（formalization-validity-domain）：forced 退出不是 §12 τ_γ=出场证书时刻，是窗口边界
    // 人为强平。仍喂 μ 桶（否则末端浮盈被丢弃低估收益），但属 forced——本 bin 不单独区分桶
    // （codex 审退出定义时可裁）。Pass 1/Pass 2 同口径处理，保证 μ 表可比。
    let last_bar = n - 1;
    for v in 0..voices.len() {
        if active.contains(&v) {
            let voice = voices[v];
            if last_bar > voice.entry_bar {
                let entry_px = prices[voice.entry_bar.min(last_bar)];
                let exit_px = prices[last_bar];
                // #563 M7：同上——entry_px≤0 时不喂 μ，只计数（守卫见平仓分支注释）。
                if entry_px > 0.0 {
                    let x_gamma = chi_dimension_three_return(
                        entry_px, exit_px, voice.qty, fee_rate, voice.dir,
                    );
                    let z = MuClass::from_certificate(
                        voice.level,
                        voice.dir,
                        voice.bits,
                        parent_dir_of(&voices, v),
                        if voice.parent.is_some() {
                            PositionState::Child
                        } else {
                            PositionState::Root
                        },
                    );
                    mu_est.observe(MuObservation { class: z, x_gamma });
                } else {
                    diag_nonpositive_px += 1;
                }
            }
        }
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
        diag_nonpositive_px,
    }
}

/// 命令行参数解析结果（#412：窗口是否切片提升为显式字段，杜绝「传了 THETA 就静默丢窗口」）。
#[derive(Debug, PartialEq)]
struct CliArgs {
    symbol: String,
    window: Option<(String, String)>,
    theta: f64,
    z_alpha: f64,
}

const USAGE: &str = "<SYMBOL> [START_DATE END_DATE [THETA [Z_ALPHA]]]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO\n  THETA: χ 阈值 θ（默认 0；负值趋近全覆盖，验收边界）\n  Z_ALPHA: LCB 置信分位（默认 0）";

/// 解析单个 `Θ_risk` 浮点参数（THETA / Z_ALPHA 共用）——静默必须变响亮：
/// 解析失败直接报错返回，不做 `unwrap_or(0.0)` 式的静默默认值退化。
fn parse_theta_like(field_name: &str, raw: &str) -> Result<f64, String> {
    raw.parse::<f64>()
        .map_err(|e| format!("{field_name} 解析失败（{raw:?}）: {e}"))
}

/// 显式按位置解析（#412 修复核心）：`window` 只取决于 `START_DATE END_DATE` 是否被传入，
/// 与是否额外传了 THETA/Z_ALPHA **无关**——不再用 `args.len()==4` 这种「参数数量决定语义」
/// 的魔数判据（旧判据在 THETA 存在时 `args.len()==5`，导致窗口切片无声跳过，是 #412 根因）。
/// 不认识的参数个数一律报错返回 `Err`（响亮失败），不做静默降级。
fn parse_cli_args(args: &[String]) -> Result<CliArgs, String> {
    if args.len() < 2 {
        return Err(format!("用法: pi_bsp_timing {USAGE}"));
    }
    let symbol = args[1].clone();
    let window = |args: &[String]| Some((args[2].clone(), args[3].clone()));
    match args.len() {
        2 => Ok(CliArgs { symbol, window: None, theta: 0.0, z_alpha: 0.0 }),
        3 => Err(format!(
            "参数数量不匹配：给了 START_DATE 但缺 END_DATE。用法: pi_bsp_timing {USAGE}"
        )),
        4 => Ok(CliArgs { symbol, window: window(args), theta: 0.0, z_alpha: 0.0 }),
        5 => {
            let theta = parse_theta_like("THETA", &args[4])?;
            Ok(CliArgs { symbol, window: window(args), theta, z_alpha: 0.0 })
        }
        6 => {
            let theta = parse_theta_like("THETA", &args[4])?;
            let z_alpha = parse_theta_like("Z_ALPHA", &args[5])?;
            Ok(CliArgs { symbol, window: window(args), theta, z_alpha })
        }
        n => Err(format!(
            "参数数量不匹配（收到 {} 个，SYMBOL 之外只接受 0/2/3/4 个）。用法: pi_bsp_timing {USAGE}",
            n - 1
        )),
    }
}

fn main() -> std::process::ExitCode {
    let raw_args: Vec<String> = std::env::args().collect();
    let cli = match parse_cli_args(&raw_args) {
        Ok(cli) => cli,
        Err(msg) => {
            eprintln!("{msg}");
            return std::process::ExitCode::from(2);
        }
    };
    let config = ThetaConfig::default();
    let full = match load_by_symbol(&cli.symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // ★#412 修复：窗口切片只看 `cli.window`（由 START/END 是否传入决定），
    // 不再受后面是否还带了 THETA/Z_ALPHA 影响——这正是本票要堵死的静默退化点。
    let dataset = match &cli.window {
        Some((start, end)) => full.slice_date_window(start, end),
        None => full,
    };
    if dataset.bars.is_empty() {
        eprintln!("空数据集——无法回测（inconclusive）");
        return std::process::ExitCode::FAILURE;
    }

    let bars = &dataset.bars;
    let n = bars.len();
    let prices: Vec<f64> = bars
        .iter()
        .map(|b| b.close as f64 * config.tick.tick_size)
        .collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let years = (n as f64 / (365.25 * 24.0 * 60.0)).max(1e-9);

    // ── χ_t 阈值过滤的两遍对比（alpha2 §13，task #41 acc-chi-theta-filter 核心）──
    // Pass 1：χ≡1 全覆盖（每条 Lift 通过证书都开仓），同时估出 μ 表。
    // Pass 2：χ=1[μ(z)>θ]，用 Pass 1 的 μ 表过滤开仓。
    // ★因果性诚实声明（alpha2 §5 / project_zero_lookahead_backtest / formalization-validity-domain）：
    //   Pass 2 用 Pass 1 **全程** in-sample μ 过滤当前开仓 = 未来函数泄漏。故 ΔN 非全等只证明
    //   **选择器逻辑生效（L1）**，**不**证明 alpha 提升（需 walk-forward μ，是 task #42 delta-r-alpha 后续工位）。
    // θ：成本+风险门槛（§13），**Θ_risk 参数，非缠论可导**（诚实标注）。默认 θ=0；由 CLI 覆盖。
    // #412 修复后：THETA 解析失败在 `parse_cli_args` 里已响亮报错退出，这里直接读已验证值。
    let theta: f64 = cli.theta;
    // z_alpha：LCB 置信分位（p25 §12，准入用 LCB(μ)=mean−z_alpha·std/√n 防高维 z 过拟合）。
    // 默认 0.0（LCB=mean ⟹ n≥2 类退化裸 μ；n=1 类 mu_lcb=None 走 empty=pass=true 全覆盖放行，
    // 与裸 μ 正样本放行同决策——负 μ 的 n=1 例外，LCB 路径放行而裸 μ 拒，下游 L2 归因）；CLI 覆盖。
    let z_alpha: f64 = cli.z_alpha;

    let pass1 = run_state_machine(bars, &prices, &config, fee_rate, None);
    let pass2 = run_state_machine(
        bars,
        &prices,
        &config,
        fee_rate,
        Some((&pass1.mu_est, theta, z_alpha)),
    );

    // ★可证伪对比（acc-chi-theta-filter 核心）：ΔN_t 序列 χ≡1 vs χ=1[μ>θ] 非全等。
    let n_diff_bars = (0..n)
        .filter(|&i| (pass1.net_per_bar[i] - pass2.net_per_bar[i]).abs() > 1e-12)
        .count();
    let sequences_differ = n_diff_bars > 0;

    // 下游 §18 指标对 **Pass 2（过滤后）** 算（χ=1[μ>θ] 策略的真实头寸序列）。
    let net_per_bar = pass2.net_per_bar.clone();
    let voices = &pass2.voices;
    let mu_est = &pass2.mu_est;
    let diag_certs = pass2.diag_certs;
    let diag_host_miss = pass2.diag_host_miss;
    let diag_has_parent = pass2.diag_has_parent;
    let diag_parent_active_found = pass2.diag_parent_active_found;
    let diag_lift_pass = pass2.diag_lift_pass;

    // ── §18 r^bsp = N^bsp·ΔP − C：逐 bar net-position MtM 权益曲线（同 π^cov NAV 口径）──
    // N_t^bsp 是连续净额（非 ±1 toggle）；equity 归一化初始=1。成本在净额变动 bar 按 |ΔN| 名义额扣。
    let nav_base: f64 = {
        // 名义基准 = 净额峰值 × 均价量级（与 significance nav_base 同口径：名义额，方向无关）。
        let peak = net_per_bar
            .iter()
            .fold(0.0f64, |a, &x| a.max(x.abs()))
            .max(1.0);
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
            if prev.abs() > 1e-12 {
                eq / prev - 1.0
            } else {
                0.0
            }
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
    for v in voices {
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

    let bh_return = if n >= 2 && prices[0].abs() > 1e-12 {
        prices[n - 1] / prices[0] - 1.0
    } else {
        0.0
    };
    let m = metrics::compute(&equity_curve, &daily_returns, &trade_pnls, years, bh_return);
    let sig = metrics::significance(
        &trade_pnls,
        &daily_returns,
        &trades,
        &prices,
        fee_rate,
        m.strat_return,
    );

    let n_voices = voices.len();
    let n_long = voices.iter().filter(|v| v.dir > 0).count();
    let n_children = voices.iter().filter(|v| v.parent.is_some()).count();
    let max_net = net_per_bar.iter().fold(0.0f64, |a, &x| a.max(x.abs()));

    println!("=== π_Θ^bsp 买卖点多声部离散择时回测（忠实 PDF §1-19）===");
    println!("品种            : {}", dataset.symbol);
    println!("bar 数          : {n}");
    println!("年化基数(years) : {years:.4}");
    println!("--- 声部状态机（§3 position instance / §12 N^bsp=Σσ_v q_v）---");
    println!(
        "声部总数        : {n_voices}（多 {n_long} / 空 {}）",
        n_voices - n_long
    );
    println!("短差子声部数    : {n_children}（§6/§16 σ_u=−σ_p）");
    println!("净额峰值 |N^bsp|: {max_net:.2}");
    println!("--- 子声部可达性诊断（647 第四根因坐实，L2 否定性结果）---");
    let host_hit = diag_certs.saturating_sub(diag_host_miss);
    let hit_pct = if diag_certs > 0 {
        100.0 * host_hit as f64 / diag_certs as f64
    } else {
        0.0
    };
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
    println!(
        "shift  mean/p   : {:.6} / {:.4}",
        sig.shift_mean_return, sig.shift_pvalue
    );
    println!(
        "indep  mean/p   : {:.6} / {:.4}",
        sig.indep_mean_return, sig.indep_pvalue
    );
    println!("beats_random    : {}", sig.theta_beats_random);
    println!("controls_degen  : {}", sig.controls_degenerate);
    // ── χ_t=1[μ>θ] 阈值过滤对比（alpha2 §13，task #41 acc-chi-theta-filter 核心证据）──
    // 可证伪断言：χ≡1（Pass 1 全覆盖）vs χ=1[μ>θ]（Pass 2 过滤）的 ΔN_t 序列**非全等**。
    // sequences_differ=true ⟹ 过滤真生效（排除假实装）；θ 足够低（全覆盖退化）时 false 是合法边界。
    // ★认识论：sequences_differ 是 **L1**（选择器逻辑生效，非 alpha 提升）——Pass 2 用 Pass 1 全程
    //   in-sample μ 过滤当前 = 未来函数泄漏，见 selector.rs 模块头。
    let pass1_voices = pass1.voices.len();
    let pass2_voices = pass2.voices.len();
    println!("--- χ_t=1[μ>θ] 阈值过滤对比（alpha2 §13，task #41；L1 过滤生效，非 L2 alpha）---");
    println!("θ（Θ_risk 参数）: {theta:.6}（成本+风险门槛，**非缠论可导**，诚实标注）");
    println!(
        "χ 否决开仓证书数 : {}（μ(z)≤θ 被滤；Pass 1 χ≡1 恒 0）",
        pass2.chi_rejected
    );
    // #563 M7：entry_px≤0 跳过喂 μ 的笔数（真实标的池预期恒 0，见 chi_dimension_three_return 前置条件）。
    println!(
        "entry_px≤0 跳过μ : {}（Pass1={} Pass2={}，>0 ⟹ 数据面异常需另查）",
        pass1.diag_nonpositive_px + pass2.diag_nonpositive_px,
        pass1.diag_nonpositive_px,
        pass2.diag_nonpositive_px,
    );
    println!("声部数 χ≡1/χ滤  : {pass1_voices} / {pass2_voices}（过滤后 ≤ 全覆盖，§13 Γ^trade⊆Γ）");
    println!("ΔN 序列差异 bar  : {n_diff_bars} / {n}（非全等={sequences_differ}）");
    if sequences_differ {
        println!("  ⟹ 过滤真生效（χ≡1 与 χ=1[μ>θ] 产不同 N_t；排除假实装）。L1 选择器逻辑成立。");
    } else if pass2.chi_rejected == 0 {
        println!("  ⟹ χ 无否决（θ 低于所有已观测 μ ∨ 无负 μ 类）⟹ 全覆盖退化（§13 边界，合法）。");
    } else {
        println!(
            "  ⚠ χ 有否决但 ΔN 全等——被滤声部不影响净额（被 anc_ok 剪/重复 carrier）。诚实标注。"
        );
    }
    // ── μ(z,a) 类别条件边际收益表（alpha2 §12，task #39，L2 真实数据可否证）──
    // 按 z=(ℓ,δ,I_γ,父向,短差,仓位态) 分桶；#65 量纲③：
    // μ(z)=E[X_γ|z]，X_γ=费扣后持仓期相对收益（逐笔 ÷qty·entry_px）。
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
    if mu_rows.is_empty() {
        println!("（无平仓观测——无声部生命周期闭合，μ 表空，inconclusive）");
    } else {
        println!(
            "{:>3} {:>3} {:>4} {:>5} {:>6} {:>6} {:>6} {:>14}",
            "ℓ", "δ", "Iγ", "父向", "短差", "仓位", "|S_z|", "μ(z)相对"
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
        println!(
            "等级: L2——π^bsp Sharpe={:.4} 子声部={n_children} 多声部对冲真激活（§19 goal#5）",
            m.sharpe
        );
    }
    std::process::ExitCode::SUCCESS
}

// #412 回归测试拆到独立文件（避免把本文件推过 coding-style.md 800 行硬顶；
// 测试逻辑仍完全属于本 bin，未违反「只动 pi_bsp_timing.rs（及其测试）」的范围）。
#[cfg(test)]
#[path = "pi_bsp_timing/cli_args_tests.rs"]
mod cli_args_tests;

// #614 并线：kimi 线的量纲③回归测试（#65 裁定）与 main 线的 #412 外置 CLI 测试互不相干，
// 并集保留——外置模块 `cli_args_tests` 覆盖 CLI 解析，本内联模块覆盖 χ 量纲③的 qty 免疫。
#[cfg(test)]
mod tests {
    use super::*;

    /// 量纲③重锚（#65 裁定 2026-07-21）：逐笔 ÷(qty·entry_px)，χ 观测对 qty 免疫。
    #[test]
    fn chi_feed_dimension_three_is_qty_immune() {
        let one = chi_dimension_three_return(100.0, 110.0, 1.0, 0.001, 1);
        let five = chi_dimension_three_return(100.0, 110.0, 5.0, 0.001, 1);
        assert!(
            (one - five).abs() < 1e-15,
            "量纲③逐笔归一化后必须 qty 免疫：{one} vs {five}"
        );
    }
}
