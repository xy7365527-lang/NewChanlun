//! 受信进程内内容读取的访问记账层。
//!
//! `Some(Job)` 是消费者侧累计合同；`None` 明确表示本层没有消费者账本，
//! 不构造、不推导、也不宣称存在无限 Job。Q/服务调用方须在边界自行执法
//! 冻结的 frame、捕获、期限、Delta 与回复资源。

use super::{
    job::{CheckedFrame, CheckedNodeUsage, Job, NodeAcceptance},
    CodecError, NodeDigest, NodeRead, NodeSource,
};
use std::time::Instant;

/// 内容遍历使用的受信进程内调用方。
pub trait Access {
    /// 返回调用方持有的消费者 Job；无消费者账本时返回 None。
    fn owner_job(&mut self) -> Option<&mut Job>;

    /// 实际调用方的分配所有者，与消费者 Job 和冻结 Q21 资源分别持有。
    /// 不从其它 byte 限额推导新 heap 数值；未显式提供时为 None。
    fn allocation_budget(&self) -> Option<&super::capture_allocation::OwnedBudget> {
        None
    }

    fn pre_io_check(&mut self) -> Result<(), CodecError> {
        match self.owner_job() {
            Some(job) => job.pre_io_check(),
            None => Ok(()),
        }
    }

    fn begin_first_read(&mut self) -> Result<(), CodecError> {
        match self.owner_job() {
            Some(job) => job.begin_first_read(),
            None => Ok(()),
        }
    }

    fn check_deadline(&mut self) -> Result<(), CodecError> {
        match self.owner_job() {
            // Job本身是&self方法；显式调用，避免本trait的&mut方法再次胜出。
            Some(job) => Job::check_deadline(job),
            None => Ok(()),
        }
    }

    fn charge_records(&mut self, accesses: u64) -> Result<(), CodecError> {
        match self.owner_job() {
            Some(job) => job.charge_records(accesses),
            None => Ok(()),
        }
    }

    fn charge_requests(&mut self, attempts: u64) -> Result<(), CodecError> {
        match self.owner_job() {
            Some(job) => job.charge_requests(attempts),
            None => Ok(()),
        }
    }

    fn charge_wire_bytes(&mut self, actual_bytes: u64) -> Result<(), CodecError> {
        match self.owner_job() {
            Some(job) => job.charge_wire_bytes(actual_bytes),
            None => Ok(()),
        }
    }

    /// 有 Job 时保留 Job::accept_node 的先 gate 后核 F 顺序；无 owner 时仍核
    /// 实际 `F(tag, body)`，但不保存累计账。
    fn accept_node(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<NodeAcceptance, CodecError> {
        match self.owner_job() {
            Some(job) => job.accept_node(digest, tag, body),
            None => {
                CheckedNodeUsage::checked(digest, tag, body)?;
                Ok(NodeAcceptance::FirstAccept)
            }
        }
    }

    /// 在旧 `accept_node` 时点核真实 Vec 并记账，返回可沿同次读取下传的帧。
    fn accept_frame(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> Result<CheckedFrame, CodecError> {
        match self.owner_job() {
            Some(job) => job.accept_frame(digest, tag, body),
            None => CheckedFrame::checked(digest, tag, body),
        }
    }

    /// 将已从真实原件核出的不可变用量重放到账本；这只是记账，不能替代
    /// 当前内容验证。
    fn accept_checked_usage(
        &mut self,
        usage: &CheckedNodeUsage,
    ) -> Result<NodeAcceptance, CodecError> {
        match self.owner_job() {
            Some(job) => job.accept_checked_usage(usage),
            None => Ok(NodeAcceptance::FirstAccept),
        }
    }

    fn read_deadline(&mut self) -> Result<Option<Instant>, CodecError> {
        match self.owner_job() {
            Some(job) => job.first_read_deadline().map(Some),
            None => Ok(None),
        }
    }
}

impl Access for Job {
    fn owner_job(&mut self) -> Option<&mut Job> {
        Some(self)
    }
}

/// 在每次节点读取前后套用 Access 的 NodeSource 适配器；不增加服务全局期限
/// 或 wire 字节策略。
pub struct MeteredSource<'a> {
    access: &'a mut dyn Access,
    source: &'a mut dyn NodeSource,
}

impl<'a> MeteredSource<'a> {
    pub fn new(access: &'a mut dyn Access, source: &'a mut dyn NodeSource) -> Self {
        Self { access, source }
    }
}

impl NodeSource for MeteredSource<'_> {
    fn max_render_bytes(&self) -> Option<u64> {
        self.source.max_render_bytes()
    }

    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.access.begin_first_read()?;
        let read = self.source.read(digest)?;
        if let NodeRead::Found { tag, body } = &read {
            self.access.accept_node(digest, *tag, body)?;
        }
        self.access.pre_io_check()?;
        Ok(read)
    }

    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.access.begin_first_read()?;
        let read = self.source.read(digest)?;
        let checked = match read {
            NodeRead::Missing => None,
            NodeRead::Found { tag, body } => Some(self.access.accept_frame(digest, tag, body)?),
        };
        self.access.pre_io_check()?;
        Ok(checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s_content::{frame, tag};
    use serde_json::json;
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    fn job(limits: ValueLimits) -> Job {
        let value = json!({
            "max_total_wire_bytes": limits.wire.to_string(),
            "max_total_node_bytes": limits.node_bytes.to_string(),
            "max_total_nodes": limits.nodes.to_string(),
            "max_total_records": limits.records.to_string(),
            "max_total_requests": "10",
            "max_elapsed_ms": "1000",
            "max_staged_bytes": "1048576",
            "max_store_bytes": "1048576"
        });
        Job::new(
            super::super::job::JobLimits::from_canonical_json(&value).unwrap(),
            super::super::job::Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap()
    }

    struct ValueLimits {
        wire: u64,
        node_bytes: u64,
        nodes: u64,
        records: u64,
    }

    #[test]
    fn deadline_check_through_access_returns_and_preserves_expiry() {
        let (send, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let start = Instant::now();
            let clock = Arc::new(Mutex::new(start));
            let clock_for_job = Arc::clone(&clock);
            let limits = super::super::job::JobLimits {
                max_total_wire_bytes: 100,
                max_total_node_bytes: 100,
                max_total_nodes: 10,
                max_total_records: 10,
                max_total_requests: 10,
                max_elapsed_ms: 1,
                max_staged_bytes: 1024,
                max_store_bytes: 1024,
            };
            let mut owner = Job::new(
                limits,
                super::super::job::Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(move || *clock_for_job.lock().unwrap()),
            )
            .unwrap();
            let access: &mut dyn Access = &mut owner;
            access.begin_first_read().unwrap();
            let before = access.check_deadline();
            *clock.lock().unwrap() = start + Duration::from_millis(2);
            let after = access.check_deadline().map_err(|e| e.code().to_owned());
            send.send((before.is_ok(), after)).unwrap();
        });
        // 单独线程让同名方法误递归表现为有界失败，而非锁死整个回归。
        let (before, after) = receive
            .recv_timeout(Duration::from_secs(1))
            .expect("Access期限检查没有返回");
        worker.join().unwrap();
        assert!(before);
        assert_eq!(after.unwrap_err(), super::super::job::DEADLINE);
    }

    #[test]
    fn job_access_delegates_dedupe_and_records_and_limit() {
        let body = vec![1, 2, 3];
        let digest = frame(tag::BYTES_CHUNK, &body);
        let mut owner = job(ValueLimits {
            wire: 100,
            node_bytes: 100,
            nodes: 1,
            records: 2,
        });
        let access: &mut dyn Access = &mut owner;
        access
            .accept_node(&digest, tag::BYTES_CHUNK, &body)
            .unwrap();
        access
            .accept_node(&digest, tag::BYTES_CHUNK, &body)
            .unwrap();
        access.charge_records(2).unwrap();
        assert!(access.charge_records(1).is_err());
        assert_eq!(owner.snapshot().nodes, 1);
        assert_eq!(owner.snapshot().records, 2);
    }

    #[test]
    fn no_owner_still_rejects_bad_frame_without_constructing_job() {
        struct NoOwner;
        impl Access for NoOwner {
            fn owner_job(&mut self) -> Option<&mut Job> {
                None
            }
        }
        let mut access = NoOwner;
        let good_body = vec![4, 5];
        let good = frame(tag::BYTES_CHUNK, &good_body);
        assert_eq!(
            access
                .accept_node(&good, tag::BYTES_CHUNK, &good_body)
                .unwrap(),
            NodeAcceptance::FirstAccept
        );
        let bad = [0u8; 32];
        assert!(access
            .accept_node(&bad, tag::BYTES_CHUNK, &good_body)
            .is_err());
        assert_eq!(access.read_deadline().unwrap(), None);
    }

    #[test]
    fn job_stop_is_checked_before_bad_digest_through_trait_object() {
        let body = vec![6, 7];
        let digest = frame(tag::BYTES_CHUNK, &body);
        let mut owner = job(ValueLimits {
            wire: 100,
            node_bytes: 100,
            nodes: 1,
            records: 1,
        });
        let access: &mut dyn Access = &mut owner;
        access
            .accept_node(&digest, tag::BYTES_CHUNK, &body)
            .unwrap();
        let bad = [0u8; 32];
        access.charge_records(2).unwrap_err();
        let error = access
            .accept_node(&bad, tag::BYTES_CHUNK, &body)
            .unwrap_err();
        assert_ne!(error.code(), crate::s_content::code::NODE_DIGEST);
    }

    #[test]
    fn trait_object_uses_jobs_absolute_deadline_before_bad_digest() {
        let start = Instant::now();
        let clock = Arc::new(Mutex::new(start));
        let clock_for_job = Arc::clone(&clock);
        let limits = super::super::job::JobLimits {
            max_total_wire_bytes: 100,
            max_total_node_bytes: 100,
            max_total_nodes: 10,
            max_total_records: 10,
            max_total_requests: 10,
            max_elapsed_ms: 1,
            max_staged_bytes: 1_048_576,
            max_store_bytes: 1_048_576,
        };
        let mut owner = Job::new(
            limits,
            super::super::job::Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || *clock_for_job.lock().unwrap()),
        )
        .unwrap();
        let access: &mut dyn Access = &mut owner;
        access.begin_first_read().unwrap();
        *clock.lock().unwrap() = start + Duration::from_millis(2);
        let bad = [0u8; 32];
        let error = access
            .accept_node(&bad, tag::BYTES_CHUNK, &[8])
            .unwrap_err();
        assert_eq!(error.code(), super::super::job::DEADLINE);
    }
    #[test]
    fn metered_source_counts_first_source_read_time_in_consumer_deadline() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        use std::time::Duration;
        let clock = Arc::new(AtomicBool::new(false));
        let captured = Arc::clone(&clock);
        let now = Instant::now();
        let limits=super::super::job::JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1024","max_total_node_bytes":"1024","max_total_nodes":"8","max_total_records":"8","max_total_requests":"8","max_elapsed_ms":"1000","max_staged_bytes":"1024","max_store_bytes":"1024"})).unwrap();
        let mut job = Job::new(
            limits,
            super::super::job::Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || {
                now + Duration::from_millis(if captured.load(Ordering::SeqCst) {
                    2000
                } else {
                    0
                })
            }),
        )
        .unwrap();
        struct Slow {
            clock: Arc<AtomicBool>,
        }
        impl NodeSource for Slow {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.clock.store(true, Ordering::SeqCst);
                Ok(NodeRead::Found {
                    tag: tag::BYTES_CHUNK,
                    body: b"slow-first-read".to_vec(),
                })
            }
        }
        let mut source = Slow { clock };
        let digest = frame(tag::BYTES_CHUNK, b"slow-first-read");
        let failure = MeteredSource::new(&mut job, &mut source)
            .read(&digest)
            .err()
            .expect("首次底层读取耗时必须在job绝对期限内");
        assert_eq!(failure.code(), super::super::job::DEADLINE);
        assert_eq!(job.snapshot().nodes, 0);
    }
    #[test]
    fn fragment_read_starts_consumer_clock_before_its_first_local_source_read() {
        use crate::s_content::{self as sc, MemoryNodes};
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        use std::time::Duration;
        let mut nodes = MemoryNodes::new();
        let root = sc::write_value(&mut nodes, &sc::Value::Null)
            .unwrap()
            .content_ref();
        let clock = Arc::new(AtomicBool::new(false));
        let copy = Arc::clone(&clock);
        let now = Instant::now();
        let caps = job(ValueLimits {
            wire: 1000,
            node_bytes: 1000,
            nodes: 8,
            records: 8,
        })
        .snapshot()
        .limits;
        let mut owner = Job::new(
            caps,
            sc::job::Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || {
                now + Duration::from_millis(if copy.load(Ordering::SeqCst) { 2000 } else { 0 })
            }),
        )
        .unwrap();
        struct Slow {
            nodes: MemoryNodes,
            clock: Arc<AtomicBool>,
        }
        impl NodeSource for Slow {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.clock.store(true, Ordering::SeqCst);
                self.nodes.read(d)
            }
        }
        let mut slow = Slow { nodes, clock };
        let error = sc::fragment::read(
            &mut slow,
            &root,
            &[],
            sc::fragment::Unit::Value,
            0,
            1,
            4096,
            &mut owner,
        )
        .unwrap_err();
        assert_eq!(error.code(), sc::job::DEADLINE);
        assert_eq!(owner.snapshot().nodes, 0);
    }
}
