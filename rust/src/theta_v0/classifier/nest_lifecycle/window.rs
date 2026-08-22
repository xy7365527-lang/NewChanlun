//! L1/L2 active window frontier 与批量观测（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// L1 层 active window frontier（票 #601：L2 完成前活窗可见；#598 裁定路线 i）
// ═══════════════════════════════════════════════════════════════════════════

/// **行进中的 L1 窗口单元**（票 #601）：当下若把行进中的 L0 段计入，L1 层**会形成、但塔上
/// 尚不存在**的候选窗口。它是 L2 活窗唯一合法的 C 腿。
///
/// 数据源纪律（票 #523 永禁清单在 L2 的对应条款）：本载体只能由
/// 「`tower[0]` 的 confirmed units + [`ActiveSegmentFrontier`] 虚拟追加单元」经**重跑同一
/// 窗口扫描判据**（`detect_centers_windowed_resume` + `center_from_segments`，L1 层 build）
/// 派生——**禁止**把 `tower[1]` 的任何已产出窗口（含协议性每 bar 重扫的末窗）当作行进中单元。
/// 后者已被 `lower_legs_from(tower[1])` 供给完成事件路径，拿它冒充活动 C = 与完成事件同源
/// 同判 = 逐字重演 #523 的「首见即完成」恒等式。
///
/// 判别行进中的充要形态（与 `WinMeta.read_end_src == usize::MAX` 等价、但不依赖该私有侧车）：
/// 重扫产出的**末窗必须含虚拟追加单元**（`win.1 + 1 == units.len()`）。窗口右端落在虚拟单元上
/// ⟺ 扫描因数据耗尽而停（无 non-extension 哨兵）⟺ 该窗口仍可能因后续数据改变；反之末窗
/// 由纯 confirmed 单元构成 ⟹ 它与 `tower[1]` 中已有的窗口同源，判 [`ActiveWindowOutcome::LowerFrontierNotAbsorbed`]，
/// 不产活动腿。
///
/// 与 confirmed L1 单元的关系：`start_index` = 窗口首单元起点、`direction` = 窗口首单元方向
/// （first-leaf 口径，与 `level_view::lower_legs_from` 的 `first_leaf_direction` 逐字同源）、
/// `lo`/`hi` = 窗口聚合外缘（= 该窗若 compose 后的 `rmove.lo()/hi()`，见
/// `recursive_tower::LeveledMove::envelope` 的投影契约）。一旦该窗口被塔 emit 为 `tower[1]`
/// 的确认单元，本 frontier 即消失，由完成事件接手。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveWindowFrontier {
    /// 窗口首单元方向（first-leaf 口径，与 `lower_legs_from` 同源）。
    pub direction: Direction,
    /// 窗口首单元起点源坐标。
    pub start_index: usize,
    /// 窗口末单元终点源坐标（= 行进中 L0 段的极值结构点；禁 as_of 冒充结构点）。
    pub end_index: usize,
    /// 窗口聚合外缘下沿。
    pub lo: Tick,
    /// 窗口聚合外缘上沿。
    pub hi: Tick,
}

impl ActiveWindowFrontier {
    /// 行进中 L1 单元的当下段形态（口径与 `level_view::leg_as_segment` /
    /// `p123_fast_replay::lifecycle_leg_as_segment` 逐字相同：方向定端点价的取序）。
    pub fn as_segment(&self) -> Segment {
        let (start_price, end_price) = match self.direction {
            Direction::Up => (self.lo, self.hi),
            Direction::Down => (self.hi, self.lo),
        };
        Segment {
            direction: self.direction,
            start_index: self.start_index,
            end_index: self.end_index,
            start_price,
            end_price,
        }
    }
}

/// [`active_l1_window_frontier`] / [`active_l2_window_frontier`] 的结果与**未产出原因码**
/// （票 #601 立、票 #602 扩 L2 层；诊断面，不进真值路径）。
///
/// 原因码回答「这只 L2/L3 完成身份为什么没有更早的 Live」——塔层特有的缺口在此照实命名，
/// 不并入 L1 的 [`PanLiveOutcome`] 码表（两者定位的是不同层的失败：本枚举失败在
/// 「下一级根本没有行进中单元可作 C」，`PanLiveOutcome` 失败在「有 C 但 A/B 结构定位不成」）。
///
/// **码表跨级共用、计数禁合并**：本枚举的语义是级别中立的（「行进中下级单元」在 L2 指
/// L1 窗口单元、在 L3 指 L2 窗口单元），故不为 L3 另起一套同义码——那是同一失败换名，
/// 属声明膨胀。消费方按 `(level, reason_tag)` 分列计数（`p123_fast_replay` 已如此）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWindowOutcome {
    /// 派生成功：行进中的本级窗口单元。
    Frontier(ActiveWindowFrontier),
    /// 下一级无行进中单元（L2：parser 无 `PendingSegment`；L3：L1 层活动窗口未派生成功
    /// ——**其具体成因在同一 bar 的 L2 诊断行上**，本码不复述，见函数文档）⟹ 无可虚拟
    /// 追加的单元。
    NoLowerFrontier,
    /// 行进中下级单元与本级 confirmed units 序不自洽（起点落在末单元内部）⟹ 拒绝，诚实空产出。
    LowerFrontierNotAfterUnits,
    /// 本级扫描断点越过 confirmed units 数（塔与 units 不同步）⟹ 拒绝重扫，不猜锚。
    ResumeAnchorOutOfRange,
    /// 虚拟追加后自断点起重扫仍无任何窗口成立（seed 判据不过）。
    NoWindowFormed,
    /// 重扫末窗**不含**行进中下级单元 ⟹ 该窗与 `tower[level]` 已有窗口同源，拒绝外推
    /// （#523 红线）。
    LowerFrontierNotAbsorbed,
    /// 首叶方向表与 confirmed units 不等长（票 #602 只在 L2 层可达）⟹ 拒绝，不猜方向。
    /// 两者必须逐位对应同一个 `tower[level]` 元素，长度不等即调用方供给失步。
    LowerLegDirsOutOfSync,
}

impl ActiveWindowOutcome {
    /// 原因码标签（dump/统计口径单一来源）。
    pub fn reason_tag(&self) -> &'static str {
        match self {
            Self::Frontier(_) => "active_window",
            Self::NoLowerFrontier => "no_lower_frontier",
            Self::LowerFrontierNotAfterUnits => "lower_frontier_not_after_units",
            Self::ResumeAnchorOutOfRange => "resume_anchor_out_of_range",
            Self::NoWindowFormed => "no_window_formed",
            Self::LowerFrontierNotAbsorbed => "lower_frontier_not_absorbed",
            Self::LowerLegDirsOutOfSync => "lower_leg_dirs_out_of_sync",
        }
    }

    pub fn frontier(&self) -> Option<ActiveWindowFrontier> {
        match self {
            Self::Frontier(frontier) => Some(*frontier),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 票 #757：批量丢弃机制不可观测实例的专门原因码（#617 方向 A 落地，#724 裁定）
//
// **观测面 only**：本段全部类型只做「读数与命名」，不进任何判据路径——不改账本字段、
// 不改事件流、不改塔/parser 的任何产出（既有行为零变化由「只新增、不改写既有写入点」
// 结构性保证）。裁定链：#724（2026-07-29 裁定：批量丢弃影响面唯一投入点 = 机制不可观测
// 实例配专门原因码）→ #617 方向 A（先探针预分级三类：机制不可观测 / 时机边界 / 可观测，
// 三类原因码进验收，不假设塔侧队列更有效）。
// ═══════════════════════════════════════════════════════════════════════════

/// 完成身份「无 earlier Live」的三类成因码（票 #757；#617 方向 A 的三类分级落码）。
///
/// **判定顺序 = 枚举顺序**（[`BatchObservabilityTracker::classify_l1`] 唯一分类点）：
///
/// 1. [`ChainConfirmedNoPendingWindow`](Self::ChainConfirmedNoPendingWindow)（机制不可观测）
///    ——C 段从未作为 pending/frontier 出现：parser `append` 的批量确认（#617 钉界：
///    一次 `append` 的 `while` 循环可连续确认多段，批内非首段的起点在其前身确认之前
///    **不存在**，故其「行进中当下状态」在任何粒度下都不存在——教义判定：非观测缺口，
///    是状态本身不存在）。本码优先判：它是三者中唯一的教义终局判定。
/// 2. [`SameBarCenterConfirmation`](Self::SameBarCenterConfirmation)（时机边界）——C 段
///    曾真实出现在 pending/frontier（可被观测），但 B 中枢到**完成信号同 bar** 才首次
///    确认（sealed），覆盖率判据严格 `<`（活窗观测须早于完成）天然排除同 bar 命中
///    （#618 钉界：判据边界产物，非数据分裂）。
/// 3. [`ObservableWindowMissed`](Self::ObservableWindowMissed)（可观测未命中）——C 曾可
///    观测且 B 在完成前已确认：实例本可被覆盖，未覆盖归因于他处（在案主因 =
///    `structure_not_locatable` 结构定位失败，#599 §6 已判出范围、#603 桶③实测 11 只）。
///    本码**不**声称归因完成，只声称「机制上可观测」这一事实分层。
///
/// 验收锚（BTC 100k 单窗 L2 级证据，禁写成规格常量——#724 V4）：三类在 L1 身份级的
/// 既有实测读数 = 18 / 17 / 11（#603 三桶，逐只清单见其探针产物）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveMissCause {
    /// 机制不可观测：批量确认批内非首段，从未有过 pending「当下状态」（#617）。
    ChainConfirmedNoPendingWindow,
    /// 时机边界：B 中枢在完成信号同 bar 才首次 sealed（#618）。
    SameBarCenterConfirmation,
    /// 可观测未命中：C 与 B 都曾可观测，未覆盖归因他处（如结构定位失败）。
    ObservableWindowMissed,
}

impl LiveMissCause {
    /// 原因码标签（dump/统计口径单一来源，禁第二处拼写）。
    pub fn reason_tag(&self) -> &'static str {
        match self {
            Self::ChainConfirmedNoPendingWindow => "chain_confirmed_no_pending_window",
            Self::SameBarCenterConfirmation => "same_bar_center_confirmation",
            Self::ObservableWindowMissed => "observable_window_missed",
        }
    }
}

/// 塔层批量产出中被「只取末窗」丢弃的**单个窗口实例**（票 #757 观测面记录载体）。
///
/// 机制（#617 L2/L3 同构钉界）：`detect_centers_windowed_resume` 一次续扫可 push m 个
/// 成立窗口（普通连出 + #148 升级重切一窗产 k 个）；[`scan_active_window`] 只取末窗
/// ⟹ 批内 m−1 个窗口从未作为「当下的行进中窗口」存在。本类型把每一只这样的实例以
/// 源坐标区间记下（级别与 as_of 由调用方补给——本类型不携带，保持扫描内核级别中立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchDroppedWindow {
    /// 被丢窗口首单元起点源坐标。
    pub start_index: usize,
    /// 被丢窗口末单元终点源坐标。
    pub end_index: usize,
}

/// 批量观测性跟踪器（票 #757）：逐 bar 累计「哪些 C 曾真实 pending」与「各级中枢首次
/// sealed 的 bar」，供完成身份的三类成因分级（[`LiveMissCause`]）。
///
/// **观测面 only**：所有输入来自调用方既有的只读视图（parser frontier / 塔 sealed 前缀
/// 水线），本跟踪器不回写任何生产状态。两条口径与既有实测严格对齐：
///
/// - `ever_pending` 的采样点 = [`active_segment_frontier`] 的 `Some` 返回——与 #603 桶①
///   「C 段从未进过队列」的探针口径逐字同源（队列喂数点同一函数），故 18 只读数可对拍；
/// - `center_sealed_first_seen` 只数 **sealed 前缀**（`tower[level][..confirmed_len]`，
///   确认水线 = #93 单一来源）内的中枢首现 bar——与 #618「完成信号同 bar 首次被确认」
///   的「确认」同口径（frontier 批内的开放单元不算确认）。
#[derive(Debug, Clone, Default)]
pub struct BatchObservabilityTracker {
    /// 曾作为行进中 C 被观测到的 L0 段起点集合（`active_segment_frontier` 逐 bar 采样）。
    ever_pending: BTreeSet<usize>,
    /// `(level, 中枢起点源坐标)` → 首次进入 sealed 前缀的 bar。首写不后移（rollback 后
    /// 重新 sealed 不改写——首确认时刻是历史事实）。
    center_sealed_first_seen: BTreeMap<(u32, usize), usize>,
    /// 各级已扫描的 sealed 前缀长度（增量游标；水线回缩时降级为 min——回缩区间内
    /// 重 sealed 的元素首见 bar 保原值，见上字段文档）。
    center_scanned: BTreeMap<u32, usize>,
}

impl BatchObservabilityTracker {
    /// 逐 bar 喂 L0 行进中 C frontier（`compute_lifecycle_due_keys` 每 bar 已算，零新增成本）。
    pub fn observe_frontier(&mut self, frontier: Option<&ActiveSegmentFrontier>) {
        if let Some(frontier) = frontier {
            self.ever_pending.insert(frontier.start_index);
        }
    }

    /// 逐 bar 喂某级塔的 sealed 前缀（`moves = tower[level]`，`sealed_len` = 确认水线，
    /// `as_of` = 当前 bar）。
    ///
    /// 增量扫描：每级只扫 `[scanned, sealed_len)` 的新 sealed 元素，O(新 sealed 数)/bar。
    /// 水线回缩（cascade bar）时游标降级为 `min(scanned, sealed_len)`，不重扫前缀；
    /// 回缩后重新 sealed 的元素首见 bar 保**原值**（首确认是历史事实，entry 首写不后移）。
    pub fn observe_tower_level(
        &mut self,
        level: u32,
        moves: &[LeveledMove],
        sealed_len: usize,
        as_of: usize,
    ) {
        let sealed_len = sealed_len.min(moves.len());
        let scanned = self.center_scanned.entry(level).or_insert(0);
        *scanned = (*scanned).min(sealed_len);
        for mv in &moves[*scanned..sealed_len] {
            self.center_sealed_first_seen
                .entry((level, mv.start_index))
                .or_insert(as_of);
        }
        *scanned = sealed_len;
    }

    /// L1 完成身份的三类成因分级（票 #757 唯一分类点；判定顺序见 [`LiveMissCause`]）。
    ///
    /// **调用契约**：`key` 必须是 level==1 的 pan 域身份（`seg_c_full.0` = C 段 L0 源坐标
    /// 起点、`b_center_start` = B 中枢在 `tower[1]` 的起点快照）；`signal_at` = 该身份的
    /// `structure_end_at`（c 结构完成信号首次到达的 bar）。调用方保证「无 earlier Live」
    /// （`observed_at >= signal_at`）——本函数只分级，不判覆盖。
    ///
    /// **级别范围（诚实登记）**：C 腿 pending 口径（`ever_pending`）只覆盖 L1 身份
    /// （C = L0 段）。L2/L3 身份的 C 是窗口单元，其机制不可观测面由塔层批量丢弃记录
    /// （[`BatchDroppedWindow`]）承载，身份级三类分级不在本票 Scope（#724 V3：L4+ 若立项
    /// 须第一天内建同级别口径）。
    pub fn classify_l1(&self, key: &LifecycleKey, signal_at: usize) -> LiveMissCause {
        if !self.ever_pending.contains(&key.seg_c_full.0) {
            return LiveMissCause::ChainConfirmedNoPendingWindow;
        }
        if self
            .center_sealed_first_seen
            .get(&(key.level, key.b_center_start))
            .is_some_and(|&sealed_at| sealed_at >= signal_at)
        {
            return LiveMissCause::SameBarCenterConfirmation;
        }
        LiveMissCause::ObservableWindowMissed
    }
}

/// [`scan_active_window`] 的成功产出：末窗在**扩展后**单元序列上的起点下标 + 该窗几何。
///
/// 方向**不在**本结构里：窗口首单元的「首叶方向」（`lower_legs_from` 口径）要按级别到不同
/// 的来源上取（L1 层 = L0 段单元自带方向；L2 层 = `tower[1]` 元素的 `first_leaf_direction`，
/// 与投影单元的 ownership 方向**不是**同一个值），故留给各级入口解析。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveWindowScan {
    /// 末窗起点在「confirmed units ++ 虚拟追加单元」序列上的下标。
    win_start: usize,
    /// 末窗首单元起点源坐标。
    start_index: usize,
    /// 末窗末单元终点源坐标（= 虚拟追加单元的终点，见吸收判据）。
    end_index: usize,
    /// 窗口聚合外缘下沿（`center.dd`）。
    lo: Tick,
    /// 窗口聚合外缘上沿（`center.gg`）。
    hi: Tick,
}

/// **行进中窗口扫描内核**（票 #602 抽出；L1 层与 L2 层共用同一实装，禁第二查法）。
///
/// 算法 = 把 `virtual_unit`（行进中的下级单元，**塔上尚不存在**）作虚拟单元接在
/// `units` 尾后，从 `resume_from` **重跑与塔逐字相同的窗口扫描**
/// （[`detect_centers_windowed_resume`]，`build` 由调用方按级别钉死），取末窗。
///
/// 为什么 `resume_from` 是合法起点：它就是塔自己每 bar 用的 resume 锚
/// （`recursive_tower::WindowScanCursor` 文档的 frontier 协议），`units[..resume_from]` 的
/// 扫描路径确定且与塔一致；本函数只在其**尾部**多喂一个单元，不改前缀。
///
/// 三道守卫（全部返回可审计原因码，不 panic、不猜）：
/// 1. **衔接**：虚拟单元起点不得落在末 confirmed 单元内部（只拒**倒灌**，不拒「有洞」
///    ——#578 复核已证「有洞」在真实数据里频繁发生，收紧属教义裁决，非本票 Scope）；
/// 2. **锚在界内**：`resume_from <= units.len()`；
/// 3. **禁外推**（#523 红线）：末窗必须含虚拟追加单元（`win.1 + 1 == extended.len()`）
///    ⟺ 扫描因数据耗尽而停（无 non-extension 哨兵）⟺ 该窗口仍开放。否则末窗由纯
///    confirmed 单元构成，与 `tower[level]` 已有窗口同源，拿它冒充活动 C = 与完成事件
///    同源同判 = 逐字重演 #523 的「首见即完成」恒等式。
///
/// **只取末窗的代数后果（票 #617 同构，照实登记，不是可调项）**：一次重扫可产出 m 个成立
/// 窗口（#148 升级重切一窗产 ⌊n/3⌋ 个子中枢，或续扫连出多窗）。`windowed.last()` 只取最后
/// 一个 ⟹ 同批更早的 m−1 个窗口在**任何**粒度上都不曾作为「当下的行进中窗口」存在过——
/// 与 #617 在 parser `append` 内 while 批量确认上的结论同构：**不是观测缺口，是那个「当下
/// 状态」根本不存在**。守卫 3 又要求末窗含虚拟单元，故批内更早窗口连被拒的机会都没有
/// （它们不满足吸收判据）。本函数不为此加队列/回补：那会凭空发明塔从未处于过的状态。
fn scan_active_window(
    units: &[UnitRange],
    virtual_unit: UnitRange,
    resume_from: usize,
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    // 票 #757（观测面 only）：批内非末窗逐只落码（机制不可观测，#617 同构钉界）。
    // 记录在守卫 3 判定**之前**——无论末窗最终是否吸收虚拟单元，批内非末窗「从未作为
    // 当下行进中窗口存在」的事实不变；m < 2 时恒为空追加，零成本。
    drops: &mut Vec<BatchDroppedWindow>,
) -> Result<ActiveWindowScan, ActiveWindowOutcome> {
    if units
        .last()
        .is_some_and(|last| virtual_unit.start_index < last.end_index)
    {
        return Err(ActiveWindowOutcome::LowerFrontierNotAfterUnits);
    }
    if resume_from > units.len() {
        return Err(ActiveWindowOutcome::ResumeAnchorOutOfRange);
    }
    let mut extended = units.to_vec();
    extended.push(virtual_unit);
    let (windowed, _metas, _cursor) = detect_centers_windowed_resume(&extended, build, resume_from);
    if windowed.len() >= 2 {
        drops.extend(windowed[..windowed.len() - 1].iter().map(|(_center, win)| {
            BatchDroppedWindow {
                start_index: extended[win.0].start_index,
                end_index: extended[win.1].end_index,
            }
        }));
    }
    let Some((center, win)) = windowed.last().copied() else {
        return Err(ActiveWindowOutcome::NoWindowFormed);
    };
    if win.1 + 1 != extended.len() {
        return Err(ActiveWindowOutcome::LowerFrontierNotAbsorbed);
    }
    Ok(ActiveWindowScan {
        win_start: win.0,
        start_index: extended[win.0].start_index,
        end_index: extended[win.1].end_index,
        // `center.dd/gg` = 窗口全单元外缘聚合（detect 的 seed 三段交 + 延伸 min/max），
        // 与该窗若 compose 后的 `rmove.lo()/hi()` 逐值相等（`LeveledMove::envelope` 投影契约）。
        lo: center.dd,
        hi: center.gg,
    })
}

/// 派生**行进中的 L1 窗口单元**（票 #601 的本体；L2 活窗的 C 来源）。
///
/// 输入（三项全部只读，零写入塔）：
/// - `l0_units`：`tower[0]` 的 `UnitRange` 投影（`TowerCache::l0_units`），即 L1 层窗口扫描的
///   confirmed 输入；
/// - `l0_frontier`：parser 的行进中段（[`active_segment_frontier`]）——**塔上不存在**的那一段；
/// - `l1_resume_from`：L1 层扫描断点 `WindowScanCursor::resume_from`
///   （`TowerCache::level_scan_cursor(1)`），即「最后一个成立窗口的起点」。
///
/// 算法 = 把 `l0_frontier` 作为**虚拟追加单元**接在 `l0_units` 尾后，从 `l1_resume_from`
/// **重跑与塔逐字相同的窗口扫描**（`detect_centers_windowed_resume` + `center_from_segments`），
/// 取末窗。为什么 `resume_from` 是合法起点：它就是塔自己每 bar 用的 resume 锚
/// （`recursive_tower::WindowScanCursor` 文档的 frontier 协议），`l0_units[..resume_from]` 的
/// 扫描路径确定且与塔一致；本函数只在其**尾部**多喂一个单元，不改前缀。
///
/// **级别范围（诚实登记，票 #601 Scope）**：本函数只派生 **L1 层**的行进中窗口——`build`
/// 固定为 `center_from_segments`（L1 层判据：完整方向交替 + 核心非空）。L3 所需的「L2 层行进中
/// 窗口」判据是 `center_from_window`（几何路径）且输入需换成 L1 层 units + 本函数的输出作
/// 虚拟单元，属另一次递归复合——**本函数不泛化未验证的级别**（不预留空参数，见 #527 §7 纪律），
/// 由 #602 另立。
///
/// 未产出时返回可审计原因码（[`ActiveWindowOutcome`]），禁以「不知道」结账。
pub fn active_l1_window_frontier(
    l0_units: &[UnitRange],
    l0_frontier: Option<&ActiveSegmentFrontier>,
    l1_resume_from: usize,
    // 票 #757（观测面 only）：批内非末窗的批量丢弃实例出参（见 [`scan_active_window`]）。
    drops: &mut Vec<BatchDroppedWindow>,
) -> ActiveWindowOutcome {
    let Some(frontier) = l0_frontier else {
        return ActiveWindowOutcome::NoLowerFrontier;
    };
    // 虚拟追加单元：口径与 `classifier::segment_to_unit` 逐字相同（lo/hi 按端点价取序，
    // 不假设方向与价序一致）。右端 = 极值结构点（`ActiveSegmentFrontier` 已禁 as_of 冒充）。
    let virtual_unit = UnitRange {
        start_index: frontier.start_index,
        end_index: frontier.extreme_at,
        direction: frontier.direction,
        lo: frontier.start_price.min(frontier.extreme),
        hi: frontier.start_price.max(frontier.extreme),
    };
    let scan = match scan_active_window(
        l0_units,
        virtual_unit,
        l1_resume_from,
        center_from_segments,
        drops,
    ) {
        Ok(scan) => scan,
        Err(outcome) => return outcome,
    };
    ActiveWindowOutcome::Frontier(ActiveWindowFrontier {
        // L1 层的窗口首单元就是一根 L0 段单元，其 `direction` **即**首叶方向
        // （`segment_to_unit` 直传段方向 ⟹ 与 `lower_legs_from(tower[0])` 的
        // `first_leaf_direction` 逐值相同）。虚拟单元命中窗口首位时同理（parser 行进中段
        // 的方向）。故本级无需外部方向表——L2 层不同，见 [`active_l2_window_frontier`]。
        direction: if scan.win_start < l0_units.len() {
            l0_units[scan.win_start].direction
        } else {
            frontier.direction
        },
        start_index: scan.start_index,
        end_index: scan.end_index,
        lo: scan.lo,
        hi: scan.hi,
    })
}

/// 派生**行进中的 L2 窗口单元**（票 #602 的本体；L3 活窗的 C 来源）。
///
/// 与 [`active_l1_window_frontier`] 的关系：同一内核（[`scan_active_window`]）、同一
/// frontier 协议、同一禁外推判据；**三处**级别相关的差异全部显式化，不靠参数默认值掩盖：
///
/// 1. **build 换几何路径**：本级用 `center::center_from_window`（上级递归层判据：全三段核心
///    非空，**无方向交替**——上级单元是中枢外缘区间，没有 §6.1 意义的方向维度，见
///    `center.rs` 的诚实有效域登记）。给 L2 层套 `center_from_segments` 会在几何判据层用
///    完整判据，产出塔上不可能存在的窗口（#601 已把这条写进拒绝理由）。
/// 2. **输入换一层**：`l1_units` = `TowerCache::level_scan_units(2)`（= `tower[1]` 的投影，
///    L2 层扫描的真输入）；虚拟单元 = [`active_l1_window_frontier`] 的产物。
/// 3. **方向另有来源**：`l1_units[i].direction` 是**投影 ownership 方向**
///    （`recursive_tower::project_to_units` 的 Q7 口径：中枢 i 落 Trend(d) 块 ⟹ d，否则
///    endpoint 比较降级），**不是** `lower_legs_from` 的 `first_leaf_direction`。而 L3 活窗
///    的 confirmed 侧腿取自 `lower_legs_from(tower[2])`（首叶口径），活动腿必须与之同口径
///    ——否则 `provide_active_pan_live_windows` 里 A/C 结构定位是拿两套方向语义对比。故
///    首叶方向由调用方按 `l1_leg_dirs`（= `lower_legs_from(tower[1])` 的 direction 列，
///    与 `l1_units` 逐位对应）供给。L1 层不需要这一项是因为那一级两套方向恰好同值（见上）。
///
/// **虚拟单元的 `direction` 字段照实登记**：填 `l1_frontier.direction`（首叶口径），而
/// `l1_units` 里同位置的元素若已 confirmed 会带 ownership 方向——两者可以不同值。这不构成
/// 分歧，因为本级 build（`center_from_window`）**不读 direction**，
/// `detect_centers_windowed_resume` 的延伸判据也只读 `lo/hi`。此处不做「方向无关紧要」的
/// 静默假设，而是把「本级扫描不消费方向」写成显式登记（090：声明 = 能力）。
///
/// 未产出时返回可审计原因码（[`ActiveWindowOutcome`]），禁以「不知道」结账。
/// `l1_frontier == None`（L1 层活动窗口本身未派生成功）⟹ [`ActiveWindowOutcome::NoLowerFrontier`]
/// ——**其具体成因由同一次重算里 L2 级的诊断行承载**（两级共用同一 bar 的诊断面，
/// `p123_fast_replay` 逐级落行），本码不复述以免同一事实两处命名产生漂移。
pub fn active_l2_window_frontier(
    l1_units: &[UnitRange],
    l1_leg_dirs: &[Direction],
    l1_frontier: Option<&ActiveWindowFrontier>,
    l2_resume_from: usize,
    // 票 #757（观测面 only）：批内非末窗的批量丢弃实例出参（见 [`scan_active_window`]）。
    drops: &mut Vec<BatchDroppedWindow>,
) -> ActiveWindowOutcome {
    let Some(frontier) = l1_frontier else {
        return ActiveWindowOutcome::NoLowerFrontier;
    };
    if l1_leg_dirs.len() != l1_units.len() {
        return ActiveWindowOutcome::LowerLegDirsOutOfSync;
    }
    let virtual_unit = UnitRange {
        start_index: frontier.start_index,
        end_index: frontier.end_index,
        direction: frontier.direction,
        lo: frontier.lo,
        hi: frontier.hi,
    };
    let scan = match scan_active_window(
        l1_units,
        virtual_unit,
        l2_resume_from,
        center_from_window,
        drops,
    ) {
        Ok(scan) => scan,
        Err(outcome) => return outcome,
    };
    ActiveWindowOutcome::Frontier(ActiveWindowFrontier {
        direction: if scan.win_start < l1_leg_dirs.len() {
            l1_leg_dirs[scan.win_start]
        } else {
            frontier.direction
        },
        start_index: scan.start_index,
        end_index: scan.end_index,
        lo: scan.lo,
        hi: scan.hi,
    })
}
