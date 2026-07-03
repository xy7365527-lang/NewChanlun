//! 区间套有限递归证书 χ（reference-theta-v0.md；契约重锚 `Origin.SubLevelDescent`）。
//!
//! ## 契约重锚（legacy Strict/Nest → `Origin.SubLevelDescent`）
//!
//! - 下钻递归 ↔ `Origin.SubLevelDescent.descend : RMove → List RMove`（compose 的逆，segment → []，
//!   `descend_level_decreases` 证下钻级别严格降 well-founded 终止）。
//! - 区间套关系 ↔ `Sub`：`J'.startTime>=J.startTime ∧ J'.endTime<=J.endTime`（J' 套在 J 内）。
//! - 子级别破中枢 ↔ `Origin.SubLevelDescent.{SubBrokeBelow,SubBrokeAbove,subLevelHasBrokenCenter}`。
//! - `Sel_Θ` 选择器 ↔ `selKey`：`(endTime, startTime, idx)` 字典序（canonical tie-break）。
//! - 终端确认 ↔ `Confirm`：`Λ≠∅`（至少一类买卖点成立，**不要求 |Λ|=1**——2/3 类可共存）。
//! - 方向化区间套证书 ↔ `N^δ_{ℓ↓e}`（spec P5 §6 line 1168）：χ 的**方向化 + 候选化精化**——基例
//!   用方向化 `Conf^δ_e`（[`BspBits::confirm_side`]），递归步追加候选谓词 `Cand^δ_ℓ`（[需人工确认]
//!   定义式，spec 疑点2）+ 子⊆父区间套（复用 [`is_sub`]）。见 [`NestCertificate`]。
//!
//! ## 核心定理（契约锚 `Origin.SubLevelDescent.descend_level_decreases`）
//!
//! 在 `Sel_Θ` 固定下，区间套递归证书的定位见证（各级 chosen 键序列）唯一；下钻级别严格降
//! （`descend_level_decreases`）⟹ 有限递归终止。唯一性依赖 `Sel_Θ`——缠论结构公理单独给不出
//! 唯一定位（需选择器固定）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：选择器是确定字典序，区间套是确定整数比较，确认是 bit-vector 非空判定。
//! 唯一性是纯结构归纳（同义反复，不冒充 L1+）。`Sel_Θ` 规则由 Θ_signal 给出（reference:24
//! canonical 分解 tie-break：最早确认时间 → 最低递归层 → 最早原始 index）。

use super::super::types::{BspBits, Side};

/// 候选定位区间（契约锚 `Origin.SubLevelDescent` 下钻区间）——携带 `Sel_Θ` 排序三键。
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
///
/// ★方向无关 = `Conf^+ ∨ Conf^-`（spec P5 §6）：`Λ≠∅` 恰为「买侧确认 [`BspBits::conf_plus`] 或
/// 卖侧确认 [`BspBits::conf_minus`]」的析取。委托两方向谓词（单一来源，消除重复析取）；区间套证书
/// `N^δ_{ℓ↓e}` 的**方向化**基例用 [`BspBits::confirm_side`]，本函数是其双向并集。
pub fn confirm(terminal: &BspBits) -> bool {
    terminal.conf_plus() || terminal.conf_minus()
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

/// 方向化区间套证书 `N^δ_{ℓ↓e}` 的递归梯级（spec P5 §6 递归步 ℓ>e 的单级数据）。
///
/// 对应 spec 递归步 `Cand^δ_ℓ(x) ∧ [J^δ_{ℓ-1}(x)⊆J^δ_ℓ(x)] ∧ N^δ_{ℓ-1↓e}` 中本级 ℓ 的两个分量：
/// - `interval` = 本级定位区间 `J^δ_ℓ`（与**子级** `J^δ_{ℓ-1}` 做闭口径 ⊆ 比较，复用 [`is_sub`]）。
/// - `cand` = 候选谓词 `Cand^δ_ℓ(x) ∈ {0,1}` 的**取值**（不是定义式）。
///
/// [需人工确认]（spec 疑点2，line 1257）：`Cand^δ_ℓ(x)` 在 PDF 中仅作符号出现，**无独立定义式**。
/// 本结构按 spec 把 `Cand^δ_ℓ(x)` 当作 0/1 谓词**取值**消费（与 `b_ℓ∈{0,1}^6` 同样是判定结果输入），
/// 其计算规则属上游（候选判据定义）——`N^δ` 只做合取组装，**不臆造** Cand 的判据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestRung {
    /// 本级 `J^δ_ℓ` 定位区间。
    pub interval: NestInterval,
    /// `Cand^δ_ℓ(x) ∈ {0,1}` 取值。[需人工确认] 定义式（spec 疑点2）。
    pub cand: bool,
}

/// 方向化区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}`（spec P5 §6 line 1168 / 结果包 line 308）。
///
/// `Chi`（χ）的**方向化 + 候选化精化**：χ 用方向无关 [`confirm`] 且无 `Cand`；本证书 (a) 基例改用
/// 方向化 [`BspBits::confirm_side`]（按 δ 选 `conf_plus`/`conf_minus`），(b) 每个递归级追加
/// `Cand^δ_ℓ` 合取。区间套关系仍复用契约锚 [`is_sub`]（不重造区间逻辑）。
#[derive(Debug, Clone, PartialEq)]
pub struct NestCertificate {
    /// 方向 δ（`Long`=+1 / `Short`=-1），固定于整条证书。
    pub side: Side,
    /// 执行级 e 终端 bit-vector（基例 `Conf^δ_e` 的输入）。
    pub terminal: BspBits,
    /// 执行级 e 定位区间 `J^δ_e`（最内层；作为最低递归级的子区间参与 ⊆）。
    pub base_interval: NestInterval,
    /// 递归级 `(e, ℓ]` 梯级，**从高到低**排列：`rungs[0]`=级 ℓ，`rungs[last]`=级 e+1。
    /// 空 ⟹ ℓ=e（纯基例）。结构上不可表达 e>ℓ（无负梯级），天然满足前置约束 e≤ℓ。
    pub rungs: Vec<NestRung>,
}

impl NestCertificate {
    /// 区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}`：按 ℓ 从高到低逐级校验。
    ///
    /// ## 结果包（六要素）
    /// - **结论**：返回 0/1（bool）值的级别递归谓词。基例 ℓ=e（`rungs` 空）取方向化确认
    ///   `Conf^δ_e(x)`=[`BspBits::confirm_side`]；递归步 ℓ>e 取 `Cand^δ_ℓ(x) ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ]
    ///   ∧ N^δ_{ℓ-1↓e}`，逐级下降至基例。
    /// - **定义依据**：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
    ///   P5 §6 分段方框（line 277-285）+ ∃! 方框（line 295）+ 结果包（line 308-312）。`side` 选 δ；
    ///   `terminal` 满足基例 `Conf^δ_e=⋁ B_{i,e}`/`⋁ S_{i,e}`；`rungs[k].cand` 提供 `Cand^δ`；
    ///   相邻 `interval` 满足 ⊆。
    /// - **边界条件**：(1) ⊆ 方向是**子⊆父** `J^δ_{ℓ-1}⊆J^δ_ℓ`（[`is_sub`]：child.lo≥parent.lo
    ///   ∧ child.hi≤parent.hi，闭口径；lo=`start_time`/hi=`end_time`），方向反则证书失效。
    ///   (2) 任一级 `Cand^δ_ℓ=false`、或任一相邻 ⊆ 不成立、或基例 `Conf^δ_e=false` ⟹ 整体翻转为 0。
    ///   (3) 方向 δ 翻转（Long↔Short）则基例改判 `conf_plus`↔`conf_minus`，结论可翻转。
    ///   (4) 前置 e≤ℓ 由结构保证（rungs 非负长度），e>ℓ 不可表达（spec：e>ℓ 递归未定义）。
    /// - **下游推论**：N^δ 是 Γ 候选集→确认信号提升（环2→环3）的方向化确认层；返回 bool ⟹ ∃!
    ///   ∈{0,1}（2 值确定函数，无需 Finset）；级别每步严格下降（rungs 缩短）⟹ 有限终止。
    ///   Lean 端对应 (ℓ-e):Nat 结构递归。
    /// - **谱系引用**：第三类边界谱系（MEMORY: theta-v0-type3-boundary）——基例只消费
    ///   [`BspBits::confirm_side`]（委托 `conf_plus`/`conf_minus` → buy3/sell3 bit），B3 边界已在
    ///   `bsp::endpoint_to_bsp`（rust `>=`）结算，本层**不重判边界**。`Cand^δ_ℓ` 定义式缺失见
    ///   spec 疑点2（[需人工确认]）。
    /// - **影响声明**：在 nest.rs 新增方向化区间套证书；不改 `Chi`/`confirm`/`is_sub`（契约锚保留），
    ///   不动 classifier/mod.rs。L0 操作语义结构（非 L2 alpha）。
    pub fn n_delta(&self) -> bool {
        Self::n_delta_rec(self.side, &self.terminal, &self.base_interval, &self.rungs)
    }

    /// `N^δ` 的级别递归核（mirror Lean (ℓ-e):Nat 结构递归）。`rungs` 从高(ℓ)到低(e+1)。
    fn n_delta_rec(side: Side, terminal: &BspBits, base: &NestInterval, rungs: &[NestRung]) -> bool {
        match rungs.split_first() {
            // 基例 ℓ=e：N^δ_{e↓e} = Conf^δ_e。
            None => terminal.confirm_side(side),
            // 递归步 ℓ>e：Cand^δ_ℓ ∧ [J^δ_{ℓ-1} ⊆ J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}。
            Some((top, rest)) => {
                // 子级 J^δ_{ℓ-1}：下一梯级区间；rest 空 ⟹ 子级是执行级 J^δ_e。
                let child = rest.first().map_or(base, |r| &r.interval);
                top.cand
                    && is_sub(child, &top.interval)
                    && Self::n_delta_rec(side, terminal, base, rest)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(et: u64, st: u64, idx: u64) -> NestInterval {
        NestInterval { end_time: et, start_time: st, idx }
    }

    fn rung(et: u64, st: u64, idx: u64, cand: bool) -> NestRung {
        NestRung { interval: interval(et, st, idx), cand }
    }

    fn buy1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.buy1 = true;
        t
    }

    fn sell1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.sell1 = true;
        t
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

    /// ★#100 问题① 验收（多个包含父候选）：J_child⊂J_a ∧ J_child⊂J_b——Sel_Θ（`select_best`/
    /// `sel_order`）从多个包含父中选**唯一且 bit-exact**（重排候选集选出同键，确定性选择器可复现）。
    #[test]
    fn sel_theta_unique_among_multiple_containing_parents() {
        let child = interval(50, 40, 0);
        // 两父都包含 child（is_sub(child, a) ∧ is_sub(child, b)），三键不同。
        let j_a = interval(80, 20, 3);
        let j_b = interval(90, 10, 7);
        assert!(is_sub(&child, &j_a));
        assert!(is_sub(&child, &j_b));
        // Sel_Θ 选唯一（end_time 大优先 ⟹ j_b 90>80）。
        let best = select_best(&[j_a, j_b]).unwrap();
        assert_eq!(best, j_b);
        // bit-exact 可复现：重排候选集选出同键。
        assert_eq!(select_best(&[j_b, j_a]).unwrap().sel_key(), best.sel_key());
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

    #[test]
    fn n_delta_base_case_directional_confirm() {
        // ℓ=e（rungs 空）：N^δ = Conf^δ_e（方向化基例）。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(50, 0, 0),
            rungs: vec![],
        };
        assert!(cert.n_delta()); // 买侧确认 + Long ⟹ 1
        // 方向翻转：买侧 bit 对 Short 不确认 ⟹ 0。
        let cert_short = NestCertificate { side: Side::Short, ..cert.clone() };
        assert!(!cert_short.n_delta());
    }

    #[test]
    fn n_delta_base_case_empty_terminal_rejected() {
        // 全零终端 ⟹ 基例 Conf^δ_e=0。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: BspBits::default(),
            base_interval: interval(50, 0, 0),
            rungs: vec![],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_two_level_full_certificate() {
        // ℓ=e+1：Cand^δ_ℓ ∧ [J^δ_e ⊆ J^δ_ℓ] ∧ Conf^δ_e。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(60, 20, 0), // J^δ_e 内层
            rungs: vec![rung(100, 0, 0, true)], // J^δ_ℓ 外层，Cand=1
        };
        assert!(is_sub(&interval(60, 20, 0), &interval(100, 0, 0))); // 子⊆父
        assert!(cert.n_delta());
    }

    #[test]
    fn n_delta_false_when_cand_false() {
        // 候选谓词 Cand^δ_ℓ=0 ⟹ 整体翻转（即便区间套 + 确认都成立）。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(60, 20, 0),
            rungs: vec![rung(100, 0, 0, false)],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_false_when_nesting_broken() {
        // 子⊄父（base end 100 超出 rung end 60）⟹ ⊆ 失败 ⟹ 0。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(100, 0, 0), // 子超界
            rungs: vec![rung(60, 20, 0, true)],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_subset_direction_is_child_in_parent() {
        // ⊆ 方向校验：父⊆子（反向）不成立——大区间作子级、小区间作父级 ⟹ child⊄parent。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(100, 0, 0), // 大区间作子级
            rungs: vec![rung(60, 20, 0, true)], // 小区间作父级
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_three_level_chain() {
        // ℓ=e+2：100 ⊇ 80 ⊇ 60，两递归级 Cand=1，买侧确认 ⟹ 1。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_interval: interval(60, 20, 0),                          // J^δ_e
            rungs: vec![rung(100, 0, 0, true), rung(80, 10, 0, true)],   // ℓ, ℓ-1
        };
        assert!(cert.n_delta());
        // 中间级 Cand=0 ⟹ 逐级合取翻转为 0。
        let cert_mid_false = NestCertificate {
            rungs: vec![rung(100, 0, 0, true), rung(80, 10, 0, false)],
            ..cert.clone()
        };
        assert!(!cert_mid_false.n_delta());
    }

    #[test]
    fn n_delta_short_direction_certificate() {
        // δ=Short：基例取 conf_minus（卖侧）。
        let cert = NestCertificate {
            side: Side::Short,
            terminal: sell1_bits(),
            base_interval: interval(60, 20, 0),
            rungs: vec![rung(100, 0, 0, true)],
        };
        assert!(cert.n_delta());
        // 同结构换 Long：卖侧 bit 对 Long 不确认 ⟹ 0。
        let cert_long = NestCertificate { side: Side::Long, ..cert.clone() };
        assert!(!cert_long.n_delta());
    }
}
