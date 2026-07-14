use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub const SCHEMA_VERSION: &str = "p68-ledger-v1";
pub const LEDGER_RULE_VERSION: u32 = 1;
pub const FIXTURE_AS_OF: usize = 4_613_598;
pub const ADOPTION_POLICY_V2: &str = "adopt_v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerScope {
    pub instrument: &'static str,
    pub timeframe: &'static str,
    pub partition: &'static str,
    pub dataset: &'static str,
}

pub const BTC_SCOPE: LedgerScope = LedgerScope {
    instrument: "BTC",
    timeframe: "1m",
    partition: "full-history",
    dataset: "btc-1m-4613599",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId {
    pub level: u32,
    pub ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerKey {
    pub scope: LedgerScope,
    pub target_level: u32,
    pub lower_id: ElementId,
    pub rule_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostRef {
    pub move_id: u64,
    pub move_version: u32,
    pub level: u32,
    pub range: SourceRange,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventProducer {
    BaseScanner,
    AdoptV1Overlay,
    AdoptV2Overlay,
    MoveReducer,
    CoordinateFinalizer,
    RuleMigration,
}

impl EventProducer {
    fn code(self) -> &'static str {
        match self {
            Self::BaseScanner => "BASE_SCANNER",
            Self::AdoptV1Overlay => "ADOPT_V1_OVERLAY",
            Self::AdoptV2Overlay => "ADOPT_V2_OVERLAY",
            Self::MoveReducer => "MOVE_REDUCER",
            Self::CoordinateFinalizer => "COORDINATE_FINALIZER",
            Self::RuleMigration => "RULE_MIGRATION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSourceRef {
    pub producer: EventProducer,
    pub stream_id: &'static str,
    pub source_event_id: u64,
    pub source_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpenReason {
    TerminalInsufficient,
    NoContainingSeedWindow,
    LeadingResidual,
    CursorGapRecomposable,
    NoHostAtAsOf,
}

impl OpenReason {
    fn code(self) -> &'static str {
        match self {
            Self::TerminalInsufficient => "TERMINAL_INSUFFICIENT",
            Self::NoContainingSeedWindow => "NO_CONTAINING_SEED_WINDOW",
            Self::LeadingResidual => "LEADING_RESIDUAL",
            Self::CursorGapRecomposable => "CURSOR_GAP_RECOMPOSABLE",
            Self::NoHostAtAsOf => "NO_HOST_AT_AS_OF",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssignmentCause {
    AdoptOrphan,
    LegalHostEstablished,
}

impl AssignmentCause {
    fn code(self) -> &'static str {
        match self {
            Self::AdoptOrphan => "ADOPT_ORPHAN",
            Self::LegalHostEstablished => "LEGAL_HOST_ESTABLISHED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdoptMechanism {
    CenterExtension,
    TrendAssembly,
    CrossWindowRecompose,
    TerminalContinuation,
}

impl AdoptMechanism {
    pub fn code(self) -> &'static str {
        match self {
            Self::CenterExtension => "CENTER_EXTENSION",
            Self::TrendAssembly => "TREND_ASSEMBLY",
            Self::CrossWindowRecompose => "CROSS_WINDOW_RECOMPOSE",
            Self::TerminalContinuation => "TERMINAL_CONTINUATION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TombstoneEvidence {
    pub lower_settled_at: usize,
    pub successor_closed_at: usize,
    pub coordinate_finalized_at: usize,
    pub exhaustive_check_at: usize,
    pub lower_settled: bool,
    pub successor_closed: bool,
    pub coordinate_finalized: bool,
    pub no_legal_or_pending_host: bool,
}

impl TombstoneEvidence {
    pub fn expected_judge_at(self) -> usize {
        self.lower_settled_at
            .max(self.successor_closed_at)
            .max(self.coordinate_finalized_at)
            .max(self.exhaustive_check_at)
    }

    fn all_conjuncts(self, judge_at: usize) -> bool {
        self.lower_settled
            && self.successor_closed
            && self.coordinate_finalized
            && self.no_legal_or_pending_host
            && judge_at == self.expected_judge_at()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    Opened {
        reason: OpenReason,
    },
    Assigned {
        cause: AssignmentCause,
        host: HostRef,
    },
    HostSupersededAdopted {
        old_host: HostRef,
        new_host: HostRef,
    },
    HostSupersededReleased {
        old_host: HostRef,
    },
    Tombstoned,
}

impl EventKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Opened { .. } => "OPENED",
            Self::Assigned { .. } => "ASSIGNED",
            Self::HostSupersededAdopted { .. } => "HOST_SUPERSEDED_ADOPTED",
            Self::HostSupersededReleased { .. } => "HOST_SUPERSEDED_RELEASED",
            Self::Tombstoned => "TOMBSTONED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdempotencyKey {
    pub schema_version: &'static str,
    pub key: LedgerKey,
    pub producer: EventProducer,
    pub source_stream: &'static str,
    pub source_event_id: u64,
    pub event_tag: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEvent {
    pub key: LedgerKey,
    pub sequence: u32,
    pub generation: u32,
    pub lower_range: SourceRange,
    pub judge_at: usize,
    pub kind: EventKind,
    pub previous_event: Option<IdempotencyKey>,
    pub correlation_id: Option<u128>,
    pub source: EventSourceRef,
    pub tombstone_evidence: Option<TombstoneEvidence>,
}

impl LedgerEvent {
    pub fn idempotency_key(&self) -> IdempotencyKey {
        IdempotencyKey {
            schema_version: SCHEMA_VERSION,
            key: self.key,
            producer: self.source.producer,
            source_stream: self.source.stream_id,
            source_event_id: self.source.source_event_id,
            event_tag: self.kind.tag(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    Assigned,
    Unassigned,
    Tombstoned,
}

impl Disposition {
    fn code(self) -> &'static str {
        match self {
            Self::Assigned => "ASSIGNED",
            Self::Unassigned => "UNASSIGNED",
            Self::Tombstoned => "TOMBSTONED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    pub disposition: Disposition,
    pub generation: u32,
    pub lower_range: SourceRange,
    pub host: Option<HostRef>,
    pub last_reason: &'static str,
    pub last_sequence: u32,
    pub last_event: IdempotencyKey,
    pub judge_at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerView {
    pub entries: BTreeMap<LedgerKey, EntryView>,
    pub supersede_edges: BTreeMap<HostRef, HostRef>,
}

impl LedgerView {
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut assigned = 0;
        let mut unassigned = 0;
        let mut tombstoned = 0;
        for entry in self.entries.values() {
            match entry.disposition {
                Disposition::Assigned => assigned += 1,
                Disposition::Unassigned => unassigned += 1,
                Disposition::Tombstoned => tombstoned += 1,
            }
        }
        (assigned, unassigned, tombstoned)
    }

    pub fn canonical_terminal_state(&self) -> Vec<u8> {
        let mut out = String::new();
        writeln!(&mut out, "schema={SCHEMA_VERSION}").expect("write String");
        for (key, entry) in &self.entries {
            let host = match entry.host {
                Some(host) => format!(
                    "{}@{}:L{}:{}-{}:{}",
                    host.move_id,
                    host.move_version,
                    host.level,
                    host.range.start,
                    host.range.end,
                    if host.completed {
                        "COMPLETED"
                    } else {
                        "PENDING"
                    }
                ),
                None => "-".to_owned(),
            };
            writeln!(
                &mut out,
                "entry|{}|{}|{}|{}|target={}|lower=L{}#{}|rule={}|range={}-{}|state={}|generation={}|host={}|reason={}|last_seq={}|last={}:{}:{}:{}:{}:{}|judge_at={}",
                key.scope.instrument,
                key.scope.timeframe,
                key.scope.partition,
                key.scope.dataset,
                key.target_level,
                key.lower_id.level,
                key.lower_id.ordinal,
                key.rule_version,
                entry.lower_range.start,
                entry.lower_range.end,
                entry.disposition.code(),
                entry.generation,
                host,
                entry.last_reason,
                entry.last_sequence,
                entry.last_event.schema_version,
                entry.last_event.producer.code(),
                entry.last_event.source_stream,
                entry.last_event.source_event_id,
                entry.last_event.event_tag,
                entry.last_event.key.lower_id.ordinal,
                entry.judge_at,
            )
            .expect("write String");
        }
        for (old, new) in &self.supersede_edges {
            writeln!(
                &mut out,
                "supersede|{}@{}->{}@{}",
                old.move_id, old.move_version, new.move_id, new.move_version
            )
            .expect("write String");
        }
        out.into_bytes()
    }

    pub fn terminal_sha256(&self) -> String {
        sha256_hex(&self.canonical_terminal_state())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    CrossLevelKey,
    InvalidRange,
    BrokenEventChain,
    InvalidTransition,
    MissingAtomicCorrelation,
    InvalidHostLevel,
    TombstoneGateFailed,
    TombstoneImmutable,
    IdempotencyConflict,
    HostMismatch,
    SupersedeCycle,
    HostAlreadySuperseded,
    DuplicateBaseKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    DuplicateNoop,
}

#[derive(Debug, Default)]
pub struct LedgerReducer {
    events: BTreeMap<IdempotencyKey, LedgerEvent>,
    entries: BTreeMap<LedgerKey, EntryView>,
    supersede_edges: BTreeMap<HostRef, HostRef>,
}

impl LedgerReducer {
    pub fn apply(&mut self, event: LedgerEvent) -> Result<ApplyOutcome, LedgerError> {
        if event.key.lower_id.level.checked_add(1) != Some(event.key.target_level) {
            return Err(LedgerError::CrossLevelKey);
        }
        if event.lower_range.start > event.lower_range.end {
            return Err(LedgerError::InvalidRange);
        }

        let event_id = event.idempotency_key();
        if let Some(existing) = self.events.get(&event_id) {
            return if existing == &event {
                Ok(ApplyOutcome::DuplicateNoop)
            } else {
                Err(LedgerError::IdempotencyConflict)
            };
        }

        let current = self.entries.get(&event.key).cloned();
        self.validate_chain(&event, current.as_ref())?;
        if current
            .as_ref()
            .is_some_and(|entry| entry.disposition == Disposition::Tombstoned)
        {
            return Err(LedgerError::TombstoneImmutable);
        }

        let mut edge_to_add = None;
        let next = match event.kind {
            EventKind::Opened { reason } => {
                if current.is_some()
                    || event.sequence != 0
                    || event.generation != 0
                    || event.previous_event.is_some()
                    || event.correlation_id.is_some()
                {
                    return Err(LedgerError::InvalidTransition);
                }
                EntryView {
                    disposition: Disposition::Unassigned,
                    generation: 0,
                    lower_range: event.lower_range,
                    host: None,
                    last_reason: reason.code(),
                    last_sequence: event.sequence,
                    last_event: event_id,
                    judge_at: event.judge_at,
                }
            }
            EventKind::Assigned { cause, host } => {
                let Some(previous) = current else {
                    return Err(LedgerError::InvalidTransition);
                };
                if previous.disposition != Disposition::Unassigned
                    || event.generation != previous.generation
                    || event.correlation_id.is_none()
                {
                    return Err(LedgerError::MissingAtomicCorrelation);
                }
                self.validate_host(&event, host)?;
                EntryView {
                    disposition: Disposition::Assigned,
                    generation: event.generation,
                    lower_range: event.lower_range,
                    host: Some(host),
                    last_reason: cause.code(),
                    last_sequence: event.sequence,
                    last_event: event_id,
                    judge_at: event.judge_at,
                }
            }
            EventKind::HostSupersededAdopted { old_host, new_host } => {
                let Some(previous) = current else {
                    return Err(LedgerError::InvalidTransition);
                };
                if previous.disposition != Disposition::Assigned
                    || previous.host != Some(old_host)
                    || event.generation != previous.generation
                {
                    return Err(LedgerError::HostMismatch);
                }
                if event.correlation_id.is_none() {
                    return Err(LedgerError::MissingAtomicCorrelation);
                }
                self.validate_host(&event, new_host)?;
                self.validate_supersede_edge(old_host, new_host)?;
                edge_to_add = Some((old_host, new_host));
                EntryView {
                    disposition: Disposition::Assigned,
                    generation: event.generation,
                    lower_range: event.lower_range,
                    host: Some(new_host),
                    last_reason: "HOST_SUPERSEDED_ADOPTED",
                    last_sequence: event.sequence,
                    last_event: event_id,
                    judge_at: event.judge_at,
                }
            }
            EventKind::HostSupersededReleased { old_host } => {
                let Some(previous) = current else {
                    return Err(LedgerError::InvalidTransition);
                };
                if previous.disposition != Disposition::Assigned || previous.host != Some(old_host)
                {
                    return Err(LedgerError::HostMismatch);
                }
                if event.correlation_id.is_none()
                    || event.generation != previous.generation.saturating_add(1)
                {
                    return Err(LedgerError::MissingAtomicCorrelation);
                }
                EntryView {
                    disposition: Disposition::Unassigned,
                    generation: event.generation,
                    lower_range: event.lower_range,
                    host: None,
                    last_reason: "HOST_SUPERSEDED_RELEASED",
                    last_sequence: event.sequence,
                    last_event: event_id,
                    judge_at: event.judge_at,
                }
            }
            EventKind::Tombstoned => {
                let Some(previous) = current else {
                    return Err(LedgerError::InvalidTransition);
                };
                if previous.disposition != Disposition::Unassigned
                    || event.generation != previous.generation
                    || event.correlation_id.is_some()
                {
                    return Err(LedgerError::InvalidTransition);
                }
                let Some(evidence) = event.tombstone_evidence else {
                    return Err(LedgerError::TombstoneGateFailed);
                };
                if !evidence.all_conjuncts(event.judge_at) {
                    return Err(LedgerError::TombstoneGateFailed);
                }
                EntryView {
                    disposition: Disposition::Tombstoned,
                    generation: event.generation,
                    lower_range: event.lower_range,
                    host: None,
                    last_reason: "FINALIZED_NO_LEGAL_HOST",
                    last_sequence: event.sequence,
                    last_event: event_id,
                    judge_at: event.judge_at,
                }
            }
        };

        if let Some((old, new)) = edge_to_add {
            self.supersede_edges.insert(old, new);
        }
        self.events.insert(event_id, event.clone());
        self.entries.insert(event.key, next);
        Ok(ApplyOutcome::Applied)
    }

    fn validate_chain(
        &self,
        event: &LedgerEvent,
        current: Option<&EntryView>,
    ) -> Result<(), LedgerError> {
        match current {
            None => {
                if event.sequence != 0 || event.previous_event.is_some() || event.generation != 0 {
                    return Err(LedgerError::BrokenEventChain);
                }
            }
            Some(previous) => {
                if event.sequence != previous.last_sequence.saturating_add(1)
                    || event.previous_event != Some(previous.last_event)
                    || event.lower_range != previous.lower_range
                    || event.judge_at < previous.judge_at
                {
                    return Err(LedgerError::BrokenEventChain);
                }
            }
        }
        Ok(())
    }

    fn validate_host(&self, event: &LedgerEvent, host: HostRef) -> Result<(), LedgerError> {
        if host.level != event.key.target_level || host.range.start > host.range.end {
            return Err(LedgerError::InvalidHostLevel);
        }
        Ok(())
    }

    fn validate_supersede_edge(&self, old: HostRef, new: HostRef) -> Result<(), LedgerError> {
        if self.supersede_edges.contains_key(&old) {
            return Err(LedgerError::HostAlreadySuperseded);
        }
        if old == new {
            return Err(LedgerError::SupersedeCycle);
        }
        let mut cursor = new;
        let mut visited = BTreeSet::new();
        while visited.insert(cursor) {
            if cursor == old {
                return Err(LedgerError::SupersedeCycle);
            }
            let Some(next) = self.supersede_edges.get(&cursor).copied() else {
                return Ok(());
            };
            cursor = next;
        }
        Err(LedgerError::SupersedeCycle)
    }

    pub fn view(&self) -> LedgerView {
        LedgerView {
            entries: self.entries.clone(),
            supersede_edges: self.supersede_edges.clone(),
        }
    }
}

pub fn replay(events: &[LedgerEvent]) -> Result<LedgerView, LedgerError> {
    let mut reducer = LedgerReducer::default();
    for event in events {
        reducer.apply(event.clone())?;
    }
    Ok(reducer.view())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseRecord {
    pub key: LedgerKey,
    pub lower_range: SourceRange,
    pub settled_at: usize,
    pub base_host: Option<HostRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdoptionCandidate {
    pub key: LedgerKey,
    pub lower_range: SourceRange,
    pub judge_at: usize,
    pub mechanism: AdoptMechanism,
    pub host: HostRef,
}

fn candidate_tie_key(candidate: &AdoptionCandidate) -> (AdoptMechanism, usize, usize, usize, u64) {
    (
        candidate.mechanism,
        candidate
            .host
            .range
            .end
            .saturating_sub(candidate.host.range.start),
        candidate.host.range.start,
        candidate.host.range.end,
        candidate.host.move_id,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationResult {
    pub base_assignments: BTreeMap<LedgerKey, HostRef>,
    pub adopted: BTreeMap<LedgerKey, AdoptionCandidate>,
    pub events: Vec<LedgerEvent>,
    pub ledger: LedgerView,
}

pub fn integrate_adopt_v1(
    records: &[BaseRecord],
    candidates: &[AdoptionCandidate],
    as_of: usize,
) -> Result<IntegrationResult, LedgerError> {
    let mut visible = BTreeMap::new();
    for record in records.iter().filter(|record| record.settled_at <= as_of) {
        if visible.insert(record.key, *record).is_some() {
            return Err(LedgerError::DuplicateBaseKey);
        }
    }

    let mut candidates_by_key: BTreeMap<LedgerKey, Vec<AdoptionCandidate>> = BTreeMap::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.judge_at <= as_of)
    {
        candidates_by_key
            .entry(candidate.key)
            .or_default()
            .push(*candidate);
    }

    let mut base_assignments = BTreeMap::new();
    let mut adopted = BTreeMap::new();
    let mut events = Vec::new();
    for (key, record) in visible {
        if let Some(host) = record.base_host {
            base_assignments.insert(key, host);
            continue;
        }

        let opened = LedgerEvent {
            key,
            sequence: 0,
            generation: 0,
            lower_range: record.lower_range,
            judge_at: record.settled_at,
            kind: EventKind::Opened {
                reason: OpenReason::NoContainingSeedWindow,
            },
            previous_event: None,
            correlation_id: None,
            source: EventSourceRef {
                producer: EventProducer::BaseScanner,
                stream_id: "btc-1m/task67-exact-three",
                source_event_id: key.lower_id.ordinal,
                source_offset: key.lower_id.ordinal,
            },
            tombstone_evidence: None,
        };
        let opened_id = opened.idempotency_key();
        events.push(opened);

        let chosen = candidates_by_key.get_mut(&key).and_then(|options| {
            options.sort_by_key(candidate_tie_key);
            options.first().copied()
        });
        if let Some(candidate) = chosen {
            let correlation_id = ((key.target_level as u128) << 96)
                | ((key.lower_id.level as u128) << 64)
                | key.lower_id.ordinal as u128;
            events.push(LedgerEvent {
                key,
                sequence: 1,
                generation: 0,
                lower_range: record.lower_range,
                judge_at: candidate.judge_at.max(record.settled_at),
                kind: EventKind::Assigned {
                    cause: AssignmentCause::AdoptOrphan,
                    host: candidate.host,
                },
                previous_event: Some(opened_id),
                correlation_id: Some(correlation_id),
                source: EventSourceRef {
                    producer: EventProducer::AdoptV1Overlay,
                    stream_id: "btc-1m/task67-adopt-v1",
                    source_event_id: candidate.host.move_id,
                    source_offset: candidate.host.move_id,
                },
                tombstone_evidence: None,
            });
            adopted.insert(key, candidate);
        }
    }

    let ledger = replay(&events)?;
    Ok(IntegrationResult {
        base_assignments,
        adopted,
        events,
        ledger,
    })
}

fn fixture_key(level: u32, ordinal: u64) -> LedgerKey {
    LedgerKey {
        scope: BTC_SCOPE,
        target_level: level + 1,
        lower_id: ElementId { level, ordinal },
        rule_version: LEDGER_RULE_VERSION,
    }
}

fn fixture_host(level: u32, ordinal: u64, start: usize, end: usize, completed: bool) -> HostRef {
    HostRef {
        move_id: ((level as u64 + 1) << 56) | ordinal,
        move_version: 1,
        level: level + 1,
        range: SourceRange { start, end },
        completed,
    }
}

pub fn task67_fixture() -> (Vec<BaseRecord>, Vec<AdoptionCandidate>) {
    let adopted = [
        (
            0,
            1126,
            137974,
            137996,
            136767,
            138631,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            3217,
            367919,
            367993,
            367527,
            368201,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            3457,
            391323,
            391355,
            390932,
            391355,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            8062,
            892963,
            892984,
            891823,
            893171,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            15685,
            1733210,
            1733340,
            1732375,
            1733617,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            15798,
            1746272,
            1746389,
            1745358,
            1747317,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            16192,
            1792727,
            1792891,
            1792356,
            1793002,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            20470,
            2306893,
            2306924,
            2305960,
            2306945,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            21613,
            2444538,
            2444617,
            2444220,
            2445064,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            23759,
            2703777,
            2703853,
            2703015,
            2704269,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            24012,
            2734544,
            2734670,
            2733227,
            2734862,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            27163,
            3102728,
            3102816,
            3101775,
            3104190,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            32353,
            3702311,
            3702494,
            3701504,
            3702699,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            33409,
            3827807,
            3827850,
            3825651,
            3827879,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            35909,
            4128452,
            4128611,
            4128007,
            4129083,
            true,
            AdoptMechanism::CenterExtension,
        ),
        (
            0,
            36752,
            4225895,
            4226074,
            4224623,
            4226074,
            false,
            AdoptMechanism::TrendAssembly,
        ),
        (
            0,
            39878,
            4598245,
            4598300,
            4597872,
            4598760,
            false,
            AdoptMechanism::TrendAssembly,
        ),
        (
            1,
            10846,
            4133078,
            4133432,
            4130475,
            4134490,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            2,
            1514,
            1864033,
            1865267,
            1859426,
            1865267,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
    ];
    let residual = [
        (0, 2498, 294379, 294678),
        (0, 3705, 417456, 417565),
        (0, 4869, 542428, 542539),
        (0, 10581, 1169165, 1169293),
        (0, 11956, 1319311, 1319352),
        (0, 12211, 1349713, 1349876),
        (0, 13019, 1437139, 1437152),
        (0, 14718, 1621897, 1622008),
        (0, 20127, 2264911, 2264933),
        (0, 20299, 2286677, 2286887),
        (0, 22171, 2510715, 2510734),
        (0, 22729, 2576876, 2576984),
        (0, 24227, 2759962, 2760015),
        (0, 25368, 2903157, 2903192),
        (0, 27013, 3084709, 3084854),
        (0, 28272, 3229763, 3229845),
        (0, 30608, 3499553, 3499792),
        (0, 30760, 3516730, 3516953),
        (0, 31288, 3577921, 3577959),
        (0, 32469, 3716206, 3716484),
        (0, 33046, 3783351, 3783467),
        (0, 35944, 4132879, 4133078),
        (0, 36442, 4190140, 4190318),
        (1, 3278, 1183803, 1184381),
        (1, 3700, 1333681, 1334083),
        (1, 5304, 1939180, 1939671),
        (1, 6576, 2446451, 2446609),
        (1, 8479, 3192471, 3192734),
        (1, 9472, 3578942, 3579315),
        (1, 9573, 3616553, 3616977),
        (1, 10947, 4171769, 4172028),
        (1, 11518, 4397210, 4397392),
        (2, 107, 145672, 147050),
    ];

    let mut records = Vec::new();
    let mut candidates = Vec::new();
    for (level, ordinal, lower_start, lower_end, host_start, host_end, completed, mechanism) in
        adopted
    {
        let key = fixture_key(level, ordinal);
        let lower_range = SourceRange {
            start: lower_start,
            end: lower_end,
        };
        let host = fixture_host(level, ordinal, host_start, host_end, completed);
        records.push(BaseRecord {
            key,
            lower_range,
            settled_at: lower_end,
            base_host: None,
        });
        candidates.push(AdoptionCandidate {
            key,
            lower_range,
            judge_at: host_end,
            mechanism,
            host,
        });
    }
    for (level, ordinal, start, end) in residual {
        records.push(BaseRecord {
            key: fixture_key(level, ordinal),
            lower_range: SourceRange { start, end },
            settled_at: end,
            base_host: None,
        });
    }
    (records, candidates)
}

pub fn run_task67_fixture() -> IntegrationResult {
    let (records, candidates) = task67_fixture();
    let result = integrate_adopt_v1(&records, &candidates, FIXTURE_AS_OF)
        .expect("frozen task67 fixture must reduce");
    let (assigned, unassigned, tombstoned) = result.ledger.counts();
    assert_eq!(records.len(), 52);
    assert_eq!(result.adopted.len(), 19);
    assert_eq!(
        result.adopted.values().filter(|c| c.host.completed).count(),
        7
    );
    assert_eq!((assigned, unassigned, tombstoned), (19, 33, 0));
    assert_eq!(assigned + unassigned + tombstoned, records.len());
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeometryBucket {
    GapAdjacent,
    MonotoneSteep,
    Other,
}

impl GeometryBucket {
    pub fn code(self) -> &'static str {
        match self {
            Self::GapAdjacent => "GAP_ADJACENT",
            Self::MonotoneSteep => "MONOTONE_STEEP",
            Self::Other => "GEOMETRY_OTHER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectorRule {
    R2,
    R3,
    R4,
    R2Prime,
}

impl ConnectorRule {
    pub fn code(self) -> &'static str {
        match self {
            Self::R2 => "R2",
            Self::R3 => "R3",
            Self::R4 => "R4",
            Self::R2Prime => "R2_PRIME",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectorChannel {
    R2,
    R3,
}

impl ConnectorChannel {
    pub fn code(self) -> &'static str {
        match self {
            Self::R2 => "R2",
            Self::R3 => "R3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResidualReason {
    R2PrimeDisabled,
    R2PrimeGeometryFailed,
    GapExemptButNoLegalHost,
    GeometryOtherOutOfScope,
    NoVisibleConnectorFactAtAsOf,
}

impl ResidualReason {
    pub fn code(self) -> &'static str {
        match self {
            Self::R2PrimeDisabled => "R2_PRIME_DISABLED",
            Self::R2PrimeGeometryFailed => "R2_PRIME_GEOMETRY_FAILED",
            Self::GapExemptButNoLegalHost => "R1_EXEMPT_BUT_NO_LEGAL_HOST",
            Self::GeometryOtherOutOfScope => "GEOMETRY_OTHER_OUT_OF_SCOPE",
            Self::NoVisibleConnectorFactAtAsOf => "NO_VISIBLE_CONNECTOR_FACT_AT_AS_OF",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterConnectorEvidence {
    pub zd: i64,
    pub zg: i64,
    pub segment_low: i64,
    pub segment_high: i64,
    pub host: HostRef,
}

impl CenterConnectorEvidence {
    fn absorbs(self, target_level: u32) -> bool {
        self.host.level == target_level
            && self.segment_high >= self.zd
            && self.segment_low <= self.zg
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectorFact {
    pub key: LedgerKey,
    pub lower_range: SourceRange,
    pub judge_at: usize,
    pub geometry: GeometryBucket,
    pub left_host: Option<HostRef>,
    pub right_host: Option<HostRef>,
    pub chain_adjacent: bool,
    pub sequence_start: bool,
    pub continuous_same_direction_no_overlap: bool,
    pub center: Option<CenterConnectorEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorOutcome {
    Adopted {
        rule: ConnectorRule,
        channel: ConnectorChannel,
        r1_exemption: bool,
        host: HostRef,
        judge_at: usize,
    },
    Residual {
        reason: ResidualReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectorDecision {
    pub key: LedgerKey,
    pub geometry: Option<GeometryBucket>,
    pub outcome: ConnectorOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdoptV2Options {
    pub enable_r2_prime: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapConnectorError {
    Ledger(LedgerError),
    DuplicateConnectorFact,
    FactOutsideResidualDomain,
    FactRangeMismatch,
    UnsatisfiableWithMonotonicity,
    ConservationViolation,
}

impl GapConnectorError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsatisfiableWithMonotonicity => "UNSATISFIABLE_WITH_MONOTONICITY",
            Self::ConservationViolation => "CONSERVATION_VIOLATION",
            Self::DuplicateConnectorFact => "DUPLICATE_CONNECTOR_FACT",
            Self::FactOutsideResidualDomain => "FACT_OUTSIDE_RESIDUAL_DOMAIN",
            Self::FactRangeMismatch => "FACT_RANGE_MISMATCH",
            Self::Ledger(_) => "LEDGER_REDUCER_ERROR",
        }
    }
}

impl From<LedgerError> for GapConnectorError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

fn r2_host(fact: ConnectorFact) -> Option<HostRef> {
    let (Some(left), Some(right)) = (fact.left_host, fact.right_host) else {
        return None;
    };
    (fact.chain_adjacent
        && left.level == fact.key.target_level
        && right.level == fact.key.target_level
        && left.range.end <= fact.lower_range.start
        && right.range.start >= fact.lower_range.end)
        // Coordinate-order policy: a connector is attributed to the following
        // already assembled host; this never creates a host.
        .then_some(right)
}

fn r3_host(fact: ConnectorFact) -> Option<HostRef> {
    fact.center
        .filter(|center| center.absorbs(fact.key.target_level))
        .map(|center| center.host)
}

fn r4_host(fact: ConnectorFact) -> Option<HostRef> {
    let right = fact.right_host?;
    (fact.sequence_start
        && fact.left_host.is_none()
        && right.level == fact.key.target_level
        && right.range.start >= fact.lower_range.end)
        .then_some(right)
}

pub fn evaluate_connector(fact: ConnectorFact, options: AdoptV2Options) -> ConnectorOutcome {
    match fact.geometry {
        GeometryBucket::GapAdjacent => {
            // R1 exempts the gap from the three-fold/containing-seed gate. It
            // does not manufacture a host, so R2/R3/R4 still must prove one.
            if let Some(host) = r2_host(fact) {
                ConnectorOutcome::Adopted {
                    rule: ConnectorRule::R2,
                    channel: ConnectorChannel::R2,
                    r1_exemption: true,
                    host,
                    judge_at: fact.judge_at,
                }
            } else if let Some(host) = r3_host(fact) {
                ConnectorOutcome::Adopted {
                    rule: ConnectorRule::R3,
                    channel: ConnectorChannel::R3,
                    r1_exemption: true,
                    host,
                    judge_at: fact.judge_at,
                }
            } else if let Some(host) = r4_host(fact) {
                ConnectorOutcome::Adopted {
                    rule: ConnectorRule::R4,
                    channel: ConnectorChannel::R2,
                    r1_exemption: true,
                    host,
                    judge_at: fact.judge_at,
                }
            } else {
                ConnectorOutcome::Residual {
                    reason: ResidualReason::GapExemptButNoLegalHost,
                }
            }
        }
        GeometryBucket::MonotoneSteep => {
            if !options.enable_r2_prime {
                return ConnectorOutcome::Residual {
                    reason: ResidualReason::R2PrimeDisabled,
                };
            }
            if !fact.continuous_same_direction_no_overlap {
                return ConnectorOutcome::Residual {
                    reason: ResidualReason::R2PrimeGeometryFailed,
                };
            }
            if let Some(host) = r2_host(fact) {
                ConnectorOutcome::Adopted {
                    rule: ConnectorRule::R2Prime,
                    channel: ConnectorChannel::R2,
                    r1_exemption: false,
                    host,
                    judge_at: fact.judge_at,
                }
            } else if let Some(host) = r3_host(fact) {
                ConnectorOutcome::Adopted {
                    rule: ConnectorRule::R2Prime,
                    channel: ConnectorChannel::R3,
                    r1_exemption: false,
                    host,
                    judge_at: fact.judge_at,
                }
            } else {
                ConnectorOutcome::Residual {
                    reason: ResidualReason::GapExemptButNoLegalHost,
                }
            }
        }
        GeometryBucket::Other => ConnectorOutcome::Residual {
            reason: ResidualReason::GeometryOtherOutOfScope,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptV2Result {
    pub options: AdoptV2Options,
    pub base_assignments: BTreeMap<LedgerKey, HostRef>,
    pub v1_events: Vec<LedgerEvent>,
    pub extension_events: Vec<LedgerEvent>,
    pub events: Vec<LedgerEvent>,
    pub v1_ledger: LedgerView,
    pub ledger: LedgerView,
    pub decisions: BTreeMap<LedgerKey, ConnectorDecision>,
    pub adopted: BTreeMap<LedgerKey, AdoptionCandidate>,
    pub residual: BTreeMap<LedgerKey, ResidualReason>,
    pub residual_domain: usize,
}

pub fn integrate_adopt_v2(
    v1: IntegrationResult,
    facts: &[ConnectorFact],
    options: AdoptV2Options,
    as_of: usize,
) -> Result<AdoptV2Result, GapConnectorError> {
    let residual_keys: BTreeSet<_> = v1
        .ledger
        .entries
        .iter()
        .filter_map(|(key, entry)| (entry.disposition == Disposition::Unassigned).then_some(*key))
        .collect();
    let residual_domain = residual_keys.len();

    let mut visible_facts = BTreeMap::new();
    for fact in facts.iter().filter(|fact| fact.judge_at <= as_of) {
        if !residual_keys.contains(&fact.key) {
            return Err(GapConnectorError::FactOutsideResidualDomain);
        }
        if visible_facts.insert(fact.key, *fact).is_some() {
            return Err(GapConnectorError::DuplicateConnectorFact);
        }
    }

    let mut extension_events = Vec::new();
    let mut adopted = BTreeMap::new();
    let mut residual = BTreeMap::new();
    let mut decisions = BTreeMap::new();
    for key in residual_keys.iter().copied() {
        let prior = &v1.ledger.entries[&key];
        let Some(fact) = visible_facts.get(&key).copied() else {
            let reason = ResidualReason::NoVisibleConnectorFactAtAsOf;
            residual.insert(key, reason);
            decisions.insert(
                key,
                ConnectorDecision {
                    key,
                    geometry: None,
                    outcome: ConnectorOutcome::Residual { reason },
                },
            );
            continue;
        };
        if fact.lower_range != prior.lower_range {
            return Err(GapConnectorError::FactRangeMismatch);
        }
        let outcome = evaluate_connector(fact, options);
        decisions.insert(
            key,
            ConnectorDecision {
                key,
                geometry: Some(fact.geometry),
                outcome,
            },
        );
        match outcome {
            ConnectorOutcome::Adopted { host, judge_at, .. } => {
                let candidate = AdoptionCandidate {
                    key,
                    lower_range: fact.lower_range,
                    judge_at,
                    mechanism: AdoptMechanism::CrossWindowRecompose,
                    host,
                };
                let correlation_id = (72u128 << 112)
                    | ((key.target_level as u128) << 96)
                    | ((key.lower_id.level as u128) << 64)
                    | key.lower_id.ordinal as u128;
                extension_events.push(LedgerEvent {
                    key,
                    sequence: prior.last_sequence.saturating_add(1),
                    generation: prior.generation,
                    lower_range: prior.lower_range,
                    judge_at: judge_at.max(prior.judge_at),
                    kind: EventKind::Assigned {
                        cause: AssignmentCause::AdoptOrphan,
                        host,
                    },
                    previous_event: Some(prior.last_event),
                    correlation_id: Some(correlation_id),
                    source: EventSourceRef {
                        producer: EventProducer::AdoptV2Overlay,
                        stream_id: "btc-1m/task72-adopt-v2",
                        source_event_id: key.lower_id.ordinal,
                        source_offset: key.lower_id.ordinal,
                    },
                    tombstone_evidence: None,
                });
                adopted.insert(key, candidate);
            }
            ConnectorOutcome::Residual { reason } => {
                residual.insert(key, reason);
            }
        }
    }

    let v1_events = v1.events.clone();
    let mut events = v1_events.clone();
    events.extend(extension_events.clone());
    if events.get(..v1_events.len()) != Some(v1_events.as_slice())
        || extension_events.iter().any(|event| {
            !matches!(
                event.kind,
                EventKind::Assigned {
                    cause: AssignmentCause::AdoptOrphan,
                    ..
                }
            ) || event.source.producer != EventProducer::AdoptV2Overlay
        })
    {
        return Err(GapConnectorError::UnsatisfiableWithMonotonicity);
    }

    let ledger = replay(&events)?;
    for (key, before) in &v1.ledger.entries {
        if before.disposition == Disposition::Assigned {
            let Some(after) = ledger.entries.get(key) else {
                return Err(GapConnectorError::UnsatisfiableWithMonotonicity);
            };
            if after.disposition != Disposition::Assigned || after.host != before.host {
                return Err(GapConnectorError::UnsatisfiableWithMonotonicity);
            }
        }
    }
    if extension_events
        .iter()
        .any(|event| matches!(event.kind, EventKind::HostSupersededReleased { .. }))
    {
        return Err(GapConnectorError::UnsatisfiableWithMonotonicity);
    }

    let final_residual = residual_keys
        .iter()
        .filter(|key| {
            ledger.entries.get(key).is_some_and(|entry| {
                entry.disposition == Disposition::Unassigned && entry.host.is_none()
            })
        })
        .count();
    if residual_domain != adopted.len() + residual.len()
        || final_residual != residual.len()
        || residual_keys.iter().any(|key| {
            ledger
                .entries
                .get(key)
                .is_some_and(|entry| entry.disposition == Disposition::Tombstoned)
        })
    {
        return Err(GapConnectorError::ConservationViolation);
    }

    Ok(AdoptV2Result {
        options,
        base_assignments: v1.base_assignments,
        v1_events,
        extension_events,
        events,
        v1_ledger: v1.ledger,
        ledger,
        decisions,
        adopted,
        residual,
        residual_domain,
    })
}

fn connector_fixture_host(level: u32, start: usize, end: usize) -> HostRef {
    HostRef {
        move_id: (((level + 1) as u64) << 56) | start as u64,
        move_version: 1,
        level: level + 1,
        range: SourceRange { start, end },
        completed: false,
    }
}

pub fn task72_connector_fixture() -> Vec<ConnectorFact> {
    let rows = [
        (
            0,
            2498,
            294379,
            294678,
            GeometryBucket::GapAdjacent,
            292940,
            294372,
            294678,
            295878,
        ),
        (
            0,
            3705,
            417456,
            417565,
            GeometryBucket::GapAdjacent,
            415558,
            417435,
            417565,
            417983,
        ),
        (
            0,
            4869,
            542428,
            542539,
            GeometryBucket::Other,
            542061,
            542411,
            542539,
            542814,
        ),
        (
            0,
            10581,
            1169165,
            1169293,
            GeometryBucket::MonotoneSteep,
            1167726,
            1169155,
            1169293,
            1169722,
        ),
        (
            0,
            11956,
            1319311,
            1319352,
            GeometryBucket::GapAdjacent,
            1318722,
            1319262,
            1319352,
            1319591,
        ),
        (
            0,
            12211,
            1349713,
            1349876,
            GeometryBucket::GapAdjacent,
            1349035,
            1349664,
            1349876,
            1350915,
        ),
        (
            0,
            13019,
            1437139,
            1437152,
            GeometryBucket::GapAdjacent,
            1435611,
            1437122,
            1437152,
            1437378,
        ),
        (
            0,
            14718,
            1621897,
            1622008,
            GeometryBucket::MonotoneSteep,
            1621249,
            1621881,
            1622008,
            1622773,
        ),
        (
            0,
            20127,
            2264911,
            2264933,
            GeometryBucket::GapAdjacent,
            2264303,
            2264882,
            2264933,
            2267159,
        ),
        (
            0,
            20299,
            2286677,
            2286887,
            GeometryBucket::Other,
            2285938,
            2286674,
            2286887,
            2288041,
        ),
        (
            0,
            22171,
            2510715,
            2510734,
            GeometryBucket::GapAdjacent,
            2510438,
            2510688,
            2510734,
            2511409,
        ),
        (
            0,
            22729,
            2576876,
            2576984,
            GeometryBucket::GapAdjacent,
            2576339,
            2576835,
            2576984,
            2577194,
        ),
        (
            0,
            24227,
            2759962,
            2760015,
            GeometryBucket::GapAdjacent,
            2759524,
            2759881,
            2760015,
            2761052,
        ),
        (
            0,
            25368,
            2903157,
            2903192,
            GeometryBucket::GapAdjacent,
            2902596,
            2903146,
            2903192,
            2904217,
        ),
        (
            0,
            27013,
            3084709,
            3084854,
            GeometryBucket::GapAdjacent,
            3084346,
            3084665,
            3084854,
            3086106,
        ),
        (
            0,
            28272,
            3229763,
            3229845,
            GeometryBucket::MonotoneSteep,
            3229324,
            3229717,
            3229845,
            3230660,
        ),
        (
            0,
            30608,
            3499553,
            3499792,
            GeometryBucket::GapAdjacent,
            3498318,
            3499535,
            3499792,
            3500137,
        ),
        (
            0,
            30760,
            3516730,
            3516953,
            GeometryBucket::MonotoneSteep,
            3516267,
            3516698,
            3516953,
            3519514,
        ),
        (
            0,
            31288,
            3577921,
            3577959,
            GeometryBucket::MonotoneSteep,
            3576515,
            3577900,
            3577959,
            3579175,
        ),
        (
            0,
            32469,
            3716206,
            3716484,
            GeometryBucket::GapAdjacent,
            3715604,
            3716190,
            3716484,
            3717787,
        ),
        (
            0,
            33046,
            3783351,
            3783467,
            GeometryBucket::GapAdjacent,
            3782616,
            3783323,
            3783467,
            3783612,
        ),
        (
            0,
            35944,
            4132879,
            4133078,
            GeometryBucket::Other,
            4129956,
            4132859,
            4133078,
            4134027,
        ),
        (
            0,
            36442,
            4190140,
            4190318,
            GeometryBucket::MonotoneSteep,
            4189772,
            4190140,
            4190318,
            4191056,
        ),
        (
            1,
            3278,
            1183803,
            1184381,
            GeometryBucket::MonotoneSteep,
            1182024,
            1183798,
            1184381,
            1188719,
        ),
        (
            1,
            3700,
            1333681,
            1334083,
            GeometryBucket::MonotoneSteep,
            1331249,
            1333610,
            1334083,
            1339107,
        ),
        (
            1,
            5304,
            1939180,
            1939671,
            GeometryBucket::GapAdjacent,
            1937199,
            1939074,
            1939671,
            1940717,
        ),
        (
            1,
            6576,
            2446451,
            2446609,
            GeometryBucket::GapAdjacent,
            2445391,
            2446425,
            2446609,
            2453060,
        ),
        (
            1,
            8479,
            3192471,
            3192734,
            GeometryBucket::GapAdjacent,
            3191716,
            3192449,
            3192734,
            3194583,
        ),
        (
            1,
            9472,
            3578942,
            3579315,
            GeometryBucket::MonotoneSteep,
            3577172,
            3578569,
            3579315,
            3580440,
        ),
        (
            1,
            9573,
            3616553,
            3616977,
            GeometryBucket::GapAdjacent,
            3613764,
            3616531,
            3616977,
            3618063,
        ),
        (
            1,
            10947,
            4171769,
            4172028,
            GeometryBucket::GapAdjacent,
            4170954,
            4171481,
            4172028,
            4173029,
        ),
        (
            1,
            11518,
            4397210,
            4397392,
            GeometryBucket::MonotoneSteep,
            4395651,
            4397179,
            4397392,
            4398924,
        ),
        (
            2,
            107,
            145672,
            147050,
            GeometryBucket::MonotoneSteep,
            139295,
            145664,
            147050,
            150305,
        ),
    ];
    rows.into_iter()
        .map(
            |(
                level,
                ordinal,
                lower_start,
                lower_end,
                geometry,
                left_start,
                left_end,
                right_start,
                right_end,
            )| {
                ConnectorFact {
                    key: fixture_key(level, ordinal),
                    lower_range: SourceRange {
                        start: lower_start,
                        end: lower_end,
                    },
                    judge_at: right_end.max(lower_end),
                    geometry,
                    left_host: Some(connector_fixture_host(level, left_start, left_end)),
                    right_host: Some(connector_fixture_host(level, right_start, right_end)),
                    chain_adjacent: true,
                    sequence_start: false,
                    continuous_same_direction_no_overlap: geometry == GeometryBucket::MonotoneSteep,
                    center: None,
                }
            },
        )
        .collect()
}

pub fn run_task72_fixture(enable_r2_prime: bool) -> Result<AdoptV2Result, GapConnectorError> {
    let facts = task72_connector_fixture();
    integrate_adopt_v2(
        run_task67_fixture(),
        &facts,
        AdoptV2Options { enable_r2_prime },
        FIXTURE_AS_OF,
    )
}

pub fn connector_channels(
    result: &AdoptV2Result,
    bucket: GeometryBucket,
) -> BTreeSet<ConnectorChannel> {
    result
        .decisions
        .values()
        .filter_map(|decision| {
            if decision.geometry != Some(bucket) {
                return None;
            }
            match decision.outcome {
                ConnectorOutcome::Adopted { channel, .. } => Some(channel),
                ConnectorOutcome::Residual { .. } => None,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreservedBaseCase {
    pub key: LedgerKey,
    pub lower_range: SourceRange,
    pub geometry: GeometryBucket,
    pub base_host: HostRef,
    pub inner_left: SourceRange,
    pub inner_right: SourceRange,
}

impl PreservedBaseCase {
    pub fn validates_r1_r2_basis(self) -> bool {
        self.geometry == GeometryBucket::GapAdjacent
            && self.base_host.level == self.key.target_level
            && self.base_host.range.start <= self.inner_left.start
            && self.inner_left.end <= self.lower_range.start
            && self.inner_right.start >= self.lower_range.end
            && self.inner_right.end <= self.base_host.range.end
            && self.base_host.range.start <= self.lower_range.start
            && self.base_host.range.end >= self.lower_range.end
    }
}

pub fn l0_37199_preserved_case() -> PreservedBaseCase {
    PreservedBaseCase {
        key: fixture_key(0, 37199),
        lower_range: SourceRange {
            start: 4_277_801,
            end: 4_277_866,
        },
        geometry: GeometryBucket::GapAdjacent,
        base_host: connector_fixture_host(0, 4_276_595, 4_280_151),
        inner_left: SourceRange {
            start: 4_277_222,
            end: 4_277_751,
        },
        inner_right: SourceRange {
            start: 4_277_866,
            end: 4_279_768,
        },
    }
}

pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(sum1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}
