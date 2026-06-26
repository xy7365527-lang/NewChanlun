//! 区间套有限递归证书 χ（reference-theta-v0.md:对齐 Nest.lean）。
//!
//! ## bit-exact 对齐 `Strict/Nest.lean`
//!
//! - `Sel_Θ` 选择器 ↔ `selOrder`：endTime 大优先 → startTime 大优先 → idx 小优先。
//! - 区间套关系 ↔ `Sub`：`J'.startTime>=J.startTime ∧ J'.endTime<=J.endTime`（J' 套在 J 内）。
//! - 选择器键 ↔ `selKey`：`(endTime, startTime, idx)`。
//! - 递归证书 ↔ `NestCertificate`：逐级 candidate + 区间套 ⊆。
//! - 终端确认 ↔ `Confirm`：`Λ≠∅`（至少一类买卖点成立，**不要求 |Λ|=1**——2/3 类可共存）。
//!
//! ## 核心定理（Nest.lean nest_certificate_unique）
//!
//! 在 `Sel_Θ` 固定下，区间套递归证书的定位见证（各级 `Sel_Θ` chosen 键序列）唯一。
//! Θ-参数化前件：唯一性依赖 `Sel_Θ`——缠论结构公理单独给不出唯一定位（需选择器固定）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：选择器是确定字典序，区间套是确定整数比较，确认是 bit-vector 非空判定。
//! 唯一性是纯结构归纳（同义反复，不冒充 L1+）。`Sel_Θ` 规则由 Θ_signal 给出（reference:24
//! canonical 分解 tie-break：最早确认时间 → 最低递归层 → 最早原始 index）。

use super::super::types::BspBits;

/// 候选定位区间（reference:对齐 `Nest.Interval`）——携带 `Sel_Θ` 排序三键。
///
/// 几何上下界由下游 `Sub` 契约（区间套缩小），本结构暴露排序所需三键，使「选择器固定
/// candidate」可机器检查。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestInterval {
    /// 结束时间（第一排序键，最新者优先 ⟹ 数值最大优先）。
    pub end_time: u64,
    /// 开始时间（第二排序键，最新者优先 ⟹ 数值最大优先）。
    pub start_time: u64,
    /// 编号（第三排序键，最小者优先 ⟹ 数值最小优先）。
    pub idx: u64,
}

impl NestInterval {
    /// `Sel_Θ` 字典序键（reference:对齐 `selKey`）：`(end_time, start_time, idx)`。
    pub fn sel_key(&self) -> (u64, u64, u64) {
        (self.end_time, self.start_time, self.idx)
    }
}

/// `Sel_Θ` 严格序（reference:对齐 `selOrder`）：a 严格优于 b。
///
/// 逐字对齐 Lean：endTime 大优先；endTime 相等则 startTime 大优先；前两相等则 idx 小优先。
pub fn sel_order(a: &NestInterval, b: &NestInterval) -> bool {
    a.end_time > b.end_time
        || (a.end_time == b.end_time && a.start_time > b.start_time)
        || (a.end_time == b.end_time && a.start_time == b.start_time && a.idx < b.idx)
}

/// 区间套关系 `Sub`（reference:对齐 `Sub`）：`J'` 套在 `J` 之内（逐级缩小定位）。
///
/// `inner.start_time>=outer.start_time ∧ inner.end_time<=outer.end_time`。
pub fn is_sub(inner: &NestInterval, outer: &NestInterval) -> bool {
    inner.start_time >= outer.start_time && inner.end_time <= outer.end_time
}

/// 从候选集合按 `Sel_Θ` 选出唯一最优候选（reference:对齐 `IsSelected`）。
///
/// 返回在 `selOrder` 下严格优于所有其他候选（或键相同）的那一个。`cands` 为空 ⟹ `None`
/// （无候选无定位）。bit-exact：用 `sel_order` 全序遍历选最优——每对比较确定。
///
/// 边界条件（Θ-参数化前件）：若 `cands` 含两个键不同但互不严格优的候选，`sel_order` 是
/// 严格全序（三键字典序），不会出现该情况——选择唯一（`selected_key_unique`）。
pub fn select_best(cands: &[NestInterval]) -> Option<NestInterval> {
    let mut best: Option<NestInterval> = None;
    for &cand in cands {
        match best {
            None => best = Some(cand),
            Some(b) => {
                // cand 严格优于当前 best ⟹ 替换（键相同则保留——sel_order 严格序无平局歧义）。
                if sel_order(&cand, &b) {
                    best = Some(cand);
                }
            }
        }
    }
    best
}

/// 区间套链一级节点（reference:对齐 `LevelNode`）。
#[derive(Debug, Clone, PartialEq)]
pub struct LevelNode {
    /// 该级别候选区间集合。
    pub cands: Vec<NestInterval>,
    /// `Sel_Θ` 从 `cands` 选出的候选。
    pub chosen: NestInterval,
}

/// 终端确认（reference:对齐 `Confirm`）：bit-vector 至少一类买卖点成立（`Λ≠∅`）。
///
/// **不要求 |Λ|=1**（Nest.lean `confirm_of_two_three`/`confirm_of_all_three`）——2/3 类
/// 共存同样确认。唯一被排除的是全零 bit-vector（无任何买卖点成立的终端）。
pub fn confirm(terminal: &BspBits) -> bool {
    terminal.buy1
        || terminal.buy2
        || terminal.buy3
        || terminal.sell1
        || terminal.sell2
        || terminal.sell3
}

/// 最终确认证书 χ（reference:对齐 `Chi`）——区间套递归证书 + 终端确认。
///
/// `chain` 操作级→执行级的级别链（ℓ₀ 表头，ℓ_k 表尾=执行级）；`terminal` 执行级 bit-vector。
#[derive(Debug, Clone, PartialEq)]
pub struct Chi {
    pub chain: Vec<LevelNode>,
    pub terminal: BspBits,
}

impl Chi {
    /// 区间套递归证书良构（reference:对齐 `NestCertificate`）：逐级相邻区间套 ⊆。
    ///
    /// 每对相邻级别 `chosen` 满足 `is_sub(下级, 上级)`（区间套逐级缩小）。空链/单级平凡成立。
    pub fn nest_valid(&self) -> bool {
        self.chain
            .windows(2)
            .all(|w| is_sub(&w[1].chosen, &w[0].chosen))
    }

    /// χ 整体确认（reference:对齐 `Chi`）：递归证书良构 ∧ 终端确认。
    pub fn is_confirmed(&self) -> bool {
        self.nest_valid() && confirm(&self.terminal)
    }

    /// 定位见证键序列（reference:对齐 `locatorKeys`）：各级 `chosen` 的 `selKey`。
    pub fn locator_keys(&self) -> Vec<(u64, u64, u64)> {
        self.chain.iter().map(|n| n.chosen.sel_key()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(et: u64, st: u64, idx: u64) -> NestInterval {
        NestInterval { end_time: et, start_time: st, idx }
    }

    #[test]
    fn sel_order_endtime_priority_bit_exact() {
        // endTime 大优先。
        let a = interval(10, 0, 0);
        let b = interval(5, 100, 0);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn sel_order_starttime_tiebreak_bit_exact() {
        // endTime 相等 → startTime 大优先。
        let a = interval(10, 8, 0);
        let b = interval(10, 5, 0);
        assert!(sel_order(&a, &b));
    }

    #[test]
    fn sel_order_idx_tiebreak_bit_exact() {
        // endTime/startTime 相等 → idx 小优先。
        let a = interval(10, 8, 1);
        let b = interval(10, 8, 5);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn sel_order_asymmetric() {
        // selOrder_asymm：a 优于 b ⟹ b 不优于 a。
        let a = interval(10, 8, 1);
        let b = interval(5, 3, 9);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn select_best_picks_lexicographic_winner() {
        let cands = vec![
            interval(5, 3, 0),
            interval(10, 8, 5),
            interval(10, 8, 1), // 同 end/start，idx 最小 ⟹ 胜
            interval(10, 5, 0),
        ];
        let best = select_best(&cands).expect("非空候选有最优");
        assert_eq!(best, interval(10, 8, 1));
    }

    #[test]
    fn select_best_empty_none() {
        assert_eq!(select_best(&[]), None);
    }

    #[test]
    fn selected_key_unique_property() {
        // selected_key_unique：同候选集任意选法键唯一（sel_order 严格全序）。
        let cands = vec![interval(10, 8, 1), interval(10, 8, 5), interval(5, 3, 0)];
        let b1 = select_best(&cands).unwrap();
        // 重排候选集选出的最优键相同。
        let reordered = vec![interval(5, 3, 0), interval(10, 8, 5), interval(10, 8, 1)];
        let b2 = select_best(&reordered).unwrap();
        assert_eq!(b1.sel_key(), b2.sel_key());
    }

    #[test]
    fn is_sub_nesting_bit_exact() {
        // inner [start>=, end<=] 套在 outer 内。
        let outer = interval(20, 0, 0);
        let inner = interval(15, 5, 0);
        assert!(is_sub(&inner, &outer)); // start 5>=0 ∧ end 15<=20
        assert!(!is_sub(&outer, &inner)); // 反向不套
    }

    #[test]
    fn confirm_nonempty_bit_exact() {
        // Λ≠∅ ⟹ confirm（confirm_of_two_three：2/3 共存也确认）。
        let mut t = BspBits::default();
        assert!(!confirm(&t)); // 全零不确认（not_confirm_of_empty）
        t.buy2 = true;
        t.buy3 = true;
        assert!(confirm(&t)); // 2/3 共存确认
    }

    #[test]
    fn confirm_empty_rejected() {
        // 全零 bit-vector 唯一被排除（not_confirm_of_empty）。
        assert!(!confirm(&BspBits::default()));
    }

    #[test]
    fn chi_nest_valid_and_confirmed() {
        // 区间套逐级缩小 + 终端确认 ⟹ χ confirmed。
        let chain = vec![
            LevelNode { cands: vec![interval(100, 0, 0)], chosen: interval(100, 0, 0) },
            LevelNode { cands: vec![interval(80, 10, 0)], chosen: interval(80, 10, 0) },
            LevelNode { cands: vec![interval(60, 20, 0)], chosen: interval(60, 20, 0) },
        ];
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        let chi = Chi { chain, terminal };
        assert!(chi.nest_valid()); // 100⊇80⊇60 逐级缩小
        assert!(chi.is_confirmed());
        assert_eq!(chi.locator_keys().len(), 3);
    }

    #[test]
    fn chi_broken_nesting_not_valid() {
        // 区间套不缩小（下级 end 超过上级）⟹ nest 不良构。
        let chain = vec![
            LevelNode { cands: vec![interval(60, 20, 0)], chosen: interval(60, 20, 0) },
            LevelNode { cands: vec![interval(100, 0, 0)], chosen: interval(100, 0, 0) }, // end 超界
        ];
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        let chi = Chi { chain, terminal };
        assert!(!chi.nest_valid());
        assert!(!chi.is_confirmed());
    }

    #[test]
    fn chi_singleton_chain_nest_trivial() {
        // 单级链（操作级=执行级）：无区间套缩小，nest 平凡良构（nest_certificate_unique_singleton）。
        let mut terminal = BspBits::default();
        terminal.buy3 = true;
        let chi = Chi {
            chain: vec![LevelNode { cands: vec![interval(50, 0, 0)], chosen: interval(50, 0, 0) }],
            terminal,
        };
        assert!(chi.nest_valid());
        assert!(chi.is_confirmed());
    }
}
