//! center_break 桥接类型（#1719 I4：中枢破坏到活假设第二层清账桥）。
//!
//! 三端点桥接：
//! 1. 事实产生：`CenterEventMachine::Broken` → `CenterBreakFact`
//! 2. 对象匹配：`PanLive` + `Book` → `HypothesisBridge`
//! 3. 第二层清账：`CenterBreakFact` + `Bridge` → `Invalidated(CenterBreak)`

use super::super::center_lifecycle::CenterId;
use super::super::types::Side;
use super::key::LifecycleKey;
use std::collections::{BTreeMap, BTreeSet};

/// 中枢破坏事实（第二层清账输入，ADR-0003 completion_anchor=center_break 路径）。
///
/// 来源：`CenterEventMachine::Broken` 事件携带完整中枢身份与破坏证据。
/// 本结构体是该事件的持久化提取，用于账本清账与恢复。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CenterBreakFact {
    /// 被破坏中枢的精确身份（`start_index + zd + zg`，对延伸稳定）。
    pub center_id: CenterId,
    /// 破坏发生的级别。
    pub level: u32,
    /// 破坏证据（三类点）的源坐标。
    pub breaker_source_index: usize,
    /// 破坏侧（buy3/sell3）。
    pub breaker_side: Side,
    /// 首次已知该事实的 bar（物理事实钟，区别于消费 bar）。
    pub known_at: usize,
}

impl CenterBreakFact {
    /// 从 `CenterLifecycleEvent::Broken` 提取破坏事实（#1719 I4 端点 1：事实产生）。
    ///
    /// 输入：`Broken` 事件（完整中枢身份、破坏证据、级别）+ 首次已知该事实的 bar。
    /// 输出：可持久化的 `CenterBreakFact`（用于清账与恢复）。
    ///
    /// `known_at` 原则：该事件首次对本系统可见的 bar（通常 = 事件产出的当前 bar）。
    /// 迟到事实的 `known_at` ≠ 物理破坏钟（`breaker_source_index` 是物理坐标）。
    pub fn from_broken_event(
        level: u32,
        center: &super::super::types::Center,
        breaker_source_index: usize,
        breaker_side: Side,
        known_at: usize,
    ) -> Self {
        Self {
            center_id: CenterId::of(center),
            level,
            breaker_source_index,
            breaker_side,
            known_at,
        }
    }

    /// 去重键：同一中枢的同一破坏证据视为同一事实。
    ///
    /// 幂等原则：重复喂入同一事实不得重复清账。去重键不含 `known_at`
    /// （迟到事实的物理钟可能不同，但实质是同一破坏）。
    pub fn dedup_key(&self) -> (CenterId, usize) {
        (self.center_id, self.breaker_source_index)
    }
}

/// 活假设对象的中枢绑定（注册时保留精确身份，支撑桥匹配与恢复）。
///
/// 绑定原则（考古文档精确匹配要求）：
/// - level + 完整 `CenterId`（含 zd/zg，不只 `start_index`）+ 原 `LifecycleKey`
/// - 具名政策版本（#1599：一次运行只读一个版本，改值即新版本）
/// - 三重绑定可证同源，支持精确对象恢复
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypothesisBridge {
    /// 原活假设账本键（六元组：level/direction/domain/seg_a/c_left/b_center_start）。
    pub lifecycle_key: LifecycleKey,
    /// 绑定的 B 中枢精确身份（完整核心，不只起点）。
    pub center_id: CenterId,
    /// 具名完成政策版本（本次运行使用的版本，恢复时必须匹配）。
    pub policy_version: String,
    /// 注册时钟（对象首次可见的 bar）。
    pub registered_at: usize,
}

impl HypothesisBridge {
    /// 查询键：按级别和中枢身份匹配对象（用于 center_break 事实查找目标对象）。
    pub fn match_key(&self) -> (u32, CenterId) {
        (self.lifecycle_key.level, self.center_id)
    }
}
