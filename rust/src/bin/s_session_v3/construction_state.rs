//! #1488 / #1481：owner 私有的 raw 变化资格与 pending move 容器。
//! 本模块不签发 state/object/witness/observation 完整变化证明；那些集合仍由独立来源核验。
//! 原件绑定来自父 owner，不能由生成器传入 prefix/verified=true 自签。

use super::super::business_json::{self as bj, RawEventsView, RawPrefixFacts, RawWinnerEdit};
use super::SnapshotSeal;
use newchan_rust::s_content as sc;
use std::cell::Cell;
use std::ops::Range;

pub(super) fn stale(detail: &str) -> String {
    format!("StaleWriter：construction资格失效：{detail}")
}

/// owner life 对应一条实际连接生涯；epoch 记录初次租约，失效后只能重建 owner。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawDomain {
    life: u64,
    epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawAdvanceBinding {
    domain: RawDomain,
    step: u64,
}
impl RawAdvanceBinding {
    pub(crate) fn domain(self) -> RawDomain {
        self.domain
    }
    pub(crate) fn is_initial(self) -> bool {
        self.step == 0
    }
}
pub(super) struct Lease {
    pub(super) serial: Cell<u64>,
    pub(super) stale: Cell<bool>,
}
impl Default for Lease {
    fn default() -> Self {
        Self {
            serial: Cell::new(1),
            stale: Cell::new(false),
        }
    }
}
impl Lease {
    pub(super) fn invalidate(&self) {
        self.stale.set(true);
        self.serial.set(self.serial.get().saturating_add(1));
    }
    pub(super) fn close(&self) -> Result<(), String> {
        let next = self
            .serial
            .get()
            .checked_add(1)
            .ok_or_else(|| stale("材料租约序号耗尽"))?;
        self.serial.set(next);
        Ok(())
    }
}

/// 只借用当前 raw；不 Clone、不序列化，不可跨事务复用。字段与生产构造均不对消费方开放。
pub(crate) struct BoundRawAdvance<'raw> {
    raw: RawEventsView<'raw>,
    previous_prefix: usize,
    previous: RawAdvanceBinding,
    current: RawAdvanceBinding,
    winner_edits: sc::CodecVec<RawWinnerEdit>,
    lease: &'raw Lease,
    lease_serial: u64,
    // 身份/版本在真实 main 快照建立后取值，不在 COMMIT 后重新采样。
    seal: SnapshotSeal,
}
impl<'raw> BoundRawAdvance<'raw> {
    pub(crate) fn raw(&self) -> RawEventsView<'raw> {
        self.raw
    }
    pub(crate) fn previous_prefix(&self) -> usize {
        self.previous_prefix
    }
    pub(crate) fn current_prefix(&self) -> usize {
        self.raw.len()
    }
    pub(crate) fn append_ordinals(&self) -> Range<usize> {
        self.previous_prefix..self.raw.len()
    }
    pub(crate) fn winner_edits(&self) -> &[RawWinnerEdit] {
        &self.winner_edits
    }
    pub(crate) fn domain(&self) -> RawDomain {
        self.current.domain
    }
    pub(crate) fn previous_binding(&self) -> RawAdvanceBinding {
        self.previous
    }
    pub(crate) fn current_binding(&self) -> RawAdvanceBinding {
        self.current
    }
    /// 消费者只能撤销既有租约，不能重签、换域或恢复；owner 回调 Err 仍同步撤销。
    pub(crate) fn invalidate_after_consumer_failure(&self) {
        self.lease.invalidate();
    }
    pub(crate) fn ensure_current(&self) -> Result<(), String> {
        if self.lease.stale.get() || self.lease.serial.get() != self.lease_serial {
            return Err(stale("raw token 已结束、回滚或撤销"));
        }
        if self.seal.life != self.current.domain.life || self.previous.domain != self.current.domain
        {
            return Err(stale("raw token 的连接域不符"));
        }
        Ok(())
    }
}
pub(super) struct CommittedRawState {
    pub(super) seal: SnapshotSeal,
    binding: RawAdvanceBinding,
    pub(super) facts: RawPrefixFacts,
    #[cfg(feature = "s_capture_native_testonly")]
    retained: usize,
}
impl CommittedRawState {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained.saturating_add(std::mem::size_of::<Self>())
    }
}
pub(super) struct PendingRawState {
    pub(super) seal: SnapshotSeal,
    binding: RawAdvanceBinding,
    pub(super) facts: RawPrefixFacts,
    #[cfg(feature = "s_capture_native_testonly")]
    retained: usize,
    closed_total_changes: Option<u64>,
}
impl PendingRawState {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained.saturating_add(std::mem::size_of::<Self>())
    }
    /// 只有父 owner 从本快照已核原件调用；previous 必须 move，失败时不能还原作成功缓存。
    pub(super) fn advance<'raw>(
        previous: Option<CommittedRawState>,
        raw: RawEventsView<'raw>,
        seal: SnapshotSeal,
        bound: &super::BoundOriginals,
        lease: &'raw Lease,
        admission: &(impl super::super::audit_store::CodecRead + ?Sized),
    ) -> Result<(Self, BoundRawAdvance<'raw>), String> {
        if lease.stale.get() {
            return Err(stale("旧夹持已失效；必须显式重建 owner"));
        }
        if raw.budget().is_some() {
            return Err(stale(
                "Owned RawPrefixIndex 尚无同账实现；禁止 Plain fallback",
            ));
        }
        #[cfg(feature = "s_capture_native_testonly")]
        let retained = previous.as_ref().map_or(0, |p| p.retained);
        let (previous_binding, previous_prefix, mut index) = match previous {
            Some(previous) => {
                if !previous.seal.same_snapshot(&seal) {
                    return Err(stale("连接、快照、读域或本连接写序变化"));
                }
                let prefix = previous.facts.prefix_len();
                let index = bj::RawPrefixIndex::restore(raw, previous.facts, bound)?;
                (previous.binding, prefix, index)
            }
            None => (
                RawAdvanceBinding {
                    domain: RawDomain {
                        life: seal.life,
                        epoch: lease.serial.get(),
                    },
                    step: 0,
                },
                0,
                bj::RawPrefixIndex::new(raw),
            ),
        };
        let changes = index.advance_admitted(raw.len(), admission)?;
        if changes.rewound || raw.len() < previous_prefix {
            return Err(stale("raw 前缀倒退"));
        }
        let current = RawAdvanceBinding {
            domain: previous_binding.domain,
            step: previous_binding
                .step
                .checked_add(1)
                .ok_or_else(|| stale("raw 步号耗尽"))?,
        };
        let token = BoundRawAdvance {
            raw,
            previous_prefix,
            previous: previous_binding,
            current,
            winner_edits: changes.winner_edits,
            lease,
            lease_serial: lease.serial.get(),
            seal,
        };
        #[cfg(feature = "s_capture_native_testonly")]
        let (facts, retained) = {
            let mut delta = super::super::audit_retention::RetainedDelta::default();
            let facts = index.into_facts_retained(&mut delta);
            let retained = delta
                .apply(retained)
                .map_err(|_| stale("raw保留账不闭合"))?;
            (facts, retained)
        };
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let facts = index.into_facts();
        let pending = Self {
            seal,
            binding: current,
            facts,
            #[cfg(feature = "s_capture_native_testonly")]
            retained,
            closed_total_changes: None,
        };
        Ok((pending, token))
    }

    /// 此方法仅对父 owner 开放；真实 effects 覆盖与业务判据闭合后调用，尚未晋升。
    pub(super) fn close_effects(
        &mut self,
        seal: SnapshotSeal,
        total_changes: u64,
    ) -> Result<(), String> {
        if self.closed_total_changes.is_some()
            || self.seal.life != seal.life
            || self.seal.transaction_id != seal.transaction_id
            || self.seal.data_version != seal.data_version
            || self.seal.schema_version != seal.schema_version
            || self.seal.domain != seal.domain
            || self.seal.write_sequence.checked_add(1) != Some(seal.write_sequence)
        {
            return Err(stale("效果闭合不属于同一次固定写"));
        }
        self.seal = seal;
        self.closed_total_changes = Some(total_changes);
        Ok(())
    }
    pub(super) fn check_commit(
        &self,
        seal: SnapshotSeal,
        total_changes: u64,
    ) -> Result<(), String> {
        if self.seal != seal || self.closed_total_changes != Some(total_changes) {
            return Err(stale("未闭合效果或闭合后再次写入"));
        }
        Ok(())
    }
    /// 仅在 SQLite COMMIT 返回成功且 owner 确认 autocommit 后 move。
    pub(super) fn committed(self) -> CommittedRawState {
        CommittedRawState {
            seal: self.seal,
            binding: self.binding,
            facts: self.facts,
            #[cfg(feature = "s_capture_native_testonly")]
            retained: self.retained,
        }
    }
}

/// 只用于 A/B 的输入与失效负控；生产构建没有此构造器。
#[cfg(test)]
pub(crate) struct RawAdvanceFixture {
    life: u64,
    lease: Lease,
    step: Cell<u64>,
}
#[cfg(test)]
impl RawAdvanceFixture {
    pub(crate) fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1 << 63);
        Self {
            life: NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
            lease: Lease::default(),
            step: Cell::new(0),
        }
    }
    pub(crate) fn bind<'raw>(
        &'raw self,
        raw: RawEventsView<'raw>,
        previous_prefix: usize,
        edits: &[RawWinnerEdit],
    ) -> Result<BoundRawAdvance<'raw>, String> {
        if previous_prefix > raw.len() || self.lease.stale.get() {
            return Err(stale("测试前缀/租约无效"));
        }
        self.lease.close()?;
        let previous = RawAdvanceBinding {
            domain: RawDomain {
                life: self.life,
                epoch: 1,
            },
            step: self.step.get(),
        };
        let current = RawAdvanceBinding {
            domain: previous.domain,
            step: previous
                .step
                .checked_add(1)
                .ok_or_else(|| stale("测试步号耗尽"))?,
        };
        self.step.set(current.step);
        let mut winner_edits = bj::new_work(raw.budget());
        for edit in edits {
            winner_edits.try_push(*edit).map_err(|e| e.to_string())?;
        }
        Ok(BoundRawAdvance {
            raw,
            previous_prefix,
            previous,
            current,
            winner_edits,
            lease: &self.lease,
            lease_serial: self.lease.serial.get(),
            seal: SnapshotSeal {
                life: self.life,
                transaction_id: current.step,
                write_sequence: current.step,
                data_version: 1,
                schema_version: 1,
                domain: [0, 1, 1, 0, 0, 1],
            },
        })
    }
    pub(crate) fn invalidate(&self) {
        self.lease.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::business_json::{RawPrefixIndex, RawWinner};
    use serde_json::json;

    #[test]
    fn owner_raw_fixture_domains_and_invalidation_are_not_prefix_lengths() {
        let raw = [json!({"identity_key":"a","revision":1,"source_coord":"0"})];
        let a = RawAdvanceFixture::new();
        let b = RawAdvanceFixture::new();
        let first = a.bind((&raw).into(), 0, &[]).unwrap();
        assert!(first.previous_binding().is_initial());
        let foreign = b.bind((&raw).into(), 0, &[]).unwrap();
        assert_ne!(first.domain(), foreign.domain());
        let second = a.bind((&raw).into(), 1, &[]).unwrap();
        assert_eq!(second.previous_binding(), first.current_binding());
        assert!(!second.previous_binding().is_initial());
        assert!(first.ensure_current().is_err());
        assert!(second.ensure_current().is_ok());
        second.invalidate_after_consumer_failure();
        assert!(second.ensure_current().is_err());
        a.invalidate();
        assert!(a.bind((&raw).into(), 1, &[]).is_err());
    }

    #[test]
    fn owner_raw_edits_are_final_identity_winners_and_allow_coordinate_swap() {
        let raw = [
            json!({"identity_key":"a","revision":1,"source_coord":"0"}),
            json!({"identity_key":"b","revision":1,"source_coord":"1"}),
            json!({"identity_key":"a","revision":2,"source_coord":"2"}),
            json!({"identity_key":"a","revision":3,"source_coord":"1"}),
            json!({"identity_key":"b","revision":2,"source_coord":"0"}),
        ];
        let mut index = RawPrefixIndex::new((&raw).into());
        assert_eq!(index.advance(2).unwrap().winner_edits.len(), 2);
        let changes = index.advance(5).unwrap();
        assert_eq!(
            &*changes.winner_edits,
            &[
                RawWinnerEdit {
                    old: Some(RawWinner {
                        coord: 0,
                        ordinal: 0
                    }),
                    new: Some(RawWinner {
                        coord: 1,
                        ordinal: 3
                    })
                },
                RawWinnerEdit {
                    old: Some(RawWinner {
                        coord: 1,
                        ordinal: 1
                    }),
                    new: Some(RawWinner {
                        coord: 0,
                        ordinal: 4
                    })
                },
            ]
        );
        assert_eq!(index.ordered().collect::<Vec<_>>(), vec![(0, 4), (1, 3)]);
        assert!(index.advance(5).unwrap().winner_edits.is_empty());
    }
}
