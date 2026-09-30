//! #1394 C §5.6：从调用方已核 anchor 根取得片段，接收方仅凭本片证明重新核范围。
//! 本层不授予裸根读取权限，不生成 token，不证明未读子图或业务来源。
//! 消费者的同一Job计唯一接纳节点与绝对期限；服务端沿自己的资源门读取。
//! wire/request/六族顶层记录及磁盘分配由实际调用层计；接收验证仍强制消费者Job。

use super::{
    access::Access,
    job::{CheckedFrame, Job},
    path::{self, PathStep},
    *,
};
use serde_json::{json, Value as J};
use std::collections::BTreeMap;

pub const INVALID: &str = "content_fragment_invalid";
pub const REPLY_LIMIT: &str = "content_fragment_reply_limit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Items,
    Bytes,
    Value,
}
impl Unit {
    pub fn name(self) -> &'static str {
        match self {
            Self::Items => "items",
            Self::Bytes => "bytes",
            Self::Value => "value",
        }
    }
    pub fn parse(s: &str) -> Result<Self, CodecError> {
        match s {
            "items" => Ok(Self::Items),
            "bytes" => Ok(Self::Bytes),
            "value" => Ok(Self::Value),
            _ => Err(err("未知unit")),
        }
    }
}

/// 证明节点的 exact transport 三字段；不是已完成子图的证书。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProofNode {
    pub root: String,
    pub tag: String,
    pub body_hex: String,
}
impl ProofNode {
    pub fn to_json(&self) -> J {
        json!({"root":self.root,"tag":self.tag,"body_hex":self.body_hex})
    }
    pub fn from_json(v: &J) -> Result<Self, CodecError> {
        exact(v, &["root", "tag", "body_hex"])?;
        let text = |k| {
            v[k].as_str()
                .map(str::to_owned)
                .ok_or_else(|| err("证明字段须为字符串"))
        };
        let node = Self {
            root: text("root")?,
            tag: text("tag")?,
            body_hex: text("body_hex")?,
        };
        node.decode()?;
        Ok(node)
    }
    fn decode(&self) -> Result<(NodeDigest, u8, Vec<u8>), CodecError> {
        let digest = parse_root(&self.root)?;
        let tag = unhex(&self.tag)?;
        if tag.len() != 1 {
            return Err(err("tag须为两位小写十六进制"));
        }
        if !matches!(tag[0], 0x00..=0x08 | 0x10..=0x14) {
            return Err(err("未知节点标签"));
        }
        let body = unhex(&self.body_hex)?;
        if frame(tag[0], &body) != digest {
            return Err(CodecError::new(code::NODE_DIGEST, "证明节点摘要不符"));
        }
        Ok((digest, tag[0], body))
    }
}

/// 外层添加真实anchor/path、token和本次capture摘要，组成规范13字段响应。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub target_ref: ContentRef,
    pub unit: Unit,
    pub offset: u64,
    pub end: u64,
    pub total: u64,
    pub items: J,
    pub nodes: Vec<ProofNode>,
    pub done: bool,
}
impl Fragment {
    fn encoded_size(&self) -> Result<u64, CodecError> {
        #[derive(serde::Serialize)]
        struct View<'a> {
            target_ref: J,
            unit: &'a str,
            offset: String,
            end: String,
            total: String,
            items: &'a J,
            nodes: &'a [ProofNode],
            done: bool,
        }
        encoded_len(&View {
            target_ref: self.target_ref.to_transport_json(),
            unit: self.unit.name(),
            offset: self.offset.to_string(),
            end: self.end.to_string(),
            total: self.total.to_string(),
            items: &self.items,
            nodes: &self.nodes,
            done: self.done,
        })
    }
    pub fn fields(&self) -> serde_json::Map<String, J> {
        json!({"target_ref":self.target_ref.to_transport_json(),"unit":self.unit.name(),"offset":self.offset.to_string(),"end":self.end.to_string(),"total":self.total.to_string(),"items":self.items,"nodes":self.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),"done":self.done}).as_object().unwrap().clone()
    }
}

/// transport path 的唯一 exact 解析：field / index / json_text 三者只取其一。
pub fn parse_path(value: &J) -> Result<Vec<PathStep>, CodecError> {
    let rows = value.as_array().ok_or_else(|| err("path须为数组"))?;
    rows.iter()
        .map(|row| {
            let o = row
                .as_object()
                .filter(|o| o.len() == 1)
                .ok_or_else(|| err("path步骤须exact单字段"))?;
            if let Some(v) = o.get("field") {
                return v
                    .as_str()
                    .map(|v| PathStep::Field(v.into()))
                    .ok_or_else(|| err("field须为UTF-8字符串"));
            }
            if let Some(v) = o.get("index") {
                return Ok(PathStep::Index(parse_canonical_u64(
                    v.as_str().ok_or_else(|| err("index须为规范非负整数文本"))?,
                    "index",
                )?));
            }
            if o.get("json_text") == Some(&J::Bool(true)) {
                return Ok(PathStep::JsonText);
            }
            Err(err("未知path步骤或json_text非true"))
        })
        .collect()
}

/// 每片重新收齐其触碰到的证明，不能因调用方曾接纳过节点就省略祖先。
pub fn read(
    source: &mut dyn NodeSource,
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
) -> Result<Fragment, CodecError> {
    access.begin_first_read()?;
    if limit == 0
        || offset > MAX_FRAME_LEN
        || limit > MAX_FRAME_LEN
        || max_reply_bytes == 0
        || max_reply_bytes > MAX_FRAME_LEN
    {
        return Err(err("offset/limit越域或limit为零"));
    }
    let requested_end = offset
        .checked_add(limit)
        .ok_or_else(|| err("offset+limit溢出u64"))?;
    let mut source = Recorder {
        source,
        access,
        nodes: BTreeMap::new(),
        reply_bytes: 0,
        max_reply_bytes,
    };
    let target = path::lookup_path(&mut source, root, path)?;
    let total = match (target.kind, unit) {
        (Kind::Array | Kind::Object, Unit::Items) | (Kind::String | Kind::Number, Unit::Bytes) => {
            target.extent.ok_or_else(|| err("目标缺extent"))?
        }
        (Kind::Null | Kind::Bool | Kind::JsonText, Unit::Value) if limit == 1 => 1,
        _ => return Err(err("目标类型/unit/limit不匹配；公开值不允许模板占位")),
    };
    if offset > total {
        return Err(err("offset超过total"));
    }
    let end = requested_end.min(total);
    // 这里只预检构造的规范JSON大小，不记为已经发生的wire传输。
    // 外层anchor/path/token/envelope仍须受同一实际回复上界检查。
    let empty = Fragment {
        target_ref: target.clone(),
        unit,
        offset,
        end,
        total,
        items: if unit == Unit::Bytes {
            json!("")
        } else {
            json!([])
        },
        nodes: vec![],
        done: end == total,
    };
    source.reserve_reply(encoded_len(&J::Object(empty.fields()))?)?;
    path::check_range_root(&mut source, &target)?;
    let items = match target.kind {
        Kind::Array => {
            let mut result = Vec::new();
            for i in offset..end {
                let mut steps = path.to_vec();
                steps.push(PathStep::Index(i));
                let child = path::lookup_path(&mut source, root, &steps)?;
                public_header(&child)?;
                let item = child.to_transport_json();
                source.reserve_reply(encoded_len(&item)? + u64::from(!result.is_empty()))?;
                result.push(item);
            }
            J::Array(result)
        }
        Kind::Object => {
            let mut result = Vec::new();
            for i in offset..end {
                let key_ref = path::object_key_at(&mut source, &target, i)?;
                let key = source.read_key(&key_ref)?;
                let mut steps = path.to_vec();
                steps.push(PathStep::Field(key.clone()));
                let child = path::lookup_path(&mut source, root, &steps)?;
                public_header(&child)?;
                let item = json!({"key":key,"value_ref":child.to_transport_json()});
                source.reserve_reply(encoded_len(&item)? + u64::from(!result.is_empty()))?;
                result.push(item);
            }
            J::Array(result)
        }
        Kind::String | Kind::Number => {
            source.reserve_reply(
                (end - offset)
                    .checked_mul(2)
                    .ok_or_else(|| err("回复字节溢出"))?,
            )?;
            J::String(hex(&path::bytes_between(
                &mut source,
                &target,
                offset,
                end,
            )?))
        }
        _ => J::Array(if offset == end {
            vec![]
        } else {
            source.reserve_reply(encoded_len(&target.to_transport_json())?)?;
            vec![target.to_transport_json()]
        }),
    };
    source.access.pre_io_check()?;
    let nodes = source
        .nodes
        .into_iter()
        .map(|(digest, (tag, body))| ProofNode {
            root: root_string(&digest),
            tag: format!("{tag:02x}"),
            body_hex: hex(&body),
        })
        .collect();
    Ok(Fragment {
        target_ref: target,
        unit,
        offset,
        end,
        total,
        items,
        nodes,
        done: end == total,
    })
}

/// 只用本片自带节点重新走同一规范路径/范围；不从接收方旧缓存补缺证明。
/// 接纳全部提供节点时仍计本Job唯一节点。额外节点不因此取得anchor归属或完整语义证明。
pub fn verify(
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    fragment: &Fragment,
    max_reply_bytes: u64,
    job: &mut Job,
) -> Result<(), CodecError> {
    job.begin_first_read()?;
    if max_reply_bytes == 0 || max_reply_bytes > MAX_FRAME_LEN {
        return Err(err("回复上界越域"));
    }
    if fragment.encoded_size()? > max_reply_bytes {
        return Err(CodecError::new(REPLY_LIMIT, "片段字段超过实际回复上界"));
    }
    let mut nodes = MemoryNodes::new();
    let mut previous = None;
    for node in &fragment.nodes {
        job.pre_io_check()?;
        let (digest, tag, body) = node.decode()?;
        if previous.is_some_and(|p| p >= digest) {
            return Err(err("证明节点须按root升序且去重"));
        }
        job.accept_node(&digest, tag, &body)?;
        nodes.put(digest, tag, &body)?;
        previous = Some(digest);
    }
    let expected = read(
        &mut nodes,
        root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        job,
    )?;
    if fragment.target_ref != expected.target_ref
        || fragment.unit != expected.unit
        || fragment.offset != expected.offset
        || fragment.end != expected.end
        || fragment.total != expected.total
        || fragment.items != expected.items
        || fragment.done != expected.done
    {
        return Err(err("片段内容/目标/位置与已给根的范围证明不同"));
    }
    job.pre_io_check()
}

struct Recorder<'a> {
    source: &'a mut dyn NodeSource,
    access: &'a mut dyn Access,
    nodes: BTreeMap<NodeDigest, (u8, Vec<u8>)>,
    reply_bytes: u64,
    max_reply_bytes: u64,
}
impl Recorder<'_> {
    fn read_key(&mut self, reference: &ContentRef) -> Result<String, CodecError> {
        let remaining = self.max_reply_bytes - self.reply_bytes;
        // JSON引号以外至少需要原UTF-8字节数；声明已超限时不展开共享DAG。
        if reference.extent.unwrap_or(MAX_FRAME_LEN).saturating_add(2) > remaining {
            return Err(CodecError::new(REPLY_LIMIT, "对象键超过剩余回复上界"));
        }
        struct KeyBuffer {
            bytes: Vec<u8>,
            limit: u64,
            exceeded: bool,
        }
        impl std::io::Write for KeyBuffer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if (self.bytes.len() as u64).saturating_add(bytes.len() as u64) > self.limit {
                    self.exceeded = true;
                    return Err(std::io::Error::other("对象键JSON转义超过回复上界"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut buffer = KeyBuffer {
            bytes: vec![],
            limit: remaining,
            exceeded: false,
        };
        let result = render_canonical(self, reference, &mut buffer);
        if buffer.exceeded {
            return Err(CodecError::new(
                REPLY_LIMIT,
                "对象键JSON转义超过剩余回复上界",
            ));
        }
        result?;
        self.access.pre_io_check()?;
        // 编码缓冲及解析后的String各受同一显式字节上界；新增证明和最终item仍统一预留。
        serde_json::from_slice(&buffer.bytes)
            .map_err(|e| CodecError::new(code::UTF8, e.to_string()))
    }

    fn reserve_reply(&mut self, bytes: u64) -> Result<(), CodecError> {
        self.reply_bytes = self
            .reply_bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.max_reply_bytes)
            .ok_or_else(|| CodecError::new(REPLY_LIMIT, "片段字段/证明超过显式回复上界"))?;
        Ok(())
    }
}
impl NodeSource for Recorder<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.access.pre_io_check()?;
        let read = self.source.read(digest)?;
        if let NodeRead::Found { tag, body } = &read {
            self.access.accept_node(digest, *tag, body)?;
            if let Some(old) = self.nodes.get(digest) {
                if old.0 != *tag || old.1 != *body {
                    return Err(err("同根不同原件"));
                }
            } else {
                let proof = ProofNode {
                    root: root_string(digest),
                    tag: format!("{tag:02x}"),
                    body_hex: hex(body),
                };
                self.reserve_reply(encoded_len(&proof)? + u64::from(!self.nodes.is_empty()))?;
                self.nodes.insert(*digest, (*tag, body.clone()));
            }
        }
        self.access.pre_io_check()?;
        Ok(read)
    }

    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.access.pre_io_check()?;
        let read = self.source.read(digest)?;
        let checked = match read {
            NodeRead::Missing => None,
            NodeRead::Found { tag, body } => {
                let frame = self.access.accept_frame(digest, tag, body)?;
                if let Some(old) = self.nodes.get(digest) {
                    if old.0 != frame.tag() || old.1 != frame.body() {
                        return Err(err("同根不同原件"));
                    }
                } else {
                    let proof = ProofNode {
                        root: root_string(digest),
                        tag: format!("{:02x}", frame.tag()),
                        body_hex: hex(frame.body()),
                    };
                    self.reserve_reply(encoded_len(&proof)? + u64::from(!self.nodes.is_empty()))?;
                    self.nodes.insert(*digest, (frame.tag(), frame.body().to_vec()));
                }
                Some(frame)
            }
        };
        self.access.pre_io_check()?;
        Ok(checked)
    }
}

#[cfg(test)]
mod checked_frame_tests {
    use super::*;
    use crate::s_content::{frame_calls_on_thread, job::{Existing, JobLimits, DEADLINE}, MemoryNodes};
    use std::time::{Duration, Instant};

    #[test]
    fn recorder_keeps_old_accounting_when_deadline_lands_during_f() {
        let digest = frame(tag::NULL, &[]);
        let mut source = MemoryNodes::new();
        source.insert_unchecked(digest, tag::NULL, vec![]);
        let limits = JobLimits::from_canonical_json(&json!({
            "max_total_wire_bytes":"1024", "max_total_node_bytes":"1024",
            "max_total_nodes":"8", "max_total_records":"8", "max_total_requests":"8",
            "max_elapsed_ms":"1", "max_staged_bytes":"1024", "max_store_bytes":"1024"
        })).unwrap();
        for checked in [false, true] {
            let before = frame_calls_on_thread();
            let now = Instant::now();
            let mut job = Job::new(
                limits,
                Existing { store_bytes: 0, staged_bytes: 0 },
                Box::new(move || now + Duration::from_millis(u64::from(frame_calls_on_thread() > before))),
            ).unwrap();
            {
                let mut recorder = Recorder {
                    source: &mut source,
                    access: &mut job,
                    nodes: BTreeMap::new(),
                    reply_bytes: 0,
                    max_reply_bytes: 1024,
                };
                let result = if checked { recorder.checked_read(&digest).map(|_| ()) }
                    else { recorder.read(&digest).map(|_| ()) };
                assert_eq!(result.unwrap_err().code(), DEADLINE, "旧 post precheck 行为");
                assert_eq!(recorder.nodes.len(), 1, "原证明收集已完成");
            }
            assert_eq!(frame_calls_on_thread() - before, 1);
            assert_eq!(job.snapshot().nodes, 1, "F 后不得多加计账前期限门");
        }
    }
}
fn public_header(r: &ContentRef) -> Result<(), CodecError> {
    if r.kind == Kind::TemplateBatch {
        Err(CodecError::new(
            code::TEMPLATE_PLACEHOLDER,
            "公开片段不能泄漏本批占位",
        ))
    } else {
        Ok(())
    }
}
fn err(e: impl std::fmt::Display) -> CodecError {
    CodecError::new(INVALID, e.to_string())
}
fn encoded_len(value: &impl serde::Serialize) -> Result<u64, CodecError> {
    struct Count(u64);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(b.len() as u64)
                .ok_or_else(|| std::io::Error::other("encoded length overflow"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).map_err(|e| err(e))?;
    Ok(count.0)
}
fn exact(v: &J, keys: &[&str]) -> Result<(), CodecError> {
    if v.as_object()
        .is_some_and(|o| o.len() == keys.len() && o.keys().all(|k| keys.contains(&k.as_str())))
    {
        Ok(())
    } else {
        Err(err("字段集合非exact"))
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str) -> Result<Vec<u8>, CodecError> {
    if text.len() % 2 != 0 {
        return Err(err("hex长度须为偶数"));
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            Ok(hex_val(pair[0]).ok_or_else(|| err("hex须为小写"))? * 16
                + hex_val(pair[1]).ok_or_else(|| err("hex须为小写"))?)
        })
        .collect()
}
