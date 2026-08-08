//! **递归 T 操作引擎**（flat 逻辑递归化，编排者裁决 2026-06-21）。
//!
//! 本模块是 flat `t_engine.rs`（battle-tested CL +120%）的**递归架构表达**——用 `TInstance`/`TRoot`
//! 结构承载 flat 的已验证操作逻辑，**不加不减**。编排者裁决：删除所有递归引擎自创的约束
//! （C1 永不翻空 / candidate gate / 连续 level=父-1 / 方向 gate），逐行对照 flat 重新实装。
//!
//! GUARD-ROLE: t-engine-rec-branch-live-python-caller
//!
//! ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#762 C7-E3 执行票核定，
//! ## 评审 FAIL 后订正——原稿"deprecated 待退役"判定不成立，见下）
//!
//! - **名分**：**现役**（机械判据：有非测试调用者 ∧ 无 `#[deprecated]` 标记 ∧ 在唯一 git 线
//!   main 上）。本文件 + `rec_stream.rs` + `rec_driver.rs` + `ffi.rs::PyRecStream`（PyO3 导出名
//!   `RecTStream`，`ffi.rs:291` `#[pyclass(name="RecTStream")]`）构成"递归 T"分支——与 `stream.rs`
//!   头部 GUARD-ROLE 块所述"flat T"分支（现役）是同目录**两个互不调用但均现役的并行实现**
//!   （本文件文档自述"是 flat `t_engine.rs` 的递归架构表达"，即对照对象非依赖对象）。
//!   `backtest.rs`/`backtest_run.rs`（`#[cfg(test)]` 模块级门控）不在本支现役范围内，是
//!   独立的 deprecated 待退役核（见该二文件头部）。
//! - **对照什么**：对照 `stream.rs`/`t_engine.rs` 现役支——(1) python 调用面：**判据固定为
//!   从 `lib.rs` pymodule 导出名反查**（`lib.rs:2660-2671` 八个导出名），非 Rust 侧类型名
//!   grep python——`PyRecStream` 的 PyO3 导出名是 `RecTStream`（rename），按此名反查命中
//!   4 处真实非测试调用者：`trading_system/strategy/rec_t_strategy.py:45`（**NT 生产策略**，
//!   在册 2026-06-21）、`analysis/capture_ratio_matrix.py:84`、
//!   `analysis/t3_exit_trigger_diag.py:35`、`backtest/rec_backtest.py:8`（原稿按 Rust 内部名
//!   `\.RecStream(` grep 得"零命中"，因 rename 而漏查，非真实零调用）；(2) 生产可达面：
//!   `rec_stream.rs:24` 生产 `use super::rec_driver`，`RecStream`/`rec_engine::` 类型经
//!   `ffi.rs::PyRecStream`→PyO3 导出→python 调用者可达——链 `RecTStream`→`PyRecStream`→
//!   `rec_stream`→{`rec_engine`,`rec_driver`} 全生产可达。`backtest.rs`/`stream.rs`/
//!   `t_engine.rs` 均不导入本子簇任何符号（`grep -rn "rec_engine::\|RecStream"
//!   rust/src/recursive_t/{stream,t_engine,backtest}.rs` 零命中，这一点原判不变）——但这只
//!   说明本子簇与 flat 支互不调用，不代表本子簇生产不可达（生产可达面是"到 python 调用者"，
//!   不是"到 flat 支"）。
//! - **与现役差在哪**：无——本子簇经改判与 flat 支同为现役，仅调用路径不同（flat 支走
//!   `TFugueStream`，本子簇走 `RecTStream`），均有 NT/analysis/backtest 侧真实调用者。
//! - **禁回灌**：本次仅加标记，未删除/未移动任何代码（六文件均保留原状，测试套件不受影响）。
//!
//! ## 与 flat 的映射（递归化 = 同行为换载体）
//! - flat `layers: Vec<Layer>`（绝对 ladder 数组）→ `instances: Vec<TInstance>`（按 level 索引）。
//! - flat `nearest_active_parent(j)` = `(j+1..MAX).find(active)` → 遍历 instances 找 level>j 最低 active。
//! - flat `highest_active()` → 遍历 instances 找最高 level active。
//! - flat 单核心 campaign 三阶段（`stage`/`core_cost_basis`/`withdrawn`/`earning_cash`）→ 在 `TRoot`
//!   （非 per-instance，per-instance 三阶段是递归自创，已删）。
//! - flat `route_bsp`/`sink`/`recover`/`drain`/`enter`/`ascend`/`emergence_upgrade`/`clear_all`/`step`
//!   → `TRoot` 同名方法，逐行对照 flat。
//!
//! ## flat route_bsp 分派（无 C1/candidate/方向 gate，编排者裁决）
//! - **有 nearest_active_parent P**（子级）：反父向 BSP → sink（P 减仓 1/3 + j 开反向短差）或 drain
//!   （j 持遗留同父向仓 → 减暴露）；同父向 BSP → recover（j 持短差则平清升回 P）或 no-op。
//! - **无父**（核心级）：空仓 → enter（全 free，方向 = BSP 方向）；同向更高 ladder → ascend；
//!   **反向 → flip（clear 全塔 + 反向 enter）**——核心**可翻空**（flat 无 C1）。
//!
//! ## 会计（单一 free 池 + 单 campaign 三阶段，复用 rec_add/rec_reduce）
//! TW = free + Σ_inst sign(d)·u·c + withdrawn。每 rec_add/rec_reduce 同价 c NAV 中性 ⟹ TW 逐 bar 守恒。
//! 三阶段（第31课）：CostReduction（降成本）→ CapitalRecovered（退本金 free→withdrawn）→ EarningShares
//! （增股数）。account_reduce 按被减层方向：核心多头 reduce → 降成本；短差腿 → short_leg_pnl。
//!
//! ## 认识论等级
//! 数据结构/会计/守恒 = L0；route_bsp 行为对照 flat = L0（flat 已 L3 验证 CL+120%）；递归化后回测
//! 收益复现 = L3（验收：与 flat 行为等价）。

use super::prove_guards::{
    prove_relabel_invariant, prove_sigma_quota, prove_sink_descends, OpTrigger, ProveGuards,
};
use super::types::Direction;
// **σ-不变配额 `f = 1/λ` 的唯一源（#943 统一，原为本模块私有 `1.0 / 3.0` 字面量）**。
// 直接引 `fugue_v3::MOBILE_FRAC`（= `1.0 / fugue_v3::LAMBDA`）——`prove_guards::prove_sigma_quota`
// 重算 canonical 用的就是它，两侧同源 ⇒ 结构上不可能再劈叉（劈叉后果：NT 生产
// `rec_t_strategy.py` → `RecTStream` → `rec_engine` 每次 sink/drain panic）。
// **数值不变**（`1.0 / 3.0` ≡ `1.0 / 3.0`，位模式相等，bit-exact），= flat MOBILE_FRAC，对照可比。
//
// **名分（#925 裁定，2026-08-07）**：`λ = 3` 的依据 `026:80` 是**原文举例、且原文同句明写可调**
// （「……但仓位可以控制，**例如**用其中的1/3，慢慢养成好习惯以后，**就可以更随心所欲一点**」；
// 同课 `026:447`【答疑】更明确「**不熟练的情况下**，如果仓位不太大，1/3或1/4是比较合适的」）
// ⇒ **不是原文规定值**，是训练轮档位。形式 `f = 1/λ` 层**待判**（否掉「级别无关」需证各档 `w`
// 不同，而 ADR 0017 裁定八已判上档测不出）。正本：
// `.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md` `## ★§925 订正`。
use crate::fugue_v3::{MOBILE_FRAC, SUB_LIQ_FACTOR};
use crate::trading::types::Polarity;
/// 活跃/零化阈值 = flat（`Layer::is_active` / `reduce_at` 零化 / `add_at` 占用 = `1e-12`）。
/// **对照 flat，不自创**：rec 此前用 `1e-9`（比 flat 大 1000×），在 BTC 几何塔深层（核心 units
/// 衰减到 (1e-12, 1e-9) 带）误把仍活跃的核心多头零化 → highest_active 跌到次级别空头 → 核心翻空
/// 发散（BTC/Structural rec −14.9% vs flat +38.9%，首个分歧 bar=1842082：lv4 核心 L1.180e-9 经
/// sink 减到 7.867e-10，flat 仍活跃骑牛，rec ≤1e-9 零化丢核心翻空）。
const EPS: f64 = 1e-12;
/// 级别上界 = flat 有效操作级别数 = `MAX_LADDER(11) - BASE_LADDER(LADDER_MOVE=3) = 8`。
/// flat 的 ladder = bsp.level + BASE_LADDER，有效 ladder 3..10 ⟺ bsp.level/t_level 0..7；ladder≥11
/// （t_level≥8）被 flat ceiling 丢弃。rec 无 BASE_LADDER 偏移，故 ceiling 直接 = 8（对照 flat：
/// emergent t_level≥8 不升级、BSP level≥8 不消费）。BTC emergent t_level 触达 8+ 时此 ceiling 关键
/// （rec 此前 MAX_LEVEL=16 让 rec 升级而 flat 不升 → 核心方向错位 → BTC/Structural 空头主导发散）。
pub const MAX_LEVEL: usize = 8;

// ════════════════════════════ 引擎配置（变体闸门，受控实验单进程多变体）════════════════════════════

/// 引擎变体配置（OFF / ANCHOR / NEST 受控对照，单进程内可显式构造多变体——避免 env 串扰）。
///
/// - **OFF** = `from_env()` 默认（trend_done_clear ON、anchor OFF、nest OFF）= 当前 main 独立腿基线。
/// - **ANCHOR** = OFF + `enable_hold_anchor`（552号趋势底仓）。
/// - **NEST** = `enable_nest`（命题4 读法乙）：旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转。
///
/// bit-exact 契约：`enable_nest=false ∧ enable_hold_anchor=false` ⇒ on_bar/route_bsp/sink/drain/enter
/// 行为与 OFF 逐字一致（新逻辑全部 flag 门控、anchor 恒 0）。
///
/// **做空腿开仓触发器（做空腿对称化，#108 L2 根因修复，编排者推动 FINAL GOAL）**：
/// #108 坐实做空腿亏 = 多空开仓触发**不对称**——多头开仓 = `view.buy[k]`（任意买点 type1/2/3，含
/// 及时的 type1 底背驰 ⇒ 骑涨，每级别 long_pnl 全正）；做空开仓 = **仅 `view.t3sell[k]`**（type3
/// 最滞后破位）⇒ 系统性晚建，错过 type1 顶背驰 → 整段 |跌幅| → type3 破位（强牛中 = 回调底 → 涨回
/// → zg 止损）。逐笔实证：做空 L0 捕获率 34-43% < 50%（BTC 36%），多头每级别全正。
///
/// 对称化（缠论「开@顶 平@底」对称性）= 把开空触发提到与多头同等及时度：
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortEntry {
    /// 默认基线（#117 已 L3）：`view.t3sell[k]`（type3 破位，最滞后）。保 #117 逐位复现。
    T3,
    /// 候选A：`view.t1sell[k]`（type1 顶背驰，与多头 type1 底背驰对称——开空于走势终完美）。
    T1,
    /// 候选B：`view.sell[k]`（任意卖点 type1/2/3，与多头 `view.buy` 完全对称）。
    Any,
}

/// **开多腿入场方向/级别门控（T1 方向错位修复，#164/R1，546/547 cascade）**：
/// #164 逐笔分类坐实 T1 方向错位（58.7%，最大可约靶子）= 入场在与持仓方向**逆向的同级别走势段**
/// （开多在跌段）。L3 解剖（BTC L0 worst 例 eb=1345932）实证：买点在**下落段 20% 处** fire 即开多
/// （非段末转折，是逆势段途中），随后段从 5540→5155（−7%）继续跌 ⇒ 真 wrong-side（**非 574 滞后税**
/// ——574 是确认在段末，此处入场在段首）。74.8% 的 T1 落在 L0（次级别），65.5% 携 cross_level_conflict
/// （入场段 ∧ 某更高级别走势段逆向 = 546/547 级别错配）。
///
/// 现状非对称：开空有 `below_core_long` 门（核心多腿之下才开空，shortleg #69），开多**零门控**
/// （任意买点 `view.buy[k]` 即开多）⇒ 多腿在下落段/逆高级别段无差别开仓 = T1 主体（long 占 T1 的 70.4%）。
/// 本 enum 把开多对称到「段方向/级别一致」门控（缠论：买点应开在**上涨段**或与高级别一致，第17/27课）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongEntry {
    /// 默认基线（#117/#113 Face A 已 L3）：`view.buy[k]` 任意买点，**零方向门控**。保 bit-exact 逐位复现。
    Any,
    /// 候选L1（同级别段方向对齐）：仅当 `view.nodes[k].direction == Up`（k 段为上涨段）才开多。
    /// 直击 T1 定义（入场段逆向）——下落段途中买点不开多（等段转上涨=顺向确认）。对称镜像开空 `nodes[k]==Down`。
    SegAlign,
    /// 候选L2（跨级别段方向对齐，攻 65.5% xlc / 546/547）：仅当**无更高活跃级别走势段逆向**
    /// （∀ j>k 的 `view.nodes[j].direction != Down`）才开多——次级别买点不在高级别下落段中逆势开多。
    /// 对称镜像开空：无更高级别 Up 段才开空。这是 cascade 级别错配（次级别操作逆主级别走势）的直接门控。
    CrossLevel,
}

#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    pub enable_earning: bool,
    pub enable_three_stage: bool,
    pub enable_trend_done_clear: bool,
    pub enable_hold_anchor: bool,
    pub enable_nest: bool,
    /// **consume平空（任务22）**：次级别反核心向背驰段定位 ⇒ 平核心仓 1/3（缩短整仓长持死扣，减强牛穿仓）。
    pub enable_nest_consume: bool,
    /// **严格逐级区间套（任务22 开放轴C）**：主翻转定位点须 top..loc 逐级背驰段一致（非仅 top 武装+a0 定位）。
    pub enable_nest_strict: bool,
    /// **读法B/读法乙递归（任务18 编排者修正）**：每级别独立腿多重赋格，消费 d_top 链 switch。旁路 instances 路径。
    pub enable_reading_b: bool,
    /// 读法B 触发器：false=走势完成链（556 读法B 基线）/ true=背驰段链（读法乙递归，触发更频繁可能解冻顶层）。
    pub reading_b_diverge: bool,
    /// **读法B 一对多空腿（任务57=53.1 编排者重写）**：每级别一对 LegPair（多腿+空腿同时在场）。
    /// 开=该级别买卖点（第17课）/ 平=反向买卖点 / 止损=否定线（进场中枢 ZG/ZD 破坏）/ 链破坏 churn 门控
    /// （第27课区间套：链完整=回调不动核心多腿，链破坏=转折动核心）。删 d_top 几何方向单腿模型。
    /// OFF=false ⇒ on_bar/单腿 reading_b/instances 路径逐字不动（bit-exact）。env `T_READING_B_PAIR`。
    pub enable_reading_b_pair: bool,
    /// **牛熊对称核心翻空（任务 bear-validate）**：放开 #69 的 `k<核心` 禁令——最高活跃级别走势完成
    /// （d_top）时核心多腿不止平到现金，而是**翻空镜像**（大额吃熊）。有效域 ⊂ 真 bear regime
    /// （231号 formalization-validity-domain：net-up 8标的均亏=灾难，须 bear 数据 L3 验证）。
    /// false ⇒ g_pair 行为与 committed #69 逐字一致（bit-exact）。env `T_PAIR_CORE_SHORT`。
    pub enable_pair_core_short: bool,
    /// **放开 k<核心 t3sell 开空（任务 bear-validate 变体2）**：移除 #69 开空门控的 `below_core_long`
    /// 限制——t3sell 亦可在核心及其上开空（=变体1 机制，max_gross>1× 大额做空）。直接测「放开 k<核心
    /// ⇒ 大额做空是否吃熊」。net-up 灾难（CL L4 单笔 −40928，539）；有效域 ⊂ 真 bear。env `T_PAIR_CORE_SHORT_T3`。
    pub enable_pair_core_short_open: bool,
    /// **Face B 均匀基准单元定仓（做空腿赚 #110，shortleg-profit-spec §5.3/§6.3）**：true ⇒ LegPair
    /// open_long_leg/open_short_leg 用 `uniform_base_units`（= INITIAL_CAPITAL×MOBILE_FRAC，级别无关，
    /// 无 depth 衰减）取代 `geom_tower_quota`（恒仓归一化 Σ=free≤1× = 压制副作用，msb §14.1）。多级别独立
    /// 叠加 → 杠杆来源A 涌现（579），否定线 [ZD,ZG] 封顶每条腿（liq=0）。**同时**令信号层（rec_stream）
    /// 填充 `view.zd/zg`（进场中枢边界）⇒ pair_stop_loss_step 真生效（RB_PAIR 下 zd/zg 恒 None=死代码，
    /// liq=0 仅靠 geom_tower 恒仓；删 geom_tower 必须同步接真否定线否则穿仓）。false ⇒ RB_PAIR/OFF 逐字
    /// 不变（bit-exact，geom_tower + zd/zg 恒 None）。env `T_FACEB`。
    pub enable_uniform_sizing: bool,
    /// **做空腿开仓触发器（做空腿对称化，#108 根因）**：默认 `T3`（= #117 基线，逐位复现）；
    /// `T1`/`Any` 把开空对称到多头 `view.buy`（候选A/B）。`below_core_long` 门 + zg 否定线封顶不变
    /// （防核心假顶翻空灾难，leverage-accept −106256）。env：`T_PAIR_SHORT_T1` / `T_PAIR_SHORT_ANY`。
    /// 只在 LegPair 路径（`enable_reading_b_pair`）经 g_pair 生效；OFF/instances 路径不跑 g_pair ⇒ bit-exact。
    pub pair_short_entry: ShortEntry,
    /// **开多腿入场方向/级别门控（T1 方向错位修复，#164/R1）**：默认 `Any`（= 现状零门控，逐位复现）；
    /// `SegAlign`（同级别段方向对齐）/ `CrossLevel`（跨级别段方向对齐，攻 65.5% xlc）把开多对称到方向门控。
    /// 对称镜像作用于开空（SegAlign/CrossLevel 同时门控开空，与 `pair_short_entry` 的**触发口径**正交——
    /// pair_short_entry 选「哪个卖点开空」，pair_long_entry 选「何方向/级别条件下才开仓」）。
    /// env：`T_PAIR_LONG_SEGALIGN` / `T_PAIR_LONG_XLEVEL`。只在 LegPair 路径经 g_pair 生效（OFF/instances bit-exact）。
    pub pair_long_entry: LongEntry,
    /// **LegPair 核心多腿涌现升级（R3 段无腿修复 #164/#6）**：true ⇒ consume_leg_pairs 在止损后、g_pair 前
    /// 检查 `view.emergent_top`——核心多腿(highest_active_long=cc)同向(Long)且 cc<涌现级别 target ⇒ relabel
    /// 上移到 target（持仓继承 long_units/long_basis = 无新资金 = 敞口不变，否定线更新为 target 中枢 ZD）。
    /// 对照 instances 路径 `emergence_upgrade`/`ascend`（LegPair 路径原缺此机制 ⇒ 核心腿卡在低级别，L4/L5
    /// 涌现段无核心腿=S1 段无腿 70.8%/高级别~100%）。多重赋格理想：每涌现级别一专属核心腿捕获本级别 |Δ|。
    /// false ⇒ Face A/B/RB_PAIR/OFF 逐字不变（bit-exact，无 relabel）。env `T_PAIR_EMERGE`。
    pub enable_pair_emergence: bool,
    /// **9 轨道操作分派（task#40 B，#39 O1-O9 τ对称轨道）**：true ⇒ route_bsp 走显式 9 轨道判别——
    /// 在 Some(p) 同父向分支补 **O3 add（= 买回/卖回 = 不动 h 同级别短差腿部分重建）** 缺口轨道
    /// （现状：同父向持短差 → recover 整条 / 无短差 → no-op，**没有"加一段"那一腿**）。其余八轨道
    /// （enter/ascend/flip/sink/recover/drain/no-op）= 已有原语的轨道标注，无行为改变。**account 过滤是
    /// route 之后独立 gate 非 route 内分支**（#40 硬约束，禁 #35 C7/初版 NL7 把账本塞进操作语义）——
    /// route 内零 if regime/account，空头镜像（add_short）一律执行（操作语义合法）。recover vs add 判别
    /// = C 任务（区间套 H¹ 定位 `is_sub_trend_done`）接口，本实装 fallback=true ⇒ 全走 recover = 现状。
    /// false ⇒ route_bsp 走旧二元 ⇒ facea 54279a503e 逐字一致（bit-exact）。env `T_ORBIT9_DISPATCH`。
    pub enable_orbit9_dispatch: bool,
    /// **节点向量路由（task#63，#61 §五-§七 双源完全分类实装 = 561 Ω 解坍缩节点侧）**：true ⇒
    /// route_bsp 读节点 j 在 **r\*（最高活跃级别 = 核心）** 的级别角色 R+/R−（不只读 nearest_active_parent
    /// 单标量 = 拍扁纤维）。**核心溶解的踏空根因①④**（#61 §五）：次级别 type1/type3（R− 反核心向回调腿）
    /// 触发 `is_reduce`→sink 时，旧路由减核心机动仓（OFF anchor=0 ⇒ mob_base=u_p ⇒ 主升浪回调处砍核心
    /// = 踏空）。§六 canonical：`N1@k<r*,R−`/`N3@k,R−` → **O6 sink 机动不动核心**（核心 units 不减，只动
    /// 机动配额）。本门控 ON ⇒ 对识别为 r* R− 回调腿的 sink，**强制核心保护**（anchor=核心整仓 ⇒ mob_base=0
    /// ⇒ sink 配额作用于零机动仓 ⇒ 核心 units 不被减 = 机动不动核心）。R+ 同向延续腿不受影响（正常 sink/recover）。
    /// false ⇒ route_bsp 不读 r* 角色 ⇒ 与 T_ORBIT9_DISPATCH 行为同（bit-exact）。env `T_ORBIT9_NODEVEC`。
    pub enable_orbit9_nodevec: bool,
    /// **L_confirm 第四轴会计（task#95 W-lconfirm，observation-only）**：true ⇒ open_*_leg 在开仓时捕获
    /// 该腿的**确认深度 c**（= 触发开仓的买卖点信号类——type1 全深度背驰链 / type3 单层区间套转折 / type2
    /// 其余）+ **腿轴 leg**（H⁰核心=`highest_active_long()==k` / H¹短差=其余），close_*_leg 时把 realized
    /// pnl 累加进 `pi_k_leg_dir_c[k][leg][dir][c]` 纤维格子。**纯观测：不参与任何仓位/方向/止损/churn 决策**
    /// ⇒ OFF/ON 决策路径逐字一致（bit-exact，新字段无消费者）。codex #92 审计指出 K×L×D 三轴 P&L 符号
    /// 在同 cell 内不定（漏第四轴 c）；本会计跑 L3 判定同 (k,leg,dir) cell 是否按 c 符号分裂。env `T_LCONFIRM_AUDIT`。
    pub enable_lconfirm_audit: bool,
    /// **逐级内在配额（task#84 子5，仓位上同调塔 597号 子5 实装授权）**：true ⇒ route_bsp 的 sink/drain/add
    /// 把固定 σ-不变配额 `m = u × MOBILE_FRAC(=1/3)` 替换为**逐级内在函数** `m = u × mobile_frac(L_pullback, L_confirm)`
    /// ——配额由触发该操作的级别 k 的**区间套回调深度 L_pullback**（d_top 深=高/t3sell 浅=低）+ **区间套确认深度
    /// L_confirm**（k-1 快触发=低/须深层=高）内在决定（split 自适应定理，tower_intrinsic_quota §3：深确认→大配额
    /// 骑主升浪 / 浅确认→小配额防踏空）。两自变量是 ConfDepth 三值代理（type1=全深度 d_top / type3=单层 t3sell /
    /// type2 居中，#95 L_confirm 第四轴实测端），**纯内在零外部 θ/振幅/regime 门**（零件1 v3 唯一非拍扁形式）。
    ///
    /// **★概念层张力（no-workaround 透明声明，不绕过）**：本路径**有意偏离** 542号 σ-不变配额（settled：
    /// f=m/u 必须级别无关 = 1/λ，T48/T59/T23 强制；`prove_sigma_quota` panic 守卫）。逐级 mobile_frac 读 L_pullback(k)/
    /// L_confirm(k) ⇒ f_k≠f_{k+1} ⇒ 破 σ-不变。这是 597号（生成态）明列的「逐级自相似纤维塔 vs 单级别拍扁底空间」
    /// **不可调和概念分离**的实装侧——597 `code_changes` 显式授权子5 #84「MOBILE_FRAC→逐级 α*_k，OFF 退化 bit-exact」。
    /// 故 **ON 路径不调 `prove_sigma_quota`**（σ-不变在 ON 不适用 = 声明的替代概念，非被绕过的守卫）；TW 中性
    /// （`prove_tw_neutral`）+ sink/recover 配对（`prove_sink_recover_balance` 数操作非数量）与配额值无关 ⇒ 两守卫 ON 仍守。
    ///
    /// OFF=false ⇒ sink/drain/add 走 `quota()`（σ-不变 1/3）+ `prove_sigma_quota` = 逐字 bit-exact。env `T_INTRINSIC_QUOTA`。
    pub enable_intrinsic_quota: bool,
    /// **payoff G 轴 = 级别-方向对齐门 + 强平可达性（task#5，539 支配失血分量修复，仓位上同调塔遗漏-G1）**：
    /// true ⇒ g_pair 开腿时叠加 G 轴两分量——
    ///
    /// **分量① 级别-方向对齐门（539 方向误读失血修复，L0 结构）**：开仓腿的极性须与**该腿自身级别 k 的
    /// 走势方向** `view.nodes[k].direction` 对齐——开多须 `Up`、开空须 `Down`。539 根因（settled §二/§289）：
    /// `d_k=Up` 却开空（卖点误读为方向）⟹ `L_confirm=∞` ⟹ 孤儿腿（无配对闭合转折节点）⟹ 永不模掉边界
    /// ∮ 浮亏到 eod = **结构性必失血**（payoff_fiber §2.2 概念A：孤儿⟹必失血 L0）。本门 = payoff_fiber §2.4
    /// 「结构孤儿失血由分类层 L0 判据消除（不开孤儿腿）= 内在结构修复」的实装侧——**只过结构孤儿（L0），
    /// 不读 regime / 不读 payoff 符号**（payoff 符号是 L3 regime 函数，分类层不降级声明，§3.2）。**与
    /// `pair_long_entry`(SegAlign/CrossLevel) 正交且更严**：`pair_long_entry` 豁免核心级 + 用跨级别口径；
    /// G 轴对**所有级别**（含核心）施加**同级别方向对齐**——核心 d_k=Up 开空亦是孤儿（539 根因不豁免核心）。
    ///
    /// **分量② 强平可达性（539 支配失血分量=孤儿腿不可达闭合 修复，L0 结构）**：分量①阻断方向误读孤儿后，
    /// 仍有「开仓时方向对齐但配对闭合转折节点 Y 永不 fire」的孤儿腿（payoff_fiber §2.2：腿不闭合 ⟹ 留在
    /// 1-链层 ⟹ ∮ 浮亏持续累积到强制平仓）。其闭合**仅经 NAV≤0 账户级强平 / eod finish**（=不可达闭合：
    /// 浮亏先吞 NAV）。本分量给 G 轴开的每条腿**强制锁定否定线**（多腿=进场中枢 ZD `view.zd[k]` / 空腿=ZG
    /// `view.zg[k]`）作**可达强平路径**——`pair_stop_loss_step` 据此在结构破坏（跌破 ZD / 涨破 ZG）即平，
    /// 把「不可达闭合（NAV 吞光才平）」修复为「可达闭合（否定线即平）」。与 `faceb_stop` 正交：`faceb_stop`
    /// 仅 `enable_uniform_sizing` 锁否定线，G 轴**独立锁**（不依赖 Face B），使强平可达性不依赖定仓模式。
    ///
    /// **概念层声明（formalization-validity-domain，诚实分层）**：G 轴是 payoff 纤维丛的**失血修复维 G**
    /// （payoff_fiber 〇节），**只修结构孤儿失血（L0）**——不声明「修复后超 BH」（payoff 符号是 L3 regime
    /// 函数，OKLO 正/GC 负，§3.3 未决，不降级声明）。**与 539 清仓 regime 守恒不冲突**：539 否证的是清仓
    /// 判据的 `top*`/`anc` regime 门（开放轴），G 轴是**开仓时的结构孤儿过滤**（539 根因侧，非被否证的清仓
    /// 修复侧），payoff_fiber §2.4 修复方向明列「不开孤儿腿 = 内在结构修复」。**不碰 instances 路径的
    /// `prove_sink_recover_balance`**（G 轴在 LegPair 的 g_pair 开腿路径，不改 sink/recover campaign 守卫）。
    ///
    /// OFF=false ⇒ g_pair 开腿不读 nodes[k] 方向 / 不锁 G 轴否定线 = 逐字 bit-exact。env `T_G_AXIS`。
    pub enable_g_axis: bool,
}

/// **确认深度 c 的可观测代理（L_confirm 第四轴，task#95）**：开仓买卖点的区间套确认深度类。
/// 缠论依据（#69 shortleg-alpha:11/26 L3 实装事实）：确认深度 ∝ 持仓尺度——主力核心绑 type1/d_top
/// （全深度背驰链贯通到 a0），次级别短差绑 type3（单层区间套转折），type2 居中。**严格非补丁**：c 不是
/// 新造代理，是 g_pair 既有门控信号（t1buy/t1sell=type1 / t3sell=type3 / 其余 buy/sell=type2）的读数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfDepth {
    /// type1（顶/底背驰=走势完成=全深度区间套链贯通 a0=`d_top`/`t1buy`/`t1sell`）——最深确认（c=深）。
    T1 = 0,
    /// type2（其余买卖点=非 type1 非 type3）——中等确认（c=中）。
    T2 = 1,
    /// type3（突破中枢+回试不回=单层区间套转折=`t3sell`）——最浅确认（c=浅，#69 次级别短差信号）。
    T3 = 2,
}

impl ConfDepth {
    /// 多腿（买点）确认深度：t1buy=type1 / 其余 buy=type2（多头无 type3 开仓信号，多头开仓走 view.buy + long_dir_ok）。
    fn from_buy(view: &LevelView, k: usize) -> Self {
        if view.t1buy[k] {
            ConfDepth::T1
        } else {
            ConfDepth::T2
        }
    }
    /// 空腿（卖点）确认深度：t1sell=type1 / t3sell=type3 / 其余 sell=type2。
    /// 优先级 type1>type3（同 bar 若 t1sell 与 t3sell 同时为真，type1 是更深确认 ⇒ 归 type1）。
    fn from_sell(view: &LevelView, k: usize) -> Self {
        if view.t1sell[k] {
            ConfDepth::T1
        } else if view.t3sell[k] {
            ConfDepth::T3
        } else {
            ConfDepth::T2
        }
    }
}

impl EngineConfig {
    /// 从 env 读（= 当前 main 行为，保 OFF 基线）。
    pub fn from_env() -> Self {
        EngineConfig {
            enable_earning: std::env::var("T_NO_EARNING").is_err(),
            enable_three_stage: std::env::var("T_NO_THREESTAGE").is_err(),
            enable_trend_done_clear: std::env::var("T_NO_TREND_DONE_CLEAR").is_err(),
            enable_hold_anchor: std::env::var("HOLD_ANCHOR").is_ok(),
            enable_nest: std::env::var("T_NEST_READING_B").is_ok(),
            enable_nest_consume: std::env::var("T_NEST_CONSUME").is_ok(),
            enable_nest_strict: std::env::var("T_NEST_STRICT").is_ok(),
            enable_reading_b: std::env::var("T_READING_B").is_ok(),
            reading_b_diverge: std::env::var("T_READING_B_DIVERGE").is_ok(),
            enable_reading_b_pair: std::env::var("T_READING_B_PAIR").is_ok(),
            enable_pair_core_short: std::env::var("T_PAIR_CORE_SHORT").is_ok(),
            enable_pair_core_short_open: std::env::var("T_PAIR_CORE_SHORT_T3").is_ok(),
            enable_uniform_sizing: std::env::var("T_FACEB").is_ok(),
            // 做空腿对称化触发器（#108 根因）：Any 优先于 T1，二者皆无 ⇒ T3（#117 基线，逐位复现）。
            // 注：不计入 any_variant_enabled()——pair_short_entry 是 face_a 默认引擎的**修饰**（开空触发口径），
            // 非独立实验变体；单独 set T_PAIR_SHORT_T1 ⇒ production() 仍走 face_a() 分支（off() 不 reset 此字段，
            // 经 from_env 基底贯穿到 face_a），= face_a + 对称开空。
            pair_short_entry: if std::env::var("T_PAIR_SHORT_ANY").is_ok() {
                ShortEntry::Any
            } else if std::env::var("T_PAIR_SHORT_T1").is_ok() {
                ShortEntry::T1
            } else {
                ShortEntry::T3
            },
            // 开多入场门控（T1 方向错位修复，#164/R1）：XLEVEL 优先于 SEGALIGN，二者皆无 ⇒ Any（现状零门控，逐位复现）。
            // 注：与 pair_short_entry 同理，是 face_a 默认引擎的**修饰**（开仓方向门控），非独立实验变体
            // （不计入 any_variant_enabled()）；单独 set ⇒ production() 仍走 face_a()，from_env 基底贯穿。
            pair_long_entry: if std::env::var("T_PAIR_LONG_XLEVEL").is_ok() {
                LongEntry::CrossLevel
            } else if std::env::var("T_PAIR_LONG_SEGALIGN").is_ok() {
                LongEntry::SegAlign
            } else {
                LongEntry::Any
            },
            enable_pair_emergence: std::env::var("T_PAIR_EMERGE").is_ok(),
            // 9 轨道操作分派（task#40 B）：缺省 OFF ⇒ route_bsp 旧二元 bit-exact（facea 54279a503e）。
            // 注：与 pair_short_entry/pair_long_entry 同理，是 instances route_bsp 路径的**修饰**（轨道判别），
            // 非独立实验变体——不计入 any_variant_enabled()（off()/production() 行为不被它改变，由其自身门控保 OFF=bit-exact）。
            enable_orbit9_dispatch: std::env::var("T_ORBIT9_DISPATCH").is_ok(),
            // 节点向量路由（task#63）：缺省 OFF ⇒ route_bsp 不读 r* 角色 = bit-exact。
            // 注：与 enable_orbit9_dispatch 同理，是 route_bsp 路径的修饰（级别角色判别），非独立实验变体——
            // 不计入 any_variant_enabled()，由其自身门控保 OFF=bit-exact。
            enable_orbit9_nodevec: std::env::var("T_ORBIT9_NODEVEC").is_ok(),
            // L_confirm 第四轴会计（task#95）：缺省 OFF ⇒ open/close_*_leg 不捕获 c/leg、不累加纤维格子
            // = 新字段恒默认值 = bit-exact。observation-only：不计入 any_variant_enabled()（不改任何决策路径）。
            enable_lconfirm_audit: std::env::var("T_LCONFIRM_AUDIT").is_ok(),
            // 逐级内在配额（task#84 子5）：缺省 OFF ⇒ sink/drain/add 走 σ-不变 1/3 + prove_sigma_quota = bit-exact。
            // 注：是 route_bsp 配额路径的**修饰**（配额读出函数），非独立 instances 实验变体——不计入
            // any_variant_enabled()（off()/production() 行为不被它改变，由其自身门控保 OFF=bit-exact）。
            enable_intrinsic_quota: std::env::var("T_INTRINSIC_QUOTA").is_ok(),
            // payoff G 轴（task#5，539 失血修复维）：缺省 OFF ⇒ g_pair 开腿不读 nodes[k] 方向、不锁 G 轴否定线
            // = 新门恒不启 = bit-exact。注：是 g_pair 开腿路径的**修饰**（方向对齐 + 强平可达性），非独立
            // instances 实验变体——不计入 any_variant_enabled()，由其自身门控保 OFF=bit-exact。
            enable_g_axis: std::env::var("T_G_AXIS").is_ok(),
        }
    }
    /// OFF 基线（trend_done_clear ON，anchor/nest OFF）。
    pub fn off() -> Self {
        let mut c = Self::from_env();
        c.enable_hold_anchor = false;
        c.enable_nest = false;
        c.enable_nest_consume = false;
        c.enable_nest_strict = false;
        c.enable_reading_b = false;
        c.reading_b_diverge = false;
        c.enable_reading_b_pair = false;
        c.enable_pair_core_short = false;
        c.enable_pair_core_short_open = false;
        c.enable_uniform_sizing = false;
        c.enable_pair_emergence = false;
        c.enable_orbit9_dispatch = false;
        c.enable_orbit9_nodevec = false;
        c.enable_intrinsic_quota = false;
        c.enable_g_axis = false;
        c
    }
    /// **Face B：核心不僵死（做空腿赚 #110，shortleg-profit-spec §5.3）**：RB_PAIR + 均匀基准单元定仓
    /// （删 geom_tower 恒仓归一化压制）+ 真否定线 [ZD,ZG] 封顶（rec_stream 填充 view.zd/zg）。
    /// 三机制（纯级别×买卖点，删 sink/recover/anchor 配额异物）：① 删 sink ⟹ 核心不被衰减（LegPair
    /// 路径本无 sink，instances sink 仅作 OFF 回归守卫保留）；② 核心否定线 = cc 级别中枢 ZD（每腿进场
    /// 中枢边界，次级别回调不触发核心否定线）；③ 核心多腿 churn 门控 d_top（次级别卖点不平核心，已在
    /// g_pair）。唯一变量 = `enable_uniform_sizing`（其余与 reading_b_pair 逐字一致）⇒ 收益差全归因
    /// 删 geom_tower + 接真否定线。L3 有效域：做空腿赚（pair_short_pnl 符号）+ liq=0 + 中间级别解压制。
    pub fn face_b() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_uniform_sizing = true;
        c
    }
    /// **Face A：核心能动 + 接受杠杆涌现（做空腿赚 #113，shortleg-profit-spec §5.2/§六/§七）**：
    /// Face B 基座（删 geom_tower 均匀定仓 + 真否定线 [ZD,ZG]）**叠加核心翻转吃熊**——核心多腿
    /// （cc=highest_active_long）在**自己级别走势完成**（`d_top[cc]`=全深度区间套链贯通真顶=第一类卖点，
    /// 区间套级联减滞后，第27课）翻空镜像（`enable_pair_core_short`，g_pair 核心 churn 段），次级别卖点
    /// 绝不翻核心（547：sub 卖点走 below_core_long 独立空腿）。两 regime 由「哪级别走势完成」自动整合
    /// （零 if regime）：net-up cc 走势未完成 ⇒ `d_top[cc]=false` ⇒ 闸门不开 ⇒ 核心不翻空（无假顶翻空
    /// 灾难，§5.2.4）；bear cc 走势向下完成 ⇒ 闸门开 ⇒ 核心翻空吃熊。接受杠杆=多级别独立腿叠加
    /// （来源A，579），每腿否定线封顶（liq=0）。**不开 `enable_pair_core_short_open`**（=移除
    /// below_core_long 门 = net-up 假顶翻空打主升浪灾难 short_pnl +9623→−106256，leverage-accept L3
    /// 坐实；§5.2.3：type2/3 仅作走势完成级联加速器，不作核心翻转独立触发）。**生产/默认引擎**
    /// （§8.1，见 `production()`）。= reading_b_pair + uniform_sizing + pair_core_short。
    pub fn face_a() -> Self {
        let mut c = Self::face_b();
        c.enable_pair_core_short = true;
        c
    }
    /// **Face A + 核心多腿涌现升级（R3 段无腿修复 #164/#6）**：Face A 基座叠加 `enable_pair_emergence`
    /// ——核心多腿跟随 emergent_top 涌现级别 relabel 上移（多重赋格：每涌现级别一专属核心腿捕获本级别 |Δ|）。
    /// 唯一变量 = `enable_pair_emergence`（其余与 face_a 逐字一致 ⇒ 收益差 + S1↓ 全归因核心涌现升级）。
    /// L3 有效域：S1 段无腿↓（尤 L4/L5 涌现段）+ coverage↑ + 逐笔 ∀r>0 改善 + bit-exact OFF + liq=0。
    /// 升格默认（并入 face_a）须 L3 验证后裁决（形式化有效域，先独立变体 L0→L3）。
    pub fn face_a_emerge() -> Self {
        let mut c = Self::face_a();
        c.enable_pair_emergence = true;
        c
    }
    /// **生产默认引擎配置（做空腿赚 #113，shortleg-profit-spec §8.1「默认开启」）**：纯级别×买卖点
    /// 统一引擎（Face A）为**生产/默认回测配置**（LegPair 路径无 sink/geom_tower 异物 ⇒ 生产路径不残留
    /// 无保护 sink，mid-scale #106 硬约束）。三档（互斥，env 显式优先）：
    /// - `T_OFF_BASELINE` 置位 ⇒ `off()`（instances OFF 基线 = bit-exact 回归守卫，R4）。
    /// - 任一显式实验变体 env 已选（`from_env` 读到 reading_b/nest/anchor/legpair/coreshort/faceb 任一）⇒
    ///   尊重该显式变体（受控实验覆盖默认，保 env 实验工具不失效）。
    /// - 无任何变体 env ⇒ `face_a()`（**默认开启**）。
    ///
    /// **why 不改 from_env 默认**：`from_env()`/`TRoot::new()` 默认 OFF 基线是 instances-path 单测
    /// （`核心级买点_enter_long` 等）的契约——改 from_env 默认会破这些单测 + OFF 回归守卫 base。故「默认
    /// 开启」落在**生产入口**（`RecStream::new_with_a0`，FFI/python 回测路径），不动 from_env。
    pub fn production() -> Self {
        if std::env::var("T_OFF_BASELINE").is_ok() {
            return Self::off(); // 显式回归守卫：instances bit-exact 基线
        }
        let env_cfg = Self::from_env();
        if env_cfg.any_variant_enabled() {
            env_cfg // 显式实验变体 env 已选 ⇒ 尊重（受控实验）
        } else {
            Self::face_a() // 无变体 env ⇒ 默认开启 Face A（§8.1）
        }
    }
    /// 是否已显式启用任一**实验变体**（instances 基线默认 earning/three_stage/trend_done_clear 不计）。
    /// `production()` 用：有显式变体 ⇒ 尊重；无 ⇒ 默认 Face A。
    fn any_variant_enabled(&self) -> bool {
        self.enable_hold_anchor
            || self.enable_nest
            || self.enable_nest_consume
            || self.enable_nest_strict
            || self.enable_reading_b
            || self.reading_b_diverge
            || self.enable_reading_b_pair
            || self.enable_pair_core_short
            || self.enable_pair_core_short_open
            || self.enable_uniform_sizing
            || self.enable_pair_emergence
    }
    /// **读法B 一对多空腿（任务57=53.1）**：每级别 LegPair（买卖点开平 + 否定线止损 + 链破坏 churn）。
    /// OFF + 仅 `enable_reading_b_pair`（旁路单腿 reading_b / instances 路径）。
    pub fn reading_b_pair() -> Self {
        let mut c = Self::off();
        c.enable_reading_b_pair = true;
        c
    }
    /// **牛熊对称核心翻空（任务 bear-validate）**：RB_PAIR + 放开 `k<核心`（核心走势完成→翻空镜像）。
    /// 唯一变量 = `enable_pair_core_short`（其余与 reading_b_pair 逐字一致 ⇒ 收益差全归因核心翻空）。
    /// L3 有效域：须真 bear regime 数据验证（net-up 8标的灾难，239号有效域受限）。
    pub fn reading_b_pair_coreshort() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_pair_core_short = true;
        c
    }
    /// **放开 k<核心 t3sell 大额做空（任务 bear-validate 变体2）**：RB_PAIR + 移除 below_core_long 门控
    /// （t3sell 在核心及其上开空 = 变体1 机制，max_gross>1×）。直接测「大额做空吃熊」。
    pub fn reading_b_pair_coreshort_t3() -> Self {
        let mut c = Self::reading_b_pair();
        c.enable_pair_core_short_open = true;
        c
    }
    /// 读法B 基线（556：每级别独立腿 + 走势完成链触发）。
    pub fn reading_b_dtop() -> Self {
        let mut c = Self::off();
        c.enable_reading_b = true;
        c.reading_b_diverge = false;
        c
    }
    /// 读法乙递归（编排者修正：每级别独立腿 + 背驰段链触发，隔离换触发器效果）。
    pub fn reading_b_diverge() -> Self {
        let mut c = Self::off();
        c.enable_reading_b = true;
        c.reading_b_diverge = true;
        c
    }
    /// ANCHOR（OFF + 趋势底仓 552号）。
    pub fn anchor() -> Self {
        let mut c = Self::off();
        c.enable_hold_anchor = true;
        c
    }
    /// **INTRINSIC_QUOTA（OFF + 逐级内在配额 task#84 子5）**：sink/drain/add 配额由 mobile_frac(L_pullback,
    /// L_confirm) 逐级内在决定（替固定 1/3）。唯一变量 = `enable_intrinsic_quota`（其余与 off() 逐字一致 ⇒
    /// 配额维差异全归因逐级自适应）。597号子5 实装授权；ON 偏离 542号 σ-不变（透明声明，见字段文档）。
    pub fn intrinsic_quota() -> Self {
        let mut c = Self::off();
        c.enable_intrinsic_quota = true;
        c
    }
    /// **G_AXIS（Face A + payoff G 轴 task#5，539 失血修复维）**：g_pair 开腿叠加级别-方向对齐门
    /// （不开方向误读孤儿腿，539 根因）+ 强平可达性（G 轴每腿锁否定线，孤儿腿不可达闭合修复）。唯一变量
    /// = `enable_g_axis`（其余与 face_a() 逐字一致 ⇒ 差异全归因 G 轴）。在 Face A 基座上（LegPair 路径有
    /// nodes/zd/zg 数据 + 真否定线 [ZD,ZG]）测 G 轴，OFF（缺 `enable_g_axis`）退化 face_a bit-exact。
    pub fn g_axis() -> Self {
        let mut c = Self::face_a();
        c.enable_g_axis = true;
        c
    }
    /// NEST（命题4 读法乙，旁路 cc 锚）。
    pub fn nest() -> Self {
        let mut c = Self::off();
        c.enable_nest = true;
        c
    }
    /// NEST + 严格逐级区间套（任务22 开放轴C，无 consume）——隔离 strict 贡献。
    pub fn nest_strict() -> Self {
        let mut c = Self::nest();
        c.enable_nest_strict = true;
        c
    }
    /// NEST + consume平空（任务22：次级别买点平空缩短持仓）。
    pub fn nest_consume() -> Self {
        let mut c = Self::nest();
        c.enable_nest_consume = true;
        c
    }
    /// NEST + consume + 严格逐级区间套（任务22 完整形态）。
    pub fn nest_consume_strict() -> Self {
        let mut c = Self::nest_consume();
        c.enable_nest_strict = true;
        c
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// ════════════════════════════ 走势节点身份（仓位骑节点）════════════════════════════

/// 走势节点身份（重锚键 = `start_bar`）。仓位骑节点——递归结构保留；操作逻辑用绝对 `level`（对照 flat ladder）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrendNode {
    pub start_bar: i64,
    pub end_bar: i64,
    pub price_lo: f64,
    pub price_hi: f64,
    pub direction: Direction,
}

impl TrendNode {
    pub fn new(
        start_bar: i64,
        end_bar: i64,
        price_lo: f64,
        price_hi: f64,
        direction: Direction,
    ) -> Self {
        TrendNode {
            start_bar,
            end_bar,
            price_lo,
            price_hi,
            direction,
        }
    }
    pub fn id_key(&self) -> i64 {
        self.start_bar
    }
}

/// 走势几何方向 → 操作极性（Up=Long / Down=Short）。
pub fn dir_to_polarity(d: Direction) -> Polarity {
    match d {
        Direction::Up => Polarity::Long,
        Direction::Down => Polarity::Short,
    }
}

/// 极性反转（ε：短差腿反父向 / flip）。
pub fn flip_pol(p: Polarity) -> Polarity {
    match p {
        Polarity::Long => Polarity::Short,
        Polarity::Short => Polarity::Long,
    }
}

// ════════════════════════════ 持仓三阶段（第31课，单 campaign）════════════════════════════

/// 持仓成本三阶段（= flat 引擎 `super::t_engine::FlatTStage`；#889 R8 改名后同名撞车已消。
/// rank 0/1/2 同构，但迁移条件两套独立实装、一致性未正式核对——划界见 `FlatTStage` 文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStage {
    CostReduction,
    CapitalRecovered,
    EarningShares,
}

impl RecStage {
    pub fn as_u8(self) -> u8 {
        match self {
            RecStage::CostReduction => 0,
            RecStage::CapitalRecovered => 1,
            RecStage::EarningShares => 2,
        }
    }
}

// ════════════════════════════ NAV/TW 中性会计原语（= flat add_at/reduce_at）════════════════════════════

/// 减仓 m：按 direction 双重会计，现金回 free，返回 realized pnl。NAV 中性。
fn rec_reduce(inst: &mut TInstance, m: f64, free: &mut f64, c: f64) -> f64 {
    let pnl = match inst.direction {
        Polarity::Long => m * (c - inst.basis),
        Polarity::Short => m * (inst.basis - c),
    };
    match inst.direction {
        Polarity::Long => *free += m * c,
        Polarity::Short => *free -= m * c,
    }
    inst.units -= m;
    if inst.units <= EPS {
        inst.units = 0.0;
        inst.basis = f64::NAN;
    }
    pnl
}

/// 加仓 m（方向 dir）：现金从 free，basis 加权。NAV 中性。
fn rec_add(inst: &mut TInstance, m: f64, dir: Polarity, free: &mut f64, c: f64) {
    match dir {
        Polarity::Long => *free -= m * c,
        Polarity::Short => *free += m * c,
    }
    if inst.units > EPS {
        assert_eq!(
            inst.direction, dir,
            "rec_add 层内单一方向违反：向 {:?} 实例加 {:?}",
            inst.direction, dir
        );
        inst.basis = (inst.basis * inst.units + c * m) / (inst.units + m);
    } else {
        inst.direction = dir;
        inst.basis = c;
    }
    inst.units += m;
}

/// 配额 = units/3（= flat mobile_quota）。
fn quota(units: f64) -> f64 {
    units * MOBILE_FRAC
}

/// **逐级内在配额比例 `mobile_frac(L_pullback, L_confirm)`（task#84 子5，split 自适应定理实装）**。
///
/// 把固定 σ-不变常数 `MOBILE_FRAC=1/3`（拍扁）替换为**两个级别读数的函数**（零件1 v3 唯一非拍扁形式，
/// tower_intrinsic_quota §1.1/§3.2）。两自变量是 `ConfDepth` 三值代理（type1=全深度 d_top 区间套链贯通 /
/// type3=单层 t3sell 转折 / type2 居中，#95 §1.1 L_confirm 第四轴可观测端）：
///
/// - **L_pullback（回调深度的内在形式）**：触发该操作的级别买卖点深度——T1(d_top 深=走势完成级)=**高**，
///   T3(t3sell 浅=单层转折级)=**低**，T2=中（#76 §2.1：d_top↔高 L_pullback / t3sell↔低 L_pullback）。
/// - **L_confirm（区间套确认深度/滞后的内在形式）**：捕获该回调的确认有多快——T1/T3 = 单层/全深度区间套
///   **单次定位** = 快触发 = **低滞后**（可操作）；T2(其余,须更多同侧证据) = **高滞后**（#76 §2.2：链快触发=低 /
///   须深层=高）。**低 L_confirm = 机动可操作（α*_k>0）；高 L_confirm = 机动套牢（α*_k→0）**。
///
/// **split 自适应（§3.2）**：`L_pullback 高 ∧ L_confirm 低 ⇒ f 大`（深回调+快确认=机动捕获跌幅，骑主升浪）；
/// `L_pullback 低 ∨ L_confirm 高 ⇒ f→小`（浅回调/慢确认=核心 full，纯牛匹配 BH 防踏空）。
///
/// **形式（自相似形变，非新外部参数 no-hardcode）**：以 σ-不变常数 `MOBILE_FRAC=1/3` 为中性中点，沿两轴
/// 各档作**乘性形变**——L_pullback 深(T1)→×2、中(T2)→×1、浅(T3)→×1/2；L_confirm 低(T1/T3)→×3/2、高(T2)→×1/2。
/// 乘积裁剪到 (0, 1]（配额不超整仓）。唯一常数仍是 MOBILE_FRAC（1/λ 中枢三段），档位是级别尺度上的序读数
/// （T1≻T2≻T3 自相似 λ 级差），**零外部 θ_abs/振幅/regime 白名单**（编排者硬约束2 / 零件1 三种 split 表）。
///
/// **认识论**：函数形式 = L0（split 自适应分类，定义域=有效域）。**「自适应后超 BH」= L3 未决**（不声明膨胀，
/// tower_intrinsic_quota §3.3：实装后须 L3 验证；本函数只交付内在配额读出，不声明收益改善）。
fn mobile_frac(l_pullback: ConfDepth, l_confirm: ConfDepth) -> f64 {
    // L_pullback 轴：深回调(T1)→大配额捕获 / 浅回调(T3)→小配额防踏空。
    let pull_factor = match l_pullback {
        ConfDepth::T1 => 2.0, // 深（d_top 走势完成级）⇒ 高 L_pullback ⇒ 放大配额
        ConfDepth::T2 => 1.0, // 中
        ConfDepth::T3 => 0.5, // 浅（t3sell 单层转折级）⇒ 低 L_pullback ⇒ 缩小配额
    };
    // L_confirm 轴：快确认(T1/T3 单次区间套定位)→可操作放大 / 慢确认(T2 须更多证据)→套牢缩小。
    let confirm_factor = match l_confirm {
        ConfDepth::T1 | ConfDepth::T3 => 1.5, // 低滞后（单层/全深度单次定位）⇒ 机动可操作 ⇒ 放大
        ConfDepth::T2 => 0.5,                 // 高滞后（须深层确认）⇒ 机动套牢 ⇒ 缩小
    };
    let f = MOBILE_FRAC * pull_factor * confirm_factor;
    // 配额不超整仓（裁剪到 (0,1]）：纯牛 case 最小档 1/3×1/2×1/2=1/12>0，结构 case 最大档 1/3×2×3/2=1 整仓。
    f.clamp(f64::MIN_POSITIVE, 1.0)
}

// ════════════════════════════ T 实例（按 level 索引，= flat Layer + 骑走势节点）════════════════════════════

/// 单级别 T 实例（= flat `Layer` + `node`）。`instances[level]` 是该绝对级别的净仓位（idle 时 units=0）。
/// 单一 direction（相邻级别 sink 短差方向相反）。三阶段在 `TRoot`（单 campaign，非 per-instance）。
#[derive(Debug, Clone, Copy)]
pub struct TInstance {
    /// 骑的走势节点（递归结构：仓位骑走势而非纯 ladder index）。enter/sink/ascend 设。
    pub node: TrendNode,
    /// 绝对级别（= flat ladder index）。
    pub level: usize,
    pub direction: Polarity,
    /// 持有股数 ≥0（符号在 direction）。
    pub units: f64,
    /// per-实例 basis（强平 + reduce pnl 用）。
    pub basis: f64,
    /// **趋势底仓下限**（552号 HOLD_ANCHOR）：`sink`/`drain` 配额不作用于此份额——趋势底仓死扣吃大
    /// 趋势，只让机动仓（units−anchor）做次级别短差。`enter` 设 = m×(1−MOBILE_FRAC)，flag off 时恒 0
    /// ⇒ 机动仓=全仓 ⇒ 配额=units/3 与 OFF 逐字一致（bit-exact）。
    pub anchor: f64,
}

impl TInstance {
    fn idle(level: usize) -> Self {
        TInstance {
            node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            level,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            anchor: 0.0,
        }
    }
    pub fn is_active(&self) -> bool {
        self.units > EPS
    }
}

// ════════════════════════════ 读法B/读法乙递归：每级别独立腿（多重赋格，任务18 编排者修正）════════════════════════════

/// **每级别独立腿**（读法B/读法乙递归，移植自 4d6856dc75）：级别 k 的腿骑 `levels[k]` 走势，
/// 消费该级别 `d_top[k]`（走势完成链 OR 背驰段链）switch。每级别独立 = leg_3 骑 L3，leg_4 骑 L4，
/// 互不干预方向（547 隔离：每级别翻自己，删 `nearest_active_parent` 跨级路由）。
#[derive(Debug, Clone, Copy)]
pub struct Leg {
    pub level: usize,
    pub direction: Polarity,
    pub units: f64,
    pub basis: f64,
    pub riding_node: TrendNode,
    pub active: bool,
}

impl Leg {
    fn idle(level: usize) -> Self {
        Leg {
            level,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            riding_node: TrendNode::new(0, 0, 0.0, 0.0, Direction::Up),
            active: false,
        }
    }
    fn fingerprint(&self) -> (u64, bool, bool) {
        (
            self.units.to_bits(),
            self.direction == Polarity::Long,
            self.active,
        )
    }
}

// ════════════════════════════ LegPair：每级别一对多空腿（任务57=53.1 编排者重写）════════════════════════════

/// **每级别一对多空腿（LegPair，自相似）**：级别 k 同时持有一条多腿（吃涨）+ 一条空腿（吃跌）。
/// 牛市 = 大级别多腿（核心）+ 次级别空腿（吃回调）同时在场 = 多空双开 = 吃所有级别涨跌幅。
///
/// 三个缠论结构点（每腿）：
/// 1. **开** = 该级别买卖点（第17课买卖点定律一 + 第27课区间套定位）：买点 fire → 多腿开 / 卖点 fire → 空腿开。
///    开仓方向由缠论买卖点涌现，**不由 node.direction 几何方向**（删单腿模型的 dir_to_polarity(node.direction)）。
/// 2. **平** = 该级别反向买卖点（与开对称）：多腿持仓中卖点 fire → 多腿平 / 空腿持仓中买点 fire → 空腿平。
/// 3. **止损 = 否定线**（进场中枢边界 ZG/ZD）：多腿 stop = 进场 ZD（跌破=结构破坏）/ 空腿 stop = 进场 ZG（涨破=结构破坏）。
///    「回试回中枢=假突破」⟹ 止损。stop_line 在开仓时锁定（进场中枢边界，骑节点不变）。
///
/// **零 `if level==top` 硬编码**：方向/开/平/止损全由缠论结构（买卖点/中枢/否定线）涌现，自相似跨级别同构。
#[derive(Debug, Clone, Copy)]
pub struct LegPair {
    pub level: usize,
    /// 多腿股数 ≥0（idle=0）。
    pub long_units: f64,
    /// 多腿 basis（开仓价）。
    pub long_basis: f64,
    /// 多腿否定线 = 进场中枢 ZD（跌破止损）。NaN=未锁定/无中枢（无 ZD 否定线 ⇒ 仅靠反向买卖点平）。
    pub long_stop: f64,
    /// 空腿股数 ≥0（idle=0）。
    pub short_units: f64,
    /// 空腿 basis（开仓价）。
    pub short_basis: f64,
    /// 空腿否定线 = 进场中枢 ZG（涨破止损）。NaN=未锁定/无中枢。
    pub short_stop: f64,
    /// 多腿建仓 raw bar（per-element 会计 instrumentation #149，capture-ratio 验证工具）。
    /// observation-only：不参与任何仓位/方向/止损决策 ⇒ bit-exact 不变。idle=-1。
    pub long_entry_bar: i64,
    /// 空腿建仓 raw bar（同上）。idle=-1。
    pub short_entry_bar: i64,
    /// **L_confirm 第四轴捕获（task#95，observation-only）**：开多腿时锁定的确认深度 c（type1/type2，
    /// 多头无 type3 开仓）+ 是否核心腿（H⁰=`highest_active_long()==k`）。close 时据此归纤维格子。
    /// 不参与任何决策 ⇒ bit-exact。仅 `enable_lconfirm_audit` ON 时写入，OFF 恒默认（T2/false）。
    pub long_conf: ConfDepth,
    pub long_is_core: bool,
    /// 开空腿确认深度 c（type1/type2/type3）+ 是否核心腿（空腿在 #69 严格次级别开 ⇒ 通常非核心，
    /// 但 RB_PAIR_T3 放开 k<核心后核心翻空空腿可为核心 ⇒ leg 轴据此分 H⁰/H¹）。observation-only。
    pub short_conf: ConfDepth,
    pub short_is_core: bool,
}

impl LegPair {
    fn idle(level: usize) -> Self {
        LegPair {
            level,
            long_units: 0.0,
            long_basis: f64::NAN,
            long_stop: f64::NAN,
            short_units: 0.0,
            short_basis: f64::NAN,
            short_stop: f64::NAN,
            long_entry_bar: -1,
            short_entry_bar: -1,
            long_conf: ConfDepth::T2,
            long_is_core: false,
            short_conf: ConfDepth::T2,
            short_is_core: false,
        }
    }
    pub fn long_active(&self) -> bool {
        self.long_units > EPS
    }
    pub fn short_active(&self) -> bool {
        self.short_units > EPS
    }
    /// 是否最高活跃多腿级别（结构涌现「核心」= 当前最高活跃 up-trend 多腿，无 `if level==top` 硬编码）。
    fn pair_fingerprint(&self) -> (u64, u64) {
        (self.long_units.to_bits(), self.short_units.to_bits())
    }
}

/// **prove_leg_isolation（547 隔离，L0 结构 panic 守卫）**：`g(k)` 只写 `legs[k]`，断言 `legs[j≠k]`
/// 在 g(k) 前后逐位不变（547 病灶「低级别信号越级翻动高级别主力」的结构否定）。
fn prove_leg_isolation(
    fp_pre: &[(u64, bool, bool); MAX_LEVEL],
    fp_post: &[(u64, bool, bool); MAX_LEVEL],
    k: usize,
) {
    for j in 0..MAX_LEVEL {
        if j == k {
            continue;
        }
        assert_eq!(
            fp_pre[j], fp_post[j],
            "547 隔离违反：g({k}) 改动了别级腿 legs[{j}]（每级别独立腿只许 g(k) 写 legs[k]）"
        );
    }
}

// ════════════════════════════ 信号视图（= flat TSignalView，按 level）════════════════════════════

/// 本次重跑信号视图（= flat `TSignalView`，索引 = level）。纯 BSP 驱动（不分 type1/2/3）。
#[derive(Debug, Clone)]
pub struct LevelView {
    /// 该 level 是否新增任意买点（fresh）。
    pub buy: [bool; MAX_LEVEL],
    /// 该 level 是否新增任意卖点（fresh）。
    pub sell: [bool; MAX_LEVEL],
    /// 该 level 是否新增 **type1 买点**（底背驰 = 下跌走势终完美，fresh）。
    /// 走势完成信号（campaign 边界重定义，546号死锁解锁）：与 `buy` 同源去重的**子集**（仅 type1）
    /// ⇒ 与 flat `TSignalView::t1buy` **bit-exact 对称**（同一结构完成事件，仅 level/ladder 索引偏移）。
    pub t1buy: [bool; MAX_LEVEL],
    /// 该 level 是否新增 **type1 卖点**（顶背驰 = 上涨走势终完美，fresh）。
    pub t1sell: [bool; MAX_LEVEL],
    /// 该 level 是否新增 **type3 卖点**（第三类卖点 = 向下突破中枢下沿 ZD + 回试高点不回中枢 = 真顶转折，fresh）。
    /// 真顶/假顶判别：突破中枢+回试不回（`detect_type3`：`leave.high<ZD ∧ pull.high<ZD`）=真转折开空腿；
    /// 假突破（回试回中枢 `pull.high≥ZD`）⇒ 无 type3 ⇒ 不开空（滤震荡假突破累积止损）。
    pub t3sell: [bool; MAX_LEVEL],
    /// 各 level 当前走势节点（enter/ascend/sink 骑节点用；None=该级无走势）。
    pub nodes: [Option<TrendNode>; MAX_LEVEL],
    /// T 迭代涌现上界 (level, 操作极性)——自下而上仓位涌现（= flat emergent_top）。None=本 bar 不升级。
    pub emergent_top: Option<(usize, Polarity)>,
    /// **命题4 读法乙——大级别背驰段闸门（self-top-down 区间套前提）**。
    /// 最高级别走势进入**背驰段**（`divergence::trend_candidate`：结构∧MACD 双确认，**未创新高**=
    /// 未走势完成，严格⊊type1）时置位为**操作极性**：上涨顶背驰段→Short（顶部，卖点 close+做空），
    /// 下跌底背驰段→Long（底部，买点 cover+做多）。None=最高级别未进入背驰段。
    /// 源头审计 src-prop13（第27课区间套）：区间套前提 = 大级别**背驰段**（非走势完成）。
    pub top_diverge: Option<Polarity>,
    /// 最高级别**当前走势几何方向**（NEST 反转去武装：armed 与当前 top 方向不一致 ⇒ top 已反转 ⇒ 清 armed）。
    /// **来源（F3 精确化）**：走势树最高非空层 `tree.levels[tl]` 的末走势 `direction`（`tl=rposition(非空 trends)`）。
    /// 该层 direction 由上游 nest/morphology 数据流（`emergent_dir`）决定——「nf/morphology 驱动」描述的是
    /// **上游数据流来源**（走势识别 morphology 决定走势方向），非本字段直接调用 nf。本字段是被动注入的投影值。
    pub top_trend_dir: Option<Direction>,
    /// **每级别背驰段操作极性**（多重赋格 + consume平空 + 严格逐级区间套，任务22）：
    /// `level_diverge[k]` = 级别 k 当前走势进入背驰段时的操作极性（顶背驰段→Short / 底背驰段→Long），None=该级未进背驰段。
    /// consume平空用：次级别（k<core）反核心向背驰段 ⇒ 平核心仓（缩短持仓）。严格逐级用：定位点须 top..loc 逐级背驰段一致。
    pub level_diverge: [Option<Polarity>; MAX_LEVEL],
    /// **每级别 d_top 区间套链贯通真顶/真底**（任务18 编排者修正：每级别独立腿多重赋格触发器）。
    /// `divergence::d_top(k, ..., use_diverge)`——`use_diverge=false`=走势完成链（556 读法B）/`true`=背驰段链（读法乙）。
    /// 读法B 路径（`enable_reading_b`）每级别独立腿 `legs[k]` 消费 `d_top[k]` switch（close+反向 open）。
    /// **任务69**：LegPair 路径（`enable_reading_b_pair`）**核心多腿 churn 门控**消费 `d_top`（最深区间套确认=走势完成
    /// 真顶，稀疏⇒保护主力骑牛不踏空）；次级别开空腿改用 `t3sell`（单层区间套转折，响应回调）⇒ 区间套确认深度按持仓尺度分级。
    pub d_top: [bool; MAX_LEVEL],
    /// **每级别当前走势末中枢核心区间 ZG（核心上沿，否定线原料）**（任务57=53.1 LegPair 止损）。
    /// 空腿止损线：价格涨破进场中枢 ZG ⇒ 向上突破=「回试回中枢=假突破」反面=结构破坏 ⇒ 空腿止损平。None=该级无中枢。
    pub zg: [Option<f64>; MAX_LEVEL],
    /// **每级别当前走势末中枢核心区间 ZD（核心下沿，否定线原料）**（任务57=53.1 LegPair 止损）。
    /// 多腿止损线：价格跌破进场中枢 ZD ⇒ 结构破坏（多头买入逻辑被否定，第17课区间套否定线）⇒ 多腿止损平。None=该级无中枢。
    pub zd: [Option<f64>; MAX_LEVEL],
}

impl LevelView {
    pub fn empty() -> Self {
        LevelView {
            buy: [false; MAX_LEVEL],
            sell: [false; MAX_LEVEL],
            t1buy: [false; MAX_LEVEL],
            t1sell: [false; MAX_LEVEL],
            t3sell: [false; MAX_LEVEL],
            nodes: [None; MAX_LEVEL],
            emergent_top: None,
            top_diverge: None,
            top_trend_dir: None,
            level_diverge: [None; MAX_LEVEL],
            d_top: [false; MAX_LEVEL],
            zg: [None; MAX_LEVEL],
            zd: [None; MAX_LEVEL],
        }
    }
}

impl Default for LevelView {
    fn default() -> Self {
        Self::empty()
    }
}

// ════════════════════════════ 递归 T 根（= flat TPositionEngine，单 free 池 + 单 campaign）════════════════════════════

/// 递归 T 引擎根（= flat `TPositionEngine`）：`instances[level]` 净仓位 + 单 free 池 + 单 campaign 三阶段。
pub struct TRoot {
    /// 各 level 净仓位（索引 = level）。
    instances: Vec<TInstance>,
    /// 单一共享现金池（NAV = free + Σ sign(d)·u·c）。
    free: f64,
    last_close: f64,

    // ── 持仓三阶段（单 campaign，= flat）──
    stage: RecStage,
    notional_in: f64,
    core_cost_basis: f64,
    campaign_entry_cost: f64,
    withdrawn: f64,
    earning_cash: f64,
    enable_earning: bool,
    enable_three_stage: bool,
    /// 核心走势完成清仓开关（546号死锁解锁的 A/B 消融门）：env `T_NO_TREND_DONE_CLEAR` 置位 ⇒ false
    /// （走势完成不清仓 = 死锁基线），默认 true。与 `T_NO_EMERGENCE` 等同类 eval 工具，flat/rec 对称。
    enable_trend_done_clear: bool,
    /// **ANCHOR**（552号 HOLD_ANCHOR）：enter 划趋势底仓 anchor=m×2/3，sink/drain 配额只作机动仓。OFF=false。
    enable_hold_anchor: bool,
    /// **NEST**（命题4 读法乙）：旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转。OFF=false。
    enable_nest: bool,
    /// **consume平空（任务22）**：次级别反核心向背驰段 ⇒ 平核心仓 1/3。
    enable_nest_consume: bool,
    /// **严格逐级区间套（任务22 开放轴C）**：主翻转定位点须 top..loc 逐级背驰段一致。
    enable_nest_strict: bool,
    /// NEST armed 操作极性（大级别背驰段武装，跨重跑持续至 top 反转 / 翻转消费）。None=未武装。
    nest_armed_op: Option<Polarity>,
    /// NEST 当前背驰段窗口是否已翻转（**每窗口仅翻一次** = 读法乙区间套定位一个转折点，非读法甲全 a0 穷尽）。
    /// 新窗口（top_diverge 极性变 / top 反转去武装）⇒ false；翻转 ⇒ true。
    nest_consumed: bool,
    /// **读法B/读法乙递归（任务18）**：每级别独立腿 + 触发器。enable_reading_b=true ⇒ on_bar 走 consume_legs。
    enable_reading_b: bool,
    reading_b_diverge: bool,
    /// **读法B 一对多空腿（任务57=53.1）**：enable_reading_b_pair=true ⇒ on_bar 走 consume_leg_pairs。
    enable_reading_b_pair: bool,
    /// **牛熊对称核心翻空（任务 bear-validate）**：true ⇒ g_pair 放开 `k<核心`，核心走势完成翻空镜像。
    enable_pair_core_short: bool,
    /// **放开 k<核心 t3sell 开空（任务 bear-validate 变体2）**：true ⇒ 移除 below_core_long 门控（t3sell 核心及其上开空）。
    enable_pair_core_short_open: bool,
    /// **Face B 均匀基准单元定仓（做空腿赚 #110）**：true ⇒ open_*_leg 用 uniform_base_units 取代
    /// geom_tower_quota（删恒仓归一化压制 → 来源A 杠杆涌现 + 真否定线封顶）。OFF=false（bit-exact）。
    enable_uniform_sizing: bool,
    /// **做空腿开仓触发器（做空腿对称化，#108 根因）**：T3=#117 基线（view.t3sell）/ T1=候选A（view.t1sell）
    /// / Any=候选B（view.sell，对称多头 view.buy）。g_pair 开空门控用，below_core_long 门不变。
    pair_short_entry: ShortEntry,
    /// **开多腿入场方向/级别门控（T1 方向错位修复，#164/R1）**：Any=现状零门控（bit-exact）/
    /// SegAlign=同级别段方向对齐 / CrossLevel=跨级别段方向对齐（攻 65.5% xlc）。g_pair 开多门控用。
    pair_long_entry: LongEntry,
    /// **LegPair 核心多腿涌现升级（R3 段无腿修复 #164/#6）**：true ⇒ consume_leg_pairs 检查 emergent_top，
    /// 核心多腿同向 relabel 上移到涌现级别（敞口不变）。OFF=false（bit-exact，无 relabel）。
    enable_pair_emergence: bool,
    /// **9 轨道操作分派（task#40 B，#39 O1-O9）**：true ⇒ route_bsp 显式 9 轨道判别 + 补 O3 add 缺口
    /// （= 买回/卖回 = 不动 h 同级别短差腿部分重建）。account 过滤是 route 之后独立 gate（route 内零账本，
    /// 空头镜像一律执行）。OFF=false ⇒ route_bsp 旧二元 bit-exact（facea 54279a503e）。env `T_ORBIT9_DISPATCH`。
    enable_orbit9_dispatch: bool,
    /// **节点向量路由（task#63，#61 §五-§七）**：true ⇒ route_bsp 读节点在 r* 的级别角色 R+/R−，对 R−
    /// 回调腿的 sink 强制核心保护（机动不动核心）。OFF=false ⇒ bit-exact。env `T_ORBIT9_NODEVEC`。
    enable_orbit9_nodevec: bool,
    /// **L_confirm 第四轴会计开关（task#95，observation-only）**：true ⇒ open/close_*_leg 捕获 c/leg 并累加
    /// 纤维格子 `pi_k_leg_dir_c`。OFF=false ⇒ 不写新数组（恒 0/默认）= bit-exact。env `T_LCONFIRM_AUDIT`。
    enable_lconfirm_audit: bool,
    /// **逐级内在配额开关（task#84 子5）**：true ⇒ sink/drain/add 配额走 `mobile_frac(L_pullback,L_confirm)`
    /// 逐级内在函数（替固定 1/3，ON 偏离 542 σ-不变 = 597 子5 授权的概念分离实装侧）。OFF=false ⇒ 走
    /// `quota()`(σ-不变 1/3) + `prove_sigma_quota` = bit-exact。env `T_INTRINSIC_QUOTA`。
    enable_intrinsic_quota: bool,
    /// **payoff G 轴开关（task#5，539 失血修复维）**：true ⇒ g_pair 开腿叠加级别-方向对齐门（不开方向
    /// 误读孤儿腿）+ 强平可达性（G 轴每腿锁否定线 zd/zg ⇒ 孤儿腿可达闭合）。OFF=false ⇒ 不读 nodes[k]
    /// 方向、不锁 G 轴否定线 = bit-exact。env `T_G_AXIS`。详见 `EngineConfig::enable_g_axis`。
    enable_g_axis: bool,
    /// **逐级 L_pullback/L_confirm 暂存（task#84 子5，仅 enable_intrinsic_quota ON 时由 on_bar 注入）**：
    /// route_bsp/sink/drain/add 不接收 view（架构现状），故在 on_bar 起始把每级别 ConfDepth 代理读出暂存于此，
    /// sink/drain/add 读 `cur_l_pullback[k]`/`cur_l_confirm[k]` 算 mobile_frac。**OFF ⇒ on_bar 不写（恒默认 T2）=
    /// 不影响任何 OFF 路径 = bit-exact**（新字段无 OFF 消费者）。L_pullback=触发腿深度代理 / L_confirm=确认滞后代理。
    cur_l_pullback: [ConfDepth; MAX_LEVEL],
    cur_l_confirm: [ConfDepth; MAX_LEVEL],
    /// **T3 出场触发诊断暂存（#164 R2，observation-only）**：close_*_leg 调用前由调用点 set，push 进
    /// exit_trigger_log（与 leg_trades 同序）。0=type1/1=type2/2=type3/3=否定线止损/5=账户强平/6=其他。
    pending_exit_trigger: u8,
    /// 初始本金（= free 初值）——Face B 均匀基准单元定仓的级别无关基准（initial_capital×MOBILE_FRAC）。
    initial_capital: f64,
    /// 每级别独立腿（legs[k] 骑 levels[k] 走势消费 d_top[k]）。
    legs: Vec<Leg>,
    /// **每级别一对多空腿（LegPair[k]，任务57=53.1）**：买卖点开平 + 否定线止损 + 链破坏 churn 门控。
    leg_pairs: Vec<LegPair>,
    /// LegPair per-level realized pnl（验收哪级别多/空腿赚/亏）。
    pub pair_long_pnl: [f64; MAX_LEVEL],
    pub pair_short_pnl: [f64; MAX_LEVEL],
    /// LegPair 观测计数（纯诊断）。
    pub pair_long_opens: [u64; MAX_LEVEL],
    pub pair_short_opens: [u64; MAX_LEVEL],
    pub pair_long_closes: [u64; MAX_LEVEL],
    pub pair_short_closes: [u64; MAX_LEVEL],
    /// 否定线止损次数（多/空腿，纯观测——验收止损先于 NAV≤0）。
    pub pair_long_stops: u64,
    pub pair_short_stops: u64,
    /// 链破坏 churn 触发核心多腿翻转次数（纯观测——解 churn 门控是否解冻核心）。
    pub pair_core_churns: u64,
    /// **G 轴分量① 级别-方向对齐门拦截次数（task#5，纯观测）**：g_pair 因方向误读（开多在 d_k=Down /
    /// 开空在 d_k=Up）被 G 轴对齐门拦截而**未开**的腿数。OFF 恒 0。验收 539 方向误读孤儿被堵的频率。
    pub g_axis_align_blocked: u64,
    /// **G 轴分量② 强平可达性否定线锁定次数（task#5，纯观测）**：G 轴 ON 时给开成的腿锁定 zd/zg 否定线
    /// （可达强平路径）的次数。OFF 恒 0。验收 G 轴每腿有可达闭合路径（非仅 NAV≤0 不可达闭合）。
    pub g_axis_reach_stops_set: u64,
    /// **核心多腿涌现升级（R3 段无腿 #164/#6）**：relabel 上移次数 / 方向不匹配跳过次数（纯观测）。
    pub pair_emergence_upgrades: u64,
    pub pair_emergence_skipped_dir: u64,
    /// 读法B 每级别 per-level realized pnl（验收哪级别腿赚/亏）。
    pub per_level_long_pnl: [f64; MAX_LEVEL],
    pub per_level_short_pnl: [f64; MAX_LEVEL],
    /// 读法B 每级别腿切换次数（d_top[k] 驱动 close+reopen，纯观测——解 556 顶层腿是否冻结）。
    pub leg_switches_by_level: [u64; MAX_LEVEL],
    pub leg_opens_by_level: [u64; MAX_LEVEL],
    /// 读法B 杠杆验收（裂隙2 异质质询）：max 毛敞口 / max 净敞口（相对 NAV，×100 整数存）。
    /// 恒仓声明 = max_gross ≤ ~100（≤1×）。>100 = 杠杆（ES+681%可能伪影）。
    pub max_gross_exp_x100: u64,
    pub max_net_exp_x100: u64,

    // ── 观测计数（纯诊断）──
    pub n_enters: u64,
    pub n_sinks: u64,
    pub n_recovers: u64,
    /// **O3 add 次数（task#40 B，= 买回/卖回 = 不动 h 同级别短差腿部分重建）**。OFF 恒 0（add 分支未激活）。
    pub n_adds: u64,
    /// O3 add per-level realized（部分平 sub 短差的 pnl，验收买回腿赚/亏；τ 镜像多空对称）。
    pub add_pnl_by_level: [f64; MAX_LEVEL],
    pub n_drains: u64,
    pub n_flips: u64,
    /// 核心走势完成清仓次数（546号死锁解锁路径触发计数，纯观测）。
    pub n_trend_done_clears: u64,
    pub n_ascends: u64,
    /// **NEST 翻转次数**（命题4 读法乙：大级别背驰段闸门 a0 定位翻转）。诊断 556 顶层是否解冻。
    pub n_nest_flips: u64,
    /// **NEST consume平空次数**（任务22：次级别反核心向背驰段平核心仓 1/3）。诊断是否缩短长持死扣。
    pub n_nest_consumes: u64,
    /// consume平空累计 realized（缩短持仓的平仓 pnl，验收减穿仓）。
    pub nest_consume_pnl: f64,
    /// NEST 逐笔（is_short, entry_bar, entry_px, exit_bar, exit_px, realized_pnl）——539 做空腿逐笔验收。
    pub nest_trades: Vec<(bool, i64, f64, i64, f64, f64)>,
    pub n_emergence_upgrades: u64,
    /// 编排者排查 2026-06-21：emergence_upgrade 统计——核心低于涌现级别本可升级的次数 / 方向不匹配跳过。
    pub n_emergence_attempts: u64,
    pub n_emergence_skipped_dir: u64,
    /// 核心 flip 序列（bar, from_dir, to_dir, j_level, is_buy）：查核心被低级别 BSP ping-pong。
    pub flip_log: Vec<(i64, Polarity, Polarity, usize, bool)>,
    /// 当前 bar（on_bar 每 bar 设，flip_log 用）。
    pub cur_bar: i64,
    /// 编排者排查 2026-06-21：sink/recover 路由对称性——per-level sink/recover + 买点路由分类
    /// （查为什么 recover(1348)<sink(2671)：哪些买点没被消费为 recover）。
    pub sink_by_level: [u64; MAX_LEVEL],
    pub recover_by_level: [u64; MAX_LEVEL],
    pub buy_recover: u64, // 父多+买点+j active Short → recover
    pub buy_noop: u64,    // 父多+买点+j 无短差 → no-op（浪费的买点）
    pub buy_sink: u64,    // 父空+买点 → sink（核心 Short 时减仓）
    pub buy_core: u64,    // 核心级买点 → enter/ascend/flip
    pub n_liquidations: u64,
    pub n_capital_recovered: u64,
    pub n_earning_deploys: u64,
    pub short_leg_pnl: f64,
    /// 短差腿 per-level realized（验收：哪个级别短差赚/亏，编排者 per-level P&L 追踪）。
    pub short_pnl_by_level: [f64; MAX_LEVEL],
    /// 强平 episode 诊断（编排者：查回补失败=空头没匹配买点被强平）：
    /// (level, 开空节点起点 bar, 开空价 basis, 强平 bar, 强平价, 是否空头)。
    pub liq_log: Vec<(usize, i64, f64, i64, f64, bool)>,
    /// #149 per-element 会计 instrumentation（capture-ratio 验证工具，observation-only）：
    /// LegPair 每笔多/空腿平仓的完整逐笔记录，供 per-(级别,走势段,方向) 捕获率归因。
    /// (level, entry_bar_raw, exit_bar_raw, entry_price, exit_price, units, is_short, realized_pnl)。
    /// entry/exit_bar = raw bar（确认时点 cur_bar）；Σ(此 log 多头 pnl)==Σ pair_long_pnl，空头同（完整性自检）。
    pub leg_trades: Vec<(usize, i64, i64, f64, f64, f64, bool, f64)>,
    /// #170 TC churn 诊断（平行于 leg_trades，同索引）：每笔平仓的触发源
    /// 0=买卖点反向平(g_pair) / 1=否定线止损(pair_stop_loss_step) / 2=NAV强平或finish收尾。
    /// observation-only：用于拆解 1-bar collapse 的因果链（卖点 churn vs 止损 vs 收尾）。
    pub leg_close_reasons: Vec<u8>,
    /// #170 TC churn 诊断：close 前由各 call site 设置的触发源（close_*_leg push 时消费）。
    pub last_close_reason: u8,
    /// #170 TC churn 诊断（平行于 leg_trades）：每笔开仓时锁定的否定线（多腿=ZD/空腿=ZG）。
    /// NaN=无否定线。用于验证 TC=开仓贴否定线（c 距 stop 极近 ⇒ 噪声微穿即止损 collapse）。
    pub leg_entry_stops: Vec<f64>,
    /// **T3 出场触发诊断（#164 R2 可约性判定，observation-only，与 leg_trades 同序对齐）**：每笔腿平仓的
    /// 出场触发类型 u8——0=type1（走势完成=574 floor 最小滞后）/1=type2/2=type3/3=否定线止损/5=账户强平/6=其他。
    /// 判据：T3 笔若主要被 type1 平 ⇒ leak 在 574 floor 内不可约；若被 type2/3 平 ⇒ floor 之上可约。
    pub exit_trigger_log: Vec<u8>,
    /// 诊断（编排者 2026-06-21）：强平时三阶段快照 (stage_id 0=CostRed/1=CapRec/2=Earn,
    /// core_cost_basis, withdrawn, notional_in, nav)。查降成本/退本金保护是否生效 + 全仓 vs 逐仓。
    pub liq_snapshot: Vec<(u8, f64, f64, f64, f64)>,
    pub earning_units_added: f64,
    pub max_core_gain_x1000: u64,

    /// **L_confirm 第四轴纤维会计（task#95 W-lconfirm，observation-only）**：per `(级别 k × 腿 leg × 方向 dir
    /// × 确认深度 c)` 累计 realized P&L。索引：`[k][leg][dir][c]`，leg: 0=H⁰核心/1=H¹短差，dir: 0=R+(long)/
    /// 1=R−(short)，c: 0=T1/1=T2/2=T3（ConfDepth）。codex #92 审计断点：同 (k,leg,dir) cell 内 P&L 符号因 c
    /// 不同而不定 ⇒ 三轴 K×L×D 分类不完备。本会计跑 L3 判定第四轴 c 成立（符号分裂）还是被否证（各 c 桶符号一致）。
    /// 纯观测：不参与任何决策 ⇒ OFF/ON 决策路径 bit-exact（新数组无消费者，仅 close_*_leg 累加）。
    pub pi_k_leg_dir_c: [[[[f64; 3]; 2]; 2]; MAX_LEVEL],
    /// per-cell 平仓笔数（纯观测，验收每个 (k,leg,dir,c) 格子的样本量，符号分裂判定的统计基础）。
    pub pi_count_k_leg_dir_c: [[[[u64; 3]; 2]; 2]; MAX_LEVEL],
    /// **机械穷尽守卫累计量（task#95，补 #92 缺的可结算底座）**：`pi_total_audited` = 所有进入纤维格子的
    /// realized P&L 总和（应 == Σ pair_long_pnl + Σ pair_short_pnl = 总 leg realized P&L）。不等 ⇒ 有未分类
    /// 的 P&L 流（分类不完备的可验证信号）。`assert_lconfirm_exhaustive()` 检验。
    pub pi_total_audited: f64,

    /// prove 守卫族（编排者裁决 2026-06-21）：BSP 触发归因（panic）+ sink/recover 平衡 / per-level
    /// 短差 pnl / 核心方向匹配（观测计数）。见 `prove_guards.rs`。
    guards: ProveGuards,
}

impl TRoot {
    /// 默认从 env 构造（= 当前 main 行为）。
    pub fn new(initial_capital: f64) -> Self {
        Self::new_with_config(initial_capital, EngineConfig::from_env())
    }

    /// 显式配置构造（受控实验：OFF / ANCHOR / NEST 单进程多变体）。
    pub fn new_with_config(initial_capital: f64, cfg: EngineConfig) -> Self {
        let instances = (0..MAX_LEVEL).map(TInstance::idle).collect();
        TRoot {
            instances,
            free: initial_capital,
            last_close: f64::NAN,
            stage: RecStage::CostReduction,
            notional_in: 0.0,
            core_cost_basis: f64::NAN,
            campaign_entry_cost: f64::NAN,
            withdrawn: 0.0,
            earning_cash: 0.0,
            enable_earning: cfg.enable_earning,
            enable_three_stage: cfg.enable_three_stage,
            enable_trend_done_clear: cfg.enable_trend_done_clear,
            enable_hold_anchor: cfg.enable_hold_anchor,
            enable_nest: cfg.enable_nest,
            enable_nest_consume: cfg.enable_nest_consume,
            enable_nest_strict: cfg.enable_nest_strict,
            nest_armed_op: None,
            nest_consumed: false,
            enable_reading_b: cfg.enable_reading_b,
            reading_b_diverge: cfg.reading_b_diverge,
            enable_reading_b_pair: cfg.enable_reading_b_pair,
            enable_pair_core_short: cfg.enable_pair_core_short,
            enable_pair_core_short_open: cfg.enable_pair_core_short_open,
            pair_short_entry: cfg.pair_short_entry,
            pair_long_entry: cfg.pair_long_entry,
            enable_uniform_sizing: cfg.enable_uniform_sizing,
            enable_pair_emergence: cfg.enable_pair_emergence,
            enable_orbit9_dispatch: cfg.enable_orbit9_dispatch,
            enable_orbit9_nodevec: cfg.enable_orbit9_nodevec,
            enable_lconfirm_audit: cfg.enable_lconfirm_audit,
            enable_intrinsic_quota: cfg.enable_intrinsic_quota,
            enable_g_axis: cfg.enable_g_axis,
            cur_l_pullback: [ConfDepth::T2; MAX_LEVEL],
            cur_l_confirm: [ConfDepth::T2; MAX_LEVEL],
            pending_exit_trigger: 6,
            initial_capital,
            legs: (0..MAX_LEVEL).map(Leg::idle).collect(),
            leg_pairs: (0..MAX_LEVEL).map(LegPair::idle).collect(),
            pair_long_pnl: [0.0; MAX_LEVEL],
            pair_short_pnl: [0.0; MAX_LEVEL],
            pair_long_opens: [0; MAX_LEVEL],
            pair_short_opens: [0; MAX_LEVEL],
            pair_long_closes: [0; MAX_LEVEL],
            pair_short_closes: [0; MAX_LEVEL],
            pair_long_stops: 0,
            pair_short_stops: 0,
            pair_core_churns: 0,
            g_axis_align_blocked: 0,
            g_axis_reach_stops_set: 0,
            pair_emergence_upgrades: 0,
            pair_emergence_skipped_dir: 0,
            per_level_long_pnl: [0.0; MAX_LEVEL],
            per_level_short_pnl: [0.0; MAX_LEVEL],
            leg_switches_by_level: [0; MAX_LEVEL],
            leg_opens_by_level: [0; MAX_LEVEL],
            max_gross_exp_x100: 0,
            max_net_exp_x100: 0,
            n_enters: 0,
            n_sinks: 0,
            n_recovers: 0,
            n_adds: 0,
            add_pnl_by_level: [0.0; MAX_LEVEL],
            n_drains: 0,
            n_flips: 0,
            n_trend_done_clears: 0,
            n_ascends: 0,
            n_nest_flips: 0,
            n_nest_consumes: 0,
            nest_consume_pnl: 0.0,
            nest_trades: Vec::new(),
            n_emergence_upgrades: 0,
            n_emergence_attempts: 0,
            n_emergence_skipped_dir: 0,
            flip_log: Vec::new(),
            cur_bar: 0,
            sink_by_level: [0; MAX_LEVEL],
            recover_by_level: [0; MAX_LEVEL],
            buy_recover: 0,
            buy_noop: 0,
            buy_sink: 0,
            buy_core: 0,
            n_liquidations: 0,
            n_capital_recovered: 0,
            n_earning_deploys: 0,
            short_leg_pnl: 0.0,
            short_pnl_by_level: [0.0; MAX_LEVEL],
            liq_log: Vec::new(),
            leg_trades: Vec::new(),
            leg_close_reasons: Vec::new(),
            last_close_reason: 0,
            leg_entry_stops: Vec::new(),
            exit_trigger_log: Vec::new(),
            liq_snapshot: Vec::new(),
            earning_units_added: 0.0,
            max_core_gain_x1000: 0,
            pi_k_leg_dir_c: [[[[0.0; 3]; 2]; 2]; MAX_LEVEL],
            pi_count_k_leg_dir_c: [[[[0; 3]; 2]; 2]; MAX_LEVEL],
            pi_total_audited: 0.0,
            guards: ProveGuards::new(MAX_LEVEL),
        }
    }

    /// prove 守卫只读访问（验收报告/测试）。
    pub fn guards(&self) -> &ProveGuards {
        &self.guards
    }

    // ──────────────── 守恒（根级别）────────────────

    /// 总 NAV = free + Σ sign(d)·u·c（不含 withdrawn）。
    pub fn nav(&self, c: f64) -> f64 {
        let mut v = self.free;
        for inst in &self.instances {
            if inst.units > 0.0 {
                // = flat `nav`（`l.units > 0.0`，非 1e-12）。
                v += match inst.direction {
                    Polarity::Long => inst.units * c,
                    Polarity::Short => -inst.units * c,
                };
            }
        }
        // 读法B/读法乙递归：每级别独立腿（instances 与 legs 按模式互斥，无双计）。
        for leg in &self.legs {
            if leg.units > 0.0 {
                v += match leg.direction {
                    Polarity::Long => leg.units * c,
                    Polarity::Short => -leg.units * c,
                };
            }
        }
        // 读法B 一对多空腿（LegPair[k]，任务57=53.1）：多腿 +u·c / 空腿 −u·c。OFF 时全 idle ⇒ 0（bit-exact）。
        for p in &self.leg_pairs {
            if p.long_units > 0.0 {
                v += p.long_units * c;
            }
            if p.short_units > 0.0 {
                v -= p.short_units * c;
            }
        }
        v
    }

    /// 总财富 TW = NAV + withdrawn（逐 bar 唯一守恒量）。
    pub fn total_wealth(&self, c: f64) -> f64 {
        self.nav(c) + self.withdrawn
    }

    fn prove_tw_neutral(&self, tw_pre: f64, c: f64) {
        let tw_post = self.total_wealth(c);
        let tol = 1e-4 * tw_pre.abs().max(1.0);
        assert!(
            (tw_post - tw_pre).abs() <= tol,
            "TW 中性违反：pre={tw_pre} post={tw_post} (c={c})"
        );
    }

    pub fn free(&self) -> f64 {
        self.free
    }
    pub fn withdrawn_total(&self) -> f64 {
        self.withdrawn
    }
    pub fn stage(&self) -> RecStage {
        self.stage
    }
    pub fn last_close(&self) -> f64 {
        self.last_close
    }

    /// 实例只读访问（诊断/测试）。
    pub fn instance(&self, level: usize) -> &TInstance {
        &self.instances[level]
    }
    /// 活跃实例数（诊断/测试）。
    pub fn n_active(&self) -> usize {
        self.instances.iter().filter(|i| i.is_active()).count()
    }
    /// 暴露 (long_units, short_units)。
    pub fn exposure(&self) -> (f64, f64) {
        let mut lu = 0.0;
        let mut su = 0.0;
        for inst in &self.instances {
            if inst.units > 0.0 {
                // = flat `exposure`（`l.units > 0.0`，非 1e-12）。
                match inst.direction {
                    Polarity::Long => lu += inst.units,
                    Polarity::Short => su += inst.units,
                }
            }
        }
        (lu, su)
    }

    // ──────────────── nearest_active_parent / highest_active（= flat）────────────────

    /// 最高活跃级别（= flat highest_active）。无 → None。
    pub fn highest_active(&self) -> Option<usize> {
        (0..MAX_LEVEL)
            .rev()
            .find(|&k| self.instances[k].is_active())
    }

    /// level j 的最近活跃祖先（严格更高的第一个 active）= flat nearest_active_parent。无 → None（核心级）。
    fn nearest_active_parent(&self, j: usize) -> Option<usize> {
        (j + 1..MAX_LEVEL).find(|&k| self.instances[k].is_active())
    }

    /// 核心多头 units（Σ 多头）= 降成本分母。
    fn core_long_units(&self) -> f64 {
        self.instances
            .iter()
            .filter(|l| l.units > EPS && l.direction == Polarity::Long)
            .map(|l| l.units)
            .sum()
    }

    // ──────────────── 三阶段会计（= flat account_reduce）────────────────

    /// 核算一次 reduce 的 realized（= flat account_reduce）。`dir` = 被减层方向。
    /// Long reduce（核心高位卖出）= 降成本；Short reduce（短差腿）= short_leg_pnl 单独算。
    fn account_reduce(&mut self, dir: Polarity, realized: f64, c: f64) {
        match dir {
            Polarity::Long => match self.stage {
                RecStage::CostReduction => {
                    let rem = self.core_long_units();
                    if rem > 1e-9 && self.core_cost_basis.is_finite() {
                        self.core_cost_basis -= realized / rem;
                        if self.campaign_entry_cost > 1e-9 {
                            let drop = ((self.campaign_entry_cost - self.core_cost_basis)
                                / self.campaign_entry_cost
                                * 1000.0)
                                .max(0.0) as u64;
                            self.max_core_gain_x1000 = self.max_core_gain_x1000.max(drop);
                        }
                        if self.core_cost_basis <= 0.0 && self.enable_three_stage {
                            self.stage = RecStage::CapitalRecovered;
                            self.n_capital_recovered += 1;
                            self.try_withdraw_capital(c);
                        }
                    }
                }
                RecStage::CapitalRecovered => self.try_withdraw_capital(c),
                RecStage::EarningShares => {
                    if realized > 0.0 {
                        self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                    }
                }
            },
            Polarity::Short => {
                self.short_leg_pnl += realized;
                if self.stage == RecStage::EarningShares && realized > 0.0 {
                    self.earning_cash = (self.earning_cash + realized).min(self.free.max(0.0));
                }
            }
        }
    }

    /// 退本金（= flat try_withdraw_capital）：free→withdrawn 移出 = 初始本金 K。
    fn try_withdraw_capital(&mut self, _c: f64) {
        let need = self.notional_in - self.withdrawn;
        if need <= 1e-9 {
            self.enter_earning();
            return;
        }
        let w = need.min(self.free.max(0.0));
        if w > 1e-12 {
            self.withdrawn += w;
            self.free -= w;
        }
        if self.withdrawn >= self.notional_in - 1e-9 {
            self.enter_earning();
        }
    }

    fn enter_earning(&mut self) {
        if self.stage != RecStage::EarningShares {
            self.stage = RecStage::EarningShares;
        }
    }

    /// 增股数部署（= flat deploy_earning）：EarningShares 买点把 earning_cash 全额买成核心 units。
    fn deploy_earning(&mut self, core: usize, c: f64) {
        if !self.enable_earning || self.stage != RecStage::EarningShares || c <= 0.0 {
            return;
        }
        let cash = self.earning_cash.min(self.free.max(0.0));
        let q = cash / c;
        if !(q > 1e-12 && q.is_finite()) {
            return;
        }
        if self.instances[core].is_active() && self.instances[core].direction != Polarity::Long {
            return;
        }
        let mut free = self.free;
        rec_add(&mut self.instances[core], q, Polarity::Long, &mut free, c);
        self.free = free;
        self.earning_cash -= q * c;
        self.n_earning_deploys += 1;
        self.earning_units_added += q;
    }

    /// campaign 结束（flip/clear/eod）：归还 withdrawn、重置三阶段（= flat reset_campaign）。
    fn reset_campaign(&mut self) {
        // campaign 终点（核心走势完成）：prove_sink_recover_balance + prove_per_level_pnl（观测）。
        self.guards.campaign_end();
        self.free += self.withdrawn;
        self.withdrawn = 0.0;
        self.stage = RecStage::CostReduction;
        self.notional_in = 0.0;
        self.core_cost_basis = f64::NAN;
        self.campaign_entry_cost = f64::NAN;
        self.earning_cash = 0.0;
    }

    // ──────────────── τ 原子（= flat enter/clear_all/ascend/sink/recover/drain）────────────────

    /// **enter**（= flat enter）：核心级空仓首次建仓，全 free 建 `dir` 方向核心仓 @ level。
    fn enter(&mut self, level: usize, dir: Polarity, node: TrendNode, c: f64) {
        if c <= 0.0 {
            return;
        }
        self.reset_campaign();
        let m = self.free / c;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let spent = m * c;
        let mut free = self.free;
        rec_add(&mut self.instances[level], m, dir, &mut free, c);
        self.free = free;
        self.instances[level].node = node;
        self.instances[level].level = level;
        // ANCHOR（552号 HOLD_ANCHOR）：核心建仓即划趋势底仓 = m×(1−MOBILE_FRAC)=m×2/3，机动仓 = m/3。
        // OFF（flag off）时 anchor=0 ⇒ 机动仓=全仓 ⇒ 配额=units/3 与 OFF 逐字一致（bit-exact）。
        self.instances[level].anchor = if self.enable_hold_anchor {
            m * (1.0 - MOBILE_FRAC)
        } else {
            0.0
        };
        self.notional_in = spent;
        self.core_cost_basis = c;
        self.campaign_entry_cost = c;
        self.stage = RecStage::CostReduction;
        self.earning_cash = 0.0;
        self.n_enters += 1;
        // prove_bsp_triggers_operation（panic）+ campaign 起点基线。
        self.guards.note_op("enter");
        self.guards.campaign_start();
    }

    /// **clear_all**（= flat clear_all）：全塔平仓到现金 + reset_campaign。
    fn clear_all(&mut self, c: f64) {
        self.guards.note_op("clear_all"); // prove_bsp_triggers_operation（panic）
        let mut free = self.free;
        for k in 0..MAX_LEVEL {
            let u = self.instances[k].units;
            if u > 1e-12 {
                rec_reduce(&mut self.instances[k], u, &mut free, c);
            }
            // ANCHOR：全塔平仓 ⇒ 无趋势底仓存活，清 anchor 防僵尸残值。OFF 时 anchor 恒 0，置 0 = no-op（bit-exact）。
            self.instances[k].anchor = 0.0;
        }
        self.free = free;
        self.reset_campaign();
    }

    /// **ascend**（= flat ascend）：核心仓 relabel 上移 from→to（to 须 idle），无新资金 NAV 中性。
    fn ascend(&mut self, from: usize, to: usize, node: TrendNode) {
        assert!(
            !self.instances[to].is_active(),
            "ascend 目标 level {to} 非 idle（units={}）：核心上移不可覆盖活跃层",
            self.instances[to].units
        );
        let (lu_pre, su_pre) = self.exposure(); // 移植守卫（A5）：relabel 前敞口快照。
        let mut moved = self.instances[from];
        moved.level = to;
        moved.node = node;
        self.instances[to] = moved;
        self.instances[from] = TInstance::idle(from);
        let (lu_post, su_post) = self.exposure();
        prove_relabel_invariant(lu_pre, su_pre, lu_post, su_post); // ascend 是级别重标定非加仓，敞口必不变。
        self.n_ascends += 1;
        self.guards.note_op("ascend"); // prove_bsp_triggers_operation（panic；BSP 或 emergence 触发）
    }

    /// **emergence_upgrade**（= flat emergence_upgrade）：自下而上涌现，核心同向 → ascend 升级到 target。
    /// ascend 逻辑 = flat 原版（同向 + cc<target_level 才 ascend）。计数为纯观测（编排者排查 2026-06-21）。
    fn emergence_upgrade(&mut self, target_level: usize, target_dir: Polarity, node: TrendNode) {
        self.guards.set_trigger(OpTrigger::Emergence); // 涌现是 ascend 的合法非 BSP 触发源
        if target_level >= MAX_LEVEL {
            return;
        }
        if let Some(cc) = self.highest_active() {
            if cc < target_level {
                self.n_emergence_attempts += 1; // 核心低于涌现级别，本可升级
                if self.instances[cc].direction == target_dir {
                    self.ascend(cc, target_level, node);
                    self.n_emergence_upgrades += 1;
                } else {
                    self.n_emergence_skipped_dir += 1; // 方向不匹配跳过（核心 dir != 涌现 dir）
                }
            }
        }
    }

    /// **sink @ (parent→sub)**（= flat sink）：父减仓 m=u_P/3 + 次级别开 flip(d_P) 短差 m。
    /// **节点 j 在 r\*（最高活跃级别=核心）的级别角色判别（task#64 O6 精修，#61 §3.4 断言U / §五）**：
    /// 返回 true ⟺ j 是**主升浪中的 R− 回调腿**（核心 H⁰ 应保护、机动 H¹ 对冲）。
    ///
    /// **判别量 = 级别角色方向轴 `view.top_trend_dir`（nf/morphology 驱动，永不退化），NOT 力度轴 d_top（退化）。**
    /// task#63 误用 `d_top[r*]`（区间套链贯通=走势完成）作判别量——但**最高涌现级别恒生长不产 type1**
    /// （[[project_l4_consoldn_no_leave_falsified]]）⇒ `d_top[r*]` 在 net-up 恒 false ⇒ 双源坍回单源 ⇒
    /// 对所有次级别反核心向 BSP（含真下跌段）全保护 ⇒ 下跌段核心裸多扛跌爆仓（CL/BRN/OKLO −100%）。
    /// **真判别量 = r* 走势方向**：最高涌现级别恒不产 type1（力度退化），但它的**方向**恒由 nf 驱动已知
    /// （morphology emergent_dir），**方向轴不退化**。
    ///
    /// **O6 按 r* 走势方向（top_trend_dir）二分（编排者操作语义裁定）**：
    /// - **r\* = R+（r* 核心层走势=Up 主升浪）∧ 次级别反核心向（回调）** ⇒ **保护核心**（true）：核心 H⁰
    ///   不动，机动 H¹ 在次级别回调走短差对冲（sink/recover）。主升浪回调不砍核心 = 不踏空。
    /// - **r\* = R−（r* 核心层走势=Down 下跌段）** ⇒ **不保护**（false）：次级别下跌 = 主跌的次级别延续，
    ///   核心 H⁰ 本就该被 sink/减（走常规路径，不裸多扛跌）。**这不是 O6 特例**（CL/BRN/OKLO 爆仓根因 =
    ///   #63 误把下跌段当回调 no-op 掉核心裸多，本修复让下跌段核心正确 sink）。
    /// - **同一观察到的「次级别下跌走势」在 r*=R+ vs r*=R− 下操作相反**——读全纤维（含 r* 角色方向分量）⇒
    ///   踏空（纤维拍扁）溶解。力度轴退化不致命，判别用非退化的级别角色方向轴。
    ///
    /// **判别量来源（task#64 O6，codex 🔴HIGH #78 修复——方法1）**：r* 走势方向取自 **`instances[rstar].node.direction`**
    /// （rstar 核心层**实际所骑走势节点**的方向），NOT `view.top_trend_dir`（走势树最高非空层方向）。
    /// **codex 时序证据链**：route_bsp 序 = 强平 → emergence_upgrade（A'）→ route_bsp（B）（rec_driver.rs:40）。
    /// A' 段 emergence_upgrade 把 highest_active 推到 emergent 层 + `ascend` 更新 `instances[rstar].node` 为
    /// emergent 层走势节点（rec_engine.rs:1269/2625）；但 `view.top_trend_dir` 来自走势树最高非空层（rec_stream
    /// rposition），二者**运行时错位**（rstar 已升高层、top_trend_dir 仍描述产生它的低层走势）⇒ 判别量取自
    /// 错误级别。改读 `instances[rstar].node.direction` = ascend 后核心层的真实走势方向 = 「r* 角色方向」的正确
    /// 来源（逐级方向第一步：每级有自己的方向，非单一 top_trend_dir）。debug_assert-only 守卫（前版 F1）已删——
    /// release 下不生效仍消费错层 top_trend_dir = papers over production bug（no-patch-mentality）。
    ///
    /// **断言U 穷尽性（§3.4）**：构成段对核心主方向要么顺(R+)要么逆(R−)，无第三态。无核心 / j≥r* ⇒ 非主升浪
    /// 回调腿 ⇒ false（保守，走常规 sink，bit-exact 安全）。rstar 必 active（highest_active 返回）⇒ node 已由
    /// enter/sink/ascend 设置 ⇒ node.direction 非退化（无 None 态）。
    fn is_rstar_pullback_leg(&self, j: usize, is_buy: bool) -> bool {
        match self.highest_active() {
            None => false, // 无核心 ⇒ 无 r* 角色
            Some(rstar) => {
                if j >= rstar {
                    return false; // j 非次级别（j 即 r* 或更高）⇒ 非回调腿
                }
                // 拓扑轴：反核心向（买点 vs 核心 Short / 卖点 vs 核心 Long）⇒ R−。
                let cdir = self.instances[rstar].direction;
                let topo_pullback = match cdir {
                    Polarity::Long => !is_buy, // 核心多：次级别卖点
                    Polarity::Short => is_buy, // 核心空：次级别买点
                };
                // 级别角色方向轴（codex 方法1：读 rstar 核心层**实际所骑走势节点**方向，非 view.top_trend_dir）：
                // 仅 rstar 走势方向 = 核心持仓方向（主升浪/主跌浪延续中）才保护核心。rstar 走势反核心向（顶背驰
                // 转向）⇒ 走势已转，核心该常规减仓（不保护）。node.direction 由 emergence_upgrade/ascend 同步到
                // 升级后的核心层走势 ⇒ 与 rstar 严格同层（消除 top_trend_dir 的级别错位）。
                let rstar_trend_dir = self.instances[rstar].node.direction;
                let rstar_continuing = match (rstar_trend_dir, cdir) {
                    (Direction::Up, Polarity::Long) => true, // 核心多 + rstar 走势仍涨 ⇒ 主升浪回调
                    (Direction::Down, Polarity::Short) => true, // 核心空 + rstar 走势仍跌 ⇒ 主跌浪反弹
                    _ => false, // rstar 走势反核心向（转向/下跌段）⇒ 不保护（核心该常规 sink）
                };
                topo_pullback && rstar_continuing
            }
        }
    }

    /// **sink 守卫包装（task#64 O6 机动不动核心，编排者操作语义裁定）**：`protect_core=true`（R+ 主升浪回调腿）
    /// ⇒ **核心 H⁰ units 不减**（不调 `rec_reduce(parent)`）**∧ 机动 H¹ 在 sub 级别开/维持反父向短差空腿**
    /// （对冲次级别回调下行）。这是「机动不动核心」的精确语义两半：核心不被砍（H⁰ 死扣骑主升浪）+ 机动腿
    /// 仍开（H¹ 对冲回调）。`protect_core=false`（R− 下跌段延续腿 / 门控 OFF）⇒ 透传原 `sink`（核心常规减仓，bit-exact）。
    ///
    /// **task#63 误实装（本修复纠正）**：#63 把 protect_core 实装为 `mob_base=0 ⇒ m=0 ⇒ 整个 sink return`
    /// ——**连机动对冲腿也杀了**（编排者诊断：「杀掉机动对冲腿」）。后果：R+ 回调处机动 H¹ 该做空 r*-1 下跌却
    /// 什么都不做 ⇒ 无对冲 + 核心裸多（sink=0 全标的）。本修复解耦「核心减仓量 m_core」与「机动腿开仓量 m_hedge」：
    /// protect_core ⇒ m_core=0（核心不动）∧ m_hedge=quota(u_p)（机动腿照常开空）。
    ///
    /// **会计严格性（NAV 中性独立于核心减仓）**：开空 `rec_add(sub, short_u, Short)` 内部 `free += short_u·c`
    /// （卖空收现金）+ sub 空头市值 −short_u·c ⟹ ΔNAV=0。故机动空腿可独立于核心减仓存在（资金来自卖空收入，
    /// **非**核心释放资本）——这是会计上严格成立的解耦，不是补丁。`prove_tw_neutral` 逐 bar 验收守恒。
    fn sink_guarded(
        &mut self,
        parent: usize,
        sub: usize,
        sub_node: TrendNode,
        c: f64,
        protect_core: bool,
    ) {
        self.sink_impl(parent, sub, sub_node, c, protect_core);
    }

    fn sink(&mut self, parent: usize, sub: usize, sub_node: TrendNode, c: f64) {
        self.sink_impl(parent, sub, sub_node, c, false);
    }

    fn sink_impl(
        &mut self,
        parent: usize,
        sub: usize,
        sub_node: TrendNode,
        c: f64,
        protect_core: bool,
    ) {
        let u_p = self.instances[parent].units;
        let pdir = self.instances[parent].direction;
        // ANCHOR（552号）：配额作用于机动仓 u_P−anchor（趋势底仓死扣不下放），OFF 时 anchor=0 ⇒ mob_base=u_P（bit-exact）。
        let mob_base = if self.enable_hold_anchor {
            (u_p - self.instances[parent].anchor).max(0.0)
        } else {
            u_p
        };
        // **机动配额 m（task#64 O6 终版：R+ 被动保护）**：
        // - protect_core=false（R− 下跌段延续 / OFF）：核心减 m=quota(mob_base) + sub 开 m 短差（原行为，bit-exact）。
        // - protect_core=true（R+ 主升浪回调）：核心不减（realized=0）∧ **不开 hedge 空腿**（见下 rec_add 门控，
        //   #567 真539：主动 hedge 强牛失血 ⇒ 被动保护）⇒ sink 退化为 no-op（仅核心保护,不动核心不开空）。
        // **逐级内在配额（task#84 子5）**：ON ⇒ m=mob_base×mobile_frac(L_pullback[sub],L_confirm[sub])（替固定 1/3，
        // 偏离 542 σ-不变=597 子5 授权概念分离实装）；OFF ⇒ quota(mob_base)=σ-不变 1/3（bit-exact）。
        let m = if self.enable_intrinsic_quota {
            mob_base * mobile_frac(self.cur_l_pullback[sub], self.cur_l_confirm[sub])
        } else {
            quota(mob_base)
        };
        if !(m > 1e-12 && m.is_finite()) || m > u_p + 1e-9 {
            return;
        }
        let mob = flip_pol(pdir);
        if self.instances[sub].is_active() && self.instances[sub].direction != mob {
            return;
        }
        // 同资本 sizing（编排者裁决 2026-06-21）：开空 units = 机动配额对应资本金额（m×parent.basis）
        // 在当前价 c 开空 = m×pb/c。价越高→同资本开的空头越少→牛市空头累积减轻、全仓 NAV 不易被拖到 0。
        // pb 必须在 reduce 前捕获（reduce 把 units 减到 ≤EPS 时会污染 basis=NaN）；表达式分组 (m*pb)/c
        // 与 flat t_engine 逐字一致保 bit-exact。
        let pb = self.instances[parent].basis;
        let short_u = if pb.is_finite() && pb > 1e-12 {
            m * pb / c
        } else {
            m
        };
        if !(short_u > 1e-12 && short_u.is_finite()) {
            return;
        }
        // 移植守卫（L0，从 spiral/fugue_v3）：区间套向心下沉 sub<parent + σ-不变配额 m=mob_base×MOBILE_FRAC。
        prove_sink_descends(parent, sub, self.cur_bar);
        // σ-不变配额守卫仅在**非逐级内在配额**路径生效——ON 路径有意偏离 542 σ-不变（597 子5 授权概念分离，
        // f_k≠f_{k+1} = 逐级纤维塔的定义本性，非被绕过的守卫）⇒ ON 跳过。向心下沉 + TW 中性 ON 仍守。
        if !self.enable_intrinsic_quota {
            prove_sigma_quota(m, mob_base, sub, self.cur_bar); // anchor OFF ⇒ mob_base=u_p（bit-exact）
        }
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        // **核心减仓（task#64 O6 终版）**：protect_core=true（R+ 主升浪回调）⇒ 核心 H⁰ 不减（realized=0）；
        // protect_core=false（R− 下跌段延续 / OFF）⇒ 核心减 m（原行为，bit-exact）。
        let realized = if protect_core {
            0.0
        } else {
            rec_reduce(&mut self.instances[parent], m, &mut free, c)
        };
        // **机动腿开空（task#64 O6 终版：R+ 不开 hedge 空腿 = 被动保护）**：
        // protect_core=false（R−/OFF）⇒ 开/维持反父向短差空腿（原行为，bit-exact）。
        // protect_core=true（R+ 主升浪回调）⇒ **不开 hedge 空腿**——主动 hedge 在强牛系统性失血 = #567
        // 已结算「强趋势 hedge 短腿=真539 不可约失血」。L3 坐实：开 hedge 则 BTC −132468%/CL −677%（账户打穿,
        // short_pnl −60M）；不开则 BTC +381.5%/CL −68.2%（爆仓全消, liq=0 无穿仓, 逐字复现 o6-level-role 报告）。
        // O6 R+ 保护正解 = **被动保护**（核心不动 + 不开新空腿），非编排者裁定字面的「机动主动做空 r*-1」——
        // 机制层 L3 否定性结果（formalization-validity-domain），目标（踏空溶解+不爆仓）由被动保护达成。
        // 有效域：⊂ net-up regime（真 bear 下 active hedge 可能赚=#567 诚实声明，8 net-up 标的无法证，231号）。
        if !protect_core {
            rec_add(&mut self.instances[sub], short_u, mob, &mut free, c);
        }
        self.free = free;
        self.instances[sub].node = sub_node;
        self.instances[sub].level = sub;
        // account_reduce 处理核心减仓 pnl（降成本/退本金）；protect_core ⇒ realized=0 ⇒ 核心账本不动（仅空腿 pnl 在 recover 结算）。
        self.account_reduce(pdir, realized, c);
        self.n_sinks += 1;
        self.guards.note_op("sink"); // prove_bsp_triggers_operation（panic）
        self.guards.on_sink(); // prove_sink_recover_balance（campaign 内累计；机动腿在两分支均建立 ⇒ 计入配对）
        if sub < MAX_LEVEL {
            self.sink_by_level[sub] += 1; // 诊断：per-level sink
        }
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **recover @ (sub→parent)**（= flat recover）：次级别走势完成 ⇒ 整条短差平清 m=u_sub 升回父 d_P。
    /// EarningShares 阶段父多头 → deploy_earning（增股数）。
    fn recover(&mut self, parent: usize, sub: usize, c: f64) {
        let pdir = self.instances[parent].direction;
        let mob = flip_pol(pdir);
        if !self.instances[sub].is_active() || self.instances[sub].direction != mob {
            return;
        }
        let m = self.instances[sub].units;
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        // 同资本反算（编排者裁决 2026-06-21）：平空释放名义资本 = m×sub.basis（= sink 时下放的核心资本）；
        // 归还核心 units = 该资本 / parent.basis ⟹ give = m×sb/pb。parent.basis 不变时 give=m（核心恢复原
        // 股数），被 deploy_earning 降本后按资本恢复（give>m）。sb 须在 reduce(sub) 前捕获（reduce 零化
        // sub→basis=NaN）；表达式分组 (m*sb)/pb 与 flat t_engine 逐字一致保 bit-exact。
        let sb = self.instances[sub].basis;
        let pb = self.instances[parent].basis;
        let give = if pb.is_finite() && pb > 1e-12 {
            m * sb / pb
        } else {
            m
        };
        if !(give > 1e-12 && give.is_finite()) {
            return;
        }
        // 移植守卫（L0）：recover 升回与 sink 向心配对，同守 sub<parent（次级别走势完成升回父级）。
        prove_sink_descends(parent, sub, self.cur_bar);
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[sub], m, &mut free, c);
        rec_add(&mut self.instances[parent], give, pdir, &mut free, c);
        self.free = free;
        self.account_reduce(mob, realized, c);
        if sub < MAX_LEVEL {
            self.short_pnl_by_level[sub] += realized; // 短差腿 per-level（验收哪级别亏）
        }
        self.n_recovers += 1;
        self.guards.note_op("recover"); // prove_bsp_triggers_operation（panic）
        self.guards.on_recover(sub, realized); // prove_sink_recover_balance + prove_per_level_pnl
        if sub < MAX_LEVEL {
            self.recover_by_level[sub] += 1; // 诊断：per-level recover
        }
        if pdir == Polarity::Long {
            self.deploy_earning(parent, c);
        }
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **drain @ j**（= flat drain）：子级持同父向遗留仓 → 反父向 BSP 减暴露 1/3（不翻转）。
    fn drain(&mut self, j: usize, c: f64) {
        let u = self.instances[j].units;
        // ANCHOR（552号）：drain 目标 j 有活跃祖先（route_bsp Some(p) 分支），非核心 ⇒ enter 未给 j 划 anchor
        // ⇒ anchor 结构性恒 0 ⇒ mob_base=u（flag ON/OFF 皆与 OFF 逐字一致）。用机动仓基准是 σ-不变配额一致形式。
        let mob_base = if self.enable_hold_anchor {
            (u - self.instances[j].anchor).max(0.0)
        } else {
            u
        };
        // 逐级内在配额（task#84 子5）：ON ⇒ m=mob_base×mobile_frac(L_pullback[j],L_confirm[j])；OFF ⇒ σ-不变 1/3。
        let m = if self.enable_intrinsic_quota {
            mob_base * mobile_frac(self.cur_l_pullback[j], self.cur_l_confirm[j])
        } else {
            quota(mob_base)
        };
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let jdir = self.instances[j].direction;
        // 移植守卫（L0）：drain 减暴露配额 σ-不变（m=mob_base×MOBILE_FRAC，级别无关；mob_base=u 当 anchor=0）。
        // ON 逐级内在配额偏离 σ-不变（597 子5 授权）⇒ 跳过本守卫（TW 中性 ON 仍守）。
        if !self.enable_intrinsic_quota {
            prove_sigma_quota(m, mob_base, j, self.cur_bar);
        }
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[j], m, &mut free, c);
        self.free = free;
        self.account_reduce(jdir, realized, c);
        self.n_drains += 1;
        self.guards.note_op("drain"); // prove_bsp_triggers_operation（panic）
        if jdir == Polarity::Short {
            self.guards.on_short_pnl(j, realized); // prove_per_level_pnl（同向遗留短差腿平仓）
        }
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **次级别走势完成判别接口（task#40 B ↔ C 任务区间套 H¹ 定位）**：区分 O7 recover（走势完成，
    /// 整条升回）vs O3 add（回调未完成，部分买回）。**B 不自造判别逻辑**（禁越界 #38 协调节点裁决）——
    /// 委托 C 的 `is_sub_trend_done(sub)`（区间套逐级收缩到 a0 = 走势完成）。**接口占位 fallback=true**
    /// ⇒ 全走 recover ⇒ T_ORBIT9_DISPATCH ON 时与 OFF 行为同（add 未激活，保 OFF/未整合 bit-exact）。
    /// C 整合时替换 body 为 `self.view.is_sub_trend_done(sub)`（或等价区间套链信号）。
    fn orbit9_sub_trend_done(&self, _sub: usize) -> bool {
        true // 占位：C 接口未就位 ⇒ 默认走势完成 ⇒ recover（O7）⇒ add（O3）不激活
    }

    /// **add @ (parent→sub)**（O3，task#40 B = 买回/卖回 = 不动 h 同级别短差腿**部分**重建）：
    /// 父级 k 同向 BSP（type1后回调，未走势完成）⇒ **部分**买回 m=quota(u_sub) 的短差升回父向 pdir，
    /// sub 级减一段短差（**非** recover 平整条 u_sub）。**不动 h**（同级别 k 内重建被 sink 减掉的腿的一段，
    /// #39 §3.1b/§3.4：莫比乌斯仅作用 h，对 O3 平凡 τ 对称 ⇒ 多头买回 ↔ 空头卖回镜像，pdir 参数化无 if 多空）。
    /// **极性不变不污染 long**（rec_add 层内单一方向 assert 守卫；add 向父级加 pdir、sub 减 mob=flip(pdir)）。
    /// NAV 中性（同价 c 平 sub + 加 parent，资本守恒 m×sb/pb）。前置 = sub 持反父向短差且 j 有活跃祖先。
    fn add(&mut self, parent: usize, sub: usize, c: f64) {
        let pdir = self.instances[parent].direction;
        let mob = flip_pol(pdir);
        if !self.instances[sub].is_active() || self.instances[sub].direction != mob {
            return; // 无短差腿可买回 ⇒ no-op（O9，不凭空 pyramid）
        }
        // O3 = 部分买回：配额 m=quota(u_sub)（机动仓 σ-不变 1/3），区别 recover 平整条 u_sub。
        // 逐级内在配额（task#84 子5）：ON ⇒ m=u_sub×mobile_frac(L_pullback[sub],L_confirm[sub])（部分买回量逐级
        // 自适应——深确认大段买回 / 浅确认小段，对称 sink 的逐级配额）；OFF ⇒ σ-不变 1/3（bit-exact）。
        let u_sub = self.instances[sub].units;
        let m = if self.enable_intrinsic_quota {
            u_sub * mobile_frac(self.cur_l_pullback[sub], self.cur_l_confirm[sub])
        } else {
            quota(u_sub)
        };
        if !(m > 1e-12 && m.is_finite()) || m > u_sub + 1e-9 {
            return;
        }
        // 同资本反算（= recover 逐字同形）：平 sub 短差 m 释放名义资本 m×sb，归还父 units = m×sb/pb。
        // sb 须在 reduce(sub) 前捕获（reduce 零化 → basis=NaN）；表达式分组 (m*sb)/pb 保会计一致。
        let sb = self.instances[sub].basis;
        let pb = self.instances[parent].basis;
        let give = if pb.is_finite() && pb > 1e-12 {
            m * sb / pb
        } else {
            m
        };
        if !(give > 1e-12 && give.is_finite()) {
            return;
        }
        // 移植守卫（L0）：add 升回与 sink 向心配对，同守 sub<parent（不动 h 同级别短差腿重建）。
        prove_sink_descends(parent, sub, self.cur_bar);
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[sub], m, &mut free, c);
        rec_add(&mut self.instances[parent], give, pdir, &mut free, c);
        self.free = free;
        self.account_reduce(mob, realized, c);
        if sub < MAX_LEVEL {
            self.add_pnl_by_level[sub] += realized; // O3 买回腿 per-level（τ 镜像多空对称）
        }
        self.n_adds += 1;
        self.guards.note_op("add"); // prove_bsp_triggers_operation（panic）
                                    // 注：add 是部分平短差升回 ⇒ 与 recover 同向降短差敞口，但**不**全清 ⇒ 不调 on_recover（整条配对守卫）。
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **route_bsp**（= flat route_bsp）：level j 的 BSP 分派。task#63：+`view` 传力度轴（双源乘积，#61 §3.0）。
    fn route_bsp(&mut self, j: usize, is_buy: bool, node: TrendNode, c: f64) {
        self.guards.set_trigger(OpTrigger::Bsp); // 本路由触发的所有原子操作归因 BSP
        match self.nearest_active_parent(j) {
            // ── 子级（有活跃祖先 P）：区间套约束，永不独立翻转 ──
            Some(p) => {
                let pdir = self.instances[p].direction;
                let mob = flip_pol(pdir);
                let is_reduce = match pdir {
                    Polarity::Long => !is_buy, // 父多：卖点=减仓信号
                    Polarity::Short => is_buy, // 父空：买点=减仓信号
                };
                if is_reduce {
                    // 反父向 BSP：sink（空/已是短差）或 drain（遗留同父向仓）。
                    if is_buy {
                        self.buy_sink += 1; // 诊断：父空+买点 → sink（减核心 Short）
                    }
                    if !self.instances[j].is_active() || self.instances[j].direction == mob {
                        // ── 节点向量路由（task#63，#61 §六 N1@k,R−/N3@k,R− → O6 sink 机动不动核心）──
                        // 读节点 j 在 r*（最高活跃级别=核心）的级别角色：j<r* ∧ 反核心向 ⇒ R−（回调腿）。
                        // R− 回调腿的 sink 在主升浪回调处砍核心 = 踏空根因①④。门控 ON ⇒ 强制核心保护
                        // （mob 配额作用于零 = 核心 units 不减，只在子级别开/维持反向短差腿）。
                        let protect_core =
                            self.enable_orbit9_nodevec && self.is_rstar_pullback_leg(j, is_buy);
                        self.sink_guarded(p, j, node, c, protect_core);
                    } else {
                        self.drain(j, c);
                    }
                } else {
                    // 同父向 BSP：j 持短差则升回；否则 no-op（不 pyramid）。
                    if self.instances[j].is_active() && self.instances[j].direction == mob {
                        if is_buy {
                            self.buy_recover += 1; // 诊断：父多+买点 → recover/add
                        }
                        // ── 9 轨道分派（task#40 B，T_ORBIT9_DISPATCH）──
                        // OFF：全走 O7 recover（平整条 = 现状 bit-exact）。
                        // ON：区分 O7 recover（次级别走势完成 → 整条升回）vs O3 add（回调未完成 → 部分买回）。
                        //     "走势完成 vs 回调"判别 = C 任务（区间套 H¹ 定位）接口 orbit9_sub_trend_done；
                        //     接口未就位时 fallback=true ⇒ 全走 recover ⇒ 与 OFF 同（add 不激活）。
                        //     account 过滤是 route 之后独立 gate（此处零 if regime/account，τ 对称 pdir 参数化）。
                        if self.enable_orbit9_dispatch && !self.orbit9_sub_trend_done(j) {
                            self.add(p, j, c); // O3：部分买回/卖回（不动 h 同级别短差腿部分重建）
                        } else {
                            self.recover(p, j, c); // O7：整条升回（走势完成）
                        }
                    } else if is_buy {
                        self.buy_noop += 1; // 诊断：父多+买点+j 无短差 → no-op（浪费的买点 = O9）
                    }
                }
            }
            // ── 核心级（无活跃祖先）：唯一独立翻转点 ──
            None => {
                if is_buy {
                    self.buy_core += 1; // 诊断：核心级买点 → enter/ascend/flip（不 recover）
                }
                let dir = if is_buy {
                    Polarity::Long
                } else {
                    Polarity::Short
                };
                match self.highest_active() {
                    None => self.enter(j, dir, node, c), // 首次建仓
                    Some(cc) => {
                        let cdir = self.instances[cc].direction;
                        if dir == cdir {
                            // 同向：更高 level ⇒ ascend 骑乘；同/低 level ⇒ no-op。
                            if j > cc && !self.instances[j].is_active() {
                                self.ascend(cc, j, node);
                            }
                        } else {
                            // 反向：核心翻转（走势终完美→新走势）——清全塔 + 同 level 反向 enter。
                            // 诊断（编排者排查 2026-06-21）：核心 flip 序列(bar/from→to/触发 BSP 级别+买卖)。
                            self.flip_log.push((self.cur_bar, cdir, dir, j, is_buy));
                            self.clear_all(c);
                            self.enter(j, dir, node, c);
                            self.n_flips += 1;
                        }
                    }
                }
            }
        }
    }

    // ──────────────── 读法B/读法乙递归：每级别独立腿骑走势消费 d_top（任务18 编排者修正）────────────────

    /// 几何塔配额（编排者裁决）：base=free×2/3，级别 k = base/λ^(top−k)，收敛 base×3/2≤free 恒仓不加杠杆。
    fn geom_tower_quota(&self, k: usize, top: usize, free_pool: f64, c: f64) -> f64 {
        if c <= 0.0 || free_pool <= 0.0 || k > top {
            return 0.0;
        }
        let base = free_pool * (2.0 / 3.0);
        let depth = top - k;
        let notional = base * MOBILE_FRAC.powi(depth as i32);
        notional / c
    }

    /// **均匀基准单元定仓（Face B，shortleg-profit-spec §6.3，删 geom_tower 恒仓归一化）**：
    /// 每级别基准单元 = `initial_capital × MOBILE_FRAC`（级别无关，**无 depth 衰减、无 if level、无 free
    /// 归一化**）。geom_tower 的 Σnotional=free 恒仓归一化把 max_gross 钳到 ≤1×（压制副作用，msb §14.1
    /// = 中间级别敞口塌缩不吃自身|涨跌幅|）；均匀定仓让多级别独立腿叠加 → max_gross 自然 >1×（杠杆来源A
    /// 涌现，579），由每腿否定线 [ZD,ZG] 分散封顶（liq=0）。
    /// **no-hardcode**：唯一常数 = MOBILE_FRAC（= 1/λ 中枢三段 σ-不变，非新倍数参数）；杠杆从级别叠加
    /// 涌现非倍数（编排者「删配额非加参数，杠杆涌现非倍数」）。固定绝对基准（非 free×frac）⇒ 不复现
    /// geom_tower 的 Σ≤free 压制（free×frac 几何收敛回 ≤1×）；账户增长时 gross/nav 自动去杠杆（安全）。
    fn uniform_base_units(&self, c: f64) -> f64 {
        if !(c > 0.0) {
            return 0.0;
        }
        self.initial_capital * MOBILE_FRAC / c
    }

    /// **腿定仓分派**（Face B 均匀基准单元 / 否则 geom_tower 恒仓配额）。单点切换保 bit-exact：
    /// `enable_uniform_sizing=false` ⇒ 逐字 geom_tower_quota（RB_PAIR/OFF 不变）。
    fn leg_open_units(&self, k: usize, top: usize, c: f64) -> f64 {
        if self.enable_uniform_sizing {
            self.uniform_base_units(c)
        } else {
            self.geom_tower_quota(k, top, self.free.max(0.0), c)
        }
    }

    /// **否定线消费门（Face B，bit-exact 防御）**：进场中枢边界 [ZD,ZG] 仅在 Face B
    /// （`enable_uniform_sizing`）作为否定线锁入腿；否则 None（RB_PAIR/OFF 下 long_stop/short_stop=NaN
    /// ⇒ pair_stop_loss_step 死代码不动 = 逐字不变）。在引擎层而非仅信号层把关，使 bit-exact 不依赖
    /// rec_stream 是否填充 view.zd/zg（防御 #69 否定线=死代码的隐性依赖）。
    fn faceb_stop(&self, raw: Option<f64>) -> Option<f64> {
        if self.enable_uniform_sizing {
            raw
        } else {
            None
        }
    }

    fn snapshot_fingerprints(&self) -> [(u64, bool, bool); MAX_LEVEL] {
        let mut fp = [(0u64, false, false); MAX_LEVEL];
        for k in 0..MAX_LEVEL {
            fp[k] = self.legs[k].fingerprint();
        }
        fp
    }

    /// **open_leg**（ride 分支）：级别 k 空腿按走势方向从单一 free 池领几何塔配额建仓。只写 legs[k]（547 隔离）。
    fn open_leg(&mut self, k: usize, dir: Polarity, node: TrendNode, top: usize, c: f64) {
        let fp_pre = self.snapshot_fingerprints();
        let m = self.geom_tower_quota(k, top, self.free.max(0.0), c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        match dir {
            Polarity::Long => self.free -= m * c,
            Polarity::Short => self.free += m * c,
        }
        self.legs[k] = Leg {
            level: k,
            direction: dir,
            units: m,
            basis: c,
            riding_node: node,
            active: true,
        };
        self.leg_opens_by_level[k] += 1;
        self.guards.note_op("open_leg");
        prove_leg_isolation(&fp_pre, &self.snapshot_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **close_leg**（switch 前半）：平 legs[k] 全量到现金，realized 记 per_level pnl。只写 legs[k]（547 隔离）。
    fn close_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.legs[k].active {
            return 0.0;
        }
        let fp_pre = self.snapshot_fingerprints();
        let leg = self.legs[k];
        let pnl = match leg.direction {
            Polarity::Long => leg.units * (c - leg.basis),
            Polarity::Short => leg.units * (leg.basis - c),
        };
        let tw_pre = self.total_wealth(c);
        match leg.direction {
            Polarity::Long => {
                self.free += leg.units * c;
                self.per_level_long_pnl[k] += pnl;
            }
            Polarity::Short => {
                self.free -= leg.units * c;
                self.per_level_short_pnl[k] += pnl;
            }
        }
        self.legs[k] = Leg::idle(k);
        self.guards.note_op("close_leg");
        prove_leg_isolation(&fp_pre, &self.snapshot_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **g(k)**（单算子三分支）：级别 k 腿消费 `levels[k]` 走势 + `d_top[k]`（触发器=走势完成链 OR 背驰段链）。
    /// switch：d_top[k] ∧ 腿活跃 → close+反向 open；ride：腿空 ∧ 有走势 → 按方向 open；hold：维持。
    /// **clear 只清触发级别**（547 隔离：绝不复用 clear_all，每级别独立 campaign）。
    fn g(&mut self, k: usize, view: &LevelView, top: usize, c: f64) {
        let d_top = view.d_top[k];
        if d_top && self.legs[k].active {
            let old_dir = self.legs[k].direction;
            self.guards.set_trigger(OpTrigger::Bsp);
            self.close_leg(k, c);
            let new_dir = flip_pol(old_dir);
            let node = view.nodes[k].unwrap_or_else(|| {
                TrendNode::new(
                    self.cur_bar,
                    self.cur_bar,
                    c,
                    c,
                    match new_dir {
                        Polarity::Long => Direction::Up,
                        Polarity::Short => Direction::Down,
                    },
                )
            });
            self.open_leg(k, new_dir, node, top, c);
            self.leg_switches_by_level[k] += 1;
            return;
        }
        if !self.legs[k].active {
            if let Some(node) = view.nodes[k] {
                let dir = dir_to_polarity(node.direction);
                self.guards.set_trigger(OpTrigger::Bsp);
                self.open_leg(k, dir, node, top, c);
            }
            return;
        }
    }

    /// **consume_legs**（单算子递归 = construct iterate 的对偶）：a0→涌现上界逐级 g(k)。
    fn consume_legs(&mut self, view: &LevelView, c: f64) {
        let top = match (0..MAX_LEVEL).rev().find(|&k| view.nodes[k].is_some()) {
            Some(t) => t,
            None => return,
        };
        for k in 0..=top {
            self.g(k, view, top, c);
        }
    }

    /// 读法B 收尾（全平所有腿到现金）。
    fn finish_legs(&mut self, c: f64) {
        if !(c.is_finite() && c > 0.0) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Eod);
        for k in 0..MAX_LEVEL {
            if self.legs[k].active {
                self.close_leg(k, c);
            }
        }
    }

    // ──────────────── 读法B 一对多空腿（LegPair，任务57=53.1 编排者重写）────────────────

    /// LegPair 指纹快照（547 隔离守卫：g_pair(k) 只许写 leg_pairs[k]）。
    fn snapshot_pair_fingerprints(&self) -> [(u64, u64); MAX_LEVEL] {
        let mut fp = [(0u64, 0u64); MAX_LEVEL];
        for k in 0..MAX_LEVEL {
            fp[k] = self.leg_pairs[k].pair_fingerprint();
        }
        fp
    }

    /// **prove_pair_isolation（547 隔离，L0 结构 panic 守卫）**：g_pair(k) 只写 leg_pairs[k]，断言
    /// leg_pairs[j≠k] 在前后逐位不变（547 病灶「低级别信号越级翻动高级别主力」的结构否定，对偶单腿守卫）。
    fn prove_pair_isolation(
        fp_pre: &[(u64, u64); MAX_LEVEL],
        fp_post: &[(u64, u64); MAX_LEVEL],
        k: usize,
    ) {
        for j in 0..MAX_LEVEL {
            if j == k {
                continue;
            }
            assert_eq!(
                fp_pre[j], fp_post[j],
                "547 隔离违反：g_pair({k}) 改动了别级 leg_pairs[{j}]（每级别独立对腿只许 g_pair(k) 写 leg_pairs[k]）"
            );
        }
    }

    /// **open_long_leg**（开多腿）：级别 k 买点 fire ⇒ 从单一 free 池领几何塔配额建多仓，否定线 = 进场 ZD。
    /// 只写 leg_pairs[k]（547 隔离）。`stop_zd`=进场中枢核心下沿（跌破止损）；None ⇒ NaN（无否定线，仅靠反向卖点平）。
    fn open_long_leg(&mut self, k: usize, top: usize, stop_zd: Option<f64>, c: f64) {
        if self.leg_pairs[k].long_active() {
            return; // 已持多腿 ⇒ 不重复开（hold，单一方向单腿）
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let m = self.leg_open_units(k, top, c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        self.free -= m * c; // 多腿建仓 = 现金换多头（NAV 中性）
        self.leg_pairs[k].long_units = m;
        self.leg_pairs[k].long_basis = c;
        self.leg_pairs[k].long_stop = stop_zd.unwrap_or(f64::NAN);
        self.leg_pairs[k].long_entry_bar = self.cur_bar; // #149 capture-ratio 逐笔账本（observation-only）
        self.pair_long_opens[k] += 1;
        self.guards.note_op("open_long_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **open_short_leg**（开空腿）：级别 k 卖点 fire ⇒ 建空仓，否定线 = 进场 ZG（涨破止损）。只写 leg_pairs[k]。
    fn open_short_leg(&mut self, k: usize, top: usize, stop_zg: Option<f64>, c: f64) {
        if self.leg_pairs[k].short_active() {
            return; // 已持空腿 ⇒ 不重复开
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let m = self.leg_open_units(k, top, c);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        let tw_pre = self.total_wealth(c);
        self.free += m * c; // 空腿建仓 = 收空头保证金现金（NAV 中性，−u·c 抵 +m·c）
        self.leg_pairs[k].short_units = m;
        self.leg_pairs[k].short_basis = c;
        self.leg_pairs[k].short_stop = stop_zg.unwrap_or(f64::NAN);
        self.leg_pairs[k].short_entry_bar = self.cur_bar; // #149 capture-ratio 逐笔账本（observation-only）
        self.pair_short_opens[k] += 1;
        self.guards.note_op("open_short_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **close_long_leg**（平多腿）：反向卖点 fire 或否定线止损 ⇒ 全量平多到现金，realized 记 pair_long_pnl。
    fn close_long_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.leg_pairs[k].long_active() {
            return 0.0;
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let u = self.leg_pairs[k].long_units;
        let basis = self.leg_pairs[k].long_basis;
        let pnl = u * (c - basis);
        let tw_pre = self.total_wealth(c);
        self.free += u * c;
        self.pair_long_pnl[k] += pnl;
        // L_confirm 第四轴会计（task#95，observation-only）：归 (k, leg=long_is_core?H⁰:H¹, dir=R+, c=long_conf)。
        if self.enable_lconfirm_audit {
            let leg = if self.leg_pairs[k].long_is_core { 0 } else { 1 };
            let conf = self.leg_pairs[k].long_conf as usize;
            self.pi_k_leg_dir_c[k][leg][0][conf] += pnl;
            self.pi_count_k_leg_dir_c[k][leg][0][conf] += 1;
            self.pi_total_audited += pnl;
        }
        // #149 capture-ratio 逐笔账本（observation-only，不改决策）：(level,entry_bar,exit_bar,entry_px,exit_px,units,is_short,pnl)
        self.leg_trades.push((
            k,
            self.leg_pairs[k].long_entry_bar,
            self.cur_bar,
            basis,
            c,
            u,
            false,
            pnl,
        ));
        self.leg_close_reasons.push(self.last_close_reason); // #170 TC 诊断
        self.leg_entry_stops.push(self.leg_pairs[k].long_stop); // #170 TC 诊断：开仓否定线 ZD
        self.exit_trigger_log.push(self.pending_exit_trigger); // #164 R2 出场触发诊断（同序）
        self.pending_exit_trigger = 6; // reset
        self.leg_pairs[k].long_units = 0.0;
        self.leg_pairs[k].long_basis = f64::NAN;
        self.leg_pairs[k].long_stop = f64::NAN;
        self.leg_pairs[k].long_entry_bar = -1;
        self.pair_long_closes[k] += 1;
        self.guards.note_op("close_long_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **close_short_leg**（平空腿）：反向买点 fire 或否定线止损 ⇒ 全量平空到现金，realized 记 pair_short_pnl。
    fn close_short_leg(&mut self, k: usize, c: f64) -> f64 {
        if !self.leg_pairs[k].short_active() {
            return 0.0;
        }
        let fp_pre = self.snapshot_pair_fingerprints();
        let u = self.leg_pairs[k].short_units;
        let basis = self.leg_pairs[k].short_basis;
        let pnl = u * (basis - c);
        let tw_pre = self.total_wealth(c);
        self.free -= u * c;
        self.pair_short_pnl[k] += pnl;
        // L_confirm 第四轴会计（task#95，observation-only）：归 (k, leg=short_is_core?H⁰:H¹, dir=R−, c=short_conf)。
        if self.enable_lconfirm_audit {
            let leg = if self.leg_pairs[k].short_is_core {
                0
            } else {
                1
            };
            let conf = self.leg_pairs[k].short_conf as usize;
            self.pi_k_leg_dir_c[k][leg][1][conf] += pnl;
            self.pi_count_k_leg_dir_c[k][leg][1][conf] += 1;
            self.pi_total_audited += pnl;
        }
        // #149 capture-ratio 逐笔账本（observation-only，不改决策）：(level,entry_bar,exit_bar,entry_px,exit_px,units,is_short,pnl)
        self.leg_trades.push((
            k,
            self.leg_pairs[k].short_entry_bar,
            self.cur_bar,
            basis,
            c,
            u,
            true,
            pnl,
        ));
        self.leg_close_reasons.push(self.last_close_reason); // #170 TC 诊断
        self.leg_entry_stops.push(self.leg_pairs[k].short_stop); // #170 TC 诊断：开仓否定线 ZG
        self.exit_trigger_log.push(self.pending_exit_trigger); // #164 R2 出场触发诊断（同序）
        self.pending_exit_trigger = 6; // reset
        self.leg_pairs[k].short_units = 0.0;
        self.leg_pairs[k].short_basis = f64::NAN;
        self.leg_pairs[k].short_stop = f64::NAN;
        self.leg_pairs[k].short_entry_bar = -1;
        self.pair_short_closes[k] += 1;
        self.guards.note_op("close_short_leg");
        Self::prove_pair_isolation(&fp_pre, &self.snapshot_pair_fingerprints(), k);
        self.prove_tw_neutral(tw_pre, c);
        pnl
    }

    /// **当前最高活跃多腿级别**（结构涌现「核心」= 当前最高活跃 up-trend 多腿，无 `if level==top` 硬编码）。
    /// LegPair 路径核心 = 最高有多腿在场的级别（自相似结构涌现，对照 instances 路径 highest_active）。
    fn highest_active_long(&self) -> Option<usize> {
        (0..MAX_LEVEL)
            .rev()
            .find(|&k| self.leg_pairs[k].long_active())
    }

    /// **LegPair 总敞口 (Σlong_units, Σshort_units)**（relabel 守卫用）。对偶 instances `exposure`。
    fn pair_exposure(&self) -> (f64, f64) {
        let mut lu = 0.0;
        let mut su = 0.0;
        for p in &self.leg_pairs {
            if p.long_units > 0.0 {
                lu += p.long_units;
            }
            if p.short_units > 0.0 {
                su += p.short_units;
            }
        }
        (lu, su)
    }

    /// **核心多腿涌现升级（R3 段无腿修复 #164/#6，对照 instances `emergence_upgrade`/`ascend`）**。
    ///
    /// **病灶**（L2 实证 BTC Face A）：LegPair 路径原缺涌现升级 ⇒ 核心多腿（`highest_active_long`）卡在
    /// 次级别买点 fire 的低级别（L0/L1/L2），而 emergent_top 涌现到 L4/L5（占行情绝大部分 bar）。L4/L5
    /// 自身 type1 买点几乎不 fire（高级别走势单调上涨无第二走势起点），核心腿永不上移 ⇒ **L4/L5 涌现段无
    /// 专属核心腿**（`pair_core_level_bars[5]=0` vs `emergent_level_bars[5]=1877899`）= S1 段无腿 70.8%/
    /// 高级别~100% 根因。最大 |Δ| 被碎成次级别 churn 穿越（T1 方向错位主导亏损），非专属核心腿捕获。
    ///
    /// **修复**（多重赋格理想）：核心多腿同向（Long）跟随 emergent_top relabel 上移到涌现级别——**持仓继承**
    /// （long_units/long_basis 不变 = 无新资金 = 敞口不变），否定线更新为涌现级别中枢 ZD（核心现骑 target 走势）。
    /// 每涌现级别获专属核心腿捕获本级别 |Δ|。与 #110 Face B「核心能翻转」正交互补（本修复=「核心能上移」）。
    ///
    /// **判据**（对照 ascend）：emergent_top=(target, Long) ∧ cc=highest_active_long<target ∧ target 多腿 idle。
    /// emergent_top=Short ⇒ 不上移多腿（下跌涌现走 d_top 核心翻空，pair_core_short，正交）。target 已有核心
    /// 多腿 ⇒ 跳过（不覆盖活跃层，对照 ascend assert）。
    fn pair_emergence_upgrade(&mut self, view: &LevelView, c: f64) {
        let (target, target_pol) = match view.emergent_top {
            Some(x) => x,
            None => return,
        };
        if target >= MAX_LEVEL {
            return;
        }
        // 只多腿上移（下跌涌现走 d_top 核心翻空，正交；对照 instances emergence_upgrade 方向门控）。
        if target_pol != Polarity::Long {
            return;
        }
        let cc = match self.highest_active_long() {
            Some(c) => c,
            None => return, // 无核心多腿 ⇒ 无可上移（核心由 g_pair 买点建立）
        };
        if cc >= target {
            return; // 核心已在涌现级别或更高
        }
        // 目标级已有核心多腿 ⇒ 跳过（不覆盖活跃层，对照 ascend `!instances[to].is_active()` assert）。
        if self.leg_pairs[target].long_active() {
            return;
        }
        // ── relabel：cc 多腿整体移到 target（敞口不变，free 不动 = 无现金流；空腿不动留 cc 级）──
        let (lu_pre, su_pre) = self.pair_exposure();
        let tw_pre = self.total_wealth(c);
        let moved_units = self.leg_pairs[cc].long_units;
        let moved_basis = self.leg_pairs[cc].long_basis;
        // 否定线更新为 target 级中枢 ZD（核心现骑 target 走势）；None（target 无中枢）⇒ 保留原否定线（不退化无保护）。
        let new_stop = self
            .faceb_stop(view.zd[target])
            .unwrap_or(self.leg_pairs[cc].long_stop);
        self.leg_pairs[target].long_units = moved_units;
        self.leg_pairs[target].long_basis = moved_basis;
        self.leg_pairs[target].long_stop = new_stop;
        self.leg_pairs[cc].long_units = 0.0;
        self.leg_pairs[cc].long_basis = f64::NAN;
        self.leg_pairs[cc].long_stop = f64::NAN;
        // relabel = 级别重标定非加仓，敞口必不变（对照 instances prove_relabel_invariant）。
        let (lu_post, su_post) = self.pair_exposure();
        assert!(
            (lu_post - lu_pre).abs() < 1e-9 && (su_post - su_pre).abs() < 1e-9,
            "pair_emergence relabel 敞口违反：long {lu_pre}->{lu_post} short {su_pre}->{su_post}（cc={cc} target={target}）"
        );
        self.pair_emergence_upgrades += 1;
        self.guards.set_trigger(OpTrigger::Emergence); // 涌现是 relabel 的合法非 BSP 触发源（对照 ascend）
        self.guards.note_op("pair_emergence_upgrade");
        self.prove_tw_neutral(tw_pre, c);
    }

    /// **否定线止损（任务57=53.1 第三结构点）**：每级别多/空腿价格否定结构破坏 ⇒ 止损平。
    /// 多腿：c < long_stop（进场 ZD，跌破=结构破坏）⇒ 平。空腿：c > short_stop（进场 ZG，涨破=结构破坏）⇒ 平。
    /// 「回试回中枢=假突破」⟹ 止损。止损先于 NAV≤0（账户级强平）生效 ⇒ 零强平判据。
    fn pair_stop_loss_step(&mut self, c: f64) {
        for k in 0..MAX_LEVEL {
            // 多腿否定线：跌破进场 ZD ⇒ 结构破坏止损。
            if self.leg_pairs[k].long_active() {
                let stop = self.leg_pairs[k].long_stop;
                if stop.is_finite() && c < stop {
                    self.guards.set_trigger(OpTrigger::Bsp); // 否定线 = 结构破坏（type3 形态对偶）= 合法触发源
                    self.last_close_reason = 1; // #170 TC 诊断：否定线止损
                    self.pending_exit_trigger = 3; // #164 R2：否定线止损
                    self.close_long_leg(k, c);
                    self.pair_long_stops += 1;
                }
            }
            // 空腿否定线：涨破进场 ZG ⇒ 结构破坏止损。
            if self.leg_pairs[k].short_active() {
                let stop = self.leg_pairs[k].short_stop;
                if stop.is_finite() && c > stop {
                    self.guards.set_trigger(OpTrigger::Bsp);
                    self.last_close_reason = 1; // #170 TC 诊断：否定线止损
                    self.pending_exit_trigger = 3; // #164 R2：否定线止损
                    self.close_short_leg(k, c);
                    self.pair_short_stops += 1;
                }
            }
        }
    }

    /// **g_pair(k)**（每级别一对腿单算子）：级别 k 消费 `view.buy[k]`/`view.sell[k]`（买卖点）开平 + 链破坏 churn 门控。
    ///
    /// 开平规则（对称）：
    /// - **买点 fire**：① 空腿持仓 ⇒ 平空（反向买卖点平，与开对称）；② 多腿空 ⇒ 开多（买点开多腿）。
    /// - **卖点 fire**：① 多腿持仓 ⇒ 平多（反向卖点平）；② 空腿空 ⇒ 开空（卖点开空腿）。
    ///
    /// **区间套-confirmed 买卖点门控（任务69）——确认深度按持仓尺度自相似分级（567洞察①「腿开平绑买卖点+区间套」）**：
    /// - **核心多腿 churn**（= highest_active_long）：只在 `d_top`（走势完成真顶 = type1买卖点 + 区间套 nesting 全深度=
    ///   最深确认，稀疏）才平/翻转；走势未完成（回调/假突破）⇒ 不平核心多腿（保护主力骑牛不踏空）。次级别 long 无门控。
    /// - **开空腿**：只在 `t3sell`（第三类卖点=突破中枢下沿+回试不回=单层区间套转折，第27课）∧ **严格次级别（k<核心）** 才开；
    ///   假突破（回试回中枢）不开（滤震荡累积止损）；核心及其上绝不翻空（net-up 假顶翻空打主浪=灾难，L3 坐实）。
    /// 自相似同构：皆该级别区间套-confirmed 买卖点驱动、确认深度∝持仓尺度（主力 d_top 全深度/短差 t3sell 单层）、
    /// 角色（core vs sub）由 `highest_active_long` 结构涌现（零 if level==N/regime）。
    ///
    /// **开多入场方向/级别门控判据（T1 方向错位修复，#164/R1）**——结构涌现，零 if regime/level==N。
    /// 返回 true ⇒ 允许在级别 k 开多腿。`Any` 恒真（bit-exact）；`SegAlign` 要求本级别段方向上涨；
    /// `CrossLevel` 要求无更高活跃级别走势段下落（攻 65.5% xlc / 546/547 级别错配）。
    ///
    /// **核心多腿级别豁免（L3 ES 退化诊断 #164/R1）**：方向门控**仅作用于次级别开多**——核心多腿级别
    /// （`highest_active_long()==k`，或核心之上 k≥core）**豁免门控**（恒返回 true）。依据：L3 解剖坐实
    /// SegAlign 在低级别（L0/L2）净改善（削逆势 churn），但在核心/高级别（ES L3 long_pnl 24990→4787，−20203）
    /// **踏空**——高级别「下落段」实为主升浪中的大押小回，核心多腿应骑走势直到走势完成（d_top churn 门控管平仓，
    /// 非入场方向门控）。与开空 `below_core_long` 门**完全对称**：开空仅次级别（核心之下），开多门控也仅次级别。
    /// 核心入场由 trend_done_clear/emergence 重建路径管，不在 g_pair 方向门控范围。
    fn long_entry_ok(&self, k: usize, view: &LevelView) -> bool {
        if self.pair_long_entry == LongEntry::Any {
            return true; // bit-exact 基线
        }
        // 核心多腿级别及其上豁免方向门控（次级别才门控，与 below_core_long 对称）。
        // 无核心（全空仓）⇒ 任意级别都算「次级别」（首次建仓须放行，否则永不建仓）。
        if let Some(core) = self.highest_active_long() {
            if k >= core {
                return true;
            }
        }
        match self.pair_long_entry {
            LongEntry::Any => true,
            LongEntry::SegAlign => matches!(
                view.nodes.get(k).and_then(|n| n.map(|t| t.direction)),
                Some(Direction::Up)
            ),
            LongEntry::CrossLevel => {
                // ∀ j>k：更高活跃级别走势段不为下落（None=该级别无活跃走势=不冲突）。
                !((k + 1)..MAX_LEVEL).any(|j| {
                    matches!(
                        view.nodes.get(j).and_then(|n| n.map(|t| t.direction)),
                        Some(Direction::Down)
                    )
                })
            }
        }
    }

    /// **开空入场方向/级别门控判据（T1 修复对称镜像，#164/R1）**——开多 `long_entry_ok` 的对偶。
    /// `Any` 恒真（bit-exact，开空仍仅受 below_core_long 门）；`SegAlign` 要求本级别段方向下跌；
    /// `CrossLevel` 要求无更高活跃级别走势段上涨。开空本就受 `below_core_long`（严格次级别）门控
    /// （g_pair short_level_ok），故无需额外核心豁免——但为对称保留同一结构（核心级开空在 net-up 已被
    /// below_core_long 封死，此处方向门控对次级别开空叠加生效）。
    fn short_entry_ok(&self, k: usize, view: &LevelView) -> bool {
        match self.pair_long_entry {
            LongEntry::Any => true,
            LongEntry::SegAlign => matches!(
                view.nodes.get(k).and_then(|n| n.map(|t| t.direction)),
                Some(Direction::Down)
            ),
            LongEntry::CrossLevel => !((k + 1)..MAX_LEVEL).any(|j| {
                matches!(
                    view.nodes.get(j).and_then(|n| n.map(|t| t.direction)),
                    Some(Direction::Up)
                )
            }),
        }
    }

    /// **G 轴分量① 级别-方向对齐门（task#5，539 方向误读失血修复，L0 结构）**：开仓腿极性须与**该腿
    /// 自身级别 k 的走势方向** `view.nodes[k].direction` 对齐——`want_long` ⇒ 须 `Up`、开空 ⇒ 须 `Down`。
    /// 539 根因：`d_k=Up` 却开空 ⟹ `L_confirm=∞` ⟹ 孤儿腿（无配对闭合转折节点）⟹ 必失血（payoff_fiber
    /// §2.2 概念A）。**对所有级别（含核心）施加同级别方向对齐**——539 根因不豁免核心（区别 `long_entry_ok`
    /// 的核心豁免 + 跨级别口径）。`nodes[k]==None`（该级无走势）⇒ 无方向读数 ⇒ 不阻断（无孤儿判据，放行）。
    /// OFF（`!enable_g_axis`）⇒ 恒 true（bit-exact，不读 nodes 方向）。
    fn g_axis_align_ok(&self, k: usize, want_long: bool, view: &LevelView) -> bool {
        if !self.enable_g_axis {
            return true; // bit-exact 基线
        }
        match view.nodes.get(k).and_then(|n| n.map(|t| t.direction)) {
            Some(Direction::Up) => want_long, // d_k=Up：只许开多（开空=方向误读孤儿，539）
            Some(Direction::Down) => !want_long, // d_k=Down：只许开空
            None => true,                     // 该级无走势 ⇒ 无方向孤儿判据 ⇒ 放行
        }
    }

    /// **G 轴分量② 强平可达性否定线（task#5，孤儿腿不可达闭合修复，L0 结构）**：G 轴 ON ⇒ 给开成的腿
    /// **独立锁定**进场中枢否定线（多腿=ZD `view.zd[k]` / 空腿=ZG `view.zg[k]`）作可达强平路径，使
    /// `pair_stop_loss_step` 能在结构破坏即平（可达闭合），而非孤儿腿仅经 NAV≤0 账户级强平（不可达闭合，
    /// 浮亏先吞 NAV）。与 `faceb_stop` 正交（不依赖 `enable_uniform_sizing`）。OFF ⇒ None（走 faceb_stop
    /// 原路径 = bit-exact）。zd/zg=None（该级无中枢）⇒ None（无否定线原料，可达性 fallback 到反向买卖点平）。
    fn g_axis_stop(&self, raw: Option<f64>) -> Option<f64> {
        if self.enable_g_axis {
            raw
        } else {
            None
        }
    }

    /// **零方向几何**：开仓方向由买卖点（buy/sell）涌现，不由 node.direction。**只写 leg_pairs[k]**（547 隔离）。
    fn g_pair(&mut self, k: usize, view: &LevelView, top: usize, c: f64) {
        let b = view.buy[k];
        let s = view.sell[k];
        if b && s {
            return; // 同 bar 同级别买卖冲突 ⇒ 跳过（对照 on_bar route_bsp 同 level 买卖冲突）
        }
        // 「核心」= 当前最高活跃多腿级别（结构涌现，无 if level==top 硬编码）。
        let is_core_long_level = self.highest_active_long() == Some(k);
        // **区间套-confirmed 买卖点驱动门控（任务69）——区间套确认深度按持仓尺度自相似分级（编排者「快且准」+ 567洞察①）**：
        //   - **核心多腿 churn**（平主力 long）← `d_top`（区间套链贯通到 a0 = 走势完成真顶 = type1买卖点 + 区间套nesting 全深度，
        //     最深确认 ⇒ 稀疏，保护主力骑牛不踏空）。注：567「d_top 错误代理」指的是把**空腿**绑 d_top（稀疏⇒空腿冻结），
        //     核心主力 churn 恰需稀疏（骑牛），故沿用 d_top。L3 坐实：核心 churn 若改频繁信号(t1sell)⇒ 趋势踏空
        //     （GC long +63522→+16227 / QQQ +36595→+10751）。
        //   - **次级别开空腿**（吃回调短差）← `t3sell`（第三类卖点=突破中枢下沿+回试不回=单层区间套转折，第27课，响应回调）。
        // 自相似原则：区间套确认深度 ∝ 持仓尺度（主力=全深度 d_top / 短差=单层 t3sell），角色由 `highest_active_long` 结构涌现
        // 决定（零 if level==N/regime）。这正是 567洞察①「腿开平绑买卖点+区间套」——主力绑走势完成、短差绑第三类转折。
        let core_done = view.d_top[k];
        // **开空触发器（做空腿对称化，#108 L2 根因修复）**：默认 T3（=view.t3sell，#117 基线，逐位复现）。
        // 候选A（T1=view.t1sell 顶背驰）/ 候选B（Any=view.sell 任意卖点 = `s`，与多头 view.buy 完全对称）
        // 把开空提到与多头 view.buy 同等及时度——消解 #108「做空仅 t3sell 晚建」不对称。`below_core_long` 门
        // + zg 否定线封顶（下方 short_level_ok / open_short_leg）**不变** ⇒ net-up 假顶仍封死核心翻空（防灾难）。
        let sub_break = match self.pair_short_entry {
            ShortEntry::T3 => view.t3sell[k],
            ShortEntry::T1 => view.t1sell[k],
            ShortEntry::Any => s, // = view.sell[k]，已在 `if s` 块内恒真 ⇒ 任意卖点（below_core_long 内）开空
        };

        // **开多腿入场方向/级别门控（T1 方向错位修复，#164/R1）**：T1（58.7%，最大可约靶子）= 入场在与
        // 持仓方向逆向的同级别走势段（开多在跌段）。L3 解剖坐实买点在下落段途中 fire 即开多 = wrong-side。
        // 门控构造涌现（零 if regime/level==N）：
        //   - SegAlign：本级别段方向上涨（`view.nodes[k].direction == Up`）才开多 = 攻 T1 定义（同级别逆向）。
        //   - CrossLevel：无更高活跃级别走势段下落（∀ j>k，`view.nodes[j].direction != Down`）才开多 = 攻
        //     65.5% xlc（546/547 级别错配：次级别买点不在高级别下落段中逆势开多）。
        //   - Any：现状零门控（bit-exact）。
        // 对称镜像作用于开空（见下方 short_dir_ok），与 below_core_long 门正交叠加。
        let long_dir_ok = self.long_entry_ok(k, view);
        // **G 轴分量① 级别-方向对齐门（task#5，539 方向误读失血修复）**：开多须 d_k=Up（同级别方向对齐），
        // 否则方向误读孤儿（539 根因）⇒ 拦截（OFF ⇒ 恒 true，bit-exact）。与 long_dir_ok 正交叠加。
        let g_long_ok = self.g_axis_align_ok(k, true, view);

        if b {
            // 买点：先平空腿（反向买卖点平），再开多腿（若多腿空 ∧ 方向门控通过 ∧ G 轴对齐通过）。
            if self.leg_pairs[k].short_active() {
                self.guards.set_trigger(OpTrigger::Bsp);
                self.last_close_reason = 0; // #170 TC 诊断：买卖点反向平
                self.pending_exit_trigger = if view.t1buy[k] { 0 } else { 1 }; // #164 R2：type1（走势完成）vs 非type1（type2/3）
                self.close_short_leg(k, c);
            }
            // G 轴对齐拦截观测（开多但 d_k≠Up = 方向误读孤儿被堵；仅多腿空 ∧ long_dir_ok 时算「本会开」的拦截）。
            if self.enable_g_axis && !self.leg_pairs[k].long_active() && long_dir_ok && !g_long_ok {
                self.g_axis_align_blocked += 1;
            }
            if !self.leg_pairs[k].long_active() && long_dir_ok && g_long_ok {
                self.guards.set_trigger(OpTrigger::Bsp);
                // G 轴 ON ⇒ 独立锁强平可达否定线 ZD（孤儿腿可达闭合修复）；OFF ⇒ faceb_stop 原路径（bit-exact）。
                let stop = self
                    .g_axis_stop(view.zd[k])
                    .or_else(|| self.faceb_stop(view.zd[k]));
                if self.enable_g_axis && stop.is_some() {
                    self.g_axis_reach_stops_set += 1;
                }
                self.open_long_leg(k, top, stop, c);
                // L_confirm 第四轴捕获（task#95，observation-only，open 成功后才捕获）：
                // c=该买点确认深度（t1buy=type1/其余=type2）；leg=H⁰核心 iff 开仓后 k 是最高活跃多腿
                // （= 骑最高/主走势=核心角色，597/#80 H⁰），否则 H¹（次级别多腿，衬底/机动角色）。
                // **注**：is_core 须在 open 后重算（open 前 highest_active_long 不含本腿；首条核心腿开仓前
                // highest_active_long()=None ⇒ is_core_long_level=false 是错的）。
                if self.enable_lconfirm_audit && self.leg_pairs[k].long_active() {
                    self.leg_pairs[k].long_conf = ConfDepth::from_buy(view, k);
                    self.leg_pairs[k].long_is_core = self.highest_active_long() == Some(k);
                }
            }
            return;
        }
        if s {
            // 卖点：先平多腿（反向买卖点平），但核心多腿受 churn 门控（链破坏才平=转折，链完整=回调不动核心）。
            if self.leg_pairs[k].long_active() {
                // 核心多腿只在走势完成（d_top=最深区间套真顶）才平（保护主力骑牛）；次级别 long 无门控（任意卖点平）。
                let may_close_core = !is_core_long_level || core_done;
                if may_close_core {
                    self.guards.set_trigger(OpTrigger::Bsp);
                    self.last_close_reason = 0; // #170 TC 诊断：买卖点反向平
                                                // 出场触发分类（#164 R2）：type1（t1sell=走势完成=574 floor 最小滞后）/type3（t3sell）/type2（其余 sell）。
                    self.pending_exit_trigger = if view.t1sell[k] {
                        0
                    } else if view.t3sell[k] {
                        2
                    } else {
                        1
                    };
                    self.close_long_leg(k, c);
                    if is_core_long_level && core_done {
                        self.pair_core_churns += 1; // 走势完成 churn 动核心多腿（真顶转折）
                                                    // **牛转熊核心翻空镜像（任务 bear-validate，编排者：最高活跃级别走势完成→核心翻空）**：
                                                    // 放开 #69 `k<核心` 禁令——核心走势完成（d_top=最深区间套真顶=type1+全深度背驰链）不止
                                                    // 平多到现金，而是**翻空**（大额吃熊，max_gross>1×=核心仓尺度）。结构涌现：is_core_long_level
                                                    // = highest_active_long==k（零 if regime/level，编排者 no-hardcode）。zg[k]=进场中枢上沿
                                                    // =否定线止损（涨破⇒牛市恢复⇒止损出，27课区间套否定）。有效域 ⊂ 真 bear（231号
                                                    // formalization-validity-domain）：net-up 假顶翻空打主升浪=灾难（539），须 bear 数据 L3。
                                                    // G 轴分量① 对齐门（task#5）：核心翻空须 d_k=Down（走势已向下完成）；若 nodes[k] 仍 Up
                                                    // 则翻空=方向误读孤儿（539 根因，核心不豁免）⇒ 拦截。OFF ⇒ g_core_short_ok 恒 true（bit-exact）。
                        let g_core_short_ok = self.g_axis_align_ok(k, false, view);
                        if self.enable_g_axis
                            && self.enable_pair_core_short
                            && !self.leg_pairs[k].short_active()
                            && !g_core_short_ok
                        {
                            self.g_axis_align_blocked += 1;
                        }
                        if self.enable_pair_core_short
                            && !self.leg_pairs[k].short_active()
                            && g_core_short_ok
                        {
                            self.guards.set_trigger(OpTrigger::Bsp);
                            // G 轴 ON ⇒ 锁强平可达否定线 ZG；OFF ⇒ faceb_stop 原路径（bit-exact）。
                            let stop = self
                                .g_axis_stop(view.zg[k])
                                .or_else(|| self.faceb_stop(view.zg[k]));
                            if self.enable_g_axis && stop.is_some() {
                                self.g_axis_reach_stops_set += 1;
                            }
                            self.open_short_leg(k, top, stop, c);
                            // L_confirm 捕获（task#95）：核心走势完成翻空 = 核心尺度（H⁰）+ 全深度确认（type1/d_top）。
                            // is_core=true（此 short 由核心 churn 触发，是核心 H⁰ 在 R− 方向的翻转，非次级别 H¹ 短差）。
                            if self.enable_lconfirm_audit && self.leg_pairs[k].short_active() {
                                self.leg_pairs[k].short_conf = ConfDepth::from_sell(view, k);
                                self.leg_pairs[k].short_is_core = true;
                            }
                        }
                    }
                }
                // 走势未完成 ∧ 核心多腿 ⇒ 不平核心多腿（回调），落到下方开空腿吃回调。
            }
            // **开空腿门控（任务69，编排者「次级别做空」+ per-level L3 诊断 + geom_tower_quota 结构）**：
            //   ① `sub_break`（=t3sell=突破中枢下沿+回试不回=真顶转折）才开空（假突破不开，滤假突破累积止损）；
            //   ② **仅在核心多腿级别之下开空（`k < 核心级别`）——绝不在核心或其上开空**。
            // 依据：编排者纲领「本级别卖点→平多 + 次级别做空」（核心只平多到现金，做空在严格次级别）。L3 per-level 坐实：
            //   高级别空腿 = geom_tower_quota 最大配额（depth=top−k 小⇒notional 大）× 在 net-up regime「假顶翻空打主升浪」
            //   = 灾难（变体1 CL L4 单笔−40928 / BTC L3 −20629 / GC L3 −7335，皆 1-3 笔巨亏）；严格次级别短差小配额吃回调
            //   正域（变体4 CL L0/L1/L2 +3145 / BTC L1/L2/L3 +10670 / GC/ES/OKLO 正）。`!is_core` 不够（杀手空腿开在核心
            //   之上非核心本身）⇒ 须 k<核心。
            // 牛熊切换（③）：核心走势完成（d_top）⇒ churn 平核心多腿到现金（不翻空，21:40 升跌完备性=上涨趋势无真顶卖点）；
            //   无核心多腿（熊市镜像）⇒ 无 long 框架 ⇒ 当前不开空（保守；熊市核心做空镜像吃熊=开放轴，8 标的均 net-up
            //   无法 L3 证伪 ⇒ 不擅自实装=避有效域膨胀）。
            // 自相似 no-hardcode：`highest_active_long()` 是结构涌现（无 if level==N/regime），k<core 跨级别同构。
            let below_core_long = self.highest_active_long().map_or(false, |core| k < core);
            // 变体2（放开 k<核心 t3sell 大额做空）：移除 below_core_long 限制 ⇒ t3sell 在核心及其上开空（max_gross>1×）。
            let short_level_ok = below_core_long || self.enable_pair_core_short_open;
            // **开空方向门控镜像（T1 修复对称，#164/R1）**：与开多 long_entry_ok 对称——SegAlign 要求本级别段
            // 下跌（nodes[k]==Down）/ CrossLevel 要求无更高活跃级别上涨段。Any ⇒ 恒真（bit-exact，开空仍仅
            // 受 below_core_long 门）。与 short_level_ok（核心隔离）正交叠加。
            let short_dir_ok = self.short_entry_ok(k, view);
            // **G 轴分量① 级别-方向对齐门（task#5，539 方向误读失血修复）**：开空须 d_k=Down（同级别方向
            // 对齐）。次级别短差腿（t3sell）若开在 d_k=Up 段 = 方向误读孤儿（539 根因）⇒ 拦截。OFF ⇒ 恒 true。
            let g_short_ok = self.g_axis_align_ok(k, false, view);
            if self.enable_g_axis
                && !self.leg_pairs[k].short_active()
                && sub_break
                && short_level_ok
                && short_dir_ok
                && !g_short_ok
            {
                self.g_axis_align_blocked += 1;
            }
            if !self.leg_pairs[k].short_active()
                && sub_break
                && short_level_ok
                && short_dir_ok
                && g_short_ok
            {
                self.guards.set_trigger(OpTrigger::Bsp);
                // G 轴 ON ⇒ 锁强平可达否定线 ZG（孤儿腿可达闭合修复）；OFF ⇒ faceb_stop 原路径（bit-exact）。
                let stop = self
                    .g_axis_stop(view.zg[k])
                    .or_else(|| self.faceb_stop(view.zg[k]));
                if self.enable_g_axis && stop.is_some() {
                    self.g_axis_reach_stops_set += 1;
                }
                self.open_short_leg(k, top, stop, c);
                // L_confirm 第四轴捕获（task#95，observation-only，open 成功后）：
                // c=该卖点确认深度（t1sell=type1/t3sell=type3/其余=type2，#69 次级别短差默认 t3=单层 c=浅）；
                // leg=H¹ 短差 当 k<核心（below_core_long）；H⁰ 核心 当放开门控后 k≥核心（coreshort_open 大额做空）。
                if self.enable_lconfirm_audit && self.leg_pairs[k].short_active() {
                    self.leg_pairs[k].short_conf = ConfDepth::from_sell(view, k);
                    self.leg_pairs[k].short_is_core = !below_core_long; // k≥核心（放开门控）⇒ 核心尺度 H⁰；k<核心 ⇒ H¹ 次级别
                }
            }
            return;
        }
    }

    /// **consume_leg_pairs**（单算子递归 = consume_legs 的对偶）：账户强平 → 止损 → a0→涌现上界逐级 g_pair(k)。
    fn consume_leg_pairs(&mut self, view: &LevelView, c: f64) {
        // ⓪ 账户级 NAV≤0 强平（诚实会计安全网）：否定线止损应先于此生效 ⇒ n_liquidations==0 = 止损有效判据。
        //    保留此块是诚实会计（不声明「永不强平」而无强平路径）；零强平由否定线止损保证，非删除强平路径。
        if self.nav(c) <= 0.0 {
            for k in 0..MAX_LEVEL {
                if self.leg_pairs[k].long_active() {
                    let entry_bar = self.cur_bar;
                    let entry_basis = self.leg_pairs[k].long_basis;
                    self.guards.set_trigger(OpTrigger::Eod);
                    self.last_close_reason = 2; // #170 TC 诊断：NAV 强平
                    self.pending_exit_trigger = 5; // #164 R2：账户强平
                    self.close_long_leg(k, c);
                    self.n_liquidations += 1;
                    self.liq_log
                        .push((k, entry_bar, entry_basis, self.cur_bar, c, false));
                }
                if self.leg_pairs[k].short_active() {
                    let entry_bar = self.cur_bar;
                    let entry_basis = self.leg_pairs[k].short_basis;
                    self.guards.set_trigger(OpTrigger::Eod);
                    self.last_close_reason = 2; // #170 TC 诊断：NAV 强平
                    self.pending_exit_trigger = 5; // #164 R2：账户强平
                    self.close_short_leg(k, c);
                    self.n_liquidations += 1;
                    self.liq_log
                        .push((k, entry_bar, entry_basis, self.cur_bar, c, true));
                }
            }
        }
        // ① 否定线止损（第三结构点，先于买卖点开平 ⇒ 否定线优先平失血腿）。
        self.pair_stop_loss_step(c);
        // ①.5 核心多腿涌现升级（R3 段无腿 #164/#6）：核心同向跟随 emergent_top relabel 上移（敞口不变）。
        //     位置 = 止损后、g_pair 前（对照 instances step「强平→emergence→route_bsp」序）。OFF/Face A/B
        //     （enable_pair_emergence=false）⇒ 提前 return（无 relabel，bit-exact）。
        if self.enable_pair_emergence {
            self.pair_emergence_upgrade(view, c);
        }
        // ② 逐级买卖点开平 + churn 门控。
        let top = match (0..MAX_LEVEL).rev().find(|&k| view.nodes[k].is_some()) {
            Some(t) => t,
            None => return,
        };
        for k in 0..=top {
            self.g_pair(k, view, top, c);
        }
    }

    /// 读法B LegPair 收尾（全平所有对腿到现金）。
    fn finish_leg_pairs(&mut self, c: f64) {
        if !(c.is_finite() && c > 0.0) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Eod);
        self.last_close_reason = 2; // #170 TC 诊断：finish 收尾
        for k in 0..MAX_LEVEL {
            if self.leg_pairs[k].long_active() {
                self.close_long_leg(k, c);
            }
            if self.leg_pairs[k].short_active() {
                self.close_short_leg(k, c);
            }
        }
    }

    /// LegPair 只读访问（诊断/L3）。
    pub fn leg_pair(&self, k: usize) -> &LegPair {
        &self.leg_pairs[k]
    }

    /// **L_confirm 第四轴纤维只读访问（task#95，L3）**：返回 `pi_k_leg_dir_c[k][leg][dir][c]` + 计数。
    pub fn lconfirm_cell(&self, k: usize, leg: usize, dir: usize, c: usize) -> (f64, u64) {
        (
            self.pi_k_leg_dir_c[k][leg][dir][c],
            self.pi_count_k_leg_dir_c[k][leg][dir][c],
        )
    }

    /// **跨级别 k 聚合的 (leg,dir,c) 纤维 P&L + 计数（task#95，符号分裂裁定用）**：固定 leg/dir，对每个 c
    /// 桶求和 Σ_k π(k,leg,dir,c)。同 (leg,dir) 下不同 c 桶的符号比较 = 第四轴成立/否证的可证伪判据。
    pub fn lconfirm_cell_sum(&self, leg: usize, dir: usize, c: usize) -> (f64, u64) {
        let mut p = 0.0;
        let mut n = 0u64;
        for k in 0..MAX_LEVEL {
            p += self.pi_k_leg_dir_c[k][leg][dir][c];
            n += self.pi_count_k_leg_dir_c[k][leg][dir][c];
        }
        (p, n)
    }

    /// **机械穷尽守卫（task#95，补 #92 缺的可结算底座）**：断言所有纤维格子的 realized P&L 之和
    /// == 总 leg realized P&L（Σ pair_long_pnl + Σ pair_short_pnl）。不等 ⇒ 有未分类的 P&L 流（分类不完备
    /// 的可验证信号）。tol=1e-6（f64 累加误差容差）。返回 (Σ纤维, Σ总leg, 是否一致)。**只在 audit ON 有意义。**
    pub fn lconfirm_exhaustive_check(&self) -> (f64, f64, bool) {
        let mut fiber_sum = 0.0;
        for k in 0..MAX_LEVEL {
            for leg in 0..2 {
                for dir in 0..2 {
                    for c in 0..3 {
                        fiber_sum += self.pi_k_leg_dir_c[k][leg][dir][c];
                    }
                }
            }
        }
        let total_leg: f64 = (0..MAX_LEVEL)
            .map(|k| self.pair_long_pnl[k] + self.pair_short_pnl[k])
            .sum();
        // pi_total_audited 是累加镜像（应 == fiber_sum 逐位）；total_leg 是独立来源（pair_*_pnl）。
        // 三者一致 ⇒ 穷尽：每笔 leg realized 都进了且仅进了一个纤维格子。
        let consistent = (fiber_sum - total_leg).abs() < 1e-6
            && (self.pi_total_audited - fiber_sum).abs() < 1e-9;
        (fiber_sum, total_leg, consistent)
    }

    /// **机械穷尽守卫断言版（task#95）**：不一致即 panic（暴露未分类 P&L 流）。L3 跑结束调用。
    pub fn assert_lconfirm_exhaustive(&self) {
        let (fiber_sum, total_leg, consistent) = self.lconfirm_exhaustive_check();
        assert!(
            consistent,
            "L_confirm 机械穷尽守卫违反（task#95）：Σ纤维格子={fiber_sum:.6} ≠ Σ总leg realized={total_leg:.6} \
             (pi_total_audited={:.6})；差={:.6} ⇒ 有未分类的 P&L 流（K×L×D×c 分类不完备）",
            self.pi_total_audited, fiber_sum - total_leg
        );
    }

    /// **Face A LegPair 核心多腿级别**（= `highest_active_long`，pub 诊断访问器，R3 段无腿实证）。
    /// 最高活跃多腿级别 = 当前「核心」。None=无活跃多腿。诊断用：查核心是否到最高涌现级别。
    pub fn pair_core_long_level(&self) -> Option<usize> {
        self.highest_active_long()
    }

    // ──────────────── NEST：命题4 读法乙（大级别背驰段闸门 + a0 区间套定位翻转）────────────────

    /// **nest_step**（命题4 读法乙，源头审计 src-prop13 / 第27课区间套）：
    /// 1. **大级别背驰段武装**（自顶向下前提）：`view.top_diverge` = 最高级别进入背驰段时的操作极性
    ///    （上涨顶背驰段→Short / 下跌底背驰段→Long）。背驰段 ⊊ 走势完成（区间套前提是背驰段非完成）。
    ///    武装跨重跑持续；top 反转（`top_trend_dir` 与 armed 不一致）⇒ 清武装（去陈旧武装）。
    /// 2. **a0 区间套定位**：在武装下，找**最低级别**与武装极性一致的 type1（顶背驰段找 type1_sell /
    ///    底背驰段找 type1_buy）= 区间套向下定位到的精确买卖点（最低级别 = 最接近 a0）。
    /// 3. **整仓翻转操作**：定位点 ∧ 当前持仓 ≠ 武装极性 ⇒ clear_all（close/cover）+ enter（做多/做空）
    ///    全仓。翻转后 cur==op ⇒ 同武装不再翻（幂等）；持仓骑走势直到反向武装定位。
    ///
    /// **不是读法甲**（全 a0 穷尽机械翻转）：唯有大级别背驰段武装下的 a0 定位点才翻转，背驰段是筛选闸门。
    /// **有效域边界**（不声明膨胀）：区间套中间级别逐级背驰段未逐层校验（仅 top 武装 + a0 最低定位）；
    /// 严格逐级嵌套（每级别背驰段）未实装，记为有效域边界（与 divergence.rs 条件3/序数简化同类）。
    fn nest_step(&mut self, view: &LevelView, bar: i64, c: f64) {
        // ── 1. 去陈旧武装：top 反转（当前 top 走势方向与 armed 期望方向不符）⇒ 清武装 + 开新窗口 ──
        if let (Some(armed), Some(dir)) = (self.nest_armed_op, view.top_trend_dir) {
            // armed Short 期望 top=Up；armed Long 期望 top=Down。不符 ⇒ top 已反转 ⇒ 清。
            let consistent = match dir {
                Direction::Up => Polarity::Short,
                Direction::Down => Polarity::Long,
            };
            if armed != consistent {
                self.nest_armed_op = None;
                self.nest_consumed = false; // 窗口结束，下个背驰段窗口可再翻
            }
        }
        // ── 1'. 武装/更新：最高级别进入背驰段 ⇒ armed = 操作极性。极性变 = 新窗口 ⇒ 重置 consumed ──
        if let Some(op) = view.top_diverge {
            if self.nest_armed_op != Some(op) {
                self.nest_consumed = false; // 新背驰段窗口（或方向反转）⇒ 允许一次新定位翻转
            }
            self.nest_armed_op = Some(op);
        }
        // ── 2. 主翻转尝试（top 背驰段闸门 + a0 区间套定位）──
        let flipped = self.nest_try_flip(view, bar, c);
        // ── 3. consume平空（任务22）：未翻转时，次级别反核心向背驰段 ⇒ 平核心仓 1/3（缩短长持死扣）──
        if !flipped && self.enable_nest_consume {
            self.nest_consume_step(view, bar, c);
        }
    }

    /// **主翻转**（top 背驰段武装 + a0 区间套定位 + 可选严格逐级）。返回是否翻转（供 consume 判断）。
    fn nest_try_flip(&mut self, view: &LevelView, bar: i64, c: f64) -> bool {
        let armed = match self.nest_armed_op {
            Some(op) => op,
            None => return false, // 未武装 = 大级别未进背驰段 ⇒ 不操作（556 可能冻结根源，L3 测）
        };
        if self.nest_consumed {
            return false; // 本背驰段窗口已翻过 ⇒ 持仓骑走势（读法乙：一个转折点）
        }
        let want_buy = armed == Polarity::Long; // 底背驰段武装 → 找 type1_buy；顶背驰段 → type1_sell
        let located = (0..MAX_LEVEL).find(|&k| {
            if want_buy {
                view.t1buy[k]
            } else {
                view.t1sell[k]
            }
        });
        let loc = match located {
            Some(k) => k,
            None => return false, // 武装但本重跑无 a0 定位点 ⇒ 等下一定位点
        };
        // ── 严格逐级区间套（任务22 开放轴C）：定位点须 loc..=top_armed_level 逐级背驰段一致 ──
        //   非仅 top 武装 + a0 定位——区间套要求每级别都处一致背驰段（嵌套校验）。top_armed_level =
        //   最高有 level_diverge 的级别（= top_diverge 来源级别）。loc..top 间任一级别非一致背驰段 ⇒ 不翻。
        if self.enable_nest_strict {
            let top_lvl = (0..MAX_LEVEL)
                .rev()
                .find(|&k| view.level_diverge[k].is_some());
            if let Some(tl) = top_lvl {
                let cascade_ok = (loc..=tl).all(|k| view.level_diverge[k] == Some(armed));
                if !cascade_ok {
                    return false; // 逐级嵌套不贯通 ⇒ 非区间套精确定位点，不翻
                }
            } else {
                return false; // 无任何级别背驰段（不应到此，top_diverge 已 Some）
            }
        }
        // ── 整仓翻转（cover/close + 反向 enter），每窗口一次（consumed 守卫）──
        let cur = self.highest_active().map(|cc| self.instances[cc].direction);
        if cur == Some(armed) {
            self.nest_consumed = true; // 已在目标方向 ⇒ 本窗口视为已消费
            return false;
        }
        // 记录被平仓位逐笔（539 验收）。
        if let Some(cc) = self.highest_active() {
            let inst = self.instances[cc];
            if inst.units > EPS && inst.basis.is_finite() {
                let is_short = inst.direction == Polarity::Short;
                let pnl = match inst.direction {
                    Polarity::Long => inst.units * (c - inst.basis),
                    Polarity::Short => inst.units * (inst.basis - c),
                };
                self.nest_trades
                    .push((is_short, inst.node.start_bar, inst.basis, bar, c, pnl));
            }
        }
        self.guards.set_trigger(OpTrigger::Bsp); // a0 定位 type1 = 合法 BSP 触发源
        self.clear_all(c);
        let node = view.nodes.get(loc).copied().flatten().unwrap_or_else(|| {
            TrendNode::new(
                bar,
                bar,
                c,
                c,
                match armed {
                    Polarity::Long => Direction::Up,
                    Polarity::Short => Direction::Down,
                },
            )
        });
        self.enter(loc, armed, node, c);
        self.n_nest_flips += 1;
        self.nest_consumed = true; // 翻转后骑走势到窗口结束（top 反转）/反向窗口再翻。
        true
    }

    /// **consume平空步（任务22 双向多重赋格 consume 侧）**：核心仓持有期间，**次级别（k<核心级别）反核心向
    /// 背驰段定位** ⇒ 平核心仓配额（1/3，缩短整仓长持死扣，减强牛穿仓）。
    ///
    /// 缠论依据：T 算子 construct（建仓）的对偶 = consume（平仓）。读法乙原始（无 consume）= 整仓骑到 top
    /// 反转才平（1-2 年死扣）。命题2 平空欠触发根 = 走势完成触发稀疏（short-cover-diag）⇒ 换**次级别背驰段**
    /// 触发（频繁）解。底背驰段（次级别底）⇒ 平空（核心 Short 减仓）；顶背驰段（次级别顶）⇒ 平多（核心 Long 减仓）。
    /// **平空不开反向腿**（区分 sink：sink 父减+子开短差；consume 仅平核心向中性，缩短暴露）。
    fn nest_consume_step(&mut self, view: &LevelView, _bar: i64, c: f64) {
        let cc = match self.highest_active() {
            Some(k) => k,
            None => return, // 无核心仓 ⇒ 无可平
        };
        let cdir = self.instances[cc].direction;
        // 次级别（严格低于核心级别）反核心向背驰段：核心 Long → 找顶背驰段(Short极性)平多；核心 Short → 找底背驰段(Long极性)平空。
        let want_op = flip_pol(cdir); // 反核心向操作极性
        let sub_diverge = (0..cc).any(|k| view.level_diverge[k] == Some(want_op));
        if !sub_diverge {
            return; // 无次级别反核心向背驰段 ⇒ 不平
        }
        // a0 区间套定位：次级别反核心向 type1（核心 Short 找 type1_buy 平空 / 核心 Long 找 type1_sell 平多）。
        let want_buy = want_op == Polarity::Long;
        let located = (0..cc).any(|k| {
            if want_buy {
                view.t1buy[k]
            } else {
                view.t1sell[k]
            }
        });
        if !located {
            return; // 无次级别定位点 ⇒ 等下一定位点
        }
        // 平核心仓配额 1/3（机动仓基准 = anchor 不参与；nest 下 anchor=0 ⇒ 全仓）。
        let u = self.instances[cc].units;
        let mob_base = (u - self.instances[cc].anchor).max(0.0);
        let m = quota(mob_base);
        if !(m > 1e-12 && m.is_finite()) {
            return;
        }
        self.guards.set_trigger(OpTrigger::Bsp); // 次级别背驰段定位 = 合法 BSP 触发源
        let tw_pre = self.total_wealth(c);
        let mut free = self.free;
        let realized = rec_reduce(&mut self.instances[cc], m, &mut free, c);
        self.free = free;
        self.account_reduce(cdir, realized, c);
        self.n_nest_consumes += 1;
        self.nest_consume_pnl += realized;
        self.prove_tw_neutral(tw_pre, c);
    }

    // ──────────────── 单 bar 步进（= flat step）────────────────

    /// 单 bar 操作步进（= flat step）：强平 → emergence_upgrade → route_bsp top-down。
    pub fn on_bar(&mut self, view: &LevelView, bar: i64, c: f64) {
        self.cur_bar = bar;
        self.last_close = c;
        self.guards.set_trigger(OpTrigger::None); // 本 bar 起始无触发源（强平不经原子函数）
        let tw_pre = self.total_wealth(c);

        // ── 逐级内在配额信号注入（task#84 子5，仅 enable_intrinsic_quota ON）──
        //   route_bsp/sink/drain/add 不接收 view（架构现状），故在此把每级别触发腿的 ConfDepth 代理暂存，
        //   sink/drain/add 据 cur_l_pullback[sub]/cur_l_confirm[sub] 算 mobile_frac。**OFF ⇒ 整块跳过 ⇒
        //   新字段恒默认 T2 ⇒ 不被任何 OFF 配额路径读 ⇒ bit-exact**（OFF 配额仍走 quota()=σ-不变 1/3）。
        //   L_pullback = 触发腿深度（type1=d_top 深/type3=t3sell 浅/type2 中）；L_confirm = 同信号的确认滞后
        //   （type1/type3 单次区间套定位=低滞后/type2 须更多证据=高滞后）。优先 sell（反核心向回调=sink 主因）。
        if self.enable_intrinsic_quota {
            for k in 0..MAX_LEVEL {
                // 触发腿确认深度：本级别若有卖点取 from_sell，否则若有买点取 from_buy，皆无则保 T2（中性）。
                let conf = if view.sell[k] {
                    ConfDepth::from_sell(view, k)
                } else if view.buy[k] {
                    ConfDepth::from_buy(view, k)
                } else {
                    ConfDepth::T2
                };
                self.cur_l_pullback[k] = conf; // 深度轴：直接用确认深度档（T1 深/T3 浅）
                self.cur_l_confirm[k] = conf; // 滞后轴：mobile_frac 内部按 T1/T3=低滞后、T2=高滞后映射
            }
        }

        // ── 读法B 一对多空腿分叉（任务57=53.1 编排者重写）：ON ⟹ 每级别 LegPair 买卖点开平 + 否定线止损 +
        //   链破坏 churn 门控（多空双开吃所有级别涨跌幅）。OFF ⟹ 逐字不动（bit-exact）。最先检查（旁路单腿/instances）。
        if self.enable_reading_b_pair {
            self.consume_leg_pairs(view, c);
            // 杠杆验收（裂隙2）：毛/净敞口相对 NAV（多空双开 ⇒ gross 含多+空）。
            let (mut gross, mut net) = (0.0f64, 0.0f64);
            for p in &self.leg_pairs {
                if p.long_units > EPS {
                    gross += p.long_units * c;
                    net += p.long_units * c;
                }
                if p.short_units > EPS {
                    gross += p.short_units * c;
                    net -= p.short_units * c;
                }
            }
            let nav = self.nav(c).max(1.0);
            self.max_gross_exp_x100 = self
                .max_gross_exp_x100
                .max((gross / nav * 100.0).max(0.0) as u64);
            self.max_net_exp_x100 = self
                .max_net_exp_x100
                .max((net.abs() / nav * 100.0).max(0.0) as u64);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

        // ── 读法B/读法乙递归分叉（任务18 编排者修正）：ON ⟹ 每级别独立腿骑走势消费 d_top（删 sink/recover/
        //   ascend/clear_all/三阶段——每级别独立骑乘取代跨级短差）。OFF ⟹ instances 路径逐字不动（bit-exact）。
        if self.enable_reading_b {
            // 强平边界（账户级 NAV≤0 ⇒ 连锁全平腿）。
            if self.nav(c) <= 0.0 {
                for k in 0..MAX_LEVEL {
                    if self.legs[k].active {
                        let entry_bar = self.legs[k].riding_node.start_bar;
                        let entry_basis = self.legs[k].basis;
                        let is_short = self.legs[k].direction == Polarity::Short;
                        self.guards.set_trigger(OpTrigger::Eod);
                        self.close_leg(k, c);
                        self.n_liquidations += 1;
                        self.liq_log
                            .push((k, entry_bar, entry_basis, bar, c, is_short));
                    }
                }
            }
            self.consume_legs(view, c);
            // 杠杆验收（裂隙2）：毛/净敞口相对 NAV（恒仓 ⇒ ≤1×）。
            let (mut gross, mut net) = (0.0f64, 0.0f64);
            for leg in &self.legs {
                if leg.units > EPS {
                    let notional = leg.units * c;
                    gross += notional;
                    net += match leg.direction {
                        Polarity::Long => notional,
                        Polarity::Short => -notional,
                    };
                }
            }
            let nav = self.nav(c).max(1.0);
            self.max_gross_exp_x100 = self
                .max_gross_exp_x100
                .max((gross / nav * 100.0).max(0.0) as u64);
            self.max_net_exp_x100 = self
                .max_net_exp_x100
                .max((net.abs() / nav * 100.0).max(0.0) as u64);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

        // ── A. 边界算子：保证金强平，按三阶段切换（= flat）──
        match self.stage {
            RecStage::EarningShares => {
                // 全仓：账户级，in-system NAV≤0 ⇒ 连锁全平。这是**账户级抵消**（核心多头利润 −
                // 短差空头亏 = net），降成本立于不败的实现——逐仓 layer 独立会破坏抵消致短差翻倍亏 6×退化。
                if self.nav(c) <= 0.0 {
                    let nav_now = self.nav(c);
                    self.liq_snapshot.push((
                        2,
                        self.core_cost_basis,
                        self.withdrawn,
                        self.notional_in,
                        nav_now,
                    ));
                    let mut free = self.free;
                    for k in 0..MAX_LEVEL {
                        let kdir = self.instances[k].direction;
                        let u = self.instances[k].units;
                        if u > 1e-12 {
                            let entry_bar = self.instances[k].node.start_bar;
                            let entry_basis = self.instances[k].basis;
                            let is_short = kdir == Polarity::Short;
                            let realized = rec_reduce(&mut self.instances[k], u, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(kdir, realized, c);
                            if is_short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                            free = self.free;
                            self.liq_log
                                .push((k, entry_bar, entry_basis, bar, c, is_short));
                        }
                    }
                    self.free = free;
                }
            }
            _ => {
                // 逐仓：每层独立 basis 判（CostReduction / CapitalRecovered）。
                for k in 0..MAX_LEVEL {
                    let l = self.instances[k];
                    if l.units > 1e-12 {
                        let liq = match l.direction {
                            Polarity::Long => c > 0.0 && c <= l.basis / SUB_LIQ_FACTOR,
                            Polarity::Short => c >= SUB_LIQ_FACTOR * l.basis,
                        };
                        if liq {
                            let sid = match self.stage {
                                RecStage::CostReduction => 0,
                                RecStage::CapitalRecovered => 1,
                                RecStage::EarningShares => 2,
                            };
                            let nav_now = self.nav(c);
                            self.liq_snapshot.push((
                                sid,
                                self.core_cost_basis,
                                self.withdrawn,
                                self.notional_in,
                                nav_now,
                            ));
                            let entry_bar = l.node.start_bar;
                            let entry_basis = l.basis;
                            let is_short = l.direction == Polarity::Short;
                            let mut free = self.free;
                            let realized =
                                rec_reduce(&mut self.instances[k], l.units, &mut free, c);
                            self.free = free;
                            self.n_liquidations += 1;
                            self.account_reduce(l.direction, realized, c);
                            if is_short {
                                self.guards.on_short_pnl(k, realized); // prove_per_level_pnl（强平空头腿）
                            }
                            self.liq_log
                                .push((k, entry_bar, entry_basis, bar, c, is_short));
                        }
                    }
                }
            }
        }

        // ── A^NEST. 命题4 读法乙（旁路 route_bsp/cc 锚，大级别背驰段闸门 a0 区间套定位翻转）──
        //   enable_nest 时**取代** A''/A'/B（OFF 路径）：单一全仓位，在大级别背驰段武装下、由 a0 区间套
        //   定位到的 type1 点整仓翻转（买点 cover+做多 / 卖点 close+做空）。强平（块 A）仍保留（诚实会计）。
        if self.enable_nest {
            self.nest_step(view, bar, c);
            self.prove_tw_neutral(tw_pre, c);
            return;
        }

        // ── A''. 核心走势完成 → 主动清仓（campaign 边界重定义，546号死锁解锁；与 flat 对称）──
        //   缠论依据（fengkong/chanlun-trading-system 退出条件 = 买入程序判断条件被否定 / 走势终完美）：
        //   核心仓骑的走势在**该 level 顶/底背驰（type1）**完成 ⇒ 买入逻辑被否定 ⇒ 清仓到现金。
        //   enter 重建的**第三条路**，独立于 flip：clear_all → reset_campaign → highest_active None
        //   ⇒ 死锁（核心 units 几何衰减永不归零 → highest_active 恒 Some → enter 永不触发，546号）解除，
        //   **下一个买点**经核心级 enter 重建（不在本 bar 反向 enter ⇒ 避免 545 做空陷阱）。
        //   触发源 = type1 BSP（走势完成的可观测形式）⇒ guards 归因 Bsp。**不动 EPS、不动几何衰减**。
        if let (true, Some(cc)) = (self.enable_trend_done_clear, self.highest_active()) {
            let core_trend_done = match self.instances[cc].direction {
                Polarity::Long => cc < MAX_LEVEL && view.t1sell[cc], // 顶背驰：上涨核心走势终完美
                Polarity::Short => cc < MAX_LEVEL && view.t1buy[cc], // 底背驰：下跌核心走势终完美
            };
            if core_trend_done {
                self.guards.set_trigger(OpTrigger::Bsp); // 走势完成 = type1 BSP 驱动（合法触发源）
                self.clear_all(c);
                self.n_trend_done_clears += 1;
                // 全平到现金 ⇒ TW 中性（同价 c）。本 bar 不再 route_bsp（等下一买点 enter 重建）。
                self.prove_tw_neutral(tw_pre, c);
                return;
            }
        }

        // ── A'. 自下而上仓位涌现升级（route_bsp 前）──
        if let Some((target_level, target_dir)) = view.emergent_top {
            let node = view
                .nodes
                .get(target_level)
                .copied()
                .flatten()
                .unwrap_or_else(|| {
                    TrendNode::new(
                        bar,
                        bar,
                        c,
                        c,
                        match target_dir {
                            Polarity::Long => Direction::Up,
                            Polarity::Short => Direction::Down,
                        },
                    )
                });
            self.emergence_upgrade(target_level, target_dir, node);
        }

        // ── B. BSP 路由：top-down（高 level 先，区间套自上而下）──
        for j in (0..MAX_LEVEL).rev() {
            let b = view.buy[j];
            let s = view.sell[j];
            if b && s {
                continue; // 同 bar 同 level 买卖冲突 → 跳过
            }
            let node = view.nodes.get(j).copied().flatten().unwrap_or_else(|| {
                TrendNode::new(
                    bar,
                    bar,
                    c,
                    c,
                    if b { Direction::Up } else { Direction::Down },
                )
            });
            if b {
                self.route_bsp(j, true, node, c);
            } else if s {
                self.route_bsp(j, false, node, c);
            }
        }

        self.prove_tw_neutral(tw_pre, c);

        // ── prove_direction_matches_trend（观测）：核心方向应 = 最高走势类型方向 ──
        // emergent_top 是本 bar 最高 completed 走势方向；highest_active 是核心仓方向。emergence_upgrade
        // 的同向 gate 应使二者匹配——失配 = 核心被低级别 BSP flip / 逆涌现未升级（已知强牛违反）。
        if let Some((_, edir)) = view.emergent_top {
            let core_dir = self.highest_active().map(|cc| self.instances[cc].direction);
            self.guards.check_direction(core_dir, edir);
        }
    }

    /// 收尾（全平到现金，归还 withdrawn）。返回 final_nav (= free)。
    pub fn finish(&mut self, c: f64) -> f64 {
        if self.enable_reading_b_pair {
            self.finish_leg_pairs(c);
            return self.free;
        }
        if self.enable_reading_b {
            self.finish_legs(c);
            return self.free;
        }
        if c.is_finite() && c > 0.0 {
            self.guards.set_trigger(OpTrigger::Eod); // eod 是 clear_all 的合法非 BSP 触发源
            self.clear_all(c);
        }
        self.free
    }

    /// 读法B 每级别独立腿只读访问（诊断/L3）。
    pub fn leg(&self, k: usize) -> &Leg {
        &self.legs[k]
    }
}

#[cfg(test)]
mod t1_direction_gate_tests {
    //! **T1 方向错位入场门控单测（#164/R1）**——`long_entry_ok`/`short_entry_ok` 三模式分支 + 核心豁免。
    //! L0 结构判据（不依赖回测数据）：门控逻辑的存在性与对称性。L3 有效域（8 标的 nav/T1）见报告。
    use super::*;

    fn root(le: LongEntry) -> TRoot {
        let mut cfg = EngineConfig::off();
        cfg.enable_reading_b_pair = true;
        cfg.pair_long_entry = le;
        TRoot::new_with_config(100_000.0, cfg)
    }

    fn view_with_node(k: usize, dir: Direction) -> LevelView {
        let mut v = LevelView::default();
        v.nodes[k] = Some(TrendNode::new(0, 1, 1.0, 2.0, dir));
        v
    }

    #[test]
    fn any_模式恒放行_bit_exact基线() {
        let r = root(LongEntry::Any);
        // 任意级别/段方向，Any 恒返回 true（= 现状零门控，bit-exact）。
        assert!(r.long_entry_ok(0, &view_with_node(0, Direction::Down)));
        assert!(r.long_entry_ok(2, &view_with_node(2, Direction::Down)));
        assert!(r.short_entry_ok(0, &view_with_node(0, Direction::Up)));
    }

    #[test]
    fn segalign_次级别下落段拒开多() {
        let r = root(LongEntry::SegAlign);
        // 无核心（全空仓）+ 本级别下落段 ⇒ 拒（攻 T1：开多在跌段）。
        assert!(!r.long_entry_ok(0, &view_with_node(0, Direction::Down)));
        // 本级别上涨段 ⇒ 放行（顺向开多）。
        assert!(r.long_entry_ok(0, &view_with_node(0, Direction::Up)));
        // 对称：开空在涨段拒、跌段放行。
        assert!(!r.short_entry_ok(0, &view_with_node(0, Direction::Up)));
        assert!(r.short_entry_ok(0, &view_with_node(0, Direction::Down)));
    }

    #[test]
    fn segalign_核心级别豁免门控_保骑牛() {
        let mut r = root(LongEntry::SegAlign);
        // 造核心多腿 @ L2（highest_active_long==2）。
        r.leg_pairs[2].long_units = 10.0;
        assert_eq!(r.highest_active_long(), Some(2));
        // 核心级别 L2 即使段下落也豁免（核心骑牛不踏空，押小回不砍仓）。
        assert!(r.long_entry_ok(2, &view_with_node(2, Direction::Down)));
        // 核心之上 L3 同豁免。
        assert!(r.long_entry_ok(3, &view_with_node(3, Direction::Down)));
        // 次级别 L0 仍受门控（下落段拒，攻次级别 T1）。
        assert!(!r.long_entry_ok(0, &view_with_node(0, Direction::Down)));
    }

    #[test]
    fn segalign_无核心时首次建仓须放行() {
        let r = root(LongEntry::SegAlign);
        assert_eq!(r.highest_active_long(), None);
        // 全空仓 + 上涨段 ⇒ 放行（首次建仓不被豁免逻辑误拒）。
        assert!(r.long_entry_ok(0, &view_with_node(0, Direction::Up)));
    }

    #[test]
    fn crosslevel_更高级别下落段拒次级别开多() {
        let r = root(LongEntry::CrossLevel);
        // 次级别 k=0 上涨段，但更高 j=2 为下落段 ⇒ 拒（攻 65.5% xlc：次级别逆主级别走势开多）。
        let mut v = view_with_node(0, Direction::Up);
        v.nodes[2] = Some(TrendNode::new(0, 1, 1.0, 2.0, Direction::Down));
        assert!(!r.long_entry_ok(0, &v));
        // 无更高级别下落段 ⇒ 放行。
        assert!(r.long_entry_ok(0, &view_with_node(0, Direction::Up)));
    }
}

#[cfg(test)]
mod orbit9_add_dispatch_tests {
    //! **9 轨道操作分派 + O3 add 单测（task#40 B，#39 O1-O9）**——L0 结构判据（不依赖回测数据）：
    //! ① add 加仓不污染 long（rec_add 层内单一方向守卫）② O3 τ 镜像（多头买回 ↔ 空头卖回对称）
    //! ③ OFF bit-exact（T_ORBIT9_DISPATCH OFF ⇒ route_bsp 旧二元，add 不激活）。
    use super::*;

    fn node(dir: Direction) -> TrendNode {
        TrendNode::new(0, 1, 1.0, 2.0, dir)
    }

    /// 造一个 instances 路径 root（off 基线），父级核心 @ parent + 已 sink 出 sub 短差。
    /// 直接调 enter/sink/add 须先 set_trigger(Bsp)（prove_bsp_triggers_operation 守卫：操作必有触发源）。
    fn root_with_sunk_short(pdir: Polarity, parent: usize, sub: usize) -> TRoot {
        let mut r = TRoot::new_with_config(100_000.0, EngineConfig::off());
        r.guards.set_trigger(OpTrigger::Bsp);
        let c = 10.0;
        // 父级建核心仓（pdir 方向）。
        r.enter(
            parent,
            pdir,
            node(if pdir == Polarity::Long {
                Direction::Up
            } else {
                Direction::Down
            }),
            c,
        );
        // sink：父减 1/3 + sub 开反父向短差。
        r.sink(
            parent,
            sub,
            node(if pdir == Polarity::Long {
                Direction::Down
            } else {
                Direction::Up
            }),
            c * 1.1,
        );
        r
    }

    #[test]
    fn add_部分买回_不污染父向极性() {
        // 父多 @ L2，sink 出 L0 空头短差。
        let mut r = root_with_sunk_short(Polarity::Long, 2, 0);
        let parent_dir_pre = r.instances[2].direction;
        let parent_u_pre = r.instances[2].units;
        let sub_u_pre = r.instances[0].units;
        assert_eq!(r.instances[0].direction, Polarity::Short, "sink 出空头短差");
        // O3 add：部分买回 m=quota(sub) 升回父多。
        r.add(2, 0, 11.5);
        // 父向极性不变（仍 Long，不被污染）。
        assert_eq!(r.instances[2].direction, parent_dir_pre);
        assert_eq!(r.instances[2].direction, Polarity::Long);
        // 父 units 增（买回一段），sub units 减（部分平短差，非整条）。
        assert!(r.instances[2].units > parent_u_pre, "父 units 应增（买回）");
        assert!(r.instances[0].units < sub_u_pre, "sub 短差应部分减");
        assert!(
            r.instances[0].units > 1e-9,
            "部分买回非整条 ⇒ sub 短差未清空"
        );
        assert_eq!(r.n_adds, 1);
    }

    #[test]
    fn add_short_τ镜像_父空卖回对称() {
        // τ 镜像：父空 @ L2，sink 出 L0 多头短差，add 卖回。
        let mut r = root_with_sunk_short(Polarity::Short, 2, 0);
        assert_eq!(
            r.instances[0].direction,
            Polarity::Long,
            "父空 sink 出多头短差"
        );
        let parent_u_pre = r.instances[2].units;
        let sub_u_pre = r.instances[0].units;
        // O3 add_short：部分卖回升回父空（同一 add 方法，pdir 参数化 ⇒ τ 镜像无 if 多空）。
        r.add(2, 0, 9.0);
        // 父向极性不变（仍 Short）。
        assert_eq!(r.instances[2].direction, Polarity::Short);
        // 对称：父 units 增（卖回），sub 短差部分减。
        assert!(
            r.instances[2].units > parent_u_pre,
            "父空 units 应增（卖回）"
        );
        assert!(
            r.instances[0].units < sub_u_pre && r.instances[0].units > 1e-9,
            "sub 短差部分减非整条"
        );
        assert_eq!(r.n_adds, 1);
    }

    #[test]
    fn add_无短差腿_noop_不pyramid() {
        // 父多 @ L2，sub L0 无短差（未 sink）⇒ add no-op（不凭空 pyramid）。
        let mut r = TRoot::new_with_config(100_000.0, EngineConfig::off());
        r.guards.set_trigger(OpTrigger::Bsp);
        r.enter(2, Polarity::Long, node(Direction::Up), 10.0);
        let u_pre = r.instances[2].units;
        r.add(2, 0, 11.0);
        assert_eq!(
            r.instances[2].units, u_pre,
            "无短差腿 ⇒ add no-op，父 units 不变"
        );
        assert_eq!(r.n_adds, 0);
    }

    #[test]
    fn off_bit_exact_orbit9未激活_全走recover() {
        // T_ORBIT9_DISPATCH OFF ⇒ route_bsp 同父向持短差全走 recover（整条），add 不激活。
        let mut r = root_with_sunk_short(Polarity::Long, 2, 0);
        assert!(!r.enable_orbit9_dispatch, "off() ⇒ orbit9 OFF");
        // 同父向 BSP（父多 + 买点）@ sub L0 ⇒ recover（整条升回），非 add。
        let c = 11.0;
        r.route_bsp(0, true, node(Direction::Up), c);
        // OFF ⇒ 走 recover（sub 短差整条清空），n_adds 恒 0。
        assert_eq!(r.n_adds, 0, "OFF ⇒ add 分支未激活（bit-exact）");
        assert_eq!(
            r.instances[0].units, 0.0,
            "OFF recover 平整条 ⇒ sub 短差清空"
        );
        assert_eq!(r.n_recovers, 1);
    }

    #[test]
    fn on_orbit9_回调未完成_走add部分买回() {
        // T_ORBIT9_DISPATCH ON + orbit9_sub_trend_done=false（回调未完成）⇒ route_bsp 同父向走 add（部分）。
        // 注：orbit9_sub_trend_done 占位 fallback=true ⇒ 现状仍走 recover；本测直接验证 add 原语接入正确性，
        //     ON 且判定未完成的整合路径由 C 接口替换占位后激活（见报告 §四接口需求）。
        let mut cfg = EngineConfig::off();
        cfg.enable_orbit9_dispatch = true;
        let mut r = TRoot::new_with_config(100_000.0, cfg);
        r.guards.set_trigger(OpTrigger::Bsp);
        let c = 10.0;
        r.enter(2, Polarity::Long, node(Direction::Up), c);
        r.sink(2, 0, node(Direction::Down), c * 1.1);
        assert!(r.enable_orbit9_dispatch);
        // 占位 fallback=true ⇒ 走势完成 ⇒ recover（与 OFF 同，保未整合 bit-exact）。
        r.route_bsp(0, true, node(Direction::Up), c);
        assert_eq!(
            r.n_adds, 0,
            "占位 fallback=true ⇒ 仍 recover（C 接口未就位 bit-exact）"
        );
        assert_eq!(r.n_recovers, 1);
    }
}

#[cfg(test)]
mod intrinsic_quota_tests {
    //! **逐级内在配额单测（task#84 子5，split 自适应定理 + OFF bit-exact）**——L0 结构判据：
    //! ① mobile_frac 纯函数：深回调+快确认→大配额 / 浅回调或慢确认→小配额，∈(0,1]（split 自适应）；
    //! ② ON sink 配额逐级自适应（深 vs 浅触发腿 ⇒ m 不同，证非固定 1/3）；
    //! ③ OFF bit-exact（T_INTRINSIC_QUOTA OFF ⇒ sink/drain/add 走 σ-不变 1/3 + prove_sigma_quota，逐字不变）。
    use super::*;

    fn node(dir: Direction) -> TrendNode {
        TrendNode::new(0, 1, 1.0, 2.0, dir)
    }

    /// ① mobile_frac 纯函数：split 自适应定理的档位映射 + 边界。
    #[test]
    fn mobile_frac_split_自适应档位() {
        // 深回调(T1) ∧ 快确认(T1) ⇒ 最大配额（机动捕获跌幅骑主升浪）= 1/3×2×3/2 = 1.0（整仓裁剪上界）。
        let deep_fast = mobile_frac(ConfDepth::T1, ConfDepth::T1);
        // 浅回调(T3) ∧ 慢确认(T2) ⇒ 最小配额（核心 full 防踏空）= 1/3×0.5×0.5 = 1/12。
        let shallow_slow = mobile_frac(ConfDepth::T3, ConfDepth::T2);
        // 中性(T2,T1)：1/3×1×3/2 = 0.5。
        let mid = mobile_frac(ConfDepth::T2, ConfDepth::T1);

        assert!(
            (deep_fast - 1.0).abs() < 1e-12,
            "深回调+快确认=最大配额(裁剪到整仓 1.0)，得 {deep_fast}"
        );
        assert!(
            (shallow_slow - 1.0 / 12.0).abs() < 1e-12,
            "浅回调+慢确认=最小配额 1/12，得 {shallow_slow}"
        );
        assert!(
            deep_fast > mid && mid > shallow_slow,
            "split 自适应单调：深快 > 中 > 浅慢"
        );
        // 边界：所有 9 组合 ∈ (0,1]（配额合法）。
        for lp in [ConfDepth::T1, ConfDepth::T2, ConfDepth::T3] {
            for lc in [ConfDepth::T1, ConfDepth::T2, ConfDepth::T3] {
                let f = mobile_frac(lp, lc);
                assert!(
                    f > 0.0 && f <= 1.0,
                    "mobile_frac({lp:?},{lc:?})={f} 须 ∈(0,1]"
                );
            }
        }
        // 与固定 σ-不变 1/3 的对照：深确认放大、浅确认缩小（拍扁的反面）。
        assert!(
            deep_fast > MOBILE_FRAC,
            "深确认配额 > 固定 1/3（自适应放大）"
        );
        assert!(
            shallow_slow < MOBILE_FRAC,
            "浅确认配额 < 固定 1/3（自适应缩小防踏空）"
        );
    }

    /// 造一个 ON 路径 root，父核心 @parent，注入触发腿 ConfDepth，sink 出 sub 短差，返回 sub 短差 units。
    fn on_sink_sub_units(l_pullback: ConfDepth, l_confirm: ConfDepth) -> f64 {
        let mut r = TRoot::new_with_config(100_000.0, EngineConfig::intrinsic_quota());
        r.guards.set_trigger(OpTrigger::Bsp);
        let c = 10.0;
        r.enter(2, Polarity::Long, node(Direction::Up), c);
        // 注入触发腿（sub=0）的逐级 L_pullback/L_confirm（模拟 on_bar 注入；sink 据此算 mobile_frac）。
        r.cur_l_pullback[0] = l_pullback;
        r.cur_l_confirm[0] = l_confirm;
        r.sink(2, 0, node(Direction::Down), c * 1.1);
        r.instances[0].units
    }

    /// ② ON sink 配额逐级自适应：深触发腿 ⇒ 大配额（sub 短差 units 大）vs 浅触发腿 ⇒ 小配额。
    #[test]
    fn on_sink_配额逐级自适应_非固定() {
        let deep = on_sink_sub_units(ConfDepth::T1, ConfDepth::T1); // 深回调+快确认=大配额
        let shallow = on_sink_sub_units(ConfDepth::T3, ConfDepth::T2); // 浅回调+慢确认=小配额
        assert!(
            deep > shallow * 1.5,
            "深确认 sink 配额应显著大于浅确认（自适应非固定 1/3），deep={deep} shallow={shallow}"
        );
        assert!(deep > 1e-9 && shallow > 1e-9, "两档配额皆 >0（开了短差腿）");
    }

    /// ③ OFF bit-exact：T_INTRINSIC_QUOTA OFF ⇒ sink 配额 = σ-不变 quota(u_p)=u_p/3（与 off() 逐字一致）+
    ///    prove_sigma_quota 守卫生效（不跳过）。注入 ConfDepth 也不影响 OFF（新字段无 OFF 消费者）。
    #[test]
    fn off_bit_exact_sink_配额恒1_3() {
        // OFF 基线（不开 intrinsic）：sink 配额 = u_p/3。
        let mut r_off = TRoot::new_with_config(100_000.0, EngineConfig::off());
        r_off.guards.set_trigger(OpTrigger::Bsp);
        let c = 10.0;
        r_off.enter(2, Polarity::Long, node(Direction::Up), c);
        // 即便“污染”ConfDepth 字段，OFF 路径也不读 ⇒ 配额恒 1/3（证新字段无 OFF 消费者）。
        r_off.cur_l_pullback[0] = ConfDepth::T1;
        r_off.cur_l_confirm[0] = ConfDepth::T1;
        let u_p = r_off.instances[2].units;
        r_off.sink(2, 0, node(Direction::Down), c * 1.1);
        // sub 短差 units = m×pb/c（同资本 sizing）；m=u_p/3。验证 OFF 配额 = σ-不变 1/3。
        // 用 add 视角更直接：父减仓量 = u_p − 剩余 = u_p/3。
        let parent_reduced = u_p - r_off.instances[2].units;
        assert!((parent_reduced - u_p * MOBILE_FRAC).abs() < 1e-6 * u_p,
            "OFF sink 父减仓 = u_p×1/3=σ-不变（ConfDepth 污染无效），减 {parent_reduced} vs 期望 {}", u_p * MOBILE_FRAC);
        assert!(!r_off.enable_intrinsic_quota, "off() ⇒ intrinsic OFF");
    }

    /// ③bis OFF prove_sigma_quota 仍守（反证非平凡）：OFF 路径若配额 ≠ 1/3 必 panic（守卫未被误跳过）。
    /// 这里直接验证 OFF 走 quota()（= prove_sigma_quota 的 canonical），ON 才偏离——通过对比父减仓量。
    #[test]
    fn off_vs_on_父减仓量对照() {
        let c = 10.0;
        // OFF：父减 u_p/3。
        let mut r_off = TRoot::new_with_config(100_000.0, EngineConfig::off());
        r_off.guards.set_trigger(OpTrigger::Bsp);
        r_off.enter(2, Polarity::Long, node(Direction::Up), c);
        let u_off = r_off.instances[2].units;
        r_off.sink(2, 0, node(Direction::Down), c * 1.1);
        let reduced_off = u_off - r_off.instances[2].units;
        // ON 深确认：父减 u_p×mobile_frac(T1,T1)=u_p×1.0=整仓（> u_p/3）。
        let mut r_on = TRoot::new_with_config(100_000.0, EngineConfig::intrinsic_quota());
        r_on.guards.set_trigger(OpTrigger::Bsp);
        r_on.enter(2, Polarity::Long, node(Direction::Up), c);
        r_on.cur_l_pullback[0] = ConfDepth::T1;
        r_on.cur_l_confirm[0] = ConfDepth::T1;
        let u_on = r_on.instances[2].units;
        r_on.sink(2, 0, node(Direction::Down), c * 1.1);
        let reduced_on = u_on - r_on.instances[2].units;
        assert!(
            reduced_on > reduced_off * 2.0,
            "ON 深确认配额(整仓)应远大于 OFF 固定 1/3：on={reduced_on} off={reduced_off}"
        );
    }
}

// ════════════════════════════ λ 源同一性守卫（#943 AC-3）════════════════════════════

/// **`f = 1/λ` 单源守卫（票 #943 AC-3）**。
///
/// **守什么**：`rec_engine` 侧算配额的常数（本模块 `MOBILE_FRAC`，`quota()` 消费）与
/// `prove_guards::prove_sigma_quota` 侧重算 canonical 的常数（`fugue_v3::MOBILE_FRAC`）
/// **必须同源**。二者曾是两份独立字面量（`rec_engine.rs:69` 私有 `1.0 / 3.0` 遮蔽
/// `fugue_v3::MOBILE_FRAC`，#925 勘察查实）——**只改 `fugue_v3::LAMBDA` 一处，NT 生产策略
/// 每次 sink/drain 直接 panic**（`rec_t_strategy.py` → `RecTStream` → `rec_engine`）。
///
/// **非重言**：比的是**两个来源的位模式**，并把引擎侧实算配额喂给守卫重算。若日后有人在本
/// 模块重新引入私有 `MOBILE_FRAC` 字面量、或两侧 `LAMBDA` 被改到不同值，① 的 `to_bits`
/// 与 ② 的 `prove_sigma_quota` 双双 fire，而不是静默劈叉到生产 panic。
#[cfg(test)]
mod lambda_single_source_tests {
    use super::{quota, MOBILE_FRAC};
    use crate::fugue_v3::MOBILE_FRAC as FUGUE_V3_MOBILE_FRAC;
    use crate::recursive_t::prove_guards::prove_sigma_quota;

    #[test]
    fn rec_quota_source_is_fugue_v3_mobile_frac() {
        // ① 源同一性（位模式相等 ⇒ 排除「值近似但写法各自漂移」）。
        assert_eq!(
            MOBILE_FRAC.to_bits(),
            FUGUE_V3_MOBILE_FRAC.to_bits(),
            "rec_engine MOBILE_FRAC={MOBILE_FRAC} ≠ fugue_v3::MOBILE_FRAC={FUGUE_V3_MOBILE_FRAC}\
             ——守卫 prove_sigma_quota 用后者重算 canonical，劈叉 ⇒ NT 生产 sink/drain 必 panic（#943）"
        );
        // ② 端到端：引擎侧实算配额喂给守卫（守卫内部用 fugue_v3 常数重算 canonical）。
        for &u in &[1.0_f64, 30.0, 1_000_000.0] {
            prove_sigma_quota(quota(u), u, 4, 0);
        }
    }
}
