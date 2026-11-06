//! 同一 reader 内整次 path 查找结果复用（#1875 / #1890）。
//!
//! 冷算一次请求里，同一次查找（同一入口、同一根声明、同一路径或字段名清单、同一
//! exact 标志）平均被重复执行 8.8 次，path 读里 84% 来自这种重复。本表让**同一个
//! reader** 在第二次做同一查找时直接交出第一次的结果。
//!
//! # 合同论证
//!
//! 1. **每一次实际执行的查找照旧逐节点真读、真核。** 表只挂在 `*_native` 入口外层：
//!    未命中时调用原查找核，读取次序、F 核验、键序与形状判据一处不跳；命中时根本不进入
//!    查找核，不调用 `NodeSource::read`/`checked_read`，因此不存在「读了却按摘要跳过」
//!    的读。`NodeSource::checked_read` 文档「不得按摘要跳过真实读取，不得以旧成功兜底
//!    错误、Missing 或变化的原件」约束的是**来源对一次读取请求的回答**；本表不改任何
//!    来源，也不在来源层回答读取，复用的是上一层「整次查找」的结果。
//! 2. **为什么整次查找的结果可以复用。** `NodeSource` 文档要求同一次核验内读集一致：
//!    同一摘要必须返回同一 (tag, body)。查找结果只由根声明、路径和沿途读到的
//!    (tag, body) 决定（`RefOutput::from_checked(kind, digest, extent)`），而沿途每个摘要
//!    都已按 F 核过。所以在同一一致读集内，同一查找第二次执行只可能逐字重放第一次的读，
//!    得到同一结果。本表只在来源能**证明**读集一致的期间生效：
//!    - 来源必须通过 [`NodeSource::snapshot_epoch`] 给出纪元，**每次取表都重取**。目前
//!      只有 `ReadStore`（只能由 `&Transaction` 构造）给出，且只在 main 库处于
//!      `SQLITE_TXN_READ`（显式事务已建立读快照、**本事务尚未写过**）时给出；纪元为
//!      (`SQLITE_FCNTL_DATA_VERSION`, `total_changes()`)。读状态下读快照固定，外部提交
//!      不可见，本连接也没有写：节点不会被改写、删除、回滚掉，表结构不会变。一旦本事务
//!      写过（含 SAVEPOINT 后写入再 `ROLLBACK TO`、DDL 如 `DROP TABLE`），状态变为
//!      WRITE，不给纪元，reader 整表作废、不再复用。事务经 SQL 结束再开（ABA）时，新
//!      事务读之前状态是 NONE（整表作废）；读后若期间任何连接改过库，数据版本已变
//!      （本连接提交或另一连接提交都会推进），整表作废。两个信号都走 FFI、不执行 SQL，
//!      不触发审计授权器（PRAGMA 会被 `audit_sql_allowed` 拒绝并计入 `sql_denials`）。
//!    - 不在事务里的来源（例如 content_store 私有夹具那种「每次 SELECT 是独立短读」的
//!      `ReadStore` 字面量，`same_material_external_commit_visible_between_short_reads`）
//!      `is_autocommit()` 为真，没有纪元，表不启用，外部提交在下一次读时照旧可见。
//!    - reader 自身 overlay：`clear_overlay` 清表；新放入 overlay 的节点若不满足
//!      F(tag, body) = 摘要（违反 `NodeSink` 合同），reader 清表后照原逻辑继续。
//! 3. **只缓存成功。** 错误（含字段缺失、形状错误、来源读错误、预算/期限/工作域拒绝）
//!    一律不入表，并清空整表；可选字段「不存在」（`Ok(None)`）也不入表。之后同一查找
//!    照旧真实执行，错误类型与次序不变。
//! 4. **只在没有逐次计费/观察的 reader 上启用。** 提供表的 reader 必须没有 WorkBudget、
//!    审计预算、期限、工作域、Owned/Closed 容器和计量策略；有任何一项就返回 None，
//!    所有查找逐次真实执行，计费次数自然不变。转发本表的包装器（`StoreView`、
//!    `ArrayAuditSource`）自身的读取、`before_path_index` 都是纯转发，无逐次副作用；
//!    它们改变的 `audit_full_array_scan`/`preserve_sequence_read_trace` 进入键。
//!
//! 计量字段（SQL 读次数、读取字节、耗时、测试读 trace）会变少；业务返回值、错误类型
//! 和错误次序不变。

use super::capture_allocation::native_owned::{NativeRefFields, NativeValue};
use super::path::PathStepView;
use super::ref_owner::RefOutput;
use super::{CodecError, ContentRef, Kind, NodeDigest, NodeSource, RefView};
use std::collections::HashMap;

/// 单表条目上限；满后不再收新条目，之后的查找照旧真实执行。
const MAX_ENTRIES: usize = 1 << 17;
/// 单表计入字节上限：键、值的堆字节加每条目 `(Key, Value)` 本体；不含 HashMap 桶与
/// 分配器开销。结果副本在复制**之前**按剩余额度筛选，超额的结果不复制、不入表。
const MAX_BYTES: usize = 64 << 20;

/// 来源快照纪元：(`SQLITE_FCNTL_DATA_VERSION`, `total_changes()`)。
pub type SnapshotEpoch = [u64; 2];

#[derive(PartialEq, Eq, Hash)]
pub(super) enum Step {
    Field(Box<str>),
    Index(u64),
    JsonText,
    Missing,
}

/// 查找形状：哪个入口、哪条路径或哪组字段名。
#[derive(PartialEq, Eq, Hash)]
pub(super) enum Op {
    /// `lookup_path_native` / `lookup_path_view_native`（`forbid_json_text = false`）。
    Path(Box<[Step]>),
    /// `lookup_optional_field_native`，只缓存字段存在时的结果。
    OptionalField(Box<str>),
    /// `lookup_fields_native`：字段名清单（保留原顺序与重复）与 exact 标志。
    Fields { names: Box<[Box<str>]>, exact: bool },
    /// `lookup_array_items_native`。
    ArrayItems,
}
impl Op {
    pub(super) fn path_view(path: &dyn super::path::PathView) -> Self {
        Self::Path(
            (0..path.len())
                // PathView 长度与步骤不符时原查找核自行拒绝（错误不入表）；占位不会命中成功条目。
                .map(|i| path.step(i).map_or(Step::Missing, Step::from))
                .collect(),
        )
    }
    pub(super) fn fields(names: &[&str], exact: bool) -> Self {
        Self::Fields {
            names: names.iter().map(|name| Box::<str>::from(*name)).collect(),
            exact,
        }
    }
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Path(steps) => {
                steps.len() * std::mem::size_of::<Step>()
                    + steps
                        .iter()
                        .map(|s| match s {
                            Step::Field(name) => name.len(),
                            _ => 0,
                        })
                        .sum::<usize>()
            }
            Self::OptionalField(name) => name.len(),
            Self::Fields { names, .. } => {
                names.len() * std::mem::size_of::<Box<str>>()
                    + names.iter().map(|n| n.len()).sum::<usize>()
            }
            Self::ArrayItems => 0,
        }
    }
}
impl From<PathStepView<'_>> for Step {
    fn from(step: PathStepView<'_>) -> Self {
        match step {
            PathStepView::Field(name) => Self::Field(name.into()),
            PathStepView::Index(index) => Self::Index(index),
            PathStepView::JsonText => Self::JsonText,
        }
    }
}

/// 根声明四字段全部进键：根错配本身是查找的一部分，声明不同就是另一次查找。
#[derive(PartialEq, Eq, Hash)]
struct Root {
    format: Box<str>,
    root: Box<str>,
    kind: Kind,
    extent: Option<u64>,
}

#[derive(PartialEq, Eq, Hash)]
pub(super) struct Key {
    root: Root,
    op: Op,
    full_array_scan: bool,
    preserve_sequence_read_trace: bool,
}
impl Key {
    fn heap_bytes(&self) -> usize {
        self.root.format.len() + self.root.root.len() + self.op.heap_bytes()
    }
}

/// 结果只存实际节点决定的 (kind, digest, extent)；命中时按原入口同一构造重建。
type Checked = (Kind, NodeDigest, Option<u64>);
enum Value {
    Ref(Checked),
    Fields(Box<[(Box<str>, Checked)]>),
    Items(Box<[Checked]>),
}
impl Value {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Ref(_) => 0,
            Self::Fields(fields) => {
                fields.len() * std::mem::size_of::<(Box<str>, Checked)>()
                    + fields.iter().map(|(n, _)| n.len()).sum::<usize>()
            }
            Self::Items(items) => items.len() * std::mem::size_of::<Checked>(),
        }
    }
}

fn checked(reference: &ContentRef) -> Result<Checked, CodecError> {
    Ok((reference.kind, reference.digest()?, reference.extent))
}
fn rebuild(c: &Checked) -> NativeValue<ContentRef> {
    <NativeValue<ContentRef> as RefOutput>::from_checked(c.0, c.1, c.2)
}

/// reader 级查找结果表。见模块文档的合同论证。
#[derive(Default)]
pub struct LookupMemo {
    epoch: Option<SnapshotEpoch>,
    map: HashMap<Key, Value>,
    bytes: usize,
}
impl LookupMemo {
    /// 整表作废（overlay 重置、reader 失败、纪元变化时调用）。
    pub fn clear(&mut self) {
        if !self.map.is_empty() {
            self.map = HashMap::new();
        }
        self.bytes = 0;
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    /// 按来源当前纪元取表：纪元与表内条目不同即整表作废。
    pub fn at_epoch(&mut self, epoch: SnapshotEpoch) -> &mut Self {
        if self.epoch != Some(epoch) {
            self.clear();
            self.epoch = Some(epoch);
        }
        self
    }
    /// 本纪元下还能为 `key` 再收多少字节的结果；不能再收（条目满、纪元已变、键本身超额）
    /// 时为 None，调用方不复制结果。
    fn room(&self, epoch: Option<SnapshotEpoch>, key: &Key) -> Option<usize> {
        if epoch.is_none() || self.epoch != epoch || self.map.len() >= MAX_ENTRIES {
            return None;
        }
        MAX_BYTES
            .checked_sub(self.bytes)?
            .checked_sub(key.heap_bytes() + std::mem::size_of::<(Key, Value)>())
    }
    fn insert(&mut self, epoch: Option<SnapshotEpoch>, key: Key, value: Value) {
        let Some(room) = self.room(epoch, &key) else {
            return;
        };
        let value_bytes = value.heap_bytes();
        if value_bytes > room {
            return;
        }
        let bytes = key.heap_bytes() + value_bytes + std::mem::size_of::<(Key, Value)>();
        if self.map.insert(key, value).is_none() {
            self.bytes += bytes;
        }
    }
}

/// 表命中或真实执行一次查找。`run` 是原查找核；`store` 决定成功结果是否入表。
fn memoized<T>(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    op: impl FnOnce() -> Op,
    hit: impl FnOnce(&Value) -> Option<Result<T, CodecError>>,
    store: impl FnOnce(&T, usize) -> Result<Option<Value>, CodecError>,
    run: impl FnOnce(&mut dyn NodeSource) -> Result<T, CodecError>,
) -> Result<T, CodecError> {
    if source.codec_work().is_some() {
        return run(source);
    }
    let full_array_scan = source.audit_full_array_scan();
    let preserve_sequence_read_trace = source.preserve_sequence_read_trace();
    let Some(memo) = source.path_lookup_memo() else {
        return run(source);
    };
    let epoch = memo.epoch;
    let key = Key {
        root: Root {
            format: reference.format().into(),
            root: reference.root().into(),
            kind: reference.kind(),
            extent: reference.extent(),
        },
        op: op(),
        full_array_scan,
        preserve_sequence_read_trace,
    };
    if let Some(found) = memo.map.get(&key).and_then(hit) {
        return found;
    }
    let result = run(source);
    match &result {
        Ok(value) => {
            // 先按剩余额度筛选再复制；超额或转存失败只放弃入表，不改变本次返回值。
            let room = source
                .path_lookup_memo()
                .and_then(|memo| memo.room(epoch, &key));
            if let Some(room) = room {
                if let Ok(Some(value)) = store(value, room) {
                    if let Some(memo) = source.path_lookup_memo() {
                        memo.insert(epoch, key, value);
                    }
                }
            }
        }
        Err(_) => {
            if let Some(memo) = source.path_lookup_memo() {
                memo.clear();
            }
        }
    }
    result
}

pub(super) fn lookup_ref(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    op: impl FnOnce() -> Op,
    run: impl FnOnce(&mut dyn NodeSource) -> Result<NativeValue<ContentRef>, CodecError>,
) -> Result<NativeValue<ContentRef>, CodecError> {
    memoized(
        source,
        reference,
        op,
        |value| match value {
            Value::Ref(c) => Some(Ok(rebuild(c))),
            _ => None,
        },
        |result, _| checked(result.as_ref()).map(|c| Some(Value::Ref(c))),
        run,
    )
}

pub(super) fn lookup_optional(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    name: &str,
    run: impl FnOnce(&mut dyn NodeSource) -> Result<Option<NativeValue<ContentRef>>, CodecError>,
) -> Result<Option<NativeValue<ContentRef>>, CodecError> {
    memoized(
        source,
        reference,
        || Op::OptionalField(name.into()),
        |value| match value {
            Value::Ref(c) => Some(Ok(Some(rebuild(c)))),
            _ => None,
        },
        |result, _| match result {
            Some(found) => checked(found.as_ref()).map(|c| Some(Value::Ref(c))),
            None => Ok(None),
        },
        run,
    )
}

pub(super) fn lookup_fields(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    names: &[&str],
    exact: bool,
    run: impl FnOnce(&mut dyn NodeSource) -> Result<NativeRefFields, CodecError>,
) -> Result<NativeRefFields, CodecError> {
    memoized(
        source,
        reference,
        || Op::fields(names, exact),
        |value| match value {
            Value::Fields(fields) => {
                let refs: Vec<(&str, ContentRef)> = fields
                    .iter()
                    .map(|(name, c)| (name.as_ref(), ContentRef::new(c.0, &c.1, c.2)))
                    .collect();
                // 与 ArrayAuditSource 字段 memo 同一重建口：无工作域，按键规范序输出。
                Some(NativeRefFields::try_from_views(
                    refs.iter().map(|(name, r)| (*name, RefView::from(r))),
                    None,
                ))
            }
            _ => None,
        },
        |result, room| {
            let names: usize = result.iter().map(|(name, _)| name.len()).sum();
            let len = result.iter().count();
            if len * std::mem::size_of::<(Box<str>, Checked)>() + names > room {
                return Ok(None);
            }
            result
                .iter()
                .map(|(name, r)| checked(r.as_ref()).map(|c| (Box::<str>::from(name), c)))
                .collect::<Result<Box<[_]>, _>>()
                .map(|fields| Some(Value::Fields(fields)))
        },
        run,
    )
}

pub(super) fn lookup_items(
    source: &mut dyn NodeSource,
    reference: RefView<'_>,
    run: impl FnOnce(
        &mut dyn NodeSource,
    ) -> Result<super::CodecVec<NativeValue<ContentRef>>, CodecError>,
) -> Result<super::CodecVec<NativeValue<ContentRef>>, CodecError> {
    memoized(
        source,
        reference,
        || Op::ArrayItems,
        |value| match value {
            Value::Items(items) => {
                let mut out = super::CodecVec::new(None);
                Some(
                    out.try_reserve(items.len())
                        .and_then(|()| items.iter().try_for_each(|c| out.try_push(rebuild(c))))
                        .map(|()| out),
                )
            }
            _ => None,
        },
        |result, room| {
            if result.len() * std::mem::size_of::<Checked>() > room {
                return Ok(None);
            }
            result
                .iter()
                .map(|r| checked(r.as_ref()))
                .collect::<Result<Box<[_]>, _>>()
                .map(|items| Some(Value::Items(items)))
        },
        run,
    )
}
