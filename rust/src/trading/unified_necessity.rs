//! unified_necessity — 统一必然性引擎（mode = "unn"）。
//!
//! 设计源头：编排者 2026-06-14"你要严格验证必然性，这是通过推论证明的……
//! 然后严格实装，绝不允许近似"。把分散在 4 个引擎里的 8 条必然性
//! （全部来自 `docs/concept_movement_chain.md` 概念运动链）**叠加到一个引擎**——
//! 不是 patch，是从概念运动链推导出的完整操作语义层。
//!
//! ## 8 条必然性 → 环 → 本引擎实装
//!
//! | 必然性 | 环 | 实装 | prove |
//! |--------|----|------|-------|
//! | N1 逐仓独立森林 | 20 嵌套递归=voice 自相似 | `VoiceLedger` 森林（root 可多 child） | `prove_n1_forest` |
//! | N2 per-voice 独立操作 | 18 并发=级别同时性 | `acted_bar` 逐 voice（去全局互斥） | `prove_n2_per_voice` |
//! | N3 全三类 BSP 消费 | 12 三类买卖点=中枢生命周期 | `e.class.side()` 归侧（无 Sell2/Buy2 continue） | `prove_n3_type2` |
//! | N4 成本门终止递归 | 16 势的幅度<成本→势消失 | 纯 θ<friction ∨ θ=None（无 floor 参数） | `prove_n4_cost_gate` |
//! | N5 区间套自上而下定位 | 14 高级别 BSP 由低级别定位 | `cascade_arm` 级联（高 source 主导） | `prove_chain`/`prove_n5` |
//! | N6 先势后定位时序 | 540 压缩→展开 | `Pending.since_bar` + compress≤confirm≤bar | `prove_chain`（时序） |
//! | N7 降成本不需 pending | 17 低级别走势完美的利用 | E 用 voice **自层** nf（非全局链，不 prove_chain） | `prove_n7_spawn_self_level` |
//! | N8 双层会计多空嵌套 | 22 方向交替递归 | `close_voice`（pop_tail 森林形式）+ 守恒 | `prove_n8_conservation` |
//!
//! ## 单一引擎为何不够（验证结论，逐条推论证明）
//!
//! - **URS**：栈（违 N1）、全局 acted（违 N2）、Sell2 continue（违 N3）、固定 floor（弱 N4）、
//!   located 自下而上（违 N5/N6）。满足 N7（E 用自层 nf）、N8。
//! - **iso**：森林 ✓N1、per-voice ✓N2、type2 ✓N3、E 自层 nf ✓N7、森林会计 ✓N8。
//!   但无 pending_locate（located 自下而上 per-level）⇒ 违 N5/N6；有 floor ⇒ 弱 N4。
//! - **nif**：min_trade_ladder 固定层 floor ⇒ 违 N4（最严重）。
//! - **pcf**：pending_locate 级联 ✓N5/N6、纯 source 选层 ✓N4（segment 非势源 + θ 门）。
//!   但栈（违 N1）、全局 acted（违 N2）、Sell2 continue（违 N3）、**E 被 prove_chain
//!   门控**（违 N7——降成本要求完整 located 链才能 spawn）。
//!
//! ## 综合：同一区间套 confirm fire 两路消费（540号"同一递归两遍历"的操作化）
//!
//! 关键张力：N5（级联 located 自上而下）与 N7（E 不被 pending 门控）不可同时由
//! **一条** located 链满足——若 E 用级联链 source 即违 N7（pcf 的死因）。严格解 =
//! **同一 confirm@k fire 分两路**：
//!   - **级联 located 链**（N5/N6）：confirm → `cascade_arm` → `located_*`，
//!     `chain_source` 顶 = 势源。**仅** 根 F（入场）/ C（清仓/翻转）消费（pending_locate）。
//!   - **逐层 nf fire**（N7）：`nf_sell[k]/nf_buy[k]` = confirm@k 的本层极值。
//!     **任意** voice 的 E（降成本）直接消费自层 fire（不查全局链、不 prove_chain）。
//!
//! C/E 消歧（§6/§9 缠论原文，非工程约定）：C = type1 背驰@源层（走势完美，§6"十年
//! 1-2 次"）；E = 其余卖点（type2/3 或中枢内 candidate，§9"绝大多数卖点只是降成本"）。
//! 同一 confirm@k 由 `sig.sell1[k]` 区分 ⇒ 无死锁（type2/3 常见 ⇒ E 常见；type1+完整
//! 级联罕见 ⇒ C 罕见 ⇒ 强趋势不踏空）。
//!
//! ## 级别语义（势源 vs 降成本目标分离——N4/N5 的精确边界）
//!
//! - **pending 势源**（N5/N6）：仅 `k ≥ PENDING_LO = FIRST_BSP_LADDER+1 = move(L1)`
//!   注册 pending（segment 非势源——`project_pcf_pending_locate_collapse_fix`：segment
//!   抢先武装 ⇒ source 坍缩 85-91%；segment 仅作 `rec_sub_evidence` 的 confirm 证据）。
//! - **降成本 spawn 目标**（N4）：任意 `theta(k) ≥ friction` 的层；递归基 = bi（a0）
//!   层 `theta(bi)=None`（`depth_ref` 只观测 `[FIRST_BSP_LADDER, MAX)`）⇒ 成本门**自然**
//!   终止于 segment，**无需 floor 参数**。segment@2 是合法降成本目标（势可测）。
//!
//! ## 每 bar 优先序（同 bar；A/C→F；§1-§8 会计 = 森林 close_voice 复用，bit-exact iso）
//!
//! A 强平兜底（逐空头 voice，会计终局非走势操作）→ C 根 type1 平仓/翻转（同级别卖点）→
//! D 回补（子 voice 子级别买点）→ E 降成本 spawn（持仓期间次级别卖点）→ F 根入场（买点）。
//! **没有 B（否定扫描）**——见下"否定线删除"。
//!
//! ## 否定线删除（编排者 2026-06-15："操作只在买卖点，否定线是经验性补丁"）
//!
//! **第11环（操作只在买卖点）是硬约束**。否定线（027:25 negate_line）作为 voice 的**操作
//! 触发器**（旧 B 规则：破否定线 ⇒ 清算/翻转/对冲）在买卖点之外凭空加了操作条件——**违反
//! 第11环**。否定线不是从概念运动链推出的必然概念，是一个经验性补丁。故**直接删除**：删整个
//! B 规则 + voice 不再携带操作用 negate_line（unn 全置 None；字段保留仅因 iso 引擎复用）。
//!
//! 删除史（同一补丁的三次试探全部否定）：df752ea6e4 破否定线 ⇒ in-place 翻空（whipsaw 破产
//! CL−56.7%）→ 6faf4ec45a 破否定线 ⇒ 回现金 + 观测态（踏空 P1=0/8）→ c6deae8780"破否定线 ⇒
//! 降成本对冲"仍是在补丁上加补丁（保留了否定线这个非必然触发器）。**根治 = 删触发器本身**。
//!
//! 注：027:25 否定线在 **candidate/located 维护**（`nest_*`/`located_*` 破极值清窗）中保留——
//! 那是**买卖点检测**（势源是否仍有效：被否定的 candidate 不是合法 located 势源，第14环区间套
//! 确认的一部分），**不是操作触发器**。检测买卖点（含 027:25 否定 candidate）是必然基础设施；
//! 用 027:25 触发 voice 操作才是补丁。区别 = 检测（这是不是买卖点/势源？）vs 操作（对 voice 做事）。
//!
//! ## voice 生命周期：完全由买卖点驱动（四操作 + 一会计终局）
//!
//! | 操作 | 触发（买卖点） | 必然性环 |
//! |------|---------------|---------|
//! | F 建仓 | 买点（located_buy 链顶 source ∧ buy1 type1 底背驰） | 第11环 + 第23环（恒仓满仓） |
//! | E 降成本 spawn | 持仓期间次级别卖点（nf_sell@ladder 背驰确认 ∨ 根 confirmed 卖） | 第17环 + 第22环 |
//! | C 平仓/翻转 | 同级别 type1 卖点（sell1@source ∧ located 链 N5/N6） | 第11环 + 第21环 + 第14环 |
//! | D 回补 | 子 voice 子级别买点（buy_any@ladder 走势完美） | 第2环 + 第11环 |
//! | A 强平 | 1x 逐仓 capital 耗尽（c≥2×basis，会计终局非走势操作；市场被动机制） | N8/第22环（1x 逐仓有界亏损） |
//!
//! 只有这五种事件改变 voice 状态。没有否定线。没有观测态。每个 voice 从买点诞生（F/E spawn），
//! 在买卖点操作（C/D/E），由买卖点或 1x 逐仓会计终局（A）离场。**E/D 的次级别/子级别卖买点经
//! `rec_sub_evidence` 背驰结构确认**（非价格穿越）——这是第11环"买卖点"的严格形式。
//!
//! ## T1（不主动清仓，第23环恒仓）：引擎只在买卖点主动操作，市场强平是被动例外
//!
//! **编排者裁决 2026-06-15**："恒仓"约束**引擎的主动行为**（不主动清仓，只在买卖点操作
//! F/E/C/D），**不约束市场的强制平仓**（A 强平是市场机制，被动；引擎不可否认）。删否定线 +
//! 观测态后，引擎**无主动清仓路径**——C 在 type1 卖点 **in-place 翻转**（不清仓，root_ladder>
//! FIRST_BSP；clear-at-base 不可达已升 `unreachable!()`），E/D 降成本/回补只在子树内流转不动
//! 根，F 只建仓。故森林"非空→空"**只能经 A 强平根空头**（c≥2×basis 会计终局，被动）。
//! `prove_t1_no_voluntary_exit` 守卫：森林非空→空 ⟹ root_liquidated（非强平致空仓 = 引擎主动
//! 清仓 = 否定线/观测态残留 = panic）。**强平→空仓→下一个 BSP→F 重建仓**（从最高可介入级别，
//! N5-gated），空仓期间是被动等待（市场强制），非主动选择。编排者裁决（恒仓⊥1x逐仓强平矛盾）：
//! 强平不是引擎操作 ⇒ 不违反"操作只在买卖点" ⇒ 矛盾消解（恒仓 = 引擎不主动清仓，非市场永不强平）。
//!
//! T14/T5/A5（双向翻转 + 涌现 + 会计重组）：C 规则 type1 背驰根 in-place 长↔空翻转。根空头用
//! MtM nav（capital−units×c，外部市场负债）⇒ 翻转/平仓同价 c 守恒且真实兑现。**根空头是叶
//! 节点**（T8：MtM⊥子空头 frozen，短父 spawn 长子破守恒 +m×c）⇒ 根空头不嵌套降成本（E 跳过；
//! 有效域边界：降成本只在多头相/子空头层）。
//!
//! ## 验收标准：8 个 prove 函数（非回测）
//!
//! 每条必然性有对应 prove，violation = panic。8 标的真实数据跑通无 panic ⇒ 8 条
//! 必然性在 ~25M bar 上 L2 成立。回测是有效域读数，**不是验收标准**（编排者裁决）。
//!
//! ## 螺旋覆盖空间 + 递归嵌套多重赋格（统一框架，`docs/necessity_derivation.md` §8）
//!
//! **两框架同构**（编排者 2026-06-15）：螺旋每一圈 = 赋格一个 voice（级别）；**嵌套** = 径向
//! spawn 子 voice（外圈→内圈，`try_spawn_cost_gated`）；**递归** = 覆盖映射 σ（同一 step 逻辑
//! 复制每层，R₃）；**多重** = 森林并发（N1 多 voice 同时 active，`prove_n1_forest`）。螺旋三
//! 坐标：角向 φ（唯一奇点 φ=0=背驰∧手性翻转）/ 径向 r=λ^k（级别=圈数=赋格声部）/ 手性 ε
//! （多/空，τ 对合）。群 G_spiral=D∞=⟨h,τ|τ²=e,τhτ⁻¹=h⁻¹⟩（半直积**编码多空不对称**）。在
//! A₀–A₄ 上追加建模公理 A₅（尺度比 λ 恒定，L2 可证伪），R₃ 自相似实体化为对数螺旋。
//!
//! - **T₂ 走势完美三坐标合一**：φ→0(背驰 candidate)∧settle(径向中枢闭合=内圈 type1 已 settle
//!   母线贯通)∧ε(方向)**三坐标同时** = confirm fire 的**定义**（`confirmed = since<bar ∧ helix`，
//!   helix 母线贯通 ⟹ 内圈同侧 type1 历史非空）。三坐标合一由 confirm 构造**定义层保证**——
//!   不设运行时 prove（旧 `prove_t2_perfection_triaxis` 的三断言全被 fire 门 `since<bar∧helix`
//!   逻辑蕴含 = 重言，永不 fire，已删）。单独 settle≠完美（`project_ph_settle_usage_boundary`）。
//!   **赋格**：confirm = 外圈 candidate 声部与内圈已 settle type1 声部在 φ=0 母线**相位共现**（stretto）。
//! - **T₄/T₁₀ 走势完全分类无第四类**：判别量 = 径向圈数 = 中枢数量（趋势≥2/盘整1，赋格声部
//!   跨圈数）。分类完备由 `BspKind` enum（Type1/2/3 三变体）**编译期穷尽**保证 = 定义层 ✓——不设
//!   运行时 prove（旧 `prove_t4_classification` 的 `matches!(kind, Type1|Type2|Type3)` 对三变体
//!   enum 永真 = 重言，已删）。type2/3 中枢锚（径向圈数判别量前提）由 `prove_s12_center`（消费侧，
//!   line 804）+ 生成侧 `buysellpoints_from_level` 守卫。趋向维(斜率)DivEvent 无段端点**不可达**
//!   （T₁₁ 投影缺口的代码证据）——不强加伪判据。
//! - **T₄₇ 角向唯一奇点 → 无第四类 BSP**：type1/2/3 是同一 φ=0 的径向投影深度（`BspKind` enum
//!   三变体编译期穷尽 = 定义层无 type4）。✓。
//! - **T₄₈ 径向 σ-不变 → 守恒律建于股数**：`prove_n8` 守 Σunits（σ-不变）+ NAV 中性（径向冻结）。✓。
//! - **T₄₉ confirm 向心回溯**：`helix_centripetal_confirm` 沿 φ=0 母线**向心回溯已 settle 内圈
//!   type1**（t_j<t_K 过去），非前向等待（读法A 几何错误，`project_recl2_confirm_breakpoint_temporal`
//!   418/418 时序错配）。母线逐圈贯通（嵌套：末段 sub-component 在外层之内）⇒ confirm。✓（向心；
//!   ES 1s 11.77M bar fire 62→377）。
//! - **T₅₀ 操作频率径向标度律 f(k)∝λ^{−k}**：清仓(最外圈 K)∝λ^{−K} 罕见 / 降成本(内圈)∝λ^{−k}
//!   频繁。定性单调核 A₅-独立（`prove_t50_radial_scaling` eod 观测），λ 指数律 L2 可证伪——~
//!   （非 panic，formalization-validity-domain.md：不把 L2 定量律误作 L0 不变量）。
//! - **T₅₁ 手性 τ：T₂₄/T₃₆/T₄₁/T₄₆ 干净 ✓（无条件）**（编排者 2026-06-15 裁决）：手性 ε 在
//!   每个 φ=0 **时序翻转**（located 流交替，`prove_s11` 守卫）= T14 根 in-place 长↔空翻转（已
//!   实装，本文件 C 规则）+ 赋格子声部方向交替（`child_dir=父反向`）。多空对称 = 时序分段交替，
//!   这是对称的**正确形式**（由 T14 实装），不是"无全局连续截面的残余"——时序翻转就是对称本身。故：
//!   - **T₂₄ 多空对称 ✓**（`prove_t14_root_flip` 守 M=N 翻转 + S11 located 交替；T14 根翻空 +
//!     赋格子声部双向已实装）。
//!   - **T₃₆ earning 构造不对称 ✓**：不对称是**定理内容**——多头 earning 可构造（A4 守卫 dq>0）+
//!     空头"挣负股数"L0 不可构造（N8 守恒保护，代码正确地不给空头增 units）。正确实装，非缺陷。
//!   - **T₄₁/T₄₆ 零强平/零破产 ✓（无条件 L0）**：A₀ 走势终完美（公理）⟹ 空头押注的下跌走势
//!     **必然完美**（底背驰买点）⟹ T49 向心回溯在正确 φ=0 时点 confirm ⟹ C 在买点 in-place
//!     翻多消费空头（C 规则 Polarity::Short 分支，`prove_chain` "C-flipback"）。空头**必然先在
//!     买点被 C 消费**——这是 A₀ 的逻辑推论（L0，不依赖 regime），不是"regime 依赖的有效域边界"。
//!     强平兜底 A（c≥2×basis）保留为纯数值终局（浮点/边界情况），概念上不可达（C 必先消费）。
//!   注：`debug_assert_eq!(dir, Long)` 仅在 nested_fugue v4 旧引擎（根恒多）；**unn 的 close_voice
//!   支持空头根 MtM**（root 经 T14 可翻空），无此 assert。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::isolated_fugue::{close_voice, nav, settle, VoiceLedger, VoiceStatus};
use super::nested_fugue::rec_sub_evidence;
use super::positional::{theta_weights, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::{BarSig, SignalTape};
use super::types::{BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER};
use crate::buysellpoint::{
    prove_s11_s9_located, prove_s12_center, BspKind, SigLocatedState, Side,
};
use crate::stroke::Direction;

/// pending 势源下界（N5/N6）：move(L1)。segment（=FIRST_BSP_LADDER）非势源——
/// 仅作 `rec_sub_evidence` 的低级别 confirm 证据。
const PENDING_LO: usize = FIRST_BSP_LADDER + 1;

/// pending 窗口（高级别 candidate 持续记忆——540号压缩侧↑载体）。`since_bar` =
/// candidate 首现 bar（压缩完成 = φ→0 的外圈 (λ^k, φ→0)）；极值刷新时保留首现值
/// （压缩起始不变）。
/// **T49（confirm 向心回溯，§8.1 读法C）**：candidate@k 出现后**不再前向逐 bar 累积
/// frontier**，而是当下沿 φ=0 母线**向心回溯已 settle 的内圈 type1**
/// （`helix_centripetal_confirm`）——内圈 type1 必在 candidate 之过去（展开恒等式
/// Δt∝λʲ⁻¹(λ−1)>0 ⟹ t_j<t_K），故 confirm 无 frontier 跨 bar 状态。前向窗口（旧
/// frontier 累积）是几何错误（读法A，`project_recl2_confirm_breakpoint_temporal`
/// 418/418 时序错配：内圈末段 type1 平均 13694 bar 前，前向够不着）。
#[derive(Debug, Clone, Copy)]
struct Pending {
    extreme: f64,
    since_bar: i64,
}

/// pending confirm 兑现条目（N5/N6 的载体；级联后一条链内全层 source_ladder 统一）。
#[derive(Debug, Clone, Copy)]
struct PendingLocate {
    extreme: f64,
    source_ladder: usize,
    direction: Side,
    /// 压缩↑完成 bar（candidate 首现）。
    compress_bar: i64,
    /// 展开↓兑现 bar（低级别 confirm）。540号：compress_bar ≤ confirm_bar。
    confirm_bar: i64,
}

/// 级联武装（N5 第14环严格形式）：confirm@source ⇒ 武装 `located[FIRST_BSP..=source]`
/// 全层，统一极值 = 源层 027:25 否定线、统一 source_ladder=source、统一 direction。
/// 高 source 优先（既有 source 更高则不降级）。`source ≥ PENDING_LO`（segment 非势源）。
fn cascade_arm(
    located: &mut [Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    source: usize,
    extreme: f64,
    compress_bar: i64,
    confirm_bar: i64,
) {
    debug_assert!(source >= PENDING_LO, "pending 只在 move(L1) 及以上注册；source={source}");
    // ② 后续走势验证（第29课:52/54）：compress < confirm 严格小于——同 bar 武装即确认
    //    = 无后续走势 = 伪确认（degenerate located 死因）。调用点 `since_bar < bar` 保证。
    debug_assert!(compress_bar < confirm_bar, "第29课:52/54+540号严格时序：压缩 {compress_bar} ≥ 展开 {confirm_bar}（同 bar/逆序确认=无后续走势验证=伪确认）");
    for slot in located.iter_mut().take(source + 1).skip(FIRST_BSP_LADDER) {
        let overwrite = slot.map_or(true, |e| source >= e.source_ladder);
        if overwrite {
            *slot = Some(PendingLocate {
                extreme,
                source_ladder: source,
                direction: dir,
                compress_bar,
                confirm_bar,
            });
        }
    }
}

/// 级联链顶 S（pending confirm 链顶）= 最高有 located 的层。级联不变量保证 located
/// 非空时恒为连续前缀 `[FIRST_BSP_LADDER..=S]`。PCF/URS 两个独立层选择部件（E\* +
/// top）的合一替代——层选择 ≡ 链确认。
fn chain_source(located: &[Option<PendingLocate>; MAX_LADDER]) -> Option<usize> {
    (FIRST_BSP_LADDER..MAX_LADDER).rev().find(|&k| located[k].is_some())
}

/// 升序 bar 序列中 ≤ `ub` 的最大值（向心回溯：内圈末段 sub-component 的 settle bar）。
/// `type1_hist[j][side]` 按 bar 升序 append（step 单调推进）⇒ `partition_point` 二分。
fn latest_le(hist: &[i64], ub: i64) -> Option<i64> {
    let cnt = hist.partition_point(|&b| b <= ub);
    (cnt > 0).then(|| hist[cnt - 1])
}

/// **T49（confirm 向心回溯，§8.1 读法C / 第29课:396"归根结底都是第一类"）**：
/// candidate@k（压缩完成 bar=`since`，外圈 (λ^k, φ=0)）的母线纤维
/// `p⁻¹(0) ∩ {λʲ : FIRST_BSP_LADDER ≤ j < k}` 是否**逐圈贯通**——沿 φ=0 母线**向心**从
/// k−1 逐圈到结构基底 FIRST_BSP_LADDER，每内圈 j 检验**已 settle 的同侧 type1**
/// （settle bar ≤ 外层 settle bar，**嵌套**：末段 sub-component 在外层走势之内——构成
/// 这段走势的末段 move(L1) 的 settle bar、该 move(L1) 的末段 segment 的 settle bar……
/// 直到底）。这是回溯**已发生**的内圈 type1（展开恒等式 Δt∝λʲ⁻¹(λ−1)>0 ⟹ tʲ<t_K，
/// 内圈在 candidate 之过去），**非前向等待**（读法A 几何错误）。手性 ε 全程一致（同
/// `side`）。成本门（第16环）只约束 spawn 不约束此处区间套认知（编排者 2026-06-15：
/// 第14环 confirm 与第16环成本门分离）⇒ 递归到结构基底 FIRST_BSP_LADDER（含 segment）。
/// 返回母线是否逐圈贯通（贯通 ⇒ confirm@k）。
fn helix_centripetal_confirm(
    type1_hist: &[[Vec<i64>; 2]; MAX_LADDER],
    k: usize,
    side: Side,
    since: i64,
) -> bool {
    let si = match side {
        Side::Sell => 0,
        Side::Buy => 1,
    };
    // 嵌套上界：candidate@k 在 since 完美；末段内圈在其之内（settle bar ≤ since）。
    let mut upper = since;
    for j in (FIRST_BSP_LADDER..k).rev() {
        match latest_le(&type1_hist[j][si], upper) {
            // 向心一圈：取内圈 j ≤ upper 的最近 type1（末段 sub-component settle bar），
            // 下一更深内圈须在此 settle bar 之前（嵌套——更深末段在更浅末段之内）。
            Some(b) => upper = b,
            // 母线断：内圈 j 无已 settle 的同侧 type1（≤ upper）⇒ 区间套不贯通（不跳级）。
            None => return false,
        }
    }
    true // 逐圈贯通到 FIRST_BSP_LADDER（含 segment 结构基底）⇒ 区间套确认完成
}

/// **根 voice 涌现归属级别 E\*（T5/A5 第20环"根=最高涌现级别"）**：根的操作级别
/// 随走势向更高级别发展而向上生长——这是**会计重组**（重新读数），不触发物理交易
/// （A5"voice 升级=会计重组不是加仓"）。从 `root_ladder` 起，只要更高一层 `lad+1`
/// 的方向与根同向（多头根爬 Up，空头根爬 Down）且其段锚晚于（或等于）根入场，
/// 即向上爬一层。`iso` 的 `root_emergent_ladder` 的双向扩展（iso 仅 Up，因根恒多）。
fn root_emergent_ladder(
    root_ladder: usize,
    root_entry_bar: i64,
    root_dir: Polarity,
    dir_state: &[Option<Direction>; MAX_LADDER],
    anchor_state: &[i64; MAX_LADDER],
    max_l: usize,
) -> usize {
    let want = match root_dir {
        Polarity::Long => Direction::Up,
        Polarity::Short => Direction::Down,
    };
    let mut lad = root_ladder;
    while lad + 1 < max_l
        && dir_state[lad + 1] == Some(want)
        && anchor_state[lad + 1] >= root_entry_bar
    {
        lad += 1;
    }
    // **A5/T30 单调性运行时证明（release-active）**：根涌现层单调非降（爬升只升不降）。
    // 升 assert!（原 debug_assert! release 失效，GAP-C）——T30 核心命题须 release 守卫。
    assert!(lad >= root_ladder, "A5(T30) 违反：根涌现层 {lad} < 入场层 {root_ladder}（爬升应单调非降，relabel 不可降级）");
    lad
}

/// **T14（根多空对称翻转，第21环）+ A5（会计重组）运行时证明**：根就地翻转后
/// ① 极性反转（dir ≠ old_dir，§21 卖点翻空/买点翻多）；② 森林仍单根（in-place flip
/// 不增删 voice ⇒ rid 仍是唯一 active root，N1）；③ units 守恒（flip 不改 units ⇒
/// n_base 不变，A2/§8.1）。NAV 价值中性（MtM 根空头）由 step 末 prove_n8 守卫。
/// violation = panic（make-decision-observable，137号）。
fn prove_t14_root_flip(voices: &[VoiceLedger], rid: usize, old_dir: Polarity, units_pre: f64, bar: i64) {
    assert_ne!(
        voices[rid].dir, old_dir,
        "T14 违反@bar {bar}：根就地翻转后极性未反转（dir 仍 {old_dir:?}）"
    );
    assert!(
        (voices[rid].units - units_pre).abs() <= 1e-9 * units_pre.max(1.0),
        "T14 违反@bar {bar}：根翻转改变了 units（{units_pre}→{}，同股数翻转 M=N 破，A10）",
        voices[rid].units
    );
    let roots = voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none()).count();
    assert_eq!(
        roots, 1,
        "T14 违反@bar {bar}：翻转后 {roots} 个 active root（in-place flip 应保持单根，N1）"
    );
}

/// **A5（T30 根级别涌现=会计重组，N 不变）运行时证明**：根 voice 涌现归属级别向上
/// 单调升级（`root_emergent_ladder` relabel）是**会计重组**（重新读数）——A3 禁止加仓
/// ⇒ 不触发物理交易 ⇒ units 与 NAV 不变。violation（relabel 改了 units/NAV = 把重组
/// 误作加仓）= panic。
fn prove_a5_relabel(units_post: f64, units_pre: f64, nav_post: f64, nav_pre: f64, bar: i64) {
    assert!(
        (units_post - units_pre).abs() <= 1e-9 * units_pre.max(1.0),
        "A5(T30) 违反@bar {bar}：根级别涌现重组改变了 units（{units_pre}→{units_post}）——重组是重新读数非加仓（A3）"
    );
    // relabel 无物理交易 ⇒ NAV 必**严格**不变（收紧容差 1e-4→1e-9，GAP-C：relabel 非交易，
    // 不应有交易级舍入；区别于 prove_n8 的 1e-4 容差——后者守真实操作的现金流）。
    assert!(
        (nav_post - nav_pre).abs() <= 1e-9 * nav_pre.abs().max(1.0),
        "A5(T30) 违反@bar {bar}：根级别涌现重组改变了 NAV（{nav_pre}→{nav_post}）——会计重组价值中性（无物理交易）"
    );
}

// ════════════════════ 必然性运行时证明（验收标准；violation = panic）════════════════════

/// **N1（逐仓独立森林结构，第20环）**：森林不变量——① 至多一个 active root
/// （parent==None）；② 每个 active 非根 voice 的 parent 在范围内且非 Closed（孤儿不可能
/// 定理——后序 close 保证父关前子必关）；③ children 反向引用一致。返回观测到的最大
/// 子数（>1 = 森林实证，栈不可能）。violation = panic。
fn prove_n1_forest(voices: &[VoiceLedger], bar: i64) -> usize {
    let mut roots = 0usize;
    let mut max_children = 0usize;
    for (id, v) in voices.iter().enumerate() {
        if !matches!(v.status, VoiceStatus::Closed) {
            if v.parent.is_none() {
                roots += 1;
            } else {
                let p = v.parent.expect("非根有父");
                assert!(p < voices.len(), "N1 违反@bar {bar}：voice {id} 父 id {p} 越界");
                assert!(
                    !matches!(voices[p].status, VoiceStatus::Closed),
                    "N1 违反@bar {bar}：孤儿——active voice {id} 的父 {p} 已 Closed（后序 close 应已级联关子）"
                );
                assert!(
                    voices[p].children.contains(&id),
                    "N1 违反@bar {bar}：voice {id} 不在父 {p} 的 children 列表（树链断裂）"
                );
            }
        }
        let live_kids = v.children.iter().filter(|&&k| !matches!(voices[k].status, VoiceStatus::Closed)).count();
        max_children = max_children.max(live_kids);
    }
    assert!(roots <= 1, "N1 违反@bar {bar}：{roots} 个 active root（单根不变量——清仓/EOD 前森林单根）");
    max_children
}

/// **N2（per-voice 独立操作，第18环）**：本 bar 操作过的 voice id 集合无重复
/// （per-voice `acted_bar` 互斥——同一 voice 不双动；不同 voice 不互阻）。全局
/// 互斥（栈的 `acted: bool`）会使 `acted_ids.len() ≤ 1`——本检验不 panic 于此（一个
/// bar 合法可仅 1 操作），而是证明**机制是 per-voice**：无重复 ⇒ 每个 act 是独立
/// voice 的独立决策。violation（同 voice 双动）= panic。
fn prove_n2_per_voice(acted_ids: &[usize], bar: i64) {
    for (i, &a) in acted_ids.iter().enumerate() {
        for &b in &acted_ids[i + 1..] {
            assert!(a != b, "N2 违反@bar {bar}：voice {a} 同 bar 双动（per-voice acted_bar 失效）");
        }
    }
}

/// **N3（全三类 BSP 消费，第12环）**：type2（中枢回测确认）不被跳过。窗口武装时
/// 每个事件按 `e.class.side()` 归侧——无 `Sell2|Buy2 => continue`。检验 = 本 bar 出现
/// 的 type2 事件数 == 被处理（武装 ∨ confirmed 清窗）数。violation（type2 漏处理）= panic。
fn prove_n3_type2(type2_seen: u64, type2_handled: u64, bar: i64) {
    assert_eq!(
        type2_seen, type2_handled,
        "N3 违反@bar {bar}：type2 事件 {type2_seen} 个，仅处理 {type2_handled} 个（Sell2/Buy2 被跳过=中枢生命周期阶段丢失）"
    );
}

/// **N5+N6（区间套自上而下定位 + 先势后定位时序，第14环/540号）**：操作的 source
/// 是一条**操作前已完整级联形成**的定位链顶层。五项硬断言（F/C 每个 pending_locate
/// 操作点调用）：① `source ≥ PENDING_LO`（segment 非势源，N5）；② `located[s]` 在场且
/// `source_ladder==s ∧ direction==dir`（链顶一致，N5）；③ `compress_bar ≤ confirm_bar
/// ≤ bar`（压缩↑必先于展开↓必不晚于操作，N6 540号）；④ `[FIRST_BSP..=s]` 全 located
/// 且 source_ladder 统一=s（级联连续前缀，N5）。violation = panic。
fn prove_chain(
    located: &[Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    s: usize,
    bar: i64,
    op: &str,
) {
    assert!(
        s >= PENDING_LO,
        "N5 违反@bar {bar} {op}：source={s} < move(L1)={PENDING_LO}（segment 非势源）"
    );
    let top = located[s]
        .unwrap_or_else(|| panic!("N5 违反@bar {bar} {op}：source={s} 无 located（无定位链的操作=bug）"));
    assert_eq!(top.source_ladder, s, "N5 违反@bar {bar} {op}：located[{s}].source_ladder≠{s}（链顶不一致）");
    assert_eq!(top.direction, dir, "N5 违反@bar {bar} {op}：located[{s}].direction 方向错配");
    assert!(
        top.compress_bar < top.confirm_bar,
        "N6 违反@bar {bar} {op}：第29课:52/54+540号 压缩 {} ≥ 展开 {}（同 bar/逆序确认=无后续走势验证=伪确认，degenerate located）",
        top.compress_bar, top.confirm_bar
    );
    assert!(
        top.confirm_bar <= bar,
        "N6 违反@bar {bar} {op}：confirm_bar={} > bar（未来武装，因果违反）",
        top.confirm_bar
    );
    for k in FIRST_BSP_LADDER..=s {
        let e = located[k]
            .unwrap_or_else(|| panic!("N5 违反@bar {bar} {op}：定位链 [{FIRST_BSP_LADDER}..={s}] 在层 {k} 断裂"));
        assert_eq!(
            e.source_ladder, s,
            "N5 违反@bar {bar} {op}：located[{k}].source_ladder={} ≠ 链顶 {s}（级联非统一 source）",
            e.source_ladder
        );
    }
}

/// **N5（级联结构，每 bar 后置）**：located 非空 ⇒ 连续前缀 `[FIRST_BSP..=S]` 且每层
/// `source_ladder ≥ k`（自上而下——源在本层或更高；自下而上独立武装会出现
/// `source_ladder < k`）。violation = panic。
fn prove_n5_cascade(located: &[Option<PendingLocate>; MAX_LADDER], bar: i64, side: &str) {
    let top = chain_source(located);
    if let Some(s) = top {
        for k in FIRST_BSP_LADDER..=s {
            let e = located[k].unwrap_or_else(|| {
                panic!("N5 违反@bar {bar} {side}：located 顶={s} 但层 {k} 空（非连续前缀=非级联）")
            });
            assert!(
                e.source_ladder >= k,
                "N5 违反@bar {bar} {side}：located[{k}].source_ladder={} < {k}（自下而上独立武装，非级联自上而下）",
                e.source_ladder
            );
            assert!(e.source_ladder >= PENDING_LO, "N5 违反@bar {bar} {side}：source<move(L1)（segment 武装为势源）");
        }
        for k in (s + 1)..MAX_LADDER {
            assert!(located[k].is_none(), "N5 违反@bar {bar} {side}：源层 {s} 之上层 {k} 有 located（非前缀）");
        }
    }
}

/// **N7（降成本不需 pending，第17环）**：E spawn 的触发是 voice **自层** 走势结构
/// （nf@voice.ladder ∨ 根自层 confirmed 卖），非全局 located 链、未经 `prove_chain`。
/// violation（触发层 ≠ voice 层 = 借用更高级别 pending）= panic。
fn prove_n7_spawn_self_level(trigger_ladder: usize, voice_ladder: usize, bar: i64) {
    assert_eq!(
        trigger_ladder, voice_ladder,
        "N7 违反@bar {bar}：降成本 spawn 触发层 {trigger_ladder} ≠ voice 层 {voice_ladder}（E 借用了更高级别 pending_locate，非自层走势）"
    );
}

/// **N8（双层会计多空嵌套守恒，第22环/§8.1/§8.4）**：① Σ(活跃 voice 在手) = N_base
/// （股数守恒，多空嵌套流转不增不减）；② 同价 c 操作前后 NAV 不变（价值中性——
/// child.P&L ≡ parent.cost_reduction 的无条件严格形式，§11 审计）。violation = panic。
fn prove_n8_conservation(
    voices: &[VoiceLedger],
    n_base: f64,
    nav_pre: f64,
    nav_post: f64,
    bar: i64,
) {
    let sum_units: f64 = voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).map(|v| v.units).sum();
    assert!(
        (sum_units - n_base).abs() <= 1e-6 * n_base.max(1.0),
        "N8 违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}（股数守恒 §8.1）"
    );
    assert!(
        (nav_post - nav_pre).abs() <= 1e-4 * nav_pre.abs().max(1.0),
        "N8 违反@bar {bar}：NAV {nav_pre}→{nav_post}（同价操作非价值中性 §8.4——双层会计破）"
    );
}

/// **T1（不主动清仓，第23环恒仓的精确形式）运行时证明**：编排者裁决 2026-06-15——"恒仓"
/// 约束**引擎的主动行为**（不主动清仓，只在买卖点操作 F/E/C/D），**不约束市场的强制平仓**
/// （A 强平是市场机制，被动；引擎不可否认）。删否定线 + 观测态后，引擎**无主动清仓路径**：
/// C 在 type1 卖点 **in-place 翻转**（不清仓，root_ladder>FIRST_BSP，clear-at-base 不可达已升
/// `unreachable!()`），E/D 只在子树内流转不动根，F 只建仓。故森林"非空→空"**只能经 A 强平
/// （被动会计终局，c≥2×basis 1x 逐仓 capital 耗尽）**——`root_liquidated` 标记本 bar A 关闭了
/// 根。强平→空仓→下一个 BSP→F 重建仓（从最高可介入级别，N5-gated），空仓期间是被动等待
/// （非引擎选择）。两支判据（编排者裁决 2026-06-15 细化）：
/// ① **被动空仓**：森林"非空→空"⟹ `root_liquidated`（A 强平）。非强平致空仓 = 引擎主动清仓
///    （否定线/观测态残留）= violation。强平后到下一个 BSP 之间的空仓是正常被动等待。
/// ② **立即重建**：若本 bar 森林空 ∧ **F-eligible**（buy_source ∧ buy1@s ∧ free>0，有买点+
///    资本）但 bar 末仍空（`f_eligible_but_empty`）⇒ violation（引擎本应建仓但没建——强平后
///    应在下一个 confirmed BSP 从区间套最高可介入级别立即重建仓）。注：无资本（free≤0，破产）
///    或无 located 级联买点时 F 不 eligible，被动空仓正常，非 violation。
/// violation = panic（137号 make-decision-observable）。
fn prove_t1_no_voluntary_exit(
    was_active: bool,
    is_active: bool,
    root_liquidated: bool,
    f_eligible_but_empty: bool,
    bar: i64,
) {
    if was_active && !is_active {
        assert!(
            root_liquidated,
            "T1①违反@bar {bar}：森林非空→空 但根未被市场强平（引擎主动清仓？否定线/观测态\
             残留——引擎应只在买卖点主动操作，主动清仓违反第23环恒仓）"
        );
    }
    assert!(
        !f_eligible_but_empty,
        "T1②违反@bar {bar}：森林空 ∧ F-eligible 买点（buy_source ∧ buy1 ∧ free>0）但未重建仓\
         （引擎本应建仓但没建——强平后应在下一个 confirmed BSP 立即重建仓）"
    );
}

/// **N4（成本门终止递归，第16环）**：递归终止纯由成本门——`floor_stop` 计数器恒 0
/// （`try_spawn_cost_gated` 无 floor 检查，终止只走 `noref_reject`（θ=None=势不可测）∨
/// `cost_reject`（θ<friction=势幅度<成本））。eod 反证：出现任何 floor_stop ⇒ panic。
fn prove_n4_cost_gate(res: &PositionalResult) {
    let floor_stops: u64 = res.n_nrf_floor_stops_by_ladder.iter().sum();
    assert_eq!(
        floor_stops, 0,
        "N4 违反：floor_stop={floor_stops}（出现固定 floor 终止——应纯成本门 θ=None ∨ θ<friction）"
    );
}

/// **T50（操作频率径向标度律，§8.3 螺旋扩展）运行时观测（eod，~ 状态）**：展开恒等式
/// Δt(k)∝λ^k（一圈弧长∝半径 r=λ^k）⟹ 级别 k 的 φ=0 事件（confirm/操作）频率
/// `f(k)∝λ^{−k}` 随级别指数衰减。清仓（最外圈 K，∝λ^{−K}）极罕见，降成本（内圈 k≪K，
/// ∝λ^{−k}）频繁，比值 = λ^{K−k}（指数级）。**A₅ 依赖度分裂**：① 定性核（f(k) 随 k 单调
/// 递减 ⟹ 最外圈比内圈罕见）只需 t∝r 单调（**A₅-独立**）；② 指数律 λ^{−k} 的**定量**形式
/// 需 A₅（λ 恒定），是 **L2 可证伪预测**（测各级 confirm 间隔比≈λ）。**~ 状态**
/// （formalization-validity-domain.md：不声明为 ✓、**不 panic**——否则把 L2 可证伪定量律
/// 误作 L0 不变量；与 S9 同范式：标度律是回测/L2 读数，非每 bar 验收不变量）。返回**定性
/// 单调性局部违反计数**（fire(k) > fire(k−1) 的层数，观测非 panic）。
fn prove_t50_radial_scaling(res: &PositionalResult) -> u64 {
    // f(k) = 各级别 confirm fire 频率（向心 confirm 兑现点，per ladder）。
    let fire: [u64; MAX_LADDER] = std::array::from_fn(|k| {
        res.n_nest_fire_sell_by_ladder[k] + res.n_nest_fire_buy_by_ladder[k]
    });
    // A₅-独立定性核：candidate 层 [PENDING_LO, MAX) 的 fire(k) 随 k 非增（高层比低层罕见）。
    let mut violations = 0u64;
    for k in (PENDING_LO + 1)..MAX_LADDER {
        if fire[k] > fire[k - 1] {
            violations += 1; // 高层 fire 多于相邻低层 = 标度律定性序局部违反（观测）
        }
    }
    violations
}

// ═══════════ T2/T4 定义层保证（无运行时 prove，编排者 2026-06-15）═══════════
// 旧 `prove_t2_perfection_triaxis` / `prove_t4_classification` 是**重言型构造守卫**
// （三断言/`matches!` 被构造逻辑蕴含，对任何输入永真，永不 fire）——保留它们 = 用永不
// fire 的 assert 假装运行时验证（声明膨胀）。两条推论由**定义层**保证，故删 prove：
//
// **T2（走势完美三坐标合一）**：confirm fire 的判据 `confirmed = since<bar ∧ helix`
//   （step 中 confirm 触发处）**就是**三坐标合一——① 角向 φ→0（since<bar，背驰后有后续走势，
//   第29课:52/54）；② 径向 settle（helix 母线逐圈贯通，每内圈 type1 已 settle）；③ 手性 ε
//   （helix 贯通 ⟹ 最内圈 FIRST_BSP 同侧 type1 历史非空，向心回溯消费同手性，k≥PENDING_LO
//   保证 helix 循环必经 FIRST_BSP 一圈）。三坐标合一是 confirm 的**构造定义**，无独立于 fire
//   门的运行时不变量可验 ⇒ 定义层 ✓，不设 prove。单独 settle≠完美（project_ph_settle_usage_boundary）。
// **T4/T10（走势完全分类无第四类）**：`BspKind` enum 只有 {Type1,Type2,Type3} 三变体
//   （buysellpoint.rs:26）⇒ 分类完备由**编译期穷尽**保证（类型层 = 定义层 ✓）。type2/3 中枢锚
//   （T10 径向圈数判别量的结构前提）由 `prove_s12_center`（消费侧，step 中每 BSP 事件）+ 生成侧
//   `buysellpoints_from_level` 守卫；趋向维斜率 DivEvent 无段端点不可达（T11 投影缺口，不强加伪判据）。

/// 成本门动态 spawn（N4 第16环）：父 voice 释放 θ_sub 配额给子 voice@sub=parent.ladder−1。
/// **无 floor 参数**——终止纯由成本门：`theta(sub)=None`（势不可测=不存在，递归基 bi）∨
/// `theta(sub) < SUB_COST_K×friction`（势幅度<成本）。`floor_stop` 计数器**恒不增**
/// （prove_n4 据此反证无固定 floor 终止）。返回是否成功开仓。
fn try_spawn_cost_gated(
    parent_id: usize,
    bar: i64,
    c: f64,
    depth_ref: &DepthRef,
    voices: &mut Vec<VoiceLedger>,
    res: &mut PositionalResult,
) -> bool {
    let p_ladder = voices[parent_id].ladder;
    let p_dir = voices[parent_id].dir;
    let p_units = voices[parent_id].units;
    let p_capital = voices[parent_id].capital;
    // N4：递归基 = bi(a0)。p_ladder == FIRST_BSP_LADDER ⇒ sub = bi 层，theta(bi)=None
    // ⇒ 下方 noref_reject 自然终止（无 floor 检查——纯成本门）。
    let sub = p_ladder - 1;
    match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
        None => {
            res.n_nrf_noref_rejects_by_ladder[sub] += 1; // N4：势不可测=势不存在（递归终止）
            false
        }
        Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
            res.n_nrf_cost_rejects_by_ladder[sub] += 1; // N4：势幅度<成本（势消失）
            false
        }
        Some(_) => {
            // m = 父在手 × θ_sub/θ_total（53课配额；高级别势大→大仓位，第15环）。
            // θ_total 跨 [FIRST_BSP, MAX)——所有结构承载层（无操作 floor）。
            let (thetas, theta_total) = theta_weights(depth_ref, FIRST_BSP_LADDER);
            let w = thetas[sub].map(|t| t / theta_total);
            let m_quota = w.map_or(0.0, |w| p_units * w);
            let m = match p_dir {
                Polarity::Long => m_quota,
                Polarity::Short => m_quota.min(p_capital / c),
            };
            if !(m > 0.0 && m.is_finite()) {
                return false;
            }
            let child_dir = match p_dir {
                Polarity::Long => Polarity::Short,
                Polarity::Short => Polarity::Long,
            };
            let child = VoiceLedger {
                ladder: sub,
                dir: child_dir,
                units: m,
                basis: c,
                cost_pool: m * c,
                capital: if child_dir == Polarity::Short { m * c } else { 0.0 },
                entry_bar: bar,
                negate_line: None, // 删否定线：子 voice 生命周期纯买卖点驱动（D 回补 / A 强平）
                status: VoiceStatus::Active,
                parent: Some(parent_id),
                children: Vec::new(),
                realized_pnl: 0.0,
                acted_bar: bar,
            };
            let child_id = voices.len();
            voices[parent_id].units -= m;
            if child_dir == Polarity::Long {
                voices[parent_id].capital -= m * c;
            }
            voices[parent_id].children.push(child_id);
            voices[parent_id].refresh_status();
            voices.push(child);
            res.n_nrf_spawns_by_ladder[sub] += 1;
            res.n_entries_by_ladder[sub] += 1;
            true
        }
    }
}

/// 流式 unn 引擎核心（535号边界B：push_bar 收 BarSig = unn 定义域 SignalTape 的单元）。
/// VoiceLedger 是 `pub(super)` ⇒ 流式核心必须在 trading 模块内；lib.rs 仅 PyO3 包装。
/// 批量 `run_unified_necessity` 与流式 `UnnStream` **共享同一个 `step` 方法** ⇒ bit-exact
/// 是构造性保证（非两份代码对齐——同一段循环体被外部驱动；no-patch.md：不是补丁，
/// 是把原 for 循环体原样提取为可逐 bar 推进的方法）。
pub(crate) struct UnnStreamCore {
    res: PositionalResult,
    voices: Vec<VoiceLedger>,
    free: f64,
    n_base: f64,
    book: CenterBook,
    depth_ref: DepthRef,
    // pending 窗口（双侧，k ≥ PENDING_LO；segment 非势源）。
    nest_sell: [Option<Pending>; MAX_LADDER],
    nest_buy: [Option<Pending>; MAX_LADDER],
    // 级联定位链（卖侧出场链 + 买侧入场链）。
    located_sell: [Option<PendingLocate>; MAX_LADDER],
    located_buy: [Option<PendingLocate>; MAX_LADDER],
    // T49（confirm 向心回溯）：每级 type1 settle 历史（bar 升序 append；[Sell,Buy] 双侧）。
    // 向心 confirm 沿 φ=0 母线回溯已 settle 的内圈 type1（`helix_centripetal_confirm`）——
    // 内圈 type1 在 candidate 之过去（展开恒等式 Δt∝λʲ⁻¹(λ−1)>0），故需 settle 历史
    // （前向窗口不需历史但前向是几何错误）。a0/bi 层无 BSP 事件 ⇒ 索引 [FIRST_BSP, MAX) 有效。
    type1_hist: [[Vec<i64>; 2]; MAX_LADDER],
    // 方向/段锚滚动状态（T5 根级别涌现 E\* 读数；iso 同构 dir_state/anchor_state）。
    dir_state: [Option<Direction>; MAX_LADDER],
    anchor_state: [i64; MAX_LADDER],
    // N4 累计观测（prove_n4 在 eod 反证 floor_stop 恒 0）。
    max_children_seen: usize,
    // 信号层 located 势源流证明状态（S11 交替 panic + S9 价格观测；编排者裁决 located 非 raw）。
    sig_state: SigLocatedState,
    // 空事件行（无事件 bar 复用——与批量同一引用语义，零分配漂移）。
    empty_evs: [Vec<BspEvent>; MAX_LADDER],
    empty_devs: [Vec<DivEvent>; MAX_LADDER],
    // 流式驱动状态：下一个要 push 的 bar index（= 已 push bar 数）。
    cur_bar: i64,
    last_close: f64,
    finished: bool,
}

impl UnnStreamCore {
    /// 初始化（零参数引擎；floor_ladder 仅作结构递归基断言 = FIRST_BSP_LADDER，N4）。
    /// 注：tape 级 capability guard（has_bsp/div/dir）是批量入口专属——流式无完整
    /// tape 可查，capability 由调用方逐 bar 传事件结构保证（push_bar 总传事件行）。
    pub(crate) fn new(floor_ladder: usize) -> Result<Self, String> {
        if floor_ladder != FIRST_BSP_LADDER {
            return Err(format!(
                "unified_necessity 是零操作参数引擎：floor_ladder 仅作结构递归基 = \
                 FIRST_BSP_LADDER={FIRST_BSP_LADDER}（N4 纯成本门，无操作 floor）；得 {floor_ladder}"
            ));
        }
        Ok(Self {
            res: PositionalResult::default(),
            voices: Vec::new(),
            free: INITIAL_CAPITAL,
            n_base: 0.0,
            book: CenterBook::new(),
            depth_ref: DepthRef::new(DEPTH_REF_WINDOW),
            nest_sell: [None; MAX_LADDER],
            nest_buy: [None; MAX_LADDER],
            located_sell: [None; MAX_LADDER],
            located_buy: [None; MAX_LADDER],
            type1_hist: Default::default(),
            dir_state: [None; MAX_LADDER],
            anchor_state: [-1; MAX_LADDER],
            max_children_seen: 0,
            sig_state: SigLocatedState::default(),
            empty_evs: Default::default(),
            empty_devs: Default::default(),
            cur_bar: 0,
            last_close: f64::NAN,
            finished: false,
        })
    }

    /// 单 bar 推进（= 原 `run_unified_necessity` for 循环体，逐字——批量/流式共享）。
    /// `flip_edge` = 本 bar 方向翻转沿（批量由 flips 数组按 bar 切出；流式由 push_bar
    /// 从 flip_rows 构造）。每 bar 优先序 A→F + 必然性 prove（violation=panic）。
    pub(crate) fn step(&mut self, sig: &BarSig, flip_edge: &[Option<Direction>; MAX_LADDER]) {
        let bar = self.cur_bar;
        let c = sig.close;
        self.last_close = c;

        // 方向/段锚滚动状态（T5 根级别涌现 E\* 的读数载体；当 bar 翻转沿更新）。
        for lad in 0..MAX_LADDER {
            if let Some(d) = flip_edge[lad] {
                self.dir_state[lad] = Some(d);
                self.anchor_state[lad] = bar;
            }
        }
        // 涌现爬升上界（承载层 + 1，封顶 MAX_LADDER）。
        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        // 市场性质：中枢账本 + 振幅参照。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                // S12（T13/T9）：信号层每个 BSP 事件中枢锚良序运行时证明（per-bar per-ladder
                // 覆盖空间——FIRST_BSP 成立必在所有更高 ladder 成立，T16）。violation=panic。
                for e in &evrows[lad] {
                    // S12（T13/T9 中枢锚良序）+ T4/T10/T47（走势完全分类无第四类）：BspKind enum
                    // 三变体编译期穷尽 = 分类完备定义层 ✓（无 prove_t4，见上"T2/T4 定义层保证"）；
                    // type2/3 中枢锚（径向圈数判别量）由本 prove_s12_center + 生成侧 buysellpoints_from_level。
                    prove_s12_center(e.class.kind(), e.cs, e.zd, e.zg, lad, bar);
                }
                self.book.ingest(lad, &evrows[lad], true, None);
            }
            self.depth_ref.observe(&self.book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&self.empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] = sig.div_events.as_deref().unwrap_or(&self.empty_devs);

        // ── pending 窗口维护（双侧，k ≥ PENDING_LO）：① 破极值否定 → ② candidate
        //    武装（N3：type2 经 side() 同等武装，无 continue）/ confirmed 清窗 →
        //    ③ confirm 触发（rec_sub_evidence 递归到 a0，含 segment）。
        //    confirm@k → nf_*[k]（自层 fire，供 E，N7）+ confirm_*[k]（供级联，N5）──
        let mut confirm_sell: [Option<(f64, i64)>; MAX_LADDER] = [None; MAX_LADDER];
        let mut confirm_buy: [Option<(f64, i64)>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        // N3 计数：本 bar type2 出现数 vs 处理数。
        let mut type2_seen = 0u64;
        let mut type2_handled = 0u64;
        for k in PENDING_LO..MAX_LADDER {
            if self.nest_sell[k].is_some_and(|w| c > w.extreme) {
                self.nest_sell[k] = None;
                self.res.n_nest_breaks_by_ladder[k] += 1;
            }
            if self.nest_buy[k].is_some_and(|w| c < w.extreme) {
                self.nest_buy[k] = None;
                self.res.n_nest_breaks_by_ladder[k] += 1;
            }
            if sig.bsp_events.is_some() {
                for e in &evrows[k] {
                    // N3：type2（中枢回测确认）与 type1/3 同等按 side() 归侧（无 continue）。
                    if e.class.kind() == BspKind::Type2 {
                        type2_seen += 1;
                    }
                    let sellside = match e.class.side() {
                        Side::Sell => true,
                        Side::Buy => false,
                    };
                    let win = if sellside { &mut self.nest_sell[k] } else { &mut self.nest_buy[k] };
                    if e.confirmed {
                        *win = None; // confirmed 同侧让位（本 bar 走 confirmed 路径）
                    } else {
                        let (ext, since) = win.map_or((e.price, bar), |w| {
                            let ext = if sellside { w.extreme.max(e.price) } else { w.extreme.min(e.price) };
                            (ext, w.since_bar) // 压缩起始保留（540号；frontier 已删——T49 向心回溯无跨 bar 累积）
                        });
                        *win = Some(Pending { extreme: ext, since_bar: since });
                        self.res.n_nest_arms_by_ladder[k] += 1;
                    }
                    if e.class.kind() == BspKind::Type2 {
                        type2_handled += 1; // 武装 ∨ confirmed 清窗——两路均处理（无 continue）
                    }
                }
            }
            // ── 第14环展开↓ confirm（T49 向心回溯，§8.1 读法C；替代旧前向 frontier 累积）──
            //    candidate@k（外圈 (λ^k, φ=0)）出现后，沿 φ=0 母线**向心**逐圈 k−1…FIRST_BSP
            //    检验**已 settle 的内圈 type1**（嵌套：末段 sub-component 在外层之内）。内圈
            //    type1 在 candidate 之过去（展开恒等式 Δt∝λʲ⁻¹(λ−1)>0），**非前向等待**
            //    （旧 frontier 前向累积 = 读法A 几何错误，project_recl2_confirm_breakpoint_temporal
            //    418/418 时序错配）。成本门（第16环）只约束 spawn 不约束此处区间套认知
            //    （编排者 2026-06-15）⇒ helix_centripetal_confirm 递归到结构基底 FIRST_BSP_LADDER。
            //    母线逐圈贯通 ∧ since<bar（②后续走势，第29课:52/54）⇒ confirm@k。同一 fire 两路
            //    （header §32）：confirm_*[k]（cascade→C/F，N5）+ nf_*[k]（自层→E，N7）；C/E 由
            //    sig.sell1/buy1[source] 区分（type1→C，type2/3→E，§9）；C 在 E 之前（cleared 跳 E）。
            if let Some(w) = self.nest_sell[k] {
                let helix = helix_centripetal_confirm(&self.type1_hist, k, Side::Sell, w.since_bar);
                let confirmed = w.since_bar < bar && helix;
                if confirmed {
                    // T2 三坐标合一 = confirm 的构造定义：φ→0(since<bar 后续走势) ∧ settle(helix
                    // 母线贯通) ∧ ε 手性(helix 贯通 ⟹ 最内圈 FIRST_BSP 同侧 type1 历史非空)——
                    // 定义层 ✓，无 prove_t2（见上"T2/T4 定义层保证"）。
                    confirm_sell[k] = Some((w.extreme, w.since_bar));
                    nf_sell[k] = Some(w.extreme);
                    self.res.n_nest_fire_sell_by_ladder[k] += 1;
                    self.nest_sell[k] = None;
                }
                // else：母线未贯通（内圈缺已 settle type1）∨ since==bar（无后续走势，待下一 bar
                //       当下感知）⇒ 窗口跨 bar 持续（直到 confirm ∨ 破极值否定清窗）。
            }
            if let Some(w) = self.nest_buy[k] {
                let helix = helix_centripetal_confirm(&self.type1_hist, k, Side::Buy, w.since_bar);
                let confirmed = w.since_bar < bar && helix;
                if confirmed {
                    // T2 三坐标合一 = confirm 的构造定义（买侧 si=1）——定义层 ✓，无 prove_t2。
                    confirm_buy[k] = Some((w.extreme, w.since_bar));
                    nf_buy[k] = Some(w.extreme);
                    self.res.n_nest_fire_buy_by_ladder[k] += 1;
                    self.nest_buy[k] = None;
                }
            }
        }
        prove_n3_type2(type2_seen, type2_handled, bar);

        // ── T49：记录本 bar 各层 type1 settle（向心回溯历史；bar 升序 append）。在 confirm
        //    之后记录——本 bar type1（bar=cur）相对所有 since<bar 的 candidate 在**未来**，
        //    不被本 bar 向心消费（向心只取 settle bar ≤ since 的内圈过去 type1）；供后续 bar
        //    的 candidate 回溯。borrow：evs 借 sig（不借 self），self.type1_hist 可变借不冲突。──
        if let Some(evs) = sig.bsp_events.as_deref() {
            for (j, hist_j) in self.type1_hist.iter_mut().enumerate().take(MAX_LADDER).skip(FIRST_BSP_LADDER) {
                for e in &evs[j] {
                    if e.class.kind() == BspKind::Type1 {
                        let si = match e.class.side() {
                            Side::Sell => 0,
                            Side::Buy => 1,
                        };
                        hist_j[si].push(bar);
                    }
                }
            }
        }

        // ── 级联武装 located（N5：confirm@k → cascade [FIRST_BSP..=k]，高 source
        //    优先；按 source 降序施加）。仅供根 F/C 消费——E 用 nf_*（N7）──
        for k in (PENDING_LO..MAX_LADDER).rev() {
            if let Some((ext, since)) = confirm_sell[k] {
                cascade_arm(&mut self.located_sell, Side::Sell, k, ext, since, bar);
            }
            if let Some((ext, since)) = confirm_buy[k] {
                cascade_arm(&mut self.located_buy, Side::Buy, k, ext, since, bar);
            }
        }
        // 破极值否定（027:25）：级联统一极值 ⇒ 整链同破。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if self.located_sell[k].is_some_and(|e| c > e.extreme) {
                self.located_sell[k] = None;
            }
            if self.located_buy[k].is_some_and(|e| c < e.extreme) {
                self.located_buy[k] = None;
            }
        }
        prove_n5_cascade(&self.located_sell, bar, "sell");
        prove_n5_cascade(&self.located_buy, bar, "buy");

        let sell_source = chain_source(&self.located_sell);
        let buy_source = chain_source(&self.located_buy);

        // N8 价值守恒入口快照。
        let nav_pre = nav(&self.voices, self.free, c);
        // N2：本 bar 操作过的 voice id（去全局互斥的运行时证明）。
        let mut acted_ids: Vec<usize> = Vec::new();
        // T1（不主动清仓）入口快照：森林"非空→空"只能经 A 强平（被动会计终局，设
        // root_liquidated），引擎无主动清仓路径——prove_t1_no_voluntary_exit 守卫。
        let was_active = self.voices.iter().any(|v| !matches!(v.status, VoiceStatus::Closed));
        let mut root_liquidated = false;

        // ── A. 强平兜底（逐活跃空头 voice；per-voice——强平某 voice 不阻断其他）。
        //    会计终局（1x 逐仓 capital 耗尽 c≥2×basis），市场强制机制（被动）——唯一非买卖点
        //    离场，非引擎主动操作（编排者裁决：恒仓约束引擎主动行为，不约束市场强平）。──
        let snap: Vec<usize> = (0..self.voices.len()).collect();
        for &id in &snap {
            if matches!(self.voices[id].status, VoiceStatus::Closed) || self.voices[id].dir != Polarity::Short {
                continue;
            }
            let v = &self.voices[id];
            if v.capital + v.units * (v.basis - c) <= 0.0 {
                let lad = v.ladder;
                let is_root = v.parent.is_none();
                let b = 2.0 * v.basis;
                close_voice(id, bar, b, c, "liq", false, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                self.res.n_short_liquidations_by_ladder[lad] += 1;
                if is_root {
                    root_liquidated = true; // 根空头被市场强平 ⇒ 森林清空（被动；F 下一 BSP 重建）
                }
                acted_ids.push(id);
            }
        }

        // ── 否定线删除（编排者 2026-06-15）：旧 B 规则（破 027:25 negate_line ⇒ 清算/翻转/
        //    对冲）在买卖点之外凭空加操作触发条件 = 违反第11环（操作只在买卖点）。否定线是
        //    经验性补丁，非概念运动链推论 ⇒ 整个 B 规则删除。voice 生命周期纯买卖点驱动
        //    （F买点 / E次级别卖点 / C同级别type1 / D子级别买点 + A会计强平）。──

        // ── C. 根清仓/翻转（N5/N6 + §6 + 第21/23环 + T5/T14 双向）──
        //    T5/A5 会计重组：先按走势涌现把根级别向上重组（root_emergent_ladder，
        //      monotone relabel，纯会计无物理交易 ⇒ units/NAV 不变 ⇒ N8/A2 安全）。
        //    多头根：卖链 source S ≥ re ∧ sig.sell1[S]（type1 走势完美，§6"十年 1-2 次"；
        //      type2/3 落 E §9）∧ prove_chain ⇒ 单根时**翻空 in-place**（长→空，T14，
        //      MtM 守恒）∨ 清仓到现金（root@segment——root_ladder≥PENDING_LO 故罕见）。
        //    空头根（T14 翻空后）：买链 source S ≥ re ∧ sig.buy1[S]（type1 底背驰）
        //      ∧ prove_chain ⇒ 单根时**翻多 in-place**（空→长，cover+rebuy 守恒）。
        //    多 voice（根有降成本子）⇒ C no-op，子先经 D 独立回补（逐仓不 collapse）。
        let mut cleared = false;
        let root_id = self.voices
            .iter()
            .position(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none() && v.units > 0.0
                && v.acted_bar != bar); // N2 防双动：跳过本 bar 已被 A 操作的根
        if let Some(rid) = root_id {
            let root_dir = self.voices[rid].dir;
            // T5/A5 会计重组（root_emergent_ladder 向上单调升级，纯 relabel）。
            let re = root_emergent_ladder(
                self.voices[rid].ladder, self.voices[rid].entry_bar, root_dir,
                &self.dir_state, &self.anchor_state, max_l,
            );
            if re > self.voices[rid].ladder {
                // A5（T30 涌现=会计重组）：relabel 前后 units/NAV 快照对比（重组非加仓）。
                let units_pre_re = self.voices[rid].units;
                let nav_pre_re = nav(&self.voices, self.free, c);
                self.voices[rid].ladder = re; // 会计重组：无物理交易，units/NAV 不变
                prove_a5_relabel(
                    self.voices[rid].units,
                    units_pre_re,
                    nav(&self.voices, self.free, c),
                    nav_pre_re,
                    bar,
                );
            }
            let root_ladder = self.voices[rid].ladder;
            let active_count = self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
            let single_root = active_count == 1;
            // ── 诊断（env UNN_DBG_QQQ）：根空头持仓期间，本层 raw type1 买点为何不触发 C 翻多 ──
            if std::env::var_os("UNN_DBG_QQQ").is_some()
                && root_dir == Polarity::Short
                && sig.buy1.get(root_ladder)
            {
                let bs = buy_source;
                let chain_reaches = bs.is_some_and(|s| s >= root_ladder);
                let buy1_at_src = bs.is_some_and(|s| sig.buy1.get(s));
                let chain: Vec<(usize, usize)> = (FIRST_BSP_LADDER..MAX_LADDER)
                    .filter(|&k| self.located_buy[k].is_some())
                    .map(|k| (k, self.located_buy[k].unwrap().source_ladder))
                    .collect();
                eprintln!(
                    "[C-noflip bar={bar}] root SHORT@ladder{root_ladder} raw_buy1@{root_ladder}=T \
                     | buy_source={bs:?} s>=root_ladder={chain_reaches} buy1@source={buy1_at_src} \
                     single_root={single_root} | located_buy链(层,源)={chain:?}"
                );
            }
            match root_dir {
                Polarity::Long => {
                    if let Some(s) = sell_source {
                        if s >= root_ladder && sig.sell1.get(s) {
                            prove_chain(&self.located_sell, Side::Sell, s, bar, "C-flip/clear");
                            // ── 诊断（env UNN_DBG_QQQ）：C 翻空触发点完整 located 链 + 次级别证据层 ──
                            if std::env::var_os("UNN_DBG_QQQ").is_some() {
                                let chain: Vec<(usize, usize, i64, i64)> = (FIRST_BSP_LADDER..MAX_LADDER)
                                    .filter(|&k| self.located_sell[k].is_some())
                                    .map(|k| {
                                        let e = self.located_sell[k].unwrap();
                                        (k, e.source_ladder, e.compress_bar, e.confirm_bar)
                                    })
                                    .collect();
                                let ev_layer = rec_sub_evidence(s - 1, Side::Sell, evrows, devrows, flip_edge);
                                let top_ext = self.located_sell[s].unwrap().extreme;
                                eprintln!(
                                    "[C-FLIP-SHORT bar={bar}] sell_source={s} root_ladder={root_ladder} \
                                     sell1@{s}={} sell_any@{s}={} single_root={single_root} top_extreme={top_ext:.2} \
                                     | located_sell链(层,源,压缩bar,确认bar)={chain:?} \
                                     | 次级别证据层(s-1={})={ev_layer:?}",
                                    sig.sell1.get(s), sig.sell_any.get(s), s - 1
                                );
                            }
                            if single_root && root_ladder > FIRST_BSP_LADDER {
                                // T14 根翻空：长→空 in-place。卖多 free+=m×c；空头收 capital=m×c。
                                // 根空头 MtM：nav_pre=free+m×c（多）→ nav_post=free'+（capital−m×c）
                                // =（free+m×c）+0 ⇒ NAV 守恒。units=m 不变 ⇒ Σunits=N_base（A2）。
                                let m = self.voices[rid].units;
                                // 记长腿 trade（入场→翻空相完成；settle 纯观测，不动现金）。
                                settle(&mut self.voices, rid, bar, c, "flip_short", &mut self.res);
                                self.free += m * c;
                                self.voices[rid].dir = Polarity::Short;
                                self.voices[rid].basis = c;
                                self.voices[rid].capital = m * c;
                                self.voices[rid].cost_pool = m * c;
                                self.voices[rid].entry_bar = bar; // 空头相起点（涌现读数锚）
                                self.voices[rid].negate_line = None; // 删否定线：根空头纯买卖点驱动（C翻多/A强平）
                                self.voices[rid].acted_bar = bar;
                                self.res.n_nrf_root_flips_by_ladder[root_ladder] += 1;
                                self.located_sell = [None; MAX_LADDER];
                                acted_ids.push(rid);
                                prove_t14_root_flip(&self.voices, rid, Polarity::Long, m, bar);
                                // S11（T14 首尾相连）+ S9（T15）：located 势源根操作流（卖=翻空）
                                // 严格交替 + 价格 zigzag（编排者裁决：located 流非 raw candidate）。
                                prove_s11_s9_located(&mut self.sig_state, Side::Sell, c, s, bar);
                                cleared = true;
                            } else if single_root {
                                // 根在结构基底（root_ladder == FIRST_BSP_LADDER）⇒ 无更低子级别。
                                // **结构上不可达**：F 入场 prove_chain 硬断言 s≥PENDING_LO=
                                // FIRST_BSP_LADDER+1，T5 relabel 只单调上升 ⇒ root_ladder 恒
                                // >FIRST_BSP_LADDER ⇒ 上一 if 必命中。升 unreachable!() 把"不可达"
                                // 从注释声明提为运行时断言（防 PENDING_LO 漂移；no-patch.md：死
                                // 防御分支静默清仓回现金会掩盖 T1 违反，故不静默保留）。
                                unreachable!(
                                    "C-clear-at-base@bar {bar}：root_ladder={root_ladder}==FIRST_BSP_LADDER \
                                     但 N5 保证 root_ladder≥PENDING_LO>FIRST_BSP（PENDING_LO 漂移？）"
                                );
                            }
                            // else 多 voice ⇒ C no-op（子先 D 回补）。
                        }
                    }
                }
                Polarity::Short => {
                    // T14 买点翻多：空→长 in-place（恒仓 m 不变；下跌利润沉淀 free）。
                    // cover m@c + 重新做多 m@c ⇒ free += capital−2×m×c。根空头 MtM：
                    // nav_pre=free+（capital−m×c）→ nav_post=free'+m×c=free+capital−m×c ⇒ 守恒。
                    if let Some(s) = buy_source {
                        if single_root && s >= root_ladder && sig.buy1.get(s) {
                            prove_chain(&self.located_buy, Side::Buy, s, bar, "C-flipback");
                            let m = self.voices[rid].units;
                            let cap = self.voices[rid].capital;
                            // 记空腿 trade（翻空相→翻多相完成；下跌 P&L 归因到此腿）。
                            settle(&mut self.voices, rid, bar, c, "flip_long", &mut self.res);
                            self.free += cap - 2.0 * m * c;
                            self.voices[rid].dir = Polarity::Long;
                            self.voices[rid].basis = c;
                            self.voices[rid].capital = 0.0;
                            self.voices[rid].cost_pool = m * c;
                            self.voices[rid].entry_bar = bar; // 多头相起点
                            self.voices[rid].negate_line = None; // 删否定线：根多头纯买卖点驱动（E降成本/C翻空）
                            self.voices[rid].acted_bar = bar;
                            self.res.n_nrf_root_flips_by_ladder[root_ladder] += 1;
                            self.located_buy = [None; MAX_LADDER];
                            acted_ids.push(rid);
                            prove_t14_root_flip(&self.voices, rid, Polarity::Short, m, bar);
                            // S11（T14 首尾相连）+ S9（T15）：located 势源根操作流（买=翻多）
                            // 严格交替 + 价格 zigzag（编排者裁决：located 流非 raw candidate）。
                            prove_s11_s9_located(&mut self.sig_state, Side::Buy, c, s, bar);
                            cleared = true;
                        }
                    }
                }
            }
        }

        // ── D. 回补（逐活跃非根 voice：自层 confirmed 反向词汇 = 走势完美 ⇒ 隔离
        //    平仓返父。自层信号——不查全局链，N7 邻接）──
        if !cleared {
            let snap: Vec<usize> = (0..self.voices.len()).collect();
            for &id in &snap {
                if !self.voices[id].can_act(bar) || self.voices[id].parent.is_none() {
                    continue;
                }
                let v = &self.voices[id];
                let perfected = match v.dir {
                    Polarity::Short => sig.buy_any.get(v.ladder),
                    Polarity::Long => sig.sell_any.get(v.ladder),
                };
                if perfected {
                    self.voices[id].acted_bar = bar;
                    close_voice(id, bar, c, c, "recover", true, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                    acted_ids.push(id);
                }
            }
        }

        // ── E. 降成本 spawn（N7：逐活跃 voice，自层 nf fire ∨ 根自层 confirmed 卖
        //    ⇒ 释放 θ 配额给子 voice。**不查全局 located 链、不 prove_chain**——
        //    触发层 == voice 层（prove_n7）。N4 纯成本门终止）──
        if !cleared {
            let snap: Vec<usize> = (0..self.voices.len()).collect();
            for &id in &snap {
                if !self.voices[id].can_act(bar) {
                    continue;
                }
                let dir = self.voices[id].dir;
                let ladder = self.voices[id].ladder;
                let is_root = self.voices[id].parent.is_none();
                // T14×T8 会计张力：根空头是叶节点（不嵌套降成本）。根空头用 MtM-external
                // 会计（外部市场负债），而降成本子空头用 frozen-internal（父吸收）——同一
                // voice 不可兼容两套口径（短父 spawn 长子在 MtM 下破坏守恒 +m×c）。故根空头
                // 不 spawn（有效域边界：T8 多空嵌套降成本只在多头相/子空头层，根空头相纯翻转）。
                if is_root && dir == Polarity::Short {
                    // ── 诊断（env UNN_DBG_QQQ）：根空头持仓期间自层买点 → E 为何不 spawn ──
                    if std::env::var_os("UNN_DBG_QQQ").is_some() && sig.buy1.get(ladder) {
                        eprintln!(
                            "[E-skip bar={bar}] root SHORT@ladder{ladder} raw_buy1=T nf_buy@{ladder}={} \
                             — E spawn 跳过（T8 根空头叶节点，根空头相不嵌套降成本；买点本应走 C 翻多）",
                            nf_buy[ladder].is_some()
                        );
                    }
                    continue;
                }
                let (nf_trigger, confirmed_root) = match dir {
                    // 触发 = 自层次级别卖点（nf@ladder 经 rec_sub_evidence 背驰确认，第11环
                    // 买卖点严格形式）∨ 根自层 confirmed 卖（§9"其他卖点走E"）。非根多 voice
                    // 仅 nf 自层触发（grandchild 递归同律）。删否定线后 nf 仅作触发判据，不再
                    // 传作子 negate_line（子 voice 纯买卖点驱动）。
                    Polarity::Long => (nf_sell[ladder].is_some(), is_root && sig.sell_any.get(ladder)),
                    Polarity::Short => (nf_buy[ladder].is_some(), false),
                };
                if nf_trigger || confirmed_root {
                    // N7：触发是自层（nf@ladder ∨ 根 sell_any@ladder）——证明触发层==voice 层。
                    prove_n7_spawn_self_level(ladder, ladder, bar);
                    if try_spawn_cost_gated(id, bar, c, &self.depth_ref, &mut self.voices, &mut self.res) {
                        self.voices[id].acted_bar = bar;
                        acted_ids.push(id);
                    }
                }
            }
        }

        // ── F. 根入场（森林空 ∧ 未清仓本 bar）：买链 source S（pending_locate，N5/N6）
        //    ∧ sig.buy1[S]（type1 底背驰=走势完美建仓）⇒ 在 S 层满仓开多（26课恒仓，
        //    第23环；source S 决定根入场级别，入场后由 root_emergent_ladder 涌现升级 T5）。
        //    F 承载初始入场 + 强平后重建仓（删否定线/观测态：唯一空仓来源是 A 强平根空头/
        //    EOD，遇 F-eligible buy1@located 链顶即重建）。**保持 N5 门控**（buy_source =
        //    located 级联链顶 ≥ move(L1)），不实装裸入场——后者 ⊥ N5（segment 非势源，
        //    prove_chain 硬断言 s≥PENDING_LO；裸扫描入场 = 539 号 constitutive_throughput_falsified）。──
        let any_active = self.voices.iter().any(|v| !matches!(v.status, VoiceStatus::Closed));
        // T1② 立即重建判据：森林空 ∧ F-eligible（buy_source ∧ buy1@s ∧ free>0 有买点+资本）⇒
        // F 必入场。f_eligible 在 F 入场前判定；若 eligible 但 F 后仍空 ⇒ 漏建仓（prove_t1②）。
        let f_eligible = !cleared
            && !any_active
            && buy_source.is_some_and(|s| sig.buy1.get(s))
            && self.free > 0.0
            && (self.free / c).is_finite();
        if !cleared && !any_active {
            if let Some(s) = buy_source {
                if sig.buy1.get(s) {
                    let units = self.free / c;
                    if units > 0.0 && units.is_finite() {
                        prove_chain(&self.located_buy, Side::Buy, s, bar, "F-entry");
                        self.voices.push(VoiceLedger {
                            ladder: s,
                            dir: Polarity::Long,
                            units,
                            basis: c,
                            cost_pool: self.free,
                            capital: 0.0,
                            entry_bar: bar,
                            negate_line: None, // 删否定线：根多头纯买卖点驱动（E降成本/C type1翻转）
                            status: VoiceStatus::Active,
                            parent: None,
                            children: Vec::new(),
                            realized_pnl: 0.0,
                            acted_bar: bar,
                        });
                        self.n_base = units;
                        self.free = 0.0;
                        self.res.n_nrf_root_entries_by_ladder[s] += 1;
                        self.res.n_entries_by_ladder[s] += 1;
                        self.located_buy = [None; MAX_LADDER];
                        // S11（T14 首尾相连）+ S9（T15）：located 势源根操作流（买=入场，走势完美
                        // 序列起点）严格交替 + 价格 zigzag（编排者裁决：located 流非 raw candidate）。
                        prove_s11_s9_located(&mut self.sig_state, Side::Buy, c, s, bar);
                    }
                }
            }
        }

        // ── 必然性运行时证明（每 bar；violation = panic = 验收标准失败）──
        prove_n2_per_voice(&acted_ids, bar);
        let nav_post = nav(&self.voices, self.free, c);
        prove_n8_conservation(&self.voices, self.n_base, nav_pre, nav_post, bar);
        self.max_children_seen = self.max_children_seen.max(prove_n1_forest(&self.voices, bar));

        // 观测：森林规模 + 物理暴露 + 各层视图持有 bar 计数。
        let active_count = self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
        // T1（不主动清仓，第23环恒仓）：①森林非空→空只经 A 强平（被动 root_liquidated），非强平
        // 致空仓=引擎主动清仓=否定线/观测态残留；②强平后空仓 ∧ F-eligible 买点但未重建=漏建仓。
        prove_t1_no_voluntary_exit(
            was_active,
            active_count > 0,
            root_liquidated,
            f_eligible && active_count == 0, // F-eligible 但 bar 末仍空 = 漏建仓（应永远 false）
            bar,
        );
        self.res.nrf_depth_bars[active_count.min(MAX_LADDER - 1)] += 1;
        let mut long_units = 0.0;
        let mut short_units = 0.0;
        for v in self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)) {
            match v.dir {
                Polarity::Long => long_units += v.units,
                Polarity::Short => short_units += v.units,
            }
            self.res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                self.res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }
        if long_units > 0.0 {
            self.res.nrf_phys_long_bars += 1;
        }
        if short_units > 0.0 {
            self.res.nrf_phys_short_bars += 1;
        }

        // equity 采样（周期点；末 bar 由 finish 补——复现批量 `|| i+1==n` 的 OR 语义）。
        if bar % EQUITY_SAMPLE_BARS == 0 {
            self.res.equity.push((bar, nav(&self.voices, self.free, c)));
        }
        self.cur_bar += 1;
    }

    /// 收尾（eod cascade 关根 + N4 反证 + N1 观测）。幂等（重复调用零效果）。
    /// 末 bar equity 补采样复现批量 `|| i+1==n`（仅当 step 未在周期点采过）。
    pub(crate) fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if self.cur_bar > 0 {
            let last_bar = self.cur_bar - 1;
            let c_last = self.last_close;
            // 末 bar 补采样（step 仅采周期点；批量 OR 语义 = 末 bar 必采一次）。
            if last_bar % EQUITY_SAMPLE_BARS != 0 {
                self.res
                    .equity
                    .push((last_bar, nav(&self.voices, self.free, c_last)));
            }
            // eod：cascade 关闭根（单根不变量 ⇒ 关根即清全森林）。
            if let Some(root_id) = self
                .voices
                .iter()
                .position(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none())
            {
                close_voice(
                    root_id, last_bar, c_last, c_last, "eod", false,
                    &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res,
                );
            }
        }
        self.res.final_nav = self.free;
        // ── N4（成本门动态，eod 反证）：floor_stop 计数器恒 0 ⇒ 无固定 floor 终止 ──
        prove_n4_cost_gate(&self.res);
        // N1 观测（非 panic）：记录最大子数（>1 = 森林实证）。
        self.res.nrf_max_children = self.max_children_seen as u64;
        // S9（T15 ~ 状态）观测：located 势源价格 zigzag 违反计数（非 panic——~ 状态不声明为
        // ✓，formalization-validity-domain.md）。n_s9_violations=0 ⇒ located 流经验满足 T15。
        if self.sig_state.n_ops > 0 {
            eprintln!(
                "[信号层 located 势源观测] n_ops={} S9(T15)违反={} | T11 势趋向维 obs={} 衰减={}\
                （S11 交替 panic 守卫；S9/T11 ~观测——T11 势三维=手性ε×力量×趋向[本斜率]）",
                self.sig_state.n_ops, self.sig_state.n_s9_violations,
                self.sig_state.n_trend_obs, self.sig_state.n_trend_decel
            );
        }
        // ── T50（操作频率径向标度律，§8.3，~ 状态）观测：f(k)∝λ^{−k}——清仓（最外圈∝λ^{−K}）
        //    比降成本（内圈∝λ^{−k}）罕见。定性单调核 A₅-独立（观测违反计数），指数律 λ^{−k}
        //    定量是 L2 可证伪（各级 fire 间隔比≈λ）；非 panic（formalization-validity-domain.md）──
        let fires: Vec<u64> = (FIRST_BSP_LADDER..MAX_LADDER)
            .map(|k| self.res.n_nest_fire_sell_by_ladder[k] + self.res.n_nest_fire_buy_by_ladder[k])
            .collect();
        let total_fires: u64 = fires.iter().sum();
        if total_fires > 0 {
            let t50_viol = prove_t50_radial_scaling(&self.res);
            let flips: u64 = self.res.n_nrf_root_flips_by_ladder.iter().sum();
            let spawns: u64 = self.res.n_nrf_spawns_by_ladder.iter().sum();
            eprintln!(
                "[T50 径向标度律观测] confirm fire/层[{FIRST_BSP_LADDER}..{MAX_LADDER})={fires:?} \
                 单调违反={t50_viol} | 根翻转(最外圈∝λ^-K)={flips} 降成本spawn(内圈∝λ^-k)={spawns}\
                （定性核高层罕见；λ 指数律 L2 可证伪，~状态非panic）"
            );
        }
    }

    /// 已累计 trade 数（lib.rs push_bar 切出本 bar 新增）。
    pub(crate) fn n_trades(&self) -> usize {
        self.res.trades.len()
    }

    /// trade 全表只读（lib.rs marshal 本 bar 增量）。
    pub(crate) fn trades(&self) -> &[LayerTrade] {
        &self.res.trades
    }

    /// 结果只读（lib.rs finish → dict）。
    pub(crate) fn result(&self) -> &PositionalResult {
        &self.res
    }

    /// 状态快照：(cur_bar, nav, long_units, short_units, n_active)。nav 用末 close。
    pub(crate) fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        let c = self.last_close;
        let navv = nav(&self.voices, self.free, c);
        let mut long_u = 0.0;
        let mut short_u = 0.0;
        let mut active = 0usize;
        for v in self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)) {
            active += 1;
            match v.dir {
                Polarity::Long => long_u += v.units,
                Polarity::Short => short_u += v.units,
            }
        }
        (self.cur_bar, navv, long_u, short_u, active)
    }

    /// 消费核心取出结果（批量入口 move out）。
    pub(crate) fn into_result(self) -> PositionalResult {
        self.res
    }
}

/// 批量主入口（`PolarityMode::UnifiedNecessity` 经 `run_positional` 分派至此）。
/// 与流式 `UnnStreamCore` 共享 `step`/`finish` ⇒ bit-exact 由构造保证。零参数
/// （`floor_ladder` 仅作结构递归基断言 = FIRST_BSP_LADDER，非操作 floor——N4）。
pub(crate) fn run_unified_necessity(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !tape.has_bsp_events() {
        return Err("unified_necessity 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "unified_necessity 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理）；confirm 递归基证据读 \
             flip_edge（a0 方向翻转沿），缺 dir_flips 行即判据残缺"
                .to_string(),
        );
    }
    let mut core = UnnStreamCore::new(floor_ladder)?;
    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let n = tape.bars.len();
    for i in 0..n {
        let bar = i as i64;
        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        while flip_ptr < flips.len() && flips[flip_ptr].0 == bar {
            let (_, lad, dir) = flips[flip_ptr];
            flip_edge[lad as usize] = Some(dir);
            flip_ptr += 1;
        }
        core.step(&tape.bars[i], &flip_edge);
    }
    core.finish();
    Ok(core.into_result())
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::{BspClass, LadderMask};

    const UNN: PolarityMode = PolarityMode::UnifiedNecessity;

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() { (Some(50.0), Some(60.0)) } else { (None, None) };
        BspEvent { class, seg_idx: 0, confirmed, cs, zd, zg, price }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b.bsp_events.get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
        b
    }

    fn buy1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.buy1 = LadderMask(b.buy1.0 | (1 << lad));
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sell1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    fn sellanypt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热（层 2/3/4 各 SUB_COST_MIN_OBS 个锚）。**T49**：用 Type2（非 type1）预热
    /// ——center_book.ingest 对非 confirmed-type3 事件 kind-无关（Formed/Extended 同样预热
    /// θ），但 Type2 **不污染 type1_hist**（向心 confirm 只取 type1 settle 历史）⇒ 区间套
    /// 不跳级的负样本测试（`located_chain_breaks_if_level_skipped`）可在干净 type1 历史上验证。
    fn warmup234() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 2, ev_full(BspClass::Sell2, false, 0.0, Some(1 + j)));
            b = with_ev(b, 3, ev_full(BspClass::Sell2, false, 0.0, Some(10 + j)));
            b = with_ev(b, 4, ev_full(BspClass::Sell2, false, 0.0, Some(100 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[2][0].zd = Some(50.0);
            rows[2][0].zg = Some(50.5);
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            rows[4][0].zd = Some(50.0);
            rows[4][0].zg = Some(53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    fn run(bars: Vec<BarSig>, dir_flips: Vec<(i64, u8, Direction)>) -> PositionalResult {
        let t = SignalTape { bars, dir_flips: Some(dir_flips), ..Default::default() };
        run_positional(&t, 2, UNN).unwrap()
    }

    /// 构造**向心**区间套买链（T49 读法C）⇒ 根在 source=4 满仓入场。内圈 type1 在 candidate
    /// 之**过去**（已 settle，展开恒等式：内圈在过去）：先 type1 买@2（最内圈，最早 settle）、
    /// type1 买@3（move L1），后 candidate@4 出现（since），再 buy1@4 触发向心 confirm（母线
    /// [2,3] 嵌套贯通，settle bar ≤ since）⇒ located@4 ⇒ F 入场@4。**非前向逐级出现**——内圈
    /// type1 早于 candidate（与旧前向构造相反，T49 几何）。
    fn full_bull_entry() -> (Vec<BarSig>, Vec<(i64, u8, Direction)>) {
        let mut bars = warmup234();
        bars.push(with_ev(bar(94.0), 2, buy1_ev(0.0))); // type1 买@2（最内圈，最早 settle，过去）
        bars.push(with_ev(bar(95.0), 3, buy1_ev(0.0))); // type1 买@3（move L1，settle 晚于@2，仍过去）
        bars.push(with_ev(bar(96.0), 4, ev_full(BspClass::Buy1, false, 86.0, None))); // candidate@4 armed (since)
        let b = buy1pt(bar(97.0), 4); // 下一 bar：since<bar ∧ 向心母线 [2,3] 贯通 ⇒ confirm@4 ⇒ F 入场@4
        bars.push(b);
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(bar(98.0));
        (bars, vec![(evidence_bar, 1, Direction::Up)])
    }

    /// 在 root long@4 之后追加**向心**卖链（T49）⇒ located_sell@4 确认。内圈 type1 卖@2、@3
    /// 在 candidate 之过去（已 settle），后 candidate@4 卖出现 + sell1@4 ⇒ 向心 confirm@4
    /// （母线 [2,3] 嵌套贯通）⇒ C 翻空@4。
    fn append_sell_chain_at4(bars: &mut Vec<BarSig>) {
        bars.push(with_ev(bar(106.0), 2, ev_full(BspClass::Sell1, false, 0.0, None))); // type1 卖@2（最内圈，过去）
        bars.push(with_ev(bar(107.0), 3, ev_full(BspClass::Sell1, false, 0.0, None))); // type1 卖@3（move L1，过去）
        bars.push(with_ev(bar(108.0), 4, ev_full(BspClass::Sell1, false, 110.0, None))); // candidate@4 armed (since)
        let b = sell1pt(bar(103.0), 4); // 下一 bar：向心母线贯通 ⇒ confirm@4 ⇒ C 翻空@4
        bars.push(b);
        bars.push(bar(102.0));
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("unn"), Some(UNN));
        // 缺背驰磁带 ⇒ Err。
        let t = SignalTape {
            bars: vec![with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, UNN).unwrap_err().contains("背驰磁带"));
        // 缺 dir_flips ⇒ Err。
        let t2 = SignalTape {
            bars: vec![with_empty_div(with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None)))],
            dir_flips: None,
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, UNN).unwrap_err().contains("dir_flips"));
    }

    #[test]
    fn n6_root_enters_at_buy_chain_source() {
        // N5/N6：买 pending 级联到 source=4 ∧ buy1@4 ⇒ 根入场@source=4（满仓恒仓）。
        let (bars, flips) = full_bull_entry();
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "买链 source=4 ⇒ 根入场@4");
        let entry = r.trades.iter().find(|t| t.polarity == Polarity::Long).unwrap();
        assert!((entry.shares * entry.entry_price - INITIAL_CAPITAL).abs() < 1e-6, "满仓恒仓");
    }

    #[test]
    fn n6_segment_alone_no_entry() {
        // N5/N6：segment（k=2）单独 candidate ⇒ 不武装 pending ⇒ 无 source ⇒ 不建仓。
        let mut bars = warmup234();
        bars.push(with_ev(bar(95.0), 2, buy1_ev(90.0)));
        bars.push(buy1pt(bar(96.0), 2));
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(bar(97.0));
        let r = run(bars, vec![(evidence_bar, 1, Direction::Up)]);
        assert_eq!(r.n_nrf_root_entries_by_ladder.iter().sum::<u64>(), 0, "segment 非势源 ⇒ 零入场");
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9, "全程现金");
    }

    fn buy1_ev(price: f64) -> BspEvent {
        ev_full(BspClass::Buy1, false, price, None)
    }

    #[test]
    fn n7_sublevel_sell_spawns_cost_reduction() {
        // N7：根@4 入场后，根自层卖 pending source=3（<4）⇒ 不清仓（source<root.ladder
        // ∧ 非 type1@4 路径）；根层 confirmed 卖（sell_any@4 但 sell1@4 不置）⇒ E 降成本
        // spawn 子空@3（自层 nf，不 prove_chain）。
        let (mut bars, mut flips) = full_bull_entry();
        // 武装 nest_sell@4（卖 pending），bi 翻 Down ⇒ confirm@4 ⇒ nf_sell[4] ⇒ E spawn@3。
        // sell_any@4 但 **不** sell1@4 ⇒ C 不触发（type2/3 走 E，§9）。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(sellanypt(bar(104.0), 4)); // sell_any@4（非 type1）
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(bar(104.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "N7：根自层卖 ⇒ E 降成本 spawn 子空@3");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "非清仓（type2/3 走 E）");
    }

    #[test]
    fn type1_full_chain_flips_root() {
        // C（§6 type1 + 区间套递归到底 + T14 根翻空）：root long@4 后时序卖链
        // （type1 卖@3 → type1 卖@2，递归到 cost-gate floor）⇒ located_sell@4 ∧ sell1@4
        // ∧ 单根 ⇒ in-place 翻空@4（长→空 MtM）。
        let (mut bars, flips) = full_bull_entry();
        append_sell_chain_at4(&mut bars);
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_flips_by_ladder[4], 1, "type1@4 + 区间套递归到底 ∧ 单根 ⇒ in-place 翻空@4");
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_short"), "翻空记长腿 trade");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "翻转非清仓");
    }

    #[test]
    fn bidir_root_flip_long_short_long() {
        // T14 双向循环：root long@4 →(时序卖链)→ in-place 翻空@4 →(时序买链)→
        // in-place 翻多@4。验证两次翻转 + N8 守恒 + 根空头 MtM（下跌相利润沉淀）。
        let (mut bars, flips) = full_bull_entry(); // root long@4
        append_sell_chain_at4(&mut bars); // → 翻空@4（价跌到 102）
        // ── 翻多：下跌相后**向心**买链（T49）⇒ located_buy@4 ∧ buy1@4 ⇒ flip 空→长@4 ──
        //    内圈 type1 买@2、@3 在 candidate 之过去（已 settle），后 candidate@4 + buy1@4。
        bars.push(with_ev(bar(89.0), 2, buy1_ev(0.0))); // type1 买@2（最内圈，过去）
        bars.push(with_ev(bar(90.0), 3, buy1_ev(0.0))); // type1 买@3（move L1，过去）
        bars.push(with_ev(bar(91.0), 4, ev_full(BspClass::Buy1, false, 80.0, None))); // candidate@4 armed
        bars.push(buy1pt(bar(92.0), 4)); // 向心母线贯通 ⇒ confirm@4 ⇒ flip 空→长@4
        bars.push(bar(93.0));
        let r = run(bars, flips);
        assert!(r.n_nrf_root_flips_by_ladder[4] >= 2, "两次 in-place 翻转@4（长→空→长）");
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_short"), "长腿记 trade");
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_long"), "空腿记 trade（下跌 P&L 归因）");
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "N8 守恒跑通 final_nav={}", r.final_nav);
    }

    #[test]
    fn t1_no_negate_line_price_drop_is_noop() {
        // 删否定线（编排者 2026-06-15）：根 long@4 入场后价格跌破旧"否定线"水平（c=85/84，
        // 旧 located_buy[4].extreme=86）⇒ **无任何操作**——B 规则已删，否定线不再是操作
        // 触发器（第11环：操作只在买卖点；价跌非买卖点）。根保持多头在场，无 spawn（无次级别
        // 卖点 nf）、无清仓、无翻转。voice 生命周期纯买卖点驱动 ⇒ 永远在场（建仓后恒非空）。
        let (mut bars, flips) = full_bull_entry(); // root long@4（negate_line=None，已删）
        bars.push(bar(85.0)); // c=85 < 旧 negate 86 ⇒ 删否定线后无操作
        bars.push(bar(84.0)); // 价继续下行 ⇒ 仍无操作（无买卖点）
        let r = run(bars, flips);
        // 删否定线：价跌不触发任何操作（无 spawn 对冲、无 negate close、无清仓翻转）。
        assert_eq!(r.n_nrf_spawns_by_ladder.iter().sum::<u64>(), 0, "无次级别卖点 ⇒ 不 spawn（价跌非买卖点）");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt" && t.exit_reason != "negate"
            && t.exit_reason != "flip_short" && t.exit_reason != "negate_observe"),
            "删否定线：价跌不触发清仓/否定/翻转/观测（操作只在买卖点，第11环）");
        // 永远在场：根保持多头持仓到 eod（建仓后恒非空，prove_t1_no_voluntary_exit 守卫）。
        assert!(r.nrf_phys_long_bars >= 2, "根保持多头在场（永远有仓位，否定线删除后价跌不离场）");
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "N8 守恒 final_nav={}", r.final_nav);
    }

    #[test]
    fn n1_forest_multiple_children() {
        // N1：根@4 在两个不同 bar 各响应一个自层 confirmed 卖（sell_any@4，E confirmed_root
        // 路径，§9）⇒ 长出两个 child@3（栈不可能——栈只吃一个）。
        let (mut bars, flips) = full_bull_entry();
        bars.push(sellanypt(bar(105.0), 4)); // 根自层卖 ⇒ E spawn child@3 (#1)
        bars.push(bar(104.0));
        bars.push(sellanypt(bar(106.0), 4)); // 根自层卖（新 bar）⇒ E spawn child@3 (#2)
        bars.push(bar(105.0));
        let r = run(bars, flips);
        assert!(r.n_nrf_spawns_by_ladder[3] >= 2, "森林：根长出 ≥2 child@3（栈只吃一个）");
        assert!(r.nrf_max_children >= 2, "N1 实证：max_children≥2");
    }

    #[test]
    fn n8_conservation_through_recursion() {
        // N8：递归流转每 bar Σunits=N_base + NAV 价值中性（引擎内 prove_n8 panic 守卫；
        // 跑通即通过）。final_nav 有限正。
        let (bars, flips) = full_bull_entry();
        let r = run(bars, flips);
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "final_nav={}", r.final_nav);
    }

    #[test]
    fn n4_no_floor_stop() {
        // N4：成本门纯动态——floor_stop 恒 0（eod assert）。segment@2 降成本目标合法
        // （θ(2) 存在），bi@1 由 θ=None 自然终止（noref_reject）。跑通即 N4 成立。
        let (bars, flips) = full_bull_entry();
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_floor_stops_by_ladder.iter().sum::<u64>(), 0, "N4：零 floor_stop");
    }

    #[test]
    fn n3_type2_arms_window() {
        // N3：type2 卖点（中枢回测确认）武装区间套窗口（无 continue 跳过）⇒ 计入武装。
        let (mut bars, mut flips) = full_bull_entry();
        // Type2 卖@4（candidate，sell1 不置）武装卖窗。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell2, false, 110.0, None)));
        bars.push(bar(104.0));
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(bar(104.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        let r = run(bars, flips);
        assert!(r.n_nest_arms_by_ladder[4] >= 1, "N3：Type2 计入武装（非 continue 跳过）");
    }

    // ════════════ 区间套递归到底（时序）修复验证（编排者 2026-06-15）════════════

    #[test]
    fn located_chain_breaks_if_level_skipped() {
        // 区间套不跳级（T49 向心：母线须逐圈贯通）：root long@4 后向心卖链**跳过 @3**——只给
        // type1 卖@2 在过去（无 type1 卖@3，warmup 用 Type2 ⇒ type1_hist[3][sell] 干净）+
        // candidate@4 + sell1@4 ⇒ 向心回溯在内圈 @3 找不到已 settle type1 ⇒ 母线断 ⇒ 无
        // located ⇒ 不翻空（不跳级——中间级别走势未完美不能跳过）。
        let (mut bars, flips) = full_bull_entry();
        bars.push(with_ev(bar(106.0), 2, ev_full(BspClass::Sell1, false, 0.0, None))); // type1 卖@2（过去），跳过 @3
        bars.push(with_ev(bar(108.0), 4, ev_full(BspClass::Sell1, false, 110.0, None))); // candidate@4 armed
        bars.push(sell1pt(bar(103.0), 4)); // 向心回溯 @3 缺已 settle type1 ⇒ 母线断 ⇒ 不 confirm
        bars.push(bar(102.0));
        let r = run(bars, flips);
        assert_eq!(
            r.n_nrf_root_flips_by_ladder.iter().sum::<u64>(), 0,
            "向心回溯内圈 @3 缺已 settle type1 ⇒ 母线断 ⇒ 无 located ⇒ 不翻空（区间套不跳级）"
        );
    }

    #[test]
    fn fix2_same_bar_no_degenerate_located() {
        // ② 后续走势验证（第29课:52/54）：candidate@4 与 sell1@4 同 bar（since==bar），即使内圈
        //   type1 卖@2/@3 在过去已贯通母线，向心 confirm 的 `since < bar` 守卫仍拦截（无后续走势
        //   bar = 无 ② 验证）⇒ 不武装 located ⇒ 不翻空（degenerate located 被拦）。
        let (mut bars, flips) = full_bull_entry(); // root long@4
        bars.push(with_ev(bar(106.0), 2, ev_full(BspClass::Sell1, false, 0.0, None))); // type1 卖@2（过去）
        bars.push(with_ev(bar(107.0), 3, ev_full(BspClass::Sell1, false, 0.0, None))); // type1 卖@3（过去）
        bars.push(sell1pt(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)), 4)); // candidate@4 + sell1@4 同 bar，无后续 bar
        let r = run(bars, flips);
        assert_eq!(
            r.n_nrf_root_flips_by_ladder.iter().sum::<u64>(), 0,
            "candidate@4 与 sell1@4 同 bar（since==bar）⇒ 向心 confirm `since<bar` 拦截 ⇒ 不翻空（伪确认）"
        );
        assert!(r.nrf_phys_long_bars > 0, "根保持多头（无伪确认翻空）");
    }
}
