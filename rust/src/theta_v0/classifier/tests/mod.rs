//! classifier 单测集（#576 按域分文件；测试函数名集合与拆分前逐名一致）。

use super::super::types::Direction;
use super::*;

mod cache_and_units;
mod classify_basics;
mod incremental_tower;
mod level_signals;

fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
    Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
}

/// 构造 merged_bars：source_index 连续 0..n，close = vals（MACD 背驰真算用）。
fn bars_from_closes(vals: &[i64]) -> Vec<super::super::types::Bar> {
    vals.iter()
        .enumerate()
        .map(|(i, &v)| super::super::types::Bar {
            source_index: i,
            timestamp: i as i64,
            open: v,
            high: v,
            low: v,
            close: v,
            volume: 1,
            untradable: false,
        })
        .collect()
}
