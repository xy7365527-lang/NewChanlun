//! #197 执行账归属键与并行记账视图（expand 阶段，补实现索引缺口 G3）。
//!
//! 出处：spec `chanlun/plans/spec-accounting-layer-alignment-20260723.md` ID-2 WP-2
//! （#185 审计推荐路线）＋ 归属键粒度用户裁定（2026-07-23）：**不用每级总余额**——
//! 首开反向（ReverseOpen）、头寸（Core）、次级别做空（Short）三账分清；余额按（账户, 级别, 仓位节点）
//! 分实例记账，聚合值仅作派生视图（可求和呈现，不作 canonical 存储）。
//!
//! 三身份与六动作映射（#194 动作分类考证 §三/§五，互斥完备）：
//! 开多/平多 → [`AccountIdentity::Core`]（本仓账，按级别分层）；
//! 开空/平空 → [`AccountIdentity::Short`]（空仓账 = CONTEXT.md「反向根」：无父反向声部）；
//! 次级别首开反向开/平 → [`AccountIdentity::ReverseOpen`]（首开反向账，父仓不动＋次级别反向双开；
//! 原 `ShortDiff` 短差账——#281 裁定更名（#283 实装），并按 ADR 0001 修正案一 修4「每级一本账」
//! 把级别维度落进身份类型：`ReverseOpen{level}`，与 `Core{level}` 同形）。
//!
//! **expand 模式纪律**：本模块只新增类型与只读旁路视图，不替换任何既有调用点语义
//! （`ExitType`/`Order`/净额 `units` 路径零行为变化）；消费这些类型修复四处现病是
//! 修复票 #198/#199/#200 的范围。查重声明：本票补实现索引缺口 G3（执行账
//! (account, level) 键缺席——数学有、实现无），为接线/补缺口，**非新建账本**
//! （`.chanlun/implementation-index-20260723.md` G3）。
//!
//! CONTEXT.md 张力的诚实声明：反向根条款有「事后恰好性：身份是参照系视图，非存储事实，
//! 账本零身份字段」。本模块的账户身份是**归属视图键**（卖出按归属唯一记账的投影），
//! 不是对声部的存储重分类——分实例键只作归属投影，与 P_sep 存储层正交。

use std::collections::HashMap;

use super::coverage::Vertical;
use super::interp::PositionNodeId;
use super::voice::VoiceSide;

/// 账户身份（#185 断言5「恰一账户」）：单值 enum，禁 Option/集合/缺省。
///
/// 每个卖出动作恰有一个账户身份；风险强平等跨账户动作须展开为多条单账户
/// [`AccountOrder`]（本视图的镜像按腿逐条过账，天然满足展开形态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountIdentity {
    /// 本仓账（按级别分层）：ambient 多根与 FollowParent 级联核心仓（含其空向级联——
    /// 方向由数量符号表达，身份不变）。一类点后 `Core(level)=0` 须成为可强断言的账本事实。
    Core { level: u32 },
    /// 首开反向账（每级一本，修4）：父声部 active 期间的次级别反向对冲声部（σ_u = −σ_p，父仓不动）。
    /// 方向两态皆合法（父空时首开反向做多，CONTEXT.md 短差条款）。
    /// 原 `ShortDiff`（短差账）——#281 裁定更名（#283 实装）；`level` = 该反向声部自身级别
    /// （修4「每级一本账」级别维度落进身份类型，与 `Core{level}` 同形）。
    ReverseOpen { level: u32 },
    /// 空仓账 = CONTEXT.md「反向根」：**无父反向声部**（ambient 空根）。
    /// 与首开反向互斥且完备：任一反向声部，有父即首开反向、无父即反向根（CONTEXT.md:93）。
    Short,
}

/// 动作理由（与账户身份正交——不再用 `ExitType` 同时表达两者，#185 修复 a 推荐路线）。
///
/// 本枚举报齐当前生产真实发出的理由；修复票消费时按需扩展（如 `CoreResidualCorrection`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionReason {
    /// 信号驱动开仓（首开/加仓当前同枚举值；细分属修复票范围）。
    Open,
    /// 一类反向证书（本级确认反转；`ExitType::CloseRoot` 在账户轴上的理由侧）。
    ReverseType1,
    /// 二类反向证书的**合法**卖出（#199 分流后仅 ReverseOpen 首开反向平 / Short 平空两身份；
    /// 核心腿二类已分流 [`ActionReason::CoreResidualCorrection`]——本仓对二类封闭，
    /// ID-3「二类点合法卖出仅短差、开空」）。`ExitType` 侧一/二类仍坍缩 CloseRoot
    /// （五枚举不动），本字段保留一类/二类之分的正交信息。
    ReverseType2,
    /// 三类反向证书（减仓语义；`ExitType::ReduceCore` 的理由侧）。
    ReverseType3,
    /// §13 结构剪枝（AncOK 连带剪/Stale prune，父驱动同 bar 连带离场）。
    StructuralPrune,
    /// P1 风险强平（`force_flat`/stop）。
    RiskExit,
    /// TW StageII 重叠腿关闭（P2 CloseOverlay 通道）。
    OverlayClose,
    /// 窗口终点 censored（未离场腿兑现到末可交易 bar）。
    WindowEnd,
    /// 二类核心残余纠错（#199「仅残余才纠错」）：二类反向清理一类点应平未平的
    /// `Core{level}` 残余——仅在核心残余实测非零时触发，否则二类不得生成本仓卖单
    /// （spec ID-3：二类清本仓只能是残余纠错；level 由 [`AccountKey`] 携带）。
    /// 账户/理由正交：typed 层五枚举不动（二类 typed 归因仍归 `ExitType::CloseRoot`，
    /// 编排者裁定 2026-07-23——本变体只长在理由轴，不进 `ExitType`）。
    CoreResidualCorrection,
    /// 二类开空通道（#200，spec WP-2 修复 c / #185 审计发现 2）：二类反向候选开
    /// **空仓账**（[`AccountIdentity::Short`] = CONTEXT.md 反向根——无 active 父的反向
    /// 声部，ambient 守卫读法：父 active⇒首开反向账、无父⇒空仓账）。票面形状
    /// `OpenShort{level, certificate}`：level 与入场证书由 [`AccountKey`] 携带
    /// （`key.level` + `key.position.entry_certificate`），理由轴只标通道名。
    /// 与 [`ActionReason::Open`] 的区别：OpenShort 专指二类候选触发的 C 账户开仓
    /// （一类首开/加仓开、首开反向开均保留 Open——通道归属经 [`reason_of_open`] 单源）。
    OpenShort,
}

/// 声部身份 → 账户身份（入场时固定，与 `reverse_exit_type` 同取 `entry_v` 入场冻结值）。
///
/// 返回 `Option` 是映射函数的诚实全域性：`Flat` 方向候选归 𝒦 记录不开腿（interp.rs
/// `assemble_gamma`），任何 `entry_v` × `Flat` 组合都无账户身份，返回 `None` 而非伪造归属。
/// **[`AccountOrder`] 的 account 字段本身不是 Option**（断言5）。
pub fn identity_of(entry_v: Vertical, side: VoiceSide, level: u32) -> Option<AccountIdentity> {
    match (entry_v, side) {
        // Flat = 无方向声部不开仓 ⟹ 无归属（全域一致，非仅 Ambient 臂）。
        (_, VoiceSide::Flat) => None,
        // 反父方向子声部 = 首开反向（方向两态皆首开反向）。
        (Vertical::ReverseOpen, _) => Some(AccountIdentity::ReverseOpen { level }),
        // 无父容器：多根 = 本仓；空根 = 反向根（无父反向声部）。
        (Vertical::Ambient, VoiceSide::Long) => Some(AccountIdentity::Core { level }),
        (Vertical::Ambient, VoiceSide::Short) => Some(AccountIdentity::Short),
        // 顺父级联 = 级联核心仓（#185 修复 a：FollowParent 归 core structural exit 的账户侧）。
        (Vertical::FollowParent, _) => Some(AccountIdentity::Core { level }),
    }
}

/// 反向触发类 → 理由（1/2/3 类候选；bsp_class 合法域外返回 `None`，不伪造理由）。
pub fn reason_of_reverse(trigger_class: u8) -> Option<ActionReason> {
    match trigger_class {
        1 => Some(ActionReason::ReverseType1),
        2 => Some(ActionReason::ReverseType2),
        3 => Some(ActionReason::ReverseType3),
        _ => None,
    }
}

/// #199 反向关闭理由的**账户分流**（「仅残余才纠错」单源判据，spec WP-2 修复 b）。
///
/// - `Core{level}` × 二类 ⟹ `core_residual=true` 时 [`ActionReason::CoreResidualCorrection`]
///   （残余纠错）；`false` 时 `None`——二类不得生成本仓卖单（ID-3：二类清本仓只能是
///   残余纠错，实测残余非零才触发）。
/// - `ReverseOpen`/`Short` × 二类 ⟹ [`ActionReason::ReverseType2`]（合法二类卖：
///   首开反向平/平空，ID-3「二类点合法卖出仅两身份」）。
/// - 其余（账户, 触发类）⟹ 委托 [`reason_of_reverse`]（一/三类账户中立，不镜像判据）。
///
/// `core_residual` 由调用侧实测喂入（生产 = 并行记账视图 `balance(Core{level}) != 0`，
/// runner π fill loop 反向关闭镜像点）；本函数不持账本，保持纯判据。
pub fn reason_of_reverse_close(
    account: AccountIdentity,
    trigger_class: u8,
    core_residual: bool,
) -> Option<ActionReason> {
    if trigger_class == 2 {
        return match account {
            AccountIdentity::Core { .. } => {
                core_residual.then_some(ActionReason::CoreResidualCorrection)
            }
            AccountIdentity::ReverseOpen { .. } | AccountIdentity::Short => reason_of_reverse(2),
        };
    }
    reason_of_reverse(trigger_class)
}

/// #200 开仓理由的**账户归属**（OpenShort 通道单源判据，spec WP-2 修复 c）。
///
/// - `Short` × 二类触发 ⟹ [`ActionReason::OpenShort`]（二类开空：无父反向声部经
///   [`identity_of`] 归空仓账——ambient 守卫读法在此落成理由标注，不另立归属判据）。
/// - 其余（账户, 触发类）⟹ [`ActionReason::Open`]（一类首开/加仓、首开反向开、
///   FollowParent 级联开均保留既有口径——OpenShort 只标二类 × 空仓账这一通道）。
///
/// 账户身份由调用侧经 [`identity_of`] 单源解析喂入（与关闭侧
/// [`reason_of_reverse_close`] 同判据，不镜像）；本函数不持候选/账本，保持纯判据。
pub fn reason_of_open(account: AccountIdentity, trigger_class: u8) -> ActionReason {
    match (account, trigger_class) {
        (AccountIdentity::Short, 2) => ActionReason::OpenShort,
        _ => ActionReason::Open,
    }
}

/// 分实例记账键 =（账户, 级别, 仓位节点）（归属键粒度裁定 2026-07-23）。
///
/// `position` = [`PositionNodeId`] 严格身份四元组（carrier, 证书, side, generation，
/// Task #166），同 carrier 先后 campaign 不碰撞。`level` = 声部级别（carrier.level）；
/// `Core{level}` 内的 level 与本字段一致（构造不变量，[`AccountKey::new`] 断言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountKey {
    pub account: AccountIdentity,
    pub level: u32,
    pub position: PositionNodeId,
}

impl AccountKey {
    pub fn new(account: AccountIdentity, level: u32, position: PositionNodeId) -> Self {
        debug_assert!(
            match account {
                AccountIdentity::Core { level: l } => l == level,
                AccountIdentity::ReverseOpen { level: l } => l == level,
                _ => true,
            },
            "#197 构造不变量：Core{{level}}/ReverseOpen{{level}} 的 level 与键 level 一致（{:?} vs {})",
            account,
            level
        );
        Self {
            account,
            level,
            position,
        }
    }
}

/// 账户订单 `{account, reason}`（spec WP-2 原名形状）：账户身份单值、理由正交，
/// (account, level) 键经 `key` 携带。`qty_delta` 有符号（多正空负；平仓与持仓反向）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccountOrder {
    pub key: AccountKey,
    pub reason: ActionReason,
    /// 有符号数量（开仓 = ±腿级 sizing 目标，与 `TypedTrade::units` 同源 `SepLeg.q_units`；
    /// 平仓 = 反向等量。当前生产减仓语义整腿离场，故平仓 = 全平；真部分减仓属修复票）。
    pub qty_delta: f64,
    /// 决策 bar（腿级账本以决策 bar 为成交时点，与 `TypedTrade` 价格口径一致；
    /// 执行延迟净额队列的逐账户投影属 #198–#200）。
    pub decision_bar: usize,
}

impl AccountOrder {
    /// 账户身份（断言5：单值 enum，禁 Option/集合/缺省）。
    pub fn account(&self) -> AccountIdentity {
        self.key.account
    }
}

/// 成交结果（链 stage 5）：携同一 (account, level, position) 键。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccountFill {
    pub order: AccountOrder,
    /// 成交价（腿级口径 = 决策 bar close，与 `TypedTrade::entry_px/exit_px` 同源）。
    pub fill_px: f64,
    pub fill_bar: usize,
    /// 本笔已实现盈亏（平仓释放部分；开仓为 0）。
    pub realized_pnl: f64,
}

/// 分实例持仓（canonical 存储单元）：数量 + 成本基 + 累计已实现。
///
/// 成本基口径 = 现势持仓的入场名义额（Σ |Δqty| × fill_px，平仓按均价比例释放）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccountInstance {
    pub key: AccountKey,
    /// 有符号现势数量（多正空负；0 = 已结清）。
    pub qty: f64,
    pub cost_basis: f64,
    pub realized_pnl: f64,
    /// 在飞 ⟺ qty ≠ 0。
    pub open: bool,
}

impl AccountInstance {
    fn new(key: AccountKey) -> Self {
        Self {
            key,
            qty: 0.0,
            cost_basis: 0.0,
            realized_pnl: 0.0,
            open: false,
        }
    }
}

/// 链纪律违例（090：显式拒绝，不静默漂移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountChainError {
    /// 成交未提交的订单（pending fill 是真实阶段：submit → pending → fill）。
    NotPending,
}

/// 并行记账视图（expand）：按（账户, 级别, 仓位节点）分实例记账，余额可读三身份。
///
/// **canonical 存储** = 分实例持仓 + 全链事件日志（订单/pending/成交结果）；
/// **聚合余额是派生视图**——[`balance`](Self::balance) 现算投影、
/// [`balance_as_of`](Self::balance_as_of) 自事件日志重放，不存任何总余额字段
/// （用户裁定 2026-07-23：不用每级总余额）。
///
/// 全链（(account, level) 键携带五阶段）：[`submit`](Self::submit) 订单入日志并进 pending
/// → [`fill`](Self::fill) 出队 pending、过账持仓/成本基、记成交结果。
#[derive(Debug, Default, Clone)]
pub struct ParallelAccountLedger {
    instances: HashMap<AccountKey, AccountInstance>,
    orders: Vec<AccountOrder>,
    pending: Vec<AccountOrder>,
    fills: Vec<AccountFill>,
}

impl ParallelAccountLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// 提交订单：入订单日志 + pending 队列（链 stage 1+2）。
    pub fn submit(&mut self, order: AccountOrder) {
        self.orders.push(order);
        self.pending.push(order);
    }

    /// 成交：pending 出队（未提交即拒，[`AccountChainError::NotPending`]），
    /// 按均价法过账持仓/成本基/已实现，记成交结果（链 stage 3+4+5）。
    pub fn fill(
        &mut self,
        order: AccountOrder,
        fill_px: f64,
        fill_bar: usize,
    ) -> Result<(), AccountChainError> {
        let pending_idx = self
            .pending
            .iter()
            .position(|p| {
                p.key == order.key
                    && p.decision_bar == order.decision_bar
                    && p.qty_delta == order.qty_delta
            })
            .ok_or(AccountChainError::NotPending)?;
        self.pending.remove(pending_idx);

        let inst = self
            .instances
            .entry(order.key)
            .or_insert_with(|| AccountInstance::new(order.key));
        let delta = order.qty_delta;
        let mut realized = 0.0;
        if inst.qty == 0.0 || inst.qty.signum() == delta.signum() {
            // 开仓/加仓：全额进成本基。
            inst.cost_basis += delta.abs() * fill_px;
            inst.qty += delta;
        } else {
            // 减仓/平仓/翻转：均价释放；翻转余量按成交价另起成本基。
            let closing = delta.abs().min(inst.qty.abs());
            let avg = inst.cost_basis / inst.qty.abs();
            realized = inst.qty.signum() * closing * (fill_px - avg);
            inst.cost_basis -= closing * avg;
            inst.qty += delta;
            let remaining = delta.abs() - closing;
            if remaining > 0.0 {
                inst.cost_basis = remaining * fill_px;
            }
        }
        inst.realized_pnl += realized;
        inst.open = inst.qty != 0.0;

        self.fills.push(AccountFill {
            order,
            fill_px,
            fill_bar,
            realized_pnl: realized,
        });
        Ok(())
    }

    /// 提交并成交（决策 bar 腿级口径一步过账；生产镜像用）。
    pub fn post(&mut self, order: AccountOrder, fill_px: f64, fill_bar: usize) {
        self.submit(order);
        self.fill(order, fill_px, fill_bar)
            .expect("#197 post：刚 submit 的订单必在 pending（不可达，健康生产）");
    }

    /// 分实例读数（canonical）。
    pub fn instance(&self, key: &AccountKey) -> Option<&AccountInstance> {
        self.instances.get(key)
    }

    pub fn instances(&self) -> &HashMap<AccountKey, AccountInstance> {
        &self.instances
    }

    pub fn orders(&self) -> &[AccountOrder] {
        &self.orders
    }

    pub fn pending(&self) -> &[AccountOrder] {
        &self.pending
    }

    pub fn fills(&self) -> &[AccountFill] {
        &self.fills
    }

    /// 按身份读余额（**派生视图**：现算投影，非存储）。
    /// `Core{level}`/`ReverseOpen{level}` 精确到该级；`Short` 为该身份全部实例之和。
    pub fn balance(&self, account: AccountIdentity) -> f64 {
        self.instances
            .values()
            .filter(|i| i.key.account == account)
            .map(|i| i.qty)
            .sum()
    }

    /// 按身份读成本基聚合（**派生视图**：现算投影，非存储，与 [`balance`](Self::balance) 同规格）
    /// ——issue #357：本仓口径落地（编排者裁定 A，`Core{level}` 身份账户）取数入口，喂
    /// [`super::short_diff_bucket::CoreCostBasisSnapshot`] 构造。同身份多仓位节点分实例成本基
    /// 求和（同 `balance` 聚合口径，仅作派生呈现，非 canonical 存储）。
    pub fn cost_basis(&self, account: AccountIdentity) -> f64 {
        self.instances
            .values()
            .filter(|i| i.key.account == account)
            .map(|i| i.cost_basis)
            .sum()
    }

    /// 按身份 × 声部方向读分量余额（**派生视图**：现算投影，非存储）——★#237 断言门
    /// 按方向拆查（#234 蓝图依据：分侧账禁净额 P_sep，「多头腿和空头腿先作为两个独立
    /// 坐标存在」；用户裁 2026-07-24）：一类卖批查**多侧**分量、一类买批查**空侧**分量。
    /// 顺父级联 Short 腿归空侧（实例键 `key.position.side`，#185 映射不动）；净额
    /// [`balance`](Self::balance) 口径不动（两派生视图共存，净额 ker N 掩盖面由拆查收口）。
    pub fn balance_side(&self, account: AccountIdentity, side: VoiceSide) -> f64 {
        self.instances
            .values()
            .filter(|i| i.key.account == account && i.key.position.side == side)
            .map(|i| i.qty)
            .sum()
    }

    /// 按身份 × 声部方向读分量成本基（**派生视图**：现算投影，非存储，与
    /// [`balance_side`](Self::balance_side) 同规格）——issue #357 关票条件 C：`Core{level}`
    /// 取数改分侧口径，本方法喂多头侧 `CoreCostBasisSnapshot::cost_basis()`，与分侧余额
    /// 同一过滤谓词（`key.position.side == side`），避免多空并存时空侧成本基污染多头快照。
    pub fn cost_basis_side(&self, account: AccountIdentity, side: VoiceSide) -> f64 {
        self.instances
            .values()
            .filter(|i| i.key.account == account && i.key.position.side == side)
            .map(|i| i.cost_basis)
            .sum()
    }

    /// 清零判据容差（#400）：`balance` 是派生视图（对 `instances`——一个
    /// `HashMap`——逐实例 `qty` 求和），HashMap 遍历顺序不定，求和顺序不同
    /// 可在浮点末位产生抖动。`1e-9` 是**经验选值，非推导界**——本仓库
    /// `backtest/fill.rs` 的 `(closed_qty - closed as f64).abs() < 1e-9` 量纲
    /// 相同（都是 qty），但防的是不同机制：那处是单个浮点值离最近整数的量化
    /// 残差；这里是对若干仓位节点 `qty` 求和的误差，而各 `qty` 上溯到
    /// `backtest/fill.rs` 的 `let base_units = equity_nav / px;`——一次真实除法，
    /// 误差量级与被除值成正比，并非固定 1e-9。多节点求和的误差界**没有推导
    /// 过**；`1e-9` 只是借用同量纲场景的经验值，不是该场景下的证明界。
    const RESIDUAL_EPS: f64 = 1e-9;

    /// 是否有残余（**清零判据**，非精确相等）：`|balance(account)| > RESIDUAL_EPS`。
    /// 收拢调用点原本手写的 `balance(..) != 0.0` 精确浮点比较——语义不变（真实
    /// 未清仓头寸远超 1e-9 量级，仍判 true），仅把「浮点末位抖动是否算清零」
    /// 这一判据常量收进 account 模块（接口归属，见 #400；未证实该抖动已在生产
    /// 发生，本方法不改变现有行为，只是给判据一个显式的、非精确相等的定义）。
    pub fn has_residual(&self, account: AccountIdentity) -> bool {
        self.balance(account).abs() > Self::RESIDUAL_EPS
    }

    /// 历史时点余额（**派生视图**：自成交事件日志重放，无快照存储）。
    pub fn balance_as_of(&self, account: AccountIdentity, bar: usize) -> f64 {
        self.fills
            .iter()
            .filter(|f| f.order.account() == account && f.fill_bar <= bar)
            .map(|f| f.order.qty_delta)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::super::coverage::Vertical;
    use super::super::interp::{EntryCertificate, PositionNodeId};
    use super::super::voice::VoiceSide;
    use super::*;
    use crate::theta_v0::classifier::recursive_tower::ElementId;

    fn pos(level: u32, ordinal: u64, side: VoiceSide, generation: u32) -> PositionNodeId {
        PositionNodeId {
            carrier: ElementId { level, ordinal },
            entry_certificate: Some(EntryCertificate {
                level,
                source_index: 0,
            }),
            side,
            generation,
        }
    }

    fn order(
        account: AccountIdentity,
        level: u32,
        side: VoiceSide,
        reason: ActionReason,
        qty_delta: f64,
        bar: usize,
    ) -> AccountOrder {
        AccountOrder {
            key: AccountKey::new(account, level, pos(level, 0, side, 0)),
            reason,
            qty_delta,
            decision_bar: bar,
        }
    }

    /// ★断言5（#185「恰一账户」）：account 是单值 enum——禁 Option/集合/缺省。
    /// 穷尽匹配无通配臂即类型层证明（若 account 是 Option/Vec，此形态无法编译）。
    #[test]
    fn account_order_carries_exactly_one_account() {
        let o = order(
            AccountIdentity::Core { level: 1 },
            1,
            VoiceSide::Long,
            ActionReason::Open,
            10.0,
            0,
        );
        let tag = match o.account() {
            AccountIdentity::Core { level } => {
                assert_eq!(level, 1);
                "core"
            }
            AccountIdentity::ReverseOpen { .. } => "reverse_open",
            AccountIdentity::Short => "short",
        };
        assert_eq!(tag, "core");
    }

    /// ★#194 完备性：六动作 → 三身份互斥完备映射。
    /// 开多/平多→Core、开空/平空→Short（无父反向声部，CONTEXT.md 反向根）、首开反向开/平→ReverseOpen。
    #[test]
    fn identity_of_covers_six_action_classes() {
        // 开多/平多：ambient 多根、顺父级联核心（FollowParent 两方向皆级联核心仓，#185 修复 a）。
        assert_eq!(
            identity_of(Vertical::Ambient, VoiceSide::Long, 1),
            Some(AccountIdentity::Core { level: 1 })
        );
        assert_eq!(
            identity_of(Vertical::FollowParent, VoiceSide::Long, 0),
            Some(AccountIdentity::Core { level: 0 })
        );
        assert_eq!(
            identity_of(Vertical::FollowParent, VoiceSide::Short, 0),
            Some(AccountIdentity::Core { level: 0 })
        );
        // 开空/平空：无父反向声部（ambient 空根）→ Short。
        assert_eq!(
            identity_of(Vertical::Ambient, VoiceSide::Short, 2),
            Some(AccountIdentity::Short)
        );
        // 首开反向开/平：反父子声部（两方向皆首开反向——父空时首开反向做多，CONTEXT.md）。
        assert_eq!(
            identity_of(Vertical::ReverseOpen, VoiceSide::Short, 0),
            Some(AccountIdentity::ReverseOpen { level: 0 })
        );
        assert_eq!(
            identity_of(Vertical::ReverseOpen, VoiceSide::Long, 0),
            Some(AccountIdentity::ReverseOpen { level: 0 })
        );
        // Flat 方向候选归 𝒦 记录不开腿 ⟹ 任何 entry_v × Flat 皆无账户身份（诚实 None，不伪造归属）。
        assert_eq!(identity_of(Vertical::Ambient, VoiceSide::Flat, 0), None);
        assert_eq!(identity_of(Vertical::ReverseOpen, VoiceSide::Flat, 0), None);
        assert_eq!(
            identity_of(Vertical::FollowParent, VoiceSide::Flat, 0),
            None
        );
    }

    /// ★理由轴与账户正交：同一理由可落不同账户，同一账户可带不同理由
    /// （不再用 ExitType 同时表达两者——#185 修复 a 的 FollowParent=Core{level}+StructuralPrune）。
    #[test]
    fn reason_axis_is_orthogonal_to_account() {
        let core_prune = order(
            AccountIdentity::Core { level: 0 },
            0,
            VoiceSide::Long,
            ActionReason::StructuralPrune,
            -5.0,
            3,
        );
        let sd_prune = order(
            AccountIdentity::ReverseOpen { level: 0 },
            0,
            VoiceSide::Short,
            ActionReason::StructuralPrune,
            5.0,
            3,
        );
        assert_eq!(core_prune.reason, sd_prune.reason, "同一理由（结构剪枝）");
        assert_ne!(core_prune.account(), sd_prune.account(), "不同账户（正交）");
        // 反向触发类 → 理由（保留 ExitType 在 CloseRoot 下坍缩的一类/二类之分）。
        assert_eq!(reason_of_reverse(1), Some(ActionReason::ReverseType1));
        assert_eq!(reason_of_reverse(2), Some(ActionReason::ReverseType2));
        assert_eq!(reason_of_reverse(3), Some(ActionReason::ReverseType3));
        assert_eq!(
            reason_of_reverse(0),
            None,
            "触发类 0 非合法反向类（诚实 None）"
        );
        assert_eq!(reason_of_reverse(4), None);
    }

    /// ★(account, level) 键全链携带：订单 → pending fill → 持仓/成本基 → 成交结果。
    #[test]
    fn key_carried_through_full_chain() {
        let mut book = ParallelAccountLedger::new();
        let o = order(
            AccountIdentity::Core { level: 1 },
            1,
            VoiceSide::Long,
            ActionReason::Open,
            10.0,
            2,
        );
        book.submit(o);
        // 链 stage 1+2：订单日志与 pending 均携同一 (account, level, position) 键。
        assert_eq!(book.orders().len(), 1);
        assert_eq!(book.pending().len(), 1);
        assert_eq!(book.orders()[0].key, o.key);
        assert_eq!(book.pending()[0].key, o.key);
        book.fill(o, 100.0, 3).expect("已提交订单可成交");
        // 链 stage 3+4：持仓实例按 (account, level, position) 分实例，成本基 = 入场名义额。
        let inst = book.instance(&o.key).expect("成交后实例存在");
        assert_eq!(inst.key, o.key);
        assert_eq!(inst.qty, 10.0);
        assert_eq!(inst.cost_basis, 1000.0, "成本基 = qty × fill_px");
        assert!(inst.open);
        // 链 stage 5：成交结果携同一键；pending 出队。
        assert_eq!(book.fills().len(), 1);
        assert_eq!(book.fills()[0].order.key, o.key);
        assert!(book.pending().is_empty(), "成交后 pending 出队");
    }

    /// ★链纪律：不能成交未提交的订单（pending fill 是真实阶段，非排版装饰）。
    #[test]
    fn fill_rejects_order_not_submitted() {
        let mut book = ParallelAccountLedger::new();
        let o = order(
            AccountIdentity::Short,
            0,
            VoiceSide::Short,
            ActionReason::Open,
            -3.0,
            0,
        );
        assert_eq!(
            book.fill(o, 100.0, 1),
            Err(AccountChainError::NotPending),
            "未 submit 直接 fill ⟹ 拒"
        );
        book.submit(o);
        book.fill(o, 100.0, 1).expect("提交后可成交");
        assert_eq!(
            book.fill(o, 100.0, 1),
            Err(AccountChainError::NotPending),
            "同一订单二次成交 ⟹ 拒（已出队）"
        );
    }

    /// ★并行记账视图：按（账户, 级别, 仓位节点）分实例记账，三身份余额可读。
    #[test]
    fn view_posts_per_instance_and_reads_three_identities() {
        let mut book = ParallelAccountLedger::new();
        // Core{1} 多根（父仓）、ReverseOpen 空子腿（父多在册）、Short 空根（无父反向声部）。
        let core_open = order(
            AccountIdentity::Core { level: 1 },
            1,
            VoiceSide::Long,
            ActionReason::Open,
            10.0,
            0,
        );
        let sd_open = order(
            AccountIdentity::ReverseOpen { level: 0 },
            0,
            VoiceSide::Short,
            ActionReason::Open,
            -4.0,
            1,
        );
        let short_open = order(
            AccountIdentity::Short,
            0,
            VoiceSide::Short,
            ActionReason::Open,
            -6.0,
            2,
        );
        book.post(core_open, 100.0, 0);
        book.post(sd_open, 100.0, 1);
        book.post(short_open, 100.0, 2);
        // 三身份余额（派生读数：多正空负）。
        assert_eq!(book.balance(AccountIdentity::Core { level: 1 }), 10.0);
        assert_eq!(
            book.balance(AccountIdentity::ReverseOpen { level: 0 }),
            -4.0
        );
        assert_eq!(book.balance(AccountIdentity::Short), -6.0);
        // 分实例：同身份不同仓位节点各记各账。
        let sd_open2 = order(
            AccountIdentity::ReverseOpen { level: 0 },
            0,
            VoiceSide::Short,
            ActionReason::Open,
            -2.0,
            3,
        );
        let sd_key2 = AccountKey::new(
            AccountIdentity::ReverseOpen { level: 0 },
            0,
            pos(0, 1, VoiceSide::Short, 0),
        );
        let sd_open2 = AccountOrder {
            key: sd_key2,
            ..sd_open2
        };
        book.post(sd_open2, 100.0, 3);
        assert_eq!(
            book.balance(AccountIdentity::ReverseOpen { level: 0 }),
            -6.0,
            "聚合 = 两实例之和（派生）"
        );
        assert_eq!(
            book.instance(&sd_open.key).unwrap().qty,
            -4.0,
            "实例一不被实例二污染"
        );
        assert_eq!(book.instance(&sd_key2).unwrap().qty, -2.0);
    }

    /// ★issue #357：`cost_basis` 聚合读数与 `balance` 同规格——同身份多实例求和，
    /// 减仓按均价法释放后聚合值同步下修（本仓口径落地取数入口的正确性证据）。
    #[test]
    fn cost_basis_aggregates_per_identity_like_balance() {
        let mut book = ParallelAccountLedger::new();
        let open = order(
            AccountIdentity::Core { level: 3 },
            3,
            VoiceSide::Long,
            ActionReason::Open,
            300.0,
            0,
        );
        book.post(open, 10.0, 0); // 300 股 @10 ⟹ cost_basis=3000
        assert_eq!(book.cost_basis(AccountIdentity::Core { level: 3 }), 3_000.0);
        assert_eq!(book.balance(AccountIdentity::Core { level: 3 }), 300.0);

        // 第二仓位节点同身份加总：另一个 carrier 的 Core{3} 仓，聚合求和（同 balance 口径）。
        let open2_key = AccountKey::new(
            AccountIdentity::Core { level: 3 },
            3,
            pos(3, 1, VoiceSide::Long, 0),
        );
        let open2 = AccountOrder {
            key: open2_key,
            reason: ActionReason::Open,
            qty_delta: 100.0,
            decision_bar: 1,
        };
        book.post(open2, 20.0, 1); // 100 股 @20 ⟹ cost_basis=2000
        assert_eq!(
            book.cost_basis(AccountIdentity::Core { level: 3 }),
            5_000.0,
            "两实例成本基求和=3000+2000"
        );
        assert_eq!(book.balance(AccountIdentity::Core { level: 3 }), 400.0);

        // 部分平仓按均价法释放成本基：第一仓位节点减 100 股（均价10）⟹ 成本基降 1000。
        let half_close = AccountOrder {
            reason: ActionReason::ReverseType1,
            qty_delta: -100.0,
            decision_bar: 2,
            ..open
        };
        book.post(half_close, 15.0, 2);
        assert_eq!(
            book.cost_basis(AccountIdentity::Core { level: 3 }),
            4_000.0,
            "部分平仓释放成本基=1000（100股×均价10），聚合读数随之同步下修=5000-1000"
        );
    }

    /// ★聚合仅派生视图（用户裁定 2026-07-23）：余额是现算投影，不是 canonical 存储；
    /// 历史时点余额自事件日志重放推导，不留总余额字段。
    #[test]
    fn aggregate_is_derived_not_canonical() {
        let mut book = ParallelAccountLedger::new();
        let open = order(
            AccountIdentity::Core { level: 0 },
            0,
            VoiceSide::Long,
            ActionReason::Open,
            10.0,
            2,
        );
        let close = AccountOrder {
            reason: ActionReason::ReverseType1,
            qty_delta: -10.0,
            decision_bar: 5,
            ..open
        };
        book.post(open, 100.0, 3);
        // 事件后余额立即反映（现算，非缓存）。
        assert_eq!(book.balance(AccountIdentity::Core { level: 0 }), 10.0);
        book.post(close, 110.0, 6);
        assert_eq!(
            book.balance(AccountIdentity::Core { level: 0 }),
            0.0,
            "一类点全平后 Core(level)=0（断言2视图层前身）"
        );
        // 历史时点自 fills 重放：开前 0、开后 10、平后 0——派生，无快照存储。
        assert_eq!(
            book.balance_as_of(AccountIdentity::Core { level: 0 }, 2),
            0.0
        );
        assert_eq!(
            book.balance_as_of(AccountIdentity::Core { level: 0 }, 3),
            10.0
        );
        assert_eq!(
            book.balance_as_of(AccountIdentity::Core { level: 0 }, 6),
            0.0
        );
        // 实例保留（canonical 分实例事实）：qty=0、已实现盈亏入账。
        let inst = book.instance(&open.key).unwrap();
        assert!(!inst.open, "全平后实例关闭");
        assert_eq!(inst.realized_pnl, 100.0, "已实现 = 10 × (110 − 100)");
        assert_eq!(inst.cost_basis, 0.0, "全平后成本基释放");
    }

    /// #400 边界钉死：容差内（|qty| < RESIDUAL_EPS=1e-9）判无残余——直接构造一个
    /// qty=9.9e-10 的实例（不经 post，避免真实开平仓路径无法精确命中边界值）。
    #[test]
    fn has_residual_false_within_tolerance() {
        let mut book = ParallelAccountLedger::new();
        let key = AccountKey::new(
            AccountIdentity::Core { level: 0 },
            0,
            pos(0, 0, VoiceSide::Long, 0),
        );
        book.instances.insert(
            key,
            AccountInstance {
                key,
                qty: 9.9e-10,
                cost_basis: 0.0,
                realized_pnl: 0.0,
                open: true,
            },
        );
        assert!(
            !book.has_residual(AccountIdentity::Core { level: 0 }),
            "9.9e-10 < RESIDUAL_EPS，应判无残余"
        );
    }

    /// #400 边界钉死：容差外（|qty| > RESIDUAL_EPS=1e-9）判有残余——防将来有人
    /// 改动容差值或把 `>` 误写成 `>=` 而不被任何测试捕获。
    #[test]
    fn has_residual_true_beyond_tolerance() {
        let mut book = ParallelAccountLedger::new();
        let key = AccountKey::new(
            AccountIdentity::Core { level: 0 },
            0,
            pos(0, 0, VoiceSide::Long, 0),
        );
        book.instances.insert(
            key,
            AccountInstance {
                key,
                qty: 1.1e-9,
                cost_basis: 0.0,
                realized_pnl: 0.0,
                open: true,
            },
        );
        assert!(
            book.has_residual(AccountIdentity::Core { level: 0 }),
            "1.1e-9 > RESIDUAL_EPS，应判有残余"
        );
    }

    /// ★#199 分流单源（红→绿）：二类反向理由按账户分流——核心腿「仅残余才纠错」
    /// （CoreResidualCorrection 仅在核心残余实测非零时触发，否则二类不得生成本仓卖单）；
    /// 首开反向/空根腿保留 ReverseType2（ID-3：二类合法卖出仅短差、开空两身份——短差名
    /// 随修1 废止，#281 更名首开反向（#283 实装）；ID-3 为教义引用保留原词）。
    #[test]
    fn reason_of_reverse_close_splits_type2_by_account() {
        // Core{level} × 二类：残余实测非零 ⟹ CoreResidualCorrection；残余零 ⟹ None（不生本仓卖单）。
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Core { level: 1 }, 2, true),
            Some(ActionReason::CoreResidualCorrection)
        );
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Core { level: 1 }, 2, false),
            None,
            "二类 + 核心残余为零 ⟹ 不得生成本仓卖单（ID-3 行为口径）"
        );
        // ReverseOpen/Short × 二类 ⟹ ReverseType2（合法二类卖：首开反向平/平空）。
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::ReverseOpen { level: 0 }, 2, true),
            Some(ActionReason::ReverseType2)
        );
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Short, 2, false),
            Some(ActionReason::ReverseType2),
            "Short 账户二类卖与核心残余读数无关（平空恒合法）"
        );
        // 一/三类账户中立（委托 reason_of_reverse 单源，不镜像判据）。
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Core { level: 0 }, 1, false),
            Some(ActionReason::ReverseType1)
        );
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::ReverseOpen { level: 0 }, 3, true),
            Some(ActionReason::ReverseType3)
        );
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Short, 1, true),
            Some(ActionReason::ReverseType1)
        );
        // 非法触发类 ⟹ None（不伪造理由，与 reason_of_reverse 同口径）。
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Core { level: 0 }, 0, true),
            None
        );
        assert_eq!(
            reason_of_reverse_close(AccountIdentity::Short, 4, true),
            None
        );
    }

    /// ★#199 断言③的分流层形态：二类产物账户约束——`ReverseType2` 永不落 Core 账
    /// （二类卖仅 ReverseOpen/Short 两身份）；Core 账的二类产物仅 CoreResidualCorrection。
    #[test]
    fn type2_reason_never_lands_on_core_as_reverse_type2() {
        for account in [
            AccountIdentity::Core { level: 0 },
            AccountIdentity::Core { level: 2 },
            AccountIdentity::ReverseOpen { level: 0 },
            AccountIdentity::Short,
        ] {
            for residual in [true, false] {
                let Some(r) = reason_of_reverse_close(account, 2, residual) else {
                    continue;
                };
                if r == ActionReason::ReverseType2 {
                    assert!(
                        !matches!(account, AccountIdentity::Core { .. }),
                        "断言③：二类卖（ReverseType2）不得落 Core 账（{account:?}）"
                    );
                }
                if matches!(account, AccountIdentity::Core { .. }) {
                    assert_eq!(
                        r,
                        ActionReason::CoreResidualCorrection,
                        "Core 账二类产物仅 CoreResidualCorrection（残余纠错）"
                    );
                }
            }
        }
    }

    /// ★#200 OpenShort 通道理由归属（spec WP-2 修复 c）：二类 × Short 账户 ⟹ OpenShort；
    /// 其余（账户 × 触发类）全保留 Open——OpenShort 只标二类开空这一通道（单源判据）。
    #[test]
    fn reason_of_open_marks_only_type2_short_account_as_open_short() {
        // 二类 × Short（无父空根）⟹ OpenShort（二类开空通道）。
        assert_eq!(
            reason_of_open(AccountIdentity::Short, 2),
            ActionReason::OpenShort
        );
        // 一/三类 × Short ⟹ Open（一类首开反向仓标准位、三类不构成开空通道）。
        assert_eq!(
            reason_of_open(AccountIdentity::Short, 1),
            ActionReason::Open
        );
        assert_eq!(
            reason_of_open(AccountIdentity::Short, 3),
            ActionReason::Open
        );
        // 二类 × ReverseOpen ⟹ Open（首开反向开是 B 账户既有通道，非 OpenShort）。
        assert_eq!(
            reason_of_open(AccountIdentity::ReverseOpen { level: 0 }, 2),
            ActionReason::Open
        );
        // 二类 × Core ⟹ Open（FollowParent 级联加仓 / Ambient 多根第二入场，非 C 账户）。
        assert_eq!(
            reason_of_open(AccountIdentity::Core { level: 0 }, 2),
            ActionReason::Open
        );
        assert_eq!(
            reason_of_open(AccountIdentity::Core { level: 1 }, 2),
            ActionReason::Open
        );
        // 非法触发类不享受通道标注（与 reason_of_reverse 同口径的防御域）。
        assert_eq!(
            reason_of_open(AccountIdentity::Short, 0),
            ActionReason::Open
        );
    }

    /// ★减仓口径：部分平仓按比例释放成本基（均价法）。
    #[test]
    fn partial_close_releases_cost_basis_pro_rata() {
        let mut book = ParallelAccountLedger::new();
        let open = order(
            AccountIdentity::Short,
            1,
            VoiceSide::Short,
            ActionReason::Open,
            -6.0,
            0,
        );
        let half_close = AccountOrder {
            reason: ActionReason::ReverseType3,
            qty_delta: 3.0,
            decision_bar: 1,
            ..open
        };
        book.post(open, 100.0, 0);
        book.post(half_close, 90.0, 1);
        let inst = book.instance(&open.key).unwrap();
        assert_eq!(inst.qty, -3.0);
        assert_eq!(inst.cost_basis, 300.0, "余 3 手 @100 的成本基");
        assert_eq!(inst.realized_pnl, 30.0, "空向已实现 = 3 × (100 − 90)");
        assert!(inst.open);
    }

    /// ★#237 断言②按方向拆查的账面投影（与断言①同源同口径，#234 蓝图依据：分侧账
    /// 禁净额 P_sep——净额同时掩盖双侧残余，ker N 不可识别；用户裁 2026-07-24）：
    /// `balance_side` 按（账户 × 声部方向）分量读余额——一类卖批查**多侧**分量、
    /// 一类买批查**空侧**分量。顺父级联 Short 腿归空侧（实例键 `position.side`，
    /// #185 映射不动）；净额 `balance` 口径不动（两派生视图共存）。
    ///
    /// **RED**：`balance_side` 缺席（编译红）。**GREEN**：分侧投影 + 卖批全平多侧后
    /// 多侧=0、空侧存活（净额≠0 = 旧口径误报面的最小复现，本票收口）。
    #[test]
    fn balance_side_splits_account_balance_by_voice_side() {
        let mut book = ParallelAccountLedger::new();
        // Core{0} 双侧在册：多侧 +100（Ambient×Long 根）＋ 空侧 −30（FollowParent×Short
        // 顺父级联——账户同归 Core{0}、方向由实例键 position.side 表达）。
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Long,
                ActionReason::Open,
                100.0,
                0,
            ),
            10.0,
            0,
        );
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Short,
                ActionReason::Open,
                -30.0,
                0,
            ),
            10.0,
            0,
        );
        // 分侧投影：多侧 +100 / 空侧 −30；净额 +70（双侧相互掩盖）。
        assert_eq!(
            book.balance_side(AccountIdentity::Core { level: 0 }, VoiceSide::Long),
            100.0
        );
        assert_eq!(
            book.balance_side(AccountIdentity::Core { level: 0 }, VoiceSide::Short),
            -30.0
        );
        assert_eq!(
            book.balance(AccountIdentity::Core { level: 0 }),
            70.0,
            "净额口径不动（派生视图共存）"
        );
        // 一类卖批全平多侧（ReverseType1 平多 −100）：多侧分量=0（卖批查零过）、
        // 空侧 −30 合法存活（不计入卖批）；净额 −30 ≠ 0 = 旧净额口径误报面。
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Long,
                ActionReason::ReverseType1,
                -100.0,
                1,
            ),
            10.0,
            1,
        );
        assert_eq!(
            book.balance_side(AccountIdentity::Core { level: 0 }, VoiceSide::Long),
            0.0,
            "一类卖批 ⇒ 该级多侧分量=0（S2「该级该方向」良构形式）"
        );
        assert_eq!(
            book.balance_side(AccountIdentity::Core { level: 0 }, VoiceSide::Short),
            -30.0,
            "顺父级联空侧腿卖批下合法存活（同向信号持有），不计入卖批"
        );
        assert_ne!(
            book.balance(AccountIdentity::Core { level: 0 }),
            0.0,
            "净额≠0——旧标量查零在此误报（m3 (1,470) 同型），拆查后由空侧分量承担"
        );
        // 一类买批全平空侧（ReverseType1 平空 +30）：空侧分量=0（买批查零过）。
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Short,
                ActionReason::ReverseType1,
                30.0,
                2,
            ),
            10.0,
            2,
        );
        assert_eq!(
            book.balance_side(AccountIdentity::Core { level: 0 }, VoiceSide::Short),
            0.0,
            "一类买批 ⇒ 该级空侧分量=0（与卖批镜像）"
        );
    }

    /// ★issue #357 关票条件 C：`cost_basis_side` 与 `balance_side` 同一过滤谓词——多空并存时
    /// 多头侧成本基不被空侧污染（`cost_basis(account)` 净额聚合会把两侧成本基相加，本方法
    /// 拆查后各自独立读数，喂 `drive_campaign_wiring` 的多头侧 `CoreCostBasisSnapshot`）。
    #[test]
    fn cost_basis_side_splits_by_voice_side_like_balance_side() {
        let mut book = ParallelAccountLedger::new();
        // Core{0} 多侧 100 股 @10（成本基1000）+ 空侧 30 股 @20（成本基600，顺父级联 Short 腿）。
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Long,
                ActionReason::Open,
                100.0,
                0,
            ),
            10.0,
            0,
        );
        book.post(
            order(
                AccountIdentity::Core { level: 0 },
                0,
                VoiceSide::Short,
                ActionReason::Open,
                -30.0,
                0,
            ),
            20.0,
            0,
        );
        assert_eq!(
            book.cost_basis_side(AccountIdentity::Core { level: 0 }, VoiceSide::Long),
            1_000.0,
            "多侧成本基独立读数，不含空侧"
        );
        assert_eq!(
            book.cost_basis_side(AccountIdentity::Core { level: 0 }, VoiceSide::Short),
            600.0,
            "空侧成本基独立读数，不含多侧"
        );
        assert_eq!(
            book.cost_basis(AccountIdentity::Core { level: 0 }),
            1_600.0,
            "净额聚合口径不动（两派生视图共存）——正是本条件要规避的污染源"
        );
    }
}
