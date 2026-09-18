//! #1394：S3一致typed捕获与原Q capture_digest；离开源事务后复用完整S3审计。
//! 只重建项目固定DDL，摘要保留源schema原文；不执行来源SQL、不持有跨请求S读锁。
//! 配置读取入口绑定Q v3资源文件；每worker长期reader在同连接版本夹持下复用完整私有缓存。
use super::*;
use rusqlite::{
    limits::Limit,
    params_from_iter,
    types::{Value as Cell, ValueRef},
    Transaction,
};
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

struct Table {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
    // 私有复制信息：现有独立索引核首次插入序；不进入原Q摘要配方。
    rowids: Option<Vec<i64>>,
}
struct Image {
    header: Vec<u8>,
    tables: Vec<Table>,
    ddls: Vec<&'static str>,
    digest: String,
    captured_bytes: u64,
}

/// 只有完整state::audit通过才构造；数据库句柄不向调用者开放写面。
pub(super) struct VerifiedCapture {
    db: Connection,
    audited: state::AuditedState,
    digest: String,
    pub captured_bytes: u64,
    pub source_versions: [i64; 3],
    verify_deadline: Instant,
    audit_receipt: Option<audit_budget::Receipt>,
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
        let mut graph = history::HashReader::new(&mut source);
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

pub(super) fn capture_verified(path: &Path, limits: Limits) -> Result<VerifiedCapture, String> {
    capture_with(path, limits, |_| {})
}
/// 当前一次捕获绑定同一份冻结资源，后续投影/Delta读取不能随请求改限额。
pub(super) struct ConfiguredCapture<'a> {
    capture: VerifiedCapture,
    resources: &'a q_resources::QResources,
}
#[derive(Debug)]
pub(super) struct DeltaReferences {
    pub anchors: Vec<Value>,
    pub usage: Vec<q_delta::Usage>,
    pub through_generation: i64,
    pub has_more: bool,
    pub validated_capture_digest: String,
}
impl ConfiguredCapture<'_> {
    /// 只从本次完整已核副本取错误回复的身份，不借请求自称已核。
    pub(super) fn identity(&self) -> Result<(String, String), String> {
        let session = meta_get_opt(&self.capture.db, "session_id")?
            .ok_or_else(|| err("已核捕获缺会话身份"))?;
        let (_, incarnation, _) = v2::protocol_identity(&self.capture.db)?;
        Ok((session, incarnation))
    }
    /// 沿旧控制器ready小回执；调用前已经完整审计，不展开Snapshot rows。
    pub(super) fn ready_cut(&self) -> Result<Value, String> {
        let (session, incarnation) = self.identity()?;
        let generation = self.capture.generation();
        let catalog = meta_get_opt(&self.capture.db, "catalog_revision")?
            .ok_or_else(|| err("已核捕获缺目录版本"))?;
        let index = self
            .capture
            .audited
            .cuts
            .last()
            .map_or("", |cut| cut.batch_id.as_str());
        Ok(
            json!({"session_id":session,"session_generation":incarnation,
            "generation":generation.to_string(),"structure_cut":format!("cut-{generation}"),
            "catalog_revision":catalog,"index_frontier":index}),
        )
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
        let policy=tx.query_row("SELECT policy_revision,retain_generations,first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1",[],|row|Ok(q_watch::Policy{
            revision:row.get(0)?,retain:row.get(1)?,first:row.get(2)?,head:row.get(3)?,
        })).map_err(err)?;
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let mut graph = history::HashReader::bounded(
            &mut source,
            deadline,
            Some(self.resources.limit(q_resources::Limit::CaptureBytes)),
        );
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
    pub(super) fn content(
        &mut self,
        request: &q_content::Request,
        producer_epoch: &str,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<Vec<u8>, String> {
        use q_resources::Limit;
        request.validate_resources(self.resources)?;
        let deadline = job.read_deadline().map_err(err)?;
        let tx = self.capture.db.transaction().map_err(sql_err)?;
        let session = meta_get_opt(&tx, "session_id")?.ok_or_else(|| err("缺会话身份"))?;
        let (_, incarnation, _) = v2::protocol_identity(&tx)?;
        request.validate_identity(&session, &incarnation)?;
        let mut source = content_store::ReadStore::new(&tx).map_err(err)?;
        let mut graph = history::HashReader::bounded(
            &mut source,
            deadline,
            Some(self.resources.limit(Limit::CaptureBytes)),
        );
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
                reader::check_projection_anchor(&actual, request.anchor())?;
                actual.root
            }
            q_content::Anchor::Delta => {
                let cut = reader::validated_delta(&tx, &self.capture.audited, request.anchor())?;
                // Content回复也包含Delta引用，不能借片段路径绕过完整闭包的三原子门。
                q_delta::measure(&mut graph, &cut.delta, self.resources, job)?;
                cut.delta.clone()
            }
        };
        let fragment = request.read(&mut graph, &root, self.resources, job)?;
        request.reply(
            fragment,
            &self.capture.digest,
            &session,
            &incarnation,
            producer_epoch,
            self.resources,
            job,
        )
    }
    pub(super) fn snapshot(
        &mut self,
        request: &q_snapshot::Request,
        producer_epoch: &str,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<Vec<u8>, String> {
        request.validate_resources(self.resources)?;
        let generation = request.generation(self.capture.generation());
        let projection = self.projection(generation, request.history_mode(), job)?;
        q_snapshot::reply(
            request,
            projection,
            self.capture.digest(),
            self.resources,
            producer_epoch,
            job,
        )
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
        let mut anchors = Vec::new();
        let mut usage = Vec::new();
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
            usage.push(q_delta::measure(
                &mut source,
                &cut.delta,
                self.resources,
                job,
            )?);
            anchors.push(reader::delta_anchor(cut));
            through = cut.metadata.generation;
        }
        job.pre_io_check().map_err(err)?;
        Ok(DeltaReferences {
            anchors,
            usage,
            through_generation: through,
            has_more: through < head,
            validated_capture_digest: self.capture.digest.clone(),
        })
    }
}

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
fn capture_with(
    path: &Path,
    limits: Limits,
    seam: impl FnMut(&str),
) -> Result<VerifiedCapture, String> {
    capture_with_deadline(path, limits, None, seam)
}
fn capture_with_deadline(
    path: &Path,
    limits: Limits,
    hard_deadline: Option<Instant>,
    seam: impl FnMut(&str),
) -> Result<VerifiedCapture, String> {
    capture_with_deadline_and_access(path, limits, hard_deadline, seam, None)
}
fn capture_with_deadline_and_access(
    path: &Path,
    limits: Limits,
    hard_deadline: Option<Instant>,
    mut seam: impl FnMut(&str),
    job: Option<&mut dyn newchan_rust::s_content::access::Access>,
) -> Result<VerifiedCapture, String> {
    limits.validate()?;
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
        let mode: String = source
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(sql_err)?;
        if mode.to_lowercase() != "wal" {
            return Err(err("源库必须由S预先建立WAL，Q不替它改模式"));
        }
        let names = source
            .prepare("PRAGMA database_list")
            .map_err(sql_err)?
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        if names != ["main"] {
            return Err(err("源连接含附加数据库"));
        }
        let v0 = version(&source)?;
        seam("after_v0");
        let tx = source.transaction().map_err(sql_err)?;
        tx.query_row(
            "SELECT key,value FROM meta ORDER BY key LIMIT 1",
            [],
            |_| Ok(()),
        )
        .map_err(sql_err)?;
        seam("after_first_read");
        let v1 = version(&tx)?;
        let image = capture_image(&tx, limits.max_capture_bytes, deadline)?;
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
fn version(c: &Connection) -> Result<i64, String> {
    c.query_row("PRAGMA data_version", [], |r| r.get(0))
        .map_err(err)
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
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// 与Python _schema同形；schema_rows含实际DDL字节，schema含列及排序后的隐式索引元组。
fn schema(c: &Connection) -> Result<(Value, Value), String> {
    let mut stmt = c
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name")
        .map_err(sql_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    let mut tables = serde_json::Map::new();
    for (kind, name, _, sql) in &rows {
        if kind == "index" {
            if sql.is_some() {
                return Err(err("未声明的显式索引"));
            }
            continue;
        }
        if kind != "table" {
            return Err(err("未声明视图或触发器"));
        }
        let mut q=c.prepare("SELECT cid,name,type,\"notnull\",dflt_value,pk FROM pragma_table_info(?1) ORDER BY cid").map_err(sql_err)?;
        let columns = q
            .query_map([name], |r| {
                Ok(json!([
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, i64>(5)?
                ]))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let mut q = c
            .prepare("SELECT name,\"unique\",origin,partial FROM pragma_index_list(?1)")
            .map_err(sql_err)?;
        let idx = q
            .query_map([name], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let mut indexes = Vec::new();
        for (name, unique, origin, partial) in idx {
            let cols = c
                .prepare("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
                .map_err(sql_err)?
                .query_map([name], |r| r.get::<_, Option<String>>(0))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            indexes.push((unique, origin, partial, cols));
        }
        indexes.sort();
        tables.insert(name.clone(), json!([columns, indexes]));
    }
    Ok((json!(rows), Value::Object(tables)))
}

fn capture_image(tx: &Transaction<'_>, max: u64, deadline: Instant) -> Result<Image, String> {
    check(deadline, "Capture")?;
    // 在获取DDL/行载荷前预检；数量上限来自同一组固定DDL产生的真实模式。
    let ddls = control_audit::schema_ddls(tx)?;
    let expected = Connection::open_in_memory().map_err(sql_err)?;
    for ddl in &ddls {
        expected.execute_batch(ddl).map_err(sql_err)?;
    }
    let (expected_rows, expected_schema) = schema(&expected)?;
    let (count, bytes): (i64, i64) = tx
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(length(CAST(sql AS BLOB))),0) FROM sqlite_schema",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(sql_err)?;
    if count < 0
        || bytes < 0
        || count > expected_rows.as_array().unwrap().len() as i64
        || bytes > max.min(128 * 1024) as i64
    {
        return Err(err("Incomplete/CaptureBytes：模式元数据超过捕获界限"));
    }
    control_audit::verify_s3_schema(tx)?;
    let (schema_rows, schema) = schema(tx)?;
    if schema != expected_schema {
        return Err(err("列/主键/隐式索引与固定DDL不同"));
    }
    let header = canonical_json(&json!({"schema":schema,"schema_rows":schema_rows})).into_bytes();
    let mut image = Image {
        header,
        tables: Vec::new(),
        ddls,
        digest: String::new(),
        captured_bytes: 0,
    };
    image.check_bytes(max)?;
    let mut digest = Sha256::new();
    digest.update(&image.header);
    for (name, meta) in schema.as_object().unwrap() {
        check(deadline, "Capture")?;
        let columns = meta[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c[1].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        let mut pk = meta[0]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c[5].as_i64().unwrap() > 0)
            .collect::<Vec<_>>();
        pk.sort_by_key(|c| c[5].as_i64().unwrap());
        let order = if name == "raw_events" {
            "seq".into()
        } else {
            pk.iter()
                .map(|c| quote(c[1].as_str().unwrap()))
                .collect::<Vec<_>>()
                .join(",")
        };
        if order.is_empty() {
            return Err(err("固定表缺少确定主键顺序"));
        }
        let lengths = columns
            .iter()
            .map(|c| format!("COALESCE(length(CAST({} AS BLOB)),0)", quote(c)))
            .collect::<Vec<_>>()
            .join("+");
        let oversized: bool = tx
            .query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM {} WHERE {lengths}>?1)",
                    quote(name)
                ),
                [max as i64],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        if oversized {
            return Err(err("Incomplete/CaptureBytes：单条持久记录超过捕获上限"));
        }
        digest.update(canonical_json(&json!(name)).as_bytes());
        let without_rowid: bool = tx
            .query_row(
                "SELECT wr FROM pragma_table_list WHERE schema='main' AND name=?1",
                [name],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        let rowid_offset = usize::from(!without_rowid);
        let mut q = tx
            .prepare(&format!(
                "SELECT {}{} FROM {} ORDER BY {order}",
                if without_rowid { "" } else { "rowid," },
                columns
                    .iter()
                    .map(|c| quote(c))
                    .collect::<Vec<_>>()
                    .join(","),
                quote(name)
            ))
            .map_err(sql_err)?;
        let mut rows = q.query([]).map_err(sql_err)?;
        let old_capacity = image.tables.capacity();
        image.tables.push(Table {
            name: name.clone(),
            columns,
            rows: Vec::new(),
            rowids: if without_rowid {
                None
            } else {
                Some(Vec::new())
            },
        });
        let table = image.tables.last().unwrap();
        let added = ((image.tables.capacity() - old_capacity) * size_of::<Table>()) as u64
            + table.name.capacity() as u64
            + (table.columns.capacity() * size_of::<String>()) as u64
            + table
                .columns
                .iter()
                .map(|c| c.capacity() as u64)
                .sum::<u64>();
        image.charge(added, max)?;
        while let Some(row) = rows.next().map_err(sql_err)? {
            check(deadline, "Capture")?;
            let n = image.tables.last().unwrap().columns.len();
            let payload = (0..n).try_fold(0u64, |sum, i| -> Result<_, String> {
                Ok(sum
                    + match row.get_ref(i + rowid_offset).map_err(sql_err)? {
                        ValueRef::Text(b) | ValueRef::Blob(b) => b.len() as u64,
                        _ => 0,
                    })
            })?;
            if image
                .captured_bytes
                .saturating_add(payload)
                .saturating_add((n * size_of::<Cell>() + size_of::<Vec<Cell>>()) as u64)
                > max
            {
                return Err(err("Incomplete/CaptureBytes：全域捕获超过事前上限"));
            }
            let mut cells = Vec::with_capacity(n);
            for i in 0..n {
                cells.push(match row.get_ref(i + rowid_offset).map_err(sql_err)? {
                    ValueRef::Null => Cell::Null,
                    ValueRef::Integer(v) => Cell::Integer(v),
                    ValueRef::Text(b) => {
                        Cell::Text(std::str::from_utf8(b).map_err(err)?.to_owned())
                    }
                    ValueRef::Blob(b) => Cell::Blob(b.to_vec()),
                    ValueRef::Real(_) => return Err(err("REAL不属于已声明持久列类型")),
                });
            }
            digest.update(b"[");
            for cell in &cells {
                hash_cell(&mut digest, cell)?;
            }
            digest.update(b"]");
            let owned = (cells.capacity() * size_of::<Cell>()) as u64
                + cells
                    .iter()
                    .map(|v| match v {
                        Cell::Text(s) => s.capacity() as u64,
                        Cell::Blob(b) => b.capacity() as u64,
                        _ => 0,
                    })
                    .sum::<u64>();
            let table = image.tables.last_mut().unwrap();
            let old_capacity = table.rows.capacity();
            table.rows.push(cells);
            let rowid_bytes = if let Some(ids) = &mut table.rowids {
                let old = ids.capacity();
                ids.push(row.get(0).map_err(sql_err)?);
                ((ids.capacity() - old) * size_of::<i64>()) as u64
            } else {
                0
            };
            let added = owned
                + rowid_bytes
                + ((table.rows.capacity() - old_capacity) * size_of::<Vec<Cell>>()) as u64;
            image.charge(added, max)?;
        }
    }
    check(deadline, "Capture")?;
    image.digest = format!("{:x}", digest.finalize());
    image.charge(image.digest.capacity() as u64, max)?;
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
                + (table.columns.capacity() * size_of::<String>()) as u64
                + table
                    .columns
                    .iter()
                    .map(|c| c.capacity() as u64)
                    .sum::<u64>()
                + (table.rows.capacity() * size_of::<Vec<Cell>>()) as u64;
            for row in &table.rows {
                n += (row.capacity() * size_of::<Cell>()) as u64;
                for cell in row {
                    n += match cell {
                        Cell::Text(t) => t.capacity() as u64,
                        Cell::Blob(b) => b.capacity() as u64,
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
fn hash_cell(hash: &mut Sha256, cell: &Cell) -> Result<(), String> {
    match cell {
        Cell::Null => {
            hash.update(b"NoneType");
            hash.update(4u64.to_be_bytes());
            hash.update(b"null");
        }
        Cell::Integer(n) => {
            let s = n.to_string();
            hash.update(b"int");
            hash.update((s.len() as u64).to_be_bytes());
            hash.update(s.as_bytes());
        }
        Cell::Blob(b) => {
            hash.update(b"b");
            hash.update((b.len() as u64).to_be_bytes());
            hash.update(b);
        }
        Cell::Text(s) => {
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
        Cell::Real(_) => return Err(err("REAL捕获单元不合法")),
    }
    Ok(())
}
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
    for table in &image.tables {
        let offset = usize::from(table.rowids.is_some());
        let marks = (1..=table.columns.len() + offset)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "INSERT INTO {} ({}{}) VALUES ({marks})",
            quote(&table.name),
            if offset == 0 { "" } else { "rowid," },
            table
                .columns
                .iter()
                .map(|c| quote(c))
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut insert = tx.prepare(&sql).map_err(sql_err)?;
        for (n, row) in table.rows.iter().enumerate() {
            check(deadline, "Verify")?;
            if let Some(ids) = &table.rowids {
                let id = Cell::Integer(ids[n]);
                insert
                    .execute(params_from_iter(std::iter::once(&id).chain(row.iter())))
                    .map_err(sql_err)?;
            } else {
                insert.execute(params_from_iter(row)).map_err(sql_err)?;
            }
        }
    }
    let foreign_errors: i64 = tx
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .map_err(sql_err)?;
    if foreign_errors != 0 {
        return Err(err("捕获内容外键闭合失败"));
    }
    tx.commit().map_err(sql_err)?;
    let digest = image.digest.clone();
    let captured_bytes = image.captured_bytes;
    drop(image);
    db.pragma_update(None, "query_only", true)
        .map_err(sql_err)?;
    let (audited, audit_receipt) = {
        let tx = db.transaction().map_err(sql_err)?;
        if let Some(job) = job {
            let (state, receipt) =
                state::audit_with_access(&tx, Some(deadline), Some(max_capture_bytes), job)?;
            (state, Some(receipt))
        } else {
            (
                state::audit_bounded(&tx, Some(deadline), Some(max_capture_bytes))?,
                None,
            )
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
    })
}

/// 每个worker独占的长期连接与至多一份完整缓存；data_version不跨连接比较。
/// 缓存无公开裸摘要入口；只有本次同连接版本/文件身份夹持成功才可借出。
pub(super) struct AuditReader {
    path: std::path::PathBuf,
    resources_digest: String,
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
    pub(super) fn new(path: std::path::PathBuf, resources: &q_resources::QResources) -> Self {
        Self {
            path,
            resources_digest: resources.digest().into(),
            source: None,
            identity: None,
            cached: None,
            stats: ReaderStats::default(),
        }
    }
    pub(super) fn close(&mut self) {
        self.cached = None;
        self.source = None;
        self.identity = None;
        self.stats.cached_bytes = 0;
    }
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
        if resources.digest() != self.resources_digest {
            self.close();
            return Err(err("reader资源与启动冻结配置不同"));
        }
        let hard = job.read_deadline().map_err(err)?;
        let result = self.prepare(resources, job, hard, &mut seam);
        let (mut packed, cache_version, verify_deadline) = match result {
            Ok(v) => v,
            Err(e) => {
                self.close();
                return Err(e);
            }
        };
        let proof = match packed.checkout(verify_deadline) {
            Ok(v) => v,
            Err(e) => {
                self.close();
                return Err(e);
            }
        };
        job.pre_io_check().map_err(err)?;
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
                self.close();
                self.source = Some(open_source(&self.path, limits, deadline)?);
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
                validate_source(source)?;
                let v0 = version(source)?;
                seam("after_v0");
                let tx = source.transaction().map_err(sql_err)?;
                tx.query_row(
                    "SELECT key,value FROM meta ORDER BY key LIMIT 1",
                    [],
                    |_| Ok(()),
                )
                .map_err(sql_err)?;
                seam("after_first_read");
                let v1 = version(&tx)?;
                let candidate = self
                    .cached
                    .as_ref()
                    .is_some_and(|(v, _)| *v == v0 && v0 == v1);
                let image = if candidate {
                    None
                } else {
                    Some(capture_image(&tx, limits.max_capture_bytes, deadline)?)
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
                let (_, packed) = self.cached.take().ok_or_else(|| err("缓存候选消失"))?;
                self.stats.cached_bytes = 0;
                packed.replay(job)?;
                self.stats.cache_hits += 1;
                job.pre_io_check().map_err(err)?;
                return Ok((packed, Some(versions[2]), verify_deadline));
            }
            self.cached = None;
            self.stats.cached_bytes = 0;
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
fn validate_source(source: &Connection) -> Result<(), String> {
    if !source.is_autocommit() {
        return Err(err("reader不得持有跨请求事务"));
    }
    let query_only: i64 = source
        .query_row("PRAGMA query_only", [], |r| r.get(0))
        .map_err(sql_err)?;
    let read_uncommitted: i64 = source
        .query_row("PRAGMA read_uncommitted", [], |r| r.get(0))
        .map_err(sql_err)?;
    if query_only != 1 || read_uncommitted != 0 {
        return Err(err("只读隔离设置改变"));
    }
    let names = source
        .prepare("PRAGMA database_list")
        .map_err(sql_err)?
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    if names != ["main"] {
        return Err(err("源连接含附加数据库"));
    }
    let mode: String = source
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(sql_err)?;
    if mode.to_lowercase() != "wal" {
        return Err(err("源库必须由S预先建立WAL，Q不替它改模式"));
    }
    Ok(())
}

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
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(Instant::now)).unwrap()
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
        let resources = q_resources::tests::resources();
        let mut job = job();
        let mut captured = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
        let digest = captured.capture.digest().to_string();
        let projection = captured
            .projection(1, projection::HistoryMode::AsKnown, &mut job)
            .unwrap();
        assert_eq!(projection.wire["fixed_cut"]["cut_generation"], "1");
        publish(&mut f, 2);
        let one = captured.delta_references(0, 2, &mut job).unwrap();
        assert_eq!(one.anchors.len(), 1);
        assert_eq!(one.through_generation, 1);
        assert!(!one.has_more);
        assert_eq!(one.validated_capture_digest, digest);
        assert_eq!(one.anchors[0]["generation"], "1");
        assert!(one.usage[0].nodes > 0 && one.usage[0].records > 0);
        let mut fresh = capture_configured(&f.db_path(), &resources, &mut job).unwrap();
        let first = fresh.delta_references(0, 1, &mut job).unwrap();
        assert_eq!(first.anchors, one.anchors);
        assert!(first.has_more);
        assert_ne!(first.validated_capture_digest, digest);
        let next = fresh.delta_references(1, 1, &mut job).unwrap();
        assert_eq!(next.through_generation, 2);
        assert!(!next.has_more);
        assert_eq!(next.anchors[0]["base_cut"], first.anchors[0]["next_cut"]);
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
        let resources = q_resources::tests::resources();
        let mut captured = capture_configured(&f.db_path(), &resources, &mut job()).unwrap();
        let all = captured.delta_references(0, 2, &mut job()).unwrap();
        assert!(all.usage[1].nodes > all.usage[0].nodes);
        let mut input = q_resources::tests::input();
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
        assert_eq!(first.anchors[0], all.anchors[0]);
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
            Ok((g, c.capture.digest().to_string(), canonical_json(&p.wire)))
        })
    }
    #[test]
    fn worker_cache_reuses_only_same_connection_current_image_and_keeps_full_content() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let r = q_resources::tests::resources();
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
        let r = q_resources::tests::resources();
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
        let r = q_resources::tests::resources();
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
        let mut input = q_resources::tests::input();
        input["limits"]["max_cache_bytes"] = json!("1");
        let r = q_resources::QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap();
        let mut reader = AuditReader::new(f.db_path(), &r);
        let first = inspect_cached(&mut reader, &r, &mut job()).unwrap();
        let second = inspect_cached(&mut reader, &r, &mut job()).unwrap();
        assert_eq!(first, second);
        assert_eq!(reader.stats().captures, 2);
        assert_eq!(reader.stats().cache_hits, 0);
        assert_eq!(reader.stats().cached_bytes, 0);
        let mut input = q_resources::tests::input();
        input["limits"]["max_query_workers"] = json!("1");
        let baseline_resources = q_resources::tests::resources();
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
        let mut input = q_resources::tests::input();
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
        use sc::path::{lookup_path, PathStep};
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
                    let array = lookup_path(&mut source, root, &[PathStep::Field((*field).into())])
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
        let r = q_resources::tests::resources();
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
        let r = q_resources::tests::resources();
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
        let r = q_resources::tests::resources();
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
