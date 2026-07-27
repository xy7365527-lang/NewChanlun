//! 互斥全定义策略 element-coverage 执行引擎——port **M29 互斥全定义策略最终定理**
//! （`Origin/MutexFinalTheorem.lean` + `SeparateStrategyTarget.lean`(M16/M17) +
//! `AncestorClosure.lean`(M16 AncOK) + `OperationRole.lean`(M09 Role 四分)）。
//!
//! ## 工位定位（生产引擎核心，与买卖点 v1 正交的新路径）
//!
//! 被全窗 L3 8/8 否证的是**买卖点 v1**（`strategy/mod.rs::recognize` 每 bsp 一 decision，
//! 离散择时）。**本文件不是它**——本文件实装 Lean 已全量真封的**全定义策略 = element-coverage**：
//! 在**每个语法元素** λ_e 入场、ρ_e 平腿，**覆盖每个笔/线段/走势**（多级嵌套赋格），而非只在
//! 离散买卖点动作。这是 Lean M29 三结论合一的「互斥全定义策略 π_Θ」在 rust 执行层的兑现
//! （rust 执行层此前零实装——`LegTarget` / 活动集 `A_t/B_t/D_t` 只在 Lean，本文件首次港下来）。
//!
//! ## Lean → rust 语义映射（每 bar t 的五步）
//!
//! | 步 | Lean 规格 | rust 实装 |
//! |----|----------|----------|
//! | 1 元素集 E | `SyntaxElement`(§二 C27 四元组 `(I_e,ε_e,ℓ_e,par)`) | [`CoverageElement`]（从 classifier levels + `RMove::Compose` 塔提取）|
//! | 2 活动集 | M16 `A_t`/`B_t`/`D_t` + `AncOK[(A_t∖D_t)∪B_t]` | [`active_set_step`]（先关后开 + 祖先闭合）|
//! | 3 角色 R(g) | spec §8 `R(g)=(H(g),V(g),δ_g)` 24 类（3×4×2） | [`operation_role`]（H 水平 × V 垂直 × δ 方向三轴）|
//! | 4 LegTarget | M17/M28 每活动元素一腿（方向 ε_e，单位 s_e） | [`leg_target`]（role/depth 权重 `w_depth`）|
//! | 5 净额执行 | Nautilus 净额兼容（毛账本 → 净持仓） | [`net_target_units`]（所有腿合并为净 `units:f64`）|
//!
//! ## ★真 Fugue 严防级别差伪造（铁律，mod.rs:433-440 codex 已裁旧 bug）
//!
//! `recognize`（v1）旧 bug：用 `depth = l_star - level_idx` 把**级别差**伪造成赋格嵌套深度
//! （codex 异质裁决 + L2 OKLO bisect 坐实 2026-06-27）。本文件**严禁**重蹈：[`CoverageElement`]
//! 的 `depth` / `parent` 只来自 `RMove::Compose.subs` 的**真父子关系**（descend 取回的真嵌套
//! 子走势），不把「元素在第几级」当作 depth。短差（ReverseOpen）的反向子声部腿 depth≥1 必须有
//! 真 Compose 父（[`extract_elements`] 的 `parent` 字段来自塔的真嵌套结构）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - **L1**（本文件）：element-coverage 引擎消费 classifier 塔产元素集 + 活动集递归 + Role 派生 +
//!   LegTarget 净额合并 = bit-exact 对齐 Lean M29 规格的**管线正确性**（验证管线，零信息增量）。
//! - **NOT L2 alpha**（gatekeeper，no-声明膨胀）：element-coverage 有无净账户 alpha 是 L2/L3
//!   经验问题，本文件**不预判有 alpha**。Lean M29 顶点诚实声明（`MutexFinalTheorem.lean` §7）：
//!   M29 是 goal 的**形式化**顶点（L0 语法层完备性：角色互斥分类 + 全元素覆盖 + 策略全定义 + ∃!），
//!   与全窗 L3（v1 实盘择时 8/8 否证）**不矛盾**——「分类完备性 ≠ 择时盈利」。本引擎是 M29 的
//!   rust 兑现，同样**不蕴含**实盘 alpha；是否盈利由下一步 L2/L3 净额回测否证检验（Lead 派）。
//!
//! ## owner 边界（互斥铁律）
//!
//! 本文件**新建**，自登记 `pub mod coverage;` 于 `strategy/mod.rs`。**不改**买卖点 v1
//! `recognize`（保留作基线对照）。runner 若需新入口，新增函数不改现有 `run_theta_v0`。
//!
//! immutable 风格：所有构造新对象，不原地修改（活动集递归返回新集合，不 mutate 旧集合）。

use super::super::classifier::descend::RMove;
use super::super::classifier::recursive_tower::{ElementId, LeveledMove};
#[cfg(test)]
use super::super::classifier::recursive_tower::{compose_level, compose_level_resume};
use std::rc::Rc;
use super::super::classifier::Classification;
use super::super::config::{RiskConfig, ThetaDirPreset, VoiceConfig};
use super::super::types::{Direction, Order, StrictAction};
use super::super::closed_loop::state::RiskMode;
use super::super::closed_loop::transition::stage_progression_eta_corrected;
use super::account;
use super::intent::{lex_argmin, lex_argmin_top_k, JThetaKey, LexCandidate};
use super::interp::{self, ActiveLeg, Buckets, Candidate};
use super::ledger::{RiskPolicy, TStage, TwEvent, TwState};
use super::protocol::{ProtocolEvent, ProtocolEventSet};
use super::voice::{depth_weight, VoiceSide};

/// DA-Q2 订单轨决策类型（保持既有 `Order` 逐字段语义）。
pub type OrderDecision = Order;
/// π_Θ 双轨积类型：订单 P1..P10 × 独立协议事件。
pub type PiThetaDecision = (OrderDecision, ProtocolEvent);

// ════════════════════════════════════════════════════════════════════════════
//  AncOK L2 探针（697 号 ceiling 暴露面度量）——thread_local 计数器
// ════════════════════════════════════════════════════════════════════════════
// ponytail: 常驻探针，非临时诊断。697 ceiling 自陈「L2/L3 待验证」——保留探针 = 让任一真实
// 数据 run 可复测 Stale 四态分派频次与 restore 祖先链恢复率。增量成本（thread_local Cell +=1）
// 只在 held-leg Stale/restore 路径命中（cdylib 分类热路径不经此），被环绕的 HashMap 操作淹没。
// 计数纯旁路，不改任何 work/raw 控制流 ⟹ 生产 bit-exact 不变。

mod ancok;
mod element;
mod role;
mod leg;
mod held;
mod step;
mod sizing;
mod compose;
#[cfg(test)]
mod test_support;

pub use ancok::{AncokProbe, active_set_step, ancestors, ancok_probe_reset, ancok_probe_snapshot};
use ancok::{ancestors_by_id_lookup, ancok_probe_bump};

pub use element::{CoverageElement, attach_bsp_carrier_indexed, attach_bsp_parent_carrier_indexed, attach_bsp_to_tree, attach_bsp_to_tree_indexed, build_tree_endpoint_index, dual_view_consistency, ending_set, extract_carrier_forest, extract_elements, from_classification_levels, starting_set};
pub(crate) use element::{ElementView, build_tree_id_index};

pub use role::{Dir, GradeRel, Horizontal, OperationRole, Vertical, build_prev_sibling_index, grade_relation, horizontal_relation, operation_role, vertical_relation};
pub(crate) use role::operation_role_indexed_split;
use role::{dir_sign, operation_role_two_segment};

pub use leg::{LegTarget, SepLeg, dir_weight, gross_target_units, leg_target, net_target_units, overlay_net_delta, w_grade};
use leg::{apply_gross_cap, strategy_target_legs};
#[cfg(test)]
use leg::element_depth;

use held::{HeldLegMatch, close_indices, element_as_leg, held_leg_tree_index_indexed, held_stale_reregister_idx, restore_ancestor_chain_from_registry};

pub use step::coverage_step_classification;
#[cfg(test)]
pub(crate) use step::coverage_step_prebuilt;
use step::coverage_step_from_buckets_sep;
#[cfg(test)]
use step::coverage_step_from_buckets;

pub use sizing::{KThetaRiskGate, PiThetaWeights, pi_theta_position, pi_theta_step, schedule_order};
use sizing::feasible_lex_candidates;
#[cfg(test)]
use sizing::pi_theta_step_prebuilt;

pub use compose::{t1_target_residual_probe_bump, t1_target_residual_probe_count, t1_target_zero_probe_bump, t1_target_zero_probe_count, t1_target_zero_probe_reset};
pub(crate) use compose::{StepTrace, TwStepCtx, VoiceVerdict, pi_theta_step_traced};
