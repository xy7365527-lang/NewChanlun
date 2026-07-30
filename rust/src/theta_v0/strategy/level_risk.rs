//! ★LEE M4 级别资金权 `w_ℓ` + 级别级风险帽（multi-level-native-execution-design-20260719
//! §D M4 行）。
//!
//! ## 本文件做什么：定义「每个塔级别 ℓ 分到多少资金比例」，**不**决定订单
//!
//! M4 定义（设计文档 §D 迁移表逐字）：「w_ℓ 资金权、级别级风险帽上线；voice.rs depth_weights
//! 与 LEE w_ℓ 的对偶关系统一」。本模块只产**权重**与**权重和校验**；把权重兑现为实际的
//! 结构基准裁剪（`cap_ℓ = w_ℓ·γ̄·U_ℓ`）在 [`super::coverage::level_cap`]／
//! [`super::coverage::clamp_levels_to_weighted_cap`]（账户/风控域，§F③ 同纪律：结构域的
//! `level_order.rs::regate` 不碰风险帽）；帽的实际施加点在 `fill.rs::plan_level_gated_order`。
//!
//! ## ★对偶统一声明（depth_weight ↔ w_ℓ，票体强制要求「谁主谁从，禁双重定价」）
//!
//! 两个权重表操作的是**不同轴**，不是同一件事的两次定价：
//!
//! | 权重 | 轴 | 定义域 | 施加点 | 施加对象 |
//! |---|---|---|---|---|
//! | `depth_weight`（`voice.rs`） | **声部深度**（同一根信号内的嵌套对冲层，depth=0,1,2…） | 单个 root 的声部树 | `coverage.rs::leg_target`／`strategy_target_legs`（**结构域**，leg 生成时） | 单条 leg 的 `q_units` |
//! | `level_weight` / `w_ℓ`（本模块） | **塔级别**（L1..L5 独立递归层，`ElementId.level`） | 跨 root 的整个级别账本 `Ledger_ℓ` | [`super::coverage::clamp_levels_to_weighted_cap`]（**账户/风控域**，`regate` 之后、`pi_theta_position` 之前） | 该级别**已聚合**的结构净目标 `net_ℓ`（多条 leg 之和） |
//!
//! **谁主谁从**：depth_weight 是**内层/先手**——它在单条 leg 诞生时就把资金分配定死（哪个
//! 深度的对冲腿拿多少 `b_v`），这个分配已经沉淀进 `net_ℓ`（`level_nets` 对同级别所有 leg
//! 求和，含各深度贡献）。level_weight 是**外层/后手**——它不重新审视任何一条 leg 内部怎么分，
//! 只对**已经算好的** `net_ℓ` 总量做跨级别的帽约束。二者是同一笔资金在两个正交维度上的
//! 依次投影（先深度、后级别），**不是同一维度算两次**：depth_weight 从不读跨级别的信息，
//! level_weight 从不拆开单条 leg 内部的深度构成——没有任何一块资金被两套权重各定价一次。
//!
//! **禁双重定价的可证伪形式**：若未来有人试图在 `strategy_target_legs`（depth_weight 的
//! 施加点）里也乘上 `level_weight`，会导致同一 `net_ℓ` 被 depth 与 level 两个权重连乘（而不是
//! depth 权重决定构成、level 权重决定裁剪），使 `Σw_ℓ≤1` 与「级别帽 = γ̄·U_ℓ 的一个正确
//! 子集」这条关系失真（帽变成 `w_ℓ` 的非线性函数）。本模块的施加点严格限定在
//! [`super::coverage::clamp_levels_to_weighted_cap`]，禁止在 leg 生成路径引入。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L0**（定义内蕴）：`level_weight` 取值、`Σw_ℓ` 求和是给定 `RiskConfig` 后的全函数，不依赖
//! 任何跑批数据；`level_weights_sum_le_one` 是配置层的代数校验，不是经验断言。
//!
//! **★参数归属（090/v3 强制声明）**：`w_ℓ` 全属 **Θ_risk**——级别之间怎么分配资金上限是
//! 风险配置的选择（与 `depth_weights`/`gamma`/`rho` 同类），**不是**缠论结构可导出的量。缠论
//! 只定义「有哪些级别」（塔级别 = 递归深度），不定义「每个级别该给多少资金」——后者是执行层
//! 风险预算问题，本模块每一处新增权重语义都不得表述为「缠论推导」。

use super::super::config::RiskConfig;

/// 取级别 `level` 的资金权重 `w_ℓ`（`config.risk.level_weights` 按 level 索引，越界/空表 ⟹ 0.0）。
///
/// 边界条件：与 [`super::voice::depth_weight`] 同纪律——**未用部分保留现金不重分配**（表外
/// 级别 w=0，自然不获资金，非错误）。空表（M4 未配置）⟹ 全部级别 w=0；此时调用方**必须**
/// 经 `risk.enforce_level_cap` 开关短路本函数，否则会把所有级别帽误裁到 0（见
/// [`super::coverage::clamp_levels_to_weighted_cap`] 的开关门禁）。
pub fn level_weight(level: u32, risk: &RiskConfig) -> f64 {
    risk.level_weights
        .get(level as usize)
        .copied()
        .unwrap_or(0.0)
}

/// `Σ_ℓ w_ℓ`（配置表内全部级别权重之和，L0 代数求和）。
pub fn level_weights_sum(risk: &RiskConfig) -> f64 {
    risk.level_weights.iter().sum()
}

/// ★票体验收「Σw_ℓ ≤ 1」机器断言（不是文档承诺——本函数是唯一权威判据）。
///
/// 空表（Σ=0）也满足 `≤1`（平凡满足，M4 未配置时不误报违规）。
pub fn level_weights_sum_le_one(risk: &RiskConfig) -> bool {
    level_weights_sum(risk) <= 1.0 + 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risk_with(weights: Vec<f64>) -> RiskConfig {
        RiskConfig {
            level_weights: weights,
            ..RiskConfig::default()
        }
    }

    /// ★按 level 索引取值，越界/空表 ⟹ 0.0（同 `depth_weight` 纪律）。
    #[test]
    fn level_weight_indexes_by_level_zero_out_of_range() {
        let risk = risk_with(vec![0.5, 0.3, 0.2]);
        assert_eq!(level_weight(0, &risk), 0.5);
        assert_eq!(level_weight(1, &risk), 0.3);
        assert_eq!(level_weight(2, &risk), 0.2);
        assert_eq!(level_weight(3, &risk), 0.0, "表外级别 w=0，非错误");
        assert_eq!(level_weight(0, &RiskConfig::default()), 0.0, "空表 ⟹ 全 0");
    }

    /// ★Σw_ℓ ≤ 1 机器断言：合规/超限/边界/空表四态。
    #[test]
    fn level_weights_sum_le_one_covers_boundary() {
        assert!(
            level_weights_sum_le_one(&risk_with(vec![0.5, 0.3, 0.2])),
            "Σ=1.0 合规（边界含）"
        );
        assert!(
            level_weights_sum_le_one(&risk_with(vec![0.3, 0.3])),
            "Σ=0.6<1 合规"
        );
        assert!(
            !level_weights_sum_le_one(&risk_with(vec![0.6, 0.5])),
            "Σ=1.1>1 违规，必须被抓到"
        );
        assert!(
            level_weights_sum_le_one(&RiskConfig::default()),
            "空表 Σ=0 平凡合规"
        );
    }

    /// ★空表时禁止权重恰好落在合规边界之外的浮点误差误报（1e-9 容差）。
    #[test]
    fn level_weights_sum_le_one_tolerates_float_noise_at_boundary() {
        let risk = risk_with(vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]);
        assert!(
            level_weights_sum_le_one(&risk),
            "1/3×3 的浮点和应在容差内判合规"
        );
    }
}
