//! Chong（重）持久操作单位接口——D2逐bar贯通的操作级别旁路。
//!
//! ## 教义锚点
//!
//! **重 = ⟨标的 S，操作级别 ℓ，专属筹码 Q⟩**（ADR 0010）：
//!
//! > `040-第40课.md:20`【正文】「可以变成 N 重层次的操作，**每一重都对应着一定的资金与筹码**」
//!
//! ## 四条核心裁定（来自 theta_v0/strategy/chong.rs）
//!
//! 1. **键 = (标的, 操作级别)，成本状态不进键**（ADR 0013）
//! 2. **各重的钱独立**（ADR 0013）：N 份专属筹码 + 一个全局上限
//! 3. **各重保证金逐仓分开**（ADR 0014）：posted = Σₖ nₖ/L_maxₖ 全额计提
//! 4. **重内单向 / 重间不仲裁**（ADR 0014）：一重内部合成后必须同向，不穿零
//!
//! ## 当前状态
//!
//! - ✅ ChongKey 和 ChongState 类型定义
//! - ✅ ChongEngine trait 签名
//! - ❌ 完整实现（待后续迭代）

use std::collections::BTreeMap;

/// 重的键 = **(标的, 操作级别)**（ADR 0013 裁定二：成本状态不进键）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChongKey {
    /// 标的 S（重绑死标的）
    pub symbol: String,
    /// 操作级别 ℓ（外生参数，与塔的结构级别同名异物）
    pub op_level: u8,
}

/// 重的状态 = ⟨专属筹码 Q，当前成本，当前持仓⟩。
///
/// 对应 `theta_v0::strategy::chong::Chong`，核心字段：
/// - quota_usd: 专属筹码（名义美元）
/// - cost_basis: 当前成本（多空分开记账）
/// - position: 当前持仓（正=多头，负=空头）
#[derive(Debug, Clone, PartialEq)]
pub struct ChongState {
    /// 专属筹码 Q（名义美元）——各重的钱独立，互不挪用（ADR 0013 裁定一）
    pub quota_usd: f64,
    
    /// 当前成本（待完整迁移时添加 CostBasis 结构体）
    pub cost_basis_placeholder: f64,
    
    /// 当前持仓（正=多头，负=空头，0=空仓）
    pub position: f64,
}

/// Chong 引擎 trait——操作级别旁路的核心接口。
///
/// 实现者负责：
/// 1. 维护多重状态（BTreeMap<ChongKey, ChongState>）
/// 2. 执行建仓/平仓操作并更新成本
/// 3. 验证约束（重内单向、逐仓计提、全局上限）
///
/// ## 判定唯一性
///
/// 本 trait 是操作级别旁路判定的**唯一生产入口**。
pub trait ChongEngine {
    /// 获取指定重的当前状态。
    ///
    /// # 返回
    ///
    /// - Some(state) - 该重存在且有状态
    /// - None - 该重未开户或已清空
    fn get_chong_state(&self, key: &ChongKey) -> Option<&ChongState>;
    
    /// 创建新重（开户）。
    ///
    /// # 参数
    ///
    /// - `key` - 重的键（标的 + 操作级别）
    /// - `quota_usd` - 专属筹码（名义美元）
    ///
    /// # 返回
    ///
    /// - Ok(()) - 开户成功
    /// - Err(msg) - 开户失败（如键已存在、配额无效等）
    fn create_chong(&mut self, key: ChongKey, quota_usd: f64) -> Result<(), String>;
    
    /// 执行操作（建仓/加仓/减仓/平仓）。
    ///
    /// # 参数
    ///
    /// - `key` - 目标重
    /// - `operation` - 操作类型和参数（占位，完整定义待迁移）
    ///
    /// # 返回
    ///
    /// - Ok(new_state) - 操作成功，返回更新后的状态
    /// - Err(msg) - 操作失败（如违反单向约束、配额不足等）
    fn execute_operation(
        &mut self,
        key: &ChongKey,
        operation: &OperationPlaceholder,
    ) -> Result<ChongState, String>;
    
    /// 获取所有重的当前状态快照。
    fn all_chongs(&self) -> &BTreeMap<ChongKey, ChongState>;
}

/// 操作占位类型（完整定义待迁移，包括：开多/开空/加仓/减仓/平仓）
#[derive(Debug, Clone)]
pub struct OperationPlaceholder;

#[cfg(test)]
mod tests {
    use super::*;

    /// 骨架测试：ChongKey 可排序（BTreeMap 要求）
    #[test]
    fn chong_key_orderable() {
        let k1 = ChongKey {
            symbol: "AAPL".to_string(),
            op_level: 1,
        };
        let k2 = ChongKey {
            symbol: "AAPL".to_string(),
            op_level: 2,
        };
        assert!(k1 < k2);
    }

    /// 骨架测试：ChongState 可克隆
    #[test]
    fn chong_state_cloneable() {
        let state = ChongState {
            quota_usd: 10000.0,
            cost_basis_placeholder: 0.0,
            position: 0.0,
        };
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }
}
