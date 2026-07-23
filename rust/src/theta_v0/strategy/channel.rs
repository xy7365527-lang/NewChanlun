//! #147 T3：出场通道 P1–P8 全互斥通道解释器（first-match + C0 兜底）。
//!
//! ## 契约（issue #147 字面边界）
//!
//! 通道谓词 P1..P8 **允许重叠**；解释器 first-match 互斥化：
//! ```text
//! C_1 = P_1
//! C_j = P_j ∧ ¬P_1 ∧ … ∧ ¬P_{j-1}    (j = 2..8)
//! C_0 = ⋀_{j=1}^8 ¬P_j                （兜底 Hold）
//! ```
//! **每声部每时刻恰命中一条通道、恰产出一枚显式裁决**——Hold 也是显式裁决
//! （[`ChannelDecision::Exit`]([`ExitType::Hold`])），不存在无裁决时刻。
//!
//! ## 槽位定型（排序冻结；#149 只填 P4/P5，不动字段顺序/优先级）
//!
//! 票面优先级链「风险强平 ≻ 本级证书平仓（S2 二分 CloseRoot/ReduceCore）≻ 短差开启 ≻ 开仓
//! ≻ 记录/加仓 ≻ Hold」展开为 8 槽：
//!
//! | 槽 | 谓词 | 裁决 | 本票状态 |
//! |----|------|------|---------|
//! | P1 | 风险强平 | `Exit(RiskExit)` | 真实装（`force_flat` 注入，与 `KThetaRiskGate.force_flat` 同源） |
//! | P2 | 本级证书平仓·S2→根清仓 | `Exit(CloseRoot)` | 真实装（[`reverse_exit_type`] 单源） |
//! | P3 | 本级证书平仓·S2→减核心 | `Exit(ReduceCore)` | 真实装（同上） |
//! | P4 | 短差平仓 | `Exit(CloseShortDiff)` | #149 真实装（顺父 child cert + parent projection） |
//! | P5 | 短差开启 | `OpenShortDiff` | #149 真实装（反父 child cert + parent projection） |
//! | P6 | 开仓 | `Open` | 真实装 |
//! | P7 | 记录 | `Record` | 真实装（#150 T7：次级别完整走势检测器 + P7 记录桶，只写账本不动仓位） |
//! | P8 | 加仓 | `AddPosition` | **占位恒 false**（后续加仓票） |
//! | C0 | Hold 兜底 | `Exit(Hold)` | 真实装（显式裁决） |
//!
//! ## 单源复用（禁止镜像）
//!
//! - 出场类型 = [`ExitType`]（interp.rs 单源枚举，**不新建镜像枚举**）；裁决枚举
//!   [`ChannelDecision`] 经 `Exit(ExitType)` 内嵌复用，仅补三个非出场动作变体。
//! - S2 二分判据 = [`reverse_exit_type`]（G4/G5 单源函数，**不镜像判据**）：P2/P3 谓词由其
//!   返回值二分，测试交叉断言同源。
//! - 反向命中 = [`reverse_signal`]（§9 closePred 反向项）；候选序 = [`theta_key`]（≺_Θ 全序）。
//!
//! ## 非本票（字面边界外，不实现）
//!
//! - 祖先耦合（父声部终结级联清仓后代）= #148 活动集层，**不进** P1–P8。
//! - 记录/加仓谓词真值（#150）：P7/P8 槽位已定型，谓词仍恒 false。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! **L0 结构定理**：`Σ_{j=0}^8 1[C_j] = 1`（互斥 + 穷尽）是 first-match 构造的组合逻辑恒等式
//! （同 mutex.rs alpha2 Doc2§9 定理 1 构造，m=8 通道链实例）。Rust 穷举 2^8 验证 = L1 管线
//! 正确性；谓词经验触发率/盈利性 = L2 未覆盖，不声明。
//!
//! ## 与 mutex.rs 的关系（非重复）
//!
//! [`super::mutex`] 是 PDF §7 P1..P10 链（含 TW 谓词 P2–P4）的 **D1 测试 oracle**（零生产
//! 消费者）；本模块是 #147 出场通道链（P1–P8；#149 已填 P4/P5，#150 仍占 P7/P8）的
//! **声部级解释器**，
//! 两链谓词编号语义不同，各锚各的契约，不互为镜像。

use super::super::classifier::recursive_tower::ElementId;
use super::coverage::Vertical;
use super::exec::reverse_signal;
use super::interp::{
    reverse_exit_type, theta_key, trigger_projection_sound, ActiveLeg, Candidate, ExitType,
    ParentCertificateProjection,
};
use super::ledger::{SplitLegError, SplitLegEvent, SplitLegLedger};
use super::voice::{short_diff_side, VoiceSide};

/// 通道谓词数 m=8（P_1..P_8，issue #147 槽位定型）。
pub const CHANNEL_M: usize = 8;

/// 通道谓词向量 `P ∈ {0,1}^8`（可重叠——多分量可同时为 true）。
///
/// 字段序 = 优先级序（P1 最先），#149 **不得重排**。P4/P5 已填真实谓词；P7/P8 仍为 #150
/// 占位槽，仅互斥化层（[`first_match`]）全域定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelPredicates {
    /// P1：风险强平 RiskExit（`KThetaRiskGate.force_flat` 同源注入）。
    pub risk_exit: bool,
    /// P2：本级证书平仓·S2 二分→CloseRoot（一/二类反向 = 根清仓）。
    pub cert_close_root: bool,
    /// P3：本级证书平仓·S2 二分→ReduceCore（三类反向 = 核心仓减仓）。
    pub cert_reduce_core: bool,
    /// P4：短差平仓（在册短差腿 + 顺父 confirmed child cert + sound parent projection）。
    pub short_diff_close: bool,
    /// P5：短差开启（父持仓 + 无短差腿 + 反父 confirmed child cert + sound parent projection）。
    pub short_diff_open: bool,
    /// P6：开仓（本声部 slot 空 ∧ 存在本级可交易候选）。
    pub open_entry: bool,
    /// P7：记录（#150 T7 真实装：本级持仓期间次级别走势类型完整走完 ∧ 全程无证书投影
    /// ⟹ 记录桶命中——只写账本记录，不动任何仓位）。
    pub record: bool,
    /// P8：加仓（**占位恒 false**，#150 后续加仓票）。
    pub add_position: bool,
}

impl ChannelPredicates {
    /// 按 1-based 谓词号读 `P_j`（j∈1..=8）。越界 ⟹ panic（内部不变量）。
    fn p(&self, j: usize) -> bool {
        match j {
            1 => self.risk_exit,
            2 => self.cert_close_root,
            3 => self.cert_reduce_core,
            4 => self.short_diff_close,
            5 => self.short_diff_open,
            6 => self.open_entry,
            7 => self.record,
            8 => self.add_position,
            _ => unreachable!("通道谓词索引 j={j} 越界（合法 1..={CHANNEL_M}）"),
        }
    }
}

/// 通道命中类号：`C0`=兜底 Hold，`Cj(j)`=最小成立谓词索引（j∈1..=8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelId {
    /// C_0：⋀_j ¬P_j（全谓词为假，Hold 兜底）。
    C0,
    /// C_j：P_j ∧ ⋀_{k<j} ¬P_k（1-based 谓词号）。
    Cj(u8),
}

/// 显式裁决（每声部每时刻恰一枚）。出场语义**复用单源 [`ExitType`]**（含显式 Hold），
/// 仅补开仓/短差开启/记录/加仓四个非 `ExitType` 动作——不镜像出场枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDecision {
    /// 出场/持有裁决（C1→RiskExit、C2→CloseRoot、C3→ReduceCore、C4→CloseShortDiff、
    /// C0→Hold；单源 [`ExitType`]）。
    Exit(ExitType),
    /// C5：短差开启（#149 实装）。
    OpenShortDiff,
    /// C6：开仓。
    Open,
    /// C7：记录（#150 T7 实装：P7 记录桶命中——只写账本 [`P7Record`]，仓位零变动）。
    Record,
    /// C8：加仓（占位通道，不可达）。
    AddPosition,
}

/// first-match 互斥化：取最小成立谓词索引 r ⟹ C_r；全 false ⟹ C_0。
///
/// **L0 定理**：任意输入恰命中一条通道（`Σ_{j=0}^8 1[C_j]=1`，构造内蕴；穷举见证
/// `tests::channel_first_match_exhaustive_2pow8`）。
pub fn first_match(p: &ChannelPredicates) -> ChannelId {
    for j in 1..=CHANNEL_M {
        if p.p(j) {
            return ChannelId::Cj(j as u8);
        }
    }
    ChannelId::C0
}

/// 通道 → 显式裁决（全定义：9 通道各恰一枚裁决，含 C0 显式 Hold）。
pub fn decision_of(c: ChannelId) -> ChannelDecision {
    match c {
        ChannelId::C0 => ChannelDecision::Exit(ExitType::Hold),
        ChannelId::Cj(1) => ChannelDecision::Exit(ExitType::RiskExit),
        ChannelId::Cj(2) => ChannelDecision::Exit(ExitType::CloseRoot),
        ChannelId::Cj(3) => ChannelDecision::Exit(ExitType::ReduceCore),
        ChannelId::Cj(4) => ChannelDecision::Exit(ExitType::CloseShortDiff),
        ChannelId::Cj(5) => ChannelDecision::OpenShortDiff,
        ChannelId::Cj(6) => ChannelDecision::Open,
        ChannelId::Cj(7) => ChannelDecision::Record,
        ChannelId::Cj(8) => ChannelDecision::AddPosition,
        ChannelId::Cj(j) => unreachable!("通道号 {j} 越界（合法 1..={CHANNEL_M}）"),
    }
}

/// #149 在册 ShortDiff 子声部。字段私有，只有 P5 的 sound projection 路径能构造。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffVoice {
    leg: ActiveLeg,
}

impl ShortDiffVoice {
    pub fn leg(&self) -> ActiveLeg {
        self.leg
    }

    pub fn side(&self) -> VoiceSide {
        self.leg.dir
    }
}

/// 父声部状态（parent slot + 至多一个 ShortDiff child slot）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceState {
    /// 本声部级别 ℓ（谓词只消费本级候选——「本级证书平仓」）。
    pub level: u32,
    /// 当前持仓腿（None = 空仓 slot）。
    pub leg: Option<ActiveLeg>,
    /// 持仓腿入场角色垂直轴（入场固定，[`reverse_exit_type`] 第一参；空仓时无效值 Ambient）。
    pub entry_v: Vertical,
    /// 当前步序号（0-based；[`advance`] 每步 +1——P7 记录端点的可重放见证坐标，#150）。
    pub step: usize,
    /// #150 T7：次级别完整走势检测器状态（仅本级持仓期间武装，持仓边界重置）。
    pub sub_cycle: SubCycleTracker,
    /// #149 独立短差子声部；P4 只清此槽，P5 只写此槽，父 `leg` 保持不动。
    pub short_diff: Option<ShortDiffVoice>,
}

/// 单时刻声部输入（事件）。
#[derive(Debug, Clone)]
pub struct VoiceStepInput {
    /// P1 判据：风险强平（`KThetaRiskGate.force_flat` 同源）。
    pub force_flat: bool,
    /// 当步候选集（本函数内按 ≺_Θ 排序后 first-match 消费；跨级候选被本级过滤——
    /// **次级别 ℓ−1 证书候选由 P7 检测器消费**，#150）。
    pub candidates: Vec<Candidate>,
    /// #150：当步父级中枢语境 κ（lean #144 文档「中枢内/中枢上/三买后/三卖后等结构位置」）。
    /// 与 `force_flat` 同为外源注入——本模块不重推中枢几何，语境由上游中枢检测口径单源提供。
    pub parent_kappa: ParentKappa,
    /// #149：从真塔父子边构造的 parent-level certificate projections。P4/P5 必须逐候选通过
    /// [`trigger_projection_sound`]；空表即 child signal 无父投影，严格不得触发短差动作。
    pub parent_projections: Vec<ParentCertificateProjection>,
}

// ════════════════════════════════════════════════════════════════════════════
//  #150 T7：P7 记录桶 + 次级别完整走势检测器（#144 数据面）
// ════════════════════════════════════════════════════════════════════════════

/// 父级中枢语境 κ（#144 lean 文档 §2.1：κ 含中枢内/中枢上/三买后/三卖后等结构位置；
/// `cycle_endpoints_project_iff_context_visible` 按买/卖端点分别应用 ContextProjectionLaw，
/// 故 P7 记录对**每个端点各自**入账 κ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParentKappa {
    /// 上游未注入语境（缺省显式 Unknown——不伪造结构位置）。
    #[default]
    Unknown,
    /// 中枢内。
    InsideCenter,
    /// 中枢上。
    AboveCenter,
    /// 中枢下。
    BelowCenter,
    /// 三买后。
    AfterThirdBuy,
    /// 三卖后。
    AfterThirdSell,
}

/// 子周期开端点挂起状态（tracker 内部：开端点证书 + 落账坐标 + 投影见证累积）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SubOpen {
    /// 开端点证书候选（次级别，已确认）。
    cand: Candidate,
    /// 开端点步序号。
    step: usize,
    /// 开端点当步父级中枢语境 κ。
    kappa: ParentKappa,
    /// 子周期存续期间（含开端点当步）是否已出现本级证书投影。
    projected: bool,
}

/// 次级别完整走势检测器状态（#150：`Default` = 未武装/无挂起开端点）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubCycleTracker {
    /// 挂起的子周期开端点（None = 等待开端点）。
    open: Option<SubOpen>,
}

/// P7 记录端点：次级别证书候选**整体单源入账**（非摘要镜像）+ 可重放坐标 + 端点 κ。
#[derive(Debug, Clone, Copy)]
pub struct SubEndpoint {
    /// 端点证书候选（含 source_index/bits/bsp_class/dir/nest_confirmed/role——
    /// #144 §5 步骤 4 可重放见证数据）。
    pub cand: Candidate,
    /// 端点步序号（回放坐标）。
    pub step: usize,
    /// 端点当步父级中枢语境 κ（#144 §5 步骤 2 ContextProjectionLaw 数据）。
    pub kappa: ParentKappa,
}

/// 子周期完成观测（检测器单步输出；`projected` 判 P7/短差域二分）。
#[derive(Debug, Clone, Copy)]
struct CompletedSubCycle {
    open: SubEndpoint,
    close: SubEndpoint,
    /// 子周期全程（开端点步..=收端点步）是否出现过本级证书投影。
    projected: bool,
}

/// 检测器单步观测：下一 tracker 状态 + 本步完成的子周期（若有）。
struct SubCycleObs {
    next: SubCycleTracker,
    completed: Option<CompletedSubCycle>,
}

/// P7 记录（#150 记录桶 schema——#144 形式化 §5 步骤 1–4 生产级验证数据面，
/// 使条件链验证无需二次改账本）：
/// 1. 跨级投影关系：`no_projection` + 两端点证书（步骤 1）；
/// 2. 父级语境规律：端点各自 κ（步骤 2）；
/// 3. 短差触发对接：投影子周期被排除出 P7（步骤 3，与 #149 短差域互斥）；
/// 4. 可重放见证：端点 `step` + `cand.source_index` + bits + Conf^δ（步骤 4）。
#[derive(Debug, Clone, Copy)]
pub struct P7Record {
    /// 本级声部级别 ℓ。
    pub voice_level: u32,
    /// 次级别（递归意义 ℓ−1）。
    pub sub_level: u32,
    /// 记录落账步（= 收端点步）。
    pub step: usize,
    /// 子周期开端点证书。
    pub open: SubEndpoint,
    /// 子周期收端点证书（反向确认）。
    pub close: SubEndpoint,
    /// 无投影标记（P7 命中前提，恒 true——显式入账使 #144 验证不需反推）。
    pub no_projection: bool,
}

/// 声部账本（#150：P7 记录桶。只写记录，不动仓位）。
#[derive(Debug, Default)]
pub struct VoiceLedger {
    /// P7 记录桶（落账序 = 命中序）。
    pub p7: Vec<P7Record>,
}

/// 已确认可交易证书候选（端点/投影共用判据：方向非 Flat ∧ 有类 ∧ 区间套证书 Conf^δ）。
fn cert_confirmed(c: &Candidate) -> bool {
    c.dir != VoiceSide::Flat && c.bsp_class != u8::MAX && c.nest_confirmed
}

/// 次级别完整走势检测器单步观测（纯函数，#150）。
///
/// 域护栏：仅本级持仓 ∧ ℓ≥1（递归底 ℓ=0 无次级别）时武装；空仓/关腿由 [`advance`] 重置。
///
/// 判据：
/// - **端点** = 次级别（ℓ−1）已确认证书候选（[`cert_confirmed`]），≺_Θ 序消费；
/// - **完整走完** = 挂起开端点后出现反向端点（[`reverse_signal`] 单源反向判据）；
///   同向次级别证书 = 走势延伸，不重置不改开端点；
/// - **证书投影** = 子周期存续任一步（含两端点当步）出现本级（ℓ）已确认证书候选——
///   #144 §5 步骤 1「真实跨级投影关系」的 L1 数据面判据（本级反向证书亦属投影，
///   且经 first-match 先落 P2/P3）。
fn observe_sub_cycle(state: &VoiceState, input: &VoiceStepInput) -> SubCycleObs {
    if state.leg.is_none() || state.level == 0 {
        return SubCycleObs { next: SubCycleTracker::default(), completed: None };
    }
    let sub_level = state.level - 1;
    // 本级证书投影见证（当步）：本级确认证书候选，或 #149 父证书投影见证
    // （`parent_projections` 即 TriggerProjectionSound 数据面——投影子周期属短差域，
    // 必须排除出 P7，互斥约定见 [`P7Record`] doc 第 3 条）。
    let projection_now = !input.parent_projections.is_empty()
        || input.candidates.iter().any(|c| c.level == state.level && cert_confirmed(c));
    let mut open = state.sub_cycle.open;
    if let Some(o) = &mut open {
        o.projected |= projection_now;
    }
    let mut completed = None;
    for c in theta_ordered(&input.candidates) {
        if c.level != sub_level || !cert_confirmed(c) {
            continue;
        }
        match open {
            // 开端点：首个次级别确认证书。
            None => {
                open = Some(SubOpen {
                    cand: *c,
                    step: state.step,
                    kappa: input.parent_kappa,
                    projected: projection_now,
                });
            }
            // 收端点：反向确认 ⟹ 子周期完整走完（单步至多一枚完成观测）。
            Some(o) if completed.is_none() && reverse_signal(o.cand.dir, &c.bits) => {
                completed = Some(CompletedSubCycle {
                    open: SubEndpoint { cand: o.cand, step: o.step, kappa: o.kappa },
                    close: SubEndpoint { cand: *c, step: state.step, kappa: input.parent_kappa },
                    projected: o.projected,
                });
                open = None;
            }
            // 同向端点：走势延伸，开端点不变。
            Some(_) => {}
        }
    }
    SubCycleObs { next: SubCycleTracker { open }, completed }
}

/// 从父声部状态 + 当步输入推导通道谓词向量（P1..P8 字段序/优先级保持 #147 原样）。
///
/// 判据（全部单源复用）：
/// - P1 = `force_flat`。
/// - P2/P3 = 持仓 ∧ 存在本级反向证书候选（[`reverse_signal`]，≺_Θ 序首个命中），
///   经 [`reverse_exit_type`]`(entry_v, trigger_class)` S2 二分：CloseRoot→P2、ReduceCore→P3。
/// - P4 = 短差在册 ∧ 存在同 child level 的顺父 confirmed certificate，其真父投影匹配当前父腿；
///   该证书相对短差腿为反向，故只关闭短差子声部。
/// - P5 = 父腿在册 ∧ 短差槽空 ∧ 存在反父 SubLevel ShortDiff confirmed certificate，其真父
///   投影匹配当前父腿；子方向由 [`short_diff_side`] 强制 `sigma_u=-sigma_parent`。
/// - P6 = 空仓 ∧ 存在本级可交易候选（dir≠Flat ∧ 有类）。
pub fn voice_predicates(state: &VoiceState, input: &VoiceStepInput) -> ChannelPredicates {
    let mut p = ChannelPredicates { risk_exit: input.force_flat, ..ChannelPredicates::default() };
    // P2/P3：本级证书平仓——持仓 ∧ ≺_Θ 序首个本级反向证书候选，S2 二分经单源 reverse_exit_type。
    if let Some(leg) = &state.leg {
        if let Some(c) = find_reverse(state.level, leg.dir, &input.candidates) {
            match reverse_exit_type(state.entry_v, c.bsp_class) {
                ExitType::CloseRoot => p.cert_close_root = true, // P2
                ExitType::ReduceCore => p.cert_reduce_core = true, // P3
                // 父腿本级反向不可能按入场角色归 ShortDiff；防御性不把它误写 P4，P4 单独按
                // 在册 child + sound parent projection 推导。
                ExitType::CloseShortDiff => {}
                ExitType::RiskExit | ExitType::Hold => {
                    unreachable!("reverse_exit_type 只产三 close 枚举")
                }
            }
        }
    }
    if find_short_diff_close(state, input).is_some() {
        p.short_diff_close = true; // P4（字段位置/优先级不动）
    }
    if find_short_diff_open(state, input).is_some() {
        p.short_diff_open = true; // P5（字段位置/优先级不动）
    }
    if state.leg.is_none() && find_open(state.level, &input.candidates).is_some() {
        // P6：开仓——空仓 slot ∧ 存在本级可交易非 ShortDiff 角色候选（ShortDiff 角色只走 P5）。
        p.open_entry = true;
    }
    // P7（#150 T7）：本级持仓期间次级别走势类型完整走完 ∧ 全程无证书投影 ⟹ 记录桶。
    // 有投影的子周期属 #149 短差域，不置位 P7——互斥由谓词自身语义保证，
    // first-match 排序不动（P7 谓词允许与 P1 重叠，互斥化在 first_match 层）。
    if let Some(cc) = observe_sub_cycle(state, input).completed {
        p.record = !cc.projected;
    }
    // P8：占位槽（加仓票），恒 false——不存在置位路径。
    p
}

/// 候选是否携带匹配当前父腿的 TriggerProjectionSound 见证。
fn has_sound_parent_projection(
    parent: &ActiveLeg,
    child: &Candidate,
    input: &VoiceStepInput,
) -> bool {
    input
        .parent_projections
        .iter()
        .any(|projection| trigger_projection_sound(parent, child, projection))
}

/// P5：首个可开启短差的真实 child certificate。
fn find_short_diff_open<'a>(
    state: &VoiceState,
    input: &'a VoiceStepInput,
) -> Option<&'a Candidate> {
    if state.short_diff.is_some() {
        return None;
    }
    let parent = state.leg.as_ref()?;
    let child_side = short_diff_side(parent.dir)?;
    theta_ordered(&input.candidates).into_iter().find(|c| {
        c.role.is_sub_level_short_diff()
            && c.dir == child_side
            && reverse_signal(parent.dir, &c.bits)
            && has_sound_parent_projection(parent, c, input)
    })
}

/// P4：首个可关闭当前短差腿的顺父 child certificate。
fn find_short_diff_close<'a>(
    state: &VoiceState,
    input: &'a VoiceStepInput,
) -> Option<&'a Candidate> {
    let parent = state.leg.as_ref()?;
    let short_diff = state.short_diff.as_ref()?;
    theta_ordered(&input.candidates).into_iter().find(|c| {
        c.level == short_diff.leg.level
            && c.dir == parent.dir
            && c.role.v == Vertical::FollowParent
            && c.role.grade == super::coverage::GradeRel::SubLevel
            && reverse_signal(short_diff.leg.dir, &c.bits)
            && has_sound_parent_projection(parent, c, input)
    })
}

/// ≺_Θ 序首个本级反向证书候选（[`reverse_signal`] §9 closePred 反向项；可交易候选域）。
fn find_reverse(level: u32, leg_dir: VoiceSide, cands: &[Candidate]) -> Option<&Candidate> {
    theta_ordered(cands).into_iter().find(|c| {
        c.level == level
            && c.dir != VoiceSide::Flat
            && c.bsp_class != u8::MAX
            && reverse_signal(leg_dir, &c.bits)
    })
}

/// ≺_Θ 序首个本级普通开仓候选（可交易 ∧ 非 ShortDiff 角色；短差开启只属 P5）。
fn find_open(level: u32, cands: &[Candidate]) -> Option<&Candidate> {
    theta_ordered(cands).into_iter().find(|c| {
        c.level == level
            && c.dir != VoiceSide::Flat
            && c.bsp_class != u8::MAX
            && c.role.v != Vertical::ShortDiff
    })
}

/// 候选按 ≺_Θ 排序（[`theta_key`] 单源全序，不 mutate 输入）。
fn theta_ordered(cands: &[Candidate]) -> Vec<&Candidate> {
    let mut ordered: Vec<&Candidate> = cands.iter().collect();
    ordered.sort_by_key(|c| theta_key(c));
    ordered
}

/// 单时刻声部裁决：谓词推导 → first-match → 恰一枚显式裁决。
pub fn step_voice(state: &VoiceState, input: &VoiceStepInput) -> (ChannelId, ChannelDecision) {
    let cid = first_match(&voice_predicates(state, input));
    (cid, decision_of(cid))
}

/// 端到端驱动：事件序列 → 完整裁决序列（`out.len() == steps.len()`，每时刻恰一枚，
/// 含显式 Hold）。状态转移（不可变，逐步产新 state）：
/// - `Exit(RiskExit|CloseRoot|ReduceCore)` ⟹ 父腿关闭（对齐 interp 规则2：被反向命中的腿
///   入 𝒟_x；ReduceCore 减核心在腿粒度同为关闭——本票腿即最小持仓单元）。
/// - `Exit(CloseShortDiff)` ⟹ **只清短差子槽，父腿不动**。
/// - `OpenShortDiff` ⟹ 写入独立 child slot，方向由 `short_diff_side(parent)` 构造。
/// - `Open` ⟹ 以触发候选建腿（entry_v = 候选角色垂直轴，入场固定）。
/// - `Exit(Hold)` ⟹ 状态不变。P7/P8 仍不可达。
pub fn run_voice(
    initial: VoiceState,
    steps: &[VoiceStepInput],
) -> Vec<(ChannelId, ChannelDecision)> {
    run_voice_ledgered(initial, steps).0
}

/// 端到端驱动 + 账本（#150）：裁决序列与 [`run_voice`] 逐时刻相同（账本层不改变通道
/// 语义），另返回 [`VoiceLedger`]——C7 命中步落一条 [`P7Record`]（只写记录，仓位零变动，
/// 转移见 [`advance`] 的 Record 分支）。
pub fn run_voice_ledgered(
    initial: VoiceState,
    steps: &[VoiceStepInput],
) -> (Vec<(ChannelId, ChannelDecision)>, VoiceLedger) {
    let mut state = initial;
    let mut out = Vec::with_capacity(steps.len());
    let mut ledger = VoiceLedger::default();
    for input in steps {
        let (cid, dec) = step_voice(&state, input);
        if dec == ChannelDecision::Record {
            let cc = observe_sub_cycle(&state, input)
                .completed
                .expect("C7 命中 ⟹ P7 真 ⟹ 子周期完成观测存在（voice_predicates 同判据）");
            debug_assert!(!cc.projected, "C7 命中 ⟹ 无投影（P7 谓词语义）");
            ledger.p7.push(P7Record {
                voice_level: state.level,
                sub_level: state.level - 1,
                step: state.step,
                open: cc.open,
                close: cc.close,
                no_projection: !cc.projected,
            });
        }
        state = advance(state, input, dec);
        out.push((cid, dec));
    }
    (out, ledger)
}

/// 状态转移（不可变：产新 state，不 mutate 旧值）。全定义 match（占位通道也有确定转移）。
///
/// #150 检测器转移：`step` 每步 +1；持仓边界（关腿/开腿）重置 tracker（「本级声部持仓
/// 期间」字面边界——半周期不跨持仓期拼接）；Hold/Record 沿 [`observe_sub_cycle`] 推进
/// （Record 后 tracker 已消费完成观测 ⟹ 自然重置，可连续记录）。
fn advance(state: VoiceState, input: &VoiceStepInput, dec: ChannelDecision) -> VoiceState {
    let sub_cycle = observe_sub_cycle(&state, input).next;
    let step = state.step + 1;
    match dec {
        // 出场：腿关闭（interp 规则2 语义——被命中腿入 𝒟_x；ReduceCore 在腿粒度同为关闭）。
        // 父持仓期结束 ⟹ tracker 重置。
        ChannelDecision::Exit(ExitType::RiskExit | ExitType::CloseRoot | ExitType::ReduceCore) => {
            VoiceState {
                leg: None,
                entry_v: Vertical::Ambient,
                step,
                sub_cycle: SubCycleTracker::default(),
                ..state
            }
        }
        // #149 P4：只关闭 hedge voice；parent leg/entry_v 逐字段保持
        // （父持仓期未结束 ⟹ #150 检测器续武装）。
        ChannelDecision::Exit(ExitType::CloseShortDiff) => {
            VoiceState { short_diff: None, step, sub_cycle, ..state }
        }
        // 显式 Hold / P7 记录（仓位零变动，只推进检测器）。
        ChannelDecision::Exit(ExitType::Hold) | ChannelDecision::Record => {
            VoiceState { step, sub_cycle, ..state }
        }
        // 开仓：以 ≺_Θ 首个开仓候选建腿（entry_v = 候选角色垂直轴，入场固定）。
        // 新持仓期起点 ⟹ tracker 重置（空仓期观测不武装，observe 已返回 default）。
        ChannelDecision::Open => {
            let c = find_open(state.level, &input.candidates)
                .expect("C6 命中 ⟹ P6 真 ⟹ 开仓候选存在（voice_predicates 同判据）");
            VoiceState {
                leg: Some(leg_from_candidate(c)),
                entry_v: c.role.v,
                step,
                sub_cycle: SubCycleTracker::default(),
                ..state
            }
        }
        // #149 P5：只开启 child hedge；parent leg/entry_v 逐字段保持。
        ChannelDecision::OpenShortDiff => {
            let parent = state
                .leg
                .expect("C5 命中 ⟹ P5 真 ⟹ parent voice 在册");
            let c = find_short_diff_open(&state, input)
                .expect("C5 命中 ⟹ P5 真 ⟹ sound ShortDiff child certificate 存在");
            let short_diff = short_diff_voice_from_candidate(&parent, c)
                .expect("P5 判据已保证 child direction = -parent direction");
            VoiceState { short_diff: Some(short_diff), step, sub_cycle, ..state }
        }
        // 占位通道（加仓票实装真转移）：仓位不变。
        ChannelDecision::AddPosition => {
            VoiceState { step, sub_cycle, ..state }
        }
    }
}

/// P5 child candidate → 独立 ShortDiff voice；构造时钉死 `sigma_u=-sigma_parent` 与父身份。
fn short_diff_voice_from_candidate(
    parent: &ActiveLeg,
    c: &Candidate,
) -> Option<ShortDiffVoice> {
    if short_diff_side(parent.dir) != Some(c.dir) {
        return None;
    }
    Some(ShortDiffVoice {
        leg: ActiveLeg {
            level: c.level,
            dir: c.dir,
            source_index: c.source_index,
            lambda: c.source_index,
            id: ElementId { level: c.level, ordinal: c.source_index as u64 },
            parent_id: Some(parent.id),
            is_boundary_root: false,
            op_parent: Some(parent.id),
        },
    })
}

/// #149 P4/P5 端到端回放状态：结构声部树 + 独立数量账本同步推进。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffReplayState {
    pub voice: VoiceState,
    pub ledger: SplitLegLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDiffReplayFrame {
    pub channel: ChannelId,
    pub decision: ChannelDecision,
    pub state: ShortDiffReplayState,
}

/// 回放 #149 短差子链：P5 同步开启结构 child 与 split leg；P4 同步只关闭这两处的 hedge 槽。
/// 其他通道沿用 [`advance`]，本函数不为其制造新的账本事件，因此现有 TwEvent/父腿会计语义不变。
pub fn run_short_diff_replay(
    initial: ShortDiffReplayState,
    steps: &[VoiceStepInput],
    short_diff_qty: u64,
) -> Result<Vec<ShortDiffReplayFrame>, SplitLegError> {
    let mut state = initial;
    let mut out = Vec::with_capacity(steps.len());
    for input in steps {
        let (channel, decision) = step_voice(&state.voice, input);
        let voice = advance(state.voice, input, decision);
        let ledger = match decision {
            ChannelDecision::OpenShortDiff => {
                let side = voice
                    .short_diff
                    .expect("OpenShortDiff 转移后 child voice 必在册")
                    .side();
                state.ledger.apply(SplitLegEvent::OpenShortDiff {
                    side,
                    qty: short_diff_qty,
                })?
            }
            ChannelDecision::Exit(ExitType::CloseShortDiff) => {
                state.ledger.apply(SplitLegEvent::CloseShortDiff)?
            }
            _ => state.ledger,
        };
        state = ShortDiffReplayState { voice, ledger };
        out.push(ShortDiffReplayFrame { channel, decision, state });
    }
    Ok(out)
}

/// 开仓候选 → 活动腿（候选腿 λ==ρ==source_index，坐 hostOf 右端点；本票独立根边界胚元 ∂）。
fn leg_from_candidate(c: &Candidate) -> ActiveLeg {
    ActiveLeg {
        level: c.level,
        dir: c.dir,
        source_index: c.source_index,
        lambda: c.source_index,
        id: ElementId { level: c.level, ordinal: c.source_index as u64 },
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::coverage::{CoverageElement, Dir, GradeRel, Horizontal, OperationRole};
    use super::super::interp::{parent_certificate_projection, ParentCertificateProjection};
    use super::super::ledger::SplitLegLedger;
    use super::super::super::types::BspBits;

    // ── 构造器 ────────────────────────────────────────────────────────────

    fn role(v: Vertical) -> OperationRole {
        OperationRole { h: Horizontal::First, v, delta: Dir::Plus, grade: GradeRel::SameLevel }
    }

    fn child_role(v: Vertical, dir: VoiceSide) -> OperationRole {
        OperationRole {
            h: Horizontal::First,
            v,
            delta: match dir {
                VoiceSide::Long => Dir::Plus,
                VoiceSide::Short => Dir::Minus,
                VoiceSide::Flat => Dir::Plus,
            },
            grade: GradeRel::SubLevel,
        }
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

    fn cand(gi: usize, level: u32, dir: VoiceSide, cls: u8, bits: BspBits, v: Vertical) -> Candidate {
        Candidate {
            level,
            source_index: 10 + gi,
            bits,
            dir,
            bsp_class: cls,
            role: role(v),
            nest_confirmed: true,
            gamma_index: gi,
            force: None,
        }
    }

    fn leg(level: u32, dir: VoiceSide) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: 5,
            lambda: 5,
            id: ElementId { level, ordinal: 5 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    fn holding(level: u32, dir: VoiceSide, entry_v: Vertical) -> VoiceState {
        VoiceState {
            level,
            leg: Some(leg(level, dir)),
            entry_v,
            step: 0,
            sub_cycle: SubCycleTracker::default(),
            short_diff: None,
        }
    }
    fn empty_voice(level: u32) -> VoiceState {
        VoiceState {
            level,
            leg: None,
            entry_v: Vertical::Ambient,
            step: 0,
            sub_cycle: SubCycleTracker::default(),
            short_diff: None,
        }
    }

    fn step(force_flat: bool, candidates: Vec<Candidate>) -> VoiceStepInput {
        VoiceStepInput {
            force_flat,
            candidates,
            parent_kappa: ParentKappa::Unknown,
            parent_projections: Vec::new(),
        }
    }

    /// 带父级中枢语境 κ 的步输入（#150 P7 记录面）。
    fn step_k(candidates: Vec<Candidate>, kappa: ParentKappa) -> VoiceStepInput {
        VoiceStepInput {
            force_flat: false,
            candidates,
            parent_kappa: kappa,
            parent_projections: Vec::new(),
        }
    }

    fn child_cand(
        gi: usize,
        level: u32,
        dir: VoiceSide,
        cls: u8,
        bits: BspBits,
        v: Vertical,
    ) -> Candidate {
        Candidate { role: child_role(v, dir), ..cand(gi, level, dir, cls, bits, v) }
    }

    /// 用真父子 `CoverageElement` 关系构造投影，测试不直接伪造 sound token。
    fn projection_for(parent: ActiveLeg, child: &Candidate) -> ParentCertificateProjection {
        let parent_element = CoverageElement {
            lambda: parent.lambda,
            rho: parent.source_index,
            eps: parent.dir,
            level: parent.level,
            parent: None,
            attached_dir: None,
            id: parent.id,
            parent_id: parent.parent_id,
        };
        let child_element = CoverageElement {
            lambda: child.source_index,
            rho: child.source_index,
            eps: child.dir,
            level: child.level,
            parent: Some(0),
            attached_dir: Some(parent.dir),
            id: ElementId { level: child.level, ordinal: child.source_index as u64 },
            parent_id: Some(parent.id),
        };
        parent_certificate_projection(child, &[child_element], &[parent_element])
            .expect("真父子元素 + confirmed child certificate 应产 parent projection")
    }

    fn projected_step(parent: ActiveLeg, child: Candidate) -> VoiceStepInput {
        let projection = projection_for(parent, &child);
        VoiceStepInput {
            force_flat: false,
            candidates: vec![child],
            parent_kappa: ParentKappa::Unknown,
            parent_projections: vec![projection],
        }
    }

    /// bits → 谓词向量解码（bit j-1 ↔ P_j）。
    fn decode(bits: u32) -> ChannelPredicates {
        let b = |j: usize| (bits >> (j - 1)) & 1 == 1;
        ChannelPredicates {
            risk_exit: b(1),
            cert_close_root: b(2),
            cert_reduce_core: b(3),
            short_diff_close: b(4),
            short_diff_open: b(5),
            open_entry: b(6),
            record: b(7),
            add_position: b(8),
        }
    }

    // ── 互斥性 + 穷尽性（谓词真值组合矩阵，2^8 穷举）──────────────────────

    /// ★核心可证伪：穷举全部 2^8=256 谓词组合，独立重算每条 `1[C_j]` 指示函数
    /// （不调 `first_match`，避免循环论证），断言 Σ==1（互斥 + 穷尽），并交叉验证
    /// 命中类号与 `first_match` 实装一致。
    #[test]
    fn channel_first_match_exhaustive_2pow8() {
        for bits in 0u32..(1 << CHANNEL_M) {
            let p = decode(bits);
            let mut sum = 0usize;
            let mut hit: Option<ChannelId> = None;
            if (1..=CHANNEL_M).all(|j| !p.p(j)) {
                sum += 1;
                hit = Some(ChannelId::C0);
            }
            for j in 1..=CHANNEL_M {
                if p.p(j) && (1..j).all(|k| !p.p(k)) {
                    sum += 1;
                    hit = Some(ChannelId::Cj(j as u8));
                }
            }
            assert_eq!(sum, 1, "bits={bits:08b}: Σ_j 1[C_j]={sum} ≠ 1（互斥性/穷尽性破裂）");
            assert_eq!(first_match(&p), hit.unwrap(), "bits={bits:08b}: 实装 ≠ 独立重算");
        }
    }

    /// P1 恒最先：P1=true 的全部 2^7 组合（无论其余谓词如何重叠）⟹ C1 = RiskExit。
    #[test]
    fn risk_exit_masks_any_predicate_combination() {
        for rest in 0u32..(1 << (CHANNEL_M - 1)) {
            let p = decode(1 | (rest << 1));
            assert_eq!(first_match(&p), ChannelId::Cj(1), "P1 成立必 C1（rest={rest:07b}）");
            assert_eq!(decision_of(ChannelId::Cj(1)), ChannelDecision::Exit(ExitType::RiskExit));
        }
    }

    /// 通道→裁决全定义：9 条通道各恰一枚显式裁决（含 C0 显式 Hold），出场语义单源 ExitType。
    #[test]
    fn decision_total_and_exit_type_single_source() {
        assert_eq!(decision_of(ChannelId::C0), ChannelDecision::Exit(ExitType::Hold));
        assert_eq!(decision_of(ChannelId::Cj(1)), ChannelDecision::Exit(ExitType::RiskExit));
        assert_eq!(decision_of(ChannelId::Cj(2)), ChannelDecision::Exit(ExitType::CloseRoot));
        assert_eq!(decision_of(ChannelId::Cj(3)), ChannelDecision::Exit(ExitType::ReduceCore));
        assert_eq!(decision_of(ChannelId::Cj(4)), ChannelDecision::Exit(ExitType::CloseShortDiff));
        assert_eq!(decision_of(ChannelId::Cj(5)), ChannelDecision::OpenShortDiff);
        assert_eq!(decision_of(ChannelId::Cj(6)), ChannelDecision::Open);
        assert_eq!(decision_of(ChannelId::Cj(7)), ChannelDecision::Record);
        assert_eq!(decision_of(ChannelId::Cj(8)), ChannelDecision::AddPosition);
    }

    // ── 声部级谓词推导（S2 二分 + P4/P5 短差 + C0 兜底）──────────────────

    /// S2 二分与消费端单源同判据：一/二类反向 ⟹ C2 CloseRoot；三类反向 ⟹ C3 ReduceCore。
    /// 交叉断言 == reverse_exit_type（同源见证，非镜像）。
    #[test]
    fn cert_close_s2_split_matches_reverse_exit_type() {
        let st = holding(0, VoiceSide::Long, Vertical::Ambient);
        for (cls, want_cj, want_et) in [
            (1u8, 2u8, ExitType::CloseRoot),
            (2u8, 2u8, ExitType::CloseRoot),
            (3u8, 3u8, ExitType::ReduceCore),
        ] {
            let input = step(false, vec![cand(0, 0, VoiceSide::Short, cls, sell(cls), Vertical::Ambient)]);
            let (cid, dec) = step_voice(&st, &input);
            assert_eq!(cid, ChannelId::Cj(want_cj), "class={cls} S2 二分通道");
            assert_eq!(dec, ChannelDecision::Exit(want_et));
            assert_eq!(
                reverse_exit_type(st.entry_v, cls),
                want_et,
                "单源交叉：通道裁决 == reverse_exit_type（class={cls}）"
            );
        }
    }

    /// RiskExit 与结构性谓词同刻同时为真 ⟹ RiskExit 恒最先命中（声部级）。
    #[test]
    fn risk_and_structural_same_instant_risk_first() {
        // 持仓 + 反向证书候选（P2 真）+ force_flat（P1 真）⟹ C1。
        let st = holding(0, VoiceSide::Long, Vertical::Ambient);
        let input = step(true, vec![cand(0, 0, VoiceSide::Short, 1, sell(1), Vertical::Ambient)]);
        let p = voice_predicates(&st, &input);
        assert!(p.risk_exit && p.cert_close_root, "前提：P1 与 P2 同刻重叠");
        assert_eq!(step_voice(&st, &input), (ChannelId::Cj(1), ChannelDecision::Exit(ExitType::RiskExit)));
        // 空仓 + 开仓候选（P6 真）+ force_flat ⟹ 仍 C1。
        let st2 = empty_voice(0);
        let input2 = step(true, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]);
        let p2 = voice_predicates(&st2, &input2);
        assert!(p2.risk_exit && p2.open_entry, "前提：P1 与 P6 同刻重叠");
        assert_eq!(step_voice(&st2, &input2), (ChannelId::Cj(1), ChannelDecision::Exit(ExitType::RiskExit)));
    }

    /// 全谓词为假 ⟹ C0/Hold 兜底（空候选 / 无关级别候选 / 持仓遇同向候选=加仓占位域）。
    #[test]
    fn all_false_falls_to_c0_explicit_hold() {
        let hold = (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold));
        // 空候选、空仓。
        assert_eq!(step_voice(&empty_voice(0), &step(false, vec![])), hold);
        // 空候选、持仓。
        assert_eq!(step_voice(&holding(0, VoiceSide::Long, Vertical::Ambient), &step(false, vec![])), hold);
        // 跨级候选不进本声部（本级过滤）。
        let cross = step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]);
        assert_eq!(step_voice(&empty_voice(0), &cross), hold);
        // 持仓 + 同向候选 = 加仓域（P8 占位恒 false）⟹ C0 Hold。
        let same_dir = step(false, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]);
        assert_eq!(step_voice(&holding(0, VoiceSide::Long, Vertical::Ambient), &same_dir), hold);
        // 不可交易候选（Flat/无类）不触发 P6。
        let flat = step(false, vec![cand(0, 0, VoiceSide::Flat, u8::MAX, BspBits::default(), Vertical::Ambient)]);
        assert_eq!(step_voice(&empty_voice(0), &flat), hold);
    }

    /// #149 改写说明：原护栏锁死 P4/P5 恒 false；本票填入真实短差谓词后，改锁「P5 真开、
    /// P4 真关，且 #150 的 P8 仍恒 false」。保留原测试位置与护栏职责，不删除历史覆盖面。
    #[test]
    fn p4_p5_real_predicates_while_p8_remains_placeholder() {
        let parent = holding(1, VoiceSide::Long, Vertical::Ambient);
        let parent_leg = parent.leg.expect("parent held");
        let open_child = child_cand(
            0,
            0,
            VoiceSide::Short,
            1,
            sell(1),
            Vertical::ShortDiff,
        );
        let open_input = projected_step(parent_leg, open_child);
        let p5 = voice_predicates(&parent, &open_input);
        assert!(p5.short_diff_open, "真实父投影 + 反父 confirmed child cert ⟹ P5");
        assert!(!p5.open_entry, "ShortDiffEntry 只能走 P5，不能漏入 P6");
        assert!(!p5.add_position, "P8 仍是占位槽");

        let hedged = advance(parent, &open_input, ChannelDecision::OpenShortDiff);
        let close_child = child_cand(
            0,
            0,
            VoiceSide::Long,
            1,
            buy(1),
            Vertical::FollowParent,
        );
        let close_input = projected_step(parent_leg, close_child);
        let p4 = voice_predicates(&hedged, &close_input);
        assert!(p4.short_diff_close, "真实父投影 + 顺父 confirmed child cert ⟹ P4");
        assert!(!p4.add_position, "P8 仍是占位槽");
    }

    // ── #150 T7：P7 记录桶 + 次级别完整走势检测器 ─────────────────────────

    /// 次级别（递归意义 ℓ−1）证书候选构造器。
    fn sub_cand(gi: usize, sub_level: u32, dir: VoiceSide, cls: u8, bits: BspBits) -> Candidate {
        cand(gi, sub_level, dir, cls, bits, Vertical::Ambient)
    }

    /// 验收1 端到端：父级中枢内完整子周期回放 ⟹ 账本出现 P7 记录（端点+κ+无投影标记）、
    /// 仓位零变动、无短差声部被开启。
    #[test]
    fn p7_end_to_end_sub_cycle_records_without_position_change() {
        let steps = vec![
            // step0：本级（ℓ=1）开仓 ⟹ C6。
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            // step1：次级别（ℓ=0）买端点证书（父级中枢内）⟹ 子周期开端点，C0 Hold。
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            // step2：空事件 ⟹ Hold。
            step(false, vec![]),
            // step3：次级别卖端点证书（反向确认，父级中枢内）⟹ 子周期完整走完，全程无投影 ⟹ C7。
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
            // step4：空事件 ⟹ Hold（记录不改变仓位，声部继续持仓）。
            step(false, vec![]),
        ];
        let (out, ledger) = run_voice_ledgered(empty_voice(1), &steps);
        assert_eq!(
            out,
            vec![
                (ChannelId::Cj(6), ChannelDecision::Open),
                (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
                (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
                (ChannelId::Cj(7), ChannelDecision::Record),
                (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
            ],
            "端到端裁决序列：开仓 → 子周期开端点 Hold → Hold → P7 记录 → Hold"
        );
        // 无短差声部被开启（验收1 + 验收2 互斥断言的通道面）。
        assert!(
            out.iter().all(|(_, d)| *d != ChannelDecision::OpenShortDiff),
            "P7 记录桶不开启任何短差声部"
        );
        // 账本恰一条 P7 记录，schema 三要素齐备。
        assert_eq!(ledger.p7.len(), 1, "恰一条 P7 记录");
        let r = &ledger.p7[0];
        assert_eq!(r.voice_level, 1);
        assert_eq!(r.sub_level, 0, "次级别 = 递归意义 ℓ−1");
        assert_eq!(r.open.step, 1, "开端点落账步（可重放见证）");
        assert_eq!(r.close.step, 3, "收端点落账步（可重放见证）");
        assert_eq!(r.step, 3, "记录落账步 = 收端点步");
        assert_eq!(r.open.kappa, ParentKappa::InsideCenter, "开端点父级中枢语境 κ");
        assert_eq!(r.close.kappa, ParentKappa::InsideCenter, "收端点父级中枢语境 κ");
        assert!(r.no_projection, "无投影标记恒 true（P7 命中前提）");
    }

    /// 验收1 仓位零变动（严格断言）：P7 命中步的状态转移除 tracker/step 外与原状态逐字段相等
    /// ——腿身份、entry_v、level 全不变；记录不产生任何开/平仓副作用。
    #[test]
    fn p7_record_step_position_strictly_unchanged() {
        // 构造「持仓 + tracker 已挂开端点」状态：经 run 前两步到达。
        let pre_steps = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
        ];
        let mut st = empty_voice(1);
        for input in &pre_steps {
            let (_, dec) = step_voice(&st, input);
            st = advance(st, input, dec);
        }
        let close_input = step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter);
        let (cid, dec) = step_voice(&st, &close_input);
        assert_eq!((cid, dec), (ChannelId::Cj(7), ChannelDecision::Record), "前提：本步 C7 命中");
        let nxt = advance(st, &close_input, dec);
        // 仓位零变动：腿（按 ElementId 身份 + 全字段）、entry_v、level 逐一相等。
        assert_eq!(nxt.leg.map(|l| l.id), st.leg.map(|l| l.id), "腿身份不变");
        assert_eq!(nxt.leg.map(|l| l.dir), st.leg.map(|l| l.dir), "腿方向不变");
        assert_eq!(nxt.entry_v, st.entry_v, "entry_v 不变");
        assert_eq!(nxt.level, st.level, "level 不变");
        assert!(nxt.leg.is_some(), "记录后声部仍持仓（只写账本不动仓位）");
    }

    /// 验收2：有投影的子周期不落 P7 记录桶（与 T5 短差通道互斥可断言——投影子周期属
    /// #149 短差域，本票 P4/P5 占位恒 false ⟹ 既不落 P7 也不产任何短差裁决）。
    #[test]
    fn p7_projected_sub_cycle_not_recorded() {
        // 投影时机三变体：子周期中段 / 开端点当步 / 收端点当步。
        let mid = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            // 本级（ℓ=1）同向确认证书候选 = 证书投影（同向 ⟹ 不触发 P2/P3/P6，纯投影见证）。
            step(false, vec![cand(2, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(3, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let at_open = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(
                vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1)), cand(2, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)],
                ParentKappa::InsideCenter,
            ),
            step_k(vec![sub_cand(3, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let at_close = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step_k(
                vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1)), cand(3, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)],
                ParentKappa::InsideCenter,
            ),
        ];
        for (name, steps) in [("中段投影", mid), ("开端点投影", at_open), ("收端点投影", at_close)] {
            let (out, ledger) = run_voice_ledgered(empty_voice(1), &steps);
            assert!(ledger.p7.is_empty(), "{name}：有投影子周期不落 P7 记录桶");
            assert!(
                out.iter().all(|(cid, _)| !matches!(cid, ChannelId::Cj(4 | 5 | 7))),
                "{name}：互斥断言——投影子周期不命中 P7，也不产短差通道裁决（P4/P5 #149 占位）"
            );
        }
    }

    /// 验收3：记录字段覆盖 #144 形式化第五节 1–4 步所需数据——
    /// 1) 跨级投影关系数据面 = `no_projection` 标记 + 两端点证书；
    /// 2) 父级语境规律（ContextProjectionLaw）= 端点各自的 κ；
    /// 3) 与短差触发对接 = 投影子周期被排除（见互斥测试）；
    /// 4) 可重放见证 = 端点 step + source_index + bits + nest_confirmed。
    #[test]
    fn p7_record_schema_covers_144_section5_fields() {
        let steps = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 2, buy(2))], ParentKappa::AfterThirdBuy),
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 3, sell(3))], ParentKappa::AboveCenter),
        ];
        let (_, ledger) = run_voice_ledgered(empty_voice(1), &steps);
        assert_eq!(ledger.p7.len(), 1);
        let r = &ledger.p7[0];
        // 端点证书完整性（Candidate 单源整体入账，非摘要镜像）。
        assert_eq!(r.open.cand.source_index, 11, "开端点 source_index（可重放见证）");
        assert_eq!(r.close.cand.source_index, 12, "收端点 source_index（可重放见证）");
        assert_eq!(r.open.cand.bsp_class, 2, "开端点证书类号");
        assert_eq!(r.close.cand.bsp_class, 3, "收端点证书类号");
        assert!(r.open.cand.bits.buy2 && r.close.cand.bits.sell3, "端点买卖点向量入账");
        assert!(r.open.cand.nest_confirmed && r.close.cand.nest_confirmed, "端点区间套证书 Conf^δ");
        assert_eq!(r.open.cand.dir, VoiceSide::Long);
        assert_eq!(r.close.cand.dir, VoiceSide::Short);
        // 端点各自 κ（ContextProjectionLaw 按买/卖端点分别应用）。
        assert_eq!(r.open.kappa, ParentKappa::AfterThirdBuy);
        assert_eq!(r.close.kappa, ParentKappa::AboveCenter);
        assert!(r.no_projection, "无投影标记");
        assert_eq!((r.voice_level, r.sub_level), (1, 0));
    }

    /// first-match 排序不被打乱：子周期完成步与更高优先级谓词同刻重叠 ⟹ 高优先级通道胜出，
    /// 记录不落账（P7 谓词可为真——谓词允许重叠，互斥化由 first-match 层完成）。
    #[test]
    fn p7_yields_to_earlier_channels_first_match() {
        // 变体 A：完成步 force_flat ⟹ C1 RiskExit，P7 谓词真但记录不写。
        let pre = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
        ];
        let mut st = empty_voice(1);
        for input in &pre {
            let (_, dec) = step_voice(&st, input);
            st = advance(st, input, dec);
        }
        let flat_close = VoiceStepInput {
            force_flat: true,
            candidates: vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))],
            parent_kappa: ParentKappa::InsideCenter,
            parent_projections: Vec::new(),
        };
        let p = voice_predicates(&st, &flat_close);
        assert!(p.risk_exit && p.record, "前提：P1 与 P7 同刻重叠（谓词允许重叠）");
        assert_eq!(
            step_voice(&st, &flat_close),
            (ChannelId::Cj(1), ChannelDecision::Exit(ExitType::RiskExit)),
            "first-match：P1 ≻ P7"
        );
        // 变体 B：完成步本级反向证书 ⟹ C2 CloseRoot 胜出（且该证书构成投影 ⟹ P7 谓词自身为假）。
        let mut st2 = empty_voice(1);
        for input in &pre {
            let (_, dec) = step_voice(&st2, input);
            st2 = advance(st2, input, dec);
        }
        let rev_close = step_k(
            vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1)), cand(3, 1, VoiceSide::Short, 1, sell(1), Vertical::Ambient)],
            ParentKappa::InsideCenter,
        );
        let p2 = voice_predicates(&st2, &rev_close);
        assert!(p2.cert_close_root && !p2.record, "本级反向证书 = 投影 ⟹ P7 谓词假 + P2 真");
        assert_eq!(
            step_voice(&st2, &rev_close),
            (ChannelId::Cj(2), ChannelDecision::Exit(ExitType::CloseRoot))
        );
    }

    /// 检测器域护栏电池：腿关闭重置 tracker / 未确认子证书不计 / 0 级声部无次级别 /
    /// 同向子证书延伸不重置 / 空仓期间子周期不武装。
    #[test]
    fn p7_detector_domain_guards() {
        // (a) 腿关闭重置：开仓 → 子买端点 → 本级反向平仓（C2，tracker 重置）→ 再开仓 →
        //     子卖端点 ⟹ 无记录（前半周期已随腿关闭作废）。
        let a = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step(false, vec![cand(2, 1, VoiceSide::Short, 1, sell(1), Vertical::Ambient)]), // C2
            step(false, vec![cand(3, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),   // C6 再开仓
            step_k(vec![sub_cand(4, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let (_, la) = run_voice_ledgered(empty_voice(1), &a);
        assert!(la.p7.is_empty(), "腿关闭 ⟹ tracker 重置，半周期不跨持仓期拼接");
        // (b) 未确认/无类/Flat 子候选不构成端点。
        let unconfirmed = Candidate { nest_confirmed: false, ..sub_cand(1, 0, VoiceSide::Long, 1, buy(1)) };
        let b = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![unconfirmed], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let (_, lb) = run_voice_ledgered(empty_voice(1), &b);
        assert!(lb.p7.is_empty(), "未确认子证书不构成子周期端点");
        // (c) 0 级声部无次级别（递归底），P7 恒不武装。
        let c = vec![
            step(false, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step(false, vec![]),
        ];
        let (_, lc) = run_voice_ledgered(empty_voice(0), &c);
        assert!(lc.p7.is_empty(), "ℓ=0 声部无次级别 ⟹ 无 P7");
        // (d) 同向子证书延伸不重置：买、买、卖 ⟹ 恰一条记录，开端点 = 首个买端点。
        let d = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(2, 0, VoiceSide::Long, 2, buy(2))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(3, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let (_, ld) = run_voice_ledgered(empty_voice(1), &d);
        assert_eq!(ld.p7.len(), 1);
        assert_eq!(ld.p7[0].open.cand.source_index, 11, "开端点 = 首个子买端点（延伸不重置）");
        // (e) 空仓期间子证书不武装 tracker：先子买端点后本级开仓再子卖端点 ⟹ 无记录
        //     （「本级声部持仓期间」字面边界）。
        let e = vec![
            step_k(vec![sub_cand(0, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step(false, vec![cand(1, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let (_, le) = run_voice_ledgered(empty_voice(1), &e);
        assert!(le.p7.is_empty(), "空仓期间子端点不武装（持仓期间字面边界）");
    }

    /// 记录后 tracker 重置：同一持仓期第二个完整无投影子周期产第二条记录（连续记录能力）。
    #[test]
    fn p7_consecutive_sub_cycles_each_recorded() {
        let steps = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(3, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(4, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
        ];
        let (out, ledger) = run_voice_ledgered(empty_voice(1), &steps);
        assert_eq!(ledger.p7.len(), 2, "两个完整无投影子周期 ⟹ 两条 P7 记录");
        assert_eq!(
            out.iter().filter(|(cid, _)| *cid == ChannelId::Cj(7)).count(),
            2,
            "C7 命中两次"
        );
        assert_eq!(ledger.p7[0].close.step, 2);
        assert_eq!(ledger.p7[1].open.step, 3, "记录后 tracker 重置，第二周期从新开端点起算");
    }

    /// run_voice 与 run_voice_ledgered 裁决序列一致（账本层不改变通道语义）。
    #[test]
    fn p7_run_voice_decision_sequence_unchanged_by_ledger() {
        let steps = vec![
            step(false, vec![cand(0, 1, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step_k(vec![sub_cand(1, 0, VoiceSide::Long, 1, buy(1))], ParentKappa::InsideCenter),
            step_k(vec![sub_cand(2, 0, VoiceSide::Short, 1, sell(1))], ParentKappa::InsideCenter),
            step(true, vec![]),
        ];
        let (out_l, _) = run_voice_ledgered(empty_voice(1), &steps);
        assert_eq!(run_voice(empty_voice(1), &steps), out_l, "run_voice ≡ run_voice_ledgered 裁决面");
    }

    /// #149 改写说明：原测试只断言 ShortDiff 候选不落 P6 并 Hold；现在补上真实 P5 正路，
    /// 同时继续锁死「绝不经 P6 开普通根仓」。
    #[test]
    fn short_diff_role_candidate_routes_via_p5_never_p6() {
        let parent = holding(1, VoiceSide::Long, Vertical::Ambient);
        let child = child_cand(
            0,
            0,
            VoiceSide::Short,
            1,
            sell(1),
            Vertical::ShortDiff,
        );
        let input = projected_step(parent.leg.unwrap(), child);
        let p = voice_predicates(&parent, &input);
        assert!(p.short_diff_open && !p.open_entry, "P5=true 且 P6=false");
        assert_eq!(
            step_voice(&parent, &input),
            (ChannelId::Cj(5), ChannelDecision::OpenShortDiff)
        );
    }

    /// TriggerProjectionSound 反例：即使 child 自称已确认且角色/方向正确，没有父级投影 token
    /// 也不得触发 ShortDiffEntry 或 ShortDiffExit。
    #[test]
    fn trigger_projection_sound_rejects_child_without_parent_projection() {
        let parent = holding(1, VoiceSide::Long, Vertical::Ambient);
        let child = child_cand(
            0,
            0,
            VoiceSide::Short,
            1,
            sell(1),
            Vertical::ShortDiff,
        );
        let input = step(false, vec![child]); // 故意没有 parent_projections
        let p = voice_predicates(&parent, &input);
        assert!(!p.short_diff_open, "无父投影不得触发 P5");
        assert!(!p.open_entry, "无父投影的 ShortDiff 候选也不得降级走 P6");
        assert_eq!(
            step_voice(&parent, &input),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold))
        );

        // 先用 sound projection 建立 hedge，再拿掉顺父 close child 的 projection：P4 也必须拒绝。
        let valid_open = child_cand(
            0,
            0,
            VoiceSide::Short,
            1,
            sell(1),
            Vertical::ShortDiff,
        );
        let open_input = projected_step(parent.leg.unwrap(), valid_open);
        let hedged = advance(parent, &open_input, ChannelDecision::OpenShortDiff);
        let close_without_projection = step(
            false,
            vec![child_cand(
                0,
                0,
                VoiceSide::Long,
                1,
                buy(1),
                Vertical::FollowParent,
            )],
        );
        let close_p = voice_predicates(&hedged, &close_without_projection);
        assert!(!close_p.short_diff_close, "无父投影不得触发 P4");
        assert_eq!(
            step_voice(&hedged, &close_without_projection),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold))
        );
    }

    /// #149 验收 1/2/4 端到端：父多腿 100 保持 → 次级反父证书开空短差 40 → 次级顺父证书
    /// CloseShortDiff；全程父腿结构与数量不动，短差腿独立记账，毛敞口=两腿和，σ_u=-σ_parent。
    #[test]
    fn end_to_end_short_diff_open_close_preserves_parent_split_leg() {
        let parent = holding(1, VoiceSide::Long, Vertical::Ambient);
        let parent_leg = parent.leg.unwrap();
        let open_child = child_cand(
            0,
            0,
            VoiceSide::Short,
            1,
            sell(1),
            Vertical::ShortDiff,
        );
        let close_child = child_cand(
            0,
            0,
            VoiceSide::Long,
            1,
            buy(1),
            Vertical::FollowParent,
        );
        let steps = vec![
            projected_step(parent_leg, open_child),
            projected_step(parent_leg, close_child),
        ];
        let initial = ShortDiffReplayState {
            voice: parent,
            ledger: SplitLegLedger::parent_only(VoiceSide::Long, 100).unwrap(),
        };
        let frames = run_short_diff_replay(initial, &steps, 40).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].channel, ChannelId::Cj(5));
        assert_eq!(frames[0].decision, ChannelDecision::OpenShortDiff);
        assert_eq!(frames[1].channel, ChannelId::Cj(4));
        assert_eq!(frames[1].decision, ChannelDecision::Exit(ExitType::CloseShortDiff));

        for frame in &frames {
            assert_eq!(frame.state.voice.leg, Some(parent_leg), "P4/P5 不得触父腿结构");
            assert_eq!(frame.state.ledger.split_legs().parent.qty, 100, "父腿数量全程恒定");
        }
        let during = frames[0].state.ledger.net_view();
        let hedge = during.split_legs().short_diff.expect("P5 后短差腿在册");
        assert_eq!(hedge.side, VoiceSide::Short);
        assert_eq!(hedge.side, VoiceSide::Long.flip(), "σ_u = -σ_parent");
        assert_eq!(hedge.qty, 40);
        assert_eq!(during.split_legs().long_qty(), 100);
        assert_eq!(during.split_legs().short_qty(), 40);
        assert_eq!(during.gross_qty(), 140, "gross = parent + short-diff");
        assert_eq!(during.net_qty(), 60);

        let after = frames[1].state.ledger.net_view();
        assert!(after.split_legs().short_diff.is_none(), "P4 只关闭短差腿");
        assert_eq!(after.split_legs().parent.qty, 100, "CloseShortDiff 不触父腿");
        assert_eq!(after.gross_qty(), 100);
        assert_eq!(after.net_qty(), 100);
    }

    /// #135 T6 验收 1 端到端：父多腿 100（短差额度默认 = 100）→ 次级反父证书请求开空
    /// 短差 150 超出父级短差额度 ⟹ 开启被拒绝：裁决面命中 C5（OpenShortDiff），账本面
    /// 走 typed 拒绝（GrossExposureExceeded），原账本不变——拒绝路径显式可断言，非静默跳过。
    #[test]
    fn end_to_end_short_diff_open_rejected_when_exceeding_parent_quota() {
        let parent = holding(1, VoiceSide::Long, Vertical::Ambient);
        let parent_leg = parent.leg.unwrap();
        let open_child = child_cand(0, 0, VoiceSide::Short, 1, sell(1), Vertical::ShortDiff);
        let open_input = projected_step(parent_leg, open_child);

        // 裁决面：该时刻命中 C5 OpenShortDiff（拒绝发生在资金约束层，不篡改通道语义）。
        assert_eq!(
            step_voice(&parent, &open_input),
            (ChannelId::Cj(5), ChannelDecision::OpenShortDiff)
        );

        let ledger = SplitLegLedger::parent_only(VoiceSide::Long, 100).unwrap();
        let initial = ShortDiffReplayState { voice: parent, ledger };
        // 请求 150 > 父级短差额度 100 ⟹ 端到端回放以 typed 拒绝终止。
        assert_eq!(
            run_short_diff_replay(initial, std::slice::from_ref(&open_input), 150),
            Err(SplitLegError::GrossExposureExceeded { requested: 150, available: 100 }),
            "超额子对冲请求走显式 typed 拒绝路径"
        );
        // 账本可断言拒绝不落账：原账本仍是纯父腿（不可变单步，拒绝无副作用）。
        assert_eq!(ledger.split_legs().parent.qty, 100);
        assert!(ledger.split_legs().short_diff.is_none(), "拒绝后无短差腿落账");
        // 同一请求在额度内（100）则照常放行——约束只裁超额，不改既有开启语义。
        let ok = run_short_diff_replay(
            ShortDiffReplayState { voice: parent, ledger },
            std::slice::from_ref(&open_input),
            100,
        )
        .unwrap();
        assert_eq!(ok[0].state.ledger.net_view().gross_qty(), 200);
    }

    // ── 端到端：事件序列 → 完整裁决序列（含显式 Hold）────────────────────

    /// 喂事件序列，断言完整裁决序列：开仓 → Hold → 一类反向根清仓 → Hold → 再开仓 →
    /// 风险强平（与反向证书同刻，RiskExit 最先）→ 空仓 Hold。每时刻恰一枚裁决。
    #[test]
    fn end_to_end_verdict_sequence_with_explicit_holds() {
        let steps = vec![
            step(false, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]), // C6 开仓
            step(false, vec![]),                                                          // C0 Hold
            step(false, vec![cand(1, 0, VoiceSide::Short, 1, sell(1), Vertical::Ambient)]), // C2 CloseRoot
            step(false, vec![]),                                                          // C0 Hold
            step(false, vec![cand(2, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]), // C6 再开仓
            step(true, vec![cand(3, 0, VoiceSide::Short, 1, sell(1), Vertical::Ambient)]), // C1 RiskExit ≻ P2
            step(false, vec![]),                                                          // C0 Hold（已空仓）
        ];
        let out = run_voice(empty_voice(0), &steps);
        assert_eq!(out.len(), steps.len(), "每时刻恰一枚裁决（无缺裁决时刻）");
        let want = vec![
            (ChannelId::Cj(6), ChannelDecision::Open),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
            (ChannelId::Cj(2), ChannelDecision::Exit(ExitType::CloseRoot)),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
            (ChannelId::Cj(6), ChannelDecision::Open),
            (ChannelId::Cj(1), ChannelDecision::Exit(ExitType::RiskExit)),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
        ];
        assert_eq!(out, want, "完整裁决序列逐时刻断言");
    }

    /// 端到端 S2 三类分支：开仓 → 三类反向 ⟹ ReduceCore（腿关闭，**非重复平仓**：后续反向
    /// 候选无腿可关，不产第二枚 close 裁决）→ slot 已空，sell1 候选按 P6 判据（空仓 ∧ 本级
    /// 可交易候选，interp 规则3 slot 语义，无方向约束）落 C6 开空头腿 → 空事件 ⟹ 显式 Hold。
    #[test]
    fn end_to_end_reduce_core_then_no_duplicate_close() {
        let steps = vec![
            step(false, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)]),
            step(false, vec![cand(1, 0, VoiceSide::Short, 3, sell(3), Vertical::Ambient)]),
            step(false, vec![cand(2, 0, VoiceSide::Short, 1, sell(1), Vertical::Ambient)]),
            step(false, vec![]),
        ];
        let out = run_voice(empty_voice(0), &steps);
        assert_eq!(
            out,
            vec![
                (ChannelId::Cj(6), ChannelDecision::Open),
                (ChannelId::Cj(3), ChannelDecision::Exit(ExitType::ReduceCore)),
                (ChannelId::Cj(6), ChannelDecision::Open), // 非重复平仓：落开仓通道
                (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold)),
            ]
        );
    }

    /// 候选消费按 ≺_Θ 序（theta_key 单源）：同刻多候选时反向证书取 ≺_Θ 首个命中者
    /// （一类先于三类 ⟹ CloseRoot 而非 ReduceCore）。
    #[test]
    fn same_instant_multi_candidate_theta_order() {
        let st = holding(0, VoiceSide::Long, Vertical::Ambient);
        // 三类在 gamma 前、一类在后：≺_Θ 以类号升序在前（theta_key 单源），命中一类 ⟹ C2。
        let c3 = cand(0, 0, VoiceSide::Short, 3, sell(3), Vertical::Ambient);
        let c1 = cand(1, 0, VoiceSide::Short, 1, sell(1), Vertical::Ambient);
        assert!(
            theta_key(&c1) < theta_key(&c3),
            "前提：一类候选 ≺_Θ 三类候选（类号键）"
        );
        let (cid, dec) = step_voice(&st, &step(false, vec![c3, c1]));
        assert_eq!(cid, ChannelId::Cj(2));
        assert_eq!(dec, ChannelDecision::Exit(ExitType::CloseRoot));
    }
}
