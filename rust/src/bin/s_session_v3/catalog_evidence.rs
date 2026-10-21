//! #1394 C §5.4/§5.7：S3 私有目录运行状态与逐轴证据的构造、核验与当前行投影。
//!
//! 三个接口（全部只处理声明的 OHLC 档）：
//! - [`build_template`]：把同次最终生成态的 `catalog_axes`、真实来源审计计数与真实
//!   withdrawals/replaces 绑成 Delta 的 `catalog_evidence` 模板（`batch_id` 用本批占位），
//!   并给出真实 `catalog_run_status`；
//! - [`verify_cut`]：在同一读集里逐项回核该证据（声明字段、运行状态、真实计数、
//!   batch/cut/profile/hash/rule/scope、13 轴字段集合与小状态形状、CC-056 本代真实
//!   修订/撤回/替代），供主控接线在完整历史核验里调用；
//! - [`stage_current`] / [`verify_current`]：把 13 轴当前值暂写进 S3 私有 `catalog` 四列
//!   （`evidence_json` 只存规范四字段 `ContentRef`），并按完整闭包回读核对。
//!
//! 边界如实：
//! - 轴内容由 `tb02a_facts::catalog_axes` **唯一**产出；本模块不重跑包含、分型、新笔或
//!   任何其它业务判定，不写第二套轴规则；
//! - 非 OHLC 档没有 13 轴：一律拒绝，不得以「跳过审计」放行；
//! - 本模块不核完整旧历史、不写独立出生/撤回索引、不代 Commit；三个接口都不等于完整
//!   S3 审计，也不提供 CLI；
//! - `stage_current` 只暂写调用方的事务、不 commit；`verify_current` 只读。

#![allow(dead_code)] // S3 发布/历史接线由主控在下游切片落地；本模块 API 现由本文件测试消费。

use super::audit_store::{CodecRead, RefValue};
use super::business_json::{self as bj, JsonOwner, RawEventsView, TextValue};
use crate::{canonical_json, content_manifest, content_manifest::ManifestMetadata, tb02a};
#[cfg(test)]
use newchan_rust::s_content::path::{lookup_path, PathStep};
use newchan_rust::s_content::{
    self as sc, ContentRef, Kind, NodeDigest, NodeSink, NodeSource, NodeStore, Value as V,
};
use newchan_rust::session_protocol::{parse_canonical_i64, strict_json};
use rusqlite::{params, Transaction};
use sc::capture_allocation::audit_json::JsonView;
use serde_json::Value;

fn err(e: impl std::fmt::Display) -> String {
    format!("CatalogEvidenceInvalid：{e}")
}

/// OHLC 声明域的 `tested_domain` 文本；逐字等于旧生产 `s_structure_session.rs` 的取值。
pub(super) const OHLC_TESTED_DOMAIN: &str =
    "TB-02-A：显式规范整数OHLC；已建方向与无同角色极值身份竞争域；初始包含/tie具名等待";

/// 目录轴唯一集合；顺序 = `tb02a_facts::catalog_axes` 的写入顺序。
pub(super) const AXIS_IDS: [&str; 13] = [
    "CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008", "CC-009",
    "CC-010", "CC-054", "CC-055", "CC-056",
];

const AXIS_FIELDS: [&str; 4] = ["impl_status", "proof_status", "run_status", "evidence"];
const EVIDENCE_CORE_FIELDS: [&str; 3] = ["scope", "object_ids", "waiting_reasons"];
const CC056_CHANGE_FIELDS: [&str; 3] = ["raw_revisions", "withdrawals", "replaces"];
/// 一条完整目录证据的 exact 字段集合（OHLC；非 OHLC 无 `axes`，本模块不受理）。
const EVIDENCE_FIELDS: [&str; 16] = [
    "tested_domain",
    "profile_id",
    "profile_hash",
    "rule_revision",
    "batch_id",
    "structure_cut",
    "classified_objects",
    "windows_total",
    "merged_bars",
    "effective_source_positions",
    "withdrawals",
    "replaces",
    "insufficient_knowledge",
    "domain_not_satisfied",
    "scope",
    "axes",
];

#[cfg(test)]
fn f(key: &str) -> PathStep {
    PathStep::Field(key.into())
}

fn field(
    s: &mut (impl CodecRead + ?Sized),
    root: &ContentRef,
    key: &str,
) -> Result<ContentRef, String> {
    normal_ref(field_ref(s, root.into(), key)?)
}

fn scalar(
    s: &mut (impl CodecRead + ?Sized),
    root: &ContentRef,
    key: &str,
) -> Result<Value, String> {
    match scalar_ref(s, root.into(), key)? {
        JsonOwner::Plain(value) => Ok(value),
        #[cfg(feature = "s_allocation_json_testonly")]
        JsonOwner::Owned(_) => Err(err(
            "prototype_audit_allocation_incomplete：owned标量不得经normal DTO返回",
        )),
    }
}

fn text(s: &mut (impl CodecRead + ?Sized), root: &ContentRef, key: &str) -> Result<String, String> {
    match text_ref(s, root, key)? {
        TextValue::Plain(text) => Ok(text),
        TextValue::Owned(_) => Err(err(
            "prototype_audit_allocation_incomplete：owned文本不得经normal DTO返回",
        )),
    }
}

fn array_field(
    s: &mut (impl CodecRead + ?Sized),
    root: &ContentRef,
    key: &str,
) -> Result<ContentRef, String> {
    normal_ref(array_field_ref(s, root, key)?)
}

/// 精确十进制计数（证据里按规范十进制文本声明）。
fn count(s: &mut (impl CodecRead + ?Sized), root: &ContentRef, key: &str) -> Result<i64, String> {
    parse_canonical_i64(&text(s, root, key)?, key).map_err(err)
}

fn field_ref(
    s: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    key: &str,
) -> Result<RefValue, String> {
    s.lookup_field_ref(root, key).map_err(err)
}
fn normal_ref(reference: RefValue) -> Result<ContentRef, String> {
    match reference {
        RefValue::Plain(r) => Ok(r),
        RefValue::Owned(_) => Err(err(
            "prototype_audit_allocation_incomplete：owned引用不得经normal DTO返回",
        )),
    }
}
fn scalar_ref(
    s: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    key: &str,
) -> Result<JsonOwner, String> {
    let child = field_ref(s, root, key)?;
    if !matches!(
        child.kind(),
        Kind::String | Kind::Number | Kind::Bool | Kind::Null
    ) {
        return Err(err(format!("{key} 不是标量")));
    }
    bj::render_json(s, child.view()).map_err(err)
}
fn text_ref<'r>(
    s: &mut (impl CodecRead + ?Sized),
    root: impl Into<sc::RefView<'r>>,
    key: &str,
) -> Result<TextValue, String> {
    let value = scalar_ref(s, root.into(), key)?;
    let text = value
        .view()
        .as_str()
        .ok_or_else(|| err(format!("{key} 不是文本")))?;
    s.native_text(text).map_err(err)
}
fn array_field_ref<'r>(
    s: &mut (impl CodecRead + ?Sized),
    root: impl Into<sc::RefView<'r>>,
    key: &str,
) -> Result<RefValue, String> {
    let child = field_ref(s, root.into(), key)?;
    if child.kind() != Kind::Array {
        return Err(err(format!("{key} 不是数组")));
    }
    Ok(child)
}
fn generated_axes_ref(
    s: &mut (impl CodecRead + ?Sized),
    generated_state: sc::RefView<'_>,
) -> Result<RefValue, String> {
    if generated_state.kind() != Kind::Object {
        return Err(err("生成态不是对象"));
    }
    let axes = field_ref(s, generated_state, "catalog_axes")?;
    if axes.kind() != Kind::Object || axes.extent() != Some(AXIS_IDS.len() as u64) {
        return Err(err(format!(
            "生成态 catalog_axes 不是13轴对象（extent={:?}）",
            axes.extent()
        )));
    }
    Ok(axes)
}

struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}

fn cv(value: &Value) -> Result<V, String> {
    Ok(match value {
        Value::Null => V::Null,
        Value::Bool(flag) => V::Bool(*flag),
        Value::String(text) => V::String(text.clone()),
        Value::Number(number) => {
            V::Number(sc::NumberToken::parse(&number.to_string()).map_err(err)?)
        }
        Value::Array(items) => V::Array(items.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(fields) => V::Object(
            fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), cv(value)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}

/// 只算摘要、不落节点的 JSON 值根（对拍独立依据用；不物化大对象）。
fn hash_json(value: &Value) -> Result<NodeDigest, String> {
    Ok(sc::write_value(&mut HashOnly, &cv(value)?)
        .map_err(err)?
        .digest)
}

fn require_ohlc(profile_id: &str) -> Result<(), String> {
    if profile_id != tb02a::PROFILE {
        return Err(err(format!(
            "当前目录证据只实现 OHLC 档 `{}`；非 OHLC 生成态没有 13 轴，拒绝按此口径放行",
            tb02a::PROFILE
        )));
    }
    Ok(())
}

/// 生成态最终 `catalog_axes` 值边；未 finish 的草案（无该字段）即拒绝，不假空。
fn generated_axes(
    s: &mut (impl CodecRead + ?Sized),
    generated_state: &ContentRef,
) -> Result<ContentRef, String> {
    normal_ref(generated_axes_ref(s, generated_state.into())?)
}

/// 真实目录运行状态（与旧生产同一判据：有分类对象即 run）。
fn declared_run(counts: &super::source_audit::Counts) -> &'static str {
    if counts.objects >= 1 {
        "run"
    } else {
        "not_run"
    }
}

/// `batch3-` + 64 位小写十六进制；只核类型化身份形状，manifest 配方核验由发布链负责。
fn real_batch_id(id: &str) -> bool {
    let Some(hex) = id.strip_prefix("batch3-") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// 规范四字段 `ContentRef` 文本（与同域 Delta 大证据同一引用形态）。
fn reference_text(reference: &RefValue) -> Result<String, String> {
    reference.digest().map_err(err)?;
    Ok(canonical_json(
        &serde_json::to_value(reference).map_err(err)?,
    ))
}

struct AxisState {
    id: &'static str,
    impl_status: TextValue,
    proof_status: TextValue,
    run_status: TextValue,
    evidence: RefValue,
}

/// 13 轴字段集合与小状态形状核验；返回每轴的状态文本与 evidence 值边。
fn axes_entries<'r>(
    s: &mut (impl CodecRead + ?Sized),
    axes: impl Into<sc::RefView<'r>>,
) -> Result<sc::CodecVec<AxisState>, String> {
    let axes = axes.into();
    if axes.kind() != Kind::Object || axes.extent() != Some(AXIS_IDS.len() as u64) {
        return Err(err(format!(
            "目录轴根不是13轴对象（extent={:?}）",
            axes.extent()
        )));
    }
    let mut out = bj::new_work(s.allocation_budget().as_ref());
    s.native_codec_room(&mut out, AXIS_IDS.len()).map_err(err)?;
    for id in AXIS_IDS {
        let axis = field_ref(s, axes, id)?;
        if axis.kind() != Kind::Object || axis.extent() != Some(AXIS_FIELDS.len() as u64) {
            return Err(err(format!("{id} 轴不是四字段状态对象")));
        }
        let impl_status = text_ref(s, &axis, "impl_status")?;
        let proof_status = text_ref(s, &axis, "proof_status")?;
        let run_status = text_ref(s, &axis, "run_status")?;
        if impl_status != "implemented"
            || proof_status != "not_proved"
            || !matches!(run_status.as_str(), "run" | "not_run" | "waiting")
        {
            return Err(err(format!(
                "{id} 轴状态越出声明域：{impl_status}/{proof_status}/{run_status}"
            )));
        }
        let evidence = field_ref(s, axis.view(), "evidence")?;
        if evidence.kind() != Kind::Object {
            return Err(err(format!("{id} 轴 evidence 不是对象")));
        }
        // 必填字段全在 + extent 相符 ⟹ 字段集合恰为声明集合（不缺、不多）。
        let expected = if id == "CC-056" {
            EVIDENCE_CORE_FIELDS.len() + CC056_CHANGE_FIELDS.len()
        } else {
            EVIDENCE_CORE_FIELDS.len()
        };
        if evidence.extent() != Some(expected as u64) {
            return Err(err(format!(
                "{id} 轴 evidence 字段数不符（extent={:?}，应为 {expected}）",
                evidence.extent()
            )));
        }
        if text_ref(s, &evidence, "scope")? != "TB-02-A" {
            return Err(err(format!("{id} 轴 evidence.scope 不是 TB-02-A")));
        }
        for key in EVIDENCE_CORE_FIELDS.iter().skip(1) {
            array_field_ref(s, &evidence, key)?;
        }
        if id == "CC-056" {
            for key in CC056_CHANGE_FIELDS {
                array_field_ref(s, &evidence, key)?;
            }
        }
        out.try_push(AxisState {
            id,
            impl_status,
            proof_status,
            run_status,
            evidence,
        })
        .map_err(err)?;
    }
    Ok(out)
}

fn text_field(fields: &mut Vec<(String, V)>, key: &str, value: String) {
    fields.push((key.to_string(), V::String(value)));
}

/// 构造真实 `catalog_run_status` 与 `catalog_evidence` 模板。
///
/// 字段、值类型、OHLC `tested_domain` 逐字沿旧生产；`batch_id` 用 `V::BatchSelf`；
/// `scope` 取 `metadata.scope` 值边；`axes` 直接接同次最终 `generated_state.catalog_axes`
/// 的值边（不重渲染、不重造轴）。8 个计数用规范十进制文本。
pub(super) fn build_template(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    generated_state: &ContentRef,
    counts: &super::source_audit::Counts,
    withdrawals: &[Value],
    replaces: &[Value],
) -> Result<(String, ContentRef), String> {
    require_ohlc(&metadata.profile_id)?;
    let axes = generated_axes(store, generated_state)?;
    let mut fields: Vec<(String, V)> = Vec::with_capacity(EVIDENCE_FIELDS.len());
    text_field(&mut fields, "tested_domain", OHLC_TESTED_DOMAIN.to_string());
    text_field(&mut fields, "profile_id", metadata.profile_id.clone());
    text_field(&mut fields, "profile_hash", metadata.profile_hash.clone());
    text_field(&mut fields, "rule_revision", metadata.rule_revision.clone());
    fields.push(("batch_id".into(), V::BatchSelf));
    text_field(&mut fields, "structure_cut", metadata.structure_cut.clone());
    text_field(
        &mut fields,
        "classified_objects",
        counts.objects.to_string(),
    );
    text_field(&mut fields, "windows_total", counts.windows.to_string());
    text_field(&mut fields, "merged_bars", counts.groups.to_string());
    text_field(
        &mut fields,
        "effective_source_positions",
        counts.effective.to_string(),
    );
    text_field(&mut fields, "withdrawals", withdrawals.len().to_string());
    text_field(&mut fields, "replaces", replaces.len().to_string());
    text_field(
        &mut fields,
        "insufficient_knowledge",
        counts.insufficient.to_string(),
    );
    text_field(
        &mut fields,
        "domain_not_satisfied",
        counts.outside_domain.to_string(),
    );
    fields.push((
        "scope".into(),
        V::edge_of_ref(&metadata.scope).map_err(err)?,
    ));
    fields.push(("axes".into(), V::Edge(axes.digest().map_err(err)?)));
    let built = sc::write_template(store, &V::Object(fields)).map_err(err)?;
    Ok((declared_run(counts).to_string(), built.content_ref()))
}

/// 在同一读集核验一条已解析的目录证据。
///
/// 只核声明的字段、运行状态、真实计数、batch/cut/profile/hash/rule/scope 与 13 轴形状；
/// CC-056 的 `raw_revisions`/`withdrawals`/`replaces` 必须与调用者的独立依据逐位同根。
/// 不重跑包含、分型、新笔或其它业务判定；不替代调用方的完整历史审计。
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_cut<'a>(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    generated_state: &ContentRef,
    delta: &ContentRef,
    raw_prefix: impl Into<RawEventsView<'a>>,
    previous_frontier: i64,
    counts: &super::source_audit::Counts,
    withdrawals: &[Value],
    replaces: &[Value],
) -> Result<(), String> {
    verify_cut_metered(
        store,
        metadata,
        generated_state,
        delta,
        raw_prefix.into(),
        previous_frontier,
        counts,
        withdrawals,
        replaces,
        None,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_cut_until<'a>(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    generated_state: &ContentRef,
    delta: &ContentRef,
    raw_prefix: impl Into<RawEventsView<'a>>,
    previous_frontier: i64,
    counts: &super::source_audit::Counts,
    withdrawals: &[Value],
    replaces: &[Value],
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    verify_cut_metered(
        store,
        metadata,
        generated_state,
        delta,
        raw_prefix.into(),
        previous_frontier,
        counts,
        withdrawals,
        replaces,
        deadline,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_cut_metered<'a>(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    generated_state: &ContentRef,
    delta: &ContentRef,
    raw_prefix: impl Into<RawEventsView<'a>>,
    previous_frontier: i64,
    counts: &super::source_audit::Counts,
    withdrawals: &[Value],
    replaces: &[Value],
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(), String> {
    let withdrawal_view = bj::ValuesArray(withdrawals);
    let replacement_view = bj::ValuesArray(replaces);
    verify_cut_in_store(
        &mut super::audit_store::StoreView::generic(store),
        metadata.view(),
        generated_state.into(),
        delta.into(),
        raw_prefix.into(),
        previous_frontier,
        counts,
        JsonView::BorrowedArray(&withdrawal_view),
        JsonView::BorrowedArray(&replacement_view),
        deadline,
        budget,
        0,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_cut_for_audit<'a, 'r>(
    store: &mut super::history::HashReader<'_>,
    metadata: content_manifest::MetadataView<'_>,
    generated_state: impl Into<sc::RefView<'r>>,
    delta: impl Into<sc::RefView<'r>>,
    raw_prefix: impl Into<RawEventsView<'a>>,
    previous_frontier: i64,
    counts: &super::source_audit::Counts,
    withdrawals: JsonView<'_>,
    replaces: JsonView<'_>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    indexed_start: Option<usize>,
) -> Result<(), String> {
    let raw_prefix = raw_prefix.into();
    // 唯一调用者 state 已完整读取并核过 raw 的 seq=ordinal 及所有历史前沿。
    // Native 无预算冷审计只物理访问新增区间；其他入口保留原逻辑收费/期限顺序。
    if indexed_start.is_some()
        && (budget.is_some()
            || deadline.is_some()
            || raw_prefix.budget().is_some()
            || store.allocation_budget().is_some())
    {
        return Err(err("目录区间索引不接收有预算/期限路径"));
    }
    let raw_start = indexed_start.unwrap_or(0);
    if indexed_start.is_some() && i64::try_from(raw_start).ok() != previous_frontier.checked_add(1)
    {
        return Err(err("目录区间与已核前沿不符"));
    }
    if raw_start > raw_prefix.len() {
        return Err(err("目录raw区间越界"));
    }
    verify_cut_in_store(
        &mut super::audit_store::StoreView::for_audit(store),
        metadata,
        generated_state.into(),
        delta.into(),
        raw_prefix,
        previous_frontier,
        counts,
        withdrawals,
        replaces,
        deadline,
        budget,
        raw_start,
    )
}
#[allow(clippy::too_many_arguments)]
fn verify_cut_in_store(
    store: &mut super::audit_store::StoreView<'_, '_>,
    metadata: content_manifest::MetadataView<'_>,
    generated_state: sc::RefView<'_>,
    delta: sc::RefView<'_>,
    raw_prefix: RawEventsView<'_>,
    previous_frontier: i64,
    counts: &super::source_audit::Counts,
    withdrawals: JsonView<'_>,
    replaces: JsonView<'_>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    raw_start: usize,
) -> Result<(), String> {
    super::history::check_deadline(deadline)?;
    require_ohlc(metadata.profile_id)?;
    store
        .verify_ref(generated_state, sc::Mode::Public)
        .map_err(err)?;
    store.verify_ref(delta, sc::Mode::Public).map_err(err)?;
    let frontier = text_ref(store, delta, "index_frontier")?;
    if !real_batch_id(&frontier) {
        return Err(err(format!(
            "Delta index_frontier `{frontier}` 不是 batch3 类型化身份"
        )));
    }
    if text_ref(store, delta, "next_cut")? != metadata.structure_cut {
        return Err(err("Delta next_cut 与冻结生成 cut 不同"));
    }
    let run = text_ref(store, delta, "catalog_run_status")?;
    if run != declared_run(counts) {
        return Err(err(format!(
            "Delta catalog_run_status=`{run}` 与真实分类结果不符"
        )));
    }
    let evidence = field_ref(store, delta, "catalog_evidence")?;
    if evidence.kind() != Kind::Object || evidence.extent() != Some(EVIDENCE_FIELDS.len() as u64) {
        return Err(err(format!(
            "catalog_evidence 不是16字段对象（extent={:?}）",
            evidence.extent()
        )));
    }
    if text_ref(store, &evidence, "tested_domain")? != OHLC_TESTED_DOMAIN {
        return Err(err("目录证据 tested_domain 不是声明 OHLC 域"));
    }
    if text_ref(store, &evidence, "batch_id")? != frontier {
        return Err(err("目录证据 batch_id 与 Delta index_frontier 不同"));
    }
    if text_ref(store, &evidence, "structure_cut")? != metadata.structure_cut {
        return Err(err("目录证据 structure_cut 与冻结生成 cut 不同"));
    }
    for (key, want) in [
        ("profile_id", metadata.profile_id),
        ("profile_hash", metadata.profile_hash),
        ("rule_revision", metadata.rule_revision),
    ] {
        if text_ref(store, &evidence, key)? != want {
            return Err(err(format!("目录证据 {key} 与冻结元数据不同")));
        }
    }
    let withdrawals_array = withdrawals
        .as_array()
        .ok_or_else(|| err("withdrawals不是数组"))?;
    let replaces_array = replaces.as_array().ok_or_else(|| err("replaces不是数组"))?;
    for (key, want) in [
        ("classified_objects", counts.objects),
        ("windows_total", counts.windows),
        ("merged_bars", counts.groups),
        ("effective_source_positions", counts.effective),
        ("withdrawals", withdrawals_array.len()),
        ("replaces", replaces_array.len()),
        ("insufficient_knowledge", counts.insufficient),
        ("domain_not_satisfied", counts.outside_domain),
    ] {
        let got = parse_canonical_i64(&text_ref(store, &evidence, key)?, key).map_err(err)?;
        let want = i64::try_from(want).map_err(|_| err(format!("{key} 实际计数超出 i64")))?;
        if got != want {
            return Err(err(format!(
                "目录证据 {key}={got} 与真实来源审计计数 {want} 不同"
            )));
        }
    }
    let want_scope = metadata.scope.digest().map_err(err)?;
    if field_ref(store, evidence.view(), "scope")?
        .digest()
        .map_err(err)?
        != want_scope
        || field_ref(store, generated_state, "scope")?
            .digest()
            .map_err(err)?
            != want_scope
    {
        return Err(err("目录证据/生成态 scope 不是同一冻结根"));
    }
    if text_ref(store, generated_state, "structure_cut")? != metadata.structure_cut {
        return Err(err("生成态 structure_cut 与冻结生成 cut 不同"));
    }
    let axes = field_ref(store, evidence.view(), "axes")?;
    let state_axes = generated_axes_ref(store, generated_state)?;
    if axes.digest().map_err(err)? != state_axes.digest().map_err(err)? {
        return Err(err(
            "catalog_evidence.axes 不是同次最终生成态的 catalog_axes 值边",
        ));
    }
    let entries = axes_entries(store, &axes)?;
    let cc056 = entries
        .iter()
        .find(|entry| entry.id == "CC-056")
        .ok_or_else(|| err("目录轴缺 CC-056"))?;
    let allocation = store.allocation_budget();
    let mut revisions = bj::ReferenceArray(bj::new_work(allocation.as_ref()));
    for ordinal in raw_start..raw_prefix.len() {
        #[cfg(test)]
        RAW_VISITS.with(|n| n.set(n.get() + 1));
        let event = raw_prefix
            .get(ordinal)
            .ok_or_else(|| err("目录raw序号越界"))?;
        super::history::check_deadline(deadline)?;
        let seq = bj::field(event, "seq")
            .as_i64()
            .ok_or_else(|| err("原始账事件缺整数 seq"))?;
        let revision = bj::field(event, "revision")
            .as_i64()
            .ok_or_else(|| err("原始账事件缺整数 revision"))?;
        if seq > previous_frontier && revision > 1 {
            store.native_codec_room(&mut revisions.0, 1).map_err(err)?;
            revisions
                .0
                .try_push(bj::RawReference::new(event, false)?)
                .map_err(err)?;
        }
    }
    let mut withdrawals_view = bj::WithdrawalArray(bj::new_work(allocation.as_ref()));
    store
        .native_codec_room(&mut withdrawals_view.0, withdrawals_array.len())
        .map_err(err)?;
    for v in withdrawals_array.iter() {
        super::history::check_deadline(deadline)?;
        withdrawals_view
            .0
            .try_push(bj::WithdrawalWire::new(v).map_err(err)?)
            .map_err(err)?;
    }
    for (key, want) in [
        (
            "raw_revisions",
            sc::capture_allocation::audit_json::JsonView::BorrowedArray(&revisions),
        ),
        ("withdrawals", withdrawals_view.view()),
        ("replaces", replaces),
    ] {
        let got = array_field_ref(store, &cc056.evidence, key)?;
        if got.digest().map_err(err)?
            != super::history::hash_json_in_store(store, want, deadline, budget)?
        {
            return Err(err(format!("CC-056 {key} 与本代独立依据不同")));
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! { static RAW_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(super) fn take_raw_visits() -> usize {
    RAW_VISITS.with(|n| n.replace(0))
}

/// S3 私有 `catalog` 当前行的本代暂写：四列按 13 轴真实值对应，其余静态列不动。
///
/// `evidence_json` 只存规范四字段 `ContentRef`（与同域 Delta 大证据同一引用形态），
/// 不重存整段大 JSON；13 行必须都存在且每次更新恰一行。只暂写调用方事务，不 commit。
pub(super) fn stage_current(
    tx: &Transaction<'_>,
    store: &mut dyn NodeStore,
    axes: &ContentRef,
) -> Result<(), String> {
    let entries = axes_entries(store, axes)?;
    for entry in &entries {
        let reference = reference_text(&entry.evidence)?;
        let changed = tx
            .execute(
                "UPDATE catalog SET impl_status=?1,proof_status=?2,run_status=?3,evidence_json=?4 WHERE catalog_id=?5",
                params![
                    entry.impl_status.as_str(),
                    entry.proof_status.as_str(),
                    entry.run_status.as_str(),
                    reference,
                    entry.id
                ],
            )
            .map_err(|e| err(format!("暂写 catalog[{}] 失败：{e}", entry.id)))?;
        if changed != 1 {
            return Err(err(format!(
                "catalog[{}] 更新行数 {changed}≠1（13 行必须都存在且每轴恰一行）",
                entry.id
            )));
        }
    }
    Ok(())
}

/// 回读核验 13 行当前目录：状态列与轴值相同，`evidence_json` 是规范四字段 `ContentRef`
/// 且与该轴 evidence 同根，并核该引用的完整闭包。
pub(super) fn verify_current(
    tx: &Transaction<'_>,
    store: &mut dyn NodeStore,
    axes: &ContentRef,
) -> Result<(), String> {
    verify_current_in_store(tx, &mut super::audit_store::StoreView::generic(store), axes)
}
pub(super) fn verify_current_for_audit<'r>(
    tx: &Transaction<'_>,
    store: &mut super::history::HashReader<'_>,
    axes: impl Into<sc::RefView<'r>>,
) -> Result<(), String> {
    verify_current_in_store(
        tx,
        &mut super::audit_store::StoreView::for_audit(store),
        axes.into(),
    )
}
fn verify_current_in_store<'r>(
    tx: &Transaction<'_>,
    store: &mut super::audit_store::StoreView<'_, '_>,
    axes: impl Into<sc::RefView<'r>>,
) -> Result<(), String> {
    let entries = axes_entries(store, axes)?;
    let budget = store.allocation_budget();
    for entry in &entries {
        let read_error = |e| err(format!("读 catalog[{}] 失败：{e}", entry.id));
        let mut query = newchan_rust::sqlite_checked::CheckedStatement::prepare(
            tx, c"SELECT impl_status,proof_status,run_status,evidence_json FROM catalog WHERE catalog_id=?1",
        ).map_err(|e| read_error(e.to_string()))?;
        query
            .bind_text(1, entry.id)
            .map_err(|e| read_error(e.to_string()))?;
        let row = match query.next().map_err(|e| read_error(e.to_string()))? {
            Some(mut row) => Some((
                store
                    .native_text(row.text(0).map_err(|e| read_error(e.to_string()))?)
                    .map_err(read_error)?,
                store
                    .native_text(row.text(1).map_err(|e| read_error(e.to_string()))?)
                    .map_err(read_error)?,
                store
                    .native_text(row.text(2).map_err(|e| read_error(e.to_string()))?)
                    .map_err(read_error)?,
                store
                    .native_text(row.text(3).map_err(|e| read_error(e.to_string()))?)
                    .map_err(read_error)?,
            )),
            None => None,
        };
        query.finalize().map_err(|e| read_error(e.to_string()))?;
        let Some((impl_status, proof_status, run_status, evidence_json)) = row else {
            return Err(err(format!("catalog[{}] 缺行", entry.id)));
        };
        if impl_status != entry.impl_status
            || proof_status != entry.proof_status
            || run_status != entry.run_status
        {
            return Err(err(format!(
                "catalog[{}] 状态列 {impl_status}/{proof_status}/{run_status} 与轴值不同",
                entry.id
            )));
        }
        let parsed = if budget.is_none() && store.codec_work().is_some() {
            store
                .parse_rendered_plain(evidence_json.as_bytes())
                .map(JsonOwner::Plain)
        } else {
            parse_json_owned(evidence_json.as_bytes(), budget.as_ref())
        }
        .map_err(|e| err(format!("catalog[{}].evidence_json 非法：{e}", entry.id)))?;
        if !canonical_matches(parsed.view(), evidence_json.as_bytes(), budget.as_ref())? {
            return Err(err(format!(
                "catalog[{}].evidence_json 不是规范 JSON 文本",
                entry.id
            )));
        }
        let claim = sc::TransportRef::from_json_view(parsed.view()).map_err(|e| {
            err(format!(
                "catalog[{}].evidence_json 不是四字段 ContentRef：{e}",
                entry.id
            ))
        })?;
        if claim.digest().map_err(err)? != entry.evidence.digest().map_err(err)? {
            return Err(err(format!(
                "catalog[{}].evidence_json 与本轴 evidence 不同根",
                entry.id
            )));
        }
        let stored = claim.try_owned_ref().map_err(err)?;
        store
            .verify_ref(stored.view(), sc::Mode::Public)
            .map_err(err)?;
    }
    Ok(())
}

fn parse_json_owned(
    bytes: &[u8],
    budget: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return sc::capture_allocation::audit_json::OwnedJson::parse(bytes, b)
            .map(JsonOwner::Owned)
            .map_err(err);
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned目录解码入口未编译",
        ));
    }
    strict_json(bytes).map(JsonOwner::Plain)
}

fn canonical_matches(
    value: sc::capture_allocation::audit_json::JsonView<'_>,
    expected: &[u8],
    budget: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<bool, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        let bytes = sc::capture_allocation::audit_json::canonical_bytes(value, b).map_err(err)?;
        return Ok(&bytes[..] == expected);
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned目录编码入口未编译",
        ));
    }
    // #1394：直接核对完整规范字节流，避免为比较另分配输出 Vec。
    struct Matches<'a> {
        remaining: &'a [u8],
        equal: bool,
    }
    impl std::io::Write for Matches<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.equal {
                match self.remaining.strip_prefix(bytes) {
                    Some(rest) => self.remaining = rest,
                    None => self.equal = false,
                }
            }
            // 不提前停止 serializer，保留它的完整错误传播。
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Matches {
        remaining: expected,
        equal: true,
    };
    serde_json::to_writer(&mut out, &value).map_err(err)?;
    Ok(out.equal && out.remaining.is_empty())
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod owned_catalog_temp_tests {
    use super::*;
    use sc::capture_allocation::{CaptureSlab, OwnedBudget};

    #[test]
    fn catalog_field_uses_borrowed_entry_without_generic_retry() {
        let mut probe = super::super::state::borrowed_field_probe::Probe::new();
        let root = probe.reference.clone();
        assert_eq!(
            field_ref(&mut probe, (&root).into(), "borrowed😀")
                .unwrap()
                .digest()
                .unwrap(),
            root.digest().unwrap()
        );
        assert_eq!(probe.fields, 1);
    }

    #[test]
    fn actual_catalog_sql_and_scalar_owners_share_live_capture_and_refuse_before_mutation() {
        let mut fixture = super::super::prepared::tests::Fixture::new();
        for n in 1..=2 {
            let id = format!("raw-{n}");
            let (event, token) =
                fixture.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
            fixture.publish(&event, &token).unwrap();
        }
        let before_changes = fixture.conn.total_changes();
        let tx = fixture.conn.transaction().unwrap();
        let audited = super::super::state::audit(&tx).unwrap();
        let generated = &audited.cuts.last().unwrap().roots.generated_state;
        let mut normal_source = crate::content_store::ReadStore::new(&tx).unwrap();
        let normal_axes = normal_source
            .lookup_field_ref(generated.view(), "catalog_axes")
            .unwrap();
        let normal = axes_entries(&mut normal_source, normal_axes.view()).unwrap();
        let expected: Vec<_> = normal
            .iter()
            .map(|e| {
                (
                    e.id,
                    e.impl_status.to_string(),
                    e.proof_status.to_string(),
                    e.run_status.to_string(),
                    e.evidence.digest().unwrap(),
                )
            })
            .collect();
        drop(normal);
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(
            crate::content_store::capture_slab_for_prototype(
                &tx,
                &budget,
                &crate::content_store::CaptureCancellation::never(),
            )
            .unwrap(),
        );
        let capture_live = budget.snapshot().live_requested_layout_bytes;
        assert!(capture_live > 0);
        super::super::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            32768,
            1_000_000,
            |store| {
                let axes = store
                    .lookup_field_ref(generated.view(), "catalog_axes")
                    .unwrap();
                let entries = axes_entries(store, axes.view()).unwrap();
                let actual: Vec<_> = entries
                    .iter()
                    .map(|e| {
                        (
                            e.id,
                            e.impl_status.to_string(),
                            e.proof_status.to_string(),
                            e.run_status.to_string(),
                            e.evidence.digest().unwrap(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected);
                assert_eq!(entries.len(), 13);
                for e in &entries {
                    assert!(matches!(&e.impl_status, TextValue::Owned(_)));
                    assert!(matches!(&e.proof_status, TextValue::Owned(_)));
                    assert!(matches!(&e.run_status, TextValue::Owned(_)));
                    assert!(matches!(&e.evidence, RefValue::Owned(_)));
                }
                verify_current_in_store(&tx, store, axes.view()).unwrap();
                let axis = field_ref(store, axes.view(), "CC-001").unwrap();
                let retained = budget.snapshot().live_requested_layout_bytes;
                assert!(retained > capture_live);
                let ballast =
                    CaptureSlab::new(0, budget.snapshot().limit_bytes - retained - 1, &budget)
                        .unwrap();
                let before_refusal = budget.snapshot().live_requested_layout_bytes;
                let failure = text_ref(store, &axis, "impl_status").unwrap_err();
                assert!(failure.contains(sc::capture_allocation::LIMIT), "{failure}");
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    before_refusal
                );
                assert_eq!(entries[0].impl_status.as_str(), expected[0].1.as_str());
                assert_eq!(entries[0].evidence.digest().unwrap(), expected[0].4);
                drop(ballast);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, retained);
                verify_current_in_store(&tx, store, axes.view()).unwrap();
            },
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            budget.snapshot().released_layouts
        );
        assert!(!budget.ledger_health().invariant_broken);
        drop(tx);
        assert_eq!(fixture.conn.total_changes(), before_changes);
    }
}

#[cfg(test)]
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::super::{object_index, raw_input, source_audit};
    use super::*;
    use crate::content_bi::PriorState;
    use crate::tests::fixture as fixture_path;
    use crate::tests::{located_ohlc_input, tb02b_oracle_rows};
    use crate::{meta_get_opt, open_db, read_effective_events, read_raw_events, sha256_hex, v2};
    use newchan_rust::s_content::{MemoryNodes, NodeRead};
    use rusqlite::{Connection, TransactionBehavior};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    const SESSION: &str = "catalog-evidence-fixture";
    const DELTA_FIELDS: [&str; 17] = [
        "generation",
        "session_id",
        "catalog_revision",
        "base_cut",
        "next_cut",
        "seq_range",
        "index_frontier",
        "input_frontier",
        "catalog_run_status",
        "catalog_evidence",
        "upserts",
        "withdrawals",
        "replaces",
        "witnesses",
        "relations",
        "observations",
        "raw_history_added",
    ];
    const GENERATED_FIELDS: [&str; 15] = [
        "generation",
        "structure_cut",
        "input_frontier",
        "catalog_revision",
        "profile_id",
        "profile_hash",
        "rule_revision",
        "raw_events",
        "effective_events",
        "objects",
        "witnesses",
        "relations",
        "observations",
        "scope",
        "catalog_axes",
    ];
    const CC056_EVIDENCE_FIELDS: [&str; 6] = [
        "scope",
        "object_ids",
        "waiting_reasons",
        "raw_revisions",
        "withdrawals",
        "replaces",
    ];

    struct Fixture {
        dir: PathBuf,
        conn: Connection,
        store: MemoryNodes,
        metadata: ManifestMetadata,
        generated_state: ContentRef,
        axes: ContentRef,
        raw: Vec<Value>,
        counts: source_audit::Counts,
        withdrawals: Vec<Value>,
        replaces: Vec<Value>,
        run: String,
        evidence_template: ContentRef,
        batch_id: String,
        delta: ContentRef,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn rebuild(
        store: &mut MemoryNodes,
        root: &ContentRef,
        fields: &[&str],
        key: &str,
        replacement: V,
    ) -> ContentRef {
        let mut out = Vec::with_capacity(fields.len());
        for name in fields {
            let value = if *name == key {
                replacement.clone()
            } else {
                V::Edge(field(store, root, name).unwrap().digest().unwrap())
            };
            out.push(((*name).to_string(), value));
        }
        sc::write_value(store, &V::Object(out))
            .unwrap()
            .content_ref()
    }

    /// 只保留给定字段名重建对象（全部取既有边）；用于构造缺字段的草案形状。
    fn rebuild_without(store: &mut MemoryNodes, root: &ContentRef, fields: &[&str]) -> ContentRef {
        let out = fields
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    V::Edge(field(store, root, name).unwrap().digest().unwrap()),
                )
            })
            .collect();
        sc::write_value(store, &V::Object(out))
            .unwrap()
            .content_ref()
    }

    fn axis_evidence(store: &mut MemoryNodes, axes: &ContentRef, id: &str) -> ContentRef {
        let axis = field(store, axes, id).unwrap();
        field(store, &axis, "evidence").unwrap()
    }

    fn read_row(conn: &Connection, id: &str) -> (String, String, String, String) {
        conn.query_row(
            "SELECT impl_status,proof_status,run_status,evidence_json FROM catalog WHERE catalog_id=?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
    }

    /// 只读节点源包装：把指定摘要藏掉，验证「引用完整闭包」不是只看根。
    struct HideNode<'a> {
        inner: &'a mut MemoryNodes,
        hidden: NodeDigest,
    }

    impl NodeSource for HideNode<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
            if *digest == self.hidden {
                return Ok(NodeRead::Missing);
            }
            self.inner.read(digest)
        }
    }

    impl NodeSink for HideNode<'_> {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            self.inner.put(digest, tag, body)
        }
    }

    /// 真 S3 域（OHLC 档案 + 签名目录整表）+ 真原始账 + 真同核生成 + 真对象 diff + 真目录轴 finish。
    /// 原始账经 S2/S3 共用接纳规则落库；消息级 writer/clock 闸门由 control/commit 测试另验。
    fn fixture() -> Fixture {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "s1394-catalog-evidence-{}-{stamp}-{number}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("s.sqlite");
        let clock_path = dir.join("clock.json");
        let clock = serde_json::json!({
            "schema_revision": "s-clock-plan/1",
            "clock_plan_id": "catalog-evidence-fixture",
            "origin_utc": "2026-09-18T00:00:00Z",
            "unit": "ns",
            "events": {"input-1": {"accept_ns": "10", "attempts": [{"begin_ns": "11", "commit_ns": "12"}]}}
        });
        std::fs::write(&clock_path, canonical_json(&clock)).unwrap();
        super::super::init_v3(
            &db,
            SESSION,
            &fixture_path("catalog/signed-catalog.json"),
            &fixture_path("profiles/ohlc_integer_tb02a_v1.json"),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let conn = open_db(&db).unwrap();
        // 14 根具名 OHLC + 一条真实 revision=2（同一业务身份、同源坐标），
        // 使 CC-056 的 raw_revisions 非空（不是空对空的比较）。
        let mut input = located_ohlc_input(&tb02b_oracle_rows());
        let mut revised = input["events"][2].clone();
        revised["revision"] = serde_json::json!("2");
        input["events"].as_array_mut().unwrap().push(revised);
        {
            let tx = conn.unchecked_transaction().unwrap();
            let parsed = tb02a::parse_input(&input).unwrap();
            let accepted = raw_input::accept_events(&tx, &parsed, Some("1")).unwrap();
            assert_eq!(accepted.len(), 15);
            assert!(accepted.iter().all(|row| row["status"] == "accepted"));
            tx.commit().unwrap();
        }
        let raw = read_raw_events(&conn).unwrap();
        assert_eq!(raw.len(), 15);
        let effective = read_effective_events(&raw).unwrap();
        assert_eq!(effective.len(), 14);
        let text = |key: &str| meta_get_opt(&conn, key).unwrap().unwrap();
        let profile_id = text("profile_id");
        assert_eq!(profile_id, tb02a::PROFILE);
        let profile_hash = text("profile_hash");
        let rule = text("rule_revision");
        let catalog = text("catalog_revision");
        let scope: Value = strict_json(text("scope").as_bytes()).unwrap();
        let frontier = raw.len() as i64 - 1;
        let mut store = MemoryNodes::default();
        let mut runtime = GenerationRuntime::new(
            format!("catalog-evidence/{}", db.display()),
            tb02a::PROFILE,
            &rule,
        );
        let generation_input = GenerationInput {
            all_events: (&raw).into(),
            effective: (&effective).into(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: frontier,
            profile_id: tb02a::PROFILE,
            profile_hash: &profile_hash,
            rule_revision: &rule,
            catalog_revision: &catalog,
            scope: &scope,
            semantic_commit_ns: None,
            prior: PriorState {
                bi_records: vec![],
                version_record: None,
            },
        };
        let mut generated = runtime.generate(&mut store, &generation_input).unwrap();
        let plan = object_index::plan(&mut store, &generated.objects, &[], 1).unwrap();
        assert!(plan.withdrawals.is_empty() && plan.replaces.is_empty());
        generated
            .finish_axes(&mut store, -1, &plan.withdrawals, &plan.replaces)
            .unwrap();
        let counts = source_audit::audit_generated(
            &mut store,
            &generated.generated_state,
            &raw,
            tb02a::PROFILE,
            &rule,
        )
        .unwrap();
        let axes = generated_axes(&mut store, &generated.generated_state).unwrap();
        let scope_ref = sc::write_value(&mut store, &cv(&scope).unwrap())
            .unwrap()
            .content_ref();
        let metadata = ManifestMetadata {
            session_id: SESSION.to_string(),
            session_generation: "1".to_string(),
            writer_epoch: "1".to_string(),
            generation: 1,
            base_cut: "cut-0".to_string(),
            structure_cut: "cut-1".to_string(),
            input_frontier: frontier,
            source_commit: content_manifest::SourceCommit {
                source_namespace: "testonly.ohlc".to_string(),
                source_epoch: "1".to_string(),
                message_id: "fixture-message".to_string(),
                payload_hash: sha256_hex(canonical_json(&input).as_bytes()),
                producer_id: "catalog-evidence-fixture".to_string(),
                producer_epoch: "1".to_string(),
                clock_event_id: "input-1".to_string(),
                attempt_index: 0,
            },
            clock_plan_hash: v2::stored_clock(&conn).unwrap().hash,
            semantic_commit_ns: "12".to_string(),
            profile_id: profile_id.clone(),
            profile_hash: profile_hash.clone(),
            rule_revision: rule.clone(),
            catalog_revision: catalog.clone(),
            scope: scope_ref,
        };
        assert_eq!(
            metadata.scope.digest().unwrap(),
            field(&mut store, &generated.generated_state, "scope")
                .unwrap()
                .digest()
                .unwrap()
        );
        let (run, evidence_template) = build_template(
            &mut store,
            &metadata,
            &generated.generated_state,
            &counts,
            &plan.withdrawals,
            &plan.replaces,
        )
        .unwrap();
        let (manifest, delta) = manifest_and_delta(
            &mut store,
            &metadata,
            &generated.generated_state,
            &evidence_template,
            &run,
        );
        let generated_state = generated.generated_state.clone();
        drop(generated);
        Fixture {
            dir,
            conn,
            store,
            metadata,
            generated_state,
            axes,
            raw,
            counts,
            withdrawals: plan.withdrawals,
            replaces: plan.replaces,
            run,
            evidence_template,
            batch_id: manifest.batch_id,
            delta,
        }
    }

    /// 只求真实 `batch3-` 清单身份并解析 Delta 的两个本批占位；七集合用空数组骨架，
    /// 发布形状的完整构造与核对由 root 接线另验（本夹具只服务目录证据路径）。
    fn manifest_and_delta(
        store: &mut MemoryNodes,
        metadata: &ManifestMetadata,
        generated_state: &ContentRef,
        evidence: &ContentRef,
        run: &str,
    ) -> (content_manifest::UnpublishedManifest, ContentRef) {
        let empty = sc::write_value(store, &V::Array(vec![]))
            .unwrap()
            .content_ref();
        let seq_range = sc::write_value(
            store,
            &V::Object(vec![
                ("from".into(), V::String("0".into())),
                ("to".into(), V::String(metadata.input_frontier.to_string())),
            ]),
        )
        .unwrap()
        .content_ref();
        let status = sc::write_value(store, &V::String(run.into()))
            .unwrap()
            .content_ref();
        let edge = |r: &ContentRef| V::Edge(r.digest().unwrap());
        let publication = sc::write_template(
            store,
            &V::Object(vec![
                ("session_id".into(), V::String(metadata.session_id.clone())),
                (
                    "generation".into(),
                    V::String(metadata.generation.to_string()),
                ),
                (
                    "structure_cut".into(),
                    V::String(metadata.structure_cut.clone()),
                ),
                (
                    "catalog_revision".into(),
                    V::String(metadata.catalog_revision.clone()),
                ),
                ("scope".into(), V::edge_of_ref(&metadata.scope).unwrap()),
                ("index_frontier".into(), V::BatchSelf),
                ("profile_id".into(), V::String(metadata.profile_id.clone())),
                (
                    "profile_hash".into(),
                    V::String(metadata.profile_hash.clone()),
                ),
                (
                    "history_mode".into(),
                    V::String("RecomputedWithRevision".into()),
                ),
                ("as_of_generation".into(), V::Null),
                ("objects".into(), edge(&empty)),
                ("withdrawn_objects".into(), edge(&empty)),
                ("witnesses".into(), edge(&empty)),
                ("relations".into(), edge(&empty)),
                ("observations".into(), edge(&empty)),
                ("raw_history".into(), edge(&empty)),
                (
                    "input_frontier".into(),
                    V::String(metadata.input_frontier.to_string()),
                ),
                ("seq_range".into(), edge(&seq_range)),
                ("catalog_run_status".into(), edge(&status)),
                ("catalog_evidence".into(), edge(evidence)),
            ]),
        )
        .unwrap()
        .content_ref();
        let delta_template = sc::write_template(
            store,
            &V::Object(vec![
                (
                    "generation".into(),
                    V::String(metadata.generation.to_string()),
                ),
                ("session_id".into(), V::String(metadata.session_id.clone())),
                (
                    "catalog_revision".into(),
                    V::String(metadata.catalog_revision.clone()),
                ),
                ("base_cut".into(), V::String(metadata.base_cut.clone())),
                ("next_cut".into(), V::String(metadata.structure_cut.clone())),
                ("seq_range".into(), edge(&seq_range)),
                ("index_frontier".into(), V::BatchSelf),
                (
                    "input_frontier".into(),
                    V::String(metadata.input_frontier.to_string()),
                ),
                ("catalog_run_status".into(), edge(&status)),
                ("catalog_evidence".into(), edge(evidence)),
                ("upserts".into(), edge(&empty)),
                ("withdrawals".into(), edge(&empty)),
                ("replaces".into(), edge(&empty)),
                ("witnesses".into(), edge(&empty)),
                ("relations".into(), edge(&empty)),
                ("observations".into(), edge(&empty)),
                ("raw_history_added".into(), edge(&empty)),
            ]),
        )
        .unwrap()
        .content_ref();
        let roots = content_manifest::ManifestRoots {
            generated_state: generated_state.clone(),
            publication_template: publication,
            delta_template,
        };
        let manifest = content_manifest::assemble_unpublished(store, metadata, &roots).unwrap();
        let mut delta = sc::path::replace_batch_placeholder(
            store,
            &roots.delta_template,
            &[f("index_frontier")],
            &manifest.batch_id,
        )
        .unwrap();
        delta = sc::path::replace_batch_placeholder(
            store,
            &delta,
            &[f("catalog_evidence"), f("batch_id")],
            &manifest.batch_id,
        )
        .unwrap();
        sc::verify(store, &delta).unwrap();
        (manifest, delta)
    }

    #[test]
    fn real_ohlc_evidence_round_trips_through_resolved_batch() {
        let mut f = fixture();
        assert_eq!(f.run, "run");
        // 模板：batch_id 是本批占位；axes 与生成态同根；8 个计数是规范十进制文本。
        sc::verify_template(&mut f.store, &f.evidence_template).unwrap();
        let template = f.evidence_template.clone();
        assert_eq!(
            field(&mut f.store, &template, "batch_id").unwrap().kind,
            Kind::TemplateBatch
        );
        assert_eq!(
            field(&mut f.store, &template, "axes")
                .unwrap()
                .digest()
                .unwrap(),
            f.axes.digest().unwrap()
        );
        for (key, want) in [
            ("classified_objects", f.counts.objects),
            ("windows_total", f.counts.windows),
            ("merged_bars", f.counts.groups),
            ("effective_source_positions", f.counts.effective),
            ("withdrawals", f.withdrawals.len()),
            ("replaces", f.replaces.len()),
            ("insufficient_knowledge", f.counts.insufficient),
            ("domain_not_satisfied", f.counts.outside_domain),
        ] {
            assert_eq!(
                text(&mut f.store, &template, key).unwrap(),
                want.to_string(),
                "{key}"
            );
        }
        assert_eq!(
            text(&mut f.store, &template, "tested_domain").unwrap(),
            OHLC_TESTED_DOMAIN
        );
        assert_eq!(
            text(&mut f.store, &template, "profile_hash").unwrap(),
            f.metadata.profile_hash
        );
        // 真实批次字符串解析后正常核验（占位必须已全部解析，公开闭包无 0x08）。
        assert!(f.batch_id.starts_with("batch3-"));
        assert_eq!(
            text(&mut f.store, &f.delta, "index_frontier").unwrap(),
            f.batch_id
        );
        verify_cut(
            &mut f.store,
            &f.metadata,
            &f.generated_state,
            &f.delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap();
        // CC-056 本代独立依据非空：真实 revision=2 行必须出现在 raw_revisions 里。
        let evidence = field(&mut f.store, &f.delta, "catalog_evidence").unwrap();
        let axes = field(&mut f.store, &evidence, "axes").unwrap();
        let cc056 = axis_evidence(&mut f.store, &axes, "CC-056");
        assert_eq!(
            array_field(&mut f.store, &cc056, "raw_revisions")
                .unwrap()
                .extent,
            Some(1)
        );
    }

    #[test]
    fn forged_counts_axes_cc056_or_metadata_are_rejected() {
        let mut f = fixture();
        verify_cut(
            &mut f.store,
            &f.metadata,
            &f.generated_state,
            &f.delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap();
        let evidence = field(&mut f.store, &f.delta, "catalog_evidence").unwrap();

        // 1) 计数篡改：闭包仍合法，但计数必须来自真实来源审计。
        let forged = rebuild(
            &mut f.store,
            &evidence,
            &EVIDENCE_FIELDS,
            "classified_objects",
            V::String("999999".into()),
        );
        let delta = rebuild(
            &mut f.store,
            &f.delta,
            &DELTA_FIELDS,
            "catalog_evidence",
            V::Edge(forged.digest().unwrap()),
        );
        sc::verify(&mut f.store, &delta).unwrap();
        let error = verify_cut(
            &mut f.store,
            &f.metadata,
            &f.generated_state,
            &delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("classified_objects"), "{error}");

        // 2) CC-056 变化：轴、证据、生成态、Delta 全链自洽（根全合法），
        //    但 raw_revisions 与调用者独立依据不同，仍必须拒绝。
        let cc056 = field(&mut f.store, &f.axes, "CC-056").unwrap();
        let cc056_evidence = field(&mut f.store, &cc056, "evidence").unwrap();
        let empty = sc::write_value(&mut f.store, &V::Array(vec![]))
            .unwrap()
            .content_ref();
        let cc056_evidence = rebuild(
            &mut f.store,
            &cc056_evidence,
            &CC056_EVIDENCE_FIELDS,
            "raw_revisions",
            V::Edge(empty.digest().unwrap()),
        );
        let cc056 = rebuild(
            &mut f.store,
            &cc056,
            &AXIS_FIELDS,
            "evidence",
            V::Edge(cc056_evidence.digest().unwrap()),
        );
        let axes = rebuild(
            &mut f.store,
            &f.axes,
            &AXIS_IDS,
            "CC-056",
            V::Edge(cc056.digest().unwrap()),
        );
        let evidence = rebuild(
            &mut f.store,
            &evidence,
            &EVIDENCE_FIELDS,
            "axes",
            V::Edge(axes.digest().unwrap()),
        );
        let state = rebuild(
            &mut f.store,
            &f.generated_state,
            &GENERATED_FIELDS,
            "catalog_axes",
            V::Edge(axes.digest().unwrap()),
        );
        let delta = rebuild(
            &mut f.store,
            &f.delta,
            &DELTA_FIELDS,
            "catalog_evidence",
            V::Edge(evidence.digest().unwrap()),
        );
        sc::verify(&mut f.store, &state).unwrap();
        sc::verify(&mut f.store, &delta).unwrap();
        let error = verify_cut(
            &mut f.store,
            &f.metadata,
            &state,
            &delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("CC-056"), "{error}");

        // 3) 调用者独立依据变化（去掉真修订行）→ 同一拒绝，不因根合法而放行。
        let without_revision: Vec<Value> = f
            .raw
            .iter()
            .filter(|event| event["revision"] == serde_json::json!(1))
            .cloned()
            .collect();
        let error = verify_cut(
            &mut f.store,
            &f.metadata,
            &f.generated_state,
            &f.delta,
            &without_revision,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("CC-056"), "{error}");

        // 4) batch_id 伪造（形状合法但与 Delta 前沿不同）→ 拒绝。
        let batch = format!("batch3-{}", "a".repeat(64));
        let current_evidence = field(&mut f.store, &f.delta, "catalog_evidence").unwrap();
        let evidence = rebuild(
            &mut f.store,
            &current_evidence,
            &EVIDENCE_FIELDS,
            "batch_id",
            V::String(batch),
        );
        let delta = rebuild(
            &mut f.store,
            &f.delta,
            &DELTA_FIELDS,
            "catalog_evidence",
            V::Edge(evidence.digest().unwrap()),
        );
        sc::verify(&mut f.store, &delta).unwrap();
        let error = verify_cut(
            &mut f.store,
            &f.metadata,
            &f.generated_state,
            &delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("batch_id"), "{error}");

        // 5) 非 OHLC 档不得以「跳过审计」放行。
        let mut metadata = f.metadata.clone();
        metadata.profile_id = "testonly_tick_1_1_ohlc".to_string();
        let error = verify_cut(
            &mut f.store,
            &metadata,
            &f.generated_state,
            &f.delta,
            &f.raw,
            -1,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("非 OHLC"), "{error}");
    }

    #[test]
    fn catalog_current_rows_stage_verify_rollback_and_reject_forged_or_missing() {
        let mut f = fixture();
        let before = read_row(&f.conn, "CC-001");
        assert_eq!(before.0, "not_implemented");
        assert_eq!(before.3, "{}");

        // 事务内暂写 → 同事务回读即新值（四列 + 规范四字段引用）→ 回滚保留旧值。
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        stage_current(&tx, &mut f.store, &f.axes).unwrap();
        verify_current(&tx, &mut f.store, &f.axes).unwrap();
        let staged = read_row(&tx, "CC-001");
        assert_eq!(staged.0, "implemented");
        assert_eq!(staged.1, "not_proved");
        assert_eq!(
            staged.3,
            reference_text(&RefValue::Plain(axis_evidence(
                &mut f.store,
                &f.axes,
                "CC-001"
            )))
            .unwrap()
        );
        tx.rollback().unwrap();
        assert_eq!(read_row(&f.conn, "CC-001"), before);

        // 真提交一次暂写，13 行逐轴可核。
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        stage_current(&tx, &mut f.store, &f.axes).unwrap();
        tx.commit().unwrap();
        for id in AXIS_IDS {
            let (impl_status, proof_status, run_status, evidence_json) = read_row(&f.conn, id);
            assert_eq!(impl_status, "implemented", "{id}");
            assert_eq!(proof_status, "not_proved", "{id}");
            assert!(
                matches!(run_status.as_str(), "run" | "not_run" | "waiting"),
                "{id}"
            );
            assert!(!evidence_json.is_empty(), "{id}");
            assert_eq!(
                evidence_json,
                reference_text(&RefValue::Plain(axis_evidence(&mut f.store, &f.axes, id))).unwrap(),
                "{id}"
            );
        }
        let tx = f.conn.transaction().unwrap();
        verify_current(&tx, &mut f.store, &f.axes).unwrap();
        tx.rollback().unwrap();

        // 12 轴（缺一轴）→ 暂写拒绝且不写任何行。
        let cc001 = field(&mut f.store, &f.axes, "CC-001").unwrap();
        let twelve = rebuild(
            &mut f.store,
            &f.axes,
            &AXIS_IDS[..12],
            "CC-001",
            V::Edge(cc001.digest().unwrap()),
        );
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let error = stage_current(&tx, &mut f.store, &twelve).unwrap_err();
        assert!(error.contains("13轴"), "{error}");
        tx.rollback().unwrap();

        // 状态列篡改 → 回读拒绝。
        f.conn
            .execute(
                "UPDATE catalog SET run_status='waiting' WHERE catalog_id='CC-010'",
                [],
            )
            .unwrap();
        let tx = f.conn.transaction().unwrap();
        assert!(verify_current(&tx, &mut f.store, &f.axes).is_err());
        tx.rollback().unwrap();

        // 旧整段大 JSON（非规范四字段引用）→ 拒绝。
        f.conn
            .execute(
                "UPDATE catalog SET run_status='run',evidence_json='{\"scope\":\"TB-02-A\"}' WHERE catalog_id='CC-010'",
                [],
            )
            .unwrap();
        let tx = f.conn.transaction().unwrap();
        assert!(verify_current(&tx, &mut f.store, &f.axes).is_err());
        tx.rollback().unwrap();

        // 引用闭包：藏掉证据里一个真实存在、路径核验不会触碰的叶子 → 仍须拒绝。
        let cc006 = axis_evidence(&mut f.store, &f.axes, "CC-006");
        let ids = array_field(&mut f.store, &cc006, "object_ids").unwrap();
        assert!(ids.extent.unwrap() >= 1);
        let hidden = lookup_path(&mut f.store, &ids, &[PathStep::Index(0)])
            .unwrap()
            .digest()
            .unwrap();
        let mut source = HideNode {
            inner: &mut f.store,
            hidden,
        };
        let tx = f.conn.transaction().unwrap();
        assert!(verify_current(&tx, &mut source, &f.axes).is_err());
        tx.rollback().unwrap();

        // 缺行 → stage/verify 都拒绝；失败事务回滚后已提交的新值不受影响。
        f.conn
            .execute("DELETE FROM catalog WHERE catalog_id='CC-007'", [])
            .unwrap();
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        assert!(stage_current(&tx, &mut f.store, &f.axes).is_err());
        assert!(verify_current(&tx, &mut f.store, &f.axes).is_err());
        tx.rollback().unwrap();
        assert_eq!(read_row(&f.conn, "CC-001").0, "implemented");
    }

    #[test]
    fn state_without_finished_axes_is_rejected() {
        let mut f = fixture();
        // 未 finish 的草案（14 字段，无 catalog_axes）：不假空、不降级。
        let draft = rebuild_without(&mut f.store, &f.generated_state, &GENERATED_FIELDS[..14]);
        sc::verify(&mut f.store, &draft).unwrap();
        let error = build_template(
            &mut f.store,
            &f.metadata,
            &draft,
            &f.counts,
            &f.withdrawals,
            &f.replaces,
        )
        .unwrap_err();
        assert!(error.contains("catalog_axes"), "{error}");
    }
}

#[cfg(test)]
mod r9_catalog_stream_tests {
    use super::*;

    #[test]
    fn native_catalog_canonical_stream_matches_complete_normal_values() {
        let values = [
            serde_json::json!(null),
            serde_json::json!(true),
            serde_json::json!(-9223372036854775807_i64),
            serde_json::json!([]),
            serde_json::json!({"scope": "中文😀", "quotes": "\"\\\n", "items": [0, false, null]}),
        ];
        for value in &values {
            let bytes = serde_json::to_vec(value).unwrap();
            assert!(canonical_matches(JsonView::Plain(value), &bytes, None).unwrap());
        }
    }
}
