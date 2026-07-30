//! 会计层：森林 nav/settle/close_voice/σ-不变配额 spawn + 成本门（架构 §4.3/§5.6）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计来源：架构 §5（操作群论表述）+ §10.3（价格 ambient ⊥ 螺旋）。语义 **bit-exact
//! 复用** unn 引擎 `isolated_fugue`/`unified_necessity` 的会计原语（R5/R6）——这是
//! "会计 = 同一螺旋第三投影"（架构 §2.2）：操作层（群作用）与会计层正交，v2 复用会计
//! 语义而只重构操作触发。
//!
//! ## 关键不变量（prove_n8 守，架构 §10.3）
//! - `Σ(active voices.units) = n_base`（units = 唯一 σ-不变 Casimir，T48）。
//! - 同价 c 操作前后 NAV 中性（价值中性）。
//! - 含价格量（NAV 的 ×c 项）随 r=λᵏ 标度，**非 σ-不变**——守恒守卫守 units 非 NAV 总额。
//!
//! ## MtM 根空头 vs frozen 子空头（范畴分层，T8，架构 §6.3）
//! - **子空头**（parent=Some）= 内部负债（父吸收）⇒ 冻结 capital，无独立 MtM。
//! - **根空头**（parent=None，T14 翻空后）= 外部市场负债 ⇒ MtM = `capital − units×c`。

use super::params::{SUB_COST_K, SUB_COST_MIN_OBS, SUB_COST_Q, SUB_FRICTION_RT, SUB_SPAWN_FRAC};
use super::prove::{prove_cross_level_closure, prove_theta_sigma_invariant};
use super::result::SpiralResult;
use super::state::{GroupAction, SpiralState};
use super::voice::{SpiralVoice, VoiceStatus};
use crate::trading::depth_ref::DepthRef;
use crate::trading::positional::LayerTrade;
use crate::trading::types::Polarity;

// ════════════════════════════ NAV（物理单真值）════════════════════════════

/// 物理 NAV：自由现金 + 活跃多头在手×价 + 活跃空头净值。
///
/// 空头净值口径分两类（T8 范畴分层）：
/// - 子空头（parent=Some）= frozen capital（父吸收内部负债）。
/// - 根空头（parent=None）= MtM `capital − units×c`（外部市场负债，T14 翻空后）。
pub fn nav(voices: &[SpiralVoice], free: f64, c: f64) -> f64 {
    let mut v = free;
    for x in voices.iter().filter(|x| x.is_active()) {
        match x.dir() {
            Polarity::Long => v += x.units * c,
            Polarity::Short => {
                v += if x.parent.is_none() {
                    x.capital - x.units * c
                } else {
                    x.capital
                };
            }
        }
    }
    v
}

// ════════════════════════════ settle（相位结算）════════════════════════════

/// 结算一个 voice 的当前相位（trade 行 + 计数 + realized_pnl）。返回视图 P&L。
/// 纯记账观测（不动 free/units/capital）——对 N8 中性。
pub fn settle(
    voices: &mut [SpiralVoice],
    id: usize,
    exit_bar: i64,
    exit_price: f64,
    reason: &'static str,
    res: &mut SpiralResult,
) -> f64 {
    let v = &voices[id];
    let dir = v.dir();
    let ladder = v.ladder();
    let pnl = match dir {
        Polarity::Long => v.units * (exit_price - v.basis),
        Polarity::Short => v.units * (v.basis - exit_price),
    };
    if dir == Polarity::Short {
        res.short_net_cash_by_ladder[ladder] += pnl;
    }
    res.trades.push(LayerTrade {
        ladder: ladder as u8,
        entry_bar: v.entry_bar,
        entry_price: v.basis,
        exit_bar,
        exit_price,
        shares: v.units,
        weight_at_entry: 1.0,
        deferred_bars: 0,
        partial: false,
        exit_reason: reason,
        polarity: dir,
    });
    res.n_exits_by_ladder[ladder] += 1;
    voices[id].realized_pnl += pnl;
    pnl
}

// ════════════════════════════ close_voice（后序关闭）════════════════════════════

/// 关闭一个 voice（后序：先 cascade 关活跃子树，再结算自身并返还父/现金）。
/// 会计原语 bit-exact 复用 unn `close_voice`（pop_tail 森林形式，§8 双层会计）。
///
/// - `px` = 自身 trade 行价；子树 cascade 用市价 `c`。
/// - `at_point` = 自身关闭是否在 confirmed 买卖点（§7 earning 时机）。
#[allow(clippy::too_many_arguments)]
pub fn close_voice(
    id: usize,
    bar: i64,
    px: f64,
    c: f64,
    reason: &'static str,
    at_point: bool,
    voices: &mut Vec<SpiralVoice>,
    free: &mut f64,
    n_base: &mut f64,
    res: &mut SpiralResult,
) {
    debug_assert!(voices[id].is_active(), "close_voice 前提：voice 活跃");
    // ① 后序：先级联关闭活跃子树（每子返还到本 voice）。
    let kids = voices[id].children.clone();
    for k in kids {
        if voices[k].is_active() {
            let lad = voices[k].ladder();
            close_voice(k, bar, c, c, "cascade", false, voices, free, n_base, res);
            res.n_cascade_closes_by_ladder[lad] += 1;
        }
    }
    // ② 结算自身相位。
    settle(voices, id, bar, px, reason, res);
    // ③ 返还父/现金（pop_tail 会计原语，id 寻址）。
    let dir = voices[id].dir();
    let units = voices[id].units;
    let capital = voices[id].capital;
    match voices[id].parent {
        None => {
            // 根弹出：在手变现回 free，N_base 归零（清仓/否定/EOD；T14 删根恒多）。
            match dir {
                Polarity::Long => *free += units * c + capital,
                // 空头根（MtM）：买回 units×c 平空，余 capital−units×c 回 free（真实兑现 P&L）。
                Polarity::Short => *free += capital - units * c,
            }
            *n_base = 0.0;
        }
        Some(p) => match dir {
            // 空子返还多父（回补/否定/强平）：capital 弹药买回 u′=min(u,K/c)，
            // 缩水 δ 传播 + N 重定基；剩余现金 = 降成本金额 ≡ 子 P&L。
            Polarity::Short => {
                let u_back = units.min(capital / c);
                let leftover = capital - u_back * c;
                let shortfall = units - u_back;
                // A4(T38)：u_back≤units ⇒ shortfall≥0（单位永久缩水 δ≥0，非负 cost_reduction）。
                debug_assert!(
                    shortfall >= -1e-9 * units.max(1.0),
                    "A4(T38) 违反@bar {bar}：亏损重定基 shortfall={shortfall} < 0"
                );
                voices[p].units += u_back;
                *n_base -= shortfall;
                res.shrink_units += shortfall;
                let pool = voices[p].cost_pool.max(0.0);
                let reduce = leftover.min(pool);
                voices[p].cost_pool -= reduce;
                let excess = leftover - reduce;
                if excess > 0.0 {
                    if at_point && voices[p].dir() == Polarity::Long {
                        // §7 earning：纯利润在买点买入 Δ，N 重定基；basis 加权。
                        let dq = excess / c;
                        debug_assert!(
                            dq > 0.0 && dq.is_finite(),
                            "A4(T38) 违反@bar {bar}：earning 重定基 dq={dq} 非正/非有限"
                        );
                        let pu = voices[p].units;
                        voices[p].basis = (voices[p].basis * pu + excess) / (pu + dq);
                        voices[p].units += dq;
                        *n_base += dq;
                        res.earning_units += dq;
                        *free += reduce;
                    } else {
                        *free += leftover;
                    }
                } else {
                    *free += leftover;
                }
            }
            // 多子（孙）返还空父（§5 递归）：卖出所得回流父 capital，利润递减父成本池。
            Polarity::Long => {
                let proceeds = units * c;
                voices[p].capital += proceeds;
                voices[p].units += units;
                let profit = units * (c - voices[id].basis);
                let reduce = profit.max(0.0).min(voices[p].cost_pool.max(0.0));
                voices[p].cost_pool -= reduce;
            }
        },
    }
    voices[id].status = VoiceStatus::Closed;
    voices[id].units = 0.0;
    if let Some(p) = voices[id].parent {
        voices[p].refresh_status();
    }
}

// ════════════════════════════ σ-不变配额 + 成本门 spawn（E）════════════════════════════

/// **配额 σ-不变规范（T18×T48×T59，第15环 = 542号）**：降成本释放给子 voice 的
/// 配额 `m_quota = f × p_units`，`f = 1/λ` 级别无关（σ-不变常数）。**刻意不取 `sub`
/// 参数**——σ-不变性的精确编码 = 配额是父在手的级别无关函数（架构 §5.6）。
pub fn sigma_invariant_quota(p_units: f64) -> f64 {
    p_units * SUB_SPAWN_FRAC
}

/// 成本门动态 spawn（N4 第16环 + E 降成本 = σ⁻¹）：父 voice 释放 σ-不变配额给子
/// voice @ `sub = parent.ladder − 1`。**无 floor 参数**——终止纯由成本门
/// （θ=None ∨ θ<K×friction）。**子 voice 状态经群作用 `σ⁻¹ ∘ τ` 构造**
/// （RadialDescend 下沉一级 + ChiralSeam 方向交替，环22）⟹ "操作=群作用"在 spawn
/// 路径显形。返回成功时的 `(child_id, sub)`。
///
/// **bit-exact 前提（Step 6）**：成本门 θ 来源 = unn 的 `DepthRef`（复用 pub 类型，
/// 不改现有引擎）——与 unn `try_spawn_cost_gated` 同一 `theta(sub, None, q, min_obs)`
/// 调用，使 E spawn 决策与 unn 逐 bar 一致（差异隔离到群作用路由层）。
pub fn try_spawn_cost_gated(
    parent_id: usize,
    bar: i64,
    c: f64,
    depth: &DepthRef,
    voices: &mut Vec<SpiralVoice>,
    res: &mut SpiralResult,
) -> Option<(usize, usize)> {
    let p_state = voices[parent_id].state;
    let p_ladder = voices[parent_id].ladder();
    let p_dir = voices[parent_id].dir();
    let p_units = voices[parent_id].units;
    let p_capital = voices[parent_id].capital;
    // N4：递归基 = bi(a0)。sub = parent−1；sub 在 [0, FIRST_BSP) 时 θ 恒 None（无中枢
    // 事件）⇒ 自然终止（无 floor 检查——纯成本门）。
    let sub = p_ladder.checked_sub(1)?;
    match depth.theta(sub, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
        None => {
            res.n_noref_rejects_by_ladder[sub] += 1; // N4：势不可测=势不存在（递归终止）
            None
        }
        Some(tq) if tq < SUB_COST_K * SUB_FRICTION_RT => {
            res.n_cost_rejects_by_ladder[sub] += 1; // N4：势幅度<成本（势消失）
            None
        }
        Some(_) => {
            let m_quota = sigma_invariant_quota(p_units);
            // 配额 σ-不变守卫（T18×T48×T59，542号缺瓦）：独立内联表达 ⇒ 漂移回 sub-依赖
            // 分配即 panic（N8 守 Σunits 守恒不覆盖 σ-不变性，故须独立守卫）。
            prove_theta_sigma_invariant(m_quota, p_units, sub, bar);
            let m = match p_dir {
                Polarity::Long => m_quota,
                Polarity::Short => m_quota.min(p_capital / c),
            };
            if !(m > 0.0 && m.is_finite()) {
                return None;
            }
            // 子状态 = σ⁻¹(parent) 下沉一级（H¹ 生成元 Δr=−1）再 τ 翻向（环22 方向交替）。
            // 群作用路由："操作=群作用"在 spawn 路径的具体实现。
            let descended = GroupAction::RadialDescend
                .apply(p_state)
                .expect("sub=parent−1≥FIRST_BSP_LADDER>0 ⇒ σ⁻¹ 有像");
            // Δr=−1 硬断言（H¹ 闭合，closure.rs P-close 的运行时守）。
            prove_cross_level_closure(p_ladder, descended.r as usize);
            res.cross_level_closures += 1;
            let child_state = GroupAction::ChiralSeam
                .apply(descended)
                .expect("子状态 φ=0（descended 继承 parent.phi=0）⇒ τ 合法");
            let child_dir = match p_dir {
                Polarity::Long => Polarity::Short,
                Polarity::Short => Polarity::Long,
            };
            debug_assert_eq!(
                if child_state.eps >= 0 {
                    Polarity::Long
                } else {
                    Polarity::Short
                },
                child_dir,
                "ChiralSeam 后子手性与方向交替规范不一致"
            );
            let mut child = SpiralVoice::new(
                child_state,
                m,
                c,
                if child_dir == Polarity::Short {
                    m * c
                } else {
                    0.0
                },
                bar,
                Some(parent_id),
            );
            child.cost_pool = m * c;
            child.acted_bar = bar;
            let child_id = voices.len();
            voices[parent_id].units -= m;
            if child_dir == Polarity::Long {
                voices[parent_id].capital -= m * c;
            }
            voices[parent_id].children.push(child_id);
            voices[parent_id].refresh_status();
            voices.push(child);
            res.n_spawns_by_ladder[sub] += 1;
            res.n_entries_by_ladder[sub] += 1;
            Some((child_id, sub))
        }
    }
}

/// 构造根 voice（F 建仓 = σ 塔起点 @ source，§5.1）。多头满仓，capital=0。
/// 状态 = `SpiralState::root(source)`（φ=0 奇点 + 径向源 + 多头 ε=+1）。
pub fn make_root(source: usize, units: f64, c: f64, bar: i64) -> SpiralVoice {
    let mut v = SpiralVoice::new(SpiralState::root(source as u8), units, c, 0.0, bar, None);
    v.cost_pool = units * c;
    v.acted_bar = bar;
    v
}
