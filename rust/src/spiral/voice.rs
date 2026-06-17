//! Voice 森林骨架（携带 SpiralState）。
//!
//! 设计来源：`docs/spiral_engine_v2_architecture.md` §4.3。语义 bit-exact 复用
//! unn 引擎 `VoiceLedger`（R5/R6），但状态用 `SpiralState`（携带 φ/r/ε 三坐标）。
//!
//! ## Step 0 范围（诚实声明，no-patch-mentality）
//! 本文件是**空骨架**——数据结构 + 最小构造/查询。操作逻辑（nav/close_voice/
//! settle/spawn/守恒，§5 操作层 + §6 P-close）在 **Step 1（会计层移植）** 填入。
//! 当前**不声明**任何操作能力（声明=能力，090号）。
//!
//! ## 关键不变量（R5，Step 1 由 `prove_n8_conservation` 守）
//! `Σ(active voices.units) = n_base`（记账两侧一致，**非建仓常数恒仓**——
//! N_base 双向重定基：earning +Δ / 亏损 −δ，架构 §12.4-5）。
//! `capital` 非逐市 MtM（空头持现金，物理单真值）。
//!
//! ## 孤儿不可能定理（T42，Step 1 由 `prove_n1_forest` 守）
//! 后序遍历保证 ≤1 active root + 无孤儿（active 非根 voice 的父必非 Closed）。

use super::state::SpiralState;
use crate::trading::types::Polarity;

/// 声部生命周期状态（§4.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceStatus {
    /// 活跃持仓。
    Active,
    /// 子空头被否定 candidate 后待回补（D 回补返父前的中间态）。
    PendingRecovery,
    /// 已平仓（后序 close 后；孤儿不可能定理依赖此终态）。
    Closed,
}

/// 一个声部 = 螺旋一圈（赋格一个 voice）。携带其 `SpiralState`（φ/r=级别/ε=方向）。
///
/// 字段语义（§4.3，Step 1 填操作）：
/// - `units`：在手单位（Σ活跃 = `n_base`；T34 σ-不变 Casimir）。
/// - `basis`：开仓均价（降成本下降，earning 可穿 0 变负）。
/// - `capital`：空头冻结现金（非逐市，物理单真值）/ 多头恒 0。
/// - `parent`/`children`：森林拓扑（root 可多 child，孤儿不可能 T42）。
/// - `acted_bar`：per-voice acted（去全局互斥，N2）。
#[derive(Debug, Clone)]
pub struct SpiralVoice {
    /// 该 voice 的螺旋坐标 `(φ, r=级别, ε=方向)`。
    pub state: SpiralState,
    pub units: f64,
    pub basis: f64,
    /// 成本池（§7 earning 判据：回补利润递减，≤0 后纯利润）。Step 1 会计载体。
    pub cost_pool: f64,
    pub capital: f64,
    pub entry_bar: i64,
    pub status: VoiceStatus,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub acted_bar: i64,
    pub realized_pnl: f64,
}

impl SpiralVoice {
    /// 构造一个新声部。`parent=None` ⟹ 根 voice（σ 塔起点，F 建仓，§5.1）。
    ///
    /// Step 0 仅提供数据构造；basis/capital 的会计语义（多头 capital=0 /
    /// 空头冻结现金）在 Step 1 由 `accounting.rs` 强制。
    pub fn new(state: SpiralState, units: f64, basis: f64, capital: f64, entry_bar: i64, parent: Option<usize>) -> Self {
        SpiralVoice {
            state,
            units,
            basis,
            // 初始成本池 = 在手金额（units×basis）；earning 阶段递减穿 0（§7）。
            cost_pool: units * basis,
            capital,
            entry_bar,
            status: VoiceStatus::Active,
            parent,
            children: Vec::new(),
            acted_bar: -1,
            realized_pnl: 0.0,
        }
    }

    /// 是否根 voice（无父）。
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// 是否活跃（非 Closed）。
    pub fn is_active(&self) -> bool {
        !matches!(self.status, VoiceStatus::Closed)
    }

    /// 径向级别 r（= 群坐标 `state.r`，单一真相源——`ladder` 不独立存储以防漂移）。
    pub fn ladder(&self) -> usize {
        self.state.r as usize
    }

    /// 持仓极性（= 手性 `state.eps` 的投影：+1↦Long / −1↦Short）。
    /// 方向是群坐标 ε 的读数，翻转由 `GroupAction::ChiralSeam` 驱动（§5.2）。
    pub fn dir(&self) -> Polarity {
        if self.state.eps >= 0 {
            Polarity::Long
        } else {
            Polarity::Short
        }
    }

    /// 可作为 D/E 动作主体（活跃 ∧ 有在手单位 ∧ 本 bar 未动作；N2 per-voice 互斥）。
    pub fn can_act(&self, bar: i64) -> bool {
        self.is_active() && self.units > 0.0 && self.acted_bar != bar
    }

    /// units 变动后刷新 husk 状态（不改 Closed）——spawn/翻转后父 units→0
    /// 转 PendingRecovery（等子回补回满）。
    pub fn refresh_status(&mut self) {
        if self.status == VoiceStatus::Closed {
            return;
        }
        self.status = if self.units > 1e-12 {
            VoiceStatus::Active
        } else {
            VoiceStatus::PendingRecovery
        };
    }
}

/// Voice 森林：后序遍历保证孤儿不可能定理（T42）。
///
/// Step 0 骨架：容器 + 最小查询。操作（push/spawn/close_voice/nav/settle）在
/// Step 1 填入（§5 操作层 + §6 P-close）。
#[derive(Debug, Clone, Default)]
pub struct SpiralForest {
    pub voices: Vec<SpiralVoice>,
    /// 股数守恒基准（Σ active units；双向重定基，非建仓常数）。
    pub n_base: f64,
}

impl SpiralForest {
    /// 空森林（创世前态）。
    pub fn new() -> Self {
        SpiralForest::default()
    }

    /// 活跃根 voice 数（N1 不变量观测：清仓/EOD 前应 ≤1，Step 1 由
    /// `prove_n1_forest` panic 守卫）。
    pub fn active_root_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| v.is_active() && v.is_root())
            .count()
    }

    /// 活跃声部 units 之和（守恒守卫的左项；Step 1 与 `n_base` 比对）。
    pub fn active_units(&self) -> f64 {
        self.voices
            .iter()
            .filter(|v| v.is_active())
            .map(|v| v.units)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_forest_has_no_root() {
        let f = SpiralForest::new();
        assert_eq!(f.active_root_count(), 0);
        assert_eq!(f.active_units(), 0.0);
    }

    #[test]
    fn root_voice_construction() {
        // F 建仓：根 voice @ (0, source, +1)，多头 capital=0。
        let mut f = SpiralForest::new();
        let root = SpiralVoice::new(SpiralState::root(3), 100.0, 50.0, 0.0, 0, None);
        assert!(root.is_root());
        assert!(root.is_active());
        f.voices.push(root);
        f.n_base = 100.0;
        assert_eq!(f.active_root_count(), 1);
        assert_eq!(f.active_units(), 100.0);
    }

    #[test]
    fn closed_voice_excluded_from_active() {
        let mut f = SpiralForest::new();
        let mut root = SpiralVoice::new(SpiralState::root(3), 100.0, 50.0, 0.0, 0, None);
        root.status = VoiceStatus::Closed;
        f.voices.push(root);
        // Closed 根不计入 active（孤儿不可能定理依赖此终态语义）。
        assert_eq!(f.active_root_count(), 0);
        assert_eq!(f.active_units(), 0.0);
    }
}
