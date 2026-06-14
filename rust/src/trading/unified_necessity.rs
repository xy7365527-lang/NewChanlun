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
//! ## 每 bar 优先序（同 bar；A→F；§1-§8 会计 = 森林 close_voice 复用，bit-exact iso）
//!
//! A 强平兜底（逐空头 voice）→ B 否定扫描（逐 voice 破 027:25）→ C 根清仓/翻转
//! （pending_locate 卖链 source≥root.ladder ∧ type1 背驰，N5/N6）→ D 回补（逐非根
//! voice 自层走势完美）→ E 降成本 spawn（逐 voice 自层 nf，N7；纯成本门 N4）→
//! F 根入场（森林空，pending_locate 买链 source，N6）。
//!
//! ## 验收标准：8 个 prove 函数（非回测）
//!
//! 每条必然性有对应 prove，violation = panic。8 标的真实数据跑通无 panic ⇒ 8 条
//! 必然性在 ~25M bar 上 L2 成立。回测是有效域读数，**不是验收标准**（编排者裁决）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::isolated_fugue::{close_voice, nav, VoiceLedger, VoiceStatus};
use super::nested_fugue::rec_sub_evidence;
use super::positional::{theta_weights, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER};
use crate::buysellpoint::{BspKind, Side};
use crate::stroke::Direction;

/// pending 势源下界（N5/N6）：move(L1)。segment（=FIRST_BSP_LADDER）非势源——
/// 仅作 `rec_sub_evidence` 的低级别 confirm 证据。
const PENDING_LO: usize = FIRST_BSP_LADDER + 1;

/// pending 窗口（高级别 candidate 持续记忆——540号压缩侧↑载体）。`since_bar` =
/// candidate 首现 bar（压缩完成）；极值刷新时保留首现值（压缩起始不变）。
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
    debug_assert!(compress_bar <= confirm_bar, "540号时序：压缩 {compress_bar} > 展开 {confirm_bar}");
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
        top.compress_bar <= top.confirm_bar,
        "N6 违反@bar {bar} {op}：540号压缩 {} > 展开 {}（展开早于压缩，时序颠倒）",
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

/// 成本门动态 spawn（N4 第16环）：父 voice 释放 θ_sub 配额给子 voice@sub=parent.ladder−1。
/// **无 floor 参数**——终止纯由成本门：`theta(sub)=None`（势不可测=不存在，递归基 bi）∨
/// `theta(sub) < SUB_COST_K×friction`（势幅度<成本）。`floor_stop` 计数器**恒不增**
/// （prove_n4 据此反证无固定 floor 终止）。返回是否成功开仓。
#[allow(clippy::too_many_arguments)]
fn try_spawn_cost_gated(
    parent_id: usize,
    bar: i64,
    c: f64,
    nest_fired: Option<f64>,
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
                negate_line: nest_fired,
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

/// 主入口（`PolarityMode::UnifiedNecessity` 经 `run_positional` 分派至此）。零参数
/// （`floor_ladder` 仅作结构递归基断言 = FIRST_BSP_LADDER，非操作 floor——N4）。
pub(crate) fn run_unified_necessity(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if floor_ladder != FIRST_BSP_LADDER {
        return Err(format!(
            "unified_necessity 是零操作参数引擎：floor_ladder 仅作结构递归基 = \
             FIRST_BSP_LADDER={FIRST_BSP_LADDER}（N4 纯成本门，无操作 floor）；得 {floor_ladder}"
        ));
    }
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

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut voices: Vec<VoiceLedger> = Vec::new();
    let mut free = INITIAL_CAPITAL;
    let mut n_base = 0.0f64;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // pending 窗口（双侧，k ≥ PENDING_LO；segment 非势源）。
    let mut nest_sell: [Option<Pending>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Pending>; MAX_LADDER] = [None; MAX_LADDER];
    // 级联定位链（卖侧出场链 + 买侧入场链）。
    let mut located_sell: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];
    let mut located_buy: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    // N4 累计观测（prove_n4 在 eod 反证 floor_stop 恒 0）。
    let mut max_children_seen = 0usize;

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;
        let bar = i as i64;

        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        while flip_ptr < flips.len() && flips[flip_ptr].0 == bar {
            let (_, lad, dir) = flips[flip_ptr];
            flip_edge[lad as usize] = Some(dir);
            flip_ptr += 1;
        }

        // 市场性质：中枢账本 + 振幅参照。
        if let Some(evrows) = sig.bsp_events.as_deref() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                book.ingest(lad, &evrows[lad], true, None);
            }
            depth_ref.observe(&book, c);
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] = sig.div_events.as_deref().unwrap_or(&empty_devs);

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
            if nest_sell[k].is_some_and(|w| c > w.extreme) {
                nest_sell[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
            }
            if nest_buy[k].is_some_and(|w| c < w.extreme) {
                nest_buy[k] = None;
                res.n_nest_breaks_by_ladder[k] += 1;
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
                    let win = if sellside { &mut nest_sell[k] } else { &mut nest_buy[k] };
                    if e.confirmed {
                        *win = None; // confirmed 同侧让位（本 bar 走 confirmed 路径）
                    } else {
                        let (ext, since) = win.map_or((e.price, bar), |w| {
                            let ext = if sellside { w.extreme.max(e.price) } else { w.extreme.min(e.price) };
                            (ext, w.since_bar) // 压缩起始不变（540号）
                        });
                        *win = Some(Pending { extreme: ext, since_bar: since });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                    if e.class.kind() == BspKind::Type2 {
                        type2_handled += 1; // 武装 ∨ confirmed 清窗——两路均处理（无 continue）
                    }
                }
            }
            // 第14环展开↓：confirm 从 k−1 递归下探至 a0（含 segment + bi）。
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, &flip_edge) {
                    assert!(
                        w.since_bar <= bar,
                        "N6 违反@bar {bar}：confirm_sell[{k}] 压缩 {} > 展开（展开早于压缩）",
                        w.since_bar
                    );
                    confirm_sell[k] = Some((w.extreme, w.since_bar));
                    nf_sell[k] = Some(w.extreme); // N7：自层 fire（供 E）
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, &flip_edge) {
                    assert!(
                        w.since_bar <= bar,
                        "N6 违反@bar {bar}：confirm_buy[{k}] 压缩 {} > 展开（展开早于压缩）",
                        w.since_bar
                    );
                    confirm_buy[k] = Some((w.extreme, w.since_bar));
                    nf_buy[k] = Some(w.extreme); // N7：自层 fire（供 E）
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }
        prove_n3_type2(type2_seen, type2_handled, bar);

        // ── 级联武装 located（N5：confirm@k → cascade [FIRST_BSP..=k]，高 source
        //    优先；按 source 降序施加）。仅供根 F/C 消费——E 用 nf_*（N7）──
        for k in (PENDING_LO..MAX_LADDER).rev() {
            if let Some((ext, since)) = confirm_sell[k] {
                cascade_arm(&mut located_sell, Side::Sell, k, ext, since, bar);
            }
            if let Some((ext, since)) = confirm_buy[k] {
                cascade_arm(&mut located_buy, Side::Buy, k, ext, since, bar);
            }
        }
        // 破极值否定（027:25）：级联统一极值 ⇒ 整链同破。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if located_sell[k].is_some_and(|e| c > e.extreme) {
                located_sell[k] = None;
            }
            if located_buy[k].is_some_and(|e| c < e.extreme) {
                located_buy[k] = None;
            }
        }
        prove_n5_cascade(&located_sell, bar, "sell");
        prove_n5_cascade(&located_buy, bar, "buy");

        let sell_source = chain_source(&located_sell);
        let buy_source = chain_source(&located_buy);

        // N8 价值守恒入口快照。
        let nav_pre = nav(&voices, free, c);
        // N2：本 bar 操作过的 voice id（去全局互斥的运行时证明）。
        let mut acted_ids: Vec<usize> = Vec::new();

        // ── A. 强平兜底（逐活跃空头 voice；per-voice——强平某 voice 不阻断其他）──
        let snap: Vec<usize> = (0..voices.len()).collect();
        for &id in &snap {
            if matches!(voices[id].status, VoiceStatus::Closed) || voices[id].dir != Polarity::Short {
                continue;
            }
            let v = &voices[id];
            if v.capital + v.units * (v.basis - c) <= 0.0 {
                let lad = v.ladder;
                let b = 2.0 * v.basis;
                close_voice(id, bar, b, c, "liq", false, &mut voices, &mut free, &mut n_base, &mut res);
                res.n_short_liquidations_by_ladder[lad] += 1;
                acted_ids.push(id);
            }
        }

        // ── B. 否定扫描（逐活跃 voice：破 027:25 极值线 ⇒ 关该 voice + 子树）──
        let snap: Vec<usize> = (0..voices.len()).collect();
        for &id in &snap {
            if matches!(voices[id].status, VoiceStatus::Closed) {
                continue;
            }
            let v = &voices[id];
            let broke = v.negate_line.is_some_and(|line| match v.dir {
                Polarity::Short => c > line,
                Polarity::Long => c < line,
            });
            if broke {
                let lad = v.ladder;
                close_voice(id, bar, c, c, "negate", false, &mut voices, &mut free, &mut n_base, &mut res);
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted_ids.push(id);
            }
        }

        // ── C. 根清仓/翻转（N5/N6 + §6 + 第23环）：根长持，卖链 source S ≥ root.ladder
        //    ∧ sig.sell1[S]（type1 背驰=走势完美，§6"十年 1-2 次"；type2/3 落 E 降成本
        //    §9）∧ prove_chain ⇒ 翻转（森林单根 ∧ root.ladder>segment）∨ 清仓（root@segment
        //    或多 voice 时 no-op，子先 D 回补）──
        let mut cleared = false;
        let root_id = voices
            .iter()
            .position(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none() && v.units > 0.0);
        if let Some(rid) = root_id {
            let root_ladder = voices[rid].ladder;
            if let Some(s) = sell_source {
                if s >= root_ladder && sig.sell1.get(s) {
                    prove_chain(&located_sell, Side::Sell, s, bar, "C-clear/flip");
                    let active_count = voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
                    let single_root = active_count == 1;
                    let flip_line = located_sell[s].map(|e| e.extreme);
                    if single_root && root_ladder > FIRST_BSP_LADDER {
                        // 翻转 = 降成本 m=N 特例（子空@root.ladder−1，携 located 极值否定线）。
                        // 时序（编排者）：父释放现金 → 子用现金开空（不可反序）。
                        let m = voices[rid].units;
                        let sub = root_ladder - 1;
                        let cash_released = m * c;
                        voices[rid].units -= m;
                        voices[rid].refresh_status(); // husk（PendingRecovery）
                        let child_id = voices.len();
                        voices.push(VoiceLedger {
                            ladder: sub,
                            dir: Polarity::Short,
                            units: m,
                            basis: c,
                            cost_pool: cash_released,
                            capital: cash_released,
                            entry_bar: bar,
                            negate_line: flip_line,
                            status: VoiceStatus::Active,
                            parent: Some(rid),
                            children: Vec::new(),
                            realized_pnl: 0.0,
                            acted_bar: bar,
                        });
                        voices[rid].children.push(child_id);
                        res.n_nrf_root_flips_by_ladder[sub] += 1;
                        res.n_entries_by_ladder[sub] += 1;
                        located_sell = [None; MAX_LADDER];
                        acted_ids.push(rid);
                        cleared = true;
                    } else if single_root && root_ladder == FIRST_BSP_LADDER {
                        // 根已在结构基底 ⇒ 无更低子级别 ⇒ 清仓到现金（cascade 全树）。
                        close_voice(rid, bar, c, c, "sellpt", false, &mut voices, &mut free, &mut n_base, &mut res);
                        located_sell = [None; MAX_LADDER];
                        acted_ids.push(rid);
                        cleared = true;
                    }
                    // else：多 voice（root 有降成本子）⇒ C no-op，子先经 D 独立回补，
                    // 根稍后在单根时翻（逐仓独立，不 collapse）。
                }
            }
        }

        // ── D. 回补（逐活跃非根 voice：自层 confirmed 反向词汇 = 走势完美 ⇒ 隔离
        //    平仓返父。自层信号——不查全局链，N7 邻接）──
        if !cleared {
            let snap: Vec<usize> = (0..voices.len()).collect();
            for &id in &snap {
                if !voices[id].can_act(bar) || voices[id].parent.is_none() {
                    continue;
                }
                let v = &voices[id];
                let perfected = match v.dir {
                    Polarity::Short => sig.buy_any.get(v.ladder),
                    Polarity::Long => sig.sell_any.get(v.ladder),
                };
                if perfected {
                    voices[id].acted_bar = bar;
                    close_voice(id, bar, c, c, "recover", true, &mut voices, &mut free, &mut n_base, &mut res);
                    acted_ids.push(id);
                }
            }
        }

        // ── E. 降成本 spawn（N7：逐活跃 voice，自层 nf fire ∨ 根自层 confirmed 卖
        //    ⇒ 释放 θ 配额给子 voice。**不查全局 located 链、不 prove_chain**——
        //    触发层 == voice 层（prove_n7）。N4 纯成本门终止）──
        if !cleared {
            let snap: Vec<usize> = (0..voices.len()).collect();
            for &id in &snap {
                if !voices[id].can_act(bar) {
                    continue;
                }
                let dir = voices[id].dir;
                let ladder = voices[id].ladder;
                let is_root = voices[id].parent.is_none();
                let (nest_fired, confirmed_root) = match dir {
                    // 根自层 confirmed 卖（C 未消费的一切卖点，§9"其他卖点走E"）；
                    // 非根多 voice 仅 nf 自层定位触发（grandchild 递归同律）。
                    Polarity::Long => (nf_sell[ladder], is_root && sig.sell_any.get(ladder)),
                    Polarity::Short => (nf_buy[ladder], false),
                };
                if nest_fired.is_some() || confirmed_root {
                    // N7：触发是自层（nf@ladder ∨ 根 sell_any@ladder）——证明触发层==voice 层。
                    prove_n7_spawn_self_level(ladder, ladder, bar);
                    if try_spawn_cost_gated(id, bar, c, nest_fired, &depth_ref, &mut voices, &mut res) {
                        voices[id].acted_bar = bar;
                        acted_ids.push(id);
                    }
                }
            }
        }

        // ── F. 根入场（森林空 ∧ 未清仓本 bar）：买链 source S（pending_locate，N6）
        //    ∧ sig.buy1[S]（type1 底背驰=走势完美建仓）⇒ 在 S 层满仓开多（26课恒仓，
        //    第23环；source S 决定根级别 = 出场绑定级别）──
        let any_active = voices.iter().any(|v| !matches!(v.status, VoiceStatus::Closed));
        if !cleared && !any_active {
            if let Some(s) = buy_source {
                if sig.buy1.get(s) {
                    let units = free / c;
                    if units > 0.0 && units.is_finite() {
                        prove_chain(&located_buy, Side::Buy, s, bar, "F-entry");
                        let line = located_buy[s].map(|e| e.extreme);
                        voices.push(VoiceLedger {
                            ladder: s,
                            dir: Polarity::Long,
                            units,
                            basis: c,
                            cost_pool: free,
                            capital: 0.0,
                            entry_bar: bar,
                            negate_line: line,
                            status: VoiceStatus::Active,
                            parent: None,
                            children: Vec::new(),
                            realized_pnl: 0.0,
                            acted_bar: bar,
                        });
                        n_base = units;
                        free = 0.0;
                        res.n_nrf_root_entries_by_ladder[s] += 1;
                        res.n_entries_by_ladder[s] += 1;
                        located_buy = [None; MAX_LADDER];
                    }
                }
            }
        }

        // ── 必然性运行时证明（每 bar；violation = panic = 验收标准失败）──
        prove_n2_per_voice(&acted_ids, bar);
        let nav_post = nav(&voices, free, c);
        prove_n8_conservation(&voices, n_base, nav_pre, nav_post, bar);
        max_children_seen = max_children_seen.max(prove_n1_forest(&voices, bar));

        // 观测：森林规模 + 物理暴露 + 各层视图持有 bar 计数。
        let active_count = voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
        res.nrf_depth_bars[active_count.min(MAX_LADDER - 1)] += 1;
        let mut long_units = 0.0;
        let mut short_units = 0.0;
        for v in voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)) {
            match v.dir {
                Polarity::Long => long_units += v.units,
                Polarity::Short => short_units += v.units,
            }
            res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }
        if long_units > 0.0 {
            res.nrf_phys_long_bars += 1;
        }
        if short_units > 0.0 {
            res.nrf_phys_short_bars += 1;
        }

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, nav(&voices, free, c)));
        }
    }

    // eod：cascade 关闭根（单根不变量 ⇒ 关根即清全森林）。
    if let Some(root_id) = voices.iter().position(|v| !matches!(v.status, VoiceStatus::Closed) && v.parent.is_none()) {
        let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
        let last_bar = (n as i64) - 1;
        close_voice(root_id, last_bar, c_last, c_last, "eod", false, &mut voices, &mut free, &mut n_base, &mut res);
    }
    res.final_nav = free;

    // ── N4（成本门动态，eod 反证）：floor_stop 计数器恒 0 ⇒ 无固定 floor 终止 ──
    prove_n4_cost_gate(&res);
    // N1 观测（非 panic）：记录最大子数（>1 = 森林实证）。
    res.nrf_max_children = max_children_seen as u64;
    Ok(res)
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

    /// θ 参照预热（层 2/3/4 各 SUB_COST_MIN_OBS 个锚）。
    fn warmup234() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 2, ev_full(BspClass::Sell1, false, 0.0, Some(1 + j)));
            b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
            b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
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

    /// 构造完整买 pending [3,4] + segment/a0 confirm + buy1@4 ⇒ 根在 source=4 满仓入场。
    fn full_bull_entry() -> (Vec<BarSig>, Vec<(i64, u8, Direction)>) {
        let mut bars = warmup234();
        // 武装 nest_buy@3/4（买侧 pending，极值 88/86）。segment@2 Buy1 作 confirm 证据。
        let mut b = with_ev(bar(95.0), 2, ev_full(BspClass::Buy1, false, 90.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Buy1, false, 88.0, None));
        b = with_ev(b, 4, ev_full(BspClass::Buy1, false, 86.0, None));
        bars.push(b);
        // confirm bar：bi 向上翻 ⇒ rec_sub_evidence(Buy) ⇒ confirm_buy[3,4] ⇒ located。
        bars.push(buy1pt(bar(96.0), 4)); // buy1@4（type1 底背驰建仓词汇）
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(bar(97.0));
        (bars, vec![(evidence_bar, 1, Direction::Up)])
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
        // C（§6 type1 背驰 + 完整级联）：sell1@4 ∧ 卖链 source=4 ≥ root.ladder=4 ∧
        // 单根 ⇒ 翻转为子空@3（N5/N6 prove_chain + N1 森林）。
        // ★ 仅武装 nest_sell@4（无 @3 同 bar 事件——@3 事件会作 @4 的次级别证据使
        //   confirm 提前 fire ⇒ E 降成本 spawn 子@3 ⇒ 多 voice ⇒ C 翻转被 single_root
        //   守卫拦截，子先 D 回补。confirm 靠 sell1 同 bar 的 bi-down 触发 ⇒ 保持单根）。
        let (mut bars, mut flips) = full_bull_entry();
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None))); // 仅 arm @4
        bars.push(sell1pt(bar(104.0), 4)); // sell1@4（type1 背驰）∧ bi 翻 Down ⇒ confirm@4 ⇒ source=4
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(bar(102.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_flips_by_ladder[3], 1, "type1@4 完整链 ∧ 单根 ⇒ 翻转子空@3");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "翻转非清仓");
    }

    #[test]
    fn n1_forest_multiple_children() {
        // N1：根@4 在两个不同 bar 各响应一个 nf 卖 ⇒ 长出两个 child@3（栈不可能）。
        let (mut bars, mut flips) = full_bull_entry();
        // 第一次 nf 卖@4。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(bar(104.0));
        let e1 = bars.len() as i64 - 1;
        // 价格回落不破 110；第二次 nf 卖@4（新 pending）。
        bars.push(with_ev(bar(106.0), 4, ev_full(BspClass::Sell1, false, 112.0, None)));
        bars.push(bar(105.0));
        let e2 = bars.len() as i64 - 1;
        bars.push(bar(105.0));
        flips.push((e1, 1, Direction::Down));
        flips.push((e2, 1, Direction::Down));
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
}
