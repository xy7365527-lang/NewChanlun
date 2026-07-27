use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  §9 环7：目标头寸 p̃_{t+1} → 全定义策略 π_Θ → 唯一订单 O_{t+1}
//        （spec §15 P12 line 717-756：𝒦_Θ / J_x / LexArgmin / Schedule_Θ；
//         定理 spec §16 P13 line 795：∀x ∃! O_{t+1}=π_Θ(x)，七链 **环7** rust 兑现）
//
//  π_Θ(x) = Schedule_Θ[ LexArgmin_{p∈𝒦_Θ(x)} J_x(p) − p_t ]   （spec line 756 方框）
// ════════════════════════════════════════════════════════════════════════════

/// J_x 目标函数权重（spec §15 line 740：`J_x = ‖p−p̃‖²_W + λ·Cost_x(p) + ν·RiskPenalty_x(p)`）。
///
/// 三权重全是 **Θ_risk 参数**（formalization-validity-domain，**非缠论可导**）：
/// - `w`：加权范数权重 w_a（spec line 744 `w_a>0` ⟹ 范数正定 ⟹ 主键跟踪误差严格凸）。
/// - `lambda`：成本倍数 λ（**复用** [`RiskConfig`] 的 `kappa` 成本倍数 κ，不引入新参数）。
/// - `nu`：风险罚倍数 ν（**复用** [`RiskConfig`] 的 `rho` 单声部风险 ρ，不引入新参数）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PiThetaWeights {
    /// w_a > 0：加权范数权重（主键跟踪误差，spec line 744）。
    pub w: f64,
    /// λ：成本倍数（次键，复用 RiskConfig.kappa）。
    pub lambda: f64,
    /// ν：风险罚倍数（第三键，复用 RiskConfig.rho）。
    pub nu: f64,
}

impl PiThetaWeights {
    /// 从 [`RiskConfig`] 派生 J_x 权重（**复用**既有 Θ_risk 参数，no-声明膨胀不引新参数）。
    ///
    /// `w=1.0`（范数主键权重，满足 spec line 744 `w_a>0` 正定要求）；`λ=κ`（`RiskConfig.kappa`
    /// 成本倍数）；`ν=ρ`（`RiskConfig.rho` 单声部风险）。
    pub fn from_risk(risk: &RiskConfig) -> Self {
        PiThetaWeights { w: 1.0, lambda: risk.kappa, nu: risk.rho }
    }
}

/// J_x 定点放大因子（浮点 J_x 分量 → [`JThetaKey`] i64 字典序键，bit-exact 确定比较）。
///
/// 对齐 intent.rs `JThetaKey` 文档「浮点 J_Θ 值乘固定缩放因子后取整，避免浮点比较的非确定性」。
const J_SCALE: f64 = 1_000.0;

/// 净持仓 flat（零仓）判定阈（lot 对齐下 |p|<此阈即视为空仓）。
const FLAT_EPS: f64 = 1e-9;

/// 定点放大 + 溢出钳制（浮点 J_x 分量 → i64 字典序键，**无 panic**）。
///
/// `(x·J_SCALE).round()` 钳到 i64 值域（边界条件：p 有界于 ±cap、权重有限 ⟹ 常规配置不触钳制；
/// 极端 base_units 触上界时钳到 i64::MAX，保字典序方向不翻转，非 bug）。
pub(crate) fn scale_key(x: f64) -> i64 {
    (x * J_SCALE).round().clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

/// 净持仓 lot 对齐（向最近 lot 取整；`lot≥1` 由调用方 `RiskConfig.default_lot.max(1)` 保证）。
pub(crate) fn lot_round(p: f64, lot: f64) -> f64 {
    (p / lot).round() * lot
}

/// 𝒦_Θ 净持仓可行幅度**无量纲**上限 `γ̄`（**方案A协变**，spec §3/§5/§6 钦定，CovariantCapital.lean GREEN）。
///
/// 返回 `risk.gamma.abs()`（= `γ̄`，无量纲比例上限，`b_j(S_k Θ) = a_k^{d_j} b_j(Θ)` 边界协变，
/// d_j=1 名义上限）。绝对上限由调用方 [`pi_theta_position`] 完成：`cap = U_ℓ · γ̄`
/// （`U_ℓ = base_units`，runner 注入的协变资本单位，满足 `U_{ℓ+k}(S_k x) = a_k U_ℓ(x)`）。
///
/// **方案A落地**（2026-06-28-absolute-capital-equivariance-resolution §6 钦定）：
/// `γ̄` 是无量纲 Θ 参数（`S_k Θ = Θ` 强形式，不含特权绝对尺度）；约束 `|p| ≤ U_ℓ · γ̄` 随
/// `a_k` 协变缩放（d_j=1 名义上限）——绝对资本特权尺度已消除，三阶段资本比例化完成。
///
/// **d_j 默认（对齐 CovariantCapital.lean §6）**：名义上限 d_j=1（本函数）；杠杆比 d_j=0
/// （[`risk::leverage_ok`]，无量纲比值天然满足）；跟踪误差 `w·(p−p̃)²` d_j=2（[`j_theta_key`]）。
///
/// **★诚实有效域**：美元级杠杆/保证金（[`risk::leverage_ok`]）仍需 runner 注入 price/equity。
/// 实盘 Nautilus 路径：`a_t = U_ℓ · ā_t`（`ā_t = p*`，`U_ℓ = base_units`，runner 层还原绝对值）。
pub(crate) fn feasible_net_cap(risk: &RiskConfig) -> f64 {
    risk.gamma.abs()
}

/// ★M4 级别级协变 cap `cap_ℓ = w_ℓ·γ̄·U_ℓ`（multi-level-native-execution-design-20260719
/// §D M4；`level_risk` 模块头「对偶统一声明」）。
///
/// **协变分解守恒**（票体验收「𝒦_Θ 协变 cap 按级别分解后仍满足」的机器判据）：账户层总 cap
/// `= γ̄·U_ℓ`（[`feasible_net_cap`]×`base_units`，同 [`feasible_lex_candidates`] 用的量）；
/// 本函数只多乘一个 `w_ℓ∈[0,1]` 因子——`cap_ℓ` 与总 cap 共用同一 `U_ℓ` 协变缩放（方案A，
/// `feasible_net_cap` 文档同纪律），故 `cap_ℓ` 本身**逐级协变**：`a_k` 缩放资本时
/// `cap_ℓ(S_k x) = a_k·cap_ℓ(x)`，与总 cap 的协变律同构，非另立一套。
///
/// `Σ_ℓ cap_ℓ = (Σ_ℓ w_ℓ)·γ̄·U_ℓ ≤ γ̄·U_ℓ`（由
/// [`super::super::level_risk::level_weights_sum_le_one`] 保证 `Σw_ℓ≤1`）——分解后的级别帽之和
/// **不超过**未分解的账户层总 cap，这正是「分解后仍满足」的代数内容（L0，见测试
/// `level_cap_decomposition_never_exceeds_account_cap`）。
pub(crate) fn level_cap(level: u32, base_units: f64, risk: &RiskConfig) -> f64 {
    feasible_net_cap(risk) * super::super::level_risk::level_weight(level, risk) * base_units.abs()
}

/// ★M4 级别级风险帽实际施加点（`risk.enforce_level_cap` 门禁，G7 `apply_gross_cap` 同款
/// 模式）：把 [`super::super::level_order::LevelOrderLedger::regate`] 产出的门控结构基准
/// `gated_ℓ` 逐级 clamp 到 `[-cap_ℓ, +cap_ℓ]`（[`level_cap`]）。
///
/// 施加点纪律（§F③ 域分离 + `level_risk` 模块头「禁双重定价」）：本函数只读**已经**按级别
/// 聚合完成的 `gated`（`net_ℓ` 之和，depth_weight 早已沉淀在其中），**不**拆解单条 leg 的深度
/// 构成——level_weight 与 depth_weight 因此不会对同一块资金重复定价。零项保留（级别封闭
/// 可读，与 [`super::super::level_order::LevelOrderPlan::deltas`] 同纪律）。
///
/// `risk.enforce_level_cap=false`（default）⟹ 调用方**不得**调用本函数（应直接跳过），
/// 而不是传入空 `level_weights` 期望本函数自然退化——空表会把 `cap_ℓ` 恒裁到 0，那是「全部
/// 级别禁止持仓」而非「级别帽未启用」，两者语义相反，门禁必须在调用方（`fill.rs`）而非本函数。
///
/// ★#351 MED-1 补课：`level_weights` 是唯一权威判据是 doc 声明（`level_risk.rs` 模块头
/// `level_weights_sum_le_one`），但接线前从未被生产路径实际调用——误配 `Σw_ℓ>1` 时裁剪照常
/// 施加，`Σcap_ℓ` 静默突破账户层总 cap（#349 影子评审 MED-1）。本函数是全仓**唯一**施加级别帽
/// 的生产入口（`level_risk.rs` 模块头「施加点严格限定」纪律），故校验必须钉死在这里，且用
/// `assert!`（非 `debug_assert!`）——这是**配置校验**（外部可错的输入），不是「代码正确则恒真」
/// 的内部不变量，必须在 release 构建下同样生效，否则生产环境的误配不会被拦截。
///
/// ★#351 MED-2 补课：本函数也用于 `fill.rs` 归因缩放后的**二次裁剪**（见调用点「帽后重越」
/// 纪律），此时输入可能含 [`super::super::level_attrib::LEVEL_ACCOUNT_RESIDUAL`] 残差桶——该桶无级别
/// 身份（不对应任何 `w_ℓ`），若按越界索引取权重会被 `level_weight` 判 0 从而错误清零，故本函数
/// 显式跳过残差桶（原样透传，不裁剪；同「不得伪造级别身份」纪律）。
pub(crate) fn clamp_levels_to_weighted_cap(
    gated: &[(u32, i64)],
    base_units: f64,
    risk: &RiskConfig,
) -> Vec<(u32, i64)> {
    assert!(
        super::super::level_risk::level_weights_sum_le_one(risk),
        "MED-1（#351）：Σw_ℓ={} > 1，级别帽配置违规——本函数是全仓唯一权威施加点，拒绝在违规\
         配置下裁剪，避免 Σcap_ℓ 静默突破账户层总 cap（level_risk.rs::level_weights_sum_le_one）",
        super::super::level_risk::level_weights_sum(risk)
    );
    gated
        .iter()
        .map(|&(lvl, q)| {
            if lvl == super::super::level_attrib::LEVEL_ACCOUNT_RESIDUAL {
                return (lvl, q); // 残差桶无级别身份，不参与级别帽（MED-2：禁伪造级别身份同纪律）
            }
            let cap = level_cap(lvl, base_units, risk);
            let cap_units = cap.floor().max(0.0) as i64;
            (lvl, q.clamp(-cap_units, cap_units))
        })
        .collect()
}

/// **𝒦_Θ 风控约束门（close_pred 折入可行集，非第二决策出口，§16 单一决策出口）**。
///
/// ## Q2 编排者裁定（close_pred 风控折进 𝒦_Θ）
///
/// reference §16 钦定**单一决策出口** `π_Θ(x)=Schedule_Θ[LexArgmin_{p∈𝒦_Θ}J_x(p)−p_t]`。退出
/// 不能有第二出口——故 close_pred 的**风控项（stop/risk）折入 𝒦_Θ 约束门**：风控触发 ⟹ 𝒦_Θ
/// 可行净持仓收窄（`force_flat`→{0}；`stop_long`→禁净多仓；`stop_short`→禁净空仓），[`lex_argmin`]
/// 在收窄集上自然产 p*→平/减——退出仍走**唯一决策出口**（p*），非独立 exit 订单。
///
/// ## close_pred 契约锚保留（no-patch-keep-primitive）
///
/// 本门由 [`super::super::exec::close_pred`] 计算（runner discharge stop/risk 读出 → close_pred → 本门），
/// **不删** close_pred 原语。`reverse_signal` 项**不**入本门——反向信号关闭活动腿走 [`interp::interpret`]
/// 的 𝒟_x（活动集递归腿级决策，那才是 §16 的腿级单出口）；本门只承载 stop/risk（账户/价格层风控）。
///
/// ## 认识论 L0（formalization-validity-domain 231号）
/// 给定风控读出后，约束门是 𝒦_Θ 区间收窄的布尔代数（确定）。风控读出本身（`stop_hit`/
/// `global_risk_close`）由 runner discharge（账户层运行时输入 E_t/价格触及，**非缠论可导**）。
// no_increase_cap: Option<f64> ⟹ 不能 derive Eq（f64 非 Eq）；PartialEq 足够（无 HashSet/Ord 用途）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KThetaRiskGate {
    /// GlobalRiskClose（Insolvent/Liquidation，P1 最高优先级）⟹ 𝒦_Θ={0}（强制全平）。
    pub force_flat: bool,
    /// 多头风控触发（结构止损触及，`close_pred(stop_long)`）⟹ 𝒦_Θ 禁净多仓（hi_cap=0）。
    pub stop_long: bool,
    /// 空头风控触发（结构止损触及，`close_pred(stop_short)`）⟹ 𝒦_Θ 禁净空仓（lo_cap=0）。
    pub stop_short: bool,
    /// M2/M3（Deleverage/CloseOnly）净幅上限=当前 |net|（margin-design §2.8；单位同 `cap`，lot 幅度）。
    /// `None`=不约束（M0/M1/M4 或未接保证金）；`Some(c)`=净持仓幅度不得超 c（禁增仓，真改订单流）。
    pub no_increase_cap: Option<f64>,
}

impl KThetaRiskGate {
    /// 无约束门（𝒦_Θ=[−cap,+cap] 全开，风控未触发）——执行层默认 + 既有 π_Θ 测试用。
    pub fn open() -> Self {
        KThetaRiskGate { force_flat: false, stop_long: false, stop_short: false, no_increase_cap: None }
    }

    /// 应用约束门到对称 cap，产 `(lo_cap, hi_cap)` 幅度（`force_flat` 优先收到 {0}）。
    fn caps(&self, cap: f64) -> (f64, f64) {
        if self.force_flat {
            return (0.0, 0.0); // GlobalRiskClose ⟹ 𝒦_Θ={0}（净持仓只能 0）
        }
        // M2/M3：净幅上限压到当前 |net|（margin-design §2.8，禁开新增仓；stop 仍各自禁一侧）。
        let cap = match self.no_increase_cap {
            Some(c) => cap.min(c.max(0.0)),
            None => cap,
        };
        let hi = if self.stop_long { 0.0 } else { cap }; // 禁净多 ⟹ 上限 0
        let lo = if self.stop_short { 0.0 } else { cap }; // 禁净空 ⟹ 下限 0
        (lo, hi)
    }

    /// 供 DC-E 与标准 π 共用同一 𝒦_Θ 边界；返回有符号净持仓区间 `[lo, hi]`。
    pub(crate) fn position_bounds(&self, cap: f64) -> (f64, f64) {
        let (lo_mag, hi) = self.caps(cap);
        (-lo_mag, hi)
    }

    /// 从 `anchor` 沿给定方向最多还能移动的整数单位；DC-E 以此作为 KΘ 上界。
    pub(crate) fn delta_capacity_units(
        &self,
        cap: f64,
        anchor: f64,
        side: VoiceSide,
    ) -> u64 {
        let (lo, hi) = self.position_bounds(cap);
        let room = match side {
            VoiceSide::Long => hi - anchor,
            VoiceSide::Short => anchor - lo,
            VoiceSide::Flat => 0.0,
        };
        room.max(0.0).floor() as u64
    }

    pub(crate) fn clamp_position(&self, cap: f64, position: f64) -> f64 {
        let (lo, hi) = self.position_bounds(cap);
        position.max(lo).min(hi)
    }
}

/// 构造有限可行集 𝒦_Θ(x) 的 **LexArgmin 代表点**（spec §15 line 721-736 八约束 + line 725 `𝒦_Θ≠∅`）。
///
/// 𝒦_Θ = lot 对齐的净持仓 `p ∈ [−cap,+cap]`（有限网格，spec P13 假设10「𝒦_Θ 非空且有限」）。返回
/// **字典序 argmin 必含的代表点**（凸跟踪目标在 lot 网格的精确有限表示，**非近似**）：
/// - `clamp(p̃)` 的 floor/ceil lot 点（**bracketing**：凸二次跟踪项 `w(p−p̃)²` 在 lot 网格的全局
///   最小点必是 `clamp(p̃)` 的两个相邻 lot 点之一）。
/// - 端点 `±hi`（杠杆/资本 cap binding 时的最优；`hi`=最大 lot 对齐幅度 `≤cap`）。
/// - 安全锚 `0`（**𝒦_Θ≠∅ 的构造性非空见证**，spec line 725 硬前提 / Lean `feasible_nonempty`）
///   + `clamp(lot(p_t))`（保持当前仓位的可行点）。
///
/// 全 lot 对齐 ∈[−hi,hi]，升序去重，`grid_index`=升序位次（**固定字典序平局规则**，spec line 791
/// 假设11 ⟹ [`JThetaKey`] 单射 ⟹ p* 唯一）。
///
/// ★凸性精确性（formalization-validity-domain，**非近似**）：主键跟踪误差 `w(p−p̃)²`（w>0）在 lot
/// 离散区间的全局最小在 `clamp(p̃)` 相邻 lot 点取得；二点等距（p̃ 恰在 lot 中点）⟹ 跟踪并列 ⟹ 次键
/// 成本破并列——两点均在本集 ⟹ **本代表集上 LexArgmin = 全 𝒦_Θ 网格 LexArgmin（精确相等）**。
pub(crate) fn feasible_candidates(p_tilde: f64, p_t: f64, lo_cap: f64, hi_cap: f64, lot: f64) -> Vec<(f64, i64)> {
    // 非对称 cap（𝒦_Θ 风控约束门 [`KThetaRiskGate::caps`] 注入）：hi_cap=净多上限、lo_cap=净空上限
    // （幅度）。对称全开时 lo_cap=hi_cap=cap（退化为旧 [−hi,hi]）；force_flat ⟹ 两者 0 ⟹ 𝒦_Θ={0}。
    let hi = ((hi_cap / lot).floor() * lot).max(0.0); // 净多最大 lot 对齐幅度 ≤ hi_cap
    let lo = -((lo_cap / lot).floor() * lot).max(0.0); // 净空最大 lot 对齐幅度 ≤ lo_cap
    let clamp = |x: f64| x.max(lo).min(hi);
    let pc = clamp(p_tilde);
    let floor_pt = clamp((pc / lot).floor() * lot);
    let ceil_pt = clamp((pc / lot).ceil() * lot);
    let anchor_pt = clamp(lot_round(p_t, lot));
    let mut pts = vec![floor_pt, ceil_pt, hi, lo, 0.0, anchor_pt];
    pts.sort_by(|a, b| a.partial_cmp(b).expect("有限 f64 候选可序"));
    pts.dedup_by(|a, b| (*a - *b).abs() < lot * 0.5); // 同 lot 点去重（保升序首个）
    pts.into_iter()
        .enumerate()
        .map(|(i, p)| (p, i as i64))
        .collect()
}

/// J_x(p) 的字典序键 [`JThetaKey`]（spec §15 line 740-744 三项 → 主/次/三键 + 平局键）。
///
/// - `tracking_err` = `w·(p−p̃)²`（‖p−p̃‖²_W 净额降维，**主键**——优先贴合目标头寸 p̃）。
/// - `trade_cost` = `λ·|p−p_t|`（Cost_x(p) 换手成本代理，**次键**）。
/// - `risk_penalty` = `ν·|p|`（RiskPenalty_x(p) 毛敞口 |p| 代理，**第三键**）。
/// - `turnover` = 0（spec §15 J_x 仅三项无独立换手项；[`JThetaKey`] 第四键留 0，换手已并入成本）。
/// - `grid_index`（升序位次，**固定平局键**，spec line 791 假设11）。
pub(crate) fn j_theta_key(
    p: f64,
    p_tilde: f64,
    p_t: f64,
    weights: PiThetaWeights,
    grid_index: i64,
) -> JThetaKey {
    JThetaKey {
        tracking_err: scale_key(weights.w * (p - p_tilde).powi(2)),
        trade_cost: scale_key(weights.lambda * (p - p_t).abs()),
        risk_penalty: scale_key(weights.nu * p.abs()),
        turnover: 0,
        grid_index,
    }
}

/// **最优头寸 p\* = LexArgmin_{p∈𝒦_Θ(x)} J_x(p)**（spec §15 line 748 方框）。
///
/// 在有限可行代表集 [`feasible_candidates`]（𝒦_Θ 的精确 LexArgmin 表示）上，用 **真字典序**
/// [`lex_argmin`]（intent.rs，对齐 Lean `Origin.LexArgmin`）选 J_x 字典序最小净持仓。
///
/// 边界条件：𝒦_Θ 恒含安全锚 0（非空，spec line 725）⟹ [`lex_argmin`] 必返回 `Some`；防御性 `None`
/// （理论不可达）归 0.0（flat 安全侧）。p̃ 在 cap 内 ⟹ p\*=lot(p̃)（跟踪主键）；p̃ 超 cap ⟹ p\*=±hi
/// （杠杆/资本 cap binding）。
pub fn pi_theta_position(
    p_tilde: f64,
    p_t: f64,
    base_units: f64, // U_ℓ：runner 注入的协变资本单位（随级别 a_k 缩放；方案A）
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate, // 𝒦_Θ 风控约束门（close_pred 折入，非第二出口；全开=open()）
) -> f64 {
    lex_argmin(&feasible_lex_candidates(p_tilde, p_t, base_units, risk, weights, gate)).unwrap_or(0.0)
}

/// 𝒦_Θ 可行代表集 → `Vec<LexCandidate<f64>>`（J_Θ 键已挂），供 [`lex_argmin`]（p_star 选址）与
/// [`lex_argmin_top_k`]（R5-a 诊断 top-k）共用——**单一构造源**保证 argmin 与 top-k 同候选集
/// （top-k 的第一名 ≡ argmin 选址，bit-exact）。
///
/// 抽取自 [`pi_theta_position`]（纯重构，候选构造逻辑逐字不变）。pub(crate) 供
/// [`pi_theta_step_traced`] 在 dump 启用路径复用算 top-k。
pub(crate) fn feasible_lex_candidates(
    p_tilde: f64,
    p_t: f64,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
) -> Vec<LexCandidate<f64>> {
    let lot = risk.default_lot.max(1) as f64;
    // 方案A：γ̄ = feasible_net_cap(risk) 无量纲；绝对上限 cap = U_ℓ · γ̄（d_j=1 协变缩放）
    let cap = feasible_net_cap(risk) * base_units.abs();
    // 𝒦_Θ 风控约束门：force_flat→{0}，stop_long→禁净多，stop_short→禁净空（Q2 折入可行集）。
    let (lo_cap, hi_cap) = gate.caps(cap);
    feasible_candidates(p_tilde, p_t, lo_cap, hi_cap, lot)
        .into_iter()
        .map(|(p, gi)| LexCandidate {
            control: p,
            key: j_theta_key(p, p_tilde, p_t, weights, gi),
        })
        .collect()
}

/// **Schedule_Θ(p\* − p_t) → 唯一订单 O_{t+1}**（spec §15 line 752 方框；八约束之 **订单执行约束**）。
///
/// 排程净持仓增量 `Δ = p\* − p_t`（lot 对齐）为单净额订单（Nautilus 净额账户单持仓）。这是**全函数**
/// （spec §16 假设12「Schedule_Θ 是函数」）——无交易时返回 `Hold`/`Wait`（qty=0），保 ∀x ∃! O_{t+1}：
/// - `Δ=0`：`Wait`（当前空仓）/ `Hold`（当前持仓），qty=0（`qty≤0` 不交易，types.rs Order 契约）。
/// - 空仓→持仓：`Buy`（p\*>0）/ `Sell`（p\*<0），qty=|p\*|。
/// - 持仓→空仓：`Close`，qty=|p_t|。
/// - 同号增持：`Add`；同号减持：`Reduce`，qty=|Δ|。
/// - 反号穿零（净反转）：`Buy`（p\*>0）/ `Sell`（p\*<0），qty=|Δ|（单净订单跨零）。
///
/// `exec_index`：执行延迟后的成交 bar（runner 传入，对齐 types.rs `Order.exec_index` / spec exec 延迟）。
pub fn schedule_order(p_star: f64, p_t: f64, exec_index: usize) -> Order {
    let delta = p_star - p_t;
    let qty = delta.abs().round() as i64;
    let flat_now = p_t.abs() < FLAT_EPS;
    let flat_next = p_star.abs() < FLAT_EPS;
    let action = if qty == 0 {
        if flat_now {
            StrictAction::Wait
        } else {
            StrictAction::Hold
        }
    } else if flat_now {
        if p_star > 0.0 {
            StrictAction::Buy
        } else {
            StrictAction::Sell
        }
    } else if flat_next {
        StrictAction::Close
    } else if (p_t > 0.0) == (p_star > 0.0) {
        // 同号：幅度增=Add，幅度减=Reduce。
        if p_star.abs() > p_t.abs() {
            StrictAction::Add
        } else {
            StrictAction::Reduce
        }
    } else {
        // 反号穿零（净反转）：方向由 p* 符号定。
        if p_star > 0.0 {
            StrictAction::Buy
        } else {
            StrictAction::Sell
        }
    };
    Order { action, qty, exec_index }
}

/// **全链 π_Θ(x)：买卖点 Γ 入场（环5+6） → 全定义策略 π_Θ（环7） → 唯一订单 O_{t+1}**。
///
/// 串通七链终段（生产入场入口，GAP-5 收口）：
/// 1. **环5+6**（[`coverage_step_classification`]）：`Classification → assemble_gamma(BspPoint) →
///    interpret(ℛ_Θ) → A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore] → p̃_{t+1}`。入场源 = **买卖点 Γ(x)**
///    （候选 `source_index` = `BspPoint.source_index`），**非走势边界** `LeveledMove.start_index`。
/// 2. **环7**（[`pi_theta_position`] + [`schedule_order`]）：`p* = LexArgmin_{p∈𝒦_Θ} J_x(p)` →
///    `O_{t+1} = Schedule_Θ(p*−p_t)`。
///
/// 返回 `(A_{t+1}, p*, O_{t+1})`：新活动集（喂下一 bar interpret 闭环）+ 新净持仓（喂下一 bar 的
/// `p_t`）+ 当前订单。**immutable**：不 mutate 输入。`base_units`=NAV 预算的基准腿仓位（runner 提供）。
///
/// ## 认识论等级（formalization-validity-domain 231号）
/// **L0/L1**（操作语义结构，**非 L2 alpha**）：确定性结构变换（Γ→桶→活动集→p̃→LexArgmin→订单），
/// `cargo test` 通过 = 管线正确性 + π_Θ 唯一订单（∀x ∃! O，spec §16）的**结构**兑现，**不**蕴含实盘
/// alpha（买卖点 v1 全窗 8/8 L3 已否证；本引擎是 M29 element-coverage 的 rust 兑现，盈利由下一步
/// L2/L3 净额回测否证检验）。
///
/// > **结果包六要素**
/// > - **结论**：全定义策略 π_Θ 的 rust 终环——买卖点 Γ 入场 → p̃ → `p*=LexArgmin_{p∈𝒦_Θ}J_x(p)`
/// >   → `O=Schedule_Θ(p*−p_t)`，产唯一订单 `(A_{t+1}, p*, O)`。
/// > - **定义依据**：spec §15 P12（𝒦_Θ⊆𝒫^sep、`𝒦_Θ≠∅`、八约束、J_x 方框 line 740、LexArgmin
/// >   方框 line 748、Schedule_Θ 方框 line 752、π_Θ 方框 line 756）+ §16 P13（∀x ∃! O_{t+1} line 795，
/// >   假设10 𝒦_Θ 非空有限、假设11 LexArgmin 固定平局、假设12 Schedule_Θ 是函数）。输入特征满足：
/// >   p̃ 来自环5+6 唯一活动集（𝒟_x/ℬ_x=ℛ_Θ(Γ(x)) 买卖点路径）；𝒦_Θ 恒含 0（非空）；JThetaKey
/// >   grid_index 单射（破平局）⟹ p* 唯一；Schedule_Θ 全函数 ⟹ O_{t+1} 唯一存在。
/// > - **边界条件**：① `𝒦_Θ≠∅` 由安全锚 0 构造性保证——若 cap<lot 则 `hi=0`，𝒦_Θ={0} 仍非空。
/// >   ② 唯一性依赖 **LexArgmin**（字典序，非普通 argmin）+ grid_index 单射——若仅 argmin 且 J_x
/// >   多最优则唯一性翻转（spec line 761）。③ p̃ 在 cap 内 ⟹ p*=lot(p̃)；超 cap ⟹ p*=±hi（杠杆/
/// >   资本 cap binding 翻转结论）。④ 八约束中 **AncOK/同单位短差/级别自相似/分账本** 在环5+6 上游
/// >   已施于 p̃（𝒦_Θ 经 p̃ 继承，不重复机件）；**手数/订单执行** 在本环施（lot 网格 + Schedule_Θ）；
/// >   **杠杆保证金** 仍需 runner price/equity（[需 runner 注入]）；**三阶段资本** 已方案A协变
/// >   （`cap=U_ℓ·γ̄`，base_units=U_ℓ，runner 按级别注入，d_j=1 名义上限，无特权绝对尺度）。
/// >   ⑤ 净额降维：p 是有符号净持仓（分账本多空腿 q^± 在 net_target_units
/// >   已降维，毛分账本须 hedging 账户，v0 净额，M29 §7 诚实声明）。
/// > - **下游推论**：`(A_{t+1}, p*)` 喂下一 bar（p* 成为下一 `p_t`，A_{t+1} 喂 interpret 闭环）；
/// >   O_{t+1} 喂 runner 净额账本（接 Nautilus 入场/出场点）。GAP-5 收口 ⟹ recognize↔coverage 入场
/// >   源统一为买卖点 Γ（trades-vs-closedloop-disjoint-paths 谱系：入场源接 Γ 连通两路径）。
/// > - **谱系引用**：GAP-5（入场源=买卖点 Γ 非走势边界，本环收口）；MEMORY
/// >   coverage-engine-needs-tower-export-bridge（互斥全定义策略=**买卖点入场**+多级角色/嵌套对冲，
/// >   非每元素覆盖——§7 λ_e 入场已删）；newchanlun-v1-fullwindow-l3-falsified（v1 8/8 否证，本环
/// >   不蕴含 alpha，L0/L1）；no-patch-keep-primitive（删 λ_e 入场组装层 §7，保 §3 区间递归原语）。
/// > - **影响声明**：新增 coverage.rs §9（[`PiThetaWeights`]/[`feasible_net_cap`]/
/// >   [`feasible_candidates`]/[`j_theta_key`]/[`pi_theta_position`]/[`schedule_order`]/本函数）；
/// >   复用 intent.rs（[`JThetaKey`]/[`LexCandidate`]/[`lex_argmin`]）+ RiskConfig（κ/ρ/γ/lot）+
/// >   types.rs（Order/StrictAction）；删除 §7 `coverage_step`（λ_e 走势边界入场，GAP-5）+ 其 3 测试；
/// >   不改 interp.rs/risk.rs/mod.rs/lakefile，coverage.rs 已注册（`pub mod coverage;` mod.rs:43）。
/// >   **方案A实装（本工位）**：`feasible_net_cap` 改为返回无量纲 `γ̄`（`risk.gamma`），
/// >   `pi_theta_position` 计算 `cap = U_ℓ·γ̄`（`U_ℓ=base_units`）——绝对资本协变化完成，
/// >   `[方案A-rust-todo]` 清零；信源：CovariantCapital.lean GREEN（feasibleSet_equivariant 全证）。
#[allow(clippy::too_many_arguments)]
pub fn pi_theta_step(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>], // per-bar 因果塔（639 σ_p=父容器方向；runner 喂前缀重分类塔）
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    voice: &VoiceConfig,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate, // 𝒦_Θ 风控约束门（close_pred 折入；全开=open()）
    protocol: &ProtocolEventSet,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, PiThetaDecision) {
    // 环5+6：买卖点 Γ 入场 → A_{t+1} + p̃（GAP-5：入场源 = BspPoint.source_index 买卖点）。
    // 执行层 σ_p=父容器方向（639；coverage_step_classification 内 assemble_gamma_with_tower 喂因果塔）。
    let (next_active, p_tilde) =
        coverage_step_classification(classification, tower, prev_active, base_units, voice, Some(risk), registry);
    // 环7：p* = LexArgmin J_x（𝒦_Θ，风控门收窄）→ O = Schedule_Θ(p*−p_t)（单一决策出口 §16）。
    let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
    let order = schedule_order(p_star, p_t, exec_index);
    (next_active, p_star, (order, protocol.selected()))
}

/// **工位 K 性能：`pi_theta_step` 用预建 `(elements, candidate_start, gamma)`**（bit-exact ==
/// [`pi_theta_step`]，仅把内部 `coverage_elements_and_gamma_with_tower` 重建替换为 runner 缓存的
/// 预建产物——消除 per-bar 双调建树 + 跨 bar 全前缀重建 O(confirmed)）。
///
/// G4（#134）后委托 [`pi_theta_step_traced`] 丢弃 trace（单源组装，决策路径 bit-exact 不变；
/// trace 构造在活动腿规模 O(|A_t|) 上，实测 |A_t|~9@16K bar，开销可忽略）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn pi_theta_step_prebuilt(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
    config: &VoiceConfig,
    protocol: &ProtocolEventSet,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, PiThetaDecision) {
    let (next_active, p_star, decision, _trace) = pi_theta_step_traced(
        work, gamma, prev_active, p_t, exec_index, base_units, risk, weights, gate, config, registry,
        None, // 旧调用方无 TW 源：P2/P3/P4 不评估（bit-exact 原路径）
        protocol,
    );
    (next_active, p_star, decision)
}

/// G4 typed exit 组合层 trace（#134，裁定4 I_Θ 组合层雏形——G5 #124 升级 I_Θ 时在此层加
/// RiskState/TwState 输入与 tw_event/exit_kind 输出）。
///
/// 记录本步腿级生命周期事件的**原料**（ExitType 判定在消费端 runner ledger builder 做——
/// 需要腿的入场角色，经 [`interp::reverse_exit_type`] 单源判据）：
/// - `closed`：被 interpret 规则2 反向关闭的腿 + 触发候选（一一对应归因）。
/// - `silent_drops`：不在 close 桶但从 active 消失的腿（§13 AncOK 连带剪 / Stale prune）。
/// - `opened`：open 桶候选中**真正准入** `next_active` 的（AncOK 后），携对应新腿。
///   restore 恢复的祖先 carrier 腿不在此列（非信号入场，无 z，不入 ledger）。
///
/// **RiskExit 通道**（#124 P1 落地）：`force_flat`（PDF §7 全互斥 C_1 强平，屏蔽 P2..P10）⟹
/// [`pi_theta_step_traced`] 在解释器上游短路，prev_active 全部经 `risk_exits` 外化为
/// `ExitType::RiskExit`、next_active=∅（幽灵腿堵口，映射设计 §6.7）。RiskExit 无触发候选（非反向
/// 信号），故独立于 `closed`（后者携触发候选喂 [`interp::reverse_exit_type`]）。
#[derive(Debug, Clone, Default)]
pub(crate) struct StepTrace {
    pub closed: Vec<(ActiveLeg, Candidate)>,
    pub silent_drops: Vec<ActiveLeg>,
    pub opened: Vec<(Candidate, ActiveLeg)>,
    /// P1 强平清空的活动腿（force_flat ⟹ RiskExit）——无触发候选，独立通道。
    pub risk_exits: Vec<ActiveLeg>,
    /// P2 CloseOverlay 关闭的重叠腿（TW StageII ∧ H>0 ⟹ 关 legacy ShortDiff 腿，PDF §7 C_2）
    /// ——无触发候选（TW 账本谓词驱动，非反向信号），独立于 `closed`；真产订单进同一
    /// schedule/fill/typed ledger（裁定4），消费端归 `ExitType::CloseShortDiff`。
    pub overlay_closes: Vec<ActiveLeg>,
    /// P3/P4 TW 账本事件分量 `TWEvent_t`（PDF §16 四元组 `(D,O,L,TWEvent)`；裁定4：P3
    /// RecoverCapital / P4 EnterEarning 无订单，成立时**消耗当步裁决**——gamma 全部推迟
    /// record 桶，屏蔽 P5..P10）。账本推进（`tw_step`）由消费端 runner 单点做。
    pub tw_event: Option<TwEvent>,
    /// ★M5 逐声部目标头寸 `P^sep_{t+1}`（多空对冲.pdf p16 关卡10 `Σ_{v∈A_{t+1}}q_vσ_v e_v` 的分量）
    /// ——runner [`OverlayState`](super::super::overlay_state) hedge-mode 簿据此建持久逐声部账本 + ΔN 订单。
    /// 空 = 本 bar 无活动腿（force_flat/无候选/AncOK 全剪）⟹ P^sep_{t+1}=∅，Net=0。只读暴露，
    /// 不进决策路径（`Σσ_v·q_units == p̃` 恒等，见 [`coverage_step_from_buckets_sep`]）。
    pub sep_legs: Vec<SepLeg>,
    /// ★opsem-dump（R5-a，基因 073a/274号）：本步 LexArgmin 的 top-3 J_Θ 候选键（字典序升序，
    /// `(JThetaKey, control)`）。经 runner `OpsemEntrySnapshot.lex_top3` 透传至 dump 的
    /// `lex_argmin_top3` 字段。**不进 p_star/J_Θ/χ 门控**——纯只读诊断切片（R5-1 铁律：dump 数据
    /// 不进 μ 桶/J_Θ 排序/χ，生产 p_star 仍由 `lex_argmin` 单独决定）。空 vec = 本步无开仓路径
    /// （P1 强平/P2 关腿/P3-P4 无订单）⟹ opened 为空 ⟹ 不被 opsem_snap 消费。
    pub lex_top3: Vec<(JThetaKey, f64)>,
}

/// I_Θ 组合层 TW/风控上下文（#124 裁定4：解释器接口升级 `I_Θ(ctx: RiskState+TwState+LegBook,
/// gamma) -> {buckets, order_effect, tw_event, exit_kind}` 的 ctx 分量；分歧A 裁决——
/// [`interp::interpret`] 签名不变，本 ctx 只进组合层 [`pi_theta_step_traced`]）。
///
/// `None`（旧调用方）⟹ TW 谓词 P2/P3/P4 不评估（bit-exact 原路径）；生产 π fill loop 传
/// `Some`（TW 真值源 = fill loop 内逐 bar 推进的 [`TwState`]，裁定4：`run_closed_loop`
/// 降级纯结构验证工具）。
pub(crate) struct TwStepCtx<'a> {
    /// TW 账本态（生产真值源，fill loop 维护）。
    pub state: &'a TwState,
    /// barrier 政策（P3/P4 判据经 [`stage_progression`] 单源）。
    pub policy: &'a RiskPolicy,
    /// 当前风控模式（EnterReady 的 RiskNormal 门）。
    pub risk_mode: RiskMode,
    /// 生产 legacy ShortDiff 活动腿 id（P2 的 H>0 判据 + 关腿对象；runner 从 typed ledger
    /// 在飞表 `entry_v == ShortDiff` 取——腿声部身份入场固定，与 TW `open_legacy_legs`
    /// 计数同源同步）。
    pub shortdiff_leg_ids: &'a std::collections::HashSet<ElementId>,
    /// η 修正量（A10 C5 裁定 (b)，TW 桥 G1）：生产 π loop 的 `cum_holding_cost` i64 shadow
    /// （funding+borrow+liq 累计量化），经 [`stage_progression_eta_corrected`] 进 P4 EnterReady
    /// 的 η 左操作数（η_corrected = tw() − eta_correction）。**0 ⟹ 与历史判据同值 bit-exact**
    /// （回归锁）。★F4 同源约束：与 ZExt 第 15 维 η_bucket 的修正量同一变量（runner 单点喂两处）。
    pub eta_correction: i64,
}

/// [`pi_theta_step_prebuilt`] 的 trace 版（G4 #134 组合层）：同一决策路径（interpret →
/// coverage_step_from_buckets → pi_theta_position → schedule_order，单源无平行状态机）+
/// 腿级生命周期差分 [`StepTrace`]。
///
/// bit-exact 见证：`interpret_with_close_triggers(..).0 == interpret(..)`（interp.rs 委托单源），
/// 其余三环逐字调用同函数 ⟹ `(next_active, p*, order)` 与 prebuilt 完全一致。
#[allow(clippy::too_many_arguments)]
pub(crate) fn pi_theta_step_traced(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
    config: &VoiceConfig,
    registry: &super::super::persistent::PersistentRegistry,
    tw: Option<&TwStepCtx>,
    protocol: &ProtocolEventSet,
) -> (Vec<ActiveLeg>, f64, PiThetaDecision, StepTrace) {
    // 协议轨只读折叠，与下方 P1..P10 订单轨正交；所有 return 分支携同一显式事件。
    let protocol_event = protocol.selected();
    // P1 强平（PDF §7 全互斥 C_1=P_1 屏蔽 P2..P10）：force_flat ⟹ 活动腿全部 RiskExit 清空、无开仓、
    // 目标 flat。**在 interpret/coverage_step 上游短路**——open 桶不进 next_active（幽灵腿堵口 §6.7）、
    // held 逐条 RiskExit 经 StepTrace 外化（G4 预留通道兑现）。触发不依赖候选集非空（gamma 空也清仓，
    // 裁定4 P1 语义）。p_star bit-exact 原路径：force_flat ⟹ 𝒦_Θ={0} ⟹ pi_theta_position=0（与 p̃ 无关，
    // 见 `pi_theta_position_force_flat_gate_clamps_to_zero`）——仅 next_active 从含幽灵腿变 ∅（跨 bar 语义
    // 修正，非本 bar order 变）。P1 成立时 TW 谓词 P2/P3/P4 一并被屏蔽（本分支先于 tw 检查）。
    if gate.force_flat {
        let p_star = pi_theta_position(0.0, p_t, base_units, risk, weights, gate);
        let order = schedule_order(p_star, p_t, exec_index);
        return (
            Vec::new(),
            p_star,
            (order, protocol_event),
            StepTrace { risk_exits: prev_active.to_vec(), ..Default::default() },
        );
    }
    // ── TW 谓词 P2/P3/P4（#124 裁定4「真统一」：TW 三阶段进 fold，PDF §7 C_2/C_3/C_4）──
    // 优先级 P2 ≻ P3 ≻ P4 ≻ P5..P10；成立时**消耗当步裁决**（gamma 全部推迟 record 桶，屏蔽
    // P5..P10 的开/平；§13 结构剪枝 AncOK/Stale 照常——那是活动集与树的状态同步，非解释器裁决）。
    // P2/P3 判据天然互斥（P2 要求 StageII，P3 的 RecoverCapital 只在 StageI 派）；P4 的
    // enter_ready 要求 open_legacy_legs==0 ⟹ P2 成立（有重叠腿）时 P4 自动不成立——优先级链
    // 与账本合法性谓词一致。tw=None（旧调用方/无 TW 源）⟹ 本段跳过，bit-exact 原路径。
    if let Some(twc) = tw {
        // P2 CloseOverlay：TW StageII ∧ H>0（legacy ShortDiff 重叠腿仍开）⟹ 关重叠腿。
        // 真产订单：合成 close 桶复用生产关腿路径（coverage_step_from_buckets 的 𝒟_x 通道 +
        // p̃ 重算 + LexArgmin + Schedule 全部原样单源），非平行订单机制。
        if twc.state.stage == TStage::CapitalRecovered {
            let overlay: Vec<ActiveLeg> = prev_active
                .iter()
                .filter(|l| twc.shortdiff_leg_ids.contains(&l.id))
                .copied()
                .collect();
            if !overlay.is_empty() {
                let buckets = Buckets {
                    close: overlay.clone(),
                    open: Vec::new(),
                    record: gamma.to_vec(), // 当步普通候选推迟记录（消耗当步裁决）
                };
                let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
                    work, prev_active, &buckets, base_units, config, Some(risk), registry,
                );
                let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
                let order = schedule_order(p_star, p_t, exec_index);
                // §13 结构剪枝（AncOK/Stale）照常 ⟹ 被剪腿仍须外化（消费端在飞表不泄漏）。
                let next_ids: std::collections::HashSet<ElementId> =
                    next_active.iter().map(|l| l.id).collect();
                let overlay_ids: std::collections::HashSet<ElementId> =
                    overlay.iter().map(|l| l.id).collect();
                let silent_drops = prev_active
                    .iter()
                    .filter(|l| !overlay_ids.contains(&l.id) && !next_ids.contains(&l.id))
                    .copied()
                    .collect();
                return (
                    next_active,
                    p_star,
                    (order, protocol_event),
                    StepTrace { overlay_closes: overlay, silent_drops, sep_legs, ..Default::default() },
                );
            }
        }
        // P3 RecoverCapital / P4 EnterEarning：无订单账本事件（stage_progression 单源判据，
        // 与 closed_loop 结构验证共用同一函数——不镜像）。输出 TWEvent_t 分量；账本推进
        // （tw_step）由消费端 runner 单点做（组合层只读 ctx，不 mutate 账本）。
        // ★A10 C5：η 修正经 stage_progression_eta_corrected（twc.eta_correction =
        // cum_holding_cost shadow；0 ⟹ bit-exact）。
        if let Some(ev) = stage_progression_eta_corrected(twc.policy, twc.state, twc.risk_mode, twc.eta_correction) {
            let buckets = Buckets {
                close: Vec::new(),
                open: Vec::new(),
                record: gamma.to_vec(), // 当步普通候选推迟记录（消耗当步裁决，屏蔽 P5..P10）
            };
            let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
                work, prev_active, &buckets, base_units, config, Some(risk), registry,
            );
            let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
            let order = schedule_order(p_star, p_t, exec_index);
            // §13 结构剪枝（AncOK/Stale）照常 ⟹ 被剪腿仍须外化（消费端在飞表不泄漏）。
            let next_ids: std::collections::HashSet<ElementId> =
                next_active.iter().map(|l| l.id).collect();
            let silent_drops = prev_active
                .iter()
                .filter(|l| !next_ids.contains(&l.id))
                .copied()
                .collect();
            return (
                next_active,
                p_star,
                (order, protocol_event),
                StepTrace { tw_event: Some(ev), silent_drops, sep_legs, ..Default::default() },
            );
        }
    }
    // 环5：解释器三桶 + close 触发归因（fold 单源，interp.rs）。
    let (buckets, close_triggers) = interp::interpret_with_close_triggers(gamma, prev_active);
    // open 候选 → 其 638 附着元素 id（work 即将 move 进 coverage_step_from_buckets，先抓）。
    let candidate_start = work.base_len();
    let open_cand_ids: Vec<(Candidate, ElementId)> = buckets
        .open
        .iter()
        .filter_map(|c| work.get(candidate_start + c.gamma_index).map(|e| (*c, e.id)))
        .collect();
    // 环6：活动集递归 + AncOK + G7 毛约束（原样单源）。★M5：sep 出口暴露逐声部 P^sep_{t+1}。
    let (next_active, p_tilde, sep_legs) =
        coverage_step_from_buckets_sep(work, prev_active, &buckets, base_units, config, Some(risk), registry);
    // 环7：LexArgmin + Schedule（原样单源）。
    let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
    let order = schedule_order(p_star, p_t, exec_index);

    // ── trace 差分（决策已定，纯只读观测）──
    let next_ids: std::collections::HashSet<ElementId> =
        next_active.iter().map(|l| l.id).collect();
    let closed: Vec<(ActiveLeg, Candidate)> =
        buckets.close.iter().copied().zip(close_triggers).collect();
    let closed_ids: std::collections::HashSet<ElementId> =
        closed.iter().map(|(l, _)| l.id).collect();
    // 静默离场：prev_active 中既未被 close 桶认领、也不在 next_active（AncOK 剪/Stale prune）。
    let silent_drops: Vec<ActiveLeg> = prev_active
        .iter()
        .filter(|l| !closed_ids.contains(&l.id) && !next_ids.contains(&l.id))
        .copied()
        .collect();
    // 真正准入的 open 候选：其附着元素 id 出现在 next_active（AncOK 未剪）。
    let opened: Vec<(Candidate, ActiveLeg)> = open_cand_ids
        .into_iter()
        .filter_map(|(c, id)| next_active.iter().find(|l| l.id == id).map(|l| (c, *l)))
        .collect();

    // ★R5-a opsem-dump：LexArgmin top-3 J_Θ 切片。仅本步有开仓时算（无开仓 bar 零开销；6 候选排序
    // O(1)）。复用 [`feasible_lex_candidates`]（与 pi_theta_position 同源候选集）⟹ top-3 首名 ≡ p_star
    // 选址。不进 p_star/J_Θ/χ——纯只读诊断（R5-1 铁律），生产决策由上方 pi_theta_position 单独定。
    let lex_top3 = if opened.is_empty() {
        Vec::new()
    } else {
        lex_argmin_top_k(&feasible_lex_candidates(p_tilde, p_t, base_units, risk, weights, gate), 3)
    };

    (
        next_active,
        p_star,
        (order, protocol_event),
        StepTrace { closed, silent_drops, opened, sep_legs, lex_top3, ..Default::default() },
    )
}

#[cfg(test)]
#[path = "sizing_tests_1.rs"]
mod tests_1;
#[cfg(test)]
#[path = "sizing_tests_2.rs"]
mod tests_2;
