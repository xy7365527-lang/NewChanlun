//! C2 盘整/趋势背驰候选事件 provider 的 run 语境身份与 memo 基础设施（#630 从
//! `level_view.rs` 拆出，纯移动零语义；来源票 #497 影子评审 MEDIUM-1）。
//! provider 入口函数（`provide_nest_candidate_events*`）见 `level_view_pan_provider.rs`。

use super::super::super::types::{Center, Direction, MoveKind, Segment, Tick};
use super::super::decompose::{MoveBlock, MoveStatus};
use super::super::divergence::{departure_move_c_start, locate_departure_move_a, self_anchors};
use super::confirm::{DivergencePair, DivergencePairId};
use super::projection::{leg_as_segment, ExactThreeProjection, LowerLeg};
use super::ProviderVersion;
use std::collections::HashMap;

pub(super) fn structural_block_span(
    projection: &ExactThreeProjection,
    block: &MoveBlock,
) -> Option<(usize, usize)> {
    Some((
        projection.seeds.get(block.start_center)?.start_index,
        projection.seeds.get(block.end_center)?.end_index,
    ))
}

pub(super) fn structural_pair_span(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    leave_index: usize,
) -> Option<(usize, usize)> {
    let leave = blocks.get(leave_index)?;
    let retest = blocks.get(leave_index + 1)?;
    if leave.status != MoveStatus::Completed || retest.status != MoveStatus::Completed {
        return None;
    }
    let leave = structural_block_span(projection, leave)?;
    let retest = structural_block_span(projection, retest)?;
    Some((leave.0, retest.1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PanSegmentIdentity {
    up: bool,
    start_index: usize,
    end_index: usize,
    start_price: Tick,
    end_price: Tick,
}

impl From<&Segment> for PanSegmentIdentity {
    fn from(segment: &Segment) -> Self {
        Self {
            up: segment.direction == Direction::Up,
            start_index: segment.start_index,
            end_index: segment.end_index,
            start_price: segment.start_price,
            end_price: segment.end_price,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PanCenterIdentity {
    zd: Tick,
    zg: Tick,
    dd: Tick,
    gg: Tick,
    start_index: usize,
    end_index: usize,
}

impl From<&Center> for PanCenterIdentity {
    fn from(center: &Center) -> Self {
        Self {
            zd: center.zd,
            zg: center.zg,
            dd: center.dd,
            gg: center.gg,
            start_index: center.start_index,
            end_index: center.end_index,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PanBlockIdentity {
    start_center: usize,
    end_center: usize,
    trend: bool,
    direction: i8,
    completed: bool,
    start_source: usize,
    end_source: usize,
}

fn pan_block_identity(
    projection: &ExactThreeProjection,
    block: &MoveBlock,
    reads_status: bool,
) -> Option<PanBlockIdentity> {
    Some(PanBlockIdentity {
        start_center: block.start_center,
        end_center: block.end_center,
        trend: block.kind == MoveKind::Trend,
        direction: match block.dir {
            Some(Direction::Up) => 1,
            Some(Direction::Down) => -1,
            None => 0,
        },
        // target/第一后继的 status 进入 structural_pair_span；第二后继只作存在性封口门。
        // 忽略第二后继的 Active→Completed，保证纯追加第四块不清已证目标。
        completed: reads_status && block.status == MoveStatus::Completed,
        start_source: projection.seeds.get(block.start_center)?.start_index,
        end_source: projection.seeds.get(block.end_center)?.end_index,
    })
}

pub(super) fn pan_owner_block_index(blocks: &[MoveBlock], center_index: usize) -> Option<usize> {
    if center_index == 0 {
        return (!blocks.is_empty()).then_some(0);
    }
    blocks
        .iter()
        .position(|block| block.start_center < center_index && center_index <= block.end_center)
}

pub(super) fn pan_block_triple(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    block_index: usize,
) -> Option<[PanBlockIdentity; 3]> {
    // 链②唯一门：目标块之后至少两个完整身份槽；不足时稳定资格不存在。
    (block_index + 2 < blocks.len()).then_some(())?;
    Some([
        pan_block_identity(projection, blocks.get(block_index)?, true)?,
        pan_block_identity(projection, blocks.get(block_index + 1)?, true)?,
        pan_block_identity(projection, blocks.get(block_index + 2)?, false)?,
    ])
}

/// run 语境键：level/window/version + segment/center + ownership block 与两个后继块。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PanMemoKey {
    pub(super) level: u32,
    pub(super) provider_window: (usize, usize),
    pub(super) projection_version: ProviderVersion,
    pub(super) segment_index: usize,
    pub(super) segment: PanSegmentIdentity,
    pub(super) center_index: usize,
    pub(super) center: PanCenterIdentity,
    pub(super) block_index: usize,
    pub(super) blocks: [PanBlockIdentity; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanEventCore {
    pub(super) structure: super::super::signal::PanDivStructure,
    pub(super) interval_a: (usize, usize),
    pub(super) intake_fallback: bool,
    pub(super) divergence_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanMemoValue {
    NoEvent,
    Event(PanEventCore),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PanMemoEntry {
    /// 本 entry 实际读取的最大 source_index；复用要求严格 `< e_src`。
    read_end_src: usize,
    /// 定位窄锚/front-anchor/Extreme 实际可见的段前缀长度。
    read_segment_count: usize,
    value: PanMemoValue,
}

/// #69 5b memo 诊断计数；只描述 memo 行为，不参与事件语义。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PanMemoStats {
    pub hits: usize,
    pub misses: usize,
    pub writes: usize,
    pub invalidations: usize,
}

/// #69 5b：由调用方按 run 持有的盘整背驰 memo。
///
/// 状态只经显式 [`PanResidence`] 进入 provider；默认入口与 `None` 始终走真冷路径。
#[derive(Debug, Default)]
pub struct PanMemo {
    entries: HashMap<PanMemoKey, PanMemoEntry>,
    /// run 的逐项精确 lower-segment 快照；entry 只存读前缀长度，避免每项复制整段前缀。
    segment_snapshot: Vec<PanSegmentIdentity>,
    stats: PanMemoStats,
}

impl PanMemo {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn stats(&self) -> PanMemoStats {
        self.stats
    }

    pub(super) fn prepare(
        &mut self,
        level: u32,
        provider_window: (usize, usize),
        projection_version: ProviderVersion,
        projection: &ExactThreeProjection,
        blocks: &[MoveBlock],
        segments: &[Segment],
        centers: &[Center],
        freeze_boundary_src: usize,
    ) {
        let current_segments: Vec<_> = segments.iter().map(PanSegmentIdentity::from).collect();
        let common_segment_prefix = self
            .segment_snapshot
            .iter()
            .zip(&current_segments)
            .take_while(|(old, current)| old == current)
            .count();
        let before = self.entries.len();
        self.entries.retain(|key, entry| {
            key.level == level
                && key.provider_window == provider_window
                && key.projection_version == projection_version
                && entry.read_end_src < freeze_boundary_src
                && entry.read_segment_count <= common_segment_prefix
                && segments
                    .get(key.segment_index)
                    .is_some_and(|segment| PanSegmentIdentity::from(segment) == key.segment)
                && centers
                    .get(key.center_index)
                    .is_some_and(|center| PanCenterIdentity::from(center) == key.center)
                && pan_block_triple(projection, blocks, key.block_index)
                    .is_some_and(|blocks| blocks == key.blocks)
        });
        self.stats.invalidations += before - self.entries.len();
        self.segment_snapshot = current_segments;
    }

    pub(super) fn lookup(&mut self, key: &PanMemoKey) -> Option<PanMemoValue> {
        match self.entries.get(key).map(|entry| entry.value) {
            Some(entry) => {
                self.stats.hits += 1;
                Some(entry)
            }
            None => {
                self.stats.misses += 1;
                None
            }
        }
    }

    pub(super) fn insert(
        &mut self,
        key: PanMemoKey,
        read_end_src: usize,
        read_segment_count: usize,
        value: PanMemoValue,
    ) {
        self.entries.insert(
            key,
            PanMemoEntry {
                read_end_src,
                read_segment_count,
                value,
            },
        );
        self.stats.writes += 1;
    }

    #[cfg(test)]
    pub(super) fn poison_for_test(&mut self) {
        let entry = self
            .entries
            .values_mut()
            .find(|entry| matches!(entry.value, PanMemoValue::Event(_)))
            .expect("测试夹具须已有 event memo");
        let PanMemoValue::Event(mut core) = entry.value else {
            unreachable!("上方已筛 Event");
        };
        core.divergence_confirmed = !core.divergence_confirmed;
        entry.value = PanMemoValue::Event(core);
    }
}

/// #69 5b：pan memo 的显式 resident seam。`freeze_boundary_src` 是 source_index 量纲；
/// `None` 表示完全绕过 memo 的真冷路径。
pub struct PanResidence<'a> {
    pub memo: &'a mut PanMemo,
    pub freeze_boundary_src: usize,
}

/// D2 provider：只读 `MoveBlock.dir`。盘整 `None` 严格产零 pair；A/C 是同趋势方向、
/// 分属相邻两个中心锚后的离开 episode，定界复用既有 divergence episode helpers。
pub fn provide_divergence_pairs(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    as_of: usize,
) -> Vec<DivergencePair> {
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let anchors = self_anchors(&segments);
    let mut pairs = Vec::new();
    for block in blocks {
        let Some(direction) = block.dir else {
            continue;
        };
        if block.end_center <= block.start_center || block.end_center >= projection.seeds.len() {
            continue;
        }
        let prev = projection.seeds[block.end_center - 1].center;
        let last = projection.seeds[block.end_center].center;
        let Some(seg_a) = locate_departure_move_a(&segments, &anchors, &prev, &last, direction)
        else {
            continue;
        };
        // #104/#105 裁定修复（di-trend-c-terminal-forward-find-ruling-20260716）：
        // C 终段 = 离开最后中枢后的**第一个**同向段（正向 find）。此前 rev().find 取
        // 全域最后一个同向段，最终快照下 C 段被锚到数据末端，Trend 背驰恒 false。
        let Some(c_terminal) = segments.iter().find(|segment| {
            segment.direction == direction
                && segment.start_index >= last.end_index
                && segment.end_index <= as_of
        }) else {
            continue;
        };
        let Some(c_start) = departure_move_c_start(
            &segments,
            &anchors,
            &last,
            direction,
            c_terminal.start_index,
        ) else {
            continue;
        };
        // ★R1（2026-07-17 代理裁定，p112 实证）：seg_c 取段 = **全离开段**——c_end 从首个
        // 同向段终点扩展为 c_start 起、end ≤ as_of 的**末个同向段**终点（037:22 全离开走势
        // 口径，p112 T3_ext 窗口）。#105 的单腿窗口（win_legs==1 @154/154）使 037:18 三买
        // 构造性无处容身；全离开段下 c 内含回拉段，三买可判。单段离开的退化名与新名逐位一致
        // （c_end == c_terminal.end）。确认时点（t*）收束在 trend_confirm_time 消费侧完成。
        let Some(c_end) = segments
            .iter()
            .rev()
            .find(|segment| {
                segment.direction == direction
                    && segment.start_index >= c_start
                    && segment.end_index <= as_of
            })
            .map(|segment| segment.end_index)
        else {
            continue;
        };
        pairs.push(DivergencePair {
            id: DivergencePairId {
                level,
                block_start_center: block.start_center,
                block_end_center: block.end_center,
                direction,
            },
            move_start: projection.seeds[block.start_center].start_index,
            seg_a,
            seg_c: (c_start, c_end),
        });
    }
    pairs
}
