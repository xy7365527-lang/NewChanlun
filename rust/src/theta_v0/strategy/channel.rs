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
//! ## 槽位定型（本票冻结排序，#149/#150 只填谓词不动槽位）
//!
//! 票面优先级链「风险强平 ≻ 本级证书平仓（S2 二分 CloseRoot/ReduceCore）≻ 短差开启 ≻ 开仓
//! ≻ 记录/加仓 ≻ Hold」展开为 8 槽：
//!
//! | 槽 | 谓词 | 裁决 | 本票状态 |
//! |----|------|------|---------|
//! | P1 | 风险强平 | `Exit(RiskExit)` | 真实装（`force_flat` 注入，与 `KThetaRiskGate.force_flat` 同源） |
//! | P2 | 本级证书平仓·S2→根清仓 | `Exit(CloseRoot)` | 真实装（[`reverse_exit_type`] 单源） |
//! | P3 | 本级证书平仓·S2→减核心 | `Exit(ReduceCore)` | 真实装（同上） |
//! | P4 | 短差平仓 | `Exit(CloseShortDiff)` | **占位恒 false**（#149） |
//! | P5 | 短差开启 | `OpenShortDiff` | **占位恒 false**（#149） |
//! | P6 | 开仓 | `Open` | 真实装 |
//! | P7 | 记录 | `Record` | **占位恒 false**（#150） |
//! | P8 | 加仓 | `AddPosition` | **占位恒 false**（#150） |
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
//! - 短差谓词真值（#149）、记录/加仓谓词真值（#150）：槽位已定型，谓词恒 false。
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
//! 消费者）；本模块是 #147 出场通道链（P1–P8，含 #149/#150 占位槽）的**声部级解释器**，
//! 两链谓词编号语义不同，各锚各的契约，不互为镜像。

use super::super::classifier::recursive_tower::ElementId;
use super::coverage::Vertical;
use super::exec::reverse_signal;
use super::interp::{reverse_exit_type, theta_key, ActiveLeg, Candidate, ExitType};
use super::voice::VoiceSide;

/// 通道谓词数 m=8（P_1..P_8，issue #147 槽位定型）。
pub const CHANNEL_M: usize = 8;

/// 通道谓词向量 `P ∈ {0,1}^8`（可重叠——多分量可同时为 true）。
///
/// 字段序 = 优先级序（P1 最先）。P4/P5/P7/P8 为占位槽：本票任何生产推导路径
/// （[`voice_predicates`]）恒不置位，仅互斥化层（[`first_match`]）全域定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelPredicates {
    /// P1：风险强平 RiskExit（`KThetaRiskGate.force_flat` 同源注入）。
    pub risk_exit: bool,
    /// P2：本级证书平仓·S2 二分→CloseRoot（一/二类反向 = 根清仓）。
    pub cert_close_root: bool,
    /// P3：本级证书平仓·S2 二分→ReduceCore（三类反向 = 核心仓减仓）。
    pub cert_reduce_core: bool,
    /// P4：短差平仓（**占位恒 false**，#149 实装谓词）。
    pub short_diff_close: bool,
    /// P5：短差开启（**占位恒 false**，#149 实装谓词）。
    pub short_diff_open: bool,
    /// P6：开仓（本声部 slot 空 ∧ 存在本级可交易候选）。
    pub open_entry: bool,
    /// P7：记录（**占位恒 false**，#150 记录桶）。
    pub record: bool,
    /// P8：加仓（**占位恒 false**，#150）。
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
    /// C5：短差开启（本票占位通道，不可达）。
    OpenShortDiff,
    /// C6：开仓。
    Open,
    /// C7：记录（本票占位通道，不可达）。
    Record,
    /// C8：加仓（本票占位通道，不可达）。
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

/// 声部状态（一个声部 = 一个 (level, 持仓腿) slot）。
#[derive(Debug, Clone, Copy)]
pub struct VoiceState {
    /// 本声部级别 ℓ（谓词只消费本级候选——「本级证书平仓」）。
    pub level: u32,
    /// 当前持仓腿（None = 空仓 slot）。
    pub leg: Option<ActiveLeg>,
    /// 持仓腿入场角色垂直轴（入场固定，[`reverse_exit_type`] 第一参；空仓时无效值 Ambient）。
    pub entry_v: Vertical,
}

/// 单时刻声部输入（事件）。
#[derive(Debug, Clone)]
pub struct VoiceStepInput {
    /// P1 判据：风险强平（`KThetaRiskGate.force_flat` 同源）。
    pub force_flat: bool,
    /// 当步候选集（本函数内按 ≺_Θ 排序后 first-match 消费；跨级候选被本级过滤）。
    pub candidates: Vec<Candidate>,
}

/// 从声部状态 + 当步输入推导通道谓词向量（生产推导路径，占位槽 P4/P5/P7/P8 恒 false）。
///
/// 判据（全部单源复用）：
/// - P1 = `force_flat`。
/// - P2/P3 = 持仓 ∧ 存在本级反向证书候选（[`reverse_signal`]，≺_Θ 序首个命中），
///   经 [`reverse_exit_type`]`(entry_v, trigger_class)` S2 二分：CloseRoot→P2、ReduceCore→P3。
///   （返回 CloseShortDiff 属 P4 域 = #149 有效域，本票占位不置位。）
/// - P6 = 空仓 ∧ 存在本级可交易候选（dir≠Flat ∧ 有类）。
pub fn voice_predicates(state: &VoiceState, input: &VoiceStepInput) -> ChannelPredicates {
    let mut p = ChannelPredicates { risk_exit: input.force_flat, ..ChannelPredicates::default() };
    // P2/P3：本级证书平仓——持仓 ∧ ≺_Θ 序首个本级反向证书候选，S2 二分经单源 reverse_exit_type。
    if let Some(leg) = &state.leg {
        if let Some(c) = find_reverse(state.level, leg.dir, &input.candidates) {
            match reverse_exit_type(state.entry_v, c.bsp_class) {
                ExitType::CloseRoot => p.cert_close_root = true, // P2
                ExitType::ReduceCore => p.cert_reduce_core = true, // P3
                // CloseShortDiff 属 P4 域（#149 有效域）——本票占位槽恒 false，不置位。
                ExitType::CloseShortDiff => {}
                ExitType::RiskExit | ExitType::Hold => {
                    unreachable!("reverse_exit_type 只产三 close 枚举")
                }
            }
        }
    } else if find_open(state.level, &input.candidates).is_some() {
        // P6：开仓——空仓 slot ∧ 存在本级可交易非 ShortDiff 角色候选（ShortDiff 角色=P5 占位域）。
        p.open_entry = true;
    }
    // P4/P5/P7/P8：占位槽（#149/#150），本票恒 false——不存在置位路径。
    p
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

/// ≺_Θ 序首个本级可开仓候选（可交易 ∧ 非 ShortDiff 角色——短差开启属 P5 占位域，#149）。
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
/// - `Exit(RiskExit|CloseRoot|ReduceCore)` ⟹ 腿关闭（对齐 interp 规则2：被反向命中的腿
///   入 𝒟_x；ReduceCore 减核心在腿粒度同为关闭——本票腿即最小持仓单元）。
/// - `Open` ⟹ 以触发候选建腿（entry_v = 候选角色垂直轴，入场固定）。
/// - `Exit(Hold)` ⟹ 状态不变。其余通道本票不可达。
pub fn run_voice(
    initial: VoiceState,
    steps: &[VoiceStepInput],
) -> Vec<(ChannelId, ChannelDecision)> {
    let mut state = initial;
    let mut out = Vec::with_capacity(steps.len());
    for input in steps {
        let (cid, dec) = step_voice(&state, input);
        state = advance(state, input, dec);
        out.push((cid, dec));
    }
    out
}

/// 状态转移（不可变：产新 state，不 mutate 旧值）。全定义 match（占位通道也有确定转移）。
fn advance(state: VoiceState, input: &VoiceStepInput, dec: ChannelDecision) -> VoiceState {
    match dec {
        // 出场：腿关闭（interp 规则2 语义——被命中腿入 𝒟_x；ReduceCore 在腿粒度同为关闭）。
        ChannelDecision::Exit(
            ExitType::RiskExit | ExitType::CloseRoot | ExitType::ReduceCore | ExitType::CloseShortDiff,
        ) => VoiceState { leg: None, entry_v: Vertical::Ambient, ..state },
        // 显式 Hold：状态不变。
        ChannelDecision::Exit(ExitType::Hold) => state,
        // 开仓：以 ≺_Θ 首个开仓候选建腿（entry_v = 候选角色垂直轴，入场固定）。
        ChannelDecision::Open => {
            let c = find_open(state.level, &input.candidates)
                .expect("C6 命中 ⟹ P6 真 ⟹ 开仓候选存在（voice_predicates 同判据）");
            VoiceState { leg: Some(leg_from_candidate(c)), entry_v: c.role.v, ..state }
        }
        // 占位通道（本票不可达；#149/#150 实装真转移）：状态不变。
        ChannelDecision::OpenShortDiff | ChannelDecision::Record | ChannelDecision::AddPosition => {
            state
        }
    }
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
    use super::super::coverage::{Dir, GradeRel, Horizontal, OperationRole};
    use super::super::super::types::BspBits;

    // ── 构造器 ────────────────────────────────────────────────────────────

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
        VoiceState { level, leg: Some(leg(level, dir)), entry_v }
    }
    fn empty_voice(level: u32) -> VoiceState {
        VoiceState { level, leg: None, entry_v: Vertical::Ambient }
    }

    fn step(force_flat: bool, candidates: Vec<Candidate>) -> VoiceStepInput {
        VoiceStepInput { force_flat, candidates }
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

    // ── 声部级谓词推导（S2 二分 + 占位槽 + C0 兜底）──────────────────────

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

    /// 占位槽护栏：本票任何生产推导路径恒不置位 P4/P5/P7/P8（场景电池覆盖持仓/空仓/
    /// 风险/反向/同向/ShortDiff 角色候选）。
    #[test]
    fn placeholder_predicates_never_fire_this_ticket() {
        let scenarios: Vec<(VoiceState, VoiceStepInput)> = vec![
            (empty_voice(0), step(false, vec![])),
            (empty_voice(0), step(true, vec![cand(0, 0, VoiceSide::Long, 1, buy(1), Vertical::Ambient)])),
            // ShortDiff 角色开仓候选：短差开启属 #149，P5 不置位（本票走 P6 开仓通道也不允许——
            // 字面边界：短差开启谓词恒 false；ShortDiff 角色候选不属 P6 开仓域 ⟹ C0）。
            (empty_voice(0), step(false, vec![cand(0, 0, VoiceSide::Short, 1, sell(1), Vertical::ShortDiff)])),
            (holding(0, VoiceSide::Long, Vertical::Ambient), step(false, vec![cand(0, 0, VoiceSide::Short, 3, sell(3), Vertical::Ambient)])),
            (holding(0, VoiceSide::Long, Vertical::Ambient), step(true, vec![])),
        ];
        for (st, input) in &scenarios {
            let p = voice_predicates(st, input);
            assert!(
                !p.short_diff_close && !p.short_diff_open && !p.record && !p.add_position,
                "占位槽 P4/P5/P7/P8 本票恒 false：state={st:?} input={input:?} ⟹ {p:?}"
            );
        }
    }

    /// ShortDiff 角色候选不落 P6（短差开启=P5 占位域）⟹ 显式 Hold，非误开仓。
    #[test]
    fn short_diff_role_candidate_not_opened_via_p6() {
        let input = step(false, vec![cand(0, 0, VoiceSide::Short, 1, sell(1), Vertical::ShortDiff)]);
        assert_eq!(
            step_voice(&empty_voice(0), &input),
            (ChannelId::C0, ChannelDecision::Exit(ExitType::Hold))
        );
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
