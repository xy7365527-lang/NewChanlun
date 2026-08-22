//! 回放喂数出口（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 回放喂数出口（票 #426 早期适配层；ADR-0003：结构完成 = 通道切换）
//
// **本段三件（`leg_as_segment_copy` / `PanLiveRun` / `provide_replay_live_windows`）
// 现仅测试可达，不是生产主接缝**（票 #605 定档核实；影子评审 shadow-421-whole S-H2 /
// shadow-527-review T-3 先期登记）：生产侧 sidecar 实走单 run typed 结果的
// `provide_active_pan_live_windows`（本文件 `pub fn provide_active_pan_live_windows`），
// p123_fast_replay.rs 自持独立 `lifecycle_leg_as_segment` 复制腿转段，均不调用本段三件。
// 全库唯一调用点是本文件 `#[cfg(test)] mod tests` 内的夹具（`feed_prefix_phases`，25 调用点
// 覆盖 12 个 #421/#426 验收测试）。**名分：4 归档（测试专用通道，2026-07-29 #605 终审，
// 编排者裁）**——生产不消费但验收测试喂书依赖（已完成 legs 语义与生产 active frontier
// 为 #523 分水岭两端，不能机械换夹具）；删除须待验收测试喂书迁移方案票，随迁移票一并执行。
// ═══════════════════════════════════════════════════════════════════════════

/// `level_view.rs:436` 私有 `leg_as_segment` 的接线侧复制（规格 Implementation Decisions
/// 「在接线侧复制一份」而非提升可见性；`runner.rs` 诊断臂与 p409 探针已有同型先例）。
/// **仅服务于下方 `provide_replay_live_windows`——该函数现仅测试可达**（票 #605）。
fn leg_as_segment_copy(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
}

/// 单个 run 的行进中通道取数（回放引擎逐前缀评估循环内已具备的量）。
/// **仅供下方 `provide_replay_live_windows` 与其测试夹具使用——生产侧不构造本结构**（票 #605）。
#[derive(Debug, Clone, Copy)]
pub struct PanLiveRun<'a> {
    pub level: u32,
    /// run 投影种子的中枢序列。
    pub centers: &'a [Center],
    /// `center_block_kind` 的块类别向量（与 `centers` 等长）。
    pub kinds: &'a [Option<MoveKind>],
    /// 次级别腿（`lower_legs_from(tower[ℓ-1])`）；出口内按 `leg_as_segment_copy` 转段。
    pub legs: &'a [LowerLeg],
}

/// 把同源 provider 的逐 run 结构展开为当前边界的活窗。
///
/// 本函数是旧 trigger/provider 适配层的展开原语，**全库唯一调用点在本文件
/// `#[cfg(test)] mod tests`**（票 #605 核实）。生产逐 bar sidecar 在 p123 内按 p409
/// 同构机制独立发现结构窗（走 `provide_active_pan_live_windows` 单 run 通道），
/// 不经本函数；后续 bar 保持身份字段不动，仅延展 `seg_c_live.1`。
pub fn provide_replay_live_windows(runs: &[PanLiveRun<'_>], as_of: usize) -> Vec<PanLiveWindow> {
    let mut live_windows = Vec::new();
    for run in runs {
        let segments: Vec<Segment> = run.legs.iter().map(leg_as_segment_copy).collect();
        // 自锚 = 逐段方向（与 `provide_nest_candidate_events_ext` level_view.rs:735 同口径，
        // 不是 `self_anchors`——禁第二查法）。
        let anchors: Vec<Option<Direction>> = segments
            .iter()
            .map(|segment| Some(segment.direction))
            .collect();
        live_windows.extend(provide_pan_live_windows(
            run.level,
            run.centers,
            run.kinds,
            &segments,
            &anchors,
            as_of,
        ));
    }
    live_windows
}

/// 完成事件的**显式 typed 输出**（票 #527）：只有当 lower unit 真正进入 completed set 时
/// 才允许构造。
///
/// 消费方**不再**按 `kind == Consolidation` 猜 provenance——那正是 #523 判定的根因之一
/// （generic 事件被无证明地当作完成）。构造方（provider/接线侧）必须给出：
/// 完成的 lower unit 身份 `completed_lower_id` 与其物理完成 bar `completed_at`
/// （完成钟两分见 [`CompletionSignal`]；事件首见 bar = 账本 `as_of`，不另设字段——#559 C3）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanCompletionEvent {
    /// 同源 provider 的候选事件本体（口径一个 bit 不动）。
    pub event: NestCandidateEvent,
    /// 真正进入 completed set 的 lower unit 身份（`ElementId`，跨 bar 稳定）。
    pub completed_lower_id: ElementId,
    /// lower unit 物理完成 bar（该单元 `end_index`）。
    pub completed_at: usize,
}

/// provider 的两相输出（票 #527 / #523 §3 伪代码）。
///
/// `Live` 必须来自行进中的 C（L1 = parser pending/tail，见 [`ActiveSegmentFrontier`]），
/// **禁**由 confirmed segments 回放重建；`Completed` 只在 lower unit 真正完成时产生。
/// 同 bar 内两相顺序固定：先 live 相、后 completion 相（不回填、不延迟）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanProviderPhase {
    Live(PanLiveWindow),
    Completed(PanCompletionEvent),
}

/// 独立逐 bar 循环的两相输入：当前 bar 的 provider 相序列。
#[derive(Debug, Clone, Copy)]
pub struct ReplayBarFeed<'a> {
    /// 当前 bar 的源坐标；账本所有钟均直接取本值，禁止回填或延迟结算。
    pub as_of: usize,
    /// 当前 bar 的 provider 两相输出（出口内按 live 相先、completion 相后消费，顺序不依赖
    /// 入参排列）。活窗身份由调用方沿 p409 机制保持，逐 bar 只延展右端。
    pub phases: &'a [PanProviderPhase],
}

/// 喂数出口计数面（诊断/审计；不进真值路径、不参与任何判定）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayFeedStats {
    /// 本前缀行进中通道产出的活窗数（不含完成事件反推的闪现观察）。
    pub live_windows: usize,
    /// 本 bar 完成事件通道的盘整事件观察数（provider 可重发，不作发生率分母）。
    pub completion_events: usize,
    /// 本 bar 真实、唯一的首完成信号数；包括同 bar 闪现与已提前终局身份（完整分母）。
    pub completion_signals: usize,
    /// 本 bar 首次由活窗观察切到完成信号的唯一身份数（与首完成分母同口径）。
    pub channel_switches: usize,
    /// 已进终态的身份被跳过喂入的活窗数（完成即停延展；终态吸收下逐位等价）。
    pub extension_suppressed: usize,
    /// 本 bar 新增的时点倒退拒绝注记数。
    pub retrograde_rejected: usize,
    /// 本 bar 新增的「完成信号已到但力度不可验」审计事实数（#428）。
    pub completion_force_unavailable: usize,
}

/// 独立逐 bar 循环的账本喂数出口（#421 逃生门主接缝）。
///
/// **侧车**：只读入参、只写 `book`，对生产 trigger、事件流、装配与证书真值零反流。
/// 每根 bar 严格分两相：先喂当前可见的全部活窗，再喂当前首次可见的完成信号。完成信号
/// 不删除同 bar 的活窗观察，也不要求身份来自更早 bar；同 bar 开合照实形成
/// Observed→StructureCompleted→终局，寿命为 0。若首见即只有完成事件，则只在当前 bar
/// 合成观察，禁止回填历史；若身份已由 Provisional→ForceOvertake 提前终局，后到首完成
/// 仍进入独立分母，但终态吸收禁止回填 `StructureCompleted`。
pub fn feed_replay_bar(
    book: &mut NestLifecycleBook,
    feed: &ReplayBarFeed<'_>,
    material: &ForceMaterial,
) -> (Vec<LifecycleRevision>, ReplayFeedStats) {
    let mut stats = ReplayFeedStats::default();
    let mut live_observations: BTreeMap<LifecycleKey, LifecycleObservation> = BTreeMap::new();
    for window in feed.phases.iter().filter_map(|phase| match phase {
        PanProviderPhase::Live(window) => Some(*window),
        PanProviderPhase::Completed(_) => None,
    }) {
        stats.live_windows += 1;
        let observation = LifecycleObservation::pan_live(window, false);
        let key = observation.key();
        // 完成即停延展：终态身份不再喂行进中窗（终态吸收下逐位等价，见
        // `terminal_bridge_hit` 文档与 F7）。
        if book.terminal_bridge_hit(&key) {
            stats.extension_suppressed += 1;
            continue;
        }
        live_observations.insert(key, observation);
    }
    // 完成相：只取盘整域（trend 域不在票 #426 范围——090 登记，见模块头）。provenance 由
    // `PanCompletionEvent` 显式携带（票 #527），不再按 `kind` 猜「这是否是完成」。
    let completions: Vec<PanCompletionEvent> = feed
        .phases
        .iter()
        .filter_map(|phase| match phase {
            PanProviderPhase::Completed(completion) => Some(*completion),
            PanProviderPhase::Live(_) => None,
        })
        .filter(|completion| completion.event.kind == NestDivergenceKind::Consolidation)
        .collect();
    stats.completion_events = completions.len();
    let mut completion_phase = Vec::new();
    let mut completion_keys: Vec<LifecycleKey> = Vec::new();
    for completed in completions {
        let completion = LifecycleObservation::event(completed.event, true);
        let completed_key = completion.key();
        // 同 bar 的 provider 可能经多个 run 重复产同一桥身份；物理观察数照记，
        // 结算与首完成分母只处理一次。
        if completion_keys
            .iter()
            .any(|seen| *seen == completed_key || bridge_identity(seen, &completed_key))
        {
            continue;
        }
        completion_keys.push(completed_key);

        let mut matching_live = live_observations
            .values()
            .find(|live| {
                let key = live.key();
                key == completed_key || bridge_identity(&key, &completed_key)
            })
            .copied();
        // 同 bar 开完又完成：首个可见事实虽只剩完成事件，仍在当前钟先建闪现观察。
        // 只复用事件的身份窗，不伪造历史 as_of。
        if matching_live.is_none() && book.bridge_entry(&completed_key).is_none() {
            let event = completed.event;
            let flash = LifecycleObservation::pan_live(
                PanLiveWindow {
                    level: event.level,
                    side: event.side,
                    seg_a: event.seg_a,
                    seg_c_live: event.interval_b,
                    b_center_start: event.b_center_start,
                    // 从完成事件反推的闪现观察，非 active frontier 通道，无洞概念，恒 0。
                    gap_len: 0,
                },
                false,
            );
            live_observations.insert(flash.key(), flash);
            matching_live = Some(flash);
        }

        // 首完成事实独立于终态：即使该身份已提前 ForceOvertake，也照实进入完整分母。
        // 完成钟两分（票 #527；#559 C3 订正）随信号一并留档，账记不混。
        if book.register_completion_signal(
            completed_key,
            feed.as_of,
            completed.completed_lower_id,
            completed.completed_at,
        ) {
            stats.completion_signals += 1;
            stats.channel_switches += 1;
        }
        let settling = matching_live
            .and_then(|live| match live.force(material) {
                // #428：完成信号已到，但行进中窗的三值力度材料不可验。用同一活窗携带
                // structure_completed=true 进入 advance，显式落 audit；不伪造事件布尔真值。
                ForceCheck::Unavailable(_) => match live {
                    LifecycleObservation::PanLive { window, .. } => {
                        Some(LifecycleObservation::pan_live(window, true))
                    }
                    LifecycleObservation::Event { .. } => None,
                },
                ForceCheck::Verified(_) => None,
            })
            .unwrap_or(completion);
        completion_phase.push(settling);
    }
    let before = book.retrograde_rejections().len();
    let unavailable_before = book.completion_force_unavailable_audits().len();
    let mut batch: Vec<LifecycleObservation> = live_observations.into_values().collect();
    batch.extend(completion_phase);
    let delta = book.advance(&batch, feed.as_of, material);
    stats.retrograde_rejected = book.retrograde_rejections().len() - before;
    stats.completion_force_unavailable =
        book.completion_force_unavailable_audits().len() - unavailable_before;
    (delta, stats)
}
