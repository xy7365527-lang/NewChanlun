//! #1394 C：对象集对拍的**共用纯函数**（旧 S2 域与新 S3 域同一份判据）。
//!
//! 旧 `compute_diff`（root `s_structure_session.rs`）里的五条身份判定在此收成一份，两个域各自
//! 只提供**键的读法**（`object_id` / `slot` / 不可变内容视图），不得再写第二套同义判据
//! （同一判断不得有宽严两档，#799）：
//!
//! 1. 活动旧身份按给定主序扫描：槽位无对应候选 = 移除；槽位有新但身份不同 = 被替代（撤 + 替换对）。
//! 2. 本次候选按给定序：同 ID 先核不可变内容（异内容即 `IdentityConflict`）→ 有活动同 ID 即
//!    「未变化」（槽位不一致即 `StorageUnavailable`）→ 无活动但有历史身份即拒绝复活 → 否则真实出生。
//!
//! **不可变内容按需读取**：`Content::read` 只在真正需要比较的那一刻调用（先既有、后候选，与旧实现
//! 同一求值次序），读失败原样上报。候选若是真实出生则**不读**它的不可变内容——旧实现就是如此；
//! 预先把内容算出来会把「没读过」变成「读失败」，那是改业务规则，不是收敛。
//!
//! 本函数固定返回**真实变化**（出生 / 撤回 / 替换）的索引。旧全域的 `LegacyFull` 完整活动集由旧
//! adapter 自己组装（既有对象沿用首次元数据），S3 按自己的发布形状落字。
//!
//! ## 边界如实
//!
//! - 撤回**理由文本**（`fact_removed` / `window_removed` / `superseded_by_revision`）与
//!   `superseded_by` 指针值属于各域数据，由 adapter 按 `Removal` 分类与新旧索引自行落字。
//! - `old` 里的 `object_id` 必须唯一（旧 adapter 的身份读集已在上游按此拒绝重复）。
//! - 重复身份、候选成员资格这类**各域自己的索引检查**不在这里：本函数只做上面五条身份判定，
//!   各域 adapter 自核（根已裁定：不为旧域规则往共用判据上加料）。
//! - 键在调用方构造：畸形候选（缺 `object_id`）在构造期即报错，而旧实现是在候选循环里才报；
//!   单缺陷输入的报错文本与顺序不变，同一次调用里「既畸形又冲突」的双缺陷输入报错先后可能不同。

#![allow(dead_code)] // S3 侧接线落在下游切片；本模块 API 现由 root adapter 与本文件测试使用。

use crate::v3::business_json as bj;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};
use std::borrow::Borrow;
use std::hash::Hash;

/// 不可变内容视图：只在真正需要比较时才读取。
///
/// 旧域实现是「对象 → 逐字段规范 JSON 文本」（读取可能失败）；S3 域是已就绪的内容根
/// （`NodeDigest`），用 [`Ready`]。
pub(crate) trait Content {
    /// 逐字段等价的不可变内容值。
    type Value: Eq;
    /// 读一次不可变内容；失败按调用方给的原文本上报，不吞、不换判据。
    fn read(&self) -> Result<Self::Value, String>;
}

/// 已就绪的不可变内容（S3 新域直接给内容根）。
pub(crate) struct Ready<T>(pub T);

impl<T: Eq + Clone> Content for Ready<T> {
    type Value = T;
    fn read(&self) -> Result<T, String> {
        Ok(self.0.clone())
    }
}

/// 同一身份键：对象 ID + 发生槽位 + 不可变内容视图。
pub(crate) struct Key<C: Content, Id = String, Slot = String> {
    pub object_id: Id,
    pub slot: Slot,
    pub immutable: C,
}

/// 既有持久身份：键 + 是否仍在活动集（活动身份构成撤回/替代扫描的主序）。
pub(crate) struct Existing<C: Content, Id = String, Slot = String> {
    pub key: Key<C, Id, Slot>,
    pub active: bool,
}

/// 撤回分类；理由文本由各域 adapter 落字。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Removal {
    /// 槽位在本轮消失（旧域 `window_removed` / 事实域 `fact_removed`）。
    Removed,
    /// 同槽位被新身份顶替（旧域 `superseded_by_revision`）。
    Superseded,
}

/// 一条撤回：旧索引 + 分类 + 替代目标（仅 `Superseded` 有）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Withdrawal {
    pub old_index: usize,
    pub removal: Removal,
    pub new_index: Option<usize>,
}

/// 一对同槽位替换：旧索引与新索引。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Replace {
    pub old_index: usize,
    pub new_index: usize,
}

/// 真实变化集：索引都指回调用方给的切片，顺序即调用方给的顺序。
#[derive(Debug)]
pub(crate) struct ChangeSet {
    /// 真实出生：`new` 索引，候选序。
    pub births: Vec<usize>,
    /// 撤回 / 被替代：`old` 索引（活动主序）。
    pub withdrawals: Vec<Withdrawal>,
    /// 同槽位替换对，与 `withdrawals` 同序。
    pub replaces: Vec<Replace>,
}

/// 纯对拍：不读库、不写库，只吃两份身份键与两侧不可变内容视图。
pub(crate) fn plan<Old, New>(new: &[Key<New>], old: &[Existing<Old>]) -> Result<ChangeSet, String>
where
    Old: Content,
    New: Content<Value = Old::Value>,
{
    let changes = plan_budgeted(new, old, None)?;
    Ok(ChangeSet {
        births: changes.births.try_into_plain().map_err(|e| e.to_string())?,
        withdrawals: changes
            .withdrawals
            .try_into_plain()
            .map_err(|e| e.to_string())?,
        replaces: changes
            .replaces
            .try_into_plain()
            .map_err(|e| e.to_string())?,
    })
}

/// 实际工作集所有者：普通与Owned入口都调用下方唯一判定核。
#[derive(Debug)]
pub(crate) struct BudgetedChangeSet {
    pub births: CodecVec<usize>,
    pub withdrawals: CodecVec<Withdrawal>,
    pub replaces: CodecVec<Replace>,
}

fn identity<Id: Borrow<str>>(id: &Id) -> &str {
    id.borrow()
}

/// 键可直接借已有文本/槽位，三个映射和结果向量直接使用调用方原账本。
pub(crate) fn plan_budgeted<Old, New, OldId, NewId, Slot>(
    new: &[Key<New, NewId, Slot>],
    old: &[Existing<Old, OldId, Slot>],
    allocation: Option<&OwnedBudget>,
) -> Result<BudgetedChangeSet, String>
where
    Old: Content,
    New: Content<Value = Old::Value>,
    OldId: Borrow<str>,
    NewId: Borrow<str>,
    Slot: Eq + Hash,
{
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned对象diff入口未启用".into());
    }
    let mut old_by_id = bj::new_map::<&str, usize>(allocation);
    let mut active_by_id = bj::new_map::<&str, usize>(allocation);
    for (index, entry) in old.iter().enumerate() {
        old_by_id
            .insert(identity(&entry.key.object_id), index)
            .map_err(|e| e.to_string())?;
        if entry.active {
            active_by_id
                .insert(identity(&entry.key.object_id), index)
                .map_err(|e| e.to_string())?;
        }
    }
    // 同槽位候选取**末位**：与旧实现 `new_objects.iter().collect::<BTreeMap<_,_>>()` 同语义。
    let mut new_by_slot = bj::new_map::<&Slot, usize>(allocation);
    for (index, key) in new.iter().enumerate() {
        new_by_slot
            .insert(&key.slot, index)
            .map_err(|e| e.to_string())?;
    }

    let mut changes = BudgetedChangeSet {
        births: bj::new_work(allocation),
        withdrawals: bj::new_work(allocation),
        replaces: bj::new_work(allocation),
    };

    // 1) 活动旧身份按给定主序：槽位消失 = 撤；槽位被异身份顶替 = 撤 + 替换对。
    for (old_index, entry) in old.iter().enumerate() {
        if !entry.active {
            continue;
        }
        match new_by_slot.get(&entry.key.slot).copied() {
            None => changes
                .withdrawals
                .try_push(Withdrawal {
                    old_index,
                    removal: Removal::Removed,
                    new_index: None,
                })
                .map_err(|e| e.to_string())?,
            Some(new_index)
                if identity(&new[new_index].object_id) != identity(&entry.key.object_id) =>
            {
                changes
                    .withdrawals
                    .try_push(Withdrawal {
                        old_index,
                        removal: Removal::Superseded,
                        new_index: Some(new_index),
                    })
                    .map_err(|e| e.to_string())?;
                changes
                    .replaces
                    .try_push(Replace {
                        old_index,
                        new_index,
                    })
                    .map_err(|e| e.to_string())?;
            }
            Some(_) => {}
        }
    }

    // 2) 本次候选按给定序：同 ID 先核不可变内容，再定「未变化 / 拒绝复活 / 真实出生」。
    for (new_index, key) in new.iter().enumerate() {
        let existing = old_by_id.get(identity(&key.object_id)).copied();
        if let Some(old_index) = existing {
            // 先既有、后候选：与旧实现同一求值次序，读失败原样上报。
            let stored_content = old[old_index].key.immutable.read()?;
            let candidate_content = key.immutable.read()?;
            if stored_content != candidate_content {
                return Err(format!(
                    "IdentityConflict：object_id `{}` 已存在且不可变内容不同（同 ID 异内容不得覆盖或吞掉）",
                    identity(&key.object_id)
                ));
            }
        }
        match active_by_id.get(identity(&key.object_id)).copied() {
            Some(old_index) => {
                if old[old_index].key.slot != key.slot {
                    return Err(format!(
                        "StorageUnavailable：object_id `{}` 的活动发生区间与本次不一致",
                        identity(&key.object_id)
                    ));
                }
                // 未变化：不重发（changes-only 域的 Delta 不重复完整活动集）。
            }
            None => {
                if existing.is_some() {
                    return Err(format!(
                        "IdentityConflict：object_id `{}` 是已撤回历史身份，不得复活",
                        identity(&key.object_id)
                    ));
                }
                changes
                    .births
                    .try_push(new_index)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn key<C: Content>(object_id: &str, slot: &str, immutable: C) -> Key<C> {
        Key {
            object_id: object_id.to_string(),
            slot: slot.to_string(),
            immutable,
        }
    }

    fn stored(object_id: &str, slot: &str, content: &str, active: bool) -> Existing<Ready<String>> {
        Existing {
            key: key(object_id, slot, Ready(content.to_string())),
            active,
        }
    }

    fn ready_key(object_id: &str, slot: &str, content: &str) -> Key<Ready<String>> {
        key(object_id, slot, Ready(content.to_string()))
    }

    /// 记录读取次数的内容视图：`Err` 表示读不出，用来锁「按需读取」的边界。
    struct View {
        value: Result<String, String>,
        reads: Rc<Cell<usize>>,
    }

    impl View {
        fn ready(value: &str) -> (View, Rc<Cell<usize>>) {
            let reads = Rc::new(Cell::new(0));
            (
                View {
                    value: Ok(value.to_string()),
                    reads: reads.clone(),
                },
                reads,
            )
        }

        fn unreadable() -> (View, Rc<Cell<usize>>) {
            let reads = Rc::new(Cell::new(0));
            (
                View {
                    value: Err("StorageUnavailable：本次对象缺不可变字段 kind".to_string()),
                    reads: reads.clone(),
                },
                reads,
            )
        }
    }

    impl Content for View {
        type Value = String;
        fn read(&self) -> Result<String, String> {
            self.reads.set(self.reads.get() + 1);
            self.value.clone()
        }
    }

    /// keep：同 ID / 同槽位 / 同不可变内容 = 未变化，不重发。
    #[test]
    fn unchanged_identity_is_not_a_change() {
        let new = vec![ready_key("a", "S1", "v1")];
        let old = vec![stored("a", "S1", "v1", true)];
        let changes = plan(&new, &old).unwrap();
        assert!(changes.births.is_empty());
        assert!(changes.withdrawals.is_empty());
        assert!(changes.replaces.is_empty());
    }

    /// birth：既无活动也无历史身份的 ID 才是真实出生，且按候选序给索引。
    #[test]
    fn birth_is_real_only_without_history_and_follows_candidate_order() {
        // `a` 是已撤回历史身份（也没出现在候选里）；`b` 是既有活动（未变化）；`c` 无任何历史 = 出生。
        let old = vec![
            stored("a", "S0", "v0", false),
            stored("b", "S1", "v1", true),
        ];
        let new = vec![ready_key("b", "S1", "v1"), ready_key("c", "S2", "v2")];
        let changes = plan(&new, &old).unwrap();
        assert_eq!(changes.births, vec![1]);
        assert!(changes.withdrawals.is_empty());
        // 空库上两条候选都按候选序出生。
        let changes = plan::<Ready<String>, Ready<String>>(&new, &[]).unwrap();
        assert_eq!(changes.births, vec![0, 1]);
    }

    /// removed：活动身份在本轮槽位消失 = 撤（按旧活动主序给索引），不是「新生」。
    #[test]
    fn vanished_slot_is_withdrawn_in_old_order() {
        let new = vec![ready_key("b", "S2", "v2")];
        let old = vec![stored("a", "S1", "v1", true), stored("b", "S2", "v2", true)];
        let changes = plan(&new, &old).unwrap();
        assert_eq!(changes.withdrawals.len(), 1);
        assert_eq!(changes.withdrawals[0].old_index, 0);
        assert!(matches!(changes.withdrawals[0].removal, Removal::Removed));
        assert_eq!(changes.withdrawals[0].new_index, None);
        assert!(changes.replaces.is_empty());
        assert!(changes.births.is_empty());
        // 两条活动身份都消失：索引保持旧活动主序。
        let changes = plan::<Ready<String>, Ready<String>>(&[], &old).unwrap();
        assert_eq!(
            changes
                .withdrawals
                .iter()
                .map(|w| w.old_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    /// replace：同槽位换了身份 = 撤旧 + 替代对；替代目标同时是真实出生（它本代才首次获知）。
    #[test]
    fn same_slot_new_identity_is_a_replace_not_a_birth() {
        let new = vec![ready_key("b", "S1", "v2")];
        let old = vec![stored("a", "S1", "v1", true)];
        let changes = plan(&new, &old).unwrap();
        // 新身份必须落库：出生集含替代目标（旧实现同此，upserts 才带得动替代目标）。
        assert_eq!(changes.births, vec![0]);
        assert_eq!(changes.withdrawals.len(), 1);
        assert_eq!(changes.withdrawals[0].old_index, 0);
        assert!(matches!(
            changes.withdrawals[0].removal,
            Removal::Superseded
        ));
        assert_eq!(changes.withdrawals[0].new_index, Some(0));
        assert_eq!(changes.replaces.len(), 1);
        assert_eq!(changes.replaces[0].old_index, 0);
        assert_eq!(changes.replaces[0].new_index, 0);
    }

    /// 同槽位多候选：取末位（与旧实现 `collect()` 同语义），撤/替/出生三件事互不顶替。
    #[test]
    fn last_candidate_of_a_slot_wins() {
        let new = vec![ready_key("a", "S1", "v1"), ready_key("b", "S1", "v2")];
        let old = vec![stored("a", "S1", "v1", true)];
        let changes = plan(&new, &old).unwrap();
        // 槽位由末位候选 `b` 接管：撤 `a` + 替代对指向候选 1。
        assert_eq!(changes.withdrawals.len(), 1);
        assert_eq!(changes.withdrawals[0].new_index, Some(1));
        assert_eq!(changes.replaces[0].new_index, 1);
        // 候选 0（`a` 未变化）不重发；候选 1（`b`）才出生。
        assert_eq!(changes.births, vec![1]);
    }

    /// withdrawn-reuse：已撤回历史身份（含撤回身份）不得复活；内容不同则先是内容冲突。
    #[test]
    fn withdrawn_history_cannot_be_revived() {
        let new = vec![ready_key("a", "S1", "v1")];
        let old = vec![stored("a", "S1", "v1", false)];
        let error = plan(&new, &old).unwrap_err();
        assert!(error.contains("已撤回历史身份，不得复活"), "{error}");
        // 同 ID 但内容已不同：内容冲突先报（旧实现同一次序），绝不落到「复活」分支。
        let new = vec![ready_key("a", "S1", "v2")];
        let error = plan(&new, &old).unwrap_err();
        assert!(
            error.contains("已存在且不可变内容不同"),
            "内容冲突必须先于复活判定：{error}"
        );
    }

    /// conflicting-ID：同 ID 异内容一律拒绝；同 ID 同内容但槽位漂移也是存储不可用。
    #[test]
    fn same_id_conflicts_are_refused() {
        let old = vec![stored("a", "S1", "v1", true)];
        let new = vec![ready_key("a", "S1", "v2")];
        let error = plan(&new, &old).unwrap_err();
        assert!(error.contains("IdentityConflict"), "{error}");
        assert!(error.contains("已存在且不可变内容不同"), "{error}");
        let new = vec![ready_key("a", "S2", "v1")];
        let error = plan(&new, &old).unwrap_err();
        assert!(error.contains("活动发生区间与本次不一致"), "{error}");
    }

    /// 读取边界：出生候选、被撤槽位、未被引用的历史身份都不读不可变内容；只有同 ID 对拍才读。
    #[test]
    fn immutable_content_is_read_only_at_the_comparison() {
        // ① 出生：不读。
        let (view, reads) = View::unreadable();
        let new = vec![key("a", "S1", view)];
        let changes = plan::<View, View>(&new, &[]).unwrap();
        assert_eq!(changes.births, vec![0]);
        assert_eq!(reads.get(), 0, "出生候选不得读不可变内容");

        // ② 槽位消失（撤）：不读旧身份内容。
        let (view, reads) = View::unreadable();
        let old = vec![Existing {
            key: key("a", "S1", view),
            active: true,
        }];
        let changes = plan::<View, View>(&[], &old).unwrap();
        assert!(matches!(changes.withdrawals[0].removal, Removal::Removed));
        assert_eq!(reads.get(), 0, "被撤槽位不得读不可变内容");

        // ③ 撤回身份未被本次候选引用：不读，也不报错。
        let (view, reads) = View::unreadable();
        let old = vec![Existing {
            key: key("a", "S1", view),
            active: false,
        }];
        let (new_view, _) = View::ready("v1");
        let new = vec![key("b", "S2", new_view)];
        let changes = plan::<View, View>(&new, &old).unwrap();
        assert_eq!(changes.births, vec![0]);
        assert_eq!(reads.get(), 0, "未被引用的历史身份不得读不可变内容");

        // ④ 同 ID 对拍：先既有、后候选，各读一次。
        let (old_view, old_reads) = View::ready("v1");
        let (new_view, new_reads) = View::ready("v1");
        let old = vec![Existing {
            key: key("a", "S1", old_view),
            active: true,
        }];
        let new = vec![key("a", "S1", new_view)];
        plan::<View, View>(&new, &old).unwrap();
        assert_eq!((old_reads.get(), new_reads.get()), (1, 1));
    }

    /// 读失败原样上报：不吞、不换成 `IdentityConflict`。
    #[test]
    fn read_failure_is_reported_as_is() {
        let (old_view, _) = View::unreadable();
        let (new_view, _) = View::ready("v1");
        let old = vec![Existing {
            key: key("a", "S1", old_view),
            active: true,
        }];
        let new = vec![key("a", "S1", new_view)];
        let error = plan::<View, View>(&new, &old).unwrap_err();
        assert!(error.contains("缺不可变字段 kind"), "{error}");
        assert!(!error.contains("IdentityConflict"), "{error}");
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_plan_matches_legacy_order_conflicts_and_slot_last_wins() {
        let budget = OwnedBudget::new(1_000_000);
        let old = vec![
            stored("dead", "S0", "v0", false),
            stored("z", "S2", "v2", true),
            stored("a", "S1", "v1", true),
        ];
        for new in [
            vec![ready_key("a", "S1", "v1"), ready_key("b", "S1", "v3")],
            vec![ready_key("fresh", "S9", "v9"), ready_key("z", "S2", "v2")],
            vec![ready_key("dead", "S0", "v0")],
            vec![ready_key("dead", "S0", "conflict")],
            vec![ready_key("a", "wrong-slot", "v1")],
        ] {
            let plain = plan(&new, &old);
            let owned = plan_budgeted(&new, &old, Some(&budget));
            match (plain, owned) {
                (Ok(plain), Ok(owned)) => {
                    assert_eq!(plain.births.as_slice(), &*owned.births);
                    assert_eq!(plain.withdrawals.as_slice(), &*owned.withdrawals);
                    assert_eq!(plain.replaces.as_slice(), &*owned.replaces);
                    assert!(budget.snapshot().live_requested_layout_bytes > 0);
                    assert!(owned.births.allocation_budget().is_some());
                }
                (Err(plain), Err(owned)) => assert_eq!(plain, owned),
                _ => panic!("普通和Owned判定不一致"),
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        // slot末位接管、活动主序和候选序直接锁定，不能仅依赖两臂一致。
        let new = vec![ready_key("a", "S1", "v1"), ready_key("b", "S1", "v3")];
        let owned = plan_budgeted(&new, &old, Some(&budget)).unwrap();
        assert_eq!(&*owned.births, &[1]);
        assert_eq!(
            owned
                .withdrawals
                .iter()
                .map(|w| w.old_index)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(owned.withdrawals[1].new_index, Some(1));
        assert_eq!(owned.replaces[0].new_index, 1);
        let plain_refusal = owned.births.try_into_plain().unwrap_err().to_string();
        assert!(plain_refusal.contains("prototype_audit_allocation_incomplete"));
        drop(owned.withdrawals);
        drop(owned.replaces);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_plan_lazy_reads_prior_failure_and_budget_refusal_release_all() {
        let budget = OwnedBudget::new(1_000_000);
        let (birth, birth_reads) = View::unreadable();
        let new = vec![key("fresh", "S1", birth)];
        let result =
            plan_budgeted::<View, View, String, String, String>(&new, &[], Some(&budget)).unwrap();
        assert_eq!(&*result.births, &[0]);
        assert_eq!(birth_reads.get(), 0);
        drop(result);
        let (prior, prior_reads) = View::unreadable();
        let (candidate, candidate_reads) = View::unreadable();
        let old = vec![Existing {
            key: key("same", "S1", prior),
            active: true,
        }];
        let new = vec![key("same", "S1", candidate)];
        let error = plan_budgeted(&new, &old, Some(&budget)).unwrap_err();
        assert!(error.contains("缺不可变字段 kind"));
        assert_eq!((prior_reads.get(), candidate_reads.get()), (1, 0));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        // 小账本在第一个映射真实分配时拒绝；预算失败不得继续读不可变内容。
        let tiny = OwnedBudget::new(1);
        let error = plan_budgeted(&new, &old, Some(&tiny)).unwrap_err();
        assert!(
            error.contains(newchan_rust::s_content::capture_allocation::LIMIT),
            "{error}"
        );
        assert_eq!(tiny.snapshot().allocation_attempts, 0);
        assert_eq!(tiny.snapshot().rejected_layouts, 1);
        assert!(tiny.allocation_request_probe().refused_bytes > 1);
        assert_eq!((prior_reads.get(), candidate_reads.get()), (1, 0));
        assert_eq!(tiny.snapshot().live_requested_layout_bytes, 0);
    }
}
