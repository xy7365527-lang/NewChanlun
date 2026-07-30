use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  #625 发现1 观测探针（review-625.md 发现1 PLAUSIBLE）——KΘ hi/lo cap binding 计数
// ════════════════════════════════════════════════════════════════════════════
// 常驻探针（ancok.rs 同惯例，thread_local Cell）：纯观测旁路，只在 caps() 既有的
// stop_long/stop_short 分支上 bump 计数，不改 hi/lo 取值/判定逻辑。用于验证评审建议的
// 检验法——「若 binding 恒零，方向口径错配（sizing.rs:125 hi/lo cap 消费者与 fill.rs 分桶
// 不同源，见 review-625.md 发现1）对净持仓的影响即为 0」。

/// KΘ 风控门 hi/lo cap 分侧 binding 次数（review-625.md 发现1 观测量）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapBindingProbe {
    /// `stop_long` 生效（hi 禁净多归零）次数。
    pub hi_binding: u64,
    /// `stop_short` 生效（lo 禁净空归零）次数。
    pub lo_binding: u64,
}

thread_local! {
    static CAP_BINDING_PROBE: std::cell::Cell<CapBindingProbe> =
        const { std::cell::Cell::new(CapBindingProbe { hi_binding: 0, lo_binding: 0 }) };
}

#[inline]
fn cap_binding_probe_bump(f: impl FnOnce(&mut CapBindingProbe)) {
    CAP_BINDING_PROBE.with(|c| {
        let mut p = c.get();
        f(&mut p);
        c.set(p);
    });
}

/// 归零 cap binding 探针（wf8 run 前调用）。
pub fn cap_binding_probe_reset() {
    CAP_BINDING_PROBE.with(|c| c.set(CapBindingProbe::default()));
}

/// 读取 cap binding 探针快照（wf8 run 后调用）。
pub fn cap_binding_probe_snapshot() -> CapBindingProbe {
    CAP_BINDING_PROBE.with(std::cell::Cell::get)
}

// ════════════════════════════════════════════════════════════════════════════
//  #628 归因扩展——逐次 binding 事件（分桶方向 vs p̃ 符号一致性 + 反事实 p* 差值）
// ════════════════════════════════════════════════════════════════════════════
// 上面的 CapBindingProbe 只计次数，不够回答 #628 票问 1（183 次逐次归因：分桶方向与 p̃
// 投影方向一致/相反各多少、相反时夹错多少）。本探针在同一触发点（`stop_long`/`stop_short`
// 生效）额外记录 `p_tilde` 符号 + 反事实 p*（移除**本次触发的那一侧**风控约束、其余门不变
// 重算的 p*）——`p_star_actual − p_star_cf` 即该次 binding 对净持仓的真实影响幅度；差值为 0
// 说明 binding 当次无实害（p̃ 本就未压向那一侧）。反事实走 `feasible_lex_candidates_raw`
// （用 `caps_raw`，不二次 bump 上面的计数探针）。

/// 单次 cap binding 归因记录（#628 阶段一观测量）。
#[derive(Debug, Clone, Copy)]
pub struct CapBindingAttributionEvent {
    /// 本次 `stop_long`（hi 禁净多）是否触发。
    pub hi_triggered: bool,
    /// 本次 `stop_short`（lo 禁净空）是否触发。
    pub lo_triggered: bool,
    /// 触发当次的目标净持仓 p̃（eps 投影口径，正=净多/负=净空，`leg.rs:281`）。
    pub p_tilde: f64,
    /// 实际 p*（既有生产路径产出，本记录不改其值）。
    pub p_star_actual: f64,
    /// 反事实 p*：仅移除 `stop_long`（其余门含 `stop_short` 不变）重算；`hi_triggered=false` 则 `None`。
    pub p_star_cf_hi: Option<f64>,
    /// 反事实 p*：仅移除 `stop_short`（其余门含 `stop_long` 不变）重算；`lo_triggered=false` 则 `None`。
    pub p_star_cf_lo: Option<f64>,
}

thread_local! {
    static CAP_BINDING_ATTRIBUTION: std::cell::RefCell<Vec<CapBindingAttributionEvent>> =
        std::cell::RefCell::new(Vec::new());
}

/// 归零 #628 归因事件表（wf8 run 前调用，与 [`cap_binding_probe_reset`] 同惯例）。
pub fn cap_binding_attribution_reset() {
    CAP_BINDING_ATTRIBUTION.with(|c| c.borrow_mut().clear());
}

/// 读取 #628 归因事件表快照（wf8 run 后调用）。
pub fn cap_binding_attribution_snapshot() -> Vec<CapBindingAttributionEvent> {
    CAP_BINDING_ATTRIBUTION.with(|c| c.borrow().clone())
}

/// 𝒦_Θ 可行代表集的**无副作用**构造（同 [`feasible_lex_candidates`]，但用 [`KThetaRiskGate::caps_raw`]
/// 而非 `caps`——不 bump [`CAP_BINDING_PROBE`]）。仅供本节反事实归因复用，非生产决策路径。
fn feasible_lex_candidates_raw(
    p_tilde: f64,
    p_t: f64,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
) -> Vec<LexCandidate<f64>> {
    let lot = risk.default_lot.max(1) as f64;
    let cap = feasible_net_cap(risk) * base_units.abs();
    let (lo_cap, hi_cap) = gate.caps_raw(cap);
    feasible_candidates(p_tilde, p_t, lo_cap, hi_cap, lot)
        .into_iter()
        .map(|(p, gi)| LexCandidate { control: p, key: j_theta_key(p, p_tilde, p_t, weights, gi) })
        .collect()
}

/// #628 阶段一归因记录：`gate` 触发 `stop_long`/`stop_short` 时，逐侧算反事实 p* 并入表。
/// `force_flat` 或两侧均未触发 ⟹ 早退（无 binding 可归因）。
fn record_cap_binding_attribution(
    p_tilde: f64,
    p_t: f64,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
    p_star_actual: f64,
) {
    if gate.force_flat || !(gate.stop_long || gate.stop_short) {
        return;
    }
    let p_star_cf_hi = gate.stop_long.then(|| {
        let cf_gate = KThetaRiskGate { stop_long: false, ..gate };
        lex_argmin(&feasible_lex_candidates_raw(p_tilde, p_t, base_units, risk, weights, cf_gate)).unwrap_or(0.0)
    });
    let p_star_cf_lo = gate.stop_short.then(|| {
        let cf_gate = KThetaRiskGate { stop_short: false, ..gate };
        lex_argmin(&feasible_lex_candidates_raw(p_tilde, p_t, base_units, risk, weights, cf_gate)).unwrap_or(0.0)
    });
    CAP_BINDING_ATTRIBUTION.with(|c| {
        c.borrow_mut().push(CapBindingAttributionEvent {
            hi_triggered: gate.stop_long,
            lo_triggered: gate.stop_short,
            p_tilde,
            p_star_actual,
            p_star_cf_hi,
            p_star_cf_lo,
        })
    });
}

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
fn scale_key(x: f64) -> i64 {
    (x * J_SCALE).round().clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

/// 净持仓 lot 对齐（向最近 lot 取整；`lot≥1` 由调用方 `RiskConfig.default_lot.max(1)` 保证）。
fn lot_round(p: f64, lot: f64) -> f64 {
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
fn feasible_net_cap(risk: &RiskConfig) -> f64 {
    risk.gamma.abs()
}

/// ★M4 级别级协变 cap `cap_ℓ = w_ℓ·γ̄·U_ℓ`（multi-level-native-execution-design-20260719
/// §D M4；[`super::super::level_risk`] 模块头「对偶统一声明」）。
///
/// **协变分解守恒**（票体验收「𝒦_Θ 协变 cap 按级别分解后仍满足」的机器判据）：账户层总 cap
/// `= γ̄·U_ℓ`（[`feasible_net_cap`]×`base_units`，同上方用的量）；本函数只多乘一个
/// `w_ℓ∈[0,1]` 因子——`cap_ℓ` 与总 cap 共用同一 `U_ℓ` 协变缩放，故 `cap_ℓ` 本身**逐级协变**：
/// `a_k` 缩放资本时 `cap_ℓ(S_k x) = a_k·cap_ℓ(x)`，与总 cap 的协变律同构，非另立一套。
///
/// `Σ_ℓ cap_ℓ = (Σ_ℓ w_ℓ)·γ̄·U_ℓ ≤ γ̄·U_ℓ`（由
/// [`super::super::level_risk::level_weights_sum_le_one`] 保证 `Σw_ℓ≤1`）——分解后的级别帽之和
/// **不超过**未分解的账户层总 cap，这正是「分解后仍满足」的代数内容（L0，见测试
/// `level_cap_decomposition_never_exceeds_account_cap`）。
pub(crate) fn level_cap(level: u32, base_units: f64, risk: &RiskConfig) -> f64 {
    feasible_net_cap(risk) * super::super::level_risk::level_weight(level, risk) * base_units.abs()
}

/// ★M4 级别级风险帽实际施加点（`risk.enforce_level_cap` 门禁，G7 [`apply_gross_cap`] 同款
/// 模式）：把已按级别聚合的门控结构基准 `gated_ℓ` 逐级 clamp 到 `[-cap_ℓ, +cap_ℓ]`
/// （[`level_cap`]）。
///
/// 施加点纪律（§F③ 域分离 + `level_risk` 模块头「禁双重定价」）：本函数只读**已经**按级别
/// 聚合完成的 `gated`（`net_ℓ` 之和，depth_weight 早已沉淀在其中），**不**拆解单条 leg 的深度
/// 构成——level_weight 与 depth_weight 因此不会对同一块资金重复定价。零项保留（级别封闭
/// 可读，与 `LevelOrderPlan::deltas` 同纪律）。
///
/// `risk.enforce_level_cap=false`（default）⟹ 调用方**不得**调用本函数（应直接跳过），
/// 而不是传入空 `level_weights` 期望本函数自然退化——空表会把 `cap_ℓ` 恒裁到 0，那是「全部
/// 级别禁止持仓」而非「级别帽未启用」，两者语义相反，门禁必须在调用方而非本函数。
///
/// ★#642 语义重放偏差照实声明：本函数是 #310 的核心数学原语（level_cap 协变分解 + 逐级
/// clamp），语义与 kimi 侧 `coverage.rs::clamp_levels_to_weighted_cap` 逐字等价；但 kimi 侧
/// `fill.rs` 的实际调用点（`plan_level_gated_order`，把本函数接进 `LevelOrderLedger::regate`
/// 输出的两处施加点）**未随本次移植接线**。
///
/// ★#714 MED-2 订正（影子评审 shadow-642-review-20260729.md，同步 #693）：上一版本段曾声称
/// main 侧接线点「目前均不存在于代码」，且字段名误写为 `capped_levels`——均不确。真实字段名是
/// [`super::super::level_order::LevelOrderPlan::cap_narrowed_levels`]（`level_order.rs:384`），
/// **该字段本身、其统计 [`super::super::level_order::LevelOrderStats::n_cap_narrowed`]、逐级
/// sparsity 判据（`level_order.rs:593`）三层均已在场**——kimi 侧原 m8 报表列消费方
/// （`wverify_run/{m8,report}.rs`）随 #644 判定「清理」出仓（未声明死文件，main 侧 `m8_e2e_all_systems_oos`
/// 已有等价内联报表，见 `wverify_run.rs`）；本条不再指向它们。缺的只是**唯一填入者**：
/// `plan_gated`（`level_order.rs:545-553`）恒把
/// `cap_narrowed_levels` 置空表，未调用本函数填入实际裁剪结果。接线成本因此不是「从头设计
/// 接口」而是「补一个生产者填充既有字段」，但填入前需先决定是否、如何对齐 main 自己
/// #355/#363/#369/#376 谱系写下的既有接口形状——仍超出本票语义重放范围，留作独立跟进项（见
/// issue642-coverage-replay 报告条目 4 订正段）。`enforce_level_cap` default=false 且本函数
/// 未被生产路径调用 ⟹ 零行为改变（M0-M3 bit-exact 不变，同 kimi 原提交声明）。
#[allow(dead_code)]
pub(crate) fn clamp_levels_to_weighted_cap(
    gated: &[(u32, i64)],
    base_units: f64,
    risk: &RiskConfig,
) -> Vec<(u32, i64)> {
    gated
        .iter()
        .map(|&(lvl, q)| {
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

    /// 应用约束门到对称 cap，产 `(lo_cap, hi_cap)` 幅度（`force_flat` 优先收到 {0}）——**无副作用版**
    /// （不 bump [`CAP_BINDING_PROBE`]，供 #628 反事实归因复用，避免二次调用污染生产计数）。
    fn caps_raw(&self, cap: f64) -> (f64, f64) {
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

    /// 应用约束门到对称 cap，产 `(lo_cap, hi_cap)` 幅度（`force_flat` 优先收到 {0}）。
    fn caps(&self, cap: f64) -> (f64, f64) {
        let (lo, hi) = self.caps_raw(cap);
        // ★#625 发现1 观测（review-625.md 发现1 PLAUSIBLE）：分侧 binding 计数，纯旁路，
        // 不改上方 hi/lo 取值——用于验证「若 binding 恒零，方向口径错配对净持仓影响为 0」。
        if !self.force_flat {
            if self.stop_long {
                cap_binding_probe_bump(|p| p.hi_binding += 1);
            }
            if self.stop_short {
                cap_binding_probe_bump(|p| p.lo_binding += 1);
            }
        }
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
fn feasible_candidates(p_tilde: f64, p_t: f64, lo_cap: f64, hi_cap: f64, lot: f64) -> Vec<(f64, i64)> {
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
fn j_theta_key(
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
    let p_star =
        lex_argmin(&feasible_lex_candidates(p_tilde, p_t, base_units, risk, weights, gate)).unwrap_or(0.0);
    // ★#628 阶段一归因：binding 触发时记录反事实 p*（早退防护在函数内，非 binding 时零开销）。
    record_cap_binding_attribution(p_tilde, p_t, base_units, risk, weights, gate, p_star);
    p_star
}

/// 𝒦_Θ 可行代表集 → `Vec<LexCandidate<f64>>`（J_Θ 键已挂），供 [`lex_argmin`]（p_star 选址）与
/// [`lex_argmin_top_k`]（R5-a 诊断 top-k）共用——**单一构造源**保证 argmin 与 top-k 同候选集
/// （top-k 的第一名 ≡ argmin 选址，bit-exact）。
///
/// 抽取自 [`pi_theta_position`]（纯重构，候选构造逻辑逐字不变）。pub(crate) 供
/// [`pi_theta_step_traced`] 在 dump 启用路径复用算 top-k。
pub(super) fn feasible_lex_candidates(
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
///    interpret(ℛ_Θ) → A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] → p̃_{t+1}`。入场源 = **买卖点 Γ(x)**
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
pub(super) fn pi_theta_step_prebuilt(
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





#[cfg(test)]
#[path = "sizing_tests_1.rs"]
mod tests_1;

#[cfg(test)]
#[path = "sizing_tests_2.rs"]
mod tests_2;
