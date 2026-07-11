//! 分账本头寸空间 P^sep + 净额映射 Net + 分账本吃到 Eat^sep（工作单元 R2，C25/C26/C29）。
//!
//! ## 本文件做什么：建立分账本头寸空间这一新代数结构的 rust 实装
//!
//! 唯一 canonical 源：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
//! §B 表 C25/C26/C29 行（23 页权威 PDF 页11–13）。契约锚（只读，不改 .lean）：
//! - C25/C26 P^sep / Net → **`formal/Origin/SeparateLedger.lean`**（`Leg`/`SepPosition`/`legNet`/
//!   `Net`/`legZero`/`legLong`/`legShort`/`legHedged` 逐一对齐；本 rust 镜像该 Lean 接口语义）。
//! - C29 Eat^sep → spec §四 页13（向上元素由多头腿覆盖、向下由空头腿覆盖、整操作区间、不抵消）；
//!   对照 `formal/Origin/VoiceEat.lean` 的声部版 `Eat`（C06，σ_v=ε_b 同向覆盖）。
//!
//! ### C25 · 分账本头寸空间 P^sep（PDF §一 页11–12）
//! ```text
//! P^sep = ∏_{v∈V} (R≥0·e⁺_v ⊕ R≥0·e⁻_v)
//! ```
//! 每个声部 v 有两个**独立的非负坐标**：多头腿 e⁺_v 与空头腿 e⁻_v。头寸
//! ```text
//! p_t = (q⁺_{v,t}, q⁻_{v,t})_{v∈V}
//! ```
//! **关键（区别于净额账本）**：同标的同量多空双开 `(Q,Q) ≡ (0,0)` **不成立**——
//! `Q·e⁺_v + Q·e⁻_v` 是两个独立头寸腿，是非零头寸（仅当 Q=0 时才退化为零）。
//!
//! ### C26 · 净额映射 Net（PDF §一 页12）
//! ```text
//! Net(p_t) = Σ_{v∈V} (q⁺_{v,t} − q⁻_{v,t})
//! ```
//! 净额映射把 `Q·e⁺ + Q·e⁻ ↦ 0`（净额下退化）。**分账本语义不做此映射**——P^sep 保留两条腿，
//! 不经 Net 折叠。Net 是从 P^sep 到 ℤ 的**有损投影**（非单射，[`net_not_injective_witness`] 坐实）。
//!
//! ### C29 · 分账本吃到 Eat^sep（PDF §四 页13）
//! ```text
//! Eat^sep(e) ⟺ ∀t∈[λ_e,ρ_e):
//!   (ε_e=+1 ⟹ q⁺_{ν(e),t}=s_e ∧ q⁻_{ν(e),t}=0) ∧
//!   (ε_e=-1 ⟹ q⁺_{ν(e),t}=0 ∧ q⁻_{ν(e),t}=s_e)
//! ```
//! 向上元素（ε_e=+1=`Side::Long`）由**多头腿**覆盖、向下元素（ε_e=-1=`Side::Short`）由**空头腿**覆盖；
//! 覆盖发生在该元素整操作区间；多空头寸**不净额抵消**（C25 的直接推论：双开两腿都在）。
//!
//! ## ★★C26 净额映射退化是有效域分离点（230号直积退化谱系，强制标注）
//!
//! C26 净额映射使分账本双开 `(Q,Q)↦0` 退化，与 230号"直积在概率度量下退化为 <3 自由度"**同构**——
//! 两腿（多/空）在净额 ℤ 上塌缩为单点 0。本文件**拒绝净额映射作为分账本的表示**：分账本头寸
//! 的相等是逐声部逐坐标相等（`Leg` 的 `PartialEq`），不经过 Net。把 P^sep 折成净额标量 = 230号
//! 直积退化重演（投影丢失多/空两腿的区分信息）。这是 R2 区别于现有净额账本的**有效域分离点**：
//! - 净额账本（`nautilus/account_adapter.rs` net_position / `strategy/ledger.rs` LedgerComp）：
//!   度量净敞口，双开后净仓可低（这正是 C26 净额映射退化要避免的）。
//! - 分账本 P^sep（本文件）：保留每声部多/空两独立腿，双开后两腿都在、不抵消。
//! no-workaround 裁定：P^sep 与净额账本**非冲突，是层分离**（SeparateLedger.lean §隔离声明一致）——
//! 它们是不同有效域的不同结构，正交并置，不强行统一进净额账本。
//!
//! ## 认识论等级（formalization-validity-domain / 231号，强制标注）
//!
//! **全文件 L0/L1**（结构镜像：rust 类型/算子与 SeparateLedger.lean 定义结构对齐 = 验证管线
//! 正确性，零信息增量）。`cargo test` 通过 = P^sep 的代数结构自洽 + 净额退化点（双开 (Q,Q)↦0）
//! 在结构上成立 + Eat^sep 判定逻辑正确，**不**是任何缠论盈利 / 实盘有效声明（那是 L2/L3，真实
//! 数据回测才可否证）。q⁺/q⁻/target 用 u64 承载手数/股数（R≥0 的整数最小单位），**不臆造价格、
//! 不承载实盘盈亏**（spec §十四：数学名称是"分账本声部级全元素覆盖"，**不是**"净资产级每笔盈利"）。
//!
//! ## 隔离声明（standalone）
//! 本文件**不依赖** `strategy::ledger`（净额账本 R=Π-A-W）、`nautilus::account_adapter`
//! （net_position），只复用 `types::Side`（方向域 ε_e）。P^sep 是与上述账本正交的新代数结构。
//!
//! 谱系：C25/C26/C29（PDF §一/§四 页11–13）→ 230号（直积退化：双开 (Q,Q)↦0 净额退化）→
//!       本文件 R2（P^sep rust 根 + Eat^sep 判定，对照 Lean SeparateLedger/VoiceEat）。

use crate::theta_v0::types::Side;

/// ★头寸腿 `Leg`（L0，C25 单声部双坐标）：声部 v 的多头腿 + 空头腿。
///
/// - `q_plus  : u64`：多头坐标 q⁺_v（e⁺_v 方向的非负持仓量，R≥0 的整数最小单位）。
/// - `q_minus : u64`：空头坐标 q⁻_v（e⁻_v 方向的非负持仓量）。
///
/// ★**两坐标独立**——这正是"多空作独立头寸坐标"的形式载体（契约锚 `SeparateLedger.Leg`）。
/// `(Q,Q)` 与 `(0,0)` 当且仅当 Q=0 时相等（逐分量 `PartialEq`），故 Q>0 的同股数双开是**非零**
/// 头寸腿（[`Leg::is_zero`] 为 false）——区别于净额账本把 (Q,Q) 抹成 0。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leg {
    pub q_plus: u64,
    pub q_minus: u64,
}

impl Leg {
    /// ★零腿 `leg_zero`（L0，契约锚 `SeparateLedger.legZero`）：(q⁺,q⁻)=(0,0)——无任何持仓。
    pub const fn zero() -> Leg {
        Leg { q_plus: 0, q_minus: 0 }
    }

    /// ★多头腿 `leg_long(q)`（L0，q·e⁺_v，契约锚 `SeparateLedger.legLong`）：纯多头持仓 (q, 0)。
    pub const fn long(q: u64) -> Leg {
        Leg { q_plus: q, q_minus: 0 }
    }

    /// ★空头腿 `leg_short(q)`（L0，q·e⁻_v，契约锚 `SeparateLedger.legShort`）：纯空头持仓 (0, q)。
    pub const fn short(q: u64) -> Leg {
        Leg { q_plus: 0, q_minus: q }
    }

    /// ★对冲腿 `leg_hedged(q)`（L0，q·e⁺_v + q·e⁻_v，契约锚 `SeparateLedger.legHedged`）：
    /// 同股数多空双开 (q, q)。q>0 时**非零**（[`Leg::is_zero`] false）——双开两腿都在、不抵消。
    pub const fn hedged(q: u64) -> Leg {
        Leg { q_plus: q, q_minus: q }
    }

    /// ★零腿判定 `is_zero`（L0）：两坐标皆 0（多空腿皆空）。对照 `SeparateLedger.legZero` 相等。
    ///
    /// ★关键：`Leg::hedged(Q).is_zero()` 当且仅当 Q=0 时为 true——双开 (Q,Q) (Q>0) **不**是零腿
    /// （净额视角下 Net=0 但分账本视角下非零，[`Leg::net`] 与本判定的分离即 C26 退化点）。
    pub const fn is_zero(&self) -> bool {
        self.q_plus == 0 && self.q_minus == 0
    }

    /// ★单腿净额 `net`（L0，C26 单声部，契约锚 `SeparateLedger.legNet`）：q⁺−q⁻（多头正、空头负）。
    ///
    /// 用 i128 承载（u64 差可负且不溢出）。`Leg::hedged(Q).net() = 0`——双开腿净额退化为 0
    /// （C26 净额映射 `Q·e⁺ + Q·e⁻ ↦ 0` 的单声部形式，★直接对应 230号直积退化：两腿在净额上
    /// 塌缩为单点）。**此函数是从分账本投影到净额视角的桥，分账本语义本身不经过它**。
    pub const fn net(&self) -> i128 {
        self.q_plus as i128 - self.q_minus as i128
    }
}

/// ★分账本头寸 `SepPosition`（L0，C25 直积元素，契约锚 `SeparateLedger.SepPosition = List Leg`）：
/// 声部族上每声部一条头寸腿。`p_t = (q⁺_{v,t}, q⁻_{v,t})_{v∈V}`——有限声部族的多空双坐标族。
///
/// 用 `Vec<Leg>` 承载有限声部族（声部索引 = `Vec` 下标 v∈[0, len)）。`SepPosition` 的相等是
/// 逐声部 `Leg` 相等（`Vec<Leg>` 的 `PartialEq`，不经 Net 折叠）。
pub type SepPosition = Vec<Leg>;

/// ★空分账本头寸 `sep_zero(n)`（L0，契约锚 `SeparateLedger.sepZero`）：n 个声部腿皆零（P^sep 原点）。
pub fn sep_zero(n: usize) -> SepPosition {
    vec![Leg::zero(); n]
}

/// ★分账本头寸净额 `net`（L0，C26，契约锚 `SeparateLedger.Net`）：Σ_v (q⁺_v − q⁻_v)。
///
/// 各声部单腿净额的代数和——多空在净额上抵消（双开两腿符号相反，故净额可塌缩）。
/// ★**有损投影**（非单射，[`net_not_injective_witness`] 坐实）：不同 P^sep 头寸可同净额，故
/// "用净额坐标表示分账本头寸"在结构上不可行（投影丢失多/空两腿的区分信息 = 230号直积退化）。
pub fn net(p: &SepPosition) -> i128 {
    p.iter().map(Leg::net).sum()
}

/// ★缠论语法元素 `SyntaxElement`（L0，C27/C28 摘要）：分账本吃到 Eat^sep 的判定输入。
///
/// 契约锚：C27 元素 `e = (I_e, ε_e, ℓ_e, par(e))` + C28 唯一头寸腿映射 ν:E→V。本结构承载
/// Eat^sep(e) 判定所需的最小四元组（区间 [λ,ρ) + 方向 ε_e + 目标单位 s_e + 规范腿索引 ν(e)）。
/// 级别 ℓ_e / 父 par(e) 不影响单元素 Eat^sep 判定（它们在自相似递归 C39 用），故本摘要不含。
///
/// - `lo : u64`（左界 λ_e）、`hi : u64`（右界 ρ_e），半开区间 I_e=[lo, hi)，要求 lo < hi。
/// - `dir : Side`（方向 ε_e：`Long`=+1 向上=多头腿覆盖、`Short`=-1 向下=空头腿覆盖）。
/// - `target : u64`（目标单位数 s_e>0，C28「σ_{ν(e)}=ε_e ∧ s_e>0」的基准）。
/// - `nu : usize`（规范头寸腿索引 ν(e)，C28 单射映射 e→V 的像；声部族 `SepPosition` 的下标）。
///
/// ★诚实标注（formalization-validity-domain）：dir/target/nu 是元素的结构参数（由缠论元素管线 +
/// C28 ν 单射算出），本结构承载它们以使 Eat^sep 谓词可表达。**不臆造价格，不承载实盘盈亏**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxElement {
    pub lo: u64,
    pub hi: u64,
    pub dir: Side,
    pub target: u64,
    pub nu: usize,
}

impl SyntaxElement {
    /// ★良构判定 `is_well_formed`（L0）：区间非空（lo<hi）∧ 目标单位为正（target>0）。
    /// 对照 Lean `Stroke.lt` + `target_pos`（笔的 L0 良构约束）。
    pub const fn is_well_formed(&self) -> bool {
        self.lo < self.hi && self.target > 0
    }

    /// ★时刻成员 `in_interval(t)`（L0，对照 Lean `InStroke`）：t 落在半开区间 [lo, hi) 内。
    pub const fn in_interval(&self, t: u64) -> bool {
        self.lo <= t && t < self.hi
    }
}

/// ★C29 分账本吃到 `eat_sep`（L0，核心谓词，契约锚 spec §四 页13）：元素 e 被其规范头寸腿 ν(e)
/// 在整操作区间 [λ_e,ρ_e) 内**按方向覆盖**——
///   - ε_e=+1（`Side::Long`，向上）⟹ ∀t∈[λ,ρ): q⁺_{ν(e),t}=s_e ∧ q⁻_{ν(e),t}=0（**多头腿**覆盖）；
///   - ε_e=-1（`Side::Short`，向下）⟹ ∀t∈[λ,ρ): q⁺_{ν(e),t}=0 ∧ q⁻_{ν(e),t}=s_e（**空头腿**覆盖）。
///
/// 输入 `position_at(t) -> &SepPosition`：时刻 t 的分账本头寸读出（数据流给定值，对照 Lean
/// `EatEnv.state`——其实时取值来自 §9 开平状态机，本函数作给定承载，不臆造）。判定遍历整区间
/// 每个离散时刻，验证规范腿 ν(e) 的两坐标恰好按方向取值。
///
/// ★语义：Eat^sep 是**语法覆盖谓词**——"整区间内规范腿按方向持目标单位、反向坐标为零、多空不
/// 抵消"是结构事实。**不是**盈利谓词（盈利 G^sep_e=s_e·ε_e(P_ρ−P_λ) 另证，实盘有效性是 L2+）。
///
/// ★边界：`position_at(t)` 返回的 `SepPosition` 长度须 > ν(e)（规范腿索引在声部族内）；越界
/// 视为不吃到（返回 false，规范腿不存在 ⟹ 未覆盖）。元素须良构（[`SyntaxElement::is_well_formed`]）。
pub fn eat_sep<'a, F>(e: &SyntaxElement, position_at: F) -> bool
where
    F: Fn(u64) -> &'a SepPosition,
{
    if !e.is_well_formed() {
        return false;
    }
    // 遍历整操作区间 [lo, hi) 每个离散时刻，验证规范腿 ν(e) 按方向覆盖。
    let mut t = e.lo;
    while t < e.hi {
        let p = position_at(t);
        let leg = match p.get(e.nu) {
            Some(l) => l,
            None => return false, // 规范腿索引越界 ⟹ 腿不存在 ⟹ 未覆盖
        };
        let covered = match e.dir {
            // 向上元素（ε_e=+1）：多头腿覆盖——q⁺=s_e ∧ q⁻=0。
            Side::Long => leg.q_plus == e.target && leg.q_minus == 0,
            // 向下元素（ε_e=-1）：空头腿覆盖——q⁺=0 ∧ q⁻=s_e。
            Side::Short => leg.q_plus == 0 && leg.q_minus == e.target,
        };
        if !covered {
            return false;
        }
        t += 1;
    }
    true
}

/// ★C28 唯一头寸腿单射检查 `nu_injective_on`（L0，契约锚 C28「ν:E→V 单射 e≠e′⟹ν(e)≠ν(e′)」）：
/// 给定一组语法元素，检查它们的规范腿索引 ν(e) 两两不同（每语法元素有自己的规范头寸腿）。
///
/// 这是 spec §九「ν 单射 ⟹ 规范吃笔腿唯一」的 rust 判定——对照 Lean `VoiceEat.DepthInjOn`
/// （声部版用 depth 单射；本分账本版用 ν 直接单射）。`true` ⟺ 元素的 nu 字段构成单射。
pub fn nu_injective_on(elements: &[SyntaxElement]) -> bool {
    for (i, a) in elements.iter().enumerate() {
        for b in &elements[i + 1..] {
            if a.nu == b.nu {
                return false;
            }
        }
    }
    true
}

/// ★不删父算子 `open_child_leg`（L0，C32/C39「(σQ,0)→(σQ,-σQ)」，契约锚
/// `SeparateLedger.parent_leg_survives_child_open`）：在已持父多头腿 (Q,0) 的声部坐标上开同股数
/// 子空头腿，腿变为 (Q,Q)——**父多头分量 q⁺ 不减**（写在独立的 q⁻ 坐标），新增空头分量 q⁻=Q。
///
/// 这是 §十一自相似递归（C39）"子声部开启不删父声部"的代数算子：子腿写在独立 q⁻ 坐标，父腿
/// q⁺ 不被触碰。返回新 [`Leg`]（immutable，不就地修改）。
///
/// ★关键（C39 不删父机制）：结果 (Q,Q) 的 [`Leg::is_zero`] 为 false（Q>0），故子开不删父——
/// 仅在净额映射 [`Leg::net`]=0 下退化（C26 退化点）。这正是 P^sep 允许 `Q·e⁺+Q·e⁻≡0` 不删父
/// 的代数前提，区别于净额账本"翻转丢递归信息"。
pub fn open_child_leg(parent_long: &Leg, q_child_short: u64) -> Leg {
    Leg {
        q_plus: parent_long.q_plus,         // 父多头分量不减（不被子空头吞噬）
        q_minus: parent_long.q_minus + q_child_short, // 子空头写在独立 q⁻ 坐标
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★C25 双开非零（契约锚 `hedged_leg_nonzero`）：Q>0 ⟹ 双开腿 (Q,Q) ≠ 零腿——P^sep 保留两腿。
    #[test]
    fn hedged_leg_nonzero() {
        let l = Leg::hedged(5);
        assert!(!l.is_zero(), "双开 (5,5) 在 P^sep 非零（两腿都在，不抵消）");
        assert_ne!(l, Leg::zero());
        // 仅 Q=0 时双开退化为零腿
        assert!(Leg::hedged(0).is_zero());
    }

    /// ★C26 双开净额退化（契约锚 `hedged_leg_net_zero`）：双开腿 (Q,Q) 净额 = 0（净额视角退化）。
    /// ★直接对应 230号直积退化：两腿在净额 ℤ 上塌缩为单点 0。
    #[test]
    fn hedged_leg_net_zero() {
        assert_eq!(Leg::hedged(5).net(), 0, "双开净额退化为 0（C26）");
        assert_eq!(Leg::hedged(100).net(), 0);
    }

    /// ★C25/C26 分离点（契约锚 `hedged_leg_nonzero_but_net_zero`，W13 主引用）：
    /// 双开腿在 P^sep 非零 **但** 净额为 0——分账本语义与净额语义的精确分离点。
    #[test]
    fn hedged_nonzero_but_net_zero() {
        let l = Leg::hedged(7);
        assert!(!l.is_zero(), "分账本视角：非零（两腿都在）");
        assert_eq!(l.net(), 0, "净额视角：退化为 0（方向暴露已抵消）");
    }

    /// ★C25 双坐标独立：多头/空头坐标互不影响（构造 (a,b) 时 q⁺只由a定、q⁻只由b定）。
    #[test]
    fn leg_coords_independent() {
        let l = Leg { q_plus: 3, q_minus: 8 };
        assert_eq!(l.q_plus, 3);
        assert_eq!(l.q_minus, 8);
        // 纯多头/纯空头/对冲三态坐标正交
        assert_eq!(Leg::long(4), Leg { q_plus: 4, q_minus: 0 });
        assert_eq!(Leg::short(4), Leg { q_plus: 0, q_minus: 4 });
        assert_eq!(Leg::hedged(4), Leg { q_plus: 4, q_minus: 4 });
    }

    /// ★C26 净额只对单向持仓忠实（契约锚 `net_long_faithful`）：纯多头腿 (q,0) 净额=q。
    #[test]
    fn net_long_faithful() {
        assert_eq!(net(&vec![Leg::long(5)]), 5);
        assert_eq!(net(&vec![Leg::short(5)]), -5, "纯空头腿净额=-q");
    }

    /// ★C26 净额映射有损（契约锚 `net_not_injective`，反退化防火墙）：双开 (1,1) vs 空仓 (0,0)
    /// 在 P^sep 中不同，净额都为 0——Net 非单射。坐实"净额映射有损"，下游不可用净额消解分账本。
    #[test]
    fn net_not_injective_witness() {
        let p1: SepPosition = vec![Leg::hedged(1)]; // (1,1)
        let p2: SepPosition = vec![Leg::zero()]; // (0,0)
        assert_ne!(p1, p2, "两个不同的 P^sep 头寸");
        assert_eq!(net(&p1), net(&p2), "净额相同（都为 0）⟹ Net 非单射");
        assert_eq!(net(&p1), 0);
    }

    /// ★净额可加（契约锚 `net_append`）：两段声部族拼接净额 = 各段净额之和。
    #[test]
    fn net_append() {
        let p1: SepPosition = vec![Leg::long(3), Leg::short(1)]; // net = 3-1 = 2
        let p2: SepPosition = vec![Leg::long(5)]; // net = 5
        let mut joined = p1.clone();
        joined.extend_from_slice(&p2);
        assert_eq!(net(&joined), net(&p1) + net(&p2));
        assert_eq!(net(&joined), 7);
    }

    /// ★空仓净额为零（契约锚 `net_sepZero`）：P^sep 原点净额 0。
    #[test]
    fn sep_zero_net() {
        assert_eq!(net(&sep_zero(4)), 0);
        assert_eq!(sep_zero(3).len(), 3);
        assert!(sep_zero(3).iter().all(Leg::is_zero));
    }

    /// ★C32/C39 不删父（契约锚 `parent_leg_survives_child_open`）：父多头腿 (Q,0) 开子空头后变
    /// (Q,Q)——父多头分量 q⁺ 不减、新增空头分量 q⁻=Q。子开不删父（双开非零，净额退化）。
    #[test]
    fn parent_leg_survives_child_open() {
        let parent = Leg::long(10); // (10, 0)
        let after = open_child_leg(&parent, 10); // 开同股数子空头
        assert_eq!(after.q_plus, 10, "父多头分量 q⁺ 不减");
        assert_eq!(after.q_minus, 10, "子空头写在独立 q⁻ 坐标");
        assert_eq!(after, Leg::hedged(10));
        assert!(!after.is_zero(), "C39 不删父：子开后腿非零（父仓存活）");
        assert_eq!(after.net(), 0, "净额视角退化（C26），但分账本视角父仓在");
    }

    /// ★不删父：非同股数子空头（一般 s_e=κ_e·s_par，0<κ≤1）父仓仍不减。
    #[test]
    fn open_child_leg_partial() {
        let parent = Leg::long(10);
        let after = open_child_leg(&parent, 6); // κ=0.6 的子空头
        assert_eq!(after.q_plus, 10, "父多头不减");
        assert_eq!(after.q_minus, 6);
        assert_eq!(after.net(), 4, "净额 10-6=4（部分对冲，未完全抵消）");
    }

    // ────────────────────────────────────────────────────────────────────
    // C29 Eat^sep 判定测试（向上元素多头腿覆盖、向下空头腿覆盖、整区间、不抵消）
    // ────────────────────────────────────────────────────────────────────

    /// ★C29 向上元素被多头腿吃到：ε_e=+1（Long）整区间 [0,3) 规范腿 ν=0 持 q⁺=5 ∧ q⁻=0 ⟹ Eat^sep。
    #[test]
    fn eat_sep_long_element() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        // 整区间规范腿 ν=0 = 多头腿 (5,0)。
        let pos: SepPosition = vec![Leg::long(5)];
        assert!(eat_sep(&e, |_t| &pos), "向上元素由多头腿 (5,0) 整区间覆盖 ⟹ Eat^sep");
    }

    /// ★C29 向下元素被空头腿吃到：ε_e=-1（Short）整区间规范腿持 q⁺=0 ∧ q⁻=s_e ⟹ Eat^sep。
    #[test]
    fn eat_sep_short_element() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Short, target: 5, nu: 0 };
        let pos: SepPosition = vec![Leg::short(5)];
        assert!(eat_sep(&e, |_t| &pos), "向下元素由空头腿 (0,5) 整区间覆盖 ⟹ Eat^sep");
    }

    /// ★C29 反向腿不吃到：向上元素 (ε=+1) 但规范腿是空头腿 (0,5) ⟹ ¬Eat^sep（方向必要性）。
    #[test]
    fn eat_sep_wrong_direction() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let pos: SepPosition = vec![Leg::short(5)]; // 空头腿，与向上元素反向
        assert!(!eat_sep(&e, |_t| &pos), "向上元素由空头腿覆盖 ⟹ 不吃到（q⁺=0≠5）");
    }

    /// ★C29 双开腿不吃到向上元素：双开 (5,5) 的 q⁻=5≠0 ⟹ ¬Eat^sep（向上要求 q⁻=0，不抵消语义）。
    /// ★这坐实 Eat^sep 的"多空不净额抵消"：双开在净额上 =5-5=0 但 Eat^sep 严格要求反向坐标为零，
    /// 故双开**不**满足单方向 Eat^sep（双开是父子两元素各自的腿，非单元素的吃到）。
    #[test]
    fn eat_sep_hedged_not_eaten_as_long() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let pos: SepPosition = vec![Leg::hedged(5)]; // (5,5)
        assert!(!eat_sep(&e, |_t| &pos), "双开 (5,5) 的 q⁻=5≠0 ⟹ 不满足向上 Eat^sep（要求 q⁻=0）");
    }

    /// ★C29 单位数不符不吃到：规范腿持 q⁺=3≠target=5 ⟹ ¬Eat^sep（C07 同单位数吃到的必要性）。
    #[test]
    fn eat_sep_wrong_units() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let pos: SepPosition = vec![Leg::long(3)]; // 单位数 3 ≠ 目标 5
        assert!(!eat_sep(&e, |_t| &pos), "q⁺=3≠s_e=5 ⟹ 不吃到（同单位数吃到要求 q⁺=s_e）");
    }

    /// ★C29 区间内中断不吃到：某时刻规范腿空仓 ⟹ ¬Eat^sep（必须整区间覆盖）。
    #[test]
    fn eat_sep_interval_break() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let covered: SepPosition = vec![Leg::long(5)];
        let empty: SepPosition = vec![Leg::zero()];
        // t=1 时空仓，整区间覆盖被打断。
        let pos_at = |t: u64| if t == 1 { &empty } else { &covered };
        assert!(!eat_sep(&e, pos_at), "t=1 空仓打断整区间覆盖 ⟹ 不吃到");
    }

    /// ★C29 规范腿索引越界不吃到：ν(e)=2 但头寸只有 1 个声部 ⟹ ¬Eat^sep（规范腿不存在）。
    #[test]
    fn eat_sep_nu_out_of_bounds() {
        let e = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 2 };
        let pos: SepPosition = vec![Leg::long(5)]; // 只有声部 0
        assert!(!eat_sep(&e, |_t| &pos), "ν=2 越界（头寸只有声部 0）⟹ 规范腿不存在 ⟹ 不吃到");
    }

    /// ★C29 不良构元素不吃到：空区间（lo=hi）⟹ ¬Eat^sep（元素须良构）。
    #[test]
    fn eat_sep_ill_formed() {
        let empty_interval = SyntaxElement { lo: 3, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let pos: SepPosition = vec![Leg::long(5)];
        assert!(!eat_sep(&empty_interval, |_t| &pos), "空区间 lo=hi ⟹ 不良构 ⟹ 不吃到");
        let zero_target = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 0, nu: 0 };
        assert!(!eat_sep(&zero_target, |_t| &pos), "target=0 ⟹ 不良构 ⟹ 不吃到");
    }

    /// ★C28 ν 单射检查：不同元素规范腿索引两两不同 ⟹ 单射。
    #[test]
    fn nu_injective_holds() {
        let elements = vec![
            SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 },
            SyntaxElement { lo: 0, hi: 2, dir: Side::Short, target: 3, nu: 1 },
            SyntaxElement { lo: 1, hi: 4, dir: Side::Long, target: 2, nu: 2 },
        ];
        assert!(nu_injective_on(&elements), "ν=0,1,2 两两不同 ⟹ 单射（每元素一腿）");
    }

    /// ★C28 ν 非单射：两元素共享规范腿索引 ⟹ 非单射（违反"每元素有自己的规范腿"）。
    #[test]
    fn nu_not_injective() {
        let elements = vec![
            SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 },
            SyntaxElement { lo: 1, hi: 4, dir: Side::Short, target: 3, nu: 0 }, // 共享 ν=0
        ];
        assert!(!nu_injective_on(&elements), "两元素共享 ν=0 ⟹ 非单射");
    }

    /// ★C29 父子双元素各被自己的腿吃到（不删父 + 分账本覆盖联合）：父向上元素 ν=0 多头腿覆盖，
    /// 子向下元素 ν=1 空头腿覆盖——同一头寸 [parent_long, child_short] 同时满足两元素的 Eat^sep。
    /// 这是分账本"每元素一腿、多空不抵消"的联合见证（对照净额账本会把父子抵消）。
    #[test]
    fn eat_sep_parent_child_both_eaten() {
        let parent = SyntaxElement { lo: 0, hi: 3, dir: Side::Long, target: 5, nu: 0 };
        let child = SyntaxElement { lo: 0, hi: 3, dir: Side::Short, target: 5, nu: 1 };
        // 声部 0 = 父多头腿 (5,0)，声部 1 = 子空头腿 (0,5)。两腿在独立坐标，不抵消。
        let pos: SepPosition = vec![Leg::long(5), Leg::short(5)];
        assert!(eat_sep(&parent, |_t| &pos), "父向上元素被声部 0 多头腿吃到");
        assert!(eat_sep(&child, |_t| &pos), "子向下元素被声部 1 空头腿吃到");
        assert!(nu_injective_on(&[parent, child]), "ν 单射（父 ν=0、子 ν=1）");
        // 净额视角：父子在不同声部，net = 5 + (-5) = 0（C26 退化），但分账本两元素都被吃到。
        assert_eq!(net(&pos), 0, "净额视角退化为 0，但分账本视角父子各被自己的腿吃到");
    }
}
