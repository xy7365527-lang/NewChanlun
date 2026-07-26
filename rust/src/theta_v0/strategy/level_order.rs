//! ★LEE M2/M3 级别订单台账 `LevelOrderLedger`（multi-level-native-execution-design-20260719
//! §C.2 伪码「order_raw = Σ_ℓ Δq_ℓ」/ §D 迁移表 M2、M3 行）。
//!
//! > **读本文件前先读这一段**：下面的 §M2 各节描述的是 **M2 阶段**的口径，其中三条已被
//! > **M3 反转**，勿按 M2 文案理解现行代码——① 目标不再每 bar 重估（改 clock_ℓ 门控）；
//! > ② 订单流不再与 M0 bit-exact（M3 起契约就是分叉）；③ `max_abs_order_residual` 不再恒 0。
//! > 现行口径以 **§M3 事件门控**节（本文件后半）为准；M2 各节保留是为了让谱系可读
//! > （012号：谱系优先于汇总），不是现行契约。
//!
//! # §M2（**历史口径**，已被 M3 反转三条）：物理订单的**量**由 `Σ_ℓ Δq_ℓ` 承载
//!
//! M2 定义（设计文档 §D 迁移表逐字）：「物理订单改由 Σ_ℓ Δq_ℓ 生成（各级目标仍每 bar 重估），
//! 证明 Σ_ℓ Δq_ℓ == ΔN，订单流与 M0 逐 bar 相等」，验收「bit-exact：成交序列不变，仅归因维度
//! 增加」。
//!
//! **能力边界（照实声明，090 反声明膨胀）**：本模块**不**独立决定订单量。物理整数目标 `T` 由
//! 账户层聚合决策锚定（`p_t + Schedule_Θ 的有符号手数`），级别台账把 `T` 划分成 `q_ℓ` 并产
//! `Δq_ℓ`；订单量 `= Σ_ℓ Δq_ℓ` 因此**恒等于** ΔN——这是**构造性同义反复（L0）**，不是可证伪
//! 的实证结论。推导方向与 §C.2 伪码字面（`order_raw = Σ_ℓ Δq_ℓ` 在前、`K_Θ_gate` 在后）**相反**：
//! 伪码是 M4 的终态形态（级别独立 sizing 后各级目标才真正独立），M2 的两条约束
//! （①订单由级别增量之和承载 ②与 M0 bit-exact）在「M2 不改聚合决策」的前提下**只允许**
//! 事后划分——见下节。诚实的一句话总结：**M2 交付的是「ΔN 的级别归因划分台账 + 订单量改经
//! 该台账出口」，不是「级别独立地决定了下多少单」**。
//!
//! 三个量，逐决策点：
//!
//! | 量 | 定义 | 不变量 |
//! |---|---|---|
//! | `held_ℓ` | 已**成交**的按级归因持仓 | `Σ_ℓ held_ℓ ≡ p_t`（净额账本 units，整数手） |
//! | `q_ℓ` | 本决策点各级**物理目标** | `Σ_ℓ q_ℓ ≡ T`（T = lot 可实现物理净目标） |
//! | `Δq_ℓ` | `q_ℓ − held_ℓ` | `Σ_ℓ Δq_ℓ ≡ T − p_t ≡ ΔN`（订单量，i64 精确） |
//!
//! 订单 `qty = |Σ_ℓ Δq_ℓ|`。**动作分类**（Buy/Sell/Add/Reduce/Close/Hold/Wait）仍取净额语义的
//! `Schedule_Θ`（设计文档 §C.2 伪码末行逐字「沿用 Schedule_Θ 单出口」）——M2 改的是**量的来源**，
//! 不是账户层动作语义；级别独立时钟（M3）与级别 sizing（M4）明确不在本步。
//!
//! ## 为什么 M2 的 Δq_ℓ 只能是 ΔN 的精确划分（结构性事实，非实装取巧）
//!
//! M2 同时受两条约束：①「订单由 Σ_ℓ Δq_ℓ 生成」；②「订单流与 M0 逐 bar bit-exact」。M0 的
//! 聚合决策（p̃ → `LexArgmin_{p∈𝒦_Θ}J_x(p)` → p\*）在 M2 **不改**（改它属 M4 级别 sizing），
//! 故 Σ_ℓ Δq_ℓ 只能恰好等于 M0 的 ΔN——即 `{Δq_ℓ}` 必须是 ΔN 的一个**精确划分**。M2 因此是
//! 「归因维度增加」而非「决策改变」，这正是迁移表把订单流分叉推迟到 M3 的原因（设计文档 §D M3
//! 行「本步起订单流与 M0 分叉，必须独立评审，不得借 M2 的 bit-exact 蒙混」）。
//!
//! ## 三类读数的证据等级（**M2 阶段**口径；M3 迁移见 §M3「等级迁移」节）
//!
//! > M3 变更两处：`max_abs_order_residual` 由 L0 恒 0 护栏 → 分叉幅度经验读数；下表所有
//! > 「L2」标注按 `formalization-validity-domain` 应为 **L1**（读数产自合成
//! > `random_walk_dataset`，不是真实行情）——**L2 需真实数据**。此处订正见 §M3 末节。
//!
//! | 读数 | 等级 | 说明 |
//! |---|---|---|
//! | `max_abs_order_residual`（`\|Σ_ℓ Δq_ℓ\| − qty_M0`） | **L0 同义反复** | 由 `T := p_t + qty_M0·sign` 的构造直接推出，**不可证伪**；保留仅作实装回归护栏（构造若被改坏立即非 0）。**不得**作为「M2 恒等成立」的实证证据引用。 |
//! | `max_abs_held_residual`（`Σ_ℓ held_ℓ − p_t`） | **L1 可证伪** | 跨**延迟成交/部分成交/拒单/丢单**的实际成交归因不变量。`on_fill` 消费的是 `units_after − units_before`（真实成交），与计划量无关 ⟹ 归因比例回缩若写错，此残差立即非 0。这是本模块唯一真正被数据检验的恒等。 |
//! | `n_rescaled` / `max_abs_struct_gap`（`Σ_ℓ net_ℓ` vs `T`） | **L1 经验读数**（M2 文案标 L2，见上方订正） | 结构级别净额与账户层物理目标的**分歧幅度**，纯观测无断言。这是 M2 阶段唯一产生信息增量的地方（M3 另增稀疏度与分叉幅度两项）：分歧不为零意味着「级别结构说的仓位」与「账户层能下的仓位」不一致，M3/M4 必须正面处理它（M2 按比例吸收，**照实登记为待裁决口径**，不冒充已解决）。 |
//!
//! ## 归因算子（确定性 + 整数精确）
//!
//! [`attribute_total`]：给定结构基准 `basis_ℓ`（= 该决策点各级结构净额 `net_ℓ`，
//! [`super::level_ledger::level_nets`] 单源）与物理总量 `T`，产 `q_ℓ` 且 `Σ_ℓ q_ℓ ≡ T`：
//!
//! - `Σ basis_ℓ == T`（**常态路径**：结构净额恰好可实现）⟹ 恒等映射，零缩放零重排；
//! - `Σ basis_ℓ ≠ 0` ⟹ 最大余数法按 `basis_ℓ` 比例缩放（i128 整数商 + 余数排序补位，
//!   无浮点 ⟹ **无结合律重排误差**）。缩放的语义依据：`𝒦_Θ` 是**账户层**可行性约束
//!   （设计文档 §C.1「净额降级为账户层约束」）⟹ 帽约束按结构比例回缩是级别中性的；
//! - `Σ basis_ℓ == 0 且 T ≠ 0` ⟹ 全部落 [`LEVEL_ACCOUNT_RESIDUAL`] 残差桶，**显式声明
//!   「无结构级别可归因」**，不伪造级别身份（roadmap:64「不得伪造中间级别证书」同纪律）。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L0/L1**：`Σ_ℓ Δq_ℓ ≡ ΔN` 是构造性整数恒等（划分的加法），零信息增量；订单流 bit-exact 是
//! 管线正确性证据，**不声明 alpha**（同 `overlay_state.rs` / `level_ledger.rs` 的认识论声明）。
//! **M3 起「订单流 bit-exact」不再成立**（分叉是契约），该句仅描述 M2 阶段。
//!
//! ---
//!
//! # ★M3 事件门控（设计文档 §D 迁移表 M3 行）：本模块自 M3 起承载**两个**级别态
//!
//! M3 定义（逐字）：「`Ledger_ℓ` 只在 `clock_ℓ` 事件时点重估目标；无事件 bar 目标=前值」。
//! 事件集定义见 [`super::level_clock`]（含 `LevelState` 逐字段对齐与缺口登记）。
//!
//! ## 两个级别态，勿混（M2 只有一个）
//!
//! | 态 | 字段 | 何时变 | 不变量 |
//! |---|---|---|---|
//! | **已成交归因** `held_ℓ` | [`LevelOrderLedger::held`] | `on_fill`（真实成交） | `Σ_ℓ held_ℓ ≡ p_t`（**L1 可证伪**，M2 起不变） |
//! | **结构计划** `q_ℓ^plan` | [`LevelOrderLedger::planned`] | `regate`（clock_ℓ 有 tick 的级别）+ `commit_planned`（下单） | `Σ_ℓ q_ℓ^plan ≡ T_prev`（构造性） |
//!
//! 二者的差 `Σ_ℓ q_ℓ^plan − p_t` 是**在途/未兑现缺口**（拒单、部分成交、`close_only` 上限）
//! ——M3 **不**自动补单（见下「为什么不锚 `p_t`」），因此这个缺口是真实的欠达。它是
//! [`LevelOrderStats::max_abs_plan_fill_gap`]（**L2 经验读数**，只登记不断言）。
//!
//! ## 为什么增量锚在 `q_ℓ^plan` 而不是 `held_ℓ`（M2 的锚）
//!
//! 设计文档的稀疏性不变量逐字：「bar i 无 ℓ 级事件 ⟹ `Δq_ℓ(i)=0`」，§C.2 伪码同字面
//! （`if events_ℓ.is_empty(): continue` ⟹ `Δq_ℓ` 根本不产生），票体验收同字面
//! （「订单时点集合 ⊆ 事件时点并集」）。三处独立表述一致 ⟹ **无歧义**：
//!
//! - 锚 `q^plan`：无 tick ⟹ 目标不变 ⟹ `Δq_ℓ = q_ℓ − q_ℓ^plan = 0` **恒成立**，稀疏性是构造性的；
//! - 锚 `held`（M2 口径）：拒单/部分成交后 `held ≠ 目标` ⟹ 后续**无事件** bar 上仍产订单
//!   （自动重试）⟹ 订单时点溢出事件集，稀疏性被破坏。
//!
//! 代价照实声明（090）：M3 **放弃了 M0/M2 的自动重试语义**——被拒的手数不会在下一个无事件
//! bar 补回，要等下一个 `clock_ℓ` 事件才重估。这是设计文档字面的直接后果，不是实装取巧；
//! 幅度由 `max_abs_plan_fill_gap` 逐决策点登记，不冒充为 0。
//!
//! ## 风控/账户层投影不在本模块（§F③ 域分离）
//!
//! `regate` 只产**结构**计划 `Σ_ℓ q_ℓ^plan = p̃_lee`。它到物理目标 `T_lee` 的投影
//! （`𝒦_Θ` 帽 + `force_flat/stop_long/stop_short` 风控门）由调用方 `fill.rs` 经
//! [`super::coverage::pi_theta_position`] **每 bar 无条件**施加 ⟹ 事件门控不门控风控
//! （票体硬约束）。本模块只接受投影后的 `T_lee` 作 [`LevelOrderLedger::plan_gated`] 入参。
//!
//! ## ★与 #308「L2 结构分歧 40%」待裁输入的**相交点**（照实登记，**不擅自裁决**）
//!
//! #308 浮出并登记为 M3/M4 待裁输入的 L2 分歧是：「逐腿 `q_units` 取整口径 vs `p̃` 聚合 lot
//! 量化口径不同」，在 `random_walk_dataset("RW3000M2", 3000, 40_000_000)` 上实测
//! `n_rescaled = 1190/3000`（≈40%）、`max|Σ_ℓ net_ℓ − T| = 5654`。
//!
//! M3 与该分歧**确有相交**——门控改变了它的**发生频率**（同一 fixture、同一读数口径，剥离
//! 对照实测）：
//!
//! | 读数 | 门控**关**（剥离对照 = M2 每 bar 重估） | 门控**开**（M3 交付态） |
//! |---|---|---|
//! | `n_rescaled` | 1190/3000（≈40%，**与 #308 登记逐值一致**） | **2/3000**（≈0.07%） |
//! | `max_abs_struct_gap` | 5654（**与 #308 登记逐值一致**） | 5401 |
//!
//! 结论必须分两句说，混为一句就是冒充已解决：
//!
//! 1. **频率下降是门控的算术后果**——重估次数从每 bar 降到事件时点，触发比例随之下降；
//! 2. **幅度基本未变（5654 → 5401）⟹ 分歧的根因未被 M3 触及**。单次重估时「结构说的仓位」
//!    与「账户层能下的仓位」的口径差依然存在，只是被门控问得少了。
//!
//! 因此：**M3 不裁决该分歧**，只把它的取值口径迁移到门控后的读数上，并如实登记两组数字。
//! 取整/量化口径究竟在哪一层统一（逐腿 vs 聚合）仍是 M4（级别 sizing `w_ℓ`）落定时必须
//! 正面裁决的输入，主控裁定前不得视作已解决（#308 待裁输入原文纪律）。
//!
//! ## M3 起 `max_abs_order_residual` 的等级迁移（**不再是恒 0 护栏**）
//!
//! M2 时它是 L0 同义反复（构造上 `|Σ_ℓ Δq_ℓ| ≡ qty_M0`）。M3 起订单流与 M0 分叉
//! （设计文档 §D M3 逐字「本步起订单流与 M0 分叉，必须独立评审，不得借 M2 的 bit-exact
//! 蒙混」）⟹ 该读数变为 **L2 分叉幅度**：`> 0` 是契约本身而非缺陷，`== 0` 反而说明门控
//! 没接上。M3 的验收因此**不**断言它为 0，改断言分叉见证（见 `runner.rs`）。

use std::collections::BTreeSet;

use super::level_attrib::{add_levels, attribute_total, merge_levels, sub_levels};
pub use super::level_attrib::{LevelUnits, LEVEL_ACCOUNT_RESIDUAL};
use crate::theta_v0::types::{Order, StrictAction};

/// ★M2 逐决策点归因见证读数（**release 可见**，非 `debug_assert`）。
///
/// #289 影子评审 MED 同款纪律：恒等证据必须在 release 下非平凡可读——只有 `debug_assert`
/// 的恒等在 release 跑批里零执行，等于没有证据。「残差恒 0」必须配「量级 > 0」才成对
/// （残差 0 而量级也 0 = 空转）。
///
/// **等级分层**（见模块头「三类读数的证据等级」表，勿混用）：`max_abs_order_residual` 是
/// **L0 同义反复**（构造性，不可证伪，仅作实装护栏）；`max_abs_held_residual` 是 **L1 可证伪**
/// （跨延迟/部分/拒单的实际成交归因）；`n_rescaled`/`max_abs_struct_gap` 是 **L2 经验读数**
/// （结构目标与账户物理目标的分歧，纯观测无断言）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelOrderStats {
    /// 决策点数（分母；非平凡性前置）。
    pub n_decisions: u64,
    /// 由 `Σ_ℓ Δq_ℓ` 生成且 `qty>0` 的订单数。
    pub n_orders_generated: u64,
    /// `max |Σ_ℓ Δq_ℓ|`（**非平凡性证据**：>0 才说明订单量真由级别增量之和承载）。
    pub max_abs_order_units: i64,
    /// `max | |Σ_ℓ Δq_ℓ| − qty_M0 |`。
    ///
    /// **等级随 M3 迁移**（见模块头「M3 起等级迁移」段）：M2 时是 L0 同义反复的恒 0 护栏；
    /// **M3 起是 L1 分叉幅度读数**（合成 fixture 上可证伪）——门控使级别目标与账户层每 bar
    /// 重估的净额目标分离，
    /// `>0` 是 M3 契约本身（订单流与 M0 分叉），不是缺陷。**不得**再作恒等护栏引用。
    pub max_abs_order_residual: i64,
    /// `max |Σ_ℓ held_ℓ − p_t|`（**L1 可证伪**：跨延迟/部分/拒单的实际成交归因完备，应恒 0）。
    pub max_abs_held_residual: i64,
    /// `max |p_t|`（**非平凡性证据**：>0 才说明持仓归因不是空账）。
    pub max_abs_net_units: i64,
    /// 落 [`LEVEL_ACCOUNT_RESIDUAL`] 桶的决策点数（诚实缺口计数）。
    pub n_residual_bucket: u64,
    /// 需比例缩放（`Σ_ℓ net_ℓ ≠ T`）的决策点数（**L2 经验读数**，无断言——实测在默认配置的
    /// 随机游走 fixture 上约占决策点四成，因 p̃ 的**聚合** lot 量化与逐腿 `q_units` 取整口径
    /// 不同；这是真实分歧，不是缺陷，M2 按比例吸收并在此登记，M3/M4 须正面裁决。**M3 门控后
    /// 同一 fixture 降至 2/3000**——频率降而根因未除，相交点分析见模块头 §M3）。
    pub n_rescaled: u64,
    /// `max |Σ_ℓ basis^gated_ℓ − T_lee|`（**L1 经验读数**）：级别结构目标与账户层投影后物理
    /// 目标的**分歧幅度**。纯观测，**不断言其为 0**——它不为 0 正是「结构说的仓位 ≠ 账户能下
    /// 的仓位」的量化。
    ///
    /// **等级订正**：M2 文案标 L2，但读数产自合成 `random_walk_dataset` ⟹ 按
    /// `formalization-validity-domain` 表应为 **L1**（L2 需真实数据）。「L2」在 M2 文案中被
    /// 当作「经验/可证伪」用，与该规则的等级轴混用了；本文件新增读数一律标 L1 并说明可证伪性。
    pub max_abs_struct_gap: i64,
    /// ★M3 `max |Σ_ℓ q_ℓ^plan − p_t|`（**L1 经验读数**，可证伪）：结构计划态与已成交态的
    /// **未兑现缺口**，同时也是订单物理量与 `Σ_ℓ Δq_ℓ` 的差额上界。
    ///
    /// 来源：拒单 / 部分成交 / `close_only` 上限。M3 锚计划态 ⟹ 不自动补单（模块头「为什么
    /// 不锚 `p_t`」），故此缺口是真实欠达，**只登记不断言为 0**（断言它为 0 等于把 M3 的
    /// 已知代价粉饰掉）。
    pub max_abs_plan_fill_gap: i64,
    /// ★M3 因 clock_ℓ 无 tick 而**保前值**（未重估）的 `(决策点, 级别)` 计数。
    ///
    /// **非平凡性证据**：>0 才说明门控真的挡下过重估；恒 0 = 门控空转（每 bar 全级别重估
    /// = 退化回 M2/M0 的 bar tick）。
    pub n_levels_held_by_clock: u64,
    /// ★M3 因 clock_ℓ 有 tick 而重估的 `(决策点, 级别)` 计数（与上一条成对读，给出门控率）。
    pub n_levels_regated: u64,
    /// ★M3 `Σ_ℓ Δq_ℓ ≠ 0` 但本决策点**无任何结构钟点**的决策点数（稀疏性的**反例计数**）。
    ///
    /// 由调用方在已知本 bar 结构钟点为空时登记。这些即「订单时点 ⊄ 结构事件时点并集」的
    /// 反例，其成因只应是风控门 / `𝒦_Θ` 帽 binding（§F③ 域分离下的合法例外）——**逐个计数
    /// 并与 `n_orders_off_structural_clock_risk_explained` 对账**，未被解释的差额即真实违例。
    pub n_orders_off_structural_clock: u64,
    /// ★M3 上一条中**能由风控/帽 binding 解释**的决策点数。二者相等 ⟹ 稀疏性无未解释违例。
    pub n_orders_off_structural_clock_risk_explained: u64,
    /// ★M3 **逐级**稀疏性反例：本决策点无 tick 却 `Δq_ℓ ≠ 0` 的 `(决策点, 级别)` 计数。
    ///
    /// 设计文档不变量的**逐级**形式是「bar i 无 ℓ 级事件 ⟹ `Δq_ℓ(i)=0`」，比 bar 级形式
    /// （「无任何级别事件 ⟹ 不发单」）**严格更强**。二者不等价的唯一路径是
    /// [`attribute_total`] 的**比例缩放**：`T_lee ≠ Σ basis^gated` 时它按比例改写**所有**级别
    /// （含无 tick 的），此时无 tick 级别也会有非零 `Δq_ℓ`。
    ///
    /// 这不是缺陷而是 §F③ 域分离的必然：缩放只由账户层帽/风控 binding 触发（结构域不会让
    /// `Σ basis^gated` 与 `T_lee` 分离），属 `CapTick`/`RiskTick` 例外。**但必须逐级计数**，
    /// 否则「逐级稀疏性」只是没被测到，而不是成立——与
    /// [`Self::n_levels_off_clock_delta_unexplained`] 配对读。
    pub n_levels_off_clock_delta: u64,
    /// ★M3 上一条中**无法**由缩放（帽/风控 binding）解释的 `(决策点, 级别)` 计数——真实违例。
    ///
    /// 恒 0 才说明逐级稀疏性成立。非 0 = 存在「无 tick、无缩放，却动了该级目标」的路径。
    pub n_levels_off_clock_delta_unexplained: u64,
    /// ★M3 风控门**非全开**（`force_flat || stop_long || stop_short`）的决策点数。
    ///
    /// 「风控门每 bar 生效」的**求值面**证据：门在每个决策点被求值并无条件施加到目标投影
    /// （`plan_level_gated_order` 第②段不读 clock ticks），本计数 >0 说明它在本跑批上真的
    /// 收窄过可行集——而不是「门在，但从未 binding」的平凡通过。
    pub n_risk_gate_active: u64,
    /// ★M3 上一条中**同时产生了订单**的决策点数（风控收窄真的落到订单上，非只改可行集）。
    pub n_risk_gate_active_with_order: u64,
}

impl LevelOrderStats {
    /// 归因完备见证成立：`Σ_ℓ held_ℓ ≡ p_t`（**L1 可证伪**）**且** 两条量级非平凡（>0）。
    ///
    /// 单看「残差 0」不构成证据——空账下残差平凡为 0（#289 MED 指出的正是这一类平凡通过）。
    ///
    /// **M3 变更**：不再合取 `max_abs_order_residual == 0`。M2 时它是 L0 构造护栏（对本谓词
    /// 本就零信息增量，见 M2 版注释）；M3 起订单流与 M0 分叉 ⟹ 它是 L2 分叉幅度，合取它
    /// 等于要求「M3 没有分叉」，与 M3 契约直接矛盾（设计文档 §D M3）。本谓词因此收敛为
    /// 它唯一真正见证过的东西：**跨延迟/部分/拒单的实际成交归因完备**。
    pub fn identity_witnessed(&self) -> bool {
        self.max_abs_held_residual == 0 && self.max_abs_order_units > 0 && self.max_abs_net_units > 0
    }

    /// ★M3 门控见证成立：门控真的挡下过重估（`n_levels_held_by_clock > 0`）**且**没有把
    /// 全部重估都挡掉（`n_levels_regated > 0`）。
    ///
    /// 双侧要求同 [`super::level_clock::LevelClockStats::sparsity_witnessed`]：全挡=账本冻死，
    /// 全不挡=门控空转（退化回 bar tick）。两者都不是 M3。
    pub fn gating_witnessed(&self) -> bool {
        self.n_levels_held_by_clock > 0 && self.n_levels_regated > 0
    }

    /// ★M3 **bar 级**稀疏性无未解释违例：无结构钟点却产订单的决策点，全部能由风控/帽解释。
    ///
    /// §F③ 域分离的可证伪形式：`{order≠0} ⊆ ∪clock^struct ∪ RiskTick ∪ CapTick`。
    pub fn sparsity_has_no_unexplained_violation(&self) -> bool {
        self.n_orders_off_structural_clock == self.n_orders_off_structural_clock_risk_explained
    }

    /// ★M3 **逐级**稀疏性无未解释违例（比 bar 级严格更强）：无 tick 的级别若 `Δq_ℓ ≠ 0`，
    /// 必须全部由 [`attribute_total`] 的帽/风控缩放解释。
    ///
    /// 对应设计文档不变量逐字「bar i 无 ℓ 级事件 ⟹ `Δq_ℓ(i)=0`」与「级别封闭：`Ledger_ℓ` 的
    /// 持仓只由 `formation_level=ℓ` 的事件改变」。
    pub fn per_level_sparsity_has_no_unexplained_violation(&self) -> bool {
        self.n_levels_off_clock_delta_unexplained == 0
    }
}

/// 单决策点的级别归因计划（`Σ_ℓ Δq_ℓ` 的生成物）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelOrderPlan {
    /// 各级物理目标 `q_ℓ`（`Σ ≡ target_total`）。
    pub targets: LevelUnits,
    /// 各级增量 `Δq_ℓ = q_ℓ − held_ℓ`（`Σ ≡ order_units`；含 `Δq_ℓ=0` 的级别，级别封闭可读）。
    pub deltas: LevelUnits,
    /// `Σ_ℓ Δq_ℓ`（**订单量的唯一来源**；i64 精确求和，无浮点重排）。
    pub order_units: i64,
    /// 本计划是否动用了 [`LEVEL_ACCOUNT_RESIDUAL`] 桶。
    pub used_residual_bucket: bool,
    /// 本计划是否经比例缩放（`Σ_ℓ basis_ℓ ≠ 物理目标`）。
    pub rescaled: bool,
    /// `Σ_ℓ net_ℓ − T`（结构净额与账户层物理目标的**有符号分歧**；L2 经验读数，见
    /// [`LevelOrderStats::max_abs_struct_gap`]）。
    pub struct_gap: i64,
}

impl LevelOrderPlan {
    /// 由 `Σ_ℓ Δq_ℓ` 生成物理订单：`qty = |Σ_ℓ Δq_ℓ|`，动作分类沿用净额 `Schedule_Θ`
    /// （设计文档 §C.2「沿用 Schedule_Θ 单出口」）。
    ///
    /// `Σ_ℓ Δq_ℓ == 0` ⟹ 动作降级为 `Hold`/`Wait`（无量可下；与 [`super::coverage::schedule_order`]
    /// 的 `qty==0` 分支同语义——持仓非空 `Hold`、空仓 `Wait`）。
    pub fn into_order(&self, action: StrictAction, holding: bool, exec_index: usize) -> Order {
        let qty = self.order_units.abs();
        if qty == 0 {
            let action = if holding { StrictAction::Hold } else { StrictAction::Wait };
            return Order { action, qty: 0, exec_index };
        }
        Order { action, qty, exec_index }
    }
}

/// [`LevelOrderLedger::observe_decision`] 的单决策点观测入参。
///
/// 打包而非平铺六个位置参数：其中三个是同型 `bool`（相邻裸 bool 传参错位不可察，且三者
/// 语义相近——「有结构钟点」「风控或帽 active」「仅风控 active」），六项恒同进同出。字段名
/// 在调用点即文档。
pub struct DecisionObs<'a> {
    /// 净额账本当前持仓（整数手）。
    pub p_t: i64,
    /// 账户层净额 `Schedule_Θ` 的量——M3 起用于量化**分叉幅度**（`max_abs_order_residual`，
    /// **L1 经验读数**，不再是恒 0 护栏；见模块头「等级迁移」段）。
    pub qty_m0: i64,
    /// 本决策点是否有任何**结构**钟点（[`super::level_clock::LevelClockTicks::has_structural`]）。
    pub has_structural_tick: bool,
    /// 风控门非全开、**或** `𝒦_Θ` 帽 binding（§F③ 域分离下「订单溢出结构事件集」的合法解释项）。
    pub risk_or_cap_active: bool,
    /// **仅**风控门非全开（不含帽）——「风控门每 bar 生效」的求值面证据；与上一项分开数
    /// 才能区分「风控收窄」与「帽收窄」。
    pub risk_gate_active: bool,
    /// 本决策点有 tick 的级别集（**逐级**稀疏性核对用，见
    /// [`LevelOrderStats::n_levels_off_clock_delta`]）。
    pub ticked: &'a BTreeSet<u32>,
}

/// ★M2/M3 级别归因台账：持有**两个**级别态——已成交归因 `held_ℓ`（`Σ ≡ p_t`）与结构计划
/// `q_ℓ^plan`（clock_ℓ 门控重估），逐决策点产 [`LevelOrderPlan`]，逐成交把实际成交量按计划
/// 比例落账。两态的语义分工见模块头「★M3 事件门控」段。
#[derive(Debug, Clone, Default)]
pub struct LevelOrderLedger {
    /// 已成交的按级归因持仓（level 升序；`Σ ≡ 净额账本 units`）。
    held: LevelUnits,
    /// ★M3 结构计划态 `q_ℓ^plan`（level 升序；`Σ ≡ 上一决策点投影后的物理目标 T_prev`）。
    ///
    /// 只在两处变：[`Self::regate`]（clock_ℓ 有 tick 的级别写入本 bar `net_ℓ`）与
    /// [`Self::commit_planned`]（下单后写入投影后的实际目标）。无 tick 的级别在 `regate` 中
    /// **原样保留**——这就是「无事件 bar 目标=前值」的实装落点。
    planned: LevelUnits,
    stats: LevelOrderStats,
}

impl LevelOrderLedger {
    pub fn new() -> Self {
        LevelOrderLedger::default()
    }

    /// 各级已成交归因持仓（只读；level 升序）。
    pub fn held(&self) -> &[(u32, i64)] {
        &self.held
    }

    /// `Σ_ℓ held_ℓ`（应恒 = 净额账本 `units`）。
    pub fn held_total(&self) -> i64 {
        self.held.iter().map(|&(_, q)| q).sum()
    }

    /// 见证读数（release 可见）。
    pub fn stats(&self) -> LevelOrderStats {
        self.stats
    }

    /// 各级结构计划态 `q_ℓ^plan`（只读；level 升序）。
    pub fn planned(&self) -> &[(u32, i64)] {
        &self.planned
    }

    /// `Σ_ℓ q_ℓ^plan` = 上一决策点投影后的物理目标 `T_prev`，也是 M3 增量的锚。
    pub fn planned_total(&self) -> i64 {
        self.planned.iter().map(|&(_, q)| q).sum()
    }

    /// ★★M3 门控重估（设计文档 §D M3「只在 clock_ℓ 事件时点重估目标；无事件 bar 目标=前值」）。
    ///
    /// 产**本决策点的结构基准** `basis^gated`（**不**改 `self.planned`——后者是上一决策点已提交
    /// 的目标，是增量的锚；提前覆写会把首次开仓的增量自消为 0）。对每个级别 ℓ：
    ///
    /// - `ticked` 含 ℓ ⟹ `basis^gated_ℓ := net_ℓ`（本 bar 结构净额；`basis` 中缺席视为 0——
    ///   该级腿已全部离场，目标归零，这是「灭」侧的正确落账，不是丢失）；
    /// - `ticked` 不含 ℓ ⟹ `basis^gated_ℓ := q_ℓ^plan`（**前值**；`basis` 里它这一 bar 的值被
    ///   **丢弃**——`net_ℓ` 的逐 bar 漂移主因是 `base_units = NAV/px` 而非结构事件，见
    ///   [`super::level_clock`] 模块头对齐表末行；门控切断的正是这条非结构通道）。
    ///
    /// 值域覆盖 `planned ∪ basis` 的级别之并，按 level 升序，零项**保留**（级别封闭可读，与
    /// [`LevelOrderPlan::deltas`] 同纪律）。逐 `(决策点, 级别)` 累计门控计数到 `stats`。
    pub fn regate(
        &mut self,
        basis: &[(u32, i64)],
        ticked: &BTreeSet<u32>,
    ) -> LevelUnits {
        // level 之并的骨架（值取 planned 侧前值；basis 独有级别前值为 0）。
        let skeleton = merge_levels(&self.planned, basis, |prev, _| prev);
        let mut gated: LevelUnits = Vec::with_capacity(skeleton.len());
        for &(lvl, prev) in &skeleton {
            if ticked.contains(&lvl) {
                let net = basis
                    .binary_search_by_key(&lvl, |&(l, _)| l)
                    .map(|i| basis[i].1)
                    .unwrap_or(0);
                gated.push((lvl, net));
                self.stats.n_levels_regated += 1;
            } else {
                gated.push((lvl, prev));
                self.stats.n_levels_held_by_clock += 1;
            }
        }
        gated
    }

    /// ★★M3 逐决策点计划：把**投影后**的物理目标 `target_total`（`T_lee` = `𝒦_Θ` 帽 + 风控门
    /// 施加后的整数手净目标，由调用方经 [`super::coverage::pi_theta_position`] 每 bar 无条件
    /// 算得）按门控基准 `gated_basis`（[`Self::regate`] 产）的结构比例分配到各级，产增量
    /// `Δq_ℓ = q_ℓ − q_ℓ^plan`。
    ///
    /// `Σ_ℓ Δq_ℓ = target_total − Σ_ℓ q_ℓ^plan = T_lee − T_prev`——**锚计划态**（M2 锚 `held`；
    /// 差异与理由见模块头「为什么增量锚在 `q_ℓ^plan`」）。因此：
    ///
    /// - 无 clock tick 且风控/帽未改目标 ⟹ `gated_basis == planned` ∧ `T_lee == T_prev`
    ///   ⟹ `attribute_total` 走恒等分支 ⟹ `Δq_ℓ ≡ 0`（**稀疏性是构造性的，不靠实测碰巧**）；
    /// - `force_flat` ⟹ `T_lee == 0` ⟹ 各级目标按比例归零 ⟹ 平仓单照出（**风控每 bar 生效**）。
    ///
    /// `struct_gap` 在 M3 语义下 = `Σ_ℓ basis^gated_ℓ − T_lee`（门控后的结构计划与投影后物理
    /// 目标的分歧，即帽/风控吃掉的部分）——仍是 L2 经验读数，不断言为 0。
    pub fn plan_gated(&self, gated_basis: &[(u32, i64)], target_total: i64) -> LevelOrderPlan {
        let (targets, used_residual_bucket, rescaled) = attribute_total(gated_basis, target_total);
        let deltas = sub_levels(&targets, &self.planned);
        let order_units = deltas.iter().map(|&(_, q)| q).sum();
        let struct_gap = gated_basis.iter().map(|&(_, q)| q).sum::<i64>() - target_total;
        LevelOrderPlan { targets, deltas, order_units, used_residual_bucket, rescaled, struct_gap }
    }

    /// ★M3 下单后把计划态推进到本决策点的实际目标（`q_ℓ^plan := q_ℓ`）。
    ///
    /// 必须在 [`Self::plan_gated`] 产出被真正用作订单**之后**调用——否则下一 bar 的
    /// 「目标=前值」会退回到旧值，门控退化为「每次都重估到同一个陈旧目标」。
    /// 零项保留（级别封闭可读）。
    pub fn commit_planned(&mut self, targets: &[(u32, i64)]) {
        self.planned = targets.to_vec();
    }

    /// 决策点见证登记（入参见 [`DecisionObs`]）。
    ///
    /// `has_structural_tick` 与 `risk_or_cap_active`/`risk_gate_active` **成对**喂入是稀疏性
    /// 验收可证伪的前提：只数「无结构钟点却产订单」而不数「其中能被风控解释的」，验收就
    /// 只能在「零风控」的窗口上做，覆盖面被悄悄缩掉。
    pub fn observe_decision(&mut self, plan: &LevelOrderPlan, obs: DecisionObs<'_>) {
        let DecisionObs {
            p_t,
            qty_m0,
            has_structural_tick,
            risk_or_cap_active,
            risk_gate_active,
            ticked,
        } = obs;
        let held_total = self.held_total();
        let planned_total = self.planned_total();
        // ★逐级稀疏性核对（比 bar 级严格更强）：无 tick 的级别若 Δq_ℓ≠0，只应源自
        //   `attribute_total` 的帽/风控比例缩放（`plan.rescaled`）；否则是真实违例。
        let mut off_clock_delta = 0u64;
        for &(lvl, d) in plan.deltas.iter() {
            if d != 0 && !ticked.contains(&lvl) {
                off_clock_delta += 1;
            }
        }
        let s = &mut self.stats;
        s.n_levels_off_clock_delta += off_clock_delta;
        if !plan.rescaled {
            s.n_levels_off_clock_delta_unexplained += off_clock_delta;
        }
        s.n_decisions += 1;
        if plan.order_units != 0 {
            s.n_orders_generated += 1;
            if !has_structural_tick {
                s.n_orders_off_structural_clock += 1;
                if risk_or_cap_active {
                    s.n_orders_off_structural_clock_risk_explained += 1;
                }
            }
        }
        if risk_gate_active {
            s.n_risk_gate_active += 1;
            if plan.order_units != 0 {
                s.n_risk_gate_active_with_order += 1;
            }
        }
        s.max_abs_order_units = s.max_abs_order_units.max(plan.order_units.abs());
        s.max_abs_order_residual =
            s.max_abs_order_residual.max((plan.order_units.abs() - qty_m0).abs());
        s.max_abs_held_residual = s.max_abs_held_residual.max((held_total - p_t).abs());
        s.max_abs_net_units = s.max_abs_net_units.max(p_t.abs());
        s.max_abs_plan_fill_gap = s.max_abs_plan_fill_gap.max((planned_total - p_t).abs());
        if plan.used_residual_bucket {
            s.n_residual_bucket += 1;
        }
        if plan.rescaled {
            s.n_rescaled += 1;
        }
        s.max_abs_struct_gap = s.max_abs_struct_gap.max(plan.struct_gap.abs());
    }

    /// ★逐成交落账：实际成交有符号手数 `executed_signed`（= `units_after − units_before`）按
    /// 该订单的计划增量 `attrib` 比例分配到 `held_ℓ`，`Σ_ℓ Δheld_ℓ ≡ executed_signed` 精确。
    ///
    /// 拒单/部分成交（[`super::super::backtest::fill::apply_fill`] 的现金约束与
    /// `close_only` 上限）⟹ `|executed_signed| < |Σ attrib|` ⟹ 同一 [`attribute_total`]
    /// 算子按计划比例回缩，归因不失配（`Σ_ℓ held_ℓ ≡ units` 因此逐 fill 保持）。
    pub fn on_fill(&mut self, attrib: &[(u32, i64)], executed_signed: i64) {
        if executed_signed == 0 {
            return;
        }
        let (applied, _, _) = attribute_total(attrib, executed_signed);
        self.held = add_levels(&self.held, &applied);
        self.held.retain(|&(_, q)| q != 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全级别 tick 的便捷集（M3 前语义的等价形态：每 bar 全级别重估）。
    fn all_ticked(basis: &[(u32, i64)], planned: &[(u32, i64)]) -> BTreeSet<u32> {
        basis.iter().chain(planned.iter()).map(|&(l, _)| l).collect()
    }

    /// ★台账 Σ_ℓ Δq_ℓ ≡ T_lee − Σ_ℓ q_ℓ^plan（M3 订单量恒等）：开→加→帽回缩→全平四步。
    ///
    /// 本例每步全级别 tick ⟹ 门控恒开 ⟹ 退化为 M2 的每 bar 重估形态，坐实「门控开时行为不塌」。
    #[test]
    fn ledger_order_units_equal_target_minus_planned() {
        let mut led = LevelOrderLedger::new();
        // t0：空账 → 目标 12（L1:10 + L2:6 + L3:−4）⟹ Δ = 12。
        let basis0 = vec![(1u32, 10i64), (2, 6), (3, -4)];
        let g0 = led.regate(&basis0, &all_ticked(&basis0, led.planned()));
        let p0 = led.plan_gated(&g0, 12);
        assert_eq!(p0.order_units, 12);
        assert_eq!(p0.targets, basis0);
        led.commit_planned(&p0.targets);
        led.on_fill(&p0.deltas, p0.order_units);
        assert_eq!(led.held_total(), 12, "Σ held ≡ 已成交净额");
        assert_eq!(led.planned_total(), 12, "Σ q^plan ≡ 已提交目标");
        // t1：目标 20（同结构比例放大）⟹ Δ = 8。
        let b1 = vec![(1u32, 16i64), (2, 8), (3, -4)];
        let g1 = led.regate(&b1, &all_ticked(&b1, led.planned()));
        let p1 = led.plan_gated(&g1, 20);
        assert_eq!(p1.order_units, 8);
        led.commit_planned(&p1.targets);
        led.on_fill(&p1.deltas, p1.order_units);
        assert_eq!(led.held_total(), 20);
        // t2：目标 5（帽 binding 缩放）⟹ Δ = −15。
        let g2 = led.regate(&b1, &all_ticked(&b1, led.planned()));
        let p2 = led.plan_gated(&g2, 5);
        assert_eq!(p2.order_units, -15);
        led.commit_planned(&p2.targets);
        led.on_fill(&p2.deltas, p2.order_units);
        assert_eq!(led.held_total(), 5);
        // t3：全平（空基准 + 目标 0）⟹ Δ = −5，两账清空。
        let g3 = led.regate(&[], &all_ticked(&[], led.planned()));
        let p3 = led.plan_gated(&g3, 0);
        assert_eq!(p3.order_units, -5);
        led.commit_planned(&p3.targets);
        led.on_fill(&p3.deltas, p3.order_units);
        assert_eq!(led.held_total(), 0);
        assert_eq!(led.planned_total(), 0);
        assert!(led.held().iter().all(|&(_, q)| q == 0), "全平后各级归因清零：{:?}", led.held());
    }

    /// ★★M3 稀疏性是**构造性**的（不是实测碰巧）：无 tick 的级别保前值 ⟹ `Δq_ℓ ≡ 0`。
    ///
    /// 关键场景 = `basis` 逐 bar 漂移（`base_units = NAV/px` 每 bar 变，见 `level_clock` 对齐表
    /// 末行）而结构无事件：门控丢弃漂移后的 `net_ℓ`，订单量恒 0。
    #[test]
    fn no_tick_bar_yields_zero_delta_even_when_basis_drifts() {
        let mut led = LevelOrderLedger::new();
        // t0：L1 有事件，建仓 10。
        let g0 = led.regate(&[(1, 10)], &[1u32].into_iter().collect());
        let p0 = led.plan_gated(&g0, 10);
        assert_eq!(p0.order_units, 10);
        led.commit_planned(&p0.targets);
        led.on_fill(&p0.deltas, 10);
        // t1..t3：**无任何 tick**，但 basis 逐 bar 漂移（NAV/价波动 ⟹ q_units 变）。
        for drifted in [11i64, 9, 13] {
            let g = led.regate(&[(1, drifted)], &BTreeSet::new());
            assert_eq!(g, vec![(1, 10)], "无 tick ⟹ 门控丢弃漂移后的 net_ℓ，取前值");
            // T_lee = Σ gated（无风控/帽介入）⟹ 恒等分支。
            let p = led.plan_gated(&g, g.iter().map(|&(_, q)| q).sum());
            assert_eq!(p.order_units, 0, "无事件 bar ⟹ Δq_ℓ ≡ 0（稀疏性构造成立）");
            assert!(p.deltas.iter().all(|&(_, d)| d == 0), "逐级增量全 0：{:?}", p.deltas);
            led.commit_planned(&p.targets);
        }
        // t4：L1 再次有事件 ⟹ 重估到当前 net_ℓ，产增量。
        let g4 = led.regate(&[(1, 13)], &[1u32].into_iter().collect());
        let p4 = led.plan_gated(&g4, 13);
        assert_eq!(p4.order_units, 3, "有事件 ⟹ 重估 10→13，Δ=3");
        let s = led.stats();
        assert!(s.n_levels_held_by_clock > 0 && s.n_levels_regated > 0);
    }

    /// ★★M3 风控每 bar 生效（§F③ 域分离）：**无任何 tick** 的 bar 上，`force_flat` 把投影后的
    /// `T_lee` 收窄到 0 ⟹ 各级目标按比例归零 ⟹ 平仓单照出。事件门控不门控风控。
    #[test]
    fn risk_flatten_fires_on_bar_with_no_clock_tick() {
        let mut led = LevelOrderLedger::new();
        let g0 = led.regate(&[(1, 6), (2, 4)], &[1u32, 2].into_iter().collect());
        let p0 = led.plan_gated(&g0, 10);
        led.commit_planned(&p0.targets);
        led.on_fill(&p0.deltas, 10);
        assert_eq!(led.planned_total(), 10);
        // 无 tick + force_flat（调用方经 pi_theta_position 得 T_lee=0）。
        let g1 = led.regate(&[(1, 6), (2, 4)], &BTreeSet::new());
        assert_eq!(g1, vec![(1, 6), (2, 4)], "结构计划保前值");
        let p1 = led.plan_gated(&g1, 0);
        assert_eq!(p1.order_units, -10, "force_flat ⟹ 平掉全部 10 手（无事件也照平）");
        assert_eq!(p1.targets, vec![(1, 0), (2, 0)], "各级目标按比例归零");
        assert_eq!(p1.struct_gap, 10, "结构计划(10) − 投影目标(0) = 10（帽/风控吃掉的部分）");
    }

    /// ★部分成交/拒单：实际成交 < 计划 ⟹ 归因按计划比例回缩，`Σ_ℓ held_ℓ ≡ 实际成交` 不失配；
    /// 计划态与成交态的缺口由 `max_abs_plan_fill_gap` **登记**（M3 不自动补单，见模块头）。
    #[test]
    fn ledger_partial_fill_keeps_held_sum_equal_to_executed_and_logs_gap() {
        let mut led = LevelOrderLedger::new();
        let g = led.regate(&[(1, 60), (2, 40)], &[1u32, 2].into_iter().collect());
        let plan = led.plan_gated(&g, 100);
        assert_eq!(plan.order_units, 100);
        led.commit_planned(&plan.targets);
        led.on_fill(&plan.deltas, 37); // 现金约束拒掉 63 手
        assert_eq!(led.held_total(), 37, "Σ held ≡ 实际成交（非计划量）");
        assert_eq!(led.held(), &[(1, 22), (2, 15)], "60:40 比例回缩到 37（22+15）");
        assert_eq!(led.planned_total(), 100, "计划态不因拒单回退");
        led.observe_decision(&plan, DecisionObs { p_t: 37, qty_m0: 100, has_structural_tick: true, risk_or_cap_active: false, risk_gate_active: false, ticked: &[1u32, 2].into_iter().collect() });
        assert_eq!(led.stats().max_abs_plan_fill_gap, 63, "未兑现缺口 100−37 如实登记");
    }

    /// ★见证读数非平凡（#289 MED 同款纪律）：归因残差 0 **且** 量级 > 0 才算见证成立；
    /// 空账（零决策）下 `identity_witnessed` 必须为 false（平凡通过不算证据）。
    #[test]
    fn stats_witness_requires_nontrivial_magnitude() {
        let empty = LevelOrderLedger::new();
        assert!(!empty.stats().identity_witnessed(), "零决策 ⟹ 见证不成立（平凡）");
        let mut led = LevelOrderLedger::new();
        let g = led.regate(&[(1, 10)], &[1u32].into_iter().collect());
        let plan = led.plan_gated(&g, 10);
        led.observe_decision(&plan, DecisionObs { p_t: 0, qty_m0: 10, has_structural_tick: true, risk_or_cap_active: false, risk_gate_active: false, ticked: &[1u32].into_iter().collect() });
        led.commit_planned(&plan.targets);
        led.on_fill(&plan.deltas, 10);
        let g2 = led.regate(&[(1, 10)], &BTreeSet::new());
        let plan2 = led.plan_gated(&g2, 10);
        led.observe_decision(&plan2, DecisionObs { p_t: 10, qty_m0: 0, has_structural_tick: false, risk_or_cap_active: false, risk_gate_active: false, ticked: &BTreeSet::new() });
        let s = led.stats();
        assert_eq!(s.max_abs_held_residual, 0, "归因完备残差恒 0（L1）");
        assert_eq!(s.max_abs_order_units, 10);
        assert_eq!(s.max_abs_net_units, 10);
        assert_eq!(s.n_decisions, 2);
        assert_eq!(s.n_orders_generated, 1, "第二步 Δ=0 ⟹ 不产订单");
        assert!(s.identity_witnessed(), "归因完备 + 量级 >0 ⟹ 见证成立");
        assert!(s.gating_witnessed(), "门控挡过 1 次、放行过 1 次 ⟹ 门控见证成立");
        assert_eq!(s.n_orders_off_structural_clock, 0, "第二步无订单 ⟹ 不计稀疏性反例");
        assert!(s.sparsity_has_no_unexplained_violation());
    }

    /// ★稀疏性反例的登记与解释配对：无结构钟点却产订单时，若风控/帽 active 则算已解释。
    #[test]
    fn off_clock_orders_are_counted_and_risk_explained() {
        let mut led = LevelOrderLedger::new();
        let g = led.regate(&[(1, 10)], &[1u32].into_iter().collect());
        let plan = led.plan_gated(&g, 10);
        led.commit_planned(&plan.targets);
        // 无结构钟点 + 风控 active ⟹ 反例被解释。
        let g2 = led.regate(&[(1, 10)], &BTreeSet::new());
        let flat = led.plan_gated(&g2, 0);
        assert_eq!(flat.order_units, -10);
        led.observe_decision(&flat, DecisionObs { p_t: 10, qty_m0: 10, has_structural_tick: false, risk_or_cap_active: true, risk_gate_active: true, ticked: &BTreeSet::new() });
        let s = led.stats();
        assert_eq!(s.n_orders_off_structural_clock, 1);
        assert_eq!(s.n_orders_off_structural_clock_risk_explained, 1);
        assert!(s.sparsity_has_no_unexplained_violation(), "全部反例可由风控解释");

        // 无结构钟点 + 风控不 active ⟹ 未解释违例。
        let mut bad = LevelOrderLedger::new();
        let gb = bad.regate(&[(1, 10)], &[1u32].into_iter().collect());
        let pb = bad.plan_gated(&gb, 10);
        bad.observe_decision(&pb, DecisionObs { p_t: 0, qty_m0: 10, has_structural_tick: false, risk_or_cap_active: false, risk_gate_active: false, ticked: &BTreeSet::new() });
        assert!(
            !bad.stats().sparsity_has_no_unexplained_violation(),
            "无钟点、无风控却产订单 = 真实违例，必须被抓到"
        );
    }

    /// ★订单构造：qty 唯一来源 = `|Σ_ℓ Δq_ℓ|`；Σ=0 ⟹ 动作降级 Hold/Wait（同净额 Schedule_Θ）。
    #[test]
    fn plan_into_order_qty_comes_from_level_delta_sum() {
        let mut led = LevelOrderLedger::new();
        let g = led.regate(&[(1, 7), (2, 3)], &[1u32, 2].into_iter().collect());
        let plan = led.plan_gated(&g, 10);
        let o = plan.into_order(StrictAction::Buy, false, 5);
        assert_eq!(o.qty, 10, "qty = |Σ_ℓ Δq_ℓ|");
        assert_eq!(o.action, StrictAction::Buy);
        assert_eq!(o.exec_index, 5);
        let zero = led.plan_gated(&[], 0);
        assert_eq!(zero.into_order(StrictAction::Buy, true, 5).action, StrictAction::Hold);
        assert_eq!(zero.into_order(StrictAction::Buy, false, 5).action, StrictAction::Wait);
        assert_eq!(zero.into_order(StrictAction::Buy, true, 5).qty, 0);
    }
}
