//! #1394：S3一致typed捕获与原Q capture_digest；离开源事务后复用完整S3审计。
//! 只重建项目固定DDL，摘要保留源schema原文；不执行来源SQL、不持有跨请求S读锁。
//! 配置读取入口绑定Q v3资源文件；每worker长期reader在同连接版本夹持下复用完整私有缓存。
use super::business_json::{self as bj, TextValue};
use super::*;
use newchan_rust::s_content as sc;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};
use newchan_rust::sqlite_checked::{CheckedRow, CheckedStatement};
#[cfg(test)]
use rusqlite::types::Value as Cell;
use rusqlite::{limits::Limit, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    mem::size_of,
    os::unix::fs::MetadataExt,
    time::{Duration, Instant},
};
#[path = "q_retained.rs"]
mod retained;

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub max_capture_bytes: u64,
    pub capture_deadline_ms: u64,
    pub verify_deadline_ms: u64,
}
impl Limits {
    fn validate(self) -> Result<(), String> {
        for n in [
            self.max_capture_bytes,
            self.capture_deadline_ms,
            self.verify_deadline_ms,
        ] {
            if n == 0 || n > newchan_rust::s_content::MAX_FRAME_LEN {
                return Err(err("捕获/核验限制必须显式为规范正整数"));
            }
        }
        Ok(())
    }
}

enum CaptureCell {
    Null,
    Integer(i64),
    Text(TextValue),
    Blob(CodecVec<u8>),
}
impl CaptureCell {
    fn view(&self) -> CellView<'_> {
        match self {
            Self::Null => CellView::Null,
            Self::Integer(n) => CellView::Integer(*n),
            Self::Text(v) => CellView::Text(v.as_str()),
            Self::Blob(v) => CellView::Blob(v),
        }
    }
    fn capacity(&self) -> usize {
        match self {
            Self::Text(v) => v.capacity(),
            Self::Blob(v) => v.capacity(),
            _ => 0,
        }
    }
}
enum CellView<'a> {
    Null,
    Integer(i64),
    Text(&'a str),
    Blob(&'a [u8]),
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
    Real,
}
struct Table {
    name: TextValue,
    columns: CodecVec<TextValue>,
    rows: CodecVec<CodecVec<CaptureCell>>,
    // 私有复制信息：现有独立索引核首次插入序；不进入原Q摘要配方。
    rowids: Option<CodecVec<i64>>,
}
struct Image {
    header: CodecVec<u8>,
    tables: CodecVec<Table>,
    ddls: CodecVec<&'static str>,
    digest: CaptureDigest,
    captured_bytes: u64,
    allocation: Option<super::q_allocation::Profile>,
}

/// 只有完整state::audit通过才构造；数据库句柄不向调用者开放写面。
pub(super) struct VerifiedCapture {
    db: Connection,
    audited: state::AuditedState,
    digest: CaptureDigest,
    pub captured_bytes: u64,
    pub source_versions: [i64; 3],
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
    verify_deadline: Instant,
    audit_receipt: Option<audit_budget::Receipt>,
    q_source: Option<newchan_rust::s_content::AuditCapture>,
    allocation: Option<super::q_allocation::Profile>,
}
/// #1394：已计算捕获摘要采用固定规范十六进制，不为长期持有与借出再分配字符串。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct CaptureDigest([u8; 64]);
impl CaptureDigest {
    fn from_str(value: &str) -> Result<Self, String> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(err("捕获摘要不是规范64位十六进制"));
        }
        let mut bytes = [0; 64];
        bytes.copy_from_slice(value.as_bytes());
        Ok(Self(bytes))
    }
    pub(super) fn as_str(&self) -> &str {
        // 所有构造均核过ASCII；数组不开放修改。
        std::str::from_utf8(&self.0).expect("规范捕获摘要ASCII")
    }
    pub(super) fn capacity(&self) -> usize {
        0
    }
}
impl PartialEq<String> for CaptureDigest {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}
impl PartialEq<str> for CaptureDigest {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl std::ops::Deref for CaptureDigest {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Display for CaptureDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl VerifiedCapture {
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
    pub(super) fn generation(&self) -> i64 {
        self.audited
            .cuts
            .last()
            .map_or(0, |c| c.metadata.generation)
    }
    pub(super) fn projection(
        &mut self,
        generation: i64,
        mode: projection::HistoryMode,
        max_metadata_bytes: u64,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<projection::Projection, String> {
        let tx = self.db.transaction().map_err(err)?;
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let deadline = job.read_deadline().map_err(err)?;
        let mut graph = history::HashReader::for_audited(
            &mut source,
            &self.audited,
            self.q_source.as_ref(),
            self.allocation.as_ref(),
            deadline,
            None,
        )
        .map_err(err)?;
        projection::build(
            &tx,
            &self.audited,
            &mut graph,
            generation,
            mode,
            max_metadata_bytes,
            job,
        )
    }
}

#[cfg(test)]
pub(super) fn capture_verified(path: &Path, limits: Limits) -> Result<VerifiedCapture, String> {
    capture_verified_with_allocation(path, limits, None)
}
#[cfg(test)]
pub(super) fn capture_verified_with_allocation(
    path: &Path,
    limits: Limits,
    allocation: Option<super::q_allocation::Profile>,
) -> Result<VerifiedCapture, String> {
    capture_with_deadline_and_access(path, limits, None, |_| {}, None, allocation)
}
/// 当前一次捕获绑定同一份冻结资源，后续投影/Delta读取不能随请求改限额。
pub(super) struct ConfiguredCapture<'a> {
    capture: VerifiedCapture,
    resources: &'a q_resources::QResources,
}
#[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
pub(super) struct DeltaReferences {
    pub anchors: CodecVec<bj::JsonOwner>,
    pub usage: CodecVec<q_delta::Usage>,
    pub through_generation: i64,
    pub has_more: bool,
    pub validated_capture_digest: CaptureDigest,
}
impl std::fmt::Debug for DeltaReferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeltaReferences")
            .field("anchors", &self.anchors.len())
            .field("usage", &&self.usage[..])
            .field("through_generation", &self.through_generation)
            .field("has_more", &self.has_more)
            .field("validated_capture_digest", &self.validated_capture_digest)
            .finish()
    }
}
fn measure_delta(
    graph: &mut history::HashReader<'_>,
    delta: &audit_store::RefValue,
    resources: &q_resources::QResources,
    job: &mut dyn newchan_rust::s_content::access::Access,
) -> Result<q_delta::Usage, String> {
    if let Some(owner) = graph.closed_owner_mut().map_err(err)? {
        q_delta::measure_closed(owner, delta.view(), resources, job)
    } else {
        q_delta::measure(graph, delta.view(), resources, job)
    }
}
impl ConfiguredCapture<'_> {
    pub(crate) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.capture.allocation.as_ref().map(|p| &p.budget)
    }
    /// 只从本次完整已核副本取错误回复的身份，不借请求自称已核。
    pub(super) fn identity(&self) -> Result<(TextValue, TextValue), String> {
        let owner = self.capture.allocation.as_ref().map(|p| &p.budget);
        let session = v2::control_meta_text(&self.capture.db, "session_id", owner)?
            .ok_or_else(|| err("已核捕获缺会话身份"))?;
        let (_, incarnation, _) = v2::protocol_identity_with_owner(&self.capture.db, owner)?;
        Ok((session, incarnation))
    }
    /// 同一完整audit后的借用字段构造ready回执，Owned不会擦为Value。
    pub(super) fn ready_cut(&self) -> Result<bj::JsonOwner, String> {
        use newchan_rust::s_content::capture_allocation::audit_json::JsonView;
        let (session, incarnation) = self.identity()?;
        let owner = self.capture.allocation.as_ref().map(|p| &p.budget);
        let generation = self.capture.generation();
        let catalog = v2::control_meta_text(&self.capture.db, "catalog_revision", owner)?
            .ok_or_else(|| err("已核捕获缺目录版本"))?;
        let index = self
            .capture
            .audited
            .cuts
            .last()
            .map_or("", |cut| cut.batch_id.as_str());
        let mut scalar = [0u8; 20];
        let gen = std::str::from_utf8(decimal_bytes(generation, &mut scalar)).expect("ASCII整数");
        let mut cut = bj::new_work(owner);
        cut.try_extend_bytes(b"cut-").map_err(|e| e.to_string())?;
        cut.try_extend_bytes(gen.as_bytes())
            .map_err(|e| e.to_string())?;
        let cut = TextValue::from_bytes(cut)?;
        struct Ready<'a> {
            fields: [(&'static str, JsonView<'a>); 6],
        }
        impl newchan_rust::s_content::capture_allocation::audit_json::JsonObject for Ready<'_> {
            fn len(&self) -> usize {
                6
            }
            fn field(&self, i: usize) -> Option<(&str, JsonView<'_>)> {
                self.fields.get(i).copied()
            }
        }
        let ready = Ready {
            fields: [
                ("session_id", JsonView::String(session.as_str())),
                ("session_generation", JsonView::String(incarnation.as_str())),
                ("generation", JsonView::String(gen)),
                ("structure_cut", JsonView::String(cut.as_str())),
                ("catalog_revision", JsonView::String(catalog.as_str())),
                ("index_frontier", JsonView::String(index)),
            ],
        };
        bj::json_from_view(JsonView::borrowed_object(&ready).map_err(err)?, owner)
    }
    pub(super) fn watch(
        &mut self,
        request: &q_watch::Request,
        leases: &q_lease::ClientLeases,
        epoch: &str,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<q_watch::Reply, String> {
        request.validate_resources(self.resources)?;
        let current = self.projection(
            self.capture.generation(),
            projection::HistoryMode::RecomputedWithRevision,
            job,
        )?;
        let deadline = job.read_deadline().map_err(err)?;
        let tx = self.capture.db.transaction().map_err(err)?;
        let mut stmt=newchan_rust::sqlite_checked::CheckedStatement::prepare(&tx,c"SELECT policy_revision,retain_generations,first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1").map_err(err)?;
        let mut row = stmt
            .next()
            .map_err(err)?
            .ok_or("QWatchInvalid：缺delivery policy")?;
        let policy = q_watch::Policy {
            revision: TextValue::from_sql(
                &mut row,
                0,
                self.capture.allocation.as_ref().map(|p| &p.budget),
            )
            .map_err(err)?,
            retain: row.integer(1).map_err(err)?,
            first: row.integer(2).map_err(err)?,
            head: row.integer(3).map_err(err)?,
        };
        drop(row);
        stmt.finalize().map_err(err)?;
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let mut graph = history::HashReader::for_audited(
            &mut source,
            &self.capture.audited,
            self.capture.q_source.as_ref(),
            self.capture.allocation.as_ref(),
            deadline,
            Some(self.resources.limit(q_resources::Limit::CaptureBytes)),
        )
        .map_err(err)?;
        if self.capture.allocation.is_some() {
            q_watch::prepare_owned(
                request,
                current,
                &self.capture.audited.cuts,
                &mut graph,
                policy,
                &self.capture.digest,
                self.resources,
                leases,
                epoch,
                job,
            )
        } else {
            q_watch::prepare(
                request,
                current,
                &self.capture.audited.cuts,
                &mut graph,
                policy,
                &self.capture.digest,
                self.resources,
                leases,
                epoch,
                job,
            )
        }
    }
    pub(super) fn content(
        &mut self,
        request: &q_content::Request,
        producer_epoch: &str,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<CodecVec<u8>, String> {
        use q_resources::Limit;
        request.validate_resources(self.resources)?;
        match (
            self.capture.allocation.as_ref(),
            request.allocation_budget()?,
        ) {
            (None, None) => {}
            (Some(profile), Some(input)) if profile.budget.same_ledger(input) => {}
            _ => {
                return Err(err(
                    "prototype_audit_allocation_incomplete：Content请求与实际捕获原账不同",
                ))
            }
        }
        let deadline = job.read_deadline().map_err(err)?;
        let tx = self.capture.db.transaction().map_err(sql_err)?;
        let owner = self.capture.allocation.as_ref().map(|p| &p.budget);
        let session =
            v2::control_meta_text(&tx, "session_id", owner)?.ok_or_else(|| err("缺会话身份"))?;
        let (_, incarnation, _) = v2::protocol_identity_with_owner(&tx, owner)?;
        request.validate_identity(&session, &incarnation)?;
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let mut graph = history::HashReader::for_audited(
            &mut source,
            &self.capture.audited,
            self.capture.q_source.as_ref(),
            self.capture.allocation.as_ref(),
            deadline,
            Some(self.resources.limit(Limit::CaptureBytes)),
        )
        .map_err(err)?;
        let root = match request.anchor_kind() {
            q_content::Anchor::Projection { generation, mode } => {
                let actual = projection::build(
                    &tx,
                    &self.capture.audited,
                    &mut graph,
                    *generation,
                    *mode,
                    self.resources.limit(Limit::CaptureBytes),
                    job,
                )?;
                reader::check_projection_anchor_view(&actual, request.anchor_view())?;
                actual.root
            }
            q_content::Anchor::Delta => {
                let cut = reader::validated_delta_view(
                    &tx,
                    &self.capture.audited,
                    request.anchor_view(),
                )?;
                // Content回复也包含Delta引用，不能借片段路径绕过完整闭包的三原子门。
                measure_delta(&mut graph, &cut.delta, self.resources, job)?;
                cut.delta.clone()
            }
        };
        if self.capture.allocation.is_some() {
            let owner = graph.closed_owner_mut().map_err(err)?.ok_or_else(|| {
                err("prototype_audit_allocation_incomplete：owned捕获缺实际Closed来源")
            })?;
            let fragment = request.read_owned(owner, root.view(), self.resources, job)?;
            request.reply_owned(
                fragment,
                &self.capture.digest,
                &session,
                &incarnation,
                producer_epoch,
                self.resources,
                job,
            )
        } else {
            let fragment = request.read(&mut graph, root.view(), self.resources, job)?;
            request
                .reply(
                    fragment,
                    &self.capture.digest,
                    &session,
                    &incarnation,
                    producer_epoch,
                    self.resources,
                    job,
                )
                .map(CodecVec::from_plain)
        }
    }
    pub(super) fn snapshot(
        &mut self,
        request: &q_snapshot::Request,
        producer_epoch: &str,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<sc::CodecVec<u8>, String> {
        request.validate_resources(self.resources)?;
        let generation = request.generation(self.capture.generation());
        let projection = self.projection(generation, request.history_mode(), job)?;
        match self.capture.allocation.as_ref() {
            Some(p) => q_snapshot::reply_owned(
                request,
                projection,
                self.capture.digest(),
                self.resources,
                producer_epoch,
                job,
                &p.budget,
            ),
            None => q_snapshot::reply(
                request,
                projection,
                self.capture.digest(),
                self.resources,
                producer_epoch,
                job,
            )
            .map(sc::CodecVec::from_plain),
        }
    }
    pub(super) fn projection(
        &mut self,
        generation: i64,
        mode: projection::HistoryMode,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<projection::Projection, String> {
        self.capture.projection(
            generation,
            mode,
            self.resources.limit(q_resources::Limit::CaptureBytes),
            job,
        )
    }
    /// 为Watch组装准备一段连续候选；任一完整Delta超限则整个调用失败，没有可交付前缀。
    /// 不是Watch wire响应：实际回复字节、cursor身份、lease和发送后推进仍由服务层承担。
    #[cfg(test)]
    pub(super) fn delta_references(
        &mut self,
        after: i64,
        max_batches: u64,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<DeltaReferences, String> {
        use q_resources::Limit;
        let head = self.capture.generation();
        if after < 0 || after > head {
            return Err(err("cursor代号越过已核永久发布历史"));
        }
        if max_batches == 0 || max_batches > self.resources.limit(Limit::WatchBatches) {
            return Err(err("Incomplete：Watch批数越过冻结上限"));
        }
        job.begin_first_read().map_err(err)?;
        let tx = self.capture.db.transaction().map_err(sql_err)?;
        let first: i64 = tx
            .query_row(
                "SELECT first_available_generation FROM s_delivery_policy WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        if after < first - 1 {
            return Err(err(
                "Incomplete/ContentAnchorUnavailable：Delta已退出保留窗口，须沿Watch Gap重建",
            ));
        }
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let allocation = self.capture.allocation.as_ref();
        let owner = allocation.map(|p| &p.budget);
        let mut graph = history::HashReader::for_audited(
            &mut source,
            &self.capture.audited,
            self.capture.q_source.as_ref(),
            allocation,
            job.read_deadline().map_err(err)?,
            Some(self.resources.limit(Limit::CaptureBytes)),
        )
        .map_err(err)?;
        let mut anchors = bj::new_work(owner);
        let mut usage = bj::new_work(owner);
        let mut through = after;
        for cut in self
            .capture
            .audited
            .cuts
            .iter()
            .filter(|c| c.metadata.generation > after)
        {
            if anchors.len() as u64 >= max_batches {
                break;
            }
            usage
                .try_push(measure_delta(&mut graph, &cut.delta, self.resources, job)?)
                .map_err(|e| e.to_string())?;
            let anchor = reader::delta_anchor_view(cut);
            anchors
                .try_push(bj::json_from_view(anchor.view(), owner)?)
                .map_err(|e| e.to_string())?;
            through = cut.metadata.generation;
        }
        job.pre_io_check().map_err(err)?;
        Ok(DeltaReferences {
            anchors,
            usage,
            through_generation: through,
            has_more: through < head,
            validated_capture_digest: self.capture.digest,
        })
    }
}

#[cfg(test)]
pub(super) fn capture_configured<'a>(
    path: &Path,
    resources: &'a q_resources::QResources,
    job: &mut dyn newchan_rust::s_content::access::Access,
) -> Result<ConfiguredCapture<'a>, String> {
    let deadline = job.read_deadline().map_err(err)?;
    job.charge_requests(1).map_err(err)?;
    let result = capture_with_deadline_and_access(
        path,
        resources.capture_limits(),
        deadline,
        |_| {},
        Some(job),
        None,
    );
    // 失败也复查整job绝对钟，不能通过开始下一捕获重置期限。
    job.pre_io_check().map_err(err)?;
    let capture = result?;
    let verify_deadline = capture.verify_deadline;
    let mut packed = retained::Packed::new(
        capture,
        resources.limit(q_resources::Limit::CaptureBytes),
        verify_deadline,
    )?;
    let capture = packed.checkout(verify_deadline)?;
    job.pre_io_check().map_err(err)?;
    Ok(ConfiguredCapture { capture, resources })
}
#[cfg(test)]
fn capture_with(
    path: &Path,
    limits: Limits,
    seam: impl FnMut(&str),
) -> Result<VerifiedCapture, String> {
    capture_with_deadline(path, limits, None, seam)
}
#[cfg(test)]
fn capture_with_deadline(
    path: &Path,
    limits: Limits,
    hard_deadline: Option<Instant>,
    seam: impl FnMut(&str),
) -> Result<VerifiedCapture, String> {
    capture_with_deadline_and_access(path, limits, hard_deadline, seam, None, None)
}
#[cfg(test)]
fn capture_with_deadline_and_access(
    path: &Path,
    limits: Limits,
    hard_deadline: Option<Instant>,
    mut seam: impl FnMut(&str),
    job: Option<&mut dyn newchan_rust::s_content::access::Access>,
    allocation: Option<super::q_allocation::Profile>,
) -> Result<VerifiedCapture, String> {
    limits.validate()?;
    if let Some(profile) = &allocation {
        profile.validate()?;
    }
    let owner = allocation.as_ref().map(|p| &p.budget);
    let capture_deadline = until(limits.capture_deadline_ms)?;
    let deadline = hard_deadline.map_or(capture_deadline, |d| d.min(capture_deadline));
    check(deadline, "Capture")?;
    let identity = file_identity(path)?;
    let mut source = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sql_err)?;
    source
        .busy_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(sql_err)?;
    source
        .set_limit(
            Limit::SQLITE_LIMIT_LENGTH,
            limits.max_capture_bytes.min(i32::MAX as u64) as i32,
        )
        .map_err(sql_err)?;
    source
        .pragma_update(None, "query_only", true)
        .map_err(sql_err)?;
    source
        .pragma_update(None, "read_uncommitted", false)
        .map_err(sql_err)?;
    source
        .progress_handler(100, Some(move || Instant::now() >= deadline))
        .map_err(sql_err)?;
    let captured = (|| {
        check(deadline, "Capture")?;
        validate_source_with_owner(&source, owner)?;
        let v0 = version(&source)?;
        seam("after_v0");
        let tx = source.transaction().map_err(sql_err)?;
        pin_first_read(&tx)?;
        seam("after_first_read");
        let v1 = version(&tx)?;
        let image = capture_image_with_allocation(
            &tx,
            limits.max_capture_bytes,
            deadline,
            allocation.as_ref(),
        )?;
        tx.rollback().map_err(sql_err)?;
        seam("after_transaction");
        let v2 = version(&source)?;
        check(deadline, "Capture")?;
        if file_identity(path)? != identity {
            return Err(err("Incomplete：捕获期间DB文件身份改变"));
        }
        Ok((image, [v0, v1, v2]))
    })();
    source
        .progress_handler(0, None::<fn() -> bool>)
        .map_err(sql_err)?;
    let (image, versions) = captured.map_err(|e| phase_error(e, deadline, "Capture"))?;
    // 不保留源连接或源读事务；副本的所有输入均来自同一已结束的读事务。
    drop(source);
    let verify_limit = until(limits.verify_deadline_ms)?;
    let verify_deadline = hard_deadline.map_or(verify_limit, |d| d.min(verify_limit));
    let mut result = verify_image_metered(image, verify_deadline, limits.max_capture_bytes, job)
        .map_err(|e| phase_error(e, verify_deadline, "Verify"))?;
    result.source_versions = versions;
    Ok(result)
}
fn allocation_refusal(error: &str) -> bool {
    error.contains(newchan_rust::s_content::capture_allocation::LIMIT)
        || error.contains("Incomplete/SqliteOutOfMemory")
}

fn until(ms: u64) -> Result<Instant, String> {
    Instant::now()
        .checked_add(Duration::from_millis(ms))
        .ok_or_else(|| err("期限溢出"))
}
fn check(deadline: Instant, phase: &str) -> Result<(), String> {
    if Instant::now() >= deadline {
        Err(err(format!("Incomplete/{phase}Deadline：超过绝对期限")))
    } else {
        Ok(())
    }
}
fn file_identity(path: &Path) -> Result<(u64, u64), String> {
    let m = std::fs::metadata(path).map_err(err)?;
    Ok((m.dev(), m.ino()))
}
fn scalar(c: &Connection, sql: &std::ffi::CStr) -> Result<i64, String> {
    let mut stmt = CheckedStatement::prepare(c, sql).map_err(sql_err)?;
    let value = stmt
        .next()
        .map_err(sql_err)?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
        .map_err(sql_err)?
        .integer(0)
        .map_err(sql_err)?;
    stmt.finalize().map_err(sql_err)?;
    Ok(value)
}
fn pin_first_read(c: &Connection) -> Result<(), String> {
    let mut stmt = CheckedStatement::prepare(c, c"SELECT key,value FROM meta ORDER BY key LIMIT 1")
        .map_err(sql_err)?;
    if stmt.next().map_err(sql_err)?.is_none() {
        return Err(sql_err(rusqlite::Error::QueryReturnedNoRows));
    }
    stmt.finalize().map_err(sql_err)
}
pub(super) fn version(c: &Connection) -> Result<i64, String> {
    scalar(c, c"PRAGMA data_version")
}

fn err(e: impl std::fmt::Display) -> String {
    format!("Q3CaptureInvalid：{e}")
}
fn sql_err(e: rusqlite::Error) -> String {
    if e.sqlite_error()
        .is_some_and(|v| v.extended_code & 255 == rusqlite::ffi::SQLITE_TOOBIG)
    {
        err(format!(
            "Incomplete/CaptureBytes：SQLite单值或行超过捕获限制：{e}"
        ))
    } else if let Some(cause) = e
        .sqlite_error()
        .filter(|v| v.extended_code & 255 == rusqlite::ffi::SQLITE_NOMEM)
    {
        // rusqlite may display only the filename when sqlite3_open_v2 returns
        // NOMEM with a NULL handle. Preserve the actual typed SQLite cause.
        err(format!(
            "Incomplete/SqliteOutOfMemory：SQLite out of memory（extended_code={}）：{e}",
            cause.extended_code
        ))
    } else {
        err(e)
    }
}
fn phase_error(e: String, deadline: Instant, phase: &str) -> String {
    if Instant::now() >= deadline {
        err(format!("Incomplete/{phase}Deadline：阶段超过绝对期限；{e}"))
    } else if e.contains("string or blob too big") {
        // 共用旧审计接口返回String；这里仅恢复原SQLite限额错误分类，仍然拒绝本次捕获。
        err(format!("Incomplete/CaptureBytes：SQLite捕获长度限制；{e}"))
    } else {
        e
    }
}
/// 同一typed schema核；普通和Owned只由真实容器allocator区分。
type SchemaRow = (TextValue, TextValue, TextValue, Option<TextValue>);
type Column = (i64, TextValue, TextValue, i64, Option<TextValue>, i64);
struct Index {
    unique: i64,
    origin: TextValue,
    partial: i64,
    columns: CodecVec<Option<TextValue>>,
}
impl Serialize for Index {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (self.unique, &self.origin, self.partial, &self.columns[..]).serialize(s)
    }
}
fn index_cmp(a: &Index, b: &Index) -> std::cmp::Ordering {
    a.unique
        .cmp(&b.unique)
        .then_with(|| a.origin.as_str().cmp(b.origin.as_str()))
        .then(a.partial.cmp(&b.partial))
        .then_with(|| {
            a.columns
                .iter()
                .map(|v| v.as_ref().map(TextValue::as_str))
                .cmp(b.columns.iter().map(|v| v.as_ref().map(TextValue::as_str)))
        })
}
struct SchemaTable {
    name: TextValue,
    columns: CodecVec<Column>,
    indexes: CodecVec<Index>,
}
struct Schema {
    rows: CodecVec<SchemaRow>,
    tables: CodecVec<SchemaTable>,
}
impl Schema {
    fn same_tables(&self, other: &Self) -> bool {
        self.tables.len() == other.tables.len()
            && self.tables.iter().zip(other.tables.iter()).all(|(a, b)| {
                a.name == b.name
                    && a.columns[..] == b.columns[..]
                    && a.indexes.len() == b.indexes.len()
                    && a.indexes
                        .iter()
                        .zip(b.indexes.iter())
                        .all(|(x, y)| index_cmp(x, y).is_eq())
            })
    }
}
struct SchemaHeader<'a>(&'a Schema);
impl Serialize for SchemaHeader<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeStruct};
        struct Tables<'a>(&'a [SchemaTable]);
        impl Serialize for Tables<'_> {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut out = s.serialize_map(Some(self.0.len()))?;
                for t in self.0 {
                    out.serialize_entry(t.name.as_str(), &(&t.columns[..], &t.indexes[..]))?;
                }
                out.end()
            }
        }
        let mut out = s.serialize_struct("CaptureHeader", 2)?;
        out.serialize_field("schema", &Tables(&self.0.tables))?;
        out.serialize_field("schema_rows", &self.0.rows[..])?;
        out.end()
    }
}
struct BytesOut {
    bytes: CodecVec<u8>,
    error: Option<String>,
}
impl Write for BytesOut {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self.bytes.try_extend_bytes(b) {
            Ok(()) => Ok(b.len()),
            Err(e) => {
                self.error = Some(e.to_string());
                Err(std::io::ErrorKind::Other.into())
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode(v: &impl Serialize, owner: Option<&OwnedBudget>) -> Result<CodecVec<u8>, String> {
    let mut out = BytesOut {
        bytes: bj::new_work(owner),
        error: None,
    };
    let result = serde_json::to_writer(&mut out, v);
    if let Some(e) = out.error {
        return Err(e);
    }
    result.map_err(err)?;
    Ok(out.bytes)
}
fn sql_text(owner: Option<&OwnedBudget>) -> BytesOut {
    BytesOut {
        bytes: bj::new_work(owner),
        error: None,
    }
}
impl BytesOut {
    fn text(&mut self, v: &str) -> Result<(), String> {
        self.bytes
            .try_extend_bytes(v.as_bytes())
            .map_err(|e| e.to_string())
    }
    fn quote(&mut self, v: &str) -> Result<(), String> {
        self.text("\"")?;
        for c in v.split_inclusive('"') {
            self.text(c)?;
            if c.ends_with('"') {
                self.text("\"")?;
            }
        }
        self.text("\"")
    }
    fn columns(&mut self, cols: &[TextValue]) -> Result<(), String> {
        for (i, c) in cols.iter().enumerate() {
            if i != 0 {
                self.text(",")?;
            }
            self.quote(c.as_str())?;
        }
        Ok(())
    }
    fn statement<'a>(&mut self, c: &'a Connection) -> Result<CheckedStatement<'a>, String> {
        self.bytes.try_push(0).map_err(|e| e.to_string())?;
        let sql = std::ffi::CStr::from_bytes_with_nul(&self.bytes).map_err(err)?;
        CheckedStatement::prepare(c, sql).map_err(sql_err)
    }
}
fn row_text(
    row: &mut CheckedRow<'_>,
    i: usize,
    owner: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    TextValue::from_str(row.text(i).map_err(sql_err)?, owner)
}
fn row_opt_text(
    row: &mut CheckedRow<'_>,
    i: usize,
    owner: Option<&OwnedBudget>,
) -> Result<Option<TextValue>, String> {
    row.optional_text(i)
        .map_err(sql_err)?
        .map(|s| TextValue::from_str(s, owner))
        .transpose()
}
fn schema(c: &Connection, owner: Option<&OwnedBudget>) -> Result<Schema, String> {
    let mut stmt = CheckedStatement::prepare(
        c,
        c"SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name",
    )
    .map_err(sql_err)?;
    let mut rows = bj::new_work(owner);
    while let Some(mut r) = stmt.next().map_err(sql_err)? {
        rows.try_push((
            row_text(&mut r, 0, owner)?,
            row_text(&mut r, 1, owner)?,
            row_text(&mut r, 2, owner)?,
            row_opt_text(&mut r, 3, owner)?,
        ))
        .map_err(|e| e.to_string())?;
    }
    stmt.finalize().map_err(sql_err)?;
    let mut tables = bj::new_work(owner);
    for (kind, name, _, sql) in &rows {
        if kind.as_str() == "index" {
            if sql.is_some() {
                return Err(err("未声明的显式索引"));
            }
            continue;
        }
        if kind.as_str() != "table" {
            return Err(err("未声明视图或触发器"));
        }
        let mut q=CheckedStatement::prepare(c,c"SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info(?1) ORDER BY cid").map_err(sql_err)?;
        q.bind_text(1, name.as_str()).map_err(sql_err)?;
        let mut columns = bj::new_work(owner);
        while let Some(mut r) = q.next().map_err(sql_err)? {
            columns
                .try_push((
                    r.integer(0).map_err(sql_err)?,
                    row_text(&mut r, 1, owner)?,
                    row_text(&mut r, 2, owner)?,
                    r.integer(3).map_err(sql_err)?,
                    row_opt_text(&mut r, 4, owner)?,
                    r.integer(5).map_err(sql_err)?,
                ))
                .map_err(|e| e.to_string())?;
        }
        q.finalize().map_err(sql_err)?;
        let mut q = CheckedStatement::prepare(
            c,
            c"SELECT name,\"unique\",origin,partial FROM pragma_index_list(?1)",
        )
        .map_err(sql_err)?;
        q.bind_text(1, name.as_str()).map_err(sql_err)?;
        let mut idx = bj::new_work(owner);
        while let Some(mut r) = q.next().map_err(sql_err)? {
            idx.try_push((
                row_text(&mut r, 0, owner)?,
                r.integer(1).map_err(sql_err)?,
                row_text(&mut r, 2, owner)?,
                r.integer(3).map_err(sql_err)?,
            ))
            .map_err(|e| e.to_string())?;
        }
        q.finalize().map_err(sql_err)?;
        let mut indexes = bj::new_work(owner);
        for (index_name, unique, origin, partial) in idx {
            let mut q = CheckedStatement::prepare(
                c,
                c"SELECT name FROM pragma_index_info(?1) ORDER BY seqno",
            )
            .map_err(sql_err)?;
            q.bind_text(1, index_name.as_str()).map_err(sql_err)?;
            let mut cols = bj::new_work(owner);
            while let Some(mut r) = q.next().map_err(sql_err)? {
                cols.try_push(row_opt_text(&mut r, 0, owner)?)
                    .map_err(|e| e.to_string())?;
            }
            q.finalize().map_err(sql_err)?;
            indexes
                .try_push(Index {
                    unique,
                    origin,
                    partial,
                    columns: cols,
                })
                .map_err(|e| e.to_string())?;
        }
        indexes.sort_unstable_by(index_cmp);
        tables
            .try_push(SchemaTable {
                name: name.try_clone()?,
                columns,
                indexes,
            })
            .map_err(|e| e.to_string())?;
    }
    tables.sort_unstable_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    Ok(Schema { rows, tables })
}
fn capture_image(tx: &Transaction<'_>, max: u64, deadline: Instant) -> Result<Image, String> {
    capture_image_with_allocation(tx, max, deadline, None)
}
fn capture_image_with_allocation(
    tx: &Transaction<'_>,
    max: u64,
    deadline: Instant,
    allocation: Option<&super::q_allocation::Profile>,
) -> Result<Image, String> {
    check(deadline, "Capture")?;
    if let Some(profile) = allocation {
        profile.validate()?;
    }
    let owner = allocation.map(|p| &p.budget);
    let ddls = control_audit::schema_ddls_with_owner(tx, owner)?;
    let expected = Connection::open_in_memory().map_err(sql_err)?;
    for ddl in &ddls {
        expected.execute_batch(ddl).map_err(sql_err)?;
    }
    let expected_schema = schema(&expected, owner)?;
    let mut q = CheckedStatement::prepare(
        tx,
        c"SELECT COUNT(*),COALESCE(SUM(length(CAST(sql AS BLOB))),0) FROM sqlite_schema",
    )
    .map_err(sql_err)?;
    let mut r = q
        .next()
        .map_err(sql_err)?
        .ok_or_else(|| err("模式统计缺行"))?;
    let count = r.integer(0).map_err(sql_err)?;
    let bytes = r.integer(1).map_err(sql_err)?;
    drop(r);
    q.finalize().map_err(sql_err)?;
    if count < 0
        || bytes < 0
        || count > expected_schema.rows.len() as i64
        || bytes > max.min(128 * 1024) as i64
    {
        return Err(err("Incomplete/CaptureBytes：模式元数据超过捕获界限"));
    }
    control_audit::verify_s3_schema_with_owner(tx, owner)?;
    let schema = schema(tx, owner)?;
    if !schema.same_tables(&expected_schema) {
        return Err(err("列/主键/隐式索引与固定DDL不同"));
    }
    drop(expected_schema);
    drop(expected);
    let header = encode(&SchemaHeader(&schema), owner)?;
    let mut image = Image {
        header,
        tables: bj::new_work(owner),
        ddls,
        digest: CaptureDigest([b'0'; 64]),
        captured_bytes: 0,
        allocation: allocation.cloned(),
    };
    image.check_bytes(max)?;
    let mut digest = Sha256::new();
    digest.update(&image.header);
    for meta in &schema.tables {
        let name = meta.name.as_str();
        check(deadline, "Capture")?;
        let mut columns = bj::new_work(owner);
        for c in &meta.columns {
            columns
                .try_push(c.1.try_clone()?)
                .map_err(|e| e.to_string())?;
        }
        let mut pk = bj::new_work(owner);
        for c in &meta.columns {
            if c.5 > 0 {
                pk.try_push(c).map_err(|e| e.to_string())?;
            }
        }
        pk.sort_unstable_by_key(|c| c.5);
        let mut order = sql_text(owner);
        if name == "raw_events" {
            order.text("seq")?;
        } else {
            for (i, c) in pk.iter().enumerate() {
                if i != 0 {
                    order.text(",")?;
                }
                order.quote(c.1.as_str())?;
            }
        }
        if order.bytes.is_empty() {
            return Err(err("固定表缺少确定主键顺序"));
        }
        let mut q = sql_text(owner);
        q.text("SELECT EXISTS(SELECT 1 FROM ")?;
        q.quote(name)?;
        q.text(" WHERE ")?;
        for (i, c) in columns.iter().enumerate() {
            if i != 0 {
                q.text("+")?;
            }
            q.text("COALESCE(length(CAST(")?;
            q.quote(c.as_str())?;
            q.text(" AS BLOB)),0)")?;
        }
        q.text(">?1)")?;
        let mut oversized = q.statement(tx)?;
        oversized.bind_integer(1, max as i64).map_err(sql_err)?;
        let oversized_value = oversized
            .next()
            .map_err(sql_err)?
            .ok_or_else(|| err("单行预检缺行"))?
            .integer(0)
            .map_err(sql_err)?;
        oversized.finalize().map_err(sql_err)?;
        if oversized_value != 0 {
            return Err(err("Incomplete/CaptureBytes：单条持久记录超过捕获上限"));
        }
        digest.update(&encode(&name, owner)?);
        let mut q = CheckedStatement::prepare(
            tx,
            c"SELECT wr FROM pragma_table_list WHERE schema='main' AND name=?1",
        )
        .map_err(sql_err)?;
        q.bind_text(1, name).map_err(sql_err)?;
        let wr = q
            .next()
            .map_err(sql_err)?
            .ok_or_else(|| err("表rowid登记缺行"))?
            .integer(0)
            .map_err(sql_err)?;
        q.finalize().map_err(sql_err)?;
        let without_rowid = wr != 0;
        let rowid_offset = usize::from(!without_rowid);
        let mut sql = sql_text(owner);
        sql.text("SELECT ")?;
        if !without_rowid {
            sql.text("rowid,")?;
        }
        sql.columns(&columns)?;
        sql.text(" FROM ")?;
        sql.quote(name)?;
        sql.text(" ORDER BY ")?;
        sql.bytes
            .try_extend_bytes(&order.bytes)
            .map_err(|e| e.to_string())?;
        let mut q = sql.statement(tx)?;
        let old = image.tables.capacity();
        image
            .tables
            .try_push(Table {
                name: meta.name.try_clone()?,
                columns,
                rows: bj::new_work(owner),
                rowids: (!without_rowid).then(|| bj::new_work(owner)),
            })
            .map_err(|e| e.to_string())?;
        let table = image.tables.last().unwrap();
        let added = ((image.tables.capacity() - old) * size_of::<Table>()) as u64
            + table.name.capacity() as u64
            + (table.columns.capacity() * size_of::<TextValue>()) as u64
            + table
                .columns
                .iter()
                .map(|c| c.capacity() as u64)
                .sum::<u64>();
        image.charge(added, max)?;
        while let Some(mut row) = q.next().map_err(sql_err)? {
            check(deadline, "Capture")?;
            let n = image.tables.last().unwrap().columns.len();
            let mut payload = 0u64;
            for i in 0..n {
                payload += match row.column_type(i + rowid_offset).map_err(sql_err)? {
                    rusqlite::types::Type::Text => {
                        row.text_bytes(i + rowid_offset).map_err(sql_err)?.len() as u64
                    }
                    rusqlite::types::Type::Blob => {
                        row.blob(i + rowid_offset).map_err(sql_err)?.len() as u64
                    }
                    _ => 0,
                };
            }
            if image.captured_bytes.saturating_add(payload).saturating_add(
                (n * size_of::<CaptureCell>() + size_of::<CodecVec<CaptureCell>>()) as u64,
            ) > max
            {
                return Err(err("Incomplete/CaptureBytes：全域捕获超过事前上限"));
            }
            let mut cells = bj::new_work(owner);
            cells.try_reserve_exact(n).map_err(|e| e.to_string())?;
            for i in 0..n {
                let c = i + rowid_offset;
                let cell = match row.column_type(c).map_err(sql_err)? {
                    rusqlite::types::Type::Null => CaptureCell::Null,
                    rusqlite::types::Type::Integer => {
                        CaptureCell::Integer(row.integer(c).map_err(sql_err)?)
                    }
                    rusqlite::types::Type::Text => {
                        let text = std::str::from_utf8(row.text_bytes(c).map_err(sql_err)?)
                            .map_err(err)?;
                        CaptureCell::Text(TextValue::from_str(text, owner)?)
                    }
                    rusqlite::types::Type::Blob => {
                        let mut b = bj::new_work(owner);
                        b.try_extend_bytes(row.blob(c).map_err(sql_err)?)
                            .map_err(|e| e.to_string())?;
                        CaptureCell::Blob(b)
                    }
                    rusqlite::types::Type::Real => return Err(err("REAL不属于已声明持久列类型")),
                };
                cells.try_push(cell).map_err(|e| e.to_string())?;
            }
            digest.update(b"[");
            for cell in &cells {
                hash_cell_view(&mut digest, cell.view())?;
            }
            digest.update(b"]");
            let owned = (cells.capacity() * size_of::<CaptureCell>()) as u64
                + cells.iter().map(|c| c.capacity() as u64).sum::<u64>();
            let table = image.tables.last_mut().unwrap();
            let old = table.rows.capacity();
            table.rows.try_push(cells).map_err(|e| e.to_string())?;
            let rowids = if let Some(ids) = &mut table.rowids {
                let old = ids.capacity();
                ids.try_push(row.integer(0).map_err(sql_err)?)
                    .map_err(|e| e.to_string())?;
                ((ids.capacity() - old) * size_of::<i64>()) as u64
            } else {
                0
            };
            let row_growth =
                ((table.rows.capacity() - old) * size_of::<CodecVec<CaptureCell>>()) as u64;
            image.charge(owned + rowids + row_growth, max)?;
        }
        q.finalize().map_err(sql_err)?;
    }
    check(deadline, "Capture")?;
    let hash = digest.finalize();
    let hex = b"0123456789abcdef";
    for (i, b) in hash.iter().enumerate() {
        image.digest.0[2 * i] = hex[(b >> 4) as usize];
        image.digest.0[2 * i + 1] = hex[(b & 15) as usize];
    }
    Ok(image)
}
impl Image {
    fn charge(&mut self, bytes: u64, max: u64) -> Result<(), String> {
        self.captured_bytes = self
            .captured_bytes
            .checked_add(bytes)
            .ok_or_else(|| err("捕获字节计数溢出"))?;
        if self.captured_bytes > max {
            Err(err("Incomplete/CaptureBytes：捕获对象容量超过上限"))
        } else {
            Ok(())
        }
    }
    fn check_bytes(&mut self, max: u64) -> Result<(), String> {
        // 按实际Vec/String容量和已拥有对象计，不按展开JSON长度、不称进程RSS。
        let mut n = size_of::<Self>() as u64
            + self.header.capacity() as u64
            + (self.tables.capacity() * size_of::<Table>()) as u64
            + (self.ddls.capacity() * size_of::<&str>()) as u64
            + self.digest.capacity() as u64;
        for table in &self.tables {
            n += table
                .rowids
                .as_ref()
                .map_or(0, |ids| (ids.capacity() * size_of::<i64>()) as u64);
            n += table.name.capacity() as u64
                + (table.columns.capacity() * size_of::<TextValue>()) as u64
                + table
                    .columns
                    .iter()
                    .map(|c| c.capacity() as u64)
                    .sum::<u64>()
                + (table.rows.capacity() * size_of::<CodecVec<CaptureCell>>()) as u64;
            for row in &table.rows {
                n += (row.capacity() * size_of::<CaptureCell>()) as u64;
                for cell in row {
                    n += match cell {
                        CaptureCell::Text(t) => t.capacity() as u64,
                        CaptureCell::Blob(b) => b.capacity() as u64,
                        _ => 0,
                    };
                }
            }
        }
        self.captured_bytes = n;
        if n > max {
            Err(err("Incomplete/CaptureBytes：捕获对象容量超过上限"))
        } else {
            Ok(())
        }
    }
}
fn decimal_bytes(n: i64, out: &mut [u8; 20]) -> &[u8] {
    let mut unsigned = n.unsigned_abs();
    let mut i = 20;
    loop {
        i -= 1;
        out[i] = b'0' + (unsigned % 10) as u8;
        unsigned /= 10;
        if unsigned == 0 {
            break;
        }
    }
    if n < 0 {
        i -= 1;
        out[i] = b'-';
    }
    &out[i..]
}
#[cfg(test)]
fn hash_cell(hash: &mut Sha256, cell: &Cell) -> Result<(), String> {
    let view = match cell {
        Cell::Null => CellView::Null,
        Cell::Integer(n) => CellView::Integer(*n),
        Cell::Text(v) => CellView::Text(v),
        Cell::Blob(v) => CellView::Blob(v),
        Cell::Real(_) => CellView::Real,
    };
    hash_cell_view(hash, view)
}
fn hash_cell_view(hash: &mut Sha256, cell: CellView<'_>) -> Result<(), String> {
    match cell {
        CellView::Null => {
            hash.update(b"NoneType");
            hash.update(4u64.to_be_bytes());
            hash.update(b"null");
        }
        CellView::Integer(n) => {
            let mut scalar = [0u8; 20];
            let s = decimal_bytes(n, &mut scalar);
            hash.update(b"int");
            hash.update((s.len() as u64).to_be_bytes());
            hash.update(s);
        }
        CellView::Blob(b) => {
            hash.update(b"b");
            hash.update((b.len() as u64).to_be_bytes());
            hash.update(b);
        }
        CellView::Text(s) => {
            struct Count(u64);
            impl Write for Count {
                fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                    self.0 += b.len() as u64;
                    Ok(b.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            struct Hash<'a>(&'a mut Sha256);
            impl Write for Hash<'_> {
                fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                    self.0.update(b);
                    Ok(b.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let mut count = Count(0);
            serde_json::to_writer(&mut count, s).map_err(err)?;
            hash.update(b"str");
            hash.update(count.0.to_be_bytes());
            serde_json::to_writer(&mut Hash(hash), s).map_err(err)?;
        }
        CellView::Real => return Err(err("REAL捕获单元不合法")),
    }
    Ok(())
}
#[cfg(test)]
fn verify_image(
    image: Image,
    deadline: Instant,
    max_capture_bytes: u64,
) -> Result<VerifiedCapture, String> {
    verify_image_metered(image, deadline, max_capture_bytes, None)
}
fn verify_image_metered(
    image: Image,
    deadline: Instant,
    max_capture_bytes: u64,
    job: Option<&mut dyn newchan_rust::s_content::access::Access>,
) -> Result<VerifiedCapture, String> {
    let mut db = import_image(&image, deadline)?;
    let digest = image.digest;
    let allocation = image.allocation.clone();
    let captured_bytes = image.captured_bytes;
    drop(image);
    db.pragma_update(None, "query_only", true)
        .map_err(sql_err)?;
    let (audited, audit_receipt, q_source) = {
        let tx = db.transaction().map_err(sql_err)?;
        match (job, allocation.as_ref()) {
            (Some(job), Some(profile)) => {
                let (state, receipt, capture) = state::audit_with_access_and_allocation(
                    &tx,
                    Some(deadline),
                    Some(max_capture_bytes),
                    job,
                    profile,
                )?;
                (state, Some(receipt), Some(capture))
            }
            (Some(job), None) => {
                let (state, receipt) =
                    state::audit_with_access(&tx, Some(deadline), Some(max_capture_bytes), job)?;
                (state, Some(receipt), None)
            }
            (None, Some(profile)) => {
                let (state, capture) = state::audit_bounded_with_allocation(
                    &tx,
                    Some(deadline),
                    Some(max_capture_bytes),
                    profile,
                )?;
                (state, None, Some(capture))
            }
            (None, None) => (
                state::audit_bounded(&tx, Some(deadline), Some(max_capture_bytes))?,
                None,
                None,
            ),
        }
    };
    check(deadline, "Verify")?;
    db.progress_handler(0, None::<fn() -> bool>)
        .map_err(sql_err)?;
    Ok(VerifiedCapture {
        db,
        audited,
        digest,
        captured_bytes,
        source_versions: [0; 3],
        verify_deadline: deadline,
        audit_receipt,
        q_source,
        allocation,
    })
}

/// 把一致捕获镜像导入私有内存副本（Q3 verify 与 #1394 财务会话共用同一段导入；不审计）。
/// 返回的连接仍挂着以 `deadline` 为界的 progress handler，由调用方在审计后撤下。
fn import_image(image: &Image, deadline: Instant) -> Result<Connection, String> {
    check(deadline, "Verify")?;
    let mut db = Connection::open_in_memory().map_err(sql_err)?;
    db.pragma_update(None, "foreign_keys", false)
        .map_err(sql_err)?;
    db.progress_handler(100, Some(move || Instant::now() >= deadline))
        .map_err(sql_err)?;
    let tx = db.transaction().map_err(sql_err)?;
    for ddl in &image.ddls {
        check(deadline, "Verify")?;
        tx.execute_batch(ddl).map_err(sql_err)?;
    }
    // 副本从空库建立；外键在全部行导入后一次核对，避免按表名排序碰到前向依赖。
    let owner = image.allocation.as_ref().map(|p| &p.budget);
    for table in &image.tables {
        let offset = usize::from(table.rowids.is_some());
        let mut sql = sql_text(owner);
        sql.text("INSERT INTO ")?;
        sql.quote(table.name.as_str())?;
        sql.text(" (")?;
        if offset != 0 {
            sql.text("rowid,")?;
        }
        sql.columns(&table.columns)?;
        sql.text(") VALUES (")?;
        for i in 1..=table.columns.len() + offset {
            if i != 1 {
                sql.text(",")?;
            }
            sql.text("?")?;
            let mut scalar = [0u8; 20];
            let bytes = decimal_bytes(i64::try_from(i).map_err(err)?, &mut scalar);
            sql.bytes
                .try_extend_bytes(bytes)
                .map_err(|e| e.to_string())?;
        }
        sql.text(")")?;
        sql.bytes.try_push(0).map_err(|e| e.to_string())?;
        let csql = std::ffi::CStr::from_bytes_with_nul(&sql.bytes).map_err(err)?;
        let mut insert = CheckedStatement::prepare_insert(&tx, csql).map_err(sql_err)?;
        for (n, row) in table.rows.iter().enumerate() {
            check(deadline, "Verify")?;
            insert.reset().map_err(sql_err)?;
            if let Some(ids) = &table.rowids {
                insert.bind_integer(1, ids[n]).map_err(sql_err)?;
            }
            for (i, cell) in row.iter().enumerate() {
                let index = i32::try_from(i + offset + 1).map_err(err)?;
                match cell.view() {
                    CellView::Null => insert.bind_null(index),
                    CellView::Integer(n) => insert.bind_integer(index, n),
                    CellView::Text(s) => insert.bind_text(index, s),
                    CellView::Blob(b) => insert.bind_blob(index, b),
                    CellView::Real => return Err(err("REAL捕获单元不合法")),
                }
                .map_err(sql_err)?;
            }
            insert.execute().map_err(sql_err)?;
        }
        insert.finalize().map_err(sql_err)?;
    }
    let foreign_errors = scalar(&tx, c"SELECT count(*) FROM pragma_foreign_key_check")?;
    if foreign_errors != 0 {
        return Err(err("捕获内容外键闭合失败"));
    }
    tx.commit().map_err(sql_err)?;
    Ok(db)
}

/// 每个worker独占的长期连接与至多一份完整缓存；data_version不跨连接比较。
/// 缓存无公开裸摘要入口；只有本次同连接版本/文件身份夹持成功才可借出。
enum ReaderPath {
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
    Plain(std::path::PathBuf),
    Shared(
        newchan_rust::s_content::capture_allocation::dispatch::DispatchArc<
            newchan_rust::s_content::capture_allocation::dispatch::DispatchPath,
        >,
    ),
}
impl std::ops::Deref for ReaderPath {
    type Target = std::path::Path;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Plain(path) => path,
            Self::Shared(path) => (**path).as_ref(),
        }
    }
}
pub(super) struct AuditReader {
    path: ReaderPath,
    resources_digest: CaptureDigest,
    allocation: Option<super::q_allocation::Profile>,
    source: Option<Connection>,
    identity: Option<(u64, u64)>,
    cached: Option<(i64, retained::Packed)>,
    stats: ReaderStats,
}
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct ReaderStats {
    pub captures: u64,
    pub cache_hits: u64,
    pub cached_bytes: u64,
}
impl AuditReader {
    #[cfg(test)]
    pub(super) fn new(path: std::path::PathBuf, resources: &q_resources::QResources) -> Self {
        Self::new_with_allocation(path, resources, None)
    }
    #[cfg(test)]
    pub(super) fn new_with_allocation(
        path: std::path::PathBuf,
        resources: &q_resources::QResources,
        allocation: Option<super::q_allocation::Profile>,
    ) -> Self {
        Self::new_with_reader_path(ReaderPath::Plain(path), resources, allocation)
    }
    /// #1394：worker与reset共用已成功分配的路径owner；无新路径副本或准入判断。
    pub(super) fn new_with_dispatch_path(
        path: newchan_rust::s_content::capture_allocation::dispatch::DispatchArc<
            newchan_rust::s_content::capture_allocation::dispatch::DispatchPath,
        >,
        resources: &q_resources::QResources,
        allocation: Option<super::q_allocation::Profile>,
    ) -> Self {
        Self::new_with_reader_path(ReaderPath::Shared(path), resources, allocation)
    }
    fn new_with_reader_path(
        path: ReaderPath,
        resources: &q_resources::QResources,
        allocation: Option<super::q_allocation::Profile>,
    ) -> Self {
        Self {
            path,
            resources_digest: CaptureDigest::from_str(resources.digest())
                .expect("frozen resource digest"),
            allocation,
            source: None,
            identity: None,
            cached: None,
            stats: ReaderStats::default(),
        }
    }
    pub(super) fn allocation_profile(&self) -> Option<&super::q_allocation::Profile> {
        self.allocation.as_ref()
    }
    pub(super) fn close(&mut self) {
        self.cached = None;
        self.source = None;
        self.identity = None;
        self.stats.cached_bytes = 0;
    }
    #[cfg(test)]
    pub(super) fn stats(&self) -> ReaderStats {
        self.stats
    }
    pub(super) fn with_capture<T>(
        &mut self,
        resources: &q_resources::QResources,
        job: &mut dyn newchan_rust::s_content::access::Access,
        action: impl FnOnce(
            &mut ConfiguredCapture<'_>,
            &mut dyn newchan_rust::s_content::access::Access,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_seam(resources, job, |_| {}, action)
    }
    fn with_seam<T>(
        &mut self,
        resources: &q_resources::QResources,
        job: &mut dyn newchan_rust::s_content::access::Access,
        mut seam: impl FnMut(&str),
        action: impl FnOnce(
            &mut ConfiguredCapture<'_>,
            &mut dyn newchan_rust::s_content::access::Access,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(profile) = &self.allocation {
            profile.validate()?;
        }
        if resources.digest() != self.resources_digest.as_str() {
            self.close();
            return Err(err("reader资源与启动冻结配置不同"));
        }
        let hard = job.read_deadline().map_err(err)?;
        let result = self.prepare(resources, job, hard, &mut seam);
        let (mut packed, cache_version, verify_deadline) = match result {
            Ok(v) => v,
            Err(e) => {
                if !allocation_refusal(&e) {
                    self.close();
                }
                return Err(e);
            }
        };
        let proof = match packed.checkout(verify_deadline) {
            Ok(v) => v,
            Err(e) => {
                if allocation_refusal(&e) {
                    if let Some(version) = cache_version {
                        self.stats.cached_bytes = packed.retained_bytes;
                        self.cached = Some((version, packed));
                    }
                } else {
                    self.close();
                }
                return Err(e);
            }
        };
        if let Err(e) = job.pre_io_check().map_err(err) {
            packed.restore(proof);
            if let Some(version) = cache_version {
                self.stats.cached_bytes = packed.retained_bytes;
                self.cached = Some((version, packed));
            }
            return Err(e);
        }
        seam("verify_complete");
        let mut capture = ConfiguredCapture {
            capture: proof,
            resources,
        };
        let output = action(&mut capture, job);
        seam("action_complete");
        packed.restore(capture.capture);
        // 缓存是action之前已独立核完的不可变捕获，不是查询结果/安装提交；
        // 沿旧AuditReader语义，查询错误或其后job停止不撤销此前有效proof。
        // 下一请求仍先夹持当前源，再按其自己的job余额重放receipt。
        if let Some(version) = cache_version {
            self.stats.cached_bytes = packed.retained_bytes;
            self.cached = Some((version, packed));
        }
        job.pre_io_check().map_err(err)?;
        output
    }
    fn prepare(
        &mut self,
        resources: &q_resources::QResources,
        job: &mut dyn newchan_rust::s_content::access::Access,
        hard: Option<Instant>,
        seam: &mut impl FnMut(&str),
    ) -> Result<(retained::Packed, Option<i64>, Instant), String> {
        use q_resources::Limit as Q;
        let limits = resources.capture_limits();
        let local = until(limits.capture_deadline_ms)?;
        let deadline = hard.map_or(local, |d| d.min(local));
        for attempt in 0..2 {
            seam("capture_start");
            job.charge_requests(1).map_err(err)?;
            check(deadline, "Capture")?;
            let identity = file_identity(&self.path)?;
            if self.source.is_none() || self.identity != Some(identity) {
                let opened = open_source(&self.path, limits, deadline)?;
                self.close();
                self.source = Some(opened);
                self.identity = Some(identity);
            }
            let source = self.source.as_mut().unwrap();
            source
                .busy_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(sql_err)?;
            source
                .progress_handler(100, Some(move || Instant::now() >= deadline))
                .map_err(sql_err)?;
            let captured = (|| {
                validate_source_with_owner(source, self.allocation.as_ref().map(|p| &p.budget))?;
                let v0 = version(source)?;
                seam("after_v0");
                let tx = source.transaction().map_err(sql_err)?;
                pin_first_read(&tx)?;
                seam("after_first_read");
                let v1 = version(&tx)?;
                let candidate = self
                    .cached
                    .as_ref()
                    .is_some_and(|(v, _)| *v == v0 && v0 == v1);
                let image = if candidate {
                    None
                } else {
                    Some(capture_image_with_allocation(
                        &tx,
                        limits.max_capture_bytes,
                        deadline,
                        self.allocation.as_ref(),
                    )?)
                };
                tx.rollback().map_err(sql_err)?;
                seam("after_transaction");
                let v2 = version(source)?;
                check(deadline, "Capture")?;
                if file_identity(&self.path)? != identity {
                    return Err(err("Incomplete：捕获期间DB文件身份改变"));
                }
                Ok((candidate, image, [v0, v1, v2]))
            })();
            source
                .progress_handler(0, None::<fn() -> bool>)
                .map_err(sql_err)?;
            let (candidate, image, versions) =
                captured.map_err(|e| phase_error(e, deadline, "Capture"))?;
            seam("verify_start");
            let local = until(limits.verify_deadline_ms)?;
            let verify_deadline = hard.map_or(local, |d| d.min(local));
            if candidate && versions.iter().all(|v| *v == versions[0]) {
                let (_, packed) = self.cached.as_ref().ok_or_else(|| err("缓存候选消失"))?;
                packed.replay(job)?;
                job.pre_io_check().map_err(err)?;
                let (_, packed) = self.cached.take().ok_or_else(|| err("缓存候选消失"))?;
                self.stats.cached_bytes = 0;
                self.stats.cache_hits += 1;
                return Ok((packed, Some(versions[2]), verify_deadline));
            }
            if candidate {
                if attempt == 1 {
                    return Err(err("Incomplete：并发提交阻止缓存复用"));
                }
                // 同一capture与job期限内至多重捕获一次，不重标旧proof。
                continue;
            }
            self.stats.captures += 1;
            let mut proof = verify_image_metered(
                image.ok_or_else(|| err("缺完整捕获"))?,
                verify_deadline,
                limits.max_capture_bytes,
                Some(job),
            )?;
            proof.source_versions = versions;
            let packed = retained::Packed::new(proof, limits.max_capture_bytes, verify_deadline)?;
            seam("before_cache_registration");
            check(verify_deadline, "Verify")?;
            let v3 = version(self.source.as_ref().unwrap())?;
            let allowance = resources.limit(Q::CacheBytes) / resources.limit(Q::QueryWorkers);
            let reusable = versions.iter().all(|v| *v == v3)
                && file_identity(&self.path)? == identity
                && packed.retained_bytes <= allowance;
            job.pre_io_check().map_err(err)?;
            // 新鲜完整proof和保留量都成功后才撤销旧缓存；失败不得把旧权威丢掉。
            self.cached = None;
            self.stats.cached_bytes = 0;
            return Ok((packed, reusable.then_some(v3), verify_deadline));
        }
        Err(err("Incomplete：捕获重试已耗尽"))
    }
}
fn open_source(path: &Path, limits: Limits, deadline: Instant) -> Result<Connection, String> {
    limits.validate()?;
    let source = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sql_err)?;
    source
        .busy_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(sql_err)?;
    source
        .set_limit(
            Limit::SQLITE_LIMIT_LENGTH,
            limits.max_capture_bytes.min(i32::MAX as u64) as i32,
        )
        .map_err(sql_err)?;
    source
        .pragma_update(None, "query_only", true)
        .map_err(sql_err)?;
    source
        .pragma_update(None, "read_uncommitted", false)
        .map_err(sql_err)?;
    Ok(source)
}
pub(super) fn validate_source(source: &Connection) -> Result<(), String> {
    validate_source_with_owner(source, None)
}
fn validate_source_with_owner(
    source: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<(), String> {
    if !source.is_autocommit() {
        return Err(err("reader不得持有跨请求事务"));
    }
    let query_only = scalar(source, c"PRAGMA query_only")?;
    let read_uncommitted = scalar(source, c"PRAGMA read_uncommitted")?;
    if query_only != 1 || read_uncommitted != 0 {
        return Err(err("只读隔离设置改变"));
    }
    let mut stmt = CheckedStatement::prepare(source, c"PRAGMA database_list").map_err(sql_err)?;
    let mut names = bj::new_work(owner);
    while let Some(mut row) = stmt.next().map_err(sql_err)? {
        names
            .try_push(row_text(&mut row, 1, owner)?)
            .map_err(|e| e.to_string())?;
    }
    stmt.finalize().map_err(sql_err)?;
    if names.len() != 1 || names[0].as_str() != "main" {
        return Err(err("源连接含附加数据库"));
    }
    let mut stmt =
        CheckedStatement::prepare(source, c"SELECT journal_mode FROM pragma_journal_mode")
            .map_err(sql_err)?;
    let mut row = stmt
        .next()
        .map_err(sql_err)?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
        .map_err(sql_err)?;
    let mode = row_text(&mut row, 0, owner)?;
    drop(row);
    stmt.finalize().map_err(sql_err)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(err("源库必须由S预先建立WAL，Q不替它改模式"));
    }
    Ok(())
}

// ===== #1394 DESIGN-R6 c2：财务只读会话的启动捕获（A6 r2 §2.1；原件字节门已按 A6-2 删除）=====
// 只复用本文件既有的 open_source/validate_source/version/pin_first_read/capture_image/import_image；
// 判据不另写。活源连接由会话持有（同连接 data_version 夹持只在这一条连接上比较）。

/// 会话启动的一次当前一致捕获：私有副本 P（尚未审计，query_only=ON）与同一读事务内的控制原件。
pub(super) struct SessionCapture {
    pub db: Connection,
    pub versions: [i64; 3],
    pub digest: CaptureDigest,
    pub captured_bytes: u64,
    /// 捕获镜像的实测堆字节：`capture_image` 前后 Δ(全部 malloc zone 在用) − Δ(SQLite 自记)。
    pub image_heap_bytes: i64,
    pub control: Vec<u8>,
}

/// 活源连接：同 Q3 `open_source`（READ_ONLY|NO_MUTEX、query_only=ON、read_uncommitted=OFF、单值长度限）。
pub(super) fn session_open_source(
    path: &Path,
    max_capture_bytes: u64,
    startup_timeout_ms: u64,
    deadline: Instant,
) -> Result<Connection, String> {
    open_source(
        path,
        Limits {
            max_capture_bytes,
            capture_deadline_ms: startup_timeout_ms,
            verify_deadline_ms: startup_timeout_ms,
        },
        deadline,
    )
}

/// 三点夹持同 `capture_with_deadline_and_access`：v0 → BEGIN → 首读 → v1 → 全域捕获与控制原件
/// （同一读事务）→ ROLLBACK → v2；要求 v0=v1=v2 且 DB 文件身份不变。捕获随后导入私有副本 P。
pub(super) fn session_capture(
    source: &mut Connection,
    path: &Path,
    max_capture_bytes: u64,
    deadline: Instant,
    control: impl FnOnce(&Connection) -> Result<Vec<u8>, String>,
    heap: impl Fn() -> Result<(u64, i64), String>,
) -> Result<SessionCapture, String> {
    check(deadline, "Capture")?;
    let identity = file_identity(path)?;
    source
        .busy_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(sql_err)?;
    source
        .progress_handler(100, Some(move || Instant::now() >= deadline))
        .map_err(sql_err)?;
    let captured = (|| {
        validate_source(source)?;
        let v0 = version(source)?;
        let tx = source.transaction().map_err(sql_err)?;
        pin_first_read(&tx)?;
        let v1 = version(&tx)?;
        let before = heap()?;
        let image = capture_image(&tx, max_capture_bytes, deadline)?;
        let after = heap()?;
        let image_heap = (i128::from(after.0) - i128::from(before.0))
            - (i128::from(after.1) - i128::from(before.1));
        let control = control(&tx)?;
        tx.rollback().map_err(sql_err)?;
        let v2 = version(source)?;
        check(deadline, "Capture")?;
        if file_identity(path)? != identity {
            return Err(err("Incomplete：捕获期间DB文件身份改变"));
        }
        if v0 != v1 || v1 != v2 {
            return Err(err("Incomplete：捕获期间data_version改变"));
        }
        Ok((image, control, [v0, v1, v2], image_heap))
    })();
    source
        .progress_handler(0, None::<fn() -> bool>)
        .map_err(sql_err)?;
    let (image, control, versions, image_heap) =
        captured.map_err(|e| phase_error(e, deadline, "Capture"))?;
    let db = import_image(&image, deadline).map_err(|e| phase_error(e, deadline, "Verify"))?;
    db.pragma_update(None, "query_only", true)
        .map_err(sql_err)?;
    let digest = image.digest;
    let captured_bytes = image.captured_bytes;
    drop(image);
    Ok(SessionCapture {
        db,
        versions,
        digest,
        captured_bytes,
        image_heap_bytes: i64::try_from(image_heap).unwrap_or(i64::MAX),
        control,
    })
}

// ===== #1394 c2 会话段结束 =====

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::{
        self as sc,
        job::{Existing, Job, JobLimits},
        NodeSink, NodeSource,
    };
    use prepared::tests::Fixture;
    use std::process::{Command, Stdio};
    fn limits() -> Limits {
        Limits {
            max_capture_bytes: 16 * 1024 * 1024,
            capture_deadline_ms: 20000,
            verify_deadline_ms: 20000,
        }
    }
    // TestOnly，不是产品期限（#1394 B1）：功能测试 job 的 max_elapsed_ms 放长到 600000，不随 runner 速度超期；测 job 期限的用例自带受控时钟。
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"600000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn python_digest(path: &Path) -> String {
        // 复用原_schema/capture_digest算法；测试只扩展schema输入，不让旧Q接受S3服务。
        let script = r#"import sys,sqlite3,json
sys.path.insert(0,sys.argv[1]);import s_query_integrity as q
c=sqlite3.connect('file:'+sys.argv[2]+'?mode=ro',uri=True);c.execute('BEGIN')
rows,schema=q._schema(c);tables={}
for table,(cols,_) in schema.items():
 names=[x[1] for x in cols];pk=[x[1] for x in sorted(cols,key=lambda x:x[5]) if x[5]]
 order='seq' if table=='raw_events' else ','.join(pk)
 tables[table]=tuple(c.execute('SELECT '+','.join(names)+' FROM '+table+' ORDER BY '+order))
print(q.capture_digest(dict(schema=schema,schema_rows=rows,tables=tables)))
"#;
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let out = Command::new("python3")
            .args(["-c", script])
            .arg(base)
            .arg(path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().into()
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn allocation_profile(budget: &OwnedBudget) -> super::super::q_allocation::Profile {
        super::super::q_allocation::Profile {
            budget: budget.clone(),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        }
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_q_cold_capture_keeps_image_audit_and_projection_on_original_ledger() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let expected = python_digest(&f.db_path());
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let mut capture = capture_verified_with_allocation(
            &f.db_path(),
            limits(),
            Some(allocation_profile(&budget)),
        )
        .unwrap();
        assert_eq!(capture.digest(), expected);
        assert_eq!(capture.generation(), 2);
        assert!(capture
            .allocation
            .as_ref()
            .unwrap()
            .budget
            .same_ledger(&budget));
        assert!(capture
            .audited
            .budget
            .as_ref()
            .unwrap()
            .same_ledger(&budget));
        assert!(matches!(&capture.audited.raw, bj::RawEvents::Owned(_)));
        assert!(matches!(
            &capture.audited.cuts[0].metadata.session_id,
            TextValue::Owned(_)
        ));
        let projection = capture
            .projection(2, projection::HistoryMode::AsKnown, 1_048_576, &mut job())
            .unwrap();
        assert!(matches!(&projection.root, audit_store::RefValue::Owned(_)));
        assert!(matches!(&projection.wire, bj::JsonOwner::Owned(_)));
        assert!(matches!(&projection.digest, TextValue::Owned(_)));
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        drop(projection);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(0);
        let error = capture_verified_with_allocation(
            &f.db_path(),
            limits(),
            Some(allocation_profile(&low)),
        )
        .err()
        .unwrap();
        assert!(allocation_refusal(&error), "{error}");
        assert_eq!(low.snapshot().allocation_attempts, 0);
        assert!(low.snapshot().rejected_layouts > 0);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_q_reader_budget_refusal_preserves_authoritative_cache_and_retries() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let mut reader =
            AuditReader::new_with_allocation(f.db_path(), &r, Some(allocation_profile(&budget)));
        let expected = reader
            .with_capture(&r, &mut job(), |c, _| Ok(c.capture.digest))
            .unwrap();
        assert!(reader.cached.is_some());
        let old_bytes = reader.stats.cached_bytes;
        let old_pages = reader.cached.as_ref().unwrap().1.page_digest_for_testonly();
        let before = budget.snapshot();
        let mut filler = bj::new_work::<u8>(Some(&budget));
        filler
            .try_reserve_exact(before.limit_bytes - before.live_requested_layout_bytes)
            .unwrap();
        let error = reader
            .with_capture(&r, &mut job(), |_, _| Ok(()))
            .err()
            .unwrap();
        assert!(allocation_refusal(&error), "{error}");
        assert!(reader.cached.is_some());
        assert_eq!(reader.stats.cached_bytes, old_bytes);
        assert_eq!(
            reader.cached.as_ref().unwrap().1.page_digest_for_testonly(),
            old_pages
        );
        drop(filler);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        let retried = reader
            .with_capture(&r, &mut job(), |c, _| {
                assert!(c
                    .capture
                    .audited
                    .budget
                    .as_ref()
                    .unwrap()
                    .same_ledger(&budget));
                Ok(c.capture.digest)
            })
            .unwrap();
        assert_eq!(retried, expected);
        assert_eq!(reader.stats.captures, 1);
        assert_eq!(reader.stats.cache_hits, 1);
        reader.close();
        drop(reader);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn real_capture_digest_matches_python_and_survives_source_commit_without_lock() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let before = f.conn.total_changes();
        let expected = python_digest(&f.db_path());
        let mut capture = capture_verified(&f.db_path(), limits()).unwrap();
        assert_eq!(capture.digest(), expected);
        assert_eq!(capture.generation(), 1);
        assert_eq!(f.conn.total_changes(), before);
        assert!(capture.captured_bytes > 0 && capture.captured_bytes <= limits().max_capture_bytes);
        let old = capture
            .projection(1, projection::HistoryMode::AsKnown, 1_048_576, &mut job())
            .unwrap();
        assert!(capture
            .db
            .execute("UPDATE meta SET value='bad' WHERE key='generation'", [])
            .is_err());
        publish(&mut f, 2);
        let same = capture
            .projection(1, projection::HistoryMode::AsKnown, 1_048_576, &mut job())
            .unwrap();
        assert_eq!(same.digest, old.digest);
        assert_eq!(capture.digest(), expected);
        let new = capture_verified(&f.db_path(), limits()).unwrap();
        assert_eq!(new.generation(), 2);
        assert_eq!(new.digest(), python_digest(&f.db_path()));
        assert_ne!(new.digest(), capture.digest());
    }
    #[test]
    fn configured_capture_binds_projection_and_full_delta_to_immutable_actual_snapshot() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let resources = q_resources::tests::functional_resources();
        let mut job = job();
        let mut captured = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
        let digest = captured.capture.digest().to_string();
        let projection = captured
            .projection(1, projection::HistoryMode::AsKnown, &mut job)
            .unwrap();
        assert_eq!(
            projection
                .wire
                .view()
                .get("fixed_cut")
                .and_then(|v| v.get("cut_generation"))
                .and_then(|v| v.as_str()),
            Some("1")
        );
        publish(&mut f, 2);
        let one = captured.delta_references(0, 2, &mut job).unwrap();
        assert_eq!(one.anchors.len(), 1);
        assert_eq!(one.through_generation, 1);
        assert!(!one.has_more);
        assert_eq!(one.validated_capture_digest, digest);
        assert_eq!(
            one.anchors[0]
                .view()
                .get("generation")
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert!(one.usage[0].nodes > 0 && one.usage[0].records > 0);
        let mut fresh = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
        let first = fresh.delta_references(0, 1, &mut job).unwrap();
        assert_eq!(first.anchors, one.anchors);
        assert!(first.has_more);
        assert_ne!(first.validated_capture_digest, digest);
        let next = fresh.delta_references(1, 1, &mut job).unwrap();
        assert_eq!(next.through_generation, 2);
        assert!(!next.has_more);
        assert_eq!(
            next.anchors[0]
                .view()
                .get("base_cut")
                .and_then(|v| v.as_str()),
            first.anchors[0]
                .view()
                .get("next_cut")
                .and_then(|v| v.as_str())
        );
        assert_eq!(job.snapshot().requests, 2);
        assert!(fresh
            .delta_references(0, 5, &mut job)
            .unwrap_err()
            .contains("Watch批数"));
        assert!(fresh
            .delta_references(3, 1, &mut job)
            .unwrap_err()
            .contains("永久发布历史"));
    }
    #[test]
    fn later_oversized_delta_refuses_entire_candidate_without_modifying_source_or_cursor() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let resources = q_resources::tests::functional_resources();
        let mut captured = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let all = captured.delta_references(0, 2, &mut job()).unwrap();
        assert!(all.usage[1].nodes > all.usage[0].nodes);
        let mut input = q_resources::tests::functional_input();
        input["limits"]["max_atomic_delta_nodes"] = json!(all.usage[0].nodes.to_string());
        let limited = q_resources::QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap();
        let mut captured = capture_configured(&f.db_path(), &limited, &mut job()).unwrap();
        let mut job = job();
        let before = f.conn.total_changes();
        let failure = captured.delta_references(0, 2, &mut job).unwrap_err();
        assert!(failure.contains("max_atomic_delta_nodes"), "{failure}");
        let used = job.snapshot().records;
        // 重试仍带原after及同一job：此前核过的第一批不能暗自推进或清账。
        let first = captured.delta_references(0, 1, &mut job).unwrap();
        assert_eq!(first.through_generation, 1);
        assert!(first.has_more);
        assert_eq!(
            serde_json::to_value(first.anchors[0].view()).unwrap(),
            serde_json::to_value(all.anchors[0].view()).unwrap()
        );
        assert_eq!(job.snapshot().records, used + first.usage[0].records);
        assert_eq!(f.conn.total_changes(), before);
    }
    #[test]
    fn configured_capture_retries_count_and_absolute_job_deadline_precedes_io() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let resources = q_resources::tests::resources();
        let clock = Arc::new(AtomicBool::new(false));
        let clock_ref = clock.clone();
        let now = Instant::now();
        let limits = JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1","max_total_node_bytes":"1","max_total_nodes":"1","max_total_records":"1","max_total_requests":"2","max_elapsed_ms":"1000","max_staged_bytes":"1","max_store_bytes":"1"})).unwrap();
        let mut job = Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || {
                now + Duration::from_millis(if clock_ref.load(Ordering::SeqCst) {
                    1000
                } else {
                    0
                })
            }),
        )
        .unwrap();
        let missing = Path::new("/deliberately-absent-q3-source.sqlite");
        assert!(capture_configured(missing, &resources, &mut job).is_err());
        assert_eq!(job.snapshot().requests, 1);
        assert!(capture_configured(missing, &resources, &mut job).is_err());
        assert_eq!(job.snapshot().requests, 2);
        clock.store(true, Ordering::SeqCst);
        let error = capture_configured(missing, &resources, &mut job)
            .err()
            .unwrap();
        assert!(error.contains("job_deadline"), "{error}");
        assert_eq!(job.snapshot().requests, 2);
        assert!(capture_with_deadline(
            missing,
            super::tests::limits(),
            Some(Instant::now()),
            |_| {}
        )
        .err()
        .unwrap()
        .contains("CaptureDeadline"));
    }
    #[test]
    fn concurrent_commits_before_and_after_snapshot_pin_do_not_mix_generations() {
        for phase in ["after_v0", "after_first_read", "after_transaction"] {
            let mut f = Fixture::new();
            publish(&mut f, 1);
            let path = f.db_path();
            let captured = capture_with(&path, limits(), |at| {
                if at == phase {
                    publish(&mut f, 2)
                }
            })
            .unwrap();
            assert_eq!(
                captured.generation(),
                if phase == "after_v0" { 2 } else { 1 },
                "{phase}"
            );
            assert_eq!(capture_verified(&path, limits()).unwrap().generation(), 2);
        }
    }
    #[test]
    fn byte_limit_bad_schema_bad_state_and_file_replacement_are_not_verified() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let valid = capture_verified(&f.db_path(), limits()).unwrap();
        let bound = Limits {
            max_capture_bytes: valid.captured_bytes - 1,
            ..limits()
        };
        let error = capture_verified(&f.db_path(), bound)
            .err()
            .expect("完整捕获少一字节须失败");
        assert!(error.contains("CaptureBytes"), "{error}");
        f.conn
            .execute_batch("CREATE VIEW unexpected AS SELECT 1")
            .unwrap();
        assert!(capture_verified(&f.db_path(), limits()).is_err());
        f.conn.execute_batch("DROP VIEW unexpected").unwrap();
        let title: String = f
            .conn
            .query_row(
                "SELECT title FROM catalog WHERE catalog_id='CC-001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        f.conn
            .execute(
                "UPDATE catalog SET title='bad' WHERE catalog_id='CC-001'",
                [],
            )
            .unwrap();
        assert!(capture_verified(&f.db_path(), limits()).is_err());
        f.conn
            .execute(
                "UPDATE catalog SET title=?1 WHERE catalog_id='CC-001'",
                [title],
            )
            .unwrap();
        let path = f.db_path();
        let old = path.with_extension("original");
        let failed = capture_with(&path, limits(), |phase| {
            if phase == "after_transaction" {
                std::fs::rename(&path, &old).unwrap();
                std::fs::copy(&old, &path).unwrap();
            }
        });
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&old, &path).unwrap();
        assert!(failed.err().unwrap().contains("文件身份改变"));
        assert_eq!(valid.generation(), 1, "已核旧capture不因另一次失败而改动");
    }
    #[test]
    fn capture_verify_and_content_traversal_honor_expired_deadlines() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let tx = f.conn.transaction().unwrap();
        let expired_at = Some(Instant::now());
        assert!(read_raw_events_until(&tx, expired_at)
            .unwrap_err()
            .contains("VerifyDeadline"));
        assert!(read_effective_events_until(&[], expired_at)
            .unwrap_err()
            .contains("VerifyDeadline"));
        assert!(tb02a::enrich_raw_until(&tx, &mut [], expired_at)
            .unwrap_err()
            .contains("VerifyDeadline"));
        assert!(
            history::hash_json_until(&json!({"comparisons":[[],[],[],[]]}), expired_at)
                .unwrap_err()
                .contains("VerifyDeadline")
        );
        assert!(
            capture_image(&tx, limits().max_capture_bytes, Instant::now())
                .err()
                .unwrap()
                .contains("CaptureDeadline")
        );
        let mut image =
            capture_image(&tx, limits().max_capture_bytes, until(20000).unwrap()).unwrap();
        let counted = image.captured_bytes;
        image.check_bytes(limits().max_capture_bytes).unwrap();
        assert_eq!(
            image.captured_bytes, counted,
            "增量容量计数须与独立全遍历相等"
        );
        assert!(
            verify_image(image, Instant::now(), limits().max_capture_bytes)
                .err()
                .unwrap()
                .contains("VerifyDeadline")
        );
        assert!(state::audit_until(&tx, Some(Instant::now()))
            .err()
            .unwrap()
            .contains("VerifyDeadline"));
        let mut source = sc::MemoryNodes::new();
        let mut expired = history::HashReader::until(&mut source, Some(Instant::now()));
        assert!(expired.read(&[0; 32]).is_err());
        assert!(expired.put([0; 32], 0, &[]).is_err());
    }
    #[test]
    fn typed_cell_framing_preserves_null_integer_unicode_and_raw_blob() {
        let rows = vec![
            Cell::Null,
            Cell::Integer(i64::MIN),
            Cell::Integer(i64::MAX),
            Cell::Text("中\0é\n\"".into()),
            Cell::Blob(vec![0, 255, 128, 1]),
        ];
        let mut hash = Sha256::new();
        for row in &rows {
            hash_cell(&mut hash, row).unwrap();
        }
        let got = format!("{:x}", hash.finalize());
        let script="import hashlib,json,struct;h=hashlib.sha256();v=[None,-2**63,2**63-1,'中\\0é\\n\"',bytes([0,255,128,1])];[(lambda b,t:h.update(t+struct.pack('>Q',len(b))+b))(x if type(x)is bytes else json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode(),b'b' if type(x)is bytes else type(x).__name__.encode()) for x in v];print(h.hexdigest())";
        let out = Command::new("python3")
            .args(["-c", script])
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(got, String::from_utf8(out.stdout).unwrap().trim());
        assert!(hash_cell(&mut Sha256::new(), &Cell::Real(1.0)).is_err());
    }

    #[test]
    fn bounded_audit_rendering_stops_compact_dag_before_large_materialization() {
        let mut nodes = sc::MemoryNodes::new();
        let block = vec![b'a'; 4096];
        let chunk = sc::frame(sc::tag::BYTES_CHUNK, &block);
        nodes.put(chunk, sc::tag::BYTES_CHUNK, &block).unwrap();
        let leaf = sc::frame(sc::tag::SEQ_LEAF, &chunk);
        nodes.put(leaf, sc::tag::SEQ_LEAF, &chunk).unwrap();
        let mut sequence = sc::SequenceRef::from_parts_unchecked(leaf, 1);
        for _ in 0..31 {
            sequence = sc::sequence_concat(&mut nodes, &sequence, &sequence).unwrap();
        }
        let extent = sequence.count() * 4096;
        let mut body = extent.to_be_bytes().to_vec();
        body.extend_from_slice(&sequence.root());
        let digest = sc::frame(sc::tag::STRING, &body);
        nodes.put(digest, sc::tag::STRING, &body).unwrap();
        let root = sc::ContentRef::new(sc::Kind::String, &digest, Some(extent));
        let mut graph =
            history::HashReader::bounded(&mut nodes, Some(until(10000).unwrap()), Some(8192));
        let mut budget = job();
        let mut source = sc::job::JobNodeSource::new(&mut budget, &mut graph);
        let mut bytes = Vec::new();
        let error = sc::render_canonical(&mut source, &root, &mut bytes).unwrap_err();
        assert!(error.to_string().contains("CaptureBytes"), "{error}");
        assert!(bytes.len() <= 8192);
    }

    #[test]
    fn native_sqlite_oversized_row_reports_incomplete_capture_budget() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        f.conn
            .execute(
                "INSERT INTO s_content_nodes(digest,tag,body) VALUES(?1,16,zeroblob(2000000))",
                [vec![255u8; 32]],
            )
            .unwrap();
        let error = capture_verified(
            &f.db_path(),
            Limits {
                max_capture_bytes: 1_000_000,
                ..limits()
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("Incomplete/CaptureBytes"), "{error}");
    }
    fn inspect_cached(
        reader: &mut AuditReader,
        r: &q_resources::QResources,
        j: &mut Job,
    ) -> Result<(i64, String, String), String> {
        reader.with_capture(r, j, |c, j| {
            let g = c.capture.generation();
            let p = c.projection(g, projection::HistoryMode::AsKnown, j)?;
            assert_eq!(c.capture.audited.raw.len(), g as usize);
            assert!(c
                .capture
                .db
                .execute("UPDATE meta SET value='bad' WHERE key='generation'", [])
                .is_err());
            Ok((
                g,
                c.capture.digest().to_string(),
                serde_json::to_string(&p.wire.view()).map_err(err)?,
            ))
        })
    }
    #[test]
    fn worker_cache_reuses_only_same_connection_current_image_and_keeps_full_content() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        let mut reader = AuditReader::new(f.db_path(), &r);
        let mut j = job();
        let cold = inspect_cached(&mut reader, &r, &mut j).unwrap();
        let bytes = reader.stats().cached_bytes;
        assert!(bytes > 0);
        let warm = inspect_cached(&mut reader, &r, &mut j).unwrap();
        assert_eq!(cold, warm);
        assert_eq!(reader.stats().captures, 1);
        assert_eq!(reader.stats().cache_hits, 1);
        assert_eq!(j.snapshot().requests, 2);
        assert!(reader.source.as_ref().unwrap().is_autocommit());
        // 查询借出私有副本期间源仍可提交；没有跨请求或跨action源读事务。
        reader
            .with_capture(&r, &mut j, |c, _| {
                publish(&mut f, 2);
                assert_eq!(c.capture.generation(), 1);
                Ok(())
            })
            .unwrap();
        let next = inspect_cached(&mut reader, &r, &mut j).unwrap();
        assert_eq!(next.0, 2);
        assert_ne!(next.1, cold.1);
        assert_eq!(reader.stats().captures, 2);
        reader.close();
        let again = inspect_cached(&mut reader, &r, &mut j).unwrap();
        assert_eq!(again, next);
        assert_eq!(reader.stats().captures, 3);
    }
    #[test]
    fn worker_cache_races_recapture_once_and_registration_never_relabels_old_image() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        let mut reader = AuditReader::new(f.db_path(), &r);
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        let mut fired = false;
        let mut j = job();
        let g = reader
            .with_seam(
                &r,
                &mut j,
                |seam| {
                    if seam == "after_transaction" && !fired {
                        fired = true;
                        publish(&mut f, 2);
                    }
                },
                |c, _| Ok(c.capture.generation()),
            )
            .unwrap();
        assert_eq!(g, 2);
        assert!(fired);
        assert_eq!(j.snapshot().requests, 2);
        assert_eq!(reader.stats().captures, 2);
        reader.close();
        let mut fired = false;
        let old = reader
            .with_seam(
                &r,
                &mut job(),
                |seam| {
                    if seam == "before_cache_registration" && !fired {
                        fired = true;
                        publish(&mut f, 3);
                    }
                },
                |c, _| Ok(c.capture.generation()),
            )
            .unwrap();
        assert_eq!(old, 2);
        assert!(reader.cached.is_none());
        assert_eq!(inspect_cached(&mut reader, &r, &mut job()).unwrap().0, 3);
    }
    #[test]
    fn worker_cache_detects_corruption_file_replacement_and_isolation_changes() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        let mut reader = AuditReader::new(f.db_path(), &r);
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        let title: String = f
            .conn
            .query_row(
                "SELECT title FROM catalog WHERE catalog_id='CC-001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        f.conn
            .execute(
                "UPDATE catalog SET title='bad' WHERE catalog_id='CC-001'",
                [],
            )
            .unwrap();
        assert!(inspect_cached(&mut reader, &r, &mut job()).is_err());
        assert!(reader.cached.is_none());
        assert!(reader.source.is_none());
        f.conn
            .execute(
                "UPDATE catalog SET title=?1 WHERE catalog_id='CC-001'",
                [title],
            )
            .unwrap();
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        reader
            .source
            .as_ref()
            .unwrap()
            .pragma_update(None, "read_uncommitted", true)
            .unwrap();
        assert!(inspect_cached(&mut reader, &r, &mut job())
            .unwrap_err()
            .contains("隔离"));
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        let second = Fixture::new();
        // 同一路径指向另一完整WAL数据库：仅替换symlink，不复制/拆散DB与WAL。
        let link = f.db_path().with_extension("reader-link");
        std::os::unix::fs::symlink(f.db_path(), &link).unwrap();
        let mut linked = AuditReader::new(link.clone(), &r);
        assert_eq!(inspect_cached(&mut linked, &r, &mut job()).unwrap().0, 1);
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(second.db_path(), &link).unwrap();
        assert_eq!(inspect_cached(&mut linked, &r, &mut job()).unwrap().0, 0);
        assert_eq!(linked.stats().cache_hits, 0);
        std::fs::remove_file(link).unwrap();
    }
    #[test]
    fn worker_retained_capacity_rejects_whole_proof_or_skips_cache_without_losing_rows() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let proof = capture_verified(&f.db_path(), limits()).unwrap();
        let packed =
            retained::Packed::new(proof, limits().max_capture_bytes, until(20000).unwrap())
                .unwrap();
        let retained = packed.retained_bytes;
        assert!(retained > 0);
        assert_eq!(
            packed.measure(retained, until(20000).unwrap()).unwrap(),
            retained
        );
        assert!(packed
            .measure(retained - 1, until(20000).unwrap())
            .unwrap_err()
            .contains("RetainedBytes"));
        let mut input = q_resources::tests::functional_input();
        input["limits"]["max_cache_bytes"] = json!("1");
        let r = q_resources::QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap();
        let mut reader = AuditReader::new(f.db_path(), &r);
        let first = inspect_cached(&mut reader, &r, &mut job()).unwrap();
        let second = inspect_cached(&mut reader, &r, &mut job()).unwrap();
        assert_eq!(first, second);
        assert_eq!(reader.stats().captures, 2);
        assert_eq!(reader.stats().cache_hits, 0);
        assert_eq!(reader.stats().cached_bytes, 0);
        let mut input = q_resources::tests::functional_input();
        input["limits"]["max_query_workers"] = json!("1");
        let baseline_resources = q_resources::tests::functional_resources();
        let mut metered = AuditReader::new(f.db_path(), &baseline_resources);
        inspect_cached(&mut metered, &baseline_resources, &mut job()).unwrap();
        input["limits"]["max_cache_bytes"] = json!(metered.stats().cached_bytes.to_string());
        let r = q_resources::QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap();
        let mut reader = AuditReader::new(f.db_path(), &r);
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        inspect_cached(&mut reader, &r, &mut job()).unwrap();
        assert_eq!(reader.stats().cache_hits, 1);
    }
    #[test]
    fn worker_pool_runs_actual_cached_projection_on_one_owned_reader() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let mut input = q_resources::tests::functional_input();
        input["limits"]["max_query_workers"] = json!("1");
        let r = std::sync::Arc::new(
            q_resources::QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap(),
        );
        let pool = q_executor::Executor::new(f.db_path(), r).unwrap();
        let read = || {
            pool.try_submit(|reader, r, _, _queued| {
                let answer = inspect_cached(reader, r, &mut job())?;
                Ok((answer, reader.stats(), std::thread::current().id()))
            })
            .unwrap()
            .recv()
            .unwrap()
        };
        let first = read();
        let second = read();
        assert_eq!(first.0, second.0);
        assert_eq!(first.2, second.2);
        assert_eq!(second.1.captures, 1);
        assert_eq!(second.1.cache_hits, 1);
        publish(&mut f, 2);
        assert_eq!(read().0 .0, 2);
        drop(pool);
    }
    fn public_history_records(proof: &mut VerifiedCapture) -> u64 {
        // 独立从已发布实际根的13个公共集合读取extent；不复用审计计费回调或内部索引计数。
        use sc::path::{lookup_path_refview, PathStep};
        let tx = proof.db.transaction().unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut count = 0;
        for cut in &proof.audited.cuts {
            for (root, fields) in [
                (
                    &cut.snapshot,
                    &[
                        "objects",
                        "withdrawn_objects",
                        "witnesses",
                        "relations",
                        "observations",
                        "raw_history",
                    ][..],
                ),
                (
                    &cut.delta,
                    &[
                        "upserts",
                        "withdrawals",
                        "replaces",
                        "witnesses",
                        "relations",
                        "observations",
                        "raw_history_added",
                    ][..],
                ),
            ] {
                for field in fields {
                    let array = lookup_path_refview(
                        &mut source,
                        root.view(),
                        &[PathStep::Field((*field).into())],
                    )
                    .unwrap();
                    assert_eq!(array.kind, sc::Kind::Array);
                    count += array.extent.unwrap();
                }
            }
        }
        count
    }
    #[test]
    fn full_audit_job_matches_public_record_extents_and_warm_reuse_keeps_cumulative_limits() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let (revision, token) = f.accept_begin(3, ("raw-1", 10, 7, 13, 4, 9), 2);
        f.publish(&revision, &token).unwrap();
        let r = q_resources::tests::functional_resources();
        let expected =
            public_history_records(&mut capture_verified(&f.db_path(), limits()).unwrap());
        assert!(expected > 0);
        let mut reader = AuditReader::new(f.db_path(), &r);
        let mut first = job();
        reader
            .with_capture(&r, &mut first, |c, _| {
                assert_eq!(c.capture.generation(), 3);
                Ok(())
            })
            .unwrap();
        let cold = first.snapshot();
        assert_eq!(cold.records, expected);
        assert!(cold.nodes > 0 && cold.node_bytes > 0);
        reader.with_capture(&r, &mut first, |_, _| Ok(())).unwrap();
        let same = first.snapshot();
        assert_eq!(same.nodes, cold.nodes);
        assert_eq!(same.node_bytes, cold.node_bytes);
        assert_eq!(same.records, 2 * expected);
        assert_eq!(same.requests, 2);
        let mut next = job();
        reader.with_capture(&r, &mut next, |_, _| Ok(())).unwrap();
        assert_eq!(
            (
                next.snapshot().nodes,
                next.snapshot().node_bytes,
                next.snapshot().records
            ),
            (cold.nodes, cold.node_bytes, cold.records)
        );
        assert_eq!(reader.stats().captures, 1);
        assert_eq!(reader.stats().cache_hits, 2);
        let mut direct = job();
        capture_configured(&f.db_path(), &r, &mut direct).unwrap();
        assert_eq!(
            (direct.snapshot().nodes, direct.snapshot().records),
            (cold.nodes, expected)
        );
        // 从既有实际用量冻结较小测试job；缓存命中也必须拒绝，不能发出旧结果。
        for which in ["nodes", "bytes", "records"] {
            // 每次失败会清cache，下一项先用新正常job暖起相同原件。
            reader.with_capture(&r, &mut job(), |_, _| Ok(())).unwrap();
            let mut caps = cold.limits;
            match which {
                "nodes" => caps.max_total_nodes = cold.nodes - 1,
                "bytes" => caps.max_total_node_bytes = cold.node_bytes - 1,
                _ => caps.max_total_records = expected - 1,
            }
            let mut small = Job::new(
                caps,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap();
            let mut delivered = false;
            let failure = reader
                .with_capture(&r, &mut small, |_, _| {
                    delivered = true;
                    Ok(())
                })
                .unwrap_err();
            assert!(failure.contains("job_limit_exceeded"), "{which}: {failure}");
            assert!(!delivered);
            assert!(reader.cached.is_none());
            assert!(small.snapshot().stopped.is_some());
        }
    }
    #[test]
    fn cold_audit_limits_stop_before_verified_capture_and_failed_retries_do_not_clear_ledger() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        let mut caps = job().snapshot().limits;
        caps.max_total_nodes = 1;
        let mut limited = Job::new(
            caps,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let error = capture_configured(&f.db_path(), &r, &mut limited)
            .err()
            .unwrap();
        assert!(error.contains("max_total_nodes"), "{error}");
        assert_eq!(limited.snapshot().nodes, 1);
        assert_eq!(limited.snapshot().requests, 1);
        assert!(capture_configured(&f.db_path(), &r, &mut limited).is_err());
        assert_eq!(limited.snapshot().nodes, 1);
        assert_eq!(limited.snapshot().requests, 1);
        // 完整框架坏原件仍被原判据拒绝，不存在receipt放行通道。
        f.conn
            .execute(
                "UPDATE catalog SET title='changed' WHERE catalog_id='CC-001'",
                [],
            )
            .unwrap();
        assert!(capture_configured(&f.db_path(), &r, &mut job()).is_err());
    }
    #[test]
    fn failed_query_can_keep_verified_cache_but_never_bypasses_next_job_budget() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::functional_resources();
        for after_budget_failure in [false, true] {
            let mut reader = AuditReader::new(f.db_path(), &r);
            let mut first = job();
            let failure = reader
                .with_capture(&r, &mut first, |_, j| -> Result<(), String> {
                    if after_budget_failure {
                        j.charge_records(u64::MAX).map_err(err)?;
                    }
                    Err("test-only-invalid-query".into())
                })
                .unwrap_err();
            assert!(failure.contains(if after_budget_failure {
                "job_limit_exceeded"
            } else {
                "invalid-query"
            }));
            // 与旧AuditReader一样：查询失败不改变此前已核的副本，不是客户端安装/发布。
            assert!(reader.cached.is_some());
            assert_eq!(reader.stats().captures, 1);
            let mut next = job();
            reader
                .with_capture(&r, &mut next, |c, _| {
                    assert_eq!(c.capture.generation(), 1);
                    Ok(())
                })
                .unwrap();
            assert_eq!(reader.stats().cache_hits, 1);
            assert!(next.snapshot().nodes > 1 && next.snapshot().records > 0);
            let mut caps = next.snapshot().limits;
            caps.max_total_nodes = 1;
            let mut small = Job::new(
                caps,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap();
            let mut action_called = false;
            assert!(reader
                .with_capture(&r, &mut small, |_, _| {
                    action_called = true;
                    Ok(())
                })
                .unwrap_err()
                .contains("max_total_nodes"));
            assert!(!action_called);
            assert!(reader.cached.is_none());
        }
    }

    #[test]
    #[ignore = "#1394 显式冻结库Q冷读测量，需Q3_CONTENT_MEASURE_DB/RESOURCES/OUTPUT/HASH"]
    fn testonly_q_frozen_capture_measurement() {
        use newchan_rust::s_content::access::Access;
        use std::fs;
        use std::io::Write;
        struct Server;
        impl Access for Server {
            fn owner_job(&mut self) -> Option<&mut Job> {
                None
            }
        }
        let path = std::path::PathBuf::from(
            std::env::var("Q3_CONTENT_MEASURE_DB").expect("显式只读库副本"),
        );
        let config = std::path::PathBuf::from(
            std::env::var("Q3_CONTENT_MEASURE_RESOURCES").expect("显式规范测试资源"),
        );
        let output = std::path::PathBuf::from(
            std::env::var("Q3_CONTENT_MEASURE_OUTPUT").expect("独立测量结果"),
        );
        let expected = std::env::var("Q3_CONTENT_MEASURE_HASH").expect("冻结DB哈希");
        assert_eq!(sha256_hex(&fs::read(&path).unwrap()), expected);
        assert!(!output.exists(), "保留已有证据，禁止覆盖");
        let r = q_resources::QResources::load(&config).unwrap();
        // 节点逐次计数只用于归因；扩展规模的耗时测量默认关闭，避免混入计数开销。
        let count_nodes = match std::env::var("Q3_CONTENT_MEASURE_COUNTS").as_deref() {
            Ok("1") => true,
            Ok("0") | Err(std::env::VarError::NotPresent) => false,
            _ => panic!("Q3_CONTENT_MEASURE_COUNTS只能为0/1或不设置"),
        };
        let mut reader = AuditReader::new(path.clone(), &r);
        let mut rows = Vec::new();
        for pass in ["cold_reader", "warm_reader"] {
            let start = Instant::now();
            let mut events = Vec::new();
            let mut verified_capture = None;
            let (result, audit_phases, work_counts) = state::traced(count_nodes, || {
                reader.with_seam(&r,&mut Server,|event| {
                events.push(json!({"phase":event,"elapsed_ns":start.elapsed().as_nanos().to_string()}));
            },|capture,access| {
                let head=capture.capture.generation();
                verified_capture=Some(json!({"generation":head.to_string(),"capture_digest":capture.capture.digest(),"captured_bytes":capture.capture.captured_bytes.to_string(),"source_versions":capture.capture.source_versions}));
                let mut deltas=Vec::new();
                if pass=="cold_reader" {
                    // 诊断永久历史根的成本，不调用受delivery保留窗口约束的公共reader。
                    // 本口在cfg(test)内，不能令已过期Delta重新获得交付资格。
                    let tx=capture.capture.db.transaction().map_err(err)?;
                    let mut source=content_store::ReadStore::new(&tx).map_err(err)?;
                    let mut graph=history::HashReader::new(&mut source);
                    for cut in &capture.capture.audited.cuts {
                        let usage=q_delta::measure(&mut graph,&cut.delta,&r,access)?;
                        deltas.push(json!({"generation":cut.metadata.generation.to_string(),"nodes":usage.nodes.to_string(),"node_bytes":usage.node_bytes.to_string(),"records":usage.records.to_string()}));
                    }
                }
                Ok(json!({"full_delta_usage":deltas}))
            })
            });
            let ok = result.is_ok();
            let stats = reader.stats();
            rows.push(json!({"pass":pass,"ok":ok,"elapsed_ns":start.elapsed().as_nanos().to_string(),"events":events,
                "audit_phases":audit_phases,"node_counts_enabled":count_nodes,"work_counts":work_counts,
                "verified_capture":verified_capture,"value":result.as_ref().ok(),"error":result.as_ref().err(),"reader_stats":{"captures":stats.captures,"cache_hits":stats.cache_hits,"cached_bytes":stats.cached_bytes}}));
            if !ok {
                break;
            } // 真正超限后记录，不换更大预算重试或假造成功。
        }
        reader.close();
        let after = sha256_hex(&fs::read(&path).unwrap());
        assert_eq!(after, expected);
        let result = json!({"schema_revision":"testonly-q-content-measurement/1","source_db":path,"source_db_sha256":expected,
            "resource_config":strict_json(r.canonical().as_bytes()).unwrap(),"resource_config_sha256":r.digest(),"passes":rows,
            "source_db_unchanged":true,"server_started":false,"resource_profile":null,
            "scope":"fresh AuditReader and same-reader cache; OS page cache not cleared; phase timestamps and permanent historical Delta closures only; no delivery-retention bypass; not RSS/storage peaks, deployment resources or full9011 acceptance"});
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .unwrap();
        f.write_all(canonical_json(&result).as_bytes()).unwrap();
        println!("Q3_MEASURE_RESULT {}", canonical_json(&result));
    }
}
