//! 区间套递归证书 N^δ + Sel_Θ 固定选择器（rust 实装，对照 `Origin.IntervalNestCertificate`）。
//!
//! 契约锚：`formal/Origin/IntervalNestCertificate.lean`（L2-B 补全工位，cov-strategy G3 /
//! cov-classification F2 缺口）+ gpt 结果包 FULL_USER_FORMULA_SOURCE.md §6（line 312-369）。
//!
//! ── 实装内容（对照 Lean 逐项 parity） ──────────────────────────────────────────
//! - [`Interval`]：候选定位区间，携 Sel_Θ 三排序键 `(end_time, start_time, idx)`
//!   （对照 Lean `Strict.Nest.Interval`）。
//! - [`sel_order`]：Sel_Θ 字典序严格优（结束时间最新 ≻ 开始时间最新 ≻ 编号最小，
//!   对照 Lean `selOrder`/`selOrderB`）。
//! - [`select_theta`]：从候选列表唯一选出 Sel_Θ 最优者（对照 Lean `selectΘ`）。
//!   **全定义**：空列表 → `None`（无候选），非空 → `Some`（确定最优者）。
//! - [`NestLevel`]：区间套链一级（级别号 + 候选集 + Candidate/Confirm 标记，
//!   对照 Lean `NestLevel`）。
//! - [`chi_bool`]：N^δ 良基递归证书 χ^δ ∈ {0,1}（对照 Lean `chiBool`/`nestCertB`）。
//!   **全定义**：无候选 / 链不良构 / 区间套不成立 ⟹ `false`（= 0），**绝不未定义**
//!   （对照结果包「若不存在候选，则值为 0，绝不能是"未定义"」）。
//!
//! ── 认识论等级（formalization-validity-domain） ──────────────────────────────
//! 全部 **L0**（纯结构 / 全序消歧 / 级别比较良基递归，不依赖数据）。各级 `candidate_ok`/
//! `confirm_ok` 是 Bool 标记（缠论语义内容由下游 BSP/Center 契约，L2 需真实 K 线 + 力度）——
//! 本模块给 N^δ 的**结构形式**（级别比较 + 区间套 ⊆ + Sel_Θ 选 chosen），不算语义内容。

/// 候选定位区间 —— 携 Sel_Θ 三排序键（对照 Lean `Strict.Nest.Interval`）。
///
/// - `end_time`：结束时间（第一排序键，最新者优先 ⟹ 数值最大优先）。
/// - `start_time`：开始时间（第二排序键，最新者优先 ⟹ 数值最大优先）。
/// - `idx`：编号（第三排序键，最小者优先 ⟹ 数值最小优先）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub end_time: u64,
    pub start_time: u64,
    pub idx: u64,
}

impl Interval {
    /// 构造候选区间。
    pub fn new(end_time: u64, start_time: u64, idx: u64) -> Interval {
        Interval { end_time, start_time, idx }
    }

    /// Sel_Θ 字典序键 `(end_time, start_time, idx)`（对照 Lean `selKey`）。
    pub fn sel_key(&self) -> (u64, u64, u64) {
        (self.end_time, self.start_time, self.idx)
    }
}

/// Sel_Θ 字典序严格优 —— `sel_order(a, b)` ⟺ `a` 在 Sel_Θ 序下严格优于 `b`
/// （对照 Lean `selOrder`/`selOrderB`）：
/// 结束时间最新（end_time 大）≻ 开始时间最新（start_time 大）≻ 编号最小（idx 小）。
pub fn sel_order(a: &Interval, b: &Interval) -> bool {
    b.end_time < a.end_time
        || (a.end_time == b.end_time && b.start_time < a.start_time)
        || (a.end_time == b.end_time && a.start_time == b.start_time && a.idx < b.idx)
}

/// Sel_Θ 选择函数 —— 从候选列表选出 Sel_Θ 最优者（对照 Lean `selectΘ`）。
///
/// - 空列表 ⟹ `None`（**无候选**，对照结果包「不存在候选，值为 0」——下游 [`chi_bool`] 据此返回 0）。
/// - 非空 ⟹ `Some(最优者)`：以首元为初值逐对裁决（[`sel_better`]），选出全序最优。
///
/// **全定义**（对任意列表有确定结果）：空 → None，非空 → Some。无「未定义」。
pub fn select_theta(cands: &[Interval]) -> Option<Interval> {
    match cands.split_first() {
        None => None,
        Some((&first, rest)) => Some(rest.iter().fold(first, |cur, &x| sel_better(cur, x))),
    }
}

/// 逐对裁决 —— 返回 Sel_Θ 序下更优者（平局/键同保留 `cur`，对照 Lean `selBetter`）。
fn sel_better(cur: Interval, x: Interval) -> Interval {
    if sel_order(&x, &cur) {
        x
    } else {
        cur
    }
}

/// 区间套链一级 ℓ_j 的全部数据（对照 Lean `NestLevel`）。
///
/// - `lvl`：本级别号 ℓ_j（与执行级 e_v 比较决定 Confirm/Candidate 分支）。
/// - `cands`：本级别候选区间集（Sel_Θ 选 chosen）。
/// - `candidate_ok`：本级别候选标记 `Candidate^δ_{ℓ_j,t}`（缠论语义下游契约，Bool 标记）。
/// - `confirm_ok`：本级别（作为执行级终端时）确认标记 `Confirm^δ_{e_v,t}`（仅 lvl = e_v 时用）。
#[derive(Debug, Clone)]
pub struct NestLevel {
    pub lvl: u32,
    pub cands: Vec<Interval>,
    pub candidate_ok: bool,
    pub confirm_ok: bool,
}

impl NestLevel {
    /// 本级别 Sel_Θ 选出的 chosen 区间（无候选 = None，对照 Lean `NestLevel.chosen`）。
    pub fn chosen(&self) -> Option<Interval> {
        select_theta(&self.cands)
    }
}

/// 区间套 ⊆ 的 Bool 判定 —— 次级别 chosen 套在本级别 chosen 之内
/// （`J_{ℓ_{j+1}} ⊆ J_{ℓ_j}`，对照 Lean `subB`）。
///
/// `Sub J' J := J'.start_time ≥ J.start_time ∧ J'.end_time ≤ J.end_time`（次级别区间更窄）。
/// 任一级无候选（chosen = None）⟹ 无法判区间套 ⟹ `false`（无候选 = 区间套不成立 = 0）。
fn sub_b(sub: &NestLevel, par: &NestLevel) -> bool {
    match (sub.chosen(), par.chosen()) {
        (Some(js), Some(jp)) => jp.start_time <= js.start_time && js.end_time <= jp.end_time,
        _ => false,
    }
}

/// N^δ 良基递归证书 χ^δ ∈ {0,1}（Bool 全定义，对照 Lean `chiBool`/`nestCertB`）。
///
/// 输入：执行级 `ev`、级别链 `chain`（ℓ₀ 在表头，级别严格递减）。按结果包 §6 递归：
/// - **空链** ⟹ `false`（无级别 = 无候选 = 0）。
/// - **末级 ℓ** 且 `ℓ.lvl == ev` ⟹ 终端 `Confirm^δ` = `ℓ.confirm_ok`（且本级有候选）。
/// - **链头 ℓ + 次级 + 余链**（`ℓ.lvl > ev`）⟹ `Candidate^δ ∧ [次级 ⊆ 本级] ∧ 次级严格低一级 ∧ 递归`。
/// - **级别不匹配 / 链反向** ⟹ `false`（链不良构 = 无定位 = 0）。
///
/// **全定义**（对任意输入有确定 Bool 值，**绝不未定义**）：无候选 / 链不良构 / 区间套不成立
/// ⟹ `false` = 0。对照结果包「χ^δ ∈ {0,1}」「若不存在候选，则值为 0，绝不能是"未定义"」。
pub fn chi_bool(ev: u32, chain: &[NestLevel]) -> bool {
    match chain {
        [] => false,
        [last] => last.lvl == ev && last.chosen().is_some() && last.confirm_ok,
        [head, next, rest @ ..] => {
            if head.lvl == ev {
                // 已到执行级却还有下级链 ⟹ 链不良构（执行级应是末级）⟹ 0
                false
            } else if head.lvl > ev {
                // 操作级/中间级：候选成立 ∧ 次级严格低一级 ∧ 次级套本级 ∧ 递归
                let mut tail = Vec::with_capacity(rest.len() + 1);
                tail.push(next.clone());
                tail.extend_from_slice(rest);
                head.candidate_ok
                    && next.lvl < head.lvl
                    && sub_b(next, head)
                    && chi_bool(ev, &tail)
            } else {
                // head.lvl < ev：链反向（违反 o_v > … > e_v）⟹ 0
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sel_Θ 全序消歧 parity（对照 Lean witness_selectΘ_* 三键各一） ──────────────

    /// 对照 Lean `witness_selectΘ_picks_latest`：三候选 end_time 5/8/3，选 end_time 最大者（=8）。
    #[test]
    fn select_theta_picks_latest_end_time() {
        let cands = [
            Interval::new(5, 0, 0),
            Interval::new(8, 0, 1),
            Interval::new(3, 0, 2),
        ];
        assert_eq!(select_theta(&cands), Some(Interval::new(8, 0, 1)));
    }

    /// 对照 Lean `witness_selectΘ_tiebreak_startTime`：end_time 同（=10），start_time 3/7/5，选最大（=7）。
    #[test]
    fn select_theta_tiebreak_start_time() {
        let cands = [
            Interval::new(10, 3, 0),
            Interval::new(10, 7, 1),
            Interval::new(10, 5, 2),
        ];
        assert_eq!(select_theta(&cands), Some(Interval::new(10, 7, 1)));
    }

    /// 对照 Lean `witness_selectΘ_tiebreak_idx`：end_time/start_time 同，idx 4/2/9，选最小（=2）。
    /// 见证第三排序键（编号最小）+ 全序不留平局。
    #[test]
    fn select_theta_tiebreak_idx() {
        let cands = [
            Interval::new(10, 5, 4),
            Interval::new(10, 5, 2),
            Interval::new(10, 5, 9),
        ];
        assert_eq!(select_theta(&cands), Some(Interval::new(10, 5, 2)));
    }

    /// 对照 Lean `witness_selectΘ_empty_none`：空候选 ⟹ None（无候选）。
    #[test]
    fn select_theta_empty_none() {
        assert_eq!(select_theta(&[]), None);
    }

    /// Sel_Θ 严格全序三歧性 parity（对照 Lean `selOrder_trichotomy`，不留平局）：
    /// 任意两键 a b，sel_order(a,b) ∨ sel_key(a)==sel_key(b) ∨ sel_order(b,a) 恰一成立。
    #[test]
    fn sel_order_trichotomy_no_tie() {
        let samples = [
            Interval::new(10, 5, 1),
            Interval::new(10, 5, 2),
            Interval::new(10, 7, 1),
            Interval::new(8, 5, 1),
            Interval::new(10, 5, 1),
        ];
        for a in &samples {
            for b in &samples {
                let ab = sel_order(a, b);
                let ba = sel_order(b, a);
                let eq = a.sel_key() == b.sel_key();
                // 恰一成立（三歧性）：true 的个数恰为 1。
                let count = [ab, ba, eq].iter().filter(|&&x| x).count();
                assert_eq!(count, 1, "trichotomy violated for {a:?} vs {b:?}");
            }
        }
    }

    // ── 测试用区间套节点（对照 Lean witExec/witMid/witOp 同值） ────────────────────

    fn wit_exec() -> NestLevel {
        NestLevel { lvl: 0, cands: vec![Interval::new(20, 10, 5)], candidate_ok: true, confirm_ok: true }
    }
    fn wit_mid() -> NestLevel {
        NestLevel { lvl: 1, cands: vec![Interval::new(22, 8, 3)], candidate_ok: true, confirm_ok: false }
    }
    fn wit_op() -> NestLevel {
        NestLevel { lvl: 2, cands: vec![Interval::new(25, 5, 1)], candidate_ok: true, confirm_ok: false }
    }

    /// 对照 Lean `witness_chiBool_one`：三级区间套链 χ^δ = 1（[5,25]⊇[8,22]⊇[10,20]）。
    #[test]
    fn chi_bool_three_level_one() {
        assert!(chi_bool(0, &[wit_op(), wit_mid(), wit_exec()]));
    }

    /// 对照 Lean `witness_chiBool_singleton_one`：操作级 = 执行级（链长 1）⟹ χ^δ = Confirm = 1。
    #[test]
    fn chi_bool_singleton_one() {
        assert!(chi_bool(0, &[wit_exec()]));
    }

    /// 对照 Lean `witness_chiBool_brokenNest_zero`：执行级 [2,30] 超出操作级 [5,25] ⟹ 区间套破 ⟹ 0。
    #[test]
    fn chi_bool_broken_nest_zero() {
        let broken_exec = NestLevel {
            lvl: 0,
            cands: vec![Interval::new(30, 2, 7)],
            candidate_ok: true,
            confirm_ok: true,
        };
        assert!(!chi_bool(0, &[wit_op(), broken_exec]));
    }

    /// 对照 Lean `chiBool_empty_eq_zero`：空链（无候选）⟹ χ^δ = 0（绝不未定义）。
    #[test]
    fn chi_bool_empty_zero() {
        assert!(!chi_bool(0, &[]));
    }

    /// 对照 Lean `chiBool_singleton_noCand_eq_zero`：末级无候选区间（cands 空）⟹ χ^δ = 0。
    /// 即使级别号匹配 + confirm = true，无候选仍 0（无候选 = 0 覆盖到终端层）。
    #[test]
    fn chi_bool_singleton_no_cand_zero() {
        let no_cand = NestLevel { lvl: 0, cands: vec![], candidate_ok: true, confirm_ok: true };
        assert!(!chi_bool(0, &[no_cand]));
    }

    /// χ^δ ∈ {0,1} 值域 parity（对照 Lean `chiBool_mem_zero_one`）：返回类型是 bool（恰两值）。
    #[test]
    fn chi_bool_is_boolean() {
        let r = chi_bool(0, &[wit_op(), wit_mid(), wit_exec()]);
        assert!(r || !r); // bool 恰 {false, true} = {0, 1}（类型层事实）
    }

    /// 级别链非严格递减 ⟹ χ^δ = 0（链不良构，对照 Lean nestCertB 的 decide(ℓ'.lvl < ℓ.lvl)）。
    #[test]
    fn chi_bool_non_decreasing_levels_zero() {
        // 次级 lvl=2 ≥ 本级 lvl=2（未严格递减）⟹ 链不良构 ⟹ 0
        let same_op = NestLevel { lvl: 2, cands: vec![Interval::new(22, 8, 3)], candidate_ok: true, confirm_ok: false };
        assert!(!chi_bool(0, &[wit_op(), same_op, wit_exec()]));
    }
}
