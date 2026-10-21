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
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(super) fn owner_life(self) -> u64 {
        self.domain.life
    }
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
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

/// #1488 受控同owner Recover迁移的换代证明：owner 在本 Recover 事务内读到的持久
/// writer_epoch（from）与目标（to）。只有 to 严格大于 from 才能构造；不含成功标记。
/// 这是 owner.recover 唯一的持久 epoch 严格上升门（原 next<=from 门并入此处，文案与位置不变）。
/// #1488 0034 P3-1：Native 下另绑本事务所见候选父账身份与冻结预算摘要，预算不隐式携带。
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code))]
pub(super) struct RecoverHandoff {
    from: i64,
    to: i64,
    #[cfg(feature = "s_capture_native_testonly")]
    parent: Option<super::audit_owner::native::BudgetStamp>,
}
impl RecoverHandoff {
    pub(super) fn new(from: i64, to: i64) -> Result<Self, String> {
        if to <= from {
            return Err("StaleWriter：recover换代必须严格上升".into());
        }
        Ok(Self {
            from,
            to,
            #[cfg(feature = "s_capture_native_testonly")]
            parent: None,
        })
    }
    /// 绑定本 Recover 事务实际持有的候选父账与冻结域；迁移时逐项比对，缺或不符即拒绝。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn bind_parent(mut self, parent: super::audit_owner::native::BudgetStamp) -> Self {
        self.parent = Some(parent);
        self
    }
    /// 负控注入：交接摘要改写为他账或同账下一冻结域。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn parent_for_test(&mut self) -> &mut Option<super::audit_owner::native::BudgetStamp> {
        &mut self.parent
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
/// 写前签发后按值离开 complete 借用；字段私有，不含 caller raw 或成功标记。
#[cfg(feature = "s_capture_native_testonly")]
pub(super) struct IssuedRawAdvance {
    previous_prefix: usize,
    current_prefix: usize,
    previous: RawAdvanceBinding,
    current: RawAdvanceBinding,
    winner_edits: sc::CodecVec<RawWinnerEdit>,
    lease_serial: u64,
    seal: SnapshotSeal,
}
#[cfg(feature = "s_capture_native_testonly")]
impl IssuedRawAdvance {
    pub(super) fn bind<'raw>(
        self,
        raw: RawEventsView<'raw>,
        lease: &'raw Lease,
    ) -> Result<BoundRawAdvance<'raw>, String> {
        if raw.len() != self.current_prefix {
            return Err(stale("写前签发材料与同一pending原件长度不符"));
        }
        let token = BoundRawAdvance {
            raw,
            previous_prefix: self.previous_prefix,
            previous: self.previous,
            current: self.current,
            winner_edits: self.winner_edits,
            lease,
            lease_serial: self.lease_serial,
            seal: self.seal,
        };
        token.ensure_current()?;
        Ok(token)
    }
    /// #1488 Recover 锁：把 token 的租约序号改记为 owner 当前序号，模拟 Recover 前当期签发的旧 token。
    #[cfg(test)]
    pub(super) fn restamp_for_test(mut self, lease: &Lease) -> Self {
        self.lease_serial = lease.serial.get();
        self
    }
}
impl<'raw> BoundRawAdvance<'raw> {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn into_issued(self) -> IssuedRawAdvance {
        IssuedRawAdvance {
            previous_prefix: self.previous_prefix,
            current_prefix: self.raw.len(),
            previous: self.previous,
            current: self.current,
            winner_edits: self.winner_edits,
            lease_serial: self.lease_serial,
            seal: self.seal,
        }
    }
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
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
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
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(super) seal: SnapshotSeal,
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    binding: RawAdvanceBinding,
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(super) facts: RawPrefixFacts,
    // 资格所属持久 writer_epoch；只由已提交的受控 Recover 迁移盖章，下一次 Recover 须从此 epoch 出发。
    #[cfg(feature = "s_capture_native_testonly")]
    writer_epoch: Option<i64>,
    #[cfg(feature = "s_capture_native_testonly")]
    set: Option<super::construction_set::CommittedSet>,
    #[cfg(feature = "s_capture_native_testonly")]
    retained: usize,
    #[cfg(feature = "s_capture_native_testonly")]
    source: Option<super::super::source_audit::NativeSourceState>,
    #[cfg(feature = "s_capture_native_testonly")]
    source_bytes: usize,
    // #1488 v4（反例D）：状态块自身的真实许可随状态move，最后析构。
    #[cfg(feature = "s_capture_native_testonly")]
    fee: Option<super::audit_owner::native::Permit>,
    // #1488 0037 P2-1：提交本资格的事务候选父账身份与冻结域摘要（提交前盖章、COMMIT 后随资格晋升）；
    // Recover 迁移以此独立记录值比对事务实际父账，不取自比对对象本身。
    #[cfg(feature = "s_capture_native_testonly")]
    parent: Option<super::audit_owner::native::BudgetStamp>,
}
impl CommittedRawState {
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn source_work(&self) -> Option<super::super::source_audit::SourceWork> {
        self.source.as_ref().map(|s| s.work)
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn source_counts(&self) -> Option<super::super::source_audit::Counts> {
        self.source.as_ref().map(|s| s.counts_for_test())
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn binding(&self) -> RawAdvanceBinding {
        self.binding
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn has_set(&self) -> bool {
        self.set.is_some()
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn writer_epoch(&self) -> Option<i64> {
        self.writer_epoch
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn has_fee(&self) -> bool {
        self.fee.is_some()
    }
    /// 状态块许可所挂父账（与 mark 同一 Ledger 时返回许可字节）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn fee_on_ledger_for_test(
        &self,
        mark: &super::audit_owner::native::LedgerMark,
    ) -> Option<usize> {
        self.fee.as_ref().and_then(|fee| fee.on_ledger_for_test(mark))
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn set_writer_epoch_for_test(&mut self, epoch: Option<i64>) {
        self.writer_epoch = epoch;
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn has_parent(&self) -> bool {
        self.parent.is_some()
    }
    /// 负控注入：抹掉资格所记父账（只为咬住“缺记录”门）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn clear_parent_for_test(&mut self) {
        self.parent = None;
    }
    /// 负控注入：资格所记父账改指 stamp（他账或旧域；只为咬住 advance 的身份链核）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn set_parent_for_test(&mut self, stamp: super::audit_owner::native::BudgetStamp) {
        self.parent = Some(stamp);
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn parent_matches_for_test(&self, budget: &super::audit_owner::native::Budget) -> bool {
        self.parent.as_ref().is_some_and(|recorded| recorded.matches(budget))
    }
    /// #1488 0041（主控裁定 (a)）：只由 audit_admission 的合法重绑点调用——完整候选经
    /// bind_complete_budget（已校同父、本请求冻结域）从 from 交接到 to 时，资格所记冻结域随之前进，
    /// 父账 id 不变。所记值须恰为 from 的同账同域，否则原值不动（Recover 照常按记录拒绝）。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn advance_parent(
        &mut self,
        from: &super::audit_owner::native::Budget,
        to: &super::audit_owner::native::Budget,
    ) {
        if let Some(next) = self.parent.as_ref().and_then(|recorded| recorded.advance(from, to)) {
            self.parent = Some(next);
        }
    }
    /// 负控注入：状态块许可换成他账许可（只为咬住 holds 门）；返回原许可。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn replace_fee_for_test(
        &mut self,
        fee: super::audit_owner::native::Permit,
    ) -> Option<super::audit_owner::native::Permit> {
        self.fee.replace(fee)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained
            .saturating_add(std::mem::size_of::<Self>())
            .saturating_add(self.source_bytes)
    }
}
pub(super) struct PendingRawState {
    pub(super) seal: SnapshotSeal,
    binding: RawAdvanceBinding,
    pub(super) facts: RawPrefixFacts,
    #[cfg(feature = "s_capture_native_testonly")]
    set: Option<super::construction_set::CommittedSet>,
    #[cfg(feature = "s_capture_native_testonly")]
    retained: usize,
    #[cfg(feature = "s_capture_native_testonly")]
    source: Option<super::super::source_audit::NativeSourceState>,
    #[cfg(feature = "s_capture_native_testonly")]
    source_bytes: usize,
    #[cfg(feature = "s_capture_native_testonly")]
    raw_token_bytes: usize,
    closed_total_changes: Option<u64>,
    #[cfg(feature = "s_capture_native_testonly")]
    writer_epoch: Option<i64>,
    // 受控 Recover 迁移：换代证明及“effects 闭合后持久 epoch 已确认等于 to”。
    recover: Option<(RecoverHandoff, bool)>,
    // 最后析构：来源/原始事实先释放，再归还PendingSet持有的原父账许可。
    #[cfg(feature = "s_capture_native_testonly")]
    pending_set: Option<super::construction_set::PendingSet>,
    // #1488 v4（反例D）：状态块许可由PendingRawState持有至自身析构/晋升，不在签发闭包出口提前归还。
    #[cfg(feature = "s_capture_native_testonly")]
    fee: Option<super::audit_owner::native::Permit>,
    // #1488 0037 P2-1：本事务候选父账与冻结域摘要；由 owner 在构造点盖章，晋升时成为资格记录值。
    #[cfg(feature = "s_capture_native_testonly")]
    parent: Option<super::audit_owner::native::BudgetStamp>,
}
impl PendingRawState {
    /// 附上状态块自身许可；旧状态带来的许可（旧origin）在替换后归还，只降累计占用。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn attach_fee(&mut self, fee: super::audit_owner::native::Permit) {
        drop(self.fee.replace(fee));
    }
    /// #1488 0037 P2-1：盖本事务候选父账身份与当前冻结域摘要；读不到当前域则不盖，
    /// 晋升后的资格无记录值，Recover 迁移按缺记录拒绝。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn stamp_parent(&mut self, parent: &super::audit_owner::native::Budget) {
        self.parent = parent.handoff_stamp().ok();
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn has_fee(&self) -> bool {
        self.fee.is_some()
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn has_pending_set(&self) -> bool {
        self.pending_set.is_some()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained
            .saturating_add(std::mem::size_of::<Self>())
            .saturating_add(self.source_bytes)
            .saturating_add(self.raw_token_bytes)
            .saturating_add(self.pending_set.as_ref().map_or(0, |s| s.retained_bytes()))
    }
    /// 只有父 owner 从本快照已核原件调用；previous 必须 move，失败时不能还原作成功缓存。
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(super) fn advance<'raw>(
        #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(unused_mut, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
        mut previous: Option<CommittedRawState>,
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
        #[cfg(feature = "s_capture_native_testonly")]
        let set = previous.as_mut().and_then(|previous| previous.set.take());
        #[cfg(feature = "s_capture_native_testonly")]
        let source = previous
            .as_mut()
            .and_then(|previous| previous.source.take());
        #[cfg(feature = "s_capture_native_testonly")]
        let source_bytes = previous
            .as_ref()
            .map_or(0, |previous| previous.source_bytes);
        #[cfg(feature = "s_capture_native_testonly")]
        let fee = previous.as_mut().and_then(|previous| previous.fee.take());
        #[cfg(feature = "s_capture_native_testonly")]
        let writer_epoch = previous.as_ref().and_then(|previous| previous.writer_epoch);
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
        #[cfg(feature = "s_capture_native_testonly")]
        let raw_token_bytes = super::super::audit_retention::vec_bytes::<RawWinnerEdit>(
            token.winner_edits.capacity(),
        );
        let pending = Self {
            seal,
            binding: current,
            facts,
            #[cfg(feature = "s_capture_native_testonly")]
            retained,
            #[cfg(feature = "s_capture_native_testonly")]
            source,
            #[cfg(feature = "s_capture_native_testonly")]
            source_bytes,
            closed_total_changes: None,
            #[cfg(feature = "s_capture_native_testonly")]
            writer_epoch,
            recover: None,
            #[cfg(feature = "s_capture_native_testonly")]
            raw_token_bytes,
            #[cfg(feature = "s_capture_native_testonly")]
            set,
            #[cfg(feature = "s_capture_native_testonly")]
            pending_set: None,
            #[cfg(feature = "s_capture_native_testonly")]
            fee,
            #[cfg(feature = "s_capture_native_testonly")]
            parent: None,
        };
        Ok((pending, token))
    }

    /// Accept/Begin 只携带最后发布游标。资格由同一owner原件和后续真实effects闭合，
    /// 此处不读新raw赢家、不推进binding，不为producer制造遗漏的一步。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn carry(
        previous: Option<CommittedRawState>,
        seal: SnapshotSeal,
        raw_len: usize,
        lease: &Lease,
    ) -> Result<Self, String> {
        if lease.stale.get() {
            return Err(stale("Carry租约已撤销"));
        }
        let writer_epoch = previous.as_ref().and_then(|previous| previous.writer_epoch);
        let (binding, facts, set, retained, source, source_bytes, fee) =
            if let Some(previous) = previous {
                if !previous.seal.same_snapshot(&seal) || raw_len < previous.facts.prefix_len() {
                    return Err(stale("Carry前沿/输入身份或实际快照不连续"));
                }
                (
                    previous.binding,
                    previous.facts,
                    previous.set,
                    previous.retained,
                    previous.source,
                    previous.source_bytes,
                    previous.fee,
                )
            } else {
                (
                    RawAdvanceBinding {
                        domain: RawDomain {
                            life: seal.life,
                            epoch: lease.serial.get(),
                        },
                        step: 0,
                    },
                    RawPrefixFacts::default(),
                    None,
                    0,
                    None,
                    0,
                    None,
                )
            };
        Ok(Self {
            seal,
            binding,
            facts,
            set,
            pending_set: None,
            retained,
            source,
            source_bytes,
            raw_token_bytes: 0,
            closed_total_changes: None,
            writer_epoch,
            recover: None,
            fee,
            parent: None,
        })
    }

    /// #1488 受控同owner Recover迁移：只接本owner未撤销、同快照的已提交资格，且资格戳
    /// （若有）必须等于本事务读到的持久 from。原件游标、集合、来源与状态块许可原样 move，
    /// 不换账、不重算、不推进binding；之后须 Recover effects 闭合、确认持久 epoch=to、
    /// 真实 COMMIT 成功才晋升，任一失败由 owner 撤销租约。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn recover(
        previous: Option<CommittedRawState>,
        seal: SnapshotSeal,
        raw_len: usize,
        lease: &Lease,
        handoff: RecoverHandoff,
        parent: &super::audit_owner::native::Budget,
    ) -> Result<Self, String> {
        let Some(previous) = previous else {
            return Err(stale("Recover迁移缺原有效资格"));
        };
        if previous.writer_epoch.is_some_and(|epoch| epoch != handoff.from) {
            return Err(stale("Recover迁移旧资格不属于当前持久epoch"));
        }
        // P3-1：换代证明所绑父账/冻结域须恰为事务实际候选父账；状态块许可须仍挂该账。
        let Some(stamp) = handoff.parent.as_ref() else {
            return Err(stale("Recover迁移缺父账交接摘要"));
        };
        if !stamp.matches(parent) {
            return Err(stale("Recover迁移父账或冻结域与交接摘要不符"));
        }
        // 0037 P2-1：资格提交时独立记下的父账身份/冻结域须恰为事务实际父账（fee=None 也核）。
        // 0041 P3-3：缺记录（盖章时读不到当前域）与记录不符分开报。
        let Some(recorded) = previous.parent.as_ref() else {
            return Err(stale("Recover迁移资格缺父账记录"));
        };
        if !recorded.matches(parent) {
            return Err(stale("Recover迁移资格所记父账或冻结域与事务父账不符"));
        }
        if previous.fee.as_ref().is_some_and(|fee| !stamp.holds(fee)) {
            return Err(stale("Recover迁移状态块许可不挂事务父账"));
        }
        let mut pending = Self::carry(Some(previous), seal, raw_len, lease)?;
        pending.parent = handoff.parent.clone();
        pending.recover = Some((handoff, false));
        Ok(pending)
    }

    /// Recover effects 闭合后由 owner 以已核 Q 的最后换代结果确认持久 epoch 恰为 to；
    /// 非 Recover 迁移状态不受影响。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn confirm_recover(&mut self, persisted: Option<i64>) -> Result<(), String> {
        match &mut self.recover {
            None => Ok(()),
            Some((handoff, confirmed))
                if !*confirmed
                    && self.closed_total_changes.is_some()
                    && persisted == Some(handoff.to) =>
            {
                *confirmed = true;
                Ok(())
            }
            Some(_) => Err(stale("Recover迁移effects未闭合或持久epoch与换代证明不符")),
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn take_source(&mut self) -> Option<super::super::source_audit::NativeSourceState> {
        // 保留原信用占帽，源状态在PublishPhase唯一局部槽暂存时不洗掉旧费用。
        self.source.take()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn install_source(
        &mut self,
        source: super::super::source_audit::NativeSourceState,
    ) -> Result<usize, String> {
        if self.source.is_some() {
            return Err(stale("来源状态重复回交owner"));
        }
        let bytes = source.retained_bytes();
        if bytes == usize::MAX {
            return Err(stale("来源状态保留账溢出"));
        }
        let growth = bytes.saturating_sub(self.source_bytes);
        self.source_bytes = bytes;
        self.source = Some(source);
        Ok(growth)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn set_seed(
        &mut self,
        previous: Option<super::construction_set::Declaration>,
    ) -> Result<super::construction_set::Seed, String> {
        super::construction_set::Seed::new(self.binding, self.seal, previous, self.set.take())
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn close_set(
        &mut self,
        mut set: super::construction_set::PendingSet,
        actual_root: sc::RefView<'_>,
    ) -> Result<(), String> {
        if self.pending_set.is_some() {
            return Err(stale("重复闭合集合effects"));
        }
        set.close_against_persisted(actual_root, self.binding)?;
        self.pending_set = Some(set);
        Ok(())
    }

    /// 此方法仅对父 owner 开放；真实 effects 覆盖与业务判据闭合后调用，尚未晋升。
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
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
        if self.recover.as_ref().is_some_and(|(_, confirmed)| !confirmed) {
            return Err(stale("Recover迁移未确认持久epoch，不得晋升"));
        }
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(set) = &self.pending_set {
            if set.requires_source() && self.source.is_none() {
                return Err(stale("来源成功结果未回交同owner状态"));
            }
            set.check_commit(seal, total_changes)?;
        }
        Ok(())
    }
    /// 仅在 SQLite COMMIT 返回成功且 owner 确认 autocommit 后 move。
    pub(super) fn committed(self) -> CommittedRawState {
        CommittedRawState {
            seal: self.seal,
            binding: self.binding,
            facts: self.facts,
            // 已确认的 Recover 迁移晋升到新 epoch 资格；其余操作沿用原戳。
            #[cfg(feature = "s_capture_native_testonly")]
            writer_epoch: self
                .recover
                .map_or(self.writer_epoch, |(handoff, _)| Some(handoff.to)),
            #[cfg(feature = "s_capture_native_testonly")]
            set: self.pending_set.map(|set| set.committed()).or(self.set),
            #[cfg(feature = "s_capture_native_testonly")]
            retained: self.retained,
            #[cfg(feature = "s_capture_native_testonly")]
            source: self.source,
            #[cfg(feature = "s_capture_native_testonly")]
            source_bytes: self.source_bytes,
            #[cfg(feature = "s_capture_native_testonly")]
            fee: self.fee,
            #[cfg(feature = "s_capture_native_testonly")]
            parent: self.parent,
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
