//! append-only 修订日志 + JSONL 外化 + 重放折叠恢复（票 #621 S1 第三段；#574 裁定六，M5=A）。
//!
//! # 唯一真相
//!
//! entry 内 append-only 修订留档即**日志本体**；[`RetraceLedger::journal`] 是它的全局定序镜像
//! （`sequence` = 产生序），是本账**唯一真相**。三态、注册快照、Restart 谱系、路由投影全部是
//! 它的折叠结果，一律不单独持久化（日志 + 状态双真相必漂移，裁定六禁）。
//!
//! - **持久化** = 把日志序列化为 JSONL（[`JsonlRetraceLogStore`]，`CompletedFreezeReducer` 先例：
//!   正式边界只追加、重放幂等、冲突拒绝）；
//! - **恢复** = [`RetraceLedger::fold`] 重放折叠回状态——折叠路径逐条**重跑内核原语**并与记录
//!   逐字段比对，任何不符即 [`RetraceLogError::FoldViolation`]；
//! - **快照** = [`RetraceSnapshot`]，**仅派生缓存**，带溯源（折叠到第 N 条 + 该前缀指纹），
//!   校验失败即扔掉、回退全量重放（[`RestoreRoute::FullReplay`]）。
//!
//! 拒收/警报（[`super::RetraceAlarms`]）**不进日志**——它们不改状态，另记 audit 流（裁定六）。
//!
//! # 门卫钟的可恢复性（诚实声明，非漂移）
//!
//! 出生钟与落锤钟是**身份事实**，随修订进日志、重放逐位复现。**门卫钟不是**：它是输入流守卫，
//! 连不产任何修订的未决观察也推它前移（裁定五「随推进前移」）。若把它塞进日志，就要为每个
//! 未决 bar 追加一条无载荷修订——那是把守卫状态伪装成身份事实。
//!
//! 因此本模块声明：**跨进程恢复后，门卫钟退回该身份留档所支持的最强下界**
//! （[`super::RetraceEntry::log_supported_gate`] = 最后一条修订的知情时）。可观察后果——
//! 恢复后，落在「最后一条修订知情时」与「重启前最后一次未决观察知情时」之间的输入不再被判
//! 倒退。日志对这段区间本就一无所知（其间没有任何身份事实发生），账本不编造它没有的知识。
//!
//! **护栏后口径**（编排者 2026-07-29 裁方案②，票 #632；影子评审 #621 MEDIUM-1 落地）：
//! 上一段声明对**终态**身份始终准确无害（只影响迟到吸收计数）；对**未决**身份，若不加额外
//! 护栏，该区间的输入不但不再被拒、还能把陈旧知情时写进首写永不改的落锤钟与成立档
//! `confirmed_as_of`（产物不可撤销）——这正是 MEDIUM-1 命中的历史行为。**S1 门卫钟本身的退回
//! 语义不变**（仍是「留档下界」，不隐瞒不编造）；本票在其上叠加一层独立护栏：调用方经
//! [`RetraceLedger::fold_recovered`] 显式声明**恢复点**（调用方自己维护、独立于本账本
//! journal 的进度标记——留档下界本身即门卫钟的退回值，任何仅从 journal 计算的量都无法覆盖
//! 「留档下界」与「重启前最后已知知情时」之间那段日志本就不知道的区间，恢复点因此只能来自
//! journal 之外）。声明恢复点后，任何早于它的知情时不得把仍处 Provisional 的身份落锤，一律
//! fail-loud 拒收 + 警报（[`super::RetraceRejection::SettleBehindRecoveryPoint`]）。未声明恢复点
//! （`fold`／活账 `new()`）零约束，本段声明对它们仍如实成立——护栏是叠加的可选层，不是对上一段
//! 声明的否定。
//!
//! # 边界输入即不信任输入
//!
//! JSONL 是外部边界：损坏的行、跳号、序列冲突一律转成 [`RetraceLogError`] 返回，不 panic
//! （内核 `settle` 的禁复活断言在折叠前由显式检查挡住）。**边界如实**（影子评审 #621 MEDIUM-3）：
//! fold 的语义校验只覆盖结构违规（跳号/终态再落锤/修订落未建仓身份）；**域语义违规**（同锚双注册/
//! Restart 前任悬空/快照载荷缺失）折叠时返回 `Ok`——前两类会在其后 `assert_invariants()` 判死，
//! 前任悬空一类可完全隐形。活路上三类均不可达（域侧守卫挡死），敞口仅限「JSONL 落盘后被外部
//! 改写」通道；fold 全覆盖校验上浮 S4 待裁。

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::ledger_kernel::{LedgerEntryCore, LedgerSettlement, LedgerState};
use super::super::level_view::CoordinateWindow;
use super::book::RetraceLedger;
use super::{
    NotConstitutedReason, RetraceEvidence, RetraceKey, RetraceObservation, RetraceRevision,
    RetraceRevisionKind,
};

/// 日志 wire schema 版本（升版即在 [`WireRecord`] 加分支，旧行不改）。
pub const LOG_SCHEMA_VERSION: u32 = 1;

/// 记录行标签。
const TAG_PROVENANCE: &str = "provenance";
/// 修订行标签。
const TAG_REVISION: &str = "revision";

/// FNV-1a 64 位偏置基（日志前缀指纹，快照溯源校验用）。
const LOG_DIGEST_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64 位质数。
const LOG_DIGEST_PRIME: u64 = 0x0000_0100_0000_01b3;

// ═══════════════════════════════════════════════════════════════════════════
// provenance / 记录 / 错误
// ═══════════════════════════════════════════════════════════════════════════

/// 日志头 provenance（裁定六：日志按 run/窗口分界，头部带窗口 + 数据基）。
///
/// **不承诺跨窗口身份连续**——换窗口即换日志。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetraceProvenance {
    /// 该账本所属递归级别。
    pub level: u32,
    /// 运行窗口（源坐标闭区间）。
    pub window: CoordinateWindow,
    /// 数据基标识（品种 / 周期 / 数据集版本，由调用方给定）。
    pub data_basis: String,
}

/// 一条日志记录 = 全局序号 + 内核修订（key / kind / **知情时** / 证据）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetraceRecord {
    pub sequence: u64,
    pub revision: RetraceRevision,
}

/// 日志层错误（边界输入不信任 ⟹ 一律 fail-loud 返回，不 panic）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetraceLogError {
    Io(String),
    InvalidJson {
        line: usize,
        message: String,
    },
    UnsupportedSchema {
        line: usize,
        schema: u64,
    },
    UnknownRecordTag {
        line: usize,
        tag: String,
    },
    /// 首行不是 provenance 头。
    MissingProvenanceHeader,
    ProvenanceMismatch {
        expected: RetraceProvenance,
        actual: RetraceProvenance,
    },
    /// 序号跳号（只追加纪律）。
    NonAppendSequence {
        expected: u64,
        actual: u64,
    },
    /// 同序号内容不一致（重放幂等的反面）。
    SequenceConflict {
        sequence: u64,
    },
    /// 记录语义不可折叠（建项落在已建仓身份、修订落在不存在的身份、终态后再落锤等）。
    FoldViolation {
        sequence: u64,
        detail: &'static str,
    },
    /// 折叠重跑内核原语产出的修订与记录不符 ⟹ 日志与内核语义漂移。
    ReplayDivergence {
        sequence: u64,
    },
}

impl From<std::io::Error> for RetraceLogError {
    fn from(value: std::io::Error) -> Self {
        RetraceLogError::Io(value.to_string())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 折叠恢复（状态 = 日志折叠）
// ═══════════════════════════════════════════════════════════════════════════

impl RetraceLedger {
    /// 全量重放折叠：`fold(log) = state`。
    pub fn fold(
        provenance: RetraceProvenance,
        records: &[RetraceRecord],
    ) -> Result<Self, RetraceLogError> {
        let mut ledger = Self::new(provenance);
        ledger.fold_tail(records)?;
        Ok(ledger)
    }

    /// 显式恢复：与 [`Self::fold`] 语义相同的全量重放折叠，额外记录调用方声明的**恢复点**
    /// （编排者 2026-07-29 裁方案②，票 #632；影子评审 #621 MEDIUM-1 修复；恢复点形状见模块头
    /// §「门卫钟的可恢复性」护栏后口径段）。
    ///
    /// `recovery_as_of` 由调用方独立维护（例如重放驱动自身的续跑断点），**不从本账本 journal
    /// 推导**——journal 能推导出的任何量都不超过留档下界，无法覆盖 MEDIUM-1 命中的那段「日志
    /// 本就不知道」的区间。声明后，任何早于 `recovery_as_of` 的知情时不得把仍处 Provisional 的
    /// 身份落锤（[`super::RetraceRejection::SettleBehindRecoveryPoint`]），门卫钟本身的退回语义
    /// 不受影响。未调用本函数（只用 [`Self::fold`]）的账本 `recovery_floor` 保持 `None`，护栏
    /// 零约束——本函数是纯增设，不改 [`Self::fold`] 的既有行为。
    pub fn fold_recovered(
        provenance: RetraceProvenance,
        records: &[RetraceRecord],
        recovery_as_of: usize,
    ) -> Result<Self, RetraceLogError> {
        let mut ledger = Self::fold(provenance, records)?;
        ledger.recovery_floor = Some(recovery_as_of);
        Ok(ledger)
    }

    pub(super) fn fold_tail(&mut self, records: &[RetraceRecord]) -> Result<(), RetraceLogError> {
        for record in records {
            self.apply_record(record)?;
        }
        Ok(())
    }

    /// 单条记录折叠：序号纪律 → 按词汇重跑内核原语 → 产出与记录逐字段比对 → 进日志。
    fn apply_record(&mut self, record: &RetraceRecord) -> Result<(), RetraceLogError> {
        let expected = self.journal.len() as u64;
        if record.sequence != expected {
            return Err(RetraceLogError::NonAppendSequence {
                expected,
                actual: record.sequence,
            });
        }
        let produced = match record.revision.kind {
            RetraceRevisionKind::Registered => self.replay_open(record)?,
            RetraceRevisionKind::SnapshotPinned | RetraceRevisionKind::Restarted { .. } => {
                self.replay_push(record)?
            }
            RetraceRevisionKind::Confirmed => self.replay_settle(record, None)?,
            RetraceRevisionKind::NotConstituted { reason } => {
                self.replay_settle(record, Some(reason))?
            }
        };
        if produced != record.revision {
            return Err(RetraceLogError::ReplayDivergence {
                sequence: record.sequence,
            });
        }
        self.recover_gate_clock(record);
        self.journal.push(*record);
        Ok(())
    }

    /// 门卫钟恢复：推到该条修订的知情时。
    ///
    /// 活路上每条修订的知情时都必然过过门卫钟（建项写、`admit` 推），故留档给出的最强下界
    /// 就是「最后一条修订的知情时」。无修订的未决观察推出的更高值不可恢复——见模块头
    /// §「门卫钟的可恢复性」。
    fn recover_gate_clock(&mut self, record: &RetraceRecord) {
        let entry = self
            .book
            .get_mut(&record.revision.key)
            .expect("刚折叠过的身份必在账");
        if entry.last_as_of() < record.revision.as_of {
            entry.set_last_as_of(record.revision.as_of);
        }
    }

    fn replay_open(&mut self, record: &RetraceRecord) -> Result<RetraceRevision, RetraceLogError> {
        let key = record.revision.key;
        let observation = RetraceObservation {
            key,
            outcome: None,
            as_of: record.revision.as_of,
        };
        let produced = self
            .book
            .open_on_observation(&observation, record.revision.as_of)
            .ok_or(RetraceLogError::FoldViolation {
                sequence: record.sequence,
                detail: "建项记录落在已建仓身份",
            })?;
        self.active_by_anchor.insert(key.anchor(), key);
        Ok(produced)
    }

    fn replay_push(&mut self, record: &RetraceRecord) -> Result<RetraceRevision, RetraceLogError> {
        let revision = record.revision;
        let entry = self
            .book
            .get_mut(&revision.key)
            .ok_or(RetraceLogError::FoldViolation {
                sequence: record.sequence,
                detail: "修订记录落在未建仓身份",
            })?;
        if entry.state.is_terminal() {
            return Err(RetraceLogError::FoldViolation {
                sequence: record.sequence,
                detail: "终态条目禁再追加非终态修订（禁复活）",
            });
        }
        Ok(entry.push_revision(revision.kind, revision.as_of, revision.evidence))
    }

    fn replay_settle(
        &mut self,
        record: &RetraceRecord,
        reason: Option<NotConstitutedReason>,
    ) -> Result<RetraceRevision, RetraceLogError> {
        let revision = record.revision;
        let state = match reason {
            None => LedgerState::Confirmed,
            Some(_) => LedgerState::Invalidated,
        };
        let entry = self
            .book
            .get_mut(&revision.key)
            .ok_or(RetraceLogError::FoldViolation {
                sequence: record.sequence,
                detail: "落锤记录落在未建仓身份",
            })?;
        if entry.state.is_terminal() {
            return Err(RetraceLogError::FoldViolation {
                sequence: record.sequence,
                detail: "终态条目禁再落锤（禁复活）",
            });
        }
        let settlement = LedgerSettlement {
            state,
            kind: revision.kind,
            reason,
            evidence: revision.evidence,
        };
        Ok(entry.settle(settlement, revision.as_of))
    }

    /// 不变量之一：`fold(journal)` 逐条复现账本（日志唯一真相，禁双真相漂移）。
    ///
    /// 门卫钟按 §「门卫钟的可恢复性」单独比对（日志给下界，非等式）。
    pub(super) fn assert_log_is_truth(&self) {
        let folded =
            Self::fold(self.provenance.clone(), &self.journal).expect("自产日志必可折叠回状态");
        assert!(
            folded.active_by_anchor == self.active_by_anchor,
            "fold(journal) 必须复现路由投影"
        );
        assert_eq!(
            folded.book.len(),
            self.book.len(),
            "fold(journal) 身份集相同"
        );
        for live in self.book.values() {
            let replayed = folded
                .book
                .get(&live.key)
                .expect("fold(journal) 必含同一身份");
            assert_identity_facts_match(live, replayed);
        }
    }
}

/// 日志承载的身份事实逐项等同；门卫钟只要求「重放值 = 留档下界 ≤ 活值」。
fn assert_identity_facts_match(live: &super::RetraceEntry, replayed: &super::RetraceEntry) {
    let key = live.key;
    assert_eq!(replayed.state, live.state, "三态：{key:?}");
    assert_eq!(replayed.revision, live.revision, "修订计数：{key:?}");
    assert!(replayed.revisions == live.revisions, "留档逐条：{key:?}");
    assert_eq!(
        replayed.registered_as_of, live.registered_as_of,
        "出生钟：{key:?}"
    );
    assert_eq!(
        replayed.terminal_as_of, live.terminal_as_of,
        "落锤钟：{key:?}"
    );
    assert_eq!(
        replayed.last_as_of,
        live.log_supported_gate(),
        "门卫钟重放值 = 留档下界：{key:?}"
    );
    assert!(
        replayed.last_as_of <= live.last_as_of,
        "门卫钟重放值不得超过活值：{key:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 派生缓存快照（裁定六：带溯源、校验失败即扔）
// ═══════════════════════════════════════════════════════════════════════════

/// 派生缓存快照：**永非真相**，只为省掉前缀重放。
#[derive(Debug, Clone)]
pub struct RetraceSnapshot {
    provenance: RetraceProvenance,
    /// 溯源①：折叠到第 N 条记录（N = 已折叠记录数）。
    folded_through: u64,
    /// 溯源②：该前缀的日志指纹。
    log_digest: u64,
    ledger: RetraceLedger,
}

impl RetraceSnapshot {
    pub fn provenance(&self) -> &RetraceProvenance {
        &self.provenance
    }

    pub fn folded_through(&self) -> u64 {
        self.folded_through
    }

    pub fn log_digest(&self) -> u64 {
        self.log_digest
    }
}

/// 缓存被拒的成因（诊断可查账）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRejection {
    /// provenance 不是同一条 run/窗口。
    ProvenanceMismatch,
    /// 缓存声称的前缀比日志还长。
    PrefixTooLong,
    /// 前缀指纹对不上（日志被改写/换轨）。
    DigestMismatch,
}

/// 恢复路径（**可观测**：AC 要求「缓存溯源校验失败回退重放」可验证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreRoute {
    /// 采纳缓存，只折叠尾段。
    CacheAdopted { folded_through: u64 },
    /// 全量重放；`cause = None` 表示压根没给缓存。
    FullReplay { cause: Option<SnapshotRejection> },
}

impl RetraceLedger {
    /// 取派生缓存快照（溯源 = 当下日志长度 + 全量指纹）。
    pub fn snapshot(&self) -> RetraceSnapshot {
        RetraceSnapshot {
            provenance: self.provenance.clone(),
            folded_through: self.journal.len() as u64,
            log_digest: digest_records(&self.journal),
            ledger: self.clone(),
        }
    }

    /// 恢复：缓存过溯源校验则采纳 + 折叠尾段，否则扔掉缓存全量重放。
    /// **本通道无恢复点护栏**（#632 L1 在案）：落锤护栏仅 `fold_recovered` 通道生效；
    /// 生产恢复路径若须护栏，改用 `fold_recovered` 或在接入层显式传恢复点（S4/消费方接入登记）。
    pub fn restore(
        provenance: RetraceProvenance,
        cache: Option<&RetraceSnapshot>,
        records: &[RetraceRecord],
    ) -> Result<(Self, RestoreRoute), RetraceLogError> {
        let Some(cache) = cache else {
            return Ok((
                Self::fold(provenance, records)?,
                RestoreRoute::FullReplay { cause: None },
            ));
        };
        if let Err(cause) = validate_cache(cache, &provenance, records) {
            return Ok((
                Self::fold(provenance, records)?,
                RestoreRoute::FullReplay { cause: Some(cause) },
            ));
        }
        let mut ledger = cache.ledger.clone();
        ledger.fold_tail(&records[cache.folded_through as usize..])?;
        Ok((
            ledger,
            RestoreRoute::CacheAdopted {
                folded_through: cache.folded_through,
            },
        ))
    }
}

fn validate_cache(
    cache: &RetraceSnapshot,
    provenance: &RetraceProvenance,
    records: &[RetraceRecord],
) -> Result<(), SnapshotRejection> {
    if cache.provenance != *provenance {
        return Err(SnapshotRejection::ProvenanceMismatch);
    }
    let prefix_len = cache.folded_through as usize;
    if prefix_len > records.len() {
        return Err(SnapshotRejection::PrefixTooLong);
    }
    if digest_records(&records[..prefix_len]) != cache.log_digest {
        return Err(SnapshotRejection::DigestMismatch);
    }
    Ok(())
}

/// 日志前缀指纹（FNV-1a over 规范 JSON 行）。
pub fn digest_records(records: &[RetraceRecord]) -> u64 {
    let mut hash = LOG_DIGEST_OFFSET_BASIS;
    for record in records {
        for byte in encode_record(record).as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(LOG_DIGEST_PRIME);
        }
    }
    hash
}

fn encode_record(record: &RetraceRecord) -> String {
    serde_json::to_string(&WireRecord::from_record(record)).expect("修订记录必可序列化")
}

// ═══════════════════════════════════════════════════════════════════════════
// JSONL 正式持久化边界（CompletedFreezeReducer 先例）
// ═══════════════════════════════════════════════════════════════════════════

/// 只追加的 JSONL 日志文件：首行 provenance 头，其后逐条修订记录。
#[derive(Debug, Clone)]
pub struct JsonlRetraceLogStore {
    path: PathBuf,
}

impl JsonlRetraceLogStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一段记录；文件为空时先落 provenance 头行。**从不 truncate/seek/重写旧字节。**
    pub fn append_all(
        &self,
        provenance: &RetraceProvenance,
        records: &[RetraceRecord],
    ) -> Result<(), RetraceLogError> {
        let mut lines = Vec::new();
        if !self.path.exists() || std::fs::metadata(&self.path)?.len() == 0 {
            lines.push(
                serde_json::to_string(&WireHeader::from_provenance(provenance))
                    .map_err(|error| RetraceLogError::Io(error.to_string()))?,
            );
        }
        for record in records {
            lines.push(encode_record(record));
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        for line in lines {
            file.write_all(format!("{line}\n").as_bytes())?;
        }
        file.sync_data()?;
        Ok(())
    }

    /// 读回：头行 provenance 必须匹配；记录只追加、同序号重复须逐字段相同（幂等）。
    pub fn load(
        &self,
        expected: &RetraceProvenance,
    ) -> Result<Vec<RetraceRecord>, RetraceLogError> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut records: Vec<RetraceRecord> = Vec::new();
        let mut header_seen = false;
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value = parse_line(&line, index + 1)?;
            match classify(&value, index + 1)? {
                WireLine::Header(header) => {
                    header.check(expected)?;
                    header_seen = true;
                }
                WireLine::Record(record) => {
                    if !header_seen {
                        return Err(RetraceLogError::MissingProvenanceHeader);
                    }
                    absorb(&mut records, record)?;
                }
            }
        }
        if !header_seen && !records.is_empty() {
            return Err(RetraceLogError::MissingProvenanceHeader);
        }
        Ok(records)
    }
}

/// 重复行幂等吸收：同序号同内容 ⟹ 跳过；同序号异内容 ⟹ 冲突拒绝；跳号 ⟹ 拒绝。
fn absorb(records: &mut Vec<RetraceRecord>, record: RetraceRecord) -> Result<(), RetraceLogError> {
    let expected = records.len() as u64;
    if record.sequence < expected {
        let existing = records[record.sequence as usize];
        return if existing == record {
            Ok(())
        } else {
            Err(RetraceLogError::SequenceConflict {
                sequence: record.sequence,
            })
        };
    }
    if record.sequence != expected {
        return Err(RetraceLogError::NonAppendSequence {
            expected,
            actual: record.sequence,
        });
    }
    records.push(record);
    Ok(())
}

fn parse_line(line: &str, number: usize) -> Result<serde_json::Value, RetraceLogError> {
    serde_json::from_str(line).map_err(|error| RetraceLogError::InvalidJson {
        line: number,
        message: error.to_string(),
    })
}

enum WireLine {
    Header(WireHeader),
    Record(RetraceRecord),
}

fn classify(value: &serde_json::Value, number: usize) -> Result<WireLine, RetraceLogError> {
    let schema = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if schema != u64::from(LOG_SCHEMA_VERSION) {
        return Err(RetraceLogError::UnsupportedSchema {
            line: number,
            schema,
        });
    }
    let tag = value
        .get("record")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let decode = |message: String| RetraceLogError::InvalidJson {
        line: number,
        message,
    };
    match tag.as_str() {
        TAG_PROVENANCE => serde_json::from_value::<WireHeader>(value.clone())
            .map(WireLine::Header)
            .map_err(|error| decode(error.to_string())),
        TAG_REVISION => serde_json::from_value::<WireRecord>(value.clone())
            .map(|wire| WireLine::Record(wire.into_record()))
            .map_err(|error| decode(error.to_string())),
        _ => Err(RetraceLogError::UnknownRecordTag { line: number, tag }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// wire 形状（字段名即 schema，升版走 schema_version）
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireHeader {
    schema_version: u32,
    record: String,
    level: u32,
    window_start: usize,
    window_end: usize,
    data_basis: String,
}

impl WireHeader {
    fn from_provenance(provenance: &RetraceProvenance) -> Self {
        Self {
            schema_version: LOG_SCHEMA_VERSION,
            record: TAG_PROVENANCE.to_owned(),
            level: provenance.level,
            window_start: provenance.window.start,
            window_end: provenance.window.end,
            data_basis: provenance.data_basis.clone(),
        }
    }

    fn check(&self, expected: &RetraceProvenance) -> Result<(), RetraceLogError> {
        let actual = RetraceProvenance {
            level: self.level,
            window: CoordinateWindow {
                start: self.window_start,
                end: self.window_end,
            },
            data_basis: self.data_basis.clone(),
        };
        if actual == *expected {
            return Ok(());
        }
        Err(RetraceLogError::ProvenanceMismatch {
            expected: expected.clone(),
            actual,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireRecord {
    schema_version: u32,
    record: String,
    sequence: u64,
    key: RetraceKey,
    kind: RetraceRevisionKind,
    as_of: usize,
    evidence: Option<RetraceEvidence>,
}

impl WireRecord {
    fn from_record(record: &RetraceRecord) -> Self {
        Self {
            schema_version: LOG_SCHEMA_VERSION,
            record: TAG_REVISION.to_owned(),
            sequence: record.sequence,
            key: record.revision.key,
            kind: record.revision.kind,
            as_of: record.revision.as_of,
            evidence: record.revision.evidence,
        }
    }

    fn into_record(self) -> RetraceRecord {
        RetraceRecord {
            sequence: self.sequence,
            revision: RetraceRevision {
                key: self.key,
                kind: self.kind,
                as_of: self.as_of,
                evidence: self.evidence,
            },
        }
    }
}
