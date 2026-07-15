//! Θ_voice + Θ_risk + Θ_exec 子模块（reference-theta-v0.md:39-54）。
//!
//! ## 契约重锚（legacy Strict → `Origin.StrategyFamily` + `Origin.VoiceTree` + `Origin.RiskProj`）
//!
//! 声部树 + 风险投影 + sizing + 执行。**策略是 Param 索引的族 π_Θ**（不是硬编码单策略）——契约锚
//! `Origin.StrategyFamily`（StrategyFamily.lean，`namespace NewChanlun.Origin.StrategyFamily`）：
//! ```text
//! structure Theta (H Z D Target Pos K Order) where     -- 单个 Θ 实例（6+1 字段）
//!   prefixEq; recog : H→Z→D; target : D→Z→Target; riskProj : RiskGrid Pos K
//!   proj : Target→Pos; exec : D→Z→Pos→Order; recog_causal : ...
//! def piTheta (θ) (h) (z) : Order :=                    -- π_Θ = exec∘proj∘target∘recog
//!   θ.exec (θ.recog h z) z (θ.proj (θ.target (θ.recog h z) z))
//! structure StrategyFamily (H Z Param Order) where      -- ★Param 索引的策略族
//!   π : Param → H → Z → Order
//!   total_unique : ∀ θ h z, ExistsUnique (fun o => π θ h z = o)
//!   causal : ∀ θ z, Causal ...
//! def familyOfTheta : (Param → Theta ...) → StrategyFamily ...  -- Θ 族 → 策略族
//! ```
//! 关键元定理：`classification_does_not_choose_unique_policy`（分类**不**推出唯一策略——存在
//! π₁≠π₂）+ `given_theta_total_unique`（给定 Θ ⟹ π_Θ 全定义唯一）。本 Rust [`StrategyFamily`]
//! 把 [`ThetaConfig`] 立为 **Param 索引**：`StrategyFamily::pi(param, decisions, bars, account)` =
//! `piTheta` 在 param=config 处的求值——消硬编码单策略（不同 config ⟹ 不同 π_Θ 族成员）。
//!
//! ## 子模块拓扑（对齐 `Origin.StrategyFamily.piTheta = exec ∘ proj ∘ target ∘ recog`）
//!
//! - [`voice`]（Θ_voice，对齐 `Origin.VoiceTree`）：声部树 σ=flip(父σ) + 4 互斥动作态 + 深度权重。
//! - [`risk`]（Θ_risk，对齐 `Origin.RiskProj`）：结构止损 + sizing 三路 min（唯一总仓位）。
//! - [`exec`]（Θ_exec，对齐 `Origin.StrategyFamily.Theta.exec`）：延迟成交 + 费用 + 止损成交 + 冲突排序。
//!
//! [`pi_strict`] 是 strategy 的可验证种子（对齐 `Origin.FullDefinitionStrategy` 动作类 9→7 投影）。
//! [`plan_orders`] 实装 π_Θ 链的 `target → proj(riskProj) → exec` 段；[`StrategyFamily::pi`] 是
//! Param 索引的族入口；`recog` 段见 [`recognize`]（对齐 `Origin.StrategyFamily.Theta.recog`）。
//!
//! ## piTheta 链的两层（`Origin.StrategyFamily.Theta` `recog`/`target`/`proj`/`exec`）
//!
//! - **recog**（`Classification + bars → 声部决策 D`，对齐 `Theta.recog : H→Z→D`）：从分类标签 +
//!   历史读出每声部的决策意图（级别 L*、方向 σ、进场/退出判定、止损/入场价）。
//! - **target → proj(riskProj) → exec**（`声部决策 + 账户 → 订单`）：[`plan_orders`] 实装此确定链，
//!   对齐 `Theta.{target,proj,riskProj,exec}`。此层吃已 recog 的决策。

/// 互斥全定义策略 element-coverage 执行引擎（M29 三结论合一的 rust 兑现，与买卖点 v1 正交的
/// 新路径——在每个语法元素 λ_e 入场、ρ_e 平腿，覆盖每个笔/线段/走势，非离散择时）。
pub mod coverage;
pub mod exec;
/// 退出决策生成器（§9 closePred）+ 持仓声部台账 `HeldVoice`——回测 runner 与生产 ThetaCore 共享单源。
pub mod exit;
/// R_Θ 解释器（七链环5）：候选集 Γ(x) → 平移不变全序 ≺_Θ → 三桶 (𝒟_x close / ℬ_x open / 𝒦_x record)。
pub mod interp;
pub mod intent;
/// 全互斥买卖点解释器：可重叠谓词 P_1..P_8 固定优先级互斥化 C_j（alpha2 §5/§6，Σ1[C_j]=1 全定义）。
pub mod mutex;
pub mod ledger;
/// 区间套递归证书 N^δ + Sel_Θ 固定选择器（对照 `Origin.IntervalNestCertificate`，L2-B 补全）。
pub mod nest;
/// **Persistent Element Layer Pi**（anc.pdf §4-§9 最小修复 = persistent overlay）。
///
/// 跨 bar 持久元素注册表——修复 Q4 "LiveDetached 误处理成 Stale" 导致 depth>0 腿被 AncOK 系统性剪掉。
/// 不变量 I1-I5（anc.pdf §7）：持久身份 / 方向不变 / parent 是关系非身份 / 操作父持久 / AncOK 作用 persistent set。
pub mod persistent;
/// **M5 声部执行层独立账本 OverlayState**（多空对冲.pdf p16 关卡10 / TARGET_STRATEGY_MAXFULL.md §M5）。
///
/// hedge-mode 逐声部头寸簿 P^sep → N=Net(P^sep) → Order_t=N_t−N_{t−1}；逐声部保
/// entry_v/exit_v/parent(v)/role(v)/pnl_v。独立于 R/TW 净额账本（674号第三会计范畴）。
pub mod overlay_state;
/// 盘整/趋势在线协议状态机与 DA-Q2 协议事件轨（订单 P1..P10 的正交积因子）。
pub mod protocol;
pub mod risk;
pub mod voice;

use super::classifier::{self, Classification};
use super::config::ThetaConfig;
use super::types::{Bar, BspBits, Order, Pos, Sig, StrictAction, Tick};
use exec::FillSide;
use risk::{SizingInput, StopInput, StopSide};
use std::rc::Rc;
use voice::{ActState, VoiceSide, VoiceState};

/// 完整结构状态 Sₗ（持仓 × 信号 = 9 状态，对齐 `Origin.FullDefinitionStrategy` 动作类前件）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictState {
    pub pos: Pos,
    pub sig: Sig,
}

/// 完全应对策略 π 种子（**对齐 `Origin.FullDefinitionStrategy.ActionClass` 9→7 投影**）。
///
/// 9 状态（3 持仓 × 3 信号）→ 7 动作全函数。这是策略族的**单点种子**（固定 Param 下的 9→7 映射），
/// 由 [`StrategyFamily`] 的 Param 索引推广为族。每一格的映射逐字对齐 Origin 动作语义：
/// - flat+buySide → Buy（建仓）
/// - flat+sellSide → Wait（空仓遇卖侧，裸空非缠论 §4.4，观望不动）
/// - flat+none → Wait（空仓无信号 = 等待，**非 Hold**，Origin 动作类 wait/hold 区分）
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

/// 账户状态 Z（契约锚 `Origin.StrategyFamily.Theta` 的账户输入 Z）。
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

/// 单声部决策 D（契约锚 `Origin.StrategyFamily.Theta.recog : H → Z → D` 的输出）。
///
/// 这是 recog 从分类标签 + 历史读出的**每声部决策意图**——target/riskProj/exec 链消费它
/// 产出订单。字段（声部级，对齐 Origin.VoiceTree `VoiceState` + Θ_risk/Θ_exec 输入）：
/// - `depth`：声部深度（根=0）。决定资金权重；与 `root_side` 一起定声部绝对方向。
/// - `root_side`：**根方向 σ_root**（信号方向：买点→Long，卖点→Short）。声部绝对方向
///   = `voice::voice_side(root_side, depth)` = σ_root·(-1)^depth（spec:41 + Fugue 推广）。
/// - `exit`/`enter_ok`：退出/进场判定（Origin.VoiceTree `Exit`/`EnterOK`，运行时 Bool）。
/// - `bsp`：该声部触发点的买卖点 bit-vector（决定止损类别 + bsp_class 冲突序）。
/// - `signal_index`：信号确认的 bar 索引（exec 延迟成交起点）。
/// - `stop_in`：结构止损输入（pivot 极值 + 最后中枢，`risk::structural_stop` 用）。
/// - `entry`：入场参考价（sizing 分母 + exec 成交基准，整数 tick）。
/// - `cost_per_unit`：每单位成本（sizing 的 κ·cost 项）。
/// - `level`：决策级别（冲突排序 `ConflictKey` 的 level，高 level 先）。
///
/// ★`root_side`（声部方向由信号定）：reference spec:41「σ=+1多/-1空」未固定根方向——3买/
/// 底背驰 → Long 根，3卖/顶背驰 → Short 根。这把 Origin.VoiceTree「根恒 Long」推广为「根方向参数化」
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

/// Θ_voice + Θ_risk + Θ_exec 顶层管线——实装 `Origin.StrategyFamily.piTheta` 链的
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
/// 1. **target（Origin.VoiceTree `actState`/`targetPos`）**：从声部状态算 4 动作态 + 目标仓位。
///    close/wait → 无新仓订单（close 触发平仓订单）；open → sizing 建仓；hold → 不动。
/// 2. **riskProj（`Origin.RiskProj` sizing）**：open 时 `risk::size_position` 算唯一 qty；
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
        // 声部深度超出 max_depth ⟹ 不开声部（Origin.VoiceTree 最多 max_depth 层，spec:40）。
        if !voice::within_max_depth(d.depth, &config.voice) {
            continue;
        }

        // 声部绝对方向 = 根方向（信号定）× depth 相对极性（`voice_side`，对齐 StrategyFamily
        // §5 `long_short_both_open_allowed:570` 多独立根）。根（depth 0）= root_side（信号方向，
        // 3买→Long/3卖→Short）；子声部按 depth 奇偶相对根翻转（赋格交替）。Origin.VoiceTree
        // `dirOfDepth`（根恒 Long）是 root_side=Long + 嵌套树的强化子情形（不同有效域）。
        let side = voice::voice_side(d.root_side, d.depth);
        let q = account.qty_at(d.depth);

        // target：4 动作态（Origin.VoiceTree actState）。
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

    // ρ_{ℓ,δ,r}/Γ_{ℓ,δ,r}/GapBuffer 状态函数解析（PDF §3）：按当前决策的 (level, side) 取
    // override；空 profile ⟹ 退化为 risk 标量 + gap=0（bit-exact 默认）。多空不强行镜像。
    let side_key = match side {
        VoiceSide::Long => crate::theta_v0::config::SideKey::Long,
        VoiceSide::Short => crate::theta_v0::config::SideKey::Short,
        VoiceSide::Flat => return None,
    };
    let (rho, gamma, gap_buffer) =
        config.sizing_profile.resolve(d.level, side_key, &config.risk);

    // sizing（riskProj，唯一总仓位）。
    // tick_size 把整数 tick 价还原为美元，与 NAV（美元）量纲对齐（spec:47 分母是美元价格）。
    let sizing = SizingInput {
        nav: account.nav,
        entry: d.entry,
        stop,
        tick_size: config.tick.tick_size,
        cost_per_unit: d.cost_per_unit,
        w_depth: voice::depth_weight(d.depth, &config.voice),
        parent_cap,
        rho,
        gamma,
        gap_buffer,
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
    // 平仓动作（Close）：StrictAction::Close（平当前腿，对齐 Origin 动作类 close）。
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

/// recog 步骤（契约锚 `Origin.StrategyFamily.Theta.recog : H → Z → D`）：`Classification + bars → 声部决策`，
/// **经 R_Θ 解释器（七链环5，spec §12）路由**（不再逐点硬编码 exit）。
///
/// ## 数据流（环3 → 环5 → 决策）
///
/// 1. **环3 组装 Γ**（[`interp::assemble_gamma`]）：从 `Classification` 的所有级别×买卖点×区间套确认
///    生成有限候选集（每候选携方向 σ_g、类号、18 类角色 R(g)、N^δ 确认）。
/// 2. **环5 ℛ_Θ 唯一化**（[`interp::interpret`]）：按时刻 x（source_index）分组 Γ(x)，每时刻按平移
///    不变全序 ≺_Θ 排序 + 确定性 fold → 三桶 (𝒟_x close / ℬ_x open / 𝒦_x record)，∃! 唯一（spec §12）。
/// 3. **桶 → 决策**（[`build_decision`]）：ℬ_x 候选 → 开仓侧决策（exit=false）；𝒟_x 活动腿 → 平仓侧
///    决策（exit=true）；𝒦_x → 记录不执行（无决策）。**exit 由分桶归属推导，非硬编码字面量**。
///
/// ## 映射要点（每个 ℬ_x/𝒟_x 候选 → 一个 `VoiceDecision`，single source 零重算）
///
/// - **depth = 0（§5 多独立根）**：每个买卖点是独立根（`parent=none`，方向由自身 bits 定，不跨级别
///   赋格翻转）；`level` = ℓ 仅用于冲突排序（spec:54 高 level 先），非 depth。
/// - **root_side = σ_g**：候选方向由 [`interp::assemble_gamma`] 经 `root_sel`（镜像反对称消歧）定。
/// - **stop_in / entry / signal_index**：从原始 `BspPoint`（`gamma_index` 索引回）零重算读出。
/// - **exit / enter_ok**：由分桶推导——ℬ_x(open)→`exit=false`/`enter_ok=true`；𝒟_x(close)→
///   `exit=true`/`enter_ok=false`。这**取代**旧 `exit: false` 硬编码（MEMORY trades-vs-closedloop：
///   旧硬编码使所有决策 exit=false ⟹ act_state 永不判 Close）。
///
/// ## 诚实有效域（formalization-validity-domain，L0）
///
/// recog 是**单帧无持仓函数**（无账户持仓 Z）⟹ 传 `A_t=空` 给 interpret ⟹ 𝒟_x（关闭活动腿）**必空**
/// （`interpret` 规则2 无腿可关），故本函数实际产 ℬ_x 开仓侧决策 + 𝒦_x 跳过。**持仓驱动的 𝒟_x
/// 非空关闭由持有 active 的 caller**（runner per-moment 传 HeldVoice 台账经 [`interp::interpret`]）
/// 驱动——与 runner 的 §9 closePred（`exec::close_pred`，contract-anchored，跨 bar/价格驱动
/// Stop∨RiskClose）互补（§12 候选冲突互斥化 ≠ §9 关闭谓词，两条不同链环）。认识论 L1（bit-exact
/// 接线，不验证 Θ 市场有效）；解释器结构本身 L0（[`interp`] 模块）。
pub fn recognize(
    classification: &Classification,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Vec<VoiceDecision> {
    // ── 环3：组装候选集 Γ（所有级别×买卖点×区间套确认，spec §12）。 ──────────────────
    // [`interp::assemble_gamma`] 产 1:1 对应 BspPoint 的候选（gamma_index = BspPoint 遍历序），
    // 故下文用同序遍历建平行 `points` 列表，用 `gamma_index` 索引回原始 `BspPoint`（零重算）。
    let gamma = interp::assemble_gamma(classification);
    let points: Vec<&classifier::bsp::BspPoint> = classification
        .levels
        .iter()
        .flat_map(|level| level.bsp.iter())
        .collect();

    // ── 环5：按时刻 x（source_index）分组 → 每时刻 ℛ_Θ(Γ(x)) 三桶唯一化（spec §12）。 ──
    // **诚实有效域（formalization-validity-domain，L0）**：recog 是**单帧无持仓函数**（签名只吃
    // classification+bars+config，无账户持仓 Z——MEMORY trades-vs-closedloop 坐实），故活动集
    // `A_t = 空`。空 A_t ⟹ 𝒟_x（应关闭活动腿）**必空**（interpret 规则2 不触发，无腿可关）——
    // 持仓驱动的关闭（𝒟_x 非空）由**持有 active 的 caller**（runner per-moment 传 HeldVoice 台账）
    // 经 [`interp::interpret`] 驱动，与 runner 的 §9 closePred（exec::close_pred，contract-anchored，
    // 跨 bar/价格驱动 Stop∨RiskClose）互补（§12 候选冲突互斥化 ≠ §9 关闭谓词，两条不同链环）。
    //
    // ★exit 不再硬编码（本轮修复核心）：决策的 `exit` 由**分桶归属**推导——候选落 ℬ_x(open) ⟹
    // exit=false；落 𝒟_x(close) ⟹ exit=true。recog 空 A_t ⟹ 全部可交易候选落 ℬ_x ⟹ exit=false
    // 由 §12 解释器分桶**计算**得出，**非** `exit: false` 字面量硬编码（旧实现 recognize_point:517
    // 的硬编码已删除，替换为 𝒟_x 驱动，无旧分支 fallback——no-patch-mentality）。
    //
    // ★每个买卖点 = §5 独立根（depth=0，方向由自身 bits 定，不跨级别赋格翻转）：`level` 字段保留
    // 候选级别（冲突排序高 level 先，spec:54）；级别差 ≠ 赋格嵌套深度（§5 多独立根 ≠ Fugue 嵌套树）。
    let mut moments: Vec<usize> = gamma.iter().map(|c| c.source_index).collect();
    moments.sort_unstable();
    moments.dedup();

    let mut decisions = Vec::new();
    for x in moments {
        let gamma_x: Vec<interp::Candidate> =
            gamma.iter().filter(|c| c.source_index == x).copied().collect();
        // A_t = 空（recog 单帧无持仓，见上诚实标注）。
        let buckets = interp::interpret(&gamma_x, &[]);

        // ℬ_x(open) ⟹ 开仓侧决策（exit=false 由「候选 ∈ open 桶」推导）。
        for cand in &buckets.open {
            if let Some(d) = build_decision(cand, points[cand.gamma_index], false, bars, config) {
                decisions.push(d);
            }
        }
        // 𝒟_x(close) ⟹ 平仓侧决策（exit=true 由「腿 ∈ close 桶」推导）。recog 空 A_t ⟹ 此桶必空
        // （interpret 规则2 不触发）；本不变量显式断言（不静默——空 A_t ⟹ ∅，coding-style 显式处理）。
        debug_assert!(
            buckets.close.is_empty(),
            "recog 空 active ⟹ 𝒟_x 必空（持仓关闭由持 active 的 caller 经 interpret 驱动）"
        );
        // 𝒦_x(record)：记录但不执行（无决策产出，对齐 spec §11「记录但暂不执行的候选」）。
    }
    decisions
}

/// 单个开/平候选 → [`VoiceDecision`]（recog 逐桶映射，single source 零重算）。
///
/// `cand`：[`interp::Candidate`]（携级别/方向/类号/角色）。`point`：原始 [`classifier::bsp::BspPoint`]
/// （携 pivot/center 结构止损源，`cand.gamma_index` 索引回，零重算）。`is_exit`：分桶推导的退出标志
/// （ℬ_x→false 开仓侧 / 𝒟_x→true 平仓侧）——**replaces 旧 `exit: false` 硬编码**（no-patch）。
///
/// 返回 `None` 当：无可交易成交 bar（信号作废，spec:50）/ 含 3 类 bit 但 center=None（classifier
/// 不变量违反，显式拒绝不静默）。这两道判据逐字保留旧 recognize_point 语义（runner 诊断
/// `recog_reject_*` 对照同序：bits→fill→center）。
fn build_decision(
    cand: &interp::Candidate,
    point: &classifier::bsp::BspPoint,
    is_exit: bool,
    bars: &[Bar],
    config: &ThetaConfig,
) -> Option<VoiceDecision> {
    // 方向 σ_root：候选已由 `interp::candidate_dir`（root_sel 镜像反对称消歧）定方向；ℬ_x/𝒟_x
    // 候选必非 Flat（Flat 已归 𝒦 不入此函数）。独立根 depth=0 ⟹ voice_side(root_side,0)=root_side。
    let root_side = cand.dir;

    // entry = 信号确认后下一可交易 bar 的 open（spec:50 延迟成交基准）。无成交 bar ⟹ 信号作废。
    let fill_index = exec::fill_bar_index(point.source_index, bars, &config.exec)?;
    let entry = bars[fill_index].open;

    // stop_in：从 BspPoint 直接构造（single source，零重算）。含 3 类 bit ⟹ center 必 Some
    // （classifier 不变量）；违反则显式返回 None（不用零 Center 静默产 zg=0 错误止损价）。
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
        }, // 无 3 类 bit：center 不被 structural_stop 读，零占位无害
    };
    let stop_in = StopInput {
        pivot_low: point.pivot_low,
        pivot_high: point.pivot_high,
        center,
    };

    Some(VoiceDecision {
        depth: 0, // §5 独立根
        root_side,
        // ★exit 由 §12 解释器分桶推导（`is_exit` = 候选 ∈ 𝒟_x），**非硬编码字面量**。
        exit: is_exit,
        // 开仓侧（ℬ_x）enter_ok=true（有信号即进场许可）；平仓侧（𝒟_x）enter_ok=false（退出态非进场）。
        enter_ok: !is_exit,
        bsp: point.bits,
        signal_index: point.source_index,
        stop_in,
        entry,
        // 成本 per-unit：v0 占位 0.0（成交费用由 exec::apply_fees 施加；每单位成本模型待 L3 标定）。
        cost_per_unit: 0.0,
        level: cand.level,
    })
}

// ──────────────────────────────────────────────────────────────────────────
//  Param 索引策略族 π_Θ（G7：契约锚 `Origin.StrategyFamily.{StrategyFamily,familyOfTheta,piTheta}`）
// ──────────────────────────────────────────────────────────────────────────

/// **Param 索引策略族 π_Θ**（契约锚 `Origin.StrategyFamily.StrategyFamily` + `familyOfTheta`）。
///
/// ★G7 消硬编码单策略：Origin `StrategyFamily.π : Param → H → Z → Order` 把策略**参数化为族**——
/// 每个 `Param` 值索引一个 `Theta` 实例（`familyOfTheta : (Param → Theta) → StrategyFamily`），
/// 求值 `π param h z = piTheta (paramToTheta param) h z`。`classification_does_not_choose_unique_policy`
/// 已证「分类不推出唯一策略」（存在 π₁≠π₂）——故策略**必须**是 Param 索引的族，硬编码单策略 =
/// 抹掉这个自由度（违背 Origin 元定理）。
///
/// 本 Rust 实装把 [`ThetaConfig`] 立为 **Param 索引**（reference-theta-v0.md:config 即 Θ 空间扫描的
/// 对象）：`family().pi(config, ...)` = `piTheta` 在 param=config 处的求值。不同 config（不同 ρ/β/
/// γ/κ/max_depth/depth_weights 等）⟹ 不同 π_Θ 族成员 ⟹ 不同订单流——这正是 Phase 6 Θ 空间扫描
/// 扫的族。`given_theta_total_unique`（给定 Θ ⟹ π_Θ 唯一）由 [`plan_orders`] 的确定性保证。
///
/// ★诚实有效域（formalization-validity-domain）：本结构是**族的入口包装**（把 Param=config 注入
/// piTheta 链），不引入新算法——`pi` 复合既有 [`recognize`]（recog）+ [`plan_orders`]（target→
/// riskProj→exec），与 Origin `piTheta = exec∘proj∘target∘recog` 的四段复合同构。认识论 L1
/// （Param 索引接线，不验证某 Param 在市场有效——那是 L2/L3）。
#[derive(Debug, Clone, Copy)]
pub struct StrategyFamily;

impl StrategyFamily {
    /// 构造策略族（无状态——族由 `pi` 的 Param 参数索引，对齐 `familyOfTheta` 的纯函数族）。
    pub fn family() -> StrategyFamily {
        StrategyFamily
    }

    /// **族成员求值 `π param h z`**（契约锚 `Origin.StrategyFamily.StrategyFamily.π : Param → H → Z → Order`）。
    ///
    /// 在 Param=`config` 处求值完整 piTheta 链 `exec ∘ proj ∘ target ∘ recog`：
    /// - `recog`（H→D）= [`recognize`]：`classification + bars → 声部决策`。
    /// - `target → proj(riskProj) → exec`（D→Order）= [`plan_orders`]：`决策 + 账户 → 订单流`。
    ///
    /// 给定 Param（config）⟹ 订单流唯一（`given_theta_total_unique`：plan_orders 确定 + 冲突排序全序）。
    /// **不同 Param ⟹ 不同 π_Θ 族成员**——`classification_does_not_choose_unique_policy` 的 Rust 兑现。
    ///
    /// 参数：`param`（= Θ 索引，config）、`classification`（C_Θ 输出 H）、`bars`（历史 H）、
    /// `account`（账户 Z）。返回该 Param 下的唯一订单流 O_{t+1}。
    pub fn pi(
        &self,
        param: &ThetaConfig,
        classification: &Classification,
        bars: &[Bar],
        account: &AccountState,
    ) -> Vec<Order> {
        // recog 段（H→D）：在 Param=param 处读出声部决策。
        let decisions = recognize(classification, bars, param);
        // target→proj(riskProj)→exec 段（D→Order）：在 Param=param 处产唯一订单流。
        plan_orders(&decisions, bars, account, param)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 对齐 Origin 动作类：9 状态全枚举 → 7 动作映射逐格验证（golden table）。
    /// 这是 `Origin.FullDefinitionStrategy.ActionClass` 9→7 投影的 Rust conformance fixture——任一格漂移 = 失败。
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

    /// Origin 动作类 wait/hold 区分关键不变量：flat 永不返回 Hold，long/short+none 永远 Hold。
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
    ///
    /// ★tick_size=1.0（tick=美元）：本测试的 entry=100/stop=90 是美元值（非 1e-8 tick）。
    /// sizing 公式要求 entry/stop 单位为美元（与 NAV 量纲对齐），故用 tick_size=1.0。
    #[test]
    fn plan_orders_single_buy_open_golden() {
        let mut cfg = ThetaConfig::default();
        cfg.tick.tick_size = 1.0; // 测试中 entry/stop 是美元值（tick_size=1 ⟹ tick=美元）
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
        // sizing（tick_size=1.0）：entry_usd=100, |d_usd|=10, cost=0。
        // 项1 = floor(0.005*1e6 / 10) = 500。
        // 项2 = floor(0.6*1.0*1e6 / 100) = 6000。
        // 项3 = MAX ⟹ qty = min(500,6000,MAX) = 500。
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
            struct_break_dir: None,
            force: None,
        }];
        Classification {
            levels: vec![LevelState { bsp: Rc::new(bsp), ..Default::default() }],
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
            struct_break_dir: None,
            force: None,
        }];
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(bsp), ..Default::default() }],
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

    /// ★L2 回归（#132 真实数据 OKLO 暴露的 usize 下溢）：低级别（L0）有 bsp、**高级别**（L1+）
    /// bsp 空时，`l_star=0`，旧代码对 level_idx=1>l_star 算 `l_star - level_idx` 下溢 panic
    /// （release wrapping 成巨大 u32 静默错误）。修复=空 bsp 级别非声部节点跳过（Origin.VoiceTree
    /// 语义）。本测试构造该多级别布局，验证不 panic 且只在 L0 产决策。
    #[test]
    fn recognize_higher_empty_levels_no_underflow() {
        let cfg = ThetaConfig::default();
        // L0 有第三类买点，L1/L2 bsp 空（多级别真实常态：高级别无信号）⟹ l_star=0。
        let l0_bsp = vec![BspPoint {
            source_index: 0,
            bits: BspBits { buy3: true, ..Default::default() },
            pivot_low: 210,
            pivot_high: 0,
            center: Some(mk_center(100, 200, 3)),
            struct_break_dir: None,
            force: None,
        }];
        let classification = Classification {
            levels: vec![
                LevelState { bsp: Rc::new(l0_bsp), ..Default::default() }, // L0：非空 bsp（l_star=0）
                LevelState::default(),                            // L1：空 bsp（level_idx 1 > l_star 0）
                LevelState::default(),                            // L2：空 bsp（level_idx 2 > l_star 0）
            ],
        };
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
        ];
        // 旧代码：level_idx=1 时 0-1 下溢 panic。修复后：高空级别跳过，只 L0 产决策（depth 0 根）。
        let decisions = recognize(&classification, &bars, &cfg);
        assert_eq!(decisions.len(), 1, "只 L0 非空 bsp 级别产决策（高空级别跳过，不下溢）");
        assert_eq!(decisions[0].depth, 0, "L0 = L* ⟹ depth 0 根");
        assert_eq!(decisions[0].level, 0, "决策级别 = L0");
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
            struct_break_dir: None,
            force: None,
        }];
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(bsp), ..Default::default() }],
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
        // cc-classifier 端到端 fixture：三段在 [100,200] 重叠 ⟹ seed 中枢 zd=100,zg=200,end=12
        // （核心冻结，PDF §5 task #142）；段3 向上离开（lo=205 > ZG=200 ⟹ non-extension——延伸语义
        // 下离开段必须与冻结核心不相交，旧 lo=150 会被吸收）；段4 向下回试低点 210>200 ⟹ 3 买 @ 20。
        let l0 = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 205, 250),
                seg(Direction::Down, 16, 20, 250, 210),
            ]),
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

    // ──────────────────────────────────────────────────────────────────────
    //  G7：Param 索引策略族 π_Θ（契约锚 Origin.StrategyFamily.{StrategyFamily,familyOfTheta}）
    // ──────────────────────────────────────────────────────────────────────

    /// ★族成员求值 = piTheta 链（契约锚 `Origin.StrategyFamily.StrategyFamily.π`）：
    /// `family().pi(config, ...)` 逐态等于 `plan_orders(recognize(...))`（recog→target→riskProj→exec 复合）。
    #[test]
    fn family_pi_equals_recog_then_plan() {
        let cfg = ThetaConfig::default();
        let classification = classification_with_buy3(0);
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
            tradable_bar(2, 2, 210, 220, 205, 215),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        // 族成员 π(param=cfg) = piTheta 四段复合（recog → target → riskProj → exec）。
        let via_family = StrategyFamily::family().pi(&cfg, &classification, &bars, &account);
        let via_chain = plan_orders(&recognize(&classification, &bars, &cfg), &bars, &account, &cfg);
        assert_eq!(via_family, via_chain, "族成员求值 = piTheta 链复合");
        assert!(!via_family.is_empty());
    }

    /// ★给定 Param ⟹ π_Θ 唯一（契约锚 `Origin.StrategyFamily.given_theta_total_unique`）：
    /// 同 Param + 同输入恒同订单流（族成员是确定函数）。
    #[test]
    fn family_given_param_total_unique() {
        let cfg = ThetaConfig::default();
        let classification = classification_with_buy3(0);
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let fam = StrategyFamily::family();
        let a = fam.pi(&cfg, &classification, &bars, &account);
        let b = fam.pi(&cfg, &classification, &bars, &account);
        assert_eq!(a, b, "给定 Param ⟹ 订单流唯一（given_theta_total_unique）");
    }

    /// ★不同 Param ⟹ 不同族成员（契约锚 `classification_does_not_choose_unique_policy`）：
    /// 同分类 + 同输入，但不同 Θ 参数（如 ρ 风险预算）⟹ 订单流可不同（sizing qty 受 ρ 影响）。
    /// 这兑现「分类不推出唯一策略」——策略由 Param 索引，非分类唯一决定。
    #[test]
    fn family_distinct_param_distinct_member() {
        let classification = classification_with_buy3(0);
        let bars = vec![
            tradable_bar(0, 0, 100, 110, 90, 105),
            tradable_bar(1, 1, 205, 215, 200, 210),
            tradable_bar(2, 2, 210, 220, 205, 215),
        ];
        let account = AccountState { nav: 1_000_000.0, voice_qty: vec![0] };
        let fam = StrategyFamily::family();

        // Param A：默认 ρ=0.005。
        let cfg_a = ThetaConfig::default();
        // Param B：ρ 翻倍（更大单声部风险预算 ⟹ sizing 项1 = floor(ρ·NAV/|entry-stop|) 翻倍）。
        let mut cfg_b = ThetaConfig::default();
        cfg_b.risk.rho = 0.010;

        let orders_a = fam.pi(&cfg_a, &classification, &bars, &account);
        let orders_b = fam.pi(&cfg_b, &classification, &bars, &account);
        // 两者都产订单（同分类），但 qty 不同（不同 Param ⟹ 不同族成员）。
        assert!(!orders_a.is_empty() && !orders_b.is_empty());
        assert_ne!(
            orders_a[0].qty, orders_b[0].qty,
            "不同 Θ 参数（ρ）⟹ 不同 π_Θ 族成员（订单 qty 不同）——分类不推出唯一策略"
        );
    }
}
