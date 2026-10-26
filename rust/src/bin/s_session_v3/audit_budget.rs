//! #1394 S3 审计计费桥：把真实节点 F 核验与逻辑记录访问接入同一个 Job。
//!
//! `Receipt` 只保存不可变的已核节点计费值和记录次数；它不是内容图证明、上下文证明
//! 或访问授权。只有真实 `CheckedNodeUsage` 能进入 receipt，重放仍经过目标 Job 的
//! 期限、停止、节点额度和记录额度门。

use super::business_json::{self as bj, BusinessMap};
use newchan_rust::s_content as sc;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::s_content::{
    job::{CheckedFrame, CheckedNodeUsage},
    NodeDigest,
};
use sc::access::Access;
use std::{cell::RefCell, mem::size_of};

pub(crate) trait Budget {
    fn precheck(&self) -> Result<(), sc::CodecError>;
    /// Found 后、计算 F 前；Missing 不调用。
    fn prepare_node(&self) -> Result<(), sc::CodecError>;
    /// 只接纳本次读取当场形成的帧；调用方已完成 prepare_node。
    fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), sc::CodecError>;
    fn node(&self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError>;
    fn record(&self) -> Result<(), sc::CodecError>;
    /// #1527×#1602：只有写前准入门面给出，供 S3 schema 核记录所见 structure_deltas 形状。
    fn before_write(&self) -> Option<&std::cell::Cell<crate::DeltaShape>> {
        None
    }
}

/// #1527：旧域节点只能来自完整原行核验；资格只借当前Job，不进缓存。
#[allow(dead_code, reason = "#1527：成员借用限定当前W访问，计费不再次展开成员")]
pub(crate) enum LegacyRecordRef<'a> {
    Json(&'a serde_json::Value),
    Canonical(&'a str),
}
pub(crate) trait LegacyObserver {
    /// 私有 mixed SQL 前缀只消费原 getter 已核过的判别；普通观察者不改变策略。
    fn select_legacy_capture(&self, _graph: bool, _records_in_graph: bool) {}
    fn select_legacy_tables(&self, _v2: bool, _ohlc: bool, _delta: bool, _graph: bool) {}
    fn visit_record(
        &self,
        family: &'static str,
        process: &'static str,
        member: LegacyRecordRef<'_>,
    ) -> Result<(), sc::CodecError>;
    fn precheck(&self) -> Result<(), sc::CodecError>;
    fn check_work(
        &self,
        work: &sc::capture_allocation::scratch::WorkScope,
    ) -> Result<(), sc::CodecError>;
    fn accept_checked(
        &self,
        row: &sc::legacy_evidence::CheckedLegacyEvidenceRow,
    ) -> Result<(), sc::CodecError>;
}
/// 仅借本次审计账与截止时间；不可留存进 capture/context/memo。
pub(crate) struct NativeMeasure<'a> {
    pub(super) budget: Option<&'a dyn Budget>,
    pub(super) deadline: Option<std::time::Instant>,
}
impl sc::CaptureReadObserver for NativeMeasure<'_> {
    fn before_read(&self) -> Result<(), sc::CodecError> {
        super::history::check_deadline(self.deadline)
            .map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        self.budget.map_or(Ok(()), Budget::precheck)
    }
    fn prepare_node(&self) -> Result<(), sc::CodecError> {
        self.budget.map_or(Ok(()), Budget::prepare_node)
    }
    fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), sc::CodecError> {
        if let Some(budget) = self.budget {
            budget.accept_prepared_frame(frame)?;
            budget.precheck()?;
        }
        Ok(())
    }
}

pub(super) struct AuditJob<'j> {
    job: RefCell<&'j mut dyn Access>,
    nodes: RefCell<BusinessMap<NodeDigest, CheckedNodeUsage>>,
    records: RefCell<u64>,
    allocation: Option<sc::capture_allocation::OwnedBudget>,
}

impl<'j> AuditJob<'j> {
    pub(super) fn new(job: &'j mut dyn Access) -> Self {
        Self::with_allocation(job, None)
    }
    pub(super) fn with_allocation(
        job: &'j mut dyn Access,
        allocation: Option<&sc::capture_allocation::OwnedBudget>,
    ) -> Self {
        Self {
            job: RefCell::new(job),
            nodes: RefCell::new(bj::new_map(allocation)),
            records: RefCell::new(0),
            allocation: allocation.cloned(),
        }
    }

    pub(super) fn finish(self) -> Receipt {
        self.try_finish()
            .expect("普通审计receipt没有fallible分配账本")
    }
    pub(super) fn try_finish(self) -> Result<Receipt, String> {
        let map = self.nodes.into_inner();
        let mut ordered = bj::new_work(self.allocation.as_ref());
        for (digest, usage) in map.iter() {
            ordered
                .try_push((*digest, *usage))
                .map_err(|e| e.to_string())?;
        }
        ordered.sort_unstable_by_key(|entry| entry.0);
        let mut nodes = bj::new_work(self.allocation.as_ref());
        for (_, usage) in ordered {
            nodes.try_push(usage).map_err(|e| e.to_string())?;
        }
        Ok(Receipt {
            nodes,
            records: self.records.into_inner(),
        })
    }

    fn record_usage(&self, usage: CheckedNodeUsage, body_len: usize) -> Result<(), sc::CodecError> {
        self.job.borrow_mut().accept_checked_usage(&usage)?;
        let digest = usage.digest();
        let mut nodes = self.nodes.borrow_mut();
        #[cfg(test)]
        let mut unique = false;
        if let Some(previous) = nodes.get(digest) {
            if previous != &usage {
                return Err(sc::CodecError::new(
                    "audit_budget",
                    "同一摘要对应的分帧计费值不一致",
                ));
            }
        } else {
            nodes.insert(*digest, usage)?;
            #[cfg(test)]
            {
                unique = true;
            }
        }
        #[cfg(test)]
        super::state::trace_checked_node(unique, sc::DOMAIN_SEPARATOR.len() + 1 + 8 + body_len);
        #[cfg(not(test))]
        let _ = body_len;
        Ok(())
    }
}

impl Budget for AuditJob<'_> {
    fn precheck(&self) -> Result<(), sc::CodecError> {
        self.job.borrow_mut().pre_io_check().map_err(codec)
    }

    fn prepare_node(&self) -> Result<(), sc::CodecError> {
        self.precheck()?;
        self.job.borrow_mut().begin_first_read()
    }

    fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), sc::CodecError> {
        self.record_usage(*frame.usage(), frame.body().len())
    }

    fn node(&self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        // 期限门先于摘要计算，保持 accept_node 的旧顺序；RefCell 借用不跨 F 核或 Job 调用。
        self.prepare_node()?;
        let usage = CheckedNodeUsage::checked(digest, tag, body)?;
        self.record_usage(usage, body.len())
    }

    fn record(&self) -> Result<(), sc::CodecError> {
        self.job.borrow_mut().charge_records(1)?;
        let mut records = self.records.borrow_mut();
        *records = records
            .checked_add(1)
            .ok_or_else(|| sc::CodecError::new("audit_budget", "审计记录计数溢出"))?;
        Ok(())
    }
}

/// 只能由 `AuditJob::finish` 产生；不实现序列化，也不提供 unchecked 构造。
pub(super) struct Receipt {
    nodes: sc::CodecVec<CheckedNodeUsage>,
    records: u64,
}

impl Receipt {
    pub(super) fn replay(&self, job: &mut dyn Access) -> Result<(), String> {
        // 仅回放消费者累计账；Q服务端的各阶段/单包门在调用方执法，不能伪造一份Job。
        if job.owner_job().is_none() {
            return Ok(());
        }
        job.pre_io_check().map_err(codec_string)?;
        for usage in &self.nodes {
            job.accept_checked_usage(usage).map_err(codec_string)?;
        }
        for _ in 0..self.records {
            job.charge_records(1).map_err(codec_string)?;
        }
        job.pre_io_check().map_err(codec_string)?;
        Ok(())
    }

    pub(super) fn retained_heap_bytes(&self) -> Result<u64, String> {
        let nodes = self
            .nodes
            .capacity()
            .checked_mul(size_of::<CheckedNodeUsage>())
            .ok_or_else(|| "审计receipt容量溢出".to_string())?;
        u64::try_from(nodes).map_err(|_| "审计receipt容量超出u64".to_string())
    }
}

fn codec(error: sc::CodecError) -> sc::CodecError {
    error
}

fn codec_string(error: sc::CodecError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::job::{Existing, JobLimits};
    use serde_json::json;
    use std::time::{Duration, Instant};

    fn job_limits(node_bytes: u64, nodes: u64, records: u64, elapsed: u64) -> JobLimits {
        JobLimits::from_canonical_json(&json!({
            "max_total_wire_bytes":"1048576",
            "max_total_node_bytes":node_bytes.to_string(),
            "max_total_nodes":nodes.to_string(),
            "max_total_records":records.to_string(),
            "max_total_requests":"20",
            "max_elapsed_ms":elapsed.to_string(),
            "max_staged_bytes":"1048576",
            "max_store_bytes":"33554432"
        }))
        .unwrap()
    }

    fn make_job(node_bytes: u64, nodes: u64, records: u64) -> Job {
        Job::new(
            job_limits(node_bytes, nodes, records, 10_000),
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap()
    }

    #[test]
    fn metered_hash_read_checks_f_once_and_keeps_receipt_accounting() {
        use newchan_rust::s_content::{MemoryNodes, NodeSource};
        let digest = sc::frame(sc::tag::NULL, &[]);
        let mut source = MemoryNodes::new();
        source.insert_unchecked(digest, sc::tag::NULL, vec![]);
        let mut job = make_job(1024, 8, 8);
        let receipt = {
            let audit = AuditJob::new(&mut job);
            let before = sc::frame_calls_on_thread();
            {
                let mut reader = super::super::history::HashReader::metered(
                    &mut source,
                    None,
                    None,
                    Some(&audit),
                );
                let frame = reader.checked_read(&digest).unwrap().expect("节点存在");
                assert_eq!(frame.digest(), &digest);
            }
            assert_eq!(
                sc::frame_calls_on_thread() - before,
                1,
                "一次实际计费读只计算一次 F"
            );
            audit.record().unwrap();
            audit.finish()
        };
        let snapshot = job.snapshot();
        assert!(snapshot.started);
        assert_eq!(snapshot.nodes, 1);
        assert_eq!(
            snapshot.node_bytes,
            (sc::DOMAIN_SEPARATOR.len() + 1 + 8) as u64
        );
        assert_eq!(snapshot.records, 1);
        let mut replay = make_job(1024, 8, 8);
        receipt.replay(&mut replay).unwrap();
        assert_eq!(replay.snapshot().nodes, snapshot.nodes);
        assert_eq!(replay.snapshot().node_bytes, snapshot.node_bytes);
        assert_eq!(replay.snapshot().records, snapshot.records);
    }

    #[test]
    fn metered_hash_read_preserves_pre_io_missing_and_bad_frame_gates() {
        use newchan_rust::s_content::{MemoryNodes, NodeSource};
        let digest = sc::frame(sc::tag::NULL, &[]);
        let mut empty = MemoryNodes::new();
        let mut missing_job = make_job(1024, 8, 8);
        {
            let audit = AuditJob::new(&mut missing_job);
            let mut reader =
                super::super::history::HashReader::metered(&mut empty, None, None, Some(&audit));
            assert!(reader.checked_read(&digest).unwrap().is_none());
        }
        assert!(!missing_job.snapshot().started, "Missing 不伪造首读起钟");
        assert_eq!(missing_job.snapshot().nodes, 0);

        let mut broken = MemoryNodes::new();
        broken.insert_unchecked(digest, sc::tag::NULL, vec![9]);
        let mut bad_job = make_job(1024, 8, 8);
        {
            let audit = AuditJob::new(&mut bad_job);
            let mut reader =
                super::super::history::HashReader::metered(&mut broken, None, None, Some(&audit));
            assert_eq!(
                reader.checked_read(&digest).unwrap_err().code(),
                sc::code::NODE_DIGEST
            );
        }
        assert!(bad_job.snapshot().started, "Found 首读在 F 前起钟");
        assert_eq!(bad_job.snapshot().nodes, 0, "坏帧不入账");
        assert_eq!(bad_job.snapshot().node_bytes, 0);

        struct CountReads(usize, bool);
        impl NodeSource for CountReads {
            fn read(&mut self, _: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                self.0 += 1;
                Ok(if self.1 {
                    sc::NodeRead::Found {
                        tag: sc::tag::NULL,
                        body: vec![],
                    }
                } else {
                    sc::NodeRead::Missing
                })
            }
        }
        let mut source = CountReads(0, false);
        let mut stopped = make_job(1024, 8, 8);
        assert!(stopped.charge_wire_bytes(1_048_577).is_err());
        {
            let audit = AuditJob::new(&mut stopped);
            let mut reader =
                super::super::history::HashReader::metered(&mut source, None, None, Some(&audit));
            assert!(reader.checked_read(&digest).is_err());
        }
        assert_eq!(source.0, 0, "停止门须先于 IO");

        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let now = Instant::now();
        let advanced = Arc::new(AtomicBool::new(false));
        let clock = Arc::clone(&advanced);
        let mut deadline_job = Job::new(
            job_limits(1024, 8, 8, 1),
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || {
                now + Duration::from_millis(u64::from(clock.load(Ordering::SeqCst)) * 2)
            }),
        )
        .unwrap();
        let mut source = CountReads(0, true);
        {
            let audit = AuditJob::new(&mut deadline_job);
            let mut reader =
                super::super::history::HashReader::metered(&mut source, None, None, Some(&audit));
            assert!(reader.checked_read(&digest).unwrap().is_some());
            advanced.store(true, Ordering::SeqCst);
            assert!(
                reader.checked_read(&digest).is_err(),
                "首次读取不能为第二次续期"
            );
        }
        assert_eq!(source.0, 1, "期限门须在第二次 IO 前拒绝");
        assert_eq!(deadline_job.snapshot().stopped, Some(sc::job::DEADLINE));
    }

    #[test]
    fn real_node_usage_is_checked_and_revisits_dedupe_nodes_but_count_records() {
        let body = b"audit-node";
        let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
        let mut job = make_job(1024, 8, 8);
        let receipt = {
            let audit = AuditJob::new(&mut job);
            audit.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            audit.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            audit.record().unwrap();
            audit.record().unwrap();
            audit.finish()
        };
        assert_eq!(job.snapshot().nodes, 1);
        assert_eq!(job.snapshot().records, 2);
        assert!(receipt.retained_heap_bytes().unwrap() >= size_of::<CheckedNodeUsage>() as u64);
    }

    #[test]
    fn bad_frame_cannot_mint_usage_or_charge_job() {
        let body = b"bad-node";
        let wrong = [7_u8; 32];
        let mut job = make_job(1024, 8, 8);
        let audit = AuditJob::new(&mut job);
        assert!(audit.node(&wrong, sc::tag::BYTES_CHUNK, body).is_err());
        assert_eq!(job.snapshot().nodes, 0);
        assert_eq!(job.snapshot().node_bytes, 0);
    }

    #[test]
    fn receipt_replay_dedupes_nodes_per_job_but_recharges_records() {
        let body = b"receipt-node";
        let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
        let mut cold = make_job(1024, 8, 8);
        let receipt = {
            let audit = AuditJob::new(&mut cold);
            audit.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            audit.record().unwrap();
            audit.record().unwrap();
            audit.finish()
        };
        let mut warm = make_job(1024, 8, 8);
        receipt.replay(&mut warm).unwrap();
        receipt.replay(&mut warm).unwrap();
        assert_eq!(warm.snapshot().nodes, 1);
        assert_eq!(warm.snapshot().records, 4);
    }

    #[test]
    fn replay_respects_node_record_and_deadline_limits() {
        let body = b"limited-node";
        let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
        let mut source = make_job(1024, 8, 8);
        let receipt = {
            let audit = AuditJob::new(&mut source);
            audit.node(&digest, sc::tag::BYTES_CHUNK, body).unwrap();
            audit
                .node(
                    &sc::frame(sc::tag::BYTES_CHUNK, b"second"),
                    sc::tag::BYTES_CHUNK,
                    b"second",
                )
                .unwrap();
            audit.record().unwrap();
            audit.finish()
        };
        let mut tiny_bytes = make_job(1, 8, 8);
        assert!(receipt
            .replay(&mut tiny_bytes)
            .unwrap_err()
            .contains("max_total_node_bytes"));
        let mut tiny_nodes = make_job(1024, 1, 8);
        assert!(receipt
            .replay(&mut tiny_nodes)
            .unwrap_err()
            .contains("max_total_nodes"));
        let mut tiny_records = make_job(1024, 8, 1);
        receipt.replay(&mut tiny_records).unwrap();
        assert!(receipt.replay(&mut tiny_records).is_err());

        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let now = Instant::now();
        let clock = Arc::new(AtomicBool::new(false));
        let copy = Arc::clone(&clock);
        let mut expired = Job::new(
            job_limits(1024, 8, 8, 1),
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || {
                now + Duration::from_millis(if copy.load(Ordering::SeqCst) { 2 } else { 0 })
            }),
        )
        .unwrap();
        expired.begin_first_read().unwrap();
        clock.store(true, Ordering::SeqCst);
        assert!(receipt
            .replay(&mut expired)
            .unwrap_err()
            .contains("job_deadline"));
        assert_eq!(expired.snapshot().nodes, 0);
    }
}
