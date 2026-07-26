//! Θ v0 参数显式化（reference-theta-v0.md 全部 `[设计选择]`/`[L3经验待标定]` 字段）。
//!
//! ## 为什么所有参数都进 config（no-patch-mentality + formalization-validity-domain）
//!
//! 编排者硬指令：`[设计选择]`/`[L3经验待标定]` 参数作**显式 config**（不硬编码，便于
//! Phase 6 Θ 空间扫描）。把这些值写死在算法里 = 声明膨胀（把"外部设计参数"伪装成
//! "缠论真理"）。config 把 Θ 的**外部参数自由度**显式化，使 Θ 成为可扫描的对象。
//!
//! ## 三类标注（reference-theta-v0.md:57）
//!
//! - `[缠论可导]`：缠论原文有依据 → **不进 config**（是结构常量，不是自由参数），
//!   直接在算法中实现（如分型严格不等号、新笔 ≥3 根间隔）。
//! - `[设计选择,默认值]`：extra-缠论必须固定的外部参数 → **进 config**，default 给
//!   reference-theta-v0.md 指定值。
//! - `[L3经验待标定]`：数值需 L2/L3 标定 → **进 config**，default 给 v0 占位值，
//!   字段注释标 `L3`。

/// 价格量化：整数 tick。无 `tick_size` 默认 `1e-8`（reference-theta-v0.md:15）。
///
/// [设计选择,默认值]。bit-exact 要求所有价格运算在整数 tick 域进行——浮点价格在
/// 引擎边界一次性量化为 `i64` tick，内部不再触碰 `f64`（消除浮点约简顺序歧义）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickConfig {
    /// tick 大小（价格最小变动单位）。default `1e-8`。
    pub tick_size: f64,
}

impl Default for TickConfig {
    fn default() -> Self {
        TickConfig { tick_size: 1e-8 }
    }
}

/// Θ_parse 参数（reference-theta-v0.md:18-25）。
///
/// 大部分 parse 规则是 `[缠论可导]`（不进 config）；本结构只承载 parse 段的
/// `[设计选择,默认值]` 自由参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParseConfig {
    /// 新笔两极值 K 间排除两端至少 N 根（reference-theta-v0.md:21）。
    ///
    /// 缠论 77/81 课新笔定义 N=3 是 `[缠论可导]`，但作为 config 字段承载以便审计/扫描
    /// （取值变更 = 改 Θ，须 change request）。default 3。
    pub new_stroke_min_gap: u32,

    /// 线段最小笔数（第77课:64 起三笔重叠 ⟹ 段 ≥ 3 笔）。
    ///
    /// 对齐 Python `a_segment_v1.segments_from_strokes_v1` 的 `min_seg_strokes`（default 3）。
    /// `[缠论可导,77课]` 但作为 config 承载以便审计（取值变更 = 改 Θ）。default 3。
    pub seg_min_strokes: u32,

    /// 第二特征序列扫描窗口（0 = 无限全扫，bit-exact 优先）。
    ///
    /// ★`[设计选择,默认值]`（second_kind.rs 模块头声明）：Python 的 `MAX_SECOND_SEQ_SCAN=50`
    /// 是 O(n²) 性能妥协，**非缠论可导**。本字段 default 0（无限）——L2 实测（OKLO 2000 笔）
    /// 窗口扩到 ∞ 输出不变，证 50 在目标数据域非约束性（单数据集 L2，不裸剥常数）。
    pub second_seq_scan_window: u32,
}

impl Default for ParseConfig {
    fn default() -> Self {
        ParseConfig {
            new_stroke_min_gap: 3,
            seg_min_strokes: 3,
            second_seq_scan_window: 0,
        }
    }
}

/// Θ_level 参数（reference-theta-v0.md:27-30）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelConfig {
    /// 最高研究级别 Lmax（reference-theta-v0.md:30）。[设计选择,默认值] default 6。
    pub l_max: u32,
    /// 某层产生 ≥N 完成部件才不自然终止（reference-theta-v0.md:30）。default 3。
    pub min_parts_per_level: u32,
}

impl Default for LevelConfig {
    fn default() -> Self {
        LevelConfig {
            l_max: 6,
            min_parts_per_level: 3,
        }
    }
}

/// Θ_signal 背驰 MACD 参数（reference-theta-v0.md:37）。
///
/// 结构前提优先；MACD(12,26,9) 是 v0 辅助度量。EMA/面积规则标 `[L3经验待标定]`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdConfig {
    /// 快线 EMA 周期。default 12。[MACD辅助缠论可导]
    pub fast: u32,
    /// 慢线 EMA 周期。default 26。
    pub slow: u32,
    /// 信号线 EMA 周期。default 9。
    pub signal: u32,
}

impl Default for MacdConfig {
    fn default() -> Self {
        MacdConfig {
            fast: 12,
            slow: 26,
            signal: 9,
        }
    }
}

/// q_Θ v1 σ_higher 分级符号权重 w_dir 预注册套（prereg-rev4 §4.3，三套并行 OOS 不能事后选——
/// 关于背驰.pdf §9.2「不能先看结果再选」）。w_dir 函数形式见 coverage.rs [`super::strategy::coverage::dir_weight`]。
///
/// ★认识论等级（231号）：
/// - **L0**（结构）：三套结构（follow/neutral/adversary）+ ReverseOpen 豁免 + 根级豁免是 formal-chain 推论
///   （买卖点.pdf §7.5 `s_g=s_α` 定理 1 + 完整的策略.pdf page4 禁一刀切）。
/// - **L2**（数值）：`η_adv[ℓ]`/`η_same[ℓ]` 具体数值须 g3 跑数前冻结（135号），本枚举只冻结构。
#[derive(Debug, Clone, PartialEq)]
pub enum ThetaDirPreset {
    /// Θ_dir_neutral：所有 (ℓ, sign) 槽 = 1.0 ⟹ `w_dir ≡ 1.0`（退化为 v0，**bit-exact == v0 对照基线**）。
    /// default（保 frozen Θ v0 bit-exact）。亦作 g2 实装自检基线：若 neutral 套 OOS ≠ v0 = 实装 bug。
    Neutral,
    /// Θ_dir_follow（顺势保权假设）：顺上级（sign=+1）保权 = 1.0，逆上级（sign=−1）降权 `η_adv[ℓ] < 1.0`。
    /// `eta_adv`：per-level 逆上级降权系数（长度 ≥ max_depth，越界层视 1.0）。
    Follow { eta_adv: Vec<f64> },
    /// Θ_dir_adversary（逆势保权假设，§6 page4「L1 主要超 beta 来源」）：逆上级（sign=−1）保权 = 1.0，
    /// 顺上级（sign=+1）降权 `η_same[ℓ] < 1.0`。`eta_same`：per-level 顺上级降权系数（同上越界规则）。
    Adversary { eta_same: Vec<f64> },
}

/// Θ_voice 参数（reference-theta-v0.md:39-42）。
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceConfig {
    /// 声部树最大层数（L*, L*-1, L*-2）。default 3。[设计选择,默认值]
    pub max_depth: u32,
    /// 深度资金权重 `w=[0.60,0.30,0.10]`。未用部分保留现金不重分配。
    pub depth_weights: Vec<f64>,
    /// f3 反事实开关：剔除 ReverseOpen（首开反向/多空对冲）子声部腿对净头寸的贡献（多重赋格增量价值测量）。
    /// default `false`=全赋格生产口径（bit-exact 不变）。`true` 仅用于 policy_backtest 反事实对照。
    /// 原 `disable_shortdiff`，#281 更名（#283 实装）。
    pub disable_reverse_open: bool,
    /// q_Θ v1 σ_higher 分级符号权重 w_dir 预注册套（prereg-rev4 §4.3）。default `Neutral`
    /// （w_dir≡1.0，frozen Θ v0 bit-exact 不变）。选 Follow/Adversary 启用 σ_higher 分级 sizing。
    pub theta_dir: ThetaDirPreset,
    /// w_grade：G 轴（grade_rel）sizing 权重（prereg-wg 推荐 (c) 因子化，GPT §九 `w_{ℓ,σ_higher,role,G,...}`）。
    /// `[SameLevel, SubLevel]`。default `[1.0, 1.0]` identity ⟹ G 轴 sizing 无差异 ⟹ frozen Θ v0 bit-exact 不变。
    /// 非 identity ⟹ [`super::strategy::coverage::leg_target`] w = depth_weight × dir_weight × w_grade[grade]
    /// （G 进 sizing 不进 μ 桶键，696 同构：轴进 A 层 ≠ 进 B 层）。与 `theta_dir`（V 轴 σ_higher）正交——
    /// G 轴独立消费，不绑 Follow/Adversary preset。C4 实质担忧的形式化：同级别反父 vs 次级别反父在 sizing 区分。
    pub w_grade: [f64; 2],
}

impl Default for VoiceConfig {
    fn default() -> Self {
        VoiceConfig {
            max_depth: 3,
            depth_weights: vec![0.60, 0.30, 0.10],
            disable_reverse_open: false,
            theta_dir: ThetaDirPreset::Neutral,
            w_grade: [1.0, 1.0],
        }
    }
}

/// Θ_risk 参数（reference-theta-v0.md:44-47）。全部 [设计选择,默认值;L3经验待标定]。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskConfig {
    /// 单声部风险 `ρ=0.005 NAV`。L3。
    pub rho: f64,
    /// 父子仓位比 `β=0.5`。L3。
    pub beta: f64,
    /// 总名义上限 `γ=1.0 NAV`。L3。
    pub gamma: f64,
    /// 成本倍数 `κ=2.0`。L3。
    ///
    /// ★同名 κ 物理隔离（A10 附则A，镜像注释，另一侧见 `ThetaConfig.risk_policy` 文档）：
    /// 本字段是 **sizing 成本倍数** κ（sizing 分母的成本项）；`risk_policy` 承载的
    /// `RiskPolicy` barrier κ 是 **barrier 缓冲系数**（η⋆=L^wc+κQ 风险政策门）——同名不同义，
    /// 互不读写（ledger.rs:249-250 注释在案）。
    pub kappa: f64,
    /// sizing 默认 lot（reference-theta-v0.md:47）。default 1。
    pub default_lot: u32,
    /// χ_t 阈值 θ（alpha2 §13 line 2241，成本/风险门槛）。`None`=χ≡1 全覆盖（默认，frozen Θ v0
    /// bit-exact 不变）；`Some(θ)`=χ=1[μ>θ] 阈值过滤（只交易正边际收益类别）。θ 是 **Θ_risk 参数,
    /// 非缠论可导**（selector.rs 诚实标注）——θ 为**常数**（不从样本 μ 分布选，避免 in-sample
    /// 泄漏，codex Q1 审查确认）。default None（不改 frozen 默认）。
    pub chi_theta: Option<f64>,
    /// χ_t 准入的 LCB 置信分位 z_alpha（严格alpha.pdf p25 §12）：准入用 LCB(μ)=mean−z_alpha·std/√n
    /// 而非裸 μ，防高维 z 过拟合。单边正态分位（如 1.645=95%，2.326=99%）。**default 0.0**（LCB=mean
    /// ⟹ **n≥2** 类退化回裸 μ 门；**n=1 类例外**——mu_lcb 返 None 走 treat_empty_as_pass，**不**退化
    /// （n=1 无方差=无 LCB 证据，拒绝是 p25 正确语义，非回归）。frozen 默认 chi_theta=None 不走此路。
    /// >0 启用置信下界收缩。Θ_risk 参数（非缠论可导）。
    pub chi_z_alpha: f64,
    /// K_Θ 毛头寸约束激活开关（G7，codex #122 终裁 + codex decide 5b46）。`false`（default）⟹
    /// 不激活（frozen Θ v0 bit-exact——净持仓约束照旧，毛敞口不设上限）；`true` ⟹ legs 折叠成净
    /// 持仓**之前**施加毛敞口上限 `Σ|s_e| ≤ γ·U_ℓ`（strict §11「毛+净必须同时约束」，缩放语义 =
    /// 逐根子树 KKT 投影，见 coverage.rs `apply_gross_cap`）。毛 cap **复用** `gamma`（与净 cap
    /// 共用同一 Θ_risk 参数，#122 裁定暂不拆 gross_gamma/net_gamma）。
    pub enforce_gross_cap: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            rho: 0.005,
            beta: 0.5,
            gamma: 1.0,
            kappa: 2.0,
            default_lot: 1,
            chi_theta: None,
            chi_z_alpha: 0.0,
            enforce_gross_cap: false,
        }
    }
}

/// Θ_exec 参数（reference-theta-v0.md:49-54）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecConfig {
    /// 信号确认后延迟成交的基础 K 根数。default 1（reference-theta-v0.md:50）。
    pub entry_delay_bars: u32,
    /// commission（bp/side）。default 1。[设计选择;L3经验待标定]
    pub commission_bps: f64,
    /// slippage（bp/side）。default 2。L3。
    pub slippage_bps: f64,
    /// tax（bp/side）。default 0。L3。
    pub tax_bps: f64,
}

impl Default for ExecConfig {
    fn default() -> Self {
        ExecConfig {
            entry_delay_bars: 1,
            commission_bps: 1.0,
            slippage_bps: 2.0,
            tax_bps: 0.0,
        }
    }
}

/// 多空（root 方向 δ）键——sizing profile 的 (level, side) 索引的 side 分量。
///
/// PDF《全互斥定义策略》§3：ρ_{ℓ,+} ≠ ρ_{ℓ,−}（多空**不强行镜像**——空头有借券费/保证金/
/// 尾部风险不同）。`Flat` 无仓位需 sizing，不入表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideKey {
    Long,
    Short,
}

/// 单条 (level, side) sizing 参数 override（PDF §3：ρ/Γ 是按级别+多空取值的状态函数）。
///
/// `gap_buffer`（PDF §2 GapBuffer，美元）：隔夜跳空/尾部滑点的额外止损距离，加进风险归一化
/// 分母 D。default 0（退化为无 gap）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizingEntry {
    pub level: u32,
    pub side: SideKey,
    pub rho: f64,
    pub gamma: f64,
    pub gap_buffer: f64,
}

/// (level, side) → (ρ, Γ, GapBuffer) override 表（PDF §3 ρ_{ℓ,δ,r}/Γ_{ℓ,δ,r} 状态函数）。
///
/// **bit-exact 边界**：`entries` 为空（default）⟹ 所有 sizing 退化为 `RiskConfig` 的标量
/// ρ/Γ + gap=0 ⟹ 与改动前逐位相同（frozen Θ v0 不破）。非空时按 (level, side) 精确匹配；
/// 无匹配条目仍退化为标量（局部 override，未覆盖的 (level, side) 走默认）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SizingProfile {
    pub entries: Vec<SizingEntry>,
}

impl SizingProfile {
    /// 解析当前决策的 (level, side) → (ρ, Γ, GapBuffer)。
    ///
    /// 命中 override 条目 ⟹ 返回其 (rho, gamma, gap_buffer)；否则退化为 `risk` 标量 + gap=0
    /// （bit-exact 默认路径）。线性扫描（表小，level×{Long,Short} ≤ 2·Lmax 条；不引哈希避免
    /// `Copy` 破坏）。首个匹配生效（调用方保证 (level, side) 唯一）。
    pub fn resolve(&self, level: u32, side: SideKey, risk: &RiskConfig) -> (f64, f64, f64) {
        for e in &self.entries {
            if e.level == level && e.side == side {
                return (e.rho, e.gamma, e.gap_buffer);
            }
        }
        (risk.rho, risk.gamma, 0.0) // 无 override ⟹ 标量退化（bit-exact）
    }
}

/// 完整 Θ v0 配置（八组件聚合）。Phase 6 Θ 空间扫描扫的就是这个对象。
///
/// `Default::default()` = reference-theta-v0.md 冻结的 Θ v0 默认值（`sizing_profile` 空 ⟹
/// sizing 退化为 `risk` 标量，bit-exact 不变）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThetaConfig {
    pub tick: TickConfig,
    pub parse: ParseConfig,
    pub level: LevelConfig,
    pub macd: MacdConfig,
    pub voice: VoiceConfig,
    pub risk: RiskConfig,
    pub exec: ExecConfig,
    /// 狭义短差（中枢震荡高抛低吸，#274/#292）落点门控。默认关闭：触发证据仍可见（协议轨），
    /// 短差动作/订单轨 frozen bit-exact。★注释订正（#292 评审尾巴，原「配对子腿」措辞是 S6
    /// 开空腿对冲形态的残留——该形态已被 ADR 0001 修正案一 修1 废止删除，本字段现口径 =
    /// 减仓回补（修1 唯一定义），非配对子腿）。
    pub center_oscillation: super::strategy::oscillation::CenterOscillationConfig,
    /// ρ_{ℓ,δ,r}/Γ_{ℓ,δ,r}/GapBuffer 状态函数 override（PDF §3）。空 ⟹ 全用 `risk` 标量。
    pub sizing_profile: SizingProfile,
    /// 真保证金模型（D2 task #113，margin-model-design v2）。`None` ⟹ MM=0 退化口径（bit-exact 现状,
    /// M1/M2/M3 不可达）；`Some` ⟹ 真实分级 MM/liq_flag/M2-M3 接线（改订单流 ⟹ MM=0 口径 alpha 冻结失效）。
    pub margin: Option<super::strategy::risk::MarginModel>,
    /// M6 成本模型（TARGET_STRATEGY_MAXFULL.md M6 / 路线.pdf p16 第十一关剩余三项：Funding/Borrow/
    /// LiquidationLoss）。`None` ⟹ 三项成本恒 0（bit-exact 现状——Commission/Slippage 仍由 `exec`
    /// fee_rate 承担，不受影响）；`Some` ⟹ 逐 bar 计提资金费/借贷 + 强平罚金进 PnL、进 R 分解、
    /// 守恒断言。**有效域（231号）**：v0 参数化常费率，真实 funding/借贷历史是外部数据缺口（L2），
    /// 机制真实装、费率待外部标定（A10 waiver 豁免外部数据源，不豁免机制）。
    pub cost_model: Option<super::strategy::risk::CostModel>,
    /// κ 风险政策（A10 附则A 裁定接口冻结）：`None` ⟹ `RiskPolicy::baseline()` κ=0，与现路径
    /// **逐字节相同（bit-exact）**；`Some` ⟹ π loop `tw_policy` 取之（`stage_progression` 的
    /// η⋆=L^wc+κQ barrier 门）。**优先序写死**：env `KAPPA_BARRIER_*`（L2 敏感性诊断覆写）>
    /// 本字段 > baseline——单源纪律防双源静默漂移；env 非法值 panic 语义不变。
    ///
    /// 取值**永走** `RiskPolicy` 三构造闸（`baseline`/`try_new`/`try_new_ratio`，κ≥0 类型不变量
    /// 对齐 Lean `kappa_nonneg`）——**禁裸 i64/f64 κ 字段出现在任何 config**（类型边界闭合）。
    /// κ=0 基线冻结；正 κ 生产取值是编排者选择类（本裁定不裁；任何 κ>0 报告强制标注
    /// 「κ＝operator 声明式风险政策（不可识别性定理2），非价格导出、非回测择优」）。
    ///
    /// ★同名 κ 物理隔离（镜像注释，另一侧见 ledger.rs RiskPolicy 文档）：本字段承载 **barrier
    /// 缓冲系数** κ（风险政策，η⋆ 门）；`RiskConfig.kappa`（本文件 `:174`，默认 2.0）是 **sizing
    /// 成本倍数** κ——同名不同义。字段命名 `risk_policy` 不带 kappa 字样即防混淆防线。
    pub risk_policy: Option<super::strategy::ledger::RiskPolicy>,
    /// 趋势背驰 D 判定口径（A2 #163 三口径 + A3 #164 Θ_LEX，关于背驰.pdf §9.2 三套预注册 Θ）。默认
    /// `MacdArea`（现行冻结判据，bit-exact 不变）——判定口径变更改变一类信号集合（⟹ ledger ⟹
    /// 残差样本），属预注册敏感，显式配置才切换。四口径：MacdArea/ThetaDom(Θ_DOM)/Conjunction/
    /// ThetaLex(Θ_LEX，weak_theta 词典序 DIF▷面积)。
    pub divergence_gauge: super::classifier::divergence::DivergenceGauge,
    /// C2 D1/D2/D5 消费 seam。默认关闭，故现有分类、信号、订单与缓存路径逐位不变。
    pub c2_level_view: super::classifier::level_view::C2LevelViewConfig,
    /// #110 投影层机制位（T3 (#172) 并门后 = **派生位**：独立配置面退役——层载由链路径
    /// 是否启用单一驱动，生产唯一写入点 = π 入口 `admission::chain_driven_level_projection`；
    /// #168 裁定 3）。默认关闭（= 链死）⟹ stamping 不构造 `LevelProjectionLayer`
    /// （`LevelState.level_projection = None`，零开销，bit-exact 不变，#110 纪律不死）。
    pub level_projection: super::classifier::projection::LevelProjectionConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 骨架契约测试：Θ v0 默认值 bit-exact 等于 reference-theta-v0.md 冻结值。
    /// 此测试是回归锚——任何 default 漂移 = 改 Θ（须 change request）。
    #[test]
    fn theta_v0_defaults_match_frozen_spec() {
        let c = ThetaConfig::default();
        assert_eq!(c.tick.tick_size, 1e-8);
        assert_eq!(c.parse.new_stroke_min_gap, 3);
        assert_eq!(c.level.l_max, 6);
        assert_eq!(c.level.min_parts_per_level, 3);
        assert_eq!((c.macd.fast, c.macd.slow, c.macd.signal), (12, 26, 9));
        assert_eq!(c.voice.max_depth, 3);
        assert_eq!(c.voice.depth_weights, vec![0.60, 0.30, 0.10]);
        assert_eq!(c.risk.rho, 0.005);
        assert_eq!(c.risk.beta, 0.5);
        assert_eq!(c.risk.gamma, 1.0);
        assert_eq!(c.risk.kappa, 2.0);
        assert_eq!(c.risk.default_lot, 1);
        assert_eq!(c.risk.chi_theta, None); // frozen：默认 χ≡1 全覆盖（无阈值过滤，task #41）
        assert!(!c.risk.enforce_gross_cap); // frozen：毛头寸约束默认不激活（G7 约束未配置=不激活）
        assert_eq!(c.exec.entry_delay_bars, 1);
        assert_eq!(c.exec.commission_bps, 1.0);
        assert_eq!(c.exec.slippage_bps, 2.0);
        assert!(!c.center_oscillation.enabled);
        assert!(!c.c2_level_view.enabled, "#73-#75 新 seam 默认必须关闭");
        // T3 (#172 并门，#168 裁定 3）：默认 = 链死 ⟹ 层不载（零开销 bit-exact 锁，#110 纪律）；
        // 「链启用 ⟹ 层必载」形态 = 派生构造子断言（生产唯一写入点 = π 入口派生，
        // admission::chain_driven_level_projection；classifier stamping 仍读本机制位）。
        assert!(!c.level_projection.enabled, "#172 并门：链死（默认）⟹ 投影层不载（零开销 bit-exact 锁）");
        assert!(
            super::super::classifier::projection::LevelProjectionConfig::for_chain(true).enabled,
            "#172 并门：链启用 ⟹ 投影层必载（派生构造子形态锁）"
        );
        assert!(
            !super::super::classifier::projection::LevelProjectionConfig::for_chain(false).enabled,
            "#172 并门：链死 ⟹ 投影层不载"
        );
        assert_eq!(c.exec.tax_bps, 0.0);
        // frozen：sizing_profile 空 ⟹ 所有 sizing 退化为 risk 标量 + gap=0（bit-exact 不变）。
        assert!(c.sizing_profile.entries.is_empty());
        // frozen：w_grade=[1.0,1.0] identity ⟹ G 轴 sizing 无差异（prereg-wg (c) 因子化，bit-exact）。
        assert_eq!(c.voice.w_grade, [1.0, 1.0]);
        // frozen（A10 附则A）：risk_policy=None ⟹ κ=0 baseline，π loop 逐字节不变（bit-exact）。
        // 同名 κ 隔离：risk.kappa=2.0（sizing 成本倍数）与 risk_policy（barrier κ）同名不同义。
        assert_eq!(c.risk_policy, None, "risk_policy 默认 None ⟹ baseline κ=0（bit-exact 锁）");
    }

    /// SizingProfile.resolve：空表 ⟹ 退化为 risk 标量 + gap=0（bit-exact 默认路径）。
    #[test]
    fn sizing_profile_empty_falls_back_to_scalar() {
        let risk = RiskConfig::default(); // rho=0.005, gamma=1.0
        let prof = SizingProfile::default();
        assert_eq!(prof.resolve(0, SideKey::Long, &risk), (0.005, 1.0, 0.0));
        assert_eq!(prof.resolve(3, SideKey::Short, &risk), (0.005, 1.0, 0.0));
    }

    /// SizingProfile.resolve（PDF §3）：(level, side) 命中 ⟹ override；多空不镜像；未命中退化。
    #[test]
    fn sizing_profile_per_level_side_override() {
        let risk = RiskConfig::default();
        let prof = SizingProfile {
            entries: vec![
                SizingEntry { level: 2, side: SideKey::Long, rho: 0.008, gamma: 1.2, gap_buffer: 5.0 },
                // 同 level 空头不镜像：更保守的 rho。
                SizingEntry { level: 2, side: SideKey::Short, rho: 0.003, gamma: 0.8, gap_buffer: 12.0 },
            ],
        };
        assert_eq!(prof.resolve(2, SideKey::Long, &risk), (0.008, 1.2, 5.0));
        assert_eq!(prof.resolve(2, SideKey::Short, &risk), (0.003, 0.8, 12.0));
        // 多空不镜像（同 level 取值不同）。
        assert_ne!(
            prof.resolve(2, SideKey::Long, &risk),
            prof.resolve(2, SideKey::Short, &risk)
        );
        // 未覆盖 level ⟹ 标量退化。
        assert_eq!(prof.resolve(5, SideKey::Long, &risk), (0.005, 1.0, 0.0));
    }
}
