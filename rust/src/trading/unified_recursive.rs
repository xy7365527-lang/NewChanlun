//! unified_recursive — 统一递归系统（从概念运动链 23 环直接翻译；mode = "urs"）。
//!
//! 设计源头：`docs/concept_movement_chain.md`（23 环）+ `docs/nested_fugue_
//! accounting.md`（§1-§8 严格会计）。编排者 2026-06-13：**不再 patch——从
//! 构成性必然一次性实装。**
//!
//! ## 与 v4（nested_fugue.rs）的唯一构成性差异：C 清仓层定义
//!
//! v2-v5/fusion_t/btra 的清仓判据都是经验性叠加（棘轮 / 事件选层 / regime
//! 门 / 白名单）。本系统从概念链第 13 环（级别=递归层次）+ 会计 §1（"根
//! voice = 最高涌现级别的 voice，只有根 voice 走势完美时才清仓"）重新推导：
//!
//! - **v4**：清仓层 = `depth_ref` 振幅棘轮 top（振幅驱动、单调攀高）。
//! - **CT（539）**：清仓层 = A′ 事件选层 max{k: sell1[k]∧located[k]}（事件
//!   驱动、任意层 type1 都可触发选层 ⇒ 强牛子趋势顶都清 = 踏空，已 L3 否证）。
//! - **本系统（URS）**：清仓层 = 根 voice 的**涌现归属级别 E\***（走势结构
//!   驱动）。E\* 从根入场级别向上爬，父级别须同时满足 ① `dir==Up`（方向
//!   向上）② `anchor ≥ root.entry_bar`（该段在持仓期内由走势自身生长出来）。
//!
//! E\* 的构成性必然（runner.rs `ExitMode::Emergent` 已 L3 验证 = emht 五标的
//! 全正，BTC+170.8pp）：
//!   · 仅 ①（牛市全塔 Up）退化为全局塔高 = Climb 设计缺陷（max_ladder 是
//!     外部参数非走势自身涌现）。
//!   · ② anchor≥entry = 该 Up 段是持仓走势**自身**生长出来的（中枢扩展 /
//!     新中枢形成 → 走势级别上升的方向行投影）。两者合取缺一不可。
//!   · 无状态每 bar 重读，高层归属翻转即时回落——清仓频率**由走势结构自然
//!     决定**：强趋势持仓期不断长出高级别 Up 段 ⇒ E\* 持续高 ⇒ 高层卖点
//!     罕见 ⇒ 清仓少（趋势保护，但非棘轮）；波动市高层段翻转/锚回落 ⇒ E\*
//!     回根层 ⇒ 根层卖点触发清仓 ⇒ 清仓多。这正是任务预测的 regime 分化，
//!     **不是 regime 门**——是同一构成性判据在不同走势结构上的自然涌现。
//!
//! ## Phase 1（2026-06-13）：清仓门 sell1 → sell_any
//!
//! C 清仓层原只消费 type1 背驰（sig.sell1[E\*]）。诊断 type1_coverage_diagnosis.md：
//! OKLO 66% 的走势反转是 type3 而非 type1 ⇒ 只消费 sell1 漏掉 2/3 操作机会。改为
//! sig.sell_any[E\*]（type1∨type2∨type3）∧ located[E\*]。located 仍由 Sell1∨Sell3 武装
//! ⇒ type3 反转经其 located 窗口即触发清仓。sell_any ⊇ sell1 ⇒ 严格放宽（旧 type1
//! 触发全保留）。降成本（E）/ 回补（D）/ 入场（F）原已消费 _any，本 Phase 不动。
//!
//! ## 概念链 → 代码映射（23 环全覆盖）
//!
//! | 环 | 概念 | 代码 |
//! |----|------|------|
//! | 4 | 离散化（唯一经验参数 a0） | K线周期（信号层输入） |
//! | 5-8 | 分型→笔→线段→中枢 | 信号层 ladder0-2（引擎产出） |
//! | 9 | 走势类型=中枢数量 | type1 背驰（sell1/buy1） |
//! | 10-12 | 背驰→三类买卖点 | BspClass type1/2/3 + DivEvent |
//! | 13 | 级别=递归层次 | `chain` 每层一个 Voice |
//! | 14 | 区间套=层间桥梁 | `rec_sub_evidence`（定位型：candidate 武装 → 次级别证据精化时点） |
//! | 15 | 级别=操作量 | m = θ 配额（53课留白） |
//! | 16 | 成本门=递归终止 | 35课成本门 + floor_ladder |
//! | 17 | 降成本 | E：spawn 子 voice |
//! | 18-20 | 并发/多重赋格/嵌套递归 | Voice 链（任意深度） |
//! | 21-22 | 多空对称/方向交替 | 子 voice 方向跟随子级别（Polarity 镜像会计） |
//! | 23 | 操盘三阶段 | F 入场 → E 降成本/D 回补 → C 清仓 → 翻转（循环） |
//!
//! ## 每 bar 优先序（同 bar 单事件；§1-§8 会计 = v4 复用，bit-exact）
//!
//! A. 强平兜底 → B. 否定扫描 → **C. 清仓（E\* 涌现层 type1∧located）** →
//! D. 尾回补（走势完美） → E. spawn 降成本 → F. 根入场（链空）。
//! A/B/D/E/F + earning + 守恒 = `nested_fugue` 会计原语逐字复用（同一份会计
//! 文档的实装）；唯一新逻辑 = C 段 E\* 涌现层 + anchor_state 维护。
//!
//! ## 零 flag 声明
//!
//! 唯一参数 = a0 粒度（K线周期，数据决定）+ floor_ladder（结构常量 =
//! BSP 承载层下界，所有标的同）。无 ClearanceMode 选项、无 regime 门、无
//! 标的白名单——任何标的用同一系统。

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

/// 根 voice 的涌现归属级别 E\*（会计 §1"根 voice = 最高涌现级别"的精确
/// 形式；runner.rs `ExitMode::Emergent` 已 L3 验证的读法）。
///
/// 从根入场级别 `root_ladder` 向上爬，父级别 lad+1 需同时满足：
///   ① `dir_state[lad+1] == Up`：该父级别当前方向向上。
///   ② `anchor_state[lad+1] >= root_entry_bar`：该父级别当前段在持仓期内
///      由走势自身生长出来（中枢扩展/新中枢形成的方向行投影）。
/// 两者合取缺一不可（仅①退化为全局塔高 = Climb 缺陷）。`max_l` 上界 =
/// 当前涌现塔高 +1（爬升被 ①② 限制，不会虚爬到塔顶除非走势真涌现到那）。
fn root_emergent_ladder(
    root_ladder: usize,
    root_entry_bar: i64,
    dir_state: &[Option<Direction>; MAX_LADDER],
    anchor_state: &[i64; MAX_LADDER],
    max_l: usize,
) -> usize {
    let mut lad = root_ladder;
    while lad + 1 < max_l
        && dir_state[lad + 1] == Some(Direction::Up)
        && anchor_state[lad + 1] >= root_entry_bar
    {
        lad += 1;
    }
    lad
}

/// Phase 0（展开侧递归确认，编排者裁决 2026-06-13）：BSP confirmed = 递归链。
///
/// `confirmed(k) = settled(k) ∧ ∃sell_any/buy_any(k-1).confirmed(k-1)`，递归基
/// `confirmed(a0) = segment.settled`（特征序列确认）。展开为：从 estar 到 a0 每层
/// 区间套定位（located）必须完整 ∧ a0（bi 层 = FIRST_BSP_LADDER-1）方向已翻向目标侧。
///
/// located[k] 是 sticky 的区间套确认（nf 触发置、破极值清，sub-evidence 已下探到 a0），
/// 是 confirmed(k) 的强形式（含次级别证据）。链完整 = 弱确认被过滤 ⇒ 消除误翻转 churn
/// （urs_phase2_flip_results.md §4：单层 located 弱确认放行误翻转，CL 505 negates）。
///
/// `want` = 目标方向（卖侧 = Down：a0 须向下翻；买侧 = Up）。`located` = 该侧定位数组。
fn recursive_confirmed(
    estar: usize,
    want: Direction,
    located: &[Option<f64>; MAX_LADDER],
    dir_state: &[Option<Direction>; MAX_LADDER],
) -> bool {
    // 递归基 a0 = bi 层（FIRST_BSP_LADDER-1）：confirmed(a0) = 该层方向翻向目标侧
    // （segment.settled 的方向投影——笔级特征序列确认的方向）。
    if dir_state[FIRST_BSP_LADDER - 1] != Some(want) {
        return false;
    }
    // 链：FIRST_BSP_LADDER 到 estar 每层 located 必须 set（rconf[k]=located[k]∧rconf[k-1]
    // 的展开 = ∀j∈[FIRST_BSP_LADDER,estar] located[j]）。
    (FIRST_BSP_LADDER..=estar).all(|k| located[k].is_some())
}

/// 【EXPERIMENTAL，编排者 2026-06-13 隔离实验，非生产 flag——产出后移除】
/// C 块动作开关：true = 清仓到现金（Phase 1+0：sell_any + 递归确认 + 清仓，隔离
/// 验证递归确认是否消除 Phase 1 的 CL 过度清仓）；false = 翻转 spawn(m=N)（Phase 2+0）。
/// 二者共用 sell_any（Phase 1）+ recursive_confirmed（Phase 0）门，仅 C 块动作不同。
const C_CLEAR_NOT_FLIP: bool = false;

/// 主入口（`PolarityMode::UnifiedRecursive` 经 `run_positional` 分派至此）。零 flag。
pub(crate) fn run_unified_recursive(
    tape: &SignalTape,
    floor_ladder: usize,
) -> Result<PositionalResult, String> {
    if !(FIRST_BSP_LADDER..MAX_LADDER).contains(&floor_ladder) {
        return Err(format!(
            "unified_recursive 要求 floor_ladder ∈ [{FIRST_BSP_LADDER}, {MAX_LADDER})（BSP \
             承载层）；floor_ladder={floor_ladder}"
        ));
    }
    if !tape.has_bsp_events() {
        return Err("unified_recursive 要求事件磁带（bsp_events 全空）".to_string());
    }
    if !(tape.has_div_events() && tape.has_dir_rows()) {
        return Err(
            "unified_recursive 要求背驰磁带 + dir_flips 行——区间套次级别证据词汇 = \
             BSP ∨ 背驰事件 ∨ bi 层方向翻转沿（027课程序定理）；E* 涌现层读 \
             dir_state（方向）+ anchor_state（段锚），缺 dir_flips 行即判据残缺"
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

    let mut nest_sell: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut nest_buy: [Option<Win>; MAX_LADDER] = [None; MAX_LADDER];
    let mut located_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];

    let flips: &[(i64, u8, Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;

    // 方向滚动状态（E* 涌现层 ①）：dir_flips → 持久逐 bar 视图。
    let mut dir_state: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    // 段锚滚动状态（E* 涌现层 ②）：每层最近一次方向翻转 bar = 方向 run 的
    // 信号观测起点（与全系统确认滞后时间口径一致；runner.rs anchor_state
    // 同构）。初值 -1（无翻转 ⇒ anchor < 任何 entry_bar ⇒ 不爬升，保守
    // 退化为根层清仓判据）。
    let mut anchor_state: [i64; MAX_LADDER] = [-1; MAX_LADDER];

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
            dir_state[lad as usize] = Some(dir); // 持久（E* ①）
            anchor_state[lad as usize] = bar; // 段锚（E* ②）
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

        // ── 区间套窗口维护（第14环）：① 打破否定 → ② candidate 武装 /
        //    confirmed 清窗 → ③ 递归证据触发（nf_* 携带触发时极值）──
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        for k in FIRST_BSP_LADDER..MAX_LADDER {
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
            // 第14环：递归下探至 a0。证据层 < k−1 ⇒ 深触发。
            let sub = k - 1;
            if let Some(w) = nest_sell[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Sell, evrows, devrows, &flip_edge) {
                    nf_sell[k] = Some(w.extreme);
                    nest_sell[k] = None;
                    res.n_nest_fire_sell_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
            if let Some(w) = nest_buy[k] {
                if let Some(j) = rec_sub_evidence(sub, Side::Buy, evrows, devrows, &flip_edge) {
                    nf_buy[k] = Some(w.extreme);
                    nest_buy[k] = None;
                    res.n_nest_fire_buy_by_ladder[k] += 1;
                    if j < sub {
                        res.n_nrf_deep_fires_by_ladder[k] += 1;
                    }
                }
            }
        }

        // 区间套定位记忆（§6.2"背驰已被区间套递归确认"）：nf 触发记录极值，
        // 价格破极值则定位失效；C 消费后清空。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if let Some(ext) = nf_sell[k] {
                located_sell[k] = Some(ext);
            }
            if located_sell[k].is_some_and(|ext| c > ext) {
                located_sell[k] = None;
            }
        }

        // 当前最高 θ 涌现层（F 入场层；振幅涌现参照——入场时无持仓锚点，
        // E* 不可算，入场看 buy 证据的最高振幅涌现层，runner.rs"入场看
        // buy1 涌现级别"对称）。
        let top = (floor_ladder..MAX_LADDER)
            .rev()
            .find(|&k| depth_ref.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS).is_some());
        // E* 涌现层爬升上界（当前塔高 +1）。
        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        // ── A. 强平兜底（尾空头 1x 逐仓解析强平）──
        let mut acted = false;
        if let Some(tail) = chain.last().copied() {
            if tail.dir == Polarity::Short
                && tail.capital + tail.units * (tail.basis - c) <= 0.0
            {
                pop_tail(
                    bar, 2.0 * tail.basis, c, "liq", false, &mut chain, &mut free,
                    &mut n_base, &mut res,
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

        // ── C. 翻转（§6 + 环21/23 + 编排者裁决 2026-06-13）：根 voice 涌现归属层
        //    E* 出现卖点（sell_any = type1∨type2∨type3）∧ located[E*] ⇒
        //    **翻转 = 降成本的 m=N 特例**（非清仓到现金）。父 voice 卖出全部 N 股
        //    （N→0，husk 保留 cost_basis/pool）→ 子 voice 用释放的全部资金（N×c）
        //    反向开空 N。和 E 降成本完全同构，只 m=N。
        //    · 否定线 = located 极值（027:25）在子 voice 生命周期保护（A 强平/B 否定）
        //      ⇒ 化解 v3 死因②（翻转无否定线 → 强平归零，project_nrf_v3_root_flip_falsified）。
        //    · 条件化 = E* 涌现层判据（非每 bar 无条件）⇒ 化解 v3 死因①（盘整税）。
        //    · 会计逻辑时序（编排者；物理一笔，会计先后）：先父释放现金（units N→0）
        //      → 再子用现金开仓（capital = 释放现金）。不可反序——子资金来自父释放。
        //    · 多 voice 链：先收敛降成本子回根（unwind_to(1) 使 root 回满）→ 再根翻转。
        //    · floor 兜底：root 已在 floor ⇒ 次级别非 BSP 承载层 ⇒ 退化清仓到现金。
        //    · 根恒 long husk：框架短头会计要求有父（pop_tail 短支 u_back 回流父 units），
        //      根级短头不可表示 ⇒ pop_tail 根分支恒 long，debug_assert(Long) 仍成立。
        //      环21"永远有方向"+环23"出场=翻转=新建仓"满足：翻转后仓位=子空 N（非
        //      现金 flat），子回补时父 husk 回满（短→多），仓位连续不中断。──
        // 守卫 root.units > 0：翻转后父 husk（units=0）已是空头状态（子空持仓），
        // 无多头可翻 ⇒ 不重复触发翻转（防 collapse+reflip churn）。husk 经 D 回补
        // 回满（units>0）后方可再翻。
        if !acted && chain.first().is_some_and(|r| r.units > 0.0) {
            let root = chain[0];
            let estar =
                root_emergent_ladder(root.ladder, root.entry_bar, &dir_state, &anchor_state, max_l);
            // Phase 0 递归确认：sell_any[estar] ∧ 区间套链完整到 a0（非单层 located）。
            // 弱确认（仅 located[estar]）被过滤 ⇒ 消除 Phase 2 误翻转 churn。
            if sig.sell_any.get(estar)
                && recursive_confirmed(estar, Direction::Down, &located_sell, &dir_state)
            {
                let flip_line = located_sell[estar]; // 027:25 否定线 = located 极值
                // Phase 3（逐仓独立，编排者裁决"子voice平仓不影响父voice"）：翻转仅在
                // chain.len()==1（根独存）时发生——根降成本 m=N → 子独立逐仓开空，父 husk
                // 不退出。链有降成本子（len>1）时**不翻转**（不 collapse）：子先经 D 独立回补
                // 各自结算，根稍后在 len==1 时翻。彻底消除"全仓翻空"的 collapse+reflip churn。
                let can_flip = !C_CLEAR_NOT_FLIP
                    && chain.len() == 1
                    && chain[0].ladder != floor_ladder
                    && chain[0].units > 0.0
                    && chain[0].units.is_finite();
                if can_flip {
                    // 翻转 = spawn(m=N) 反向子。时序：父释放现金 → 子使用（会计逻辑序）。
                    let m = chain[0].units; // 全部 N 股（根独存 ⇒ = N）
                    let sub = chain[0].ladder - 1;
                    let cash_released = m * c; // step1：父卖出全部 N 股，释放现金
                    chain[0].units -= m; // 父账本 N→0（husk，保留 cost_pool/basis）
                    chain.push(Voice {
                        ladder: sub,
                        dir: Polarity::Short,
                        units: m,
                        basis: c,
                        cost_pool: cash_released,
                        capital: cash_released, // step2：子用释放现金反向开空 N
                        entry_bar: bar,
                        negate_line: flip_line,
                    });
                    res.n_nrf_root_flips_by_ladder[sub] += 1; // 翻转（与降成本 spawn 区分）
                    res.n_entries_by_ladder[sub] += 1;
                    located_sell = [None; MAX_LADDER];
                    acted = true;
                } else if C_CLEAR_NOT_FLIP || chain[0].ladder == floor_ladder {
                    // Phase 1+0 隔离（C_CLEAR_NOT_FLIP）∨ floor 兜底 ⇒ 清仓到现金。
                    let _ = flip_line;
                    unwind_to(0, bar, c, c, "sellpt", &mut chain, &mut free, &mut n_base, &mut res);
                    located_sell = [None; MAX_LADDER];
                    acted = true;
                }
                // else：len>1（有降成本子）⇒ C 是 no-op（不翻转/不清仓/不清 located），
                // 根层卖点落 E 降成本，子先独立回补，根稍后在 len==1 时翻（逐仓独立）。
            }
        }

        // ── D. 尾回补（非根尾的 confirmed 反向点@own-level = 子级别走势
        //    完美 ⇒ 平子 + 父回满 + cost_pool 传导 + earning 时机）──
        if !acted && chain.len() > 1 {
            let tail = *chain.last().expect("len>1");
            let perfected = match tail.dir {
                Polarity::Short => sig.buy_any.get(tail.ladder),
                Polarity::Long => sig.sell_any.get(tail.ladder),
            };
            if perfected {
                pop_tail(bar, c, c, "recover", true, &mut chain, &mut free, &mut n_base, &mut res);
                acted = true;
            }
        }

        // ── E. spawn（降成本释放）：尾的 nest 定位反向点@own-level ∨ 根尾
        //    的 confirmed 卖（C 未消费的一切卖点，§9"其他卖点全部走E"）⇒
        //    释放 θ 配额 m 给子 voice。终止 = floor ∨ 35课成本门 ──
        if !acted {
            if let Some(tail) = chain.last().copied() {
                let (nest_fired, confirmed_sell_root) = match tail.dir {
                    Polarity::Long => (
                        nf_sell[tail.ladder],
                        chain.len() == 1 && sig.sell_any.get(tail.ladder),
                    ),
                    Polarity::Short => (nf_buy[tail.ladder], false),
                };
                if nest_fired.is_some() || confirmed_sell_root {
                    if tail.ladder == floor_ladder {
                        res.n_nrf_floor_stops_by_ladder[tail.ladder] += 1;
                    } else {
                        let sub = tail.ladder - 1;
                        match depth_ref.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                            None => res.n_nrf_noref_rejects_by_ladder[sub] += 1,
                            Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
                                res.n_nrf_cost_rejects_by_ladder[sub] += 1;
                            }
                            Some(_) => {
                                let (thetas, theta_total) = theta_weights(&depth_ref, floor_ladder);
                                let w = thetas[sub].map(|t| t / theta_total);
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
                                            ladder: sub,
                                            dir: Polarity::Short,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: m * c,
                                            entry_bar: bar,
                                            negate_line: nest_fired,
                                        },
                                        Polarity::Long => Voice {
                                            ladder: sub,
                                            dir: Polarity::Long,
                                            units: m,
                                            basis: c,
                                            cost_pool: m * c,
                                            capital: 0.0,
                                            entry_bar: bar,
                                            negate_line: nest_fired,
                                        },
                                    };
                                    chain[j].units -= m;
                                    if child_dir == Polarity::Long {
                                        chain[j].capital -= m * c;
                                    }
                                    chain.push(child);
                                    res.n_nrf_spawns_by_ladder[sub] += 1;
                                    res.n_entries_by_ladder[sub] += 1;
                                    acted = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── F. 根入场（链空）：最高 θ 涌现层买证据 ⇒ 满仓开多 ──
        if !acted && chain.is_empty() {
            if let Some(top) = top {
                let nf = nf_buy[top];
                if sig.buy_any.get(top) || nf.is_some() {
                    let units = free / c;
                    if units > 0.0 && units.is_finite() {
                        let line = if sig.buy_any.get(top) { None } else { nf };
                        chain.push(Voice {
                            ladder: top,
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
                        res.n_nrf_root_entries_by_ladder[top] += 1;
                        res.n_entries_by_ladder[top] += 1;
                    }
                }
            }
        }

        // ── 全局不变量（§8.1）：Σ 链上在手单位 = N_base ──
        let sum_units: f64 = chain.iter().map(|v| v.units).sum();
        if (sum_units - n_base).abs() > 1e-6 * n_base.max(1.0) {
            return Err(format!(
                "不变量违反@bar {bar}：Σunits={sum_units} ≠ N_base={n_base}（守恒律 §8.1）"
            ));
        }

        // 观测：链深度直方图 + 物理暴露 + 各层视图持有 bar 计数。
        res.nrf_depth_bars[chain.len().min(MAX_LADDER - 1)] += 1;
        let long_units: f64 = chain
            .iter()
            .filter(|v| v.dir == Polarity::Long)
            .map(|v| v.units)
            .sum();
        let short_units: f64 = chain
            .iter()
            .filter(|v| v.dir == Polarity::Short)
            .map(|v| v.units)
            .sum();
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

    const URS: PolarityMode = PolarityMode::UnifiedRecursive;

    fn bar(close: f64) -> BarSig {
        BarSig { close, max_ladder: 5, ..Default::default() }
    }

    fn ev_full(class: BspClass, confirmed: bool, price: f64, cs: Option<i64>) -> BspEvent {
        let (zd, zg) = if cs.is_some() { (Some(50.0), Some(60.0)) } else { (None, None) };
        BspEvent { class, seg_idx: 0, confirmed, cs, zd, zg, price }
    }

    fn with_ev(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        let rows = b
            .bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()));
        rows[lad].push(e);
        b
    }

    fn with_empty_div(mut b: BarSig) -> BarSig {
        b.div_events.get_or_insert_with(Box::default);
        b
    }

    fn buypt(mut b: BarSig, lad: usize) -> BarSig {
        b.buy_any = LadderMask(b.buy_any.0 | (1 << lad));
        b
    }

    fn sell1pt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell1 = LadderMask(b.sell1.0 | (1 << lad));
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// type2/type3 卖点：sell_any 置位但 sell1 **不**置（type1 专属位空）。
    /// Phase 1 判别夹具——改 sell1→sell_any 前清仓门读不到此点。
    fn sellanypt(mut b: BarSig, lad: usize) -> BarSig {
        b.sell_any = LadderMask(b.sell_any.0 | (1 << lad));
        b
    }

    /// θ 参照预热（层 3/4 各 SUB_COST_MIN_OBS 个锚；同 nested_fugue）。
    fn warmup34() -> Vec<BarSig> {
        let mut bars = Vec::new();
        for j in 0..SUB_COST_MIN_OBS as i64 {
            let mut b = with_ev(bar(100.0), 3, ev_full(BspClass::Sell1, false, 0.0, Some(10 + j)));
            b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 0.0, Some(100 + j)));
            let rows = b.bsp_events.as_deref_mut().unwrap();
            rows[3][0].zd = Some(50.0);
            rows[3][0].zg = Some(51.0);
            rows[4][0].zd = Some(50.0);
            rows[4][0].zg = Some(53.0);
            bars.push(if j == 0 { with_empty_div(b) } else { b });
        }
        bars
    }

    /// 运行 URS：磁带带 dir_flips（E* 涌现层数据），无 trend_flips（URS 不读）。
    fn run(bars: Vec<BarSig>, dir_flips: Vec<(i64, u8, Direction)>) -> PositionalResult {
        let t = SignalTape { bars, dir_flips: Some(dir_flips), ..Default::default() };
        run_positional(&t, 2, URS).unwrap()
    }

    #[test]
    fn parse_and_guards() {
        assert_eq!(PolarityMode::parse("urs"), Some(URS));
        // 缺背驰磁带 ⇒ Err。
        let t = SignalTape {
            bars: vec![with_ev(bar(100.0), 3, ev_full(BspClass::Buy1, true, 100.0, None))],
            dir_flips: Some(Vec::new()),
            ..Default::default()
        };
        assert!(run_positional(&t, 2, URS).unwrap_err().contains("背驰磁带"));
        // 缺 dir_flips ⇒ Err（E* 涌现层判据残缺）。
        let t2 = SignalTape {
            bars: vec![with_empty_div(with_ev(
                bar(100.0),
                3,
                ev_full(BspClass::Buy1, true, 100.0, None),
            ))],
            dir_flips: None,
            ..Default::default()
        };
        assert!(run_positional(&t2, 2, URS).unwrap_err().contains("dir_flips"));
    }

    #[test]
    fn root_enters_full_position_at_top() {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(bar(101.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_root_entries_by_ladder[4], 1, "根开在最高 θ 涌现层");
        assert!((r.final_nav - 101_000.0).abs() < 1e-6);
    }

    #[test]
    fn emergent_climb_blocks_root_level_sell1() {
        // E* 涌现爬升：根@4 入场，父级别 5 在持仓期内生长出 Up 段
        // （dir_flip(bar, 5, Up) 在 entry 之后）⇒ E*=5。根层(4) 的 type1
        // sell1 ∧ located[4] **不**触发清仓（清仓判据看 E*=5，5 层无 sell1）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // entry @ bar 10
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // located[4]
        bars.push(sell1pt(bar(103.0), 4)); // 根层 type1——但 E*=5 ⇒ 不清仓
        bars.push(bar(103.0));
        // 父级别 5 在 entry(bar 10) 后翻 Up（bar 11）⇒ anchor[5]=11 ≥ 10 ∧ Up。
        let r = run(bars, vec![(11, 5, Direction::Up)]);
        assert!(
            r.trades.iter().all(|t| t.exit_reason != "sellpt"),
            "E*=5（父级别持仓期生长 Up）⇒ 根层 type1 不清仓（趋势保护）"
        );
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "根层 confirmed 卖走 E 降成本");
    }

    /// Phase 0 完整看跌递归链：root@4 入场 + 武装 nest@2/3/4 + bi 向下翻转
    /// （fire 全部 nest ⇒ located[2,3,4]，且 dir_state[bi]==Down）+ sell_any@4。
    /// 返回 (bars, dir_flips)，flip 发生在末第二根（estar=4 完整链确认）。
    fn full_bear_chain() -> (Vec<BarSig>, Vec<(i64, u8, Direction)>) {
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // entry
        // 武装 nest_sell@2/3/4（极值 106/108/110）。
        let mut b = with_ev(bar(105.0), 2, ev_full(BspClass::Sell1, false, 106.0, None));
        b = with_ev(b, 3, ev_full(BspClass::Sell1, false, 108.0, None));
        b = with_ev(b, 4, ev_full(BspClass::Sell1, false, 110.0, None));
        bars.push(b);
        // 证据 bar：bi 向下翻转 ⇒ rec_sub_evidence(sub=1)=Down ⇒ fire nest@2/3/4
        // ⇒ located[2,3,4] 全置 ∧ dir_state[bi]==Down。nf_sell[4] 同触发 E 降成本子@3（len=2）。
        bars.push(bar(104.0));
        let evidence_bar = bars.len() as i64 - 1;
        // recover bar：buy_any@3 ⇒ D 回补降成本子@3 ⇒ len 回 1（Phase 3 逐仓独立：
        // 子先独立回补，根才能翻）。located[2,3,4] 价格未破极值故persist。
        bars.push(buypt(bar(103.0), 3));
        // flip bar：sell_any@4 ∧ 完整递归链 ∧ len==1 ⇒ 翻转为子空@3。
        bars.push(sellanypt(bar(102.0), 4));
        bars.push(bar(102.0));
        (bars, vec![(evidence_bar, 1, Direction::Down)])
    }

    #[test]
    fn recursive_confirmed_requires_full_chain_to_a0() {
        // Phase 0 helper 纯函数验证：递归确认 = 链完整[2,estar] ∧ a0(bi) 方向翻向目标侧。
        let mut located: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        located[4] = Some(110.0);
        dir[FIRST_BSP_LADDER - 1] = Some(Direction::Down);
        assert!(
            !recursive_confirmed(4, Direction::Down, &located, &dir),
            "缺中间层 located[2]/[3] ⇒ 链断（弱确认被过滤）"
        );
        located[2] = Some(50.0);
        located[3] = Some(80.0);
        assert!(
            recursive_confirmed(4, Direction::Down, &located, &dir),
            "located[2,3,4] 全置 ∧ bi-down ⇒ 递归确认成立"
        );
        dir[FIRST_BSP_LADDER - 1] = Some(Direction::Up);
        assert!(
            !recursive_confirmed(4, Direction::Down, &located, &dir),
            "a0(bi) 方向 Up ⇒ 卖侧递归基不成立 ⇒ 链断"
        );
    }

    #[test]
    fn weak_confirm_blocks_flip_phase0() {
        // Phase 0 核心价值：弱确认（仅 located[4]，无完整链/无 bi-down）**不触发翻转**。
        // E*=4、sell_any@4、located[4] 齐备，但 located[2]/[3] 缺 + dir_state[bi]≠Down ⇒
        // recursive_confirmed=false ⇒ C 不翻转 ⇒ 根层卖点改走 E 降成本（spawn@3）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4)); // entry
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None))); // located[4]
        bars.push(sellanypt(bar(103.0), 4)); // 根层卖点——但弱确认 ⇒ 不翻转
        bars.push(bar(103.0));
        let r = run(bars, vec![(5, 5, Direction::Up)]); // 无 bi(1) 翻转 ⇒ 递归基不成立
        assert_eq!(
            r.n_nrf_root_flips_by_ladder[3], 0,
            "Phase 0：弱确认（链不完整）被过滤 ⇒ 不翻转"
        );
        assert!(r.n_nrf_spawns_by_ladder[3] >= 1, "根层卖点改走 E 降成本");
    }

    #[test]
    fn full_recursive_chain_enables_flip() {
        // Phase 0+2：完整看跌递归链（located[2,3,4] ∧ bi-down）∧ sell_any@4 ⇒ 翻转为子空@3。
        let (bars, flips) = full_bear_chain();
        let r = run(bars, flips);
        assert_eq!(
            r.n_nrf_root_flips_by_ladder[3], 1,
            "完整递归链 ⇒ 翻转 = 子空@3 诞生（spawn m=N）"
        );
        assert!(
            r.trades.iter().all(|t| t.exit_reason != "sellpt"),
            "翻转非清仓——无 sellpt trade（仓位连续不中断）"
        );
    }

    #[test]
    fn flip_child_carries_negate_line() {
        // v3 死因②修复：翻转子空携带 027:25 否定线（=located[4] 极值 110）；升破 ⇒ B 否定有界平仓。
        let (mut bars, flips) = full_bear_chain();
        bars.push(bar(115.0)); // 升破 110 ⇒ B 否定平子空
        let r = run(bars, flips);
        assert_eq!(r.n_nrf_root_flips_by_ladder[3], 1, "完整链 ⇒ 翻转产生子空@3");
        assert_eq!(
            r.n_nrf_negate_closes_by_ladder[3], 1,
            "子空 negate_line=located 极值 110，升破 ⇒ B 否定平仓（v3 死因②修复）"
        );
    }

    #[test]
    fn spawn_releases_theta_quota() {
        // §2 卖出原子：根@4 confirmed 卖 ⇒ 释放 m=N×θ₃/θ_total 给子空@3。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(bar(104.0));
        let r = run(bars, vec![]);
        assert_eq!(r.n_nrf_spawns_by_ladder[3], 1, "子空@3 诞生");
        let short = r.trades.iter().find(|t| t.polarity == Polarity::Short).unwrap();
        assert!((short.shares - 250.0).abs() < 1e-9, "θ 配额 m = 1000×1/4");
        assert!((r.final_nav - 104_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn recovery_refills_parent() {
        // §3 回补原子：confirmed 买@3 ⇒ 平子 + 父回满 + 降成本。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(buypt(bar(96.0), 3)); // 子级别走势完美 ⇒ 回补
        bars.push(bar(96.0));
        let r = run(bars, vec![]);
        let rec = r.trades.iter().find(|t| t.exit_reason == "recover").unwrap();
        assert_eq!((rec.ladder, rec.polarity), (3, Polarity::Short));
        let pnl = rec.shares * (rec.entry_price - rec.exit_price);
        assert!((pnl - 2000.0).abs() < 1e-6, "子 P&L = 250×8 = 2000");
        assert!((r.final_nav - 98_000.0).abs() < 1e-6, "final={}", r.final_nav);
    }

    #[test]
    fn conservation_invariant_holds() {
        // §8.1 守恒：递归流转不增不减（引擎内部每 bar 检验，未 Err 即通过）。
        let mut bars = warmup34();
        bars.push(buypt(bar(100.0), 4));
        bars.push(with_ev(bar(105.0), 4, ev_full(BspClass::Sell1, false, 110.0, None)));
        bars.push(with_ev(bar(104.0), 3, ev_full(BspClass::Sell1, true, 0.0, None)));
        bars.push(buypt(bar(95.0), 3));
        bars.push(bar(95.0));
        let r = run(bars, vec![]);
        // 根回满 1000 股；现金流闭合 = 1000×95 + 250×(104−95)。
        let expect = 1000.0 * 95.0 + 250.0 * 9.0;
        assert!((r.final_nav - expect).abs() < 1e-6, "final={} expect={expect}", r.final_nav);
    }
}
