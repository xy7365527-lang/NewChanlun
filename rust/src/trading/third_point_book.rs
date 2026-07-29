//! 三类点成立登记账 —— 交易层消费买卖点账本**成立档**的入口（票 #638；#575 消费方接线 2/3）。
//!
//! # 一句话
//!
//! 账本判胜 ⟹ [`ThirdPointPack`]（身份 + 侧 + 四条边转正快照 + leave/retest 双位置 + 双知情时
//! + 死亡证明）⟹ 本账**照实登记**（幂等 + 改口 fail-loud）+ 只读观测面。登记不判、不滤、不投影：
//! 交易层在此只做「在册」，**不做「可不可买」**。
//!
//! # 为什么落在这里（票面点名：核 `spiral/signal.rs`、`trading/runner.rs` 两族，选与 #587 迟到
//! 过滤同侧的位置，理由如下——修复轮补核 `spiral/signal.rs`，候选集补齐为四）
//!
//! 开工亲核的交易层信号通道现状（2026-07-29，本 worktree；修复轮 2026-07-29 补核
//! `spiral/signal.rs`）：
//!
//! - `crate::trading` 是**私有模块**（`lib.rs:49` `mod trading;`），`runner.rs` 头注声明
//!   `run_organic` 控制流逐字移植 Python `organic_fugue.run_organic`、是 **V0≡P5 逐位等价的
//!   承重面**；
//! - 交易层的 BSP 事件源 = [`super::tape`] 的 legacy Python-parity 磁带（`SignalTape`），**不是**
//!   `theta_v0` 塔链；入场判据读的是**布尔行** `sig.buy1` / `sig.buy_any`（`runner.rs:807`/`:824`），
//!   压根不经 [`super::types::BspEvent`]；
//! - 接线前，`trading → theta_v0` 的唯一生产接触面是 `center_book.rs:35`（票 #637 引入的死亡
//!   证明消费面）；
//! - `crate::spiral::signal`（票面点名的另一族，`rust/src/spiral/signal.rs`）自称「信号层是
//!   操作层的**唯一定义域单元**」（`signal.rs:3`），消费同一份 legacy 磁带（`signal.rs:24-27`
//!   `use crate::trading::{center_book::CenterBook, tape::BarSig, types::BspEvent}`）；
//!   `SignalState::process`（`signal.rs:173-310`）内含 `prove_t53_connection_assoc`（:288-289）/
//!   `prove_n5_cascade`（:299-300）两条 **panic 级**结构守卫；`prove_chain`（定义于 `signal.rs:317`）
//!   不在 `process` 内，由 `spiral/engine.rs` 逐 bar 调用（:175/:209/:302），是 V0≡P5 parity 的
//!   另一段承重路径；
//!   它持有的 `CenterBook`（`signal.rs:157` `book: CenterBook::new()`）是自己私有的实例，喂的是
//!   legacy `BspEvent` 行——`spiral/` 目录全文对 `RetraceLedger`/`ThirdPointPack`/`StandbyWatch`
//!   **零 import**（grep 实证），与 `theta_v0::classifier::retrace_ledger` 目前零接触面。
//!
//! 据此定位本票的接入位置 = **`trading/` 下新独立模块（本文件）**，理由四条（候选 A/B/C/D 全覆盖）：
//!
//! 1. **与 #587 同侧 = 观测登记侧，不是入场判据侧**。#587 对迟到三类点的裁定是「登记观测，不作
//!    独立信号消费」（ADR-0001 补充十六，`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:273`，
//!    开工亲核原文）。本票交付的正是那个「登记观测」面，故它必须落在**不参与入场判据**的位置。
//!    挂 `runner.rs` 主循环（候选 C）是入场判据侧，且触碰 V0≡P5 parity 承重面，方向相反；
//! 2. **不占用中枢账词汇**。`CenterBook`（候选 A）说的是**中枢生死**，本票登记的是**三类点成立
//!    本体事件**——同一教义事件的不同断言面（见下 §语义边界）。把「三类点成立」塞进中枢账要
//!    扩它的只读面并混两套词汇；
//! 3. **隔离最净**。本模块（候选 B）零入边（不被 `runner`/`master`/`allocator` 任何主链模块
//!    `use`）、只依赖 `theta_v0` 账本的公开产出面，不改 parity 面一行；
//! 4. **候选 D（`spiral::signal`）排除，理由与候选 C 同族**：它自述「操作层唯一定义域单元」——
//!    按定义就是判据侧，方向与候选 C 相同、与 #587「登记观测侧」相反；结构上
//!    `SignalState::process` 是逐 bar 热路径 + panic 级结构守卫，插入登记逻辑等于新开一条
//!    `theta_v0 → spiral` 的边，而这条边目前**不存在**（零 import，见上）——比候选 B（零入边，
//!    只挂账本公开产出面）风险高一个数量级，且它内部的 `CenterBook` 是喂 legacy `BspEvent`
//!    的私有实例，与 `theta_v0` 的 [`ThirdPointPack`] 是两套不相干的数据源，接入前还得先立一条
//!    本不存在的桥。
//!
//! # 两项裁定（编排者 2026-07-29，本票据此实施）
//!
//! - **裁定一（迟到判据）**：交易层把 [`ThirdPointPack`] **一律照实登记**，**不新造迟到判据**
//!   ——见下 §迟到处置；「迟到判据长什么样」归 **#680**。**关系澄清（修复轮）**：#680 票面
//!   `blocked_by` 本票，这是 GitHub 依赖边（本票的登记入口是 #680 判据落笔的前提对象）；但
//!   #680 自己声明的**前置条件**是 `RetraceLedger` 接生产驱动、有真实 pack 数据流（#575 后续
//!   票），本票只是缘起 / 登记面提供者——不要把 `blocked_by` 读成「#638 是 #680 的前置条件」。
//! - **裁定二（接线口径）**：[`super::super::theta_v0::classifier::retrace_ledger::RetraceLedger`]
//!   的**生产驱动源**（喂真实 replay 事件）归 #575 后续票，本票**不立生产驱动挂点**；票面「生产
//!   调用点 ≥1（grep 可证）」按字面结 = 类型 [`ThirdPointPack`] / 方法 `established_pack` 出现于
//!   生产代码即达标（同 #637 裁定 3A / #639 裁定 B 先例）。本模块是该出现位置（非
//!   `#[cfg(test)]`），**不是**「已被生产调用方驱动」意义上的达标——端到端由靶向测试（测试内经
//!   `RetraceLedger::observe` 真实产 Confirmed）覆盖。
//!
//! # 迟到处置口径（#587 字面 + 本票无量化判据）
//!
//! 本账对 `confirmed_as_of`（落锤知情时）**不设任何判定分支**：无阈值、无「当前时刻」入参、
//! 无排序/丢弃规则。迟到的处置 = **照实在册 + 不产独立信号**（#587 字面：「不追……登记观测，
//! 不作独立信号消费」）——「迟到」由「照登记且本账不产信号」这一行为本身表达，与 #637
//! `CenterBook::consume_death_certificate` 的「不问迟到」同构。
//!
//! **为什么本票不给量化判据**（如实登记，非遗漏）：判「迟到」需要一个与 `confirmed_as_of`
//! 可比的当下时钟坐标，而交易层的 bar 坐标（磁带行号）与 `theta_v0` 账本的知情时是**两套引擎
//! 各自的坐标**，仓内无对齐依据；同族约束见票 #637 裁定 2A（`RetracePoint.price` 是 theta_v0
//! `Tick`（刻度计数）、trading 侧价格是原始 `f64`，**跨量纲比较禁止**）。故本账的任何「位置」
//! 与「时点」字段一律只登记、不比较。判据留白归 **#680**（票面已含 #637 2A 与时钟坐标约束）。
//!
//! # 禁区（#574 裁定八）：备战档不接
//!
//! 本模块**不 `use`、不登记、不接受**
//! [`super::super::theta_v0::classifier::retrace_ledger::StandbyWatch`]（备战档，未决候选）——
//! 裁定八：备战档不许被消费成买入信号。两道墙：
//!
//! 1. **入口 bound 即闸门**：[`ThirdPointBook::register`] 要求 `S: TradableSignal`
//!    （[`TradableSignal`] 只 [`ThirdPointPack`] 实现，`StandbyWatch` 故意不实现）
//!    **∧** `S: Into<ThirdPointPack>`（账本 `portal` 模块头明文声明两型互无 `From`/`Into`）——两条
//!    bound 各自独立否掉备战档，传入即编译失败；
//! 2. **字段表即墙（修复轮订正强度措辞）**：`StandbyWatch` 结构性没有 `retest_end` /
//!    `confirmed_as_of` / `death_certificate`，即便日后有人显式给它实现 `TradableSignal`
//!    （破坏性改动，非静默扩权），**也只能用假值凑出这三项载荷**——测试
//!    `standby_watch_is_barred_from_the_registration_entry` 的字段表全解构保证的是「字段表
//!    一变即编译失败」（活守卫 = **变更探测器**），不是「凑不出载荷」；真正挡住绕路的是
//!    ①目前 `TradableSignal` 只有 [`ThirdPointPack`] 一个实现者、②两型互无 `From`/`Into`，
//!    绕路需要显式新增 impl（可见的破坏性改动）——结构墙让绕路**显式**，不让绕路**不可能**。
//!
//! **编译期负向强制的位置声明（如实登记）**：`trading` 是私有模块，`compile_fail` doctest 以
//! **外部 crate** 身份编译，够不到本模块的私有路径——在此写 `compile_fail` 只会因「模块私有」
//! 而失败，是假证明，故本票**不写**。同一 trait 上的负向编译期强制由账本侧既有 doctest 承担
//! （`retrace_ledger/portal.rs:51-70`，`cargo test --doc` 覆盖：`place_order(watch)` 编译失败），
//! 本票的 bound 与它是同一道闸；本模块侧的等价证据 = 上述字段表全解构测试 + 路径面断言
//! （登记账内只有成立档身份）。
//!
//! # 语义边界：同一教义事件的第三条观测通道
//!
//! 18 课定理三充要条件下，「三类点成立」与「中枢死亡」是同一教义事件的两个断言面。仓内因此有
//! 三条通道，本账是第三条：
//!
//! | 通道 | 位置 | 登记的是 |
//! |---|---|---|
//! | ① | `CenterBook::ingest` 的 confirmed Type3 kill 分支 | 中枢死亡（自 BSP 事件锚 diff） |
//! | ② | `CenterBook::consume_death_certificate`（#637） | 中枢死亡（消费账本的外部证明） |
//! | ③ | 本账 [`ThirdPointBook::register`]（#638） | **三类点成立本体**（不发中枢死亡语义） |
//!
//! 本账**不发**任何中枢生死语义、**不动** `CenterBook` 的 `dead`/`frozen`/`version`、**不递**
//! `pack.death_certificate` 给 `CenterBook::consume_death_certificate`：后者按 ladder 消费
//! （`consume_death_certificate(ladder, cert)`），而 [`ThirdPointPack`] 不带 ladder/级别坐标
//! （级别在账本的 `RetraceProvenance.level` 上，与 trading 的 ladder 是两套引擎的坐标，仓内无
//! 对齐依据）。跨引擎级别坐标对齐 ⟹ #575 驱动票；本账只登记本体事件，三条通道互不代劳。
//!
//! **#664 关系声明**：[`ThirdPointPack`] 自带 `side`（买/卖侧），故 #664（`CenterDeathCertificate`
//! 无 `side` 的方向盲区）**不约束**本账的 pack 登记面——本账在册的每条成立档方向明确。若 #575
//! 驱动票日后把 `pack.death_certificate` 转投通道 ②，#664 盲区照旧适用（cert 本身仍无 side），
//! 走 #664 修，不在本票。
//!
//! # 本票不做（范围外，勿在此模块寻找）
//!
//! - **信号产出 / 过滤规则**：本账不产买入信号、不做迟到过滤、不做力度/位置判据（裁定一）；
//! - **生产驱动挂点**：无 `RetraceLedger` 生产实例、不进 `runner` 主循环（裁定二）；
//! - **备战档消费形态**：裁定八禁区，另裁（登记表 row 2' 保持「未接线」）；
//! - **账本侧任何改动**：`retrace_ledger/` 只改了消费方登记表的 doc 文字（row 2 + 计数）。
//!
//! # 名分（世代宪法 §1，`docs/agents/generation-constitution.md`）
//!
//! 本模块 = **新立消费入口，驱动链未接**（#575 后续票）。按 §1 机械判据，「现役」要求**有非
//! 测试调用者**，本模块当前只有测试调用者 ⟹ 字面落「生产零可达但有测试调用者」那一格，而该格
//! 的名分标签是「deprecated 待退役」——**语义方向相反**（本模块是新生入口，不是退役残留）。
//! 三面同构（#637 通道 ② / #639 短差通道 / 本票）均处此境。如实登记这处判据张力，**不自裁**：
//! 是否为「新生入口」补判据例外或立第五格，上浮宪法线（#500 图）。
//!
//! ⚠ 副作用登记：本模块的 `pub` 项在非测试构建下无调用者 ⟹ `cargo build` 报 `dead_code`
//! 警告（与 #639 的 `t3_in_c_grade_reason_to_pan_div_subtype` 等项同族，仓内既有先例）。
//! 驱动链接上即消失，本票不加 `#[allow(dead_code)]` 掩盖。

use std::collections::BTreeMap;

use crate::theta_v0::classifier::retrace_ledger::{
    RetraceKey, RetraceLedger, ThirdPointPack, TradableSignal,
};

/// 一次登记的判别（#637 [`super::center_book::DeathCertificateOutcome`] 同构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThirdPointRegistration {
    /// 首次登记：该身份此前不在册，全字段照实入册（`registrations` 前进）。
    Registered,
    /// 幂等确认：该身份已在册且内容逐字段相等——零动作（`idempotent_repeats` 前进）。
    AlreadyRegistered,
}

/// 登记冲突：fail-loud，不静默吞掉分歧（#637 `ConflictingCertificate` 同族）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThirdPointConflict {
    /// 同一身份收到两份内容不同的成立档 = 上游账本改口（账本侧的同族错误码是
    /// `NotConstitutedReason::CenterRebased`）。在册内容**不被覆盖**，冲突原样上报调用方。
    ConflictingPack { identity: RetraceKey },
}

/// 三类点成立登记账：幂等登记 + 只读观测面，零判据。
///
/// **不变量**：`established` 的每个值满足 `value.identity == key`（登记入口是唯一写入点，键取
/// `pack.identity`）；条数 == `registrations`（首次登记数）——幂等确认与冲突都不改 map。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThirdPointBook {
    /// 身份 → 成立档证据包（`BTreeMap` ⟹ 观测面按身份键序，确定性，与账本
    /// [`RetraceLedger::established`] 同一口径）。
    established: BTreeMap<RetraceKey, ThirdPointPack>,
    /// 首次登记次数（观测面：本账见过多少个不同的成立档身份）。
    pub registrations: u64,
    /// 幂等确认次数（重复消费同一身份同一内容的次数；显著非零 ⟹ 上游在重复投递）。
    pub idempotent_repeats: u64,
}

impl ThirdPointBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// 成立档消费入口（票 #638；#575 消费方接线 2/3）：**照实登记**一份成立档。
    ///
    /// **入口 bound = 禁区闸门**（模块头 §禁区第 1 道墙）：`S: TradableSignal` 只有
    /// [`ThirdPointPack`] 满足（[`TradableSignal`]，`StandbyWatch` 故意不实现），
    /// `S: Into<ThirdPointPack>` 由标准库自反 `From` 提供、账本 `portal` 模块头明文声明备战档与成立档
    /// 互无 `From`/`Into`。**闸门强度声明（修复轮订正）**：具体型入参 `pack: ThirdPointPack`
    /// 本身就是最强的编译期闸门，闸门强度与泛型无关；泛型在此把 `TradableSignal` 显式写进签名，
    /// 是**同强度**下把禁区 trait 变成可 grep 的教义锚，不是「比具体型入参更强的闸门」。
    ///
    /// **判定次序**：
    /// 1. 该身份已在册 ∧ 内容逐字段相等 ⟹ [`ThirdPointRegistration::AlreadyRegistered`]，零动作；
    /// 2. 该身份已在册 ∧ 内容不等 ⟹ [`ThirdPointConflict::ConflictingPack`]，在册内容不动
    ///    （fail-loud，见该错误码 doc）；
    /// 3. 否则入册 ⟹ [`ThirdPointRegistration::Registered`]。
    ///
    /// **不做**：不比较任何知情时、不与 bar 坐标对表、不产信号（裁定一 + 模块头 §迟到处置）。
    pub fn register<S>(&mut self, signal: S) -> Result<ThirdPointRegistration, ThirdPointConflict>
    where
        S: TradableSignal + Into<ThirdPointPack>,
    {
        let pack: ThirdPointPack = signal.into();
        if let Some(seen) = self.established.get(&pack.identity) {
            return if *seen == pack {
                self.idempotent_repeats += 1;
                Ok(ThirdPointRegistration::AlreadyRegistered)
            } else {
                Err(ThirdPointConflict::ConflictingPack {
                    identity: pack.identity,
                })
            };
        }
        self.established.insert(pack.identity, pack);
        self.registrations += 1;
        Ok(ThirdPointRegistration::Registered)
    }

    /// 全量登记：把账本当下的成立档集合（[`RetraceLedger::established`]）逐条过
    /// [`Self::register`]，返回**本次新登记**的证据包（按身份键序）。
    ///
    /// 幂等：账本不变时重复调用返回空 vec（已在册身份走幂等确认）。
    ///
    /// **首个冲突即返回，已登记部分保留**（如实声明，非疏漏）：本账是登记账，无回滚语义——
    /// 上游改口是事故，调用方拿到 [`ThirdPointConflict`] 后应查账本，而不是期待本账把在册事实
    /// 撤回。**`admitted` 清单在 `Err` 路径上随 `?` 整体丢弃**（修复轮补充声明）：冲突之前已
    /// 成功登记的身份**确实进了 [`Self::established`]**（`self` 是 `&mut`，写入不回滚），但
    /// 函数返回值只有 `Err`，拿不到那份清单——调用方想知道「这次到底新登记了谁」，应在拿到
    /// `Err` 后以 [`Self::established`] 重查在册状态，不要依赖本次调用的返回值。测试
    /// `sync_from_ledger_stops_at_first_conflict_but_keeps_prior_admissions` 钉死这条路径。
    ///
    /// **复杂度**：每次调用全量遍历账本 Confirmed 集合（O(账本成立档条目数)），**非增量**；
    /// 若 #575 驱动票把它接进逐帧循环，总量为 O(条目数 × 调用次数)，接入前应评估是否改增量读
    /// （同 #639 `drain_pan_div_short_retrace_observations` 的同款登记）。
    pub fn sync_from_ledger(
        &mut self,
        ledger: &RetraceLedger,
    ) -> Result<Vec<ThirdPointPack>, ThirdPointConflict> {
        let mut admitted = Vec::new();
        for pack in ledger.established() {
            if self.register(pack)? == ThirdPointRegistration::Registered {
                admitted.push(pack);
            }
        }
        Ok(admitted)
    }

    /// 单身份登记：读账本的单身份成立档门户（[`RetraceLedger::established_pack`]）。
    ///
    /// `Ok(None)` = 该身份在账本侧**不是** `Confirmed`（未决 / 判败 / 不存在）⟹ 照实不登记，
    /// 不报错——「没有成立档」不是事故。
    pub fn register_from_ledger(
        &mut self,
        ledger: &RetraceLedger,
        key: &RetraceKey,
    ) -> Result<Option<ThirdPointRegistration>, ThirdPointConflict> {
        match ledger.established_pack(key) {
            None => Ok(None),
            Some(pack) => self.register(pack).map(Some),
        }
    }

    // ── 观测只读面（零判据：全字段直读） ──

    /// 在册成立档全集（按身份键序，确定性——与账本 [`RetraceLedger::established`] 同口径）。
    pub fn established(&self) -> Vec<ThirdPointPack> {
        self.established.values().copied().collect()
    }

    /// 单身份在册证据包（全字段原样，含 `confirmed_as_of` 与死亡证明——供 #680 读取）。
    pub fn pack(&self, key: &RetraceKey) -> Option<ThirdPointPack> {
        self.established.get(key).copied()
    }

    pub fn contains(&self, key: &RetraceKey) -> bool {
        self.established.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.established.len()
    }

    pub fn is_empty(&self) -> bool {
        self.established.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::theta_v0::classifier::level_view::CoordinateWindow;
    use crate::theta_v0::classifier::retrace_ledger::{
        CenterFrame, RetraceInput, RetraceKey, RetraceLedger, RetraceOutcome, RetracePoint,
        RetraceProvenance, RetraceSide, StandbyWatch, StrictCompletedPair, ThirdPointPack,
        TradableSignal,
    };
    use crate::theta_v0::types::Direction;

    fn ledger() -> RetraceLedger {
        RetraceLedger::new(RetraceProvenance {
            level: 2,
            window: CoordinateWindow { start: 0, end: 9_999 },
            data_basis: "third_point_book-test".to_owned(),
        })
    }

    fn frame(start_index: usize) -> CenterFrame {
        CenterFrame {
            zd: 1_000,
            zg: 1_200,
            start_index,
            end_index: start_index + 10,
        }
    }

    fn key_of(center: CenterFrame, departure: usize) -> RetraceKey {
        RetraceKey {
            frame: center,
            departure_move_index: departure,
        }
    }

    /// 向上离开 + 回抽的观察（`outcome = None` 即未决，`Some(Success)` 即判胜落锤）。
    fn up_input(
        center: CenterFrame,
        departure: usize,
        outcome: Option<RetraceOutcome>,
        as_of: usize,
    ) -> RetraceInput {
        RetraceInput {
            center,
            pair: StrictCompletedPair {
                leave_move_index: departure,
                retest_move_index: departure + 1,
            },
            leave_direction: Direction::Up,
            retest_direction: Direction::Down,
            leave_end: RetracePoint {
                index: center.end_index + 10,
                price: center.zg + 50,
            },
            retest_end: Some(RetracePoint {
                index: center.end_index + 20,
                price: center.zg + 10,
            }),
            outcome,
            as_of,
        }
    }

    /// 真实产一份成立档：经 [`RetraceLedger::observe`] 判胜落锤，再从
    /// [`RetraceLedger::established_pack`] 取出——不手搓 [`ThirdPointPack`] 字面量
    /// （票面验收第 1 条「端到端」的正确用法，#637 先例同构）。
    fn confirmed_pack(
        book: &mut RetraceLedger,
        center: CenterFrame,
        departure: usize,
        as_of: usize,
    ) -> ThirdPointPack {
        book.observe(&up_input(center, departure, Some(RetraceOutcome::Success), as_of))
            .unwrap();
        book.established_pack(&key_of(center, departure))
            .expect("判胜落锤后必有成立档")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 端到端：产 Confirmed → pack → 交易层登记入口 → 观测面可见
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn established_pack_registers_end_to_end_verbatim() {
        let mut ledger = ledger();
        let pack = confirmed_pack(&mut ledger, frame(100), 3, 860);
        let mut book = ThirdPointBook::new();
        assert!(book.is_empty(), "新账空册");

        assert_eq!(book.register(pack), Ok(ThirdPointRegistration::Registered));
        assert!(!book.is_empty());
        // 照实登记（裁定一）：全字段原样在案，交易层不投影、不改写、不过滤。
        assert_eq!(book.pack(&pack.identity), Some(pack));
        assert_eq!(book.established(), vec![pack]);
        assert_eq!(book.established(), ledger.established(), "登记面 ≡ 账本产出面");
        assert_eq!(book.len(), 1);
        assert_eq!(book.registrations, 1);
        assert_eq!(book.idempotent_repeats, 0);
    }

    #[test]
    fn ledger_sync_registers_every_established_pack_in_key_order() {
        let mut ledger = ledger();
        let late = confirmed_pack(&mut ledger, frame(400), 11, 900);
        let early = confirmed_pack(&mut ledger, frame(100), 3, 920);
        let mut book = ThirdPointBook::new();

        let admitted = book.sync_from_ledger(&ledger).unwrap();
        assert_eq!(admitted, vec![early, late], "按身份键序，无平局歧义");
        assert_eq!(book.len(), 2);
        // 幂等：同一账本重复同步零新增（已登记身份走幂等确认）。
        assert_eq!(book.sync_from_ledger(&ledger).unwrap(), Vec::new());
        assert_eq!(book.registrations, 2);
        assert_eq!(book.idempotent_repeats, 2);
    }

    #[test]
    fn repeated_registration_of_the_same_pack_is_idempotent_zero_action() {
        let mut ledger = ledger();
        let pack = confirmed_pack(&mut ledger, frame(100), 3, 860);
        let mut book = ThirdPointBook::new();

        assert_eq!(book.register(pack), Ok(ThirdPointRegistration::Registered));
        assert_eq!(
            book.register(pack),
            Ok(ThirdPointRegistration::AlreadyRegistered)
        );
        assert_eq!(book.len(), 1);
        assert_eq!(book.registrations, 1, "首次登记数不因重复消费前进");
        assert_eq!(book.idempotent_repeats, 1);
    }

    #[test]
    fn conflicting_pack_for_the_same_identity_fails_loud() {
        let mut ledger = ledger();
        let pack = confirmed_pack(&mut ledger, frame(100), 3, 860);
        let mut book = ThirdPointBook::new();
        assert_eq!(book.register(pack), Ok(ThirdPointRegistration::Registered));

        // 负控手搓：同身份、内容不同 = 上游账本改口（#637 ConflictingCertificate 同族）。
        let tampered = ThirdPointPack {
            confirmed_as_of: pack.confirmed_as_of + 7,
            ..pack
        };
        assert_eq!(
            book.register(tampered),
            Err(ThirdPointConflict::ConflictingPack {
                identity: pack.identity
            })
        );
        assert_eq!(book.pack(&pack.identity), Some(pack), "在册内容不被改口覆盖");
    }

    /// 影子 MEDIUM-2：`sync_from_ledger` 的冲突早退是模块唯一「带早退 + 留下部分副作用」的
    /// 路径，此前零测试覆盖。钉三件事：① 命中冲突即 `Err`；② 冲突身份在册旧内容不被账本侧
    /// 新内容覆盖；③ 冲突之前（键序更早）已成功登记的身份**保留在册**——`admitted` 返回清单
    /// 随 `?` 丢弃，但对 `self.established` 的写入不回滚（doc `:sync_from_ledger` 已补声明）。
    #[test]
    fn sync_from_ledger_stops_at_first_conflict_but_keeps_prior_admissions() {
        let mut ledger = ledger();
        // 键序：frame(100) < frame(400)（start_index 排前）——earlier 先于 later 被 sync 处理。
        let earlier = confirmed_pack(&mut ledger, frame(100), 3, 860);
        let later = confirmed_pack(&mut ledger, frame(400), 11, 900);

        let mut book = ThirdPointBook::new();
        // 预登记 later 身份的旧内容（与账本侧稍后产出的 later 内容不同 = 改口冲突源）。
        let stale = ThirdPointPack {
            confirmed_as_of: later.confirmed_as_of + 7,
            ..later
        };
        assert_eq!(book.register(stale), Ok(ThirdPointRegistration::Registered));

        let result = book.sync_from_ledger(&ledger);
        assert_eq!(
            result,
            Err(ThirdPointConflict::ConflictingPack {
                identity: later.identity
            }),
            "later 身份在册内容（stale）与账本侧产出内容不同 ⟹ 冲突"
        );

        // 已登记部分保留：earlier 身份键序早于 later，冲突前已被 sync 成功登记（用第二条合法
        // pack 验证部分入册——本次调用的返回值虽是 Err，但 self.established 的写入不回滚）。
        assert_eq!(
            book.pack(&earlier.identity),
            Some(earlier),
            "冲突前已登记部分保留在册"
        );
        // 在册旧内容不被冲突覆盖：later 身份仍是预登记的 stale 版本，不是账本侧的 later。
        assert_eq!(
            book.pack(&later.identity),
            Some(stale),
            "在册旧内容不被冲突覆盖"
        );
        assert_eq!(book.registrations, 2, "stale 预登记 1 + earlier 冲突前登记 1 = 2");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 迟到处置（裁定一 + #587 字面）：照实登记，不产独立信号，无量化判据
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn late_pack_is_registered_verbatim_regardless_of_confirmed_clock() {
        let mut fresh_ledger = ledger();
        let fresh = confirmed_pack(&mut fresh_ledger, frame(400), 11, 5_000);
        // 另一本账上产一份知情时远早于 fresh 的成立档（交易层视角的「迟到」形态）。
        let mut late_ledger = ledger();
        let late = confirmed_pack(&mut late_ledger, frame(100), 3, 12);
        assert!(late.confirmed_as_of < fresh.confirmed_as_of);

        let mut book = ThirdPointBook::new();
        assert_eq!(book.register(fresh), Ok(ThirdPointRegistration::Registered));
        // 迟到者照实登记（#587「登记观测，不作独立信号消费」；本票不造迟到判据，见 #680）。
        assert_eq!(book.register(late), Ok(ThirdPointRegistration::Registered));
        // 账本侧零参与（修复轮订正）：不再用 `assert_eq!(late_ledger, ledger_before)` 钉——该断言
        // 在任何实现下都恒真（`register` 的签名 `fn register<S>(&mut self, signal: S)` 根本不接
        // `RetraceLedger` 引用，`late_ledger` 与被测代码之间无任何通路），删去，不留假钉子。
        // 「账本侧零参与」由类型系统保证：`register` 只收 `Copy` 值，`sync_from_ledger` /
        // `register_from_ledger` 只收 `&RetraceLedger`（见该二者签名），无内部可变性即无回写通路。
        assert_eq!(book.pack(&late.identity), Some(late));
        assert_eq!(
            book.pack(&late.identity).map(|pack| pack.confirmed_as_of),
            Some(12),
            "知情时原样在册，供 #680 读取"
        );
        assert_eq!(book.len(), 2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 禁区证明（裁定八）：备战档不进本票任何登记/信号路径
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn standby_watch_is_barred_from_the_registration_entry() {
        let mut ledger = ledger();
        let pack = confirmed_pack(&mut ledger, frame(100), 3, 860);
        // 另一中枢上留一个未决候选（备战档素材）。
        ledger.observe(&up_input(frame(400), 11, None, 880)).unwrap();
        let watch: StandbyWatch = ledger.standby()[0];

        // ① 正面：入口 bound 即禁区闸门——成立档过 `TradableSignal`（`register` 签名同 bound）。
        fn gate<S: TradableSignal + Into<ThirdPointPack>>(signal: S) -> ThirdPointPack {
            signal.into()
        }
        assert_eq!(gate(pack), pack);

        // ② 反面（结构墙）：备战档字段表全解构——三项载荷（回抽位置 / 落锤知情时 / 死亡证明）
        // 结构性缺席，故即便有人日后给它实现 `TradableSignal`，也拿不出 `ThirdPointPack`。
        // 字段表若变，本行编译失败（活守卫，非事后描述）。
        let StandbyWatch {
            identity: _,
            side: _,
            frame: _,
            leave_end: _,
            registered_as_of: _,
        } = watch;

        // ③ 路径面：登记账里只有成立档身份，备战档身份从不在册。
        let mut book = ThirdPointBook::new();
        book.sync_from_ledger(&ledger).unwrap();
        assert_eq!(book.established(), vec![pack]);
        assert!(book.pack(&watch.identity).is_none());
    }

    #[test]
    fn provisional_and_failed_identities_never_reach_the_book() {
        let mut ledger = ledger();
        let confirmed = confirmed_pack(&mut ledger, frame(100), 3, 860);
        ledger.observe(&up_input(frame(400), 11, None, 880)).unwrap();
        ledger
            .observe(&up_input(
                frame(700),
                21,
                Some(RetraceOutcome::RetestReenters),
                900,
            ))
            .unwrap();

        let mut book = ThirdPointBook::new();
        assert_eq!(book.sync_from_ledger(&ledger).unwrap(), vec![confirmed]);
        assert_eq!(book.len(), 1, "未决 / 判败身份不进成立档登记账");
        assert!(!book.contains(&key_of(frame(400), 11)), "未决身份不在册");
        assert!(book.pack(&key_of(frame(700), 21)).is_none(), "判败身份不在册");
    }

    #[test]
    fn register_from_ledger_reads_the_single_identity_portal() {
        let mut ledger = ledger();
        let pack = confirmed_pack(&mut ledger, frame(100), 3, 860);
        ledger.observe(&up_input(frame(400), 11, None, 880)).unwrap();
        let mut book = ThirdPointBook::new();

        // Confirmed 身份 → 登记；非 Confirmed 身份 → `Ok(None)`（账本无成立档，照实不登记）。
        assert_eq!(
            book.register_from_ledger(&ledger, &pack.identity),
            Ok(Some(ThirdPointRegistration::Registered))
        );
        assert_eq!(
            book.register_from_ledger(&ledger, &key_of(frame(400), 11)),
            Ok(None)
        );
        assert_eq!(book.established(), vec![pack]);

        // 侧与位置照实可读（观测面 = 全字段直读，无判据）。
        let seen = book.pack(&pack.identity).unwrap();
        assert_eq!(seen.side, RetraceSide::Buy);
        assert_eq!(seen.retest_end, pack.retest_end);
        assert_eq!(seen.death_certificate, pack.death_certificate);
    }
}
