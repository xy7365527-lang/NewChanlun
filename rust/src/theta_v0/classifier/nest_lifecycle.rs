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
//!   provenance。**级别有效域**：只有 L1 有 active lower-frontier；L2/L3 的 `LeveledMove`
//!   在塔上没有 Active/Completed 表达（#523 遗留 1），故其身份仍只经完成相进账本——那是
//!   provider 能力缺口，**不冒充** true-flash。
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
use super::divergence::{
    same_color_area, same_dir_hist_peak, segment_dif_peak, segments_diverge_or,
};
use super::ledger_kernel::{
    first_write_clock, LedgerAdmission, LedgerBook, LedgerDelta, LedgerEntryCore, LedgerPolicy,
    LedgerRetrogradeRejection, LedgerRevision, LedgerSettlement, LedgerState,
};
use super::level_view::{LowerLeg, NestCandidateEvent, NestDivergenceKind};
use super::center::{center_from_segments, UnitRange};
use super::recursive_tower::{detect_centers_windowed_resume, map_src_to_close_idx, ElementId};
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

/// 同**锚**判定（票 #559）：身份键去掉 `seg_c_full` 的五元相等。
///
/// 与 [`bridge_identity`] 的分工：桥要求 C 左端也相同（右端延展吸收为同身份）；同锚只要求
/// A/B 锚与级别/方向/域相同，**允许 C 左端不同**——那正是「provider 换到下一个 C」的形态。
/// 两者关系：`bridge_identity ⟹ same_anchor`（严格更强）。
fn same_anchor(a: &LifecycleKey, b: &LifecycleKey) -> bool {
    a.level == b.level
        && a.side == b.side
        && a.kind == b.kind
        && a.seg_a == b.seg_a
        && a.b_center_start == b.b_center_start
}

/// 中枢升级认领判据（票 #603 档 1「暂认中枢回溯认领」，#599 编排者裁定 2026-07-28）。
///
/// **严格同锚**：`level/side/kind/seg_a/seg_c_full.0` 五项全等（= 同一背驰假设的同一 C 段），
/// **只放开 `b_center_start`**，且要求新 B 严格晚于旧 B（`old < new`，单调前进）。
///
/// 语义（为什么这不是"回填历史"）：B 的取值来自
/// [`nearest_confirmed_center_idx`] 对**已确认**中枢集合的查询——中枢从"未确认"变为"已确认"
/// 是回顾性、单调的状态跃迁，查询答案随之更新反映的是「当下可得的已确认信息集合变大了」，
/// 不是对未来的预判（`parser/tail.rs:11-14` 裁定的边界在此侧）。与 [`bridge_identity`] 已经
/// 承认的「C 收束/回扩/延展」是同一原则的推广：同一 A/C 组合，用当下最新已确认信息重算一个
/// 辅助参数。**单调方向是合法性的前提**——反向替换（用更早的中枢覆盖当前答案）才真正构成
/// 回填，本判据禁之。
///
/// **跨锚零实装**（#599 §4.2 教义否定，编排者采纳）：`seg_a` 是背驰检验的离开段，两个不同
/// `seg_a` 是两个**不同的背驰假设**（巧合共享同一 C 段），合并 = 用一个假设的最终结果回溯
/// 重定义另一个假设曾经的观测内容 = `tail.rs`「强行分类为最终结果」。故 `seg_a` 全等是硬
/// 约束，本函数不提供任何放开它的分支。
///
/// 与 [`bridge_identity`] 的关系：两者**互斥**（桥要求 `b_center_start` 相等，本判据要求严格
/// 不等）⟹ `advance` 第 1 步先桥后认领，匹配不重叠。C 右端不进本判据（与桥同款，右端随
/// as_of/收束变动，不是身份）。
fn bridge_by_center_upgrade(old: &LifecycleKey, new: &LifecycleKey) -> bool {
    old.level == new.level
        && old.side == new.side
        && old.kind == new.kind
        && old.seg_a == new.seg_a
        && old.seg_c_full.0 == new.seg_c_full.0
        && old.b_center_start < new.b_center_start
}

// ═══════════════════════════════════════════════════════════════════════════
// 三态 / 原因码 / 力度三值化（卡 §2.3/§4 + #78 修复 1）
// ═══════════════════════════════════════════════════════════════════════════

/// 三态（E2E-D5 最小子集；`Unresolved` 出切片——模块头 090 登记 2）。
///
/// **本类型即内核三态**（票 #573 T1 重基）：[`LedgerState`] 承载「未决 / 成立 / 失效」这条
/// **账本纪律**（终态吸收、禁复活、终态钟只写一次），三个变体逐位同名同序。域语义
/// ——何时进入哪一态、该态在缠论下的名分——仍在本模块，逐条登记如下：
///
/// - `Provisional` = 活假设（061:26「都可以先假设是进入背驰段」——默认态，非例外态）;
/// - `Confirmed` = first_provable 已写 ∧ c 结构完成 ∧ 完成时复核仍弱（024:24）;
/// - `Invalidated` = 力度反超（061:26，可由 Provisional 直接到达，无需结构先完成）、
///   从未构成（061:28）或身份消失（E2E §1:81）；终态留档，禁删除模拟失效。
///
/// `is_terminal` 由内核提供（同一条纪律，无域内容），语义一个 bit 不变。
pub type NestEventState = LedgerState;

/// 活假设账本的泛化面实例（票 #573 T1）：把 11 个对象无关责任点接到内核
/// [`LedgerBook`]，域侧只留自己的判据（桥/同锚/中枢升级、三态业务条件、原因码含义、
/// 域钟语义、provider/feed、力度判定、完成信号与消失扫描）。
///
/// 四组类型参数的域取值：`Key` = [`LifecycleKey`]（六元组）、`Observation` =
/// [`LifecycleObservation`]（事件 / pan 活窗双通道）、`RevisionKind` =
/// [`LifecycleRevisionKind`]、ReasonPayload = [`InvalidatedReason`] + [`ForceEvidence`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestPolicy;

impl LedgerPolicy for NestPolicy {
    type Key = LifecycleKey;
    type Observation = LifecycleObservation;
    type RevisionKind = LifecycleRevisionKind;
    type Reason = InvalidatedReason;
    type Evidence = ForceEvidence;
    type Entry = NestLifecycleEntry;

    fn observation_key(observation: &Self::Observation) -> Self::Key {
        observation.key()
    }

    fn opened_revision_kind() -> Self::RevisionKind {
        LifecycleRevisionKind::Observed
    }
}

/// 身份消失的两类成因（票 #559 编排者裁定 2026-07-28；挂 #523 遗留问题 2 名下）。
///
/// 裁定原文：「凡消失的身份必须带可审计原因码，且原因码拆两类落**账本字段**——
/// (a) 假设被推翻（真终局）；(b) 观测接缝伪影（provider 换轨丢下）。混记会污染
/// 寿命/反超率统计，禁。」
///
/// **判据（`advance` 第 8 步现算，唯一分类点）**：消失身份的**锚**
/// = `(level, side, kind, seg_a, b_center_start)`（身份键去掉 `seg_c_full`）。
/// 本 prefix 的观察集合里若仍有同锚身份（必然是不同 `seg_c_full.0`——同左端会被
/// [`bridge_identity`] 吸收成 `Supersedes` 而不进消失扫描），则该锚的假设并未死，
/// 只是 provider 把 C 换到了下一段 ⟹ (b)；否则该锚本 prefix 完全不再产窗 ⟹ (a)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanishCause {
    /// (a) 假设被推翻（真终局）：本 prefix 该锚不再产出任何窗，结构前提被后续演化否证。
    /// 教义依据：`parser/tail.rs`「未完成走势只能唯一分类为当下状态，不能强行分类为
    /// 最终结果」⟹ 基于当下状态建立的活假设必然可被后续结构演化取消。
    HypothesisRefuted,
    /// (b) 观测接缝伪影：同 prefix 同锚仍有窗/事件产出，只是 C 段左端换了
    /// （`successor_c_start`）。假设本身没死，是 provider 换轨把旧 key 丢下——与 #523
    /// 钉的根因同类（从「首见即完成」变成「窗口错配」）。
    ///
    /// `successor_c_start` = 本 prefix 同锚新身份的 `seg_c_full.0`；锚其余五元与消失身份
    /// 逐位相同，故该值唯一定位接手身份（右端随 bar 延展，不进身份）。同 prefix 存在多个
    /// 同锚新身份时取 `BTreeSet` 序首个（确定性，无平局歧义）。
    ObservationSeam { successor_c_start: usize },
}

/// 失效原因码（入 revision 载荷，诊断可查账；裁定 #64 §2(b)）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidatedReason {
    /// 被反超：**曾构成、后被否证**（061:26「一旦力度大于前者，那么就可以断定背驰段不
    /// 成立」）——要求曾写入 first_provable，是「先成立、再被推翻」。
    ForceOvertake,
    /// 身份消失（上一 prefix 有、本 prefix 不再产出该 key；E2E §1:81）。成因两分见
    /// [`VanishCause`]（票 #559 裁定：混记禁）。
    IdentityVanished { cause: VanishCause },
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
    /// 中枢升级认领（票 #603 档 1）：同一候选（同 `seg_a` 同 C 左端）的 B 参照被一个**更晚
    /// 确认、因而更近**的中枢替换（判据 [`bridge_by_center_upgrade`]）。两种形态，由前身
    /// 死活决定，**都不改前身条目的任何 bit**：
    ///
    /// - 前身仍 `Provisional` ⟹ **迁移**：与 `Supersedes` 同款（remove + 继承五钟 + 链留痕），
    ///   前身条目被迁移走，无 `Invalidated`、无 `IdentityVanished`；
    /// - 前身已终态 ⟹ **认领留痕**：新身份独立建仓（`Observed` 后紧跟本修订），前身条目
    ///   原样留档（终态吸收/禁复活/终态钟只写一次三条不变量一个 bit 不动，E2E §1:83）。
    ///   本修订是两条历史之间**唯一**的可审计关联——统计口径按**本修订自带的 `from`** 反查
    ///   即可把「被认领的前身终局」从反超/寿命分母中剔除（#599 §5-1「口径正确性修复」）。
    ///   **禁经 `superseded_from` 中转**（票 #619 H1）：那是「当下来源指针」，每次桥迁移
    ///   整体覆盖；修订链 append-only、永不覆盖，才是认领事件的权威载体。
    CenterUpgraded { from: LifecycleKey },
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

/// 迁移来源码（[`NestLifecycleBook::migrate_entry`] 的唯一入参形态，票 #619 L12）。
///
/// 只说「哪一种迁移」，**不带 `from`**——`from` 由迁移点自己填（它就是被 `remove` 的那个键），
/// 调用方无从填错。两处逐句同构的迁移块收敛到单一写入点后，「写 `superseded_from`」与「写哪
/// 种来源修订」的一致性由类型保证，而非靠两处代码各自遵守纪律（Fowler: Duplicated Code）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationKind {
    /// 白名单工程桥（除 C 右端外全等，判据 [`bridge_identity`]）。
    Bridge,
    /// 中枢升级认领（严格同锚 + B 单调前进，判据 [`bridge_by_center_upgrade`]，票 #603 档 1）。
    CenterUpgrade,
}

/// 一条修订（entry 留档 + advance 增量返回双通道；禁删除模拟失效，E2E §1:81）。
///
/// **本类型即内核修订**（票 #573 T1 重基）：[`LedgerRevision`] 四字段与旧域结构逐位同构
/// ——`key`（修订发生时 entry 的身份键，Supersedes 后为新键）、`kind`、`as_of`（产生该修订
/// 的 prefix，一切钟 ≤ 此 as_of，E2E-S5 判据）、`evidence`（力度证据：ForceOvertake /
/// NeverConstituted 且力度序列齐备时现算，IdentityVanished 恒 None）。append-only 纪律与
/// 「计数 == 留档长度」由内核唯一追加点 [`LedgerEntryCore::push_revision`] 结构性保证。
pub type LifecycleRevision = LedgerRevision<NestPolicy>;

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
    /// 身份消失成因（票 #559 裁定：两类落账本字段）。**恒与 `invalidated_reason` 同真**
    /// ——`IdentityVanished{cause}` ⟺ `Some(cause)`，其余原因码恒 `None`
    /// （`assert_invariants` 逐条钉死；唯一写入点 `invalidate`）。
    pub vanish_cause: Option<VanishCause>,
    /// 力度证据终态留档（ForceOvertake 与 NeverConstituted 现算，材料缺则诚实 None；
    /// IdentityVanished 恒 None——无力度语义）。
    pub force_evidence: Option<ForceEvidence>,
    /// 最近一次力度不可验注记 prefix（同 as_of 幂等去重基准；只作去重，不阻塞恢复推进）。
    pub force_unavailable_at: Option<usize>,
    /// 身份来源链的**当下来源指针**（两码：白名单桥迁移 `Supersedes` / 中枢升级认领
    /// `CenterUpgraded`，两种迁移都不改写任何钟）。
    ///
    /// **有效域（票 #619 H1 订正）**：本字段是单个 `Option`、**每次迁移整体覆盖**——它只
    /// 回答「这只 entry 的当下身份是从哪个键搬过来/认领来的」，不承载历史。留痕认领产出的
    /// 新身份若在后续 bar 被桥迁移，本字段即被改写为「上一 bar 的自己」，与终态前身的关联
    /// 从本字段消失。故**认领关联的权威来源是 append-only 的 `CenterUpgraded` 修订**
    /// （`revisions` 内自带 `from`，见 [`NestLifecycleBook::settlement_stats`] 的 claimed
    /// 集合），本字段不得用于任何跨 bar 的口径统计。
    pub superseded_from: Option<LifecycleKey>,
    /// 该身份已见最大 as_of（#78 修复 2 倒退守卫基准）。
    pub last_as_of: usize,
    /// 修订留档（append-only）。
    pub revisions: Vec<LifecycleRevision>,
}

/// 条目通用面（票 #573 T1）：域字段布局一个 bit 不动，只经访问器把内核所需的读写口
/// 暴露出去。追加（`push_revision`）与终态落账（`settle`）两条写入路径由内核默认实现
/// 独占 ⟹「修订计数 == 留档长度」不再靠本模块自觉维护，而是结构性成立。
impl LedgerEntryCore<NestPolicy> for NestLifecycleEntry {
    /// 建空白条目：三态默认 Provisional、观察钟与门卫钟同取 `as_of`、五钟其余为空、
    /// 计数 0、留档空、无来源链。建项修订由 [`LedgerBook::open_on_observation`] 追加。
    fn open(key: LifecycleKey, as_of: usize) -> Self {
        Self {
            key,
            state: NestEventState::Provisional,
            revision: 0,
            observed_at: as_of,
            first_provable_at: None,
            structure_end_at: None,
            confirmed_at: None,
            invalidated_at: None,
            invalidated_reason: None,
            vanish_cause: None,
            force_evidence: None,
            force_unavailable_at: None,
            superseded_from: None,
            last_as_of: as_of,
            revisions: Vec::new(),
        }
    }

    fn key(&self) -> LifecycleKey {
        self.key
    }

    fn set_key(&mut self, key: LifecycleKey) {
        self.key = key;
    }

    fn state(&self) -> NestEventState {
        self.state
    }

    fn set_state(&mut self, state: NestEventState) {
        self.state = state;
    }

    fn revision_count(&self) -> u32 {
        self.revision
    }

    fn set_revision_count(&mut self, count: u32) {
        self.revision = count;
    }

    fn revisions(&self) -> &[LifecycleRevision] {
        &self.revisions
    }

    fn revisions_mut(&mut self) -> &mut Vec<LifecycleRevision> {
        &mut self.revisions
    }

    /// 出生钟 = 首见 prefix（建仓写一次、无任何改写点）。
    fn opened_at(&self) -> usize {
        self.observed_at
    }

    fn last_as_of(&self) -> usize {
        self.last_as_of
    }

    fn set_last_as_of(&mut self, as_of: usize) {
        self.last_as_of = as_of;
    }

    /// 落锤钟合并读数：域按终态分列两只字段（终态互斥 ⟹ 至多一只有值）。
    fn settled_at(&self) -> Option<usize> {
        self.confirmed_at.or(self.invalidated_at)
    }

    fn migrated_from(&self) -> Option<LifecycleKey> {
        self.superseded_from
    }

    fn set_migrated_from(&mut self, from: LifecycleKey) {
        self.superseded_from = Some(from);
    }

    /// 终态载荷落账（内核 [`LedgerEntryCore::settle`] 的域侧写入部）：按终态写对应的落锤钟
    /// 与载荷。`vanish_cause` 由 `reason` 现场投影 ⟹ 两个字段结构上不可能不一致；
    /// Confirmed 臂不碰任何 Invalidated 侧字段（正向终态无原因码、无力度证据）。
    fn write_settlement(
        &mut self,
        state: NestEventState,
        reason: Option<InvalidatedReason>,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) {
        match state {
            NestEventState::Confirmed => self.confirmed_at = Some(as_of),
            NestEventState::Invalidated => {
                self.invalidated_at = Some(as_of);
                self.invalidated_reason = reason;
                self.vanish_cause = match reason.expect("Invalidated 必有原因码") {
                    InvalidatedReason::IdentityVanished { cause } => Some(cause),
                    InvalidatedReason::ForceOvertake | InvalidatedReason::NeverConstituted => None,
                };
                self.force_evidence = evidence;
            }
            NestEventState::Provisional => unreachable!("非终态不入终态落账"),
        }
    }
}

impl NestLifecycleEntry {
    /// 转入终态 `Invalidated` 并留档（三个原因码的**唯一**写入点——原因码与力度证据同写，
    /// 结构上兑现模块头 090 登记 4「原因码与力度证据入载荷」；无删除路径，禁删除模拟失效）。
    ///
    /// `evidence`：ForceOvertake / NeverConstituted 传本 prefix 现算证据（材料缺则诚实
    /// None）；IdentityVanished 传 None（无力度语义）。转终态与追加修订的次序纪律由内核
    /// [`LedgerEntryCore::settle`] 承担（全内核唯一转终态点），本方法只装域载荷。
    fn invalidate(
        &mut self,
        reason: InvalidatedReason,
        as_of: usize,
        evidence: Option<ForceEvidence>,
    ) -> LifecycleRevision {
        self.settle(
            LedgerSettlement {
                state: NestEventState::Invalidated,
                kind: LifecycleRevisionKind::Invalidated { reason },
                reason: Some(reason),
                evidence,
            },
            as_of,
        )
    }

    /// 转入终态 `Confirmed` 并留档（完成时复核仍弱，024:24；正向终态无原因码、无力度证据）。
    fn confirm(&mut self, as_of: usize) -> LifecycleRevision {
        self.settle(
            LedgerSettlement {
                state: NestEventState::Confirmed,
                kind: LifecycleRevisionKind::Confirmed,
                reason: None,
                evidence: None,
            },
            as_of,
        )
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
    /// 诊断字段（票 #592 裁定）：frontier 起点与最近 confirmed 段末端的洞长（无洞=0），
    /// 与 [`provide_active_pan_live_windows`] 内 `frontier_not_after_confirmed` 判定
    /// 同源同值——源头照实全收、拒绝判断留给消费端（账本照实记录哲学，不在本字段上二次
    /// 加判据）。非 L1 active-frontier 通道（`provide_pan_live_windows`/完成事件闪现）不涉及
    /// frontier 概念，恒 `0`。
    pub gap_len: usize,
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
///
/// **本类型即内核注记**（票 #573 T1 重基）：三字段逐位同构——`key`、`last_as_of`
/// （该身份已见最大 as_of）、`rejected_as_of`（被拒绝的倒退 as_of）。
pub type RetrogradeRejection = LedgerRetrogradeRejection<LifecycleKey>;

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
///
/// **完成钟 provenance 两分**（票 #527；#523 遗留问题 3；票 #559 条件 C3 订正）：两个时点
/// 分列，禁互相冒充——`completed_at`（lower unit **物理完成** bar，= 该单元 `end_index`）
/// ≤ `as_of`（**账本收到** bar，= `advance` 的时钟）。二者当前并不相等，故不得合并记账。
///
/// **C3 订正（为什么是两钟而不是三钟）**：#527 曾分列第三钟 `observed_completion_at`
/// （完成 Event 首次可见 bar）。但现行接线下 provider 相的构造与账本喂入在**同一 bar、
/// 同一调用链**内完成（`p123_fast_replay::lifecycle_bar_phases` → `feed_lifecycle_bar`），
/// 该钟被硬写为 `as_of`，BTC 100k 全量 248/248 恒等 ⟹ 它不是独立可测时点，只是 `as_of`
/// 的别名。保留一个恒等于另一字段的钟 = 声明代码不具备的分辨力（090 声明 = 能力）。
/// 故删除，只留两钟；`as_of − completed_at` 即完整的完成可见性滞后读数（口径不变）。
/// 若将来 provider 相构造与账本喂入解耦（批量/跨 bar 缓冲），第三钟才成为真实时点，
/// 届时按真实产出重新引入——**不预留空字段**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionSignal {
    pub key: LifecycleKey,
    /// 账本收到 bar（`advance` 时钟）。
    pub as_of: usize,
    /// 完成的 lower unit 身份（provider 查证给出，消费方不按 `kind` 猜 provenance）。
    pub completed_lower_id: ElementId,
    /// lower unit 物理完成 bar。
    pub completed_at: usize,
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
    /// 其中被**中枢升级认领**的前身数（票 #603 档 1）：该反超身份随后被一个同锚、B 更晚的
    /// 身份以 `CenterUpgraded` 认领 ⟹ 那条反超是「更精确的 B 出现之前的暂时状态」，
    /// 按 #599 §5-1 的口径正确性修复应从反超分母中剔除。
    ///
    /// **账本一个 bit 不改**：前身条目仍是 `Invalidated{ForceOvertake}` 原样留档（终态吸收/
    /// 禁复活/终态钟只写一次三条不变量不动）——纠误只发生在**口径层**，
    /// `force_overtake_count - force_overtake_claimed_count` 即纠误后的反超数。
    pub force_overtake_claimed_count: usize,
    pub never_constituted_count: usize,
    /// 身份消失 (a) 假设被推翻（真终局）——票 #559 裁定两类分列，禁合并计数。
    pub identity_vanished_refuted_count: usize,
    /// 身份消失 (b) 观测接缝伪影（provider 换轨丢下）——**不得**进入寿命/反超率统计
    /// （票 #559 裁定：混记会污染统计）。
    pub identity_vanished_seam_count: usize,
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
    /// 账本内核（票 #573 T1）：per-key 注册表 + 倒退拒绝注记面。11 个对象无关责任点
    /// （建项 / 追加 / 计数一致 / 倒退拒绝 / 终态吸收 / 钟首写 / 增量 / 迁移 / 枚举 /
    /// 不变量骨架）全在此，本模块只加自己的判据与域结算。
    ///
    /// 倒退拒绝注记随内核走：同一倒退喂入重复执行重复记录（重复违规事实本身；与
    /// ForceUnavailable 同 as_of 幂等不对称——原 #78 评审 Low 登记同判，不阻塞）。
    ledger: LedgerBook<NestPolicy>,
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
        self.ledger.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ledger.is_empty()
    }

    /// entry 读面（TDD 接缝外部行为断言面）。
    pub fn get(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        self.ledger.get(key)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&LifecycleKey, &NestLifecycleEntry)> {
        self.ledger.entries()
    }

    /// 倒退拒绝注记门户（#78 修复 2 审计面）。
    pub fn retrograde_rejections(&self) -> &[RetrogradeRejection] {
        self.ledger.retrograde_rejections()
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
            entry_count: self.ledger.len(),
            ..LifecycleSettlementStats::default()
        };
        let mut nonflash_lifetimes = Vec::new();
        let mut force_overtake_lifetimes = Vec::new();
        // 票 #603 档 1：被 `CenterUpgraded` 认领的前身键集合。来源取 **append-only 修订链里
        // 每一条 `CenterUpgraded` 自带的 `from`**（票 #619 H1 订正）——不经 `superseded_from`
        // 中转：后者是「当下来源指针」，认领方只要多活一个 bar 被桥迁移（dump 实证桥迁移逐
        // bar 发生），该指针即被改写为「上一 bar 的自己」，认领关联静默丢失 ⟹ 本口径少计。
        // 取全部 `from`（不止首条）：同一 entry 可先留痕认领终态前身、再逐 bar 迁移，迁移写
        // 入的 `from` 是自己的旧键——旧键已被 `remove`、不在 `entries` 内，`claimed.contains`
        // 恒不命中，不污染口径。账本自足，不另存状态。
        let claimed: std::collections::BTreeSet<LifecycleKey> = self
            .ledger
            .values()
            .flat_map(|entry| entry.revisions.iter())
            .filter_map(|revision| match revision.kind {
                LifecycleRevisionKind::CenterUpgraded { from } => Some(from),
                _ => None,
            })
            .collect();
        for entry in self.ledger.values() {
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
                        stats.force_overtake_claimed_count +=
                            usize::from(claimed.contains(&entry.key));
                        (entry.invalidated_at, true)
                    }
                    InvalidatedReason::NeverConstituted => {
                        stats.never_constituted_count += 1;
                        (entry.invalidated_at, false)
                    }
                    InvalidatedReason::IdentityVanished { cause } => {
                        match cause {
                            VanishCause::HypothesisRefuted => {
                                stats.identity_vanished_refuted_count += 1
                            }
                            VanishCause::ObservationSeam { .. } => {
                                stats.identity_vanished_seam_count += 1
                            }
                        }
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
        self.ledger.in_state(NestEventState::Confirmed)
    }

    /// 谱系构建节点读面（裁定 #64 §2(a)「构建放开」）。
    ///
    /// ★红线：本读面仅供谱系**构建**侧；**禁止**出现在任何
    /// `d_parent_interval_snapshot`/装配候选/证书真值路径（nest.rs:802-810/:987-989
    /// 装配输入不变；白名单工程桥不得进证书真值路径——裁定 §2(c) 裁定义务）。
    pub fn lineage_nodes(&self) -> Vec<&NestLifecycleEntry> {
        self.ledger.values().collect()
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
        let mut delta = LedgerDelta::new();
        let mut seen = std::collections::BTreeSet::new();
        for obs in observations {
            let key = obs.key();
            seen.insert(key);
            // 第 1 步：建仓 / 白名单桥迁移 / 终态吸收（含桥匹配到终态）。
            if !self.ledger.contains(&key) {
                match self.bridge_match(&key) {
                    Some(old_key) => {
                        let old = self.ledger.get(&old_key).expect("桥匹配键在册");
                        let (old_last_as_of, old_terminal) =
                            (old.last_as_of, old.state.is_terminal());
                        // 同一身份的倒退喂入：显式拒绝（不迁移、不建仓、零 revision）。
                        if as_of < old_last_as_of {
                            self.ledger.reject_retrograde(key, old_last_as_of, as_of);
                            continue;
                        }
                        // 终态吸收含桥匹配：终态不迁移、不建仓、零输出（禁复活，E2E §1:83）。
                        if old_terminal {
                            continue;
                        }
                        // Supersedes 迁移（仅 Provisional 可达——终态已在上一步吸收）：
                        // 钟不动、链留痕（单一写入点 [`Self::migrate_entry`]）。
                        let revision =
                            self.migrate_entry(old_key, key, MigrationKind::Bridge, as_of);
                        delta.record(revision);
                    }
                    None => {
                        // 档 1（票 #603 / #599 裁定）：桥未命中 ⟹ 试中枢升级认领（严格同锚，
                        // 判据 [`bridge_by_center_upgrade`]）。**前身条目一律不改任何 bit**，
                        // 两形态由前身死活决定：
                        //  - 前身仍 Provisional 且非倒退 ⟹ **迁移**（继承五钟，与 Supersedes 同款）；
                        //  - 前身已终态 / 倒退喂入 ⟹ **认领留痕**（新身份独立建仓 + 关联修订，
                        //    前身原样留档——终态吸收/禁复活/终态钟只写一次三条不变量不动）。
                        // 优选由 `center_upgrade_match` 一并给出（票 #619 H2：留痕形态把终态
                        // 前身留在册 ⟹ 同链可**匹配**多只，必须择优而非按字节序盲取）。
                        let claim = self.center_upgrade_match(&key, as_of);
                        match claim {
                            (Some(old_key), true) => {
                                // 迁移（钟不动、链留痕、无 Invalidated；单一写入点）。
                                let revision = self.migrate_entry(
                                    old_key,
                                    key,
                                    MigrationKind::CenterUpgrade,
                                    as_of,
                                );
                                delta.record(revision);
                            }
                            (claimed_from, _) => {
                                // 新身份建仓（内核首次观察建项：Provisional + 观察钟/门卫钟同取
                                // as_of + 一条 `Observed` 建项修订；observed_at 建仓写一次、无任何
                                // 改写点。倒退 as_of 的新身份无基线可违照建——#78 评审信息项，
                                // observed_at ≤ last_as_of 不受损）。`claimed_from` 有值 ⟹ 紧跟
                                // 一条认领留痕修订。
                                let revision = self
                                    .ledger
                                    .open_on_observation(obs, as_of)
                                    .expect("建仓分支的身份此前不在册");
                                delta.record(revision);
                                if let Some(old_key) = claimed_from {
                                    let entry =
                                        self.ledger.get_mut(&key).expect("本轮刚建仓的身份在册");
                                    entry.set_migrated_from(old_key);
                                    let claim_revision = entry.push_revision(
                                        LifecycleRevisionKind::CenterUpgraded { from: old_key },
                                        as_of,
                                        None,
                                    );
                                    delta.record(claim_revision);
                                }
                            }
                        }
                    }
                }
            }
            // 第 2/3 步：倒退守卫（per-identity last_as_of；本 prefix 新建 entry 恒通过——
            // last_as_of = as_of，守卫不触发）+ 终态吸收（禁复活）。两者由内核准入一次给出。
            match self.ledger.admit(&key, as_of) {
                // 零 revision、entry 零改动（显式拒绝 + 注记，非静默吸收——#78 修复 2）。
                LedgerAdmission::RetrogradeRejected => continue,
                // 门卫钟已前移，但终态不复活 ⟹ 零输出（E2E §1:83）。
                LedgerAdmission::TerminalAbsorbed => continue,
                LedgerAdmission::Accepted => {}
            }
            let entry = self.ledger.get_mut(&key).expect("entry 已建仓");
            // 第 4 步：力度求值（事件通道恒 Verified；活窗三值化）。
            let force = obs.force(material);
            // 第 5 步：first_provable 首次写入（仅 Verified(true)——#78 核验 T14 锚定；
            // Unavailable 不写 first_provable）。
            if force == ForceCheck::Verified(true)
                && first_write_clock(&mut entry.first_provable_at, as_of)
            {
                let revision =
                    entry.push_revision(LifecycleRevisionKind::FirstProvable, as_of, None);
                delta.record(revision);
            }
            // 第 6 步：反超判负 / Unavailable 审计注记。反超只要求曾可证；结构完成不是
            // 前置条件，因此这里合法产生 Provisional→ForceOvertake 第二终局路径（Q3）。
            match force {
                ForceCheck::Verified(false) => {
                    if entry.first_provable_at.is_some() {
                        // 反超（061:26）：曾可证 ∧ 当前不再弱 ⟹ Invalidated(ForceOvertake)。
                        let evidence = obs.force_evidence(material);
                        let revision =
                            entry.invalidate(InvalidatedReason::ForceOvertake, as_of, evidence);
                        delta.record(revision);
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
                        delta.record(revision);
                    }
                    continue;
                }
                ForceCheck::Verified(true) => {}
            }
            // 第 7 步：完成时复核（024:24——同一谓词在完成窗上重算为真才结算；禁
            // 「曾经弱过」冒充，E2E §4.1:151；复核用本 prefix 现算 force，不沿用旧值）。
            if obs.structure_completed() {
                if first_write_clock(&mut entry.structure_end_at, as_of) {
                    let revision = entry.push_revision(
                        LifecycleRevisionKind::StructureCompleted,
                        as_of,
                        None,
                    );
                    delta.record(revision);
                }
                if entry.first_provable_at.is_some() && force == ForceCheck::Verified(true) {
                    let revision = entry.confirm(as_of);
                    delta.record(revision);
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
                    let revision =
                        entry.invalidate(InvalidatedReason::NeverConstituted, as_of, evidence);
                    delta.record(revision);
                }
                // 完成观察已反超者已被第 6 步接住（Invalidated(ForceOvertake) 而非
                // Confirmed，confirmed_at 保持 None，T7(b) 锁定）；若是此前活窗先反超，
                // 则合法绕过 StructureCompleted——两条否证路径不互相冒充。
            }
        }
        // 第 8 步：身份消失扫描（上一 prefix 有、本 prefix 不再产出该 key ⟹ Invalidated；
        // 倒退 prefix 不制造 IdentityVanished——last_as_of > as_of 的身份跳过，#78 修复 2）。
        let vanished: Vec<LifecycleKey> = self
            .ledger
            .entries()
            .filter(|(key, entry)| {
                entry.state == NestEventState::Provisional
                    && entry.last_as_of <= as_of
                    && !seen.contains(key)
            })
            .map(|(key, _)| *key)
            .collect();
        for key in vanished {
            // 成因两分（票 #559 裁定，唯一分类点）：本 prefix 观察集合里若仍有同锚身份
            // （必然 c 左端不同——同左端已被桥吸收成 Supersedes，不进本扫描），则该锚的
            // 假设未死，是 provider 换轨丢下旧 key ⟹ 观测接缝伪影；否则该锚本 prefix
            // 完全不再产窗 ⟹ 假设被推翻。判据只读本 prefix 的 `seen`，不做历史重建。
            let cause = seen
                .iter()
                .find(|other| **other != key && same_anchor(other, &key))
                .map(|successor| VanishCause::ObservationSeam {
                    successor_c_start: successor.seg_c_full.0,
                })
                .unwrap_or(VanishCause::HypothesisRefuted);
            let entry = self.ledger.get_mut(&key).expect("扫描键存在");
            // 证据传 None：IdentityVanished 无力度语义（invariant 逐条钉死）。
            let revision =
                entry.invalidate(InvalidatedReason::IdentityVanished { cause }, as_of, None);
            delta.record(revision);
        }
        // 库内 debug 构建自动核验；release 不作“自动核验”声明。生产 p123 接线在每个
        // trigger 喂入后显式调用本公开入口。
        #[cfg(debug_assertions)]
        self.assert_invariants();
        delta.into_vec()
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
        // 账本级骨架（票 #573 T1，对象无关）：注册表键一致、计数 == 留档、出生钟 ≤ 门卫钟、
        // 终态 ⟺ 终态钟、留档时序非降、首条修订为建项词汇、来源链不自环。域断言在其之上
        // 逐条叠加（两层互不替代——下面全部既有域断言语义一条不改）。
        self.ledger.assert_core_invariants();
        for entry in self.ledger.values() {
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
            // 票 #559 新不变量（全态覆盖）：`vanish_cause` 有值 ⟺ 原因码是 IdentityVanished。
            // 「凡消失必带可审计两类原因码」的账本级钉死点；其它终局/活假设恒无成因字段。
            assert_eq!(
                entry.vanish_cause.is_some(),
                matches!(
                    entry.invalidated_reason,
                    Some(InvalidatedReason::IdentityVanished { .. })
                ),
                "vanish_cause 有值 ⟺ IdentityVanished：{key:?}"
            );
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
                    let reason = entry.invalidated_reason.expect("Invalidated 必有原因码");
                    match reason {
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
                        InvalidatedReason::IdentityVanished { cause } => {
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
                            // 票 #559 新不变量：凡消失必带可审计成因，且账本字段与原因码
                            // 载荷逐位一致（单一写入点 `invalidate` 的结构性保证）。
                            assert_eq!(
                                entry.vanish_cause,
                                Some(cause),
                                "身份消失成因入账本字段且与原因码载荷一致：{key:?}"
                            );
                            if let VanishCause::ObservationSeam { successor_c_start } = cause {
                                assert_ne!(
                                    successor_c_start, key.seg_c_full.0,
                                    "接缝成因的接手身份 c 左端必异于消失身份（同左端应被桥吸收）：{key:?}"
                                );
                            }
                        }
                    }
                }
            }
            if let Some(from) = entry.superseded_from {
                assert!(from != key, "身份来源链不自环：{key:?}");
                // 两码互斥且穷尽（票 #603 档 1）：桥迁移（`Supersedes`，B 相等）或中枢升级认领
                // （`CenterUpgraded`，B 严格更晚）——`bridge_identity` 与
                // `bridge_by_center_upgrade` 在 `b_center_start` 上互斥（相等 vs 严格小于）。
                assert!(
                    bridge_identity(&from, &key) || bridge_by_center_upgrade(&from, &key),
                    "身份来源链两端满足桥身份或中枢升级认领：{key:?}"
                );
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
            // 完成钟两分单调（票 #527，#559 C3 订正）：物理完成 ≤ 账本收到。二者恒等不是
            // 要求，恒序才是——违序 ⟹ provider 回填/前视，停线。
            assert!(
                signal.completed_at <= signal.as_of,
                "完成钟两分单调 completed ≤ 账本：{:?}",
                signal.key
            );
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
    fn register_completion_signal(
        &mut self,
        key: LifecycleKey,
        as_of: usize,
        completed_lower_id: ElementId,
        completed_at: usize,
    ) -> bool {
        if self.completion_signal_seen(&key)
            || self
                .bridge_entry(&key)
                .is_some_and(|entry| as_of < entry.last_as_of)
        {
            return false;
        }
        self.completion_signals.push(CompletionSignal {
            key,
            as_of,
            completed_lower_id,
            completed_at,
        });
        true
    }

    fn bridge_entry(&self, key: &LifecycleKey) -> Option<&NestLifecycleEntry> {
        match self.ledger.get(key) {
            Some(entry) => Some(entry),
            None => self.bridge_match(key).and_then(|old| self.ledger.get(&old)),
        }
    }

    /// 白名单桥匹配：除 seg_c 右端外全等的既有键（同身份不同右端的键在 book 内至多一只，
    /// 迁移即替换 ⟹ 匹配唯一）。
    fn bridge_match(&self, key: &LifecycleKey) -> Option<LifecycleKey> {
        self.ledger
            .keys()
            .find(|old| **old != *key && bridge_identity(old, key))
            .copied()
    }

    /// 身份迁移：把 `old_key` 的整条 entry 原样搬到 `new_key` 下，写当下来源指针 + 追加一条
    /// 来源修订。**全 book 仅有的两个迁移点收敛于此**（票 #619 L12），搬运本身由内核
    /// [`LedgerBook::migrate`] 独占执行（票 #573 T1：旧键退表 → 改键 → 来源链留痕 →
    /// 追加迁移修订 → 新键入表）。
    ///
    /// 「钟一个 bit 不动」由实现形态保证——entry 整体搬走，五钟不经任何赋值。本方法只负责
    /// **域侧的来源码 → 修订词汇映射**（见 [`MigrationKind`]）：`from` 恒取 `old_key`
    /// （被退表的那个键），调用方不参与构造 ⟹「写来源指针」与「写哪种来源修订」不可能不一致。
    fn migrate_entry(
        &mut self,
        old_key: LifecycleKey,
        new_key: LifecycleKey,
        migration: MigrationKind,
        as_of: usize,
    ) -> LifecycleRevision {
        let kind = match migration {
            MigrationKind::Bridge => LifecycleRevisionKind::Supersedes { from: old_key },
            MigrationKind::CenterUpgrade => LifecycleRevisionKind::CenterUpgraded { from: old_key },
        };
        self.ledger.migrate(old_key, new_key, kind, as_of)
    }

    /// 中枢升级认领匹配（票 #603 档 1）：同锚（`seg_a` + C 左端全等）且 B 严格更晚的既有键，
    /// 连同「该前身此刻可否迁移」一并给出（`Provisional` ∧ 非倒退喂入）。
    ///
    /// **只服务 `advance` 第 1 步的建仓分支**——`bridge_entry`/`terminal_bridge_hit`/
    /// `completion_signal_seen` 一律仍走 [`bridge_identity`]（严格更强），本方法不参与那三处
    /// 判定：认领是「两个身份之间的关联」，不是「它们是同一个身份」，把它塞进桥语义会让终态
    /// 前身把新身份一并吸收（禁复活的适用面被误扩），那正是本设计要避免的。
    ///
    /// ## 多候选语义（票 #619 H2 订正）
    ///
    /// **可匹配者可以有多只**：认领留痕形态**恰恰把终态前身留在册**（那是它的设计要点），
    /// 于是同一条链上可同时存在「已终态的 P1（B 较早）」与「留痕后新建、仍 `Provisional` 的
    /// P2（B 居中）」；第三次 B 升级到达时两只都满足 [`bridge_by_center_upgrade`]。
    /// 旧口径「同链至多一只**存活**」把「存活」当成了「匹配」，论证不成立——已订正。
    ///
    /// **优选规则**（确定性，无平局歧义）：
    /// 1. 首选 `BTreeMap` 序首个**可迁移**者（`Provisional` ∧ `as_of ≥ last_as_of`）；
    /// 2. 无可迁移者时退回 `BTreeMap` 序首个匹配者，走认领留痕。
    ///
    /// 为什么不能按 [`LifecycleKey`] 序盲取：`sort_tuple`（见 :134）在 `b_center_start` **之前**
    /// 先比 `seg_c_full`，而 C 右端随 `as_of` 漂移、**与前身死活完全无关** ⟹ 盲取会以「谁的 C
    /// 右端更小」决定要不要迁移。若盲选到终态前身，本该被迁移的那只**存活**前身不被迁移 ⟹
    /// 五钟不继承、该 entry 沦为孤儿 ⟹ 后续走 `IdentityVanished`（凭空多一条身份消失）。
    fn center_upgrade_match(
        &self,
        key: &LifecycleKey,
        as_of: usize,
    ) -> (Option<LifecycleKey>, bool) {
        let mut fallback = None;
        for (old_key, old) in self.ledger.entries() {
            if !bridge_by_center_upgrade(old_key, key) {
                continue;
            }
            if old.state == NestEventState::Provisional && as_of >= old.last_as_of {
                return (Some(*old_key), true);
            }
            fallback.get_or_insert(*old_key);
        }
        (fallback, false)
    }
}

/// pan 活窗产出机（卡 §3：中枢 + seg_a 定位沿用 `locate_pan_div_structure`——窄锚优先、
/// A′ 回退与 provider 同序同判（level_view.rs:826-839）；Extreme 预滤沿用
/// `pan_div_structure_extreme`，禁第二查法）。
///
/// 对每只 Consolidation 中枢取**末个**可定位离开段的结构锚，活窗 = `(seg_c.0, as_of)`
/// （c 窗右端 = prefix 边界，含行进中 bar）。本函数只做定位与产窗，不消费力度
/// （力度三值化在 advance 内现算）。#421 现由 `p123_fast_replay` 在生产每根 bar 调用
/// `feed_replay_bar`（活窗相先、完成相后）；本函数是其 `PanLiveWindow` 结构定位与初建的
/// 参照实装（T11/T14 真实夹具锚定）。
///
/// ★#523 根因登记 / 票 #527 分水岭：本函数的 C 取自**已完成** lower legs，与完成事件
/// provider 同源同判 ⟹ 它产的窗最早只能在完成候选也已可构造时出生（首见即完成）。
/// **生产 L1 活窗已改由 [`provide_active_pan_live_windows`] 从 active C frontier 产**；
/// 本函数保留为结构定位参照与 p409 反事实探针（`post-completion holding counterfactual`）
/// 的产窗口径，**不再**是生产「完成前活窗」的来源。
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
                // 本函数的 C 恒取自已完成 segments（非 active frontier），无洞概念，恒 0。
                gap_len: 0,
            },
        );
    }
    latest.into_values().collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// L1 active C frontier（票 #527：完成前活窗可见）
// ═══════════════════════════════════════════════════════════════════════════

/// L1 的**行进中 C 段**（parser 未确认线段的当下状态；票 #527 / #523 §3 L1）。
///
/// 数据源纪律（票面永禁清单）：本载体只能由 parser 的 `tail`（`OpenTail.pendingSegment`）
/// + 其对应的未确认笔序列构造——**禁止**从 confirmed segments 回放重建。它表达
/// 「当下状态」（方向/起点/当前极值，parser/tail.rs 逐字口径），不预判该段将如何终结。
///
/// 与 confirmed 段的关系：`start_index` = pending 段首笔起点（= 上一 confirmed 段终点），
/// 一旦 parser 把该段 emit 进 `ParseLayer.segments`，本 frontier 即消失并由完成事件接手
/// （同一 `c_start` ⟹ 桥判同身份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSegmentFrontier {
    /// 未确认段方向（= 剩余笔首笔方向，第67课线段方向口径）。
    pub direction: Direction,
    /// 未确认段起点源坐标。
    pub start_index: usize,
    /// 未确认段起点价（剩余笔首笔起点价）。
    pub start_price: Tick,
    /// 段方向上当下触及的极值（与 `parser::tail` 的 `current_extreme` 同口径）。
    pub extreme: Tick,
    /// 该极值所在的**结构点**源坐标（极值所在笔端点；禁用 as_of 冒充结构点）。
    pub extreme_at: usize,
}

impl ActiveSegmentFrontier {
    /// 行进中 C 段的当下段形态（右端 = 极值结构点，不是 as_of——as_of 只进活窗右端）。
    pub fn as_segment(&self) -> Segment {
        Segment {
            direction: self.direction,
            start_index: self.start_index,
            end_index: self.extreme_at,
            start_price: self.start_price,
            end_price: self.extreme,
        }
    }
}

/// 从 parser 单层输出读出 L1 的 active C frontier（唯一构造点）。
///
/// 口径与 `parser::tail::pending_segment` 同源：pending 段 = `strokes[pending_start..]`，
/// 方向 = 首笔方向，极值 = 剩余笔在段方向上的极值。本函数额外解析**极值所在结构点**
/// （`current_extreme` 不携带坐标——tail 只存当下状态值），用于给行进中段一个真实右端。
///
/// 诚实边界：`tail` 无 `PendingSegment` / pending 首笔无法在 `strokes` 上定位 / 极值退化在
/// 段起点（该段尚未推进）⟹ `None`（无行进中 C 可定位，不造窗）。
pub fn active_segment_frontier(l0: &ParseLayer) -> Option<ActiveSegmentFrontier> {
    let (direction, start_index) = l0.tail.iter().find_map(|pending| match pending {
        PendingTail::PendingSegment {
            direction,
            start_index,
            ..
        } => Some((*direction, *start_index)),
        _ => None,
    })?;
    // pending 首笔定位（strokes 按 start_index 升序；坐标不等 ⟹ 定位失败，诚实 None）。
    let pending_start = l0.strokes.partition_point(|s| s.start_index < start_index);
    let rest = l0.strokes.get(pending_start..)?;
    let first = rest.first()?;
    if first.start_index != start_index {
        return None;
    }
    // 极值与其结构点：逐笔取两端，方向上取极值；平局保最早坐标（parser :16 同 discipline）。
    let mut extreme = first.start_price;
    let mut extreme_at = first.start_index;
    for stroke in rest {
        for (price, at) in [
            (stroke.start_price, stroke.start_index),
            (stroke.end_price, stroke.end_index),
        ] {
            let better = match direction {
                Direction::Up => price > extreme,
                Direction::Down => price < extreme,
            };
            if better {
                extreme = price;
                extreme_at = at;
            }
        }
    }
    if extreme_at <= start_index {
        return None; // 段起点即极值 ⟹ 行进中段尚未推进，无可定位的 C。
    }
    Some(ActiveSegmentFrontier {
        direction,
        start_index,
        start_price: first.start_price,
        extreme,
        extreme_at,
    })
}

/// 活窗产出机（票 #527 立、票 #601 泛化）：**confirmed A/B 锚 + active C 腿**。
///
/// 与 [`provide_pan_live_windows`] 的分水岭（#523 根因）：后者遍历**已完成** lower legs 找 C，
/// 故最早只能在完成候选也已可构造时出生（首见即完成）；本函数的 C **只能**是行进中的下级腿
/// （`active`），A 锚与 B 中枢仍取 confirmed 侧 ⟹ 活窗在完成前即可见。
///
/// **级别中立**（票 #601）：本函数从不读 `level` 以外的级别信息，判据全部作用在传入的
/// `centers`/`kinds`/`confirmed_segments`/`active` 四元组上——`level` 只随窗口带出，不参与判定。
/// 故 L1 与 L2 共用同一实装（禁第二查法）：
/// - **L1**：`active` = parser 行进中段（[`active_segment_frontier`] → [`ActiveSegmentFrontier::as_segment`]），
///   `confirmed_segments` = `tower[0]` 投影；
/// - **L2**：`active` = 行进中的 L1 窗口单元（[`active_l1_window_frontier`] →
///   [`ActiveWindowFrontier::as_segment`]），`confirmed_segments` = `tower[1]` 投影。
///
/// 调用方**必须**保证 `active` 是塔上尚不存在的行进中腿——从已完成 tower unit 外推会重演
/// #523 的同源恒等式（该守卫在两个 frontier 构造函数内，不在本函数内；本函数不校验来源）。
///
/// 结构判据一律复用同一套单一来源（禁第二查法）：`nearest_confirmed_center_idx` 取 B、
/// `locate_pan_div_structure`（窄锚）→ `locate_pan_div_structure_front_anchor`（A′ 回退）、
/// `pan_div_structure_extreme` 预滤——与完成事件 provider（level_view.rs pan 分支）同序同判。
///
/// **措辞限定**（票 #591 裁定）：「同序同判」指**同一输入序列 ⟹ 同一判定**——两路径调用的
/// `nearest_confirmed_center_idx` 是同一纯函数、逐字相同的切片构造算法。这**不**蕴含「任一
/// 时刻两路径的查询结果一致」：活窗路径按当前全局唯一 pending frontier 稀疏采样（仅在
/// `(forest_epoch, frontier)` 变化时重算），完成路径在结构已完全确认后对已固定的
/// `segment.start_index` 做一次性事后查询——中枢确认可落在活窗路径的稀疏重算盲区内，
/// 此时两路径在**同一 bar** 查询同一纯函数会因**入参切片不同步**（活窗侧尚未看见刚确认的
/// 中枢）而给出不同结果，这是时序差（无害），不是判据分歧（32893 现场分析）。
///
/// **c_start 稳定性**（#523 遗留问题 2；票 #559 条件 C2 订正——原文写成无限定的恒等断言，
/// BTC 100k 实测 5 例反例，故收窄为下述限定表述）：
///
/// 限定成立的是「**同一** C 段」上的恒等：活窗左端 = `structure.seg_c.0` = λ_C，由
/// `departure_move_c_start` 在 `[B.end_index, C.start_index]` 窗口上定界；该窗口只含衔接连续的
/// confirmed 段与行进中段自身（衔接由上面的守卫钉死），行进中段 emit 为 confirmed 后
/// `start_index`/`direction` 不变 ⟹ 同一 λ_C ⟹ **该 C 段的**完成事件与**该 C 段的**活窗
/// `seg_c.0` 恒等，桥（除右端外全等）判同身份。
///
/// **不成立的是跨 C 段的恒等**（反例来源）：完成事件的首次可见 bar 晚于 lower unit 物理完成
/// bar（BTC 100k 滞后中位 57.5、最大 5190）。若该滞后超过 C 段活窗的存活期，完成事件到账时
/// parser 的 pending 段早已换成**下一个** C ⟹ 当下活窗的 `seg_c.0` 与到账完成事件的
/// `seg_c.0` 属于两个不同的 C，桥不判同身份 ⟹ 旧活窗记
/// `IdentityVanished{ObservationSeam}`、完成事件另起闪现。这是**观测接缝**，不是 λ_C 不稳；
/// 两类成因的账本区分见 [`VanishCause`]（票 #559 裁定）。
///
/// 活窗右端 = `as_of`（卡 §3 设计内行为，随 bar 前进；不进身份键）。
///
/// 返回 [`PanLiveOutcome`]——**未产窗时给出可审计的原因码**（验收 1 第二分支要求「明确、
/// 可审计」；诊断只写不判，不进任何真值路径）。
pub fn provide_active_pan_live_windows(
    level: u32,
    centers: &[Center],
    kinds: &[Option<MoveKind>],
    confirmed_segments: &[Segment],
    active: Segment,
    as_of: usize,
) -> PanLiveOutcome {
    // 诊断字段（票 #592）：frontier 起点与最近 confirmed 段末端的洞长——与下方
    // `frontier_not_after_confirmed` 判定同源同值（同一个 `confirmed_segments.last()`）。
    // 无 confirmed 段（B 锚尚未建立）时无洞可定义，记 0；产窗必经 `NoConfirmedCenterBefore`
    // 分支拒绝，该 0 值不会流入下游 Window 载荷。
    let gap_len = confirmed_segments
        .last()
        .map_or(0, |last| active.start_index.saturating_sub(last.end_index));
    // 行进中段必须严格晚于全部 confirmed 段（否则不是 frontier ⟹ 拒绝，诚实空产出）。
    //
    // **仍只查 `<` 不查 `==`**（票 #578 复核，推翻本注记曾经的「`>` 分支生产不可达」断言）：
    // 票 #559 条件 C2 曾主张「parser 笔首尾相接 ⟹ 末段 end_index == frontier.start_index 恒真，
    // `>` 分支不可达」，并引用「BTC 100k 实测 frontier_gap 分布逐 bar 恒 0」为据——但该数字
    // **从未由任何真做 `!=`/`>` 判别的探针实际测过**：旧夹具坐标（段间 +1 不共端点）令 `==`
    // 守卫在测试里恒拒，从来没人能在不改夹具的前提下把 `>` 分支接上真实数据跑一遍。
    // 票 #578 把夹具坐标对齐生产共端点约定后，**首次**具备条件真做这个探针：改 `<`→`!=`
    // 编译通过、单测全绿后，用同一 BTC 数据跑 p123 20k bar 对拍，`frontier_not_after_confirmed`
    // 从 0 跳到 **210**（`window` 命中同时从 101 降到 58）——即 `active.start_index >
    // last.end_index`（有洞，非倒灌）在真实数据里频繁发生，C2 的「恒可达」断言是未经验证的
    // 声明膨胀（090 号语法禁令）。故**不收紧**本守卫：`<`→`!=` 会真实拒绝当下被接受的活窗，
    // 不是生产零行为变化，是否应该拒绝这些「有洞」frontier 需要教义/架构裁决，非本票 Scope
    // （见 §7 遗留 6）。
    if confirmed_segments
        .last()
        .is_some_and(|last| active.start_index < last.end_index)
    {
        return PanLiveOutcome::FrontierNotAfterConfirmed;
    }
    if active.end_index > as_of {
        return PanLiveOutcome::FrontierAheadOfClock; // 禁前视：结构点尚未到达当前 bar。
    }
    // 与完成时同构：行进中段并入段序列尾（λ_C/包络与完成后同一算式，见函数文档）。
    let mut segments = confirmed_segments.to_vec();
    segments.push(active);
    let anchors_self: Vec<Option<Direction>> = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    let Some(center_index) = nearest_confirmed_center_idx(centers, active.start_index) else {
        return PanLiveOutcome::NoConfirmedCenterBefore;
    };
    if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
        return PanLiveOutcome::CenterNotConsolidation;
    }
    let Some(structure) = locate_pan_div_structure(
        &centers[center_index],
        &active,
        &segments,
        &anchors_self,
    )
    .filter(|structure| pan_div_structure_extreme(structure, &segments))
    .or_else(|| {
        locate_pan_div_structure_front_anchor(
            &centers[center_index],
            &active,
            &segments,
            &anchors_self,
        )
        .filter(|structure| pan_div_structure_extreme(structure, &segments))
    }) else {
        return PanLiveOutcome::StructureNotLocatable;
    };
    PanLiveOutcome::Window(PanLiveWindow {
        level,
        side: structure.side,
        seg_a: structure.seg_a,
        seg_c_live: (structure.seg_c.0, as_of.max(structure.seg_c.0)),
        b_center_start: centers[center_index].start_index,
        gap_len,
    })
}

/// L1 活窗定位的结果与**未产窗原因码**（票 #527；诊断面，不进真值路径）。
///
/// 原因码回答「这只完成身份为什么没有更早的 Live」——闪现若非 true-flash，必须能落到
/// 其中某一码上，禁以「不知道」结账。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanLiveOutcome {
    /// 定位成功：行进中 C 的活窗。
    Window(PanLiveWindow),
    /// 行进中段与 confirmed 段序不自洽（frontier 起点落在末 confirmed 段内部）。
    FrontierNotAfterConfirmed,
    /// 行进中段的极值结构点尚未到达当前 bar（禁前视）。
    FrontierAheadOfClock,
    /// 行进中段之前没有任何已确认中枢（B 锚不可见）。
    NoConfirmedCenterBefore,
    /// 最近已确认中枢不属 Consolidation 块（盘整域外）。
    CenterNotConsolidation,
    /// 窄锚与 A′ 回退都无法定位 A/C 结构，或 Extreme 预滤未过（C 尚未破 A 极值）。
    StructureNotLocatable,
}

impl PanLiveOutcome {
    /// 原因码标签（dump/统计口径单一来源）。
    pub fn reason_tag(&self) -> &'static str {
        match self {
            Self::Window(_) => "window",
            Self::FrontierNotAfterConfirmed => "frontier_not_after_confirmed",
            Self::FrontierAheadOfClock => "frontier_ahead_of_clock",
            Self::NoConfirmedCenterBefore => "no_confirmed_center_before",
            Self::CenterNotConsolidation => "center_not_consolidation",
            Self::StructureNotLocatable => "structure_not_locatable",
        }
    }

    pub fn window(&self) -> Option<PanLiveWindow> {
        match self {
            Self::Window(window) => Some(*window),
            _ => None,
        }
    }
}

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

/// [`active_l1_window_frontier`] 的结果与**未产出原因码**（票 #601；诊断面，不进真值路径）。
///
/// 原因码回答「这只 L2 完成身份为什么没有更早的 Live」——L2 特有的缺口在此照实命名，
/// 不并入 L1 的 [`PanLiveOutcome`] 码表（两者定位的是不同层的失败：本枚举失败在
/// 「L1 层根本没有行进中单元可作 C」，`PanLiveOutcome` 失败在「有 C 但 A/B 结构定位不成」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWindowOutcome {
    /// 派生成功：行进中的 L1 窗口单元。
    Frontier(ActiveWindowFrontier),
    /// L0 无行进中段（parser 无 `PendingSegment`）⟹ 无可虚拟追加的单元。
    NoLowerFrontier,
    /// 行进中 L0 段与 `tower[0]` 单元序不自洽（起点落在末单元内部）⟹ 拒绝，诚实空产出。
    LowerFrontierNotAfterUnits,
    /// L1 层扫描断点越过 `tower[0]` 单元数（塔与 units 不同步）⟹ 拒绝重扫，不猜锚。
    ResumeAnchorOutOfRange,
    /// 虚拟追加后自断点起重扫仍无任何窗口成立（seed 判据不过）。
    NoWindowFormed,
    /// 重扫末窗**不含**行进中 L0 段 ⟹ 该窗与 `tower[1]` 已有窗口同源，拒绝外推（#523 红线）。
    LowerFrontierNotAbsorbed,
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
        }
    }

    pub fn frontier(&self) -> Option<ActiveWindowFrontier> {
        match self {
            Self::Frontier(frontier) => Some(*frontier),
            _ => None,
        }
    }
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
) -> ActiveWindowOutcome {
    let Some(frontier) = l0_frontier else {
        return ActiveWindowOutcome::NoLowerFrontier;
    };
    // 衔接守卫与 L1 活窗同口径（`provide_active_pan_live_windows` 的
    // `frontier_not_after_confirmed`）：只拒**倒灌**（起点落在末单元内部），不拒「有洞」
    // ——#578 复核已证「有洞」在真实数据里频繁发生，收紧属教义裁决，非本票 Scope。
    if l0_units
        .last()
        .is_some_and(|last| frontier.start_index < last.end_index)
    {
        return ActiveWindowOutcome::LowerFrontierNotAfterUnits;
    }
    if l1_resume_from > l0_units.len() {
        return ActiveWindowOutcome::ResumeAnchorOutOfRange;
    }
    // 虚拟追加单元：口径与 `classifier::segment_to_unit` 逐字相同（lo/hi 按端点价取序，
    // 不假设方向与价序一致）。右端 = 极值结构点（`ActiveSegmentFrontier` 已禁 as_of 冒充）。
    let mut units = l0_units.to_vec();
    units.push(UnitRange {
        start_index: frontier.start_index,
        end_index: frontier.extreme_at,
        direction: frontier.direction,
        lo: frontier.start_price.min(frontier.extreme),
        hi: frontier.start_price.max(frontier.extreme),
    });
    let (windowed, _metas, _cursor) =
        detect_centers_windowed_resume(&units, center_from_segments, l1_resume_from);
    let Some((center, win)) = windowed.last().copied() else {
        return ActiveWindowOutcome::NoWindowFormed;
    };
    // 末窗必须含虚拟追加单元（⟺ 扫描因数据耗尽而停 ⟺ 窗口仍开放）；否则该窗由纯 confirmed
    // 单元构成，与 `tower[1]` 已有窗口同源 ⟹ 拒绝外推（#523 红线，见结构体文档）。
    if win.1 + 1 != units.len() {
        return ActiveWindowOutcome::LowerFrontierNotAbsorbed;
    }
    ActiveWindowOutcome::Frontier(ActiveWindowFrontier {
        direction: units[win.0].direction,
        start_index: units[win.0].start_index,
        end_index: units[win.1].end_index,
        // `center.dd/gg` = 窗口全单元外缘聚合（detect 的 seed 三段交 + 延伸 min/max），
        // 与该窗若 compose 后的 `rmove.lo()/hi()` 逐值相等（`LeveledMove::envelope` 投影契约）。
        lo: center.dd,
        hi: center.gg,
    })
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

/// 把同源 provider 的逐 run 结构展开为当前边界的活窗。
///
/// 本函数是旧 trigger/provider 适配层的展开原语。生产逐 bar sidecar 在 p123 内按 p409
/// 同构机制独立发现结构窗；后续 bar 保持身份字段不动，仅延展 `seg_c_live.1`。
pub fn provide_replay_live_windows(
    runs: &[PanLiveRun<'_>],
    as_of: usize,
) -> Vec<PanLiveWindow> {
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
        assert!(old.force_evidence.is_none(), "IdentityVanished 恒无力度证据");
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
            &[LifecycleObservation::pan_live(pan_window((50, 69), 70, 89), false)],
            89,
            &m,
        );
        assert_eq!(d.len(), 1, "仅 Observed");
        // as_of=99：同锚（seg_a 不变），C 左端换成 90 ⟹ 桥不判同 ⟹ 旧 key 消失。
        let d = book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 69), 90, 99), false)],
            99,
            &m,
        );
        assert_eq!(d.len(), 2, "新 C 身份 Observed + 旧 C 身份 IdentityVanished");
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
            LifecycleRevisionKind::Invalidated {
                reason: InvalidatedReason::IdentityVanished { .. }
            }
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
            Segment { direction: Direction::Up, start_index: 59, end_index: 69, start_price: 96, end_price: 104 },
            Segment { direction: Direction::Down, start_index: 69, end_index: 79, start_price: 103, end_price: 93 },
            Segment { direction: Direction::Up, start_index: 79, end_index: 89, start_price: 94, end_price: 99 },
            Segment { direction: Direction::Down, start_index: 89, end_index: 99, start_price: 98, end_price: 92 },
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

        // as_of=79：真实定位产窗（窄锚 A=(50,59)、c_start=69 同锚），三通道成立
        // （5<20、1<5、0.5<2）⟹ first_provable=79。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 79);
        assert_eq!(windows.len(), 1, "单一身份活窗");
        assert_eq!(windows[0].seg_a, (50, 59), "窄锚 A 经真实 locate_pan_div_structure 锚定");
        assert_eq!(windows[0].seg_c_live, (69, 79), "c_start_live 与完成后 seg_c.0 同锚（卡 §3）");
        let obs: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        book.advance(&obs, 79, &m);
        assert_eq!(
            book.get(&key_pan((50, 59), (69, 79))).unwrap().first_provable_at,
            Some(79)
        );

        // as_of=99：活窗延展，真实现算三通道全假 ⟹ Invalidated(ForceOvertake) 可审计。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 99);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].seg_c_live, (69, 99), "活窗右端随 as_of 前进（设计内行为）");
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
        // 终态吸收（含桥匹配）：as_of=109 活窗延展 (69,109) 仍零输出。
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
        let key79 = key_pan((50, 59), (69, 79));
        let e = book.get(&key79).unwrap();
        assert_eq!(e.state, NestEventState::Provisional, "「不可验」≠「不再弱」");
        assert_eq!(e.first_provable_at, None, "Unavailable 不写 first_provable");
        assert_eq!(e.invalidated_at, None, "Unavailable 不判 Invalidated");

        // 同 as_of 注记幂等：重复 advance 零新 revision。
        let d = book.advance(&obs, 79, &no_force);
        assert!(d.is_empty(), "同 as_of 注记幂等去重");
        assert_eq!(book.get(&key79).unwrap().revisions.len(), 2, "Observed + 一条注记");

        // as_of=89：坐标映射失败子场景（close_src 截断到 69 之前 ⟹ c 窗映射失败）
        // ⟹ CoordinateMapFailed 注记，仍不判 Invalidated。
        let windows = provide_pan_live_windows(1, &centers, &kinds, &segments, &anchors, 89);
        let obs89: Vec<_> = windows.iter().map(|w| LifecycleObservation::pan_live(*w, false)).collect();
        let short_src = identity_close_src(69);
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
        feed_replay_bar(book, &ReplayBarFeed { as_of, phases: &phases }, material)
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
            let delta = feed_prefix_phases(&mut book, &runs, &events, 99, &m)
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
        assert_eq!(terminal.structure_end_at, None, "Q3 第二终局路径不经结构完成");

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
        assert_eq!(frontier.extreme, 92, "与 parser tail 的 current_extreme 同口径");
        assert_eq!(frontier.extreme_at, 95, "极值结构点 = 极值所在笔端点，非 as_of");
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
        let outcome =
            provide_active_pan_live_windows(1, &centers, &kinds, confirmed, frontier.as_segment(), 95);
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
            provide_active_pan_live_windows(1, &centers, &kinds, confirmed, frontier.as_segment(), 94),
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
        let gapped_window =
            provide_active_pan_live_windows(1, &centers, &kinds, confirmed, gapped.as_segment(), 95)
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
        let window =
            provide_active_pan_live_windows(1, &centers, &kinds, &segments[..4], frontier.as_segment(), 95)
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
        let outcome = active_l1_window_frontier(&units, Some(&l0_frontier), 0);
        let frontier = outcome.frontier().expect("行进中 L0 段被吸收 ⟹ 有行进中 L1 单元");
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
        let outcome = active_l1_window_frontier(&units, Some(&l0_frontier), 0);
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
        let none = active_l1_window_frontier(&units, None, 0);
        assert_eq!(none, ActiveWindowOutcome::NoLowerFrontier);
        assert_eq!(none.reason_tag(), "no_lower_frontier");

        // (b) 倒灌：行进中段起点落在末 confirmed 单元内部。
        let backfill = ActiveSegmentFrontier {
            start_index: 25,
            ..l0_frontier
        };
        assert_eq!(
            active_l1_window_frontier(&units, Some(&backfill), 0),
            ActiveWindowOutcome::LowerFrontierNotAfterUnits
        );

        // (c) 重扫锚越过 units 长度（塔与 units 不同步）⟹ 不猜锚。
        assert_eq!(
            active_l1_window_frontier(&units, Some(&l0_frontier), units.len() + 1),
            ActiveWindowOutcome::ResumeAnchorOutOfRange
        );

        // (d) 虚拟追加后仍无窗口成立（方向不交替 ⟹ seed 判据不过）。
        let no_alternation = vec![units[0], units[0]];
        assert_eq!(
            active_l1_window_frontier(&no_alternation, Some(&l0_frontier), 0),
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 120), false)],
            120,
            &m,
        );
        assert_eq!(book.len(), 1);

        // 更近的中枢 b=30 确认 ⟹ 同 seg_a、同 C 左端、B 严格更晚 ⟹ 认领。
        let upgraded = PanLiveWindow {
            b_center_start: 30,
            ..pan_window((50, 59), 70, 129)
        };
        let delta = book.advance(
            &[LifecycleObservation::pan_live(upgraded, false)],
            129,
            &m,
        );
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
        assert_eq!(entry.superseded_from.map(|from| from.b_center_start), Some(20));
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );
        // 活窗延展 ⟹ 力度反超 ⟹ 前身进终态。
        hist[130..=139].fill(-3.0);
        dif[130..=139].fill(-6.0);
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), false)],
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
        let delta = book.advance(
            &[LifecycleObservation::pan_live(upgraded, false)],
            139,
            &m,
        );
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
        assert!(!bridge_by_center_upgrade(&base, &other_c), "C 左端不同不认领");
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
        assert!(bridge_by_center_upgrade(&base, &upgraded), "严格同锚 + B 单调前进 ⟹ 认领");

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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
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
            !delta
                .iter()
                .any(|revision| matches!(revision.kind, LifecycleRevisionKind::CenterUpgraded { .. })),
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 139), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
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
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 129), false)],
            129,
            &m,
        );

        // bar 130：力度反超 ⟹ 桥迁移后进终态，终态键 c=(70,130)。三通道须**同时**不衰减
        // （`segments_diverge_or` 是或关系），故单根强 bar 需压过 a=[50,59] 的整段面积。
        hist[130..=149].fill(-30.0);
        dif[130..=149].fill(-60.0);
        let m = material(&hist, &dif, &close_src);
        book.advance(
            &[LifecycleObservation::pan_live(pan_window((50, 59), 70, 130), false)],
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
