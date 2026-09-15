//! #1394 第二段：完整证据的私有内容寻址共享表示（`evidence-graph/2`）。
//!
//! 本模块只解决**物理表示**：把 S 已经算出的完整规范 JSON（批次、结构事实等）换成一棵
//! 内容寻址的**无损**节点图，使跨代重复的原件只落一次盘，同时保证：
//!
//! - **公开身份不变**：节点图不参与 `object_id`/`batch_id`/任何公开字段。展开字节必须与
//!   `canonical_json` 逐字节相同；公开哈希仍按**完整展开字节**重算，不改成 Merkle 身份。
//! - **不是「先构造再压缩」**：调用方把已算好的完整 Value 交给本模块，模块在**写入前**用
//!   自己的展开结果与同一份规范字节对拍；对拍失败即拒绝写入。本节**不声称**上游构造已增量。
//! - **字节精确**：叶保存现有规范字节；JSON 字符串内部嵌套的另一层 JSON 只有在
//!   「解析→按同一 serializer 回写→与该字符串内容逐字节相同」时才拆开共享，否则整段作叶。
//!   转义序列按字符处理，不会把 `\"` 拆到两个叶之间。
//! - **显式版本化**：新库在 `s_evidence_format` 落唯一一行 `evidence-graph/2`；表缺 = 旧库
//!   （整块 BLOB），表在而值未知/缺行/多行一律拒绝。不提供「严格失败再试宽松」的降级通道。
//! - **不原地迁移**：只在新建库声明；旧库只读兼容，旧 writer 见到多出的表即被 schema 门拒绝。
//! - **迭代展开**：展开用显式栈，图深与输入长度无关，避免 9011 层递归。
//! - **同 ID 异内容拒绝**：节点与根按内容寻址；同 ID 已存在时逐字节比对，不符即 `IdentityConflict`。

use super::*;

/// 私有格式登记表（不在公共协议里，也不参与公开对象/批次身份）。
pub(super) const FORMAT_TABLE: &str = "s_evidence_format";
/// 不可变节点表。
pub(super) const NODES_TABLE: &str = "s_evidence_nodes";
/// 公开身份 → 私有根的映射表。
pub(super) const ROOTS_TABLE: &str = "s_evidence_roots";
/// 本 reader/writer 唯一声明的图格式。
pub(super) const GRAPH_FORMAT: &str = "evidence-graph/2";

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
    /// 新库：完整规范字节经节点图落盘，`batches.canonical_bytes` 只留空占位。
    Graph,
}

impl Format {
    pub(super) fn graph(self) -> bool {
        matches!(self, Format::Graph)
    }
    /// 缓存/证书身份的稳定标签：物理域变化必须使旧证书失效。
    pub(super) fn tag(self) -> &'static str {
        match self {
            Format::LegacyBlob => "batch-blob/0",
            Format::Graph => GRAPH_FORMAT,
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
    if format != GRAPH_FORMAT {
        return Err(format!(
            "SchemaUnsupported：未知证据图格式 `{format}`（本 reader 只认无登记表的旧整块域与 `{GRAPH_FORMAT}`）"
        ));
    }
    Ok(Format::Graph)
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

/// 公开批次身份 → 私有根 key。公开 ID 与私有根句柄分列，永不混用。
pub(super) fn batch_root_key(batch_id: &str) -> String {
    format!("batch/{batch_id}")
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
    kind: i64,
    payload: Vec<u8>,
    expanded_length: u64,
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
                    decode_into(node, &mut stack)?;
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

fn decode_into(node: &Node, stack: &mut Vec<Frame>) -> Result<(), String> {
    if node.kind == KIND_LEAF {
        stack.push(Frame::Lit(node.payload.clone()));
        return Ok(());
    }
    let mut frames = Vec::new();
    let mut i = 0usize;
    while i < node.payload.len() {
        match node.payload[i] {
            TAG_LIT => {
                if i + 5 > node.payload.len() {
                    return Err("StorageUnavailable：拼接节点字面量长度截断".into());
                }
                let len = u32::from_be_bytes(
                    node.payload[i + 1..i + 5]
                        .try_into()
                        .map_err(|_| "StorageUnavailable：字面量长度解码失败")?,
                ) as usize;
                let start = i + 5;
                let end = start + len;
                if end > node.payload.len() {
                    return Err("StorageUnavailable：拼接节点字面量越界".into());
                }
                frames.push(Frame::Lit(node.payload[start..end].to_vec()));
                i = end;
            }
            TAG_REF => {
                if i + 33 > node.payload.len() {
                    return Err("StorageUnavailable：拼接节点引用截断".into());
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(&node.payload[i + 1..i + 33]);
                frames.push(Frame::Open(id));
                i += 33;
            }
            TAG_SLOT => {
                if i + 3 > node.payload.len() {
                    return Err("StorageUnavailable：拼接节点参数槽截断".into());
                }
                let idx = u16::from_be_bytes([node.payload[i + 1], node.payload[i + 2]]);
                frames.push(Frame::Slot(idx));
                i += 3;
            }
            other => {
                return Err(format!(
                    "StorageUnavailable：拼接节点分量标签 `{other}` 未声明"
                ))
            }
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

/// 私有图的读取器：同一次读操作内节点只解码一次，容量有限时只失去复用，不改变合法性判据。
pub(super) struct Reader<'a> {
    conn: &'a Connection,
    budget: usize,
    used: usize,
    cache: std::collections::HashMap<[u8; 32], std::sync::Arc<Node>>,
    /// 本次读取实际解码过的节点 id（供「不可达残留」核对）。
    touched: std::collections::BTreeSet<[u8; 32]>,
}

impl<'a> Reader<'a> {
    pub(super) fn new(conn: &'a Connection, budget_bytes: usize) -> Self {
        Reader {
            conn,
            budget: budget_bytes,
            used: 0,
            cache: std::collections::HashMap::new(),
            touched: std::collections::BTreeSet::new(),
        }
    }

    pub(super) fn touched(&self) -> &std::collections::BTreeSet<[u8; 32]> {
        &self.touched
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

    /// 展开一个根，并在返回前核：节点自身可寻址、长度自洽、展开字节与根声明的长度/摘要一致。
    pub(super) fn read_fixed_cut(&mut self, root_key: &str) -> Result<Vec<u8>, String> {
        let (root, params, expanded_len, hash) = Reader::root_row(self.conn, root_key)?
            .ok_or_else(|| format!("StorageUnavailable：私有根 `{root_key}` 缺失"))?;
        let mut param_bytes: Vec<Vec<u8>> = Vec::with_capacity(params.len());
        for id in &params {
            let node = self.load(id)?;
            if node.kind != KIND_LEAF {
                return Err(format!(
                    "StorageUnavailable：私有根 `{root_key}` 参数槽引用了非叶节点"
                ));
            }
            param_bytes.push(node.payload.clone());
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
                    decode_into(&node, &mut stack)?;
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

    /// 按 id 取节点：先核内容寻址与长度自洽，再入缓存。
    fn load(&mut self, id: &[u8; 32]) -> Result<std::sync::Arc<Node>, String> {
        self.touched.insert(*id);
        if let Some(node) = self.cache.get(id) {
            return Ok(node.clone());
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
        let node = std::sync::Arc::new(Node {
            kind,
            payload,
            expanded_length: declared,
        });
        let size = node.payload.len() + 64;
        if self.used + size > self.budget {
            self.cache.clear();
            self.used = 0;
        }
        self.used += size;
        self.cache.insert(*id, node.clone());
        Ok(node)
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
                Reader::new(&conn, 0).read_fixed_cut(&key).unwrap(),
                canonical_bytes_of_value(&json)
            );
            assert_eq!(
                Reader::new(&conn, 1024 * 1024)
                    .read_fixed_cut(&key)
                    .unwrap(),
                canonical_bytes_of_value(&json)
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
