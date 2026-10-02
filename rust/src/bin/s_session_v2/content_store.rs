//! #1394 C：S3 的内容节点持久边界，使用调用方的同一个 SQLite 事务。
//!
//! s-content/1 与旧 evidence-graph 码本分表；只有新 s-session/3 域可以初始化或读取。
//! 本层保存节点，不发布 cut、不核业务出生许可；完整闭包和独立索引仍由发布者核验。

use newchan_rust::s_content::{self, code, CodecError, NodeDigest, NodeRead, NodeSink, NodeSource};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::BTreeMap;

fn storage_error(error: impl std::fmt::Display) -> CodecError {
    CodecError::new(code::SOURCE, format!("S3 内容存储：{error}"))
}

fn ensure_domain(conn: &Connection) -> Result<(), CodecError> {
    let protocol: Option<String> = conn
        .query_row(
            "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    if protocol.as_deref() != Some("s-session/3") {
        return Err(CodecError::new(
            code::FORMAT,
            "内容节点表只属于已登记的 s-session/3 域",
        ));
    }
    Ok(())
}

pub(super) fn ensure_format(conn: &Connection) -> Result<(), CodecError> {
    ensure_domain(conn)?;
    let format: Option<String> = conn
        .query_row(
            "SELECT content_format FROM s_content_meta WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    if format.as_deref() != Some(s_content::CONTENT_FORMAT) {
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
    // 每次真实 SELECT 当前原件；tag 转换和错误处理先于任何证明复用。
    #[cfg(test)]
    SQL_READS.with(|calls| calls.set(calls.get() + 1));
    let row: Option<(i64, Vec<u8>)> = conn
        .prepare_cached("SELECT tag, body FROM s_content_nodes WHERE digest=?1")
        .map_err(storage_error)?
        .query_row(params![&digest[..]], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(storage_error)?;
    let Some((stored_tag, body)) = row else {
        return Ok(None);
    };
    let tag = u8::try_from(stored_tag)
        .map_err(|_| CodecError::new(code::NODE_DIGEST, "节点 tag 超过一个字节"))?;
    Ok(Some((tag, body)))
}

#[cfg(test)]
thread_local! { static SQL_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
pub(super) fn sql_reads_on_thread() -> u64 {
    SQL_READS.with(std::cell::Cell::get)
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
        if let Some((old, _)) = self.frames.get(digest) {
            if old.tag() == tag && old.body() == body.as_slice() {
                return old.checked_same_material(digest, tag, body);
            }
            let (_, capacity) = self.frames.remove(digest).expect("已找到的局部素材");
            self.retained_capacity -= capacity;
        }
        let frame = s_content::job::CheckedFrame::checked(digest, tag, body)?;
        // 复制前先按长度/条数筛选，复制后再按 Vec 的实际 capacity 筛选。
        let remaining = max_bytes.saturating_sub(self.retained_capacity);
        if self.frames.len() < max_entries && frame.body().len() <= remaining {
            let retained = frame.body().to_vec();
            let capacity = retained.capacity();
            if capacity <= remaining {
                let proof = frame.checked_same_material(digest, tag, retained)?;
                self.frames.insert(*digest, (proof, capacity));
                self.retained_capacity += capacity;
            }
        }
        Ok(frame)
    }
}

pub(super) struct TxStore<'a, 'conn> {
    tx: &'a Transaction<'conn>,
}

impl<'a, 'conn> TxStore<'a, 'conn> {
    pub(super) fn new(tx: &'a Transaction<'conn>) -> Result<Self, CodecError> {
        ensure_format(tx)?;
        Ok(Self { tx })
    }
}

impl NodeSource for TxStore<'_, '_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_node(self.tx, digest)
    }

    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        read_checked_node(self.tx, digest)
    }
}

impl NodeSink for TxStore<'_, '_> {
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        check_digest(&digest, tag, body)?;
        match read_node(self.tx, &digest)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } => {
                if old_tag != tag || old_body != body {
                    return Err(CodecError::new(
                        code::NODE_DIGEST,
                        "同摘要异原件：保留旧节点并拒绝写入",
                    ));
                }
            }
            NodeRead::Missing => {
                self.tx
                    .prepare_cached(
                        "INSERT INTO s_content_nodes(digest, tag, body) VALUES(?1, ?2, ?3)",
                    )
                    .map_err(storage_error)?
                    .execute(params![&digest[..], i64::from(tag), body])
                    .map_err(storage_error)?;
            }
        }
        Ok(())
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

#[cfg(test)]
pub(crate) fn capture_for_prototype(
    tx: &Transaction<'_>,
) -> Result<s_content::AuditCapture, CodecError> {
    ensure_format(tx)?;
    let mut nodes = s_content::MemoryNodes::new();
    let mut stmt = tx
        .prepare("SELECT digest,tag,body FROM s_content_nodes")
        .map_err(storage_error)?;
    let mut rows = stmt.query([]).map_err(storage_error)?;
    while let Some(r) = rows.next().map_err(storage_error)? {
        let d: Vec<u8> = r.get(0).map_err(storage_error)?;
        let d: NodeDigest = d.try_into().map_err(|_| storage_error("捕获摘要长度"))?;
        let t: i64 = r.get(1).map_err(storage_error)?;
        let t: u8 = t.try_into().map_err(|_| storage_error("捕获标签域"))?;
        nodes.insert_unchecked(d, t, r.get(2).map_err(storage_error)?);
    }
    Ok(s_content::AuditCapture::new(nodes))
}
