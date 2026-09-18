//! #1394 C §5.6.1：完整实际 Delta 的三项原子限额。
//! 调用者先绑定已发布历史；本层不凭裸根授权，也不以片段proof代替完整闭包。
use super::q_resources::{Limit, QResources};
use newchan_rust::s_content::{
    self as sc,
    job::Job,
    path::{lookup_path, PathStep},
    CodecError, ContentRef, Kind, NodeDigest, NodeRead, NodeSource,
};
use std::collections::BTreeSet;

const RECORDS: [&str; 7] = [
    "upserts",
    "withdrawals",
    "replaces",
    "witnesses",
    "relations",
    "observations",
    "raw_history_added",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Usage {
    pub nodes: u64,
    pub node_bytes: u64,
    pub records: u64,
}

pub(super) fn measure(
    source: &mut dyn NodeSource,
    delta: &ContentRef,
    resources: &QResources,
    job: &mut Job,
) -> Result<Usage, String> {
    if delta.kind != Kind::Object || delta.extent != Some(17) {
        return Err("QDeltaInvalid：完整Delta须为D十字段与七变化集合".into());
    }
    let mut source = Meter {
        source,
        job,
        seen: BTreeSet::new(),
        nodes: 0,
        bytes: 0,
        max_nodes: resources.limit(Limit::DeltaNodes),
        max_bytes: resources.limit(Limit::DeltaNodeBytes),
    };
    // 先核七个真实数组header的合计，超记录限额无需展开大数组。
    let mut arrays = Vec::new();
    let mut records = 0u128;
    for name in RECORDS {
        let array =
            lookup_path(&mut source, delta, &[PathStep::Field(name.into())]).map_err(err)?;
        if array.kind != Kind::Array {
            return Err(format!("QDeltaInvalid：{name}不是数组"));
        }
        let n = array
            .extent
            .ok_or_else(|| "QDeltaInvalid：数组缺长度".to_string())?;
        records += u128::from(n);
        if records > u128::from(resources.limit(Limit::DeltaRecords)) {
            return Err(limit("max_atomic_delta_records").to_string());
        }
        arrays.push((array, n));
    }
    // 根对象、D十字段、数组/值/块/序列/字段节点均计入，包括未出现在任何片段proof中的节点。
    sc::verify(&mut source, delta).map_err(err)?;
    // 通用codec校验不定义记录访问；这里逐条访问七集合，失败/重复访问同样记账。
    // 不渲染正文，不把嵌套字段、catalog项或唯一节点数量当成records。
    for (array, n) in arrays {
        for i in 0..n {
            source.job.charge_records(1).map_err(err)?;
            lookup_path(&mut source, &array, &[PathStep::Index(i)]).map_err(err)?;
        }
    }
    source.job.pre_io_check().map_err(err)?;
    Ok(Usage {
        nodes: source.nodes,
        node_bytes: source.bytes,
        records: records as u64,
    })
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QDeltaInvalid：{e}")
}
fn limit(name: &str) -> CodecError {
    CodecError::new(
        "atomic_delta_limit",
        format!("Incomplete：完整Delta超过{name}，本请求不得推进游标"),
    )
}
struct Meter<'a> {
    source: &'a mut dyn NodeSource,
    job: &'a mut Job,
    seen: BTreeSet<NodeDigest>,
    nodes: u64,
    bytes: u64,
    max_nodes: u64,
    max_bytes: u64,
}
impl NodeSource for Meter<'_> {
    fn max_render_bytes(&self) -> Option<u64> {
        self.source.max_render_bytes()
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.job.begin_first_read()?;
        let read = self.source.read(digest)?;
        self.job.check_deadline()?;
        if let NodeRead::Found { tag, body } = &read {
            // 全job先核实际字节/digest，跨Delta复用仍受job期限与身份校验约束。
            self.job.accept_node(digest, *tag, body)?;
            if !self.seen.contains(digest) {
                let bytes = u128::from(self.bytes)
                    + (sc::DOMAIN_SEPARATOR.len() + 1 + 8) as u128
                    + body.len() as u128;
                if bytes > u128::from(self.max_bytes) {
                    return Err(limit("max_atomic_delta_node_bytes"));
                }
                if self.nodes >= self.max_nodes {
                    return Err(limit("max_atomic_delta_nodes"));
                }
                self.bytes = bytes as u64;
                self.nodes += 1;
                self.seen.insert(*digest);
            }
        }
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::{
        job::{Existing, JobLimits},
        MemoryNodes, Value as V,
    };
    use serde_json::{json, Value};
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"1000","max_total_requests":"20","max_elapsed_ms":"10000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
    }
    fn fixture() -> (MemoryNodes, ContentRef, Usage) {
        let mut nodes = MemoryNodes::new();
        let mut fields = [
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
        ]
        .map(|k| (k.into(), V::String(format!("D-header-{k}"))))
        .to_vec();
        for name in RECORDS {
            fields.push((
                name.into(),
                V::Array(vec![V::String("shared-record".into()); 3]),
            ));
        }
        let delta = sc::write_value(&mut nodes, &V::Object(fields))
            .unwrap()
            .content_ref();
        // 本fixture写入的所有节点都可达；随后加入孤儿，检验计量不扫整库。
        let expected = Usage {
            nodes: nodes.len() as u64,
            node_bytes: nodes
                .iter()
                .map(|(_, _, b)| (b"NewChanlun/SContent/1\0".len() + 9 + b.len()) as u64)
                .sum(),
            records: 21,
        };
        sc::write_value(&mut nodes, &V::String("unreachable-orphan".into())).unwrap();
        (nodes, delta, expected)
    }
    fn resources_with(key: &str, n: u64) -> QResources {
        let mut input = super::super::q_resources::tests::input();
        input["limits"][key] = json!(n.to_string());
        QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap()
    }
    #[test]
    fn full_closure_includes_header_and_shared_nodes_once_with_independent_delta_caps() {
        let (mut nodes, delta, expected) = fixture();
        let config = super::super::q_resources::tests::resources();
        let mut job = job();
        assert_eq!(
            measure(&mut nodes, &delta, &config, &mut job).unwrap(),
            expected
        );
        assert_eq!(job.snapshot().nodes, expected.nodes);
        assert_eq!(job.snapshot().node_bytes, expected.node_bytes);
        assert_eq!(job.snapshot().records, 21);
        assert_eq!(
            measure(&mut nodes, &delta, &config, &mut job).unwrap(),
            expected
        );
        assert_eq!(job.snapshot().nodes, expected.nodes);
        assert_eq!(job.snapshot().records, 42);
        for (name, exact) in [
            ("max_atomic_delta_node_bytes", expected.node_bytes),
            ("max_atomic_delta_nodes", expected.nodes),
            ("max_atomic_delta_records", 21),
        ] {
            assert!(measure(&mut nodes, &delta, &resources_with(name, exact), &mut job).is_ok());
            let failure = measure(
                &mut nodes,
                &delta,
                &resources_with(name, exact - 1),
                &mut job,
            )
            .unwrap_err();
            assert!(failure.contains(name), "{failure}");
        }
    }
    #[test]
    fn missing_header_descendant_and_record_preflight_are_rejected() {
        let (mut nodes, delta, _) = fixture();
        let config = super::super::q_resources::tests::resources();
        let missing = nodes
            .iter()
            .find(|(_, t, b)| *t == sc::tag::BYTES_CHUNK && *b == b"D-header-generation")
            .unwrap()
            .0
            .to_owned();
        struct Missing<'a>(&'a mut MemoryNodes, NodeDigest);
        impl NodeSource for Missing<'_> {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                if *d == self.1 {
                    Ok(NodeRead::Missing)
                } else {
                    self.0.read(d)
                }
            }
        }
        assert!(measure(
            &mut Missing(&mut nodes, missing),
            &delta,
            &config,
            &mut job()
        )
        .unwrap_err()
        .contains("node_missing"));
        let mut value = super::super::q_resources::tests::input();
        value["limits"]["max_atomic_delta_records"] = Value::String("1".into());
        let limited = QResources::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        // 记录预检先于缺失正文的完整闭包读取；坏图仍绝不返回引用。
        assert!(measure(
            &mut Missing(&mut nodes, missing),
            &delta,
            &limited,
            &mut job()
        )
        .unwrap_err()
        .contains("max_atomic_delta_records"));
    }
}
