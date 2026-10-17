//! #1394 C：S3 的内容节点持久边界，使用调用方的同一个 SQLite 事务。
//!
//! s-content/1 与旧 evidence-graph 码本分表；只有新 s-session/3 域可以初始化或读取。
//! 本层保存节点，不发布 cut、不核业务出生许可；完整闭包和独立索引仍由发布者核验。

use crate::v3::audit_store::{CodecRead, NativeAdmission};
use newchan_rust::s_content::{self, code, CodecError, NodeDigest, NodeRead, NodeSink, NodeSource};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use s_content::capture_allocation::scratch::{NativeReservation, WorkScope};
use std::collections::BTreeMap;

fn storage_error(error: impl std::fmt::Display) -> CodecError {
    CodecError::new(code::SOURCE, format!("S3 内容存储：{error}"))
}

/// 实际 SELECT 自有 guard 在文本转换内部先核 NOMEM，再开放借用。
fn query_text_equals(
    conn: &Connection,
    sql: &std::ffi::CStr,
    column_name: &'static str,
    expected: &str,
) -> rusqlite::Result<Option<bool>> {
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(conn, sql)?;
    let result = match stmt.next()? {
        Some(mut row) => Some(
            row.text(0).map_err(|cause| match cause {
                rusqlite::Error::InvalidColumnType(index, _, kind) => {
                    rusqlite::Error::InvalidColumnType(index, column_name.to_owned(), kind)
                }
                cause => cause,
            })? == expected,
        ),
        None => None,
    };
    stmt.finalize()?;
    Ok(result)
}

fn ensure_domain(conn: &Connection) -> Result<(), CodecError> {
    let protocol = query_text_equals(
        conn,
        c"SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
        "protocol_revision",
        "s-session/3",
    )
    .map_err(storage_error)?;
    if protocol != Some(true) {
        return Err(CodecError::new(
            code::FORMAT,
            "内容节点表只属于已登记的 s-session/3 域",
        ));
    }
    Ok(())
}

pub(super) fn ensure_format(conn: &Connection) -> Result<(), CodecError> {
    ensure_domain(conn)?;
    let format = query_text_equals(
        conn,
        c"SELECT content_format FROM s_content_meta WHERE singleton=1",
        "content_format",
        s_content::CONTENT_FORMAT,
    )
    .map_err(storage_error)?;
    if format != Some(true) {
        return Err(CodecError::new(
            code::FORMAT,
            "内容存储缺少 s-content/1 格式登记",
        ));
    }
    Ok(())
}

pub(super) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS s_content_meta (
        singleton INTEGER PRIMARY KEY CHECK(singleton=1), content_format TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS s_content_nodes (
        digest BLOB PRIMARY KEY CHECK(typeof(digest)='blob' AND length(digest)=32),
        tag INTEGER NOT NULL CHECK(tag>=0 AND tag<=255), body BLOB NOT NULL
    ) WITHOUT ROWID;";

/// 仅在调用者的新 S3 域事务内初始化；不会升级旧域或改写既存格式登记。
pub(super) fn init_schema(tx: &Transaction<'_>) -> Result<(), CodecError> {
    ensure_domain(tx)?;
    tx.execute_batch(SCHEMA).map_err(storage_error)?;
    tx.execute(
        "INSERT OR IGNORE INTO s_content_meta VALUES(1, ?1)",
        params![s_content::CONTENT_FORMAT],
    )
    .map_err(storage_error)?;
    ensure_format(tx)
}

fn check_digest(digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
    if s_content::frame(tag, body) != *digest {
        return Err(CodecError::new(
            code::NODE_DIGEST,
            format!("{} 的节点原件与摘要不符", s_content::root_string(digest)),
        ));
    }
    Ok(())
}

fn read_node(conn: &Connection, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
    match read_checked_node(conn, digest)? {
        None => Ok(NodeRead::Missing),
        Some(frame) => {
            let (tag, body) = frame.into_parts();
            Ok(NodeRead::Found { tag, body })
        }
    }
}

fn read_material(
    conn: &Connection,
    digest: &NodeDigest,
) -> Result<Option<(u8, Vec<u8>)>, CodecError> {
    read_material_admitted(conn, digest, None)
}

/// 借用经过 NOMEM 检查的 SQL 正文，取得父账许可后才生成 Rust 副本。
fn read_material_admitted(
    conn: &Connection,
    digest: &NodeDigest,
    work: Option<&WorkScope>,
) -> Result<Option<(u8, Vec<u8>)>, CodecError> {
    let result = read_material_reserved(conn, digest, work)?;
    Ok(result.map(|(tag, body, permit)| {
        if let Some(permit) = permit {
            permit.retain();
        }
        (tag, body)
    }))
}
fn read_material_reserved(
    conn: &Connection,
    digest: &NodeDigest,
    work: Option<&WorkScope>,
) -> Result<Option<(u8, Vec<u8>, Option<NativeReservation>)>, CodecError> {
    #[cfg(test)]
    SQL_READS.with(|calls| calls.set(calls.get() + 1));
    // 保留无 scope 路线原有的 statement cache；两路只改变正文取得/许可方式。
    let result: Option<(i64, Vec<u8>, Option<NativeReservation>)> = if work.is_none() {
        conn.prepare_cached("SELECT tag, body FROM s_content_nodes WHERE digest=?1")
            .map_err(storage_error)?
            .query_row(params![&digest[..]], |row| {
                Ok((row.get(0)?, row.get(1)?, None))
            })
            .optional()
            .map_err(storage_error)?
    } else {
        let mut statement = newchan_rust::sqlite_checked::CheckedStatement::prepare(
            conn,
            c"SELECT tag, body FROM s_content_nodes WHERE digest=?1",
        )
        .map_err(storage_error)?;
        statement.bind_blob(1, digest).map_err(storage_error)?;
        let row = match statement.next().map_err(storage_error)? {
            Some(mut row) => {
                let tag = row.integer(0).map_err(storage_error)?;
                let (body, permit) =
                    native_body_reserved(work, row.blob(1).map_err(storage_error)?)?;
                Some((tag, body, permit))
            }
            None => None,
        };
        statement.finalize().map_err(storage_error)?;
        row
    };
    let Some(row) = result else {
        return Ok(None);
    };
    // 失败前保持聚合所有权：body 字段先析构，permit 字段随后归还。
    let tag = u8::try_from(row.0)
        .map_err(|_| CodecError::new(code::NODE_DIGEST, "节点 tag 超过一个字节"))?;
    let (_, body, permit) = row;
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    RUNTIME_READS.with(|reads| {
        if let Some(reads) = reads.borrow_mut().as_mut() {
            reads.insert(*digest);
        }
    });
    Ok(Some((tag, body, permit)))
}

/// Native 正文复制在许可之后；无 scope 的旧调用维持同一复制语义。
pub(super) fn native_body_copy(
    work: Option<&WorkScope>,
    body: &[u8],
) -> Result<Vec<u8>, CodecError> {
    let (body, permit) = native_body_reserved(work, body)?;
    if let Some(permit) = permit {
        permit.retain();
    }
    Ok(body)
}
pub(super) fn native_body_reserved(
    work: Option<&WorkScope>,
    body: &[u8],
) -> Result<(Vec<u8>, Option<NativeReservation>), CodecError> {
    let permit = work.map(|w| w.reserve_native(body.len())).transpose()?;
    let mut result = Vec::new();
    result.try_reserve_exact(body.len()).map_err(|_| {
        work.map_or_else(
            || CodecError::new(s_content::capture_allocation::LIMIT, "Native正文分配失败"),
            |w| w.reject(s_content::capture_allocation::scratch::ScratchFailure::Allocation),
        )
    })?;
    result.extend_from_slice(body);
    Ok((result, permit))
}
/// 新整树上界先许可，旧许可仍持有；插入完成后用新许可承接现有树。
/// 避免把每次可能 split 的最坏路径当成永久新增节点，累计成 n log n 的虚费。
pub(super) fn native_tree_storage<K, V>(
    work: Option<&WorkScope>,
    len: usize,
) -> Result<Option<NativeReservation>, CodecError> {
    let Some(work) = work else {
        return Ok(None);
    };
    #[cfg(feature = "s_capture_native_testonly")]
    {
        work.reserve_native(crate::v3::audit_retention::tree_slots::<K, V>(len))
            .map(Some)
    }
    #[cfg(not(feature = "s_capture_native_testonly"))]
    {
        let _ = len;
        Err(work.reject(s_content::capture_allocation::scratch::ScratchFailure::Ledger))
    }
}

fn read_checked_node_scoped(
    conn: &Connection,
    digest: &NodeDigest,
    work: Option<&WorkScope>,
) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
    match read_material_reserved(conn, digest, work)? {
        None => Ok(None),
        Some((tag, body, permit)) => {
            let frame = s_content::job::CheckedFrame::checked(digest, tag, body)?;
            Ok(Some(s_content::job::ScopedFrame::new(frame, permit)))
        }
    }
}

fn read_checked_node_admitted(
    conn: &Connection,
    digest: &NodeDigest,
    work: Option<&WorkScope>,
) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
    match read_material_admitted(conn, digest, work)? {
        None => Ok(None),
        Some((tag, body)) => s_content::job::CheckedFrame::checked(digest, tag, body).map(Some),
    }
}

#[cfg(test)]
thread_local! { static SQL_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
pub(super) fn sql_reads_on_thread() -> u64 {
    SQL_READS.with(std::cell::Cell::get)
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
thread_local! { static RUNTIME_READS: std::cell::RefCell<Option<std::collections::BTreeSet<NodeDigest>>> = const { std::cell::RefCell::new(None) }; }
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) fn begin_runtime_reads() {
    RUNTIME_READS.with(|reads| {
        assert!(reads.borrow().is_none());
        *reads.borrow_mut() = Some(Default::default());
    });
}
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) fn end_runtime_reads() -> Vec<NodeDigest> {
    RUNTIME_READS.with(|reads| reads.borrow_mut().take().unwrap().into_iter().collect())
}

fn read_checked_node(
    conn: &Connection,
    digest: &NodeDigest,
) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
    match read_material(conn, digest)? {
        None => Ok(None),
        Some((tag, body)) => s_content::job::CheckedFrame::checked(digest, tag, body).map(Some),
    }
}

// 仅限制优化资格；不是请求预算，也不是全部 heap/RSS 上界。
const SAME_MATERIAL_BYTES: usize = 4 * 1024 * 1024;
const SAME_MATERIAL_ENTRIES: usize = 65_536;
#[derive(Default)]
struct SameMaterial {
    frames: BTreeMap<NodeDigest, (s_content::job::CheckedFrame, usize)>,
    retained_capacity: usize,
    // frames 先析构；许可只在整棵实际树释放后归还。
    tree_permit: Option<NativeReservation>,
}

impl SameMaterial {
    fn check(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> Result<s_content::job::CheckedFrame, CodecError> {
        self.check_with_limits(
            digest,
            tag,
            body,
            SAME_MATERIAL_BYTES,
            SAME_MATERIAL_ENTRIES,
        )
    }

    fn check_with_limits(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
        max_bytes: usize,
        max_entries: usize,
    ) -> Result<s_content::job::CheckedFrame, CodecError> {
        self.check_with_limits_admitted(digest, tag, body, max_bytes, max_entries, None)
    }
    fn check_with_limits_admitted(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
        max_bytes: usize,
        max_entries: usize,
        work: Option<&WorkScope>,
    ) -> Result<s_content::job::CheckedFrame, CodecError> {
        let body = if let Some((old, _)) = self.frames.get(digest) {
            match old.try_rebind_same_material(digest, tag, body) {
                s_content::job::SameMaterialRebind::Same(frame) => return Ok(frame),
                s_content::job::SameMaterialRebind::Changed(body) => {
                    let (_, capacity) = self.frames.remove(digest).expect("已找到的局部素材");
                    self.retained_capacity -= capacity;
                    body
                }
            }
        } else {
            body
        };
        let frame = s_content::job::CheckedFrame::checked(digest, tag, body)?;
        // 复制前先按长度/条数筛选，复制后再按 Vec 的实际 capacity 筛选。
        let remaining = max_bytes.saturating_sub(self.retained_capacity);
        if self.frames.len() < max_entries && frame.body().len() <= remaining {
            let next = self
                .frames
                .len()
                .checked_add(1)
                .ok_or_else(|| NativeAdmission(work).native_overflow())?;
            let tree_permit = native_tree_storage::<
                NodeDigest,
                (s_content::job::CheckedFrame, usize),
            >(work, next)?;
            let retained = native_body_copy(work, frame.body())?;
            let capacity = retained.capacity();
            if capacity <= remaining {
                let proof = frame.checked_same_material(digest, tag, retained)?;
                self.frames.insert(*digest, (proof, capacity));
                self.tree_permit = tree_permit;
                self.retained_capacity += capacity;
            }
        }
        Ok(frame)
    }
}

/// 直接 PRAGMA 的 metadata 也属于证据：SQLITE_IGNORE 可成功返回零列，必须拒绝。
/// 所有行严格取型并读到末尾；正常空列表仅在列名/列数已核实后才成立。
#[cfg(feature = "s_capture_native_testonly")]
fn native_pragma_rows<T>(
    conn: &Connection,
    sql: &str,
    columns: &[&str],
    mut decode: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, CodecError> {
    let mut stmt = conn.prepare(sql).map_err(storage_error)?;
    let result = (|| {
        if stmt.column_names() != columns {
            return Err(storage_error(format!("Native PRAGMA metadata 不符：{sql}")));
        }
        let mut rows = stmt.query([]).map_err(storage_error)?;
        let mut values = Vec::new();
        while let Some(row) = rows.next().map_err(storage_error)? {
            values.push(decode(row).map_err(storage_error)?);
        }
        Ok(values)
    })();
    let finalized = stmt.finalize().map_err(storage_error);
    let values = result?;
    finalized?;
    Ok(values)
}

/// #1488 owner effects 复用同一节点保持判据，不另造宽严两档。
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn owner_node_insert_gate(tx: &Transaction<'_>) -> Result<bool, CodecError> {
    native_insert_can_preserve(tx)
}

#[cfg(feature = "s_capture_native_testonly")]
fn native_insert_can_preserve(conn: &Connection) -> Result<bool, CodecError> {
    fn tables(
        conn: &Connection,
        schema: &str,
    ) -> Result<Vec<(String, String, String, i64, i64, i64)>, CodecError> {
        native_pragma_rows(
            conn,
            &format!("PRAGMA {schema}.table_list"),
            &["schema", "name", "type", "ncol", "wr", "strict"],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
    }
    let main = tables(conn, "main")?;
    let temp = tables(conn, "temp")?;
    // 每个正常 temp.table_list 必含 schema 表，空结果不证明没有 TEMP 遮蔽。
    if !temp.iter().any(|r| {
        r.0 == "temp"
            && r.1 == "sqlite_temp_schema"
            && r.2 == "table"
            && r.3 == 5
            && r.4 == 0
            && r.5 == 0
    }) {
        return Err(storage_error("Native temp.table_list 缺少 schema 见证"));
    }
    let shadowed = temp.iter().any(|r| {
        r.1.eq_ignore_ascii_case("s_content_nodes") || r.1.eq_ignore_ascii_case("s_content_meta")
    });
    let normal = main
        .iter()
        .filter(|r| r.1.eq_ignore_ascii_case("s_content_nodes"))
        .collect::<Vec<_>>();
    if shadowed
        || normal.len() != 1
        || normal[0].0 != "main"
        || normal[0].1 != "s_content_nodes"
        || normal[0].2 != "table"
        || normal[0].3 != 3
        || normal[0].4 != 1
        || !matches!(normal[0].5, 0 | 1)
    {
        return Ok(false);
    }
    let columns: Vec<(i64, String, String, i64, Option<String>, i64, i64)> = native_pragma_rows(
        conn,
        "PRAGMA main.table_xinfo('s_content_nodes')",
        &[
            "cid",
            "name",
            "type",
            "notnull",
            "dflt_value",
            "pk",
            "hidden",
        ],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        },
    )?;
    let columns_ok = columns.len() == 3
        && columns
            .iter()
            .zip([
                (0, "digest", "BLOB", 1, 1),
                (1, "tag", "INTEGER", 1, 0),
                (2, "body", "BLOB", 1, 0),
            ])
            .all(|(r, expected)| {
                r.0 == expected.0
                    && r.1 == expected.1
                    && r.2.eq_ignore_ascii_case(expected.2)
                    && r.3 == expected.3
                    && r.4.is_none()
                    && r.5 == expected.4
                    && r.6 == 0
            });
    let foreign_keys: Vec<(
        i64,
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
    )> = native_pragma_rows(
        conn,
        "PRAGMA main.foreign_key_list('s_content_nodes')",
        &[
            "id",
            "seq",
            "table",
            "from",
            "to",
            "on_update",
            "on_delete",
            "match",
        ],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    )?;
    let indexes: Vec<(i64, String, i64, String, i64)> = native_pragma_rows(
        conn,
        "PRAGMA main.index_list('s_content_nodes')",
        &["seq", "name", "unique", "origin", "partial"],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let mut indexes_ok = indexes.len() == 1;
    for index in indexes {
        indexes_ok &= index.0 == 0 && index.2 == 1 && index.3 == "pk" && index.4 == 0;
        // 名称由 SQLite 返回；字符串引用必须转义，不能拼成任意 SQL。
        let quoted = index.1.replace('\'', "''");
        let info: Vec<(i64, i64, Option<String>, i64, Option<String>, i64)> = native_pragma_rows(
            conn,
            &format!("PRAGMA main.index_xinfo('{quoted}')"),
            &["seqno", "cid", "name", "desc", "coll", "key"],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )?;
        indexes_ok &= info.len() == 3
            && info
                .iter()
                .zip([(0, 0, "digest", 1), (1, 1, "tag", 0), (2, 2, "body", 0)])
                .all(|(r, expected)| {
                    r.0 == expected.0
                        && r.1 == expected.1
                        && r.2.as_deref() == Some(expected.2)
                        && r.3 == 0
                        && r.4.as_deref() == Some("BINARY")
                        && r.5 == expected.3
                });
    }
    Ok(columns_ok && foreign_keys.is_empty() && indexes_ok)
}

#[cfg(feature = "s_capture_native_testonly")]
fn native_insert_only_target(before: u64, after: u64, direct: u64) -> bool {
    direct == 1 && after.checked_sub(before) == Some(1)
}

/// #1394 Native prepare only: exclusive, snapshot-bound node-read phase. No SQL/connection
/// getter, Deref, savepoint, rollback or staged owner is exposed. Actual puts always stay cold.
#[cfg(feature = "s_capture_native_testonly")]
pub(super) struct NativeReadLease<'lease, 'conn> {
    tx: &'lease mut Transaction<'conn>,
    work: Option<&'lease WorkScope>,
    material: SameMaterial,
    poisoned: bool,
    insert_can_preserve: bool,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    stage: Option<&'lease super::staged_nodes::StageOwner<'lease>>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl<'lease, 'conn> NativeReadLease<'lease, 'conn> {
    pub(super) fn new(tx: &'lease mut Transaction<'conn>) -> Result<Self, CodecError> {
        Self::new_admitted(tx, None)
    }
    pub(super) fn new_admitted(
        tx: &'lease mut Transaction<'conn>,
        work: Option<&'lease WorkScope>,
    ) -> Result<Self, CodecError> {
        if tx.is_autocommit() {
            return Err(storage_error("Native读取租约要求活动事务"));
        }
        let isolation: Vec<i64> =
            native_pragma_rows(tx, "PRAGMA read_uncommitted", &["read_uncommitted"], |r| {
                r.get(0)
            })?;
        if isolation.as_slice() != [0] {
            return Err(storage_error(
                "Native读取租约要求read_uncommitted=0；不修改配置",
            ));
        }
        ensure_format(tx)?;
        let insert_can_preserve = native_insert_can_preserve(tx)?;
        Ok(Self {
            tx,
            work,
            material: Default::default(),
            poisoned: false,
            insert_can_preserve,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            stage: None,
        })
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn new_admitted_staged(
        tx: &'lease mut Transaction<'conn>,
        work: &'lease WorkScope,
        stage: &'lease super::staged_nodes::StageOwner<'lease>,
    ) -> Result<Self, CodecError> {
        stage.check_transaction(tx)?;
        if !stage
            .work()
            .is_some_and(|stage_work| std::ptr::eq(stage_work, work))
        {
            return Err(storage_error("暂存租约必须继承同一已签发工作域"));
        }
        let mut lease = Self::new_admitted(tx, Some(work))?;
        lease.stage = Some(stage);
        Ok(lease)
    }
    fn clear(&mut self) {
        self.material = SameMaterial::default();
    }
    fn check_live(&mut self) -> Result<(), CodecError> {
        if self.poisoned || self.tx.is_autocommit() {
            self.clear();
            self.poisoned = true;
            return Err(storage_error(
                "Native读取租约已失败或事务已结束；不得重试/回落",
            ));
        }
        Ok(())
    }
    fn current_checked(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        self.current_scoped(digest)
            .map(|f| f.map(s_content::job::ScopedFrame::into_frame))
    }
    fn current_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        self.check_live()?;
        let result = (|| {
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            if let Some(stage) = self.stage {
                stage.check_transaction(self.tx)?;
            }
            if let Some((frame, _)) = self.material.frames.get(digest) {
                let (body, permit) = native_body_reserved(self.work, frame.body())?;
                let frame = frame.checked_same_material(digest, frame.tag(), body)?;
                #[cfg(all(test, feature = "s_capture_native_testonly"))]
                if let Some(stage) = self.stage {
                    stage.check_original(digest, frame.tag(), frame.body())?;
                }
                return Ok(Some(s_content::job::ScopedFrame::new(frame, permit)));
            }
            match read_material_reserved(self.tx, digest, self.work)? {
                Some((tag, body, permit)) => {
                    let frame = self.material.check_with_limits_admitted(
                        digest,
                        tag,
                        body,
                        SAME_MATERIAL_BYTES,
                        SAME_MATERIAL_ENTRIES,
                        self.work,
                    )?;
                    #[cfg(all(test, feature = "s_capture_native_testonly"))]
                    if let Some(stage) = self.stage {
                        stage.check_original(digest, frame.tag(), frame.body())?;
                    }
                    Ok(Some(s_content::job::ScopedFrame::new(frame, permit)))
                }
                None => {
                    #[cfg(all(test, feature = "s_capture_native_testonly"))]
                    if let Some(stage) = self.stage {
                        return stage.read_scoped(digest);
                    }
                    Ok(None)
                }
            }
        })();
        if result.is_err() {
            self.clear();
            self.poisoned = true;
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            if let Some(stage) = self.stage {
                stage.reject_after_error();
            }
        }
        result
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl NodeSource for NativeReadLease<'_, '_> {
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        self.current_scoped(d)
    }
    fn codec_work(&self) -> Option<&WorkScope> {
        self.work
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.current_checked(digest).map(|frame| match frame {
            None => NodeRead::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_parts();
                NodeRead::Found { tag, body }
            }
        })
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        self.current_checked(digest)
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl NodeSink for NativeReadLease<'_, '_> {
    fn codec_sink_work(&self) -> Option<&WorkScope> {
        self.work
    }
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        self.check_live()?;
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(stage) = self.stage {
            let result = (|| {
                stage.check_transaction(self.tx)?;
                TxStore::new_with_stage(self.tx, Some(stage))?
                    .with_work(self.work)
                    .put(digest, tag, body)
            })();
            if result.is_err() {
                self.clear();
                self.poisoned = true;
            }
            // SQL未变：SameMaterial只持真实SQL素材，绝不缓存overlay。
            return result;
        }
        // Private short reborrow avoids a self-referential &mut Transaction + TxStore pair.
        // Constructor already checked the format once, exactly as original TxStore::new does.
        let material = &mut self.material;
        let tx = &*self.tx;
        let preserve = self.insert_can_preserve;
        let mut parked = None;
        let result = TxStore {
            tx,
            work: self.work,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            stage: None,
        }
        .put_before_write(digest, tag, body, || {
            let mut old = std::mem::take(material);
            if preserve {
                // PK REPLACE 隐式删除不计入 total_changes；唯一冲突键限定为 digest，
                // 因而即使 authorizer 将现有目标读成 Missing，也绝不恢复目标旧素材。
                if let Some((_, capacity)) = old.frames.remove(&digest) {
                    old.retained_capacity -= capacity;
                }
                parked = Some((old, tx.total_changes()));
            }
        });
        if result.is_ok() {
            if let Some((old, before)) = parked {
                let after = tx.total_changes();
                let direct = tx.changes();
                if native_insert_only_target(before, after, direct) {
                    self.material = old;
                }
            }
        }
        if result.is_err() {
            self.clear();
            self.poisoned = true;
        }
        result
    }
}

// Fixed publication operations only. No connection/SQL getter is exposed to the caller.
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn publication_insert_gate(tx: &mut Transaction<'_>) -> Result<bool, CodecError> {
    let lease = NativeReadLease::new(tx)?;
    Ok(lease.insert_can_preserve)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn publication_read_checked(
    tx: &Transaction<'_>,
    digest: &NodeDigest,
    work: Option<&WorkScope>,
) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
    read_checked_node_scoped(tx, digest, work)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(super) enum PublicationPutEffect {
    Noop,
    InsertPreservingBase,
    Unknown,
}
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn publication_put(
    tx: &Transaction<'_>,
    insert_can_preserve: bool,
    digest: NodeDigest,
    tag: u8,
    body: &[u8],
    work: Option<&WorkScope>,
) -> Result<PublicationPutEffect, CodecError> {
    let mut before_write = None;
    TxStore {
        tx,
        work,
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        stage: None,
    }
    .put_before_write(digest, tag, body, || {
        before_write = Some(tx.total_changes())
    })?;
    Ok(match before_write {
        None => PublicationPutEffect::Noop,
        Some(before)
            if insert_can_preserve
                && native_insert_only_target(before, tx.total_changes(), tx.changes()) =>
        {
            PublicationPutEffect::InsertPreservingBase
        }
        Some(_) => PublicationPutEffect::Unknown,
    })
}

pub(super) struct TxStore<'a, 'conn> {
    tx: &'a Transaction<'conn>,
    work: Option<&'a WorkScope>,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    stage: Option<&'a super::staged_nodes::StageOwner<'a>>,
}

impl<'a, 'conn> TxStore<'a, 'conn> {
    pub(super) fn with_work(mut self, work: Option<&'a WorkScope>) -> Self {
        self.work = work;
        self
    }
    pub(super) fn new(tx: &'a Transaction<'conn>) -> Result<Self, CodecError> {
        ensure_format(tx)?;
        Ok(Self {
            tx,
            work: None,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            stage: None,
        })
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn new_with_stage(
        tx: &'a Transaction<'conn>,
        stage: Option<&'a super::staged_nodes::StageOwner<'a>>,
    ) -> Result<Self, CodecError> {
        let mut store = Self::new(tx)?;
        if let Some(owner) = stage {
            owner.check_transaction(tx)?;
        }
        store.stage = stage;
        Ok(store)
    }
}

impl NodeSource for TxStore<'_, '_> {
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        let result = (|| {
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            if let Some(stage) = self.stage {
                stage.check_transaction(self.tx)?;
            }
            let current = read_checked_node_scoped(self.tx, d, self.work)?;
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            if let Some(stage) = self.stage {
                if let Some(frame) = &current {
                    stage.check_original(d, frame.tag(), frame.body())?;
                } else {
                    return stage.read_scoped(d);
                }
            }
            Ok(current)
        })();
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if result.is_err() {
            if let Some(stage) = self.stage {
                stage.reject_after_error();
            }
        }
        result
    }
    fn codec_work(&self) -> Option<&WorkScope> {
        self.work
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.checked_read(digest).map(|frame| match frame {
            None => NodeRead::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_parts();
                NodeRead::Found { tag, body }
            }
        })
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        self.checked_read_scoped(digest)
            .map(|frame| frame.map(s_content::job::ScopedFrame::into_frame))
    }
}

impl TxStore<'_, '_> {
    fn put_before_write(
        &mut self,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
        before_write: impl FnOnce(),
    ) -> Result<(), CodecError> {
        let result = self.put_before_write_inner(digest, tag, body, before_write);
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if result.is_err() {
            if let Some(stage) = self.stage {
                stage.reject_after_error();
            }
        }
        result
    }
    fn put_before_write_inner(
        &mut self,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
        before_write: impl FnOnce(),
    ) -> Result<(), CodecError> {
        check_digest(&digest, tag, body)?;
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(stage) = self.stage {
            stage.check_transaction(self.tx)?;
            stage.check_original(&digest, tag, body)?;
        }
        #[cfg(feature = "s_capture_native_testonly")]
        let before = self.tx.total_changes();
        let current = read_checked_node_scoped(self.tx, &digest, self.work)?;
        #[cfg(feature = "s_capture_native_testonly")]
        let inserted = current.is_none();
        match current {
            Some(frame) => {
                if frame.tag() != tag || frame.body() != body {
                    return Err(CodecError::new(
                        code::NODE_DIGEST,
                        "同摘要异原件：保留旧节点并拒绝写入",
                    ));
                }
            }
            None => {
                before_write();
                #[cfg(all(test, feature = "s_capture_native_testonly"))]
                if let Some(stage) = self.stage {
                    return stage.put_new(digest, tag, body);
                }
                self.tx
                    .prepare_cached(
                        "INSERT INTO s_content_nodes(digest, tag, body) VALUES(?1, ?2, ?3)",
                    )
                    .map_err(storage_error)?
                    .execute(params![&digest[..], i64::from(tag), body])
                    .map_err(storage_error)?;
            }
        }
        #[cfg(feature = "s_capture_native_testonly")]
        crate::v3::audit_owner::note_persisted_node(self.tx, digest, before, inserted);
        Ok(())
    }
}

impl NodeSink for TxStore<'_, '_> {
    fn codec_sink_work(&self) -> Option<&WorkScope> {
        self.work
    }
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        self.put_before_write(digest, tag, body, || {})
    }
}

pub(super) struct ReadStore<'a> {
    conn: &'a Connection,
    same_material: Option<SameMaterial>,
}

impl<'a> ReadStore<'a> {
    /// 整次闭包读取共用同一事务快照；调用者不得逐节点重新创建事务。
    pub(super) fn new(tx: &'a Transaction<'_>) -> Result<Self, CodecError> {
        ensure_format(tx)?;
        Ok(Self {
            conn: tx,
            same_material: None,
        })
    }

    /// 仅完整 audit 的无计费分支显式启用；局部素材随 reader 销毁。
    pub(super) fn enable_audit_same_material(&mut self) {
        self.same_material = Some(SameMaterial::default());
    }
}

impl NodeSource for ReadStore<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_node(self.conn, digest)
    }

    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        let Some(material) = self.same_material.as_mut() else {
            return read_checked_node(self.conn, digest);
        };
        match read_material(self.conn, digest)? {
            None => Ok(None),
            Some((tag, body)) => material.check(digest, tag, body).map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled<'a>(tx: &'a Transaction<'_>) -> ReadStore<'a> {
        let mut reader = ReadStore::new(tx).unwrap();
        reader.enable_audit_same_material();
        reader
    }

    #[test]
    fn same_material_reads_sql_twice_hashes_once_and_owns_current_bytes() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let body = vec![1, 2, 3];
        let digest = s_content::frame(s_content::tag::BYTES_CHUNK, &body);
        TxStore::new(&tx)
            .unwrap()
            .put(digest, s_content::tag::BYTES_CHUNK, &body)
            .unwrap();
        let mut reader = enabled(&tx);
        let sql_before = SQL_READS.with(std::cell::Cell::get);
        let f_before = s_content::frame_calls_on_thread();
        let first = reader.checked_read(&digest).unwrap().unwrap();
        let usage = *first.usage();
        let (_, mut returned) = first.into_parts();
        returned[0] = 255;
        let second = reader.checked_read(&digest).unwrap().unwrap();
        assert_eq!(second.body(), body);
        assert_eq!(second.usage(), &usage);
        assert_eq!(SQL_READS.with(std::cell::Cell::get) - sql_before, 2);
        assert_eq!(s_content::frame_calls_on_thread() - f_before, 1);
        let mut other = enabled(&tx);
        let before = s_content::frame_calls_on_thread();
        assert!(other.checked_read(&digest).unwrap().is_some());
        assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        let before = s_content::frame_calls_on_thread();
        assert!(matches!(
            reader.read(&digest).unwrap(),
            NodeRead::Found { .. }
        ));
        assert_eq!(
            s_content::frame_calls_on_thread() - before,
            1,
            "read保留原F路径"
        );
    }

    #[test]
    fn same_material_actual_transaction_changes_recheck_and_remove_stale_proof() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let digest = s_content::frame(s_content::tag::BYTES_CHUNK, &[7]);
        TxStore::new(&tx)
            .unwrap()
            .put(digest, s_content::tag::BYTES_CHUNK, &[7])
            .unwrap();
        let mut reader = enabled(&tx);
        assert!(reader.checked_read(&digest).unwrap().is_some());
        assert_eq!(reader.same_material.as_ref().unwrap().retained_capacity, 1);
        tx.execute(
            "UPDATE s_content_nodes SET body=x'08' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        let before = s_content::frame_calls_on_thread();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::NODE_DIGEST
        );
        assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        assert!(reader.same_material.as_ref().unwrap().frames.is_empty());
        assert_eq!(reader.same_material.as_ref().unwrap().retained_capacity, 0);
        tx.execute(
            "UPDATE s_content_nodes SET body=x'07' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert!(reader.checked_read(&digest).unwrap().is_some());
        tx.execute(
            "UPDATE s_content_nodes SET tag=0 WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::NODE_DIGEST
        );
    }

    #[test]
    fn same_material_current_missing_sql_error_and_tag_error_never_fall_back() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(digest, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut reader = enabled(&tx);
        assert!(reader.checked_read(&digest).unwrap().is_some());
        tx.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert!(reader.checked_read(&digest).unwrap().is_none());
        tx.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        tx.execute(
            "INSERT INTO s_content_nodes VALUES(?1, 256, x'')",
            params![&digest[..]],
        )
        .unwrap();
        let before = s_content::frame_calls_on_thread();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::NODE_DIGEST
        );
        assert_eq!(
            s_content::frame_calls_on_thread(),
            before,
            "tag转换先于复用和F"
        );
        tx.execute_batch("DROP TABLE s_content_nodes").unwrap();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::SOURCE
        );
    }

    #[test]
    fn same_material_external_commit_visible_between_short_reads() {
        let path =
            std::env::temp_dir().join(format!("nc-1394-material-{}.sqlite", std::process::id()));
        let mut conn = Connection::open(&path).unwrap();
        conn.execute_batch(super::super::v2::SCHEMA_V2).unwrap();
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1, 's-session/3', '1', 'clock', '{}', 1, 'accept')",
            [],
        )
        .unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL").unwrap();
        let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        {
            let tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            TxStore::new(&tx)
                .unwrap()
                .put(digest, s_content::tag::BOOL_TRUE, &[])
                .unwrap();
            tx.commit().unwrap();
        }
        let external = Connection::open(&path).unwrap();
        // 私有夹具接受 Connection，不扩大生产new；每次SELECT是独立短读，无长事务旧快照。
        let mut reader = ReadStore {
            conn: &conn,
            same_material: Some(SameMaterial::default()),
        };
        assert!(reader.checked_read(&digest).unwrap().is_some());
        external
            .execute(
                "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
                params![&digest[..]],
            )
            .unwrap();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::NODE_DIGEST
        );
        external
            .execute(
                "UPDATE s_content_nodes SET body=x'',tag=?1 WHERE digest=?2",
                params![s_content::tag::BOOL_FALSE, &digest[..]],
            )
            .unwrap();
        assert_eq!(
            reader.checked_read(&digest).unwrap_err().code(),
            code::NODE_DIGEST
        );
        drop(reader);
        drop(external);
        drop(conn);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn same_material_qualification_full_and_oversized_keep_strict_checks() {
        let tag = s_content::tag::BYTES_CHUNK;
        let digest = s_content::frame(tag, &[1, 2]);
        let mut material = SameMaterial::default();
        for _ in 0..2 {
            let before = s_content::frame_calls_on_thread();
            material
                .check_with_limits(&digest, tag, vec![1, 2], 1, 1)
                .unwrap();
            assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        }
        assert!(material.frames.is_empty());
        material
            .check_with_limits(&digest, tag, vec![1, 2], 2, 1)
            .unwrap();
        let second = s_content::frame(tag, &[3]);
        for _ in 0..2 {
            let before = s_content::frame_calls_on_thread();
            material
                .check_with_limits(&second, tag, vec![3], 2, 1)
                .unwrap();
            assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        }
        assert_eq!(material.frames.len(), 1);
        assert_eq!(material.retained_capacity, 2);
        assert_eq!(
            material
                .check_with_limits(&second, tag, vec![4], 2, 1)
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        assert_eq!(
            material
                .check_with_limits(&digest, tag, vec![2, 1], 2, 1)
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        assert_eq!(material.retained_capacity, 0);
        assert!(material.frames.is_empty());
        material
            .check_with_limits(&second, tag, vec![3], 2, 1)
            .unwrap();
        assert_eq!(
            material.retained_capacity, 1,
            "移除坏素材正确回减，后续合法原件可保存"
        );
    }

    #[test]
    fn same_material_changed_root_and_node_role_still_use_current_verifier() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let mut store = TxStore::new(&tx).unwrap();
        let value = s_content::write_value(&mut store, &s_content::Value::Bool(true)).unwrap();
        let replacement =
            s_content::write_value(&mut store, &s_content::Value::Bool(false)).unwrap();
        let array = s_content::write_value(
            &mut store,
            &s_content::Value::Array(vec![s_content::Value::Bool(true)]),
        )
        .unwrap();
        let leaf = s_content::frame(s_content::tag::SEQ_LEAF, &value.digest);
        let mut reader = enabled(&tx);
        s_content::verify(&mut reader, &value.content_ref()).unwrap();
        assert_eq!(
            reader
                .checked_read(&replacement.digest)
                .unwrap()
                .unwrap()
                .digest(),
            &replacement.digest
        );
        let mut wrong_header = value.content_ref();
        wrong_header.root = s_content::root_string(&replacement.digest);
        wrong_header.kind = s_content::Kind::String;
        assert!(s_content::verify(&mut reader, &wrong_header).is_err());
        s_content::verify(&mut reader, &array.content_ref()).unwrap();
        assert!(reader
            .same_material
            .as_ref()
            .unwrap()
            .frames
            .contains_key(&leaf));
        let before = s_content::frame_calls_on_thread();
        let err = s_content::verify(
            &mut reader,
            &s_content::ContentRef::new(s_content::Kind::Bool, &leaf, None),
        )
        .unwrap_err();
        assert_eq!(err.code(), code::VALUE_KIND);
        assert_eq!(
            s_content::frame_calls_on_thread(),
            before,
            "复用F但当前Value角色仍拒绝"
        );
    }

    #[test]
    fn checked_sqlite_read_hashes_once_and_rechecks_current_row() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(digest, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut reader = ReadStore::new(&tx).unwrap();
        for _ in 0..2 {
            let before = s_content::frame_calls_on_thread();
            let checked = reader.checked_read(&digest).unwrap().unwrap();
            assert_eq!(checked.digest(), &digest);
            assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        }
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert_eq!(
            reader.checked_read(&digest).err().unwrap().code(),
            code::NODE_DIGEST
        );
        tx.execute(
            "UPDATE s_content_nodes SET tag=?1,body=x'' WHERE digest=?2",
            params![s_content::tag::BOOL_FALSE, &digest[..]],
        )
        .unwrap();
        assert_eq!(
            reader.checked_read(&digest).err().unwrap().code(),
            code::NODE_DIGEST
        );
        tx.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        let before = s_content::frame_calls_on_thread();
        assert!(reader.checked_read(&digest).unwrap().is_none());
        assert_eq!(s_content::frame_calls_on_thread(), before);
    }

    #[test]
    fn checked_txstore_read_hashes_once_without_retaining_a_frame() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        let mut store = TxStore::new(&tx).unwrap();
        store.put(digest, s_content::tag::BOOL_TRUE, &[]).unwrap();
        let before = s_content::frame_calls_on_thread();
        assert_eq!(
            store.checked_read(&digest).unwrap().unwrap().digest(),
            &digest
        );
        assert_eq!(s_content::frame_calls_on_thread() - before, 1);
        tx.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert!(store.checked_read(&digest).unwrap().is_none());
    }

    // 使用 S 的实际协议元数据表形状；本夹具只证明节点存储，不证明完整 clock/session 初始化。
    fn database(protocol: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(super::super::v2::SCHEMA_V2).unwrap();
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1, ?1, '1', 'test-clock', '{}', 1, 'accept')",
            params![protocol],
        )
        .unwrap();
        conn
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_read_lease_sql_f_headers_and_cold_lifecycle() {
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let mut plain = TxStore::new(&tx).unwrap();
        let root = s_content::write_value(
            &mut plain,
            &s_content::Value::Array(vec![s_content::Value::Bool(true)]),
        )
        .unwrap()
        .content_ref();
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        plain.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
        drop(plain);
        let digest = root.digest().unwrap();
        let before = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        let mut ordinary = TxStore::new(&tx).unwrap();
        for _ in 0..8 {
            ordinary.checked_read(&digest).unwrap().unwrap();
        }
        drop(ordinary);
        assert_eq!(sql_reads_on_thread() - before, 8);
        assert_eq!(s_content::frame_calls_on_thread() - f, 8);
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        let before = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        for _ in 0..8 {
            lease.checked_read(&digest).unwrap().unwrap();
        }
        assert_eq!(sql_reads_on_thread() - before, 1);
        assert_eq!(s_content::frame_calls_on_thread() - f, 1);
        // A 已成功读；B 的无写入 put 必须冷查 SQL/F，且保留 A 的成功读素材。
        for cached_b in [false, true] {
            if cached_b {
                lease.checked_read(&b).unwrap().unwrap();
            }
            let before = sql_reads_on_thread();
            let f = s_content::frame_calls_on_thread();
            lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
            assert_eq!(sql_reads_on_thread() - before, 1, "put 总是实际 SELECT");
            assert_eq!(s_content::frame_calls_on_thread() - f, 2, "新旧原件各核 F");
            assert!(lease.material.frames.contains_key(&digest));
            lease.checked_read(&digest).unwrap().unwrap();
            assert_eq!(sql_reads_on_thread() - before, 1, "A 仍命中租约素材");
            assert_eq!(s_content::frame_calls_on_thread() - f, 2);
        }
        s_content::verify(&mut lease, &root).unwrap();
        for bad in [
            s_content::ContentRef {
                format: "bad".into(),
                ..root.clone()
            },
            s_content::ContentRef {
                kind: s_content::Kind::Object,
                ..root.clone()
            },
            s_content::ContentRef {
                extent: Some(2),
                ..root.clone()
            },
        ] {
            assert!(s_content::verify(&mut lease, &bad).is_err());
        }
        let missing = [0; 32];
        assert!(matches!(lease.read(&missing).unwrap(), NodeRead::Missing));
        assert!(!lease.material.frames.contains_key(&missing));
        drop(lease);
        let mut fresh = NativeReadLease::new(&mut tx).unwrap();
        assert!(fresh.material.frames.is_empty());
        let before = sql_reads_on_thread();
        fresh.checked_read(&digest).unwrap();
        assert_eq!(sql_reads_on_thread() - before, 1);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_preserves_only_prior_other_reads_and_keeps_put_cold() {
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        assert!(lease.insert_can_preserve);
        lease.checked_read(&a).unwrap();
        let sql = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        let changes = lease.tx.total_changes();
        lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
        assert_eq!(sql_reads_on_thread() - sql, 1);
        assert_eq!(
            s_content::frame_calls_on_thread() - f,
            1,
            "put 入参核 F；Missing 无旧 F"
        );
        assert_eq!(lease.tx.total_changes() - changes, 1, "实际 INSERT");
        assert!(lease.material.frames.contains_key(&a));
        assert!(
            !lease.material.frames.contains_key(&b),
            "禁止用 put 入参 seed"
        );
        lease.checked_read(&a).unwrap();
        assert_eq!(sql_reads_on_thread() - sql, 1);
        assert_eq!(s_content::frame_calls_on_thread() - f, 1);
        lease.checked_read(&b).unwrap();
        assert_eq!(sql_reads_on_thread() - sql, 2, "B 首读仍 cold");
        assert_eq!(s_content::frame_calls_on_thread() - f, 2);
        assert_eq!(lease.material.retained_capacity, 0);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_effects_cover_main_temp_indirect_delete_replace_and_ignore() {
        // pure SELECT / AFTER IGNORE 不改其它 digest，可复用；真正附加写即全清。
        for (trigger, retained, old_present, old_valid, target_present) in [
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN SELECT 1; END;", true, true, true, true),
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01' WHERE digest<>NEW.digest; END;", false, true, false, true),
            ("CREATE TEMP TRIGGER effect AFTER INSERT ON main.s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01' WHERE digest<>NEW.digest; END;", false, true, false, true),
            ("CREATE TABLE side(x); CREATE TRIGGER hop AFTER INSERT ON side BEGIN UPDATE s_content_nodes SET body=x'01'; END; CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN INSERT INTO side VALUES(1); END;", false, true, false, true),
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN DELETE FROM s_content_nodes WHERE digest<>NEW.digest; END;", false, false, false, true),
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN INSERT OR REPLACE INTO s_content_nodes SELECT digest,tag,x'01' FROM s_content_nodes WHERE digest<>NEW.digest; END;", false, true, false, true),
            ("CREATE TRIGGER effect BEFORE INSERT ON s_content_nodes BEGIN SELECT RAISE(IGNORE); END;", false, true, true, false),
            ("CREATE TRIGGER effect BEFORE INSERT ON s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01'; SELECT RAISE(IGNORE); END;", false, true, false, false),
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN SELECT RAISE(IGNORE); END;", true, true, true, true),
            ("CREATE TRIGGER effect AFTER INSERT ON s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01' WHERE digest<>NEW.digest; SELECT RAISE(IGNORE); END;", false, true, false, true),
        ] {
            let mut conn = database("s-session/3");
            let mut tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
            let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
            TxStore::new(&tx).unwrap().put(a, s_content::tag::BOOL_TRUE, &[]).unwrap();
            tx.execute_batch(trigger).unwrap();
            let mut lease = NativeReadLease::new(&mut tx).unwrap();
            assert!(lease.insert_can_preserve, "trigger 本身不预先排除：{trigger}");
            lease.checked_read(&a).unwrap();
            lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
            assert_eq!(lease.material.frames.contains_key(&a), retained, "{trigger}");
            assert!(!lease.material.frames.contains_key(&b));
            assert!(!lease.poisoned);
            let target: bool = lease.tx.query_row("SELECT EXISTS(SELECT 1 FROM s_content_nodes WHERE digest=?1)", params![&b[..]], |r| r.get(0)).unwrap();
            assert_eq!(target, target_present, "{trigger}");
            let result = lease.checked_read(&a);
            if old_valid {
                assert!(result.unwrap().is_some(), "{trigger}");
            } else if old_present {
                assert_eq!(result.unwrap_err().code(), code::NODE_DIGEST, "{trigger}");
                assert!(lease.poisoned && lease.material.frames.is_empty());
            } else {
                assert!(result.unwrap().is_none(), "{trigger}");
            }
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_hidden_loaded_trigger_uses_actual_effects() {
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        tx.execute_batch("CREATE TRIGGER hidden_effect AFTER INSERT ON s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01' WHERE digest<>NEW.digest; END;").unwrap();
        // 加载 INSERT 的 trigger 程序，再删除 raw schema 记录而不 bump schema_version。
        tx.prepare("INSERT INTO s_content_nodes VALUES(?1,?2,?3)")
            .unwrap()
            .finalize()
            .unwrap();
        tx.execute_batch("PRAGMA writable_schema=ON; DELETE FROM sqlite_schema WHERE type='trigger' AND name='hidden_effect'; PRAGMA writable_schema=OFF;").unwrap();
        assert_eq!(
            tx.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type='trigger'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        assert!(lease.insert_can_preserve);
        lease.checked_read(&a).unwrap();
        let changes = lease.tx.total_changes();
        lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
        assert_eq!(
            lease.tx.total_changes() - changes,
            2,
            "隐藏已加载 trigger 确实执行"
        );
        assert!(lease.material.frames.is_empty());
        assert_eq!(
            lease.checked_read(&a).unwrap_err().code(),
            code::NODE_DIGEST
        );
        assert!(lease.poisoned);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_loaded_shape_and_temp_shadow_reject_preservation() {
        for schema in [
            "CREATE UNIQUE INDEX extra ON s_content_nodes(tag);",
            "CREATE INDEX extra ON s_content_nodes(body);",
            "CREATE INDEX extra ON s_content_nodes((tag+1)) WHERE tag>0;",
            "CREATE UNIQUE INDEX extra ON s_content_nodes(tag); PRAGMA main.index_list('s_content_nodes'); PRAGMA writable_schema=ON; DELETE FROM sqlite_schema WHERE name='extra'; PRAGMA writable_schema=OFF;",
            "CREATE TEMP TABLE S_CONTENT_NODES(digest BLOB PRIMARY KEY,tag INTEGER,body BLOB) WITHOUT ROWID;",
            "CREATE TEMP VIEW S_CONTENT_NODES AS SELECT digest,tag,body FROM main.s_content_nodes;",
            "CREATE TEMP TABLE S_CONTENT_META(singleton INTEGER,content_format TEXT); INSERT INTO S_CONTENT_META VALUES(1,'s-content/1');",
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL);",
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL, extra INTEGER GENERATED ALWAYS AS (tag+1) VIRTUAL) WITHOUT ROWID;",
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL, UNIQUE(tag) ON CONFLICT REPLACE) WITHOUT ROWID;",
            "CREATE TABLE parent(tag INTEGER PRIMARY KEY); DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL REFERENCES parent(tag),body BLOB NOT NULL) WITHOUT ROWID;",
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB COLLATE NOCASE PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL) WITHOUT ROWID;",
            "DROP TABLE s_content_nodes; CREATE VIRTUAL TABLE s_content_nodes USING fts5(digest,tag,body);",
            "DROP TABLE s_content_nodes;",
        ] {
            let mut conn = database("s-session/3");
            let mut tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            tx.execute_batch(schema).unwrap();
            let lease = NativeReadLease::new(&mut tx).unwrap();
            assert!(!lease.insert_can_preserve, "{schema}");
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_unqualified_unique_replace_keeps_old_full_clear() {
        for schema in [
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL UNIQUE ON CONFLICT REPLACE,body BLOB NOT NULL) WITHOUT ROWID;",
            "CREATE TEMP TABLE S_CONTENT_NODES(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL UNIQUE ON CONFLICT REPLACE,body BLOB NOT NULL) WITHOUT ROWID;",
            "DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL UNIQUE ON CONFLICT REPLACE,body BLOB NOT NULL) WITHOUT ROWID; PRAGMA main.index_list('s_content_nodes'); PRAGMA writable_schema=ON; UPDATE sqlite_schema SET sql='CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL) WITHOUT ROWID' WHERE name='s_content_nodes'; PRAGMA writable_schema=OFF;",
        ] {
            let mut conn = database("s-session/3");
            let mut tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            tx.execute_batch(schema).unwrap();
            let a = s_content::frame(s_content::tag::BYTES_CHUNK, &[1]);
            let b = s_content::frame(s_content::tag::BYTES_CHUNK, &[2]);
            TxStore::new(&tx).unwrap().put(a, s_content::tag::BYTES_CHUNK, &[1]).unwrap();
            let mut lease = NativeReadLease::new(&mut tx).unwrap();
            assert!(!lease.insert_can_preserve, "{schema}");
            lease.checked_read(&a).unwrap();
            let changes = lease.tx.total_changes();
            lease.put(b, s_content::tag::BYTES_CHUNK, &[2]).unwrap();
            assert_eq!(lease.tx.total_changes() - changes, 1, "另一个 digest 的隐式删除不计数：{schema}");
            assert!(lease.material.frames.is_empty());
            assert!(lease.checked_read(&a).unwrap().is_none(), "{schema}");
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_trigger_replace_delete_ignore_still_counts_inner_insert() {
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        tx.execute_batch("PRAGMA recursive_triggers=ON;").unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        tx.execute_batch("CREATE TRIGGER stop_delete AFTER DELETE ON s_content_nodes BEGIN SELECT RAISE(IGNORE); END; CREATE TRIGGER effect AFTER INSERT ON s_content_nodes WHEN NEW.tag<>OLD_TAG_PLACEHOLDER BEGIN INSERT OR REPLACE INTO s_content_nodes SELECT digest,tag,x'01' FROM s_content_nodes WHERE digest<>NEW.digest; END;".replace("OLD_TAG_PLACEHOLDER", &s_content::tag::BOOL_TRUE.to_string()).as_str()).unwrap();
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        assert!(lease.insert_can_preserve);
        lease.checked_read(&a).unwrap();
        let before = lease.tx.total_changes();
        lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
        assert_eq!(lease.tx.total_changes() - before, 2);
        assert!(lease.material.frames.is_empty());
        assert_eq!(
            lease.checked_read(&a).unwrap_err().code(),
            code::NODE_DIGEST
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_actual_constraint_and_insert_deny_errors_poison_unchanged() {
        use rusqlite::hooks::{AuthAction, Authorization};
        for deny in [false, true] {
            let mut conn = database("s-session/3");
            let mut tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            if !deny {
                tx.execute_batch("DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL CHECK(body<>x'02')) WITHOUT ROWID;").unwrap();
            }
            let a = s_content::frame(s_content::tag::BYTES_CHUNK, &[1]);
            let b = s_content::frame(s_content::tag::BYTES_CHUNK, &[2]);
            TxStore::new(&tx)
                .unwrap()
                .put(a, s_content::tag::BYTES_CHUNK, &[1])
                .unwrap();
            let mut lease = NativeReadLease::new(&mut tx).unwrap();
            assert!(lease.insert_can_preserve);
            lease.checked_read(&a).unwrap();
            if deny {
                lease
                    .tx
                    .authorizer(Some(|ctx: rusqlite::hooks::AuthContext<'_>| {
                        if matches!(
                            ctx.action,
                            AuthAction::Insert {
                                table_name: "s_content_nodes"
                            }
                        ) {
                            Authorization::Deny
                        } else {
                            Authorization::Allow
                        }
                    }))
                    .unwrap();
            }
            // 普通 TxStore 的同一个真正 INSERT 失败先取原错误，租约必须逐字传播。
            let original = TxStore::new(lease.tx)
                .unwrap()
                .put(b, s_content::tag::BYTES_CHUNK, &[2])
                .unwrap_err();
            let sql = sql_reads_on_thread();
            let f = s_content::frame_calls_on_thread();
            let before = lease.tx.total_changes();
            let error = lease.put(b, s_content::tag::BYTES_CHUNK, &[2]).unwrap_err();
            assert_eq!(error.to_string(), original.to_string());
            assert_eq!(sql_reads_on_thread() - sql, 1);
            assert_eq!(s_content::frame_calls_on_thread() - f, 1);
            assert_eq!(lease.tx.total_changes(), before);
            assert!(lease.poisoned && lease.material.frames.is_empty());
            assert!(lease.checked_read(&a).is_err());
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_ignored_pragma_metadata_is_error_including_temp_replace_shadow() {
        use rusqlite::hooks::{AuthAction, Authorization};
        for ignored in [
            "read_uncommitted",
            "table_list",
            "table_xinfo",
            "foreign_key_list",
            "index_list",
            "index_xinfo",
        ] {
            let mut conn = database("s-session/3");
            let mut tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            tx.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
                if matches!(ctx.action, AuthAction::Pragma { pragma_name, .. } if pragma_name.eq_ignore_ascii_case(ignored)) {
                    Authorization::Ignore
                } else { Authorization::Allow }
            })).unwrap();
            let error = NativeReadLease::new(&mut tx).err().unwrap();
            assert_eq!(error.code(), code::SOURCE, "{ignored}");
        }
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        tx.execute_batch("CREATE TEMP TABLE S_CONTENT_NODES(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL UNIQUE ON CONFLICT REPLACE,body BLOB NOT NULL) WITHOUT ROWID;").unwrap();
        // 仅忽略 temp table_list：main 的正常结构绝不能替 TEMP 背书。
        tx.authorizer(Some(|ctx: rusqlite::hooks::AuthContext<'_>| {
            if ctx.database_name == Some("temp")
                && matches!(
                    ctx.action,
                    AuthAction::Pragma {
                        pragma_name: "table_list",
                        ..
                    }
                )
            {
                Authorization::Ignore
            } else {
                Authorization::Allow
            }
        }))
        .unwrap();
        assert_eq!(
            NativeReadLease::new(&mut tx).err().unwrap().code(),
            code::SOURCE
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_pk_replace_misread_missing_removes_only_target_proof() {
        use rusqlite::hooks::{AuthAction, Authorization};
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        tx.execute_batch("DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY ON CONFLICT REPLACE, tag INTEGER NOT NULL, body BLOB NOT NULL) WITHOUT ROWID;").unwrap();
        let a = s_content::frame(s_content::tag::BYTES_CHUNK, &[1, 2]);
        let b = s_content::frame(s_content::tag::BYTES_CHUNK, &[3]);
        let mut store = TxStore::new(&tx).unwrap();
        store.put(a, s_content::tag::BYTES_CHUNK, &[1, 2]).unwrap();
        store.put(b, s_content::tag::BYTES_CHUNK, &[3]).unwrap();
        drop(store);
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        assert!(lease.insert_can_preserve);
        lease.checked_read(&a).unwrap();
        lease.checked_read(&b).unwrap();
        assert_eq!(lease.material.retained_capacity, 3);
        lease
            .tx
            .authorizer(Some(|ctx: rusqlite::hooks::AuthContext<'_>| {
                if matches!(
                    ctx.action,
                    AuthAction::Read {
                        table_name: "s_content_nodes",
                        column_name: "digest"
                    }
                ) {
                    Authorization::Ignore
                } else {
                    Authorization::Allow
                }
            }))
            .unwrap();
        let changes = lease.tx.total_changes();
        lease.put(b, s_content::tag::BYTES_CHUNK, &[3]).unwrap();
        assert_eq!(
            lease.tx.total_changes() - changes,
            1,
            "REPLACE 删除不入计数"
        );
        assert!(lease.material.frames.contains_key(&a));
        assert!(!lease.material.frames.contains_key(&b));
        assert_eq!(lease.material.retained_capacity, 2);
        lease
            .tx
            .authorizer(None::<fn(rusqlite::hooks::AuthContext<'_>) -> Authorization>)
            .unwrap();
        let sql = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        lease.checked_read(&b).unwrap();
        assert_eq!(sql_reads_on_thread() - sql, 1);
        assert_eq!(s_content::frame_calls_on_thread() - f, 1);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_insert_counter_witness_rejects_every_unproven_effect() {
        for (before, after, direct, expected) in [
            (7, 8, 1, true),
            (7, 7, 1, false),
            (7, 9, 1, false),
            (7, 8, 0, false),
            (7, 8, 2, false),
            (8, 7, 1, false),
            (u64::MAX, 0, 1, false),
        ] {
            assert_eq!(native_insert_only_target(before, after, direct), expected);
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_read_lease_put_trigger_errors_and_poison_do_not_repair() {
        let mut conn = database("s-session/3");
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        tx.execute_batch(&format!("CREATE TRIGGER lease_corrupt AFTER INSERT ON s_content_nodes BEGIN UPDATE s_content_nodes SET body=x'01' WHERE digest=x'{}'; END;",&s_content::root_string(&a)[4..])).unwrap();
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        lease.checked_read(&a).unwrap();
        assert!(!lease.material.frames.is_empty());
        lease.put(b, s_content::tag::BOOL_FALSE, &[]).unwrap();
        assert!(lease.material.frames.is_empty());
        assert_eq!(
            lease.checked_read(&a).unwrap_err().code(),
            code::NODE_DIGEST
        );
        assert!(lease.poisoned && lease.material.frames.is_empty());
        let before = sql_reads_on_thread();
        assert!(lease.checked_read(&b).is_err());
        assert!(lease.put(b, s_content::tag::BOOL_FALSE, &[]).is_err());
        assert_eq!(sql_reads_on_thread(), before);
        drop(lease);
        tx.execute_batch("DROP TRIGGER lease_corrupt;CREATE TRIGGER lease_deny BEFORE INSERT ON s_content_nodes BEGIN SELECT RAISE(ABORT,'actual-insert-denied');END;").unwrap();
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        lease.checked_read(&b).unwrap();
        let c = s_content::frame(s_content::tag::NULL, &[]);
        assert!(lease
            .put(c, s_content::tag::NULL, &[])
            .unwrap_err()
            .to_string()
            .contains("actual-insert-denied"));
        assert!(lease.poisoned && lease.material.frames.is_empty());
        drop(lease);
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        lease.checked_read(&b).unwrap();
        let before = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        assert!(lease.put(b, s_content::tag::BOOL_TRUE, &[]).is_err());
        assert_eq!(sql_reads_on_thread(), before, "摘要不符先于 SQL 拒绝");
        assert_eq!(s_content::frame_calls_on_thread() - f, 1);
        assert!(lease.poisoned && lease.material.frames.is_empty());
        drop(lease);
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        lease.checked_read(&b).unwrap();
        let before = sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        // A 被真实 INSERT 触发器改坏；提交合法原件也须冷读旧行并拒绝，不能修复。
        assert_eq!(
            lease
                .put(a, s_content::tag::BOOL_TRUE, &[])
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        assert_eq!(sql_reads_on_thread() - before, 1);
        assert_eq!(s_content::frame_calls_on_thread() - f, 2);
        assert!(lease.poisoned && lease.material.frames.is_empty());
        let before = sql_reads_on_thread();
        assert!(lease.checked_read(&b).is_err());
        assert!(lease.put(b, s_content::tag::BOOL_FALSE, &[]).is_err());
        assert_eq!(sql_reads_on_thread(), before);
        drop(lease);
        assert_eq!(
            tx.query_row(
                "SELECT body FROM s_content_nodes WHERE digest=?1",
                params![&a[..]],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .unwrap(),
            vec![1],
            "拒绝同摘要坏原件后旧节点原样保留"
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_read_lease_isolation_activity_and_actual_interrupt() {
        let mut conn = database("s-session/3");
        conn.execute_batch("PRAGMA read_uncommitted=1").unwrap();
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        assert!(NativeReadLease::new(&mut tx)
            .err()
            .unwrap()
            .to_string()
            .contains("read_uncommitted=0"));
        assert_eq!(
            tx.query_row("PRAGMA read_uncommitted", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1,
            "constructor must not rewrite isolation"
        );
        tx.rollback().unwrap();
        conn.execute_batch("PRAGMA read_uncommitted=0").unwrap();
        let mut tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        TxStore::new(&tx)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        for put in [false, true] {
            let mut lease = NativeReadLease::new(&mut tx).unwrap();
            lease.checked_read(&a).unwrap();
            // Test-only private access installs a failing hook; the production lease exposes none.
            lease.tx.progress_handler(1, Some(|| true)).unwrap();
            let before = sql_reads_on_thread();
            let f = s_content::frame_calls_on_thread();
            let error = if put {
                lease.put(a, s_content::tag::BOOL_TRUE, &[]).unwrap_err()
            } else {
                lease.checked_read(&[0; 32]).unwrap_err()
            };
            assert_eq!(error.code(), code::SOURCE);
            assert_eq!(sql_reads_on_thread() - before, 1);
            assert_eq!(s_content::frame_calls_on_thread() - f, u64::from(put));
            assert!(lease.poisoned && lease.material.frames.is_empty());
            lease.tx.progress_handler(0, None::<fn() -> bool>).unwrap();
            let before = sql_reads_on_thread();
            assert!(lease.checked_read(&a).is_err());
            assert!(lease.put(a, s_content::tag::BOOL_TRUE, &[]).is_err());
            assert_eq!(sql_reads_on_thread(), before);
        }
        fn deny(_: rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization {
            rusqlite::hooks::Authorization::Deny
        }
        for put in [false, true] {
            let mut lease = NativeReadLease::new(&mut tx).unwrap();
            lease.checked_read(&a).unwrap();
            lease.tx.authorizer(Some(deny)).unwrap();
            let before = sql_reads_on_thread();
            let f = s_content::frame_calls_on_thread();
            let error = if put {
                lease.put(a, s_content::tag::BOOL_TRUE, &[]).unwrap_err()
            } else {
                lease.checked_read(&[0; 32]).unwrap_err()
            };
            assert_eq!(error.code(), code::SOURCE);
            assert_eq!(sql_reads_on_thread() - before, 1);
            assert_eq!(s_content::frame_calls_on_thread() - f, u64::from(put));
            assert!(lease.poisoned && lease.material.frames.is_empty());
            lease
                .tx
                .authorizer(
                    None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>,
                )
                .unwrap();
            let before = sql_reads_on_thread();
            assert!(lease.checked_read(&a).is_err());
            assert!(lease.put(a, s_content::tag::BOOL_TRUE, &[]).is_err());
            assert_eq!(sql_reads_on_thread(), before);
        }
        tx.execute_batch("ROLLBACK").unwrap();
        assert!(NativeReadLease::new(&mut tx)
            .err()
            .unwrap()
            .to_string()
            .contains("活动事务"));
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_read_lease_wal_snapshot_and_upgrade_failure() {
        let path = std::env::temp_dir().join(format!(
            "s1394-lease-wal-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;PRAGMA read_uncommitted=0;")
            .unwrap();
        conn.execute_batch(super::super::v2::SCHEMA_V2).unwrap();
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1,'s-session/3','1','test-clock','{}',1,'accept')",
            [],
        )
        .unwrap();
        let setup = conn.transaction().unwrap();
        init_schema(&setup).unwrap();
        let a = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        TxStore::new(&setup)
            .unwrap()
            .put(a, s_content::tag::BOOL_TRUE, &[])
            .unwrap();
        setup.commit().unwrap();
        let mut tx = conn.transaction().unwrap();
        let mut lease = NativeReadLease::new(&mut tx).unwrap();
        lease.checked_read(&a).unwrap();
        let external = Connection::open(&path).unwrap();
        external
            .execute(
                "DELETE FROM s_content_nodes WHERE digest=?1",
                params![&a[..]],
            )
            .unwrap();
        assert!(lease.checked_read(&a).unwrap().is_some());
        // Force a real SQL read of the same snapshot, not only the cached result.
        lease.clear();
        assert!(lease.checked_read(&a).unwrap().is_some());
        let b = s_content::frame(s_content::tag::BOOL_FALSE, &[]);
        assert!(lease.put(b, s_content::tag::BOOL_FALSE, &[]).is_err());
        assert!(lease.poisoned && lease.material.frames.is_empty());
        drop(lease);
        tx.rollback().unwrap();
        let tx = conn.transaction().unwrap();
        assert!(TxStore::new(&tx)
            .unwrap()
            .checked_read(&a)
            .unwrap()
            .is_none());
        tx.rollback().unwrap();
        drop(external);
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[test]
    fn content_store_roundtrip_uses_same_transaction_and_survives_commit() {
        let mut conn = database("s-session/3");
        let built = {
            let tx = conn.transaction().unwrap();
            init_schema(&tx).unwrap();
            let mut store = TxStore::new(&tx).unwrap();
            let built = s_content::write_value(
                &mut store,
                &s_content::Value::Array(vec![
                    s_content::Value::String("完整证据".into()),
                    s_content::Value::Number(s_content::NumberToken::parse("1.0").unwrap()),
                ]),
            )
            .unwrap();
            s_content::verify(&mut store, &built.content_ref()).unwrap();
            tx.commit().unwrap();
            built
        };
        let tx = conn.transaction().unwrap();
        let mut reader = ReadStore::new(&tx).unwrap();
        let mut rendered = Vec::new();
        s_content::render_canonical(&mut reader, &built.content_ref(), &mut rendered).unwrap();
        assert_eq!(rendered, "[\"完整证据\",1.0]".as_bytes());
    }

    #[test]
    fn content_store_rollback_does_not_leave_an_imported_node() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        tx.commit().unwrap();
        let digest = {
            let tx = conn.transaction().unwrap();
            let mut store = TxStore::new(&tx).unwrap();
            let built = s_content::write_value(&mut store, &s_content::Value::Bool(true)).unwrap();
            assert!(matches!(
                store.read(&built.digest).unwrap(),
                NodeRead::Found { .. }
            ));
            tx.rollback().unwrap();
            built.digest
        };
        let tx = conn.transaction().unwrap();
        assert!(matches!(
            ReadStore::new(&tx).unwrap().read(&digest).unwrap(),
            NodeRead::Missing
        ));
    }

    #[test]
    fn content_domain_and_format_borrow_text_keep_missing_null_type_and_order_rejections() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER,protocol_revision); CREATE TABLE s_content_meta(singleton INTEGER,content_format);").unwrap();
        let missing = ensure_format(&conn).unwrap_err();
        assert_eq!(missing.code(), code::FORMAT);
        assert!(missing.detail().contains("s-session/3"));
        conn.execute("INSERT INTO s_protocol_meta VALUES(1,'s-session/3')", [])
            .unwrap();
        let missing = ensure_format(&conn).unwrap_err();
        assert_eq!(missing.code(), code::FORMAT);
        assert!(missing.detail().contains("格式登记"));
        conn.execute(
            "INSERT INTO s_content_meta VALUES(1,?1)",
            params![s_content::CONTENT_FORMAT],
        )
        .unwrap();
        ensure_format(&conn).unwrap();
        for value in ["NULL", "7", "x'00'", "CAST(x'ff' AS TEXT)"] {
            conn.execute_batch(&format!(
                "UPDATE s_protocol_meta SET protocol_revision={value}"
            ))
            .unwrap();
            let old = conn
                .query_row(
                    "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_err();
            let rejected = ensure_format(&conn).unwrap_err();
            assert_eq!(rejected.to_string(), storage_error(old).to_string());
            assert_eq!(rejected.code(), code::SOURCE);
        }
        conn.execute(
            "UPDATE s_protocol_meta SET protocol_revision='s-session/3'",
            [],
        )
        .unwrap();
        for value in ["NULL", "7", "x'00'", "CAST(x'ff' AS TEXT)"] {
            conn.execute_batch(&format!("UPDATE s_content_meta SET content_format={value}"))
                .unwrap();
            let old = conn
                .query_row(
                    "SELECT content_format FROM s_content_meta WHERE singleton=1",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_err();
            let rejected = ensure_format(&conn).unwrap_err();
            assert_eq!(rejected.to_string(), storage_error(old).to_string());
            assert_eq!(rejected.code(), code::SOURCE);
        }
        conn.execute(
            "UPDATE s_content_meta SET content_format=?1",
            params![s_content::CONTENT_FORMAT],
        )
        .unwrap();
        ensure_format(&conn).unwrap();
        conn.execute(
            "UPDATE s_protocol_meta SET protocol_revision='s-session/2'",
            [],
        )
        .unwrap();
        let wrong = ensure_format(&conn).unwrap_err();
        assert_eq!(wrong.code(), code::FORMAT);
        assert!(wrong.detail().contains("s-session/3"));
    }

    #[test]
    fn content_store_refuses_legacy_domain_and_does_not_create_tables() {
        let mut conn = database("s-session/2");
        let tx = conn.transaction().unwrap();
        assert_eq!(init_schema(&tx).unwrap_err().code(), code::FORMAT);
        let count: i64 = tx
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name LIKE 's_content_%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        tx.rollback().unwrap();
        let tx = conn.transaction().unwrap();
        assert!(ReadStore::new(&tx).is_err());
    }

    #[test]
    fn content_store_rejects_forged_write_and_corrupt_existing_bytes() {
        let mut conn = database("s-session/3");
        let tx = conn.transaction().unwrap();
        init_schema(&tx).unwrap();
        let mut store = TxStore::new(&tx).unwrap();
        let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
        assert_eq!(
            store
                .put(digest, s_content::tag::BOOL_FALSE, &[])
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        assert!(matches!(store.read(&digest).unwrap(), NodeRead::Missing));
        store.put(digest, s_content::tag::BOOL_TRUE, &[]).unwrap();
        store.put(digest, s_content::tag::BOOL_TRUE, &[]).unwrap();
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert_eq!(store.read(&digest).unwrap_err().code(), code::NODE_DIGEST);
        assert_eq!(
            store
                .put(digest, s_content::tag::BOOL_TRUE, &[])
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        let body: Vec<u8> = tx
            .query_row(
                "SELECT body FROM s_content_nodes WHERE digest=?1",
                params![&digest[..]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(body, [1], "不得覆盖坏原件假装修复成功");
    }
}

/// #1394：显式 Native TestOnly 当前事务捕获；普通容器，不附资源额度或生产资格。
#[cfg(any(test, feature = "s_capture_native_testonly"))]
pub(crate) fn capture_for_prototype(
    tx: &Transaction<'_>,
) -> Result<s_content::AuditCapture, CodecError> {
    capture_for_request_prototype(tx).map(|(capture, _)| capture)
}

/// #1394 Native：digest只复制到栈上32字节；非BLOB仍调用原Vec解码取得原类型错误。
/// 不提前读取tag/body，摘要长度仍先于标签域与正文错误。
#[cfg(any(test, feature = "s_capture_native_testonly"))]
fn request_capture_digest(row: &rusqlite::Row<'_>) -> Result<NodeDigest, CodecError> {
    #[cfg(feature = "s_capture_native_testonly")]
    {
        match row.get_ref(0).map_err(storage_error)? {
            rusqlite::types::ValueRef::Blob(bytes) => {
                let digest = bytes
                    .try_into()
                    .map_err(|_| storage_error("捕获摘要长度"))?;
                #[cfg(test)]
                PROTOTYPE_CAPTURE_WORK.with(|work| {
                    work.borrow_mut().digest_blob_borrows += 1;
                });
                Ok(digest)
            }
            _ => {
                // 保持rusqlite的列索引、列名、storage class和原InvalidColumnType。
                #[cfg(test)]
                PROTOTYPE_CAPTURE_WORK.with(|work| {
                    work.borrow_mut().digest_vec_decode_calls += 1;
                });
                let digest: Vec<u8> = row.get(0).map_err(storage_error)?;
                digest.try_into().map_err(|_| storage_error("捕获摘要长度"))
            }
        }
    }
    #[cfg(not(feature = "s_capture_native_testonly"))]
    {
        #[cfg(test)]
        PROTOTYPE_CAPTURE_WORK.with(|work| {
            work.borrow_mut().digest_vec_decode_calls += 1;
        });
        let digest: Vec<u8> = row.get(0).map_err(storage_error)?;
        digest.try_into().map_err(|_| storage_error("捕获摘要长度"))
    }
}

/// 请求独占的去重容器；容量不是额度，也不是可跨请求复用的审计结论。
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Default)]
pub(crate) struct OriginalScanScratch {
    seen: std::collections::HashSet<NodeDigest>,
    #[cfg(test)]
    capacity_growths: u64,
}
#[cfg(feature = "s_capture_native_testonly")]
impl OriginalScanScratch {
    pub(crate) fn capacity_nodes(&self) -> usize {
        self.seen.capacity()
    }
    #[cfg(test)]
    pub(crate) fn capacity_growths(&self) -> u64 {
        self.capacity_growths
    }
    fn begin(&mut self) {
        // 即使上次扫描在读错处返回，下一次也从空集合重读全部当前SQL行。
        self.seen.clear();
    }
    fn insert_admitted(
        &mut self,
        digest: NodeDigest,
        work: Option<&WorkScope>,
    ) -> Result<bool, CodecError> {
        let Some(work) = work else {
            return Ok(self.insert(digest));
        };
        if self.seen.contains(&digest) {
            return Ok(false);
        }
        #[cfg(test)]
        let before = self.seen.capacity();
        let bytes = NativeAdmission(Some(work))
            .native_hash_bytes::<NodeDigest, ()>(self.seen.len(), self.seen.capacity())?;
        if bytes > 0 {
            let permit = work.reserve_native(bytes)?;
            self.seen.try_reserve(1).map_err(|_| {
                work.reject(s_content::capture_allocation::scratch::ScratchFailure::Allocation)
            })?;
            permit.retain();
        }
        let first = self.seen.insert(digest);
        #[cfg(test)]
        if self.seen.capacity() > before {
            self.capacity_growths += 1;
        }
        Ok(first)
    }
    fn insert(&mut self, digest: NodeDigest) -> bool {
        #[cfg(test)]
        let before = self.seen.capacity();
        let first = self.seen.insert(digest);
        #[cfg(test)]
        if self.seen.capacity() > before {
            self.capacity_growths += 1;
        }
        first
    }
}

/// Native cold entry: checked borrowed SQL bodies are copied only after parent admission.
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn capture_for_request_prototype_admitted(
    tx: &Transaction<'_>,
    work: Option<&WorkScope>,
) -> Result<(s_content::AuditCapture, usize), CodecError> {
    if work.is_none() {
        return capture_for_request_prototype(tx);
    }
    capture_originals_admitted(tx, None, work).map(|(capture, count, _)| (capture, count))
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn capture_with_original_subset_for_source_cache_admitted(
    tx: &Transaction<'_>,
    prior: Option<(&s_content::AuditCapture, usize)>,
    work: Option<&WorkScope>,
) -> Result<(s_content::AuditCapture, usize, bool), CodecError> {
    if work.is_none() {
        return capture_with_original_subset_for_source_cache(tx, prior);
    }
    capture_originals_admitted(tx, prior, work)
}
#[cfg(feature = "s_capture_native_testonly")]
fn scan_request_nodes_checked(
    tx: &Transaction<'_>,
    mut visit: impl FnMut(NodeDigest, u8, &[u8]) -> Result<(), CodecError>,
) -> Result<(), CodecError> {
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        tx,
        c"SELECT digest,tag,body FROM s_content_nodes",
    )
    .map_err(storage_error)?;
    while let Some(mut row) = stmt.next().map_err(storage_error)? {
        let digest: NodeDigest = row
            .blob(0)
            .map_err(storage_error)?
            .try_into()
            .map_err(|_| storage_error("捕获摘要长度"))?;
        #[cfg(test)]
        PROTOTYPE_CAPTURE_WORK.with(|work| work.borrow_mut().digest_blob_borrows += 1);
        let tag = u8::try_from(row.integer(1).map_err(storage_error)?)
            .map_err(|_| storage_error("捕获标签域"))?;
        let body = row.blob(2).map_err(storage_error)?;
        #[cfg(test)]
        record_prototype_row(body.len());
        visit(digest, tag, body)?;
    }
    stmt.finalize().map_err(storage_error)
}
#[cfg(feature = "s_capture_native_testonly")]
fn capture_originals_admitted(
    tx: &Transaction<'_>,
    prior: Option<(&s_content::AuditCapture, usize)>,
    work: Option<&WorkScope>,
) -> Result<(s_content::AuditCapture, usize, bool), CodecError> {
    ensure_format(tx)?;
    #[cfg(test)]
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.capture_builds += 1;
        work.node_scans += 1;
        work.rebind_scans += u64::from(prior.is_some());
    });
    // Locals drop in reverse order: nodes first, whole-tree permit afterwards.
    let mut tree_permit = None;
    let mut nodes = s_content::MemoryNodes::new();
    let mut scratch = OriginalScanScratch::default();
    let mut matched = 0usize;
    let mut same = prior.is_some();
    scan_request_nodes_checked(tx, |digest, tag, bytes| {
        let unique = scratch.insert_admitted(digest, work)?;
        same &= unique;
        if let Some((capture, _)) = prior {
            if let Some((old_tag, old_body)) = capture.original(&digest) {
                matched += usize::from(unique);
                same &= old_tag == tag && old_body == bytes;
            }
        }
        let next = nodes
            .len()
            .checked_add(usize::from(!nodes.contains(&digest)))
            .ok_or_else(|| NativeAdmission(work).native_overflow())?;
        let next_permit = native_tree_storage::<NodeDigest, (u8, Vec<u8>)>(work, next)?;
        let body = native_body_copy(work, bytes)?;
        nodes.insert_unchecked(digest, tag, body);
        tree_permit = next_permit;
        Ok(())
    })?;
    same &= prior.is_some_and(|(_, count)| matched == count);
    let count = nodes.len();
    let capture = s_content::AuditCapture::new(nodes);
    // Return owns the tree; conservative retention until the caller's entire parent domain ends.
    if let Some(permit) = tree_permit {
        permit.retain();
    }
    Ok((capture, count, same))
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn current_originals_equal_for_request_with_scratch_admitted(
    tx: &Transaction<'_>,
    capture: &s_content::AuditCapture,
    expected_nodes: usize,
    scratch: &mut OriginalScanScratch,
    work: Option<&WorkScope>,
) -> Result<bool, CodecError> {
    if work.is_none() {
        return current_originals_equal_for_request_with_scratch(
            tx,
            capture,
            expected_nodes,
            scratch,
        );
    }
    scratch.begin();
    ensure_format(tx)?;
    #[cfg(test)]
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.node_scans += 1;
        work.rebind_scans += 1;
    });
    let mut equal = true;
    scan_request_nodes_checked(tx, |digest, tag, body| {
        let first = scratch.insert_admitted(digest, work)?;
        equal &= first
            && capture
                .original(&digest)
                .is_some_and(|(old_tag, old)| old_tag == tag && old == body);
        Ok(())
    })?;
    Ok(equal && scratch.seen.len() == expected_nodes)
}

/// 计数取自刚构建的完整原件集合，只供私有Native请求slot核对覆盖；不增加capture字段。
#[cfg(any(test, feature = "s_capture_native_testonly"))]
pub(crate) fn capture_for_request_prototype(
    tx: &Transaction<'_>,
) -> Result<(s_content::AuditCapture, usize), CodecError> {
    ensure_format(tx)?;
    #[cfg(test)]
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.capture_builds += 1;
        work.node_scans += 1;
    });
    let mut nodes = s_content::MemoryNodes::new();
    let mut stmt = tx
        .prepare("SELECT digest,tag,body FROM s_content_nodes")
        .map_err(storage_error)?;
    let mut rows = stmt.query([]).map_err(storage_error)?;
    while let Some(r) = rows.next().map_err(storage_error)? {
        let d = request_capture_digest(r)?;
        let t: i64 = r.get(1).map_err(storage_error)?;
        let t: u8 = t.try_into().map_err(|_| storage_error("捕获标签域"))?;
        let body: Vec<u8> = r.get(2).map_err(storage_error)?;
        #[cfg(test)]
        record_prototype_row(body.len());
        nodes.insert_unchecked(d, t, body);
    }
    let count = nodes.len();
    Ok((s_content::AuditCapture::new(nodes), count))
}

/// 原SQL全扫描、原解码次序、原始digest/tag/body逐字节绑定；不创建第二份body owner。
/// 普通材料变化仍读到末尾，不能吞掉后行的SQL/type错误；只有完整扫描成功才能失去复用资格。
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn current_originals_equal_for_request(
    tx: &Transaction<'_>,
    capture: &s_content::AuditCapture,
    expected_nodes: usize,
) -> Result<bool, CodecError> {
    // 既有测试/独立调用者保留兼容入口；请求主路径使用独占scratch入口。
    let mut scratch = OriginalScanScratch::default();
    current_originals_equal_for_request_with_scratch(tx, capture, expected_nodes, &mut scratch)
}

#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn current_originals_equal_for_request_with_scratch(
    tx: &Transaction<'_>,
    capture: &s_content::AuditCapture,
    expected_nodes: usize,
    scratch: &mut OriginalScanScratch,
) -> Result<bool, CodecError> {
    scratch.begin();
    ensure_format(tx)?;
    #[cfg(test)]
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.node_scans += 1;
        work.rebind_scans += 1;
    });
    let mut stmt = tx
        .prepare("SELECT digest,tag,body FROM s_content_nodes")
        .map_err(storage_error)?;
    let mut rows = stmt.query([]).map_err(storage_error)?;
    let mut equal = true;
    while let Some(r) = rows.next().map_err(storage_error)? {
        let d = request_capture_digest(r)?;
        let t: i64 = r.get(1).map_err(storage_error)?;
        let t: u8 = t.try_into().map_err(|_| storage_error("捕获标签域"))?;
        let body = match r.get_ref(2).map_err(storage_error)? {
            rusqlite::types::ValueRef::Blob(body) => body,
            _ => {
                // 非BLOB仍走原Vec解码取得完全相同的InvalidColumnType，不改变NULL/TEXT等错误。
                let _: Vec<u8> = r.get(2).map_err(storage_error)?;
                unreachable!("Vec<u8>只接受BLOB")
            }
        };
        #[cfg(test)]
        record_prototype_row(body.len());
        // 必须去重；仅count+lookup会接受重复旧digest替代遗漏旧digest的坏schema。
        let first = scratch.insert(d);
        let same = capture
            .original(&d)
            .is_some_and(|(tag, original)| tag == t && original == body);
        equal &= first && same;
    }
    Ok(equal && scratch.seen.len() == expected_nodes)
}

/// #1394 隔离测试：本次完整捕获同时逐tuple绑定旧原件超集，允许正常新增节点。
/// 不把旧memo移入新捕获；任何行的SQL/type错误均直接返回，包括已发现mismatch之后。
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn capture_with_original_subset_for_source_cache(
    tx: &Transaction<'_>,
    prior: Option<(&s_content::AuditCapture, usize)>,
) -> Result<(s_content::AuditCapture, usize, bool), CodecError> {
    ensure_format(tx)?;
    #[cfg(test)]
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.capture_builds += 1;
        work.node_scans += 1;
        work.rebind_scans += u64::from(prior.is_some());
    });
    let mut nodes = s_content::MemoryNodes::new();
    let mut seen = std::collections::HashSet::new();
    let mut matched = 0usize;
    let mut same = prior.is_some();
    let mut stmt = tx
        .prepare("SELECT digest,tag,body FROM s_content_nodes")
        .map_err(storage_error)?;
    let mut rows = stmt.query([]).map_err(storage_error)?;
    while let Some(row) = rows.next().map_err(storage_error)? {
        let digest = request_capture_digest(row)?;
        let tag: i64 = row.get(1).map_err(storage_error)?;
        let tag: u8 = tag.try_into().map_err(|_| storage_error("捕获标签域"))?;
        let body: Vec<u8> = row.get(2).map_err(storage_error)?;
        #[cfg(test)]
        record_prototype_row(body.len());
        let unique = seen.insert(digest);
        same &= unique;
        if let Some((capture, _)) = prior {
            if let Some((old_tag, old_body)) = capture.original(&digest) {
                matched += usize::from(unique);
                same &= old_tag == tag && old_body == body.as_slice();
            }
        }
        nodes.insert_unchecked(digest, tag, body);
    }
    drop(rows);
    stmt.finalize().map_err(storage_error)?;
    same &= prior.is_some_and(|(_, count)| matched == count);
    let count = nodes.len();
    Ok((s_content::AuditCapture::new(nodes), count, same))
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PrototypeCaptureWork {
    pub capture_builds: u64,
    pub node_scans: u64,
    pub rebind_scans: u64,
    pub decoded_rows: u64,
    pub body_bytes: u64,
    pub digest_blob_borrows: u64,
    pub digest_vec_decode_calls: u64,
}
#[cfg(test)]
thread_local! {
    static PROTOTYPE_CAPTURE_WORK: std::cell::RefCell<PrototypeCaptureWork> = Default::default();
}
#[cfg(test)]
fn record_prototype_row(body_bytes: usize) {
    PROTOTYPE_CAPTURE_WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.decoded_rows += 1;
        work.body_bytes += body_bytes as u64;
    });
}
#[cfg(test)]
pub(crate) fn prototype_capture_work_on_thread() -> PrototypeCaptureWork {
    PROTOTYPE_CAPTURE_WORK.with(|work| *work.borrow())
}

/// #1394：封闭测试取消策略。只能数检查点，不能执行任意回调或持有事务。
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
pub(crate) struct CaptureCancellation {
    checks: std::cell::Cell<usize>,
    cancel_at: Option<usize>,
    deadline: Option<std::time::Instant>,
}
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
impl CaptureCancellation {
    pub(crate) fn never() -> Self {
        Self {
            checks: std::cell::Cell::new(0),
            cancel_at: None,
            deadline: None,
        }
    }
    fn at_check(check: usize) -> Self {
        Self {
            checks: std::cell::Cell::new(0),
            cancel_at: Some(check),
            deadline: None,
        }
    }
    pub(crate) fn until(deadline: Option<std::time::Instant>) -> Self {
        Self {
            checks: std::cell::Cell::new(0),
            cancel_at: None,
            deadline,
        }
    }
    fn check(&self) -> Result<(), CodecError> {
        if self
            .deadline
            .is_some_and(|d| std::time::Instant::now() >= d)
        {
            return Err(CodecError::new("verify_deadline", "捕获读集期限已到"));
        }
        let checks = self.checks.get().saturating_add(1);
        self.checks.set(checks);
        if self.cancel_at.is_some_and(|at| checks >= at) {
            Err(CodecError::new(
                "prototype_capture_cancelled",
                "测试捕获已取消",
            ))
        } else {
            Ok(())
        }
    }
}

/// #1394：显式测试额度，局部 capture 两个固定 Layout 的写前门。绝非全 audit 准入。
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
pub(crate) fn capture_slab_for_prototype(
    tx: &Transaction<'_>,
    budget: &s_content::capture_allocation::OwnedBudget,
    cancellation: &CaptureCancellation,
) -> Result<s_content::capture_allocation::CaptureSlab, CodecError> {
    use s_content::capture_allocation::CaptureSlab;
    ensure_format(tx)?;
    // Two passes in the same transaction. Read borrowed BLOBs, never row.get::<Vec<u8>>.
    // No LENGTH/SUM allocation estimate and no count-to-byte unit mixing.
    fn borrowed_row<'a>(
        r: &'a mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    ) -> Result<(NodeDigest, u8, &'a [u8]), CodecError> {
        let digest: NodeDigest = r
            .blob(0)
            .map_err(storage_error)?
            .try_into()
            .map_err(|_| storage_error("捕获摘要长度"))?;
        let tag: i64 = r.integer(1).map_err(storage_error)?;
        let tag: u8 = tag.try_into().map_err(|_| storage_error("捕获标签域"))?;
        let body = r.blob(2).map_err(storage_error)?;
        Ok((digest, tag, body))
    }
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        tx,
        c"SELECT digest,tag,body FROM s_content_nodes ORDER BY digest",
    )
    .map_err(storage_error)?;
    let mut count = 0usize;
    let mut bodies = 0usize;
    {
        while let Some(mut r) = stmt.next().map_err(storage_error)? {
            cancellation.check()?;
            let (_, _, body) = borrowed_row(&mut r)?;
            count = count
                .checked_add(1)
                .ok_or_else(|| storage_error("节点数溢出"))?;
            bodies = bodies
                .checked_add(body.len())
                .ok_or_else(|| storage_error("body 字节数溢出"))?;
            budget.preflight(CaptureSlab::requested_bytes(count, bodies)?)?;
        }
    }
    cancellation.check()?;
    let mut slab = CaptureSlab::new(count, bodies, budget)?;
    stmt.reset().map_err(storage_error)?;
    {
        while let Some(mut r) = stmt.next().map_err(storage_error)? {
            cancellation.check()?;
            let (d, t, body) = borrowed_row(&mut r)?;
            slab.push(d, t, body)?;
        }
    }
    cancellation.check()?;
    stmt.finalize().map_err(storage_error)?;
    slab.finish()
}

#[cfg(test)]
mod capture_allocation_tests {
    use super::*;
    use s_content::capture_allocation::{CaptureSlab, OwnedBudget, INCOMPLETE, LIMIT};

    fn fixture() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER,protocol_revision TEXT); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
        let tx = c.transaction().unwrap();
        init_schema(&tx).unwrap();
        s_content::write_value(
            &mut TxStore::new(&tx).unwrap(),
            &s_content::Value::String("x".repeat(5000)),
        )
        .unwrap();
        tx.commit().unwrap();
        c
    }
    fn required(tx: &Transaction<'_>) -> usize {
        let (n, bytes): (i64, i64) = tx
            .query_row(
                "SELECT count(*),sum(length(body)) FROM s_content_nodes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        CaptureSlab::requested_bytes(n.try_into().unwrap(), bytes.try_into().unwrap()).unwrap()
    }

    #[test]
    #[ignore = "requires caller-owned private SQLite copy; never connect original"]
    fn owned_current_capture_boundaries() {
        let mut c = Connection::open_with_flags(
            std::env::var("AUDIT_PROTOTYPE_DB").unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let tx = c.transaction().unwrap();
        let min = required(&tx);
        let mut reports = Vec::new(); // Test report outside measured capture owners.
        for cap in [min - 1, min, min + 1] {
            let budget = OwnedBudget::new(cap);
            let capture = capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never());
            if cap < min {
                assert_eq!(capture.err().unwrap().code(), LIMIT);
                assert_eq!(budget.snapshot().copied_rows, 0);
                assert_eq!(budget.snapshot().allocation_attempts, 0);
            } else {
                let capture = capture.unwrap();
                assert_eq!(capture.snapshot().live_requested_layout_bytes, min);
                drop(capture);
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            reports.push(serde_json::json!({"case":"capture-boundary","meter":budget.snapshot()}));
        }
        for cap in [2 * min - 1, 2 * min, 2 * min + 1] {
            let budget = OwnedBudget::new(cap);
            let first =
                capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never()).unwrap();
            let clone = first.try_clone();
            if cap < 2 * min {
                assert_eq!(clone.err().unwrap().code(), LIMIT);
                assert_eq!(budget.snapshot().allocated_layouts, 2);
            } else {
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 2 * min);
                drop(clone.unwrap());
            }
            drop(first);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            reports
                .push(serde_json::json!({"case":"simultaneous-clone","meter":budget.snapshot()}));
        }
        let result = serde_json::json!({"scope":"capture index and body requested Layout only", "whole_audit_admitted":false,"minimum_capture_bytes":min,"record_layout_bytes":CaptureSlab::requested_bytes(1,0).unwrap(),"cases":reports});
        std::fs::write(
            std::env::var("AUDIT_PROTOTYPE_REPORT").unwrap(),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        eprintln!("CURRENT_BOUNDARIES {result}");
    }

    #[test]
    #[ignore = "intentional RED control: unchanged legacy loader allocates despite candidate zero budget"]
    fn legacy_loader_red_exceeds_zero_payload() {
        let mut c = fixture();
        let tx = c.transaction().unwrap();
        let capture = capture_for_prototype(&tx).unwrap();
        let actual = capture.retention()["body_vec_capacity_bytes"]
            .as_u64()
            .unwrap();
        eprintln!("LEGACY_RED body_vec_capacity_bytes={actual} candidate_limit_bytes=0");
        assert_eq!(actual, 0, "旧 loader 没有写前门；此控制应失败");
    }

    #[test]
    fn capture_payload_exact_boundary_and_units() {
        let mut c = fixture();
        let tx = c.transaction().unwrap();
        let min = required(&tx);
        for cap in [min - 1, min, min + 1] {
            let budget = OwnedBudget::new(cap);
            let capture = capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never());
            if cap < min {
                assert_eq!(capture.err().unwrap().code(), LIMIT);
                let m = budget.snapshot();
                assert_eq!(m.allocated_layouts, 0);
                assert_eq!(m.copied_rows, 0);
            } else {
                let capture = capture.unwrap();
                assert_eq!(budget.snapshot().live_requested_layout_bytes, min);
                assert_eq!(
                    capture
                        .require_complete_audit_coverage()
                        .unwrap_err()
                        .code(),
                    INCOMPLETE
                );
                assert_eq!(budget.snapshot().allocated_layouts, 2);
                drop(capture);
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            eprintln!(
                "BOUNDARY {}",
                serde_json::to_string(&budget.snapshot()).unwrap()
            );
        }
        // A slot is NOT a byte: charge the actual columns, alignment and final padding.
        let (index, _) = std::alloc::Layout::array::<NodeDigest>(1)
            .unwrap()
            .extend(std::alloc::Layout::array::<usize>(1).unwrap())
            .unwrap();
        let (index, _) = index
            .extend(std::alloc::Layout::array::<u8>(1).unwrap())
            .unwrap();
        assert_eq!(
            CaptureSlab::requested_bytes(1, 0).unwrap(),
            index.pad_to_align().size()
        );
        assert!(CaptureSlab::requested_bytes(1, 0).unwrap() > 1);
        assert!(CaptureSlab::requested_bytes(usize::MAX, 1).is_err());
        let count: usize = tx
            .query_row("SELECT count(*) FROM s_content_nodes", [], |r| {
                r.get::<_, i64>(0).map(|n| usize::try_from(n).unwrap())
            })
            .unwrap();
        assert!(capture_slab_for_prototype(
            &tx,
            &OwnedBudget::new(count),
            &CaptureCancellation::never()
        )
        .is_err());
    }

    #[test]
    fn simultaneous_clones_release_cancel_and_failed_growth() {
        let mut c = fixture();
        let tx = c.transaction().unwrap();
        let min = required(&tx);
        let budget = OwnedBudget::new(2 * min - 1);
        let first =
            capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never()).unwrap();
        let before = budget.snapshot();
        assert_eq!(first.try_clone().err().unwrap().code(), LIMIT);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        drop(first);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);

        let budget = OwnedBudget::new(2 * min);
        let first =
            capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never()).unwrap();
        let second = first.try_clone().unwrap();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 2 * min);
        drop(first);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, min);
        drop(second);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        eprintln!(
            "CLONE {}",
            serde_json::to_string(&budget.snapshot()).unwrap()
        );

        let count: usize = tx
            .query_row("SELECT count(*) FROM s_content_nodes", [], |r| {
                r.get::<_, i64>(0).map(|n| usize::try_from(n).unwrap())
            })
            .unwrap();
        // Before either pass, before first copy, after one row, and after all rows.
        for (at, allocations, copied_rows) in [
            (1, 0, 0),
            (count + 1, 0, 0),
            (count + 2, 2, 0),
            (count + 3, 2, 1),
            (2 * count + 2, 2, count as u64),
        ] {
            let budget = OwnedBudget::new(min);
            let cancellation = CaptureCancellation::at_check(at);
            let result = capture_slab_for_prototype(&tx, &budget, &cancellation);
            assert_eq!(result.err().unwrap().code(), "prototype_capture_cancelled");
            let m = budget.snapshot();
            assert_eq!(cancellation.checks.get(), at);
            assert_eq!(m.copied_rows, copied_rows);
            assert_eq!(m.allocated_layouts, allocations);
            assert_eq!(m.live_requested_layout_bytes, 0);
            assert_eq!(m.allocated_layouts, m.released_layouts);
            eprintln!(
                "CLOSED_CANCEL at={at} {}",
                serde_json::to_string(&m).unwrap()
            );
        }

        let budget = OwnedBudget::new(1024);
        let mut slab = CaptureSlab::new(1, 1, &budget).unwrap();
        assert_eq!(slab.push([0; 32], 0, &[0; 2]).unwrap_err().code(), LIMIT);
        assert_eq!(budget.snapshot().copied_rows, 0);
        assert!(slab.finish().is_err());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn borrowed_oversized_body_and_error_cleanup() {
        let mut c = fixture();
        let tx = c.transaction().unwrap();
        // SQLite owns this corrupt row. The capture loader must not first obtain an owned Vec.
        tx.execute("UPDATE s_content_nodes SET body=zeroblob(1048576) WHERE digest=(SELECT min(digest) FROM s_content_nodes)", []).unwrap();
        let budget = OwnedBudget::new(4096);
        assert_eq!(
            capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never())
                .err()
                .unwrap()
                .code(),
            LIMIT
        );
        assert_eq!(budget.snapshot().allocated_layouts, 0);
        assert_eq!(budget.snapshot().copied_body_bytes, 0);
        tx.execute("UPDATE s_content_nodes SET body='not-blob' WHERE digest=(SELECT min(digest) FROM s_content_nodes)", []).unwrap();
        assert_eq!(
            capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never())
                .err()
                .unwrap()
                .code(),
            code::SOURCE
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        eprintln!(
            "BORROWED_REJECT {}",
            serde_json::to_string(&budget.snapshot()).unwrap()
        );
    }

    #[test]
    fn slab_owner_preserves_bytes_digest_errors_and_cold_binding() {
        let mut c = fixture();
        let tx = c.transaction().unwrap();
        let capture_required = required(&tx);
        // This semantic fixture also verifies through same-ledger memo/work owners.
        // Exact capture-only admission remains locked by the separate boundary tests.
        let verification_testonly_headroom = 1024 * 1024;
        let budget = OwnedBudget::new(
            capture_required
                .checked_add(verification_testonly_headroom)
                .unwrap(),
        );
        let slab = capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never()).unwrap();
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            capture_required
        );
        let legacy = capture_for_prototype(&tx).unwrap();
        let mut stmt = tx.prepare("SELECT digest FROM s_content_nodes").unwrap();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let digest: NodeDigest = row
                .get_ref(0)
                .unwrap()
                .as_blob()
                .unwrap()
                .try_into()
                .unwrap();
            assert_eq!(slab.original(&digest), legacy.original(&digest));
        }
        drop(rows);
        drop(stmt);
        let root = s_content::write_value(
            &mut s_content::MemoryNodes::new(),
            &s_content::Value::String("x".repeat(5000)),
        )
        .unwrap()
        .content_ref();
        let capture = s_content::AuditCapture::from_slab_for_allocation_prototype(slab);
        capture.verify(&root, s_content::Mode::Public).unwrap();
        let mut bad = root.clone();
        bad.extent = Some(4999);
        assert_eq!(
            capture
                .verify(&bad, s_content::Mode::Public)
                .unwrap_err()
                .code(),
            code::EXTENT
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        tx.execute(
            "UPDATE s_content_nodes SET body=zeroblob(length(body)) WHERE tag=16",
            [],
        )
        .unwrap();
        let broken = s_content::AuditCapture::from_slab_for_allocation_prototype(
            capture_slab_for_prototype(&tx, &budget, &CaptureCancellation::never()).unwrap(),
        );
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            capture_required
        );
        for _ in 0..2 {
            assert_eq!(
                broken
                    .verify(&root, s_content::Mode::Public)
                    .unwrap_err()
                    .code(),
                code::NODE_DIGEST
            );
        }
        drop(broken);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

// #1394 编译负例：闭包在两遍间写同事务的旧调用必须被类型边界拒绝。
// cargo rustc --release --features s_session --bin s_structure_session -- --test --cfg issue1394_capture_callback_refusal
#[cfg(all(test, issue1394_capture_callback_refusal))]
fn capture_callback_must_not_compile(
    tx: &Transaction<'_>,
    budget: &s_content::capture_allocation::OwnedBudget,
    n: usize,
) {
    let mut calls = 0;
    let _ = capture_slab_for_prototype(tx, budget, || {
        calls += 1;
        if calls == n + 1 {
            tx.execute("UPDATE s_content_nodes SET body=zeroblob(length(body))", [])
                .unwrap();
        }
        false
    });
}
