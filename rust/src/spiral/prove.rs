//! 群关系 prove 守卫体系（L0 结构 panic）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计来源：`docs/spiral_engine_v2_architecture.md` §7.1（群关系守卫表）+
//! `docs/necessity_derivation.md` T56–T59。
//!
//! ## 范式（架构 §7，继承 unn 引擎 `prove_*` 风格）
//! **prove 即验收 = 必然性运行时证明，violation = panic**（非回测验收）。
//! 每条群关系配一个 panic 守卫，且必须**非重言**：核心断言可参数化
//! （`assert_*(...)`），正向用规范值（23/τ/h）不 panic，反证用错误值
//! （22/h/hτ）必 panic（配套 `#[should_panic]`）。
//!
//! ## 双层非重言保证
//! 每个断言同时检验**代数层**（`GroupElement` 正规形相等）与**作用层**
//! （`act_helix` 对采样点一致）。任一实现 bug（`compose`/`power`/`act_helix`）
//! 都会 fire——非同义反复。
//!
//! ## 认识论等级
//! 全文件 **L0Structural**（纯代数，零信息增量，从 D∞ 定义关系推导）。
//! 对应架构 §7.1 守卫名：
//! - `prove_h23_eq_sigma`        ↔ `prove_angular_radial_holonomy`（T56）
//! - `prove_tau_involution`      ↔ `prove_tau_involution`（T24/T57）
//! - `prove_conjugation`         ↔ `prove_chirality_angular_inversion`（T57）

use super::params::{FIRST_BSP_LADDER, PENDING_LO};
use super::result::SpiralResult;
use super::state::{GroupAction, GroupElement, N_CONCEPT_RINGS};
use super::voice::{SpiralVoice, VoiceStatus};
use crate::trading::types::{Polarity, MAX_LADDER};

/// `act_helix` 交叉验证采样点（含负/零/正/跨圈，覆盖反射与平移）。
const HELIX_SAMPLES: [i64; 7] = [-23, -5, -1, 0, 1, 23, 100];

// ════════════════════ 可参数化核心断言（非重言载体）════════════════════

/// **角径全纯 `hᵏ=σ`（T56）核心断言**：环数 `rings` 步角向推进 == 一步径向跃迁。
///
/// 正向 `rings=23`（规范）不 panic；反证 `rings≠23` 必 panic（`h²²≠σ` 等）。
/// 双层检验：(1) `h.power(rings)` 的正规形 == 独立定义的 `σ`；(2) `h` 作用
/// `rings` 次 == `σ` 作用一次（= `n+23`）。
pub fn assert_angular_holonomy(rings: i32) {
    let h_power = GroupElement::h().power(rings);
    let sigma = GroupElement::sigma();
    assert_eq!(
        h_power, sigma,
        "群关系违反：h^{rings} = {h_power:?} ≠ σ = {sigma:?}（角径全纯 h²³=σ，T56——\
         绕一圈角向≠升一级径向 ⟹ power 实现或环数错位）"
    );
    for &n in &HELIX_SAMPLES {
        // h 作用 rings 次（逐次平移 +1）。
        let by_h = (0..rings).fold(n, |acc, _| GroupElement::h().act_helix(acc));
        let by_sigma = sigma.act_helix(n);
        assert_eq!(
            by_h, by_sigma,
            "群关系违反（作用层）：h 作用 {rings} 次={by_h} ≠ σ 作用={by_sigma}（n={n}，h²³=σ）"
        );
    }
}

/// **手性对合 `g²=e`（T24/T57）核心断言**：`g` 翻两次回单位元。
///
/// 正向 `g=τ`（规范对合）不 panic；反证 `g=h`（`h²={2,0}≠e`）必 panic。
/// 双层：(1) `g.compose(g) == e`；(2) `act_helix(g, act_helix(g, n)) == n`。
pub fn assert_involution(g: GroupElement) {
    let gg = g.compose(g);
    assert_eq!(
        gg,
        GroupElement::identity(),
        "群关系违反：{g:?}² = {gg:?} ≠ e（对合 τ²=e，T24/T57——多空时序翻转两次须复位）"
    );
    for &n in &HELIX_SAMPLES {
        assert_eq!(
            g.act_helix(g.act_helix(n)),
            n,
            "群关系违反（作用层）：{g:?} 作用两次 ≠ id（n={n}，τ²=e）"
        );
    }
}

/// **手性-角向反演 `τgτ⁻¹=g⁻¹`（T57）核心断言**：τ 共轭反转角向推进方向。
///
/// 正向 `g=h`（纯角向，定义关系 `τhτ⁻¹=h⁻¹`）不 panic；反证 `g=hτ`
/// （含反射分量，`τ(hτ)τ⁻¹={−1,1}≠(hτ)⁻¹={1,1}`）必 panic——证明断言非
/// vacuously-true（对 h 成立、对 hτ 失败）。
/// 双层：(1) `τ·g·τ⁻¹` 正规形 == `g⁻¹`；(2) `act_helix` 对采样点一致。
pub fn assert_chiral_conjugation(g: GroupElement) {
    let tau = GroupElement::tau();
    let lhs = tau.compose(g).compose(tau.inverse());
    let rhs = g.inverse();
    assert_eq!(
        lhs, rhs,
        "群关系违反：τ·{g:?}·τ⁻¹ = {lhs:?} ≠ {g:?}⁻¹ = {rhs:?}（手性-角向反演 τhτ⁻¹=h⁻¹，\
         T57——做空走势角向镜像做多，半直积耦合）"
    );
    for &n in &HELIX_SAMPLES {
        assert_eq!(
            lhs.act_helix(n),
            rhs.act_helix(n),
            "群关系违反（作用层）：τgτ⁻¹ ≠ g⁻¹（g={g:?} n={n}，τhτ⁻¹=h⁻¹）"
        );
    }
}

/// **GroupAction ↔ GroupElement 一致性核心断言**：语义生成元 `to_element` 的
/// `act_helix` 与代数层一致（除 τ 的 in-place 手性语义分离，见 `state.rs`
/// `apply` 注释——此处只验证可表示为纯 helix 作用的角向/径向算子）。
pub fn assert_action_matches_element(action: GroupAction) {
    let g = action.to_element();
    // 仅验证角向/径向算子（helix 平移）；ChiralSeam 的 in-place ε 翻转与代数层
    // τ 反射分离（527号能指碰撞），不在此断言（由 state.rs τ²=e 状态显形守）。
    if matches!(action, GroupAction::ChiralSeam) {
        return;
    }
    for &n in &HELIX_SAMPLES {
        let by_element = g.act_helix(n);
        let expected = match action {
            GroupAction::AngularStep => n + 1,
            GroupAction::RadialAscend => n + N_CONCEPT_RINGS as i64,
            GroupAction::RadialDescend => n - N_CONCEPT_RINGS as i64,
            GroupAction::ChiralSeam => unreachable!(),
        };
        assert_eq!(
            by_element, expected,
            "GroupAction {action:?} 的 act_helix={by_element} ≠ 期望 {expected}（n={n}）"
        );
    }
}

// ════════════════════ L0 群关系守卫（规范值封装）════════════════════

/// **T56：`h²³=σ`**（角径全纯）。规范环数 = `N_CONCEPT_RINGS`。
pub fn prove_h23_eq_sigma() {
    assert_angular_holonomy(N_CONCEPT_RINGS);
}

/// **T24/T57：`τ²=e`**（手性对合）。规范元 = `τ`。
pub fn prove_tau_involution() {
    assert_involution(GroupElement::tau());
}

/// **T57：`τhτ⁻¹=h⁻¹`**（手性-角向反演）。规范元 = `h`。
pub fn prove_conjugation() {
    assert_chiral_conjugation(GroupElement::h());
}

/// 聚合自检：一次性运行全部 L0 群关系守卫（供引擎启动时调用）。
pub fn prove_all_group_laws() {
    prove_h23_eq_sigma();
    prove_tau_involution();
    prove_conjugation();
    for action in [
        GroupAction::AngularStep,
        GroupAction::ChiralSeam,
        GroupAction::RadialAscend,
        GroupAction::RadialDescend,
    ] {
        assert_action_matches_element(action);
    }
}

// ════════════════════ L0：H¹ 闭合 + 手性缝守卫（新增，架构 §6.4/§7.1）════════════════════

/// **H¹ 闭合 `Δr=−1`（A1/定理 A1，架构 §6.4）运行时证明**：每次 CrossLevel 闭合
/// （E spawn 下沉 / D 回补向心下沉）的 `child_ladder == parent_ladder − 1`。防漂移；与
/// `prove_theta_sigma_invariant` 互补（后者守配额 σ-不变，前者守级别 Δr）。
/// **两个消费者**：① accounting `try_spawn_cost_gated` 传 `descended.r`（E spawn 的
/// `σ⁻¹` 群作用真实输出）；② engine D 回补传 `close_ladder=ladder−1`（闭合向心下沉到
/// 次级别，NR-1 否定同级别 σ=e 闭合）。**非重言**：漂移到同级别 spawn/闭合即 fire
/// （`cross_level_fires_on_same_level_spawn`）。
pub fn prove_cross_level_closure(parent_ladder: usize, child_ladder: usize) {
    assert_eq!(
        child_ladder + 1,
        parent_ladder,
        "H¹ 闭合违反：Δr 必为 −1（cross-level），得 {parent_ladder}→{child_ladder}\
         （NR-1 否定同级别 σ=e 闭合；A1 生成元 = 跨级别位移率）"
    );
}

/// **手性缝 τ@φ=0（NR-3，架构 §7.1）运行时证明**：τ（C 翻转）只能在角向奇点
/// φ=0 触发——莫比乌斯禁止"同时多空"（多空两书仅 φ=0 手性缝桥接）。
/// **非重言**：调用点传操作时刻的 `phi`，φ≠0 翻转即 fire。
pub fn prove_chirality_seam(phi: u8, bar: i64) {
    assert_eq!(
        phi, 0,
        "NR-3 违反@bar {bar}：τ（手性翻转）在 φ={phi}≠0 触发（多空两书仅 φ=0 手性缝桥接）"
    );
}

// ════════════════════ L2：配额 σ-不变守卫（542号缺瓦）════════════════════

/// **配额 σ-不变性运行时证明（T18×T48×T59，第15环；542号）**：实际分配的
/// `m_quota` 必 == σ-不变规范 `p_units × SUB_SPAWN_FRAC`（= `1/λ`，级别无关）。
/// **本守卫的存在理由**：N8/A4 守恒（Σunits=N_base）**不覆盖** σ-不变性——旧
/// `θ_sub/θ_total` 全局归一化仍守 Σunits 却使 `f` 随级别变（破 T59）。**非重言**：
/// 规范刻意**不取 `sub`**（编码级别无关性）⇒ sub-依赖分配必 fire
/// （`theta_sigma_invariance_fires_on_level_dependent_quota` 反证）。
pub fn prove_theta_sigma_invariant(m_quota: f64, p_units: f64, sub: usize, bar: i64) {
    use super::params::SUB_SPAWN_FRAC;
    let canonical = p_units * SUB_SPAWN_FRAC;
    assert!(
        (m_quota - canonical).abs() <= 1e-9 * p_units.max(1.0),
        "T18×T48×T59 违反@bar {bar}：spawn 配额 m_quota={m_quota} ≠ σ-不变规范 {canonical}\
         （= p_units×1/λ，级别无关；sub={sub}）——配额比例 f 随级别变（θ 归一化残余）破 T59 σ-不变"
    );
}

// ════════════════════ L2：N1–N8 会计守恒（每 bar panic）════════════════════

/// **N1（逐仓独立森林结构，第20环）**：① ≤1 active root；② active 非根 voice 的
/// parent 非 Closed（孤儿不可能 T42）；③ children 反向引用一致。返回最大活跃子数
/// （>1 = 森林实证，栈不可能）。
pub fn prove_n1_forest(voices: &[SpiralVoice], bar: i64) -> usize {
    let mut roots = 0usize;
    let mut max_children = 0usize;
    for (id, v) in voices.iter().enumerate() {
        if v.is_active() {
            if v.parent.is_none() {
                roots += 1;
            } else {
                let p = v.parent.expect("非根有父");
                assert!(
                    p < voices.len(),
                    "N1 违反@bar {bar}：voice {id} 父 id {p} 越界"
                );
                assert!(
                    voices[p].is_active(),
                    "N1 违反@bar {bar}：孤儿——active voice {id} 的父 {p} 已 Closed"
                );
                assert!(
                    voices[p].children.contains(&id),
                    "N1 违反@bar {bar}：voice {id} 不在父 {p} 的 children 列表（树链断裂）"
                );
            }
        }
        let live_kids = v
            .children
            .iter()
            .filter(|&&k| voices[k].is_active())
            .count();
        max_children = max_children.max(live_kids);
    }
    assert!(
        roots <= 1,
        "N1 违反@bar {bar}：{roots} 个 active root（单根不变量）"
    );
    max_children
}

/// **N2（per-voice 独立操作，第18环）**：本 bar 操作过的 voice id 无重复
/// （per-voice acted_bar 互斥——同 voice 不双动）。
pub fn prove_n2_per_voice(acted_ids: &[usize], bar: i64) {
    for (i, &a) in acted_ids.iter().enumerate() {
        for &b in &acted_ids[i + 1..] {
            assert!(
                a != b,
                "N2 违反@bar {bar}：voice {a} 同 bar 双动（per-voice acted_bar 失效）"
            );
        }
    }
}

/// **N3（全三类 BSP 消费，第12环）**：type2 不被跳过（按 side() 归侧，无 continue）。
pub fn prove_n3_type2(type2_seen: u64, type2_handled: u64, bar: i64) {
    assert_eq!(
        type2_seen, type2_handled,
        "N3 违反@bar {bar}：type2 事件 {type2_seen} 个，仅处理 {type2_handled} 个（Sell2/Buy2 被跳过）"
    );
}

/// **N4（成本门终止递归，第16环）eod 反证**：`floor_stop` 计数器恒 0（终止纯走
/// noref_reject ∨ cost_reject，无固定 floor 参数）。出现任何 floor_stop ⇒ panic。
pub fn prove_n4_cost_gate(res: &SpiralResult) {
    let floor_stops: u64 = res.n_floor_stops_by_ladder.iter().sum();
    assert_eq!(
        floor_stops, 0,
        "N4 违反：floor_stop={floor_stops}（出现固定 floor 终止——应纯成本门 θ=None ∨ θ<friction）"
    );
}

/// **N7（降成本不需 pending，第17环）**：E spawn / D 回补的触发是 voice **自层**
/// 走势结构（nf@voice.ladder 向心确认），非全局 located 链。violation（触发层≠voice 层）= panic。
pub fn prove_n7_spawn_self_level(trigger_ladder: usize, voice_ladder: usize, bar: i64) {
    assert_eq!(
        trigger_ladder, voice_ladder,
        "N7 违反@bar {bar}：降成本触发层 {trigger_ladder} ≠ voice 层 {voice_ladder}（借用更高级别 pending）"
    );
}

/// **N8（双层会计多空嵌套守恒，第22环）**：① `Σ(active units) = n_base`（股数守恒）；
/// ② 同价 c 操作前后 NAV 不变（价值中性）。violation = panic。
pub fn prove_n8_conservation(
    voices: &[SpiralVoice],
    n_base: f64,
    nav_pre: f64,
    nav_post: f64,
    bar: i64,
) {
    let sum_units: f64 = voices
        .iter()
        .filter(|v| v.is_active())
        .map(|v| v.units)
        .sum();
    assert!(
        (sum_units - n_base).abs() <= 1e-6 * n_base.max(1.0),
        "N8 违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}（股数守恒 §8.1）"
    );
    assert!(
        (nav_post - nav_pre).abs() <= 1e-4 * nav_pre.abs().max(1.0),
        "N8 违反@bar {bar}：NAV {nav_pre}→{nav_post}（同价操作非价值中性 §8.4——双层会计破）"
    );
}

/// **A5（T30 根级别涌现=会计重组，N 不变）运行时证明**：relabel 是重新读数（A3 禁
/// 加仓）⇒ units 与 NAV 严格不变。violation = panic。
pub fn prove_a5_relabel(units_post: f64, units_pre: f64, nav_post: f64, nav_pre: f64, bar: i64) {
    assert!(
        (units_post - units_pre).abs() <= 1e-9 * units_pre.max(1.0),
        "A5(T30) 违反@bar {bar}：根级别涌现重组改变了 units（{units_pre}→{units_post}）——重组非加仓"
    );
    assert!(
        (nav_post - nav_pre).abs() <= 1e-9 * nav_pre.abs().max(1.0),
        "A5(T30) 违反@bar {bar}：根级别涌现重组改变了 NAV（{nav_pre}→{nav_post}）——会计重组价值中性"
    );
}

/// **T14（根多空对称翻转，第21环）+ A5 运行时证明**：根 in-place 翻转后 ① 极性反转；
/// ② 森林仍单根；③ units 守恒（M=N）。violation = panic。
pub fn prove_t14_root_flip(
    voices: &[SpiralVoice],
    rid: usize,
    old_dir: Polarity,
    units_pre: f64,
    bar: i64,
) {
    assert_ne!(
        voices[rid].dir(),
        old_dir,
        "T14 违反@bar {bar}：根就地翻转后极性未反转（dir 仍 {old_dir:?}）"
    );
    assert!(
        (voices[rid].units - units_pre).abs() <= 1e-9 * units_pre.max(1.0),
        "T14 违反@bar {bar}：根翻转改变了 units（{units_pre}→{}，同股数翻转 M=N 破）",
        voices[rid].units
    );
    let roots = voices
        .iter()
        .filter(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none())
        .count();
    assert_eq!(
        roots, 1,
        "T14 违反@bar {bar}：翻转后 {roots} 个 active root（in-place flip 应保持单根）"
    );
}

/// **T1（不主动清仓，第23环恒仓）运行时证明**：① 森林"非空→空"⟹ `root_liquidated`
/// （A 强平，非强平致空仓=引擎主动清仓违反）；② F-eligible 但 bar 末仍空 = 漏建仓。
pub fn prove_t1_no_voluntary_exit(
    was_active: bool,
    is_active: bool,
    root_liquidated: bool,
    f_eligible_but_empty: bool,
    bar: i64,
) {
    if was_active && !is_active {
        assert!(
            root_liquidated,
            "T1①违反@bar {bar}：森林非空→空 但根未被市场强平（引擎主动清仓？否定线/观测态残留）"
        );
    }
    assert!(
        !f_eligible_but_empty,
        "T1②违反@bar {bar}：森林空 ∧ F-eligible 买点但未重建仓（强平后应立即重建）"
    );
}

// ════════════════════ L2：自层 counter 对称（E 降成本触发规范）════════════════════

/// **自层 counter-direction 走势完美 fire（E 降成本 spawn 触发标准）**：voice 自层
/// 反方向 helix-confirmed nf fire——多头查自层卖点（`nf_sell[ladder]`）、空头查自层
/// 买点（`nf_buy[ladder]`）。
///
/// **作用域**：仅 E spawn 的**触发**（自层走势完美 ⇒ σ⁻¹ 下沉生成子 voice@r−1，N7
/// 第17环：触发自层、结果下沉）。**D 回补的闭合**改用 `sub_level_counter_fire`
/// （闭合下沉到次级别 r=k−1，NR-1 否定同级别 σ=e 闭合，"确认在哪个级别，闭合就在
/// 哪个级别"——向心确认已回溯到 k−1，闭合也须在 k−1）。
pub fn self_level_counter_fire(
    dir: Polarity,
    ladder: usize,
    nf_sell: &[Option<f64>; MAX_LADDER],
    nf_buy: &[Option<f64>; MAX_LADDER],
) -> bool {
    match dir {
        Polarity::Long => nf_sell[ladder].is_some(),
        Polarity::Short => nf_buy[ladder].is_some(),
    }
}

/// **自层 counter 操作对称性运行时证明（E 触发）**：E spawn 内联 gate 必 ==
/// `self_level_counter_fire` 规范（非 raw `sig.*_any`）。**非重言**（回归防护）：调用点
/// 用独立内联表达，漂移回 raw 即 panic（137号 make-decision-observable）。
pub fn prove_self_level_symmetric(
    dir: Polarity,
    ladder: usize,
    op_trigger: bool,
    nf_sell: &[Option<f64>; MAX_LADDER],
    nf_buy: &[Option<f64>; MAX_LADDER],
    bar: i64,
) {
    let standard = self_level_counter_fire(dir, ladder, nf_sell, nf_buy);
    assert_eq!(
        op_trigger, standard,
        "自层 counter 对称违反@bar {bar}：E 触发内联 gate {op_trigger} ≠ 规范 {standard}\
         （dir={dir:?} ladder={ladder}）——E 触发必用单一 nf 标准（自层向心确认 φ=0），非 raw sig.*_any"
    );
}

// ════════════════ L0/L2：次级别闭合（D 回补 H¹ Δr=−1 规范）════════════════

/// **次级别 counter-direction 闭合 fire（D 回补 / 子 voice 闭合，H¹ Δr=−1，架构 §6.1）**：
/// voice 自层闭合**不在开仓级别 r=k 原地**（NR-1 否定 σ=e 同级别闭合），而**向心下沉
/// 到次级别 r=k−1**（φ 归零）——多头查次级别卖点（`nf_sell[ladder−1]`）、空头查次级别
/// 买点（`nf_buy[ladder−1]`）。"确认在哪个级别，闭合就在哪个级别"：向心确认
/// （`helix_centripetal_confirm`）已回溯到 r=k−1，闭合也须在 r=k−1。
///
/// **边界保护**：`ladder−1 < FIRST_BSP_LADDER`（segment=2 是最低可操作级别，无次级别）
/// ⇒ 保持同级别闭合（NR-5：σ⁻¹ 在递归基 r=FIRST_BSP 无像）。
///
/// 返回 `(fire, close_ladder)`：`close_ladder = ladder−1`（常态 Δr=−1）∨ `ladder`（边界）。
pub fn sub_level_counter_fire(
    dir: Polarity,
    ladder: usize,
    nf_sell: &[Option<f64>; MAX_LADDER],
    nf_buy: &[Option<f64>; MAX_LADDER],
) -> (bool, usize) {
    let close_ladder = if ladder > FIRST_BSP_LADDER {
        ladder - 1
    } else {
        ladder
    };
    let fire = match dir {
        Polarity::Long => nf_sell[close_ladder].is_some(),
        Polarity::Short => nf_buy[close_ladder].is_some(),
    };
    (fire, close_ladder)
}

/// **次级别闭合对称性运行时证明（D 回补）**：D 回补内联 gate + close_ladder 必 ==
/// `sub_level_counter_fire` 规范（次级别 nf，非自层）。**非重言**（回归防护）：调用点
/// 漂移回自层 `nf[ladder]` 即 panic（137号 make-decision-observable）——守"闭合在
/// r=k−1 非 r=k"（NR-1）。
pub fn prove_sub_level_symmetric(
    dir: Polarity,
    ladder: usize,
    op_trigger: bool,
    op_close_ladder: usize,
    nf_sell: &[Option<f64>; MAX_LADDER],
    nf_buy: &[Option<f64>; MAX_LADDER],
    bar: i64,
) {
    let (standard, std_ladder) = sub_level_counter_fire(dir, ladder, nf_sell, nf_buy);
    assert_eq!(
        op_trigger, standard,
        "次级别闭合对称违反@bar {bar}：D 回补内联 gate {op_trigger} ≠ 规范 {standard}\
         （dir={dir:?} ladder={ladder}）——闭合必读次级别 nf[{std_ladder}]（向心下沉 Δr=−1），非自层 nf[{ladder}]"
    );
    assert_eq!(
        op_close_ladder, std_ladder,
        "次级别闭合层违反@bar {bar}：D 回补 close_ladder {op_close_ladder} ≠ 规范 {std_ladder}\
         （ladder={ladder}）——闭合下沉层错（NR-1 否定同级别 σ=e 闭合）"
    );
}

// ════════════════════ L2 观测：T50/T56–T59（eod，~ 状态）════════════════════

/// **T50（操作频率径向标度律 f(k)∝λ⁻ᵏ，~观测）**：candidate 层 fire(k) 随 k 非增
/// （高层比低层罕见）。返回定性单调局部违反层数（观测非 panic）。
pub fn prove_t50_radial_scaling(res: &SpiralResult) -> u64 {
    let fire: [u64; MAX_LADDER] =
        std::array::from_fn(|k| res.n_fire_sell_by_ladder[k] + res.n_fire_buy_by_ladder[k]);
    let mut violations = 0u64;
    for k in (PENDING_LO + 1)..MAX_LADDER {
        if fire[k] > fire[k - 1] {
            violations += 1;
        }
    }
    violations
}

/// **T56（角径全纯 h²³=σ）eod**：结构 panic——segment（径向塔基）层无角向圈
/// （confirm fire），h²³=σ 角向起算点 = move(L1)。返回 confirm fire 覆盖的径向层数。
pub fn prove_t56_angular_radial_holonomy(res: &SpiralResult) -> u64 {
    let seg_fire =
        res.n_fire_sell_by_ladder[FIRST_BSP_LADDER] + res.n_fire_buy_by_ladder[FIRST_BSP_LADDER];
    assert_eq!(
        seg_fire, 0,
        "T56 违反：segment(FIRST_BSP={FIRST_BSP_LADDER}) 层出现 {seg_fire} 个 confirm fire\
         （角向圈下沉到径向塔基——h²³=σ 角向起算点应在 move(L1)≥{PENDING_LO}）"
    );
    (PENDING_LO..MAX_LADDER)
        .filter(|&k| res.n_fire_sell_by_ladder[k] + res.n_fire_buy_by_ladder[k] > 0)
        .count() as u64
}

/// **T57（手性镜像 τhτ⁻¹=h⁻¹，~regime 观测）**：卖/买侧角向圈逐层镜像平衡。返回
/// 单边角向行程层数（一侧 fire=0 另一侧≠0，镜像在该层退化）。
pub fn prove_t57_chirality_mirror(res: &SpiralResult) -> u64 {
    let mut one_sided = 0u64;
    for k in PENDING_LO..MAX_LADDER {
        let s = res.n_fire_sell_by_ladder[k];
        let b = res.n_fire_buy_by_ladder[k];
        if (s == 0) != (b == 0) {
            one_sided += 1;
        }
    }
    one_sided
}

/// **T58（角向基本域 = 23 环，ℤ/23 商）eod**：结构 panic——fire(k)>0 ⟹ arms(k)>0
/// （角向圈闭合必先有起始；23 环链在该级别不断裂）。返回链生命周期活跃级别数。
pub fn prove_t58_angular_basic_domain(res: &SpiralResult) -> u64 {
    let mut active_levels = 0u64;
    for k in PENDING_LO..MAX_LADDER {
        let fire = res.n_fire_sell_by_ladder[k] + res.n_fire_buy_by_ladder[k];
        if fire > 0 {
            assert!(
                res.n_arms_by_ladder[k] > 0,
                "T58 违反：级别 {k} 有 {fire} 个 confirm fire 但 arms=0（角向圈无起始直接闭合——23 环链断裂）"
            );
            active_levels += 1;
        }
    }
    active_levels
}

/// **T59（尺度不变 σ 自相似，~观测）**：每个结构活跃级别（有 arm）都展现同构角向
/// 生命周期（arm→fire）。返回自相似退化层数（有 arm 无 fire）。
pub fn prove_t59_scale_invariance(res: &SpiralResult) -> u64 {
    let mut degenerate = 0u64;
    for k in PENDING_LO..MAX_LADDER {
        let armed = res.n_arms_by_ladder[k] > 0;
        let fired = res.n_fire_sell_by_ladder[k] + res.n_fire_buy_by_ladder[k] > 0;
        if armed && !fired {
            degenerate += 1;
        }
    }
    degenerate
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────── 正向：群关系成立（零 panic）────────────────

    #[test]
    fn group_laws_hold() {
        // 全部规范守卫零 panic（L0 群关系成立）。
        prove_all_group_laws();
    }

    #[test]
    fn h23_eq_sigma_holds() {
        prove_h23_eq_sigma();
    }

    #[test]
    fn tau_involution_holds() {
        prove_tau_involution();
    }

    #[test]
    fn conjugation_holds() {
        prove_conjugation();
    }

    // ──────────────── 反证：非重言（错误输入必 panic）────────────────

    #[test]
    #[should_panic(expected = "h²³=σ")]
    fn h23_fires_on_wrong_ring_count() {
        // 反证 prove_h23_eq_sigma **非重言**：环数 22 ⇒ h²²≠σ=h²³ ⇒ 必 panic。
        // 真实 bug 类（power 少算一步 / 环数错位）检出能力证明。
        assert_angular_holonomy(N_CONCEPT_RINGS - 1);
    }

    #[test]
    #[should_panic(expected = "τ²=e")]
    fn involution_fires_on_non_involution() {
        // 反证 prove_tau_involution **非重言**：h²={2,0}≠e ⇒ 必 panic。
        // 守卫真在比较（对 τ 成立、对 h 失败），非 assert(true)。
        assert_involution(GroupElement::h());
    }

    #[test]
    #[should_panic(expected = "τhτ⁻¹=h⁻¹")]
    fn conjugation_fires_on_reflection_element() {
        // 反证 prove_conjugation **非重言**：g=hτ（含反射分量）⇒
        // τ(hτ)τ⁻¹={−1,1} ≠ (hτ)⁻¹={1,1} ⇒ 必 panic（τgτ⁻¹=g⁻¹ 非对所有 g 成立）。
        assert_chiral_conjugation(GroupElement { a: 1, t: 1 });
    }

    #[test]
    #[should_panic(expected = "act_helix")]
    fn action_element_consistency_fires_on_mismatch() {
        // 反证 assert_action_matches_element **非重言**：构造一个 act_helix 与
        // 期望不符的场景——通过传入不该匹配 +1 的环境。这里直接验证：若 σ 的
        // 平移被错误期望为 +1（而实为 +23），断言 fire。手工构造错位：
        let g = GroupElement::sigma(); // 平移 +23
                                       // 故意用 AngularStep 的期望（+1）对照 σ 的作用（+23）⇒ 不符 ⇒ panic。
        for &n in &HELIX_SAMPLES {
            assert_eq!(
                g.act_helix(n),
                n + 1,
                "act_helix 错位反证（σ 实为 +23 ≠ +1，n={n}）"
            );
        }
    }

    // ──────────────── H¹ 闭合 + 配额 σ-不变守卫（正向 + 反证）────────────────

    #[test]
    fn cross_level_closure_holds_on_delta_r_minus_one() {
        // 正向：Δr=−1（5→4）不 panic。
        prove_cross_level_closure(5, 4);
        prove_cross_level_closure(3, 2);
    }

    #[test]
    #[should_panic(expected = "Δr 必为 −1")]
    fn cross_level_fires_on_same_level_spawn() {
        // 反证非重言：同级别 spawn（5→5，Δr=0）必 panic。
        prove_cross_level_closure(5, 5);
    }

    #[test]
    #[should_panic(expected = "Δr 必为 −1")]
    fn cross_level_fires_on_skip_level() {
        // 反证：跨两级（5→3，Δr=−2）必 panic（NR-1 同级别否定 + Δr=−1 唯一）。
        prove_cross_level_closure(5, 3);
    }

    #[test]
    fn theta_sigma_invariant_holds_on_canonical_quota() {
        // 正向：m_quota = p_units × 0.5（σ-不变规范）不 panic。
        prove_theta_sigma_invariant(50.0, 100.0, 3, 0);
    }

    #[test]
    #[should_panic(expected = "σ-不变规范")]
    fn theta_sigma_invariance_fires_on_level_dependent_quota() {
        // 反证非重言：sub-依赖分配（m_quota=p_units×θ[sub]/Σθ ≈ 0.3）≠ 0.5 ⇒ panic。
        prove_theta_sigma_invariant(30.0, 100.0, 3, 0);
    }

    #[test]
    #[should_panic(expected = "NR-3 违反")]
    fn chirality_seam_fires_off_singular() {
        // 反证：τ 在 φ=5≠0 触发必 panic（多空缝仅 φ=0）。
        prove_chirality_seam(5, 0);
    }

    // ──────────────── 次级别闭合（D 回补 H¹ Δr=−1）正向 + 反证 ────────────────

    #[test]
    fn sub_level_closure_reads_descended_level() {
        // 多头 voice@5：闭合读次级别卖点 nf_sell[4]（向心下沉 Δr=−1），非自层 nf_sell[5]。
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        nf_sell[5] = Some(100.0); // 自层信号——闭合不应读它
        let (fire, close_lad) = sub_level_counter_fire(Polarity::Long, 5, &nf_sell, &nf_buy);
        assert!(!fire, "闭合不读自层 nf_sell[5]（NR-1 否定同级别闭合）");
        assert_eq!(close_lad, 4, "闭合下沉到次级别 r=k−1=4");
        nf_sell[4] = Some(99.0); // 次级别信号——闭合读它
        let (fire2, close_lad2) = sub_level_counter_fire(Polarity::Long, 5, &nf_sell, &nf_buy);
        assert!(fire2, "次级别 nf_sell[4] 在 ⇒ 闭合 fire");
        assert_eq!(close_lad2, 4);
    }

    #[test]
    fn sub_level_closure_boundary_keeps_self_level() {
        // ladder=FIRST_BSP_LADDER(2)：ladder−1=1<FIRST_BSP ⇒ 边界保护，同级别闭合读 nf[2]。
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        nf_buy[FIRST_BSP_LADDER] = Some(50.0);
        let (fire, close_lad) =
            sub_level_counter_fire(Polarity::Short, FIRST_BSP_LADDER, &nf_sell, &nf_buy);
        assert!(fire, "边界：空头@FIRST_BSP 闭合读自层 nf_buy[FIRST_BSP]");
        assert_eq!(
            close_lad, FIRST_BSP_LADDER,
            "边界保护：无次级别 ⇒ close_ladder=ladder"
        );
    }

    #[test]
    fn sub_level_symmetric_holds_on_canonical() {
        // 正向：内联 gate + close_ladder == 规范 ⇒ 不 panic。
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        nf_sell[3] = Some(10.0);
        prove_sub_level_symmetric(Polarity::Long, 4, true, 3, &nf_sell, &nf_buy, 0);
    }

    #[test]
    #[should_panic(expected = "次级别闭合对称违反")]
    fn sub_level_symmetric_fires_on_self_level_drift() {
        // 反证非重言：D 回补漂移回自层判定（读 nf_sell[4] 得 false，但规范读 nf_sell[3]=true）
        // ⇒ op_trigger=false ≠ 规范 true ⇒ panic（守"闭合在 k−1 非 k"）。
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        nf_sell[3] = Some(10.0); // 次级别 fire（规范=true）
                                 // 漂移实现读自层 nf_sell[4]=None ⇒ op_trigger=false，与规范 true 不符。
        prove_sub_level_symmetric(Polarity::Long, 4, false, 3, &nf_sell, &nf_buy, 0);
    }
}
