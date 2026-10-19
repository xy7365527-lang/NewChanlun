#![cfg(feature = "s_session")]
//! #1394 采纳 C §5.4/§5.6：内容图路径定位与本批占位（0x08）定点替换的靶向验收。
//!
//! 边界（照实）：本文件只验低层算子。出生／首次观察／catalog 位置的授权与
//! publish 前全闭包核验归调用方；这里用「独立构造 oracle + 全 verify/render」证明
//! 替换结果与直接构造逐字相等，并用读/写仪表证明大数组定点替换不展开全叶。
//! 核验阶段：模板上下文（`verify_template`，仍允许 0x08）只是中间态；一旦进入公开逻辑值
//! 就必须过 `verify`（Public）——单占位用例的替换结果正是走这条末级判据。

use newchan_rust::s_content::path::{
    lookup_object_fields, lookup_path, path_code, replace_batch_placeholder, PathStep,
};
use newchan_rust::s_content::{
    checked_frame_len, code, frame, render_canonical, sequence_concat, tag, verify,
    verify_template, write_template, write_value, CodecError, ContentRef, JsonValue, Kind,
    MemoryNodes, NodeDigest, NodeRead, NodeSink, NodeSource, NumberToken, SequenceRef, Value,
};

/// 仪表码：只用于测试资源门（不是生产码）。
const BUDGET_CODE: &str = "test_budget";

#[test]
fn exact_field_set_matches_individual_paths_with_bounded_reads() {
    let mut store = MeteredStore::new();
    let names = (0..20).map(|n| format!("field-{n:02}")).collect::<Vec<_>>();
    let root = write_value(
        &mut store,
        &Value::Object(
            names
                .iter()
                .map(|k| (k.clone(), Value::Array(vec![Value::string("shared"); 1024])))
                .collect(),
        ),
    )
    .unwrap()
    .content_ref();
    let keys = names.iter().map(String::as_str).collect::<Vec<_>>();
    store.reads = 0;
    let singles = keys
        .iter()
        .map(|k| lookup_path(&mut store, &root, &[PathStep::Field((*k).into())]).unwrap())
        .collect::<Vec<_>>();
    let individual_reads = store.reads;
    store.reads = 0;
    let fields = lookup_object_fields(&mut store, &root, &keys).unwrap();
    assert_eq!(fields.values().cloned().collect::<Vec<_>>(), singles);
    eprintln!(
        "field reads: individual={individual_reads}, exact={}",
        store.reads
    );
    // 根、每个序列节点、字段、键与值头；不得展开1024元素的值子树。
    assert!(store.reads <= 12 * keys.len() + 4, "{} reads", store.reads);
    assert!(store.reads * 4 < individual_reads);
    let subset = newchan_rust::s_content::path::lookup_object_fields_subset(
        &mut store,
        &root,
        &[keys[19], keys[0]],
    )
    .unwrap();
    assert_eq!(subset.len(), 2);
    assert_eq!(subset[keys[0]], singles[0]);
    assert_eq!(subset[keys[19]], singles[19]);
    assert!(newchan_rust::s_content::path::lookup_object_fields_subset(
        &mut store,
        &root,
        &["absent"],
    )
    .is_err());
    assert!(lookup_object_fields(&mut store, &root, &keys[..19]).is_err());
    let mut wrong = keys.clone();
    wrong[0] = "wrong";
    assert!(lookup_object_fields(&mut store, &root, &wrong).is_err());
    let mut duplicate = keys.clone();
    duplicate[0] = keys[1];
    assert!(lookup_object_fields(&mut store, &root, &duplicate).is_err());
    for bad in [
        ContentRef {
            kind: Kind::Array,
            ..root.clone()
        },
        ContentRef {
            extent: Some(21),
            ..root.clone()
        },
        ContentRef {
            format: "bad".into(),
            ..root.clone()
        },
    ] {
        assert!(lookup_object_fields(&mut store, &bad, &keys).is_err());
    }
    store.read_cap = Some(store.reads + 5);
    assert_eq!(
        lookup_object_fields(&mut store, &root, &keys)
            .unwrap_err()
            .code(),
        BUDGET_CODE
    );
}

#[test]
fn bulk_fields_reject_hash_valid_but_noncanonical_sequence() {
    let mut store = MeteredStore::new();
    let names = (0..20).map(|n| format!("key-{n:02}")).collect::<Vec<_>>();
    let root = write_value(
        &mut store,
        &Value::Object(names.iter().map(|k| (k.clone(), Value::Null)).collect()),
    )
    .unwrap()
    .content_ref();
    let (_, object_body) = store.inner.get(&root.digest().unwrap()).unwrap();
    let mut object_body = object_body.to_vec();
    let seq: NodeDigest = object_body[8..40].try_into().unwrap();
    let (_, seq_body) = store.inner.get(&seq).unwrap();
    let mut wrong_seq = seq_body.to_vec();
    // n=20要求左16/右4，交换仍有合法摘要却违反唯一分解。
    wrong_seq[8..40].copy_from_slice(&seq_body[40..72]);
    wrong_seq[40..72].copy_from_slice(&seq_body[8..40]);
    let digest = frame(tag::SEQ_BRANCH, &wrong_seq);
    store.put(digest, tag::SEQ_BRANCH, &wrong_seq).unwrap();
    object_body[8..40].copy_from_slice(&digest);
    let digest = frame(tag::OBJECT, &object_body);
    store.put(digest, tag::OBJECT, &object_body).unwrap();
    let bad = ContentRef {
        root: newchan_rust::s_content::root_string(&digest),
        ..root
    };
    let keys = names.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(lookup_object_fields(&mut store, &bad, &keys).is_err());
    assert!(newchan_rust::s_content::path::lookup_object_fields_subset(
        &mut store,
        &bad,
        &[keys[0], keys[19]],
    )
    .is_err());
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 合法 batch_id 形状：`batch3-` + 64 位小写十六进制（§5.3）。
fn batch_id(seed: u8) -> String {
    format!("batch3-{}", hex_of(&[seed; 32]))
}

fn number(token: &str) -> Value {
    Value::Number(NumberToken::parse(token).expect("合法数字 token"))
}

/// §5.4 允许占位的位置形状：本批新生对象 batch_id、首次观察 batch_id、
/// catalog_evidence.batch_id、当前发布 index_frontier 的嵌套副本。
fn release_template() -> Value {
    Value::Object(vec![
        (
            "generated_state".to_string(),
            Value::Object(vec![
                ("generation".to_string(), number("7")),
                ("batch_id".to_string(), Value::BatchSelf),
            ]),
        ),
        (
            "observations".to_string(),
            Value::Array(vec![Value::BatchSelf, Value::string("x")]),
        ),
    ])
}

/// 读/写仪表（NodeSource + NodeSink ⟹ NodeStore）：计数真实节点读写；
/// 可选硬上限在越界时返回 `test_budget`，证明上限真的在拦而不是被绕过。
#[derive(Default)]
struct MeteredStore {
    inner: MemoryNodes,
    reads: usize,
    writes: usize,
    read_cap: Option<usize>,
    write_cap: Option<usize>,
}

impl MeteredStore {
    fn new() -> Self {
        Self::default()
    }
}

impl NodeSource for MeteredStore {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.reads += 1;
        if let Some(cap) = self.read_cap {
            if self.reads > cap {
                return Err(CodecError::new(
                    BUDGET_CODE,
                    format!("读次数 {} 超过仪表上限 {cap}", self.reads),
                ));
            }
        }
        self.inner.read(digest)
    }
}

impl NodeSink for MeteredStore {
    fn put(&mut self, digest: NodeDigest, t: u8, body: &[u8]) -> Result<(), CodecError> {
        self.writes += 1;
        if let Some(cap) = self.write_cap {
            if self.writes > cap {
                return Err(CodecError::new(
                    BUDGET_CODE,
                    format!("写次数 {} 超过仪表上限 {cap}", self.writes),
                ));
            }
        }
        self.inner.put(digest, t, body)
    }
}

/// 2^bits 个同一 0x08 占位叶的规范数组：按 concat 自合并构造（不物化逻辑叶）。
fn big_placeholder_array(bits: u32) -> (MeteredStore, ContentRef) {
    let mut store = MeteredStore::new();
    let placeholder = frame(tag::BATCH_SELF, &[]);
    store
        .put(placeholder, tag::BATCH_SELF, &[])
        .expect("写占位节点");
    let leaf = frame(tag::SEQ_LEAF, &placeholder);
    store
        .put(leaf, tag::SEQ_LEAF, &placeholder)
        .expect("写单叶序列");
    let mut seq = SequenceRef::from_parts_unchecked(leaf, 1);
    for _ in 0..bits {
        seq = sequence_concat(&mut store, &seq, &seq).expect("同根规范合并");
    }
    let count = seq.count();
    let mut body = Vec::new();
    body.extend_from_slice(&checked_frame_len(count).expect("叶数上限"));
    body.extend_from_slice(&seq.root());
    let array = frame(tag::ARRAY, &body);
    store.put(array, tag::ARRAY, &body).expect("写数组节点");
    (store, ContentRef::new(Kind::Array, &array, Some(count)))
}

#[test]
fn nested_field_index_and_json_text_lookup_follow_actual_nodes() {
    let mut store = MemoryNodes::new();
    let built = write_template(&mut store, &release_template()).expect("写发布模板");
    let reference = built.content_ref();

    let state = lookup_path(
        &mut store,
        &reference,
        &[PathStep::Field("generated_state".to_string())],
    )
    .expect("定位 generated_state");
    assert_eq!(state.kind, Kind::Object);
    assert_eq!(state.extent, Some(2));
    verify_template(&mut store, &state).expect("对象子树仍是模板上下文");

    let batch = lookup_path(
        &mut store,
        &reference,
        &[
            PathStep::Field("generated_state".to_string()),
            PathStep::Field("batch_id".to_string()),
        ],
    )
    .expect("定位 batch_id 占位");
    assert_eq!(batch.kind, Kind::TemplateBatch);
    assert_eq!(batch.extent, None);
    assert_eq!(batch.digest().expect("摘要"), frame(tag::BATCH_SELF, &[]));

    let first = lookup_path(
        &mut store,
        &reference,
        &[
            PathStep::Field("observations".to_string()),
            PathStep::Index(0),
        ],
    )
    .expect("定位 observations[0]");
    assert_eq!(first.kind, Kind::TemplateBatch);

    let second = lookup_path(
        &mut store,
        &reference,
        &[
            PathStep::Field("observations".to_string()),
            PathStep::Index(1),
        ],
    )
    .expect("定位 observations[1]");
    assert_eq!(second.kind, Kind::String);
    assert_eq!(second.extent, Some(1));
    // 返回的引用与既有核验器对拍：公开值可核，占位只在模板上下文可核。
    verify(&mut store, &second).expect("字符串元素是公开逻辑值");
    assert_eq!(
        verify(&mut store, &batch)
            .expect_err("公开上下文拒绝占位")
            .code(),
        code::TEMPLATE_PLACEHOLDER
    );
    verify_template(&mut store, &batch).expect("模板上下文接受占位");

    // §5.6：json_text 必须显式进入；进入后可按普通路径继续读。
    let mut with_text = MemoryNodes::new();
    let built = write_template(
        &mut with_text,
        &Value::Object(vec![
            (
                "view".to_string(),
                Value::JsonText(Box::new(JsonValue::Object(vec![(
                    "text".to_string(),
                    JsonValue::Number(NumberToken::parse("1").expect("数字")),
                )]))),
            ),
            ("batch_id".to_string(), Value::BatchSelf),
        ]),
    )
    .expect("写含文本视图的模板");
    let reference = built.content_ref();
    let inner = lookup_path(
        &mut with_text,
        &reference,
        &[PathStep::Field("view".to_string()), PathStep::JsonText],
    )
    .expect("显式进入文本视图");
    assert_eq!(inner.kind, Kind::Object);
    assert_eq!(inner.extent, Some(1));
    let text = lookup_path(
        &mut with_text,
        &reference,
        &[
            PathStep::Field("view".to_string()),
            PathStep::JsonText,
            PathStep::Field("text".to_string()),
        ],
    )
    .expect("文本视图内部字段");
    assert_eq!(text.kind, Kind::Number);
    // 不声明 json_text 就想取内部字段：步骤类型与当前节点不符。
    assert_eq!(
        lookup_path(
            &mut with_text,
            &reference,
            &[
                PathStep::Field("view".to_string()),
                PathStep::Field("text".to_string()),
            ],
        )
        .expect_err("文本视图必须显式进入")
        .code(),
        path_code::PATH_STEP_KIND
    );
}

#[test]
fn lookup_rejects_missing_out_of_range_and_wrong_step_kind() {
    let mut store = MemoryNodes::new();
    let built = write_template(&mut store, &release_template()).expect("写发布模板");
    let reference = built.content_ref();

    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[PathStep::Field("nope".to_string())],
        )
        .expect_err("缺字段必须拒")
        .code(),
        path_code::PATH_FIELD_MISSING
    );
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[
                PathStep::Field("observations".to_string()),
                PathStep::Index(2),
            ],
        )
        .expect_err("越界必须拒")
        .code(),
        path_code::PATH_INDEX_RANGE
    );
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[
                PathStep::Field("observations".to_string()),
                PathStep::Index(u64::MAX),
            ],
        )
        .expect_err("u64::MAX 位置必须拒")
        .code(),
        path_code::PATH_INDEX_RANGE
    );
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[
                PathStep::Field("observations".to_string()),
                PathStep::Field("x".to_string()),
            ],
        )
        .expect_err("数组上取字段必须拒")
        .code(),
        path_code::PATH_STEP_KIND
    );
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::Index(0)])
            .expect_err("对象上取下标必须拒")
            .code(),
        path_code::PATH_STEP_KIND
    );
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[
                PathStep::Field("observations".to_string()),
                PathStep::Index(1),
                PathStep::Field("x".to_string()),
            ],
        )
        .expect_err("字符串上取字段必须拒")
        .code(),
        path_code::PATH_STEP_KIND
    );
    // 未声明 json_text 就进文本视图。
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[
                PathStep::Field("observations".to_string()),
                PathStep::JsonText
            ],
        )
        .expect_err("数组不是文本视图")
        .code(),
        path_code::PATH_STEP_KIND
    );
}

#[test]
fn lookup_rejects_forged_claims_digests_and_missing_nodes() {
    let mut store = MemoryNodes::new();
    let built = write_template(&mut store, &release_template()).expect("写发布模板");
    let reference = built.content_ref();

    let mut wrong_kind = reference.clone();
    wrong_kind.kind = Kind::Array;
    assert_eq!(
        lookup_path(&mut store, &wrong_kind, &[])
            .expect_err("kind 与真实 tag 不符")
            .code(),
        code::KIND
    );
    let mut wrong_extent = reference.clone();
    wrong_extent.extent = Some(999);
    assert_eq!(
        lookup_path(&mut store, &wrong_extent, &[])
            .expect_err("extent 与真实 header 不符")
            .code(),
        code::EXTENT
    );
    let mut wrong_format = reference.clone();
    wrong_format.format = "s-content/2".to_string();
    assert_eq!(
        lookup_path(&mut store, &wrong_format, &[])
            .expect_err("format 不符")
            .code(),
        code::FORMAT
    );
    let mut upper_root = reference.clone();
    upper_root.root = upper_root.root.to_uppercase();
    assert_eq!(
        lookup_path(&mut store, &upper_root, &[])
            .expect_err("根文本大小写不符")
            .code(),
        code::ROOT
    );

    // 摘要伪造：摘要与 (tag, body) 重算不符。
    let mut forged = MemoryNodes::new();
    let claimed = frame(tag::NULL, &[]);
    forged.insert_unchecked(claimed, tag::STRING, vec![0u8; 40]);
    let forged_ref = ContentRef::new(Kind::String, &claimed, Some(0));
    assert_eq!(
        lookup_path(&mut forged, &forged_ref, &[])
            .expect_err("摘要伪造必须拒")
            .code(),
        code::NODE_DIGEST
    );

    // 节点缺失：根摘要不在读集里。
    let mut empty = MemoryNodes::new();
    let missing = frame(tag::NULL, &[]);
    let missing_ref = ContentRef::new(Kind::Null, &missing, None);
    assert_eq!(
        lookup_path(&mut empty, &missing_ref, &[])
            .expect_err("缺节点必须拒")
            .code(),
        code::NODE_MISSING
    );
}

#[test]
fn declared_count_overflow_and_shape_mismatch_are_rejected() {
    // ① 声明长度超 2^63-1：不是规范里能表达的数组。
    let mut store = MemoryNodes::new();
    let placeholder = frame(tag::BATCH_SELF, &[]);
    store.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
    let leaf = frame(tag::SEQ_LEAF, &placeholder);
    store.insert_unchecked(leaf, tag::SEQ_LEAF, placeholder.to_vec());
    let mut body = Vec::new();
    body.extend_from_slice(&u64::MAX.to_be_bytes());
    body.extend_from_slice(&leaf);
    let array = frame(tag::ARRAY, &body);
    store.insert_unchecked(array, tag::ARRAY, body.clone());
    let reference = ContentRef::new(Kind::Array, &array, Some(u64::MAX));
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::Index(0)])
            .expect_err("声明长度溢出必须拒")
            .code(),
        code::LENGTH_OVERFLOW
    );

    // ② 分支声明 n 与到达处的期望叶数不符。
    let mut store = MemoryNodes::new();
    store.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
    store.insert_unchecked(leaf, tag::SEQ_LEAF, placeholder.to_vec());
    let mut branch_body = Vec::new();
    branch_body.extend_from_slice(&3u64.to_be_bytes());
    branch_body.extend_from_slice(&leaf);
    branch_body.extend_from_slice(&leaf);
    let branch = frame(tag::SEQ_BRANCH, &branch_body);
    store.insert_unchecked(branch, tag::SEQ_BRANCH, branch_body.clone());
    let mut array_body = Vec::new();
    array_body.extend_from_slice(&checked_frame_len(2).expect("长度"));
    array_body.extend_from_slice(&branch);
    let array = frame(tag::ARRAY, &array_body);
    store.insert_unchecked(array, tag::ARRAY, array_body.clone());
    let reference = ContentRef::new(Kind::Array, &array, Some(2));
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::Index(0)])
            .expect_err("分支声明与期望叶数不符必须拒")
            .code(),
        code::SEQUENCE_SHAPE
    );

    // ③ 对象字段序列的叶必须是 0x14 字段节点。
    let mut store = MemoryNodes::new();
    let null = frame(tag::NULL, &[]);
    store.insert_unchecked(null, tag::NULL, Vec::new());
    let leaf = frame(tag::SEQ_LEAF, &null);
    store.insert_unchecked(leaf, tag::SEQ_LEAF, null.to_vec());
    let mut object_body = Vec::new();
    object_body.extend_from_slice(&checked_frame_len(1).expect("长度"));
    object_body.extend_from_slice(&leaf);
    let object = frame(tag::OBJECT, &object_body);
    store.insert_unchecked(object, tag::OBJECT, object_body.clone());
    let reference = ContentRef::new(Kind::Object, &object, Some(1));
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::Field("a".to_string())],)
            .expect_err("字段叶标签不符必须拒")
            .code(),
        code::SEQUENCE_SHAPE
    );
}

#[test]
fn replace_touches_only_the_named_path_and_keeps_old_root_readable() {
    let mut store = MemoryNodes::new();
    let shared = Value::Object(vec![
        ("batch_id".to_string(), Value::BatchSelf),
        ("keep".to_string(), number("1")),
    ]);
    let built = write_template(&mut store, &Value::Array(vec![shared.clone(), shared]))
        .expect("写共享子树模板");
    let old_root = built.content_ref();
    let id = batch_id(0xab);

    let before_left =
        lookup_path(&mut store, &old_root, &[PathStep::Index(0)]).expect("旧根左子树");
    let before_right =
        lookup_path(&mut store, &old_root, &[PathStep::Index(1)]).expect("旧根右子树");
    assert_eq!(before_left.root, before_right.root, "同一子树在两点共享");

    let new_root = replace_batch_placeholder(
        &mut store,
        &old_root,
        &[PathStep::Index(1), PathStep::Field("batch_id".to_string())],
        &id,
    )
    .expect("定点替换第二份子树");
    assert_ne!(new_root.root, old_root.root, "被改路径必须产生新根");
    assert_eq!(new_root.kind, Kind::Array);
    assert_eq!(new_root.extent, Some(2));

    // 指定路径变。
    let replaced = lookup_path(
        &mut store,
        &new_root,
        &[PathStep::Index(1), PathStep::Field("batch_id".to_string())],
    )
    .expect("新根指定路径");
    assert_eq!(replaced.kind, Kind::String);
    assert_eq!(replaced.extent, Some(id.len() as u64));
    // 同一下标 0 的共享子树不动（逐位同一个根）。
    let after_left = lookup_path(&mut store, &new_root, &[PathStep::Index(0)]).expect("新根左子树");
    assert_eq!(
        after_left.root, before_left.root,
        "未指定路径的共享子树不得重建"
    );
    let still = lookup_path(
        &mut store,
        &new_root,
        &[PathStep::Index(0), PathStep::Field("batch_id".to_string())],
    )
    .expect("未指定路径仍是占位");
    assert_eq!(still.kind, Kind::TemplateBatch);
    // 旧根与旧闭包不变：仍可在模板上下文核验、仍指向原占位。
    verify_template(&mut store, &old_root).expect("旧根仍可核");
    let old_still = lookup_path(
        &mut store,
        &old_root,
        &[PathStep::Index(1), PathStep::Field("batch_id".to_string())],
    )
    .expect("旧根仍指向占位");
    assert_eq!(old_still.kind, Kind::TemplateBatch);
    // 新根仍是模板（还有未解析占位），公开核验必须拒。
    verify_template(&mut store, &new_root).expect("新根仍是可核模板");
    assert_eq!(
        verify(&mut store, &new_root)
            .expect_err("未解析占位不能当公开值")
            .code(),
        code::TEMPLATE_PLACEHOLDER
    );
}

#[test]
fn replace_then_verify_and_render_match_independent_oracle() {
    let id = batch_id(0x5a);
    let mut store = MemoryNodes::new();
    let built = write_template(
        &mut store,
        &Value::Object(vec![
            ("batch_id".to_string(), Value::BatchSelf),
            ("n".to_string(), number("1")),
            ("scope".to_string(), Value::string("s")),
        ]),
    )
    .expect("写单占位模板");
    let template = built.content_ref();

    let resolved = replace_batch_placeholder(
        &mut store,
        &template,
        &[PathStep::Field("batch_id".to_string())],
        &id,
    )
    .expect("解析唯一占位");
    assert_eq!(resolved.kind, Kind::Object);
    assert_eq!(resolved.extent, Some(3));
    verify(&mut store, &resolved).expect("解析后是公开逻辑值");

    // oracle ①：用 write_value 独立构造同一公开值，摘要逐字相等。
    let mut oracle = MemoryNodes::new();
    let expected = write_value(
        &mut oracle,
        &Value::Object(vec![
            ("batch_id".to_string(), Value::string(id.clone())),
            ("n".to_string(), number("1")),
            ("scope".to_string(), Value::string("s")),
        ]),
    )
    .expect("独立构造期望值");
    assert_eq!(expected.digest, resolved.digest().expect("摘要"));

    // oracle ②：规范渲染文本逐字相等（对象键按 UTF-8 升序）。
    let mut rendered = Vec::new();
    render_canonical(&mut store, &resolved, &mut rendered).expect("渲染");
    let text = String::from_utf8(rendered).expect("UTF-8");
    assert_eq!(text, format!(r#"{{"batch_id":"{id}","n":1,"scope":"s"}}"#));

    // 旧模板根不因替换而改变。
    assert_eq!(
        verify(&mut store, &template)
            .expect_err("模板根仍含占位")
            .code(),
        code::TEMPLATE_PLACEHOLDER
    );
    let still = lookup_path(
        &mut store,
        &template,
        &[PathStep::Field("batch_id".to_string())],
    )
    .expect("旧根仍指向占位");
    assert_eq!(still.kind, Kind::TemplateBatch);
}

#[test]
fn replace_rejects_bad_batch_id_and_non_placeholder_targets_without_writing() {
    let mut store = MemoryNodes::new();
    let built = write_template(
        &mut store,
        &Value::Object(vec![
            ("batch_id".to_string(), Value::BatchSelf),
            ("note".to_string(), Value::string("n")),
        ]),
    )
    .expect("写模板");
    let reference = built.content_ref();
    let path = [PathStep::Field("batch_id".to_string())];

    let bad_ids = vec![
        String::new(),
        "batch3-".to_string(),
        format!("batch3-{}", "a".repeat(63)),
        format!("batch3-{}", "a".repeat(65)),
        format!("batch3-{}", "A".repeat(64)),
        format!("batch4-{}", "a".repeat(64)),
        format!(" batch3-{}", "a".repeat(64)),
    ];
    for bad in &bad_ids {
        let before = store.len();
        let err = replace_batch_placeholder(&mut store, &reference, &path, bad)
            .expect_err("坏 batch_id 必须拒");
        assert_eq!(err.code(), path_code::BATCH_ID, "`{bad}`");
        assert_eq!(store.len(), before, "拒绝时不得写节点（`{bad}`）");
    }

    // 目标不是 0x08。
    let before = store.len();
    let err = replace_batch_placeholder(
        &mut store,
        &reference,
        &[PathStep::Field("note".to_string())],
        &batch_id(0x01),
    )
    .expect_err("非占位目标必须拒");
    assert_eq!(err.code(), code::VALUE_KIND);
    assert_eq!(store.len(), before);

    // 目标缺失。
    let before = store.len();
    let err = replace_batch_placeholder(
        &mut store,
        &reference,
        &[PathStep::Field("nope".to_string())],
        &batch_id(0x01),
    )
    .expect_err("目标缺失必须拒");
    assert_eq!(err.code(), path_code::PATH_FIELD_MISSING);
    assert_eq!(store.len(), before);

    // 替换路径不得进入 json_text（即使目标本该是占位）。
    let mut with_text = MemoryNodes::new();
    let built = write_template(
        &mut with_text,
        &Value::Object(vec![(
            "view".to_string(),
            Value::JsonText(Box::new(JsonValue::Object(vec![(
                "batch_id".to_string(),
                JsonValue::String("x".to_string()),
            )]))),
        )]),
    )
    .expect("写文本视图模板");
    let reference = built.content_ref();
    let before = with_text.len();
    let err = replace_batch_placeholder(
        &mut with_text,
        &reference,
        &[PathStep::Field("view".to_string()), PathStep::JsonText],
        &batch_id(0x02),
    )
    .expect_err("替换路径禁入 json_text");
    assert_eq!(err.code(), path_code::PATH_JSON_TEXT);
    assert_eq!(with_text.len(), before, "拒绝时不得写节点");

    // 0x08 body 必须为空：手写敌意节点（合法摘要、非法形状）。
    let mut hostile = MemoryNodes::new();
    let bad_placeholder = frame(tag::BATCH_SELF, &[0x00]);
    hostile.insert_unchecked(bad_placeholder, tag::BATCH_SELF, vec![0x00]);
    let reference = ContentRef::new(Kind::TemplateBatch, &bad_placeholder, None);
    assert_eq!(
        replace_batch_placeholder(&mut hostile, &reference, &[], &batch_id(0x03))
            .expect_err("占位 body 必须为空")
            .code(),
        code::BODY_SHAPE
    );
}

#[test]
fn json_text_inner_placeholder_and_nested_view_are_rejected() {
    // 手写敌意图：0x07 → 0x08（文本视图内部出现占位）。
    let mut store = MemoryNodes::new();
    let placeholder = frame(tag::BATCH_SELF, &[]);
    store.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
    let json_text = frame(tag::JSON_TEXT, &placeholder);
    store.insert_unchecked(json_text, tag::JSON_TEXT, placeholder.to_vec());
    let reference = ContentRef::new(Kind::JsonText, &json_text, None);
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::JsonText])
            .expect_err("文本视图内部不得有占位")
            .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );

    // 0x07 → 0x07（嵌套文本视图）。
    let inner_text = frame(tag::JSON_TEXT, &json_text);
    store.insert_unchecked(inner_text, tag::JSON_TEXT, json_text.to_vec());
    let reference = ContentRef::new(Kind::JsonText, &inner_text, None);
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::JsonText])
            .expect_err("文本视图内部不得再嵌套文本视图")
            .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );

    // 0x07 body 必须是内部值的 32 字节摘要。
    let bad_text = frame(tag::JSON_TEXT, &[0x00]);
    store.insert_unchecked(bad_text, tag::JSON_TEXT, vec![0x00]);
    let reference = ContentRef::new(Kind::JsonText, &bad_text, None);
    assert_eq!(
        lookup_path(&mut store, &reference, &[PathStep::JsonText])
            .expect_err("文本视图 body 形状不符")
            .code(),
        code::BODY_SHAPE
    );

    // 深层 ①：0x07 → 0x06（字段 a）→ 0x08：文本视图**内部任意深度**都不得有占位。
    let mut store = MemoryNodes::new();
    let mut keys = MemoryNodes::new();
    let key = write_value(&mut keys, &Value::string("a"))
        .expect("写键字符串")
        .digest;
    for (d, t, b) in keys.iter() {
        store.insert_unchecked(*d, t, b.to_vec());
    }
    let placeholder = frame(tag::BATCH_SELF, &[]);
    store.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
    let mut field_body = [0u8; 64];
    field_body[..32].copy_from_slice(&key);
    field_body[32..].copy_from_slice(&placeholder);
    let field = frame(tag::OBJECT_FIELD, &field_body);
    store.insert_unchecked(field, tag::OBJECT_FIELD, field_body.to_vec());
    let seq_leaf = frame(tag::SEQ_LEAF, &field);
    store.insert_unchecked(seq_leaf, tag::SEQ_LEAF, field.to_vec());
    let mut object_body = Vec::new();
    object_body.extend_from_slice(&checked_frame_len(1).expect("长度"));
    object_body.extend_from_slice(&seq_leaf);
    let object = frame(tag::OBJECT, &object_body);
    store.insert_unchecked(object, tag::OBJECT, object_body.clone());
    let json_text = frame(tag::JSON_TEXT, &object);
    store.insert_unchecked(json_text, tag::JSON_TEXT, object.to_vec());
    let reference = ContentRef::new(Kind::JsonText, &json_text, None);
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[PathStep::JsonText, PathStep::Field("a".to_string()),],
        )
        .expect_err("文本视图深层占位必须拒")
        .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );

    // 深层 ②：0x07 → 0x05（1 元素）→ 0x07：内部再嵌套文本视图同样拒。
    let mut store = MemoryNodes::new();
    let null = frame(tag::NULL, &[]);
    store.insert_unchecked(null, tag::NULL, Vec::new());
    let inner_text = frame(tag::JSON_TEXT, &null);
    store.insert_unchecked(inner_text, tag::JSON_TEXT, null.to_vec());
    let elem_leaf = frame(tag::SEQ_LEAF, &inner_text);
    store.insert_unchecked(elem_leaf, tag::SEQ_LEAF, inner_text.to_vec());
    let mut array_body = Vec::new();
    array_body.extend_from_slice(&checked_frame_len(1).expect("长度"));
    array_body.extend_from_slice(&elem_leaf);
    let array = frame(tag::ARRAY, &array_body);
    store.insert_unchecked(array, tag::ARRAY, array_body.clone());
    let outer_text = frame(tag::JSON_TEXT, &array);
    store.insert_unchecked(outer_text, tag::JSON_TEXT, array.to_vec());
    let reference = ContentRef::new(Kind::JsonText, &outer_text, None);
    assert_eq!(
        lookup_path(
            &mut store,
            &reference,
            &[PathStep::JsonText, PathStep::Index(0)],
        )
        .expect_err("文本视图深层嵌套文本视图必须拒")
        .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );
}

#[test]
fn large_shared_array_point_replacement_stays_bounded() {
    const BITS: u32 = 50;
    let bound = 8 * (BITS as usize + 2);
    let id = batch_id(0x5a);

    let (mut store, reference) = big_placeholder_array(BITS);
    let count = reference.extent.expect("数组长度");
    assert_eq!(count, 1u64 << BITS);
    assert!(
        store.inner.len() < 4 * (BITS as usize + 2),
        "构造期不得物化逻辑叶：实际节点 {}",
        store.inner.len()
    );
    // 读/写都用同一个仪表硬上限拦：上限是「不展开全叶」的证明装置，不是生产默认值。
    let reads_before = store.reads;
    let writes_before = store.writes;
    store.read_cap = Some(reads_before + bound);
    store.write_cap = Some(writes_before + bound);
    let replaced =
        replace_batch_placeholder(&mut store, &reference, &[PathStep::Index(count - 1)], &id)
            .expect("大数组定点替换");
    let reads = store.reads - reads_before;
    let writes = store.writes - writes_before;
    assert!(reads <= bound, "读次数 {reads} 超过仪表上限 {bound}");
    assert!(writes <= bound, "写次数 {writes} 超过仪表上限 {bound}");
    assert!(reads < (1 << 20), "读次数 {reads} 说明展开了逻辑叶");
    assert!(writes < (1 << 20), "写次数 {writes} 说明展开了逻辑叶");
    store.read_cap = None;
    store.write_cap = None;
    assert!(
        store.inner.len() < 4 * (BITS as usize + 2),
        "替换后仍不得物化逻辑叶：实际节点 {}",
        store.inner.len()
    );

    let last = lookup_path(&mut store, &replaced, &[PathStep::Index(count - 1)]).expect("新位置");
    assert_eq!(last.kind, Kind::String);
    assert_eq!(last.extent, Some(id.len() as u64));
    let first = lookup_path(&mut store, &replaced, &[PathStep::Index(0)]).expect("未动位置");
    assert_eq!(first.kind, Kind::TemplateBatch, "其余同摘要占位不动");
    verify_template(&mut store, &replaced).expect("大模板仍可全闭包核验");

    // 字段查找不得展开字段值大子图：{a:1, zz:<2^50 数组>}。
    let (mut store, big) = big_placeholder_array(BITS);
    let built = write_template(
        &mut store,
        &Value::Object(vec![
            ("a".to_string(), number("1")),
            ("zz".to_string(), Value::Edge(big.digest().expect("摘要"))),
        ]),
    )
    .expect("写含大字段的对象");
    let object = built.content_ref();
    assert_eq!(object.extent, Some(2));
    let reads_before = store.reads;
    store.read_cap = Some(reads_before + bound);
    let found =
        lookup_path(&mut store, &object, &[PathStep::Field("zz".to_string())]).expect("定位大字段");
    let reads = store.reads - reads_before;
    assert!(
        reads <= bound,
        "字段查找读次数 {reads} 超过仪表上限 {bound}"
    );
    store.read_cap = None;
    assert_eq!(found.kind, Kind::Array);
    assert_eq!(found.extent, Some(1u64 << BITS));
    assert!(
        reads <= bound,
        "字段查找读次数 {reads} 超过仪表上限 {bound}（不得展开值子图）"
    );
    assert!(
        lookup_path(&mut store, &object, &[PathStep::Field("yy".to_string())])
            .expect_err("缺字段必须拒")
            .code()
            == path_code::PATH_FIELD_MISSING
    );

    // 仪表负控：同一构型在极小上限下必须触发 test_budget（上限真的在拦）。
    let (mut low, low_reference) = big_placeholder_array(BITS);
    let low_count = low_reference.extent.expect("数组长度");
    low.read_cap = Some(low.reads + 4);
    assert_eq!(
        replace_batch_placeholder(
            &mut low,
            &low_reference,
            &[PathStep::Index(low_count - 1)],
            &id,
        )
        .expect_err("低上限必须拦")
        .code(),
        BUDGET_CODE
    );
}
