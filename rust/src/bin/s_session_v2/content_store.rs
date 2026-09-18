//! #1394 C：S3 的内容节点持久边界，使用调用方的同一个 SQLite 事务。
//!
//! s-content/1 与旧 evidence-graph 码本分表；只有新 s-session/3 域可以初始化或读取。
//! 本层保存节点，不发布 cut、不核业务出生许可；完整闭包和独立索引仍由发布者核验。

use newchan_rust::s_content::{self, code, CodecError, NodeDigest, NodeRead, NodeSink, NodeSource};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

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
    let row: Option<(i64, Vec<u8>)> = conn
        .query_row(
            "SELECT tag, body FROM s_content_nodes WHERE digest=?1",
            params![&digest[..]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    let Some((stored_tag, body)) = row else {
        return Ok(NodeRead::Missing);
    };
    let tag = u8::try_from(stored_tag)
        .map_err(|_| CodecError::new(code::NODE_DIGEST, "节点 tag 超过一个字节"))?;
    check_digest(digest, tag, &body)?;
    Ok(NodeRead::Found { tag, body })
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
                    .execute(
                        "INSERT INTO s_content_nodes(digest, tag, body) VALUES(?1, ?2, ?3)",
                        params![&digest[..], i64::from(tag), body],
                    )
                    .map_err(storage_error)?;
            }
        }
        Ok(())
    }
}

pub(super) struct ReadStore<'a> {
    conn: &'a Connection,
}

impl<'a> ReadStore<'a> {
    /// 整次闭包读取共用同一事务快照；调用者不得逐节点重新创建事务。
    pub(super) fn new(tx: &'a Transaction<'_>) -> Result<Self, CodecError> {
        ensure_format(tx)?;
        Ok(Self { conn: tx })
    }
}

impl NodeSource for ReadStore<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_node(self.conn, digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
