//! #1394 C §5.6.1：完整实际 Delta 的三项原子限额。
//! 调用者先绑定已发布历史；本层不凭裸根授权，也不以片段proof代替完整闭包。
use super::audit_store::RefValue;
use super::audit_store::{NativeAdmission, NativeTree};
use super::q_resources::{Limit, QResources};
use newchan_rust::s_content::access::Access;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::s_content::{
    self as sc, CodecError, Kind, NodeDigest, NodeRead, NodeSource, RefView,
};

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

/// 普通/具体Closed源仅改变容器所有者与读适配；记录预检、闭包与原子三门共一核。
pub(super) fn measure<'a>(
    source: &mut dyn NodeSource,
    delta: impl Into<RefView<'a>>,
    resources: &QResources,
    job: &mut dyn Access,
) -> Result<Usage, String> {
    let mut meter = Meter {
        source,
        job,
        seen: NativeTree::default(),
        nodes: 0,
        bytes: 0,
        max_nodes: resources.limit(Limit::DeltaNodes),
        max_bytes: resources.limit(Limit::DeltaNodeBytes),
    };
    measure_core(&mut PlainDelta(&mut meter), delta.into(), resources, None)
}
pub(super) fn measure_closed(
    source: &mut sc::ClosedCapturedOwner<'_>,
    delta: RefView<'_>,
    resources: &QResources,
    job: &mut dyn Access,
) -> Result<Usage, String> {
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    {
        let _ = (source, delta, resources, job);
        return Err("prototype_audit_allocation_incomplete：owned Delta计量未启用".into());
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        let budget = source.allocation_budget();
        let mut reader = ClosedDelta {
            source,
            job,
            meter: ClosedCounts {
                seen: super::business_json::new_map(Some(&budget)),
                nodes: 0,
                bytes: 0,
                max_nodes: resources.limit(Limit::DeltaNodes),
                max_bytes: resources.limit(Limit::DeltaNodeBytes),
            },
        };
        measure_core(&mut reader, delta, resources, Some(&budget))
    }
}
trait DeltaRead {
    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        None
    }
    fn lookup(&mut self, root: RefView<'_>, step: Step<'_>) -> Result<RefValue, CodecError>;
    fn verify(&mut self, root: RefView<'_>) -> Result<(), CodecError>;
    fn record(&mut self) -> Result<(), CodecError>;
    fn finish(&mut self, records: u64) -> Result<Usage, CodecError>;
}
enum Step<'a> {
    Field(&'a str),
    Index(u64),
}
fn measure_core(
    reader: &mut impl DeltaRead,
    delta: RefView<'_>,
    resources: &QResources,
    budget: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<Usage, String> {
    if delta.kind() != Kind::Object || delta.extent() != Some(17) {
        return Err("QDeltaInvalid：完整Delta须为D十字段与七变化集合".into());
    }
    let mut arrays = super::business_json::new_work(budget);
    let mut records = 0u128;
    for name in RECORDS {
        let array = reader.lookup(delta, Step::Field(name)).map_err(err)?;
        if array.kind() != Kind::Array {
            return Err(format!("QDeltaInvalid：{name}不是数组"));
        }
        let n = array
            .extent()
            .ok_or_else(|| "QDeltaInvalid：数组缺长度".to_string())?;
        records += u128::from(n);
        if records > u128::from(resources.limit(Limit::DeltaRecords)) {
            return Err(limit("max_atomic_delta_records").to_string());
        }
        if let Some(work) = reader.codec_work() {
            arrays.try_reserve_native(work, 1).map_err(err)?;
        }
        arrays.try_push((array, n)).map_err(err)?;
    }
    reader.verify(delta).map_err(err)?;
    for (array, n) in arrays {
        for i in 0..n {
            reader.record().map_err(err)?;
            reader.lookup(array.view(), Step::Index(i)).map_err(err)?;
        }
    }
    reader.finish(records as u64).map_err(err)
}
struct PlainDelta<'a, 'source>(&'a mut Meter<'source>);
impl DeltaRead for PlainDelta<'_, '_> {
    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.0.codec_work()
    }
    fn lookup(&mut self, r: RefView<'_>, step: Step<'_>) -> Result<RefValue, CodecError> {
        let path = SingleStep(match step {
            Step::Field(k) => sc::path::PathStepView::Field(k),
            Step::Index(i) => sc::path::PathStepView::Index(i),
        });
        sc::path::lookup_path_view_native(self.0, r, &path).map(RefValue::Plain)
    }
    fn verify(&mut self, r: RefView<'_>) -> Result<(), CodecError> {
        sc::verify_refview(self.0, r, sc::Mode::Public)
    }
    fn record(&mut self) -> Result<(), CodecError> {
        self.0.job.charge_records(1)
    }
    fn finish(&mut self, records: u64) -> Result<Usage, CodecError> {
        self.0.job.pre_io_check()?;
        Ok(Usage {
            nodes: self.0.nodes,
            node_bytes: self.0.bytes,
            records,
        })
    }
}
struct SingleStep<'a>(sc::path::PathStepView<'a>);
impl sc::path::PathView for SingleStep<'_> {
    fn len(&self) -> usize {
        1
    }
    fn step(&self, index: usize) -> Option<sc::path::PathStepView<'_>> {
        (index == 0).then_some(self.0)
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
struct ClosedCounts {
    seen: super::business_json::BusinessMap<NodeDigest, ()>,
    nodes: u64,
    bytes: u64,
    max_nodes: u64,
    max_bytes: u64,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl ClosedCounts {
    fn accept(&mut self, d: &NodeDigest, body: &[u8]) -> Result<(), CodecError> {
        if !self.seen.contains_key(d) {
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
            self.seen.insert(*d, ())?;
        }
        Ok(())
    }
}
/// QAccess的前/后read调用点承接原Meter的begin/source/deadline/accept/caps/codec-F顺序。
/// 后点不新增期限门；完整measure末尾才执行原post检查。
#[cfg(feature = "s_allocation_json_testonly")]
struct DeltaAccess<'a> {
    job: &'a mut dyn Access,
    counts: &'a mut ClosedCounts,
    active_read: bool,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl Access for DeltaAccess<'_> {
    fn owner_job(&mut self) -> Option<&mut sc::job::Job> {
        self.job.owner_job()
    }
    fn pre_io_check(&mut self) -> Result<(), CodecError> {
        if self.active_read {
            self.active_read = false;
            Ok(())
        } else {
            self.job.begin_first_read()?;
            self.active_read = true;
            Ok(())
        }
    }
    fn accept_node(
        &mut self,
        d: &NodeDigest,
        t: u8,
        b: &[u8],
    ) -> Result<sc::job::NodeAcceptance, CodecError> {
        self.job.check_deadline()?;
        let accepted = self.job.accept_node(d, t, b)?;
        self.counts.accept(d, b)?;
        // 原Meter是raw NodeSource；read_checked对同一当次body还有这一次完整F。
        sc::job::CheckedNodeUsage::checked(d, t, b)?;
        Ok(accepted)
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
fn closed_missing_deadline<T>(
    result: Result<T, CodecError>,
    job: &mut dyn Access,
) -> Result<T, CodecError> {
    // immutable captured owner缺原件对应旧Meter的NodeRead::Missing，而非source Err；
    // 原Meter在这个Missing之后仍先核deadline，再由codec返回node_missing。
    if result
        .as_ref()
        .is_err_and(|error| error.code() == sc::code::NODE_MISSING)
    {
        job.check_deadline()?;
    }
    result
}
#[cfg(feature = "s_allocation_json_testonly")]
struct ClosedDelta<'a, 'capture> {
    source: &'a mut sc::ClosedCapturedOwner<'capture>,
    job: &'a mut dyn Access,
    meter: ClosedCounts,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl DeltaRead for ClosedDelta<'_, '_> {
    fn lookup(&mut self, r: RefView<'_>, step: Step<'_>) -> Result<RefValue, CodecError> {
        let path = SingleStep(match step {
            Step::Field(k) => sc::path::PathStepView::Field(k),
            Step::Index(i) => sc::path::PathStepView::Index(i),
        });
        let mut access = DeltaAccess {
            job: self.job,
            counts: &mut self.meter,
            active_read: false,
        };
        let result = self
            .source
            .with_q_access(&mut access, |scope| scope.lookup_path_view_owned(r, &path))
            .map(RefValue::Owned);
        drop(access);
        closed_missing_deadline(result, self.job)
    }
    fn verify(&mut self, r: RefView<'_>) -> Result<(), CodecError> {
        let mut access = DeltaAccess {
            job: self.job,
            counts: &mut self.meter,
            active_read: false,
        };
        let result = self.source.with_q_access(&mut access, |scope| {
            scope.verify_fresh_owned(r, sc::Mode::Public)
        });
        drop(access);
        closed_missing_deadline(result, self.job)
    }
    fn record(&mut self) -> Result<(), CodecError> {
        self.job.charge_records(1)
    }
    fn finish(&mut self, records: u64) -> Result<Usage, CodecError> {
        self.job.pre_io_check()?;
        Ok(Usage {
            nodes: self.meter.nodes,
            node_bytes: self.meter.bytes,
            records,
        })
    }
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
    job: &'a mut dyn Access,
    seen: NativeTree<NodeDigest, ()>,
    nodes: u64,
    bytes: u64,
    max_nodes: u64,
    max_bytes: u64,
}
impl NodeSource for Meter<'_> {
    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source.codec_work()
    }
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
            if !self.seen.contains_key(digest) {
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
                self.seen.insert_admitted(
                    *digest,
                    (),
                    &NativeAdmission(self.source.codec_work()),
                )?;
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
        ContentRef, MemoryNodes, Value as V,
    };
    use serde_json::{json, Value};
    // TestOnly，不是产品期限（#1394 B1）：功能测试 job 的 max_elapsed_ms 放长到 600000，不随 runner 速度超期；测 job 期限的用例自带受控时钟。
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"1000","max_total_requests":"20","max_elapsed_ms":"600000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
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
        let mut input = super::super::q_resources::tests::functional_input();
        input["limits"][key] = json!(n.to_string());
        QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap()
    }
    #[test]
    fn full_closure_includes_header_and_shared_nodes_once_with_independent_delta_caps() {
        let (mut nodes, delta, expected) = fixture();
        let config = super::super::q_resources::tests::functional_resources();
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
        let config = super::super::q_resources::tests::functional_resources();
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
        let mut value = super::super::q_resources::tests::functional_input();
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
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn closed_delta_common_measure_keeps_real_f_job_usage_atomic_gates_and_drop0() {
        fn fixed_job() -> Job {
            let limits = job().snapshot().limits;
            let now = std::time::Instant::now();
            Job::new(
                limits,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(move || now),
            )
            .unwrap()
        }
        let (nodes, delta, expected) = fixture();
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let base = sc::capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = sc::capture_allocation::OwnedBudget::new(base + 4_000_000);
        let mut slab = sc::capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (digest, tag, body) in nodes.iter() {
            slab.push(*digest, tag, body).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let mut owner = capture
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let mut configs = vec![super::super::q_resources::tests::functional_resources()];
        for (key, exact) in [
            ("max_atomic_delta_node_bytes", expected.node_bytes),
            ("max_atomic_delta_nodes", expected.nodes),
            ("max_atomic_delta_records", expected.records),
        ] {
            configs.push(resources_with(key, exact));
            configs.push(resources_with(key, exact - 1));
        }
        for resources in configs {
            let mut ordinary = nodes.clone();
            let mut oldjob = fixed_job();
            let mut newjob = fixed_job();
            let before = sc::frame_calls_on_thread();
            let old = measure(&mut ordinary, &delta, &resources, &mut oldjob);
            let oldf = sc::frame_calls_on_thread() - before;
            let before = sc::frame_calls_on_thread();
            let new = measure_closed(&mut owner, (&delta).into(), &resources, &mut newjob);
            let newf = sc::frame_calls_on_thread() - before;
            assert_eq!(new, old);
            assert_eq!(newjob.snapshot(), oldjob.snapshot());
            assert_eq!(newf, oldf, "必须对拍真实F，不以猜测计数冒充");
            assert_eq!(owner.overlay_peak_used(), (0, 0));
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            assert_eq!(capture.retained_layout_bytes().unwrap(), base as u64);
        }
        drop(owner);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
