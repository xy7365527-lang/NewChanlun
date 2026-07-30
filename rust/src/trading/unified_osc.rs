//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! unified_osc — 统一配置 U 的 osc 层：相位递归路由（Phase-Recursive Routing）。
//!
//! 设计：`analysis/unified_config_regime_research.md` §3（候选架构）。
//! 任务（2026-06-12）：缠论不能被标的特化——一个配置，所有标的，零标的级参数。
//! 白名单家族（co/h1/h3/h4）的三条二分线互不重合的在册解释 = 各机制判别量是
//! 49课:52 二相判定在不同级别/侧面的投影；统一形式 = **不问"这个标的开不开
//! osc"，而问"此刻走势结构在哪个级别处于震荡相且振幅过门"**（038:28 regime
//! 由分解级别内生定义——零外部标签的最强原文支持）。
//!
//! ## 三门合取（全部从走势结构实时读取，零新参数）
//!
//! 对锚层 k 自下而上找第一个满足合取的级别 j ∈ {k, k+1, …}：
//! - **① 相位门** ¬in_trend(j)：049:52"那种中枢完成后的向上移动时的差价是
//!   不能做的……前提是中枢震荡依旧"。in_trend(j) = trend_state[j] ∧ dir==Up
//!   （17课趋势定义 ≥2 同向中枢的引擎直读，fusion_t 同款时钟）。①不过 ⇒
//!   继续上移（049:44"如果整个市场都找不到值得介入的……就可以根据这些大点
//!   级别的中枢震荡来操作"；026:183 趋势态级别上移）。
//! - **② 振幅门** θ_q(j) ≥ k×friction：035:30"级别越小，平均的买卖点间波幅
//!   也越小……不足以让交易成本、交易误差等相对买卖点间波幅足够小，这样的
//!   操作，从长期的角度看，是没有意义的"——H4 在册判据逐字（DepthRef 因果
//!   滚动 P50，零前瞻；常数 = 35课成本门 SUB_COST_K × SUB_FRICTION_RT，
//!   零新参数）。
//! - **③ 强震荡门（g2）** 当前震荡围绕前上涨 move 最后中枢：093:26"整个
//!   震荡的区间，就要以上涨的最后一个中枢为依据，只要围绕着该区间，就是
//!   强的震荡，否则……就是弱的震荡了。弱的震荡，一般一旦确认，最好还是
//!   不参与"。**严格可导出，零编纂注依赖**（research v2 §2.4 g2 推导链：
//!   (1,0) = 向上笔顶分型构造 [91:32] 纯结构定义——v1"均线载体剥离 declared
//!   近似"已勘误 [v2 偏差登记簿 #4]；093:24 原文桥接周期图记号到小级别
//!   上涨走势类型）。操作化（"围绕该区间"的结构读法）：j 层当前存活中枢
//!   区间 [zd,zg] 与该层最近一次 dir==Up 期间的最后中枢区间（UpRef 快照）
//!   **有交叠** ⇒ 强震荡（当前震荡结构仍以该区间为依据）；完全脱离
//!   （整体跌落）⇒ 弱震荡拒开。点态读法（当前价格 vs 参照区间邻域）是
//!   另一合法投影——本实现取结构读法（震荡的主语是"震荡的区间"非瞬时
//!   价格），差异列结果文档边界条件，净效应由 P5 消融裁决。参照不可定义
//!   （该层从未有 dir==Up 的存活中枢）⇒ 保守拒绝独立计数（"不静默放行"
//!   在册先例）。有效域边界（v2 偏差登记簿 #3）：093:28 盘整背驰结束的
//!   情形二是编纂注重建——g2 有效域暂限趋势背驰结束后的震荡，情形二待
//!   源头审计。g2 退出词汇（093:22"最后一次就不回补"）本轮由三卖否决 +
//!   ZD 兑现近似承载（declared 近似，列开放轴）。
//!
//! 若存在最小的 j：在 j 层开 osc 腿；全塔无满足级别 ⇒ 不开，恒仓吃趋势
//! （053:34"在趋势的情况下，一般小级别的买卖点并不一定要参与"= fusion_t
//! 基座零接触退化——P2 的构造性保证）。
//!
//! ## 路由语义的严格声明（090号近似义务）
//!
//! "第一个满足合取的级别"按字面实现：①②③任一不过都继续上扫（②不过 =
//! 该级别波幅不足以摊薄成本 ⇒ 035:32 多级别立体授权下找更大级别；③不过 =
//! 该级别震荡是弱的 ⇒ 更大级别的震荡结构独立判定）。research §3.3 对账表
//! "θ_q(k+1) 不过门 ⇒ 不开"描述的是典型结局（更高层通常无中枢/无触发），
//! 不是另一种算法——此读法差异列结果文档边界条件（P4 判据照常可判：CL 的
//! 上移腿净亏按路由层分桶 osc_net_cash_at_level 直读）。
//!
//! ## 与在册失效案例的对账（research §3.3）
//!
//! - H3 OKLO 死因一（k+1 是更大级别僵尸腿）：递归路由下 k+1 若 in_trend
//!   为真继续上移；全塔趋势 ⇒ 不开（OKLO 强趋势段全塔同向自然关闭）。
//! - H3 OKLO 死因二（槽竞争）：本引擎中每层 slice 即独立槽——层 k 的路由腿
//!   只动层 k 自己的股数，不占其它层的开腿资格（j 层声部自身的 osc 与层 k
//!   路由到 j 的腿并行不悖）。LOU 语境的"上移腿占 k 层槽"在 slice 模型中
//!   无对应物；residual 竞争 = 层 k 股数在外期间 k 层节奏不可重入，这是
//!   物理事实（同一笔筹码不能卖两次）非会计槽位，列结果文档边界条件。
//! - co 伤正域（深回调盈利腿陪葬）：递归路由不删除趋势态 osc 而是上移。
//!
//! ## osc 腿会计（049:64 + 26课恒仓极性）
//!
//! 开（先卖）：j 层触发判据 c ≥ ZG(j) ∧ sub_sell(j−1)（LOU osc 开腿逐字；
//! 049:64"在中枢上方全部抛出筹码"⇒ 全抛本层 slice）。卖出所得入共享池
//! （C 轴判决在册：资金池耦合根因被否证，earmark 不复制）。
//! 回补（如数接回）出口集（一 bar 一动作，优先序注释见 `step_exit`）：
//! - 趋势相满仓义务（049:52，j 层相位翻趋势 ⇒ 强制回补；counter_sub 先例）；
//! - 44课铰链（044:44）：本层 k 级卖点先到 ⇒ 卖出升级为减仓出清
//!   （身份事后授予，记账以实际变现点——hinge_escalate 在册同构）；
//! - 中枢死亡（非三卖）⇒ 立即回补（"中枢向上移动时就应该满仓"）；
//! - ZD 触线兑现（L0 定理：恒盈利）；
//! - sc 中枢上移出口（049:54 新中枢语义：新 ZD > 锚 ZG ⇒ 立即回补）；
//! - 本层 k 级买点（026:34"只要有买点就要买入"）；
//! - **三卖否决**（049:52"一旦出现第三类卖点，就不能回补了"）：锚中枢被
//!   confirmed Sell3 终结 ⇒ 回补通道收窄为 ZD 触线（更低处）/铰链/eod
//!   （o3s 在册严格形式同构）。
//! 资金不足 ⇒ restore_due 逐 bar 重试（D7 推迟同构，义务恒在）。
//!
//! 谱系：534号（49课二相消解）、533号（结构滞后三层谱型——kind 行滞后仍是
//! ①门固有暴露窗口）、regime 函数第1-9例（本配置若 P1 达成则九例二分线
//! 统一为三门内生取值）。93课强弱震荡此前无谱系条目——③门进入实装，
//! "均线载体剥离"概念分离须新开谱系记录（research 结果包第5要素）。

use super::center_book::{CenterBook, LiveCenter};
use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
use super::depth_ref::DepthRef;
use super::positional::{LayerState, LayerTrade, PositionalResult};
use super::positional_fusion::{PhaseView, SUB_COST_K, SUB_FRICTION_RT};
use super::tape::BarSig;
use super::types::{Polarity, FIRST_BSP_LADDER, MAX_LADDER};
use crate::stroke::Direction;

/// osc 腿在外态（层 k 的 slice 已按 j 层节奏卖出，等待回补/升级裁决）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct OscOut {
    /// 卖出的股数（= 卖出 bar 本层全部股数，049:64"全部抛出筹码"）。
    pub shares: f64,
    pub sell_bar: i64,
    pub sell_price: f64,
    /// 回补义务已触发但资金不足 ⇒ 逐 bar 重试（D7 推迟同构）。
    pub restore_due: bool,
    /// 路由层 j（出口跟随锚——H3 在册"出口跟随锚不跟随槽"）。
    pub alad: usize,
    /// 锚中枢快照（开腿时 j 层存活中枢）。
    pub cs: i64,
    pub zd: f64,
    pub zg: f64,
    /// 三卖否决 latch（049:52"不能回补"——置位后回补通道收窄）。
    pub dead_down: bool,
}

/// 统一配置 U 的 osc 层状态（per-bar 由 run_fusion 驱动；fusion_t 路径
/// osc=Off 时本结构恒空——零接触退化）。
#[derive(Debug)]
pub(crate) struct OscLayer {
    /// 层 k → 在外腿。不变式：outs[k].is_some() ⇒ layers[k] 为 Long。
    pub outs: [Option<OscOut>; MAX_LADDER],
    /// ③门参照：层 j 最近一次 dir==Up 期间的最后存活中枢区间 (zd, zg)
    /// （093:26"上涨的最后一个中枢"——dir 翻落时快照自然冻结）。
    up_ref: [Option<(f64, f64)>; MAX_LADDER],
}

impl OscLayer {
    pub fn new() -> Self {
        OscLayer {
            outs: [None; MAX_LADDER],
            up_ref: [None; MAX_LADDER],
        }
    }

    /// ③门参照刷新（每 bar，市场性质——与持仓/配置无关）。dir==Up 期间
    /// 持续跟随存活中枢；dir 翻落后保持最后快照 = "上涨的最后一个中枢"。
    /// 读法声明：93课"上涨"按 research §2 映射行操作化为 dir==Up 的 move
    /// （方向行直读），不要求 kind==Trend——更严的"上涨走势类型"读法列
    /// 结果文档边界条件。
    pub fn observe_refs(&mut self, book: &CenterBook, dir_state: &[Option<Direction>; MAX_LADDER]) {
        for j in FIRST_BSP_LADDER..MAX_LADDER {
            if dir_state[j] == Some(Direction::Up) {
                if let Some(lc) = book.alive(j) {
                    self.up_ref[j] = Some((lc.zd, lc.zg));
                }
            }
        }
    }

    /// 相位递归路由：自下而上找第一个满足 ①∧②∧③ 的级别 j。
    /// 拒因逐级计数（P3 机制可观测性——判别力为零的门按 H2 先例否证）。
    /// 计数条件声明：调用方以"本 bar 存在次级别卖证据（sell_any≠0）"预滤，
    /// 全部 n_route_* 计数共享该条件（纯性能预滤——无卖证据的 bar 任何
    /// 层都不可能触发开腿，路由结果无消费者）。
    #[allow(clippy::too_many_arguments)]
    fn route(
        &self,
        k: usize,
        strong_gate: bool,
        h1_freeze: bool,
        phase: &dyn Fn(usize) -> PhaseView,
        book: &CenterBook,
        depth_ref: &DepthRef,
        res: &mut PositionalResult,
    ) -> Option<(usize, LiveCenter)> {
        for j in k..MAX_LADDER {
            // ① 相位门：osc 词汇的对象域 = OSC 相位（049:52 前提"中枢震荡
            // 依旧"；MOVE↓ 同拒——049:40 不参与下跌）。不过 ⇒ 上移
            // （049:44/026:183 递归形式）。KindDir 时钟下 ≠Osc ⟺ ==MoveUp，
            // 与在册 phase_up 布尔行为逐位等值。
            if phase(j) != PhaseView::Osc {
                res.n_route_phase_skips[j] += 1;
                continue;
            }
            // 对象存在性（先于一切门——"对象不存在时门无可拦截之物"在册）
            let Some(lc) = book.alive(j) else {
                res.n_route_no_center[j] += 1;
                continue;
            };
            // ② 振幅门（H4 逐字：θ_q < k×friction ⇒ 035:30 无长期意义；
            //    参照不可定义 ⇒ 保守拒绝独立计数）
            match depth_ref.theta(j, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                None => {
                    res.n_route_amp_noref[j] += 1;
                    continue;
                }
                Some(t) if t < SUB_COST_K * SUB_FRICTION_RT => {
                    res.n_route_amp_rejects[j] += 1;
                    continue;
                }
                Some(_) => {}
            }
            // H1 candidate 冻结（h1_freeze；osc_candidate_freeze 在册判据
            // 下沉，LOU 链序 H4 之后逐字）：锚层存在未决 candidate type3
            // 离开段 ⇒ 走势方向未定 ⇒ 不开。窗口由价格回中枢边界否定
            // （run_fusion 每 bar negate_pending_departure）。
            if h1_freeze && book.has_pending_departure(j) {
                res.n_route_h1_freezes[j] += 1;
                continue;
            }
            // ③ 强震荡门（093:26；fusion_uw 消融臂关闭本门）。
            //    NaN 边界比较恒 false ⇒ 拒（保守方向，CenterBook NaN 纪律同构）
            if strong_gate {
                match self.up_ref[j] {
                    None => {
                        res.n_route_weak_noref[j] += 1;
                        continue;
                    }
                    Some((rzd, rzg)) => {
                        let overlap = lc.zd <= rzg && lc.zg >= rzd;
                        if !overlap {
                            res.n_route_weak_rejects[j] += 1;
                            continue;
                        }
                    }
                }
            }
            res.n_route_selected_by_level[j] += 1;
            return Some((j, lc));
        }
        res.n_route_exhausted += 1; // 全塔拒绝 ⇒ 恒仓吃趋势（053:34）
        None
    }

    /// 开腿尝试（阶段 B：层 k Long 且无在外腿时每 bar 调用）。
    /// 触发判据 = LOU osc 开腿逐字：!frozen(j) ∧ c ≥ ZG(j) ∧ sub_sell(j−1)。
    /// j ≥ k ≥ FIRST_BSP_LADDER ⇒ j−1 ≥ 1（笔=a0 越界在类型层不可达）。
    #[allow(clippy::too_many_arguments)]
    pub fn try_open(
        &mut self,
        k: usize,
        c: f64,
        bar: i64,
        sig: &BarSig,
        strong_gate: bool,
        h1_freeze: bool,
        phase: &dyn Fn(usize) -> PhaseView,
        book: &CenterBook,
        depth_ref: &DepthRef,
        layers: &[LayerState; MAX_LADDER],
        pool: &mut f64,
        res: &mut PositionalResult,
    ) {
        debug_assert!(self.outs[k].is_none(), "调用前提：层 k 无在外腿");
        let LayerState::Long { shares, .. } = layers[k] else {
            return;
        };
        let Some((j, lc)) = self.route(k, strong_gate, h1_freeze, phase, book, depth_ref, res)
        else {
            return;
        };
        let sub_sell = sig.sell_any.get(j - 1);
        if book.is_frozen(j) || !(c >= lc.zg) || !sub_sell {
            return;
        }
        *pool += shares * c; // 股数→现金等价转换（NAV 不变）
        self.outs[k] = Some(OscOut {
            shares,
            sell_bar: bar,
            sell_price: c,
            restore_due: false,
            alad: j,
            cs: lc.seg_start,
            zd: lc.zd,
            zg: lc.zg,
            dead_down: false,
        });
        res.n_osc_opens_by_ladder[k] += 1;
        res.n_osc_open_at_level[j] += 1;
        if j > k {
            res.n_osc_upshift_opens_by_ladder[k] += 1;
        }
    }

    /// 出口集（阶段 A：层 k 有在外腿时每 bar 调用；一 bar 一动作）。
    ///
    /// 优先序：三卖 latch 更新 → 趋势相满仓义务（049:52，counter_sub 先例
    /// 同序；相位查路由层 alad——出口跟随锚）→ 44课铰链（本层卖点升级出清，
    /// 前提 ¬in_trend(k)——k 趋势相内 k 卖点无响应，049:52 停削语义对在外
    /// 腿同样成立）→ 回补触发集（死亡>ZD>sc>k买>义务遗留，LOU 出口序
    /// 同构）。dead_down 下回补通道收窄为 ZD 触线/义务遗留（049:52"不能
    /// 回补"，铰链/eod 是卖侧出口不受限）。
    #[allow(clippy::too_many_arguments)]
    pub fn step_exit(
        &mut self,
        k: usize,
        c: f64,
        bar: i64,
        sig: &BarSig,
        nf_sell: bool,
        nf_buy: bool,
        phase: &dyn Fn(usize) -> PhaseView,
        book: &CenterBook,
        layers: &mut [LayerState; MAX_LADDER],
        pool: &mut f64,
        res: &mut PositionalResult,
    ) {
        let mut osc = self.outs[k].expect("调用前提：outs[k] 为 Some");
        res.n_osc_out_bars_by_ladder[k] += 1;
        if osc.alad > k {
            res.n_osc_up_out_bars_by_ladder[k] += 1;
        }
        // 三卖否决 latch（049:52——方向判据：仅向下终结触发）
        if !osc.dead_down && book.is_dead_down(osc.alad, osc.cs) {
            osc.dead_down = true;
            self.outs[k] = Some(osc);
            res.n_osc_sell3_vetos_by_ladder[k] += 1;
        }
        // 1. 趋势相满仓义务（j 层相位翻 MOVE↑ ⇒ 对象域消失，强制回补，
        //    049:52；MOVE↓ 不强制——"不能回补"方向由 dead_down latch 承载）
        if !osc.dead_down && phase(osc.alad) == PhaseView::MoveUp {
            if self.try_restore(k, c, bar, pool, layers, res) {
                res.n_osc_phase_restores_by_ladder[k] += 1;
            }
            return;
        }
        // 2. 44课铰链：本层 k 级卖点先到 ⇒ 升级为减仓出清（身份事后授予，
        //    记账以实际变现点 = 卖出 bar close；hinge_escalate 在册同构）。
        //    前提 ¬in_trend(k)：counter_sub 在册铰链只在 ¬tp(k) 分支可达
        //    （`if tp {回补} else if sell {升级}`）——049:52 趋势相停削语义
        //    下 k 卖点无响应（既不削也不升级）。上移腿（alad>k）在 k 趋势相
        //    内被 k 卖点升级出清 = 绕过停削的旁路，禁止（与基座零接触矛盾）。
        if (sig.sell_any.get(k) || nf_sell) && phase(k) != PhaseView::MoveUp {
            let LayerState::Long {
                entry_bar,
                entry_price,
                weight,
                deferred_bars,
                partial,
                ..
            } = layers[k]
            else {
                unreachable!("osc 在外 ⇒ 本层恒 Long（腿生命周期内层不变迁）")
            };
            res.trades.push(LayerTrade {
                ladder: k as u8,
                entry_bar,
                entry_price,
                exit_bar: osc.sell_bar,
                exit_price: osc.sell_price,
                shares: osc.shares,
                weight_at_entry: weight,
                deferred_bars,
                partial,
                exit_reason: "osc_escalate",
                polarity: Polarity::Long,
            });
            res.n_osc_escalates_by_ladder[k] += 1;
            res.n_exits_by_ladder[k] += 1;
            layers[k] = LayerState::Flat;
            self.outs[k] = None;
            return;
        }
        if (sig.sell_any.get(k) || nf_sell) && phase(k) == PhaseView::MoveUp {
            // k 趋势相内 k 卖点对在外腿的停削抑制（可观测：上移腿在 k 趋势
            // 中本会被铰链旁路出清的事件数）。
            res.n_osc_trend_hold_sells_by_ladder[k] += 1;
        }
        // 3. 回补触发集（归因按优先序：死亡 > ZD > sc > k买 > 义务遗留）
        let dead = book.is_dead(osc.alad, osc.cs);
        let zd_hit = c <= osc.zd;
        let shifted = book
            .alive(osc.alad)
            .is_some_and(|lc| lc.seg_start > osc.cs && lc.zd > osc.zg);
        let kbuy = sig.buy_any.get(k) || nf_buy;
        let triggered = if osc.dead_down {
            zd_hit || osc.restore_due
        } else {
            dead || zd_hit || shifted || kbuy || osc.restore_due
        };
        if triggered && self.try_restore(k, c, bar, pool, layers, res) {
            if !osc.dead_down && dead {
                res.n_osc_death_restores_by_ladder[k] += 1;
            } else if zd_hit {
                res.n_osc_zd_restores_by_ladder[k] += 1;
            } else if !osc.dead_down && shifted {
                res.n_osc_shift_restores_by_ladder[k] += 1;
            } else if !osc.dead_down && kbuy {
                res.n_osc_kbuy_restores_by_ladder[k] += 1;
            } else {
                res.n_osc_due_restores_by_ladder[k] += 1;
            }
        }
    }

    /// 回补（如数接回）。成功 ⇒ trade 行（"osc_diff"）+ 本层 Long 重锚到
    /// 当下 + 清在外态；pool 不足 ⇒ restore_due 逐 bar 重试并计数
    /// （try_restore/counter_sub 逐字同构，耦合资金语义 escrow≡0）。
    fn try_restore(
        &mut self,
        k: usize,
        c: f64,
        bar: i64,
        pool: &mut f64,
        layers: &mut [LayerState; MAX_LADDER],
        res: &mut PositionalResult,
    ) -> bool {
        let osc = self.outs[k].expect("调用前提：outs[k] 为 Some");
        let cost = osc.shares * c;
        if cost > *pool {
            self.outs[k] = Some(OscOut {
                restore_due: true,
                ..osc
            });
            res.n_osc_restore_defer_bars += 1;
            return false;
        }
        let LayerState::Long {
            entry_bar,
            entry_price,
            shares,
            weight,
            deferred_bars,
            partial,
        } = layers[k]
        else {
            unreachable!("osc 在外 ⇒ 本层恒 Long（腿生命周期内层不变迁）")
        };
        debug_assert!(
            (shares - osc.shares).abs() < 1e-12,
            "全抛/如数接回 ⇒ 股数恒等"
        );
        *pool -= cost;
        let net = osc.shares * osc.sell_price - cost;
        res.osc_net_cash_by_ladder[k] += net;
        res.osc_net_cash_at_level[osc.alad] += net;
        res.trades.push(LayerTrade {
            ladder: k as u8,
            entry_bar,
            entry_price,
            exit_bar: osc.sell_bar,
            exit_price: osc.sell_price,
            shares: osc.shares,
            weight_at_entry: weight,
            deferred_bars,
            partial,
            exit_reason: "osc_diff",
            polarity: Polarity::Long,
        });
        layers[k] = LayerState::Long {
            entry_bar: bar,
            entry_price: c,
            shares,
            weight,
            deferred_bars: 0,
            partial,
        };
        self.outs[k] = None;
        true
    }
}
