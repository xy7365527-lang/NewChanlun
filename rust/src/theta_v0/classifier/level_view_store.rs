//! C2 CompletedFreeze event-store adapter（task #75）。
//!
//! JSONL 是正式持久化边界：只追加，不覆盖；缓存键包含 D1/D2/D3 三 provider 版本。reopen 先把
//! legacy v0 行迁移为 v1，再经同一 reducer 重建冻结态。重复重放相同事件是幂等的，冲突事件拒绝。

use super::super::types::{Direction, MoveKind, Tick};
use super::level_view::{
    AssembledMove, C2CacheKey, C2PersistenceKey, CompletionStatus, ConfirmKey, LevelAsOfView,
    LevelViewQuery,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// #69 5a：趋势确认的已证状态。`TerminalFalse` 仅表示单调力度关系已经终假；
/// 结构或坐标仍不可验时必须保持 `Scanning`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmState {
    Confirmed(usize),
    TerminalFalse,
    Scanning,
}

impl ConfirmState {
    pub fn as_option(self) -> Option<usize> {
        match self {
            Self::Confirmed(t) => Some(t),
            Self::TerminalFalse | Self::Scanning => None,
        }
    }
}

/// #69 5a：单个 divergence pair 在已封 lower-leg 前缀上的扫描累积。
///
/// 字段 `pub(super)`（非 `pub`）：跨 `level_view.rs`/`level_view_confirm.rs`/`level_view::tests`
/// （`classifier` 子树内兄弟模块）访问，同时不对 crate 外及 `classifier` 之外暴露内部字段
/// （#497 归位；#630 影子评审 MEDIUM-2 后继评估：确认核心 `trend_confirm_state_core`/
/// `scan_confirm_cursor` 拆分至 `level_view_confirm.rs` 后与本类型分处两个 sibling 文件，
/// 两者最近公共祖先仍是 `classifier`，`pub(super)` 已是此分解下的最小可见性——newtype 包壳/
/// 字段私有化+方法面三条收紧路径均要求把确认核心与本类型重新并回同一文件，超出本票范围，
/// 留作后继票；当前处置 = **约定等级声明 + 不变量测试钉死**，而非编译期保证）。
#[derive(Debug, Clone)]
pub struct ConfirmCursor {
    /// 下一个尚未消费的 lower-leg 下标；只允许落在确认水线内——此不变量现由约定维持
    /// （非编译期私有字段保证），由
    /// `level_view::tests::confirm::confirm_cursor_incremental_matches_cold_at_every_boundary`
    /// （`cursor.k0 <= confirmed_len` 逐步骤钉死 + 单调不倒退）与
    /// `level_view::tests::confirm::confirm_store_resets_on_watermark_rollback_and_structure_change`
    /// （水线回退重建）两条测试固定。
    pub(super) k0: usize,
    pub(super) env: Option<(Tick, Tick)>,
    pub(super) acc_hi: Option<usize>,
    pub(super) area_c: f64,
    pub(super) dif_max: f64,
    pub(super) dif_min: f64,
    pub(super) hist_max: f64,
    pub(super) hist_min: f64,
    pub(super) state: ConfirmState,
}

impl Default for ConfirmCursor {
    fn default() -> Self {
        Self {
            k0: 0,
            env: None,
            acc_hi: None,
            area_c: 0.0,
            dif_max: f64::NEG_INFINITY,
            dif_min: f64::INFINITY,
            hist_max: f64::NEG_INFINITY,
            hist_min: f64::INFINITY,
            state: ConfirmState::Scanning,
        }
    }
}

/// #69 5a：per-level divergence-pair cursor 映射；实际持有者在 bin `LevelDerived`。
#[derive(Debug, Default)]
pub struct ConfirmCursorStore {
    pub(super) cursors: HashMap<ConfirmKey, ConfirmCursor>,
}

impl ConfirmCursorStore {
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// 上层 run 分区变化时删除已消失 run，避免陈旧 key 永久驻留。
    pub fn retain_run_starts(&mut self, level: u32, run_starts: impl IntoIterator<Item = usize>) {
        let run_starts: BTreeSet<_> = run_starts.into_iter().collect();
        self.cursors
            .retain(|key, _| key.level != level || run_starts.contains(&key.run_window.start));
    }

    pub(super) fn retain_active_for_run(
        &mut self,
        level: u32,
        run_start: usize,
        active: &[ConfirmKey],
    ) {
        self.cursors.retain(|key, _| {
            key.level != level
                || key.run_window.start != run_start
                || active.iter().any(|candidate| candidate == key)
        });
    }

    pub(super) fn cursor_mut(&mut self, key: ConfirmKey) -> &mut ConfirmCursor {
        self.cursors.entry(key).or_default()
    }

    #[cfg(test)]
    pub(super) fn poison_for_test(&mut self, state: ConfirmState) {
        for cursor in self.cursors.values_mut() {
            cursor.state = state;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompletedMoveId {
    pub level: u32,
    pub start_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenCompletedMove {
    pub id: CompletedMoveId,
    pub judge_at: usize,
    pub entry_bar: Option<usize>,
    pub end_index: usize,
    pub direction: Option<Direction>,
    pub kind: MoveKind,
    pub center_indices: Vec<usize>,
}

impl FrozenCompletedMove {
    fn from_move(level: u32, as_of: usize, value: &AssembledMove) -> Option<Self> {
        matches!(value.completion, CompletionStatus::Completed { .. }).then(|| Self {
            id: CompletedMoveId {
                level,
                start_index: value.start_index,
            },
            judge_at: as_of,
            entry_bar: as_of.checked_add(1),
            end_index: value.end_index,
            direction: value.direction,
            kind: value.kind,
            center_indices: value.center_indices.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedFreezeEvent {
    pub sequence: u64,
    pub created_at: usize,
    pub cache_key: String,
    pub snapshot: FrozenCompletedMove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Io(String),
    InvalidJson { line: usize, message: String },
    UnsupportedSchema { line: usize, schema: u64 },
    InvalidEnum { field: &'static str, value: String },
    CacheKeyMismatch { expected: String, actual: String },
    SequenceConflict(u64),
    NonAppendSequence { expected: u64, actual: u64 },
    CompletedFreezeViolation(CompletedMoveId),
    QueryMismatch,
    Version(String),
}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        StoreError::Io(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct JsonlCompletedEventStore {
    path: PathBuf,
}

impl JsonlCompletedEventStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 单次写入只使用 append 模式；从不 truncate/seek/重写旧字节。
    pub fn append(&self, event: &CompletedFreezeEvent) -> Result<(), StoreError> {
        let wire = WireEventV1::from_event(event);
        let line =
            serde_json::to_string(&wire).map_err(|error| StoreError::Io(error.to_string()))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let record = format!("{line}\n");
        file.write_all(record.as_bytes())?;
        file.sync_data()?;
        Ok(())
    }

    /// reopen/migration reader。v0 没有 schema/cache_key/entry_bar，由调用方 pin 的当前 cache key
    /// 确定性补齐；v1 必须逐行自带并匹配该键。
    pub fn load(
        &self,
        expected_key: &C2PersistenceKey,
    ) -> Result<Vec<CompletedFreezeEvent>, StoreError> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut events = Vec::new();
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            let line_number = line_index + 1;
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(&line).map_err(|error| StoreError::InvalidJson {
                    line: line_number,
                    message: error.to_string(),
                })?;
            let event = match value.get("schema_version").and_then(|value| value.as_u64()) {
                None | Some(0) => {
                    let old: WireEventV0 =
                        serde_json::from_value(value).map_err(|error| StoreError::InvalidJson {
                            line: line_number,
                            message: error.to_string(),
                        })?;
                    old.migrate(expected_key)?
                }
                Some(1) => {
                    let wire: WireEventV1 =
                        serde_json::from_value(value).map_err(|error| StoreError::InvalidJson {
                            line: line_number,
                            message: error.to_string(),
                        })?;
                    wire.into_event()?
                }
                Some(schema) => {
                    return Err(StoreError::UnsupportedSchema {
                        line: line_number,
                        schema,
                    })
                }
            };
            if event.cache_key != expected_key.as_str() {
                return Err(StoreError::CacheKeyMismatch {
                    expected: expected_key.as_str().to_owned(),
                    actual: event.cache_key,
                });
            }
            events.push(event);
        }
        Ok(events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletedFreezeReducer {
    cache_key: String,
    events: BTreeMap<u64, CompletedFreezeEvent>,
    frozen: BTreeMap<CompletedMoveId, FrozenCompletedMove>,
}

impl CompletedFreezeReducer {
    pub fn new(cache_key: &C2PersistenceKey) -> Self {
        Self {
            cache_key: cache_key.as_str().to_owned(),
            ..Self::default()
        }
    }

    pub fn replay<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a CompletedFreezeEvent>,
    ) -> Result<(), StoreError> {
        for event in events {
            self.apply(event.clone())?;
        }
        Ok(())
    }

    pub fn apply(&mut self, event: CompletedFreezeEvent) -> Result<(), StoreError> {
        if event.cache_key != self.cache_key {
            return Err(StoreError::CacheKeyMismatch {
                expected: self.cache_key.clone(),
                actual: event.cache_key,
            });
        }
        if let Some(existing) = self.events.get(&event.sequence) {
            return if existing == &event {
                Ok(())
            } else {
                Err(StoreError::SequenceConflict(event.sequence))
            };
        }
        let expected = self.events.len() as u64;
        if event.sequence != expected {
            return Err(StoreError::NonAppendSequence {
                expected,
                actual: event.sequence,
            });
        }
        if let Some(existing) = self.frozen.get(&event.snapshot.id) {
            if existing != &event.snapshot {
                return Err(StoreError::CompletedFreezeViolation(event.snapshot.id));
            }
        } else {
            self.frozen
                .insert(event.snapshot.id.clone(), event.snapshot.clone());
        }
        self.events.insert(event.sequence, event);
        Ok(())
    }

    pub fn frozen(&self) -> &BTreeMap<CompletedMoveId, FrozenCompletedMove> {
        &self.frozen
    }

    pub fn events(&self) -> &BTreeMap<u64, CompletedFreezeEvent> {
        &self.events
    }

    fn next_sequence(&self) -> u64 {
        self.events.len() as u64
    }
}

/// seam → 正式 store 的事务边界：先在 clone reducer 上验证，写入成功后才提交内存态。
#[derive(Debug, Clone)]
pub struct CompletedFreezeAdapter {
    store: JsonlCompletedEventStore,
    reducer: CompletedFreezeReducer,
    query: LevelViewQuery,
}

impl CompletedFreezeAdapter {
    pub fn new(store: JsonlCompletedEventStore, query: LevelViewQuery) -> Result<Self, StoreError> {
        Self::reopen(store, query)
    }

    pub fn reopen(
        store: JsonlCompletedEventStore,
        query: LevelViewQuery,
    ) -> Result<Self, StoreError> {
        let key = C2PersistenceKey::from_query(&query)
            .map_err(|error| StoreError::Version(format!("{error:?}")))?;
        let events = store.load(&key)?;
        let mut reducer = CompletedFreezeReducer::new(&key);
        reducer.replay(&events)?;
        Ok(Self {
            store,
            reducer,
            query,
        })
    }

    pub fn reducer(&self) -> &CompletedFreezeReducer {
        &self.reducer
    }

    pub fn observe(&mut self, view: &LevelAsOfView) -> Result<usize, StoreError> {
        if view.query.level != self.query.level
            || view.query.coordinate_window != self.query.coordinate_window
            || view.query.version != self.query.version
        {
            return Err(StoreError::QueryMismatch);
        }
        let expected_view_key = C2CacheKey::from_query(&view.query)
            .map_err(|error| StoreError::Version(format!("{error:?}")))?;
        if view.cache_key != expected_view_key {
            return Err(StoreError::QueryMismatch);
        }

        // 所有旧 Completed 必须仍存在且逐字段相同；先全验，失败时零写入。
        for (id, frozen) in self.reducer.frozen() {
            let Some(now) = view
                .moves
                .iter()
                .find(|value| value.start_index == id.start_index)
            else {
                return Err(StoreError::CompletedFreezeViolation(id.clone()));
            };
            let Some(candidate) = FrozenCompletedMove::from_move(id.level, frozen.judge_at, now)
            else {
                return Err(StoreError::CompletedFreezeViolation(id.clone()));
            };
            if candidate.end_index != frozen.end_index
                || candidate.direction != frozen.direction
                || candidate.kind != frozen.kind
                || candidate.center_indices != frozen.center_indices
            {
                return Err(StoreError::CompletedFreezeViolation(id.clone()));
            }
        }

        let mut appended = 0usize;
        for value in &view.moves {
            let Some(snapshot) =
                FrozenCompletedMove::from_move(view.query.level, view.query.as_of, value)
            else {
                continue;
            };
            if self.reducer.frozen.contains_key(&snapshot.id) {
                continue;
            }
            let event = CompletedFreezeEvent {
                sequence: self.reducer.next_sequence(),
                created_at: view.query.as_of,
                cache_key: self.reducer.cache_key.clone(),
                snapshot,
            };
            let mut candidate = self.reducer.clone();
            candidate.apply(event.clone())?;
            self.store.append(&event)?;
            self.reducer = candidate;
            appended += 1;
        }
        Ok(appended)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireSnapshot {
    level: u32,
    start_index: usize,
    judge_at: usize,
    entry_bar: Option<usize>,
    end_index: usize,
    direction: Option<String>,
    kind: String,
    center_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireEventV1 {
    schema_version: u32,
    sequence: u64,
    created_at: usize,
    cache_key: String,
    snapshot: WireSnapshot,
}

impl WireEventV1 {
    fn from_event(event: &CompletedFreezeEvent) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            sequence: event.sequence,
            created_at: event.created_at,
            cache_key: event.cache_key.clone(),
            snapshot: WireSnapshot::from_snapshot(&event.snapshot),
        }
    }

    fn into_event(self) -> Result<CompletedFreezeEvent, StoreError> {
        Ok(CompletedFreezeEvent {
            sequence: self.sequence,
            created_at: self.created_at,
            cache_key: self.cache_key,
            snapshot: self.snapshot.into_snapshot()?,
        })
    }
}

/// v0：历史 prototype 行，无 schema/cache_key/entry_bar。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireEventV0 {
    sequence: u64,
    created_at: usize,
    level: u32,
    start_index: usize,
    judge_at: usize,
    end_index: usize,
    direction: Option<String>,
    kind: String,
    center_indices: Vec<usize>,
}

impl WireEventV0 {
    fn migrate(self, key: &C2PersistenceKey) -> Result<CompletedFreezeEvent, StoreError> {
        let snapshot = WireSnapshot {
            level: self.level,
            start_index: self.start_index,
            judge_at: self.judge_at,
            entry_bar: self.judge_at.checked_add(1),
            end_index: self.end_index,
            direction: self.direction,
            kind: self.kind,
            center_indices: self.center_indices,
        }
        .into_snapshot()?;
        Ok(CompletedFreezeEvent {
            sequence: self.sequence,
            created_at: self.created_at,
            cache_key: key.as_str().to_owned(),
            snapshot,
        })
    }
}

impl WireSnapshot {
    fn from_snapshot(value: &FrozenCompletedMove) -> Self {
        Self {
            level: value.id.level,
            start_index: value.id.start_index,
            judge_at: value.judge_at,
            entry_bar: value.entry_bar,
            end_index: value.end_index,
            direction: value.direction.map(|direction| match direction {
                Direction::Up => "up".to_owned(),
                Direction::Down => "down".to_owned(),
            }),
            kind: match value.kind {
                MoveKind::Trend => "trend".to_owned(),
                MoveKind::Consolidation => "consolidation".to_owned(),
            },
            center_indices: value.center_indices.clone(),
        }
    }

    fn into_snapshot(self) -> Result<FrozenCompletedMove, StoreError> {
        let direction = match self.direction.as_deref() {
            None => None,
            Some("up") => Some(Direction::Up),
            Some("down") => Some(Direction::Down),
            Some(value) => {
                return Err(StoreError::InvalidEnum {
                    field: "direction",
                    value: value.to_owned(),
                })
            }
        };
        let kind = match self.kind.as_str() {
            "trend" => MoveKind::Trend,
            "consolidation" => MoveKind::Consolidation,
            value => {
                return Err(StoreError::InvalidEnum {
                    field: "kind",
                    value: value.to_owned(),
                })
            }
        };
        Ok(FrozenCompletedMove {
            id: CompletedMoveId {
                level: self.level,
                start_index: self.start_index,
            },
            judge_at: self.judge_at,
            entry_bar: self.entry_bar,
            end_index: self.end_index,
            direction,
            kind,
            center_indices: self.center_indices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::level_view::{
        C2VersionTuple, CompletionEvidence, CoordinateWindow, LevelViewQuery,
    };
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "newchan-c2-{name}-{}-{}.jsonl",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn query() -> LevelViewQuery {
        LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 99 },
            as_of: 99,
            version: C2VersionTuple::auto_pairing(),
        }
    }

    fn completed_view(query: LevelViewQuery) -> LevelAsOfView {
        let key = C2CacheKey::from_query(&query).unwrap();
        LevelAsOfView {
            query,
            cache_key: key,
            pairs: Vec::new(),
            pair_confirmations: Vec::new(),
            moves: vec![AssembledMove {
                start_index: 0,
                end_index: 89,
                direction: Some(Direction::Up),
                kind: MoveKind::Trend,
                completion: CompletionStatus::Completed {
                    as_of: 99,
                    evidence: CompletionEvidence::SubsequentMove,
                },
                center_indices: vec![0, 1],
            }],
        }
    }

    #[test]
    fn append_only_prefix_and_completed_freeze_reject_write() {
        let path = path("append-freeze");
        let store = JsonlCompletedEventStore::new(&path);
        let mut adapter = CompletedFreezeAdapter::new(store, query()).unwrap();
        let view = completed_view(query());
        assert_eq!(adapter.observe(&view).unwrap(), 1);
        let before = std::fs::read(&path).unwrap();

        let mut appended = view.clone();
        appended.query.as_of = 100;
        appended.cache_key = C2CacheKey::from_query(&appended.query).unwrap();
        appended.moves.push(AssembledMove {
            start_index: 90,
            end_index: 99,
            direction: None,
            kind: MoveKind::Consolidation,
            completion: CompletionStatus::Completed {
                as_of: 100,
                evidence: CompletionEvidence::SubsequentMove,
            },
            center_indices: vec![2],
        });
        assert_eq!(adapter.observe(&appended).unwrap(), 1);
        let after_append = std::fs::read(&path).unwrap();
        assert!(
            after_append.starts_with(&before),
            "第二个事件必须只追加在旧字节之后"
        );
        assert!(after_append.len() > before.len());

        let mut changed = appended;
        changed.query.as_of = 101;
        changed.cache_key = C2CacheKey::from_query(&changed.query).unwrap();
        changed.moves[0].end_index = 90;
        assert!(matches!(
            adapter.observe(&changed),
            Err(StoreError::CompletedFreezeViolation(_))
        ));
        let after = std::fs::read(&path).unwrap();
        assert_eq!(after, after_append, "冻结冲突必须零写入");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reopen_and_replay_twice_are_idempotent() {
        let path = path("reopen");
        let store = JsonlCompletedEventStore::new(&path);
        let mut adapter = CompletedFreezeAdapter::new(store.clone(), query()).unwrap();
        adapter.observe(&completed_view(query())).unwrap();

        let reopened = CompletedFreezeAdapter::reopen(store.clone(), query()).unwrap();
        assert_eq!(reopened.reducer().frozen(), adapter.reducer().frozen());
        let key = C2PersistenceKey::from_query(&query()).unwrap();
        let events = store.load(&key).unwrap();
        let mut reducer = CompletedFreezeReducer::new(&key);
        reducer.replay(&events).unwrap();
        let once = reducer.clone();
        reducer.replay(&events).unwrap();
        assert_eq!(reducer, once, "同一事件流重放两次必须等于一次");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_v0_migrates_on_reopen() {
        let path = path("migration");
        let old = serde_json::json!({
            "sequence": 0,
            "created_at": 99,
            "level": 1,
            "start_index": 0,
            "judge_at": 99,
            "end_index": 89,
            "direction": "up",
            "kind": "trend",
            "center_indices": [0, 1]
        });
        std::fs::write(&path, format!("{}\n", old)).unwrap();
        let reopened =
            CompletedFreezeAdapter::reopen(JsonlCompletedEventStore::new(&path), query()).unwrap();
        let frozen = reopened.reducer().frozen().values().next().unwrap();
        assert_eq!(frozen.entry_bar, Some(100));
        assert_eq!(frozen.direction, Some(Direction::Up));
        let _ = std::fs::remove_file(path);
    }
}
