//! Audit 流（票 #622 S2；#574 契约裁定六）：四类警报统一 append-only 落盘。
//!
//! # 四类警报
//!
//! 与 [`super::book::RetraceAlarms`] 的四个计数字段一一对应：残废拒收（含单活跃候选纪律 + 两拍
//! 证据一致性守卫 + 改口处死知情时护栏，票 #622 S2）/ 死人挂号拒收 / 门卫钟倒退拒收 / 终态后
//! 迟到静默吸收。本模块只多做一件事——
//! 把同一批事件序列化为 append-only JSONL（`CompletedFreezeReducer` 先例：只追加、重放幂等、
//! 冲突拒绝），供离线审计。
//!
//! # 审计归审计，不进真相恢复路径
//!
//! [`super::book::RetraceLedger::fold`] 的签名只吃 `&[RetraceRecord]`（修订日志）——audit 流从不
//! 是它的入参，结构上不可能参与折叠。篡改 audit 流不改变任何一条 `fold(journal)` 的产出，因为
//! 折叠函数压根不读它。四类事件本身也不改账本状态（它们是**未被采纳的输入**的记录：拒收=零改写，
//! 倒退拒收=零改写，迟到吸收=零改写）——真正改状态的事件（注册/判胜/判败/改口处死/Restart）
//! 一律走 [`super::log`] 的日志本体，不重复进本流。
//!
//! # 与 [`RetraceRejection`](super::RetraceRejection) 的关系
//!
//! 本流不直接内嵌完整的 [`super::RetraceRejection`] 值（它携带 `Direction` 等未实现
//! `Serialize`/`Deserialize` 的核心类型，且强行序列化会让 wire 格式绑死内部错误类型的字段布局）。
//! [`RetraceRejectionCode`] 是审计只需要的轻量标签——分类，不是完整证据；完整证据仍在
//! `observe()` 的 `Result::Err` 里即时可得，本流不重复搬运。

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::log::RetraceProvenance;
use super::{CenterAnchor, RetraceKey};

/// Audit 流 wire schema 版本（升版即在 wire 类型加分支，旧行不改）。
pub const AUDIT_SCHEMA_VERSION: u32 = 1;

/// 记录行标签。
const TAG_PROVENANCE: &str = "provenance";
/// 事件行标签。
const TAG_AUDIT: &str = "audit";

// ═══════════════════════════════════════════════════════════════════════════
// 事件词汇
// ═══════════════════════════════════════════════════════════════════════════

/// 注册期拒收细分码（[`super::RetraceRejection`] 的轻量标签）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetraceRejectionCode {
    MalformedFrame,
    MissingRetestPosition,
    LeaveNotOutsideCenter,
    RetestSameDirection,
    NotAdjacent,
    ActiveCandidateNotSettled,
    TerminalEvidenceContradictsRegistration,
    RebaseAsOfBehindGate,
}

/// 四类警报事件（裁定六）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RetraceAuditEvent {
    /// 残废拒收（四桶 + 单活跃候选纪律 + 两拍证据一致性守卫 + 改口处死知情时护栏，票 #622 S2）。
    ResidualRejected {
        as_of: usize,
        code: RetraceRejectionCode,
    },
    /// 死人挂号拒收（裁定四：已死中枢新 departure）。
    DeadCenterRejected {
        anchor: CenterAnchor,
        attempted: RetraceKey,
        /// 死亡证明开具时间（供审计核对拒收是否合理，不重复携带完整证书结构）。
        death_issued_as_of: usize,
        as_of: usize,
    },
    /// 门卫钟倒退拒收。
    RetrogradeRejected {
        key: RetraceKey,
        last_as_of: usize,
        rejected_as_of: usize,
    },
    /// 终态后同身份迟到输入静默吸收。
    LateAbsorbed {
        key: RetraceKey,
        terminal_as_of: usize,
        late_as_of: usize,
    },
}

/// 一条 audit 记录 = 全局序号 + 事件（append-only，序号即产生序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceAuditRecord {
    pub sequence: u64,
    pub event: RetraceAuditEvent,
}

/// Audit 层错误（边界输入不信任 ⟹ fail-loud 返回，不 panic）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetraceAuditError {
    Io(String),
    InvalidJson { line: usize, message: String },
    UnsupportedSchema { line: usize, schema: u64 },
    UnknownRecordTag { line: usize, tag: String },
    MissingProvenanceHeader,
    ProvenanceMismatch {
        expected: RetraceProvenance,
        actual: RetraceProvenance,
    },
    NonAppendSequence { expected: u64, actual: u64 },
    SequenceConflict { sequence: u64 },
}

impl From<std::io::Error> for RetraceAuditError {
    fn from(value: std::io::Error) -> Self {
        RetraceAuditError::Io(value.to_string())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// JSONL 正式持久化边界（CompletedFreezeReducer 先例，对标 log.rs）
// ═══════════════════════════════════════════════════════════════════════════

/// 只追加的 JSONL audit 文件：首行 provenance 头，其后逐条事件记录。
#[derive(Debug, Clone)]
pub struct JsonlRetraceAuditStore {
    path: PathBuf,
}

impl JsonlRetraceAuditStore {
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
        records: &[RetraceAuditRecord],
    ) -> Result<(), RetraceAuditError> {
        let mut lines = Vec::new();
        if !self.path.exists() || std::fs::metadata(&self.path)?.len() == 0 {
            lines.push(
                serde_json::to_string(&AuditWireHeader::from_provenance(provenance))
                    .map_err(|error| RetraceAuditError::Io(error.to_string()))?,
            );
        }
        for record in records {
            lines.push(encode_record(record));
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
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
    ) -> Result<Vec<RetraceAuditRecord>, RetraceAuditError> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut records: Vec<RetraceAuditRecord> = Vec::new();
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
                        return Err(RetraceAuditError::MissingProvenanceHeader);
                    }
                    absorb(&mut records, record)?;
                }
            }
        }
        if !header_seen && !records.is_empty() {
            return Err(RetraceAuditError::MissingProvenanceHeader);
        }
        Ok(records)
    }
}

/// 重复行幂等吸收：同序号同内容 ⟹ 跳过；同序号异内容 ⟹ 冲突拒绝；跳号 ⟹ 拒绝。
fn absorb(
    records: &mut Vec<RetraceAuditRecord>,
    record: RetraceAuditRecord,
) -> Result<(), RetraceAuditError> {
    let expected = records.len() as u64;
    if record.sequence < expected {
        let existing = records[record.sequence as usize];
        return if existing == record {
            Ok(())
        } else {
            Err(RetraceAuditError::SequenceConflict {
                sequence: record.sequence,
            })
        };
    }
    if record.sequence != expected {
        return Err(RetraceAuditError::NonAppendSequence {
            expected,
            actual: record.sequence,
        });
    }
    records.push(record);
    Ok(())
}

fn parse_line(line: &str, number: usize) -> Result<serde_json::Value, RetraceAuditError> {
    serde_json::from_str(line).map_err(|error| RetraceAuditError::InvalidJson {
        line: number,
        message: error.to_string(),
    })
}

enum WireLine {
    Header(AuditWireHeader),
    Record(RetraceAuditRecord),
}

fn classify(value: &serde_json::Value, number: usize) -> Result<WireLine, RetraceAuditError> {
    let schema = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if schema != u64::from(AUDIT_SCHEMA_VERSION) {
        return Err(RetraceAuditError::UnsupportedSchema {
            line: number,
            schema,
        });
    }
    let tag = value
        .get("record")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let decode = |message: String| RetraceAuditError::InvalidJson {
        line: number,
        message,
    };
    match tag.as_str() {
        TAG_PROVENANCE => serde_json::from_value::<AuditWireHeader>(value.clone())
            .map(WireLine::Header)
            .map_err(|error| decode(error.to_string())),
        TAG_AUDIT => serde_json::from_value::<AuditWireRecord>(value.clone())
            .map(|wire| WireLine::Record(wire.into_record()))
            .map_err(|error| decode(error.to_string())),
        _ => Err(RetraceAuditError::UnknownRecordTag { line: number, tag }),
    }
}

fn encode_record(record: &RetraceAuditRecord) -> String {
    serde_json::to_string(&AuditWireRecord::from_record(record)).expect("audit 记录必可序列化")
}

// ═══════════════════════════════════════════════════════════════════════════
// wire 形状（字段名即 schema，升版走 schema_version；对标 log.rs）
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditWireHeader {
    schema_version: u32,
    record: String,
    level: u32,
    window_start: usize,
    window_end: usize,
    data_basis: String,
}

impl AuditWireHeader {
    fn from_provenance(provenance: &RetraceProvenance) -> Self {
        Self {
            schema_version: AUDIT_SCHEMA_VERSION,
            record: TAG_PROVENANCE.to_owned(),
            level: provenance.level,
            window_start: provenance.window.start,
            window_end: provenance.window.end,
            data_basis: provenance.data_basis.clone(),
        }
    }

    fn check(&self, expected: &RetraceProvenance) -> Result<(), RetraceAuditError> {
        let actual = RetraceProvenance {
            level: self.level,
            window: super::super::level_view::CoordinateWindow {
                start: self.window_start,
                end: self.window_end,
            },
            data_basis: self.data_basis.clone(),
        };
        if actual == *expected {
            return Ok(());
        }
        Err(RetraceAuditError::ProvenanceMismatch {
            expected: expected.clone(),
            actual,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditWireRecord {
    schema_version: u32,
    record: String,
    sequence: u64,
    #[serde(flatten)]
    event: RetraceAuditEvent,
}

impl AuditWireRecord {
    fn from_record(record: &RetraceAuditRecord) -> Self {
        Self {
            schema_version: AUDIT_SCHEMA_VERSION,
            record: TAG_AUDIT.to_owned(),
            sequence: record.sequence,
            event: record.event,
        }
    }

    fn into_record(self) -> RetraceAuditRecord {
        RetraceAuditRecord {
            sequence: self.sequence,
            event: self.event,
        }
    }
}
