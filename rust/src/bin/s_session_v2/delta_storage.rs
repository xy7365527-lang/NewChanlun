//! #1394：S 私有存储格式登记与两域 Delta 判据。
//!
//! 本模块只管一件事：**这个库的 StructureDelta 是完整活动集还是真实变化集**。它不参与结构判定，
//! 不重算对象，也不改写任何已封存字节。
//!
//! - 新 init 明确落一行登记（`changes-only/1`）；无登记表的旧库继续旧 full 域，不迁移、不混代。
//! - 表存在时必须恰一行、且值为本模块唯一声明的模式；缺行/多行/未声明值/结构不符一律拒绝。
//! - 不提供「旧全量核验失败，再试稀疏」的降级通道：模式由这一处读出，判据只按它走一次。
//! - 模式是缓存身份的一部分（`tag()`），跨模式不复用任何已核证书。
use super::*;

/// 私有格式登记表（不在公共协议里，也不参与公开对象/批次身份）。
pub(super) const TABLE: &str = "s_storage_format";
/// 本 reader/writer 唯一声明的新域模式值。
pub(super) const CHANGES_ONLY: &str = "changes-only/1";
/// 新 init 落库的建表语句；`normalize_schema` 后与持久 DDL 逐字对拍。
pub(super) const DDL: &str = "CREATE TABLE IF NOT EXISTS s_storage_format (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  storage_format TEXT NOT NULL
)";

/// 库的 Delta 域。只有两个明确取值：无登记表的旧 full 域，或登记为 `changes-only/1` 的新域。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Mode {
    /// 无登记表的旧库：Delta 仍是完整活动集（旧 writer 语义，逐字节不变）。
    LegacyFull,
    /// 显式 `changes-only/1`：Delta 只含本代新生/替代对象与各族首次发布。
    ChangesOnly,
}

impl Mode {
    pub(super) fn changes_only(self) -> bool {
        matches!(self, Mode::ChangesOnly)
    }
    /// 缓存/证书身份的稳定标签：模式变化必须使旧证书失效。
    pub(super) fn tag(self) -> &'static str {
        match self {
            Mode::LegacyFull => "legacy-full/0",
            Mode::ChangesOnly => CHANGES_ONLY,
        }
    }
}

/// 读模式。表不存在 = 旧 full 域；表存在 = 恰一行 + 唯一声明值，否则拒绝（不降级、不猜）。
pub(super) fn read_mode(conn: &Connection) -> Result<Mode, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            params![TABLE],
            |r| r.get(0),
        )
        .map_err(|e| format!("StorageUnavailable：读私有格式登记失败：{e}"))?;
    if exists == 0 {
        return Ok(Mode::LegacyFull);
    }
    let mut stmt = conn
        .prepare("SELECT singleton,storage_format FROM s_storage_format")
        .map_err(|e| format!("StorageUnavailable：私有格式登记结构不符：{e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| format!("StorageUnavailable：读私有格式登记失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("StorageUnavailable：私有格式登记行损坏：{e}"))?;
    validate_mode_rows(rows.len(), rows.first().map(|(n, s)| (*n, s.as_str())))
}
pub(super) fn validate_mode_rows(count: usize, first: Option<(i64, &str)>) -> Result<Mode, String> {
    if count != 1 {
        return Err(format!(
            "StorageUnavailable：私有格式登记必须恰一行（实际 {} 行，缺行/多行一律拒绝）",
            count
        ));
    }
    let (singleton, format) = first.expect("one row has a first row");
    if singleton != 1 {
        return Err("StorageUnavailable：私有格式登记 singleton 非 1".into());
    }
    if format != CHANGES_ONLY {
        return Err(format!("SchemaUnsupported：未知存储模式 `{format}`（本 reader 只认无登记表的旧 full 域与 `{CHANGES_ONLY}`）"));
    }
    Ok(Mode::ChangesOnly)
}

/// 新 init 落登记表与唯一一行。只在新建库调用；旧库不补、不迁移、不混代。
pub(super) fn declare_new(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(DDL)
        .map_err(|e| format!("建私有格式登记失败：{e}"))?;
    conn.execute(
        "INSERT INTO s_storage_format(singleton,storage_format) VALUES(1,?1)",
        params![CHANGES_ONLY],
    )
    .map_err(|e| format!("写私有格式登记失败：{e}"))?;
    Ok(())
}
