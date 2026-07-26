//! ★LEE M2 订单归因改造 `LevelOrderLedger`（multi-level-native-execution-design-20260719
//! §C.2 伪码「order_raw = Σ_ℓ Δq_ℓ」/ §D 迁移表 M2 行）。
//!
//! ## 本文件做什么：物理订单的**量**改由 `Σ_ℓ Δq_ℓ` 承载；其**值**仍由账户层 ΔN 锚定
//!
//! M2 定义（设计文档 §D 迁移表逐字）：「物理订单改由 Σ_ℓ Δq_ℓ 生成（各级目标仍每 bar 重估），
//! 证明 Σ_ℓ Δq_ℓ == ΔN，订单流与 M0 逐 bar 相等」，验收「bit-exact：成交序列不变，仅归因维度
//! 增加」。
//!
//! **能力边界（照实声明，090 反声明膨胀）**：本模块**不**独立决定订单量。物理整数目标 `T` 由
//! 账户层聚合决策锚定（`p_t + Schedule_Θ 的有符号手数`），级别台账把 `T` 划分成 `q_ℓ` 并产
//! `Δq_ℓ`；订单量 `= Σ_ℓ Δq_ℓ` 因此**恒等于** ΔN——这是**构造性同义反复（L0）**，不是可证伪
//! 的实证结论。推导方向与 §C.2 伪码字面（`order_raw = Σ_ℓ Δq_ℓ` 在前、`K_Θ_gate` 在后）**相反**：
//! 伪码是 M4 的终态形态（级别独立 sizing 后各级目标才真正独立），M2 的两条约束
//! （①订单由级别增量之和承载 ②与 M0 bit-exact）在「M2 不改聚合决策」的前提下**只允许**
//! 事后划分——见下节。诚实的一句话总结：**M2 交付的是「ΔN 的级别归因划分台账 + 订单量改经
//! 该台账出口」，不是「级别独立地决定了下多少单」**。
//!
//! 三个量，逐决策点：
//!
//! | 量 | 定义 | 不变量 |
//! |---|---|---|
//! | `held_ℓ` | 已**成交**的按级归因持仓 | `Σ_ℓ held_ℓ ≡ p_t`（净额账本 units，整数手） |
//! | `q_ℓ` | 本决策点各级**物理目标** | `Σ_ℓ q_ℓ ≡ T`（T = lot 可实现物理净目标） |
//! | `Δq_ℓ` | `q_ℓ − held_ℓ` | `Σ_ℓ Δq_ℓ ≡ T − p_t ≡ ΔN`（订单量，i64 精确） |
//!
//! 订单 `qty = |Σ_ℓ Δq_ℓ|`。**动作分类**（Buy/Sell/Add/Reduce/Close/Hold/Wait）仍取净额语义的
//! `Schedule_Θ`（设计文档 §C.2 伪码末行逐字「沿用 Schedule_Θ 单出口」）——M2 改的是**量的来源**，
//! 不是账户层动作语义；级别独立时钟（M3）与级别 sizing（M4）明确不在本步。
//!
//! ## 为什么 M2 的 Δq_ℓ 只能是 ΔN 的精确划分（结构性事实，非实装取巧）
//!
//! M2 同时受两条约束：①「订单由 Σ_ℓ Δq_ℓ 生成」；②「订单流与 M0 逐 bar bit-exact」。M0 的
//! 聚合决策（p̃ → `LexArgmin_{p∈𝒦_Θ}J_x(p)` → p\*）在 M2 **不改**（改它属 M4 级别 sizing），
//! 故 Σ_ℓ Δq_ℓ 只能恰好等于 M0 的 ΔN——即 `{Δq_ℓ}` 必须是 ΔN 的一个**精确划分**。M2 因此是
//! 「归因维度增加」而非「决策改变」，这正是迁移表把订单流分叉推迟到 M3 的原因（设计文档 §D M3
//! 行「本步起订单流与 M0 分叉，必须独立评审，不得借 M2 的 bit-exact 蒙混」）。
//!
//! ## 三类读数的证据等级（formalization-validity-domain 强制标注；勿混为一谈）
//!
//! | 读数 | 等级 | 说明 |
//! |---|---|---|
//! | `max_abs_order_residual`（`\|Σ_ℓ Δq_ℓ\| − qty_M0`） | **L0 同义反复** | 由 `T := p_t + qty_M0·sign` 的构造直接推出，**不可证伪**；保留仅作实装回归护栏（构造若被改坏立即非 0）。**不得**作为「M2 恒等成立」的实证证据引用。 |
//! | `max_abs_held_residual`（`Σ_ℓ held_ℓ − p_t`） | **L1 可证伪** | 跨**延迟成交/部分成交/拒单/丢单**的实际成交归因不变量。`on_fill` 消费的是 `units_after − units_before`（真实成交），与计划量无关 ⟹ 归因比例回缩若写错，此残差立即非 0。这是本模块唯一真正被数据检验的恒等。 |
//! | `n_rescaled` / `max_abs_struct_gap`（`Σ_ℓ net_ℓ` vs `T`） | **L2 经验读数** | 结构级别净额与账户层物理目标的**分歧幅度**，纯观测无断言。这是 M2 真正产生信息增量的地方：分歧不为零意味着「级别结构说的仓位」与「账户层能下的仓位」不一致，M3/M4 必须正面处理它（M2 按比例吸收，**照实登记为待裁决口径**，不冒充已解决）。 |
//!
//! ## 归因算子（确定性 + 整数精确）
//!
//! [`attribute_total`]：给定结构基准 `basis_ℓ`（= 该决策点各级结构净额 `net_ℓ`，
//! [`super::level_ledger::level_nets`] 单源）与物理总量 `T`，产 `q_ℓ` 且 `Σ_ℓ q_ℓ ≡ T`：
//!
//! - `Σ basis_ℓ == T`（**常态路径**：结构净额恰好可实现）⟹ 恒等映射，零缩放零重排；
//! - `Σ basis_ℓ ≠ 0` ⟹ 最大余数法按 `basis_ℓ` 比例缩放（i128 整数商 + 余数排序补位，
//!   无浮点 ⟹ **无结合律重排误差**）。缩放的语义依据：`𝒦_Θ` 是**账户层**可行性约束
//!   （设计文档 §C.1「净额降级为账户层约束」）⟹ 帽约束按结构比例回缩是级别中性的；
//! - `Σ basis_ℓ == 0 且 T ≠ 0` ⟹ 全部落 [`LEVEL_ACCOUNT_RESIDUAL`] 残差桶，**显式声明
//!   「无结构级别可归因」**，不伪造级别身份（roadmap:64「不得伪造中间级别证书」同纪律）。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L0/L1**：`Σ_ℓ Δq_ℓ ≡ ΔN` 是构造性整数恒等（划分的加法），零信息增量；订单流 bit-exact 是
//! 管线正确性证据，**不声明 alpha**（同 `overlay_state.rs` / `level_ledger.rs` 的认识论声明）。

use crate::theta_v0::types::{Order, StrictAction};

/// 无结构级别可归因时的账户层残差桶（`Σ_ℓ basis_ℓ == 0` 而物理目标 ≠ 0）。
///
/// 语义：该手数**没有**任何级别的结构净额作依据（如 pan_div 子腿投影经账户层 clamp 后的
/// 净目标、或全部腿不足最小手数被剔除后残留的网格化目标）。落显式残差桶而非摊给某个真实
/// 级别——伪造级别身份是 M2 明确禁止的（设计文档 §C.3「跨级授权走谱系显式边」同纪律）。
pub const LEVEL_ACCOUNT_RESIDUAL: u32 = u32::MAX;

/// 按级归因表（level 升序、level 无重复；级别数常态 ≤ 6 ⟹ 用 `Vec` 而非 `BTreeMap`，
/// 保确定序的同时避免逐 bar 树分配）。
pub type LevelUnits = Vec<(u32, i64)>;

/// ★M2 逐决策点归因见证读数（**release 可见**，非 `debug_assert`）。
///
/// #289 影子评审 MED 同款纪律：恒等证据必须在 release 下非平凡可读——只有 `debug_assert`
/// 的恒等在 release 跑批里零执行，等于没有证据。「残差恒 0」必须配「量级 > 0」才成对
/// （残差 0 而量级也 0 = 空转）。
///
/// **等级分层**（见模块头「三类读数的证据等级」表，勿混用）：`max_abs_order_residual` 是
/// **L0 同义反复**（构造性，不可证伪，仅作实装护栏）；`max_abs_held_residual` 是 **L1 可证伪**
/// （跨延迟/部分/拒单的实际成交归因）；`n_rescaled`/`max_abs_struct_gap` 是 **L2 经验读数**
/// （结构目标与账户物理目标的分歧，纯观测无断言）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelOrderStats {
    /// 决策点数（分母；非平凡性前置）。
    pub n_decisions: u64,
    /// 由 `Σ_ℓ Δq_ℓ` 生成且 `qty>0` 的订单数。
    pub n_orders_generated: u64,
    /// `max |Σ_ℓ Δq_ℓ|`（**非平凡性证据**：>0 才说明订单量真由级别增量之和承载）。
    pub max_abs_order_units: i64,
    /// `max | |Σ_ℓ Δq_ℓ| − qty_M0 |`（**L0 同义反复**的实装护栏，恒 0；不可作恒等的实证证据）。
    pub max_abs_order_residual: i64,
    /// `max |Σ_ℓ held_ℓ − p_t|`（**L1 可证伪**：跨延迟/部分/拒单的实际成交归因完备，应恒 0）。
    pub max_abs_held_residual: i64,
    /// `max |p_t|`（**非平凡性证据**：>0 才说明持仓归因不是空账）。
    pub max_abs_net_units: i64,
    /// 落 [`LEVEL_ACCOUNT_RESIDUAL`] 桶的决策点数（诚实缺口计数）。
    pub n_residual_bucket: u64,
    /// 需比例缩放（`Σ_ℓ net_ℓ ≠ T`）的决策点数（**L2 经验读数**，无断言——实测在默认配置的
    /// 随机游走 fixture 上约占决策点四成，因 p̃ 的**聚合** lot 量化与逐腿 `q_units` 取整口径
    /// 不同；这是真实分歧，不是缺陷，M2 按比例吸收并在此登记，M3/M4 须正面裁决）。
    pub n_rescaled: u64,
    /// `max |Σ_ℓ net_ℓ − T|`（**L2 经验读数**）：级别结构净额与账户层物理目标的**分歧幅度**。
    /// 纯观测，**不断言其为 0**——它不为 0 正是「结构说的仓位 ≠ 账户能下的仓位」的量化，
    /// 是 M2 唯一产生信息增量的读数（其余两条残差分别是 L0 同义反复与 L1 管线正确性）。
    pub max_abs_struct_gap: i64,
}

impl LevelOrderStats {
    /// 恒等见证成立：两条残差恒 0 **且** 两条量级非平凡（>0）。
    ///
    /// 单看「残差 0」不构成证据——空账下残差平凡为 0（#289 MED 指出的正是这一类平凡通过）。
    /// **有效域**：本谓词见证的是 `max_abs_held_residual`（L1 可证伪）+ 非平凡量级；其中的
    /// `max_abs_order_residual` 项是 L0 构造护栏，对本谓词**不贡献信息增量**（见字段注释）。
    pub fn identity_witnessed(&self) -> bool {
        self.max_abs_order_residual == 0
            && self.max_abs_held_residual == 0
            && self.max_abs_order_units > 0
            && self.max_abs_net_units > 0
    }
}

/// 单决策点的级别归因计划（`Σ_ℓ Δq_ℓ` 的生成物）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelOrderPlan {
    /// 各级物理目标 `q_ℓ`（`Σ ≡ target_total`）。
    pub targets: LevelUnits,
    /// 各级增量 `Δq_ℓ = q_ℓ − held_ℓ`（`Σ ≡ order_units`；含 `Δq_ℓ=0` 的级别，级别封闭可读）。
    pub deltas: LevelUnits,
    /// `Σ_ℓ Δq_ℓ`（**订单量的唯一来源**；i64 精确求和，无浮点重排）。
    pub order_units: i64,
    /// 本计划是否动用了 [`LEVEL_ACCOUNT_RESIDUAL`] 桶。
    pub used_residual_bucket: bool,
    /// 本计划是否经比例缩放（`Σ_ℓ basis_ℓ ≠ 物理目标`）。
    pub rescaled: bool,
    /// `Σ_ℓ net_ℓ − T`（结构净额与账户层物理目标的**有符号分歧**；L2 经验读数，见
    /// [`LevelOrderStats::max_abs_struct_gap`]）。
    pub struct_gap: i64,
}

impl LevelOrderPlan {
    /// 由 `Σ_ℓ Δq_ℓ` 生成物理订单：`qty = |Σ_ℓ Δq_ℓ|`，动作分类沿用净额 `Schedule_Θ`
    /// （设计文档 §C.2「沿用 Schedule_Θ 单出口」）。
    ///
    /// `Σ_ℓ Δq_ℓ == 0` ⟹ 动作降级为 `Hold`/`Wait`（无量可下；与 [`super::coverage::schedule_order`]
    /// 的 `qty==0` 分支同语义——持仓非空 `Hold`、空仓 `Wait`）。
    pub fn into_order(&self, action: StrictAction, holding: bool, exec_index: usize) -> Order {
        let qty = self.order_units.abs();
        if qty == 0 {
            let action = if holding { StrictAction::Hold } else { StrictAction::Wait };
            return Order { action, qty: 0, exec_index };
        }
        Order { action, qty, exec_index }
    }
}

/// ★M2 级别归因台账：持有已成交的按级归因持仓 `held_ℓ`（`Σ_ℓ held_ℓ ≡ p_t`），逐决策点产
/// [`LevelOrderPlan`]，逐成交把实际成交量按计划比例落账。
#[derive(Debug, Clone, Default)]
pub struct LevelOrderLedger {
    /// 已成交的按级归因持仓（level 升序；`Σ ≡ 净额账本 units`）。
    held: LevelUnits,
    stats: LevelOrderStats,
}

impl LevelOrderLedger {
    pub fn new() -> Self {
        LevelOrderLedger::default()
    }

    /// 各级已成交归因持仓（只读；level 升序）。
    pub fn held(&self) -> &[(u32, i64)] {
        &self.held
    }

    /// `Σ_ℓ held_ℓ`（应恒 = 净额账本 `units`）。
    pub fn held_total(&self) -> i64 {
        self.held.iter().map(|&(_, q)| q).sum()
    }

    /// 见证读数（release 可见）。
    pub fn stats(&self) -> LevelOrderStats {
        self.stats
    }

    /// ★逐决策点计划：结构基准 `basis`（各级结构净额 `net_ℓ`）+ 物理净目标 `target_total`
    /// （lot 可实现，整数手）→ 各级物理目标 `q_ℓ` + 增量 `Δq_ℓ`，`Σ_ℓ Δq_ℓ = target_total − Σ held_ℓ`。
    pub fn plan(&self, basis: &[(u32, i64)], target_total: i64) -> LevelOrderPlan {
        let (targets, used_residual_bucket, rescaled) = attribute_total(basis, target_total);
        let deltas = sub_levels(&targets, &self.held);
        let order_units = deltas.iter().map(|&(_, q)| q).sum();
        let struct_gap = basis.iter().map(|&(_, q)| q).sum::<i64>() - target_total;
        LevelOrderPlan { targets, deltas, order_units, used_residual_bucket, rescaled, struct_gap }
    }

    /// 决策点见证登记（`p_t` = 净额账本当前持仓，整数手；`qty_m0` = 净额 `Schedule_Θ` 的量）。
    pub fn observe_decision(&mut self, plan: &LevelOrderPlan, p_t: i64, qty_m0: i64) {
        let held_total = self.held_total();
        let s = &mut self.stats;
        s.n_decisions += 1;
        if plan.order_units != 0 {
            s.n_orders_generated += 1;
        }
        s.max_abs_order_units = s.max_abs_order_units.max(plan.order_units.abs());
        s.max_abs_order_residual =
            s.max_abs_order_residual.max((plan.order_units.abs() - qty_m0).abs());
        s.max_abs_held_residual = s.max_abs_held_residual.max((held_total - p_t).abs());
        s.max_abs_net_units = s.max_abs_net_units.max(p_t.abs());
        if plan.used_residual_bucket {
            s.n_residual_bucket += 1;
        }
        if plan.rescaled {
            s.n_rescaled += 1;
        }
        s.max_abs_struct_gap = s.max_abs_struct_gap.max(plan.struct_gap.abs());
    }

    /// ★逐成交落账：实际成交有符号手数 `executed_signed`（= `units_after − units_before`）按
    /// 该订单的计划增量 `attrib` 比例分配到 `held_ℓ`，`Σ_ℓ Δheld_ℓ ≡ executed_signed` 精确。
    ///
    /// 拒单/部分成交（[`super::super::backtest::fill::apply_fill`] 的现金约束与
    /// `close_only` 上限）⟹ `|executed_signed| < |Σ attrib|` ⟹ 同一 [`attribute_total`]
    /// 算子按计划比例回缩，归因不失配（`Σ_ℓ held_ℓ ≡ units` 因此逐 fill 保持）。
    pub fn on_fill(&mut self, attrib: &[(u32, i64)], executed_signed: i64) {
        if executed_signed == 0 {
            return;
        }
        let (applied, _, _) = attribute_total(attrib, executed_signed);
        self.held = add_levels(&self.held, &applied);
        self.held.retain(|&(_, q)| q != 0);
    }
}

/// ★归因算子：把物理总量 `total` 按结构基准 `basis` 确定性分配到各级，`Σ_ℓ out_ℓ ≡ total`
/// **整数精确**（无浮点 ⟹ 无结合律重排误差）。
///
/// 返回 `(各级量, 是否动用残差桶, 是否经比例缩放)`。三分支见模块头「归因算子」段。
///
/// 最大余数法细则：`num_ℓ = basis_ℓ · total`（i128 防溢出），`q_ℓ = num_ℓ / B`（Rust 整除向零
/// 截断），余数 `r_ℓ = num_ℓ % B`；亏空 `d = total − Σ q_ℓ`（`|d| < 级别数`）按 `|r_ℓ|` 降序、
/// 同余数按 level 升序逐个补 `sign(d)`——**全序确定**（level 唯一 ⟹ 无平局歧义）。
pub fn attribute_total(basis: &[(u32, i64)], total: i64) -> (LevelUnits, bool, bool) {
    let b_sum: i64 = basis.iter().map(|&(_, q)| q).sum();
    if b_sum == total {
        // 常态路径：结构净额恰好 = 物理目标 ⟹ 恒等映射（零缩放、零重排）。
        return (basis.to_vec(), false, false);
    }
    if b_sum == 0 {
        // 无结构基准可缩放：全部落显式账户层残差桶（不伪造级别身份）。
        let mut out: LevelUnits = basis.iter().map(|&(l, _)| (l, 0)).collect();
        upsert(&mut out, LEVEL_ACCOUNT_RESIDUAL, total);
        return (out, true, true);
    }
    // 最大余数法（i128 整数，确定序）。
    let total_i = total as i128;
    let b_i = b_sum as i128;
    let mut out: LevelUnits = Vec::with_capacity(basis.len());
    let mut rems: Vec<(i128, u32, usize)> = Vec::with_capacity(basis.len());
    let mut assigned: i128 = 0;
    for (idx, &(lvl, b)) in basis.iter().enumerate() {
        let num = b as i128 * total_i;
        let q = num / b_i;
        rems.push(((num % b_i).abs(), lvl, idx));
        assigned += q;
        out.push((lvl, q as i64));
    }
    let deficit = total_i - assigned;
    // 亏空上界：每级截断至多丢 1 手 ⟹ |d| ≤ 级别数（`rems.len()`）⟹ 一趟补位必然补完，
    // 不需要回绕（回绕分支在此界下永不可达 ⟹ 不写，speculative generality）。
    debug_assert!(
        deficit.unsigned_abs() as usize <= rems.len(),
        "最大余数法亏空上界违例：|{deficit}| > 级别数 {}",
        rems.len()
    );
    // |r_ℓ| 降序、level 升序（level 唯一 ⟹ 全序）。
    rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let step: i64 = if deficit > 0 { 1 } else { -1 };
    for &(_, _, idx) in rems.iter().take(deficit.unsigned_abs() as usize) {
        out[idx].1 += step;
    }
    (out, false, true)
}

/// 逐级相减 `a − b`（两表按 level 升序；结果覆盖两表 level 之并，含 0 项）。
fn sub_levels(a: &[(u32, i64)], b: &[(u32, i64)]) -> LevelUnits {
    merge_levels(a, b, |x, y| x - y)
}

/// 逐级相加 `a + b`（两表按 level 升序；结果覆盖两表 level 之并）。
fn add_levels(a: &[(u32, i64)], b: &[(u32, i64)]) -> LevelUnits {
    merge_levels(a, b, |x, y| x + y)
}

/// 两张按 level 升序表的逐级归并（level 之并，缺席视为 0，结果仍按 level 升序）。
fn merge_levels(a: &[(u32, i64)], b: &[(u32, i64)], f: impl Fn(i64, i64) -> i64) -> LevelUnits {
    let mut out: LevelUnits = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() || j < b.len() {
        match (a.get(i), b.get(j)) {
            (Some(&(la, qa)), Some(&(lb, qb))) if la == lb => {
                out.push((la, f(qa, qb)));
                i += 1;
                j += 1;
            }
            (Some(&(la, qa)), Some(&(lb, _))) if la < lb => {
                out.push((la, f(qa, 0)));
                i += 1;
            }
            (Some(_), Some(&(lb, qb))) => {
                out.push((lb, f(0, qb)));
                j += 1;
            }
            (Some(&(la, qa)), None) => {
                out.push((la, f(qa, 0)));
                i += 1;
            }
            (None, Some(&(lb, qb))) => {
                out.push((lb, f(0, qb)));
                j += 1;
            }
            (None, None) => unreachable!("循环条件保证至少一侧非空"),
        }
    }
    out
}

/// 按 level 升序插入/累加一项（保持升序不变式）。
fn upsert(out: &mut LevelUnits, level: u32, q: i64) {
    match out.binary_search_by_key(&level, |&(l, _)| l) {
        Ok(idx) => out[idx].1 += q,
        Err(idx) => out.insert(idx, (level, q)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★归因算子核心不变量（M2 验收锚）：任意基准 × 任意总量 ⟹ `Σ_ℓ out_ℓ ≡ total` 精确。
    #[test]
    fn attribute_total_sums_exactly_for_all_shapes() {
        let cases: Vec<(Vec<(u32, i64)>, i64)> = vec![
            (vec![], 0),
            (vec![], 7),                                   // 空基准 + 非零目标 ⟹ 残差桶
            (vec![(1, 10), (2, 6), (3, -4)], 12),          // 恒等（Σ basis == total）
            (vec![(1, 10), (2, 6), (3, -4)], 6),           // 缩放（cap binding 同形）
            (vec![(1, 10), (2, 6), (3, -4)], 0),           // 缩放到 0
            (vec![(1, 10), (2, 6), (3, -4)], -5),          // 反号缩放
            (vec![(1, 7), (2, 7), (3, 7)], 10),            // 三等分 10 ⟹ 余数补位
            (vec![(1, 5), (2, -5)], 3),                    // Σ basis == 0 ⟹ 残差桶
            (vec![(0, 1)], i32::MAX as i64),               // 大数（i128 防溢出）
            (vec![(1, -3), (2, -9)], -4),                  // 全负基准
        ];
        for (basis, total) in cases {
            let (out, residual, _) = attribute_total(&basis, total);
            let sum: i64 = out.iter().map(|&(_, q)| q).sum();
            assert_eq!(sum, total, "Σ_ℓ out_ℓ ≡ total（basis={basis:?}, total={total}）");
            // level 升序 + 无重复（确定序，bit-exact 可复现前置）。
            for w in out.windows(2) {
                assert!(w[0].0 < w[1].0, "level 升序无重复：{out:?}");
            }
            // 残差桶只在无结构基准时动用。
            let has_res = out.iter().any(|&(l, q)| l == LEVEL_ACCOUNT_RESIDUAL && q != 0);
            assert_eq!(has_res, residual && total != 0, "残差桶标记与内容一致：{out:?}");
        }
    }

    /// ★恒等分支零重排：`Σ basis == total` ⟹ 逐字节返回 basis（常态路径无缩放无余数补位）。
    #[test]
    fn attribute_total_identity_branch_is_verbatim() {
        let basis = vec![(1u32, 10i64), (2, 6), (3, -4)];
        let (out, residual, rescaled) = attribute_total(&basis, 12);
        assert_eq!(out, basis, "恒等分支逐字节 = basis");
        assert!(!residual && !rescaled, "恒等分支不标缩放/残差");
    }

    /// ★缩放按结构比例（账户层帽回缩是级别中性的）：basis=[L1:10, L2:5]、total=9 ⟹ [6,3]。
    #[test]
    fn attribute_total_scales_proportionally() {
        let (out, _, rescaled) = attribute_total(&[(1, 10), (2, 5)], 9);
        assert!(rescaled);
        assert_eq!(out, vec![(1, 6), (2, 3)], "10:5 = 2:1 ⟹ 9 ↦ 6:3");
    }

    /// ★残差桶诚实声明：Σ basis == 0 而 total ≠ 0 ⟹ 不摊给真实级别，落 `LEVEL_ACCOUNT_RESIDUAL`。
    #[test]
    fn attribute_total_residual_bucket_does_not_fake_level_identity() {
        let (out, residual, _) = attribute_total(&[(1, 5), (2, -5)], 3);
        assert!(residual);
        assert_eq!(out, vec![(1, 0), (2, 0), (LEVEL_ACCOUNT_RESIDUAL, 3)]);
    }

    /// ★台账 Σ_ℓ Δq_ℓ ≡ T − Σ_ℓ held_ℓ（订单量恒等，M2 核心）：开→加→部分平→全平四步。
    #[test]
    fn ledger_order_units_equal_target_minus_held() {
        let mut led = LevelOrderLedger::new();
        // t0：空账 → 目标 12（L1:10 + L2:6 + L3:−4）⟹ Δ = 12。
        let basis0 = vec![(1u32, 10i64), (2, 6), (3, -4)];
        let p0 = led.plan(&basis0, 12);
        assert_eq!(p0.order_units, 12);
        assert_eq!(p0.targets, basis0);
        led.on_fill(&p0.deltas, p0.order_units);
        assert_eq!(led.held_total(), 12, "Σ held ≡ 已成交净额");
        // t1：目标 20（同结构比例放大）⟹ Δ = 8。
        let p1 = led.plan(&[(1, 16), (2, 8), (3, -4)], 20);
        assert_eq!(p1.order_units, 8);
        led.on_fill(&p1.deltas, p1.order_units);
        assert_eq!(led.held_total(), 20);
        // t2：目标 5（cap binding 缩放）⟹ Δ = −15。
        let p2 = led.plan(&[(1, 16), (2, 8), (3, -4)], 5);
        assert_eq!(p2.order_units, -15);
        led.on_fill(&p2.deltas, p2.order_units);
        assert_eq!(led.held_total(), 5);
        // t3：全平（空基准 + 目标 0）⟹ Δ = −5，账清空。
        let p3 = led.plan(&[], 0);
        assert_eq!(p3.order_units, -5);
        led.on_fill(&p3.deltas, p3.order_units);
        assert_eq!(led.held_total(), 0);
        assert!(led.held().iter().all(|&(_, q)| q == 0), "全平后各级归因清零：{:?}", led.held());
    }

    /// ★部分成交/拒单：实际成交 < 计划 ⟹ 归因按计划比例回缩，`Σ_ℓ held_ℓ ≡ 实际成交` 不失配。
    #[test]
    fn ledger_partial_fill_keeps_held_sum_equal_to_executed() {
        let mut led = LevelOrderLedger::new();
        let plan = led.plan(&[(1, 60), (2, 40)], 100);
        assert_eq!(plan.order_units, 100);
        led.on_fill(&plan.deltas, 37); // 现金约束拒掉 63 手
        assert_eq!(led.held_total(), 37, "Σ held ≡ 实际成交（非计划量）");
        assert_eq!(led.held(), &[(1, 22), (2, 15)], "60:40 比例回缩到 37（22+15）");
    }

    /// ★见证读数非平凡（#289 MED 同款纪律）：残差恒 0 **且** 量级 > 0 才算见证成立；
    /// 空账（零决策）下 `identity_witnessed` 必须为 false（平凡通过不算证据）。
    #[test]
    fn stats_witness_requires_nontrivial_magnitude() {
        let empty = LevelOrderLedger::new();
        assert!(!empty.stats().identity_witnessed(), "零决策 ⟹ 见证不成立（平凡）");
        let mut led = LevelOrderLedger::new();
        let plan = led.plan(&[(1, 10)], 10);
        led.observe_decision(&plan, 0, 10);
        led.on_fill(&plan.deltas, 10);
        let plan2 = led.plan(&[(1, 10)], 10);
        led.observe_decision(&plan2, 10, 0);
        let s = led.stats();
        assert_eq!(s.max_abs_order_residual, 0, "订单量残差恒 0");
        assert_eq!(s.max_abs_held_residual, 0, "归因完备残差恒 0");
        assert_eq!(s.max_abs_order_units, 10);
        assert_eq!(s.max_abs_net_units, 10);
        assert_eq!(s.n_decisions, 2);
        assert_eq!(s.n_orders_generated, 1, "第二步 Δ=0 ⟹ 不产订单");
        assert!(s.identity_witnessed(), "残差 0 + 量级 >0 ⟹ 见证成立");
    }

    /// ★订单构造：qty 唯一来源 = `|Σ_ℓ Δq_ℓ|`；Σ=0 ⟹ 动作降级 Hold/Wait（同净额 Schedule_Θ）。
    #[test]
    fn plan_into_order_qty_comes_from_level_delta_sum() {
        let led = LevelOrderLedger::new();
        let plan = led.plan(&[(1, 7), (2, 3)], 10);
        let o = plan.into_order(StrictAction::Buy, false, 5);
        assert_eq!(o.qty, 10, "qty = |Σ_ℓ Δq_ℓ|");
        assert_eq!(o.action, StrictAction::Buy);
        assert_eq!(o.exec_index, 5);
        let zero = led.plan(&[], 0);
        assert_eq!(zero.into_order(StrictAction::Buy, true, 5).action, StrictAction::Hold);
        assert_eq!(zero.into_order(StrictAction::Buy, false, 5).action, StrictAction::Wait);
        assert_eq!(zero.into_order(StrictAction::Buy, true, 5).qty, 0);
    }
}
