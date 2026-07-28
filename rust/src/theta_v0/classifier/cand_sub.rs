//! #552（N2）跨级区间包含谓词 `C⊆C`（SPEC #547 US10/US11/US15/US18，闭环 #544）。
//!
//! 语义无待裁项（ADR-0006「Sub = C⊆C 同口径」+ #246「相切=重合全域」已钉死）：
//! **子级候选事件的 C 段区间 ⊆ 父级候选事件的 C 段区间**，`source_index` 坐标域
//! （生产不变量：nest 级 k == classifier lvl k == 塔级 k ⟹ 跨层比较无级别换算），
//! 闭区间含端点，相切（端点相等）算**包含**。
//!
//! ## 对照纪律（audit §6 / SPEC US18）
//!
//! 不复读 [`super::nest::is_sub`]——那是 `NestInterval` 的 `(start_time, end_time)` **时间**
//! 坐标谓词，属 nest 对照身份（ADR-0005）。
//!
//! **谓词的包含不等式**唯一委托上游单一来源 [`super::cand_event::interval_is_sub`]；探针的
//! 相切 / 相离 / 退化判据亦同源于 `cand_event`（[`intervals_touch`] / [`intervals_are_disjoint`] /
//! [`interval_is_degenerate`]）。**本模块不重写任何区间不等式**——三者共用同一条退化守卫，
//! #246 相切口径与退化语义因此无第二处可漂移。本模块自己只加**跨级分支**
//! （`child.event_level < parent.event_level`）与按对计数。
//!
//! ## 零消费（SPEC US12/US13）
//!
//! 本模块纯只读：谓词与扫描函数均不被 BSP / 级别形成 / 门 / admission / 订单路径调用，
//! 调用点 = 本模块单测 + 诊断 bin `issue550_event_battery`（只印计数不判真值）。

use std::collections::BTreeMap;

use super::cand_event::{
    interval_is_degenerate, interval_is_sub, intervals_are_disjoint, intervals_touch,
    CandidateEvent, CandidateKey, CandidateStreams,
};

/// 跨级候选 `C⊆C`。坐标均为 `source_index`，无级别换算。
///
/// 两个合取分量各有单一来源：
/// - **跨级分支**（本模块）：`child.event_level < parent.event_level`。严格小于——同级不构成
///   子级关系（同级两候选互不为对方的次级别），级别大者亦不能是级别小者的子级。不要求**相邻**：
///   ℓ 与 ℓ+2 之间同样可判（区间套逐级缩小是链的性质，不是本谓词的前件）。
/// - **区间包含**（[`interval_is_sub`]）：闭区间含端点、相切算包含。
pub fn candidate_is_sub(child: &CandidateEvent, parent: &CandidateEvent) -> bool {
    child.event_level < parent.event_level && interval_is_sub(child.interval, parent.interval)
}

/// 一对相邻级 `(child_level, child_level+1)` 的包含判定计数（只读探针）。
///
/// 计数口径全部按**对**（child × parent 笛卡尔积）统计，`pairs` 是分母：任何比值报数都以它为
/// 底，`pairs == 0` 表示该相邻级在本窗口无对可判（真空），不得与「判过但全不成立」混同。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdjacentLevelContainment {
    pub child_level: u32,
    pub parent_level: u32,
    /// 子级事件数（每 key 最新 revision）。
    pub child_events: usize,
    /// 父级事件数（每 key 最新 revision）。
    pub parent_events: usize,
    /// 判定对数 = `child_events * parent_events`。
    pub pairs: usize,
    /// [`candidate_is_sub`] 成立的对数。
    pub contained: usize,
    /// 成立且**至少一端点相等**的对数（#246 相切命中）。
    pub touching: usize,
    /// 成立且两端点均严格内含的对数。
    pub strict: usize,
    /// 两区间无交的对数（相离）。退化对不计入（见 [`Self::degenerate`]）。
    pub disjoint: usize,
    /// 该对中**至少一端区间退化**（`start > end`）的对数。
    ///
    /// 退化对不进 `contained` / `touching` / `strict` / `disjoint` 任何真值桶——三个区间判据
    /// （[`interval_is_sub`] / [`intervals_touch`] / [`intervals_are_disjoint`]）对退化输入一律
    /// 判 false。单列本桶是为了让这些对**可见**：不单列则它们计入分母 `pairs` 却不进任何桶，
    /// 计数不划分、静默丢格，「判过但全不成立」与「根本没进判定」不可区分。
    pub degenerate: usize,
    /// 反向对（把父级事件当 child 传入）区间包含成立、但被跨级分支拒的对数。
    ///
    /// 非零 ⟹ 级别分支在真实数据上**真起作用**（不是恒真装饰）——防真空绿的直接证据。
    pub reverse_blocked_by_level: usize,
}

/// 同级对的级别门计数（只读探针）：同级两事件间区间包含成立、但被跨级分支拒的对数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SameLevelBlock {
    /// 同级有序对数（含自反对：一个事件与自己）。
    pub pairs: usize,
    /// 其中区间包含（[`interval_is_sub`]）成立的对数。
    pub interval_ok: usize,
    /// 其中被跨级分支拒的对数。恒等于 `interval_ok`（同级 ⟹ 级别分支必假）。
    pub blocked: usize,
    /// `blocked` 中的**自反**对数（事件与自己，`a.key == b.key`）——恒等于同级事件总数。
    ///
    /// 单列出来是因为它**没有信息量**：自反包含是区间谓词的自反性，与级别门无关。
    /// 「级别门真起作用」的证据是 [`Self::non_reflexive_blocked`]，不是 `blocked` 本身。
    pub reflexive_blocked: usize,
}

impl SameLevelBlock {
    /// 非自反的级别门拦截数：两个**不同**候选同级且区间相含、被跨级分支拒的对数。
    ///
    /// [`count_same_level`] 产出的实例恒满足 `blocked >= reflexive_blocked`（自反对是被拒对的子集）。
    /// 但本结构体字段全 `pub` 且无构造器守这条不变量（探针载体，诊断 bin 直接读字段打印），
    /// 违例实例在类型层可表达 ⟹ 用 `saturating_sub`：`reflexive_blocked > blocked` 时**饱和到 0**，
    /// 不 panic（debug）也不回绕成天文数字（release）——回绕值会被 bin 原样印成「拦截数」，
    /// 一个溢出的假计数比 0 更难识别。
    pub fn non_reflexive_blocked(self) -> usize {
        self.blocked.saturating_sub(self.reflexive_blocked)
    }
}

/// 相邻级包含扫描结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainmentScan {
    /// 每个存在的相邻级对一条，按 `child_level` 升序。
    pub levels: Vec<AdjacentLevelContainment>,
    pub same_level: SameLevelBlock,
}

impl ContainmentScan {
    /// 全部相邻级的判定对数合计（`0` = 真空扫描，任何「包含率」结论在此不可读）。
    pub fn total_pairs(&self) -> usize {
        self.levels.iter().map(|entry| entry.pairs).sum()
    }

    /// 全部相邻级的包含成立数合计。
    pub fn total_contained(&self) -> usize {
        self.levels.iter().map(|entry| entry.contained).sum()
    }

    /// 全部相邻级的相切命中数合计。
    pub fn total_touching(&self) -> usize {
        self.levels.iter().map(|entry| entry.touching).sum()
    }
}

/// 每 key 最新 revision，按 `event_level` 分组（`BTreeMap` ⟹ 级别升序确定）。
///
/// 取最新 revision 而非全史：包含判定问的是「当前几何是否相含」，同一 key 的历史 revision
/// 是同一候选的旧几何，计入会把一个候选按修订次数重复计数、污染分母。
///
/// **不按 `state` 过滤**（含 `Invalidated` 终态）：`C⊆C` 是纯几何谓词，与候选活/死状态正交；
/// 「哪些候选有资格进链」是下游 N3 链构造的裁量，在此提前过滤等于把未裁定的语义塞进谓词。
fn latest_by_level(streams: &CandidateStreams) -> BTreeMap<u32, Vec<CandidateEvent>> {
    let mut latest = BTreeMap::<CandidateKey, CandidateEvent>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    let mut by_level = BTreeMap::<u32, Vec<CandidateEvent>>::new();
    for event in latest.into_values() {
        by_level.entry(event.event_level).or_default().push(event);
    }
    by_level
}

/// 相邻级包含只读探针：对每对相邻级 `(ℓ, ℓ+1)` 的候选事件跑 [`candidate_is_sub`] 并报数。
///
/// 只读：不改事件流、不产生任何观察或修订，返回值只被打印/断言。
pub fn scan_adjacent_containment(streams: &CandidateStreams) -> ContainmentScan {
    let by_level = latest_by_level(streams);
    let mut levels = Vec::new();
    for (&child_level, children) in &by_level {
        let parent_level = child_level + 1;
        let Some(parents) = by_level.get(&parent_level) else {
            continue;
        };
        levels.push(count_pairs(child_level, parent_level, children, parents));
    }
    ContainmentScan {
        levels,
        same_level: count_same_level(&by_level),
    }
}

fn count_pairs(
    child_level: u32,
    parent_level: u32,
    children: &[CandidateEvent],
    parents: &[CandidateEvent],
) -> AdjacentLevelContainment {
    let mut entry = AdjacentLevelContainment {
        child_level,
        parent_level,
        child_events: children.len(),
        parent_events: parents.len(),
        pairs: children.len() * parents.len(),
        ..AdjacentLevelContainment::default()
    };
    for child in children {
        for parent in parents {
            if interval_is_degenerate(child.interval) || interval_is_degenerate(parent.interval) {
                entry.degenerate += 1;
                continue;
            }
            if candidate_is_sub(child, parent) {
                entry.contained += 1;
                if intervals_touch(child.interval, parent.interval) {
                    entry.touching += 1;
                } else {
                    entry.strict += 1;
                }
            }
            if intervals_are_disjoint(child.interval, parent.interval) {
                entry.disjoint += 1;
            }
            // 反向对：区间成立却被级别分支拒 ⟹ 级别门在本窗口真起作用。
            if interval_is_sub(parent.interval, child.interval) && !candidate_is_sub(parent, child)
            {
                entry.reverse_blocked_by_level += 1;
            }
        }
    }
    entry
}

fn count_same_level(by_level: &BTreeMap<u32, Vec<CandidateEvent>>) -> SameLevelBlock {
    let mut block = SameLevelBlock::default();
    for events in by_level.values() {
        for a in events {
            for b in events {
                block.pairs += 1;
                if interval_is_sub(a.interval, b.interval) {
                    block.interval_ok += 1;
                    if !candidate_is_sub(a, b) {
                        block.blocked += 1;
                        if a.key == b.key {
                            block.reflexive_blocked += 1;
                        }
                    }
                }
            }
        }
    }
    block
}

#[cfg(test)]
mod tests {
    use super::super::super::types::Side;
    use super::super::cand_event::{
        CandidateEventBook, CandidateKind, CandidateObservation, CandidateState, ParentFingerprint,
        StructuralPredicates, CANDIDATE_RULE_VERSION,
    };
    use super::*;

    fn event(level: u32, interval: (usize, usize)) -> CandidateEvent {
        CandidateEvent {
            key: key(level, interval.0),
            kind: CandidateKind::Trend,
            event_level: level,
            center_ids: Some((10, 20)),
            candidate_group_id: 1,
            pair_id: 2,
            structural_predicates: StructuralPredicates {
                direction: true,
                comparable: true,
                extreme: true,
            },
            extreme_proof: (11, 19),
            third_class_proof: None,
            interval,
            state: CandidateState::Provisional,
            observed_at: interval.1,
            first_provable_at: Some(interval.1),
            confirmed_at: None,
            invalidated_at: None,
            revision: 0,
            supersedes_revision: None,
            revision_at: interval.1,
        }
    }

    fn key(level: u32, c_start: usize) -> CandidateKey {
        CandidateKey {
            rule_version: CANDIDATE_RULE_VERSION,
            level,
            kind: CandidateKind::Trend,
            side: Side::Long,
            previous_center_start: Some(10),
            parent: ParentFingerprint {
                center_start: 20,
                zd: 100,
                zg: 110,
            },
            seg_a: (11, 19),
            c_start,
        }
    }

    /// 真值表①：区间维（级别固定为合法跨级 0<1）——相切/严格/相离/反向/同区间逐格。
    #[test]
    fn cross_level_sub_truth_table_on_interval_axis() {
        let parent = event(1, (10, 20));
        // 同区间（两端同时相切）= 包含（#246 相切=重合全域）。
        assert!(candidate_is_sub(&event(0, (10, 20)), &parent));
        // 左端相切。
        assert!(candidate_is_sub(&event(0, (10, 15)), &parent));
        // 右端相切。
        assert!(candidate_is_sub(&event(0, (15, 20)), &parent));
        // 严格包含（两端点均内含）。
        assert!(candidate_is_sub(&event(0, (11, 19)), &parent));
        // 左端越界（反向不包含）。
        assert!(!candidate_is_sub(&event(0, (9, 20)), &parent));
        // 右端越界。
        assert!(!candidate_is_sub(&event(0, (10, 21)), &parent));
        // 完全相离（右侧）。
        assert!(!candidate_is_sub(&event(0, (30, 40)), &parent));
        // 完全相离（左侧）。
        assert!(!candidate_is_sub(&event(0, (1, 5)), &parent));
        // 部分交叠（左出右内）。
        assert!(!candidate_is_sub(&event(0, (5, 15)), &parent));
        // 反包含（父⊆子，方向反了）。
        assert!(!candidate_is_sub(&event(0, (0, 30)), &parent));
    }

    /// 真值表②：级别维（区间固定为成立的严格包含）——同级/反向级/相邻级/跨两级逐格。
    #[test]
    fn cross_level_sub_truth_table_on_level_axis() {
        let inner = (11, 19);
        let outer = (10, 20);
        // 相邻级：子级 < 父级 ⟹ 成立。
        assert!(candidate_is_sub(&event(0, inner), &event(1, outer)));
        // 跨两级：谓词不要求相邻。
        assert!(candidate_is_sub(&event(0, inner), &event(2, outer)));
        // 同级：级别分支严格小于 ⟹ 拒（即便区间包含成立）。
        assert!(!candidate_is_sub(&event(1, inner), &event(1, outer)));
        assert!(
            interval_is_sub(inner, outer),
            "同级被拒的是级别分支，不是区间"
        );
        // 反向级别：级别大者不能是级别小者的子级。
        assert!(!candidate_is_sub(&event(2, inner), &event(1, outer)));
        // 反向级别 + 同区间：两端相切也不救级别分支。
        assert!(!candidate_is_sub(&event(1, outer), &event(0, outer)));
    }

    /// 违例构造（`reflexive_blocked > blocked`）下非自反拦截数饱和到 0，不 panic 也不回绕。
    ///
    /// 字段全 `pub` 且无构造器守 `blocked >= reflexive_blocked`（探针结构体，诊断 bin 直接读字段），
    /// 故该违例在类型层可表达：裸 `-` 会在 debug 下 panic、release 下回绕成天文数字并被 bin 印出。
    #[test]
    fn non_reflexive_blocked_saturates_on_inconsistent_counts() {
        let block = SameLevelBlock {
            blocked: 1,
            reflexive_blocked: 3,
            ..SameLevelBlock::default()
        };
        assert_eq!(block.non_reflexive_blocked(), 0);
    }

    /// 退化区间（start > end）在跨级分支成立时仍被区间侧拒——单一来源的传导锁。
    #[test]
    fn degenerate_interval_is_rejected_through_shared_interval_predicate() {
        assert!(!candidate_is_sub(&event(0, (20, 10)), &event(1, (10, 20))));
        assert!(!candidate_is_sub(&event(0, (11, 19)), &event(1, (20, 10))));
        // 单点区间（start == end）合法，可被包含也可作父。
        assert!(candidate_is_sub(&event(0, (15, 15)), &event(1, (10, 20))));
        assert!(candidate_is_sub(&event(0, (15, 15)), &event(1, (15, 15))));
    }

    fn observation(level: u32, c_start: usize, interval: (usize, usize)) -> CandidateObservation {
        CandidateObservation {
            key: key(level, c_start),
            kind: CandidateKind::Trend,
            center_ids: Some((10, 20)),
            candidate_group_id: 1,
            pair_id: 2,
            structural_predicates: StructuralPredicates {
                direction: true,
                comparable: true,
                extreme: true,
            },
            extreme_proof: (11, 19),
            third_class_proof: None,
            interval,
            state: CandidateState::Provisional,
            first_provable_at: Some(interval.1),
            confirmed_at: None,
        }
    }

    /// 扫描探针在合成事件流上逐格计数：相邻级对、相切、严格、相离、两个级别门计数。
    ///
    /// 全部观察同批提交：`advance` 对本批未见的 key 判 `Invalidated`（缺席失效），分批提交会
    /// 让先入簿的父级在下一批被判失效——夹具须与事件簿的观察语义一致，不是风格选择。
    #[test]
    fn scan_counts_containment_touching_and_level_gate_blocks() {
        let mut book = CandidateEventBook::default();
        book.advance(
            &[
                // L1 父：一条 (10,20)。
                observation(1, 10, (10, 20)),
                // L0 子：相切 (10,15)、严格 (12,18)、相离 (30,40)。
                observation(0, 10, (10, 15)),
                observation(0, 12, (12, 18)),
                observation(0, 30, (30, 40)),
            ],
            40,
        );
        let scan = scan_adjacent_containment(&book.streams());

        assert_eq!(scan.levels.len(), 1, "只有 (0,1) 一对相邻级");
        let entry = scan.levels[0];
        assert_eq!((entry.child_level, entry.parent_level), (0, 1));
        assert_eq!((entry.child_events, entry.parent_events), (3, 1));
        assert_eq!(entry.pairs, 3);
        assert_eq!(entry.contained, 2, "相切 + 严格各一");
        assert_eq!(entry.touching, 1);
        assert_eq!(entry.strict, 1);
        assert_eq!(entry.disjoint, 1, "(30,40) 与 (10,20) 无交");
        assert_eq!(scan.total_pairs(), 3);
        assert_eq!(scan.total_contained(), 2);
        assert_eq!(scan.total_touching(), 1);

        // 反向级别门：父 (10,20) 反向套子 (30,40)? 否；套 (10,15)/(12,18)? 否 ⟹ 本例 0。
        assert_eq!(entry.reverse_blocked_by_level, 0);
        // 同级门：L0 三条互不相含（(12,18) ⊄ (10,15) 等），只有自反对成立且全被级别门拒。
        assert_eq!(scan.same_level.pairs, 3 * 3 + 1);
        assert_eq!(scan.same_level.interval_ok, 4, "三条 L0 自反 + L1 自反");
        assert_eq!(scan.same_level.blocked, scan.same_level.interval_ok);
        assert_eq!(scan.same_level.reflexive_blocked, 4, "本例拦截全是自反");
        assert_eq!(
            scan.same_level.non_reflexive_blocked(),
            0,
            "本例无两个不同候选同级相含"
        );
    }

    /// 退化区间的对进专门桶：不进 `contained` / `touching` / `strict` / `disjoint` 任何真值桶。
    ///
    /// 反事实（就地手写不等式时期）：相离判据 `parent.1 < child.0` 不带退化守卫 ⟹ 退化子区间
    /// `(60,50)` 对父 `(10,20)` 被判「相离」（`20 < 60` 成立），而退化父 `(40,30)` 对子 `(12,18)`
    /// 既不进 `contained`（`interval_is_sub` 有守卫）也不进 `disjoint`（`18 < 40` 不成立、
    /// `30 < 12` 不成立）⟹ 静默丢格。
    #[test]
    fn degenerate_intervals_go_to_their_own_bucket_not_silently_dropped() {
        let mut book = CandidateEventBook::default();
        book.advance(
            &[
                // L1 父：正常 (10,20) + 退化 (40,30)。
                observation(1, 10, (10, 20)),
                observation(1, 40, (40, 30)),
                // L0 子：严格包含 (12,18)、相离 (30,40)、退化 (60,50)。
                observation(0, 12, (12, 18)),
                observation(0, 30, (30, 40)),
                observation(0, 60, (60, 50)),
            ],
            60,
        );
        let scan = scan_adjacent_containment(&book.streams());
        let entry = scan.levels[0];
        assert_eq!((entry.child_events, entry.parent_events), (3, 2));
        assert_eq!(entry.pairs, 6);
        assert_eq!(entry.contained, 1, "只有 (12,18) ⊆ (10,20)");
        assert_eq!(entry.strict, 1);
        assert_eq!(entry.touching, 0);
        assert_eq!(entry.disjoint, 1, "只有 (30,40) 与 (10,20) 无交");
        assert_eq!(
            entry.degenerate, 4,
            "6 对中只有「正常子 × 正常父」2 对非退化"
        );
        // 本夹具下四桶恰好划分分母——退化对既不丢也不重复计入其他桶。
        assert_eq!(
            entry.contained + entry.disjoint + entry.degenerate,
            entry.pairs
        );
    }

    /// 同级门的非自反拦截：两个**不同**候选同级且区间相含 ⟹ 级别分支是唯一拒因。
    #[test]
    fn same_level_block_separates_reflexive_from_real_interception() {
        let mut book = CandidateEventBook::default();
        book.advance(
            &[
                observation(0, 10, (10, 20)),
                observation(0, 12, (12, 18)),
                observation(1, 30, (30, 40)),
            ],
            40,
        );
        let scan = scan_adjacent_containment(&book.streams());
        assert_eq!(scan.same_level.pairs, 2 * 2 + 1);
        // (12,18)⊆(10,20) 一条非自反 + 三条自反。
        assert_eq!(scan.same_level.interval_ok, 4);
        assert_eq!(scan.same_level.blocked, 4);
        assert_eq!(scan.same_level.reflexive_blocked, 3);
        assert_eq!(
            scan.same_level.non_reflexive_blocked(),
            1,
            "同级两个不同候选相含被级别门拒 ⟹ 级别分支非装饰"
        );
    }

    /// 反向级别门非真空的合成锁：父区间被子区间包含时，级别分支是唯一拒因。
    #[test]
    fn reverse_blocked_by_level_counts_interval_ok_but_level_rejected() {
        let mut book = CandidateEventBook::default();
        // L1 父 (12,18) 被 L0 子 (10,20) 包含——反向区间成立，级别分支拒。
        book.advance(
            &[observation(1, 12, (12, 18)), observation(0, 10, (10, 20))],
            20,
        );
        let scan = scan_adjacent_containment(&book.streams());
        let entry = scan.levels[0];
        assert_eq!(entry.pairs, 1);
        assert_eq!(entry.contained, 0, "子级区间反而更宽 ⟹ C⊆C 不成立");
        assert_eq!(
            entry.reverse_blocked_by_level, 1,
            "反向区间包含成立却被级别分支拒 ⟹ 级别门非装饰"
        );
    }

    /// 只取每 key 最新 revision：同一候选的生长历史不重复计入分母。
    #[test]
    fn scan_reads_latest_revision_only() {
        let mut book = CandidateEventBook::default();
        book.advance(
            &[observation(1, 10, (10, 20)), observation(0, 10, (10, 12))],
            12,
        );
        // 父级观察原样重提（投影相同 ⟹ 幂等跳过），子级右端生长 12→15 ⟹ 追加 revision 1。
        book.advance(
            &[observation(1, 10, (10, 20)), observation(0, 10, (10, 15))],
            15,
        );
        let stream_len: usize = book.streams().iter().map(|stream| stream.len()).sum();
        assert_eq!(stream_len, 3, "L0 两次修订 + L1 一条");
        let scan = scan_adjacent_containment(&book.streams());
        assert_eq!(
            scan.levels[0].child_events, 1,
            "同 key 两 revision 只算一条"
        );
        assert_eq!(scan.levels[0].pairs, 1);
        assert_eq!(scan.levels[0].contained, 1);
        assert_eq!(scan.levels[0].touching, 1, "左端相切");
    }
}
