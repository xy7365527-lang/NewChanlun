//! s-content 路径定位与本批占位（0x08）定点替换（#1394 采纳 C §5.4/§5.6）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! §5.4（S 发布清单与无循环派生）与 §5.6（片段、证明和有界完整读取）。
//!
//! **这是低层算子，不是任命书，也不是 publish-valid 证明**：本模块只做两件事——
//! ① 按 §5.6 的 path 形状（field / index / json_text）从给定根定位到目标值；
//! ② 只把**已定位的真实 0x08 空 body 占位**替换成本批 `batch3-` 值，并重写该路径的
//! 标量与祖先节点。
//!
//! 哪些位置「允许」出现本批占位（本批新生对象的 batch_id、本批首次观察的 batch_id、
//! catalog_evidence.batch_id、当前发布的 index_frontier 及其嵌套副本）由调用方凭
//! 独立出生／首次观察／代号／当前批次上下文逐项证明；本模块**不判断位置语义**，也不读取
//! manifest 的 batch_id 配方。写成功**不等于** publish 通过：调用方仍须对被替换后的根做
//! 完整闭包核验（`verify_template`）与跨根绑定核对，并按 §5.4 保证 0x08 不从最终公开逻辑值
//! 泄漏。
//!
//! 读写纪律（照实）：
//! - 每个读到的节点都按 F(tag, body) 重算摘要（`read_checked`）：缺节点、换字节即拒绝；
//!   根还逐项对 `ContentRef` 的 kind / extent / format / root 文本；
//! - 只沿路径触碰 O(树深 + 步骤数) 的节点：数组按 §5.2 的 k 规则下行，对象按 UTF-8 键序
//!   二分下行（每层用左右子树边界键核一遍规范键序，O(树深²) 读，仍与逻辑叶数无关）；
//!   普通定位不物化完整逻辑值；私有完整 root[] 枚举只收集成员 ContentRef，
//!   不按声明长度预分配，也不递归；
//! - 对象字段查找只读字段节点（0x14，64 字节 body）与键的字节块，不展开字段值子图；
//! - 替换只重写路径上的标量与祖先，共享子树（同摘要）逐位保留：未指定路径的节点摘要不变，
//!   旧根与其闭包不被改动；
//! - 替换先做只读通过（含目标形状与 batch_id 校验），通过后才写；拒绝时不写任何节点；
//! - 写面无事务：中途 `put` 失败可能留下**内容合法但未被引用**的节点，由调用方的存储策略清理。
//! - json_text 是**有上下文**的：经 `{json_text: true}` 进入后，其内部任意深度的值节点都不准
//!   再出现 0x07／0x08（与 `verify` 的 JsonInner 上下文同口径），本模块逐跳拒绝。
//!
//! 核验阶段（发布链里要分清）：
//! - `verify_template` 只是**中间态**核验：模板上下文仍允许 0x08，通过不等于可发布；
//! - 凡进入公开逻辑值的根（实际 snapshot／Delta／Q 投影／E-B 对象摘要等）最终必须以
//!   `verify`（Public 上下文）通过——0x08 不得从最终公开逻辑值泄漏。本模块只做定点替换，
//!   这次上下文切换与跨根绑定判断不在本模块内。
//!
//! 本模块不覆盖：Q 的 wire／分页／证明／资源计费（§5.6.1）、跨会话锚定与 Incomplete 语义、
//! 旧新 crosswalk。也不解析 transport 里的 path 文本：`{index: 规范非负整数文本}` 的解析与
//! 溢出拒绝在调用方（编码面已有 `parse_root`/`checked_frame_len` 同类判据）。

use super::{
    be_u64, body_shape_error, byte_chunk_shape_error, byte_extent_error, checked_frame_len, code,
    digest_at, frame, framed_len, header_extent, is_sequence_tag, length_overflow_error,
    read_checked, root_string, sequence_shape_error, split_index, tag, unsupported_value_tag_error,
    write_string_node, CodecError, ContentRef, Kind, LeafIter, NodeDigest, NodeSource, NodeStore,
    CHUNK_BYTES, CONTENT_FORMAT, MAX_FRAME_LEN,
};
use std::cmp::Ordering;

/// 本模块的稳定拒绝码（与 §5.1–5.3 的 content codec 码互不冲突）。
pub mod path_code {
    /// 对象里没有该字段（规范键序里不存在该键）。
    pub const PATH_FIELD_MISSING: &str = "path_field_missing";
    /// 数组下标越界：`index` 不小于该数组声明的元素数。
    pub const PATH_INDEX_RANGE: &str = "path_index_out_of_range";
    /// 步骤类型与当前节点类型不符（field 非对象／index 非数组／json_text 非文本视图）。
    pub const PATH_STEP_KIND: &str = "path_step_kind";
    /// 替换路径不允许进入 json_text 文本视图（§5.4：文本视图内部不得有占位）。
    pub const PATH_JSON_TEXT: &str = "path_json_text_forbidden";
    /// batch_id 不是 §5.3 的 `batch3-` + 64 位小写十六进制。
    pub const BATCH_ID: &str = "batch_id";
}

/// §5.6 的有序路径步骤；每步只取一种 exact 形状。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStep {
    /// `{field: UTF-8字段名}`：进入对象字段。
    Field(String),
    /// `{index: 规范非负整数}`：取数组元素（调用方负责 transport 文本的规范解析）。
    Index(u64),
    /// `{json_text: true}`：进入 §5.2 的明确 JSON 文本视图（内部值仍要显式路径）。
    JsonText,
}

/// 按 §5.6 的 path 从 `reference` 指出的根定位目标值。
///
/// 返回的目标 `ContentRef` 由**实际节点**的 tag/body 决定（kind、extent、root 逐项重算）；
/// 根另核调用方声明的四字段。只核路径触碰到的节点与序列形状：**不是全闭包核验**，
/// 也不是 publish-valid 证明（同 `content_ref_of` 的口径）。
pub fn lookup_path(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<ContentRef, CodecError> {
    let descent = navigate(source, reference, path, false)?;
    let kind =
        Kind::from_tag(descent.tag).ok_or_else(|| unsupported_value_tag_error(descent.tag))?;
    let extent = header_extent(descent.tag, &descent.body)?;
    Ok(ContentRef {
        format: CONTENT_FORMAT.to_string(),
        root: root_string(&descent.digest),
        kind,
        extent,
    })
}

/// 私有审计用的完整 root[] 枚举（#1394）：与旧逐 Index 的 Value 角色一致。
/// 每个逻辑序列位置核 count/k、每个实际读取核本次 F；同摘要的重复位置仍逐次读。
/// 栈只保存摘要、期望数量和基准位置，不保存 body、不跨调用复用、不展开成员子图。
/// Ref 不携带 JsonInner 上下文：此口不接完整 path，也不代替上层 public/template verify。
/// 调用方只可在明确的完整审计 BudgetNone 分支启用；Some/fragment 保留原读取门序。
#[doc(hidden)]
pub fn lookup_array_items(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
) -> Result<Vec<ContentRef>, CodecError> {
    let (count, root) = {
        let node = navigate(source, reference, &[], false)?;
        if node.tag != tag::ARRAY {
            return Err(step_kind_error("完整数组", "0x05 数组", node.tag));
        }
        (framed_len(&node.body, "数组")?, digest_at(&node.body, 8))
    };
    let mut stack = vec![(root, count, 0u64)];
    let mut result = Vec::new();
    while let Some((digest, count, base_index)) = stack.pop() {
        match read_seq_node(source, &digest, count)? {
            SeqNode::Empty => {}
            SeqNode::Branch { k, left, right } => {
                // 两侧实际出栈时各核期望数量；完整遍历不为每个 item 重读共同祖先。
                stack.push((right, count - k, base_index + k));
                stack.push((left, k, base_index));
            }
            SeqNode::Leaf(value) => {
                source.before_path_index(&[], base_index)?;
                let (t, body) = read_checked(source, &value)?;
                // 保留 descend_seq 的 M 叶拒绝码；其它内部节点沿旧 Value 角色拒绝。
                if is_sequence_tag(t) {
                    return Err(sequence_shape_error(format!(
                        "元素序列的叶不能又是序列节点（tag={t:02x}）"
                    )));
                }
                let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
                let extent = header_extent(t, &body)?;
                result.push(ContentRef::new(kind, &value, extent));
            }
        }
    }
    Ok(result)
}

/// 在一次规范序列遍历中读取固定 exact 字段集。各键按 UTF-8 字节顺序逐字对拍，
/// 根的四字段、每层 k 分解、字段节点及值头均沿既有判据核验；不展开值的子图。
/// 只按调用方字段清单分配，不按来源声明长度分配；结果不跨调用缓存。
/// 本口不代替全闭包或字段业务语义核验。
pub fn lookup_object_fields(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    names: &[&str],
) -> Result<std::collections::BTreeMap<String, ContentRef>, CodecError> {
    object_fields(source, reference, names, true)
}

/// 一次遍历对象字段序列，取出指定的小字段集合。其它字段保留但不展开其值。
/// 返回规模只由请求字段清单决定；这不是 exact schema 或完整闭包证明。
pub fn lookup_object_fields_subset(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    names: &[&str],
) -> Result<std::collections::BTreeMap<String, ContentRef>, CodecError> {
    object_fields(source, reference, names, false)
}

fn object_fields(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    names: &[&str],
    exact: bool,
) -> Result<std::collections::BTreeMap<String, ContentRef>, CodecError> {
    let node = navigate(source, reference, &[], false)?;
    if node.tag != tag::OBJECT {
        return Err(step_kind_error("对象字段集", "0x06 对象", node.tag));
    }
    let count = framed_len(&node.body, "对象")?;
    if exact && count != names.len() as u64 {
        return Err(sequence_shape_error("对象字段数与固定 exact 清单不同"));
    }
    let mut keys = names.to_vec();
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodecError::new(code::DUPLICATE_KEY, "exact 清单重复键"));
    }
    let mut result = std::collections::BTreeMap::new();
    let mut stack = vec![(digest_at(&node.body, 8), count)];
    let mut at = 0;
    let mut previous_key = None;
    // #1394 C §5：仅完整审计 None 的 Subset，在本次 DFS 中保留前/当前两短键。
    // 字节来自本位置实际读取且完整核验；不按摘要复用，也不携带值/闭包证明。
    let short_keys = !exact && source.audit_full_array_scan();
    let mut previous_bytes: Option<Vec<u8>> = None;
    while let Some((digest, count)) = stack.pop() {
        match read_seq_node(source, &digest, count)? {
            SeqNode::Empty => {}
            SeqNode::Branch { k, left, right } => {
                // 后续实际读到两子树并逐层检查期望数量；不用为每个字段再下行一遍。
                stack.push((right, count - k));
                stack.push((left, k));
            }
            SeqNode::Leaf(field) => {
                let (key, value) = read_field(source, &field)?;
                let current_bytes = if short_keys {
                    read_short_key(source, &key)?
                } else {
                    None
                };
                if let Some(previous) = previous_key.filter(|_| !exact) {
                    let order = match (&previous_bytes, &current_bytes) {
                        (Some(a), Some(b)) => a.cmp(b),
                        _ => cmp_key_nodes(source, &previous, &key)?,
                    };
                    match order {
                        Ordering::Less => {}
                        Ordering::Equal => {
                            return Err(CodecError::new(code::DUPLICATE_KEY, "对象重复键"))
                        }
                        Ordering::Greater => {
                            return Err(CodecError::new(
                                code::OBJECT_KEY_ORDER,
                                "对象键未按UTF-8顺序",
                            ))
                        }
                    }
                }
                previous_key = Some(key);
                previous_bytes = current_bytes;
                if !exact {
                    let Some(expected) = keys.get(at) else {
                        continue;
                    };
                    match cmp_field_key(source, &key, previous_bytes.as_deref(), expected)? {
                        Ordering::Less => continue,
                        Ordering::Greater => return Err(field_missing_error(expected)),
                        Ordering::Equal => {}
                    }
                }
                let expected = keys
                    .get(at)
                    .ok_or_else(|| sequence_shape_error("字段超出清单"))?;
                if cmp_field_key(source, &key, previous_bytes.as_deref(), expected)?
                    != Ordering::Equal
                {
                    return Err(field_missing_error(expected));
                }
                let reference = super::content_ref_of(source, &root_string(&value))?;
                result.insert((*expected).into(), reference);
                at += 1;
            }
        }
    }
    if at != keys.len() {
        return Err(sequence_shape_error("字段序列长度与清单不符"));
    }
    Ok(result)
}

/// 长键不具资格，仍交给既有流式比较；短键必须核唯一空/单叶分解、
/// 实际块角色、精确声明长度和全部 UTF-8。每个逻辑位置重新读当次 F。
fn read_short_key(
    source: &mut dyn NodeSource,
    key: &NodeDigest,
) -> Result<Option<Vec<u8>>, CodecError> {
    let (root, declared) = read_string_header(source, key)?;
    if declared > CHUNK_BYTES as u64 {
        return Ok(None);
    }
    let bytes = match read_seq_node(source, &root, u64::from(declared != 0))? {
        SeqNode::Empty => Vec::new(),
        SeqNode::Leaf(leaf) => {
            let (t, bytes) = read_checked(source, &leaf)?;
            check_chunk(t, &bytes)?;
            if bytes.len() as u64 != declared {
                return Err(byte_extent_error(bytes.len() as u64, declared));
            }
            bytes
        }
        SeqNode::Branch { .. } => unreachable!("短键期望块数只能是 0 或 1"),
    };
    std::str::from_utf8(&bytes).map_err(|_| super::utf8_error())?;
    Ok(Some(bytes))
}

fn cmp_field_key(
    source: &mut dyn NodeSource,
    key: &NodeDigest,
    bytes: Option<&[u8]>,
    expected: &str,
) -> Result<Ordering, CodecError> {
    match bytes {
        Some(bytes) => Ok(bytes.cmp(expected.as_bytes())),
        None => cmp_key_bytes(source, key, expected.as_bytes()),
    }
}

/// 片段读取共用的序列头检查；不把未读取的子图当作已完整验证。
pub(super) fn check_range_root(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
) -> Result<(), CodecError> {
    let node = navigate(source, reference, &[], false)?;
    let count = match reference.kind {
        Kind::Array | Kind::Object => framed_len(&node.body, "集合")?,
        Kind::String | Kind::Number => {
            framed_len(&node.body, "字节值")?.div_ceil(CHUNK_BYTES as u64)
        }
        Kind::JsonText => {
            // value 的 items 仍是 07；证明须带真实直接 inner header，供后续显式路径选 unit。
            // 只核此头的 F、JsonInner 角色和 extent，不展开其序列或正文。
            let (t, body) = read_value_node(source, &digest_at(&node.body, 0), true)?;
            header_extent(t, &body)?;
            return Ok(());
        }
        _ => return Ok(()),
    };
    let root = digest_at(&node.body, 8);
    if let SeqNode::Branch { k, left, right } = read_seq_node(source, &root, count)? {
        check_branch_children(source, &left, &right, count, k)?;
    }
    Ok(())
}

/// 按原规范序列位置定位对象键；调用方须有界展开键，值仍按完整路径核上下文。
pub(super) fn object_key_at(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    index: u64,
) -> Result<ContentRef, CodecError> {
    let node = navigate(source, reference, &[], false)?;
    if node.tag != tag::OBJECT {
        return Err(step_kind_error("对象位置", "0x06 对象", node.tag));
    }
    let count = framed_len(&node.body, "对象")?;
    let root = digest_at(&node.body, 8);
    let (_, field) = descend_seq(source, &root, count, index)?;
    let key = read_field(source, &field)?.0;
    for (at, before) in [
        (index.checked_sub(1), true),
        (index.checked_add(1).filter(|i| *i < count), false),
    ] {
        if let Some(at) = at {
            let (_, neighbor) = descend_seq(source, &root, count, at)?;
            let other = read_field(source, &neighbor)?.0;
            let order = if before {
                cmp_key_nodes(source, &other, &key)?
            } else {
                cmp_key_nodes(source, &key, &other)?
            };
            if order != Ordering::Less {
                return Err(CodecError::new(
                    code::OBJECT_KEY_ORDER,
                    "片段对象键非严格UTF-8顺序",
                ));
            }
        }
    }
    let reference = super::content_ref_of(source, &root_string(&key))?;
    if reference.kind != Kind::String {
        return Err(step_kind_error("对象键", "0x03 字符串", node.tag));
    }
    Ok(reference)
}

/// 只读取覆盖所求区间的规范字节块；不会线性扫描 offset 之前的叶。
pub(super) fn bytes_between(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    offset: u64,
    end: u64,
) -> Result<Vec<u8>, CodecError> {
    let node = navigate(source, reference, &[], false)?;
    if !matches!(reference.kind, Kind::String | Kind::Number) {
        return Err(step_kind_error("bytes", "字符串或数字", node.tag));
    }
    let total = framed_len(&node.body, "字节值")?;
    if offset > end || end > total {
        return Err(index_range_error(end, total));
    }
    let count = total.div_ceil(CHUNK_BYTES as u64);
    let root = digest_at(&node.body, 8);
    check_range_root(source, reference)?;
    let mut output = Vec::new();
    if offset == end {
        return Ok(output);
    }
    for index in offset / CHUNK_BYTES as u64..=(end - 1) / CHUNK_BYTES as u64 {
        let (_, leaf) = descend_seq(source, &root, count, index)?;
        let (tag, body) = read_checked(source, &leaf)?;
        check_chunk(tag, &body)?;
        let start = index * CHUNK_BYTES as u64;
        let expected = (total - start).min(CHUNK_BYTES as u64);
        if body.len() as u64 != expected {
            return Err(byte_extent_error(body.len() as u64, expected));
        }
        let lo = offset.saturating_sub(start) as usize;
        let hi = (end - start).min(expected) as usize;
        output.extend_from_slice(&body[lo..hi]);
    }
    Ok(output)
}

/// 把路径定位到的真实 0x08 空 body 占位替换为本批 `batch_id` 字符串，只重写该路径的标量
/// 与祖先节点。
///
/// 前置（调用方义务）：`path` 指向的位置确属 §5.4 允许的本批占位之一，且 `batch_id` 已由
/// manifest 配方实际求出。本函数只核形状：目标必须是 0x08 且 body 为空；`batch_id` 必须是
/// `batch3-` + 64 位小写十六进制；替换路径不得进入 json_text 文本视图。
///
/// 返回新根 `ContentRef`。**写成功不是 publish-valid**：请在调用方事务内对结果再跑完整闭包
/// 核验（`verify_template`）与跨根绑定核对。
pub fn replace_batch_placeholder(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
    batch_id: &str,
) -> Result<ContentRef, CodecError> {
    check_batch_id(batch_id)?;
    if path.iter().any(|step| matches!(step, PathStep::JsonText)) {
        return Err(CodecError::new(
            path_code::PATH_JSON_TEXT,
            "替换路径不允许进入 json_text 文本视图".to_string(),
        ));
    }

    // 只读通过：定位 + 全部形状核验都在写之前完成。
    let descent = navigate(&mut *store, reference, path, true)?;
    if descent.tag != tag::BATCH_SELF {
        return Err(CodecError::new(
            code::VALUE_KIND,
            format!(
                "替换目标必须是 0x08 模板占位，实得 tag=0x{:02x}（{}）",
                descent.tag,
                Kind::from_tag(descent.tag).map_or("内部节点", Kind::name)
            ),
        ));
    }
    if !descent.body.is_empty() {
        return Err(body_shape_error(
            descent.tag,
            &descent.body,
            "占位 body 必须为空",
        ));
    }

    // 写通过：新标量 → 自内向外重建路径祖先（共享子树原样引用）。
    let (mut digest, mut kind, mut extent) = write_batch_string(&mut *store, batch_id)?;
    for ancestor in descent.ancestors.iter().rev() {
        match ancestor {
            Ancestor::Array { declared, branches } => {
                let leaf = frame(tag::SEQ_LEAF, &digest);
                store.put(leaf, tag::SEQ_LEAF, &digest)?;
                let fields = rebuild_sequence(store, branches, leaf)?;
                let mut body = Vec::with_capacity(40);
                body.extend_from_slice(&checked_frame_len(*declared)?);
                body.extend_from_slice(&fields);
                digest = frame(tag::ARRAY, &body);
                store.put(digest, tag::ARRAY, &body)?;
                kind = Kind::Array;
                extent = Some(*declared);
            }
            Ancestor::Object {
                declared,
                branches,
                key,
            } => {
                let mut field_body = [0u8; 64];
                field_body[..32].copy_from_slice(key);
                field_body[32..].copy_from_slice(&digest);
                let field = frame(tag::OBJECT_FIELD, &field_body);
                store.put(field, tag::OBJECT_FIELD, &field_body)?;
                let leaf = frame(tag::SEQ_LEAF, &field);
                store.put(leaf, tag::SEQ_LEAF, &field)?;
                let fields = rebuild_sequence(store, branches, leaf)?;
                let mut body = Vec::with_capacity(40);
                body.extend_from_slice(&checked_frame_len(*declared)?);
                body.extend_from_slice(&fields);
                digest = frame(tag::OBJECT, &body);
                store.put(digest, tag::OBJECT, &body)?;
                kind = Kind::Object;
                extent = Some(*declared);
            }
        }
    }
    Ok(ContentRef {
        format: CONTENT_FORMAT.to_string(),
        root: root_string(&digest),
        kind,
        extent,
    })
}

// ── 定位 ────────────────────────────────────────────────────────────────────

/// 需要按新子摘要重建的一层祖先。
enum Ancestor {
    /// 数组：重建元素序列与 0x05 数组节点。
    Array {
        declared: u64,
        branches: Vec<BranchFrame>,
    },
    /// 对象：重建 0x14 字段节点（键不变）、字段序列与 0x06 对象节点。
    Object {
        declared: u64,
        branches: Vec<BranchFrame>,
        key: NodeDigest,
    },
}

/// 序列下行时记下的分支帧：从外到内（自根向下）排列，供替换时自内向外重建。
#[derive(Debug, Clone, Copy)]
struct BranchFrame {
    n: u64,
    left: NodeDigest,
    right: NodeDigest,
    went_left: bool,
}

/// 定位结果：目标节点的真实 (tag, body, 摘要) 与祖先重建帧。
struct Descent {
    digest: NodeDigest,
    tag: u8,
    body: Vec<u8>,
    ancestors: Vec<Ancestor>,
}

/// 沿 path 下行。`forbid_json_text` 为真时（替换）当场拒绝 json_text 步骤。
fn navigate(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    path: &[PathStep],
    forbid_json_text: bool,
) -> Result<Descent, CodecError> {
    let root = reference.digest()?;
    let (mut cur_tag, mut cur_body) = read_checked(source, &root)?;
    let root_kind = Kind::from_tag(cur_tag).ok_or_else(|| unsupported_value_tag_error(cur_tag))?;
    if root_kind != reference.kind {
        return Err(CodecError::new(
            code::KIND,
            format!(
                "ContentRef.kind={} 与真实节点 tag={cur_tag:02x}（{}）不符",
                reference.kind.name(),
                root_kind.name()
            ),
        ));
    }
    let root_extent = header_extent(cur_tag, &cur_body)?;
    if root_extent != reference.extent {
        return Err(CodecError::new(
            code::EXTENT,
            format!(
                "ContentRef.extent={:?} 与真实节点 header {:?} 不符",
                reference.extent, root_extent
            ),
        ));
    }

    let mut cur_digest = root;
    // json_text 上下文：进入文本视图后不再复位（其内部任意深度都受同一禁令）。
    let mut json_inner = false;
    let mut ancestors: Vec<Ancestor> = Vec::with_capacity(path.len().min(64));
    for (step_position, step) in path.iter().enumerate() {
        match step {
            PathStep::Field(name) => {
                if cur_tag != tag::OBJECT {
                    return Err(step_kind_error("field", "0x06 对象", cur_tag));
                }
                let declared = framed_len(&cur_body, "对象")?;
                let fields_root = digest_at(&cur_body, 8);
                let (branches, key, value) = find_field(source, &fields_root, declared, name)?;
                ancestors.push(Ancestor::Object {
                    declared,
                    branches,
                    key,
                });
                let (t, body) = read_value_node(source, &value, json_inner)?;
                (cur_tag, cur_body, cur_digest) = (t, body, value);
            }
            PathStep::Index(index) => {
                if cur_tag != tag::ARRAY {
                    return Err(step_kind_error("index", "0x05 数组", cur_tag));
                }
                let declared = framed_len(&cur_body, "数组")?;
                if *index >= declared {
                    return Err(index_range_error(*index, declared));
                }
                source.before_path_index(&path[..step_position], *index)?;
                let elements_root = digest_at(&cur_body, 8);
                let (branches, leaf) = descend_seq(source, &elements_root, declared, *index)?;
                ancestors.push(Ancestor::Array { declared, branches });
                let (t, body) = read_value_node(source, &leaf, json_inner)?;
                (cur_tag, cur_body, cur_digest) = (t, body, leaf);
            }
            PathStep::JsonText => {
                if forbid_json_text {
                    return Err(CodecError::new(
                        path_code::PATH_JSON_TEXT,
                        "替换路径不允许进入 json_text 文本视图".to_string(),
                    ));
                }
                if cur_tag != tag::JSON_TEXT {
                    return Err(step_kind_error("json_text", "0x07 文本视图", cur_tag));
                }
                // 防御式复述：进入后才置 json_inner，故此处通常不可达；语义与 verify 同口径。
                if json_inner {
                    return Err(json_inner_nested_text_error());
                }
                if cur_body.len() != 32 {
                    return Err(body_shape_error(
                        cur_tag,
                        &cur_body,
                        "body 必须是内部值的 32 字节摘要",
                    ));
                }
                let inner = digest_at(&cur_body, 0);
                let (t, body) = read_value_node(source, &inner, true)?;
                json_inner = true;
                (cur_tag, cur_body, cur_digest) = (t, body, inner);
            }
        }
    }
    Ok(Descent {
        digest: cur_digest,
        tag: cur_tag,
        body: cur_body,
        ancestors,
    })
}

/// 读节点并要求它是值节点（0x00..=0x08）。
///
/// `json_inner` 为真表示该位置在某个 json_text 文本视图内部：此处任意深度都不允
/// 0x07（嵌套文本视图）与 0x08（批次占位）——与 `verify` 的 `Flavor::JsonInner` 同口径。
fn read_value_node(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
    json_inner: bool,
) -> Result<(u8, Vec<u8>), CodecError> {
    let (t, body) = read_checked(source, digest)?;
    if Kind::from_tag(t).is_none() {
        return Err(unsupported_value_tag_error(t));
    }
    if json_inner {
        match t {
            tag::JSON_TEXT => return Err(json_inner_nested_text_error()),
            tag::BATCH_SELF => {
                return Err(CodecError::new(
                    code::JSON_TEXT_INTERNAL_KIND,
                    "json_text 内部不允许批次占位（0x08）".to_string(),
                ))
            }
            _ => {}
        }
    }
    Ok((t, body))
}

fn json_inner_nested_text_error() -> CodecError {
    CodecError::new(
        code::JSON_TEXT_INTERNAL_KIND,
        "json_text 内部不允许再出现文本视图（0x07）".to_string(),
    )
}

fn step_kind_error(step: &str, want: &str, got: u8) -> CodecError {
    CodecError::new(
        path_code::PATH_STEP_KIND,
        format!("{step} 步骤要求 {want} 节点，实得 tag=0x{got:02x}"),
    )
}

fn index_range_error(index: u64, count: u64) -> CodecError {
    CodecError::new(
        path_code::PATH_INDEX_RANGE,
        format!("数组位置 {index} 不小于声明元素数 {count}"),
    )
}

fn field_missing_error(name: &str) -> CodecError {
    CodecError::new(
        path_code::PATH_FIELD_MISSING,
        format!("对象里没有字段 `{name}`"),
    )
}

// ── 规范序列下行（只读面） ──────────────────────────────────────────────────

/// 已按期望叶数核过 header 的序列节点。
#[derive(Debug, Clone, Copy)]
enum SeqNode {
    Empty,
    Leaf(NodeDigest),
    Branch {
        k: u64,
        left: NodeDigest,
        right: NodeDigest,
    },
}

/// 单节点核验：摘要↔(tag, body) 重算、tag/长度与期望叶数、分支声明 n 等于期望叶数。
/// 只核这一个节点（同 `sequence.rs` 的触碰边界口径；更深子树不在本层核）。
fn read_seq_node(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
    count: u64,
) -> Result<SeqNode, CodecError> {
    if count > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "序列期望叶数 {count} 超过 2^63-1"
        )));
    }
    if count == 0 {
        // 空节点由规范唯一确定，但**不能拿摘要代原件**：已触碰的位置仍须读出实际节点，
        // 逐项核 (tag, body) 与摘要——缺原件一律拒绝（同 read_checked 口径）。
        let empty = frame(tag::SEQ_EMPTY, &[]);
        let (t, body) = read_checked(source, digest)?;
        if t != tag::SEQ_EMPTY || *digest != empty {
            return Err(sequence_shape_error(format!(
                "叶数为 0 的序列根必须是空序列节点 {}，实得 tag=0x{t:02x}（{}）",
                root_string(&empty),
                root_string(digest)
            )));
        }
        if !body.is_empty() {
            return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
        }
        return Ok(SeqNode::Empty);
    }
    let (t, body) = read_checked(source, digest)?;
    if count == 1 {
        if t != tag::SEQ_LEAF {
            return Err(sequence_shape_error(format!(
                "叶数为 1 的序列根必须是单叶序列节点 0x12，实得 tag=0x{t:02x}（{}）",
                root_string(digest)
            )));
        }
        if body.len() != 32 {
            return Err(sequence_shape_error(format!(
                "单叶序列 body 必须是 32 字节叶摘要，实得 {} 字节（{}）",
                body.len(),
                root_string(digest)
            )));
        }
        return Ok(SeqNode::Leaf(digest_at(&body, 0)));
    }
    if t != tag::SEQ_BRANCH {
        return Err(sequence_shape_error(format!(
            "叶数 {count} ≥ 2 的序列根必须是分支序列节点 0x13，实得 tag=0x{t:02x}（{}）",
            root_string(digest)
        )));
    }
    if body.len() != 72 {
        return Err(sequence_shape_error(format!(
            "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节），实得 {} 字节（{}）",
            body.len(),
            root_string(digest)
        )));
    }
    let declared = be_u64(&body);
    if declared > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "分支序列声明 n={declared} 超过 2^63-1（{}）",
            root_string(digest)
        )));
    }
    if declared != count {
        return Err(sequence_shape_error(format!(
            "分支序列声明 n={declared} 与期望叶数 {count} 不符（{}）",
            root_string(digest)
        )));
    }
    let k = split_index(count).expect("count ≥ 2 时 k 必然存在");
    Ok(SeqNode::Branch {
        k,
        left: digest_at(&body, 8),
        right: digest_at(&body, 40),
    })
}

/// 核分支两侧的直接子节点各自应有的叶数（k 规则在本地边界的实例）。
fn check_branch_children(
    source: &mut dyn NodeSource,
    left: &NodeDigest,
    right: &NodeDigest,
    n: u64,
    k: u64,
) -> Result<(), CodecError> {
    read_seq_node(source, left, k)?;
    read_seq_node(source, right, n - k)?;
    Ok(())
}

/// 按 §5.2 的 k 规则下行到叶位置 `index`（只走一条路径，不展开叶）。
fn descend_seq(
    source: &mut dyn NodeSource,
    root: &NodeDigest,
    count: u64,
    index: u64,
) -> Result<(Vec<BranchFrame>, NodeDigest), CodecError> {
    if index >= count {
        return Err(index_range_error(index, count));
    }
    let mut branches: Vec<BranchFrame> = Vec::new();
    let mut digest = *root;
    let mut at = index;
    let mut n = count;
    loop {
        match read_seq_node(source, &digest, n)? {
            SeqNode::Empty => return Err(index_range_error(index, count)),
            SeqNode::Leaf(leaf) => {
                // 叶本身若是序列节点，规范分解就已被破坏（与 verify 同口径）。
                let (t, _body) = read_checked(source, &leaf)?;
                if is_sequence_tag(t) {
                    return Err(sequence_shape_error(format!(
                        "元素序列的叶不能又是序列节点（tag={t:02x}）"
                    )));
                }
                return Ok((branches, leaf));
            }
            SeqNode::Branch { k, left, right } => {
                check_branch_children(source, &left, &right, n, k)?;
                if at < k {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: true,
                    });
                    digest = left;
                    n = k;
                } else {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: false,
                    });
                    digest = right;
                    at -= k;
                    n -= k;
                }
            }
        }
    }
}

/// 沿 k 规则下行到最左（`right=false`）／最右（`right=true`）叶。
fn boundary_leaf(
    source: &mut dyn NodeSource,
    root: &NodeDigest,
    count: u64,
    right: bool,
) -> Result<NodeDigest, CodecError> {
    let mut digest = *root;
    let mut n = count;
    loop {
        match read_seq_node(source, &digest, n)? {
            SeqNode::Empty => {
                return Err(sequence_shape_error(
                    "边界下行遇到空序列（期望叶数 ≥ 1）".to_string(),
                ))
            }
            SeqNode::Leaf(leaf) => return Ok(leaf),
            SeqNode::Branch {
                k,
                left,
                right: right_digest,
            } => {
                check_branch_children(source, &left, &right_digest, n, k)?;
                if right {
                    digest = right_digest;
                    n -= k;
                } else {
                    digest = left;
                    n = k;
                }
            }
        }
    }
}

/// 读 0x14 字段节点，返回 (键摘要, 值摘要)。
fn read_field(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
) -> Result<(NodeDigest, NodeDigest), CodecError> {
    let (t, body) = read_checked(source, digest)?;
    if t != tag::OBJECT_FIELD || body.len() != 64 {
        return Err(sequence_shape_error(format!(
            "对象字段序列的叶必须是 0x14 字段节点（64 字节 body），读到 tag={t:02x}（{} 字节）",
            body.len()
        )));
    }
    Ok((digest_at(&body, 0), digest_at(&body, 32)))
}

/// 按 UTF-8 键序在下行路径上查找字段；只读字段节点与键字节块，不展开字段值。
///
/// 每层用左子树末键与右子树首键核一遍规范键序（左 < 右），再以左子树末键决定去向：
/// `wanted <= 左末键` 走左，否则走右。查找不到即 `path_field_missing`。
fn find_field(
    source: &mut dyn NodeSource,
    root: &NodeDigest,
    count: u64,
    name: &str,
) -> Result<(Vec<BranchFrame>, NodeDigest, NodeDigest), CodecError> {
    let wanted = name.as_bytes();
    let mut branches: Vec<BranchFrame> = Vec::new();
    let mut digest = *root;
    let mut n = count;
    loop {
        if n == 0 {
            return Err(field_missing_error(name));
        }
        match read_seq_node(source, &digest, n)? {
            SeqNode::Empty => return Err(field_missing_error(name)),
            SeqNode::Leaf(field) => {
                let (key, value) = read_field(source, &field)?;
                return match cmp_key_bytes(source, &key, wanted)? {
                    Ordering::Equal => Ok((branches, key, value)),
                    _ => Err(field_missing_error(name)),
                };
            }
            SeqNode::Branch { k, left, right } => {
                check_branch_children(source, &left, &right, n, k)?;
                let left_max = boundary_leaf(source, &left, k, true)?;
                let right_min = boundary_leaf(source, &right, n - k, false)?;
                let left_key = read_field(source, &left_max)?.0;
                let right_key = read_field(source, &right_min)?.0;
                match cmp_key_nodes(source, &left_key, &right_key)? {
                    Ordering::Less => {}
                    Ordering::Equal => {
                        return Err(CodecError::new(
                            code::DUPLICATE_KEY,
                            "对象出现重复键（同一键摘要跨分支重复）".to_string(),
                        ))
                    }
                    Ordering::Greater => {
                        return Err(CodecError::new(
                            code::OBJECT_KEY_ORDER,
                            "对象键未按 UTF-8 字节升序".to_string(),
                        ))
                    }
                }
                if cmp_key_bytes(source, &left_key, wanted)? == Ordering::Less {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: false,
                    });
                    digest = right;
                    n -= k;
                } else {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: true,
                    });
                    digest = left;
                    n = k;
                }
            }
        }
    }
}

/// 读字符串节点 header，返回 (字节序列根, 声明字节数)。
fn read_string_header(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
) -> Result<(NodeDigest, u64), CodecError> {
    let (t, body) = read_checked(source, digest)?;
    if t != tag::STRING {
        return Err(CodecError::new(
            code::VALUE_KIND,
            format!("对象键必须是字符串节点（0x03），读到 tag={t:02x}"),
        ));
    }
    let declared = framed_len(&body, "字符串")?;
    Ok((digest_at(&body, 8), declared))
}

/// 键与目标字段名的字节序比较：首个差异即定序，不收集整串。
/// 读到全部字节且长度一致才算相等；键更长为 Greater、更短为 Less。
fn cmp_key_bytes(
    source: &mut dyn NodeSource,
    key: &NodeDigest,
    wanted: &[u8],
) -> Result<Ordering, CodecError> {
    let (bytes_root, declared) = read_string_header(source, key)?;
    let mut iter = LeafIter::new(bytes_root);
    let mut pos = 0usize;
    let mut total: u64 = 0;
    while let Some((lt, _leaf, chunk)) = iter.next(source)? {
        if lt != tag::BYTES_CHUNK {
            return Err(byte_chunk_shape_error(format!(
                "字节序列的叶必须是 0x10 块节点，读到 tag={lt:02x}"
            )));
        }
        if chunk.is_empty() || chunk.len() > CHUNK_BYTES {
            return Err(byte_chunk_shape_error(format!(
                "块长度 {} 必须落在 1..=4096",
                chunk.len()
            )));
        }
        total += chunk.len() as u64;
        for &byte in &chunk {
            if pos == wanted.len() {
                return Ok(Ordering::Greater);
            }
            match byte.cmp(&wanted[pos]) {
                Ordering::Equal => pos += 1,
                other => return Ok(other),
            }
        }
    }
    if total != declared {
        return Err(byte_extent_error(total, declared));
    }
    Ok(if pos == wanted.len() {
        Ordering::Equal
    } else {
        Ordering::Less
    })
}

/// 两个键摘要的字节序比较（规范键序判定用）：同摘要即同键，不读内容。
fn cmp_key_nodes(
    source: &mut dyn NodeSource,
    a: &NodeDigest,
    b: &NodeDigest,
) -> Result<Ordering, CodecError> {
    if a == b {
        return Ok(Ordering::Equal);
    }
    let (root_a, declared_a) = read_string_header(source, a)?;
    let (root_b, declared_b) = read_string_header(source, b)?;
    let mut iter_a = LeafIter::new(root_a);
    let mut iter_b = LeafIter::new(root_b);
    let mut buf_a: Vec<u8> = Vec::new();
    let mut buf_b: Vec<u8> = Vec::new();
    let mut pos_a = 0usize;
    let mut pos_b = 0usize;
    let mut total_a: u64 = 0;
    let mut total_b: u64 = 0;
    loop {
        if pos_a == buf_a.len() {
            match iter_a.next(source)? {
                Some((lt, _leaf, chunk)) => {
                    check_chunk(lt, &chunk)?;
                    total_a += chunk.len() as u64;
                    buf_a = chunk;
                    pos_a = 0;
                }
                None => buf_a.clear(),
            }
        }
        if pos_b == buf_b.len() {
            match iter_b.next(source)? {
                Some((lt, _leaf, chunk)) => {
                    check_chunk(lt, &chunk)?;
                    total_b += chunk.len() as u64;
                    buf_b = chunk;
                    pos_b = 0;
                }
                None => buf_b.clear(),
            }
        }
        match (pos_a < buf_a.len(), pos_b < buf_b.len()) {
            (false, false) => break,
            (false, true) => return Ok(Ordering::Less),
            (true, false) => return Ok(Ordering::Greater),
            (true, true) => {}
        }
        let x = buf_a[pos_a];
        let y = buf_b[pos_b];
        if x != y {
            return Ok(if x < y {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        pos_a += 1;
        pos_b += 1;
    }
    if total_a != declared_a {
        return Err(byte_extent_error(total_a, declared_a));
    }
    if total_b != declared_b {
        return Err(byte_extent_error(total_b, declared_b));
    }
    Ok(Ordering::Equal)
}

fn check_chunk(lt: u8, chunk: &[u8]) -> Result<(), CodecError> {
    if lt != tag::BYTES_CHUNK {
        return Err(byte_chunk_shape_error(format!(
            "字节序列的叶必须是 0x10 块节点，读到 tag={lt:02x}"
        )));
    }
    if chunk.is_empty() || chunk.len() > CHUNK_BYTES {
        return Err(byte_chunk_shape_error(format!(
            "块长度 {} 必须落在 1..=4096",
            chunk.len()
        )));
    }
    Ok(())
}

// ── 写入（只由替换调用） ────────────────────────────────────────────────────

/// 本批 batch_id 形状：`batch3-` + 64 位小写十六进制（§5.3 的 `batch3-` 前缀）。
fn check_batch_id(batch_id: &str) -> Result<(), CodecError> {
    let bad = |why: &str| {
        CodecError::new(
            path_code::BATCH_ID,
            format!("batch_id `{batch_id}` 非法：{why}"),
        )
    };
    let hex = batch_id
        .strip_prefix("batch3-")
        .ok_or_else(|| bad("缺少 `batch3-` 前缀"))?;
    if hex.len() != 64 {
        return Err(bad("十六进制长度必须是 64"));
    }
    if !hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(bad("只允许 0-9 与小写 a-f"));
    }
    Ok(())
}

/// 写 batch_id 字符串值：复用编码面的字符串节点写法（0x03 = U64 长度 + 字节序列根）。
fn write_batch_string(
    store: &mut dyn NodeStore,
    text: &str,
) -> Result<(NodeDigest, Kind, Option<u64>), CodecError> {
    let n = u64::try_from(text.len())
        .map_err(|_| length_overflow_error("字符串字节数超过 u64 表达".to_string()))?;
    let digest = write_string_node(&mut *store, text)?;
    Ok((digest, Kind::String, Some(n)))
}

/// 自内向外重建分支链：每层写入同一 n 的新 0x13 节点，另一侧子树原样引用。
fn rebuild_sequence(
    store: &mut dyn NodeStore,
    branches: &[BranchFrame],
    mut child: NodeDigest,
) -> Result<NodeDigest, CodecError> {
    for f in branches.iter().rev() {
        let mut body = [0u8; 72];
        body[..8].copy_from_slice(&checked_frame_len(f.n)?);
        if f.went_left {
            body[8..40].copy_from_slice(&child);
            body[40..72].copy_from_slice(&f.right);
        } else {
            body[8..40].copy_from_slice(&f.left);
            body[40..72].copy_from_slice(&child);
        }
        let digest = frame(tag::SEQ_BRANCH, &body);
        store.put(digest, tag::SEQ_BRANCH, &body)?;
        child = digest;
    }
    Ok(child)
}
