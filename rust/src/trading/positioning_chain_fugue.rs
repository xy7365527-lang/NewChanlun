//! positioning_chain_fugue — 区间套定位链驱动赋格（mode = "pcf"）。
//!
//! 设计源头：编排者 2026-06-14"那你要实现啊" → "他某种意义上是必然递归的，
//! 你来实装，严格实装" → 2026-06-14"那你打算怎么办？"→"我去实装"（pending_locate
//! 时序重构）。把 `nested_interval_fugue` 的**固定 `min_trade_ladder`** 替换为
//! **走势结构动态决定的操作层级**。固定参数不是区间套；区间套（第14环）是高级别
//! 买卖点由**低级别精确定位**。
//!
//! ## 坍缩根因 → pending_locate 时序重构（本次实装）
//!
//! 旧 pcf（自上而下级联，已坍缩）：对**每个** `k≥FIRST_BSP_LADDER` 的 nf
//! （candidate@k active ∧ rec_sub_evidence(k−1) confirm）调用 `cascade_arm`。
//! 问题在 **segment（k=FIRST_BSP_LADDER=2）**：segment candidate 频繁武装 ∧
//! 其次级别证据（a0 翻转沿）频繁 ⇒ `nf[2]` 频繁触发 ⇒ `cascade_arm(located, 2)`
//! ⇒ source=2。`chain_source` 取最高 located，当只有 source=2 武装时坍缩到
//! segment（真实数据 85-91%，`project_pcf_1s_a0_source_collapse`）。这违背区间套
//! ——区间套要求操作绑定**高级别势**（势大→仓位大→长持），segment 只提供定位精度。
//!
//! **pending_locate 重构（先势后定位的正确时序）**：
//!   - **pending（势）= 高级别 candidate**（k ≥ FIRST_BSP_LADDER+1 = move(L1)
//!     及以上）的持续记忆（`nest_*` 窗口承载，candidate 武装即在场，破极值否定）。
//!     **segment（k=FIRST_BSP_LADDER）不注册 pending**——它不是势源。
//!   - **confirm（定位）= 低级别反转**（`rec_sub_evidence(S−1)` 递归到 a0，覆盖
//!     segment + bi）。confirm 的级别恒 < pending 级别 S（区间套：低定位高）。
//!   - **located 武装 = pending confirm**：最高 active pending@S（≥3）经低级别
//!     confirm ⇒ 级联武装 `located[FIRST_BSP_LADDER..=S]`，source=S。**segment
//!     的反转只作 confirm 证据，不独立武装 located** ⇒ source 永不坍缩到 segment。
//!
//! 旧 pcf 的"时序反了"（低级别 nf 先操作、高级别 candidate 后到）的精确形式 =
//! segment 的 `nf[2]` 抢先武装 source=2。重构后 segment 退出势源，时序回正：
//! pending（高级别势）持续等待 → confirm（低级别定位）兑现 ⇒ source=高级别。
//!
//! ## 级联不变量（必然性检验的运行时基础）
//!
//! `located`（任一侧）非空时**恒为连续前缀** `[FIRST_BSP_LADDER..=S]`，且全层**统一
//! 极值 E（源层 027:25 否定线）+ 统一 source_ladder=S ≥ FIRST_BSP_LADDER+1**。
//! 由两条构造规则保证：
//!   ① **级联写统一极值**：confirm@S 触发 ⇒ `[FIRST_BSP_LADDER..=S]` 全写源层 S
//!      的极值（含 located[segment]——segment 被高级别定位**覆盖**，自身不发起）。
//!   ② **高 source 优先**：仅当新 source ≥ 既有 source 时覆盖（高级别反转主导，
//!      不被后来的低级别 candidate 降级）。
//! ⇒ 破极值（`c > E`，统一极值）**整链同破**（清空或不变，无逐层断裂）。
//! ⇒ `∀k∈[FIRST_BSP_LADDER..=S] located[k].source_ladder == S ≥ FIRST_BSP_LADDER+1`。
//!
//! ## 与 URS（unified_recursive）的唯一构成性差异：层选择机制
//!
//! | 机制 | URS | PCF（本引擎） |
//! |------|-----|--------------|
//! | 入场层 | `top` = 最高 θ 涌现层（与定位链无关） | `buy_source` = pending confirm 链顶 S |
//! | 出场层 | `E*` = dir/anchor 爬升（独立部件） | `sell_source` = pending confirm 链顶 S |
//! | 降成本/出场分界 | E\* vs floor | `source` ≷ `voice.ladder`（势在该层之上=出场，之下=降成本） |
//! | located 武装方向 | 自下而上（全层同时对齐） | **pending（高级别势）+ confirm（低级别定位）** |
//! | 操作参数 | floor_ladder（结构常量） | **零**（source 由链动态产出，segment 不入势源） |
//!
//! ## 操作语义（source 驱动，第13/14/15/16/17/23环）
//!
//! 每 bar 同 bar 单事件（A→B→C→D→E→F；§1-§8 会计 = nested_fugue 原语逐字复用）：
//! - **A 强平兜底 / B 否定扫描**：URS 逐字（027:25 否定线）。
//! - **C 根出场/翻转**（第23环）：根 voice 长持，`sell_source = Some(S)` 且
//!   `S ≥ root.ladder`（势在**入场级别或更高**反转 ⇒ 出场对齐 source）∧ len==1 ⇒
//!   翻转 = 降成本 m=N 特例（子空@root.ladder−1，携 located 极值否定线）。
//!   `root.ladder == FIRST_BSP_LADDER`（无更低子级别）⇒ 清仓到现金。len>1（有
//!   降成本子）⇒ 不翻（子先经 D 独立回补，逐仓独立）。
//! - **D 尾回补**（第17环）：len>1，尾 voice 的反向链 source `S ≥ tail.ladder`
//!   （势在尾级别或更高反转回来 = 子级别走势完美）⇒ 平子返父。
//! - **E 降成本 spawn**（第15/16/17环）：尾 voice 的反向链 source `S < tail.ladder`
//!   （**次级别**反转 = 区间套定位到更低层）⇒ 在 S 层开反向子，量 = θ_S 配额
//!   （高级别势大→大仓位）。`source ≥ FIRST_BSP_LADDER+1` ⇒ 子最低 @move(L1)
//!   （segment 不入势源 ⇒ 降成本止于 move(L1)，segment 仅 confirm）。
//! - **F 根入场**（第23环）：链空，`buy_source = Some(S)` ⇒ 在 S 层满仓开多
//!   （26课恒仓——根=基；source S 决定出场绑定级别）。
//!
//! **"没有被定位链定位的 BSP = 不操作"**（成本门自动终止，第16环）：一切新 voice
//! 创建（F/C/D/E）都被 `chain_source` 门控——无完整链即无 source 即无操作。固定
//! floor 不存在。**segment 单独的反转不创建任何 voice**（必然性检验1/3）。
//!
//! ## 谱系引用（强制——本机制与 539号已否证机制同族，必须区分）
//!
//! 539号 A′ 合取选层 `top* = max{k : sell1[k] ∧ located[k]}`（**单层** located
//! 选清仓层）已 L3 否证（强牛高层 located 频繁 ⇒ 过度清仓 ⇒ 踏空，
//! `project_constitutive_throughput_falsified`）。PCF **不是** A′ 重演：
//!   (a) PCF 要求**整条链** [segment..S] 全 located（pending confirm 级联，
//!       A′ 仅单层）；
//!   (b) PCF **出场对齐入场 source**（voice 持到 source 级反向链，A′ 无入场-出场
//!       级别绑定）。强牛中高层卖链罕见完成 ⇒ source 级反向链不来 ⇒ 不清仓
//!       （踏空机制被 source 绑定堵住——这是 PCF 的待检验假设，非已结算结论）。
//! 信号层（buysellpoint.rs）零改动；会计层（守恒律）零改动——守恒 violation = panic。

use super::center_book::CenterBook;
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::{DepthRef, DEPTH_REF_WINDOW};
use super::nested_fugue::{nav, pop_tail, rec_sub_evidence, unwind_to, Voice, Win};
use super::positional::{theta_weights, PositionalResult, EQUITY_SAMPLE_BARS};
use super::positional_fusion::{SUB_COST_K, SUB_FRICTION_RT};
use super::tape::SignalTape;
use super::types::{BspClass, BspEvent, DivEvent, Polarity, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER};
use crate::buysellpoint::Side;
use crate::stroke::Direction;

/// pending_locate 兑现条目（编排者 pending_locate 方案的核心数据结构）。
///
/// 用户方案的 `PendingLocate{source_ladder, direction, extreme, arm_bar}`：
/// pending（高级别 candidate 持续记忆）由 `nest_*` 窗口承载（candidate 在场即
/// pending 在场，破极值否定 = pending 失效）；**confirm（低级别定位）兑现后**写入
/// `located`，此结构即"已定位的 pending 请求"。
///   - `source_ladder` = pending 的级别（≥ FIRST_BSP_LADDER+1，必然性检验1）。
///     级联不变量下，一条链内全层 source_ladder 统一 = 链顶 S。
///   - `direction` = pending 方向（卖侧出场链 / 买侧入场链）。located_sell/located_buy
///     分数组下方向由数组隐含，此字段为自描述 + 必然性检验的显式载体。
///   - `extreme` = 027:25 否定线（价格破之则定位失效）。级联下全层统一 = 源层极值。
///   - `arm_bar` = 本层 confirm 兑现 bar（链龄诊断 + 因果必然性检验）。
#[derive(Debug, Clone, Copy)]
struct PendingLocate {
    extreme: f64,
    source_ladder: usize,
    direction: Side,
    arm_bar: i64,
}

/// pending confirm 兑现的级联武装（第14环严格形式）：最高 active pending@source
/// 经低级别 confirm ⇒ 级联武装 `located[FIRST_BSP_LADDER..=source]` 全层，统一
/// 极值 = 源层 027:25 否定线，统一 source_ladder=source，统一 direction=dir。
///
/// **调用前提**：`source ≥ FIRST_BSP_LADDER+1`（segment 不入势源——pending 只在
/// move(L1) 及以上注册）。级联仍写到 `located[FIRST_BSP_LADDER]`（segment 被高级别
/// 定位**覆盖**，作为链底定位记忆，但其 source_ladder 指向高级别 S，非自身）。
///
/// **高 source 优先**（级联不变量 ②）：仅当 source ≥ 既有 source_ladder（或该层
/// 空）时覆盖——高级别反转主导，不被后来的低级别 candidate 降级。多个 confirm 同
/// bar 触发时由调用方按 source 降序施加，保证最高 source 先占位。
fn cascade_arm(
    located: &mut [Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    source: usize,
    extreme: f64,
    bar: i64,
) {
    debug_assert!(
        source > FIRST_BSP_LADDER,
        "pending 只在 move(L1) 及以上注册（segment 非势源）；source={source}"
    );
    for slot in located.iter_mut().take(source + 1).skip(FIRST_BSP_LADDER) {
        let overwrite = slot.map_or(true, |e| source >= e.source_ladder);
        if overwrite {
            *slot = Some(PendingLocate {
                extreme,
                source_ladder: source,
                direction: dir,
                arm_bar: bar,
            });
        }
    }
}

/// 级联链顶 S（pending confirm 链顶）= 最高有 located 的层。级联不变量保证 located
/// 非空时恒为连续前缀 `[FIRST_BSP_LADDER..=S]` ⇒ 找到最高层即得完整链顶。
///
/// 这是 PCF 的唯一层选择机制——替代 URS 的 `root_emergent_ladder`(E\*) +
/// `top`(最高 θ 层) 两个独立部件。层选择 ≡ 链确认（合一）。
fn chain_source(located: &[Option<PendingLocate>; MAX_LADDER]) -> Option<usize> {
    (FIRST_BSP_LADDER..MAX_LADDER).rev().find(|&k| located[k].is_some())
}

/// 必然性检验（第14环严格形式的**运行时证明**）：证明一个操作的 source 是一条
/// **操作前已完整形成**的定位链顶层，且 source 严格高于 segment（pending_locate
/// 必然性检验1：建仓 source > segment）。F/C/D/E 每个操作点调用——违反即 panic。
/// 消费 `PendingLocate` 全字段（声明=能力——故非声明膨胀）。
///
/// 四项独立证明（与 `chain_source` 重算，非循环——证明级联不变量在操作点成立）：
/// ① `source > FIRST_BSP_LADDER`（segment 非势源——必然性检验1）；
/// ② located[s] 在场且 `source_ladder == s ∧ direction == dir`（s 是链顶，级联统
///    一 source/方向正确）；③ `arm_bar ≤ bar`（因果：链在操作前 confirm，无未来
///    定位）；④ `[FIRST_BSP_LADDER..=s]` 全 located（级联连续前缀——"没有定位链
///    的交易 = bug"的逐操作硬断言）。
fn prove_chain(
    located: &[Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    s: usize,
    bar: i64,
    op: &str,
) {
    assert!(
        s > FIRST_BSP_LADDER,
        "必然性违反@bar {bar} {op}：source={s} ≤ segment={FIRST_BSP_LADDER}（segment 非势源，pending_locate 检验1）"
    );
    let top = located[s].unwrap_or_else(|| {
        panic!("必然性违反@bar {bar} {op}：source={s} 无 located 条目（无定位链的操作=bug）")
    });
    assert_eq!(
        top.source_ladder, s,
        "必然性违反@bar {bar} {op}：located[{s}].source_ladder={} ≠ 操作 source {s}（链顶不一致）",
        top.source_ladder
    );
    assert_eq!(
        top.direction, dir,
        "必然性违反@bar {bar} {op}：located[{s}].direction 与操作方向不一致（链方向错配）"
    );
    assert!(
        top.arm_bar <= bar,
        "必然性违反@bar {bar} {op}：located[{s}].arm_bar={} > bar（未来武装，因果违反）",
        top.arm_bar
    );
    for k in FIRST_BSP_LADDER..=s {
        assert!(
            located[k].is_some(),
            "必然性违反@bar {bar} {op}：定位链 [{FIRST_BSP_LADDER}..={s}] 在层 {k} 断裂（不完整链的操作=bug）"
        );
    }
}

/// 主入口（`PolarityMode::PositioningChain` 经 `run_positional` 分派至此）。
///
/// `floor_ladder` = 结构 BSP 承载层下界（= FIRST_BSP_LADDER，递归基，所有标的同的
/// 结构常量，**非操作 floor**——操作层由 source 决定）。无 min_trade_ladder。
pub(crate) fn run_positioning_chain_fugue(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "positioning_chain_fugue 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})\
             （结构 BSP 承载层下界 = 递归基，非操作 floor）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("positioning_chain_fugue 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "positioning_chain_fugue 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理）；confirm 触发的递归基证据读 \
             flip_edge（a0 方向翻转沿），缺 dir_flips 行即判据残缺"
                .to_string(),
        );
    }

    let n = tape.bars.len();
    let mut res = PositionalResult::default();
    let mut chain: Vec<Voice> = Vec::with_capacity(MAX_LADDER);
    let mut free = INITIAL_CAPITAL;
    let mut n_base = 0.0f64;
    let mut book = CenterBook::new();
    let mut depth_ref = DepthRef::new(DEPTH_REF_WINDOW);

    // pending 窗口（双侧；高级别 candidate 武装、confirmed 清窗、破极值否定）。
    // **仅在 move(L1) 及以上（k ≥ PENDING_LO）维护**——segment 非势源。
    const PENDING_LO: usize = FIRST_BSP_LADDER + 1;
    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    // 双侧区间套定位链（pending confirm 兑现后的链；卖侧出场链 + 买侧入场链）。
    let mut located_sell: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];
    let mut located_buy: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    // a0（bi 层）方向翻转沿（rec_sub_evidence 的递归基证据词汇 = confirm）。

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

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
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── pending 窗口维护（双侧，仅 k ≥ PENDING_LO = move(L1)）：① 破极值
        //    否定（pending 失效）→ ② 高级别 candidate 武装 / confirmed 清窗 →
        //    ③ confirm 触发（rec_sub_evidence 递归到 a0，含 segment）──
        //
        // segment（k=FIRST_BSP_LADDER）**不维护 pending**——其 BSP/翻转仅作
        // rec_sub_evidence 的低级别 confirm 证据（高级别 pending 由它定位）。
        let mut confirm_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut confirm_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
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
                    let sellside = match e.class {
                        BspClass::Sell1 | BspClass::Sell3 => true,
                        BspClass::Buy1 | BspClass::Buy3 => false,
                        BspClass::Sell2 | BspClass::Buy2 => continue,
                    };
                    let win = if sellside { &mut nest_sell[k] } else { &mut nest_buy[k] };
                    if e.confirmed {
                        *win = None;
                    } else {
                        let ext = win.map_or(e.price, |w| {
                            if sellside {
                                w.extreme.max(e.price)
                            } else {
                                w.extreme.min(e.price)
                            }
                        });
                        *win = Some(Win { extreme: ext });
                        res.n_nest_arms_by_ladder[k] += 1;
                    }
                }
            }
            // 第14环：confirm 从 k−1 递归下探至 a0（含 segment + bi）。confirm
            // 层 < k−1 ⇒ 深定位。pending 持续 ⇒ confirm 任意 bar 兑现（先势后定位）。
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, &flip_edge) {
                    confirm_sell[k] = Some(w.extreme);
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, &flip_edge) {
                    confirm_buy[k] = Some(w.extreme);
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }

        // ── pending confirm 兑现 → 级联武装 located（仅 k ≥ PENDING_LO）：
        //    confirm@k 触发 ⇒ cascade [FIRST_BSP..=k]（统一源层极值 + source=k）。
        //    按 source 降序施加，保证最高 source 先占位（高 source 优先）。
        //    **segment 无 confirm_* 条目 ⇒ source 永不坍缩到 segment。**──
        for k in (PENDING_LO..MAX_LADDER).rev() {
            if let Some(ext) = confirm_sell[k] {
                cascade_arm(&mut located_sell, Side::Sell, k, ext, bar);
            }
            if let Some(ext) = confirm_buy[k] {
                cascade_arm(&mut located_buy, Side::Buy, k, ext, bar);
            }
        }
        // 破极值否定（027:25）：级联统一极值 ⇒ 整链同破（清空或不变，连续前缀
        // 不变量保持）。卖侧 c>extreme、买侧 c<extreme。含 located[segment]（被高
        // 级别定位覆盖的链底，随链同破）。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if located_sell[k].is_some_and(|e| c > e.extreme) {
                located_sell[k] = None;
            }
            if located_buy[k].is_some_and(|e| c < e.extreme) {
                located_buy[k] = None;
            }
        }

        // 双侧链顶 source（PCF 唯一层选择机制 = 最高有 located 的层；恒 ≥ PENDING_LO）。
        let sell_source = chain_source(&located_sell);
        let buy_source = chain_source(&located_buy);

        // 会计必然性（§8.2 存一次 / §8.4 bar 内连续核心）：同价 c 的一切操作
        // （A-F）是现金↔股数↔voice 的**价值中性**转换——操作前后 NAV(同 c) 必相等。
        let nav_pre_ops = nav(&chain, free, c);

        // ── A. 强平兜底（尾空头 1x 逐仓解析强平）──
        let mut acted = false;
        if let Some(tail) = chain.last().copied() {
            if tail.dir == Polarity::Short && tail.capital + tail.units * (tail.basis - c) <= 0.0 {
                pop_tail(
                    bar, 2.0 * tail.basis, c, "liq", false, &mut chain, &mut free, &mut n_base,
                    &mut res,
                );
                res.n_short_liquidations_by_ladder[tail.ladder] += 1;
                acted = true;
            }
        }

        // ── B. 否定扫描（根→尾第一个破 027:25 极值线 ⇒ 该层及以深解栈）──
        if !acted {
            let broke = chain.iter().position(|v| {
                v.negate_line.is_some_and(|line| match v.dir {
                    Polarity::Short => c > line,
                    Polarity::Long => c < line,
                })
            });
            if let Some(g) = broke {
                let lad = chain[g].ladder;
                unwind_to(g, bar, c, c, "negate", &mut chain, &mut free, &mut n_base, &mut res);
                res.n_nrf_negate_closes_by_ladder[lad] += 1;
                acted = true;
            }
        }

        // ── C. 根出场/翻转（第23环；出场对齐入场 source）：根长持 ∧ 卖链 source
        //    S ≥ root.ladder（势在入场级别或更高反转）∧ len==1 ⇒ 翻转 = 降成本
        //    m=N 特例（子空@root.ladder−1，携 located 极值否定线，027:25）。
        //    root.ladder == FIRST_BSP_LADDER（无更低子级别）⇒ 清仓到现金。
        //    len>1 ⇒ 不翻（子先经 D 独立回补——逐仓独立）。──
        if !acted && chain.first().is_some_and(|r| r.units > 0.0) {
            let root = chain[0];
            if let Some(s) = sell_source {
                if s >= root.ladder {
                    prove_chain(&located_sell, Side::Sell, s, bar, "C-flip/clear");
                    let flip_line = located_sell[s].map(|e| e.extreme);
                    let can_flip = chain.len() == 1
                        && root.ladder > FIRST_BSP_LADDER
                        && root.units > 0.0
                        && root.units.is_finite();
                    if can_flip {
                        // 翻转 = spawn(m=N) 反向子。时序：父释放现金 → 子使用。
                        let m = chain[0].units;
                        let sub = chain[0].ladder - 1;
                        let cash_released = m * c;
                        chain[0].units -= m; // 父 husk（保留 cost_pool/basis）
                        chain.push(Voice {
                            ladder: sub,
                            dir: Polarity::Short,
                            units: m,
                            basis: c,
                            cost_pool: cash_released,
                            capital: cash_released,
                            entry_bar: bar,
                            negate_line: flip_line,
                        });
                        res.n_nrf_root_flips_by_ladder[sub] += 1;
                        res.n_entries_by_ladder[sub] += 1;
                        located_sell = [None; MAX_LADDER];
                        acted = true;
                    } else if chain.len() == 1 && chain[0].ladder == FIRST_BSP_LADDER {
                        // 根已在链底 ⇒ 无更低子级别 ⇒ 清仓到现金。
                        unwind_to(0, bar, c, c, "sellpt", &mut chain, &mut free, &mut n_base, &mut res);
                        located_sell = [None; MAX_LADDER];
                        acted = true;
                    }
                    // else：len>1 ⇒ C no-op（子先经 D 回补，根稍后在 len==1 时翻）。
                }
            }
        }

        // ── D. 尾回补（第17环；子级别走势完美）：len>1，尾 voice 反向链 source
        //    S ≥ tail.ladder（势在尾级别或更高反转回来）⇒ 平子返父 + cost_pool
        //    传导 + earning 时机。──
        if !acted && chain.len() > 1 {
            let tail = *chain.last().expect("len>1");
            let (rev_source, rev_located, rev_dir) = match tail.dir {
                Polarity::Short => (buy_source, &located_buy, Side::Buy),
                Polarity::Long => (sell_source, &located_sell, Side::Sell),
            };
            if let Some(s) = rev_source {
                if s >= tail.ladder {
                    prove_chain(rev_located, rev_dir, s, bar, "D-recover");
                    pop_tail(bar, c, c, "recover", true, &mut chain, &mut free, &mut n_base, &mut res);
                    acted = true;
                }
            }
        }

        // ── E. 降成本 spawn（第15/16/17环）：尾 voice 反向链 source S < tail.ladder
        //    （次级别反转——区间套定位到更低层）⇒ 在 S 层开反向子，量 = θ_S 配额
        //    （高级别势大→大仓位；35课成本门拒小级别散单）。S ≥ PENDING_LO ⇒ 子
        //    最低 @move(L1)（segment 非势源 ⇒ 降成本止于 move(L1)）。──
        if !acted {
            if let Some(tail) = chain.last().copied() {
                let (rev_source, rev_located, rev_dir) = match tail.dir {
                    Polarity::Long => (sell_source, &located_sell, Side::Sell),
                    Polarity::Short => (buy_source, &located_buy, Side::Buy),
                };
                let rev_line = rev_source.and_then(|s| rev_located[s]).map(|e| e.extreme);
                if let Some(s) = rev_source {
                    if s < tail.ladder {
                        prove_chain(rev_located, rev_dir, s, bar, "E-spawn");
                        // 量 = tail.units × θ_S / θ_total（53课配额；高级别势大）。
                        match depth_ref.theta(s, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                            None => res.n_nrf_noref_rejects_by_ladder[s] += 1,
                            Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                                res.n_nrf_cost_rejects_by_ladder[s] += 1;
                            }
                            Some(_) => {
                                let (thetas, theta_total) =
                                    theta_weights(&depth_ref, FIRST_BSP_LADDER);
                                let w = thetas[s].map(|t| t / theta_total);
                                let m_quota = w.map_or(0.0, |w| tail.units * w);
                                let m = match tail.dir {
                                    Polarity::Long => m_quota,
                                    Polarity::Short => m_quota.min(tail.capital / c),
                                };
                                if m > 0.0 && m.is_finite() {
                                    let j = chain.len() - 1;
                                    let child_dir = match tail.dir {
                                        Polarity::Long => Polarity::Short,
                                        Polarity::Short => Polarity::Long,
                                    };
                                    let child = match child_dir {
                                        Polarity::Short => Voice {
                                            ladder: s,
                                            dir: Polarity::Short,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: m * c,
                                            entry_bar: bar,
                                            negate_line: rev_line,
                                        },
                                        Polarity::Long => Voice {
                                            ladder: s,
                                            dir: Polarity::Long,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: 0.0,
                                            entry_bar: bar,
                                            negate_line: rev_line,
                                        },
                                    };
                                    chain[j].units -= m;
                                    if child_dir == Polarity::Long {
                                        chain[j].capital -= m * c;
                                    }
                                    chain.push(child);
                                    res.n_nrf_spawns_by_ladder[s] += 1;
                                    res.n_entries_by_ladder[s] += 1;
                                    acted = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── F. 根入场(第23环)：链空 ∧ 完整买链 source S ⇒ 在 S 层满仓开多
        //    （26课恒仓——根=基，source S 决定出场绑定级别）。否定线 = 买链顶极值。──
        if !acted && chain.is_empty() {
            if let Some(s) = buy_source {
                let units = free / c;
                if units > 0.0 && units.is_finite() {
                    prove_chain(&located_buy, Side::Buy, s, bar, "F-entry");
                    let line = located_buy[s].map(|e| e.extreme);
                    chain.push(Voice {
                        ladder: s,
                        dir: Polarity::Long,
                        units,
                        basis: c,
                        cost_pool: free,
                        capital: 0.0,
                        entry_bar: bar,
                        negate_line: line,
                    });
                    n_base = units;
                    free = 0.0;
                    res.n_nrf_root_entries_by_ladder[s] += 1;
                    res.n_entries_by_ladder[s] += 1;
                    located_buy = [None; MAX_LADDER];
                }
            }
        }

        // ── 会计必然性 #4（bar 内价值中性）：操作后 NAV(同 c) == 操作前 ──
        let nav_post_ops = nav(&chain, free, c);
        assert!(
            (nav_pre_ops - nav_post_ops).abs() <= 1e-6 * nav_pre_ops.abs().max(1.0),
            "会计必然性违反@bar {bar}：操作非价值中性 NAV {nav_pre_ops}→{nav_post_ops}\
             （同价 c 操作必须现金↔股数等价，§8.2 存一次 / §8.4 bar 内连续）"
        );

        // ── 全局不变量（§8.1）：Σ 链上在手单位 = N_base。守恒 violation = panic
        //    （任务裁决：会计 bug 必须立即 abort，不静默吞错）。──
        let sum_units: f64 = chain.iter().map(|v| v.units).sum();
        assert!(
            (sum_units - n_base).abs() <= 1e-6 * n_base.max(1.0),
            "守恒律 §8.1 违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}"
        );

        // 观测：链深度 + 物理暴露 + 各层视图持有 bar 计数。
        res.nrf_depth_bars[chain.len().min(MAX_LADDER - 1)] += 1;
        let long_units: f64 = chain.iter().filter(|v| v.dir == Polarity::Long).map(|v| v.units).sum();
        let short_units: f64 =
            chain.iter().filter(|v| v.dir == Polarity::Short).map(|v| v.units).sum();
        if long_units > 0.0 {
            res.nrf_phys_long_bars += 1;
        }
        if short_units > 0.0 {
            res.nrf_phys_short_bars += 1;
        }
        for v in &chain {
            res.held_bars_by_ladder[v.ladder] += 1;
            if v.dir == Polarity::Short {
                res.short_held_bars_by_ladder[v.ladder] += 1;
            }
        }

        if bar % EQUITY_SAMPLE_BARS == 0 || i + 1 == n {
            res.equity.push((bar, nav(&chain, free, c)));
        }
    }

    // eod：全链解栈。
    if !chain.is_empty() {
        let c_last = tape.bars.last().map_or(f64::NAN, |b| b.close);
        let last_bar = (n as i64) - 1;
        unwind_to(0, last_bar, c_last, c_last, "eod", &mut chain, &mut free, &mut n_base, &mut res);
    }
    res.final_nav = free;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::positional::{run_positional, PolarityMode};
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::LadderMask;

    const PCF: PolarityMode = PolarityMode::PositioningChain;

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() { (Some(50.0), Some(60.0)) } else { (None, None) };
        BspEvent { class, seg_idx: 0, confirmed, cs, zd, zg, price }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows =
            b.bsp_events.get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
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
        run_positional(&t, 2, PCF).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("pcf"), Some(PCF));
        // 缺背驰磁带 ⇒ Err。
        let t = SignalTape {
            bars: vec![with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, PCF).unwrap_err().contains("背驰磁带"));
        // 缺 dir_flips ⇒ Err。
        let t2 = SignalTape {
            bars: vec![with_empty_div(with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None)))],
            dir_flips: None,
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, PCF).unwrap_err().contains("dir_flips"));
    }

    #[test]
    fn cascade_arm_fills_chain_downward() {
        // pending confirm 级联：confirm@4 ⇒ 武装 [2,3,4] 全层（含 located[segment]
        // 被高级别覆盖），统一极值 110/source=4/dir=Sell。
        let mut loc: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];
        cascade_arm(&mut loc, Side::Sell, 4, 110.0, 7);
        for k in [2usize, 3, 4] {
            let e = loc[k].expect("级联武装 [2..4] 全层");
            assert_eq!(e.source_ladder, 4, "全层统一 source=链顶 4（含 segment 被覆盖）");
            assert_eq!(e.extreme, 110.0, "全层统一极值=源层 027:25 否定线");
            assert_eq!(e.direction, Side::Sell);
            assert_eq!(e.arm_bar, 7);
        }
        assert!(loc[5].is_none(), "源层之上不武装");
        // 高 source 优先：低级别 confirm@3（ext 90）不降级既有 source=4。
        cascade_arm(&mut loc, Side::Sell, 3, 90.0, 8);
        assert_eq!(loc[2].unwrap().source_ladder, 4, "低 source 不降级（高 source 主导）");
        assert_eq!(loc[2].unwrap().extreme, 110.0, "极值仍为源层 4");
        // 更高 source@6（ext 200）⇒ 覆盖全部 [2..6] 为统一 6/200（含原 [2,3,4]）。
        cascade_arm(&mut loc, Side::Sell, 6, 200.0, 9);
        for k in 2usize..=6 {
            let e = loc[k].expect("更高级联覆盖 [2..6]");
            assert_eq!(e.source_ladder, 6, "更高 source 统一覆盖");
            assert_eq!(e.extreme, 200.0);
        }
    }

    #[test]
    fn chain_source_finds_highest_located() {
        // chain_source = 最高有 located 的层（级联保证连续前缀，source ≥ 3）。
        let mut loc: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];
        assert_eq!(chain_source(&loc), None, "空 ⇒ 无链");
        cascade_arm(&mut loc, Side::Sell, 4, 110.0, 0);
        assert_eq!(chain_source(&loc), Some(4), "级联 [2..4] ⇒ 链顶=4");
        // 关键区分：单独高级别 pending（无需中间层独立 candidate）即给出高 source。
        let mut loc2: [Option<PendingLocate>; MAX_LADDER] = [None; MAX_LADDER];
        cascade_arm(&mut loc2, Side::Sell, 6, 150.0, 0);
        assert_eq!(chain_source(&loc2), Some(6), "高级别单独 pending 级联 ⇒ source=6 不坍缩");
    }

    /// pending_locate 本质检验：**只**武装高级别 pending@4（无 2/3 独立 candidate），
    /// bi 向下翻（segment confirm）⇒ confirm@4 ⇒ 级联 [2,3,4] ⇒ source=4 ≥
    /// root.ladder ⇒ 翻转。segment 单独反转不会武装 located（不入势源）。
    #[test]
    fn high_pending_with_segment_confirm_cascades_to_flip() {
        let (mut bars, mut flips) = full_bull_entry(); // 根入场@source=4
        // 仅武装 nest_sell@4（高级别单独 pending，无 2/3）。
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(bar(104.0)); // bi 翻 Down（segment/a0 confirm）⇒ confirm@4 ⇒ 级联 source=4
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(sellanypt(bar(102.0), 4));
        bars.push(bar(102.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        let r = run(bars, flips);
        assert_eq!(
            r.n_nrf_root_flips_by_ladder[3], 1,
            "高级别单独 pending + 低级别 confirm ⇒ source=4 ≥ root.ladder=4 ⇒ 翻转"
        );
    }

    /// 必然性检验3：**segment 单独 candidate（无高级别 pending）⇒ 不武装 located
    /// ⇒ 不建仓**（source > segment 检验1 的对偶）。
    #[test]
    fn segment_candidate_alone_no_entry() {
        let mut bars = warmup234();
        // 仅 segment（k=2）买 candidate + bi 向上翻（segment 自身的 confirm）。
        bars.push(with_ev(bar(95.0), 2, ev_full(BspClass::Buy1, false, 90.0, None)));
        bars.push(bar(96.0));
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(bar(97.0));
        // bi 翻 Up：segment candidate 不维护 pending ⇒ 无 confirm_buy ⇒ 无 located。
        let r = run(bars, vec![(evidence_bar, 1, Direction::Up)]);
        assert_eq!(
            r.n_nrf_root_entries_by_ladder.iter().sum::<u64>(),
            0,
            "segment 单独 candidate ⇒ 零入场（source 必 > segment）"
        );
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9, "全程现金");
    }

    /// 构造高级别买 pending [3,4] + segment/a0 confirm ⇒ 根在 source=4 满仓入场。
    /// 武装 nest_buy@3/4（高级别 pending）→ bi 向上翻（confirm）⇒ located_buy[2,3,4]。
    fn full_bull_entry() -> (Vec<BarSig>, Vec<(i64, u8, Direction)>) {
        let mut bars = warmup234();
        // 武装 nest_buy@3/4（买侧 pending，极值 88/86 = 低点）。segment@2 的 Buy1
        // 不武装 pending，但作 rec_sub_evidence 的 confirm 证据。
        let mut b = with_ev(bar(95.0), 2, ev_full(BspClass::Buy1, false, 90.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Buy1, false, 88.0, None));
        b = with_ev(b, 4, ev_full(BspClass::Buy1, false, 86.0, None));
        bars.push(b);
        // confirm bar：bi 向上翻 ⇒ rec_sub_evidence(Buy) ⇒ confirm_buy[3,4] ⇒ located。
        bars.push(bar(96.0));
        let evidence_bar = bars.len() as i64 - 1;
        bars.push(bar(97.0)); // 入场后持有
        (bars, vec![(evidence_bar, 1, Direction::Up)])
    }

    #[test]
    fn root_enters_at_buy_chain_source() {
        let (bars, flips) = full_bull_entry();
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "买 pending 级联到 source=4 ⇒ 根入场@source=4");
        // 满仓恒仓（价格无关不变量）：shares × entry_price == 全部初始资金。
        let entry = r.trades.iter().find(|t| t.polarity == Polarity::Long).unwrap();
        assert!(
            (entry.shares * entry.entry_price - INITIAL_CAPITAL).abs() < 1e-6,
            "满仓恒仓（全部资金入场）shares={} px={}",
            entry.shares,
            entry.entry_price
        );
    }

    #[test]
    fn no_buy_chain_no_entry() {
        // 仅孤立高级别 candidate@4（无 bi 翻 confirm）⇒ pending 未兑现 ⇒ 不入场。
        let mut bars = warmup234();
        bars.push(with_ev(bar(95.0), 4, ev_full(BspClass::Buy1, false, 90.0, None)));
        bars.push(bar(96.0));
        let r = run(bars, vec![]); // 无 bi 翻 ⇒ confirm 不成立
        assert_eq!(r.n_nrf_root_entries_by_ladder.iter().sum::<u64>(), 0, "无 confirm ⇒ 零入场");
        assert!((r.final_nav - INITIAL_CAPITAL).abs() < 1e-9, "全程现金");
    }

    #[test]
    fn sub_level_sell_chain_spawns_cost_reduction() {
        // 根@4 入场后，次级别卖 pending source=3 (<4) ⇒ E 降成本 spawn 子空@3。
        let (mut bars, mut flips) = full_bull_entry();
        // 武装 nest_sell@3（高级别卖 pending，极值高点），bi 翻 Down ⇒ confirm@3。
        // segment@2 的 Sell1 作 confirm 证据，不武装 pending。
        let mut b = with_ev(bar(105.0), 2, ev_full(BspClass::Sell1, false, 106.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 108.0, None));
        bars.push(b);
        bars.push(bar(104.0));
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(bar(104.0));
        flips.push((sell_ev_bar, 1, Direction::Down)); // bi 翻 Down ⇒ confirm_sell[3]
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "卖 pending source=3 < root.ladder=4 ⇒ 降成本 spawn@3");
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "次级别反转非清仓");
    }

    /// 完整卖 pending 到 source ≥ root.ladder ⇒ 根翻转。
    fn full_bear_chain_at_root() -> (Vec<BarSig>, Vec<(i64, u8, Direction)>) {
        let (mut bars, mut flips) = full_bull_entry();
        // 武装 nest_sell@3/4（高级别卖 pending）。segment@2 作 confirm 证据。
        let mut b = with_ev(bar(105.0), 2, ev_full(BspClass::Sell1, false, 106.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 108.0, None));
        b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 110.0, None));
        bars.push(b);
        bars.push(bar(104.0)); // bi 翻 Down ⇒ confirm_sell[3,4] ⇒ source=4
        let sell_ev_bar = bars.len() as i64 - 1;
        bars.push(sellanypt(bar(102.0), 4));
        bars.push(bar(102.0));
        flips.push((sell_ev_bar, 1, Direction::Down));
        (bars, flips)
    }

    #[test]
    fn full_sell_chain_at_root_flips() {
        let (bars, flips) = full_bear_chain_at_root();
        let r = run(bars, flips);
        assert_eq!(
            r.n_nrf_root_flips_by_ladder[3], 1,
            "卖 pending source=4 ≥ root.ladder=4 ∧ len==1 ⇒ 翻转为子空@3"
        );
        assert!(r.trades.iter().all(|t| t.exit_reason != "sellpt"), "翻转非清仓");
    }

    #[test]
    fn flip_child_carries_negate_line() {
        // 翻转子空携 027:25 否定线 = located_sell[source=4] 极值 110；升破 ⇒ B 否定平仓。
        let (mut bars, flips) = full_bear_chain_at_root();
        bars.push(bar(115.0)); // 升破 110 ⇒ 否定
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_flips_by_ladder[3], 1);
        assert_eq!(
            r.n_nrf_negate_closes_by_ladder[3], 1,
            "子空 negate=located 极值 110，升破 ⇒ B 否定平仓"
        );
    }

    #[test]
    fn conservation_holds_through_recursion() {
        // §8.1 守恒：递归流转每 bar Σunits=N_base（引擎内 assert 守卫；未 panic 即通过）。
        let (bars, flips) = full_bear_chain_at_root();
        let r = run(bars, flips);
        // 翻转后子空持有；eod 全链解栈，final_nav 有限。
        assert!(r.final_nav.is_finite() && r.final_nav > 0.0, "final_nav={}", r.final_nav);
    }
}
