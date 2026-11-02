//! #1394 2A-S：正式 S 的**同代增量状态**与稳定对象复用（内部缓存）。
//!
//! 三条边界（#1394 已授权）：
//!
//! 1. **缓存是性能结构**：不充任准入/证据权威。冷启动、身份不符、真实修订都退化为「从头
//!    重建状态 + 未命中即重算」，输出与全量路径逐字段相同；容量/命中状态不改变任何判据。
//! 2. **只从已证稳定前缀追加**：指纹逐位相同的输入前缀才复用；前缀内任一位置变化（真实
//!    revision / 撤回 / 回退）即从**最早受影响坐标**起失效事实缓存，并保守重放整段输入
//!    （parser 状态是 append-only，重放是 O(n) 追加，不是 O(n²) 重建）。不拒绝真实修订。
//! 3. **完整原件与 ID 复用**：稳定步骤/组直接复用上一代完整对象原件（同一内容构造出的
//!    `object_id`），本代只把元数据投影到本代 generation/cut；不重新展开来源集合、不重算
//!    规范字节与哈希。
//!
//! 缓存键 = 持久身份（库路径 + 会话 + 会话代 + 规则修订 + 档案 + 目录修订）。键不同即
//! 丢弃旧车道——不跨库、不跨规则、不跨档案复用。

use super::*;
use crate::v3::business_json as bj;
use newchan_rust::s_content::capture_allocation::audit_json::JsonView;
use newchan_rust::session_protocol::parse_canonical_i64;
use newchan_rust::theta_v0::parser::inclusion::{InclusionGroupFact, IncrFactsState};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// #1488：沿用 generation 的同一 OHLC 取值配方；delta 资格口与旧 full 都调用此函数。
pub(super) fn bar_of(event: JsonView<'_>) -> Result<Bar, String> {
    let price = |key: &str| -> Result<i64, String> {
        parse_canonical_i64(
            bj::field(event, key)
                .as_str()
                .ok_or_else(|| format!("StorageUnavailable：OHLC价 `{key}` 缺失"))?,
            key,
        )
    };
    Ok(Bar {
        source_index: usize::try_from(bj::coord(event)?)
            .map_err(|_| "StorageUnavailable：源坐标不能无损转换为 source_index".to_string())?,
        timestamp: parse_canonical_i64(bj::field(event, "ts").as_str().unwrap_or(""), "timestamp")?,
        open: price("open")?,
        high: price("high")?,
        low: price("low")?,
        close: price("close")?,
        volume: 1.0,
        untradable: false,
    })
}

/// 输入指纹：稳定前缀判定只比「解析真正吃进去的 6 个字段」。
#[derive(Clone, Copy, PartialEq, Eq)]
struct InputFp {
    coord: i64,
    timestamp: i64,
    open: i64,
    high: i64,
    low: i64,
    close: i64,
}

impl InputFp {
    fn of(bar: &Bar) -> Self {
        InputFp {
            coord: bar.source_index as i64,
            timestamp: bar.timestamp,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
        }
    }
}

/// 一代的推进结果（诊断/测量面，不进公开输出）。
///
/// 生产路径只写入、不读取（缓存行为本身已经体现在输出一致性上）；测量/测试面读取它。
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub(super) struct AdvanceOutcome {
    /// 逐位相同的稳定前缀长度。
    pub stable_prefix: usize,
    /// 本次真正追加的 bar 数（重放时计全量）。
    pub appended: usize,
    /// 是否因真实修订/回退触发保守重放。
    pub replayed: bool,
    /// 最早受影响坐标（重放时的失效起点）。
    pub invalidated_from: Option<i64>,
}

/// 同代车道：解析增量状态 + 输入指纹 + 事实原件缓存。
pub(super) struct Lane {
    key: String,
    fps: Vec<InputFp>,
    state: IncrFactsState,
    facts: tb02a_facts::FactCache,
    /// 事实依赖精确原始引用；OHLC 未变的 revision 也必须使相关原件失效。
    source_refs: Vec<(i64, Value)>,
    /// 最近一次推进的结果（测量面读取；不参与任何判据）。
    last: AdvanceOutcome,
}

impl Lane {
    fn new(key: &str) -> Self {
        Lane {
            key: key.to_string(),
            fps: Vec::new(),
            state: IncrFactsState::empty(),
            facts: tb02a_facts::FactCache::new(),
            source_refs: Vec::new(),
            last: AdvanceOutcome::default(),
        }
    }

    pub(super) fn state(&self) -> &IncrFactsState {
        &self.state
    }

    /// 最近一次推进结果（稳定前缀长度 / 追加数 / 是否重放 / 最早受影响坐标）。
    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "#1608：推进结果诊断读数，现无测试断言调用；保留供诊断"
    )]
    pub(super) fn last_outcome(&self) -> AdvanceOutcome {
        self.last
    }

    /// 从「真实生效输入」的已证稳定前缀追加；前缀变化则保守重放。
    pub(super) fn advance(&mut self, bars: &[Bar]) -> AdvanceOutcome {
        let fps: Vec<InputFp> = bars.iter().map(InputFp::of).collect();
        let common = self
            .fps
            .iter()
            .zip(&fps)
            .take_while(|(a, b)| a == b)
            .count();
        let mut outcome = AdvanceOutcome {
            stable_prefix: common,
            ..AdvanceOutcome::default()
        };
        if common == self.fps.len() {
            for bar in &bars[common..] {
                self.state.append(*bar);
            }
            outcome.appended = bars.len() - common;
            self.fps = fps;
            self.last = outcome;
            return outcome;
        }
        // 前缀内变化（真实修订/撤回/回退）：最早受影响坐标 = 首个不一致位置的**原坐标**。
        let from = self.fps[common].coord;
        self.facts.invalidate_from(from);
        outcome.replayed = true;
        outcome.invalidated_from = Some(from);
        // 保守重放：parser 状态从零按同一输入重放（append-only，O(n) 追加）。
        let mut state = IncrFactsState::empty();
        for bar in bars {
            state.append(*bar);
        }
        self.state = state;
        outcome.appended = bars.len();
        self.fps = fps;
        self.last = outcome;
        outcome
    }

    /// 用同代增量状态构造 CC-004/CC-005/CC-007/CC-054（与全量路径逐字段相同）。
    pub(super) fn build_typed(
        &mut self,
        group_facts: &[InclusionGroupFact],
        raw: &BTreeMap<i64, Value>,
        shapes: &[Value],
        gen: i64,
        cut: &str,
        frontier: i64,
    ) -> (Vec<Value>, Vec<Value>) {
        let current_refs: Vec<_> = raw
            .iter()
            .map(|(coord, event)| (*coord, tb02a_facts::raw_ref(event)))
            .collect();
        let common = self
            .source_refs
            .iter()
            .zip(&current_refs)
            .take_while(|(old, current)| old == current)
            .count();
        if common < self.source_refs.len() {
            let mut from = self.source_refs[common].0;
            if let Some((coord, _)) = current_refs.get(common) {
                from = from.min(*coord);
            }
            self.facts.invalidate_from(from);
        }
        self.source_refs = current_refs;
        tb02a_facts::build_incremental(
            &self.state,
            group_facts,
            raw,
            shapes,
            gen,
            cut,
            frontier,
            &mut self.facts,
        )
    }

    /// (事实原件命中, 未命中, 比较单元实算, 比较单元复用)。
    #[cfg(test)]
    pub(super) fn counts(&self) -> (usize, usize, usize, usize) {
        (
            self.facts.hits(),
            self.facts.misses(),
            self.facts.units.computed(),
            self.facts.units.reused(),
        )
    }

    /// 清空复用面（等价新进程冷启动；键不变）。
    #[cfg(test)]
    pub(super) fn clear_reuse(&mut self) {
        self.facts.clear();
        self.source_refs.clear();
        self.fps.clear();
        self.state = IncrFactsState::empty();
        self.last = AdvanceOutcome::default();
    }
}

/// #1488：仅由 owner 的当前 raw 资格推进的新车道；不与旧全 fps 车道交叉复用。
/// 新增保留容器由实际 CodecVec 持原账；既有 parser/fact 内部存储的分配闭合仍是独立责任。
pub(super) struct DeltaLane {
    rows: newchan_rust::s_content::CodecVec<DeltaRawRow>,
    state: IncrFactsState,
    facts: tb02a_facts::FactCache,
    binding: Option<crate::v3::audit_owner::RawAdvanceBinding>,
    prefix: usize,
    poisoned: bool,
    last_changes: Option<ParserChanges>,
    previous_len: usize,
    view_qualified: bool,
    /// 任意通用回调可能污染保留历史；只能通过新建车道清除，不能跨代重新签资格。
    unqualified_history: bool,
}

#[derive(Clone, Copy)]
pub(super) struct DeltaRawRow {
    pub ordinal: usize,
    pub bar: Bar,
}

/// #1488 A2：只借同一车道成功推进后的真实生效账；不保存跨事务 JsonView。
/// 构造口在 DeltaLane 内，消费者不能自行填写 dirty rank 或替换 parser。
#[derive(Clone, Copy)]
pub(super) struct EffectiveRawView<'a> {
    rows: &'a [DeltaRawRow],
    state: &'a IncrFactsState,
    bound: &'a crate::v3::audit_owner::BoundRawAdvance<'a>,
    changes: ParserChanges,
    previous_len: usize,
}

impl<'a> EffectiveRawView<'a> {
    pub(super) fn ensure_current(self) -> Result<(), String> {
        self.bound.ensure_current()
    }
    pub(super) fn bound(self) -> &'a crate::v3::audit_owner::BoundRawAdvance<'a> {
        self.bound
    }
    pub(super) fn state(self) -> &'a IncrFactsState {
        self.state
    }
    pub(super) fn len(self) -> usize {
        self.rows.len()
    }
    pub(super) fn previous_len(self) -> usize {
        self.previous_len
    }
    pub(super) fn dirty_from(self) -> usize {
        self.changes
            .effective_dirty_from
            .unwrap_or(self.rows.len())
            .min(self.rows.len())
    }
    pub(super) fn bars(self) -> RawBars<'a> {
        RawBars::Delta(self.rows)
    }
    pub(super) fn rank(self, coord: i64) -> Result<usize, usize> {
        self.rows
            .binary_search_by_key(&coord, |row| row.bar.source_index as i64)
    }
    pub(super) fn get(
        self,
        coord: i64,
    ) -> Option<newchan_rust::s_content::capture_allocation::audit_json::JsonView<'a>> {
        self.rank(coord)
            .ok()
            .and_then(|rank| self.bound.raw().get(self.rows[rank].ordinal))
    }
    pub(super) fn rows_from(self, rank: usize) -> EffectiveRawRows<'a> {
        EffectiveRawRows {
            rows: self.rows[rank..].iter(),
            raw: self.bound.raw(),
        }
    }
    pub(super) fn range(self, start: i64, end: i64) -> EffectiveRawRows<'a> {
        let lo = self
            .rows
            .partition_point(|row| (row.bar.source_index as i64) < start);
        let hi = self
            .rows
            .partition_point(|row| (row.bar.source_index as i64) <= end)
            .max(lo);
        EffectiveRawRows {
            rows: self.rows[lo..hi].iter(),
            raw: self.bound.raw(),
        }
    }
}

pub(super) struct EffectiveRawRows<'a> {
    rows: std::slice::Iter<'a, DeltaRawRow>,
    raw: crate::v3::business_json::RawEventsView<'a>,
}
impl<'a> Iterator for EffectiveRawRows<'a> {
    type Item = (
        i64,
        newchan_rust::s_content::capture_allocation::audit_json::JsonView<'a>,
    );
    fn next(&mut self) -> Option<Self::Item> {
        self.rows.next().map(|row| {
            (
                row.bar.source_index as i64,
                self.raw
                    .get(row.ordinal)
                    .expect("DeltaLane 只保存已解析且绑定当前前缀的 ordinal"),
            )
        })
    }
}

/// slice 与同账 Delta rows 共用 CC007 的访问语义，不复制全 raw bars。
#[derive(Clone, Copy)]
pub(super) enum RawBars<'a> {
    Slice(&'a [Bar]),
    Delta(&'a [DeltaRawRow]),
}
impl<'a> RawBars<'a> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Slice(v) => v.len(),
            Self::Delta(v) => v.len(),
        }
    }
    pub(super) fn get(self, rank: usize) -> Option<&'a Bar> {
        match self {
            Self::Slice(v) => v.get(rank),
            Self::Delta(v) => v.get(rank).map(|r| &r.bar),
        }
    }
    pub(super) fn range(self, range: std::ops::Range<usize>) -> Self {
        match self {
            Self::Slice(v) => Self::Slice(&v[range]),
            Self::Delta(v) => Self::Delta(&v[range]),
        }
    }
    pub(super) fn iter(self) -> impl Iterator<Item = &'a Bar> {
        (0..self.len()).map(move |i| self.get(i).expect("范围内"))
    }
    pub(super) fn partition_point(self, mut predicate: impl FnMut(&Bar) -> bool) -> usize {
        let (mut left, mut right) = (0, self.len());
        while left < right {
            let mid = left + (right - left) / 2;
            if predicate(self.get(mid).expect("范围内")) {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        left
    }
}

/// 同核变化，不能单独充任 raw/事务权限；所有 rank 均指真实坐标升序生效账。
#[derive(Debug, Clone, Copy)]
pub(super) struct ParserChanges {
    pub old_steps: usize,
    pub new_steps: usize,
    #[allow(dead_code, reason = "#1488 再合 #1608：8组合实测无调用方，保留不删")]
    pub old_groups: usize,
    pub new_groups: usize,
    pub group_dirty_from: Option<usize>,
    pub merged_dirty_from: Option<usize>,
    pub effective_dirty_from: Option<usize>,
    pub source_revision_from: Option<i64>,
    pub replayed_from: Option<usize>,
    pub parsed_bars: usize,
}

impl DeltaLane {
    /// #1481：只在冷认证内从已核发布前缀建立进程缓存；不签发原账或历史资格。
    #[cfg(all(feature = "s_incremental", unix))]
    pub(super) fn cold_seed(
        raw: bj::RawEventsView<'_>,
        prefix: usize,
        binding: crate::v3::audit_owner::RawAdvanceBinding,
        admission: &(impl crate::v3::audit_store::CodecRead + ?Sized),
        deadline: Option<Instant>,
        budget: &dyn crate::v3::audit_budget::Budget,
    ) -> Result<Self, String> {
        if !binding.is_unbound_cold() || prefix > raw.len() || raw.budget().is_some() {
            return Err("GenError：冷车道必须来自未绑定的已核发布前缀".into());
        }
        let mut winners = bj::RawPrefixIndex::new(raw);
        winners.advance_measured(prefix, admission, deadline, Some(budget))?;
        let mut lane = Self::new(None)?;
        if let Some(work) = admission.codec_work() {
            lane.rows
                .try_reserve_native(work, winners.ordered().count())
                .map_err(|e| e.to_string())?;
        } else {
            return Err("GenError：冷车道缺原生工作域".into());
        }
        for (_, ordinal) in winners.ordered() {
            crate::v3::check_deadline(deadline)?;
            budget.precheck().map_err(|e| e.to_string())?;
            let bar = bar_of(raw.get(ordinal).ok_or("GenError：冷winner原行缺失")?)?;
            let change = lane.state.append_with_changes(bar);
            if !change.shared_available() {
                return Err("GenError：冷winner非严格升序".into());
            }
            lane.rows
                .try_push_native(
                    DeltaRawRow { ordinal, bar },
                    admission.codec_work().ok_or("GenError：冷车道缺工作域")?,
                )
                .map_err(|e| e.to_string())?;
        }
        lane.prefix = prefix;
        lane.binding = Some(binding);
        Ok(lane)
    }

    #[cfg(all(feature = "s_incremental", unix))]
    pub(super) fn rebind_cold_owner(
        &mut self,
        old: crate::v3::audit_owner::RawAdvanceBinding,
        current: crate::v3::audit_owner::RawAdvanceBinding,
        prefix: usize,
    ) -> Result<(), String> {
        if !old.is_unbound_cold()
            || current.is_unbound_cold()
            || !current.is_initial()
            || self.binding != Some(old)
            || self.prefix != prefix
            || self.poisoned
            || self.view_qualified
            || self.unqualified_history
        {
            return Err("GenError：冷车道交接域或前沿不符".into());
        }
        self.binding = Some(current);
        Ok(())
    }

    pub(super) fn new(
        budget: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
    ) -> Result<Self, String> {
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        if budget.is_some() {
            return Err("prototype_audit_allocation_incomplete：delta lane 缺 Owned 构建".into());
        }
        Ok(Self {
            rows: newchan_rust::s_content::CodecVec::new_with_allocation(budget),
            state: IncrFactsState::empty(),
            facts: tb02a_facts::FactCache::new(),
            binding: None,
            prefix: 0,
            poisoned: false,
            last_changes: None,
            previous_len: 0,
            view_qualified: false,
            unqualified_history: false,
        })
    }

    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub(super) fn state(&self) -> &IncrFactsState {
        &self.state
    }

    pub(super) fn rows(&self) -> &[DeltaRawRow] {
        &self.rows
    }

    pub(super) fn effective_view<'a>(
        &'a self,
        bound: &'a crate::v3::audit_owner::BoundRawAdvance<'_>,
    ) -> Result<EffectiveRawView<'a>, String> {
        bound.ensure_current()?;
        if self.poisoned
            || !self.view_qualified
            || self.binding != Some(bound.current_binding())
            || self.prefix != bound.current_prefix()
        {
            bound.invalidate_after_consumer_failure();
            return Err("GenError：effective view 不属于当前成功推进的车道".into());
        }
        Ok(EffectiveRawView {
            rows: &self.rows,
            state: &self.state,
            bound,
            changes: self
                .last_changes
                .ok_or_else(|| "GenError：effective view 缺推进记录".to_string())?,
            previous_len: self.previous_len,
        })
    }

    /// 可签借用视图的唯一推进口：bar 只从当前 bound 的真实原行按原配方解析。
    /// 通用 parse 回调口仍供 A1 测试/内部工作描述使用，但不能取得 prepared 资格。
    #[cfg(test)]
    pub(super) fn advance_owner_bound(
        &mut self,
        bound: &crate::v3::audit_owner::BoundRawAdvance<'_>,
    ) -> Result<ParserChanges, String> {
        self.advance_owner_bound_with_work(bound, None)
    }

    pub(super) fn advance_owner_bound_with_work(
        &mut self,
        bound: &crate::v3::audit_owner::BoundRawAdvance<'_>,
        work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    ) -> Result<ParserChanges, String> {
        if self.unqualified_history {
            bound.invalidate_after_consumer_failure();
            self.view_qualified = false;
            self.binding = None;
            self.poisoned = true;
            return Err("GenError：通用解析历史不可升格为 owner 原账资格，须新建车道".into());
        }
        let raw = bound.raw();
        let changes = self.advance_bound_checked(
            bound,
            |ordinal| {
                let event = raw
                    .get(ordinal)
                    .ok_or_else(|| "GenError：新 winner 原账缺失".to_string())?;
                bar_of(event)
            },
            work,
        )?;
        self.view_qualified = true;
        Ok(changes)
    }

    /// parse 只收到 owner 提供的最终新 winner；旧 bars 直接借本车道，不重新解码/建 fps。
    /// 调用即永久撤销此实例的 owner 资格；历史须通过新建车道从原件重新解析。
    /// 任一错误丢弃 pending 车道的资格，调用者必须丢弃整个实例，不能失败后转旧 advance。
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub(super) fn advance_bound(
        &mut self,
        bound: &crate::v3::audit_owner::BoundRawAdvance<'_>,
        parse: impl FnMut(usize) -> Result<Bar, String>,
    ) -> Result<ParserChanges, String> {
        self.unqualified_history = true;
        self.advance_bound_checked(bound, parse, None)
    }

    fn advance_bound_checked(
        &mut self,
        bound: &crate::v3::audit_owner::BoundRawAdvance<'_>,
        mut parse: impl FnMut(usize) -> Result<Bar, String>,
        work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    ) -> Result<ParserChanges, String> {
        self.view_qualified = false;
        // 旧 Plain 车道保留原分配合同；只有冷阶段已绑定 Native 的 rows 需要活 work。
        let work = if self.rows.is_native_bound() {
            work
        } else {
            None
        };
        let result = self.advance_bound_inner(bound, &mut parse, work);
        if result.is_err() {
            bound.invalidate_after_consumer_failure();
            self.binding = None;
            self.poisoned = true;
        }
        result
    }

    fn advance_bound_inner(
        &mut self,
        bound: &crate::v3::audit_owner::BoundRawAdvance<'_>,
        parse: &mut impl FnMut(usize) -> Result<Bar, String>,
        work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    ) -> Result<ParserChanges, String> {
        bound.ensure_current()?;
        if self.rows.is_native_bound() && work.is_none() {
            return Err("GenError：冷Native车道推进缺当前工作域".into());
        }
        if self.poisoned
            || self.prefix != bound.previous_prefix()
            || match self.binding {
                Some(previous) => previous != bound.previous_binding(),
                None => {
                    self.prefix != 0
                        || !self.rows.is_empty()
                        || !bound.previous_binding().is_initial()
                }
            }
        {
            return Err("GenError：delta lane 原账域或前代 binding 不匹配，须丢弃".into());
        }
        let raw = bound.raw();
        if !match (self.rows.allocation_budget(), raw.budget()) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_ledger(b),
            _ => false,
        } {
            return Err("prototype_audit_allocation_incomplete：delta lane 原账本改变".into());
        }
        let old_len = self.rows.len();
        let mut changes = ParserChanges {
            old_steps: self.state.step_count(),
            new_steps: self.state.step_count(),
            old_groups: self.state.group_count(),
            new_groups: self.state.group_count(),
            group_dirty_from: None,
            merged_dirty_from: None,
            effective_dirty_from: None,
            source_revision_from: None,
            replayed_from: None,
            parsed_bars: 0,
        };
        struct Edit {
            old: Option<crate::v3::audit_owner::RawWinner>,
            new: Option<DeltaRawRow>,
            same_coord: bool,
        }
        let mut edits = newchan_rust::s_content::CodecVec::new_with_allocation(raw.budget());
        edits
            .try_reserve_exact(bound.winner_edits().len())
            .map_err(|e| e.to_string())?;
        let mut numeric_dirty = None;
        let mut removed = 0usize;
        let mut added = 0usize;
        for edit in bound.winner_edits() {
            let previous = match edit.old {
                Some(old) => {
                    let rank = self
                        .rows
                        .binary_search_by_key(&old.coord, |row| row.bar.source_index as i64)
                        .map_err(|_| "GenError：delta lane 旧 winner 坐标缺失".to_string())?;
                    let row = self.rows[rank];
                    if row.ordinal != old.ordinal {
                        return Err("GenError：delta lane 旧 winner ordinal 不匹配".into());
                    }
                    changes.effective_dirty_from =
                        min_frontier(changes.effective_dirty_from, Some(rank));
                    changes.source_revision_from =
                        min_source(changes.source_revision_from, old.coord);
                    removed += 1;
                    Some((rank, row))
                }
                None => None,
            };
            let next = match edit.new {
                Some(new) => {
                    let bar = parse(new.ordinal)?;
                    if i64::try_from(bar.source_index).ok() != Some(new.coord) {
                        return Err("GenError：delta lane 新 winner 坐标与真实 bar 不符".into());
                    }
                    let rank = self
                        .rows
                        .binary_search_by_key(&new.coord, |row| row.bar.source_index as i64)
                        .unwrap_or_else(|rank| rank);
                    changes.effective_dirty_from =
                        min_frontier(changes.effective_dirty_from, Some(rank));
                    if rank < old_len {
                        changes.source_revision_from =
                            min_source(changes.source_revision_from, new.coord);
                    }
                    added += 1;
                    Some((
                        rank,
                        DeltaRawRow {
                            ordinal: new.ordinal,
                            bar,
                        },
                    ))
                }
                None => None,
            };
            if !matches!((previous, next), (Some((_, a)), Some((_, b))) if InputFp::of(&a.bar) == InputFp::of(&b.bar))
            {
                numeric_dirty = min_frontier(numeric_dirty, previous.map(|(rank, _)| rank));
                numeric_dirty = min_frontier(numeric_dirty, next.map(|(rank, _)| rank));
            }
            let same_coord =
                matches!((edit.old, edit.new), (Some(old), Some(new)) if old.coord == new.coord);
            edits
                .try_push(Edit {
                    old: edit.old,
                    new: next.map(|(_, row)| row),
                    same_coord,
                })
                .map_err(|e| e.to_string())?;
        }
        let new_len = old_len
            .checked_sub(removed)
            .and_then(|n| n.checked_add(added))
            .ok_or_else(|| "GenError：delta lane winner 数量不一致".to_string())?;
        let additional = new_len.saturating_sub(old_len);
        match work {
            Some(work) => self.rows.try_reserve_native(work, additional),
            None => self.rows.try_reserve_exact(additional),
        }
        .map_err(|e| e.to_string())?;
        // 坐标变化的旧 winner 先全部移除；不能按 identity 边删边加而误拒合法坐标互换。
        for edit in edits.iter().filter(|edit| !edit.same_coord) {
            if let Some(old) = edit.old {
                let rank = self
                    .rows
                    .binary_search_by_key(&old.coord, |row| row.bar.source_index as i64)
                    .map_err(|_| "GenError：delta lane 重复移除旧 winner".to_string())?;
                self.rows.remove(rank);
            }
        }
        for edit in edits.iter() {
            if let Some(new) = edit.new {
                let rank = self
                    .rows
                    .binary_search_by_key(&(new.bar.source_index as i64), |row| {
                        row.bar.source_index as i64
                    });
                if edit.same_coord {
                    self.rows[rank
                        .map_err(|_| "GenError：delta lane 原位 winner 缺失".to_string())?] = new;
                } else {
                    let rank = rank.map_or_else(Ok, |_| {
                        Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".to_string())
                    })?;
                    match work {
                        Some(work) => self.rows.try_push_native(new, work),
                        None => self.rows.try_push(new),
                    }
                    .map_err(|e| e.to_string())?;
                    self.rows[rank..].rotate_right(1);
                }
            }
        }
        if let Some(from) = changes.source_revision_from {
            // 即使 InputFp 完全相同，真实 seq/revision 更替也撤销来源事实缓存。
            self.facts.invalidate_from(from);
        }
        if let Some(rank) = numeric_dirty {
            let rank = rank.min(changes.old_steps).min(self.rows.len());
            if rank < changes.old_steps {
                self.state
                    .rewind_to(rank)
                    .map_err(|e| format!("GenError：delta parser 回退失败：{}", e.reason))?;
                changes.replayed_from = Some(rank);
                changes.group_dirty_from = Some(self.state.group_count().saturating_sub(1));
                changes.merged_dirty_from = Some(self.state.merged_slice().len().saturating_sub(1));
            }
            for row in &self.rows[rank..] {
                let appended = self.state.append_with_changes(row.bar);
                if !appended.shared_available() {
                    return Err("GenError：有效 raw winner 非严格升序，不能沿用共享 parser".into());
                }
                changes.group_dirty_from =
                    min_frontier(changes.group_dirty_from, appended.group_dirty_from());
                changes.merged_dirty_from =
                    min_frontier(changes.merged_dirty_from, appended.merged_dirty_from());
                changes.parsed_bars += 1;
            }
        }
        changes.new_steps = self.state.step_count();
        changes.new_groups = self.state.group_count();
        self.prefix = bound.current_prefix();
        self.binding = Some(bound.current_binding());
        self.previous_len = old_len;
        self.last_changes = Some(changes);
        Ok(changes)
    }
}

fn min_frontier(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn min_source(previous: Option<i64>, coord: i64) -> Option<i64> {
    Some(previous.map_or(coord, |old| old.min(coord)))
}

#[cfg(test)]
mod delta_tests {
    use super::*;
    use crate::v3::audit_owner::{RawAdvanceFixture, RawWinner, RawWinnerEdit};
    use crate::v3::business_json::{RawEventsView, RawPrefixFixture as RawPrefixIndex};
    use newchan_rust::theta_v0::parser::inclusion::process_inclusion_with_facts;

    fn fixture() -> (Vec<Value>, Vec<Bar>) {
        let rows = [
            (2, 10, 5, 1),
            (7, 12, 7, 1),
            (12, 11, 8, 1),
            (7, 12, 7, 2),
            (12, 14, 9, 2),
        ];
        let mut events = Vec::new();
        let mut bars = Vec::new();
        for (ordinal, (coord, high, low, revision)) in rows.into_iter().enumerate() {
            events.push(serde_json::json!({
                "identity_key": format!("id-{coord}"), "source_coord": coord.to_string(),
                "revision": revision, "seq": ordinal + 1, "ts": coord.to_string(),
                "open": low.to_string(), "close": high.to_string(), "high": high.to_string(), "low": low.to_string(),
                "input_revision": revision.to_string(), "payload_hash": format!("hash-{ordinal}"),
                "receipt_id": format!("receipt-{ordinal}"), "event_id": format!("event-{coord}"),
            }));
            bars.push(Bar {
                source_index: coord,
                timestamp: coord as i64,
                open: low,
                high,
                low,
                close: high,
                volume: 1.0,
                untradable: false,
            });
        }
        (events, bars)
    }

    #[test]
    fn delta_effective_view_rank_range_and_parser_are_same_bound_without_projection() {
        let (events, bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new((&events).into());
        let mut lane = DeltaLane::new(None).unwrap();
        let edits = index.advance(3).unwrap();
        let bound = owner
            .bind((&events[..3]).into(), 0, &edits.winner_edits)
            .unwrap();
        lane.advance_owner_bound(&bound).unwrap();
        let view = lane.effective_view(&bound).unwrap();
        assert!(std::ptr::eq(view.state(), lane.state()));
        assert_eq!(view.previous_len(), 0);
        assert_eq!(view.dirty_from(), 0);
        assert_eq!(
            view.rows_from(1).map(|(c, _)| c).collect::<Vec<_>>(),
            vec![7, 12]
        );
        assert_eq!(
            view.range(3, 11).map(|(c, _)| c).collect::<Vec<_>>(),
            vec![7]
        );
        assert!(view.range(20, 5).next().is_none());
        assert_eq!(view.get(7).unwrap().get("seq").unwrap().as_i64(), Some(2));
        assert!(view.get(8).is_none());
        assert_eq!(
            view.bars().range(1..3).iter().copied().collect::<Vec<_>>(),
            bars[1..3]
        );
        owner.invalidate();
        assert!(view.ensure_current().is_err());
    }

    #[test]
    fn delta_effective_view_rejects_self_reported_parse_even_with_valid_raw_binding() {
        let (events, mut bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new((&events).into());
        let edits = index.advance(3).unwrap();
        let bound = owner
            .bind((&events[..3]).into(), 0, &edits.winner_edits)
            .unwrap();
        bars[1].high += 50; // 坐标相同仍不能把调用者的 OHLC 自报签成 prepared 资格。
        let mut lane = DeltaLane::new(None).unwrap();
        lane.advance_bound(&bound, |i| Ok(bars[i])).unwrap();
        assert!(lane.effective_view(&bound).is_err());
        assert!(bound.ensure_current().is_err());
    }

    #[test]
    fn delta_owner_rejects_generic_history_on_later_no_edit_generation_without_intermediate_view() {
        let (events, mut bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new((&events).into());
        let first = index.advance(1).unwrap();
        let bound = owner
            .bind((&events[..1]).into(), 0, &first.winner_edits)
            .unwrap();
        bars[0].high += 50;
        let mut lane = DeltaLane::new(None).unwrap();
        lane.advance_bound(&bound, |i| Ok(bars[i])).unwrap();
        assert_eq!(lane.rows()[0].bar.high, bars[0].high);
        // 故意不在通用推进后取 view：下一次合法 token 本身不能洗掉旧来源污染。
        let unchanged = index.advance(1).unwrap();
        assert!(unchanged.winner_edits.is_empty());
        let next = owner
            .bind((&events[..1]).into(), 1, &unchanged.winner_edits)
            .unwrap();
        assert!(next.ensure_current().is_ok());
        let error = lane.advance_owner_bound(&next).unwrap_err();
        assert!(error.contains("通用解析历史不可升格"));
        assert!(next.ensure_current().is_err());
        assert!(lane.effective_view(&next).is_err());
        // 只有显式新建车道并重新从 initial 真原件解析才恢复；旧实例仍不可升格。
        let fresh_owner = RawAdvanceFixture::new();
        let fresh = fresh_owner
            .bind((&events[..1]).into(), 0, &first.winner_edits)
            .unwrap();
        let mut rebuilt = DeltaLane::new(None).unwrap();
        rebuilt.advance_owner_bound(&fresh).unwrap();
        assert_eq!(
            rebuilt
                .effective_view(&fresh)
                .unwrap()
                .bars()
                .get(0)
                .unwrap()
                .high,
            10
        );
        assert!(lane.unqualified_history && lane.poisoned);
    }

    #[test]
    fn delta_owner_rejects_intervening_generic_revision_before_other_winner_append() {
        let (mut events, mut bars) = fixture();
        events.truncate(4);
        bars.truncate(4);
        let appended = serde_json::json!({
            "identity_key":"id-17", "source_coord":"17", "revision":1, "seq":5,
            "ts":"17", "open":"12", "high":"16", "low":"12", "close":"16",
            "input_revision":"1", "payload_hash":"hash-4", "receipt_id":"receipt-4", "event_id":"event-17",
        });
        bars.push(bar_of((&appended).into()).unwrap());
        events.push(appended);
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new((&events).into());
        let first = index.advance(3).unwrap();
        let bound = owner
            .bind((&events[..3]).into(), 0, &first.winner_edits)
            .unwrap();
        let mut lane = DeltaLane::new(None).unwrap();
        lane.advance_owner_bound(&bound).unwrap();
        assert!(lane.view_qualified);
        let revision = index.advance(4).unwrap();
        let revised = owner
            .bind((&events[..4]).into(), 3, &revision.winner_edits)
            .unwrap();
        bars[3].high += 50; // 真实修订的 coord=7 被通用回调污染；第三代仅追加 coord=17。
        lane.advance_bound(&revised, |i| Ok(bars[i])).unwrap();
        assert_eq!(lane.rows()[1].bar.high, bars[3].high);
        // 故意不消费中间 view，复现“合法 owner → 通用修订 → 合法追加”的跨代路径。
        let appended = index.advance(5).unwrap();
        assert_eq!(appended.winner_edits.len(), 1);
        assert_eq!(appended.winner_edits[0].new.unwrap().coord, 17);
        let next = owner
            .bind((&events[..5]).into(), 4, &appended.winner_edits)
            .unwrap();
        assert!(next.ensure_current().is_ok());
        assert!(lane
            .advance_owner_bound(&next)
            .unwrap_err()
            .contains("通用解析历史不可升格"));
        assert!(next.ensure_current().is_err());
        assert!(lane.effective_view(&next).is_err());
        assert_eq!(lane.rows().len(), 3, "拒绝发生在任何追加/parser 推进之前");
    }

    #[test]
    fn delta_prepared_rejects_equal_but_foreign_parser_and_untracked_source_epoch() {
        let (events, bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new((&events).into());
        let mut lane = DeltaLane::new(None).unwrap();
        let edits = index.advance(3).unwrap();
        let bound = owner
            .bind((&events[..3]).into(), 0, &edits.winner_edits)
            .unwrap();
        lane.advance_owner_bound(&bound).unwrap();
        let view = lane.effective_view(&bound).unwrap();
        let mut producer = content_producer::Cc004Producer::new(tb02a::PROFILE, "r");
        let mut store = newchan_rust::s_content::MemoryNodes::new();
        let prepared = producer
            .prepare_raw_history_delta(&mut store, view)
            .unwrap();
        producer
            .parser_fractal_sources(&prepared, lane.state())
            .unwrap();
        let mut other = IncrFactsState::empty();
        for bar in &bars[..3] {
            other.append(*bar);
        }
        assert_eq!(other.materialize_facts(), lane.state().materialize_facts());
        assert!(producer.parser_fractal_sources(&prepared, &other).is_err());
        let foreign = content_producer::Cc004Producer::new(tb02a::PROFILE, "r");
        assert!(foreign
            .parser_fractal_sources(&prepared, lane.state())
            .is_err());
        let spec = content_producer::RawRefSpec::from_event((&events[3]).into()).unwrap();
        producer.bind_source(&mut store, 7, spec).unwrap();
        assert!(producer
            .parser_fractal_sources(&prepared, lane.state())
            .is_err());
        assert!(producer.check_effective_view(view).is_err());
        owner.invalidate();
        assert!(producer
            .parser_fractal_sources(&prepared, lane.state())
            .is_err());
    }

    #[test]
    fn delta_lane_reads_only_changed_winners_and_replays_only_numeric_suffix() {
        let (events, bars) = fixture();
        let mut index = RawPrefixIndex::new(RawEventsView::plain(&events));
        let owner = RawAdvanceFixture::new();
        let mut lane = DeltaLane::new(None).unwrap();
        let mut previous = 0;
        for (prefix, expected_parsed) in [(2, 2), (3, 1), (4, 0), (5, 1)] {
            let edits = index.advance(prefix).unwrap();
            let bound = owner
                .bind(
                    RawEventsView::plain(&events[..prefix]),
                    previous,
                    &edits.winner_edits,
                )
                .unwrap();
            let mut decoded = 0;
            let changes = lane
                .advance_bound(&bound, |ordinal| {
                    assert!(ordinal >= previous, "追加/修订不能重新解码旧 raw");
                    decoded += 1;
                    Ok(bars[ordinal])
                })
                .unwrap();
            assert_eq!(decoded, edits.winner_edits.len());
            assert_eq!(changes.parsed_bars, expected_parsed);
            let effective: Vec<_> = index.ordered().map(|(_, ordinal)| bars[ordinal]).collect();
            assert_eq!(
                lane.state.materialize_facts(),
                process_inclusion_with_facts(&effective)
            );
            if prefix == 4 {
                assert_eq!(changes.source_revision_from, Some(7));
                assert_eq!(changes.replayed_from, None);
            }
            if prefix == 5 {
                assert_eq!(changes.replayed_from, Some(2));
            }
            previous = prefix;
        }
    }

    #[test]
    fn delta_lane_same_ohlc_revision_invalidates_real_fact_cache() {
        let (events, bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new(RawEventsView::plain(&events));
        let mut lane = DeltaLane::new(None).unwrap();
        let first = index.advance(3).unwrap();
        let bound = owner
            .bind(RawEventsView::plain(&events[..3]), 0, &first.winner_edits)
            .unwrap();
        lane.advance_bound(&bound, |ordinal| Ok(bars[ordinal]))
            .unwrap();
        let mut raw: BTreeMap<i64, Value> = index
            .ordered()
            .map(|(coord, ordinal)| (coord, events[ordinal].clone()))
            .collect();
        let groups = lane.state.materialize_groups();
        tb02a_facts::build_incremental(
            &lane.state,
            &groups,
            &raw,
            &[],
            1,
            "c1",
            3,
            &mut lane.facts,
        );
        tb02a_facts::build_incremental(
            &lane.state,
            &groups,
            &raw,
            &[],
            1,
            "c1",
            3,
            &mut lane.facts,
        );
        assert!(lane.facts.hits() > 0, "负控前必须真命中缓存");
        let misses = lane.facts.misses();
        let edits = index.advance(4).unwrap();
        let revision = owner
            .bind(RawEventsView::plain(&events[..4]), 3, &edits.winner_edits)
            .unwrap();
        let changes = lane
            .advance_bound(&revision, |ordinal| Ok(bars[ordinal]))
            .unwrap();
        assert_eq!(changes.parsed_bars, 0);
        raw.insert(7, events[3].clone());
        let groups = lane.state.materialize_groups();
        let got = tb02a_facts::build_incremental(
            &lane.state,
            &groups,
            &raw,
            &[],
            2,
            "c2",
            4,
            &mut lane.facts,
        );
        let want = tb02a_facts::build(
            &process_inclusion_with_facts(&bars[..3]),
            &raw,
            &[],
            2,
            "c2",
            4,
        );
        assert_eq!(got, want);
        assert!(
            lane.facts.misses() > misses,
            "来源版本变了必须真正撤销旧事实"
        );
    }

    #[test]
    fn delta_lane_mismatched_old_ordinal_poison_has_no_complete_fallback() {
        let (events, bars) = fixture();
        let owner = RawAdvanceFixture::new();
        let mut index = RawPrefixIndex::new(RawEventsView::plain(&events));
        let first = index.advance(3).unwrap();
        let mut lane = DeltaLane::new(None).unwrap();
        let bound = owner
            .bind(RawEventsView::plain(&events[..3]), 0, &first.winner_edits)
            .unwrap();
        lane.advance_bound(&bound, |ordinal| Ok(bars[ordinal]))
            .unwrap();
        // 只在 cfg(test) 故意伪造工作描述；生产 owner 不应签发这种变化。
        let wrong = [RawWinnerEdit {
            old: Some(RawWinner {
                coord: 7,
                ordinal: 0,
            }),
            new: Some(RawWinner {
                coord: 7,
                ordinal: 3,
            }),
        }];
        let bound = owner
            .bind(RawEventsView::plain(&events[..4]), 3, &wrong)
            .unwrap();
        let error = lane
            .advance_bound(&bound, |_| panic!("错旧身份须先拒绝，不能解码或走全量"))
            .unwrap_err();
        assert!(error.contains("ordinal"));
        assert!(bound.ensure_current().is_err());
        assert!(lane.binding.is_none());
        assert!(lane.poisoned);
    }
}

/// 测量/测试面：清空本线程车道。
#[cfg(test)]
pub(super) fn warm_lane_reset() {
    LANE.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

/// 测量/测试面：把本线程车道清成冷启动（保留键，清空状态与缓存）。
#[cfg(test)]
pub(super) fn cold_lane() {
    LANE.with(|slot| {
        if let Some(lane) = slot.borrow_mut().as_mut() {
            lane.clear_reuse();
        }
    });
}

/// 测量面：当前车道累计计数 (命中, 未命中, 单元实算, 单元复用)。
#[cfg(test)]
pub(super) fn lane_counts() -> (usize, usize, usize, usize) {
    LANE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(Lane::counts)
            .unwrap_or((0, 0, 0, 0))
    })
}

thread_local! {
    /// 线程内唯一车道槽。
    ///
    /// 为什么不是全局 `Mutex`：车道内的完整证据节点用 `Rc` 共享（单线程性能结构），不是
    /// `Send`。S 的写路径本身是单写者：serve 模式把**全部请求**派发在同一条 dispatcher
    /// 线程上（`session_transport::serve_unix` 的 handler 循环），一次性 CLI 也各占一条线程；
    /// 因此「线程内复用」等价于「写者内复用」，且不需要把 Rc 跨线程搬运。
    pub(super) static LANE: RefCell<Option<Lane>> = const { RefCell::new(None) };
}

/// 车道句柄：取用 → （可能）推进 → Drop 时归还。身份不符则旧车道作废。
pub(super) struct LaneGuard {
    lane: Option<Lane>,
}

impl LaneGuard {
    pub(super) fn take(key: &str) -> Self {
        let lane = LANE.with(|slot| {
            let mut slot = slot.borrow_mut();
            match slot.take() {
                Some(lane) if lane.key == key => Some(lane),
                _ => None,
            }
        });
        LaneGuard { lane }
    }

    /// 取得（必要时新建）本键车道。
    pub(super) fn lane(&mut self, key: &str) -> &mut Lane {
        if self.lane.is_none() {
            self.lane = Some(Lane::new(key));
        }
        self.lane.as_mut().expect("刚建立")
    }
}

impl Drop for LaneGuard {
    fn drop(&mut self) {
        if let Some(lane) = self.lane.take() {
            LANE.with(|slot| {
                *slot.borrow_mut() = Some(lane);
            });
        }
    }
}

/// 车道键：持久身份（不跨库/跨规则/跨档案复用）。
pub(super) fn lane_key(
    db: &Path,
    session_id: &str,
    session_generation: &str,
    rule_revision: &str,
    profile_id: &str,
    catalog_revision: &str,
) -> String {
    format!(
        "{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
        db.display(),
        session_id,
        session_generation,
        rule_revision,
        profile_id,
        catalog_revision
    )
}

#[cfg(test)]
thread_local! {
    /// 测试专用：强制走全量 oracle 路径（用于「增量 vs 全量」逐代 batch 对拍）。
    static ORACLE: RefCell<bool> = const { RefCell::new(false) };
}

/// 测试专用开关：`true` = 本线程内 `advance_core` 走全量 oracle（生产构建无此分支）。
#[cfg(test)]
pub(super) fn set_oracle(enabled: bool) {
    ORACLE.with(|slot| *slot.borrow_mut() = enabled);
}

pub(super) fn oracle_enabled() -> bool {
    #[cfg(test)]
    {
        return ORACLE.with(|slot| *slot.borrow());
    }
    #[cfg(not(test))]
    false
}

/// #1394 测量面（内部）：阶段 wall time 只在测试显式开启时累计，不进公开输出。
pub(super) mod phase {
    use super::{AtomicBool, Instant, Ordering};
    use std::cell::RefCell;

    static ENABLED: AtomicBool = AtomicBool::new(false);
    thread_local! {
        static TOTALS: RefCell<Vec<(&'static str, u128)>> = const { RefCell::new(Vec::new()) };
    }

    #[cfg(test)]
    pub(crate) fn enable(enabled: bool) {
        ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub(crate) fn record(name: &'static str, nanos: u128) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        TOTALS.with(|totals| totals.borrow_mut().push((name, nanos)));
    }

    pub(crate) fn begin() -> Instant {
        Instant::now()
    }

    #[cfg(test)]
    pub(crate) fn reset() {
        TOTALS.with(|totals| totals.borrow_mut().clear());
    }

    #[cfg(test)]
    pub(crate) fn totals() -> Vec<(&'static str, u128)> {
        TOTALS.with(|totals| totals.borrow().clone())
    }

    /// 按阶段名汇总（保持首次出现顺序）。
    #[cfg(test)]
    pub(crate) fn summary() -> Vec<(&'static str, u128, usize)> {
        let mut out: Vec<(&'static str, u128, usize)> = Vec::new();
        for (name, nanos) in totals() {
            match out.iter_mut().find(|(existing, _, _)| *existing == name) {
                Some((_, total, count)) => {
                    *total += nanos;
                    *count += 1;
                }
                None => out.push((name, nanos, 1)),
            }
        }
        out
    }
}
