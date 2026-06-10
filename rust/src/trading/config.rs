//! 配置与变体表 — 有机赋格 v2 消融轴（v2 §8.1）。
//!
//! 与 Python v1 `OrganicConfig` 的轴差异（v2 矛盾修正的配置面）：
//!   - `master_seg_end` 删除（C1：master 出场 = sell1 类型隔离，无配置可重开三触发）
//!   - `sub_anchor` 新增（C3 G1：REV 开腿次级别方向锚定；消融轴 S）
//!   - `tranche` 新增（C4：递归建仓 vs 一次性满 frac_k——V1f/V1r 隔离轴）
//!   - `sell2_trigger` 新增（C6 §5.3 矩阵 Sell2 格的显式表态；消融轴 R2，默认关）
//!   - `rev_gate` 语义升级为点态门（C7；v1 积累集语义无对应配置位——被整体替换）

use crate::buysellpoint::BspKind;

/// 市场语境。futures（INV-3 真实空头）不在枚举里——F1 实装时新增变体，
/// 编译器强制所有 match 点表态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketMode {
    Stock,
}

/// 止损模式。A/B 当前同语义（2% 核心止损），保留区分位（Python parity）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopMode {
    None,
    A,
    B,
}

impl StopMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(StopMode::None),
            "A" => Some(StopMode::A),
            "B" => Some(StopMode::B),
            _ => None,
        }
    }

    pub fn is_on(self) -> bool {
        matches!(self, StopMode::A | StopMode::B)
    }
}

/// REV 关腿消融轴 R（type1 买分量的确认强度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevClose {
    /// confirmed type1 买（T5 原样）。
    Conf,
    /// type1 买放宽为 candidate。
    Cand,
    /// candidate type1 买 ∧ 次级别买点（27课区间套）。
    Nested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sizing {
    Equal,
    Structure,
}

/// REV 开腿锚定强度（C3 消融轴 S）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAnchor {
    /// 无锚定（v1 形态，对照基线用）。
    Off,
    /// G1 默认：dir_row[k−1] == Down（次级别方向行，需 D3 磁带行）。
    Direction,
    /// D 强锚定：次级别反向中枢已形成（消融轴 S）。
    CenterFormed,
}

/// 有机赋格 v2 配置。P5 继承轴默认值 = P5 逐字（V0≡P5 守卫的基线锚定）。
#[derive(Debug, Clone)]
pub struct OrganicConfig {
    // ── P5 继承轴 ──
    pub open_kinds: Vec<BspKind>,
    pub hard_type3: bool,
    pub pre_type3: bool,
    pub center_gate: bool,
    pub theta_amp: f64,
    pub same_center_close: bool,
    pub osc_mode: bool,
    pub osc_buy_sub: bool,
    // ── 有机扩展轴（v2） ──
    pub rev_mode: bool,
    pub sub_anchor: SubAnchor,
    /// false = 一次性 frac_k（V1f）；true = tranche 递归建仓/平仓（V1r，需 D3 磁带行）。
    pub tranche: bool,
    pub rev_gate: bool,
    pub rev_close: RevClose,
    /// C6 §5.3 Sell2 格：默认 no-op；消融轴 R2 显式表态位。
    pub sell2_trigger: bool,
    pub sizing: Sizing,
    pub earning_reaction: bool,
    pub market_mode: MarketMode,
    /// T4b-(c) run_anchor 归属容差（设计未给值的实现常数，显式化为配置而非魔数）。
    pub t4b_anchor_margin: i64,
}

impl Default for OrganicConfig {
    fn default() -> Self {
        OrganicConfig {
            open_kinds: vec![BspKind::Type1, BspKind::Type2],
            hard_type3: true,
            pre_type3: true,
            center_gate: true,
            theta_amp: 0.01,
            same_center_close: true,
            osc_mode: true,
            osc_buy_sub: false,
            rev_mode: false,
            sub_anchor: SubAnchor::Off,
            tranche: false,
            rev_gate: false,
            rev_close: RevClose::Conf,
            sell2_trigger: false,
            sizing: Sizing::Equal,
            earning_reaction: false,
            market_mode: MarketMode::Stock,
            t4b_anchor_margin: 0,
        }
    }
}

/// v2 变体表（§8.1 消融轴正交分解）。`O0` 为 `V0` 别名（对账脚本兼容）。
pub fn variant(name: &str) -> Option<OrganicConfig> {
    let base = OrganicConfig::default();
    match name {
        // V0：≡P5 逐位守卫（trades+trace+counters 三面）
        "V0" | "O0" => Some(base),
        // V1：rev 裸开（无 G1/tranche/门）——v1 O1v 的 v2 语义类比
        // （差异仅 C2 osc 不截断 + C5 单 tranche 区间套读出退化为 home 层），
        // 与在册 O1v 基线（Δ=−63.7/−41.4/−186.2pp）直接可比，隔离 C2 的因果。
        "V1" => Some(OrganicConfig { rev_mode: true, ..base }),
        // V1f：G1 锚定的独立因果（一次性 frac_k = O1v+G1）
        "V1f" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            ..base
        }),
        // V1r：递归建仓/平仓的独立因果（V1r−V1f）
        "V1r" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            ..base
        }),
        // V2：门 v2 因果（V2−V1r）+ 门开率定义域
        "V2" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            ..base
        }),
        // V3：规模轴（含 REV）
        "V3" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            sizing: Sizing::Structure,
            ..base
        }),
        // V3′：规模轴独立重验（无 REV）——v1 判据3 欠账
        "V3p" => Some(OrganicConfig {
            sizing: Sizing::Structure,
            ..base
        }),
        // V4：earning 反作用（前置：触发率>0）
        "V4" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            sizing: Sizing::Structure,
            earning_reaction: true,
            ..base
        }),
        // 消融 S：强锚定（次级别反向中枢已形成）
        "VS" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::CenterFormed,
            tranche: true,
            ..base
        }),
        // 消融 R：REV 关腿 type1 买确认强度（v1 O2c/O2n 轴的 v2 形态，exploratory）
        "VRc" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_close: RevClose::Cand,
            ..base
        }),
        "VRn" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_close: RevClose::Nested,
            ..base
        }),
        // 消融 R2：Sell2 入段终结触发集（§5.3 矩阵 Sell2 格的表态轴，exploratory）
        "VR2" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            sell2_trigger: true,
            ..base
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_is_p5() {
        let cfg = variant("V0").unwrap();
        assert!(!cfg.rev_mode);
        assert_eq!(cfg.open_kinds, vec![BspKind::Type1, BspKind::Type2]);
        assert!(cfg.hard_type3 && cfg.pre_type3 && cfg.center_gate);
        assert_eq!(cfg.theta_amp, 0.01);
        assert!(cfg.osc_mode && !cfg.osc_buy_sub);
        assert_eq!(cfg.sizing, Sizing::Equal);
    }

    #[test]
    fn o0_alias() {
        assert!(variant("O0").is_some());
        assert!(variant("nonexistent").is_none());
    }
}
