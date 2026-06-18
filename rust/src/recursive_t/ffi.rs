//! PyO3 binding for 统一递归算子 T。
//!
//! a₀ = 线段序列（与 v3 递归层 `move = zhongshu_from_segments` 对齐，设计文档 §2），
//! 输出全塔买卖点。用于 T 输出与 v3 nf 信号 L2 对照（§6.6 阶段2）+ 三模式回测对照
//! （编排者 2026-06-18 裁决「三条路实测」，escalation `2026-06-18-1752-t-stepc-macd-
//! scope-vs-direction.md`）。
//!
//! 步骤c 走势完美判定模式由 `mode` 选取（默认纯结构，向后兼容历史单参数调用）；MACD
//! 面积由调用方经 `seg_areas` 注入（每段红/绿柱面积），T 自身不算 MACD——保持 standalone
//! 纯结构定位，close 价格不污染 T 的拓扑构造（escalation 附录·坐标系陷阱）。

use pyo3::prelude::*;

use super::{iterate, Direction, PerfectionMode, Unit};

/// 解析方向字符串。
fn parse_dir(s: &str) -> Direction {
    match s {
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("invalid segment direction: {other:?}"),
    }
}

/// 解析走势完美判定模式字符串。默认（`None`）= `Structural`；兼容 "structure"/"structural"。
fn parse_mode(s: Option<&str>) -> PerfectionMode {
    match s {
        None | Some("structure") | Some("structural") => PerfectionMode::Structural,
        Some("and") => PerfectionMode::And,
        Some("or") => PerfectionMode::Or,
        Some(other) => panic!("invalid perfection mode: {other:?} (expect structure/and/or)"),
    }
}

/// 用线段序列驱动 T，返回全塔买卖点。
///
/// `segs`：每段 `(i0, i1, dir, high, low, confirmed, kind_is_settled)`——与
/// `segments_from_strokes_v1` 输出对齐。仅 `confirmed AND settled` 线段作为 a₀ 单元
/// （与 v3 `zhongshu_from_segments` 过滤口径一致，保证 T/v3 在同一 a₀ 上对照）。
///
/// `seg_areas`（可选）：每段 `(area_pos, area_neg)`，**与 `segs` 同序同长**（过滤前对齐）。
/// 调用方经 `macd::macd_area_for_range` + `merged_to_raw` 算出（均取非负，绿柱用 `.abs()`），
/// 供 `And`/`Or` 模式的 MACD 面积背驰判据。`None` → area 全 0（纯结构，与历史单参数调用
/// 逐位等价）。
///
/// `mode`（可选）：步骤c 走势完美判定模式 "structure"（默认）/ "and" / "or"。
///
/// 返回：`(kind, bar, price, level)` 列表。kind ∈ {type1_buy/sell, type2_*, type3_*}；
/// bar = 买卖点所在 a₀ 单元端点的 bar（i0/i1，**merged bar 坐标**——回测取 close 价须经
/// `merged_to_raw` 转 raw bar）；level = T 迭代深度（0 = 最低走势级别，由线段构成；
/// 向上递归到涌现上界 r*）。
#[pyfunction]
#[pyo3(signature = (segs, seg_areas=None, mode=None))]
pub fn run_recursive_t(
    segs: Vec<(usize, usize, String, f64, f64, bool, bool)>,
    seg_areas: Option<Vec<(f64, f64)>>,
    mode: Option<String>,
) -> Vec<(String, i64, f64, usize)> {
    let perfection = parse_mode(mode.as_deref());
    // area 与 segs 同序对齐（过滤前）；缺省全 0。长度不符 = 调用方错误，fail fast。
    let areas: Vec<(f64, f64)> = match seg_areas {
        Some(a) => {
            assert_eq!(
                a.len(),
                segs.len(),
                "seg_areas 长度 ({}) 须与 segs ({}) 一致",
                a.len(),
                segs.len()
            );
            a
        }
        None => vec![(0.0, 0.0); segs.len()],
    };
    let units: Vec<Unit> = segs
        .iter()
        .zip(areas.iter())
        .filter(|((_, _, _, _, _, confirmed, settled), _)| *confirmed && *settled)
        .map(|((i0, i1, dir, high, low, _, _), (area_pos, area_neg))| Unit {
            high: *high,
            low: *low,
            start_bar: *i0 as i64,
            end_bar: *i1 as i64,
            direction: parse_dir(dir),
            level: 0,
            inner_zhongshu_count: 0,
            area_pos: *area_pos,
            area_neg: *area_neg,
        })
        .collect();
    let tree = iterate(units, perfection);
    tree.all_bsps()
        .iter()
        .map(|b| (b.kind.as_str().to_string(), b.bar, b.price, b.level))
        .collect()
}
