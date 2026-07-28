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
//! - **feed 契约**：只在回放引擎重估 trigger 投喂，时钟精度 = trigger 粒度；每个 trigger
//!   先喂此刻可见的全部活窗观察，再喂完成信号。同一身份 c 窗左端不动、右端按 provider
//!   产出延展；两个 trigger 之间首次可证只能在下一 trigger 被记录（允许晚记，不能早记），
//!   禁止回填历史。`observed_at` 与 `first_provable_at` 均在首次实际 `advance` 写入，故跳过
//!   非 trigger prefix 不可能构造 `first_provable_at < observed_at`。若窗在两个 trigger
//!   间开完又完成，则在同一 trigger 照实形成 Observed→StructureCompleted→终局，寿命为 0；
//!   零寿命是闪现子集，不得丢弃或拖到下一 trigger。完成后停止活窗延展。
//! - **出切片项**（卡 §9 原样维持）：WireV1 全量 EventKey/StateKey/修订链、
//!   谱系两钟 opened/closed、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition
//!   诊断钟、DeferOrphan 重判、Lean 侧 ActiveTail↔OpenTailSystem 桥、白名单桥与两元锚
//!   （`NestCandidateEventExt.extreme_price/group_anchor`，#110/#206 线已入库）并轨。
//! - **v3 硬禁令合规**：全部判据 = 确定性结构/力度谓词（无概率/统计推断、无回测验证、
//!   无 EMH）；测试全部为确定性合成序列（T5/T11/T14 用真实 provider 夹具）。

use super::super::types::{Center, Direction, MoveKind, Segment, Side};
use super::divergence::{
    same_color_area, same_dir_hist_peak, segment_dif_peak, segments_diverge_or,
};
use super::level_view::{LowerLeg, NestCandidateEvent, NestDivergenceKind};
use super::recursive_tower::map_src_to_close_idx;
use super::signal::{
    locate_pan_div_structure, locate_pan_div_structure_front_anchor,
    nearest_confirmed_center_idx, pan_div_structure_extreme,
};
use std::collections::BTreeMap;

// ═══════════════════════════════════════════════════════════════════════════
// 身份键与桥（卡 §2.2/§6.3）
// ═══════════════════════════════════════════════════════════════════════════

/// 生命周期身份键（E2E §6.1:248：状态、钟与行进中的区间不进入事件身份键）。
///
/// 六元组 = `(level, side, kind, seg_a, seg_c_full, b_center_start)`。trend 域
/// `seg_c_full` 取事件产出坐标 `interval_b`（工程桥，模块头 090 登记 1）；pan 活窗域
/// 取 `(c_start_live, live_end)`（卡 §3）。`b_center_start` = B 中枢身份快照
/// （level_view.rs:495-503 prefix 首次观察快照纪律，延伸不改写 start_index）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleKey {
    pub level: u32,
    pub side: Side,
    pub kind: NestDivergenceKind,
    pub seg_a: (usize, usize),
    pub seg_c_full: (usize, usize),
    pub b_center_start: usize,
}

/// `Side` 无 Ord derive（types.rs 口径不动）——排序/判等经判别值投影，不改上游类型。
fn side_tag(side: Side) -> u8 {
    match side {
        Side::Long => 0,
        Side::Short => 1,
    }
}

impl LifecycleKey {
    fn sort_tuple(&self) -> (u32, u8, NestDivergenceKind, (usize, usize), (usize, usize), usize) {
        (
            self.level,
            side_tag(self.side),
            self.kind,
            self.seg_a,
            self.seg_c_full,
            self.b_center_start,
        )
    }
}

impl PartialOrd for LifecycleKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LifecycleKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_tuple().cmp(&other.sort_tuple())
    }
}

/// 白名单工程桥（卡 §6.3 原样）：除 seg_c 右端外全等判同身份。
///
/// 吸收三形态——trend 确认收束（`[c_start, c_end] → [c_start, t*]`）、T5-OR 终假回扩、
/// pan 活窗延展（右端随 as_of 前进，设计内行为）。seg_a 改变 / seg_c 左端改变 / 其他任何
/// 分量改变 ⟹ 越界不判同身份（走身份消失路径，T8 负面对照锁定）。
///
/// 口径登记（spec #232 ID-5 字面张力）：ID-5「新 interval_b ⊆ 旧」仅覆盖收束一形态；
/// spec Solution 三形态（收束/回扩/活窗延展）与 T1（回扩）/T3（延展）锚定要求右端双向
/// 可动——本函数取三形态侧（右端不限方向），字面张力归编排者澄清（实装说明 20260724
/// 对照节登记，本票不替裁）。
fn bridge_identity(a: &LifecycleKey, b: &LifecycleKey) -> bool {
    a.level == b.level
        && a.side == b.side
        && a.kind == b.kind
        && a.seg_a == b.seg_a
        && a.seg_c_full.0 == b.seg_c_full.0
        && a.b_center_start == b.b_center_start
}

// ═══════════════════════════════════════════════════════════════════════════
// 三态 / 原因码 / 力度三值化（卡 §2.3/§4 + #78 修复 1）
// ═══════════════════════════════════════════════════════════════════════════

/// 三态（E2E-D5 最小子集；`Unresolved` 出切片——模块头 090 登记 2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestEventState {
    /// 活假设（061:26「都可以先假设是进入背驰段」——Provisional 是默认态，非例外态）。
    Provisional,
    /// first_provable 已写 ∧ c 结构完成 ∧ 完成时复核仍弱（024:24）。
    Confirmed,
    /// 力度反超（061:26，可由 Provisional 直接到达，无需结构先完成）、从未构成
    /// （061:28）或身份消失（E2E §1:81）。
    /// 终态留档，禁删除模拟失效。
    Invalidated,
}

impl NestEventState {
    /// 终态（Confirmed/Invalidated）⟹ 吸收：同 key 任何后续观察零输出（E2E §1:83 禁复活）。
    pub fn is_terminal(self) -> bool {
        matches!(self, NestEventState::Confirmed | NestEventState::Invalidated)
    }
}

/// 失效原因码（入 revision 载荷，诊断可查账；裁定 #64 §2(b)）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidatedReason {
    /// 被反超：**曾构成、后被否证**（061:26「一旦力度大于前者，那么就可以断定背驰段不
    /// 成立」）——要求曾写入 first_provable，是「先成立、再被推翻」。
    ForceOvertake,
    /// 身份消失（上一 prefix 有、本 prefix 不再产出该 key；E2E §1:81）。
    IdentityVanished,
    /// 从未构成：**根本未构成**（061:28「因为背驰如果没有创新高，是不存在的」）——结构
    /// 完成时该活假设从未写入 first_provable，与 ForceOvertake 的「曾构成」严格互补
    /// （两码的 first_provable_at 一有一无，assert_invariants 逐条钉死）。
    NeverConstituted,
}

/// 力度不可验原因码（#78 修复 1：「不可验」≠「不再弱」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailReason {
    /// 缺 hist/dif 力度序列。
    MissingForceSeries,
    /// `map_src_to_close_idx` 源坐标 → close 下标映射失败。
    CoordinateMapFailed,
}

/// 力度核查三值化（#78 修复 1 全量）：**仅 `Verified(false)` 判 Invalidated**；
/// `Unavailable` 只记审计注记（不判 Invalidated、不写 first_provable）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceCheck {
    /// 力度谓词在当 prefix 窗口上现算结果（true = 仍弱 / false = 不再弱）。
    Verified(bool),
    /// 力度不可验（原因码入注记）。
    Unavailable(UnavailReason),
}

/// 力度证据载荷（裁定 #64 §4「反超证据（力度对比量）入 revision 载荷」；票 #425 起
/// `NeverConstituted` 同样现算入载荷——两条力度类否证均可查账）。
///
/// 逐通道现算复用 `same_color_area`/`segment_dif_peak`/`same_dir_hist_peak`
/// （与 `segments_diverge_or` 同一组原语，禁第二查法）；仅审计载荷，不进真值路径。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceEvidence {
    pub area_a: f64,
    pub area_c: f64,
    pub dif_peak_a: f64,
    pub dif_peak_c: f64,
    pub hist_peak_a: f64,
    pub hist_peak_c: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// 修订 / entry（E2E §1:83 修订协议最小形态）
// ═══════════════════════════════════════════════════════════════════════════

/// 修订种类（业务载荷投影变化才产一条，E2E §1:83）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifecycleRevisionKind {
    /// ∅ → Provisional：首次观察到比较对。
    Observed,
    /// 白名单工程桥身份迁移（收束/回扩/活窗延展）：钟不动、链留痕、无 Invalidated。
    Supersedes { from: LifecycleKey },
    /// D1–D4 首次同真（仅 `Verified(true)` 分支写入——#78 核验 T14 锚定）。
    FirstProvable,
    /// c 结构完成信号首次到达。仅力度可验且此前未由第二终局路径吸收时产生；同一 prefix
    /// 随后结算为 Confirmed 或 Invalidated(NeverConstituted)。曾可证身份若先在活窗观察
    /// 被力度反超，合法链为 Provisional→Invalidated(ForceOvertake)，不伪造本修订。
    /// Unavailable prefix 只写 ForceUnavailable + `CompletionForceUnavailableAudit`，
    /// 数据补齐并重发信号后再留痕。
    StructureCompleted,
    /// 力度不可验审计注记（不判 Invalidated、不写 first_provable）。
    ForceUnavailable { reason: UnavailReason },
    /// Provisional → Confirmed（完成时复核仍弱，024:24）。
    Confirmed,
    /// Provisional → Invalidated；原因逐一为 ForceOvertake、NeverConstituted 或
    /// IdentityVanished（逐码入载荷，可区分——T6 / T17）。
    Invalidated { reason: InvalidatedReason },
}

/// 一条修订（entry 留档 + advance 增量返回双通道；禁删除模拟失效，E2E §1:81）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifecycleRevision {
    /// 修订发生时 entry 的身份键（Supersedes 后为新键）。
    pub key: LifecycleKey,
    pub kind: LifecycleRevisionKind,
    /// 产生该修订的 prefix（一切钟 ≤ 此 as_of，E2E-S5 判据）。
    pub as_of: usize,
    /// 力度证据（ForceOvertake / NeverConstituted 且力度序列齐备时现算；
    /// IdentityVanished 恒 None）。
    pub evidence: Option<ForceEvidence>,
}

/// 生命周期 entry：身份 + 三态 + 五钟 + 留档（卡 §2.3 + #78 修复 1/2 字段）。
///
/// 五钟 `observed_at / first_provable_at / structure_end_at / confirmed_at /
/// invalidated_at`（E2E §4.1:144）：首次写入不后移、终态钟只写一次、无删除 API。
#[derive(Debug, Clone, PartialEq)]
pub struct NestLifecycleEntry {
    pub key: LifecycleKey,
    pub state: NestEventState,
    /// 修订计数（== `revisions.len()`；业务载荷投影变化才 +1）。
    pub revision: u32,
    /// 首见 prefix；建仓写一次、无任何改写点。
    pub observed_at: usize,
    /// D1–D4 首次同真 prefix；仅 `Verified(true)` 写入、只写一次。
    pub first_provable_at: Option<usize>,
    /// c 结构完成 prefix（首次到达写一次）。
    pub structure_end_at: Option<usize>,
    /// 首次进 Confirmed。
    pub confirmed_at: Option<usize>,
    /// 首次进 Invalidated。
    pub invalidated_at: Option<usize>,
    /// 失效原因码（ForceOvertake / NeverConstituted / IdentityVanished 三者可区分）。
    pub invalidated_reason: Option<InvalidatedReason>,
    /// 力度证据终态留档（ForceOvertake 与 NeverConstituted 现算，材料缺则诚实 None；
    /// IdentityVanished 恒 None——无力度语义）。
    pub force_evidence: Option<ForceEvidence>,
    /// 最近一次力度不可验注记 prefix（同 as_of 幂等去重基准；只作去重，不阻塞恢复推进）。
    pub force_unavailable_at: Option<usize>,
    /// 白名单桥迁移链留痕（迁移不改写任何钟）。
    pub superseded_from: Option<LifecycleKey>,
    /// 该身份已见最大 as_of（#78 修复 2 倒退守卫基准）。
    pub last_as_of: usize,
    /// 修订留档（append-only）。
    pub revisions: Vec<LifecycleRevision>,
}

impl NestLifecycleEntry {
    /// 追加一条修订（计数与留档同步推进；E2E §1:83 业务载荷投影变化才调用）。
    /// revision.key 取调用时 entry 的当前键（Supersedes 迁移先改键再推送 ⟹ 新键）。
    fn push_revision(
        &mut self,
        kind: LifecycleRevisionKind,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) -> LifecycleRevision {
        self.revision += 1;
        let revision = LifecycleRevision {
            key: self.key,
            kind,
            as_of,
            evidence,
        };
        self.revisions.push(revision);
        revision
    }

    /// 转入终态 `Invalidated` 并留档（三个原因码的**唯一**写入点——原因码与力度证据同写，
    /// 结构上兑现模块头 090 登记 4「原因码与力度证据入载荷」；无删除路径，禁删除模拟失效）。
    ///
    /// `evidence`：ForceOvertake / NeverConstituted 传本 prefix 现算证据（材料缺则诚实
    /// None）；IdentityVanished 传 None（无力度语义）。
    fn invalidate(
        &mut self,
        reason: InvalidatedReason,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) -> LifecycleRevision {
        self.state = NestEventState::Invalidated;
        self.invalidated_at = Some(as_of);
        self.invalidated_reason = Some(reason);
        self.force_evidence = evidence;
        self.push_revision(LifecycleRevisionKind::Invalidated { reason }, as_of, evidence)
    }
}

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
    fn key(&self) -> LifecycleKey {
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

    fn structure_completed(&self) -> bool {
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
    fn force(&self, material: &ForceMaterial) -> ForceCheck {
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
    fn force_evidence(&self, material: &ForceMaterial) -> Option<ForceEvidence> {
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

/// 倒退 prefix 显式拒绝注记（#78 修复 2：禁静默吸收）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrogradeRejection {
    pub key: LifecycleKey,
    /// 该身份已见最大 as_of。
    pub last_as_of: usize,
    /// 被拒绝的倒退 as_of。
    pub rejected_as_of: usize,
}

/// 完成信号已到但力度不可验的独立审计事实（#428）。
///
/// 该事实只进入审计面，不改变 #78 的结算语义：对应 entry 仍为 Provisional，
/// 待力度材料补齐并重发完成信号后再按原协议结算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionForceUnavailableAudit {
    pub key: LifecycleKey,
    pub as_of: usize,
    pub reason: UnavailReason,
}

/// 数据源首次给出结构完成信号的独立事实；按桥身份唯一。
///
/// 它与终态分开留档：身份可先由 Provisional→ForceOvertake 终局，之后到达的首完成仍须
/// 进入真实分母，但终态吸收禁止据此回填 `StructureCompleted`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionSignal {
    pub key: LifecycleKey,
    pub as_of: usize,
}

/// 一组寿命读数（单位为 `as_of` 的 bar-index 差，不冒充 trigger 次数）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LifetimeDistribution {
    pub count: usize,
    pub min: Option<usize>,
    pub median: Option<f64>,
    pub max: Option<usize>,
}

impl LifetimeDistribution {
    fn from_values(mut values: Vec<usize>) -> Self {
        values.sort_unstable();
        let count = values.len();
        if count == 0 {
            return Self::default();
        }
        let median = if count % 2 == 0 {
            (values[count / 2 - 1] as f64 + values[count / 2] as f64) / 2.0
        } else {
            values[count / 2] as f64
        };
        Self {
            count,
            min: values.first().copied(),
            median: Some(median),
            max: values.last().copied(),
        }
    }
}

/// 生命周期终局与寿命的只读统计面；闪现（寿命 0）与非闪现分开。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LifecycleSettlementStats {
    pub entry_count: usize,
    pub first_provable_count: usize,
    pub provisional_count: usize,
    pub confirmed_count: usize,
    pub force_overtake_count: usize,
    pub never_constituted_count: usize,
    pub identity_vanished_count: usize,
    pub flash_terminal_count: usize,
    pub nonflash_lifetime: LifetimeDistribution,
    pub force_overtake_lifetime: LifetimeDistribution,
}

// ═══════════════════════════════════════════════════════════════════════════
// NestLifecycleBook（sidecar 注册表）
// ═══════════════════════════════════════════════════════════════════════════

/// sidecar 注册表（范式复用：strategy/persistent.rs Pi 注册表与 `PersistentElement`、
/// recursive_tower.rs `CpScanOwnership` 生命周期挂 book 内对象 + 显式推进函数、
/// p92 YieldBook first-write-wins `entry().or_insert()`）。
///
/// 事件流零改动（T5 bit-exact 护栏）；状态只活在本 entry。**消费边界**（裁定 #64 §2(a)）：
/// 本 book 是观察记录——不进 `d_parent_interval_snapshot/terminal` 输入、不改
/// `divergence_confirmed` 布尔口径、N^δ 装配仍只消费已闭合完整 c_p。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestLifecycleBook {
    entries: BTreeMap<LifecycleKey, NestLifecycleEntry>,
    /// 倒退拒绝审计注记（#78 修复 2）。同一倒退喂入重复执行重复记录（重复违规事实本身；
    /// 与 ForceUnavailable 同 as_of 幂等不对称——原 #78 评审 Low 登记同判，不阻塞）。
    retrograde_rejections: Vec<RetrogradeRejection>,
    /// 数据源真实首完成信号（按桥身份唯一；与终态独立，禁用终态冒充「已经完成」）。
    completion_signals: Vec<CompletionSignal>,
    /// 完成信号与力度不可验同时发生的独立审计面（#428；按桥身份唯一）。
    completion_force_unavailable_audits: Vec<CompletionForceUnavailableAudit>,
}

impl NestLifecycleBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// entry 读面（TDD 接缝外部行为断言面）。
    pub fn get(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        self.entries.get(key)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&LifecycleKey, &NestLifecycleEntry)> {
        self.entries.iter()
    }

    /// 倒退拒绝注记门户（#78 修复 2 审计面）。
    pub fn retrograde_rejections(&self) -> &[RetrogradeRejection] {
        &self.retrograde_rejections
    }

    /// 数据源真实首完成信号门户（按桥身份唯一；独立于状态终局）。
    pub fn completion_signals(&self) -> &[CompletionSignal] {
        &self.completion_signals
    }

    /// 完成信号已到但力度不可验的审计门户（只读，不参与状态判定）。
    pub fn completion_force_unavailable_audits(&self) -> &[CompletionForceUnavailableAudit] {
        &self.completion_force_unavailable_audits
    }

    /// 汇总终局分布与寿命；闪现只计终局钟与 `observed_at` 同刻的身份。
    pub fn settlement_stats(&self) -> LifecycleSettlementStats {
        let mut stats = LifecycleSettlementStats {
            entry_count: self.entries.len(),
            ..LifecycleSettlementStats::default()
        };
        let mut nonflash_lifetimes = Vec::new();
        let mut force_overtake_lifetimes = Vec::new();
        for entry in self.entries.values() {
            stats.first_provable_count += usize::from(entry.first_provable_at.is_some());
            let (terminal_at, force_overtake) = match entry.state {
                NestEventState::Provisional => {
                    stats.provisional_count += 1;
                    (None, false)
                }
                NestEventState::Confirmed => {
                    stats.confirmed_count += 1;
                    (entry.confirmed_at, false)
                }
                NestEventState::Invalidated => match entry
                    .invalidated_reason
                    .expect("Invalidated 必有原因码")
                {
                    InvalidatedReason::ForceOvertake => {
                        stats.force_overtake_count += 1;
                        (entry.invalidated_at, true)
                    }
                    InvalidatedReason::NeverConstituted => {
                        stats.never_constituted_count += 1;
                        (entry.invalidated_at, false)
                    }
                    InvalidatedReason::IdentityVanished => {
                        stats.identity_vanished_count += 1;
                        (entry.invalidated_at, false)
                    }
                },
            };
            let Some(terminal_at) = terminal_at else {
                continue;
            };
            let lifetime = terminal_at
                .checked_sub(entry.observed_at)
                .expect("终局钟不得早于 observed_at");
            if lifetime == 0 {
                stats.flash_terminal_count += 1;
            } else {
                nonflash_lifetimes.push(lifetime);
            }
            if force_overtake {
                force_overtake_lifetimes.push(lifetime);
            }
        }
        stats.nonflash_lifetime = LifetimeDistribution::from_values(nonflash_lifetimes);
        stats.force_overtake_lifetime =
            LifetimeDistribution::from_values(force_overtake_lifetimes);
        stats
    }

    /// 消费侧 Closed-only（裁定 #64 §2(a)「消费不放开」）：只放 Confirmed。
    /// Provisional/Invalidated 永不经本门户离开 book——Invalidated 可查账
    /// （entries/revisions 全程留档），不开放消费（模块头 090 登记 4）。
    pub fn consumable_closed(&self) -> Vec<&NestLifecycleEntry> {
        self.entries
            .values()
            .filter(|entry| entry.state == NestEventState::Confirmed)
            .collect()
    }

    /// 谱系构建节点读面（裁定 #64 §2(a)「构建放开」）。
    ///
    /// ★红线：本读面仅供谱系**构建**侧；**禁止**出现在任何
    /// `d_parent_interval_snapshot`/装配候选/证书真值路径（nest.rs:802-810/:987-989
    /// 装配输入不变；白名单工程桥不得进证书真值路径——裁定 §2(c) 裁定义务）。
    pub fn lineage_nodes(&self) -> Vec<&NestLifecycleEntry> {
        self.entries.values().collect()
    }

    /// 推进一 prefix（卡 §2.3 转移表 + #78 修复全量）。返回本 prefix 新产出修订
    /// （终态吸收/幂等/拒绝 = 空）。步骤：
    /// 1. 建仓 / 白名单桥迁移（Supersedes）/ 终态吸收（含桥匹配到终态）；桥匹配分支自带
    ///    倒退守卫（被拒 key 尚未建仓，T18 锁定）；
    /// 2. 倒退守卫（per-identity last_as_of，显式拒绝 + 注记；直接匹配分支，T15 锁定）；
    /// 3. 终态吸收（禁复活）；
    /// 4. 力度求值（事件通道恒 Verified；活窗三值化）；
    /// 5. first_provable 首次写入（仅 Verified(true)）；
    /// 6. 反超判负（仅 Verified(false) ⟹ Invalidated(ForceOvertake)）/
    ///    Unavailable 审计注记（不判 Invalidated、不进确认）；
    /// 7. 完成时复核（本 prefix 现算 force，禁「曾经弱过」冒充，E2E §4.1:151）：曾可证 ∧
    ///    仍弱 ⟹ Confirmed；从未可证 ⟹ Invalidated(NeverConstituted)（061:28，ADR-0003）；
    /// 8. 身份消失扫描（倒退 prefix 不制造 IdentityVanished）。
    pub fn advance(
        &mut self,
        observations: &[LifecycleObservation],
        as_of: usize,
        material: &ForceMaterial,
    ) -> Vec<LifecycleRevision> {
        let mut delta = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for obs in observations {
            let key = obs.key();
            seen.insert(key);
            // 第 1 步：建仓 / 白名单桥迁移 / 终态吸收（含桥匹配到终态）。
            if !self.entries.contains_key(&key) {
                match self.bridge_match(&key) {
                    Some(old_key) => {
                        let old = &self.entries[&old_key];
                        // 同一身份的倒退喂入：显式拒绝（不迁移、不建仓、零 revision）。
                        if as_of < old.last_as_of {
                            let last_as_of = old.last_as_of;
                            self.retrograde_rejections.push(RetrogradeRejection {
                                key,
                                last_as_of,
                                rejected_as_of: as_of,
                            });
                            continue;
                        }
                        // 终态吸收含桥匹配：终态不迁移、不建仓、零输出（禁复活，E2E §1:83）。
                        if old.state.is_terminal() {
                            continue;
                        }
                        // Supersedes 迁移（仅 Provisional 可达——终态已在上一步吸收）：
                        // 全 book 唯一 remove 点；钟不动、链留痕。
                        let mut entry = self.entries.remove(&old_key).expect("桥匹配键存在");
                        entry.superseded_from = Some(old_key);
                        entry.key = key;
                        let revision = entry.push_revision(
                            LifecycleRevisionKind::Supersedes { from: old_key },
                            as_of,
                            None,
                        );
                        self.entries.insert(key, entry);
                        delta.push(revision);
                    }
                    None => {
                        // 新身份建仓（observed_at 建仓写一次、无任何改写点；倒退 as_of 的
                        // 新身份无基线可违照建——#78 评审信息项，observed_at ≤ last_as_of 不受损）。
                        let revision = LifecycleRevision {
                            key,
                            kind: LifecycleRevisionKind::Observed,
                            as_of,
                            evidence: None,
                        };
                        let entry = NestLifecycleEntry {
                            key,
                            state: NestEventState::Provisional,
                            revision: 1,
                            observed_at: as_of,
                            first_provable_at: None,
                            structure_end_at: None,
                            confirmed_at: None,
                            invalidated_at: None,
                            invalidated_reason: None,
                            force_evidence: None,
                            force_unavailable_at: None,
                            superseded_from: None,
                            last_as_of: as_of,
                            revisions: vec![revision],
                        };
                        self.entries.insert(key, entry);
                        delta.push(revision);
                    }
                }
            }
            // 第 2 步：倒退守卫（per-identity last_as_of；本 prefix 新建 entry 恒通过——
            // last_as_of = as_of，守卫不触发）。
            let last_as_of = self.entries[&key].last_as_of;
            if as_of < last_as_of {
                self.retrograde_rejections.push(RetrogradeRejection {
                    key,
                    last_as_of,
                    rejected_as_of: as_of,
                });
                continue; // 零 revision、entry 零改动（显式拒绝，非静默吸收——#78 修复 2）
            }
            let entry = self.entries.get_mut(&key).expect("entry 已建仓");
            entry.last_as_of = as_of;
            // 第 3 步：终态吸收（禁复活）。
            if entry.state.is_terminal() {
                continue;
            }
            // 第 4 步：力度求值（事件通道恒 Verified；活窗三值化）。
            let force = obs.force(material);
            // 第 5 步：first_provable 首次写入（仅 Verified(true)——#78 核验 T14 锚定；
            // Unavailable 不写 first_provable）。
            if entry.first_provable_at.is_none() && force == ForceCheck::Verified(true) {
                entry.first_provable_at = Some(as_of);
                let revision = entry.push_revision(LifecycleRevisionKind::FirstProvable, as_of, None);
                delta.push(revision);
            }
            // 第 6 步：反超判负 / Unavailable 审计注记。反超只要求曾可证；结构完成不是
            // 前置条件，因此这里合法产生 Provisional→ForceOvertake 第二终局路径（Q3）。
            match force {
                ForceCheck::Verified(false) => {
                    if entry.first_provable_at.is_some() {
                        // 反超（061:26）：曾可证 ∧ 当前不再弱 ⟹ Invalidated(ForceOvertake)。
                        let evidence = obs.force_evidence(material);
                        let revision = entry.invalidate(
                            InvalidatedReason::ForceOvertake,
                            as_of,
                            evidence,
                        );
                        delta.push(revision);
                        continue;
                    }
                }
                ForceCheck::Unavailable(reason) => {
                    // 「不可验」≠「不再弱」：只记审计注记（按桥身份唯一），不判
                    // Invalidated、不写 first_provable、不进完成确认（entry 保持原态）。
                    // audit 按完成信号桥身份唯一；provider 跨 trigger 重发不重复抬高发生率分子。
                    if obs.structure_completed()
                        && !self
                            .completion_force_unavailable_audits
                            .iter()
                            .any(|audit| audit.key == key || bridge_identity(&audit.key, &key))
                    {
                        self.completion_force_unavailable_audits
                            .push(CompletionForceUnavailableAudit { key, as_of, reason });
                    }
                    if entry.force_unavailable_at != Some(as_of) {
                        entry.force_unavailable_at = Some(as_of);
                        let revision = entry.push_revision(
                            LifecycleRevisionKind::ForceUnavailable { reason },
                            as_of,
                            None,
                        );
                        delta.push(revision);
                    }
                    continue;
                }
                ForceCheck::Verified(true) => {}
            }
            // 第 7 步：完成时复核（024:24——同一谓词在完成窗上重算为真才结算；禁
            // 「曾经弱过」冒充，E2E §4.1:151；复核用本 prefix 现算 force，不沿用旧值）。
            if obs.structure_completed() {
                if entry.structure_end_at.is_none() {
                    entry.structure_end_at = Some(as_of);
                    let revision = entry.push_revision(
                        LifecycleRevisionKind::StructureCompleted,
                        as_of,
                        None,
                    );
                    delta.push(revision);
                }
                if entry.first_provable_at.is_some() && force == ForceCheck::Verified(true) {
                    entry.state = NestEventState::Confirmed;
                    entry.confirmed_at = Some(as_of);
                    let revision =
                        entry.push_revision(LifecycleRevisionKind::Confirmed, as_of, None);
                    delta.push(revision);
                } else if entry.first_provable_at.is_none() {
                    // 从未构成（061:28「因为背驰如果没有创新高，是不存在的」，ADR-0003）：
                    // 结构完成时从未写入 first_provable ⟹ 该假设根本未构成，转终态挂
                    // NeverConstituted，不再以活假设身份挂账。**与 ForceOvertake 严格互补**
                    // ——后者要求「曾可证」（061:26 曾构成后被否证），此处恒无（T17 分辨）。
                    // 力度证据同 ForceOvertake 现算入载荷（090 登记 4 可查账）。
                    // 到达本臂时 force 必为 Verified(false)：Verified(true) 已在第 5 步写入
                    // first_provable（走上一臂），Unavailable 已在第 6 步 continue。
                    // **残留登记**：完成 prefix 上力度不可验时
                    // 第 6 步先 continue ⟹ 本臂不到达、该身份滞留活假设（#78「不可验 ≠ 不再
                    // 弱」优先——宁可推迟结算，不从缺失数据造否证；T19 锁定）。数据补齐后的
                    // 完成信号照常结算；始终不补则永久滞留。
                    let evidence = obs.force_evidence(material);
                    let revision = entry.invalidate(
                        InvalidatedReason::NeverConstituted,
                        as_of,
                        evidence,
                    );
                    delta.push(revision);
                }
                // 完成观察已反超者已被第 6 步接住（Invalidated(ForceOvertake) 而非
                // Confirmed，confirmed_at 保持 None，T7(b) 锁定）；若是此前活窗先反超，
                // 则合法绕过 StructureCompleted——两条否证路径不互相冒充。
            }
        }
        // 第 8 步：身份消失扫描（上一 prefix 有、本 prefix 不再产出该 key ⟹ Invalidated；
        // 倒退 prefix 不制造 IdentityVanished——last_as_of > as_of 的身份跳过，#78 修复 2）。
        let vanished: Vec<LifecycleKey> = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                entry.state == NestEventState::Provisional
                    && entry.last_as_of <= as_of
                    && !seen.contains(key)
            })
            .map(|(key, _)| *key)
            .collect();
        for key in vanished {
            let entry = self.entries.get_mut(&key).expect("扫描键存在");
            // 证据传 None：IdentityVanished 无力度语义（invariant 逐条钉死）。
            let revision = entry.invalidate(InvalidatedReason::IdentityVanished, as_of, None);
            delta.push(revision);
        }
        // 库内 debug 构建自动核验；release 不作“自动核验”声明。生产 p123 接线在每个
        // trigger 喂入后显式调用本公开入口。
        #[cfg(debug_assertions)]
        self.assert_invariants();
        delta
    }

    /// 钟不变量显式化（review judgement call 4：公开可调用，不只靠 debug_assert）。
    ///
    /// 全列：revision 计数 == 留档长度；observed_at ≤ last_as_of；钟序
    /// observed ≤ first_provable ≤ structure_end ≤ confirmed；一切钟 ≤ last_as_of；
    /// 反超 invalidated ≥ first_provable（允许 structure_end 为空；身份消失路径独立）；终态互洽
    /// （state ⟺ 终态钟、终态互斥）；IdentityVanished 恒无力度证据；迁移链两端满足桥身份；
    /// **被反超与从未构成在 first_provable 上严格互补**（前者恒有、后者恒无），从未构成的
    /// invalidated_at == structure_end_at（票 #425）；首完成信号按桥身份唯一且不早于观察钟。
    pub fn assert_invariants(&self) {
        for entry in self.entries.values() {
            let key = entry.key;
            assert_eq!(
                entry.revision as usize,
                entry.revisions.len(),
                "revision 计数 == 留档长度：{key:?}"
            );
            assert!(
                entry.observed_at <= entry.last_as_of,
                "observed_at ≤ last_as_of：{key:?}"
            );
            if let Some(first) = entry.first_provable_at {
                assert!(
                    entry.observed_at <= first && first <= entry.last_as_of,
                    "observed ≤ first_provable ≤ last_as_of：{key:?}"
                );
            }
            if let Some(structure_end) = entry.structure_end_at {
                assert!(
                    entry.observed_at <= structure_end && structure_end <= entry.last_as_of,
                    "observed ≤ structure_end ≤ last_as_of：{key:?}"
                );
                if let Some(first) = entry.first_provable_at {
                    assert!(first <= structure_end, "first_provable ≤ structure_end：{key:?}");
                }
            }
            match entry.state {
                NestEventState::Provisional => {
                    assert!(
                        entry.confirmed_at.is_none() && entry.invalidated_at.is_none(),
                        "Provisional 无终态钟：{key:?}"
                    );
                }
                NestEventState::Confirmed => {
                    let confirmed = entry.confirmed_at.expect("Confirmed 必有 confirmed_at");
                    let first = entry.first_provable_at.expect("Confirmed 必先可证");
                    let structure_end = entry.structure_end_at.expect("Confirmed 必先结构完成");
                    assert!(
                        first <= structure_end
                            && structure_end <= confirmed
                            && confirmed <= entry.last_as_of,
                        "钟序 first ≤ structure_end ≤ confirmed ≤ last_as_of：{key:?}"
                    );
                    assert!(entry.invalidated_at.is_none(), "终态互斥：{key:?}");
                }
                NestEventState::Invalidated => {
                    let invalidated = entry.invalidated_at.expect("Invalidated 必有 invalidated_at");
                    assert!(
                        entry.observed_at <= invalidated,
                        "observed ≤ invalidated：{key:?}"
                    );
                    assert!(entry.confirmed_at.is_none(), "终态互斥：{key:?}");
                    match entry.invalidated_reason.expect("Invalidated 必有原因码") {
                        InvalidatedReason::ForceOvertake => {
                            let first = entry
                                .first_provable_at
                                .expect("反超定义要求曾可证（卡 §4.1）");
                            assert!(first <= invalidated, "反超 invalidated ≥ first_provable：{key:?}");
                            // 反超发生于该身份被投喂的 prefix（last_as_of 已先推进到当 prefix）。
                            assert!(invalidated <= entry.last_as_of, "反超 invalidated ≤ last_as_of：{key:?}");
                        }
                        InvalidatedReason::NeverConstituted => {
                            // 从未构成（061:28）与被反超（061:26）严格互补：前者恒无
                            // first_provable，后者恒有（上一臂）——两码不可混。
                            assert!(
                                entry.first_provable_at.is_none(),
                                "从未构成 ⟹ 首次可证时点恒空（否则应走 ForceOvertake）：{key:?}"
                            );
                            // 结算点 = 结构完成那一刻（advance 第 7 步同 prefix 内结算，
                            // 终态吸收禁后续改写 ⟹ 两钟恒等）。
                            assert_eq!(
                                entry.structure_end_at,
                                Some(invalidated),
                                "从未构成 ⟹ invalidated_at == structure_end_at：{key:?}"
                            );
                            assert!(
                                invalidated <= entry.last_as_of,
                                "从未构成 invalidated ≤ last_as_of：{key:?}"
                            );
                        }
                        InvalidatedReason::IdentityVanished => {
                            // 身份消失路径 invalidated_at 独立于 first_provable_at（卡 §2.3）；
                            // 失效在「本 prefix 不再产出」时结算 ⟹ invalidated ≥ last_as_of。
                            assert!(
                                entry.last_as_of <= invalidated,
                                "身份消失 invalidated ≥ last_as_of：{key:?}"
                            );
                            assert!(
                                entry.force_evidence.is_none(),
                                "IdentityVanished 恒无力度证据：{key:?}"
                            );
                        }
                    }
                }
            }
            if let Some(from) = entry.superseded_from {
                assert!(from != key, "迁移链不自环：{key:?}");
                assert!(bridge_identity(&from, &key), "迁移链两端满足桥身份：{key:?}");
            }
        }
        for (index, signal) in self.completion_signals.iter().enumerate() {
            assert!(
                !self.completion_signals[..index].iter().any(|prior| {
                    prior.key == signal.key || bridge_identity(&prior.key, &signal.key)
                }),
                "首完成信号按桥身份唯一：{:?}",
                signal.key
            );
            if let Some(entry) = self.bridge_entry(&signal.key) {
                assert!(
                    entry.observed_at <= signal.as_of,
                    "完成信号不得早于观察钟：{:?}",
                    signal.key
                );
            }
        }
    }

    /// 该身份（含桥同身份）是否已进终态——喂数出口「完成即停延展」的判据。
    ///
    /// 终态吸收（advance 第 1/3 步）使照喂与不喂的 book 逐位相同（零 revision、
    /// `last_as_of` 不动、身份消失扫描只看 Provisional）⟹ 跳过是纯成本优化，非行为改动
    /// （F5 等价性测试锚定）。
    fn terminal_bridge_hit(&self, key: &LifecycleKey) -> bool {
        let entry = self.bridge_entry(key);
        entry.is_some_and(|entry| entry.state.is_terminal())
    }

    /// 同一完成信号是否已在此前 trigger 留档；终态本身不得冒充完成信号。
    fn completion_signal_seen(&self, key: &LifecycleKey) -> bool {
        self.completion_signals
            .iter()
            .any(|signal| signal.key == *key || bridge_identity(&signal.key, key))
    }

    /// 留档真实首完成；倒退信号不入分母，随后由 `advance` 的既有守卫显式拒绝。
    fn register_completion_signal(&mut self, key: LifecycleKey, as_of: usize) -> bool {
        if self.completion_signal_seen(&key)
            || self
                .bridge_entry(&key)
                .is_some_and(|entry| as_of < entry.last_as_of)
        {
            return false;
        }
        self.completion_signals.push(CompletionSignal { key, as_of });
        true
    }

    fn bridge_entry(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        match self.entries.get(key) {
            Some(entry) => Some(entry),
            None => self
                .bridge_match(key)
                .and_then(|old| self.entries.get(&old)),
        }
    }

    /// 白名单桥匹配：除 seg_c 右端外全等的既有键（同身份不同右端的键在 book 内至多一只，
    /// 迁移即替换 ⟹ 匹配唯一）。
    fn bridge_match(&self, key: &LifecycleKey) -> Option<LifecycleKey> {
        self.entries
            .keys()
            .find(|old| **old != *key && bridge_identity(old, key))
            .copied()
    }
}

/// pan 活窗产出机（卡 §3：中枢 + seg_a 定位沿用 `locate_pan_div_structure`——窄锚优先、
/// A′ 回退与 provider 同序同判（level_view.rs:826-839）；Extreme 预滤沿用
/// `pan_div_structure_extreme`，禁第二查法）。
///
/// 对每只 Consolidation 中枢取**末个**可定位离开段的结构锚，活窗 = `(seg_c.0, as_of)`
/// （c 窗右端 = prefix 边界，含行进中 bar）。本函数只做定位与产窗，不消费力度
/// （力度三值化在 advance 内现算）。#421 已由 `p123_fast_replay` 在生产重估 trigger
/// 调用 `feed_replay_prefix`，本函数是其 `PanLiveWindow` 行进中通道的参照实装
/// （T11/T14 真实夹具锚定）。
/// 可见性登记：保留 `pub`——本函数是交付「消费契约」的喂入参照（两轴评审发现项取
/// 登记分支）；`pub(crate)` 在非 test 构建无调用方会触发 dead_code 警告，违反零新增警告线。
pub fn provide_pan_live_windows(
    level: u32,
    centers: &[Center],
    kinds: &[Option<MoveKind>],
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    as_of: usize,
) -> Vec<PanLiveWindow> {
    // 同一身份（中枢/方向/seg_a/c_start）多只离开段可定位时末段覆盖——每身份活窗唯一；
    // BTreeMap 保序 ⟹ 产出确定性（v3 硬禁令）。
    let mut latest: BTreeMap<(usize, u8, (usize, usize), usize), PanLiveWindow> = BTreeMap::new();
    for segment in segments.iter().filter(|s| s.end_index <= as_of) {
        let Some(center_index) = nearest_confirmed_center_idx(centers, segment.start_index)
        else {
            continue;
        };
        if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
            continue;
        }
        // 窄锚优先、A′ 回退（061:28 中枢前最近同向段）——与 provider pan 分支同序同判。
        let Some(structure) = locate_pan_div_structure(
            &centers[center_index],
            segment,
            segments,
            anchors_self,
        )
        .filter(|structure| pan_div_structure_extreme(structure, segments))
        .or_else(|| {
            locate_pan_div_structure_front_anchor(
                &centers[center_index],
                segment,
                segments,
                anchors_self,
            )
            .filter(|structure| pan_div_structure_extreme(structure, segments))
        }) else {
            continue;
        };
        latest.insert(
            (center_index, side_tag(structure.side), structure.seg_a, structure.seg_c.0),
            PanLiveWindow {
                level,
                side: structure.side,
                seg_a: structure.seg_a,
                // c 窗右端 = prefix 边界（含行进中 bar；卡 §3 设计内行为）。
                seg_c_live: (structure.seg_c.0, as_of.max(structure.seg_c.0)),
                b_center_start: centers[center_index].start_index,
            },
        );
    }
    latest.into_values().collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// 回放喂数出口（票 #426 主接缝；ADR-0003：结构完成 = 通道切换）
// ═══════════════════════════════════════════════════════════════════════════

/// `level_view.rs:436` 私有 `leg_as_segment` 的接线侧复制（规格 Implementation Decisions
/// 「在接线侧复制一份」而非提升可见性；`runner.rs` 诊断臂与 p409 探针已有同型先例）。
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

/// 一个前缀的喂数输入：行进中通道（逐 run 产窗）+ 完成事件通道（数据源候选事件）。
#[derive(Debug, Clone, Copy)]
pub struct ReplayPrefixFeed<'a> {
    /// 本次喂数的 prefix 边界（= 引擎的重估触发点；契约「时钟精度 = 触发点粒度」）。
    pub as_of: usize,
    pub runs: &'a [PanLiveRun<'a>],
    /// 数据源本前缀产出的候选事件；本出口只取 `Consolidation` 域（trend 不在票 #426 范围）。
    pub completion_events: &'a [NestCandidateEvent],
}

/// 喂数出口计数面（诊断/审计；不进真值路径、不参与任何判定）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayFeedStats {
    /// 本前缀行进中通道产出的活窗数（不含完成事件反推的闪现观察）。
    pub live_windows: usize,
    /// 本前缀完成事件通道的盘整事件观察数（可含跨 trigger 重发，不作发生率分母）。
    pub completion_events: usize,
    /// 本前缀真实、唯一的首完成信号数；包括同 trigger 闪现与已提前终局身份（完整分母）。
    pub completion_signals: usize,
    /// 本前缀首次由活窗观察切到完成信号的唯一身份数（与首完成分母同口径）。
    pub channel_switches: usize,
    /// 已进终态的身份被跳过喂入的活窗数（完成即停延展；终态吸收下逐位等价）。
    pub extension_suppressed: usize,
    /// 本前缀新增的时点倒退拒绝注记数。
    pub retrograde_rejected: usize,
    /// 本前缀新增的「完成信号已到但力度不可验」审计事实数（#428）。
    pub completion_force_unavailable: usize,
}

/// 回放循环的账本喂数出口（票 #426 主接缝）。
///
/// 给定某一前缀上数据源的产出，生成该步的观察记录、喂入账本、返回该步的变更集。
/// **侧车**：只读入参、只写 `book`，对数据源事件流零反流（F7 逐字节护栏锚定）。
///
/// 通道分工（ADR-0003）：
/// - **行进中通道** = `provide_pan_live_windows` 逐 run 现产的活窗（比较窗 = 起点到当前），
///   `structure_completed = false`；
/// - **完成事件通道** = 数据源候选事件的盘整域（比较窗 = 定位到的完成段），
///   `structure_completed = true`——**切换本身即结构完成信号，不另造判据**。
///
/// 每个 trigger 严格分两相：先喂全部可见活窗，再喂完成信号。完成信号只对账该身份，
/// 不删除同 trigger 的活窗观察，也不要求身份必须来自更早 trigger；因此同 trigger 首见
/// 可照实形成 Observed→StructureCompleted→终局的零寿命闪现链。若稀疏 trigger 之间开窗
/// 又完成、当前只剩完成事件，则由该事件在当前 `as_of` 反推一条闪现观察，禁止丢首完成。
/// 已提前由 Provisional→ForceOvertake 终局的身份仍独立登记后到的首完成，但终态吸收禁止
/// 回填 StructureCompleted。所有钟只取当前 trigger，禁止回填历史。
pub fn feed_replay_prefix(
    book: &mut NestLifecycleBook,
    feed: &ReplayPrefixFeed<'_>,
    material: &ForceMaterial,
) -> (Vec<LifecycleRevision>, ReplayFeedStats) {
    let mut stats = ReplayFeedStats::default();
    let mut live_observations: BTreeMap<LifecycleKey, LifecycleObservation> = BTreeMap::new();
    // 完成事件通道：只取盘整域（trend 域不在票 #426 范围——090 登记，见模块头）。
    let completions: Vec<LifecycleObservation> = feed
        .completion_events
        .iter()
        .filter(|event| event.kind == NestDivergenceKind::Consolidation)
        .map(|event| LifecycleObservation::event(*event, true))
        .collect();
    stats.completion_events = completions.len();
    for run in feed.runs {
        let segments: Vec<Segment> = run.legs.iter().map(leg_as_segment_copy).collect();
        // 自锚 = 逐段方向（与 `provide_nest_candidate_events_ext` level_view.rs:735 同口径，
        // 不是 `self_anchors`——禁第二查法）。
        let anchors: Vec<Option<Direction>> = segments
            .iter()
            .map(|segment| Some(segment.direction))
            .collect();
        for window in provide_pan_live_windows(
            run.level,
            run.centers,
            run.kinds,
            &segments,
            &anchors,
            feed.as_of,
        ) {
            stats.live_windows += 1;
            let observation = LifecycleObservation::pan_live(window, false);
            let key = observation.key();
            // 完成即停延展：终态身份不再喂行进中窗（终态吸收下逐位等价，见
            // `terminal_bridge_hit` 文档与 F5）。
            if book.terminal_bridge_hit(&key) {
                stats.extension_suppressed += 1;
                continue;
            }
            live_observations.insert(key, observation);
        }
    }
    let mut completion_phase = Vec::new();
    let mut completion_keys: Vec<LifecycleKey> = Vec::new();
    for completion in completions {
        let completed_key = completion.key();
        // 同 trigger 的 provider 可能经多个 run 重复产同一桥身份；物理观察数照记，
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
        // 稀疏 trigger 间开完又完成：首个可见事实虽只剩完成事件，仍在当前钟先建闪现观察。
        // 只复用事件的身份窗，不伪造历史 as_of。
        if matching_live.is_none() && book.bridge_entry(&completed_key).is_none() {
            let LifecycleObservation::Event { event, .. } = completion else {
                unreachable!("完成通道只由事件观察构造")
            };
            let flash = LifecycleObservation::pan_live(
                PanLiveWindow {
                    level: event.level,
                    side: event.side,
                    seg_a: event.seg_a,
                    seg_c_live: event.interval_b,
                    b_center_start: event.b_center_start,
                },
                false,
            );
            live_observations.insert(flash.key(), flash);
            matching_live = Some(flash);
        }

        // 首完成事实独立于终态：即使该身份已提前 ForceOvertake，也照实进入完整分母。
        if book.register_completion_signal(completed_key, feed.as_of) {
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

// ═══════════════════════════════════════════════════════════════════════════
// 测试族（卡 §7 + 勘误 + #78 锚定；全部确定性合成序列，T5/T11/T14 真实 provider 夹具）
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::center::UnitRange;
    use super::super::decompose::{center_block_kind, MoveBlock, MoveStatus};
    use super::super::divergence::self_anchors;
    use super::super::level_view::{
        assemble_level_view, lower_legs_from, provide_nest_candidate_events,
        project_extended_windows_carried_only, C2LevelViewConfig, C2VersionTuple,
        CoordinateWindow, LevelViewMaterial, LevelViewQuery, ProjectionMaterial,
    };
    use super::super::recursive_tower::{ElementId, LeveledMove};
    use super::super::super::types::Tick;

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
            &[LifecycleObservation::event(trend_event((120, 149), false, 149, 149), false)],
            149,
            &m,
        );
        assert_eq!(d.len(), 1);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Observed));
        assert_eq!(book.get(&key_trend((120, 149))).unwrap().state, NestEventState::Provisional);

        // as_of=155：确认事件（t*=155，interval_b 收束 (120,155)）——桥迁移 + first_provable。
        let d = book.advance(
            &[LifecycleObservation::event(trend_event((120, 155), true, 155, 155), false)],
            155,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 149))));
        assert!(matches!(d[1].kind, LifecycleRevisionKind::FirstProvable));

        // as_of=169：T5-OR 终假，provider 回扩坐标改发未确认事件 (120,169)——桥迁移后
        // Verified(false) ∧ 曾可证 ⟹ Invalidated(ForceOvertake)。
        let d = book.advance(
            &[LifecycleObservation::event(trend_event((120, 169), false, 169, 169), false)],
            169,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 155))));
        assert!(matches!(
            d[1].kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
        ));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 149, "observed_at 跨迁移不后移（E2E §1:83）");
        assert_eq!(entry.first_provable_at, Some(155), "first_provable = t*（不后移）");
        assert_eq!(entry.invalidated_at, Some(169));
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(entry.invalidated_reason, Some(InvalidatedReason::ForceOvertake));
        assert_eq!(entry.superseded_from, Some(key_trend((120, 155))), "迁移链留痕");
        // 反超证据载荷可查账（US-03）：c 三通道全面反超 a。
        let ev = entry.force_evidence.expect("ForceOvertake 留证据载荷");
        assert_eq!((ev.area_a, ev.area_c), (60.0, 150.0));
        assert_eq!((ev.dif_peak_a, ev.dif_peak_c), (5.0, 6.0));
        assert_eq!((ev.hist_peak_a, ev.hist_peak_c), (2.0, 3.0));

        // t=179 终态吸收：同 key 任何后续观察零输出（禁复活，E2E §1:83）。
        let d = book.advance(
            &[LifecycleObservation::event(trend_event((120, 169), false, 169, 179), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );
        assert_eq!(d.len(), 2, "Observed + FirstProvable");
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 129))).unwrap().first_provable_at,
            Some(129)
        );

        // as_of=139：c 窗延展柱力反超（面积 15+30=45、柱峰 3.0、黄白线峰 6.0）
        // ⟹ 三通道全假 ⟹ Invalidated(ForceOvertake)@139。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), false)],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 79), false)],
            79,
            &m,
        );
        // phase 2（as_of=99）：延展至恰好等力——面积 5+2+13=20、柱峰 2.0、黄白线峰 5.0。
        hist_eq[80] = -2.0;
        hist_eq[81..=93].fill(-1.0);
        dif_eq[80] = -5.0;
        let m = material(&hist_eq, &dif_eq, &close_src);
        let d = book_eq.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 99), false)],
            99,
            &m,
        );
        assert!(
            d.iter().any(|r| matches!(
                r.kind,
                LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), false)],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 149), false)],
            149,
            &m,
        );
        assert!(d.is_empty(), "延展窗零新 revision");
        // 状态机层 (b)：构造性「复活」弱窗（右端回缩到 129）——同样桥匹配终态 ⟹ 吸收。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 125), false)],
            125,
            &m,
        );
        // as_of=130/135：活窗延展（桥迁移 Supersedes），钟不后移。
        book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 130), false)],
            130,
            &m,
        );
        book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 135), false)],
            135,
            &m,
        );
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.observed_at, 125, "observed_at 写入不后移");
        assert_eq!(entry.first_provable_at, Some(125), "first_provable 写入不后移");
        // 同 as_of 重复 advance 幂等零 delta（E2E §1:83 as_of = state_as_of 最小形态）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 135), false)],
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
                status: MoveStatus::Completed,
            },
            MoveBlock {
                start_center: 1,
                end_center: 2,
                kind: MoveKind::Consolidation,
                dir: None,
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
        let baseline =
            provide_nest_candidate_events(1, &projection, &blocks, &legs, &view, &hist, &dif, &close_src);
        assert!(!baseline.is_empty(), "夹具应产事件（R1 全合取 confirmed 在案）");

        // 挂 book：同输入重跑 provider + 全量事件喂 advance。
        let attached =
            provide_nest_candidate_events(1, &projection, &blocks, &legs, &view, &hist, &dif, &close_src);
        let observations: Vec<_> = attached
            .iter()
            .map(|event| LifecycleObservation::event(*event, true))
            .collect();
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        let _ = book.advance(&observations, 139, &m);

        // 双判相等：PartialEq + Debug 序列化逐字节。
        assert_eq!(attached, baseline, "挂 book 后 provider 事件流逐字段相等（PartialEq）");
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
    #[test]
    fn t6_identity_vanish_on_structure_switch() {
        let close_src = identity_close_src(120);
        let hist = vec![0.0; 120];
        let dif = vec![0.0; 120];
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();
        // as_of=89：窄锚身份（seg_a=(50,69)，c 活窗 (70,89)）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 69), 70, 89), false)],
            89,
            &m,
        );
        assert_eq!(d.len(), 1, "仅 Observed（零力度序列 ⟹ Verified(false)，从未可证不判负）");
        // as_of=99：结构选择切换为 A′ 回退（seg_a=(20,39)）——seg_a 改变 ⟹ 桥不判同身份
        // （白名单不越界）；旧 key 本 prefix 不再产出 ⟹ Invalidated(IdentityVanished)@99。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((20, 39), 70, 99), false)],
            99,
            &m,
        );
        assert_eq!(d.len(), 2, "新身份 Observed + 旧身份 IdentityVanished");
        let old = book
            .get(&key_pan((50, 69), (70, 89)))
            .expect("旧 entry 保留（禁删除模拟失效）");
        assert_eq!(old.state, NestEventState::Invalidated);
        assert_eq!(old.invalidated_reason, Some(InvalidatedReason::IdentityVanished));
        assert_eq!(old.invalidated_at, Some(99));
        assert!(old.force_evidence.is_none(), "IdentityVanished 恒无力度证据");
        assert!(
            matches!(
                old.revisions.last().unwrap().kind,
                LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::IdentityVanished }
            ),
            "原因码入 revision 载荷（与 ForceOvertake 可区分）"
        );
        assert_eq!(book.len(), 2, "新身份建仓不受影响");
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), true)],
            139,
            &m,
        );
        assert_eq!(d.len(), 3, "Supersedes + StructureCompleted + Confirmed");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Confirmed);
        assert_eq!(entry.first_provable_at, Some(129), "first_provable 不后移");
        assert_eq!(entry.structure_end_at, Some(139));
        assert_eq!(entry.confirmed_at, Some(139));
        assert_eq!(book.consumable_closed().len(), 1, "Confirmed 进消费侧（Closed-only）");

        // (b) 完成窗已反超 ⟹ 可从 Provisional 直接 Invalidated(ForceOvertake)，
        // 无需先经 StructureCompleted，且 confirmed_at 保持 None。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let mut book_b = NestLifecycleBook::new();
        let m = material(&hist, &dif, &close_src);
        book_b.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );
        let d = book_b.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), true)],
            139,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
        )));
        assert!(d.iter().all(|r| !matches!(r.kind, LifecycleRevisionKind::Confirmed)));
        let entry_b = book_b.entries().next().unwrap().1;
        assert_eq!(entry_b.state, NestEventState::Invalidated);
        assert_eq!(entry_b.confirmed_at, None, "完成窗已反超不得 Confirmed（024:24）");
        assert_eq!(
            entry_b.structure_end_at, None,
            "Q3 第二终局路径不伪造 StructureCompleted"
        );
        assert!(book_b.consumable_closed().is_empty());
        book_b.assert_invariants();
    }

    /// T8 trend 身份迁移豁免（白名单工程桥，模块头 090 登记 1）：收束记 Supersedes
    /// （链留痕、钟不动、无 Invalidated）；负面对照 seg_a 改变 ⟹ 白名单不越界 ⟹
    /// IdentityVanished。feed 在每个 trigger 严格先活窗观察、后完成信号；同 trigger
    /// 闪现允许 Observed→StructureCompleted→终局同钟。`observed_at` 与
    /// `first_provable_at` 均在首次实际 `advance` 写入，跳过非 trigger prefix 只会晚记，
    /// 禁止回填，因此不能构造 `first_provable_at < observed_at`。
    #[test]
    fn t8_trend_identity_migration_whitelist() {
        let close_src = identity_close_src(200);
        let m = ForceMaterial::unavailable(&close_src); // 事件通道恒 Verified，无需力度序列
        let mut book = NestLifecycleBook::new();
        book.advance(
            &[LifecycleObservation::event(trend_event((120, 149), false, 149, 149), false)],
            149,
            &m,
        );
        // 确认收束 [120,149] → [120,155]：记 Supersedes（迁移链留痕、钟不动、无 Invalidated）。
        let d = book.advance(
            &[LifecycleObservation::event(trend_event((120, 155), true, 155, 155), false)],
            155,
            &m,
        );
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Supersedes { from } if from == key_trend((120, 149))));
        assert_eq!(book.len(), 1, "迁移即替换（唯一 remove 点，仅 Provisional 可达）");
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.superseded_from, Some(key_trend((120, 149))), "迁移链留痕");
        assert_eq!(entry.observed_at, 149, "钟不动");
        assert_eq!(entry.first_provable_at, Some(155));
        assert!(
            entry.revisions.iter().all(|r| !matches!(r.kind, LifecycleRevisionKind::Invalidated { .. })),
            "白名单迁移不记 Invalidated"
        );

        // 负面对照：seg_a 改变 ⟹ 白名单不越界 ⟹ 旧 key 走身份消失路径。
        let mut seg_a_changed = trend_event((120, 169), false, 169, 169);
        seg_a_changed.seg_a = (60, 109);
        let d = book.advance(&[LifecycleObservation::event(seg_a_changed, false)], 169, &m);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::IdentityVanished }
        )));
        let new_entry = book
            .get(&LifecycleKey { seg_a: (60, 109), ..key_trend((120, 169)) })
            .unwrap();
        assert!(
            matches!(new_entry.revisions.as_slice(), [LifecycleRevision { kind: LifecycleRevisionKind::Observed, .. }]),
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
            Center { zd: 98, zg: 110, dd: 90, gg: 116, start_index: 0, end_index: 39 },
            1,
            ElementId { level: 1, ordinal: 0 },
        );
        let w1 = LeveledMove::compose(
            &lower[4..8],
            Center { zd: 135, zg: 140, dd: 125, gg: 150, start_index: 40, end_index: 79 },
            1,
            ElementId { level: 1, ordinal: 1 },
        );
        let w2 = LeveledMove::compose(
            &lower[8..12],
            Center { zd: 160, zg: 165, dd: 140, gg: 175, start_index: 80, end_index: 119 },
            1,
            ElementId { level: 1, ordinal: 2 },
        );
        (vec![w0, w1, w2], lower)
    }

    /// pan 真实夹具：Consolidation 中枢 [20,49]（核心 [100,110]）+ 窄锚结构——
    /// A=[50,59]（Down 105→95 破核心）→ 回中枢段 [60,69]（Up 96→104 重回核心）→
    /// C episode [70,79]（Down 103→93 破核心新低）∪ [80,89]（Up 94→99 回拉不重回核心，
    /// episode 不复位）∪ [90,99]（Down 98→92 续创新低）。
    /// 力度：a 窗面积 20/柱峰 2.0/黄白线峰 5.0；c 一窗（[70,79]）全面更弱；
    /// c 延展段（[80,99]）取值由调用方定（T11 反超 / T14 保持弱）。
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
        let center = Center { zd: 100, zg: 110, dd: 90, gg: 120, start_index: 20, end_index: 49 };
        // 单中枢链 blocks=[Consolidation 0..0] ⟹ C_0 归 B₁ = Consolidation
        // （decompose.rs center_block_kind ownership 分区同口径）。
        let kinds = center_block_kind(
            1,
            &[MoveBlock {
                start_center: 0,
                end_center: 0,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Completed,
            }],
        );
        let segments = vec![
            Segment { direction: Direction::Down, start_index: 50, end_index: 59, start_price: 105, end_price: 95 },
            Segment { direction: Direction::Up, start_index: 60, end_index: 69, start_price: 96, end_price: 104 },
            Segment { direction: Direction::Down, start_index: 70, end_index: 79, start_price: 103, end_price: 93 },
            Segment { direction: Direction::Up, start_index: 80, end_index: 89, start_price: 94, end_price: 99 },
            Segment { direction: Direction::Down, start_index: 90, end_index: 99, start_price: 98, end_price: 92 },
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
        let (centers, kinds, segments, anchors, hist, dif, close_src) = pan_real_fixture(-3.0, -6.0);
        let m = material(&hist, &dif, &close_src);
        let mut book = NestLifecycleBook::new();

        // as_of=79：真实定位产窗（窄锚 A=(50,59)、c_start=70 同锚），三通道成立
        // （5<20、1<5、0.5<2）⟹ first_provable=79。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 79);
        assert_eq!(windows.len(), 1, "单一身份活窗");
        assert_eq!(windows[0].seg_a, (50, 59), "窄锚 A 经真实 locate_pan_div_structure 锚定");
        assert_eq!(windows[0].seg_c_live, (70, 79), "c_start_live 与完成后 seg_c.0 同锚（卡 §3）");
        let obs: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        book.advance(&obs, 79, &m);
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 79))).unwrap().first_provable_at,
            Some(79)
        );

        // as_of=99：活窗延展，真实现算三通道全假 ⟹ Invalidated(ForceOvertake) 可审计。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 99);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].seg_c_live, (70, 99), "活窗右端随 as_of 前进（设计内行为）");
        let obs: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        let d = book.advance(&obs, 99, &m);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
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
        assert!(book.consumable_closed().is_empty(), "Invalidated 可查账、不开放消费");
        // 终态吸收（含桥匹配）：as_of=109 活窗延展 (70,109) 仍零输出。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 109);
        let obs: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
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
        let (centers, kinds, segments, anchors, hist, dif, close_src) = pan_real_fixture(-0.1, -0.5);
        let mut book = NestLifecycleBook::new();

        // as_of=79：缺 hist/dif ⟹ MissingForceSeries 注记；无 Invalidated、不写 first_provable。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 79);
        let obs: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        let no_force = ForceMaterial::unavailable(&close_src);
        let d = book.advance(&obs, 79, &no_force);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable { reason: UnavailReason::MissingForceSeries }
        )));
        let key79 = key_pan((50, 59), (70, 79));
        let e = book.get(&key79).unwrap();
        assert_eq!(e.state, NestEventState::Provisional, "「不可验」≠「不再弱」");
        assert_eq!(e.first_provable_at, None, "Unavailable 不写 first_provable");
        assert_eq!(e.invalidated_at, None, "Unavailable 不判 Invalidated");

        // 同 as_of 注记幂等：重复 advance 零新 revision。
        let d = book.advance(&obs, 79, &no_force);
        assert!(d.is_empty(), "同 as_of 注记幂等去重");
        assert_eq!(book.get(&key79).unwrap().revisions.len(), 2, "Observed + 一条注记");

        // as_of=89：坐标映射失败子场景（close_src 截断到 70 之前 ⟹ c 窗映射失败）
        // ⟹ CoordinateMapFailed 注记，仍不判 Invalidated。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 89);
        let obs89: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        let short_src = identity_close_src(70);
        let m_trunc = ForceMaterial {
            hist: Some(&hist),
            dif: Some(&dif),
            close_src: &short_src,
        };
        let d = book.advance(&obs89, 89, &m_trunc);
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable { reason: UnavailReason::CoordinateMapFailed }
        )));
        assert_eq!(book.entries().next().unwrap().1.state, NestEventState::Provisional);

        // as_of=99：数据补齐 + 结构完成 ⟹ 恢复推进至 Confirmed（force_unavailable_at
        // 只作去重基准，不留残留状态阻塞）。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 99);
        let obs99: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, true)).collect();
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(&obs99, 99, &m);
        assert!(d.iter().any(|r| matches!(r.kind, LifecycleRevisionKind::Confirmed)));
        let entry = book.entries().next().unwrap().1;
        assert_eq!(entry.state, NestEventState::Confirmed);
        assert_eq!(entry.first_provable_at, Some(99), "补齐后 first_provable 正常写入");
        assert_eq!(entry.structure_end_at, Some(99));
        assert_eq!(entry.confirmed_at, Some(99));
        assert_eq!(book.consumable_closed().len(), 1, "Confirmed 可消费（Closed-only 边界内）");
        // 谱系构建读面（裁定 #64 §2(a)「构建放开」）：全 entry 可见；消费侧仍 Closed-only。
        assert_eq!(book.lineage_nodes().len(), 1);
        // 全程零 Invalidated（Unavailable 从未假杀身份——#78 修复 1 语义）。
        assert!(book.entries().all(|(_, e)| e.state != NestEventState::Invalidated));
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 99), false)],
            99,
            &m,
        );
        assert_eq!(d.len(), 1, "结构未完成 ⟹ 仅 Observed");
        let alive = book.get(&key_pan((50, 59), (70, 99))).unwrap();
        assert_eq!(alive.state, NestEventState::Provisional, "结构未完成不提前判负");
        assert_eq!(alive.first_provable_at, None, "从未可证");

        // 结构完成（ADR-0003：通道切换即完成信号）⟹ 转终态 + NeverConstituted。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 109), true)],
            109,
            &m,
        );
        assert_eq!(d.len(), 3, "Supersedes + StructureCompleted + Invalidated");
        assert!(matches!(
            d[2].kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::NeverConstituted }
        ));
        let key = key_pan((50, 59), (70, 109));
        let entry = book.get(&key).expect("终态留档不删（谱系保留）");
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(entry.invalidated_reason, Some(InvalidatedReason::NeverConstituted));
        assert_eq!(entry.first_provable_at, None, "从未构成 ⟹ 首次可证时点恒空");
        assert_eq!(entry.structure_end_at, Some(109));
        assert_eq!(entry.invalidated_at, Some(109), "结构完成即结算，不再挂账");
        assert_eq!(entry.revisions.len(), 4, "Observed + Supersedes + StructureCompleted + Invalidated");
        assert!(book.consumable_closed().is_empty(), "终态不进消费侧（Closed-only）");
        // 力度证据入载荷（模块头 090 登记 4「原因码与力度证据入载荷」）：c 三通道全面强于 a
        // ——审计者据此复核「从未构成」结论（a 面积 10×0.5、c 面积 40×3.0）。
        let ev = entry.force_evidence.expect("从未构成留力度证据载荷");
        assert_eq!((ev.area_a, ev.area_c), (5.0, 120.0));
        assert_eq!((ev.dif_peak_a, ev.dif_peak_c), (1.0, 6.0));
        assert_eq!((ev.hist_peak_a, ev.hist_peak_c), (0.5, 3.0));

        // 终态吸收（禁复活）：同身份后续活窗延展零输出。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 119), true)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 99), false)],
            99,
            &m_weak,
        );
        let d = book_weak.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 109), true)],
            109,
            &m_weak,
        );
        assert!(
            d.iter().any(|r| matches!(r.kind, LifecycleRevisionKind::Confirmed)),
            "反例：c 窗真弱 ⟹ 结构完成走 Confirmed"
        );
        let entry_weak = book_weak.entries().next().unwrap().1;
        assert_eq!(entry_weak.state, NestEventState::Confirmed);
        assert_eq!(entry_weak.invalidated_reason, None, "反例：NeverConstituted 不落");
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 99), true)],
            99,
            &no_force,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::ForceUnavailable { reason: UnavailReason::MissingForceSeries }
        )));
        assert!(
            d.iter().all(|r| !matches!(r.kind, LifecycleRevisionKind::Invalidated { .. })),
            "力度不可验 ⟹ 不判负（不伪造从未构成）"
        );
        let entry = book.get(&key_pan((50, 59), (70, 99))).unwrap();
        assert_eq!(entry.state, NestEventState::Provisional, "滞留活假设，诚实存疑");
        assert_eq!(entry.invalidated_reason, None);
        assert_eq!(entry.structure_end_at, None, "不可验 prefix 的完成信号不留痕（#78 原语义）");

        // as_of=109：数据补齐 + 完成信号重发 ⟹ 此时才结算为「从未构成」。
        let m = material(&hist, &dif, &close_src);
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 109), true)],
            109,
            &m,
        );
        assert!(d.iter().any(|r| matches!(
            r.kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::NeverConstituted }
        )));
        let entry = book.get(&key_pan((50, 59), (70, 109))).unwrap();
        assert_eq!(entry.invalidated_reason, Some(InvalidatedReason::NeverConstituted));
        assert_eq!(entry.structure_end_at, Some(109), "结算推迟到数据补齐的那一 prefix");
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 100), false)],
            100,
            &m,
        );
        let established = key_pan((50, 59), (70, 100));
        let before = book.get(&established).unwrap().clone();

        // 倒退 as_of=90 喂**右端不同**的同身份窗 (70,110)：该 key 不在 book 内 ⟹ 走桥匹配
        // 分支 ⟹ 显式拒绝（不迁移、不建仓、零 revision）。
        let bridged = key_pan((50, 59), (70, 110));
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 110), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 110), false)],
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
        };
        let win_b = |live_end: usize| PanLiveWindow {
            level: 2,
            side: Side::Long,
            seg_a: (110, 119),
            seg_c_live: (130, live_end),
            b_center_start: 105,
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
        assert_eq!(b.invalidated_reason, Some(InvalidatedReason::NeverConstituted));
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
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::ForceOvertake }
        ));
        assert!(matches!(
            b.revisions.last().unwrap().kind,
            LifecycleRevisionKind::Invalidated { reason: InvalidatedReason::NeverConstituted }
        ));

        // 分桶恰好一对一（不是「两条都落进同一桶」的蒙混）。
        let by_reason = |reason: InvalidatedReason| {
            book.entries()
                .filter(|(_, e)| e.invalidated_reason == Some(reason))
                .count()
        };
        assert_eq!(by_reason(InvalidatedReason::ForceOvertake), 1);
        assert_eq!(by_reason(InvalidatedReason::NeverConstituted), 1);
        assert_eq!(by_reason(InvalidatedReason::IdentityVanished), 0, "两者都不是身份消失");
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 100), false)],
            100,
            &m,
        );
        let key = key_pan((50, 59), (70, 100));
        let before = book.get(&key).unwrap().clone();

        // 倒退 as_of=90 喂同 key ⟹ 显式拒绝：零 revision、entry 零改动、注记在案。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 100), false)],
            90,
            &m,
        );
        assert!(d.is_empty(), "倒退 prefix 零 revision（禁静默吸收）");
        assert_eq!(book.get(&key).unwrap(), &before, "entry 零改动");
        assert_eq!(
            book.retrograde_rejections(),
            &[RetrogradeRejection { key, last_as_of: 100, rejected_as_of: 90 }]
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 100), false)],
            100,
            &m,
        );
        assert!(d.is_empty(), "同 as_of 幂等零 delta");

        // 合法前进照常：as_of=110 活窗延展（桥迁移 Supersedes，无新注记）。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 110), false)],
            110,
            &m,
        );
        assert_eq!(d.len(), 1);
        assert!(matches!(d[0].kind, LifecycleRevisionKind::Supersedes { .. }));
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
        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        assert_eq!(stats.live_windows, 1, "行进中通道产出单一活窗身份");
        assert_eq!(stats.completion_events, 0);
        assert_eq!(stats.channel_switches, 0, "无完成事件 ⟹ 不切换");
        assert_eq!(delta.len(), 2, "Observed + FirstProvable");
        let entry = book.get(&key_pan((50, 59), (70, 79))).expect("活假设建仓");
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
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 79)))
                .unwrap()
                .structure_end_at,
            None
        );

        // as_of=99：完成事件通道首次产出该身份 ⟹ 行进中窗让位、结构完成置真。
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &events,
            },
            &m,
        );
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
            .get(&key_pan((50, 59), (70, 99)))
            .expect("桥迁移后新键");
        assert_eq!(entry.structure_end_at, Some(99));
        assert_eq!(entry.state, NestEventState::Confirmed);

        // as_of=109：行进中窗仍在产（右端追 as_of），但身份已终态 ⟹ 停止喂入、零延展修订。
        let revisions_before = entry.revisions.len();
        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 109,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        assert_eq!(stats.live_windows, 1, "数据源仍产窗（未改数据源）");
        assert_eq!(
            stats.extension_suppressed, 1,
            "完成即停延展：终态身份不再喂行进中窗"
        );
        assert!(delta.is_empty(), "零延展修订");
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 99)))
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
            feed_replay_prefix(
                &mut book,
                &ReplayPrefixFeed {
                    as_of: 79,
                    runs: &runs,
                    completion_events: &[],
                },
                &m,
            );
            assert_eq!(
                book.get(&key_pan((50, 59), (70, 79)))
                    .unwrap()
                    .first_provable_at,
                Some(79),
                "切换前已可证（反超定义要求曾构成）"
            );
            let events = [pan_event((50, 59), (70, 99), event_confirmed, 99)];
            let delta = feed_replay_prefix(
                &mut book,
                &ReplayPrefixFeed {
                    as_of: 99,
                    runs: &runs,
                    completion_events: &events,
                },
                &m,
            )
            .0;
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
            feed_replay_prefix(
                &mut book,
                &ReplayPrefixFeed {
                    as_of: 79,
                    runs: &runs,
                    completion_events: &[],
                },
                &m,
            );
            let events = [pan_event((50, 59), (70, 99), false, 99)];
            feed_replay_prefix(
                &mut book,
                &ReplayPrefixFeed {
                    as_of: 99,
                    runs: &runs,
                    completion_events: &events,
                },
                &m,
            );
            let entry = book
                .get(&key_pan((50, 59), (70, 99)))
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
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 89,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        let snapshot = book.clone();
        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
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

    /// F6 侧车零反流（照 T5 逐字节护栏形态）：喂数出口对完成事件通道的入参**只读**，
    /// 出口前后事件流逐字节相等；且挂账本与不挂账本两种配置下入参流逐字节相同。
    #[test]
    fn f6_feed_exit_is_sidecar_events_byte_identical() {
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
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        // 不挂账本（基线）：入参原样。
        let baseline = events;
        // 挂账本：同一入参过一遍出口。
        let mut book = NestLifecycleBook::new();
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &events,
            },
            &m,
        );
        assert_eq!(events, baseline, "事件流逐字段相等（PartialEq）");
        assert_eq!(
            format!("{events:?}"),
            format!("{baseline:?}"),
            "Debug 序列化逐字节相等"
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
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &events,
            },
            &m,
        );
        assert_eq!(
            book.get(&key_pan((50, 59), (70, 99))).unwrap().state,
            NestEventState::Confirmed,
            "前置：身份已进终态"
        );
        // A 侧：出口跳过喂入（stats.extension_suppressed 记账）。
        let mut skipped = book.clone();
        let (delta_skip, stats) = feed_replay_prefix(
            &mut skipped,
            &ReplayPrefixFeed {
                as_of: 109,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
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
        let (_, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 139,
                runs: &runs,
                completion_events: &[trend],
            },
            &m,
        );
        assert_eq!(stats.completion_events, 0, "trend 域不计入完成事件通道");
        assert!(
            book.entries()
                .all(|(key, _)| key.kind == NestDivergenceKind::Consolidation),
            "trend 域事件不在账本建仓"
        );

        // 对照臂：先在前一 trigger 建活身份；盘整域完成事件随后正常计入并结算。
        let pan = pan_event((50, 59), (70, 99), true, 99);
        let mut book_pan = NestLifecycleBook::new();
        feed_replay_prefix(
            &mut book_pan,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        let (_, stats_pan) = feed_replay_prefix(
            &mut book_pan,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &[pan],
            },
            &m,
        );
        assert_eq!(stats_pan.completion_events, 1);
        assert!(
            book_pan.get(&key_pan((50, 59), (70, 99))).is_some(),
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
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        let mut book = NestLifecycleBook::new();

        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &events,
            },
            &m,
        );
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
            .get(&key_pan((50, 59), (70, 99)))
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
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &[],
            },
            &m,
        );
        let terminal = book.entries().next().unwrap().1;
        assert_eq!(
            terminal.invalidated_reason,
            Some(InvalidatedReason::ForceOvertake)
        );
        assert_eq!(terminal.structure_end_at, None, "Q3 第二终局路径不经结构完成");

        let events = [pan_event((50, 59), (70, 99), true, 99)];
        let (_, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 109,
                runs: &[],
                completion_events: &events,
            },
            &m,
        );
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
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        let mut book = NestLifecycleBook::new();
        let (delta, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &[],
                completion_events: &events,
            },
            &m,
        );
        assert_eq!(stats.completion_signals, 1);
        assert!(matches!(delta.first().unwrap().kind, LifecycleRevisionKind::Observed));
        assert!(delta
            .iter()
            .any(|revision| matches!(revision.kind, LifecycleRevisionKind::StructureCompleted)));
        assert!(matches!(
            delta.last().unwrap().kind,
            LifecycleRevisionKind::Confirmed
        ));
        assert_eq!(book.settlement_stats().flash_terminal_count, 1);
    }

    /// F9 #428 升格硬项：完成信号已到但力度不可验时不得静默接受。
    ///
    /// 必须经生产主接缝 `feed_replay_prefix` 可达；这条只要求独立审计事实可查，
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

        feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 79,
                runs: &runs,
                completion_events: &[],
            },
            &available,
        );
        let events = [pan_event((50, 59), (70, 99), true, 99)];
        let (_, stats) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 99,
                runs: &runs,
                completion_events: &events,
            },
            &no_force,
        );

        assert_eq!(
            book.completion_force_unavailable_audits(),
            &[CompletionForceUnavailableAudit {
                key: key_pan((50, 59), (70, 99)),
                as_of: 99,
                reason: UnavailReason::MissingForceSeries,
            }],
            "完成信号与力度缺失须经生产接缝成为同一条独立审计事实"
        );
        assert_eq!(stats.channel_switches, 1, "完成信号命中先前活身份");
        assert_eq!(stats.completion_signals, 1, "唯一完成信号作为发生率分母");
        assert_eq!(stats.completion_force_unavailable, 1);

        let (_, repeated) = feed_replay_prefix(
            &mut book,
            &ReplayPrefixFeed {
                as_of: 109,
                runs: &runs,
                completion_events: &events,
            },
            &no_force,
        );
        assert_eq!(repeated.completion_signals, 0, "跨 trigger 重发不重复抬高分母");
        assert_eq!(repeated.channel_switches, 0, "同一信号重发不是第二次通道切换");
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
}
