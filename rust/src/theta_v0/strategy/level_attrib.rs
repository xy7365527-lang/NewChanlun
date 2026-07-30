//! ★LEE 级别归因**算子层**（无状态、整数精确；从 `level_order.rs` 抽出，M3 拆分）。
//!
//! ## 为什么与台账分家
//!
//! `level_order.rs` 承载的是**有状态**的台账（`held_ℓ` 已成交归因 + `q_ℓ^plan` 结构计划 +
//! 见证读数），本文件承载的是它调用的**纯函数**：把一个物理总量按结构基准确定性地划分到
//! 各级别。两者的变更理由不同（台账随迁移里程碑 M1→M4 演进；算子只在「如何划分」的口径
//! 变化时才动）⟹ 按 `coding-style.md`「organize by feature」与 Divergent Change 分家。
//!
//! ## 算子契约（三分支，`Σ_ℓ out_ℓ ≡ total` 整数精确）
//!
//! [`attribute_total`]：给定结构基准 `basis_ℓ`（各级结构净额，
//! [`super::level_ledger::level_nets`] 单源）与物理总量 `total`：
//!
//! - `Σ basis_ℓ == total`（**常态路径**）⟹ 恒等映射，零缩放零重排；
//! - `Σ basis_ℓ ≠ 0` ⟹ 最大余数法按 `basis_ℓ` 比例缩放（i128 整数商 + 余数排序补位，
//!   **无浮点 ⟹ 无结合律重排误差**）。缩放的语义依据：`𝒦_Θ` 是**账户层**可行性约束
//!   （设计文档 §C.1「净额降级为账户层约束」）⟹ 帽约束按结构比例回缩是级别中性的；
//! - `Σ basis_ℓ == 0 且 total ≠ 0` ⟹ 全部落 [`LEVEL_ACCOUNT_RESIDUAL`] 残差桶，**显式声明
//!   「无结构级别可归因」**，不伪造级别身份（roadmap:64「不得伪造中间级别证书」同纪律）。
//!
//! ## 认识论等级
//!
//! **L0**：全部性质（`Σ ≡ total`、确定序、无重排）都是构造性整数恒等，不依赖任何数据 ⟹
//! 零信息增量。本文件不产生任何经验读数。

/// 无结构级别可归因时的账户层残差桶（`Σ_ℓ basis_ℓ == 0` 而物理目标 ≠ 0）。
///
/// 语义：该手数**没有**任何级别的结构净额作依据（如 pan_div 子腿投影经账户层 clamp 后的
/// 净目标、或全部腿不足最小手数被剔除后残留的网格化目标）。落显式残差桶而非摊给某个真实
/// 级别——伪造级别身份是明确禁止的（设计文档 §C.3「跨级授权走谱系显式边」同纪律）。
///
/// ★#309 影子评审 LOW-1（**照实登记，不在本票内改语义**）：本桶不产生任何
/// [`super::level_clock::LevelEventKind`] 事件——一旦经 `LevelOrderLedger::commit_planned`
/// 写入 `planned`，`regate` 因 `ticked.contains(&LEVEL_ACCOUNT_RESIDUAL)` 恒假而永远走
/// 保前值分支（不会因新的结构事件被重估/清零）；它仍可能被 `attribute_total` 的比例缩放
/// （帽/风控 binding 触发，见 `level_order.rs` `CapTick`/`RiskTick` 例外）间接改动，但没有
/// 任何**结构**路径能主动清零它。默认 `ThetaConfig`（pan_div 惰性）下 `Σ basis_ℓ==total==0`
/// 恒走恒等分支，本桶实测零动用（`n_residual_bucket==0`）⟹ 该缺口当前不可达，**这是设计
/// 决定还是漏门未经裁决**——若后续启用 pan_div/center_oscillation 等使本桶非零动用，需先
/// 决定「残差桶要不要有自己的 clock 事件类」，本文件不擅自新增该事件类。
pub const LEVEL_ACCOUNT_RESIDUAL: u32 = u32::MAX;

/// 按级归因表（level 升序、level 无重复；级别数常态 ≤ 6 ⟹ 用 `Vec` 而非 `BTreeMap`，
/// 保确定序的同时避免逐 bar 树分配）。
pub type LevelUnits = Vec<(u32, i64)>;

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
pub(crate) fn sub_levels(a: &[(u32, i64)], b: &[(u32, i64)]) -> LevelUnits {
    merge_levels(a, b, |x, y| x - y)
}

/// 逐级相加 `a + b`（两表按 level 升序；结果覆盖两表 level 之并）。
pub(crate) fn add_levels(a: &[(u32, i64)], b: &[(u32, i64)]) -> LevelUnits {
    merge_levels(a, b, |x, y| x + y)
}

/// 两张按 level 升序表的逐级归并（level 之并，缺席视为 0，结果仍按 level 升序）。
pub(crate) fn merge_levels(
    a: &[(u32, i64)],
    b: &[(u32, i64)],
    f: impl Fn(i64, i64) -> i64,
) -> LevelUnits {
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
pub(crate) fn upsert(out: &mut LevelUnits, level: u32, q: i64) {
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
            (vec![], 7),                          // 空基准 + 非零目标 ⟹ 残差桶
            (vec![(1, 10), (2, 6), (3, -4)], 12), // 恒等（Σ basis == total）
            (vec![(1, 10), (2, 6), (3, -4)], 6),  // 缩放（cap binding 同形）
            (vec![(1, 10), (2, 6), (3, -4)], 0),  // 缩放到 0
            (vec![(1, 10), (2, 6), (3, -4)], -5), // 反号缩放
            (vec![(1, 7), (2, 7), (3, 7)], 10),   // 三等分 10 ⟹ 余数补位
            (vec![(1, 5), (2, -5)], 3),           // Σ basis == 0 ⟹ 残差桶
            (vec![(0, 1)], i32::MAX as i64),      // 大数（i128 防溢出）
            (vec![(1, -3), (2, -9)], -4),         // 全负基准
        ];
        for (basis, total) in cases {
            let (out, residual, _) = attribute_total(&basis, total);
            let sum: i64 = out.iter().map(|&(_, q)| q).sum();
            assert_eq!(
                sum, total,
                "Σ_ℓ out_ℓ ≡ total（basis={basis:?}, total={total}）"
            );
            // level 升序 + 无重复（确定序，bit-exact 可复现前置）。
            for w in out.windows(2) {
                assert!(w[0].0 < w[1].0, "level 升序无重复：{out:?}");
            }
            // 残差桶只在无结构基准时动用。
            let has_res = out
                .iter()
                .any(|&(l, q)| l == LEVEL_ACCOUNT_RESIDUAL && q != 0);
            assert_eq!(
                has_res,
                residual && total != 0,
                "残差桶标记与内容一致：{out:?}"
            );
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
}
