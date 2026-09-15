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
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            params![FORMAT_TABLE],
            |r| r.get(0),
        )
        .map_err(|e| format!("StorageUnavailable：读私有图格式登记失败：{e}"))?;
    if exists == 0 {
        return Ok(Format::LegacyBlob);
    }
    let mut stmt = conn
        .prepare("SELECT singleton,graph_format FROM s_evidence_format")
        .map_err(|e| format!("StorageUnavailable：私有图格式登记结构不符：{e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| format!("StorageUnavailable：读私有图格式登记失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("StorageUnavailable：私有图格式登记行损坏：{e}"))?;
    let [(singleton, format)] = rows.as_slice() else {
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
    let conn_id = conn as *const Connection as usize;
    if let Some(cached) = cache::capture_format(conn_id) {
        return Ok(cached);
    }
    let format = read_format(conn)?;
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
                return Err(format!("StorageUnavailable：私有根 `{root_key}` 缺逻辑记录标识"));
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
pub(super) fn split_fact_root(bytes: &[u8], what: &str) -> Result<(String, String, String), String> {
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
        let mut stack: Vec<Frame> = Vec::new();
        match child {
            Child::Lit(b) => stack.push(Frame::Lit(b.clone())),
            Child::Slot(i) => stack.push(Frame::Slot(*i)),
            Child::Ref(id) => stack.push(Frame::Open(*id)),
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
    let mut parts = Vec::new();
    let mut i = 0usize;
    while i < payload.len() {
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
                parts.push(Part::Lit(&payload[start..end]));
                i = end;
            }
            TAG_REF => {
                if i + 33 > payload.len() {
                    return Err("StorageUnavailable：拼接节点引用截断".into());
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(&payload[i + 1..i + 33]);
                parts.push(Part::Ref(id));
                i += 33;
            }
            TAG_SLOT => {
                if i + 3 > payload.len() {
                    return Err("StorageUnavailable：拼接节点参数槽截断".into());
                }
                parts.push(Part::Slot(u16::from_be_bytes([
                    payload[i + 1],
                    payload[i + 2],
                ])));
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

fn decode_into(kind: i64, payload: &[u8], stack: &mut Vec<Frame>) -> Result<(), String> {
    if kind == KIND_LEAF {
        stack.push(Frame::Lit(payload.to_vec()));
        return Ok(());
    }
    let mut frames = Vec::new();
    for part in split_seq(payload)? {
        match part {
            Part::Lit(bytes) => frames.push(Frame::Lit(bytes.to_vec())),
            Part::Ref(id) => frames.push(Frame::Open(id)),
            Part::Slot(idx) => frames.push(Frame::Slot(idx)),
        }
    }
    for f in frames.into_iter().rev() {
        stack.push(f);
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
    slots: Vec<[u8; 32]>,
    expanded_length: u64,
    content_sha256: String,
}

/// 一次根读取的结果：展开字节、根声明长度与内容摘要、以及按需解码的规范 JSON。
pub(super) struct VerifiedRoot {
    pub(super) bytes: std::sync::Arc<[u8]>,
    pub(super) declared_length: u64,
    pub(super) content_sha256: String,
    pub(super) value: Option<std::sync::Arc<Value>>,
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
    pub(super) root_cert: Option<std::sync::Arc<cache::RootCert>>,
}

/// 一次根闭包核验的结果：根行 + 派生量 + 指纹 + 原件门 + 闭包内唯一节点集。
struct ClosureCheck {
    row: RootRow,
    digest: [u8; 32],
    /// 根行与闭包内每个唯一节点都在本捕获与保留证书逐字节相同。
    originals_verified: bool,
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
    h.update(row.content_sha256.as_bytes());
    h.update(subtree);
    h.finalize().into()
}

/// 子树摘要：id 已经把 payload（含子节点 id 与顺序）钉死，故摘要再收 id、种类、声明长度与子摘要。
fn node_subtree_digest(
    kind: i64,
    id: &[u8; 32],
    length: u64,
    children: &[[u8; 32]],
) -> [u8; 32] {
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

/// 私有图的读取器：同一次捕获内每个唯一节点原行只读一次；跨捕获复用已核句柄，容量有限时
/// 只失去复用，不改变合法性判据。
pub(super) struct Reader<'a> {
    conn: &'a Connection,
    conn_id: usize,
    /// 本次捕获的临时读集守卫：析构即作废本次读入的节点行。
    _capture: cache::CaptureGuard,
    /// 本次读取实际读到过的节点 id（供「不可达残留」核对）。
    touched: std::collections::BTreeSet<[u8; 32]>,
}

impl<'a> Reader<'a> {
    /// `budget_bytes` = 本次捕获**临时读集**的保留上限（同次捕获每个唯一节点原行只读一次所用）。
    /// 跨请求的持久复用由 `cache::configure` 的单一预算决定。
    pub(super) fn new(conn: &'a Connection, budget_bytes: usize) -> Self {
        Reader {
            conn,
            conn_id: conn as *const Connection as usize,
            _capture: cache::begin_capture(budget_bytes),
            touched: std::collections::BTreeSet::new(),
        }
    }

    pub(super) fn touched(&self) -> &std::collections::BTreeSet<[u8; 32]> {
        &self.touched
    }

    /// 把捕获作用域交给调用方保管：同一次根审计的后续读取继续共享同一份节点读集。
    pub(super) fn into_capture(self) -> cache::CaptureGuard {
        self._capture
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
        let row: Option<(Vec<u8>, Vec<u8>, i64, String)> = conn
            .query_row(
                "SELECT node_id,slot_ids,expanded_length,content_sha256 FROM s_evidence_roots WHERE root_key=?1",
                params![root_key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("StorageUnavailable：读私有根失败：{e}"))?;
        let Some((node_id, slots, len, hash)) = row else {
            return Ok(None);
        };
        let root = decode_id(&node_id)
            .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 节点 id 非 32 字节"))?;
        if slots.len() % 32 != 0 {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 参数槽列长度非 32 倍数"
            ));
        }
        let mut params = Vec::with_capacity(slots.len() / 32);
        for chunk in slots.chunks(32) {
            let mut id = [0u8; 32];
            id.copy_from_slice(chunk);
            params.push(id);
        }
        let expanded = u64::try_from(len)
            .map_err(|_| format!("StorageUnavailable：私有根 `{root_key}` 长度非负越界"))?;
        if !is_hex64(&hash) {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 内容摘要非 64 位十六进制"
            ));
        }
        Ok(Some((root, params, expanded, hash)))
    }

    /// 闭包遍历（**不展开**）：从当前新鲜行读根行与闭包内每个唯一节点原行，逐节点核内容寻址、
    /// 种类、参数叶身份与声明长度自洽，最后给出与**节点 id/种类/长度 + 根行**一一对应的闭包指纹。
    ///
    /// 指纹相同 ⟺ 本次读到的闭包与原次同形：节点 id 是内容地址且每次都用刚读到的 payload 重算，
    /// 故不比较「ID/hash 自述」，而是比较**当前字节算出的**指纹。
    fn closure(
        &mut self,
        root_key: &str,
    ) -> Result<ClosureCheck, String> {
        let (root, slots, expanded_len, hash) = Reader::root_row(self.conn, root_key)?
            .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 缺失"))?;
        // 声明槽未必都被模板引用：只核每个槽叶自身可寻址；实际占用的槽字节在遍历时累计。
        let mut slot_len = vec![0u64; slots.len()];
        for id in &slots {
            let node = self.load(id)?;
            if node.kind != KIND_LEAF {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 参数槽引用了非叶节点"
                ));
            }
        }
        for (i, id) in slots.iter().enumerate() {
            slot_len[i] = self.load(id)?.expanded_length;
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
        let originals_verified = cache::root_cert_match(
            root_key,
            &row.node,
            &row.slots,
            row.expanded_length,
            &row.content_sha256,
        )
        .is_some()
            && cache::cert_is_certified(self.conn_id, root);
        Ok(ClosureCheck {
            row,
            digest,
            originals_verified,
        })
    }

    /// 一个与根无关的节点派生量：`length`（不含槽）、`slot_counts`、子树摘要。
    ///
    /// 后序迭代 + 捕获内 memo；`on_path` 只含当前路径——闭包成环一律拒绝（不得用无限遍历替代判据）。
    fn derived_of(&mut self, root: [u8; 32]) -> Result<cache::NodeDerived, String> {
        if let Some(found) = cache::derived_get(self.conn_id, &root) {
            // 本次捕获已按新鲜行结算过该节点：仍要记账（可达集必须覆盖它），但不重复读行。
            self.touched.insert(root);
            return Ok(found);
        }
        let mut on_path: std::collections::BTreeSet<[u8; 32]> = std::collections::BTreeSet::new();
        // 本次遍历的即时工作集：即使捕获级 memo 因限额/写入缺失，也能就地重算，
        // 绝不把「限额不足」判成 StorageUnavailable。
        let mut local: std::collections::BTreeMap<[u8; 32], cache::NodeDerived> =
            std::collections::BTreeMap::new();
        let mut stack: Vec<([u8; 32], bool)> = vec![(root, false)];
        while let Some((id, closing)) = stack.pop() {
            // 热路径只问存在性（不克隆派生量：`slot_counts` 是 Vec，克隆会为每个节点多一次分配）。
            if local.contains_key(&id) || cache::derived_has(self.conn_id, &id) {
                self.touched.insert(id);
                continue;
            }
            let node = self.load(&id)?;
            if node.kind == KIND_LEAF {
                let derived = cache::NodeDerived {
                    length: node.expanded_length,
                    slot_counts: Vec::new(),
                    digest: node_subtree_digest(KIND_LEAF, &id, node.expanded_length, &[]),
                };
                local.insert(id, derived.clone());
                cache::derived_put(self.conn_id, id, derived);
                // 叶子没有子依赖：本次逐字节命中即可认证（认证搭在既有后序遍历上，零额外遍历）。
                if cache::cert_marked(self.conn_id, id) {
                    cache::cert_certify(self.conn_id, id);
                }
                continue;
            }
            let parts = split_seq(&node.payload)?;
            if !closing {
                if !on_path.insert(id) {
                    return Err(format!(
                        "StorageUnavailable：私有图闭包出现环（节点 {} 自引用）",
                        hex(&id)
                    ));
                }
                stack.push((id, true));
                for child in node.children.iter().rev() {
                    stack.push((*child, false));
                }
                continue;
            }
            // 子节点派生量必须已结算；被逐出/未结算时重新下沉重算，不误判为存储损坏。
            let mut missing: Option<[u8; 32]> = None;
            for part in &parts {
                if let Part::Ref(child) = part {
                    if !local.contains_key(child) && !cache::derived_has(self.conn_id, child) {
                        missing = Some(*child);
                        break;
                    }
                }
            }
            if let Some(child) = missing {
                stack.push((id, true));
                stack.push((child, false));
                continue;
            }
            let mut length = 0u64;
            let mut counts: Vec<u64> = Vec::new();
            let mut children: Vec<[u8; 32]> = Vec::new();
            for part in parts {
                match part {
                    Part::Lit(bytes) => length = length.saturating_add(bytes.len() as u64),
                    Part::Slot(index) => {
                        let index = index as usize;
                        if counts.len() <= index {
                            counts.resize(index + 1, 0);
                        }
                        counts[index] = counts[index].saturating_add(1);
                    }
                    Part::Ref(child) => {
                        let entry = local
                            .get(&child)
                            .cloned()
                            .or_else(|| cache::derived_get(self.conn_id, &child))
                            .ok_or_else(|| {
                                format!(
                                    "StorageUnavailable：私有节点 {} 的子节点 {} 未在闭包内结算",
                                    hex(&id),
                                    hex(&child)
                                )
                            })?;
                        length = length.saturating_add(entry.length);
                        if counts.len() < entry.slot_counts.len() {
                            counts.resize(entry.slot_counts.len(), 0);
                        }
                        for (index, count) in entry.slot_counts.iter().enumerate() {
                            counts[index] = counts[index].saturating_add(*count);
                        }
                        children.push(child);
                    }
                }
            }
            if length != node.expanded_length {
                return Err(format!(
                    "StorageUnavailable：私有节点 {} 声明长度 {} 与实际展开 {length} 不符",
                    hex(&id),
                    node.expanded_length
                ));
            }
            on_path.remove(&id);
            let derived = cache::NodeDerived {
                length,
                slot_counts: counts,
                digest: node_subtree_digest(KIND_SEQ, &id, length, &children),
            };
            local.insert(id, derived.clone());
            cache::derived_put(self.conn_id, id, derived);
            // 子节点（本帧的后序先决条件）已全部结算：只要每个直接子节点都已认证，本节点也认证。
            if cache::cert_marked(self.conn_id, id)
                && children
                    .iter()
                    .all(|child| cache::cert_is_certified(self.conn_id, *child))
            {
                cache::cert_certify(self.conn_id, id);
            }
        }
        local
            .get(&root)
            .cloned()
            .or_else(|| cache::derived_get(self.conn_id, &root))
            .ok_or_else(|| "StorageUnavailable：私有节点派生量缺失（实现缺陷）".to_string())
    }

    /// 读一个根（**单次闭包遍历**）：指纹命中已核句柄即复用其已验展开字节/解析结果；未命中则
    /// 完整展开并核公开长度与摘要后入句柄。两条路径的返回必须逐字节相同。
    pub(super) fn read_root(
        &mut self,
        root_key: &str,
        with_value: bool,
    ) -> Result<VerifiedRoot, String> {
        let check = self.closure(root_key)?;
        // 复用门：闭包指纹只是**附加**校验；真正的门是 (a) 该根保留着的原件证书与本次新鲜根行
        // 逐字节相同、(b) 闭包内每个唯一节点都在本次捕获与保留证书逐字节相同。
        let mut retained = if check.originals_verified {
            cache::root_cert_get(root_key)
        } else {
            None
        };
        let cached = retained
            .as_ref()
            .and_then(|rc| cache::handle_get_certified(root_key, rc, &check.digest));
        let reused = cached.is_some();
        #[cfg(test)]
        cache::counter(|c| {
            c.root_reads += 1;
            if reused {
                c.root_handle_hits += 1;
            } else {
                c.root_expansions += 1;
            }
        });
        let (bytes, mut value) = match cached {
            Some((bytes, value)) => (bytes, value),
            None => {
                let expanded = self.expand(root_key, &check.row)?;
                let bytes: std::sync::Arc<[u8]> = std::sync::Arc::from(expanded.as_slice());
                // 已核闭包：把本次读到的**原件证书**保留下来（共享同一份原节点，按唯一保留量
                // 记账），供后续捕获逐字节比较。只在成功核过长度/摘要之后才保留。
                let mut ids: std::collections::BTreeSet<[u8; 32]> =
                    std::collections::BTreeSet::new();
                let mut frontier: Vec<[u8; 32]> = vec![check.row.node];
                while let Some(id) = frontier.pop() {
                    if !ids.insert(id) {
                        continue;
                    }
                    // 命中本捕获读集时不再读 SQLite；读集被逐出时按新鲜行重读。
                    frontier.extend(self.load(&id)?.children.iter().copied());
                }
                cache::cert_commit(self.conn_id, &ids);
                let cert = cache::RootCert {
                    node: check.row.node,
                    slots: check.row.slots.clone(),
                    expanded_length: check.row.expanded_length,
                    content_sha256: check.row.content_sha256.clone(),
                };
                retained = cache::handle_put_certified(
                    root_key,
                    &cert,
                    check.digest,
                    bytes.clone(),
                    None,
                );
                (bytes, None)
            }
        };
        if with_value && value.is_none() {
            let parsed = v2::strict_json(&bytes)
                .map_err(|e| format!("StorageUnavailable：私有根 `{root_key}` 非严格 JSON：{e}"))?;
            if !parsed.is_object() {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 非 JSON 对象"
                ));
            }
            let parsed = std::sync::Arc::new(parsed);
            if let Some(cert) = retained.clone() {
                retained = cache::handle_put_certified(
                    root_key,
                    &cert,
                    check.digest,
                    bytes.clone(),
                    Some(parsed.clone()),
                );
            }
            value = Some(parsed);
            #[cfg(test)]
            cache::counter(|c| c.root_parses += 1);
        }
        Ok(VerifiedRoot {
            bytes,
            declared_length: check.row.expanded_length,
            content_sha256: check.row.content_sha256,
            value,
            closure: check.digest,
            reused,
            originals_verified: check.originals_verified,
            root_cert: retained,
        })
    }

    /// 读一个根的展开字节。
    pub(super) fn read_fixed_cut(&mut self, root_key: &str) -> Result<std::sync::Arc<[u8]>, String> {
        Ok(self.read_root(root_key, false)?.bytes)
    }

    /// 读一个根并解码为规范 JSON 对象（同一句柄缓存，含展开字节）。
    pub(super) fn read_fixed_cut_value(
        &mut self,
        root_key: &str,
    ) -> Result<(std::sync::Arc<[u8]>, std::sync::Arc<Value>), String> {
        let root = self.read_root(root_key, true)?;
        let value = root
            .value
            .ok_or("StorageUnavailable：私有根解码结果缺失（实现缺陷）")?;
        Ok((root.bytes, value))
    }

    /// 完整展开（冷路径）：返回前核根声明的长度与摘要，长度增长受根声明上界约束。
    fn expand(&mut self, root_key: &str, row: &RootRow) -> Result<Vec<u8>, String> {
        let expanded_len = row.expanded_length;
        let hash = row.content_sha256.as_str();
        let root = row.node;
        let mut param_bytes: Vec<Vec<u8>> = Vec::with_capacity(row.slots.len());
        for id in &row.slots {
            let node = self.load(id)?;
            if node.kind != KIND_LEAF {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 参数槽引用了非叶节点"
                ));
            }
            param_bytes.push(node.payload.to_vec());
        }
        // 声明长度来自数据库，不能用它直接请求任意大的分配；实际输出逐段增长。
        let mut out: Vec<u8> = Vec::new();
        let mut stack: Vec<Frame> = vec![Frame::Open(root)];
        let mut slot_bytes = 0u64;
        while let Some(item) = stack.pop() {
            match item {
                Frame::Lit(bytes) => append_checked(&mut out, &bytes, expanded_len)?,
                Frame::Slot(i) => {
                    let bytes = param_bytes.get(i as usize).ok_or_else(|| {
                        format!("StorageUnavailable：私有根 `{root_key}` 参数槽 {i} 越界")
                    })?;
                    append_checked(&mut out, bytes, expanded_len)?;
                    slot_bytes += bytes.len() as u64;
                }
                Frame::Open(id) => {
                    let node = self.load(&id)?;
                    stack.push(Frame::Close {
                        id,
                        start: out.len(),
                        slot_bytes_at_open: slot_bytes,
                    });
                    decode_into(node.kind, &node.payload, &mut stack)?;
                }
                Frame::Close {
                    id,
                    start,
                    slot_bytes_at_open,
                } => {
                    let node = self.load(&id)?;
                    let actual = (out.len() - start) as u64;
                    let without_slots = actual - (slot_bytes - slot_bytes_at_open);
                    if without_slots != node.expanded_length {
                        return Err(format!(
                            "StorageUnavailable：私有节点 {} 声明长度 {} 与实际展开 {without_slots} 不符",
                            hex(&id),
                            node.expanded_length
                        ));
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
        if sha256_hex(&out) != hash {
            return Err(format!(
                "StorageUnavailable：私有根 `{root_key}` 展开字节与登记摘要不符（内容被改）"
            ));
        }
        Ok(out)
    }

    /// 按 id 取节点**原件证书**：本次捕获已读过就直接复用；否则从**新鲜行**读入，先核内容寻址与
    /// 长度自洽，再与保留证书**逐字节**比较（命中才打上原件门标记），最后记入本次捕获的临时读集
    /// （超预算只失去复用，当次读照常返回）。
    fn load(&mut self, id: &[u8; 32]) -> Result<std::sync::Arc<cache::Cert>, String> {
        self.touched.insert(*id);
        cache::mark_reachable(self.conn_id, *id);
        if let Some(node) = cache::node_get(self.conn_id, id) {
            #[cfg(test)]
            cache::counter(|c| c.node_reused_in_capture += 1);
            return Ok(node);
        }
        let (kind, payload, len): (i64, Vec<u8>, i64) = self
            .conn
            .query_row(
                "SELECT node_kind,payload,expanded_length FROM s_evidence_nodes WHERE node_id=?1",
                params![&id[..]],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| format!("StorageUnavailable：读私有节点失败：{e}"))?
            .ok_or_else(|| format!("StorageUnavailable：私有节点 {} 缺失", hex(id)))?;
        if kind != KIND_LEAF && kind != KIND_SEQ {
            return Err(format!(
                "StorageUnavailable：私有节点 {} 种类 `{kind}` 未声明",
                hex(id)
            ));
        }
        if node_id(kind, &payload) != *id {
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
        // 拼接节点的分量语法在每次展开的 `decode_into` 里逐项核；其声明长度由展开时的
        // `Frame::Close` 与 `Reader::read_fixed_cut` 的根长度/摘要共同核对。
        // 证书按内容寻址键**引用**直接子证书（叶子为空）；标签未声明/截断一律拒绝，不猜。
        let children = if kind == KIND_LEAF {
            Vec::new()
        } else {
            let mut children = Vec::new();
            for part in split_seq(&payload)? {
                if let Part::Ref(child) = part {
                    children.push(child);
                }
            }
            children
        };
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
                cache::cert_matched_mark(self.conn_id, *id);
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
                (
                    std::sync::Arc::new(cache::Cert {
                        id: *id,
                        kind,
                        payload: std::sync::Arc::from(payload),
                        expanded_length: declared,
                        children,
                    }),
                    true,
                )
            }
        };
        cache::node_put(self.conn_id, cert.clone(), owned);
        Ok(cert)
    }
}

fn append_checked(out: &mut Vec<u8>, bytes: &[u8], declared: u64) -> Result<(), String> {
    let next = out
        .len()
        .checked_add(bytes.len())
        .ok_or("StorageUnavailable：证据图展开长度溢出")?;
    if next as u64 > declared {
        return Err("StorageUnavailable：证据图实际展开超过根声明长度".into());
    }
    out.try_reserve(bytes.len())
        .map_err(|_| "StorageUnavailable：证据图展开内存不足")?;
    out.extend_from_slice(bytes);
    Ok(())
}

/// 展开栈帧：`Open` 进入一个节点、`Close` 收束并核该节点的声明长度（DFS 输出连续）。
enum Frame {
    Open([u8; 32]),
    Close {
        id: [u8; 32],
        start: usize,
        /// 打开该节点时已消耗的槽字节数：节点声明长度不含槽字节，故核对时要把它减掉。
        slot_bytes_at_open: u64,
    },
    Lit(Vec<u8>),
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
