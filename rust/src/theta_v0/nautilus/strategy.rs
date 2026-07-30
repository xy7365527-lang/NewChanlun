//! `ThetaStrategy` Nautilus Rust-native 适配器骨架（串 4 个 adapter + 退出生成器接入点）。
//!
//! ## 职责（设计文档 §4.1-4.2 数据流）
//!
//! Nautilus Rust-native `Strategy`（`StrategyCore` + `DataActor`）的 `on_bar` 中：
//! 1. [`super::bar_adapter`]：Nautilus Bar → S_Θ `types::Bar`，追加到累积窗口。
//! 2. S_Θ 管线：`parse_layer → classify → recognize → plan_orders`（`StrategyFamily::pi`）。
//! 3. [`super::account_adapter`]：Nautilus portfolio → S_Θ `AccountState`（sizing 用真实账户）。
//! 4. [`super::order_adapter`]：S_Θ `Order` → Nautilus 下单意图 → `order_factory` → `submit_order`。
//! 5. 退出生成器接入点：§9 closePred（止损/反向 BSP/RiskClose）触发 → Close 订单走 Nautilus。
//!
//! ## ★骨架（nautilus 依赖未加，不编译）：真实 Strategy trait 以注释 + TODO 锚定。
//!
//! 真实结构（context7 `write_rust_strategy.md`，待依赖后兑现）：
//! ```ignore
//! use nautilus_common::actor::DataActor;
//! use nautilus_model::data::Bar;
//! use nautilus_trading::{nautilus_strategy, strategy::StrategyCore};
//!
//! pub struct ThetaStrategy {
//!     core: StrategyCore,              // order_factory + portfolio 集成
//!     instrument_id: InstrumentId,
//!     bar_type: BarType,
//!     config: ThetaConfig,             // S_Θ 参数（Param 索引族成员）
//!     bars: Vec<theta_v0::types::Bar>, // 累积窗口
//! }
//! nautilus_strategy!(ThetaStrategy);
//! impl DataActor for ThetaStrategy {
//!     fn on_start(&mut self) -> anyhow::Result<()> { self.subscribe_bars(self.bar_type); Ok(()) }
//!     fn on_bar(&mut self, bar: &Bar) -> anyhow::Result<()> { self.on_bar_inner(bar)?; Ok(()) }
//!     fn on_order_filled(&mut self, e: &OrderFilled) -> anyhow::Result<()> { /* 对账 */ Ok(()) }
//!     fn on_position_closed(&mut self, e: &PositionClosed) -> anyhow::Result<()> { /* 清台账 */ Ok(()) }
//! }
//! ```

use crate::theta_v0::classifier::streaming::OwnedIncrementalClassifier;
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::strategy::exit::{
    exit_decision_for_nested, parent_invalid_at, record_held_voice, subtree_close_exit_decisions,
    HeldVoice,
};
use crate::theta_v0::strategy::voice::{self, VoiceSide};
use crate::theta_v0::strategy::{self, AccountState, VoiceDecision};
use crate::theta_v0::types::{Bar, Order, StrictAction};

use super::account_adapter::{self, PortfolioSnapshot};
use super::order_adapter::{self, OrderIntent};

/// `ThetaStrategy` 适配器的 S_Θ 侧核心状态（与 Nautilus `StrategyCore` 组合）。
///
/// ★骨架：依赖加入后，把此结构嵌入真实 `ThetaStrategy { core: StrategyCore, inner: ThetaCore }`，
/// `on_bar` 调 [`ThetaCore::plan_for_bar`]，再用返回的 `OrderIntent` 调 `core.order_factory()`（TODO）。
#[derive(Debug, Clone)]
pub struct ThetaCore {
    /// S_Θ 参数（Param 索引策略族成员，`StrategyFamily::pi` 的 param）。
    ///
    /// ★#346 MED-3：私有（非 `pub`）——`classifier` 字段持有构造期克隆的同值拷贝（见下），
    /// 两份拷贝的一致性只在「外部无写路径」下成立；`pub` 会开一条绕过 `ThetaCore::new` 的直接
    /// 赋值口子（`core.config = other`），使两份拷贝静默分叉且无检测。私有化 = 唯一写入口收敛到
    /// `new`（构造期一次性克隆两份），物理上排除分叉，非仅口头承诺。
    config: ThetaConfig,
    /// 累积 bar 窗口（source_index = 窗口序号；S_Θ 管线吃完整窗口，非单 bar）。
    pub bars: Vec<Bar>,
    /// ★持仓声部台账（退出决策生成器的缠论语义真相源，按 depth 索引，长度 = max_depth）。
    ///
    /// **边界注释（no-workaround，消双源的存在论分界）**：持仓**数量**真相源 = Nautilus
    /// portfolio（[`PortfolioSnapshot`]）；本字段 `held` 是**缠论语义**真相源——`stop`（结构止损价）
    /// 与 `decision`（入场决策快照）**无 Nautilus 对应物**（Nautilus 只记数量/成本，不记缠论买卖点
    /// 类别/中枢止损），故必须自维护。消除的是「数量双账」（runner 模拟 units vs portfolio），
    /// 缠论状态与数量正交，自维护 held **不违反**消除双源原则。
    held: Vec<Option<HeldVoice>>,
    /// ★逐 bar 决策分组（退出生成器 `reverse_signal` 项的源，与回测 runner `groups` 同构）。
    ///
    /// `groups[i]` = 第 i 个 bar 的开仓侧 recog 决策。[`exit_decision_for`] 读 `groups[i]` 判当前 bar
    /// 是否出现反向 BSP（χ^{σ_p}）。生产路径逐 bar 追加（owned `VoiceDecision`），调用时构造
    /// `&[Vec<&VoiceDecision>]` 视图喂 `exit_decision_for`（与 runner 传全局 groups 同签名，零分叉）。
    groups: Vec<Vec<VoiceDecision>>,
    /// ★#345：自持缓冲区增量分类器（`OwnedIncrementalClassifier`，无生命周期参数）——替代
    /// 每 bar 对 `self.bars` 全量重跑 `parse_layer`+`classify_with_tower`（O(n²) 根因，#342）。
    /// owned 一份 `config` 拷贝（构造期克隆）。`self.config` 仍是权威源，`classifier` 内部拷贝
    /// 只读、无独立写路径——**分叉不可达**由上方 `config` 字段私有化保证（唯一写入口 = `new`
    /// 构造期，两份拷贝同源同帧克隆），非本注释单方面声明（#346 MED-3：声明须有对应的物理约束，
    /// `pub` 字段配"永不分叉"是声明膨胀，090号）。
    classifier: OwnedIncrementalClassifier,
}

impl ThetaCore {
    /// 构造（给定 Θ config，空窗口，空台账，空增量分类器缓存）。
    pub fn new(config: ThetaConfig) -> Self {
        let max_depth = config.voice.max_depth as usize;
        let classifier = OwnedIncrementalClassifier::new(config.clone());
        ThetaCore {
            config,
            bars: Vec::new(),
            held: vec![None; max_depth],
            groups: Vec::new(),
            classifier,
        }
    }

    /// **每 bar 决策（适配层核心，设计文档 §4.2 数据流）**。
    ///
    /// 1. 追加新 bar（已由 [`super::bar_adapter::from_nautilus_bar`] 转换为 S_Θ `types::Bar`，
    ///    `source_index = self.bars.len()`）。
    /// 2. S_Θ 管线：`parse_layer → classify → StrategyFamily::pi(config, cls, bars, account)`
    ///    （= recognize + plan_orders，产唯一订单流）。
    /// 3. 每个 S_Θ `Order` → [`OrderIntent`]（order_adapter，持仓方向从 portfolio 快照推）。
    ///
    /// 返回本 bar 的下单意图列表（调用方 strategy.on_bar 用 `core.order_factory()` 提交，TODO）。
    ///
    /// ## ★诚实有效域（设计文档 §4.4）
    ///
    /// - 持仓真相源 = Nautilus portfolio（`snap`）——sizing 用真实账户 NAV+净仓，**不用** S_Θ
    ///   `plan_and_fill_mtm` 的内部模拟台账（生产路径只用 recognize+plan_orders 产订单，fill/equity
    ///   由 Nautilus venue 撮合，见设计文档 §4.4）。
    /// - **退出决策生成器**（runner.rs `exit_decision_for`，§9 closePred）的缠论触发逻辑在生产路径
    ///   需移植到此处（持仓 + 当前 bar → 止损/反向 BSP/RiskClose → Close 决策）——本骨架**标接入点**
    ///   （TODO），不内联实现（避免与 runner 双源；移植时复用 runner 的 `exit_decision_for` 逻辑，
    ///   持仓从 `snap` 读而非内部台账）。这是 no-patch 的「声明边界」：骨架不假装退出逻辑已就位。
    ///
    /// 边界条件：bars 不足以产生结构（< min_parts_per_level）⟹ classify 产空 ⟹ 无决策 ⟹ 空意图
    /// （诚实退化，对齐 runner.rs `structureless_data_yields_empty_orders`）。
    ///
    /// ## 退出生成器接入（§9 closePred，设计文档 §4.4——本次实装）
    ///
    /// 顺序 = **退出先于开仓**（spec:54）：
    /// 1. **退出侧**：对 `held` 中每个持仓声部调 [`exit_decision_for`]（equity = `snap.nav +
    ///    snap.unrealized_pnl`，对齐 `Origin.TotalWealth = free + holding`）→ X=true 产 `exit=true`
    ///    决策 → `plan_orders` 产 Close → 转 Close 意图。持仓真相从 `snap` 读（非内部台账）。
    /// 2. **开仓侧**：当前 bar 的 recog 决策 → `plan_orders` 产开仓订单 → 转意图；开仓侧（Buy/Sell/
    ///    Add）成交意图对应的决策 `record_held_voice` 记台账（供后续 bar 退出判定）。
    ///
    /// ★生产 vs 回测的差异（no-workaround 诚实标注）：回测 runner 用**内部延迟成交队列**
    /// （`exit_orders_at`，模拟撮合时序），生产路径**无延迟队列**——Close 意图直接交 Nautilus venue
    /// 撮合，`exit_pending`/全平清台账由 venue fill 回调（`on_order_filled`/`on_position_closed`，
    /// 骨架 TODO）驱动。本函数产**意图**（退出判定逻辑与 runner bit-exact 共享 `exit_decision_for`），
    /// 撮合时序差异归 venue，不分叉退出判定。
    pub fn plan_for_bar(&mut self, new_bar: Bar, snap: &PortfolioSnapshot) -> Vec<OrderIntent> {
        self.bars.push(new_bar);
        let i = self.bars.len() - 1;

        // account / 方向从 Nautilus portfolio（真实持仓真相源）。
        let account: AccountState =
            account_adapter::to_account_state(snap, self.config.voice.max_depth);
        let pos_dir = account_adapter::position_dir(snap);

        // recog 段（当前窗口 → 开仓侧声部决策）。与 `groups` 追加同源（退出生成器读 groups[i]）。
        let decisions = self.recognize_current(new_bar);
        // ★关⑤：groups 只收**根域开仓决策**（exit=false ∧ depth==0）——§9 反向项信号池：
        // interpret 规则2 已消费的反向触发（close 决策携入场快照 bsp，非当 bar 信号）与
        // ReverseOpen 子决策的反父 bits 均不入池（M13：父仓穿越次级反向信号持有，短差由子腿
        // 承担，非父平仓触发）——与 runner 双账路径（plan_and_fill_mtm_dual）同口径。
        self.groups.push(
            decisions
                .iter()
                .filter(|d| !d.exit && d.depth == 0)
                .copied()
                .collect(),
        );

        let mut intents: Vec<OrderIntent> = Vec::new();

        // ── 1. 退出侧（spec:54 退出先于开仓）：逐持仓声部 §9 closePred → Close 意图。 ──
        // equity = NAV + 未实现盈亏（mark-to-market，对齐 Origin.TotalWealth = free + holding）。
        let equity_now = snap.nav + snap.unrealized_pnl;
        // groups 视图（&[Vec<&VoiceDecision>]）：与 runner 传全局 groups 同签名喂 exit_decision_for。
        let groups_view: Vec<Vec<&VoiceDecision>> =
            self.groups.iter().map(|g| g.iter().collect()).collect();
        let mut exit_decisions: Vec<VoiceDecision> = Vec::new();
        for depth in 0..self.held.len() {
            let hv = match self.held[depth] {
                Some(hv) => hv,
                None => continue, // 空仓声部
            };
            if hv.exit_pending {
                continue; // Close 意图已发，待 venue fill（不重复触发，对齐 spec:50）
            }
            if account.qty_at(depth as u32) == 0 {
                continue; // portfolio 已无此声部持仓（venue 已平）
            }
            // ★关⑤：parent_invalid 实义化（父槽空仓 ∨ 父 exit_pending，exit.rs:121 恒假占位消除）。
            let parent_invalid = parent_invalid_at(&self.held, depth);
            if let Some(exit_d) = exit_decision_for_nested(
                &hv,
                depth,
                &self.bars[i],
                i,
                &groups_view,
                equity_now,
                parent_invalid,
            ) {
                // ★关⑤级联发射（M16 AncOK 父关则子关，最深优先）：父退出 ⟹ 全部更深
                // held 强制退出（pending 槽跳过，fill 前抑制）。
                // ★#183 T4 归一：生产级联归一到镜像函数（held 槽压缩链投影 → subtree_close；
                // 散装 cascade_exit_decisions 已下线）。
                for d in subtree_close_exit_decisions(&self.held, depth, exit_d, i) {
                    exit_decisions.push(d);
                    if let Some(slot) = self.held.get_mut(d.depth as usize) {
                        if let Some(ref mut h) = slot {
                            h.exit_pending = true; // 标记 pending（避免 fill 前重复触发同一退出）
                        }
                    }
                }
            }
        }
        if !exit_decisions.is_empty() {
            let exit_orders =
                strategy::plan_orders(&exit_decisions, &self.bars, &account, &self.config);
            for o in &exit_orders {
                if let Some(intent) =
                    order_adapter::to_order_intent(o, pos_dir, self.entry_tick_for(o))
                {
                    intents.push(intent);
                }
            }
        }

        // ── 2. 开仓侧：当前 bar recog 决策 → 开仓订单 → 意图；成交侧记台账。 ──
        let open_orders = strategy::plan_orders(&decisions, &self.bars, &account, &self.config);
        for o in &open_orders {
            // 开仓订单（Buy/Sell/Add）⟹ 记台账（退出生成器读它）。按方向匹配决策回找
            // depth/止损源（与 runner `matched` 同逻辑——decisions↔orders 非一一，按方向 + depth 配）。
            if matches!(
                o.action,
                StrictAction::Buy | StrictAction::Sell | StrictAction::Add
            ) {
                if let Some(d) = decisions.iter().find(|d| {
                    let side = voice::voice_side(d.root_side, d.depth);
                    matches!(
                        (o.action, side),
                        (StrictAction::Buy | StrictAction::Add, VoiceSide::Long)
                            | (StrictAction::Sell, VoiceSide::Short)
                    )
                }) {
                    record_held_voice(&mut self.held, d);
                }
            }
            if let Some(intent) = order_adapter::to_order_intent(o, pos_dir, self.entry_tick_for(o))
            {
                intents.push(intent);
            }
        }

        intents
    }

    /// 清空持仓声部台账（缠论语义真相源）。
    ///
    /// ③ `ThetaStrategy::on_position_closed` 调用：venue 报仓位全平 ⟹ 清 held（缠论买卖点/止损
    /// 语义状态随持仓消失而清空；持仓**数量**真相由 Nautilus portfolio 管，本台账只管缠论语义）。
    pub fn clear_held(&mut self) {
        for slot in self.held.iter_mut() {
            *slot = None;
        }
    }

    /// recog 段（`append_bar 增量分类 → recognize_nested`）：当前 bar 窗口 → 开仓侧声部决策。
    ///
    /// ★关⑤接线（施工图 §4.6）起点是 [`classifier::classify_with_tower`]
    /// （Classification **bit-identical**，分类层零漂移契约）。
    ///
    /// ★#345：`classify` 段改走 `self.classifier.append_bar(new_bar)`（`OwnedIncrementalClassifier`，
    /// bit-exact 等价于全量 `classify_with_tower(parse_layer(&self.bars))`——见
    /// `classifier::streaming` 模块头 + `backtest::incremental::tests::owned_bit_exact_*`）。
    /// 旧实现每 bar 对 `self.bars` 全量重跑 `parse_layer`+`classify_with_tower`，是 O(n²) 根因
    /// （#342：`chanlun/review-results/nt-engine-scaling-profile-20260726.md`）；`append_bar`
    /// 只增量处理新追加的这一根 bar（inclusion O(1) + 下游 O(尾部)，摊还 O(1)/bar）。
    ///
    /// `new_bar` 必须与刚 push 进 `self.bars`（`plan_for_bar` 调用处）的那根**同一个值**——
    /// 增量分类器内部状态与 `self.bars` 长度必须逐 bar 锁步（一次 `plan_for_bar` 调用 = 一次
    /// `bars.push` + 一次 `append_bar`，不允许跳 bar/重复调用，否则增量血缘与 `self.bars`
    /// 失配，bit-exact 契约破裂）。
    ///
    /// 接 [`strategy::recognize_nested`]（候选源换真嵌套塔取真 ReverseOpen 角色 + held 活动投影
    /// 喂 interpret ⟹ 角色门四合取产 **depth>0 子声部**，root_side 继承树根）。
    /// venue 侧 hedge-mode 账户前提（q⁺/q⁻ 双腿共存）列部署裁定（施工图 §7 L7）——
    /// 本适配层只产决策/意图，venue 撮合语义不变。
    ///
    /// ★诚实：等价 `StrategyFamily::pi` 的 recog 段（不含 target→exec 的 plan_orders）——拆出
    /// 单独 recog 是因退出生成器需要 `decisions`（喂 `groups[i]` 的 reverse_signal 项）+ 开仓侧
    /// 分别走 plan_orders（退出决策与开仓决策不可混批，否则冲突排序语义错）。
    fn recognize_current(&mut self, new_bar: Bar) -> Vec<VoiceDecision> {
        // ★#346 MED-2：旧护栏 `self.bars.last() == Some(&new_bar)` 恒真（`plan_for_bar` 总是先
        // push 再传同一个 new_bar 调本函数，与分类器内部状态毫无关系——手工 push 后跳过一次
        // append_bar 也照样通过，评审实证见 lockstep_guard_catches_skipped_bar）。真正的锁步
        // 契约是「分类器认为自己吃过多少根 bar」== `self.bars.len()`（append_bar 前应差 1，
        // append_bar 后应相等）——用 `assert_eq!`（非 `debug_assert_eq!`）：本仓库以 `cargo test
        // --release` 为验收命令，`debug_assert!` 在 release profile 下被完全编译剥离
        // （`[profile.release]` 未设 `debug-assertions = true`，实测坐实：`debug_assert!(false)`
        // 探针在 `cargo test --release` 下空跑不 panic），
        // 用它做护栏等于没有护栏。
        assert_eq!(
            self.classifier.bar_count(),
            self.bars.len() - 1,
            "recognize_current: 分类器内部 bar 计数({})与 self.bars.len()-1({}) 失配\
             （跳 bar/重复调用/绕道，增量分类器契约破裂）",
            self.classifier.bar_count(),
            self.bars.len() - 1
        );
        let (classification, tower) = self.classifier.append_bar(new_bar);
        assert_eq!(
            self.classifier.bar_count(),
            self.bars.len(),
            "recognize_current: append_bar 后分类器内部计数({})应与 self.bars.len()({}) 同步",
            self.classifier.bar_count(),
            self.bars.len()
        );
        let active = strategy::held_voice_projection(&self.held);
        strategy::recognize_nested(&classification, &tower, &active, &self.bars, &self.config)
    }

    /// 取订单对应的限价 tick（开仓限价腿）。
    ///
    /// ★骨架占位：S_Θ `Order` 不直接携带 entry 价（entry 在 `VoiceDecision` 中，plan_orders 已消费）。
    /// 生产路径需让 `plan_orders` 或本适配层保留 decision→order 的 entry 映射（TODO）。当前骨架返回
    /// `None`（市价腿）——限价腿的 entry 价回填待 plan_orders 接口扩展（报 theta_v0 owner，非本工位改）。
    fn entry_tick_for(&self, _order: &Order) -> Option<crate::theta_v0::types::Tick> {
        None // TODO: 限价腿 entry 价回填（依赖 plan_orders 暴露 decision→order entry 映射）
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::types::Bar;

    fn mk_bar(idx: usize, close: i64) -> Bar {
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: close,
            high: close,
            low: close,
            close,
            volume: 100,
            untradable: false,
        }
    }

    fn flat_snap() -> PortfolioSnapshot {
        PortfolioSnapshot {
            nav: 1_000_000.0,
            net_position: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
        }
    }

    /// L0 骨架退化验证：无结构 bar 序列 ⟹ 空下单意图（对齐 runner 诚实退化）。
    ///
    /// ★认识论 L0（合成单调 bar，验证适配管线串通 + 正确退化，零信息增量）。这**不是** L2
    /// （L2 需真实数据 + 真实 Nautilus 引擎，依赖未加）。
    #[test]
    fn structureless_bars_yield_no_intents() {
        let mut core = ThetaCore::new(ThetaConfig::default());
        let snap = flat_snap();
        // 单调上涨（无顶底交替 ⟹ 无缠论结构 ⟹ 无买卖点 ⟹ 无订单）。
        let mut intents = Vec::new();
        for i in 0..50 {
            intents = core.plan_for_bar(mk_bar(i, 1000 + i as i64), &snap);
        }
        assert!(
            intents.is_empty(),
            "单调数据无结构 ⟹ 空下单意图（诚实退化，非缺陷）"
        );
        assert_eq!(core.bars.len(), 50, "bar 窗口累积到 50");
    }

    /// ★#346 MED-2 反例：锁步护栏必须能真正抓到跳 bar。旧护栏
    /// `self.bars.last() == Some(&new_bar)` 恒真——手工往 `self.bars` push 一根后再走
    /// `plan_for_bar`，旧护栏照样通过（评审实证）。新护栏比对 `classifier.bar_count()` 与
    /// `self.bars.len()`：手工 push 后 `plan_for_bar` 内部 `self.bars.len()` 变 2、
    /// `classifier.bar_count()` 仍是 0，`recognize_current` 起手的 `assert_eq!` 必须炸。
    #[test]
    #[should_panic(expected = "失配")]
    fn lockstep_guard_catches_skipped_bar() {
        let mut core = ThetaCore::new(ThetaConfig::default());
        let snap = flat_snap();
        // 手工跳 bar：直接 push 到 self.bars，绕过分类器（模拟"漏调 append_bar"实现 bug）。
        core.bars.push(mk_bar(0, 1000));
        // 合法路径喂下一根——plan_for_bar 内部 push 后 self.bars.len()=2，
        // classifier.bar_count()=0，锁步护栏在 recognize_current 起手必须 panic。
        core.plan_for_bar(mk_bar(1, 1001), &snap);
    }

    /// ★#345 集成层 bit-exact（code-review Standards/Spec 双轴浮出的缺口订正）：既有
    /// `owned_bit_exact_*` 只在 `OwnedIncrementalClassifier` 组件级验证，`recognize_current`
    /// 这一接线点本身从未在 `ThetaCore::plan_for_bar`（唯一合法生产入口）驱动下验证过。
    ///
    /// 本测试**只**经 `plan_for_bar` 逐 bar 喂（不像 `held_long_stop_hit_yields_close_intent`
    /// 那样手工 `core.bars.push`/`core.held[..]=` 绕过 recog 路径），逐 bar 用克隆-窥视法验证
    /// `self.classifier.append_bar` 的返回值 == legacy 全量 `classify_with_tower(parse_layer(..))`：
    /// `plan_for_bar` 调前克隆 `core.classifier`（`OwnedIncrementalClassifier: Clone`），调后用该
    /// 克隆重放同一根 bar——纯函数（同起始状态+同输入 bar ⟹ 同输出），不需要新增生产代码访问器。
    #[test]
    fn recognize_current_integration_bit_exact_via_plan_for_bar() {
        use crate::theta_v0::classifier;
        use crate::theta_v0::parser;

        let config = ThetaConfig::default();
        let mut core = ThetaCore::new(config.clone());
        let snap = flat_snap();
        for i in 0..80usize {
            let cycle = ((i as f64) / 11.0).sin() * 40.0;
            let bar = mk_bar(i, 1000 + i as i64 + cycle as i64);
            let classifier_before = core.classifier.clone();
            core.plan_for_bar(bar, &snap);
            let (owned_cls, owned_tower) = classifier_before.clone().append_bar(bar);

            let l0 = parser::parse_layer(&core.bars, &config);
            let (legacy_cls, legacy_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(
                owned_cls, legacy_cls,
                "bar {i}: plan_for_bar 内部增量分类 != legacy 全量"
            );
            assert_eq!(
                owned_tower.len(),
                legacy_tower.len(),
                "bar {i}: tower 层数不同"
            );
            for (lvl, (ol, ll)) in owned_tower.iter().zip(legacy_tower.iter()).enumerate() {
                assert_eq!(ol, ll, "bar {i} lvl {lvl}: LeveledMove 不同");
            }
        }
    }

    /// 窗口累积：每 bar 追加，source_index 单调。
    #[test]
    fn bars_accumulate_with_monotone_index() {
        let mut core = ThetaCore::new(ThetaConfig::default());
        let snap = flat_snap();
        core.plan_for_bar(mk_bar(0, 1000), &snap);
        core.plan_for_bar(mk_bar(1, 1010), &snap);
        assert_eq!(core.bars.len(), 2);
        assert_eq!(core.bars[0].source_index, 0);
        assert_eq!(core.bars[1].source_index, 1);
    }

    use crate::theta_v0::strategy::exit::HeldVoice;
    use crate::theta_v0::strategy::risk::StopInput;
    use crate::theta_v0::strategy::voice::VoiceSide;
    use crate::theta_v0::strategy::VoiceDecision;
    use crate::theta_v0::types::{BspBits, Center};

    /// 持多 snap（net_position>0 ⟹ portfolio 持多，voice_qty[0]>0）。
    fn long_snap(nav: f64, qty: f64) -> PortfolioSnapshot {
        PortfolioSnapshot {
            nav,
            net_position: qty,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
        }
    }

    /// 入场决策快照（depth 0 做多根 1 买，止损 pivot/center 源 → structural_stop 算 hv.stop）。
    fn buy1_decision(signal_index: usize) -> VoiceDecision {
        VoiceDecision {
            depth: 0,
            root_side: VoiceSide::Long,
            exit: false,
            enter_ok: true,
            bsp: BspBits {
                buy1: true,
                ..Default::default()
            },
            signal_index,
            stop_in: StopInput {
                pivot_low: 950,
                pivot_high: 1100,
                center: Center {
                    zd: 1000,
                    zg: 1080,
                    dd: 940,
                    gg: 1090,
                    start_index: 0,
                    end_index: 5,
                },
            },
            entry: 1000,
            cost_per_unit: 0.0,
            level: 0,
        }
    }

    /// ★退出生成器自检（验收门，认识论 L1 接线）：持仓触止损 ⟹ 产 Close 意图。
    ///
    /// 直接注入 `held` 台账（绕过开仓侧 recog 结构构造——开仓侧由 strategy 层 L2 端到端测试
    /// `real_classify_to_orders_end_to_end` 覆盖；本自检专验「退出判定 → Close 意图」接线链）。
    /// 验证：exit_decision_for（Stop 触发）→ plan_orders（产 Close）→ to_order_intent（持多 ⟹
    /// Sell + reduce_only）整条接通。
    ///
    /// ★延迟语义（no-workaround 诚实标注）：退出 Close 经 `build_exit_order` 的 `fill_bar_index`
    /// 算成交基准 bar。回测默认 `entry_delay_bars=1`（模拟「信号确认后下一根成交」），但 ThetaCore
    /// 流式逐 bar、退出在**末根** bar 触发 ⟹ delay=1 时 fill_bar_index(i) 找 i+1 不存在 ⟹ 丢单。
    /// 生产路径用 `entry_delay_bars=0`（§4.4 line 271-273「延迟归 venue」：生产不模拟延迟，意图即时
    /// 发 venue，真实延迟由 venue 撮合）⟹ fill_bar_index(i)=i ⟹ 末根 bar 即成交基准 ⟹ 产 Close。
    /// 故本测试用 delay=0（生产路径的正确 Param，非 hack）。delay=0 是 plan_orders 既有合法 config
    /// 值（u32），不改 plan_orders 代码、runner bit-exact 不受影响。
    #[test]
    fn held_long_stop_hit_yields_close_intent() {
        // ★生产路径 Param：entry_delay_bars=0（§4.4 line 271-273「延迟归 venue」的落实）。
        // 回测默认 delay=1 模拟「信号确认后下一根成交」；生产路径**不模拟延迟**——意图即时发
        // Nautilus venue，真实延迟由 venue 撮合实现。ThetaCore 每根 bar 流式调用、退出在末根 bar
        // 触发，delay=0 ⟹ fill_bar_index(i)=i（当前 bar 即成交基准）⟹ plan_orders 产 Close。
        // 这是生产路径的**正确 Θ 参数**（由 #5d 构造生产 ThetaConfig 时设定），非测试 hack。
        let mut cfg = ThetaConfig::default();
        cfg.exec.entry_delay_bars = 0;
        let mut core = ThetaCore::new(cfg);

        // bar 0：注入持多台账后此 bar low=900 跌破 stop=950 ⟹ Stop 触发；bar 1 作 Close 成交 bar。
        let snap = long_snap(1_000_000.0, 300.0); // portfolio 持多 300 手（voice_qty[0]=300）

        // 先喂 bar 0（建立窗口 + groups[0]），手动注入持多台账（模拟开仓已成交、portfolio 已持多）。
        let stop_bar = Bar {
            source_index: 0,
            timestamp: 0,
            open: 1000,
            high: 1010,
            low: 900, // low ≤ stop(950) ⟹ 多头止损触及（exec::stop_hit）
            close: 920,
            volume: 100,
            untradable: false,
        };
        core.bars.push(stop_bar);
        core.groups.push(Vec::new()); // bar 0 无开仓决策（手动注入台账，非 recog 路径）
                                      // ★#346 MED-2 锁步护栏：手动 push bar 0 到 self.bars 时须同步喂分类器（生产路径每根
                                      // bar 恒过 recognize_current→append_bar，手动注入台账不是绕过分类器的理由——分类器
                                      // 内部 bar_count 与 self.bars.len() 一旦失配，护栏在下一次 plan_for_bar 必炸）。
        core.classifier.append_bar(stop_bar);
        core.held[0] = Some(HeldVoice {
            side: VoiceSide::Long,
            stop: 950,
            decision: buy1_decision(0),
            exit_pending: false,
        });

        // 用 plan_for_bar 喂 bar 1（退出在 bar 1 评估：bar 1 也 low 跌破 stop；bar 1 作成交 bar）。
        let exit_eval_bar = Bar {
            source_index: 1,
            timestamp: 1,
            open: 915,
            high: 920,
            low: 900,
            close: 905,
            volume: 100,
            untradable: false,
        };
        let intents = core.plan_for_bar(exit_eval_bar, &snap);

        // 退出生成器应产至少一条 Close 意图（持多 ⟹ Sell + reduce_only）。
        let close_intent = intents.iter().find(|oi| oi.reduce_only);
        assert!(
            close_intent.is_some(),
            "持仓触止损 ⟹ 产 Close 意图（退出生成器接通：exit_decision_for→plan_orders→intent）"
        );
        let ci = close_intent.unwrap();
        assert_eq!(ci.side, order_adapter::OrderSideLike::Sell, "平多 ⟹ Sell");
        assert!(ci.reduce_only, "退出 ⟹ reduce_only");
        assert!(ci.qty > 0, "全平当前持仓 ⟹ qty>0");
        // 台账标记 pending（避免下一 bar 重复触发）。
        assert!(
            core.held[0].map(|h| h.exit_pending).unwrap_or(false),
            "退出触发后 held.exit_pending=true（fill 前不重复入意图）"
        );
    }

    /// 退出 pending 幂等：已触发退出的声部下一 bar 不重复产 Close（spec:50 一次触发一次平仓）。
    #[test]
    fn exit_pending_no_duplicate_close() {
        // delay=0（生产路径 Param，同上）——确保「无 Close」来自 pending 跳过，非 fill_bar_index 落空。
        let mut cfg = ThetaConfig::default();
        cfg.exec.entry_delay_bars = 0;
        let mut core = ThetaCore::new(cfg);
        let snap = long_snap(1_000_000.0, 300.0);
        core.bars.push(mk_bar(0, 1000));
        core.groups.push(Vec::new());
        // ★#346 MED-2 锁步护栏：同上，手动注入 bar 0 须同步喂分类器。
        core.classifier.append_bar(mk_bar(0, 1000));
        // 注入已 pending 的台账（上一 bar 已触发退出，Close 在 venue 撮合中）。
        core.held[0] = Some(HeldVoice {
            side: VoiceSide::Long,
            stop: 950,
            decision: buy1_decision(0),
            exit_pending: true, // 已 pending
        });
        let stop_bar = Bar {
            source_index: 1,
            timestamp: 1,
            open: 900,
            high: 905,
            low: 890,
            close: 895,
            volume: 100,
            untradable: false,
        };
        let intents = core.plan_for_bar(stop_bar, &snap);
        assert!(
            intents.iter().all(|oi| !oi.reduce_only),
            "exit_pending ⟹ 不重复产 Close 意图（一次触发一次平仓）"
        );
    }
}
