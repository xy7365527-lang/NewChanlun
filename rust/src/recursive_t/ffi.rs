//! PyO3 binding for 统一递归算子 T。
//!
//! a₀ = 线段序列（与 v3 递归层 `move = zhongshu_from_segments` 对齐，设计文档 §2），
//! 输出全塔买卖点。用于第二步：T（纯结构）输出与 v3 nf 信号 L2 对照（§6.6 阶段2）。

use pyo3::prelude::*;

use super::{iterate, Direction, Unit};

/// 解析方向字符串。
fn parse_dir(s: &str) -> Direction {
    match s {
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("invalid segment direction: {other:?}"),
    }
}

/// 用线段序列驱动 T，返回全塔买卖点。
///
/// `segs`：每段 `(i0, i1, dir, high, low, confirmed, kind_is_settled)`——与
/// `segments_from_strokes_v1` 输出对齐。仅 `confirmed AND settled` 线段作为 a₀ 单元
/// （与 v3 `zhongshu_from_segments` 过滤口径一致，保证 T/v3 在同一 a₀ 上对照）。
///
/// 返回：`(kind, bar, price, level)` 列表。kind ∈ {type1_buy/sell, type2_*, type3_*}；
/// bar = 买卖点所在 a₀ 单元端点的 bar（i0/i1）；level = T 迭代深度（0 = 最低走势级别，
/// 由线段构成；向上递归到涌现上界 r*）。纯结构、零 MACD（§6.6 裁决）。
#[pyfunction]
pub fn run_recursive_t(
    segs: Vec<(usize, usize, String, f64, f64, bool, bool)>,
) -> Vec<(String, i64, f64, usize)> {
    let units: Vec<Unit> = segs
        .iter()
        .filter(|(_, _, _, _, _, confirmed, settled)| *confirmed && *settled)
        .map(|(i0, i1, dir, high, low, _, _)| {
            Unit::stroke(*low, *high, *i0 as i64, *i1 as i64, parse_dir(dir))
        })
        .collect();
    let tree = iterate(units);
    tree.all_bsps()
        .iter()
        .map(|b| (b.kind.as_str().to_string(), b.bar, b.price, b.level))
        .collect()
}
