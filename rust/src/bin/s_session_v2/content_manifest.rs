//! #1394 C §5.4：按值边组装尚未发布的批次清单。
//!
//! 本模块只核码本闭包、三个根的顶层 schema 和跨根发布坐标；不授予出生位置许可，
//! 不写独立索引、clock 或 cut。输入标量须由调用者的同一个 Begin/Commit 读集取得。
//! 模板内 0x08 的业务许可、对象/来源/Delta 对拍仍由正式发布者核验。
#![allow(dead_code)]

use newchan_rust::s_content::{
    self,
    path::{lookup_path, PathStep},
    Built, CodecError, ContentRef, Kind, NodeDigest, NodeStore, NumberToken, Value as V,
};
use newchan_rust::session_protocol::{nonnegative, positive};
use std::collections::BTreeMap;
type Fields = BTreeMap<String, ContentRef>;

const GENERATED_FIELDS: &[&str] = &[
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
];
const SNAPSHOT_FIELDS: &[&str] = &[
    "session_id",
    "generation",
    "structure_cut",
    "catalog_revision",
    "scope",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "history_mode",
    "as_of_generation",
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "catalog_evidence",
];
const DELTA_FIELDS: &[&str] = &[
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

#[derive(Clone, Debug)]
pub(super) struct SourceCommit {
    pub source_namespace: String,
    pub source_epoch: String,
    pub message_id: String,
    pub payload_hash: String,
    pub producer_id: String,
    pub producer_epoch: String,
    pub clock_event_id: String,
    pub attempt_index: i64,
}

#[derive(Clone, Debug)]
pub(super) struct ManifestMetadata {
    pub session_id: String,
    pub session_generation: String,
    pub writer_epoch: String,
    pub generation: i64,
    pub base_cut: String,
    pub structure_cut: String,
    pub input_frontier: i64,
    pub source_commit: SourceCommit,
    pub clock_plan_hash: String,
    pub semantic_commit_ns: String,
    pub profile_id: String,
    pub profile_hash: String,
    pub rule_revision: String,
    pub catalog_revision: String,
    pub scope: ContentRef,
}

#[derive(Clone, Debug)]
pub(super) struct ManifestRoots {
    pub generated_state: ContentRef,
    pub publication_template: ContentRef,
    pub delta_template: ContentRef,
}

pub(super) struct UnpublishedManifest {
    pub value: Built,
    pub batch_id: String,
}

pub(super) fn assemble_unpublished(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &ManifestRoots,
) -> Result<UnpublishedManifest, CodecError> {
    check_metadata(metadata)?;
    // 同一次读集核完整闭包；路径算子只核触碰边界，不能替代这里的全闭包校验。
    s_content::verify(store, &metadata.scope)?;
    require_kind(&metadata.scope, Kind::Object, "scope")?;
    s_content::verify(store, &roots.generated_state)?;
    s_content::verify_template_many(
        store,
        [
            roots.publication_template.clone(),
            roots.delta_template.clone(),
        ],
    )?;
    let generated_fields = exact_object(store, &roots.generated_state, GENERATED_FIELDS, true)?;
    let publication_fields =
        exact_object(store, &roots.publication_template, SNAPSHOT_FIELDS, false)?;
    let delta_fields = exact_object(store, &roots.delta_template, DELTA_FIELDS, false)?;

    let generated = &generated_fields;
    let publication = &publication_fields;
    let delta = &delta_fields;
    // 保留旧生成态的 JSON number；公开 snapshot/Delta 控制量则继续是规范十进制字符串。
    match_value(store, generated, "generation", number(metadata.generation)?)?;
    match_value(
        store,
        generated,
        "input_frontier",
        number(metadata.input_frontier)?,
    )?;
    for root in [publication, delta] {
        match_value(
            store,
            root,
            "generation",
            text(metadata.generation.to_string()),
        )?;
        match_value(
            store,
            root,
            "input_frontier",
            text(metadata.input_frontier.to_string()),
        )?;
        match_value(store, root, "session_id", text(&metadata.session_id))?;
        require_kind(&field(store, root, "seq_range")?, Kind::Object, "seq_range")?;
        require_kind(
            &field(store, root, "catalog_run_status")?,
            Kind::String,
            "catalog_run_status",
        )?;
        require_kind(
            &field(store, root, "catalog_evidence")?,
            Kind::Object,
            "catalog_evidence",
        )?;
        // 当前发布头由本批清单派生，不能预填旧批次或形成 hash 自引用。
        require_kind(
            &field(store, root, "index_frontier")?,
            Kind::TemplateBatch,
            "index_frontier",
        )?;
        let evidence = field(store, root, "catalog_evidence")?;
        let evidence_batch = lookup_path(store, &evidence, &[PathStep::Field("batch_id".into())])?;
        require_kind(
            &evidence_batch,
            Kind::TemplateBatch,
            "catalog_evidence.batch_id",
        )?;
    }
    for root in [generated, publication, delta] {
        match_value(
            store,
            root,
            "catalog_revision",
            text(&metadata.catalog_revision),
        )?;
    }
    for root in [generated, publication] {
        match_value(store, root, "structure_cut", text(&metadata.structure_cut))?;
        match_value(store, root, "profile_id", text(&metadata.profile_id))?;
        match_value(store, root, "profile_hash", text(&metadata.profile_hash))?;
        match_digest(store, root, "scope", metadata.scope.digest()?)?;
    }
    match_value(
        store,
        generated,
        "rule_revision",
        text(&metadata.rule_revision),
    )?;
    match_value(store, delta, "base_cut", text(&metadata.base_cut))?;
    match_value(store, delta, "next_cut", text(&metadata.structure_cut))?;
    for key in ["seq_range", "catalog_run_status", "catalog_evidence"] {
        let expected = field(store, publication, key)?.digest()?;
        match_digest(store, delta, key, expected)?;
    }
    for (root, fields) in [
        (
            generated,
            &[
                "raw_events",
                "effective_events",
                "objects",
                "witnesses",
                "relations",
                "observations",
            ][..],
        ),
        (
            publication,
            &[
                "objects",
                "withdrawn_objects",
                "witnesses",
                "relations",
                "observations",
                "raw_history",
            ][..],
        ),
        (
            delta,
            &[
                "upserts",
                "withdrawals",
                "replaces",
                "witnesses",
                "relations",
                "observations",
                "raw_history_added",
            ][..],
        ),
    ] {
        for key in fields {
            require_kind(&field(store, root, key)?, Kind::Array, key)?;
        }
    }
    let source = &metadata.source_commit;
    let source_commit = V::Object(vec![
        ("source_namespace".into(), text(&source.source_namespace)),
        ("source_epoch".into(), text(&source.source_epoch)),
        ("message_id".into(), text(&source.message_id)),
        ("payload_hash".into(), text(&source.payload_hash)),
        ("producer_id".into(), text(&source.producer_id)),
        ("producer_epoch".into(), text(&source.producer_epoch)),
        ("clock_event_id".into(), text(&source.clock_event_id)),
        (
            "attempt_index".into(),
            text(source.attempt_index.to_string()),
        ),
    ]);
    let value = s_content::write_template(
        store,
        &V::Object(vec![
            ("schema_revision".into(), text("s-batch-manifest/1")),
            ("content_format".into(), text(s_content::CONTENT_FORMAT)),
            ("protocol_revision".into(), text("s-session/3")),
            ("session_id".into(), text(&metadata.session_id)),
            (
                "session_generation".into(),
                text(&metadata.session_generation),
            ),
            ("writer_epoch".into(), text(&metadata.writer_epoch)),
            ("generation".into(), text(metadata.generation.to_string())),
            ("base_cut".into(), text(&metadata.base_cut)),
            ("structure_cut".into(), text(&metadata.structure_cut)),
            (
                "input_frontier".into(),
                text(metadata.input_frontier.to_string()),
            ),
            ("source_commit".into(), source_commit),
            ("clock_plan_hash".into(), text(&metadata.clock_plan_hash)),
            (
                "semantic_commit_ns".into(),
                text(&metadata.semantic_commit_ns),
            ),
            ("profile_id".into(), text(&metadata.profile_id)),
            ("profile_hash".into(), text(&metadata.profile_hash)),
            ("rule_revision".into(), text(&metadata.rule_revision)),
            ("catalog_revision".into(), text(&metadata.catalog_revision)),
            ("scope".into(), V::Edge(metadata.scope.digest()?)),
            (
                "generated_state".into(),
                V::Edge(roots.generated_state.digest()?),
            ),
            (
                "publication_template".into(),
                V::Edge(roots.publication_template.digest()?),
            ),
            (
                "delta_template".into(),
                V::Edge(roots.delta_template.digest()?),
            ),
        ]),
    )?;
    let identity = s_content::frame(0x21, &value.digest);
    let batch_id = format!("batch3-{}", &s_content::root_string(&identity)[4..]);
    Ok(UnpublishedManifest { value, batch_id })
}

fn text(value: impl Into<String>) -> V {
    V::String(value.into())
}
fn number(value: i64) -> Result<V, CodecError> {
    Ok(V::Number(NumberToken::parse(&value.to_string())?))
}
fn context_error(detail: impl Into<String>) -> CodecError {
    CodecError::new("manifest_context", detail)
}

fn check_metadata(m: &ManifestMetadata) -> Result<(), CodecError> {
    if m.generation < 1 || m.input_frontier < -1 || m.source_commit.attempt_index < 0 {
        return Err(context_error(
            "代号须为正、输入前沿不得小于 -1、attempt_index 不得为负",
        ));
    }
    for (name, value) in [
        ("session_generation", &m.session_generation),
        ("writer_epoch", &m.writer_epoch),
        ("producer_epoch", &m.source_commit.producer_epoch),
    ] {
        positive(value, name).map_err(context_error)?;
    }
    nonnegative(&m.semantic_commit_ns, "semantic_commit_ns").map_err(context_error)?;
    for (name, value) in [
        ("session_id", &m.session_id),
        ("base_cut", &m.base_cut),
        ("structure_cut", &m.structure_cut),
        ("profile_id", &m.profile_id),
        ("rule_revision", &m.rule_revision),
        ("catalog_revision", &m.catalog_revision),
        ("source_namespace", &m.source_commit.source_namespace),
        ("source_epoch", &m.source_commit.source_epoch),
        ("message_id", &m.source_commit.message_id),
        ("producer_id", &m.source_commit.producer_id),
        ("clock_event_id", &m.source_commit.clock_event_id),
    ] {
        if value.is_empty() {
            return Err(context_error(format!("{name} 不得为空")));
        }
    }
    for (name, value) in [
        ("profile_hash", &m.profile_hash),
        ("clock_plan_hash", &m.clock_plan_hash),
        ("payload_hash", &m.source_commit.payload_hash),
    ] {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(context_error(format!(
                "{name} 必须是 SHA256 的 64 位小写十六进制"
            )));
        }
    }
    Ok(())
}

fn require_kind(value: &ContentRef, kind: Kind, name: &str) -> Result<(), CodecError> {
    if value.kind != kind {
        return Err(CodecError::new(
            "manifest_schema",
            format!("{name} 应为 {}", kind.name()),
        ));
    }
    Ok(())
}

fn field(_store: &mut dyn NodeStore, root: &Fields, name: &str) -> Result<ContentRef, CodecError> {
    root.get(name)
        .cloned()
        .ok_or_else(|| CodecError::new("manifest_schema", format!("根缺字段{name}")))
}

fn exact_object(
    store: &mut dyn NodeStore,
    root: &ContentRef,
    fields: &[&str],
    allow_axes: bool,
) -> Result<Fields, CodecError> {
    require_kind(root, Kind::Object, "根")?;
    let n = fields.len() as u64;
    let has_axes = allow_axes && root.extent == Some(n + 1);
    if root.extent != Some(n) && !has_axes {
        return Err(CodecError::new(
            "manifest_schema",
            "顶层 exact 字段数量不符",
        ));
    }
    let mut names = fields.to_vec();
    if has_axes {
        names.push("catalog_axes");
    }
    let values = s_content::path::lookup_object_fields(store, root, &names)?;
    if has_axes {
        require_kind(&values["catalog_axes"], Kind::Object, "catalog_axes")?;
    }
    Ok(values)
}

fn match_digest(
    store: &mut dyn NodeStore,
    root: &Fields,
    key: &str,
    expected: NodeDigest,
) -> Result<(), CodecError> {
    if field(store, root, key)?.digest()? != expected {
        return Err(CodecError::new(
            "manifest_binding",
            format!("{key} 与当前发布上下文或相邻根不符"),
        ));
    }
    Ok(())
}

fn match_value(
    store: &mut dyn NodeStore,
    root: &Fields,
    key: &str,
    expected: V,
) -> Result<(), CodecError> {
    let expected = s_content::write_value(store, &expected)?.digest;
    match_digest(store, root, key, expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::{MemoryNodes, NumberToken, Value as V};
    use serde_json::{json, Value as J};

    fn cv(value: &J) -> V {
        match value {
            J::Null => V::Null,
            J::Bool(v) => V::Bool(*v),
            J::Number(v) => V::Number(NumberToken::parse(&v.to_string()).unwrap()),
            J::String(v) => V::String(v.clone()),
            J::Array(v) => V::Array(v.iter().map(cv).collect()),
            J::Object(v) => V::Object(v.iter().map(|(k, v)| (k.clone(), cv(v))).collect()),
        }
    }

    fn fixture() -> (MemoryNodes, ManifestMetadata, ManifestRoots, J, J, J) {
        let mut store = MemoryNodes::new();
        let scope = json!({"structure":"CompleteCut","economic":"not_started"});
        let scope_ref = s_content::write_value(&mut store, &cv(&scope))
            .unwrap()
            .content_ref();
        let metadata = ManifestMetadata {
            session_id: "manifest-test".into(),
            session_generation: "1".into(),
            writer_epoch: "2".into(),
            generation: 2,
            base_cut: "cut-1".into(),
            structure_cut: "cut-2".into(),
            input_frontier: 1,
            source_commit: SourceCommit {
                source_namespace: "fixture.feed".into(),
                source_epoch: "fixture-epoch".into(),
                message_id: "message-2".into(),
                payload_hash: "a".repeat(64),
                producer_id: "fixture-producer".into(),
                producer_epoch: "2".into(),
                clock_event_id: "event-2".into(),
                attempt_index: 0,
            },
            clock_plan_hash: "b".repeat(64),
            semantic_commit_ns: "24".into(),
            profile_id: "fixture-profile".into(),
            profile_hash: "c".repeat(64),
            rule_revision: "fixture-rule".into(),
            catalog_revision: "fixture-catalog".into(),
            scope: scope_ref,
        };
        let generated = json!({
            "generation":2,"structure_cut":"cut-2","input_frontier":1,
            "catalog_revision":"fixture-catalog","profile_id":"fixture-profile",
            "profile_hash":"c".repeat(64),"rule_revision":"fixture-rule", "scope":scope,
            "raw_events":[],"effective_events":[],"objects":[],"witnesses":[],
            "relations":[],"observations":[]
        });
        let publication = json!({
            "session_id":"manifest-test","generation":"2","structure_cut":"cut-2",
            "catalog_revision":"fixture-catalog","scope":scope,"index_frontier":null,
            "profile_id":"fixture-profile","profile_hash":"c".repeat(64),
            "history_mode":"RecomputedWithRevision","as_of_generation":null,
            "objects":[],"withdrawn_objects":[],"witnesses":[],"relations":[],
            "observations":[],"raw_history":[],"input_frontier":"1",
            "seq_range":{"from":"1","to":"1"},"catalog_run_status":"not_run",
            "catalog_evidence":{"batch_id":null}
        });
        let delta = json!({
            "generation":"2","session_id":"manifest-test","catalog_revision":"fixture-catalog",
            "base_cut":"cut-1","next_cut":"cut-2","seq_range":{"from":"1","to":"1"},
            "index_frontier":null,"input_frontier":"1","catalog_run_status":"not_run",
            "catalog_evidence":{"batch_id":null},"upserts":[],"withdrawals":[],
            "replaces":[],"witnesses":[],"relations":[],"observations":[],"raw_history_added":[]
        });
        let roots = roots(&mut store, &generated, &publication, &delta);
        (store, metadata, roots, generated, publication, delta)
    }

    fn template(value: &J) -> V {
        let V::Object(mut fields) = cv(value) else {
            panic!("fixture object")
        };
        for (key, value) in &mut fields {
            if key == "index_frontier" {
                *value = V::BatchSelf;
            }
            if key == "catalog_evidence" {
                *value = V::Object(vec![("batch_id".into(), V::BatchSelf)]);
            }
        }
        V::Object(fields)
    }

    fn roots(store: &mut MemoryNodes, generated: &J, publication: &J, delta: &J) -> ManifestRoots {
        ManifestRoots {
            generated_state: s_content::write_value(store, &cv(generated))
                .unwrap()
                .content_ref(),
            publication_template: s_content::write_template(store, &template(publication))
                .unwrap()
                .content_ref(),
            delta_template: s_content::write_template(store, &template(delta))
                .unwrap()
                .content_ref(),
        }
    }

    #[test]
    fn content_manifest_binds_complete_template_edges_without_descriptor_hashing() {
        let (mut store, metadata, roots, _, _, _) = fixture();
        let built = assemble_unpublished(&mut store, &metadata, &roots).unwrap();
        assert_eq!(built.value.extent, Some(21));
        // 由独立 Python 按批准的 F/M/V 配方与上述显式 fixture 计算；不调用 Rust 产期望。
        assert_eq!(
            built.value.root(),
            "sc1-69b379691366b15839155bdbf66cb52e8d83db3742db97dd38a99397a9ce5fd6"
        );
        assert_eq!(
            built.batch_id,
            "batch3-cf784d122da0fe22e82c1e1162224416da8be5c676bd14d8d442e4c0ab8b259a"
        );
        s_content::verify_template(&mut store, &built.value.content_ref()).unwrap();
        // 模板尚有 0x08：组装成功不能被当作可公开的已发布切面。
        assert_eq!(
            s_content::verify(&mut store, &built.value.content_ref())
                .unwrap_err()
                .code(),
            s_content::code::TEMPLATE_PLACEHOLDER
        );
    }

    #[test]
    fn content_manifest_rejects_missing_extra_and_wrong_collection_fields() {
        let (mut store, metadata, _, generated, publication, delta) = fixture();
        let mut missing = generated.clone();
        missing.as_object_mut().unwrap().remove("observations");
        let input = roots(&mut store, &missing, &publication, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());

        let mut extra = generated.clone();
        extra["unapproved_field"] = json!([]);
        let input = roots(&mut store, &extra, &publication, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());
        let mut extra = publication.clone();
        extra["q_projection_root"] = json!("not-a-manifest-input");
        let input = roots(&mut store, &generated, &extra, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());
        let mut extra = delta.clone();
        extra["batch_id"] = json!("not-a-manifest-input");
        let input = roots(&mut store, &generated, &publication, &extra);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());

        let mut wrong = generated.clone();
        wrong["observations"] = json!({});
        let input = roots(&mut store, &wrong, &publication, &delta);
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            "manifest_schema"
        );
        let mut ohlc = generated;
        ohlc["catalog_axes"] = json!({});
        let input = roots(&mut store, &ohlc, &publication, &delta);
        assemble_unpublished(&mut store, &metadata, &input).unwrap();
    }

    #[test]
    fn content_manifest_rejects_mixed_generation_profile_scope_and_delta_header() {
        let (mut store, metadata, _, generated, publication, delta) = fixture();
        for (family, key, value) in [
            (0, "generation", json!(3)),
            (0, "generation", json!("2")),
            (0, "rule_revision", json!("different-rule")),
            (1, "profile_id", json!("different-profile")),
            (
                1,
                "scope",
                json!({"structure":"PartialCut","economic":"not_started"}),
            ),
            (2, "base_cut", json!("cut-0")),
            (2, "next_cut", json!("cut-3")),
            (2, "seq_range", json!({"from":"0","to":"1"})),
            (2, "catalog_run_status", json!("run")),
        ] {
            let (mut g, mut p, mut d) = (generated.clone(), publication.clone(), delta.clone());
            let target = match family {
                0 => &mut g,
                1 => &mut p,
                _ => &mut d,
            };
            target[key] = value;
            let input = roots(&mut store, &g, &p, &d);
            assert_eq!(
                assemble_unpublished(&mut store, &metadata, &input)
                    .err()
                    .unwrap()
                    .code(),
                "manifest_binding",
                "{family}.{key}"
            );
        }
    }

    #[test]
    fn content_manifest_binds_all_source_commit_fields_and_clock_epoch() {
        let (mut store, metadata, roots, _, _, _) = fixture();
        let original = assemble_unpublished(&mut store, &metadata, &roots)
            .unwrap()
            .batch_id;
        for field in 0..12 {
            let mut changed = metadata.clone();
            match field {
                0 => changed.source_commit.source_namespace.push_str("-changed"),
                1 => changed.source_commit.source_epoch.push_str("-changed"),
                2 => changed.source_commit.message_id.push_str("-changed"),
                3 => changed.source_commit.payload_hash = "d".repeat(64),
                4 => changed.source_commit.producer_id.push_str("-changed"),
                5 => changed.source_commit.producer_epoch = "3".into(),
                6 => changed.source_commit.clock_event_id.push_str("-changed"),
                7 => changed.source_commit.attempt_index = 1,
                8 => changed.clock_plan_hash = "e".repeat(64),
                9 => changed.semantic_commit_ns = "25".into(),
                10 => changed.writer_epoch = "3".into(),
                11 => changed.session_generation = "2".into(),
                _ => unreachable!(),
            }
            assert_ne!(
                assemble_unpublished(&mut store, &changed, &roots)
                    .unwrap()
                    .batch_id,
                original,
                "field {field}"
            );
        }
        let mut invalid = metadata.clone();
        invalid.source_commit.attempt_index = -1;
        assert_eq!(
            assemble_unpublished(&mut store, &invalid, &roots)
                .err()
                .unwrap()
                .code(),
            "manifest_context"
        );
        invalid = metadata;
        invalid.session_generation = "01".into();
        assert_eq!(
            assemble_unpublished(&mut store, &invalid, &roots)
                .err()
                .unwrap()
                .code(),
            "manifest_context"
        );
    }

    #[test]
    fn content_manifest_rejects_missing_nodes_and_generated_placeholders() {
        let (mut store, metadata, mut input, generated, _, _) = fixture();
        input.generated_state.root = format!("sc1-{}", "f".repeat(64));
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            s_content::code::NODE_MISSING
        );
        let V::Object(mut fields) = cv(&generated) else {
            unreachable!()
        };
        fields
            .iter_mut()
            .find(|(key, _)| key == "objects")
            .unwrap()
            .1 = V::Array(vec![V::BatchSelf]);
        input.generated_state = s_content::write_template(&mut store, &V::Object(fields))
            .unwrap()
            .content_ref();
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            s_content::code::TEMPLATE_PLACEHOLDER
        );
    }
}
