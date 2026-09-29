#![cfg(feature = "s_session")]
//! #1394 C §5.1–5.3 的独立向量契约。
//!
//! 验收面（照实，不含服务义务）：23 个正例根、22 个可渲染值的规范字节、
//! 正例可达的 96 个节点与向量逐项一致、10 个非法根全部拒绝，
//! 以及向量未覆盖而公开入口会遇到的类型/重复/缺失/范围错误。
//!
//! 向量文件是已批准输入（sha256 见下），不是本实现自己产出的期望值。

use newchan_rust::s_content::{
    self, code, ContentRef, JsonValue, Kind, MemoryNodes, NodeRead, NodeSource, NumberToken, Value,
};
use serde_json::Value as J;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const VECTORS_SHA256: &str = "78984501cbb465be8251e3d212e34c0cb6fb33a5aab9a4999a3f1db592ce71f0";
const EXPECTED_POSITIVE: usize = 23;
const EXPECTED_INVALID: usize = 10;
const EXPECTED_NODES: usize = 123;
const EXPECTED_REACHABLE: usize = 96;
const EXPECTED_RENDERABLE: usize = 22;

#[test]
fn template_batch_rechecks_root_claims_and_json_inner_flavor() {
    use s_content::NodeSink;
    let mut nodes = MemoryNodes::new();
    let root = s_content::write_template(&mut nodes, &Value::Object(vec![
        ("batch".into(), Value::BatchSelf),
    ])).unwrap().content_ref();
    s_content::verify_template_many(&mut nodes, [root.clone(), root.clone()]).unwrap();
    assert!(s_content::verify_many(&mut nodes, [root.clone()]).is_err());
    let wrong = ContentRef { extent: Some(2), ..root.clone() };
    assert!(s_content::verify_template_many(&mut nodes, [root.clone(), wrong]).is_err());
    let batch = s_content::write_template(&mut nodes, &Value::BatchSelf).unwrap();
    let digest = s_content::frame(s_content::tag::JSON_TEXT, &batch.digest);
    nodes.put(digest, s_content::tag::JSON_TEXT, &batch.digest).unwrap();
    let inner = s_content::content_ref_of(&mut nodes, &s_content::root_string(&digest)).unwrap();
    // 第一次Value上下文通过不能让第二次JsonInner上下文复用成通过。
    assert!(s_content::verify_template_many(&mut nodes, [batch.content_ref(), inner]).is_err());
}

fn vectors_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../.chanlun/review-results/issue1394-content-v3-20260916/inputs/content-format-vectors.json",
    )
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_to_bytes(text: &str) -> Vec<u8> {
    assert!(text.len() % 2 == 0, "十六进制长度必须为偶");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("十六进制"))
        .collect()
}

fn load_vectors() -> J {
    let path = vectors_path();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("读取独立向量 {} 失败：{e}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex_of(&hasher.finalize());
    assert_eq!(got, VECTORS_SHA256, "向量文件不是已批准的那一份");
    serde_json::from_slice(&bytes).expect("向量 JSON 可解析")
}

/// 向量节点表：root 文本 → (tag, body)。
fn node_index(vectors: &J) -> HashMap<String, (u8, Vec<u8>)> {
    let records = vectors["nodes"].as_array().expect("nodes 数组");
    assert_eq!(records.len(), EXPECTED_NODES, "向量节点数");
    records
        .iter()
        .map(|record| {
            let root = record["root"].as_str().expect("root").to_string();
            let tag = u8::from_str_radix(record["tag"].as_str().expect("tag"), 16).expect("tag 十六进制");
            let body = hex_to_bytes(record["body_hex"].as_str().expect("body_hex"));
            (root, (tag, body))
        })
        .collect()
}

/// 单节点引用的子摘要（按 §5.2 的内部 framing）。
fn child_roots(tag: u8, body: &[u8]) -> Vec<String> {
    let root = |bytes: &[u8]| format!("sc1-{}", hex_of(bytes));
    match tag {
        0x00 | 0x01 | 0x02 | 0x08 | 0x10 | 0x11 => Vec::new(),
        0x03 | 0x04 | 0x05 | 0x06 => vec![root(&body[8..40])],
        0x07 | 0x12 => vec![root(body)],
        0x13 => vec![root(&body[8..40]), root(&body[40..72])],
        0x14 => vec![root(&body[0..32]), root(&body[32..64])],
        other => panic!("向量出现未登记 tag {other:02x}"),
    }
}

fn reachable_roots(roots: &[String], index: &HashMap<String, (u8, Vec<u8>)>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack: Vec<String> = roots.to_vec();
    while let Some(root) = stack.pop() {
        if !seen.insert(root.clone()) {
            continue;
        }
        let (tag, body) = index
            .get(&root)
            .unwrap_or_else(|| panic!("向量缺节点 {root}"));
        for child in child_roots(*tag, body) {
            stack.push(child);
        }
    }
    seen
}

fn positive_roots(vectors: &J) -> Vec<String> {
    vectors["positive"]
        .as_array()
        .expect("positive 数组")
        .iter()
        .map(|case| case["root"].as_str().expect("root").to_string())
        .collect()
}

fn value_from_expr(expr: &J) -> Value {
    match expr["t"].as_str().expect("表达式类型") {
        "null" => Value::Null,
        "bool" => Value::Bool(expr["value"].as_bool().expect("bool 值")),
        "string" => Value::string(expr["value"].as_str().expect("字符串值")),
        "number" => Value::Number(
            NumberToken::parse(expr["token"].as_str().expect("数字 token")).expect("合法 token"),
        ),
        "array" => Value::Array(
            expr["items"]
                .as_array()
                .expect("items")
                .iter()
                .map(value_from_expr)
                .collect(),
        ),
        "object" => Value::Object(
            expr["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(|field| {
                    let pair = field.as_array().expect("字段对");
                    (
                        pair[0].as_str().expect("键").to_string(),
                        value_from_expr(&pair[1]),
                    )
                })
                .collect(),
        ),
        "json_text" => Value::JsonText(Box::new(json_from_expr(&expr["value"]))),
        "batch_self" => Value::BatchSelf,
        other => panic!("向量出现未知表达式类型 `{other}`"),
    }
}

/// json_text 内部值：类型上就没有文本视图与批次占位。
fn json_from_expr(expr: &J) -> JsonValue {
    match expr["t"].as_str().expect("表达式类型") {
        "null" => JsonValue::Null,
        "bool" => JsonValue::Bool(expr["value"].as_bool().expect("bool 值")),
        "string" => JsonValue::String(expr["value"].as_str().expect("字符串值").to_string()),
        "number" => JsonValue::Number(
            NumberToken::parse(expr["token"].as_str().expect("数字 token")).expect("合法 token"),
        ),
        "array" => JsonValue::Array(
            expr["items"]
                .as_array()
                .expect("items")
                .iter()
                .map(json_from_expr)
                .collect(),
        ),
        "object" => JsonValue::Object(
            expr["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(|field| {
                    let pair = field.as_array().expect("字段对");
                    (
                        pair[0].as_str().expect("键").to_string(),
                        json_from_expr(&pair[1]),
                    )
                })
                .collect(),
        ),
        other => panic!("json_text 内部不允许 `{other}`"),
    }
}

struct VectorStore {
    nodes: HashMap<String, (u8, Vec<u8>)>,
}

impl NodeSource for VectorStore {
    fn read(
        &mut self,
        digest: &s_content::NodeDigest,
    ) -> Result<NodeRead, s_content::CodecError> {
        Ok(match self.nodes.get(&s_content::root_string(digest)) {
            Some((tag, body)) => NodeRead::Found {
                tag: *tag,
                body: body.clone(),
            },
            None => NodeRead::Missing,
        })
    }
}

#[test]
fn positive_vectors_roots_render_and_node_set() {
    let vectors = load_vectors();
    let positives = vectors["positive"].as_array().expect("positive 数组");
    assert_eq!(positives.len(), EXPECTED_POSITIVE);
    let index = node_index(&vectors);

    let mut store = MemoryNodes::new();
    let mut rendered = 0usize;
    for case in positives {
        let name = case["name"].as_str().expect("name");
        let root = case["root"].as_str().expect("root");
        let value = value_from_expr(&case["expression"]);
        let built = if case["expression"]["t"].as_str() == Some("batch_self") {
            // 占位只能走显式模板上下文的写入入口。
            let err = s_content::write_value(&mut store, &value)
                .expect_err("占位不得写进通用公开逻辑值");
            assert_eq!(err.code(), code::TEMPLATE_PLACEHOLDER, "{name}");
            s_content::write_template(&mut store, &value)
                .unwrap_or_else(|e| panic!("{name}：模板上下文写入失败 {e}"))
        } else {
            s_content::write_value(&mut store, &value)
                .unwrap_or_else(|e| panic!("{name}：写入失败 {e}"))
        };
        assert_eq!(built.root(), root, "{name}：根与独立向量不一致");
        let reference = built.content_ref();
        let derived = s_content::content_ref_of(&mut store, root)
            .unwrap_or_else(|e| panic!("{name}：从节点来源读根失败 {e}"));
        assert_eq!(derived, reference, "{name}：读出的 ContentRef 应与写入一致");

        match case.get("canonical_render_hex").and_then(|h| h.as_str()) {
            Some(want) => {
                let verified = s_content::verify(&mut store, &reference)
                    .unwrap_or_else(|e| panic!("{name}：闭包核验失败 {e}"));
                assert_eq!(verified.kind, reference.kind, "{name}");
                assert_eq!(verified.extent, reference.extent, "{name}");
                let mut out = Vec::new();
                s_content::render_canonical(&mut store, &reference, &mut out)
                    .unwrap_or_else(|e| panic!("{name}：渲染失败 {e}"));
                assert_eq!(hex_of(&out), want, "{name}：规范渲染字节不一致");
                rendered += 1;
            }
            None => {
                let err = s_content::verify(&mut store, &reference)
                    .expect_err("当前批次占位在公开逻辑值里必须拒绝");
                assert_eq!(err.code(), code::TEMPLATE_PLACEHOLDER, "{name}");
                s_content::verify_template(&mut store, &reference)
                    .unwrap_or_else(|e| panic!("{name}：显式模板上下文核验失败 {e}"));
                let mut out = Vec::new();
                let err = s_content::render_canonical(&mut store, &reference, &mut out)
                    .expect_err("未解析的批次占位不渲染");
                assert_eq!(err.code(), code::TEMPLATE_PLACEHOLDER, "{name}");
                assert!(out.is_empty(), "{name}：拒绝时不得写出半截渲染");
            }
        }
    }
    assert_eq!(rendered, EXPECTED_RENDERABLE, "可渲染正例数");

    // 正例可达节点集合：本实现写出的节点与独立向量逐项一致（tag + body + 摘要）。
    let reachable = reachable_roots(&positive_roots(&vectors), &index);
    assert_eq!(reachable.len(), EXPECTED_REACHABLE, "正例可达节点数");
    assert_eq!(
        store.len(),
        EXPECTED_REACHABLE,
        "写入面产出的节点集合规模应等于正例可达节点数"
    );
    for root in &reachable {
        let (tag, body) = index.get(root).expect("向量节点");
        let digest = s_content::parse_root(root).expect("根文本");
        let got = store
            .get(&digest)
            .unwrap_or_else(|| panic!("实现未产出向量里的节点 {root}"));
        assert_eq!(got.0, *tag, "{root}：tag 不一致");
        assert_eq!(got.1, body.as_slice(), "{root}：body 不一致");
        assert_eq!(
            s_content::frame(*tag, body),
            digest,
            "{root}：F(tag, body) 重算不符"
        );
    }
}

#[test]
fn invalid_encoded_roots_are_rejected_in_both_modes() {
    let vectors = load_vectors();
    let index = node_index(&vectors);
    let invalids = vectors["invalid_encoded_roots"]
        .as_array()
        .expect("invalid 数组");
    assert_eq!(invalids.len(), EXPECTED_INVALID);
    for case in invalids {
        let name = case["name"].as_str().expect("name");
        let want = case["python_error"].as_str().expect("python_error");
        // 该向量把 Python 端异常原文记为期望值；本实现的稳定码是 `utf8`。
        let want = if want.starts_with("'utf-8' codec") {
            code::UTF8
        } else {
            want
        };
        let root = case["root"].as_str().expect("root");
        let mut store = VectorStore {
            nodes: index.clone(),
        };
        let reference = s_content::content_ref_of(&mut store, root)
            .unwrap_or_else(|e| panic!("{name}：从节点来源读根失败 {e}"));
        let err = s_content::verify(&mut store, &reference)
            .expect_err(&format!("{name}：公开核验必须拒绝"));
        assert_eq!(err.code(), want, "{name}");
        let err = s_content::verify_template(&mut store, &reference)
            .expect_err(&format!("{name}：模板核验必须拒绝"));
        assert_eq!(
            err.code(),
            want,
            "{name}：模板上下文不得放行该非法图"
        );
    }
}

#[test]
fn vector_uncovered_rejections() {
    // 缺节点：根文本合法但来源里没有该摘要。
    let mut empty = MemoryNodes::new();
    let ghost = s_content::root_string(&[0u8; 32]);
    assert_eq!(
        s_content::content_ref_of(&mut empty, &ghost)
            .expect_err("缺失节点必须拒绝")
            .code(),
        code::NODE_MISSING
    );

    // 范围：声明长度超过 2^63-1。
    let mut store = MemoryNodes::new();
    let bytes_root = s_content::bytes_root(b"x");
    let mut body = Vec::new();
    body.extend_from_slice(&(1u64 << 63).to_be_bytes());
    body.extend_from_slice(&bytes_root);
    let digest = s_content::frame(0x03, &body);
    store.insert_unchecked(digest, 0x03, body);
    assert_eq!(
        s_content::content_ref_of(&mut store, &s_content::root_string(&digest))
            .expect_err("越界声明长度必须拒绝")
            .code(),
        code::LENGTH_OVERFLOW
    );

    // 未知标签与内部节点冒充根：两种拒绝码分开。
    let mut store = MemoryNodes::new();
    let odd = s_content::frame(0x42, &[]);
    store.insert_unchecked(odd, 0x42, Vec::new());
    assert_eq!(
        s_content::content_ref_of(&mut store, &s_content::root_string(&odd))
            .expect_err("未登记标签")
            .code(),
        code::UNKNOWN_TAG
    );
    let mut store = MemoryNodes::new();
    let chunk = s_content::frame(0x10, b"x");
    store.insert_unchecked(chunk, 0x10, b"x".to_vec());
    assert_eq!(
        s_content::content_ref_of(&mut store, &s_content::root_string(&chunk))
            .expect_err("块节点不能当根")
            .code(),
        code::VALUE_KIND
    );

    // 重复键：手工构造两个同键字段（写入面本身拒绝重复键，故直接注节点）。
    let mut store = MemoryNodes::new();
    let key = s_content::write_value(&mut store, &Value::string("k")).expect("键节点");
    let first = s_content::write_value(&mut store, &Value::string("a")).expect("值一");
    let second = s_content::write_value(&mut store, &Value::string("b")).expect("值二");
    let field = |key: &s_content::NodeDigest, value: &s_content::NodeDigest| {
        let mut body = [0u8; 64];
        body[..32].copy_from_slice(key);
        body[32..].copy_from_slice(value);
        (s_content::frame(0x14, &body), body)
    };
    let (f1, b1) = field(&key.digest, &first.digest);
    let (f2, b2) = field(&key.digest, &second.digest);
    store.insert_unchecked(f1, 0x14, b1.to_vec());
    store.insert_unchecked(f2, 0x14, b2.to_vec());
    let fields = s_content::write_sequence(&mut store, &[f1, f2]).expect("字段序列");
    let mut body = Vec::new();
    body.extend_from_slice(&2u64.to_be_bytes());
    body.extend_from_slice(&fields);
    let object = s_content::frame(0x06, &body);
    store.insert_unchecked(object, 0x06, body);
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&object),
        kind: Kind::Object,
        extent: Some(2),
    };
    assert_eq!(
        s_content::verify(&mut store, &reference)
            .expect_err("重复键必须拒绝")
            .code(),
        code::DUPLICATE_KEY
    );
}

// ── review finding：重复子树不得按逻辑叶展开（核验/渲染的读取必须受限） ──────

/// 读次数受上限约束的来源：超限即报错，用来证明没有按逻辑叶展开。
struct CountingNodes {
    inner: MemoryNodes,
    reads: usize,
    limit: usize,
}

impl NodeSource for CountingNodes {
    fn read(
        &mut self,
        digest: &s_content::NodeDigest,
    ) -> Result<NodeRead, s_content::CodecError> {
        self.reads += 1;
        if self.reads > self.limit {
            return Err(s_content::CodecError::new(
                "source",
                format!(
                    "读取次数 {} 超过上限 {} —— 说明按逻辑叶展开了重复子树",
                    self.reads, self.limit
                ),
            ));
        }
        self.inner.read(digest)
    }
}

/// 只接纳少量字节的 sink：写满即报错。
struct TinySink {
    remaining: usize,
    written: Vec<u8>,
}

impl std::io::Write for TinySink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "sink 已满"));
        }
        let n = buf.len().min(self.remaining);
        self.written.extend_from_slice(&buf[..n]);
        self.remaining -= n;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn put_seq_leaf(store: &mut MemoryNodes, leaf: &s_content::NodeDigest) -> s_content::NodeDigest {
    let digest = s_content::frame(s_content::tag::SEQ_LEAF, leaf);
    store.insert_unchecked(digest, s_content::tag::SEQ_LEAF, leaf.to_vec());
    digest
}

fn put_branch(
    store: &mut MemoryNodes,
    n: u64,
    left: &s_content::NodeDigest,
    right: &s_content::NodeDigest,
) -> s_content::NodeDigest {
    let mut body = Vec::with_capacity(72);
    body.extend_from_slice(&n.to_be_bytes());
    body.extend_from_slice(left);
    body.extend_from_slice(right);
    let digest = s_content::frame(s_content::tag::SEQ_BRANCH, &body);
    store.insert_unchecked(digest, s_content::tag::SEQ_BRANCH, body);
    digest
}

/// 规范重复序列：一层一个分支节点、左右同根，逻辑叶数 2^levels。
fn repeated_sequence(
    store: &mut MemoryNodes,
    leaf: &s_content::NodeDigest,
    levels: u32,
) -> s_content::NodeDigest {
    let mut node = put_seq_leaf(store, leaf);
    for level in 1..=levels {
        let n = 1u64 << level;
        node = put_branch(store, n, &node, &node);
    }
    node
}

fn put_framed_node(
    store: &mut MemoryNodes,
    t: u8,
    declared: u64,
    child: &s_content::NodeDigest,
) -> s_content::NodeDigest {
    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&declared.to_be_bytes());
    body.extend_from_slice(child);
    let digest = s_content::frame(t, &body);
    store.insert_unchecked(digest, t, body);
    digest
}

fn put_string_node(store: &mut MemoryNodes, bytes: &[u8]) -> s_content::NodeDigest {
    let root = s_content::write_bytes(store, bytes).expect("字节序列");
    put_framed_node(store, s_content::tag::STRING, bytes.len() as u64, &root)
}

#[test]
fn repeated_subtree_is_verified_without_logical_expansion() {
    const LEVELS: u32 = 50;
    let mut store = MemoryNodes::new();
    let element = s_content::write_value(&mut store, &Value::string("x")).expect("元素");
    let elems = repeated_sequence(&mut store, &element.digest, LEVELS);
    let total = 1u64 << LEVELS;
    let array = put_framed_node(&mut store, s_content::tag::ARRAY, total, &elems);
    let unique = store.len();
    assert!(unique < 64, "重复图唯一节点数应很小，实为 {unique}");

    let limit = unique * 4 + 8;
    let mut source = CountingNodes {
        inner: store,
        reads: 0,
        limit,
    };
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&array),
        kind: Kind::Array,
        extent: Some(total),
    };
    let verified = s_content::verify(&mut source, &reference).expect("重复子树应可核");
    assert_eq!(verified.nodes, unique, "nodes 只计不同摘要");
    assert!(
        source.reads <= limit,
        "读取次数 {} 应受上限 {limit} 约束（唯一节点 {unique}）",
        source.reads
    );

    // 让上限真的咬一次：4 次读取必然不足 —— 证明上面的「通过」不是上限失效换来的。
    let mut stripped = CountingNodes {
        inner: source.inner,
        reads: 0,
        limit: 4,
    };
    assert_eq!(
        s_content::verify(&mut stripped, &reference)
            .expect_err("读次数上限必须真的生效")
            .code(),
        "source"
    );
}

#[test]
fn render_stops_at_sink_error_without_expanding() {
    const LEVELS: u32 = 50;
    let mut store = MemoryNodes::new();
    let element = s_content::write_value(&mut store, &Value::string("x")).expect("元素");
    let elems = repeated_sequence(&mut store, &element.digest, LEVELS);
    let total = 1u64 << LEVELS;
    let array = put_framed_node(&mut store, s_content::tag::ARRAY, total, &elems);
    let unique = store.len();
    let limit = unique * 8 + 64;
    let mut source = CountingNodes {
        inner: store,
        reads: 0,
        limit,
    };
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&array),
        kind: Kind::Array,
        extent: Some(total),
    };
    let mut sink = TinySink {
        remaining: 8,
        written: Vec::new(),
    };
    let err = s_content::render_canonical(&mut source, &reference, &mut sink)
        .expect_err("sink 写满即失败");
    assert_eq!(err.code(), code::IO, "sink 错误必须原样上抛");
    assert!(sink.written.len() <= 8, "只接纳了少量字节");
    assert!(
        source.reads <= limit,
        "渲染在读次数 {} 内中止（上限 {limit}）",
        source.reads
    );
}

#[test]
fn repeated_byte_sequences_fold_with_fsm() {
    const LEVELS: u32 = 50;
    let declared = 4096u64 << LEVELS;

    // 字符串：重复 2^50 个 4096 字节块 → 声明长度 2^62
    let mut store = MemoryNodes::new();
    let chunk = vec![b'x'; 4096];
    let chunk_digest = s_content::frame(s_content::tag::BYTES_CHUNK, &chunk);
    store.insert_unchecked(chunk_digest, s_content::tag::BYTES_CHUNK, chunk);
    let seq = repeated_sequence(&mut store, &chunk_digest, LEVELS);
    let string_node = put_framed_node(&mut store, s_content::tag::STRING, declared, &seq);
    let unique = store.len();
    let limit = unique * 4 + 8;
    let mut source = CountingNodes {
        inner: store,
        reads: 0,
        limit,
    };
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&string_node),
        kind: Kind::String,
        extent: Some(declared),
    };
    let verified = s_content::verify(&mut source, &reference).expect("重复字符串应可核");
    assert_eq!(verified.nodes, unique);
    assert!(source.reads <= limit, "读取次数受限");

    // 数字 token：重复 2^50 个 4096 位数字块 → 文法状态机在折叠摘要上判接受
    let mut store = MemoryNodes::new();
    let digits = vec![b'1'; 4096];
    let digit_chunk = s_content::frame(s_content::tag::BYTES_CHUNK, &digits);
    store.insert_unchecked(digit_chunk, s_content::tag::BYTES_CHUNK, digits);
    let seq = repeated_sequence(&mut store, &digit_chunk, LEVELS);
    let number_node = put_framed_node(&mut store, s_content::tag::NUMBER, declared, &seq);
    let unique = store.len();
    let mut source = CountingNodes {
        inner: store,
        reads: 0,
        limit: unique * 4 + 8,
    };
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&number_node),
        kind: Kind::Number,
        extent: Some(declared),
    };
    assert!(
        s_content::verify(&mut source, &reference).is_ok(),
        "重复数字块应可核（文法接受）"
    );

    // 数字负例：块以 0 开头，重复后仍是前导零 → 拒绝
    let mut store = MemoryNodes::new();
    let zero_chunk = vec![b'0'; 4096];
    let zero_digest = s_content::frame(s_content::tag::BYTES_CHUNK, &zero_chunk);
    store.insert_unchecked(zero_digest, s_content::tag::BYTES_CHUNK, zero_chunk);
    let seq = repeated_sequence(&mut store, &zero_digest, 3);
    let bad_number = put_framed_node(&mut store, s_content::tag::NUMBER, 4096 * 8, &seq);
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&bad_number),
        kind: Kind::Number,
        extent: Some(4096 * 8),
    };
    assert_eq!(
        s_content::verify(&mut store, &reference)
            .expect_err("前导零数字必须拒绝")
            .code(),
        code::NUMBER_TOKEN
    );
}

#[test]
fn utf8_across_block_boundary_is_checked() {
    // 正例：0xE6 落在第一块末、0xB1 0x89 在第二块（与向量 utf8_cross_chunk 同型）
    let mut bytes = vec![b'x'; 4095];
    bytes.extend_from_slice(&[0xe6, 0xb1, 0x89]);
    let text = String::from_utf8(bytes).expect("合法 UTF-8");
    let mut store = MemoryNodes::new();
    let built = s_content::write_value(&mut store, &Value::String(text)).expect("写入跨块字符串");
    assert!(s_content::verify(&mut store, &built.content_ref()).is_ok());

    // 负例：跨块截断（0xE6 之后不是续字节）
    let mut broken = vec![b'x'; 4095];
    broken.extend_from_slice(&[0xe6, 0x41]);
    let mut store = MemoryNodes::new();
    let node = put_string_node(&mut store, &broken);
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&node),
        kind: Kind::String,
        extent: Some(broken.len() as u64),
    };
    assert_eq!(
        s_content::verify(&mut store, &reference)
            .expect_err("跨块截断必须拒绝")
            .code(),
        code::UTF8
    );
}

#[test]
fn json_context_reuse_is_checked_per_context() {
    // 同一数组摘要：模板上下文合法；进入 json_text 内部必须拒绝。
    let mut store = MemoryNodes::new();
    let array =
        s_content::write_template(&mut store, &Value::Array(vec![Value::BatchSelf])).expect("模板数组");
    assert!(s_content::verify_template(&mut store, &array.content_ref()).is_ok());
    let json_text = s_content::frame(s_content::tag::JSON_TEXT, &array.digest);
    store.insert_unchecked(json_text, s_content::tag::JSON_TEXT, array.digest.to_vec());
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&json_text),
        kind: Kind::JsonText,
        extent: None,
    };
    assert_eq!(
        s_content::verify_template(&mut store, &reference)
            .expect_err("模板权限不得穿透 json_text 内部")
            .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );

    // 反向：同一文本视图摘要先作为公开值核过，进入外层 json_text 内部仍必须拒绝。
    let mut store = MemoryNodes::new();
    let inner = s_content::write_value(
        &mut store,
        &Value::JsonText(Box::new(JsonValue::String("x".to_string()))),
    )
    .expect("文本视图");
    assert!(s_content::verify(&mut store, &inner.content_ref()).is_ok());
    let leaf = put_seq_leaf(&mut store, &inner.digest);
    let array = put_framed_node(&mut store, s_content::tag::ARRAY, 1, &leaf);
    let outer = s_content::frame(s_content::tag::JSON_TEXT, &array);
    store.insert_unchecked(outer, s_content::tag::JSON_TEXT, array.to_vec());
    let reference = ContentRef {
        format: "s-content/1".to_string(),
        root: s_content::root_string(&outer),
        kind: Kind::JsonText,
        extent: None,
    };
    assert_eq!(
        s_content::verify(&mut store, &reference)
            .expect_err("json_text 嵌套必须拒绝")
            .code(),
        code::JSON_TEXT_INTERNAL_KIND
    );
}
