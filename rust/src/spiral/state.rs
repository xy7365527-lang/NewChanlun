//! 螺旋状态与 D∞ 群作用（v2 第一性原理核心，L0）。
//!
//! 设计来源：`docs/spiral_engine_v2_architecture.md` §4.1/§4.2 + §1.1 三坐标表 +
//! `docs/necessity_derivation.md` T56–T59（群关系定理）。
//!
//! ## 存在论（架构 §0.1 结论）
//! 状态空间 = D∞ 群作用空间坐标 `(φ, r, ε)`；交易操作 = 群元素在状态上的作用。
//! 群 `G = D∞ = ⟨h, τ | τ²=e, τhτ⁻¹=h⁻¹⟩ ≅ ℤ⋊ℤ/2`，`σ := h²³`。
//!
//! ## 两层结构（关键区分）
//! - **代数层** `GroupElement{a,t}`（正规形 `hᵃτᵗ`）：群结构的**真值载体**。
//!   群关系 `τ²=e`/`τhτ⁻¹=h⁻¹`/`h²³=σ` 是 `compose`/`power` 代数的**推论**
//!   （被正规形吸收，无法构造违反 D∞ 的元素）。配套 `act_helix`（忠实表示
//!   `D∞ ↪ Isom(ℤ)`）用于群关系的作用层交叉验证。
//! - **语义层** `GroupAction`（缠论生成元）：带 NR 禁区守卫的状态转移
//!   （τ 仅在 φ=0、σ⁻¹ 在 r=0 无像）。`apply` 返回新状态（immutable）。
//!
//! ## 认识论等级（formalization-validity-domain）
//! 全文件 **L0**（纯代数/定义层，零信息增量）。群关系由 `prove.rs` 运行时
//! panic 守卫。`23`/`11` 的具体值：`23` = A-链建模约定（L0 定量，架构 §12.4-4）；
//! `MAX_LADDER=11` = NR-5 有界塔（L0 结构）+ 工程截断（非 L0），复用 `types.rs`
//! 单一真相源以防漂移。
//!
//! ## 527号命名分离
//! 本 `eps`（手性 ℤ/2）≠ 527号 `σ∈{±1}`（走势方向态）；本 `sigma` 永远指
//! 径向跃迁 `h²³`。径向算子命名 `sigma`/手性命名 `eps`/`tau`，禁用裸 `sigma`
//! 指代手性。

use crate::trading::types::MAX_LADDER;

/// 角向环数（ℤ/23 商，T58）。一个走势绕满一圈角向 = 23 个辩证环节
/// （`concept_movement_chain.md` 23 环）。`h²³=σ`：23 步角向推进 = 1 步径向跃迁。
///
/// 认识论：L0 定量 A-链建模约定（架构 §12.4-4）——若链重切分为 N 环则 `hᴺ=σ`，
/// 23 的具体值不作字面 panic（与 φ 隐式同口径，架构 §8.5）；可证伪的是
/// **基本域有限性**（`prove_h23_eq_sigma` 守 power 实现 + 作用一致性）。
pub const N_CONCEPT_RINGS: i32 = 23;

// ════════════════════════════ 螺旋状态（φ, r, ε）════════════════════════════

/// 螺旋状态 = D∞ 群作用空间坐标。三坐标严格对应（角向, 径向, 手性）。
///
/// | 分量 | 取值空间 | 群生成元 | 缠论语义 |
/// |------|---------|---------|---------|
/// | `phi` 角向 | ℤ/23ℤ | `h` | 走势进度，唯一奇点 φ=0（背驰∧手性翻转，T47）|
/// | `r` 径向 | 0..MAX_LADDER | `σ=h²³` | 级别 = 圈数 = λᵏ（笔→线段→走势→…）|
/// | `eps` 手性 | {±1} | `τ` | 多/空，仅在 φ=0 翻转（NR-3）|
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpiralState {
    /// 角向相位 ∈ 0..N_CONCEPT_RINGS（ℤ/23ℤ），唯一奇点 `phi==0`。
    pub phi: u8,
    /// 径向级别 ∈ 0..MAX_LADDER（有界塔）。径向口径：0=bar(a0)/1=笔/2=线段/
    /// 3=move(L1)/4..=递归层（与 `types.rs::ladder_name` 同口径）。
    pub r: u8,
    /// 手性 ∈ {+1, −1}（多/空）。仅在 `phi==0` 翻转（NR-3 手性缝）。
    pub eps: i8,
}

impl SpiralState {
    /// 构造一个螺旋状态。`eps` 非 ±1 在 debug 下断言失败（值域契约）。
    pub fn new(phi: u8, r: u8, eps: i8) -> Self {
        debug_assert!(eps == 1 || eps == -1, "eps 须 ∈ {{+1,-1}}，得 {eps}");
        debug_assert!((phi as i32) < N_CONCEPT_RINGS, "phi 须 < 23，得 {phi}");
        debug_assert!((r as usize) < MAX_LADDER, "r 须 < MAX_LADDER，得 {r}");
        SpiralState { phi, r, eps }
    }

    /// 根入场初态：`(0, source, +1)`（φ=0 奇点 + 径向源 + 多头），架构 §5.1。
    pub fn root(source: u8) -> Self {
        SpiralState::new(0, source, 1)
    }

    /// helix 编码：`n = r·23 + φ`。把"绕一圈角向 = 升一级径向"（`h²³=σ`）
    /// 编码为单一平移（`h: n↦n+1`、`σ=h²³: n↦n+23`）。
    pub fn helix(&self) -> u32 {
        (self.r as u32) * (N_CONCEPT_RINGS as u32) + (self.phi as u32)
    }

    /// 从 helix 坐标解码（`eps` 须显式带入——helix 线不含手性维度）。
    pub fn from_helix(n: u32, eps: i8) -> Self {
        let rings = N_CONCEPT_RINGS as u32;
        SpiralState::new((n % rings) as u8, (n / rings) as u8, eps)
    }

    /// φ=0：唯一奇点（背驰 ∧ 手性可翻转，T47）。操作层只在此触发（买卖点）。
    pub fn is_singular(&self) -> bool {
        self.phi == 0
    }
}

// ════════════════════════════ D∞ 群元素（代数层）════════════════════════════

/// D∞ 群元素的正规形：`g = hᵃ · τᵗ`（`a∈ℤ`, `t∈{0,1}`）。`σ=h²³` 是 `a=23,t=0`
/// 的特例。
///
/// **正规形唯一**：任意 `{a,t}` ↔ 唯一群元素。`compose` 始终归约到正规形 ⟹
/// 群关系 `τ²=e`/`τhτ⁻¹=h⁻¹` 被正规形吸收（无法构造违反 D∞ 的元素）。
/// 因此 `PartialEq` 直接比较 `{a,t}` 即为群元素相等。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupElement {
    /// 角向幂次（`h` 的指数）。`σ=h²³` ⟹ `a=23`。
    pub a: i32,
    /// 手性位（`τ` 的指数，∈ {0,1}）。`t=1` ⟹ 含一次反射（reflection 元）。
    pub t: u8,
}

impl GroupElement {
    /// 单位元 `e = h⁰τ⁰`。
    pub fn identity() -> Self {
        GroupElement { a: 0, t: 0 }
    }

    /// 角向推进生成元 `h`（`a=1, t=0`）。
    pub fn h() -> Self {
        GroupElement { a: 1, t: 0 }
    }

    /// 手性翻转生成元 `τ`（`a=0, t=1`），R₂ 对合。
    pub fn tau() -> Self {
        GroupElement { a: 0, t: 1 }
    }

    /// 径向跃迁元 `σ := h²³`（`a=N_CONCEPT_RINGS, t=0`）。
    ///
    /// **独立于 `h.power(23)` 定义**（字面 `a=23`）——这使 `prove_h23_eq_sigma`
    /// **非重言**：它验证的是"`power` 实现 + helix 作用"使 `h²³` 等于这个独立
    /// 定义的 `σ`，传入错误环数（如 22）会 fire。
    pub fn sigma() -> Self {
        GroupElement {
            a: N_CONCEPT_RINGS,
            t: 0,
        }
    }

    /// 群乘法（正规形归约）。
    ///
    /// D∞ 乘法规则：`τh = h⁻¹τ`（由 `τhτ⁻¹=h⁻¹`）⟹
    /// `τᵗ¹ hᵃ² = h^{(−1)^{t1}·a2} τᵗ¹`，故
    /// `(hᵃ¹τᵗ¹)(hᵃ²τᵗ²) = h^{a1+(−1)^{t1}·a2} τ^{(t1+t2) mod 2}`。
    pub fn compose(self, other: GroupElement) -> GroupElement {
        let sign = if self.t == 0 { 1 } else { -1 };
        GroupElement {
            a: self.a + sign * other.a,
            t: (self.t + other.t) % 2,
        }
    }

    /// 逆元。`t=0`：`(hᵃ)⁻¹ = h⁻ᵃ`；`t=1`：reflection 元自逆（`(hᵃτ)² = e`）。
    pub fn inverse(self) -> GroupElement {
        if self.t == 0 {
            GroupElement { a: -self.a, t: 0 }
        } else {
            // hᵃτ 是反射，自逆：compose({a,1},{a,1}) = {a + (−1)·a, 0} = {0,0} = e。
            GroupElement { a: self.a, t: 1 }
        }
    }

    /// 群幂 `gⁿ`（`n` 可负，`g⁻ⁿ = (g⁻¹)ⁿ`）。通用循环（不依赖闭式，
    /// 让 `prove_h23` 经由 `compose` 链验证实现正确性）。
    pub fn power(self, n: i32) -> GroupElement {
        let (base, count) = if n >= 0 {
            (self, n)
        } else {
            (self.inverse(), -n)
        };
        let mut acc = GroupElement::identity();
        for _ in 0..count {
            acc = acc.compose(base);
        }
        acc
    }

    /// 忠实表示 `D∞ ↪ Isom(ℤ)`（作用于 helix 线 ℤ）：`g·n = (−1)ᵗ·n + a`。
    /// `h: n↦n+1`（平移）、`τ: n↦−n`（反射）、`σ=h²³: n↦n+23`。
    ///
    /// 同态性：`act_helix(g1, act_helix(g2, n)) == act_helix(compose(g1,g2), n)`
    /// （由 `(−1)^{t1}((−1)^{t2}n+a2)+a1 = (−1)^{t1+t2}n + ((−1)^{t1}a2+a1)`）。
    /// 用于群关系在**作用层**的交叉验证（非重言强化）。
    pub fn act_helix(self, n: i64) -> i64 {
        let sign = if self.t == 0 { 1 } else { -1 };
        sign * n + (self.a as i64)
    }
}

// ════════════════════════════ 群作用（语义层）════════════════════════════

/// 合法操作的群生成元（架构 §4.2）。映射到 `GroupElement` 经 `to_element`；
/// 语义状态转移经 `apply`（带 NR 禁区守卫）。
///
/// | 变体 | 生成元 | NR 禁区 |
/// |------|--------|---------|
/// | `AngularStep` | `h: φ↦φ+1` | — （唯一前向算子）|
/// | `ChiralSeam` | `τ: ε↦−ε` | NR-3：仅 φ=0 合法 |
/// | `RadialAscend` | `σ: r↦r+1` | NR-5：r=MAX_LADDER−1 顶层无像 |
/// | `RadialDescend` | `σ⁻¹: r↦r−1` | NR-5：r=0（a0）底层无像；**H¹ 生成元 Δr=−1**|
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupAction {
    /// `h`：角向推进（前向）。helix `n↦n+1`，φ 满 23 自然进位 r（`h²³=σ` 显形）。
    AngularStep,
    /// `τ`：手性翻转（in-place，r 不变，M=N，架构 §5.2 C 操作语义）。仅 φ=0。
    ChiralSeam,
    /// `σ`：级别涌现（根向上生长）。r↦r+1。
    RadialAscend,
    /// `σ⁻¹`：向心下沉（闭合落点）。r↦r−1。`Δr=−1` = H¹ 生成元本身（A1）。
    RadialDescend,
}

/// 群作用违反 NR 禁区（语义层的非法转移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupViolation {
    /// τ 在 φ≠0 触发（NR-3：多空两书仅 φ=0 手性缝桥接）。
    ChiralSeamOffSingular { phi: u8 },
    /// σ⁻¹ 在 r=0 触发（NR-5：底层 a0 分辨率不可约，无 σ⁻¹ 像）。
    RadialUnderflowAtFloor,
    /// σ（或角向进位）越过 r=MAX_LADDER−1（NR-5：顶层 σ 无像，有界塔截断）。
    RadialOverflowAtCeiling { attempted_r: usize },
}

impl GroupAction {
    /// 映射到代数层群元素（`to_element` ↔ `apply` 经 `act_helix` 一致——见
    /// `prove.rs::assert_action_matches_element`）。
    pub fn to_element(self) -> GroupElement {
        match self {
            GroupAction::AngularStep => GroupElement::h(),
            GroupAction::ChiralSeam => GroupElement::tau(),
            GroupAction::RadialAscend => GroupElement::sigma(),
            GroupAction::RadialDescend => GroupElement::sigma().inverse(),
        }
    }

    /// 群作用：返回新状态（immutable，coding-style）。非法作用返回 `Err`。
    ///
    /// 语义状态转移（带 NR 守卫），**非纯 `act_helix`**——`τ` 是 in-place 手性
    /// 翻转（r 不变，§5.2），与代数层 `τ: n↦−n`（级别线反射）分离（同 527号
    /// 能指碰撞：同一 `τ` 在代数层与操作层所指不同）。
    pub fn apply(self, s: SpiralState) -> Result<SpiralState, GroupViolation> {
        match self {
            GroupAction::AngularStep => {
                // helix n↦n+1：φ 满 23 自然进位 r（"绕一圈角向 = 升一级径向"）。
                let n = s.helix() + 1;
                let new = SpiralState::from_helix(n, s.eps);
                if (new.r as usize) >= MAX_LADDER {
                    Err(GroupViolation::RadialOverflowAtCeiling {
                        attempted_r: new.r as usize,
                    })
                } else {
                    Ok(new)
                }
            }
            GroupAction::ChiralSeam => {
                // NR-3：τ 仅在 φ=0 合法；in-place 翻 ε（r/φ 不变，M=N）。
                if !s.is_singular() {
                    Err(GroupViolation::ChiralSeamOffSingular { phi: s.phi })
                } else {
                    Ok(SpiralState {
                        phi: s.phi,
                        r: s.r,
                        eps: -s.eps,
                    })
                }
            }
            GroupAction::RadialAscend => {
                // σ: r↦r+1（级别涌现）。NR-5：r=MAX_LADDER−1 顶层无像。
                let nr = s.r as usize + 1;
                if nr >= MAX_LADDER {
                    Err(GroupViolation::RadialOverflowAtCeiling { attempted_r: nr })
                } else {
                    Ok(SpiralState {
                        phi: s.phi,
                        r: nr as u8,
                        eps: s.eps,
                    })
                }
            }
            GroupAction::RadialDescend => {
                // σ⁻¹: r↦r−1（向心下沉，Δr=−1）。NR-5：r=0（a0）无像。
                if s.r == 0 {
                    Err(GroupViolation::RadialUnderflowAtFloor)
                } else {
                    Ok(SpiralState {
                        phi: s.phi,
                        r: s.r - 1,
                        eps: s.eps,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────── SpiralState 基本操作 ────────────────

    #[test]
    fn helix_roundtrip() {
        // helix 编码 ↔ 解码逐位往返（eps 显式带入）。
        for r in 0..(MAX_LADDER as u8) {
            for phi in 0..(N_CONCEPT_RINGS as u8) {
                for &eps in &[1i8, -1] {
                    let s = SpiralState::new(phi, r, eps);
                    let n = s.helix();
                    assert_eq!(SpiralState::from_helix(n, eps), s, "helix 往返破：{s:?}");
                }
            }
        }
    }

    #[test]
    fn root_is_singular_long() {
        let s = SpiralState::root(3);
        assert!(s.is_singular());
        assert_eq!((s.phi, s.r, s.eps), (0, 3, 1));
    }

    // ──────────────── GroupElement 代数 ────────────────

    #[test]
    fn compose_identity_neutral() {
        let e = GroupElement::identity();
        for g in [
            GroupElement::h(),
            GroupElement::tau(),
            GroupElement::sigma(),
        ] {
            assert_eq!(e.compose(g), g, "e·g≠g：{g:?}");
            assert_eq!(g.compose(e), g, "g·e≠g：{g:?}");
        }
    }

    #[test]
    fn inverse_yields_identity() {
        for g in [
            GroupElement::h(),
            GroupElement::tau(),
            GroupElement::sigma(),
            GroupElement { a: 5, t: 1 },
            GroupElement { a: -7, t: 0 },
        ] {
            assert_eq!(
                g.compose(g.inverse()),
                GroupElement::identity(),
                "g·g⁻¹≠e：{g:?}"
            );
            assert_eq!(
                g.inverse().compose(g),
                GroupElement::identity(),
                "g⁻¹·g≠e：{g:?}"
            );
        }
    }

    #[test]
    fn act_helix_homomorphism() {
        // 同态性：act(g1, act(g2, n)) == act(g1·g2, n)（忠实表示）。
        let elems = [
            GroupElement::h(),
            GroupElement::tau(),
            GroupElement::sigma(),
            GroupElement { a: 3, t: 1 },
            GroupElement { a: -2, t: 0 },
        ];
        for g1 in elems {
            for g2 in elems {
                for n in -10i64..=10 {
                    assert_eq!(
                        g1.act_helix(g2.act_helix(n)),
                        g1.compose(g2).act_helix(n),
                        "同态破：g1={g1:?} g2={g2:?} n={n}"
                    );
                }
            }
        }
    }

    // ──────────────── 群关系在状态空间的显形 ────────────────

    #[test]
    fn angular_step_23_equals_radial_ascend() {
        // h²³=σ 状态层显形：从 (0,r,ε) 连续 AngularStep 23 次 == RadialAscend 一次。
        for r in 0..(MAX_LADDER as u8 - 1) {
            for &eps in &[1i8, -1] {
                let s0 = SpiralState::new(0, r, eps);
                let mut s = s0;
                for _ in 0..N_CONCEPT_RINGS {
                    s = GroupAction::AngularStep.apply(s).expect("角向推进未越界");
                }
                let by_sigma = GroupAction::RadialAscend.apply(s0).expect("σ 未越界");
                assert_eq!(s, by_sigma, "h²³≠σ 状态显形：r={r} eps={eps}");
                assert_eq!((s.phi, s.r, s.eps), (0, r + 1, eps));
            }
        }
    }

    #[test]
    fn chiral_seam_involution_in_state() {
        // τ²=e 状态层显形：φ=0 翻两次回原（r/units 不变由 §5.2 in-place 保证）。
        let s = SpiralState::new(0, 4, 1);
        let once = GroupAction::ChiralSeam.apply(s).expect("φ=0 翻转合法");
        assert_eq!(once.eps, -1);
        let twice = GroupAction::ChiralSeam.apply(once).expect("再翻合法");
        assert_eq!(twice, s, "τ²≠e 状态显形");
    }

    // ──────────────── NR 禁区守卫 ────────────────

    #[test]
    fn chiral_seam_off_singular_is_violation() {
        // NR-3：τ 在 φ≠0 非法。
        let s = SpiralState::new(5, 3, 1);
        assert_eq!(
            GroupAction::ChiralSeam.apply(s),
            Err(GroupViolation::ChiralSeamOffSingular { phi: 5 })
        );
    }

    #[test]
    fn radial_descend_at_floor_is_violation() {
        // NR-5：σ⁻¹ 在 r=0（a0）无像。
        let s = SpiralState::new(0, 0, 1);
        assert_eq!(
            GroupAction::RadialDescend.apply(s),
            Err(GroupViolation::RadialUnderflowAtFloor)
        );
    }

    #[test]
    fn radial_ascend_at_ceiling_is_violation() {
        // NR-5：σ 在 r=MAX_LADDER−1（顶层）无像。
        let s = SpiralState::new(0, MAX_LADDER as u8 - 1, 1);
        assert_eq!(
            GroupAction::RadialAscend.apply(s),
            Err(GroupViolation::RadialOverflowAtCeiling {
                attempted_r: MAX_LADDER
            })
        );
    }

    #[test]
    fn radial_descend_is_delta_r_minus_one() {
        // H¹ 生成元 Δr=−1（A1）：σ⁻¹ 把 r 严格降一。
        let s = SpiralState::new(0, 5, 1);
        let descended = GroupAction::RadialDescend.apply(s).expect("r>0 合法");
        assert_eq!(descended.r, s.r - 1, "Δr≠−1");
    }
}
