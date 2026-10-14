//! #1406 / 根裁定 OQ-1：外效到重的累计分配函数（ADR 0032 裁定二/三修订，
//! 政策 `frozen-manifest-proportional-cumulative/2`）。
//!
//! Balinski–Young 配额法：逐单位 f = 1..F，在满足 a_i + 1 ≤ ⌈f·q_i/Q⌉ 的重中取 q_i/(a_i+1)
//! 最大者加 1；并列先给冻结请求量较大者，再按 manifest 的 `tie_break_order`。
//! 性质（资金要求）：ΣA = F、A_i 单调不减、⌊F·q_i/Q⌋ ≤ A_i ≤ ⌈F·q_i/Q⌉、A_i(Q) = q_i。
//! 逐笔与累计、成交与 bust 反冲、佣金分摊全部只调本模块这一份函数（#799、#804）。
// #1406 C4 接线前生产路径尚无调用者，只被本模块测试消费。
#![cfg_attr(not(test), allow(dead_code))]

/// 政策 id：只此一版；`/1`（最大余数法）从未持久落地，无旧库按 `/1` 恢复。
pub(crate) const FILL_ALLOCATION_POLICY_ID: &str = "frozen-manifest-proportional-cumulative/2";

/// 本引擎逐单位推进的上界（技术资源界，不是资金政策值）：超出返回 typed Unsupported。
pub(crate) const MAX_APPORTION_UNITS: i64 = 1 << 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Apportionment {
    pub alloc: Vec<i64>,
    /// 优先级与冻结请求量都相同、只能由 `tie_break_order` 决定的单位数（ADR 0032 裁定三：TB-06 报告用）。
    pub tie_breaks: u64,
}

fn validate(total: i64, weights: &[i64], tie_rank: &[usize]) -> Result<i128, String> {
    if weights.is_empty() || tie_rank.len() != weights.len() {
        return Err("InvalidDomain：分配参与集合为空或 tie_break_order 长度不符".into());
    }
    let mut seen = vec![false; weights.len()];
    for &r in tie_rank {
        if r >= weights.len() || std::mem::replace(&mut seen[r], true) {
            return Err("InvalidDomain：tie_break_order 不是参与集合的排列".into());
        }
    }
    if weights.iter().any(|&w| w <= 0) {
        return Err("InvalidDomain：冻结请求量须为正整数".into());
    }
    let q = weights
        .iter()
        .try_fold(0i128, |a, &w| a.checked_add(i128::from(w)));
    let q = q.ok_or("InvalidDomain：冻结请求量合计溢出")?;
    if total < 0 || i128::from(total) > q {
        return Err("InvalidDomain：累计量须在 0..=Σ冻结请求量".into());
    }
    if total > MAX_APPORTION_UNITS {
        return Err("Unsupported：累计量超出本引擎逐单位分配界（TestOnly 技术界）".into());
    }
    Ok(q)
}

/// `tie_rank[i]` = 第 i 个参与重在 manifest `tie_break_order` 中的位置（须为 0..n 的排列）。
pub(crate) fn apportion(
    total: i64,
    weights: &[i64],
    tie_rank: &[usize],
) -> Result<Apportionment, String> {
    let q = validate(total, weights, tie_rank)?;
    let mut alloc = vec![0i64; weights.len()];
    let mut tie_breaks = 0u64;
    for f in 1..=i128::from(total) {
        let mut best: Option<usize> = None;
        let mut tied = false;
        for (i, &w) in weights.iter().enumerate() {
            let w = i128::from(w);
            let upper = (f * w + q - 1) / q;
            if i128::from(alloc[i]) + 1 > upper {
                continue;
            }
            let Some(b) = best else {
                best = Some(i);
                continue;
            };
            let wb = i128::from(weights[b]);
            // q_i/(a_i+1) 与 q_b/(a_b+1) 交叉相乘，精确比较。
            let lhs = w * (i128::from(alloc[b]) + 1);
            let rhs = wb * (i128::from(alloc[i]) + 1);
            let better = if lhs != rhs {
                lhs > rhs
            } else if w != wb {
                w > wb
            } else {
                tie_rank[i] < tie_rank[b]
            };
            if lhs == rhs && w == wb {
                tied = true;
            }
            if better {
                if lhs != rhs || w != wb {
                    tied = false;
                }
                best = Some(i);
            }
        }
        let b = best.ok_or("InvalidDomain：配额法无可分配者（违反上配额不变式）")?;
        alloc[b] += 1;
        if tied {
            tie_breaks += 1;
        }
    }
    Ok(Apportionment { alloc, tie_breaks })
}

/// `A(to) − A(from)`；`from ≤ to`。成交增量与 bust 反冲（`A(F) − A(F−b)`）都走这里。
pub(crate) fn increments(
    from: i64,
    to: i64,
    weights: &[i64],
    tie_rank: &[usize],
) -> Result<Vec<i64>, String> {
    if from > to {
        return Err("InvalidDomain：增量区间倒置".into());
    }
    let hi = apportion(to, weights, tie_rank)?.alloc;
    let lo = apportion(from, weights, tie_rank)?.alloc;
    let d: Vec<i64> = hi.iter().zip(&lo).map(|(h, l)| h - l).collect();
    if d.iter().any(|&x| x < 0) {
        return Err("InvalidDomain：分配增量为负（单调性被破坏）".into());
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rows(doc: &str, key: &str) -> Vec<Value> {
        serde_json::from_str::<Value>(doc).unwrap()[key]
            .as_array()
            .unwrap()
            .clone()
    }
    fn ints(v: &Value) -> Vec<i64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect()
    }
    fn ranks(v: &Value) -> Vec<usize> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect()
    }
    const HAND: &str = include_str!("../../../s_session/tests/fixtures/tb03c/HAND-ORACLE.json");
    const TABLE: &str =
        include_str!("../../../s_session/tests/fixtures/tb03c/APPORTION-ORACLE.json");

    #[test]
    fn hand_tables_match_including_tie_rules() {
        for case in rows(HAND, "apportion_cases") {
            let w = ints(&case["weights"]);
            let t = ranks(&case["tie_rank"]);
            for (f, want) in case["sequence"].as_array().unwrap().iter().enumerate() {
                assert_eq!(
                    apportion(f as i64, &w, &t).unwrap().alloc,
                    ints(want),
                    "{}",
                    case["name"]
                );
            }
        }
    }

    #[test]
    fn exhaustive_small_domain_equals_independent_oracle_table() {
        let table = rows(TABLE, "rows");
        assert_eq!(table.len(), 2 * (36 + 125 + 81));
        for row in table {
            let w = ints(&row["weights"]);
            let t = ranks(&row["tie_rank"]);
            for (f, want) in row["sequence"].as_array().unwrap().iter().enumerate() {
                assert_eq!(
                    apportion(f as i64, &w, &t).unwrap().alloc,
                    ints(want),
                    "{w:?} {t:?} F={f}"
                );
            }
        }
    }

    #[test]
    fn declared_properties_hold_on_every_prefix() {
        for (w, t) in [
            (vec![1, 3, 3], vec![0, 1, 2]),
            (vec![7, 2, 9, 4, 1], vec![4, 3, 2, 1, 0]),
            (vec![13, 13, 13], vec![2, 0, 1]),
            (vec![40, 1], vec![1, 0]),
        ] {
            let q: i64 = w.iter().sum();
            let mut prev = vec![0; w.len()];
            for f in 0..=q {
                let a = apportion(f, &w, &t).unwrap().alloc;
                assert_eq!(a.iter().sum::<i64>(), f);
                for i in 0..w.len() {
                    assert!(a[i] >= prev[i], "单调 {w:?} F={f}");
                    assert!(
                        a[i] * q >= f * w[i] - (q - 1) && a[i] * q <= f * w[i] + (q - 1),
                        "配额界"
                    );
                    assert!(a[i] <= w[i]);
                }
                prev = a;
            }
            assert_eq!(prev, w, "A(Q)=q");
        }
    }

    #[test]
    fn adr0032_counterexample_has_no_negative_increment() {
        // 原文最大余数法在 q=(1,3,3)、F 3→4 给出 (1,1,1)→(0,2,2)。
        assert_eq!(
            increments(3, 4, &[1, 3, 3], &[0, 1, 2]).unwrap(),
            vec![0, 0, 1]
        );
    }

    #[test]
    fn cumulative_and_per_report_splits_agree() {
        // K9：同一累计量不论按几笔报来，逐笔增量之和都等于累计分配。
        let (w, t) = (vec![5, 2, 4], vec![1, 0, 2]);
        for cuts in [
            vec![0, 11],
            vec![0, 3, 11],
            vec![0, 1, 2, 7, 8, 11],
            (0..=11).collect(),
        ] {
            let mut sum = vec![0; 3];
            for pair in cuts.windows(2) {
                for (s, d) in sum
                    .iter_mut()
                    .zip(increments(pair[0], pair[1], &w, &t).unwrap())
                {
                    *s += d;
                }
            }
            assert_eq!(sum, apportion(11, &w, &t).unwrap().alloc);
        }
    }

    #[test]
    fn bust_reverses_exactly_under_same_function() {
        // T-21：撤 b 单位的反冲 = A(F) − A(F−b)，合计为 b、逐重非负；撤到零即全反冲。
        let (w, t) = (vec![1, 3, 3], vec![0, 1, 2]);
        let rev = increments(5, 7, &w, &t).unwrap();
        assert_eq!(rev, vec![1, 0, 1]);
        assert_eq!(increments(0, 7, &w, &t).unwrap(), w);
    }

    #[test]
    fn policy_id_is_the_quota_revision() {
        assert_eq!(
            FILL_ALLOCATION_POLICY_ID,
            "frozen-manifest-proportional-cumulative/2"
        );
    }

    #[test]
    fn tie_breaks_are_counted_for_reporting() {
        assert_eq!(apportion(3, &[1, 1, 1], &[1, 2, 0]).unwrap().tie_breaks, 2);
        assert_eq!(apportion(7, &[1, 3, 3], &[0, 1, 2]).unwrap().tie_breaks, 3);
        assert_eq!(apportion(3, &[2, 1], &[0, 1]).unwrap().tie_breaks, 0);
    }

    #[test]
    fn rejects_out_of_domain_inputs_with_typed_errors() {
        let bad: [(i64, Vec<i64>, Vec<usize>); 7] = [
            (1, vec![], vec![]),
            (1, vec![0, 2], vec![0, 1]),
            (1, vec![-1, 2], vec![0, 1]),
            (4, vec![1, 2], vec![0, 1]),
            (-1, vec![1, 2], vec![0, 1]),
            (1, vec![1, 2], vec![0, 0]),
            (1, vec![1, 2], vec![0]),
        ];
        for (f, w, t) in bad {
            assert!(
                apportion(f, &w, &t)
                    .unwrap_err()
                    .starts_with("InvalidDomain"),
                "{f} {w:?} {t:?}"
            );
        }
        assert!(increments(3, 2, &[1, 2], &[0, 1])
            .unwrap_err()
            .starts_with("InvalidDomain"));
        let big = MAX_APPORTION_UNITS + 1;
        assert!(apportion(big, &[big], &[0])
            .unwrap_err()
            .starts_with("Unsupported"));
    }
}
