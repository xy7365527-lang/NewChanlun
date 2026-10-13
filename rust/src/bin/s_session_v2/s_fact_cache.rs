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
use newchan_rust::theta_v0::parser::inclusion::{InclusionGroupFact, IncrFactsState};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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
