//! 观察契约（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 观察契约（卡 §3/§4.3：活窗与结构完成信号由调用方喂入）
// ═══════════════════════════════════════════════════════════════════════════

/// pan 活窗契约（调用方喂入；卡 §3）。
///
/// 行进中 c 窗 = `[c_start_live, live_end]`：`c_start_live` = 中枢最近确认段后首个离开段
/// 起点，与完成后 `structure.seg_c.0` 同锚——段序对齐校验由身份键自然兑现：若完成事件的
/// `seg_c.0 ≠ c_start_live`，桥不判同身份，该 key 走身份消失路径（诚实记 Invalidated
/// 而非改锚）。右端 `live_end` 随 as_of 前进（设计内行为，卡 §3；桥记 Supersedes 吸收）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanLiveWindow {
    pub level: u32,
    pub side: Side,
    pub seg_a: (usize, usize),
    /// 活窗 c 段源坐标闭区间 `(c_start_live, live_end)`。
    pub seg_c_live: (usize, usize),
    pub b_center_start: usize,
    /// 诊断字段（票 #592 裁定）：frontier 起点与最近 confirmed 段末端的洞长（无洞=0），
    /// 与 [`provide_active_pan_live_windows`] 内 `frontier_not_after_confirmed` 判定
    /// 同源同值——源头照实全收、拒绝判断留给消费端（账本照实记录哲学，不在本字段上二次
    /// 加判据）。非 L1 active-frontier 通道（`provide_pan_live_windows`/完成事件闪现）不涉及
    /// frontier 概念，恒 `0`。
    pub gap_len: usize,
}

/// 活窗右端判据（票 #604 单一权威；原四处独立复刻 `as_of.max(c_start)` 收敛于此）。
///
/// `seg_c_live` 右端 = `as_of` 与 `c_start` 的较大值：正常前进中 `as_of ≥ c_start`，
/// 右端随 `as_of` 推进（[`PanLiveWindow`] 文档「右端随 as_of 前进」的落地）；`as_of == c_start`
/// 两值相等语义无差。钳位分支（`as_of < c_start`）为防御性兜底——2026-07-28 影子评审
/// 探针实测 20k/100k 零命中，生产可达性未证实（声明等级：防御非实测路径）；
/// 命中时退化为单点区间 `[c_start, c_start]`，防止产出右端早于左端的倒挂区间。
pub fn active_window_right_edge(c_start: usize, as_of: usize) -> usize {
    as_of.max(c_start)
}

/// 推进观察（advance 的唯一输入；确定性结构/力度谓词，v3 硬禁令合规）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifecycleObservation {
    /// 事件通道：provider 事件本体（trend / pan 完成事件）。力度恒
    /// `Verified(divergence_confirmed)`（事件通道不做活窗现算——#78 修复 1 后
    /// 「Verified 路径逐 bit 不变」语义一致）。
    Event {
        event: NestCandidateEvent,
        /// c 结构完成信号（卡 §4.3：trend = Active→Completed / SegmentTermination；
        /// pan = seg_c 进入已完成段集——判据属调用方，本模块不越权自判）。
        structure_completed: bool,
    },
    /// pan 活窗通道：行进中 c 窗。力度在活窗上三值化现算（`segments_diverge_or`
    /// 单一力度引擎，禁第二查法）。
    PanLive {
        window: PanLiveWindow,
        structure_completed: bool,
    },
}

impl LifecycleObservation {
    /// 事件通道构造。
    pub fn event(event: NestCandidateEvent, structure_completed: bool) -> Self {
        Self::Event {
            event,
            structure_completed,
        }
    }

    /// pan 活窗通道构造。
    pub fn pan_live(window: PanLiveWindow, structure_completed: bool) -> Self {
        Self::PanLive {
            window,
            structure_completed,
        }
    }

    /// 身份键（E2E §6.1:248：键不含状态/钟/行进中区间——trend 域 seg_c_full 取事件
    /// 产出坐标 `interval_b` 是工程桥口径，模块头 090 登记 1）。
    pub(super) fn key(&self) -> LifecycleKey {
        match self {
            Self::Event { event, .. } => LifecycleKey {
                level: event.level,
                side: event.side,
                kind: event.kind,
                seg_a: event.seg_a,
                seg_c_full: event.interval_b,
                b_center_start: event.b_center_start,
            },
            Self::PanLive { window, .. } => LifecycleKey {
                level: window.level,
                side: window.side,
                kind: NestDivergenceKind::Consolidation,
                seg_a: window.seg_a,
                seg_c_full: window.seg_c_live,
                b_center_start: window.b_center_start,
            },
        }
    }

    fn side(&self) -> Side {
        match self {
            Self::Event { event, .. } => event.side,
            Self::PanLive { window, .. } => window.side,
        }
    }

    pub(super) fn structure_completed(&self) -> bool {
        match self {
            Self::Event {
                structure_completed,
                ..
            }
            | Self::PanLive {
                structure_completed,
                ..
            } => *structure_completed,
        }
    }

    /// 证据窗口（源坐标）：(seg_a, seg_c 当前形态)。
    fn evidence_windows(&self) -> ((usize, usize), (usize, usize)) {
        match self {
            Self::Event { event, .. } => (event.seg_a, event.interval_b),
            Self::PanLive { window, .. } => (window.seg_a, window.seg_c_live),
        }
    }

    /// 力度求值：事件通道恒 `Verified(divergence_confirmed)`（布尔口径一个 bit 不动）；
    /// 活窗三值化现算（缺 hist/dif ⟹ MissingForceSeries；映射失败 ⟹ CoordinateMapFailed；
    /// 齐备 ⟹ `Verified(segments_diverge_or(...))`——divergence.rs 单一力度引擎）。
    pub(super) fn force(&self, material: &ForceMaterial) -> ForceCheck {
        match self {
            // 事件通道恒 Verified（divergence_confirmed 布尔口径一个 bit 不动，卡 §5.2）。
            Self::Event { event, .. } => ForceCheck::Verified(event.divergence_confirmed),
            // 活窗三值化现算（#78 修复 1 全量）。
            Self::PanLive { window, .. } => {
                let (Some(hist), Some(dif)) = (material.hist, material.dif) else {
                    return ForceCheck::Unavailable(UnavailReason::MissingForceSeries);
                };
                let (Some(a), Some(c)) = (
                    map_src_to_close_idx(material.close_src, window.seg_a.0, window.seg_a.1),
                    map_src_to_close_idx(
                        material.close_src,
                        window.seg_c_live.0,
                        window.seg_c_live.1,
                    ),
                ) else {
                    return ForceCheck::Unavailable(UnavailReason::CoordinateMapFailed);
                };
                ForceCheck::Verified(segments_diverge_or(hist, dif, window.side, a, c))
            }
        }
    }

    /// 反超证据现算（`segments_diverge_or` 同组原语逐通道取值；材料/映射缺 ⟹ None
    /// 诚实缺证）。仅审计载荷，不进真值路径。
    pub(super) fn force_evidence(&self, material: &ForceMaterial) -> Option<ForceEvidence> {
        let (hist, dif) = (material.hist?, material.dif?);
        let (seg_a, seg_c) = self.evidence_windows();
        let a = map_src_to_close_idx(material.close_src, seg_a.0, seg_a.1)?;
        let c = map_src_to_close_idx(material.close_src, seg_c.0, seg_c.1)?;
        let side = self.side();
        let direction = match side {
            Side::Long => Direction::Down,
            Side::Short => Direction::Up,
        };
        Some(ForceEvidence {
            area_a: same_color_area(hist, a.0, a.1, side),
            area_c: same_color_area(hist, c.0, c.1, side),
            dif_peak_a: segment_dif_peak(dif, a.0, a.1, direction),
            dif_peak_c: segment_dif_peak(dif, c.0, c.1, direction),
            hist_peak_a: same_dir_hist_peak(hist, a.0, a.1, side),
            hist_peak_c: same_dir_hist_peak(hist, c.0, c.1, side),
        })
    }
}

/// 力度序列材料（活窗现算与反超证据的输入；缺 hist/dif ⟹ MissingForceSeries）。
#[derive(Debug, Clone, Copy)]
pub struct ForceMaterial<'a> {
    pub hist: Option<&'a [f64]>,
    pub dif: Option<&'a [f64]>,
    /// `merged_bars` 下标 → source_index 升序映射（`map_src_to_close_idx` 同口径）。
    pub close_src: &'a [usize],
}

impl<'a> ForceMaterial<'a> {
    /// 无力度序列材料（事件通道够用——恒 Verified；活窗通道得 MissingForceSeries 注记）。
    pub fn unavailable(close_src: &'a [usize]) -> Self {
        Self {
            hist: None,
            dif: None,
            close_src,
        }
    }
}
