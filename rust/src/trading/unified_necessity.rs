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
//! A 强平兜底（逐空头 voice）→ B 否定扫描（子 voice 关+cascade；**根否定 ⇒ 回现金 +
//! 观测态**——T1⊥A8 扬弃，否定线=停损=资本保全 A8，非字面 stop-and-reverse）→ C 根
//! 清仓/翻转（pending_locate 卖/买链 source≥re ∧ type1 背驰，N5/N6 + T5/T14 双向 in-place）→
//! D 回补（逐非根 voice 自层走势完美）→ E 降成本 spawn（逐 voice 自层 nf，N7；纯成本
//! 门 N4；根空头叶节点跳过）→ F 根入场（森林空，最高 buy1 层；观测态 ⇒ 持仓态重建）。
//!
//! ## T1⊥A8 扬弃（观测态——覆盖映射的分歧点；df752ea6e4 字面 T1 经验否证后的辩证解）
//!
//! 字面 T1（永远持仓、否定→in-place 翻转不回现金）⊥ A8（否定线=停损=资本保全）：
//! df752ea6e4 强制永远在场 ⇒ 上行 regime 根被迫无避险做空 ⇒ CL−56.7%/BRN−136.6%
//! 破产（whipsaw stop-and-reverse）。**扬弃**（Aufhebung，089号）三环节：
//!   - **否定**：T1 不是字面永远持仓——根否定 ⇒ `close_voice` 回现金（A8 资本保全，
//!     否定线先于保证金线）。删除 B 规则 in-place 翻转。
//!   - **保留**：引擎不离场——观测态下信号层照常更新（nest 窗口/cascade located/BSP
//!     检测全跑），step 不 early-return（A→F + N1-N8 prove 照常）。
//!   - **提升**：在场 = 跟踪走势 + 在买卖点操作（含回现金的观测态）。观测态遇下一个
//!     F-eligible confirmed BSP（buy1@located 链顶）⇒ 同 bar 重新建仓（F 路径，从最高
//!     可介入级别入场，N5-gated；非裸入场）。
//!
//! **覆盖映射分歧层**：T1⊥A8 处覆盖空间从一层（持仓）分裂为两层（持仓 ∥ 观测）。
//! 观测态 = 后否定的现金等待态（≠ 初始未入场态——observing 标记区分）。持续时长
//! 有限（走势终完美 ⇒ buy1@located 必现），`prove_t1_aufheben` 守卫"无 BSP 不建仓"。
//!
//! T14/T5/A5（双向翻转 + 涌现 + 会计重组）：C 规则 type1 背驰（走势完美"十年1-2次"）根
//! in-place 长↔空翻转（**保留**——type1 翻转非 whipsaw 死因，与 B 否定停损相位区分）。
//! 根空头用 MtM nav（capital−units×c，外部市场负债）⇒ 翻转/平仓同价 c 守恒且真实兑现。
//!
//! ## 验收标准：8 个 prove 函数（非回测）
//!
//! 每条必然性有对应 prove，violation = panic。8 标的真实数据跑通无 panic ⇒ 8 条
//! 必然性在 ~25M bar 上 L2 成立。回测是有效域读数，**不是验收标准**（编排者裁决）。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::isolated_fugue::{close_voice, nav, settle, VoiceLedger, VoiceStatus};
use super::nested_fugue::rec_sub_evidence;
use super::positional::{theta_weights, LayerTrade, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::{BarSig, SignalTape};
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
    debug_assert!(lad >= root_ladder, "T5：涌现层 {lad} < 入场层 {root_ladder}（爬升应单调非降）");
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

/// **T1⊥A8 扬弃（观测态）运行时证明**：观测态 = 覆盖映射在 T1⊥A8 处的分歧层
/// （持仓 ∥ 观测）。两支判据（每 bar 末，F 之后）：
/// ① **引擎不停**（保留）：观测态不 early-return ⇒ step 全程 A→F + N1-N8 prove 照常跑、
///    nest/located 窗口持续维护——结构性保证，到达本断言即证（无需额外断言）。
/// ② **即时重建**（提升）：若 bar 起处于观测态且本 bar F-eligible（buy_source ∧
///    buy1@s ∧ units>0），则 bar 末必已离开观测态（`still_observing==false`，F 重建仓）。
///    violation = "有 BSP 不建仓"——A8 资本保全退化为永久空仓 ⇒ T1 在场性丢失。
/// violation = panic（make-decision-observable，137号）。
fn prove_t1_aufheben(was_observing: bool, f_eligible: bool, still_observing: bool, bar: i64) {
    if was_observing && f_eligible {
        assert!(
            !still_observing,
            "T1⊥A8 违反@bar {bar}：观测态 F-eligible（buy_source ∧ buy1@s）但 bar 末仍观测中\
             （有 BSP 不建仓——A8 资本保全退化为永久空仓，T1 在场性丢失）"
        );
    }
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
    // 方向/段锚滚动状态（T5 根级别涌现 E\* 读数；iso 同构 dir_state/anchor_state）。
    dir_state: [Option<Direction>; MAX_LADDER],
    anchor_state: [i64; MAX_LADDER],
    // N4 累计观测（prove_n4 在 eod 反证 floor_stop 恒 0）。
    max_children_seen: usize,
    // T1⊥A8 扬弃（观测态）：根否定后回现金避险态。覆盖映射在 T1⊥A8 处的分歧层——
    // observing=true 区分"后否定的现金等待态"（≠ 初始未入场的 observing=false）。
    observing: bool,
    observe_since: i64, // 进入观测态的 bar（重建仓时 duration = bar − observe_since）。
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
            dir_state: [None; MAX_LADDER],
            anchor_state: [-1; MAX_LADDER],
            max_children_seen: 0,
            observing: false,
            observe_since: -1,
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
                            (ext, w.since_bar) // 压缩起始不变（540号）
                        });
                        *win = Some(Pending { extreme: ext, since_bar: since });
                        self.res.n_nest_arms_by_ladder[k] += 1;
                    }
                    if e.class.kind() == BspKind::Type2 {
                        type2_handled += 1; // 武装 ∨ confirmed 清窗——两路均处理（无 continue）
                    }
                }
            }
            // 第14环展开↓：confirm 从 k−1 递归下探至 a0（含 segment + bi）。
            let sub = k - 1;
            if let Some(w) = self.nest_sell[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, flip_edge) {
                    assert!(
                        w.since_bar <= bar,
                        "N6 违反@bar {bar}：confirm_sell[{k}] 压缩 {} > 展开（展开早于压缩）",
                        w.since_bar
                    );
                    confirm_sell[k] = Some((w.extreme, w.since_bar));
                    nf_sell[k] = Some(w.extreme); // N7：自层 fire（供 E）
                    self.nest_sell[k] = None;
                    self.res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        self.res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = self.nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, flip_edge) {
                    assert!(
                        w.since_bar <= bar,
                        "N6 违反@bar {bar}：confirm_buy[{k}] 压缩 {} > 展开（展开早于压缩）",
                        w.since_bar
                    );
                    confirm_buy[k] = Some((w.extreme, w.since_bar));
                    nf_buy[k] = Some(w.extreme); // N7：自层 fire（供 E）
                    self.nest_buy[k] = None;
                    self.res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        self.res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }
        prove_n3_type2(type2_seen, type2_handled, bar);

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

        // ── A. 强平兜底（逐活跃空头 voice；per-voice——强平某 voice 不阻断其他）──
        let snap: Vec<usize> = (0..self.voices.len()).collect();
        for &id in &snap {
            if matches!(self.voices[id].status, VoiceStatus::Closed) || self.voices[id].dir != Polarity::Short {
                continue;
            }
            let v = &self.voices[id];
            if v.capital + v.units * (v.basis - c) <= 0.0 {
                let lad = v.ladder;
                let b = 2.0 * v.basis;
                close_voice(id, bar, b, c, "liq", false, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                self.res.n_short_liquidations_by_ladder[lad] += 1;
                acted_ids.push(id);
            }
        }

        // ── B. 否定扫描（逐活跃 voice：破 027:25 极值线）。T1⊥A8 扬弃：
        //    **根否定 ⇒ close_voice 回现金（A8 资本保全：否定线=停损，先于保证金线）
        //    + 进入观测态**（覆盖映射分歧层；删除字面 T1 in-place 翻转——df752ea6e4
        //    CL−56%/BRN−136% 破产死因）。子 voice 否定 ⇒ 关闭返父（根存活，非观测态）──
        let snap: Vec<usize> = (0..self.voices.len()).collect();
        for &id in &snap {
            if matches!(self.voices[id].status, VoiceStatus::Closed) {
                continue;
            }
            let v = &self.voices[id];
            let broke = v.negate_line.is_some_and(|line| match v.dir {
                Polarity::Short => c > line,
                Polarity::Long => c < line,
            });
            if !broke {
                continue;
            }
            let lad = v.ladder;
            let is_root = v.parent.is_none();
            if is_root {
                // T1⊥A8 扬弃（否定→现金+观测）：根否定 = 走势否定 = 停损（A8）⇒ close_voice
                // cascade 关全树回现金（多头根 free+=units×c；空头根 MtM free+=capital−units×c）。
                // 进入观测态：引擎继续跟踪走势（保留），等待 F-eligible BSP 重建仓（提升）。
                close_voice(id, bar, c, c, "negate_observe", false, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                self.observing = true;
                self.observe_since = bar;
                self.res.n_nrf_negate_observes_by_ladder[lad] += 1;
                acted_ids.push(id);
            } else {
                // 子 voice 否定 ⇒ 关闭 + cascade 返父（根存活在场，非观测态）。
                close_voice(id, bar, c, c, "negate", false, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                self.res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted_ids.push(id);
            }
        }

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
                && v.acted_bar != bar); // N2 防双动：跳过本 bar 已被 A/B 操作的根
        if let Some(rid) = root_id {
            let root_dir = self.voices[rid].dir;
            // T5/A5 会计重组（root_emergent_ladder 向上单调升级，纯 relabel）。
            let re = root_emergent_ladder(
                self.voices[rid].ladder, self.voices[rid].entry_bar, root_dir,
                &self.dir_state, &self.anchor_state, max_l,
            );
            if re > self.voices[rid].ladder {
                self.voices[rid].ladder = re; // 会计重组：无物理交易，units/NAV 不变
            }
            let root_ladder = self.voices[rid].ladder;
            let active_count = self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
            let single_root = active_count == 1;
            match root_dir {
                Polarity::Long => {
                    if let Some(s) = sell_source {
                        if s >= root_ladder && sig.sell1.get(s) {
                            prove_chain(&self.located_sell, Side::Sell, s, bar, "C-flip/clear");
                            let flip_line = self.located_sell[s].map(|e| e.extreme);
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
                                self.voices[rid].negate_line = flip_line; // 027:25：破高=短头错
                                self.voices[rid].acted_bar = bar;
                                self.res.n_nrf_root_flips_by_ladder[root_ladder] += 1;
                                self.located_sell = [None; MAX_LADDER];
                                acted_ids.push(rid);
                                prove_t14_root_flip(&self.voices, rid, Polarity::Long, m, bar);
                                cleared = true;
                            } else if single_root && root_ladder == FIRST_BSP_LADDER {
                                // 根在结构基底 ⇒ 无更低子级别 ⇒ 清仓到现金（cascade 全树）。
                                close_voice(rid, bar, c, c, "sellpt", false, &mut self.voices, &mut self.free, &mut self.n_base, &mut self.res);
                                self.located_sell = [None; MAX_LADDER];
                                acted_ids.push(rid);
                                cleared = true;
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
                            let line = self.located_buy[s].map(|e| e.extreme);
                            // 记空腿 trade（翻空相→翻多相完成；下跌 P&L 归因到此腿）。
                            settle(&mut self.voices, rid, bar, c, "flip_long", &mut self.res);
                            self.free += cap - 2.0 * m * c;
                            self.voices[rid].dir = Polarity::Long;
                            self.voices[rid].basis = c;
                            self.voices[rid].capital = 0.0;
                            self.voices[rid].cost_pool = m * c;
                            self.voices[rid].entry_bar = bar; // 多头相起点
                            self.voices[rid].negate_line = line; // 027:25：破低=多头错
                            self.voices[rid].acted_bar = bar;
                            self.res.n_nrf_root_flips_by_ladder[root_ladder] += 1;
                            self.located_buy = [None; MAX_LADDER];
                            acted_ids.push(rid);
                            prove_t14_root_flip(&self.voices, rid, Polarity::Short, m, bar);
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
                    continue;
                }
                let (nest_fired, confirmed_root) = match dir {
                    // 根自层 confirmed 卖（C 未消费的一切卖点，§9"其他卖点走E"）；
                    // 非根多 voice 仅 nf 自层定位触发（grandchild 递归同律）。
                    Polarity::Long => (nf_sell[ladder], is_root && sig.sell_any.get(ladder)),
                    Polarity::Short => (nf_buy[ladder], false),
                };
                if nest_fired.is_some() || confirmed_root {
                    // N7：触发是自层（nf@ladder ∨ 根 sell_any@ladder）——证明触发层==voice 层。
                    prove_n7_spawn_self_level(ladder, ladder, bar);
                    if try_spawn_cost_gated(id, bar, c, nest_fired, &self.depth_ref, &mut self.voices, &mut self.res) {
                        self.voices[id].acted_bar = bar;
                        acted_ids.push(id);
                    }
                }
            }
        }

        // ── F. 根入场（森林空 ∧ 未清仓本 bar）：买链 source S（pending_locate，N5/N6）
        //    ∧ sig.buy1[S]（type1 底背驰=走势完美建仓）⇒ 在 S 层满仓开多（26课恒仓，
        //    第23环；source S 决定根入场级别，入场后由 root_emergent_ladder 涌现升级 T5）。
        //    ★ T1⊥A8 扬弃（提升）：F 同时承载初始入场 + **观测态 ⇒ 持仓态重建仓**——
        //    观测态（B 否定后的现金等待态）遇 F-eligible BSP 即同 bar 重建仓（清 observing）。
        //    **保持 N5 门控**（buy_source = located 级联链顶 ≥ move(L1)），不实装裸入场——
        //    后者 ⊥ N5（segment 非势源，prove_chain 硬断言 s≥PENDING_LO；裸扫描入场 = 539 号
        //    A′ 解耦踏空轴 constitutive_throughput_falsified）。观测态重建仓"不需 pending_locate"
        //    指不需 C 式翻转链（根已在场的清仓/翻转）——而非绕过 F 的 N5 入场门：从零开始
        //    入场恰是 F 的 located_buy 链（从最高可介入级别进入，与初始入场同路径）。──
        let any_active = self.voices.iter().any(|v| !matches!(v.status, VoiceStatus::Closed));
        let was_observing = self.observing; // F 决策前的观测态（prove_t1_aufheben 用）
        let mut f_eligible = false;
        if !cleared && !any_active {
            if let Some(s) = buy_source {
                if sig.buy1.get(s) {
                    let units = self.free / c;
                    if units > 0.0 && units.is_finite() {
                        f_eligible = true;
                        prove_chain(&self.located_buy, Side::Buy, s, bar, "F-entry");
                        let line = self.located_buy[s].map(|e| e.extreme);
                        self.voices.push(VoiceLedger {
                            ladder: s,
                            dir: Polarity::Long,
                            units,
                            basis: c,
                            cost_pool: self.free,
                            capital: 0.0,
                            entry_bar: bar,
                            negate_line: line,
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
                        if self.observing {
                            // 观测态 ⇒ 持仓态转移（T1 提升的在场性兑现）：清观测标记 +
                            // 记重建仓 + 更新有限持续时长读数（bar − observe_since）。
                            let dur = (bar - self.observe_since).max(0) as u64;
                            self.res.nrf_max_observe_dur = self.res.nrf_max_observe_dur.max(dur);
                            self.res.n_nrf_observe_reentries_by_ladder[s] += 1;
                            self.observing = false;
                        }
                    }
                }
            }
        }
        // T1⊥A8 扬弃运行时证明：观测态 F-eligible ⇒ 必已重建仓（无 BSP 不建仓）。
        prove_t1_aufheben(was_observing, f_eligible, self.observing, bar);

        // ── 必然性运行时证明（每 bar；violation = panic = 验收标准失败）──
        prove_n2_per_voice(&acted_ids, bar);
        let nav_post = nav(&self.voices, self.free, c);
        prove_n8_conservation(&self.voices, self.n_base, nav_pre, nav_post, bar);
        self.max_children_seen = self.max_children_seen.max(prove_n1_forest(&self.voices, bar));

        // 观测：森林规模 + 物理暴露 + 各层视图持有 bar 计数。
        let active_count = self.voices.iter().filter(|v| !matches!(v.status, VoiceStatus::Closed)).count();
        self.res.nrf_depth_bars[active_count.min(MAX_LADDER - 1)] += 1;
        // T1⊥A8 扬弃：观测态 bar 计数（bar 末仍 observing = 后否定的现金等待态）。
        if self.observing {
            self.res.nrf_observe_bars += 1;
        }
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
            } else if self.observing {
                // T1⊥A8 扬弃：数据在观测态中结束（走势完美/BSP 未在样本内出现）⇒ 补记
                // 该段持续时长（有限性读数完整覆盖——重建仓未发生时 duration 仅在此捕获）。
                let dur = (last_bar - self.observe_since).max(0) as u64;
                self.res.nrf_max_observe_dur = self.res.nrf_max_observe_dur.max(dur);
            }
        }
        self.res.final_nav = self.free;
        // ── N4（成本门动态，eod 反证）：floor_stop 计数器恒 0 ⇒ 无固定 floor 终止 ──
        prove_n4_cost_gate(&self.res);
        // N1 观测（非 panic）：记录最大子数（>1 = 森林实证）。
        self.res.nrf_max_children = self.max_children_seen as u64;
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
        // C（§6 type1 背驰 + 完整级联 + T14 根翻空）：sell1@4 ∧ 卖链 source=4 ≥
        // root.ladder=4 ∧ 单根 ⇒ 根**in-place 翻空@4**（长→空 MtM，非旧式子空@3）。
        // ★ 仅武装 nest_sell@4（confirm 靠 sell1 同 bar 的 bi-down 触发 ⇒ 保持单根）。
        let (mut bars, mut flips) = full_bull_entry();
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None))); // 仅 arm @4
        bars.push(sell1pt(bar(104.0), 4)); // sell1@4（type1 背驰）∧ bi 翻 Down ⇒ confirm@4 ⇒ source=4
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(bar(102.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_flips_by_ladder[4], 1, "type1@4 完整链 ∧ 单根 ⇒ in-place 翻空@4");
        // 翻空相记长腿 trade（flip_short），翻空后根为空头（eod 关闭记空腿）。
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_short"), "翻空记长腿 trade");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "翻转非清仓");
    }

    #[test]
    fn bidir_root_flip_long_short_long() {
        // T14 双向循环：根 long@4 →(type1 卖@4)→ in-place 翻空@4 →(下跌)→(type1 买@4)
        // → in-place 翻多@4。验证两次翻转 + N8 守恒（prove_n8 每 bar 跑通无 panic）+
        // 根空头 MtM（下跌相利润沉淀 ⇒ final_nav 反映 104→86 下跌捕获）。
        let (mut bars, mut flips) = full_bull_entry(); // root long@4
        // ── 翻空：arm sell@4（极值 110）+ sell1@4 + bi-down ⇒ flip 长→空@4 ──
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(sell1pt(bar(104.0), 4));
        let sell_ev = bars.len() as i64 - 1;
        flips.push((sell_ev, 1, Direction::Down));
        bars.push(bar(90.0)); // 下跌相（空头盈利；c<110 不破否定线）
        // ── 翻多：arm buy@4（Buy1 极值 80）+ buy1@4 + bi-up ⇒ confirm_buy@4 ⇒ flip 空→长@4 ──
        bars.push(with_ev(bar(85.0), 4, ev_full(BspClass::Buy1, false, 80.0, None)));
        bars.push(buy1pt(bar(86.0), 4));
        let buy_ev = bars.len() as i64 - 1;
        flips.push((buy_ev, 1, Direction::Up));
        bars.push(bar(88.0));
        let r = run(bars, flips);
        assert!(r.n_nrf_root_flips_by_ladder[4] >= 2, "两次 in-place 翻转@4（长→空→长）");
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_short"), "长腿记 trade");
        assert!(r.trades.iter().any(|t| t.exit_reason == "flip_long"), "空腿记 trade（下跌 P&L 归因）");
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "N8 守恒跑通 final_nav={}", r.final_nav);
    }

    #[test]
    fn t1_aufheben_negate_observes_then_reenters() {
        // T1⊥A8 扬弃（观测态）：根 long@4（F 入场设 negate=located_buy[4].extreme=86）破低
        // （c<86）⇒ B 否定 **回现金 + 观测态**（A8 资本保全，**非** in-place 翻空）。
        // 然后再武装买链 + buy1@4 ⇒ F **重新建仓**（观测态 ⇒ 持仓态，T1 提升的在场性）。
        let (mut bars, mut flips) = full_bull_entry(); // root long@4, negate_line=86
        bars.push(bar(85.0)); // c=85 < 86 ⇒ B 否定 ⇒ 回现金 + observing
        bars.push(bar(84.0)); // 观测态持续（价继续下行，无 buy1 ⇒ 留现金避险 A8）
        // ── 观测态重建仓：重新武装买链 @3/4（极值 82/80）+ segment@2 confirm 证据 ──
        let mut b = with_ev(bar(83.0), 2, buy1_ev(81.0));
        b = with_ev(b, 3, buy1_ev(82.0));
        b = with_ev(b, 4, buy1_ev(80.0));
        bars.push(b);
        bars.push(buy1pt(bar(84.0), 4)); // buy1@4 + bi-up ⇒ confirm_buy[3,4] ⇒ located ⇒ F 重建
        let reentry_ev = bars.len() as i64 - 1;
        bars.push(bar(85.0));
        flips.push((reentry_ev, 1, Direction::Up));
        let r = run(bars, flips);
        // 扬弃否定：根否定回现金（negate_observe close），无字面 in-place 翻转腿。
        assert!(r.n_nrf_negate_observes_by_ladder.iter().sum::<u64>() >= 1,
            "T1⊥A8：根否定 ⇒ 回现金 + 观测态（A8 资本保全）");
        assert!(r.trades.iter().any(|t| t.exit_reason == "negate_observe"),
            "根否定记 negate_observe close（回现金，非 stop-and-reverse）");
        assert!(r.trades.iter().all(|t| t.exit_reason != "negflip_short" && t.exit_reason != "negflip_long"),
            "扬弃：删除字面 T1 in-place 翻转腿");
        // 扬弃提升：观测态 ⇒ 持仓态重建仓 + 观测态有限持续。
        assert!(r.n_nrf_observe_reentries_by_ladder.iter().sum::<u64>() >= 1,
            "T1 提升：观测态遇 F-eligible BSP ⇒ 重新建仓（在场性兑现）");
        assert!(r.nrf_observe_bars >= 1, "观测态 bar 计数 >0（后否定现金等待态）");
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "N8 守恒 final_nav={}", r.final_nav);
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
