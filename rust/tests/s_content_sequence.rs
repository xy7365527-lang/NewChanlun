#![cfg(feature = "s_session")]
//! #1394 采纳 C §5.9(1)「内容公共 API 缝」下的序列算子靶向验收。
//!
//! 两路对拍 oracle：① 已批准独立向量 `inputs/content-format-vectors.json`（Python 与独立
//! Node 实现共同产出，不是本实现自证）；② 基线已有的 `write_sequence`（同仓既有静态编码）。
//!
//! 资源门是**测试仪表**，不是生产默认值：对 2^50 个同叶的重复压缩子树设读/写硬上限，
//! 并用低阈值负控证明该上限真的会拦。

use newchan_rust::s_content::sequence::{
    sequence_append, sequence_concat, sequence_split, SequenceRef,
};
use newchan_rust::s_content::{
    self, code, frame, root_string, tag, CodecError, MemoryNodes, NodeDigest, NodeRead, NodeSink,
    NodeSource,
};
use serde_json::Value as J;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// 已批准输入的 SHA256（§5.10 记录值；向量是输入，不是本实现自己产出的期望值）。
const VECTORS_SHA256: &str = "78984501cbb465be8251e3d212e34c0cb6fb33a5aab9a4999a3f1db592ce71f0"; // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
/// 预算门被触发时的测试码（仪表码，不是生产码）。
const BUDGET_CODE: &str = "test_budget";

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_to_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "十六进制长度必须为偶");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("十六进制"))
        .collect()
}

fn hex_root(bytes: &[u8]) -> String {
    format!("sc1-{}", hex_of(bytes))
}

fn digest_at(bytes: &[u8], offset: usize) -> NodeDigest {
    let mut d = [0u8; 32];
    d.copy_from_slice(&bytes[offset..offset + 32]);
    d
}

/// 确定性叶摘要：只是不透明的 32 字节内容图节点摘要，与真实业务值无关。
fn leaf_digest(i: u64) -> NodeDigest {
    let mut h = Sha256::new();
    h.update(b"NewChanlun/SContent/sequence-test/leaf\x00");
    h.update(i.to_be_bytes());
    let out: [u8; 32] = h.finalize().into();
    out
}

fn leaves(n: u64) -> Vec<NodeDigest> {
    (0..n).map(leaf_digest).collect()
}

/// 从根出发收集序列节点闭包（不进入 0x12 的叶值：叶摘要是不透明值）。
fn closure(source: &mut dyn NodeSource, roots: &[NodeDigest]) -> BTreeMap<String, (u8, Vec<u8>)> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<NodeDigest> = roots.to_vec();
    while let Some(digest) = stack.pop() {
        let (t, body) = match source.read(&digest).expect("读节点") {
            NodeRead::Found { tag, body } => (tag, body),
            NodeRead::Missing => panic!("闭包里缺节点 {}", root_string(&digest)),
        };
        out.insert(root_string(&digest), (t, body.clone()));
        if t == tag::SEQ_BRANCH {
            stack.push(digest_at(&body, 8));
            stack.push(digest_at(&body, 40));
        }
    }
    out
}

fn copy_nodes(from: &MemoryNodes, to: &mut MemoryNodes) {
    for (digest, t, body) in from.iter() {
        to.insert_unchecked(*digest, t, body.to_vec());
    }
}

/// 空序列节点由规范唯一确定、由算子按需物化；闭包对拍前先物化一次。
fn materialize_empty(store: &mut MemoryNodes, seq: &SequenceRef) {
    if seq.is_empty() {
        let _ = sequence_split(store, seq, 0).expect("物化空序列节点");
    }
}

/// 平衡配对合并：只调用规范 concat，覆盖非平衡形状。
fn pairwise_concat(store: &mut dyn s_content::NodeStore, parts: Vec<SequenceRef>) -> SequenceRef {
    let mut level = parts;
    if level.is_empty() {
        return SequenceRef::empty();
    }
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut it = level.into_iter();
        while let Some(a) = it.next() {
            match it.next() {
                Some(b) => next.push(sequence_concat(&mut *store, &a, &b).expect("concat")),
                None => next.push(a),
            }
        }
        level = next;
    }
    level.pop().expect("非空")
}

// ── 独立向量 oracle ──────────────────────────────────────────────────────────

fn vectors_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../.chanlun/review-results/issue1394-content-v3-20260916/inputs/content-format-vectors.json",
    )
}

fn vector_doc() -> J {
    let path = vectors_path();
    let bytes = std::fs::read(&path).expect("读已批准向量（仓库内相对路径）");
    let actual = hex_of(&Sha256::digest(&bytes));
    assert_eq!(actual, VECTORS_SHA256, "向量文件与 §5.10 记录不一致");
    serde_json::from_slice(&bytes).expect("向量 JSON")
}

fn vector_nodes(doc: &J) -> BTreeMap<String, (u8, Vec<u8>)> {
    doc["nodes"]
        .as_array()
        .expect("nodes 数组")
        .iter()
        .map(|n| {
            let root = n["root"].as_str().expect("root").to_string();
            let t = u8::from_str_radix(n["tag"].as_str().expect("tag"), 16).expect("tag 十六进制");
            let body = hex_to_bytes(n["body_hex"].as_str().expect("body_hex"));
            (root, (t, body))
        })
        .collect()
}

/// 向量里某序列根的叶摘要（按左到右顺序）。
fn vector_seq_leaves(nodes: &BTreeMap<String, (u8, Vec<u8>)>, seq_root: &str) -> Vec<NodeDigest> {
    let mut out = Vec::new();
    let mut stack = vec![seq_root.to_string()];
    while let Some(root) = stack.pop() {
        let (t, body) = nodes
            .get(&root)
            .unwrap_or_else(|| panic!("向量缺节点 {root}"));
        match *t {
            tag::SEQ_EMPTY => assert!(body.is_empty(), "空序列 body 必须为空"),
            tag::SEQ_LEAF => {
                assert_eq!(body.len(), 32, "单叶 body 必须是 32 字节叶摘要");
                out.push(digest_at(body, 0));
            }
            tag::SEQ_BRANCH => {
                assert_eq!(body.len(), 72, "分支 body 必须是 U64(n)+左右子摘要");
                stack.push(hex_root(&body[40..72]));
                stack.push(hex_root(&body[8..40]));
            }
            other => panic!("向量序列位置出现 tag={other:02x}"),
        }
    }
    out
}

/// 向量里某序列根的序列节点闭包。
fn vector_seq_closure(
    nodes: &BTreeMap<String, (u8, Vec<u8>)>,
    seq_root: &str,
) -> BTreeMap<String, (u8, Vec<u8>)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![seq_root.to_string()];
    while let Some(root) = stack.pop() {
        let (t, body) = nodes
            .get(&root)
            .unwrap_or_else(|| panic!("向量缺节点 {root}"));
        out.insert(root, (*t, body.clone()));
        if *t == tag::SEQ_BRANCH {
            stack.push(hex_root(&body[40..72]));
            stack.push(hex_root(&body[8..40]));
        }
    }
    out
}

#[test]
fn sequence_ops_match_approved_independent_vector() {
    let doc = vector_doc();
    let nodes = vector_nodes(&doc);
    let positive = doc["positive"].as_array().expect("positive 数组");

    for name in ["array_1", "array_2", "array_3", "array_5"] {
        let case = positive
            .iter()
            .find(|p| p["name"] == name)
            .unwrap_or_else(|| panic!("向量缺正例 {name}"));
        let array_root = case["root"].as_str().expect("数组根").to_string();
        let (t, body) = nodes.get(&array_root).expect("数组节点");
        assert_eq!(*t, tag::ARRAY, "{name} 应是数组节点");
        let n = u64::from_be_bytes(body[..8].try_into().expect("U64"));
        let seq_root = hex_root(&body[8..40]);
        let expected_leaves = vector_seq_leaves(&nodes, &seq_root);
        assert_eq!(expected_leaves.len() as u64, n, "{name} 叶数");
        let expected_closure = vector_seq_closure(&nodes, &seq_root);

        // ① append 逐个重建：根与全部节点逐字等于独立向量
        let mut built_store = MemoryNodes::new();
        let mut built = SequenceRef::empty();
        for leaf in &expected_leaves {
            built = sequence_append(&mut built_store, &built, leaf).expect("append");
        }
        assert_eq!(built.count(), n, "{name} 计数");
        assert_eq!(built.root_text(), seq_root, "{name} append 根");
        assert_eq!(
            closure(&mut built_store, &[built.root()]),
            expected_closure,
            "{name} append 节点"
        );

        // ② concat 平衡配对合并（含非平衡形状）
        let mut merged_store = MemoryNodes::new();
        let parts: Vec<SequenceRef> = expected_leaves
            .iter()
            .map(|leaf| {
                sequence_append(&mut merged_store, &SequenceRef::empty(), leaf).expect("单叶")
            })
            .collect();
        let merged = pairwise_concat(&mut merged_store, parts);
        assert_eq!(merged.root_text(), seq_root, "{name} concat 根");
        assert_eq!(
            closure(&mut merged_store, &[merged.root()]),
            expected_closure,
            "{name} concat 节点"
        );

        // ③ 每个切点都 split，逐段与 write_sequence 对拍，再拼回原根
        let mut full_store = MemoryNodes::new();
        let mut full = SequenceRef::empty();
        for leaf in &expected_leaves {
            full = sequence_append(&mut full_store, &full, leaf).expect("append");
        }
        for cut in 0..=n {
            let (head, tail) = sequence_split(&mut full_store, &full, cut).expect("split");
            let mut oracle = MemoryNodes::new();
            let head_root =
                s_content::write_sequence(&mut oracle, &expected_leaves[..cut as usize])
                    .expect("oracle head");
            let tail_root =
                s_content::write_sequence(&mut oracle, &expected_leaves[cut as usize..])
                    .expect("oracle tail");
            assert_eq!(head.root(), head_root, "{name} cut={cut} 左段根");
            assert_eq!(tail.root(), tail_root, "{name} cut={cut} 右段根");
            assert_eq!(head.count(), cut, "{name} cut={cut} 左段计数");
            assert_eq!(tail.count(), n - cut, "{name} cut={cut} 右段计数");
            let rejoined = sequence_concat(&mut full_store, &head, &tail).expect("rejoin");
            assert_eq!(rejoined.root_text(), seq_root, "{name} cut={cut} 拼回根");
        }
    }
}

// ── write_sequence 对拍 oracle ───────────────────────────────────────────────

#[test]
fn sequence_ops_match_write_sequence_oracle_across_shapes() {
    for n in [
        0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 33, 64, 65, 100,
    ] {
        let leaf_list = leaves(n);
        let mut oracle_store = MemoryNodes::new();
        let oracle_root = s_content::write_sequence(&mut oracle_store, &leaf_list).expect("oracle");
        let oracle_closure = closure(&mut oracle_store, &[oracle_root]);

        // 逐个 append
        let mut store = MemoryNodes::new();
        let mut seq = SequenceRef::empty();
        for leaf in &leaf_list {
            seq = sequence_append(&mut store, &seq, leaf).expect("append");
        }
        materialize_empty(&mut store, &seq);
        assert_eq!(seq.root(), oracle_root, "n={n} append 根");
        assert_eq!(seq.count(), n, "n={n} append 计数");
        assert_eq!(
            closure(&mut store, &[seq.root()]),
            oracle_closure,
            "n={n} append 节点"
        );

        // 单叶后配对 concat
        let mut concat_store = MemoryNodes::new();
        let parts: Vec<SequenceRef> = leaf_list
            .iter()
            .map(|leaf| {
                sequence_append(&mut concat_store, &SequenceRef::empty(), leaf).expect("单叶")
            })
            .collect();
        let merged = pairwise_concat(&mut concat_store, parts);
        materialize_empty(&mut concat_store, &merged);
        assert_eq!(merged.root(), oracle_root, "n={n} concat 根");
        assert_eq!(
            closure(&mut concat_store, &[merged.root()]),
            oracle_closure,
            "n={n} concat 节点"
        );
    }
}

/// 空结果必须落盘：`concat(empty, empty)` 在**新空 store** 上返回的 0x11 根要能立刻读回。
#[test]
fn sequence_empty_result_is_persisted_and_readable() {
    let empty = SequenceRef::empty();
    let mut fresh = MemoryNodes::new();
    let both = sequence_concat(&mut fresh, &empty, &empty).expect("空+空");
    assert!(both.is_empty());
    assert_eq!(both.root_text(), empty.root_text());
    assert!(fresh.contains(&both.root()), "空结果必须已写入 store");
    match fresh.read(&both.root()).expect("读回空结果") {
        NodeRead::Found { tag: t, body } => {
            assert_eq!(t, tag::SEQ_EMPTY);
            assert!(body.is_empty(), "空序列节点 body 必须为空");
        }
        NodeRead::Missing => panic!("空结果节点缺失"),
    }

    // split 的空侧同样物化
    let mut store = MemoryNodes::new();
    let mut three = SequenceRef::empty();
    for leaf in leaves(3).iter() {
        three = sequence_append(&mut store, &three, leaf).expect("append");
    }
    let (head, tail) = sequence_split(&mut store, &three, 0).expect("cut 0");
    assert!(head.is_empty() && store.contains(&head.root()));
    match store.read(&head.root()).expect("读回空侧") {
        NodeRead::Found { tag: t, body } => {
            assert_eq!(t, tag::SEQ_EMPTY);
            assert!(body.is_empty());
        }
        NodeRead::Missing => panic!("split 空侧节点缺失"),
    }
    assert_eq!(tail.root(), three.root());

    // 空 + X：返回 X 本身，且 X 的节点一定已在 store（入口校验读过）
    let mut oracle = MemoryNodes::new();
    let three_root = s_content::write_sequence(&mut oracle, &leaves(3)).expect("oracle 3");
    let via = sequence_concat(
        &mut oracle,
        &SequenceRef::empty(),
        &SequenceRef::from_parts_unchecked(three_root, 3),
    )
    .expect("空左");
    assert_eq!(via.root(), three_root);
    assert!(oracle.contains(&via.root()));
}

#[test]
fn sequence_concat_empty_and_split_boundaries() {
    let mut store = MemoryNodes::new();
    let empty = SequenceRef::empty();
    let mut three = SequenceRef::empty();
    for (i, leaf) in leaves(3).iter().enumerate() {
        three = sequence_append(&mut store, &three, leaf).expect("append");
        assert_eq!(three.count(), i as u64 + 1);
    }

    // 空 + X = X；X + 空 = X；空 + 空 = 空（不依赖 store 里已有空节点）
    let mut fresh = MemoryNodes::new();
    copy_nodes(&store, &mut fresh);
    assert_eq!(
        sequence_concat(&mut fresh, &empty, &three)
            .expect("空左")
            .root(),
        three.root()
    );
    assert_eq!(
        sequence_concat(&mut fresh, &three, &empty)
            .expect("空右")
            .root(),
        three.root()
    );
    let both = sequence_concat(&mut fresh, &empty, &empty).expect("空+空");
    assert!(both.is_empty());
    assert_eq!(both.root_text(), empty.root_text());

    // 幂 2 边界：两块不相交的 2 叶拼成 n=4（完美形状）；3 + 5 为非平衡 concat
    let mut perfect = MemoryNodes::new();
    let build_block = |store: &mut MemoryNodes, ids: &[u64]| -> SequenceRef {
        let mut s = SequenceRef::empty();
        for i in ids {
            s = sequence_append(store, &s, &leaf_digest(*i)).expect("append");
        }
        s
    };
    let two_a = build_block(&mut perfect, &[0, 1]);
    let two_b = build_block(&mut perfect, &[2, 3]);
    let four = sequence_concat(&mut perfect, &two_a, &two_b).expect("2+2");
    let mut oracle = MemoryNodes::new();
    let four_oracle = s_content::write_sequence(&mut oracle, &leaves(4)).expect("oracle 4");
    assert_eq!(four.root(), four_oracle, "2+2 应为 n=4 规范根");

    let mut odd = MemoryNodes::new();
    let a = build_block(&mut odd, &[0, 1, 2]);
    let b = build_block(&mut odd, &[3, 4, 5, 6, 7]);
    let eight = sequence_concat(&mut odd, &a, &b).expect("3+5");
    let mut oracle2 = MemoryNodes::new();
    let eight_oracle = s_content::write_sequence(&mut oracle2, &leaves(8)).expect("oracle 8");
    assert_eq!(eight.root(), eight_oracle, "3+5 应为 n=8 规范根");

    // 切在 0 / count：空侧也被物化出来，之后可直接继续 append
    let (head, tail) = sequence_split(&mut store, &three, 0).expect("cut 0");
    assert!(head.is_empty());
    assert_eq!(tail.root(), three.root());
    assert!(store.contains(&head.root()), "空侧应已物化");
    let grown = sequence_append(&mut store, &tail, &leaf_digest(99)).expect("空侧后可续 append");
    let mut oracle3 = MemoryNodes::new();
    let grown_oracle =
        s_content::write_sequence(&mut oracle3, &[leaves(3), vec![leaf_digest(99)]].concat())
            .expect("oracle 3+1");
    assert_eq!(grown.root(), grown_oracle);
    let (head2, tail2) = sequence_split(&mut store, &three, 3).expect("cut n");
    assert_eq!(head2.root(), three.root());
    assert!(tail2.is_empty());
}

#[test]
fn sequence_split_then_concat_restores_root() {
    let leaf_list = leaves(70);
    let mut oracle = MemoryNodes::new();
    let oracle_root = s_content::write_sequence(&mut oracle, &leaf_list).expect("oracle");

    let mut store = MemoryNodes::new();
    let mut seq = SequenceRef::empty();
    for leaf in &leaf_list {
        seq = sequence_append(&mut store, &seq, leaf).expect("append");
    }
    assert_eq!(seq.root(), oracle_root);

    // 多次 split + 顺序 concat 复原；每段与 oracle 逐段对拍
    for cuts in [
        vec![0u64, 1, 2, 35, 70],
        vec![0, 34, 35, 36, 70],
        vec![0, 70],
        vec![0, 0, 70, 70],
        vec![0, 3, 3, 67, 70],
    ] {
        let mut work = MemoryNodes::new();
        copy_nodes(&store, &mut work);
        let mut parts = Vec::new();
        let mut rest = seq;
        let mut consumed = 0u64;
        for &cut in cuts.iter().skip(1) {
            let (head, tail) = sequence_split(&mut work, &rest, cut - consumed).expect("split");
            parts.push(head);
            rest = tail;
            consumed = cut;
        }
        // 逐段与 write_sequence 对拍（同一叶序列的规范根）
        let mut start = 0usize;
        for part in &parts {
            let mut part_oracle = MemoryNodes::new();
            let expect = s_content::write_sequence(
                &mut part_oracle,
                &leaf_list[start..start + part.count() as usize],
            )
            .expect("oracle 段");
            assert_eq!(part.root(), expect, "cuts={cuts:?} 段根");
            start += part.count() as usize;
        }
        let rejoined = pairwise_concat(&mut work, parts);
        assert_eq!(rejoined.root(), oracle_root, "cuts={cuts:?} 拼回根");
        // 结果可立即继续 append，且与 oracle 一致
        let grown = sequence_append(&mut work, &rejoined, &leaf_digest(999)).expect("续 append");
        let mut grown_oracle = MemoryNodes::new();
        let expect = s_content::write_sequence(
            &mut grown_oracle,
            &[leaf_list.clone(), vec![leaf_digest(999)]].concat(),
        )
        .expect("oracle n+1");
        assert_eq!(grown.root(), expect, "cuts={cuts:?} 续 append 根");
    }
}

/// 单次算子的写入面：split 只写两段结果可达的节点；concat 会写少量中间规范节点
/// （实测 11 叶 concat 多出 1 个；仪表上界 8·bits²，不是复杂度定理）。
#[test]
fn sequence_ops_write_only_reachable_nodes() {
    let leaf_list = leaves(11);
    let mut seed = MemoryNodes::new();
    // 输入闭包 + 期望根都在同一个 store 里（write_sequence 落规范节点）
    let five_root = s_content::write_sequence(&mut seed, &leaf_list[..5]).expect("oracle 5");
    let six_root = s_content::write_sequence(&mut seed, &leaf_list[5..]).expect("oracle 6");
    let full_root = s_content::write_sequence(&mut seed, &leaf_list).expect("oracle 11");

    // concat：算子新增的节点必须全部落在结果闭包内（输入闭包之外不许留悬空节点）
    let mut store = MemoryNodes::new();
    copy_nodes(&seed, &mut store);
    let before: std::collections::HashSet<String> =
        store.iter().map(|(d, _, _)| root_string(d)).collect();
    let merged = sequence_concat(
        &mut store,
        &SequenceRef::from_parts_unchecked(five_root, 5),
        &SequenceRef::from_parts_unchecked(six_root, 6),
    )
    .expect("concat");
    assert_eq!(merged.root(), full_root);
    let mut allowed = closure(&mut store, &[merged.root()]);
    allowed.insert(
        root_string(&SequenceRef::empty().root()),
        (tag::SEQ_EMPTY, Vec::new()),
    );
    let mut extra = Vec::new();
    for (digest, t, body) in store.iter() {
        let key = root_string(digest);
        if before.contains(&key) || allowed.contains_key(&key) {
            continue;
        }
        extra.push((key, t, body.to_vec()));
    }
    // 中间规范节点：自身必须按 k 规则自洽（本节点 + 直接子节点声明计数）
    for (key, t, body) in &extra {
        assert_eq!(*t, tag::SEQ_BRANCH, "中间节点 {key} 应是分支节点");
        assert_eq!(body.len(), 72, "中间节点 {key} 的 body 长度");
        let n = u64::from_be_bytes(body[..8].try_into().expect("U64"));
        assert!(n >= 2, "中间节点 {key} 声明 n={n}");
        let k = 1u64 << (63 - (n - 1).leading_zeros());
        for (child, want) in [(digest_at(body, 8), k), (digest_at(body, 40), n - k)] {
            let (ct, cbody) = store.get(&child).expect("中间节点的子节点");
            if want == 1 {
                assert_eq!(ct, tag::SEQ_LEAF, "叶位置 {key}");
                assert_eq!(cbody.len(), 32);
            } else {
                assert_eq!(ct, tag::SEQ_BRANCH, "分支位置 {key}");
                assert_eq!(
                    u64::from_be_bytes(cbody[..8].try_into().expect("U64")),
                    want,
                    "子节点声明计数 n={want}"
                );
            }
        }
    }
    let bits = 64 - 11u64.leading_zeros() as u64;
    assert!(
        extra.len() as u64 <= 8 * (bits + 2) * (bits + 2),
        "中间节点 {} 个，超过仪表上界 {}",
        extra.len(),
        8 * (bits + 2) * (bits + 2)
    );

    // split：算子新增的节点必须全部落在两段结果闭包的并集内（允许空节点）
    let mut store2 = MemoryNodes::new();
    copy_nodes(&seed, &mut store2);
    let before2: std::collections::HashSet<String> =
        store2.iter().map(|(d, _, _)| root_string(d)).collect();
    let (head, tail) = sequence_split(
        &mut store2,
        &SequenceRef::from_parts_unchecked(full_root, 11),
        4,
    )
    .expect("split");
    let mut covered = closure(&mut store2, &[head.root()]);
    covered.extend(closure(&mut store2, &[tail.root()]));
    covered.insert(
        root_string(&SequenceRef::empty().root()),
        (tag::SEQ_EMPTY, Vec::new()),
    );
    for (digest, t, body) in store2.iter() {
        let key = root_string(digest);
        if before2.contains(&key) {
            continue;
        }
        let entry = covered
            .get(&key)
            .unwrap_or_else(|| panic!("split 写出悬空节点 {key}（tag=0x{t:02x}）"));
        assert_eq!(*entry, (t, body.to_vec()));
    }
}

/// split 切在规范边界（等于某个 k）时不写新节点。
#[test]
fn sequence_split_at_canonical_boundary_writes_nothing() {
    let mut store = MemoryNodes::new();
    let mut seq = SequenceRef::empty();
    for leaf in leaves(17).iter() {
        seq = sequence_append(&mut store, &seq, leaf).expect("append");
    }
    let before = store.len();
    let (head, tail) = sequence_split(&mut store, &seq, 16).expect("cut 16");
    assert_eq!(store.len(), before, "切在 k=16 不应写新节点");
    assert_eq!(head.count(), 16);
    assert_eq!(tail.count(), 1);
    let before2 = store.len();
    let (head2, tail2) = sequence_split(&mut store, &seq, 12).expect("cut 12");
    assert_eq!(head2.count(), 12);
    assert_eq!(tail2.count(), 5);
    assert!(store.len() > before2, "切在 12 需要新前缀节点");
}

// ── 拒绝面：错误节点、非法长度、越界 ────────────────────────────────────────

fn expect_shape(err: CodecError, what: &str) {
    assert_eq!(err.code(), code::SEQUENCE_SHAPE, "{what} 的拒绝码");
}

#[test]
fn sequence_rejects_wrong_nodes_and_lengths() {
    let leaf_list = leaves(3);
    let mut store = MemoryNodes::new();
    let root = s_content::write_sequence(&mut store, &leaf_list).expect("oracle");
    let leaf_list5 = leaves(5);
    let mut store5 = MemoryNodes::new();
    let root5 = s_content::write_sequence(&mut store5, &leaf_list5).expect("oracle 5");

    // 叶数与声明不符
    expect_shape(
        sequence_concat(
            &mut store,
            &SequenceRef::from_parts_unchecked(root, 4),
            &SequenceRef::empty(),
        )
        .expect_err("n=3 的根不能当 n=4"),
        "声明叶数与分支 n 不符",
    );
    // 单叶节点当多叶
    let leaf_node = frame(tag::SEQ_LEAF, &leaf_list[0]);
    expect_shape(
        sequence_concat(
            &mut store,
            &SequenceRef::from_parts_unchecked(leaf_node, 2),
            &SequenceRef::empty(),
        )
        .expect_err("0x12 不能当 n=2"),
        "单叶节点当分支",
    );
    // 分支节点当单叶
    expect_shape(
        sequence_concat(
            &mut store,
            &SequenceRef::from_parts_unchecked(root, 1),
            &SequenceRef::empty(),
        )
        .expect_err("0x13 不能当 n=1"),
        "分支节点当单叶",
    );
    // 空序列根写错
    expect_shape(
        sequence_concat(
            &mut store,
            &SequenceRef::from_parts_unchecked(leaf_node, 0),
            &SequenceRef::empty(),
        )
        .expect_err("n=0 必须是空序列节点"),
        "空序列根不符",
    );
    // 非序列节点冒充序列根
    let mut value_store = MemoryNodes::new();
    let value = s_content::write_value(
        &mut value_store,
        &s_content::Value::string("not-a-sequence"),
    )
    .expect("写字符串值");
    expect_shape(
        sequence_concat(
            &mut value_store,
            &SequenceRef::from_parts_unchecked(value.digest, 1),
            &SequenceRef::empty(),
        )
        .expect_err("值节点不能当序列"),
        "值节点冒充序列",
    );
    // 缺节点
    let missing = sequence_concat(
        &mut store,
        &SequenceRef::from_parts_unchecked([7u8; 32], 1),
        &SequenceRef::empty(),
    )
    .expect_err("缺节点必须拒绝");
    assert_eq!(missing.code(), code::NODE_MISSING);
    // 同摘要换字节
    let mut tampered = MemoryNodes::new();
    copy_nodes(&store, &mut tampered);
    let body = {
        let (_, body) = store
            .get(&root)
            .map(|(t, b)| (t, b.to_vec()))
            .expect("分支节点");
        let mut bad = body;
        bad[7] ^= 0x01;
        bad
    };
    tampered.insert_unchecked(root, tag::SEQ_BRANCH, body);
    let digest_err = sequence_concat(
        &mut tampered,
        &SequenceRef::from_parts_unchecked(root, 3),
        &SequenceRef::empty(),
    )
    .expect_err("同摘要换字节必须拒绝");
    assert_eq!(digest_err.code(), code::NODE_DIGEST);

    // 拆分越界
    expect_shape(
        sequence_split(&mut store5, &SequenceRef::from_parts_unchecked(root5, 5), 6)
            .expect_err("越界拆分"),
        "拆分越界",
    );
    // 期望叶数本身超出 2^63-1
    let over = sequence_append(
        &mut store5,
        &SequenceRef::from_parts_unchecked(root5, s_content::MAX_FRAME_LEN + 1),
        &leaf_list[0],
    )
    .expect_err("叶数超上限");
    assert_eq!(over.code(), code::LENGTH_OVERFLOW);
    // 根文本解析：前缀/大小写严格
    let text = root_string(&root);
    let parsed = SequenceRef::from_root_text_unchecked(&text, 3).expect("解析根文本");
    assert_eq!(parsed.root(), root);
    assert_eq!(
        SequenceRef::from_root_text_unchecked(&text.to_uppercase(), 3)
            .expect_err("大写拒绝")
            .code(),
        code::ROOT
    );
    assert_eq!(
        SequenceRef::from_root_text_unchecked(&text, s_content::MAX_FRAME_LEN + 1)
            .expect_err("叶数越界")
            .code(),
        code::LENGTH_OVERFLOW
    );
}

#[test]
fn sequence_concat_rejects_length_overflow_on_repeated_subtree() {
    // 2^62 与 2^62 合并 = 2^63 > 2^63-1：必须当场拒绝，不做任何展开
    let (nodes, root) = perfect_chain(leaf_digest(0), 62);
    let mut store = BudgetStore::new(nodes, 4096, 4096);
    let half = SequenceRef::from_parts_unchecked(root, 1u64 << 62);
    let err = sequence_concat(&mut store, &half, &half).expect_err("叶数溢出必须拒绝");
    assert_eq!(err.code(), code::LENGTH_OVERFLOW);
}

// ── 资源门：重复压缩子树上的硬上限与低阈值负控 ──────────────────────────────

/// 计数 + 硬上限的 store 包装（测试仪表）。
struct BudgetStore {
    nodes: HashMap<NodeDigest, (u8, Vec<u8>)>,
    reads: u64,
    writes: u64,
    read_cap: u64,
    write_cap: u64,
}

impl BudgetStore {
    fn new(nodes: HashMap<NodeDigest, (u8, Vec<u8>)>, read_cap: u64, write_cap: u64) -> Self {
        Self {
            nodes,
            reads: 0,
            writes: 0,
            read_cap,
            write_cap,
        }
    }
}

impl NodeSource for BudgetStore {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.reads += 1;
        if self.reads > self.read_cap {
            return Err(CodecError::new(
                BUDGET_CODE,
                format!("读预算 {} 已耗尽（第 {} 次读）", self.read_cap, self.reads),
            ));
        }
        Ok(match self.nodes.get(digest) {
            Some((t, b)) => NodeRead::Found {
                tag: *t,
                body: b.clone(),
            },
            None => NodeRead::Missing,
        })
    }
}

impl NodeSink for BudgetStore {
    fn put(&mut self, digest: NodeDigest, t: u8, body: &[u8]) -> Result<(), CodecError> {
        self.writes += 1;
        if self.writes > self.write_cap {
            return Err(CodecError::new(
                BUDGET_CODE,
                format!(
                    "写预算 {} 已耗尽（第 {} 次写）",
                    self.write_cap, self.writes
                ),
            ));
        }
        let actual = frame(t, body);
        assert_eq!(actual, digest, "写入摘要必须与 (tag, body) 重算一致");
        self.nodes.insert(digest, (t, body.to_vec()));
        Ok(())
    }
}

/// 2^depth 个同叶的完美子树：只物化 depth+1 个真实节点（重复子节点同摘要）。
fn perfect_chain(leaf: NodeDigest, depth: u32) -> (HashMap<NodeDigest, (u8, Vec<u8>)>, NodeDigest) {
    let mut nodes = HashMap::new();
    let mut count = 1u64;
    let mut cur = frame(tag::SEQ_LEAF, &leaf);
    nodes.insert(cur, (tag::SEQ_LEAF, leaf.to_vec()));
    for _ in 0..depth {
        count *= 2;
        let mut body = [0u8; 72];
        body[..8].copy_from_slice(&count.to_be_bytes());
        body[8..40].copy_from_slice(&cur);
        body[40..72].copy_from_slice(&cur);
        let digest = frame(tag::SEQ_BRANCH, &body);
        nodes.insert(digest, (tag::SEQ_BRANCH, body.to_vec()));
        cur = digest;
    }
    (nodes, cur)
}

/// 与逻辑叶数无关的**仪表**上界族：8·bits 与 8·bits²（bits = 叶数的二进制位数）。
/// 这是测试仪表，不是本实现的最坏复杂度定理（最坏界未证明，照实按测得值记）。
fn bits_of(n: u64) -> u64 {
    64 - n.leading_zeros() as u64
}

fn linear_bound(n: u64) -> u64 {
    8 * (bits_of(n) + 2)
}

fn quadratic_bound(n: u64) -> u64 {
    linear_bound(n) * (bits_of(n) + 2)
}

#[test]
fn sequence_ops_stay_within_hard_bounds_on_repeated_subtree() {
    const DEPTH: u32 = 50;
    // 仪表硬上限：读 2^12 / 写 2^10，与 2^51 个逻辑叶完全不可比
    // （实测整段 four-op 序列：读 318 / 写 106；朴素全叶展开在第一轮 4096 读上限即被拒）
    const READ_CAP: u64 = 1 << 12;
    const WRITE_CAP: u64 = 1 << 10;
    let (nodes, root) = perfect_chain(leaf_digest(0), DEPTH);
    let logical = 1u64 << DEPTH;
    let mut store = BudgetStore::new(nodes, READ_CAP, WRITE_CAP);

    let big = SequenceRef::from_parts_unchecked(root, logical);
    // 追加一个叶：只在右边界动
    let mark = (store.reads, store.writes);
    let appended = sequence_append(&mut store, &big, &leaf_digest(1)).expect("append");
    assert_eq!(appended.count(), logical + 1);
    let used = store.reads - mark.0 + store.writes - mark.1;
    assert!(
        used <= linear_bound(logical),
        "append 用了 {used} 次节点读写，超过仪表上界 {}",
        linear_bound(logical)
    );
    // 结果可立即继续 append
    let mark = (store.reads, store.writes);
    let appended2 = sequence_append(&mut store, &appended, &leaf_digest(2)).expect("append 2");
    assert_eq!(appended2.count(), logical + 2);
    let used = store.reads - mark.0 + store.writes - mark.1;
    assert!(
        used <= linear_bound(logical),
        "续 append 用了 {used} 次节点读写，超过仪表上界 {}",
        linear_bound(logical)
    );
    // 两个重复子树 concat：2^50 + 2^50 = 2^51
    let mark = (store.reads, store.writes);
    let doubled = sequence_concat(&mut store, &big, &big).expect("concat");
    assert_eq!(doubled.count(), 1u64 << 51);
    let used = store.reads - mark.0 + store.writes - mark.1;
    assert!(
        used <= quadratic_bound(doubled.count()),
        "concat 用了 {used} 次节点读写，超过仪表上界 {}",
        quadratic_bound(doubled.count())
    );
    // 在重复子树内部切一刀（不是规范边界，必须走 concat 拼回）
    let mark = (store.reads, store.writes);
    let (head, tail) = sequence_split(&mut store, &doubled, logical - 1).expect("split");
    assert_eq!(head.count() + tail.count(), 1u64 << 51);
    let used = store.reads - mark.0 + store.writes - mark.1;
    assert!(
        used <= quadratic_bound(doubled.count()),
        "split 用了 {used} 次节点读写，超过仪表上界 {}",
        quadratic_bound(doubled.count())
    );
    assert!(
        store.reads <= READ_CAP && store.writes <= WRITE_CAP,
        "读 {} / 写 {} 超过硬上限（逻辑叶 {logical}）",
        store.reads,
        store.writes
    );
    // 与逻辑叶数不可比：2^50 叶的展开需要 ~2^51 次节点读写
    assert!(
        store.reads + store.writes < (1u64 << 20),
        "读写 {} 与逻辑叶规模 2^{DEPTH} 不成比例",
        store.reads + store.writes
    );

    // 结构对拍（不展开）：切在 2^50 处，左段必须还是那棵重复子树，右段是 2 叶完美块
    let (left, right) = sequence_split(&mut store, &appended2, logical).expect("cut at 2^50");
    assert_eq!(left.root(), root, "左段应是原重复子树根");
    let mut oracle = MemoryNodes::new();
    let two_root = s_content::write_sequence(&mut oracle, &[leaf_digest(1), leaf_digest(2)])
        .expect("oracle 2");
    assert_eq!(right.root(), two_root, "右段应是 [leaf1, leaf2] 的规范根");
}

#[test]
fn sequence_hard_bound_is_enforced_negative_control() {
    let (nodes, root) = perfect_chain(leaf_digest(0), 50);
    let big = SequenceRef::from_parts_unchecked(root, 1u64 << 50);

    // 同一操作，高上限通过、低上限必须被拒绝：证明上限是仪表而不是装饰
    let mut generous = BudgetStore::new(nodes.clone(), 4096, 4096);
    sequence_append(&mut generous, &big, &leaf_digest(3)).expect("高上限应通过");

    let mut tight = BudgetStore::new(nodes, 2, 64);
    let err = sequence_append(&mut tight, &big, &leaf_digest(3)).expect_err("低上限必须拒绝");
    assert_eq!(err.code(), BUDGET_CODE);
}

#[test]
fn sequence_ops_read_and_write_scale_with_height() {
    for depth in [10u32, 20, 30, 50] {
        let (nodes, root) = perfect_chain(leaf_digest(0), depth);
        let mut store = BudgetStore::new(nodes, 4096, 4096);
        let seq = SequenceRef::from_parts_unchecked(root, 1u64 << depth);
        let grown = sequence_append(&mut store, &seq, &leaf_digest(1)).expect("append");
        assert_eq!(grown.count(), (1u64 << depth) + 1);
        let bound = 8 * (depth as u64 + 2);
        assert!(
            store.reads <= bound && store.writes <= bound,
            "depth={depth} 读 {} / 写 {} 超过与树高同阶的仪表上界 {bound}",
            store.reads,
            store.writes
        );
    }
}

// #1394 调用内 memo 实际分配：与原算子逐次读写对拍，释放边界是单次调用。
#[cfg(feature = "s_allocation_json_testonly")]
mod local_memo_allocation {
    use super::*;
    use newchan_rust::s_content::capture_allocation::{OwnedBudget, LIMIT};
    use newchan_rust::s_content::sequence::{
        sequence_append_with_allocation, sequence_concat_with_allocation,
        sequence_split_with_allocation,
    };
    use newchan_rust::s_content::CodecVec;

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Read(NodeDigest),
        Write(NodeDigest, u8, Vec<u8>),
    }
    struct TraceStore {
        inner: BudgetStore,
        events: Vec<Event>,
        live_before_event: Vec<Option<usize>>,
        budget: Option<OwnedBudget>,
        live_at_error: Option<usize>,
    }
    impl TraceStore {
        fn new(nodes: HashMap<NodeDigest, (u8, Vec<u8>)>, budget: Option<&OwnedBudget>) -> Self {
            Self {
                inner: BudgetStore::new(nodes, 4096, 4096),
                events: Vec::new(),
                live_before_event: Vec::new(),
                budget: budget.cloned(),
                live_at_error: None,
            }
        }
        fn record(&mut self, event: Event) {
            self.live_before_event.push(
                self.budget
                    .as_ref()
                    .map(|b| b.snapshot().live_requested_layout_bytes),
            );
            self.events.push(event);
        }
        fn error(&mut self) {
            self.live_at_error = self
                .budget
                .as_ref()
                .map(|b| b.snapshot().live_requested_layout_bytes);
        }
    }
    impl NodeSource for TraceStore {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.record(Event::Read(*digest));
            let result = self.inner.read(digest);
            if result.is_err() {
                self.error();
            }
            result
        }
    }
    impl NodeSink for TraceStore {
        fn put(&mut self, digest: NodeDigest, t: u8, body: &[u8]) -> Result<(), CodecError> {
            self.record(Event::Write(digest, t, body.to_vec()));
            let result = self.inner.put(digest, t, body);
            if result.is_err() {
                self.error();
            }
            result
        }
    }
    fn baseline(budget: &OwnedBudget) -> CodecVec<u8> {
        let mut owner = CodecVec::new_with_allocation(Some(budget));
        owner.try_reserve_exact(17).unwrap();
        owner
    }
    fn nodes_from_memory(store: &MemoryNodes) -> HashMap<NodeDigest, (u8, Vec<u8>)> {
        store
            .iter()
            .map(|(root, t, body)| (*root, (t, body.to_vec())))
            .collect()
    }

    #[test]
    fn owned_sequence_independent_vectors_cut_join_and_append_match_plain_bytes_and_trace() {
        let doc = vector_doc();
        let vectors = vector_nodes(&doc);
        for name in ["array_1", "array_2", "array_3", "array_5"] {
            let case = doc["positive"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == name)
                .unwrap();
            let (_, body) = vectors.get(case["root"].as_str().unwrap()).unwrap();
            let root_text = hex_root(&body[8..40]);
            let leaves = vector_seq_leaves(&vectors, &root_text);
            let budget = OwnedBudget::new(1 << 20);
            let _owner = baseline(&budget);
            let base = budget.snapshot().live_requested_layout_bytes;
            let mut plain = TraceStore::new(HashMap::new(), None);
            let mut owned = TraceStore::new(HashMap::new(), Some(&budget));
            let mut p = SequenceRef::empty();
            let mut o = p;
            for leaf in &leaves {
                p = sequence_append(&mut plain, &p, leaf).unwrap();
                o = sequence_append_with_allocation(&mut owned, &o, leaf, Some(&budget)).unwrap();
                assert_eq!(p, o);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            }
            assert_eq!(o.root_text(), root_text);
            assert_eq!(plain.events, owned.events);
            assert_eq!(plain.inner.nodes, owned.inner.nodes);
            let mut closure_store = MemoryNodes::new();
            for (digest, (t, body)) in &owned.inner.nodes {
                closure_store.insert_unchecked(*digest, *t, body.clone());
            }
            assert_eq!(
                closure(&mut closure_store, &[o.root()]),
                vector_seq_closure(&vectors, &root_text)
            );
            for cut in 0..=o.count() {
                let ps = sequence_split(&mut plain, &p, cut).unwrap();
                let os =
                    sequence_split_with_allocation(&mut owned, &o, cut, Some(&budget)).unwrap();
                assert_eq!(ps, os);
                p = sequence_concat(&mut plain, &ps.0, &ps.1).unwrap();
                o = sequence_concat_with_allocation(&mut owned, &os.0, &os.1, Some(&budget))
                    .unwrap();
                assert_eq!(p, o);
                assert_eq!(o.root_text(), root_text);
                assert_eq!(plain.events, owned.events);
                assert_eq!(plain.inner.nodes, owned.inner.nodes);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            }
        }
    }

    #[test]
    fn owned_sequence_compressed_noncanonical_cut_grow_and_source_errors_release_local_memo() {
        let (nodes, root) = perfect_chain(leaf_digest(0), 16);
        let seq = SequenceRef::from_parts_unchecked(root, 1 << 16);
        // 交替二进制切点让split回程中的concat再次下钻，表已分配后仍有真实read。
        // n-1切点的read全部发生在首次memo插入前，不能用来验错误时释放活表。
        let cut = seq.count() / 3;
        let budget = OwnedBudget::new(1 << 20);
        let _owner = baseline(&budget);
        let base = budget.snapshot().live_requested_layout_bytes;
        let mut plain = TraceStore::new(nodes.clone(), None);
        let mut owned = TraceStore::new(nodes.clone(), Some(&budget));
        let expected = sequence_split(&mut plain, &seq, cut).unwrap();
        let result = sequence_split_with_allocation(&mut owned, &seq, cut, Some(&budget)).unwrap();
        assert_eq!(result, expected);
        assert_eq!(plain.events, owned.events);
        assert_eq!(plain.inner.nodes, owned.inner.nodes);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        let peak = budget.snapshot().peak_requested_layout_bytes;
        assert!(peak > base);
        for fail_read in [true, false] {
            // 只根据这次真实成功运算中当时活跃的账本读数选择故障点。
            let target = owned
                .events
                .iter()
                .zip(&owned.live_before_event)
                .rposition(|(event, live)| {
                    matches!(event, Event::Read(_)) == fail_read
                        && live.is_some_and(|bytes| bytes > base)
                })
                .expect("该构型必须有memo已分配后的实际读和写");
            let ordinal = owned.events[..=target]
                .iter()
                .filter(|event| matches!(event, Event::Read(_)) == fail_read)
                .count() as u64;
            let live_at_target = owned.live_before_event[target].unwrap();
            let mut p = TraceStore::new(nodes.clone(), None);
            let mut o = TraceStore::new(nodes.clone(), Some(&budget));
            if fail_read {
                p.inner.read_cap = ordinal - 1;
                o.inner.read_cap = ordinal - 1;
            } else {
                p.inner.write_cap = ordinal - 1;
                o.inner.write_cap = ordinal - 1;
            }
            let pe = sequence_split(&mut p, &seq, cut).unwrap_err();
            let oe = sequence_split_with_allocation(&mut o, &seq, cut, Some(&budget)).unwrap_err();
            assert_eq!(pe.to_string(), oe.to_string());
            assert_eq!(pe.code(), BUDGET_CODE);
            assert_eq!(p.events, o.events);
            assert_eq!(o.events, owned.events[..=target]);
            assert_eq!(p.inner.nodes, o.inner.nodes);
            assert_eq!(o.live_at_error, Some(live_at_target));
            assert!(
                o.live_at_error.unwrap() > base,
                "错误时已有真实memo表，返回后才释放"
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        }
        // 同一个公开测试额度，实际占据剩余空间，仅留下已测峰值少1字节。
        let mut blocker = CodecVec::<u8>::new_with_allocation(Some(&budget));
        blocker
            .try_reserve_exact(budget.snapshot().limit_bytes - peak + 1)
            .unwrap();
        let blocked_base = budget.snapshot().live_requested_layout_bytes;
        let mut rejected = TraceStore::new(nodes, Some(&budget));
        let error =
            sequence_split_with_allocation(&mut rejected, &seq, cut, Some(&budget)).unwrap_err();
        assert_eq!(error.code(), LIMIT);
        let probe = budget.allocation_request_probe();
        assert!(probe.refused_kind.contains("SequenceRef"));
        assert!(
            probe.live_before_refusal > blocked_base,
            "grow拒绝时旧表仍活"
        );
        assert!(probe.refused_bytes > 0);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, blocked_base);
        assert!(!rejected.events.is_empty());
        // 节点写入不在memo返回前回滚；事务rollback/discard由prepared/Runtime现有测试验收。
        drop(blocker);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[test]
    fn owned_sequence_bad_nodes_preserve_first_error_and_read_order() {
        let mut memory = MemoryNodes::new();
        let root = s_content::write_sequence(&mut memory, &leaves(5)).unwrap();
        let nodes = nodes_from_memory(&memory);
        let budget = OwnedBudget::new(1 << 20);
        for (root_input, count, corrupt) in [
            (root, 4, false),
            ([7; 32], 1, false),
            (root, 5, true),
            (frame(tag::SEQ_LEAF, &leaf_digest(0)), 0, false),
        ] {
            let mut input = nodes.clone();
            if corrupt {
                input.get_mut(&root).unwrap().1[7] ^= 1;
            }
            let mut plain = TraceStore::new(input.clone(), None);
            let mut owned = TraceStore::new(input, Some(&budget));
            let seq = SequenceRef::from_parts_unchecked(root_input, count);
            let p = sequence_concat(&mut plain, &seq, &SequenceRef::empty()).unwrap_err();
            let o = sequence_concat_with_allocation(
                &mut owned,
                &seq,
                &SequenceRef::empty(),
                Some(&budget),
            )
            .unwrap_err();
            assert_eq!(p.to_string(), o.to_string());
            assert_eq!(plain.events, owned.events);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
}

#[cfg(not(feature = "s_allocation_json_testonly"))]
#[test]
fn sequence_allocation_entry_refuses_unsupported_owned_build_without_plain_fallback() {
    let budget = s_content::capture_allocation::OwnedBudget::new(1 << 20);
    let mut store = MemoryNodes::new();
    let error = s_content::sequence_append_with_allocation(
        &mut store,
        &SequenceRef::empty(),
        &leaf_digest(0),
        Some(&budget),
    )
    .unwrap_err();
    assert_eq!(error.code(), s_content::capture_allocation::INCOMPLETE);
    assert_eq!(budget.snapshot().allocation_attempts, 0);
    assert_eq!(store.len(), 0);
}
