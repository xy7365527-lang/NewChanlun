//! V3 活假设状态机（`NestLifecycleBook`，issue #231 重建票；map #59 Destination 组件）。
//!
//! 本模块是「活假设状态机」的重建实装：原实装（2026-07-20，1283 行 + #64 续作 + #78 修复，
//! 实装报告 `chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md`）从未提交、
//! untracked 被删、全库无副本（#229 定案）。本文件按合并规格 spec #232
//! （`chanlun/review-results/spec-v3-lifecycle-rebuild-20260724.md`）重建：实装卡 §2–§7
//! （sidecar 注册表 + 三态 + 五钟）+ 勘误（trend 反超真实不可达）+ #78 修复全量
//! （ForceCheck 三值化 / as_of 单调守卫）+ #64 裁定边界（构建放开、消费不放开）。
//! #421 已把状态机作为 sidecar 接入 `p123_fast_replay` 的重估触发循环；既有 YieldBook、
//! stdout 与 `P116_DUMP` 均不读本 book，生命周期修订只写独立 `P421_LIFECYCLE_DUMP`。
//!
//! # 090 登记（声明 = 能力）
//!
//! 1. **白名单工程桥——禁入证书真值路径**：trend 域 provider 确认分支把 `interval_b`/
//!    `turn_source` 收束到 `[c_start, t*]`（level_view.rs:786-789），全离开段坐标不随事件
//!    暴露；本模块身份键 `LifecycleKey.seg_c_full` 取事件产出坐标（未确认 = 全离开段，
//!    确认 = 收束段），靠 `bridge_identity`（除 seg_c 右端外全等判同身份）吸收
//!    收束/回扩/活窗延展三形态，记 `Supersedes` 不记 Invalidated。**这是工程桥，不是
//!    E2E §6.1:248 WireV1 EventKey；并轨前不得进入任何证书真值路径**（卡 §6.3/§9；
//!    裁定 #64 §2(c) 裁定义务）。N^δ 装配、`d_parent_interval_snapshot/terminal`、
//!    基例门均不读本 book。
//! 2. **Unresolved 出切片**（卡 §2.4）：交付三态链 Provisional→Confirmed/Invalidated，
//!    非 E2E-D5 全四态；Unresolved 要求 proxy 前提四态与 provider 失败原因通道
//!    （v0 provider 对映射失败静默置 false，无此通道），与 DeferOrphan 重判一并后续切片。
//! 3. **trend 反超：合成保留、真实不可达**（勘误 erratum-v3-lifecycle-impl-20260721）：
//!    `trend_confirm_time` 在首个 T2∧T5-OR 同真点即返回 t*，proxy 单调 ⟹ 同一身份内
//!    `confirm_times` Some 前缀稳定，Some→None 在真实 provider 事件流上不可达。T1 的
//!    trend 合成反超路径保留为合成测试，不代表真实可达；**真实可达通道 = pan 活窗**
//!    （T11 真实 provider 夹具实证）。
//! 4. **#64 §5 验收线字面矛盾登记**：评审措辞「Invalidated 至少一个真实案例可触发、
//!    可消费、可查账」与 §2(a)「消费不放开」字面冲突（§5 原文「可消费」已经
//!    2026-07-21 编排者裁定改为「可查账」）。本模块按 §2(a) 执行：Invalidated 全程
//!    留档可查账（禁删除、原因码与力度证据入载荷），**消费侧仍 Closed-only**
//!    （`consumable_closed` 只放 Confirmed）；矛盾登记归编排者澄清
//!    （spec #232 Further Notes），本模块不替裁。
//!
//! # 口径与契约（卡 §4.3/§5.3 + 实装报告 §5 登记）
//!
//! - **等力亦失效**：通道谓词为严格 `<`（divergence.rs:331-333，等值不算衰减）——
//!   「力度反超」本模块口径 = 没有任何一个通道还支持 c 弱于 a。工程口径，与
//!   `is_divergence` 同 discipline，不冒充教义逐字（卡 §4.3；T2 等力子场景锁定）。
//! - **背驰否证两类分开**（ADR-0003 / 票 #425）：**被反超** = 曾构成、后被否证
//!   （061:26「一旦力度大于前者，那么就可以断定背驰段不成立」，要求曾写 first_provable）；
//!   **从未构成** = 根本未构成（061:28「因为背驰如果没有创新高，是不存在的」）——结构完成
//!   时从未写 first_provable ⟹ 转终态挂 `NeverConstituted`，不再以活假设身份挂账。
//!   **工程口径登记**：本模块的「从未构成」判据是**力度谓词从未可证**，不是逐字的
//!   「未创新高」——创新高预滤由产窗侧 `pan_div_structure_extreme` 承担（未创新高的窗根本
//!   不建仓）。与「等力亦失效」同 discipline：工程口径，不冒充教义逐字。两码
//!   在 first_provable 上严格互补（`assert_invariants` 逐条钉死），不互相冒充；沿用既有
//!   终态 `Invalidated`、**不新增第四态** ⟹ 终态互斥/终态吸收/留档不删三条不变量不重写。
//!   结构未完成期间 force 假而从未可证仍诚实滞留 Provisional（未到结算点，不提前判负）。
//!   **第二终局路径**：只要曾可证，后续活窗力度反超即可从 Provisional 直接转
//!   `Invalidated{ForceOvertake}`，无需先经 `StructureCompleted`（061:26；#421 Q3）。
//! - **judge_at 一个 bit 不动**（卡 §5.2）：字段/写入点/回填/CERT 主键/D3 统计全部保持；
//!   新五钟只活在本模块 entry，`divergence_confirmed` 布尔口径不动。
//! - **provider 两相（票 #527，#523 根因修复）**：活窗与完成事件不再共用「已完成 lower
//!   legs + 同一 locate/extreme」这一套输入——那使活窗最早只能与完成候选同刻出生
//!   （首见即完成，代码拓扑上的近似恒等式）。现在 provider 输出显式 typed 的
//!   [`PanProviderPhase`]：`Live` 的 C **只能**来自行进中段（L1 = parser
//!   `OpenTail.pendingSegment`，见 [`ActiveSegmentFrontier`] / [`provide_active_pan_live_windows`]），
//!   `Completed` **只能**在 lower unit 真正进入 completed set 时构造（[`PanCompletionEvent`]
//!   携 `completed_lower_id`/`completed_at`）。消费方不再按 `kind == Consolidation` 猜
//!   provenance。**级别有效域**（票 #527 → #601 → #602 逐级推进，照实登记当下状态）：
//!   L1/L2/L3 三级都已有 active lower-frontier——L1 取 parser 的行进中段
//!   （[`ActiveSegmentFrontier`]）、L2 取行进中的 L1 窗口单元（[`active_l1_window_frontier`]）、
//!   L3 取行进中的 L2 窗口单元（[`active_l2_window_frontier`]）。`LeveledMove` 在塔上**仍**
//!   没有 Active/Completed 之分（#523 遗留 1 的塔侧现状未变），活动语义由 provider 侧
//!   **只读派生**补齐（#598 裁定路线 i），不写塔。**L4 及以上仍无 active frontier**
//!   （`p123_fast_replay::recompute_lifecycle_window_stems` 的级别上界写死），其身份只经
//!   完成相进账本——那是 provider 能力缺口，**不冒充** true-flash。
//! - **身份消失不变量（票 #559 编排者裁定 2026-07-28，替换 #421 的旧口径）**：旧不变量
//!   「`IdentityVanished = 0`」**已撤销**——它从来不是设计保证，而是「首见即完成」bug 的
//!   副产品（活窗与完成同刻出生 ⟹ 身份从不跨 bar 存活 ⟹ 无从消失）。活窗真实存在后，
//!   parser 教义「未完成走势不预判最终结果」直接蕴含行进中身份可被取消，是**合法新终局
//!   形态**。**新不变量**：*凡消失的身份必须带可审计原因码，且成因拆两类落账本字段*——
//!   (a) [`VanishCause::HypothesisRefuted`] 假设被推翻（真终局）；
//!   (b) [`VanishCause::ObservationSeam`] 观测接缝伪影（provider 换轨丢下，与 #523 根因同
//!   类）。两类禁混记：(b) 不是假设失效，混入寿命/反超率统计会吃进伪影
//!   （`assert_invariants` 全态钉死 `vanish_cause` 有值 ⟺ 原因码是 IdentityVanished）。
//!   本条挂 **#523 遗留问题 2**（稳定身份）名下。
//! - **feed 契约（#421 逃生门）**：生产 trigger/事件流不动；sidecar 另走同源、独立的
//!   **逐 bar 喂数循环**。每根 bar 先喂此刻可见的全部活窗观察，再喂该 bar 首次可见的完成
//!   信号；同一身份 c 窗左端不动、右端逐 bar 延展。`observed_at` 与
//!   `first_provable_at` 只在当前 bar 的实际 `advance` 首写，禁止回填历史或人为延迟完成。
//!   同 bar 开合照实形成 Observed→StructureCompleted→终局，寿命为 0；零寿命是独立统计的
//!   闪现子集，事件不得丢弃。若先前已沿第二终局路径 Provisional→
//!   Invalidated{ForceOvertake} 吸收，后到首完成只进独立完成分母，禁止回填
//!   StructureCompleted。完成后停止活窗延展。
//! - **出切片项**（卡 §9 原样维持）：WireV1 全量 EventKey/StateKey/修订链、
//!   谱系两钟 opened/closed、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition
//!   诊断钟、DeferOrphan 重判、Lean 侧 ActiveTail↔OpenTailSystem 桥、白名单桥与两元锚
//!   （`NestCandidateEventExt.extreme_price/group_anchor`，#110/#206 线已入库）并轨。
//! - **v3 硬禁令合规**：全部判据 = 确定性结构/力度谓词（无概率/统计推断、无回测验证、
//!   无 EMH）；测试全部为确定性合成序列（T5/T11/T14 用真实 provider 夹具）。

use super::super::parser::ParseLayer;
use super::super::types::{Center, Direction, MoveKind, PendingTail, Segment, Side, Tick};
use super::center::{center_from_segments, center_from_window, UnitRange};
use super::divergence::{
    same_color_area, same_dir_hist_peak, segment_dif_peak, segments_diverge_or,
};
use super::ledger_kernel::{
    first_write_clock, LedgerAdmission, LedgerBook, LedgerDelta, LedgerEntryCore, LedgerPolicy,
    LedgerRetrogradeRejection, LedgerRevision, LedgerSettlement, LedgerState,
};
use super::level_view::{LowerLeg, NestCandidateEvent, NestDivergenceKind};
use super::recursive_tower::{
    detect_centers_windowed_resume, map_src_to_close_idx, ElementId, LeveledMove,
};
use super::signal::{
    locate_pan_div_structure, locate_pan_div_structure_front_anchor, nearest_confirmed_center_idx,
    pan_div_structure_extreme,
};
use std::collections::{BTreeMap, BTreeSet};

// ── #1188（B01）：八块职责拆分（纯移动零行为，消费面零改）────────────────────────
// 身份键与桥 → `key`；域类型（三态/原因码/力度/修订/entry）→ `entry`；
// 观察契约 → `observation`；审计/统计载体 → `stats`；注册表 → `book`；
// pan 活窗产出机 + L1 active C frontier → `pan`；L1/L2 active window frontier →
// `window`；回放喂数出口 → `feed`。重导出保 `nest_lifecycle::X` 原路径逐字不变。
mod book;
mod entry;
mod feed;
mod key;
mod observation;
mod pan;
mod stats;
mod window;

pub use book::NestLifecycleBook;
pub use entry::{
    ForceCheck, ForceEvidence, InvalidatedReason, LifecycleRevision, LifecycleRevisionKind,
    NestEventState, NestLifecycleEntry, NestPolicy, UnavailReason, VanishCause,
};
pub use feed::{
    feed_replay_bar, provide_replay_live_windows, PanCompletionEvent, PanLiveRun, PanProviderPhase,
    ReplayBarFeed, ReplayFeedStats,
};
pub use key::LifecycleKey;
pub use observation::{
    active_window_right_edge, ForceMaterial, LifecycleObservation, PanLiveWindow,
};
pub use pan::{
    active_segment_frontier, provide_active_pan_live_windows, provide_pan_live_windows,
    ActiveSegmentFrontier, PanLiveOutcome,
};
pub use stats::{
    CompletionForceUnavailableAudit, CompletionSignal, LifecycleSettlementStats,
    LifetimeDistribution, RetrogradeRejection,
};
pub use window::{
    active_l1_window_frontier, active_l2_window_frontier, ActiveWindowFrontier,
    ActiveWindowOutcome, BatchDroppedWindow, BatchObservabilityTracker, LiveMissCause,
};

// 测试夹具经 `use super::*` 消费桥判据；私有再导入（父模块可见性内）维持原可见性。
use key::{bridge_by_center_upgrade, bridge_identity};

#[cfg(test)]
mod tests {
    use super::super::super::types::Tick;
    use super::super::center::UnitRange;
    use super::super::decompose::{center_block_kind, MoveBlock, MoveStatus};
    use super::super::divergence::self_anchors;
    use super::super::level_view::{
        assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
        provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
        LevelViewMaterial, LevelViewQuery, ProjectionMaterial,
    };
    use super::super::recursive_tower::{ElementId, LeveledMove};
    use super::*;

    // ── 合成观察构造 ────────────────────────────────────────────────────────

    /// trend 合成事件（T1/T8 用——合成路径，真实可达性见各测试注释）。
    fn trend_event(
        interval_b: (usize, usize),
        confirmed: bool,
        turn_source: usize,
        as_of: usize,
    ) -> NestCandidateEvent {
        NestCandidateEvent {
            level: 1,
            side: Side::Short,
            kind: NestDivergenceKind::Trend,
            seg_a: (80, 109),
            interval_b,
            interval_a: (40, 79),
            divergence_confirmed: confirmed,
            turn_source,
            judge_at: as_of,
            provider_window: (0, 199),
            intake_fallback: false,
            b_center_start: 40,
        }
    }

    fn pan_window(seg_a: (usize, usize), c_start: usize, live_end: usize) -> PanLiveWindow {
        PanLiveWindow {
            level: 1,
            side: Side::Long,
            seg_a,
            seg_c_live: (c_start, live_end),
            b_center_start: 20,
            gap_len: 0,
        }
    }

    fn key_trend(seg_c_full: (usize, usize)) -> LifecycleKey {
        LifecycleKey {
            level: 1,
            side: Side::Short,
            kind: NestDivergenceKind::Trend,
            seg_a: (80, 109),
            seg_c_full,
            b_center_start: 40,
        }
    }

    fn key_pan(seg_a: (usize, usize), seg_c_full: (usize, usize)) -> LifecycleKey {
        LifecycleKey {
            level: 1,
            side: Side::Long,
            kind: NestDivergenceKind::Consolidation,
            seg_a,
            seg_c_full,
            b_center_start: 20,
        }
    }

    fn material<'a>(hist: &'a [f64], dif: &'a [f64], close_src: &'a [usize]) -> ForceMaterial<'a> {
        ForceMaterial {
            hist: Some(hist),
            dif: Some(dif),
            close_src,
        }
    }

    fn identity_close_src(n: usize) -> Vec<usize> {
        (0..n).collect()
    }

    /// 票 #604：活窗右端判据单一权威——三支边界逐点核验（正常前进/相等/钳位）。
    #[test]
    fn issue604_active_window_right_edge_boundaries() {
        // as_of > c_start：正常前进，右端随 as_of。
        assert_eq!(active_window_right_edge(69, 95), 95);
        // as_of == c_start：两值相等，无歧义。
        assert_eq!(active_window_right_edge(69, 69), 69);
        // as_of < c_start：钳位退化为单点区间，防止右端早于左端的倒挂。
        assert_eq!(active_window_right_edge(69, 40), 69);
        // 边界外值：0 与 usize::MAX 两端探底。
        assert_eq!(active_window_right_edge(0, 0), 0);
        assert_eq!(active_window_right_edge(0, usize::MAX), usize::MAX);
    }

    /// T1 trend 反超可触发（**合成路径，真实不可达——勘误 20260721**：
    /// trend_confirm_time 首个 T2∧T5-OR 同真点即返回 t*，proxy 单调 ⟹ confirm_times
    /// Some→None 在真实 provider 事件流上不可达；真实可达通道 = pan 活窗 T11）。
    ///
    /// 序列：observed=149 →（收束迁移 Supersedes）first_provable=155（t*）
    /// →（T5-OR 终假回扩）invalidated=169 / ForceOvertake → t=179 终态吸收零输出。
    /// 本测试同时证明「现状把反超丢成 None」被本 book 接住留档（原因码 + 证据载荷）。
    #[test]
    fn t1_trend_force_overtake_invalidates_and_terminal_absorbs() {
        let close_src = identity_close_src(200);
        // 力度材料（Short = 向上离开：红柱面积/正峰）：a=[80,109] 弱，c=[120,169] 全面反超。
        let mut hist = vec![0.0; 200];
        hist[80..=109].fill(2.0); // a：面积 60、柱峰 2.0
        hist[120..=169].fill(3.0); // c：面积 150、柱峰 3.0
        let mut dif = vec![0.0; 200];
        dif[80..=109].fill(5.0);
        dif[120..=169].fill(6.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        // as_of=149：未确认事件（interval_b = 全离开段 (120,149)）——force=Verified(false)
        // 但从未可证 ⟹ 滞留 Provisional（反超定义要求「曾可证」，卡 §4.1）。
        let d = book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 149), false, 149, 149),
                false,
            )],
            149,
            &m,
        );
        assert_eq!(d.len(), 1);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Observed));
        assert_eq!(
            book.get(&key_trend((120, 149))).unwrap().state,
            NestEventState::Provisional
        );

        // as_of=155：确认事件（t*=155，interval_b 收束 (120,155)）——桥迁移 + first_provable。
        let d = book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 155), true, 155, 155),
                false,
            )],
            155,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(
            matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 149)))
        );
        assert!(matches!(d[1].kind, LifecycleRevisionKind::FirstProvable));

        // as_of=169：T5-OR 终假，provider 回扩坐标改发未确认事件 (120,169)——桥迁移后
        // Verified(false) ∧ 曾可证 ⟹ Invalidated(ForceOvertake)。
        let d = book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 169), false, 169, 169),
                false,
            )],
            169,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(
            matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 155)))
        );
        assert!(matches!(
            d[1].kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        ));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(
            entry.observed_at, 149,
            "observed_at 跨迁移不后移（E2E §1:83）"
        );
        assert_eq!(
            entry.first_provable_at,
            Some(155),
            "first_provable = t*（不后移）"
        );
        assert_eq!(entry.invalidated_at, Some(169));
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(
            entry.invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );
        assert_eq!(
            entry.superseded_from,
            Some(key_trend((120, 155))),
            "迁移链留痕"
        );
        // 反超证据载荷可查账（US-03）：c 三通道全面反超 a。
        let ev = entry.force_evidence.expect("ForceOvertake 留证据载荷");
        assert_eq!((ev.area_a, ev.area_c), (60.0, 150.0));
        assert_eq!((ev.dif_peak_a, ev.dif_peak_c), (5.0, 6.0));
        assert_eq!((ev.hist_peak_a, ev.hist_peak_c), (2.0, 3.0));

        // t=179 终态吸收：同 key 任何后续观察零输出（禁复活，E2E §1:83）。
        let d = book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 169), false, 169, 179),
                false,
            )],
            179,
            &m,
        );
        assert!(d.is_empty(), "终态吸收零输出");
        assert_eq!(book.len(), 1, "终态不另立新 key");
        book.assert_invariants();
    }

    /// T2 pan 活窗反超可触发 + 等力边界（卡 §7：a 窗固定、c 活窗逐 bar 延展）。
    #[test]
    fn t2_pan_live_window_force_overtake_and_equal_force_boundary() {
        let close_src = identity_close_src(140);
        // a=[50,59]（Long = 向下离开：绿柱/负峰）：面积 20、柱峰 2.0、黄白线峰 5.0。
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        // c 活窗 [70, as_of]：初弱（0.25/bar、柱峰 0.25、黄白线峰 1.0）。
        hist[70..=129].fill(-0.25);
        dif[70..=139].fill(-1.0);

        // as_of=129：三通道成立（15<20、0.25<2、1<5）⟹ first_provable=129。
        let mut book = NestLifecycleBook::new();
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        assert_eq!(d.len(), 2, "Observed + FirstProvable");
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 129)))
                .unwrap()
                .first_provable_at,
            Some(129)
        );

        // as_of=139：c 窗延展柱力反超（面积 15+30=45、柱峰 3.0、黄白线峰 6.0）
        // ⟹ 三通道全假 ⟹ Invalidated(ForceOvertake)@139。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                false,
            )],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        )));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(entry.first_provable_at, Some(129), "first_provable 不后移");
        assert_eq!(entry.invalidated_at, Some(139));

        // ── 等力边界子场景（卡 §4.3 口径登记：严格 <，等值不算衰减——**等力亦失效**）──
        // c 窗三通道全部恰好等于 a（面积 20==20、柱峰 2.0==2.0、黄白线峰 5.0==5.0）
        // ⟹ OR 不成立 ⟹ Verified(false)；曾可证 ⟹ Invalidated。
        let mut hist_eq = vec![0.0; 140];
        hist_eq[50..=59].fill(-2.0);
        let mut dif_eq = vec![0.0; 140];
        dif_eq[50..=59].fill(-5.0);
        // phase 1（as_of=79）：先弱（面积 5、柱峰 0.5、黄白线峰 1.0）⟹ first_provable。
        hist_eq[70..=79].fill(-0.5);
        dif_eq[70..=79].fill(-1.0);
        let mut book_eq = NestLifecycleBook::new();
        let m = material(&hist_eq, &dif_eq, &close_src);
        book_eq.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 79),
                false,
            )],
            79,
            &m,
        );
        // phase 2（as_of=99）：延展至恰好等力——面积 5+2+13=20、柱峰 2.0、黄白线峰 5.0。
        hist_eq[80] = -2.0;
        hist_eq[81..=93].fill(-1.0);
        dif_eq[80] = -5.0;
        let m = material(&hist_eq, &dif_eq, &close_src);
        let d = book_eq.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 99),
                false,
            )],
            99,
            &m,
        );
        assert!(
            d.iter().any(|r| matches!(
                r.kind,
                LifecycleRevisionKind::Invalidated {
                    reason: InvalidatedReason::ForceOvertake
                }
            )),
            "等力亦失效（严格 < 口径，divergence.rs:331-333；工程口径不冒充教义逐字）"
        );
        book_eq.assert_invariants();
    }

    /// T3 单调不复活（卡 §4.2：c 窗只延不缩 ⟹ 三通道各自单调 真→假 ⟹ 翻假唯一且永久）。
    #[test]
    fn t3_no_revival_after_invalidation() {
        let close_src = identity_close_src(160);
        let mut hist = vec![0.0; 160];
        hist[50..=59].fill(-2.0);
        hist[70..=129].fill(-0.25);
        hist[130..=139].fill(-3.0); // 反超段
        hist[140..=149].fill(-0.1); // 翻假后再灌弱柱
        let mut dif = vec![0.0; 160];
        dif[50..=59].fill(-5.0);
        dif[70..=129].fill(-1.0);
        dif[130..=139].fill(-6.0);
        dif[140..=149].fill(-0.5);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                false,
            )],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        )));
        let revisions_at_invalidation = book.entries().next().unwrap().1.revision;

        // 谓词层可执行证伪器：翻假后延展窗（灌弱柱 [140,149]）segments_diverge_or 仍假
        // （面积/峰值单调不减 ⟹ 真→假单调，divergence.rs:291-327）。
        assert!(
            !segments_diverge_or(&hist, &dif, Side::Long, (50, 59), (70, 149)),
            "谓词层：翻假后延展窗仍假（单调性禁止自行复真）"
        );

        // 状态机层 (a)：延展窗（右端前进）——桥匹配到终态 entry ⟹ 终态吸收。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 149),
                false,
            )],
            149,
            &m,
        );
        assert!(d.is_empty(), "延展窗零新 revision");
        // 状态机层 (b)：构造性「复活」弱窗（右端回缩到 129）——同样桥匹配终态 ⟹ 吸收。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            149,
            &m,
        );
        assert!(d.is_empty(), "构造性复活弱窗零新 revision");
        assert_eq!(
            book.entries().next().unwrap().1.revision,
            revisions_at_invalidation,
            "revision 数零增长"
        );
        assert_eq!(book.len(), 1, "终态不复活、不另立新 key");
        book.assert_invariants();
    }

    /// T4 钟不变量（卡 §5.3 断言 1-5 全列 + `assert_invariants` 公开可调用）。
    #[test]
    fn t4_clock_invariants() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        hist[70..=139].fill(-0.25);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        dif[70..=139].fill(-1.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        // as_of=125：observed=125、first_provable=125 一次写入。
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 125),
                false,
            )],
            125,
            &m,
        );
        // as_of=130/135：活窗延展（桥迁移 Supersedes），钟不后移。
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 130),
                false,
            )],
            130,
            &m,
        );
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 135),
                false,
            )],
            135,
            &m,
        );
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 125, "observed_at 写入不后移");
        assert_eq!(
            entry.first_provable_at,
            Some(125),
            "first_provable 写入不后移"
        );
        // 同 as_of 重复 advance 幂等零 delta（E2E §1:83 as_of = state_as_of 最小形态）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 135),
                false,
            )],
            135,
            &m,
        );
        assert!(d.is_empty(), "同 as_of 幂等零 delta");
        // assert_invariants 公开可调用（钟序/≤ as_of/终态互洽/反超 invalidated ≥
        // first_provable 全列；不只靠 debug_assert——review judgement call 4）。
        book.assert_invariants();
    }

    /// T5 bit-exact 护栏（US-05）：真实夹具（复刻 level_view extended_windows + R1 全合取
    /// MACD + retest 块满足 structural_pair_span），挂/不挂 book 重跑 provider，
    /// 输出 PartialEq + Debug 序列化双判相等——sidecar 反流改生产有结构性证据为否。
    #[test]
    fn t5_bit_exact_guard_provider_stream_untouched() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        // retest 块（Consolidation, Completed）：interval_a 需要 leave→retest 块对
        // （structural_pair_span level_view.rs:513-526 两块均 Completed）。
        let blocks = [
            MoveBlock {
                start_center: 0,
                end_center: 2,
                kind: MoveKind::Trend,
                dir: Some(Direction::Up),
                level_lift: 0,
                status: MoveStatus::Completed,
            },
            MoveBlock {
                start_center: 1,
                end_center: 2,
                kind: MoveKind::Consolidation,
                dir: None,
                level_lift: 0,
                status: MoveStatus::Completed,
            },
        ];
        // R1 全合取 MACD 夹具（level_view.rs auto_pairing_completes_only_after_real_macd_divergence
        // 同参数）：b 段红柱面积 60/柱峰 2.0，c 估计窗 2.0/0.1（T5-OR 成立）；dif 在 w2 span
        // 内变号（T4 回拉 0 轴成立）。
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();
        let query = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 139 },
            as_of: 139,
            version: C2VersionTuple::auto_pairing(),
        };
        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query,
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &blocks,
                lower_legs: &legs,
                hist: &hist,
                dif: &dif,
                close_src: &close_src,
            },
        )
        .unwrap();

        // 不挂 book：provider 输出基线。
        let baseline = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
        );
        assert!(
            !baseline.is_empty(),
            "夹具应产事件（R1 全合取 confirmed 在案）"
        );

        // 挂 book：同输入重跑 provider + 全量事件喂 advance。
        let attached = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
        );
        let observations: Vec<_> = attached
            .iter()
            .map(|event| LifecycleObservation::event(*event, true))
            .collect();
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let _ = book.advance(&observations, 139, &m);

        // 双判相等：PartialEq + Debug 序列化逐字节。
        assert_eq!(
            attached, baseline,
            "挂 book 后 provider 事件流逐字段相等（PartialEq）"
        );
        assert_eq!(
            format!("{attached:?}"),
            format!("{baseline:?}"),
            "Debug 序列化逐字节相等"
        );
        // advance 后入参事件流逐字节不变（sidecar 不反流改生产）。
        let replay: Vec<NestCandidateEvent> = observations
            .iter()
            .map(|obs| match obs {
                LifecycleObservation::Event { event, .. } => *event,
                LifecycleObservation::PanLive { .. } => unreachable!("T5 全事件通道"),
            })
            .collect();
        assert_eq!(replay, baseline, "入参事件流在 advance 后逐字节不变");
        book.assert_invariants();
    }

    /// T6 身份消失（pan 窄锚 → A′ 回退切换，level_view.rs:826-839 两路）：上一 prefix 的
    /// key 本 prefix 不再产出 ⟹ Invalidated(IdentityVanished)，entry 保留（禁删除模拟
    /// 失效），与 ForceOvertake 原因码可区分（E2E §1:81 + 裁定 #64 §2(b)）。
    ///
    /// **成因 (a) 用例**（票 #559）：本 prefix 的新身份 `seg_a` 不同 ⟹ **锚已变** ⟹ 旧锚
    /// 本 prefix 完全不再产窗 ⟹ [`VanishCause::HypothesisRefuted`]（假设被推翻，真终局）。
    /// 成因 (b)（同锚换 C 的观测接缝）见 `t6b_identity_vanish_observation_seam`。
    #[test]
    fn t6_identity_vanish_on_structure_switch() {
        let close_src = identity_close_src(120);
        let hist = vec![0.0; 120];
        let dif = vec![0.0; 120];
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        // as_of=89：窄锚身份（seg_a=(50,69)，c 活窗 (70,89)）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 69), 70, 89),
                false,
            )],
            89,
            &m,
        );
        assert_eq!(
            d.len(),
            1,
            "仅 Observed（零力度序列 ⟹ Verified(false)，从未可证不判负）"
        );
        // as_of=99：结构选择切换为 A′ 回退（seg_a=(20,39)）——seg_a 改变 ⟹ 桥不判同身份
        // （白名单不越界）；旧 key 本 prefix 不再产出 ⟹ Invalidated(IdentityVanished)@99。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((20, 39), 70, 99),
                false,
            )],
            99,
            &m,
        );
        assert_eq!(d.len(), 2, "新身份 Observed + 旧身份 IdentityVanished");
        let old = book
            .get(&key_pan((50, 69), (70, 89)))
            .expect("旧 entry 保留（禁删除模拟失效）");
        assert_eq!(old.state, NestEventState::Invalidated);
        assert_eq!(
            old.invalidated_reason,
            Some(InvalidatedReason::IdentityVanished {
                cause: VanishCause::HypothesisRefuted
            })
        );
        assert_eq!(
            old.vanish_cause,
            Some(VanishCause::HypothesisRefuted),
            "成因入账本字段（票 #559 裁定：不是只写 dump 文本）"
        );
        assert_eq!(old.invalidated_at, Some(99));
        assert!(
            old.force_evidence.is_none(),
            "IdentityVanished 恒无力度证据"
        );
        assert!(
            matches!(
                old.revisions.last().unwrap().kind,
                LifecycleRevisionKind::Invalidated {
                    reason: InvalidatedReason::IdentityVanished {
                        cause: VanishCause::HypothesisRefuted
                    }
                }
            ),
            "原因码（含成因）入 revision 载荷（与 ForceOvertake 可区分）"
        );
        assert_eq!(book.len(), 2, "新身份建仓不受影响");
        let stats = book.settlement_stats();
        assert_eq!(stats.identity_vanished_refuted_count, 1, "(a) 类计 1");
        assert_eq!(stats.identity_vanished_seam_count, 0, "(b) 类不被污染");
        book.assert_invariants();
    }

    /// T6b 身份消失成因 (b)：**同锚换 C** 的观测接缝伪影（票 #559 裁定 (b) 类）。
    ///
    /// 场景与 T6 严格互补：`seg_a`/`b_center_start`/level/side 全同（**锚未变**），只有 C 段
    /// 左端从 70 换到 90（parser 的 pending 段推进到下一个 C，或滞后到账的完成事件另指一个
    /// C）。桥（除右端外全等）判**不同**身份 ⟹ 旧 key 本 prefix 不再产出 ⟹ 消失；但该锚
    /// 仍在产窗 ⟹ 假设没死 ⟹ [`VanishCause::ObservationSeam`]，并记下接手身份的 c 左端。
    ///
    /// 这是 BTC 100k 上 #559 §1.1(c) 5 例反例的合成最小复现：两类若混记，(b) 的寿命会被
    /// 当成「活假设存活了 N bar 后被证伪」进入寿命/反超率统计（裁定明令禁止）。
    #[test]
    fn t6b_identity_vanish_observation_seam() {
        let close_src = identity_close_src(120);
        let hist = vec![0.0; 120];
        let dif = vec![0.0; 120];
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        // as_of=89：锚 seg_a=(50,69)，C 段左端 70。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 69), 70, 89),
                false,
            )],
            89,
            &m,
        );
        assert_eq!(d.len(), 1, "仅 Observed");
        // as_of=99：同锚（seg_a 不变），C 左端换成 90 ⟹ 桥不判同 ⟹ 旧 key 消失。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 69), 90, 99),
                false,
            )],
            99,
            &m,
        );
        assert_eq!(
            d.len(),
            2,
            "新 C 身份 Observed + 旧 C 身份 IdentityVanished"
        );
        let old = book
            .get(&key_pan((50, 69), (70, 89)))
            .expect("旧 entry 保留");
        assert_eq!(
            old.vanish_cause,
            Some(VanishCause::ObservationSeam {
                successor_c_start: 90
            }),
            "接缝成因 + 接手身份 c 左端入账本字段"
        );
        assert_eq!(
            old.invalidated_reason,
            Some(InvalidatedReason::IdentityVanished {
                cause: VanishCause::ObservationSeam {
                    successor_c_start: 90
                }
            })
        );
        let stats = book.settlement_stats();
        assert_eq!(stats.identity_vanished_seam_count, 1, "(b) 类计 1");
        assert_eq!(
            stats.identity_vanished_refuted_count, 0,
            "(a) 类不被污染——两类分列是裁定的账本要求"
        );
        book.assert_invariants();
    }

    /// T7 完成时复核（024:24 机械表达——「用『曾经弱过』替代完成时复核」是 E2E §4.1:151
    /// 明令禁止项；复核用本 prefix 现算 force，不沿用 first_provable 旧值）。
    #[test]
    fn t7_completion_time_recheck() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        hist[70..=139].fill(-0.1); // c 窗恒弱：面积 7、柱峰 0.1
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        dif[70..=139].fill(-1.0);

        // (a) 完成窗仍弱 ⟹ Confirmed（structure_end = confirmed = 139，first_provable 不后移）。
        let mut book = NestLifecycleBook::new();
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                true,
            )],
            139,
            &m,
        );
        assert_eq!(d.len(), 3, "Supersedes + StructureCompleted + Confirmed");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Confirmed);
        assert_eq!(entry.first_provable_at, Some(129), "first_provable 不后移");
        assert_eq!(entry.structure_end_at, Some(139));
        assert_eq!(entry.confirmed_at, Some(139));
        assert_eq!(
            book.consumable_closed().len(),
            1,
            "Confirmed 进消费侧（Closed-only）"
        );

        // (b) 完成窗已反超 ⟹ 可从 Provisional 直接 Invalidated(ForceOvertake)，
        // 无需先经 StructureCompleted，且 confirmed_at 保持 None。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let mut book_b = NestLifecycleBook::new();
        let m = material(&hist, &dif, &close_src);
        book_b.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        let d = book_b.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                true,
            )],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        )));
        assert!(d
            .iter()
            .all(|r| !matches!(r.kind, LifecycleRevisionKind::Confirmed)));
        let entry_b = book_b.entries().next().unwrap().1;
        assert_eq!(entry_b.state, NestEventState::Invalidated);
        assert_eq!(
            entry_b.confirmed_at, None,
            "完成窗已反超不得 Confirmed（024:24）"
        );
        assert_eq!(
            entry_b.structure_end_at, None,
            "Q3 第二终局路径不伪造 StructureCompleted"
        );
        assert!(book_b.consumable_closed().is_empty());
        book_b.assert_invariants();
    }

    /// T8 trend 身份迁移豁免（白名单工程桥，模块头 090 登记 1）：收束记 Supersedes
    /// （链留痕、钟不动、无 Invalidated）；负面对照 seg_a 改变 ⟹ 白名单不越界 ⟹
    /// IdentityVanished。现行生产经 `feed_replay_bar` 逐 bar 两相喂：每根 bar 严格先活窗
    /// 观察、后完成信号；同 bar 闪现允许 Observed→StructureCompleted→终局同钟。
    /// `observed_at` 与 `first_provable_at` 均在首次实际 `advance` 写入，禁止回填，因此
    /// 不能构造 `first_provable_at < observed_at`。测试内 `feed_prefix_phases` adapter
    /// 仅为夹具展开：按给定 as_of 直喂，**不存在**「跳过非 trigger prefix」这条路径
    /// （票 #559 条件 C4：该尾句是 legacy `ReplayPrefixFeed` 语境的残留，那对符号已由
    /// #527 删除，故原句在夹具语境下悬空，此处按实际能力收窄）。
    #[test]
    fn t8_trend_identity_migration_whitelist() {
        let close_src = identity_close_src(200);
        let m = ForceMaterial::unavailable(&close_src); // 事件通道恒 Verified，无需力度序列
        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 149), false, 149, 149),
                false,
            )],
            149,
            &m,
        );
        // 确认收束 [120,149] → [120,155]：记 Supersedes（迁移链留痕、钟不动、无 Invalidated）。
        let d = book.advance(
            &[LifecycleObservation::event(
                trend_event((120, 155), true, 155, 155),
                false,
            )],
            155,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(
            matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 149)))
        );
        assert_eq!(
            book.len(),
            1,
            "迁移即替换（唯一 remove 点，仅 Provisional 可达）"
        );
        let entry = book.entries().next().unwrap().1;
        assert_eq!(
            entry.superseded_from,
            Some(key_trend((120, 149))),
            "迁移链留痕"
        );
        assert_eq!(entry.observed_at, 149, "钟不动");
        assert_eq!(entry.first_provable_at, Some(155));
        assert!(
            entry
                .revisions
                .iter()
                .all(|r| !matches!(r.kind, LifecycleRevisionKind::Invalidated { .. })),
            "白名单迁移不记 Invalidated"
        );

        // 负面对照：seg_a 改变 ⟹ 白名单不越界 ⟹ 旧 key 走身份消失路径。
        let mut seg_a_changed = trend_event((120, 169), false, 169, 169);
        seg_a_changed.seg_a = (60, 109);
        let d = book.advance(
            &[LifecycleObservation::event(seg_a_changed, false)],
            169,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::IdentityVanished { .. }
            }
        )));
        let new_entry = book
            .get(&LifecycleKey {
                seg_a: (60, 109),
                ..key_trend((120, 169))
            })
            .unwrap();
        assert!(
            matches!(
                new_entry.revisions.as_slice(),
                [LifecycleRevision {
                    kind: LifecycleRevisionKind::Observed,
                    ..
                }]
            ),
            "seg_a 改变 ⟹ 新身份建仓（无 Supersedes——白名单不越界）"
        );
        book.assert_invariants();
    }

    // ── 真实 provider 夹具（T11/T14 共用）──────────────────────────────────

    fn unit(start: usize, dir: Direction, lo: Tick, hi: Tick, ordinal: u64) -> LeveledMove {
        LeveledMove::from_unit(
            &UnitRange {
                start_index: start,
                end_index: start + 9,
                direction: dir,
                lo,
                hi,
            },
            ElementId { level: 0, ordinal },
        )
    }

    /// 复刻 level_view.rs 测试 extended_windows（含 R1 回试腿13）。
    fn extended_windows() -> (Vec<LeveledMove>, Vec<LeveledMove>) {
        use Direction::{Down, Up};
        let lower = vec![
            unit(0, Up, 90, 110, 0),
            unit(10, Down, 95, 115, 1),
            unit(20, Up, 98, 112, 2),
            unit(30, Down, 96, 116, 3),
            unit(40, Up, 130, 145, 4),
            unit(50, Down, 132, 148, 5),
            unit(60, Up, 135, 150, 6),
            unit(70, Down, 125, 140, 7),
            unit(80, Up, 155, 170, 8),
            unit(90, Down, 150, 165, 9),
            unit(100, Up, 160, 175, 10),
            unit(110, Down, 140, 155, 11),
            unit(120, Up, 180, 190, 12),
            // R1 回试腿（Down，低点 170 > w2.zg=165 不重回核心）——与腿12 构成对 w2 的三买。
            unit(130, Down, 170, 195, 13),
        ];
        let w0 = LeveledMove::compose(
            &lower[0..4],
            &[Center {
                zd: 98,
                zg: 110,
                dd: 90,
                gg: 116,
                start_index: 0,
                end_index: 39,
            }],
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        let w1 = LeveledMove::compose(
            &lower[4..8],
            &[Center {
                zd: 135,
                zg: 140,
                dd: 125,
                gg: 150,
                start_index: 40,
                end_index: 79,
            }],
            1,
            ElementId {
                level: 1,
                ordinal: 1,
            },
        );
        let w2 = LeveledMove::compose(
            &lower[8..12],
            &[Center {
                zd: 160,
                zg: 165,
                dd: 140,
                gg: 175,
                start_index: 80,
                end_index: 119,
            }],
            1,
            ElementId {
                level: 1,
                ordinal: 2,
            },
        );
        (vec![w0, w1, w2], lower)
    }

    /// pan 真实夹具：Consolidation 中枢 [20,49]（核心 [100,110]）+ 窄锚结构——
    /// A=[50,59]（Down 105→95 破核心）→ 回中枢段 [59,69]（Up 96→104 重回核心）→
    /// C episode [69,79]（Down 103→93 破核心新低）∪ [79,89]（Up 94→99 回拉不重回核心，
    /// episode 不复位）∪ [89,99]（Down 98→92 续创新低）。
    /// 力度：a 窗面积 20/柱峰 2.0/黄白线峰 5.0；c 一窗（[69,79]）全面更弱；
    /// c 延展段（[79,99]）取值由调用方定（T11 反超 / T14 保持弱）。
    #[allow(clippy::too_many_arguments)]
    fn pan_real_fixture(
        c_ext_hist: f64,
        c_ext_dif: f64,
    ) -> (
        Vec<Center>,
        Vec<Option<MoveKind>>,
        Vec<Segment>,
        Vec<Option<Direction>>,
        Vec<f64>,
        Vec<f64>,
        Vec<usize>,
    ) {
        let center = Center {
            zd: 100,
            zg: 110,
            dd: 90,
            gg: 120,
            start_index: 20,
            end_index: 49,
        };
        // 单中枢链 blocks=[Consolidation 0..0] ⟹ C_0 归 B₁ = Consolidation
        // （decompose.rs center_block_kind ownership 分区同口径）。
        let kinds = center_block_kind(
            1,
            &[MoveBlock {
                start_center: 0,
                end_center: 0,
                kind: MoveKind::Consolidation,
                dir: None,
                level_lift: 0,
                status: MoveStatus::Completed,
            }],
        );
        let segments = vec![
            Segment {
                direction: Direction::Down,
                start_index: 50,
                end_index: 59,
                start_price: 105,
                end_price: 95,
            },
            Segment {
                direction: Direction::Up,
                start_index: 59,
                end_index: 69,
                start_price: 96,
                end_price: 104,
            },
            Segment {
                direction: Direction::Down,
                start_index: 69,
                end_index: 79,
                start_price: 103,
                end_price: 93,
            },
            Segment {
                direction: Direction::Up,
                start_index: 79,
                end_index: 89,
                start_price: 94,
                end_price: 99,
            },
            Segment {
                direction: Direction::Down,
                start_index: 89,
                end_index: 99,
                start_price: 98,
                end_price: 92,
            },
        ];
        let anchors = self_anchors(&segments);
        let mut hist = vec![0.0; 120];
        hist[50..60].fill(-2.0); // a：面积 20、柱峰 2.0
        hist[70..80].fill(-0.5); // c 一窗：面积 5、柱峰 0.5
        hist[80..100].fill(c_ext_hist); // c 延展段：调用方定
        let mut dif = vec![0.0; 120];
        dif[50..60].fill(-5.0); // a：黄白线峰 5.0
        dif[70..80].fill(-1.0); // c 一窗：1.0
        dif[80..100].fill(c_ext_dif); // c 延展段：调用方定
        let close_src = identity_close_src(120);
        (vec![center], kinds, segments, anchors, hist, dif, close_src)
    }

    /// T11 pan 活窗真实反超（**勘误定案的唯一真实可达通道，必过**）：真实 provider
    /// 夹具——段序列/中枢经真实 `locate_pan_div_structure`（窄锚）锚定 c_start，
    /// 力度经真实 `segments_diverge_or` 现算；反超 ⟹ Invalidated(ForceOvertake) +
    /// ForceEvidence 可查账 + 终态留档不可消费。
    #[test]
    fn t11_real_provider_pan_live_force_overtake_auditable() {
        // c 延展段灌强柱：hist -3.0（面积 +60、柱峰 3.0）、dif -6.0（黄白线峰 6.0）。
        let (centers, kinds, segments, anchors, hist, dif, close_src) =
            pan_real_fixture(-3.0, -6.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        // as_of=79：真实定位产窗（窄锚 A=(50,59)、c_start=69 同锚），三通道成立
        // （5<20、1<5、0.5<2）⟹ first_provable=79。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 79);
        assert_eq!(windows.len(), 1, "单一身份活窗");
        assert_eq!(
            windows[0].seg_a,
            (50, 59),
            "窄锚 A 经真实 locate_pan_div_structure 锚定"
        );
        assert_eq!(
            windows[0].seg_c_live,
            (69, 79),
            "c_start_live 与完成后 seg_c.0 同锚（卡 §3）"
        );
        let obs: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, false))
            .collect();
        book.advance(&obs, 79, &m);
        assert_eq!(
            book.get(&key_pan((50, 59), (69, 79)))
                .unwrap()
                .first_provable_at,
            Some(79)
        );

        // as_of=99：活窗延展，真实现算三通道全假 ⟹ Invalidated(ForceOvertake) 可审计。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 99);
        assert_eq!(windows.len(), 1);
        assert_eq!(
            windows[0].seg_c_live,
            (69, 99),
            "活窗右端随 as_of 前进（设计内行为）"
        );
        let obs: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, false))
            .collect();
        let d = book.advance(&obs, 99, &m);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        )));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(entry.invalidated_at, Some(99));
        assert_eq!(entry.first_provable_at, Some(79), "first_provable 不后移");
        // ForceEvidence 可查账（US-03）：真实原语现算，三通道 c 全面反超 a。
        let ev = entry.force_evidence.expect("反超证据留档（裁定 #64 §4）");
        assert_eq!((ev.area_a, ev.area_c), (20.0, 65.0));
        assert_eq!((ev.dif_peak_a, ev.dif_peak_c), (5.0, 6.0));
        assert_eq!((ev.hist_peak_a, ev.hist_peak_c), (2.0, 3.0));
        // 终态留档不可消费（消费侧 Closed-only，裁定 #64 §2(a)）。
        assert!(
            book.consumable_closed().is_empty(),
            "Invalidated 可查账、不开放消费"
        );
        // 终态吸收（含桥匹配）：as_of=109 活窗延展 (69,109) 仍零输出。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 109);
        let obs: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, false))
            .collect();
        let d = book.advance(&obs, 109, &m);
        assert!(d.is_empty(), "终态吸收（含桥匹配到终态）零输出");
        book.assert_invariants();
    }

    /// T14 Unavailable 不判 Invalidated（#78 修复 1 锚定）：缺 hist/dif ⟹
    /// ForceUnavailable 注记、无 Invalidated、不写 first_provable；CoordinateMapFailed
    /// 子场景；同 as_of 注记幂等；数据补齐恢复推进至 Confirmed（无残留状态阻塞）。
    #[test]
    fn t14_unavailable_never_invalidates() {
        // c 延展段保持弱（面积 +2、柱峰 0.1、黄白线峰 0.5）——恢复推进应至 Confirmed。
        let (centers, kinds, segments, anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let mut book = NestLifecycleBook::new();

        // as_of=79：缺 hist/dif ⟹ MissingForceSeries 注记；无 Invalidated、不写 first_provable。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 79);
        let obs: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, false))
            .collect();
        let no_force = ForceMaterial::unavailable(&close_src);
        let d = book.advance(&obs, 79, &no_force);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable {
                reason: UnavailReason::MissingForceSeries
            }
        )));
        let key79 = key_pan((50, 59), (69, 79));
        let e = book.get(&key79).unwrap();
        assert_eq!(
            e.state,
            NestEventState::Provisional,
            "「不可验」≠「不再弱」"
        );
        assert_eq!(e.first_provable_at, None, "Unavailable 不写 first_provable");
        assert_eq!(e.invalidated_at, None, "Unavailable 不判 Invalidated");

        // 同 as_of 注记幂等：重复 advance 零新 revision。
        let d = book.advance(&obs, 79, &no_force);
        assert!(d.is_empty(), "同 as_of 注记幂等去重");
        assert_eq!(
            book.get(&key79).unwrap().revisions.len(),
            2,
            "Observed + 一条注记"
        );

        // as_of=89：坐标映射失败子场景（close_src 截断到 69 之前 ⟹ c 窗映射失败）
        // ⟹ CoordinateMapFailed 注记，仍不判 Invalidated。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 89);
        let obs89: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, false))
            .collect();
        let short_src = identity_close_src(69);
        let m_trunc = ForceMaterial {
            hist: Some(&hist),
            dif: Some(&dif),
            close_src: &short_src,
        };
        let d = book.advance(&obs89, 89, &m_trunc);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable {
                reason: UnavailReason::CoordinateMapFailed
            }
        )));
        assert_eq!(
            book.entries().next().unwrap().1.state,
            NestEventState::Provisional
        );

        // as_of=99：数据补齐 + 结构完成 ⟹ 恢复推进至 Confirmed（force_unavailable_at
        // 只作去重基准，不留残留状态阻塞）。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 99);
        let obs99: Vec<_> = windows
            .iter()
            .map(|w| LifecycleObservation::pan_live(*w, true))
            .collect();
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(&obs99, 99, &m);
        assert!(d
            .iter()
            .any(|r| matches!(r.kind, LifecycleRevisionKind::Confirmed)));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Confirmed);
        assert_eq!(
            entry.first_provable_at,
            Some(99),
            "补齐后 first_provable 正常写入"
        );
        assert_eq!(entry.structure_end_at, Some(99));
        assert_eq!(entry.confirmed_at, Some(99));
        assert_eq!(
            book.consumable_closed().len(),
            1,
            "Confirmed 可消费（Closed-only 边界内）"
        );
        // 谱系构建读面（裁定 #64 §2(a)「构建放开」）：全 entry 可见；消费侧仍 Closed-only。
        assert_eq!(book.lineage_nodes().len(), 1);
        // 全程零 Invalidated（Unavailable 从未假杀身份——#78 修复 1 语义）。
        assert!(book
            .entries()
            .all(|(_, e)| e.state != NestEventState::Invalidated));
        book.assert_invariants();
    }

    /// T16 从未构成（061:28「因为背驰如果没有创新高，是不存在的」）：结构完成时该活假设
    /// 从未写入首次可证时点 ⟹ 转入终态并挂 NeverConstituted，而非继续以活假设身份挂账。
    ///
    /// 反例注入在 (b)：同一夹具把 c 窗换成真弱 ⟹ 结构完成走 Confirmed、原因码不落
    /// NeverConstituted——(a) 的断言不是构造性恒真（不是「结构完成即挂新码」）。
    #[test]
    fn t16_never_constituted_on_structure_completion() {
        let close_src = identity_close_src(160);
        // a=[50,59]（Long = 向下离开）：面积 5、柱峰 0.5、黄白线峰 1.0——a 本身就弱。
        let mut hist = vec![0.0; 160];
        hist[50..=59].fill(-0.5);
        let mut dif = vec![0.0; 160];
        dif[50..=59].fill(-1.0);
        // c 活窗 [70, as_of] 恒强于 a（面积 ≥ 90、柱峰 3.0、黄白线峰 6.0）⟹ 三通道恒假
        // ⟹ first_provable 从未写入（「背驰没有创新高就不存在」的机械表达）。
        hist[70..=159].fill(-3.0);
        dif[70..=159].fill(-6.0);
        let m = material(&hist, &dif, &close_src);

        // (a) 结构未完成期间：诚实滞留活假设（只有 Observed，无终态钟）。
        let mut book = NestLifecycleBook::new();
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 99),
                false,
            )],
            99,
            &m,
        );
        assert_eq!(d.len(), 1, "结构未完成 ⟹ 仅 Observed");
        let alive = book.get(&key_pan((50, 59), (70, 99))).unwrap();
        assert_eq!(
            alive.state,
            NestEventState::Provisional,
            "结构未完成不提前判负"
        );
        assert_eq!(alive.first_provable_at, None, "从未可证");

        // 结构完成（ADR-0003：通道切换即完成信号）⟹ 转终态 + NeverConstituted。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 109),
                true,
            )],
            109,
            &m,
        );
        assert_eq!(d.len(), 3, "Supersedes + StructureCompleted + Invalidated");
        assert!(matches!(
            d[2].kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::NeverConstituted
            }
        ));
        let key = key_pan((50, 59), (70, 109));
        let entry = book.get(&key).expect("终态留档不删（谱系保留）");
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(
            entry.invalidated_reason,
            Some(InvalidatedReason::NeverConstituted)
        );
        assert_eq!(entry.first_provable_at, None, "从未构成 ⟹ 首次可证时点恒空");
        assert_eq!(entry.structure_end_at, Some(109));
        assert_eq!(entry.invalidated_at, Some(109), "结构完成即结算，不再挂账");
        assert_eq!(
            entry.revisions.len(),
            4,
            "Observed + Supersedes + StructureCompleted + Invalidated"
        );
        assert!(
            book.consumable_closed().is_empty(),
            "终态不进消费侧（Closed-only）"
        );
        // 力度证据入载荷（模块头 090 登记 4「原因码与力度证据入载荷」）：c 三通道全面强于 a
        // ——审计者据此复核「从未构成」结论（a 面积 10×0.5、c 面积 40×3.0）。
        let ev = entry.force_evidence.expect("从未构成留力度证据载荷");
        assert_eq!((ev.area_a, ev.area_c), (5.0, 120.0));
        assert_eq!((ev.dif_peak_a, ev.dif_peak_c), (1.0, 6.0));
        assert_eq!((ev.hist_peak_a, ev.hist_peak_c), (0.5, 3.0));

        // 终态吸收（禁复活）：同身份后续活窗延展零输出。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 119),
                true,
            )],
            119,
            &m,
        );
        assert!(d.is_empty(), "终态吸收零输出");
        assert_eq!(book.len(), 1, "终态不另立新 key");
        book.assert_invariants();

        // (b) 反例注入：同一时序、c 窗真弱（面积 2、柱峰 0.05、黄白线峰 0.5）⟹ 结构完成
        // 走 Confirmed，NeverConstituted 不落——(a) 的断言不是「结构完成即挂新码」的恒真。
        let mut hist_weak = vec![0.0; 160];
        hist_weak[50..=59].fill(-0.5);
        hist_weak[70..=159].fill(-0.05);
        let mut dif_weak = vec![0.0; 160];
        dif_weak[50..=59].fill(-1.0);
        dif_weak[70..=159].fill(-0.5);
        let m_weak = material(&hist_weak, &dif_weak, &close_src);
        let mut book_weak = NestLifecycleBook::new();
        book_weak.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 99),
                false,
            )],
            99,
            &m_weak,
        );
        let d = book_weak.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 109),
                true,
            )],
            109,
            &m_weak,
        );
        assert!(
            d.iter()
                .any(|r| matches!(r.kind, LifecycleRevisionKind::Confirmed)),
            "反例：c 窗真弱 ⟹ 结构完成走 Confirmed"
        );
        let entry_weak = book_weak.entries().next().unwrap().1;
        assert_eq!(entry_weak.state, NestEventState::Confirmed);
        assert_eq!(
            entry_weak.invalidated_reason, None,
            "反例：NeverConstituted 不落"
        );
        book_weak.assert_invariants();
    }

    /// T19 力度不可验的结构完成 prefix **不伪造**「从未构成」（#78「不可验 ≠ 不再弱」
    /// 优先于本票新增的结算路径；090 纪律：照实否定合格，伪造失败）。
    ///
    /// 该 prefix 既不写 structure_end_at 也不判负，身份滞留活假设；数据补齐后的完成信号
    /// 才结算 —— 不可验只**推迟**结算，不制造结论。数据始终不补则永久滞留（已知残留，
    /// 挂账不在本票范围）。
    #[test]
    fn t19_unavailable_force_at_completion_does_not_fabricate_never_constituted() {
        let close_src = identity_close_src(160);
        // 同 T16 夹具：a 弱、c 恒强 ⟹ 力度可验时该身份必属「从未构成」。
        let mut hist = vec![0.0; 160];
        hist[50..=59].fill(-0.5);
        hist[70..=159].fill(-3.0);
        let mut dif = vec![0.0; 160];
        dif[50..=59].fill(-1.0);
        dif[70..=159].fill(-6.0);

        let mut book = NestLifecycleBook::new();
        // as_of=99：结构完成信号已到，但力度序列缺失 ⟹ 只留注记，不结算。
        let no_force = ForceMaterial::unavailable(&close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 99),
                true,
            )],
            99,
            &no_force,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable {
                reason: UnavailReason::MissingForceSeries
            }
        )));
        assert!(
            d.iter()
                .all(|r| !matches!(r.kind, LifecycleRevisionKind::Invalidated { .. })),
            "力度不可验 ⟹ 不判负（不伪造从未构成）"
        );
        let entry = book.get(&key_pan((50, 59), (70, 99))).unwrap();
        assert_eq!(
            entry.state,
            NestEventState::Provisional,
            "滞留活假设，诚实存疑"
        );
        assert_eq!(entry.invalidated_reason, None);
        assert_eq!(
            entry.structure_end_at, None,
            "不可验 prefix 的完成信号不留痕（#78 原语义）"
        );

        // as_of=109：数据补齐 + 完成信号重发 ⟹ 此时才结算为「从未构成」。
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 109),
                true,
            )],
            109,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::NeverConstituted
            }
        )));
        let entry = book.get(&key_pan((50, 59), (70, 109))).unwrap();
        assert_eq!(
            entry.invalidated_reason,
            Some(InvalidatedReason::NeverConstituted)
        );
        assert_eq!(
            entry.structure_end_at,
            Some(109),
            "结算推迟到数据补齐的那一 prefix"
        );
        book.assert_invariants();
    }

    /// T18 桥匹配分支的时点倒退拒绝（票 #425 补既有覆盖缺口）：T15 走的是**直接匹配**
    /// 分支——被拒 key 已在 book 内（advance 第 2 步）；本测试走**桥匹配**分支——被拒 key
    /// 尚未建仓、经白名单桥匹配到既有身份后在第 1 步内被拒（右端不同 ⟹ 新 key）。
    /// 「新 key 未建仓」这条断言把分支钉死在桥匹配上：直接匹配分支要求 key 已存在。
    #[test]
    fn t18_retrograde_rejected_on_bridge_match_branch() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        hist[70..=139].fill(-0.25);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        dif[70..=139].fill(-1.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        // as_of=100 建仓（三通道成立 ⟹ 同 prefix 写 first_provable）。
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 100),
                false,
            )],
            100,
            &m,
        );
        let established = key_pan((50, 59), (70, 100));
        let before = book.get(&established).unwrap().clone();

        // 倒退 as_of=90 喂**右端不同**的同身份窗 (70,110)：该 key 不在 book 内 ⟹ 走桥匹配
        // 分支 ⟹ 显式拒绝（不迁移、不建仓、零 revision）。
        let bridged = key_pan((50, 59), (70, 110));
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 110),
                false,
            )],
            90,
            &m,
        );
        assert!(d.is_empty(), "桥匹配分支倒退 prefix 零 revision");
        assert!(
            book.get(&bridged).is_none(),
            "被拒 key 未建仓——本例确实走桥匹配分支（直接匹配分支要求 key 已存在）"
        );
        assert_eq!(book.len(), 1, "桥匹配倒退不另立新 key");
        assert_eq!(
            book.get(&established).unwrap(),
            &before,
            "既有 entry 零改动（含钟与 last_as_of）"
        );
        assert_eq!(
            book.retrograde_rejections(),
            &[RetrogradeRejection {
                key: bridged,
                last_as_of: 100,
                rejected_as_of: 90
            }],
            "注记记被拒的新 key 与既有身份的 last_as_of"
        );
        assert_eq!(
            book.get(&established).unwrap().state,
            NestEventState::Provisional,
            "桥匹配倒退不制造 IdentityVanished"
        );

        // 合法前进照常：同一窗 as_of=110 ⟹ 桥迁移 Supersedes（拒绝不留残疾）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 110),
                false,
            )],
            110,
            &m,
        );
        assert_eq!(d.len(), 1);
        assert!(
            matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == established)
        );
        assert_eq!(book.retrograde_rejections().len(), 1, "合法前进无新注记");
        book.assert_invariants();
    }

    /// T17 两类背驰否证在同一 book 内可分辨（票 #425 验收：不得靠同一断言蒙混）：
    /// **被反超**（061:26 曾构成、后被否证）与**从未构成**（061:28 根本未构成）并存，
    /// 各自的原因码、首次可证时点、结构完成时点三项两两相反。
    ///
    /// 两身份取不同级别 + 互不重叠的力度窗（level 1 / level 2），彼此不经白名单桥判同。
    #[test]
    fn t17_never_constituted_distinguishable_from_force_overtake() {
        let close_src = identity_close_src(200);
        let mut hist = vec![0.0; 200];
        let mut dif = vec![0.0; 200];
        // 身份 A（level 1，被反超）：a=[10,19] 面积 20/柱峰 2.0/黄白线峰 5.0；
        // c 先弱（[30,49] 面积 5/柱峰 0.25/黄白线峰 1.0），延展段 [50,69] 全面反超。
        hist[10..=19].fill(-2.0);
        dif[10..=19].fill(-5.0);
        hist[30..=49].fill(-0.25);
        dif[30..=49].fill(-1.0);
        hist[50..=69].fill(-3.0);
        dif[50..=69].fill(-6.0);
        // 身份 B（level 2，从未构成）：a=[110,119] 面积 5/柱峰 0.5/黄白线峰 1.0；
        // c=[130,…] 恒强于 a ⟹ 三通道恒假 ⟹ first_provable 从未写入。
        hist[110..=119].fill(-0.5);
        dif[110..=119].fill(-1.0);
        hist[130..=159].fill(-3.0);
        dif[130..=159].fill(-6.0);
        let m = material(&hist, &dif, &close_src);

        let win_a = |live_end: usize| PanLiveWindow {
            level: 1,
            side: Side::Long,
            seg_a: (10, 19),
            seg_c_live: (30, live_end),
            b_center_start: 5,
            gap_len: 0,
        };
        let win_b = |live_end: usize| PanLiveWindow {
            level: 2,
            side: Side::Long,
            seg_a: (110, 119),
            seg_c_live: (130, live_end),
            b_center_start: 105,
            gap_len: 0,
        };
        let key_of = |w: PanLiveWindow| LifecycleKey {
            level: w.level,
            side: w.side,
            kind: NestDivergenceKind::Consolidation,
            seg_a: w.seg_a,
            seg_c_full: w.seg_c_live,
            b_center_start: w.b_center_start,
        };

        // as_of=160：A 三通道成立 ⟹ first_provable；B 恒假 ⟹ 仅 Observed。
        let mut book = NestLifecycleBook::new();
        let d = book.advance(
            &[
                LifecycleObservation::pan_live(win_a(49), false),
                LifecycleObservation::pan_live(win_b(149), false),
            ],
            160,
            &m,
        );
        assert_eq!(d.len(), 3, "A: Observed + FirstProvable；B: Observed");

        // as_of=170：A 活窗延展被反超（结构未完成）；B 结构完成而从未可证。
        let d = book.advance(
            &[
                LifecycleObservation::pan_live(win_a(69), false),
                LifecycleObservation::pan_live(win_b(159), true),
            ],
            170,
            &m,
        );
        assert_eq!(
            d.iter()
                .filter(|r| matches!(r.kind, LifecycleRevisionKind::Invalidated { .. }))
                .count(),
            2,
            "两条否证各产一条终态修订"
        );

        let a = book.get(&key_of(win_a(69))).expect("被反超身份留档不删");
        let b = book.get(&key_of(win_b(159))).expect("从未构成身份留档不删");
        assert_ne!(a.key, b.key, "两身份不同 key");
        assert_eq!(a.state, NestEventState::Invalidated);
        assert_eq!(b.state, NestEventState::Invalidated);

        // ① 原因码相反。
        assert_eq!(a.invalidated_reason, Some(InvalidatedReason::ForceOvertake));
        assert_eq!(
            b.invalidated_reason,
            Some(InvalidatedReason::NeverConstituted)
        );
        assert_ne!(
            a.invalidated_reason, b.invalidated_reason,
            "两类否证不得共用一个原因码"
        );
        // ② 首次可证时点相反（教义分界：曾构成 vs 根本未构成）。
        assert_eq!(a.first_provable_at, Some(160), "被反超 = 曾构成（061:26）");
        assert_eq!(b.first_provable_at, None, "从未构成 = 根本未构成（061:28）");
        // ③ 结构完成时点相反（被反超无须等结构完成；从未构成恰在结构完成时结算）。
        assert_eq!(a.structure_end_at, None);
        assert_eq!(b.structure_end_at, Some(170));

        // 修订载荷同样可分辨（诊断查账走 revisions，不只走 entry 字段）。
        assert!(matches!(
            a.revisions.last().unwrap().kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        ));
        assert!(matches!(
            b.revisions.last().unwrap().kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::NeverConstituted
            }
        ));

        // 分桶恰好一对一（不是「两条都落进同一桶」的蒙混）。
        let by_reason = |reason: InvalidatedReason| {
            book.entries()
                .filter(|(_, e)| e.invalidated_reason == Some(reason))
                .count()
        };
        assert_eq!(by_reason(InvalidatedReason::ForceOvertake), 1);
        assert_eq!(by_reason(InvalidatedReason::NeverConstituted), 1);
        assert_eq!(
            book.entries()
                .filter(|(_, e)| e.vanish_cause.is_some())
                .count(),
            0,
            "两者都不是身份消失（成因字段恒空）"
        );
        assert_eq!(book.len(), 2, "终态留档不删");
        assert!(book.consumable_closed().is_empty(), "终态一律不进消费侧");
        book.assert_invariants();
    }

    /// T15 倒退显式拒绝（#78 修复 2 锚定）：倒退 prefix 拒绝 + 注记 + entry 零改动；
    /// 同 as_of 幂等；倒退不产生 IdentityVanished；合法前进照常。
    #[test]
    fn t15_retrograde_prefix_explicitly_rejected() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        hist[70..=139].fill(-0.25);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        dif[70..=139].fill(-1.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        // as_of=100：建仓 + first_provable（30×0.25=7.5<20、0.25<2、1<5 三通道成立）。
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 100),
                false,
            )],
            100,
            &m,
        );
        let key = key_pan((50, 59), (70, 100));
        let before = book.get(&key).unwrap().clone();

        // 倒退 as_of=90 喂同 key ⟹ 显式拒绝：零 revision、entry 零改动、注记在案。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 100),
                false,
            )],
            90,
            &m,
        );
        assert!(d.is_empty(), "倒退 prefix 零 revision（禁静默吸收）");
        assert_eq!(book.get(&key).unwrap(), &before, "entry 零改动");
        assert_eq!(
            book.retrograde_rejections(),
            &[RetrogradeRejection {
                key,
                last_as_of: 100,
                rejected_as_of: 90
            }]
        );

        // 倒退 prefix 不制造 IdentityVanished：空投喂 @90，last_as_of=100 > 90 的身份跳过。
        let d = book.advance(&[], 90, &m);
        assert!(d.is_empty());
        assert_eq!(
            book.get(&key).unwrap().state,
            NestEventState::Provisional,
            "倒退不制造 IdentityVanished"
        );

        // 同 as_of 幂等：重复 advance 零 delta。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 100),
                false,
            )],
            100,
            &m,
        );
        assert!(d.is_empty(), "同 as_of 幂等零 delta");

        // 合法前进照常：as_of=110 活窗延展（桥迁移 Supersedes，无新注记）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 110),
                false,
            )],
            110,
            &m,
        );
        assert_eq!(d.len(), 1);
        assert!(matches!(
            d[0].kind,
            LifecycleRevisionKind::Supersedes { .. }
        ));
        assert_eq!(book.retrograde_rejections().len(), 1, "合法前进无新注记");
        book.assert_invariants();
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 回放喂数出口（票 #426 主接缝；ADR-0003 通道切换 = 结构完成）
    // ═══════════════════════════════════════════════════════════════════════

    /// 把夹具段序列还原为 `LowerLeg`（喂数出口按生产口径自 legs 取段，测试须从 legs 侧喂；
    /// `leg_as_segment` 的逆构造——`id` 不进段，取序号占位）。
    fn legs_of(segments: &[Segment]) -> Vec<LowerLeg> {
        segments
            .iter()
            .enumerate()
            .map(|(index, segment)| LowerLeg {
                id: ElementId {
                    level: 0,
                    ordinal: index as u64,
                },
                direction: segment.direction,
                start_index: segment.start_index,
                end_index: segment.end_index,
                lo: segment.start_price.min(segment.end_price),
                hi: segment.start_price.max(segment.end_price),
            })
            .collect()
    }

    /// 测试夹具 adapter：把「逐 run 结构 + 完成事件」展开为 [`PanProviderPhase`] 两相后喂入。
    ///
    /// 完成相的 lower 证明按 `interval_b` 右端在 legs 上查证（夹具自洽 ⟹ 命中）；查不到时
    /// 用**显式合成占位**（`ordinal = u64::MAX`）并保持三钟同 as_of——这是合成夹具的诚实
    /// 声明（如 F8 的 trend 域事件，其在完成相被域过滤，lower 证明不参与任何判定）。
    /// 生产路径**不走**本 adapter：p123 从 tower 查证 lower unit，查不到即报错停线。
    fn feed_prefix_phases(
        book: &mut NestLifecycleBook,
        runs: &[PanLiveRun<'_>],
        completion_events: &[NestCandidateEvent],
        as_of: usize,
        material: &ForceMaterial,
    ) -> (Vec<LifecycleRevision>, ReplayFeedStats) {
        let mut phases: Vec<PanProviderPhase> = provide_replay_live_windows(runs, as_of)
            .into_iter()
            .map(PanProviderPhase::Live)
            .collect();
        for event in completion_events {
            let leg = runs
                .iter()
                .flat_map(|run| run.legs.iter())
                .find(|leg| leg.end_index == event.interval_b.1);
            let (completed_lower_id, completed_at) = match leg {
                Some(leg) => (leg.id, leg.end_index),
                None => (
                    ElementId {
                        level: event.level.saturating_sub(1),
                        ordinal: u64::MAX,
                    },
                    as_of,
                ),
            };
            phases.push(PanProviderPhase::Completed(PanCompletionEvent {
                event: *event,
                completed_lower_id,
                completed_at,
            }));
        }
        feed_replay_bar(
            book,
            &ReplayBarFeed {
                as_of,
                phases: &phases,
            },
            material,
        )
    }

    /// 测试夹具：把活窗/完成事件直接组装为两相（逐 bar 出口的最小构造）。
    fn bar_phases(
        live_windows: &[PanLiveWindow],
        completion_events: &[NestCandidateEvent],
        completed_at: usize,
    ) -> Vec<PanProviderPhase> {
        let mut phases: Vec<PanProviderPhase> = live_windows
            .iter()
            .copied()
            .map(PanProviderPhase::Live)
            .collect();
        for event in completion_events {
            phases.push(PanProviderPhase::Completed(PanCompletionEvent {
                event: *event,
                completed_lower_id: ElementId {
                    level: event.level.saturating_sub(1),
                    ordinal: 0,
                },
                completed_at,
            }));
        }
        phases
    }

    /// F1 喂数出口的行进中通道：给定某一前缀上数据源的产出（级别/中枢/块类型/腿），
    /// 出口自建观察记录、喂入账本、返回该步变更集。
    ///
    /// 与 T11 同夹具同 as_of ⟹ 出口路径与直接调 `provide_pan_live_windows` + `advance`
    /// 的既有路径同判（出口不引入第二产窗法）。
    #[test]
    fn f1_replay_feed_live_channel_emits_observation() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-3.0, -6.0);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let (delta, stats) = feed_prefix_phases(&mut book, &runs, &[], 79, &m);
        assert_eq!(stats.live_windows, 1, "行进中通道产出单一活窗身份");
        assert_eq!(stats.completion_events, 0);
        assert_eq!(stats.channel_switches, 0, "无完成事件 ⟹ 不切换");
        assert_eq!(delta.len(), 2, "Observed + FirstProvable");
        let entry = book.get(&key_pan((50, 59), (69, 79))).expect("活假设建仓");
        assert_eq!(entry.observed_at, 79);
        assert_eq!(entry.first_provable_at, Some(79));
        assert_eq!(entry.structure_end_at, None, "行进中通道不给结构完成信号");
        book.assert_invariants();
    }

    /// 完成事件通道的盘整事件（字段取值对齐 `level_view.rs:869-884` 盘整分支：
    /// `interval_b = structure.seg_c`、`b_center_start = centers[center_index].start_index`）。
    fn pan_event(
        seg_a: (usize, usize),
        interval_b: (usize, usize),
        confirmed: bool,
        as_of: usize,
    ) -> NestCandidateEvent {
        NestCandidateEvent {
            level: 1,
            side: Side::Long,
            kind: NestDivergenceKind::Consolidation,
            seg_a,
            interval_b,
            interval_a: (20, 49),
            divergence_confirmed: confirmed,
            turn_source: interval_b.1,
            judge_at: as_of,
            provider_window: (20, 119),
            intake_fallback: false,
            b_center_start: 20,
        }
    }

    /// F2 通道切换 = 结构完成（ADR-0003）：同一身份从行进中通道转入完成事件通道时，
    /// 结构完成信号被置真；此后该身份不再产生迁移修订（完成即停延展）。
    #[test]
    fn f2_channel_switch_sets_structure_completed_and_stops_extension() {
        // c 延展段保持弱 ⟹ 完成时复核仍弱 ⟹ Confirmed。
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];

        // as_of=79：仅行进中通道 ⟹ 结构完成信号未置。
        feed_prefix_phases(&mut book, &runs, &[], 79, &m);
        assert_eq!(
            book.get(&key_pan((50, 59), (69, 79)))
                .unwrap()
                .structure_end_at,
            None
        );

        // as_of=99：完成事件通道首次产出该身份 ⟹ 行进中窗让位、结构完成置真。
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let (delta, stats) = feed_prefix_phases(&mut book, &runs, &events, 99, &m);
        assert_eq!(stats.completion_events, 1);
        assert_eq!(
            stats.channel_switches, 1,
            "同一身份在完成事件通道产出 ⟹ 行进中窗让位"
        );
        assert!(
            delta
                .iter()
                .any(|r| matches!(r.kind, LifecycleRevisionKind::StructureCompleted)),
            "结构完成信号由通道切换给出（不另造判据）"
        );
        let entry = book
            .get(&key_pan((50, 59), (69, 99)))
            .expect("桥迁移后新键");
        assert_eq!(entry.structure_end_at, Some(99));
        assert_eq!(entry.state, NestEventState::Confirmed);

        // as_of=109：行进中窗仍在产（右端追 as_of），但身份已终态 ⟹ 停止喂入、零延展修订。
        let revisions_before = entry.revisions.len();
        let (delta, stats) = feed_prefix_phases(&mut book, &runs, &[], 109, &m);
        assert_eq!(stats.live_windows, 1, "数据源仍产窗（未改数据源）");
        assert_eq!(
            stats.extension_suppressed, 1,
            "完成即停延展：终态身份不再喂行进中窗"
        );
        assert!(delta.is_empty(), "零延展修订");
        assert_eq!(
            book.get(&key_pan((50, 59), (69, 99)))
                .unwrap()
                .revisions
                .len(),
            revisions_before,
            "修订链不再增长"
        );
        book.assert_invariants();
    }

    /// F3 通道切换处的**力度跃变**（规格 Implementation Decisions §「两通道的比较窗不同」）：
    /// 行进中通道现算 `[c_start, as_of]`、完成事件通道取事件自带布尔口径，两条求值路径在
    /// 切换那一刻**可能翻转**；同 trigger 必须先处理活窗，再处理完成信号。
    ///
    /// 三臂分别钉住：活窗仍弱→完成事件判假、两路同真、活窗已反超→完成事件判真。第三臂
    /// 依 2026-07-28 Q3 裁定从 Provisional 直接 `ForceOvertake`，完成事件不得把它救回。
    #[test]
    fn f3_channel_switch_force_jump_and_q1_order_are_visible() {
        for (c_ext_hist, c_ext_dif, event_confirmed, expect_overtake) in [
            (-0.1, -0.5, false, true),
            (-0.1, -0.5, true, false),
            (-3.0, -6.0, true, true),
        ] {
            let (centers, kinds, segments, _anchors, hist, dif, close_src) =
                pan_real_fixture(c_ext_hist, c_ext_dif);
            let legs = legs_of(&segments);
            let m = material(&hist, &dif, &close_src);
            let runs = [PanLiveRun {
                level: 1,
                centers: &centers,
                kinds: &kinds,
                legs: &legs,
            }];
            let mut book = NestLifecycleBook::new();
            feed_prefix_phases(&mut book, &runs, &[], 79, &m);
            assert_eq!(
                book.get(&key_pan((50, 59), (69, 79)))
                    .unwrap()
                    .first_provable_at,
                Some(79),
                "切换前已可证（反超定义要求曾构成）"
            );
            let events = [pan_event((50, 59), (69, 99), event_confirmed, 99)];
            let delta = feed_prefix_phases(&mut book, &runs, &events, 99, &m).0;
            let overtaken = delta.iter().any(|r| {
                matches!(
                    r.kind,
                    LifecycleRevisionKind::Invalidated {
                        reason: InvalidatedReason::ForceOvertake
                    }
                )
            });
            let confirmed = delta
                .iter()
                .any(|r| matches!(r.kind, LifecycleRevisionKind::Confirmed));
            assert_eq!(
                overtaken, expect_overtake,
                "live_ext=({c_ext_hist},{c_ext_dif}) event={event_confirmed} ⟹ 反超={expect_overtake}"
            );
            assert_eq!(confirmed, !expect_overtake, "两臂结论互斥");
            if c_ext_hist == -3.0 {
                let entry = book.entries().next().unwrap().1;
                assert_eq!(
                    entry.structure_end_at, None,
                    "Q3：活窗先判反超时允许 Provisional 直接终局，不伪造 StructureCompleted"
                );
            }
            book.assert_invariants();
        }
    }

    /// F4 两类否证终局经喂数出口分别落码、终态留档不删（061:26 被反超 vs 061:28 从未构成）。
    ///
    /// 两臂**只差 a 段力度**（其余时序/结构/事件逐字相同）：a 强 ⟹ 曾可证 ⟹ 被反超；
    /// a 弱 ⟹ 从未可证 ⟹ 从未构成。同一断言组在两臂上给出相反原因码 ⟹ 不靠同一断言蒙混。
    #[test]
    fn f4_two_falsification_terminals_land_distinct_reasons() {
        let mut outcomes = Vec::new();
        for a_strong in [true, false] {
            let (centers, kinds, segments, _anchors, mut hist, mut dif, close_src) =
                pan_real_fixture(-3.0, -6.0);
            if !a_strong {
                // a 段改弱（面积 5、柱峰 0.5、黄白线峰 1.0）⟹ c 自首个前缀起即强于 a
                // ⟹ first_provable 从未写入。结构定位不读力度 ⟹ 活窗身份两臂相同。
                hist[50..60].fill(-0.5);
                dif[50..60].fill(-1.0);
            }
            let legs = legs_of(&segments);
            let m = material(&hist, &dif, &close_src);
            let runs = [PanLiveRun {
                level: 1,
                centers: &centers,
                kinds: &kinds,
                legs: &legs,
            }];
            let mut book = NestLifecycleBook::new();
            feed_prefix_phases(&mut book, &runs, &[], 79, &m);
            let events = [pan_event((50, 59), (69, 99), false, 99)];
            feed_prefix_phases(&mut book, &runs, &events, 99, &m);
            let entry = book
                .get(&key_pan((50, 59), (69, 99)))
                .expect("终态留档不删（谱系保留，禁删除模拟失效）");
            assert_eq!(entry.state, NestEventState::Invalidated);
            assert!(
                entry.force_evidence.is_some(),
                "两类力度否证均现算证据入载荷（可回溯复核）"
            );
            assert!(book.consumable_closed().is_empty(), "终态不进消费侧");
            book.assert_invariants();
            outcomes.push((entry.invalidated_reason, entry.first_provable_at.is_some()));
        }
        assert_eq!(
            outcomes,
            vec![
                (Some(InvalidatedReason::ForceOvertake), true),
                (Some(InvalidatedReason::NeverConstituted), false),
            ],
            "曾构成 ⟹ 被反超；从未构成 ⟹ 新原因码，两码在 first_provable 上严格互补"
        );
    }

    /// F5 喂数节拍：时点倒退被显式拒绝，账本零改动且拒绝有注记可查（#78 修复 2 语义
    /// 经出口透出）。
    #[test]
    fn f5_retrograde_prefix_rejected_with_zero_book_change() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let mut book = NestLifecycleBook::new();
        feed_prefix_phases(&mut book, &runs, &[], 89, &m);
        let snapshot = book.clone();
        let (delta, stats) = feed_prefix_phases(&mut book, &runs, &[], 79, &m);
        assert!(delta.is_empty(), "倒退 prefix 零 revision");
        assert_eq!(stats.retrograde_rejected, 1, "拒绝有注记可查（非静默吸收）");
        let rejection = *book.retrograde_rejections().last().unwrap();
        assert_eq!((rejection.last_as_of, rejection.rejected_as_of), (89, 79));
        // 账本零改动：除注记向量外 entry 全等（注记本身是审计面，不是账本状态）。
        assert_eq!(
            book.entries().collect::<Vec<_>>(),
            snapshot.entries().collect::<Vec<_>>(),
            "倒退喂入后 entry 逐字段零改动"
        );
        book.assert_invariants();
    }

    /// F6 侧车零反流（合成事件等价性护栏）：喂数出口对完成事件通道的入参**只读**；
    /// 出口前后的 Copy 事件流以 `PartialEq` 比较，并比较其 Debug 表示。这不是生产
    /// 序列化字节护栏。
    #[test]
    fn f6_feed_exit_is_sidecar_events_equivalent() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        // 不挂账本（基线）：入参原样。
        let baseline = events;
        // 挂账本：同一入参过一遍出口。
        let mut book = NestLifecycleBook::new();
        feed_prefix_phases(&mut book, &runs, &events, 99, &m);
        assert_eq!(events, baseline, "Copy 后事件流等价（PartialEq）");
        assert_eq!(
            format!("{events:?}"),
            format!("{baseline:?}"),
            "事件流 Debug 表示相等"
        );
        assert!(!book.is_empty(), "账本确实被喂到（护栏不是空跑）");
        book.assert_invariants();
    }

    /// F7 「完成即停延展」的跳过是**纯成本优化**：终态身份跳过喂入与照喂逐位同 book
    /// （终态吸收 ⟹ 零 revision、`last_as_of` 不动、身份消失扫描只看 Provisional）。
    ///
    /// 这条把 `terminal_bridge_hit` 的等价性声明变成可证伪断言——若终态吸收哪天不再吸收，
    /// 两侧 book 立刻不等。
    #[test]
    fn f7_terminal_skip_equals_feeding_terminal_identity() {
        let (centers, kinds, segments, anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let mut book = NestLifecycleBook::new();
        feed_prefix_phases(&mut book, &runs, &[], 79, &m);
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        feed_prefix_phases(&mut book, &runs, &events, 99, &m);
        assert_eq!(
            book.get(&key_pan((50, 59), (69, 99))).unwrap().state,
            NestEventState::Confirmed,
            "前置：身份已进终态"
        );
        // A 侧：出口跳过喂入（stats.extension_suppressed 记账）。
        let mut skipped = book.clone();
        let (delta_skip, stats) = feed_prefix_phases(&mut skipped, &runs, &[], 109, &m);
        assert_eq!(stats.extension_suppressed, 1);
        // B 侧：绕过出口、把同一活窗照喂进 advance。
        let mut fed = book.clone();
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 109);
        assert_eq!(
            windows.len(),
            1,
            "数据源确实仍在产该窗（跳过不是因为没产出）"
        );
        let observations: Vec<_> = windows
            .iter()
            .map(|window| LifecycleObservation::pan_live(*window, false))
            .collect();
        let delta_fed = fed.advance(&observations, 109, &m);
        assert_eq!(delta_skip, delta_fed, "两侧变更集逐字段相等（均为空）");
        assert_eq!(skipped, fed, "两侧 book 逐字段相等 ⟹ 跳过是纯成本优化");
    }

    /// F8 范围边界：本出口的完成事件通道**只取盘整域**（票 #426 只接盘整背驰活假设；
    /// trend 域不在范围——模块头 090 登记）。同一批入参里混入 trend 事件时，它既不计入
    /// 完成事件数，也不在账本里建仓。
    ///
    /// 对照臂：把同一事件的 `kind` 换成 `Consolidation` ⟹ 立刻计入并建仓 ⟹ 本断言不是
    /// 「事件从来不建仓」的恒真。
    #[test]
    fn f8_completion_channel_takes_consolidation_domain_only() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        // trend 域事件（seg_a/中枢均与活窗身份无关，单独成身份）。
        let mut trend = pan_event((120, 129), (130, 139), true, 139);
        trend.kind = NestDivergenceKind::Trend;
        let mut book = NestLifecycleBook::new();
        let (_, stats) = feed_prefix_phases(&mut book, &runs, &[trend], 139, &m);
        assert_eq!(stats.completion_events, 0, "trend 域不计入完成事件通道");
        assert!(
            book.entries()
                .all(|(key, _)| key.kind == NestDivergenceKind::Consolidation),
            "trend 域事件不在账本建仓"
        );

        // 对照臂：先在前一 trigger 建活身份；盘整域完成事件随后正常计入并结算。
        let pan = pan_event((50, 59), (69, 99), true, 99);
        let mut book_pan = NestLifecycleBook::new();
        feed_prefix_phases(&mut book_pan, &runs, &[], 79, &m);
        let (_, stats_pan) = feed_prefix_phases(&mut book_pan, &runs, &[pan], 99, &m);
        assert_eq!(stats_pan.completion_events, 1);
        assert!(
            book_pan.get(&key_pan((50, 59), (69, 99))).is_some(),
            "盘整域事件命中先前活身份后正常结算（对照臂）"
        );
    }

    /// F9a Q1/Q2 回归：同一 trigger 首次同时看见活窗与完成事件时，严格按
    /// Observed→完成信号的顺序照实结算；零寿命是合法闪现子集，不得等下一 trigger 重发。
    #[test]
    fn f9a_same_trigger_observe_then_complete_is_flash_lifetime() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let mut book = NestLifecycleBook::new();

        let (delta, stats) = feed_prefix_phases(&mut book, &runs, &events, 99, &m);
        assert_eq!(stats.completion_signals, 1, "首完成不得丢出分母");
        assert_eq!(stats.channel_switches, 1, "同 trigger 观察后立即切完成通道");
        assert!(matches!(delta[0].kind, LifecycleRevisionKind::Observed));
        assert!(matches!(
            delta[delta.len() - 2].kind,
            LifecycleRevisionKind::StructureCompleted
        ));
        assert!(matches!(
            delta[delta.len() - 1].kind,
            LifecycleRevisionKind::Confirmed
        ));
        let completed = book
            .get(&key_pan((50, 59), (69, 99)))
            .expect("闪现身份须留完整链");
        assert_eq!(completed.state, NestEventState::Confirmed);
        assert_eq!(completed.observed_at, 99);
        assert_eq!(completed.structure_end_at, Some(99));
        assert_eq!(completed.confirmed_at, Some(99));
        let settlement = book.settlement_stats();
        assert_eq!(settlement.flash_terminal_count, 1);
        assert_eq!(settlement.nonflash_lifetime.count, 0);
    }

    /// F9b Q1「不丢任何首完成」：即使身份已由 Q3 的 Provisional→ForceOvertake 提前终局，
    /// 后到的真实首完成仍须进入唯一分母；终态吸收不等于完成信号从未发生。
    #[test]
    fn f9b_first_completion_counts_after_provisional_force_overtake() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-3.0, -6.0);
        let legs = legs_of(&segments);
        let m = material(&hist, &dif, &close_src);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let mut book = NestLifecycleBook::new();
        feed_prefix_phases(&mut book, &runs, &[], 79, &m);
        feed_prefix_phases(&mut book, &runs, &[], 99, &m);
        let terminal = book.entries().next().unwrap().1;
        assert_eq!(
            terminal.invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );
        assert_eq!(
            terminal.structure_end_at, None,
            "Q3 第二终局路径不经结构完成"
        );

        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let (_, stats) = feed_prefix_phases(&mut book, &[], &events, 109, &m);
        assert_eq!(stats.completion_signals, 1, "首完成分母覆盖已提前终局身份");
        assert_eq!(stats.channel_switches, 1);
        assert_eq!(book.completion_signals().len(), 1, "首完成事实独立留档");
        assert_eq!(
            book.entries().next().unwrap().1.structure_end_at,
            None,
            "终态吸收：后到完成信号不回填历史"
        );
    }

    /// F9c 稀疏 trigger 间开窗又完成：若首个可见事实就是完成信号，须在同 trigger
    /// 先补一条零寿命观察再结算，不能静默丢弃。
    #[test]
    fn f9c_completion_only_first_signal_is_recorded_as_flash() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let mut book = NestLifecycleBook::new();
        let (delta, stats) = feed_prefix_phases(&mut book, &[], &events, 99, &m);
        assert_eq!(stats.completion_signals, 1);
        assert!(matches!(
            delta.first().unwrap().kind,
            LifecycleRevisionKind::Observed
        ));
        assert!(delta
            .iter()
            .any(|revision| matches!(revision.kind, LifecycleRevisionKind::StructureCompleted)));
        assert!(matches!(
            delta.last().unwrap().kind,
            LifecycleRevisionKind::Confirmed
        ));
        assert_eq!(book.settlement_stats().flash_terminal_count, 1);
    }

    /// F10a 逃生门逐 bar 两相：同一 bar 必须先落活窗观察，再落完成与终局；
    /// 同 bar 开合照实归入零寿命闪现，不许为制造寿命延后完成。
    #[test]
    fn f10a_bar_feed_orders_live_before_completion_and_keeps_flash() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let windows = [pan_window((50, 59), 69, 99)];
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let mut book = NestLifecycleBook::new();

        let phases = bar_phases(&windows, &events, 99);
        let (delta, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &phases,
            },
            &m,
        );

        let observed = delta
            .iter()
            .position(|revision| matches!(revision.kind, LifecycleRevisionKind::Observed))
            .expect("同 bar 活窗必须先被观察");
        let completed = delta
            .iter()
            .position(|revision| matches!(revision.kind, LifecycleRevisionKind::StructureCompleted))
            .expect("同 bar 完成信号不得丢失");
        let terminal = delta
            .iter()
            .position(|revision| matches!(revision.kind, LifecycleRevisionKind::Confirmed))
            .expect("同 bar 完成后必须结算");
        assert!(observed < completed && completed < terminal);
        assert!(delta.iter().all(|revision| revision.as_of == 99));
        assert_eq!(stats.completion_signals, 1);
        assert_eq!(book.settlement_stats().flash_terminal_count, 1);
        assert_eq!(book.settlement_stats().nonflash_lifetime.count, 0);
    }

    /// F10b 逃生门跨 bar 链：真实活窗逐 bar 延展，先可证、后反超，允许
    /// Provisional→FirstProvable→ForceOvertake；全链 as_of 单调且不伪造完成修订。
    #[test]
    fn f10b_bar_feed_grows_cross_bar_force_overtake_monotonically() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-3.0, -6.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        let first = [pan_window((50, 59), 69, 79)];
        let born_phases = bar_phases(&first, &[], 79);
        let (born, _) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 79,
                phases: &born_phases,
            },
            &m,
        );
        assert!(born
            .iter()
            .any(|revision| matches!(revision.kind, LifecycleRevisionKind::Observed)));
        assert!(born
            .iter()
            .any(|revision| matches!(revision.kind, LifecycleRevisionKind::FirstProvable)));
        assert_eq!(
            book.entries().next().unwrap().1.state,
            NestEventState::Provisional
        );

        let later = [pan_window((50, 59), 69, 99)];
        let later_phases = bar_phases(&later, &[], 99);
        let (overtaken, _) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &later_phases,
            },
            &m,
        );
        assert!(overtaken.iter().any(|revision| matches!(
            revision.kind,
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::ForceOvertake
            }
        )));
        assert!(book
            .entries()
            .next()
            .unwrap()
            .1
            .revisions
            .windows(2)
            .all(|pair| pair[0].as_of <= pair[1].as_of));
        let terminal = book.entries().next().unwrap().1;
        assert_eq!(terminal.invalidated_at, Some(99));
        assert_eq!(terminal.structure_end_at, None);
        assert_eq!(
            book.settlement_stats().force_overtake_lifetime.median,
            Some(20.0)
        );
    }

    /// F10c completion-only 窗：逐 bar 循环首见即完成时仍先合成当前 bar 的观察，
    /// 再完整结算；事件与首完成分母均不得丢弃。
    #[test]
    fn f10c_bar_feed_records_completion_only_window_without_loss() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let mut book = NestLifecycleBook::new();

        let phases = bar_phases(&[], &events, 99);
        let (delta, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &phases,
            },
            &m,
        );

        assert_eq!(stats.completion_events, 1);
        assert_eq!(stats.completion_signals, 1);
        assert_eq!(book.completion_signals().len(), 1);
        assert!(matches!(
            delta.first().unwrap().kind,
            LifecycleRevisionKind::Observed
        ));
        assert!(matches!(
            delta.last().unwrap().kind,
            LifecycleRevisionKind::Confirmed
        ));
        assert_eq!(book.settlement_stats().flash_terminal_count, 1);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 票 #527：完成前活窗可见（L1 active C frontier）
    // ═══════════════════════════════════════════════════════════════════════

    /// 合成 `ParseLayer`：只填 `active_segment_frontier` 实际消费的两个字段
    /// （`strokes` 与 `tail`），其余取默认（本函数不读它们——测试夹具最小化）。
    fn parse_layer_with_pending(
        strokes: Vec<super::super::super::types::Stroke>,
        pending: Option<PendingTail>,
    ) -> ParseLayer {
        ParseLayer {
            strokes: std::rc::Rc::new(strokes),
            tail: std::rc::Rc::new(pending.into_iter().collect()),
            ..ParseLayer::default()
        }
    }

    fn stroke(
        direction: Direction,
        start_index: usize,
        end_index: usize,
        start_price: Tick,
        end_price: Tick,
    ) -> super::super::super::types::Stroke {
        super::super::super::types::Stroke {
            direction,
            start_index,
            end_index,
            start_price,
            end_price,
        }
    }

    /// P1 active C frontier 只从 parser 的未完成尾部读出（票 #527 数据源纪律）。
    ///
    /// 三臂：(a) 有 pending 段 ⟹ 方向/起点/起点价/极值/极值**结构点**逐值可读；
    /// (b) 无 pending 段 ⟹ None（不回落到 confirmed 段回放重建）；
    /// (c) 极值退化在段起点（段尚未推进）⟹ None（不造零长 C）。
    #[test]
    fn p1_active_frontier_reads_parser_pending_tail_only() {
        // (a) pending 段 = 两笔：Down 90→94（低 92 在 index 95），Up 92→96。
        let strokes = vec![
            stroke(Direction::Down, 90, 95, 98, 92),
            stroke(Direction::Up, 95, 99, 92, 96),
        ];
        let layer = parse_layer_with_pending(
            strokes.clone(),
            Some(PendingTail::PendingSegment {
                direction: Direction::Down,
                start_index: 90,
                current_extreme: 92,
            }),
        );
        let frontier = active_segment_frontier(&layer).expect("有 pending 段 ⟹ 有 frontier");
        assert_eq!(frontier.direction, Direction::Down);
        assert_eq!(frontier.start_index, 90);
        assert_eq!(frontier.start_price, 98);
        assert_eq!(
            frontier.extreme, 92,
            "与 parser tail 的 current_extreme 同口径"
        );
        assert_eq!(
            frontier.extreme_at, 95,
            "极值结构点 = 极值所在笔端点，非 as_of"
        );
        assert_eq!(
            frontier.as_segment(),
            Segment {
                direction: Direction::Down,
                start_index: 90,
                end_index: 95,
                start_price: 98,
                end_price: 92,
            },
            "行进中段右端 = 极值结构点（禁 as_of 冒充结构点）"
        );

        // (b) 无 pending 段 ⟹ None。
        assert!(active_segment_frontier(&parse_layer_with_pending(strokes, None)).is_none());

        // (c) 极值退化在段起点（首笔即反向，方向上从未推进）⟹ None。
        let degenerate = parse_layer_with_pending(
            vec![stroke(Direction::Down, 90, 95, 92, 98)],
            Some(PendingTail::PendingSegment {
                direction: Direction::Down,
                start_index: 90,
                current_extreme: 92,
            }),
        );
        assert!(active_segment_frontier(&degenerate).is_none());
    }

    /// P2 完成前活窗可见（票 #527 本体）：C 只用行进中段、A/B 锚用 confirmed 侧，
    /// 产出的身份与该段完成后的完成事件身份**逐分量相同**（除右端）。
    ///
    /// 负控同测：frontier 落在末 confirmed 段内部 ⟹ 拒绝产窗（原因码可审计），
    /// 保证「活窗只能来自行进中段」不是靠注释保证的。
    #[test]
    fn p2_active_frontier_live_window_matches_completed_identity() {
        let (centers, kinds, segments, _anchors, _hist, _dif, _close_src) =
            pan_real_fixture(-0.1, -0.5);
        // confirmed 侧 = 前 4 段；第 5 段 (89,99) 尚在行进中（极值 92 已在 95 打出）。
        let confirmed = &segments[..4];
        let frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 89,
            start_price: 98,
            extreme: 92,
            extreme_at: 95,
        };
        let outcome = provide_active_pan_live_windows(
            1,
            &centers,
            &kinds,
            confirmed,
            frontier.as_segment(),
            95,
        );
        let window = outcome.window().expect("行进中 C 破核心新低 ⟹ 活窗可见");
        assert_eq!(window.seg_a, (50, 59), "A 锚取 confirmed 侧窄锚");
        assert_eq!(window.b_center_start, 20, "B 中枢取 confirmed 侧");
        assert_eq!(
            window.seg_c_live,
            (69, 95),
            "c_start = λ_C（与完成事件 seg_c.0 同锚）；右端随 as_of"
        );
        assert_eq!(
            window.gap_len, 0,
            "frontier.start_index(89) == confirmed.last().end_index(89) ⟹ 共端点，gap_len=0"
        );
        // 与完成后的事件身份同桥（除右端外全等）——完成时 seg_c=(69,99)。
        let completed = pan_event((50, 59), (69, 99), true, 99);
        let live_key = LifecycleObservation::pan_live(window, false).key();
        let completed_key = LifecycleObservation::event(completed, true).key();
        assert!(
            bridge_identity(&live_key, &completed_key),
            "活窗与完成事件必须是同一身份（否则活窗白活）"
        );

        // 负控：frontier 起点落在末 confirmed 段内部（倒灌）⟹ 不是 frontier ⟹ 拒绝 + 原因码。
        let bogus = ActiveSegmentFrontier {
            start_index: 75,
            ..frontier
        };
        assert_eq!(
            provide_active_pan_live_windows(1, &centers, &kinds, confirmed, bogus.as_segment(), 95),
            PanLiveOutcome::FrontierNotAfterConfirmed
        );
        // 负控二：结构点尚未到达当前 bar ⟹ 禁前视。
        assert_eq!(
            provide_active_pan_live_windows(
                1,
                &centers,
                &kinds,
                confirmed,
                frontier.as_segment(),
                94
            ),
            PanLiveOutcome::FrontierAheadOfClock
        );
        // 有洞 frontier（票 #578 首次证实可达；票 #592 裁定：选 A 保持全收 + `gap_len`
        // 诊断字段落账本）：frontier 起点与末 confirmed 段之间有洞（89→90，不共端点）——
        // 现行 `<` 守卫**不拒绝**这一臂，照常产窗，且产出窗携带真实洞长供下游按需过滤
        // （源头照实全收、拒绝判断留给消费端，不在源头做硬编码 accept/reject）。
        let gapped = ActiveSegmentFrontier {
            start_index: confirmed.last().unwrap().end_index + 1,
            ..frontier
        };
        let gapped_window = provide_active_pan_live_windows(
            1,
            &centers,
            &kinds,
            confirmed,
            gapped.as_segment(),
            95,
        )
        .window()
        .expect("有洞 frontier 当下被接受产窗（#592 选 A 裁定）");
        assert_eq!(
            gapped_window.gap_len, 1,
            "gap_len = frontier.start_index(90) − confirmed.last().end_index(89) = 1"
        );
    }

    /// P3 Live→Completed 同身份、零回填、非闪现寿命（票 #527 验收 1/2 的最小语义）。
    ///
    /// as_of=95 由行进中 C 产 Live（observed_at=95，**不是** c_start=69——零回填）；
    /// as_of=99 该段完成 ⟹ 同一身份收完成信号并结算，寿命 = 99-95 = 4（非闪现）。
    #[test]
    fn p3_live_before_completion_settles_same_identity_without_backfill() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 89,
            start_price: 98,
            extreme: 92,
            extreme_at: 95,
        };
        let window = provide_active_pan_live_windows(
            1,
            &centers,
            &kinds,
            &segments[..4],
            frontier.as_segment(),
            95,
        )
        .window()
        .expect("完成前活窗");
        let mut book = NestLifecycleBook::new();
        let live_phase = [PanProviderPhase::Live(window)];
        feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 95,
                phases: &live_phase,
            },
            &m,
        );
        let live_entry = book.entries().next().expect("活窗建仓").1;
        assert_eq!(live_entry.observed_at, 95, "observed_at = 首次实际观察 bar");
        assert_ne!(live_entry.observed_at, 69, "禁回填到 c_start");
        assert_eq!(live_entry.state, NestEventState::Provisional);
        assert_eq!(live_entry.structure_end_at, None, "完成前不置结构完成");

        // 该段在 99 完成 ⟹ 完成相以同一身份到达。
        let completion = [PanProviderPhase::Completed(PanCompletionEvent {
            event: pan_event((50, 59), (69, 99), true, 99),
            completed_lower_id: ElementId {
                level: 0,
                ordinal: 4,
            },
            completed_at: 99,
        })];
        let (delta, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &completion,
            },
            &m,
        );
        assert_eq!(stats.completion_signals, 1);
        assert_eq!(book.len(), 1, "桥迁移而非新建身份");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 95, "完成不改写观察钟");
        assert_eq!(entry.structure_end_at, Some(99));
        assert_eq!(entry.state, NestEventState::Confirmed);
        assert!(delta
            .iter()
            .any(|revision| matches!(revision.kind, LifecycleRevisionKind::StructureCompleted)));
        let settlement = book.settlement_stats();
        assert_eq!(settlement.flash_terminal_count, 0, "不再是闪现");
        assert_eq!(settlement.nonflash_lifetime.count, 1);
        assert_eq!(settlement.nonflash_lifetime.min, Some(4), "寿命 = 99-95");
        let signal = book.completion_signals()[0];
        assert!(
            signal.as_of > book.entries().next().unwrap().1.observed_at,
            "验收 1 主分支：存在 earlier Live 且 observed_at < completion_as_of"
        );
        book.assert_invariants();
    }

    /// P4 true-flash 路径照实记（无 earlier Live ⟹ 零寿命闪现，不为造寿命推迟完成）。
    ///
    /// 与 P3 同一夹具、同一身份，唯一差别是完成前没有活窗相 ⟹ 同 bar 出生并完成。
    #[test]
    fn p4_completion_without_earlier_live_stays_flash() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let phases = [PanProviderPhase::Completed(PanCompletionEvent {
            event: pan_event((50, 59), (69, 99), true, 99),
            completed_lower_id: ElementId {
                level: 0,
                ordinal: 4,
            },
            completed_at: 99,
        })];
        let (delta, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &phases,
            },
            &m,
        );
        assert_eq!(stats.completion_signals, 1, "闪现也不丢分母");
        assert!(matches!(
            delta.first().unwrap().kind,
            LifecycleRevisionKind::Observed
        ));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 99);
        assert_eq!(entry.confirmed_at, Some(99));
        assert_eq!(book.settlement_stats().flash_terminal_count, 1);
        // 真 true-flash 的可审计判据：物理完成 bar == 账本收到 bar（完成钟两分同值，
        // #559 C3 订正——第三钟已删，它在生产恒等于 as_of，分列它不产生分辨力）。
        let signal = book.completion_signals()[0];
        assert_eq!((signal.completed_at, signal.as_of), (99, 99));
    }

    /// P5 完成钟两分可分辨（票 #527；票 #559 条件 C3 订正：物理完成 / 账本收到）。
    ///
    /// 两值刻意不相等；顺序不变量由 `assert_invariants` 同步钉死（违序 ⟹ panic）。
    /// **C3 订正记**：原版断言三值 99/105/110 互不相等，但中间那个「事件首见」钟在生产
    /// 接线下被硬写为 `as_of`（248/248 恒等），只有夹具能造出 105 —— 即断言的是夹具自洽，
    /// 不是生产分辨力。第三钟已删，本测试相应收窄为两钟。
    #[test]
    fn p5_completion_clock_two_points_are_distinguishable() {
        let (_centers, _kinds, _segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let phases = [PanProviderPhase::Completed(PanCompletionEvent {
            event: pan_event((50, 59), (69, 99), true, 110),
            completed_lower_id: ElementId {
                level: 0,
                ordinal: 4,
            },
            completed_at: 99,
        })];
        feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 110,
                phases: &phases,
            },
            &m,
        );
        let signal = book.completion_signals()[0];
        assert_eq!(signal.completed_at, 99, "lower unit 物理完成 bar");
        assert_eq!(signal.as_of, 110, "账本收到 bar");
        assert_ne!(
            signal.completed_at, signal.as_of,
            "两钟可分辨——完成可见性滞后 = as_of − completed_at"
        );
        assert_eq!(
            signal.completed_lower_id,
            ElementId {
                level: 0,
                ordinal: 4
            },
            "完成的 lower unit 身份显式携带（消费方不按 kind 猜 provenance）"
        );
        assert_eq!(
            book.entries().next().unwrap().1.structure_end_at,
            Some(110),
            "账本钟仍取 advance 的 as_of——两钟分列，禁互相冒充"
        );
        book.assert_invariants();
    }

    /// F9 #428 升格硬项：完成信号已到但力度不可验时不得静默接受。
    ///
    /// 必须经生产主接缝 `feed_replay_bar` 可达；这条只要求独立审计事实可查，
    /// 不替编排者裁定新终态。账本仍诚实滞留 Provisional，后续材料补齐后可照原协议继续结算。
    #[test]
    fn f9_completion_force_unavailable_is_explicit_audit() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let legs = legs_of(&segments);
        let runs = [PanLiveRun {
            level: 1,
            centers: &centers,
            kinds: &kinds,
            legs: &legs,
        }];
        let available = material(&hist, &dif, &close_src);
        let no_force = ForceMaterial::unavailable(&close_src);
        let mut book = NestLifecycleBook::new();

        feed_prefix_phases(&mut book, &runs, &[], 79, &available);
        let events = [pan_event((50, 59), (69, 99), true, 99)];
        let (_, stats) = feed_prefix_phases(&mut book, &runs, &events, 99, &no_force);

        assert_eq!(
            book.completion_force_unavailable_audits(),
            &[CompletionForceUnavailableAudit {
                key: key_pan((50, 59), (69, 99)),
                as_of: 99,
                reason: UnavailReason::MissingForceSeries,
            }],
            "完成信号与力度缺失须经生产接缝成为同一条独立审计事实"
        );
        assert_eq!(stats.channel_switches, 1, "完成信号命中先前活身份");
        assert_eq!(stats.completion_signals, 1, "唯一完成信号作为发生率分母");
        assert_eq!(stats.completion_force_unavailable, 1);

        let (_, repeated) = feed_prefix_phases(&mut book, &runs, &events, 109, &no_force);
        assert_eq!(
            repeated.completion_signals, 0,
            "跨 trigger 重发不重复抬高分母"
        );
        assert_eq!(
            repeated.channel_switches, 0,
            "同一信号重发不是第二次通道切换"
        );
        assert_eq!(
            repeated.completion_force_unavailable, 0,
            "同一桥身份重发不重复抬高不可验分子"
        );
        assert_eq!(book.completion_force_unavailable_audits().len(), 1);
        let entry = book.entries().next().expect("重发后桥迁移身份仍在").1;
        assert_eq!(
            entry.state,
            NestEventState::Provisional,
            "本票不擅自裁定新终态"
        );
        assert_eq!(entry.structure_end_at, None, "保持 #78 既有结算语义");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 票 #601：L2 活窗（行进中 L1 窗口单元 + 级别中立 provider）
    // ═══════════════════════════════════════════════════════════════════════

    /// L1 层扫描的最小 L0 units 夹具：方向交替 + 全三段核心非空 ⟹ seed 成立，核心 `[105,120]`。
    ///
    /// 单元坐标共端点（生产 parser 段账本约定），供虚拟追加单元衔接。
    fn l0_units_seed() -> Vec<UnitRange> {
        vec![
            UnitRange {
                start_index: 0,
                end_index: 10,
                direction: Direction::Up,
                lo: 100,
                hi: 120,
            },
            UnitRange {
                start_index: 10,
                end_index: 20,
                direction: Direction::Down,
                lo: 105,
                hi: 120,
            },
            UnitRange {
                start_index: 20,
                end_index: 30,
                direction: Direction::Up,
                lo: 105,
                hi: 125,
            },
        ]
    }

    /// Q1 行进中 L0 段被 L1 窗口吸收 ⟹ 产出**塔上不存在**的行进中 L1 单元。
    ///
    /// 该 frontier 的右端落在行进中 L0 段的极值结构点（40）——`tower[1]` 上任何已产出窗口的
    /// 右端都只能是 confirmed 单元终点（≤30），故这个形态在塔上不存在，不是外推。
    #[test]
    fn q1_active_l1_window_frontier_absorbs_pending_l0_segment() {
        let units = l0_units_seed();
        // 虚拟单元 [110,118] 与冻结核心 [105,120] 相交 ⟹ 延伸吸收。
        let l0_frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 30,
            start_price: 118,
            extreme: 110,
            extreme_at: 40,
        };
        let outcome = active_l1_window_frontier(&units, Some(&l0_frontier), 0, &mut Vec::new());
        let frontier = outcome
            .frontier()
            .expect("行进中 L0 段被吸收 ⟹ 有行进中 L1 单元");
        assert_eq!(outcome.reason_tag(), "active_window");
        assert_eq!(
            frontier.direction,
            Direction::Up,
            "方向 = 窗口首单元方向（first-leaf 口径，与 lower_legs_from 同源）"
        );
        assert_eq!(frontier.start_index, 0, "起点 = 窗口首单元起点");
        assert_eq!(
            frontier.end_index, 40,
            "右端 = 行进中 L0 段的极值结构点——塔上已产出窗口的右端不可能到 40"
        );
        assert_eq!(
            (frontier.lo, frontier.hi),
            (100, 125),
            "外缘 = 窗口全单元聚合（含虚拟单元）"
        );
        let segment = frontier.as_segment();
        assert_eq!(
            (segment.start_price, segment.end_price),
            (100, 125),
            "Up ⟹ (lo, hi)——与 level_view::leg_as_segment 逐字同口径"
        );
        assert_eq!((segment.start_index, segment.end_index), (0, 40));
    }

    /// Q2 **禁外推**（#523 红线）：行进中 L0 段未被吸收 ⟹ 末窗由纯 confirmed 单元构成，
    /// 它与 `tower[1]` 已有窗口同源 ⟹ 拒绝产活动腿。
    #[test]
    fn q2_active_l1_window_frontier_rejects_tower_only_window() {
        let units = l0_units_seed();
        // 虚拟单元 [130,140] 整体高于核心上沿 120 ⟹ non-extension ⟹ 末窗停在 units[2]。
        let l0_frontier = ActiveSegmentFrontier {
            direction: Direction::Up,
            start_index: 30,
            start_price: 130,
            extreme: 140,
            extreme_at: 40,
        };
        let outcome = active_l1_window_frontier(&units, Some(&l0_frontier), 0, &mut Vec::new());
        assert_eq!(outcome, ActiveWindowOutcome::LowerFrontierNotAbsorbed);
        assert_eq!(outcome.reason_tag(), "lower_frontier_not_absorbed");
        assert!(
            outcome.frontier().is_none(),
            "拒绝把已完成 tower unit 冒充行进中单元（重演 #523 的路径必须为空产出）"
        );
    }

    /// Q3 未产出时的原因码逐条可达（禁以「不知道」结账）。
    #[test]
    fn q3_active_l1_window_frontier_reason_codes() {
        let units = l0_units_seed();
        let l0_frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 30,
            start_price: 118,
            extreme: 110,
            extreme_at: 40,
        };

        // (a) parser 无 pending 段。
        let none = active_l1_window_frontier(&units, None, 0, &mut Vec::new());
        assert_eq!(none, ActiveWindowOutcome::NoLowerFrontier);
        assert_eq!(none.reason_tag(), "no_lower_frontier");

        // (b) 倒灌：行进中段起点落在末 confirmed 单元内部。
        let backfill = ActiveSegmentFrontier {
            start_index: 25,
            ..l0_frontier
        };
        assert_eq!(
            active_l1_window_frontier(&units, Some(&backfill), 0, &mut Vec::new()),
            ActiveWindowOutcome::LowerFrontierNotAfterUnits
        );

        // (c) 重扫锚越过 units 长度（塔与 units 不同步）⟹ 不猜锚。
        assert_eq!(
            active_l1_window_frontier(&units, Some(&l0_frontier), units.len() + 1, &mut Vec::new()),
            ActiveWindowOutcome::ResumeAnchorOutOfRange
        );

        // (d) 虚拟追加后仍无窗口成立（方向不交替 ⟹ seed 判据不过）。
        let no_alternation = vec![units[0], units[0]];
        assert_eq!(
            active_l1_window_frontier(&no_alternation, Some(&l0_frontier), 0, &mut Vec::new()),
            ActiveWindowOutcome::NoWindowFormed
        );
    }

    /// Q4 L2 活窗：provider 级别中立 + 同身份衔接 + 零回填。
    ///
    /// 夹具说明（诚实标注）：`pan_real_fixture` 的 centers/segments 是**级别无关**的结构对象
    /// （`Center`/`Segment` 不携带级别），本测试用它驱动 `level = 2` 的产窗路径——测的是
    /// 「provider 对任意 level 同判 + L2 身份的桥衔接与观察钟」，**不是** L2 真实塔数据的
    /// 端到端复现（后者由 BTC 100k 生产回放验收，见交付报告）。C 腿取 [`ActiveWindowFrontier`]
    /// （L2 唯一合法来源），不是 `ActiveSegmentFrontier`。
    #[test]
    fn q4_l2_live_window_bridges_completion_without_backfill() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        // 行进中 L1 单元（Down，起点 89 = 末 confirmed 段终点，右端 95 = 结构点）。
        let active_l1 = ActiveWindowFrontier {
            direction: Direction::Down,
            start_index: 89,
            end_index: 95,
            lo: 92,
            hi: 98,
        };
        let outcome = provide_active_pan_live_windows(
            2,
            &centers,
            &kinds,
            &segments[..4],
            active_l1.as_segment(),
            95,
        );
        let window = outcome.window().expect("L2 完成前活窗");
        assert_eq!(window.level, 2, "level 只随窗口带出，不参与判定");
        assert_eq!(window.seg_a, (50, 59));
        assert_eq!(window.b_center_start, 20);
        assert_eq!(window.seg_c_live, (69, 95), "c_start = λ_C；右端随 as_of");

        let mut book = NestLifecycleBook::new();
        let live_phase = [PanProviderPhase::Live(window)];
        feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 95,
                phases: &live_phase,
            },
            &m,
        );
        let live_entry = book.entries().next().expect("L2 活窗建仓").1;
        assert_eq!(live_entry.key.level, 2, "身份键携 L2");
        assert_eq!(live_entry.observed_at, 95, "observed_at = 首次实际观察 bar");
        assert_ne!(live_entry.observed_at, 69, "禁回填到 c_start");

        // 同一 L2 身份的完成事件到达 ⟹ 桥迁移（不新建），观察钟不被改写。
        let completion = [PanProviderPhase::Completed(PanCompletionEvent {
            event: NestCandidateEvent {
                level: 2,
                ..pan_event((50, 59), (69, 99), true, 99)
            },
            completed_lower_id: ElementId {
                level: 1,
                ordinal: 4,
            },
            completed_at: 99,
        })];
        let (_, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &completion,
            },
            &m,
        );
        assert_eq!(stats.completion_signals, 1);
        assert_eq!(book.len(), 1, "桥迁移而非新建身份");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 95, "完成不改写观察钟");
        assert_eq!(entry.structure_end_at, Some(99));
        let settlement = book.settlement_stats();
        assert_eq!(settlement.flash_terminal_count, 0, "L2 不再是闪现");
        assert_eq!(settlement.nonflash_lifetime.min, Some(4), "寿命 = 99-95");
        let signal = book.completion_signals()[0];
        assert!(
            signal.as_of > entry.observed_at,
            "验收 1 主分支在 L2 成立：存在 earlier Live"
        );
        book.assert_invariants();
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 票 #602：L3 活窗（行进中 L2 窗口单元；build/输入层/方向来源三处换级）
    // ═══════════════════════════════════════════════════════════════════════

    /// L2 层扫描的最小 L1 units 夹具：三段几何重叠 ⟹ `center_from_window` 的核心
    /// `[max lo, min hi] = [105,120]` 非空 ⟹ seed 成立。
    ///
    /// **方向故意全同向**（Up/Up/Up）：L2 层判据是几何路径，**不查方向交替**；同一夹具喂
    /// L1 层判据（`center_from_segments`）必判 `None`——R2 用这条对照把「build 真的换了」钉死，
    /// 不靠读代码相信。
    fn l1_units_seed() -> Vec<UnitRange> {
        vec![
            UnitRange {
                start_index: 0,
                end_index: 10,
                direction: Direction::Up,
                lo: 100,
                hi: 120,
            },
            UnitRange {
                start_index: 10,
                end_index: 20,
                direction: Direction::Up,
                lo: 105,
                hi: 120,
            },
            UnitRange {
                start_index: 20,
                end_index: 30,
                direction: Direction::Up,
                lo: 105,
                hi: 125,
            },
        ]
    }

    /// 行进中的 L1 窗口单元（虚拟追加用）：与末 confirmed 单元共端点，几何触及核心 ⟹ 延伸吸收。
    fn active_l1_unit() -> ActiveWindowFrontier {
        ActiveWindowFrontier {
            direction: Direction::Down,
            start_index: 30,
            end_index: 40,
            lo: 110,
            hi: 118,
        }
    }

    /// R1 行进中 L1 单元被 L2 层窗口吸收 ⟹ 产出**塔上不存在**的行进中 L2 单元；
    /// 方向取**首叶方向表**而非投影 units 自带的 ownership 方向。
    ///
    /// 方向这一条是 L3 特有的正确性点：`level_scan_units(2)[i].direction` 是
    /// `project_to_units` 的 ownership 方向，`lower_legs_from(tower[1])[i].direction` 是
    /// `first_leaf_direction`——两者**可以不同值**（本夹具故意让它们相反）。L3 活窗的
    /// confirmed 侧腿取首叶口径，活动腿必须同口径，否则 A/C 结构定位在拿两套方向语义对比。
    #[test]
    fn r1_active_l2_window_frontier_absorbs_active_l1_unit_with_leg_dirs() {
        let units = l1_units_seed();
        // 首叶方向表与 units 自带方向**逐位相反**（投影 ownership ≠ 首叶，见函数文档）。
        let leg_dirs = [Direction::Down, Direction::Down, Direction::Down];
        let active = active_l1_unit();
        let outcome =
            active_l2_window_frontier(&units, &leg_dirs, Some(&active), 0, &mut Vec::new());
        let frontier = outcome
            .frontier()
            .expect("行进中 L1 单元被吸收 ⟹ 有行进中 L2 单元");
        assert_eq!(outcome.reason_tag(), "active_window");
        assert_eq!(
            frontier.direction,
            Direction::Down,
            "方向 = 首叶方向表[win.0]（Down），**不是** units[0].direction（Up）"
        );
        assert_eq!(frontier.start_index, 0, "起点 = 窗口首单元起点");
        assert_eq!(
            frontier.end_index, 40,
            "右端 = 行进中 L1 单元的终点——`tower[2]` 已产出窗口的右端只能落在 confirmed 单元上（≤30）"
        );
        assert_eq!(
            (frontier.lo, frontier.hi),
            (100, 125),
            "外缘 = 窗口全单元聚合（含虚拟单元）"
        );
        let segment = frontier.as_segment();
        assert_eq!(
            (segment.start_price, segment.end_price),
            (125, 100),
            "Down ⟹ (hi, lo)——与 level_view::leg_as_segment 逐字同口径"
        );
        assert_eq!((segment.start_index, segment.end_index), (0, 40));
    }

    /// R2 **build 真的换了**（对照，不是读代码相信）：同一份输入 + 同一份几何虚拟单元，
    /// L2 层（几何路径 `center_from_window`）成窗，L1 层（完整判据 `center_from_segments`，
    /// 要求方向交替）判无窗。
    ///
    /// 反过来说：若 L3 复用 `active_l1_window_frontier`，本夹具这类**同向重叠**的上级窗口
    /// 会被整类丢掉（塔的 L2 层扫描恰恰会产出它们）。
    #[test]
    fn r2_l2_layer_uses_geometric_build_unlike_l1_layer() {
        let units = l1_units_seed();
        let leg_dirs = [Direction::Up, Direction::Up, Direction::Up];
        let active = active_l1_unit();
        assert!(
            active_l2_window_frontier(&units, &leg_dirs, Some(&active), 0, &mut Vec::new())
                .frontier()
                .is_some(),
            "几何路径只要核心非空即成窗"
        );
        // 同一几何形态换成 L1 层的输入类型（虚拟单元 lo/hi 逐值相同）。
        let l0_frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 30,
            start_price: 118,
            extreme: 110,
            extreme_at: 40,
        };
        assert_eq!(
            active_l1_window_frontier(&units, Some(&l0_frontier), 0, &mut Vec::new()),
            ActiveWindowOutcome::NoWindowFormed,
            "完整判据要求方向交替 ⟹ 全同向输入无窗（这正是 L3 不能复用 L1 层实装的原因）"
        );
    }

    /// R3 L2 层未产出时的原因码逐条可达（禁以「不知道」结账），含 L3 新增的方向表守卫。
    #[test]
    fn r3_active_l2_window_frontier_reason_codes() {
        let units = l1_units_seed();
        let leg_dirs = [Direction::Up, Direction::Up, Direction::Up];
        let active = active_l1_unit();

        // (a) 下一级无行进中单元（L2 层派生本身失败 / 本 bar 无行进中 L0 段）。
        let none = active_l2_window_frontier(&units, &leg_dirs, None, 0, &mut Vec::new());
        assert_eq!(none, ActiveWindowOutcome::NoLowerFrontier);
        assert_eq!(none.reason_tag(), "no_lower_frontier");

        // (b) 首叶方向表与 units 不等长 ⟹ 拒绝，不猜方向。
        let short =
            active_l2_window_frontier(&units, &leg_dirs[..2], Some(&active), 0, &mut Vec::new());
        assert_eq!(short, ActiveWindowOutcome::LowerLegDirsOutOfSync);
        assert_eq!(short.reason_tag(), "lower_leg_dirs_out_of_sync");

        // (c) 倒灌：行进中单元起点落在末 confirmed 单元内部。
        let backfill = ActiveWindowFrontier {
            start_index: 25,
            ..active
        };
        assert_eq!(
            active_l2_window_frontier(&units, &leg_dirs, Some(&backfill), 0, &mut Vec::new()),
            ActiveWindowOutcome::LowerFrontierNotAfterUnits
        );

        // (d) 重扫锚越过 units 长度 ⟹ 不猜锚。
        assert_eq!(
            active_l2_window_frontier(
                &units,
                &leg_dirs,
                Some(&active),
                units.len() + 1,
                &mut Vec::new()
            ),
            ActiveWindowOutcome::ResumeAnchorOutOfRange
        );

        // (e) 虚拟追加后无窗成立（三段无共同重叠 ⟹ 几何 seed 判据不过）。
        let disjoint = vec![
            UnitRange {
                start_index: 0,
                end_index: 10,
                direction: Direction::Up,
                lo: 100,
                hi: 110,
            },
            UnitRange {
                start_index: 10,
                end_index: 30,
                direction: Direction::Up,
                lo: 200,
                hi: 210,
            },
        ];
        assert_eq!(
            active_l2_window_frontier(&disjoint, &leg_dirs[..2], Some(&active), 0, &mut Vec::new()),
            ActiveWindowOutcome::NoWindowFormed
        );

        // (f) **禁外推**（#523 红线）：虚拟单元整体在核心之上 ⟹ non-extension ⟹ 末窗由纯
        //     confirmed 单元构成，与 `tower[2]` 已有窗口同源 ⟹ 拒绝。
        let above = ActiveWindowFrontier {
            direction: Direction::Up,
            start_index: 30,
            end_index: 40,
            lo: 130,
            hi: 140,
        };
        let refused =
            active_l2_window_frontier(&units, &leg_dirs, Some(&above), 0, &mut Vec::new());
        assert_eq!(refused, ActiveWindowOutcome::LowerFrontierNotAbsorbed);
        assert!(
            refused.frontier().is_none(),
            "拿 tower[2] 已产出窗口冒充行进中单元的路径必须空产出（重演 #523 的锁）"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 票 #757：批量丢弃机制不可观测实例的专门原因码（观测面 only）
    // ═══════════════════════════════════════════════════════════════════════

    /// R5 塔层批量丢弃实例逐只落码：一次重扫产出 m=2 窗口（窗1 = units[0..2] 被 u3 判
    /// non-extension 关闭；窗2 = units[3..5] + 虚拟单元延伸吸收）⟹ 批内非末窗（窗1）
    /// 进 `drops`，末窗正常产出。m=1 时 `drops` 恒为空追加。
    #[test]
    fn r5_scan_active_window_records_batch_dropped_windows() {
        let mut units = l0_units_seed();
        // 窗2 seed：方向交替（Down/Up/Down）+ 核心 [135,145] 非空；u3 整体高于窗1 核心
        // 上沿 120 ⟹ 对窗1 是 non-extension 哨兵，窗1 关闭、扫描从 u3 续进。
        units.extend_from_slice(&[
            UnitRange {
                start_index: 30,
                end_index: 40,
                direction: Direction::Down,
                lo: 130,
                hi: 150,
            },
            UnitRange {
                start_index: 40,
                end_index: 50,
                direction: Direction::Up,
                lo: 135,
                hi: 150,
            },
            UnitRange {
                start_index: 50,
                end_index: 60,
                direction: Direction::Down,
                lo: 130,
                hi: 145,
            },
        ]);
        // 虚拟单元 [138,142] 与窗2 核心 [135,145] 相交 ⟹ 窗2 延伸吸收（守卫 3 通过）。
        let l0_frontier = ActiveSegmentFrontier {
            direction: Direction::Down,
            start_index: 60,
            start_price: 142,
            extreme: 138,
            extreme_at: 70,
        };
        let mut drops = Vec::new();
        let outcome = active_l1_window_frontier(&units, Some(&l0_frontier), 0, &mut drops);
        let frontier = outcome.frontier().expect("末窗吸收虚拟单元 ⟹ 产出");
        assert_eq!(
            (frontier.start_index, frontier.end_index),
            (30, 70),
            "末窗 = units[3..5] + 虚拟单元"
        );
        assert_eq!(
            drops,
            vec![BatchDroppedWindow {
                start_index: 0,
                end_index: 30,
            }],
            "批内非末窗（窗1 = units[0..2]）逐只落码，坐标 = 首单元起点..末单元终点"
        );

        // m=1 对照：Q1 夹具（单窗吸收）⟹ 空追加。
        let mut drops1 = Vec::new();
        let outcome1 = active_l1_window_frontier(
            &l0_units_seed(),
            Some(&ActiveSegmentFrontier {
                direction: Direction::Down,
                start_index: 30,
                start_price: 118,
                extreme: 110,
                extreme_at: 40,
            }),
            0,
            &mut drops1,
        );
        assert!(outcome1.frontier().is_some());
        assert!(drops1.is_empty(), "m=1 时无批量丢弃实例");
    }

    /// R6 L1 身份级三类成因分级（唯一分类点 `classify_l1`，判定顺序 = 枚举顺序）：
    /// 机制不可观测（C 从未 pending）→ 时机边界（B 同 bar 首 sealed）→ 可观测未命中。
    #[test]
    fn r6_batch_observability_tracker_three_tier_classification() {
        let mut tracker = BatchObservabilityTracker::default();
        // C 段 100 曾作为行进中 C 被观测；200 从未出现。
        tracker.observe_frontier(Some(&ActiveSegmentFrontier {
            direction: Direction::Up,
            start_index: 100,
            start_price: 10,
            extreme: 12,
            extreme_at: 110,
        }));
        tracker.observe_frontier(None);
        // B 中枢首 sealed：start=50 @ bar 5；start=60 @ bar 8（增量喂两拍）。
        let unit = |start: usize, ordinal: u64| {
            LeveledMove::from_unit(
                &UnitRange {
                    start_index: start,
                    end_index: start + 10,
                    direction: Direction::Up,
                    lo: 100,
                    hi: 120,
                },
                ElementId { level: 1, ordinal },
            )
        };
        let moves = vec![unit(50, 0), unit(60, 1)];
        tracker.observe_tower_level(1, &moves[..1], 1, 5);
        tracker.observe_tower_level(1, &moves, 2, 8);

        let key = |c_start: usize, b_center_start: usize| LifecycleKey {
            level: 1,
            side: Side::Long,
            kind: NestDivergenceKind::Consolidation,
            seg_a: (0, 10),
            seg_c_full: (c_start, c_start + 10),
            b_center_start,
        };
        assert_eq!(
            tracker.classify_l1(&key(200, 50), 8),
            LiveMissCause::ChainConfirmedNoPendingWindow,
            "C=200 从未 pending ⟹ 机制不可观测（判定顺序最优先，B 再早也改不了）"
        );
        assert_eq!(
            tracker.classify_l1(&key(100, 60), 8),
            LiveMissCause::SameBarCenterConfirmation,
            "C 曾 pending 但 B 在完成信号同 bar 才首 sealed ⟹ 时机边界"
        );
        assert_eq!(
            tracker.classify_l1(&key(100, 50), 8),
            LiveMissCause::ObservableWindowMissed,
            "C 曾 pending 且 B 早已 sealed ⟹ 可观测未命中（归因他处）"
        );
        assert_eq!(
            LiveMissCause::ChainConfirmedNoPendingWindow.reason_tag(),
            "chain_confirmed_no_pending_window"
        );
        assert_eq!(
            LiveMissCause::SameBarCenterConfirmation.reason_tag(),
            "same_bar_center_confirmation"
        );
        assert_eq!(
            LiveMissCause::ObservableWindowMissed.reason_tag(),
            "observable_window_missed"
        );
    }

    /// R7 sealed 首见游标：增量只扫新 sealed 元素；水线回缩后重新 sealed 的元素首见 bar
    /// 保原值（首确认是历史事实，首写不后移）。
    #[test]
    fn r7_tracker_sealed_first_seen_survives_watermark_retreat() {
        let mut tracker = BatchObservabilityTracker::default();
        tracker.observe_frontier(Some(&ActiveSegmentFrontier {
            direction: Direction::Up,
            start_index: 100,
            start_price: 10,
            extreme: 12,
            extreme_at: 110,
        }));
        let unit = |start: usize, ordinal: u64| {
            LeveledMove::from_unit(
                &UnitRange {
                    start_index: start,
                    end_index: start + 10,
                    direction: Direction::Up,
                    lo: 100,
                    hi: 120,
                },
                ElementId { level: 1, ordinal },
            )
        };
        let moves = vec![unit(50, 0), unit(60, 1), unit(70, 2)];
        tracker.observe_tower_level(1, &moves[..2], 2, 5); // 50/60 sealed @5
        tracker.observe_tower_level(1, &moves[..1], 1, 6); // 水线回缩到 1
        tracker.observe_tower_level(1, &moves, 3, 9); // 恢复 + 70 新 sealed @9
        let key = |b_center_start: usize| LifecycleKey {
            level: 1,
            side: Side::Long,
            kind: NestDivergenceKind::Consolidation,
            seg_a: (0, 10),
            seg_c_full: (100, 110),
            b_center_start,
        };
        // 60 曾在 bar 5 sealed、回缩后 bar 9 重新 sealed——首见保 5 ⟹ 对 signal@8 不判时机边界。
        assert_eq!(
            tracker.classify_l1(&key(60), 8),
            LiveMissCause::ObservableWindowMissed,
            "回缩重 sealed 不改写首见 bar（5 < 8 ⟹ 非同 bar 确认）"
        );
        // 70 首 sealed @9 ≥ signal@8 ⟹ 时机边界。
        assert_eq!(
            tracker.classify_l1(&key(70), 8),
            LiveMissCause::SameBarCenterConfirmation
        );
    }

    /// R4 L3 活窗：provider 级别中立在 L3 上成立 + 同身份衔接 + 零回填。
    ///
    /// 夹具说明（诚实标注，同 Q4）：`pan_real_fixture` 的 centers/segments 是**级别无关**的
    /// 结构对象，本测试用它驱动 `level = 3` 的产窗路径——测的是「provider 对任意 level 同判
    /// + L3 身份的桥衔接与观察钟」，**不是** L3 真实塔数据的端到端复现（后者由 BTC 100k
    /// 生产回放验收，见交付报告）。C 腿取 [`ActiveWindowFrontier`]（L3 唯一合法来源）。
    #[test]
    fn r4_l3_live_window_bridges_completion_without_backfill() {
        let (centers, kinds, segments, _anchors, hist, dif, close_src) =
            pan_real_fixture(-0.1, -0.5);
        let m = material(&hist, &dif, &close_src);
        let active_l2 = ActiveWindowFrontier {
            direction: Direction::Down,
            start_index: 89,
            end_index: 95,
            lo: 92,
            hi: 98,
        };
        let outcome = provide_active_pan_live_windows(
            3,
            &centers,
            &kinds,
            &segments[..4],
            active_l2.as_segment(),
            95,
        );
        let window = outcome.window().expect("L3 完成前活窗");
        assert_eq!(window.level, 3, "level 只随窗口带出，不参与判定");
        assert_eq!(window.seg_a, (50, 59));
        assert_eq!(window.b_center_start, 20);
        assert_eq!(window.seg_c_live, (69, 95), "c_start = λ_C；右端随 as_of");

        let mut book = NestLifecycleBook::new();
        let live_phase = [PanProviderPhase::Live(window)];
        feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 95,
                phases: &live_phase,
            },
            &m,
        );
        let live_entry = book.entries().next().expect("L3 活窗建仓").1;
        assert_eq!(live_entry.key.level, 3, "身份键携 L3");
        assert_eq!(live_entry.observed_at, 95, "observed_at = 首次实际观察 bar");
        assert_ne!(live_entry.observed_at, 69, "禁回填到 c_start");

        // 同一 L3 身份的完成事件到达 ⟹ 桥迁移（不新建），观察钟不被改写。
        let completion = [PanProviderPhase::Completed(PanCompletionEvent {
            event: NestCandidateEvent {
                level: 3,
                ..pan_event((50, 59), (69, 99), true, 99)
            },
            completed_lower_id: ElementId {
                level: 2,
                ordinal: 7,
            },
            completed_at: 99,
        })];
        let (_, stats) = feed_replay_bar(
            &mut book,
            &ReplayBarFeed {
                as_of: 99,
                phases: &completion,
            },
            &m,
        );
        assert_eq!(stats.completion_signals, 1);
        assert_eq!(book.len(), 1, "桥迁移而非新建身份");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 95, "完成不改写观察钟");
        assert_eq!(entry.structure_end_at, Some(99));
        let settlement = book.settlement_stats();
        assert_eq!(settlement.flash_terminal_count, 0, "L3 不再是闪现");
        assert_eq!(settlement.nonflash_lifetime.min, Some(4), "寿命 = 99-95");
        let signal = book.completion_signals()[0];
        assert!(
            signal.as_of > entry.observed_at,
            "验收 1 主分支在 L3 成立：存在 earlier Live"
        );
        book.assert_invariants();
    }

    // ── 票 #603（#599 裁定）：档 1 暂认中枢严格同锚回溯认领 ──────

    /// 档 1-a：暂认中枢回溯认领——**前身仍 Provisional** ⟹ 迁移（五钟继承、链留痕、
    /// 无 `IdentityVanished`），与 `Supersedes` 同款。
    #[test]
    fn issue603_center_upgrade_claims_provisional_predecessor_by_migration() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        hist[70..=129].fill(-0.25);
        dif[70..=139].fill(-1.0);
        let m = material(&hist, &dif, &close_src);

        // 暂认中枢 b=20 下建仓并首次可证。
        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 120),
                false,
            )],
            120,
            &m,
        );
        assert_eq!(book.len(), 1);

        // 更近的中枢 b=30 确认 ⟹ 同 seg_a、同 C 左端、B 严格更晚 ⟹ 认领。
        let upgraded = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 129)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(upgraded, false)], 129, &m);
        assert!(
            delta.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::CenterUpgraded { from } if from.b_center_start == 20
            )),
            "记 CenterUpgraded 而非另起身份"
        );
        assert_eq!(book.len(), 1, "迁移而非新建：book 内仍只有一只身份");
        let (key, entry) = book.entries().next().unwrap();
        assert_eq!(key.b_center_start, 30);
        assert_eq!(entry.observed_at, 120, "认领迁移不改写观察钟");
        assert_eq!(entry.first_provable_at, Some(120), "首次可证钟不后移");
        assert_eq!(
            entry.superseded_from.map(|from| from.b_center_start),
            Some(20)
        );
        assert_eq!(entry.state, NestEventState::Provisional, "认领不产生终局");
        book.assert_invariants();
    }

    /// 档 1-b：**前身已终态** ⟹ 认领留痕（新身份独立建仓 + 关联修订），前身条目一个 bit
    /// 不动——终态吸收/禁复活/终态钟只写一次三条不变量优先于认领（E2E §1:83）。
    /// 纠误发生在**口径层**：`force_overtake_claimed_count` 给出应从反超分母剔除的数。
    #[test]
    fn issue603_center_upgrade_leaves_terminal_predecessor_untouched() {
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        hist[70..=129].fill(-0.25);
        dif[70..=139].fill(-1.0);

        let mut book = NestLifecycleBook::new();
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        // 活窗延展 ⟹ 力度反超 ⟹ 前身进终态。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                false,
            )],
            139,
            &m,
        );
        let terminal_before = book.get(&key_pan((50, 59), (70, 139))).cloned().unwrap();
        assert_eq!(
            terminal_before.invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );

        // 更近的中枢确认后，同锚新身份到达。
        let upgraded = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 139)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(upgraded, false)], 139, &m);
        assert!(
            delta
                .iter()
                .any(|revision| matches!(revision.kind, LifecycleRevisionKind::Observed)),
            "终态前身不得吸收掉新身份（否则该完成候选整只从账本消失）"
        );
        assert!(delta.iter().any(|revision| matches!(
            revision.kind,
            LifecycleRevisionKind::CenterUpgraded { from } if from.b_center_start == 20
        )));
        assert_eq!(book.len(), 2, "留痕关联，不合并");
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 139))).unwrap(),
            &terminal_before,
            "前身条目逐位不动（禁复活、终态钟只写一次）"
        );
        let settlement = book.settlement_stats();
        assert_eq!(settlement.force_overtake_count, 1, "账本反超数不改");
        assert_eq!(
            settlement.force_overtake_claimed_count, 1,
            "口径层给出应剔除的误计数（35→33 的可计算来源）"
        );
        book.assert_invariants();
    }

    /// 档 1-c（教义边界，**跨锚零实装**）：`seg_a` 不同 = 两个不同的背驰假设，即便共享同一
    /// C 段也**不**认领（#599 §4.2 判定越界，编排者采纳）；B 反向（更早的中枢）同样不认领。
    #[test]
    fn issue603_center_upgrade_rejects_cross_anchor_and_backward_center() {
        let base = key_pan((50, 59), (70, 129));
        let cross_anchor = LifecycleKey {
            seg_a: (40, 49),
            b_center_start: 30,
            ..base
        };
        assert!(
            !bridge_by_center_upgrade(&base, &cross_anchor),
            "跨锚（seg_a 不同）不认领——两个不同背驰假设巧合共享 C 段"
        );
        let backward = LifecycleKey {
            b_center_start: 10,
            ..base
        };
        assert!(
            !bridge_by_center_upgrade(&base, &backward),
            "B 反向（更早的中枢）不认领——那才是真正的回填历史"
        );
        let other_c = LifecycleKey {
            seg_c_full: (80, 129),
            b_center_start: 30,
            ..base
        };
        assert!(
            !bridge_by_center_upgrade(&base, &other_c),
            "C 左端不同不认领"
        );
        let same_b = LifecycleKey {
            seg_c_full: (70, 139),
            ..base
        };
        assert!(
            !bridge_by_center_upgrade(&base, &same_b) && bridge_identity(&base, &same_b),
            "B 相等归 bridge_identity，两码互斥"
        );
        let upgraded = LifecycleKey {
            b_center_start: 30,
            ..base
        };
        assert!(
            bridge_by_center_upgrade(&base, &upgraded),
            "严格同锚 + B 单调前进 ⟹ 认领"
        );

        // 端到端：跨锚身份到达时照常独立建仓，不产生任何认领修订。
        let close_src = identity_close_src(140);
        let mut hist = vec![0.0; 140];
        hist[50..=59].fill(-2.0);
        hist[40..=49].fill(-2.0);
        let mut dif = vec![0.0; 140];
        dif[50..=59].fill(-5.0);
        dif[40..=49].fill(-5.0);
        hist[70..=129].fill(-0.25);
        dif[70..=129].fill(-1.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        let cross = PanLiveWindow {
            seg_a: (40, 49),
            b_center_start: 30,
            ..pan_window((50, 59), 70, 129)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(cross, false)], 129, &m);
        assert!(
            !delta.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::CenterUpgraded { .. }
            )),
            "跨锚零实装：diff 自证无此路径"
        );
        assert_eq!(book.len(), 2, "两个独立假设各自成身份");
        book.assert_invariants();
    }

    // ── 票 #619（影子评审 #603 收口）：多步链场景 ─────────────────────

    /// 认领留痕后，**认领方多活一个 bar 被桥迁移**——认领关联必须存活。
    ///
    /// 这是 H1 的可复现失效路径：`superseded_from` 是单个「当下来源指针」，桥迁移逐 bar
    /// 发生（BTC 100k dump 实证），一旦被改写成「上一 bar 的自己」，经它反查认领前身的口径
    /// 就静默丢数。本测试同时钉住两件事：(a) 该指针确实被覆盖（成因不被掩盖）；
    /// (b) `force_overtake_claimed_count` 仍取到 1（来源已改为 append-only 修订链）。
    #[test]
    fn issue619_claim_association_survives_subsequent_bridge_migration() {
        let close_src = identity_close_src(150);
        let mut hist = vec![0.0; 150];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 150];
        dif[50..=59].fill(-5.0);
        hist[70..=129].fill(-0.25);
        dif[70..=149].fill(-1.0);
        let m = material(&hist, &dif, &close_src);

        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        assert_eq!(
            book.entries().next().unwrap().1.first_provable_at,
            Some(129),
            "前身曾可证（反超的前置条件）"
        );

        // 力度反超 ⟹ 前身（桥迁移到 c=(70,139) 后）进终态。
        hist[130..=149].fill(-3.0);
        dif[130..=149].fill(-6.0);
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 139),
                false,
            )],
            139,
            &m,
        );
        let terminal_key = key_pan((50, 59), (70, 139));
        assert_eq!(
            book.get(&terminal_key).unwrap().invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );

        // 同 prefix：B 升级到达 ⟹ 前身已终态 ⟹ 认领留痕。
        let upgraded = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 139)
        };
        book.advance(&[LifecycleObservation::pan_live(upgraded, false)], 139, &m);
        assert_eq!(
            book.settlement_stats().force_overtake_claimed_count,
            1,
            "认领当刻口径成立"
        );

        // ★ 认领方多活一个 bar：活窗右端延展 ⟹ 桥迁移 ⟹ `superseded_from` 被覆盖。
        let extended = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 140)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(extended, false)], 140, &m);
        assert!(
            delta
                .iter()
                .any(|revision| matches!(revision.kind, LifecycleRevisionKind::Supersedes { .. })),
            "认领方被桥迁移（本测试的前提事件）"
        );

        let claimer_key = LifecycleKey {
            b_center_start: 30,
            ..key_pan((50, 59), (70, 140))
        };
        let claimer = book.get(&claimer_key).expect("认领方在册");
        assert_eq!(
            claimer.superseded_from,
            Some(LifecycleKey {
                b_center_start: 30,
                ..key_pan((50, 59), (70, 139))
            }),
            "当下来源指针已被改写为「上一 bar 的自己」——H1 成因，不得用于跨 bar 口径"
        );
        assert!(
            claimer.revisions.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::CenterUpgraded { from } if from == terminal_key
            )),
            "认领事件仍留在 append-only 修订链里（永不覆盖）"
        );
        assert_eq!(
            book.settlement_stats().force_overtake_claimed_count,
            1,
            "纠误口径不随桥迁移丢数（票 #619 H1）"
        );
        book.assert_invariants();
    }

    /// 同锚 B **连升两次**：两次都走迁移形态，五钟一路继承，两条认领事件逐条留痕。
    ///
    /// 缺口来源（票 #619）：档 1 的三个测试链长都是 1，多步链从未被覆盖——H1/H2 至今
    /// 未被发现的直接原因。
    #[test]
    fn issue619_center_upgrade_chain_migrates_twice_keeping_clocks() {
        let close_src = identity_close_src(150);
        let mut hist = vec![0.0; 150];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 150];
        dif[50..=59].fill(-5.0);
        // 全程弱力度 ⟹ 前身恒 Provisional ⟹ 两次升级都走迁移形态。
        hist[70..=149].fill(-0.25);
        dif[70..=149].fill(-1.0);
        let m = material(&hist, &dif, &close_src);

        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );
        let second = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 130)
        };
        book.advance(&[LifecycleObservation::pan_live(second, false)], 130, &m);
        let third = PanLiveWindow {
            b_center_start: 40,
            ..pan_window((50, 59), 70, 131)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(third, false)], 131, &m);

        assert!(
            delta.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::CenterUpgraded { from } if from.b_center_start == 30
            )),
            "第二次升级认领的是第一次升级后的身份"
        );
        assert_eq!(book.len(), 1, "连升两次仍是一只身份（迁移，非分裂）");
        let (key, entry) = book.entries().next().unwrap();
        assert_eq!(key.b_center_start, 40);
        assert_eq!(entry.observed_at, 129, "两次迁移都不改写观察钟");
        assert_eq!(entry.first_provable_at, Some(129), "首次可证钟一路不后移");
        let upgrades: Vec<usize> = entry
            .revisions
            .iter()
            .filter_map(|revision| match revision.kind {
                LifecycleRevisionKind::CenterUpgraded { from } => Some(from.b_center_start),
                _ => None,
            })
            .collect();
        assert_eq!(
            upgrades,
            vec![20, 30],
            "两条认领事件逐条留痕，后一条不覆盖前一条"
        );
        book.assert_invariants();
    }

    /// 多候选择优（H2）：留痕把终态前身留在册 ⟹ 第三次 B 升级时**死/活两只同时匹配**，
    /// 必须迁移那只**存活**的，不能按 `BTreeMap` 序盲取。
    ///
    /// 本用例刻意让终态前身在键序上**排在前面**（`seg_c_full` 先于 `b_center_start` 参与
    /// 排序，而 C 右端与死活无关）——盲取实现会选中终态者 ⟹ 存活前身不被迁移 ⟹ 五钟不
    /// 继承、沦为孤儿 ⟹ 凭空多一条 `IdentityVanished`。
    #[test]
    fn issue619_center_upgrade_prefers_migratable_predecessor_over_terminal_one() {
        let close_src = identity_close_src(150);
        let mut hist = vec![0.0; 150];
        hist[50..=59].fill(-2.0);
        let mut dif = vec![0.0; 150];
        dif[50..=59].fill(-5.0);
        hist[70..=129].fill(-0.25);
        dif[70..=149].fill(-1.0);
        let m = material(&hist, &dif, &close_src);

        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 129),
                false,
            )],
            129,
            &m,
        );

        // bar 130：力度反超 ⟹ 桥迁移后进终态，终态键 c=(70,130)。三通道须**同时**不衰减
        // （`segments_diverge_or` 是或关系），故单根强 bar 需压过 a=[50,59] 的整段面积。
        hist[130..=149].fill(-30.0);
        dif[130..=149].fill(-60.0);
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(
                pan_window((50, 59), 70, 130),
                false,
            )],
            130,
            &m,
        );
        let terminal_key = key_pan((50, 59), (70, 130));
        let terminal_before = book.get(&terminal_key).cloned().expect("终态前身在册");
        assert_eq!(
            terminal_before.invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );

        // bar 131：第二次 B（前身已终态）⟹ 留痕新建 ⟹ 同链出现第二只可匹配者。
        let second = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 131)
        };
        book.advance(&[LifecycleObservation::pan_live(second, false)], 131, &m);
        let live_key = LifecycleKey {
            b_center_start: 30,
            ..key_pan((50, 59), (70, 131))
        };
        assert_eq!(
            book.get(&live_key).expect("留痕新身份在册").state,
            NestEventState::Provisional
        );
        assert!(
            terminal_key < live_key,
            "本用例的排序前提：终态前身在 BTreeMap 序上更靠前（盲取会取到它）"
        );

        // bar 132：第三次 B ⟹ 两只都满足判据，必须选存活的那只。
        let third = PanLiveWindow {
            b_center_start: 40,
            ..pan_window((50, 59), 70, 132)
        };
        let delta = book.advance(&[LifecycleObservation::pan_live(third, false)], 132, &m);
        assert!(
            delta.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::CenterUpgraded { from } if from == live_key
            )),
            "优选可迁移（存活）前身，而非键序首个（终态）前身——票 #619 H2"
        );
        assert!(
            !delta.iter().any(|revision| matches!(
                revision.kind,
                LifecycleRevisionKind::Invalidated {
                    reason: InvalidatedReason::IdentityVanished { .. }
                }
            )),
            "存活前身被迁移走 ⟹ 不留孤儿、不产生凭空的身份消失"
        );
        assert_eq!(book.len(), 2, "终态前身留档 + 迁移后的新身份");
        let migrated = book
            .get(&LifecycleKey {
                b_center_start: 40,
                ..key_pan((50, 59), (70, 132))
            })
            .expect("迁移后的身份在册");
        assert_eq!(migrated.observed_at, 131, "五钟随迁移继承（不是重新建仓）");
        assert_eq!(
            book.get(&terminal_key).unwrap(),
            &terminal_before,
            "终态前身仍逐位不动"
        );
        book.assert_invariants();
    }
}
