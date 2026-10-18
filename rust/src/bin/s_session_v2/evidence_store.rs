//! #1394 第二段：完整证据的私有内容寻址共享表示（`evidence-graph/3`）。
//!
//! 本模块只解决**物理表示**：把 S 已经算出的完整规范 JSON（批次、Delta 原件、结构事实的
//! payload/input_refs/source_coords）换成一棵内容寻址的**无损**节点图，使跨代重复的原件只落
//! 一次盘，同时保证：
//!
//! - **公开身份不变**：节点图不参与 `object_id`/`batch_id`/任何公开字段。展开字节必须与
//!   `canonical_json` 逐字节相同；公开哈希仍按**完整展开字节**重算，不改成 Merkle 身份。
//! - **不是「先构造再压缩」**：调用方把已算好的完整 Value 交给本模块，模块在**写入前**用
//!   自己的展开结果与同一份规范字节对拍；对拍失败即拒绝写入。本节**不声称**上游构造已增量。
//! - **字节精确**：叶保存现有规范字节；JSON 字符串内部嵌套的另一层 JSON 只有在
//!   「解析→按同一 serializer 回写→与该字符串内容逐字节相同」时才拆开共享，否则整段作叶。
//!   转义序列按字符处理，不会把 `\"` 拆到两个叶之间。
//! - **显式版本化**：新库在 `s_evidence_format` 落唯一一行 `evidence-graph/3`；表缺 = 旧库
//!   （整块 BLOB）。已落的 `evidence-graph/2`（只把 batch 收进私有图）继续**只读**兼容——同一
//!   个名字下不得换存储含义，故 v3 是新的显式版本而不是原地改判据。表在而值未知/缺行/多行一律
//!   拒绝。不提供「严格失败再试宽松」的降级通道，也不提供原地迁移。
//! - **不原地迁移**：只在新建库声明；旧库（整块域与 graph/2 域）不补、不迁、不混代。
//! - **迭代遍历/展开**：闭包遍历与展开都走显式栈，图深与输入长度无关，避免 9011 层递归。
//! - **同 ID 异内容拒绝**：节点与根按内容寻址；同 ID 已存在时逐字节比对，不符即 `IdentityConflict`。
//! - **已核句柄复用（2D）**：每次捕获仍从**新鲜行**读全部根、全部唯一节点原行，重算闭包指纹；
//!   指纹与此前成功核过公开长度/摘要的那一次相同才复用其展开字节/解析结果。节点内容、长度、
//!   根行或参数槽任一变化都会改变指纹，从而撤销复用并严格重核。缓存超预算只失去复用。

use super::*;
use crate::v3::audit_store::{CodecRead, NativeAdmission};
use crate::v3::business_json::{new_map, new_work, BusinessMap, TextValue};
use newchan_rust::s_content::capture_allocation::scratch::WorkScope;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};
use newchan_rust::sqlite_checked::CheckedStatement;

/// Checked SQL保typed cause；固定投影只补回rusqlite原有列名。
pub(super) fn sql_columns(error: rusqlite::Error, names: &[&str]) -> rusqlite::Error {
    match error {
        rusqlite::Error::InvalidColumnType(i, _, kind) => rusqlite::Error::InvalidColumnType(
            i,
            names.get(i).copied().unwrap_or("").to_owned(),
            kind,
        ),
        other => other,
    }
}
fn work_copy<T: Copy>(bytes: &[T], owner: Option<&OwnedBudget>) -> Result<CodecVec<T>, String> {
    work_copy_admitted(bytes, owner, None)
}
fn work_copy_admitted<T: Copy>(
    bytes: &[T],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<T>, String> {
    let admission = NativeAdmission(work);
    let mut out = new_work(owner);
    admission
        .native_codec_room(&mut out, bytes.len())
        .map_err(|e| e.to_string())?;
    out.try_reserve_exact(bytes.len())
        .map_err(|e| e.to_string())?;
    for byte in bytes {
        out.try_push(*byte).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
fn work_resize<T: Copy>(v: &mut CodecVec<T>, len: usize, value: T) -> Result<(), String> {
    work_resize_admitted(v, len, value, None)
}
fn work_resize_admitted<T: Copy>(
    v: &mut CodecVec<T>,
    len: usize,
    value: T,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let admission = NativeAdmission(work);
    if len > v.len() {
        admission
            .native_codec_room(v, len - v.len())
            .map_err(|e| e.to_string())?;
        v.try_reserve_exact(len - v.len())
            .map_err(|e| e.to_string())?;
    }
    while v.len() < len {
        v.try_push(value).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn hash_matches(bytes: &[u8], expected: &str) -> bool {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let hex = b"0123456789abcdef";
    expected.bytes().eq(digest
        .iter()
        .flat_map(|b| [hex[(b >> 4) as usize], hex[(b & 15) as usize]]))
}

/// 私有格式登记表（不在公共协议里，也不参与公开对象/批次身份）。
pub(super) const FORMAT_TABLE: &str = "s_evidence_format";
/// 不可变节点表。
pub(super) const NODES_TABLE: &str = "s_evidence_nodes";
/// 公开身份 → 私有根的映射表。
pub(super) const ROOTS_TABLE: &str = "s_evidence_roots";
/// 本 writer 唯一声明、且唯一可写的图格式：批次 + Delta 原件 + 结构事实三处大原件同图。
pub(super) const GRAPH_FORMAT: &str = "evidence-graph/3";
/// 上一版图格式：只把 batch 收进私有图（Delta/事实仍在行内整块列）。本 reader 只读兼容。
pub(super) const GRAPH_FORMAT_V2: &str = "evidence-graph/2";

/// 数组分块宽度：同一数组在相邻代际之间只要块内容相同就整块复用。
const BLOCK: usize = 16;
/// 超过该长度的 JSON 字符串单独作叶（小于此长度内联更省行开销）。
const STRING_LEAF_MIN: usize = 256;

/// 新 init 落库的私有 DDL；`normalize_schema` 后与持久 DDL 逐字对拍。
pub(super) const DDL: &str = "CREATE TABLE IF NOT EXISTS s_evidence_format (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  graph_format TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS s_evidence_nodes (
  node_id BLOB PRIMARY KEY,
  node_kind INTEGER NOT NULL,
  payload BLOB NOT NULL,
  expanded_length INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS s_evidence_roots (
  root_key TEXT PRIMARY KEY,
  node_id BLOB NOT NULL,
  slot_ids BLOB NOT NULL,
  expanded_length INTEGER NOT NULL,
  content_sha256 TEXT NOT NULL
)";

/// 节点种类：0 = 精确字节叶；1 = 有序拼接节点。
const KIND_LEAF: i64 = 0;
const KIND_SEQ: i64 = 1;
/// 拼接节点 payload 的分量标签。
const TAG_LIT: u8 = 0;
const TAG_REF: u8 = 1;
const TAG_SLOT: u8 = 2;

/// 库的批次物理域：无登记表的旧库（整块 BLOB），或显式 `evidence-graph/2`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Format {
    /// 旧库：`batches.canonical_bytes` 存完整规范字节，读取直接取 BLOB。
    LegacyBlob,
    /// `evidence-graph/2`：只有 batch 走节点图，Delta/事实仍是行内整块列。**只读**。
    GraphV2,
    /// `evidence-graph/3`：批次、Delta 原件、结构事实三处大原件同图；对应行内列只留空占位。
    GraphV3,
}

impl Format {
    /// 批次是否走私有图（v2/v3 都是）。
    pub(super) fn graph(self) -> bool {
        matches!(self, Format::GraphV2 | Format::GraphV3)
    }
    /// Delta 原件与结构事实大列是否走私有图（只有 v3）。
    pub(super) fn records_in_graph(self) -> bool {
        matches!(self, Format::GraphV3)
    }
    /// 写入域门：v2 库只读兼容，不原地迁移、不换存储含义。
    pub(super) fn writable(self) -> Result<(), String> {
        match self {
            Format::LegacyBlob | Format::GraphV3 => Ok(()),
            Format::GraphV2 => Err(format!(
                "SchemaUnsupported：本库声明 `{GRAPH_FORMAT_V2}`（只有 batch 走私有图）；本 writer 只写 \
                 `{GRAPH_FORMAT}`，旧格式库只读兼容，不原地迁移、不降级猜测"
            )),
        }
    }
    /// 缓存/证书身份的稳定标签：物理域变化必须使旧证书失效。
    pub(super) fn tag(self) -> &'static str {
        match self {
            Format::LegacyBlob => "batch-blob/0",
            Format::GraphV2 => GRAPH_FORMAT_V2,
            Format::GraphV3 => GRAPH_FORMAT,
        }
    }
}

/// 读物理域。表不存在 = 旧库；表存在 = 恰一行 + 唯一声明值，否则拒绝（不降级、不猜）。
pub(super) fn read_format(conn: &Connection) -> Result<Format, String> {
    read_format_with_owner(conn, None)
}
pub(super) fn read_format_with_owner(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<Format, String> {
    read_format_admitted(conn, owner, None)
}
pub(super) fn read_format_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<Format, String> {
    let admission = NativeAdmission(work);
    let read_err = |e| {
        format!(
            "StorageUnavailable：读私有图格式登记失败：{}",
            sql_columns(e, &["COUNT(*)"])
        )
    };
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
    )
    .map_err(read_err)?;
    stmt.bind_text(1, FORMAT_TABLE).map_err(read_err)?;
    let mut row = stmt
        .next()
        .map_err(read_err)?
        .ok_or("StorageUnavailable：读私有图格式登记失败：count 缺行")?;
    let exists = row.integer(0).map_err(read_err)?;
    drop(row);
    stmt.finalize().map_err(read_err)?;
    if exists == 0 {
        return Ok(Format::LegacyBlob);
    }
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT singleton,graph_format FROM s_evidence_format",
    )
    .map_err(|e| format!("StorageUnavailable：私有图格式登记结构不符：{e}"))?;
    let mut rows = new_work(owner);
    while let Some(mut row) = stmt.next().map_err(|e| {
        format!(
            "StorageUnavailable：私有图格式登记行损坏：{}",
            sql_columns(e, &["singleton", "graph_format"])
        )
    })? {
        let singleton = row.integer(0).map_err(|e| {
            format!(
                "StorageUnavailable：私有图格式登记行损坏：{}",
                sql_columns(e, &["singleton", "graph_format"])
            )
        })?;
        let format = TextValue::from_str_admitted(
            row.text(1).map_err(|e| {
                format!(
                    "StorageUnavailable：私有图格式登记行损坏：{}",
                    sql_columns(e, &["singleton", "graph_format"])
                )
            })?,
            owner,
            work,
        )?;
        admission
            .native_codec_room(&mut rows, 1)
            .map_err(|e| e.to_string())?;
        rows.try_push((singleton, format))
            .map_err(|e| e.to_string())?;
    }
    stmt.finalize().map_err(|e| {
        format!(
            "StorageUnavailable：私有图格式登记行损坏：{}",
            sql_columns(e, &["singleton", "graph_format"])
        )
    })?;
    let [(singleton, format)] = &rows[..] else {
        return Err(format!(
            "StorageUnavailable：私有图格式登记必须恰一行（实际 {} 行，缺行/多行一律拒绝）",
            rows.len()
        ));
    };
    if *singleton != 1 {
        return Err("StorageUnavailable：私有图格式登记 singleton 非 1".into());
    }
    match format.as_str() {
        GRAPH_FORMAT => Ok(Format::GraphV3),
        GRAPH_FORMAT_V2 => Ok(Format::GraphV2),
        other => Err(format!(
            "SchemaUnsupported：未知证据图格式 `{other}`（本 reader 只认无登记表的旧整块域、\
             `{GRAPH_FORMAT_V2}` 与 `{GRAPH_FORMAT}`）"
        )),
    }
}

/// 读物理域（本捕获内按连接复用同一结果；捕获结束即作废，下次仍从当前行重读）。
///
/// 这是全部读写路径的**唯一**格式入口——不提供「严格失败再试宽松」的降级通道。
pub(super) fn format(conn: &Connection) -> Result<Format, String> {
    format_admitted(conn, None)
}
pub(super) fn format_admitted(
    conn: &Connection,
    work: Option<&WorkScope>,
) -> Result<Format, String> {
    let conn_id = conn as *const Connection as usize;
    if let Some(cached) = cache::capture_format(conn_id) {
        return Ok(cached);
    }
    let format = read_format_admitted(conn, None, work)?;
    cache::capture_format_set(conn_id, format);
    Ok(format)
}

/// 私有图三表是否齐全：0 张 = 旧整块域；3 张 = 图域；其余（半降级）一律拒绝。
pub(super) fn graph_tables_present(conn: &Connection) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN (?1,?2,?3)",
            params![FORMAT_TABLE, NODES_TABLE, ROOTS_TABLE],
            |r| r.get(0),
        )
        .map_err(|e| format!("StorageUnavailable：核私有证据图表失败：{e}"))?;
    match count {
        0 => Ok(false),
        3 => Ok(true),
        n => Err(format!(
            "StorageUnavailable：私有证据图表不齐全（实际 {n}/3，三表必须同生）"
        )),
    }
}

/// 新 init 落私有 DDL 与唯一一行声明。只在新建库调用；旧库不补、不迁移、不混代。
pub(super) fn declare_new(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(DDL)
        .map_err(|e| format!("建私有证据图表失败：{e}"))?;
    conn.execute(
        "INSERT INTO s_evidence_format(singleton,graph_format) VALUES(1,?1)",
        params![GRAPH_FORMAT],
    )
    .map_err(|e| format!("写私有图格式登记失败：{e}"))?;
    Ok(())
}

/// 根所属的逻辑记录族。命名空间是物理层的显式声明：一个根对应且只对应一条逻辑记录。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RootKind {
    /// 不可变批次：逻辑记录 = `batches` 行（公开身份 = `batch_id`）。
    Batch,
    /// 一代 StructureDelta 原件：逻辑记录 = `structure_deltas` 行（主键 = generation）。
    Delta,
    /// 一条结构事实的三列大原件（payload/input_refs/source_coords）：逻辑记录 = `structure_facts` 行。
    Fact,
}

impl RootKind {
    fn prefix(self) -> &'static str {
        match self {
            RootKind::Batch => "batch/",
            RootKind::Delta => "delta/",
            RootKind::Fact => "fact/",
        }
    }
}

/// 公开批次身份 → 私有根 key。公开 ID 与私有根句柄分列，永不混用。
pub(super) fn batch_root_key(batch_id: &str) -> String {
    format!("{}{batch_id}", RootKind::Batch.prefix())
}
pub(super) fn batch_root_key_with_owner(
    batch_id: &str,
    owner: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    batch_root_key_admitted(batch_id, owner, None)
}
pub(super) fn batch_root_key_admitted(
    batch_id: &str,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<TextValue, String> {
    let admission = NativeAdmission(work);
    let mut bytes = new_work(owner);
    admission
        .native_codec_room(
            &mut bytes,
            RootKind::Batch
                .prefix()
                .len()
                .checked_add(batch_id.len())
                .ok_or_else(|| admission.native_overflow().to_string())?,
        )
        .map_err(|e| e.to_string())?;
    bytes
        .try_extend_bytes(RootKind::Batch.prefix().as_bytes())
        .map_err(|e| e.to_string())?;
    bytes
        .try_extend_bytes(batch_id.as_bytes())
        .map_err(|e| e.to_string())?;
    TextValue::from_bytes(bytes)
}

/// 一代 Delta 原件 → 私有根 key。
pub(super) fn delta_root_key(generation: i64) -> String {
    format!("{}{generation}", RootKind::Delta.prefix())
}

/// 一条结构事实 → 私有根 key（事实表主键就是 object_id）。
pub(super) fn fact_root_key(object_id: &str) -> String {
    format!("{}{object_id}", RootKind::Fact.prefix())
}

/// 解析根 key 的命名空间；未声明的命名一律拒绝（不猜、不忽略）。
pub(super) fn root_kind(root_key: &str) -> Result<(RootKind, &str), String> {
    for kind in [RootKind::Batch, RootKind::Delta, RootKind::Fact] {
        if let Some(rest) = root_key.strip_prefix(kind.prefix()) {
            if rest.is_empty() {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 缺逻辑记录标识"
                ));
            }
            if kind == RootKind::Delta {
                let generation = parse_canonical_i64(rest, "Delta 私有根代数")
                    .map_err(|e| format!("StorageUnavailable：{e}"))?;
                if generation < 1 {
                    return Err(format!(
                        "StorageUnavailable：Delta 私有根 `{root_key}` 代数必须为正整数"
                    ));
                }
            }
            return Ok((kind, rest));
        }
    }
    Err(format!(
        "StorageUnavailable：私有根 `{root_key}` 不在已声明命名空间（batch/delta/fact）"
    ))
}

/// 事实根的展开形态：三列原件的精确文本。
pub(super) fn fact_root_value(
    payload_json: &str,
    input_refs_json: &str,
    source_coords_json: &str,
) -> Value {
    json!([payload_json, input_refs_json, source_coords_json])
}

/// 从事实根展开字节还原三列精确文本（数组布局、元素必须是字符串；否则拒绝）。
pub(super) fn split_fact_root(
    bytes: &[u8],
    what: &str,
) -> Result<(String, String, String), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| format!("StorageUnavailable：{what} 事实原件非 UTF-8：{e}"))?;
    let value = v2::strict_json(text.as_bytes())
        .map_err(|e| format!("StorageUnavailable：{what} 事实原件非严格 JSON：{e}"))?;
    let items = value
        .as_array()
        .ok_or_else(|| format!("StorageUnavailable：{what} 事实原件必须是三元素数组"))?;
    let [payload, refs, coords] = items.as_slice() else {
        return Err(format!(
            "StorageUnavailable：{what} 事实原件必须是三元素数组（实际 {}）",
            items.len()
        ));
    };
    let take = |v: &Value, label: &str| -> Result<String, String> {
        v.as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("StorageUnavailable：{what} 事实原件{label}必须是文本"))
    };
    Ok((
        take(payload, "payload")?,
        take(refs, "input_refs")?,
        take(coords, "source_coords")?,
    ))
}

// ───────────────────────────── 写入：staging ─────────────────────────────

/// 一次 staging 内可被抽成「代际参数槽」的标量。
///
/// 判据是**精确类型 + 精确规范字节**双匹配：同一批次的生成态字段（generation / cut /
/// 事务时钟）在每一代都取同一个值，抽成槽之后同一模板跨代共享；槽值本身另存为独立叶。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SlotKind {
    /// 数值：规范十进制文本。
    Number,
    /// 字符串：`serde_json::to_string(&str)` 的带引号规范字节。
    Str,
}

/// 调用方给定的一条槽声明。
pub(super) struct SlotSpec {
    pub kind: SlotKind,
    pub bytes: Vec<u8>,
}

impl SlotSpec {
    pub(super) fn number(text: &str) -> Self {
        SlotSpec {
            kind: SlotKind::Number,
            bytes: text.as_bytes().to_vec(),
        }
    }
    pub(super) fn string(value: &str) -> Self {
        SlotSpec {
            kind: SlotKind::Str,
            bytes: serde_json::to_string(value)
                .expect("字符串规范序列化")
                .into_bytes(),
        }
    }
}

/// 拼接节点的一个分量。
enum Child {
    /// 内联字面字节（只在叶以下的小片段出现）。
    Lit(Vec<u8>),
    /// 引用一个不可变节点。
    Ref([u8; 32]),
    /// 代际参数槽。
    Slot(u16),
}

/// 一个不可变节点。
#[derive(Clone)]
pub(super) struct Node {
    pub(super) kind: i64,
    pub(super) payload: Vec<u8>,
    pub(super) expanded_length: u64,
}

/// `stage_complete` 的产物：本代首次生成的节点集合 + 根句柄。
///
/// 节点按 node_id 升序排列，写入顺序确定，便于对拍。
pub(super) struct Staged {
    pub(super) nodes: Vec<([u8; 32], Node)>,
    pub(super) root: [u8; 32],
    pub(super) slot_ids: Vec<[u8; 32]>,
    pub(super) expanded_length: u64,
    pub(super) content_sha256: String,
    /// staging 期间新生成的节点数（不是库内新增数；库内新增由发布时计数）。
    pub(super) staged_nodes: usize,
    pub(super) staged_bytes: u64,
}

/// 一次 staging 的累加器。
pub(super) struct Stager {
    params: Vec<Vec<u8>>,
    index: std::collections::BTreeMap<(u8, Vec<u8>), u16>,
    nodes: std::collections::BTreeMap<[u8; 32], Node>,
    /// staging 内部引用了一个本代未生成的节点（只可能是实现缺陷）：记录后由 `stage_complete` 拒绝。
    missing_ref: Option<[u8; 32]>,
}

impl Stager {
    pub(super) fn new(slots: &[SlotSpec]) -> Result<Self, String> {
        if slots.len() > u16::MAX as usize {
            return Err("SchemaUnsupported：代际参数槽超出 u16 域".into());
        }
        let mut params = Vec::with_capacity(slots.len());
        let mut index = std::collections::BTreeMap::new();
        for (i, s) in slots.iter().enumerate() {
            let tag = match s.kind {
                SlotKind::Number => 0u8,
                SlotKind::Str => 1u8,
            };
            // 同代两个声明取同一个值时（例：某代 generation 与 input_frontier 同为 "1"），
            // 保留先出现的那个下标；重复声明不再单列独立槽，模板形状因此跨代稳定。
            index.entry((tag, s.bytes.clone())).or_insert(i as u16);
            params.push(s.bytes.clone());
        }
        Ok(Stager {
            params,
            index,
            nodes: std::collections::BTreeMap::new(),
            missing_ref: None,
        })
    }

    /// 交给 `stage_complete` 的槽字节（发布前登记为独立叶）。
    pub(super) fn param_bytes(&self) -> &[Vec<u8>] {
        &self.params
    }

    fn leaf(&mut self, bytes: Vec<u8>) -> Child {
        let id = node_id(KIND_LEAF, &bytes);
        if !self.nodes.contains_key(&id) {
            self.nodes.insert(
                id,
                Node {
                    kind: KIND_LEAF,
                    expanded_length: bytes.len() as u64,
                    payload: bytes,
                },
            );
        }
        Child::Ref(id)
    }

    /// 生成（或命中已生成的）拼接节点。
    fn seq(&mut self, children: Vec<Child>) -> Child {
        let mut payload = Vec::with_capacity(children.len() * 8);
        let mut expanded = 0u64;
        for c in &children {
            match c {
                Child::Lit(b) => {
                    payload.push(TAG_LIT);
                    payload.extend_from_slice(&(b.len() as u32).to_be_bytes());
                    payload.extend_from_slice(b);
                    expanded += b.len() as u64;
                }
                Child::Ref(id) => {
                    payload.push(TAG_REF);
                    payload.extend_from_slice(id);
                    match self.nodes.get(id) {
                        Some(n) => expanded += n.expanded_length,
                        // 引用本代未生成的节点：长度不可知，留待 `stage_complete` 拒绝（不猜 0）。
                        None => self.missing_ref = Some(*id),
                    }
                }
                Child::Slot(i) => {
                    payload.push(TAG_SLOT);
                    payload.extend_from_slice(&i.to_be_bytes());
                    // 槽的字节不计入节点声明长度：槽取值随代际变化，节点必须留下一个
                    // **与参数无关**的长度不变量，否则跨代复用后无法再核对任何长度。
                    let _ = i;
                }
            }
        }
        let id = node_id(KIND_SEQ, &payload);
        if !self.nodes.contains_key(&id) {
            self.nodes.insert(
                id,
                Node {
                    kind: KIND_SEQ,
                    payload,
                    expanded_length: expanded,
                },
            );
        }
        Child::Ref(id)
    }

    /// 把一个规范 JSON 值 intern 成节点。`escape` = true 表示这些字节落在一个 JSON 字符串
    /// 内部，字面量必须先按同一 serializer 转义。
    fn intern(&mut self, value: &Value, escape: bool) -> Result<Child, String> {
        match value {
            Value::Object(map) => {
                let mut children = Vec::with_capacity(map.len() * 4 + 1);
                children.push(Child::Lit(esc(escape, "{")));
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        children.push(Child::Lit(esc(escape, ",")));
                    }
                    let key = serde_json::to_string(k).map_err(|e| e.to_string())?;
                    children.push(Child::Lit(esc(escape, &format!("{key}:"))));
                    let child = self.intern(v, escape)?;
                    children.push(child);
                }
                children.push(Child::Lit(esc(escape, "}")));
                Ok(self.seq(children))
            }
            Value::Array(items) => self.intern_array(items, escape),
            Value::String(_) if escape => {
                // 字符串内部的字符串：整段按同一 serializer **再转义一次**后作叶。
                // 少了这一步，候选展开就永远比目标短（`"` 未写成 `\"`），整段拆分会被
                // 自核对安静地退回成一个大叶——字节仍对，但跨代共享退化成按字符串整体去重。
                let canonical = canonical_bytes_of_value(value);
                let text = std::str::from_utf8(&canonical)
                    .map_err(|e| format!("SchemaUnsupported：字符串规范字节非 UTF-8：{e}"))?;
                let mut escaped = Vec::with_capacity(canonical.len() + 2);
                push_escaped(&mut escaped, text);
                Ok(self.leaf(escaped))
            }
            Value::String(s) => {
                let bytes = canonical_bytes_of_value(value);
                if !escape {
                    if let Some(slot) = self.match_slot(1, &bytes) {
                        return Ok(Child::Slot(slot));
                    }
                }
                if s.len() >= STRING_LEAF_MIN {
                    if let Some(inner) = decomposable_json_text(s) {
                        if let Some(child) = self.try_string_decomposition(inner, &bytes)? {
                            return Ok(child);
                        }
                    }
                    return Ok(self.leaf(bytes));
                }
                Ok(Child::Lit(bytes))
            }
            Value::Number(_) => {
                let bytes = canonical_bytes_of_value(value);
                if !escape {
                    if let Some(slot) = self.match_slot(0, &bytes) {
                        return Ok(Child::Slot(slot));
                    }
                }
                Ok(Child::Lit(bytes))
            }
            Value::Bool(_) | Value::Null => Ok(Child::Lit(canonical_bytes_of_value(value))),
        }
    }

    /// JSON 字符串内部嵌 JSON 文本时的无损拆分：先按子值 intern（escape 模式），
    /// 再把候选展开与目标字节逐字节对拍；不一致即放弃（调用方退回整段作叶）。
    fn try_string_decomposition(
        &mut self,
        inner: Value,
        target: &[u8],
    ) -> Result<Option<Child>, String> {
        let snapshot = self.nodes.clone();
        let mut children = Vec::with_capacity(3);
        children.push(Child::Lit(b"\"".to_vec()));
        children.push(self.intern(&inner, true)?);
        children.push(Child::Lit(b"\"".to_vec()));
        let candidate = self.seq(children);
        let expanded = self.expand_staged(&candidate)?;
        if expanded == target {
            Ok(Some(candidate))
        } else {
            self.nodes = snapshot;
            Ok(None)
        }
    }

    fn intern_array(&mut self, items: &[Value], escape: bool) -> Result<Child, String> {
        if items.is_empty() {
            return Ok(self.seq(vec![
                Child::Lit(esc(escape, "[")),
                Child::Lit(esc(escape, "]")),
            ]));
        }
        if items.len() <= BLOCK {
            let mut children = Vec::with_capacity(items.len() * 2 + 1);
            children.push(Child::Lit(esc(escape, "[")));
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    children.push(Child::Lit(esc(escape, ",")));
                }
                let child = self.intern(item, escape)?;
                children.push(child);
            }
            children.push(Child::Lit(esc(escape, "]")));
            return Ok(self.seq(children));
        }
        let mut blocks: Vec<Child> = Vec::with_capacity(items.len() / BLOCK + 1);
        for group in items.chunks(BLOCK) {
            let mut children = Vec::with_capacity(group.len() * 2);
            for (i, item) in group.iter().enumerate() {
                if i > 0 {
                    children.push(Child::Lit(esc(escape, ",")));
                }
                let child = self.intern(item, escape)?;
                children.push(child);
            }
            blocks.push(self.seq(children));
        }
        while blocks.len() > 1 {
            let mut next: Vec<Child> = Vec::with_capacity(blocks.len().div_ceil(2));
            let mut it = blocks.into_iter();
            while let Some(left) = it.next() {
                match it.next() {
                    Some(right) => {
                        next.push(self.seq(vec![left, Child::Lit(esc(escape, ",")), right]))
                    }
                    None => next.push(left),
                }
            }
            blocks = next;
        }
        Ok(self.seq(vec![
            Child::Lit(esc(escape, "[")),
            blocks.pop().expect("非空块树"),
            Child::Lit(esc(escape, "]")),
        ]))
    }

    fn match_slot(&self, tag: u8, bytes: &[u8]) -> Option<u16> {
        self.index.get(&(tag, bytes.to_vec())).copied()
    }

    /// staging 内部展开（自核对用；同一次 staging 的节点都在内存里）。
    fn expand_staged(&self, child: &Child) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut stack = new_work::<Frame>(None);
        match child {
            Child::Lit(b) => stack
                .try_push(Frame::Lit(CodecVec::from_plain(b.clone())))
                .map_err(|e| e.to_string())?,
            Child::Slot(i) => stack.try_push(Frame::Slot(*i)).map_err(|e| e.to_string())?,
            Child::Ref(id) => stack
                .try_push(Frame::Open(*id))
                .map_err(|e| e.to_string())?,
        }
        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Lit(b) => out.extend_from_slice(&b),
                Frame::Slot(i) => out.extend_from_slice(&self.params[i as usize]),
                Frame::Open(id) => {
                    let node = self
                        .nodes
                        .get(&id)
                        .ok_or("StorageUnavailable：staging 展开缺节点")?;
                    decode_into(node.kind, &node.payload, &mut stack)?;
                }
                Frame::Close { .. } => {
                    return Err("StorageUnavailable：staging 展开出现非法收束帧".into())
                }
            }
        }
        Ok(out)
    }
}

/// 字面量按需转义（`escape` 由所在层级决定，与 `serde_json` 同规则）。
fn esc(escape: bool, text: &str) -> Vec<u8> {
    if !escape {
        return text.as_bytes().to_vec();
    }
    let mut out = Vec::with_capacity(text.len());
    push_escaped(&mut out, text);
    out
}

/// `serde_json` 默认字符串转义：`"`/`\`/控制字符，非 ASCII 原样。
fn push_escaped(out: &mut Vec<u8>, text: &str) {
    for ch in text.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{0c}' => out.extend_from_slice(b"\\f"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
}

/// 字符串内容是否**无损**可拆：严格解析成 JSON 后必须仍是容器。
///
/// 不做「解析→宽松回写→猜回原件」：拆分的候选展开必须与目标字节逐字节相等，否则退回整段作叶。
fn decomposable_json_text(text: &str) -> Option<Value> {
    let parsed = v2::strict_json(text.as_bytes()).ok()?;
    match parsed {
        Value::Object(_) | Value::Array(_) => Some(parsed),
        _ => None,
    }
}

fn node_id(kind: i64, payload: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update([kind as u8]);
    h.update(payload);
    h.finalize().into()
}

/// 拼接节点的一个分量（解析结果，字面量只留字节切片）。
enum Part<'a> {
    Lit(&'a [u8]),
    Ref([u8; 32]),
    Slot(u16),
}

/// 按标签逐项解析拼接节点负载：语法未声明、截断、越界一律拒绝（不猜、不忽略余量）。
fn split_seq(payload: &[u8]) -> Result<Vec<Part<'_>>, String> {
    split_seq_work(payload, None)?
        .try_into_plain()
        .map_err(|e| e.to_string())
}
fn split_seq_work<'a>(
    payload: &'a [u8],
    owner: Option<&OwnedBudget>,
) -> Result<CodecVec<Part<'a>>, String> {
    split_seq_work_admitted(payload, owner, None)
}
fn split_seq_work_admitted<'a>(
    payload: &'a [u8],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<Part<'a>>, String> {
    let admission = NativeAdmission(work);
    let mut parts = new_work(owner);
    let mut i = 0usize;
    while i < payload.len() {
        admission
            .native_codec_room(&mut parts, 1)
            .map_err(|e| e.to_string())?;
        match payload[i] {
            TAG_LIT => {
                if i + 5 > payload.len() {
                    return Err("StorageUnavailable：拼接节点字面量长度截断".into());
                }
                let len = u32::from_be_bytes(
                    payload[i + 1..i + 5]
                        .try_into()
                        .map_err(|_| "StorageUnavailable：字面量长度解码失败")?,
                ) as usize;
                let start = i + 5;
                let end = start + len;
                if end > payload.len() {
                    return Err("StorageUnavailable：拼接节点字面量越界".into());
                }
                parts
                    .try_push(Part::Lit(&payload[start..end]))
                    .map_err(|e| e.to_string())?;
                i = end;
            }
            TAG_REF => {
                if i + 33 > payload.len() {
                    return Err("StorageUnavailable：拼接节点引用截断".into());
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(&payload[i + 1..i + 33]);
                parts.try_push(Part::Ref(id)).map_err(|e| e.to_string())?;
                i += 33;
            }
            TAG_SLOT => {
                if i + 3 > payload.len() {
                    return Err("StorageUnavailable：拼接节点参数槽截断".into());
                }
                parts
                    .try_push(Part::Slot(u16::from_be_bytes([
                        payload[i + 1],
                        payload[i + 2],
                    ])))
                    .map_err(|e| e.to_string())?;
                i += 3;
            }
            other => {
                return Err(format!(
                    "StorageUnavailable：拼接节点分量标签 `{other}` 未声明"
                ))
            }
        }
    }
    Ok(parts)
}

fn decode_into(kind: i64, payload: &[u8], stack: &mut CodecVec<Frame>) -> Result<(), String> {
    decode_into_admitted(kind, payload, stack, None)
}
fn decode_into_admitted(
    kind: i64,
    payload: &[u8],
    stack: &mut CodecVec<Frame>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let admission = NativeAdmission(work);
    let owner = stack.allocation_budget().cloned();
    if kind == KIND_LEAF {
        admission
            .native_codec_room(stack, 1)
            .map_err(|e| e.to_string())?;
        stack
            .try_push(Frame::Lit(work_copy_admitted(
                payload,
                owner.as_ref(),
                work,
            )?))
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let mut frames = new_work(owner.as_ref());
    for part in split_seq_work_admitted(payload, owner.as_ref(), work)? {
        let frame = match part {
            Part::Lit(bytes) => Frame::Lit(work_copy_admitted(bytes, owner.as_ref(), work)?),
            Part::Ref(id) => Frame::Open(id),
            Part::Slot(idx) => Frame::Slot(idx),
        };
        admission
            .native_codec_room(&mut frames, 1)
            .map_err(|e| e.to_string())?;
        frames.try_push(frame).map_err(|e| e.to_string())?;
    }
    for f in frames.into_iter().rev() {
        admission
            .native_codec_room(stack, 1)
            .map_err(|e| e.to_string())?;
        stack.try_push(f).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 完整 staging：把已算好的完整规范 Value 换成图，并在写入前用自身展开与同一份规范字节对拍。
pub(super) fn stage_complete(value: &Value, slots: &[SlotSpec]) -> Result<Staged, String> {
    let canonical = canonical_bytes_of_value(value);
    let mut stager = Stager::new(slots)?;
    let root = stager.intern(value, false)?;
    if let Some(id) = stager.missing_ref {
        return Err(format!(
            "SchemaUnsupported：私有证据图 staging 引用了本代未生成的节点 {}（实现缺陷，拒绝写入）",
            hex(&id)
        ));
    }
    let expanded = stager.expand_staged(&root)?;
    if expanded != canonical {
        return Err(
            "SchemaUnsupported：私有证据图的展开字节与同一份规范字节不符（无损性自核对失败，拒绝写入）"
                .into(),
        );
    }
    let mut slot_ids = Vec::with_capacity(stager.params.len());
    for i in 0..stager.params.len() {
        let bytes = stager.params[i].clone();
        let id = node_id(KIND_LEAF, &bytes);
        stager.nodes.entry(id).or_insert(Node {
            kind: KIND_LEAF,
            payload: bytes,
            expanded_length: 0,
        });
        // 参数叶的展开长度必须与其字节一致（上面 or_insert 分支补正）。
        if let Some(n) = stager.nodes.get_mut(&id) {
            n.expanded_length = n.payload.len() as u64;
        }
        slot_ids.push(id);
    }
    let root_id = match root {
        Child::Ref(id) => id,
        _ => return Err("SchemaUnsupported：根必须是节点引用".into()),
    };
    let nodes: Vec<([u8; 32], Node)> = stager.nodes.into_iter().collect();
    let staged_bytes: u64 = nodes.iter().map(|(_, n)| n.payload.len() as u64 + 40).sum();
    let staged_nodes = nodes.len();
    Ok(Staged {
        nodes,
        root: root_id,
        slot_ids,
        expanded_length: canonical.len() as u64,
        content_sha256: sha256_hex(&canonical),
        staged_nodes,
        staged_bytes,
    })
}

/// 把 staging 产物与根登记写进当前事务。
///
/// - 同 ID 已存在时逐字节比对不可变内容，不符即 `IdentityConflict`（不吞、不留待将来审计发现）。
/// - 根在同一事务登记；本 attempt 只写自己的节点闭包，不触碰其它根。
pub(super) fn publish_refs(
    tx: &Connection,
    root_key: &str,
    staged: &Staged,
) -> Result<(usize, u64), String> {
    // 写入前作废临时读集：写后重读必须重新读新鲜行，不得被写前读到的节点行掩盖。
    cache::end_capture();
    let format = read_format(tx)?;
    format.writable()?;
    let (kind, _) = root_kind(root_key)?;
    if kind != RootKind::Batch && !format.records_in_graph() {
        return Err(format!(
            "SchemaUnsupported：格式 `{}` 未声明 `{root_key}` 所在的写域，拒绝写入",
            format.tag()
        ));
    }
    let mut inserted = 0usize;
    let mut inserted_bytes = 0u64;
    {
        let mut insert = tx
            .prepare("INSERT OR IGNORE INTO s_evidence_nodes(node_id,node_kind,payload,expanded_length) VALUES(?1,?2,?3,?4)")
            .map_err(|e| format!("prepare 私有节点失败：{e}"))?;
        let mut read = tx
            .prepare(
                "SELECT node_kind,payload,expanded_length FROM s_evidence_nodes WHERE node_id=?1",
            )
            .map_err(|e| format!("prepare 私有节点读取失败：{e}"))?;
        for (id, node) in &staged.nodes {
            let changed = insert
                .execute(params![
                    &id[..],
                    node.kind,
                    node.payload,
                    i64::try_from(node.expanded_length)
                        .map_err(|_| "StorageUnavailable：节点展开长度超出 i64 域")?,
                ])
                .map_err(|e| format!("写私有节点失败：{e}"))?;
            if changed == 1 {
                inserted += 1;
                inserted_bytes += node.payload.len() as u64;
                if kind == RootKind::Batch
                    && inserted == 1
                    && staged.nodes.len() > 1
                    && std::env::var("S_SESSION_PAUSE").ok().as_deref()
                        == Some("after_evidence_node")
                {
                    // #1394：真实节点页已刷入当前未提交事务，根行尚未写入；外部按标记 PID 注入 SIGKILL。
                    tx.cache_flush()
                        .map_err(|e| format!("TestOnly：节点页刷新失败：{e}"))?;
                    testonly_pause("after_evidence_node");
                }
                continue;
            }
            // 同 ID 必须同内容：逐字节比对，不符即拒绝发布。
            let (kind, payload, length): (i64, Vec<u8>, i64) = read
                .query_row(params![&id[..]], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(|e| format!("读既有私有节点失败：{e}"))?;
            if kind != node.kind
                || payload != node.payload
                || u64::try_from(length).ok() != Some(node.expanded_length)
            {
                return Err(format!(
                    "IdentityConflict：私有节点 {} 同 ID 异内容（既有 {} 字节 / 本次 {} 字节），拒绝发布",
                    hex(id),
                    payload.len(),
                    node.payload.len()
                ));
            }
        }
    }
    let slot_blob: Vec<u8> = staged.slot_ids.concat();
    let changed = tx
        .execute(
            "INSERT OR IGNORE INTO s_evidence_roots(root_key,node_id,slot_ids,expanded_length,content_sha256) VALUES(?1,?2,?3,?4,?5)",
            params![
                root_key,
                &staged.root[..],
                slot_blob,
                i64::try_from(staged.expanded_length)
                    .map_err(|_| "StorageUnavailable：根展开长度超出 i64 域")?,
                staged.content_sha256,
            ],
        )
        .map_err(|e| format!("写私有根失败：{e}"))?;
    if changed == 0 {
        let (node_id, slots, len, hash): (Vec<u8>, Vec<u8>, i64, String) = tx
            .query_row(
                "SELECT node_id,slot_ids,expanded_length,content_sha256 FROM s_evidence_roots WHERE root_key=?1",
                params![root_key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| format!("读既有私有根失败：{e}"))?;
        if node_id != staged.root
            || slots != slot_blob
            || usize::try_from(len).ok() != Some(staged.expanded_length as usize)
            || hash != staged.content_sha256
        {
            return Err(format!(
                "IdentityConflict：私有根 `{root_key}` 同 key 异内容，拒绝覆写"
            ));
        }
    }
    let replaced = tx
        .execute(
            "DELETE FROM s_evidence_roots WHERE root_key=?1 AND node_id<>?2",
            params![root_key, &staged.root[..]],
        )
        .map_err(|e| format!("核私有根唯一性失败：{e}"))?;
    if replaced != 0 {
        return Err(format!(
            "IdentityConflict：私有根 `{root_key}` 出现同 key 多根"
        ));
    }
    Ok((inserted, inserted_bytes))
}

// ───────────────────────────── 读取：展开 ─────────────────────────────

/// 一个私有根行的完整声明（fresh 读入，不含展开）。
struct RootRow {
    node: [u8; 32],
    slots: CodecVec<[u8; 32]>,
    expanded_length: u64,
    content_sha256: TextValue,
}

/// 一次根读取的结果：展开字节、根声明长度与内容摘要、以及按需解码的规范 JSON。
pub(super) struct VerifiedRoot {
    pub(super) bytes: crate::v3::native_shared::NativeArc<[u8]>,
    pub(super) declared_length: u64,
    pub(super) content_sha256: TextValue,
    pub(super) value: Option<crate::v3::native_shared::NativeArc<crate::SharedPlainJson>>,
    /// 本次用新鲜行算出的闭包指纹（供「对原件内容敏感」的前缀/复用判据）。
    pub(super) closure: [u8; 32],
    /// 本次是否命中已核句柄（true = 未重新展开；false = 本次完整展开并核过）。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) reused: bool,
    /// 本次是否把根行与闭包内**每个唯一节点**的当前原行与保留证书**逐字节**比较过且全部相同。
    /// 这才是复用门的原件那一半；闭包指纹只作附加校验。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) originals_verified: bool,
    /// 该根当前保留的原件证书（审计前缀的原件证据；`None` = 本次无法保留）。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) root_cert: Option<crate::v3::native_shared::NativeArc<cache::RootCert>>,
}

/// 只有当前完整闭包逐字节命中已核原件才成立；没有展开字节或解码值。
pub(super) struct VerifiedRootProof {
    pub(super) declared_length: u64,
    pub(super) content_sha256: TextValue,
    pub(super) closure: [u8; 32],
    pub(super) root_cert: crate::v3::native_shared::NativeArc<cache::RootCert>,
}

/// 一次根闭包核验的结果：根行 + 派生量 + 指纹 + 原件门 + 闭包内唯一节点集。
struct ClosureCheck {
    row: RootRow,
    digest: [u8; 32],
    /// 根行与闭包内每个唯一节点都在本捕获与保留证书逐字节相同。
    originals_verified: bool,
    #[cfg(test)]
    reuse_outcome: Option<cache::ReuseOutcome>,
}

/// 闭包指纹：把**当前新鲜行**读到的根行（根指针、有序参数槽列、声明长度、内容摘要）与闭包内
/// 每个节点的 (id, kind, 声明长度) 按确定性顺序摘要。节点 id 由该节点 payload 内容寻址，且本次
/// 已用刚读到的 payload 重算过，故指纹相同即「根行 + 每个节点的 kind/payload/length/slot 信息」
/// 与此前成功核验原字节的那一次精确相等；任一变化都改变指纹、都撤销复用。
fn closure_digest(row: &RootRow, subtree: &[u8; 32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"evidence-root-closure/1");
    h.update(row.node);
    h.update((row.slots.len() as u64).to_be_bytes());
    for slot in &row.slots {
        h.update(slot);
    }
    h.update(row.expanded_length.to_be_bytes());
    h.update(row.content_sha256.as_str().as_bytes());
    h.update(subtree);
    h.finalize().into()
}

/// 子树摘要：id 已经把 payload（含子节点 id 与顺序）钉死，故摘要再收 id、种类、声明长度与子摘要。
fn node_subtree_digest(kind: i64, id: &[u8; 32], length: u64, children: &[[u8; 32]]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"evidence-node-subtree/1");
    h.update([kind as u8]);
    h.update(id);
    h.update(length.to_be_bytes());
    h.update((children.len() as u64).to_be_bytes());
    for child in children {
        h.update(child);
    }
    h.finalize().into()
}

/// Owned 读取不借普通缓存的 Arc；节点与派生量在调用方原账上实际持有。
struct OwnedNode {
    kind: i64,
    payload: CodecVec<u8>,
    expanded_length: u64,
    children: CodecVec<[u8; 32]>,
}
impl OwnedNode {
    fn try_clone(&self, b: &OwnedBudget) -> Result<Self, String> {
        Ok(Self {
            kind: self.kind,
            payload: work_copy(&self.payload, Some(b))?,
            expanded_length: self.expanded_length,
            children: work_copy(&self.children, Some(b))?,
        })
    }
}
enum ReadNode {
    Plain(crate::v3::native_shared::NativeArc<cache::Cert>),
    Owned(OwnedNode),
}
impl ReadNode {
    fn kind(&self) -> i64 {
        match self {
            Self::Plain(n) => n.kind,
            Self::Owned(n) => n.kind,
        }
    }
    fn payload(&self) -> &[u8] {
        match self {
            Self::Plain(n) => &n.payload,
            Self::Owned(n) => &n.payload,
        }
    }
    fn length(&self) -> u64 {
        match self {
            Self::Plain(n) => n.expanded_length,
            Self::Owned(n) => n.expanded_length,
        }
    }
    fn children(&self) -> &[[u8; 32]] {
        match self {
            Self::Plain(n) => &n.children,
            Self::Owned(n) => &n.children,
        }
    }
}
struct ReadDerived {
    length: u64,
    slot_counts: CodecVec<u64>,
    digest: [u8; 32],
}
impl ReadDerived {
    fn try_clone(
        &self,
        owner: Option<&OwnedBudget>,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        Ok(Self {
            length: self.length,
            slot_counts: work_copy_admitted(&self.slot_counts, owner, work)?,
            digest: self.digest,
        })
    }
}
struct OwnedReadState {
    budget: OwnedBudget,
    nodes: BusinessMap<[u8; 32], OwnedNode>,
    derived: BusinessMap<[u8; 32], ReadDerived>,
}
pub(super) struct OwnedVerifiedRoot {
    pub(super) bytes: CodecVec<u8>,
    pub(super) declared_length: u64,
    pub(super) content_sha256: TextValue,
}

/// 私有图的读取器：同一次捕获内每个唯一节点原行只读一次；跨捕获复用已核句柄，容量有限时
/// 只失去复用，不改变合法性判据。
pub(super) struct Reader<'a> {
    conn: &'a Connection,
    conn_id: usize,
    work: Option<&'a WorkScope>,
    /// 本次捕获的临时读集守卫：析构即作废本次读入的节点行。
    _capture: Option<cache::CaptureGuard>,
    owned: Option<OwnedReadState>,
    /// 本次读取实际读到过的节点 id（供「不可达残留」核对）。
    touched: crate::v3::audit_store::NativeSet<[u8; 32]>,
}

impl<'a> Reader<'a> {
    /// `budget_bytes` = 本次捕获**临时读集**的保留上限（同次捕获每个唯一节点原行只读一次所用）。
    /// 跨请求的持久复用由 `cache::configure` 的单一预算决定。
    pub(super) fn new(conn: &'a Connection, budget_bytes: usize) -> Self {
        Self::new_admitted(conn, budget_bytes, None).expect("None admission cannot fail")
    }
    pub(super) fn new_admitted(
        conn: &'a Connection,
        budget_bytes: usize,
        work: Option<&'a WorkScope>,
    ) -> Result<Self, String> {
        Ok(Reader {
            conn,
            conn_id: conn as *const Connection as usize,
            work,
            _capture: Some(cache::begin_capture_admitted(budget_bytes, work)?),
            owned: None,
            touched: Default::default(),
        })
    }

    pub(super) fn with_owner(conn: &'a Connection, budget: &OwnedBudget) -> Self {
        Self {
            conn,
            conn_id: conn as *const Connection as usize,
            work: None,
            _capture: None,
            touched: Default::default(),
            owned: Some(OwnedReadState {
                budget: budget.clone(),
                nodes: new_map(Some(budget)),
                derived: new_map(Some(budget)),
            }),
        }
    }
    pub(super) fn work(&self) -> Option<&'a WorkScope> {
        self.work
    }
    fn owner(&self) -> Option<&OwnedBudget> {
        self.owned.as_ref().map(|o| &o.budget)
    }
    fn mark_touched(&mut self, id: [u8; 32]) -> Result<(), String> {
        if self.owned.is_none() {
            self.touched
                .insert_admitted(id, &NativeAdmission(self.work))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn derived_has(&self, id: &[u8; 32]) -> bool {
        match &self.owned {
            Some(o) => o.derived.contains_key(id),
            None => cache::derived_has(self.conn_id, id),
        }
    }
    fn derived_get(&self, id: &[u8; 32]) -> Result<Option<ReadDerived>, String> {
        match &self.owned {
            Some(o) => o
                .derived
                .get(id)
                .map(|v| v.try_clone(Some(&o.budget), None))
                .transpose(),
            None => Ok(
                cache::derived_get_admitted(self.conn_id, id, self.work)?.map(|v| ReadDerived {
                    length: v.length,
                    slot_counts: v.slot_counts,
                    digest: v.digest,
                }),
            ),
        }
    }
    fn derived_put(&mut self, id: [u8; 32], v: ReadDerived) -> Result<(), String> {
        match &mut self.owned {
            Some(o) => {
                o.derived.insert(id, v).map_err(|e| e.to_string())?;
            }
            None => cache::derived_put_admitted(
                self.conn_id,
                id,
                cache::NodeDerived {
                    length: v.length,
                    slot_counts: v.slot_counts,
                    digest: v.digest,
                },
                self.work,
            )?,
        }
        Ok(())
    }
    fn load_work(&mut self, id: &[u8; 32]) -> Result<ReadNode, String> {
        let Some(state) = self.owned.as_mut() else {
            return self.load(id).map(ReadNode::Plain);
        };
        if let Some(node) = state.nodes.get(id) {
            return node.try_clone(&state.budget).map(ReadNode::Owned);
        }
        let (kind, payload, declared) =
            read_node_row_admitted(self.conn, id, Some(&state.budget), None)?;
        let children = node_children(kind, &payload, Some(&state.budget))?;
        #[cfg(test)]
        cache::counter(|c| {
            c.node_rows_read += 1;
            c.node_payload_bytes_read += payload.len();
        });
        let node = OwnedNode {
            kind,
            payload,
            expanded_length: declared,
            children,
        };
        let result = node.try_clone(&state.budget)?;
        state.nodes.insert(*id, node).map_err(|e| e.to_string())?;
        Ok(ReadNode::Owned(result))
    }
    pub(super) fn read_root_owned(&mut self, root_key: &str) -> Result<OwnedVerifiedRoot, String> {
        if self.owned.is_none() {
            return Err("prototype_audit_allocation_incomplete：私有图Owned入口缺原账".into());
        }
        let check = self.closure(root_key)?;
        let bytes = self.expand_work(root_key, &check.row)?;
        Ok(OwnedVerifiedRoot {
            bytes,
            declared_length: check.row.expanded_length,
            content_sha256: check.row.content_sha256,
        })
    }
    pub(super) fn touched(&self) -> &std::collections::BTreeSet<[u8; 32]> {
        &self.touched
    }

    /// 把捕获作用域交给调用方保管：同一次根审计的后续读取继续共享同一份节点读集。
    pub(super) fn into_capture(self) -> cache::CaptureGuard {
        self._capture.expect("ordinary Reader owns capture")
    }

    /// 本读取器的连接身份（捕获内节点/派生量的分列键）。
    pub(super) fn conn_id(&self) -> usize {
        self.conn_id
    }

    /// 读 root 行（不含展开）。
    pub(super) fn root_row(
        conn: &Connection,
        root_key: &str,
    ) -> Result<Option<([u8; 32], Vec<[u8; 32]>, u64, String)>, String> {
        Ok(read_root_row(conn, root_key, None)?.map(|row| {
            (
                row.node,
                row.slots.try_into_plain().expect("ordinary root slots"),
                row.expanded_length,
                row.content_sha256.as_str().to_owned(),
            )
        }))
    }

    /// 闭包遍历（**不展开**）：从当前新鲜行读根行与闭包内每个唯一节点原行，逐节点核内容寻址、
    /// 种类、参数叶身份与声明长度自洽，最后给出与**节点 id/种类/长度 + 根行**一一对应的闭包指纹。
    ///
    /// 指纹由当前内容计算，只作附加校验；复用另须根行、全部节点与参数槽原件逐字节匹配。
    fn closure(&mut self, root_key: &str) -> Result<ClosureCheck, String> {
        let row = read_root_row_admitted(self.conn, root_key, self.owner(), self.work)?
            .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 缺失"))?;
        let RootRow {
            node: root,
            slots,
            expanded_length: expanded_len,
            content_sha256: hash,
        } = row;
        let owner = self.owner().cloned();
        let work = self.work;
        // 声明槽未必都被模板引用：只核每个槽叶自身可寻址；实际占用的槽字节在遍历时累计。
        let mut slot_len = new_work(owner.as_ref());
        work_resize_admitted(&mut slot_len, slots.len(), 0u64, work)?;
        for id in &slots {
            let node = self.load_work(id)?;
            if node.kind() != KIND_LEAF {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 参数槽引用了非叶节点"
                ));
            }
        }
        for (i, id) in slots.iter().enumerate() {
            slot_len[i] = self.load_work(id)?.length();
        }
        // 与根无关的派生量（不含槽长度 + 子树槽出现次数 + 子树摘要）按**新鲜行**自下而上算一次，
        // 同一次捕获内跨根共享（每个唯一节点只解析一次，不再按根重复遍历整棵闭包）。
        let derived = self.derived_of(root)?;
        let mut slot_bytes = 0u64;
        for (index, count) in derived.slot_counts.iter().enumerate() {
            let bytes = slot_len.get(index).ok_or_else(|| {
                format!("StorageUnavailable：私有根 `{root_key}` 引用了未声明的参数槽 {index}")
            })?;
            slot_bytes = slot_bytes.saturating_add(bytes.saturating_mul(*count));
        }
        // 根节点长度（不含槽字节）+ 展开树里实际占用的槽字节必须等于根声明长度。
        if derived.length.saturating_add(slot_bytes) != expanded_len {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 展开长度不符（声明 {expanded_len}，闭包合计 {}）",
                derived.length.saturating_add(slot_bytes)
            ));
        }
        let row = RootRow {
            node: root,
            slots,
            expanded_length: expanded_len,
            content_sha256: hash,
        };
        let digest = closure_digest(&row, &derived.digest);
        // 原件门：根行原件（根指针 + 有序槽 + 声明长度 + 内容摘要）与闭包内**每个唯一节点**的
        // 当前原行都必须在本次捕获与保留证书逐字节相同，才允许复用展开/解析/审计。
        // 节点那一半沿**保留证书**的有序子引用认证（同次认证去重，整次捕获 O(唯一节点)）。
        #[cfg(test)]
        let mut reuse_outcome = None;
        let originals_verified = self.owned.is_none()
            && cache::root_cert_match(
                root_key,
                &row.node,
                &row.slots,
                row.expanded_length,
                row.content_sha256.as_str(),
                #[cfg(test)]
                &mut reuse_outcome,
            )
            .is_some()
            && cache::cert_is_certified(self.conn_id, root)
            && row
                .slots
                .iter()
                .all(|id| cache::cert_marked(self.conn_id, *id));
        #[cfg(test)]
        if !originals_verified && reuse_outcome.is_none() {
            reuse_outcome = Some(cache::ReuseOutcome::ClosureOriginalsIncomplete);
        }
        Ok(ClosureCheck {
            row,
            digest,
            originals_verified,
            #[cfg(test)]
            reuse_outcome,
        })
    }

    /// 一个与根无关的节点派生量：`length`（不含槽）、`slot_counts`、子树摘要。
    ///
    /// 后序迭代 + 捕获内 memo；`on_path` 只含当前路径——闭包成环一律拒绝（不得用无限遍历替代判据）。
    fn derived_of(&mut self, root: [u8; 32]) -> Result<ReadDerived, String> {
        let owner = self.owner().cloned();
        let work = self.work;
        let admission = NativeAdmission(work);
        if let Some(found) = self.derived_get(&root)? {
            // 本次捕获已按新鲜行结算过该节点：仍要记账（可达集必须覆盖它），但不重复读行。
            self.mark_touched(root)?;
            return Ok(found);
        }
        let mut on_path = new_map::<[u8; 32], ()>(owner.as_ref());
        // 本次遍历的即时工作集：即使捕获级 memo 因限额/写入缺失，也能就地重算，
        // 绝不把「限额不足」判成 StorageUnavailable。
        let mut local = new_map::<[u8; 32], ReadDerived>(owner.as_ref());
        let mut stack = new_work(owner.as_ref());
        admission
            .native_codec_room(&mut stack, 1)
            .map_err(|e| e.to_string())?;
        stack.try_push((root, false)).map_err(|e| e.to_string())?;
        while let Some((id, closing)) = stack.pop() {
            // 热路径只问存在性（不克隆派生量：`slot_counts` 是 Vec，克隆会为每个节点多一次分配）。
            if local.contains_key(&id) || self.derived_has(&id) {
                self.mark_touched(id)?;
                continue;
            }
            let node = self.load_work(&id)?;
            if node.kind() == KIND_LEAF {
                let derived = ReadDerived {
                    length: node.length(),
                    slot_counts: new_work(owner.as_ref()),
                    digest: node_subtree_digest(KIND_LEAF, &id, node.length(), &[]),
                };
                admission
                    .native_map_room(&mut local)
                    .map_err(|e| e.to_string())?;
                local
                    .insert(id, derived.try_clone(owner.as_ref(), work)?)
                    .map_err(|e| e.to_string())?;
                self.derived_put(id, derived)?;
                // 叶子没有子依赖：本次逐字节命中即可认证（认证搭在既有后序遍历上，零额外遍历）。
                if self.owned.is_none() && cache::cert_marked(self.conn_id, id) {
                    cache::cert_certify_admitted(self.conn_id, id, self.work)?;
                }
                continue;
            }
            let parts = split_seq_work_admitted(node.payload(), owner.as_ref(), work)?;
            if !closing {
                admission
                    .native_map_room(&mut on_path)
                    .map_err(|e| e.to_string())?;
                if on_path.insert(id, ()).map_err(|e| e.to_string())?.is_some() {
                    return Err(format!(
                        "StorageUnavailable：私有图闭包出现环（节点 {} 自引用）",
                        hex(&id)
                    ));
                }
                admission
                    .native_codec_room(&mut stack, 1)
                    .map_err(|e| e.to_string())?;
                stack.try_push((id, true)).map_err(|e| e.to_string())?;
                for child in node.children().iter().rev() {
                    admission
                        .native_codec_room(&mut stack, 1)
                        .map_err(|e| e.to_string())?;
                    stack.try_push((*child, false)).map_err(|e| e.to_string())?;
                }
                continue;
            }
            // 子节点派生量必须已结算；被逐出/未结算时重新下沉重算，不误判为存储损坏。
            let mut missing: Option<[u8; 32]> = None;
            for part in &parts {
                if let Part::Ref(child) = part {
                    if !local.contains_key(child) && !self.derived_has(child) {
                        missing = Some(*child);
                        break;
                    }
                }
            }
            if let Some(child) = missing {
                admission
                    .native_codec_room(&mut stack, 1)
                    .map_err(|e| e.to_string())?;
                stack.try_push((id, true)).map_err(|e| e.to_string())?;
                admission
                    .native_codec_room(&mut stack, 1)
                    .map_err(|e| e.to_string())?;
                stack.try_push((child, false)).map_err(|e| e.to_string())?;
                continue;
            }
            let mut length = 0u64;
            let mut counts = new_work::<u64>(owner.as_ref());
            let mut children = new_work(owner.as_ref());
            for part in parts {
                match part {
                    Part::Lit(bytes) => length = length.saturating_add(bytes.len() as u64),
                    Part::Slot(index) => {
                        let index = index as usize;
                        if counts.len() <= index {
                            work_resize_admitted(&mut counts, index + 1, 0, work)?;
                        }
                        counts[index] = counts[index].saturating_add(1);
                    }
                    Part::Ref(child) => {
                        let entry = match local.get(&child) {
                            Some(v) => Some(v.try_clone(owner.as_ref(), work)?),
                            None => self.derived_get(&child)?,
                        }
                        .ok_or_else(|| {
                            format!(
                                "StorageUnavailable：私有节点 {} 的子节点 {} 未在闭包内结算",
                                hex(&id),
                                hex(&child)
                            )
                        })?;
                        length = length.saturating_add(entry.length);
                        if counts.len() < entry.slot_counts.len() {
                            work_resize_admitted(&mut counts, entry.slot_counts.len(), 0, work)?;
                        }
                        for (index, count) in entry.slot_counts.iter().enumerate() {
                            counts[index] = counts[index].saturating_add(*count);
                        }
                        admission
                            .native_codec_room(&mut children, 1)
                            .map_err(|e| e.to_string())?;
                        children.try_push(child).map_err(|e| e.to_string())?;
                    }
                }
            }
            if length != node.length() {
                return Err(format!(
                    "StorageUnavailable：私有节点 {} 声明长度 {} 与实际展开 {length} 不符",
                    hex(&id),
                    node.length()
                ));
            }
            on_path.remove(&id);
            let derived = ReadDerived {
                length,
                slot_counts: counts,
                digest: node_subtree_digest(KIND_SEQ, &id, length, &children),
            };
            admission
                .native_map_room(&mut local)
                .map_err(|e| e.to_string())?;
            local
                .insert(id, derived.try_clone(owner.as_ref(), work)?)
                .map_err(|e| e.to_string())?;
            self.derived_put(id, derived)?;
            // 子节点（本帧的后序先决条件）已全部结算：只要每个直接子节点都已认证，本节点也认证。
            if self.owned.is_none()
                && cache::cert_marked(self.conn_id, id)
                && children
                    .iter()
                    .all(|child| cache::cert_is_certified(self.conn_id, *child))
            {
                cache::cert_certify_admitted(self.conn_id, id, self.work)?;
            }
        }
        match local.get(&root) {
            Some(v) => Some(v.try_clone(owner.as_ref(), work)?),
            None => self.derived_get(&root)?,
        }
        .ok_or_else(|| "StorageUnavailable：私有节点派生量缺失（实现缺陷）".to_string())
    }

    /// 只证明当前完整原件；缺缓存返回 None，闭包错误原样传播，不替换全文读取出口。
    pub(super) fn prove_root(
        &mut self,
        root_key: &str,
    ) -> Result<Option<VerifiedRootProof>, String> {
        #[cfg(test)]
        cache::counter(|c| c.root_proof_reads += 1);
        let check = self.closure(root_key).map_err(|error| {
            #[cfg(test)]
            cache::counter(|c| c.root_proof_errors += 1);
            error
        })?;
        if !check.originals_verified {
            cache::invalidate_root(root_key);
            #[cfg(test)]
            cache::counter(|c| c.root_proof_misses += 1);
            return Ok(None);
        }
        let Some(root_cert) = cache::root_cert_get(root_key) else {
            #[cfg(test)]
            cache::counter(|c| c.root_proof_misses += 1);
            return Ok(None);
        };
        #[cfg(test)]
        cache::counter(|c| c.root_proof_hits += 1);
        Ok(Some(VerifiedRootProof {
            declared_length: check.row.expanded_length,
            content_sha256: NativeAdmission(self.work)
                .native_text(check.row.content_sha256.as_str())
                .map_err(|e| e.to_string())?,
            closure: check.digest,
            root_cert,
        }))
    }

    /// 读一个根：完整原件证明与附加指纹均命中，才复用已验展开字节/解析结果；未命中则
    /// 完整展开并核公开长度与摘要后入句柄。两条路径的返回必须逐字节相同。
    pub(super) fn read_root(
        &mut self,
        root_key: &str,
        with_value: bool,
    ) -> Result<VerifiedRoot, String> {
        let admission = NativeAdmission(self.work);
        let check = self.closure(root_key).map_err(|error| {
            #[cfg(test)]
            cache::counter(|c| c.reuse_gate_not_reached += 1);
            error
        })?;
        #[cfg(test)]
        let mut reuse_outcome = check.reuse_outcome;
        if !check.originals_verified {
            cache::invalidate_root(root_key);
        }
        // 复用门：闭包指纹只是**附加**校验；真正的门是 (a) 该根保留着的原件证书与本次新鲜根行
        // 逐字节相同、(b) 闭包内每个唯一节点都在本次捕获与保留证书逐字节相同。
        let mut retained = if check.originals_verified {
            cache::root_cert_get(root_key)
        } else {
            None
        };
        let cached = retained.as_ref().and_then(|rc| {
            cache::handle_get_certified(
                root_key,
                rc,
                &check.digest,
                #[cfg(test)]
                &mut reuse_outcome,
            )
        });
        let reused = cached.is_some();
        #[cfg(test)]
        cache::counter(|c| {
            c.root_reads += 1;
            c.record_reuse(
                root_key,
                reuse_outcome.expect("root reuse gate must record one outcome"),
            );
            if reused {
                c.root_handle_hits += 1;
            } else {
                c.root_expansions += 1;
            }
        });
        let (bytes, mut value) = match cached {
            Some((bytes, value)) => (bytes, value),
            None => {
                let expanded = self.expand_work(root_key, &check.row)?;
                let bytes =
                    crate::v3::native_shared::NativeArc::try_copy_slice(&expanded, self.work)
                        .map_err(|e| e.to_string())?;
                if cache::retention_enabled() {
                    // 已核闭包：把本次读到的**原件证书**保留下来（共享同一份原节点，按唯一保留量
                    // 记账），供后续捕获逐字节比较。只在成功核过长度/摘要之后才保留。
                    let mut ids = crate::v3::audit_store::NativeSet::default();
                    let mut frontier = newchan_rust::s_content::CodecVec::default();
                    admission
                        .native_vec_room(&mut frontier, check.row.slots.len() + 1)
                        .map_err(|e| e.to_string())?;
                    for id in &check.row.slots {
                        frontier.try_push(*id).map_err(|e| e.to_string())?;
                    }
                    frontier
                        .try_push(check.row.node)
                        .map_err(|e| e.to_string())?;
                    while let Some(id) = frontier.pop() {
                        if !ids
                            .insert_admitted(id, &admission)
                            .map_err(|e| e.to_string())?
                        {
                            continue;
                        }
                        // 命中本捕获读集时不再读 SQLite；读集被逐出时按新鲜行重读。
                        let node = self.load(&id)?;
                        admission
                            .native_vec_room(&mut frontier, node.children.len())
                            .map_err(|e| e.to_string())?;
                        for id in &node.children {
                            frontier.try_push(*id).map_err(|e| e.to_string())?;
                        }
                    }
                    cache::cert_commit_admitted(self.conn_id, &ids, self.work)?;
                    let mut slots = CodecVec::default();
                    admission
                        .native_vec_room(&mut slots, check.row.slots.len())
                        .map_err(|e| e.to_string())?;
                    for slot in &check.row.slots {
                        slots.try_push(*slot).map_err(|e| e.to_string())?;
                    }
                    let cert = cache::RootCert::new_admitted(
                        check.row.node,
                        slots,
                        check.row.expanded_length,
                        admission
                            .native_string(check.row.content_sha256.as_str())
                            .map_err(|e| e.to_string())?,
                    );
                    retained = if cache::certify_retained_originals_admitted(
                        self.conn_id,
                        &ids,
                        self.work,
                    )? {
                        cache::handle_put_certified_admitted(
                            root_key,
                            &cert,
                            check.digest,
                            bytes.clone(),
                            None,
                            self.work,
                        )?
                    } else {
                        #[cfg(test)]
                        cache::counter(|c| c.root_proof_rebuild_incomplete += 1);
                        None
                    };
                }
                (bytes, None)
            }
        };
        if with_value && value.is_none() {
            let parsed =
                crate::v3::business_json::strict_json_owner_admitted(&bytes, None, self.work)
                    .map(|parsed| match parsed {
                        crate::v3::business_json::JsonOwner::Plain(value) => value,
                        #[cfg(feature = "s_allocation_json_testonly")]
                        crate::v3::business_json::JsonOwner::Owned(_) => {
                            unreachable!("Native Reader uses plain JSON")
                        }
                    })
                    .map_err(|e| match e {
                        crate::v3::business_json::StrictOwnerError::Syntax(cause) => {
                            format!("StorageUnavailable：私有根 `{root_key}` 非严格 JSON：{cause}")
                        }
                        #[cfg(feature = "s_allocation_json_testonly")]
                        crate::v3::business_json::StrictOwnerError::OwnedSyntax(cause) => {
                            format!("StorageUnavailable：私有根 `{root_key}` 非严格 JSON：{cause}")
                        }
                        crate::v3::business_json::StrictOwnerError::Allocation(cause) => {
                            cause.to_string()
                        }
                    })?;
            if !parsed.is_object() {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 非 JSON 对象"
                ));
            }
            let parsed = crate::share_plain_json_admitted(parsed, self.work)?;
            if let Some(cert) = retained.clone() {
                retained = cache::handle_put_certified_admitted(
                    root_key,
                    &cert,
                    check.digest,
                    bytes.clone(),
                    Some(parsed.clone()),
                    self.work,
                )?;
            }
            value = Some(parsed);
            #[cfg(test)]
            cache::counter(|c| c.root_parses += 1);
        }
        Ok(VerifiedRoot {
            bytes,
            declared_length: check.row.expanded_length,
            content_sha256: NativeAdmission(self.work)
                .native_text(check.row.content_sha256.as_str())
                .map_err(|e| e.to_string())?,
            value,
            closure: check.digest,
            reused,
            originals_verified: check.originals_verified,
            root_cert: retained,
        })
    }

    /// 读一个根的展开字节。
    pub(super) fn read_fixed_cut(
        &mut self,
        root_key: &str,
    ) -> Result<crate::v3::native_shared::NativeArc<[u8]>, String> {
        Ok(self.read_root(root_key, false)?.bytes)
    }

    /// 读一个根并解码为规范 JSON 对象（同一句柄缓存，含展开字节）。
    pub(super) fn read_fixed_cut_value(
        &mut self,
        root_key: &str,
    ) -> Result<
        (
            crate::v3::native_shared::NativeArc<[u8]>,
            crate::v3::native_shared::NativeArc<crate::SharedPlainJson>,
        ),
        String,
    > {
        let root = self.read_root(root_key, true)?;
        let value = root
            .value
            .ok_or("StorageUnavailable：私有根解码结果缺失（实现缺陷）")?;
        Ok((root.bytes, value))
    }

    /// 完整展开（冷路径）：返回前核根声明的长度与摘要，长度增长受根声明上界约束。
    fn expand(&mut self, root_key: &str, row: &RootRow) -> Result<Vec<u8>, String> {
        self.expand_work(root_key, row)?
            .try_into_plain()
            .map_err(|e| e.to_string())
    }
    fn expand_work(&mut self, root_key: &str, row: &RootRow) -> Result<CodecVec<u8>, String> {
        let owner = self.owner().cloned();
        let work = self.work;
        let admission = NativeAdmission(work);
        let expanded_len = row.expanded_length;
        let hash = row.content_sha256.as_str();
        let root = row.node;
        let mut param_bytes = new_work(owner.as_ref());
        admission
            .native_codec_room(&mut param_bytes, row.slots.len())
            .map_err(|e| e.to_string())?;
        param_bytes
            .try_reserve_exact(row.slots.len())
            .map_err(|e| e.to_string())?;
        for id in &row.slots {
            let node = self.load_work(id)?;
            if node.kind() != KIND_LEAF {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 参数槽引用了非叶节点"
                ));
            }
            param_bytes
                .try_push(work_copy_admitted(node.payload(), owner.as_ref(), work)?)
                .map_err(|e| e.to_string())?;
        }
        // 声明长度来自数据库，不能用它直接请求任意大的分配；实际输出逐段增长。
        let mut out = new_work(owner.as_ref());
        let mut stack = new_work(owner.as_ref());
        admission
            .native_codec_room(&mut stack, 1)
            .map_err(|e| e.to_string())?;
        stack
            .try_push(Frame::Open(root))
            .map_err(|e| e.to_string())?;
        let mut slot_bytes = 0u64;
        // #1394：仅在固定事务内复用本次已完成的展开区间；自动提交仍逐次 Open 读新鲜行。
        // 只记 out 的区间和槽字节增量，不持有原件/证书；参数固定，out 只追加，区间始终有效。
        let mut completed = (!self.conn.is_autocommit())
            .then(|| new_map::<[u8; 32], (usize, usize, u64)>(owner.as_ref()));
        while let Some(item) = stack.pop() {
            match item {
                Frame::Lit(bytes) => append_checked_admitted(&mut out, &bytes, expanded_len, work)?,
                Frame::Slot(i) => {
                    let bytes = param_bytes.get(i as usize).ok_or_else(|| {
                        format!("StorageUnavailable：私有根 `{root_key}` 参数槽 {i} 越界")
                    })?;
                    append_checked_admitted(&mut out, bytes, expanded_len, work)?;
                    slot_bytes += bytes.len() as u64;
                }
                Frame::Open(id) => {
                    if let Some((start, end, slots)) =
                        completed.as_ref().and_then(|spans| spans.get(&id)).copied()
                    {
                        let len = end - start;
                        let next = out
                            .len()
                            .checked_add(len)
                            .ok_or("StorageUnavailable：证据图展开长度溢出")?;
                        if next as u64 > expanded_len {
                            return Err("StorageUnavailable：证据图实际展开超过根声明长度".into());
                        }
                        admission
                            .native_codec_room(&mut out, len)
                            .map_err(|e| e.to_string())?;
                        out.try_reserve_exact(len).map_err(|e| e.to_string())?;
                        for offset in start..end {
                            let byte = out[offset];
                            out.try_push(byte).map_err(|e| e.to_string())?;
                        }
                        slot_bytes += slots;
                        continue;
                    }
                    let node = self.load_work(&id)?;
                    admission
                        .native_codec_room(&mut stack, 1)
                        .map_err(|e| e.to_string())?;
                    stack
                        .try_push(Frame::Close {
                            id,
                            declared_length: node.length(),
                            start: out.len(),
                            slot_bytes_at_open: slot_bytes,
                        })
                        .map_err(|e| e.to_string())?;
                    decode_into_admitted(node.kind(), node.payload(), &mut stack, work)?;
                }
                Frame::Close {
                    id,
                    declared_length,
                    start,
                    slot_bytes_at_open,
                } => {
                    let actual = (out.len() - start) as u64;
                    let slots = slot_bytes - slot_bytes_at_open;
                    let without_slots = actual - slots;
                    if without_slots != declared_length {
                        return Err(format!(
                            "StorageUnavailable：私有节点 {} 声明长度 {} 与实际展开 {without_slots} 不符",
                            hex(&id),
                            declared_length
                        ));
                    }
                    if let Some(spans) = completed.as_mut() {
                        if !spans.contains_key(&id) {
                            admission
                                .native_map_room(spans)
                                .map_err(|e| e.to_string())?;
                        }
                        spans
                            .insert(id, (start, out.len(), slots))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        if out.len() as u64 != expanded_len {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 展开长度不符（声明 {expanded_len}，实际 {}）",
                out.len()
            ));
        }
        if !hash_matches(&out, hash) {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 展开字节与登记摘要不符（内容被改）"
            ));
        }
        Ok(out)
    }

    /// 按 id 取节点**原件证书**：本次捕获已读过就直接复用；否则从**新鲜行**读入，先核内容寻址与
    /// 长度自洽，再与保留证书**逐字节**比较（命中才打上原件门标记），最后记入本次捕获的临时读集
    /// （超预算只失去复用，当次读照常返回）。
    fn load(
        &mut self,
        id: &[u8; 32],
    ) -> Result<crate::v3::native_shared::NativeArc<cache::Cert>, String> {
        let admission = NativeAdmission(self.work);
        self.mark_touched(*id)?;
        cache::mark_reachable_admitted(self.conn_id, *id, self.work)?;
        if let Some(node) = cache::node_get(self.conn_id, id) {
            #[cfg(test)]
            cache::counter(|c| c.node_reused_in_capture += 1);
            return Ok(node);
        }
        let (kind, payload, declared) = if self.work.is_some() {
            let (kind, payload, declared) = read_node_row_admitted(self.conn, id, None, self.work)?;
            (kind, payload, declared)
        } else {
            // None 保留普通读取适配器作为独立对照；Native 借 SQL 原件后才许可复制。
            let mut statement = self.conn.prepare_cached("SELECT node_kind,payload,expanded_length FROM s_evidence_nodes WHERE node_id=?1")
                .map_err(|e| format!("StorageUnavailable：读私有节点失败：{e}"))?;
            let (kind, payload, len): (i64, Vec<u8>, i64) = statement
                .query_row(params![&id[..]], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .optional()
                .map_err(|e| format!("StorageUnavailable：读私有节点失败：{e}"))?
                .ok_or_else(|| format!("StorageUnavailable：私有节点 {} 缺失", hex(id)))?;
            drop(statement);
            let declared = verify_node_row(id, kind, &payload, len)?;
            (kind, CodecVec::from_plain(payload), declared)
        };
        let children = node_children_admitted(kind, &payload, None, self.work)?;
        #[cfg(test)]
        cache::counter(|c| {
            c.node_rows_read += 1;
            c.node_payload_bytes_read += payload.len();
        });
        // 精确原件门：与保留证书**逐字节**比较（不是比较摘要）。命中才允许复用展开/解析/审计；
        // 没有证书或原件不符时只失去复用——判据仍由各读取路径自己的完整核验承担。
        let (cert, owned) = match cache::cert_get(id) {
            Some(retained) if retained.same_originals(kind, &payload, declared) => {
                #[cfg(test)]
                cache::counter(|c| c.cert_compared += 1);
                cache::cert_matched_mark_admitted(self.conn_id, *id, self.work)?;
                (retained, false)
            }
            candidate => {
                let _ = &candidate;
                #[cfg(test)]
                cache::counter(|c| {
                    if candidate.is_some() {
                        c.cert_compared += 1;
                        c.cert_mismatched += 1;
                    }
                });
                let payload =
                    crate::v3::native_shared::NativeArc::try_copy_slice(&payload, self.work)
                        .map_err(|e| e.to_string())?;
                (
                    crate::v3::native_shared::NativeArc::try_new_shallow(
                        cache::Cert {
                            id: *id,
                            kind,
                            payload,
                            expanded_length: declared,
                            children,
                        },
                        self.work,
                    )
                    .map_err(|e| e.to_string())?,
                    true,
                )
            }
        };
        cache::node_put_admitted(self.conn_id, cert.clone(), owned, self.work)?;
        Ok(cert)
    }
}

fn read_node_row_admitted(
    conn: &Connection,
    id: &[u8; 32],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(i64, CodecVec<u8>, u64), String> {
    let err = |e| {
        format!(
            "StorageUnavailable：读私有节点失败：{}",
            sql_columns(e, &["node_kind", "payload", "expanded_length"])
        )
    };
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT node_kind,payload,expanded_length FROM s_evidence_nodes WHERE node_id=?1",
    )
    .map_err(err)?;
    stmt.bind_blob(1, id).map_err(err)?;
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| format!("StorageUnavailable：私有节点 {} 缺失", hex(id)))?;
    let kind = row.integer(0).map_err(err)?;
    let payload = work_copy_admitted(row.blob(1).map_err(err)?, owner, work)?;
    let len = row.integer(2).map_err(err)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    let declared = verify_node_row(id, kind, &payload, len)?;
    Ok((kind, payload, declared))
}

fn verify_node_row(id: &[u8; 32], kind: i64, payload: &[u8], len: i64) -> Result<u64, String> {
    if kind != KIND_LEAF && kind != KIND_SEQ {
        return Err(format!(
            "StorageUnavailable：私有节点 {} 种类 `{kind}` 未声明",
            hex(id)
        ));
    }
    if node_id(kind, payload) != *id {
        return Err(format!(
            "StorageUnavailable：私有节点 {} 同 ID 异内容（内容寻址不符）",
            hex(id)
        ));
    }
    let declared = u64::try_from(len)
        .map_err(|_| format!("StorageUnavailable：私有节点 {} 长度非负越界", hex(id)))?;
    if kind == KIND_LEAF {
        if declared != payload.len() as u64 {
            return Err(format!(
                "StorageUnavailable：私有叶 {} 声明长度与实际不符",
                hex(id)
            ));
        }
    }
    Ok(declared)
}
fn node_children(
    kind: i64,
    payload: &[u8],
    owner: Option<&OwnedBudget>,
) -> Result<CodecVec<[u8; 32]>, String> {
    node_children_admitted(kind, payload, owner, None)
}
fn node_children_admitted(
    kind: i64,
    payload: &[u8],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<[u8; 32]>, String> {
    let admission = NativeAdmission(work);
    let mut children = new_work(owner);
    if kind != KIND_LEAF {
        for part in split_seq_work_admitted(payload, owner, work)? {
            if let Part::Ref(child) = part {
                admission
                    .native_codec_room(&mut children, 1)
                    .map_err(|e| e.to_string())?;
                children.try_push(child).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(children)
}
fn read_root_row(
    conn: &Connection,
    root_key: &str,
    owner: Option<&OwnedBudget>,
) -> Result<Option<RootRow>, String> {
    read_root_row_admitted(conn, root_key, owner, None)
}
fn read_root_row_admitted(
    conn: &Connection,
    root_key: &str,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<Option<RootRow>, String> {
    let admission = NativeAdmission(work);
    let err = |e| {
        format!(
            "StorageUnavailable：读私有根失败：{}",
            sql_columns(
                e,
                &["node_id", "slot_ids", "expanded_length", "content_sha256"]
            )
        )
    };
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT node_id,slot_ids,expanded_length,content_sha256 FROM s_evidence_roots WHERE root_key=?1").map_err(err)?;
    stmt.bind_text(1, root_key).map_err(err)?;
    let Some(mut row) = stmt.next().map_err(err)? else {
        stmt.finalize().map_err(err)?;
        return Ok(None);
    };
    let node = work_copy_admitted(row.blob(0).map_err(err)?, owner, work)?;
    let slots = work_copy_admitted(row.blob(1).map_err(err)?, owner, work)?;
    let len = row.integer(2).map_err(err)?;
    let hash = TextValue::from_str_admitted(row.text(3).map_err(err)?, owner, work)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    let root = decode_id(&node)
        .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 节点 id 非 32 字节"))?;
    if slots.len() % 32 != 0 {
        return Err(format!(
            "StorageUnavailable：私有根 `{root_key}` 参数槽列长度非 32 倍数"
        ));
    }
    let mut params = new_work(owner);
    admission
        .native_codec_room(&mut params, slots.len() / 32)
        .map_err(|e| e.to_string())?;
    params
        .try_reserve_exact(slots.len() / 32)
        .map_err(|e| e.to_string())?;
    for chunk in slots.chunks(32) {
        let mut id = [0u8; 32];
        id.copy_from_slice(chunk);
        params.try_push(id).map_err(|e| e.to_string())?;
    }
    let expanded = u64::try_from(len)
        .map_err(|_| format!("StorageUnavailable：私有根 `{root_key}` 长度非负越界"))?;
    if !is_hex64(hash.as_str()) {
        return Err(format!(
            "StorageUnavailable：私有根 `{root_key}` 内容摘要非 64 位十六进制"
        ));
    }
    Ok(Some(RootRow {
        node: root,
        slots: params,
        expanded_length: expanded,
        content_sha256: hash,
    }))
}

fn append_checked(out: &mut CodecVec<u8>, bytes: &[u8], declared: u64) -> Result<(), String> {
    append_checked_admitted(out, bytes, declared, None)
}
fn append_checked_admitted(
    out: &mut CodecVec<u8>,
    bytes: &[u8],
    declared: u64,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let admission = NativeAdmission(work);
    let next = out
        .len()
        .checked_add(bytes.len())
        .ok_or("StorageUnavailable：证据图展开长度溢出")?;
    if next as u64 > declared {
        return Err("StorageUnavailable：证据图实际展开超过根声明长度".into());
    }
    admission
        .native_codec_room(out, bytes.len())
        .map_err(|e| e.to_string())?;
    out.try_reserve_exact(bytes.len())
        .map_err(|e| e.to_string())?;
    out.try_extend_bytes(bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// 展开栈帧：`Open` 进入一个节点、`Close` 收束并核该节点的声明长度（DFS 输出连续）。
enum Frame {
    Open([u8; 32]),
    Close {
        id: [u8; 32],
        /// 来自本次 Open 已核节点的标量；不延长节点/负载持有，不再为同一帧读第二遍。
        declared_length: u64,
        start: usize,
        /// 打开该节点时已消耗的槽字节数：节点声明长度不含槽字节，故核对时要把它减掉。
        slot_bytes_at_open: u64,
    },
    Lit(CodecVec<u8>),
    Slot(u16),
}

fn hex(id: &[u8; 32]) -> String {
    hex_bytes(id)
}

pub(super) fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_id(bytes: &[u8]) -> Option<[u8; 32]> {
    let arr: [u8; 32] = bytes.try_into().ok()?;
    Some(arr)
}

fn is_hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
/// 测试用：把某个根**整体替换**成新内容（绕过同 key 异内容检查），用于模拟「发布方把错误的
/// Delta/事实原件写进私有图」。替换后立即清理不再可达的节点，保持图的不变式不变。
pub(super) fn force_replace_root(
    conn: &Connection,
    root_key: &str,
    value: &Value,
) -> Result<(), String> {
    let staged = stage_complete(value, &[])?;
    {
        let mut insert = conn
            .prepare("INSERT OR IGNORE INTO s_evidence_nodes(node_id,node_kind,payload,expanded_length) VALUES(?1,?2,?3,?4)")
            .map_err(|e| e.to_string())?;
        for (id, node) in &staged.nodes {
            insert
                .execute(params![
                    &id[..],
                    node.kind,
                    node.payload,
                    i64::try_from(node.expanded_length).map_err(|_| "节点长度越界")?,
                ])
                .map_err(|e| e.to_string())?;
        }
    }
    conn.execute(
        "INSERT OR REPLACE INTO s_evidence_roots(root_key,node_id,slot_ids,expanded_length,content_sha256) VALUES(?1,?2,?3,?4,?5)",
        params![
            root_key,
            &staged.root[..],
            staged.slot_ids.concat(),
            i64::try_from(staged.expanded_length).map_err(|_| "根长度越界")?,
            staged.content_sha256,
        ],
    )
    .map_err(|e| e.to_string())?;
    prune_unreachable_nodes(conn)?;
    Ok(())
}

#[cfg(test)]
/// 测试用：删除不在任何根闭包内的节点（替换根之后清掉旧闭包残留）。
pub(super) fn prune_unreachable_nodes(conn: &Connection) -> Result<usize, String> {
    let mut reader = Reader::new(conn, 0);
    for key in root_keys(conn)? {
        reader.closure(&key)?;
    }
    let touched = reader.touched().clone();
    let orphan = unreachable_nodes(conn, &touched)?;
    let mut deleted = 0usize;
    for id in &orphan {
        conn.execute(
            "DELETE FROM s_evidence_nodes WHERE node_id=?1",
            params![&id[..]],
        )
        .map_err(|e| format!("删除不可达节点失败：{e}"))?;
        deleted += 1;
    }
    Ok(deleted)
}

/// 库内全部节点必须都被本次捕获的根闭包覆盖（不可达残留不得发布）。
///
/// 可达到的节点集是节点表 id 的子集，且每个可达节点都由 `Reader::load` 按 id 新鲜读入并核过
/// 内容寻址；故「表内行数 == 本次捕获派生量条数」即两边集合相等，无需再扫一遍全表。
/// 只在不等时做一次全表扫描，指出首个不可达节点。
pub(super) fn ensure_all_nodes_reachable(
    conn: &Connection,
    reachable: usize,
) -> Result<(), String> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM s_evidence_nodes", [], |r| r.get(0))
        .map_err(|e| format!("StorageUnavailable：核私有节点表失败：{e}"))?;
    if usize::try_from(total).ok() == Some(reachable) {
        return Ok(());
    }
    let touched: std::collections::BTreeSet<[u8; 32]> = std::collections::BTreeSet::new();
    let orphan = unreachable_nodes(conn, &touched)?;
    Err(format!(
        "StorageUnavailable：私有证据图节点覆盖不符（表内 {total} 行 / 本次可达 {reachable} 个；首例不可达 {}）",
        orphan
            .first()
            .map(|id| hex(id))
            .unwrap_or_else(|| "-".to_string())
    ))
}

/// 库内全部节点是否都被给定根闭包覆盖（不可达残留不得发布）。
pub(super) fn unreachable_nodes(
    conn: &Connection,
    touched: &std::collections::BTreeSet<[u8; 32]>,
) -> Result<Vec<[u8; 32]>, String> {
    let mut stmt = conn
        .prepare("SELECT node_id FROM s_evidence_nodes")
        .map_err(|e| format!("StorageUnavailable：读私有节点表失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| r.get::<_, Vec<u8>>(0))
        .map_err(|e| format!("StorageUnavailable：读私有节点表失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("StorageUnavailable：私有节点行损坏：{e}"))?;
    let mut orphan = Vec::new();
    for row in rows {
        let id = decode_id(&row)
            .ok_or_else(|| "StorageUnavailable：私有节点 id 非 32 字节".to_string())?;
        if !touched.contains(&id) {
            orphan.push(id);
        }
    }
    Ok(orphan)
}

/// 库内全部私有根 key（升序稳定序）。
pub(super) fn root_keys(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT root_key FROM s_evidence_roots ORDER BY root_key")
        .map_err(|e| format!("StorageUnavailable：读私有根表失败：{e}"))?;
    let keys = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| format!("StorageUnavailable：读私有根表失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("StorageUnavailable：私有根行损坏：{e}"))?;
    Ok(keys)
}

/// 节点图统计（供成本证据）。
pub(super) fn stats(conn: &Connection) -> Result<(i64, i64), String> {
    conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(length(payload)),0) FROM s_evidence_nodes",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| format!("StorageUnavailable：读私有节点统计失败：{e}"))
}

/// 拼接节点的每一项在展开时是否只按字面读取（测试与审计用的结构探针）。
#[cfg(test)]
pub(super) fn describe(conn: &Connection, root_key: &str) -> Result<Value, String> {
    let (root, params, len, hash) = Reader::root_row(conn, root_key)?
        .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 缺失"))?;
    Ok(json!({
        "root": hex_bytes(&root),
        "params": params.len().to_string(),
        "expanded_length": len.to_string(),
        "content_sha256": hash,
    }))
}

#[cfg(test)]
mod metadata_regressions {
    use super::*;

    fn publish_shared_slotted_root(conn: &Connection, key: &str, slot: u64) -> (RootRow, Vec<u8>) {
        let part = json!({"x": {"v": slot}, "y": {"v": slot}});
        let value = json!({"a": part, "b": part});
        let staged = stage_complete(&value, &[SlotSpec::number(&slot.to_string())]).unwrap();
        // 根、两次引用的父 SEQ、四次引用的子 SEQ、参数叶，只有四个唯一节点。
        assert_eq!(staged.nodes.len(), 4);
        publish_refs(conn, key, &staged).unwrap();
        let (node, slots, expanded_length, content_sha256) =
            Reader::root_row(conn, key).unwrap().unwrap();
        (
            RootRow {
                node,
                slots: CodecVec::from_plain(slots),
                expanded_length,
                content_sha256: TextValue::from_str(&content_sha256, None).unwrap(),
            },
            canonical_bytes_of_value(&value),
        )
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_reader_keeps_shared_slot_bytes_on_caller_ledger_and_releases() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let (_, expected) = publish_shared_slotted_root(&conn, "batch/owned", 9);
        let tx = conn.unchecked_transaction().unwrap();
        let budget = OwnedBudget::new(1 << 20);
        let mut reader = Reader::with_owner(&tx, &budget);
        let root = reader.read_root_owned("batch/owned").unwrap();
        assert_eq!(&root.bytes[..], &expected);
        assert!(root.bytes.allocation_budget().unwrap().same_ledger(&budget));
        assert!(budget.snapshot().live_requested_layout_bytes > root.bytes.capacity());
        drop(reader);
        assert!(budget.snapshot().live_requested_layout_bytes >= root.bytes.capacity());
        drop(root);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(0);
        let mut reader = Reader::with_owner(&tx, &low);
        let error = reader.read_root_owned("batch/owned").err().unwrap();
        assert!(error.starts_with(newchan_rust::s_content::capture_allocation::LIMIT));
        assert_eq!(low.snapshot().allocation_attempts, 0);
        assert!(low.snapshot().rejected_layouts > 0);
        drop(reader);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_reader_and_plain_reader_keep_bad_root_first_error() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        publish_shared_slotted_root(&conn, "batch/owned", 9);
        for update in [
            "UPDATE s_evidence_roots SET slot_ids='wrong type'",
            "UPDATE s_evidence_roots SET slot_ids=X'00',expanded_length=-1,content_sha256='bad'",
            "UPDATE s_evidence_roots SET slot_ids=X'',expanded_length=-1,content_sha256='bad'",
            "UPDATE s_evidence_roots SET expanded_length=0,content_sha256='bad'",
        ] {
            conn.execute(update, []).unwrap();
            let ordinary = {
                let mut r = Reader::new(&conn, 0);
                r.read_root("batch/owned", false).err().unwrap()
            };
            let budget = OwnedBudget::new(1 << 20);
            let owned = {
                let mut r = Reader::with_owner(&conn, &budget);
                r.read_root_owned("batch/owned").err().unwrap()
            };
            assert_eq!(ordinary, owned);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }

    #[test]
    fn completed_spans_read_nested_shared_slot_subtrees_once_in_transaction() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let (row, expected) = publish_shared_slotted_root(&conn, "batch/shared", 9);
        let tx = conn.unchecked_transaction().unwrap();
        assert!(!tx.is_autocommit());
        let mut reader = Reader::new(&tx, 0);
        cache::counters_reset();
        assert_eq!(reader.expand("batch/shared", &row).unwrap(), expected);
        // 真实 SQL 读数：一个参数叶 + 三个唯一 SEQ；旧展开会读 1 + 1 + 2 + 4 = 8 行。
        assert_eq!(cache::counters_snapshot().node_rows_read, 4);
        assert_eq!(cache::retained_bytes(), 0);
        assert_eq!(cache::capture_bytes_now(), 0);
        let full = reader.read_root("batch/shared", true).unwrap();
        assert_eq!(full.bytes.as_ref(), expected);
        assert_eq!(
            full.value.as_deref().map(|v| v.as_ref()),
            Some(&v2::strict_json(&expected).unwrap())
        );
        assert_eq!(cache::retained_bytes(), 0);
        assert_eq!(cache::capture_bytes_now(), 0);
    }

    #[test]
    fn completed_spans_keep_autocommit_reads_and_do_not_cross_roots_or_slots() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let (first, first_bytes) = publish_shared_slotted_root(&conn, "batch/first", 9);
        let (second, second_bytes) = publish_shared_slotted_root(&conn, "batch/second", 1000);
        assert_eq!(first.node, second.node);
        assert_ne!(&first.slots[..], &second.slots[..]);
        assert!(conn.is_autocommit());
        {
            let mut reader = Reader::new(&conn, 0);
            cache::counters_reset();
            assert_eq!(reader.expand("batch/first", &first).unwrap(), first_bytes);
            // 自动提交调用没有固定快照，仍按每次 Open 读当前行。
            assert_eq!(cache::counters_snapshot().node_rows_read, 8);
        }
        let tx = conn.unchecked_transaction().unwrap();
        let mut reader = Reader::new(&tx, 0);
        for (key, row, expected) in [
            ("batch/first", &first, &first_bytes),
            ("batch/second", &second, &second_bytes),
            ("batch/first", &first, &first_bytes),
        ] {
            cache::counters_reset();
            assert_eq!(&reader.expand(key, row).unwrap(), expected);
            // 同一 Reader、相同模板也不能继承前一次根/参数的展开区间。
            assert_eq!(cache::counters_snapshot().node_rows_read, 4);
            assert_eq!(cache::retained_bytes(), 0);
            assert_eq!(cache::capture_bytes_now(), 0);
        }
    }

    #[test]
    fn completed_spans_still_reject_damaged_nodes_and_root_bounds() {
        for (damage, message) in [
            ("node_length", "声明长度"),
            ("node_hash", "同 ID 异内容"),
            ("missing_node", "缺失"),
            ("unknown_tag", "分量标签 `255` 未声明"),
            ("root_hash", "展开字节与登记摘要不符"),
            ("root_short", "实际展开超过根声明长度"),
            ("root_long", "展开长度不符"),
        ] {
            cache::configure(0);
            let conn = Connection::open_in_memory().unwrap();
            declare_new(&conn).unwrap();
            let (row, _) = publish_shared_slotted_root(&conn, "batch/damaged", 9);
            let tx = conn.unchecked_transaction().unwrap();
            match damage {
                "node_length" => {
                    let payload: Vec<u8> = tx
                        .query_row(
                            "SELECT payload FROM s_evidence_nodes WHERE node_id=?1",
                            params![&row.node[..]],
                            |r| r.get(0),
                        )
                        .unwrap();
                    let shared = split_seq(&payload)
                        .unwrap()
                        .into_iter()
                        .find_map(|part| {
                            if let Part::Ref(id) = part {
                                Some(id)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    // 首次 Close 就须拒绝共享子树的坏长度，不能先登记为已完成区间。
                    tx.execute(
                        "UPDATE s_evidence_nodes SET expanded_length=expanded_length+1 WHERE node_id=?1",
                        params![&shared[..]],
                    )
                    .unwrap();
                }
                "node_hash" => {
                    tx.execute(
                        "UPDATE s_evidence_nodes SET payload=x'ff' WHERE node_id=?1",
                        params![&row.node[..]],
                    )
                    .unwrap();
                }
                "missing_node" => {
                    tx.execute(
                        "DELETE FROM s_evidence_nodes WHERE node_id=?1",
                        params![&row.node[..]],
                    )
                    .unwrap();
                }
                "unknown_tag" => {
                    // 使用真实内容 ID，使错误落在标签语法门，不能被同 ID 异内容门代替。
                    let id = node_id(KIND_SEQ, &[255]);
                    tx.execute(
                        "UPDATE s_evidence_nodes SET node_id=?1,payload=x'ff' WHERE node_id=?2",
                        params![&id[..], &row.node[..]],
                    )
                    .unwrap();
                    tx.execute("UPDATE s_evidence_roots SET node_id=?1", params![&id[..]])
                        .unwrap();
                }
                "root_hash" => {
                    tx.execute(
                        "UPDATE s_evidence_roots SET content_sha256=?1",
                        params!["0".repeat(64)],
                    )
                    .unwrap();
                }
                "root_short" | "root_long" => {
                    // short 在第二次父 SEQ 的整个区间追加时越界；long 仍须核最终等式。
                    let length =
                        row.expanded_length as i64 + if damage == "root_short" { -2 } else { 1 };
                    tx.execute(
                        "UPDATE s_evidence_roots SET expanded_length=?1",
                        params![length],
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            let (node, slots, expanded_length, content_sha256) =
                Reader::root_row(&tx, "batch/damaged").unwrap().unwrap();
            let current = RootRow {
                node,
                slots: CodecVec::from_plain(slots),
                expanded_length,
                content_sha256: TextValue::from_str(&content_sha256, None).unwrap(),
            };
            let mut reader = Reader::new(&tx, 0);
            let error = reader.expand("batch/damaged", &current).unwrap_err();
            assert!(error.contains(message), "{damage}: {error}");
            // 公开读取仍先走完整闭包，损坏不得由区间复用放行。
            assert!(reader.read_root("batch/damaged", true).is_err(), "{damage}");
            assert_eq!(cache::retained_bytes(), 0);
            assert_eq!(cache::capture_bytes_now(), 0);
        }
    }

    fn slotted_frame_fixture() -> (Connection, RootRow, Vec<u8>) {
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let value = json!({"v": 9});
        let staged = stage_complete(&value, &[SlotSpec::number("9")]).unwrap();
        publish_refs(&conn, "batch/frame", &staged).unwrap();
        let (node, slots, expanded_length, content_sha256) =
            Reader::root_row(&conn, "batch/frame").unwrap().unwrap();
        assert_eq!(slots.len(), 1);
        (
            conn,
            RootRow {
                node,
                slots: CodecVec::from_plain(slots),
                expanded_length,
                content_sha256: TextValue::from_str(&content_sha256, None).unwrap(),
            },
            canonical_bytes_of_value(&value),
        )
    }

    #[test]
    fn frame_close_uses_open_length_without_another_zero_cache_sql_read() {
        cache::configure(0);
        let (conn, row, expected) = slotted_frame_fixture();
        let mut reader = Reader::new(&conn, 0);
        for _ in 0..2 {
            cache::counters_reset();
            assert_eq!(reader.expand("batch/frame", &row).unwrap(), expected);
            // 此真实小图只有一个根帧和一个参数叶：各读一次；下一次展开仍重新读两行。
            assert_eq!(cache::counters_snapshot().node_rows_read, 2);
            assert_eq!(cache::retained_bytes(), 0);
            assert_eq!(cache::capture_bytes_now(), 0);
        }
        assert_eq!(
            reader
                .read_root("batch/frame", true)
                .unwrap()
                .bytes
                .as_ref(),
            expected
        );
        cache::configure(0);
    }

    #[test]
    fn frame_close_still_rejects_bad_declared_length_after_slot_subtraction() {
        cache::configure(0);
        let (conn, row, _) = slotted_frame_fixture();
        conn.execute(
            "UPDATE s_evidence_nodes SET expanded_length=expanded_length+1 WHERE node_id=?1",
            params![&row.node[..]],
        )
        .unwrap();
        let error = Reader::new(&conn, 0)
            .expand("batch/frame", &row)
            .unwrap_err();
        assert!(error.contains("声明长度 7 与实际展开 6 不符"), "{error}");
        let error = Reader::new(&conn, 0)
            .read_root("batch/frame", true)
            .err()
            .unwrap();
        assert!(error.contains("声明长度 7 与实际展开 6 不符"), "{error}");
        assert_eq!(cache::retained_bytes(), 0);
        assert_eq!(cache::capture_bytes_now(), 0);
        cache::configure(0);
    }

    #[test]
    fn prepared_node_query_rebinds_and_reads_current_rows_with_zero_retention() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let a = stage_complete(&json!({"value": "first"}), &[]).unwrap();
        let b = stage_complete(&json!({"value": "second"}), &[]).unwrap();
        publish_refs(&conn, "batch/first", &a).unwrap();
        publish_refs(&conn, "batch/second", &b).unwrap();
        assert_ne!(a.root, b.root);
        let mut reader = Reader::new(&conn, 0);
        let first = reader.load(&a.root).unwrap();
        let second = reader.load(&b.root).unwrap();
        assert_eq!(first.id, a.root);
        assert_eq!(second.id, b.root);
        assert_ne!(first.payload, second.payload);
        assert_eq!(reader.load(&a.root).unwrap().payload, first.payload);
        assert_eq!(cache::retained_bytes(), 0);
        assert_eq!(cache::capture_bytes_now(), 0);

        // 同一连接、同一 SQL 计划也必须看到当前行；不得把上次的正确结果当新鲜原件。
        let mut damaged = first.payload.to_vec();
        damaged.push(0);
        conn.execute(
            "UPDATE s_evidence_nodes SET payload=?1 WHERE node_id=?2",
            params![damaged, &a.root[..]],
        )
        .unwrap();
        let error = reader.load(&a.root).err().unwrap();
        assert!(error.contains("同 ID 异内容"), "{error}");
        conn.execute(
            "DELETE FROM s_evidence_nodes WHERE node_id=?1",
            params![&b.root[..]],
        )
        .unwrap();
        let error = reader.load(&b.root).err().unwrap();
        assert!(error.contains("缺失"), "{error}");
        assert_eq!(cache::retained_bytes(), 0);
        assert_eq!(cache::capture_bytes_now(), 0);
    }

    #[test]
    fn evidence_graph_parameter_lengths_are_verified_and_conflicts_refused() {
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        for value in [9, 1000] {
            let json = json!({"v": value});
            let staged = stage_complete(&json, &[SlotSpec::number(&value.to_string())]).unwrap();
            let key = format!("batch/test-{value}");
            publish_refs(&conn, &key, &staged).unwrap();
            assert_eq!(
                Reader::new(&conn, 0).read_fixed_cut(&key).unwrap().as_ref(),
                canonical_bytes_of_value(&json).as_slice()
            );
            assert_eq!(
                Reader::new(&conn, 1024 * 1024)
                    .read_fixed_cut(&key)
                    .unwrap()
                    .as_ref(),
                canonical_bytes_of_value(&json).as_slice()
            );
        }
        let staged = stage_complete(&json!({"v": 9}), &[SlotSpec::number("9")]).unwrap();
        conn.execute(
            "UPDATE s_evidence_nodes SET expanded_length=expanded_length+1 WHERE node_id=?1",
            params![&staged.root[..]],
        )
        .unwrap();
        let error = Reader::new(&conn, 1024 * 1024)
            .read_fixed_cut("batch/test-9")
            .unwrap_err();
        assert!(error.contains("声明长度"), "{error}");
        let error = publish_refs(&conn, "batch/test-9", &staged).unwrap_err();
        assert!(error.contains("IdentityConflict"), "{error}");
    }

    #[test]
    fn evidence_graph_previous_candidate_format_is_explicitly_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        conn.execute(
            "UPDATE s_evidence_format SET graph_format='evidence-graph/1'",
            [],
        )
        .unwrap();
        assert!(read_format(&conn)
            .unwrap_err()
            .contains("SchemaUnsupported"));
    }

    #[test]
    fn evidence_graph_forged_root_length_is_rejected_without_large_allocation() {
        let conn = Connection::open_in_memory().unwrap();
        declare_new(&conn).unwrap();
        let staged = stage_complete(&json!({"v": 9}), &[]).unwrap();
        publish_refs(&conn, "batch/test", &staged).unwrap();
        conn.execute(
            "UPDATE s_evidence_roots SET expanded_length=?1",
            params![i64::MAX],
        )
        .unwrap();
        let error = Reader::new(&conn, 0)
            .read_fixed_cut("batch/test")
            .unwrap_err();
        assert!(error.contains("根 `batch/test` 展开长度不符"), "{error}");
    }
}
