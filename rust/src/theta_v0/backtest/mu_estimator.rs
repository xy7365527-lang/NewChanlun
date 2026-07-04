//! μ(z,a) 类别条件边际收益估计器（alpha2.pdf §12-§14，task #39 mu-estimator）。
//!
//! ## 命题（alpha2 §12 / §16）
//!
//! 把买卖点操作状态拆成**全互斥类别** z，在每类上估计条件边际收益样本均值：
//!
//! ```text
//!   z = (ℓ, δ, I_γ, 父声部方向, 短差/顺势, 仓位态)         (§16 line 3262)
//!   τ_γ = inf{u > t : 出现该声部出场证书或风险退出}          (§12 line 2140)
//!   X_γ = δ·(P_τγ − P_t) − C_{t:τγ}                          (§12 line 2147)
//!   μ(z) = E[X_γ | Z = z] ≈ (1/|S_z|) Σ_{γ∈S_z} X_γ          (§12 line 2173, 样本均值)
//! ```
//!
//! 严格 alpha 条件 `μ(z) > 0`（§12 line 2180）；`μ(z) ≤ 0` ⟹ 该类在此退出规则/成本模型/
//! 样本下无正期望（§12 line 2186）。本模块**只估计 μ**——不做 χ_θ 阈值过滤（那是下游
//! chi-theta-filter 工位 acc-chi-theta-filter），不做 argmax_a 动作选择。诚实声明：本模块
//! 是 alpha2「估计 μ」这一步（§17 line 3519 流程的 `估计 μ` 环节），不是选择器。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! | 组件 | 等级 | 理由 |
//! |------|------|------|
//! | 估计器逻辑（[`MuEstimator`] 分桶/求均值/[`MuClass`] 编码） | **L1** | 给定观测序列求条件均值是确定性变换，验证算法正确，零信息增量 |
//! | 真实数据驱动的 μ 值 | **L2** | 喂真实历史交易的 X_γ 才能否证「某类 z 有正边际收益」（正信息增量） |
//!
//! **合成数据 μ 值是 L1**——自造 X_γ 求均值只验证分桶/平均无 bug，不验证任何 z 类在市场有
//! 正期望（合成数据独立性验证是同义反复，231号）。本模块单测全部喂合成观测 ⟹ L1。
//!
//! ## 因果性硬约束（alpha2 §5 / formalization-validity-domain / project_zero_lookahead_backtest）
//!
//! `X_γ` 的 `P_τγ` 必须是持仓**实际兑现**到未来退出时刻 τ_γ 的价格——这是 F_τγ-可测的真实
//! 退出，**不是**端点后视 `ε_e = sign(P_ρe − P_λe)`（用段终点反推方向 = 未来函数泄漏）。
//! 本模块不计算 τ_γ（退出时刻由上游交易轨迹给定，[`MuObservation::x_gamma`] 由调用方按
//! 真实 entry/exit 价格用 [`marginal_return`] 算出后传入）。μ 估计器只对**已实现**的 X_γ 分桶——
//! 这是 μ 与构造性恒真 G_e 的本质区别：G_e 用端点拼接恒真（L0 同义反复），μ 用实际兑现可否证。

use std::collections::HashMap;

use super::metrics::trade_abs_pnl;
use crate::theta_v0::classifier::divergence::ForceStateA5;
use crate::theta_v0::strategy::coverage::Horizontal;
use crate::theta_v0::types::BspBits;

/// 仓位态（z 的分量，§16 line 3262「仓位态」）。
///
/// 区分声部在持仓树中的角色：根声部（无父，主趋势腿）vs 子声部（有父，对冲/短差腿）。
/// 这是 z 全互斥分类的一维——不同仓位态不混（alpha2 §18「多空双开状态不会混在一起」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PositionState {
    /// 根声部（§3 ⊥，host 父容器=边界胚元 ∂，无父声部）。主趋势持仓腿。
    Root,
    /// 子声部（§3 有父声部 p(v)）。对冲/短差腿，依附父持仓期内。
    Child,
}

/// 全互斥分类值 z（alpha2 §12 `γ=(c,ℓ,δ,I_γ,t)` + §16 line 3262 扩展态）。
///
/// 六维全互斥（§18：级别/三类/短差顺势/父声部方向/仓位态/多空 不混）：
/// - `level` ℓ：买卖点所在级别（Voice.carrier.level）。
/// - `delta` δ ∈ {+1,−1}：持仓方向（Voice.dir；买点 +1 / 卖点 −1）。
/// - `i_class` I_γ⊆{1,2,3}：买卖点类别集合 **bit-vector 不压扁**（[`BspBits::class_index`]
///   的 6-bit，B1/B2/B3/S1/S2/S3 各独立 ⟹ 2B/3B 重合保留，§P4 §5 非互斥三分）。
/// - `parent_dir` σ_p：父声部方向（根声部 = 0/Ambient，去根化非「未持仓」；runner.rs:218）。
/// - `short_swing` 短差/顺势：子声部 σ_u=−σ_p ⟹ 短差（true）；同向 ⟹ 顺势（false）。
///   `Voice::child_dir(parent_dir) = −parent_dir`（pi_bsp_timing.rs:122 §6/§16）。
/// - `position` 仓位态：[`PositionState`]。
/// - `horizontal` H(g) 水平关系（同父前兄弟顺/反/无，[`Horizontal`]）：R(g)=(H,V,δ) 的 H 轴
///   （codex #81 裁定 `h_axis_in_canonical_z: accept`——补入 canonical Z 保 R(g)18 类忠实）。
///   `Some(h)` 由 [`super::selector::z_of_candidate`] 从 `Candidate.role.h` 填（真候选路径）；
///   `None` = 本构造口径未定 H（[`MuClass::from_certificate`] 的裸证书分量不含前兄弟关系，
///   pi_bsp_timing 从 Voice 构 z 无 H 源——**诚实标 None 不伪造 First**，231号/no-claim-inflation）。
///   `None` 在同一消费路径内恒定 ⟹ 不改分桶（如 perm_test/wverify 按 (ℓ,bsp,δ,σ_p) 4 维分桶不读 H）。
///   winner selection 不用 H（codex `h_axis_in_default_selection: conditional`）：H 只进 z 报告，
///   降维 [`UClass::project_to_u`] 默认丢 H（§30 抗 winner's curse）。
/// - `sigma_higher` σ^higher 上级方向态（第 9 维，codex-q1 G2）：canonical z 含之（oracle 上界/
///   完整性声明用），[`UClass::project_to_u`] 同 H 丢弃（selection 抗碎片化）——复用 H 轴分层先例。
/// - `cand_channel`/`nest_depth`/`origin_level`/`risk_mode` 第 10-13 维（G3 #138，《完整的策略》§6
///   z 完整形态的 CandType/Ndepth/ℓ 起始级/RiskMode+MarginState 四条目）：见各字段文档。
///   §6 其余条目的承载/缺口声明见 `.chanlun/review-results/g3-impl-20260703.md` 维度对照表
///   （Jchain=由 (ℓ,e) 代数派生；ExitType=TypedTrade ledger 层已接（G4）不进 F_t 可测桶键；
///   TStage=第 14 维已接（#149，GAP3 桥后 π fill loop `tw.stage` 真值，见 `t_stage` 字段文档）；
///   ηBucket=第 15 维已接（#175，终裁 a5-etabucket-stance-ruling-20260704.md 推翻 #149「无生产者」
///   判定：γ_t = 现有 η_t/η_* 比较判据的离散化，η_t 生产者=`TwState::tw()`、η_* 生产者=
///   `RiskPolicy::eta_star()` 早已存在，零新数据源——见 `eta_bucket` 字段文档）；
///   CostBucket=生产路径无数据源，诚实缺口+证明义务）。
///
/// 派生 `Eq + Hash` ⟹ 可作 HashMap key（分桶载体）；`Ord` ⟹ 可作 BTreeMap key（有序报告）。
/// **全互斥**：每个 z 是 {0,1}^6 × 级别 × 方向 × 父向 × 短差 × 仓位态 × H 的唯一组合，无重叠
/// （§13 精细分类优势定理的可计算落点；补 H 后升 R(g)18 类完整表达）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MuClass {
    /// 执行级别 e（§6 的 e，**G3 语义澄清**：本字段自始装的是候选/买卖点所在级别 = 信号执行级
    /// ——econ 门路径「执行级 e=lvl，rungs 从 tower[lvl+1..] 收集上级语境」（econ_positive
    /// collect_signals 有效域注释）。§6 的起始级 ℓ 是独立维，见 `origin_level`。旧文档把本字段
    /// 标注为 ℓ 系口径漂移，G3 更正（诚实：字段值语义从未变，变的是标注）。
    pub level: u32,
    pub delta: i8,
    pub i_class: u8,
    pub parent_dir: i8,
    pub short_swing: bool,
    pub position: PositionState,
    /// H(g) 水平关系（`Some`=真候选 z_of_candidate 填；`None`=裸证书口径未定 H，见类型文档）。
    pub horizontal: Option<Horizontal>,
    /// β^div 力度支配态（`关于背驰.pdf` §9.1，beta-bucket-design v2 第 8 维）。`Some`=真候选路径
    /// `z_of_candidate` 从 `Candidate.force`（A6 #159 透传 `BspPoint.force`）经 A/C 段
    /// `ForceProxies::force_state()` 填；`None`=无力度源口径
    /// （`from_certificate`/无 ForceProxies 候选，同 `horizontal` 的诚实 None，231号不伪造）。
    pub force_state: Option<ForceStateA5>,
    /// σ_higher 上级方向态（第 9 维，codex-q1 G2 终裁翻转 #81：《完整的策略》§6 要求 z ⊇
    /// (ℓ,δ,σ_higher)；667号实证——级别依赖调制器，效应符号随 level 翻转，不进 z 会把符号相反
    /// 子群体平均掉）。`Some(v)`=生产路径由 [`super::selector::sigma_higher_at`] 从塔真值填
    /// （+1 上级净涨 / −1 净跌 / 0 持平或无上级——**0 是计算结果非未知**）；`None`=裸证书口径
    /// 无 tower/bars 源（[`MuClass::from_certificate`]），诚实 None 不伪造（231号，同 `horizontal`）。
    /// 与 σ_p（`parent_dir`，持仓树父声部方向）**并列独立**，不合并（#81 反对合并的原理由仍成立）。
    pub sigma_higher: Option<i8>,
    /// Cand 门通道类型（第 10 维，§6 CandType，G3 #138）。`Some`=经 econ 统计层二通道准入门
    /// （[`super::econ_positive::NestTrigger`]：Type1 趋势背驰段 / Type2/3 次级 Type1 下沉锚 /
    /// 小转大 Xzd），collect_signals 从已 pass 的 GateCertificate 派生（P0-1 同源，非重算）；
    /// `None`=本构造口径未经准入门（runner π 路径候选不过 Nest/Xzd 门、from_certificate 裸口径），
    /// 诚实 None 不伪造（231号，同 horizontal 先例）。
    ///
    /// **§6 域声明**：PDF 域 {LiveCand, SettledCand, Force}——当前管线确认-bar 部署下候选恒为
    /// Settled 口径（Live 未确认候选无生产者，架构口径缺口见 g3 结果包），Force 口径已由
    /// `force_state` 第 8 维独立承载；本维承载的是「门通道」轴（任务 #138 对 CandType 的裁定读法）。
    pub cand_channel: Option<super::econ_positive::NestTrigger>,
    /// 区间套下沉深度 Ndepth（第 11 维，§6，G3 #138）= `NestCertificate.rungs.len()`。
    /// `Some(0)`=经 Nest 门且基例（ℓ=e，纯 Conf^δ_e——0 是计算结果非未知，G2 口径）；
    /// `Some(d>0)`=真跨级 J 嵌套 d 级；`None`=未经区间套门（Xzd 通道无下沉概念 / runner π 路径 /
    /// 裸口径）——**不是 0**，深度概念在该口径未定义。BTC 实测 95.36% 基例、4.64% d=1
    /// （econ_positive 有效域注释），本维使该退化在 μ̂ 分桶层可观测。
    pub nest_depth: Option<u8>,
    /// 起始级别 ℓ（第 12 维，§6 的 ℓ，G3 #138）：区间套链顶级别。Nest 通道 = `level + rungs.len()`
    /// （从高级 ℓ 背驰段逐级下沉定位到执行级 e=level）；Xzd/无门候选 = `level`（起始=执行，
    /// 无下沉——真值非占位：级别事实对任何候选有定义）；`None`=from_certificate 裸口径
    /// （无链语境，不伪造 ℓ=e）。恒等式 `origin_level = level + nest_depth`（Nest 通道，
    /// debug_assert 见 z 装配点）——Jchain（§6 区间套包含链）由 N^δ 定义强制逐级相邻
    /// （nest.rs `rungs[0]`=级ℓ..`rungs[last]`=级e+1 连续、无跳级），链签名 ≅ (ℓ,e)，
    /// 故 Jchain 无独立自由度，由本维 + `level` 完整携带（对照表论证，非缺口）。
    pub origin_level: Option<u32>,
    /// 账户风险模式（第 13 维，§6 RiskMode+MarginState 双覆盖，G3 #138）。
    /// [`RiskMode`](crate::theta_v0::strategy::risk::RiskMode) M0-M4 是 margin-design §2.4-§2.7
    /// 从保证金输入 (equity, MM, B1, B2, liq_flag) 派生的完整保证金状态机——同时是 §6「RiskMode:
    /// 正常/去杠杆/强平」的细化（M4 / M2∪M3 / M0∪M1）与「MarginState: 保证金状态」的离散化
    /// （equity 对 {0, MM, MM+B1, MM+B2} 阈值划分），单字段双覆盖，粒度 ⊇ 二者。
    /// `Some`=runner π fill loop 账本态真值（k_theta_risk_gate 每 bar 已算，bar 级同值 ⟹
    /// 训练 entry_z 与 χ 查询同口径，G2 护航点同款保证）；`None`=无账本口径（econ 统计层信号
    /// 收集无 equity/持仓、裸口径），诚实 None。
    pub risk_mode: Option<crate::theta_v0::strategy::risk::RiskMode>,
    /// 取本金三阶段 TStage（第 14 维，§6 TStage，#149 zdims）。
    /// [`TStage`](crate::theta_v0::strategy::ledger::TStage) = 缠师第31课降成本/退本金/增股数
    /// 三阶段（`Origin.TotalWealth.TStage`，OQ-9 单向不可逆）。生产者 = π fill loop 的 TW 账本
    /// `tw.stage`（#124 裁定4 单一真值源，#140 A' 后 P3/P4 现实可达）——G3 时该缺口的关闭条件
    /// 「GAP3 桥落地」已成立（g3 结果包 #15 行证明义务履行）。
    /// `Some`=runner π fill loop 当 bar 决策点账本相位真值（与 `TwStepCtx.state` 同一 `tw` 变量
    /// ⟹ 与 P2/P3/P4 谓词同口径；训练 entry_z 与 χ 查询共用同一 ext ⟹ 同口径，G2/G3 护航点
    /// 同款）；`None`=无 TW 账本口径（econ 统计层信号收集、裸证书），诚实 None（231号）。
    pub t_stage: Option<crate::theta_v0::strategy::ledger::TStage>,
    /// γ_t 四桶 ηBucket（第 15 维，§6 ηBucket「负成本缓冲状态」，#175）。
    /// [`EtaBucket`](crate::theta_v0::strategy::ledger::EtaBucket) = PDF §10 γ_t 分段式四态
    /// （Deficit/Zero/PositiveUnsafe/PositiveSafe）。终裁 a5-etabucket-stance-ruling-20260704.md
    /// （立场B）：被分类量 η_t = `TwState::tw()`——与 `enter_ready` 的 `η≥η⋆` 合取项左操作数
    /// **同一个量**；η_* = `RiskPolicy::eta_star()`。纯派生分类，零新数据源。
    /// `Some`=runner π fill loop 当 bar 决策点 `tw_policy.eta_bucket(&tw)`（与 `t_stage` 同一
    /// `tw` 变量同一装配点 ⟹ 同口径无时序错位；训练 entry_z 与 χ 查询共用同一 ext ⟹ 同口径，
    /// G2/G3 护航点同款）；`None`=无 TW 账本口径（econ 统计层信号收集、裸证书），诚实 None（231号）。
    pub eta_bucket: Option<crate::theta_v0::strategy::ledger::EtaBucket>,
}

impl MuClass {
    /// prereg §1.1 bsp_class 主类号（1/2/3 取最低；买卖由 delta 编码，1 类=buy1|sell1）。
    /// 从 i_class 6-bit 掩码恢复（bit0=buy1..bit5=sell3；class_index 可逆）。
    pub fn bsp_class(&self) -> u8 {
        let b = self.i_class;
        if b & 0b001_001 != 0 { 1 } else if b & 0b010_010 != 0 { 2 }
        else if b & 0b100_100 != 0 { 3 } else { 0 }
    }

    /// 从证书原始分量构造 z（§12 `γ=(c,ℓ,δ,I_γ,t)` + §16 扩展态）。
    ///
    /// `i_class` 取 [`BspBits::class_index`]——**不压扁** I_γ（2B/3B 重合保留为不同 z）。
    /// `short_swing` 由 `delta` 与 `parent_dir` 关系判定：子声部且 δ=−σ_p ⟹ 短差（§6/§16）；
    /// 根声部（`parent_dir=0`）恒顺势（无父可对冲，short_swing=false）。
    ///
    /// `horizontal=None`：裸证书分量（ℓ,δ,I_γ,σ_p,仓位态）不含前兄弟关系 H——H 需 `Candidate.role.h`
    /// （见 [`super::selector::z_of_candidate`]，真候选路径填 `Some(h)`）。此构造口径（pi_bsp_timing
    /// 从 Voice 构 z、合成测试）无 H 源，诚实标 `None` 不伪造 `First`（231号/no-claim-inflation）。
    pub fn from_certificate(
        level: u32,
        delta: i8,
        bits: BspBits,
        parent_dir: i8,
        position: PositionState,
    ) -> Self {
        // 短差判定：有父（parent_dir≠0）且方向与父反向（δ=−σ_p）⟹ 短差对冲腿（§6/§16）。
        // 根声部（parent_dir=0）无父，short_swing 恒 false（顺势主腿）。
        let short_swing = parent_dir != 0 && delta == -parent_dir;
        MuClass {
            level,
            delta,
            i_class: bits.class_index(),
            parent_dir,
            short_swing,
            position,
            horizontal: None,
            force_state: None,
            sigma_higher: None,
            // G3 四维（#138）：裸证书口径无准入门/链语境/账本态——诚实 None 同 horizontal 先例。
            cand_channel: None,
            nest_depth: None,
            origin_level: None,
            risk_mode: None,
            t_stage: None, // #149：裸证书口径无 TW 账本，同 risk_mode 诚实 None。
            eta_bucket: None, // #175：裸证书口径无 TW 账本，同 t_stage 诚实 None。
        }
    }
}

/// 低维投影类 u（《全互斥定义策略2》§30 压缩映射 ϕ:Z→U 的像）。
///
/// ## 命题（维数控制，§29-§30）
///
/// 过拟合自由度 = 状态组合数 K（§29：`K=|ℓ|·2·|I_γ|·|σ_p|·|短差|·|仓位态|…`）。K 大 ⟹
/// 每类样本 n̄=M/K 小 ⟹ winner's curse（高维 z 上 argmax μ 选中估计噪声）。压缩映射
/// ϕ:Z→U 把高维 z 折叠到低维 u（§30），降 K ⟹ 升 n̄ ⟹ 抗过拟合。
///
/// 本投影（§30 给出的低维例 `u=(level_bucket, δ, role, divergence_bucket)`）：
/// - `level_bucket`：级别分桶（[`UClass::level_bucket`]，把相邻 level 合并 ⟹ 压 |ℓ|）。
/// - `delta` δ：持仓方向（保留——方向是不可折叠的操作极性，§16）。
/// - `role`：仓位角色三值（Root / ChildTrend 顺势子 / ChildSwing 短差子）——把
///   `(parent_dir, short_swing, position)` 三维 z 分量折叠为一维语义角色（§16 操作态本质）。
/// - `divergence`：背驰二值（I_γ 6-bit 是否含一类买卖点 B1/S1）——把 {0,1}^6 的 I_γ 压成
///   bool（§5 一类买卖点=背驰确认，是操作上最强信号；2/3 类与是否伴一类的细分对 μ 贡献小，
///   §11 可被 OOS-value-gated 删维删去）。
///
/// **降维真实性**（测试断言）：|U| < |Z|——ϕ 是非单射满射（多个 z 映到同一 u），
/// `n_classes(U) < n_classes(Z)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UClass {
    pub level_bucket: u32,
    pub delta: i8,
    pub role: VoiceRole,
    pub divergence: bool,
}

/// 仓位角色（u 的分量）——`(parent_dir, short_swing, position)` 的语义折叠（§16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VoiceRole {
    /// 根声部（主趋势腿，无父）。
    Root,
    /// 顺势子声部（有父且同向，加仓腿）。
    ChildTrend,
    /// 短差子声部（有父且反向 δ=−σ_p，对冲腿）。
    ChildSwing,
}

impl UClass {
    /// 级别分桶（§30 压 |ℓ|）：相邻两级合并为一桶（`level/2`）。
    ///
    /// ponytail: `level/2` 整除分桶——最简单的相邻级别合并。若实测某分界点压掉了有 μ 区分度
    /// 的级别，改为显式 match 边界即可（OOS-gated 删维会暴露该需求）。
    pub fn level_bucket(level: u32) -> u32 {
        level / 2
    }

    /// 压缩映射 ϕ:Z→U（§30）——把高维 z 折叠到低维 u（确定性，同 z 恒映同 u）。
    ///
    /// **H 轴丢弃**（codex #81 `h_axis_in_default_selection: conditional`）：`z.horizontal` 不进 u——
    /// H(g) 是结构关系非操作极性，selection/降维层折叠掉以抗 winner's curse（§29-30）；H 只在 z
    /// 层报告保 R(g) 忠实。故本函数不读 `z.horizontal`（多个 H 的 z 映同一 u）。
    /// **σ_higher 同 H 丢弃**（codex-q1 G2 碎片化防护）：canonical z 完备 vs UClass 降维 selection
    /// 的既有分层直接复用——本函数不读 `z.sigma_higher`。
    pub fn project_to_u(z: &MuClass) -> UClass {
        let role = match (z.position, z.short_swing) {
            (PositionState::Root, _) => VoiceRole::Root,
            (PositionState::Child, true) => VoiceRole::ChildSwing,
            (PositionState::Child, false) => VoiceRole::ChildTrend,
        };
        // 背驰二值：I_γ 6-bit 是否含一类买卖点（B1=bit0=1 / S1=bit3=8，class_index 权重布局）。
        let divergence = z.i_class & 0b001001 != 0;
        UClass {
            level_bucket: UClass::level_bucket(z.level),
            delta: z.delta,
            role,
            divergence,
        }
    }
}

/// 单笔交易观测：分类值 z + 已实现交易收益 X_γ（§12 line 2147）。
///
/// `x_gamma` 是**已兑现**的 `δ(P_τγ−P_t)−C`——由调用方用真实 entry/exit 价格经
/// [`marginal_return`] 算出（F_τγ-可测，非端点后视）。μ 估计器只消费已实现值，不重算退出时刻。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MuObservation {
    pub class: MuClass,
    pub x_gamma: f64,
}

/// 单信号残差记录（alpha分离.pdf §1/§4.2 去污管线的逐笔载体，task #82）。
///
/// 残差减法 `Y_i = δ_i(H_i − B̂_i) − C_i`（alpha分离.pdf §1，p1-2）与分层置换（§4.2，p5-6：
/// 分层键 `s(i)=(ℓ, h bucket, time block, σ_higher)`）都在**残差**上做——[`MuObservation`] /
/// [`MuEstimator::trades`] 只携带 δ-baked 的 X_γ，无法支持残差置换（层内重新赋 δ 需 **δ-free**
/// 基 `H−B̂`，X_γ 已把原始 δ 烘进值里）。本记录补全该管线：
/// - `resid_base` r_i = H_i − B̂_i（**δ-free**；H_i=P_out−P_in 原始持有窗涨跌，B̂_i=持有窗市场漂移积分）。
/// - `cost` C_i（成本，与 δ 无关 ⟹ 置换 δ 时恒定）。
/// - `h_bucket` 持有期桶（§4.2 分层维；h_i=exit_bar−entry_bar 分桶，[`h_bucket`]）。
/// - `time_block` 时间块（§4.2 分层维；控制市场 regime 漂移，entry 位置分块）。
///
/// `class` 提供 (ℓ,δ,bsp_class,σ^H) 供分层/分桶。残差方向签名 PnL 见 [`ResidualTrade::y`]。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResidualTrade {
    pub class: MuClass,
    pub resid_base: f64,
    pub cost: f64,
    pub h_bucket: u8,
    pub time_block: u32,
}

impl ResidualTrade {
    /// 残差方向签名 PnL `Y_i = δ_i·(H_i−B̂_i) − C_i`（alpha分离.pdf §1，去 beta 后的可交易结构 alpha 基）。
    /// 与 X_i=δ_i·H_i−C_i 的关系：`Y_i = X_i − δ_i·B̂_i`（残差 = 原始 PnL 减去方向签名的持有窗 beta）。
    pub fn y(&self) -> f64 {
        self.class.delta as f64 * self.resid_base - self.cost
    }
}

/// 持有期桶（alpha分离.pdf §4.2 分层维 `h bucket`，p5）。`h`=exit_bar−entry_bar（bar 计）。
///
/// ponytail: 1-min bar 固定边界 <1h / 1-4h / 4h-1d / >1d（4 桶）。分层目的是把同持有量级的信号
/// 归组以控 B̂_i 同质性——若某标的 bar 频率不同或分位边界更贴合，改按标的分位切边即可（当前固定边界够用）。
pub fn h_bucket(h: usize) -> u8 {
    match h {
        0..=59 => 0,
        60..=239 => 1,
        240..=1439 => 2,
        _ => 3,
    }
}

/// 交易收益 X_γ = δ(P_τγ−P_t) − C_{t:τγ}（§12 line 2147）。
///
/// **复用** [`trade_abs_pnl`]（metrics.rs 单一来源的方向感知 PnL：多头 `q(P_τ(1−f)−P_t(1+f))`，
/// 空头 `q(P_t(1−f)−P_τ(1+f))`，canonical 镜像 σ→−σ）——X_γ 的 `δ·(·)−C` 正是该公式：方向由
/// `delta` 选公式（δ=+1 long / δ=−1 short），成本 C 由 `fee_rate` 双边扣（建+平仓各 ·(1±f)）。
///
/// 单位诚实：返回**绝对**收益（与 metrics 一致），非归一化——调用方若需归一化自行 ÷nav_base。
///
/// # Panics（debug）
/// `delta ∉ {+1,−1}` ⟹ debug 断言失败（z 的 δ 只能是买/卖方向，fail-fast 非静默）。
pub fn marginal_return(
    entry_px: f64,
    exit_px: f64,
    qty: f64,
    fee_rate: f64,
    delta: i8,
) -> f64 {
    debug_assert!(delta == 1 || delta == -1, "δ 必须 ∈ {{+1,−1}}，收到 {delta}");
    trade_abs_pnl(entry_px, exit_px, qty, fee_rate, delta == 1)
}

/// 单 z 桶的 Welford 在线均值/方差累加器（Welford 1962，数值稳定，无 ΣX² 灾难性抵消）。
///
/// 状态 `(n, mean, m2)`：`mean = ΣX_γ/n`，`m2 = Σ(X_γ−mean)²`。样本方差 = `m2/(n−1)`
/// （无偏，贝塞尔校正；n<2 时未定义 ⟹ 见 [`Welford::std_sample`]）。LCB 需方差 ⟹ 必须
/// 存二阶量；选 Welford 而非 (ΣX,ΣX²) 因后者大样本下 ΣX² 与 (ΣX)²/n 相减灾难性抵消。
#[derive(Debug, Clone, Copy, Default)]
struct Welford {
    n: u64,
    mean: f64,
    m2: f64,
}

impl Welford {
    /// 累加一个观测（Welford 递推，O(1)）。
    fn push(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    /// 样本标准差 √(m2/(n−1))（无偏方差开方）。`n<2` ⟹ `None`（方差未定义）。
    fn std_sample(&self) -> Option<f64> {
        if self.n < 2 {
            None
        } else {
            Some((self.m2 / (self.n - 1) as f64).sqrt())
        }
    }
}

/// μ(z) 条件边际收益估计器（§12 line 2173 样本均值）。
///
/// 按 [`MuClass`] z 分桶 Welford 累加 X_γ，`mu(z) = mean_z`（条件期望的样本估计），
/// 并存方差供 [`MuEstimator::mu_lcb`] 算置信下界（§12 实操选择器 χ_t 用 LCB(μ) 非裸 μ，
/// 防高维 z 过拟合估计噪声）。**不做** χ_θ 过滤 / argmax 选择（下游工位）。
#[derive(Debug, Clone, Default)]
pub struct MuEstimator {
    // ponytail: 全量逐笔留存供 perm_test 置换（Welford 聚合量算不出置换）；OOS BTC 数万笔可接受。
    trades: Vec<(MuClass, f64)>,
    /// z → Welford(n, mean, m2)。样本均值 = mean（[`MuEstimator::mu`]）。
    buckets: HashMap<MuClass, Welford>,
}

impl MuEstimator {
    pub fn new() -> Self {
        MuEstimator::default()
    }

    /// 累加一笔观测到对应 z 桶（Welford 在线递推，O(1) 摊销）。
    pub fn observe(&mut self, obs: MuObservation) {
        self.buckets.entry(obs.class).or_default().push(obs.x_gamma);
        self.trades.push((obs.class, obs.x_gamma));
    }

    /// 逐笔明细（perm_test 输入，投影为 (ℓ,bsp_class,δ,X_γ)）。
    pub fn trades(&self) -> &[(MuClass, f64)] {
        &self.trades
    }

    /// 变异系数 CV = σ̂/|μ̂|（§3.1 功效门输入）。None ⟹ n<2 或 μ̂=0（判 ¬powered）。
    pub fn cv(&self, class: &MuClass) -> Option<f64> {
        let w = self.buckets.get(class)?;
        let std = w.std_sample()?;
        if w.mean == 0.0 { None } else { Some(std / w.mean.abs()) }
    }

    /// 批量累加（迭代器 fold，等价逐笔 [`MuEstimator::observe`]）。
    pub fn observe_all(&mut self, obs: impl IntoIterator<Item = MuObservation>) {
        for o in obs {
            self.observe(o);
        }
    }

    /// μ(z) = E[X_γ|Z=z] 样本估计（§12）。
    ///
    /// 返回 `Some(mean)`（该 z 有观测），`None`（该 z 无样本——空类无估计，**不**冒充 μ=0；
    /// 空类与 μ=0 是不同认识状态：前者无数据，后者有数据且均值为 0）。
    pub fn mu(&self, class: &MuClass) -> Option<f64> {
        self.buckets.get(class).map(|w| w.mean)
    }

    /// LCB(μ(z)) = mean − z_α·(std/√n) 单边置信下界（§12 实操选择器 χ_t 的准入量）。
    ///
    /// `z_alpha` 是单边正态分位（如 95%→1.645，99%→2.326）——由调用方按置信水平传入
    /// （估计器不绑定分布表，下游 selector 决定置信水平）。语义（诚实标注）：
    /// - `n≥2`：`Some(mean − z_alpha·std/√n)`，标准误 std/√n 随 √n 收敛 ⟹ LCB→mean。
    /// - `n=1`：标准差未定义 ⟹ `None`（**不**冒充 LCB=mean——单样本无方差信息，给不出
    ///   收缩后的保守下界；下游 selector 自行决定单样本是否准入，估计器不替它造数）。
    /// - 无样本：`None`（与 [`MuEstimator::mu`] 一致——空类无估计）。
    ///
    /// LCB≤mean 恒成立（`z_alpha≥0` 且 std/√n≥0）⟹ 置信下界不超过点估计（不乐观）。
    pub fn mu_lcb(&self, class: &MuClass, z_alpha: f64) -> Option<f64> {
        let w = self.buckets.get(class)?;
        let std = w.std_sample()?; // n<2 ⟹ None
        Some(w.mean - z_alpha * std / (w.n as f64).sqrt())
    }

    /// UCB(μ(z)) = mean + z_α·(std/√n) 单边置信上界（三态判据 FALSIFIED 的准入量）。
    ///
    /// 与 [`MuEstimator::mu_lcb`] 严格对称——同 `z_alpha`、同标准误 std/√n，符号相反。
    /// 用途（acc-alpha 预注册 §3.1 三态判定）：`powered ∧ LCB≤0 ∧ UCB≤0` ⟹ FALSIFIED
    /// （有功效地判定 μ≤0，真纯 beta）；`LCB≤0<UCB` ⟹ INCONCLUSIVE（判据无检出力，
    /// `LCB≤0 ⊬ μ≤0`，667/231）。缺 UCB 则无法区分「有功效地否证」与「underpowered」——
    /// 这是 §6 上报矛盾的形式化落点。语义（诚实标注，镜像 mu_lcb）：
    /// - `n≥2`：`Some(mean + z_alpha·std/√n)`，标准误随 √n 收敛 ⟹ UCB→mean。
    /// - `n=1`：标准差未定义 ⟹ `None`（不冒充 UCB=mean——单样本无方差信息）。
    /// - 无样本：`None`（空类无估计）。
    ///
    /// UCB≥mean 恒成立（`z_alpha≥0` 且 std/√n≥0）⟹ 置信上界不低于点估计（不悲观）。
    pub fn mu_ucb(&self, class: &MuClass, z_alpha: f64) -> Option<f64> {
        let w = self.buckets.get(class)?;
        let std = w.std_sample()?; // n<2 ⟹ None
        Some(w.mean + z_alpha * std / (w.n as f64).sqrt())
    }

    /// 该 z 类的样本量 |S_z|（统计功效判定用——小样本 μ 估计不可靠）。
    pub fn count(&self, class: &MuClass) -> u64 {
        self.buckets.get(class).map_or(0, |w| w.n)
    }

    /// pooled 均值 μ_pooled = (Σ_z n_z·mean_z)/(Σ_z n_z)，聚合范围 = 同 `(level, delta)`
    /// 的所有 z 类（PDF §8 收缩目标 = 同级别同多空 pooled mean）。
    ///
    /// 加权合并各桶 `(n, mean)`——`Σ n·mean = ΣΣX_γ` ⟹ 等于把同 (level,delta) 全部 X_γ
    /// 拉平后求总均值（与逐笔合并 bit-exact）。`None` ⟹ 该 (level,delta) 无任何样本。
    fn pooled_mean(&self, level: u32, delta: i8) -> Option<f64> {
        let mut sum = 0.0_f64;
        let mut n_total = 0_u64;
        for (z, w) in &self.buckets {
            if z.level == level && z.delta == delta {
                sum += w.mean * w.n as f64;
                n_total += w.n;
            }
        }
        if n_total == 0 {
            None
        } else {
            Some(sum / n_total as f64)
        }
    }

    /// 层级收缩 μ_shrink(z) = w_z·mean_z + (1−w_z)·μ_pooled（PDF §8，样本稀疏类抗过拟合）。
    ///
    /// `w_z = n_z/(n_z + σ²_z/τ²)`，`σ²_z` = 本类样本方差（Welford `m2/(n−1)`），`tau_sq` τ² =
    /// 级别间先验方差（PDF θ_ℓ~N(0,τ_ℓ²)，由调用方传入）。`μ_pooled` = 同 (level,delta) 的
    /// [`pooled_mean`]（收缩目标）。
    ///
    /// 语义（诚实标注，与 [`MuEstimator::mu_lcb`] 的 None 语义**区别**）：
    /// - `n_z≥2`：σ²_z 有定义 ⟹ `Some(w_z·mean_z + (1−w_z)·μ_pooled)`。n_z 大 ⟹ w_z→1
    ///   ⟹ 信本类均值；n_z 小 ⟹ w_z→0 ⟹ 收缩到 pooled。
    /// - `n_z<2`：样本方差未定义 ⟹ `w_z=0` ⟹ **完全收缩到 μ_pooled**（样本太少就别信它，
    ///   借 pooled 估计——这是收缩的意义，与 mu_lcb「拒绝返 None」相反：mu_shrink 保功效）。
    /// - 该 (level,delta) 无任何样本（连 pooled 都没有）⟹ `None`（无可借的估计）。
    ///
    /// `tau_sq≤0` ⟹ debug 断言失败（先验方差须正——τ²=0 退化为分母 ∞ ⟹ w_z=0 全收缩，
    /// τ²<0 无意义，fail-fast 非静默）。
    pub fn mu_shrink(&self, class: &MuClass, tau_sq: f64) -> Option<f64> {
        debug_assert!(tau_sq > 0.0, "τ²（tau_sq）须 > 0，收到 {tau_sq}");
        let pooled = self.pooled_mean(class.level, class.delta)?;
        let w = self.buckets.get(class)?;
        // n<2 ⟹ 样本方差未定义 ⟹ w_z=0 ⟹ 完全收缩到 pooled（保功效，非拒绝）。
        let w_z = match w.std_sample() {
            None => 0.0,
            Some(std) => {
                let var_z = std * std; // σ²_z
                w.n as f64 / (w.n as f64 + var_z / tau_sq)
            }
        };
        Some(w_z * w.mean + (1.0 - w_z) * pooled)
    }

    /// 收缩视图：返回新 [`MuEstimator`]，每桶 `mean ← mu_shrink(z,τ²)`、**保留原 n/m2**（acc-three-way-l2）。
    ///
    /// 用途：让 frozen selector（[`super::runner::run_theta_v0_pi_chi`]，准入 `mu(z)>θ`）对**收缩后**
    /// 的 μ 兑现 ΔR——selector 一行不改，只换喂给它的 est。三路对比中 shrinkage 路 = `shrunk_view(τ²)`
    /// 配 `z_alpha=0`（裸 μ 门），与裸 μ/LCB 同 walk-forward harness。保留原 n ⟹ n_L3 池大小三路可比
    /// （shrinkage 卖点 = 保功效，可被 [`MuEstimator::count`] / n_L3 验证）。
    ///
    /// **m2 保留是已知陷阱**：mean 被收缩但 m2（⟹ std）仍是原始样本 ⟹ 对 shrunk_view 再求
    /// [`MuEstimator::mu_lcb`] 会得到 `shrunk_mean − z_α·orig_std/√n`，**LCB 语义失真**（下界基于的
    /// mean 不再是该桶样本均值）。故 **shrunk_view 仅配 `z_alpha=0` 裸门用**，禁止再 LCB。
    ///
    /// n<2 桶：[`MuEstimator::mu_shrink`] 完全收缩到 pooled（w_z=0），新桶 mean=pooled、n/m2 不变。
    /// `mu_shrink` 仅当 pooled None 才返 None，而该桶 ∈ 自己的 (level,delta) pooled ⟹ pooled 必 Some
    /// ⟹ unwrap_or 的 fallback 不可达（保险保留原 mean，不 panic）。
    pub fn shrunk_view(&self, tau_sq: f64) -> MuEstimator {
        debug_assert!(tau_sq > 0.0, "τ²（tau_sq）须 > 0，收到 {tau_sq}");
        let mut buckets = HashMap::with_capacity(self.buckets.len());
        for (z, w) in &self.buckets {
            let shrunk_mean = self.mu_shrink(z, tau_sq).unwrap_or(w.mean);
            buckets.insert(*z, Welford { n: w.n, mean: shrunk_mean, m2: w.m2 });
        }
        // 收缩仅改 bucket 的 mean，不改原始逐笔——perm_test 置换基于 trades，故视图须保留 trades。
        // （预存编译缺口修复：commit 8ccbe58137 加 `trades` 字段时漏改本构造子；见 ws-gap3-bridge 汇报。）
        MuEstimator { buckets, trades: self.trades.clone() }
    }

    /// 合并另一估计器的全部桶（Chan/Welford 并行合并，bit-exact 等价逐笔顺序累加同一桶）。
    ///
    /// 用途（cross-fit OOS，acc-crossfit-oos）：K-fold 的 train 是**不连续时间块**（held-out fold
    /// 在中间时 train=前段+后段）。各连续段独立 [`build_walk_forward_mu`] 估 μ 后合并到一个 est——
    /// **不拼接成单 Dataset**（拼接会在接缝处制造虚假相邻笔/段，污染分类）。
    ///
    /// 合并公式（Chan et al. 1979 并行 Welford，无灾难性抵消）：
    /// `n=n_a+n_b`，`δ=mean_b−mean_a`，`mean=mean_a+δ·n_b/n`，`m2=m2_a+m2_b+δ²·n_a·n_b/n`。
    pub fn merge(&mut self, other: &MuEstimator) {
        for (z, wb) in &other.buckets {
            let wa = self.buckets.entry(*z).or_default();
            if wb.n == 0 {
                continue;
            }
            if wa.n == 0 {
                *wa = *wb;
                continue;
            }
            let n = wa.n + wb.n;
            let delta = wb.mean - wa.mean;
            let mean = wa.mean + delta * (wb.n as f64) / (n as f64);
            let m2 = wa.m2 + wb.m2 + delta * delta * (wa.n as f64) * (wb.n as f64) / (n as f64);
            *wa = Welford { n, mean, m2 };
        }
    }

    /// 已观测的全部 z 类及其 μ 估计（按需消费；顺序不定，HashMap 无序）。
    pub fn iter_mu(&self) -> impl Iterator<Item = (MuClass, f64)> + '_ {
        self.buckets.iter().map(|(z, w)| (*z, w.mean))
    }

    /// 已观测的全部 z 类及其 `(n, mean, var_sample)`——跨品种 pooling/ICC 方差分解用
    /// （[`super::pooling_icc`]）。`var_sample = m2/(n−1)`（无偏样本方差），`n<2` ⟹ `None`
    /// （单样本类内方差未定义，与 [`MuEstimator::mu_lcb`] 同诚实语义）。顺序不定（HashMap）。
    pub fn iter_class_stats(&self) -> impl Iterator<Item = (MuClass, u64, f64, Option<f64>)> + '_ {
        self.buckets
            .iter()
            .map(|(z, w)| (*z, w.n, w.mean, w.std_sample().map(|s| s * s)))
    }

    /// 已观测 z 类的数量（分桶覆盖了多少互斥类别）。
    pub fn n_classes(&self) -> usize {
        self.buckets.len()
    }
}

/// 低维 μ(u) 估计器（《全互斥定义策略2》§29-§30 维数控制 ϕ:Z→U 的聚合）。
///
/// 把高维 z 桶经压缩映射 [`UClass::project_to_u`] 重聚合到低维 u 桶——多个 z 映同一 u ⟹
/// 各 z 的 X_γ 合流到一个 u 桶 ⟹ n̄(u)=M/|U| > n̄(z)=M/|Z|（§29 升每类样本，抗 winner's
/// curse）。聚合用 Chan 并行 Welford（[`MuEstimator::merge`] 同核），与逐笔重路由 bit-exact。
///
/// 认识论等级：聚合逻辑 L1（确定性变换），真实数据驱动的 μ(u) 值 L2（同 [`MuEstimator`] 标注）。
#[derive(Debug, Clone, Default)]
pub struct UEstimator {
    buckets: HashMap<UClass, Welford>,
}

impl UEstimator {
    /// 从已估好的高维 [`MuEstimator`] 按 ϕ:Z→U 聚合（§30 折叠）。
    ///
    /// 每个 z 桶 `(n, mean, m2)` 经 [`UClass::project_to_u`] 并入其像 u 桶——用 Chan 并行
    /// Welford 合并（与 [`MuEstimator::merge`] 同公式）⟹ μ(u) = 落入 u 的全部 X_γ 的总均值
    /// （与逐笔按 u 累加 bit-exact），方差/n 同样合并（供功效判定）。
    pub fn from_z(z_est: &MuEstimator) -> UEstimator {
        let mut buckets: HashMap<UClass, Welford> = HashMap::new();
        for (z, wb) in &z_est.buckets {
            if wb.n == 0 {
                continue;
            }
            let u = UClass::project_to_u(z);
            let wa = buckets.entry(u).or_default();
            if wa.n == 0 {
                *wa = *wb;
                continue;
            }
            let n = wa.n + wb.n;
            let delta = wb.mean - wa.mean;
            let mean = wa.mean + delta * (wb.n as f64) / (n as f64);
            let m2 = wa.m2 + wb.m2 + delta * delta * (wa.n as f64) * (wb.n as f64) / (n as f64);
            *wa = Welford { n, mean, m2 };
        }
        UEstimator { buckets }
    }

    /// μ(u) 样本均值（§12 类比，u 域）。`None` ⟹ 空类（与 [`MuEstimator::mu`] 同语义）。
    pub fn mu(&self, class: &UClass) -> Option<f64> {
        self.buckets.get(class).map(|w| w.mean)
    }

    /// 该 u 类样本量 |S_u|（功效判定——u 域 n̄ 应 > z 域）。
    pub fn count(&self, class: &UClass) -> u64 {
        self.buckets.get(class).map_or(0, |w| w.n)
    }

    /// 已观测 u 类数 |U|（降维真实性：应 < z 域 [`MuEstimator::n_classes`]）。
    pub fn n_classes(&self) -> usize {
        self.buckets.len()
    }

    /// 已观测 u 类及 μ 估计（顺序不定）。
    pub fn iter_mu(&self) -> impl Iterator<Item = (UClass, f64)> + '_ {
        self.buckets.iter().map(|(u, w)| (*u, w.mean))
    }
}

/// 可删维度（《全互斥定义策略2》§11 OOS-value-gated 删维的候选轴）。
///
/// §11：「不能提升 OOS value 的维度删除/正则化」。本枚举列出 u 的可删维度轴——
/// [`oos_gated_drop`] 对每个轴评估「删该维度后 OOS value 是否不降」，标可删维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropAxis {
    /// 级别桶维度（删 ⟹ 不按级别分类）。
    LevelBucket,
    /// 角色维度（删 ⟹ root/child 不分）。
    Role,
    /// 背驰维度（删 ⟹ 不按一类买卖点分）。
    Divergence,
}

/// OOS-value-gated 删维评估（§11 骨架）。
///
/// §11：「不能提升 OOS value 的维度删除/正则化」。给定待评估维度轴与「删该轴后的 OOS value
/// 评估函数」`oos_value`（由调用方提供——它依赖 walk-forward harness 兑现的样本外 ΔR，是
/// L2/L3 量，估计器不内造），返回每个轴的「删除后 OOS value 不降 ⟹ 可删」判定。
///
/// **诚实声明（骨架边界）**：本函数只编排「逐轴调用 OOS 评估 + 比较基线」的判定逻辑——真正的
/// OOS value（删维前/后两次 walk-forward 回测的样本外收益）由调用方 `oos_value` 闭包提供。
/// 估计器**不计算** OOS value（那需完整回测管线 + 真实数据，是下游 acc-crossfit-oos 工位的
/// 产出）。`baseline` = 不删任何维度（全维 u）的 OOS value。
///
/// 判定：`oos_value(axis) >= baseline − tol` ⟹ 该轴可删（删它 OOS 不降，§11）。`tol` 容忍
/// 噪声（删维后 OOS value 微降在 tol 内仍算「不降」，避免噪声驱动的保留）。
pub fn oos_gated_drop<F>(
    axes: &[DropAxis],
    baseline: f64,
    tol: f64,
    mut oos_value: F,
) -> Vec<(DropAxis, bool)>
where
    F: FnMut(DropAxis) -> f64,
{
    axes.iter()
        .map(|&axis| (axis, oos_value(axis) >= baseline - tol))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_bits() -> BspBits {
        BspBits { buy1: true, ..Default::default() }
    }

    #[test]
    fn cv_is_std_over_abs_mean() {
        let z = MuClass::from_certificate(1,1,buy_bits(),0,PositionState::Root);
        let mut e = MuEstimator::new();
        e.observe(MuObservation{class:z,x_gamma:8.0});
        e.observe(MuObservation{class:z,x_gamma:12.0});
        assert!((e.cv(&z).unwrap()-8.0_f64.sqrt()/10.0).abs()<1e-9);
        let z2=MuClass::from_certificate(2,1,buy_bits(),0,PositionState::Root);
        let mut e2=MuEstimator::new(); e2.observe(MuObservation{class:z2,x_gamma:5.0});
        assert_eq!(e2.cv(&z2),None);
    }

    /// X_γ = δ(P_τ−P_t)−C 与 metrics 方向感知 PnL bit-exact 一致（复用单一来源，无公式漂移）。
    #[test]
    fn marginal_return_matches_directional_pnl() {
        // 多头 δ=+1：低买(100)高卖(110)，零成本 ⟹ +10。
        assert_eq!(marginal_return(100.0, 110.0, 1.0, 0.0, 1), 10.0);
        // 空头 δ=−1：高卖(120)低买(100)，零成本 ⟹ +20（空头盈利）。
        assert_eq!(marginal_return(120.0, 100.0, 1.0, 0.0, -1), 20.0);
        // 空头在上涨(100→110)亏损 ⟹ −10。
        assert_eq!(marginal_return(100.0, 110.0, 1.0, 0.0, -1), -10.0);
    }

    /// 残差 Y_i = δ(H−B̂)−C，且 Y_i = X_i − δ·B̂（alpha分离.pdf §1）。
    #[test]
    fn residual_trade_y_is_signed_debeta_minus_cost() {
        let z = MuClass::from_certificate(0, 1, buy_bits(), 0, PositionState::Root);
        // H=100, B̂=30, C=5, δ=+1 ⟹ Y = 1·(100−30) − 5 = 65。
        let rt = ResidualTrade { class: z, resid_base: 100.0 - 30.0, cost: 5.0, h_bucket: 0, time_block: 0 };
        assert!((rt.y() - 65.0).abs() < 1e-12);
        // Y = X − δ·B̂：X = δ·H − C = 100 − 5 = 95；δ·B̂ = 30 ⟹ Y = 95 − 30 = 65。
        let (h, b_hat, c) = (100.0, 30.0, 5.0);
        let x = 1.0 * h - c;
        assert!((rt.y() - (x - 1.0 * b_hat)).abs() < 1e-12, "Y = X − δ·B̂");
        // 卖方向 δ=−1：H=−40（下跌）, B̂=−20（下漂）, C=3 ⟹ Y = −1·(−40−(−20)) − 3 = 20 − 3 = 17。
        let z_sell = MuClass::from_certificate(0, -1, BspBits { sell1: true, ..Default::default() }, 0, PositionState::Root);
        let rt_s = ResidualTrade { class: z_sell, resid_base: -40.0 - (-20.0), cost: 3.0, h_bucket: 0, time_block: 0 };
        assert!((rt_s.y() - 17.0).abs() < 1e-12);
    }

    /// 持有期桶单调边界（§4.2 h bucket，1-min bar：<1h/1-4h/4h-1d/>1d）。
    #[test]
    fn h_bucket_monotone_edges() {
        assert_eq!(h_bucket(0), 0);
        assert_eq!(h_bucket(59), 0);
        assert_eq!(h_bucket(60), 1);
        assert_eq!(h_bucket(239), 1);
        assert_eq!(h_bucket(240), 2);
        assert_eq!(h_bucket(1439), 2);
        assert_eq!(h_bucket(1440), 3);
        assert_eq!(h_bucket(100_000), 3);
    }

    /// 成本 C_{t:τ} 双边扣（建+平仓各 ·(1±fee)）⟹ X_γ 比零成本低。
    #[test]
    fn marginal_return_deducts_cost() {
        let gross = marginal_return(100.0, 110.0, 1.0, 0.0, 1);
        let net = marginal_return(100.0, 110.0, 1.0, 0.01, 1);
        assert!(net < gross, "含成本 X_γ({net}) 必 < 零成本({gross})");
    }

    /// μ(z) = ΣX_γ/|S_z| 样本均值（§12 line 2173）。
    #[test]
    fn mu_is_sample_mean_per_class() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe_all([
            MuObservation { class: z, x_gamma: 10.0 },
            MuObservation { class: z, x_gamma: 20.0 },
            MuObservation { class: z, x_gamma: 30.0 },
        ]);
        assert_eq!(est.mu(&z), Some(20.0)); // (10+20+30)/3
        assert_eq!(est.count(&z), 3);
    }

    /// 不同 z 不混（§18 全互斥）——同级别同方向但 I_γ 不同 ⟹ 不同桶，μ 独立。
    #[test]
    fn distinct_classes_do_not_mix() {
        let z_b1 = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let z_b2 = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        assert_ne!(z_b1, z_b2, "B1 与 B2 是不同 I_γ ⟹ 不同 z");
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_b1, x_gamma: 10.0 });
        est.observe(MuObservation { class: z_b2, x_gamma: -10.0 });
        assert_eq!(est.mu(&z_b1), Some(10.0));
        assert_eq!(est.mu(&z_b2), Some(-10.0)); // §12 line 2186：μ≤0 该类无正期望
        assert_eq!(est.n_classes(), 2);
    }

    /// I_γ 不压扁：2B+3B 重合（buy2∧buy3）是与单一 buy2 不同的 z（§P4 §5 非互斥三分）。
    #[test]
    fn i_gamma_not_collapsed_coincident_bsp() {
        let only_b2 = MuClass::from_certificate(
            1,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let b2_and_b3 = MuClass::from_certificate(
            1,
            1,
            BspBits { buy2: true, buy3: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        assert_ne!(only_b2.i_class, b2_and_b3.i_class, "重合买卖点保留为不同 I_γ");
    }

    /// 短差判定：子声部 δ=−σ_p ⟹ short_swing=true（§6/§16 σ_u=−σ_p 对冲腿）。
    #[test]
    fn short_swing_when_child_opposes_parent() {
        // 父向上 σ_p=+1，子声部 δ=−1（反向）⟹ 短差。
        let short = MuClass::from_certificate(2, -1, buy_bits(), 1, PositionState::Child);
        assert!(short.short_swing);
        // 子声部同向 δ=+1=σ_p ⟹ 顺势（非短差）。
        let trend = MuClass::from_certificate(2, 1, buy_bits(), 1, PositionState::Child);
        assert!(!trend.short_swing);
        // 根声部 parent_dir=0 ⟹ 恒顺势（无父可对冲）。
        let root = MuClass::from_certificate(2, 1, buy_bits(), 0, PositionState::Root);
        assert!(!root.short_swing);
    }

    /// 空类返回 None（无样本 ≠ μ=0；不冒充估计）。
    #[test]
    fn empty_class_returns_none_not_zero() {
        let z = MuClass::from_certificate(5, 1, buy_bits(), 0, PositionState::Root);
        let est = MuEstimator::new();
        assert_eq!(est.mu(&z), None);
        assert_eq!(est.mu_lcb(&z, 1.645), None);
        assert_eq!(est.count(&z), 0);
    }

    /// LCB ≤ mean 恒成立（置信下界不超过点估计——z_α≥0 且 std/√n≥0，不乐观）。
    #[test]
    fn lcb_never_exceeds_mean() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe_all([
            MuObservation { class: z, x_gamma: 5.0 },
            MuObservation { class: z, x_gamma: 15.0 },
            MuObservation { class: z, x_gamma: 10.0 },
        ]);
        let mean = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, 1.645).unwrap();
        assert!(lcb <= mean, "LCB({lcb}) 必 ≤ mean({mean})");
    }

    /// LCB 单调性：固定均值/标准差，n 增大 ⟹ 标准误 std/√n 收缩 ⟹ LCB 单调逼近 mean。
    /// 用同一对称样本 {mean−s, mean+s} 重复 k 份 ⟹ mean、样本 std 不变，仅 n 变。
    #[test]
    fn lcb_converges_to_mean_as_n_grows() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let (m, s) = (20.0_f64, 4.0_f64); // 每对 {16,24}：均值 20，n 趋大时样本 std→s
        let lcb_at = |reps: usize| -> f64 {
            let mut est = MuEstimator::new();
            for _ in 0..reps {
                est.observe(MuObservation { class: z, x_gamma: m - s });
                est.observe(MuObservation { class: z, x_gamma: m + s });
            }
            est.mu_lcb(&z, 1.645).unwrap()
        };
        let (lcb_small, lcb_large) = (lcb_at(2), lcb_at(50)); // n=4 vs n=100
        // 均值恒为 20，样本 std 在两规模下都 ≈4（对称样本）⟹ 仅 √n 不同。
        assert!(lcb_large > lcb_small, "n↑ ⟹ LCB 上移逼近 mean：{lcb_large} > {lcb_small}");
        assert!(lcb_large < m, "LCB 仍 < mean（n 有限，标准误 > 0）");
        assert!((m - lcb_large) < (m - lcb_small) * 0.3, "√n 收敛：大样本 gap 显著缩小");
    }

    /// UCB 与 LCB 对称：同 z_α 下 LCB≤mean≤UCB，且 UCB−mean = mean−LCB（同标准误，符号相反）。
    /// FALSIFIED 判据（powered ∧ LCB≤0 ∧ UCB≤0）依赖 UCB 与 LCB 的这层对称——mean<0 时二者才可能同 ≤0。
    #[test]
    fn ucb_symmetric_to_lcb_straddles_mean() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe_all([
            MuObservation { class: z, x_gamma: 5.0 },
            MuObservation { class: z, x_gamma: 15.0 },
            MuObservation { class: z, x_gamma: 10.0 },
        ]);
        let mean = est.mu(&z).unwrap();
        let lcb = est.mu_lcb(&z, 1.645).unwrap();
        let ucb = est.mu_ucb(&z, 1.645).unwrap();
        assert!(lcb <= mean && mean <= ucb, "LCB({lcb}) ≤ mean({mean}) ≤ UCB({ucb})");
        assert!((ucb - mean) - (mean - lcb) < 1e-9, "对称：UCB−mean = mean−LCB");
    }

    /// n=1 边界：UCB 与 LCB 同诚实语义——单样本方差未定义 ⟹ 均返回 None。
    #[test]
    fn ucb_undefined_for_single_sample() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 42.0 });
        assert_eq!(est.mu_ucb(&z, 1.645), None, "单样本方差未定义 ⟹ UCB None");
    }

    /// n=1 边界：单样本方差未定义 ⟹ mu_lcb 返回 None（不冒充 LCB=mean，诚实语义）。
    /// mu(z) 仍返回该单点（点估计有定义，与 LCB 语义分离）。
    #[test]
    fn lcb_undefined_for_single_sample() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z, x_gamma: 42.0 });
        assert_eq!(est.mu(&z), Some(42.0), "单样本点估计有定义");
        assert_eq!(est.mu_lcb(&z, 1.645), None, "单样本方差未定义 ⟹ LCB None");
        assert_eq!(est.count(&z), 1);
    }

    /// 收缩单调性：固定 mean_z/pooled/σ²_z，n_z↑ ⟹ w_z↑ ⟹ mu_shrink 单调逼近 mean_z。
    /// 同一对称样本 {mean−s,mean+s} 重复 k 份 ⟹ mean_z、σ²_z 不变，仅 n 变。另设一个高样本
    /// 同 (level,delta) 邻类拉低 pooled，使 mean_z≠pooled ⟹ 收缩方向可观测。
    #[test]
    fn shrink_converges_to_mean_as_n_grows() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        // 邻类：同 level=3 同 delta=+1，I_γ 不同 ⟹ 拉低 pooled（大量低收益样本）。
        let neighbor = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let (m, s) = (50.0_f64, 4.0_f64); // z 类均值 50，σ²_z 恒定
        let shrink_at = |reps: usize| -> f64 {
            let mut est = MuEstimator::new();
            // 邻类灌 1000 笔 x=0 ⟹ pooled 被强拉向 0（远离 z 的 50）。
            for _ in 0..1000 {
                est.observe(MuObservation { class: neighbor, x_gamma: 0.0 });
            }
            for _ in 0..reps {
                est.observe(MuObservation { class: z, x_gamma: m - s });
                est.observe(MuObservation { class: z, x_gamma: m + s });
            }
            est.mu_shrink(&z, 1.0).unwrap()
        };
        let (small, large) = (shrink_at(1), shrink_at(50)); // n_z=2 vs n_z=100
        assert!(large > small, "n_z↑ ⟹ w_z↑ ⟹ 收缩值上移逼近 mean：{large} > {small}");
        assert!(large < m, "n_z 有限 ⟹ w_z<1 ⟹ 仍 < mean_z");
        assert!(small > 0.0, "即使 n_z 小，w_z>0 ⟹ 未完全坍到 pooled(≈0)");
    }

    /// 收缩方向：n_z=1 高偏离类 ⟹ w_z=0 ⟹ mu_shrink 完全坍到 pooled（vs 裸 mu 不拉）。
    /// 这是 n<2 保功效语义（借 pooled），与 mu_lcb 的 None 拒绝相反。
    #[test]
    fn shrink_single_sample_collapses_to_pooled() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let neighbor = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        let mut est = MuEstimator::new();
        // 邻类大量 x=10 ⟹ 主导 pooled。
        for _ in 0..100 {
            est.observe(MuObservation { class: neighbor, x_gamma: 10.0 });
        }
        // z 单样本极端偏离值 1000。
        est.observe(MuObservation { class: z, x_gamma: 1000.0 });
        assert_eq!(est.mu(&z), Some(1000.0), "裸 μ 不拉，仍是单样本值");
        // pooled = (100·10 + 1·1000)/101 ≈ 19.8；n_z=1 ⟹ w_z=0 ⟹ 完全坍到 pooled。
        let pooled = (100.0 * 10.0 + 1000.0) / 101.0;
        let shrunk = est.mu_shrink(&z, 1.0).unwrap();
        assert!((shrunk - pooled).abs() < 1e-9, "n=1 ⟹ 完全收缩到 pooled：{shrunk} ≈ {pooled}");
        assert_ne!(est.mu_lcb(&z, 1.645), Some(shrunk), "mu_lcb n=1 返 None，与 mu_shrink 语义分离");
    }

    /// pooled 聚合正确性：同 (level,delta) 按样本加权聚合，跨 level / 跨 delta 不混。
    #[test]
    fn pooled_mean_aggregates_same_level_delta_only() {
        let z_a = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        // 同 level=3 同 delta=+1，I_γ 不同 ⟹ 进同一 pooled。
        let z_b = MuClass::from_certificate(
            3,
            1,
            BspBits { buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        // 跨 level（5≠3）⟹ 不进 pooled(3,+1)。
        let z_other_level = MuClass::from_certificate(5, 1, buy_bits(), 0, PositionState::Root);
        // 跨 delta（−1≠+1）⟹ 不进 pooled(3,+1)。
        let z_other_delta = MuClass::from_certificate(3, -1, buy_bits(), 0, PositionState::Root);
        let mut est = MuEstimator::new();
        est.observe(MuObservation { class: z_a, x_gamma: 10.0 }); // n=1
        est.observe(MuObservation { class: z_b, x_gamma: 30.0 }); // n=1
        est.observe(MuObservation { class: z_b, x_gamma: 50.0 }); // n=2 ⟹ z_b 均值 40
        est.observe(MuObservation { class: z_other_level, x_gamma: 1000.0 });
        est.observe(MuObservation { class: z_other_delta, x_gamma: -1000.0 });
        // pooled(3,+1) = (10 + 30 + 50)/3 = 30（z_a 1 笔 + z_b 2 笔；其他 level/delta 不混）。
        assert_eq!(est.pooled_mean(3, 1), Some(30.0));
        // pooled(5,+1) 只含 z_other_level。
        assert_eq!(est.pooled_mean(5, 1), Some(1000.0));
        // pooled(3,−1) 只含 z_other_delta。
        assert_eq!(est.pooled_mean(3, -1), Some(-1000.0));
        // 无样本的 (level,delta) ⟹ None。
        assert_eq!(est.pooled_mean(7, 1), None);
    }

    /// 无样本 z 且其 (level,delta) 也无样本 ⟹ mu_shrink None（无可借估计）。
    #[test]
    fn shrink_none_when_no_pooled_sample() {
        let z = MuClass::from_certificate(9, 1, buy_bits(), 0, PositionState::Root);
        let est = MuEstimator::new();
        assert_eq!(est.mu_shrink(&z, 1.0), None);
    }

    /// merge：合并两 est bit-exact 等价单 est 顺序累加同一桶（Chan 并行 Welford，mean+方差）。
    #[test]
    fn merge_equals_sequential_accumulation() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let xs = [10.0, 12.0, 8.0, 30.0, 25.0, 5.0, 18.0];
        // 单 est 顺序累加全部。
        let mut single = MuEstimator::new();
        for &x in &xs {
            single.observe(MuObservation { class: z, x_gamma: x });
        }
        // 两 est 各累加一半再 merge。
        let mut a = MuEstimator::new();
        let mut b = MuEstimator::new();
        for &x in &xs[..3] {
            a.observe(MuObservation { class: z, x_gamma: x });
        }
        for &x in &xs[3..] {
            b.observe(MuObservation { class: z, x_gamma: x });
        }
        a.merge(&b);
        assert_eq!(a.count(&z), single.count(&z), "merge 后 n 一致");
        assert!((a.mu(&z).unwrap() - single.mu(&z).unwrap()).abs() < 1e-12, "merge mean bit-exact");
        // 方差（经 std）一致 ⟹ m2 合并正确（LCB 依赖）。
        let (sa, ss) = (a.mu_lcb(&z, 1.645).unwrap(), single.mu_lcb(&z, 1.645).unwrap());
        assert!((sa - ss).abs() < 1e-10, "merge LCB（含方差）bit-exact：{sa} vs {ss}");
        // 空 merge / merge 空：恒等。
        let mut c = single.clone();
        c.merge(&MuEstimator::new());
        assert_eq!(c.mu(&z), single.mu(&z), "merge 空 est 恒等");
    }

    /// shrunk_view：各桶 mean==mu_shrink、n/count 保留（n_L3 池大小不变，保功效卖点可验证）。
    #[test]
    fn shrunk_view_replaces_mean_keeps_n() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root);
        let neighbor =
            MuClass::from_certificate(3, 1, BspBits { buy2: true, ..Default::default() }, 0, PositionState::Root);
        let mut est = MuEstimator::new();
        // 邻类大量低值拉低 pooled，z 高值 ⟹ 收缩可观测。
        for _ in 0..100 {
            est.observe(MuObservation { class: neighbor, x_gamma: 0.0 });
        }
        est.observe_all([
            MuObservation { class: z, x_gamma: 40.0 },
            MuObservation { class: z, x_gamma: 60.0 },
        ]); // z 均值 50，n=2
        let tau_sq = 1.0;
        let view = est.shrunk_view(tau_sq);
        // 各桶 mean 恰为 mu_shrink，n 保留（count 不变 ⟹ n_L3 可比）。
        for (zc, _) in est.iter_mu() {
            assert_eq!(view.mu(&zc), est.mu_shrink(&zc, tau_sq), "桶 mean 应==mu_shrink");
            assert_eq!(view.count(&zc), est.count(&zc), "n 保留 ⟹ n_L3 池可比");
        }
        // z 收缩后 < 原 mean（被 pooled 向 0 拉）但 > 0（n=2 ⟹ w_z>0 未全坍）。
        let shrunk_z = view.mu(&z).unwrap();
        assert!(shrunk_z < 50.0 && shrunk_z > 0.0, "收缩方向正确：0 < {shrunk_z} < 50");
        assert_eq!(view.n_classes(), est.n_classes(), "桶集合不变");
    }

    /// project_to_u 确定性：同 z 恒映同 u（§30 ϕ 是函数，纯确定性变换）。
    #[test]
    fn project_to_u_is_deterministic() {
        let z = MuClass::from_certificate(3, 1, buy_bits(), 1, PositionState::Child);
        assert_eq!(UClass::project_to_u(&z), UClass::project_to_u(&z), "同 z 恒映同 u");
    }

    /// ϕ 折叠真实性：相邻 level + 不压扁的 I_γ 在 z 域是不同类，在 u 域折叠为同一 u（§30 降维）。
    #[test]
    fn project_to_u_folds_distinct_z() {
        // level 2 与 3 相邻（同 bucket=1），I_γ 只 B1 vs B1+B2（都含一类 ⟹ divergence 同 true），
        // 同 δ=+1 同 Root ⟹ 折叠为同一 u。
        let z_a = MuClass::from_certificate(2, 1, buy_bits(), 0, PositionState::Root);
        let z_b = MuClass::from_certificate(
            3,
            1,
            BspBits { buy1: true, buy2: true, ..Default::default() },
            0,
            PositionState::Root,
        );
        assert_ne!(z_a, z_b, "z 域不同类（level + I_γ 不同）");
        assert_eq!(UClass::project_to_u(&z_a), UClass::project_to_u(&z_b), "u 域折叠为同一 u");
    }

    /// 短差/顺势/根 三角色折叠正确（§16 (parent_dir,short_swing,position)→role）。
    #[test]
    fn project_to_u_role_folding() {
        let root = MuClass::from_certificate(2, 1, buy_bits(), 0, PositionState::Root);
        let swing = MuClass::from_certificate(2, -1, buy_bits(), 1, PositionState::Child); // δ=−σ_p
        let trend = MuClass::from_certificate(2, 1, buy_bits(), 1, PositionState::Child); // δ=σ_p
        assert_eq!(UClass::project_to_u(&root).role, VoiceRole::Root);
        assert_eq!(UClass::project_to_u(&swing).role, VoiceRole::ChildSwing);
        assert_eq!(UClass::project_to_u(&trend).role, VoiceRole::ChildTrend);
    }

    /// 降维真实性 |U| < |Z|（§29 核心——ϕ 非单射 ⟹ u 类数严格少于 z 类数）。
    /// 构造多个 z 折叠到少数 u，验证 UEstimator::from_z 后 n_classes(U) < n_classes(Z)。
    #[test]
    fn u_classes_fewer_than_z_classes() {
        let mut est = MuEstimator::new();
        // 4 个不同 z：level{2,3}（同 bucket=1）× I_γ{B1, B1+B2}（都 divergence=true），同 δ/Root
        // ⟹ 全折叠为 1 个 u。
        for level in [2, 3] {
            for bits in [buy_bits(), BspBits { buy1: true, buy2: true, ..Default::default() }] {
                let z = MuClass::from_certificate(level, 1, bits, 0, PositionState::Root);
                est.observe(MuObservation { class: z, x_gamma: 10.0 });
            }
        }
        let u_est = UEstimator::from_z(&est);
        assert_eq!(est.n_classes(), 4, "z 域 4 类");
        assert_eq!(u_est.n_classes(), 1, "u 域折叠为 1 类");
        assert!(u_est.n_classes() < est.n_classes(), "降维真实：|U| < |Z|");
    }

    /// from_z 聚合 bit-exact：u 桶 μ = 落入它的全部 z 的 X_γ 总均值（Chan Welford 等价逐笔）。
    #[test]
    fn u_aggregation_is_bit_exact_pooled_mean() {
        let mut est = MuEstimator::new();
        // 两个 z 折叠到同一 u：各灌不同 X_γ。
        let z_a = MuClass::from_certificate(2, 1, buy_bits(), 0, PositionState::Root);
        let z_b = MuClass::from_certificate(3, 1, buy_bits(), 0, PositionState::Root); // 同 bucket
        est.observe_all([
            MuObservation { class: z_a, x_gamma: 10.0 },
            MuObservation { class: z_a, x_gamma: 20.0 },
            MuObservation { class: z_b, x_gamma: 60.0 },
        ]);
        let u_est = UEstimator::from_z(&est);
        let u = UClass::project_to_u(&z_a);
        assert_eq!(u_est.count(&u), 3, "u 桶聚合 3 笔");
        // (10+20+60)/3 = 30，u 域 n̄=3 > z 域各类 n̄（z_a=2, z_b=1）⟹ 升每类样本。
        assert!((u_est.mu(&u).unwrap() - 30.0).abs() < 1e-12, "u μ = 全部 X_γ 总均值 30");
    }

    /// oos_gated_drop：OOS 不降的轴标可删，OOS 显著降的轴标保留（§11 OOS-value-gated）。
    #[test]
    fn oos_gated_drop_marks_droppable_axes() {
        let baseline = 100.0;
        let result = oos_gated_drop(
            &[DropAxis::LevelBucket, DropAxis::Role, DropAxis::Divergence],
            baseline,
            1.0, // tol
            |axis| match axis {
                DropAxis::LevelBucket => 100.5, // OOS 微升 ⟹ 可删
                DropAxis::Role => 99.5,         // 微降但在 tol 内 ⟹ 可删
                DropAxis::Divergence => 80.0,   // 显著降 ⟹ 保留（背驰维度有 OOS 价值）
            },
        );
        assert_eq!(result[0], (DropAxis::LevelBucket, true), "OOS 升 ⟹ 可删");
        assert_eq!(result[1], (DropAxis::Role, true), "tol 内不降 ⟹ 可删");
        assert_eq!(result[2], (DropAxis::Divergence, false), "OOS 显著降 ⟹ 保留");
    }
}
