//! #1394 C §5.5/5.6.1：内存中的 Watch 客户租约与有界积压。
//!
//! 这是服务层调用的内存状态，不写 SQLite、不创建第二份 Delta，也不推进持久
//! delivery floor。调用方必须先核验 cursor 的身份、非负代号和同 cut 关系；本层
//! 仍拒绝明显违反这些不变量的负值或 after > head。

use super::business_json::{self as bj, BusinessMap, TextValue};
use super::q_resources::{Limit, QResources};
use newchan_rust::s_content::capture_allocation::OwnedBudget;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

struct Lease {
    identity: (TextValue, TextValue),
    delivered: i64,
    initial_head: i64,
    observed_head: i64,
    overflow: bool,
    expires: Instant,
}

/// 只保留客户租约和有界积压计数；资源值与摘要在构造时冻结。
pub(super) struct ClientLeases {
    max_leases: u64,
    lease_duration: Duration,
    max_pending: u64,
    resource_digest: TextValue,
    allocation: Option<OwnedBudget>,
    clients: Mutex<BusinessMap<TextValue, Lease>>,
}

impl ClientLeases {
    pub(super) fn new(resources: &QResources) -> Self {
        Self::with_allocation(resources, None).expect("None lease同核")
    }
    pub(super) fn with_allocation(
        resources: &QResources,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        if allocation.is_some() {
            return Err("prototype_audit_allocation_incomplete：owned租约未启用".into());
        }
        Ok(Self {
            max_leases: resources.limit(Limit::ClientLeases),
            lease_duration: Duration::from_millis(resources.limit(Limit::ClientLeaseMs)),
            max_pending: resources.limit(Limit::ClientPendingBatches),
            resource_digest: TextValue::from_str(resources.digest(), allocation)?,
            allocation: allocation.cloned(),
            clients: Mutex::new(bj::new_map(allocation)),
        })
    }
    pub(super) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.allocation.as_ref()
    }
    pub(super) fn resource_digest(&self) -> &str {
        &self.resource_digest
    }

    pub(super) fn observe(
        &self,
        client: &str,
        identity: (&str, &str),
        after: i64,
        head: i64,
        now: Instant,
    ) -> Result<bool, String> {
        validate_cursor(after, head)?;
        let expires = now
            .checked_add(self.lease_duration)
            .ok_or_else(|| "ClientLeaseInvalid：租约期限溢出单调时钟域".to_string())?;

        let mut clients = self
            .clients
            .lock()
            .map_err(|_| "ClientLeaseInvalid：客户租约锁已中毒".to_string())?;

        // Python 的原合同是 expires < now；恰好到期瞬间仍然有效。
        clients.retain(|_, lease| lease.expires >= now);
        let identity_owned = (
            TextValue::from_str(identity.0, self.allocation.as_ref())?,
            TextValue::from_str(identity.1, self.allocation.as_ref())?,
        );

        // 每次观察先把同一 session identity 的所有客户推进到本次 head，
        // sticky overflow 只会在这里置位，不会因 delivered 自动清除。
        for (_, lease) in clients.iter_mut() {
            if lease.identity == identity_owned {
                lease.observed_head = lease.observed_head.max(head);
                if pending(lease) > self.max_pending {
                    lease.overflow = true;
                }
            }
        }

        if !clients.contains_key(client) {
            if clients.len() as u64 >= self.max_leases {
                return Err("Incomplete：客户租约达到事前上限".into());
            }
            clients
                .insert(
                    TextValue::from_str(client, self.allocation.as_ref())?,
                    Lease {
                        identity: identity_owned,
                        delivered: after,
                        // 新客户不因历史 head 立即形成积压。
                        initial_head: head,
                        observed_head: head,
                        overflow: false,
                        expires,
                    },
                )
                .map_err(|e| e.to_string())?;
        } else {
            let lease = clients.get_mut(client).expect("租约必须存在");
            if lease.identity != identity_owned {
                // 同名客户换 identity 是替换，不占用新的租约名额。
                *lease = Lease {
                    identity: identity_owned,
                    delivered: after,
                    initial_head: head,
                    observed_head: head,
                    overflow: false,
                    expires,
                };
            } else {
                if after >= lease.observed_head {
                    // 已完成的完整快照重建：允许清除 sticky overflow 并重置基线。
                    lease.delivered = after;
                    lease.initial_head = head;
                    lease.overflow = false;
                }
                lease.expires = expires;
            }
        }
        let lease = clients.get(client).expect("租约必须存在");
        Ok(lease.overflow)
    }

    pub(super) fn delivered(
        &self,
        client: &str,
        identity: (&str, &str),
        after: i64,
    ) -> Result<(), String> {
        if after < 0 {
            return Err("InvalidDomain：delivered after_generation 必须非负".into());
        }
        let identity = (identity.0, identity.1);
        let mut clients = self
            .clients
            .lock()
            .map_err(|_| "ClientLeaseInvalid：客户租约锁已中毒".to_string())?;
        if let Some(lease) = clients.get_mut(client) {
            if lease.identity.0.as_str() == identity.0 && lease.identity.1.as_str() == identity.1 {
                // 成功发送后只单调推进；不创建、不续租，也不清 sticky overflow。
                lease.delivered = lease.delivered.max(after);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn delivered_generation(&self, client: &str) -> Option<i64> {
        self.clients
            .lock()
            .expect("测试读取租约时锁不应中毒")
            .get(client)
            .map(|lease| lease.delivered)
    }
}

fn validate_cursor(after: i64, head: i64) -> Result<(), String> {
    if after < 0 || head < 0 || after > head {
        Err("InvalidDomain：Watch cursor 代号必须满足 0<=after<=head".into())
    } else {
        Ok(())
    }
}

fn pending(lease: &Lease) -> u64 {
    let baseline = lease.delivered.max(lease.initial_head);
    if lease.observed_head <= baseline {
        0
    } else {
        (lease.observed_head - baseline) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn with_limits(leases: u64, lease_ms: u64, pending: u64) -> QResources {
        let mut input = super::super::q_resources::tests::functional_input();
        input["limits"]["max_client_leases"] = json!(leases.to_string());
        input["limits"]["client_lease_ms"] = json!(lease_ms.to_string());
        input["limits"]["max_client_pending_batches"] = json!(pending.to_string());
        QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap()
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_lease_same_ttl_sticky_identity_delivered_and_quota_refusal_keep_prior_state() {
        use newchan_rust::s_content::capture_allocation::CaptureSlab;
        let r = with_limits(4, 100, 2);
        let normal = ClientLeases::new(&r);
        let budget = OwnedBudget::new(100000);
        let owned = ClientLeases::with_allocation(&r, Some(&budget)).unwrap();
        let t = Instant::now();
        for (client, identity, after, head) in [
            ("a", ("s", "1"), 0, 0),
            ("b", ("s", "1"), 0, 2),
            ("a", ("s", "1"), 0, 3),
        ] {
            assert_eq!(
                owned.observe(client, identity, after, head, t).unwrap(),
                normal.observe(client, identity, after, head, t).unwrap()
            );
        }
        owned.delivered("a", ("s", "1"), 1).unwrap();
        normal.delivered("a", ("s", "1"), 1).unwrap();
        assert_eq!(
            owned.observe("a", ("s", "1"), 1, 3, t).unwrap(),
            normal.observe("a", ("s", "1"), 1, 3, t).unwrap()
        );
        let live = budget.snapshot().live_requested_layout_bytes;
        let fill = CaptureSlab::new(0, budget.snapshot().limit_bytes - live, &budget).unwrap();
        assert!(owned.observe("new", ("s", "1"), 0, 3, t).is_err());
        assert_eq!(owned.delivered_generation("a"), Some(1));
        assert_eq!(owned.delivered_generation("new"), None);
        drop(fill);
        assert_eq!(
            owned.observe("a", ("s", "1"), 3, 3, t).unwrap(),
            normal.observe("a", ("s", "1"), 3, 3, t).unwrap()
        );
        assert_eq!(
            owned.delivered_generation("a"),
            normal.delivered_generation("a")
        );
        assert_eq!(
            owned
                .observe("a", ("s", "2"), 0, 0, t + Duration::from_millis(100))
                .unwrap(),
            normal
                .observe("a", ("s", "2"), 0, 0, t + Duration::from_millis(100))
                .unwrap()
        );
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn new_clients_do_not_inherit_old_history_and_same_identity_overflow_is_sticky_then_reset() {
        let r = with_limits(4, 100, 2);
        let leases = ClientLeases::new(&r);
        assert_eq!(leases.resource_digest(), r.digest());
        let t = Instant::now();
        assert!(!leases.observe("a", ("s", "1"), 0, 0, t).unwrap());
        assert!(!leases.observe("b", ("s", "1"), 0, 2, t).unwrap());
        assert!(leases.observe("a", ("s", "1"), 0, 3, t).unwrap());
        leases.delivered("a", ("s", "1"), 1).unwrap();
        assert!(leases.observe("a", ("s", "1"), 1, 3, t).unwrap());
        assert!(!leases.observe("a", ("s", "1"), 3, 3, t).unwrap());
        assert_eq!(leases.delivered_generation("a"), Some(3));
    }

    #[test]
    fn expiry_boundary_capacity_and_identity_replacement_match_python_contract() {
        let r = with_limits(1, 1, 10);
        let leases = ClientLeases::new(&r);
        let t = Instant::now();
        assert!(!leases.observe("a", ("s", "1"), 0, 0, t).unwrap());
        // expires == now remains valid; a strictly later instant expires it.
        assert!(!leases
            .observe("a", ("s", "1"), 0, 0, t + Duration::from_millis(1))
            .unwrap());
        assert!(leases
            .observe("b", ("s", "1"), 0, 0, t + Duration::from_millis(1))
            .is_err());
        assert!(!leases
            .observe("a", ("s", "2"), 0, 0, t + Duration::from_millis(1))
            .unwrap());
        assert!(leases
            .observe("b", ("s", "2"), 0, 0, t + Duration::from_millis(1))
            .is_err());
    }

    #[test]
    fn delivered_is_monotonic_identity_scoped_and_does_not_renew_or_create() {
        let r = with_limits(2, 1, 10);
        let leases = ClientLeases::new(&r);
        let t = Instant::now();
        leases.delivered("missing", ("s", "1"), 7).unwrap();
        assert_eq!(leases.delivered_generation("missing"), None);
        leases.observe("a", ("s", "1"), 0, 2, t).unwrap();
        leases.delivered("a", ("other", "1"), 2).unwrap();
        leases.delivered("a", ("s", "1"), 5).unwrap();
        leases.delivered("a", ("s", "1"), 1).unwrap();
        assert_eq!(leases.delivered_generation("a"), Some(5));
        // 旧回复晚到且高于当前 observed_head 时，Python 合同的负 pending 算零，
        // 不得伪造 overflow。
        assert!(!leases.observe("a", ("s", "1"), 0, 3, t).unwrap());
        // delivered does not renew; the lease is gone after a strictly later instant.
        leases
            .observe(
                "a",
                ("s", "1"),
                0,
                3,
                t + Duration::from_millis(1) + Duration::from_nanos(1),
            )
            .unwrap();
        assert_eq!(leases.delivered_generation("a"), Some(0));
    }

    #[test]
    fn concurrent_observe_and_delivered_are_short_locked_and_monotonic() {
        use std::sync::Arc;
        use std::thread;

        let r = with_limits(32, 100, 100);
        let leases = Arc::new(ClientLeases::new(&r));
        let t = Instant::now();
        leases.observe("a", ("s", "1"), 0, 0, t).unwrap();

        let mut handles = Vec::new();
        for i in 1..=16_i64 {
            let leases = Arc::clone(&leases);
            handles.push(thread::spawn(move || {
                let client = format!("client-{i}");
                leases.observe(&client, ("s", "1"), 0, i, t).unwrap();
                leases.delivered("a", ("s", "1"), i).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(leases.delivered_generation("a"), Some(16));
    }

    #[test]
    fn invalid_cursor_and_expiry_overflow_are_rejected_without_panicking() {
        let r = with_limits(1, i64::MAX as u64, 1);
        let leases = ClientLeases::new(&r);
        let t = Instant::now();
        assert!(leases.observe("a", ("s", "1"), -1, 0, t).is_err());
        assert!(leases.observe("a", ("s", "1"), 2, 1, t).is_err());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            leases.observe("a", ("s", "1"), 0, 0, t)
        }));
        assert!(result.is_ok(), "期限溢出不得panic");
        if let Ok(Err(error)) = result {
            assert!(error.contains("溢出"), "{error}");
        }
    }
}
