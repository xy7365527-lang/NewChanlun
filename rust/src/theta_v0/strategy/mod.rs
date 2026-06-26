//! Θ_voice + Θ_risk + Θ_exec 子模块（reference-theta-v0.md:39-54）。
//!
//! ## 范围（bit-exact 对齐 `Strict/Fugue.lean` / `RiskProj.lean` / `Op.lean` /
//! `StrategyFamily.lean`）
//!
//! 声部树 + 风险投影 + sizing + 执行。给定 Θ ⟹ 订单 O_{t+1} 唯一（StrategyFamily 元定理）。
//!
//! ## 子模块拓扑（对齐 StrategyFamily.lean `piTheta = exec ∘ riskProj ∘ target ∘ recog`）
//!
//! - [`voice`]（Θ_voice，对齐 `Fugue.lean`）：声部树 σ=(-1)^depth + 4 互斥动作态 + 深度权重。
//! - [`risk`]（Θ_risk，对齐 `RiskProj.lean`）：结构止损 + sizing 三路 min（唯一总仓位）。
//! - [`exec`]（Θ_exec）：延迟成交 + 费用 + 止损成交 + 不可交易过滤 + 冲突排序。
//!
//! [`pi_strict`] 是 strategy 的可验证种子（bit-exact 对齐 `Op.lean` `piStrict`，9 状态→7 动作）。
//! [`plan_orders`] 实装 π_Θ 链的 `target → riskProj → exec` 段（声部决策 → 风险投影 → 执行
//! ⟹ 唯一订单流）；`recog` 段（[`recognize`]）阻塞于 `bsp` 索引语义 change request。
//!
//! ## piTheta 链的两层（StrategyFamily.lean `recog`/`target`/`riskProj`/`exec`）
//!
//! - **recog**（`Classification + bars → 声部决策 D`）：从分类标签 + 历史读出每声部的决策意图
//!   （级别 L*、方向 σ、进场/退出判定、止损/入场价）。recog 的**价格桥接**（买卖点 bit-vector
//!   索引 → pivot/entry 价）依赖 cc-classifier 冻结的 `bsp` 索引语义——见 [`recognize`]。
//! - **target → riskProj → exec**（`声部决策 + 账户 → 订单`）：[`plan_orders`] 实装此确定链，
//!   bit-exact 对齐 Fugue/RiskProj/Θ_exec。此层**不依赖** `bsp` 索引语义（吃已 recog 的决策）。

pub mod exec;
pub mod intent;
pub mod ledger;
pub mod risk;
pub mod voice;

use super::classifier::{self, Classification};
use super::config::ThetaConfig;
use super::types::{Bar, BspBits, Order, Pos, Sig, StrictAction, Tick};
use exec::FillSide;
use risk::{SizingInput, StopInput, StopSide};
use voice::{ActState, VoiceSide, VoiceState};

/// 完整结构状态 Sₗ（Strict/Op.lean `StrictState`：持仓 × 信号 = 9 状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictState {
    pub pos: Pos,
    pub sig: Sig,
}

/// 完全应对策略 π（**bit-exact 对齐 `Strict/Op.lean` `piStrict`**）。
///
/// 9 状态（3 持仓 × 3 信号）→ 7 动作全函数。每一格的映射逐字对齐 Op.lean：
/// - flat+buySide → Buy（建仓）
/// - flat+sellSide → Wait（空仓遇卖侧，裸空非缠论 §4.4，观望不动）
/// - flat+none → Wait（空仓无信号 = 等待，**非 Hold**，Op.lean codex#2 关键修正）
/// - long+buySide → Add（降成本买回）
/// - long+sellSide → Reduce（降成本减仓）
/// - long+none → Hold（持多不动，有仓位）
/// - short+buySide → Close（平空头腿）
/// - short+sellSide → Sell（空头翻转/止损）
/// - short+none → Hold（持空不动，有仓位）
///
/// 全函数：match 穷尽 9 格（Rust 编译器静态保证穷尽性，对齐 Lean total 证明义务）。
/// 边界条件：此映射只依赖 `(pos,sig)`，不依赖 config——若 Θ 引入新动作态/状态会翻转
/// （须 change request，不在 Θ v0 内）。
pub fn pi_strict(s: StrictState) -> StrictAction {
    match (s.pos, s.sig) {
        (Pos::Flat, Sig::BuySide) => StrictAction::Buy,
        (Pos::Flat, Sig::SellSide) => StrictAction::Wait,
        (Pos::Flat, Sig::None) => StrictAction::Wait,
        (Pos::Long, Sig::BuySide) => StrictAction::Add,
        (Pos::Long, Sig::SellSide) => StrictAction::Reduce,
        (Pos::Long, Sig::None) => StrictAction::Hold,
        (Pos::Short, Sig::BuySide) => StrictAction::Close,
        (Pos::Short, Sig::SellSide) => StrictAction::Sell,
        (Pos::Short, Sig::None) => StrictAction::Hold,
    }
}

/// 账户状态 Z（StrategyFamily.lean `Theta` 的账户输入 Z）。
///
/// strategy-owned（不进共享 types.rs——账户状态是 strategy 层的运行时输入，非 parse/classify
/// 的结构对象）。字段：
/// - `nav`：账户净值 NAV（sizing 的 ρ·NAV / γ·NAV 基数）。
/// - `voice_qty`：各声部当前持仓手数（按深度索引，0=根 L*；空 = 该深度空仓）。
///
/// ★诚实：NAV/持仓是账户层运行时状态，不由缠论或 Θ 推导——本结构作为给定 Z 承载
/// （对齐 piTheta 的 `z : Z` 参数）。
#[derive(Debug, Clone, PartialEq)]
pub struct AccountState {
    pub nav: f64,
    pub voice_qty: Vec<u32>,
}

impl AccountState {
    /// 取深度 `depth` 声部的当前持仓手数（越界 = 该深度空仓，返回 0）。
    pub fn qty_at(&self, depth: u32) -> u32 {
        self.voice_qty.get(depth as usize).copied().unwrap_or(0)
    }
}

/// 单声部决策 D（StrategyFamily.lean `recog : H → Z → D` 的输出）。
///
/// 这是 recog 从分类标签 + 历史读出的**每声部决策意图**——target/riskProj/exec 链消费它
/// 产出订单。字段（声部级，对齐 Fugue.lean `VoiceState` + Θ_risk/Θ_exec 输入）：
/// - `depth`：声部深度（根=0）。决定资金权重；与 `root_side` 一起定声部绝对方向。
/// - `root_side`：**根方向 σ_root**（信号方向：买点→Long，卖点→Short）。声部绝对方向
///   = `voice::voice_side(root_side, depth)` = σ_root·(-1)^depth（spec:41 + Fugue 推广）。
/// - `exit`/`enter_ok`：退出/进场判定（Fugue.lean `Exit`/`EnterOK`，运行时 Bool）。
/// - `bsp`：该声部触发点的买卖点 bit-vector（决定止损类别 + bsp_class 冲突序）。
/// - `signal_index`：信号确认的 bar 索引（exec 延迟成交起点）。
/// - `stop_in`：结构止损输入（pivot 极值 + 最后中枢，`risk::structural_stop` 用）。
/// - `entry`：入场参考价（sizing 分母 + exec 成交基准，整数 tick）。
/// - `cost_per_unit`：每单位成本（sizing 的 κ·cost 项）。
/// - `level`：决策级别（冲突排序 `ConflictKey` 的 level，高 level 先）。
///
/// ★`root_side`（声部方向由信号定）：reference spec:41「σ=+1多/-1空」未固定根方向——3买/
/// 底背驰 → Long 根，3卖/顶背驰 → Short 根。这把 Fugue.lean「根恒 Long」推广为「根方向参数化」
/// （`voice::voice_side` 是 `dir_of_depth` 的有效域推广，Fugue 是 root_side=Long 特例）。v0
/// recognize 产单声部（depth 0，无嵌套对冲），σ = root_side = 信号方向。
///
/// ★诚实：`bsp`/`stop_in`/`entry`/`signal_index`/`root_side` 由 recog 从 classifier 的
/// `BspPoint`（路 B，single source）+ bars 读出。本结构作为 recog **输出**承载，
/// target→exec 链消费它。
#[derive(Debug, Clone, Copy)]
pub struct VoiceDecision {
    pub depth: u32,
    pub root_side: VoiceSide,
    pub exit: bool,
    pub enter_ok: bool,
    pub bsp: BspBits,
    pub signal_index: usize,
    pub stop_in: StopInput,
    pub entry: Tick,
    pub cost_per_unit: f64,
    pub level: u32,
}

/// 把声部方向映射到平仓/开仓的成交方向（Θ_exec `FillSide`）。
///
/// 开多（Long）= 买入；开空（Short）= 卖出。平仓方向相反（平多=卖，平空=买）——
/// 由 `is_exit` 决定是否取反。Flat 无方向（不应到达此函数，调用方保证非 Flat）。
fn fill_side_of(side: VoiceSide, is_exit: bool) -> Option<FillSide> {
    let base = match side {
        VoiceSide::Long => FillSide::Buy,
        VoiceSide::Short => FillSide::Sell,
        VoiceSide::Flat => return None,
    };
    Some(if is_exit {
        // 平仓方向相反（平多=卖，平空=买）。
        match base {
            FillSide::Buy => FillSide::Sell,
            FillSide::Sell => FillSide::Buy,
        }
    } else {
        base
    })
}

/// 取声部触发点的 1/2/3 类号（冲突排序用，reference-theta-v0.md:54）。
///
/// **方向感知**（与 `risk::structural_stop` 一致）：声部方向 `side` 由 depth 唯一确定
/// （`dir_of_depth`），Long 只看买点位（buy1/2/3），Short 只看卖点位（sell1/2/3）——
/// 一个声部不会混看反方向买卖点。按 BSP bit-vector 取**最小**成立的类号（1 类先于 2 类
/// 先于 3 类，spec:54 同 level 顺序）。买卖点非互斥（BspBits 可多位），同一点持多类时取
/// 最先执行的类（最小类号）。无对应方向任何类 / Flat ⟹ 返回 `u8::MAX`（排最后，非交易点）。
fn min_bsp_class(bits: &BspBits, side: VoiceSide) -> u8 {
    let (c1, c2, c3) = match side {
        VoiceSide::Long => (bits.buy1, bits.buy2, bits.buy3),
        VoiceSide::Short => (bits.sell1, bits.sell2, bits.sell3),
        VoiceSide::Flat => return u8::MAX,
    };
    if c1 {
        1
    } else if c2 {
        2
    } else if c3 {
        3
    } else {
        u8::MAX
    }
}

/// Θ_voice + Θ_risk + Θ_exec 顶层管线——实装 StrategyFamily.lean `piTheta` 链的
/// **`target → riskProj → exec` 段**（`recog` 段见 [`recognize`]，阻塞于 change request）。
///
/// 给定**已 recog 的声部决策列表** `decisions`（= recog 输出 D，本函数不含 recog 步骤）+
/// 历史 `bars`（H）+ 账户 `account`（Z）+ config（Θ），产出**确定且唯一**的订单流 O_{t+1}
/// ——bit-exact 对齐 Fugue（4 动作态）+ RiskProj（sizing 唯一总仓位）+ Θ_exec（成交/排序）。
///
/// ★诚实：本函数**不是**完整 piTheta（缺 recog 段）——它从 D（声部决策）开始，
/// 等价于 `piTheta` 的后三段 `exec ∘ riskProj.project ∘ target`。`recog`（H→D）的价格桥接
/// 阻塞于 `bsp` 索引语义裁定（见 [`recognize`]）。此分层使后三段可独立 bit-exact 验证。
///
/// ## 管线（每声部）
///
/// 1. **target（Fugue.lean `actState`/`targetPos`）**：从声部状态算 4 动作态 + 目标仓位。
///    close/wait → 无新仓订单（close 触发平仓订单）；open → sizing 建仓；hold → 不动。
/// 2. **riskProj（RiskProj.lean sizing）**：open 时 `risk::size_position` 算唯一 qty；
///    qty<=0 不交易（spec:47）。止损价由 `risk::structural_stop` 定。
/// 3. **exec（Θ_exec）**：`exec::fill_bar_index` 定延迟成交 bar；exec_index 写入订单。
///    不可交易 bar 过滤（无成交 bar ⟹ 该声部无订单）。
///
/// ## 冲突排序（reference-theta-v0.md:54）
///
/// 所有产出订单按 `exec::ConflictKey` 字典序排（止损/退出先于开仓；高 level 先；同 level
/// 1/2/3 类；平局 timestamp/source_index）——保证订单流**唯一确定**（StrategyFamily 元定理：
/// 给定 Θ ⟹ O_{t+1} 唯一）。
///
/// 边界条件：`decisions` 空 ⟹ 空订单流。某声部 Flat 方向 / 无止损 / 无成交 bar ⟹ 该声部
/// 跳过（不产出非法订单，对齐 spec「qty<=0 不交易 / 不可交易过滤」）。
pub fn plan_orders(
    decisions: &[VoiceDecision],
    bars: &[Bar],
    account: &AccountState,
    config: &ThetaConfig,
) -> Vec<Order> {
    let mut planned: Vec<(exec::ConflictKey, Order)> = Vec::new();

    for d in decisions {
        // 声部深度超出 max_depth ⟹ 不开声部（Fugue.lean 最多 max_depth 层，spec:40）。
        if !voice::within_max_depth(d.depth, &config.voice) {
            continue;
        }

        // 声部绝对方向 = 根方向（信号定）× depth 相对极性（`voice_side`，对齐 StrategyFamily
        // §5 `long_short_both_open_allowed:570` 多独立根）。根（depth 0）= root_side（信号方向，
        // 3买→Long/3卖→Short）；子声部按 depth 奇偶相对根翻转（赋格交替）。Fugue.lean
        // `dirOfDepth`（根恒 Long）是 root_side=Long + 嵌套树的强化子情形（不同有效域）。
        let side = voice::voice_side(d.root_side, d.depth);
        let q = account.qty_at(d.depth);

        // target：4 动作态（Fugue.lean actState）。
        let vstate = VoiceState {
            depth: d.depth,
            b: 0, // b_v（开仓基准）由 sizing 算出，target 阶段只需 act_state 判定
            q,
            exit: d.exit,
            enter_ok: d.enter_ok,
        };

        match voice::act_state(&vstate) {
            ActState::Close => {
                // 平仓订单：平掉当前 q 手（spec:54 退出先于开仓）。
                if q == 0 {
                    continue; // 无仓可平
                }
                if let Some(order) =
                    build_exit_order(d, side, q as i64, bars, config)
                {
                    let (ts, src) = signal_tie_keys(d, bars);
                    let key = exec::ConflictKey::new(
                        true,
                        d.level,
                        min_bsp_class(&d.bsp, side),
                        ts,
                        src,
                        d.depth,
                    );
                    planned.push((key, order));
                }
            }
            ActState::Open => {
                // 开仓订单：sizing 唯一 qty（riskProj）。
                if let Some(order) = build_open_order(d, side, account, bars, config) {
                    let (ts, src) = signal_tie_keys(d, bars);
                    let key = exec::ConflictKey::new(
                        false,
                        d.level,
                        min_bsp_class(&d.bsp, side),
                        ts,
                        src,
                        d.depth,
                    );
                    planned.push((key, order));
                }
            }
            // hold/wait：不产出订单（持仓不动 / 空仓观望）。
            ActState::Hold | ActState::Wait => {}
        }
    }

    // 冲突排序（spec:54 字典序）：唯一确定的订单流。
    planned.sort_by_key(|(key, _)| *key);
    planned.into_iter().map(|(_, order)| order).collect()
}

/// 构造开仓订单（target=open 分支）：sizing + 延迟成交 + 止损。
///
/// 返回 `None` 当：方向 Flat / 无结构止损 / qty<=0（不交易）/ 无可交易成交 bar。
fn build_open_order(
    d: &VoiceDecision,
    side: VoiceSide,
    account: &AccountState,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Option<Order> {
    let stop_side = match side {
        VoiceSide::Long => StopSide::Long,
        VoiceSide::Short => StopSide::Short,
        VoiceSide::Flat => return None,
    };
    // 结构止损价（risk::structural_stop）。
    let stop = risk::structural_stop(stop_side, &d.bsp, &d.stop_in)?;

    // 父子上限（spec:45 β）：根用 root_parent_cap，子用 parent_cap(父持仓)。
    let parent_cap = if d.depth == 0 {
        risk::root_parent_cap()
    } else {
        risk::parent_cap(account.qty_at(d.depth - 1) as i64, &config.risk)
    };

    // sizing（riskProj，唯一总仓位）。
    let sizing = SizingInput {
        nav: account.nav,
        entry: d.entry,
        stop,
        cost_per_unit: d.cost_per_unit,
        w_depth: voice::depth_weight(d.depth, &config.voice),
        parent_cap,
    };
    let qty = risk::size_position(&sizing, &config.risk);
    if qty <= 0 {
        return None; // qty<=0 不交易（spec:47）
    }

    // 延迟成交 bar（exec）。
    let exec_index = exec::fill_bar_index(d.signal_index, bars, &config.exec)?;

    // 开仓动作（StrictAction）：买入=Buy，卖出=Sell（声部方向）。
    let action = match side {
        VoiceSide::Long => StrictAction::Buy,
        VoiceSide::Short => StrictAction::Sell,
        VoiceSide::Flat => return None,
    };
    Some(Order { action, qty, exec_index })
}

/// 构造平仓订单（target=close 分支）：止损成交 + 延迟成交。
///
/// 平仓数量 = 当前持仓 q（全平）。成交 bar 经 `exec::fill_bar_index` 延迟。
/// 返回 `None` 当：方向 Flat / 无可交易成交 bar。
fn build_exit_order(
    d: &VoiceDecision,
    side: VoiceSide,
    qty: i64,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Option<Order> {
    let _ = fill_side_of(side, true)?; // 平仓方向校验（Flat ⟹ None）

    let exec_index = exec::fill_bar_index(d.signal_index, bars, &config.exec)?;
    // 平仓动作（Close）：StrictAction::Close（平当前腿，对齐 Op.lean）。
    Some(Order { action: StrictAction::Close, qty, exec_index })
}

/// 取声部冲突排序的 (timestamp, source_index) 平局键（reference-theta-v0.md:54 + :16）。
///
/// 从信号确认 bar（`signal_index`）取 `(timestamp, source_index)`——这是该订单**触发结构
/// 对象**的原始 bar 平局键（spec:16「所有平局按 (timestamp, source_index) 升序裁决」）。
///
/// 边界条件：`signal_index` 越界 ⟹ 用 `(0, signal_index)` 兜底（timestamp 缺省 0，
/// source_index 退化为 signal_index 本身——仍配合 `ConflictKey` 的 depth 终局键保证全序）。
fn signal_tie_keys(d: &VoiceDecision, bars: &[Bar]) -> (i64, usize) {
    match bars.get(d.signal_index) {
        Some(b) => (b.timestamp, b.source_index),
        None => (0, d.signal_index),
    }
}

/// recog 步骤（StrategyFamily.lean `recog : H → Z → D`）：`Classification + bars → 声部决策`。
///
/// 从 cc-classifier 的 `Classification`（分类标签 S，路 B `BspPoint` 携带结构止损价 single
/// source）+ `bars`（历史 H）读出每个买卖点的 [`VoiceDecision`]（决策 D）。**零结构重算**
/// （pivot/center 直接读 `BspPoint`，不从 bars 重算结构，对齐 Lead single-source 裁定）。
///
/// ## 映射（每个 `BspPoint` → 一个 `VoiceDecision`）
///
/// - **决策级别 L\***（spec:40 根=当前最高有效决策级别）：L\* = 含非空 bsp 的最高级别索引。
///   声部深度 `depth = L\* - ℓ`（L\* 级 bsp → depth 0 根；低级 bsp → 更深声部）。
///   超 `max_depth` 的深度由 [`plan_orders`] 的 `within_max_depth` 过滤（spec:40 最多 3 层）。
/// - **根方向 root_side**（spec:41 σ 由信号定）：bits 有买点位 → Long；有卖点位 → Short
///   （`bsp_root_side`）。声部绝对方向 = `voice::voice_side(root_side, depth)`。
/// - **stop_in**：从 `BspPoint` 的 `pivot_low`/`pivot_high`/`center` 直接构造 `StopInput`
///   （single source，零重算）。`center` 为 `None`（无 3 类）时用零 `Center` 占位（1/2 类
///   止损只用 pivot，不碰 center；3 类 bit ⟹ center 必 Some，classifier 不变量保证）。
/// - **entry**：信号确认后下一可交易 bar 的 open（spec:50 延迟成交基准价）——
///   `bars[fill_index].open`，`fill_index = exec::fill_bar_index(source_index)`。无成交 bar
///   ⟹ 该买卖点不产决策（信号作废，对齐 spec:50「无下一根」）。
/// - **enter_ok**：买卖点信号非空 = 进场许可（有 BspPoint 即有信号）。`exit` = false
///   （recognize 产**开仓侧**决策；平仓/止损由持仓状态 + 后续 bar 触发，属运行时循环，
///   不在单帧 recog 内——v0 recog 是信号→开仓决策，退出决策由账户层后续驱动）。
/// - **signal_index** = `BspPoint.source_index`（触发点原始 K 序，exec 延迟起点 + 平局键）。
/// - **level** = ℓ（决策级别，冲突排序高 level 先）。
///
/// ★诚实有效域（formalization-validity-domain）：v0 classifier 只在 L0 产第三类买卖点
/// （signal.rs 上级留空），故 recognize 实际产 L0 单声部决策（depth 0）。本函数写**通用**
/// 级别映射（L\* = 最高非空 bsp 级别），classifier 未来填充上级 bsp 时自然支持多层声部树，
/// 无需改本函数——v0 是其单层特例。认识论 L1（bit-exact 接线，不验证 Θ 市场有效）。
pub fn recognize(
    classification: &Classification,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Vec<VoiceDecision> {
    // 决策级别 L* = 含非空 bsp 的最高级别索引（spec:40 根=最高有效决策级别）。
    let l_star = match classification
        .levels
        .iter()
        .enumerate()
        .filter(|(_, ls)| !ls.bsp.is_empty())
        .map(|(idx, _)| idx)
        .max()
    {
        Some(l) => l,
        None => return Vec::new(), // 无任何买卖点 ⟹ 无决策（空订单流）
    };

    let mut decisions = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        // 声部深度 = L* - ℓ（L* 级 → depth 0 根；低级 → 更深）。L* ≥ level_idx（L* 是最高）。
        let depth = (l_star - level_idx) as u32;
        for point in &level.bsp {
            if let Some(d) = recognize_point(point, depth, level_idx as u32, bars, config) {
                decisions.push(d);
            }
        }
    }
    decisions
}

/// 单个 `BspPoint` → `VoiceDecision`（recog 的逐点映射，single source 零重算）。
///
/// 返回 `None` 当：bits 无任何买卖点位（非交易点）/ 无可交易成交 bar（信号作废，spec:50）。
fn recognize_point(
    point: &classifier::bsp::BspPoint,
    depth: u32,
    level: u32,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Option<VoiceDecision> {
    // 根方向 σ_root：bits 买侧 → Long，卖侧 → Short（spec:41 信号定方向）。
    // 做多/做空侧都接通——Short 根在 StrategyFamily §5 `long_short_both_open_allowed:570`
    // **已证允许**（独立根 side=false），故 σ 由信号定方向对齐 §5（非 workaround，无分叉）。
    // change request #2 已由 Lead 撤销（无真矛盾，是有效域辨识：v0 单声部独立根落 §5 多独立根，
    // 非 Fugue 嵌套树定义域）。`voice::voice_side(root_side, depth)` 兑现 §5 多独立根。
    let root_side = bsp_root_side(&point.bits)?; // None = 空 bits（非交易点）

    // entry = 信号确认后下一可交易 bar 的 open（spec:50 延迟成交基准）。
    let fill_index = exec::fill_bar_index(point.source_index, bars, &config.exec)?;
    let entry = bars[fill_index].open;

    // stop_in：从 BspPoint 直接构造（single source，零重算，对齐 cc-classifier 路 B 契约）。
    // ★显式校验 classifier 不变量（不静默吞，coding-style「不可交易/退化情况显式处理」）：
    // 含 3 类 bit（buy3/sell3）⟹ center 必 Some（is_third 蕴含 left_center，cc-classifier 保证）。
    // 若不变量被违反（含 3 类 bit 但 center=None），**显式返回 None 不产决策**（不用零 Center
    // 静默产 zg=0/zd=0 的错误止损价）。无 3 类 bit 时 center 不被 `structural_stop` 读取
    // （1/2 类只用 pivot），用零占位无害。
    let has_third = point.bits.buy3 || point.bits.sell3;
    let center = match point.center {
        Some(c) => c,
        None if has_third => return None, // 不变量违反：含 3 类 bit 但无 center（显式拒绝）
        None => super::types::Center {
            zd: 0,
            zg: 0,
            dd: 0,
            gg: 0,
            start_index: 0,
            end_index: 0,
        }, // 无 3 类 bit：center 不被读，零占位无害
    };
    let stop_in = StopInput {
        pivot_low: point.pivot_low,
        pivot_high: point.pivot_high,
        center,
    };

    Some(VoiceDecision {
        depth,
        root_side,
        exit: false, // recog 产开仓侧决策；退出由账户层后续 bar 驱动（见函数注释）
        enter_ok: true, // 有买卖点信号 = 进场许可
        bsp: point.bits,
        signal_index: point.source_index,
        stop_in,
        entry,
        // 成本 per-unit（sizing κ·cost 项的风险预算缓冲）：v0 占位 0.0（成交费用由
        // exec::apply_fees 在成交价上施加；每单位成本模型待 L3 标定，spec:45 κ 已 config）。
        cost_per_unit: 0.0,
        level,
    })
}

/// 从买卖点 bit-vector 判根方向（reference spec:41：买点→Long，卖点→Short）。
///
/// 买点位（buy1/2/3 任一）→ `Long`（底背驰/回试做多）；卖点位（sell1/2/3 任一）→ `Short`
/// （顶背驰/回抽做空）。一个 `BspPoint` 是单方向的（classifier signal.rs 按 `is_sell_side`
/// 置买或卖位，不混）——若混（买卖位同真，理论不应出现）则买侧优先（确定性裁决）。
/// 全空 bits ⟹ `None`（非交易点）。
fn bsp_root_side(bits: &BspBits) -> Option<VoiceSide> {
    if bits.buy1 || bits.buy2 || bits.buy3 {
        Some(VoiceSide::Long)
    } else if bits.sell1 || bits.sell2 || bits.sell3 {
        Some(VoiceSide::Short)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bit-exact 对齐 Op.lean：9 状态全枚举 → 7 动作映射逐格验证（golden table）。
    /// 这是 Op.lean `piStrict` 的 Rust conformance fixture——任一格漂移 = bit-exact 失败。
    #[test]
    fn pi_strict_matches_op_lean_nine_states() {
        use Pos::*;
        use Sig::*;
        use StrictAction::*;
        let cases = [
            (Flat, BuySide, Buy),
            (Flat, SellSide, Wait),
            (Flat, Sig::None, Wait),
            (Long, BuySide, Add),
            (Long, SellSide, Reduce),
            (Long, Sig::None, Hold),
            (Short, BuySide, Close),
            (Short, SellSide, Sell),
            (Short, Sig::None, Hold),
        ];
        for (pos, sig, expected) in cases {
            assert_eq!(pi_strict(StrictState { pos, sig }), expected);
        }
    }

    /// Op.lean wait/hold 区分关键不变量：flat 永不返回 Hold，long/short+none 永远 Hold。
    #[test]
    fn wait_only_for_flat_hold_only_for_held_positions() {
        use Pos::*;
        use Sig::*;
        // flat 三态都不是 Hold（空仓观望 = Wait）。
        for sig in [BuySide, SellSide, Sig::None] {
            assert_ne!(pi_strict(StrictState { pos: Flat, sig }), StrictAction::Hold);
        }
        // 持仓 + 无信号 = Hold。
        assert_eq!(
            pi_strict(StrictState { pos: Long, sig: Sig::None }),
            StrictAction::Hold
        );
        assert_eq!(
            pi_strict(StrictState { pos: Short, sig: Sig::None }),
            StrictAction::Hold
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  plan_orders 顶层管线（target → riskProj → exec）golden + 唯一性
    // ──────────────────────────────────────────────────────────────────────

    use super::super::types::{Bar, BspBits, Center};

    fn tradable_bar(idx: usize, ts: i64, o: Tick, h: Tick, l: Tick, c: Tick) -> Bar {
        Bar {
            source_index: idx,
            timestamp: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 100,
            untradable: false,
        }
    }

    fn mk_center(zd: Tick, zg: Tick, start: usize) -> Center {
        Center { zd, zg, dd: zd - 10, gg: zg + 10, start_index: start, end_index: start + 9 }
    }

    /// 根声部 1 买开仓决策（depth 0 = Long）。
    fn buy1_root_decision(signal_index: usize) -> VoiceDecision {
        VoiceDecision {
            depth: 0,
            root_side: VoiceSide::Long, // 1 买 → 做多根
            exit: false,
            enter_ok: true,
            bsp: BspBits { buy1: true, ..Default::default() },
            signal_index,
            stop_in: StopInput {
                pivot_low: 90,
                pivot_high: 210,
                center: mk_center(100, 200, 3),
            },
            entry: 100,
            cost_per_unit: 0.0,
            level: 5,
        }
    }

    /// golden：单根声部 1 买 open → 一条 Buy 订单，qty=sizing，exec_index=signal+1。
    #[test]
    fn plan_orders_single_buy_open_golden() {
        let cfg = ThetaConfig::default();
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 101, 111, 99, 108),
            tradable_bar(2, 2, 102, 112, 100, 109),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let decisions = vec![buy1_root_decision(0)];

        let orders = plan_orders(&decisions, &bars, &account, &cfg);
        assert_eq!(orders.len(), 1);
        let o = orders[0];
        assert_eq!(o.action, StrictAction::Buy);
        // sizing：entry=100, stop=90 (|d|=10), cost=0 ⟹ 项1=floor(0.005*1e6/10)=500；
        // 项2=floor(0.6*1.0*1e6/100)=6000；项3=MAX ⟹ qty=500。
        assert_eq!(o.qty, 500);
        // 延迟 1 根 ⟹ 成交在 signal_index+1 = 1。
        assert_eq!(o.exec_index, 1);
    }

    /// golden：close 决策 → 平仓订单（全平当前持仓，action=Close）。
    #[test]
    fn plan_orders_close_yields_exit_order() {
        let cfg = ThetaConfig::default();
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 95, 100, 88, 92),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![300] };
        let mut d = buy1_root_decision(0);
        d.exit = true; // 退出态 ⟹ close
        let orders = plan_orders(&[d], &bars, &account, &cfg);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].action, StrictAction::Close);
        assert_eq!(orders[0].qty, 300); // 全平当前持仓
        assert_eq!(orders[0].exec_index, 1);
    }

    /// hold/wait 不产出订单（持仓不动 / 空仓观望）。
    #[test]
    fn plan_orders_hold_wait_no_order() {
        let cfg = ThetaConfig::default();
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105), tradable_bar(1, 1, 101, 111, 99, 108)];
        // hold：q>0 + ¬exit + ¬enter_ok。
        let account_hold = AccountState { nav: 1_000_000.0, voice_qty: vec![300] };
        let mut d_hold = buy1_root_decision(0);
        d_hold.enter_ok = false;
        assert_eq!(plan_orders(&[d_hold], &bars, &account_hold, &cfg).len(), 0);
        // wait：q=0 + ¬exit + ¬enter_ok。
        let account_wait = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let mut d_wait = buy1_root_decision(0);
        d_wait.enter_ok = false;
        assert_eq!(plan_orders(&[d_wait], &bars, &account_wait, &cfg).len(), 0);
    }

    /// qty<=0 不交易（spec:47）：父空仓 ⟹ 子声部开仓 qty=0 ⟹ 无订单。
    #[test]
    fn plan_orders_child_zero_qty_no_trade() {
        let cfg = ThetaConfig::default();
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105), tradable_bar(1, 1, 101, 111, 99, 108)];
        // 子声部 depth 1（Short），父 depth 0 空仓 ⟹ parent_cap=0 ⟹ qty=0。
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0, 0] };
        let mut d = buy1_root_decision(0);
        d.depth = 1;
        // depth 1 = Short，须有卖点位才有止损。
        d.bsp = BspBits { sell1: true, ..Default::default() };
        let orders = plan_orders(&[d], &bars, &account, &cfg);
        assert_eq!(orders.len(), 0);
    }

    /// 不可交易过滤（spec:53）：成交 bar 全 untradable ⟹ 无订单。
    #[test]
    fn plan_orders_untradable_no_fill() {
        let cfg = ThetaConfig::default();
        let mut untradable = tradable_bar(1, 1, 101, 111, 99, 108);
        untradable.untradable = true;
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105), untradable];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        // 信号在 0，延迟落点 1 不可交易，无后续 ⟹ 无成交 bar ⟹ 无订单。
        let orders = plan_orders(&[buy1_root_decision(0)], &bars, &account, &cfg);
        assert_eq!(orders.len(), 0);
    }

    /// 超出 max_depth 的声部不开（spec:40：最多 max_depth 层）。
    #[test]
    fn plan_orders_beyond_max_depth_skipped() {
        let cfg = ThetaConfig::default(); // max_depth=3
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105), tradable_bar(1, 1, 101, 111, 99, 108)];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![100, 50, 25, 12] };
        let mut d = buy1_root_decision(0);
        d.depth = 3; // depth 3 >= max_depth 3 ⟹ 不开
        let orders = plan_orders(&[d], &bars, &account, &cfg);
        assert_eq!(orders.len(), 0);
    }

    /// 冲突排序（spec:54）：止损/退出先于开仓；高 level 先。订单流唯一确定（StrategyFamily
    /// 元定理：给定 Θ ⟹ O_{t+1} 唯一——输入决策顺序不影响输出顺序）。
    #[test]
    fn plan_orders_conflict_ordering_deterministic() {
        let cfg = ThetaConfig::default();
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 101, 111, 99, 108),
            tradable_bar(2, 2, 102, 112, 100, 109),
        ];
        // 根（depth 0）空仓 ⟹ 可 open；子（depth 1）持仓 80 ⟹ 可 close。
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0, 80] };

        // 开仓决策：根 depth 0 空仓 + enter_ok ⟹ Open（level 5）。
        let mut open_d = buy1_root_decision(0);
        open_d.enter_ok = true;
        open_d.exit = false;
        open_d.level = 5;
        // 退出决策：子 depth 1 持仓 + exit ⟹ Close（level 3）。
        let mut exit_d = buy1_root_decision(0);
        exit_d.depth = 1;
        exit_d.exit = true;
        exit_d.level = 3;

        // 两种输入顺序应产出相同排序（退出先于开仓，spec:54）。
        let a = plan_orders(&[open_d, exit_d], &bars, &account, &cfg);
        let b = plan_orders(&[exit_d, open_d], &bars, &account, &cfg);
        assert_eq!(a, b); // 顺序无关 ⟹ 订单流唯一
        assert_eq!(a.len(), 2);
        // 退出（Close）先于开仓（Buy）——exit_first 是第一键。
        assert_eq!(a[0].action, StrictAction::Close);
        assert_eq!(a[1].action, StrictAction::Buy);
    }

    /// F1 修复验证（订单唯一性，多声部共享触发结构）：两个不同深度声部共享同一中枢
    /// （同 level/class/timestamp/source_index），仅声部深度不同 ⟹ ConflictKey 仍全序，
    /// 订单流不依赖输入顺序（depth 终局键保证，对齐 given_theta_total_unique）。
    #[test]
    fn plan_orders_unique_when_voices_share_structure() {
        let cfg = ThetaConfig::default();
        let bars = vec![
            tradable_bar(0, 7, 100, 110, 90, 105),
            tradable_bar(1, 8, 101, 111, 99, 108),
            tradable_bar(2, 9, 102, 112, 100, 109),
        ];
        // 根（depth 0，Long）持仓 200，子（depth 1，Short）持仓 80——两者都退出（close），
        // 共享同一信号 bar（signal_index=0 ⟹ 同 timestamp/source_index）+ 同中枢 + 同 level/class。
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![200, 80] };
        let mut root = buy1_root_decision(0);
        root.exit = true;
        root.level = 4;
        let mut child = buy1_root_decision(0);
        child.exit = true;
        child.depth = 1;
        child.level = 4; // 同 level（人为构造前五键碰撞）
        child.bsp = BspBits { sell1: true, ..Default::default() }; // depth1=Short，类号同为 1

        let a = plan_orders(&[root, child], &bars, &account, &cfg);
        let b = plan_orders(&[child, root], &bars, &account, &cfg);
        // 输入顺序不影响输出（depth 终局键裁决）——订单流唯一。
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
        // 浅声部（depth 0）排在深声部（depth 1）前（depth 升序）。
        assert_eq!(a[0].qty, 200); // 根（depth 0）全平
        assert_eq!(a[1].qty, 80); // 子（depth 1）全平
    }

    /// property（订单唯一性）：plan_orders 是确定函数——同输入恒同输出（StrategyFamily
    /// `given_theta_total_unique`：给定 Θ + 决策 + 账户 ⟹ 订单唯一）。
    #[test]
    fn plan_orders_deterministic_idempotent() {
        let cfg = ThetaConfig::default();
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 101, 111, 99, 108),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let decisions = vec![buy1_root_decision(0)];
        let first = plan_orders(&decisions, &bars, &account, &cfg);
        let second = plan_orders(&decisions, &bars, &account, &cfg);
        assert_eq!(first, second);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  recognize（Classification → VoiceDecision）golden（recog 段，single source）
    // ──────────────────────────────────────────────────────────────────────

    use super::super::classifier::{Classification, LevelState};
    use super::super::classifier::bsp::BspPoint;

    /// 构造含一个第三类买点的单级 Classification（L0 = L*，single source BspPoint）。
    fn classification_with_buy3(source_index: usize) -> Classification {
        let bsp = vec![BspPoint {
            source_index,
            bits: BspBits { buy3: true, ..Default::default() },
            pivot_low: 210,
            pivot_high: 0,
            center: Some(mk_center(100, 200, 3)),
        }];
        Classification {
            levels: vec![LevelState { bsp, ..Default::default() }],
        }
    }

    /// recognize golden：buy3 BspPoint → VoiceDecision（root_side=Long，stop_in single source，
    /// entry=下一可交易 bar open，signal_index=source_index）。
    #[test]
    fn recognize_buy3_to_voice_decision() {
        let cfg = ThetaConfig::default();
        let classification = classification_with_buy3(0);
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210), // 成交 bar（signal+1）
        ];
        let decisions = recognize(&classification, &bars, &cfg);
        assert_eq!(decisions.len(), 1);
        let d = decisions[0];
        assert_eq!(d.root_side, VoiceSide::Long); // 买点 → 做多根
        assert_eq!(d.depth, 0); // L* = L0 ⟹ depth 0 根
        assert!(d.enter_ok && !d.exit);
        assert!(d.bsp.buy3);
        assert_eq!(d.signal_index, 0);
        // stop_in single source（零重算）：pivot_low/center 直接读 BspPoint。
        assert_eq!(d.stop_in.pivot_low, 210);
        assert_eq!(d.stop_in.center.zg, 200);
        // entry = 下一可交易 bar(1) 的 open。
        assert_eq!(d.entry, 205);
    }

    /// recognize → plan_orders 端到端：3 买信号 → 一条 Buy 订单（piTheta 全链 L1）。
    #[test]
    fn recognize_then_plan_orders_end_to_end() {
        let cfg = ThetaConfig::default();
        let classification = classification_with_buy3(0);
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
            tradable_bar(2, 2, 210, 220, 205, 215),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        // 完整 piTheta：recognize（recog）→ plan_orders（target→riskProj→exec）。
        let decisions = recognize(&classification, &bars, &cfg);
        let orders = plan_orders(&decisions, &bars, &account, &cfg);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].action, StrictAction::Buy); // 3 买 → 做多开仓
        assert!(orders[0].qty > 0);
        assert_eq!(orders[0].exec_index, 1); // 延迟 1 根成交
    }

    /// recognize 3 卖信号 → root_side=Short → Sell 订单（change request #2 撤销后接通做空侧）。
    /// 做空侧 Short 根对齐 StrategyFamily §5 `long_short_both_open_allowed:570`（独立根 side=false
    /// 已证允许，非 workaround，无 σ 分叉）。`voice_side(Short, 0)=Short`，spec:41 顶背驰做空。
    #[test]
    fn recognize_sell3_yields_short_sell_order() {
        let cfg = ThetaConfig::default();
        let bsp = vec![BspPoint {
            source_index: 0,
            bits: BspBits { sell3: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 90,
            center: Some(mk_center(100, 200, 3)),
        }];
        let classification = Classification {
            levels: vec![LevelState { bsp, ..Default::default() }],
        };
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 95, 100, 85, 90),
        ];
        // 做空侧接通：recog 产 Short 根决策（对齐 §5 多独立根）。
        let decisions = recognize(&classification, &bars, &cfg);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].root_side, VoiceSide::Short); // 卖点 → 做空根
        // 端到端：做空信号产 Sell 订单（不丢失）。
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let orders = plan_orders(&decisions, &bars, &account, &cfg);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].action, StrictAction::Sell);
        assert!(orders[0].qty > 0);
    }

    /// recognize 空 Classification → 空 decisions（无买卖点 ⟹ 无决策）。
    #[test]
    fn recognize_empty_classification_no_decisions() {
        let cfg = ThetaConfig::default();
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105)];
        assert!(recognize(&Classification::default(), &bars, &cfg).is_empty());
        // 有级别但 bsp 空 ⟹ 无决策。
        let empty_bsp = Classification {
            levels: vec![LevelState::default()],
        };
        assert!(recognize(&empty_bsp, &bars, &cfg).is_empty());
    }

    /// recognize 无可交易成交 bar → 该买卖点不产决策（信号作废，spec:50）。
    #[test]
    fn recognize_no_fill_bar_drops_signal() {
        let cfg = ThetaConfig::default();
        let classification = classification_with_buy3(0);
        // 信号在 index 0，但只有一根 bar（无下一根成交）⟹ 信号作废。
        let bars = vec![tradable_bar(0, 0, 100, 110, 90, 105)];
        assert!(recognize(&classification, &bars, &cfg).is_empty());
    }

    /// recognize 显式校验 classifier 不变量（不静默吞）：含 3 类 bit 但 center=None ⟹ 不产
    /// 决策（拒绝，不用零 Center 静默产 zg=0 错误止损）。这守卫 cc-classifier「含 3 类 bit ⟹
    /// center 必 Some」不变量在 strategy 侧的显式落实。
    #[test]
    fn recognize_third_bit_without_center_rejected() {
        let cfg = ThetaConfig::default();
        // 违反不变量构造：buy3=true 但 center=None（cc-classifier 保证不会发生，此处守卫）。
        let bsp = vec![BspPoint {
            source_index: 0,
            bits: BspBits { buy3: true, ..Default::default() },
            pivot_low: 210,
            pivot_high: 0,
            center: None, // 不变量违反
        }];
        let classification = Classification {
            levels: vec![LevelState { bsp, ..Default::default() }],
        };
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
        ];
        // 显式拒绝（不静默用零 Center 产 zg=0 止损）。
        assert!(recognize(&classification, &bars, &cfg).is_empty());
    }

    /// ★真实全链端到端（Lead 验证门）：`classify → recognize → plan_orders` 产非空订单。
    ///
    /// 用 cc-classifier #79 端到端 fixture（3 重叠线段 → 1 中枢 → 1 第三类买点，
    /// pivot_low=210, center.zg=200），喂**真实** `classifier::classify`（非手构 BspPoint），
    /// 验证整条 piTheta 链（C_Θ → recog → target → riskProj → exec）在真实分类输出上跑通
    /// 产非空订单流。这是 L2 关键路径的 strategy 侧端到端证据（n_orders>0 的前提）。
    #[test]
    fn real_classify_to_orders_end_to_end() {
        use super::super::parser::ParseLayer;
        use super::super::types::{Direction, Segment};

        let cfg = ThetaConfig::default();
        let seg = |dir, si, ei, sp, ep| Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        };
        // cc-classifier 端到端 fixture：三段在 [100,200] 重叠 ⟹ 中枢 zd=100,zg=200,end=12；
        // 段3 向上离开（端点 250>200）；段4 向下回试低点 210>=200 ⟹ 3 买 @ source_index=20。
        let l0 = ParseLayer {
            segments: vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 150, 250),
                seg(Direction::Down, 16, 20, 250, 210),
            ],
            ..Default::default()
        };

        // 真实 classify（非手构）：产非空 Classification + L0 第三类买点。
        let classification = classifier::classify(&l0, &cfg);
        assert!(!classification.levels.is_empty(), "classify 产非空");
        assert!(
            classification.levels[0].bsp.iter().any(|p| p.bits.buy3),
            "L0 含第三类买点（cc-classifier #79 端到端）"
        );

        // bars：第三类买点 @ source_index=20，需 index 21 作成交 bar（延迟 1 根，spec:50）。
        // 构造 22 根可交易 bar（index 0..=21），entry/成交在 21。
        let mut bars = Vec::new();
        for i in 0..22 {
            bars.push(tradable_bar(i, i as i64, 200, 220, 195, 210));
        }
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0, 0, 0] };

        // 完整 piTheta 链：classify → recognize → plan_orders。
        let decisions = recognize(&classification, &bars, &cfg);
        assert!(!decisions.is_empty(), "真实分类 → 非空声部决策");
        assert_eq!(decisions[0].root_side, VoiceSide::Long, "3 买 → 做多根");

        let orders = plan_orders(&decisions, &bars, &account, &cfg);
        assert!(!orders.is_empty(), "L2 关键路径：真实链产非空订单流（n_orders>0）");
        assert_eq!(orders[0].action, StrictAction::Buy, "3 买 → Buy 开仓");
        assert!(orders[0].qty > 0, "sizing 产正手数");
    }
}
