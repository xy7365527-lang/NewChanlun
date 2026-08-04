//! ★LEE M4 级别资金权 `w_ℓ` + 级别级风险帽（multi-level-native-execution-design-20260719
//! §D M4 行）。
//!
//! ## 本文件做什么：定义「每个塔级别 ℓ 分到多少资金比例」，**不**决定订单
//!
//! M4 定义（设计文档 §D 迁移表逐字）：「w_ℓ 资金权、级别级风险帽上线；voice.rs depth_weights
//! 与 LEE w_ℓ 的对偶关系统一」。本模块只产**权重**与**权重和校验**；把权重兑现为实际的
//! 结构基准裁剪（`cap_ℓ = w_ℓ·γ̄·U_ℓ`）在 [`super::coverage::level_cap`]／
//! [`super::coverage::clamp_levels_to_weighted_cap`]（账户/风控域，§F③ 同纪律：结构域的
//! `level_order.rs::regate` 不碰风险帽）；帽的实际施加点在 `fill.rs:5180` 的
//! `enforce_level_cap` 门内（`clamp_levels_to_weighted_cap` 生产唯一调用点 = `fill.rs:5195`）。
//! ⚠️ 订正（#889 R4）：本行此前写「施加点在 `fill.rs::plan_level_gated_order`」——#841 查实
//! 该函数全仓**不存在**（6 处点名引用、0 处定义、0 处调用），指针作废如上。
//!
//! ## ★对偶统一声明（depth_weight ↔ w_ℓ）——#841 实测订正版
//!
//! **#841（2026-08-01 关票）已推翻本段原论证前提「两轴正交」**：实测 **绝对级别 =
//! `root_level − depth`**（前 2 万 bar 反例 0/1,756,996、前 20 万 bar 反例 0/13,265,200，
//! 探针与生产对拍 105.8 万次全过后取数）——parent 链每上溯一步 level 恰好 −1
//! （`coverage/element.rs:200-202` 铁律「子元素 level = 父 level − 1」）。故「声部深度」
//! 与「塔级别」是**同一条级别轴**的相对/绝对两种坐标，不是两个正交维度；两张权重表分的
//! 是**同一根轴上的钱**（一个是根的函数、一个是级别的函数）。
//!
//! | 权重 | 轴 | 定义域 | 施加点 | 施加对象 |
//! |---|---|---|---|---|
//! | `depth_weight`（`voice.rs`） | 级别轴·**相对坐标**（相对当前最高有效决策级别 L* 的下溯层数 depth=0,1,2…） | 单个 root 的声部树 | `coverage.rs::leg_target`／`strategy_target_legs`（**结构域**，leg 生成时，**乘法**：`w = depth_weight × dir_weight × w_grade`，`coverage/leg.rs:159`） | 单条 leg 的 `q_units` |
//! | `level_weight` / `w_ℓ`（本模块） | 级别轴·**绝对坐标**（`ElementId.level`） | 跨 root 的整个级别账本 `Ledger_ℓ` | [`super::coverage::clamp_levels_to_weighted_cap`]（**账户/风控域**，`fill.rs:5180` 门内，**clamp 取 min，非乘法**） | 该级别**已聚合**的结构净目标 `net_ℓ`（多条 leg 之和） |
//!
//! **谁主谁从**：depth_weight 是**内层/先手**——它在单条 leg 诞生时就把资金分配定死，这个
//! 分配已经沉淀进 `net_ℓ`（`level_nets` 对同级别所有 leg 求和，含各深度贡献）。level_weight
//! 是**外层/后手**——它不重新审视任何一条 leg 内部怎么分，只对**已经算好的** `net_ℓ` 总量
//! 做跨级别的帽约束。
//!
//! **「禁双重定价」结论仍成立，但成立的理由变了（#841 ③）**：真正守住「不连乘」的**不是**
//! 正交性（已否证），而是「**一个乘法、一个 clamp**」这个施加形态——depth_weight 在 leg 生成
//! 时**乘**进 `q_units`，level_weight 在聚合净额上**取 min 裁剪**，二者从不对同一笔资金各乘
//! 一次。⚠️ 这是**偶然形态不是结构保证**：若把 `cap_ℓ` 从 clamp 改成比例缩放（乘法），同一
//! `net_ℓ` 立刻被 depth 与 level 两套权重连乘，原论证给不出保护。原可证伪形式（「若在
//! `strategy_target_legs` 里也乘 `level_weight` 就连乘」）**从未被触发**——即原论证从未被
//! 真正检验过（#841 ③ 照实登记）。本模块的施加点仍严格限定在
//! [`super::coverage::clamp_levels_to_weighted_cap`]，禁止在 leg 生成路径引入——**且禁止把
//! clamp 改成乘法形态**（前者是现有代码的施加点纪律，后者是 #841 订正后新增的等重纪律）。
//!
//! **#841 ② 连带后果（照实）**：同一绝对级别可被多条 `(root_level, depth)` 路径同时供给
//! （实测 20 万 bar 窗：L0 被 (0,0)/(1,1)/(2,2) 三条、L1 被 (1,0)/(2,1)/(3,2) 三条、L2 被
//! (2,0)/(3,1) 两条；严格同 bar 口径 96.10% 采样 bar）⟹ `040:20`「每个级别对应一定的资金
//! 与筹码」在本仓**不是良定义的**——级别的资金份额不是级别的函数，是根的函数。本模块的
//! `w_ℓ` 是唯一按**绝对**级别分钱的表（default 关闭，见下）；它的名分与份额数值归 #839
//! 形态裁定与 #835 分区比例票，本文件不裁。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L0**（定义内蕴）：`level_weight` 取值、`Σw_ℓ` 求和是给定 `RiskConfig` 后的全函数，不依赖
//! 任何跑批数据；`level_weights_sum_le_one` 是配置层的代数校验，不是经验断言。
//!
//! **★参数归属（090/v3 强制声明，#840 R4 / #889 订正版）**：`w_ℓ` 的**数值**全属 **Θ_risk**
//! ——各级资金上限取多少是风险配置的选择（与 `depth_weights`/`gamma`/`rho` 同类），缠论只给
//! 骨架不给数。但**「按级别配资金与筹码」这个机制的存在性有原文**：`038-第38课.md:30`【正文】
//! 「你可以设定某个量的筹码按某个级别的分解操作，另一个量的筹码按另一个更大级别的分解操作」、
//! `040-第40课.md:20`【正文】「每一重都对应着一定的资金与筹码」、`040-第40课.md:22`【正文】
//! 「只是在不同级别中投入的筹码与资金不同而已」——无原文的只是**数值**（量词只有「一定的／
//! 某个量」）。本段此前写「缠论不定义『每个级别该给多少资金』」，#840 裁定**只对一半**
//! （存在性有原文、数值无原文）。据此照实写：`enforce_level_cap=false` + 空表 default
//! （`config.rs:228-229`）**关掉的不是与原文无关的默认，是有原文骨架的机制**——`040:20`
//! 唯一明确要求的那件事，生产上不生效（#840 仓内落位）。另照实（#841 ⑤）：该开关**不是
//! 死开关**——`enforce_level_cap=true` 反事实四臂（含 2 万 bar 复测共八组对照）与 default
//! 臂无一逐位相同，帽真 binding；default 臂在实测窗口大亏、压小仓位自然少亏，「帽越紧亏损
//! 越小」**不是 alpha 证据**（#841 诚实补注原文）。

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
