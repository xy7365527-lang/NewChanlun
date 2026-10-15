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

use super::ref_owner::RefOutput;
use super::{
    be_u64, body_shape_error, byte_chunk_shape_error, byte_extent_error, checked_frame_len, code,
    digest_at, frame, framed_len, header_extent, is_sequence_tag, length_overflow_error,
    root_string, sequence_shape_error, split_index, tag, unsupported_value_tag_error,
    write_string_node, CodecContext, CodecError, CodecFields, ContentRef, Kind, LeafIter,
    NodeDigest, NodeSource, NodeStore, VerifierBody, WorkVec, CHUNK_BYTES, CONTENT_FORMAT,
    MAX_FRAME_LEN,
};
use super::{OwnedRefFields, RefOwner, RefView};
use std::cmp::Ordering;

fn native_reference<R: RefOutput>(
    source: &CodecContext<'_, '_>,
    kind: Kind,
    digest: NodeDigest,
    extent: Option<u64>,
) -> Result<R, CodecError> {
    let permit = source.reserve_native(R::NATIVE_HEAP)?;
    let reference = R::from_checked(kind, digest, extent);
    if let Some(permit) = permit {
        permit.retain();
    }
    Ok(reference)
}

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

/// 借用步骤只改变所有者表示；导航、类型与 JsonInner 判据仍由同一 navigate 核执行。
#[derive(Clone, Copy, Debug)]
pub enum PathStepView<'a> {
    Field(&'a str),
    Index(u64),
    JsonText,
}
pub trait PathView {
    fn len(&self) -> usize;
    fn step(&self, index: usize) -> Option<PathStepView<'_>>;
    /// 普通公开source的既有路径回调保持原prefix；Closed borrowed路径不擦除成String。
    fn plain_prefix(&self, _end: usize) -> Option<&[PathStep]> {
        None
    }
}
impl PathView for [PathStep] {
    fn len(&self) -> usize {
        self.len()
    }
    fn step(&self, index: usize) -> Option<PathStepView<'_>> {
        self.get(index).map(|step| match step {
            PathStep::Field(s) => PathStepView::Field(s),
            PathStep::Index(i) => PathStepView::Index(*i),
            PathStep::JsonText => PathStepView::JsonText,
        })
    }
    fn plain_prefix(&self, end: usize) -> Option<&[PathStep]> {
        self.get(..end)
    }
}
impl PathView for &[PathStep] {
    fn len(&self) -> usize {
        (*self).len()
    }
    fn step(&self, index: usize) -> Option<PathStepView<'_>> {
        (*self).step(index)
    }
    fn plain_prefix(&self, end: usize) -> Option<&[PathStep]> {
        (*self).plain_prefix(end)
    }
}
impl<const N: usize> PathView for [PathStep; N] {
    fn len(&self) -> usize {
        N
    }
    fn step(&self, index: usize) -> Option<PathStepView<'_>> {
        self.as_slice().step(index)
    }
    fn plain_prefix(&self, end: usize) -> Option<&[PathStep]> {
        self.as_slice().plain_prefix(end)
    }
}
/// 保留原请求路径，不为每个Fragment item复制整份字段文本。
pub struct ExtendedPath<'a> {
    pub base: &'a dyn PathView,
    pub last: PathStepView<'a>,
}
impl PathView for ExtendedPath<'_> {
    fn len(&self) -> usize {
        self.base.len() + 1
    }
    fn step(&self, index: usize) -> Option<PathStepView<'_>> {
        if index == self.base.len() {
            Some(self.last)
        } else {
            self.base.step(index)
        }
    }
    fn plain_prefix(&self, end: usize) -> Option<&[PathStep]> {
        if end <= self.base.len() {
            self.base.plain_prefix(end)
        } else {
            None
        }
    }
}

/// 按 §5.6 的 path 从 `reference` 指出的根定位目标值。
///
/// 返回的目标 `ContentRef` 由**实际节点**的 tag/body 决定（kind、extent、root 逐项重算）；
/// 根另核调用方声明的四字段。只核路径触碰到的节点与序列形状：**不是全闭包核验**，
/// 也不是 publish-valid 证明（同 `content_ref_of` 的口径）。
pub(super) fn lookup_path_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<ContentRef, CodecError> {
    lookup_path_output(source, reference.into(), path)
}
pub(super) fn lookup_path_owned_ctx(
    source: &mut super::ClosedCapturedOwner<'_>,
    reference: RefView<'_>,
    path: &[PathStep],
) -> Result<RefOwner, CodecError> {
    lookup_path_output(&mut CodecContext::Closed(source), reference, path)
}
pub(super) fn lookup_path_output<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    path: &[PathStep],
) -> Result<R, CodecError> {
    lookup_path_view_output(source, reference, &path, false)
}
pub(super) fn lookup_path_view_output<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    path: &dyn PathView,
    forbid_json_text: bool,
) -> Result<R, CodecError> {
    let operation_work = source.work_budget();
    let operation_result = lookup_path_view_output_inner(source, reference, path, forbid_json_text);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn lookup_path_view_output_inner<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    path: &dyn PathView,
    forbid_json_text: bool,
) -> Result<R, CodecError> {
    let descent = navigate_path_view(source, reference, path, forbid_json_text)?;
    let kind =
        Kind::from_tag(descent.tag).ok_or_else(|| unsupported_value_tag_error(descent.tag))?;
    let extent = header_extent(descent.tag, &descent.body)?;
    native_reference(source, kind, descent.digest, extent)
}

/// 单层可选字段仍使用相同键序搜索核；不存在是正常值，真实错误照常结束工作域。
pub(super) fn lookup_optional_field_output<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    name: &str,
) -> Result<Option<R>, CodecError> {
    let work = source.work_budget();
    let result = (|| {
        let root = navigate_path_view(source, reference, &[], false)?;
        if root.tag != tag::OBJECT {
            return Err(step_kind_error("field", "0x06 对象", root.tag));
        }
        let declared = framed_len(&root.body, "对象")?;
        let fields = digest_at(&root.body, 8);
        let Some((_, _, value)) = find_field_optional(source, &fields, declared, name)? else {
            return Ok(None);
        };
        let (tag, body) = read_value_node(source, &value, false)?;
        let kind = Kind::from_tag(tag).ok_or_else(|| unsupported_value_tag_error(tag))?;
        native_reference(source, kind, value, header_extent(tag, &body)?).map(Some)
    })();
    if let Some(work) = work {
        work.observe(&result);
    }
    result
}

pub fn lookup_optional_field_refview(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    name: &str,
) -> Result<Option<ContentRef>, CodecError> {
    lookup_optional_field_output(&mut CodecContext::Source(source), reference, name)
}

/// 私有审计用的完整 root[] 枚举（#1394）：与旧逐 Index 的 Value 角色一致。
/// 每个逻辑序列位置核 count/k、每个实际读取核本次 F；同摘要的重复位置仍逐次读。
/// 栈只保存摘要、期望数量和基准位置，不保存 body、不跨调用复用、不展开成员子图。
/// Ref 不携带 JsonInner 上下文：此口不接完整 path，也不代替上层 public/template verify。
/// 调用方只可在明确的完整审计 BudgetNone 分支启用；Some/fragment 保留原读取门序。
#[doc(hidden)]
pub(super) fn lookup_array_items_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
) -> Result<WorkVec<ContentRef>, CodecError> {
    lookup_array_output(source, reference.into())
}
pub(super) fn lookup_array_items_owned_ctx(
    source: &mut super::ClosedCapturedOwner<'_>,
    reference: RefView<'_>,
) -> Result<WorkVec<RefOwner>, CodecError> {
    lookup_array_output(&mut CodecContext::Closed(source), reference)
}
pub(super) fn lookup_array_output<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
) -> Result<WorkVec<R>, CodecError> {
    let operation_work = source.work_budget();
    let operation_result = lookup_array_output_inner(source, reference);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn lookup_array_output_inner<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
) -> Result<WorkVec<R>, CodecError> {
    let (count, root) = {
        let node = navigate_view(source, reference, &[], false)?;
        if node.tag != tag::ARRAY {
            return Err(step_kind_error("完整数组", "0x05 数组", node.tag));
        }
        (framed_len(&node.body, "数组")?, digest_at(&node.body, 8))
    };
    let budget = source.work_budget();
    let mut stack = WorkVec::with_work_capacity(1, budget.as_ref())?;
    stack.push((root, count, 0u64))?;
    let mut result = WorkVec::new(source.budget().as_ref());
    while let Some((digest, count, base_index)) = stack.pop() {
        match read_seq_node(source, &digest, count)? {
            SeqNode::Empty => {}
            SeqNode::Branch { k, left, right } => {
                // 两侧实际出栈时各核期望数量；完整遍历不为每个 item 重读共同祖先。
                stack.push((right, count - k, base_index + k))?;
                stack.push((left, k, base_index))?;
            }
            SeqNode::Leaf(value) => {
                source.before_path_index(&[], base_index)?;
                let (t, body) = source.read(&value)?;
                // 保留 descend_seq 的 M 叶拒绝码；其它内部节点沿旧 Value 角色拒绝。
                if is_sequence_tag(t) {
                    return Err(sequence_shape_error(format!(
                        "元素序列的叶不能又是序列节点（tag={t:02x}）"
                    )));
                }
                let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
                let extent = header_extent(t, &body)?;
                source.plain_vec_room(&mut result, 1)?;
                result.push(native_reference(source, kind, value, extent)?)?;
            }
        }
    }
    Ok(result)
}

/// 在一次规范序列遍历中读取固定 exact 字段集。各键按 UTF-8 字节顺序逐字对拍，
/// 根的四字段、每层 k 分解、字段节点及值头均沿既有判据核验；不展开值的子图。
/// 只按调用方字段清单分配，不按来源声明长度分配；结果不跨调用缓存。
/// 本口不代替全闭包或字段业务语义核验。
pub(super) fn lookup_object_fields_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    names: &[&str],
) -> Result<CodecFields, CodecError> {
    object_fields(source, reference, names, true)
}

/// 一次遍历对象字段序列，取出指定的小字段集合。其它字段保留但不展开其值。
/// 返回规模只由请求字段清单决定；这不是 exact schema 或完整闭包证明。
pub(super) fn lookup_object_fields_subset_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    names: &[&str],
) -> Result<CodecFields, CodecError> {
    object_fields(source, reference, names, false)
}

pub(super) fn lookup_object_fields_owned_ctx<'names>(
    source: &mut super::ClosedCapturedOwner<'_>,
    reference: RefView<'_>,
    names: &[&'names str],
    exact: bool,
) -> Result<OwnedRefFields<'names>, CodecError> {
    object_fields_output(&mut CodecContext::Closed(source), reference, names, exact)
}
/// 固定业务 schema 的调用局部结果；字段名借用，引用为无堆 RefOwner。
/// 仅改变结果表示，不携带闭包/事务资格，不得作为跨代成功缓存。
#[doc(hidden)]
pub struct FixedFields<'names> {
    entries: [Option<(&'names str, RefOwner)>; 15],
    len: usize,
}
impl FixedFields<'_> {
    pub fn get(&self, name: &str) -> Option<&RefOwner> {
        self.entries[..self.len].iter().find_map(|entry| {
            entry
                .as_ref()
                .filter(|(key, _)| *key == name)
                .map(|(_, r)| r)
        })
    }
    pub fn get_required(&self, name: &str) -> Result<&RefOwner, CodecError> {
        self.get(name).ok_or_else(|| field_missing_error(name))
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
impl<'names> FieldOutput<'names> for FixedFields<'names> {
    type Reference = RefOwner;
    fn new(_: Option<&super::capture_allocation::OwnedBudget>) -> Result<Self, CodecError> {
        Ok(Self {
            entries: [None; 15],
            len: 0,
        })
    }
    fn append(&mut self, name: &'names str, reference: RefOwner) -> Result<(), CodecError> {
        if self.len == self.entries.len() {
            return Err(sequence_shape_error("固定字段结果超过15字段"));
        }
        if self.len > 0 && self.entries[self.len - 1].as_ref().unwrap().0 >= name {
            return Err(CodecError::static_detail(
                code::OBJECT_KEY_ORDER,
                "codec字段结果追加必须严格既有canonical序",
            ));
        }
        self.entries[self.len] = Some((name, reference));
        self.len += 1;
        Ok(())
    }
}
/// 复用同一个 exact 字段判据，只把结果保存于最多15个栈槽。
/// 上界在任何来源读取前检查；既有 keys/DFS 瞬态工作集未更换或冒称 Owned。
#[doc(hidden)]
pub fn lookup_object_fields_fixed<'names>(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    names: &[&'names str],
) -> Result<FixedFields<'names>, CodecError> {
    if names.len() > 15 {
        return Err(sequence_shape_error("固定字段结果超过15字段"));
    }
    object_fields_output(&mut CodecContext::Source(source), reference, names, true)
}

/// 仅用于按调用方原有顺序读取metadata：缺字段不在扫描顺序提前报错。
/// 不授予exact schema资格；调用方仍须在原晚核验点另跑exact入口。
struct MetadataFixedFields<'names>(FixedFields<'names>);
impl<'names> FieldOutput<'names> for MetadataFixedFields<'names> {
    const DEFER_MISSING: bool = true;
    type Reference = RefOwner;
    fn new(budget: Option<&super::capture_allocation::OwnedBudget>) -> Result<Self, CodecError> {
        FixedFields::new(budget).map(Self)
    }
    fn append(&mut self, name: &'names str, reference: RefOwner) -> Result<(), CodecError> {
        self.0.append(name, reference)
    }
}
/// 单次收集最多15个metadata字段，缺失项由get_required按消费顺序拒绝。
/// 只改变查找结果表示；不代替fresh闭包、exact schema或wire后重新读取。
#[doc(hidden)]
pub fn lookup_object_metadata_fixed<'names>(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    names: &[&'names str],
) -> Result<FixedFields<'names>, CodecError> {
    if names.len() > 15 {
        return Err(sequence_shape_error("固定字段结果超过15字段"));
    }
    object_fields_output::<MetadataFixedFields<'names>>(
        &mut CodecContext::Source(source),
        reference,
        names,
        false,
    )
    .map(|fields| fields.0)
}

pub(super) trait FieldOutput<'names>: Sized {
    const DEFER_MISSING: bool = false;
    type Reference: RefOutput;
    fn new(budget: Option<&super::capture_allocation::OwnedBudget>) -> Result<Self, CodecError>;
    fn append(&mut self, name: &'names str, reference: Self::Reference) -> Result<(), CodecError>;
}
impl<'names> FieldOutput<'names> for CodecFields {
    type Reference = ContentRef;
    fn new(budget: Option<&super::capture_allocation::OwnedBudget>) -> Result<Self, CodecError> {
        Ok(CodecFields::new(budget))
    }
    fn append(&mut self, name: &'names str, reference: ContentRef) -> Result<(), CodecError> {
        CodecFields::append(self, name, reference)
    }
}
impl<'names> FieldOutput<'names> for OwnedRefFields<'names> {
    type Reference = RefOwner;
    fn new(budget: Option<&super::capture_allocation::OwnedBudget>) -> Result<Self, CodecError> {
        budget.map(OwnedRefFields::new).ok_or_else(|| {
            CodecError::static_detail(
                super::capture_allocation::INCOMPLETE,
                "owned fields必须来自实际capture预算",
            )
        })
    }
    fn append(&mut self, name: &'names str, reference: RefOwner) -> Result<(), CodecError> {
        OwnedRefFields::append(self, name, reference)
    }
}
fn object_fields(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    names: &[&str],
    exact: bool,
) -> Result<CodecFields, CodecError> {
    object_fields_output(source, reference.into(), names, exact)
}
pub(super) fn object_fields_output<'names, O: FieldOutput<'names>>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    names: &[&'names str],
    exact: bool,
) -> Result<O, CodecError> {
    let operation_work = source.work_budget();
    let operation_result = object_fields_output_inner(source, reference, names, exact);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn object_fields_output_inner<'names, O: FieldOutput<'names>>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    names: &[&'names str],
    exact: bool,
) -> Result<O, CodecError> {
    let node = navigate_view(source, reference, &[], false)?;
    if node.tag != tag::OBJECT {
        return Err(step_kind_error("对象字段集", "0x06 对象", node.tag));
    }
    let count = framed_len(&node.body, "对象")?;
    if exact && count != names.len() as u64 {
        return Err(sequence_shape_error("对象字段数与固定 exact 清单不同"));
    }
    let budget = source.work_budget();
    let mut keys = WorkVec::with_work_capacity(names.len(), budget.as_ref())?;
    keys.extend_from_slice(names)?;
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodecError::new(code::DUPLICATE_KEY, "exact 清单重复键"));
    }
    let mut result = O::new(source.budget().as_ref())?;
    let mut stack = WorkVec::with_work_capacity(1, budget.as_ref())?;
    stack.push((digest_at(&node.body, 8), count))?;
    let mut at = 0;
    let mut previous_key = None;
    // #1394 C §5：仅完整审计 None 的 Subset，在本次 DFS 中保留前/当前两短键。
    // 字节来自本位置实际读取且完整核验；不按摘要复用，也不携带值/闭包证明。
    let short_keys = !exact && source.audit_full_array_scan();
    let mut previous_bytes: Option<VerifierBody<'_>> = None;
    while let Some((digest, count)) = stack.pop() {
        match read_seq_node(source, &digest, count)? {
            SeqNode::Empty => {}
            SeqNode::Branch { k, left, right } => {
                // 后续实际读到两子树并逐层检查期望数量；不用为每个字段再下行一遍。
                stack.push((right, count - k))?;
                stack.push((left, k))?;
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
                        (Some(a), Some(b)) => a[..].cmp(&b[..]),
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
                    // Metadata输出仅延迟缺失诊断；原Subset仍在相同第一处失败。
                    // 不复制字段/键判据，也不在任何失败后走另一条查找路径。
                    let mut matched = false;
                    while let Some(expected) = keys.get(at) {
                        match cmp_field_key(source, &key, previous_bytes.as_deref(), expected)? {
                            Ordering::Less => break,
                            Ordering::Greater if O::DEFER_MISSING => at += 1,
                            Ordering::Greater => return Err(field_missing_error(expected)),
                            Ordering::Equal => {
                                matched = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        continue;
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
                let (kind, extent) = reference_header(source, &value)?;
                let reference = native_reference(source, kind, value, extent)?;
                // std BTree B=6；一个字段保留一个完整节点，上界含至多
                // log2(n)+2 次分裂/新根。键正文另外全额计费。
                let height = usize::BITS as usize - (at + 1).leading_zeros() as usize;
                let node = 11 * std::mem::size_of::<(String, ContentRef)>()
                    + 16 * std::mem::size_of::<usize>();
                let field_bytes = node
                    .checked_mul(height + 2)
                    .and_then(|n| n.checked_add(expected.len()))
                    .ok_or_else(|| {
                        source.work_budget().map_or_else(
                            super::capture_allocation::work::allocation_error,
                            |work| {
                                work.native_failure(
                                    super::capture_allocation::scratch::ScratchFailure::Budget,
                                )
                            },
                        )
                    })?;
                let permit = source.reserve_native(field_bytes)?;
                result.append(expected, reference)?;
                if let Some(permit) = permit {
                    permit.retain();
                }
                at += 1;
            }
        }
    }
    if at != keys.len() && !O::DEFER_MISSING {
        return Err(sequence_shape_error("字段序列长度与清单不符"));
    }
    Ok(result)
}

/// 长键不具资格，仍交给既有流式比较；短键必须核唯一空/单叶分解、
/// 实际块角色、精确声明长度和全部 UTF-8。每个逻辑位置重新读当次 F。
fn read_short_key<'capture>(
    source: &mut CodecContext<'_, 'capture>,
    key: &NodeDigest,
) -> Result<Option<VerifierBody<'capture>>, CodecError> {
    let (root, declared) = read_string_header(source, key)?;
    if declared > CHUNK_BYTES as u64 {
        return Ok(None);
    }
    let bytes = match read_seq_node(source, &root, u64::from(declared != 0))? {
        SeqNode::Empty => VerifierBody::empty(),
        SeqNode::Leaf(leaf) => {
            let (t, bytes) = source.read(&leaf)?;
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
    source: &mut CodecContext<'_, '_>,
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
pub(super) fn check_range_root_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
) -> Result<(), CodecError> {
    check_range_root_view_ctx(source, reference.into())
}
pub(super) fn check_range_root_view_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
) -> Result<(), CodecError> {
    let node = navigate_view(source, reference, &[], false)?;
    let count = match reference.kind() {
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
pub(super) fn object_key_at_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    index: u64,
) -> Result<ContentRef, CodecError> {
    object_key_at_view_ctx(source, reference.into(), index)
}
pub(super) fn object_key_at_view_ctx<R: RefOutput>(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    index: u64,
) -> Result<R, CodecError> {
    let node = navigate_view(source, reference, &[], false)?;
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
    let (kind, extent) = reference_header(source, &key)?;
    if kind != Kind::String {
        return Err(step_kind_error("对象键", "0x03 字符串", node.tag));
    }
    native_reference(source, kind, key, extent)
}

/// 只读取覆盖所求区间的规范字节块；不会线性扫描 offset 之前的叶。
pub(super) fn bytes_between_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    offset: u64,
    end: u64,
) -> Result<WorkVec<u8>, CodecError> {
    bytes_between_view_ctx(source, reference.into(), offset, end)
}
pub(super) fn bytes_between_view_ctx(
    source: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    offset: u64,
    end: u64,
) -> Result<WorkVec<u8>, CodecError> {
    let node = navigate_view(source, reference, &[], false)?;
    if !matches!(reference.kind(), Kind::String | Kind::Number) {
        return Err(step_kind_error("bytes", "字符串或数字", node.tag));
    }
    let total = framed_len(&node.body, "字节值")?;
    if offset > end || end > total {
        return Err(index_range_error(end, total));
    }
    let count = total.div_ceil(CHUNK_BYTES as u64);
    let root = digest_at(&node.body, 8);
    check_range_root_view_ctx(source, reference)?;
    let mut output = WorkVec::new(source.budget().as_ref());
    if offset == end {
        return Ok(output);
    }
    for index in offset / CHUNK_BYTES as u64..=(end - 1) / CHUNK_BYTES as u64 {
        let (_, leaf) = descend_seq(source, &root, count, index)?;
        let (tag, body) = source.read(&leaf)?;
        check_chunk(tag, &body)?;
        let start = index * CHUNK_BYTES as u64;
        let expected = (total - start).min(CHUNK_BYTES as u64);
        if body.len() as u64 != expected {
            return Err(byte_extent_error(body.len() as u64, expected));
        }
        let lo = offset.saturating_sub(start) as usize;
        let hi = (end - start).min(expected) as usize;
        source.plain_vec_room(&mut output, hi - lo)?;
        output.extend_from_slice(&body[lo..hi])?;
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
pub(super) fn replace_batch_placeholder_ctx(
    store: &mut CodecContext<'_, '_>,
    reference: &ContentRef,
    path: &[PathStep],
    batch_id: &str,
) -> Result<ContentRef, CodecError> {
    replace_output(store, reference.into(), path, batch_id)
}
pub(super) fn replace_output<R: RefOutput>(
    store: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    path: &[PathStep],
    batch_id: &str,
) -> Result<R, CodecError> {
    let operation_work = store.work_budget();
    let operation_result = replace_output_inner(store, reference, path, batch_id);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn replace_output_inner<R: RefOutput>(
    store: &mut CodecContext<'_, '_>,
    reference: RefView<'_>,
    path: &[PathStep],
    batch_id: &str,
) -> Result<R, CodecError> {
    check_batch_id(batch_id)?;
    if path.iter().any(|step| matches!(step, PathStep::JsonText)) {
        return Err(CodecError::new(
            path_code::PATH_JSON_TEXT,
            "替换路径不允许进入 json_text 文本视图".to_string(),
        ));
    }

    // 只读通过：定位 + 全部形状核验都在写之前完成。
    let descent = navigate_view(&mut *store, reference, path, true)?;
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
                let leaf = store.emit(tag::SEQ_LEAF, &digest)?;
                let fields = rebuild_sequence(store, branches, leaf)?;
                let budget = store.work_budget();
                let mut body = WorkVec::with_work_capacity(40, budget.as_ref())?;
                body.extend_from_slice(&checked_frame_len(*declared)?)?;
                body.extend_from_slice(&fields)?;
                digest = store.emit(tag::ARRAY, &body)?;
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
                let field = store.emit(tag::OBJECT_FIELD, &field_body)?;
                let leaf = store.emit(tag::SEQ_LEAF, &field)?;
                let fields = rebuild_sequence(store, branches, leaf)?;
                let budget = store.work_budget();
                let mut body = WorkVec::with_work_capacity(40, budget.as_ref())?;
                body.extend_from_slice(&checked_frame_len(*declared)?)?;
                body.extend_from_slice(&fields)?;
                digest = store.emit(tag::OBJECT, &body)?;
                kind = Kind::Object;
                extent = Some(*declared);
            }
        }
    }
    native_reference(store, kind, digest, extent)
}

// ── 定位 ────────────────────────────────────────────────────────────────────

/// 需要按新子摘要重建的一层祖先。
enum Ancestor {
    /// 数组：重建元素序列与 0x05 数组节点。
    Array {
        declared: u64,
        branches: WorkVec<BranchFrame>,
    },
    /// 对象：重建 0x14 字段节点（键不变）、字段序列与 0x06 对象节点。
    Object {
        declared: u64,
        branches: WorkVec<BranchFrame>,
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
struct Descent<'capture> {
    digest: NodeDigest,
    tag: u8,
    body: VerifierBody<'capture>,
    ancestors: WorkVec<Ancestor>,
}

/// 沿 path 下行。`forbid_json_text` 为真时（替换）当场拒绝 json_text 步骤。
fn navigate<'capture>(
    source: &mut CodecContext<'_, 'capture>,
    reference: &ContentRef,
    path: &[PathStep],
    forbid_json_text: bool,
) -> Result<Descent<'capture>, CodecError> {
    navigate_view(source, reference.into(), path, forbid_json_text)
}
fn navigate_view<'capture>(
    source: &mut CodecContext<'_, 'capture>,
    reference: RefView<'_>,
    path: &[PathStep],
    forbid_json_text: bool,
) -> Result<Descent<'capture>, CodecError> {
    navigate_path_view(source, reference, &path, forbid_json_text)
}
fn navigate_path_view<'capture>(
    source: &mut CodecContext<'_, 'capture>,
    reference: RefView<'_>,
    path: &dyn PathView,
    forbid_json_text: bool,
) -> Result<Descent<'capture>, CodecError> {
    let root = reference.digest()?;
    let (mut cur_tag, mut cur_body) = source.read(&root)?;
    let root_kind = Kind::from_tag(cur_tag).ok_or_else(|| unsupported_value_tag_error(cur_tag))?;
    if root_kind != reference.kind() {
        return Err(CodecError::new(
            code::KIND,
            format!(
                "ContentRef.kind={} 与真实节点 tag={cur_tag:02x}（{}）不符",
                reference.kind().name(),
                root_kind.name()
            ),
        ));
    }
    let root_extent = header_extent(cur_tag, &cur_body)?;
    if root_extent != reference.extent() {
        return Err(CodecError::new(
            code::EXTENT,
            format!(
                "ContentRef.extent={:?} 与真实节点 header {:?} 不符",
                reference.extent(),
                root_extent
            ),
        ));
    }

    let mut cur_digest = root;
    // json_text 上下文：进入文本视图后不再复位（其内部任意深度都受同一禁令）。
    let mut json_inner = false;
    let budget = source.work_budget();
    let mut ancestors: WorkVec<Ancestor> =
        WorkVec::with_work_capacity(path.len().min(64), budget.as_ref())?;
    for step_position in 0..path.len() {
        match path
            .step(step_position)
            .ok_or_else(|| sequence_shape_error("借用path步骤缺失"))?
        {
            PathStepView::Field(name) => {
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
                })?;
                let (t, body) = read_value_node(source, &value, json_inner)?;
                (cur_tag, cur_body, cur_digest) = (t, body, value);
            }
            PathStepView::Index(index) => {
                if cur_tag != tag::ARRAY {
                    return Err(step_kind_error("index", "0x05 数组", cur_tag));
                }
                let declared = framed_len(&cur_body, "数组")?;
                if index >= declared {
                    return Err(index_range_error(index, declared));
                }
                source.before_path_view_index(path, step_position, index)?;
                let elements_root = digest_at(&cur_body, 8);
                let (branches, leaf) = descend_seq(source, &elements_root, declared, index)?;
                ancestors.push(Ancestor::Array { declared, branches })?;
                let (t, body) = read_value_node(source, &leaf, json_inner)?;
                (cur_tag, cur_body, cur_digest) = (t, body, leaf);
            }
            PathStepView::JsonText => {
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
fn read_value_node<'capture>(
    source: &mut CodecContext<'_, 'capture>,
    digest: &NodeDigest,
    json_inner: bool,
) -> Result<(u8, VerifierBody<'capture>), CodecError> {
    let (t, body) = source.read(digest)?;
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
    source: &mut CodecContext<'_, '_>,
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
        let (t, body) = source.read(digest)?;
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
    let (t, body) = source.read(digest)?;
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
    source: &mut CodecContext<'_, '_>,
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
    source: &mut CodecContext<'_, '_>,
    root: &NodeDigest,
    count: u64,
    index: u64,
) -> Result<(WorkVec<BranchFrame>, NodeDigest), CodecError> {
    if index >= count {
        return Err(index_range_error(index, count));
    }
    let budget = source.work_budget();
    let mut branches: WorkVec<BranchFrame> = WorkVec::new_work(budget.as_ref());
    let mut digest = *root;
    let mut at = index;
    let mut n = count;
    loop {
        match read_seq_node(source, &digest, n)? {
            SeqNode::Empty => return Err(index_range_error(index, count)),
            SeqNode::Leaf(leaf) => {
                // 叶本身若是序列节点，规范分解就已被破坏（与 verify 同口径）。
                let (t, _body) = source.read(&leaf)?;
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
                    })?;
                    digest = left;
                    n = k;
                } else {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: false,
                    })?;
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
    source: &mut CodecContext<'_, '_>,
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
    source: &mut CodecContext<'_, '_>,
    digest: &NodeDigest,
) -> Result<(NodeDigest, NodeDigest), CodecError> {
    let (t, body) = source.read(digest)?;
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
    source: &mut CodecContext<'_, '_>,
    root: &NodeDigest,
    count: u64,
    name: &str,
) -> Result<(WorkVec<BranchFrame>, NodeDigest, NodeDigest), CodecError> {
    find_field_optional(source, root, count, name)?.ok_or_else(|| field_missing_error(name))
}

fn find_field_optional(
    source: &mut CodecContext<'_, '_>,
    root: &NodeDigest,
    count: u64,
    name: &str,
) -> Result<Option<(WorkVec<BranchFrame>, NodeDigest, NodeDigest)>, CodecError> {
    let wanted = name.as_bytes();
    let budget = source.work_budget();
    let mut branches: WorkVec<BranchFrame> = WorkVec::new_work(budget.as_ref());
    let mut digest = *root;
    let mut n = count;
    loop {
        if n == 0 {
            return Ok(None);
        }
        match read_seq_node(source, &digest, n)? {
            SeqNode::Empty => return Ok(None),
            SeqNode::Leaf(field) => {
                let (key, value) = read_field(source, &field)?;
                return match cmp_key_bytes(source, &key, wanted)? {
                    Ordering::Equal => Ok(Some((branches, key, value))),
                    _ => Ok(None),
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
                    })?;
                    digest = right;
                    n -= k;
                } else {
                    branches.push(BranchFrame {
                        n,
                        left,
                        right,
                        went_left: true,
                    })?;
                    digest = left;
                    n = k;
                }
            }
        }
    }
}

/// 读字符串节点 header，返回 (字节序列根, 声明字节数)。
fn read_string_header(
    source: &mut CodecContext<'_, '_>,
    digest: &NodeDigest,
) -> Result<(NodeDigest, u64), CodecError> {
    let (t, body) = source.read(digest)?;
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
    source: &mut CodecContext<'_, '_>,
    key: &NodeDigest,
    wanted: &[u8],
) -> Result<Ordering, CodecError> {
    let (bytes_root, declared) = read_string_header(source, key)?;
    let mut iter = LeafIter::new(bytes_root, source.work_budget().as_ref())?;
    let mut pos = 0usize;
    let mut total: u64 = 0;
    while let Some((lt, _leaf, chunk)) = iter.next_context(source)? {
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
        for &byte in chunk.iter() {
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
    source: &mut CodecContext<'_, '_>,
    a: &NodeDigest,
    b: &NodeDigest,
) -> Result<Ordering, CodecError> {
    if a == b {
        return Ok(Ordering::Equal);
    }
    let (root_a, declared_a) = read_string_header(source, a)?;
    let (root_b, declared_b) = read_string_header(source, b)?;
    let mut iter_a = LeafIter::new(root_a, source.work_budget().as_ref())?;
    let mut iter_b = LeafIter::new(root_b, source.work_budget().as_ref())?;
    let mut buf_a = VerifierBody::empty();
    let mut buf_b = VerifierBody::empty();
    let mut pos_a = 0usize;
    let mut pos_b = 0usize;
    let mut total_a: u64 = 0;
    let mut total_b: u64 = 0;
    loop {
        if pos_a == buf_a.len() {
            match iter_a.next_context(source)? {
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
            match iter_b.next_context(source)? {
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
    store: &mut CodecContext<'_, '_>,
    text: &str,
) -> Result<(NodeDigest, Kind, Option<u64>), CodecError> {
    let n = u64::try_from(text.len())
        .map_err(|_| length_overflow_error("字符串字节数超过 u64 表达".to_string()))?;
    let digest = write_string_node(&mut *store, text)?;
    Ok((digest, Kind::String, Some(n)))
}

/// 自内向外重建分支链：每层写入同一 n 的新 0x13 节点，另一侧子树原样引用。
fn rebuild_sequence(
    store: &mut CodecContext<'_, '_>,
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
        let digest = store.emit(tag::SEQ_BRANCH, &body)?;
        child = digest;
    }
    Ok(child)
}

pub fn lookup_path(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    p: &[PathStep],
) -> Result<ContentRef, CodecError> {
    lookup_path_ctx(&mut CodecContext::Source(s), r, p)
}
pub fn lookup_array_items(
    s: &mut dyn NodeSource,
    r: &ContentRef,
) -> Result<Vec<ContentRef>, CodecError> {
    lookup_array_items_ctx(&mut CodecContext::Source(s), r)?.into_plain()
}
pub fn lookup_object_fields(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    n: &[&str],
) -> Result<std::collections::BTreeMap<String, ContentRef>, CodecError> {
    lookup_object_fields_ctx(&mut CodecContext::Source(s), r, n)?.into_plain()
}
pub fn lookup_object_fields_subset(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    n: &[&str],
) -> Result<std::collections::BTreeMap<String, ContentRef>, CodecError> {
    lookup_object_fields_subset_ctx(&mut CodecContext::Source(s), r, n)?.into_plain()
}
pub fn replace_batch_placeholder(
    s: &mut dyn NodeStore,
    r: &ContentRef,
    p: &[PathStep],
    b: &str,
) -> Result<ContentRef, CodecError> {
    replace_batch_placeholder_ctx(&mut CodecContext::Store(s), r, p, b)
}
pub(super) fn check_range_root(s: &mut dyn NodeSource, r: &ContentRef) -> Result<(), CodecError> {
    check_range_root_ctx(&mut CodecContext::Source(s), r)
}
pub(super) fn object_key_at(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    i: u64,
) -> Result<ContentRef, CodecError> {
    object_key_at_ctx(&mut CodecContext::Source(s), r, i)
}
pub(super) fn bytes_between(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    o: u64,
    e: u64,
) -> Result<Vec<u8>, CodecError> {
    bytes_between_ctx(&mut CodecContext::Source(s), r, o, e)?.into_plain()
}

/// Root node role/header kernel, also used by content_ref_ctx through the worker's adapter.
pub(super) fn reference_header(
    source: &mut CodecContext<'_, '_>,
    digest: &NodeDigest,
) -> Result<(Kind, Option<u64>), CodecError> {
    let (t, body) = source.read(digest)?;
    let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
    let extent = header_extent(t, &body)?;
    Ok((kind, extent))
}

#[cfg(test)]
mod owned_error_order_tests {
    use super::super::{write_value, MemoryNodes, NodeRead, Value};
    use super::*;
    #[test]
    fn common_owned_result_kernel_keeps_sourceerr_and_index_gate_precedence() {
        struct Failure {
            nodes: MemoryNodes,
            reads: usize,
            gates: usize,
            stop: usize,
            gate_error: bool,
        }
        impl NodeSource for Failure {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.reads += 1;
                if self.reads == self.stop {
                    return Err(CodecError::static_detail(
                        code::SOURCE,
                        "exact source failure",
                    ));
                }
                self.nodes.read(d)
            }
            fn before_path_index(&mut self, _: &[PathStep], _: u64) -> Result<(), CodecError> {
                self.gates += 1;
                if self.gate_error {
                    Err(CodecError::static_detail(
                        code::SOURCE,
                        "exact deadline/index gate",
                    ))
                } else {
                    Ok(())
                }
            }
        }
        let mut nodes = MemoryNodes::new();
        let reference = write_value(&mut nodes, &Value::Array(vec![Value::Null]))
            .unwrap()
            .content_ref();
        for (stop, gate_error) in [(1, false), (2, false), (usize::MAX, true)] {
            let mut normal = Failure {
                nodes: nodes.clone(),
                reads: 0,
                gates: 0,
                stop,
                gate_error,
            };
            let mut owned = Failure {
                nodes: nodes.clone(),
                reads: 0,
                gates: 0,
                stop,
                gate_error,
            };
            let steps = [PathStep::Index(0)];
            let expected =
                lookup_path_ctx(&mut CodecContext::Source(&mut normal), &reference, &steps)
                    .unwrap_err();
            // Test-only use of the shared representation kernel, not a public owned facade.
            let actual = lookup_path_output::<RefOwner>(
                &mut CodecContext::Source(&mut owned),
                (&reference).into(),
                &steps,
            )
            .unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(owned.reads, normal.reads);
            assert_eq!(owned.gates, normal.gates);
        }
    }
}

/// normal动态来源的借用Ref适配；不选择Owned容器或另一份预算。
#[doc(hidden)]
pub fn lookup_path_refview(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    path: &[PathStep],
) -> Result<ContentRef, CodecError> {
    lookup_path_output(&mut CodecContext::Source(source), reference, path)
}
#[doc(hidden)]
pub fn lookup_path_view_refview(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    path: &dyn PathView,
) -> Result<ContentRef, CodecError> {
    lookup_path_view_output(&mut CodecContext::Source(source), reference, path, false)
}
#[doc(hidden)]
pub fn lookup_fields_refview(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    names: &[&str],
    subset: bool,
) -> Result<CodecFields, CodecError> {
    object_fields_output(&mut CodecContext::Source(source), reference, names, !subset)
}

/// normal动态存储的借用Ref写入适配；Owned写面仍要求实际Closed owner。
#[doc(hidden)]
pub fn replace_batch_placeholder_refview(
    store: &mut dyn NodeStore,
    reference: RefView<'_>,
    path: &[PathStep],
    batch: &str,
) -> Result<ContentRef, CodecError> {
    replace_output(&mut CodecContext::Store(store), reference, path, batch)
}
pub(super) fn replace_batch_placeholder_owned_ctx(
    store: &mut super::ClosedCapturedOwner<'_>,
    reference: RefView<'_>,
    path: &[PathStep],
    batch: &str,
) -> Result<RefOwner, CodecError> {
    replace_output(&mut CodecContext::Closed(store), reference, path, batch)
}

/// normal动态来源的借用数组枚举；同核返回普通ContentRef，不选择Owned或另一份预算。
#[doc(hidden)]
pub fn lookup_array_items_refview(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
) -> Result<Vec<ContentRef>, CodecError> {
    lookup_array_output(&mut CodecContext::Source(source), reference)?.into_plain()
}

#[cfg(test)]
mod fixed_fields_tests {
    use super::*;
    use crate::s_content::{write_value, MemoryNodes, Value};

    #[test]
    fn fixed_fields_exact_matches_existing_refs_and_rejects_before_read_above_bound() {
        struct Reads {
            nodes: MemoryNodes,
            reads: usize,
        }
        impl NodeSource for Reads {
            fn read(&mut self, d: &NodeDigest) -> Result<super::super::NodeRead, CodecError> {
                self.reads += 1;
                self.nodes.read(d)
            }
        }
        let mut source = Reads {
            nodes: MemoryNodes::new(),
            reads: 0,
        };
        let root = write_value(
            &mut source.nodes,
            &Value::Object(vec![
                (
                    "a".into(),
                    Value::Number(super::super::NumberToken::parse("7").unwrap()),
                ),
                ("b".into(), Value::Null),
            ]),
        )
        .unwrap()
        .content_ref();
        let old = lookup_object_fields(&mut source, &root, &["b", "a"]).unwrap();
        let new = lookup_object_fields_fixed(&mut source, (&root).into(), &["b", "a"]).unwrap();
        assert_eq!(new.len(), 2);
        for key in ["a", "b"] {
            let current = new.get(key).unwrap();
            assert_eq!(current.digest(), old[key].digest().unwrap());
            assert_eq!(current.kind(), old[key].kind);
            assert_eq!(current.extent(), old[key].extent);
            assert_eq!(current.format(), old[key].format);
            assert_eq!(current.root(), old[key].root);
        }
        let reads = source.reads;
        assert!(lookup_object_fields_fixed(&mut source, (&root).into(), &["a"; 16]).is_err());
        assert_eq!(source.reads, reads, "上界先于来源读取");
        for names in [&["a"][..], &["a", "b", "c"][..], &["a", "a"][..]] {
            let old = lookup_object_fields(&mut source, &root, names)
                .err()
                .unwrap();
            let new = lookup_object_fields_fixed(&mut source, (&root).into(), names)
                .err()
                .unwrap();
            assert_eq!(new, old, "固定输出复用相同exact判据");
        }
    }

    #[test]
    fn fixed_metadata_missing_errors_follow_consumption_order_not_schema_scan() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(
            &mut nodes,
            &Value::Object(vec![("middle".into(), Value::Null)]),
        )
        .unwrap()
        .content_ref();
        let fields = lookup_object_metadata_fixed(
            &mut nodes,
            (&root).into(),
            &["after", "middle", "before"],
        )
        .unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields.get_required("middle").unwrap().kind(), Kind::Null);
        for key in ["before", "after"] {
            assert_eq!(
                fields.get_required(key).unwrap_err(),
                field_missing_error(key)
            );
        }
        assert!(lookup_object_fields_fixed(
            &mut nodes,
            (&root).into(),
            &["after", "middle", "before"]
        )
        .is_err());
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_fields_owner_full_capacity_has_zero_global_allocations_and_zero_ledger_delta() {
        use crate::s_content::capture_allocation::{audit_json::tests::observed, OwnedBudget};
        let original = OwnedBudget::new(0);
        let foreign = OwnedBudget::new(0);
        assert!(!original.same_ledger(&foreign));
        let names = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o",
        ];
        let before = original.snapshot();
        let ((), calls) = observed(|| {
            let mut fields = FixedFields::new(Some(&original)).unwrap();
            for (i, name) in names.iter().enumerate() {
                fields
                    .append(name, RefOwner::new(Kind::Null, [i as u8; 32], None))
                    .unwrap();
            }
            for (i, name) in names.iter().enumerate() {
                assert_eq!(fields.get(name).unwrap().digest(), [i as u8; 32]);
            }
            let moved = fields;
            assert_eq!(moved.len(), 15);
            drop(moved);
        });
        assert_eq!(calls, 0, "结果owner构造/满容量/读取/移动/释放均无堆分配");
        let after = original.snapshot();
        assert_eq!(after.allocation_attempts, before.allocation_attempts);
        assert_eq!(
            after.live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        assert_eq!(
            after.peak_requested_layout_bytes,
            before.peak_requested_layout_bytes
        );
        assert_eq!(foreign.snapshot().allocation_attempts, 0);
        eprintln!("fixed_fields owner actual_global_allocations={calls} ledger_before={before:?} ledger_after={after:?}");
    }
}
