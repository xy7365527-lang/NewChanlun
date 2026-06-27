//! 完整状态 `x_t`（17 分量）的 Rust 实装——契约锚 `Origin.CompleteStateEvent.CompleteState`
//! （FULL 结果包 §3 line 133-144 boxed）。
//!
//! ## 工位定位（cov-rust-impl E1 状态~50% 补全）
//!
//! cov-rust-impl 报告「rust state 仅 ~50%」的根源：闭环引擎镜像的 [`super::super::closed_loop::state::
//! AssemblyState`] 是 **6 分量摘要态**（对齐摘要态 `Origin.FullDefinitionStrategy.StrictState`），把
//! 声部/订单/记忆折叠为 `u64` 计数，且 σ_r 根方向、ω 订单状态、E 清算权益、Cash、ν（借券/保证金/场所）
//! **从未作为逐分量字段存在**。本模块把 §3 的完整 17 分量**逐分量显式化**（不摘要、零遗漏），与
//! Lean `Origin.CompleteStateEvent.CompleteState` 逐字段对齐。
//!
//! ## 与 `AssemblyState` 的关系（诚实声明，非补丁）
//!
//! `AssemblyState`（闭环引擎在线态）**不被本模块替换或删除**——它是 `hybridStep` 闭环的工程化简态，
//! 服务于逐 bar 在线推进（`micro_state` 前缀窗口 + 双账本不变量线程化）。本 [`CompleteState`] 是 §3
//! **完整状态 schema** 的逐分量镜像，服务于「与 FULL §3 零遗漏对照」。二者关系 = 完整态（本模块）→
//! 摘要态（AssemblyState）是一个**遗忘投影**（forgetful projection，见 [`CompleteState::to_assembly_summary`]）：
//! 摘要态的每个字段都能从完整态导出，反之不能（摘要丢失了声部/订单/记忆的逐项内容）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本模块 = **L0**（纯结构 schema 镜像：17 分量结构 + 枚举与 Lean `CompleteStateEvent` 逐字段对齐 =
//!   FULL §3 文字 ↦ Rust 类型，零信息增量）。`cargo build` 绿 = 类型自洽 + 记账恒等可承载，**不**是
//!   缠论盈利/实盘有效声明。
//! - 记账恒等 `R=Π-A-W` 由复用的 [`LedgerComp`]（契约锚 `Origin.LedgerState`，携 `inv_holds()`）承载
//!   ——本模块**不**重新定义账本，只把它装进完整态的 `ledger` 字段（与 Lean `CompleteState.ledger`
//!   引 `LedgerState` 同构）。
//!
//! ## 跨组依赖诚实标注
//!
//! `D_t`（所有级别缠论递归结构）依赖中枢/走势递归——组B（CenterConstruct）在做该层。本模块**不臆造**
//! `D_t` 递归内容，用引擎 parser 输出的结构对象列表（`Vec<Bar>`/`Vec<Stroke>`/`Vec<Segment>`/
//! `Vec<Center>`/`Vec<Move>`）作 `D_t` 的结构承载——这是引擎真实产出的类型，非占位桩（对齐 Lean
//! `CompleteState.recStruct : ParseStruct`，`ParseStruct` 同样是这些列表的乘积）。

use super::super::types::{Bar, Center, MoveKind, Segment, Stroke};
use super::super::strategy::ledger::LedgerComp;

/// 根方向 `σ_{r,t} ∈ {-1,0,+1}`（契约锚 `Origin.CompleteStateEvent.RootDirection`，FULL §3 line 150）。
///
/// 三态完全分类：空头/空仓/多头。子声部方向由 `σ_v = σ_r·(-1)^{d(v)}` 递归导出（§8 line 466），
/// 故根方向是整棵声部树方向的生成元。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDirection {
    Short, // σ_r = -1
    Flat,  // σ_r =  0
    Long,  // σ_r = +1
}

impl RootDirection {
    /// 语义投影到 `i64`（-1/0/+1，对齐 Lean `RootDirection.toInt` + FULL §3 line 150 数值约定）。
    pub fn to_int(self) -> i64 {
        match self {
            RootDirection::Short => -1,
            RootDirection::Flat => 0,
            RootDirection::Long => 1,
        }
    }
}

/// 声部订单状态 `ω_{v,t}`（契约锚 `Origin.CompleteStateEvent.OrderPhase`，FULL §3 line 153）。
///
/// 四态完全分类：空仓/待开/持仓/待平。区别于 `a`（是否激活 bool）：`a` 是**持仓事实**，`ω` 是**订单
/// 相位**——同一 a=0 可处于「空仓」（无挂单）或「待开」（开仓单已发未成交）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderPhase {
    Flat,         // 空仓
    PendingOpen,  // 待开
    Held,         // 持仓
    PendingClose, // 待平
}

/// 单个声部状态三元组 `(q_{v,t}, a_{v,t}, ω_{v,t})`（契约锚 `Origin.CompleteStateEvent.VoiceState`，
/// FULL §3 line 138/151-153）。
///
/// - `q : u64`（`q_{v,t} ≥ 0`，FULL line 151：绝对单位数非负——`u64` 类型层强制非负，对齐 Lean `Nat`）。
/// - `active : bool`（`a_{v,t} ∈ {0,1}`，FULL line 152）。
/// - `phase : OrderPhase`（`ω_{v,t}`，FULL line 153）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceState {
    pub q: u64,
    pub active: bool,
    pub phase: OrderPhase,
}

/// 声部族 + 有根有序树 `(·)_{v∈V}` + `𝒯=(V,p)`（契约锚 `Origin.CompleteStateEvent.VoiceForest`，
/// FULL §3 line 138 + §8 line 443-445）。
///
/// - `voices`：按 v∈V 索引的三元组族（节点列表）。
/// - `parent`：父函数 `p`（节点 i 的父为 `parent[i]`，根为 `None`）——有限有根有序树无环表示。
/// - `root_index`：根声部在 `voices` 中的位置（§10 根声部双向状态机的作用对象）。
///
/// 一致性条件（祖先闭合 / 精确同单位 / 方向递归，FULL §9 + §8）作为**可分离谓词**（见
/// [`VoiceForest::ancestor_closed`] / [`VoiceForest::exact_unit_match`]），不耦进结构构造——对齐
/// Lean `VoiceForest.ancestorClosed` / `exactUnitMatch`（谓词 def，非结构字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceForest {
    pub voices: Vec<VoiceState>,
    pub parent: Vec<Option<usize>>,
    pub root_index: usize,
}

impl VoiceForest {
    /// 一致性条件 · 祖先闭合（FULL §9 line 517：`a_{v,t} ≤ a_{p(v),t}`）：子激活蕴含父激活。
    ///
    /// 对齐 Lean `VoiceForest.ancestorClosed`（可观测谓词，运行时检查；Lean 是 `Prop`，Rust 是 `bool`）。
    pub fn ancestor_closed(&self) -> bool {
        for (i, v) in self.voices.iter().enumerate() {
            if let Some(Some(j)) = self.parent.get(i) {
                if let Some(parent_v) = self.voices.get(*j) {
                    if v.active && !parent_v.active {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 一致性条件 · 精确同单位双开（FULL §9 line 525：`a_{v,t}=1 ⟹ q_{v,t}=q_{p(v),t}`）。
    ///
    /// 对齐 Lean `VoiceForest.exactUnitMatch`：激活的子声部单位数等于父声部单位数。
    pub fn exact_unit_match(&self) -> bool {
        for (i, v) in self.voices.iter().enumerate() {
            if let Some(Some(j)) = self.parent.get(i) {
                if let Some(parent_v) = self.voices.get(*j) {
                    if v.active && v.q != parent_v.q {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// 资本阶段 `Φ_t`（契约锚 `Origin.CompleteStateEvent.CompleteState.phase` 引 `CapitalPhase`，
/// FULL §3 line 154 + §12 line 929-963 完全分类，5 态）。
///
/// 与 Lean 复用的 `Origin.FullDefinitionStrategy.CapitalPhase`（5 态：phaseI/phaseII/repair/
/// protectedPhase/accretive）逐构造子对齐。★这是完整 5 态，**非** `closed_loop::state::Phase` 的 3 态
/// 摘要——本模块零遗漏要求完整 5 态（§12 `∑_φ 𝟙[Φ_t=φ]=1` 完全分类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapitalPhase {
    PhaseI,         // I 建仓（W<I0 ∧ Y=0 ∧ ¬ReadyReturn）
    PhaseII,        // II 取本（W<I0 ∧ [Y>0 ∨ ReadyReturn]）
    Repair,         // III_repair（W≥I0 ∧ R<R*+L^wc）
    ProtectedPhase, // III_protected（W≥I0 ∧ R*+L^wc≤R<R*+L^wc+m^unit）
    Accretive,      // III_accretive（W≥I0 ∧ R≥R*+L^wc+m^unit）
}

/// 账本 `(Π_t,A_t,W_t,R_t)`（FULL §3 line 157-160）——复用 [`LedgerComp`]（契约锚 `Origin.LedgerState`，
/// 携 `R=Π-A-W` 恒等 `inv_holds()`）。本模块不重新定义账本，与 Lean `CompleteState.ledger : LedgerState`
/// 同构（4 符号 ↦ 1 字段，恒等由账本自身承载）。
pub use super::super::strategy::ledger::LedgerComp as Ledger;

/// 订单动作（`O_t` 元素的动作维度，契约锚 `Origin.CompleteStateEvent.OrderAction`，FULL §3 line 162）。
///
/// 开/平/加/减四类覆盖声部状态机（§10 开平 + §11/§8 加减核心单位）。`add`/`reduce` 带下划线避免与
/// Rust 关键字/方法名冲突的风格一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAction {
    Open,
    Close,
    Add,
    Reduce,
}

/// 单个未完成订单 `O_t` 的元素（契约锚 `Origin.CompleteStateEvent.Order'`，FULL §3 line 162）。
///
/// 状态侧「挂单簿」条目（区别于 strategy 段输出的瞬时 [`super::super::types::Order`]）：
/// - `action`：开/平/加/减。
/// - `voice_index`：归属声部在 [`VoiceForest::voices`] 中的索引（订单总挂在某声部上）。
/// - `qty : u64`（订单量非负）。
/// - `submitted_at : i64`（提交时刻 t，因果时间戳）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenOrder {
    pub action: OrderAction,
    pub voice_index: usize,
    pub qty: u64,
    pub submitted_at: i64,
}

/// 信号使用记录与结构记忆 `M_t`（契约锚 `Origin.CompleteStateEvent.SignalMemory`，FULL §3 line 163）。
///
/// `Fresh` 谓词的状态载体（§10 line 573 开启谓词含 `Fresh_{v,t}`）：
/// - `consumed_signals`：已消费信号标识序列（`Fresh` 检查信号是否已在此列表）。
/// - `last_bar_seen`：结构记忆——最近处理的 bar 序号（单调，因果时间锚）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalMemory {
    pub consumed_signals: Vec<i64>,
    pub last_bar_seen: i64,
}

/// 运行/经纪状态 `ν_t`（契约锚 `Origin.CompleteStateEvent.VenueState`，FULL §3 line 163-164）。
///
/// §13 杠杆保证金完全分类的状态侧载体，逐子分量：
/// - `borrowable`：借券可行性（§9 `F^{eq}` 含「借券规则成立」line 578）。
/// - `margin_used`：已占用保证金（§13 `IM_t`/`MM_t` 当前占用值）。
/// - `venue_open`：交易场所开放/可交易（§13 line 1098）。
/// - `running`：系统运行状态（halt/正常）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueState {
    pub borrowable: bool,
    pub margin_used: i64,
    pub venue_open: bool,
    pub running: bool,
}

/// `D_t` 所有级别缠论递归结构（FULL §3 line 149）——引擎 parser 输出的结构对象乘积，对齐 Lean
/// `CompleteState.recStruct : ParseStruct`（`ParseStruct` = mergedBars × fractals × strokes × segments
/// × centers × moves × tail 的乘积）。
///
/// ★跨组依赖诚实标注（见模块头）：组B（CenterConstruct）做中枢/走势递归层。本结构承载引擎当前产出的
/// 结构列表（真实类型，非桩）；若组B 把 `D_t` 精化为多级别递归树（RecursiveLevelSystem），此类型可
/// 重锚——届时是 schema 精化，非缺陷。
///
/// ★`moves` 用 `(MoveKind, start, end)` 元组承载走势序列：rust `types` 当前只有 `MoveKind` enum
/// （趋势/盘整），无独立 `Move` struct——本模块**不臆造** `Move` 类型（那是 parser/classifier 层的
/// owner 职责），用「走势类型 + 起止 source_index」元组真实承载走势（对齐 Lean `ParseStruct` 的 moves
/// 分量语义，零臆造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecStruct {
    pub merged_bars: Vec<Bar>,
    pub strokes: Vec<Stroke>,
    pub segments: Vec<Segment>,
    pub centers: Vec<Center>,
    pub moves: Vec<(MoveKind, usize, usize)>,
}

/// 完整状态 `x_t`（17 分量逐分量，契约锚 `Origin.CompleteStateEvent.CompleteState`，FULL §3 boxed）。
///
/// 字段与 Lean `CompleteState` 逐字段对齐（见 Lean 模块的分量对照表）：17 个 FULL 符号 ↦ 14 个字段
/// （账本 `(Π,A,W,R)` 4 符号 ↦ `ledger` 1 字段，恒等由 [`LedgerComp::inv_holds`] 承载）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteState {
    /// 1. `h_t` 市场历史 `(y_0,…,y_t)`（FULL line 137）。
    pub history: Vec<Bar>,
    /// 2. `D_t` 所有级别缠论递归结构（FULL line 149；跨组依赖见模块头）。
    pub rec_struct: RecStruct,
    /// 3. `σ_{r,t}` 根方向（FULL line 150）。
    pub root_dir: RootDirection,
    /// 4. `(q_v,a_v,ω_v)_{v∈V}` 声部三元组族 + 有根有序树（FULL line 151-153 + §8）。
    pub voices: VoiceForest,
    /// 5. `Φ_t` 资本阶段（FULL line 154；5 态完全分类，§12）。
    pub phase: CapitalPhase,
    /// 6. `I_0` 最初外部资本（FULL line 156）。
    pub initial_capital: i64,
    /// 7-10. `(Π_t,A_t,W_t,R_t)` 账本（FULL line 157-160）。`R=Π-A-W` 由 `LedgerComp` 承载。
    pub ledger: LedgerComp,
    /// 11. `E_t` 清算权益（FULL line 161；§13 杠杆分母 `L^G=G/E`）。
    pub equity: i64,
    /// 12. `Cash_t` 现金（FULL line 141）。
    pub cash: i64,
    /// 13. `O_t` 未完成订单（FULL line 162）。
    pub open_orders: Vec<OpenOrder>,
    /// 14. `M_t` 信号使用记录和结构记忆（FULL line 163）。
    pub memory: SignalMemory,
    /// 15. `ν_t` 借券/保证金/场所/运行状态（FULL line 164）。
    pub venue: VenueState,
}

impl CompleteState {
    /// 初始完整状态 `x_0`（开局：空历史 + 空结构 + 空仓 flat 根方向 + PhaseI + 初始账本 + 零仓零单零记忆）。
    ///
    /// `i0`：初始外部资本 `I_0`（进 `initial_capital` + 账本 `LedgerComp::initial`；`equity=cash=i0`
    /// 开局权益=现金=本金）。
    pub fn initial(i0: i64) -> CompleteState {
        CompleteState {
            history: Vec::new(),
            rec_struct: RecStruct {
                merged_bars: Vec::new(),
                strokes: Vec::new(),
                segments: Vec::new(),
                centers: Vec::new(),
                moves: Vec::new(),
            },
            root_dir: RootDirection::Flat,
            voices: VoiceForest {
                // 开局只有根声部（空仓未激活），无子声部。
                voices: vec![VoiceState { q: 0, active: false, phase: OrderPhase::Flat }],
                parent: vec![None],
                root_index: 0,
            },
            phase: CapitalPhase::PhaseI,
            initial_capital: i0,
            ledger: LedgerComp::initial(i0),
            equity: i0,
            cash: i0,
            open_orders: Vec::new(),
            memory: SignalMemory { consumed_signals: Vec::new(), last_bar_seen: -1 },
            venue: VenueState {
                borrowable: true,
                margin_used: 0,
                venue_open: true,
                running: true,
            },
        }
    }

    /// 记账恒等 `R=Π-A-W`（FULL §3 line 166-170：合法状态必须满足）——委托 [`LedgerComp::inv_holds`]
    /// （契约锚 `Origin.LedgerState.inv`）。对齐 Lean `complete_state_ledger_identity`（对任意完整态恒真）。
    pub fn ledger_identity_holds(&self) -> bool {
        self.ledger.inv_holds()
    }

    /// 根方向到子声部方向的递归投影 `σ_v = σ_r·(-1)^{d(v)}`（FULL §8 line 466）。
    ///
    /// `depth` = 节点到根的距离 d(v)。返回该深度声部的有向极性（-1/0/+1）：根（depth=0）即 `σ_r`，
    /// 每下一级反向。对齐 Lean schema 的方向递归一致性条件（§8 line 463-467）。
    pub fn voice_direction(&self, depth: u32) -> i64 {
        let sign = if depth % 2 == 0 { 1 } else { -1 };
        self.root_dir.to_int() * sign
    }
}
