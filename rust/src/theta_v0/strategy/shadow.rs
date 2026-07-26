//! #196 阶段 A：shadow 双链比对（零行为变更）。
//!
//! 在生产 π loop 组合层裁决点（[`crate::theta_v0::strategy::coverage::pi_theta_step_traced`]
//! 调用点，runner）之后**并行**跑 channel 适配层：仅记录分歧，不改裁决与订单流。
//! 本模块是 coverage tests `t2_cross_level_confirmed_certificate_holds_l1_position_end_to_end`
//! （#146 T2 验收1）内双链交叉断言（票面/spec 锚编号 coverage.rs:5068）同构断言思路的
//! **全量化**（spec WP-3 阶段 A seam：
//! `chanlun/plans/spec-accounting-layer-alignment-20260723.md` Testing Decisions）。
//!
//! ## 输入适配层（票面口径：新建适配代码，非新建解释器）
//!
//! - 活动集 → 每 slot 一声部：slot = `(level, σ)`（interp 规则3/4 的 slot 唯一性语义；
//!   VoiceState 单腿 slot 恰好覆盖——同级双向腿 = 两个独立 slot 各自裁决）。
//! - candidates 按声部分发 clone：全量 clone（channel 谓词自带级别过滤，`find_reverse`/
//!   `find_open`/`observe_sub_cycle` 均按 level 匹配），不新建切分逻辑。
//! - parent_projections 生产构造：runner 侧用现成
//!   [`crate::theta_v0::strategy::interp::parent_certificate_projection`] 逐候选构造，
//!   本模块按 `parent_id == 腿.id` 逐声部过滤分发。
//! - `parent_kappa` 恒 [`ParentKappa::Unknown`]：P7 记录语境降级（用户裁定 2026-07-23
//!   接受为 v1 状态，计入 fog；缺口 G7 在册）。
//!
//! ## 零行为变更边界（票面硬约束）
//!
//! - channel 只出裁决不建腿：声部腿槽每 bar 由生产活动集镜像覆写，[`channel::step_voice`]
//!   只读；[`channel::shadow_observe`] 只推进 P7 检测器/步计数（Hold 转移语义），
//!   `advance` 不进生产路径；生产侧建腿/清腿原样保留。
//! - `VoiceState.short_diff` 槽恒 `None`：生产短差由 TW/PanDiv 链独立承担（channel 8 槽
//!   无 TW 通道，备忘 §2）——P4 恒不触发；P7 裁决无生产对应语义，记入
//!   [`DivergenceKind::ChannelOnly`]（预期内分歧，非缺陷）。#282：P5 短差开启槽已删
//!   （#280 裁定，S6 开空腿形态废止），channel 不再产 `OpenShortDiff` 裁决。
//! - 本模块不持有/不修改任何生产状态；分歧只进 [`ShadowStepRecord`] 内存累积 +
//!   env 门控（`THETA_V0_SHADOW_DIVERGENCE_PATH`）落盘，不进 opsem dump、不进订单轨。
//!
//! ## 运行时成本（memo §4 已批准量级）
//!
//! shadow 每 bar 无条件执行（票面验收①要求生产路径真实运行，非门控）：每 slot 一次
//! candidates 全量 clone + 每 bar 一次事实索引重建——env 未设时为零产出纯观测开销，
//! 与「订单轨 bit-exact 不变」正交（只读观测，无写入生产状态）。

use std::collections::{HashMap, HashSet};

use super::super::classifier::recursive_tower::ElementId;
use super::channel::{self, ChannelDecision, ChannelId, ParentKappa, VoiceState, VoiceStepInput};
use super::coverage::{StepTrace, Vertical, VoiceVerdict};
use super::interp::{ActiveLeg, Candidate, ExitType, ParentCertificateProjection};
use super::voice::VoiceSide;

/// shadow 声部键：`(level, σ)`——interp slot 语义（规则3/4 同 slot 至多一腿）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VoiceKey {
    level: u32,
    dir: VoiceSide,
}

/// 生产事实（#201 阶段 B：持仓声部由 [`StepTrace.verdicts`] 显式裁决序列**单源**推导，
/// 空仓声部按 slot 查 `opened` 保留桶；每声部每 bar 恰一枚）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProductionFact {
    /// 持仓声部：腿在 `next_active` 延续。
    Held,
    /// 持仓声部：腿入 close 桶（携 typed 裁决）。
    Closed(ExitType),
    /// 持仓声部：P1 强平清空。
    RiskExited,
    /// 持仓声部：TW P2 CloseOverlay 关闭（channel 8 槽无 TW 通道——预期缺口）。
    OverlayClosed,
    /// 持仓声部：§13 AncOK/Stale 静默剪除（channel 无结构剪枝语义——预期缺口）。
    SilentDropped,
    /// 空仓声部：本 slot 有候选真准入（opened）。
    Opened,
    /// 空仓声部：本 slot 无开仓。
    Idle,
}

/// 分歧类别（裁决×事实交叉表的全枚举；`Match` 只计数不落记录）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DivergenceKind {
    /// channel 裁决与生产事实一致。
    Match,
    /// 同是出场但 typed 不一致（如 channel CloseRoot vs 生产 ReduceCore）。
    ExitTypedMismatch,
    /// channel 裁出场，生产腿延续（TW 屏蔽/AncOK 幸存/多候选 fold 结构差）。
    ChannelExitProductionHold,
    /// channel 裁 Hold，生产腿离场（ShortDiff entry_v 已知缺口/TW overlay/证书门差）。
    ChannelHoldProductionExit,
    /// channel 裁开仓，生产未开（§13 AncOK 准入门剪 / 候选被散装 fold 规则2 消费为关闭
    /// 触发——channel 声部独立互斥 vs fold 消费语义的结构差，预期缺口）。
    ChannelOpenProductionIdle,
    /// channel 裁 Hold，生产本 slot 开新腿（channel 排除 ShortDiff 角色候选开仓，interp
    /// 规则3 不排除——已知语义差；或同 bar 先关后开的 fold/声部互斥结构差）。
    ChannelHoldProductionOpen,
    /// channel 独有裁决，生产无对应语义（Record/AddPosition——P7/P8 生产落点不在
    /// channel 域，票面明知；P5 `OpenShortDiff` 已随 #282 删除）。
    ChannelOnly,
    /// 生产 silent drop（AncOK/Stale），channel 无此语义（预期缺口）。
    ProductionSilentDrop,
}

/// 每声部每 bar 一条比对记录（仅 `kind != Match` 落存；Match 只进 [`ShadowStats`]）。
#[derive(Debug, Clone)]
pub(crate) struct ShadowStepRecord {
    /// bar 序号（runner 主循环 `i`，决策 bar）。
    pub bar: usize,
    /// 声部 slot。
    pub voice_level: u32,
    pub voice_dir: VoiceSide,
    /// 持仓腿身份（None = 空仓 slot）。
    pub leg_id: Option<ElementId>,
    /// channel 命中的通道与裁决。
    pub channel: ChannelId,
    pub decision: ChannelDecision,
    /// 同 slot 生产事实。
    pub production: ProductionFact,
    /// 分歧类别。
    pub kind: DivergenceKind,
}

/// 全量统计（含 Match——「全量化」指覆盖每 bar 每声部，存储上只分歧落记录）。
#[derive(Debug, Default)]
pub(crate) struct ShadowStats {
    /// 已比对的声部-步总数（= 各 kind 计数之和）。
    pub voice_steps: u64,
    /// channel 裁决与生产事实一致数。
    pub matches: u64,
    pub exit_typed_mismatch: u64,
    pub channel_exit_production_hold: u64,
    pub channel_hold_production_exit: u64,
    pub channel_open_production_idle: u64,
    pub channel_hold_production_open: u64,
    pub channel_only: u64,
    pub production_silent_drop: u64,
}

impl ShadowStats {
    fn bump(&mut self, kind: DivergenceKind) {
        self.voice_steps += 1;
        match kind {
            DivergenceKind::Match => self.matches += 1,
            DivergenceKind::ExitTypedMismatch => self.exit_typed_mismatch += 1,
            DivergenceKind::ChannelExitProductionHold => self.channel_exit_production_hold += 1,
            DivergenceKind::ChannelHoldProductionExit => self.channel_hold_production_exit += 1,
            DivergenceKind::ChannelOpenProductionIdle => self.channel_open_production_idle += 1,
            DivergenceKind::ChannelHoldProductionOpen => self.channel_hold_production_open += 1,
            DivergenceKind::ChannelOnly => self.channel_only += 1,
            DivergenceKind::ProductionSilentDrop => self.production_silent_drop += 1,
        }
    }

    /// 分歧总数（voice_steps − matches）。
    pub fn divergences(&self) -> u64 {
        self.voice_steps - self.matches
    }
}

/// shadow 声部簿：跨 bar 持久的每 slot [`VoiceState`]（P7 检测器/步计数连续；
/// 腿槽每 bar 由生产活动集镜像覆写——channel 不建腿不清腿）。
#[derive(Debug, Default)]
pub(crate) struct ShadowVoiceBook {
    voices: HashMap<VoiceKey, VoiceState>,
    records: Vec<ShadowStepRecord>,
    stats: ShadowStats,
}

impl ShadowVoiceBook {
    /// 每 bar 一次：生产裁决已定（`trace` 携 #201 显式 per-voice 裁决序列），shadow 并行跑
    /// channel 并逐声部比对。**只读**全部生产输入；产出仅入本簿（内存）——不改任何生产状态。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn observe_and_compare(
        &mut self,
        bar: usize,
        prev_active: &[ActiveLeg],
        gamma: &[Candidate],
        entry_v: &HashMap<ElementId, Vertical>,
        force_flat: bool,
        parent_projections: &[ParentCertificateProjection],
        trace: &StepTrace,
    ) {
        // ① 本 bar slot 集：持仓 slot（活动集 (level,σ)，interp 规则3/4 slot 唯一性）∪
        //    候选 slot（gamma 可交易方向——空仓侧 P6 裁决对象，与生产喂 interpret 同一 χ 后集）。
        let mut held: HashMap<VoiceKey, ActiveLeg> = HashMap::new();
        for leg in prev_active {
            if leg.dir == VoiceSide::Flat {
                continue; // Flat 不应入活动集（ActiveLeg doc）——防御跳过，不冒充裁决对象。
            }
            // 同级同向多腿不应出现（slot 唯一性）；防御取首条，不多头裁决。
            held.entry(VoiceKey { level: leg.level, dir: leg.dir }).or_insert(*leg);
        }
        let mut live_keys: HashSet<VoiceKey> = held.keys().copied().collect();
        for c in gamma {
            if c.dir == VoiceSide::Flat {
                continue;
            }
            live_keys.insert(VoiceKey { level: c.level, dir: c.dir });
        }

        // 生产事实索引（#201 阶段 B：持仓声部由 `StepTrace.verdicts` 显式裁决序列单源推导——
        // trace/裁决层统一；CloseShortDiff 源头区分查 `overlay_closes` 保留桶、空仓声部查
        // `opened` 保留桶——加轨不减轨，五桶全部保留）。
        let verdicts: HashMap<ElementId, ExitType> =
            trace.verdicts.iter().map(|v| (v.leg.id, v.exit)).collect();
        let overlay: HashSet<ElementId> = trace.overlay_closes.iter().map(|l| l.id).collect();
        let opened: HashSet<VoiceKey> =
            trace.opened.iter().map(|(c, _l)| VoiceKey { level: c.level, dir: c.dir }).collect();

        // ② 逐 slot：镜像构造/覆写 voice → step_voice 裁决 → 比对 → shadow_observe 推进。
        for key in live_keys.iter().copied() {
            let leg = held.get(&key).copied();
            let input = VoiceStepInput {
                force_flat,
                candidates: gamma.to_vec(), // 按声部分发 clone；谓词自带级别过滤。
                parent_kappa: ParentKappa::Unknown, // 票面降级（fog 在册，模块 doc）。
                parent_projections: match &leg {
                    // P4/P7 只消费「以本声部腿为父」的投影（trigger_projection_sound 逐字段
                    // 匹配 parent_id；P7 projection_now 口径 = 本声部父投影非空；#282：P5
                    // 消费面随槽删除）。
                    Some(l) => parent_projections
                        .iter()
                        .filter(|p| p.parent_id() == l.id)
                        .copied()
                        .collect(),
                    None => Vec::new(), // 空仓声部无父腿 ⟹ 无投影（P7 域护栏外）。
                },
            };
            // voice 状态：同 id 腿延续 ⟹ 检测器/步计数持久（腿槽覆写为生产真腿——
            // source_index 随父延伸漂移，id/lambda 稳定）；持仓边界（id 切换/空仓↔持仓）
            // ⟹ 重置（「本级持仓期间」字面边界，对齐 advance 持仓期语义）。
            let state = match (self.voices.get(&key), leg) {
                (Some(v), Some(l)) if v.leg.map(|x| x.id) == Some(l.id) => {
                    VoiceState { leg: Some(l), ..*v }
                }
                (Some(v), None) if v.leg.is_none() => *v,
                _ => VoiceState {
                    level: key.level,
                    leg,
                    entry_v: leg
                        .and_then(|l| entry_v.get(&l.id).copied())
                        .unwrap_or(Vertical::Ambient),
                    step: 0,
                    sub_cycle: channel::SubCycleTracker::default(),
                    short_diff: None, // v1：生产短差由 TW/PanDiv 链承担（模块 doc 降级声明）。
                },
            };
            let (cid, dec) = channel::step_voice(&state, &input);
            let fact = production_fact(leg, key, &verdicts, &overlay, &opened);
            let kind = classify(dec, fact);
            self.stats.bump(kind);
            if kind != DivergenceKind::Match {
                self.records.push(ShadowStepRecord {
                    bar,
                    voice_level: key.level,
                    voice_dir: key.dir,
                    leg_id: leg.map(|l| l.id),
                    channel: cid,
                    decision: dec,
                    production: fact,
                    kind,
                });
            }
            // 推进（Hold 转移语义：只推进检测器/步计数；仓位转移由生产侧承担）。
            self.voices.insert(key, channel::shadow_observe(&state, &input));
        }
        // ③ 消失 slot（无腿且无候选）从簿清除——持仓/观测边界不跨 bar 残留。
        self.voices.retain(|k, _| live_keys.contains(k));
    }

    /// 分歧记录（仅 `kind != Match`）。
    pub(crate) fn records(&self) -> &[ShadowStepRecord] {
        &self.records
    }

    /// 全量统计。
    pub(crate) fn stats(&self) -> &ShadowStats {
        &self.stats
    }

    /// 分歧报告渲染（汇总 + 逐条分歧；落盘/落 resolution 共用）。
    pub(crate) fn render_report(&self) -> String {
        use std::fmt::Write;
        let st = &self.stats;
        let mut s = String::new();
        writeln!(s, "# shadow 双链比对分歧报告（#196 阶段 A，零行为变更）").unwrap();
        writeln!(
            s,
            "voice_steps={} matches={} divergences={}",
            st.voice_steps,
            st.matches,
            st.divergences()
        )
        .unwrap();
        writeln!(s, "  exit_typed_mismatch={}", st.exit_typed_mismatch).unwrap();
        writeln!(s, "  channel_exit_production_hold={}", st.channel_exit_production_hold).unwrap();
        writeln!(s, "  channel_hold_production_exit={}", st.channel_hold_production_exit).unwrap();
        writeln!(s, "  channel_open_production_idle={}", st.channel_open_production_idle).unwrap();
        writeln!(s, "  channel_hold_production_open={}", st.channel_hold_production_open).unwrap();
        writeln!(s, "  channel_only={}", st.channel_only).unwrap();
        writeln!(s, "  production_silent_drop={}", st.production_silent_drop).unwrap();
        writeln!(s, "## 逐条分歧（bar, slot, leg, channel, decision, production, kind）").unwrap();
        for r in &self.records {
            writeln!(
                s,
                "bar={} slot=(L{},{:?}) leg={:?} channel={:?} decision={:?} production={:?} kind={:?}",
                r.bar, r.voice_level, r.voice_dir, r.leg_id, r.channel, r.decision,
                r.production, r.kind
            )
            .unwrap();
        }
        s
    }
}

/// 生产事实推导（#201 阶段 B：持仓声部由 `StepTrace.verdicts` 显式裁决序列**单源**推导——
/// trace/裁决层统一，与旧五桶推导恒等：closed→其 typed、risk_exits→RiskExit、延续→Hold）。
/// `CloseShortDiff` 的源头区分（TW P2 overlay vs 规则2 短差关闭）仍查 `overlay_closes` 保留桶
/// （与 typed ledger 粒度一致）；空仓声部按 slot 查 `opened` 保留桶。裁决序列无记录 = §13
/// 结构剪除（silent_drops 轨，非裁决——组合层不变量 `prev_active = verdicts ⊎ silent_drops`）
/// 或序列缺口（防御：不吞异常，如实归 SilentDropped 落分歧）。
fn production_fact(
    leg: Option<ActiveLeg>,
    key: VoiceKey,
    verdicts: &HashMap<ElementId, ExitType>,
    overlay: &HashSet<ElementId>,
    opened: &HashSet<VoiceKey>,
) -> ProductionFact {
    match leg {
        Some(l) => match verdicts.get(&l.id) {
            Some(ExitType::Hold) => ProductionFact::Held,
            Some(ExitType::RiskExit) => ProductionFact::RiskExited,
            Some(ExitType::CloseShortDiff) if overlay.contains(&l.id) => {
                ProductionFact::OverlayClosed
            }
            Some(e) => ProductionFact::Closed(*e),
            None => ProductionFact::SilentDropped,
        },
        None => {
            if opened.contains(&key) {
                ProductionFact::Opened
            } else {
                ProductionFact::Idle
            }
        }
    }
}

/// 裁决 × 事实 → 分歧类别（全定义 match；`Match` 之外即分歧，只计数/落记录不改状态）。
fn classify(dec: ChannelDecision, fact: ProductionFact) -> DivergenceKind {
    use ChannelDecision as D;
    use DivergenceKind as K;
    use ProductionFact as F;
    match (dec, fact) {
        // 证书出场：typed 一致 Match，不一致 ExitTypedMismatch（单源 reverse_exit_type 下
        // 正常数据不应出现——出现即两链 S2 二分/entry_v 口径裂口的见证）。
        (D::Exit(a), F::Closed(b))
            if matches!(a, ExitType::CloseRoot | ExitType::ReduceCore | ExitType::CloseShortDiff) =>
        {
            if a == b {
                K::Match
            } else {
                K::ExitTypedMismatch
            }
        }
        (D::Exit(ExitType::RiskExit), F::RiskExited) => K::Match,
        // P1 对空仓 slot：无仓可平，裁决与 Idle 事实同效。
        (D::Exit(ExitType::RiskExit), F::Idle) => K::Match,
        // TW overlay 关短差（channel 无 TW 槽的防御对齐；当前谓词结构下不可达）。
        (D::Exit(ExitType::CloseShortDiff), F::OverlayClosed) => K::Match,
        (D::Exit(ExitType::Hold), F::Held | F::Idle) => K::Match,
        (D::Exit(ExitType::Hold), F::Closed(_) | F::RiskExited | F::OverlayClosed) => {
            K::ChannelHoldProductionExit
        }
        (D::Exit(ExitType::Hold), F::SilentDropped) => K::ProductionSilentDrop,
        (D::Exit(ExitType::Hold), F::Opened) => K::ChannelHoldProductionOpen,
        // channel 裁出场，生产静默剪：AncOK/Stale 缺口优先归类（channel 无结构剪枝语义）。
        (D::Exit(_), F::SilentDropped) => K::ProductionSilentDrop,
        (D::Exit(_), F::Held | F::Idle) => K::ChannelExitProductionHold,
        // 出场裁决 vs 其余事实（typed 跨类：RiskExit↔Closed 等，正常数据不可达）。
        (D::Exit(_), _) => K::ExitTypedMismatch,
        (D::Open, F::Opened) => K::Match,
        // channel 裁开仓，生产未开：§13 AncOK 准入门剪除（channel 无 AncOK 语义，预期缺口）。
        (D::Open, F::Idle) => K::ChannelOpenProductionIdle,
        // 持仓声部裁 Open 不可达（P6 护栏 leg.is_none()），防御归 ChannelOnly。
        (D::Open, _) => K::ChannelOnly,
        // Record/AddPosition：生产落点不在 channel 域（P7 记录桶生产无落点，P8 占位
        // 不可达）——票面明知的语义缺口。P5 `OpenShortDiff` 已随 #282 删除，不再出现于本臂。
        (D::Record | D::AddPosition, _) => K::ChannelOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::coverage::{Dir, GradeRel, Horizontal, OperationRole};
    use crate::theta_v0::types::BspBits;

    // ── 构造器（对齐 channel.rs tests 的烤料口径）──────────────────────────

    fn role(v: Vertical) -> OperationRole {
        OperationRole { h: Horizontal::First, v, delta: Dir::Plus, grade: GradeRel::SameLevel }
    }

    fn buy(k: u8) -> BspBits {
        match k {
            1 => BspBits { buy1: true, ..Default::default() },
            2 => BspBits { buy2: true, ..Default::default() },
            _ => BspBits { buy3: true, ..Default::default() },
        }
    }

    fn sell(k: u8) -> BspBits {
        match k {
            1 => BspBits { sell1: true, ..Default::default() },
            2 => BspBits { sell2: true, ..Default::default() },
            _ => BspBits { sell3: true, ..Default::default() },
        }
    }

    fn cand(gi: usize, level: u32, dir: VoiceSide, cls: u8, bits: BspBits) -> Candidate {
        Candidate {
            level,
            source_index: 10 + gi,
            bits,
            dir,
            bsp_class: cls,
            role: role(Vertical::Ambient),
            nest_confirmed: true,
            gamma_index: gi,
            force: None,
        }
    }

    fn leg(level: u32, dir: VoiceSide, ordinal: u64) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: ordinal as usize,
            lambda: ordinal as usize,
            id: ElementId { level, ordinal },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    /// 全 Hold 夹具：#201 起持仓声部事实由 `trace.verdicts` 单源推导——延续腿须显式携
    /// Hold 裁决（与生产四 return 点同口径：延续 = Hold，prev_active 次序）。
    fn hold_trace(legs: &[ActiveLeg]) -> StepTrace {
        StepTrace {
            verdicts: legs
                .iter()
                .map(|&leg| VoiceVerdict { leg, exit: ExitType::Hold })
                .collect(),
            ..Default::default()
        }
    }

    // ── slice 1：活动集 → 声部构造/持久化 ─────────────────────────────────

    /// 活动集（含同级异向两腿）映射为独立 slot；同 id 腿跨 bar 延续 ⟹ P7 检测器/步计数
    /// 持久（非每 bar 重置）；无候选无反向 ⟹ channel 裁 Hold 与生产 Held 全 Match。
    #[test]
    fn active_set_maps_to_slots_and_tracker_persists_across_bars() {
        let mut book = ShadowVoiceBook::default();
        let long_l1 = leg(1, VoiceSide::Long, 5);
        let short_l1 = leg(1, VoiceSide::Short, 6); // 同级异向 = 独立 slot
        let entry_v = HashMap::new();
        let active = vec![long_l1, short_l1];

        for bar in 0..3 {
            book.observe_and_compare(
                bar, &active, &[], &entry_v, false, &[], &hold_trace(&active),
            );
        }
        // 两 slot 各自持久；step 跨 bar 连续推进（bar0 裁于 step0 → 推进；bar2 后 step=3）。
        assert_eq!(book.voices.len(), 2, "同级异向两腿 = 两个独立声部 slot");
        for key in [
            VoiceKey { level: 1, dir: VoiceSide::Long },
            VoiceKey { level: 1, dir: VoiceSide::Short },
        ] {
            let voice = book.voices.get(&key).expect("slot 持久在簿");
            assert_eq!(voice.step, 3, "同 id 腿延续 ⟹ 步计数跨 bar 连续（key={key:?}）");
            assert_eq!(voice.leg.map(|l| l.id), active.iter().find(|l| l.dir == key.dir).map(|l| l.id));
        }
        // 无候选无反向 ⟹ 每 slot 每 bar 裁 Hold ⟷ 生产 Held：6 步全 Match、零分歧记录。
        assert_eq!(book.stats().voice_steps, 6);
        assert_eq!(book.stats().matches, 6);
        assert_eq!(book.stats().divergences(), 0);
        assert!(book.records().is_empty(), "Match 只计数不落记录");
    }

    /// 持仓边界（腿 id 切换/腿消失）⟹ 检测器与步计数重置（「本级持仓期间」字面边界）；
    /// slot 消失（无腿且无候选）⟹ 从簿清除。
    #[test]
    fn holding_boundary_resets_tracker_and_vanished_slot_is_evicted() {
        let mut book = ShadowVoiceBook::default();
        let entry_v = HashMap::new();
        let leg_a = leg(1, VoiceSide::Long, 5);
        book.observe_and_compare(0, &[leg_a], &[], &entry_v, false, &[], &hold_trace(&[leg_a]));
        book.observe_and_compare(1, &[leg_a], &[], &entry_v, false, &[], &hold_trace(&[leg_a]));
        let key = VoiceKey { level: 1, dir: VoiceSide::Long };
        assert_eq!(book.voices[&key].step, 2);

        // bar2：同 slot 换腿（id ordinal 5→7）⟹ 新持仓期，step 重置为 0 后推进到 1。
        let leg_b = leg(1, VoiceSide::Long, 7);
        book.observe_and_compare(2, &[leg_b], &[], &entry_v, false, &[], &hold_trace(&[leg_b]));
        assert_eq!(book.voices[&key].step, 1, "腿 id 切换 = 新持仓期 ⟹ 步计数重置");
        assert_eq!(book.voices[&key].leg.map(|l| l.id), Some(leg_b.id));

        // bar3：腿消失且无候选 ⟹ slot 从簿清除（下 bar 无持久状态残留）。
        book.observe_and_compare(3, &[], &[], &entry_v, false, &[], &hold_trace(&[]));
        assert!(book.voices.is_empty(), "无腿无候选的 slot 不残留在簿");
    }

    // ── slice 2：比对规则（classify × production_fact 全 kind）──────────────

    /// classify 全分支对照表（独立真值来源 = 裁决×事实交叉表的语义定义，非重算）。
    #[test]
    fn classify_covers_full_decision_fact_cross_table() {
        use ChannelDecision as D;
        use DivergenceKind as K;
        use ProductionFact as F;
        let cases: &[(ChannelDecision, ProductionFact, DivergenceKind)] = &[
            // 出场 typed 一致/不一致。
            (D::Exit(ExitType::CloseRoot), F::Closed(ExitType::CloseRoot), K::Match),
            (D::Exit(ExitType::ReduceCore), F::Closed(ExitType::ReduceCore), K::Match),
            (D::Exit(ExitType::CloseRoot), F::Closed(ExitType::ReduceCore), K::ExitTypedMismatch),
            (D::Exit(ExitType::CloseShortDiff), F::Closed(ExitType::CloseShortDiff), K::Match),
            // P1。
            (D::Exit(ExitType::RiskExit), F::RiskExited, K::Match),
            (D::Exit(ExitType::RiskExit), F::Idle, K::Match),
            (D::Exit(ExitType::RiskExit), F::Held, K::ChannelExitProductionHold),
            // Hold 各侧。
            (D::Exit(ExitType::Hold), F::Held, K::Match),
            (D::Exit(ExitType::Hold), F::Idle, K::Match),
            (D::Exit(ExitType::Hold), F::Closed(ExitType::CloseRoot), K::ChannelHoldProductionExit),
            (D::Exit(ExitType::Hold), F::OverlayClosed, K::ChannelHoldProductionExit),
            (D::Exit(ExitType::Hold), F::SilentDropped, K::ProductionSilentDrop),
            (D::Exit(ExitType::Hold), F::Opened, K::ChannelHoldProductionOpen),
            // 出场 vs 持有/剪除。
            (D::Exit(ExitType::CloseRoot), F::Held, K::ChannelExitProductionHold),
            (D::Exit(ExitType::CloseRoot), F::SilentDropped, K::ProductionSilentDrop),
            (D::Exit(ExitType::CloseShortDiff), F::OverlayClosed, K::Match),
            (D::Exit(ExitType::ReduceCore), F::RiskExited, K::ExitTypedMismatch),
            // 开仓。
            (D::Open, F::Opened, K::Match),
            (D::Open, F::Idle, K::ChannelOpenProductionIdle),
            (D::Open, F::Held, K::ChannelOnly), // 防御不可达分支
            // channel 独有裁决（P5 `OpenShortDiff` 已随 #282 删除，不在本表）。
            (D::Record, F::Held, K::ChannelOnly),
            (D::AddPosition, F::Held, K::ChannelOnly),
        ];
        for (dec, fact, want) in cases {
            assert_eq!(classify(*dec, *fact), *want, "({dec:?}, {fact:?})");
        }
    }

    /// 端到端样例（T2 跨级同构断言思路的全量化核心）：持仓 Long L1 + 同级一类反向已确认证书
    /// ⟹ 持仓声部裁 Exit(CloseRoot)；生产 closed 携同 typed ⟹ 该 slot Match。
    /// 同候选在空仓 (L1,Short) slot 裁 Open——散装 fold 规则2 已将其消费为关闭触发
    /// （不入 open 桶），channel 声部独立语义 vs fold 消费语义的结构差如实落分歧记录。
    /// 第二条链：生产未关（Held）⟹ ChannelExitProductionHold 分歧落记录。
    #[test]
    fn reverse_certificate_close_matches_production_or_records_divergence() {
        let held = leg(1, VoiceSide::Long, 5);
        let trigger = cand(0, 1, VoiceSide::Short, 1, sell(1));
        let gamma = vec![trigger];
        let entry_v = HashMap::new();

        // ① 生产同关同 typed ⟹ 持仓 slot Match；空仓 slot Open-vs-Idle 结构差落分歧。
        //    #201：closed 桶与 verdicts 同填（生产四 return 点同口径——closed 腿携其 typed）。
        let mut book = ShadowVoiceBook::default();
        let trace = StepTrace {
            closed: vec![(held, trigger, ExitType::CloseRoot)],
            verdicts: vec![VoiceVerdict { leg: held, exit: ExitType::CloseRoot }],
            ..Default::default()
        };
        book.observe_and_compare(7, &[held], &gamma, &entry_v, false, &[], &trace);
        assert_eq!(book.stats().voice_steps, 2, "持仓 slot + 反向候选的空仓 slot");
        assert_eq!(book.stats().matches, 1, "同构关闭 slot Match（T2 同构思路全量化）");
        assert_eq!(
            book.stats().channel_open_production_idle, 1,
            "候选被 fold 规则2 消费为关闭触发 ⟹ 空仓 slot Open 无生产对应"
        );
        assert_eq!(book.records().len(), 1);

        // ② 生产未关 ⟹ 两 slot 各落一条分歧（HashSet 序不定，按 kind 检索）。
        let mut book = ShadowVoiceBook::default();
        book.observe_and_compare(7, &[held], &gamma, &entry_v, false, &[], &hold_trace(&[held]));
        assert_eq!(book.stats().divergences(), 2);
        assert_eq!(book.stats().channel_exit_production_hold, 1);
        assert_eq!(book.stats().channel_open_production_idle, 1);
        let r = book
            .records()
            .iter()
            .find(|r| r.kind == DivergenceKind::ChannelExitProductionHold)
            .expect("持仓 slot 分歧在册");
        assert_eq!(r.bar, 7);
        assert_eq!((r.voice_level, r.voice_dir), (1, VoiceSide::Long));
        assert_eq!(r.leg_id, Some(held.id));
        assert_eq!(r.decision, ChannelDecision::Exit(ExitType::CloseRoot));
        assert_eq!(r.production, ProductionFact::Held);
        // 报告渲染含汇总与逐条。
        let report = book.render_report();
        assert!(report.contains("divergences=2"));
        assert!(report.contains("ChannelExitProductionHold"));
    }

    /// 空仓 slot：channel 裁 Open；生产 opened 同 slot ⟹ Match，否则 ChannelOpenProductionIdle。
    #[test]
    fn open_decision_matches_opened_slot_or_records_idle() {
        let trigger = cand(0, 0, VoiceSide::Long, 1, buy(1));
        let gamma = vec![trigger];
        let entry_v = HashMap::new();
        let new_leg = leg(0, VoiceSide::Long, 10);

        let mut book = ShadowVoiceBook::default();
        let trace = StepTrace { opened: vec![(trigger, new_leg)], ..Default::default() };
        book.observe_and_compare(0, &[], &gamma, &entry_v, false, &[], &trace);
        assert_eq!(book.stats().matches, 1, "channel Open ⟷ 生产 Opened 同 slot");

        let mut book = ShadowVoiceBook::default();
        book.observe_and_compare(0, &[], &gamma, &entry_v, false, &[], &hold_trace(&[]));
        assert_eq!(book.stats().channel_open_production_idle, 1, "AncOK 剪除类缺口");
        assert_eq!(book.records()[0].kind, DivergenceKind::ChannelOpenProductionIdle);
    }

    /// P1：force_flat ⟹ 持仓声部裁 Exit(RiskExit) ⟷ 生产 risk_exits 全 Match；
    /// 空仓 slot 的 P1 裁决对 Idle 同效（Match）。
    #[test]
    fn force_flat_p1_matches_risk_exits() {
        let held = leg(1, VoiceSide::Long, 5);
        let trigger = cand(0, 0, VoiceSide::Long, 1, buy(1));
        let gamma = vec![trigger];
        let entry_v = HashMap::new();
        // #201：risk_exits 桶与 verdicts 同填（生产 P1 分支同口径——每声部恰一枚 RiskExit）。
        let trace = StepTrace {
            risk_exits: vec![held],
            verdicts: vec![VoiceVerdict { leg: held, exit: ExitType::RiskExit }],
            ..Default::default()
        };
        let mut book = ShadowVoiceBook::default();
        book.observe_and_compare(0, &[held], &gamma, &entry_v, true, &[], &trace);
        assert_eq!(book.stats().voice_steps, 2, "持仓 slot + 空仓候选 slot 各一枚裁决");
        assert_eq!(book.stats().matches, 2, "P1 双链同裁（全互斥 C1 屏蔽）");
        assert!(book.records().is_empty());
    }
}
