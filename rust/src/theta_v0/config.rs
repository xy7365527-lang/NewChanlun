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

/// Θ_voice 参数（reference-theta-v0.md:39-42）。
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceConfig {
    /// 声部树最大层数（L*, L*-1, L*-2）。default 3。[设计选择,默认值]
    pub max_depth: u32,
    /// 深度资金权重 `w=[0.60,0.30,0.10]`。未用部分保留现金不重分配。
    pub depth_weights: Vec<f64>,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        VoiceConfig {
            max_depth: 3,
            depth_weights: vec![0.60, 0.30, 0.10],
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
    pub kappa: f64,
    /// sizing 默认 lot（reference-theta-v0.md:47）。default 1。
    pub default_lot: u32,
    /// χ_t 阈值 θ（alpha2 §13 line 2241，成本/风险门槛）。`None`=χ≡1 全覆盖（默认，frozen Θ v0
    /// bit-exact 不变）；`Some(θ)`=χ=1[μ>θ] 阈值过滤（只交易正边际收益类别）。θ 是 **Θ_risk 参数，
    /// 非缠论可导**（selector.rs 诚实标注）——θ 为**常数**（不从样本 μ 分布选，避免 in-sample
    /// 泄漏，codex Q1 审查确认）。default None（不改 frozen 默认）。
    pub chi_theta: Option<f64>,
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

/// 完整 Θ v0 配置（七组件聚合）。Phase 6 Θ 空间扫描扫的就是这个对象。
///
/// `Default::default()` = reference-theta-v0.md 冻结的 Θ v0 默认值。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThetaConfig {
    pub tick: TickConfig,
    pub parse: ParseConfig,
    pub level: LevelConfig,
    pub macd: MacdConfig,
    pub voice: VoiceConfig,
    pub risk: RiskConfig,
    pub exec: ExecConfig,
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
        assert_eq!(c.exec.entry_delay_bars, 1);
        assert_eq!(c.exec.commission_bps, 1.0);
        assert_eq!(c.exec.slippage_bps, 2.0);
        assert_eq!(c.exec.tax_bps, 0.0);
    }
}
